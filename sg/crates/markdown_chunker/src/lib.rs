// src/lib.rs
//! # Markdown Chunker
//!
//! A production-grade markdown chunker for RAG systems with multilingual support
//! (English, Japanese, Chinese, Korean). Uses a hybrid hierarchy-aware strategy
//! for optimal semantic chunking.
//!
//! ## Features
//!
//! - **Hierarchy-aware chunking**: Preserves markdown structure and header context
//! - **Multilingual support**: CJK (Chinese, Japanese, Korean) detection and handling
//! - **Semantic overlap**: Maintains context continuity between chunks
//! - **Structure preservation**: Never splits code blocks or tables
//! - **Hybrid strategy**: Automatically switches between hierarchy-aware and recursive splitting
//!
//! ## Quick Start
//!
//! ```rust
//! use markdown_chunker::Chunker;
//!
//! let markdown = "# Introduction\n\nThis is a test document.";
//! let chunker = Chunker::default();
//! let chunks = chunker.chunk(markdown);
//!
//! for chunk in chunks {
//!     println!("Chunk: {} tokens", chunk.metadata.token_count);
//! }
//! ```
//!
//! ## Advanced Usage
//!
//! ```rust
//! use markdown_chunker::Chunker;
//!
//! let chunker = Chunker::builder()
//!     .max_tokens(800)
//!     .min_tokens(100)
//!     .overlap_tokens(100)
//!     .build();
//!
//! let chunks = chunker.chunk("# My Document\n\nContent here.");
//! ```

pub mod chunker;
pub mod metadata;
pub mod overlap;
pub mod segmentation;
pub mod token_counter;

pub use metadata::{extract_links, Chunk, ChunkMetadata, ChunkType, Link, LinkType};

use chunker::hybrid::HybridChunker;
use overlap::OverlapStrategy;

/// Main chunker interface for markdown text.
///
/// Provides both simple default configuration and advanced builder pattern.
pub struct Chunker {
    max_tokens: usize,
    min_tokens: usize,
    overlap_tokens: usize,
    add_overlap: bool,
}

impl Default for Chunker {
    /// Create a chunker with default settings.
    ///
    /// Default configuration:
    /// - max_tokens: 800
    /// - min_tokens: 100
    /// - overlap_tokens: 100
    /// - add_overlap: true
    fn default() -> Self {
        Self {
            max_tokens: 800,
            min_tokens: 100,
            overlap_tokens: 100,
            add_overlap: true,
        }
    }
}

impl Chunker {
    /// Create a builder for custom configuration.
    ///
    /// # Example
    ///
    /// ```rust
    /// use markdown_chunker::Chunker;
    ///
    /// let chunker = Chunker::builder()
    ///     .max_tokens(500)
    ///     .min_tokens(50)
    ///     .overlap_tokens(50)
    ///     .build();
    /// ```
    pub fn builder() -> ChunkerBuilder {
        ChunkerBuilder::new()
    }

    /// Chunk the markdown text into semantically meaningful pieces.
    ///
    /// Returns a vector of chunks with metadata including token count,
    /// chunk type, header hierarchy, and position.
    ///
    /// Note: CRLF line endings are automatically normalized to LF before chunking.
    ///
    /// # Example
    ///
    /// ```rust
    /// use markdown_chunker::Chunker;
    ///
    /// let chunker = Chunker::default();
    /// let markdown = "# Introduction\n\nThis is content.\n\n## Section\n\nMore content.";
    /// let chunks = chunker.chunk(markdown);
    ///
    /// for chunk in chunks {
    ///     println!("Position: {}, Tokens: {}",
    ///         chunk.metadata.position,
    ///         chunk.metadata.token_count);
    /// }
    /// ```
    pub fn chunk(&self, text: &str) -> Vec<Chunk> {
        // Normalize CRLF to LF for consistent splitting behavior
        // Windows-style line endings (\r\n) would break the paragraph separators
        let normalized = if text.contains('\r') {
            std::borrow::Cow::Owned(text.replace("\r\n", "\n").replace('\r', "\n"))
        } else {
            std::borrow::Cow::Borrowed(text)
        };

        let chunker = HybridChunker::new(self.max_tokens, self.min_tokens);
        let chunks = chunker.chunk(&normalized);

        if self.add_overlap {
            let overlap = OverlapStrategy::new(self.overlap_tokens);
            overlap.apply(chunks)
        } else {
            chunks
        }
    }
}

