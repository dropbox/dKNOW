//! Cascade layout predictor implementation.
//!
//! Supports three layout detection backends:
//! - **Heuristic** (~1ms): Fast text-block grouping for simple documents
//! - **RT-DETR** (~240ms CPU, ~120ms INT8): Full 17-class model including form elements
//! - **YOLO** (~590ms CPU, ~10ms GPU): DocLayout-YOLO-DocLayNet with 11 classes
//!
//! **Note (N=3491):** YOLO is actually 2.5x SLOWER than RT-DETR on CPU due to larger
//! input size (1120x1120 vs 640x640). YOLO only provides speedup with GPU acceleration
//! (CUDA, CoreML/Metal). For CPU-only deployment, use `Auto` mode (2-tier cascade).

// Complexity estimation uses usize for counts and f64 for statistics.
// Precision loss is acceptable for page-level metrics (dimensions < 10000).
// Timing values from Instant::elapsed() are u128 nanoseconds, cast to u64 for logging.
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]

use crate::baseline::LayoutCluster;
use crate::error::Result;
use crate::models::complexity_estimator::{
    Complexity, ComplexityEstimator, ComplexityStats, TextBlock,
};
use crate::models::heuristic_layout::HeuristicLayoutDetector;
#[cfg(feature = "coreml")]
use crate::models::layout_predictor::DocLayoutYoloCoreML;
use crate::models::layout_predictor::{DocLayoutYolo, LayoutPredictorModel};
use crate::pipeline::SimpleTextCell;
use ndarray::Array3;
use std::path::Path;
use std::time::Instant;

/// Cascade routing mode for layout detection.
///
/// Controls when to use fast heuristic-based detection vs. full ML model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CascadeMode {
    /// Always use RT-DETR ML model (no cascade routing).
    ///
    /// Equivalent to the original behavior. Use when:
    /// - Maximum accuracy is required (17 classes including forms)
    /// - All documents have complex layouts
    /// - You want consistent processing time
    #[default]
    AlwaysML,

    /// Use heuristics for simple pages, RT-DETR ML for complex.
    ///
    /// 2-tier cascade. Provides:
    /// - 5-10x speedup on simple documents
    /// - 90%+ accuracy maintained on complex pages
    Auto,

    /// 3-tier cascade: heuristic → YOLO → RT-DETR.
    ///
    /// **ONLY RECOMMENDED WITH GPU ACCELERATION.** Uses DocLayout-YOLO for
    /// standard documents, RT-DETR only for form-heavy documents.
    ///
    /// **Warning (N=3491):** On CPU, YOLO is 2.5x SLOWER than RT-DETR due to
    /// larger input size. This mode only provides speedup with GPU (CUDA/CoreML).
    /// For CPU deployment, use `Auto` mode instead.
    ///
    /// Routing (with GPU):
    /// - Simple pages → Heuristic (~1ms)
    /// - Standard pages → YOLO (~10ms GPU, 11 `DocLayNet` classes)
    /// - Form-heavy pages → RT-DETR (~240ms CPU, 17 classes)
    ///
    /// Requires YOLO model to be loaded via `with_yolo_model()`.
    AutoWithYolo,

    /// Always use heuristics (fastest, for simple documents only).
    ///
    /// Use with caution - tables and figures will not be detected.
    /// Best for:
    /// - Documents known to be simple (single column text)
    /// - Speed-critical applications where some accuracy loss is acceptable
    AlwaysHeuristic,

    /// Always use YOLO model (11 classes, GPU-optimized).
    ///
    /// **Warning (N=3491):** On CPU, YOLO is 2.5x SLOWER than RT-DETR.
    /// Only use this mode with GPU acceleration (CUDA/CoreML/Metal).
    ///
    /// Best for (with GPU only):
    /// - Standard documents without form elements
    /// - Speed-critical applications with GPU available
    /// - ~10ms per page with GPU, ~590ms on CPU
    ///
    /// Requires YOLO model to be loaded via `with_yolo_model()`.
    AlwaysYolo,

    /// Always use CoreML YOLO model (macOS only, Apple Neural Engine).
    ///
    /// **Best performance on Apple Silicon Macs.**
    /// Achieves ~74ms per page (7.0x faster than ONNX CPU).
    ///
    /// Best for:
    /// - macOS with Apple Silicon (M1/M2/M3)
    /// - Production deployments on Mac
    /// - Maximum throughput with ANE acceleration
    ///
    /// Requires CoreML model to be loaded via `with_coreml_model()`.
    /// Falls back to ONNX YOLO or RT-DETR if CoreML not available.
    #[cfg(feature = "coreml")]
    AlwaysCoreML,

    /// 3-tier cascade with CoreML: heuristic → CoreML YOLO → RT-DETR.
    ///
    /// **RECOMMENDED for macOS with Apple Silicon.**
    ///
    /// Uses CoreML for standard documents (7.0x faster than ONNX),
    /// RT-DETR only for form-heavy documents requiring 17 classes.
    ///
    /// Routing:
    /// - Simple pages → Heuristic (~1ms)
    /// - Standard pages → CoreML YOLO (~74ms ANE, 11 classes)
    /// - Form-heavy pages → RT-DETR (~120ms INT8, 17 classes)
    ///
    /// Requires CoreML model to be loaded via `with_coreml_model()`.
    #[cfg(feature = "coreml")]
    AutoWithCoreML,

    /// Conservative: use heuristics only for definitely simple pages.
    ///
    /// More conservative than Auto mode. Falls back to ML for any
    /// uncertainty. Good balance between speed and safety.
    Conservative,
}

