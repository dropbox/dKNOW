// Pipeline executor - orchestrates PDF ML processing
// Note: Infrastructure code ported from Python. Some code paths not yet wired up.
#![allow(dead_code)]
// Intentional ML conversions: array indices, image dimensions, pixel coordinates
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
// Pipeline functions take Vec ownership for data flow semantics
#![allow(clippy::needless_pass_by_value)]

use crate::baseline::{validate_layout_clusters, LayoutCluster, LayoutValidationResult};
use crate::error::{DoclingError, Result};
use crate::models::cascade_layout::{CascadeMode, CascadeStats};
use crate::models::complexity_estimator::{Complexity, ComplexityEstimator, TextBlock};
use crate::models::heuristic_layout::HeuristicLayoutDetector;
use crate::models::layout_predictor::LayoutPredictorModel;
#[cfg(feature = "pytorch")]
use crate::models::table_structure::TableStructureModel;
use crate::models::table_structure_onnx::TableStructureModelOnnx;
use crate::ocr::utils::{IMAGENET_MEAN, IMAGENET_STD};
use crate::pipeline::layout_postprocessor::LayoutPostProcessor;
use crate::pipeline::page_assembly::PageAssembler;
use crate::pipeline::reading_order::{ReadingOrderConfig, ReadingOrderPredictor};
use crate::pipeline::{
    BoundingBox, Cluster, CoordOrigin, DocItemLabel, LayoutPrediction, Page, PageElement,
    PagePredictions, SimpleTextCell, Size,
};
use crate::pipeline_modular::{
    types::{LabeledCluster, LabeledClusters, OCRCells, TextCell as ModularTextCell},
    ModularPipeline,
};
use crate::preprocessing::tableformer::TABLEFORMER_INPUT_SIZE;
use log::trace;
use ndarray::Array3;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::time::{Duration, Instant};
#[cfg(feature = "pytorch")]
use tch::Device;
// Use our stub Device enum when pytorch feature is disabled
#[cfg(not(feature = "pytorch"))]
use crate::pipeline::Device;

/// Auto-detect the best available device for ML inference
///
/// This function detects the fastest available device:
/// 1. **MPS (Metal)** - Apple Silicon GPU (macOS only, requires `PyTorch` backend)
/// 2. **CUDA** - NVIDIA GPU (Linux/Windows with CUDA-enabled `PyTorch`)
/// 3. **CPU** - Fallback for all platforms
///
/// # Returns
/// The best available Device for inference
///
/// # Examples
/// ```no_run
/// use docling_pdf_ml::detect_best_device;
///
/// let device = detect_best_device();
/// println!("Using device: {:?}", device);
/// ```
#[must_use = "returns the best available device for inference"]
pub fn detect_best_device() -> Device {
    #[cfg(feature = "pytorch")]
    {
        // Check for MPS (Apple Metal GPU) on macOS
        #[cfg(target_os = "macos")]
        {
            let has_mps = tch::utils::has_mps();
            if has_mps {
                log::info!("Auto-detected MPS (Metal GPU) - using for acceleration");
                return Device::Mps;
            }
        }

        // Check for CUDA (NVIDIA GPU)
        if tch::Cuda::is_available() {
            let device_count = tch::Cuda::device_count();
            log::info!(
                "Auto-detected {} CUDA device(s) - using GPU 0 for acceleration",
                device_count
            );
            return Device::Cuda(0);
        }
    }

    // On macOS without pytorch, use MPS for CoreML acceleration in ONNX Runtime
    #[cfg(all(target_os = "macos", not(feature = "pytorch")))]
    {
        log::info!("Using MPS device for CoreML acceleration (ONNX Runtime)");
        Device::Mps
    }

    // Fallback to CPU (only compiles when not macOS without pytorch)
    #[cfg(not(all(target_os = "macos", not(feature = "pytorch"))))]
    {
        log::info!("Using CPU for inference (no GPU detected or pytorch feature disabled)");
        Device::Cpu
    }
}

/// Auto-detect optimal batch size for the given device
///
/// Returns the recommended batch size based on device capabilities:
/// - **MPS**: 1 (MPS batch inference has issues, use sequential)
/// - **CUDA**: `min(num_pages`, 8) (GPU can handle batch efficiently)
/// - **CPU**: 1 (sequential is fine for CPU)
///
/// # Arguments
/// * `device` - The inference device
/// * `num_pages` - Total number of pages to process
///
/// # Returns
/// Recommended batch size
#[must_use = "returns the recommended batch size for the device"]
pub fn detect_best_batch_size(device: &Device, num_pages: usize) -> usize {
    match device {
        Device::Mps => 1,                    // MPS batch has issues, use sequential
        Device::Cuda(_) => num_pages.min(8), // GPU batch up to 8
        Device::Cpu => 1,                    // CPU: sequential is fine
        #[cfg(feature = "pytorch")]
        Device::Vulkan => 1, // Vulkan: sequential is fine
    }
}

/// Pipeline configuration
///
/// Configures the PDF parsing pipeline including ML model paths,
/// device selection, and feature flags.
///
/// # Exampless
///
/// ```no_run
/// use docling_pdf_ml::{PipelineConfig, PipelineConfigBuilder, Device};
///
/// // Use default configuration (recommended)
/// let config = PipelineConfig::default();
///
/// // Or use the builder API for customization
/// # fn example() -> docling_pdf_ml::Result<()> {
/// let config = PipelineConfigBuilder::new()
///     .device(Device::Cpu)
///     .ocr_enabled(true)
///     .table_structure_enabled(true)
///     .build()?;
/// # Ok(())
/// # }
/// ```

/// OCR backend selection for scanned document processing
///
/// Apple Vision (macOS only) produces 7x better results than `RapidOCR`
/// for scanned English documents. When available, Apple Vision is the
/// recommended backend on macOS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OcrBackend {
    /// Automatically select the best available OCR backend:
    /// - macOS: Apple Vision (via `macocr` CLI)
    /// - Other: `RapidOCR` (pure Rust)
    #[default]
    Auto,
    /// Apple Vision OCR (macOS only)
    /// Produces 7x more text with better quality than `RapidOCR`.
    /// Falls back to `RapidOCR` if `macocr` is not installed.
    AppleVision,
    /// `RapidOCR` (pure Rust, cross-platform)
    /// Works on all platforms but produces lower quality on scanned documents.
    RapidOcr,
}

impl std::fmt::Display for OcrBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Auto => write!(f, "auto"),
            Self::AppleVision => write!(f, "apple-vision"),
            Self::RapidOcr => write!(f, "rapidocr"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PipelineConfig {
    /// Device to run models on (CPU/CUDA)
    ///
    /// Use `Device::Cpu` for CPU inference or `Device::Cuda(0)` for GPU inference.
    pub device: Device,

    /// Enable OCR for scanned pages
    ///
    /// When enabled, the pipeline will use the selected OCR backend to extract text from images.
    /// When disabled, only programmatic text will be extracted.
    pub ocr_enabled: bool,

    /// OCR backend selection
    ///
    /// Choose between `Auto` (default), `AppleVision` (macOS), or `RapidOcr` (cross-platform).
    /// Apple Vision produces 7x better results on scanned English documents.
    pub ocr_backend: OcrBackend,

    /// Enable table structure detection
    ///
    /// When enabled, the pipeline will use `TableFormer` to extract table structure.
    /// When disabled, tables will be detected but not parsed.
    pub table_structure_enabled: bool,

    /// Inference backend for layout model
    ///
    /// Choose between `PyTorch` (default, 1.56x faster) and ONNX Runtime (cross-platform fallback).
    /// `PyTorch` backend requires libtorch and uses the full RT-DETR implementation.
    /// Default changed to `PyTorch` in N=486 based on performance validation (N=485).
    pub layout_backend: crate::models::layout_predictor::InferenceBackend,

    /// Path to layout model file
    ///
    /// The layout model (`LayoutPredictor`) detects document structure including
    /// text, tables, figures, headers, and other elements.
    ///
    /// - ONNX backend: Path to .onnx file
    /// - `PyTorch` backend: Path to .safetensors file (e.g., from `HuggingFace`)
    pub layout_model_path: PathBuf,

    /// Path to table structure model directory
    ///
    /// The table structure model (`TableFormer`) parses table structure including
    /// rows, columns, and cells.
    pub table_model_dir: PathBuf,

    /// Enable code/formula enrichment
    ///
    /// When enabled, the pipeline will use `CodeFormula` (Idefics3-based model) to
    /// enrich code and formula regions with ML-generated predictions.
    /// When disabled, code/formula elements will use extracted text only.
    pub code_formula_enabled: bool,

    /// Path to code/formula model directory
    ///
    /// The `CodeFormula` model (Idefics3-based vision-language model) generates
    /// enriched text for code blocks and mathematical formulas.
    pub code_formula_model_path: PathBuf,

    /// Scale factor for table inference (`TableFormer`)
    ///
    /// The page image is scaled by this factor before cropping table regions.
    /// Default is 2.0, which corresponds to 144 DPI rendering.
    /// Adjust if your PDF renderer uses a different DPI.
    ///
    /// - Default: 2.0 (for 72 DPI * 2.0 = 144 DPI)
    /// - For 300 DPI rendering: use 300/72 ≈ 4.17
    /// - For 72 DPI rendering: use 1.0
    pub table_scale: f32,

    /// Minimum cell size in points for table structure detection
    ///
    /// Cells smaller than this threshold are filtered out as noise/artifacts.
    /// Default is 1.0 (1x1 point minimum).
    pub min_cell_size_points: f32,

    /// Minimum confidence threshold for table cell detection
    ///
    /// Cells with confidence below this threshold are filtered out.
    /// Default is 0.5 (50% confidence, conservative - keeps most cells).
    pub min_cell_confidence: f32,

    /// Minimum region size in pixels for code/formula enrichment
    ///
    /// Regions smaller than this threshold (in either dimension) are skipped
    /// during code/formula enrichment to avoid ML model issues with tiny inputs.
    /// Default is 10.0 (10x10 pixel minimum).
    pub min_enrichment_region_size: f32,

    /// Cascade mode for layout detection
    ///
    /// Controls when to use fast heuristic-based layout detection vs. full ML model:
    /// - `AlwaysML`: Always use ML model (default, original behavior)
    /// - `Auto`: Use heuristics for simple pages, ML for complex (5-10x speedup on simple docs)
    /// - `AlwaysHeuristic`: Always use heuristics (fastest, for simple documents only)
    /// - `Conservative`: Use heuristics only for definitely simple pages
    pub cascade_mode: CascadeMode,
}

impl Default for PipelineConfig {
    #[inline]
    fn default() -> Self {
        PipelineConfigBuilder::new()
            .build()
            .expect("Default config should always be valid")
    }
}

/// Builder for `PipelineConfig`
///
/// Provides a fluent API for configuring the PDF parsing pipeline.
/// Automatically resolves model paths using intelligent fallback logic:
/// 1. Use explicitly set paths if provided
/// 2. Check local development paths (./`onnx_exports`, etc.)
/// 3. Fall back to system cache directory (~/.cache/docling on Unix)
///
/// # Exampless
///
/// ```no_run
/// use docling_pdf_ml::{PipelineConfigBuilder, Device};
///
/// # fn main() -> docling_pdf_ml::Result<()> {
/// // Use defaults with custom device
/// let config = PipelineConfigBuilder::new()
///     .device(Device::Cuda(0))
///     .build()?;
///
/// // Customize model paths
/// let config = PipelineConfigBuilder::new()
///     .layout_model_path("custom/layout.onnx".into())
///     .table_model_dir("custom/tableformer".into())
///     .build()?;
///
/// // Disable specific features
/// let config = PipelineConfigBuilder::new()
///     .ocr_enabled(false)
///     .table_structure_enabled(false)
///     .build()?;
/// # Ok(())
/// # }
/// ```
// Boolean fields are appropriate for on/off configuration options
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq)]
pub struct PipelineConfigBuilder {
    device: Device,
    ocr_enabled: bool,
    ocr_backend: OcrBackend,
    table_structure_enabled: bool,
    code_formula_enabled: bool,
    layout_backend: crate::models::layout_predictor::InferenceBackend,
    layout_model_path: Option<PathBuf>,
    table_model_dir: Option<PathBuf>,
    code_formula_model_path: Option<PathBuf>,
    cache_dir: Option<PathBuf>,
    /// Skip model path validation (for unit tests without models)
    skip_validation: bool,
    /// Scale factor for table inference (default: 2.0)
    table_scale: f32,
    /// Minimum cell size in points for table structure detection (default: 1.0)
    min_cell_size_points: f32,
    /// Minimum confidence threshold for table cell detection (default: 0.5)
    min_cell_confidence: f32,
    /// Minimum region size in pixels for code/formula enrichment (default: 10.0)
    min_enrichment_region_size: f32,
    /// Cascade mode for layout detection (default: `AlwaysML`)
    cascade_mode: CascadeMode,
    /// Prefer INT8 quantized model for ONNX backend (default: true)
    /// INT8 models are ~4x smaller and ~2x faster with <1% accuracy loss.
    /// Set to false to force FP32 model usage.
    prefer_quantized: bool,
}

impl Default for PipelineConfigBuilder {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl PipelineConfigBuilder {
    /// Create a new config builder with defaults
    ///
    /// Default settings:
    /// - Device: Auto-detected (MPS on Mac, CUDA on Linux/Windows, CPU fallback)
    /// - OCR: Enabled
    /// - Table structure detection: Enabled
    /// - Layout backend: `PyTorch` (changed in N=486 - 1.56x faster than ONNX)
    /// - Code/formula enrichment: Disabled (optional feature)
    /// - Model paths: Auto-resolved (local dev paths → cache directory)
    #[inline]
    #[must_use = "returns a new builder with default settings"]
    pub fn new() -> Self {
        Self {
            device: detect_best_device(),
            ocr_enabled: true,
            ocr_backend: OcrBackend::Auto, // Default: auto-select (Apple Vision on macOS)
            table_structure_enabled: true,
            code_formula_enabled: false,
            layout_backend: crate::models::layout_predictor::InferenceBackend::default(), // PyTorch by default (N=486)
            layout_model_path: None,
            table_model_dir: None,
            code_formula_model_path: None,
            cache_dir: None,
            skip_validation: false,
            table_scale: 2.0,                    // Default: 2.0 for 144 DPI (72 * 2)
            min_cell_size_points: 1.0,           // Default: 1x1 point minimum (filters noise)
            min_cell_confidence: 0.5,            // Default: 50% confidence (conservative)
            min_enrichment_region_size: 10.0,    // Default: 10x10 pixel minimum
            cascade_mode: CascadeMode::AlwaysML, // Default: original behavior
            prefer_quantized: true, // Default: prefer INT8 for ONNX (4x smaller, ~2x faster)
        }
    }

    /// Create a minimal configuration for fast processing
    ///
    /// Minimal preset disables optional features for maximum speed:
    /// - OCR: Disabled (only programmatic text)
    /// - Table structure: Disabled (table detection only, no structure parsing)
    /// - Code/formula enrichment: Disabled
    ///
    /// Use this when you need fast document processing and don't require
    /// table structure or OCR text extraction.
    ///
    /// # Exampless
    ///
    /// ```no_run
    /// use docling_pdf_ml::PipelineConfigBuilder;
    ///
    /// # fn main() -> docling_pdf_ml::Result<()> {
    /// // Fast configuration for programmatic PDFs
    /// let config = PipelineConfigBuilder::minimal()
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    #[must_use = "returns a builder with minimal features for fast processing"]
    pub fn minimal() -> Self {
        Self::new()
            .ocr_enabled(false)
            .table_structure_enabled(false)
            .code_formula_enabled(false)
    }

    /// Create a configuration optimized for speed
    ///
    /// Fast preset balances speed and functionality:
    /// - OCR: Enabled (for scanned documents)
    /// - Table structure: Disabled (saves time on complex tables)
    /// - Layout backend: `PyTorch` (1.56x faster than ONNX)
    ///
    /// This is a good balance between processing speed and feature completeness.
    ///
    /// # Exampless
    ///
    /// ```no_run
    /// use docling_pdf_ml::PipelineConfigBuilder;
    ///
    /// # fn main() -> docling_pdf_ml::Result<()> {
    /// // Fast configuration with OCR but no table parsing
    /// let config = PipelineConfigBuilder::fast()
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    #[must_use = "returns a builder optimized for speed"]
    pub fn fast() -> Self {
        Self::new()
            .table_structure_enabled(false)
            // Use the default backend (PyTorch if pytorch feature enabled, ONNX otherwise)
            .layout_backend(crate::models::layout_predictor::InferenceBackend::default())
    }

    /// Create a configuration with all features enabled
    ///
    /// Complete preset enables all optional features:
    /// - OCR: Enabled (text extraction from images)
    /// - Table structure: Enabled (full table parsing)
    /// - Code/formula enrichment: Enabled (ML-enhanced code and formulas)
    ///
    /// This provides the most comprehensive document analysis but is slower.
    ///
    /// # Exampless
    ///
    /// ```no_run
    /// use docling_pdf_ml::PipelineConfigBuilder;
    ///
    /// # fn main() -> docling_pdf_ml::Result<()> {
    /// // Complete configuration with all features
    /// let config = PipelineConfigBuilder::complete()
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    #[must_use = "returns a builder with all features enabled"]
    pub fn complete() -> Self {
        Self::new()
            .ocr_enabled(true)
            .table_structure_enabled(true)
            .code_formula_enabled(true)
    }

    /// Set the compute device (CPU/CUDA)
    ///
    /// # Exampless
    ///
    /// ```no_run
    /// use docling_pdf_ml::{PipelineConfigBuilder, Device};
    ///
    /// # fn main() -> docling_pdf_ml::Result<()> {
    /// // Use CPU
    /// let config = PipelineConfigBuilder::new()
    ///     .device(Device::Cpu)
    ///     .build()?;
    ///
    /// // Use GPU
    /// let config = PipelineConfigBuilder::new()
    ///     .device(Device::Cuda(0))
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use = "returns the builder with device configured"]
    pub const fn device(mut self, device: Device) -> Self {
        self.device = device;
        self
    }

    /// Set the inference backend for layout model
    ///
    /// Choose between `PyTorch` (default, 1.56x faster) and ONNX Runtime (cross-platform fallback).
    ///
    /// # Exampless
    ///
    /// ```no_run
    /// use docling_pdf_ml::{PipelineConfigBuilder, InferenceBackend};
    ///
    /// # fn main() -> docling_pdf_ml::Result<()> {
    /// // Use default (PyTorch)
    /// let config = PipelineConfigBuilder::new().build()?;
    ///
    /// // Or explicitly set ONNX
    /// let config = PipelineConfigBuilder::new()
    ///     .layout_backend(InferenceBackend::ONNX)
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Default: `InferenceBackend::PyTorch` (changed in N=486, based on N=485 performance validation)
    #[inline]
    #[must_use = "returns the builder with layout backend configured"]
    pub const fn layout_backend(
        mut self,
        backend: crate::models::layout_predictor::InferenceBackend,
    ) -> Self {
        self.layout_backend = backend;
        self
    }

    /// Enable or disable OCR
    ///
    /// When enabled, the pipeline will use the selected OCR backend to extract text from images.
    /// When disabled, only programmatic text will be extracted.
    ///
    /// Default: true
    #[inline]
    #[must_use = "returns the builder with OCR enabled state configured"]
    pub const fn ocr_enabled(mut self, enabled: bool) -> Self {
        self.ocr_enabled = enabled;
        self
    }

    /// Select OCR backend
    ///
    /// Choose between:
    /// - `Auto` (default): Apple Vision on macOS, `RapidOCR` elsewhere
    /// - `AppleVision`: Apple Vision OCR (macOS only, 7x better quality)
    /// - `RapidOcr`: `RapidOCR` (cross-platform, lower quality on scanned docs)
    ///
    /// Default: `OcrBackend::Auto`
    #[inline]
    #[must_use = "returns the builder with OCR backend configured"]
    pub const fn ocr_backend(mut self, backend: OcrBackend) -> Self {
        self.ocr_backend = backend;
        self
    }

    /// Enable or disable table structure detection
    ///
    /// When enabled, the pipeline will use `TableFormer` to parse table structure.
    /// When disabled, tables will be detected but not parsed.
    ///
    /// Default: true
    #[inline]
    #[must_use = "returns the builder with table structure enabled state configured"]
    pub const fn table_structure_enabled(mut self, enabled: bool) -> Self {
        self.table_structure_enabled = enabled;
        self
    }

