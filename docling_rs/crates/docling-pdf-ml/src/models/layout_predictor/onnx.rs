// Layout predictor ONNX backend
// Note: Infrastructure code - some helpers ported from Python not yet wired up.
#![allow(dead_code)]
// Image dimensions and coordinates are converted between usize (array indexing)
// and f32/i32 (normalized model I/O). Precision loss is acceptable for dimensions < 10000.
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]

use crate::baseline::{BBox, LayoutCluster};
#[cfg(feature = "pytorch")]
use crate::preprocessing::layout::layout_preprocess;
use crate::preprocessing::layout::{
    layout_preprocess_with_size, LayoutResolution, DEFAULT_LAYOUT_RESOLUTION,
};
use ndarray::{Array, Array3, Array4, ArrayD};
use ort::execution_providers::{
    CPUExecutionProvider, CUDAExecutionProvider, CoreMLExecutionProvider,
};
use ort::session::Session;
use std::path::Path;

// PyTorch dependencies - only available with pytorch feature
#[cfg(feature = "pytorch")]
use tch::{nn, CModule, Device, Tensor};

// Use stub Device when pytorch is disabled
#[cfg(not(feature = "pytorch"))]
use crate::pipeline::Device;

#[cfg(feature = "pytorch")]
use super::pytorch_backend::{
    model::{RTDetrV2Config, RTDetrV2ForObjectDetection},
    weights,
};

/// Inference backend for `LayoutPredictor`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InferenceBackend {
    /// ONNX Runtime backend
    ///
    /// Uses ONNX Runtime with execution providers:
    /// - CPU: Pure CPU inference
    /// - CUDA: GPU acceleration with CPU fallback
    /// - `CoreML`: Apple Silicon acceleration (macOS)
    ///
    /// Pros: Cross-platform, mature, good CPU performance
    /// Cons: Limited GPU utilization, slower than `PyTorch` on GPU
    ONNX,

    /// PyTorch (libtorch) backend
    ///
    /// **STATUS: IMPLEMENTED (N=465)** - Full RT-DETR implementation validated.
    /// **DEFAULT: Changed to PyTorch (N=486)** - 1.56x faster than ONNX (N=485).
    ///
    /// Uses PyTorch C++ library via tch-rs bindings with full RT-DETR v2 implementation.
    /// Requires safetensors weights from HuggingFace (not TorchScript).
    ///
    /// **Implementation:** Native RT-DETR architecture in Rust (N=387-464)
    /// - ResNet backbone, hybrid encoder, 6-layer decoder
    /// - Systematic validation: 100% match with Python (tolerance 1e-3)
    /// - Divergence: logits 4.05e-4, boxes 2.99e-5
    ///
    /// **Performance (N=485):**
    /// - PyTorch: 153.43 ms/page (6.52 pages/sec)
    /// - ONNX: 239.35 ms/page (4.18 pages/sec)
    /// - Speedup: 1.56x faster (85.93 ms improvement, 35.9% faster)
    ///
    /// **See:**
    /// - CLAUDE.md (PyTorch Backend Status section)
    /// - reports/.../n485_pytorch_backend_performance_validation_2025-11-12-23-15.md
    /// - reports/.../n464_decoder_validation_complete_2025-11-12-16-05.md
    ///
    /// Pros: Better GPU utilization, 1.56x faster, higher throughput, full control
    /// Cons: Larger binary size, requires libtorch, more complex implementation
    ///
    /// **NOTE:** Only available with the `pytorch` feature enabled.
    #[cfg(feature = "pytorch")]
    PyTorch,
}

impl Default for InferenceBackend {
    #[inline]
    fn default() -> Self {
        // When pytorch feature is enabled, default to PyTorch (faster)
        #[cfg(feature = "pytorch")]
        return Self::PyTorch;
        // When pytorch feature is disabled, default to ONNX
        #[cfg(not(feature = "pytorch"))]
        return Self::ONNX;
    }
}

impl std::fmt::Display for InferenceBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ONNX => write!(f, "onnx"),
            #[cfg(feature = "pytorch")]
            Self::PyTorch => write!(f, "pytorch"),
        }
    }
}

impl std::str::FromStr for InferenceBackend {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "onnx" => Ok(Self::ONNX),
            #[cfg(feature = "pytorch")]
            "pytorch" | "torch" | "libtorch" => Ok(Self::PyTorch),
            #[cfg(not(feature = "pytorch"))]
            "pytorch" | "torch" | "libtorch" => Err(
                "PyTorch backend not available. Enable 'pytorch' feature to use PyTorch."
                    .to_string(),
            ),
            _ => {
                #[cfg(feature = "pytorch")]
                let expected = "onnx, pytorch";
                #[cfg(not(feature = "pytorch"))]
                let expected = "onnx";
                Err(format!("Unknown backend '{s}'. Expected: {expected}"))
            }
        }
    }
}

pub struct LayoutPredictorModel {
    backend: InferenceBackend,
    onnx_session: Option<Session>,
    #[cfg(feature = "pytorch")]
    pytorch_model: Option<CModule>, // Deprecated - kept for backward compatibility
    #[cfg(feature = "pytorch")]
    pytorch_varstore: Option<nn::VarStore>,
    #[cfg(feature = "pytorch")]
    pytorch_rtdetr: Option<RTDetrV2ForObjectDetection>,
    #[allow(
        dead_code,
        reason = "stored for potential future device-based optimizations"
    )]
    device: Device,
}

impl LayoutPredictorModel {
    /// Load `LayoutPredictor` model with ONNX backend (default)
    ///
    /// Convenience method that defaults to ONNX backend for backward compatibility.
    /// For `PyTorch` backend, use `load_with_backend()`.
    ///
    /// # Arguments
    ///
    /// * `model_path` - Path to ONNX model file (.onnx)
    /// * `device` - Device to run inference on (CPU, CUDA, MPS)
    ///
    /// # Returns
    ///
    /// Loaded model ready for inference
    ///
    /// # Errors
    ///
    /// Returns error if model file not found or failed to load
    #[must_use = "returns loaded model that should be used"]
    pub fn load(model_path: &Path, device: Device) -> Result<Self, Box<dyn std::error::Error>> {
        Self::load_with_backend(model_path, device, InferenceBackend::ONNX)
    }

