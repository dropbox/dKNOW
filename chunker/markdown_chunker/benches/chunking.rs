use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use markdown_chunker::Chunker;

// Helper function to generate markdown text of various sizes
fn generate_markdown(word_count: usize) -> String {
    let mut content = String::new();
    content.push_str("# Main Title\n\n");

    let mut words_written = 2; // "Main Title"
    let mut section = 1;

    while words_written < word_count {
        content.push_str(&format!("\n## Section {}\n\n", section));
        words_written += 2;

        // Add a paragraph
        let paragraph_size = (word_count - words_written).min(100);
        for i in 0..paragraph_size {
            content.push_str("word ");
            words_written += 1;

            if i % 20 == 19 {
                content.push_str("sentence. ");
            }
        }

        content.push_str("\n\n");

        // Add a code block every few sections
        if section % 3 == 0 && words_written < word_count - 50 {
            content.push_str("```rust\n");
            content.push_str("fn example() {\n");
            content.push_str("    println!(\"Hello, world!\");\n");
            content.push_str("}\n");
            content.push_str("```\n\n");
            words_written += 10;
        }

        // Add a list every few sections
        if section % 4 == 0 && words_written < word_count - 30 {
            for i in 1..=5 {
                content.push_str(&format!("- List item {} with some content\n", i));
                words_written += 5;
            }
            content.push_str("\n");
        }

        section += 1;

        if words_written >= word_count {
            break;
        }
    }

    content
}

// Benchmark default chunker with varying document sizes
fn benchmark_varying_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("document_size");

    for size in [1_000, 5_000, 10_000, 50_000, 100_000].iter() {
        let markdown = generate_markdown(*size);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_words", size)),
            &markdown,
            |b, md| {
                let chunker = Chunker::default();
                b.iter(|| {
                    let chunks = chunker.chunk(black_box(md));
                    black_box(chunks);
                });
            },
        );
    }

    group.finish();
}

// Benchmark different configurations
fn benchmark_configurations(c: &mut Criterion) {
    let markdown = generate_markdown(10_000);
    let mut group = c.benchmark_group("configurations");

    // Default configuration
    group.bench_function("default", |b| {
        let chunker = Chunker::default();
        b.iter(|| {
            let chunks = chunker.chunk(black_box(&markdown));
            black_box(chunks);
        });
    });

    // Small chunks
    group.bench_function("small_chunks_400", |b| {
        let chunker = Chunker::builder()
            .max_tokens(400)
            .min_tokens(50)
            .overlap_tokens(50)
            .build();
        b.iter(|| {
            let chunks = chunker.chunk(black_box(&markdown));
            black_box(chunks);
        });
    });

    // Large chunks
    group.bench_function("large_chunks_1200", |b| {
        let chunker = Chunker::builder()
            .max_tokens(1200)
            .min_tokens(200)
            .overlap_tokens(150)
            .build();
        b.iter(|| {
            let chunks = chunker.chunk(black_box(&markdown));
            black_box(chunks);
        });
    });

    // No overlap
    group.bench_function("no_overlap", |b| {
        let chunker = Chunker::builder()
            .add_overlap(false)
            .build();
        b.iter(|| {
            let chunks = chunker.chunk(black_box(&markdown));
            black_box(chunks);
        });
    });

    group.finish();
}

// Benchmark code-heavy documents
fn benchmark_code_heavy(c: &mut Criterion) {
    let mut markdown = String::new();
    markdown.push_str("# API Documentation\n\n");

    for i in 0..50 {
        markdown.push_str(&format!("## Function {}\n\n", i));
        markdown.push_str("This function does something important.\n\n");
        markdown.push_str("```rust\n");
        markdown.push_str(&format!("pub fn function_{}(param: i32) -> Result<(), Error> {{\n", i));
        markdown.push_str("    // Implementation\n");
        markdown.push_str("    let result = complex_operation(param);\n");
        markdown.push_str("    validate_result(&result)?;\n");
        markdown.push_str("    Ok(())\n");
        markdown.push_str("}\n");
        markdown.push_str("```\n\n");
    }

    c.bench_function("code_heavy_document", |b| {
        let chunker = Chunker::default();
        b.iter(|| {
            let chunks = chunker.chunk(black_box(&markdown));
            black_box(chunks);
        });
    });
}

// Benchmark multilingual content
fn benchmark_multilingual(c: &mut Criterion) {
    let mut markdown = String::new();

    // Japanese content
    markdown.push_str("# はじめに\n\n");
    markdown.push_str("これは日本語のテキストです。");
    markdown.push_str("複数の文章が含まれています。");
    markdown.push_str("RAGシステムのためのチャンキングをテストします。\n\n");

    markdown.push_str("## 詳細\n\n");
    for _ in 0..100 {
        markdown.push_str("これはサンプルテキストです。");
    }
    markdown.push_str("\n\n");

    // Chinese content
    markdown.push_str("# 介绍\n\n");
    markdown.push_str("这是中文文本。");
    markdown.push_str("包含多个句子。");
    markdown.push_str("测试RAG系统的分块功能。\n\n");

    markdown.push_str("## 详情\n\n");
    for _ in 0..100 {
        markdown.push_str("这是示例文本。");
    }

    c.bench_function("multilingual_cjk", |b| {
        let chunker = Chunker::default();
        b.iter(|| {
            let chunks = chunker.chunk(black_box(&markdown));
            black_box(chunks);
        });
    });
}

criterion_group!(
    benches,
    benchmark_varying_sizes,
    benchmark_configurations,
    benchmark_code_heavy,
    benchmark_multilingual
);
criterion_main!(benches);
