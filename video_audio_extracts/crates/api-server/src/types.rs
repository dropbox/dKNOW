//! API request and response types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Source of media file to process
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum MediaSource {
    /// URL to download media from
    #[serde(rename = "url")]
    Url { location: String },
    /// File already uploaded (local path)
    #[serde(rename = "upload")]
    Upload { location: String },
    /// S3 bucket location
    #[serde(rename = "s3")]
    S3 { location: String },
}

/// Processing priority mode
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProcessingPriority {
    /// Real-time processing (minimum latency)
    Realtime,
    /// Bulk processing (maximum throughput)
    Bulk,
}

/// Quality mode for processing
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum QualityMode {
    /// Fast processing with lower quality
    Fast,
    /// Balanced quality and speed
    Balanced,
    /// High accuracy with slower processing
    Accurate,
}

/// Processing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingConfig {
    /// Processing priority
    pub priority: ProcessingPriority,
    /// Required features (must succeed)
    #[serde(default)]
    pub required_features: Vec<String>,
    /// Optional features (may fail without failing job)
    #[serde(default)]
    pub optional_features: Vec<String>,
    /// Quality mode
    #[serde(default = "default_quality_mode")]
    pub quality_mode: QualityMode,
}

fn default_quality_mode() -> QualityMode {
    QualityMode::Balanced
}

/// Streaming configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingConfig {
    /// Whether streaming is enabled
    pub enabled: bool,
    /// Streaming protocol (sse or websocket)
    #[serde(default = "default_protocol")]
    pub protocol: String,
}

fn default_protocol() -> String {
    "sse".to_string()
}

/// Real-time processing request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeRequest {
    /// Media source
    pub source: MediaSource,
    /// Processing configuration
    pub processing: ProcessingConfig,
    /// Streaming configuration (optional)
    #[serde(default)]
    pub streaming: Option<StreamingConfig>,
}

/// File in a bulk batch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkFile {
    /// Unique file identifier
    pub id: String,
    /// Media source
    pub source: MediaSource,
    /// Per-file processing configuration
    pub processing: ProcessingConfig,
}

/// Bulk batch configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkConfig {
    /// Batch priority
    pub priority: ProcessingPriority,
    /// Optimization target
    #[serde(default = "default_optimize_for")]
    pub optimize_for: String,
    /// Callback URL for completion notification (optional)
    #[serde(default)]
    pub callback_url: Option<String>,
}

fn default_optimize_for() -> String {
    "throughput".to_string()
}

/// Bulk processing request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkRequest {
    /// Batch identifier
    pub batch_id: String,
    /// Files to process
    pub files: Vec<BulkFile>,
    /// Batch configuration
    pub batch_config: BulkConfig,
}

/// Job status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    /// Job is queued
    Queued,
    /// Job is running
    Running,
    /// Job completed successfully
    Completed,
    /// Job failed
    Failed,
}

/// Real-time processing response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeResponse {
    /// Job identifier
    pub job_id: String,
    /// Job status
    pub status: JobStatus,
    /// Status message
    #[serde(default)]
    pub message: Option<String>,
}

/// Bulk processing response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkResponse {
    /// Batch identifier
    pub batch_id: String,
    /// Job identifiers for each file
    pub job_ids: Vec<String>,
    /// Status message
    pub message: String,
}

/// Job status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobStatusResponse {
    /// Job identifier
    pub job_id: String,
    /// Current status
    pub status: JobStatus,
    /// Total tasks
    pub total_tasks: usize,
    /// Completed tasks
    pub completed_tasks: usize,
    /// Failed tasks
    pub failed_tasks: usize,
    /// Error message (if failed)
    #[serde(default)]
    pub error: Option<String>,
}

/// Job result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResult {
    /// Job identifier
    pub job_id: String,
    /// Job status
    pub status: JobStatus,
    /// Processing results (`task_id` -> result)
    pub results: HashMap<String, serde_json::Value>,
    /// Error message (if failed)
    #[serde(default)]
    pub error: Option<String>,
}

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Service status
    pub status: String,
    /// Service version
    pub version: String,
}

