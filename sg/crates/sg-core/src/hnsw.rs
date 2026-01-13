//! Hierarchical Navigable Small World (HNSW) graph for fast approximate nearest neighbor search.
//!
//! HNSW provides O(log n) approximate nearest neighbor search, significantly faster than
//! linear scan for large numbers of vectors. This implementation is optimized for
//! cluster center navigation where:
//! - Number of nodes is moderate (64-4096 clusters)
//! - Nodes change infrequently (on cluster split/merge)
//! - Queries are frequent (every search and insert)
//!
//! Reference: "Efficient and robust approximate nearest neighbor search using
//! Hierarchical Navigable Small World graphs" (Malkov & Yashunin, 2018)

use rand::Rng;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};

/// Default maximum number of neighbors per node at each layer
const DEFAULT_M: usize = 16;

/// Maximum neighbors for ground layer (layer 0) - typically 2*M
const DEFAULT_M0: usize = 32;

/// Size of dynamic candidate list during construction
const DEFAULT_EF_CONSTRUCTION: usize = 100;

/// Size of dynamic candidate list during search
const DEFAULT_EF_SEARCH: usize = 50;

/// Level multiplier for probabilistic layer selection (1/ln(M))
const DEFAULT_ML: f32 = 0.36; // ~1/ln(16)

/// Minimum nodes before HNSW provides benefit over linear scan
pub const MIN_NODES_FOR_HNSW: usize = 64;

/// Node in the HNSW graph
#[derive(Clone)]
struct HnswNode {
    /// Neighbors at each layer (layer -> neighbor ids)
    neighbors: Vec<Vec<usize>>,
    /// Maximum layer this node appears in
    max_layer: usize,
}

/// HNSW graph for approximate nearest neighbor search
pub struct HnswGraph {
    /// All nodes in the graph
    nodes: Vec<HnswNode>,
    /// Entry point (highest layer node)
    entry_point: Option<usize>,
    /// Maximum layer in the graph
    max_layer: usize,
    /// Max neighbors per node at layers > 0
    m: usize,
    /// Max neighbors per node at layer 0
    m0: usize,
    /// Construction ef (candidate list size)
    ef_construction: usize,
    /// Search ef (candidate list size)
    ef_search: usize,
    /// Level multiplier for random layer selection
    ml: f32,
}

/// Candidate for nearest neighbor search (score, node_id)
#[derive(Clone, Copy)]
struct Candidate {
    score: f32,
    id: usize,
}

impl PartialEq for Candidate {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score && self.id == other.id
    }
}

impl Eq for Candidate {}

impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse order for max-heap behavior (highest score first)
        other
            .score
            .partial_cmp(&self.score)
            .unwrap_or(Ordering::Equal)
    }
}

/// Min-heap candidate (for furthest neighbor tracking)
#[derive(Clone, Copy)]
struct MinCandidate {
    score: f32,
    id: usize,
}

impl PartialEq for MinCandidate {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score && self.id == other.id
    }
}

impl Eq for MinCandidate {}

impl PartialOrd for MinCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MinCandidate {
    fn cmp(&self, other: &Self) -> Ordering {
        // Normal order for min-heap behavior (lowest score first, to pop furthest)
        self.score
            .partial_cmp(&other.score)
            .unwrap_or(Ordering::Equal)
    }
}

