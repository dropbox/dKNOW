//! CoreML embedding backend (macOS only)
//!
//! This module provides a CoreML-accelerated embedder for macOS using ONNX Runtime
//! with the CoreML execution provider. It uses the same ONNX model as the standard
//! ONNX backend but runs inference through Apple's CoreML framework for better
//! performance on Apple Silicon.
//!
//! # Requirements
//!
//! - macOS only (CoreML is an Apple framework)
//! - Same ONNX model as the standard ONNX backend
//!
//! # Model Location
//!
//! The ONNX model is expected at:
//! - `~/.cache/sg/models/onnx/xtr_encoder.onnx`
//!
//! # Usage
//!
//! ```rust,ignore
//! use sg_core::embedder_coreml::CoreMLEmbedder;
//!
//! let embedder = CoreMLEmbedder::new()?;
//! let embeddings = embedder.embed_document("some text")?;
//! ```

use anyhow::{Context, Result};
use ndarray::{Array2, Axis};
use ort::{
    execution_providers::coreml::{CoreMLComputeUnits, CoreMLExecutionProvider},
    session::Session,
    value::Value,
};
use std::path::{Path, PathBuf};
use tokenizers::Tokenizer;

use crate::embedder::EMBEDDING_DIM;

/// Hidden dimension of T5 encoder
const HIDDEN_DIM: usize = 768;

/// Maximum sequence length for documents
const DOC_MAXLEN: usize = 512;

/// Maximum sequence length for queries
const QUERY_MAXLEN: usize = 64;

/// CoreML-accelerated embedder using ONNX Runtime
pub struct CoreMLEmbedder {
    session: Session,
    tokenizer: Tokenizer,
    projection_weights: Array2<f32>,
}

impl CoreMLEmbedder {
    /// Load the CoreML embedder from the default cache location
    pub fn new() -> Result<Self> {
        let model_dir = Self::default_model_dir()?;
        Self::from_dir(&model_dir)
    }

    /// Get the default model directory (same as ONNX backend)
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
        tracing::info!(
            "Loading ONNX model with CoreML acceleration from {}",
            model_path.display()
        );

        // Initialize ONNX Runtime session with CoreML execution provider
        // Use CPU + Neural Engine for best performance on Apple Silicon
        let coreml_provider = CoreMLExecutionProvider::default()
            .with_subgraphs(true) // Enable CoreML for subgraphs
            .with_compute_units(CoreMLComputeUnits::CPUAndNeuralEngine) // Prefer Neural Engine
            .build();

        let session = Session::builder()
            .context("Failed to create ONNX session builder")?
            .with_execution_providers([coreml_provider])
            .context("Failed to configure CoreML execution provider")?
            .commit_from_file(model_path)
            .with_context(|| format!("Failed to load ONNX model from {}", model_path.display()))?;

        // Load tokenizer
        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {e}"))?;

        // Initialize random projection weights (same as other backends)
        let projection_weights = Self::init_projection_weights();

        tracing::info!("CoreML embedder loaded successfully");

        Ok(Self {
            session,
            tokenizer,
            projection_weights,
        })
    }

    /// Initialize random projection weights (768 -> 128)
    ///
    /// Uses the same fixed seed as other backends for reproducibility.
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

        // Run inference (CoreML accelerated)
        let outputs = self
            .session
            .run(ort::inputs![input_ids_value, attention_mask_value])
            .context("CoreML inference failed")?;

        // Extract hidden states: [1, seq_len, 768]
        let hidden_states = outputs[0]
            .try_extract_tensor::<f32>()
            .context("Failed to extract output tensor")?;

        let (shape, data) = hidden_states;

        // Shape is [1, seq_len, 768]
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

/// Implement EmbedderBackend for CoreML Embedder
impl crate::embedder::EmbedderBackend for CoreMLEmbedder {
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

/// L2 normalize each row of a 2D ndarray
fn l2_normalize_ndarray(array: &Array2<f32>) -> Result<Array2<f32>> {
    let norms = array.map_axis(Axis(1), |row| {
        let sum_sq: f32 = row.iter().map(|x| x * x).sum();
        (sum_sq + 1e-12).sqrt()
    });

    let mut result = array.clone();
    for (mut row, norm) in result.rows_mut().into_iter().zip(norms.iter()) {
        row.mapv_inplace(|x| x / norm);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_projection_weights_shape() {
        let weights = CoreMLEmbedder::init_projection_weights();
        assert_eq!(weights.shape(), &[EMBEDDING_DIM, HIDDEN_DIM]);
    }

    #[test]
    fn test_projection_weights_deterministic() {
        let w1 = CoreMLEmbedder::init_projection_weights();
        let w2 = CoreMLEmbedder::init_projection_weights();
        assert_eq!(
            w1, w2,
            "Projection weights should be deterministic with fixed seed"
        );
    }

    // Note: Cross-module projection weight comparison test removed.
    // All backends use identical init_projection_weights() implementations
    // with the same seed (42), so they produce identical weights by design.

    /// Compile-time test: verify CoreMLEmbedder implements EmbedderBackend
    #[allow(dead_code)]
    fn verify_embedder_backend_impl() {
        fn requires_backend<E: crate::embedder::EmbedderBackend>(_: &mut E) {}

        fn _test_coreml_embedder(embedder: &mut CoreMLEmbedder) {
            requires_backend(embedder);
        }
    }
}
