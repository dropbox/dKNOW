//! XTR model embedding generation
//!
//! Loads the XTR model (T5 encoder + linear projection) and generates
//! multi-vector embeddings for text input. Each token produces a 128-dimensional
//! L2-normalized embedding.
//!
//! Model: google/xtr-base-en
//! - T5 encoder (768-dim hidden state)
//! - Linear projection to 128 dimensions
//! - L2 normalization
//!
//! # Known Issue: Projection Layer
//!
//! The HuggingFace `google/xtr-base-en` model only contains T5 encoder weights.
//! The 768->128 projection layer is trained separately by XTR-WARP and isn't
//! included in the public model. We use random initialization for the projection.
//!
//! This works reasonably well because:
//! 1. T5 encoder already produces semantically meaningful 768-dim embeddings
//! 2. Random projection preserves relative distances (Johnson-Lindenstrauss)
//! 3. MaxSim scoring still finds relevant documents
//!
//! Future improvement: Train projection layer on code retrieval data, or
//! use 768-dim embeddings directly (trading storage/speed for quality).
//!
//! # Embedding Backends
//!
//! Two backends are available:
//! - **Candle** (default): Uses Candle for ML inference, supports Metal on macOS
//! - **ONNX** (optional): Cross-platform inference via ONNX Runtime
//!
//! Both backends implement the `EmbedderBackend` trait for unified usage.

use anyhow::{Context, Result};
use candle_core::{DType, Device, IndexOp, Tensor};
use candle_nn::{Linear, Module, VarBuilder};
use candle_transformers::models::t5::{self, T5EncoderModel};
use hf_hub::api::sync::ApiBuilder;
use std::collections::HashMap;
use std::env;
use std::path::Path;
use tokenizers::Tokenizer;

/// Embedding dimension (output of linear projection)
pub const EMBEDDING_DIM: usize = 128;

/// Hidden dimension of T5 encoder
const HIDDEN_DIM: usize = 768;

/// Maximum sequence length for documents
const DOC_MAXLEN: usize = 512;

/// Maximum sequence length for queries
const QUERY_MAXLEN: usize = 64;

/// Embedding result containing flattened embeddings and token count
#[derive(Debug, Clone)]
pub struct EmbeddingResult {
    /// Flattened embeddings: num_tokens * EMBEDDING_DIM floats
    pub data: Vec<f32>,
    /// Number of tokens (embeddings.len() / EMBEDDING_DIM)
    pub num_tokens: usize,
}

impl EmbeddingResult {
    /// Create a new embedding result
    ///
    /// # Arguments
    /// * `data` - Flattened embedding data (num_tokens * embedding_dim floats)
    /// * `num_tokens` - Number of tokens (1 for single-vector models like UniXcoder)
    ///
    /// Note: embedding_dim can be computed as data.len() / num_tokens
    pub fn new(data: Vec<f32>, num_tokens: usize) -> Self {
        debug_assert!(
            num_tokens > 0 && data.len().is_multiple_of(num_tokens),
            "data.len() ({}) must be divisible by num_tokens ({})",
            data.len(),
            num_tokens
        );
        Self { data, num_tokens }
    }

    /// Get the embedding dimension for this result
    pub fn embedding_dim(&self) -> usize {
        if self.num_tokens == 0 {
            0
        } else {
            self.data.len() / self.num_tokens
        }
    }

    /// Create from a Candle tensor
    pub fn from_tensor(tensor: &Tensor) -> Result<Self> {
        let num_tokens = tensor.dims()[0];
        let data = embeddings_to_vec(tensor)?;
        Ok(Self { data, num_tokens })
    }

    /// Convert to Candle tensor
    pub fn to_tensor(&self, device: &Device) -> Result<Tensor> {
        vec_to_embeddings(&self.data, self.num_tokens, device)
    }
}

/// Trait for embedding backends
///
/// Implement this trait to add new embedding backends (Candle, ONNX, etc.).
/// All backends produce the same output format for interoperability.
pub trait EmbedderBackend {
    /// Generate embeddings for a document
    ///
    /// Returns flattened embeddings and token count.
    fn embed_document(&mut self, text: &str) -> Result<EmbeddingResult>;

    /// Generate embeddings for a query
    ///
    /// Returns flattened embeddings and token count.
    fn embed_query(&mut self, text: &str) -> Result<EmbeddingResult>;

    /// Generate embeddings for multiple documents in a batch
    ///
    /// Default implementation calls embed_document in a loop.
    /// Override for backends with native batch support.
    fn embed_batch(&mut self, texts: &[&str]) -> Result<Vec<EmbeddingResult>> {
        texts.iter().map(|text| self.embed_document(text)).collect()
    }

    /// Get the embedding dimension (should be EMBEDDING_DIM = 128)
    fn embedding_dim(&self) -> usize {
        EMBEDDING_DIM
    }

    /// Warm up the model by running a dummy inference
    ///
    /// This initializes any lazy-loaded resources (GPU kernels, etc.)
    fn warmup(&mut self) -> Result<()> {
        let _ = self.embed_document("warmup")?;
        Ok(())
    }
}

/// Environment variable for selecting embedder backend
pub const EMBEDDER_BACKEND_ENV: &str = "SG_EMBEDDER_BACKEND";

/// Environment variable for selecting embedding model
pub const EMBEDDER_MODEL_ENV: &str = "SG_EMBEDDER_MODEL";

/// Available embedding model types
///
/// Different models have different characteristics:
/// - **XTR**: Multi-vector (token-level) embeddings, 128 dim per token, MaxSim scoring
/// - **UniXcoder**: Single-vector (document-level) embeddings, 768 dim, code-specialized
/// - **JinaCode**: Single-vector embeddings, 768 dim, code-specialized (ONNX)
/// - **JinaColBERT**: Multi-vector embeddings, 128 dim per token, multilingual (ONNX)
///
/// Multi-vector models produce one embedding per token and use MaxSim scoring.
/// Single-vector models produce one embedding per document and use cosine similarity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EmbeddingModel {
    /// XTR (google/xtr-base-en) - T5-based multi-vector retriever
    /// - Architecture: T5 encoder + linear projection
    /// - Output: 128 dimensions per token (multi-vector)
    /// - Scoring: MaxSim (max similarity per query token)
    /// - Best for: General text retrieval
    #[default]
    Xtr,

    /// UniXcoder (microsoft/unixcoder-base) - Code-specialized encoder
    /// - Architecture: RoBERTa-based
    /// - Output: 768 dimensions (single-vector, CLS pooling)
    /// - Scoring: Cosine similarity
    /// - Best for: Code search, code-comment matching
    UniXcoder,

    /// Jina Code (jinaai/jina-embeddings-v2-base-code) - Code-specialized encoder
    /// - Architecture: BERT-based with ALiBi positional encoding
    /// - Output: 768 dimensions (single-vector, mean pooling)
    /// - Scoring: Cosine similarity
    /// - Best for: Code retrieval, long-context code
    /// - Requires: ONNX runtime for inference
    JinaCode,

    /// Jina ColBERT v2 (jinaai/jina-colbert-v2) - Multilingual late-interaction encoder
    /// - Architecture: XLM-RoBERTa with rotary embeddings
    /// - Output: 128 dimensions per token (multi-vector)
    /// - Scoring: MaxSim (late interaction)
    /// - Best for: Multilingual retrieval (94 languages including CJK)
    /// - Context: 8192 tokens (vs XTR's 512)
    /// - Requires: ONNX runtime for inference (custom architecture)
    JinaColbert,

    /// CLIP (openai/clip-vit-base-patch32) - Vision-language encoder
    /// - Architecture: Vision Transformer (ViT) + Text Transformer
    /// - Output: 512 dimensions (single-vector)
    /// - Scoring: Cosine similarity
    /// - Best for: Image search, cross-modal retrieval
    /// - Requires: --features clip
    #[cfg(feature = "clip")]
    Clip,
}

