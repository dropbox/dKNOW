//! Product Quantization benchmarks
//!
//! Benchmarks the ProductQuantizer which compresses 512-byte embeddings
//! down to 16 bytes (32x compression) for efficient storage and search.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use sg_core::{ProductQuantizer, EMBEDDING_DIM};

/// Generate random normalized embedding vector
fn random_embedding(seed: u64) -> Vec<f32> {
    let mut data = Vec::with_capacity(EMBEDDING_DIM);

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

/// Create a trained quantizer for benchmarking
fn trained_quantizer(num_training: usize) -> ProductQuantizer {
    let training_data: Vec<Vec<f32>> = (0..num_training)
        .map(|i| random_embedding(i as u64))
        .collect();

    let mut pq = ProductQuantizer::new();
    pq.train(&training_data).expect("training failed");
    pq
}

/// Benchmark quantizer training with different dataset sizes
fn bench_pq_train(c: &mut Criterion) {
    let mut group = c.benchmark_group("pq_train");
    group.sample_size(10); // Training is slow, reduce sample count

    for num_training in [256, 500, 1000] {
        let training_data: Vec<Vec<f32>> = (0..num_training)
            .map(|i| random_embedding(i as u64))
            .collect();

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{num_training}_embeddings")),
            &training_data,
            |b, training_data| {
                b.iter(|| {
                    let mut pq = ProductQuantizer::new();
                    pq.train(black_box(training_data)).unwrap();
                    black_box(pq)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark encoding embeddings to quantized form
fn bench_pq_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("pq_encode");

    let pq = trained_quantizer(500);
    let embeddings: Vec<Vec<f32>> = (0..100)
        .map(|i| random_embedding(i as u64 + 10000))
        .collect();

    // Single embedding encode
    group.bench_function("single", |b| {
        let emb = &embeddings[0];
        b.iter(|| black_box(pq.encode(black_box(emb)).unwrap()));
    });

    // Batch encoding
    for batch_size in [10, 50, 100] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("batch_{batch_size}")),
            &embeddings[..batch_size],
            |b, embeddings| {
                b.iter(|| {
                    let mut encoded = Vec::with_capacity(embeddings.len());
                    for emb in embeddings.iter() {
                        encoded.push(pq.encode(black_box(emb)).unwrap());
                    }
                    black_box(encoded)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark decoding quantized embeddings back to full precision
fn bench_pq_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("pq_decode");

    let pq = trained_quantizer(500);
    let embeddings: Vec<Vec<f32>> = (0..100)
        .map(|i| random_embedding(i as u64 + 10000))
        .collect();
    let encoded: Vec<_> = embeddings.iter().map(|e| pq.encode(e).unwrap()).collect();

    // Single decode
    group.bench_function("single", |b| {
        let enc = &encoded[0];
        b.iter(|| black_box(pq.decode(black_box(enc))));
    });

    // Batch decoding
    for batch_size in [10, 50, 100] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("batch_{batch_size}")),
            &encoded[..batch_size],
            |b, encoded| {
                b.iter(|| {
                    let mut decoded = Vec::with_capacity(encoded.len());
                    for enc in encoded.iter() {
                        decoded.push(pq.decode(black_box(enc)));
                    }
                    black_box(decoded)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark distance table computation (done once per query)
fn bench_pq_distance_table(c: &mut Criterion) {
    let mut group = c.benchmark_group("pq_distance_table");

    let pq = trained_quantizer(500);
    let query = random_embedding(99999);

    group.bench_function("compute", |b| {
        b.iter(|| black_box(pq.compute_distance_table(black_box(&query)).unwrap()));
    });

    group.finish();
}

/// Benchmark asymmetric distance computation (hot path during search)
fn bench_pq_asymmetric_distance(c: &mut Criterion) {
    let mut group = c.benchmark_group("pq_asymmetric_distance");

    let pq = trained_quantizer(500);
    let query = random_embedding(99999);
    let distance_table = pq.compute_distance_table(&query).unwrap();

    // Encode some documents
    let embeddings: Vec<Vec<f32>> = (0..1000)
        .map(|i| random_embedding(i as u64 + 10000))
        .collect();
    let encoded: Vec<_> = embeddings.iter().map(|e| pq.encode(e).unwrap()).collect();

    // Single distance computation
    group.bench_function("single", |b| {
        let enc = &encoded[0];
        b.iter(|| black_box(distance_table.asymmetric_distance(black_box(enc))));
    });

    // Batch distance computation (simulates ranking candidates)
    for num_candidates in [10, 100, 500, 1000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{num_candidates}_candidates")),
            &encoded[..num_candidates],
            |b, encoded| {
                b.iter(|| {
                    let mut distances = Vec::with_capacity(encoded.len());
                    for enc in encoded.iter() {
                        distances.push(distance_table.asymmetric_distance(black_box(enc)));
                    }
                    black_box(distances)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_pq_train,
    bench_pq_encode,
    bench_pq_decode,
    bench_pq_distance_table,
    bench_pq_asymmetric_distance
);
criterion_main!(benches);
