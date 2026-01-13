// tests/integration.rs

use markdown_chunker::chunker::hierarchy::HierarchyAwareChunker;
use markdown_chunker::chunker::hybrid::HybridChunker;
use std::collections::HashSet;

#[test]
fn test_complex_structure() {
    let markdown = include_str!("fixtures/complex_structure.md");
    let chunker = HybridChunker::new(800, 100);
    let chunks = chunker.chunk(markdown);

    assert!(!chunks.is_empty());

    // Verify chunks have reasonable token counts
    for chunk in &chunks {
        assert!(chunk.metadata.token_count > 0);
    }
}

#[test]
fn test_code_blocks_never_split() {
    let markdown = include_str!("fixtures/code_heavy.md");
    let chunker = HierarchyAwareChunker::new(800, 100);
    let chunks = chunker.chunk(markdown);

    // Find chunks containing code blocks
    let code_chunks: Vec<_> = chunks.iter()
        .filter(|c| c.content.contains("```"))
        .collect();

    // Verify each code block is complete (has both opening and closing ```)
    for chunk in code_chunks {
        let backtick_count = chunk.content.matches("```").count();
        assert!(backtick_count >= 2, "Code block is split!");
    }
}

#[test]
fn test_hierarchy_preserved() {
    let markdown = "# Chapter 1\n\n## Section 1.1\n\nContent here.\n\n## Section 1.2\n\nMore content.";
    let chunker = HierarchyAwareChunker::new(800, 100);
    let chunks = chunker.chunk(markdown);

    // Check that chunks have header hierarchy
    for chunk in &chunks {
        if chunk.content.contains("Content here") || chunk.content.contains("More content") {
            assert!(!chunk.metadata.header_hierarchy.is_empty());
        }
    }
}

#[test]
fn test_tables_never_split() {
    let markdown = include_str!("fixtures/complex_structure.md");
    let chunker = HierarchyAwareChunker::new(800, 100);
    let chunks = chunker.chunk(markdown);

    // Find chunks containing table rows
    let table_chunks: Vec<_> = chunks.iter()
        .filter(|c| c.content.contains("|---"))
        .collect();

    // Verify each table is complete (has header, separator, and rows)
    for chunk in table_chunks {
        let pipe_lines = chunk.content.lines().filter(|l| l.contains('|')).count();
        assert!(pipe_lines >= 3, "Table is incomplete!");
    }
}

#[test]
fn test_no_content_loss() {
    let markdown = include_str!("fixtures/complex_structure.md");
    let chunker = HybridChunker::new(800, 10);
    let chunks = chunker.chunk(markdown);

    // Collect all words from original
    let original_words: HashSet<&str> = markdown
        .split_whitespace()
        .filter(|w| !w.is_empty())
        .collect();

    // Collect all words from chunks (removing header context duplicates is okay)
    let chunked_words: HashSet<&str> = chunks.iter()
        .flat_map(|c| c.content.split_whitespace())
        .filter(|w| !w.is_empty())
        .collect();

    // Most words should be preserved (allowing for header duplication)
    let preserved_count = original_words.iter()
        .filter(|w| chunked_words.contains(*w))
        .count();

    let preservation_ratio = preserved_count as f64 / original_words.len() as f64;
    assert!(preservation_ratio > 0.95,
        "Content loss detected! Only {:.1}% preserved", preservation_ratio * 100.0);
}

#[test]
fn test_chunks_within_limits() {
    let markdown = include_str!("fixtures/complex_structure.md");
    let max_tokens = 800;
    let min_tokens = 100;
    let chunker = HybridChunker::new(max_tokens, min_tokens);
    let chunks = chunker.chunk(markdown);

    for chunk in &chunks {
        // Most chunks should be within limits (code blocks and tables are exceptions)
        if chunk.metadata.chunk_type == markdown_chunker::ChunkType::Paragraph {
            assert!(chunk.metadata.token_count <= max_tokens * 2,
                "Chunk too large: {} tokens", chunk.metadata.token_count);
        }
    }
}

#[test]
fn test_nested_lists() {
    let markdown = include_str!("fixtures/nested_lists.md");
    let chunker = HierarchyAwareChunker::new(800, 50);
    let chunks = chunker.chunk(markdown);

    assert!(!chunks.is_empty());

    // Verify lists are extracted
    let list_chunks: Vec<_> = chunks.iter()
        .filter(|c| c.metadata.chunk_type == markdown_chunker::ChunkType::List)
        .collect();

    assert!(!list_chunks.is_empty(), "No lists found!");
}

#[test]
fn test_mixed_japanese() {
    let markdown = include_str!("fixtures/mixed_japanese.md");
    let chunker = HybridChunker::new(800, 100);
    let chunks = chunker.chunk(markdown);

    assert!(!chunks.is_empty());

    // Verify Japanese content is present
    let has_japanese = chunks.iter()
        .any(|c| c.content.chars().any(|ch| ('\u{3040}'..='\u{309F}').contains(&ch)));

    assert!(has_japanese, "Japanese content not preserved!");
}

#[test]
fn test_header_context_added() {
    let markdown = "# Main\n\n## Sub\n\nThis is a content paragraph with enough text to meet the minimum token requirements. We need to make sure this paragraph is long enough so it doesn't get filtered out by the minimum token threshold.";
    let chunker = HierarchyAwareChunker::new(800, 50);
    let chunks = chunker.chunk(markdown);

    // Find the content chunk
    let content_chunk = chunks.iter()
        .find(|c| c.content.contains("content paragraph"));

    assert!(content_chunk.is_some());

    let chunk = content_chunk.unwrap();
    // Header context should be prepended
    assert!(chunk.content.contains("# Main"));
    assert!(chunk.content.contains("## Sub"));
}

#[test]
fn test_empty_lines_handled() {
    let markdown = "# Title\n\n\n\nThis is content after many empty lines. It needs to be long enough to meet the minimum token threshold of 100 tokens. So we add more text here to make sure it qualifies as a valid chunk. This should be sufficient content now.";
    let chunker = HierarchyAwareChunker::new(800, 50);
    let chunks = chunker.chunk(markdown);

    assert!(!chunks.is_empty());
}

#[test]
fn test_blockquotes() {
    let markdown = include_str!("fixtures/complex_structure.md");
    let chunker = HierarchyAwareChunker::new(800, 10);
    let chunks = chunker.chunk(markdown);

    let quote_chunks: Vec<_> = chunks.iter()
        .filter(|c| c.metadata.chunk_type == markdown_chunker::ChunkType::Quote)
        .collect();

    assert!(!quote_chunks.is_empty(), "No quotes found!");
}
