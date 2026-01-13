//! Output verification tests for ODP parser
//!
//! These tests verify the quality of the markdown output by examining
//! the actual text content generated from ODP files.

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

/// Test that `simple_presentation.odp` produces well-formatted output
#[test]
fn test_simple_presentation_output() {
    let path = test_corpus_dir().join("simple_presentation.odp");
    let doc = parse_odp_file(&path).expect("Failed to parse simple_presentation.odp");

    println!("=== simple_presentation.odp output ===");
    println!("{}", doc.text);
    println!("=== end output ===");

    // Verify slide markers are present
    assert!(
        doc.text.contains("## Slide 1"),
        "Should have first slide marker"
    );

    // Verify content is present
    assert!(
        doc.text.len() > 30,
        "Output should have substantial content"
    );

    // Verify proper slide separation (should have newlines between slides)
    if doc.slide_count > 1 {
        assert!(
            doc.text.contains("## Slide 2"),
            "Should have second slide marker"
        );
    }
}

/// Test that `project_overview.odp` produces multiple slides
#[test]
fn test_project_overview_output() {
    let path = test_corpus_dir().join("project_overview.odp");
    let doc = parse_odp_file(&path).expect("Failed to parse project_overview.odp");

    println!("=== project_overview.odp output ===");
    println!("{}", doc.text);
    println!("=== end output ===");

    // Verify multiple slides
    assert!(doc.text.contains("## Slide 1"), "Should have first slide");

    // Verify content structure
    assert!(
        doc.text.len() > 50,
        "Output should have substantial content"
    );

    // Count slide markers
    let slide_marker_count = doc.text.matches("## Slide").count();
    assert_eq!(
        slide_marker_count, doc.slide_count,
        "Slide marker count should match slide count"
    );
}

/// Test that training.odp produces well-structured output
#[test]
fn test_training_output_format() {
    let path = test_corpus_dir().join("training.odp");
    let doc = parse_odp_file(&path).expect("Failed to parse training.odp");

    println!("=== training.odp output ===");
    println!("{}", doc.text);
    println!("=== end output ===");

    // Verify slide markers
    assert!(doc.text.contains("## Slide"), "Should have slide markers");

    // Verify content structure
    assert!(doc.text.len() > 50, "Output should have content");

    // Verify we have multiple slides (training usually has many)
    assert!(
        doc.slide_count >= 3,
        "Training should have at least 3 slides"
    );
}