impl std::fmt::Display for CascadeMode {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlwaysML => write!(f, "always_ml"),
            Self::Auto => write!(f, "auto"),
            Self::AutoWithYolo => write!(f, "auto_with_yolo"),
            Self::AlwaysHeuristic => write!(f, "always_heuristic"),
            Self::AlwaysYolo => write!(f, "always_yolo"),
            #[cfg(feature = "coreml")]
            Self::AlwaysCoreML => write!(f, "always_coreml"),
            #[cfg(feature = "coreml")]
            Self::AutoWithCoreML => write!(f, "auto_with_coreml"),
            Self::Conservative => write!(f, "conservative"),
        }
    }
}

impl std::str::FromStr for CascadeMode {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "always_ml" | "alwaysml" | "ml" => Ok(Self::AlwaysML),
            "auto" => Ok(Self::Auto),
            "auto_with_yolo" | "autowithyolo" | "yolo_auto" => Ok(Self::AutoWithYolo),
            "always_heuristic" | "alwaysheuristic" | "heuristic" => Ok(Self::AlwaysHeuristic),
            "always_yolo" | "alwaysyolo" | "yolo" => Ok(Self::AlwaysYolo),
            #[cfg(feature = "coreml")]
            "always_coreml" | "alwayscoreml" | "coreml" => Ok(Self::AlwaysCoreML),
            #[cfg(feature = "coreml")]
            "auto_with_coreml" | "autowithcoreml" | "coreml_auto" => Ok(Self::AutoWithCoreML),
            #[cfg(not(feature = "coreml"))]
            "always_coreml" | "alwayscoreml" | "coreml" | "auto_with_coreml" | "autowithcoreml"
            | "coreml_auto" => {
                Err("CoreML modes require the 'coreml' feature to be enabled".to_string())
            }
            "conservative" => Ok(Self::Conservative),
            _ => Err(format!(
                "Unknown cascade mode '{}'. Expected: auto, always_ml, always_heuristic, \
                 always_yolo, auto_with_yolo, conservative{}",
                s,
                if cfg!(feature = "coreml") {
                    ", always_coreml, auto_with_coreml"
                } else {
                    ""
                }
            )),
        }
    }
}

/// Statistics for cascade routing performance analysis.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CascadeStats {
    /// Complexity classification statistics
    pub complexity_stats: ComplexityStats,
    /// Total time spent on complexity estimation (microseconds)
    pub complexity_time_us: u64,
    /// Total time spent on heuristic detection (microseconds)
    pub heuristic_time_us: u64,
    /// Total time spent on YOLO detection (microseconds)
    pub yolo_time_us: u64,
    /// Total time spent on `CoreML` YOLO detection (microseconds)
    pub coreml_time_us: u64,
    /// Total time spent on RT-DETR ML detection (microseconds)
    pub ml_time_us: u64,
    /// Number of pages routed to heuristic path
    pub heuristic_count: usize,
    /// Number of pages routed to YOLO path
    pub yolo_count: usize,
    /// Number of pages routed to `CoreML` path
    pub coreml_count: usize,
    /// Number of pages routed to RT-DETR ML path
    pub ml_count: usize,
}

impl CascadeStats {
    /// Calculate estimated time saved by cascade routing.
    ///
    /// Uses actual benchmarks (N=3491, N=3522):
    /// - RT-DETR: ~240ms (FP32), ~120ms (INT8)
    /// - YOLO ONNX: ~590ms on CPU (2.5x SLOWER than RT-DETR!)
    /// - `CoreML` YOLO: ~74ms on ANE (7.0x faster than ONNX CPU)
    /// - Heuristic: ~1ms
    ///
    /// **Note:** ONNX YOLO only saves time with GPU acceleration (~10ms).
    /// `CoreML` YOLO provides significant savings on macOS (~46ms saved per page).
    #[inline]
    #[must_use = "returns the estimated time saved by cascade routing in milliseconds"]
    pub fn estimated_time_saved_ms(&self) -> f64 {
        // Using INT8 RT-DETR baseline (~120ms)
        // Each heuristic page saves ~119ms (120ms - 1ms)
        // CoreML page saves ~46ms (120ms - 74ms)
        // YOLO on CPU: NEGATIVE savings (-470ms each!) - not counted
        (self.heuristic_count as f64).mul_add(119.0, self.coreml_count as f64 * 46.0)
    }

    /// Calculate speedup factor from cascade routing.
    ///
    /// Uses actual benchmarks (N=3491, N=3522):
    /// - RT-DETR INT8: ~120ms
    /// - YOLO on CPU: ~590ms (2.5x slower!)
    /// - `CoreML` YOLO: ~74ms (ANE, 1.6x faster than RT-DETR)
    /// - Heuristic: ~1ms
    #[inline]
    #[must_use = "returns the speedup factor from cascade routing"]
    pub fn speedup_factor(&self) -> f64 {
        let total_pages =
            self.heuristic_count + self.yolo_count + self.coreml_count + self.ml_count;
        if total_pages == 0 {
            return 1.0;
        }

        // Without cascade: all pages use RT-DETR ML (~120ms INT8)
        let ml_only_time = total_pages as f64 * 120.0;

        // With cascade: heuristics ~1ms, CoreML ~74ms, YOLO ~590ms (CPU), RT-DETR ~120ms
        // Note: YOLO is SLOWER on CPU - only beneficial with GPU
        // Note: CoreML provides real speedup (7.0x vs ONNX, 1.6x vs RT-DETR)
        let cascade_time = (self.heuristic_count as f64).mul_add(
            1.0,
            (self.coreml_count as f64).mul_add(
                74.0, // CoreML is fastest ML option
                (self.yolo_count as f64).mul_add(590.0, self.ml_count as f64 * 120.0), // YOLO slower on CPU!
            ),
        );

        if cascade_time > 0.0 {
            ml_only_time / cascade_time
        } else {
            1.0
        }
    }

