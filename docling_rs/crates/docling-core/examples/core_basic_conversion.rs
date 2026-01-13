//! Basic document conversion example
//!
//! This example demonstrates the simplest use case:
//! Convert a single document to markdown.
//!
//! Usage:
//! ```bash
//! cargo run --example basic_conversion -- path/to/document.pdf
//! ```

use docling_backend::DocumentConverter;
use docling_core::DoclingError;
use std::env;

fn main() -> Result<(), DoclingError> {
    // Get file path from command line
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <file_path>", args[0]);
        eprintln!("Example: {} document.pdf", args[0]);
        std::process::exit(1);
    }
    let file_path = &args[1];

    println!("Converting: {file_path}");

    // Create converter
    let converter = DocumentConverter::new()?;

    // Convert document
    let result = converter.convert(file_path)?;

    // Print markdown output
    println!("\n=== Markdown Output ===\n");
    println!("{}", result.document.markdown);

    // Print metadata
    println!("\n=== Metadata ===");
    println!("Format: {:?}", result.document.format);
    if let Some(num_pages) = result.document.metadata.num_pages {
        println!("Pages: {num_pages}");
    }
    println!("Characters: {}", result.document.metadata.num_characters);
    if let Some(title) = &result.document.metadata.title {
        println!("Title: {title}");
    }

    // Print performance info
    println!("\n=== Performance ===");
    println!("Conversion time: {:?}", result.latency);

    Ok(())
}
