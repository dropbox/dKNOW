//! Product Quantization (PQ) for embedding compression
//!
//! Implements product quantization to reduce embedding storage from 512 bytes
//! (128 × f32) to 16 bytes (16 subspaces × 1 byte index).
//!
//! Key concepts:
//! - Split 128-dim vector into M=16 subspaces of 8 dimensions each
//! - Train K=256 centroids per subspace via k-means
//! - Encode: find nearest centroid index in each subspace
//! - Decode: concatenate centroids to approximate original vector
//!
//! Search uses Asymmetric Distance Computation (ADC):
//! - Query is NOT quantized (use full precision)
//! - Database vectors are quantized
//! - Precompute query-to-centroid distances for each subspace
//! - Score = sum of precomputed distances for each code
//!
//! References:
//! - Jégou et al., "Product Quantization for Nearest Neighbor Search" (2011)
//! - <https://arxiv.org/abs/1702.08734> (Optimized Product Quantization)

use crate::embedder::EMBEDDING_DIM;
use rand::Rng;

/// Number of subspaces (M)
pub const NUM_SUBSPACES: usize = 16;

/// Dimensions per subspace (D/M = 128/16 = 8)
pub const SUBSPACE_DIM: usize = EMBEDDING_DIM / NUM_SUBSPACES;

/// Number of centroids per subspace (K = 256, fits in 1 byte)
pub const NUM_CENTROIDS: usize = 256;

/// Maximum k-means iterations for training
const MAX_KMEANS_ITERATIONS: usize = 25;

/// Convergence threshold for k-means
const CONVERGENCE_THRESHOLD: f32 = 1e-6;

/// Minimum training samples for PQ
pub const MIN_TRAINING_SAMPLES: usize = 256;

/// Product quantizer for embedding compression
#[derive(Clone)]
pub struct ProductQuantizer {
    /// Centroids for each subspace: [NUM_SUBSPACES][NUM_CENTROIDS][SUBSPACE_DIM]
    centroids: Vec<Vec<Vec<f32>>>,
    /// Whether the quantizer has been trained
    trained: bool,
}

/// Quantized embedding (16 bytes for 128-dim vector)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuantizedEmbedding {
    /// Centroid indices for each subspace
    pub codes: [u8; NUM_SUBSPACES],
}

/// Distance table for asymmetric distance computation
pub struct DistanceTable {
    /// Precomputed distances: [NUM_SUBSPACES][NUM_CENTROIDS]
    /// `table[m][k]` = squared distance from query subvector m to centroid k
    table: Vec<Vec<f32>>,
}

impl Default for ProductQuantizer {
    fn default() -> Self {
        Self::new()
    }
}

impl ProductQuantizer {
    /// Create a new untrained product quantizer
    pub fn new() -> Self {
        // Initialize with random centroids (will be trained later)
        let mut rng = rand::rng();
        let centroids = (0..NUM_SUBSPACES)
            .map(|_| {
                (0..NUM_CENTROIDS)
                    .map(|_| {
                        (0..SUBSPACE_DIM)
                            .map(|_| rng.random::<f32>() * 2.0 - 1.0)
                            .collect()
                    })
                    .collect()
            })
            .collect();

        Self {
            centroids,
            trained: false,
        }
    }

    /// Check if the quantizer has been trained
    pub fn is_trained(&self) -> bool {
        self.trained
    }

    /// Train the product quantizer on a set of embeddings
    ///
    /// Uses k-means clustering in each subspace to learn optimal centroids.
    pub fn train(&mut self, embeddings: &[Vec<f32>]) -> anyhow::Result<()> {
        if embeddings.len() < MIN_TRAINING_SAMPLES {
            anyhow::bail!(
                "Need at least {} training samples, got {}",
                MIN_TRAINING_SAMPLES,
                embeddings.len()
            );
        }

        // Validate embedding dimensions
        for emb in embeddings {
            if emb.len() != EMBEDDING_DIM {
                anyhow::bail!(
                    "Embedding dimension mismatch: expected {}, got {}",
                    EMBEDDING_DIM,
                    emb.len()
                );
            }
        }

        // Train each subspace independently
        for m in 0..NUM_SUBSPACES {
            // Extract subvectors for this subspace
            let subvectors: Vec<Vec<f32>> = embeddings
                .iter()
                .map(|emb| {
                    let start = m * SUBSPACE_DIM;
                    let end = start + SUBSPACE_DIM;
                    emb[start..end].to_vec()
                })
                .collect();

            // Run k-means to find centroids
            self.centroids[m] = kmeans_subspace(&subvectors, NUM_CENTROIDS)?;
        }

        self.trained = true;
        Ok(())
    }