    /// Percentage of pages using fast path (heuristic + `CoreML` + YOLO).
    #[inline]
    #[must_use = "returns the percentage of pages using fast path"]
    pub fn fast_path_percentage(&self) -> f64 {
        let total = self.heuristic_count + self.yolo_count + self.coreml_count + self.ml_count;
        if total == 0 {
            return 0.0;
        }
        (self.heuristic_count + self.coreml_count + self.yolo_count) as f64 / total as f64 * 100.0
    }

    /// Percentage of pages using heuristic path only.
    #[inline]
    #[must_use = "returns the percentage of pages using heuristic path"]
    pub fn heuristic_percentage(&self) -> f64 {
        let total = self.heuristic_count + self.yolo_count + self.coreml_count + self.ml_count;
        if total == 0 {
            return 0.0;
        }
        self.heuristic_count as f64 / total as f64 * 100.0
    }

    /// Percentage of pages using YOLO (ONNX) path.
    #[inline]
    #[must_use = "returns the percentage of pages using YOLO path"]
    pub fn yolo_percentage(&self) -> f64 {
        let total = self.heuristic_count + self.yolo_count + self.coreml_count + self.ml_count;
        if total == 0 {
            return 0.0;
        }
        self.yolo_count as f64 / total as f64 * 100.0
    }

    /// Percentage of pages using `CoreML` path.
    #[inline]
    #[must_use = "returns the percentage of pages using CoreML path"]
    pub fn coreml_percentage(&self) -> f64 {
        let total = self.heuristic_count + self.yolo_count + self.coreml_count + self.ml_count;
        if total == 0 {
            return 0.0;
        }
        self.coreml_count as f64 / total as f64 * 100.0
    }
}

/// Cascade layout predictor with adaptive routing.
///
/// Routes pages to heuristic or RT-DETR layout detection based on
/// document complexity. Provides significant speedup for simple documents
/// while maintaining accuracy for complex layouts.
///
/// ## 2-Tier Cascade (Auto mode - RECOMMENDED for CPU)
///
/// ```text
/// Page → Complexity Estimator (~1ms)
///   ├─ Simple (text only)     → Heuristic (~1ms)
///   └─ Complex (17 classes)   → RT-DETR (~120ms INT8)
/// ```
///
/// ## 3-Tier Cascade with `CoreML` (`AutoWithCoreML` - RECOMMENDED for macOS)
///
/// **Best performance on Apple Silicon.** `CoreML` is 7.0x faster than ONNX CPU.
///
/// ```text
/// Page → Complexity Estimator (~1ms)
///   ├─ Simple (text only)     → Heuristic (~1ms)
///   ├─ Standard (11 classes)  → CoreML YOLO (~74ms ANE)
///   └─ Complex (17 classes)   → RT-DETR (~120ms INT8)
/// ```
///
/// Use `with_coreml_model()` to enable CoreML-based routing (macOS only).
///
/// ## 3-Tier Cascade (`AutoWithYolo` mode - GPU ONLY)
///
/// **Warning:** YOLO is 2.5x SLOWER than RT-DETR on CPU. Only use with GPU.
///
/// ```text
/// Page → Complexity Estimator (~1ms)
///   ├─ Simple (text only)     → Heuristic (~1ms)
///   ├─ Standard (11 classes)  → YOLO (~10ms GPU, ~590ms CPU!)
///   └─ Complex (17 classes)   → RT-DETR (~120ms INT8)
/// ```
///
/// Use `with_yolo_model()` to enable YOLO-based routing (GPU only).
pub struct CascadeLayoutPredictor {
    /// RT-DETR ML-based layout model (17 classes including forms)
    ml_model: LayoutPredictorModel,
    /// YOLO-based fast layout model (11 `DocLayNet` classes) - ONNX
    yolo_model: Option<DocLayoutYolo>,
    /// `CoreML`-based fast layout model (11 `DocLayNet` classes) - Apple Neural Engine
    #[cfg(feature = "coreml")]
    coreml_model: Option<DocLayoutYoloCoreML>,
    /// Rule-based complexity estimator
    complexity_estimator: ComplexityEstimator,
    /// Rule-based heuristic layout detector
    heuristic_detector: HeuristicLayoutDetector,
    /// Cascade routing mode
    mode: CascadeMode,
    /// Accumulated statistics
    stats: CascadeStats,
}

impl CascadeLayoutPredictor {
    /// Create a new cascade layout predictor.
    ///
    /// # Arguments
    ///
    /// * `ml_model` - Pre-loaded RT-DETR layout predictor (`LayoutPredictorModel`)
    /// * `mode` - Cascade routing mode (Auto recommended for most cases)
    ///
    /// For 3-tier cascade with YOLO, use `with_yolo_model()` after construction.
    /// For 3-tier cascade with `CoreML` (macOS), use `with_coreml_model()` after construction.
    #[inline]
    #[must_use = "returns a new cascade layout predictor"]
    pub fn new(ml_model: LayoutPredictorModel, mode: CascadeMode) -> Self {
        Self {
            ml_model,
            yolo_model: None,
            #[cfg(feature = "coreml")]
            coreml_model: None,
            complexity_estimator: ComplexityEstimator::new(),
            heuristic_detector: HeuristicLayoutDetector::new(),
            mode,
            stats: CascadeStats::default(),
        }
    }

