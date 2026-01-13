//! Cache key generation and metadata

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Cache key for content-addressed storage
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CacheKey {
    /// Hash of input data
    pub input_hash: String,

    /// Plugin name
    pub plugin_name: String,

    /// Plugin version
    pub plugin_version: u32,

    /// Hash of operation parameters
    pub operation_hash: String,
}

impl CacheKey {
    /// Create a new cache key
    pub fn new(
        input_data: &[u8],
        plugin_name: impl Into<String>,
        plugin_version: u32,
        operation_data: &[u8],
    ) -> Self {
        let input_hash = blake3::hash(input_data);
        let operation_hash = blake3::hash(operation_data);

        Self {
            input_hash: input_hash.to_hex().to_string(),
            plugin_name: plugin_name.into(),
            plugin_version,
            operation_hash: operation_hash.to_hex().to_string(),
        }
    }

    /// Create a key string for filesystem storage
    pub fn to_key_string(&self) -> String {
        format!(
            "{}_{}_{}_{}",
            self.plugin_name, self.plugin_version, self.input_hash, self.operation_hash
        )
    }

    /// Parse a key string back into a CacheKey
    pub fn from_key_string(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('_').take(4).collect();
        if parts.len() != 4 {
            return None;
        }

        Some(Self {
            plugin_name: parts[0].to_string(),
            plugin_version: parts[1].parse().ok()?,
            input_hash: parts[2].to_string(),
            operation_hash: parts[3].to_string(),
        })
    }
}

/// Metadata about cached results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetadata {
    /// When this was cached
    pub created_at: SystemTime,

    /// Plugin version that created this
    pub plugin_version: u32,

    /// Size of cached data in bytes
    pub size_bytes: u64,
}

impl CacheMetadata {
    /// Create new cache metadata
    pub fn new(plugin_version: u32, size_bytes: u64) -> Self {
        Self {
            created_at: SystemTime::now(),
            plugin_version,
            size_bytes,
        }
    }

    /// Get age of cache entry
    pub fn age(&self) -> std::time::Duration {
        SystemTime::now()
            .duration_since(self.created_at)
            .unwrap_or_default()
    }
}

// ============================================================================
// In-Memory Pipeline Cache
// ============================================================================

use crate::plugin::PluginData;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

/// Thread-safe in-memory cache for pipeline intermediate results
///
/// This cache eliminates duplicate work within a single pipeline execution.
/// Key use case: Avoid re-extracting keyframes when multiple plugins
/// (object_detection, face_detection, vision_embeddings) all need keyframes
/// from the same video.
#[derive(Clone)]
pub struct PipelineCache {
    /// Cached results keyed by (plugin_name, operation_name, input_hash)
    results: Arc<Mutex<HashMap<PipelineCacheKey, CachedPluginResult>>>,

    /// Memory limit in bytes (0 = unlimited)
    max_memory_bytes: usize,

    /// Current memory usage estimate
    memory_usage: Arc<Mutex<usize>>,
}

/// Key for pipeline cache lookup
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PipelineCacheKey {
    /// Plugin name
    plugin_name: String,

    /// Operation name
    operation_name: String,

    /// Hash of input data
    input_hash: u64,
}

/// Cached result with metadata
#[derive(Debug, Clone)]
struct CachedPluginResult {
    /// The actual output data
    data: PluginData,

    /// Estimated memory usage in bytes
    estimated_size: usize,

    /// When this was cached
    cached_at: SystemTime,
}

impl PipelineCache {
    /// Create a new empty cache with unlimited memory
    pub fn new() -> Self {
        Self::with_memory_limit(0)
    }

    /// Create a cache with a memory limit (in bytes)
    /// When limit is reached, oldest entries are evicted (LRU-like)
    pub fn with_memory_limit(max_memory_bytes: usize) -> Self {
        Self {
            results: Arc::new(Mutex::new(HashMap::with_capacity(50))),
            max_memory_bytes,
            memory_usage: Arc::new(Mutex::new(0)),
        }
    }

    /// Try to get a cached result
    pub fn get(
        &self,
        plugin_name: &str,
        operation_name: &str,
        input: &PluginData,
    ) -> Option<PluginData> {
        let key = PipelineCacheKey {
            plugin_name: plugin_name.to_string(),
            operation_name: operation_name.to_string(),
            input_hash: Self::hash_plugin_data(input),
        };

        let cache = self.results.lock().unwrap();
        cache.get(&key).map(|cached| cached.data.clone())
    }

