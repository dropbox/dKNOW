//! Embedding throughput benchmarks
//!
//! Benchmarks embedding generation using the XTR model.
//! Note: These benchmarks require downloading/loading the model on first run.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use sg_core::{make_device, Embedder};
use std::sync::OnceLock;

/// Global embedder instance to avoid model loading in each benchmark
/// This is acceptable for benchmarking since we're measuring inference time, not loading time
static EMBEDDER: OnceLock<std::sync::Mutex<Embedder>> = OnceLock::new();

fn get_embedder() -> &'static std::sync::Mutex<Embedder> {
    EMBEDDER.get_or_init(|| {
        let device = make_device();
        let embedder = Embedder::new(&device).expect("Failed to load embedder");
        std::sync::Mutex::new(embedder)
    })
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

/// Maximum document length (tokens)
const DOC_MAXLEN: usize = 512;

/// Benchmark single document embedding with varying sizes
fn bench_embed_single(c: &mut Criterion) {
    let mut group = c.benchmark_group("embed_single");
    group.sample_size(20); // Fewer samples due to model inference time

    let embedder = get_embedder();

    for word_count in [50, 100, 200, 350] {
        let text = generate_text(word_count);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{word_count}_words")),
            &text,
            |b, text| {
                b.iter(|| {
                    let mut emb = embedder.lock().unwrap();
                    let result = emb.embed_document(black_box(text)).expect("embed failed");
                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark query embedding (shorter text, faster)
fn bench_embed_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("embed_query");
    group.sample_size(30);

    let embedder = get_embedder();

    for word_count in [3, 5, 10, 20] {
        let text = generate_text(word_count);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{word_count}_words")),
            &text,
            |b, text| {
                b.iter(|| {
                    let mut emb = embedder.lock().unwrap();
                    let result = emb.embed_query(black_box(text)).expect("embed failed");
                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark batch embedding (multiple documents at once)
fn bench_embed_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("embed_batch");
    group.sample_size(10); // Even fewer samples for batch operations

    let embedder = get_embedder();

    // Generate batch of documents
    let texts: Vec<String> = (0..16).map(|i| generate_text(100 + i * 10)).collect();
    let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();

    for batch_size in [4, 8, 16] {
        let batch: Vec<&str> = text_refs[..batch_size].to_vec();

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("batch_{batch_size}")),
            &batch,
            |b, batch| {
                b.iter(|| {
                    let mut emb = embedder.lock().unwrap();
                    let result = emb
                        .embed_batch(black_box(batch), DOC_MAXLEN)
                        .expect("embed failed");
                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_embed_single,
    bench_embed_query,
    bench_embed_batch
);
criterion_main!(benches);