    /// Load `LayoutPredictor` model with specified backend
    ///
    /// # Arguments
    ///
    /// * `model_path` - Path to model file (.onnx for ONNX, .pt for `PyTorch`)
    /// * `device` - Device to run inference on (CPU, CUDA, MPS)
    /// * `backend` - Inference backend to use (ONNX or `PyTorch`)
    ///
    /// # Returns
    ///
    /// Loaded model ready for inference
    ///
    /// # Errors
    ///
    /// Returns error if model file not found or failed to load
    #[must_use = "returns loaded model that should be used"]
    pub fn load_with_backend(
        model_path: &Path,
        device: Device,
        backend: InferenceBackend,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        match backend {
            InferenceBackend::ONNX => Self::load_onnx(model_path, device),
            #[cfg(feature = "pytorch")]
            InferenceBackend::PyTorch => Self::load_pytorch(model_path, device),
        }
    }

    /// Load ONNX backend
    fn load_onnx(model_path: &Path, device: Device) -> Result<Self, Box<dyn std::error::Error>> {
        // Determine optimal thread count (use physical cores, not hyperthreads)
        // Can be overridden via LAYOUT_ONNX_THREADS environment variable
        let num_threads = std::env::var("LAYOUT_ONNX_THREADS")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or_else(|| {
                // Default to physical core count, clamped to reasonable range
                let physical_cores = std::thread::available_parallelism()
                    .map(|p| p.get() / 2) // Assume 2x hyperthreading
                    .unwrap_or(4);
                physical_cores.clamp(1, 8)
            });

        log::debug!("LayoutPredictor ONNX session using {num_threads} threads");

        // Allow forcing CPU execution provider for debugging
        // Note (N=3614): CoreML was initially suspected of producing incorrect results,
        // but investigation confirmed both CPU and CoreML produce identical correct outputs.
        let effective_device = if std::env::var("ORT_FORCE_CPU").is_ok() {
            log::debug!("ORT_FORCE_CPU set - forcing CPU execution provider");
            Device::Cpu
        } else {
            device
        };

        // Configure execution provider based on device
        let session = match effective_device {
            Device::Cpu => {
                log::debug!("Creating LayoutPredictor session with CPU execution provider");
                Session::builder()?
                    .with_intra_threads(num_threads)?
                    .commit_from_file(model_path)?
            }
            Device::Cuda(_) => {
                log::debug!("Creating LayoutPredictor session with CUDA execution provider");
                // ONNX Runtime CUDA with CPU fallback
                Session::builder()?
                    .with_execution_providers([
                        CUDAExecutionProvider::default().build(),
                        CPUExecutionProvider::default().build(),
                    ])?
                    .commit_from_file(model_path)?
            }
            Device::Mps => {
                log::debug!("Creating LayoutPredictor session with CoreML execution provider (MPS not supported by ONNX Runtime)");
                // ONNX Runtime doesn't support MPS, but CoreML can leverage Apple Silicon
                Session::builder()?
                    .with_execution_providers([CoreMLExecutionProvider::default().build()])?
                    .commit_from_file(model_path)?
            }
            #[cfg(feature = "pytorch")]
            Device::Vulkan => {
                log::warn!("Vulkan device not supported by ONNX Runtime, falling back to CPU");
                Session::builder()?.commit_from_file(model_path)?
            }
        };

        log::debug!("Loaded LayoutPredictor ONNX model on device: {effective_device:?}");
        log::debug!(
            "  Inputs: {:?}",
            session.inputs.iter().map(|i| &i.name).collect::<Vec<_>>()
        );
        log::debug!(
            "  Outputs: {:?}",
            session.outputs.iter().map(|o| &o.name).collect::<Vec<_>>()
        );

        Ok(Self {
            backend: InferenceBackend::ONNX,
            onnx_session: Some(session),
            #[cfg(feature = "pytorch")]
            pytorch_model: None,
            #[cfg(feature = "pytorch")]
            pytorch_varstore: None,
            #[cfg(feature = "pytorch")]
            pytorch_rtdetr: None,
            device: effective_device,
        })
    }

    /// Load PyTorch backend (Full RT-DETR implementation)
    ///
    /// Loads the full RT-DETR v2 model with safetensors weights from HuggingFace.
    /// This implementation uses the native PyTorch backend validated in N=449-464.
    ///
    /// # Arguments
    ///
    /// * `model_path` - Path to model safetensors file (e.g., from HuggingFace cache)
    /// * `device` - Device to run inference on (CPU, CUDA, MPS)
    ///
    /// # Returns
    ///
    /// Loaded model ready for inference
    ///
    /// # Errors
    ///
    /// Returns error if model file not found or failed to load
    #[cfg(feature = "pytorch")]
    fn load_pytorch(model_path: &Path, device: Device) -> Result<Self, Box<dyn std::error::Error>> {
        log::debug!(
            "Loading LayoutPredictor PyTorch model from {:?}",
            model_path
        );
        log::debug!("  Device: {:?}", device);

        // Set environment variables for PyTorch
        std::env::set_var("LIBTORCH_USE_PYTORCH", "1");
        std::env::set_var("LIBTORCH_BYPASS_VERSION_CHECK", "1");

        // Read config from HuggingFace (model.safetensors is in same dir as config.json)
        let config_path = model_path.parent().unwrap().join("config.json");
        let config = if config_path.exists() {
            let config_str = std::fs::read_to_string(&config_path)?;
            let config_json: serde_json::Value = serde_json::from_str(&config_str)?;

            let mut cfg = RTDetrV2Config::default();
            cfg.num_labels = config_json["num_labels"]
                .as_i64()
                .or_else(|| {
                    config_json
                        .get("id2label")
                        .and_then(|v| v.as_object())
                        .map(|obj| obj.len() as i64)
                })
                .unwrap_or(cfg.num_labels);
            cfg.d_model = config_json["d_model"].as_i64().unwrap_or(cfg.d_model);
            cfg.encoder_hidden_dim = config_json["encoder_hidden_dim"]
                .as_i64()
                .unwrap_or(cfg.encoder_hidden_dim);
            cfg.num_queries = config_json["num_queries"]
                .as_i64()
                .unwrap_or(cfg.num_queries);
            cfg.decoder_layers = config_json["decoder_layers"]
                .as_i64()
                .unwrap_or(cfg.decoder_layers);
            if let Some(encode_proj) = config_json.get("encode_proj_layers") {
                if let Some(arr) = encode_proj.as_array() {
                    cfg.encode_proj_layers = arr.iter().filter_map(|v| v.as_i64()).collect();
                }
            }
            cfg
        } else {
            RTDetrV2Config::default()
        };

        log::debug!(
            "  Config: {} labels, {} decoder layers",
            config.num_labels,
            config.decoder_layers
        );

        // Create model
        let mut vs = nn::VarStore::new(device);
        let model = RTDetrV2ForObjectDetection::new(&vs.root(), config)
            .map_err(|e| format!("Failed to create RT-DETR model: {}", e))?;

        // Load weights
        weights::load_weights_into(&mut vs, model_path)
            .map_err(|e| format!("Failed to load weights: {}", e))?;

        vs.freeze();

        log::debug!("Loaded LayoutPredictor PyTorch model successfully");

        Ok(LayoutPredictorModel {
            backend: InferenceBackend::PyTorch,
            onnx_session: None,
            pytorch_model: None,
            pytorch_varstore: Some(vs),
            pytorch_rtdetr: Some(model),
            device,
        })
    }

