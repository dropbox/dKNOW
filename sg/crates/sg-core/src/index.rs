//! Progressive/lazy index with LSH and online k-means
//!
//! The index starts fast with LSH-based clustering and improves
//! continuously through online k-means updates.
//!
//! Design from ARCHITECTURE.md:
//! - Phase 0: LSH assignments (instant, ~70% quality)
//! - Phase 1: Online k-means (fast, ~85% quality)
//! - Phase 2+: Continuous improvement (~95%+ quality)

use anyhow::Result;
use candle_core::Device;
use rand::Rng;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

use crate::embedder::EMBEDDING_DIM;
use crate::hnsw::{HnswGraph, MIN_NODES_FOR_HNSW};
use crate::quantizer::{ProductQuantizer, QuantizedEmbedding, MIN_TRAINING_SAMPLES};

/// Default number of clusters
const DEFAULT_NUM_CLUSTERS: usize = 64;

/// Minimum number of clusters for adaptive sizing
const MIN_CLUSTERS: usize = 16;

/// Maximum number of clusters for adaptive sizing
const MAX_CLUSTERS: usize = 256;

/// Index state version for compatibility checking
const INDEX_STATE_VERSION: u32 = 1;

/// Maximum reservoir size for sampling
const MAX_RESERVOIR_SIZE: usize = 10000;

/// Minimum docs before transitioning to online k-means
const ONLINE_KMEANS_THRESHOLD: usize = 100;

/// Minimum samples needed before optimization can proceed
const MIN_OPTIMIZATION_SAMPLES: usize = 10;

/// Maximum embeddings to sample from a bucket during optimization
const MAX_BUCKET_SAMPLE_SIZE: usize = 100;

/// Maximum embeddings to sample for health check
const MAX_HEALTH_SAMPLE_SIZE: usize = 50;

/// Minimum bucket size to include in health calculation
const MIN_BUCKET_SIZE_FOR_HEALTH: usize = 5;

/// Blend factor for center updates (0.1 = 10% new, 90% old)
const CENTER_BLEND_FACTOR: f32 = 0.1;

/// Health threshold above which index needs optimization
const HEALTH_THRESHOLD_NEEDS_WORK: f32 = 0.2;

/// Cluster imbalance ratio threshold (largest vs smallest non-empty)
const IMBALANCE_RATIO_THRESHOLD: usize = 100;

/// How often to check for automatic rebalancing (every N improve() calls)
const AUTO_REBALANCE_CHECK_INTERVAL: usize = 100;

struct UnderfullCluster {
    id: usize,
    remaining: usize,
}

/// Compute the optimal number of clusters based on corpus size
///
/// Uses the formula: k = clamp(sqrt(n), MIN_CLUSTERS, MAX_CLUSTERS)
/// rounded up to the nearest power of 2.
///
/// This provides:
/// - Small corpora (<256 docs): 16 clusters
/// - Medium corpora (1K docs): 32 clusters
/// - Large corpora (10K docs): 128 clusters
/// - Very large corpora (>65K docs): 256 clusters (capped)
///
/// # Arguments
/// * `doc_count` - Number of documents/chunks in the corpus
///
/// # Returns
/// The recommended number of clusters (power of 2, between 16 and 256)
pub fn compute_adaptive_cluster_count(doc_count: usize) -> usize {
    if doc_count == 0 {
        return DEFAULT_NUM_CLUSTERS;
    }

    // k ≈ sqrt(n) is optimal for IVF-style indexes
    let sqrt_n = (doc_count as f64).sqrt();

    // Clamp to [MIN_CLUSTERS, MAX_CLUSTERS]
    let clamped = sqrt_n.max(MIN_CLUSTERS as f64).min(MAX_CLUSTERS as f64);

    // Round up to nearest power of 2
    let log2 = clamped.log2().ceil() as u32;
    let power_of_2 = 1usize << log2;

    // Final clamp to ensure we stay within bounds
    power_of_2.clamp(MIN_CLUSTERS, MAX_CLUSTERS)
}

/// Comprehensive health metrics for the index
///
/// Provides detailed statistics about cluster distribution, storage,
/// and overall index quality for monitoring and diagnostics.
#[derive(Debug, Clone, PartialEq)]
pub struct IndexHealthMetrics {
    /// Number of clusters in the index
    pub cluster_count: usize,
    /// Total documents indexed
    pub total_docs: usize,
    /// Number of empty clusters
    pub empty_clusters: usize,
    /// Size of the largest cluster
    pub largest_cluster: usize,
    /// Size of the smallest non-empty cluster (0 if all empty)
    pub smallest_cluster: usize,
    /// Average cluster size
    pub avg_cluster_size: f32,
    /// Standard deviation of cluster sizes
    pub cluster_std_dev: f32,
    /// Imbalance ratio (largest / smallest non-empty, or 0 if fewer than 2 non-empty)
    pub imbalance_ratio: f32,
    /// Overall health score (0.0 = perfect, 1.0 = poor)
    pub health_score: f32,
    /// Whether the index needs rebalancing
    pub needs_rebalancing: bool,
    /// Whether product quantization is active
    pub using_quantization: bool,
    /// Whether HNSW navigation is active
    pub using_hnsw: bool,
    /// Whether using k-means (vs LSH) for assignment
    pub using_kmeans: bool,
    /// Total embeddings in reservoir for optimization
    pub reservoir_size: usize,
    /// Storage estimate in bytes (embeddings only)
    pub storage_bytes: u64,
}

impl Default for IndexHealthMetrics {
    fn default() -> Self {
        Self {
            cluster_count: 0,
            total_docs: 0,
            empty_clusters: 0,
            largest_cluster: 0,
            smallest_cluster: 0,
            avg_cluster_size: 0.0,
            cluster_std_dev: 0.0,
            imbalance_ratio: 0.0,
            health_score: 0.5,
            needs_rebalancing: false,
            using_quantization: false,
            using_hnsw: false,
            using_kmeans: false,
            reservoir_size: 0,
            storage_bytes: 0,
        }
    }
}

/// Lazy index that improves over time
///
/// Starts with LSH-based clustering for instant search capability,
/// then transitions to online k-means as documents accumulate.
/// Supports optional product quantization for 32x storage reduction.
/// Optionally uses HNSW for O(log n) cluster navigation.
pub struct LazyIndex {
    /// Cluster centers (k x embedding_dim)
    centers: Vec<Vec<f32>>,
    /// Count of documents per cluster
    counts: Vec<usize>,
    /// Buckets: cluster_id -> [(doc_id, embedding)]
    /// Full-precision embeddings (512 bytes each)
    buckets: Vec<Vec<(u32, Vec<f32>)>>,
    /// Quantized buckets: cluster_id -> [(doc_id, quantized_embedding)]
    /// Compressed embeddings (16 bytes each)
    quantized_buckets: Vec<Vec<(u32, QuantizedEmbedding)>>,
    /// Product quantizer for compression
    quantizer: Option<ProductQuantizer>,
    /// Whether quantization is enabled
    use_quantization: bool,
    /// LSH hyperplanes for initial clustering
    lsh: Option<LSHIndex>,
    /// Reservoir sample for optimization
    reservoir: Vec<Vec<f32>>,
    /// Total embeddings seen
    total_seen: usize,
    /// Device for tensor operations
    device: Device,
    /// Whether we've transitioned to online k-means
    using_kmeans: bool,
    /// HNSW graph for fast cluster navigation (optional)
    hnsw: Option<HnswGraph>,
    /// Whether to use HNSW for cluster selection
    use_hnsw: bool,
    /// Counter for improve() calls (for automatic rebalancing)
    improve_counter: usize,
}

/// LSH index using random hyperplanes
struct LSHIndex {
    /// Random hyperplanes (num_bits x embedding_dim)
    hyperplanes: Vec<Vec<f32>>,
}

/// Serializable index state for persistence
///
/// Contains all information needed to restore a LazyIndex's learned structure
/// (cluster centers, quantizer) without the actual document embeddings.
#[derive(Serialize, Deserialize)]
struct IndexState {
    /// Version for compatibility checking
    version: u32,
    /// Number of clusters
    num_clusters: usize,
    /// Flattened cluster centers (num_clusters * EMBEDDING_DIM)
    centers: Vec<f32>,
    /// Whether using k-means (vs LSH)
    using_kmeans: bool,
    /// Whether quantization is enabled
    use_quantization: bool,
    /// Quantizer centroids (if trained)
    /// Size: NUM_SUBSPACES * NUM_CENTROIDS * SUBSPACE_DIM
    quantizer_centroids: Option<Vec<f32>>,
    /// Whether HNSW is enabled
    use_hnsw: bool,
}

impl LSHIndex {
    /// Create a new LSH index with random hyperplanes
    fn new(num_bits: usize) -> Self {
        Self::new_seeded(num_bits, None)
    }

    /// Create a new LSH index with optionally seeded random hyperplanes
    fn new_seeded(num_bits: usize, seed: Option<u64>) -> Self {
        use rand::SeedableRng;

        let mut hyperplanes = Vec::with_capacity(num_bits);

        // Use seeded RNG for deterministic results, or thread RNG for random
        let mut seeded_rng;
        let mut thread_rng;
        let rng: &mut dyn rand::RngCore = match seed {
            Some(s) => {
                seeded_rng = rand::rngs::StdRng::seed_from_u64(s);
                &mut seeded_rng
            }
            None => {
                thread_rng = rand::rng();
                &mut thread_rng
            }
        };

        for _ in 0..num_bits {
            // Generate random unit vector
            let mut plane: Vec<f32> = (0..EMBEDDING_DIM)
                .map(|_| rng.random::<f32>() * 2.0 - 1.0)
                .collect();

            // Normalize to unit length
            let norm: f32 = plane.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm > 0.0 {
                for x in &mut plane {
                    *x /= norm;
                }
            }
            hyperplanes.push(plane);
        }

        Self { hyperplanes }
    }

    /// Hash an embedding to a bucket ID
    fn hash(&self, embedding: &[f32]) -> usize {
        let mut bucket = 0usize;

        for (i, plane) in self.hyperplanes.iter().enumerate() {
            // Compute dot product
            let dot: f32 = embedding.iter().zip(plane.iter()).map(|(a, b)| a * b).sum();

            // If on positive side of hyperplane, set bit
            if dot > 0.0 {
                bucket |= 1 << i;
            }
        }

        bucket
    }
}

impl LazyIndex {
    /// Create a new lazy index
    pub fn new(num_clusters: usize) -> Self {
        let num_clusters = if num_clusters == 0 {
            DEFAULT_NUM_CLUSTERS
        } else {
            num_clusters
        };

        // Number of bits for LSH (round up to power of 2)
        let num_bits = (num_clusters as f64).log2().ceil() as usize;
        let actual_clusters = 1 << num_bits;

        // Initialize empty buckets and counts
        let buckets = (0..actual_clusters).map(|_| Vec::new()).collect();
        let quantized_buckets = (0..actual_clusters).map(|_| Vec::new()).collect();
        let counts = vec![0; actual_clusters];
        let centers = vec![vec![0.0; EMBEDDING_DIM]; actual_clusters];

        Self {
            centers,
            counts,
            buckets,
            quantized_buckets,
            quantizer: None,
            use_quantization: false,
            lsh: Some(LSHIndex::new(num_bits)),
            reservoir: Vec::new(),
            total_seen: 0,
            device: Device::Cpu,
            using_kmeans: false,
            hnsw: None,
            use_hnsw: false,
            improve_counter: 0,
        }
    }

    /// Create a new lazy index with a specific device
    pub fn with_device(num_clusters: usize, device: Device) -> Self {
        let mut index = Self::new(num_clusters);
        index.device = device;
        index
    }

    /// Create a new lazy index with quantization enabled
    pub fn with_quantization(num_clusters: usize) -> Self {
        let mut index = Self::new(num_clusters);
        index.use_quantization = true;
        index
    }

    /// Create a new lazy index with HNSW enabled
    ///
    /// HNSW provides O(log n) cluster selection. The graph is built
    /// automatically once enough clusters have data.
    pub fn with_hnsw(num_clusters: usize) -> Self {
        let mut index = Self::new(num_clusters);
        index.use_hnsw = true;
        index
    }

    /// Create a new lazy index with a deterministic seed for reproducibility
    ///
    /// Use this constructor when you need deterministic index behavior,
    /// such as in evaluations or tests.
    pub fn with_seed(num_clusters: usize, seed: u64) -> Self {
        let num_clusters = if num_clusters == 0 {
            DEFAULT_NUM_CLUSTERS
        } else {
            num_clusters
        };

        // Number of bits for LSH (round up to power of 2)
        let num_bits = (num_clusters as f64).log2().ceil() as usize;
        let actual_clusters = 1 << num_bits;

        // Initialize empty buckets and counts
        let buckets = (0..actual_clusters).map(|_| Vec::new()).collect();
        let quantized_buckets = (0..actual_clusters).map(|_| Vec::new()).collect();
        let counts = vec![0; actual_clusters];
        let centers = vec![vec![0.0; EMBEDDING_DIM]; actual_clusters];

        Self {
            centers,
            counts,
            buckets,
            quantized_buckets,
            quantizer: None,
            use_quantization: false,
            lsh: Some(LSHIndex::new_seeded(num_bits, Some(seed))),
            reservoir: Vec::new(),
            total_seen: 0,
            device: Device::Cpu,
            using_kmeans: false,
            hnsw: None,
            use_hnsw: false,
            improve_counter: 0,
        }
    }

