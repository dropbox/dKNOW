//! ONNX Runtime embedding backend
//!
//! This module provides an ONNX Runtime-based embedder for cross-platform inference.
//! The ONNX model must be exported using the `scripts/export_onnx.py` script.
//!
//! # Model Location
//!
//! The ONNX model is expected at:
//! - `~/.cache/sg/models/onnx/xtr_encoder.onnx`
//!
//! # Usage
//!
//! ```rust,ignore
//! use sg_core::embedder_onnx::OnnxEmbedder;
//!
//! let embedder = OnnxEmbedder::new()?;
//! let embeddings = embedder.embed_document("some text")?;
//! ```

use anyhow::{Context, Result};
use ndarray::Array2;
use ort::{session::Session, value::Value};
use std::path::{Path, PathBuf};
use tokenizers::Tokenizer;

use crate::embedder::{l2_normalize_ndarray, EMBEDDING_DIM};

/// Hidden dimension of T5 encoder
const HIDDEN_DIM: usize = 768;

/// Maximum sequence length for documents
const DOC_MAXLEN: usize = 512;

/// Maximum sequence length for queries
const QUERY_MAXLEN: usize = 64;

/// ONNX Runtime-based embedder
pub struct OnnxEmbedder {
    session: Session,
    tokenizer: Tokenizer,
    projection_weights: Array2<f32>,
}

impl OnnxEmbedder {
    /// Load the ONNX embedder from the default cache location
    pub fn new() -> Result<Self> {
        let model_dir = Self::default_model_dir()?;
        Self::from_dir(&model_dir)
    }

    /// Get the default model directory
    fn default_model_dir() -> Result<PathBuf> {
        let cache_dir = dirs::cache_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine cache directory"))?;
        Ok(cache_dir.join("sg").join("models").join("onnx"))
    }

    /// Load from a specific directory containing the ONNX model and tokenizer
    pub fn from_dir(model_dir: &Path) -> Result<Self> {
        let model_path = model_dir.join("xtr_encoder.onnx");
        let tokenizer_path = model_dir.join("tokenizer.json");

        if !model_path.exists() {
            return Err(anyhow::anyhow!(
                "ONNX model not found at {}. Run: python scripts/export_onnx.py",
                model_path.display()
            ));
        }

        if !tokenizer_path.exists() {
            return Err(anyhow::anyhow!(
                "Tokenizer not found at {}. Run: python scripts/export_onnx.py",
                tokenizer_path.display()
            ));
        }

        Self::from_files(&model_path, &tokenizer_path)
    }

    /// Load from specific file paths
    pub fn from_files(model_path: &Path, tokenizer_path: &Path) -> Result<Self> {
        tracing::info!("Loading ONNX model from {}", model_path.display());

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

        // Initialize random projection weights (same as Candle backend)
        // We use a fixed seed for reproducibility
        let projection_weights = Self::init_projection_weights();

        tracing::info!("ONNX embedder loaded successfully");

        Ok(Self {
            session,
            tokenizer,
            projection_weights,
        })
    }

    /// Initialize random projection weights (768 -> 128)
    fn init_projection_weights() -> Array2<f32> {
        use rand::{rngs::StdRng, Rng, SeedableRng};

        // Use fixed seed for reproducibility across backends
        let mut rng = StdRng::seed_from_u64(42);

        // Random normal initialization
        let data: Vec<f32> = (0..EMBEDDING_DIM * HIDDEN_DIM)
            .map(|_| {
                // Box-Muller transform for normal distribution
                let u1: f32 = rng.random();
                let u2: f32 = rng.random();
                (-2.0 * u1.ln()).sqrt() * (2.0 * std::f32::consts::PI * u2).cos()
            })
            .collect();

        Array2::from_shape_vec((EMBEDDING_DIM, HIDDEN_DIM), data)
            .expect("Failed to create projection weights array")
    }

    /// Generate embeddings for a document
    pub fn embed_document(&mut self, text: &str) -> Result<Array2<f32>> {
        self.embed(text, DOC_MAXLEN)
    }

    /// Generate embeddings for a query
    pub fn embed_query(&mut self, text: &str) -> Result<Array2<f32>> {
        self.embed(text, QUERY_MAXLEN)
    }

    /// Generate embeddings for text with specified max length
    fn embed(&mut self, text: &str, max_len: usize) -> Result<Array2<f32>> {
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
            .context("ONNX inference failed")?;

        // Extract hidden states: [1, seq_len, 768]
        let hidden_states = outputs[0]
            .try_extract_tensor::<f32>()
            .context("Failed to extract output tensor")?;

        let (shape, data) = hidden_states;

        // Shape is [1, seq_len, 768] - get dimensions by iteration
        let dims: Vec<usize> = shape.iter().map(|&d| d as usize).collect();

        // Remove batch dimension: [seq_len, 768]
        let hidden_2d = Array2::from_shape_vec((dims[1], dims[2]), data.to_vec())
            .context("Failed to reshape hidden states")?;

        // Apply projection: [seq_len, 768] @ [768, 128].T -> [seq_len, 128]
        let embeddings = hidden_2d.dot(&self.projection_weights.t());

        // L2 normalize
        let normalized = l2_normalize_ndarray(&embeddings)?;

        Ok(normalized)
    }

