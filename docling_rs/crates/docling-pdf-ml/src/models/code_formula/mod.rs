//! # Code/Formula Enrichment Model - Idefics3 Vision-Language Model
//!
//! This module implements the Idefics3 vision-language model for enriching code blocks
//! and mathematical formulas with natural language descriptions or LaTeX representations.
//!
//! ## Architecture
//!
//! Idefics3 is a **multimodal Transformer** combining vision and language understanding:
//! - **Vision Encoder:** SiglipVisionTransformer (12 layers, 768 hidden dim)
//! - **Vision-Language Connector:** Gated cross-attention mechanism
//! - **Text Decoder:** VLlama3ForCausalLM (30 layers, 576 hidden dim)
//! - **Generation:** Autoregressive decoding with KV cache
//!
//! ## Use Cases
//!
//! ### Code Block Enrichment
//! - Input: Code region image from PDF
//! - Output: Natural language description of code functionality
//! - Example: "Python function for computing Fibonacci numbers"
//!
//! ### Formula Enrichment
//! - Input: Mathematical formula image from PDF
//! - Output: LaTeX representation or description
//! - Example: "\int_0^\infty e^{-x^2} dx = \frac{\sqrt{\pi}}{2}"
//!
//! ## Backend
//!
//! **PyTorch (tch-rs) only** - ONNX export not supported because:
//! - Autoregressive generation requires dynamic control flow
//! - KV cache management cannot be expressed in static ONNX graph
//! - Vision-language cross-attention is complex and dynamic
//!
//! ## Usage Example
//!
//! ```no_run
//! use docling_pdf_ml::models::code_formula::Idefics3Model;
//! use image::DynamicImage;
//!
//! // Load model (large, ~2GB weights)
//! let model = Idefics3Model::new("path/to/weights", "cpu")?;
//!
//! // Prepare code/formula region image
//! let code_image: DynamicImage = extract_code_region(&page_image, &code_bbox)?;
//!
//! // Generate description
//! let description = model.generate(&code_image, "Describe this code:")?;
//! println!("Code description: {}", description);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! # fn extract_code_region(img: &DynamicImage, bbox: &(f32,f32,f32,f32)) -> Result<DynamicImage, Box<dyn std::error::Error>> { unimplemented!() }
//! ```
//!
//! ## Performance
//!
//! CodeFormula enrichment is **computationally expensive**:
//! - **Latency:** ~500-2000 ms per region (depends on generation length)
//! - **Memory:** ~2GB model weights + ~500MB runtime memory
//! - **Device support:** CPU, CUDA, MPS (Apple Silicon)
//! - **Recommendation:** Enable only when code/formula descriptions are required
//!
//! ## Configuration
//!
//! The model is **disabled by default** in the pipeline. To enable:
//!
//! ```no_run
//! use docling_pdf_ml::PipelineConfigBuilder;
//!
//! let config = PipelineConfigBuilder::default()
//!     .enable_code_enrichment(true)
//!     .enable_formula_enrichment(true)
//!     .build()?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Model Details
//!
//! Key hyperparameters:
//! - **Vision encoder:** 12 Transformer layers, 768 hidden dim
//! - **Text decoder:** 30 Transformer layers, 576 hidden dim
//! - **Attention heads:** 12 (vision), 9 (text)
//! - **Vocabulary size:** 151,646 tokens
//! - **Max sequence length:** 8,192 tokens
//! - **Image size:** 364×364 pixels (vision encoder input)
//!
//! ## Model Source
//!
//! The model is downloaded from HuggingFace:
//! - **Repository:** `HuggingFaceM4/Idefics3-8B-Llama3` (or similar variant)
//! - **Architecture:** `Idefics3ForConditionalGeneration`
//! - **Framework:** PyTorch (converted to tch-rs format)
//!
//! ## References
//!
//! - **Python implementation:** `docling/models/code_formula_model.py`
//! - **HuggingFace transformers:** `transformers/models/idefics3/`
//! - **Paper:** Idefics3 technical report (HuggingFace)

pub mod config;
pub mod connector;
pub mod preprocessor;
pub mod text_decoder;
pub mod tokenizer;
pub mod vision;

use config::Idefics3Config;
use connector::Idefics3Connector;
use text_decoder::TextDecoder;
use vision::VisionTransformer;

use std::path::Path;
use tch::{nn, nn::Module, Tensor};

/// Idefics3 vision-language model for code and formula enrichment
///
/// Architecture:
/// - Vision encoder: SiglipVisionTransformer (12 layers, 768 hidden)
/// - Connector: Pixel shuffle + MLP projection (768*16 → 576)
/// - Text decoder: Llama-based (30 layers, 576 hidden)
///
/// Forward pass:
/// 1. pixel_values → vision_model → vision_embeddings [batch, 1024, 768]
/// 2. vision_embeddings → connector → projected_embeddings [batch, 64, 576]
/// 3. input_ids → text_embeddings [batch, seq_len, 576]
/// 4. Insert projected_embeddings at `<image>` token positions
/// 5. merged_embeddings → text_decoder → hidden_states
/// 6. hidden_states → lm_head → logits [batch, seq_len, vocab_size]
pub struct Idefics3Model {
    config: Idefics3Config,
    vision_model: VisionTransformer,
    connector: Idefics3Connector,
    text_decoder: TextDecoder,
    lm_head: nn::Linear,
    var_store: nn::VarStore,
}

impl std::fmt::Debug for Idefics3Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Idefics3Model")
            .field("config", &self.config)
            .field("vision_model", &self.vision_model)
            .field("connector", &self.connector)
            .field("text_decoder", &self.text_decoder)
            .field("lm_head", &"<Linear>")
            .field("var_store", &"<VarStore>")
            .finish()
    }
}

