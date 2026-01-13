//! Format Detection Example
//!
//! This example demonstrates automatic format detection and batch conversion
//! of documents with different formats.
//!
//! Run with:
//! ```bash
//! cargo run --example format_detection -- path/to/documents/*
//! ```

use docling_backend::DocumentConverter;
use docling_core::{InputFormat, Result};
use std::collections::HashMap;
use std::env;
use std::path::Path;

fn main() -> Result<()> {
    // Get input files from command-line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <path-to-documents...>", args[0]);
        eprintln!("\nExample:");
        eprintln!("  {} documents/*", args[0]);
        eprintln!("  {} file1.pdf file2.docx file3.html", args[0]);
        std::process::exit(1);
    }

    let input_paths: Vec<&str> = args[1..].iter().map(String::as_str).collect();

    println!(
        "Processing {} files with automatic format detection",
        input_paths.len()
    );
    println!();

    // Group files by detected format
    let grouped_files = group_by_format(&input_paths);

    // Display format distribution
    println!("=== Format Distribution ===");
    for (format, files) in &grouped_files {
        println!("{:?}: {} file(s)", format, files.len());
        for file in files {
            println!(
                "  - {}",
                Path::new(file).file_name().unwrap().to_string_lossy()
            );
        }
    }
    println!();

    // Create converter (uses automatic format detection)
    let converter = DocumentConverter::new()?;

    // Process all files
    println!("=== Converting Documents ===");
    let mut success_count = 0;
    let mut failure_count = 0;
    let mut total_chars = 0;

    for path in &input_paths {
        print!(
            "Converting {}... ",
            Path::new(path).file_name().unwrap().to_string_lossy()
        );

        match converter.convert(path) {
            Ok(result) => {
                let chars = result.document.metadata.num_characters;
                println!("✓ {} chars, {:?}", chars, result.latency);
                success_count += 1;
                total_chars += chars;
            }
            Err(e) => {
                println!("✗ {e}");
                failure_count += 1;
            }
        }
    }

    // Display summary
    println!();
    println!("=== Conversion Summary ===");
    println!("Total files: {}", input_paths.len());
    println!("Success: {success_count}");
    println!("Failures: {failure_count}");
    println!("Total characters extracted: {total_chars}");

    // Display format-specific tips
    println!();
    display_format_tips();

    Ok(())
}

/// Group files by their detected format based on file extension
fn group_by_format<'a>(paths: &[&'a str]) -> HashMap<InputFormat, Vec<&'a str>> {
    let mut grouped: HashMap<InputFormat, Vec<&str>> = HashMap::new();

    for path in paths {
        let format = detect_format_from_extension(path);
        grouped.entry(format).or_default().push(path);
    }

    grouped
}

/// Detect format from file extension
fn detect_format_from_extension(path: &str) -> InputFormat {
    let path_obj = Path::new(path);

    if let Some(ext) = path_obj.extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();
        match ext_str.as_str() {
            "pdf" => InputFormat::Pdf,
            "docx" => InputFormat::Docx,
            "doc" => InputFormat::Doc,
            "pptx" | "ppt" => InputFormat::Pptx,
            "xlsx" | "xls" => InputFormat::Xlsx,
            "html" | "htm" => InputFormat::Html,
            "md" | "markdown" => InputFormat::Md,
            "csv" | "txt" => InputFormat::Csv,
            "jpg" | "jpeg" | "png" | "tiff" | "tif" | "bmp" | "webp" | "gif" => InputFormat::Pdf, // Images handled as PDF
            _ => InputFormat::Pdf, // Default fallback
        }
    } else {
        InputFormat::Pdf
    }
}

/// Display tips for different document formats
fn display_format_tips() {
    println!("=== Format-Specific Tips ===");
    println!();

    println!("PDF Documents:");
    println!("  • Digital PDFs (with embedded text) convert fastest");
    println!("  • Scanned PDFs require OCR (use --ocr flag)");
    println!("  • Complex layouts may need manual review");
    println!();

    println!("Microsoft Office (DOCX, PPTX, XLSX):");
    println!("  • Tables are preserved in markdown format");
    println!("  • Charts and diagrams are extracted when possible");
    println!("  • Macros and VBA code are not extracted");
    println!();

    println!("HTML Documents:");
    println!("  • JavaScript content may not render");
    println!("  • External CSS styles are not applied");
    println!("  • Best for static HTML content");
    println!();

    println!("Images (JPG, PNG, TIFF):");
    println!("  • Require OCR for text extraction");
    println!("  • High-resolution images (300+ DPI) work best");
    println!("  • Support depends on image quality and contrast");
    println!();

    println!("Markdown and Plain Text:");
    println!("  • Convert instantly (no parsing needed)");
    println!("  • Useful for normalization and validation");
    println!();

    println!("Unsupported Formats:");
    println!("  • Check the documentation for full format support");
    println!("  • Some formats may need conversion to supported types");
}
