//! Metadata Extraction Example
//!
//! This example demonstrates how to extract metadata from documents,
//! including page count, character count, format information, and more.
//!
//! Run with:
//! ```bash
//! cargo run --example metadata_extraction -- path/to/document.pdf
//! ```

use docling_backend::DocumentConverter;
use docling_core::Result;
use std::env;
use std::path::Path;

fn main() -> Result<()> {
    // Get input files from command-line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <path-to-documents...>", args[0]);
        eprintln!("\nExample:");
        eprintln!("  {} document.pdf", args[0]);
        eprintln!("  {} doc1.pdf doc2.docx doc3.html", args[0]);
        std::process::exit(1);
    }

    let input_paths: Vec<&str> = args[1..].iter().map(String::as_str).collect();

    println!("Extracting metadata from {} document(s)", input_paths.len());
    println!();

    // Create converter
    let converter = DocumentConverter::new()?;

    // Process each document
    for (i, path) in input_paths.iter().enumerate() {
        println!("=== Document {}/{} ===", i + 1, input_paths.len());
        println!("Path: {path}");
        println!();

        match extract_metadata(&converter, path) {
            Ok(()) => println!("✓ Metadata extracted successfully"),
            Err(e) => eprintln!("✗ Failed to extract metadata: {e}"),
        }

        if i < input_paths.len() - 1 {
            println!();
            println!("{}", "-".repeat(80));
            println!();
        }
    }

    Ok(())
}

/// Extract and display comprehensive metadata from a document
fn extract_metadata(converter: &DocumentConverter, path: &str) -> Result<()> {
    // Convert the document
    let result = converter.convert(path)?;
    let metadata = &result.document.metadata;

    // Display file information
    println!("File Information:");
    println!(
        "  Filename: {}",
        Path::new(path).file_name().unwrap().to_string_lossy()
    );
    if let Some(ext) = Path::new(path).extension() {
        println!("  Extension: {}", ext.to_string_lossy());
    }

    // Display format information
    println!();
    println!("Format Information:");

    // Display content statistics
    println!();
    println!("Content Statistics:");
    if let Some(pages) = metadata.num_pages {
        println!("  Pages: {pages}");
    } else {
        println!("  Pages: N/A (not applicable for this format)");
    }

    let chars = metadata.num_characters;
    println!("  Characters: {chars}");
    if chars > 0 {
        println!("  Estimated words: ~{}", chars / 5); // Rough estimate
        println!("  Estimated reading time: ~{} minutes", (chars / 5) / 200); // 200 wpm
    }

    // Display conversion performance
    println!();
    println!("Conversion Performance:");
    println!("  Conversion time: {:?}", result.latency);
    if chars > 0 {
        let chars_per_sec = chars as f64 / result.latency.as_secs_f64();
        println!("  Speed: {chars_per_sec:.0} characters/second");
    }

    // Display structured content availability
    println!();
    println!("Structured Content:");
    if result.document.has_structured_content() {
        println!("  ✓ Structured content available");

        if let Some(blocks) = result.document.blocks() {
            println!("  Content blocks: {}", blocks.len());

            // Count block types (if available in the future)
            // For now, just show the count
        } else {
            println!("  Content blocks: 0");
        }
    } else {
        println!("  ✗ No structured content (markdown-only output)");
    }

    // Display markdown preview
    println!();
    println!("Content Preview (first 500 characters):");
    let preview = if result.document.markdown.len() > 500 {
        format!("{}...", &result.document.markdown[..500])
    } else {
        result.document.markdown.clone()
    };
    println!("{}", "-".repeat(80));
    println!("{preview}");
    println!("{}", "-".repeat(80));

    // Additional analysis
    println!();
    println!("Content Analysis:");
    let markdown = &result.document.markdown;

    // Count lines
    let line_count = markdown.lines().count();
    println!("  Lines: {line_count}");

    // Count headers
    let header_count = markdown
        .lines()
        .filter(|line| line.starts_with('#'))
        .count();
    println!("  Headers: {header_count}");

    // Count tables (rough estimate)
    let table_count = markdown.matches("| ").count() / 3; // Rough estimate
    if table_count > 0 {
        println!("  Tables: ~{table_count}");
    }

    // Count code blocks
    let code_block_count = markdown.matches("```").count() / 2;
    if code_block_count > 0 {
        println!("  Code blocks: {code_block_count}");
    }

    // Count links
    let link_count = markdown.matches("](").count();
    if link_count > 0 {
        println!("  Links: {link_count}");
    }

    Ok(())
}
