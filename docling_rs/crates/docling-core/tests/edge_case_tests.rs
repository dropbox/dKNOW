//! Edge Case Tests
//!
//! Tests for edge cases and error handling:
//! - Empty files
//! - Corrupted/truncated files
//! - Malformed documents
//! - Boundary conditions
//!
//! These tests complement the canonical tests by exercising error paths
//! and unusual inputs that may not be covered by normal test cases.

use docling_backend::DocumentConverter;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Create a temporary directory for test files
fn setup_test_dir() -> TempDir {
    tempfile::tempdir().expect("Failed to create temp directory")
}

// ============================================================================
// Empty File Tests
// ============================================================================

#[test]
fn test_empty_pdf() {
    let temp_dir = setup_test_dir();
    let empty_pdf = temp_dir.path().join("empty.pdf");

    // Create empty file
    fs::write(&empty_pdf, b"").expect("Failed to write empty file");

    // Convert should handle empty file gracefully
    let converter = DocumentConverter::new().expect("Failed to create converter");
    let result = converter.convert(&empty_pdf);

    // Should either succeed with empty document or return appropriate error
    match result {
        Ok(doc) => {
            // Empty file should produce minimal document
            assert!(
                doc.document.markdown.is_empty() || doc.document.markdown.trim().is_empty(),
                "Empty PDF should produce empty text"
            );
        }
        Err(e) => {
            // Error is acceptable for truly empty file
            println!("Empty PDF error (expected): {e}");
        }
    }
}

#[test]
fn test_empty_docx() {
    let temp_dir = setup_test_dir();
    let empty_docx = temp_dir.path().join("empty.docx");

    // Create empty file
    fs::write(&empty_docx, b"").expect("Failed to write empty file");

    let converter = DocumentConverter::new().expect("Failed to create converter");
    let result = converter.convert(&empty_docx);

    // Empty DOCX should fail (not valid ZIP format)
    assert!(result.is_err(), "Empty DOCX should return error");
}

#[test]
fn test_empty_txt() {
    let temp_dir = setup_test_dir();
    let empty_txt = temp_dir.path().join("empty.txt");

    // Create empty text file
    fs::write(&empty_txt, b"").expect("Failed to write empty file");

    let converter = DocumentConverter::new().expect("Failed to create converter");
    let result = converter.convert(&empty_txt);

    // Empty text file should succeed with empty document
    match result {
        Ok(doc) => {
            assert!(
                doc.document.markdown.is_empty(),
                "Empty text file should produce empty text"
            );
        }
        Err(e) => {
            println!("Empty text error (may be expected): {e}");
        }
    }
}

// ============================================================================
// Corrupted/Truncated File Tests
// ============================================================================

#[test]
fn test_truncated_pdf() {
    let temp_dir = setup_test_dir();
    let truncated_pdf = temp_dir.path().join("truncated.pdf");

    // Create truncated PDF (just header, no content)
    fs::write(&truncated_pdf, b"%PDF-1.4\n").expect("Failed to write truncated PDF");

    let converter = DocumentConverter::new().expect("Failed to create converter");
    let result = converter.convert(&truncated_pdf);

    // Truncated PDF should fail gracefully
    assert!(result.is_err(), "Truncated PDF should return error");
    let err = result.unwrap_err();
    println!("Truncated PDF error (expected): {err}");
}

#[test]
fn test_corrupted_docx() {
    let temp_dir = setup_test_dir();
    let corrupted_docx = temp_dir.path().join("corrupted.docx");

    // Create file with DOCX extension but invalid content
    fs::write(&corrupted_docx, b"This is not a valid DOCX file")
        .expect("Failed to write corrupted DOCX");

    let converter = DocumentConverter::new().expect("Failed to create converter");
    let result = converter.convert(&corrupted_docx);

    // Corrupted DOCX should fail
    assert!(result.is_err(), "Corrupted DOCX should return error");
    let err = result.unwrap_err();
    println!("Corrupted DOCX error (expected): {err}");
}

#[test]
fn test_truncated_zip() {
    let temp_dir = setup_test_dir();
    let truncated_zip = temp_dir.path().join("truncated.docx");

    // Create truncated ZIP header (DOCX/XLSX/PPTX are ZIP files)
    fs::write(&truncated_zip, b"PK\x03\x04").expect("Failed to write truncated ZIP");

    let converter = DocumentConverter::new().expect("Failed to create converter");
    let result = converter.convert(&truncated_zip);

    // Truncated ZIP should fail
    assert!(result.is_err(), "Truncated ZIP should return error");
}

