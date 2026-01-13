//! Jina-Code-v2 embedding backend
//!
//! This module provides a Jina-Code-v2 embedder for code search.
//! Model: jinaai/jina-embeddings-v2-base-code
//!
//! # Features
//!
//! - **Single-vector**: Produces one 768-dim embedding per document (mean pooling)
//! - **30+ programming languages**: Python, JavaScript, C++, Java, Rust, Go, etc.
//! - **8192 token context**: Much longer than UniXcoder's 512 tokens
//! - **Cosine similarity**: Standard single-vector scoring
//!
//! # Model Details
//!
//! - Architecture: JinaBert with ALiBi positional encoding
//! - Parameters: 161M
//! - Output: 768 dimensions (mean pooling over all tokens)
//! - Pooling: Mean (average of all token embeddings)
//!
//! # Comparison with UniXcoder
//!
//! | Feature | Jina-Code-v2 | UniXcoder |
//! |---------|--------------|-----------|
//! | Context | 8192 tokens | 512 tokens |
//! | Languages | 30+ | 6 |
//! | Pooling | Mean | CLS |
//! | Dim | 768 | 768 |

use anyhow::{Context, Result};
use hf_hub::api::sync::ApiBuilder;
use ndarray::{Array1, Array2, Axis};
use ort::{session::Session, value::Value};
use std::path::{Path, PathBuf};
use tokenizers::Tokenizer;

use crate::embedder::{EmbedderBackend, EmbeddingResult};

/// Jina-Code-v2 embedding dimension
pub const JINA_CODE_DIM: usize = 768;

/// Maximum sequence length for documents (Jina-Code supports 8192)
const DOC_MAXLEN: usize = 8192;

/// Maximum sequence length for queries (shorter for efficiency)
const QUERY_MAXLEN: usize = 256;

/// Jina-Code-v2 embedder using ONNX Runtime
///
/// This embedder uses the Jina-Code-v2 ONNX model for inference.
/// It produces single-vector embeddings using mean pooling.
pub struct JinaCodeEmbedder {
    session: Session,
    tokenizer: Tokenizer,
}

impl JinaCodeEmbedder {
    /// Load the Jina-Code embedder from HuggingFace hub
    ///
    /// Downloads the model and tokenizer to the cache directory if not present.
    pub fn new() -> Result<Self> {
        let model_dir = Self::ensure_model_downloaded()?;
        Self::from_dir(&model_dir)
    }

    /// Get the cache directory for Jina-Code model
    fn cache_dir() -> Result<PathBuf> {
        let cache_dir = dirs::cache_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine cache directory"))?;
        Ok(cache_dir.join("sg").join("models").join("jina-code-v2"))
    }

    /// Download model files from HuggingFace if not cached
    fn ensure_model_downloaded() -> Result<PathBuf> {
        let cache_dir = Self::cache_dir()?;
        let onnx_path = cache_dir.join("model_fp16.onnx");
        let tokenizer_path = cache_dir.join("tokenizer.json");

        // Check if already downloaded
        if onnx_path.exists() && tokenizer_path.exists() {
            tracing::debug!("Using cached Jina-Code model at {}", cache_dir.display());
            return Ok(cache_dir);
        }

        std::fs::create_dir_all(&cache_dir)?;

        // Try hf-hub API first
        match Self::download_via_hub(&cache_dir) {
            Ok(()) => return Ok(cache_dir),
            Err(e) => {
                tracing::warn!("hf-hub download failed: {}. Trying manual download.", e);
            }
        }

        // Fallback to manual download
        Self::download_manual(&cache_dir)?;
        Ok(cache_dir)
    }

    /// Download using hf-hub API
    fn download_via_hub(cache_dir: &Path) -> Result<()> {
        let api = ApiBuilder::new()
            .with_progress(true)
            .build()
            .context("Failed to create HuggingFace API")?;

        let repo = api.model("jinaai/jina-embeddings-v2-base-code".to_string());

        tracing::info!("Downloading Jina-Code-v2 model files...");

        // Download fp16 ONNX model (smaller than full precision)
        let onnx_remote = repo
            .get("onnx/model_fp16.onnx")
            .context("Failed to download ONNX model")?;
        std::fs::copy(&onnx_remote, cache_dir.join("model_fp16.onnx"))?;

        // Download tokenizer
        let tokenizer_remote = repo
            .get("tokenizer.json")
            .context("Failed to download tokenizer")?;
        std::fs::copy(&tokenizer_remote, cache_dir.join("tokenizer.json"))?;

        tracing::info!("Jina-Code-v2 model downloaded successfully");
        Ok(())
    }