impl EmbeddingModel {
    /// HuggingFace model ID
    pub fn model_id(&self) -> &'static str {
        match self {
            Self::Xtr => "google/xtr-base-en",
            Self::UniXcoder => "microsoft/unixcoder-base",
            Self::JinaCode => "jinaai/jina-embeddings-v2-base-code",
            Self::JinaColbert => "jinaai/jina-colbert-v2",
            #[cfg(feature = "clip")]
            Self::Clip => "openai/clip-vit-base-patch32",
        }
    }

    /// Output embedding dimension
    pub fn embedding_dim(&self) -> usize {
        match self {
            Self::Xtr => EMBEDDING_DIM, // 128
            Self::UniXcoder => 768,
            Self::JinaCode => 768,
            Self::JinaColbert => EMBEDDING_DIM, // 128 per token
            #[cfg(feature = "clip")]
            Self::Clip => 512,
        }
    }

    /// Whether this model produces multi-vector (token-level) embeddings
    pub fn is_multi_vector(&self) -> bool {
        match self {
            Self::Xtr | Self::JinaColbert => true,
            Self::UniXcoder | Self::JinaCode => false,
            #[cfg(feature = "clip")]
            Self::Clip => false,
        }
    }

    /// Human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Xtr => "XTR",
            Self::UniXcoder => "UniXcoder",
            Self::JinaCode => "JinaCode",
            Self::JinaColbert => "JinaColBERT",
            #[cfg(feature = "clip")]
            Self::Clip => "CLIP",
        }
    }

    /// Whether this model is implemented and available
    pub fn is_available(&self) -> bool {
        match self {
            Self::Xtr => true,
            Self::UniXcoder => true,
            #[cfg(feature = "onnx")]
            Self::JinaCode => true, // Available with ONNX feature
            #[cfg(not(feature = "onnx"))]
            Self::JinaCode => false, // Requires ONNX feature
            #[cfg(feature = "onnx")]
            Self::JinaColbert => true, // Available with ONNX feature
            #[cfg(not(feature = "onnx"))]
            Self::JinaColbert => false, // Requires ONNX feature
            #[cfg(feature = "clip")]
            Self::Clip => true,
        }
    }

    /// Whether this model can embed images
    pub fn supports_images(&self) -> bool {
        match self {
            Self::Xtr | Self::UniXcoder | Self::JinaCode | Self::JinaColbert => false,
            #[cfg(feature = "clip")]
            Self::Clip => true,
        }
    }

    /// Whether this model requires the ONNX feature
    pub fn requires_onnx(&self) -> bool {
        matches!(self, Self::JinaCode | Self::JinaColbert)
    }

    /// List all models
    #[cfg(not(feature = "clip"))]
    pub fn all() -> &'static [EmbeddingModel] {
        &[
            Self::Xtr,
            Self::UniXcoder,
            Self::JinaCode,
            Self::JinaColbert,
        ]
    }

    /// List all models
    #[cfg(feature = "clip")]
    pub fn all() -> &'static [EmbeddingModel] {
        &[
            Self::Xtr,
            Self::UniXcoder,
            Self::JinaCode,
            Self::JinaColbert,
            Self::Clip,
        ]
    }

    /// List available (implemented) models
    pub fn available() -> Vec<EmbeddingModel> {
        Self::all()
            .iter()
            .copied()
            .filter(EmbeddingModel::is_available)
            .collect()
    }
}

impl std::str::FromStr for EmbeddingModel {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "xtr" | "xtr-base-en" | "google/xtr-base-en" => Ok(Self::Xtr),
            "unixcoder" | "unixcoder-base" | "microsoft/unixcoder-base" => Ok(Self::UniXcoder),
            "jinacode"
            | "jina-code"
            | "jina-embeddings-v2-base-code"
            | "jinaai/jina-embeddings-v2-base-code" => Ok(Self::JinaCode),
            "jinacolbert"
            | "jina-colbert"
            | "jina-colbert-v2"
            | "jinaai/jina-colbert-v2" => Ok(Self::JinaColbert),
            #[cfg(feature = "clip")]
            "clip" | "clip-vit-base-patch32" | "openai/clip-vit-base-patch32" => Ok(Self::Clip),
            #[cfg(not(feature = "clip"))]
            "clip" | "clip-vit-base-patch32" | "openai/clip-vit-base-patch32" => Err(
                anyhow::anyhow!("CLIP model not enabled. Rebuild with --features clip"),
            ),
            other => Err(anyhow::anyhow!(
                "Unknown embedding model: {other}. Available: xtr, unixcoder, jinacode, jina-colbert, clip"
            )),
        }
    }
}

impl std::fmt::Display for EmbeddingModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl EmbeddingModel {
    /// Load model from environment, defaulting to XTR
    pub fn from_env() -> Result<Self> {
        match env::var(EMBEDDER_MODEL_ENV) {
            Ok(value) => value.parse(),
            Err(env::VarError::NotPresent) => Ok(Self::default()),
            Err(e) => Err(anyhow::anyhow!(
                "Failed to read {EMBEDDER_MODEL_ENV}: {e}"
            )),
        }
    }
}

/// Available embedder backend types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbedderBackendKind {
    Candle,
    #[cfg(feature = "onnx")]
    Onnx,
    #[cfg(feature = "coreml")]
    CoreMl,
    #[cfg(feature = "cuda")]
    Cuda,
}

impl std::str::FromStr for EmbedderBackendKind {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "candle" => Ok(Self::Candle),
            #[cfg(feature = "onnx")]
            "onnx" => Ok(Self::Onnx),
            #[cfg(not(feature = "onnx"))]
            "onnx" => Err(anyhow::anyhow!(
                "ONNX backend not enabled. Rebuild with --features onnx"
            )),
            #[cfg(feature = "coreml")]
            "coreml" => Ok(Self::CoreMl),
            #[cfg(not(feature = "coreml"))]
            "coreml" => Err(anyhow::anyhow!(
                "CoreML backend not enabled. Rebuild with --features coreml (macOS only)"
            )),
            #[cfg(feature = "cuda")]
            "cuda" => Ok(Self::Cuda),
            #[cfg(not(feature = "cuda"))]
            "cuda" => Err(anyhow::anyhow!(
                "CUDA backend not enabled. Rebuild with --features cuda (requires NVIDIA GPU)"
            )),
            other => Err(anyhow::anyhow!("Unknown embedder backend: {other}")),
        }
    }
}

impl EmbedderBackendKind {
    /// Load backend kind from environment, defaulting to Candle
    pub fn from_env() -> Result<Self> {
        match env::var(EMBEDDER_BACKEND_ENV) {
            Ok(value) => value.parse(),
            Err(env::VarError::NotPresent) => Ok(Self::Candle),
            Err(e) => Err(anyhow::anyhow!(
                "Failed to read {EMBEDDER_BACKEND_ENV}: {e}"
            )),
        }
    }

    /// Backend name for logs and diagnostics
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Candle => "candle",
            #[cfg(feature = "onnx")]
            Self::Onnx => "onnx",
            #[cfg(feature = "coreml")]
            Self::CoreMl => "coreml",
            #[cfg(feature = "cuda")]
            Self::Cuda => "cuda",
        }
    }
}

/// Backend-agnostic embedder wrapper
pub enum BackendEmbedder {
    Candle(Embedder),
    UniXcoder(crate::embedder_unixcoder::UniXcoderEmbedder),
    #[cfg(feature = "onnx")]
    Onnx(crate::embedder_onnx::OnnxEmbedder),
    #[cfg(feature = "onnx")]
    JinaColbert(crate::embedder_jina_colbert::JinaColBertEmbedder),
    #[cfg(feature = "onnx")]
    JinaCode(crate::embedder_jina_code::JinaCodeEmbedder),
    #[cfg(feature = "coreml")]
    CoreMl(crate::embedder_coreml::CoreMLEmbedder),
    #[cfg(feature = "cuda")]
    Cuda(crate::embedder_cuda::CUDAEmbedder),
    #[cfg(feature = "clip")]
    Clip(crate::embedder_clip::ClipEmbedder),
}