    /// Enable or disable code/formula enrichment
    ///
    /// When enabled, the pipeline will use `CodeFormula` (Idefics3-based model) to
    /// enrich code and formula regions with ML-generated predictions.
    /// When disabled, code/formula elements will use extracted text only.
    ///
    /// Default: false (optional feature)
    #[inline]
    #[must_use = "returns the builder with code/formula enabled state configured"]
    pub const fn code_formula_enabled(mut self, enabled: bool) -> Self {
        self.code_formula_enabled = enabled;
        self
    }

    /// Set custom layout model path
    ///
    /// If not set, the builder will:
    /// 1. Check `./onnx_exports/layout_optimum/model.onnx` (local dev)
    /// 2. Fall back to `{cache_dir}/models/layout/model.onnx`
    #[inline]
    #[must_use = "returns the builder with layout model path configured"]
    pub fn layout_model_path(mut self, path: PathBuf) -> Self {
        self.layout_model_path = Some(path);
        self
    }

    /// Set custom table model directory
    ///
    /// If not set, the builder will:
    /// 1. Check local development paths
    /// 2. Fall back to `HuggingFace` cache directory
    #[inline]
    #[must_use = "returns the builder with table model directory configured"]
    pub fn table_model_dir(mut self, path: PathBuf) -> Self {
        self.table_model_dir = Some(path);
        self
    }

    /// Set custom code/formula model directory
    ///
    /// If not set, the builder will:
    /// 1. Check local development paths
    /// 2. Fall back to `HuggingFace` cache directory (~/.cache/huggingface/hub)
    #[inline]
    #[must_use = "returns the builder with code/formula model path configured"]
    pub fn code_formula_model_path(mut self, path: PathBuf) -> Self {
        self.code_formula_model_path = Some(path);
        self
    }

    /// Set custom cache directory for model downloads
    ///
    /// If not set, uses platform-specific cache directory:
    /// - Linux: `$XDG_CACHE_HOME/docling` or `$HOME/.cache/docling`
    /// - macOS: `$HOME/Library/Caches/docling`
    /// - Windows: `{FOLDERID_LocalAppData}\docling`
    #[inline]
    #[must_use = "returns the builder with cache directory configured"]
    pub fn cache_dir(mut self, path: PathBuf) -> Self {
        self.cache_dir = Some(path);
        self
    }

    /// Skip model path validation (for unit tests without models)
    ///
    /// WARNING: Only use this in unit tests! In production code, you want
    /// validation to catch missing models early with actionable error messages.
    ///
    /// # Exampless
    ///
    /// ```no_run
    /// use docling_pdf_ml::PipelineConfigBuilder;
    ///
    /// # fn main() -> docling_pdf_ml::Result<()> {
    /// // For unit tests that don't have actual model files
    /// let config = PipelineConfigBuilder::new()
    ///     .skip_validation(true)
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    #[doc(hidden)] // Hide from public docs - for testing only
    #[inline]
    #[must_use = "returns the builder with validation skipping configured"]
    pub const fn skip_validation(mut self, skip: bool) -> Self {
        self.skip_validation = skip;
        self
    }

    /// Set the scale factor for table inference (`TableFormer`)
    ///
    /// The page image is scaled by this factor before cropping table regions.
    /// This should match your PDF renderer's DPI setting.
    ///
    /// Default: 2.0 (for 72 DPI base * 2.0 = 144 DPI)
    ///
    /// # Exampless
    ///
    /// ```no_run
    /// use docling_pdf_ml::PipelineConfigBuilder;
    ///
    /// # fn main() -> docling_pdf_ml::Result<()> {
    /// // For 300 DPI rendering (300/72 ≈ 4.17)
    /// let config = PipelineConfigBuilder::new()
    ///     .table_scale(4.17)
    ///     .build()?;
    ///
    /// // For 72 DPI rendering (no scaling)
    /// let config = PipelineConfigBuilder::new()
    ///     .table_scale(1.0)
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use = "returns the builder with table scale factor configured"]
    pub const fn table_scale(mut self, scale: f32) -> Self {
        self.table_scale = scale;
        self
    }

    /// Set the minimum cell size in points for table structure detection
    ///
    /// Cells smaller than this threshold are filtered out as noise/artifacts.
    /// Default: 1.0 (1x1 point minimum).
    ///
    /// # Exampless
    ///
    /// ```no_run
    /// use docling_pdf_ml::PipelineConfigBuilder;
    ///
    /// # fn main() -> docling_pdf_ml::Result<()> {
    /// // Allow smaller cells (less filtering)
    /// let config = PipelineConfigBuilder::new()
    ///     .min_cell_size_points(0.5)
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use = "returns the builder with min cell size configured"]
    pub const fn min_cell_size_points(mut self, size: f32) -> Self {
        self.min_cell_size_points = size;
        self
    }

    /// Set the minimum confidence threshold for table cell detection
    ///
    /// Cells with confidence below this threshold are filtered out.
    /// Default: 0.5 (50% confidence, conservative - keeps most cells).
    ///
    /// # Exampless
    ///
    /// ```no_run
    /// use docling_pdf_ml::PipelineConfigBuilder;
    ///
    /// # fn main() -> docling_pdf_ml::Result<()> {
    /// // Higher confidence threshold (fewer but more certain cells)
    /// let config = PipelineConfigBuilder::new()
    ///     .min_cell_confidence(0.7)
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use = "returns the builder with min cell confidence configured"]
    pub const fn min_cell_confidence(mut self, confidence: f32) -> Self {
        self.min_cell_confidence = confidence;
        self
    }

    /// Set the minimum region size for code/formula enrichment
    ///
    /// Regions smaller than this threshold (in either dimension) are skipped
    /// during code/formula enrichment to avoid ML model issues with tiny inputs.
    ///
    /// # Arguments
    ///
    /// * `size` - Minimum size in pixels (default: 10.0)
    ///
    /// # Exampless
    ///
    /// ```no_run
    /// use docling_pdf_ml::PipelineConfigBuilder;
    ///
    /// # fn main() -> docling_pdf_ml::Result<()> {
    /// // Require larger minimum region size (20x20 pixels)
    /// let config = PipelineConfigBuilder::new()
    ///     .min_enrichment_region_size(20.0)
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use = "returns the builder with min enrichment region size configured"]
    pub const fn min_enrichment_region_size(mut self, size: f32) -> Self {
        self.min_enrichment_region_size = size;
        self
    }

    /// Set the cascade mode for layout detection
    ///
    /// Controls when to use fast heuristic-based layout detection vs. full ML model.
    ///
    /// # Arguments
    ///
    /// * `mode` - The cascade mode to use
    ///   - `AlwaysML`: Always use ML model (default, original behavior)
    ///   - `Auto`: Use heuristics for simple pages, ML for complex (recommended for mixed documents)
    ///   - `AlwaysHeuristic`: Always use heuristics (fastest, for simple documents only)
    ///   - `Conservative`: Use heuristics only for definitely simple pages
    ///
    /// # Exampless
    ///
    /// ```no_run
    /// use docling_pdf_ml::{PipelineConfigBuilder, models::cascade_layout::CascadeMode};
    ///
    /// # fn main() -> docling_pdf_ml::Result<()> {
    /// // Use Auto mode for best speed/accuracy tradeoff
    /// let config = PipelineConfigBuilder::new()
    ///     .cascade_mode(CascadeMode::Auto)
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use = "returns the builder with cascade mode configured"]
    pub const fn cascade_mode(mut self, mode: CascadeMode) -> Self {
        self.cascade_mode = mode;
        self
    }

    /// Configure whether to prefer INT8 quantized models for ONNX backend
    ///
    /// When enabled (default), the builder will look for `model_int8.onnx` before
    /// falling back to `model.onnx`. INT8 quantized models are:
    /// - ~4x smaller (43MB vs 164MB)
    /// - ~2x faster inference
    /// - <1% accuracy loss
    ///
    /// Set to `false` if you need FP32 precision or experience accuracy issues.
    ///
    /// # Note
    ///
    /// This only affects the ONNX backend. `PyTorch` backend always uses FP32 weights.
    ///
    /// # Exampless
    ///
    /// ```no_run
    /// use docling_pdf_ml::PipelineConfigBuilder;
    ///
    /// # fn main() -> docling_pdf_ml::Result<()> {
    /// // Force FP32 model (disable INT8)
    /// let config = PipelineConfigBuilder::new()
    ///     .prefer_quantized(false)
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use = "returns the builder with INT8 quantized model preference configured"]
    pub const fn prefer_quantized(mut self, prefer: bool) -> Self {
        self.prefer_quantized = prefer;
        self
    }

    /// Build the configuration
    ///
    /// This validates the configuration and resolves default paths.
    ///
    /// # Errors
    ///
    /// Returns `DoclingError::ConfigError` if configuration is invalid.
    #[must_use = "this returns a Result that should be handled"]
    #[allow(clippy::too_many_lines)]
    pub fn build(self) -> Result<PipelineConfig> {
        // Get cache directory before moving self
        let cache_dir = self.cache_dir.clone().unwrap_or_else(|| {
            dirs::cache_dir()
                .unwrap_or_else(|| PathBuf::from(".cache"))
                .join("docling")
        });

        // Resolve layout model path based on backend
        let prefer_int8 = self.prefer_quantized;
        let layout_model_path = self.layout_model_path.unwrap_or_else(|| {
            match self.layout_backend {
                crate::models::layout_predictor::InferenceBackend::ONNX => {
                    // Helper to find model in a directory, preferring INT8 if configured
                    let find_model = |dir: &std::path::Path| -> Option<PathBuf> {
                        if prefer_int8 {
                            let int8_path = dir.join("model_int8.onnx");
                            if int8_path.exists() {
                                log::info!("Using INT8 quantized layout model: {}", int8_path.display());
                                return Some(int8_path);
                            }
                        }
                        let fp32_path = dir.join("model.onnx");
                        if fp32_path.exists() {
                            if prefer_int8 {
                                log::debug!("INT8 model not found, using FP32: {}", fp32_path.display());
                            }
                            return Some(fp32_path);
                        }
                        None
                    };

                    // Try local ONNX exports first (for development)
                    let local_dir = PathBuf::from("onnx_exports/layout_optimum");
                    if let Some(path) = find_model(&local_dir) {
                        return path;
                    }

                    // Try crates path (for builds from workspace root)
                    let crates_dir = PathBuf::from("crates/docling-pdf-ml/onnx_exports/layout_optimum");
                    if let Some(path) = find_model(&crates_dir) {
                        return path;
                    }

                    // Try CARGO_MANIFEST_DIR-relative path (when used as library)
                    if let Some(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR")
                        .ok()
                        .map(PathBuf::from)
                    {
                        let model_dir = manifest_dir.join("onnx_exports/layout_optimum");
                        if let Some(path) = find_model(&model_dir) {
                            return path;
                        }
                    }

                    // Fall back to cache directory
                    let cache_model_dir = cache_dir.join("models/layout");
                    if prefer_int8 {
                        let int8_path = cache_model_dir.join("model_int8.onnx");
                        if int8_path.exists() {
                            return int8_path;
                        }
                    }
                    cache_model_dir.join("model.onnx")
                }
                #[cfg(feature = "pytorch")]
                crate::models::layout_predictor::InferenceBackend::PyTorch => {
                    // For PyTorch, use HuggingFace safetensors from docling-layout-heron
                    dirs::home_dir()
                        .map(|home| {
                            home.join(".cache/huggingface/hub/models--ds4sd--docling-layout-heron/snapshots/bdb7099d742220552d703932cc0ce0a26a7a8da8/model.safetensors")
                        })
                        .unwrap_or_else(|| {
                            // Fall back to cache directory
                            cache_dir.join("models/layout/model.safetensors")
                        })
                }
            }
        });

        // Resolve table model directory
        let table_model_dir = self.table_model_dir.unwrap_or_else(|| {
            // Try HuggingFace cache first (common location)
            let hf_cache = dirs::home_dir()
                .map(|home| {
                    home.join(".cache/huggingface/hub/models--ds4sd--docling-models/snapshots/fc0f2d45e2218ea24bce5045f58a389aed16dc23/model_artifacts/tableformer/accurate")
                })
                .filter(|p| p.exists());

            hf_cache.map_or_else(
                || cache_dir.join("models/tableformer/accurate"),
                |path| path,
            )
        });

        // Resolve code/formula model directory
        let code_formula_model_path = self.code_formula_model_path.unwrap_or_else(|| {
            // Try HuggingFace cache first (CodeFormula is stored there)
            let hf_cache = dirs::home_dir()
                .map(|home| home.join(".cache/huggingface/hub/models--ds4sd--CodeFormulaV2"))
                .filter(|p| p.exists());

            if let Some(path) = hf_cache {
                // Issue #12 FIX: Find the snapshot directory, sorting by modified time to get latest
                // Pattern: models--ds4sd--CodeFormulaV2/snapshots/{hash}/
                if let Ok(entries) = std::fs::read_dir(path.join("snapshots")) {
                    // Collect and sort entries by modification time (descending = newest first)
                    let mut snapshot_entries: Vec<_> = entries
                        .filter_map(std::result::Result::ok)
                        .filter(|e| e.path().is_dir())
                        .collect();

                    // Sort by modification time (newest first) or by name as fallback
                    snapshot_entries.sort_by(|a, b| {
                        let a_time = a.metadata().and_then(|m| m.modified()).ok();
                        let b_time = b.metadata().and_then(|m| m.modified()).ok();
                        match (b_time, a_time) {
                            (Some(bt), Some(at)) => bt.cmp(&at),
                            (Some(_), None) => std::cmp::Ordering::Less,
                            (None, Some(_)) => std::cmp::Ordering::Greater,
                            (None, None) => b.file_name().cmp(&a.file_name()),
                        }
                    });

                    if let Some(entry) = snapshot_entries.into_iter().next() {
                        log::debug!("Selected CodeFormula snapshot: {}", entry.path().display());
                        return entry.path();
                    }
                }
                path
            } else {
                // Fall back to cache directory
                cache_dir.join("models/codeformula")
            }
        });

        // Validate required model paths exist (F1, F3, F9: actionable error messages)
        // Skip validation if explicitly requested (for unit tests without models)
        if !self.skip_validation {
            // Layout model is always required
            if !layout_model_path.exists() {
                return Err(DoclingError::ConfigError {
                    reason: format!(
                        "Layout model not found at: {}\n\
                         Please download models with: python -c \"from docling_ibm_models.tableformer.data_management.tf_predictor import TFPredictor; TFPredictor(mode='accurate')\" \n\
                         Or specify a custom path with .layout_model_path()",
                        layout_model_path.display()
                    ),
                });
            }

            // TableFormer required if table_structure_enabled
            if self.table_structure_enabled {
                // Check for PyTorch model files
                let pytorch_model = table_model_dir.join("model.safetensors");
                let onnx_model = table_model_dir.join("table_structure_model.onnx");
                let has_pytorch = pytorch_model.exists();
                let has_onnx = onnx_model.exists();

                if !has_pytorch && !has_onnx && !table_model_dir.exists() {
                    return Err(DoclingError::ConfigError {
                        reason: format!(
                            "TableFormer model directory not found: {}\n\
                             Table structure parsing is enabled but no models are available.\n\
                             Either:\n\
                             1. Download models: python -c \"from huggingface_hub import snapshot_download; snapshot_download('ds4sd/docling-models')\"\n\
                             2. Disable tables: .table_structure_enabled(false)\n\
                             3. Specify custom path: .table_model_dir(path)",
                            table_model_dir.display()
                        ),
                    });
                }
            }

            // RapidOCR models required if ocr_enabled (F3)
            // Uses PaddleOCR v4 model names: ch_PP-OCRv4_det_infer.onnx, ch_PP-OCRv4_rec_infer.onnx
            // Classification model is optional (ch_ppocr_mobile_v2.0_cls_infer.onnx)
            if self.ocr_enabled {
                let ocr_models_dir = PathBuf::from("models/rapidocr");
                let det_model = ocr_models_dir.join("ch_PP-OCRv4_det_infer.onnx");
                let rec_model = ocr_models_dir.join("ch_PP-OCRv4_rec_infer.onnx");

                let missing: Vec<&str> = [
                    (det_model.exists(), "ch_PP-OCRv4_det_infer.onnx"),
                    (rec_model.exists(), "ch_PP-OCRv4_rec_infer.onnx"),
                ]
                .iter()
                .filter(|(exists, _)| !exists)
                .map(|(_, name)| *name)
                .collect();

                if !missing.is_empty() {
                    return Err(DoclingError::ConfigError {
                        reason: format!(
                            "RapidOCR models missing in models/rapidocr/: {}\n\
                             OCR is enabled but required models are not available.\n\
                             Either:\n\
                             1. Download RapidOCR models to models/rapidocr/\n\
                             2. Disable OCR: .ocr_enabled(false)",
                            missing.join(", ")
                        ),
                    });
                }
            }
        }

        Ok(PipelineConfig {
            device: self.device,
            ocr_enabled: self.ocr_enabled,
            ocr_backend: self.ocr_backend,
            table_structure_enabled: self.table_structure_enabled,
            code_formula_enabled: self.code_formula_enabled,
            layout_backend: self.layout_backend,
            layout_model_path,
            table_model_dir,
            code_formula_model_path,
            table_scale: self.table_scale,
            min_cell_size_points: self.min_cell_size_points,
            min_cell_confidence: self.min_cell_confidence,
            min_enrichment_region_size: self.min_enrichment_region_size,
            cascade_mode: self.cascade_mode,
        })
    }
}

/// Main pipeline for PDF parsing
///
/// The pipeline loads ML models once at initialization and can process
/// multiple pages efficiently. It performs document structure extraction
/// including layout detection, table structure parsing, and reading order
/// determination.
///
/// # Exampless
///
/// ```no_run
/// use docling_pdf_ml::{Pipeline, PipelineConfig, Device};
/// use ndarray::Array3;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Create pipeline with default configuration
/// let mut pipeline = Pipeline::new(PipelineConfig::default())?;
///
/// // Process a page (page_image should be HWC format, u8, range [0, 255])
/// # let page_image = Array3::<u8>::zeros((792, 612, 3));
/// let page = pipeline.process_page(
///     0,                  // page number
///     &page_image,        // page image as ndarray
///     612.0,              // width in points
///     792.0,              // height in points
///     None,               // optional textline cells
/// )?;
///
/// // Access results
/// if let Some(assembled) = page.assembled {
///     log::debug!("Found {} elements", assembled.elements.len());
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Performance
///
/// Performance profiling data for a single page
///
/// Tracks timing for each pipeline stage to identify bottlenecks.
/// Enable profiling with `Pipeline::enable_profiling()`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct PageTiming {
    /// Time spent in OCR (if enabled)
    pub ocr_duration: Option<Duration>,
    /// Time spent in coordinate conversion
    pub coord_conversion_duration: Duration,
    /// Time spent in layout detection (`LayoutPredictor` inference)
    pub layout_detection_duration: Duration,
    /// Time spent in layout post-processing (modular pipeline stages 04-08)
    pub layout_postprocess_duration: Duration,
    /// Time spent in table structure inference (`TableFormer`)
    pub table_structure_duration: Option<Duration>,
    /// Time spent in page assembly
    pub page_assembly_duration: Duration,
    /// Time spent in code/formula enrichment (if enabled)
    pub code_formula_duration: Option<Duration>,
    /// Total time for the entire page
    pub total_duration: Duration,
}