    /// Create predictor with custom estimator and detector settings.
    #[inline]
    #[must_use = "returns a new predictor with custom settings"]
    pub fn with_settings(
        ml_model: LayoutPredictorModel,
        mode: CascadeMode,
        complexity_estimator: ComplexityEstimator,
        heuristic_detector: HeuristicLayoutDetector,
    ) -> Self {
        Self {
            ml_model,
            yolo_model: None,
            #[cfg(feature = "coreml")]
            coreml_model: None,
            complexity_estimator,
            heuristic_detector,
            mode,
            stats: CascadeStats::default(),
        }
    }

    /// Add YOLO model for fast layout detection.
    ///
    /// Enables `AutoWithYolo` and `AlwaysYolo` cascade modes.
    ///
    /// # Arguments
    ///
    /// * `yolo_model` - Pre-loaded DocLayout-YOLO model
    ///
    /// # Example
    ///
    /// ```ignore
    /// use docling_pdf_ml::models::layout_predictor::DocLayoutYolo;
    /// use docling_pdf_ml::models::cascade_layout::{CascadeLayoutPredictor, CascadeMode};
    ///
    /// let yolo = DocLayoutYolo::load(Path::new("models/doclayout_yolo_doclaynet.onnx"))?;
    /// let predictor = CascadeLayoutPredictor::new(rt_detr_model, CascadeMode::AutoWithYolo)
    ///     .with_yolo_model(yolo);
    /// ```
    #[inline]
    #[must_use = "returns the predictor with YOLO model configured"]
    pub fn with_yolo_model(mut self, yolo_model: DocLayoutYolo) -> Self {
        self.yolo_model = Some(yolo_model);
        self
    }

    /// Load and add YOLO model from path.
    ///
    /// Convenience method that loads the YOLO model and adds it.
    ///
    /// # Arguments
    ///
    /// * `model_path` - Path to DocLayout-YOLO ONNX model
    ///
    /// # Errors
    ///
    /// Returns error if model fails to load.
    #[must_use = "builder pattern returns updated Self that should be used"]
    pub fn with_yolo_model_path(mut self, model_path: &Path) -> Result<Self> {
        let yolo = DocLayoutYolo::load(model_path).map_err(|e| {
            crate::error::DoclingError::ConfigError {
                reason: format!("Failed to load YOLO model: {e}"),
            }
        })?;
        self.yolo_model = Some(yolo);
        Ok(self)
    }

    /// Check if YOLO model is loaded.
    #[inline]
    #[must_use = "returns whether the YOLO model is loaded"]
    pub const fn has_yolo_model(&self) -> bool {
        self.yolo_model.is_some()
    }

    /// Add CoreML model for Apple Neural Engine acceleration.
    ///
    /// Enables `AutoWithCoreML` and `AlwaysCoreML` cascade modes (macOS only).
    /// Provides 7.0x speedup over ONNX CPU on Apple Silicon.
    ///
    /// # Arguments
    ///
    /// * `coreml_model` - Pre-loaded DocLayout-YOLO CoreML model
    ///
    /// # Example
    ///
    /// ```ignore
    /// use docling_pdf_ml::models::layout_predictor::DocLayoutYoloCoreML;
    /// use docling_pdf_ml::models::cascade_layout::{CascadeLayoutPredictor, CascadeMode};
    ///
    /// let coreml = DocLayoutYoloCoreML::load(Path::new("models/doclayout_yolo_doclaynet.mlmodel"))?;
    /// let predictor = CascadeLayoutPredictor::new(rt_detr_model, CascadeMode::AutoWithCoreML)
    ///     .with_coreml_model(coreml);
    /// ```
    #[cfg(feature = "coreml")]
    #[inline]
    #[must_use = "returns the predictor with CoreML model configured"]
    pub fn with_coreml_model(mut self, coreml_model: DocLayoutYoloCoreML) -> Self {
        self.coreml_model = Some(coreml_model);
        self
    }

    /// Load and add CoreML model from path.
    ///
    /// Convenience method that loads the CoreML model and adds it.
    ///
    /// # Arguments
    ///
    /// * `model_path` - Path to DocLayout-YOLO CoreML model (.mlmodel or .mlmodelc)
    ///
    /// # Errors
    ///
    /// Returns error if model fails to load.
    #[cfg(feature = "coreml")]
    #[must_use = "builder pattern returns updated Self that should be used"]
    pub fn with_coreml_model_path(mut self, model_path: &Path) -> Result<Self> {
        let coreml = DocLayoutYoloCoreML::load(model_path).map_err(|e| {
            crate::error::DoclingError::ConfigError {
                reason: format!("Failed to load CoreML model: {e}"),
            }
        })?;
        self.coreml_model = Some(coreml);
        Ok(self)
    }

    /// Check if CoreML model is loaded.
    #[cfg(feature = "coreml")]
    #[inline]
    #[must_use = "returns whether the CoreML model is loaded"]
    pub const fn has_coreml_model(&self) -> bool {
        self.coreml_model.is_some()
    }

    /// Get accumulated cascade statistics.
    #[inline]
    #[must_use = "returns a reference to the cascade statistics"]
    pub const fn stats(&self) -> &CascadeStats {
        &self.stats
    }

    /// Reset statistics.
    #[inline]
    pub fn reset_stats(&mut self) {
        self.stats = CascadeStats::default();
    }

    /// Get current cascade mode.
    #[inline]
    #[must_use = "returns the current cascade mode"]
    pub const fn mode(&self) -> CascadeMode {
        self.mode
    }

    /// Set cascade mode.
    #[inline]
    pub fn set_mode(&mut self, mode: CascadeMode) {
        self.mode = mode;
    }

