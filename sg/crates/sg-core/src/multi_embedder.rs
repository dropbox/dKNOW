//! Multi-embedding model router
//!
//! Routes content to the optimal embedding model based on content type:
//! - CJK content → Jina-ColBERT-v2 (native CJK tokenization, P@1=1.00)
//! - Code files → UniXcoder (20% better P@1 on code)
//! - Prose/text → XTR (9x better on natural language)
//!
//! This implements the content-type routing recommendation from
//! the multi-embedding evaluation (eval/MULTI_EMBEDDING_RESULTS.md).

use std::path::Path;

use anyhow::Result;
use markdown_chunker::segmentation::cjk::has_cjk;

use crate::code_preprocessor::{is_code_file, looks_like_code_query};
use crate::embedder::{EmbedderBackend, EmbeddingModel, EmbeddingResult};

/// Content type for routing to appropriate embedder
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentType {
    /// CJK content - Chinese, Japanese, Korean (route to Jina-ColBERT-v2)
    Cjk,
    /// Source code (route to UniXcoder)
    Code,
    /// Natural language prose (route to XTR)
    Text,
}

impl ContentType {
    /// Detect content type from file path
    pub fn from_path(path: &Path) -> Self {
        if is_code_file(path) {
            Self::Code
        } else {
            Self::Text
        }
    }

    /// Detect content type from query string
    ///
    /// Priority order:
    /// 1. CJK content → Jina-ColBERT-v2 (native CJK tokenization)
    /// 2. Code identifiers → UniXcoder (camelCase, snake_case)
    /// 3. Default → XTR (natural language)
    pub fn from_query(query: &str) -> Self {
        // CJK takes priority - Jina-ColBERT achieves P@1=1.00 on Japanese
        // without needing hybrid search fallback that XTR requires
        if has_cjk(query) {
            Self::Cjk
        } else if looks_like_code_query(query) {
            Self::Code
        } else {
            Self::Text
        }
    }

    /// Get the recommended embedding model for this content type
    pub fn recommended_model(&self) -> EmbeddingModel {
        match self {
            Self::Cjk => EmbeddingModel::JinaColbert,
            Self::Code => EmbeddingModel::UniXcoder,
            Self::Text => EmbeddingModel::Xtr,
        }
    }
}

/// Multi-model embedder that routes to XTR or UniXcoder based on content type
///
/// For production deployment:
/// - Code files are embedded with UniXcoder (20% better P@1 on code)
/// - Prose/text files are embedded with XTR (9x better on natural language)
///
/// The embedder handles lazy initialization - models are only loaded when first used.
pub struct MultiEmbedder {
    /// XTR embedder for general text (lazy loaded)
    xtr: Option<Box<dyn EmbedderBackend + Send>>,
    /// UniXcoder embedder for code (lazy loaded)
    unixcoder: Option<Box<dyn EmbedderBackend + Send>>,
    /// Device to use for model loading
    device: candle_core::Device,
    /// Whether to enable automatic model routing
    auto_routing: bool,
    /// Force a specific model (overrides auto routing)
    forced_model: Option<EmbeddingModel>,
}

impl MultiEmbedder {
    /// Create a new multi-embedder with lazy model loading
    pub fn new(device: candle_core::Device) -> Self {
        Self {
            xtr: None,
            unixcoder: None,
            device,
            auto_routing: true,
            forced_model: None,
        }
    }

    /// Create with a specific model forced (disables auto-routing)
    pub fn with_model(device: candle_core::Device, model: EmbeddingModel) -> Self {
        Self {
            xtr: None,
            unixcoder: None,
            device,
            auto_routing: false,
            forced_model: Some(model),
        }
    }

    /// Enable or disable automatic content-type routing
    pub fn set_auto_routing(&mut self, enabled: bool) {
        self.auto_routing = enabled;
        if enabled {
            self.forced_model = None;
        }
    }

    /// Force a specific model (disables auto-routing)
    pub fn set_forced_model(&mut self, model: Option<EmbeddingModel>) {
        self.forced_model = model;
        self.auto_routing = model.is_none();
    }

    /// Get XTR embedder, loading if necessary
    fn get_xtr(&mut self) -> Result<&mut (dyn EmbedderBackend + Send)> {
        if self.xtr.is_none() {
            tracing::info!("Loading XTR model for text embedding");
            let embedder = crate::embedder::Embedder::new(&self.device)?;
            self.xtr = Some(Box::new(embedder));
        }
        Ok(self.xtr.as_mut().unwrap().as_mut())
    }