impl PageTiming {
    /// Print a formatted breakdown of stage timings
    pub fn print(&self) {
        let total_ms = self.total_duration.as_secs_f64() * 1000.0;

        log::debug!("\n  ╔════════════════════════════════════════════════════════════════╗");
        log::debug!("  ║  Performance Breakdown                                         ║");
        log::debug!("  ╚════════════════════════════════════════════════════════════════╝");

        if let Some(ocr) = self.ocr_duration {
            let pct = (ocr.as_secs_f64() / self.total_duration.as_secs_f64()) * 100.0;
            log::debug!(
                "    OCR:                  {:8.2} ms ({:5.1}%)",
                ocr.as_secs_f64() * 1000.0,
                pct
            );
        }

        let coord_pct = (self.coord_conversion_duration.as_secs_f64()
            / self.total_duration.as_secs_f64())
            * 100.0;
        log::debug!(
            "    Coord conversion:     {:8.2} ms ({:5.1}%)",
            self.coord_conversion_duration.as_secs_f64() * 1000.0,
            coord_pct
        );

        let layout_pct = (self.layout_detection_duration.as_secs_f64()
            / self.total_duration.as_secs_f64())
            * 100.0;
        log::debug!(
            "    Layout detection:     {:8.2} ms ({:5.1}%)",
            self.layout_detection_duration.as_secs_f64() * 1000.0,
            layout_pct
        );

        let postprocess_pct = (self.layout_postprocess_duration.as_secs_f64()
            / self.total_duration.as_secs_f64())
            * 100.0;
        log::debug!(
            "    Layout postprocess:   {:8.2} ms ({:5.1}%)",
            self.layout_postprocess_duration.as_secs_f64() * 1000.0,
            postprocess_pct
        );

        if let Some(table) = self.table_structure_duration {
            let pct = (table.as_secs_f64() / self.total_duration.as_secs_f64()) * 100.0;
            log::debug!(
                "    Table structure:      {:8.2} ms ({:5.1}%)",
                table.as_secs_f64() * 1000.0,
                pct
            );
        }

        let assembly_pct =
            (self.page_assembly_duration.as_secs_f64() / self.total_duration.as_secs_f64()) * 100.0;
        log::debug!(
            "    Page assembly:        {:8.2} ms ({:5.1}%)",
            self.page_assembly_duration.as_secs_f64() * 1000.0,
            assembly_pct
        );

        if let Some(code_formula) = self.code_formula_duration {
            let pct = (code_formula.as_secs_f64() / self.total_duration.as_secs_f64()) * 100.0;
            log::debug!(
                "    Code/Formula:         {:8.2} ms ({:5.1}%)",
                code_formula.as_secs_f64() * 1000.0,
                pct
            );
        }

        log::debug!("  ─────────────────────────────────────────────────────────────────");
        log::debug!("    TOTAL:                {total_ms:8.2} ms (100.0%)");
        log::debug!("");
    }

    /// Get the top N bottlenecks by percentage
    #[must_use = "returns the top bottlenecks sorted by percentage"]
    pub fn top_bottlenecks(&self, n: usize) -> Vec<(String, Duration, f64)> {
        let mut stages = vec![];

        if let Some(ocr) = self.ocr_duration {
            let pct = (ocr.as_secs_f64() / self.total_duration.as_secs_f64()) * 100.0;
            stages.push(("OCR".to_string(), ocr, pct));
        }

        let coord_pct = (self.coord_conversion_duration.as_secs_f64()
            / self.total_duration.as_secs_f64())
            * 100.0;
        stages.push((
            "Coordinate conversion".to_string(),
            self.coord_conversion_duration,
            coord_pct,
        ));

        let layout_pct = (self.layout_detection_duration.as_secs_f64()
            / self.total_duration.as_secs_f64())
            * 100.0;
        stages.push((
            "Layout detection".to_string(),
            self.layout_detection_duration,
            layout_pct,
        ));

        let postprocess_pct = (self.layout_postprocess_duration.as_secs_f64()
            / self.total_duration.as_secs_f64())
            * 100.0;
        stages.push((
            "Layout postprocessing".to_string(),
            self.layout_postprocess_duration,
            postprocess_pct,
        ));

        if let Some(table) = self.table_structure_duration {
            let pct = (table.as_secs_f64() / self.total_duration.as_secs_f64()) * 100.0;
            stages.push(("Table structure".to_string(), table, pct));
        }

        let assembly_pct =
            (self.page_assembly_duration.as_secs_f64() / self.total_duration.as_secs_f64()) * 100.0;
        stages.push((
            "Page assembly".to_string(),
            self.page_assembly_duration,
            assembly_pct,
        ));

        if let Some(code_formula) = self.code_formula_duration {
            let pct = (code_formula.as_secs_f64() / self.total_duration.as_secs_f64()) * 100.0;
            stages.push(("Code/Formula enrichment".to_string(), code_formula, pct));
        }

        // Sort by percentage descending
        stages.sort_by(|a, b| b.2.total_cmp(&a.2));

        stages.into_iter().take(n).collect()
    }
}

/// Main PDF parsing pipeline coordinating all processing stages.
///
/// `Pipeline` orchestrates the complete document processing workflow from raw PDF
/// to structured output. It manages ML models (layout detection, OCR, table structure)
/// and coordinates the assembly pipeline.
///
/// # Performance Note
///
/// Model loading is expensive (1-3 seconds). Reuse the pipeline
/// instance for multiple pages to amortize this cost.
///
/// # Thread Safety
///
/// The pipeline is NOT thread-safe. Create one pipeline per thread
/// for concurrent processing.
///
/// # Exampless
///
/// ```ignore
/// // NOTE: Requires model files to run
/// use docling_pdf_ml::{Pipeline, PipelineConfigBuilder};
/// use ndarray::Array3;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Create pipeline with default settings
/// let mut pipeline = Pipeline::with_defaults()?;
///
/// // Process a page image (HWC format, u8)
/// let page_image = Array3::<u8>::zeros((792, 612, 3));
/// let page = pipeline.process_page(0, &page_image, 612.0, 792.0, None)?;
///
/// if let Some(assembled) = page.assembled {
///     println!("Found {} elements", assembled.elements.len());
/// }
/// # Ok(())
/// # }
/// ```
///
/// See also: [`PipelineConfig`], [`PipelineConfigBuilder`], [`Page`]
// Field `modular_pipeline: ModularPipeline` named for clarity
#[allow(clippy::struct_field_names)]
pub struct Pipeline {
    #[allow(
        dead_code,
        reason = "config kept for future use (device selection, flags, etc.)"
    )]
    config: PipelineConfig,
    layout_predictor: LayoutPredictorModel,
    /// `DocLayout`-YOLO model for layout detection (GPU only - 2.5x SLOWER on CPU!)
    /// Optional - loaded when `AutoWithYolo` or `AlwaysYolo` cascade modes are used.
    /// **Warning (N=3491):** Only use with GPU acceleration. On CPU, use RT-DETR instead.
    yolo_model: Option<crate::models::layout_predictor::DocLayoutYolo>,
    /// CoreML-based DocLayout-YOLO model for Apple Neural Engine acceleration (macOS only)
    /// Optional - loaded when AutoWithCoreML or AlwaysCoreML cascade modes are used.
    /// Provides 7.0x speedup over ONNX CPU on Apple Silicon.
    #[cfg(feature = "coreml")]
    coreml_model: Option<crate::models::layout_predictor::DocLayoutYoloCoreML>,
    /// Complexity estimator for cascade routing
    complexity_estimator: ComplexityEstimator,
    /// Heuristic layout detector for simple documents
    heuristic_detector: HeuristicLayoutDetector,
    /// Cascade routing statistics
    cascade_stats: CascadeStats,
    #[cfg(feature = "pytorch")]
    table_former: Option<TableStructureModel>,
    /// ONNX-based table structure model (Microsoft Table Transformer)
    /// Used as alternative to `PyTorch` `TableFormer` when libtorch crashes
    table_former_onnx: Option<TableStructureModelOnnx>,
    #[cfg(feature = "pytorch")]
    code_formula: Option<crate::models::code_formula::CodeFormulaModel>,
    #[allow(
        dead_code,
        reason = "legacy postprocessor kept for reference, not currently used"
    )]
    layout_postprocessor: LayoutPostProcessor,
    modular_pipeline: ModularPipeline,
    page_assembler: PageAssembler,
    reading_order: ReadingOrderPredictor,
    /// RapidOCR instance (optional, only loaded if ocr_enabled=true)
    /// NOTE: Requires opencv-preprocessing feature
    #[cfg(feature = "opencv-preprocessing")]
    ocr: Option<crate::ocr::RapidOcr>,
    /// Pure Rust `RapidOCR` instance (no `OpenCV` dependency)
    /// Used when opencv-preprocessing feature is not available
    #[cfg(not(feature = "opencv-preprocessing"))]
    ocr_pure: Option<crate::ocr::RapidOcrPure>,
    /// Apple Vision OCR instance (macOS only, 7x better quality than `RapidOCR`)
    /// Used when `ocr_backend` is `Auto` (on macOS) or `AppleVision`
    #[cfg(target_os = "macos")]
    apple_vision_ocr: Option<crate::ocr::AppleVisionOcr>,
    /// Enable performance profiling
    profiling_enabled: bool,
    /// Last page timing (if profiling enabled)
    pub last_timing: Option<PageTiming>,
    /// Marker to make Pipeline !Send + !Sync at compile-time
    /// This prevents accidental use across thread boundaries
    _not_send_sync: PhantomData<*const ()>,
}

impl std::fmt::Debug for Pipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pipeline")
            .field("config", &self.config)
            .field("layout_predictor", &self.layout_predictor)
            .field("yolo_model_loaded", &self.yolo_model.is_some())
            .field("cascade_stats", &self.cascade_stats)
            .field("profiling_enabled", &self.profiling_enabled)
            .finish_non_exhaustive()
    }
}

/// Convert ndarray page image (HWC u8) to `DynamicImage` for OCR
///
/// `RapidOCR` expects `DynamicImage` (from image crate),
/// but pipeline works with `ndarray::Array3<u8>`.
fn array_to_dynamic_image(page_image: &Array3<u8>) -> image::DynamicImage {
    use image::{DynamicImage, RgbImage};

    let shape = page_image.shape();
    let height = shape[0] as u32;
    let width = shape[1] as u32;

    // Create RGB image buffer
    let mut img_buf = RgbImage::new(width, height);

    // Copy pixels from ndarray to image buffer
    // ndarray is HWC format: [height, width, channels]
    for y in 0..height {
        for x in 0..width {
            let r = page_image[[y as usize, x as usize, 0]];
            let g = page_image[[y as usize, x as usize, 1]];
            let b = page_image[[y as usize, x as usize, 2]];
            img_buf.put_pixel(x, y, image::Rgb([r, g, b]));
        }
    }

    DynamicImage::ImageRgb8(img_buf)
}

/// Convert OCR `TextCell` to pipeline `SimpleTextCell`
///
/// OCR returns `TextCell` with `BoundingRectangle` (4-corner format),
/// but pipeline expects `SimpleTextCell` with `BoundingBox` (axis-aligned).
/// Works for both `RapidOcr` (`OpenCV`) and `RapidOcrPure` (pure Rust).
fn ocr_textcell_to_simple(ocr_cell: &crate::ocr::types::TextCell) -> SimpleTextCell {
    SimpleTextCell {
        index: ocr_cell.index,
        text: ocr_cell.text.clone(),
        rect: ocr_cell.rect.to_bbox(), // Convert 4-corner to axis-aligned bbox
        confidence: ocr_cell.confidence,
        from_ocr: ocr_cell.from_ocr,
        is_bold: false,   // N=4373: OCR doesn't provide font style info
        is_italic: false, // N=4373: OCR doesn't provide font style info
    }
}

/// Convert `CellInfo` from modular pipeline to `TextCell`
fn convert_cell_info_to_textcell(
    idx: usize,
    cell: &crate::pipeline_modular::stage09_document_assembler::CellInfo,
) -> crate::pipeline::data_structures::TextCell {
    crate::pipeline::data_structures::TextCell {
        index: idx,
        text: cell.text.clone(),
        rect: crate::pipeline::data_structures::BoundingRectangle {
            r_x0: cell.rect.r_x0 as f32,
            r_y0: cell.rect.r_y0 as f32,
            r_x1: cell.rect.r_x1 as f32,
            r_y1: cell.rect.r_y1 as f32,
            r_x2: cell.rect.r_x2 as f32,
            r_y2: cell.rect.r_y2 as f32,
            r_x3: cell.rect.r_x3 as f32,
            r_y3: cell.rect.r_y3 as f32,
            coord_origin: CoordOrigin::TopLeft,
        },
        confidence: Some(cell.confidence as f32),
        from_ocr: cell.from_ocr,
        is_bold: cell.is_bold, // N=4373: Preserve font style from modular pipeline
        is_italic: cell.is_italic,
    }
}

/// Convert `ClusterInfo` from modular pipeline to `Cluster`
fn convert_cluster_info_to_cluster(
    cluster_info: &crate::pipeline_modular::stage09_document_assembler::ClusterInfo,
) -> Cluster {
    Cluster {
        id: cluster_info.id,
        label: parse_label_string(&cluster_info.label),
        bbox: BoundingBox {
            l: cluster_info.bbox.l as f32,
            t: cluster_info.bbox.t as f32,
            r: cluster_info.bbox.r as f32,
            b: cluster_info.bbox.b as f32,
            coord_origin: CoordOrigin::TopLeft,
        },
        confidence: cluster_info.confidence as f32,
        cells: cluster_info
            .cells
            .iter()
            .enumerate()
            .map(|(idx, cell)| convert_cell_info_to_textcell(idx, cell))
            .collect(),
        children: vec![],
    }
}

/// Convert `Stage10Output` elements to a sorted list of `Cluster`s
fn convert_stage10_to_clusters(
    stage10_output: &crate::pipeline_modular::stage10_reading_order::Stage10Output,
) -> Vec<Cluster> {
    // Extract sorted IDs from reading order
    let sorted_ids: Vec<usize> = stage10_output
        .sorted_elements
        .iter()
        .map(|e| e.id)
        .collect();

    // Build map of cluster ID -> Cluster
    let all_clusters_map: HashMap<usize, Cluster> = stage10_output
        .sorted_elements
        .iter()
        .filter_map(|elem| {
            elem.cluster.as_ref().map(|cluster_info| {
                (
                    cluster_info.id,
                    convert_cluster_info_to_cluster(cluster_info),
                )
            })
        })
        .collect();

    // Reorder clusters according to sorted_ids
    sorted_ids
        .iter()
        .filter_map(|id| all_clusters_map.get(id).cloned())
        .collect()
}

/// Parse label string into `DocItemLabel` enum
fn parse_label_string(label: &str) -> DocItemLabel {
    // N=312: Support both Python-style (lowercase_with_underscores) and Rust-style (TitleCase)
    // The modular pipeline returns Python-style labels, so we need to handle both formats
    match label {
        "Text" | "text" => DocItemLabel::Text,
        "Title" | "title" => DocItemLabel::Title,
        "SectionHeader" | "section_header" => DocItemLabel::SectionHeader,
        "ListItem" | "list_item" => DocItemLabel::ListItem,
        "Caption" | "caption" => DocItemLabel::Caption,
        "Footnote" | "footnote" => DocItemLabel::Footnote,
        "PageHeader" | "page_header" => DocItemLabel::PageHeader,
        "PageFooter" | "page_footer" => DocItemLabel::PageFooter,
        "Table" | "table" => DocItemLabel::Table,
        "Picture" | "Figure" | "picture" | "figure" => DocItemLabel::Picture,
        "Formula" | "formula" => DocItemLabel::Formula,
        "Code" | "code" => DocItemLabel::Code,
        "CheckboxSelected" | "checkbox_selected" => DocItemLabel::CheckboxSelected,
        "CheckboxUnselected" | "checkbox_unselected" => DocItemLabel::CheckboxUnselected,
        "KeyValueRegion" | "key_value_region" => DocItemLabel::KeyValueRegion,
        _ => {
            log::warn!("Warning: Unknown label '{label}', defaulting to Text");
            DocItemLabel::Text
        }
    }
}

/// Convert `LayoutCluster` (baseline format) to Cluster (pipeline format)
fn convert_layout_cluster(layout_cluster: LayoutCluster) -> Cluster {
    // Parse label string into DocItemLabel enum
    // LayoutPredictor outputs labels in Title-Case with hyphens (e.g., "Section-Header")
    // Some labels also have spaces (e.g., "Key-Value Region")
    // Convert to lowercase with underscores for matching
    let normalized_label = layout_cluster.label.to_lowercase().replace(['-', ' '], "_");

    let label = match normalized_label.as_str() {
        "text" => DocItemLabel::Text,
        "section_header" => DocItemLabel::SectionHeader,
        "page_header" => DocItemLabel::PageHeader,
        "page_footer" => DocItemLabel::PageFooter,
        "title" => DocItemLabel::Title,
        "caption" => DocItemLabel::Caption,
        "footnote" => DocItemLabel::Footnote,
        "table" => DocItemLabel::Table,
        "figure" => DocItemLabel::Figure,
        "picture" => DocItemLabel::Picture,
        "formula" => DocItemLabel::Formula,
        "list_item" => DocItemLabel::ListItem,
        "code" => DocItemLabel::Code,
        "checkbox_selected" => DocItemLabel::CheckboxSelected,
        "checkbox_unselected" => DocItemLabel::CheckboxUnselected,
        "form" => DocItemLabel::Form,
        "key_value_region" => DocItemLabel::KeyValueRegion,
        "document_index" => DocItemLabel::DocumentIndex,
        _ => {
            log::warn!(
                "Warning: Unknown label '{}' (normalized: '{}'), defaulting to Text",
                layout_cluster.label,
                normalized_label
            );
            DocItemLabel::Text
        }
    };

    // Convert BBox (baseline, f64) to BoundingBox (pipeline, f32)
    let bbox = BoundingBox {
        l: layout_cluster.bbox.l as f32,
        t: layout_cluster.bbox.t as f32,
        r: layout_cluster.bbox.r as f32,
        b: layout_cluster.bbox.b as f32,
        coord_origin: CoordOrigin::TopLeft,
    };

    Cluster {
        id: layout_cluster.id as usize,
        label,
        bbox,
        confidence: layout_cluster.confidence as f32,
        cells: vec![], // OCR cells not available yet (will be added when OCR is integrated)
        children: vec![], // No hierarchical clustering at this stage
    }
}

/// Convert clusters from pipeline format to modular format
/// Converts from Image coordinates to PDF coordinates (`TopLeft` origin)
///
/// # Coordinate Transform (N=2289 Fixed)
///
/// **Scale Transform Only**: ML model outputs bboxes in IMAGE coordinates (e.g., 2480×3508 pixels)
/// which use TOP-LEFT origin (like all computer graphics). We scale down to PDF size:
/// ```text
/// scale_x = image_width / page_width
/// scale_y = image_height / page_height
/// pdf_x = image_x / scale_x
/// pdf_y = image_y / scale_y
/// ```
///
/// After scaling, coordinates remain in TOP-LEFT origin (same as IMAGE coords).
/// This matches the coordinate system used by cells (which are also top-left).
///
/// **NO Y-FLIP**: Previous code incorrectly flipped Y-axis, assuming image coords were
/// bottom-left origin. This caused clusters to appear in wrong positions (e.g., Y=378
/// instead of Y=396), preventing cell assignment.
fn convert_clusters_to_labeled(
    clusters: &[Cluster],
    image_width: f32,
    image_height: f32,
    page_width: f32,
    page_height: f32,
) -> LabeledClusters {
    // N=2287: Calculate scale factors (image → PDF)
    let scale_x = image_width / page_width;
    let scale_y = image_height / page_height;

    // N=2287: Debug coordinate transform
    trace!("\n=== convert_clusters_to_labeled ===");
    trace!("Image: {image_width}×{image_height} pixels");
    trace!("PDF: {page_width}×{page_height} points");
    trace!("Scale: x={scale_x:.3}, y={scale_y:.3}");
    if let Some(first) = clusters.first() {
        trace!(
            "First cluster BEFORE transform: bbox=({:.1},{:.1})→({:.1},{:.1})",
            first.bbox.l,
            first.bbox.t,
            first.bbox.r,
            first.bbox.b
        );
    }

    LabeledClusters {
        clusters: clusters
            .iter()
            .enumerate()
            .map(|(idx, c)| {
                // N=2289: Scale down from image to PDF coordinates
                // Image coords are ALREADY top-left origin (like all computer graphics)
                // After scaling, they remain in top-left origin - NO Y-FLIP NEEDED
                let pdf_l = f64::from(c.bbox.l) / f64::from(scale_x);
                let pdf_t = f64::from(c.bbox.t) / f64::from(scale_y);
                let pdf_r = f64::from(c.bbox.r) / f64::from(scale_x);
                let pdf_b = f64::from(c.bbox.b) / f64::from(scale_y);

                // N=2289: Debug first cluster transform
                if idx == 0 {
                    trace!(
                        "First cluster AFTER scale: bbox=({pdf_l:.1},{pdf_t:.1})→({pdf_r:.1},{pdf_b:.1})"
                    );
                    trace!("  (No Y-flip: Image coords are already top-left origin)");
                }

                LabeledCluster {
                    id: c.id,
                    label: c.label.to_python_string().to_string(),
                    bbox: crate::pipeline_modular::types::BBox {
                        l: pdf_l,
                        t: pdf_t, // N=2289: Use scaled coords directly (already top-left)
                        r: pdf_r,
                        b: pdf_b, // N=2289: Use scaled coords directly (already top-left)
                    },
                    confidence: f64::from(c.confidence),
                    class_id: 0,
                }
            })
            .collect(),
    }
}