    /// Run inference on a single page.
    ///
    /// # Arguments
    ///
    /// * `image` - Page image as RGB array (`HxWx3`)
    /// * `text_cells` - Text cells extracted from PDF/OCR
    /// * `page_width` - Page width in points
    /// * `page_height` - Page height in points
    ///
    /// # Returns
    ///
    /// Vector of layout clusters with labels and bounding boxes.
    #[allow(clippy::branches_sharing_code)] // Intentional: timing each branch separately
    #[allow(clippy::too_many_lines)]
    pub fn infer(
        &mut self,
        image: &Array3<u8>,
        text_cells: &[SimpleTextCell],
        page_width: f32,
        page_height: f32,
    ) -> Result<Vec<LayoutCluster>> {
        // Fast path: mode overrides (no complexity estimation)
        match self.mode {
            CascadeMode::AlwaysML => {
                let start = Instant::now();
                let result = self.infer_ml(image);
                self.stats.ml_time_us += start.elapsed().as_micros() as u64;
                self.stats.ml_count += 1;
                return result;
            }
            CascadeMode::AlwaysHeuristic => {
                let start = Instant::now();
                let text_blocks = Self::cells_to_text_blocks(text_cells);
                let clusters = self.infer_heuristic(&text_blocks, page_width, page_height);
                self.stats.heuristic_time_us += start.elapsed().as_micros() as u64;
                self.stats.heuristic_count += 1;
                return Ok(clusters);
            }
            CascadeMode::AlwaysYolo => {
                return self.infer_yolo_with_fallback(image);
            }
            #[cfg(feature = "coreml")]
            CascadeMode::AlwaysCoreML => {
                return self.infer_coreml_with_fallback(image);
            }
            #[cfg(feature = "coreml")]
            CascadeMode::AutoWithCoreML => {
                // Continue to complexity estimation
            }
            CascadeMode::Auto | CascadeMode::AutoWithYolo | CascadeMode::Conservative => {
                // Continue to complexity estimation
            }
        }

        // Convert cells to text blocks for complexity estimation
        let text_blocks = Self::cells_to_text_blocks(text_cells);

        // Estimate complexity
        let start = Instant::now();
        let (complexity, features) =
            self.complexity_estimator
                .estimate(image, &text_blocks, page_width, page_height);
        self.stats.complexity_time_us += start.elapsed().as_micros() as u64;
        self.stats.complexity_stats.record(complexity);

        // Route based on complexity and mode
        match self.mode {
            CascadeMode::Auto => {
                // 2-tier: heuristic for simple, RT-DETR for rest
                if complexity == Complexity::Simple {
                    let start = Instant::now();
                    let clusters = self.infer_heuristic(&text_blocks, page_width, page_height);
                    self.stats.heuristic_time_us += start.elapsed().as_micros() as u64;
                    self.stats.heuristic_count += 1;
                    Ok(clusters)
                } else {
                    let start = Instant::now();
                    let result = self.infer_ml(image);
                    self.stats.ml_time_us += start.elapsed().as_micros() as u64;
                    self.stats.ml_count += 1;
                    result
                }
            }
            CascadeMode::AutoWithYolo => {
                // 3-tier: heuristic → YOLO → RT-DETR
                match complexity {
                    Complexity::Simple => {
                        // Very simple pages: use heuristic (1ms)
                        let start = Instant::now();
                        let clusters = self.infer_heuristic(&text_blocks, page_width, page_height);
                        self.stats.heuristic_time_us += start.elapsed().as_micros() as u64;
                        self.stats.heuristic_count += 1;
                        Ok(clusters)
                    }
                    Complexity::Moderate => {
                        // Standard pages: use YOLO (4ms) if available
                        self.infer_yolo_with_fallback(image)
                    }
                    Complexity::Complex => {
                        // Complex/form-heavy pages: check if forms detected
                        // If form elements detected, use RT-DETR (17 classes)
                        // Otherwise use YOLO (11 classes is sufficient)
                        if features.has_form_elements {
                            let start = Instant::now();
                            let result = self.infer_ml(image);
                            self.stats.ml_time_us += start.elapsed().as_micros() as u64;
                            self.stats.ml_count += 1;
                            result
                        } else {
                            self.infer_yolo_with_fallback(image)
                        }
                    }
                }
            }
            CascadeMode::Conservative => {
                // Conservative: only use heuristic for definitely simple pages
                if complexity == Complexity::Simple
                    && self
                        .complexity_estimator
                        .is_definitely_simple(&text_blocks, page_width)
                {
                    let start = Instant::now();
                    let clusters = self.infer_heuristic(&text_blocks, page_width, page_height);
                    self.stats.heuristic_time_us += start.elapsed().as_micros() as u64;
                    self.stats.heuristic_count += 1;
                    Ok(clusters)
                } else {
                    let start = Instant::now();
                    let result = self.infer_ml(image);
                    self.stats.ml_time_us += start.elapsed().as_micros() as u64;
                    self.stats.ml_count += 1;
                    result
                }
            }
            #[cfg(feature = "coreml")]
            CascadeMode::AutoWithCoreML => {
                // 3-tier cascade with CoreML: heuristic → CoreML YOLO → RT-DETR
                match complexity {
                    Complexity::Simple => {
                        // Very simple pages: use heuristic (1ms)
                        let start = Instant::now();
                        let clusters = self.infer_heuristic(&text_blocks, page_width, page_height);
                        self.stats.heuristic_time_us += start.elapsed().as_micros() as u64;
                        self.stats.heuristic_count += 1;
                        Ok(clusters)
                    }
                    Complexity::Moderate => {
                        // Standard pages: use CoreML (~74ms ANE) if available
                        self.infer_coreml_with_fallback(image)
                    }
                    Complexity::Complex => {
                        // Complex/form-heavy pages: check if forms detected
                        // If form elements detected, use RT-DETR (17 classes)
                        // Otherwise use CoreML (11 classes is sufficient)
                        if features.has_form_elements {
                            let start = Instant::now();
                            let result = self.infer_ml(image);
                            self.stats.ml_time_us += start.elapsed().as_micros() as u64;
                            self.stats.ml_count += 1;
                            result
                        } else {
                            self.infer_coreml_with_fallback(image)
                        }
                    }
                }
            }
            // AlwaysML, AlwaysHeuristic, AlwaysYolo, AlwaysCoreML handled above
            _ => unreachable!(),
        }
    }

