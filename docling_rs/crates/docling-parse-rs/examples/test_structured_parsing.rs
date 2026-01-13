//! Test structured parsing with docling-parse
//!
//! This example demonstrates parsing a PDF and converting to SegmentedPdfPage.

use anyhow::Result;
use docling_parse_rs::DoclingParser;
use std::path::PathBuf;

fn main() -> Result<()> {
    // Initialize parser
    let mut parser = DoclingParser::new("error")?;

    // Test file path
    let test_file = PathBuf::from("test-corpus/pdf/code_and_formula.pdf");

    if !test_file.exists() {
        eprintln!("Test file not found: {:?}", test_file);
        eprintln!("Please run from the docling_rs root directory");
        return Ok(());
    }

    println!("Loading document: {:?}", test_file);
    parser.load_document("test_doc", &test_file, None)?;

    let num_pages = parser.number_of_pages("test_doc")?;
    println!("Document has {} pages", num_pages);

    // Parse first page with structured output
    println!("\nParsing page 0 with structured output...");
    let segmented_page = parser.parse_page_structured("test_doc", 0)?;

    println!("\n=== Page Geometry ===");
    println!("Width: {:.2}", segmented_page.dimension.width());
    println!("Height: {:.2}", segmented_page.dimension.height());
    println!("Angle: {:.2}", segmented_page.dimension.angle);
    println!(
        "Boundary type: {:?}",
        segmented_page.dimension.boundary_type
    );

    println!("\n=== Text Cells ===");
    println!("Line cells: {}", segmented_page.textline_cells.len());
    println!("Word cells: {}", segmented_page.word_cells.len());
    println!("Char cells: {}", segmented_page.char_cells.len());
    println!("Has lines: {}", segmented_page.has_textlines);
    println!("Has words: {}", segmented_page.has_words);
    println!("Has chars: {}", segmented_page.has_chars);

    // Show first few line cells
    println!("\n=== First 5 Line Cells ===");
    for (i, cell) in segmented_page.textline_cells.iter().take(5).enumerate() {
        println!(
            "Cell {}: '{}' (bbox: {:.1}, {:.1}, {:.1}, {:.1})",
            i,
            cell.text.chars().take(50).collect::<String>(),
            cell.rect.r_x0,
            cell.rect.r_y0,
            cell.rect.r_x2,
            cell.rect.r_y2
        );
    }

    // Calculate total text length
    let total_text: String = segmented_page
        .textline_cells
        .iter()
        .map(|c| c.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    println!("\nTotal text length: {} chars", total_text.len());
    println!(
        "First 200 chars: {}",
        total_text.chars().take(200).collect::<String>()
    );

    // Unload document
    parser.unload_document("test_doc")?;
    println!("\n✓ Test completed successfully!");

    Ok(())
}
