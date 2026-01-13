//! Plugin trait and related types

use crate::cache::CacheMetadata;
use crate::error::PluginError;
use crate::{Context, Operation};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Core plugin trait - all plugins must implement this
#[async_trait]
pub trait Plugin: Send + Sync {
    /// Unique plugin identifier
    fn name(&self) -> &str;

    /// Get plugin configuration
    fn config(&self) -> &PluginConfig;

    /// Check if this plugin can handle the given input type
    fn supports_input(&self, input_type: &str) -> bool;

    /// Check if this plugin produces the given output type
    fn produces_output(&self, output_type: &str) -> bool;

    /// Execute the plugin operation
    async fn execute(
        &self,
        ctx: &Context,
        request: &PluginRequest,
    ) -> Result<PluginResponse, PluginError>;

    /// Validate cached result is still valid
    fn is_valid_cache_hit(
        &self,
        _cached_response: &PluginResponse,
        cache_metadata: &CacheMetadata,
    ) -> bool {
        // Default: check plugin version and timestamp
        let config = self.config();
        cache_metadata.plugin_version >= config.cache.version
            && cache_metadata.created_at >= config.cache.invalidate_before
    }

    /// Optional: Streaming execution for real-time results
    async fn execute_streaming(
        &self,
        ctx: &Context,
        request: &PluginRequest,
    ) -> Result<PluginStreamingResponse, PluginError> {
        // Default: fall back to buffered execution
        let response = self.execute(ctx, request).await?;
        Ok(PluginStreamingResponse::Complete(response))
    }
}

/// Plugin configuration loaded from YAML manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    /// Plugin name
    pub name: String,

    /// Description
    pub description: String,

    /// Supported input format extensions
    pub inputs: Vec<String>,

    /// Output types produced
    pub outputs: Vec<String>,

    /// Runtime configuration
    pub config: RuntimeConfig,

    /// Performance characteristics
    pub performance: PerformanceConfig,

    /// Cache configuration
    pub cache: CacheConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    /// Maximum input file size in MB
    pub max_file_size_mb: u64,

    /// Whether GPU is required
    pub requires_gpu: bool,

    /// Whether this is experimental
    pub experimental: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Average processing time per GB of input
    pub avg_processing_time_per_gb: String,

    /// Memory required per file in MB
    pub memory_per_file_mb: u64,

    /// Whether streaming is supported
    pub supports_streaming: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Whether caching is enabled
    pub enabled: bool,

    /// Cache version (increment to invalidate all caches)
    pub version: u32,

    /// Invalidate cache entries before this date
    #[serde(with = "systemtime_serde")]
    pub invalidate_before: SystemTime,
}

/// Request passed to plugin execution
#[derive(Debug, Clone)]
pub struct PluginRequest {
    /// The operation to perform
    pub operation: Operation,

    /// Input data (could be raw bytes, file path, etc.)
    pub input: PluginData,
}

/// Response from plugin execution
#[derive(Debug, Clone)]
pub struct PluginResponse {
    /// Output data
    pub output: PluginData,

    /// Processing duration
    pub duration: std::time::Duration,

    /// Any warnings or notes
    pub warnings: Vec<String>,
}

/// Data passed between plugins
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginData {
    /// Raw bytes
    Bytes(Vec<u8>),

    /// File path reference
    FilePath(std::path::PathBuf),

    /// JSON value
    Json(serde_json::Value),

    /// Multiple outputs (e.g., keyframes)
    Multiple(Vec<PluginData>),
}

/// Streaming response for real-time output
#[derive(Debug, Clone)]
pub enum PluginStreamingResponse {
    /// Partial result (e.g., transcript segment, detected object)
    Partial(PartialResult),

    /// Final complete result
    Complete(PluginResponse),
}

/// Partial result during streaming execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialResult {
    /// Type of partial result
    pub result_type: String,

    /// Partial data
    pub data: serde_json::Value,

    /// Progress (0.0 to 1.0)
    pub progress: Option<f32>,
}

// Custom serialization for SystemTime
mod systemtime_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::{SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let duration = time.duration_since(UNIX_EPOCH).unwrap();
        serializer.serialize_u64(duration.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(UNIX_EPOCH + std::time::Duration::from_secs(secs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_data_serialization() {
        let data = PluginData::Bytes(vec![1, 2, 3, 4]);
        let json = serde_json::to_string(&data).unwrap();
        let deserialized: PluginData = serde_json::from_str(&json).unwrap();

        match deserialized {
            PluginData::Bytes(bytes) => assert_eq!(bytes, vec![1, 2, 3, 4]),
            _ => panic!("Expected Bytes variant"),
        }
    }

    #[test]
    fn test_partial_result() {
        let partial = PartialResult {
            result_type: "transcript_segment".to_string(),
            data: serde_json::json!({"text": "Hello world"}),
            progress: Some(0.5),
        };

        assert_eq!(partial.result_type, "transcript_segment");
        assert_eq!(partial.progress, Some(0.5));
    }
}
