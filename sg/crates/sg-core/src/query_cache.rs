//! LRU cache for query embeddings
//!
//! Caches query embeddings to avoid re-embedding repeated queries.
//! Uses a simple bounded HashMap with FIFO eviction when capacity is reached.
//!
//! This is Phase 12.1 from the roadmap.

use std::collections::HashMap;

/// Default maximum cache size (number of queries)
pub const DEFAULT_CACHE_SIZE: usize = 128;

/// Result of embedding a query (cached or computed)
#[derive(Debug, Clone)]
pub struct CachedEmbedding {
    /// The embedding data (flattened float array)
    pub data: Vec<f32>,
    /// Number of tokens in the query
    pub num_tokens: usize,
}

/// LRU cache for query embeddings
///
/// Stores query string -> embedding mapping with bounded capacity.
/// When capacity is reached, oldest entries are evicted (FIFO).
#[derive(Debug)]
pub struct QueryCache {
    /// Map from query string to embedding
    entries: HashMap<String, CachedEmbedding>,
    /// Insertion order for FIFO eviction (oldest first)
    order: Vec<String>,
    /// Maximum number of entries
    capacity: usize,
    /// Cache hit count for statistics
    hits: u64,
    /// Cache miss count for statistics
    misses: u64,
}

impl Default for QueryCache {
    fn default() -> Self {
        Self::new(DEFAULT_CACHE_SIZE)
    }
}

impl QueryCache {
    /// Create a new cache with the specified capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: HashMap::with_capacity(capacity),
            order: Vec::with_capacity(capacity),
            capacity: capacity.max(1), // Minimum capacity of 1
            hits: 0,
            misses: 0,
        }
    }

    /// Get a cached embedding for a query, if present
    pub fn get(&mut self, query: &str) -> Option<&CachedEmbedding> {
        if self.entries.contains_key(query) {
            self.hits += 1;
            self.entries.get(query)
        } else {
            self.misses += 1;
            None
        }
    }

    /// Insert a new embedding into the cache
    ///
    /// If the cache is at capacity, evicts the oldest entry.
    pub fn insert(&mut self, query: String, embedding: CachedEmbedding) {
        // If already present, just update
        if let std::collections::hash_map::Entry::Occupied(mut e) =
            self.entries.entry(query.clone())
        {
            e.insert(embedding);
            return;
        }

        // Evict oldest entries if at capacity
        while self.order.len() >= self.capacity {
            if let Some(oldest) = self.order.first().cloned() {
                self.entries.remove(&oldest);
                self.order.remove(0);
            }
        }

        // Insert new entry
        self.order.push(query.clone());
        self.entries.insert(query, embedding);
    }

    /// Clear all cached entries
    pub fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
    }

    /// Get the number of cached entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get cache hit count
    pub fn hits(&self) -> u64 {
        self.hits
    }

    /// Get cache miss count
    pub fn misses(&self) -> u64 {
        self.misses
    }

    /// Get cache hit rate (0.0 to 1.0)
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    /// Get the cache capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_embedding(data: Vec<f32>, num_tokens: usize) -> CachedEmbedding {
        CachedEmbedding { data, num_tokens }
    }

    #[test]
    fn test_cache_basic() {
        let mut cache = QueryCache::new(10);
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);

        let emb = make_embedding(vec![1.0, 2.0, 3.0], 3);
        cache.insert("hello".to_string(), emb.clone());

        assert_eq!(cache.len(), 1);
        assert!(!cache.is_empty());

        let result = cache.get("hello");
        assert!(result.is_some());
        assert_eq!(result.unwrap().data, vec![1.0, 2.0, 3.0]);
        assert_eq!(result.unwrap().num_tokens, 3);
    }

    #[test]
    fn test_cache_miss() {
        let mut cache = QueryCache::new(10);
        let emb = make_embedding(vec![1.0], 1);
        cache.insert("hello".to_string(), emb);

        let result = cache.get("world");
        assert!(result.is_none());
    }

    #[test]
    fn test_cache_eviction() {
        let mut cache = QueryCache::new(3);

        cache.insert("a".to_string(), make_embedding(vec![1.0], 1));
        cache.insert("b".to_string(), make_embedding(vec![2.0], 1));
        cache.insert("c".to_string(), make_embedding(vec![3.0], 1));

        assert_eq!(cache.len(), 3);

        // Adding a fourth should evict "a"
        cache.insert("d".to_string(), make_embedding(vec![4.0], 1));

        assert_eq!(cache.len(), 3);
        assert!(cache.get("a").is_none()); // "a" was evicted
        assert!(cache.get("b").is_some());
        assert!(cache.get("c").is_some());
        assert!(cache.get("d").is_some());
    }

    #[test]
    fn test_cache_update() {
        let mut cache = QueryCache::new(10);

        cache.insert("query".to_string(), make_embedding(vec![1.0], 1));
        cache.insert("query".to_string(), make_embedding(vec![2.0], 2));

        assert_eq!(cache.len(), 1);
        let result = cache.get("query").unwrap();
        assert_eq!(result.data, vec![2.0]);
        assert_eq!(result.num_tokens, 2);
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = QueryCache::new(10);
        cache.insert("a".to_string(), make_embedding(vec![1.0], 1));
        cache.insert("b".to_string(), make_embedding(vec![2.0], 1));

        assert_eq!(cache.len(), 2);

        cache.clear();

        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_statistics() {
        let mut cache = QueryCache::new(10);
        cache.insert("a".to_string(), make_embedding(vec![1.0], 1));

        // Miss
        cache.get("b");
        assert_eq!(cache.misses(), 1);
        assert_eq!(cache.hits(), 0);

        // Hit
        cache.get("a");
        assert_eq!(cache.misses(), 1);
        assert_eq!(cache.hits(), 1);

        // Another hit
        cache.get("a");
        assert_eq!(cache.hits(), 2);

        assert!((cache.hit_rate() - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_default_cache() {
        let cache = QueryCache::default();
        assert_eq!(cache.capacity(), DEFAULT_CACHE_SIZE);
    }

    #[test]
    fn test_minimum_capacity() {
        let cache = QueryCache::new(0);
        assert_eq!(cache.capacity(), 1); // Minimum enforced
    }
}