    /// Encode an embedding to quantized form
    pub fn encode(&self, embedding: &[f32]) -> anyhow::Result<QuantizedEmbedding> {
        if embedding.len() != EMBEDDING_DIM {
            anyhow::bail!(
                "Embedding dimension mismatch: expected {}, got {}",
                EMBEDDING_DIM,
                embedding.len()
            );
        }

        let mut codes = [0u8; NUM_SUBSPACES];

        for (m, code) in codes.iter_mut().enumerate() {
            let start = m * SUBSPACE_DIM;
            let end = start + SUBSPACE_DIM;
            let subvector = &embedding[start..end];

            // Find nearest centroid
            let mut best_k = 0;
            let mut best_dist = f32::INFINITY;

            for (k, centroid) in self.centroids[m].iter().enumerate() {
                let dist = squared_distance(subvector, centroid);
                if dist < best_dist {
                    best_dist = dist;
                    best_k = k;
                }
            }

            *code = best_k as u8;
        }

        Ok(QuantizedEmbedding { codes })
    }

    /// Decode a quantized embedding back to approximate full embedding
    pub fn decode(&self, quantized: &QuantizedEmbedding) -> Vec<f32> {
        let mut embedding = Vec::with_capacity(EMBEDDING_DIM);

        for m in 0..NUM_SUBSPACES {
            let k = quantized.codes[m] as usize;
            embedding.extend_from_slice(&self.centroids[m][k]);
        }

        embedding
    }

    /// Create a distance table for asymmetric distance computation
    ///
    /// For a query vector, precompute distances from each query subvector
    /// to all centroids. This allows O(M) distance computation per database
    /// vector instead of O(D).
    pub fn compute_distance_table(&self, query: &[f32]) -> anyhow::Result<DistanceTable> {
        if query.len() != EMBEDDING_DIM {
            anyhow::bail!(
                "Query dimension mismatch: expected {}, got {}",
                EMBEDDING_DIM,
                query.len()
            );
        }

        let table = (0..NUM_SUBSPACES)
            .map(|m| {
                let start = m * SUBSPACE_DIM;
                let end = start + SUBSPACE_DIM;
                let query_sub = &query[start..end];

                self.centroids[m]
                    .iter()
                    .map(|centroid| squared_distance(query_sub, centroid))
                    .collect()
            })
            .collect();

        Ok(DistanceTable { table })
    }

    /// Export centroids for persistence
    pub fn export_centroids(&self) -> Vec<f32> {
        let mut data = Vec::with_capacity(NUM_SUBSPACES * NUM_CENTROIDS * SUBSPACE_DIM);

        for subspace in &self.centroids {
            for centroid in subspace {
                data.extend_from_slice(centroid);
            }
        }

        data
    }

    /// Import centroids from persistence
    pub fn import_centroids(&mut self, data: &[f32]) -> anyhow::Result<()> {
        let expected_size = NUM_SUBSPACES * NUM_CENTROIDS * SUBSPACE_DIM;
        if data.len() != expected_size {
            anyhow::bail!(
                "Centroid data size mismatch: expected {}, got {}",
                expected_size,
                data.len()
            );
        }

        let mut offset = 0;
        for m in 0..NUM_SUBSPACES {
            for k in 0..NUM_CENTROIDS {
                self.centroids[m][k] = data[offset..offset + SUBSPACE_DIM].to_vec();
                offset += SUBSPACE_DIM;
            }
        }

        self.trained = true;
        Ok(())
    }
}

impl DistanceTable {
    /// Compute asymmetric squared distance to a quantized embedding
    ///
    /// This is O(M) instead of O(D) because distances are precomputed.
    pub fn asymmetric_distance(&self, quantized: &QuantizedEmbedding) -> f32 {
        let mut total = 0.0;

        for m in 0..NUM_SUBSPACES {
            let k = quantized.codes[m] as usize;
            total += self.table[m][k];
        }

        total
    }

    /// Compute asymmetric cosine similarity to a quantized embedding
    ///
    /// Note: This assumes the query and database vectors are already
    /// L2-normalized. The distance table stores squared distances,
    /// so we convert using: cos_sim = 1 - dist²/2
    pub fn asymmetric_cosine_similarity(&self, quantized: &QuantizedEmbedding) -> f32 {
        let dist_sq = self.asymmetric_distance(quantized);
        // For normalized vectors: ||a - b||² = 2 - 2*cos(a,b)
        // So cos(a,b) = 1 - ||a - b||²/2
        1.0 - dist_sq / 2.0
    }
}

