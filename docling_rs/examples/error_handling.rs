//! Error Handling Example
//!
//! This example demonstrates robust error handling patterns for document conversion,
//! including retry logic, validation, and graceful degradation.
//!
//! Run with:
//! ```bash
//! cargo run --example error_handling -- path/to/document.pdf
//! ```

use docling_backend::DocumentConverter;
use docling_core::{DoclingError, Result};
use std::env;
use std::path::Path;
use std::thread;
use std::time::Duration;

fn main() {
    // Get input file from command-line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <path-to-document>", args[0]);
        eprintln!("\nExample:");
        eprintln!("  {} document.pdf", args[0]);
        eprintln!("  {} invalid.pdf", args[0]);
        eprintln!("  {} nonexistent.pdf", args[0]);
        std::process::exit(1);
    }

    let input_path = &args[1];

    // Example 1: Basic error handling with pattern matching
    println!("=== Example 1: Basic Error Handling ===");
    match convert_with_error_handling(input_path) {
        Ok(markdown) => {
            println!("✓ Conversion successful!");
            println!("Output length: {} characters", markdown.len());
        }
        Err(e) => {
            eprintln!("✗ Conversion failed: {e}");
        }
    }
    println!();

    // Example 2: Retry logic for transient failures
    println!("=== Example 2: Retry Logic ===");
    match convert_with_retry(input_path, 3) {
        Ok(markdown) => {
            println!("✓ Conversion successful (possibly after retries)!");
            println!("Output length: {} characters", markdown.len());
        }
        Err(e) => {
            eprintln!("✗ Conversion failed after retries: {e}");
        }
    }
    println!();

    // Example 3: Validation before conversion
    println!("=== Example 3: Pre-conversion Validation ===");
    match validate_and_convert(input_path) {
        Ok(markdown) => {
            println!("✓ Validation passed and conversion successful!");
            println!("Output length: {} characters", markdown.len());
        }
        Err(e) => {
            eprintln!("✗ Validation or conversion failed: {e}");
        }
    }
}

/// Convert with comprehensive error handling
fn convert_with_error_handling(path: &str) -> Result<String> {
    // Create converter
    let converter = DocumentConverter::new().map_err(|e| {
        eprintln!("Failed to create converter: {e}");
        e
    })?;

    // Convert document with detailed error handling
    match converter.convert(path) {
        Ok(result) => {
            println!("Converted successfully in {:?}", result.latency);
            Ok(result.document.markdown)
        }
        Err(DoclingError::IoError(msg)) => {
            eprintln!("File not found: {msg}");
            eprintln!("Please check the file path and try again.");
            Err(DoclingError::IoError(msg))
        }
        Err(DoclingError::FormatError(msg)) => {
            eprintln!("Unsupported format: {msg}");
            eprintln!("Supported formats: PDF, DOCX, HTML, Markdown, images, etc.");
            Err(DoclingError::FormatError(msg))
        }
        Err(DoclingError::ConversionError(msg)) => {
            eprintln!("Conversion failed: {msg}");
            eprintln!("The document may be corrupted or malformed.");
            Err(DoclingError::ConversionError(msg))
        }
        Err(DoclingError::PythonError(msg)) => {
            eprintln!("Python backend error: {msg}");
            eprintln!("Please ensure Python docling package is installed correctly.");
            Err(DoclingError::PythonError(msg))
        }
        Err(e) => {
            eprintln!("Unexpected error: {e}");
            Err(e)
        }
    }
}

/// Convert with retry logic for transient failures
fn convert_with_retry(path: &str, max_retries: u32) -> Result<String> {
    let mut last_error = None;

    for attempt in 1..=max_retries {
        println!("Attempt {attempt}/{max_retries}");

        match DocumentConverter::new() {
            Ok(converter) => match converter.convert(path) {
                Ok(result) => {
                    if attempt > 1 {
                        println!("✓ Succeeded on attempt {attempt}");
                    }
                    return Ok(result.document.markdown);
                }
                Err(e) => {
                    last_error = Some(e);
                    eprintln!(
                        "✗ Attempt {} failed: {}",
                        attempt,
                        last_error.as_ref().unwrap()
                    );

                    // Don't retry on certain errors
                    match &last_error {
                        Some(DoclingError::IoError(_)) => break,
                        Some(DoclingError::FormatError(_)) => break,
                        _ => {}
                    }

                    if attempt < max_retries {
                        let delay = Duration::from_secs(2u64.pow(attempt - 1)); // Exponential backoff
                        println!("Waiting {delay:?} before retry...");
                        thread::sleep(delay);
                    }
                }
            },
            Err(e) => {
                last_error = Some(e);
                eprintln!(
                    "✗ Failed to create converter: {}",
                    last_error.as_ref().unwrap()
                );
            }
        }
    }

    Err(last_error.unwrap())
}

/// Validate input before attempting conversion
fn validate_and_convert(path: &str) -> Result<String> {
    // Validation 1: Check if file exists
    let file_path = Path::new(path);
    if !file_path.exists() {
        return Err(DoclingError::ConversionError(format!(
            "File does not exist: {path}"
        )));
    }

    // Validation 2: Check if it's a file (not a directory)
    if !file_path.is_file() {
        return Err(DoclingError::ConversionError(format!(
            "Path is not a file: {path}"
        )));
    }

    // Validation 3: Check file extension
    let supported_extensions = [
        "pdf", "docx", "doc", "pptx", "ppt", "xlsx", "xls", "html", "htm", "md", "txt", "jpg",
        "jpeg", "png", "tiff", "tif", "bmp", "webp",
    ];

    if let Some(ext) = file_path.extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();
        if !supported_extensions.contains(&ext_str.as_str()) {
            eprintln!("Warning: Extension '{ext_str}' may not be supported");
        }
    } else {
        eprintln!("Warning: File has no extension");
    }

    // Validation 4: Check file size (warn if very large)
    if let Ok(metadata) = file_path.metadata() {
        let size_mb = metadata.len() as f64 / 1_024_000.0;
        if size_mb > 100.0 {
            eprintln!("Warning: File is very large ({size_mb:.1} MB), conversion may take a while");
        }
        println!("File size: {size_mb:.2} MB");
    }

    // All validations passed, attempt conversion
    println!("Validation passed, converting...");
    let converter = DocumentConverter::new()?;
    let result = converter.convert(path)?;

    Ok(result.document.markdown)
}
