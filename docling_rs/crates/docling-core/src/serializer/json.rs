//! JSON serialization for Document and `DoclingDocument`
//!
//! This module provides JSON serialization using `serde_json`.
//! Since Document and `DoclingDocument` already implement Serialize,
//! this is mostly a convenience wrapper with formatting options.

use crate::document::{DoclingDocument, Document};
use serde_json::{self, to_string, to_string_pretty};

/// Options for JSON serialization
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JsonOptions {
    /// Pretty-print with indentation (default: true)
    pub pretty: bool,
    /// Indentation string when pretty=true (default: 2 spaces)
    pub indent: String,
}

impl Default for JsonOptions {
    #[inline]
    fn default() -> Self {
        Self {
            pretty: true,
            indent: "  ".to_string(),
        }
    }
}

/// JSON serializer for Document and `DoclingDocument`
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct JsonSerializer {
    options: JsonOptions,
}

impl JsonSerializer {
    /// Create a new JSON serializer with default options (pretty-printed)
    #[inline]
    #[must_use = "creates serializer with default options"]
    pub fn new() -> Self {
        Self {
            options: JsonOptions::default(),
        }
    }

    /// Create a new JSON serializer with custom options
    #[inline]
    #[must_use = "creates serializer with custom options"]
    pub const fn with_options(options: JsonOptions) -> Self {
        Self { options }
    }

    /// Serialize a Document to JSON
    ///
    /// # Errors
    /// Returns error if serialization fails
    #[must_use = "this function returns serialized JSON that should be used"]
    pub fn serialize_document(&self, doc: &Document) -> Result<String, serde_json::Error> {
        if self.options.pretty {
            to_string_pretty(doc)
        } else {
            to_string(doc)
        }
    }

    /// Serialize a `DoclingDocument` to JSON
    ///
    /// # Errors
    /// Returns error if serialization fails
    #[must_use = "this function returns serialized JSON that should be used"]
    pub fn serialize_docling_document(
        &self,
        doc: &DoclingDocument,
    ) -> Result<String, serde_json::Error> {
        if self.options.pretty {
            to_string_pretty(doc)
        } else {
            to_string(doc)
        }
    }

    /// Serialize a Document to compact JSON (no pretty-printing)
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails.
    #[must_use = "this function returns serialized JSON that should be used"]
    pub fn serialize_compact(doc: &Document) -> Result<String, serde_json::Error> {
        to_string(doc)
    }

    /// Serialize a `DoclingDocument` to compact JSON (no pretty-printing)
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails.
    #[must_use = "this function returns serialized JSON that should be used"]
    pub fn serialize_docling_document_compact(
        doc: &DoclingDocument,
    ) -> Result<String, serde_json::Error> {
        to_string(doc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::InputFormat;

    #[test]
    fn test_json_serialization_basic() {
        let doc = Document::from_markdown(
            "# Hello World\n\nThis is a test.".to_string(),
            InputFormat::Md,
        );

        let serializer = JsonSerializer::new();
        let json = serializer.serialize_document(&doc).unwrap();

        // Should contain the markdown content
        assert!(json.contains("Hello World"));
        assert!(json.contains("This is a test"));

        // Should be pretty-printed (contains newlines)
        assert!(json.contains('\n'));
    }

    #[test]
    fn test_json_serialization_compact() {
        let doc = Document::from_markdown("# Test".to_string(), InputFormat::Md);

        let serializer = JsonSerializer::with_options(JsonOptions {
            pretty: false,
            indent: "  ".to_string(),
        });
        let json = serializer.serialize_document(&doc).unwrap();

        // Should contain content
        assert!(json.contains("Test"));

        // Should be compact (minimal whitespace)
        // Note: serde_json still includes some whitespace even in compact mode
        assert!(!json.contains("\n  "));
    }

    #[test]
    fn test_json_serialization_metadata() {
        let mut doc = Document::from_markdown("# Test".to_string(), InputFormat::Pdf);
        doc.metadata.title = Some("Test Document".to_string());
        doc.metadata.author = Some("Test Author".to_string());
        doc.metadata.num_pages = Some(42);

        let serializer = JsonSerializer::new();
        let json = serializer.serialize_document(&doc).unwrap();

        // Should contain metadata
        assert!(json.contains("Test Document"));
        assert!(json.contains("Test Author"));
        assert!(json.contains("42"));
    }

    #[test]
    fn test_json_deserialization() {
        let doc = Document::from_markdown("# Hello".to_string(), InputFormat::Html);

        let serializer = JsonSerializer::new();
        let json = serializer.serialize_document(&doc).unwrap();

        // Should be able to deserialize back
        let deserialized: Document = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.markdown, doc.markdown);
        assert_eq!(deserialized.format, doc.format);
    }

    #[test]
    fn test_json_serializer_default() {
        let default = JsonSerializer::default();
        let new = JsonSerializer::new();
        assert_eq!(default, new);
    }
}
