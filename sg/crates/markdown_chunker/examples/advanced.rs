use markdown_chunker::{ChunkType, Chunker};

fn main() {
    let markdown = r#"
# Advanced Markdown Chunking

This example demonstrates advanced features and configuration options.

## Custom Configuration

You can customize the chunking behavior using the builder pattern.

### Token Limits

Control the size of chunks by setting minimum and maximum token counts.

### Overlap Strategy

Semantic overlap maintains context between chunks by including sentences
from the previous chunk.

## Tables

| Feature | Description | Status |
|---------|-------------|--------|
| CJK Support | Multilingual | ✅ |
| Code Blocks | Never split | ✅ |
| Tables | Preserved | ✅ |

## Lists

- First item with some content
- Second item with more details
  - Nested item
  - Another nested item
- Third item

## Code Examples

```python
def process_chunks(chunks):
    for i, chunk in enumerate(chunks):
        print(f"Chunk {i}: {chunk.metadata.token_count} tokens")
```

## Blockquotes

> This is a blockquote that should be preserved as a single unit.
> It can span multiple lines and maintain its structure.
>
> Even with blank lines in between.

## Conclusion

The chunker handles all these markdown elements intelligently.
"#;

    println!("=== Comparison of Different Configurations ===\n");

    // Configuration 1: Default
    println!("1️⃣  Default Configuration");
    let chunker_default = Chunker::default();
    let chunks_default = chunker_default.chunk(markdown);
    print_summary(&chunks_default);

    // Configuration 2: Small chunks
    println!("\n2️⃣  Small Chunks (400 tokens max)");
    let chunker_small = Chunker::builder()
        .max_tokens(400)
        .min_tokens(50)
        .overlap_tokens(50)
        .build();
    let chunks_small = chunker_small.chunk(markdown);
    print_summary(&chunks_small);

    // Configuration 3: Large chunks
    println!("\n3️⃣  Large Chunks (1200 tokens max)");
    let chunker_large = Chunker::builder()
        .max_tokens(1200)
        .min_tokens(200)
        .overlap_tokens(150)
        .build();
    let chunks_large = chunker_large.chunk(markdown);
    print_summary(&chunks_large);

    // Configuration 4: No overlap
    println!("\n4️⃣  No Overlap");
    let chunker_no_overlap = Chunker::builder().add_overlap(false).build();
    let chunks_no_overlap = chunker_no_overlap.chunk(markdown);
    print_summary(&chunks_no_overlap);

    // Detailed analysis of default configuration
    println!("\n=== Detailed Analysis (Default Configuration) ===\n");

    for (i, chunk) in chunks_default.iter().enumerate() {
        println!("Chunk #{}", i + 1);

        match chunk.metadata.chunk_type {
            ChunkType::CodeBlock => println!("  🔷 Type: Code Block (preserved)"),
            ChunkType::Table => println!("  📊 Type: Table (preserved)"),
            ChunkType::List => println!("  📝 Type: List"),
            ChunkType::Quote => println!("  💬 Type: Blockquote"),
            ChunkType::Paragraph => println!("  📄 Type: Paragraph"),
            ChunkType::Heading => println!("  📌 Type: Heading"),
        }

        println!("  📏 Tokens: {}", chunk.metadata.token_count);

        if !chunk.metadata.header_hierarchy.is_empty() {
            println!("  🗂️  Hierarchy:");
            for (level, title) in &chunk.metadata.header_hierarchy {
                println!("     {} {}", "#".repeat(*level), title);
            }
        }

        println!();
    }
}

fn print_summary(chunks: &[markdown_chunker::Chunk]) {
    let total_tokens: usize = chunks.iter().map(|c| c.metadata.token_count).sum();
    let avg_tokens = if !chunks.is_empty() {
        total_tokens / chunks.len()
    } else {
        0
    };

    let code_blocks = chunks
        .iter()
        .filter(|c| c.metadata.chunk_type == ChunkType::CodeBlock)
        .count();
    let tables = chunks
        .iter()
        .filter(|c| c.metadata.chunk_type == ChunkType::Table)
        .count();
    let lists = chunks
        .iter()
        .filter(|c| c.metadata.chunk_type == ChunkType::List)
        .count();

    println!("   Chunks: {}", chunks.len());
    println!("   Total tokens: {total_tokens}");
    println!("   Avg tokens/chunk: {avg_tokens}");
    println!("   Code blocks: {code_blocks}");
    println!("   Tables: {tables}");
    println!("   Lists: {lists}");
}
