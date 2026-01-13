//! K-means clustering benchmarks
//!
//! Benchmarks the LazyIndex online k-means clustering which assigns embeddings
//! to clusters during indexing and selects candidate clusters during search.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use sg_core::{LazyIndex, EMBEDDING_DIM};

/// Generate random normalized embedding vector (single token)
fn random_embedding(seed: u64) -> Vec<f32> {
    let mut data = Vec::with_capacity(EMBEDDING_DIM);

    // Simple deterministic pseudo-random for reproducibility
    let mut s = seed;
    for _ in 0..EMBEDDING_DIM {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
        let val = ((s >> 33) as f32) / (u32::MAX as f32) - 0.5;
        data.push(val);
    }

    // L2 normalize
    let norm: f32 = data.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in &mut data {
            *x /= norm;
        }
    }

    data
}

/// Benchmark adding embeddings to the index
fn bench_kmeans_add(c: &mut Criterion) {
    let mut group = c.benchmark_group("kmeans_add");

    // Generate embeddings upfront
    let embeddings: Vec<Vec<f32>> = (0..1000).map(|i| random_embedding(i as u64)).collect();

    for num_clusters in [16, 32, 64, 128] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{num_clusters}_clusters")),
            &num_clusters,
            |b, &num_clusters| {
                b.iter_with_setup(
                    || LazyIndex::new(num_clusters),
                    |mut index| {
                        // Add 100 embeddings to measure add throughput
                        for (i, emb) in embeddings.iter().take(100).enumerate() {
                            let _ = index.add(black_box(i as u32), black_box(emb));
                        }
                        black_box(index)
                    },
                );
            },
        );
    }

    group.finish();
}

/// Benchmark the improve() operation which refines cluster centers
fn bench_kmeans_improve(c: &mut Criterion) {
    let mut group = c.benchmark_group("kmeans_improve");

    for num_docs in [100, 500, 1000] {
        let embeddings: Vec<Vec<f32>> = (0..num_docs).map(|i| random_embedding(i as u64)).collect();

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{num_docs}_docs")),
            &embeddings,
            |b, embeddings| {
                b.iter_with_setup(
                    || {
                        let mut index = LazyIndex::new(32);
                        for (i, emb) in embeddings.iter().enumerate() {
                            let _ = index.add(i as u32, emb);
                        }
                        index
                    },
                    |mut index| {
                        index.improve();
                        black_box(index)
                    },
                );
            },
        );
    }

    group.finish();
}

/// Benchmark cluster selection during search
fn bench_kmeans_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("kmeans_search");

    // Build an index with a realistic number of documents
    let num_docs = 1000;
    let embeddings: Vec<Vec<f32>> = (0..num_docs).map(|i| random_embedding(i as u64)).collect();

    let mut index = LazyIndex::new(32);
    for (i, emb) in embeddings.iter().enumerate() {
        let _ = index.add(i as u32, emb);
    }
    // Run improve to get good cluster centers
    for _ in 0..5 {
        index.improve();
    }

    // Query embedding
    let query = random_embedding(99999);

    for top_k in [5, 10, 20, 50] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("top_{top_k}")),
            &top_k,
            |b, &top_k| {
                b.iter(|| black_box(index.search(black_box(&query), black_box(top_k)).unwrap()));
            },
        );
    }

    group.finish();
}

/// Benchmark cluster selection with varying cluster counts
fn bench_kmeans_cluster_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("kmeans_cluster_scaling");

    let num_docs = 500;
    let embeddings: Vec<Vec<f32>> = (0..num_docs).map(|i| random_embedding(i as u64)).collect();
    let query = random_embedding(99999);

    for num_clusters in [16, 32, 64, 128] {
        let mut index = LazyIndex::new(num_clusters);
        for (i, emb) in embeddings.iter().enumerate() {
            let _ = index.add(i as u32, emb);
        }
        for _ in 0..3 {
            index.improve();
        }

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{num_clusters}_clusters")),
            &index,
            |b, index| {
                b.iter(|| black_box(index.search(black_box(&query), black_box(10)).unwrap()));
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_kmeans_add,
    bench_kmeans_improve,
    bench_kmeans_search,
    bench_kmeans_cluster_scaling
);
criterion_main!(benches);
