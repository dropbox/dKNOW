//! UniXcoder embedding model
//!
//! microsoft/unixcoder-base is a RoBERTa-based model specialized for code.
//! Unlike XTR (multi-vector), UniXcoder produces a single 768-dimensional
//! embedding per document using CLS token pooling.
//!
//! Architecture: RoBERTa (BERT-compatible)
//! Output: 768 dimensions (single-vector, CLS pooling)
//! Scoring: Cosine similarity (not MaxSim)

use anyhow::{Context, Result};
use candle_core::{DType, Device, IndexOp, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config as BertConfig};
use hf_hub::api::sync::ApiBuilder;
use std::path::Path;
use tokenizers::Tokenizer;

use crate::embedder::{EmbedderBackend, EmbeddingResult};

/// UniXcoder embedding dimension
pub const UNIXCODER_DIM: usize = 768;

/// Maximum sequence length for UniXcoder
const MAX_SEQ_LEN: usize = 512;

/// UniXcoder embedder using Candle
pub struct UniXcoderEmbedder {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
}

impl UniXcoderEmbedder {
    /// Load UniXcoder from HuggingFace hub
    pub fn new(device: &Device) -> Result<Self> {
        Self::from_pretrained("microsoft/unixcoder-base", device)
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

        tracing::info!("Downloading UniXcoder model files from {}", model_id);
        let config_path = repo
            .get("config.json")
            .context("Failed to get config.json")?;

        // UniXcoder uses BPE tokenizer (vocab.json + merges.txt), not tokenizer.json
        let vocab_path = repo.get("vocab.json").context("Failed to get vocab.json")?;
        let merges_path = repo.get("merges.txt").context("Failed to get merges.txt")?;

        let weights_path = repo
            .get("model.safetensors")
            .or_else(|_| repo.get("pytorch_model.bin"))
            .context("Failed to get model weights")?;

        Self::from_files_bpe(
            &config_path,
            &vocab_path,
            &merges_path,
            &weights_path,
            device,
        )
    }

    fn try_manual_download(model_id: &str, device: &Device) -> Result<Self> {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("sg")
            .join("models")
            .join(model_id.replace('/', "_"));

        std::fs::create_dir_all(&cache_dir)?;

        let base_url = format!("https://huggingface.co/{model_id}/resolve/main");

        // UniXcoder uses BPE tokenizer (vocab.json + merges.txt)
        let files = [
            "config.json",
            "vocab.json",
            "merges.txt",
            "pytorch_model.bin",
        ];
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

        Self::from_files_bpe(&paths[0], &paths[1], &paths[2], &paths[3], device)
    }

    /// Load from local files using tokenizer.json
    pub fn from_files(
        config_path: &Path,
        tokenizer_path: &Path,
        weights_path: &Path,
        device: &Device,
    ) -> Result<Self> {
        // Load tokenizer from tokenizer.json
        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {e}"))?;

        Self::from_files_with_tokenizer(config_path, tokenizer, weights_path, device)
    }

    /// Load from local files using BPE tokenizer (vocab.json + merges.txt)
    pub fn from_files_bpe(
        config_path: &Path,
        vocab_path: &Path,
        merges_path: &Path,
        weights_path: &Path,
        device: &Device,
    ) -> Result<Self> {
        use tokenizers::models::bpe::BPE;
        use tokenizers::pre_tokenizers::byte_level::ByteLevel;
        use tokenizers::processors::roberta::RobertaProcessing;
        use tokenizers::AddedToken;

        // Build BPE tokenizer from vocab.json and merges.txt
        let bpe = BPE::from_file(vocab_path.to_str().unwrap(), merges_path.to_str().unwrap())
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to load BPE model: {e}"))?;

        let mut tokenizer = Tokenizer::new(bpe);

        // Configure tokenizer like RoBERTa
        tokenizer.with_pre_tokenizer(Some(ByteLevel::new(false, true, false)));
        tokenizer.with_post_processor(Some(
            RobertaProcessing::new(("</s>".to_string(), 2), ("<s>".to_string(), 0))
                .trim_offsets(true)
                .add_prefix_space(false),
        ));

        // Add special tokens
        let special_tokens = vec![
            AddedToken::from("<s>", true),
            AddedToken::from("<pad>", true),
            AddedToken::from("</s>", true),
            AddedToken::from("<unk>", true),
            AddedToken::from("<mask>", true),
        ];
        tokenizer.add_special_tokens(&special_tokens);

        Self::from_files_with_tokenizer(config_path, tokenizer, weights_path, device)
    }