    /// Get UniXcoder embedder, loading if necessary
    fn get_unixcoder(&mut self) -> Result<&mut (dyn EmbedderBackend + Send)> {
        if self.unixcoder.is_none() {
            tracing::info!("Loading UniXcoder model for code embedding");
            let embedder = crate::embedder_unixcoder::UniXcoderEmbedder::new(&self.device)?;
            self.unixcoder = Some(Box::new(embedder));
        }
        Ok(self.unixcoder.as_mut().unwrap().as_mut())
    }

    /// Get the embedder for a specific model
    fn get_embedder_for_model(
        &mut self,
        model: EmbeddingModel,
    ) -> Result<&mut (dyn EmbedderBackend + Send)> {
        match model {
            EmbeddingModel::Xtr => self.get_xtr(),
            EmbeddingModel::UniXcoder => self.get_unixcoder(),
            EmbeddingModel::JinaCode => {
                anyhow::bail!("JinaCode model not yet implemented")
            }
            EmbeddingModel::JinaColbert => {
                anyhow::bail!("JinaColBERT model not yet implemented - requires ONNX backend")
            }
            #[cfg(feature = "clip")]
            EmbeddingModel::Clip => {
                anyhow::bail!("CLIP model not supported in MultiEmbedder - use ClipEmbedder directly for images")
            }
        }
    }

    /// Embed a document with automatic model selection based on file path
    pub fn embed_document_with_path(&mut self, path: &Path, text: &str) -> Result<EmbeddingResult> {
        let model = self.select_model_for_path(path);
        let embedder = self.get_embedder_for_model(model)?;
        embedder.embed_document(text)
    }

    /// Embed a query with automatic model selection based on query content
    pub fn embed_query_auto(&mut self, query: &str) -> Result<EmbeddingResult> {
        let model = self.select_model_for_query(query);
        let embedder = self.get_embedder_for_model(model)?;
        embedder.embed_query(query)
    }

    /// Select model based on file path and settings
    pub fn select_model_for_path(&self, path: &Path) -> EmbeddingModel {
        if let Some(model) = self.forced_model {
            return model;
        }
        if !self.auto_routing {
            return EmbeddingModel::default();
        }
        ContentType::from_path(path).recommended_model()
    }

    /// Select model based on query content and settings
    pub fn select_model_for_query(&self, query: &str) -> EmbeddingModel {
        if let Some(model) = self.forced_model {
            return model;
        }
        if !self.auto_routing {
            return EmbeddingModel::default();
        }
        ContentType::from_query(query).recommended_model()
    }

    /// Get the embedding dimension for a given model
    pub fn embedding_dim_for_model(&self, model: EmbeddingModel) -> usize {
        model.embedding_dim()
    }

    /// Check if a model is loaded
    pub fn is_model_loaded(&self, model: EmbeddingModel) -> bool {
        match model {
            EmbeddingModel::Xtr => self.xtr.is_some(),
            EmbeddingModel::UniXcoder => self.unixcoder.is_some(),
            EmbeddingModel::JinaCode => false,
            EmbeddingModel::JinaColbert => false, // Not yet implemented
            #[cfg(feature = "clip")]
            EmbeddingModel::Clip => false, // CLIP handled separately via ClipEmbedder
        }
    }

    /// Preload both models for fast embedding
    pub fn preload_all(&mut self) -> Result<()> {
        let _ = self.get_xtr()?;
        let _ = self.get_unixcoder()?;
        Ok(())
    }

    /// Warm up loaded models
    pub fn warmup(&mut self) -> Result<()> {
        if let Some(ref mut xtr) = self.xtr {
            xtr.warmup()?;
        }
        if let Some(ref mut unixcoder) = self.unixcoder {
            unixcoder.warmup()?;
        }
        Ok(())
    }
}

/// Implement EmbedderBackend for MultiEmbedder
///
/// Note: This implementation uses XTR by default for embed_document
/// and embed_query. Use the path-aware methods for automatic routing.
impl EmbedderBackend for MultiEmbedder {
    fn embed_document(&mut self, text: &str) -> Result<EmbeddingResult> {
        // Without path info, we default to the forced model or XTR
        let model = self.forced_model.unwrap_or(EmbeddingModel::Xtr);
        let embedder = self.get_embedder_for_model(model)?;
        embedder.embed_document(text)
    }

