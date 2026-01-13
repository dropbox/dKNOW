//! CLIP embedding model for image and text search
//!
//! OpenAI's CLIP (Contrastive Language-Image Pre-training) model enables
//! cross-modal retrieval between images and text.
//!
//! Architecture: Vision Transformer (ViT) + Text Transformer
//! Output: 512 dimensions (single-vector)
//! Scoring: Cosine similarity
//!
//! This module provides both text and image embedding capabilities:
//! - Text embeddings for cross-modal queries (search images with text)
//! - Image embeddings for indexing images
//!
//! Model: openai/clip-vit-base-patch32

use anyhow::{Context, Result};
use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::clip::{ClipConfig, ClipModel};
use hf_hub::api::sync::ApiBuilder;
use std::path::Path;
use tokenizers::Tokenizer;

use crate::embedder::{EmbedderBackend, EmbeddingResult};

/// CLIP embedding dimension (ViT-B/32)
pub const CLIP_DIM: usize = 512;

/// CLIP image input size (224x224 pixels)
pub const CLIP_IMAGE_SIZE: usize = 224;

/// Maximum sequence length for CLIP text encoder
const MAX_SEQ_LEN: usize = 77;

/// CLIP embedder using Candle
///
/// Supports both text and image embedding for cross-modal search.
pub struct ClipEmbedder {
    model: ClipModel,
    tokenizer: Tokenizer,
    device: Device,
}

impl ClipEmbedder {
    /// Load CLIP from HuggingFace hub
    pub fn new(device: &Device) -> Result<Self> {
        Self::from_pretrained("openai/clip-vit-base-patch32", device)
    }

    /// Load from a specific model path or HuggingFace model ID
    pub fn from_pretrained(model_id: &str, device: &Device) -> Result<Self> {
        // Try hf-hub first
        match Self::try_hf_hub(model_id, device) {
            Ok(embedder) => return Ok(embedder),
            Err(e) => {
                tracing::debug!("hf-hub download failed: {}, trying manual download", e);
            }
        }

        Self::try_manual_download(model_id, device)
    }

    fn try_hf_hub(model_id: &str, device: &Device) -> Result<Self> {
        let api = ApiBuilder::new()
            .with_progress(true)
            .build()
            .context("Failed to create HuggingFace API")?;
        let repo = api.model(model_id.to_string());

        tracing::info!("Downloading CLIP model files from {}", model_id);
        let config_path = repo
            .get("config.json")
            .context("Failed to get config.json")?;
        let tokenizer_path = repo
            .get("tokenizer.json")
            .context("Failed to get tokenizer.json")?;
        let weights_path = repo
            .get("model.safetensors")
            .or_else(|_| repo.get("pytorch_model.bin"))
            .context("Failed to get model weights")?;

        Self::from_files(&config_path, &tokenizer_path, &weights_path, device)
    }

    fn try_manual_download(model_id: &str, device: &Device) -> Result<Self> {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("sg")
            .join("models")
            .join(model_id.replace('/', "_"));

        std::fs::create_dir_all(&cache_dir)?;

        let base_url = format!("https://huggingface.co/{model_id}/resolve/main");

        let files = ["config.json", "tokenizer.json", "pytorch_model.bin"];
        let mut paths = Vec::new();

        for file in &files {
            let local_path = cache_dir.join(file);
            if !local_path.exists() {
                let url = format!("{base_url}/{file}");
                tracing::info!("Downloading {}...", file);

                let response = ureq::get(&url)
                    .call()
                    .with_context(|| format!("Failed to download {file}"))?;

                let mut out = std::fs::File::create(&local_path)?;
                std::io::copy(&mut response.into_reader(), &mut out)?;
                tracing::info!("Downloaded {}", file);
            }
            paths.push(local_path);
        }

        Self::from_files(&paths[0], &paths[1], &paths[2], device)
    }

    /// Load from local files
    pub fn from_files(
        _config_path: &Path,
        tokenizer_path: &Path,
        weights_path: &Path,
        device: &Device,
    ) -> Result<Self> {
        // Use standard ViT-B/32 config
        let config = ClipConfig::vit_base_patch32();

        // Load tokenizer
        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {e}"))?;

        // Load model weights
        let dtype = if device.is_cuda() {
            DType::F16
        } else {
            DType::F32
        };
        let vb = if weights_path
            .extension()
            .is_some_and(|ext| ext == "safetensors")
        {
            unsafe { VarBuilder::from_mmaped_safetensors(&[weights_path], dtype, device)? }
        } else {
            VarBuilder::from_pth(weights_path, dtype, device)?
        };

        let model = ClipModel::new(vb, &config)?;

        Ok(Self {
            model,
            tokenizer,
            device: device.clone(),
        })
    }