impl HnswGraph {
    /// Create a new empty HNSW graph with default parameters
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            entry_point: None,
            max_layer: 0,
            m: DEFAULT_M,
            m0: DEFAULT_M0,
            ef_construction: DEFAULT_EF_CONSTRUCTION,
            ef_search: DEFAULT_EF_SEARCH,
            ml: DEFAULT_ML,
        }
    }

    /// Create HNSW graph with custom parameters
    pub fn with_params(m: usize, ef_construction: usize, ef_search: usize) -> Self {
        let m0 = m * 2;
        let ml = 1.0 / (m as f32).ln();
        Self {
            nodes: Vec::new(),
            entry_point: None,
            max_layer: 0,
            m,
            m0,
            ef_construction,
            ef_search,
            ml,
        }
    }

    /// Number of nodes in the graph
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Check if graph is empty
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Insert a new node into the graph
    ///
    /// The `embeddings` slice contains all node embeddings, where
    /// `embeddings[id * dim..(id+1) * dim]` is the embedding for node `id`.
    pub fn insert(&mut self, id: usize, embeddings: &[f32], dim: usize) {
        // Ensure we have space for this node
        while self.nodes.len() <= id {
            self.nodes.push(HnswNode {
                neighbors: Vec::new(),
                max_layer: 0,
            });
        }

        // Determine random layer for this node
        let node_layer = self.random_layer();

        // Initialize node's neighbor lists
        self.nodes[id].max_layer = node_layer;
        self.nodes[id].neighbors = (0..=node_layer).map(|_| Vec::new()).collect();

        if self.entry_point.is_none() {
            // First node
            self.entry_point = Some(id);
            self.max_layer = node_layer;
            return;
        }

        let entry_point = self.entry_point.unwrap();
        let query = Self::get_embedding(embeddings, id, dim);

        // Find entry point at the appropriate layer
        let mut ep = entry_point;

        // Traverse from top layer down to node_layer + 1
        for layer in (node_layer + 1..=self.max_layer).rev() {
            let nearest = self.search_layer_single(query, ep, layer, embeddings, dim);
            ep = nearest;
        }

        // Insert at layers from node_layer down to 0
        for layer in (0..=node_layer.min(self.max_layer)).rev() {
            let candidates =
                self.search_layer(query, ep, self.ef_construction, layer, embeddings, dim);

            // Select neighbors using simple heuristic
            let max_neighbors = if layer == 0 { self.m0 } else { self.m };
            let neighbors = self.select_neighbors(&candidates, max_neighbors);

            // Add bidirectional edges
            self.nodes[id].neighbors[layer] = neighbors.clone();

            for &neighbor_id in &neighbors {
                if neighbor_id < self.nodes.len() && layer < self.nodes[neighbor_id].neighbors.len()
                {
                    self.nodes[neighbor_id].neighbors[layer].push(id);

                    // Prune if over capacity
                    let neighbor_max = if layer == 0 { self.m0 } else { self.m };
                    if self.nodes[neighbor_id].neighbors[layer].len() > neighbor_max {
                        // Keep best neighbors
                        let neighbor_emb = Self::get_embedding(embeddings, neighbor_id, dim);
                        let mut scored: Vec<(f32, usize)> = self.nodes[neighbor_id].neighbors
                            [layer]
                            .iter()
                            .map(|&n| {
                                let n_emb = Self::get_embedding(embeddings, n, dim);
                                (cosine_similarity(neighbor_emb, n_emb), n)
                            })
                            .collect();
                        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(Ordering::Equal));
                        scored.truncate(neighbor_max);
                        self.nodes[neighbor_id].neighbors[layer] =
                            scored.into_iter().map(|(_, n)| n).collect();
                    }
                }
            }

            // Update entry point for next layer
            if !candidates.is_empty() {
                ep = candidates[0].1;
            }
        }

        // Update global entry point if this node reaches a higher layer
        if node_layer > self.max_layer {
            self.max_layer = node_layer;
            self.entry_point = Some(id);
        }
    }

    /// Search for k nearest neighbors to a query
    ///
    /// Returns (score, node_id) pairs sorted by score descending.
    pub fn search(
        &self,
        query: &[f32],
        k: usize,
        embeddings: &[f32],
        dim: usize,
    ) -> Vec<(f32, usize)> {
        if self.entry_point.is_none() {
            return Vec::new();
        }

        let entry_point = self.entry_point.unwrap();
        let mut ep = entry_point;

        // Traverse from top layer down to layer 1
        for layer in (1..=self.max_layer).rev() {
            ep = self.search_layer_single(query, ep, layer, embeddings, dim);
        }

        // Search at layer 0 with ef candidates
        let candidates = self.search_layer(query, ep, self.ef_search.max(k), 0, embeddings, dim);

        // Return top-k
        candidates.into_iter().take(k).collect()
    }

    /// Get embedding slice for a node
    fn get_embedding(embeddings: &[f32], id: usize, dim: usize) -> &[f32] {
        let start = id * dim;
        let end = start + dim;
        if end <= embeddings.len() {
            &embeddings[start..end]
        } else {
            &[] // Return empty slice if out of bounds
        }
    }

    /// Generate random layer for a new node
    fn random_layer(&self) -> usize {
        let mut rng = rand::rng();
        let r: f32 = rng.random();
        (-r.ln() * self.ml).floor() as usize
    }

    /// Search a single layer for the nearest neighbor (greedy)
    fn search_layer_single(
        &self,
        query: &[f32],
        entry_point: usize,
        layer: usize,
        embeddings: &[f32],
        dim: usize,
    ) -> usize {
        let mut current = entry_point;
        let mut current_dist =
            cosine_similarity(query, Self::get_embedding(embeddings, current, dim));

        loop {
            let mut changed = false;

            if layer < self.nodes[current].neighbors.len() {
                for &neighbor in &self.nodes[current].neighbors[layer] {
                    if neighbor < self.nodes.len() {
                        let dist = cosine_similarity(
                            query,
                            Self::get_embedding(embeddings, neighbor, dim),
                        );
                        if dist > current_dist {
                            current = neighbor;
                            current_dist = dist;
                            changed = true;
                        }
                    }
                }
            }

            if !changed {
                break;
            }
        }

        current
    }

    /// Search a layer for ef nearest neighbors
    ///
    /// Returns (score, node_id) pairs sorted by score descending.
    fn search_layer(
        &self,
        query: &[f32],
        entry_point: usize,
        ef: usize,
        layer: usize,
        embeddings: &[f32],
        dim: usize,
    ) -> Vec<(f32, usize)> {
        let mut visited = HashSet::new();
        visited.insert(entry_point);

        // Max-heap for candidates (best first)
        let mut candidates = BinaryHeap::new();
        // Min-heap for result tracking (worst first, to know when to stop)
        let mut result = BinaryHeap::new();

        let entry_score =
            cosine_similarity(query, Self::get_embedding(embeddings, entry_point, dim));
        candidates.push(Candidate {
            score: entry_score,
            id: entry_point,
        });
        result.push(MinCandidate {
            score: entry_score,
            id: entry_point,
        });

        while let Some(current) = candidates.pop() {
            // Get worst result score
            let worst_result = result.peek().map(|c| c.score).unwrap_or(f32::NEG_INFINITY);

            // If current is worse than all results, stop
            if current.score < worst_result && result.len() >= ef {
                break;
            }

            // Explore neighbors
            if current.id < self.nodes.len() && layer < self.nodes[current.id].neighbors.len() {
                for &neighbor in &self.nodes[current.id].neighbors[layer] {
                    if !visited.contains(&neighbor) && neighbor < self.nodes.len() {
                        visited.insert(neighbor);

                        let score = cosine_similarity(
                            query,
                            Self::get_embedding(embeddings, neighbor, dim),
                        );

                        // Add to candidates if better than worst result
                        if score > worst_result || result.len() < ef {
                            candidates.push(Candidate {
                                score,
                                id: neighbor,
                            });
                            result.push(MinCandidate {
                                score,
                                id: neighbor,
                            });

                            // Prune result if over capacity
                            if result.len() > ef {
                                result.pop(); // Remove worst
                            }
                        }
                    }
                }
            }
        }

        // Convert result to sorted vec
        let mut sorted: Vec<(f32, usize)> = result.into_iter().map(|c| (c.score, c.id)).collect();
        sorted.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(Ordering::Equal));
        sorted
    }

    /// Select best neighbors from candidates using simple heuristic
    #[allow(clippy::unused_self)]
    fn select_neighbors(&self, candidates: &[(f32, usize)], max_neighbors: usize) -> Vec<usize> {
        // Simple: take top-scoring candidates
        candidates
            .iter()
            .take(max_neighbors)
            .map(|(_, id)| *id)
            .collect()
    }

    /// Rebuild the graph from scratch with given embeddings
    ///
    /// This is more efficient than incremental inserts when building
    /// from a known set of vectors.
    pub fn rebuild(&mut self, embeddings: &[f32], dim: usize, num_nodes: usize) {
        // Clear existing graph
        self.nodes.clear();
        self.entry_point = None;
        self.max_layer = 0;

        // Insert all nodes
        for id in 0..num_nodes {
            self.insert(id, embeddings, dim);
        }
    }

    /// Clear the graph
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.entry_point = None;
        self.max_layer = 0;
    }
}

