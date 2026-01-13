//! YAML serialization for Document and `DoclingDocument`
//!
//! This module provides YAML serialization using `serde_yaml`.
//! YAML is more human-readable than JSON and useful for configuration and debugging.

use crate::document::{DoclingDocument, Document};
use serde_yaml;

/// Options for YAML serialization
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct YamlOptions {
    /// Currently no options, but reserved for future use
    /// (`serde_yaml` has fewer formatting options than `serde_json`)
    _placeholder: (),
}

/// YAML serializer for Document and `DoclingDocument`
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct YamlSerializer {
    #[allow(
        dead_code,
        reason = "stored for future use when serde_yaml adds formatting options"
    )]
    options: YamlOptions,
}

impl YamlSerializer {
    /// Create a new YAML serializer with default options
    #[inline]
    #[must_use = "creates serializer with default options"]
    pub const fn new() -> Self {
        Self {
            options: YamlOptions { _placeholder: () },
        }
    }

    /// Create a new YAML serializer with custom options
    #[inline]
    #[must_use = "creates serializer with custom options"]
    pub const fn with_options(options: YamlOptions) -> Self {
        Self { options }
    }

    /// Serialize a Document to YAML
    ///
    /// # Errors
    /// Returns error if serialization fails
    #[must_use = "this function returns serialized YAML that should be used"]
    pub fn serialize_document(&self, doc: &Document) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(doc)
    }

    /// Serialize a `DoclingDocument` to YAML
    ///
    /// # Errors
    /// Returns error if serialization fails
    #[must_use = "this function returns serialized YAML that should be used"]
    pub fn serialize_docling_document(
        &self,
        doc: &DoclingDocument,
    ) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(doc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::InputFormat;

    #[test]
    fn test_yaml_serialization_basic() {
        let doc = Document::from_markdown(
            "# Hello World\n\nThis is a test.".to_string(),
            InputFormat::Md,
        );

        let serializer = YamlSerializer::new();
        let yaml = serializer.serialize_document(&doc).unwrap();

        // Should contain the markdown content
        assert!(yaml.contains("Hello World"));
        assert!(yaml.contains("This is a test"));

        // YAML should have field names
        assert!(yaml.contains("markdown:"));
        assert!(yaml.contains("format:"));
    }

    #[test]
    fn test_yaml_serialization_metadata() {
        let mut doc = Document::from_markdown("# Test".to_string(), InputFormat::Pdf);
        doc.metadata.title = Some("Test Document".to_string());
        doc.metadata.author = Some("Test Author".to_string());
        doc.metadata.num_pages = Some(42);

        let serializer = YamlSerializer::new();
        let yaml = serializer.serialize_document(&doc).unwrap();

        // Should contain metadata
        assert!(yaml.contains("Test Document"));
        assert!(yaml.contains("Test Author"));
        assert!(yaml.contains("42"));
        assert!(yaml.contains("metadata:"));
    }

    #[test]
    fn test_yaml_deserialization() {
        let doc = Document::from_markdown("# Hello".to_string(), InputFormat::Html);

        let serializer = YamlSerializer::new();
        let yaml = serializer.serialize_document(&doc).unwrap();

        // Should be able to deserialize back
        let deserialized: Document = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(deserialized.markdown, doc.markdown);
        assert_eq!(deserialized.format, doc.format);
    }

    #[test]
    fn test_yaml_human_readable() {
        let doc = Document::from_markdown("# Test".to_string(), InputFormat::Md);

        let serializer = YamlSerializer::new();
        let yaml = serializer.serialize_document(&doc).unwrap();

        // YAML should be human-readable with newlines and indentation
        assert!(yaml.contains('\n'));
        // YAML uses indentation for structure
        assert!(yaml.contains("  ") || yaml.contains("format:"));
    }

    #[test]
    fn test_yaml_serializer_default() {
        let default = YamlSerializer::default();
        let new = YamlSerializer::new();
        assert_eq!(default, new);
    }
}
