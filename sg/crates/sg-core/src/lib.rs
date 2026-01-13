//! sg-core: Core library for SuperGrep semantic search
//!
//! This crate provides:
//! - XTR model embedding generation
//! - Progressive/lazy indexing with LSH and online k-means
//! - MaxSim-based semantic search
//! - SQLite storage for documents and embeddings
//!
//! Based on rust-warp: <https://github.com/jhansen_dbx/rust-warp>

pub mod chunker;
pub mod code_preprocessor;
pub mod dedup;
pub mod document;
pub mod embedder;
pub mod encoding;
pub mod file_types;
pub mod hnsw;
pub mod index;
pub mod memory;
pub mod quantizer;
pub mod query_cache;
pub mod rerank;
pub mod search;
pub mod storage;
pub mod summary;

#[cfg(feature = "onnx")]
pub mod embedder_onnx;

#[cfg(feature = "onnx")]
pub mod embedder_jina_colbert;

#[cfg(feature = "onnx")]
pub mod embedder_jina_code;

#[cfg(feature = "coreml")]
pub mod embedder_coreml;

#[cfg(feature = "cuda")]
pub mod embedder_cuda;

#[cfg(feature = "openvino")]
pub mod embedder_openvino;

#[cfg(feature = "tensorrt")]
pub mod embedder_tensorrt;

#[cfg(feature = "ocr")]
pub mod ocr;

#[cfg(feature = "document-processing")]
pub mod table_detector;

#[cfg(feature = "audio-transcription")]
pub mod whisper;

#[cfg(feature = "clip")]
pub mod embedder_clip;

pub mod embedder_unixcoder;
pub mod multi_embedder;

// Re-exports
pub use chunker::chunk_document;
pub use code_preprocessor::{
    is_code_file, looks_like_code_query, preprocess_code, preprocess_query, CodeLanguage,
};
pub use dedup::BloomDedup;
pub use document::{
    extract_frontmatter, format_frontmatter_header, is_markdown_file, process_markdown_frontmatter,
    read_file_content, Frontmatter,
};

#[cfg(feature = "audio-transcription")]
pub use document::{transcribe_audio_file, transcribe_audio_file_with_transcriber};

#[cfg(feature = "audio-transcription")]
pub use whisper::{Transcriber, WhisperModel};

#[cfg(feature = "clip")]
pub use document::{embed_image_file, embed_image_file_with_embedder, ImageEmbedding};

#[cfg(feature = "clip")]
pub use embedder_clip::{ClipEmbedder, CLIP_DIM, CLIP_IMAGE_SIZE};

pub use embedder::{
    embeddings_to_vec, load_embedder_from_env, maxsim, maxsim_from_vecs, similarity_from_vecs,
    BackendEmbedder, Embedder, EmbedderBackend, EmbedderBackendKind, EmbeddingModel,
    EmbeddingResult, EMBEDDER_BACKEND_ENV, EMBEDDER_MODEL_ENV, EMBEDDING_DIM,
};
#[cfg(feature = "onnx")]
pub use embedder_jina_code::{JinaCodeEmbedder, JINA_CODE_DIM};
#[cfg(feature = "onnx")]
pub use embedder_jina_colbert::{JinaColBertEmbedder, JINA_COLBERT_DIM};
pub use embedder_unixcoder::UNIXCODER_DIM;
pub use encoding::{
    decode_to_utf8, decode_with_encoding, detect_encoding, is_valid_text_encoding, read_text_file,
    read_text_file_utf8, DetectedEncoding,
};
pub use file_types::{
    detect_file_type, detect_file_type_from_buffer, is_audio_file, is_binary_file,
    is_document_file, is_image_file, is_indexable_by_content, is_indexable_path, is_media_file,
    is_text_file, is_video_file, validate_text_file, DetectedFileType,
};
pub use hnsw::{HnswGraph, MIN_NODES_FOR_HNSW};
pub use index::{compute_adaptive_cluster_count, IndexHealthMetrics, LazyIndex};
pub use multi_embedder::{
    detect_optimal_model, detect_optimal_model_with_stats, ContentType, ModelDetectionResult,
    MultiEmbedder,
};
pub use quantizer::{DistanceTable, ProductQuantizer, QuantizedEmbedding, NUM_SUBSPACES};
pub use query_cache::{CachedEmbedding, QueryCache, DEFAULT_CACHE_SIZE};
pub use search::{
    get_index_state_path, index_directory, index_directory_backend, index_directory_with_options,
    index_directory_with_options_backend, index_file, index_file_backend, is_system_temp_path,
    load_or_create_bloom_filter, load_or_create_index, optimal_batch_size, populate_lazy_index,
    save_index_state, search, search_backend, search_cached, search_clustered,
    search_clustered_backend, semantic_search_backend, semantic_search_cached, should_skip_dir,
    IndexDirectoryOptions, IndexStats, SearchOptions, SearchResult, DEFAULT_CROSSFILE_BATCH_SIZE,
};
pub use rerank::{LLMReranker, NoOpReranker, RerankOptions, Reranker, ScoreBoostReranker};