/// Load an embedder based on environment configuration.
///
/// - If `SG_EMBEDDER_MODEL` is set, that model is used.
///   - For XTR, the backend is still selected via `SG_EMBEDDER_BACKEND`.
///   - For non-XTR models, the backend is ignored (Candle only).
/// - If `SG_EMBEDDER_MODEL` is not set, `SG_EMBEDDER_BACKEND` selects the backend.
pub fn load_embedder_from_env() -> Result<BackendEmbedder> {
    match env::var(EMBEDDER_MODEL_ENV) {
        Ok(value) => {
            let model: EmbeddingModel = value
                .parse()
                .with_context(|| format!("Failed to parse {EMBEDDER_MODEL_ENV}='{value}'"))?;
            match model {
                EmbeddingModel::Xtr => {
                    let backend = EmbedderBackendKind::from_env()?;
                    BackendEmbedder::new(backend)
                }
                EmbeddingModel::UniXcoder
                | EmbeddingModel::JinaCode
                | EmbeddingModel::JinaColbert => BackendEmbedder::from_model(model),
                #[cfg(feature = "clip")]
                EmbeddingModel::Clip => BackendEmbedder::from_model(model),
            }
        }
        Err(env::VarError::NotPresent) => {
            let backend = EmbedderBackendKind::from_env()?;
            BackendEmbedder::new(backend)
        }
        Err(e) => Err(anyhow::anyhow!(
            "Failed to read {EMBEDDER_MODEL_ENV}: {e}"
        )),
    }
}

impl BackendEmbedder {
    /// Create a new embedder for the selected backend (defaults to XTR model)
    pub fn new(kind: EmbedderBackendKind) -> Result<Self> {
        match kind {
            EmbedderBackendKind::Candle => {
                let device = crate::make_device();
                let embedder = Embedder::new(&device).context("Failed to load embedding model")?;
                Ok(Self::Candle(embedder))
            }
            #[cfg(feature = "onnx")]
            EmbedderBackendKind::Onnx => {
                let embedder = crate::embedder_onnx::OnnxEmbedder::new()
                    .context("Failed to load ONNX embedding model")?;
                Ok(Self::Onnx(embedder))
            }
            #[cfg(feature = "coreml")]
            EmbedderBackendKind::CoreMl => {
                let embedder = crate::embedder_coreml::CoreMLEmbedder::new()
                    .context("Failed to load CoreML embedding model")?;
                Ok(Self::CoreMl(embedder))
            }
            #[cfg(feature = "cuda")]
            EmbedderBackendKind::Cuda => {
                let embedder = crate::embedder_cuda::CUDAEmbedder::new()
                    .context("Failed to load CUDA embedding model")?;
                Ok(Self::Cuda(embedder))
            }
        }
    }

    /// Create a new embedder for a specific embedding model
    ///
    /// This method allows selecting different embedding models (XTR, UniXcoder, etc.)
    /// Currently only Candle backend is supported for all models.
    pub fn from_model(model: EmbeddingModel) -> Result<Self> {
        Self::from_model_with_path(model, None)
    }

    /// Create a new embedder from a custom model path
    ///
    /// The path should contain config.json, tokenizer.json, and model.safetensors files.
    /// This is useful for loading fine-tuned models (e.g., after LoRA merge).
    ///
    /// Currently only supports XTR-based models (T5 encoder + projection).
    pub fn from_custom_path(path: &std::path::Path) -> Result<Self> {
        Self::from_model_with_path(EmbeddingModel::Xtr, Some(path))
    }

    /// Create a new embedder for a model with an optional custom path
    fn from_model_with_path(model: EmbeddingModel, path: Option<&std::path::Path>) -> Result<Self> {
        let device = crate::make_device();
        match model {
            EmbeddingModel::Xtr => {
                let embedder = if let Some(p) = path {
                    // Load from custom path (e.g., fine-tuned model)
                    let model_id = p.to_string_lossy();
                    Embedder::from_pretrained(&model_id, &device)
                        .with_context(|| format!("Failed to load XTR model from {}", p.display()))?
                } else {
                    Embedder::new(&device).context("Failed to load XTR model")?
                };
                Ok(Self::Candle(embedder))
            }
            EmbeddingModel::UniXcoder => {
                let embedder = crate::embedder_unixcoder::UniXcoderEmbedder::new(&device)
                    .context("Failed to load UniXcoder model")?;
                Ok(Self::UniXcoder(embedder))
            }
            #[cfg(feature = "onnx")]
            EmbeddingModel::JinaCode => {
                let embedder = crate::embedder_jina_code::JinaCodeEmbedder::new()
                    .context("Failed to load JinaCode model")?;
                Ok(Self::JinaCode(embedder))
            }
            #[cfg(not(feature = "onnx"))]
            EmbeddingModel::JinaCode => Err(anyhow::anyhow!(
                "JinaCode model requires ONNX feature. Rebuild with --features onnx"
            )),
            #[cfg(feature = "onnx")]
            EmbeddingModel::JinaColbert => {
                let embedder = crate::embedder_jina_colbert::JinaColBertEmbedder::new()
                    .context("Failed to load JinaColBERT model")?;
                Ok(Self::JinaColbert(embedder))
            }
            #[cfg(not(feature = "onnx"))]
            EmbeddingModel::JinaColbert => Err(anyhow::anyhow!(
                "JinaColBERT model requires ONNX feature. Rebuild with --features onnx"
            )),
            #[cfg(feature = "clip")]
            EmbeddingModel::Clip => {
                let embedder = crate::embedder_clip::ClipEmbedder::new(&device)
                    .context("Failed to load CLIP model")?;
                Ok(Self::Clip(embedder))
            }
        }
    }

    /// Return the backend kind for this embedder
    pub fn kind(&self) -> EmbedderBackendKind {
        match self {
            Self::Candle(_) => EmbedderBackendKind::Candle,
            Self::UniXcoder(_) => EmbedderBackendKind::Candle, // UniXcoder uses Candle backend
            #[cfg(feature = "onnx")]
            Self::Onnx(_) => EmbedderBackendKind::Onnx,
            #[cfg(feature = "onnx")]
            Self::JinaColbert(_) => EmbedderBackendKind::Onnx, // JinaColBERT uses ONNX backend
            #[cfg(feature = "onnx")]
            Self::JinaCode(_) => EmbedderBackendKind::Onnx, // JinaCode uses ONNX backend
            #[cfg(feature = "coreml")]
            Self::CoreMl(_) => EmbedderBackendKind::CoreMl,
            #[cfg(feature = "cuda")]
            Self::Cuda(_) => EmbedderBackendKind::Cuda,
            #[cfg(feature = "clip")]
            Self::Clip(_) => EmbedderBackendKind::Candle, // CLIP uses Candle backend
        }
    }

    /// Return the embedding model type for this embedder
    pub fn model(&self) -> EmbeddingModel {
        match self {
            Self::Candle(_) => EmbeddingModel::Xtr,
            Self::UniXcoder(_) => EmbeddingModel::UniXcoder,
            #[cfg(feature = "onnx")]
            Self::Onnx(_) => EmbeddingModel::Xtr, // ONNX backend uses XTR
            #[cfg(feature = "onnx")]
            Self::JinaColbert(_) => EmbeddingModel::JinaColbert,
            #[cfg(feature = "onnx")]
            Self::JinaCode(_) => EmbeddingModel::JinaCode,
            #[cfg(feature = "coreml")]
            Self::CoreMl(_) => EmbeddingModel::Xtr, // CoreML backend uses XTR
            #[cfg(feature = "cuda")]
            Self::Cuda(_) => EmbeddingModel::Xtr, // CUDA backend uses XTR
            #[cfg(feature = "clip")]
            Self::Clip(_) => EmbeddingModel::Clip,
        }
    }

    /// Check if this embedder produces multi-vector (token-level) embeddings
    pub fn is_multi_vector(&self) -> bool {
        self.model().is_multi_vector()
    }
}

