// src/metadata.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub content: String,
    pub metadata: ChunkMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    pub position: usize,
    pub token_count: usize,
    pub char_count: usize,
    pub language: Option<String>,
    pub chunk_type: ChunkType,
    pub header_hierarchy: Vec<(usize, String)>,
    /// Links found in this chunk (markdown links and wiki-style links)
    pub links: Vec<Link>,
}

/// A link extracted from markdown content
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Link {
    /// The display text of the link
    pub text: String,
    /// The link target (URL, path, or wiki-style reference)
    pub target: String,
    /// The type of link
    pub link_type: LinkType,
}

/// Types of links found in markdown
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LinkType {
    /// Standard markdown link `[text](url)`
    Markdown,
    /// Wiki-style link `[[page]]` or `[[page|text]]`
    Wiki,
    /// Reference-style link `[text][ref]`
    Reference,
    /// Autolink `<url>` or bare URL
    Autolink,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ChunkType {
    Paragraph,
    CodeBlock,
    List,
    Table,
    Quote,
    Heading,
}

/// Extract all links from markdown content
pub fn extract_links(content: &str) -> Vec<Link> {
    let mut links = Vec::new();

    // Extract standard markdown links: [text](url)
    extract_markdown_links(content, &mut links);

    // Extract wiki-style links: [[page]] or [[page|text]]
    extract_wiki_links(content, &mut links);

    // Extract reference-style links: [text][ref]
    extract_reference_links(content, &mut links);

    // Extract autolinks: <url> and bare URLs
    extract_autolinks(content, &mut links);

    links
}

/// Extract standard markdown links `[text](url)`
fn extract_markdown_links(content: &str, links: &mut Vec<Link>) {
    let mut chars = content.char_indices().peekable();

    while let Some((i, c)) = chars.next() {
        if c == '[' {
            // Check if this is an image link ![...] - skip those
            if i > 0 && content.as_bytes().get(i - 1) == Some(&b'!') {
                continue;
            }

            // Find matching ]
            let mut bracket_depth = 1;
            let mut text_end = None;
            let text_start = i + 1;

            for (j, ch) in chars.by_ref() {
                match ch {
                    '[' => bracket_depth += 1,
                    ']' => {
                        bracket_depth -= 1;
                        if bracket_depth == 0 {
                            text_end = Some(j);
                            break;
                        }
                    }
                    _ => {}
                }
            }

            if let Some(text_end) = text_end {
                // Check for (url) immediately after ]
                if let Some(&(_, '(')) = chars.peek() {
                    chars.next(); // consume '('
                    let url_start = text_end + 2;
                    let mut paren_depth = 1;
                    let mut url_end = None;

                    for (j, ch) in chars.by_ref() {
                        match ch {
                            '(' => paren_depth += 1,
                            ')' => {
                                paren_depth -= 1;
                                if paren_depth == 0 {
                                    url_end = Some(j);
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }

                    if let Some(url_end) = url_end {
                        let text = &content[text_start..text_end];
                        let target = &content[url_start..url_end];

                        // Skip empty links and code-like patterns
                        if !text.is_empty() && !target.is_empty() {
                            // Remove title from URL if present: url "title" -> url
                            let target = target
                                .split_once(['"', '\''])
                                .map(|(url, _)| url.trim())
                                .unwrap_or(target)
                                .trim();

                            links.push(Link {
                                text: text.to_string(),
                                target: target.to_string(),
                                link_type: LinkType::Markdown,
                            });
                        }
                    }
                }
            }
        }
    }
}

/// Extract wiki-style links `[[page]]` or `[[page|text]]`
fn extract_wiki_links(content: &str, links: &mut Vec<Link>) {
    let mut i = 0;
    let bytes = content.as_bytes();

    while i < bytes.len().saturating_sub(3) {
        if bytes[i] == b'[' && bytes[i + 1] == b'[' {
            // Found opening [[
            let start = i + 2;
            let mut end = None;

            // Find closing ]]
            for j in start..bytes.len().saturating_sub(1) {
                if bytes[j] == b']' && bytes[j + 1] == b']' {
                    end = Some(j);
                    break;
                }
            }

            if let Some(end) = end {
                let inner = &content[start..end];

                // Check for pipe separator: [[page|display text]]
                let (target, text) = if let Some(pipe_pos) = inner.find('|') {
                    (&inner[..pipe_pos], &inner[pipe_pos + 1..])
                } else {
                    (inner, inner)
                };

                if !target.is_empty() {
                    links.push(Link {
                        text: text.trim().to_string(),
                        target: target.trim().to_string(),
                        link_type: LinkType::Wiki,
                    });
                }

                i = end + 2;
                continue;
            }
        }
        i += 1;
    }
}

/// Extract reference-style links `[text][ref]`
fn extract_reference_links(content: &str, links: &mut Vec<Link>) {
    let mut chars = content.char_indices().peekable();

    while let Some((i, c)) = chars.next() {
        if c == '[' {
            // Check if this is an image link ![...] - skip those
            if i > 0 && content.as_bytes().get(i - 1) == Some(&b'!') {
                continue;
            }

            // Find matching ]
            let mut bracket_depth = 1;
            let mut text_end = None;
            let text_start = i + 1;

            for (j, ch) in chars.by_ref() {
                match ch {
                    '[' => bracket_depth += 1,
                    ']' => {
                        bracket_depth -= 1;
                        if bracket_depth == 0 {
                            text_end = Some(j);
                            break;
                        }
                    }
                    _ => {}
                }
            }

            if let Some(text_end) = text_end {
                // Check for [ref] immediately after ]
                if let Some(&(_, '[')) = chars.peek() {
                    chars.next(); // consume '['
                    let ref_start = text_end + 2;
                    let mut ref_end = None;

                    for (j, ch) in chars.by_ref() {
                        if ch == ']' {
                            ref_end = Some(j);
                            break;
                        }
                    }

                    if let Some(ref_end) = ref_end {
                        let text = &content[text_start..text_end];
                        let reference = &content[ref_start..ref_end];

                        // Skip empty references
                        if !text.is_empty() {
                            // If reference is empty, use text as reference
                            let target = if reference.is_empty() {
                                text
                            } else {
                                reference
                            };

                            links.push(Link {
                                text: text.to_string(),
                                target: target.to_string(),
                                link_type: LinkType::Reference,
                            });
                        }
                    }
                }
            }
        }
    }
}

/// Extract autolinks `<url>` and bare URLs
fn extract_autolinks(content: &str, links: &mut Vec<Link>) {
    // Extract angle-bracket autolinks: <https://example.com>
    let mut i = 0;
    let bytes = content.as_bytes();

    while i < bytes.len() {
        if bytes[i] == b'<' {
            // Look for closing >
            let start = i + 1;
            if let Some(end_offset) = content[start..].find('>') {
                let inner = &content[start..start + end_offset];

                // Check if it looks like a URL
                if inner.starts_with("http://")
                    || inner.starts_with("https://")
                    || inner.starts_with("mailto:")
                    || inner.starts_with("ftp://")
                {
                    links.push(Link {
                        text: inner.to_string(),
                        target: inner.to_string(),
                        link_type: LinkType::Autolink,
                    });
                }
                i = start + end_offset + 1;
                continue;
            }
        }
        i += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_markdown_links() {
        let content = "Check out [this link](https://example.com) for more info.";
        let links = extract_links(content);

        assert_eq!(links.len(), 1);
        assert_eq!(links[0].text, "this link");
        assert_eq!(links[0].target, "https://example.com");
        assert_eq!(links[0].link_type, LinkType::Markdown);
    }

    #[test]
    fn test_extract_markdown_links_with_title() {
        let content = r#"See [docs](./README.md "Documentation") here."#;
        let links = extract_links(content);

        assert_eq!(links.len(), 1);
        assert_eq!(links[0].text, "docs");
        assert_eq!(links[0].target, "./README.md");
    }

    #[test]
    fn test_extract_wiki_links() {
        let content = "See [[Getting Started]] for setup, or [[API Reference|API docs]].";
        let links = extract_links(content);

        assert_eq!(links.len(), 2);

        assert_eq!(links[0].text, "Getting Started");
        assert_eq!(links[0].target, "Getting Started");
        assert_eq!(links[0].link_type, LinkType::Wiki);

        assert_eq!(links[1].text, "API docs");
        assert_eq!(links[1].target, "API Reference");
        assert_eq!(links[1].link_type, LinkType::Wiki);
    }

    #[test]
    fn test_extract_reference_links() {
        let content = "Read the [introduction][intro] section.";
        let links = extract_links(content);

        assert_eq!(links.len(), 1);
        assert_eq!(links[0].text, "introduction");
        assert_eq!(links[0].target, "intro");
        assert_eq!(links[0].link_type, LinkType::Reference);
    }

    #[test]
    fn test_extract_autolinks() {
        let content = "Visit <https://example.com> or <mailto:test@example.com>.";
        let links = extract_links(content);

        assert_eq!(links.len(), 2);
        assert_eq!(links[0].target, "https://example.com");
        assert_eq!(links[0].link_type, LinkType::Autolink);
        assert_eq!(links[1].target, "mailto:test@example.com");
    }

    #[test]
    fn test_multiple_links() {
        let content = r"
# Documentation

Check [our guide](./guide.md) and [[FAQ]] for help.
Also see [API][api-ref] documentation.

<https://github.com/example>
";
        let links = extract_links(content);

        assert_eq!(links.len(), 4);
    }

    #[test]
    fn test_no_links() {
        let content = "This is plain text without any links.";
        let links = extract_links(content);

        assert!(links.is_empty());
    }

    #[test]
    fn test_skip_image_links() {
        let content = "Here is an image: ![alt text](image.png) but [this](link.md) is a link.";
        let links = extract_links(content);

        // Should only find the regular link, not the image
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].text, "this");
        assert_eq!(links[0].target, "link.md");
    }

    #[test]
    fn test_nested_brackets() {
        let content = "Check [[nested [brackets]]] here.";
        let links = extract_links(content);

        // Should handle nested brackets gracefully
        assert!(!links.is_empty());
    }

    #[test]
    fn test_relative_paths() {
        let content = r"
- [Parent](../README.md)
- [Sibling](./other.md)
- [Absolute](/docs/api.md)
";
        let links = extract_links(content);

        assert_eq!(links.len(), 3);
        assert_eq!(links[0].target, "../README.md");
        assert_eq!(links[1].target, "./other.md");
        assert_eq!(links[2].target, "/docs/api.md");
    }
}
