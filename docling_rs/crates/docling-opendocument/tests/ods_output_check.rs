//! Output verification tests for ODS parser
//!
//! These tests verify the quality of the markdown output by examining
//! the actual text content generated from ODS files.

use docling_opendocument::ods::parse_ods_file;
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
        .join("ods")
}

/// Test that `simple_spreadsheet.ods` produces well-formatted output
#[test]
fn test_simple_spreadsheet_output() {
    let path = test_corpus_dir().join("simple_spreadsheet.ods");
    let doc = parse_ods_file(&path).expect("Failed to parse simple_spreadsheet.ods");

    println!("=== simple_spreadsheet.ods output ===");
    println!("{}", doc.text);
    println!("=== end output ===");

    // Verify sheet header format
    assert!(
        doc.text.contains("## Sheet: Sheet1"),
        "Should have properly formatted sheet header"
    );

    // Verify content is present
    assert!(
        doc.text.len() > 30,
        "Output should have substantial content"
    );

    // Verify pipe-delimited row format
    assert!(
        doc.text.contains(" | "),
        "Should use pipe delimiters between cells"
    );
}

/// Test that `multi_sheet.ods` produces multiple sheet sections
#[test]
fn test_multi_sheet_output() {
    let path = test_corpus_dir().join("multi_sheet.ods");
    let doc = parse_ods_file(&path).expect("Failed to parse multi_sheet.ods");

    println!("=== multi_sheet.ods output ===");
    println!("{}", doc.text);
    println!("=== end output ===");

    // Verify both sheets are present
    assert!(
        doc.text.contains("## Sheet: Sales"),
        "Should have Sales sheet"
    );
    assert!(
        doc.text.contains("## Sheet: Expenses"),
        "Should have Expenses sheet"
    );

    // Verify sheets are properly separated
    let sheet_count = doc.text.split("## Sheet:").count();
    assert_eq!(sheet_count, 3, "Should have 3 parts (empty + 2 sheets)");

    // Verify content is present
    assert!(
        doc.text.len() > 100,
        "Output should have substantial content"
    );
}

/// Test that budget.ods produces well-formatted financial data
#[test]
fn test_budget_output_format() {
    let path = test_corpus_dir().join("budget.ods");
    let doc = parse_ods_file(&path).expect("Failed to parse budget.ods");

    println!("=== budget.ods output ===");
    println!("{}", doc.text);
    println!("=== end output ===");

    // Verify sheet header
    assert!(doc.text.contains("## Sheet:"), "Should have sheet header");

    // Verify content structure
    assert!(doc.text.len() > 50, "Output should have content");
    assert!(doc.text.contains(" | "), "Should use pipe delimiters");
}