impl Idefics3Model {
    /// Create a new Idefics3 model with given device
    ///
    /// This creates the model structure but does NOT load weights.
    /// Use load_weights() after creation to load from SafeTensors.
    pub fn new(config: &Idefics3Config, device: tch::Device) -> Self {
        let vs = nn::VarStore::new(device);
        let root = vs.root();

        // SafeTensors uses "model." prefix for all weights
        let model_root = &root / "model";

        let vision_model =
            VisionTransformer::new(&(&model_root / "vision_model"), &config.vision_config);
        let connector = Idefics3Connector::new(&(&model_root / "connector"), config);
        let text_decoder = TextDecoder::new(&(&model_root / "text_model"), &config.text_config);

        // LM head is at root level (not under model.)
        // No bias (SafeTensors only has lm_head.weight, not lm_head.bias)
        let hidden_size = config.text_config.hidden_size as i64;
        let vocab_size = config.text_config.vocab_size as i64;
        let lm_head_config = nn::LinearConfig {
            bias: false,
            ..Default::default()
        };
        let lm_head = nn::linear(&root / "lm_head", hidden_size, vocab_size, lm_head_config);

        Self {
            config: config.clone(),
            vision_model,
            connector,
            text_decoder,
            lm_head,
            var_store: vs,
        }
    }

    /// Get vision embeddings (vision encoder only, before connector)
    ///
    /// Input: pixel_values [batch, num_channels, height, width]
    ///        OR [batch, num_images, num_channels, height, width] (Patch n' Pack format)
    /// Output: vision embeddings [batch, num_patches, vision_hidden]
    ///
    /// Example:
    /// - Input: [1, 3, 512, 512] (single RGB image)
    /// - Output: [1, 1024, 768] (32x32 patches, 768 dim)
    ///
    /// Example with multiple images (Patch n' Pack):
    /// - Input: [1, 9, 3, 512, 512] (9 image patches)
    /// - Reshape to: [9, 3, 512, 512]
    /// - Output: [9, 1024, 768]
    pub fn get_vision_embeddings(
        &self,
        pixel_values: &Tensor,
    ) -> Result<Tensor, Box<dyn std::error::Error>> {
        let ndim = pixel_values.dim();

        // Handle both [B, C, H, W] and [B, N, C, H, W] formats
        let vision_embeddings = if ndim == 5 {
            // Patch n' Pack format: [B, N, C, H, W]
            let size = pixel_values.size();
            let batch_size = size[0];
            let num_images = size[1];
            let channels = size[2];
            let height = size[3];
            let width = size[4];

            // Reshape to [B*N, C, H, W] for batch processing
            let pixel_values_flat =
                pixel_values.view([batch_size * num_images, channels, height, width]);

            // Vision encoder forward
            self.vision_model.forward(&pixel_values_flat, false)?
        } else if ndim == 4 {
            // Standard format: [B, C, H, W]
            self.vision_model.forward(pixel_values, false)?
        } else {
            return Err(format!("Invalid pixel_values shape: expected 4D [B, C, H, W] or 5D [B, N, C, H, W], got {}D", ndim).into());
        };

        Ok(vision_embeddings)
    }

    /// Get image features (vision encoder + connector)
    ///
    /// Input: pixel_values [batch, num_channels, height, width]
    ///        OR [batch, num_images, num_channels, height, width] (Patch n' Pack format)
    /// Output: projected vision embeddings [batch, num_patches_reduced, text_hidden]
    ///
    /// Example:
    /// - Input: [1, 3, 512, 512] (single RGB image)
    /// - Vision output: [1, 1024, 768] (32x32 patches, 768 dim)
    /// - Connector output: [1, 64, 576] (8x8 patches after pixel shuffle, 576 text dim)
    ///
    /// Example with multiple images (Patch n' Pack):
    /// - Input: [1, 9, 3, 512, 512] (9 image patches)
    /// - Reshape to: [9, 3, 512, 512]
    /// - Vision output: [9, 1024, 768]
    /// - Connector output: [9, 64, 576]
    /// - Flatten patches: [1, 576, 576] (9*64 = 576 patches)
    pub fn get_image_features(
        &self,
        pixel_values: &Tensor,
    ) -> Result<Tensor, Box<dyn std::error::Error>> {
        let ndim = pixel_values.dim();

        // Handle both [B, C, H, W] and [B, N, C, H, W] formats
        let (vision_embeddings, num_images) = if ndim == 5 {
            // Patch n' Pack format: [B, N, C, H, W]
            let size = pixel_values.size();
            let batch_size = size[0];
            let num_images = size[1];
            let channels = size[2];
            let height = size[3];
            let width = size[4];

            // Reshape to [B*N, C, H, W] for batch processing
            let pixel_values_flat =
                pixel_values.view([batch_size * num_images, channels, height, width]);

            // Vision encoder forward
            let vision_embeddings = self.vision_model.forward(&pixel_values_flat, false)?;

            (vision_embeddings, num_images)
        } else if ndim == 4 {
            // Standard format: [B, C, H, W]
            let vision_embeddings = self.vision_model.forward(pixel_values, false)?;
            (vision_embeddings, 1)
        } else {
            return Err(format!("Invalid pixel_values shape: expected 4D [B, C, H, W] or 5D [B, N, C, H, W], got {}D", ndim).into());
        };

        // Connector forward (pixel shuffle + projection)
        // vision_embeddings: [B*N, num_patches, vision_hidden] or [B, num_patches, vision_hidden]
        let projected_embeddings = self.connector.forward(&vision_embeddings);

        // Python implementation does NOT reshape back - it returns [B*N, P, H] directly
        // where B*N is the number of real (non-padding) images after filtering
        // See: transformers/models/idefics3/modeling_idefics3.py:727
        //
        // projected_embeddings: [B*N, reduced_patches, text_hidden]
        // For example: [9, 64, 576] for 9 image patches
        Ok(projected_embeddings)
    }

    /// Forward pass: input_ids → logits
    ///
    /// Input: input_ids [batch, seq_len]
    /// Output: logits [batch, seq_len, vocab_size]
    ///
    /// This is the complete text generation forward pass:
    /// 1. input_ids → text_decoder → hidden_states [batch, seq_len, hidden_size]
    /// 2. hidden_states → lm_head → logits [batch, seq_len, vocab_size]
    pub fn forward(
        &self,
        input_ids: &Tensor,
        attention_mask: Option<&Tensor>,
        train: bool,
    ) -> Result<Tensor, Box<dyn std::error::Error>> {
        // Text decoder: input_ids → hidden_states
        let hidden_states = self
            .text_decoder
            .forward(input_ids, attention_mask, train)?;

        // LM head: hidden_states → logits
        let logits = hidden_states.apply(&self.lm_head);

        Ok(logits)
    }

