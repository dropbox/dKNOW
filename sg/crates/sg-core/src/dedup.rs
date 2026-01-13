//! Bloom filter for fast duplicate content detection
//!
//! Uses a Bloom filter to quickly check if a chunk's content_hash might already
//! exist in the index. This enables:
//! - **Cross-file dedup**: Reuse embeddings from ANY document with matching content
//! - **Fast existence checks**: Avoid DB queries for definitely-new chunks
//!
//! The filter has a small false positive rate (~1%), meaning sometimes we'll
//! query the DB for a hash that doesn't exist. But false negatives are impossible,
//! so we never miss an existing hash.

use bloomfilter::Bloom;

/// Expected number of items (chunk hashes) in a typical index
const DEFAULT_ITEMS: usize = 100_000;

/// Target false positive rate (1%)
const FALSE_POSITIVE_RATE: f64 = 0.01;

/// Bloom filter for content hash deduplication
///
/// Provides O(1) probabilistic existence checks for chunk content hashes.
/// Used during indexing to quickly determine if a chunk might already exist,
/// enabling cross-file embedding reuse.
pub struct BloomDedup {
    filter: Bloom<String>,
    /// Number of items added (for stats)
    items_count: usize,
}

impl BloomDedup {
    /// Create a new empty Bloom filter with default capacity
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_ITEMS)
    }

    /// Create a new Bloom filter with specified expected item count
    pub fn with_capacity(expected_items: usize) -> Self {
        let filter = Bloom::new_for_fp_rate(expected_items.max(1), FALSE_POSITIVE_RATE);
        Self {
            filter,
            items_count: 0,
        }
    }

    /// Add a content hash to the filter
    pub fn add(&mut self, content_hash: &str) {
        self.filter.set(&content_hash.to_string());
        self.items_count += 1;
    }

    /// Check if a content hash might exist in the index
    ///
    /// Returns:
    /// - `false`: Definitely NOT in index (can skip DB query)
    /// - `true`: MIGHT be in index (should query DB to confirm)
    pub fn might_contain(&self, content_hash: &str) -> bool {
        self.filter.check(&content_hash.to_string())
    }

    /// Number of items added to the filter
    pub fn len(&self) -> usize {
        self.items_count
    }

    /// Check if filter is empty
    pub fn is_empty(&self) -> bool {
        self.items_count == 0
    }

    /// Clear all items from the filter
    pub fn clear(&mut self) {
        self.filter.clear();
        self.items_count = 0;
    }

    /// Serialize the filter to bytes for persistence
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Header: items_count, num_bits, k_num
        bytes.extend_from_slice(&(self.items_count as u64).to_le_bytes());
        bytes.extend_from_slice(&self.filter.number_of_bits().to_le_bytes());
        bytes.extend_from_slice(&self.filter.number_of_hash_functions().to_le_bytes());

        // SIP keys for hash functions
        let sip_keys = self.filter.sip_keys();
        bytes.extend_from_slice(&sip_keys[0].0.to_le_bytes());
        bytes.extend_from_slice(&sip_keys[0].1.to_le_bytes());
        bytes.extend_from_slice(&sip_keys[1].0.to_le_bytes());
        bytes.extend_from_slice(&sip_keys[1].1.to_le_bytes());

        // Bitmap data
        let bitmap = self.filter.bitmap();
        bytes.extend_from_slice(&(bitmap.len() as u64).to_le_bytes());
        bytes.extend_from_slice(&bitmap);

        bytes
    }

    /// Deserialize the filter from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        // Minimum size: header (8+8+4) + sip keys (32) + bitmap len (8) = 60 bytes
        if bytes.len() < 60 {
            return None;
        }

        let mut cursor = 0;

        // Read header
        let items_count = u64::from_le_bytes(bytes[cursor..cursor + 8].try_into().ok()?) as usize;
        cursor += 8;

        let num_bits = u64::from_le_bytes(bytes[cursor..cursor + 8].try_into().ok()?);
        cursor += 8;

        let k_num = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().ok()?);
        cursor += 4;

        // Read SIP keys
        let sip0_k0 = u64::from_le_bytes(bytes[cursor..cursor + 8].try_into().ok()?);
        cursor += 8;
        let sip0_k1 = u64::from_le_bytes(bytes[cursor..cursor + 8].try_into().ok()?);
        cursor += 8;
        let sip1_k0 = u64::from_le_bytes(bytes[cursor..cursor + 8].try_into().ok()?);
        cursor += 8;
        let sip1_k1 = u64::from_le_bytes(bytes[cursor..cursor + 8].try_into().ok()?);
        cursor += 8;

        let sip_keys = [(sip0_k0, sip0_k1), (sip1_k0, sip1_k1)];

        // Read bitmap
        let bitmap_len = u64::from_le_bytes(bytes[cursor..cursor + 8].try_into().ok()?) as usize;
        cursor += 8;

        if bytes.len() < cursor + bitmap_len {
            return None;
        }

        let bitmap = &bytes[cursor..cursor + bitmap_len];

        // Reconstruct the filter
        let filter = Bloom::from_existing(bitmap, num_bits, k_num, sip_keys);

        Some(Self {
            filter,
            items_count,
        })
    }

    /// Estimated memory usage in bytes
    pub fn memory_bytes(&self) -> usize {
        self.filter.bitmap().len() + std::mem::size_of::<Self>()
    }

    /// Theoretical false positive rate based on current fill level
    pub fn current_fp_rate(&self) -> f64 {
        if self.items_count == 0 {
            return 0.0;
        }

        let m = self.filter.number_of_bits() as f64;
        let k = self.filter.number_of_hash_functions() as f64;
        let n = self.items_count as f64;

        // FP rate ≈ (1 - e^(-k*n/m))^k
        (1.0 - (-k * n / m).exp()).powf(k)
    }
}