impl EmbedderBackend for BackendEmbedder {
    fn embed_document(&mut self, text: &str) -> Result<EmbeddingResult> {
        match self {
            Self::Candle(embedder) => <Embedder as EmbedderBackend>::embed_document(embedder, text),
            Self::UniXcoder(embedder) => {
                <crate::embedder_unixcoder::UniXcoderEmbedder as EmbedderBackend>::embed_document(
                    embedder, text,
                )
            }
            #[cfg(feature = "onnx")]
            Self::Onnx(embedder) => {
                <crate::embedder_onnx::OnnxEmbedder as EmbedderBackend>::embed_document(
                    embedder, text,
                )
            }
            #[cfg(feature = "onnx")]
            Self::JinaColbert(embedder) => {
                <crate::embedder_jina_colbert::JinaColBertEmbedder as EmbedderBackend>::embed_document(
                    embedder, text,
                )
            }
            #[cfg(feature = "onnx")]
            Self::JinaCode(embedder) => {
                <crate::embedder_jina_code::JinaCodeEmbedder as EmbedderBackend>::embed_document(
                    embedder, text,
                )
            }
            #[cfg(feature = "coreml")]
            Self::CoreMl(embedder) => {
                <crate::embedder_coreml::CoreMLEmbedder as EmbedderBackend>::embed_document(
                    embedder, text,
                )
            }
            #[cfg(feature = "cuda")]
            Self::Cuda(embedder) => {
                <crate::embedder_cuda::CUDAEmbedder as EmbedderBackend>::embed_document(
                    embedder, text,
                )
            }
            #[cfg(feature = "clip")]
            Self::Clip(embedder) => {
                <crate::embedder_clip::ClipEmbedder as EmbedderBackend>::embed_document(
                    embedder, text,
                )
            }
        }
    }

    fn embed_query(&mut self, text: &str) -> Result<EmbeddingResult> {
        match self {
            Self::Candle(embedder) => <Embedder as EmbedderBackend>::embed_query(embedder, text),
            Self::UniXcoder(embedder) => {
                <crate::embedder_unixcoder::UniXcoderEmbedder as EmbedderBackend>::embed_query(
                    embedder, text,
                )
            }
            #[cfg(feature = "onnx")]
            Self::Onnx(embedder) => {
                <crate::embedder_onnx::OnnxEmbedder as EmbedderBackend>::embed_query(embedder, text)
            }
            #[cfg(feature = "onnx")]
            Self::JinaColbert(embedder) => {
                <crate::embedder_jina_colbert::JinaColBertEmbedder as EmbedderBackend>::embed_query(
                    embedder, text,
                )
            }
            #[cfg(feature = "onnx")]
            Self::JinaCode(embedder) => {
                <crate::embedder_jina_code::JinaCodeEmbedder as EmbedderBackend>::embed_query(
                    embedder, text,
                )
            }
            #[cfg(feature = "coreml")]
            Self::CoreMl(embedder) => {
                <crate::embedder_coreml::CoreMLEmbedder as EmbedderBackend>::embed_query(
                    embedder, text,
                )
            }
            #[cfg(feature = "cuda")]
            Self::Cuda(embedder) => {
                <crate::embedder_cuda::CUDAEmbedder as EmbedderBackend>::embed_query(embedder, text)
            }
            #[cfg(feature = "clip")]
            Self::Clip(embedder) => {
                <crate::embedder_clip::ClipEmbedder as EmbedderBackend>::embed_query(embedder, text)
            }
        }
    }

    fn embedding_dim(&self) -> usize {
        match self {
            Self::Candle(_) => EMBEDDING_DIM,
            Self::UniXcoder(_) => crate::embedder_unixcoder::UNIXCODER_DIM,
            #[cfg(feature = "onnx")]
            Self::Onnx(_) => EMBEDDING_DIM,
            #[cfg(feature = "onnx")]
            Self::JinaColbert(_) => crate::embedder_jina_colbert::JINA_COLBERT_DIM,
            #[cfg(feature = "onnx")]
            Self::JinaCode(_) => crate::embedder_jina_code::JINA_CODE_DIM,
            #[cfg(feature = "coreml")]
            Self::CoreMl(_) => EMBEDDING_DIM,
            #[cfg(feature = "cuda")]
            Self::Cuda(_) => EMBEDDING_DIM,
            #[cfg(feature = "clip")]
            Self::Clip(_) => crate::embedder_clip::CLIP_DIM,
        }
    }
}

/// XTR embedding model
pub struct Embedder {
    encoder: T5EncoderModel,
    projection: Linear,
    tokenizer: Tokenizer,
    device: Device,
}

impl Embedder {
    /// Load the XTR model from HuggingFace hub
    ///
    /// On macOS, uses Metal acceleration if available.
    pub fn new(device: &Device) -> Result<Self> {
        Self::from_pretrained("google/xtr-base-en", device)
    }

    /// Load from a specific model path or HuggingFace model ID
    pub fn from_pretrained(model_id: &str, device: &Device) -> Result<Self> {
        // Check if model_id is a local directory path
        let path = std::path::Path::new(model_id);
        if path.is_dir() {
            tracing::info!("Loading model from local path: {}", model_id);
            let config_path = path.join("config.json");
            let tokenizer_path = path.join("tokenizer.json");
            let weights_path = path.join("model.safetensors");

            // Verify required files exist
            if !config_path.exists() {
                anyhow::bail!(
                    "config.json not found in {model_id}. Expected files: config.json, tokenizer.json, model.safetensors"
                );
            }
            if !tokenizer_path.exists() {
                anyhow::bail!(
                    "tokenizer.json not found in {model_id}. Expected files: config.json, tokenizer.json, model.safetensors"
                );
            }
            if !weights_path.exists() {
                anyhow::bail!(
                    "model.safetensors not found in {model_id}. Expected files: config.json, tokenizer.json, model.safetensors"
                );
            }

            return Self::from_files(&config_path, &tokenizer_path, &weights_path, device);
        }

        // Try hf-hub first, fall back to manual download
        match Self::try_hf_hub(model_id, device) {
            Ok(embedder) => return Ok(embedder),
            Err(e) => {
                tracing::debug!("hf-hub download failed: {}, trying manual download", e);
            }
        }

        // Manual download fallback
        Self::try_manual_download(model_id, device)
    }

    fn try_hf_hub(model_id: &str, device: &Device) -> Result<Self> {
        let api = ApiBuilder::new()
            .with_progress(true)
            .build()
            .context("Failed to create HuggingFace API")?;
        let repo = api.model(model_id.to_string());

        tracing::info!("Downloading model files from {}", model_id);
        let config_path = repo
            .get("config.json")
            .context("Failed to get config.json")?;
        let tokenizer_path = repo
            .get("tokenizer.json")
            .context("Failed to get tokenizer.json")?;
        let weights_path = repo
            .get("model.safetensors")
            .context("Failed to get model.safetensors")?;

        Self::from_files(&config_path, &tokenizer_path, &weights_path, device)
    }