    /// Get reference to VarStore (for debugging/weight inspection)
    #[inline]
    #[must_use = "returns the VarStore reference for weight inspection"]
    pub fn var_store(&self) -> &nn::VarStore {
        &self.var_store
    }

    /// Forward pass: embeddings → logits
    ///
    /// Input: embeddings [batch, seq_len, hidden_size]
    /// Output: logits [batch, seq_len, vocab_size]
    ///
    /// This is useful for vision-language models where image features are merged with text embeddings.
    pub fn forward_with_embeddings(
        &self,
        embeddings: &Tensor,
        attention_mask: Option<&Tensor>,
        train: bool,
    ) -> Result<Tensor, Box<dyn std::error::Error>> {
        // Text decoder: embeddings → hidden_states
        let hidden_states =
            self.text_decoder
                .forward_with_embeddings(embeddings, attention_mask, train)?;

        // LM head: hidden_states → logits
        let logits = hidden_states.apply(&self.lm_head);

        Ok(logits)
    }

    /// Get hidden states before LM head (for debugging)
    ///
    /// Input: embeddings [batch, seq_len, hidden_size]
    /// Output: hidden_states [batch, seq_len, hidden_size]
    ///
    /// This returns the decoder output BEFORE the LM head projection.
    /// Useful for comparing intermediate activations with Python baseline.
    pub fn get_hidden_states(
        &self,
        embeddings: &Tensor,
        attention_mask: Option<&Tensor>,
        train: bool,
    ) -> Result<Tensor, Box<dyn std::error::Error>> {
        self.text_decoder
            .forward_with_embeddings(embeddings, attention_mask, train)
    }

    /// Get text embeddings from input IDs (for debugging)
    ///
    /// Input: input_ids [batch, seq_len]
    /// Output: embeddings [batch, seq_len, hidden_size]
    pub fn get_text_embeddings(&self, input_ids: &Tensor) -> Tensor {
        self.text_decoder.embed_tokens.forward(input_ids)
    }

    /// Get layer-by-layer hidden states for debugging
    ///
    /// Returns hidden states after each of the 30 decoder layers + final norm.
    /// Useful for comparing Rust vs Python layer-by-layer outputs.
    ///
    /// Returns: Vec of (layer_idx, hidden_states) where layer_idx=99 is final norm
    pub fn get_layer_outputs(
        &self,
        embeddings: &Tensor,
        attention_mask: Option<&Tensor>,
        train: bool,
    ) -> Result<Vec<(usize, Tensor)>, Box<dyn std::error::Error>> {
        self.text_decoder
            .forward_with_layer_outputs(embeddings, attention_mask, train)
    }

    /// Access text decoder for debugging
    #[inline]
    #[must_use = "returns the text decoder reference for debugging"]
    pub fn text_decoder(&self) -> &TextDecoder {
        &self.text_decoder
    }

    /// Load model weights from SafeTensors file
    ///
    /// This should be called after model creation to load pretrained weights.
    pub fn load_weights<P: AsRef<Path>>(
        &mut self,
        model_path: P,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use safetensors::SafeTensors;
        use std::fs;

        log::debug!(
            "Loading weights from SafeTensors: {:?}",
            model_path.as_ref()
        );

        // Read SafeTensors file
        let buffer = fs::read(model_path.as_ref())?;
        let tensors = SafeTensors::deserialize(&buffer)?;

        log::debug!(
            "SafeTensors file contains {} tensors",
            tensors.names().len()
        );

        // Copy tensors into VarStore variables
        let mut loaded_count = 0;
        let mut missing_count = 0;
        let mut missing_names: Vec<String> = Vec::new();

        for (vs_name, mut vs_tensor) in self.var_store.variables() {
            // Try to find tensor in SafeTensors
            if let Ok(tensor_view) = tensors.tensor(&vs_name) {
                let shape: Vec<i64> = tensor_view.shape().iter().map(|&x| x as i64).collect();

                // Verify shape matches
                let vs_shape = vs_tensor.size();
                if vs_shape != shape.as_slice() {
                    log::debug!(
                        "  WARNING: Shape mismatch for '{}': VarStore {:?} vs SafeTensors {:?}",
                        vs_name,
                        vs_shape,
                        shape
                    );
                    missing_count += 1;
                    missing_names.push(vs_name.clone());
                    continue;
                }

                // Load tensor data
                let data = tensor_view.data();
                let loaded_tensor = match tensor_view.dtype() {
                    safetensors::Dtype::F32 => {
                        let float_data: &[f32] = bytemuck::cast_slice(data);
                        tch::Tensor::from_slice(float_data).reshape(&shape)
                    }
                    safetensors::Dtype::F16 => {
                        // F16 not directly supported, convert to F32
                        let f16_data: &[half::f16] = bytemuck::cast_slice(data);
                        let f32_data: Vec<f32> = f16_data.iter().map(|&x| x.to_f32()).collect();
                        tch::Tensor::from_slice(&f32_data).reshape(&shape)
                    }
                    safetensors::Dtype::BF16 => {
                        // BF16 (BFloat16) - convert to F32
                        let bf16_data: &[half::bf16] = bytemuck::cast_slice(data);
                        let f32_data: Vec<f32> = bf16_data.iter().map(|&x| x.to_f32()).collect();
                        tch::Tensor::from_slice(&f32_data).reshape(&shape)
                    }
                    safetensors::Dtype::I64 => {
                        let int_data: &[i64] = bytemuck::cast_slice(data);
                        tch::Tensor::from_slice(int_data).reshape(&shape)
                    }
                    _ => {
                        log::debug!(
                            "  WARNING: Unsupported dtype for '{}': {:?}",
                            vs_name,
                            tensor_view.dtype()
                        );
                        missing_count += 1;
                        missing_names.push(vs_name.clone());
                        continue;
                    }
                };

                // Copy into VarStore (using no_grad to avoid autograd)
                tch::no_grad(|| {
                    vs_tensor.copy_(&loaded_tensor);
                });

                loaded_count += 1;
            } else {
                missing_count += 1;
                missing_names.push(vs_name.clone());
            }
        }

        log::debug!("✓ Copied {} tensors into VarStore", loaded_count);
        if missing_count > 0 {
            log::debug!(
                "  WARNING: {} VarStore variables not found in SafeTensors file",
                missing_count
            );
            log::debug!("  First 10 missing:");
            for name in missing_names.iter().take(10) {
                log::debug!("    - {}", name);
            }
        }

        Ok(())
    }

