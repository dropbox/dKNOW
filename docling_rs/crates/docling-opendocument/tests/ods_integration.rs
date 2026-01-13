//! Integration tests for ODS (`OpenDocument` Spreadsheet) parser
//!
//! Tests the complete ODS parsing pipeline with real-world test files.

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

/// Test parsing `simple_spreadsheet.ods`
#[test]
fn test_parse_simple_spreadsheet() {
    let path = test_corpus_dir().join("simple_spreadsheet.ods");
    assert!(path.exists(), "Test file not found: {path:?}");

    let doc = parse_ods_file(&path).expect("Failed to parse simple_spreadsheet.ods");

    // Basic validation
    assert!(!doc.text.is_empty(), "Document text should not be empty");
    assert!(doc.sheet_count > 0, "Should have at least one sheet");
    assert!(
        doc.cell_count > 0,
        "Should have at least one cell with data"
    );

    // Verify structure
    assert!(
        doc.text.contains("## Sheet:"),
        "Should contain sheet header"
    );

    println!("simple_spreadsheet.ods parsed successfully:");
    println!("  Sheets: {}", doc.sheet_count);
    println!("  Cells: {}", doc.cell_count);
    println!("  Rows: {}", doc.row_count);
    println!("  Sheet names: {:?}", doc.sheet_names);
}

/// Test parsing `multi_sheet.ods`
#[test]
fn test_parse_multi_sheet() {
    let path = test_corpus_dir().join("multi_sheet.ods");
    assert!(path.exists(), "Test file not found: {path:?}");

    let doc = parse_ods_file(&path).expect("Failed to parse multi_sheet.ods");

    // Multi-sheet validation
    assert!(!doc.text.is_empty(), "Document text should not be empty");
    assert!(doc.sheet_count >= 2, "Should have at least two sheets");
    assert!(doc.cell_count > 0, "Should have cells with data");

    // Verify multiple sheet headers
    let sheet_header_count = doc.text.matches("## Sheet:").count();
    assert_eq!(
        sheet_header_count, doc.sheet_count,
        "Should have header for each sheet"
    );

    println!("multi_sheet.ods parsed successfully:");
    println!("  Sheets: {}", doc.sheet_count);
    println!("  Cells: {}", doc.cell_count);
    println!("  Rows: {}", doc.row_count);
    println!("  Sheet names: {:?}", doc.sheet_names);
}

/// Test parsing `budget.ods`
#[test]
fn test_parse_budget() {
    let path = test_corpus_dir().join("budget.ods");
    assert!(path.exists(), "Test file not found: {path:?}");

    let doc = parse_ods_file(&path).expect("Failed to parse budget.ods");

    // Budget spreadsheet validation
    assert!(!doc.text.is_empty(), "Document text should not be empty");
    assert!(doc.sheet_count > 0, "Should have at least one sheet");
    assert!(doc.cell_count > 0, "Should have cells with data");

    println!("budget.ods parsed successfully:");
    println!("  Sheets: {}", doc.sheet_count);
    println!("  Cells: {}", doc.cell_count);
    println!("  Rows: {}", doc.row_count);
    println!("  Sheet names: {:?}", doc.sheet_names);
}

/// Test parsing `inventory.ods`
#[test]
fn test_parse_inventory() {
    let path = test_corpus_dir().join("inventory.ods");
    assert!(path.exists(), "Test file not found: {path:?}");

    let doc = parse_ods_file(&path).expect("Failed to parse inventory.ods");

    // Inventory spreadsheet validation
    assert!(!doc.text.is_empty(), "Document text should not be empty");
    assert!(doc.sheet_count > 0, "Should have at least one sheet");
    assert!(doc.cell_count > 0, "Should have cells with data");

    println!("inventory.ods parsed successfully:");
    println!("  Sheets: {}", doc.sheet_count);
    println!("  Cells: {}", doc.cell_count);
    println!("  Rows: {}", doc.row_count);
    println!("  Sheet names: {:?}", doc.sheet_names);
}

/// Test parsing `test_data.ods`
#[test]
fn test_parse_test_data() {
    let path = test_corpus_dir().join("test_data.ods");
    assert!(path.exists(), "Test file not found: {path:?}");

    let doc = parse_ods_file(&path).expect("Failed to parse test_data.ods");

    // Test data spreadsheet validation
    assert!(!doc.text.is_empty(), "Document text should not be empty");
    assert!(doc.sheet_count > 0, "Should have at least one sheet");
    assert!(doc.cell_count > 0, "Should have cells with data");

    println!("test_data.ods parsed successfully:");
    println!("  Sheets: {}", doc.sheet_count);
    println!("  Cells: {}", doc.cell_count);
    println!("  Rows: {}", doc.row_count);
    println!("  Sheet names: {:?}", doc.sheet_names);
}