    /// Manual download fallback
    fn download_manual(cache_dir: &Path) -> Result<()> {
        let base_url = "https://huggingface.co/jinaai/jina-embeddings-v2-base-code/resolve/main";

        // Download fp16 ONNX model
        let onnx_url = format!("{base_url}/onnx/model_fp16.onnx");
        let onnx_path = cache_dir.join("model_fp16.onnx");
        if !onnx_path.exists() {
            tracing::info!("Downloading ONNX model (~321MB)...");
            let response = ureq::get(&onnx_url)
                .call()
                .context("Failed to download ONNX model")?;
            let mut file = std::fs::File::create(&onnx_path)?;
            std::io::copy(&mut response.into_reader(), &mut file)?;
        }

        // Download tokenizer
        let tokenizer_url = format!("{base_url}/tokenizer.json");
        let tokenizer_path = cache_dir.join("tokenizer.json");
        if !tokenizer_path.exists() {
            tracing::info!("Downloading tokenizer...");
            let response = ureq::get(&tokenizer_url)
                .call()
                .context("Failed to download tokenizer")?;
            let mut file = std::fs::File::create(&tokenizer_path)?;
            std::io::copy(&mut response.into_reader(), &mut file)?;
        }

        tracing::info!("Jina-Code-v2 model downloaded successfully");
        Ok(())
    }

    /// Load from a specific directory containing the ONNX model and tokenizer
    pub fn from_dir(model_dir: &Path) -> Result<Self> {
        let model_path = model_dir.join("model_fp16.onnx");
        let tokenizer_path = model_dir.join("tokenizer.json");

        if !model_path.exists() {
            return Err(anyhow::anyhow!(
                "Jina-Code ONNX model not found at {}. The model will be downloaded automatically.",
                model_path.display()
            ));
        }

        if !tokenizer_path.exists() {
            return Err(anyhow::anyhow!(
                "Tokenizer not found at {}. The tokenizer will be downloaded automatically.",
                tokenizer_path.display()
            ));
        }

        Self::from_files(&model_path, &tokenizer_path)
    }

    /// Load from specific file paths
    pub fn from_files(model_path: &Path, tokenizer_path: &Path) -> Result<Self> {
        tracing::info!(
            "Loading Jina-Code-v2 ONNX model from {}",
            model_path.display()
        );

        // Initialize ONNX Runtime session
        let session = Session::builder()
            .context("Failed to create ONNX session builder")?
            .with_intra_threads(4)
            .context("Failed to set thread count")?
            .commit_from_file(model_path)
            .with_context(|| format!("Failed to load ONNX model from {}", model_path.display()))?;

        // Load tokenizer
        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {e}"))?;

        tracing::info!("Jina-Code-v2 embedder loaded successfully");

        Ok(Self { session, tokenizer })
    }

    /// Generate embedding for a document using mean pooling
    pub fn embed_document(&mut self, text: &str) -> Result<Array1<f32>> {
        self.embed(text, DOC_MAXLEN)
    }

    /// Generate embedding for a query
    pub fn embed_query(&mut self, text: &str) -> Result<Array1<f32>> {
        self.embed(text, QUERY_MAXLEN)
    }

