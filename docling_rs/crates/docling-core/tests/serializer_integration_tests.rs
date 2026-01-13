//! Integration tests for JSON and YAML serializers
//!
//! These tests verify that serializers work correctly with real document structures,
//! not just the simple unit test cases.

use docling_core::{Document, InputFormat, JsonSerializer, YamlSerializer};

/// Test JSON serialization with complex document metadata
#[test]
fn test_json_serializer_with_full_metadata() {
    let mut doc = Document::from_markdown(
        "# Test Document\n\n## Section 1\n\nContent here.\n\n## Section 2\n\nMore content."
            .to_string(),
        InputFormat::Pdf,
    );

    // Set comprehensive metadata (using actual DocumentMetadata fields)
    doc.metadata.title = Some("Integration Test Document".to_string());
    doc.metadata.author = Some("Test Suite".to_string());
    doc.metadata.num_pages = Some(5);
    doc.metadata.num_characters = 1000;

    // Serialize to JSON
    let serializer = JsonSerializer::new();
    let json = serializer
        .serialize_document(&doc)
        .expect("JSON serialization failed");

    // Verify JSON contains all metadata fields
    assert!(json.contains("Integration Test Document"));
    assert!(json.contains("Test Suite"));
    assert!(json.contains("1000")); // num_characters

    // Verify JSON structure is valid
    let _parsed: serde_json::Value = serde_json::from_str(&json).expect("JSON should be valid");
}

/// Test YAML serialization with complex document metadata
#[test]
fn test_yaml_serializer_with_full_metadata() {
    let mut doc = Document::from_markdown(
        "# Test Document\n\n## Section 1\n\nContent here.".to_string(),
        InputFormat::Docx,
    );

    // Set comprehensive metadata
    doc.metadata.title = Some("YAML Test Document".to_string());
    doc.metadata.author = Some("YAML Test Suite".to_string());
    doc.metadata.num_pages = Some(3);

    // Serialize to YAML
    let serializer = YamlSerializer::new();
    let yaml = serializer
        .serialize_document(&doc)
        .expect("YAML serialization failed");

    // Verify YAML contains metadata
    assert!(yaml.contains("YAML Test Document"));
    assert!(yaml.contains("YAML Test Suite"));
    assert!(yaml.contains("metadata:"));

    // Verify YAML structure is valid
    let _parsed: serde_yaml::Value = serde_yaml::from_str(&yaml).expect("YAML should be valid");
}

/// Test JSON round-trip: serialize then deserialize
#[test]
fn test_json_roundtrip_preservation() {
    let mut doc = Document::from_markdown(
        "# Roundtrip Test\n\nThis tests serialization → deserialization.".to_string(),
        InputFormat::Html,
    );
    doc.metadata.title = Some("Roundtrip Document".to_string());
    doc.metadata.num_pages = Some(1);

    // Serialize
    let serializer = JsonSerializer::new();
    let json = serializer
        .serialize_document(&doc)
        .expect("JSON serialization failed");

    // Deserialize
    let deserialized: Document = serde_json::from_str(&json).expect("JSON deserialization failed");

    // Verify all fields preserved
    assert_eq!(deserialized.markdown, doc.markdown);
    assert_eq!(deserialized.format, doc.format);
    assert_eq!(deserialized.metadata.title, doc.metadata.title);
    assert_eq!(deserialized.metadata.num_pages, doc.metadata.num_pages);
}

/// Test YAML round-trip: serialize then deserialize
#[test]
fn test_yaml_roundtrip_preservation() {
    let mut doc = Document::from_markdown(
        "# YAML Roundtrip\n\nContent preservation test.".to_string(),
        InputFormat::Csv,
    );
    doc.metadata.author = Some("YAML Tester".to_string());

    // Serialize
    let serializer = YamlSerializer::new();
    let yaml = serializer
        .serialize_document(&doc)
        .expect("YAML serialization failed");

    // Deserialize
    let deserialized: Document = serde_yaml::from_str(&yaml).expect("YAML deserialization failed");

    // Verify all fields preserved
    assert_eq!(deserialized.markdown, doc.markdown);
    assert_eq!(deserialized.format, doc.format);
    assert_eq!(deserialized.metadata.author, doc.metadata.author);
}