    fn embed_query(&mut self, text: &str) -> Result<EmbeddingResult> {
        // For queries, auto-detect based on content if enabled
        self.embed_query_auto(text)
    }

    fn embedding_dim(&self) -> usize {
        // Return the default model's dimension
        self.forced_model
            .unwrap_or(EmbeddingModel::Xtr)
            .embedding_dim()
    }

    fn warmup(&mut self) -> Result<()> {
        MultiEmbedder::warmup(self)
    }
}

/// Detect the optimal embedding model for a collection of files.
///
/// Scans the files and determines whether the corpus is primarily code or text:
/// - If >50% of files are code files → UniXcoder (20% better P@1 on code)
/// - Otherwise → XTR (9x better on prose)
///
/// This enables automatic model selection for `sg index --auto-model`.
///
/// # Arguments
/// * `files` - Slice of file paths to analyze
///
/// # Returns
/// The recommended `EmbeddingModel` based on corpus composition.
///
/// # Example
/// ```
/// use std::path::PathBuf;
/// use sg_core::multi_embedder::detect_optimal_model;
///
/// let files = vec![
///     PathBuf::from("main.rs"),
///     PathBuf::from("lib.rs"),
///     PathBuf::from("README.md"),
/// ];
/// let model = detect_optimal_model(&files);
/// // Returns UniXcoder since 2/3 files are code
/// ```
pub fn detect_optimal_model(files: &[std::path::PathBuf]) -> EmbeddingModel {
    if files.is_empty() {
        return EmbeddingModel::default();
    }

    let code_count = files
        .iter()
        .filter(|f| ContentType::from_path(f) == ContentType::Code)
        .count();

    let code_ratio = code_count as f64 / files.len() as f64;

    // Use UniXcoder if >50% of files are code
    if code_ratio > 0.5 {
        EmbeddingModel::UniXcoder
    } else {
        EmbeddingModel::Xtr
    }
}

/// Result of model detection including statistics
#[derive(Debug, Clone)]
pub struct ModelDetectionResult {
    /// The recommended model
    pub model: EmbeddingModel,
    /// Total number of files analyzed
    pub total_files: usize,
    /// Number of code files
    pub code_files: usize,
    /// Number of text files
    pub text_files: usize,
    /// Ratio of code files (0.0 to 1.0)
    pub code_ratio: f64,
}

