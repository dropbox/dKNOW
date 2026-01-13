use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// RTF parser using rtf-parser crate
///
/// Parses Rich Text Format (.rtf) files to extract structured content.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct RtfParser;

impl RtfParser {
    /// Parse an RTF file from a path
    ///
    /// Uses rtf-parser v0.4.2 (<https://github.com/d0rianb/rtf-parser>)
    /// Implements RTF specification 1.9 with UTF-16 unicode support
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file cannot be read (I/O error)
    /// - The RTF content is malformed
    ///
    /// # Examples
    /// ```no_run
    /// use docling_legacy::RtfParser;
    ///
    /// let doc = RtfParser::parse_file("document.rtf").unwrap();
    /// ```
    #[must_use = "parsing produces a result that should be handled"]
    pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<rtf_parser::RtfDocument> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .context(format!("Failed to read RTF file: {}", path.display()))?;

        Self::parse_str(&content)
    }

    /// Parse RTF content from a string
    ///
    /// # Errors
    ///
    /// Returns an error if the RTF content is malformed.
    ///
    /// # Examples
    /// ```
    /// use docling_legacy::RtfParser;
    ///
    /// let rtf = r#"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
    /// \f0\fs60 Hello, World!
    /// }"#;
    ///
    /// let doc = RtfParser::parse_str(rtf).unwrap();
    /// ```
    #[must_use = "parsing produces a result that should be handled"]
    pub fn parse_str(content: &str) -> Result<rtf_parser::RtfDocument> {
        // Use rtf-parser's RtfDocument::try_from
        rtf_parser::RtfDocument::try_from(content)
            .map_err(|e| anyhow::anyhow!("Failed to parse RTF: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_rtf() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs60 Hello, World!
}";

        let doc = RtfParser::parse_str(rtf).unwrap();
        // Extract text from body
        let text: String = doc
            .body
            .iter()
            .map(|block| block.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(text.contains("Hello, World!"));
    }

    #[test]
    fn test_parse_with_formatting() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24 This is \b bold \b0 and this is \i italic\i0.
}";

        let doc = RtfParser::parse_str(rtf).unwrap();
        let text: String = doc
            .body
            .iter()
            .map(|block| block.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(text.contains("bold"));
        assert!(text.contains("italic"));
    }

    #[test]
    fn test_parse_with_paragraph() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24 First paragraph.\par
Second paragraph.
}";

        let doc = RtfParser::parse_str(rtf).unwrap();
        let text: String = doc
            .body
            .iter()
            .map(|block| block.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(text.contains("First paragraph"));
        assert!(text.contains("Second paragraph"));
    }
}