    /// Embed text using CLIP's text encoder
    fn embed_text(&mut self, text: &str) -> Result<EmbeddingResult> {
        // Tokenize
        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| anyhow::anyhow!("Tokenization failed: {e}"))?;

        let mut input_ids: Vec<i64> = encoding.get_ids().iter().map(|&x| x as i64).collect();

        // Truncate or pad to MAX_SEQ_LEN
        if input_ids.len() > MAX_SEQ_LEN {
            input_ids.truncate(MAX_SEQ_LEN);
        }
        while input_ids.len() < MAX_SEQ_LEN {
            input_ids.push(0); // Pad token
        }

        // Create tensor
        let input_ids = Tensor::new(&input_ids[..], &self.device)?.unsqueeze(0)?;

        // Get text features
        let text_features = self.model.get_text_features(&input_ids)?;

        // L2 normalize
        let normalized = l2_normalize(&text_features)?;

        // Extract embedding
        let embedding = normalized.squeeze(0)?.to_vec1::<f32>()?;

        Ok(EmbeddingResult {
            data: embedding,
            num_tokens: 1, // Single-vector model
        })
    }

    /// Embed an image using CLIP's vision encoder
    ///
    /// The image should be preprocessed to 224x224 RGB and normalized.
    /// Input tensor shape: (1, 3, 224, 224)
    pub fn embed_image(&mut self, image_tensor: &Tensor) -> Result<EmbeddingResult> {
        // Get vision features
        let image_features = self.model.get_image_features(image_tensor)?;

        // L2 normalize
        let normalized = l2_normalize(&image_features)?;

        // Extract embedding
        let embedding = normalized.squeeze(0)?.to_vec1::<f32>()?;

        Ok(EmbeddingResult {
            data: embedding,
            num_tokens: 1,
        })
    }

    /// Preprocess an image for CLIP
    ///
    /// Resizes to 224x224, converts to RGB, and normalizes with CLIP's
    /// mean/std values.
    pub fn preprocess_image(&self, image: &image::DynamicImage) -> Result<Tensor> {
        use image::imageops::FilterType;

        // Resize to 224x224
        let resized = image.resize_exact(
            CLIP_IMAGE_SIZE as u32,
            CLIP_IMAGE_SIZE as u32,
            FilterType::Triangle,
        );

        // Convert to RGB
        let rgb = resized.to_rgb8();

        // Convert to float tensor and normalize
        let mut data = Vec::with_capacity(3 * CLIP_IMAGE_SIZE * CLIP_IMAGE_SIZE);

        // CLIP normalization values (standard ImageNet values used by CLIP)
        let mean = [0.48145466_f32, 0.4578275, 0.40821073];
        #[allow(clippy::excessive_precision)] // Standard CLIP constants
        let std = [0.26862954_f32, 0.26130258, 0.27577711];

        // HWC -> CHW format
        for c in 0..3 {
            for y in 0..CLIP_IMAGE_SIZE {
                for x in 0..CLIP_IMAGE_SIZE {
                    let pixel = rgb.get_pixel(x as u32, y as u32);
                    let value = pixel[c] as f32 / 255.0;
                    let normalized = (value - mean[c]) / std[c];
                    data.push(normalized);
                }
            }
        }

        let tensor =
            Tensor::from_vec(data, (1, 3, CLIP_IMAGE_SIZE, CLIP_IMAGE_SIZE), &self.device)?;

        Ok(tensor)
    }

    /// Embed an image from a file path
    pub fn embed_image_file(&mut self, path: &Path) -> Result<EmbeddingResult> {
        let image = image::open(path)
            .with_context(|| format!("Failed to open image: {}", path.display()))?;
        let tensor = self.preprocess_image(&image)?;
        self.embed_image(&tensor)
    }
}

impl EmbedderBackend for ClipEmbedder {
    fn embed_document(&mut self, text: &str) -> Result<EmbeddingResult> {
        self.embed_text(text)
    }

    fn embed_query(&mut self, text: &str) -> Result<EmbeddingResult> {
        self.embed_text(text)
    }

    fn embedding_dim(&self) -> usize {
        CLIP_DIM
    }
}

/// L2 normalize a tensor along the last dimension
fn l2_normalize(tensor: &Tensor) -> Result<Tensor> {
    let norm = tensor.sqr()?.sum_keepdim(1)?.sqrt()?;
    let normalized = tensor.broadcast_div(&norm.clamp(1e-12, f64::MAX)?)?;
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clip_dim() {
        assert_eq!(CLIP_DIM, 512);
    }

    #[test]
    fn test_clip_image_size() {
        assert_eq!(CLIP_IMAGE_SIZE, 224);
    }
}
