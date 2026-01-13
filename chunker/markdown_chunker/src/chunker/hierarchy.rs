// src/chunker/hierarchy.rs

use crate::metadata::{Chunk, ChunkMetadata, ChunkType};
use crate::token_counter::TokenCounter;

/// Markdown parser that extracts structural elements
pub struct MarkdownParser<'a> {
    lines: Vec<&'a str>,
    pub position: usize,
}

impl<'a> MarkdownParser<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            lines: text.lines().collect(),
            position: 0,
        }
    }

    /// Parse header: returns (level, title)
    pub fn parse_header(line: &str) -> Option<(usize, String)> {
        let trimmed = line.trim_start();
        let hash_count = trimmed.chars().take_while(|&c| c == '#').count();

        if hash_count > 0 && hash_count <= 6 && trimmed.len() > hash_count {
            let rest = &trimmed[hash_count..];
            if rest.starts_with(' ') {
                let title = rest.trim().to_string();
                return Some((hash_count, title));
            }
        }
        None
    }

    pub fn is_code_block_start(line: &str) -> bool {
        line.trim_start().starts_with("```")
    }

    pub fn is_list_item(line: &str) -> bool {
        let trimmed = line.trim_start();
        trimmed.starts_with("- ") ||
        trimmed.starts_with("* ") ||
        trimmed.starts_with("+ ") ||
        Self::is_ordered_list(trimmed)
    }

    fn is_ordered_list(line: &str) -> bool {
        let mut chars = line.chars().peekable();

        // Must start with digit
        if !chars.peek().map_or(false, |c| c.is_numeric()) {
            return false;
        }

        // Skip all leading digits
        while chars.peek().map_or(false, |c| c.is_numeric()) {
            chars.next();
        }

        // Next must be '.'
        if chars.next() != Some('.') {
            return false;
        }

        // Must be followed by space or end of string
        matches!(chars.next(), None | Some(' '))
    }

    pub fn is_blockquote(line: &str) -> bool {
        line.trim_start().starts_with('>')
    }

    pub fn is_table_row(line: &str) -> bool {
        let trimmed = line.trim();
        trimmed.starts_with('|') && trimmed.ends_with('|')
    }

    /// Extract complete code block
    pub fn extract_code_block(&mut self) -> Option<String> {
        if self.position >= self.lines.len() ||
           !Self::is_code_block_start(self.lines[self.position]) {
            return None;
        }

        let start = self.position;
        self.position += 1;

        while self.position < self.lines.len() {
            if self.lines[self.position].trim_start().starts_with("```") {
                self.position += 1;
                return Some(self.lines[start..self.position].join("\n"));
            }
            self.position += 1;
        }

        Some(self.lines[start..self.position].join("\n"))  // Unclosed block
    }

    /// Extract complete list
    pub fn extract_list(&mut self) -> Option<String> {
        if self.position >= self.lines.len() ||
           !Self::is_list_item(self.lines[self.position]) {
            return None;
        }

        let start = self.position;

        while self.position < self.lines.len() {
            let line = self.lines[self.position];

            if Self::is_list_item(line) || line.starts_with("  ") || line.trim().is_empty() {
                self.position += 1;

                if line.trim().is_empty() &&
                   self.position < self.lines.len() &&
                   self.lines[self.position].trim().is_empty() {
                    break;
                }
            } else {
                break;
            }
        }

        if start == self.position { None } else { Some(self.lines[start..self.position].join("\n")) }
    }

    /// Extract complete table
    pub fn extract_table(&mut self) -> Option<String> {
        if self.position >= self.lines.len() ||
           !Self::is_table_row(self.lines[self.position]) {
            return None;
        }

        let start = self.position;

        while self.position < self.lines.len() {
            if Self::is_table_row(self.lines[self.position]) {
                self.position += 1;
            } else {
                break;
            }
        }

        if start == self.position { None } else { Some(self.lines[start..self.position].join("\n")) }
    }

    /// Extract blockquote
    pub fn extract_blockquote(&mut self) -> Option<String> {
        if self.position >= self.lines.len() ||
           !Self::is_blockquote(self.lines[self.position]) {
            return None;
        }

        let start = self.position;

        while self.position < self.lines.len() {
            let line = self.lines[self.position];

            if Self::is_blockquote(line) || line.trim().is_empty() {
                self.position += 1;
            } else {
                break;
            }
        }

        if start == self.position { None } else { Some(self.lines[start..self.position].join("\n")) }
    }

    /// Extract paragraph
    pub fn extract_paragraph(&mut self) -> Option<String> {
        if self.position >= self.lines.len() { return None; }

        let start = self.position;

        while self.position < self.lines.len() {
            let line = self.lines[self.position];

            if line.trim().is_empty() ||
               Self::parse_header(line).is_some() ||
               Self::is_code_block_start(line) ||
               Self::is_list_item(line) ||
               Self::is_blockquote(line) ||
               Self::is_table_row(line) {
                break;
            }

            self.position += 1;
        }

        if start == self.position { None } else { Some(self.lines[start..self.position].join("\n")) }
    }
}