    /// Generate embedding for text with specified max length
    fn embed(&mut self, text: &str, max_len: usize) -> Result<Array1<f32>> {
        // Tokenize
        let (input_ids, attention_mask) = self.tokenize(text, max_len)?;
        let seq_len = input_ids.len();

        // Convert to ONNX inputs
        let input_ids_array =
            Array2::from_shape_vec((1, seq_len), input_ids.iter().map(|&x| x as i64).collect())
                .context("Failed to create input_ids array")?;
        let attention_mask_array = Array2::from_shape_vec(
            (1, seq_len),
            attention_mask.iter().map(|&x| x as i64).collect(),
        )
        .context("Failed to create attention_mask array")?;

        let input_ids_value =
            Value::from_array(input_ids_array).context("Failed to create input_ids Value")?;
        let attention_mask_value = Value::from_array(attention_mask_array)
            .context("Failed to create attention_mask Value")?;

        // Run inference
        let outputs = self
            .session
            .run(ort::inputs![input_ids_value, attention_mask_value])
            .context("Jina-Code inference failed")?;

        // Extract embeddings: expected shape [1, seq_len, 768]
        let embeddings_tensor = outputs[0]
            .try_extract_tensor::<f32>()
            .context("Failed to extract output tensor")?;

        let (shape, data) = embeddings_tensor;
        let dims: Vec<usize> = shape.iter().map(|&d| d as usize).collect();

        // Validate output shape
        if dims.len() != 3 || dims[0] != 1 {
            return Err(anyhow::anyhow!(
                "Unexpected output shape: {dims:?}. Expected [1, seq_len, 768]"
            ));
        }

        // Extract as [seq_len, embed_dim]
        let seq_len = dims[1];
        let embed_dim = dims[2];
        let embeddings = Array2::from_shape_vec((seq_len, embed_dim), data.to_vec())
            .context("Failed to reshape embeddings")?;

        // Mean pooling: average over all tokens (respecting attention mask)
        // For simplicity, we average all tokens since we don't pad
        let mean_embedding = embeddings
            .mean_axis(Axis(0))
            .ok_or_else(|| anyhow::anyhow!("Failed to compute mean pooling"))?;

        // L2 normalize
        let normalized = l2_normalize_1d(&mean_embedding)?;

        Ok(normalized)
    }

    /// Tokenize text and return input_ids and attention_mask
    fn tokenize(&self, text: &str, max_len: usize) -> Result<(Vec<u32>, Vec<u32>)> {
        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| anyhow::anyhow!("Tokenization failed: {e}"))?;

        let mut input_ids: Vec<u32> = encoding.get_ids().to_vec();

        // Truncate if needed
        if input_ids.len() > max_len {
            input_ids.truncate(max_len);
        }

        // Create attention mask (all 1s for non-padded tokens)
        let attention_mask: Vec<u32> = vec![1; input_ids.len()];

        Ok((input_ids, attention_mask))
    }

    /// Get embedding dimension
    pub fn embedding_dim(&self) -> usize {
        JINA_CODE_DIM
    }
}

/// L2 normalize a 1D array
fn l2_normalize_1d(array: &Array1<f32>) -> Result<Array1<f32>> {
    let norm = array.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-12);
    Ok(array.mapv(|x| x / norm))
}

/// Implement EmbedderBackend for Jina-Code
impl EmbedderBackend for JinaCodeEmbedder {
    fn embed_document(&mut self, text: &str) -> Result<EmbeddingResult> {
        let array = JinaCodeEmbedder::embed_document(self, text)?;
        let data = array.to_vec();
        // Single-vector: 1 "token" with JINA_CODE_DIM dimensions
        Ok(EmbeddingResult::new(data, 1))
    }

    fn embed_query(&mut self, text: &str) -> Result<EmbeddingResult> {
        let array = JinaCodeEmbedder::embed_query(self, text)?;
        let data = array.to_vec();
        Ok(EmbeddingResult::new(data, 1))
    }

    fn embedding_dim(&self) -> usize {
        JINA_CODE_DIM
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_dim_constant() {
        assert_eq!(JINA_CODE_DIM, 768);
    }

    #[test]
    fn test_doc_maxlen_constant() {
        // Jina-Code supports 8192 tokens
        assert_eq!(DOC_MAXLEN, 8192);
    }

    #[test]
    fn test_query_maxlen_constant() {
        assert_eq!(QUERY_MAXLEN, 256);
    }

    #[test]
    fn test_l2_normalize() {
        let arr = Array1::from_vec(vec![3.0, 4.0]);
        let normalized = l2_normalize_1d(&arr).unwrap();
        assert!((normalized[0] - 0.6).abs() < 1e-5);
        assert!((normalized[1] - 0.8).abs() < 1e-5);
    }

    #[test]
    fn test_l2_normalize_unit() {
        let arr = Array1::from_vec(vec![1.0, 0.0, 0.0]);
        let normalized = l2_normalize_1d(&arr).unwrap();
        assert!((normalized[0] - 1.0).abs() < 1e-5);
        assert!((normalized[1] - 0.0).abs() < 1e-5);
    }

    /// Compile-time test: verify JinaCodeEmbedder implements EmbedderBackend
    #[allow(dead_code)]
    fn verify_embedder_backend_impl() {
        fn requires_backend<E: EmbedderBackend>(_: &mut E) {}

        fn _test_jina_code_embedder(embedder: &mut JinaCodeEmbedder) {
            requires_backend(embedder);
        }
    }
}