    /// Preprocess image according to `RTDetrImageProcessor` config
    /// - Resize to 640x640
    /// - Rescale pixels by 1/255 (`do_rescale`: true, `rescale_factor`: 0.00392156862745098)
    /// - NO normalization (`do_normalize`: false in config)
    /// - Convert to NCHW format (batch, channels, height, width)
    fn preprocess_image(
        &self,
        image: &ArrayD<f32>,
    ) -> Result<Array4<f32>, Box<dyn std::error::Error>> {
        // Input image is HWC (height, width, channels) in range [0, 255]
        // Target size from preprocessor config
        let target_size = DEFAULT_LAYOUT_RESOLUTION;

        // Resize image to target_size x target_size using bilinear interpolation
        let resized = self.resize_bilinear(image, target_size, target_size)?;

        // Rescale: divide by 255 (do_rescale: true, rescale_factor: 1/255)
        let rescaled = &resized / 255.0;

        // Convert to NCHW format (batch, channels, height, width)
        // NO normalization - the RT-DETR model config has do_normalize: false
        let mut output = Array4::<f32>::zeros((1, 3, target_size, target_size));

        for c in 0..3 {
            for h in 0..target_size {
                for w in 0..target_size {
                    output[[0, c, h, w]] = rescaled[[h, w, c]];
                }
            }
        }

        Ok(output)
    }

