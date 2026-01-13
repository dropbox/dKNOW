//! Jina-ColBERT-v2 embedding backend
//!
//! This module provides a Jina-ColBERT-v2 embedder for multilingual late-interaction retrieval.
//! Model: jinaai/jina-colbert-v2
//!
//! # Features
//!
//! - **Multi-vector (ColBERT)**: Produces 128-dim embedding per token
//! - **94 languages**: Native support for CJK and other scripts
//! - **8192 token context**: Much longer than XTR's 512 tokens
//! - **MaxSim scoring**: Same late-interaction scoring as XTR
//!
//! # Model Details
//!
//! - Architecture: XLM-RoBERTa with rotary position embeddings
//! - Parameters: ~0.6B (BF16)
//! - Output: 128 dimensions per token (Matryoshka - can truncate to 96 or 64)
//! - Special tokens: `[QueryMarker]` prefix for queries, `[DocumentMarker]` for documents
//!
//! # Usage
//!
//! ```rust,ignore
//! use sg_core::embedder_jina_colbert::JinaColBertEmbedder;
//!
//! let mut embedder = JinaColBertEmbedder::new()?;
//! let doc_emb = embedder.embed_document("Some text in any language")?;
//! let query_emb = embedder.embed_query("検索クエリ")?; // Japanese query
//! ```

use anyhow::{Context, Result};
use hf_hub::api::sync::ApiBuilder;
use ndarray::Array2;
use ort::{session::Session, value::Value};
use std::path::{Path, PathBuf};
use tokenizers::Tokenizer;

use crate::embedder::{l2_normalize_ndarray, EmbedderBackend, EmbeddingResult};

/// Jina-ColBERT output dimension (128-dim per token)
pub const JINA_COLBERT_DIM: usize = 128;

/// Maximum sequence length for documents (Jina-ColBERT supports 8192)
const DOC_MAXLEN: usize = 8192;

/// Maximum sequence length for queries
const QUERY_MAXLEN: usize = 128;

/// Query prefix marker for Jina-ColBERT
const QUERY_PREFIX: &str = "[QueryMarker] ";

/// Document prefix marker for Jina-ColBERT
const DOCUMENT_PREFIX: &str = "[DocumentMarker] ";

/// Jina-ColBERT-v2 embedder using ONNX Runtime
///
/// This embedder uses the official Jina-ColBERT-v2 ONNX model for inference.
/// It produces multi-vector (token-level) embeddings for late-interaction retrieval.
pub struct JinaColBertEmbedder {
    session: Session,
    tokenizer: Tokenizer,
}

impl JinaColBertEmbedder {
    /// Load the Jina-ColBERT embedder from HuggingFace hub
    ///
    /// Downloads the model and tokenizer to the cache directory if not present.
    pub fn new() -> Result<Self> {
        let model_dir = Self::ensure_model_downloaded()?;
        Self::from_dir(&model_dir)
    }

    /// Get the cache directory for Jina-ColBERT model
    fn cache_dir() -> Result<PathBuf> {
        let cache_dir = dirs::cache_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine cache directory"))?;
        Ok(cache_dir.join("sg").join("models").join("jina-colbert-v2"))
    }