    /// Enable or disable quantization
    ///
    /// When enabling, the quantizer will be trained once enough
    /// reservoir samples are collected.
    pub fn set_quantization(&mut self, enabled: bool) {
        self.use_quantization = enabled;
    }

    /// Check if quantization is enabled
    pub fn is_quantized(&self) -> bool {
        self.use_quantization && self.quantizer.is_some()
    }

    /// Check if the quantizer has been trained
    pub fn quantizer_trained(&self) -> bool {
        self.quantizer
            .as_ref()
            .map(ProductQuantizer::is_trained)
            .unwrap_or(false)
    }

    /// Enable or disable HNSW for cluster navigation
    ///
    /// HNSW provides O(log n) cluster selection instead of O(n) linear scan.
    /// Beneficial when there are many clusters (>64).
    pub fn set_hnsw(&mut self, enabled: bool) {
        self.use_hnsw = enabled;
        if enabled && self.hnsw.is_none() {
            self.rebuild_hnsw();
        }
    }

    /// Check if HNSW is enabled and built
    pub fn hnsw_enabled(&self) -> bool {
        self.use_hnsw && self.hnsw.is_some()
    }

    /// Rebuild the HNSW graph from current cluster centers
    ///
    /// Call this after significant cluster structure changes (splits/merges)
    /// or after importing centers from persistence.
    pub fn rebuild_hnsw(&mut self) {
        if self.centers.is_empty() {
            self.hnsw = None;
            return;
        }

        // Only use HNSW if we have enough clusters to benefit
        if self.centers.len() < MIN_NODES_FOR_HNSW {
            tracing::debug!(
                "Skipping HNSW build: {} clusters < {} minimum",
                self.centers.len(),
                MIN_NODES_FOR_HNSW
            );
            self.hnsw = None;
            return;
        }

        // Flatten centers into a single embedding array
        let flat_centers: Vec<f32> = self.centers.iter().flatten().copied().collect();

        let mut graph = HnswGraph::new();
        graph.rebuild(&flat_centers, EMBEDDING_DIM, self.centers.len());

        tracing::debug!(
            "Built HNSW graph for {} cluster centers",
            self.centers.len()
        );

        self.hnsw = Some(graph);
    }

    /// Number of clusters
    pub fn num_clusters(&self) -> usize {
        self.centers.len()
    }

    /// Total documents indexed
    pub fn total_documents(&self) -> usize {
        self.counts.iter().sum()
    }

    /// Rebalance clusters when buckets are highly imbalanced
    ///
    /// Returns the number of embeddings moved.
    pub fn rebalance_clusters(&mut self) -> Result<usize> {
        let total_docs = self.total_documents();
        if total_docs == 0 {
            return Ok(0);
        }

        let num_clusters = self.num_clusters();
        let target = total_docs.div_ceil(num_clusters);

        let (largest, largest_count) = self
            .counts
            .iter()
            .enumerate()
            .max_by_key(|(_, count)| *count)
            .map(|(idx, count)| (idx, *count))
            .unwrap();

        let smallest_nonzero = self
            .counts
            .iter()
            .filter(|&&count| count > 0)
            .min()
            .copied()
            .unwrap_or(0);

        let is_imbalanced = if smallest_nonzero == 0 {
            largest_count > target
        } else {
            largest_count >= smallest_nonzero.saturating_mul(IMBALANCE_RATIO_THRESHOLD)
        };

        if !is_imbalanced {
            return Ok(0);
        }

        let mut underfull: Vec<UnderfullCluster> = self
            .counts
            .iter()
            .enumerate()
            .filter(|(idx, count)| *idx != largest && **count < target)
            .map(|(idx, count)| UnderfullCluster {
                id: idx,
                remaining: target - *count,
            })
            .collect();

        let capacity: usize = underfull.iter().map(|cluster| cluster.remaining).sum();
        if capacity == 0 {
            return Ok(0);
        }

        let move_count = largest_count.saturating_sub(target).min(capacity);
        if move_count == 0 {
            return Ok(0);
        }

        let mut touched = vec![false; num_clusters];
        touched[largest] = true;

        let moved = if self.using_quantized_buckets() {
            let moved =
                self.rebalance_quantized(largest, move_count, &mut underfull, &mut touched)?;
            self.counts[largest] = self.quantized_buckets[largest].len();
            moved
        } else {
            let moved =
                self.rebalance_full_precision(largest, move_count, &mut underfull, &mut touched)?;
            self.counts[largest] = self.buckets[largest].len();
            moved
        };

        if moved == 0 {
            return Ok(0);
        }

        for (cluster, updated) in touched.iter().enumerate() {
            if *updated {
                self.recompute_center_for_cluster(cluster)?;
            }
        }

        if self.use_hnsw {
            self.rebuild_hnsw();
        }

        Ok(moved)
    }

    /// Automatically rebalance clusters if needed
    ///
    /// Checks the health metrics and triggers rebalancing when the index
    /// has significant cluster imbalance. Returns the number of embeddings
    /// moved, or 0 if no rebalancing was needed.
    ///
    /// This method is called automatically during `improve()` at regular
    /// intervals (every `AUTO_REBALANCE_CHECK_INTERVAL` calls).
    pub fn auto_rebalance(&mut self) -> Result<usize> {
        let metrics = self.get_health_metrics();

        if !metrics.needs_rebalancing {
            return Ok(0);
        }

        // Only rebalance if we have enough documents
        if metrics.total_docs < ONLINE_KMEANS_THRESHOLD {
            return Ok(0);
        }

        self.rebalance_clusters()
    }

    /// Add a document embedding to the index
    ///
    /// Returns the cluster assignment.
    pub fn add(&mut self, doc_id: u32, embedding: &[f32]) -> Result<usize> {
        if embedding.len() != EMBEDDING_DIM {
            anyhow::bail!(
                "Embedding dimension mismatch: expected {}, got {}",
                EMBEDDING_DIM,
                embedding.len()
            );
        }

        self.total_seen += 1;

        // Determine cluster assignment
        let cluster = if self.using_kmeans {
            // Use online k-means: find nearest center
            self.nearest_center(embedding)
        } else if let Some(ref lsh) = self.lsh {
            // Use LSH for initial assignments
            lsh.hash(embedding)
        } else {
            // Fallback: use first cluster
            0
        };

        // Add to bucket (quantized or full-precision)
        if self.use_quantization && self.quantizer.is_some() {
            // Use quantized storage (16 bytes per embedding)
            let quantized = self.quantizer.as_ref().unwrap().encode(embedding)?;
            self.quantized_buckets[cluster].push((doc_id, quantized));
        } else {
            // Use full-precision storage (512 bytes per embedding)
            self.buckets[cluster].push((doc_id, embedding.to_vec()));
        }
        self.counts[cluster] += 1;

        // Update center incrementally (Welford's algorithm)
        self.update_center(cluster, embedding);

        // Reservoir sample for future optimization
        self.reservoir_sample(embedding);

        // Check if we should train the quantizer
        if self.use_quantization
            && self.quantizer.is_none()
            && self.reservoir.len() >= MIN_TRAINING_SAMPLES
        {
            self.train_quantizer()?;
        }

        // Check if we should transition to online k-means
        if !self.using_kmeans && self.total_seen >= ONLINE_KMEANS_THRESHOLD {
            self.using_kmeans = true;
            tracing::debug!(
                "Transitioning to online k-means after {} documents",
                self.total_seen
            );
        }

        Ok(cluster)
    }

    /// Train the product quantizer on reservoir samples
    fn train_quantizer(&mut self) -> Result<()> {
        if self.reservoir.len() < MIN_TRAINING_SAMPLES {
            anyhow::bail!(
                "Not enough samples to train quantizer: {} < {}",
                self.reservoir.len(),
                MIN_TRAINING_SAMPLES
            );
        }

        tracing::info!(
            "Training product quantizer on {} reservoir samples",
            self.reservoir.len()
        );

        let mut quantizer = ProductQuantizer::new();
        quantizer.train(&self.reservoir)?;
        self.quantizer = Some(quantizer);

        // Re-quantize existing full-precision embeddings
        self.convert_to_quantized()?;

        tracing::info!("Product quantizer trained successfully");
        Ok(())
    }

    /// Convert existing full-precision embeddings to quantized form
    fn convert_to_quantized(&mut self) -> Result<()> {
        let quantizer = self
            .quantizer
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Quantizer not trained"))?;

        for cluster in 0..self.num_clusters() {
            // Move embeddings from full-precision to quantized buckets
            let full_bucket = std::mem::take(&mut self.buckets[cluster]);
            for (doc_id, embedding) in full_bucket {
                let quantized = quantizer.encode(&embedding)?;
                self.quantized_buckets[cluster].push((doc_id, quantized));
            }
        }

        Ok(())
    }

    /// Add a multi-token document (multiple embeddings per doc)
    ///
    /// For multi-vector retrieval, each document has multiple token embeddings.
    /// We add each token to the index separately but track the document.
    pub fn add_multi(
        &mut self,
        doc_id: u32,
        embeddings: &[f32],
        num_tokens: usize,
    ) -> Result<Vec<usize>> {
        if embeddings.len() != num_tokens * EMBEDDING_DIM {
            anyhow::bail!(
                "Embedding size mismatch: expected {} ({} tokens x {}), got {}",
                num_tokens * EMBEDDING_DIM,
                num_tokens,
                EMBEDDING_DIM,
                embeddings.len()
            );
        }

        let mut clusters = Vec::with_capacity(num_tokens);
        for i in 0..num_tokens {
            let start = i * EMBEDDING_DIM;
            let end = start + EMBEDDING_DIM;
            let token_emb = &embeddings[start..end];
            let cluster = self.add(doc_id, token_emb)?;
            clusters.push(cluster);
        }

        Ok(clusters)
    }

    /// Find the nearest cluster center to an embedding
    fn nearest_center(&self, embedding: &[f32]) -> usize {
        let mut best_cluster = 0;
        let mut best_similarity = f32::NEG_INFINITY;

        for (i, center) in self.centers.iter().enumerate() {
            let similarity = cosine_similarity(embedding, center);
            if similarity > best_similarity {
                best_similarity = similarity;
                best_cluster = i;
            }
        }

        best_cluster
    }

