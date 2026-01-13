//! Storage layer for video and audio extraction system
//!
//! This module provides interfaces and implementations for storing extracted data:
//! - **Object Storage (S3/MinIO)**: Raw media files, extracted audio, keyframes, thumbnails
//! - **Vector Database (Qdrant)**: Embeddings for semantic search
//! - **Metadata Database (`PostgreSQL`)**: Structured data, timelines, relationships
//!
//! # Architecture
//!
//! The storage layer follows the three-tier architecture from `AI_TECHNICAL_SPEC.md` section 1.4:
//! - Object storage for large binary assets (video, audio, images)
//! - Vector database for similarity search on embeddings
//! - Relational database for structured metadata and relationships
//!
//! # Example
//!
//! ```rust,no_run
//! use video_audio_storage::{StorageConfig, ObjectStorage, S3ObjectStorage};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = StorageConfig::default();
//!     let storage = S3ObjectStorage::new(config.s3).await?;
//!
//!     // Store a file
//!     let data = b"audio data...";
//!     storage.store_file("job123/audio.wav", data).await?;
//!
//!     // Retrieve a file
//!     let retrieved = storage.retrieve_file("job123/audio.wav").await?;
//!
//!     Ok(())
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

pub mod metadata_storage;
pub mod object_storage;
pub mod vector_storage;

pub use metadata_storage::{MetadataStorage, PostgresConfig, PostgresMetadataStorage};
pub use object_storage::{ObjectStorage, S3Config, S3ObjectStorage};
pub use vector_storage::{QdrantConfig, QdrantVectorStorage, VectorStorage};

/// Storage layer errors
#[derive(Error, Debug)]
pub enum StorageError {
    #[error("S3 error: {0}")]
    S3Error(String),

    #[error("Qdrant error: {0}")]
    QdrantError(String),

    #[error("PostgreSQL error: {0}")]
    PostgresError(String),

    #[error("File not found: {0}")]
    NotFound(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Other error: {0}")]
    Other(String),
}

/// Result type for storage operations
pub type StorageResult<T> = Result<T, StorageError>;

/// Complete storage configuration for all backends
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StorageConfig {
    /// S3/MinIO configuration for object storage
    #[serde(default)]
    pub s3: S3Config,

    /// Qdrant configuration for vector storage
    #[serde(default)]
    pub qdrant: QdrantConfig,

    /// `PostgreSQL` configuration for metadata storage
    #[serde(default)]
    pub postgres: PostgresConfig,
}

/// Media metadata stored in `PostgreSQL`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaMetadata {
    /// Unique job ID
    pub job_id: String,

    /// Original file path or URL
    pub input_path: String,

    /// Media format (container)
    pub format: String,

    /// Duration in seconds
    pub duration_secs: f64,

    /// Number of streams
    pub num_streams: usize,

    /// Width x Height for video
    pub resolution: Option<(u32, u32)>,

    /// Frame rate for video
    pub frame_rate: Option<f64>,

    /// Audio sample rate
    pub sample_rate: Option<u32>,

    /// Number of audio channels
    pub audio_channels: Option<u16>,

    /// Processing timestamp
    pub processed_at: chrono::DateTime<chrono::Utc>,

    /// Additional metadata
    pub extra: HashMap<String, String>,
}

/// Transcription segment stored in `PostgreSQL`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionSegment {
    /// Parent job ID
    pub job_id: String,

    /// Segment index
    pub segment_id: usize,

    /// Start time in seconds
    pub start_time: f64,

    /// End time in seconds
    pub end_time: f64,

    /// Transcribed text
    pub text: String,

    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,

    /// Speaker ID (if diarization available)
    pub speaker_id: Option<String>,
}

/// Object detection result stored in `PostgreSQL`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionResult {
    /// Parent job ID
    pub job_id: String,

    /// Frame index or timestamp
    pub frame_id: String,

    /// Object class ID
    pub class_id: u32,

    /// Object class name
    pub class_name: String,

    /// Detection confidence (0.0 to 1.0)
    pub confidence: f32,

    /// Bounding box (normalized coordinates: x, y, width, height)
    pub bbox: (f32, f32, f32, f32),
}