    /// Store a result in the cache
    pub fn put(
        &self,
        plugin_name: &str,
        operation_name: &str,
        input: &PluginData,
        output: &PluginData,
    ) {
        let key = PipelineCacheKey {
            plugin_name: plugin_name.to_string(),
            operation_name: operation_name.to_string(),
            input_hash: Self::hash_plugin_data(input),
        };

        let estimated_size = Self::estimate_size(output);

        // Check if we need to evict entries to stay under memory limit
        if self.max_memory_bytes > 0 {
            self.evict_if_needed(estimated_size);
        }

        let cached = CachedPluginResult {
            data: output.clone(),
            estimated_size,
            cached_at: SystemTime::now(),
        };

        let mut cache = self.results.lock().unwrap();
        if cache.insert(key, cached).is_none() {
            // New entry added, update memory usage
            let mut memory = self.memory_usage.lock().unwrap();
            *memory += estimated_size;
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> PipelineCacheStats {
        let cache = self.results.lock().unwrap();
        let memory = self.memory_usage.lock().unwrap();

        PipelineCacheStats {
            entries: cache.len(),
            memory_bytes: *memory,
            max_memory_bytes: self.max_memory_bytes,
        }
    }

    /// Clear all cached results
    pub fn clear(&self) {
        let mut cache = self.results.lock().unwrap();
        cache.clear();

        let mut memory = self.memory_usage.lock().unwrap();
        *memory = 0;
    }

    /// Hash PluginData for cache key
    fn hash_plugin_data(data: &PluginData) -> u64 {
        use std::collections::hash_map::DefaultHasher;

        let mut hasher = DefaultHasher::new();

        match data {
            PluginData::Bytes(bytes) => {
                "Bytes".hash(&mut hasher);
                bytes.hash(&mut hasher);
            }
            PluginData::FilePath(path) => {
                "FilePath".hash(&mut hasher);
                path.hash(&mut hasher);

                // Include file metadata for better cache invalidation
                if let Ok(metadata) = std::fs::metadata(path) {
                    if let Ok(modified) = metadata.modified() {
                        format!("{:?}", modified).hash(&mut hasher);
                    }
                    metadata.len().hash(&mut hasher);
                }
            }
            PluginData::Json(value) => {
                "Json".hash(&mut hasher);
                // Hash the JSON string representation
                if let Ok(json_str) = serde_json::to_string(value) {
                    json_str.hash(&mut hasher);
                }
            }
            PluginData::Multiple(items) => {
                "Multiple".hash(&mut hasher);
                items.len().hash(&mut hasher);
                for item in items {
                    Self::hash_plugin_data(item).hash(&mut hasher);
                }
            }
        }

        hasher.finish()
    }

    /// Estimate memory usage of PluginData
    fn estimate_size(data: &PluginData) -> usize {
        match data {
            PluginData::Bytes(bytes) => bytes.len(),
            PluginData::FilePath(path) => {
                // Just the path string size (not the file)
                path.to_string_lossy().len()
            }
            PluginData::Json(value) => {
                // Rough estimate: JSON string length
                serde_json::to_string(value).map(|s| s.len()).unwrap_or(0)
            }
            PluginData::Multiple(items) => items.iter().map(Self::estimate_size).sum(),
        }
    }

    /// Evict old entries if adding new_size would exceed memory limit
    fn evict_if_needed(&self, new_size: usize) {
        let mut memory = self.memory_usage.lock().unwrap();

        if self.max_memory_bytes == 0 || *memory + new_size <= self.max_memory_bytes {
            return; // No eviction needed
        }

        // Need to evict entries - use LRU-like policy (oldest first)
        let mut cache = self.results.lock().unwrap();

        // Sort entries by cached_at timestamp and collect keys to remove
        let mut entries: Vec<_> = Vec::with_capacity(cache.len());
        entries.extend(
            cache
                .iter()
                .map(|(k, v)| (k.clone(), v.cached_at, v.estimated_size)),
        );
        entries.sort_by_key(|(_, timestamp, _)| *timestamp);

        // Evict oldest entries until we have room
        let mut freed = 0;
        let keys_to_remove: Vec<_> = entries
            .into_iter()
            .take_while(|(_, _, size)| {
                if *memory - freed + new_size <= self.max_memory_bytes {
                    false
                } else {
                    freed += *size;
                    true
                }
            })
            .map(|(key, _, _)| key)
            .collect();

        for key in keys_to_remove {
            cache.remove(&key);
        }

        *memory -= freed;
    }
}

impl Default for PipelineCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Pipeline cache statistics
#[derive(Debug, Clone)]
pub struct PipelineCacheStats {
    /// Number of cached entries
    pub entries: usize,

    /// Current memory usage (bytes)
    pub memory_bytes: usize,

    /// Maximum memory limit (0 = unlimited)
    pub max_memory_bytes: usize,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_generation() {
        let key = CacheKey::new(b"input data", "transcription", 1, b"operation params");

        assert_eq!(key.plugin_name, "transcription");
        assert_eq!(key.plugin_version, 1);
        assert!(!key.input_hash.is_empty());
        assert!(!key.operation_hash.is_empty());
    }

    #[test]
    fn test_cache_key_to_string() {
        let key = CacheKey::new(b"input data", "transcription", 1, b"operation params");

        let key_string = key.to_key_string();
        assert!(key_string.starts_with("transcription_1_"));
    }

    #[test]
    fn test_cache_key_roundtrip() {
        let key = CacheKey::new(b"input data", "transcription", 1, b"operation params");

        let key_string = key.to_key_string();
        let parsed = CacheKey::from_key_string(&key_string).unwrap();

        assert_eq!(parsed.plugin_name, key.plugin_name);
        assert_eq!(parsed.plugin_version, key.plugin_version);
        assert_eq!(parsed.input_hash, key.input_hash);
        assert_eq!(parsed.operation_hash, key.operation_hash);
    }

    #[test]
    fn test_cache_metadata_age() {
        let metadata = CacheMetadata::new(1, 1024);
        let age = metadata.age();
        // Age should be very small (just created)
        assert!(age.as_secs() < 1);
    }

    #[test]
    fn test_pipeline_cache_basic() {
        use std::path::PathBuf;
        let cache = PipelineCache::new();

        let input = PluginData::FilePath(PathBuf::from("/tmp/test.mp4"));
        let output = PluginData::Json(serde_json::json!({"frames": 100}));

        // Initially empty
        assert!(cache.get("plugin1", "op1", &input).is_none());

        // Add to cache
        cache.put("plugin1", "op1", &input, &output);

        // Should be cached now
        let cached = cache.get("plugin1", "op1", &input).unwrap();
        match cached {
            PluginData::Json(value) => {
                assert_eq!(value["frames"], 100);
            }
            _ => panic!("Wrong data type"),
        }

        // Different plugin/operation should miss
        assert!(cache.get("plugin2", "op1", &input).is_none());
        assert!(cache.get("plugin1", "op2", &input).is_none());
    }

    #[test]
    fn test_pipeline_cache_stats() {
        use std::path::PathBuf;
        let cache = PipelineCache::with_memory_limit(1024);

        let input = PluginData::FilePath(PathBuf::from("/tmp/test.mp4"));
        let output = PluginData::Bytes(vec![0u8; 100]);

        cache.put("plugin1", "op1", &input, &output);

        let stats = cache.stats();
        assert_eq!(stats.entries, 1);
        assert!(stats.memory_bytes > 0);
        assert_eq!(stats.max_memory_bytes, 1024);
    }

    #[test]
    fn test_pipeline_cache_clear() {
        use std::path::PathBuf;
        let cache = PipelineCache::new();

        let input = PluginData::FilePath(PathBuf::from("/tmp/test.mp4"));
        let output = PluginData::Bytes(vec![1, 2, 3]);

        cache.put("plugin1", "op1", &input, &output);
        assert!(cache.get("plugin1", "op1", &input).is_some());

        cache.clear();
        assert!(cache.get("plugin1", "op1", &input).is_none());

        let stats = cache.stats();
        assert_eq!(stats.entries, 0);
        assert_eq!(stats.memory_bytes, 0);
    }
}
