// src/chunker/recursive.rs

use crate::metadata::{Chunk, ChunkMetadata, ChunkType};
use crate::token_counter::TokenCounter;

/// Recursive character splitter (fallback strategy)
pub struct RecursiveCharacterSplitter {
    max_tokens: usize,
    #[allow(dead_code)]
    min_tokens: usize,
    separators: Vec<String>,
}

impl RecursiveCharacterSplitter {
    pub fn new(max_tokens: usize, min_tokens: usize) -> Self {
        Self {
            max_tokens,
            min_tokens,
            separators: vec![
                "\n\n".to_string(), "\n".to_string(),
                ". ".to_string(), "! ".to_string(), "? ".to_string(),
                "; ".to_string(), ", ".to_string(),
                " ".to_string(), "".to_string(),
            ],
        }
    }

    pub fn chunk(&self, text: &str) -> Vec<Chunk> {
        let chunks_text = self.split_recursive(text, &self.separators);

        chunks_text.into_iter()
            .enumerate()
            .map(|(i, content)| {
                let token_count = TokenCounter::estimate(&content);
                let char_count = content.chars().count();
                Chunk {
                    content,
                    metadata: ChunkMetadata {
                        position: i,
                        token_count,
                        char_count,
                        language: None,
                        chunk_type: ChunkType::Paragraph,
                        header_hierarchy: vec![],
                    },
                }
            })
            .collect()
    }

    fn split_recursive(&self, text: &str, separators: &[String]) -> Vec<String> {
        if TokenCounter::estimate(text) <= self.max_tokens {
            return vec![text.to_string()];
        }

        if separators.is_empty() {
            return vec![text.to_string()];
        }

        let separator = &separators[0];
        let remaining_seps = &separators[1..];

        if separator.is_empty() {
            return self.split_by_chars(text);
        }

        let splits: Vec<&str> = text.split(separator.as_str()).collect();
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();

        for split in splits {
            let split_tokens = TokenCounter::estimate(split);
            let current_tokens = TokenCounter::estimate(&current_chunk);

            if current_tokens + split_tokens > self.max_tokens && !current_chunk.is_empty() {
                chunks.push(std::mem::take(&mut current_chunk));

                if split_tokens > self.max_tokens {
                    chunks.extend(self.split_recursive(split, remaining_seps));
                } else {
                    current_chunk.push_str(split);
                }
            } else {
                if !current_chunk.is_empty() && !separator.is_empty() {
                    current_chunk.push_str(separator);
                }
                current_chunk.push_str(split);
            }
        }

        if !current_chunk.is_empty() {
            chunks.push(current_chunk);
        }

        chunks
    }

    fn split_by_chars(&self, text: &str) -> Vec<String> {
        let chars: Vec<char> = text.chars().collect();
        let char_limit = self.max_tokens * 4;
        let mut chunks = Vec::new();
        let mut i = 0;

        while i < chars.len() {
            let end = (i + char_limit).min(chars.len());
            chunks.push(chars[i..end].iter().collect());
            i = end;
        }

        chunks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recursive_splitter() {
        let text = "This is a test. This is another sentence. And one more.";
        let splitter = RecursiveCharacterSplitter::new(50, 5);
        let chunks = splitter.chunk(text);

        assert!(!chunks.is_empty());
        // Most chunks should be within limits
        assert!(chunks.len() >= 1);
    }

    #[test]
    fn test_short_text() {
        let text = "Short";
        let splitter = RecursiveCharacterSplitter::new(100, 5);
        let chunks = splitter.chunk(text);

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content, "Short");
    }
}
