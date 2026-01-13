//! Memory profiling benchmarks for hot paths
//!
//! Measures memory allocations during key operations to track memory usage
//! and detect memory regressions.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use sg_core::memory::{format_bytes, get_rss_bytes, MemoryGuard};
use sg_core::quantizer::ProductQuantizer;
use sg_core::{make_device, Embedder, LazyIndex, EMBEDDING_DIM};
use std::sync::OnceLock;

/// Global embedder instance to avoid model loading in each benchmark
static EMBEDDER: OnceLock<std::sync::Mutex<Embedder>> = OnceLock::new();

fn get_embedder() -> &'static std::sync::Mutex<Embedder> {
    EMBEDDER.get_or_init(|| {
        let device = make_device();
        let embedder = Embedder::new(&device).expect("Failed to load embedder");
        std::sync::Mutex::new(embedder)
    })
}

/// Generate sample embedding vectors
fn generate_embeddings(count: usize, dim: usize) -> Vec<Vec<f32>> {
    (0..count)
        .map(|i| {
            (0..dim)
                .map(|j| ((i * dim + j) as f32 * 0.001).sin())
                .collect()
        })
        .collect()
}

/// Generate text of varying lengths
fn generate_text(word_count: usize) -> String {
    let words = [
        "semantic",
        "search",
        "embedding",
        "vector",
        "model",
        "transformer",
        "attention",
        "neural",
        "network",
        "query",
        "document",
        "retrieval",
        "similarity",
        "clustering",
        "index",
    ];
    let mut text = String::new();
    for i in 0..word_count {
        if i > 0 {
            text.push(' ');
        }
        text.push_str(words[i % words.len()]);
    }
    text
}

/// Benchmark memory usage during embedding generation
fn bench_embedding_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_embedding");
    group.sample_size(10);

    let embedder = get_embedder();

    for word_count in [50, 200, 500] {
        let text = generate_text(word_count);

        group.bench_with_input(BenchmarkId::new("embed", word_count), &text, |b, text| {
            b.iter_custom(|iters| {
                let mut total_peak = 0usize;
                let mut embedder = embedder.lock().unwrap();

                for _ in 0..iters {
                    let guard = MemoryGuard::new("embed");
                    let _ = embedder.embed_document(black_box(text));
                    let delta = guard.delta();
                    total_peak += delta.peak_delta;
                }

                // Return RSS delta as duration-like metric
                // (criterion displays this, though it's actually bytes)
                std::time::Duration::from_nanos(total_peak as u64 / iters.max(1))
            });
        });
    }

    group.finish();
}

/// Benchmark memory usage during batch embedding
fn bench_batch_embedding_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_batch_embed");
    group.sample_size(10);

    let embedder = get_embedder();

    for batch_size in [4, 8, 16] {
        let texts: Vec<String> = (0..batch_size).map(|_| generate_text(100)).collect();
        let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();

        group.bench_with_input(
            BenchmarkId::new("batch", batch_size),
            &text_refs,
            |b, texts| {
                b.iter_custom(|iters| {
                    let mut total_peak = 0usize;
                    let mut embedder = embedder.lock().unwrap();

                    for _ in 0..iters {
                        let guard = MemoryGuard::new("batch_embed");
                        let _ = embedder.embed_batch(black_box(texts), 512);
                        let delta = guard.delta();
                        total_peak += delta.peak_delta;
                    }

                    std::time::Duration::from_nanos(total_peak as u64 / iters.max(1))
                });
            },
        );
    }

    group.finish();
}

/// Benchmark memory usage during quantizer training
fn bench_quantizer_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_quantizer");
    group.sample_size(10);

    for sample_count in [256, 512, 1024] {
        let embeddings = generate_embeddings(sample_count, EMBEDDING_DIM);

        group.bench_with_input(
            BenchmarkId::new("train", sample_count),
            &embeddings,
            |b, embeddings| {
                b.iter_custom(|iters| {
                    let mut total_peak = 0usize;

                    for _ in 0..iters {
                        let guard = MemoryGuard::new("quantizer_train");
                        let mut quantizer = ProductQuantizer::new();
                        let _ = quantizer.train(black_box(embeddings));
                        let delta = guard.delta();
                        total_peak += delta.peak_delta;
                    }

                    std::time::Duration::from_nanos(total_peak as u64 / iters.max(1))
                });
            },
        );
    }

    group.finish();
}

/// Benchmark memory usage during index operations
fn bench_index_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_index");
    group.sample_size(10);

    for embedding_count in [100, 500, 1000] {
        let embeddings = generate_embeddings(embedding_count, EMBEDDING_DIM);

        group.bench_with_input(
            BenchmarkId::new("add_embeddings", embedding_count),
            &embeddings,
            |b, embeddings| {
                b.iter_custom(|iters| {
                    let mut total_peak = 0usize;

                    for _ in 0..iters {
                        let guard = MemoryGuard::new("index_add");
                        let mut index = LazyIndex::new(16);

                        for (i, emb) in embeddings.iter().enumerate() {
                            let _ = index.add(i as u32, black_box(emb));
                        }

                        let delta = guard.delta();
                        total_peak += delta.peak_delta;
                    }

                    std::time::Duration::from_nanos(total_peak as u64 / iters.max(1))
                });
            },
        );
    }

    group.finish();
}

/// Benchmark memory per embedding in LazyIndex
fn bench_index_memory_per_embedding(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_per_embedding");
    group.sample_size(10);

    for embedding_count in [1000, 5000, 10000] {
        let embeddings = generate_embeddings(embedding_count, EMBEDDING_DIM);

        group.bench_with_input(
            BenchmarkId::new("bytes_per_emb", embedding_count),
            &embeddings,
            |b, embeddings| {
                b.iter_custom(|_iters| {
                    let start_rss = get_rss_bytes().unwrap_or(0);

                    let mut index = LazyIndex::new(16);
                    for (i, emb) in embeddings.iter().enumerate() {
                        let _ = index.add(i as u32, emb);
                    }

                    let end_rss = get_rss_bytes().unwrap_or(0);
                    let memory_used = end_rss.saturating_sub(start_rss);
                    let bytes_per_emb = memory_used / embedding_count.max(1);

                    // Return bytes per embedding as duration (for display purposes)
                    std::time::Duration::from_nanos(bytes_per_emb as u64)
                });
            },
        );
    }

    group.finish();
}

/// Report current memory usage
fn report_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_report");
    group.sample_size(10);

    group.bench_function("rss_baseline", |b| {
        b.iter(|| {
            let rss = get_rss_bytes().unwrap_or(0);
            black_box(rss);
        });
    });

    // Report embedder memory after loading
    let embedder = get_embedder();
    drop(embedder.lock().unwrap()); // Ensure model is loaded

    if let Some(rss) = get_rss_bytes() {
        eprintln!("\n=== Memory Report ===");
        eprintln!("Process RSS after model load: {}", format_bytes(rss));
        eprintln!("=====================\n");
    }

    group.finish();
}

criterion_group!(
    benches,
    report_memory_usage,
    bench_embedding_memory,
    bench_batch_embedding_memory,
    bench_quantizer_memory,
    bench_index_memory,
    bench_index_memory_per_embedding,
);

criterion_main!(benches);