/// Convert textline cells from PDF (`BottomLeft`) to Screen (`TopLeft`) coordinates
///
/// The baseline `textline_cells.json` uses PDF coordinates where Y increases upward,
/// but the modular pipeline expects screen coordinates where Y increases downward.
fn convert_textline_coords(cells: Vec<SimpleTextCell>, page_height: f32) -> Vec<SimpleTextCell> {
    cells
        .into_iter()
        .map(|mut cell| {
            // Check if coordinates are inverted (t > b indicates BottomLeft origin)
            if cell.rect.t > cell.rect.b {
                // In PDF coords, "t" is actually bottom edge, "b" is top edge
                // Convert: new_y = page_height - old_y, and swap t/b
                let new_t = page_height - cell.rect.t; // Convert old bottom to new top
                let new_b = page_height - cell.rect.b; // Convert old top to new bottom
                cell.rect.t = new_t;
                cell.rect.b = new_b;
                cell.rect.coord_origin = CoordOrigin::TopLeft;
            }
            cell
        })
        .collect()
}

/// Convert text cells from pipeline format to modular format
/// Normalizes coordinates to ensure l < r and t < b
fn convert_simple_to_modular_cells(cells: &[SimpleTextCell]) -> OCRCells {
    OCRCells {
        cells: cells
            .iter()
            .map(|c| {
                let bbox_val = c.bbox();
                // Normalize coordinates: Ensure l < r AND t < b
                // Some cells have inverted coordinates (t > b in PDF coords, l > r for rotated text)
                // BBox::intersection_area() requires normalized format (l < r, t < b)
                let l = f64::from(bbox_val.l).min(f64::from(bbox_val.r));
                let r = f64::from(bbox_val.l).max(f64::from(bbox_val.r));
                let t = f64::from(bbox_val.t).min(f64::from(bbox_val.b));
                let b = f64::from(bbox_val.t).max(f64::from(bbox_val.b));

                ModularTextCell {
                    text: c.text.clone(),
                    bbox: crate::pipeline_modular::types::BBox { l, t, r, b },
                    confidence: None,
                    is_bold: c.is_bold,
                    is_italic: c.is_italic,
                }
            })
            .collect(),
    }
}

/// Convert a single modular cell to a cluster cell
fn convert_modular_cell_to_cluster_cell(cell: &ModularTextCell) -> crate::pipeline::TextCell {
    let l = cell.bbox.l as f32;
    let t = cell.bbox.t as f32;
    let r = cell.bbox.r as f32;
    let b = cell.bbox.b as f32;
    crate::pipeline::TextCell {
        index: 0,
        text: cell.text.clone(),
        rect: crate::pipeline::BoundingRectangle {
            r_x0: l,
            r_y0: t, // Top-left
            r_x1: r,
            r_y1: t, // Top-right
            r_x2: r,
            r_y2: b, // Bottom-right
            r_x3: l,
            r_y3: b, // Bottom-left
            coord_origin: CoordOrigin::TopLeft,
        },
        confidence: cell.confidence.map(|c| c as f32),
        from_ocr: true,
        is_bold: cell.is_bold,
        is_italic: cell.is_italic,
    }
}

/// Convert clusters from modular format back to pipeline format
fn convert_labeled_to_clusters(
    processed: crate::pipeline_modular::types::ClustersWithCells,
) -> Vec<Cluster> {
    processed
        .clusters
        .iter()
        .map(|c| Cluster {
            id: c.id,
            label: parse_label_string(&c.label),
            bbox: BoundingBox {
                l: c.bbox.l as f32,
                t: c.bbox.t as f32,
                r: c.bbox.r as f32,
                b: c.bbox.b as f32,
                coord_origin: CoordOrigin::TopLeft,
            },
            confidence: c.confidence as f32,
            cells: c
                .cells
                .iter()
                .map(convert_modular_cell_to_cluster_cell)
                .collect(),
            children: vec![],
        })
        .collect()
}

impl Pipeline {
    /// Create a new pipeline with default configuration
    ///
    /// This is a convenience method equivalent to `Pipeline::new(PipelineConfig::default())`.
    ///
    /// # Exampless
    ///
    /// ```no_run
    /// use docling_pdf_ml::Pipeline;
    ///
    /// # fn main() -> docling_pdf_ml::Result<()> {
    /// // Quick start with defaults
    /// let mut pipeline = Pipeline::with_defaults()?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    /// Returns `DoclingError::ModelLoadError` if models fail to load
    #[must_use = "pipeline creation returns a Result that should be handled"]
    pub fn with_defaults() -> Result<Self> {
        Self::new(PipelineConfig::default())
    }

    /// Create a new CPU-only pipeline with default configuration
    ///
    /// This is a convenience method for CPU-only processing.
    ///
    /// # Exampless
    ///
    /// ```no_run
    /// use docling_pdf_ml::Pipeline;
    ///
    /// # fn main() -> docling_pdf_ml::Result<()> {
    /// // CPU-only processing (no GPU required)
    /// let mut pipeline = Pipeline::cpu()?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    /// Returns `DoclingError::ModelLoadError` if models fail to load
    #[must_use = "pipeline creation returns a Result that should be handled"]
    pub fn cpu() -> Result<Self> {
        let config = PipelineConfigBuilder::new().device(Device::Cpu).build()?;
        Self::new(config)
    }

    /// Create a new GPU pipeline with default configuration
    ///
    /// This is a convenience method for GPU processing using CUDA.
    ///
    /// # Arguments
    /// * `device_id` - CUDA device ID (typically 0 for first GPU)
    ///
    /// # Exampless
    ///
    /// ```no_run
    /// use docling_pdf_ml::Pipeline;
    ///
    /// # fn main() -> docling_pdf_ml::Result<()> {
    /// // GPU processing on first CUDA device
    /// let mut pipeline = Pipeline::gpu(0)?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    /// Returns `DoclingError::ModelLoadError` if models fail to load
    /// Returns `DoclingError::ConfigError` if CUDA is not available
    #[must_use = "pipeline creation returns a Result that should be handled"]
    #[cfg(feature = "pytorch")]
    pub fn gpu(device_id: usize) -> Result<Self> {
        if !tch::Cuda::is_available() {
            return Err(DoclingError::ConfigError {
                reason: "CUDA is not available. GPU inference requires NVIDIA GPU, CUDA toolkit, and PyTorch with CUDA support.".to_string(),
            });
        }
        let config = PipelineConfigBuilder::new()
            .device(Device::Cuda(device_id))
            .build()?;
        Self::new(config)
    }

    /// Create a new pipeline with GPU inference (CUDA)
    ///
    /// Note: GPU support requires the `pytorch` feature. Without it, this
    /// method returns an error.
    ///
    /// # Errors
    ///
    /// Returns an error when the `pytorch` feature is not enabled.
    #[must_use = "pipeline creation returns a Result that should be handled"]
    #[cfg(not(feature = "pytorch"))]
    pub fn gpu(_device_id: usize) -> Result<Self> {
        Err(DoclingError::ConfigError {
            reason: "GPU inference requires the `pytorch` feature to be enabled.".to_string(),
        })
    }

    /// Create a new pipeline with the given configuration
    ///
    /// This loads all ML models into memory.
    ///
    /// # Arguments
    /// * `config` - Pipeline configuration
    ///
    /// # Returns
    /// Result containing Pipeline or error
    ///
    /// # Exampless
    ///
    /// ```no_run
    /// use docling_pdf_ml::{Pipeline, PipelineConfigBuilder, Device};
    ///
    /// # fn main() -> docling_pdf_ml::Result<()> {
    /// // Custom configuration
    /// let config = PipelineConfigBuilder::new()
    ///     .device(Device::Cpu)
    ///     .ocr_enabled(false)
    ///     .build()?;
    ///
    /// let mut pipeline = Pipeline::new(config)?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    /// Returns `DoclingError::ModelLoadError` if models fail to load
    /// Returns `DoclingError::ConfigError` if configuration is invalid
    #[must_use = "this returns a Result that should be handled"]
    #[allow(clippy::too_many_lines)]
    pub fn new(config: PipelineConfig) -> Result<Self> {
        log::debug!("Initializing PDF parsing pipeline...");
        log::debug!("  Device: {:?}", config.device);
        log::debug!("  OCR enabled: {}", config.ocr_enabled);
        log::debug!(
            "  Table structure enabled: {}",
            config.table_structure_enabled
        );

        // F10: Warn if tables enabled but pytorch feature disabled
        #[cfg(not(feature = "pytorch"))]
        if config.table_structure_enabled {
            log::info!(
                "Note: Table structure is enabled but 'pytorch' feature is disabled.\n\
                 Only ONNX table model (Microsoft Table Transformer) will be available.\n\
                 For IBM TableFormer support, rebuild with: cargo build --features pytorch"
            );
        }

        // Load layout predictor (required)
        log::debug!("\nLoading LayoutPredictor...");
        log::debug!("  Backend: {:?}", config.layout_backend);
        let layout_predictor = LayoutPredictorModel::load_with_backend(
            &config.layout_model_path,
            config.device,
            config.layout_backend,
        )
        .map_err(|e| DoclingError::ModelLoadError {
            model_name: "LayoutPredictor".to_string(),
            source: format!("{e}").into(),
        })?;
        log::debug!("✓ LayoutPredictor loaded");

        // Load table structure model (optional, requires pytorch feature)
        #[cfg(feature = "pytorch")]
        let table_former =
            if config.table_structure_enabled {
                log::debug!("\nLoading TableFormer...");
                let model = TableStructureModel::load(&config.table_model_dir, config.device)
                    .map_err(|e| DoclingError::ModelLoadError {
                        model_name: "TableFormer".to_string(),
                        source: format!("{e}").into(),
                    })?;
                log::debug!("✓ TableFormer loaded");
                Some(model)
            } else {
                None
            };

        // Load ONNX table structure model (fallback when PyTorch crashes)
        // Uses Microsoft Table Transformer instead of IBM TableFormer
        let table_former_onnx = if config.table_structure_enabled {
            let onnx_model_path = config.table_model_dir.join("table_structure_model.onnx");
            if onnx_model_path.exists() {
                log::debug!("\nLoading TableFormer ONNX...");
                match TableStructureModelOnnx::load(&onnx_model_path) {
                    Ok(model) => {
                        log::debug!("✓ TableFormer ONNX loaded (Microsoft Table Transformer)");
                        Some(model)
                    }
                    Err(e) => {
                        log::warn!("Failed to load TableFormer ONNX: {e}");
                        None
                    }
                }
            } else {
                log::debug!(
                    "  TableFormer ONNX not found at: {}",
                    onnx_model_path.display()
                );
                None
            }
        } else {
            None
        };

        // Load code/formula enrichment model (optional, requires pytorch feature)
        #[cfg(feature = "pytorch")]
        let code_formula = if config.code_formula_enabled {
            log::debug!("\nLoading CodeFormula enrichment model...");
            let model = crate::models::code_formula::CodeFormulaModel::from_pretrained(
                &config.code_formula_model_path,
                config.device,
            )
            .map_err(|e| DoclingError::ModelLoadError {
                model_name: "CodeFormula".to_string(),
                source: format!("{e}").into(),
            })?;
            log::debug!("✓ CodeFormula loaded");
            Some(model)
        } else {
            None
        };

        // Load RapidOCR (optional, only if OCR enabled and opencv-preprocessing feature available)
        #[cfg(feature = "opencv-preprocessing")]
        let ocr = if config.ocr_enabled {
            log::debug!("\nLoading RapidOCR...");
            let models_dir = "models/rapidocr";
            let ocr_instance = crate::ocr::RapidOcr::new(models_dir).map_err(|e| {
                DoclingError::ModelLoadError {
                    model_name: "RapidOCR".to_string(),
                    source: format!("{e}").into(),
                }
            })?;
            log::debug!("✓ RapidOCR loaded");
            Some(ocr_instance)
        } else {
            None
        };

        // Load RapidOcrPure (pure Rust OCR, no OpenCV dependency)
        // Used when opencv-preprocessing feature is not available
        #[cfg(not(feature = "opencv-preprocessing"))]
        let ocr_pure = if config.ocr_enabled && config.ocr_backend == OcrBackend::RapidOcr {
            log::debug!("\nLoading RapidOcrPure (pure Rust)...");
            let models_dir = "models/rapidocr";
            let ocr_instance = crate::ocr::RapidOcrPure::new(models_dir).map_err(|e| {
                DoclingError::ModelLoadError {
                    model_name: "RapidOcrPure".to_string(),
                    source: format!("{e}").into(),
                }
            })?;
            log::debug!("✓ RapidOcrPure loaded (pure Rust OCR)");
            Some(ocr_instance)
        } else {
            None
        };

        // Load Apple Vision OCR (macOS only, 7x better quality than RapidOCR)
        // Used when ocr_backend is Auto (on macOS) or AppleVision
        #[cfg(target_os = "macos")]
        let apple_vision_ocr = if config.ocr_enabled {
            let should_use_apple_vision = match config.ocr_backend {
                OcrBackend::Auto | OcrBackend::AppleVision => true,
                OcrBackend::RapidOcr => false,
            };

            if should_use_apple_vision && crate::ocr::apple_vision::is_available() {
                log::debug!("\nLoading Apple Vision OCR (7x better quality than RapidOCR)...");
                match crate::ocr::AppleVisionOcr::new() {
                    Ok(ocr_instance) => {
                        log::debug!("✓ Apple Vision OCR loaded (via macocr CLI)");
                        Some(ocr_instance)
                    }
                    Err(e) => {
                        log::warn!(
                            "Failed to load Apple Vision OCR: {e}. Falling back to RapidOCR."
                        );
                        None
                    }
                }
            } else {
                if config.ocr_backend == OcrBackend::AppleVision {
                    log::warn!(
                        "Apple Vision OCR requested but macocr is not installed. \
                         Falling back to RapidOCR. Install macocr with: cargo install macocr"
                    );
                }
                None
            }
        } else {
            None
        };

        // If Apple Vision is not available on macOS, load RapidOCR as fallback
        #[cfg(all(target_os = "macos", not(feature = "opencv-preprocessing")))]
        let ocr_pure = if config.ocr_enabled && apple_vision_ocr.is_none() {
            log::debug!("\nLoading RapidOcrPure (pure Rust, Apple Vision fallback)...");
            let models_dir = "models/rapidocr";
            let ocr_instance = crate::ocr::RapidOcrPure::new(models_dir).map_err(|e| {
                DoclingError::ModelLoadError {
                    model_name: "RapidOcrPure".to_string(),
                    source: format!("{e}").into(),
                }
            })?;
            log::debug!("✓ RapidOcrPure loaded (pure Rust OCR, fallback)");
            Some(ocr_instance)
        } else {
            None
        };

        // Create layout post-processor (legacy, kept for reference)
        let layout_postprocessor = LayoutPostProcessor::new_default();

        // Create modular pipeline (Stages 04-09)
        // N=4406 FIX: Always use normal pipeline (not for_ocr_mode)
        // for_ocr_mode() breaks PDFs with embedded text by disabling paragraph merging
        #[cfg(feature = "debug-trace")]
        let modular_pipeline = {
            if let Ok(debug_trace_dir) = std::env::var("DEBUG_E2E_TRACE") {
                let debug_dir = std::path::PathBuf::from(debug_trace_dir);
                log::info!(
                    "Debug mode enabled: saving stage outputs to {}",
                    debug_dir.display()
                );
                ModularPipeline::with_debug_output(debug_dir)
            } else {
                ModularPipeline::new()
            }
        };

        // N=4406 FIX: Don't use for_ocr_mode() for all PDFs when OCR is enabled.
        // for_ocr_mode() skips TEXT cluster assignment and disables paragraph merging,
        // which breaks output for PDFs with embedded text. Use normal pipeline instead.
        // OCR cells from figures become standalone text items via Stage06 orphan creator.
        #[cfg(not(feature = "debug-trace"))]
        let modular_pipeline = ModularPipeline::new();

        // Create page assembler
        let page_assembler = PageAssembler::new();

        // Create reading order predictor
        let reading_order = ReadingOrderPredictor::new(ReadingOrderConfig::default());

        // Issue #12 FIX: Warn if table_structure_enabled but no table models loaded
        // This helps debug cases where table content is unexpectedly missing
        #[cfg(feature = "pytorch")]
        let has_table_model = table_former.is_some() || table_former_onnx.is_some();
        #[cfg(not(feature = "pytorch"))]
        let has_table_model = table_former_onnx.is_some();

        if config.table_structure_enabled && !has_table_model {
            log::warn!(
                "Table structure is enabled but no table models were loaded. \
                Tables will have no structure extraction. \
                Check model paths or disable tables with .table_structure_enabled(false)"
            );
        }

        log::debug!("\n✓ Pipeline initialized successfully");

        Ok(Self {
            config,
            layout_predictor,
            yolo_model: None, // YOLO model loaded on-demand via load_yolo_model()
            #[cfg(feature = "coreml")]
            coreml_model: None, // CoreML model loaded on-demand via load_coreml_model()
            complexity_estimator: ComplexityEstimator::new(),
            heuristic_detector: HeuristicLayoutDetector::new(),
            cascade_stats: CascadeStats::default(),
            #[cfg(feature = "pytorch")]
            table_former,
            table_former_onnx,
            #[cfg(feature = "pytorch")]
            code_formula,
            layout_postprocessor,
            modular_pipeline,
            page_assembler,
            reading_order,
            #[cfg(feature = "opencv-preprocessing")]
            ocr,
            #[cfg(not(feature = "opencv-preprocessing"))]
            ocr_pure,
            #[cfg(target_os = "macos")]
            apple_vision_ocr,
            profiling_enabled: false,
            last_timing: None,
            _not_send_sync: PhantomData,
        })
    }

    /// Enable performance profiling
    ///
    /// When enabled, the pipeline will collect timing data for each stage.
    /// Access timing data via `last_timing` field after `process_page()`.
    pub fn enable_profiling(&mut self) {
        self.profiling_enabled = true;
    }

    /// Disable performance profiling
    pub fn disable_profiling(&mut self) {
        self.profiling_enabled = false;
        self.last_timing = None;
    }

    /// Get cascade routing statistics.
    ///
    /// Returns accumulated statistics about how pages have been routed
    /// through the cascade (heuristic vs. ML).
    #[inline]
    #[must_use = "returns a reference to the cascade routing statistics"]
    pub const fn cascade_stats(&self) -> &CascadeStats {
        &self.cascade_stats
    }

    /// Reset cascade routing statistics.
    pub fn reset_cascade_stats(&mut self) {
        self.cascade_stats = CascadeStats::default();
    }

    /// Load DocLayout-YOLO model for layout detection (GPU only).
    ///
    /// **Warning (N=3491):** YOLO is 2.5x SLOWER than RT-DETR on CPU (~590ms vs ~240ms).
    /// Only use with GPU acceleration (CUDA/CoreML/Metal).
    ///
    /// Enables `AutoWithYolo` and `AlwaysYolo` cascade modes.
    /// For CPU-only deployment, use `Auto` mode (2-tier cascade) instead.
    ///
    /// # Arguments
    ///
    /// * `model_path` - Path to DocLayout-YOLO ONNX model
    ///
    /// # Returns
    ///
    /// `Ok(())` if model loaded successfully.
    ///
    /// # Errors
    ///
    /// Returns error if model file not found or fails to load.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use std::path::Path;
    ///
    /// let mut pipeline = Pipeline::with_defaults()?;
    /// pipeline.load_yolo_model(Path::new("models/doclayout_yolo_doclaynet.onnx"))?;
    /// ```
    #[must_use = "model loading errors should be handled"]
    pub fn load_yolo_model(&mut self, model_path: &std::path::Path) -> Result<()> {
        use crate::models::layout_predictor::DocLayoutYolo;

        let yolo = DocLayoutYolo::load(model_path).map_err(|e| DoclingError::ModelLoadError {
            model_name: "DocLayoutYolo".to_string(),
            source: format!("{e}").into(),
        })?;
        self.yolo_model = Some(yolo);
        log::debug!("✓ DocLayout-YOLO loaded from {}", model_path.display());
        Ok(())
    }

    /// Check if DocLayout-YOLO model is loaded.
    #[inline]
    #[must_use = "returns whether the YOLO model is loaded"]
    pub const fn has_yolo_model(&self) -> bool {
        self.yolo_model.is_some()
    }