/// Detect the optimal embedding model with detailed statistics.
///
/// Similar to `detect_optimal_model` but returns detailed statistics
/// about the corpus composition for display to users.
pub fn detect_optimal_model_with_stats(files: &[std::path::PathBuf]) -> ModelDetectionResult {
    let total_files = files.len();

    if total_files == 0 {
        return ModelDetectionResult {
            model: EmbeddingModel::default(),
            total_files: 0,
            code_files: 0,
            text_files: 0,
            code_ratio: 0.0,
        };
    }

    let code_files = files
        .iter()
        .filter(|f| ContentType::from_path(f) == ContentType::Code)
        .count();

    let text_files = total_files - code_files;
    let code_ratio = code_files as f64 / total_files as f64;

    let model = if code_ratio > 0.5 {
        EmbeddingModel::UniXcoder
    } else {
        EmbeddingModel::Xtr
    };

    ModelDetectionResult {
        model,
        total_files,
        code_files,
        text_files,
        code_ratio,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_type_from_path() {
        assert_eq!(
            ContentType::from_path(Path::new("main.rs")),
            ContentType::Code
        );
        assert_eq!(
            ContentType::from_path(Path::new("script.py")),
            ContentType::Code
        );
        assert_eq!(
            ContentType::from_path(Path::new("README.md")),
            ContentType::Text
        );
        assert_eq!(
            ContentType::from_path(Path::new("novel.txt")),
            ContentType::Text
        );
        assert_eq!(
            ContentType::from_path(Path::new("data.json")),
            ContentType::Text
        );
    }

    #[test]
    fn test_content_type_from_query() {
        // CJK queries - highest priority, route to Jina-ColBERT
        assert_eq!(ContentType::from_query("日本語テスト"), ContentType::Cjk); // Japanese
        assert_eq!(ContentType::from_query("中文测试"), ContentType::Cjk); // Chinese
        assert_eq!(ContentType::from_query("한글 테스트"), ContentType::Cjk); // Korean
        assert_eq!(ContentType::from_query("search for 東京"), ContentType::Cjk); // Mixed with CJK

        // Code-like queries (no CJK)
        assert_eq!(ContentType::from_query("getUserName"), ContentType::Code);
        assert_eq!(ContentType::from_query("get_user_name"), ContentType::Code);
        assert_eq!(ContentType::from_query("HTTPServer"), ContentType::Code);

        // Natural language queries (no CJK, no code patterns)
        assert_eq!(
            ContentType::from_query("vampire Transylvania"),
            ContentType::Text
        );
        assert_eq!(
            ContentType::from_query("search for files"),
            ContentType::Text
        );
        assert_eq!(
            ContentType::from_query("how to handle errors"),
            ContentType::Text
        );
    }

    #[test]
    fn test_recommended_model() {
        assert_eq!(
            ContentType::Cjk.recommended_model(),
            EmbeddingModel::JinaColbert
        );
        assert_eq!(
            ContentType::Code.recommended_model(),
            EmbeddingModel::UniXcoder
        );
        assert_eq!(ContentType::Text.recommended_model(), EmbeddingModel::Xtr);
    }

    #[test]
    fn test_multi_embedder_model_selection() {
        let device = candle_core::Device::Cpu;
        let embedder = MultiEmbedder::new(device);

        // Auto-routing enabled by default
        assert_eq!(
            embedder.select_model_for_path(Path::new("main.rs")),
            EmbeddingModel::UniXcoder
        );
        assert_eq!(
            embedder.select_model_for_path(Path::new("novel.txt")),
            EmbeddingModel::Xtr
        );
    }

    #[test]
    fn test_multi_embedder_cjk_query_routing() {
        let device = candle_core::Device::Cpu;
        let embedder = MultiEmbedder::new(device);

        // CJK queries should route to JinaColbert
        assert_eq!(
            embedder.select_model_for_query("株式会社の財務報告"),
            EmbeddingModel::JinaColbert
        );
        assert_eq!(
            embedder.select_model_for_query("한국어 문서 검색"),
            EmbeddingModel::JinaColbert
        );

        // Non-CJK queries use normal routing
        assert_eq!(
            embedder.select_model_for_query("search for documents"),
            EmbeddingModel::Xtr
        );
        assert_eq!(
            embedder.select_model_for_query("findUserByEmail"),
            EmbeddingModel::UniXcoder
        );
    }

    #[test]
    fn test_multi_embedder_forced_model() {
        let device = candle_core::Device::Cpu;
        let embedder = MultiEmbedder::with_model(device, EmbeddingModel::Xtr);

        // Forced model overrides auto-routing
        assert_eq!(
            embedder.select_model_for_path(Path::new("main.rs")),
            EmbeddingModel::Xtr
        );
        assert_eq!(
            embedder.select_model_for_path(Path::new("novel.txt")),
            EmbeddingModel::Xtr
        );
    }

    #[test]
    fn test_detect_optimal_model_empty() {
        let files: Vec<std::path::PathBuf> = vec![];
        assert_eq!(detect_optimal_model(&files), EmbeddingModel::default());
    }

    #[test]
    fn test_detect_optimal_model_code_majority() {
        let files = vec![
            std::path::PathBuf::from("main.rs"),
            std::path::PathBuf::from("lib.rs"),
            std::path::PathBuf::from("README.md"),
        ];
        // 2/3 are code → UniXcoder
        assert_eq!(detect_optimal_model(&files), EmbeddingModel::UniXcoder);
    }

    #[test]
    fn test_detect_optimal_model_text_majority() {
        let files = vec![
            std::path::PathBuf::from("main.rs"),
            std::path::PathBuf::from("novel.txt"),
            std::path::PathBuf::from("README.md"),
            std::path::PathBuf::from("guide.md"),
        ];
        // 1/4 are code → XTR
        assert_eq!(detect_optimal_model(&files), EmbeddingModel::Xtr);
    }

    #[test]
    fn test_detect_optimal_model_with_stats() {
        let files = vec![
            std::path::PathBuf::from("main.rs"),
            std::path::PathBuf::from("lib.rs"),
            std::path::PathBuf::from("test.py"),
            std::path::PathBuf::from("README.md"),
        ];
        let result = detect_optimal_model_with_stats(&files);

        assert_eq!(result.total_files, 4);
        assert_eq!(result.code_files, 3); // .rs and .py
        assert_eq!(result.text_files, 1);
        assert_eq!(result.model, EmbeddingModel::UniXcoder);
        assert!((result.code_ratio - 0.75).abs() < 0.01);
    }
}
