# Markdown Chunker

A production-grade markdown chunker for RAG (Retrieval-Augmented Generation) systems with multilingual support. Built in Rust for optimal performance.

## Features

- **🌐 Multilingual Support**: Native support for English, Japanese, Chinese, and Korean (CJK)
- **🧠 Hierarchy-Aware**: Preserves markdown structure and includes header context
- **⚡ High Performance**: Processes 10K words in ~1ms
- **🎯 Semantic Overlap**: Maintains context continuity between chunks at sentence boundaries
- **🔒 Structure Preservation**: Never splits code blocks or tables
- **🔄 Hybrid Strategy**: Automatically switches between hierarchy-aware and recursive splitting

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
markdown_chunker = "0.1.0"
```

## Quick Start

```rust
use markdown_chunker::Chunker;

fn main() {
    let markdown = r#"
# Introduction

This is a test document with multiple sections.

## Background

Here's some background information that spans
multiple lines and contains important context.

```rust
fn example() {
    println!("Code blocks are never split!");
}
```

## Conclusion

Final thoughts and summary.
"#;

    let chunker = Chunker::default();
    let chunks = chunker.chunk(markdown);

    for (i, chunk) in chunks.iter().enumerate() {
        println!("Chunk {}: {} tokens", i, chunk.metadata.token_count);
        println!("Type: {:?}", chunk.metadata.chunk_type);
        println!("Headers: {:?}", chunk.metadata.header_hierarchy);
        println!("---");
    }
}
```

## Advanced Usage

### Custom Configuration

```rust
use markdown_chunker::Chunker;

let chunker = Chunker::builder()
    .max_tokens(800)        // Maximum tokens per chunk
    .min_tokens(100)        // Minimum tokens per chunk
    .overlap_tokens(100)    // Target overlap between chunks
    .add_overlap(true)      // Enable/disable overlap
    .build();

let chunks = chunker.chunk("# My Document\n\nContent here.");
```

### Working with Metadata

```rust
use markdown_chunker::{Chunker, ChunkType};

let chunker = Chunker::default();
let chunks = chunker.chunk(markdown_text);

for chunk in chunks {
    // Access chunk metadata
    println!("Position: {}", chunk.metadata.position);
    println!("Tokens: {}", chunk.metadata.token_count);
    println!("Characters: {}", chunk.metadata.char_count);

    // Filter by chunk type
    match chunk.metadata.chunk_type {
        ChunkType::CodeBlock => println!("This is a code block"),
        ChunkType::Table => println!("This is a table"),
        ChunkType::List => println!("This is a list"),
        ChunkType::Quote => println!("This is a blockquote"),
        ChunkType::Paragraph => println!("This is a paragraph"),
        ChunkType::Heading => println!("This is a heading"),
    }

    // Access header hierarchy
    for (level, title) in &chunk.metadata.header_hierarchy {
        println!("Header level {}: {}", level, title);
    }
}
```

### Multilingual Support

The chunker automatically detects and handles CJK (Chinese, Japanese, Korean) text:

```rust
use markdown_chunker::Chunker;

let japanese = "# はじめに\n\nこれは日本語のテキストです。";
let chinese = "# 介绍\n\n这是中文文本。";
let korean = "# 소개\n\n이것은 한국어 텍스트입니다。";

let chunker = Chunker::default();

let ja_chunks = chunker.chunk(japanese);
let zh_chunks = chunker.chunk(chinese);
let ko_chunks = chunker.chunk(korean);
```

## How It Works

### Chunking Strategy

The chunker uses a **hybrid strategy** that automatically selects the best approach:

1. **Hierarchy-Aware Chunking** (Primary)
   - Used when markdown has clear structure (headers + paragraph breaks)
   - Maintains header context in each chunk
   - Preserves document hierarchy
   - Never splits code blocks or tables

2. **Recursive Character Splitting** (Fallback)
   - Used for unstructured or minimally-structured text
   - Splits on multiple separators (paragraphs → sentences → words → characters)
   - Ensures chunks stay within token limits

### Key Algorithms

**CJK Detection**: Automatically identifies Chinese, Japanese, and Korean text for proper tokenization.

**Sentence Segmentation**: Uses Unicode sentence boundaries with language-specific enhancements for Japanese (。！？) and Chinese (。！？；).

**Token Estimation**:
- English: ~4 characters per token
- CJK: ~2 characters per token
- Mixed content: Weighted by CJK ratio

**Semantic Overlap**: Adds the last N sentences from the previous chunk to maintain context continuity.

## Performance

Benchmarks on Apple Silicon (M-series):

| Document Size | Time      | Throughput    |
|---------------|-----------|---------------|
| 1K words      | ~95 µs    | ~10.5M words/s|
| 10K words     | ~1 ms     | ~10M words/s  |
| 100K words    | ~10 ms    | ~10M words/s  |

**Target**: < 100ms for 10K words ✅ (achieved in ~1ms, **100x faster**)

## Architecture

```
src/
├── lib.rs              # Public API (Chunker, ChunkerBuilder)
├── metadata.rs         # Data structures (Chunk, ChunkMetadata, ChunkType)
├── token_counter.rs    # Token estimation
├── segmentation/
│   ├── cjk.rs         # CJK detection and language identification
│   └── unicode.rs     # Sentence segmentation
├── chunker/
│   ├── hierarchy.rs   # Hierarchy-aware chunking (primary strategy)
│   ├── recursive.rs   # Recursive character splitting (fallback)
│   └── hybrid.rs      # Strategy selector (orchestrator)
└── overlap.rs         # Semantic overlap at sentence boundaries
```

## Design Principles

### Critical Rules

✅ **MUST DO**:
1. Never split code blocks (preserve ` ``` ` boundaries)
2. Never split tables (keep entire table in one chunk)
3. Add header context (prepend parent headers to chunks)
4. Preserve hierarchy (store header path in metadata)
5. Complete sentences (overlap at sentence boundaries only)
6. Language agnostic (use Unicode segmentation by default)

❌ **MUST NOT DO**:
1. Don't split on spaces for CJK (no spaces between words)
2. Don't use hard token cutoffs (split at sentence boundaries)
3. Don't lose context (always include header hierarchy)
4. Don't ignore structure (parse markdown elements)
5. Don't assume language (detect or use universal algorithms)

## Use Cases

- **RAG Systems**: Chunk documents for embedding and retrieval
- **Documentation Processing**: Process technical docs with code examples
- **Content Analysis**: Break down large documents for analysis
- **Search Indexing**: Create searchable chunks of markdown content
- **Translation Pipelines**: Process multilingual documents

## Examples

See the `examples/` directory for complete examples:

- `basic.rs` - Simple chunking example
- `advanced.rs` - Custom configuration and metadata usage
- `multilingual.rs` - Working with CJK text
- `benchmark.rs` - Performance testing

Run an example:

```bash
cargo run --example basic
```

## Testing

Run all tests:

```bash
cargo test
```

Run benchmarks:

```bash
cargo bench
```

## Contributing

Contributions are welcome! Please ensure:

1. All tests pass (`cargo test`)
2. Code is formatted (`cargo fmt`)
3. No clippy warnings (`cargo clippy`)
4. Benchmarks maintain performance targets

## License

MIT OR Apache-2.0

## Acknowledgments

Built with:
- [unicode-segmentation](https://crates.io/crates/unicode-segmentation) - Unicode text segmentation
- [serde](https://crates.io/crates/serde) - Serialization framework
- [criterion](https://crates.io/crates/criterion) - Benchmarking framework