    /// Set model to evaluation mode (disable dropout, etc.)
    pub fn eval(&mut self) {
        self.var_store.freeze();
    }

    /// Load model from HuggingFace model name or local path
    ///
    /// Resolves model name to HuggingFace cache directory and loads config + weights.
    /// Example: "ds4sd/CodeFormulaV2" → ~/.cache/huggingface/hub/models--ds4sd--CodeFormulaV2/
    pub fn from_pretrained(
        model_name_or_path: &str,
        device: tch::Device,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Check if it's a local path
        let model_dir = if std::path::Path::new(model_name_or_path).exists() {
            std::path::PathBuf::from(model_name_or_path)
        } else {
            // Resolve HuggingFace model name to cache directory
            // Example: ds4sd/CodeFormulaV2 → models--ds4sd--CodeFormulaV2
            let model_slug = model_name_or_path.replace('/', "--");
            let cache_dir = dirs::cache_dir()
                .ok_or("Failed to get cache directory")?
                .join("huggingface")
                .join("hub")
                .join(format!("models--{}", model_slug));

            log::debug!("Resolved cache dir: {:?}", cache_dir);

            if !cache_dir.exists() {
                return Err(format!(
                    "Model not found in cache: {}\nExpected: {:?}\nPlease download model first",
                    model_name_or_path, cache_dir
                )
                .into());
            }

            // HuggingFace cache uses snapshots directory
            // Find the most recent snapshot (usually there's only one)
            let snapshots_dir = cache_dir.join("snapshots");
            log::debug!("Snapshots dir: {:?}", snapshots_dir);

            if !snapshots_dir.exists() {
                return Err(format!(
                    "Invalid model cache structure: snapshots directory not found in {:?}",
                    cache_dir
                )
                .into());
            }

            // Get the first (and usually only) snapshot
            let snapshot = std::fs::read_dir(&snapshots_dir)?
                .next()
                .ok_or("No snapshot found in model cache")??
                .path();

            log::debug!("Using snapshot: {:?}", snapshot);
            snapshot
        };

        // Load configuration
        log::debug!("Loading config from: {:?}", model_dir);
        let config = config::Idefics3Config::from_pretrained(&model_dir)?;

        // Create model
        let mut model = Idefics3Model::new(&config, device);

        // Load weights
        let model_path = model_dir.join("model.safetensors");
        if model_path.exists() {
            model.load_weights(&model_path)?;
            model.eval();
        } else {
            return Err(format!("model.safetensors not found in {:?}", model_dir).into());
        }

        Ok(model)
    }
}

/// CodeFormula model for enriching code and formula regions
///
/// This is a high-level wrapper around Idefics3Model that provides
/// a simple API for enriching code and formula regions.
pub struct CodeFormulaModel {
    model: Idefics3Model,
    config: Idefics3Config,
    tokenizer: tokenizer::Idefics3Tokenizer,
    preprocessor: preprocessor::Idefics3Preprocessor,
}

impl std::fmt::Debug for CodeFormulaModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CodeFormulaModel")
            .field("model", &self.model)
            .field("config", &self.config)
            .field("tokenizer", &"<Idefics3Tokenizer>")
            .field("preprocessor", &"<Idefics3Preprocessor>")
            .finish()
    }
}

impl CodeFormulaModel {
    /// Load model from HuggingFace directory
    pub fn from_pretrained<P: AsRef<Path>>(
        model_dir: P,
        device: tch::Device,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Load configuration
        let config = Idefics3Config::from_pretrained(&model_dir)?;

        log::debug!("Loaded Idefics3 config:");
        log::debug!("  Model type: {}", config.model_type);
        log::debug!("  Architecture: {:?}", config.architectures);
        log::debug!(
            "  Text: {} layers, {} hidden, {} vocab",
            config.num_text_layers(),
            config.text_hidden_size(),
            config.vocab_size()
        );
        log::debug!(
            "  Vision: {} layers, {} hidden, {}x{} patches",
            config.num_vision_layers(),
            config.vision_hidden_size(),
            config.patch_size(),
            config.patch_size()
        );

        // Create model
        let mut model = Idefics3Model::new(&config, device);

        // Load weights
        let model_path = model_dir.as_ref().join("model.safetensors");
        if model_path.exists() {
            log::debug!("Loading weights from: {:?}", model_path);
            model.load_weights(&model_path)?;
            model.eval();
        } else {
            log::debug!("Warning: model.safetensors not found, using random weights");
        }

        // Load tokenizer and preprocessor
        let tokenizer = tokenizer::Idefics3Tokenizer::from_pretrained(&model_dir)?;
        let preprocessor = preprocessor::Idefics3Preprocessor::new();

        Ok(Self {
            model,
            config,
            tokenizer,
            preprocessor,
        })
    }

    /// Get reference to underlying Idefics3Model (for debugging/weight inspection)
    #[inline]
    #[must_use = "returns the underlying Idefics3Model reference"]
    pub fn inner_model(&self) -> &Idefics3Model {
        &self.model
    }

