//! JSON Round-Trip Example
//!
//! Demonstrates how to use the JSON backend for round-trip workflows:
//! 1. Convert a document to get structured output
//! 2. Serialize to JSON
//! 3. Load JSON back into a Document
//! 4. Verify integrity
//!
//! This is useful for:
//! - Testing document processing pipelines
//! - Saving/loading intermediate results
//! - Interoperability between Python and Rust docling
//! - Debugging document structure

use docling_backend::RustDocumentConverter;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== JSON Round-Trip Example ===\n");

    // Step 1: Convert a markdown document
    println!("Step 1: Converting markdown document...");
    let markdown_content = r"# Hello World

This is a test document with **bold** and *italic* text.

## Section 1

- Item 1
- Item 2
- Item 3

## Section 2

Some more content here.
";

    // Create a temporary markdown file
    let temp_dir = std::env::temp_dir();
    let md_path = temp_dir.join("test_roundtrip.md");
    fs::write(&md_path, markdown_content)?;

    // Convert using Rust backend (not Python)
    let converter = RustDocumentConverter::new()?;
    let result = converter.convert(&md_path)?;
    let original_doc = result.document;

    println!(
        "  Original markdown length: {} chars",
        original_doc.markdown.len()
    );
    println!("  Original format: {:?}", original_doc.format);
    println!(
        "  Has structured content: {}",
        original_doc.has_structured_content()
    );
    if let Some(blocks) = original_doc.blocks() {
        println!("  Content blocks: {}", blocks.len());
    }

    // Step 2: Serialize to JSON
    println!("\nStep 2: Serializing to JSON...");
    let json_string = serde_json::to_string_pretty(&original_doc)?;
    println!("  JSON size: {} bytes", json_string.len());

    // Save JSON to file
    let json_path = temp_dir.join("test_roundtrip.json");
    fs::write(&json_path, &json_string)?;
    println!("  Saved to: {}", json_path.display());

    // Step 3: Load JSON back into Document
    println!("\nStep 3: Loading JSON back into Document...");
    let loaded_result = converter.convert(&json_path)?;
    let loaded_doc = loaded_result.document;

    println!(
        "  Loaded markdown length: {} chars",
        loaded_doc.markdown.len()
    );
    println!("  Loaded format: {:?}", loaded_doc.format);
    println!(
        "  Has structured content: {}",
        loaded_doc.has_structured_content()
    );
    if let Some(blocks) = loaded_doc.blocks() {
        println!("  Content blocks: {}", blocks.len());
    }

    // Step 4: Verify integrity
    println!("\nStep 4: Verifying round-trip integrity...");
    let markdown_matches = original_doc.markdown == loaded_doc.markdown;
    let format_matches = original_doc.format == loaded_doc.format; // Original format is preserved
    let chars_match = original_doc.metadata.num_characters == loaded_doc.metadata.num_characters;

    println!(
        "  Markdown content matches: {}",
        if markdown_matches { "✓" } else { "✗" }
    );
    println!(
        "  Original format preserved: {}",
        if format_matches { "✓" } else { "✗" }
    );
    println!(
        "  Character count matches: {}",
        if chars_match { "✓" } else { "✗" }
    );

    // Check structured content
    let blocks_match = match (original_doc.blocks(), loaded_doc.blocks()) {
        (Some(orig), Some(loaded)) => orig.len() == loaded.len(),
        (None, None) => true,
        _ => false,
    };
    println!(
        "  Structured content matches: {}",
        if blocks_match { "✓" } else { "✗" }
    );

    if markdown_matches && format_matches && chars_match && blocks_match {
        println!("\n✓ Round-trip successful! Document integrity preserved.");
    } else {
        println!("\n✗ Round-trip failed. Some data was lost or changed.");
    }

    // Clean up
    fs::remove_file(&md_path)?;
    fs::remove_file(&json_path)?;

    println!("\n=== Example Complete ===");
    Ok(())
}