    /// Run YOLO inference, falling back to RT-DETR if YOLO not available.
    #[allow(clippy::branches_sharing_code)] // Intentional: timing each branch separately
    fn infer_yolo_with_fallback(&mut self, image: &Array3<u8>) -> Result<Vec<LayoutCluster>> {
        if let Some(ref mut yolo) = self.yolo_model {
            let start = Instant::now();
            let result =
                yolo.infer(image)
                    .map_err(|e| crate::error::DoclingError::InferenceError {
                        model_name: "DocLayoutYolo".to_string(),
                        source: format!("{e}").into(),
                    });
            self.stats.yolo_time_us += start.elapsed().as_micros() as u64;
            self.stats.yolo_count += 1;
            result
        } else {
            // Fallback to RT-DETR if YOLO not loaded
            let start = Instant::now();
            let result = self.infer_ml(image);
            self.stats.ml_time_us += start.elapsed().as_micros() as u64;
            self.stats.ml_count += 1;
            result
        }
    }

    /// Run CoreML inference, falling back to YOLO then RT-DETR if CoreML not available.
    ///
    /// Provides 7.0x speedup over ONNX CPU on Apple Silicon via Apple Neural Engine.
    #[cfg(feature = "coreml")]
    fn infer_coreml_with_fallback(&mut self, image: &Array3<u8>) -> Result<Vec<LayoutCluster>> {
        if let Some(ref mut coreml) = self.coreml_model {
            let start = Instant::now();
            let result =
                coreml
                    .infer(image)
                    .map_err(|e| crate::error::DoclingError::InferenceError {
                        model_name: "DocLayoutYoloCoreML".to_string(),
                        source: format!("{e}").into(),
                    });
            self.stats.coreml_time_us += start.elapsed().as_micros() as u64;
            self.stats.coreml_count += 1;
            result
        } else if let Some(ref mut yolo) = self.yolo_model {
            // Fallback to ONNX YOLO if CoreML not loaded
            let start = Instant::now();
            let result =
                yolo.infer(image)
                    .map_err(|e| crate::error::DoclingError::InferenceError {
                        model_name: "DocLayoutYolo".to_string(),
                        source: format!("{e}").into(),
                    });
            self.stats.yolo_time_us += start.elapsed().as_micros() as u64;
            self.stats.yolo_count += 1;
            result
        } else {
            // Fallback to RT-DETR if neither CoreML nor YOLO loaded
            let start = Instant::now();
            let result = self.infer_ml(image);
            self.stats.ml_time_us += start.elapsed().as_micros() as u64;
            self.stats.ml_count += 1;
            result
        }
    }

    /// Run inference on multiple pages (batch mode).
    ///
    /// Note: Batch mode always uses ML model for now. Individual page routing
    /// would require unbatching, which may reduce efficiency. Future optimization
    /// could group simple pages for heuristic processing.
    pub fn infer_batch(
        &mut self,
        images: &[Array3<u8>],
        text_cells_per_page: &[Vec<SimpleTextCell>],
        page_widths: &[f32],
        page_heights: &[f32],
    ) -> Result<Vec<Vec<LayoutCluster>>> {
        // Validate inputs
        if images.len() != text_cells_per_page.len()
            || images.len() != page_widths.len()
            || images.len() != page_heights.len()
        {
            return Err(crate::error::DoclingError::ConfigError {
                reason: "Mismatched batch input lengths".to_string(),
            });
        }

        // AlwaysML mode: use batch inference for efficiency
        if self.mode == CascadeMode::AlwaysML {
            return self.infer_ml_batch(images);
        }

        // AlwaysHeuristic mode: process all with heuristics
        if self.mode == CascadeMode::AlwaysHeuristic {
            return Ok(images
                .iter()
                .zip(text_cells_per_page.iter())
                .zip(page_widths.iter().zip(page_heights.iter()))
                .map(|((_, cells), (&w, &h))| {
                    let text_blocks = Self::cells_to_text_blocks(cells);
                    self.infer_heuristic(&text_blocks, w, h)
                })
                .collect());
        }

        // Auto/Conservative mode: route individually
        // Note: This loses batch efficiency but provides per-page routing.
        // For maximum efficiency with mixed documents, consider pre-classifying
        // and batching by complexity class.
        images
            .iter()
            .zip(text_cells_per_page.iter())
            .zip(page_widths.iter().zip(page_heights.iter()))
            .map(|((image, cells), (&w, &h))| self.infer(image, cells, w, h))
            .collect()
    }

    /// Convert `SimpleTextCell` to `TextBlock` for heuristic processing.
    fn cells_to_text_blocks(cells: &[SimpleTextCell]) -> Vec<TextBlock> {
        cells
            .iter()
            .map(|cell| {
                // Estimate font size from bounding box height
                // This is an approximation; actual font size may differ
                let font_size = (cell.rect.b - cell.rect.t).abs();

                TextBlock::new(
                    (cell.rect.l, cell.rect.t, cell.rect.r, cell.rect.b),
                    font_size,
                    cell.text.clone(),
                )
            })
            .collect()
    }