    /// Forward pass with preprocessed inputs (input_ids + pixel_values)
    ///
    /// Input: input_ids [batch, seq_len], pixel_values [batch, channels, height, width]
    /// Output: logits [batch, seq_len, vocab_size]
    ///
    /// This method processes vision features and merges them with text embeddings,
    /// then runs the full forward pass to get logits. Useful for debugging generation.
    pub fn forward_with_preprocessed(
        &self,
        input_ids: &Tensor,              // [batch, seq_len]
        pixel_values: &Tensor, // [batch, channels, height, width] or [batch, num_images, channels, height, width]
        attention_mask: Option<&Tensor>, // Optional [batch, seq_len]
    ) -> Result<Tensor, Box<dyn std::error::Error>> {
        // Get image features
        let image_features = self.model.get_image_features(pixel_values)?;

        // Embed prompt tokens
        let embeddings = self.model.text_decoder.embed_tokens.forward(input_ids);

        // Find <image> token positions and merge image features
        let image_token_id = self.tokenizer.image_token_id() as i64;
        let input_ids_vec: Vec<i64> = input_ids.flatten(0, -1).try_into()?;
        let image_positions: Vec<usize> = input_ids_vec
            .iter()
            .enumerate()
            .filter_map(|(i, &id)| (id == image_token_id).then_some(i))
            .collect();

        let merged_embeddings = if !image_positions.is_empty() {
            // Need to clone to allow in-place modification (copy_ method)
            let embeddings_mut = embeddings.shallow_clone();
            // Merge image features at <image> token positions
            let size = image_features.size();
            let num_images = size[0] as usize;
            let patches_per_image = size[1] as usize;
            let total_patches = num_images * patches_per_image;

            if image_positions.len() != total_patches {
                return Err(format!(
                    "Mismatch: {} <image> tokens but {} image feature patches",
                    image_positions.len(),
                    total_patches
                )
                .into());
            }

            // Flatten image_features: [num_images, patches_per_image, hidden_size] → [total_patches, hidden_size]
            let image_features_flat = image_features.view([total_patches as i64, size[2]]);

            // Replace <image> tokens with image features
            for (patch_idx, &token_pos) in image_positions.iter().enumerate() {
                let patch = image_features_flat
                    .narrow(0, patch_idx as i64, 1)
                    .squeeze_dim(0);
                embeddings_mut
                    .narrow(1, token_pos as i64, 1)
                    .squeeze_dim(1)
                    .copy_(&patch);
            }

            embeddings_mut
        } else {
            embeddings
        };

        // Forward pass with merged embeddings
        self.model
            .forward_with_embeddings(&merged_embeddings, attention_mask, false)
    }

    /// Generate tokens from preprocessed inputs (for testing/validation)
    ///
    /// This method accepts preprocessed inputs directly and returns generated token IDs.
    /// Useful for Phase 1 validation where inputs are already preprocessed.
    pub fn generate_from_preprocessed(
        &self,
        input_ids: &Tensor,              // [batch, seq_len]
        pixel_values: &Tensor, // [batch, channels, height, width] or [batch, num_images, channels, height, width]
        attention_mask: Option<&Tensor>, // Optional [batch, seq_len]
        max_new_tokens: usize,
    ) -> Result<Tensor, Box<dyn std::error::Error>> {
        // Get image features
        let image_features = self.model.get_image_features(pixel_values)?;

        // Convert input_ids tensor to Vec<u32> for tokenizer compatibility
        let input_ids_vec: Vec<i64> = input_ids.flatten(0, -1).try_into()?;
        let prompt_tokens: Vec<u32> = input_ids_vec.iter().map(|&x| x as u32).collect();

        // Generate using existing generate method
        let generated_tokens = self.generate(&prompt_tokens, &image_features, max_new_tokens)?;

        // Skip prompt tokens (only return newly generated tokens, matching Python behavior)
        // Python does: generated_ids[:, inputs.input_ids.shape[1]:]
        let prompt_len = prompt_tokens.len();
        let new_tokens: Vec<u32> = generated_tokens.iter().skip(prompt_len).copied().collect();

        // Convert back to tensor for consistency
        let generated_ids: Vec<i64> = new_tokens.iter().map(|&x| x as i64).collect();
        let result = Tensor::from_slice(&generated_ids)
            .view([1, generated_ids.len() as i64])
            .to(self.model.var_store.device());

        Ok(result)
    }

    /// Decode token IDs to text
    #[must_use = "token decoding returns a result that should be processed"]
    pub fn decode_tokens(&self, token_ids: &Tensor) -> Result<String, Box<dyn std::error::Error>> {
        // Extract token IDs from tensor
        let ids_vec: Vec<i64> = token_ids.flatten(0, -1).try_into()?;
        let ids_u32: Vec<u32> = ids_vec.iter().map(|&x| x as u32).collect();

        // Decode using tokenizer
        self.tokenizer.decode(&ids_u32, false)
    }

    /// Post-process generated text (remove special tokens, extract language)
    pub fn post_process(&self, text: &str) -> EnrichmentResult {
        let (cleaned_text, language) = self.tokenizer.post_process(text);
        EnrichmentResult {
            text: cleaned_text,
            language,
        }
    }

    /// Enrich a code or formula region
    ///
    /// Input: pixel_values [1, 3, H, W] (preprocessed image)
    /// Output: EnrichmentResult with text and optional language
    pub fn enrich(
        &self,
        pixel_values: &Tensor, // Preprocessed image tensor [1, 3, H, W]
        label: &str,           // "code" or "formula"
    ) -> Result<EnrichmentResult, Box<dyn std::error::Error>> {
        // Step 1: Generate prompt based on label
        let prompt = self.tokenizer.apply_chat_template(label, true)?;

        // Step 2: Encode prompt to token IDs
        let mut prompt_tokens = self.tokenizer.encode(&prompt, false)?;

        // Step 3: Get image features (vision encoder + connector)
        let image_features = self.model.get_image_features(pixel_values)?;

        // Step 3b: Expand <image> token to match number of patches
        // The connector reduces vision patches from 1024 → 64 (scale_factor=4)
        // We need to replace the single <image> token with 64 <image> tokens
        let image_token_id = self.tokenizer.image_token_id();
        let patches_per_image = image_features.size()[1] as usize; // 64 patches

        // Find <image> token position and expand it
        if let Some(image_pos) = prompt_tokens.iter().position(|&t| t == image_token_id) {
            // Remove the single <image> token
            prompt_tokens.remove(image_pos);
            // Insert 64 <image> tokens at the same position
            let expanded_tokens = vec![image_token_id; patches_per_image];
            prompt_tokens.splice(image_pos..image_pos, expanded_tokens);
        } else {
            return Err("Prompt must contain <image> token".into());
        }

        // Step 4: Generate tokens autoregressively
        let max_new_tokens = 512; // Match Python's max_new_tokens
        let generated_tokens = self.generate(&prompt_tokens, &image_features, max_new_tokens)?;

        // Step 5: Decode generated tokens (skip prompt tokens)
        let new_tokens: Vec<u32> = generated_tokens
            .iter()
            .skip(prompt_tokens.len())
            .copied()
            .collect();
        let generated_text = self.tokenizer.decode(&new_tokens, false)?;

        // Step 6: Post-process (remove special tokens, extract language)
        let (cleaned_text, language) = self.tokenizer.post_process(&generated_text);

        Ok(EnrichmentResult {
            text: cleaned_text,
            language,
        })
    }