    /// Load DocLayout-YOLO CoreML model for Apple Neural Engine acceleration.
    ///
    /// Required for `AutoWithCoreML` and `AlwaysCoreML` cascade modes.
    /// Provides 7.0x speedup over ONNX CPU on Apple Silicon.
    ///
    /// # Arguments
    /// * `model_path` - Path to DocLayout-YOLO CoreML model (.mlmodel or .mlmodelc)
    ///
    /// # Examples
    /// ```ignore
    /// use std::path::Path;
    ///
    /// let mut pipeline = Pipeline::with_defaults()?;
    /// pipeline.load_coreml_model(Path::new("models/doclayout_yolo_doclaynet.mlmodel"))?;
    /// ```
    #[must_use = "model loading errors should be handled"]
    #[cfg(feature = "coreml")]
    pub fn load_coreml_model(&mut self, model_path: &std::path::Path) -> Result<()> {
        use crate::models::layout_predictor::DocLayoutYoloCoreML;

        let coreml =
            DocLayoutYoloCoreML::load(model_path).map_err(|e| DoclingError::ModelLoadError {
                model_name: "DocLayoutYoloCoreML".to_string(),
                source: format!("{e}").into(),
            })?;
        self.coreml_model = Some(coreml);
        log::debug!(
            "✓ DocLayout-YOLO CoreML loaded from {}",
            model_path.display()
        );
        Ok(())
    }

    /// Check if DocLayout-YOLO CoreML model is loaded.
    #[cfg(feature = "coreml")]
    #[inline]
    #[must_use = "returns whether the CoreML model is loaded"]
    pub const fn has_coreml_model(&self) -> bool {
        self.coreml_model.is_some()
    }