    /// Download model files from HuggingFace if not cached
    fn ensure_model_downloaded() -> Result<PathBuf> {
        let cache_dir = Self::cache_dir()?;
        let onnx_path = cache_dir.join("model.onnx");
        let onnx_data_path = cache_dir.join("model.onnx_data");
        let tokenizer_path = cache_dir.join("tokenizer.json");

        // Check if already downloaded (need all 3 files: model.onnx, model.onnx_data, tokenizer.json)
        if onnx_path.exists() && onnx_data_path.exists() && tokenizer_path.exists() {
            tracing::debug!("Using cached Jina-ColBERT model at {}", cache_dir.display());
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

        let repo = api.model("jinaai/jina-colbert-v2".to_string());

        tracing::info!("Downloading Jina-ColBERT-v2 model files...");

        // Download ONNX model (main file - contains graph structure)
        let onnx_remote = repo
            .get("onnx/model.onnx")
            .context("Failed to download ONNX model")?;
        std::fs::copy(&onnx_remote, cache_dir.join("model.onnx"))?;

        // Download ONNX external data file (contains weights - 2.2GB)
        let onnx_data_remote = repo
            .get("onnx/model.onnx_data")
            .context("Failed to download ONNX model weights")?;
        std::fs::copy(&onnx_data_remote, cache_dir.join("model.onnx_data"))?;

        // Download tokenizer
        let tokenizer_remote = repo
            .get("tokenizer.json")
            .context("Failed to download tokenizer")?;
        std::fs::copy(&tokenizer_remote, cache_dir.join("tokenizer.json"))?;

        tracing::info!("Jina-ColBERT-v2 model downloaded successfully");
        Ok(())
    }

    /// Manual download fallback
    fn download_manual(cache_dir: &Path) -> Result<()> {
        let base_url = "https://huggingface.co/jinaai/jina-colbert-v2/resolve/main";

        // Download ONNX model (main file - contains graph structure)
        let onnx_url = format!("{base_url}/onnx/model.onnx");
        let onnx_path = cache_dir.join("model.onnx");
        if !onnx_path.exists() {
            tracing::info!("Downloading ONNX model graph (~4MB)...");
            let response = ureq::get(&onnx_url)
                .call()
                .context("Failed to download ONNX model")?;
            let mut file = std::fs::File::create(&onnx_path)?;
            std::io::copy(&mut response.into_reader(), &mut file)?;
        }

        // Download ONNX external data file (contains weights - 2.2GB)
        let onnx_data_url = format!("{base_url}/onnx/model.onnx_data");
        let onnx_data_path = cache_dir.join("model.onnx_data");
        if !onnx_data_path.exists() {
            tracing::info!("Downloading ONNX model weights (~2.2GB)...");
            let response = ureq::get(&onnx_data_url)
                .call()
                .context("Failed to download ONNX model weights")?;
            let mut file = std::fs::File::create(&onnx_data_path)?;
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

        tracing::info!("Jina-ColBERT-v2 model downloaded successfully");
        Ok(())
    }

    /// Load from a specific directory containing the ONNX model and tokenizer
    pub fn from_dir(model_dir: &Path) -> Result<Self> {
        let model_path = model_dir.join("model.onnx");
        let tokenizer_path = model_dir.join("tokenizer.json");

        if !model_path.exists() {
            return Err(anyhow::anyhow!(
                "Jina-ColBERT ONNX model not found at {}. The model will be downloaded automatically.",
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
            "Loading Jina-ColBERT-v2 ONNX model from {}",
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

        tracing::info!("Jina-ColBERT-v2 embedder loaded successfully");

        Ok(Self { session, tokenizer })
    }

    /// Generate embeddings for a document
    pub fn embed_document(&mut self, text: &str) -> Result<Array2<f32>> {
        let prefixed_text = format!("{DOCUMENT_PREFIX}{text}");
        self.embed(&prefixed_text, DOC_MAXLEN)
    }

    /// Generate embeddings for a query
    pub fn embed_query(&mut self, text: &str) -> Result<Array2<f32>> {
        let prefixed_text = format!("{QUERY_PREFIX}{text}");
        self.embed(&prefixed_text, QUERY_MAXLEN)
    }

    /// Generate embeddings for text with specified max length
    fn embed(&mut self, text: &str, max_len: usize) -> Result<Array2<f32>> {
        // Tokenize (Jina-ColBERT does NOT lowercase - it's multilingual)
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
            .context("Jina-ColBERT inference failed")?;

        // Extract embeddings: expected shape [1, seq_len, 128]
        let embeddings_tensor = outputs[0]
            .try_extract_tensor::<f32>()
            .context("Failed to extract output tensor")?;

        let (shape, data) = embeddings_tensor;
        let dims: Vec<usize> = shape.iter().map(|&d| d as usize).collect();

        // Validate output shape
        if dims.len() != 3 || dims[0] != 1 {
            return Err(anyhow::anyhow!(
                "Unexpected output shape: {dims:?}. Expected [1, seq_len, {JINA_COLBERT_DIM}]"
            ));
        }

        // Remove batch dimension: [seq_len, embed_dim]
        let embed_dim = dims[2];
        let embeddings = Array2::from_shape_vec((dims[1], embed_dim), data.to_vec())
            .context("Failed to reshape embeddings")?;

        // Truncate to 128 dims if needed (Matryoshka embeddings)
        let embeddings = if embed_dim > JINA_COLBERT_DIM {
            embeddings
                .slice(ndarray::s![.., ..JINA_COLBERT_DIM])
                .to_owned()
        } else {
            embeddings
        };

        // L2 normalize each token embedding
        let normalized = l2_normalize_ndarray(&embeddings)?;

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
        JINA_COLBERT_DIM
    }
}

/// Implement EmbedderBackend for Jina-ColBERT
impl EmbedderBackend for JinaColBertEmbedder {
    fn embed_document(&mut self, text: &str) -> Result<EmbeddingResult> {
        let array = JinaColBertEmbedder::embed_document(self, text)?;
        let num_tokens = array.nrows();
        let data = embeddings_to_vec(&array);
        Ok(EmbeddingResult::new(data, num_tokens))
    }

    fn embed_query(&mut self, text: &str) -> Result<EmbeddingResult> {
        let array = JinaColBertEmbedder::embed_query(self, text)?;
        let num_tokens = array.nrows();
        let data = embeddings_to_vec(&array);
        Ok(EmbeddingResult::new(data, num_tokens))
    }

    fn embedding_dim(&self) -> usize {
        JINA_COLBERT_DIM
    }
}

/// Convert ndarray embeddings to flat f32 vec for storage
pub fn embeddings_to_vec(embeddings: &Array2<f32>) -> Vec<f32> {
    embeddings.iter().copied().collect()
}

/// Convert flat f32 vec back to ndarray embeddings
pub fn vec_to_embeddings(data: &[f32], num_tokens: usize) -> Result<Array2<f32>> {
    Array2::from_shape_vec((num_tokens, JINA_COLBERT_DIM), data.to_vec())
        .context("Failed to convert vec to embeddings")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_dim_constant() {
        assert_eq!(JINA_COLBERT_DIM, 128);
    }

    #[test]
    fn test_doc_maxlen_constant() {
        // Jina-ColBERT supports 8192 tokens
        assert_eq!(DOC_MAXLEN, 8192);
    }

    #[test]
    fn test_query_prefix() {
        assert_eq!(QUERY_PREFIX, "[QueryMarker] ");
    }

    #[test]
    fn test_document_prefix() {
        assert_eq!(DOCUMENT_PREFIX, "[DocumentMarker] ");
    }

    #[test]
    fn test_embeddings_conversion() {
        let data: Vec<f32> = (0..256).map(|i| i as f32).collect();
        let embeddings = vec_to_embeddings(&data, 2).unwrap();
        assert_eq!(embeddings.shape(), &[2, JINA_COLBERT_DIM]);

        let back = embeddings_to_vec(&embeddings);
        assert_eq!(data, back);
    }

    /// Compile-time test: verify JinaColBertEmbedder implements EmbedderBackend
    #[allow(dead_code)]
    fn verify_embedder_backend_impl() {
        fn requires_backend<E: EmbedderBackend>(_: &mut E) {}

        fn _test_jina_colbert_embedder(embedder: &mut JinaColBertEmbedder) {
            requires_backend(embedder);
        }
    }
}
