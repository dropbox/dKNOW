//! Text chunking for long document embedding
//!
//! Documents longer than the model's context window (512 tokens) must be split
//! into chunks that can each be embedded independently. This module provides
//! the chunking strategy used for indexing.
//!
//! Uses the `markdown_chunker` crate for hierarchy-aware chunking that:
//! - Preserves markdown structure (headers, code blocks, tables)
//! - Never splits code blocks or tables mid-content
//! - Maintains header context for search result display
//! - Supports CJK (Chinese, Japanese, Korean) text

use markdown_chunker::Chunker as MdChunker;
use xxhash_rust::xxh3::xxh3_64;

/// Target chunk size in tokens (512 tokens is the XTR model limit)
pub const CHUNK_TARGET_TOKENS: usize = 512;

/// Minimum chunk size in tokens (filter tiny chunks, except code/tables)
pub const CHUNK_MIN_TOKENS: usize = 50;

/// Overlap between chunks in tokens (provides context continuity)
pub const CHUNK_OVERLAP_TOKENS: usize = 100;

/// A link extracted from chunk content
#[derive(Debug, Clone)]
pub struct ChunkLink {
    /// The display text of the link
    pub text: String,
    /// The link target (URL, path, or wiki-style reference)
    pub target: String,
    /// Whether this is an internal link (relative path) vs external (URL)
    pub is_internal: bool,
}

/// A chunk of a document with location information
#[derive(Debug, Clone)]
pub struct Chunk {
    /// Zero-based index of this chunk within the document
    pub index: usize,
    /// Starting line number (0-indexed)
    pub start_line: usize,
    /// Ending line number (0-indexed, inclusive)
    pub end_line: usize,
    /// The chunk content
    pub content: String,
    /// Header context for search result display (e.g., "# Title > ## Section")
    pub header_context: String,
    /// Programming language for code blocks (e.g., "rust", "python")
    pub language: Option<String>,
    /// Content hash for differential updates (xxHash64)
    pub content_hash: String,
    /// Links found in this chunk (markdown links, wiki-style links, etc.)
    pub links: Vec<ChunkLink>,
}

/// Compute a fast hash of chunk content for differential updates
///
/// Uses xxHash64 for speed - we don't need cryptographic strength,
/// just collision resistance for content change detection.
pub fn compute_chunk_hash(content: &str) -> String {
    format!("{:016x}", xxh3_64(content.as_bytes()))
}

/// Split document into overlapping chunks suitable for embedding
///
/// Uses markdown_chunker for hierarchy-aware chunking that:
/// - Preserves markdown structure
/// - Never splits code blocks or tables
/// - Maintains header context
/// - Supports multilingual text (CJK)
///
/// Each chunk includes a content_hash for differential updates.
pub fn chunk_document(content: &str) -> Vec<Chunk> {
    if content.is_empty() {
        return vec![Chunk {
            index: 0,
            start_line: 0,
            end_line: 0,
            content: String::new(),
            header_context: String::new(),
            language: None,
            content_hash: compute_chunk_hash(""),
            links: vec![],
        }];
    }

    let chunker = MdChunker::builder()
        .max_tokens(CHUNK_TARGET_TOKENS)
        .min_tokens(CHUNK_MIN_TOKENS)
        .overlap_tokens(CHUNK_OVERLAP_TOKENS)
        .build();

    let md_chunks = chunker.chunk(content);

    if md_chunks.is_empty() {
        let content_hash = compute_chunk_hash(content);
        // Extract links even from non-chunked content
        let links = markdown_chunker::extract_links(content)
            .into_iter()
            .map(|link| ChunkLink {
                text: link.text,
                target: link.target.clone(),
                is_internal: is_internal_link(&link.target),
            })
            .collect();
        return vec![Chunk {
            index: 0,
            start_line: 0,
            end_line: content.lines().count().saturating_sub(1),
            content: content.to_string(),
            header_context: String::new(),
            language: None,
            content_hash,
            links,
        }];
    }

    // Build line offset map for position-to-line conversion
    let line_offsets = build_line_offsets(content);

    let mut search_start = 0;

    md_chunks
        .into_iter()
        .enumerate()
        .map(|(idx, md_chunk)| {
            let header_context = format_header_hierarchy(&md_chunk.metadata.header_hierarchy);
            let (start_line, end_line, next_search_start) =
                find_chunk_lines(content, &md_chunk.content, &line_offsets, search_start);
            search_start = next_search_start;
            let content_hash = compute_chunk_hash(&md_chunk.content);
            let language = md_chunk.metadata.language;
            let links = md_chunk
                .metadata
                .links
                .into_iter()
                .map(|link| ChunkLink {
                    text: link.text,
                    target: link.target.clone(),
                    is_internal: is_internal_link(&link.target),
                })
                .collect();

            Chunk {
                index: idx,
                start_line,
                end_line,
                content: md_chunk.content,
                header_context,
                language,
                content_hash,
                links,
            }
        })
        .collect()
}

