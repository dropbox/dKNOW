//! End-to-end search latency benchmarks
//!
//! Benchmarks the full search pipeline including query embedding and scoring.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use sg_core::{
    chunk_document, embeddings_to_vec, make_device, search, Embedder, SearchOptions, DB,
};
use std::path::Path;
use std::sync::OnceLock;

/// Global embedder instance
static EMBEDDER: OnceLock<std::sync::Mutex<Embedder>> = OnceLock::new();

fn get_embedder() -> &'static std::sync::Mutex<Embedder> {
    EMBEDDER.get_or_init(|| {
        let device = make_device();
        let embedder = Embedder::new(&device).expect("Failed to load embedder");
        std::sync::Mutex::new(embedder)
    })
}

/// Generate a corpus of documents
fn generate_corpus(num_docs: usize, words_per_doc: usize) -> Vec<(String, String)> {
    let topics = [
        "rust programming language memory safety ownership",
        "python machine learning neural network tensorflow",
        "javascript web development react frontend",
        "database sql query optimization indexing",
        "kubernetes docker container orchestration devops",
        "encryption security authentication authorization",
        "api rest graphql microservices architecture",
        "testing unit integration end to end automation",
    ];

    (0..num_docs)
        .map(|i| {
            let topic = topics[i % topics.len()];
            let mut content = format!("# Document {i} - {topic}\n\n");

            let words: Vec<&str> = topic.split_whitespace().collect();
            for _ in 0..(words_per_doc / 10) {
                for (j, word) in words.iter().enumerate() {
                    content.push_str(word);
                    content.push(' ');
                    if j % 5 == 4 {
                        content.push_str(". ");
                    }
                }
                content.push('\n');
            }

            (format!("/test/doc_{i}.md"), content)
        })
        .collect()
}

/// Create an in-memory database with indexed documents
fn setup_db(num_docs: usize, words_per_doc: usize) -> DB {
    let db = DB::in_memory().expect("Failed to create DB");
    let corpus = generate_corpus(num_docs, words_per_doc);

    let mut embedder_guard = get_embedder().lock().unwrap();

    for (path_str, content) in corpus {
        // Add document
        let path = Path::new(&path_str);
        let doc_id = db
            .add_document(path, &content)
            .expect("Failed to add document");

        // Chunk and embed
        let chunks = chunk_document(&content);
        for chunk in chunks {
            let tensor = embedder_guard
                .embed_document(&chunk.content)
                .expect("Failed to embed");

            let num_tokens = tensor.dims()[0];
            let data = embeddings_to_vec(&tensor).expect("Failed to convert embeddings");

            let chunk_id = db
                .add_chunk(doc_id, chunk.index, chunk.start_line, chunk.end_line, "")
                .expect("Failed to add chunk");

            db.add_chunk_embeddings(chunk_id, &data, num_tokens)
                .expect("Failed to add embeddings");
        }
    }

    db
}

/// Benchmark search with varying corpus sizes
fn bench_search_corpus_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_corpus_size");
    group.sample_size(20);

    for num_docs in [10, 50, 100] {
        let db = setup_db(num_docs, 500);
        let queries = [
            "rust memory safety",
            "python neural network",
            "database optimization",
        ];

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{num_docs}_docs")),
            &db,
            |b, db| {
                b.iter(|| {
                    let mut embedder = get_embedder().lock().unwrap();
                    let query = queries[0];
                    let opts = SearchOptions {
                        top_k: 10,
                        ..Default::default()
                    };
                    let results = search(black_box(db), &mut embedder, black_box(query), opts)
                        .expect("Search failed");
                    black_box(results)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark search with varying result limits
fn bench_search_limits(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_limits");
    group.sample_size(20);

    let db = setup_db(50, 500);

    for top_k in [5, 10, 20, 50] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("top_{top_k}")),
            &db,
            |b, db| {
                b.iter(|| {
                    let mut embedder = get_embedder().lock().unwrap();
                    let query = "programming language development";
                    let opts = SearchOptions {
                        top_k,
                        ..Default::default()
                    };
                    let results = search(black_box(db), &mut embedder, black_box(query), opts)
                        .expect("Search failed");
                    black_box(results)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark search with varying query lengths
fn bench_search_query_length(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_query_length");
    group.sample_size(20);

    let db = setup_db(30, 500);

    let queries = [
        ("short", "rust"),
        ("medium", "rust programming language"),
        ("long", "rust programming language memory safety ownership borrowing"),
        ("very_long", "rust programming language memory safety ownership borrowing lifetimes type system compile time guarantees zero cost abstractions"),
    ];

    for (name, query) in queries {
        group.bench_with_input(BenchmarkId::from_parameter(name), &db, |b, db| {
            b.iter(|| {
                let mut embedder = get_embedder().lock().unwrap();
                let opts = SearchOptions {
                    top_k: 10,
                    ..Default::default()
                };
                let results = search(black_box(db), &mut embedder, black_box(query), opts)
                    .expect("Search failed");
                black_box(results)
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_search_corpus_size,
    bench_search_limits,
    bench_search_query_length
);
criterion_main!(benches);