    /// Tokenize text and return input_ids and attention_mask
    fn tokenize(&self, text: &str, max_len: usize) -> Result<(Vec<u32>, Vec<u32>)> {
        // Lowercase (XTR lowercases input)
        let text = text.to_lowercase();

        let encoding = self
            .tokenizer
            .encode(text.as_str(), true)
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
        EMBEDDING_DIM
    }
}

/// Implement EmbedderBackend for ONNX Embedder
impl crate::embedder::EmbedderBackend for OnnxEmbedder {
    fn embed_document(&mut self, text: &str) -> Result<crate::embedder::EmbeddingResult> {
        let array = self.embed(text, DOC_MAXLEN)?;
        let num_tokens = array.nrows();
        let data = embeddings_to_vec(&array);
        Ok(crate::embedder::EmbeddingResult::new(data, num_tokens))
    }

    fn embed_query(&mut self, text: &str) -> Result<crate::embedder::EmbeddingResult> {
        let array = self.embed(text, QUERY_MAXLEN)?;
        let num_tokens = array.nrows();
        let data = embeddings_to_vec(&array);
        Ok(crate::embedder::EmbeddingResult::new(data, num_tokens))
    }
}

/// Convert ndarray embeddings to flat f32 vec for storage
pub fn embeddings_to_vec(embeddings: &Array2<f32>) -> Vec<f32> {
    embeddings.iter().copied().collect()
}

/// Convert flat f32 vec back to ndarray embeddings
pub fn vec_to_embeddings(data: &[f32], num_tokens: usize) -> Result<Array2<f32>> {
    Array2::from_shape_vec((num_tokens, EMBEDDING_DIM), data.to_vec())
        .context("Failed to convert vec to embeddings")
}

/// Compute MaxSim score between query and document embeddings
pub fn maxsim_ndarray(query_emb: &Array2<f32>, doc_emb: &Array2<f32>) -> f32 {
    // query_emb: [q_tokens, dim]
    // doc_emb: [d_tokens, dim]

    let q_tokens = query_emb.nrows();

    // For each query token, find max similarity with any doc token
    let mut total_score = 0.0f32;

    for q_idx in 0..q_tokens {
        let q_row = query_emb.row(q_idx);
        let mut max_sim = f32::NEG_INFINITY;

        for d_idx in 0..doc_emb.nrows() {
            let d_row = doc_emb.row(d_idx);
            let sim: f32 = q_row.iter().zip(d_row.iter()).map(|(a, b)| a * b).sum();
            if sim > max_sim {
                max_sim = sim;
            }
        }

        total_score += max_sim;
    }

    total_score / q_tokens as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_projection_weights_shape() {
        let weights = OnnxEmbedder::init_projection_weights();
        assert_eq!(weights.shape(), &[EMBEDDING_DIM, HIDDEN_DIM]);
    }

    #[test]
    fn test_projection_weights_deterministic() {
        let w1 = OnnxEmbedder::init_projection_weights();
        let w2 = OnnxEmbedder::init_projection_weights();
        assert_eq!(
            w1, w2,
            "Projection weights should be deterministic with fixed seed"
        );
    }

    #[test]
    fn test_maxsim_ndarray() {
        let query =
            Array2::from_shape_vec((2, 4), vec![1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0]).unwrap();

        let doc = Array2::from_shape_vec(
            (3, 4),
            vec![
                1.0, 0.0, 0.0, 0.0, // matches query token 0
                0.0, 0.5, 0.5, 0.0, // partial match to query token 1
                0.0, 0.0, 0.0, 1.0, // no match
            ],
        )
        .unwrap();

        let score = maxsim_ndarray(&query, &doc);
        // Query token 0: max sim = 1.0
        // Query token 1: max sim = 0.5
        // Average: 0.75
        assert!((score - 0.75).abs() < 1e-5);
    }

    #[test]
    fn test_embeddings_conversion() {
        let data: Vec<f32> = (0..256).map(|i| i as f32).collect();
        let embeddings = vec_to_embeddings(&data, 2).unwrap();
        assert_eq!(embeddings.shape(), &[2, EMBEDDING_DIM]);

        let back = embeddings_to_vec(&embeddings);
        assert_eq!(data, back);
    }

    /// Compile-time test: verify OnnxEmbedder implements EmbedderBackend
    ///
    /// This test doesn't run (OnnxEmbedder::new() requires model files),
    /// but it verifies at compile time that the trait is correctly implemented.
    #[allow(dead_code)]
    fn verify_embedder_backend_impl() {
        fn requires_backend<E: crate::embedder::EmbedderBackend>(_: &mut E) {}

        // This line will fail to compile if OnnxEmbedder doesn't implement EmbedderBackend
        fn _test_onnx_embedder(embedder: &mut OnnxEmbedder) {
            requires_backend(embedder);
        }
    }
}