impl QuantizedEmbedding {
    /// Create from raw bytes
    pub fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        if bytes.len() != NUM_SUBSPACES {
            anyhow::bail!(
                "Invalid quantized embedding size: expected {}, got {}",
                NUM_SUBSPACES,
                bytes.len()
            );
        }

        let mut codes = [0u8; NUM_SUBSPACES];
        codes.copy_from_slice(bytes);

        Ok(Self { codes })
    }

    /// Convert to raw bytes
    pub fn to_bytes(&self) -> &[u8; NUM_SUBSPACES] {
        &self.codes
    }
}

/// Squared Euclidean distance between two vectors
fn squared_distance(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| {
            let diff = x - y;
            diff * diff
        })
        .sum()
}

/// K-means clustering for a single subspace
fn kmeans_subspace(data: &[Vec<f32>], k: usize) -> anyhow::Result<Vec<Vec<f32>>> {
    if data.is_empty() {
        anyhow::bail!("Cannot run k-means on empty data");
    }

    let dim = data[0].len();

    // Initialize centroids using k-means++ style initialization
    let mut centroids = initialize_centroids(data, k);

    let mut prev_inertia = f32::INFINITY;

    for _iter in 0..MAX_KMEANS_ITERATIONS {
        // Assign each point to nearest centroid
        let assignments: Vec<usize> = data
            .iter()
            .map(|point| {
                centroids
                    .iter()
                    .enumerate()
                    .map(|(i, c)| (i, squared_distance(point, c)))
                    .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                    .unwrap()
                    .0
            })
            .collect();

        // Update centroids
        let mut new_centroids = vec![vec![0.0; dim]; k];
        let mut counts = vec![0usize; k];

        for (point, &cluster) in data.iter().zip(assignments.iter()) {
            for (j, &val) in point.iter().enumerate() {
                new_centroids[cluster][j] += val;
            }
            counts[cluster] += 1;
        }

        // Average and handle empty clusters
        for (centroid, &count) in new_centroids.iter_mut().zip(counts.iter()) {
            if count > 0 {
                for val in centroid.iter_mut() {
                    *val /= count as f32;
                }
            } else {
                // Empty cluster: reinitialize to random point
                let random_point = &data[rand::rng().random_range(0..data.len())];
                centroid.clone_from(random_point);
            }
        }

        // Calculate inertia (sum of squared distances to centroids)
        let inertia: f32 = data
            .iter()
            .zip(assignments.iter())
            .map(|(point, &cluster)| squared_distance(point, &new_centroids[cluster]))
            .sum();

        // Check for convergence
        if (prev_inertia - inertia).abs() < CONVERGENCE_THRESHOLD * prev_inertia {
            centroids = new_centroids;
            break;
        }

        prev_inertia = inertia;
        centroids = new_centroids;
    }

    Ok(centroids)
}

/// Initialize centroids using k-means++ style selection
fn initialize_centroids(data: &[Vec<f32>], k: usize) -> Vec<Vec<f32>> {
    let mut rng = rand::rng();
    let mut centroids = Vec::with_capacity(k);

    // Choose first centroid randomly
    let first_idx = rng.random_range(0..data.len());
    centroids.push(data[first_idx].clone());

    // Choose remaining centroids with probability proportional to distance²
    while centroids.len() < k {
        let distances: Vec<f32> = data
            .iter()
            .map(|point| {
                centroids
                    .iter()
                    .map(|c| squared_distance(point, c))
                    .min_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap()
            })
            .collect();

        let total: f32 = distances.iter().sum();
        if total == 0.0 {
            // All points are at existing centroids, just pick random
            let idx = rng.random_range(0..data.len());
            centroids.push(data[idx].clone());
        } else {
            // Weighted random selection
            let threshold = rng.random::<f32>() * total;
            let mut cumsum = 0.0;
            for (i, &dist) in distances.iter().enumerate() {
                cumsum += dist;
                if cumsum >= threshold {
                    centroids.push(data[i].clone());
                    break;
                }
            }
            // Fallback if we somehow didn't select one
            if centroids.len() < k {
                let idx = rng.random_range(0..data.len());
                centroids.push(data[idx].clone());
            }
        }
    }

    centroids
}

#[cfg(test)]
mod tests {
    use super::*;

    fn random_embedding() -> Vec<f32> {
        let mut rng = rand::rng();
        (0..EMBEDDING_DIM)
            .map(|_| rng.random::<f32>() * 2.0 - 1.0)
            .collect()
    }