    /// Process a batch of regions for enrichment
    ///
    /// Uses batched vision encoding for better performance:
    /// 1. Batch preprocessing: All images → [N, 3, 512, 512] tensor
    /// 2. Batch vision encoding: One forward pass → [N, 64, 576] features
    /// 3. Sequential generation: Loop over features, generate text for each
    ///
    /// # Arguments
    /// * `images` - Cropped region images (image::DynamicImage)
    /// * `labels` - Region labels ("code" or "formula")
    ///
    /// # Returns
    /// * Vec of enrichment results (text + optional language)
    pub fn process_batch(
        &self,
        images: &[image::DynamicImage],
        labels: &[&str],
    ) -> Result<Vec<EnrichmentResult>, Box<dyn std::error::Error>> {
        if images.len() != labels.len() {
            return Err(format!(
                "Mismatch: {} images but {} labels",
                images.len(),
                labels.len()
            )
            .into());
        }

        if images.is_empty() {
            return Ok(Vec::new());
        }

        // Step 1: Batch preprocessing (all images at once)
        // DynamicImage[] → Tensor [N, 3, 512, 512]
        let pixel_values_batch = self.preprocessor.preprocess_batch(images, None)?;

        // Step 2: Batch vision encoding (single forward pass for all images)
        // [N, 3, 512, 512] → [N, 64, 576] image features
        let image_features_batch = self.model.get_image_features(&pixel_values_batch)?;

        // Verify batch size matches
        let batch_size = image_features_batch.size()[0] as usize;
        if batch_size != images.len() {
            return Err(format!(
                "Batch size mismatch: expected {}, got {}",
                images.len(),
                batch_size
            )
            .into());
        }

        // Step 3: Sequential generation (loop over batched features)
        // For each region: image_features [64, 576] → generate text
        let mut results = Vec::with_capacity(images.len());
        for (i, &label) in labels.iter().enumerate() {
            // Extract features for this region: [64, 576]
            let image_features = image_features_batch.select(0, i as i64);

            // Add batch dimension: [64, 576] → [1, 64, 576]
            let image_features = image_features.unsqueeze(0);

            // Generate text for this region
            let result = self.enrich_with_features(&image_features, label)?;
            results.push(result);
        }

        Ok(results)
    }

    /// Enrich a code/formula region using pre-computed image features
    ///
    /// This is a helper method for batch processing. Instead of computing
    /// vision features from pixel_values, it uses pre-computed features.
    ///
    /// # Arguments
    /// * `image_features` - Pre-computed vision features [1, 64, 576]
    /// * `label` - Region label ("code" or "formula")
    ///
    /// # Returns
    /// * EnrichmentResult with text and optional language
    fn enrich_with_features(
        &self,
        image_features: &Tensor,
        label: &str,
    ) -> Result<EnrichmentResult, Box<dyn std::error::Error>> {
        // Step 1: Generate prompt based on label
        let prompt = self.tokenizer.apply_chat_template(label, true)?;

        // Step 2: Encode prompt to token IDs
        let mut prompt_tokens = self.tokenizer.encode(&prompt, false)?;

        // Step 3: Expand <image> token to match number of patches
        // The connector reduces vision patches from 1024 → 64 (scale_factor=4)
        // We need to replace the single <image> token with 64 <image> tokens
        let image_token_id = self.tokenizer.image_token_id();
        let patches_per_image = image_features.size()[1] as usize; // 64 patches

        // Find <image> token position and expand it
        if let Some(image_pos) = prompt_tokens.iter().position(|&t| t == image_token_id) {
            // Remove the single <image> token
            prompt_tokens.remove(image_pos);
            // Insert 64 <image> tokens at the same position
            let expanded_tokens = vec![image_token_id; patches_per_image];
            prompt_tokens.splice(image_pos..image_pos, expanded_tokens);
        } else {
            return Err("Prompt must contain <image> token".into());
        }

        // Step 4: Generate tokens autoregressively
        let max_new_tokens = 512; // Match Python's max_new_tokens
        let generated_tokens = self.generate(&prompt_tokens, image_features, max_new_tokens)?;

        // Step 5: Decode generated tokens (skip prompt tokens)
        let new_tokens: Vec<u32> = generated_tokens
            .iter()
            .skip(prompt_tokens.len())
            .copied()
            .collect();
        let generated_text = self.tokenizer.decode(&new_tokens, false)?;

        // Step 6: Post-process (remove special tokens, extract language)
        let (cleaned_text, language) = self.tokenizer.post_process(&generated_text);

        Ok(EnrichmentResult {
            text: cleaned_text,
            language,
        })
    }