    /// Load from config and weights with a pre-built tokenizer
    fn from_files_with_tokenizer(
        config_path: &Path,
        tokenizer: Tokenizer,
        weights_path: &Path,
        device: &Device,
    ) -> Result<Self> {
        // Load config
        let config_str = std::fs::read_to_string(config_path)
            .with_context(|| format!("Failed to read config: {}", config_path.display()))?;
        let config: BertConfig =
            serde_json::from_str(&config_str).context("Failed to parse BERT config")?;

        // Load weights - support both safetensors and pytorch formats
        let vb = if weights_path.extension().is_some_and(|e| e == "safetensors") {
            unsafe {
                VarBuilder::from_mmaped_safetensors(&[weights_path], DType::F32, device)
                    .context("Failed to load safetensors weights")?
            }
        } else {
            // Load pytorch .bin format
            VarBuilder::from_pth(weights_path, DType::F32, device)
                .context("Failed to load pytorch weights")?
        };

        // Load model - UniXcoder uses "roberta" prefix in weights
        let model = BertModel::load(vb.pp("roberta"), &config)
            .or_else(|_| BertModel::load(vb, &config))
            .context("Failed to load UniXcoder model")?;

        Ok(Self {
            model,
            tokenizer,
            device: device.clone(),
        })
    }

    /// Tokenize text with UniXcoder tokenizer
    fn tokenize(&self, text: &str) -> Result<Vec<u32>> {
        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| anyhow::anyhow!("Tokenization failed: {e}"))?;

        let mut tokens: Vec<u32> = encoding.get_ids().to_vec();

        // Truncate if needed
        if tokens.len() > MAX_SEQ_LEN {
            tokens.truncate(MAX_SEQ_LEN);
        }

        Ok(tokens)
    }

    /// Generate embedding for text using CLS pooling
    fn embed(&mut self, text: &str) -> Result<Tensor> {
        let tokens = self.tokenize(text)?;
        let seq_len = tokens.len();

        // Create input tensors
        let input_ids = Tensor::new(&tokens[..], &self.device)?.unsqueeze(0)?; // [1, seq_len]
        let token_type_ids = Tensor::zeros((1, seq_len), DType::U32, &self.device)?;

        // Run forward pass
        let hidden_states = self
            .model
            .forward(&input_ids, &token_type_ids, None)
            .context("UniXcoder forward pass failed")?;

        // hidden_states: [1, seq_len, 768]
        // CLS pooling: take the first token's embedding
        let cls_embedding = hidden_states.i((0, 0))?; // [768]

        // L2 normalize
        let cls_embedding = l2_normalize_1d(&cls_embedding)?;

        Ok(cls_embedding)
    }

    /// Get embedding dimension
    pub fn embedding_dim(&self) -> usize {
        UNIXCODER_DIM
    }

    /// Get the device
    pub fn device(&self) -> &Device {
        &self.device
    }
}

impl EmbedderBackend for UniXcoderEmbedder {
    fn embed_document(&mut self, text: &str) -> Result<EmbeddingResult> {
        let tensor = self.embed(text)?;
        let data = tensor.to_vec1::<f32>()?;
        // Single-vector: 1 "token" with 768 dimensions
        // We store as if it's 1 token with dim 768, but for compatibility
        // we need to handle this differently in scoring
        Ok(EmbeddingResult::new(data, 1))
    }

    fn embed_query(&mut self, text: &str) -> Result<EmbeddingResult> {
        // For single-vector models, query and document embedding are the same
        self.embed_document(text)
    }

    fn embedding_dim(&self) -> usize {
        UNIXCODER_DIM
    }
}

/// L2 normalize a 1D tensor
fn l2_normalize_1d(tensor: &Tensor) -> Result<Tensor> {
    let norm = tensor
        .sqr()?
        .sum_all()?
        .sqrt()?
        .to_scalar::<f32>()?
        .max(1e-12);
    Ok((tensor / norm as f64)?)
}

/// Cosine similarity between two single-vector embeddings
///
/// For single-vector models like UniXcoder, we use cosine similarity
/// instead of MaxSim. Since embeddings are L2-normalized, this is
/// equivalent to dot product.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 0.0).abs() < 1e-5);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![-1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - (-1.0)).abs() < 1e-5);
    }

    #[test]
    fn test_cosine_similarity_empty() {
        let a: Vec<f32> = vec![];
        let b: Vec<f32> = vec![];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn test_cosine_similarity_mismatched_length() {
        let a = vec![1.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn test_unixcoder_dim() {
        assert_eq!(UNIXCODER_DIM, 768);
    }
}