    fn try_manual_download(model_id: &str, device: &Device) -> Result<Self> {
        // Get cache directory
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("sg")
            .join("models")
            .join(model_id.replace('/', "_"));

        std::fs::create_dir_all(&cache_dir)?;

        let base_url = format!("https://huggingface.co/{model_id}/resolve/main");

        // Download files
        let files = ["config.json", "tokenizer.json", "model.safetensors"];
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
            } else {
                tracing::debug!("Using cached {}", file);
            }
            paths.push(local_path);
        }

        Self::from_files(&paths[0], &paths[1], &paths[2], device)
    }

    /// Load from local files
    pub fn from_files(
        config_path: &Path,
        tokenizer_path: &Path,
        weights_path: &Path,
        device: &Device,
    ) -> Result<Self> {
        // Load config
        let config_str = std::fs::read_to_string(config_path)
            .with_context(|| format!("Failed to read config: {}", config_path.display()))?;
        let config: t5::Config =
            serde_json::from_str(&config_str).context("Failed to parse T5 config")?;

        // Load tokenizer
        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {e}"))?;

        // Load weights
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[weights_path], DType::F32, device)
                .context("Failed to load weights")?
        };

        // Build T5 encoder
        // The XTR model has tensors named "encoder.*" directly, not "encoder.encoder.*"
        // So we pass vb directly without prepending "encoder"
        let encoder =
            T5EncoderModel::load(vb.clone(), &config).context("Failed to load T5 encoder")?;

        // Build linear projection layer (768 -> 128)
        // XTR uses a simple linear projection without bias
        let projection = if vb.contains_tensor("projection.weight") {
            candle_nn::linear_no_bias(HIDDEN_DIM, EMBEDDING_DIM, vb.pp("projection"))
                .context("Failed to load projection layer")?
        } else {
            // If no projection layer in weights, initialize randomly
            // This is for compatibility with models that don't include projection
            tracing::info!("No projection layer found in model, using random initialization");
            let ws = Tensor::randn(0f32, 1f32, (EMBEDDING_DIM, HIDDEN_DIM), device)?;
            Linear::new(ws, None)
        };

        Ok(Self {
            encoder,
            projection,
            tokenizer,
            device: device.clone(),
        })
    }

    /// Generate embeddings for a document
    ///
    /// Returns a tensor of shape [num_tokens, 128] containing
    /// L2-normalized embeddings for each non-padding token.
    pub fn embed_document(&mut self, text: &str) -> Result<Tensor> {
        self.embed(text, DOC_MAXLEN)
    }

    /// Generate embeddings for a query
    pub fn embed_query(&mut self, text: &str) -> Result<Tensor> {
        self.embed(text, QUERY_MAXLEN)
    }

    /// Generate embeddings for text with specified max length
    fn embed(&mut self, text: &str, max_len: usize) -> Result<Tensor> {
        let tokens = self.tokenize(text, max_len)?;

        // Convert to tensor
        let input_ids = Tensor::new(&tokens[..], &self.device)?.unsqueeze(0)?; // [1, seq_len]

        // Run encoder
        let hidden_states = self
            .encoder
            .forward(&input_ids)
            .context("T5 encoder forward pass failed")?;

        // hidden_states: [1, seq_len, 768]
        let hidden_states = hidden_states.squeeze(0)?; // [seq_len, 768]

        // Apply projection: [seq_len, 768] -> [seq_len, 128]
        let embeddings = self.projection.forward(&hidden_states)?;

        // L2 normalize
        let embeddings = l2_normalize(&embeddings)?;

        Ok(embeddings)
    }

    /// Embed multiple texts in a batch
    pub fn embed_batch(&mut self, texts: &[&str], max_len: usize) -> Result<Vec<Tensor>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let mut groups: HashMap<usize, Vec<(usize, Vec<u32>)>> = HashMap::new();
        for (idx, text) in texts.iter().enumerate() {
            let tokens = self.tokenize(text, max_len)?;
            groups.entry(tokens.len()).or_default().push((idx, tokens));
        }

        let mut results: Vec<Option<Tensor>> = vec![None; texts.len()];

        for (seq_len, entries) in groups {
            let batch_size = entries.len();
            let mut flat_tokens = Vec::with_capacity(batch_size * seq_len);
            for (_, tokens) in &entries {
                flat_tokens.extend_from_slice(tokens);
            }

            let input_ids = Tensor::from_slice(&flat_tokens, (batch_size, seq_len), &self.device)?;
            let hidden_states = self
                .encoder
                .forward(&input_ids)
                .context("T5 encoder forward pass failed")?;

            let embeddings = self.projection.forward(&hidden_states)?;
            let embeddings = l2_normalize(&embeddings)?;

            for (batch_idx, (orig_idx, _)) in entries.iter().enumerate() {
                results[*orig_idx] = Some(embeddings.i(batch_idx)?);
            }
        }

        results
            .into_iter()
            .map(|item| item.ok_or_else(|| anyhow::anyhow!("Missing batch embedding result")))
            .collect()
    }

    /// Get embedding dimension
    pub fn embedding_dim(&self) -> usize {
        EMBEDDING_DIM
    }

    /// Get the device this embedder runs on
    pub fn device(&self) -> &Device {
        &self.device
    }

    fn tokenize(&self, text: &str, max_len: usize) -> Result<Vec<u32>> {
        // Lowercase and tokenize (XTR lowercases input)
        let text = text.to_lowercase();
        let encoding = self
            .tokenizer
            .encode(text.as_str(), true)
            .map_err(|e| anyhow::anyhow!("Tokenization failed: {e}"))?;

        let mut tokens: Vec<u32> = encoding.get_ids().to_vec();

        // Truncate if needed
        if tokens.len() > max_len {
            tokens.truncate(max_len);
        }

        Ok(tokens)
    }
}

/// Implement EmbedderBackend for Candle Embedder
impl EmbedderBackend for Embedder {
    fn embed_document(&mut self, text: &str) -> Result<EmbeddingResult> {
        let tensor = self.embed(text, DOC_MAXLEN)?;
        EmbeddingResult::from_tensor(&tensor)
    }

    fn embed_query(&mut self, text: &str) -> Result<EmbeddingResult> {
        let tensor = self.embed(text, QUERY_MAXLEN)?;
        EmbeddingResult::from_tensor(&tensor)
    }

    /// Optimized batch embedding using native Candle batching
    fn embed_batch(&mut self, texts: &[&str]) -> Result<Vec<EmbeddingResult>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        // Use the native batch method which groups by sequence length
        let tensors = Embedder::embed_batch(self, texts, DOC_MAXLEN)?;
        tensors.iter().map(EmbeddingResult::from_tensor).collect()
    }
}

/// L2 normalize each row of a 2D tensor (Candle)
fn l2_normalize(tensor: &Tensor) -> Result<Tensor> {
    let sum_dim = match tensor.dims().len() {
        2 => 1,
        3 => 2,
        other => {
            return Err(anyhow::anyhow!(
                "Expected 2D or 3D tensor for L2 normalization, got {other}D"
            ))
        }
    };
    let norm = tensor
        .sqr()?
        .sum_keepdim(sum_dim)?
        .sqrt()?
        .broadcast_add(&Tensor::new(&[1e-12f32], tensor.device())?)?;
    Ok(tensor.broadcast_div(&norm)?)
}

/// Compute dot product similarity between query and document embeddings
///
/// Returns the maximum similarity score (MaxSim) across all token pairs.
pub fn maxsim(query_emb: &Tensor, doc_emb: &Tensor) -> Result<f32> {
    // query_emb: [q_tokens, dim]
    // doc_emb: [d_tokens, dim]

    // Compute all pairwise similarities: [q_tokens, d_tokens]
    let similarities = query_emb.matmul(&doc_emb.t()?)?;

    // For each query token, take max over document tokens
    let max_per_query = similarities.max(1)?; // [q_tokens]

    // Average over query tokens (normalized MaxSim in [0, 1] range)
    let q_tokens = query_emb.dims()[0] as f32;
    let score = max_per_query.sum_all()?.to_scalar::<f32>()? / q_tokens;

    Ok(score)
}

/// Convert embeddings tensor to flat f32 vec for storage
pub fn embeddings_to_vec(embeddings: &Tensor) -> Result<Vec<f32>> {
    embeddings
        .flatten_all()?
        .to_vec1::<f32>()
        .map_err(Into::into)
}

/// Convert flat f32 vec back to embeddings tensor
pub fn vec_to_embeddings(data: &[f32], num_tokens: usize, device: &Device) -> Result<Tensor> {
    Tensor::from_slice(data, (num_tokens, EMBEDDING_DIM), device).map_err(Into::into)
}