    /// Run heuristic-based layout detection.
    fn infer_heuristic(
        &self,
        text_blocks: &[TextBlock],
        page_width: f32,
        page_height: f32,
    ) -> Vec<LayoutCluster> {
        self.heuristic_detector
            .detect(text_blocks, page_width, page_height)
    }

    /// Run ML-based layout detection (single image).
    fn infer_ml(&mut self, image: &Array3<u8>) -> Result<Vec<LayoutCluster>> {
        self.ml_model
            .infer(image)
            .map_err(|e| crate::error::DoclingError::InferenceError {
                model_name: "LayoutPredictor".to_string(),
                source: format!("{e}").into(),
            })
    }

    /// Run ML-based layout detection (batch).
    fn infer_ml_batch(&mut self, images: &[Array3<u8>]) -> Result<Vec<Vec<LayoutCluster>>> {
        self.ml_model
            .infer_batch(images)
            .map_err(|e| crate::error::DoclingError::InferenceError {
                model_name: "LayoutPredictor".to_string(),
                source: format!("{e}").into(),
            })
    }

    /// Get underlying ML model (for direct access when needed).
    #[inline]
    #[must_use = "returns a reference to the underlying ML model"]
    pub const fn ml_model(&self) -> &LayoutPredictorModel {
        &self.ml_model
    }