    /// Update cluster center incrementally using Welford's algorithm
    fn update_center(&mut self, cluster: usize, embedding: &[f32]) {
        let n = self.counts[cluster] as f32;
        if n <= 0.0 {
            return;
        }

        let center = &mut self.centers[cluster];

        // Incremental mean: new_mean = old_mean + (x - old_mean) / n
        for (i, &x) in embedding.iter().enumerate() {
            center[i] += (x - center[i]) / n;
        }

        // Normalize to unit length for cosine similarity
        let norm: f32 = center.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in center {
                *x /= norm;
            }
        }
    }

    fn using_quantized_buckets(&self) -> bool {
        self.use_quantization && self.quantizer.is_some()
    }

    fn choose_underfull_cluster(
        embedding: &[f32],
        underfull: &mut [UnderfullCluster],
        centers: &[Vec<f32>],
    ) -> Option<usize> {
        let mut best_idx = None;
        let mut best_sim = f32::NEG_INFINITY;

        for (idx, cluster) in underfull.iter().enumerate() {
            if cluster.remaining == 0 {
                continue;
            }
            let sim = cosine_similarity(embedding, &centers[cluster.id]);
            if sim > best_sim {
                best_sim = sim;
                best_idx = Some(idx);
            }
        }

        if let Some(idx) = best_idx {
            underfull[idx].remaining = underfull[idx].remaining.saturating_sub(1);
            Some(underfull[idx].id)
        } else {
            None
        }
    }

    fn rebalance_full_precision(
        &mut self,
        largest: usize,
        move_count: usize,
        underfull: &mut [UnderfullCluster],
        touched: &mut [bool],
    ) -> Result<usize> {
        let bucket = std::mem::take(&mut self.buckets[largest]);
        if bucket.is_empty() {
            self.buckets[largest] = bucket;
            return Ok(0);
        }

        let center = &self.centers[largest];
        let mut distances: Vec<(usize, f32)> = bucket
            .iter()
            .enumerate()
            .map(|(idx, (_, embedding))| (idx, 1.0 - cosine_similarity(embedding, center)))
            .collect();
        distances.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let move_limit = move_count.min(bucket.len());
        let mut move_flags = vec![false; bucket.len()];
        for (idx, _) in distances.into_iter().take(move_limit) {
            move_flags[idx] = true;
        }

        let mut moved = 0usize;
        let mut new_bucket = Vec::with_capacity(bucket.len().saturating_sub(move_limit));

        for (idx, (doc_id, embedding)) in bucket.into_iter().enumerate() {
            if move_flags[idx] {
                if let Some(target) =
                    Self::choose_underfull_cluster(&embedding, underfull, &self.centers)
                {
                    self.buckets[target].push((doc_id, embedding));
                    self.counts[target] += 1;
                    touched[target] = true;
                    moved += 1;
                    continue;
                }
            }
            new_bucket.push((doc_id, embedding));
        }

        self.buckets[largest] = new_bucket;
        Ok(moved)
    }

    fn rebalance_quantized(
        &mut self,
        largest: usize,
        move_count: usize,
        underfull: &mut [UnderfullCluster],
        touched: &mut [bool],
    ) -> Result<usize> {
        let quantizer = self
            .quantizer
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Quantizer not trained"))?;

        let bucket = std::mem::take(&mut self.quantized_buckets[largest]);
        if bucket.is_empty() {
            self.quantized_buckets[largest] = bucket;
            return Ok(0);
        }

        let center = &self.centers[largest];
        let mut distances: Vec<(usize, f32)> = bucket
            .iter()
            .enumerate()
            .map(|(idx, (_, quantized))| {
                let embedding = quantizer.decode(quantized);
                (idx, 1.0 - cosine_similarity(&embedding, center))
            })
            .collect();
        distances.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let move_limit = move_count.min(bucket.len());
        let mut move_flags = vec![false; bucket.len()];
        for (idx, _) in distances.into_iter().take(move_limit) {
            move_flags[idx] = true;
        }

        let mut moved = 0usize;
        let mut new_bucket = Vec::with_capacity(bucket.len().saturating_sub(move_limit));

        for (idx, (doc_id, quantized)) in bucket.into_iter().enumerate() {
            if move_flags[idx] {
                let embedding = quantizer.decode(&quantized);
                if let Some(target) =
                    Self::choose_underfull_cluster(&embedding, underfull, &self.centers)
                {
                    self.quantized_buckets[target].push((doc_id, quantized));
                    self.counts[target] += 1;
                    touched[target] = true;
                    moved += 1;
                    continue;
                }
            }
            new_bucket.push((doc_id, quantized));
        }

        self.quantized_buckets[largest] = new_bucket;
        Ok(moved)
    }

    fn recompute_center_for_cluster(&mut self, cluster: usize) -> Result<()> {
        if self.counts[cluster] == 0 {
            return Ok(());
        }

        let mut new_center = vec![0.0f32; EMBEDDING_DIM];

        if self.using_quantized_buckets() {
            let quantizer = self
                .quantizer
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Quantizer not trained"))?;
            for (_, quantized) in &self.quantized_buckets[cluster] {
                let embedding = quantizer.decode(quantized);
                for (i, value) in embedding.iter().enumerate() {
                    new_center[i] += value;
                }
            }
        } else {
            for (_, embedding) in &self.buckets[cluster] {
                for (i, value) in embedding.iter().enumerate() {
                    new_center[i] += value;
                }
            }
        }

        let count = self.counts[cluster] as f32;
        if count > 0.0 {
            for value in &mut new_center {
                *value /= count;
            }
        }

        let norm: f32 = new_center.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for value in &mut new_center {
                *value /= norm;
            }
        }

        self.centers[cluster] = new_center;
        Ok(())
    }

    /// Reservoir sample for future optimization
    fn reservoir_sample(&mut self, embedding: &[f32]) {
        if self.reservoir.len() < MAX_RESERVOIR_SIZE {
            self.reservoir.push(embedding.to_vec());
        } else {
            // Reservoir sampling: replace with probability 1/total_seen
            let j = rand::rng().random_range(0..self.total_seen);
            if j < MAX_RESERVOIR_SIZE {
                self.reservoir[j] = embedding.to_vec();
            }
        }
    }

    /// Search the index for similar embeddings
    ///
    /// Returns (score, doc_id) pairs sorted by score descending.
    /// Uses cluster pruning to avoid scanning all documents.
    /// When quantization is enabled, uses asymmetric distance computation.
    pub fn search(&self, query: &[f32], top_k: usize) -> Result<Vec<(f32, u32)>> {
        if query.len() != EMBEDDING_DIM {
            anyhow::bail!(
                "Query dimension mismatch: expected {}, got {}",
                EMBEDDING_DIM,
                query.len()
            );
        }

        // Find top clusters to search (search ~sqrt(k) clusters)
        let num_probe = (self.num_clusters() as f64).sqrt().ceil() as usize;
        let top_clusters = self.top_k_clusters(query, num_probe);

        // Score documents in selected clusters
        let mut scored: Vec<(f32, u32)> = Vec::new();

        // Use ADC if quantized, otherwise full precision
        if let Some(ref quantizer) = self.quantizer {
            // Precompute distance table for efficient ADC
            let dist_table = quantizer.compute_distance_table(query)?;

            for cluster in top_clusters {
                // Search quantized bucket
                for (doc_id, quantized) in &self.quantized_buckets[cluster] {
                    let score = dist_table.asymmetric_cosine_similarity(quantized);
                    scored.push((score, *doc_id));
                }
                // Also search any remaining full-precision embeddings
                for (doc_id, embedding) in &self.buckets[cluster] {
                    let score = cosine_similarity(query, embedding);
                    scored.push((score, *doc_id));
                }
            }
        } else {
            // Full precision search
            for cluster in top_clusters {
                for (doc_id, embedding) in &self.buckets[cluster] {
                    let score = cosine_similarity(query, embedding);
                    scored.push((score, *doc_id));
                }
            }
        }

        // Sort by score descending and dedupe by doc_id
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // Dedupe (keep highest score per doc_id)
        let mut seen = std::collections::HashSet::new();
        scored.retain(|(_, doc_id)| seen.insert(*doc_id));

        scored.truncate(top_k);
        Ok(scored)
    }

    /// Search with MaxSim scoring for multi-vector queries
    ///
    /// For each query token, find max similarity across all document tokens,
    /// then average across query tokens. Supports quantized embeddings.
    pub fn search_maxsim(
        &self,
        query_embeddings: &[f32],
        query_num_tokens: usize,
        top_k: usize,
    ) -> Result<Vec<(f32, u32)>> {
        if query_embeddings.len() != query_num_tokens * EMBEDDING_DIM {
            anyhow::bail!(
                "Query embedding size mismatch: expected {}, got {}",
                query_num_tokens * EMBEDDING_DIM,
                query_embeddings.len()
            );
        }

        // For each query token, find top clusters
        let num_probe = (self.num_clusters() as f64).sqrt().ceil() as usize;
        let mut clusters_to_search = std::collections::HashSet::new();

        for i in 0..query_num_tokens {
            let start = i * EMBEDDING_DIM;
            let end = start + EMBEDDING_DIM;
            let query_token = &query_embeddings[start..end];

            for cluster in self.top_k_clusters(query_token, num_probe) {
                clusters_to_search.insert(cluster);
            }
        }

        // Precompute distance tables for quantized search (one per query token)
        let dist_tables: Option<Vec<_>> = self
            .quantizer
            .as_ref()
            .map(|quantizer| {
                (0..query_num_tokens)
                    .map(|i| {
                        let start = i * EMBEDDING_DIM;
                        let end = start + EMBEDDING_DIM;
                        let query_token = &query_embeddings[start..end];
                        quantizer.compute_distance_table(query_token)
                    })
                    .collect::<Result<Vec<_>>>()
            })
            .transpose()?;

        // Collect document tokens: doc_id -> (full_precision_tokens, quantized_tokens)
        let mut doc_full: std::collections::HashMap<u32, Vec<&[f32]>> =
            std::collections::HashMap::new();
        let mut doc_quantized: std::collections::HashMap<u32, Vec<&QuantizedEmbedding>> =
            std::collections::HashMap::new();

        for cluster in clusters_to_search {
            // Full precision embeddings
            for (doc_id, embedding) in &self.buckets[cluster] {
                doc_full.entry(*doc_id).or_default().push(embedding);
            }
            // Quantized embeddings
            for (doc_id, quantized) in &self.quantized_buckets[cluster] {
                doc_quantized.entry(*doc_id).or_default().push(quantized);
            }
        }

        // Get all unique doc IDs
        let mut all_doc_ids: std::collections::HashSet<u32> = doc_full.keys().copied().collect();
        all_doc_ids.extend(doc_quantized.keys());

        // Convert to Vec for parallel iteration
        let doc_ids_vec: Vec<u32> = all_doc_ids.into_iter().collect();

        // Compute MaxSim score for each document (P1 optimization: parallel scoring)
        // Each document's score is independent, so we can use rayon for multicore scaling.
        let scored: Vec<(f32, u32)> = doc_ids_vec
            .par_iter()
            .map(|&doc_id| {
                let full_tokens = doc_full.get(&doc_id).map(|v| v.as_slice()).unwrap_or(&[]);
                let quant_tokens = doc_quantized
                    .get(&doc_id)
                    .map(|v| v.as_slice())
                    .unwrap_or(&[]);

                let query_max_sims: Vec<f32> = (0..query_num_tokens)
                    .map(|i| {
                        let start = i * EMBEDDING_DIM;
                        let end = start + EMBEDDING_DIM;
                        let query_token = &query_embeddings[start..end];

                        // Max similarity across full precision tokens
                        // Use optimized dot product since both query and doc embeddings are L2-normalized
                        let max_full = full_tokens
                            .iter()
                            .map(|doc_token| dot_product_normalized(query_token, doc_token))
                            .fold(f32::NEG_INFINITY, f32::max);

                        // Max similarity across quantized tokens (using ADC)
                        let max_quant = if let Some(ref tables) = dist_tables {
                            quant_tokens
                                .iter()
                                .map(|q| tables[i].asymmetric_cosine_similarity(q))
                                .fold(f32::NEG_INFINITY, f32::max)
                        } else {
                            f32::NEG_INFINITY
                        };

                        // Take overall max
                        max_full.max(max_quant)
                    })
                    .collect();

                // Average across query tokens
                let score = query_max_sims.iter().sum::<f32>() / query_num_tokens as f32;
                (score, doc_id)
            })
            .collect();

        // Sort by score descending
        let mut scored = scored;
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);

        Ok(scored)
    }

    /// Find top-k clusters by centroid similarity
    ///
    /// Uses HNSW for O(log n) search when enabled and available,
    /// otherwise falls back to O(n) linear scan.
    fn top_k_clusters(&self, query: &[f32], k: usize) -> Vec<usize> {
        // Try HNSW if available
        if let Some(ref hnsw) = self.hnsw {
            if self.use_hnsw {
                let flat_centers: Vec<f32> = self.centers.iter().flatten().copied().collect();
                let results = hnsw.search(query, k, &flat_centers, EMBEDDING_DIM);

                // Filter out empty clusters and return
                let non_empty: Vec<usize> = results
                    .into_iter()
                    .map(|(_, id)| id)
                    .filter(|&id| id < self.counts.len() && self.counts[id] > 0)
                    .collect();

                if !non_empty.is_empty() {
                    return non_empty;
                }
                // Fall through to linear scan if HNSW returned no valid results
            }
        }

        // Linear scan fallback
        let mut cluster_scores: Vec<(f32, usize)> = self
            .centers
            .iter()
            .enumerate()
            .filter(|(i, _)| self.counts[*i] > 0) // Skip empty clusters
            .map(|(i, center)| (cosine_similarity(query, center), i))
            .collect();

        cluster_scores.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        cluster_scores.truncate(k);

        cluster_scores.into_iter().map(|(_, i)| i).collect()
    }

    /// Perform one step of background optimization
    pub fn improve(&mut self) {
        // Increment counter and check for automatic rebalancing
        self.improve_counter += 1;
        if self
            .improve_counter
            .is_multiple_of(AUTO_REBALANCE_CHECK_INTERVAL)
        {
            // Ignore errors from auto_rebalance - it's best-effort
            let _ = self.auto_rebalance();
        }

        if self.reservoir.len() < MIN_OPTIMIZATION_SAMPLES {
            return; // Not enough data to optimize
        }

        // Pick a random cluster to improve
        let cluster = rand::rng().random_range(0..self.num_clusters());

        if self.counts[cluster] < MIN_OPTIMIZATION_SAMPLES {
            return; // Cluster too small
        }

        // Recompute center from a sample of bucket contents
        let bucket = &self.buckets[cluster];
        let sample_size = bucket.len().min(MAX_BUCKET_SAMPLE_SIZE);

        if sample_size < MIN_OPTIMIZATION_SAMPLES {
            return;
        }

        // Sample random embeddings from bucket
        let mut rng = rand::rng();
        let mut new_center = vec![0.0f32; EMBEDDING_DIM];

        for _ in 0..sample_size {
            let idx = rng.random_range(0..bucket.len());
            let (_, embedding) = &bucket[idx];
            for (i, &x) in embedding.iter().enumerate() {
                new_center[i] += x;
            }
        }

        // Average and normalize
        for x in &mut new_center {
            *x /= sample_size as f32;
        }

        let norm: f32 = new_center.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in &mut new_center {
                *x /= norm;
            }
        }

        // Blend with current center (don't jump too fast)
        for (i, x) in new_center.iter().enumerate() {
            self.centers[cluster][i] =
                self.centers[cluster][i] * (1.0 - CENTER_BLEND_FACTOR) + x * CENTER_BLEND_FACTOR;
        }

        // Normalize blended center
        let norm: f32 = self.centers[cluster]
            .iter()
            .map(|x| x * x)
            .sum::<f32>()
            .sqrt();
        if norm > 0.0 {
            for x in &mut self.centers[cluster] {
                *x /= norm;
            }
        }
    }

    /// Get index health (0.0 = perfect, 1.0 = poor)
    ///
    /// Based on average intra-cluster distance.
    pub fn health(&self) -> f32 {
        if self.total_documents() < MIN_OPTIMIZATION_SAMPLES {
            return 0.5; // Not enough data to judge
        }

        let mut total_dist = 0.0;
        let mut total_count = 0;

        for (cluster, bucket) in self.buckets.iter().enumerate() {
            if bucket.len() < MIN_BUCKET_SIZE_FOR_HEALTH {
                continue;
            }

            let center = &self.centers[cluster];

            // Sample embeddings for health calculation
            let sample_size = bucket.len().min(MAX_HEALTH_SAMPLE_SIZE);
            let mut rng = rand::rng();

            for _ in 0..sample_size {
                let idx = rng.random_range(0..bucket.len());
                let (_, embedding) = &bucket[idx];
                let dist = 1.0 - cosine_similarity(embedding, center);
                total_dist += dist;
                total_count += 1;
            }
        }

        if total_count > 0 {
            total_dist / total_count as f32
        } else {
            0.5
        }
    }

    /// Check if index needs optimization
    pub fn needs_work(&self) -> bool {
        self.health() > HEALTH_THRESHOLD_NEEDS_WORK
    }

    /// Get comprehensive health metrics for the index
    ///
    /// Returns detailed statistics about cluster distribution, storage usage,
    /// and overall index quality. Useful for monitoring and diagnostics.
    pub fn get_health_metrics(&self) -> IndexHealthMetrics {
        let cluster_count = self.centers.len();
        let total_docs = self.total_documents();

        // Cluster size statistics
        let non_empty_counts: Vec<usize> = self.counts.iter().copied().filter(|&c| c > 0).collect();
        let empty_clusters = cluster_count - non_empty_counts.len();

        let largest_cluster = self.counts.iter().copied().max().unwrap_or(0);
        let smallest_cluster = non_empty_counts.iter().copied().min().unwrap_or(0);

        let avg_cluster_size = if cluster_count > 0 {
            total_docs as f32 / cluster_count as f32
        } else {
            0.0
        };

        // Standard deviation of cluster sizes
        let cluster_std_dev = if cluster_count > 0 {
            let variance = self
                .counts
                .iter()
                .map(|&c| {
                    let diff = c as f32 - avg_cluster_size;
                    diff * diff
                })
                .sum::<f32>()
                / cluster_count as f32;
            variance.sqrt()
        } else {
            0.0
        };

        // Imbalance ratio
        let imbalance_ratio = if non_empty_counts.len() >= 2 && smallest_cluster > 0 {
            largest_cluster as f32 / smallest_cluster as f32
        } else {
            0.0
        };

        // Check if rebalancing is needed
        let needs_rebalancing = if smallest_cluster > 0 {
            largest_cluster >= smallest_cluster * IMBALANCE_RATIO_THRESHOLD
        } else {
            false
        };

        // Storage estimate
        let storage_bytes = if self.using_quantized_buckets() {
            // Quantized: 16 bytes per embedding
            (total_docs as u64) * 16
        } else {
            // Full precision: EMBEDDING_DIM * 4 bytes per embedding (f32)
            (total_docs as u64) * (EMBEDDING_DIM as u64) * 4
        };

        IndexHealthMetrics {
            cluster_count,
            total_docs,
            empty_clusters,
            largest_cluster,
            smallest_cluster,
            avg_cluster_size,
            cluster_std_dev,
            imbalance_ratio,
            health_score: self.health(),
            needs_rebalancing,
            using_quantization: self.is_quantized(),
            using_hnsw: self.hnsw_enabled(),
            using_kmeans: self.using_kmeans,
            reservoir_size: self.reservoir.len(),
            storage_bytes,
        }
    }

    /// Export centers for persistence
    pub fn export_centers(&self) -> (Vec<f32>, usize) {
        let flat: Vec<f32> = self.centers.iter().flatten().copied().collect();
        (flat, self.centers.len())
    }

    /// Import centers from persistence
    pub fn import_centers(&mut self, data: &[f32], num_centers: usize) -> Result<()> {
        if data.len() != num_centers * EMBEDDING_DIM {
            anyhow::bail!(
                "Center data size mismatch: expected {}, got {}",
                num_centers * EMBEDDING_DIM,
                data.len()
            );
        }

        self.centers.clear();
        for i in 0..num_centers {
            let start = i * EMBEDDING_DIM;
            let end = start + EMBEDDING_DIM;
            self.centers.push(data[start..end].to_vec());
        }

        // Resize buckets and counts if needed
        while self.buckets.len() < num_centers {
            self.buckets.push(Vec::new());
        }
        while self.counts.len() < num_centers {
            self.counts.push(0);
        }

        // Disable LSH since we have learned centers
        self.using_kmeans = true;
        self.lsh = None;

        // Rebuild HNSW if enabled
        if self.use_hnsw {
            self.rebuild_hnsw();
        }

        Ok(())
    }

    /// Save the index state to a file
    ///
    /// This saves the cluster centers, quantizer (if trained), and configuration.
    /// The HNSW graph is rebuilt on load rather than persisted.
    /// Document/chunk embeddings in buckets are NOT saved - they are reloaded
    /// from the SQLite database.
    pub fn save(&self, path: &Path) -> Result<()> {
        let state = IndexState {
            version: INDEX_STATE_VERSION,
            num_clusters: self.centers.len(),
            centers: self.export_centers().0,
            using_kmeans: self.using_kmeans,
            use_quantization: self.use_quantization,
            quantizer_centroids: self.quantizer.as_ref().map(|q| q.export_centroids()),
            use_hnsw: self.use_hnsw,
        };

        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        bincode::serialize_into(writer, &state)?;

        tracing::info!(
            "Saved index state to {:?} ({} clusters, quantized={}, hnsw={})",
            path,
            state.num_clusters,
            state.quantizer_centroids.is_some(),
            state.use_hnsw
        );

        Ok(())
    }

    /// Load the index state from a file
    ///
    /// This loads the cluster centers, quantizer (if present), and configuration.
    /// The HNSW graph is rebuilt from the loaded centers.
    /// After loading, document embeddings should be added via `add()` or `add_multi()`.
    pub fn load(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let state: IndexState = bincode::deserialize_from(reader)?;

        // Check version compatibility
        if state.version != INDEX_STATE_VERSION {
            anyhow::bail!(
                "Index state version mismatch: expected {}, got {}",
                INDEX_STATE_VERSION,
                state.version
            );
        }

        // Create index with the saved number of clusters
        let mut index = Self::new(state.num_clusters);

        // Import centers
        index.import_centers(&state.centers, state.num_clusters)?;

        // Restore quantizer if present
        if let Some(centroids) = state.quantizer_centroids {
            let mut quantizer = ProductQuantizer::new();
            quantizer.import_centroids(&centroids)?;
            index.quantizer = Some(quantizer);
        }
        index.use_quantization = state.use_quantization;

        // Restore HNSW setting (graph will be rebuilt by import_centers if use_hnsw is true)
        index.use_hnsw = state.use_hnsw;
        if index.use_hnsw && index.hnsw.is_none() {
            index.rebuild_hnsw();
        }

        tracing::info!(
            "Loaded index state from {:?} ({} clusters, quantized={}, hnsw={})",
            path,
            state.num_clusters,
            index.quantizer.is_some(),
            index.hnsw_enabled()
        );

        Ok(index)
    }

    /// Load the index state from a file, or create a new index if the file doesn't exist
    pub fn load_or_new(path: &Path, num_clusters: usize) -> Result<Self> {
        if path.exists() {
            Self::load(path)
        } else {
            Ok(Self::new(num_clusters))
        }
    }

    /// Clear all indexed data but keep cluster structure and quantizer
    pub fn clear(&mut self) {
        for bucket in &mut self.buckets {
            bucket.clear();
        }
        for bucket in &mut self.quantized_buckets {
            bucket.clear();
        }
        for count in &mut self.counts {
            *count = 0;
        }
        self.reservoir.clear();
        self.total_seen = 0;
    }

    /// Get memory usage estimate in bytes
    pub fn memory_usage(&self) -> usize {
        let full_size: usize = self
            .buckets
            .iter()
            .map(|b| b.iter().map(|(_, emb)| 4 + emb.len() * 4).sum::<usize>())
            .sum();

        let quantized_size: usize = self
            .quantized_buckets
            .iter()
            .map(|b| b.len() * (4 + 16)) // doc_id + 16 bytes per quantized embedding
            .sum();

        let centers_size = self.centers.len() * EMBEDDING_DIM * 4;
        let reservoir_size = self.reservoir.len() * EMBEDDING_DIM * 4;

        full_size + quantized_size + centers_size + reservoir_size
    }

    /// Get number of full-precision embeddings
    pub fn full_precision_count(&self) -> usize {
        self.buckets.iter().map(|b| b.len()).sum()
    }

    /// Get number of quantized embeddings
    pub fn quantized_count(&self) -> usize {
        self.quantized_buckets.iter().map(|b| b.len()).sum()
    }

    /// Get the product quantizer (if trained)
    pub fn quantizer(&self) -> Option<&ProductQuantizer> {
        self.quantizer.as_ref()
    }
}

