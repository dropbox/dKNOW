// src/chunker/hybrid.rs

use crate::chunker::hierarchy::HierarchyAwareChunker;
use crate::chunker::recursive::RecursiveCharacterSplitter;
use crate::metadata::Chunk;

/// Hybrid chunker that selects the best strategy based on content
pub struct HybridChunker {
    max_tokens: usize,
    min_tokens: usize,
}

impl HybridChunker {
    pub fn new(max_tokens: usize, min_tokens: usize) -> Self {
        Self {
            max_tokens,
            min_tokens,
        }
    }

    pub fn chunk(&self, text: &str) -> Vec<Chunk> {
        if self.has_markdown_structure(text) {
            HierarchyAwareChunker::new(self.max_tokens, self.min_tokens).chunk(text)
        } else {
            RecursiveCharacterSplitter::new(self.max_tokens, self.min_tokens).chunk(text)
        }
    }

    #[allow(clippy::unused_self)]
    fn has_markdown_structure(&self, text: &str) -> bool {
        let has_headers = text.lines().any(|line| line.trim_start().starts_with('#'));
        let paragraph_breaks = text.matches("\n\n").count();
        has_headers && paragraph_breaks > 5
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hybrid_with_markdown() {
        let text = "# Header\n\n## Subheader\n\nThis is the first paragraph with enough content to meet minimum token requirements.\n\nThis is the second paragraph with more interesting content.\n\nAnother paragraph.\n\nYet another paragraph with sufficient text.\n\nOne more paragraph.\n\nAnother one here.\n\nFinal paragraph with content.";
        let chunker = HybridChunker::new(100, 10);
        let chunks = chunker.chunk(text);

        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_hybrid_without_markdown() {
        let text = "This is plain text. No headers or structure.";
        let chunker = HybridChunker::new(100, 10);
        let chunks = chunker.chunk(text);

        assert!(!chunks.is_empty());
    }
}