pub use memory::{
    format_bytes, format_bytes_signed, get_memory_stats, get_rss_bytes, reset_memory_stats,
    MemoryDelta, MemoryGuard, MemoryStats, TrackingAllocator,
};
#[cfg(feature = "clip")]
pub use search::{
    index_image, index_images_in_directory, search_images, ImageIndexStats, ImageSearchOptions,
    ImageSearchResult,
};
pub use storage::{
    CompactionStats, Document, DocumentSummary, FileSummaryRecord, ImageEmbeddingRecord,
    ImageRecord, ImageSummary, DB,
};
pub use summary::{
    detect_mime, extract_first_line, extract_first_line_from_content, get_summary,
    get_summary_from_content, read_xattr, store_summary, write_xattr, xattr_supported,
    FileSummary, SummarySource, MAX_SUMMARY_LEN, XATTR_KEY,
};

/// Create the appropriate compute device for the current platform
pub fn make_device() -> candle_core::Device {
    #[cfg(target_os = "macos")]
    {
        candle_core::Device::new_metal(0).unwrap_or(candle_core::Device::Cpu)
    }
    #[cfg(not(target_os = "macos"))]
    {
        candle_core::Device::Cpu
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_make_device() {
        let device = make_device();
        // On macOS, it should return Metal or CPU
        // On other platforms, it should return CPU
        #[cfg(target_os = "macos")]
        {
            // Accept either Metal or CPU (Metal may fail on some systems)
            assert!(device.is_metal() || device.is_cpu());
        }
        #[cfg(not(target_os = "macos"))]
        {
            assert!(device.is_cpu());
        }
    }

    #[test]
    fn test_reexports_compile() {
        let _ = EMBEDDING_DIM;
        let _doc = Document {
            id: 1,
            path: "/tmp/doc.rs".to_string(),
            hash: "hash".to_string(),
            content: "content".to_string(),
            line_count: 1,
        };
        let _db = DB::in_memory().unwrap();
        let _search_options = SearchOptions::default();
        let _index_options = IndexDirectoryOptions::default();
        let _stats = IndexStats::default();
        let _result = SearchResult::default();
        let _lazy_index = LazyIndex::new(0);

        let _search_fn: fn(
            &DB,
            &mut Embedder,
            &str,
            SearchOptions,
        ) -> anyhow::Result<Vec<SearchResult>> = search;
        let _search_clustered_fn: fn(
            &DB,
            &LazyIndex,
            &mut Embedder,
            &str,
            SearchOptions,
        ) -> anyhow::Result<Vec<SearchResult>> = search_clustered;
        let _index_file_fn: fn(&DB, &mut Embedder, &Path) -> anyhow::Result<Option<u32>> =
            index_file;
        let _index_directory_fn: fn(
            &DB,
            &mut Embedder,
            &Path,
            Option<&indicatif::ProgressBar>,
        ) -> anyhow::Result<IndexStats> = index_directory;
        let _index_directory_with_options_fn: fn(
            &DB,
            &mut Embedder,
            &Path,
            Option<&indicatif::ProgressBar>,
            IndexDirectoryOptions,
        ) -> anyhow::Result<IndexStats> = index_directory_with_options;
        let _populate_lazy_index_fn: fn(&DB, &mut LazyIndex) -> anyhow::Result<usize> =
            populate_lazy_index;
        let _is_system_temp_path_fn: fn(&Path) -> bool = is_system_temp_path;
        let _should_skip_dir_fn: fn(&str) -> bool = should_skip_dir;
        let _maxsim_fn: fn(&candle_core::Tensor, &candle_core::Tensor) -> anyhow::Result<f32> =
            maxsim;
    }
}