    /// Create causal attention mask for autoregressive generation
    ///
    /// Returns a mask of shape [1, 1, seq_len, seq_len] where:
    /// - mask[i, j] = 0 if i >= j (can attend)
    /// - mask[i, j] = -inf if i < j (cannot attend to future)
    ///
    /// This ensures each position can only attend to itself and previous positions.
    fn create_causal_mask(seq_len: i64, device: tch::Device) -> Tensor {
        // Create a lower triangular matrix of 1s (including diagonal)
        let ones = Tensor::ones([seq_len, seq_len], (tch::Kind::Float, device));
        let lower_tri = ones.tril(0); // Lower triangular: 1 where i >= j, 0 where i < j

        // Create upper triangular (excluding diagonal) by inverting
        // upper_tri: 0 where i >= j, 1 where i < j
        let upper_tri = Tensor::ones([seq_len, seq_len], (tch::Kind::Float, device)) - &lower_tri;

        // Convert to attention mask:
        // Where lower_tri = 1 (can attend): mask = 0
        // Where upper_tri = 1 (cannot attend): mask = -inf
        let neg_inf = -1e9_f64; // Use -1e9 instead of -inf for numerical stability
        let attention_mask = upper_tri * neg_inf;

        // Add batch and head dimensions: [seq_len, seq_len] → [1, 1, seq_len, seq_len]
        attention_mask.unsqueeze(0).unsqueeze(0)
    }

    /// Autoregressive generation (greedy decoding)
    ///
    /// Algorithm:
    /// 1. Embed prompt tokens and merge image features at <image> positions
    /// 2. Create causal attention mask to prevent attending to future positions
    /// 3. Loop until EOS or max_length:
    ///    a. Run forward pass with causal mask
    ///    b. Get logits for last position
    ///    c. Greedy: argmax to get next token
    ///    d. Embed new token and append
    ///    e. Extend causal mask for new position
    /// 4. Return generated sequence
    ///
    /// Note: This is a simple implementation without KV cache.
    /// Future optimization: add KV cache to avoid recomputing past tokens.
    fn generate(
        &self,
        prompt_tokens: &[u32],
        image_features: &Tensor, // Vision features [1, num_patches, hidden_size]
        max_new_tokens: usize,
    ) -> Result<Vec<u32>, Box<dyn std::error::Error>> {
        // Convert prompt tokens to i64 tensor
        let prompt_ids: Vec<i64> = prompt_tokens.iter().map(|&x| x as i64).collect();
        let prompt_tensor = Tensor::from_slice(&prompt_ids)
            .view([1, prompt_ids.len() as i64])
            .to(self.model.var_store.device());

        // Embed prompt tokens
        let mut embeddings = self.model.text_decoder.embed_tokens.forward(&prompt_tensor);

        // Merge image features at <image> token positions
        let image_token_id = self.tokenizer.image_token_id() as i64;

        // Find <image> token positions in prompt
        let image_positions: Vec<usize> = prompt_ids
            .iter()
            .enumerate()
            .filter_map(|(i, &id)| (id == image_token_id).then_some(i))
            .collect();

        if !image_positions.is_empty() {
            // Python implementation: masked_scatter to replace <image> tokens with image features
            // See: transformers/models/idefics3/modeling_idefics3.py:683
            //
            // image_features: [num_images, patches_per_image, hidden_size]
            // Example: [9, 64, 576] = 9 images * 64 patches = 576 total patches
            // These 576 patches are flattened and scattered into 576 <image> token positions

            let num_image_tokens = image_positions.len();
            let size = image_features.size();
            let num_images = size[0] as usize;
            let patches_per_image = size[1] as usize;
            let total_patches = num_images * patches_per_image;

            if num_image_tokens != total_patches {
                return Err(format!(
                    "Mismatch: {} <image> tokens but {} image feature patches ({} images * {} patches)",
                    num_image_tokens, total_patches, num_images, patches_per_image
                ).into());
            }

            // Flatten image_features: [num_images, patches_per_image, hidden_size] → [total_patches, hidden_size]
            let image_features_flat = image_features.view([total_patches as i64, size[2]]);

            // Replace <image> tokens with image features using masked scatter approach
            // embeddings: [1, seq_len, hidden_size]
            // For each <image> token position, replace with corresponding image feature patch

            let seq_len = embeddings.size()[1] as usize;
            let hidden_size = embeddings.size()[2];

            // Create new embeddings tensor by copying and replacing
            for (patch_idx, &token_pos) in image_positions.iter().enumerate() {
                // Get the patch embedding: [hidden_size]
                let patch = image_features_flat
                    .narrow(0, patch_idx as i64, 1)
                    .squeeze_dim(0);

                // Replace the embedding at token_pos
                // embeddings: [1, seq_len, hidden_size]
                // embeddings.narrow(1, token_pos, 1): [1, 1, hidden_size]
                embeddings
                    .narrow(1, token_pos as i64, 1)
                    .squeeze_dim(1)
                    .copy_(&patch);
            }
        }

        let eos_token_id = self.tokenizer.eos_token_id() as i64;
        let mut generated_tokens = prompt_ids.clone();
        let mut generated_count = 0;
        let device = self.model.var_store.device();

        // Autoregressive generation loop
        while generated_count < max_new_tokens {
            let seq_len = embeddings.size()[1];

            // Create causal attention mask for current sequence length
            let attention_mask = Self::create_causal_mask(seq_len, device);

            // Forward pass: embeddings → logits [1, seq_len, vocab_size]
            let logits =
                self.model
                    .forward_with_embeddings(&embeddings, Some(&attention_mask), false)?;

            // Get logits for last position [1, vocab_size]
            let last_logits = logits.select(1, -1);

            // Greedy decoding: argmax
            let next_token = last_logits.argmax(-1, false);

            // Check for EOS
            let next_token_id = i64::try_from(&next_token)?;
            if next_token_id == eos_token_id {
                break;
            }

            // Embed new token and append to embeddings
            let next_token_tensor = next_token.view([1, 1]);
            let next_embedding = self
                .model
                .text_decoder
                .embed_tokens
                .forward(&next_token_tensor);
            embeddings = Tensor::cat(&[embeddings, next_embedding], 1);

            // Track generated tokens
            generated_tokens.push(next_token_id);
            generated_count += 1;
        }

        // Convert back to u32 Vec
        let generated_u32: Vec<u32> = generated_tokens.iter().map(|&x| x as u32).collect();

        Ok(generated_u32)
    }
}

