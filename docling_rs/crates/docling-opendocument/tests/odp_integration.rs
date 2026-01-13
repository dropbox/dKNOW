//! Integration tests for ODP (`OpenDocument` Presentation) parser
//!
//! Tests the complete ODP parsing pipeline with real-world test files.

use docling_opendocument::odp::parse_odp_file;
use std::path::PathBuf;

/// Get path to test corpus directory
fn test_corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test-corpus")
        .join("opendocument")
        .join("odp")
}

/// Test parsing `simple_presentation.odp`
#[test]
fn test_parse_simple_presentation() {
    let path = test_corpus_dir().join("simple_presentation.odp");
    assert!(path.exists(), "Test file not found: {path:?}");

    let doc = parse_odp_file(&path).expect("Failed to parse simple_presentation.odp");

    // Basic validation
    assert!(!doc.text.is_empty(), "Document text should not be empty");
    assert!(doc.slide_count > 0, "Should have at least one slide");

    // Verify structure
    assert!(
        doc.text.contains("## Slide"),
        "Should contain slide markers"
    );

    println!("simple_presentation.odp parsed successfully:");
    println!("  Slides: {}", doc.slide_count);
    println!("  Title: {:?}", doc.title);
    println!("  Author: {:?}", doc.author);
}

/// Test parsing `project_overview.odp`
#[test]
fn test_parse_project_overview() {
    let path = test_corpus_dir().join("project_overview.odp");
    assert!(path.exists(), "Test file not found: {path:?}");

    let doc = parse_odp_file(&path).expect("Failed to parse project_overview.odp");

    // Project overview validation
    assert!(!doc.text.is_empty(), "Document text should not be empty");
    assert!(doc.slide_count > 0, "Should have at least one slide");

    println!("project_overview.odp parsed successfully:");
    println!("  Slides: {}", doc.slide_count);
    println!("  Title: {:?}", doc.title);
    println!("  Author: {:?}", doc.author);
}

/// Test parsing `sales_pitch.odp`
#[test]
fn test_parse_sales_pitch() {
    let path = test_corpus_dir().join("sales_pitch.odp");
    assert!(path.exists(), "Test file not found: {path:?}");

    let doc = parse_odp_file(&path).expect("Failed to parse sales_pitch.odp");

    // Sales pitch validation
    assert!(!doc.text.is_empty(), "Document text should not be empty");
    assert!(doc.slide_count > 0, "Should have at least one slide");

    println!("sales_pitch.odp parsed successfully:");
    println!("  Slides: {}", doc.slide_count);
    println!("  Title: {:?}", doc.title);
    println!("  Author: {:?}", doc.author);
}

/// Test parsing `technical_talk.odp`
#[test]
fn test_parse_technical_talk() {
    let path = test_corpus_dir().join("technical_talk.odp");
    assert!(path.exists(), "Test file not found: {path:?}");

    let doc = parse_odp_file(&path).expect("Failed to parse technical_talk.odp");

    // Technical talk validation
    assert!(!doc.text.is_empty(), "Document text should not be empty");
    assert!(doc.slide_count > 0, "Should have at least one slide");

    println!("technical_talk.odp parsed successfully:");
    println!("  Slides: {}", doc.slide_count);
    println!("  Title: {:?}", doc.title);
    println!("  Author: {:?}", doc.author);
}

/// Test parsing training.odp
#[test]
fn test_parse_training() {
    let path = test_corpus_dir().join("training.odp");
    assert!(path.exists(), "Test file not found: {path:?}");

    let doc = parse_odp_file(&path).expect("Failed to parse training.odp");

    // Training presentation validation
    assert!(!doc.text.is_empty(), "Document text should not be empty");
    assert!(doc.slide_count > 0, "Should have at least one slide");

    println!("training.odp parsed successfully:");
    println!("  Slides: {}", doc.slide_count);
    println!("  Title: {:?}", doc.title);
    println!("  Author: {:?}", doc.author);
}
