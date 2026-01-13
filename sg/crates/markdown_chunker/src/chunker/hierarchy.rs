// src/chunker/hierarchy.rs

use crate::metadata::{extract_links, Chunk, ChunkMetadata, ChunkType};
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

    /// Extract language identifier from code fence line
    /// e.g., "```rust" -> Some("rust"), "```" -> None
    pub fn parse_code_fence_language(line: &str) -> Option<String> {
        let trimmed = line.trim_start();
        if !trimmed.starts_with("```") {
            return None;
        }
        let after_backticks = trimmed[3..].trim();
        if after_backticks.is_empty() {
            return None;
        }
        // Take the first word (language identifier), ignoring any metadata after space
        // e.g., "rust,linenos" -> "rust", "python title='example'" -> "python"
        after_backticks
            .split(|c: char| c.is_whitespace() || c == ',')
            .next()
            .filter(|s| !s.is_empty())
            .map(str::to_lowercase)
    }

    pub fn is_list_item(line: &str) -> bool {
        let trimmed = line.trim_start();
        trimmed.starts_with("- ")
            || trimmed.starts_with("* ")
            || trimmed.starts_with("+ ")
            || Self::is_ordered_list(trimmed)
    }

    fn is_ordered_list(line: &str) -> bool {
        let mut chars = line.chars().peekable();

        // Must start with digit
        if !chars.peek().is_some_and(|c| c.is_numeric()) {
            return false;
        }

        // Skip all leading digits
        while chars.peek().is_some_and(|c| c.is_numeric()) {
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

    /// Extract complete code block with optional language identifier
    /// Returns (content, language) tuple
    pub fn extract_code_block(&mut self) -> Option<(String, Option<String>)> {
        if self.position >= self.lines.len()
            || !Self::is_code_block_start(self.lines[self.position])
        {
            return None;
        }

        // Extract language from opening fence
        let language = Self::parse_code_fence_language(self.lines[self.position]);

        let start = self.position;
        self.position += 1;

        while self.position < self.lines.len() {
            if self.lines[self.position].trim_start().starts_with("```") {
                self.position += 1;
                return Some((self.lines[start..self.position].join("\n"), language));
            }
            self.position += 1;
        }

        Some((self.lines[start..self.position].join("\n"), language)) // Unclosed block
    }

    /// Extract complete list
    pub fn extract_list(&mut self) -> Option<String> {
        if self.position >= self.lines.len() || !Self::is_list_item(self.lines[self.position]) {
            return None;
        }

        let start = self.position;

        while self.position < self.lines.len() {
            let line = self.lines[self.position];

            if Self::is_list_item(line) || line.starts_with("  ") || line.trim().is_empty() {
                self.position += 1;

                if line.trim().is_empty()
                    && self.position < self.lines.len()
                    && self.lines[self.position].trim().is_empty()
                {
                    break;
                }
            } else {
                break;
            }
        }

        if start == self.position {
            None
        } else {
            Some(self.lines[start..self.position].join("\n"))
        }
    }

    /// Extract complete table
    pub fn extract_table(&mut self) -> Option<String> {
        if self.position >= self.lines.len() || !Self::is_table_row(self.lines[self.position]) {
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

        if start == self.position {
            None
        } else {
            Some(self.lines[start..self.position].join("\n"))
        }
    }

    /// Extract blockquote
    pub fn extract_blockquote(&mut self) -> Option<String> {
        if self.position >= self.lines.len() || !Self::is_blockquote(self.lines[self.position]) {
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

        if start == self.position {
            None
        } else {
            Some(self.lines[start..self.position].join("\n"))
        }
    }

    /// Extract paragraph
    pub fn extract_paragraph(&mut self) -> Option<String> {
        if self.position >= self.lines.len() {
            return None;
        }

        let start = self.position;

        while self.position < self.lines.len() {
            let line = self.lines[self.position];

            if line.trim().is_empty()
                || Self::parse_header(line).is_some()
                || Self::is_code_block_start(line)
                || Self::is_list_item(line)
                || Self::is_blockquote(line)
                || Self::is_table_row(line)
            {
                break;
            }

            self.position += 1;
        }

        if start == self.position {
            None
        } else {
            Some(self.lines[start..self.position].join("\n"))
        }
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

            // Extract content block with optional language for code blocks
            let (content, chunk_type, language) = if MarkdownParser::is_code_block_start(line) {
                let (code_content, lang) = parser.extract_code_block().unwrap_or_default();
                (code_content, ChunkType::CodeBlock, lang)
            } else if MarkdownParser::is_table_row(line) {
                (
                    parser.extract_table().unwrap_or_default(),
                    ChunkType::Table,
                    None,
                )
            } else if MarkdownParser::is_list_item(line) {
                (
                    parser.extract_list().unwrap_or_default(),
                    ChunkType::List,
                    None,
                )
            } else if MarkdownParser::is_blockquote(line) {
                (
                    parser.extract_blockquote().unwrap_or_default(),
                    ChunkType::Quote,
                    None,
                )
            } else if line.trim().is_empty() {
                parser.position += 1;
                continue;
            } else {
                (
                    parser.extract_paragraph().unwrap_or_default(),
                    ChunkType::Paragraph,
                    None,
                )
            };

            if content.trim().is_empty() {
                continue;
            }

            // Build chunk with context
            let final_content = if self.add_header_context && !header_stack.is_empty() {
                format!(
                    "{}\n\n{}",
                    self.build_header_context(&header_stack),
                    content
                )
            } else {
                content.clone()
            };

            let token_count = TokenCounter::estimate(&final_content);

            // Include chunk if: it's a code block/table (never split these), or meets min_tokens
            let is_structural =
                chunk_type == ChunkType::CodeBlock || chunk_type == ChunkType::Table;
            if is_structural || token_count >= self.min_tokens {
                // Extract links from the content
                let links = extract_links(&final_content);

                chunks.push(Chunk {
                    content: final_content,
                    metadata: ChunkMetadata {
                        position: chunk_position,
                        token_count,
                        char_count: content.chars().count(),
                        language,
                        chunk_type,
                        header_hierarchy: header_stack.clone(),
                        links,
                    },
                });
                chunk_position += 1;
            }
        }

        chunks
    }

    #[allow(clippy::unused_self)]
    fn update_header_stack(&self, stack: &mut Vec<(usize, String)>, level: usize, title: String) {
        stack.retain(|(l, _)| *l < level);
        stack.push((level, title));
    }

    #[allow(clippy::unused_self)]
    fn build_header_context(&self, stack: &[(usize, String)]) -> String {
        if stack.is_empty() {
            return String::new();
        }

        // Pre-calculate capacity
        let capacity: usize = stack
            .iter()
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

    #[test]
    fn test_parse_code_fence_language() {
        // Basic languages
        assert_eq!(
            MarkdownParser::parse_code_fence_language("```rust"),
            Some("rust".to_string())
        );
        assert_eq!(
            MarkdownParser::parse_code_fence_language("```python"),
            Some("python".to_string())
        );
        assert_eq!(
            MarkdownParser::parse_code_fence_language("```javascript"),
            Some("javascript".to_string())
        );

        // Case normalization (should lowercase)
        assert_eq!(
            MarkdownParser::parse_code_fence_language("```Rust"),
            Some("rust".to_string())
        );
        assert_eq!(
            MarkdownParser::parse_code_fence_language("```PYTHON"),
            Some("python".to_string())
        );

        // No language specified
        assert_eq!(MarkdownParser::parse_code_fence_language("```"), None);
        assert_eq!(MarkdownParser::parse_code_fence_language("```   "), None);

        // Language with metadata (common in markdown processors)
        assert_eq!(
            MarkdownParser::parse_code_fence_language("```rust,linenos"),
            Some("rust".to_string())
        );
        assert_eq!(
            MarkdownParser::parse_code_fence_language("```python title='example'"),
            Some("python".to_string())
        );

        // Indented code fence
        assert_eq!(
            MarkdownParser::parse_code_fence_language("  ```rust"),
            Some("rust".to_string())
        );

        // Not a code fence
        assert_eq!(
            MarkdownParser::parse_code_fence_language("normal text"),
            None
        );
        assert_eq!(MarkdownParser::parse_code_fence_language("``rust"), None);
    }

    #[test]
    fn test_extract_code_block_with_language() {
        let markdown = "```rust\nfn main() {}\n```";
        let mut parser = MarkdownParser::new(markdown);
        let result = parser.extract_code_block();
        assert!(result.is_some());
        let (content, language) = result.unwrap();
        assert!(content.contains("fn main()"));
        assert_eq!(language, Some("rust".to_string()));
    }

    #[test]
    fn test_extract_code_block_without_language() {
        let markdown = "```\nsome code\n```";
        let mut parser = MarkdownParser::new(markdown);
        let result = parser.extract_code_block();
        assert!(result.is_some());
        let (content, language) = result.unwrap();
        assert!(content.contains("some code"));
        assert_eq!(language, None);
    }

    #[test]
    fn test_chunk_code_block_has_language() {
        let markdown = "# Header\n\n```python\nprint('hello')\n```";
        let chunker = HierarchyAwareChunker::new(800, 0);
        let chunks = chunker.chunk(markdown);

        // Find the code block chunk
        let code_chunk = chunks
            .iter()
            .find(|c| c.metadata.chunk_type == ChunkType::CodeBlock);
        assert!(code_chunk.is_some());
        assert_eq!(
            code_chunk.unwrap().metadata.language,
            Some("python".to_string())
        );
    }
}