    /// Resize image using bilinear interpolation
    #[allow(
        clippy::unnecessary_wraps,
        reason = "Result kept for consistency with other image processing methods"
    )]
    // Method signature kept for API consistency with other LayoutPredictor methods
    #[allow(clippy::unused_self)]
    fn resize_bilinear(
        &self,
        image: &ArrayD<f32>,
        target_h: usize,
        target_w: usize,
    ) -> Result<Array3<f32>, Box<dyn std::error::Error>> {
        let shape = image.shape();
        let (orig_h, orig_w, channels) = (shape[0], shape[1], shape[2]);

        let mut resized = Array::zeros((target_h, target_w, channels));

        let scale_h = orig_h as f32 / target_h as f32;
        let scale_w = orig_w as f32 / target_w as f32;

        for c in 0..channels {
            for h in 0..target_h {
                for w in 0..target_w {
                    // Calculate source coordinates
                    let src_h = h as f32 * scale_h;
                    let src_w = w as f32 * scale_w;

                    // Get integer parts
                    let h0 = src_h.floor() as usize;
                    let w0 = src_w.floor() as usize;
                    let h1 = (h0 + 1).min(orig_h - 1);
                    let w1 = (w0 + 1).min(orig_w - 1);

                    // Get fractional parts
                    let dh = src_h - h0 as f32;
                    let dw = src_w - w0 as f32;

                    // Bilinear interpolation
                    let p00 = image[[h0, w0, c]];
                    let p01 = image[[h0, w1, c]];
                    let p10 = image[[h1, w0, c]];
                    let p11 = image[[h1, w1, c]];

                    let p0 = (1.0 - dw).mul_add(p00, p01 * dw);
                    let p1 = (1.0 - dw).mul_add(p10, p11 * dw);
                    let p = (1.0 - dh).mul_add(p0, p1 * dh);

                    resized[[h, w, c]] = p;
                }
            }
        }

        Ok(resized)
    }

    /// Run inference on image
    ///
    /// Dispatches to backend-specific implementation (ONNX or `PyTorch`)
    pub fn infer(
        &mut self,
        image: &Array3<u8>,
    ) -> Result<Vec<LayoutCluster>, Box<dyn std::error::Error>> {
        self.infer_with_resolution(image, LayoutResolution::Full)
    }

    /// Run inference on image with configurable resolution
    ///
    /// Allows trading accuracy for speed by using lower resolution:
    /// - `Full` (640x640): Baseline accuracy
    /// - `Medium` (512x512): ~1.56x faster, ~1-2% accuracy loss
    /// - `Fast` (448x448): ~2.04x faster, ~3-5% accuracy loss
    ///
    /// # Arguments
    ///
    /// * `image` - Input image (Array3<u8>, HWC format, [0-255])
    /// * `resolution` - Target resolution for inference
    ///
    /// # Returns
    ///
    /// * `Vec<LayoutCluster>` - Detected layout elements with bounding boxes
    ///
    /// # Example
    ///
    /// ```ignore
    /// use docling_pdf_ml::preprocessing::layout::LayoutResolution;
    ///
    /// // Fast mode for quick previews
    /// let clusters = model.infer_with_resolution(&image, LayoutResolution::Fast)?;
    ///
    /// // Medium mode for balanced speed/accuracy
    /// let clusters = model.infer_with_resolution(&image, LayoutResolution::Medium)?;
    /// ```
    pub fn infer_with_resolution(
        &mut self,
        image: &Array3<u8>,
        resolution: LayoutResolution,
    ) -> Result<Vec<LayoutCluster>, Box<dyn std::error::Error>> {
        match self.backend {
            InferenceBackend::ONNX => self.infer_onnx_with_resolution(image, resolution),
            #[cfg(feature = "pytorch")]
            InferenceBackend::PyTorch => self.infer_pytorch_with_resolution(image, resolution),
        }
    }

    /// Run batch inference on multiple images
    ///
    /// More efficient than calling `infer()` repeatedly when processing multiple pages.
    /// Batches ML inference to amortize model overhead and improve GPU utilization.
    ///
    /// # Arguments
    ///
    /// * `images` - Vector of images (Array3<u8>, HWC format, [0-255])
    ///
    /// # Returns
    ///
    /// * `Vec<Vec<LayoutCluster>>` - Clusters for each image (preserves input order)
    ///
    /// # Performance
    ///
    /// - Single page: 60-82 ms/page
    /// - Batch (10 pages): Expected 1.5-2x throughput improvement
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let images = vec![page0_image, page1_image, page2_image];
    /// let results = model.infer_batch(&images)?;
    /// for (i, clusters) in results.iter().enumerate() {
    ///     log::debug!("Page {}: {} clusters", i, clusters.len());
    /// }
    /// ```
    pub fn infer_batch(
        &mut self,
        images: &[Array3<u8>],
    ) -> Result<Vec<Vec<LayoutCluster>>, Box<dyn std::error::Error>> {
        if images.is_empty() {
            return Ok(vec![]);
        }

        match self.backend {
            InferenceBackend::ONNX => {
                // ONNX: Fall back to sequential processing (batching not yet implemented)
                images.iter().map(|img| self.infer_onnx(img)).collect()
            }
            #[cfg(feature = "pytorch")]
            InferenceBackend::PyTorch => self.infer_pytorch_batch(images),
        }
    }

    /// ONNX inference implementation (default 640x640 resolution)
    fn infer_onnx(
        &mut self,
        image: &Array3<u8>,
    ) -> Result<Vec<LayoutCluster>, Box<dyn std::error::Error>> {
        self.infer_onnx_with_resolution(image, LayoutResolution::Full)
    }

    /// ONNX inference implementation with configurable resolution
    #[allow(clippy::similar_names)] // orig_h_px vs orig_w_px are height/width - intentionally similar
    #[allow(clippy::too_many_lines)]
    fn infer_onnx_with_resolution(
        &mut self,
        image: &Array3<u8>,
        resolution: LayoutResolution,
    ) -> Result<Vec<LayoutCluster>, Box<dyn std::error::Error>> {
        // Get original image dimensions for scaling boxes back
        let (orig_h_px, orig_w_px, _) = image.dim();
        let orig_h = orig_h_px as f32;
        let orig_w = orig_w_px as f32;

        // 1. Preprocess image with specified resolution
        // Default 640x640 for baseline accuracy; lower resolutions trade accuracy for speed
        let target_size = resolution.size();
        let preprocessed = layout_preprocess_with_size(image, target_size);

        // 2. Convert to ort::Value - convert Array4 to (shape, Vec) tuple format
        let shape = preprocessed.shape().to_vec();
        let (data, _offset) = preprocessed.into_raw_vec_and_offset();

        // DEBUG: Print preprocessed tensor stats and save to file
        if std::env::var("DEBUG_ONNX").is_ok() {
            use ndarray::Array4;
            use ndarray_npy::WriteNpyExt;

            let min = data.iter().copied().fold(f32::INFINITY, f32::min);
            let max = data.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            let sum: f32 = data.iter().sum();
            let mean = sum / data.len() as f32;
            eprintln!("\n=== DEBUG: Preprocessed Tensor ===");
            eprintln!("Shape: {shape:?}");
            eprintln!("Stats: min={min:.6}, max={max:.6}, mean={mean:.6}");
            // First 10 values
            eprintln!("First 10 values: {:?}", &data[0..10.min(data.len())]);
            // Values at specific location (batch=0, channel=0, row=300, col=300)
            // idx = batch * C*H*W + channel * H*W + row * W + col = 300 * W + 300
            let idx = 300 * shape[3] + 300;
            eprintln!("Value at (0, 0, 300, 300): {:.6}", data[idx]);
            // Save tensor to file for Python comparison
            let tensor_for_save =
                Array4::from_shape_vec((shape[0], shape[1], shape[2], shape[3]), data.clone())
                    .unwrap();
            let save_path = std::path::Path::new("/tmp/rust_preprocessed_tensor.npy");
            if let Ok(file) = std::fs::File::create(save_path) {
                let _ = tensor_for_save.write_npy(file);
                eprintln!("Saved tensor to /tmp/rust_preprocessed_tensor.npy");
            }
            eprintln!("=== END DEBUG ===\n");
        }

        // Create ort input value using same pattern as Phase 1 test
        let input_value = ort::value::Value::from_array((shape.as_slice(), data.clone()))?;

        // DEBUG: Verify the tensor ort received
        if std::env::var("DEBUG_ONNX").is_ok() {
            let (extracted_shape, extracted_data) =
                input_value.try_extract_tensor::<f32>().unwrap();
            eprintln!("\n=== DEBUG: Tensor passed to ONNX ===");
            eprintln!("Device: {:?}", self.device);
            eprintln!("Shape from ort: {extracted_shape:?}");
            eprintln!("First 10 values from ort: {:?}", &extracted_data[0..10]);
            // Verify shape matches
            let expected_len = shape.iter().product::<usize>();
            eprintln!(
                "Expected len: {}, Actual len: {}",
                expected_len,
                extracted_data.len()
            );
            // Verify data matches
            let data_diff: f32 = data
                .iter()
                .zip(extracted_data.iter())
                .map(|(a, b)| (a - b).abs())
                .sum();
            eprintln!("Data diff sum: {data_diff:.10}");
            eprintln!("=== END DEBUG ===\n");
        }

        // 3. Run ONNX inference
        let session = self
            .onnx_session
            .as_mut()
            .ok_or("ONNX session not loaded")?;
        let outputs = session.run(ort::inputs!["pixel_values" => input_value])?;

        // 4. Extract outputs - logits and pred_boxes
        // try_extract_tensor returns (shape, data_slice)
        let (logits_shape, logits_data) = outputs["logits"].try_extract_tensor::<f32>()?;
        let (_pred_boxes_shape, pred_boxes_data) =
            outputs["pred_boxes"].try_extract_tensor::<f32>()?;

        // Copy data to owned vectors to release the borrow on outputs
        let logits_shape_vec = logits_shape.iter().map(|&d| d as usize).collect::<Vec<_>>();
        let logits_vec = logits_data.to_vec();
        let pred_boxes_vec = pred_boxes_data.to_vec();

        // Drop outputs to release mutable borrow
        drop(outputs);

        // DEBUG: Print raw ONNX outputs for comparison with Python
        if std::env::var("DEBUG_ONNX").is_ok() {
            eprintln!("\n=== DEBUG: Raw ONNX Outputs ===");
            eprintln!("Logits shape: {logits_shape_vec:?}");
            eprintln!(
                "Logits range: [{:.6}, {:.6}]",
                logits_vec.iter().copied().fold(f32::INFINITY, f32::min),
                logits_vec.iter().copied().fold(f32::NEG_INFINITY, f32::max)
            );
            eprintln!(
                "First 17 logits (query 0, all classes): {:?}",
                &logits_vec[0..17.min(logits_vec.len())]
            );
            eprintln!(
                "First 4 pred_boxes (query 0): {:?}",
                &pred_boxes_vec[0..4.min(pred_boxes_vec.len())]
            );
            // Check Text class (index 9) max confidence
            let num_queries = 300;
            let num_classes = 17;
            let text_max_logit = (0..num_queries)
                .map(|q| logits_vec[q * num_classes + 9])
                .fold(f32::NEG_INFINITY, f32::max);
            let text_max_conf = 1.0 / (1.0 + (-text_max_logit).exp());
            eprintln!(
                "Text class (9) max logit: {text_max_logit:.6}, max conf: {text_max_conf:.6}"
            );
            // Check Section-Header class (index 7) max confidence
            let sh_max_logit = (0..num_queries)
                .map(|q| logits_vec[q * num_classes + 7])
                .fold(f32::NEG_INFINITY, f32::max);
            let sh_max_conf = 1.0 / (1.0 + (-sh_max_logit).exp());
            eprintln!(
                "Section-Header class (7) max logit: {sh_max_logit:.6}, max conf: {sh_max_conf:.6}"
            );
            eprintln!("=== END DEBUG ===\n");
        }

        // 5. Post-process: convert to LayoutCluster
        let clusters = self.post_process(
            &logits_shape_vec,
            &logits_vec,
            &pred_boxes_vec,
            orig_w,
            orig_h,
        )?;

        Ok(clusters)
    }

    /// PyTorch inference implementation (Full RT-DETR)
    ///
    /// **STATUS: IMPLEMENTED (N=465)** - Full RT-DETR architecture validated in N=449-464.
    ///
    /// Uses native PyTorch backend with systematic validation:
    /// - Phase 1 (N=449): ResNet backbone validated ✅
    /// - Phase 2 (N=450): Hybrid encoder validated ✅
    /// - Phase 3 (N=451-454): Input preparation validated ✅
    /// - Phase 4 (N=455-464): Decoder (6 layers) validated ✅
    /// - Phase 5 (N=464): End-to-end validation ✅
    ///
    /// Divergence from Python: logits 4.05e-4, boxes 2.99e-5 (within 1e-3 tolerance)
    #[cfg(feature = "pytorch")]
    fn infer_pytorch(
        &mut self,
        image: &Array3<u8>,
    ) -> Result<Vec<LayoutCluster>, Box<dyn std::error::Error>> {
        self.infer_pytorch_with_resolution(image, LayoutResolution::Full)
    }

    /// PyTorch inference implementation with configurable resolution
    #[cfg(feature = "pytorch")]
    fn infer_pytorch_with_resolution(
        &mut self,
        image: &Array3<u8>,
        resolution: LayoutResolution,
    ) -> Result<Vec<LayoutCluster>, Box<dyn std::error::Error>> {
        use std::time::Instant;

        // Get original image dimensions for scaling boxes back
        let (orig_h_px, orig_w_px, _) = image.dim();
        let orig_h = orig_h_px as f32;
        let orig_w = orig_w_px as f32;

        // 1. Preprocess image with specified resolution
        let target_size = resolution.size();
        let preprocess_start = Instant::now();
        let preprocessed = layout_preprocess_with_size(image, target_size);
        let preprocess_time = preprocess_start.elapsed();
        if std::env::var("PROFILE_MODEL").is_ok() {
            log::warn!(
                "[PROFILE] Preprocessing: {:.2} ms",
                preprocess_time.as_secs_f64() * 1000.0
            );
        }

        // DEBUG: Save preprocessing output for comparison
        if let Ok(ref debug_dir) = std::env::var("DEBUG_E2E_TRACE") {
            use ndarray_npy::WriteNpyExt;
            use std::fs::File;
            use std::io::BufWriter;
            use std::path::Path;

            let debug_path = Path::new(debug_dir);
            let prep_path = debug_path.join("stage1_preprocessed.npy");
            if let Ok(file) = File::create(&prep_path) {
                let writer = BufWriter::new(file);
                let _ = preprocessed.write_npy(writer);
                log::info!("💾 Saved preprocessing: {}", prep_path.display());
            }
        }

        // 2. Convert ndarray to tch::Tensor
        let shape = preprocessed.shape().to_vec();
        let (data, _offset): (Vec<f32>, _) = preprocessed.into_raw_vec_and_offset();
        let input_tensor = Tensor::from_slice(&data)
            .reshape([
                shape[0] as i64,
                shape[1] as i64,
                shape[2] as i64,
                shape[3] as i64,
            ])
            .to_device(self.device);

        // 3. Run PyTorch inference
        let model = self
            .pytorch_rtdetr
            .as_ref()
            .ok_or("PyTorch model not loaded")?;

        let forward_start = Instant::now();
        let outputs = model
            .forward(&input_tensor)
            .map_err(|e| format!("PyTorch forward pass failed: {}", e))?;
        let forward_time = forward_start.elapsed();
        if std::env::var("PROFILE_MODEL").is_ok() {
            log::warn!(
                "[PROFILE] Forward pass (total): {:.2} ms",
                forward_time.as_secs_f64() * 1000.0
            );
        }

        // 4. Extract outputs (squeeze batch dimension)
        // outputs.logits: [1, num_queries, num_labels] → [num_queries, num_labels]
        // outputs.pred_boxes: [1, num_queries, 4] → [num_queries, 4]
        let logits = outputs.logits.squeeze_dim(0);
        let pred_boxes = outputs.pred_boxes.squeeze_dim(0);

        // Get shapes
        let logits_size = logits.size();
        let logits_shape_vec = vec![1, logits_size[0] as usize, logits_size[1] as usize];

        // Convert tensors to Vec<f32>
        let logits_flat = logits.flatten(0, -1);
        let pred_boxes_flat = pred_boxes.flatten(0, -1);
        let logits_vec: Vec<f32> = Vec::try_from(&logits_flat)
            .map_err(|e| format!("Failed to convert logits tensor to Vec: {:?}", e))?;
        let pred_boxes_vec: Vec<f32> = Vec::try_from(&pred_boxes_flat)
            .map_err(|e| format!("Failed to convert pred_boxes tensor to Vec: {:?}", e))?;

        // DEBUG: Save raw ML outputs for comparison with Python baseline
        if let Ok(ref debug_dir) = std::env::var("DEBUG_E2E_TRACE") {
            use ndarray::Array3;
            use ndarray_npy::WriteNpyExt;
            use std::fs::File;
            use std::io::BufWriter;
            use std::path::Path;

            let debug_path = Path::new(debug_dir);

            // Save logits: [1, 300, 17]
            let logits_array = Array3::from_shape_vec(
                (1, logits_shape_vec[1], logits_shape_vec[2]),
                logits_vec.clone(),
            )
            .ok();

            if let Some(arr) = logits_array {
                let logits_path = debug_path.join("stage2_rust_logits.npy");
                if let Ok(file) = File::create(&logits_path) {
                    let writer = BufWriter::new(file);
                    let _ = arr.write_npy(writer);
                    log::info!("💾 Saved raw logits: {}", logits_path.display());
                }
            }

            // Save pred_boxes: [1, 300, 4]
            // For now, just save as JSON for easier debugging
            let boxes_path = debug_path.join("stage2_rust_pred_boxes.json");
            let boxes_json = serde_json::json!({
                "shape": [1, 300, 4],
                "data": pred_boxes_vec
            });
            if let Ok(json_str) = serde_json::to_string_pretty(&boxes_json) {
                let _ = std::fs::write(&boxes_path, json_str);
                log::info!("💾 Saved raw boxes: {}", boxes_path.display());
            }
        }

        // DEBUG: Print raw PyTorch outputs for comparison
        if std::env::var("DEBUG_PYTORCH").is_ok() {
            log::debug!("\n=== DEBUG: Raw PyTorch Outputs ===");
            log::debug!("Logits shape: {:?}", logits_shape_vec);
            log::debug!("Pred_boxes shape: {:?}", pred_boxes.size());
            log::debug!(
                "First 20 logits: {:?}",
                &logits_vec[0..17.min(logits_vec.len())]
            );
            log::debug!(
                "First 4 pred_boxes: {:?}",
                &pred_boxes_vec[0..4.min(pred_boxes_vec.len())]
            );
            log::debug!("=== END DEBUG ===\n");
        }

        // 5. Post-process: convert to LayoutCluster (same logic as ONNX)
        let postprocess_start = Instant::now();
        let clusters = self.post_process(
            &logits_shape_vec,
            &logits_vec,
            &pred_boxes_vec,
            orig_w,
            orig_h,
        )?;
        let postprocess_time = postprocess_start.elapsed();
        if std::env::var("PROFILE_MODEL").is_ok() {
            log::warn!(
                "[PROFILE] Post-processing: {:.2} ms",
                postprocess_time.as_secs_f64() * 1000.0
            );
        }

        Ok(clusters)
    }

    /// PyTorch batch inference implementation
    ///
    /// Processes multiple images in a single batch to improve GPU utilization
    /// and amortize model overhead.
    ///
    /// # Implementation Strategy
    ///
    /// 1. Preprocess all images independently
    /// 2. Stack into N×3×640×640 batch tensor
    /// 3. Single forward pass for entire batch
    /// 4. Unpack results and post-process per image
    ///
    /// # Performance
    ///
    /// Expected 1.5-2x throughput improvement for 10-page batch vs sequential processing.
    #[cfg(feature = "pytorch")]
    fn infer_pytorch_batch(
        &mut self,
        images: &[Array3<u8>],
    ) -> Result<Vec<Vec<LayoutCluster>>, Box<dyn std::error::Error>> {
        use std::time::Instant;

        let batch_size = images.len();
        log::debug!("  [Batch] Processing {} pages in batch", batch_size);

        // MPS WORKAROUND: Batch processing with MPS device causes crashes in deformable attention
        // Fall back to sequential processing for batch_size > 1 on MPS
        if batch_size > 1 && matches!(self.device, tch::Device::Mps) {
            log::warn!("  [Batch] MPS device detected with batch_size > 1. Falling back to sequential processing.");
            log::warn!("  [Batch] Note: PyTorch MPS backend has known issues with batch deformable attention.");

            // Process sequentially using single-page inference
            let mut all_clusters = Vec::with_capacity(batch_size);
            for (idx, image) in images.iter().enumerate() {
                log::debug!(
                    "  [Batch/Sequential] Processing page {}/{}",
                    idx + 1,
                    batch_size
                );
                let clusters = self.infer_pytorch(image)?;
                all_clusters.push(clusters);
            }
            return Ok(all_clusters);
        }

        // Store original dimensions for each image (for scaling boxes back)
        let orig_dimensions: Vec<(f32, f32)> = images
            .iter()
            .map(|img| {
                let (h, w, _) = img.dim();
                (w as f32, h as f32)
            })
            .collect();

        // 1. Preprocess all images independently
        let preprocess_start = Instant::now();
        let preprocessed_images: Vec<Array4<f32>> = images.iter().map(layout_preprocess).collect();
        let preprocess_time = preprocess_start.elapsed();
        if std::env::var("PROFILE_MODEL").is_ok() {
            log::warn!(
                "[PROFILE] Batch preprocessing ({} images): {:.2} ms ({:.2} ms/image)",
                batch_size,
                preprocess_time.as_secs_f64() * 1000.0,
                preprocess_time.as_secs_f64() * 1000.0 / batch_size as f64
            );
        }

        // 2. Stack into batch tensor: N×3×640×640
        // Each preprocessed image is 1×3×640×640, we need to concatenate along batch dimension
        let stack_start = Instant::now();
        let batch_tensor = {
            // Convert each Array4 to Tensor and collect
            let tensors: Vec<Tensor> = preprocessed_images
                .iter()
                .map(|arr| {
                    let shape = arr.shape().to_vec();
                    let (data, _offset): (Vec<f32>, _) = arr.clone().into_raw_vec_and_offset();
                    Tensor::from_slice(&data).reshape([
                        shape[0] as i64,
                        shape[1] as i64,
                        shape[2] as i64,
                        shape[3] as i64,
                    ])
                })
                .collect();

            // Stack tensors along batch dimension (dim 0)
            // This converts N tensors of [1, 3, 640, 640] into [N, 3, 640, 640]
            Tensor::cat(&tensors, 0).to_device(self.device)
        };
        let stack_time = stack_start.elapsed();
        if std::env::var("PROFILE_MODEL").is_ok() {
            log::warn!(
                "[PROFILE] Batch stacking: {:.2} ms",
                stack_time.as_secs_f64() * 1000.0
            );
        }

        // Verify batch tensor shape
        let batch_shape = batch_tensor.size();
        assert_eq!(
            batch_shape[0], batch_size as i64,
            "Batch dimension mismatch"
        );
        assert_eq!(batch_shape[1], 3, "Channel dimension should be 3");
        assert_eq!(batch_shape[2], 640, "Height should be 640");
        assert_eq!(batch_shape[3], 640, "Width should be 640");

        // 3. Run single forward pass for entire batch
        let model = self
            .pytorch_rtdetr
            .as_ref()
            .ok_or("PyTorch model not loaded")?;

        let forward_start = Instant::now();
        let outputs = model
            .forward(&batch_tensor)
            .map_err(|e| format!("PyTorch batch forward pass failed: {}", e))?;
        let forward_time = forward_start.elapsed();
        if std::env::var("PROFILE_MODEL").is_ok() {
            log::warn!(
                "[PROFILE] Batch forward pass ({} images): {:.2} ms ({:.2} ms/image)",
                batch_size,
                forward_time.as_secs_f64() * 1000.0,
                forward_time.as_secs_f64() * 1000.0 / batch_size as f64
            );
        }

        // 4. Unpack results and post-process per image
        // outputs.logits: [N, num_queries, num_labels]
        // outputs.pred_boxes: [N, num_queries, 4]
        let postprocess_start = Instant::now();
        let mut all_clusters = Vec::with_capacity(batch_size);

        for (batch_idx, &(orig_w, orig_h)) in orig_dimensions.iter().enumerate() {
            // Extract this image's outputs
            let logits = outputs.logits.get(batch_idx as i64); // [num_queries, num_labels]
            let pred_boxes = outputs.pred_boxes.get(batch_idx as i64); // [num_queries, 4]

            // Get shapes
            let logits_size = logits.size();
            let logits_shape_vec = vec![1, logits_size[0] as usize, logits_size[1] as usize];

            // Convert tensors to Vec<f32>
            let logits_flat = logits.flatten(0, -1);
            let pred_boxes_flat = pred_boxes.flatten(0, -1);
            let logits_vec: Vec<f32> = Vec::try_from(&logits_flat)
                .map_err(|e| format!("Failed to convert logits tensor to Vec: {:?}", e))?;
            let pred_boxes_vec: Vec<f32> = Vec::try_from(&pred_boxes_flat)
                .map_err(|e| format!("Failed to convert pred_boxes tensor to Vec: {:?}", e))?;

            // Post-process this image
            let clusters = self.post_process(
                &logits_shape_vec,
                &logits_vec,
                &pred_boxes_vec,
                orig_w,
                orig_h,
            )?;

            all_clusters.push(clusters);
        }

        let postprocess_time = postprocess_start.elapsed();
        if std::env::var("PROFILE_MODEL").is_ok() {
            log::warn!(
                "[PROFILE] Batch post-processing ({} images): {:.2} ms ({:.2} ms/image)",
                batch_size,
                postprocess_time.as_secs_f64() * 1000.0,
                postprocess_time.as_secs_f64() * 1000.0 / batch_size as f64
            );
        }

        log::debug!("  [Batch] Completed: {} pages processed", batch_size);
        Ok(all_clusters)
    }

    fn post_process(
        &self,
        logits_shape: &[usize],
        logits_data: &[f32],
        pred_boxes_data: &[f32],
        orig_w: f32,
        orig_h: f32,
    ) -> Result<Vec<LayoutCluster>, Box<dyn std::error::Error>> {
        // Stage 2: HuggingFace-style postprocessing (threshold=0.3)
        // This produces ~98 raw clusters before label-specific filtering
        let stage2_clusters =
            self.stage2_hf_postprocess(logits_shape, logits_data, pred_boxes_data, orig_w, orig_h)?;

        // NOTE (N=189): stage3_layout_filter was removed from here.
        // Python applies label-specific filtering AFTER this stage in the pipeline.
        // The baseline "stage4_final_clusters.json" is actually Stage 2 output (98 clusters),
        // not Stage 3 filtered output (~31 clusters).
        // Label filtering is now applied in the layout_postprocessor where Python does it.

        Ok(stage2_clusters)
    }

    /// Stage 2: `HuggingFace` `post_process_object_detection`
    /// Produces ~98 raw clusters with threshold=0.3
    #[allow(clippy::unnecessary_wraps, reason = "Result kept for API consistency")]
    // Method signature kept for API consistency with other LayoutPredictor methods
    #[allow(clippy::unused_self)]
    // cx, cy, w, h, l, t, r, b are standard bbox coordinate names
    #[allow(clippy::many_single_char_names)]
    #[allow(clippy::too_many_lines)]
    fn stage2_hf_postprocess(
        &self,
        logits_shape: &[usize],
        logits_data: &[f32],
        pred_boxes_data: &[f32],
        orig_w: f32,
        orig_h: f32,
    ) -> Result<Vec<LayoutCluster>, Box<dyn std::error::Error>> {
        // logits shape: [batch, num_queries, num_classes]
        // pred_boxes shape: [batch, num_queries, 4] in format [cx, cy, w, h] normalized [0, 1]

        let num_queries = logits_shape[1];
        let num_classes = logits_shape[2];

        // RT-DETR class labels (0-indexed, from onnx_exports/layout_optimum/config.json)
        // Verified against HuggingFace model config id2label mapping
        // Labels match Python baseline JSON format (title case with hyphens)
        let class_labels = [
            "Caption",             // 0
            "Footnote",            // 1
            "Formula",             // 2
            "List-item",           // 3 (exception: lowercase 'i')
            "Page-Footer",         // 4 (capital F)
            "Page-Header",         // 5 (capital H)
            "Picture",             // 6
            "Section-Header",      // 7 (capital H)
            "Table",               // 8
            "Text",                // 9
            "Title",               // 10
            "Document Index",      // 11
            "Code",                // 12
            "Checkbox-Selected",   // 13
            "Checkbox-Unselected", // 14
            "Form",                // 15
            "Key-Value Region",    // 16
        ];

        // Stage 2 threshold (HuggingFace postprocessing)
        // This produces ~98 raw clusters before label-specific filtering
        // N=3613: Use DEBUG_LOW_THRESHOLD to test with lower threshold
        let threshold = if std::env::var("DEBUG_LOW_THRESHOLD").is_ok() {
            0.05 // Much lower threshold to see hidden detections
        } else {
            0.3 // Default threshold
        };

        // Step 1: Apply sigmoid to all logits (focal loss)
        let scores: Vec<f32> = logits_data
            .iter()
            .map(|&logit| {
                1.0 / (1.0 + (-logit).exp()) // sigmoid
            })
            .collect();

        // Step 2: Top-k selection (topk over flattened scores)
        // Python: scores, index = torch.topk(scores.flatten(1), num_top_queries, axis=-1)
        // CRITICAL: After topk, Python uses boxes.gather() to select only the boxes
        // corresponding to the selected query indices, ensuring each query appears at most once.
        let mut score_indices: Vec<(f32, usize)> = scores
            .iter()
            .enumerate()
            .map(|(i, &score)| (score, i))
            .collect();

        // Sort by score descending, with index as tie-breaker for stability
        // Python's torch.topk is stable - it preserves original order when scores are equal
        // Rust's sort_by is NOT stable by default, so we add index as secondary key
        // N=356: Round scores to 1e-5 precision to eliminate spurious ML precision differences
        // Scores within 0.00001 are considered equal and use index as tie-breaker
        score_indices.sort_by(|a, b| {
            let score_a = (a.0 * 100_000.0).round() / 100_000.0; // Round to 5 decimal places
            let score_b = (b.0 * 100_000.0).round() / 100_000.0;
            score_b
                .partial_cmp(&score_a)
                .unwrap()
                .then_with(|| a.1.cmp(&b.1)) // If scores equal (within 1e-5), prefer lower index
        });

        // Take top num_queries (300)
        score_indices.truncate(num_queries);

        // Step 2b: Extract query indices and gather boxes
        // Python: index = index // num_classes; boxes = boxes.gather(dim=1, index=index)
        // This ensures each query (bbox) appears at most once in the results
        let selected_data: Vec<(f32, usize, usize)> = score_indices
            .iter()
            .map(|(score, flat_idx)| {
                let query_idx = flat_idx / num_classes;
                let class_idx = flat_idx % num_classes;
                (*score, query_idx, class_idx)
            })
            .collect();

        // Step 3: Extract predictions and filter by threshold
        let mut clusters = Vec::new();
        for (score, query_idx, class_idx) in &selected_data {
            if *score <= threshold {
                continue; // Filter by threshold
            }

            // NOTE: Do NOT filter class 16 here! Python's Stage 3 (HF post-processing)
            // keeps all classes including "No object" (class 16). This is filtered later
            // in Stage 4 (LayoutPostprocessor label filtering).

            // Get bounding box for this query
            let bbox_idx = *query_idx * 4;
            let cx = pred_boxes_data[bbox_idx];
            let cy = pred_boxes_data[bbox_idx + 1];
            let w = pred_boxes_data[bbox_idx + 2];
            let h = pred_boxes_data[bbox_idx + 3];

            // Convert from [cx, cy, w, h] normalized to [l, t, r, b] absolute
            // HuggingFace: boxes = center_to_corners_format(out_bbox) * scale_fct
            // NOTE: Do NOT clamp coordinates! Python's HF post_process_object_detection
            // does not clamp, allowing slightly negative values (e.g., -0.388).
            // Clamping was causing 0.388 pixel diffs on jfk_scanned/page_1.
            let l = f64::from((cx - w / 2.0) * orig_w);
            let t = f64::from((cy - h / 2.0) * orig_h);
            let r = f64::from((cx + w / 2.0) * orig_w);
            let b = f64::from((cy + h / 2.0) * orig_h);

            // Get label string (use original HuggingFace model labels - no normalization)
            let label = if *class_idx < class_labels.len() {
                class_labels[*class_idx].to_string()
            } else {
                format!("unknown_{class_idx}")
            };

            clusters.push(LayoutCluster {
                id: clusters.len() as i32,
                label,
                confidence: f64::from(*score),
                bbox: BBox { l, t, r, b },
            });
        }

        // NOTE: NO deduplication! Python's implementation doesn't deduplicate.
        // The topk + gather logic ensures each query (bbox) appears at most once, but
        // different queries can have identical bbox coordinates with different labels.
        // This matches Python's HuggingFace post_process_object_detection exactly.

        // N=3613: Add debug logging to diagnose layout detection issues
        if std::env::var("DEBUG_LAYOUT").is_ok() {
            let mut label_counts: std::collections::HashMap<String, usize> =
                std::collections::HashMap::new();
            for cluster in &clusters {
                *label_counts.entry(cluster.label.clone()).or_insert(0) += 1;
            }
            eprintln!(
                "[DEBUG_LAYOUT] Layout postprocess: {} clusters above threshold {:.2}",
                clusters.len(),
                threshold
            );
            for (label, count) in &label_counts {
                eprintln!("[DEBUG_LAYOUT]   - {label}: {count} detections");
            }
            // Log top scores for each class to diagnose low confidence
            let mut max_scores: std::collections::HashMap<String, f32> =
                std::collections::HashMap::new();
            for (score, _, class_idx) in &selected_data {
                let label = if *class_idx < class_labels.len() {
                    class_labels[*class_idx].to_string()
                } else {
                    format!("unknown_{class_idx}")
                };
                let entry = max_scores.entry(label).or_insert(0.0);
                if *score > *entry {
                    *entry = *score;
                }
            }
            eprintln!("[DEBUG_LAYOUT] Max scores by class (top 5):");
            let mut sorted_scores: Vec<_> = max_scores.into_iter().collect();
            sorted_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            for (label, score) in sorted_scores.iter().take(5) {
                eprintln!("[DEBUG_LAYOUT]   - {label}: {score:.3}");
            }
        }

        Ok(clusters)
    }

    // NOTE (N=189-191): stage3_layout_filter function was removed from the inference pipeline.
    // Label filtering is now done in layout_postprocessor where Python does it.
    // This preserves the waterfall validation where Stage 2→3 passes raw clusters.
}