/// Builder for configuring a Chunker with custom settings.
pub struct ChunkerBuilder {
    max_tokens: usize,
    min_tokens: usize,
    overlap_tokens: usize,
    add_overlap: bool,
}

impl ChunkerBuilder {
    /// Create a new builder with default settings.
    pub fn new() -> Self {
        Self {
            max_tokens: 800,
            min_tokens: 100,
            overlap_tokens: 100,
            add_overlap: true,
        }
    }

    /// Set the maximum number of tokens per chunk.
    ///
    /// Default: 800
    pub fn max_tokens(mut self, max: usize) -> Self {
        self.max_tokens = max;
        self
    }

    /// Set the minimum number of tokens per chunk.
    ///
    /// Chunks smaller than this will be filtered out, except for
    /// code blocks and tables which are always preserved.
    ///
    /// Default: 100
    pub fn min_tokens(mut self, min: usize) -> Self {
        self.min_tokens = min;
        self
    }

    /// Set the target overlap in tokens between chunks.
    ///
    /// Overlap is added at sentence boundaries for semantic continuity.
    ///
    /// Default: 100
    pub fn overlap_tokens(mut self, overlap: usize) -> Self {
        self.overlap_tokens = overlap;
        self
    }

    /// Set whether to add overlap between chunks.
    ///
    /// Default: true
    pub fn add_overlap(mut self, add: bool) -> Self {
        self.add_overlap = add;
        self
    }

    /// Build the Chunker with the configured settings.
    pub fn build(self) -> Chunker {
        Chunker {
            max_tokens: self.max_tokens,
            min_tokens: self.min_tokens,
            overlap_tokens: self.overlap_tokens,
            add_overlap: self.add_overlap,
        }
    }
}

impl Default for ChunkerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crlf_normalization() {
        // CRLF content should produce the same chunks as LF content
        let crlf_content = "First paragraph with some text.\r\n\r\nSecond paragraph with more text.\r\n\r\nThird paragraph here.";
        let lf_content = "First paragraph with some text.\n\nSecond paragraph with more text.\n\nThird paragraph here.";

        let chunker = Chunker::builder()
            .max_tokens(10) // Force splitting
            .min_tokens(1)
            .overlap_tokens(0)
            .add_overlap(false)
            .build();

        let crlf_chunks = chunker.chunk(crlf_content);
        let lf_chunks = chunker.chunk(lf_content);

        // Both should produce the same number of chunks
        assert_eq!(
            crlf_chunks.len(),
            lf_chunks.len(),
            "CRLF and LF content should produce the same number of chunks"
        );

        // Content should be equivalent (after normalization)
        for (crlf, lf) in crlf_chunks.iter().zip(lf_chunks.iter()) {
            assert_eq!(
                crlf.content.replace('\r', ""),
                lf.content,
                "Chunk content should match after normalization"
            );
        }
    }

    #[test]
    fn test_mixed_line_endings() {
        // Content with mixed line endings
        let mixed_content = "Line one\r\nLine two\nLine three\rLine four";

        let chunker = Chunker::default();
        let chunks = chunker.chunk(mixed_content);

        // Should not panic and should produce chunks
        assert!(!chunks.is_empty());

        // All \r characters should be normalized to \n in the chunks
        for chunk in &chunks {
            assert!(
                !chunk.content.contains('\r'),
                "Chunks should not contain \\r characters"
            );
        }
    }
}