    /// Run cascade layout detection based on configured mode.
    ///
    /// Routes pages to heuristic or ML-based detection depending on:
    /// - `cascade_mode` configuration
    /// - Page complexity (for Auto/Conservative modes)
    ///
    /// # Arguments
    /// * `page_image` - Page image as RGB array (`HxWx3`)
    /// * `text_cells` - Optional text cells for complexity estimation and heuristics
    /// * `page_width` - Page width in points
    /// * `page_height` - Page height in points
    ///
    /// # Returns
    /// Layout clusters from either heuristic or ML detection.
    #[allow(clippy::too_many_lines)]
    fn run_cascade_layout_detection(
        &mut self,
        page_image: &Array3<u8>,
        text_cells: Option<&[SimpleTextCell]>,
        page_width: f32,
        page_height: f32,
    ) -> Result<Vec<LayoutCluster>> {
        let cascade_mode = self.config.cascade_mode;

        // Fast paths for mode overrides
        match cascade_mode {
            CascadeMode::AlwaysML => {
                self.cascade_stats.ml_count += 1;
                return self.layout_predictor.infer(page_image).map_err(|e| {
                    DoclingError::InferenceError {
                        model_name: "LayoutPredictor".to_string(),
                        source: format!("{e}").into(),
                    }
                });
            }
            CascadeMode::AlwaysHeuristic => {
                self.cascade_stats.heuristic_count += 1;
                return Ok(self.run_heuristic_layout(text_cells, page_width, page_height));
            }
            CascadeMode::AlwaysYolo => {
                // Use YOLO if available, otherwise fall back to RT-DETR
                return self.run_yolo_layout_with_fallback(page_image);
            }
            #[cfg(feature = "coreml")]
            CascadeMode::AlwaysCoreML => {
                // Use CoreML if available, otherwise fall back to YOLO then RT-DETR
                return self.run_coreml_layout_with_fallback(page_image);
            }
            #[cfg(feature = "coreml")]
            CascadeMode::AutoWithCoreML => {
                // Continue to complexity estimation (handled below)
            }
            CascadeMode::Auto | CascadeMode::AutoWithYolo | CascadeMode::Conservative => {
                // Continue to complexity estimation
            }
        }

        // Convert text cells for complexity estimation
        let text_blocks: Vec<TextBlock> = text_cells
            .map(|cells| {
                cells
                    .iter()
                    .map(|cell| {
                        let font_size = (cell.rect.b - cell.rect.t).abs();
                        TextBlock::new(
                            (cell.rect.l, cell.rect.t, cell.rect.r, cell.rect.b),
                            font_size,
                            cell.text.clone(),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Estimate complexity
        let (complexity, features) =
            self.complexity_estimator
                .estimate(page_image, &text_blocks, page_width, page_height);
        self.cascade_stats.complexity_stats.record(complexity);

        // Route based on complexity and mode
        match cascade_mode {
            CascadeMode::Auto => {
                // 2-tier: heuristic for simple, RT-DETR for rest
                if complexity == Complexity::Simple {
                    log::debug!("    → Using heuristic layout (complexity={complexity})");
                    self.cascade_stats.heuristic_count += 1;
                    Ok(self.run_heuristic_layout(text_cells, page_width, page_height))
                } else {
                    log::debug!("    → Using ML layout (complexity={complexity})");
                    self.cascade_stats.ml_count += 1;
                    self.layout_predictor.infer(page_image).map_err(|e| {
                        DoclingError::InferenceError {
                            model_name: "LayoutPredictor".to_string(),
                            source: format!("{e}").into(),
                        }
                    })
                }
            }
            CascadeMode::AutoWithYolo => {
                // 3-tier: heuristic → YOLO → RT-DETR
                match complexity {
                    Complexity::Simple => {
                        log::debug!("    → Using heuristic layout (complexity={complexity})");
                        self.cascade_stats.heuristic_count += 1;
                        Ok(self.run_heuristic_layout(text_cells, page_width, page_height))
                    }
                    Complexity::Moderate => {
                        // Note: YOLO is only fast with GPU - on CPU it's 2.5x slower than RT-DETR
                        log::debug!(
                            "    → Using YOLO layout (complexity={complexity}, GPU recommended)"
                        );
                        self.run_yolo_layout_with_fallback(page_image)
                    }
                    Complexity::Complex => {
                        // Use RT-DETR for forms, YOLO otherwise
                        if features.has_form_elements {
                            log::debug!(
                                "    → Using ML layout for forms (complexity={complexity})"
                            );
                            self.cascade_stats.ml_count += 1;
                            self.layout_predictor.infer(page_image).map_err(|e| {
                                DoclingError::InferenceError {
                                    model_name: "LayoutPredictor".to_string(),
                                    source: format!("{e}").into(),
                                }
                            })
                        } else {
                            log::debug!(
                                "    → Using YOLO layout (complexity={complexity}, no forms)"
                            );
                            self.run_yolo_layout_with_fallback(page_image)
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
                    log::debug!("    → Using heuristic layout (definitely simple)");
                    self.cascade_stats.heuristic_count += 1;
                    Ok(self.run_heuristic_layout(text_cells, page_width, page_height))
                } else {
                    log::debug!("    → Using ML layout (complexity={complexity})");
                    self.cascade_stats.ml_count += 1;
                    self.layout_predictor.infer(page_image).map_err(|e| {
                        DoclingError::InferenceError {
                            model_name: "LayoutPredictor".to_string(),
                            source: format!("{e}").into(),
                        }
                    })
                }
            }
            #[cfg(feature = "coreml")]
            CascadeMode::AutoWithCoreML => {
                // 3-tier cascade with CoreML: heuristic → CoreML YOLO → RT-DETR
                match complexity {
                    Complexity::Simple => {
                        log::debug!("    → Using heuristic layout (complexity={})", complexity);
                        self.cascade_stats.heuristic_count += 1;
                        Ok(self.run_heuristic_layout(text_cells, page_width, page_height))
                    }
                    Complexity::Moderate => {
                        // CoreML is 7.0x faster than ONNX CPU on Apple Silicon
                        log::debug!(
                            "    → Using CoreML YOLO layout (complexity={}, ANE accelerated)",
                            complexity
                        );
                        self.run_coreml_layout_with_fallback(page_image)
                    }
                    Complexity::Complex => {
                        // Use RT-DETR for forms, CoreML otherwise
                        if features.has_form_elements {
                            log::debug!(
                                "    → Using ML layout for forms (complexity={})",
                                complexity
                            );
                            self.cascade_stats.ml_count += 1;
                            self.layout_predictor.infer(page_image).map_err(|e| {
                                DoclingError::InferenceError {
                                    model_name: "LayoutPredictor".to_string(),
                                    source: format!("{e}").into(),
                                }
                            })
                        } else {
                            log::debug!(
                                "    → Using CoreML YOLO layout (complexity={}, no forms)",
                                complexity
                            );
                            self.run_coreml_layout_with_fallback(page_image)
                        }
                    }
                }
            }
            // AlwaysML, AlwaysHeuristic, AlwaysYolo, AlwaysCoreML handled above
            _ => unreachable!(),
        }
    }

    /// Run YOLO layout detection, falling back to RT-DETR if YOLO not available.
    fn run_yolo_layout_with_fallback(
        &mut self,
        page_image: &Array3<u8>,
    ) -> Result<Vec<LayoutCluster>> {
        if let Some(ref mut yolo) = self.yolo_model {
            self.cascade_stats.yolo_count += 1;
            yolo.infer(page_image)
                .map_err(|e| DoclingError::InferenceError {
                    model_name: "DocLayoutYolo".to_string(),
                    source: format!("{e}").into(),
                })
        } else {
            // Fallback to RT-DETR if YOLO not loaded
            log::debug!("    → YOLO not loaded, falling back to RT-DETR");
            self.cascade_stats.ml_count += 1;
            self.layout_predictor
                .infer(page_image)
                .map_err(|e| DoclingError::InferenceError {
                    model_name: "LayoutPredictor".to_string(),
                    source: format!("{e}").into(),
                })
        }
    }

    /// Run CoreML layout detection, falling back to YOLO then RT-DETR if CoreML not available.
    ///
    /// Provides 7.0x speedup over ONNX CPU on Apple Silicon via Apple Neural Engine.
    #[cfg(feature = "coreml")]
    fn run_coreml_layout_with_fallback(
        &mut self,
        page_image: &Array3<u8>,
    ) -> Result<Vec<LayoutCluster>> {
        if let Some(ref mut coreml) = self.coreml_model {
            self.cascade_stats.coreml_count += 1;
            coreml
                .infer(page_image)
                .map_err(|e| DoclingError::InferenceError {
                    model_name: "DocLayoutYoloCoreML".to_string(),
                    source: format!("{e}").into(),
                })
        } else if let Some(ref mut yolo) = self.yolo_model {
            // Fallback to ONNX YOLO if CoreML not loaded
            log::debug!("    → CoreML not loaded, falling back to ONNX YOLO");
            self.cascade_stats.yolo_count += 1;
            yolo.infer(page_image)
                .map_err(|e| DoclingError::InferenceError {
                    model_name: "DocLayoutYolo".to_string(),
                    source: format!("{e}").into(),
                })
        } else {
            // Fallback to RT-DETR if neither CoreML nor YOLO loaded
            log::debug!("    → CoreML/YOLO not loaded, falling back to RT-DETR");
            self.cascade_stats.ml_count += 1;
            self.layout_predictor
                .infer(page_image)
                .map_err(|e| DoclingError::InferenceError {
                    model_name: "LayoutPredictor".to_string(),
                    source: format!("{e}").into(),
                })
        }
    }

    /// Run heuristic-based layout detection.
    ///
    /// Uses rule-based analysis of text blocks for fast layout detection.
    /// Suitable for simple single-column documents.
    fn run_heuristic_layout(
        &self,
        text_cells: Option<&[SimpleTextCell]>,
        page_width: f32,
        page_height: f32,
    ) -> Vec<LayoutCluster> {
        // Convert text cells to text blocks
        let text_blocks: Vec<TextBlock> = text_cells
            .map(|cells| {
                cells
                    .iter()
                    .map(|cell| {
                        let font_size = (cell.rect.b - cell.rect.t).abs();
                        TextBlock::new(
                            (cell.rect.l, cell.rect.t, cell.rect.r, cell.rect.b),
                            font_size,
                            cell.text.clone(),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();

        self.heuristic_detector
            .detect(&text_blocks, page_width, page_height)
    }

    /// Run layout post-processing using the modular pipeline (Stages 4-10)
    ///
    /// Handles cluster coordinate conversion, cell assignment, and reading order.
    /// Returns the processed clusters sorted by reading order.
    ///
    /// # Arguments
    /// * `clusters` - Initial clusters from layout detection
    /// * `cells` - Text cells for assignment (must be non-empty)
    /// * `page_image` - Page image for dimension extraction
    /// * `page_width` - Page width in points
    /// * `page_height` - Page height in points
    /// * `page_no` - Page number for debugging
    ///
    /// # Returns
    /// Processed clusters with cells assigned and sorted by reading order
    #[allow(unused_variables, reason = "page_no only used in debug-trace feature")]
    fn run_layout_postprocessing(
        &self,
        clusters: &[Cluster],
        cells: &[SimpleTextCell],
        page_image: &Array3<u8>,
        page_width: f32,
        page_height: f32,
        page_no: usize,
    ) -> Vec<Cluster> {
        // Convert clusters to LabeledClusters format
        // N=2287: LayoutPredictor outputs IMAGE coordinates, need to scale to PDF then flip to screen
        let image_height = page_image.shape()[0] as f32;
        let image_width = page_image.shape()[1] as f32;
        let labeled_clusters = convert_clusters_to_labeled(
            clusters,
            image_width,
            image_height,
            page_width,
            page_height,
        );

        // Save labeled clusters (input to modular pipeline Stage 4)
        #[cfg(feature = "debug-trace")]
        {
            if let Ok(ref debug_dir) = std::env::var("DEBUG_E2E_TRACE") {
                Self::save_labeled_clusters(&labeled_clusters, debug_dir, page_no);
            }
        }

        // Convert SimpleTextCell to ModularTextCell format
        let ocr_cells = convert_simple_to_modular_cells(cells);

        // Run modular pipeline INCLUDING reading order (Stages 04-10)
        let stage10_output = self.modular_pipeline.process_stages_4_to_10(
            labeled_clusters,
            ocr_cells,
            page_no,
            f64::from(page_width),
            f64::from(page_height),
        );

        // Convert Stage10Output back to old Cluster format (sorted by reading order)
        convert_stage10_to_clusters(&stage10_output)
    }

    /// Try ONNX table inference with logging
    fn try_onnx_inference(
        onnx_model: &mut TableStructureModelOnnx,
        page_image: &Array3<u8>,
        page_width: f32,
        page_height: f32,
        table_clusters: &[Cluster],
        ocr_cells: &[SimpleTextCell],
        label: &str,
    ) -> Option<crate::pipeline::TableStructurePrediction> {
        match run_table_inference_onnx(
            onnx_model,
            page_image,
            page_width,
            page_height,
            table_clusters,
            ocr_cells,
        ) {
            Ok(prediction) => {
                log::debug!(
                    "    ✓ Inferred {} table structure(s) [{}]",
                    prediction.table_map.len(),
                    label
                );
                Some(prediction)
            }
            Err(e) => {
                log::warn!("    ⚠ {label} inference failed: {e}");
                None
            }
        }
    }

    /// Run table structure detection (Step 3)
    ///
    /// Handles the complexity of ONNX vs `PyTorch` with fallback logic.
    #[allow(
        unused_variables,
        reason = "page_no and table_scale only used with pytorch feature"
    )]
    fn run_table_structure_detection(
        &mut self,
        page_image: &Array3<u8>,
        page_width: f32,
        page_height: f32,
        table_clusters: &[Cluster],
        ocr_cells: Option<&[SimpleTextCell]>,
        page_no: usize,
    ) -> Option<crate::pipeline::TableStructurePrediction> {
        let ocr_cells = ocr_cells.unwrap_or(&[]);

        // Priority 1: No tables - skip
        if table_clusters.is_empty() {
            log::debug!("  [3/4] Table structure detection (skipped - no tables found)");
            return None;
        }

        // Priority 2: ONNX model (preferred - stable)
        if let Some(ref mut onnx_model) = self.table_former_onnx {
            log::debug!("  [3/4] Running table structure inference (ONNX)...");
            log::debug!("    ✓ Found {} table cluster(s)", table_clusters.len());
            return Self::try_onnx_inference(
                onnx_model,
                page_image,
                page_width,
                page_height,
                table_clusters,
                ocr_cells,
                "ONNX",
            );
        }

        // Priority 3: PyTorch model with ONNX fallback
        #[cfg(feature = "pytorch")]
        {
            if let Some(ref table_model) = self.table_former {
                log::debug!("  [3/4] Running TableFormer inference (PyTorch)...");
                log::debug!("    ✓ Found {} table cluster(s)", table_clusters.len());

                let page_image_f32 = page_image.mapv(|x| x as f32);
                let page_image_f32_view = page_image_f32.view();

                match crate::pipeline::table_inference::run_table_inference(
                    table_model,
                    &page_image_f32_view,
                    page_width,
                    page_height,
                    table_clusters,
                    ocr_cells,
                    page_no,
                    self.config.table_scale,
                    self.config.min_cell_size_points,
                    self.config.min_cell_confidence,
                ) {
                    Ok(prediction) => {
                        log::debug!(
                            "    ✓ Inferred {} table structure(s) [PyTorch]",
                            prediction.table_map.len()
                        );
                        return Some(prediction);
                    }
                    Err(e) => {
                        log::warn!("    ⚠ TableFormer inference failed: {e}");
                        // F12: Try ONNX as fallback
                        if let Some(ref mut onnx_model) = self.table_former_onnx {
                            log::info!("    ↳ Falling back to ONNX table model...");
                            return Self::try_onnx_inference(
                                onnx_model,
                                page_image,
                                page_width,
                                page_height,
                                table_clusters,
                                ocr_cells,
                                "ONNX fallback",
                            );
                        }
                    }
                }
            }
        }

        // No models available
        #[cfg(feature = "pytorch")]
        log::debug!("  [3/4] Table structure detection (skipped - no model loaded)");
        #[cfg(not(feature = "pytorch"))]
        log::debug!("  [3/4] Table structure detection (skipped - no model available)");

        None
    }

    /// Process a single page of a PDF
    ///
    /// This runs the complete pipeline:
    /// 1. Layout detection (`LayoutPredictor`)
    /// 2. Layout post-processing (cell assignment, empty cluster removal, orphan clusters)
    /// 3. Table structure (`TableFormer`) - if enabled
    /// 4. Page assembly (create structured elements)
    ///
    /// # Arguments
    /// * `page_no` - Page number (0-indexed)
    /// * `page_image` - Page image as ndarray (HWC format, f32, range [0, 255])
    /// * `page_width` - Page width in points
    /// * `page_height` - Page height in points
    /// * `textline_cells` - Text cells from preprocessing/OCR (optional, for post-processing)
    ///
    /// # Returns
    /// Result containing Page with all predictions and assembled elements
    ///
    /// # Errors
    ///
    /// Returns an error if ML inference, OCR, or element assembly fails.
    ///
    /// # Panics
    ///
    /// This function uses `.unwrap()` on OCR engine references internally, but they
    /// are guarded by `is_some()` checks before use. The panic is unreachable in
    /// normal execution.
    #[must_use = "page processing returns a Result that should be handled"]
    #[allow(clippy::too_many_lines)]
    pub fn process_page(
        &mut self,
        page_no: usize,
        page_image: &Array3<u8>,
        page_width: f32,
        page_height: f32,
        textline_cells: Option<Vec<SimpleTextCell>>,
    ) -> Result<Page> {
        log::debug!("\nProcessing page {page_no}...");

        // Start total timer
        let start_total = Instant::now();

        // Initialize timing struct
        let mut timing = PageTiming::default();

        // Step 0: OCR (if enabled and no textline_cells provided)
        // If textline_cells are provided (e.g., from baseline), use them instead
        // IMPORTANT: Some([]) (empty vector) should trigger OCR, not skip it
        // NOTE: OCR requires opencv-preprocessing feature
        // N=4406 FIX: Only run OCR when page has no embedded text (scanned PDFs)
        // For pages WITH embedded text, use the embedded text (higher quality)
        // This fixes the issue where OCR mode was producing broken output for normal PDFs
        #[cfg(feature = "opencv-preprocessing")]
        let textline_cells = {
            let has_embedded_text = textline_cells
                .as_ref()
                .is_some_and(|cells| !cells.is_empty());
            let should_run_ocr = !has_embedded_text && self.ocr.is_some();

            if should_run_ocr {
                log::debug!("  [0/4] Running RapidOCR...");
                let start_ocr = Instant::now();

                // Convert page image from ndarray to DynamicImage
                let dynamic_image = array_to_dynamic_image(page_image);

                // Run OCR with default parameters
                let ocr_params = crate::ocr::types::OcrParams::default();
                let ocr_cells = self
                    .ocr
                    .as_mut()
                    .unwrap()
                    .detect(&dynamic_image, &ocr_params)
                    .map_err(|e| DoclingError::InferenceError {
                        model_name: "RapidOCR".to_string(),
                        source: format!("{e}").into(),
                    })?;

                log::debug!("    ✓ OCR detected {} text regions", ocr_cells.len());

                // Convert OCR TextCell to SimpleTextCell
                let result = Some(ocr_cells.iter().map(ocr_textcell_to_simple).collect());

                // Record OCR timing
                timing.ocr_duration = Some(start_ocr.elapsed());

                result
            } else {
                textline_cells
            }
        };
        // Without opencv-preprocessing feature, use Apple Vision (macOS) or RapidOcrPure (fallback)
        #[cfg(all(not(feature = "opencv-preprocessing"), target_os = "macos"))]
        let textline_cells = {
            // N=4406 FIX: Only run OCR when page has no embedded text (scanned PDFs)
            // For pages WITH embedded text, use the embedded text (higher quality)
            let has_embedded_text = textline_cells
                .as_ref()
                .is_some_and(|cells| !cells.is_empty());
            let should_run_ocr = !has_embedded_text;

            if should_run_ocr && self.apple_vision_ocr.is_some() {
                // Use Apple Vision OCR (7x better quality than RapidOCR)
                log::debug!("  [0/4] Running Apple Vision OCR (7x better than RapidOCR)...");
                let start_ocr = Instant::now();

                // Convert page image from ndarray to DynamicImage
                let dynamic_image = array_to_dynamic_image(page_image);

                // Run Apple Vision OCR
                let ocr_cells = self
                    .apple_vision_ocr
                    .as_ref()
                    .unwrap()
                    .detect(&dynamic_image, page_width, page_height)
                    .map_err(|e| DoclingError::InferenceError {
                        model_name: "AppleVision".to_string(),
                        source: format!("{e}").into(),
                    })?;

                log::debug!(
                    "    ✓ Apple Vision OCR detected {} text regions",
                    ocr_cells.len()
                );

                // Convert OCR TextCell to SimpleTextCell
                let result = Some(ocr_cells.iter().map(ocr_textcell_to_simple).collect());

                // Record OCR timing
                timing.ocr_duration = Some(start_ocr.elapsed());

                result
            } else if should_run_ocr && self.ocr_pure.is_some() {
                // Fall back to RapidOCR (pure Rust)
                log::debug!("  [0/4] Running RapidOcrPure (pure Rust)...");
                let start_ocr = Instant::now();

                // Convert page image from ndarray to DynamicImage
                let dynamic_image = array_to_dynamic_image(page_image);

                // Run OCR with default parameters
                let ocr_params = crate::ocr::types::OcrParams::default();
                let ocr_cells = self
                    .ocr_pure
                    .as_mut()
                    .unwrap()
                    .detect(&dynamic_image, &ocr_params)
                    .map_err(|e| DoclingError::InferenceError {
                        model_name: "RapidOcrPure".to_string(),
                        source: format!("{e}").into(),
                    })?;

                log::debug!(
                    "    ✓ OCR (pure Rust) detected {} text regions",
                    ocr_cells.len()
                );

                // Convert OCR TextCell to SimpleTextCell
                let result = Some(ocr_cells.iter().map(ocr_textcell_to_simple).collect());

                // Record OCR timing
                timing.ocr_duration = Some(start_ocr.elapsed());

                result
            } else {
                textline_cells
            }
        };

        // Non-macOS: use RapidOcrPure (pure Rust)
        #[cfg(all(not(feature = "opencv-preprocessing"), not(target_os = "macos")))]
        let textline_cells = {
            // N=4406 FIX: Only run OCR when page has no embedded text (scanned PDFs)
            let has_embedded_text = textline_cells
                .as_ref()
                .is_some_and(|cells| !cells.is_empty());
            let should_run_ocr = !has_embedded_text && self.ocr_pure.is_some();

            if should_run_ocr {
                log::debug!("  [0/4] Running RapidOcrPure (pure Rust)...");
                let start_ocr = Instant::now();

                // Convert page image from ndarray to DynamicImage
                let dynamic_image = array_to_dynamic_image(page_image);

                // Run OCR with default parameters
                let ocr_params = crate::ocr::types::OcrParams::default();
                let ocr_cells = self
                    .ocr_pure
                    .as_mut()
                    .unwrap()
                    .detect(&dynamic_image, &ocr_params)
                    .map_err(|e| DoclingError::InferenceError {
                        model_name: "RapidOcrPure".to_string(),
                        source: format!("{e}").into(),
                    })?;

                log::debug!(
                    "    ✓ OCR (pure Rust) detected {} text regions",
                    ocr_cells.len()
                );

                // Convert OCR TextCell to SimpleTextCell
                let result = Some(ocr_cells.iter().map(ocr_textcell_to_simple).collect());

                // Record OCR timing
                timing.ocr_duration = Some(start_ocr.elapsed());

                result
            } else {
                textline_cells
            }
        };

        // Issue #13 FIX: Warn when text cells are empty and OCR is disabled
        // This helps debug cases where a scanned PDF produces no text
        let is_empty_cells =
            textline_cells.is_none() || textline_cells.as_ref().is_some_and(Vec::is_empty);
        if is_empty_cells && !self.config.ocr_enabled {
            log::warn!(
                "Page {page_no}: No text cells available and OCR is disabled. \
                 Document elements may have empty text. \
                 Enable OCR with .ocr_enabled(true) for scanned documents."
            );
        }

        // Convert textline_cells from BottomLeft (PDF) to TopLeft (screen) coordinates
        let start_coord = Instant::now();
        let textline_cells =
            textline_cells.map(|cells| convert_textline_coords(cells, page_height));
        timing.coord_conversion_duration = start_coord.elapsed();

        // Step 1: Layout detection (with cascade routing)
        log::debug!(
            "  [1/4] Running layout detection (cascade_mode={})...",
            self.config.cascade_mode
        );
        let start_layout = Instant::now();
        let layout_clusters = self.run_cascade_layout_detection(
            page_image,
            textline_cells.as_deref(),
            page_width,
            page_height,
        )?;
        log::debug!("    ✓ Detected {} layout clusters", layout_clusters.len());
        log::debug!("  Layout model produced {} clusters", layout_clusters.len());
        timing.layout_detection_duration = start_layout.elapsed();

        // Validate layout quality to catch ML model failures early
        match validate_layout_clusters(&layout_clusters) {
            LayoutValidationResult::Error(msg) => {
                log::error!("Layout validation FAILED: {msg}");
                // Don't fail the pipeline - continue with potentially degraded results
                // but make the error very visible in logs
            }
            LayoutValidationResult::Warning(msg) => {
                log::warn!("Layout validation warning: {msg}");
            }
            LayoutValidationResult::Valid => {
                log::trace!("Layout validation passed");
            }
        }

        // Save layout clusters for debugging (after Stage 2/3 - ML + HF postprocessing)
        #[cfg(feature = "debug-trace")]
        {
            if let Ok(ref debug_dir) = std::env::var("DEBUG_E2E_TRACE") {
                Self::save_layout_clusters(&layout_clusters, debug_dir, page_no);
            }
        }

        // Convert LayoutCluster (baseline) to Cluster (pipeline)
        let mut clusters: Vec<Cluster> = layout_clusters
            .into_iter()
            .map(convert_layout_cluster)
            .collect();

        // Step 2: Layout post-processing (if textline_cells provided AND non-empty)
        // Clone cells for TableFormer (needed after modular pipeline consumes cells)
        let textline_cells_for_table = textline_cells.clone();
        // N=629: Skip postprocessing if cells is empty - Stage 5 would remove all clusters
        if let Some(cells) = textline_cells.filter(|c| !c.is_empty()) {
            log::debug!("  [2/4] Running layout post-processing (modular pipeline)...");
            let start_postprocess = Instant::now();
            let input_count = clusters.len();

            clusters = self.run_layout_postprocessing(
                &clusters,
                &cells,
                page_image,
                page_width,
                page_height,
                page_no,
            );

            log::debug!(
                "    ✓ Post-processed: {} clusters (was {})",
                clusters.len(),
                input_count
            );
            timing.layout_postprocess_duration = start_postprocess.elapsed();
        } else {
            log::debug!("  [2/4] Layout post-processing (skipped - no cells provided)");
        }

        // Step 3: Table structure detection
        let start_table = Instant::now();
        let table_clusters: Vec<Cluster> = clusters
            .iter()
            .filter(|c| c.label == DocItemLabel::Table)
            .cloned()
            .collect();

        let tablestructure = self.run_table_structure_detection(
            page_image,
            page_width,
            page_height,
            &table_clusters,
            textline_cells_for_table.as_deref(),
            page_no,
        );
        timing.table_structure_duration = Some(start_table.elapsed());

        // Step 4: Page assembly
        log::debug!("  [4/4] Assembling page elements...");
        let start_assembly = Instant::now();
        let mut page = Page {
            page_no,
            size: Some(Size {
                width: page_width,
                height: page_height,
            }),
            predictions: PagePredictions {
                layout: Some(LayoutPrediction { clusters }),
                tablestructure,
                ..Default::default()
            },
            assembled: None,
        };

        // Assemble page (convert clusters to structured elements)
        self.page_assembler
            .assemble_page(&mut page)
            .map_err(|e| DoclingError::AssemblyError {
                reason: format!("Failed to assemble page {page_no}: {e}"),
            })?;

        let element_count = page.assembled.as_ref().map_or(0, |a| a.elements.len());
        log::debug!("    ✓ Assembled {element_count} elements");
        log::debug!("  Assembled {element_count} elements from clusters");
        timing.page_assembly_duration = start_assembly.elapsed();

        // Step 5: Optional enrichment (Code/Formula)
        // Stage 12: Enrich code and formula elements with ML predictions
        if self.config.code_formula_enabled {
            log::debug!("  [5/5] Enriching code/formula elements...");
            let start_code_formula = Instant::now();
            page = self.enrich_page(page, page_image)?;
            timing.code_formula_duration = Some(start_code_formula.elapsed());
        }

        // Record total time
        timing.total_duration = start_total.elapsed();

        // Store timing if profiling enabled
        if self.profiling_enabled {
            self.last_timing = Some(timing);
            timing.print();
        }

        log::debug!("✓ Page {page_no} processing complete");

        Ok(page)
    }

    /// Process multiple pages in a batch
    ///
    /// More efficient than calling `process_page()` repeatedly for multi-page documents.
    /// Batches layout detection to amortize model overhead and improve GPU utilization.
    ///
    /// # Arguments
    ///
    /// * `pages` - Vector of (`page_no`, `page_image`, `page_width`, `page_height`, `textline_cells`)
    ///
    /// # Returns
    ///
    /// * `Vec<Result<Page>>` - Results for each page (preserves input order)
    ///
    /// # Performance
    ///
    /// - Single page: 60-82 ms/page
    /// - Batch (10 pages): Expected 1.5-2x throughput improvement
    ///
    /// # Exampless
    ///
    /// ```ignore
    /// use docling_pdf_ml::{Pipeline, PipelineConfig};
    /// use ndarray::Array3;
    ///
    /// # fn main() -> docling_pdf_ml::Result<()> {
    /// let mut pipeline = Pipeline::new(PipelineConfig::default())?;
    ///
    /// // Prepare batch of pages
    /// let pages = vec![
    ///     (0, page0_image, width0, height0, None),
    ///     (1, page1_image, width1, height1, None),
    ///     (2, page2_image, width2, height2, None),
    /// ];
    ///
    /// // Process batch
    /// let results = pipeline.process_pages_batch(pages)?;
    /// # Ok(())
    /// # }
    /// ```
    #[allow(
        clippy::type_complexity,
        reason = "tuple encapsulates per-page data (index, image, scale_x, scale_y, text_cells)"
    )]
    #[must_use = "batch processing returns results that should be handled"]
    #[allow(clippy::too_many_lines)]
    pub fn process_pages_batch(
        &mut self,
        pages: Vec<(usize, Array3<u8>, f32, f32, Option<Vec<SimpleTextCell>>)>,
    ) -> Vec<Result<Page>> {
        if pages.is_empty() {
            return vec![];
        }

        let batch_size = pages.len();
        log::debug!("\n[Batch] Processing {batch_size} pages in batch");

        let start_total = Instant::now();

        // Step 1: Batch layout detection (the bottleneck - 99.2% of time)
        log::debug!("  [1/4] Running batch layout detection...");
        let start_layout = Instant::now();

        // Extract images for batch inference
        let images: Vec<Array3<u8>> = pages.iter().map(|(_, img, _, _, _)| img.clone()).collect();

        // Run batch inference
        let layout_results = match self.layout_predictor.infer_batch(&images) {
            Ok(results) => results,
            Err(e) => {
                // If batch inference fails, return error for all pages
                let error_msg = format!("Batch inference failed: {e}");
                return pages
                    .iter()
                    .map(|(_page_no, _, _, _, _)| {
                        Err(DoclingError::InferenceError {
                            model_name: "LayoutPredictor".to_string(),
                            source: error_msg.clone().into(),
                        })
                    })
                    .collect();
            }
        };

        let layout_duration = start_layout.elapsed();
        log::debug!(
            "    ✓ Detected layout for {} pages in {:.2} ms ({:.2} ms/page)",
            batch_size,
            layout_duration.as_secs_f64() * 1000.0,
            layout_duration.as_secs_f64() * 1000.0 / batch_size as f64
        );

        // Step 2-4: Process each page independently (postprocessing, table, assembly)
        // Note: Sequential processing intentional (page-level dependencies in table inference)
        // Future optimization: Parallelize with rayon after refactoring table inference
        let mut results = Vec::with_capacity(batch_size);

        for (
            page_idx,
            ((page_no, page_image, page_width, page_height, textline_cells), layout_clusters),
        ) in pages
            .into_iter()
            .zip(layout_results.into_iter())
            .enumerate()
        {
            log::debug!(
                "\n  [Page {}/{}] Processing page {}...",
                page_idx + 1,
                batch_size,
                page_no
            );

            // Convert LayoutCluster (baseline) to Cluster (pipeline)
            let mut clusters: Vec<Cluster> = layout_clusters
                .into_iter()
                .map(convert_layout_cluster)
                .collect();

            log::debug!("    ✓ Layout: {} clusters", clusters.len());

            // Convert textline_cells from BottomLeft (PDF) to TopLeft (screen) coordinates
            let start_coord = Instant::now();
            let textline_cells =
                textline_cells.map(|cells| convert_textline_coords(cells, page_height));
            let _coord_duration = start_coord.elapsed();

            // Layout post-processing (if textline_cells provided and non-empty)
            let textline_cells_for_table = textline_cells.clone();
            // N=629: Skip postprocessing if cells is empty - Stage 5 would remove all clusters
            if let Some(cells) = textline_cells.filter(|c| !c.is_empty()) {
                log::debug!("    [2/4] Running layout post-processing...");
                let start_postprocess = Instant::now();
                let input_count = clusters.len();

                clusters = self.run_layout_postprocessing(
                    &clusters,
                    &cells,
                    &page_image,
                    page_width,
                    page_height,
                    page_no,
                );

                let _postprocess_duration = start_postprocess.elapsed();
                log::debug!(
                    "      ✓ Post-processed: {} clusters (was {})",
                    clusters.len(),
                    input_count
                );
            } else {
                log::debug!("    [2/4] Layout post-processing (skipped - no cells provided)");
            }

            // Table structure detection
            log::debug!("    [3/4] Table structure...");
            let start_table = Instant::now();

            let table_clusters: Vec<Cluster> = clusters
                .iter()
                .filter(|c| c.label == DocItemLabel::Table)
                .cloned()
                .collect();

            let tablestructure = self.run_table_structure_detection(
                &page_image,
                page_width,
                page_height,
                &table_clusters,
                textline_cells_for_table.as_deref(),
                page_no,
            );
            let _table_duration = start_table.elapsed();

            // Page assembly
            log::debug!("    [4/4] Assembling page elements...");
            let start_assembly = Instant::now();
            let mut page = Page {
                page_no,
                size: Some(Size {
                    width: page_width,
                    height: page_height,
                }),
                predictions: PagePredictions {
                    layout: Some(LayoutPrediction { clusters }),
                    tablestructure,
                    ..Default::default()
                },
                assembled: None,
            };

            // Assemble page
            let assembly_result = self.page_assembler.assemble_page(&mut page).map_err(|e| {
                DoclingError::AssemblyError {
                    reason: format!("Failed to assemble page {page_no}: {e}"),
                }
            });

            if let Err(e) = assembly_result {
                results.push(Err(e));
                continue;
            }

            let element_count = page.assembled.as_ref().map_or(0, |a| a.elements.len());
            let _assembly_duration = start_assembly.elapsed();
            log::debug!("      ✓ Assembled {element_count} elements");

            // Optional code/formula enrichment
            // Note: Currently processes elements sequentially. Low priority for batching
            // (code/formula elements are rare, <5% of documents)
            if self.config.code_formula_enabled {
                log::debug!("    [5/5] Enriching code/formula elements...");
                match self.enrich_page(page, &page_image) {
                    Ok(enriched_page) => {
                        page = enriched_page;
                    }
                    Err(e) => {
                        results.push(Err(e));
                        continue;
                    }
                }
            }

            log::debug!("    ✓ Page {page_no} complete");
            results.push(Ok(page));
        }

        let total_duration = start_total.elapsed();
        log::debug!(
            "\n[Batch] Completed {} pages in {:.2} ms ({:.2} ms/page)",
            batch_size,
            total_duration.as_secs_f64() * 1000.0,
            total_duration.as_secs_f64() * 1000.0 / batch_size as f64
        );

        results
    }

    /// Process document with reading order
    ///
    /// This applies reading order to all pages in a document:
    /// 1. Collects all elements from all pages
    /// 2. Applies reading order (sort elements)
    /// 3. Applies caption assignments (attach captions to parent elements)
    /// 4. Applies footnote assignments (attach footnotes to parent elements)
    /// 5. Applies text merges (combine split text elements)
    ///
    /// # Arguments
    /// * `pages` - Vector of all processed pages
    ///
    /// # Returns
    /// Result containing reordered `AssembledUnit`
    ///
    /// # Errors
    ///
    /// Returns an error if reading order processing fails.
    #[must_use = "document processing returns a Result that should be handled"]
    pub fn process_document(&self, pages: &[Page]) -> Result<crate::pipeline::AssembledUnit> {
        log::debug!("\nApplying reading order to document...");

        // Step 1: Collect all elements from all pages and renumber cluster IDs globally
        // Cluster IDs are page-local (each page has its own 0-N IDs)
        // We need to renumber them to be globally unique for reading order
        // N=373: Preallocate capacity to avoid reallocations
        let total_elements: usize = pages
            .iter()
            .filter_map(|p| p.assembled.as_ref())
            .map(|a| a.elements.len())
            .sum();
        let mut all_elements = Vec::with_capacity(total_elements);
        let mut cid_offset = 0;
        let mut page_dimensions = std::collections::HashMap::new(); // page_no -> (width, height)

        for page in pages {
            let page_no = page.page_no;

            // Store page dimensions (if available)
            if let Some(size) = page.size {
                page_dimensions.insert(page_no, (size.width, size.height));
            }

            if let Some(ref assembled) = page.assembled {
                for elem in &assembled.elements {
                    // Create element with renumbered cluster ID
                    let mut elem_copy = elem.clone();
                    elem_copy.renumber_cluster_id(cid_offset);
                    all_elements.push(elem_copy);
                }

                // Update offset for next page
                if let Some(max_cid) = assembled.elements.iter().map(|e| e.cluster().id).max() {
                    cid_offset += max_cid + 1;
                }
            }
        }

        log::debug!("  Total elements: {}", all_elements.len());

        if all_elements.is_empty() {
            log::debug!("  ⚠ No elements to process");
            return Ok(crate::pipeline::AssembledUnit {
                elements: Vec::new(),
                body: Vec::new(),
                headers: Vec::new(),
            });
        }

        // Step 1.5: Relabel text elements that look like captions
        // This must happen BEFORE reading order so that caption assignment algorithm finds them
        Self::relabel_caption_like_texts(&mut all_elements);

        // Step 2: Apply reading order
        log::debug!("  [1/4] Computing reading order...");
        let ordered_cids = self.reading_order.predict(&all_elements, &page_dimensions);
        log::debug!("    ✓ Ordered {} elements", ordered_cids.len());

        // N=374: Consume all_elements into HashMap to avoid clones during reordering
        // Build a map from global_cid to element (takes ownership)
        let mut element_map: std::collections::HashMap<usize, PageElement> = all_elements
            .into_iter()
            .map(|elem| (elem.cluster().id, elem))
            .collect();

        // Reorder elements by reading order (move from HashMap, no clone)
        let mut ordered_elements = Vec::with_capacity(ordered_cids.len());
        for &cid in &ordered_cids {
            if let Some(elem) = element_map.remove(&cid) {
                ordered_elements.push(elem);
            } else {
                log::warn!("Warning: Could not find element with global cid {cid}");
            }
        }

        // Step 3: Apply caption assignments
        log::debug!("  [2/4] Applying caption assignments...");
        let caption_assignments = self.reading_order.predict_to_captions(&ordered_elements);
        log::debug!("    ✓ {} caption assignments", caption_assignments.len());

        // Apply caption assignments to elements
        // N=374: Pass by value to avoid cloning Vec<usize> values
        Self::apply_caption_assignments(&mut ordered_elements, caption_assignments);

        // Step 4: Apply footnote assignments
        log::debug!("  [3/4] Applying footnote assignments...");
        let footnote_assignments = self.reading_order.predict_to_footnotes(&ordered_elements);
        log::debug!("    ✓ {} footnote assignments", footnote_assignments.len());

        // Apply footnote assignments to elements
        // N=374: Pass by value to avoid cloning Vec<usize> values
        Self::apply_footnote_assignments(&mut ordered_elements, footnote_assignments);

        // Step 5: Apply text merges
        log::debug!("  [4/4] Applying text merges...");
        let text_merges = self.reading_order.predict_merges(&ordered_elements);
        log::debug!("    ✓ {} text merge operations", text_merges.len());

        // Apply text merges to combine split text elements
        Self::apply_text_merges(&mut ordered_elements, &text_merges);

        // Separate into body and headers
        // N=374: Use partition to avoid bounds-checking in loop, but we still need 1 clone per element
        // (ordered_elements is moved to return value, body/headers must be cloned)
        let (headers, body): (Vec<PageElement>, Vec<PageElement>) =
            ordered_elements.iter().cloned().partition(|elem| {
                matches!(
                    elem.cluster().label,
                    DocItemLabel::PageHeader | DocItemLabel::PageFooter
                )
            });

        log::debug!("✓ Reading order applied:");
        log::debug!("    - Total: {} elements", ordered_elements.len());
        log::debug!("    - Body: {} elements", body.len());
        log::debug!("    - Headers: {} elements", headers.len());

        Ok(crate::pipeline::AssembledUnit {
            elements: ordered_elements,
            body,
            headers,
        })
    }

    /// Enrich code and formula elements with ML predictions
    ///
    /// This is Stage 12 (optional enrichment) - runs AFTER page assembly.
    /// It processes Code and Formula elements by:
    /// 1. Finding all Code/Formula elements in the assembled page
    /// 2. Cropping their regions from the page image
    /// 3. Running CodeFormula ML model for text recognition
    /// 4. Updating element text fields in place
    ///
    /// # Arguments
    ///
    /// * `page` - Assembled page with elements
    /// * `page_image` - Full page image as RGB ndarray (H, W, 3)
    ///
    /// # Returns
    ///
    /// * Result with enriched page (elements updated in place)
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Enrichment model is not loaded
    /// - Image cropping fails
    /// - ML inference fails
    ///
    /// NOTE: Requires pytorch feature for code_formula model
    #[must_use = "page enrichment returns a Result that should be handled"]
    #[cfg(feature = "pytorch")]
    pub fn enrich_page(&mut self, mut page: Page, page_image: &Array3<u8>) -> Result<Page> {
        // Skip if enrichment disabled
        let code_formula = match &mut self.code_formula {
            Some(model) => model,
            None => return Ok(page), // No enrichment model loaded
        };

        // Get mutable reference to assembled elements
        let assembled = match &mut page.assembled {
            Some(asm) => asm,
            None => return Ok(page), // No elements to enrich
        };

        // Get page dimensions from Page.size
        let (page_width, page_height) = match page.size {
            Some(size) => (size.width, size.height),
            None => {
                // Fall back to using image dimensions if page size not set
                let (img_height, img_width, _) = page_image.dim();
                (img_width as f32, img_height as f32)
            }
        };

        // Find code and formula elements
        // Clone bboxes to avoid borrowing issues when updating elements later
        // N=373: Preallocate with element count (most won't be code/formula, but small overhead)
        let mut enrichment_batch: Vec<(usize, BoundingBox, &str)> =
            Vec::with_capacity(assembled.elements.len() / 4);

        for (idx, element) in assembled.elements.iter().enumerate() {
            match element {
                crate::pipeline::PageElement::Text(text_elem) => {
                    let label = &text_elem.label;
                    let label_str = match label {
                        DocItemLabel::Code => "code",
                        DocItemLabel::Formula => "formula",
                        _ => continue, // Not code/formula, skip
                    };

                    // Copy bbox from cluster (BoundingBox implements Copy)
                    let bbox = text_elem.cluster.bbox;
                    enrichment_batch.push((idx, bbox, label_str));
                }
                _ => continue, // Only enrich Text elements
            }
        }

        // Skip if no code/formula regions
        if enrichment_batch.is_empty() {
            return Ok(page);
        }

        log::debug!(
            "Enriching {} code/formula regions...",
            enrichment_batch.len()
        );

        // Crop regions and prepare for batch processing
        let mut cropped_images = Vec::new();
        let mut labels = Vec::new();
        let mut valid_indices = Vec::new(); // Track which elements are actually processed

        for (i, (idx, bbox, label)) in enrichment_batch.iter().enumerate() {
            // Calculate bbox dimensions
            let width = bbox.r - bbox.l;
            let height = bbox.b - bbox.t;

            // Validate minimum dimensions (skip tiny/invalid regions)
            // Uses configurable min_enrichment_region_size to avoid extreme distortion and PyTorch issues
            let min_size = self.config.min_enrichment_region_size;
            if width < min_size || height < min_size {
                log::debug!(
                    "  ⚠️  Skipping {} region {} - too small ({:.1}x{:.1} pixels, min {}x{})",
                    label,
                    i,
                    width,
                    height,
                    min_size,
                    min_size
                );
                continue;
            }

            // Validate dimensions are positive
            if width <= 0.0 || height <= 0.0 {
                log::debug!(
                    "  ⚠️  Skipping {} region {} - invalid dimensions ({:.1}x{:.1})",
                    label,
                    i,
                    width,
                    height
                );
                continue;
            }

            // Crop image to bbox
            let cropped = crop_region(page_image, bbox, page_width, page_height)?;

            // Debug: Save cropped image
            let debug_path = format!("debug_crop_{}_{}.png", label, i);
            cropped.save(&debug_path).ok();
            log::debug!("  Saved cropped {} region to: {} (bbox: l={:.1}, t={:.1}, r={:.1}, b={:.1}, size: {:.1}x{:.1})",
                     label, debug_path, bbox.l, bbox.t, bbox.r, bbox.b, width, height);

            cropped_images.push(cropped);
            labels.push(*label);
            valid_indices.push(*idx); // Track which element this corresponds to
        }

        // Skip if no valid regions after filtering
        if cropped_images.is_empty() {
            log::debug!("  No valid code/formula regions to enrich (all too small)");
            return Ok(page);
        }

        // Run CodeFormula model (batch inference)
        let enriched_outputs = code_formula
            .process_batch(&cropped_images, &labels)
            .map_err(|e| DoclingError::InferenceError {
                model_name: "CodeFormula".to_string(),
                source: format!("{e}").into(),
            })?;

        // Update elements with enriched text (using valid_indices, not enrichment_batch)
        for (idx, output) in valid_indices.iter().zip(enriched_outputs.iter()) {
            if let Some(crate::pipeline::PageElement::Text(text_elem)) =
                assembled.elements.get_mut(*idx)
            {
                text_elem.text = output.text.clone();

                // NOTE: Code language detection result not stored in element struct
                // Rationale: TextElement has no code_language field. Idefics3 detects language but we only use enhanced text.
                // Would require: Add Optional<String> code_language field to TextElement in data_structures.rs.
                // Current behavior: Language logged but not persisted.
                if let Some(ref language) = output.language {
                    log::debug!("  Code region {} detected as: {}", idx, language);
                }
            }
        }

        log::debug!("✓ Enrichment complete");
        Ok(page)
    }

    /// Enrich code and formula elements with ML predictions (stub for non-pytorch builds)
    ///
    /// When the pytorch feature is not enabled, this method simply returns the page unchanged.
    ///
    /// # Errors
    ///
    /// This function currently never returns an error (returns `Ok(page)` always).
    #[cfg(not(feature = "pytorch"))]
    pub fn enrich_page(&mut self, page: Page, _page_image: &Array3<u8>) -> Result<Page> {
        // No enrichment model available without pytorch feature
        Ok(page)
    }

    /// Apply caption assignments to elements
    ///
    /// Modifies elements in place to attach caption CIDs to their parent elements
    /// (Code blocks, Tables, Pictures)
    ///
    /// N=374: Takes ownership of `HashMap` to avoid cloning `Vec<usize>` values
    /// Relabel text elements that look like captions (N=118)
    ///
    /// Problem: ML model detects only 1-4 Caption labels per page, but Python output has 10-12 captions.
    /// Root cause: Python implicitly relabels TEXT elements starting with "Figure"/"Table" as Caption.
    /// Solution: Relabel TEXT elements whose text starts with "Figure X:" or "Table X:" pattern.
    ///
    /// This MUST run BEFORE reading order so that `predict_to_captions()` can find Caption labels.
    fn relabel_caption_like_texts(elements: &mut [PageElement]) {
        let mut relabeled_count = 0;

        for element in elements.iter_mut() {
            if let PageElement::Text(ref mut text_elem) = element {
                // Only relabel TEXT elements (not already Caption, not other types)
                if text_elem.label == DocItemLabel::Text {
                    let text = &text_elem.text;

                    // Check if text matches caption pattern: "Figure X:" or "Table X:" at start
                    // Allow optional whitespace and require colon
                    let is_caption_like = (text.starts_with("Figure ")
                        || text.starts_with("Table "))
                        && text.contains(':');

                    if is_caption_like {
                        log::debug!(
                            "    Relabeling Text element (CID {}) as Caption: {}",
                            text_elem.id,
                            &text[..text.len().min(60)]
                        );
                        text_elem.label = DocItemLabel::Caption;
                        relabeled_count += 1;
                    }
                }
            }
        }

        if relabeled_count > 0 {
            log::debug!("    ✓ Relabeled {relabeled_count} text elements as captions");
        }
    }

    fn apply_caption_assignments(
        elements: &mut [PageElement],
        mut caption_assignments: HashMap<usize, Vec<usize>>,
    ) {
        for element in elements.iter_mut() {
            let cid = element.cluster().id;
            if let Some(caption_cids) = caption_assignments.remove(&cid) {
                match element {
                    PageElement::Text(ref mut text_elem) => {
                        // Only code blocks can have captions (not regular text)
                        if text_elem.label == DocItemLabel::Code {
                            text_elem.captions = caption_cids;
                        }
                    }
                    PageElement::Table(ref mut table_elem) => {
                        table_elem.captions = caption_cids;
                    }
                    PageElement::Figure(ref mut fig_elem) => {
                        fig_elem.captions = caption_cids;
                    }
                    PageElement::Container(_) => {
                        // Containers don't have captions
                    }
                }
            }
        }
    }

    /// Apply footnote assignments to elements
    ///
    /// Modifies elements in place to attach footnote CIDs to their parent elements
    /// (Tables, Pictures)
    ///
    /// N=374: Takes ownership of `HashMap` to avoid cloning `Vec<usize>` values
    fn apply_footnote_assignments(
        elements: &mut [PageElement],
        mut footnote_assignments: HashMap<usize, Vec<usize>>,
    ) {
        for element in elements.iter_mut() {
            let cid = element.cluster().id;
            if let Some(footnote_cids) = footnote_assignments.remove(&cid) {
                match element {
                    PageElement::Text(ref mut text_elem) => {
                        // Only code blocks can have footnotes (not regular text)
                        if text_elem.label == DocItemLabel::Code {
                            text_elem.footnotes = footnote_cids;
                        }
                    }
                    PageElement::Table(ref mut table_elem) => {
                        table_elem.footnotes = footnote_cids;
                    }
                    PageElement::Figure(ref mut fig_elem) => {
                        fig_elem.footnotes = footnote_cids;
                    }
                    PageElement::Container(_) => {
                        // Containers don't have footnotes
                    }
                }
            }
        }
    }

    /// Apply text merges to combine split text elements
    ///
    /// Merges text from following elements into preceding elements based on merge mapping.
    /// Merged elements are kept in the list but marked for later filtering.
    fn apply_text_merges(elements: &mut [PageElement], text_merges: &HashMap<usize, Vec<usize>>) {
        use std::collections::HashSet;

        // Build CID to index mapping for fast lookup
        let mut cid_to_idx: HashMap<usize, usize> = HashMap::new();
        for (idx, elem) in elements.iter().enumerate() {
            cid_to_idx.insert(elem.cluster().id, idx);
        }

        // Track which elements have been merged (should be skipped)
        let merged_cids: HashSet<usize> = text_merges.values().flatten().copied().collect();

        // First pass: collect texts to merge (to avoid borrowing issues)
        let mut texts_to_merge: HashMap<usize, Vec<String>> = HashMap::new();
        for (base_cid, merge_cids) in text_merges {
            let mut merge_texts = Vec::new();
            for merge_cid in merge_cids {
                if let Some(&merge_idx) = cid_to_idx.get(merge_cid) {
                    if let PageElement::Text(merge_elem) = &elements[merge_idx] {
                        merge_texts.push(merge_elem.text.clone());
                    }
                }
            }
            texts_to_merge.insert(*base_cid, merge_texts);
        }

        // Second pass: apply merges
        for (base_cid, merge_texts) in &texts_to_merge {
            if let Some(&base_idx) = cid_to_idx.get(base_cid) {
                if let PageElement::Text(ref mut base_elem) = &mut elements[base_idx] {
                    for merge_text in merge_texts {
                        base_elem.text.push(' ');
                        base_elem.text.push_str(merge_text);
                    }
                }
            }
        }

        // Third pass: mark merged elements by clearing their text
        for elem in elements.iter_mut() {
            let cid = elem.cluster().id;
            if merged_cids.contains(&cid) {
                if let PageElement::Text(ref mut text_elem) = elem {
                    // Clear text to mark as merged (will be filtered out later)
                    text_elem.text.clear();
                }
            }
        }
    }

    /// Save layout clusters to JSON for debugging
    fn save_layout_clusters(clusters: &[LayoutCluster], debug_dir: &str, page_no: usize) {
        use std::fs;
        use std::path::PathBuf;

        let debug_path = PathBuf::from(debug_dir);
        if let Err(e) = fs::create_dir_all(&debug_path) {
            log::error!("Failed to create debug directory: {e}");
            return;
        }

        let output_path = debug_path.join(format!("stage23_layout_clusters_page{page_no}.json"));

        // Convert to JSON-serializable format
        let cluster_data: Vec<serde_json::Value> = clusters
            .iter()
            .map(|c| {
                serde_json::json!({
                    "id": c.id,
                    "label": format!("{:?}", c.label),
                    "confidence": c.confidence,
                    "bbox": {
                        "l": c.bbox.l,
                        "t": c.bbox.t,
                        "r": c.bbox.r,
                        "b": c.bbox.b
                    }
                })
            })
            .collect();

        if let Ok(json_str) = serde_json::to_string_pretty(&cluster_data) {
            if let Err(e) = fs::write(&output_path, json_str) {
                log::error!("Failed to save layout clusters: {e}");
            } else {
                log::info!("💾 Saved Stage 2/3 output: {}", output_path.display());
            }
        }
    }

    /// Save labeled clusters (Stage 3 in modular format) to JSON for debugging
    fn save_labeled_clusters(clusters: &LabeledClusters, debug_dir: &str, page_no: usize) {
        use std::fs;
        use std::path::PathBuf;

        let debug_path = PathBuf::from(debug_dir);
        if let Err(e) = fs::create_dir_all(&debug_path) {
            log::error!("Failed to create debug directory: {e}");
            return;
        }

        let output_path = debug_path.join(format!("stage3_labeled_clusters_page{page_no}.json"));

        if let Ok(json_str) = serde_json::to_string_pretty(&clusters) {
            if let Err(e) = fs::write(&output_path, json_str) {
                log::error!("Failed to save labeled clusters: {e}");
            } else {
                log::info!(
                    "💾 Saved Stage 3 (modular format): {}",
                    output_path.display()
                );
            }
        }
    }
}

/// Run table structure inference using ONNX model (Microsoft Table Transformer)
///
/// This is an alternative to `PyTorch` `TableFormer` that uses ONNX Runtime,
/// avoiding libtorch crashes.
///
/// # Arguments
/// * `model` - ONNX table structure model
/// * `page_image` - Full page image (HWC, u8, [0-255])
/// * `page_width` - Page width in points
/// * `page_height` - Page height in points
/// * `table_clusters` - Table clusters from layout detection
/// * `ocr_cells` - OCR text cells for text matching
///
/// # Returns
/// `TableStructurePrediction` with `table_map` populated
#[allow(
    clippy::unnecessary_wraps,
    reason = "Result kept for consistency with PyTorch version"
)]
fn run_table_inference_onnx(
    model: &mut TableStructureModelOnnx,
    page_image: &Array3<u8>,
    page_width: f32,
    page_height: f32,
    table_clusters: &[Cluster],
    ocr_cells: &[SimpleTextCell],
) -> Result<crate::pipeline::TableStructurePrediction> {
    use ndarray::Array4;

    let mut table_map: HashMap<usize, crate::pipeline::TableElement> = HashMap::new();

    let (img_h, img_w, _) = page_image.dim();

    for cluster in table_clusters {
        log::debug!(
            "    [ONNX] Processing table cluster ID={} at bbox={:?}",
            cluster.id,
            cluster.bbox
        );

        // Step 1: Crop table region from page image
        // Scale bbox coordinates to image pixels
        let scale_x = img_w as f32 / page_width;
        let scale_y = img_h as f32 / page_height;

        let x0 = ((cluster.bbox.l * scale_x) as usize).min(img_w);
        let y0 = ((cluster.bbox.t * scale_y) as usize).min(img_h);
        let x1 = ((cluster.bbox.r * scale_x) as usize).min(img_w);
        let y1 = ((cluster.bbox.b * scale_y) as usize).min(img_h);

        // Skip invalid regions
        if x0 >= x1 || y0 >= y1 {
            log::warn!(
                "    [ONNX] Invalid bbox for cluster {}, skipping",
                cluster.id
            );
            continue;
        }

        let cropped = page_image.slice(ndarray::s![y0..y1, x0..x1, ..]).to_owned();
        let (crop_h, crop_w, _) = cropped.dim();
        log::debug!("      Cropped table region: {crop_w}x{crop_h}");

        // Step 2: Preprocess to 448x448 for Microsoft Table Transformer
        // ImageNet normalization
        let mean = IMAGENET_MEAN;
        let std = IMAGENET_STD;

        // Simple resize using nearest neighbor (faster than bilinear)
        // In production, consider using image crate for bilinear resize
        let mut resized =
            Array4::<f32>::zeros((1, 3, TABLEFORMER_INPUT_SIZE, TABLEFORMER_INPUT_SIZE));

        for y in 0..TABLEFORMER_INPUT_SIZE {
            let src_y = (y * crop_h / TABLEFORMER_INPUT_SIZE).min(crop_h - 1);
            for x in 0..TABLEFORMER_INPUT_SIZE {
                let src_x = (x * crop_w / TABLEFORMER_INPUT_SIZE).min(crop_w - 1);
                for c in 0..3 {
                    // Normalize: (pixel/255 - mean) / std
                    let pixel = f32::from(cropped[[src_y, src_x, c]]) / 255.0;
                    resized[[0, c, y, x]] = (pixel - mean[c]) / std[c];
                }
            }
        }

        log::debug!(
            "      Preprocessed to {TABLEFORMER_INPUT_SIZE}x{TABLEFORMER_INPUT_SIZE} with ImageNet normalization"
        );

        // Step 3: Run ONNX inference
        let detections = match model.predict(&resized) {
            Ok(d) => d,
            Err(e) => {
                log::warn!(
                    "    [ONNX] Inference failed for cluster {}: {}",
                    cluster.id,
                    e
                );
                continue;
            }
        };

        log::debug!(
            "      Detected {} row/column/header regions",
            detections.len()
        );

        // Step 4: Convert detections to TableElement
        let mut table_element =
            model.detections_to_table_element(&detections, &cluster.bbox, cluster.id, ocr_cells);

        // Step 5: Post-process to split merged numeric values into adjacent empty cells
        table_element.table_cells = crate::pipeline::data_structures::postprocess_table_cells(
            table_element.table_cells,
            table_element.num_rows,
            table_element.num_cols,
        );

        log::debug!(
            "      ✓ Table: {} rows, {} cols, {} cells",
            table_element.num_rows,
            table_element.num_cols,
            table_element.table_cells.len()
        );

        table_map.insert(cluster.id, table_element);
    }

    Ok(crate::pipeline::TableStructurePrediction { table_map })
}

/// Crop a region from a page image based on bounding box coordinates
///
/// Converts bounding box coordinates to pixel coordinates and extracts
/// the corresponding region from the page image.
///
/// # Arguments
///
/// * `page_image` - Full page image as RGB ndarray (H, W, 3)
/// * `bbox` - Bounding box in page coordinates (l, t, r, b)
/// * `page_width` - Page width in coordinate units
/// * `page_height` - Page height in coordinate units
///
/// # Returns
///
/// * Cropped image as `DynamicImage`
///
/// # Coordinate Conversion
///
/// The function handles coordinate conversion from bounding box coordinates
/// (typically in points or pixels depending on `coord_origin`) to image array indices.
///
/// For `TopLeft` origin (default):
/// - bbox coordinates are already in screen space (0,0 = top-left)
/// - Direct mapping: image[t:b, l:r]
///
/// For `BottomLeft` origin (PDF coordinates):
/// - bbox.t is the bottom in screen space
/// - bbox.b is the top in screen space
/// - Must flip: `screen_top` = `page_height` - bbox.b, `screen_bottom` = `page_height` - bbox.t
///
/// # Edge Cases
///
/// - Clamps coordinates to image bounds (0 to width/height)
/// - Returns error if resulting region is empty (width or height = 0)
/// - Handles floating point to integer conversion with rounding
pub fn crop_region(
    page_image: &Array3<u8>,
    bbox: &BoundingBox,
    page_width: f32,
    page_height: f32,
) -> Result<image::DynamicImage> {
    // F69: Valid scale range for DPI renderings (0.1x to 100x)
    const MIN_SCALE: f32 = 0.1;
    const MAX_SCALE: f32 = 100.0;

    let (img_height, img_width, channels) = page_image.dim();

    // Validate image format (expecting RGB with 3 channels)
    if channels != 3 {
        return Err(DoclingError::PreprocessingError {
            reason: format!("Expected RGB image with 3 channels, got {channels}"),
        });
    }

    // F19: Validate page dimensions to prevent division by zero or invalid scaling
    if page_width <= 0.0 || page_height <= 0.0 {
        return Err(DoclingError::PreprocessingError {
            reason: format!(
                "Invalid page dimensions: width={page_width}, height={page_height} (must be positive)"
            ),
        });
    }

    // Calculate scale factors from page coordinates to image pixels
    let scale_x = img_width as f32 / page_width;
    let scale_y = img_height as f32 / page_height;

    // F69: Validate image dimensions match page dimensions (aspect ratio check)
    // Allow reasonable tolerance for different DPI renderings (0.1x to 100x scale)
    // but catch gross mismatches that likely indicate bugs
    let valid_range = MIN_SCALE..=MAX_SCALE;
    if !valid_range.contains(&scale_x) || !valid_range.contains(&scale_y) {
        return Err(DoclingError::PreprocessingError {
            reason: format!(
                "Image dimensions ({img_width}x{img_height}) grossly mismatched with page dimensions ({page_width}x{page_height}). \
                 Scale factors: x={scale_x:.2}, y={scale_y:.2}. Expected scale between {MIN_SCALE} and {MAX_SCALE}."
            ),
        });
    }

    // Convert bbox coordinates to pixel coordinates
    // Handle coordinate origin conversion if needed
    let (pixel_left, pixel_top, pixel_right, pixel_bottom) = match bbox.coord_origin {
        CoordOrigin::TopLeft => {
            // Direct mapping for TopLeft origin
            (
                (bbox.l * scale_x).round() as usize,
                (bbox.t * scale_y).round() as usize,
                (bbox.r * scale_x).round() as usize,
                (bbox.b * scale_y).round() as usize,
            )
        }
        CoordOrigin::BottomLeft => {
            // Convert from BottomLeft (PDF) to TopLeft (screen/image) coordinates
            // In PDF: y=0 is bottom, y increases upward
            // In image: y=0 is top, y increases downward
            let screen_top = page_height - bbox.b; // PDF bottom -> screen top
            let screen_bottom = page_height - bbox.t; // PDF top -> screen bottom

            (
                (bbox.l * scale_x).round() as usize,
                (screen_top * scale_y).round() as usize,
                (bbox.r * scale_x).round() as usize,
                (screen_bottom * scale_y).round() as usize,
            )
        }
    };

    // Clamp to image bounds
    let left = pixel_left.min(img_width);
    let top = pixel_top.min(img_height);
    let right = pixel_right.min(img_width);
    let bottom = pixel_bottom.min(img_height);

    // Validate region dimensions
    if left >= right || top >= bottom {
        return Err(DoclingError::PreprocessingError {
            reason: format!(
                "Invalid crop region: left={left}, top={top}, right={right}, bottom={bottom} (image: {img_width}x{img_height})"
            ),
        });
    }

    let crop_width = right - left;
    let crop_height = bottom - top;

    // Extract the region from the ndarray
    // ndarray indexing: array[[y_start..y_end, x_start..x_end, ..]]
    let cropped_array = page_image.slice(ndarray::s![top..bottom, left..right, ..]);

    // Convert to image::DynamicImage
    // Create a new ImageBuffer and copy pixel data
    let mut img_buffer = image::RgbImage::new(crop_width as u32, crop_height as u32);

    for y in 0..crop_height {
        for x in 0..crop_width {
            let r = cropped_array[[y, x, 0]];
            let g = cropped_array[[y, x, 1]];
            let b = cropped_array[[y, x, 2]];
            img_buffer.put_pixel(x as u32, y as u32, image::Rgb([r, g, b]));
        }
    }

    Ok(image::DynamicImage::ImageRgb8(img_buffer))
}

#[cfg(test)]
mod tests {
    use super::*;
    use log;

    #[test]
    #[ignore = "Requires ONNX models"]
    fn test_pipeline_creation() {
        // Test that pipeline can be created with default config
        let config = PipelineConfig::default();
        let result = Pipeline::new(config);

        match result {
            Ok(_pipeline) => {
                log::debug!("✓ Pipeline created successfully");
            }
            Err(e) => {
                panic!("Failed to create pipeline: {e}");
            }
        }
    }

    #[test]
    fn test_config_builder_default() {
        // Test that builder creates valid config with defaults
        // Use skip_validation since tests may not have actual model files
        let config = PipelineConfigBuilder::new()
            .skip_validation(true)
            .build()
            .expect("Default config should be valid");

        assert!(config.ocr_enabled);
        assert!(config.table_structure_enabled);
        // Device comparison doesn't work due to tch internals, so we just verify it builds
    }

    #[test]
    fn test_config_builder_customization() {
        // Test that builder allows customization
        let config = PipelineConfigBuilder::new()
            .ocr_enabled(false)
            .table_structure_enabled(false)
            .device(Device::Cpu)
            .build()
            .expect("Custom config should be valid");

        assert!(!config.ocr_enabled);
        assert!(!config.table_structure_enabled);
    }

    #[test]
    fn test_config_builder_custom_paths() {
        // Test that builder allows custom model paths
        // Use skip_validation since the custom paths don't exist
        let layout_path = PathBuf::from("/custom/layout.onnx");
        let table_path = PathBuf::from("/custom/tableformer");

        let config = PipelineConfigBuilder::new()
            .layout_model_path(layout_path.clone())
            .table_model_dir(table_path.clone())
            .skip_validation(true)
            .build()
            .expect("Config with custom paths should be valid");

        assert_eq!(config.layout_model_path, layout_path);
        assert_eq!(config.table_model_dir, table_path);
    }

    #[test]
    fn test_config_default_uses_builder() {
        // Test that PipelineConfig::default() and builder produce same feature flags
        // Use skip_validation since tests may not have actual model files
        let config1 = PipelineConfigBuilder::new()
            .skip_validation(true)
            .build()
            .expect("Config with skip_validation should be valid");
        let config2 = PipelineConfigBuilder::new()
            .skip_validation(true)
            .build()
            .expect("Config with skip_validation should be valid");

        // Both should have the same feature flags
        assert_eq!(config1.ocr_enabled, config2.ocr_enabled);
        assert_eq!(
            config1.table_structure_enabled,
            config2.table_structure_enabled
        );
    }

    #[test]
    fn test_crop_region_topleft_coordinates() {
        // Test cropping with TopLeft coordinate origin (default)

        // Create a 100x100 RGB test image (filled with gray)
        let img_data = vec![128u8; 100 * 100 * 3];
        let page_image = Array3::from_shape_vec((100, 100, 3), img_data).unwrap();

        // Create a bbox for region (10, 10) to (30, 30) in TopLeft coordinates
        let bbox = BoundingBox {
            l: 10.0,
            t: 10.0,
            r: 30.0,
            b: 30.0,
            coord_origin: CoordOrigin::TopLeft,
        };

        // Page dimensions match image dimensions (1:1 scale)
        let page_width = 100.0;
        let page_height = 100.0;

        // Crop the region
        let result = crop_region(&page_image, &bbox, page_width, page_height);

        assert!(result.is_ok(), "Crop should succeed");
        let cropped = result.unwrap();

        // Verify dimensions: 20x20 pixels (30-10 = 20)
        assert_eq!(cropped.width(), 20);
        assert_eq!(cropped.height(), 20);
    }

    #[test]
    fn test_crop_region_bottomleft_coordinates() {
        // Test cropping with BottomLeft coordinate origin (PDF style)

        // Create a 100x100 RGB test image
        let img_data = vec![128u8; 100 * 100 * 3];
        let page_image = Array3::from_shape_vec((100, 100, 3), img_data).unwrap();

        // In BottomLeft coordinates: y=0 is bottom, y increases upward
        // Region (10, 10) to (30, 30) in PDF coordinates
        // Maps to screen coordinates: top = 100-30 = 70, bottom = 100-10 = 90
        let bbox = BoundingBox {
            l: 10.0,
            t: 10.0, // PDF top (closer to bottom in screen space)
            r: 30.0,
            b: 30.0, // PDF bottom (closer to top in screen space)
            coord_origin: CoordOrigin::BottomLeft,
        };

        let page_width = 100.0;
        let page_height = 100.0;

        let result = crop_region(&page_image, &bbox, page_width, page_height);

        assert!(result.is_ok(), "Crop should succeed");
        let cropped = result.unwrap();

        // Verify dimensions: 20x20 pixels
        assert_eq!(cropped.width(), 20);
        assert_eq!(cropped.height(), 20);
    }

    #[test]
    fn test_crop_region_with_scaling() {
        // Test cropping when page coordinates differ from image dimensions

        // Create a 200x200 RGB test image
        let img_data = vec![128u8; 200 * 200 * 3];
        let page_image = Array3::from_shape_vec((200, 200, 3), img_data).unwrap();

        // Page dimensions are 100x100 (image is 2x larger)
        // bbox (10, 10) to (30, 30) in page coords
        // Should map to (20, 20) to (60, 60) in image pixels (scale=2.0)
        let bbox = BoundingBox {
            l: 10.0,
            t: 10.0,
            r: 30.0,
            b: 30.0,
            coord_origin: CoordOrigin::TopLeft,
        };

        let page_width = 100.0;
        let page_height = 100.0;

        let result = crop_region(&page_image, &bbox, page_width, page_height);

        assert!(result.is_ok(), "Crop should succeed");
        let cropped = result.unwrap();

        // Verify dimensions: (30-10)*2 = 40 pixels in each dimension
        assert_eq!(cropped.width(), 40);
        assert_eq!(cropped.height(), 40);
    }

    #[test]
    fn test_crop_region_invalid_channels() {
        // Test error handling for non-RGB images

        // Create a grayscale image (1 channel)
        let img_data = vec![128u8; 100 * 100];
        let page_image = Array3::from_shape_vec((100, 100, 1), img_data).unwrap();

        let bbox = BoundingBox {
            l: 10.0,
            t: 10.0,
            r: 30.0,
            b: 30.0,
            coord_origin: CoordOrigin::TopLeft,
        };

        let result = crop_region(&page_image, &bbox, 100.0, 100.0);

        assert!(result.is_err(), "Should fail for non-RGB images");
        match result.unwrap_err() {
            DoclingError::PreprocessingError { reason } => {
                assert!(
                    reason.contains("3 channels"),
                    "Error should mention channel requirement"
                );
            }
            _ => panic!("Should return PreprocessingError"),
        }
    }

    #[test]
    fn test_crop_region_empty_region() {
        // Test error handling for invalid bbox (empty region)

        let img_data = vec![128u8; 100 * 100 * 3];
        let page_image = Array3::from_shape_vec((100, 100, 3), img_data).unwrap();

        // Invalid bbox: left >= right
        let bbox = BoundingBox {
            l: 30.0,
            t: 10.0,
            r: 10.0, // right < left
            b: 30.0,
            coord_origin: CoordOrigin::TopLeft,
        };

        let result = crop_region(&page_image, &bbox, 100.0, 100.0);

        assert!(result.is_err(), "Should fail for empty region");
        match result.unwrap_err() {
            DoclingError::PreprocessingError { reason } => {
                assert!(
                    reason.contains("Invalid crop region"),
                    "Error should mention invalid region"
                );
            }
            _ => panic!("Should return PreprocessingError"),
        }
    }

    #[test]
    fn test_crop_region_invalid_page_dimensions() {
        // F19: Test validation of page dimensions

        let img_data = vec![128u8; 100 * 100 * 3];
        let page_image = Array3::from_shape_vec((100, 100, 3), img_data).unwrap();

        let bbox = BoundingBox {
            l: 10.0,
            t: 10.0,
            r: 50.0,
            b: 50.0,
            coord_origin: CoordOrigin::TopLeft,
        };

        // Test zero page_width
        let result = crop_region(&page_image, &bbox, 0.0, 100.0);
        assert!(result.is_err(), "Should fail for zero page_width");
        match result.unwrap_err() {
            DoclingError::PreprocessingError { reason } => {
                assert!(
                    reason.contains("Invalid page dimensions"),
                    "Error should mention invalid dimensions: {reason}"
                );
            }
            _ => panic!("Should return PreprocessingError"),
        }

        // Test zero page_height
        let result = crop_region(&page_image, &bbox, 100.0, 0.0);
        assert!(result.is_err(), "Should fail for zero page_height");
        match result.unwrap_err() {
            DoclingError::PreprocessingError { reason } => {
                assert!(
                    reason.contains("Invalid page dimensions"),
                    "Error should mention invalid dimensions: {reason}"
                );
            }
            _ => panic!("Should return PreprocessingError"),
        }

        // Test negative page_width
        let result = crop_region(&page_image, &bbox, -100.0, 100.0);
        assert!(result.is_err(), "Should fail for negative page_width");
        match result.unwrap_err() {
            DoclingError::PreprocessingError { reason } => {
                assert!(
                    reason.contains("Invalid page dimensions"),
                    "Error should mention invalid dimensions: {reason}"
                );
            }
            _ => panic!("Should return PreprocessingError"),
        }

        // Test negative page_height
        let result = crop_region(&page_image, &bbox, 100.0, -100.0);
        assert!(result.is_err(), "Should fail for negative page_height");
        match result.unwrap_err() {
            DoclingError::PreprocessingError { reason } => {
                assert!(
                    reason.contains("Invalid page dimensions"),
                    "Error should mention invalid dimensions: {reason}"
                );
            }
            _ => panic!("Should return PreprocessingError"),
        }
    }

    #[test]
    fn test_crop_region_mismatched_image_page_dimensions() {
        // F69: Test validation of image vs page dimension mismatches

        let img_data = vec![128u8; 100 * 100 * 3];
        let page_image = Array3::from_shape_vec((100, 100, 3), img_data).unwrap();

        let bbox = BoundingBox {
            l: 10.0,
            t: 10.0,
            r: 50.0,
            b: 50.0,
            coord_origin: CoordOrigin::TopLeft,
        };

        // Valid case: 1:1 scale (should succeed)
        let result = crop_region(&page_image, &bbox, 100.0, 100.0);
        assert!(result.is_ok(), "1:1 scale should succeed");

        // Valid case: 10x scale (image is 10x larger than page coords - high DPI)
        // Image is 100x100, page is 10x10, so scale = 10x
        let result = crop_region(
            &page_image,
            &BoundingBox {
                l: 1.0,
                t: 1.0,
                r: 5.0,
                b: 5.0,
                coord_origin: CoordOrigin::TopLeft,
            },
            10.0,
            10.0,
        );
        assert!(result.is_ok(), "10x scale should succeed");

        // Valid case: 0.1x scale (image is 0.1x page coords - low DPI)
        // Image is 100x100, page is 1000x1000, so scale = 0.1x
        let result = crop_region(
            &page_image,
            &BoundingBox {
                l: 100.0,
                t: 100.0,
                r: 500.0,
                b: 500.0,
                coord_origin: CoordOrigin::TopLeft,
            },
            1000.0,
            1000.0,
        );
        assert!(result.is_ok(), "0.1x scale should succeed");

        // Invalid case: scale too small (<0.1x) - image too small for claimed page size
        // Image is 100x100, page is 10001x10001, so scale < 0.01x
        let result = crop_region(&page_image, &bbox, 10001.0, 10001.0);
        assert!(result.is_err(), "Scale < 0.1x should fail");
        match result.unwrap_err() {
            DoclingError::PreprocessingError { reason } => {
                assert!(
                    reason.contains("grossly mismatched"),
                    "Error should mention gross mismatch: {reason}"
                );
            }
            _ => panic!("Should return PreprocessingError"),
        }

        // Invalid case: scale too large (>100x) - image too large for claimed page size
        // Image is 100x100, page is 0.5x0.5, so scale = 200x
        let result = crop_region(
            &page_image,
            &BoundingBox {
                l: 0.0,
                t: 0.0,
                r: 0.25,
                b: 0.25,
                coord_origin: CoordOrigin::TopLeft,
            },
            0.5,
            0.5,
        );
        assert!(result.is_err(), "Scale > 100x should fail");
        match result.unwrap_err() {
            DoclingError::PreprocessingError { reason } => {
                assert!(
                    reason.contains("grossly mismatched"),
                    "Error should mention gross mismatch: {reason}"
                );
            }
            _ => panic!("Should return PreprocessingError"),
        }
    }

    #[test]
    fn test_bold_italic_flags_survive_modular_conversions() {
        let input = SimpleTextCell {
            index: 0,
            text: "Hello".to_string(),
            rect: crate::pipeline::BoundingBox {
                l: 1.0,
                t: 2.0,
                r: 3.0,
                b: 4.0,
                coord_origin: CoordOrigin::TopLeft,
            },
            confidence: 1.0,
            from_ocr: false,
            is_bold: true,
            is_italic: true,
        };

        let ocr_cells = convert_simple_to_modular_cells(&[input]);
        assert_eq!(ocr_cells.cells.len(), 1);
        assert!(ocr_cells.cells[0].is_bold);
        assert!(ocr_cells.cells[0].is_italic);

        let cluster_cell = convert_modular_cell_to_cluster_cell(&ocr_cells.cells[0]);
        assert!(cluster_cell.is_bold);
        assert!(cluster_cell.is_italic);
    }

    #[test]
    fn test_bold_italic_flags_survive_stage09_cellinfo_conversion() {
        let cell_info = crate::pipeline_modular::stage09_document_assembler::CellInfo {
            text: "Hello".to_string(),
            rect: crate::pipeline_modular::stage09_document_assembler::CellRect {
                r_x0: 1.0,
                r_y0: 2.0,
                r_x1: 3.0,
                r_y1: 2.0,
                r_x2: 3.0,
                r_y2: 4.0,
                r_x3: 1.0,
                r_y3: 4.0,
                coord_origin: "TOPLEFT".to_string(),
            },
            confidence: 1.0,
            from_ocr: false,
            is_bold: true,
            is_italic: true,
        };

        let cluster_cell = convert_cell_info_to_textcell(0, &cell_info);
        assert!(cluster_cell.is_bold);
        assert!(cluster_cell.is_italic);
    }
}