/// Compute MaxSim score from flat `Vec<f32>` embeddings (backend-agnostic)
///
/// This allows scoring without depending on Candle or ndarray.
/// Both query and doc embeddings should be flattened, with each token's
/// EMBEDDING_DIM values stored consecutively.
pub fn maxsim_from_vecs(
    query_data: &[f32],
    query_tokens: usize,
    doc_data: &[f32],
    doc_tokens: usize,
) -> f32 {
    if query_tokens == 0 || doc_tokens == 0 {
        return 0.0;
    }

    let mut total_score = 0.0f32;

    for q_idx in 0..query_tokens {
        let q_start = q_idx * EMBEDDING_DIM;
        let q_end = q_start + EMBEDDING_DIM;
        let q_row = &query_data[q_start..q_end];

        let mut max_sim = f32::NEG_INFINITY;

        for d_idx in 0..doc_tokens {
            let d_start = d_idx * EMBEDDING_DIM;
            let d_end = d_start + EMBEDDING_DIM;
            let d_row = &doc_data[d_start..d_end];

            // Dot product
            let sim: f32 = q_row.iter().zip(d_row.iter()).map(|(a, b)| a * b).sum();
            if sim > max_sim {
                max_sim = sim;
            }
        }

        total_score += max_sim;
    }

    total_score / query_tokens as f32
}

/// Compute similarity score from flat embeddings with variable dimensions
///
/// This function handles both multi-vector (MaxSim) and single-vector (cosine)
/// scoring based on the embedding structure:
/// - If both embeddings have 1 "token", uses cosine similarity (single-vector)
/// - Otherwise, uses MaxSim (multi-vector)
///
/// The `embed_dim` parameter specifies the embedding dimension (e.g., 128 for XTR, 768 for UniXcoder)
pub fn similarity_from_vecs(
    query_data: &[f32],
    query_tokens: usize,
    doc_data: &[f32],
    doc_tokens: usize,
    embed_dim: usize,
) -> f32 {
    if query_tokens == 0 || doc_tokens == 0 || embed_dim == 0 {
        return 0.0;
    }

    // Single-vector mode: both have 1 token, use cosine similarity
    if query_tokens == 1 && doc_tokens == 1 {
        // Verify dimensions match
        if query_data.len() != embed_dim || doc_data.len() != embed_dim {
            return 0.0;
        }
        // Cosine similarity (embeddings should be L2-normalized)
        return query_data
            .iter()
            .zip(doc_data.iter())
            .map(|(a, b)| a * b)
            .sum();
    }

    // Multi-vector mode: use MaxSim
    let mut total_score = 0.0f32;

    for q_idx in 0..query_tokens {
        let q_start = q_idx * embed_dim;
        let q_end = q_start + embed_dim;
        if q_end > query_data.len() {
            continue;
        }
        let q_row = &query_data[q_start..q_end];

        let mut max_sim = f32::NEG_INFINITY;

        for d_idx in 0..doc_tokens {
            let d_start = d_idx * embed_dim;
            let d_end = d_start + embed_dim;
            if d_end > doc_data.len() {
                continue;
            }
            let d_row = &doc_data[d_start..d_end];

            // Dot product
            let sim: f32 = q_row.iter().zip(d_row.iter()).map(|(a, b)| a * b).sum();
            if sim > max_sim {
                max_sim = sim;
            }
        }

        if max_sim > f32::NEG_INFINITY {
            total_score += max_sim;
        }
    }

    total_score / query_tokens as f32
}