    /// Get mutable reference to underlying ML model.
    #[inline]
    pub fn ml_model_mut(&mut self) -> &mut LayoutPredictorModel {
        &mut self.ml_model
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cascade_mode_display() {
        assert_eq!(CascadeMode::AlwaysML.to_string(), "always_ml");
        assert_eq!(CascadeMode::Auto.to_string(), "auto");
        assert_eq!(CascadeMode::AutoWithYolo.to_string(), "auto_with_yolo");
        assert_eq!(CascadeMode::AlwaysHeuristic.to_string(), "always_heuristic");
        assert_eq!(CascadeMode::AlwaysYolo.to_string(), "always_yolo");
        #[cfg(feature = "coreml")]
        assert_eq!(CascadeMode::AlwaysCoreML.to_string(), "always_coreml");
        #[cfg(feature = "coreml")]
        assert_eq!(CascadeMode::AutoWithCoreML.to_string(), "auto_with_coreml");
        assert_eq!(CascadeMode::Conservative.to_string(), "conservative");
    }

    #[test]
    fn test_cascade_stats_speedup() {
        // All ML (RT-DETR INT8): speedup = 1.0
        let mut stats = CascadeStats {
            ml_count: 10,
            heuristic_count: 0,
            yolo_count: 0,
            ..Default::default()
        };
        assert!((stats.speedup_factor() - 1.0).abs() < 0.01);

        // 50% heuristic: speedup ≈ 2x
        // ML time: 5*120 + 5*1 = 605ms, without cascade: 10*120 = 1200ms
        // speedup = 1200/605 ≈ 1.98
        stats.ml_count = 5;
        stats.heuristic_count = 5;
        stats.yolo_count = 0;
        let speedup = stats.speedup_factor();
        assert!(speedup > 1.5 && speedup < 2.5);

        // All heuristic: speedup ≈ 120x (120ms/1ms)
        stats.ml_count = 0;
        stats.heuristic_count = 10;
        stats.yolo_count = 0;
        let speedup = stats.speedup_factor();
        assert!(speedup > 100.0);

        // All YOLO on CPU: speedup < 1 (YOLO is SLOWER!)
        // 120ms RT-DETR / 590ms YOLO ≈ 0.2x
        stats.ml_count = 0;
        stats.heuristic_count = 0;
        stats.yolo_count = 10;
        let speedup = stats.speedup_factor();
        assert!(
            speedup < 0.3,
            "YOLO should be slower than RT-DETR on CPU: {speedup}"
        );

        // Mixed 3-tier on CPU: actually slower due to YOLO!
        // 30% heuristic, 50% YOLO, 20% ML
        stats.heuristic_count = 3;
        stats.yolo_count = 5;
        stats.ml_count = 2;
        // Time: 3*1 + 5*590 + 2*120 = 3 + 2950 + 240 = 3193ms
        // Without cascade: 10*120 = 1200ms
        // speedup = 1200/3193 ≈ 0.38x (SLOWER!)
        let speedup = stats.speedup_factor();
        assert!(
            speedup < 0.5,
            "Mixed cascade with YOLO should be slower on CPU: {speedup}"
        );
    }

    #[test]
    fn test_cascade_stats_percentage() {
        // 2-tier: 30% heuristic, 70% ML
        let mut stats = CascadeStats {
            heuristic_count: 3,
            ml_count: 7,
            yolo_count: 0,
            ..Default::default()
        };
        assert!((stats.heuristic_percentage() - 30.0).abs() < 0.01);
        assert!((stats.yolo_percentage() - 0.0).abs() < 0.01);
        assert!((stats.fast_path_percentage() - 30.0).abs() < 0.01);

        // All heuristic
        stats.heuristic_count = 10;
        stats.ml_count = 0;
        stats.yolo_count = 0;
        assert!((stats.heuristic_percentage() - 100.0).abs() < 0.01);
        assert!((stats.fast_path_percentage() - 100.0).abs() < 0.01);

        // 3-tier: 20% heuristic, 50% YOLO, 30% ML
        stats.heuristic_count = 2;
        stats.yolo_count = 5;
        stats.ml_count = 3;
        assert!((stats.heuristic_percentage() - 20.0).abs() < 0.01);
        assert!((stats.yolo_percentage() - 50.0).abs() < 0.01);
        assert!((stats.fast_path_percentage() - 70.0).abs() < 0.01);
    }

    #[test]
    fn test_cascade_stats_coreml() {
        // CoreML provides real speedup (74ms vs 120ms RT-DETR)
        // All CoreML: speedup = 120/74 ≈ 1.62x
        let mut stats = CascadeStats {
            coreml_count: 10,
            ml_count: 0,
            heuristic_count: 0,
            yolo_count: 0,
            ..Default::default()
        };
        let speedup = stats.speedup_factor();
        assert!(
            speedup > 1.5 && speedup < 1.7,
            "CoreML should be 1.6x faster than RT-DETR: {speedup}"
        );

        // Mixed with CoreML: 30% heuristic, 50% CoreML, 20% ML
        stats.heuristic_count = 3;
        stats.coreml_count = 5;
        stats.yolo_count = 0;
        stats.ml_count = 2;
        // Time: 3*1 + 5*74 + 2*120 = 3 + 370 + 240 = 613ms
        // Without cascade: 10*120 = 1200ms
        // speedup = 1200/613 ≈ 1.96x
        let speedup = stats.speedup_factor();
        assert!(
            speedup > 1.8 && speedup < 2.1,
            "CoreML cascade should provide ~2x speedup: {speedup}"
        );

        // CoreML percentage should be tracked
        assert!((stats.coreml_percentage() - 50.0).abs() < 0.01);

        // Fast path includes CoreML
        assert!((stats.fast_path_percentage() - 80.0).abs() < 0.01);

        // Estimated time saved with CoreML
        // 3 heuristic pages: 3 * 119ms = 357ms
        // 5 CoreML pages: 5 * 46ms = 230ms
        // Total: 587ms
        let saved = stats.estimated_time_saved_ms();
        assert!(
            (saved - 587.0).abs() < 1.0,
            "Time saved should be ~587ms: {saved}"
        );
    }

    #[test]
    fn test_cells_to_text_blocks() {
        let cells = vec![SimpleTextCell {
            index: 0,
            text: "Hello World".to_string(),
            rect: crate::pipeline::BoundingBox {
                l: 50.0,
                t: 100.0,
                r: 200.0,
                b: 120.0,
                coord_origin: crate::pipeline::CoordOrigin::TopLeft,
            },
            confidence: 1.0,
            from_ocr: false,
            is_bold: false,
            is_italic: false,
        }];

        let blocks = CascadeLayoutPredictor::cells_to_text_blocks(&cells);

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].text, "Hello World");
        assert!((blocks[0].font_size - 20.0).abs() < 0.01); // b - t = 120 - 100 = 20
    }

    #[test]
    fn test_cascade_mode_from_str() {
        // Exact matches from Display output
        assert_eq!(
            "always_ml".parse::<CascadeMode>().unwrap(),
            CascadeMode::AlwaysML
        );
        assert_eq!("auto".parse::<CascadeMode>().unwrap(), CascadeMode::Auto);
        assert_eq!(
            "auto_with_yolo".parse::<CascadeMode>().unwrap(),
            CascadeMode::AutoWithYolo
        );
        assert_eq!(
            "always_heuristic".parse::<CascadeMode>().unwrap(),
            CascadeMode::AlwaysHeuristic
        );
        assert_eq!(
            "always_yolo".parse::<CascadeMode>().unwrap(),
            CascadeMode::AlwaysYolo
        );
        assert_eq!(
            "conservative".parse::<CascadeMode>().unwrap(),
            CascadeMode::Conservative
        );

        // Short aliases
        assert_eq!("ml".parse::<CascadeMode>().unwrap(), CascadeMode::AlwaysML);
        assert_eq!(
            "heuristic".parse::<CascadeMode>().unwrap(),
            CascadeMode::AlwaysHeuristic
        );
        assert_eq!(
            "yolo".parse::<CascadeMode>().unwrap(),
            CascadeMode::AlwaysYolo
        );

        // Case insensitive
        assert_eq!(
            "ALWAYS_ML".parse::<CascadeMode>().unwrap(),
            CascadeMode::AlwaysML
        );
        assert_eq!("Auto".parse::<CascadeMode>().unwrap(), CascadeMode::Auto);

        // Hyphen to underscore conversion
        assert_eq!(
            "always-ml".parse::<CascadeMode>().unwrap(),
            CascadeMode::AlwaysML
        );
        assert_eq!(
            "auto-with-yolo".parse::<CascadeMode>().unwrap(),
            CascadeMode::AutoWithYolo
        );

        // Invalid
        assert!("invalid".parse::<CascadeMode>().is_err());
    }

    #[test]
    fn test_cascade_mode_roundtrip() {
        let modes = [
            CascadeMode::AlwaysML,
            CascadeMode::Auto,
            CascadeMode::AutoWithYolo,
            CascadeMode::AlwaysHeuristic,
            CascadeMode::AlwaysYolo,
            CascadeMode::Conservative,
        ];
        for mode in modes {
            let s = mode.to_string();
            let parsed: CascadeMode = s.parse().unwrap();
            assert_eq!(parsed, mode, "Roundtrip failed for {mode:?}");
        }
    }

    #[cfg(feature = "coreml")]
    #[test]
    fn test_cascade_mode_coreml_from_str() {
        assert_eq!(
            "always_coreml".parse::<CascadeMode>().unwrap(),
            CascadeMode::AlwaysCoreML
        );
        assert_eq!(
            "auto_with_coreml".parse::<CascadeMode>().unwrap(),
            CascadeMode::AutoWithCoreML
        );
        assert_eq!(
            "coreml".parse::<CascadeMode>().unwrap(),
            CascadeMode::AlwaysCoreML
        );
    }
}