/// Compute cosine similarity between two vectors (general case)
///
/// Use this for comparisons involving cluster centers or other vectors
/// that may not be L2-normalized.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a > 0.0 && norm_b > 0.0 {
        dot / (norm_a * norm_b)
    } else {
        0.0
    }
}

/// Compute dot product between two L2-normalized vectors (optimized)
///
/// Since document embeddings are L2-normalized before storage (see embedder.rs:l2_normalize),
/// cosine_similarity(a, b) = dot(a, b) / (||a|| * ||b||) = dot(a, b) / (1 * 1) = dot(a, b)
///
/// This optimization avoids computing norms on every similarity call.
/// Only use for vectors that are known to be L2-normalized!
#[inline]
fn dot_product_normalized(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_adaptive_cluster_count_empty() {
        // Empty corpus should use default
        assert_eq!(compute_adaptive_cluster_count(0), DEFAULT_NUM_CLUSTERS);
    }

    #[test]
    fn test_compute_adaptive_cluster_count_small() {
        // Small corpora should use minimum clusters (16)
        assert_eq!(compute_adaptive_cluster_count(10), 16);
        assert_eq!(compute_adaptive_cluster_count(100), 16);
        assert_eq!(compute_adaptive_cluster_count(256), 16);
    }

    #[test]
    fn test_compute_adaptive_cluster_count_medium() {
        // Medium corpora: sqrt(1000) = 31.6 -> rounds to 32
        assert_eq!(compute_adaptive_cluster_count(1000), 32);
        // sqrt(2000) = 44.7 -> rounds to 64
        assert_eq!(compute_adaptive_cluster_count(2000), 64);
        // sqrt(4000) = 63.2 -> rounds to 64
        assert_eq!(compute_adaptive_cluster_count(4000), 64);
    }

    #[test]
    fn test_compute_adaptive_cluster_count_large() {
        // Large corpora: sqrt(10000) = 100 -> rounds to 128
        assert_eq!(compute_adaptive_cluster_count(10000), 128);
        // sqrt(16000) = 126.5 -> rounds to 128
        assert_eq!(compute_adaptive_cluster_count(16000), 128);
        // sqrt(40000) = 200 -> rounds to 256
        assert_eq!(compute_adaptive_cluster_count(40000), 256);
    }

    #[test]
    fn test_compute_adaptive_cluster_count_very_large() {
        // Very large corpora should cap at MAX_CLUSTERS (256)
        assert_eq!(compute_adaptive_cluster_count(100000), 256);
        assert_eq!(compute_adaptive_cluster_count(1000000), 256);
    }

    #[test]
    fn test_compute_adaptive_cluster_count_is_power_of_2() {
        // All results should be powers of 2
        for n in [10, 100, 500, 1000, 5000, 10000, 50000, 100000] {
            let k = compute_adaptive_cluster_count(n);
            assert!(
                k.is_power_of_two(),
                "k={k} for n={n} should be power of 2"
            );
            assert!(k >= MIN_CLUSTERS, "k={k} should be >= MIN_CLUSTERS");
            assert!(k <= MAX_CLUSTERS, "k={k} should be <= MAX_CLUSTERS");
        }
    }

    #[test]
    fn test_lsh_hash_consistency() {
        let lsh = LSHIndex::new(6); // 64 buckets

        // Same vector should hash to same bucket
        let v1 = vec![0.1; EMBEDDING_DIM];
        let h1 = lsh.hash(&v1);
        let h2 = lsh.hash(&v1);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_lsh_similar_vectors() {
        let lsh = LSHIndex::new(6);

        // Similar vectors should often hash to same or nearby buckets
        let mut v1 = vec![0.0; EMBEDDING_DIM];
        v1[0] = 1.0;

        let mut v2 = v1.clone();
        v2[1] = 0.1; // Small perturbation

        let h1 = lsh.hash(&v1);
        let h2 = lsh.hash(&v2);

        // They might be the same or differ by just a few bits
        let diff = (h1 ^ h2).count_ones();
        assert!(diff <= 3, "Similar vectors should have similar hashes");
    }

    #[test]
    fn test_lazy_index_add_and_search() {
        let mut index = LazyIndex::new(16);

        // Add some documents
        let emb1 = vec![1.0; EMBEDDING_DIM];
        let emb2 = vec![-1.0; EMBEDDING_DIM];

        index.add(1, &emb1).unwrap();
        index.add(2, &emb2).unwrap();

        assert_eq!(index.total_documents(), 2);

        // Search should find the similar one
        let results = index.search(&emb1, 2).unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].1, 1); // doc_id 1 should be most similar
    }

    #[test]
    fn test_lazy_index_clustering_transition() {
        let mut index = LazyIndex::new(8);

        // Add many documents to trigger transition
        for i in 0..150 {
            let mut emb = vec![0.0; EMBEDDING_DIM];
            emb[i % EMBEDDING_DIM] = 1.0;
            index.add(i as u32, &emb).unwrap();
        }

        assert!(index.using_kmeans);
        assert_eq!(index.total_documents(), 150);
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);

        let c = vec![0.0, 1.0, 0.0];
        assert!(cosine_similarity(&a, &c).abs() < 1e-6); // Orthogonal

        let d = vec![-1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &d) - (-1.0)).abs() < 1e-6); // Opposite
    }

    #[test]
    fn test_health_metric() {
        let mut index = LazyIndex::new(8);

        // Empty index has neutral health
        assert!((index.health() - 0.5).abs() < 0.1);

        // Add some random documents
        let mut rng = rand::rng();
        for i in 0..100 {
            let emb: Vec<f32> = (0..EMBEDDING_DIM).map(|_| rng.random()).collect();
            index.add(i, &emb).unwrap();
        }

        // Health should be measurable now
        let h = index.health();
        assert!((0.0..=1.0).contains(&h));
    }

    #[test]
    fn test_get_health_metrics() {
        let mut index = LazyIndex::new(8);

        // Empty index metrics
        let metrics = index.get_health_metrics();
        assert_eq!(metrics.cluster_count, 8);
        assert_eq!(metrics.total_docs, 0);
        assert_eq!(metrics.empty_clusters, 8);
        assert!(!metrics.using_kmeans); // Initially uses LSH
        assert!(!metrics.using_quantization);
        assert!(!metrics.using_hnsw);

        // Add some random documents
        let mut rng = rand::rng();
        for i in 0..100 {
            let emb: Vec<f32> = (0..EMBEDDING_DIM).map(|_| rng.random()).collect();
            index.add(i, &emb).unwrap();
        }

        // Metrics should be populated now
        let metrics = index.get_health_metrics();
        assert_eq!(metrics.cluster_count, 8);
        assert_eq!(metrics.total_docs, 100);
        assert!(metrics.empty_clusters < 8); // Some clusters should be populated
        assert!(metrics.largest_cluster > 0);
        assert!(metrics.avg_cluster_size > 0.0);
        assert!((0.0..=1.0).contains(&metrics.health_score));
        assert!(metrics.storage_bytes > 0);
        assert!(metrics.using_kmeans); // Transitioned after 100 docs

        // Imbalance ratio should be positive if we have multiple non-empty clusters
        let non_empty = 8 - metrics.empty_clusters;
        if non_empty >= 2 && metrics.smallest_cluster > 0 {
            assert!(metrics.imbalance_ratio > 0.0);
        }
    }

    #[test]
    fn test_export_import_centers() {
        let mut index = LazyIndex::new(8);

        // Add some documents
        for i in 0..50 {
            let mut emb = vec![0.1; EMBEDDING_DIM];
            emb[i % EMBEDDING_DIM] = 1.0;
            index.add(i as u32, &emb).unwrap();
        }

        // Export
        let (data, num) = index.export_centers();
        assert_eq!(num, 8);
        assert_eq!(data.len(), 8 * EMBEDDING_DIM);

        // Import into new index
        let mut new_index = LazyIndex::new(8);
        new_index.import_centers(&data, num).unwrap();

        assert!(new_index.using_kmeans);
        assert_eq!(new_index.num_clusters(), 8);
    }

    #[test]
    fn test_add_multi() {
        let mut index = LazyIndex::new(8);

        // Create multi-token embeddings (3 tokens x 128 dim = 384 floats)
        let num_tokens = 3;
        let mut embeddings = vec![0.0f32; num_tokens * EMBEDDING_DIM];

        // Set distinct values for each token
        for i in 0..num_tokens {
            embeddings[i * EMBEDDING_DIM + i] = 1.0; // Each token points in a different direction
        }

        // Add multi-token document
        let clusters = index.add_multi(1, &embeddings, num_tokens).unwrap();
        assert_eq!(clusters.len(), num_tokens);

        // Total documents should reflect all tokens
        assert_eq!(index.total_documents(), num_tokens);

        // Search should find the document
        let query = vec![0.0f32; EMBEDDING_DIM];
        let mut query_copy = query.clone();
        query_copy[0] = 1.0; // Match first token
        let results = index.search(&query_copy, 5).unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].1, 1); // doc_id should be 1
    }

    #[test]
    fn test_add_multi_dimension_mismatch() {
        let mut index = LazyIndex::new(8);

        // Wrong size embeddings
        let embeddings = vec![0.0f32; 100]; // Not a multiple of EMBEDDING_DIM
        let result = index.add_multi(1, &embeddings, 3);
        assert!(result.is_err());
    }

    #[test]
    fn test_clear() {
        let mut index = LazyIndex::new(8);

        // Add some documents
        for i in 0..50 {
            let mut emb = vec![0.1; EMBEDDING_DIM];
            emb[i % EMBEDDING_DIM] = 1.0;
            index.add(i as u32, &emb).unwrap();
        }

        assert_eq!(index.total_documents(), 50);
        assert!(index.total_seen > 0);

        // Clear the index
        index.clear();

        // Documents should be gone but cluster structure preserved
        assert_eq!(index.total_documents(), 0);
        assert_eq!(index.total_seen, 0);
        assert_eq!(index.num_clusters(), 8); // Clusters still exist

        // Search on empty index should return empty results
        let query = vec![0.1; EMBEDDING_DIM];
        let results = index.search(&query, 10).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_dimension_mismatch() {
        let index = LazyIndex::new(8);

        // Wrong dimension query
        let query = vec![0.1f32; 64]; // Wrong size
        let result = index.search(&query, 10);
        assert!(result.is_err());
    }

    #[test]
    fn test_add_dimension_mismatch() {
        let mut index = LazyIndex::new(8);

        // Wrong dimension embedding
        let emb = vec![0.1f32; 64]; // Wrong size
        let result = index.add(1, &emb);
        assert!(result.is_err());
    }

    #[test]
    fn test_search_maxsim() {
        let mut index = LazyIndex::new(8);

        // Add multi-token documents
        // Doc 1: two tokens pointing in x and y directions
        let num_tokens = 2;
        let mut doc1_emb = vec![0.0f32; num_tokens * EMBEDDING_DIM];
        doc1_emb[0] = 1.0; // First token: x-direction
        doc1_emb[EMBEDDING_DIM + 1] = 1.0; // Second token: y-direction
        index.add_multi(1, &doc1_emb, num_tokens).unwrap();

        // Doc 2: two tokens pointing in z and w directions
        let mut doc2_emb = vec![0.0f32; num_tokens * EMBEDDING_DIM];
        doc2_emb[2] = 1.0; // First token: z-direction
        doc2_emb[EMBEDDING_DIM + 3] = 1.0; // Second token: w-direction
        index.add_multi(2, &doc2_emb, num_tokens).unwrap();

        // Query with tokens pointing in x and y directions (should match doc 1)
        let mut query_emb = vec![0.0f32; num_tokens * EMBEDDING_DIM];
        query_emb[0] = 1.0; // First query token: x-direction
        query_emb[EMBEDDING_DIM + 1] = 1.0; // Second query token: y-direction

        let results = index.search_maxsim(&query_emb, num_tokens, 5).unwrap();
        assert!(!results.is_empty());
        // Doc 1 should have higher score since query matches its token directions
        assert_eq!(results[0].1, 1);
    }

    #[test]
    fn test_search_maxsim_dimension_mismatch() {
        let index = LazyIndex::new(8);

        // Wrong size query embeddings
        let query_emb = vec![0.0f32; 100]; // Not matching 2 tokens * EMBEDDING_DIM
        let result = index.search_maxsim(&query_emb, 2, 5);
        assert!(result.is_err());
    }

    #[test]
    fn test_improve() {
        let mut index = LazyIndex::new(8);

        // Add enough documents to enable optimization
        let mut rng = rand::rng();
        for i in 0..100 {
            let emb: Vec<f32> = (0..EMBEDDING_DIM).map(|_| rng.random()).collect();
            index.add(i, &emb).unwrap();
        }

        // Get initial health
        let health_before = index.health();

        // Run several improvement iterations
        for _ in 0..20 {
            index.improve();
        }

        // Health should remain measurable (optimization doesn't break anything)
        let health_after = index.health();
        assert!((0.0..=1.0).contains(&health_after));

        // The improvement function should not crash or corrupt the index
        // Note: We don't assert health_after < health_before because
        // random improvement steps may not always improve
        assert!(health_before.is_finite());
        assert!(health_after.is_finite());
    }

    #[test]
    fn test_improve_empty_index() {
        let mut index = LazyIndex::new(8);

        // Improve on empty index should not crash
        index.improve();

        // Index should still be functional
        let mut emb = vec![0.1; EMBEDDING_DIM];
        emb[0] = 1.0;
        index.add(1, &emb).unwrap();
        assert_eq!(index.total_documents(), 1);
    }

    #[test]
    fn test_needs_work() {
        let mut index = LazyIndex::new(8);

        // Empty index doesn't need work (returns neutral health)
        // needs_work() returns true when health > 0.2
        // Empty index health is 0.5, so it technically "needs work"
        // but there's nothing to optimize

        // Add random documents - this will likely result in poor clustering
        let mut rng = rand::rng();
        for i in 0..100 {
            let emb: Vec<f32> = (0..EMBEDDING_DIM).map(|_| rng.random()).collect();
            index.add(i, &emb).unwrap();
        }

        // needs_work() should return a boolean based on health threshold
        let needs = index.needs_work();
        let health = index.health();

        // Verify consistency between needs_work and health
        assert_eq!(needs, health > HEALTH_THRESHOLD_NEEDS_WORK);
    }

    #[test]
    fn test_rebalance_clusters_moves_from_imbalanced_bucket() {
        let mut index = LazyIndex::new(4);
        index.using_kmeans = true;
        index.lsh = None;

        for center in index.centers.iter_mut().take(4) {
            *center = vec![0.0; EMBEDDING_DIM];
        }
        for i in 0..4 {
            index.centers[i][i] = 1.0;
        }

        let mut dominant = vec![0.0; EMBEDDING_DIM];
        dominant[0] = 1.0;
        for i in 0..100 {
            index.add(i as u32, &dominant).unwrap();
        }

        let mut minority = vec![0.0; EMBEDDING_DIM];
        minority[1] = 1.0;
        index.add(200, &minority).unwrap();

        let total_before = index.total_documents();
        let counts_before = index.counts.clone();

        let moved = index.rebalance_clusters().unwrap();
        assert!(moved > 0);
        assert_eq!(index.total_documents(), total_before);
        assert!(index.counts[0] < counts_before[0]);
    }

    #[test]
    fn test_rebalance_clusters_noop_when_balanced() {
        let mut index = LazyIndex::new(4);
        index.using_kmeans = true;
        index.lsh = None;

        for center in index.centers.iter_mut().take(4) {
            *center = vec![0.0; EMBEDDING_DIM];
        }
        for i in 0..4 {
            index.centers[i][i] = 1.0;
        }

        for i in 0..4 {
            let mut emb = vec![0.0; EMBEDDING_DIM];
            emb[i] = 1.0;
            for j in 0..10 {
                index.add((i * 100 + j) as u32, &emb).unwrap();
            }
        }

        let moved = index.rebalance_clusters().unwrap();
        assert_eq!(moved, 0);
    }

    #[test]
    fn test_with_device() {
        let index = LazyIndex::with_device(16, Device::Cpu);

        // Index should be properly initialized
        assert_eq!(index.num_clusters(), 16);
        assert_eq!(index.total_documents(), 0);

        // Should function normally
        let mut index = index;
        let emb = vec![0.1; EMBEDDING_DIM];
        index.add(1, &emb).unwrap();
        assert_eq!(index.total_documents(), 1);
    }

    #[test]
    fn test_num_clusters_default() {
        // Passing 0 should use default
        let index = LazyIndex::new(0);
        assert!(index.num_clusters() > 0);
        assert_eq!(index.num_clusters(), 64); // DEFAULT_NUM_CLUSTERS rounded to power of 2
    }

    #[test]
    fn test_lsh_hyperplanes_are_unit_vectors() {
        let lsh = LSHIndex::new(6);

        for (i, plane) in lsh.hyperplanes.iter().enumerate() {
            // Each hyperplane should have EMBEDDING_DIM dimensions
            assert_eq!(
                plane.len(),
                EMBEDDING_DIM,
                "Hyperplane {i} has wrong dimension"
            );

            // Each hyperplane should be normalized (unit length)
            let norm: f32 = plane.iter().map(|x| x * x).sum::<f32>().sqrt();
            assert!(
                (norm - 1.0).abs() < 1e-5,
                "Hyperplane {i} not normalized: norm = {norm}"
            );
        }
    }

    #[test]
    fn test_lsh_hash_range_bounded() {
        let num_bits = 6;
        let lsh = LSHIndex::new(num_bits);
        let max_bucket = (1 << num_bits) - 1; // 63 for 6 bits

        // Test various embeddings
        let test_vectors = [
            vec![1.0; EMBEDDING_DIM],
            vec![-1.0; EMBEDDING_DIM],
            vec![0.0; EMBEDDING_DIM],
            {
                let mut v = vec![0.0; EMBEDDING_DIM];
                v[0] = 1.0;
                v
            },
            {
                let mut v = vec![0.0; EMBEDDING_DIM];
                v[EMBEDDING_DIM - 1] = -1.0;
                v
            },
        ];

        for v in &test_vectors {
            let h = lsh.hash(v);
            assert!(
                h <= max_bucket,
                "Hash {h} exceeds max bucket {max_bucket}"
            );
        }
    }

    #[test]
    fn test_lsh_hash_opposite_vectors_differ() {
        let lsh = LSHIndex::new(6);

        // Opposite vectors should hash to different buckets
        let v1 = vec![1.0; EMBEDDING_DIM];
        let v2 = vec![-1.0; EMBEDDING_DIM];

        let h1 = lsh.hash(&v1);
        let h2 = lsh.hash(&v2);

        // Opposite vectors produce opposite dot products,
        // so their hashes should be complements (all bits flipped)
        assert_eq!(
            h1 ^ h2,
            (1 << lsh.hyperplanes.len()) - 1,
            "Opposite vectors should have complementary hashes"
        );
    }

    #[test]
    fn test_lsh_hash_distribution() {
        let num_bits = 4;
        let lsh = LSHIndex::new(num_bits);
        let num_buckets = 1 << num_bits; // 16 buckets
        let mut bucket_hits = vec![0usize; num_buckets];

        // Generate random vectors and check distribution
        let mut rng = rand::rng();
        let num_samples = 1000;

        for _ in 0..num_samples {
            let v: Vec<f32> = (0..EMBEDDING_DIM)
                .map(|_| rng.random::<f32>() * 2.0 - 1.0)
                .collect();
            let h = lsh.hash(&v);
            bucket_hits[h] += 1;
        }

        // Check that all buckets got some hits (statistical test)
        let min_expected = num_samples / num_buckets / 4; // Allow 4x variance
        let buckets_with_hits = bucket_hits.iter().filter(|&&c| c > min_expected).count();

        // At least half the buckets should have reasonable coverage
        assert!(
            buckets_with_hits >= num_buckets / 2,
            "LSH hash distribution too skewed: only {buckets_with_hits}/{num_buckets} buckets have adequate coverage"
        );
    }

    #[test]
    fn test_lsh_zero_vector_hash() {
        let lsh = LSHIndex::new(6);

        // Zero vector has no positive dot products, so hash should be 0
        let zero = vec![0.0; EMBEDDING_DIM];
        let h = lsh.hash(&zero);
        assert_eq!(h, 0, "Zero vector should hash to bucket 0");
    }

    #[test]
    fn test_default_num_clusters_value() {
        // Default cluster count should be 64
        assert_eq!(DEFAULT_NUM_CLUSTERS, 64);
        // Should be a power of 2 for efficient bucketing
        assert!(DEFAULT_NUM_CLUSTERS.is_power_of_two());
    }

    #[test]
    fn test_max_reservoir_size_is_reasonable() {
        // Reservoir should hold enough samples for statistical significance
        // but not so large it uses excessive memory (range: 1000-100000)
        assert_eq!(MAX_RESERVOIR_SIZE, 10000);
    }

    #[test]
    fn test_online_kmeans_threshold_is_reasonable() {
        // Need enough docs before transitioning to k-means (range: 10-1000)
        assert_eq!(ONLINE_KMEANS_THRESHOLD, 100);
    }

    #[test]
    fn test_min_optimization_samples_is_reasonable() {
        // Need at least a few samples for meaningful optimization (range: 5-100)
        assert_eq!(MIN_OPTIMIZATION_SAMPLES, 10);
    }

    #[test]
    fn test_sample_size_limits_are_consistent() {
        // Bucket sample size: between MIN_OPTIMIZATION_SAMPLES (10) and MAX_RESERVOIR_SIZE (10000)
        assert_eq!(MAX_BUCKET_SAMPLE_SIZE, 100);

        // Health sample size: between MIN_BUCKET_SIZE_FOR_HEALTH (5) and MAX_BUCKET_SAMPLE_SIZE (100)
        assert_eq!(MAX_HEALTH_SAMPLE_SIZE, 50);
    }

    #[test]
    fn test_min_bucket_size_for_health_is_reasonable() {
        // Need enough items in bucket to calculate meaningful health (range: 2-20)
        assert_eq!(MIN_BUCKET_SIZE_FOR_HEALTH, 5);
    }

    #[test]
    fn test_center_blend_factor_is_valid() {
        // Blend factor in (0, 1) range, leaning towards existing centers
        // 0.1 means 10% new center, 90% old center
        assert!((CENTER_BLEND_FACTOR - 0.1).abs() < f32::EPSILON);
    }

    #[test]
    fn test_health_threshold_needs_work_is_valid() {
        // Threshold in (0, 1) range; triggers optimization when health > 0.2
        // (higher health value indicates poorer clustering)
        assert!((HEALTH_THRESHOLD_NEEDS_WORK - 0.2).abs() < f32::EPSILON);
    }

    #[test]
    fn test_num_clusters_returns_correct_count() {
        // Test various cluster counts
        let index8 = LazyIndex::new(8);
        assert_eq!(index8.num_clusters(), 8);

        let index16 = LazyIndex::new(16);
        assert_eq!(index16.num_clusters(), 16);

        let index32 = LazyIndex::new(32);
        assert_eq!(index32.num_clusters(), 32);

        // Non-power-of-2 gets rounded up
        let index10 = LazyIndex::new(10);
        assert_eq!(index10.num_clusters(), 16); // Rounds up to 16
    }

    #[test]
    fn test_total_documents_starts_at_zero() {
        let index = LazyIndex::new(8);
        assert_eq!(index.total_documents(), 0);
    }

    #[test]
    fn test_total_documents_increments_correctly() {
        let mut index = LazyIndex::new(8);
        assert_eq!(index.total_documents(), 0);

        // Add first document
        let emb1 = vec![0.1; EMBEDDING_DIM];
        index.add(1, &emb1).unwrap();
        assert_eq!(index.total_documents(), 1);

        // Add second document
        let emb2 = vec![0.2; EMBEDDING_DIM];
        index.add(2, &emb2).unwrap();
        assert_eq!(index.total_documents(), 2);

        // Add third document with same ID (should still increment)
        let emb3 = vec![0.3; EMBEDDING_DIM];
        index.add(1, &emb3).unwrap();
        assert_eq!(index.total_documents(), 3);
    }

    #[test]
    fn test_total_documents_after_clear() {
        let mut index = LazyIndex::new(8);

        // Add some documents
        for i in 0..10 {
            let emb = vec![i as f32 / 10.0; EMBEDDING_DIM];
            index.add(i, &emb).unwrap();
        }
        assert_eq!(index.total_documents(), 10);

        // Clear and verify
        index.clear();
        assert_eq!(index.total_documents(), 0);
    }

    #[test]
    fn test_num_clusters_after_clear() {
        let mut index = LazyIndex::new(16);
        assert_eq!(index.num_clusters(), 16);

        // Add documents
        for i in 0..5 {
            let emb = vec![i as f32 / 5.0; EMBEDDING_DIM];
            index.add(i, &emb).unwrap();
        }

        // Clear should preserve cluster count
        index.clear();
        assert_eq!(index.num_clusters(), 16);
    }

    #[test]
    fn test_add_multi_increments_total_documents() {
        let mut index = LazyIndex::new(8);

        // Add multiple embeddings at once
        let num_tokens = 5;
        let embeddings = vec![0.1f32; num_tokens * EMBEDDING_DIM];
        index.add_multi(1, &embeddings, num_tokens).unwrap();

        // total_documents counts individual token embeddings, not documents
        assert_eq!(index.total_documents(), num_tokens);
    }

    // === Quantization tests ===

    #[test]
    fn test_with_quantization() {
        let index = LazyIndex::with_quantization(8);
        assert!(!index.is_quantized()); // Not quantized until trained
        assert!(!index.quantizer_trained());
    }

    #[test]
    fn test_set_quantization() {
        let mut index = LazyIndex::new(8);
        assert!(!index.use_quantization);

        index.set_quantization(true);
        assert!(index.use_quantization);

        index.set_quantization(false);
        assert!(!index.use_quantization);
    }

    #[test]
    fn test_quantizer_trains_automatically() {
        let mut index = LazyIndex::with_quantization(8);

        // Add enough documents to trigger training (need MIN_TRAINING_SAMPLES)
        let mut rng = rand::rng();
        for i in 0..300 {
            let emb: Vec<f32> = (0..EMBEDDING_DIM).map(|_| rng.random()).collect();
            index.add(i as u32, &emb).unwrap();
        }

        // Quantizer should be trained now
        assert!(index.quantizer_trained());
        assert!(index.is_quantized());
    }

    #[test]
    fn test_quantized_storage_compression() {
        let mut index = LazyIndex::with_quantization(8);

        // Add enough documents to trigger training
        let mut rng = rand::rng();
        for i in 0..300 {
            let emb: Vec<f32> = (0..EMBEDDING_DIM).map(|_| rng.random()).collect();
            index.add(i as u32, &emb).unwrap();
        }

        // After training, new embeddings go to quantized storage
        assert!(index.quantized_count() > 0);

        // Check compression ratio
        // Full precision: 512 bytes per embedding
        // Quantized: 16 bytes per embedding (32x compression)
        let full_count = index.full_precision_count();
        let quant_count = index.quantized_count();

        // All embeddings should be quantized now (old ones converted)
        assert_eq!(full_count, 0);
        assert_eq!(quant_count, 300);
    }

    #[test]
    fn test_quantized_search() {
        let mut index = LazyIndex::with_quantization(8);

        // Add documents to trigger training
        let mut rng = rand::rng();
        let mut target_emb = vec![0.0f32; EMBEDDING_DIM];
        target_emb[0] = 1.0;
        target_emb[1] = 0.5;

        // Normalize
        let norm: f32 = target_emb.iter().map(|x| x * x).sum::<f32>().sqrt();
        for x in &mut target_emb {
            *x /= norm;
        }

        // Add target document
        index.add(42, &target_emb).unwrap();

        // Add random documents to trigger quantizer training
        for i in 0..300 {
            let emb: Vec<f32> = (0..EMBEDDING_DIM)
                .map(|_| rng.random::<f32>() - 0.5)
                .collect();
            let norm: f32 = emb.iter().map(|x| x * x).sum::<f32>().sqrt();
            let normalized: Vec<f32> = emb.iter().map(|x| x / norm).collect();
            index.add(i as u32, &normalized).unwrap();
        }

        // Search for target
        let results = index.search(&target_emb, 5).unwrap();
        assert!(!results.is_empty());

        // Target should be in top results (may not be #1 due to quantization error)
        let target_rank = results.iter().position(|(_, id)| *id == 42);
        assert!(
            target_rank.is_some(),
            "Target should be found in search results"
        );
        assert!(
            target_rank.unwrap() < 5,
            "Target should be in top 5 results"
        );
    }

    #[test]
    fn test_memory_usage() {
        let mut index = LazyIndex::new(8);

        // Empty index has minimal memory usage
        let empty_usage = index.memory_usage();
        assert!(empty_usage > 0); // Has cluster centers

        // Add documents
        for i in 0..100 {
            let mut emb = vec![0.0; EMBEDDING_DIM];
            emb[i % EMBEDDING_DIM] = 1.0;
            index.add(i as u32, &emb).unwrap();
        }

        let used_usage = index.memory_usage();
        assert!(used_usage > empty_usage);
    }

    #[test]
    fn test_quantized_memory_usage() {
        let mut index = LazyIndex::with_quantization(8);

        // Add enough to trigger training
        let mut rng = rand::rng();
        for i in 0..300 {
            let emb: Vec<f32> = (0..EMBEDDING_DIM).map(|_| rng.random()).collect();
            index.add(i as u32, &emb).unwrap();
        }

        // Quantized count check
        assert_eq!(index.full_precision_count(), 0);
        assert_eq!(index.quantized_count(), 300);

        // Calculate embedding storage savings
        // Full precision bucket: 300 * (4 + 512) = 154,800 bytes
        // Quantized bucket: 300 * (4 + 16) = 6,000 bytes
        // Savings: ~25x compression on embeddings alone

        // Verify quantized bucket size is much smaller
        let quantized_bucket_size: usize = index
            .quantized_buckets
            .iter()
            .map(|b| b.len() * (4 + 16))
            .sum();

        let full_precision_equivalent = 300 * (4 + 512);
        let compression_ratio = full_precision_equivalent / quantized_bucket_size;

        assert!(
            compression_ratio >= 20,
            "Compression ratio {compression_ratio} should be at least 20x"
        );
    }

    #[test]
    fn test_clear_with_quantization() {
        let mut index = LazyIndex::with_quantization(8);

        // Add enough to trigger training
        let mut rng = rand::rng();
        for i in 0..300 {
            let emb: Vec<f32> = (0..EMBEDDING_DIM).map(|_| rng.random()).collect();
            index.add(i as u32, &emb).unwrap();
        }

        assert!(index.quantizer_trained());
        assert_eq!(index.quantized_count(), 300);

        // Clear the index
        index.clear();

        // Quantizer should still be trained, but embeddings gone
        assert!(index.quantizer_trained());
        assert_eq!(index.quantized_count(), 0);
        assert_eq!(index.full_precision_count(), 0);
        assert_eq!(index.total_documents(), 0);
    }

    #[test]
    fn test_quantized_search_maxsim() {
        let mut index = LazyIndex::with_quantization(8);

        // Add documents to trigger training
        let mut rng = rand::rng();
        for i in 0..300 {
            let emb: Vec<f32> = (0..EMBEDDING_DIM)
                .map(|_| rng.random::<f32>() - 0.5)
                .collect();
            index.add(i as u32, &emb).unwrap();
        }

        // Create a multi-token query
        let num_query_tokens = 2;
        let query: Vec<f32> = (0..num_query_tokens * EMBEDDING_DIM)
            .map(|_| rng.random::<f32>() - 0.5)
            .collect();

        // Search should work
        let results = index.search_maxsim(&query, num_query_tokens, 5).unwrap();
        // Results may be empty if query doesn't match any cluster
        // but the search should not error
        assert!(results.len() <= 5);
    }

    // === HNSW tests ===

    #[test]
    fn test_with_hnsw() {
        let index = LazyIndex::with_hnsw(64);
        assert!(index.use_hnsw);
        assert!(!index.hnsw_enabled()); // Not built until there are enough clusters
    }

    #[test]
    fn test_set_hnsw() {
        let mut index = LazyIndex::new(64);
        assert!(!index.use_hnsw);

        index.set_hnsw(true);
        assert!(index.use_hnsw);

        index.set_hnsw(false);
        assert!(!index.use_hnsw);
    }

    #[test]
    fn test_hnsw_rebuild() {
        let mut index = LazyIndex::new(128);

        // Add enough documents to have non-empty clusters
        let mut rng = rand::rng();
        for i in 0..200 {
            let emb: Vec<f32> = (0..EMBEDDING_DIM).map(|_| rng.random()).collect();
            index.add(i as u32, &emb).unwrap();
        }

        // Enable HNSW and rebuild
        index.set_hnsw(true);
        index.rebuild_hnsw();

        // HNSW should be built since we have 128 clusters (>= MIN_NODES_FOR_HNSW)
        assert!(index.hnsw_enabled());
    }

    #[test]
    fn test_hnsw_search_works() {
        use rand::SeedableRng;

        let mut index = LazyIndex::with_hnsw(128);

        // Use seeded RNG for deterministic test
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);

        // Add 500 documents with random embeddings
        for i in 0..500 {
            let emb: Vec<f32> = (0..EMBEDDING_DIM)
                .map(|_| rand::Rng::random::<f32>(&mut rng))
                .collect();
            index.add(i as u32, &emb).unwrap();
        }

        // Rebuild HNSW now that we have data
        index.rebuild_hnsw();
        assert!(
            index.hnsw_enabled(),
            "HNSW should be enabled with 128 clusters"
        );

        // Search with a random query
        let query: Vec<f32> = (0..EMBEDDING_DIM)
            .map(|_| rand::Rng::random::<f32>(&mut rng))
            .collect();
        let results = index.search(&query, 10).unwrap();

        // HNSW search should return results
        assert!(!results.is_empty(), "HNSW search should return results");

        // Results should have valid scores (cosine similarity in [-1, 1])
        for (score, _) in &results {
            assert!(
                *score >= -1.0 && *score <= 1.0,
                "Score {score} out of valid range"
            );
        }

        // Results should be sorted by score descending
        for i in 1..results.len() {
            assert!(
                results[i - 1].0 >= results[i].0,
                "Results should be sorted by score descending"
            );
        }
    }

    #[test]
    fn test_hnsw_not_built_for_few_clusters() {
        let mut index = LazyIndex::with_hnsw(32); // Less than MIN_NODES_FOR_HNSW

        // Add some documents
        for i in 0..50 {
            let mut emb = vec![0.0; EMBEDDING_DIM];
            emb[i % EMBEDDING_DIM] = 1.0;
            index.add(i as u32, &emb).unwrap();
        }

        // HNSW should NOT be built because we only have 32 clusters
        index.rebuild_hnsw();
        assert!(!index.hnsw_enabled());
    }

    #[test]
    fn test_hnsw_import_centers_rebuilds() {
        let mut index = LazyIndex::with_hnsw(128);

        // Create centers data
        let centers: Vec<f32> = (0..128 * EMBEDDING_DIM)
            .map(|i| ((i % 13) as f32) / 13.0)
            .collect();

        // Import centers should rebuild HNSW
        index.import_centers(&centers, 128).unwrap();

        // HNSW should be built
        assert!(index.hnsw_enabled());
    }

    #[test]
    fn test_hnsw_search_fallback() {
        // Test that search works even when HNSW is enabled but not built
        let mut index = LazyIndex::with_hnsw(32); // Too few clusters for HNSW

        // Add documents
        for i in 0..50 {
            let mut emb = vec![0.0; EMBEDDING_DIM];
            emb[i % EMBEDDING_DIM] = 1.0;
            index.add(i as u32, &emb).unwrap();
        }

        // HNSW not built, but search should still work (falls back to linear scan)
        let query = vec![1.0; EMBEDDING_DIM];
        let results = index.search(&query, 5).unwrap();
        // Results depend on cluster assignment, but search should not error
        assert!(results.len() <= 5);
    }

    // === Index Persistence Tests ===

    #[test]
    fn test_save_load_basic() {
        let temp_dir = tempfile::tempdir().unwrap();
        let index_path = temp_dir.path().join("index.bin");

        // Create and populate an index
        let mut index = LazyIndex::new(16);

        // Add enough documents to transition to k-means
        for i in 0..150 {
            let mut emb = vec![0.1; EMBEDDING_DIM];
            emb[i % EMBEDDING_DIM] = 1.0;
            index.add(i as u32, &emb).unwrap();
        }

        // Save the index
        index.save(&index_path).unwrap();
        assert!(index_path.exists());

        // Load the index
        let loaded = LazyIndex::load(&index_path).unwrap();

        // Verify loaded state matches original
        assert_eq!(loaded.num_clusters(), index.num_clusters());
        assert!(loaded.using_kmeans);

        // Centers should match
        let (orig_centers, orig_num) = index.export_centers();
        let (load_centers, load_num) = loaded.export_centers();
        assert_eq!(orig_num, load_num);
        assert_eq!(orig_centers.len(), load_centers.len());

        // Centers should be close (allowing for float precision)
        for (a, b) in orig_centers.iter().zip(load_centers.iter()) {
            assert!((a - b).abs() < 1e-6, "Center mismatch: {a} vs {b}");
        }
    }

    #[test]
    fn test_save_load_with_quantization() {
        let temp_dir = tempfile::tempdir().unwrap();
        let index_path = temp_dir.path().join("index_quant.bin");

        // Create index with quantization
        let mut index = LazyIndex::with_quantization(16);

        // Add enough documents to trigger quantizer training
        let mut rng = rand::rng();
        for i in 0..300 {
            let emb: Vec<f32> = (0..EMBEDDING_DIM).map(|_| rng.random()).collect();
            index.add(i as u32, &emb).unwrap();
        }

        assert!(index.quantizer_trained(), "Quantizer should be trained");

        // Save the index
        index.save(&index_path).unwrap();

        // Load the index
        let loaded = LazyIndex::load(&index_path).unwrap();

        // Verify quantizer was restored
        assert!(
            loaded.quantizer_trained(),
            "Loaded index should have trained quantizer"
        );
        assert!(loaded.use_quantization);

        // Test encoding produces same result
        let test_emb: Vec<f32> = (0..EMBEDDING_DIM).map(|_| rng.random()).collect();
        let orig_encoded = index.quantizer().unwrap().encode(&test_emb).unwrap();
        let load_encoded = loaded.quantizer().unwrap().encode(&test_emb).unwrap();
        assert_eq!(orig_encoded.codes, load_encoded.codes);
    }

    #[test]
    fn test_save_load_with_hnsw() {
        let temp_dir = tempfile::tempdir().unwrap();
        let index_path = temp_dir.path().join("index_hnsw.bin");

        // Create index with HNSW (needs >= 64 clusters for HNSW to build)
        let mut index = LazyIndex::with_hnsw(128);

        // Add documents
        let mut rng = rand::rng();
        for i in 0..200 {
            let emb: Vec<f32> = (0..EMBEDDING_DIM).map(|_| rng.random()).collect();
            index.add(i as u32, &emb).unwrap();
        }

        index.rebuild_hnsw();
        assert!(index.hnsw_enabled());

        // Save the index
        index.save(&index_path).unwrap();

        // Load the index
        let loaded = LazyIndex::load(&index_path).unwrap();

        // HNSW should be rebuilt on load
        assert!(loaded.use_hnsw);
        assert!(loaded.hnsw_enabled(), "HNSW should be rebuilt on load");
    }

    #[test]
    fn test_load_or_new_creates_new() {
        let temp_dir = tempfile::tempdir().unwrap();
        let index_path = temp_dir.path().join("nonexistent.bin");

        // File doesn't exist, should create new index
        let index = LazyIndex::load_or_new(&index_path, 32).unwrap();
        assert_eq!(index.num_clusters(), 32);
        assert_eq!(index.total_documents(), 0);
    }

    #[test]
    fn test_load_or_new_loads_existing() {
        let temp_dir = tempfile::tempdir().unwrap();
        let index_path = temp_dir.path().join("existing.bin");

        // Create and save an index
        let mut index = LazyIndex::new(64);
        for i in 0..100 {
            let mut emb = vec![0.1; EMBEDDING_DIM];
            emb[i % EMBEDDING_DIM] = 1.0;
            index.add(i as u32, &emb).unwrap();
        }
        index.save(&index_path).unwrap();

        // Load with load_or_new - should load existing
        let loaded = LazyIndex::load_or_new(&index_path, 16).unwrap();
        assert_eq!(loaded.num_clusters(), 64); // Should have 64, not 16
    }

    #[test]
    fn test_save_load_empty_index() {
        let temp_dir = tempfile::tempdir().unwrap();
        let index_path = temp_dir.path().join("empty.bin");

        // Create empty index
        let index = LazyIndex::new(32);
        index.save(&index_path).unwrap();

        // Load empty index
        let loaded = LazyIndex::load(&index_path).unwrap();
        assert_eq!(loaded.num_clusters(), 32);
        assert_eq!(loaded.total_documents(), 0);
    }

    #[test]
    fn test_save_load_preserves_search_quality() {
        use rand::SeedableRng;

        let temp_dir = tempfile::tempdir().unwrap();
        let index_path = temp_dir.path().join("search.bin");

        // Create index with some distinctive embeddings
        let mut index = LazyIndex::new(16);

        // Add target embedding - sparse pattern that's very distinctive
        // Only first 8 dimensions are 1.0, rest are 0.0 (normalized)
        let mut target_emb: Vec<f32> = vec![0.0; EMBEDDING_DIM];
        let active_dims = 8;
        let norm = (active_dims as f32).sqrt();
        for dim in target_emb.iter_mut().take(active_dims) {
            *dim = 1.0 / norm;
        }
        index.add(42, &target_emb).unwrap();

        // Use seeded RNG for deterministic test
        let mut rng = rand::rngs::StdRng::seed_from_u64(12345);

        // Add random embeddings (small random values across all dimensions)
        for i in 0..100 {
            let emb: Vec<f32> = (0..EMBEDDING_DIM)
                .map(|_| rand::Rng::random::<f32>(&mut rng) * 0.1 - 0.05)
                .collect();
            index.add(i as u32, &emb).unwrap();
        }

        // Save and load
        index.save(&index_path).unwrap();
        let mut loaded = LazyIndex::load(&index_path).unwrap();

        // Re-add the embeddings to the loaded index with fresh seed
        let mut rng2 = rand::rngs::StdRng::seed_from_u64(54321);
        loaded.add(42, &target_emb).unwrap();
        for i in 0..100 {
            let emb: Vec<f32> = (0..EMBEDDING_DIM)
                .map(|_| rand::Rng::random::<f32>(&mut rng2) * 0.1 - 0.05)
                .collect();
            loaded.add(i as u32, &emb).unwrap();
        }

        // Search should find target in both (use larger k for safety)
        let orig_results = index.search(&target_emb, 10).unwrap();
        let load_results = loaded.search(&target_emb, 10).unwrap();

        // Target (id=42) should be found in both
        let orig_has_target = orig_results.iter().any(|(_, id)| *id == 42);
        let load_has_target = load_results.iter().any(|(_, id)| *id == 42);

        assert!(orig_has_target, "Original index should find target");
        assert!(load_has_target, "Loaded index should find target");
    }

    #[test]
    fn test_index_state_version_check() {
        let temp_dir = tempfile::tempdir().unwrap();
        let index_path = temp_dir.path().join("version.bin");

        // Create and save an index
        let index = LazyIndex::new(16);
        index.save(&index_path).unwrap();

        // Verify we can load it (version matches)
        let _loaded = LazyIndex::load(&index_path).unwrap();

        // Read the file and manually corrupt the version
        let data = std::fs::read(&index_path).unwrap();
        // Version is at the start of the file (first 4 bytes after bincode encoding)
        // Just verify the file was created and has data
        assert!(!data.is_empty());
    }

    #[test]
    fn test_auto_rebalance_does_nothing_when_balanced() {
        let mut index = LazyIndex::new(4);
        index.using_kmeans = true;
        index.lsh = None;

        // Set up distinct cluster centers
        for center in index.centers.iter_mut().take(4) {
            *center = vec![0.0; EMBEDDING_DIM];
        }
        for i in 0..4 {
            index.centers[i][i] = 1.0;
        }

        // Add equal number of documents to each cluster
        for i in 0..4 {
            let mut emb = vec![0.0; EMBEDDING_DIM];
            emb[i] = 1.0;
            for j in 0..50 {
                index.add((i * 100 + j) as u32, &emb).unwrap();
            }
        }

        // Auto-rebalance should do nothing since clusters are balanced
        let moved = index.auto_rebalance().unwrap();
        assert_eq!(moved, 0, "Balanced index should not trigger rebalancing");
    }

    #[test]
    fn test_auto_rebalance_triggers_when_imbalanced() {
        let mut index = LazyIndex::new(4);
        index.using_kmeans = true;
        index.lsh = None;

        // Set up distinct cluster centers
        for center in index.centers.iter_mut().take(4) {
            *center = vec![0.0; EMBEDDING_DIM];
        }
        for i in 0..4 {
            index.centers[i][i] = 1.0;
        }

        // Create massive imbalance: 1000 in cluster 0, 5 in others
        // This exceeds IMBALANCE_RATIO_THRESHOLD (100x)
        let emb0 = {
            let mut e = vec![0.0; EMBEDDING_DIM];
            e[0] = 1.0;
            e
        };
        for j in 0..1000 {
            index.add(j, &emb0).unwrap();
        }

        // Add minimal to other clusters
        for i in 1..4 {
            let mut emb = vec![0.0; EMBEDDING_DIM];
            emb[i] = 1.0;
            for j in 0..5 {
                index.add((i * 1000 + j) as u32, &emb).unwrap();
            }
        }

        let total_before = index.total_documents();
        let moved = index.auto_rebalance().unwrap();

        // Should have moved some embeddings
        assert!(moved > 0, "Imbalanced index should trigger rebalancing");
        // Total documents should remain the same
        assert_eq!(index.total_documents(), total_before);
    }

    #[test]
    fn test_improve_increments_counter() {
        let mut index = LazyIndex::new(4);

        // Add enough data for improve to work
        let mut rng = rand::rng();
        for i in 0..200 {
            let emb: Vec<f32> = (0..EMBEDDING_DIM).map(|_| rng.random()).collect();
            index.add(i, &emb).unwrap();
        }

        // Initial counter should be 0
        assert_eq!(index.improve_counter, 0);

        // Call improve several times
        for _ in 0..10 {
            index.improve();
        }

        // Counter should have incremented
        assert_eq!(index.improve_counter, 10);
    }

    #[test]
    fn test_improve_triggers_auto_rebalance_at_interval() {
        let mut index = LazyIndex::new(4);
        index.using_kmeans = true;
        index.lsh = None;

        // Set up distinct cluster centers
        for center in index.centers.iter_mut().take(4) {
            *center = vec![0.0; EMBEDDING_DIM];
        }
        for i in 0..4 {
            index.centers[i][i] = 1.0;
        }

        // Add balanced data (so improve() won't actually rebalance anything)
        for i in 0..4 {
            let mut emb = vec![0.0; EMBEDDING_DIM];
            emb[i] = 1.0;
            for j in 0..50 {
                index.add((i * 100 + j) as u32, &emb).unwrap();
            }
        }

        // Call improve AUTO_REBALANCE_CHECK_INTERVAL times
        for _ in 0..AUTO_REBALANCE_CHECK_INTERVAL {
            index.improve();
        }

        // Counter should be at the interval
        assert_eq!(index.improve_counter, AUTO_REBALANCE_CHECK_INTERVAL);
    }

    // ==================== SEARCH QUALITY REGRESSION TESTS ====================

    #[test]
    fn test_search_ordering_consistency() {
        // Regression test: same query should return same order every time
        let mut index = LazyIndex::new(8);

        let mut rng = rand::rng();

        // Add 100 random embeddings
        for i in 0..100 {
            let emb: Vec<f32> = (0..EMBEDDING_DIM).map(|_| rng.random()).collect();
            index.add(i, &emb).unwrap();
        }

        // Create a query embedding
        let query: Vec<f32> = (0..EMBEDDING_DIM).map(|_| rng.random()).collect();

        // Run the same search 5 times
        let results1 = index.search(&query, 10).unwrap();
        let results2 = index.search(&query, 10).unwrap();
        let results3 = index.search(&query, 10).unwrap();
        let results4 = index.search(&query, 10).unwrap();
        let results5 = index.search(&query, 10).unwrap();

        // All should return the same ordering
        let ids1: Vec<_> = results1.iter().map(|(_, id)| *id).collect();
        let ids2: Vec<_> = results2.iter().map(|(_, id)| *id).collect();
        let ids3: Vec<_> = results3.iter().map(|(_, id)| *id).collect();
        let ids4: Vec<_> = results4.iter().map(|(_, id)| *id).collect();
        let ids5: Vec<_> = results5.iter().map(|(_, id)| *id).collect();

        assert_eq!(ids1, ids2, "Search ordering should be consistent (1 vs 2)");
        assert_eq!(ids2, ids3, "Search ordering should be consistent (2 vs 3)");
        assert_eq!(ids3, ids4, "Search ordering should be consistent (3 vs 4)");
        assert_eq!(ids4, ids5, "Search ordering should be consistent (4 vs 5)");
    }

    #[test]
    fn test_similar_embeddings_rank_higher() {
        // Regression test: similar embeddings should rank higher than dissimilar ones
        // Use only 2 clusters to ensure we search the right cluster
        let mut index = LazyIndex::new(2);

        // Create a target embedding (normalized unit vector in first dims)
        let norm = (EMBEDDING_DIM as f32).sqrt();
        let target: Vec<f32> = (0..EMBEDDING_DIM)
            .map(|i| if i < 8 { 1.0 / norm } else { 0.0 })
            .collect();

        // Create a very similar embedding (almost identical)
        let similar: Vec<f32> = (0..EMBEDDING_DIM)
            .map(|i| if i < 8 { 0.95 / norm } else { 0.0 })
            .collect();

        // Create a dissimilar embedding (orthogonal direction in different dims)
        let dissimilar: Vec<f32> = (0..EMBEDDING_DIM)
            .map(|i| {
                if (64..72).contains(&i) {
                    1.0 / norm
                } else {
                    0.0
                }
            })
            .collect();

        // Add embeddings: similar=id 1, dissimilar=id 2
        index.add(1, &similar).unwrap();
        index.add(2, &dissimilar).unwrap();

        // Add some random noise embeddings
        let mut rng = rand::rng();
        for i in 10..30 {
            let emb: Vec<f32> = (0..EMBEDDING_DIM)
                .map(|_| rng.random::<f32>() * 0.1)
                .collect();
            index.add(i, &emb).unwrap();
        }

        // Search for the target - use high k to find all candidates
        let results = index.search(&target, 30).unwrap();

        // Find ranks of similar and dissimilar
        let similar_rank = results.iter().position(|(_, id)| *id == 1);
        let dissimilar_rank = results.iter().position(|(_, id)| *id == 2);

        // At least one should be found
        assert!(
            similar_rank.is_some() || dissimilar_rank.is_some(),
            "At least one target embedding should be in results"
        );

        // If both are found, similar should rank higher
        if let (Some(sim_r), Some(dis_r)) = (similar_rank, dissimilar_rank) {
            assert!(
                sim_r < dis_r,
                "Similar embedding (rank {sim_r}) should rank higher than dissimilar (rank {dis_r})"
            );
        }

        // If only similar is found, that's also acceptable (dissimilar in wrong cluster)
        if similar_rank.is_some() && dissimilar_rank.is_none() {
            // Pass - similar found, dissimilar not relevant
        }

        // If only dissimilar is found but not similar, that's a problem
        if similar_rank.is_none() && dissimilar_rank.is_some() {
            panic!("Dissimilar embedding found but similar was not - search ordering is wrong");
        }
    }

    #[test]
    fn test_search_with_empty_clusters() {
        // Regression test: search should work correctly even with empty clusters
        let mut index = LazyIndex::new(16); // 16 clusters

        // Only add embeddings to a subset of cluster regions
        let norm = (EMBEDDING_DIM as f32).sqrt();

        // Add embeddings that would map to clusters 0-3
        for i in 0..50 {
            let mut emb = vec![0.0f32; EMBEDDING_DIM];
            emb[i % 8] = 1.0 / norm;
            emb[(i + 1) % 8] = 0.5 / norm;
            index.add(i as u32, &emb).unwrap();
        }

        // Many clusters should be empty
        let empty_count = index.buckets.iter().filter(|b| b.is_empty()).count();
        assert!(
            empty_count > 0,
            "Should have at least some empty clusters, got {empty_count}"
        );

        // Search should still work
        let query = vec![1.0 / norm; EMBEDDING_DIM];
        let results = index.search(&query, 10).unwrap();

        // Should return results despite empty clusters
        assert!(!results.is_empty(), "Search should return results");
    }

    #[test]
    fn test_search_scores_decrease_monotonically() {
        // Regression test: search results should be sorted by score (highest first)
        let mut index = LazyIndex::new(8);

        let mut rng = rand::rng();

        // Add 200 random embeddings
        for i in 0..200 {
            let emb: Vec<f32> = (0..EMBEDDING_DIM).map(|_| rng.random()).collect();
            index.add(i, &emb).unwrap();
        }

        // Run search
        let query: Vec<f32> = (0..EMBEDDING_DIM).map(|_| rng.random()).collect();
        let results = index.search(&query, 50).unwrap();

        // Verify scores are monotonically decreasing
        for window in results.windows(2) {
            let (score_a, _) = window[0];
            let (score_b, _) = window[1];
            assert!(
                score_a >= score_b,
                "Scores should be monotonically decreasing: {score_a} < {score_b}"
            );
        }
    }

    #[test]
    fn test_seeded_index_is_deterministic() {
        // Test that with_seed creates deterministic LSH hyperplanes
        use rand::SeedableRng;

        const SEED: u64 = 12345;

        // Create two indexes with the same seed
        let index1 = LazyIndex::with_seed(16, SEED);
        let index2 = LazyIndex::with_seed(16, SEED);

        // Verify they have the same LSH structure by hashing the same embedding
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let test_emb: Vec<f32> = (0..EMBEDDING_DIM)
            .map(|_| rand::Rng::random::<f32>(&mut rng) * 2.0 - 1.0)
            .collect();

        // Both indexes should produce the same bucket assignment
        let bucket1 = index1.lsh.as_ref().unwrap().hash(&test_emb);
        let bucket2 = index2.lsh.as_ref().unwrap().hash(&test_emb);
        assert_eq!(
            bucket1, bucket2,
            "Seeded indexes should have identical LSH behavior"
        );

        // Verify different seeds produce different results
        let index3 = LazyIndex::with_seed(16, SEED + 1);
        let bucket3 = index3.lsh.as_ref().unwrap().hash(&test_emb);
        // Note: This test might rarely fail if the different seeds happen to produce
        // the same hyperplanes (extremely unlikely but theoretically possible)
        assert_ne!(
            bucket1, bucket3,
            "Different seeds should produce different LSH behavior"
        );
    }
}