/// Embedding vector for semantic search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingVector {
    /// Parent job ID
    pub job_id: String,

    /// Vector ID (unique within job)
    pub vector_id: String,

    /// Embedding type (e.g., "`clip_frame`", "`sentence_text`")
    pub embedding_type: String,

    /// High-dimensional vector
    pub vector: Vec<f32>,

    /// Associated metadata (`frame_id`, `segment_id`, etc.)
    pub metadata: HashMap<String, String>,
}

/// Timeline entry representing an event or extracted feature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEntry {
    /// Parent job ID
    pub job_id: String,

    /// Entry type (e.g., "transcription", "detection", "keyframe")
    pub entry_type: String,

    /// Start time in seconds
    pub start_time: f64,

    /// End time in seconds (same as `start_time` for instantaneous events)
    pub end_time: f64,

    /// Entry data (JSON)
    pub data: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_config_default() {
        let config = StorageConfig::default();
        assert_eq!(config.s3.bucket, "video-audio-extracts");
        assert_eq!(config.qdrant.collection, "media_embeddings");
        assert_eq!(config.postgres.database, "video_audio_extracts");
    }

    #[test]
    fn test_media_metadata_creation() {
        let metadata = MediaMetadata {
            job_id: "test-job".to_string(),
            input_path: "/path/to/video.mp4".to_string(),
            format: "mp4".to_string(),
            duration_secs: 120.5,
            num_streams: 2,
            resolution: Some((1920, 1080)),
            frame_rate: Some(30.0),
            sample_rate: Some(48000),
            audio_channels: Some(2),
            processed_at: chrono::Utc::now(),
            extra: HashMap::new(),
        };

        assert_eq!(metadata.job_id, "test-job");
        assert_eq!(metadata.format, "mp4");
        assert_eq!(metadata.resolution, Some((1920, 1080)));
    }

    #[test]
    fn test_transcription_segment_creation() {
        let segment = TranscriptionSegment {
            job_id: "test-job".to_string(),
            segment_id: 0,
            start_time: 0.0,
            end_time: 5.2,
            text: "Hello world".to_string(),
            confidence: 0.95,
            speaker_id: Some("speaker_1".to_string()),
        };

        assert_eq!(segment.segment_id, 0);
        assert_eq!(segment.text, "Hello world");
        assert_eq!(segment.confidence, 0.95);
    }

    #[test]
    fn test_detection_result_creation() {
        let detection = DetectionResult {
            job_id: "test-job".to_string(),
            frame_id: "frame_0".to_string(),
            class_id: 0,
            class_name: "person".to_string(),
            confidence: 0.87,
            bbox: (0.1, 0.2, 0.3, 0.4),
        };

        assert_eq!(detection.class_name, "person");
        assert_eq!(detection.bbox, (0.1, 0.2, 0.3, 0.4));
    }

    #[test]
    fn test_embedding_vector_creation() {
        let embedding = EmbeddingVector {
            job_id: "test-job".to_string(),
            vector_id: "vec_0".to_string(),
            embedding_type: "clip_frame".to_string(),
            vector: vec![0.1, 0.2, 0.3],
            metadata: HashMap::new(),
        };

        assert_eq!(embedding.embedding_type, "clip_frame");
        assert_eq!(embedding.vector.len(), 3);
    }

    #[test]
    fn test_timeline_entry_creation() {
        let entry = TimelineEntry {
            job_id: "test-job".to_string(),
            entry_type: "transcription".to_string(),
            start_time: 0.0,
            end_time: 5.2,
            data: serde_json::json!({"text": "Hello world"}),
        };

        assert_eq!(entry.entry_type, "transcription");
        assert_eq!(entry.start_time, 0.0);
    }
}
