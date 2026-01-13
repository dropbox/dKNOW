//! Example of using docling-parse-rs to parse a PDF

use docling_parse_rs::{DoclingParser, Result};
use std::env;
use std::path::PathBuf;

fn main() -> Result<()> {
    // Get PDF path from command line
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <pdf_path>", args[0]);
        std::process::exit(1);
    }

    let pdf_path = PathBuf::from(&args[1]);
    if !pdf_path.exists() {
        eprintln!("File not found: {}", pdf_path.display());
        std::process::exit(1);
    }

    println!("Parsing PDF: {}", pdf_path.display());

    // Create parser
    let mut parser = DoclingParser::new("info")?;
    println!("Parser created");

    // Load document
    parser.load_document("test_doc", &pdf_path, None)?;
    println!("Document loaded");

    // Get number of pages
    let num_pages = parser.number_of_pages("test_doc")?;
    println!("Number of pages: {}", num_pages);

    // Parse first page
    if num_pages > 0 {
        println!("\nParsing page 0...");
        let page = parser.parse_page("test_doc", 0)?;

        // Save raw JSON to file for analysis
        if let Some(json) = &page.raw_json {
            std::fs::write("/tmp/docling_parse_page0.json", json).ok();
            println!(
                "Saved {} bytes to /tmp/docling_parse_page0.json",
                json.len()
            );

            // Print preview
            let preview = if json.len() > 500 { &json[..500] } else { json };
            println!("\nJSON preview:\n{}", preview);
            if json.len() > 500 {
                println!("\n... (truncated, see /tmp/docling_parse_page0.json)");
            }
        }

        println!("\nParsed page info:");
        println!("  Page number: {}", page.page);
        println!(
            "  Dimensions: {}x{}",
            page.dimensions.width, page.dimensions.height
        );
        println!("  Number of cells: {}", page.cells.len());

        if !page.cells.is_empty() {
            println!("\nFirst cell:");
            let cell = &page.cells[0];
            println!("  Text: {:?}", cell.text);
            println!(
                "  BBox: ({}, {}, {}, {})",
                cell.bbox.x0, cell.bbox.y0, cell.bbox.x1, cell.bbox.y1
            );
        }
    }

    // Unload document
    parser.unload_document("test_doc")?;
    println!("\nDocument unloaded");

    Ok(())
}
