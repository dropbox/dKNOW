//! Custom Serialization Example
//!
//! This example demonstrates how to customize markdown serialization options,
//! including indentation, escaping rules, and formatting preferences.
//!
//! Run with:
//! ```bash
//! cargo run --example custom_serialization -- path/to/document.pdf
//! ```

use docling_backend::DocumentConverter;
use docling_core::{MarkdownOptions, MarkdownSerializer, Result};
use std::env;

fn main() -> Result<()> {
    // Get input file from command-line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <path-to-document>", args[0]);
        eprintln!("\nExample:");
        eprintln!("  {} document.pdf", args[0]);
        std::process::exit(1);
    }

    let input_path = &args[1];

    println!("Converting document with custom serialization options: {input_path}");
    println!();

    // Create converter
    let converter = DocumentConverter::new()?;
    let result = converter.convert(input_path)?;

    println!("✓ Conversion successful!");
    println!();

    // Example 1: Default serialization
    println!("=== Example 1: Default Serialization ===");
    println!("Default options:");
    println!("  - 4-space indentation");
    println!("  - Escape underscores: true");
    println!("  - Escape HTML: true");
    println!();
    display_preview(&result.document.markdown, "Default");
    println!();

    // Example 2: Custom indentation (2 spaces)
    println!("=== Example 2: Custom Indentation (2 spaces) ===");
    let options_2space = MarkdownOptions {
        indent: 2,
        escape_underscores: true,
        escape_html: true,
        include_furniture: false,
        max_list_depth: 10,
        linkify_urls: true,
        insert_page_breaks: false,
    };
    let _serializer_2space = MarkdownSerializer::with_options(options_2space);
    // Note: In the current implementation, markdown is generated during conversion
    // This example shows how to configure options for custom serialization
    println!("Options:");
    println!("  - 2-space indentation");
    println!("  - Escape underscores: true");
    println!("  - Escape HTML: true");
    println!();

    // Example 3: No underscore escaping
    println!("=== Example 3: No Underscore Escaping ===");
    let options_no_escape = MarkdownOptions {
        indent: 4,
        escape_underscores: false,
        escape_html: true,
        include_furniture: false,
        max_list_depth: 10,
        linkify_urls: true,
        insert_page_breaks: false,
    };
    let _serializer_no_escape = MarkdownSerializer::with_options(options_no_escape);
    println!("Options:");
    println!("  - 4-space indentation");
    println!("  - Escape underscores: false (allows italic/bold formatting)");
    println!("  - Escape HTML: true");
    println!();

    // Example 4: Allow HTML pass-through
    println!("=== Example 4: Allow HTML Pass-through ===");
    let options_html = MarkdownOptions {
        indent: 4,
        escape_underscores: true,
        escape_html: false,
        include_furniture: false,
        max_list_depth: 10,
        linkify_urls: true,
        insert_page_breaks: false,
    };
    let _serializer_html = MarkdownSerializer::with_options(options_html);
    println!("Options:");
    println!("  - 4-space indentation");
    println!("  - Escape underscores: true");
    println!("  - Escape HTML: false (allows HTML tags)");
    println!();

    // Example 5: Minimal escaping (for clean output)
    println!("=== Example 5: Minimal Escaping (Clean Output) ===");
    let options_minimal = MarkdownOptions {
        indent: 2,
        escape_underscores: false,
        escape_html: false,
        include_furniture: false,
        max_list_depth: 10,
        linkify_urls: true,
        insert_page_breaks: false,
    };
    let _serializer_minimal = MarkdownSerializer::with_options(options_minimal);
    println!("Options:");
    println!("  - 2-space indentation");
    println!("  - Escape underscores: false");
    println!("  - Escape HTML: false");
    println!("  Use case: Clean, readable markdown for human consumption");
    println!();

    // Tips for choosing options
    println!("{}", "=".repeat(80));
    println!("Choosing the Right Options:");
    println!();
    println!("Indentation:");
    println!("  • 4 spaces: Standard markdown, GitHub-style");
    println!("  • 2 spaces: More compact, saves horizontal space");
    println!();
    println!("Escape Underscores:");
    println!("  • true: Safer, prevents accidental italic/bold formatting");
    println!("  •       Use for technical docs with variable names (e.g., my_variable)");
    println!("  • false: Cleaner output, allows intentional formatting");
    println!("  •        Use for general text without underscores");
    println!();
    println!("Escape HTML:");
    println!("  • true: Safer, prevents HTML injection/rendering");
    println!("  •       Use when markdown will be rendered to HTML");
    println!("  • false: Allows HTML tags in output");
    println!("  •        Use when HTML pass-through is desired");
    println!("{}", "=".repeat(80));

    Ok(())
}

/// Display a preview of the markdown output
fn display_preview(markdown: &str, label: &str) {
    let preview_length = 400;
    let preview = if markdown.len() > preview_length {
        format!("{}...", &markdown[..preview_length])
    } else {
        markdown.to_string()
    };

    println!("Preview ({label}):");
    println!("{}", "-".repeat(80));
    println!("{preview}");
    println!("{}", "-".repeat(80));
}
