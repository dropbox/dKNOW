//! MaxSim scoring benchmarks
//!
//! Benchmarks the core MaxSim algorithm which computes similarity between
//! multi-vector embeddings. This is the hot path during search.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use sg_core::{maxsim_from_vecs, EMBEDDING_DIM};

/// Generate random normalized embedding vector
fn random_embedding(num_tokens: usize) -> Vec<f32> {
    let total = num_tokens * EMBEDDING_DIM;
    let mut data = Vec::with_capacity(total);

    // Simple deterministic pseudo-random for reproducibility
    let mut seed: u64 = 12345;
    for _ in 0..total {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let val = ((seed >> 33) as f32) / (u32::MAX as f32) - 0.5;
        data.push(val);
    }

    // L2 normalize each token's embedding
    for i in 0..num_tokens {
        let start = i * EMBEDDING_DIM;
        let end = start + EMBEDDING_DIM;
        let norm: f32 = data[start..end].iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in &mut data[start..end] {
                *x /= norm;
            }
        }
    }

    data
}

/// Benchmark MaxSim with varying query sizes
fn bench_maxsim_query_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("maxsim_query_size");

    // Fixed document size (typical chunk: ~100 tokens)
    let doc_tokens = 100;
    let doc_emb = random_embedding(doc_tokens);

    // Varying query sizes
    for query_tokens in [8, 16, 32, 64] {
        let query_emb = random_embedding(query_tokens);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("query_{query_tokens}_doc_{doc_tokens}")),
            &(query_emb, doc_emb.clone()),
            |b, (query, doc)| {
                b.iter(|| {
                    black_box(maxsim_from_vecs(
                        black_box(query),
                        query_tokens,
                        black_box(doc),
                        doc_tokens,
                    ))
                });
            },
        );
    }

    group.finish();
}

/// Benchmark MaxSim with varying document sizes
fn bench_maxsim_doc_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("maxsim_doc_size");

    // Fixed query size (typical: 16 tokens)
    let query_tokens = 16;
    let query_emb = random_embedding(query_tokens);

    // Varying document sizes (different chunk sizes)
    for doc_tokens in [50, 100, 200, 400] {
        let doc_emb = random_embedding(doc_tokens);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("query_{query_tokens}_doc_{doc_tokens}")),
            &(query_emb.clone(), doc_emb),
            |b, (query, doc)| {
                b.iter(|| {
                    black_box(maxsim_from_vecs(
                        black_box(query),
                        query_tokens,
                        black_box(doc),
                        doc_tokens,
                    ))
                });
            },
        );
    }

    group.finish();
}

/// Benchmark scoring multiple documents (simulates search)
fn bench_maxsim_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("maxsim_batch");

    let query_tokens = 16;
    let query_emb = random_embedding(query_tokens);
    let doc_tokens = 100;

    // Varying number of documents to score (simulates candidate set)
    for num_docs in [10, 50, 100, 500] {
        let docs: Vec<Vec<f32>> = (0..num_docs)
            .map(|_| random_embedding(doc_tokens))
            .collect();

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{num_docs}_docs")),
            &docs,
            |b, docs| {
                b.iter(|| {
                    let mut scores = Vec::with_capacity(docs.len());
                    for doc in docs {
                        let score = maxsim_from_vecs(
                            black_box(&query_emb),
                            query_tokens,
                            black_box(doc),
                            doc_tokens,
                        );
                        scores.push(score);
                    }
                    black_box(scores)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_maxsim_query_size,
    bench_maxsim_doc_size,
    bench_maxsim_batch
);
criterion_main!(benches);
