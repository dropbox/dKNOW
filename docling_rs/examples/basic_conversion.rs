//! Basic Document Conversion Example
//!
//! This example demonstrates the simplest usage of docling-core:
//! converting a single document to markdown format.
//!
//! Run with:
//! ```bash
//! cargo run --example basic_conversion -- path/to/document.pdf
//! ```

use docling_backend::DocumentConverter;
use docling_core::Result;
use std::env;

fn main() -> Result<()> {
    // Get input file from command-line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <path-to-document>", args[0]);
        eprintln!("\nExample:");
        eprintln!("  {} document.pdf", args[0]);
        eprintln!("  {} report.docx", args[0]);
        eprintln!("  {} page.html", args[0]);
        std::process::exit(1);
    }

    let input_path = &args[1];

    println!("Converting document: {input_path}");
    println!();

    // Create a converter with default settings (text-only mode)
    // This is the fastest mode but won't work for scanned documents
    let converter = DocumentConverter::new()?;

    // Convert the document
    let result = converter.convert(input_path)?;

    // Display conversion results
    println!("✓ Conversion successful!");
    println!();
    println!("Document Metadata:");
    println!("  Pages: {:?}", result.document.metadata.num_pages);
    println!(
        "  Characters: {:?}",
        result.document.metadata.num_characters
    );
    println!("  Conversion time: {:?}", result.latency);
    println!();

    // Display the markdown output
    println!("Markdown Output:");
    println!("{}", "=".repeat(80));
    println!("{}", result.document.markdown);
    println!("{}", "=".repeat(80));

    Ok(())
}