/// Test JSON compact vs pretty output
#[test]
fn test_json_compact_vs_pretty() {
    let doc = Document::from_markdown("# Test".to_string(), InputFormat::Md);

    // Pretty output
    let pretty_serializer = JsonSerializer::new();
    let pretty_json = pretty_serializer
        .serialize_document(&doc)
        .expect("Pretty JSON failed");

    // Compact output
    let compact_serializer = JsonSerializer::with_options(docling_core::JsonOptions {
        pretty: false,
        indent: "  ".to_string(),
    });
    let compact_json = compact_serializer
        .serialize_document(&doc)
        .expect("Compact JSON failed");

    // Pretty should have more whitespace
    assert!(pretty_json.len() > compact_json.len());

    // Both should deserialize to same document
    let pretty_doc: Document = serde_json::from_str(&pretty_json).unwrap();
    let compact_doc: Document = serde_json::from_str(&compact_json).unwrap();
    assert_eq!(pretty_doc.markdown, compact_doc.markdown);
}

/// Test serialization with empty metadata
#[test]
fn test_serializers_with_minimal_document() {
    let doc = Document::from_markdown("# Minimal".to_string(), InputFormat::Pdf);

    // JSON should work
    let json_serializer = JsonSerializer::new();
    let json = json_serializer.serialize_document(&doc).unwrap();
    assert!(json.contains("Minimal"));

    // YAML should work
    let yaml_serializer = YamlSerializer::new();
    let yaml = yaml_serializer.serialize_document(&doc).unwrap();
    assert!(yaml.contains("Minimal"));
}

/// Test serialization with very long markdown content
#[test]
fn test_serializers_with_large_content() {
    // Create a large markdown document
    let mut markdown = String::from("# Large Document\n\n");
    for i in 0..100 {
        markdown.push_str(&format!("## Section {i}\n\n"));
        markdown.push_str(&format!("This is section {i} content. ").repeat(50));
        markdown.push_str("\n\n");
    }

    let doc = Document::from_markdown(markdown.clone(), InputFormat::Pdf);

    // JSON serialization should handle large content
    let json_serializer = JsonSerializer::new();
    let json = json_serializer
        .serialize_document(&doc)
        .expect("Large JSON serialization failed");
    assert!(json.len() > 10000); // Should be large

    // YAML serialization should handle large content
    let yaml_serializer = YamlSerializer::new();
    let yaml = yaml_serializer
        .serialize_document(&doc)
        .expect("Large YAML serialization failed");
    assert!(yaml.len() > 10000); // Should be large

    // Should deserialize correctly
    let deserialized: Document = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.markdown, markdown);
}

/// Test serialization with special characters and unicode
#[test]
fn test_serializers_with_special_characters() {
    let markdown = "# Special Characters\n\n\
        Unicode: 你好世界 🌍 🚀\n\
        Quotes: \"double\" 'single'\n\
        Symbols: & < > © ® ™\n\
        Math: ∑ ∫ √ ∞\n\
        Emoji: 😀 👍 ❤️";

    let doc = Document::from_markdown(markdown.to_string(), InputFormat::Html);

    // JSON should escape properly
    let json_serializer = JsonSerializer::new();
    let json = json_serializer
        .serialize_document(&doc)
        .expect("Special chars JSON failed");

    // Should deserialize correctly preserving unicode
    let deserialized: Document = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.markdown, markdown);

    // YAML should handle unicode
    let yaml_serializer = YamlSerializer::new();
    let yaml = yaml_serializer
        .serialize_document(&doc)
        .expect("Special chars YAML failed");

    let yaml_deserialized: Document = serde_yaml::from_str(&yaml).unwrap();
    assert_eq!(yaml_deserialized.markdown, markdown);
}

/// Test serialization with all supported input formats
#[test]
fn test_serializers_all_formats() {
    let formats = vec![
        InputFormat::Pdf,
        InputFormat::Docx,
        InputFormat::Html,
        InputFormat::Md,
        InputFormat::Csv,
        InputFormat::Xlsx,
        InputFormat::Pptx,
    ];

    let json_serializer = JsonSerializer::new();
    let yaml_serializer = YamlSerializer::new();

    for format in formats {
        let doc = Document::from_markdown(format!("# Test {format:?}"), format);

        // Both serializers should work for all formats
        let json = json_serializer.serialize_document(&doc).unwrap();
        let yaml = yaml_serializer.serialize_document(&doc).unwrap();

        // Verify format is serialized correctly
        assert!(json.contains(&format!("{format:?}").to_uppercase()));
        assert!(yaml.contains(&format!("{format:?}").to_uppercase()));
    }
}
