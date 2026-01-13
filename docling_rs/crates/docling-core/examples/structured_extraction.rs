//! Structured content extraction example
//!
//! This example demonstrates extracting structured content (DocItems)
//! from documents, allowing you to process headings, tables, lists, etc.
//! separately from raw markdown.
//!
//! Usage:
//! ```bash
//! cargo run --example structured_extraction -- path/to/document.docx
//! ```

use docling_backend::DocumentConverter;
use docling_core::{DocItem, DoclingError};
use std::env;

fn analyze_doc_items(doc_items: &[DocItem]) {
    let mut heading_count = 0;
    let mut text_count = 0;
    let mut table_count = 0;
    let mut list_count = 0;
    let mut picture_count = 0;

    println!("\n=== Document Structure ===\n");

    for (idx, item) in doc_items.iter().enumerate() {
        match item {
            DocItem::SectionHeader { text, level, .. } => {
                heading_count += 1;
                println!(
                    "{}. Heading (level {}): {}",
                    idx + 1,
                    level,
                    text.chars().take(60).collect::<String>()
                );
            }
            DocItem::Text { text, .. } => {
                text_count += 1;
                if idx < 10 {
                    // Only print first 10 text items
                    println!(
                        "{}. Text: {}...",
                        idx + 1,
                        text.chars().take(60).collect::<String>()
                    );
                }
            }
            DocItem::Table { data, .. } => {
                table_count += 1;
                println!(
                    "{}. Table: {} rows × {} cols",
                    idx + 1,
                    data.num_rows,
                    data.num_cols
                );
                if let Some(first_row) = data.grid.first() {
                    if let Some(first_cell) = first_row.first() {
                        println!(
                            "   First cell: {}",
                            first_cell.text.chars().take(40).collect::<String>()
                        );
                    }
                }
            }
            DocItem::ListItem { text, marker, .. } => {
                list_count += 1;
                if idx < 10 {
                    // Only print first 10 list items
                    println!(
                        "{}. List Item ({}): {}...",
                        idx + 1,
                        marker,
                        text.chars().take(50).collect::<String>()
                    );
                }
            }
            DocItem::Picture { .. } => {
                picture_count += 1;
                println!("{}. Picture/Image", idx + 1);
            }
            _ => {
                // Other item types
            }
        }
    }

    println!("\n=== Statistics ===");
    println!("Total items: {}", doc_items.len());
    println!("Headings: {heading_count}");
    println!("Text blocks: {text_count}");
    println!("Tables: {table_count}");
    println!("List items: {list_count}");
    println!("Pictures: {picture_count}");
}

fn main() -> Result<(), DoclingError> {
    // Get file path from command line
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <file_path>", args[0]);
        eprintln!("Example: {} document.docx", args[0]);
        std::process::exit(1);
    }
    let file_path = &args[1];

    println!("Extracting structured content from: {file_path}");

    // Create converter
    let converter = DocumentConverter::new()?;

    // Convert document
    let result = converter.convert(file_path)?;

    // Check if document has structured content
    if let Some(doc_items) = &result.document.content_blocks {
        analyze_doc_items(doc_items);

        // Example: Extract all headings
        println!("\n=== All Headings ===");
        let headings: Vec<(usize, &str)> = doc_items
            .iter()
            .filter_map(|item| match item {
                DocItem::SectionHeader { text, level, .. } => Some((*level, text.as_str())),
                _ => None,
            })
            .collect();

        for (level, text) in headings {
            println!("{}{}", "  ".repeat(level), text);
        }

        // Example: Extract all tables
        println!("\n=== All Tables ===");
        let table_count = doc_items
            .iter()
            .filter(|item| matches!(item, DocItem::Table { .. }))
            .count();
        println!("Found {table_count} tables");
    } else {
        println!("\nNote: This format doesn't support structured extraction yet.");
        println!("Only markdown output is available.");
        println!("\nMarkdown output (first 500 chars):");
        println!(
            "{}",
            result
                .document
                .markdown
                .chars()
                .take(500)
                .collect::<String>()
        );
    }

    Ok(())
}