impl std::fmt::Debug for LayoutPredictorModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LayoutPredictorModel")
            .field("backend", &self.backend)
            .field("device", &self.device)
            .field("onnx_loaded", &self.onnx_session.is_some())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use log;

    #[test]
    fn test_model_load() {
        let model_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("onnx_exports/layout_predictor_rtdetr.onnx");

        if model_path.exists() {
            let model = LayoutPredictorModel::load(&model_path, Device::Cpu);
            assert!(model.is_ok(), "Failed to load model: {:?}", model.err());
        } else {
            log::debug!("Skipping test - ONNX model not found at {model_path:?}");
        }
    }

    #[test]
    fn test_inference_backend_from_str() {
        // ONNX always available
        assert_eq!(
            "onnx".parse::<InferenceBackend>().unwrap(),
            InferenceBackend::ONNX
        );
        assert_eq!(
            "ONNX".parse::<InferenceBackend>().unwrap(),
            InferenceBackend::ONNX
        );

        // PyTorch when feature enabled
        #[cfg(feature = "pytorch")]
        {
            assert_eq!(
                "pytorch".parse::<InferenceBackend>().unwrap(),
                InferenceBackend::PyTorch
            );
            assert_eq!(
                "torch".parse::<InferenceBackend>().unwrap(),
                InferenceBackend::PyTorch
            );
            assert_eq!(
                "libtorch".parse::<InferenceBackend>().unwrap(),
                InferenceBackend::PyTorch
            );
        }

        // PyTorch should error when feature disabled
        #[cfg(not(feature = "pytorch"))]
        {
            assert!("pytorch".parse::<InferenceBackend>().is_err());
        }

        // Invalid
        assert!("invalid".parse::<InferenceBackend>().is_err());
        assert!("".parse::<InferenceBackend>().is_err());
    }

    #[test]
    fn test_inference_backend_roundtrip() {
        // ONNX roundtrip
        let s = InferenceBackend::ONNX.to_string();
        let parsed: InferenceBackend = s.parse().unwrap();
        assert_eq!(parsed, InferenceBackend::ONNX);

        // PyTorch roundtrip (when available)
        #[cfg(feature = "pytorch")]
        {
            let s = InferenceBackend::PyTorch.to_string();
            let parsed: InferenceBackend = s.parse().unwrap();
            assert_eq!(parsed, InferenceBackend::PyTorch);
        }
    }
}