    fn normalize(v: &mut [f32]) {
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in v.iter_mut() {
                *x /= norm;
            }
        }
    }

    #[test]
    fn test_quantizer_dimensions() {
        // Verify dimension constants are consistent
        assert_eq!(NUM_SUBSPACES * SUBSPACE_DIM, EMBEDDING_DIM);
        assert_eq!(EMBEDDING_DIM, 128);
        assert_eq!(NUM_SUBSPACES, 16);
        assert_eq!(SUBSPACE_DIM, 8);
        assert_eq!(NUM_CENTROIDS, 256);
    }

    #[test]
    fn test_quantized_embedding_size() {
        // Quantized embedding should be exactly 16 bytes
        let qe = QuantizedEmbedding {
            codes: [0u8; NUM_SUBSPACES],
        };
        assert_eq!(qe.to_bytes().len(), 16);
        assert_eq!(std::mem::size_of::<QuantizedEmbedding>(), 16);
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let pq = ProductQuantizer::new();
        let original = random_embedding();

        let quantized = pq.encode(&original).unwrap();
        let decoded = pq.decode(&quantized);

        // Decoded should have same dimension
        assert_eq!(decoded.len(), EMBEDDING_DIM);

        // Note: Without training, reconstruction won't be accurate
        // This just tests the roundtrip mechanics
    }

    #[test]
    fn test_encode_dimension_mismatch() {
        let pq = ProductQuantizer::new();
        let wrong_size = vec![0.0f32; 64]; // Wrong size

        let result = pq.encode(&wrong_size);
        assert!(result.is_err());
    }

    #[test]
    fn test_distance_table_creation() {
        let pq = ProductQuantizer::new();
        let query = random_embedding();

        let table = pq.compute_distance_table(&query).unwrap();

        // Table should have distances for all subspaces and centroids
        assert_eq!(table.table.len(), NUM_SUBSPACES);
        for subtable in &table.table {
            assert_eq!(subtable.len(), NUM_CENTROIDS);
        }
    }

    #[test]
    fn test_distance_table_dimension_mismatch() {
        let pq = ProductQuantizer::new();
        let wrong_size = vec![0.0f32; 64];

        let result = pq.compute_distance_table(&wrong_size);
        assert!(result.is_err());
    }

    #[test]
    fn test_asymmetric_distance_is_positive() {
        let pq = ProductQuantizer::new();
        let query = random_embedding();
        let emb = random_embedding();

        let quantized = pq.encode(&emb).unwrap();
        let table = pq.compute_distance_table(&query).unwrap();
        let dist = table.asymmetric_distance(&quantized);

        assert!(dist >= 0.0, "Distance should be non-negative");
    }

    #[test]
    fn test_asymmetric_distance_zero_to_self() {
        let pq = ProductQuantizer::new();
        let emb = random_embedding();

        let quantized = pq.encode(&emb).unwrap();
        let decoded = pq.decode(&quantized);
        let table = pq.compute_distance_table(&decoded).unwrap();
        let dist = table.asymmetric_distance(&quantized);

        // Distance to self (after decode) should be very small
        assert!(
            dist < 1e-5,
            "Distance from decoded embedding to its quantized form should be ~0"
        );
    }

    #[test]
    fn test_quantized_embedding_from_bytes() {
        let bytes = [1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let qe = QuantizedEmbedding::from_bytes(&bytes).unwrap();

        assert_eq!(qe.codes, bytes);
        assert_eq!(qe.to_bytes(), &bytes);
    }

    #[test]
    fn test_quantized_embedding_from_bytes_wrong_size() {
        let bytes = [1u8, 2, 3]; // Too small
        let result = QuantizedEmbedding::from_bytes(&bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_export_import_centroids() {
        let pq1 = ProductQuantizer::new();
        let exported = pq1.export_centroids();

        // Should have correct size
        let expected_size = NUM_SUBSPACES * NUM_CENTROIDS * SUBSPACE_DIM;
        assert_eq!(exported.len(), expected_size);

        // Import into new quantizer
        let mut pq2 = ProductQuantizer::new();
        pq2.import_centroids(&exported).unwrap();

        // Encoding should produce same results
        let emb = random_embedding();
        let q1 = pq1.encode(&emb).unwrap();
        let q2 = pq2.encode(&emb).unwrap();
        assert_eq!(q1.codes, q2.codes);
    }

    #[test]
    fn test_import_centroids_wrong_size() {
        let mut pq = ProductQuantizer::new();
        let bad_data = vec![0.0f32; 100]; // Wrong size

        let result = pq.import_centroids(&bad_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_squared_distance() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 0.0, 0.0];
        assert!((squared_distance(&a, &b) - 1.0).abs() < 1e-6);

        let c = vec![1.0, 1.0, 1.0];
        let d = vec![0.0, 0.0, 0.0];
        assert!((squared_distance(&c, &d) - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_train_requires_min_samples() {
        let mut pq = ProductQuantizer::new();
        let few_samples: Vec<Vec<f32>> = (0..10).map(|_| random_embedding()).collect();

        let result = pq.train(&few_samples);
        assert!(result.is_err());
    }

    #[test]
    fn test_train_dimension_mismatch() {
        let mut pq = ProductQuantizer::new();
        let bad_samples = vec![vec![0.0f32; 64]; MIN_TRAINING_SAMPLES]; // Wrong dim

        let result = pq.train(&bad_samples);
        assert!(result.is_err());
    }

    #[test]
    fn test_train_and_encode() {
        let mut pq = ProductQuantizer::new();

        // Generate training data
        let training_data: Vec<Vec<f32>> = (0..MIN_TRAINING_SAMPLES * 2)
            .map(|_| random_embedding())
            .collect();

        // Train should succeed
        pq.train(&training_data).unwrap();
        assert!(pq.is_trained());

        // Encoding should still work
        let emb = random_embedding();
        let quantized = pq.encode(&emb).unwrap();
        assert_eq!(quantized.codes.len(), NUM_SUBSPACES);
    }

    #[test]
    fn test_trained_quantizer_better_reconstruction() {
        let mut pq = ProductQuantizer::new();

        // Generate clustered training data (more structure = better PQ)
        let mut training_data = Vec::new();
        let mut rng = rand::rng();

        for _cluster in 0..16 {
            // Generate cluster center
            let center: Vec<f32> = (0..EMBEDDING_DIM)
                .map(|_| rng.random::<f32>() * 2.0 - 1.0)
                .collect();

            // Generate points around center
            for _ in 0..64 {
                let point: Vec<f32> = center
                    .iter()
                    .map(|&c| c + rng.random::<f32>() * 0.2 - 0.1)
                    .collect();
                training_data.push(point);
            }
        }

        // Train the quantizer
        pq.train(&training_data).unwrap();

        // Test reconstruction on training data
        let mut total_error = 0.0;
        for emb in training_data.iter().take(100) {
            let quantized = pq.encode(emb).unwrap();
            let decoded = pq.decode(&quantized);
            let error: f32 = emb
                .iter()
                .zip(decoded.iter())
                .map(|(a, b)| (a - b).powi(2))
                .sum();
            total_error += error;
        }
        let avg_error = total_error / 100.0;

        // Trained PQ should have reasonable reconstruction error
        // (much better than random centroids)
        assert!(
            avg_error < EMBEDDING_DIM as f32,
            "Average reconstruction error {avg_error} is too high"
        );
    }

    #[test]
    fn test_asymmetric_cosine_similarity() {
        let mut pq = ProductQuantizer::new();

        // Generate training data
        let training_data: Vec<Vec<f32>> = (0..MIN_TRAINING_SAMPLES * 2)
            .map(|_| {
                let mut v = random_embedding();
                normalize(&mut v);
                v
            })
            .collect();

        pq.train(&training_data).unwrap();

        // Test cosine similarity on normalized vectors
        let mut query = random_embedding();
        normalize(&mut query);

        let mut emb = random_embedding();
        normalize(&mut emb);

        let quantized = pq.encode(&emb).unwrap();
        let table = pq.compute_distance_table(&query).unwrap();
        let sim = table.asymmetric_cosine_similarity(&quantized);

        // Cosine similarity should be in [-1, 1] range
        // (approximately, due to quantization)
        assert!(
            (-1.5..=1.5).contains(&sim),
            "Cosine similarity {sim} out of expected range"
        );
    }

    #[test]
    fn test_quantizer_default() {
        let pq = ProductQuantizer::default();
        assert!(!pq.is_trained());
        assert_eq!(pq.centroids.len(), NUM_SUBSPACES);
    }

    #[test]
    fn test_compression_ratio() {
        // Original: 128 f32 = 512 bytes
        // Quantized: 16 u8 = 16 bytes
        // Compression ratio: 32x

        let original_size = EMBEDDING_DIM * std::mem::size_of::<f32>();
        let quantized_size = std::mem::size_of::<QuantizedEmbedding>();

        assert_eq!(original_size, 512);
        assert_eq!(quantized_size, 16);
        assert_eq!(original_size / quantized_size, 32);
    }
}
