//! Test for word break fix (BUG #33)
//!
//! Verifies that words are NOT broken mid-word due to PDF cell boundaries.

#[cfg(feature = "pdf")]
use docling_backend::pdfium_adapter::PdfiumFast;

/// Quick test that cell extraction doesn't produce broken words
/// This tests the cell extraction without loading ML models (much faster)
#[cfg(feature = "pdf")]
#[test]
fn test_word_breaks_in_cells() -> Result<(), Box<dyn std::error::Error>> {
    let pdf_path = std::path::Path::new("../../test-corpus/pdf/multi_page.pdf");

    println!("\n=== WORD BREAK FIX TEST (Cell Extraction) ===\n");

    let pdfium = PdfiumFast::new()?;
    let doc = pdfium.load_pdf_from_file(pdf_path, None)?;

    // Extract cells from first page
    let page = doc.load_page(0)?;
    let page_height = page.height() as f64;
    let cells = page.extract_text_cells(page_height)?;

    // Join all cell text with spaces
    let text: String = cells
        .iter()
        .map(|c| c.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");

    println!(
        "Extracted {} cells, {} chars total",
        cells.len(),
        text.len()
    );

    // Check for broken words
    let broken_patterns = ["professi onal", "spee d"];

    let mut issues = Vec::new();

    for pattern in broken_patterns {
        if text.contains(pattern) {
            issues.push(format!("Found broken word: '{}'", pattern));
        }
    }

    // The "fi elds" case is on page 2, check that too
    let page2 = doc.load_page(2)?;
    let page2_height = page2.height() as f64;
    let cells2 = page2.extract_text_cells(page2_height)?;
    let text2: String = cells2
        .iter()
        .map(|c| c.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");

    if text2.contains("fi elds") || text2.contains(" fi elds") {
        issues.push("Found broken word: 'fi elds'".to_string());
    }

    if !issues.is_empty() {
        println!("\n❌ WORD BREAK ISSUES FOUND:");
        for issue in &issues {
            println!("  - {}", issue);
        }
        panic!("Word breaks not fixed: {:?}", issues);
    }

    println!("✓ No broken words found!");

    Ok(())
}

/// Test that actual ligature cases are preserved correctly
#[cfg(feature = "pdf")]
#[test]
#[ignore = "Need a PDF with actual ffi ligatures to test"]
fn test_ligature_preservation() -> Result<(), Box<dyn std::error::Error>> {
    // This test would need a PDF with actual ligature tokens like "ffi"
    // to verify we still preserve those cases correctly
    Ok(())
}