/// Query modality for semantic search
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum QueryModality {
    /// Text query (e.g., "people walking on beach")
    #[serde(rename = "text")]
    Text { query: String },
    /// Image query (base64-encoded image or URL)
    #[serde(rename = "image")]
    Image { location: MediaSource },
    /// Audio query (base64-encoded audio or URL)
    #[serde(rename = "audio")]
    Audio { location: MediaSource },
}

/// Semantic search request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    /// Query (text, image, or audio)
    pub query: QueryModality,
    /// Maximum number of results to return
    #[serde(default = "default_search_limit")]
    pub limit: usize,
    /// Filter by embedding type (e.g., "`clip_frame`", "`sentence_text`", "`clap_audio`")
    #[serde(default)]
    pub embedding_type: Option<String>,
    /// Filter by job ID
    #[serde(default)]
    pub job_id: Option<String>,
    /// Include embedding vectors in results
    #[serde(default)]
    pub include_vectors: bool,
}

fn default_search_limit() -> usize {
    10
}

/// Search result item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultItem {
    /// Vector ID
    pub vector_id: String,
    /// Similarity score (0.0 to 1.0, higher is more similar)
    pub score: f32,
    /// Job ID this vector belongs to
    pub job_id: String,
    /// Embedding type
    pub embedding_type: String,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
    /// Embedding vector (if requested)
    #[serde(default)]
    pub vector: Option<Vec<f32>>,
}

/// Semantic search response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    /// Search results (ordered by similarity)
    pub results: Vec<SearchResultItem>,
    /// Number of results returned
    pub count: usize,
    /// Query modality
    pub query_type: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_media_source_url_serialization() {
        let source = MediaSource::Url {
            location: "https://example.com/video.mp4".to_string(),
        };
        let json = serde_json::to_string(&source).unwrap();
        assert!(json.contains("url"));
        assert!(json.contains("example.com"));
    }

    #[test]
    fn test_processing_priority_serialization() {
        let priority = ProcessingPriority::Realtime;
        let json = serde_json::to_string(&priority).unwrap();
        assert_eq!(json, "\"realtime\"");

        let priority = ProcessingPriority::Bulk;
        let json = serde_json::to_string(&priority).unwrap();
        assert_eq!(json, "\"bulk\"");
    }

    #[test]
    fn test_quality_mode_serialization() {
        let mode = QualityMode::Fast;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, "\"fast\"");

        let mode = QualityMode::Balanced;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, "\"balanced\"");

        let mode = QualityMode::Accurate;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, "\"accurate\"");
    }

    #[test]
    fn test_job_status_serialization() {
        let status = JobStatus::Queued;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"queued\"");

        let status = JobStatus::Running;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"running\"");

        let status = JobStatus::Completed;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"completed\"");

        let status = JobStatus::Failed;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"failed\"");
    }

    #[test]
    fn test_default_quality_mode() {
        assert_eq!(default_quality_mode(), QualityMode::Balanced);
    }

    #[test]
    fn test_default_protocol() {
        assert_eq!(default_protocol(), "sse");
    }

    #[test]
    fn test_default_optimize_for() {
        assert_eq!(default_optimize_for(), "throughput");
    }

    #[test]
    fn test_realtime_request_deserialization() {
        let json = r#"{
            "source": {
                "type": "url",
                "location": "https://example.com/video.mp4"
            },
            "processing": {
                "priority": "realtime",
                "required_features": ["transcription", "keyframes"],
                "optional_features": ["objects"],
                "quality_mode": "balanced"
            }
        }"#;

        let request: RealtimeRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.processing.priority, ProcessingPriority::Realtime);
        assert_eq!(request.processing.required_features.len(), 2);
        assert_eq!(request.processing.quality_mode, QualityMode::Balanced);
    }

    #[test]
    fn test_bulk_request_deserialization() {
        let json = r#"{
            "batch_id": "batch_123",
            "files": [
                {
                    "id": "file_1",
                    "source": {
                        "type": "s3",
                        "location": "s3://bucket/video1.mp4"
                    },
                    "processing": {
                        "priority": "bulk",
                        "required_features": ["transcription"]
                    }
                }
            ],
            "batch_config": {
                "priority": "bulk",
                "optimize_for": "throughput"
            }
        }"#;

        let request: BulkRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.batch_id, "batch_123");
        assert_eq!(request.files.len(), 1);
        assert_eq!(request.batch_config.priority, ProcessingPriority::Bulk);
    }
}
