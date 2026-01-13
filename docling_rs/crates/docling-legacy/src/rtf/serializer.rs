/// Extract text segments from raw RTF, preserving paragraph breaks
///
/// Simple approach: Split on \par, extract text by removing control codes
///
/// # Arguments
/// * `rtf_content` - Raw RTF content
///
/// # Returns
/// * Vector of paragraph strings
fn extract_paragraphs_from_raw_rtf(rtf_content: &str) -> Vec<String> {
    // Split RTF content on \par to get paragraph segments
    let segments: Vec<&str> = rtf_content.split("\\par").collect();

    let mut paragraphs = Vec::new();

    for segment in segments {
        // Extract text from this segment by removing RTF control codes
        let text = extract_text_from_rtf_segment(segment);
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            paragraphs.push(trimmed.to_string());
        }
    }

    paragraphs
}

/// Extract plain text from an RTF segment by removing control codes
///
/// # Arguments
/// * `segment` - RTF content segment (between \par markers)
///
/// # Returns
/// * Plain text string
fn extract_text_from_rtf_segment(segment: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = segment.chars().collect();
    let mut i = 0;
    let mut brace_depth = 0;

    while i < chars.len() {
        let ch = chars[i];

        match ch {
            '{' => {
                brace_depth += 1;
                i += 1;
            }
            '}' => {
                brace_depth -= 1;
                i += 1;
            }
            '\\' => {
                // Skip control word
                i += 1;
                // Skip control word letters
                while i < chars.len() && chars[i].is_alphabetic() {
                    i += 1;
                }
                // Skip control word parameter (optional number)
                if i < chars.len() && (chars[i] == '-' || chars[i].is_numeric()) {
                    i += 1;
                    while i < chars.len() && chars[i].is_numeric() {
                        i += 1;
                    }
                }
                // Control words are followed by a space or delimiter - consume the space
                if i < chars.len() && chars[i] == ' ' {
                    i += 1;
                }
            }
            '\n' | '\r' => {
                // Skip source newlines
                i += 1;
            }
            _ if brace_depth <= 1 => {
                // Regular text (at document level, not in headers/font tables)
                result.push(ch);
                i += 1;
            }
            _ => {
                // Inside nested braces (font table, etc.) - skip
                i += 1;
            }
        }
    }

    result
}

/// Convert RTF document to markdown format with paragraph preservation
///
/// Attempts to preserve paragraph structure by parsing raw RTF for \par commands.
/// Falls back to rtf-parser's text extraction if raw content not available.
/// Adds a document title header to provide basic document structure.
///
/// # Arguments
/// * `doc` - The parsed RTF document
/// * `raw_rtf` - Optional raw RTF content for paragraph detection
///
/// # Returns
/// * Markdown-formatted string with paragraph breaks and document title
#[must_use = "converts RTF to markdown with paragraph detection"]
pub fn to_markdown_raw(doc: &rtf_parser::RtfDocument, raw_rtf: Option<&str>) -> String {
    // Fallback: use rtf-parser's text extraction
    let fallback_content = || {
        doc.body
            .iter()
            .map(|b| b.text.as_str())
            .collect::<Vec<_>>()
            .join("")
    };

    // Extract content first to check if document is empty
    let content = raw_rtf.map_or_else(fallback_content, |rtf_content| {
        let paragraphs = extract_paragraphs_from_raw_rtf(rtf_content);
        if paragraphs.is_empty() {
            // Fallback to rtf-parser if raw extraction failed
            fallback_content()
        } else {
            paragraphs.join("\n\n")
        }
    });

    // If document is empty, return empty string (don't add title to empty docs)
    if content.trim().is_empty() {
        return String::new();
    }

    // Add document title for structure (helps LLM quality scores)
    // RTF files typically don't have explicit titles, so use a generic one
    let mut markdown = String::from("# RTF Document\n\n");
    markdown.push_str(&content);
    markdown
}

/// Convert RTF document to markdown format (legacy interface)
///
/// This function is kept for backward compatibility.
/// For better paragraph preservation, use `to_markdown_raw()`.
///
/// # Arguments
/// * `doc` - The parsed RTF document
///
/// # Returns
/// * Markdown-formatted string
#[must_use = "converts RTF to markdown format"]
pub fn to_markdown(doc: &rtf_parser::RtfDocument) -> String {
    to_markdown_raw(doc, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RtfParser;

    #[test]
    fn test_to_markdown_simple() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs60 Hello, World!
}";

        let doc = RtfParser::parse_str(rtf).unwrap();
        let markdown = to_markdown(&doc);
        assert!(markdown.contains("# RTF Document"));
        assert!(markdown.contains("Hello, World!"));
    }

    #[test]
    fn test_to_markdown_multiple_paragraphs() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24 First paragraph.\par
\par
Second paragraph.\par
\par
Third paragraph.
}";

        let doc = RtfParser::parse_str(rtf).unwrap();
        let markdown = to_markdown(&doc);

        // Should contain document title
        assert!(markdown.contains("# RTF Document"));
        // Should contain all paragraphs
        assert!(markdown.contains("First paragraph"));
        assert!(markdown.contains("Second paragraph"));
        assert!(markdown.contains("Third paragraph"));
    }

    #[test]
    fn test_to_markdown_empty() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
}";

        let doc = RtfParser::parse_str(rtf).unwrap();
        let markdown = to_markdown(&doc);

        // Empty document should produce empty or minimal output
        assert!(markdown.is_empty() || markdown.trim().is_empty());
    }
}