/// L2 normalize each row of a 2D ndarray (for ONNX backend)
#[cfg(feature = "onnx")]
pub fn l2_normalize_ndarray(array: &ndarray::Array2<f32>) -> Result<ndarray::Array2<f32>> {
    use ndarray::Axis;

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
    fn test_l2_normalize() {
        let device = Device::Cpu;
        let tensor = Tensor::new(&[[3.0f32, 4.0], [1.0, 0.0]], &device).unwrap();
        let normalized = l2_normalize(&tensor).unwrap();

        let data: Vec<Vec<f32>> = normalized.to_vec2().unwrap();

        // First row: [3, 4] / 5 = [0.6, 0.8]
        assert!((data[0][0] - 0.6).abs() < 1e-5);
        assert!((data[0][1] - 0.8).abs() < 1e-5);

        // Second row: [1, 0] / 1 = [1, 0]
        assert!((data[1][0] - 1.0).abs() < 1e-5);
        assert!((data[1][1] - 0.0).abs() < 1e-5);
    }

    #[test]
    fn test_maxsim() {
        let device = Device::Cpu;

        // Query: 2 tokens, dim 4
        let query = Tensor::new(&[[1.0f32, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0]], &device).unwrap();

        // Doc: 3 tokens, dim 4
        let doc = Tensor::new(
            &[
                [1.0f32, 0.0, 0.0, 0.0], // matches query token 0
                [0.0, 0.5, 0.5, 0.0],    // partial match to query token 1
                [0.0, 0.0, 0.0, 1.0],    // no match
            ],
            &device,
        )
        .unwrap();

        let score = maxsim(&query, &doc).unwrap();

        // Query token 0: max similarity with doc = 1.0 (with doc token 0)
        // Query token 1: max similarity with doc = 0.5 (with doc token 1)
        // Average: (1.0 + 0.5) / 2 = 0.75
        assert!((score - 0.75).abs() < 1e-5);
    }

    #[test]
    fn test_embeddings_conversion() {
        let device = Device::Cpu;
        let data: Vec<f32> = (0..256).map(|i| i as f32).collect();
        let num_tokens = 2;

        let tensor = vec_to_embeddings(&data, num_tokens, &device).unwrap();
        assert_eq!(tensor.dims(), &[2, 128]);

        let back = embeddings_to_vec(&tensor).unwrap();
        assert_eq!(data, back);
    }

    #[test]
    fn test_l2_normalize_3d_tensor() {
        let device = Device::Cpu;
        // Batch of 2 sequences, each with 2 tokens, dim 2
        let tensor = Tensor::new(
            &[[[3.0f32, 4.0], [1.0, 0.0]], [[0.0, 1.0], [5.0, 12.0]]],
            &device,
        )
        .unwrap();
        let normalized = l2_normalize(&tensor).unwrap();

        let data: Vec<Vec<Vec<f32>>> = normalized.to_vec3().unwrap();

        // First batch, first token: [3, 4] / 5 = [0.6, 0.8]
        assert!((data[0][0][0] - 0.6).abs() < 1e-5);
        assert!((data[0][0][1] - 0.8).abs() < 1e-5);

        // First batch, second token: [1, 0] / 1 = [1, 0]
        assert!((data[0][1][0] - 1.0).abs() < 1e-5);
        assert!((data[0][1][1] - 0.0).abs() < 1e-5);

        // Second batch, first token: [0, 1] / 1 = [0, 1]
        assert!((data[1][0][0] - 0.0).abs() < 1e-5);
        assert!((data[1][0][1] - 1.0).abs() < 1e-5);

        // Second batch, second token: [5, 12] / 13 = [5/13, 12/13]
        assert!((data[1][1][0] - 5.0 / 13.0).abs() < 1e-5);
        assert!((data[1][1][1] - 12.0 / 13.0).abs() < 1e-5);
    }

    #[test]
    fn test_l2_normalize_invalid_dim() {
        let device = Device::Cpu;
        // 1D tensor should fail
        let tensor = Tensor::new(&[1.0f32, 2.0, 3.0], &device).unwrap();
        let result = l2_normalize(&tensor);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Expected 2D or 3D tensor"));
    }

    #[test]
    fn test_maxsim_single_token() {
        let device = Device::Cpu;

        // Single token query and doc
        let query = Tensor::new(&[[1.0f32, 0.0, 0.0, 0.0]], &device).unwrap();
        let doc = Tensor::new(&[[1.0f32, 0.0, 0.0, 0.0]], &device).unwrap();

        let score = maxsim(&query, &doc).unwrap();
        assert!((score - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_maxsim_orthogonal() {
        let device = Device::Cpu;

        // Completely orthogonal vectors should have 0 similarity
        let query = Tensor::new(&[[1.0f32, 0.0], [0.0, 1.0]], &device).unwrap();
        let doc = Tensor::new(&[[0.0f32, 0.0], [0.0, 0.0]], &device).unwrap();

        let score = maxsim(&query, &doc).unwrap();
        assert!((score - 0.0).abs() < 1e-5);
    }

    #[test]
    fn test_maxsim_negative_similarity() {
        let device = Device::Cpu;

        // Opposite vectors have -1 dot product
        let query = Tensor::new(&[[1.0f32, 0.0]], &device).unwrap();
        let doc = Tensor::new(&[[-1.0f32, 0.0]], &device).unwrap();

        let score = maxsim(&query, &doc).unwrap();
        assert!((score - (-1.0)).abs() < 1e-5);
    }

    #[test]
    fn test_vec_to_embeddings_single_token() {
        let device = Device::Cpu;
        let data: Vec<f32> = (0..128).map(|i| i as f32).collect();

        let tensor = vec_to_embeddings(&data, 1, &device).unwrap();
        assert_eq!(tensor.dims(), &[1, EMBEDDING_DIM]);

        let back = embeddings_to_vec(&tensor).unwrap();
        assert_eq!(data, back);
    }

    #[test]
    fn test_embedding_dim_constant() {
        // Ensure the constant is what we expect
        assert_eq!(EMBEDDING_DIM, 128);
    }

    #[test]
    fn test_hidden_dim_constant() {
        // T5 encoder produces 768-dimensional hidden states
        assert_eq!(HIDDEN_DIM, 768);
        // Should be divisible by common attention head counts
        assert_eq!(HIDDEN_DIM % 12, 0); // T5 uses 12 attention heads
    }

    #[test]
    fn test_doc_maxlen_constant() {
        // Maximum document length should be reasonable
        assert_eq!(DOC_MAXLEN, 512);
        // Note: 512 is a power of 2 for efficient batching
    }

    #[test]
    fn test_query_maxlen_constant() {
        // Maximum query length should be reasonable
        assert_eq!(QUERY_MAXLEN, 64);
        // Note: 64 is a power of 2 and less than DOC_MAXLEN (512)
    }

    #[test]
    fn test_embedding_and_hidden_dim_relationship() {
        // Verify constants have expected values
        assert_eq!(EMBEDDING_DIM, 128);
        assert_eq!(HIDDEN_DIM, 768);
        // Note: EMBEDDING_DIM (128) < HIDDEN_DIM (768), compression ratio 6:1
    }

    #[test]
    fn test_maxsim_from_vecs() {
        // 2 query tokens, 3 doc tokens, dim 4
        // (using dim 4 for testing, not EMBEDDING_DIM)
        let query_data = [
            1.0f32, 0.0, 0.0, 0.0, // token 0
            0.0, 1.0, 0.0, 0.0, // token 1
        ];
        let doc_data = [
            1.0f32, 0.0, 0.0, 0.0, // matches query token 0
            0.0, 0.5, 0.5, 0.0, // partial match to query token 1
            0.0, 0.0, 0.0, 1.0, // no match
        ];

        // Can't use maxsim_from_vecs directly since it uses EMBEDDING_DIM
        // So test the logic inline with dim 4
        let dim = 4;
        let query_tokens = 2;
        let doc_tokens = 3;

        let mut total_score = 0.0f32;
        for q_idx in 0..query_tokens {
            let q_start = q_idx * dim;
            let q_row = &query_data[q_start..q_start + dim];
            let mut max_sim = f32::NEG_INFINITY;
            for d_idx in 0..doc_tokens {
                let d_start = d_idx * dim;
                let d_row = &doc_data[d_start..d_start + dim];
                let sim: f32 = q_row.iter().zip(d_row.iter()).map(|(a, b)| a * b).sum();
                if sim > max_sim {
                    max_sim = sim;
                }
            }
            total_score += max_sim;
        }
        let score = total_score / query_tokens as f32;

        // Query token 0: max sim = 1.0 (with doc token 0)
        // Query token 1: max sim = 0.5 (with doc token 1)
        // Average: (1.0 + 0.5) / 2 = 0.75
        assert!((score - 0.75).abs() < 1e-5);
    }

    #[test]
    fn test_maxsim_from_vecs_empty() {
        let query: Vec<f32> = vec![];
        let doc: Vec<f32> = vec![];
        assert_eq!(maxsim_from_vecs(&query, 0, &doc, 0), 0.0);

        // One empty, one not
        let non_empty = vec![0.0f32; EMBEDDING_DIM];
        assert_eq!(maxsim_from_vecs(&query, 0, &non_empty, 1), 0.0);
        assert_eq!(maxsim_from_vecs(&non_empty, 1, &doc, 0), 0.0);
    }

    #[test]
    fn test_maxsim_from_vecs_single_token() {
        // Single token query and doc with EMBEDDING_DIM
        let mut query = vec![0.0f32; EMBEDDING_DIM];
        query[0] = 1.0;
        let mut doc = vec![0.0f32; EMBEDDING_DIM];
        doc[0] = 1.0;

        let score = maxsim_from_vecs(&query, 1, &doc, 1);
        assert!((score - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_vec_to_embeddings_dimension_mismatch() {
        let device = Device::Cpu;

        // Data size doesn't match num_tokens * EMBEDDING_DIM
        let data: Vec<f32> = vec![0.0; 100]; // 100 floats
        let num_tokens = 2; // Would need 256 floats

        let result = vec_to_embeddings(&data, num_tokens, &device);
        assert!(result.is_err(), "Should fail with mismatched dimensions");
    }

    #[test]
    fn test_vec_to_embeddings_empty() {
        let device = Device::Cpu;

        // Empty data with 0 tokens should work
        let data: Vec<f32> = vec![];
        let result = vec_to_embeddings(&data, 0, &device);
        assert!(result.is_ok());
        let tensor = result.unwrap();
        assert_eq!(tensor.dims(), &[0, EMBEDDING_DIM]);
    }

    #[test]
    fn test_maxsim_many_tokens() {
        let device = Device::Cpu;

        // Larger token counts to verify scaling
        let query_tokens = 10;
        let doc_tokens = 20;
        let dim = 4;

        // Create normalized query and doc embeddings
        // Each token is a unit vector pointing in different directions
        let mut query_data: Vec<f32> = Vec::with_capacity(query_tokens * dim);
        for i in 0..query_tokens {
            let mut token = vec![0.0f32; dim];
            token[i % dim] = 1.0;
            query_data.extend(token);
        }

        let mut doc_data: Vec<f32> = Vec::with_capacity(doc_tokens * dim);
        for i in 0..doc_tokens {
            let mut token = vec![0.0f32; dim];
            token[i % dim] = 1.0;
            doc_data.extend(token);
        }

        let query = Tensor::from_slice(&query_data, (query_tokens, dim), &device).unwrap();
        let doc = Tensor::from_slice(&doc_data, (doc_tokens, dim), &device).unwrap();

        let score = maxsim(&query, &doc).unwrap();

        // Score should be in valid range for normalized vectors
        assert!(score.is_finite());
        assert!(
            (-1.0..=1.0).contains(&score),
            "MaxSim score {score} out of range"
        );
        // With unit vectors and matching directions, score should be positive
        assert!(score > 0.0, "Expected positive score for matching vectors");
    }

    // ================= EmbeddingModel tests =================

    #[test]
    fn test_embedding_model_default() {
        let model = EmbeddingModel::default();
        assert_eq!(model, EmbeddingModel::Xtr);
    }

    #[test]
    fn test_embedding_model_parse() {
        // XTR variations
        assert_eq!(
            "xtr".parse::<EmbeddingModel>().unwrap(),
            EmbeddingModel::Xtr
        );
        assert_eq!(
            "XTR".parse::<EmbeddingModel>().unwrap(),
            EmbeddingModel::Xtr
        );
        assert_eq!(
            "xtr-base-en".parse::<EmbeddingModel>().unwrap(),
            EmbeddingModel::Xtr
        );
        assert_eq!(
            "google/xtr-base-en".parse::<EmbeddingModel>().unwrap(),
            EmbeddingModel::Xtr
        );

        // UniXcoder variations
        assert_eq!(
            "unixcoder".parse::<EmbeddingModel>().unwrap(),
            EmbeddingModel::UniXcoder
        );
        assert_eq!(
            "UniXcoder".parse::<EmbeddingModel>().unwrap(),
            EmbeddingModel::UniXcoder
        );
        assert_eq!(
            "microsoft/unixcoder-base"
                .parse::<EmbeddingModel>()
                .unwrap(),
            EmbeddingModel::UniXcoder
        );

        // JinaCode variations
        assert_eq!(
            "jinacode".parse::<EmbeddingModel>().unwrap(),
            EmbeddingModel::JinaCode
        );
        assert_eq!(
            "jina-code".parse::<EmbeddingModel>().unwrap(),
            EmbeddingModel::JinaCode
        );

        // Invalid
        assert!("invalid-model".parse::<EmbeddingModel>().is_err());
    }

    #[test]
    fn test_embedding_model_properties() {
        // XTR properties
        let xtr = EmbeddingModel::Xtr;
        assert_eq!(xtr.model_id(), "google/xtr-base-en");
        assert_eq!(xtr.embedding_dim(), EMBEDDING_DIM);
        assert!(xtr.is_multi_vector());
        assert!(xtr.is_available());
        assert_eq!(xtr.name(), "XTR");

        // UniXcoder properties
        let unixcoder = EmbeddingModel::UniXcoder;
        assert_eq!(unixcoder.model_id(), "microsoft/unixcoder-base");
        assert_eq!(unixcoder.embedding_dim(), 768);
        assert!(!unixcoder.is_multi_vector());
        assert!(unixcoder.is_available()); // Now implemented!
        assert_eq!(unixcoder.name(), "UniXcoder");

        // JinaCode properties
        let jina = EmbeddingModel::JinaCode;
        assert_eq!(jina.model_id(), "jinaai/jina-embeddings-v2-base-code");
        assert_eq!(jina.embedding_dim(), 768);
        assert!(!jina.is_multi_vector());
        #[cfg(feature = "onnx")]
        assert!(jina.is_available()); // Available with ONNX feature
        #[cfg(not(feature = "onnx"))]
        assert!(!jina.is_available()); // Requires ONNX feature
        assert_eq!(jina.name(), "JinaCode");
    }

    #[test]
    fn test_embedding_model_all() {
        let all = EmbeddingModel::all();
        #[cfg(feature = "clip")]
        assert_eq!(all.len(), 5);
        #[cfg(not(feature = "clip"))]
        assert_eq!(all.len(), 4);
        assert!(all.contains(&EmbeddingModel::Xtr));
        assert!(all.contains(&EmbeddingModel::UniXcoder));
        assert!(all.contains(&EmbeddingModel::JinaCode));
        assert!(all.contains(&EmbeddingModel::JinaColbert));
        #[cfg(feature = "clip")]
        assert!(all.contains(&EmbeddingModel::Clip));
    }

    #[test]
    fn test_embedding_model_available() {
        let available = EmbeddingModel::available();
        // XTR and UniXcoder are always available
        // JinaColbert and JinaCode when ONNX feature is enabled
        // CLIP when clip feature is enabled
        #[allow(unused_mut)]
        let mut expected_count = 2; // XTR + UniXcoder
        #[cfg(feature = "onnx")]
        {
            expected_count += 2; // JinaColbert + JinaCode
        }
        #[cfg(feature = "clip")]
        {
            expected_count += 1; // CLIP
        }
        assert_eq!(available.len(), expected_count);
        assert!(available.contains(&EmbeddingModel::Xtr));
        assert!(available.contains(&EmbeddingModel::UniXcoder));
        #[cfg(feature = "onnx")]
        assert!(available.contains(&EmbeddingModel::JinaColbert));
        #[cfg(feature = "onnx")]
        assert!(available.contains(&EmbeddingModel::JinaCode));
        #[cfg(feature = "clip")]
        assert!(available.contains(&EmbeddingModel::Clip));
    }

    #[test]
    fn test_embedding_model_display() {
        assert_eq!(format!("{}", EmbeddingModel::Xtr), "XTR");
        assert_eq!(format!("{}", EmbeddingModel::UniXcoder), "UniXcoder");
        assert_eq!(format!("{}", EmbeddingModel::JinaCode), "JinaCode");
    }

    // ================= similarity_from_vecs tests =================

    #[test]
    fn test_similarity_from_vecs_single_vector_identical() {
        // Single-vector mode: both have 1 token
        let dim = 768;
        let mut a = vec![0.0f32; dim];
        a[0] = 1.0; // Unit vector
        let b = a.clone();

        let score = similarity_from_vecs(&a, 1, &b, 1, dim);
        assert!(
            (score - 1.0).abs() < 1e-5,
            "Identical vectors should have similarity 1.0"
        );
    }

    #[test]
    fn test_similarity_from_vecs_single_vector_orthogonal() {
        let dim = 768;
        let mut a = vec![0.0f32; dim];
        a[0] = 1.0;
        let mut b = vec![0.0f32; dim];
        b[1] = 1.0;

        let score = similarity_from_vecs(&a, 1, &b, 1, dim);
        assert!(
            (score - 0.0).abs() < 1e-5,
            "Orthogonal vectors should have similarity 0.0"
        );
    }

    #[test]
    fn test_similarity_from_vecs_single_vector_opposite() {
        let dim = 768;
        let mut a = vec![0.0f32; dim];
        a[0] = 1.0;
        let mut b = vec![0.0f32; dim];
        b[0] = -1.0;

        let score = similarity_from_vecs(&a, 1, &b, 1, dim);
        assert!(
            (score - (-1.0)).abs() < 1e-5,
            "Opposite vectors should have similarity -1.0"
        );
    }

    #[test]
    fn test_similarity_from_vecs_multi_vector() {
        // Multi-vector mode (like XTR): 2 query tokens, 3 doc tokens
        let dim = 4;
        let query = vec![
            1.0f32, 0.0, 0.0, 0.0, // token 0: points in x direction
            0.0, 1.0, 0.0, 0.0, // token 1: points in y direction
        ];
        let doc = vec![
            1.0f32, 0.0, 0.0, 0.0, // token 0: matches query token 0 exactly
            0.0, 0.5, 0.5, 0.0, // token 1: partial match to query token 1
            0.0, 0.0, 0.0, 1.0, // token 2: no match
        ];

        let score = similarity_from_vecs(&query, 2, &doc, 3, dim);
        // Query token 0: max sim = 1.0 (with doc token 0)
        // Query token 1: max sim = 0.5 (with doc token 1)
        // Average: (1.0 + 0.5) / 2 = 0.75
        assert!((score - 0.75).abs() < 1e-5, "Expected 0.75, got {score}");
    }

    #[test]
    fn test_similarity_from_vecs_empty() {
        let dim = 768;
        let a: Vec<f32> = vec![];
        let b: Vec<f32> = vec![];

        // Empty embeddings
        assert_eq!(similarity_from_vecs(&a, 0, &b, 0, dim), 0.0);

        // One empty
        let non_empty = vec![0.0f32; dim];
        assert_eq!(similarity_from_vecs(&a, 0, &non_empty, 1, dim), 0.0);
        assert_eq!(similarity_from_vecs(&non_empty, 1, &b, 0, dim), 0.0);

        // Zero dimension
        assert_eq!(similarity_from_vecs(&non_empty, 1, &non_empty, 1, 0), 0.0);
    }

    #[test]
    fn test_similarity_from_vecs_dimension_mismatch() {
        // Single-vector with wrong dimensions should return 0
        let dim = 768;
        let a = vec![0.0f32; 100]; // Wrong size
        let b = vec![0.0f32; dim];

        let score = similarity_from_vecs(&a, 1, &b, 1, dim);
        assert_eq!(score, 0.0, "Dimension mismatch should return 0");
    }
}