impl Default for HnswGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.is_empty() || b.is_empty() || a.len() != b.len() {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a > 0.0 && norm_b > 0.0 {
        dot / (norm_a * norm_b)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIM: usize = 8;

    fn make_embedding(values: &[f32]) -> Vec<f32> {
        let mut v = vec![0.0; DIM];
        for (i, &val) in values.iter().enumerate() {
            if i < DIM {
                v[i] = val;
            }
        }
        // Normalize
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in &mut v {
                *x /= norm;
            }
        }
        v
    }

    #[test]
    fn test_empty_graph() {
        let graph = HnswGraph::new();
        assert!(graph.is_empty());
        assert_eq!(graph.len(), 0);
    }

    #[test]
    fn test_single_node() {
        let mut graph = HnswGraph::new();
        let embeddings = make_embedding(&[1.0, 0.0]);
        graph.insert(0, &embeddings, DIM);

        assert_eq!(graph.len(), 1);
        assert!(!graph.is_empty());

        let results = graph.search(&embeddings, 1, &embeddings, DIM);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].1, 0);
    }

    #[test]
    fn test_two_nodes() {
        let mut graph = HnswGraph::new();

        let emb1 = make_embedding(&[1.0, 0.0]);
        let emb2 = make_embedding(&[0.0, 1.0]);
        let mut embeddings = emb1.clone();
        embeddings.extend(&emb2);

        graph.insert(0, &embeddings, DIM);
        graph.insert(1, &embeddings, DIM);

        assert_eq!(graph.len(), 2);

        // Search for first node
        let results = graph.search(&emb1, 2, &embeddings, DIM);
        assert_eq!(results[0].1, 0);
    }

    #[test]
    fn test_multiple_nodes() {
        let mut graph = HnswGraph::new();
        let num_nodes = 10;

        let mut embeddings = Vec::new();
        for i in 0..num_nodes {
            let mut emb = vec![0.0; DIM];
            emb[i % DIM] = 1.0;
            let norm: f32 = emb.iter().map(|x| x * x).sum::<f32>().sqrt();
            for x in &mut emb {
                *x /= norm;
            }
            embeddings.extend(&emb);
        }

        for i in 0..num_nodes {
            graph.insert(i, &embeddings, DIM);
        }

        assert_eq!(graph.len(), num_nodes);

        // Search for each node
        for i in 0..num_nodes {
            let query = HnswGraph::get_embedding(&embeddings, i, DIM);
            let results = graph.search(query, 3, &embeddings, DIM);
            assert!(!results.is_empty());
            // The exact node should be first or close to first
            let found = results.iter().any(|(_, id)| *id == i);
            assert!(found, "Node {i} not found in search results");
        }
    }

    #[test]
    fn test_nearest_neighbor_accuracy() {
        let mut graph = HnswGraph::new();

        // Create nodes at different positions
        let node_embeddings = vec![
            make_embedding(&[1.0, 0.0, 0.0]), // Node 0
            make_embedding(&[0.9, 0.1, 0.0]), // Node 1 - close to 0
            make_embedding(&[0.0, 1.0, 0.0]), // Node 2
            make_embedding(&[0.0, 0.0, 1.0]), // Node 3
        ];

        let mut embeddings = Vec::new();
        for emb in &node_embeddings {
            embeddings.extend(emb);
        }

        for i in 0..4 {
            graph.insert(i, &embeddings, DIM);
        }

        // Query close to node 0
        let query = make_embedding(&[0.95, 0.05, 0.0]);
        let results = graph.search(&query, 2, &embeddings, DIM);

        // Node 0 or 1 should be first (both are close)
        assert!(results[0].1 == 0 || results[0].1 == 1);
    }

    #[test]
    fn test_rebuild() {
        let mut graph = HnswGraph::new();

        let mut embeddings = Vec::new();
        for i in 0..5 {
            let mut emb = vec![0.0; DIM];
            emb[i % DIM] = 1.0;
            embeddings.extend(&emb);
        }

        graph.rebuild(&embeddings, DIM, 5);
        assert_eq!(graph.len(), 5);

        // Search should work after rebuild
        let query = HnswGraph::get_embedding(&embeddings, 0, DIM);
        let results = graph.search(query, 3, &embeddings, DIM);
        assert!(!results.is_empty());
    }

    #[test]
    fn test_clear() {
        let mut graph = HnswGraph::new();

        let embeddings = make_embedding(&[1.0, 0.0]);
        graph.insert(0, &embeddings, DIM);
        assert_eq!(graph.len(), 1);

        graph.clear();
        assert!(graph.is_empty());
        assert_eq!(graph.len(), 0);
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);

        let c = vec![0.0, 1.0, 0.0];
        assert!(cosine_similarity(&a, &c).abs() < 1e-6);

        let d = vec![-1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &d) - (-1.0)).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_empty() {
        assert_eq!(cosine_similarity(&[], &[]), 0.0);
        assert_eq!(cosine_similarity(&[1.0], &[]), 0.0);
    }

    #[test]
    fn test_custom_params() {
        let graph = HnswGraph::with_params(8, 50, 25);
        assert!(graph.is_empty());
    }

    #[test]
    fn test_search_empty_graph() {
        let graph = HnswGraph::new();
        let query = vec![1.0; DIM];
        let results = graph.search(&query, 5, &[], DIM);
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_returns_correct_count() {
        let mut graph = HnswGraph::new();

        let mut embeddings = Vec::new();
        for i in 0..10 {
            let mut emb = vec![0.0; DIM];
            emb[i % DIM] = 1.0;
            embeddings.extend(&emb);
        }

        for i in 0..10 {
            graph.insert(i, &embeddings, DIM);
        }

        // Request more than available
        let query = HnswGraph::get_embedding(&embeddings, 0, DIM);
        let results = graph.search(query, 20, &embeddings, DIM);
        assert!(results.len() <= 10);

        // Request fewer than available
        let results = graph.search(query, 3, &embeddings, DIM);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_deterministic_search() {
        let mut graph = HnswGraph::new();

        let mut embeddings = Vec::new();
        for i in 0..5 {
            let mut emb = vec![0.0; DIM];
            emb[i % DIM] = 1.0;
            embeddings.extend(&emb);
        }

        // Rebuild multiple times and check consistency
        graph.rebuild(&embeddings, DIM, 5);

        let query = HnswGraph::get_embedding(&embeddings, 0, DIM);
        let results1 = graph.search(query, 3, &embeddings, DIM);

        // Search again - should give same results
        let results2 = graph.search(query, 3, &embeddings, DIM);

        assert_eq!(results1.len(), results2.len());
        for (r1, r2) in results1.iter().zip(results2.iter()) {
            assert_eq!(r1.1, r2.1);
        }
    }
}