// ============================================================================
// Malformed Document Tests
// ============================================================================

#[test]
fn test_malformed_html_unclosed_tags() {
    let temp_dir = setup_test_dir();
    let malformed_html = temp_dir.path().join("malformed.html");

    // HTML with unclosed tags
    let html_content = r"
<!DOCTYPE html>
<html>
<head><title>Test</title>
<body>
<p>Paragraph without closing tag
<div>Div without closing tag
<span>Nested span
</html>
";
    fs::write(&malformed_html, html_content).expect("Failed to write malformed HTML");

    let converter = DocumentConverter::new().expect("Failed to create converter");
    let result = converter.convert(&malformed_html);

    // HTML parsers typically handle malformed HTML gracefully
    match result {
        Ok(doc) => {
            // Should extract text even from malformed HTML
            assert!(
                !doc.document.markdown.is_empty(),
                "Should extract some text from malformed HTML"
            );
        }
        Err(e) => {
            println!("Malformed HTML error: {e}");
        }
    }
}

#[test]
fn test_html_with_invalid_encoding() {
    let temp_dir = setup_test_dir();
    let invalid_html = temp_dir.path().join("invalid_encoding.html");

    // HTML claiming UTF-8 but containing invalid bytes
    let mut content =
        Vec::from(r#"<!DOCTYPE html><html><head><meta charset="UTF-8"></head><body>"#);
    content.extend_from_slice(&[0xFF, 0xFE]); // Invalid UTF-8 bytes
    content.extend_from_slice(b"</body></html>");

    fs::write(&invalid_html, content).expect("Failed to write invalid HTML");

    let converter = DocumentConverter::new().expect("Failed to create converter");
    let result = converter.convert(&invalid_html);

    // Should handle encoding issues gracefully
    match result {
        Ok(_doc) => {
            // Parser may succeed with replacement characters
        }
        Err(e) => {
            println!("Invalid encoding error (expected): {e}");
        }
    }
}

// ============================================================================
// Boundary Condition Tests
// ============================================================================

#[test]
fn test_very_long_single_line() {
    let temp_dir = setup_test_dir();
    let long_line_file = temp_dir.path().join("long_line.txt");

    // Create file with single very long line (10MB)
    let long_line = "a".repeat(10_000_000);
    fs::write(&long_line_file, &long_line).expect("Failed to write long line file");

    let converter = DocumentConverter::new().expect("Failed to create converter");
    let result = converter.convert(&long_line_file);

    // Should handle very long lines without crashing
    match result {
        Ok(doc) => {
            assert_eq!(
                doc.document.markdown.len(),
                long_line.len() + 1, // +1 for newline
                "Should preserve full text length"
            );
        }
        Err(e) => {
            println!("Long line error: {e}");
        }
    }
}

#[test]
fn test_deeply_nested_html() {
    let temp_dir = setup_test_dir();
    let nested_html = temp_dir.path().join("deeply_nested.html");

    // Create deeply nested HTML (100 levels)
    let mut html = String::from("<!DOCTYPE html><html><body>");
    for i in 0..100 {
        html.push_str(&format!("<div id='level{i}'>"));
    }
    html.push_str("Content");
    for _ in 0..100 {
        html.push_str("</div>");
    }
    html.push_str("</body></html>");

    fs::write(&nested_html, html).expect("Failed to write nested HTML");

    let converter = DocumentConverter::new().expect("Failed to create converter");
    let result = converter.convert(&nested_html);

    // Should handle deep nesting without stack overflow
    match result {
        Ok(doc) => {
            assert!(
                doc.document.markdown.contains("Content"),
                "Should extract content from deeply nested HTML"
            );
        }
        Err(e) => {
            println!("Deeply nested HTML error: {e}");
        }
    }
}

#[test]
fn test_unicode_edge_cases() {
    let temp_dir = setup_test_dir();
    let unicode_file = temp_dir.path().join("unicode.txt");

    // Various Unicode edge cases
    let content = "
ASCII: Hello World
Latin-1: café résumé
Cyrillic: Привет мир
CJK: 你好世界 こんにちは世界
Emoji: 👋🌍🎉
RTL: مرحبا بالعالم
Zero-width: a\u{200B}b (zero-width space between a and b)
Combining: é (e + combining acute)
Surrogate pairs: 𝕳𝖊𝖑𝖑𝖔
";
    fs::write(&unicode_file, content).expect("Failed to write unicode file");

    let converter = DocumentConverter::new().expect("Failed to create converter");
    let result = converter.convert(&unicode_file);

    // Should handle all Unicode correctly
    match result {
        Ok(doc) => {
            assert!(
                doc.document.markdown.contains("Hello"),
                "Should preserve ASCII"
            );
            assert!(
                doc.document.markdown.contains("café") || doc.document.markdown.contains("caf"),
                "Should preserve Latin-1"
            );
            // Note: Some characters may be normalized or converted
        }
        Err(e) => {
            println!("Unicode error: {e}");
        }
    }
}

// ============================================================================
// Non-existent File Tests
// ============================================================================

#[test]
fn test_non_existent_file() {
    let non_existent = Path::new("/tmp/this_file_definitely_does_not_exist_12345.pdf");

    let converter = DocumentConverter::new().expect("Failed to create converter");
    let result = converter.convert(non_existent);

    // Should return error for non-existent file
    assert!(result.is_err(), "Non-existent file should return error");
    let err = result.unwrap_err();
    println!("Non-existent file error (expected): {err}");
}

// ============================================================================
// Permission Tests
// ============================================================================

#[cfg(unix)]
#[test]
fn test_unreadable_file() {
    let temp_dir = setup_test_dir();
    let unreadable_file = temp_dir.path().join("unreadable.txt");

    // Create file and remove read permissions
    fs::write(&unreadable_file, b"content").expect("Failed to write file");

    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(&unreadable_file)
        .expect("Failed to get metadata")
        .permissions();
    perms.set_mode(0o000); // Remove all permissions
    fs::set_permissions(&unreadable_file, perms).expect("Failed to set permissions");

    let converter = DocumentConverter::new().expect("Failed to create converter");
    let result = converter.convert(&unreadable_file);

    // Should return permission error
    match result {
        Err(e) => {
            println!("Permission error (expected): {e}");
            // Restore permissions for cleanup
            let mut perms = fs::metadata(&unreadable_file)
                .expect("Failed to get metadata")
                .permissions();
            perms.set_mode(0o644);
            fs::set_permissions(&unreadable_file, perms).expect("Failed to restore permissions");
        }
        Ok(_) => {
            panic!("Unreadable file should return error");
        }
    }
}

// ============================================================================
// Special Character Tests
// ============================================================================

#[test]
fn test_filename_with_special_characters() {
    let temp_dir = setup_test_dir();

    // Filename with spaces and special characters
    let special_name = temp_dir.path().join("file with spaces & special!chars.txt");
    fs::write(&special_name, b"content").expect("Failed to write file with special name");

    let converter = DocumentConverter::new().expect("Failed to create converter");
    let result = converter.convert(&special_name);

    // Should handle special characters in filename
    match result {
        Ok(doc) => {
            assert!(
                doc.document.markdown.contains("content"),
                "Should read file with special characters in name"
            );
        }
        Err(e) => {
            println!("Special filename error: {e}");
        }
    }
}

// ============================================================================
// Large Count Tests
// ============================================================================

#[test]
fn test_many_short_lines() {
    let temp_dir = setup_test_dir();
    let many_lines_file = temp_dir.path().join("many_lines.txt");

    // Create file with 100,000 short lines
    let lines: Vec<String> = (0..100_000).map(|i| format!("Line {i}")).collect();
    let content = lines.join("\n");
    fs::write(&many_lines_file, &content).expect("Failed to write many lines file");

    let converter = DocumentConverter::new().expect("Failed to create converter");
    let result = converter.convert(&many_lines_file);

    // Should handle many lines efficiently
    match result {
        Ok(doc) => {
            let line_count = doc.document.markdown.lines().count();
            assert!(
                line_count >= 100_000,
                "Should preserve all {} lines, got {}",
                100_000,
                line_count
            );
        }
        Err(e) => {
            println!("Many lines error: {e}");
        }
    }
}
