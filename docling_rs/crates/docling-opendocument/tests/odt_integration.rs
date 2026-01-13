//! Integration tests for ODT parsing

use docling_opendocument::odt::parse_odt_file;
use std::path::Path;

#[test]
fn test_parse_simple_text() {
    let path = Path::new("../../test-corpus/opendocument/odt/simple_text.odt");
    let doc = parse_odt_file(path).expect("Failed to parse simple_text.odt");

    assert!(
        doc.text.contains("Simple Document"),
        "Should contain heading"
    );
    assert!(
        doc.text.contains("This is a simple ODT document"),
        "Should contain text"
    );
    assert_eq!(doc.paragraph_count, 3); // 1 heading + 2 paragraphs
}

#[test]
fn test_parse_multi_paragraph() {
    let path = Path::new("../../test-corpus/opendocument/odt/multi_paragraph.odt");
    let doc = parse_odt_file(path).expect("Failed to parse multi_paragraph.odt");

    assert!(doc.paragraph_count > 0, "Should have paragraphs");
    assert!(!doc.text.is_empty(), "Should have text content");
}

#[test]
fn test_parse_meeting_notes() {
    let path = Path::new("../../test-corpus/opendocument/odt/meeting_notes.odt");
    let doc = parse_odt_file(path).expect("Failed to parse meeting_notes.odt");

    assert!(!doc.text.is_empty(), "Should have text content");
}

#[test]
fn test_parse_report() {
    let path = Path::new("../../test-corpus/opendocument/odt/report.odt");
    let doc = parse_odt_file(path).expect("Failed to parse report.odt");

    assert!(doc.text.contains("Test Report"), "Should contain title");
    assert!(doc.paragraph_count > 0, "Should have paragraphs");
}

#[test]
fn test_parse_technical_spec() {
    let path = Path::new("../../test-corpus/opendocument/odt/technical_spec.odt");
    let doc = parse_odt_file(path).expect("Failed to parse technical_spec.odt");

    assert!(!doc.text.is_empty(), "Should have text content");
}
