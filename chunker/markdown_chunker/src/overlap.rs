// src/overlap.rs

use crate::metadata::Chunk;
use crate::segmentation::unicode::SentenceSegmenter;
use crate::token_counter::TokenCounter;

/// Strategy for adding semantic overlap between chunks.
/// Overlaps at sentence boundaries to maintain context continuity.
pub struct OverlapStrategy {
    overlap_tokens: usize,
}

impl OverlapStrategy {
    pub fn new(overlap_tokens: usize) -> Self {
        Self { overlap_tokens }
    }

    /// Apply overlap to a list of chunks by prepending suffixes from previous chunks.
    pub fn apply(&self, chunks: Vec<Chunk>) -> Vec<Chunk> {
        if chunks.len() <= 1 {
            return chunks;
        }

        let mut overlapped = Vec::new();

        for i in 0..chunks.len() {
            let mut content = chunks[i].content.clone();

            // Add suffix from previous chunk
            if i > 0 {
                let prev_suffix = self.get_sentence_suffix(&chunks[i - 1].content, self.overlap_tokens);
                if !prev_suffix.is_empty() {
                    content = format!("{}\n\n{}", prev_suffix, content);
                }
            }

            // Calculate metadata before moving content
            let token_count = TokenCounter::estimate(&content);
            let char_count = content.chars().count();

            let mut chunk = chunks[i].clone();
            chunk.content = content;
            chunk.metadata.token_count = token_count;
            chunk.metadata.char_count = char_count;
            overlapped.push(chunk);
        }

        overlapped
    }

    /// Extract the last N sentences from text up to target_tokens.
    fn get_sentence_suffix(&self, text: &str, target_tokens: usize) -> String {
        let sentences = SentenceSegmenter::split_universal(text);
        let mut suffix = Vec::new();
        let mut token_count = 0;

        // Iterate from the end, collecting sentences until we hit the target
        for sentence in sentences.iter().rev() {
            let sentence_tokens = TokenCounter::estimate(sentence);

            // Stop if adding this sentence would exceed target (unless we have no sentences yet)
            if token_count + sentence_tokens > target_tokens && !suffix.is_empty() {
                break;
            }

            suffix.push(sentence);
            token_count += sentence_tokens;
        }

        // Reverse and join (collected in reverse order)
        suffix.iter().rev().map(|s| s.as_str()).collect::<Vec<_>>().join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::{ChunkMetadata, ChunkType};

    #[test]
    fn test_overlap_single_chunk() {
        let chunks = vec![Chunk {
            content: "Single chunk.".to_string(),
            metadata: ChunkMetadata {
                position: 0,
                token_count: 10,
                char_count: 13,
                language: None,
                chunk_type: ChunkType::Paragraph,
                header_hierarchy: vec![],
            },
        }];

        let strategy = OverlapStrategy::new(50);
        let overlapped = strategy.apply(chunks.clone());

        assert_eq!(overlapped.len(), 1);
        assert_eq!(overlapped[0].content, "Single chunk.");
    }

    #[test]
    fn test_overlap_multiple_chunks() {
        let chunks = vec![
            Chunk {
                content: "First chunk. This is sentence one. This is sentence two.".to_string(),
                metadata: ChunkMetadata {
                    position: 0,
                    token_count: 50,
                    char_count: 57,
                    language: None,
                    chunk_type: ChunkType::Paragraph,
                    header_hierarchy: vec![],
                },
            },
            Chunk {
                content: "Second chunk.".to_string(),
                metadata: ChunkMetadata {
                    position: 1,
                    token_count: 10,
                    char_count: 13,
                    language: None,
                    chunk_type: ChunkType::Paragraph,
                    header_hierarchy: vec![],
                },
            },
        ];

        let strategy = OverlapStrategy::new(50);
        let overlapped = strategy.apply(chunks);

        assert_eq!(overlapped.len(), 2);
        // First chunk unchanged
        assert_eq!(overlapped[0].content, "First chunk. This is sentence one. This is sentence two.");
        // Second chunk should have overlap from first
        assert!(overlapped[1].content.contains("sentence"));
        assert!(overlapped[1].content.contains("Second chunk"));
    }

    #[test]
    fn test_sentence_suffix_extraction() {
        let strategy = OverlapStrategy::new(20);
        let text = "First sentence. Second sentence. Third sentence. Fourth sentence.";
        let suffix = strategy.get_sentence_suffix(text, 20);

        // Should get some sentences from the end
        assert!(!suffix.is_empty());
        assert!(suffix.contains("sentence"));
    }

    #[test]
    fn test_empty_chunks() {
        let chunks = vec![];
        let strategy = OverlapStrategy::new(50);
        let overlapped = strategy.apply(chunks);

        assert_eq!(overlapped.len(), 0);
    }
}