/// Result of code/formula enrichment
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnrichmentResult {
    pub text: String,
    pub language: Option<String>, // Programming language for code, None for formula
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> Idefics3Config {
        use config::{PerceiverConfig, TextConfig, VisionConfig};
        use std::collections::HashMap;

        let mut max_image_size = HashMap::new();
        max_image_size.insert("height".to_string(), 512);
        max_image_size.insert("width".to_string(), 512);

        let mut size = HashMap::new();
        size.insert("height".to_string(), 512);
        size.insert("width".to_string(), 512);

        Idefics3Config {
            flash_attn_2_enabled: false,
            architectures: vec!["Idefics3ForConditionalGeneration".to_string()],
            attention_bias: false,
            attention_dropout: 0.0,
            bos_token_id: 1,
            eos_token_id: 2,
            freeze_lm_head: false,
            freeze_text_layers: false,
            freeze_text_module_exceptions: vec![],
            freeze_vision_layers: false,
            freeze_vision_module_exceptions: vec![],
            head_dim: 64,
            hidden_act: "silu".to_string(),
            hidden_size: 576,
            image_token_id: 128257,
            initializer_range: 0.02,
            intermediate_size: 1536,
            max_position_embeddings: 512,
            mlp_bias: false,
            model_type: "idefics3".to_string(),
            neftune_noise_alpha: 0.0,
            num_attention_heads: 9,
            num_hidden_layers: 2, // Reduced for testing
            num_key_value_heads: 3,
            pad_token_id: 0,
            perceiver_config: PerceiverConfig {
                attention_dropout: 0.0,
                hidden_act: "silu".to_string(),
                model_type: "idefics3_perceiver".to_string(),
                num_key_value_heads: 3,
                qk_layer_norms_perceiver: false,
                resampler_depth: 2,
                resampler_head_dim: 64,
                resampler_n_heads: 12,
                resampler_n_latents: 64,
            },
            pixel_shuffle_factor: 2,
            pretraining_tp: 1,
            qk_layer_norms: false,
            rms_norm_eps: 1e-6,
            rope_scaling: None,
            rope_theta: 10000.0,
            scale_factor: 2,
            text_config: TextConfig {
                architectures: vec!["VLlamaForCausalLM".to_string()],
                attention_bias: false,
                attention_dropout: 0.0,
                bos_token_id: 1,
                eos_token_id: 2,
                head_dim: 64,
                hidden_act: "silu".to_string(),
                hidden_size: 576,
                initializer_range: 0.02,
                intermediate_size: 1536,
                max_position_embeddings: 512,
                mlp_bias: false,
                model_type: "vlllama".to_string(),
                num_attention_heads: 9,
                num_hidden_layers: 2, // Reduced for testing
                num_key_value_heads: 3,
                pretraining_tp: 1,
                rms_norm_eps: 1e-6,
                rope_scaling: None,
                rope_theta: 10000.0,
                torch_dtype: "float32".to_string(),
                use_cache: true,
                vocab_size: 152064,
                tie_word_embeddings: false,
            },
            tie_word_embeddings: false,
            torch_dtype: "float32".to_string(),
            transformers_version: "4.44.0".to_string(),
            use_cache: true,
            use_resampler: false,
            vision_config: VisionConfig {
                attention_dropout: 0.0,
                hidden_act: "gelu_pytorch_tanh".to_string(),
                hidden_size: 768,
                image_size: 512,
                initializer_range: 0.02,
                intermediate_size: 3072,
                layer_norm_eps: 1e-6,
                max_image_size,
                model_type: "siglip_vision_model".to_string(),
                num_attention_heads: 12,
                num_channels: 3,
                num_hidden_layers: 2, // Reduced for testing
                patch_size: 16,
                size,
                torch_dtype: "float32".to_string(),
                use_base_siglip: true,
            },
        }
    }

    #[test]
    fn test_load_config() {
        let model_dir = std::path::Path::new(env!("HOME"))
            .join(".cache/huggingface/hub/models--ds4sd--CodeFormulaV2/snapshots/ecedbe111d15c2dc60bfd4a823cbe80127b58af4");

        if !model_dir.exists() {
            log::warn!("Skipping test: model directory not found");
            return;
        }

        let device = tch::Device::Cpu;
        let model = CodeFormulaModel::from_pretrained(&model_dir, device);
        assert!(model.is_ok(), "Failed to load model: {:?}", model.err());
    }

    #[test]
    #[ignore] // Flaky: segfaults due to tch-rs/PyTorch memory management issues (N=476)
    fn test_get_image_features() {
        // Create a small test config
        let config = create_test_config();
        let device = tch::Device::Cpu;
        let model = Idefics3Model::new(&config, device);

        // Dummy image [1, 3, 512, 512]
        let pixel_values = Tensor::randn([1, 3, 512, 512], (tch::Kind::Float, tch::Device::Cpu));

        // Get image features
        let features = model.get_image_features(&pixel_values).unwrap();

        // Expected: [1, 256, 576]
        // - 512x512 image → 32x32 patches (patch_size=16) → 1024 patches
        // - Pixel shuffle 4→1 (scale_factor=2) → 1024/4 = 256 patches
        // - Projection: 768*4 → 576
        assert_eq!(features.size(), vec![1, 256, 576]);
    }

    #[test]
    fn test_forward() {
        // Create a small test config
        let config = create_test_config();
        let device = tch::Device::Cpu;
        let model = Idefics3Model::new(&config, device);

        // Dummy input_ids [batch=2, seq_len=10]
        let input_ids = Tensor::randint(
            config.text_config.vocab_size as i64,
            [2, 10],
            (tch::Kind::Int64, tch::Device::Cpu),
        );

        // Forward pass
        let logits = model.forward(&input_ids, None, false).unwrap();

        // Expected: [batch=2, seq_len=10, vocab_size]
        assert_eq!(logits.size()[0], 2);
        assert_eq!(logits.size()[1], 10);
        assert_eq!(logits.size()[2], config.text_config.vocab_size as i64);
    }
}