/// Determine if a link target is internal (relative path) vs external (URL)
fn is_internal_link(target: &str) -> bool {
    // External links start with a protocol
    if target.starts_with("http://")
        || target.starts_with("https://")
        || target.starts_with("mailto:")
        || target.starts_with("ftp://")
        || target.starts_with("//")
    {
        return false;
    }

    // Internal links are relative paths or wiki-style references
    // Examples: ./README.md, ../docs/api.md, /absolute/path.md, [[wiki-style]]
    true
}

/// Format header hierarchy as a readable path
fn format_header_hierarchy(hierarchy: &[(usize, String)]) -> String {
    if hierarchy.is_empty() {
        return String::new();
    }

    hierarchy
        .iter()
        .map(|(level, text)| {
            let prefix = "#".repeat(*level);
            format!("{prefix} {text}")
        })
        .collect::<Vec<_>>()
        .join(" > ")
}

/// Build a map of byte offsets to line numbers
fn build_line_offsets(content: &str) -> Vec<usize> {
    let mut offsets = vec![0];
    for (i, c) in content.char_indices() {
        if c == '\n' {
            offsets.push(i + 1);
        }
    }
    offsets
}

/// Find the line range for a chunk within the document.
///
/// Uses a forward search starting at `search_start` to reduce ambiguity when
/// chunk content repeats in the document.
fn find_chunk_lines(
    document: &str,
    chunk_content: &str,
    line_offsets: &[usize],
    search_start: usize,
) -> (usize, usize, usize) {
    // Find the chunk content within the document, starting at search_start.
    if let Some(start_offset) = document
        .get(search_start..)
        .and_then(|s| s.find(chunk_content).map(|pos| pos + search_start))
    {
        let end_offset = start_offset + chunk_content.len();

        // Binary search for start line
        let start_line = match line_offsets.binary_search(&start_offset) {
            Ok(line) => line,
            Err(line) => line.saturating_sub(1),
        };

        // Binary search for end line
        let end_line = match line_offsets.binary_search(&end_offset) {
            Ok(line) => line.saturating_sub(1),
            Err(line) => line.saturating_sub(1),
        };

        (
            start_line,
            end_line.max(start_line),
            start_offset.saturating_add(1),
        )
    } else {
        // Fallback: can't find exact match (possibly due to overlap modifications)
        // Use chunk line count as approximation
        let chunk_lines = chunk_content.lines().count();
        (0, chunk_lines.saturating_sub(1), search_start)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_small_document() {
        let content = "Small document\nwith few lines.";
        let chunks = chunk_document(content);
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].content.contains("Small document"));
        assert_eq!(chunks[0].index, 0);
    }

    #[test]
    fn test_chunk_empty_document() {
        let content = "";
        let chunks = chunk_document(content);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content, "");
    }

    #[test]
    fn test_chunk_large_document() {
        // Create document with ~2000 tokens worth of content
        // Each line has 10 words to simulate realistic prose
        let content: String = (0..200)
            .map(|i| format!("This is line number {i} with several words to make it longer and more realistic for testing the chunking behavior."))
            .collect::<Vec<_>>()
            .join("\n\n");
        let chunks = chunk_document(&content);

        // Should create multiple chunks (2000+ tokens / 512 max = at least 3-4 chunks)
        assert!(
            chunks.len() >= 2,
            "Should create multiple chunks, got {}",
            chunks.len()
        );

        // Verify chunk indices are sequential
        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.index, i, "Chunk index should be sequential");
        }
    }

    #[test]
    fn test_chunk_with_markdown_headers() {
        let content = r"# Main Title

Introduction paragraph with some content.

## Section One

Content for section one with more details.

## Section Two

Content for section two.

### Subsection

Detailed content here.
";
        let chunks = chunk_document(content);
        assert!(!chunks.is_empty());

        // The first chunk should have header context
        // (exact format depends on where chunk boundaries fall)
    }

    #[test]
    fn test_chunk_preserves_code_blocks() {
        let content = r#"# Code Example

Here's some code:

```rust
fn main() {
    println!("Hello, world!");
    let x = 42;
    let y = x + 1;
}
```

More text after the code.
"#;
        let chunks = chunk_document(content);

        // Code block should not be split
        let code_chunks: Vec<_> = chunks
            .iter()
            .filter(|c| c.content.contains("fn main()"))
            .collect();

        assert!(!code_chunks.is_empty(), "Should find code block");

        // The code block should be intact
        for chunk in code_chunks {
            if chunk.content.contains("fn main()") {
                assert!(
                    chunk.content.contains("println!"),
                    "Code block should be kept together"
                );
            }
        }
    }

    #[test]
    fn test_header_context_formatting() {
        let hierarchy = vec![
            (1, "Main Title".to_string()),
            (2, "Section".to_string()),
            (3, "Subsection".to_string()),
        ];

        let formatted = format_header_hierarchy(&hierarchy);
        assert_eq!(formatted, "# Main Title > ## Section > ### Subsection");
    }

    #[test]
    fn test_empty_header_context() {
        let hierarchy: Vec<(usize, String)> = vec![];
        let formatted = format_header_hierarchy(&hierarchy);
        assert_eq!(formatted, "");
    }

    #[test]
    fn test_chunk_preserves_all_content() {
        // Create a document where we can verify no content is lost
        let content: String = (0..100)
            .map(|i| format!("unique_word_{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let chunks = chunk_document(&content);

        // Every unique word should appear in at least one chunk
        for i in 0..100 {
            let word = format!("unique_word_{i}");
            let found = chunks.iter().any(|c| c.content.contains(&word));
            assert!(found, "Word {word} should be in at least one chunk");
        }
    }

    #[test]
    fn test_line_offset_building() {
        let content = "line one\nline two\nline three";
        let offsets = build_line_offsets(content);
        assert_eq!(offsets.len(), 3);
        assert_eq!(offsets[0], 0);
        assert_eq!(offsets[1], 9); // After first newline
        assert_eq!(offsets[2], 18); // After second newline
    }

    #[test]
    fn test_compute_chunk_hash() {
        // Same content should produce same hash
        let hash1 = compute_chunk_hash("hello world");
        let hash2 = compute_chunk_hash("hello world");
        assert_eq!(hash1, hash2);

        // Different content should produce different hash
        let hash3 = compute_chunk_hash("hello world!");
        assert_ne!(hash1, hash3);

        // Hash should be 16 hex characters (64 bits)
        assert_eq!(hash1.len(), 16);
        assert!(hash1.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_chunk_has_content_hash() {
        let content = "Small document\nwith few lines.";
        let chunks = chunk_document(content);

        // Every chunk should have a non-empty content hash
        assert!(!chunks.is_empty());
        for chunk in &chunks {
            assert!(!chunk.content_hash.is_empty());
            assert_eq!(chunk.content_hash.len(), 16);
        }

        // Hash should be deterministic
        let chunks2 = chunk_document(content);
        for (c1, c2) in chunks.iter().zip(chunks2.iter()) {
            assert_eq!(c1.content_hash, c2.content_hash);
        }
    }

    #[test]
    fn test_code_block_language_extraction() {
        let content = r#"# Code Example

Here's some Rust code:

```rust
fn main() {
    println!("Hello");
}
```

And some Python:

```python
print("Hello")
```

Plain code block:

```
no language
```
"#;
        let chunks = chunk_document(content);

        // Find chunks with code blocks
        let rust_chunk = chunks.iter().find(|c| c.content.contains("fn main()"));
        let python_chunk = chunks
            .iter()
            .find(|c| c.content.contains("print(\"Hello\")"));
        let plain_chunk = chunks.iter().find(|c| c.content.contains("no language"));

        assert!(rust_chunk.is_some(), "Should find Rust code block");
        assert_eq!(rust_chunk.unwrap().language, Some("rust".to_string()));

        assert!(python_chunk.is_some(), "Should find Python code block");
        assert_eq!(python_chunk.unwrap().language, Some("python".to_string()));

        assert!(plain_chunk.is_some(), "Should find plain code block");
        assert_eq!(plain_chunk.unwrap().language, None);
    }

    #[test]
    fn test_link_extraction() {
        let content = r"# Documentation

Check out [our guide](./guide.md) for setup instructions.
Also see the [API reference](https://api.example.com/docs).

For more info, visit [[Getting Started]] or read the [intro][intro-ref].

<https://example.com/auto>
";
        let chunks = chunk_document(content);

        // Should extract links
        let all_links: Vec<_> = chunks.iter().flat_map(|c| &c.links).collect();
        assert!(!all_links.is_empty(), "Should extract links from content");

        // Check for internal link (relative path)
        let internal = all_links.iter().find(|l| l.target == "./guide.md");
        assert!(internal.is_some(), "Should find internal link");
        assert!(
            internal.unwrap().is_internal,
            "Relative path should be internal"
        );

        // Check for external link (URL)
        let external = all_links
            .iter()
            .find(|l| l.target == "https://api.example.com/docs");
        assert!(external.is_some(), "Should find external link");
        assert!(!external.unwrap().is_internal, "URL should be external");

        // Check for wiki-style link
        let wiki = all_links.iter().find(|l| l.target == "Getting Started");
        assert!(wiki.is_some(), "Should find wiki-style link");
        assert!(wiki.unwrap().is_internal, "Wiki link should be internal");
    }

    #[test]
    fn test_is_internal_link() {
        // External links
        assert!(!is_internal_link("https://example.com"));
        assert!(!is_internal_link("http://example.com"));
        assert!(!is_internal_link("mailto:test@example.com"));
        assert!(!is_internal_link("ftp://files.example.com"));
        assert!(!is_internal_link("//cdn.example.com/file.js"));

        // Internal links
        assert!(is_internal_link("./README.md"));
        assert!(is_internal_link("../docs/api.md"));
        assert!(is_internal_link("/absolute/path.md"));
        assert!(is_internal_link("relative/path.md"));
        assert!(is_internal_link("Getting Started")); // wiki-style
    }
}