impl Default for BloomDedup {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for BloomDedup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BloomDedup")
            .field("items_count", &self.items_count)
            .field("memory_bytes", &self.memory_bytes())
            .field(
                "fp_rate",
                &format!("{:.4}%", self.current_fp_rate() * 100.0),
            )
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_operations() {
        let mut filter = BloomDedup::new();
        assert!(filter.is_empty());
        assert_eq!(filter.len(), 0);

        // Add some hashes
        filter.add("abc123def456");
        filter.add("xyz789uvw012");

        assert!(!filter.is_empty());
        assert_eq!(filter.len(), 2);

        // Check existence
        assert!(filter.might_contain("abc123def456"));
        assert!(filter.might_contain("xyz789uvw012"));

        // Non-existent hash should (almost certainly) return false
        assert!(!filter.might_contain("nonexistent00"));
    }

    #[test]
    fn test_serialization_roundtrip() {
        let mut filter = BloomDedup::new();

        // Add some hashes
        for i in 0..100 {
            filter.add(&format!("hash_{i:08x}"));
        }

        // Serialize and deserialize
        let bytes = filter.to_bytes();
        let restored = BloomDedup::from_bytes(&bytes).expect("deserialization failed");

        // Verify state
        assert_eq!(filter.len(), restored.len());

        // Check all original hashes exist in restored filter
        for i in 0..100 {
            let hash = format!("hash_{i:08x}");
            assert!(restored.might_contain(&hash), "missing hash: {hash}");
        }
    }

    #[test]
    fn test_clear() {
        let mut filter = BloomDedup::new();

        filter.add("hash1");
        filter.add("hash2");
        assert_eq!(filter.len(), 2);

        filter.clear();
        assert!(filter.is_empty());
        assert_eq!(filter.len(), 0);

        // After clear, hashes should not be found
        // Note: might still get false positives, but very unlikely with empty filter
    }

    #[test]
    fn test_false_positive_rate() {
        let mut filter = BloomDedup::with_capacity(1000);

        // Add 1000 items
        for i in 0..1000 {
            filter.add(&format!("existing_{i:08x}"));
        }

        // Check 1000 non-existent items
        let mut false_positives = 0;
        for i in 0..1000 {
            if filter.might_contain(&format!("nonexistent_{i:08x}")) {
                false_positives += 1;
            }
        }

        // FP rate should be around 1% (allow some variance)
        let fp_rate = false_positives as f64 / 1000.0;
        assert!(
            fp_rate < 0.05, // Allow up to 5% for statistical variance
            "False positive rate too high: {:.2}%",
            fp_rate * 100.0
        );
    }

    #[test]
    fn test_memory_efficiency() {
        let filter = BloomDedup::with_capacity(100_000);

        // For 100K items at 1% FP rate, Bloom filter needs ~120KB
        // Much smaller than storing 100K hashes directly (~1.6MB for 16-char hashes)
        let mem = filter.memory_bytes();
        assert!(mem < 200_000, "Memory usage too high: {mem} bytes");
    }

    #[test]
    fn test_empty_deserialization() {
        // Too short should fail
        assert!(BloomDedup::from_bytes(&[0u8; 10]).is_none());

        // Empty valid filter should work
        let filter = BloomDedup::new();
        let bytes = filter.to_bytes();
        let restored = BloomDedup::from_bytes(&bytes).unwrap();
        assert!(restored.is_empty());
    }
}