/// Hierarchy-aware chunker (primary strategy)
pub struct HierarchyAwareChunker {
    #[allow(dead_code)]
    max_tokens: usize,
    min_tokens: usize,
    add_header_context: bool,
}

impl HierarchyAwareChunker {
    pub fn new(max_tokens: usize, min_tokens: usize) -> Self {
        Self {
            max_tokens,
            min_tokens,
            add_header_context: true,
        }
    }

    pub fn chunk(&self, text: &str) -> Vec<Chunk> {
        let mut chunks = Vec::new();
        let mut parser = MarkdownParser::new(text);
        let mut header_stack: Vec<(usize, String)> = Vec::new();
        let mut chunk_position = 0;

        while parser.position < parser.lines.len() {
            let line = parser.lines[parser.position];

            // Check for header
            if let Some((level, title)) = MarkdownParser::parse_header(line) {
                self.update_header_stack(&mut header_stack, level, title);
                parser.position += 1;
                continue;
            }

            // Extract content block
            let (content, chunk_type) = if MarkdownParser::is_code_block_start(line) {
                (parser.extract_code_block().unwrap_or_default(), ChunkType::CodeBlock)
            } else if MarkdownParser::is_table_row(line) {
                (parser.extract_table().unwrap_or_default(), ChunkType::Table)
            } else if MarkdownParser::is_list_item(line) {
                (parser.extract_list().unwrap_or_default(), ChunkType::List)
            } else if MarkdownParser::is_blockquote(line) {
                (parser.extract_blockquote().unwrap_or_default(), ChunkType::Quote)
            } else if line.trim().is_empty() {
                parser.position += 1;
                continue;
            } else {
                (parser.extract_paragraph().unwrap_or_default(), ChunkType::Paragraph)
            };

            if content.trim().is_empty() { continue; }

            // Build chunk with context
            let final_content = if self.add_header_context && !header_stack.is_empty() {
                format!("{}\n\n{}", self.build_header_context(&header_stack), content)
            } else {
                content.clone()
            };

            let token_count = TokenCounter::estimate(&final_content);

            // Never split code blocks or tables
            if chunk_type == ChunkType::CodeBlock || chunk_type == ChunkType::Table {
                chunks.push(Chunk {
                    content: final_content,
                    metadata: ChunkMetadata {
                        position: chunk_position,
                        token_count,
                        char_count: content.chars().count(),
                        language: None,
                        chunk_type,
                        header_hierarchy: header_stack.clone(),
                    },
                });
                chunk_position += 1;
            } else if token_count >= self.min_tokens {
                chunks.push(Chunk {
                    content: final_content,
                    metadata: ChunkMetadata {
                        position: chunk_position,
                        token_count,
                        char_count: content.chars().count(),
                        language: None,
                        chunk_type,
                        header_hierarchy: header_stack.clone(),
                    },
                });
                chunk_position += 1;
            }
        }

        chunks
    }

    fn update_header_stack(&self, stack: &mut Vec<(usize, String)>, level: usize, title: String) {
        stack.retain(|(l, _)| *l < level);
        stack.push((level, title));
    }

    fn build_header_context(&self, stack: &[(usize, String)]) -> String {
        if stack.is_empty() {
            return String::new();
        }

        // Pre-calculate capacity
        let capacity: usize = stack.iter()
            .map(|(level, title)| level + 1 + title.len() + 1) // # + space + title + \n
            .sum();

        let mut result = String::with_capacity(capacity);
        for (i, (level, title)) in stack.iter().enumerate() {
            if i > 0 {
                result.push('\n');
            }
            for _ in 0..*level {
                result.push('#');
            }
            result.push(' ');
            result.push_str(title);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_header() {
        assert_eq!(
            MarkdownParser::parse_header("# Title"),
            Some((1, "Title".to_string()))
        );
        assert_eq!(
            MarkdownParser::parse_header("## Section"),
            Some((2, "Section".to_string()))
        );
        assert_eq!(
            MarkdownParser::parse_header("### Subsection"),
            Some((3, "Subsection".to_string()))
        );
        assert_eq!(MarkdownParser::parse_header("####NoSpace"), None);
        assert_eq!(MarkdownParser::parse_header("Not a header"), None);
    }

    #[test]
    fn test_is_code_block_start() {
        assert!(MarkdownParser::is_code_block_start("```rust"));
        assert!(MarkdownParser::is_code_block_start("```"));
        assert!(!MarkdownParser::is_code_block_start("code"));
    }

    #[test]
    fn test_is_list_item() {
        assert!(MarkdownParser::is_list_item("- Item"));
        assert!(MarkdownParser::is_list_item("* Item"));
        assert!(MarkdownParser::is_list_item("+ Item"));
        assert!(MarkdownParser::is_list_item("1. Item"));
        assert!(!MarkdownParser::is_list_item("Not a list"));
    }

    #[test]
    fn test_is_blockquote() {
        assert!(MarkdownParser::is_blockquote("> Quote"));
        assert!(!MarkdownParser::is_blockquote("Not a quote"));
    }

    #[test]
    fn test_is_table_row() {
        assert!(MarkdownParser::is_table_row("| A | B |"));
        assert!(!MarkdownParser::is_table_row("Not a table"));
    }
}
