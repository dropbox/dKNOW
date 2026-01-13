//! Batch Processing Example
//!
//! This example demonstrates how to process multiple documents efficiently,
//! including parallel processing with error handling.
//!
//! Run with:
//! ```bash
//! cargo run --example batch_processing -- path/to/documents/*.pdf
//! ```

use docling_backend::DocumentConverter;
use docling_core::Result;
use rayon::prelude::*;
use std::env;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

fn main() -> Result<()> {
    // Get input files from command-line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <path-to-documents...>", args[0]);
        eprintln!("\nExample:");
        eprintln!("  {} document1.pdf document2.docx document3.html", args[0]);
        eprintln!("  {} documents/*.pdf", args[0]);
        std::process::exit(1);
    }

    let input_paths: Vec<PathBuf> = args[1..].iter().map(PathBuf::from).collect();

    println!("Batch processing {} documents", input_paths.len());
    println!();

    // Sequential processing example
    println!("=== Sequential Processing ===");
    sequential_processing(&input_paths)?;
    println!();

    // Parallel processing example
    println!("=== Parallel Processing ===");
    parallel_processing(&input_paths)?;

    Ok(())
}

/// Process documents sequentially (one at a time)
fn sequential_processing(paths: &[PathBuf]) -> Result<()> {
    let start = Instant::now();
    let converter = DocumentConverter::new()?;

    let mut success_count = 0;
    let mut failure_count = 0;

    for (i, path) in paths.iter().enumerate() {
        print!(
            "[{}/{}] Processing {}... ",
            i + 1,
            paths.len(),
            path.display()
        );

        match converter.convert(path) {
            Ok(result) => {
                println!(
                    "✓ ({} chars, {:?})",
                    result.document.metadata.num_characters, result.latency
                );
                success_count += 1;
            }
            Err(e) => {
                println!("✗ Error: {e}");
                failure_count += 1;
            }
        }
    }

    let elapsed = start.elapsed();
    println!();
    println!("Sequential Processing Summary:");
    println!("  Total time: {elapsed:?}");
    println!("  Success: {success_count}");
    println!("  Failures: {failure_count}");
    println!(
        "  Average time per document: {:?}",
        elapsed / paths.len() as u32
    );

    Ok(())
}

/// Process documents in parallel using rayon
fn parallel_processing(paths: &[PathBuf]) -> Result<()> {
    let start = Instant::now();

    let success_count = Arc::new(AtomicUsize::new(0));
    let failure_count = Arc::new(AtomicUsize::new(0));

    // Process documents in parallel
    // Note: Each thread creates its own converter instance
    let results: Vec<_> = paths
        .par_iter()
        .map(|path| {
            let converter = DocumentConverter::new().expect("Failed to create converter");

            match converter.convert(path) {
                Ok(result) => {
                    success_count.fetch_add(1, Ordering::Relaxed);
                    println!(
                        "✓ {}: {} chars, {:?}",
                        path.display(),
                        result.document.metadata.num_characters,
                        result.latency
                    );
                    Ok(result.document.markdown.len())
                }
                Err(e) => {
                    failure_count.fetch_add(1, Ordering::Relaxed);
                    eprintln!("✗ {}: {}", path.display(), e);
                    Err(e)
                }
            }
        })
        .collect();

    let elapsed = start.elapsed();
    let success = success_count.load(Ordering::Relaxed);
    let failures = failure_count.load(Ordering::Relaxed);

    println!();
    println!("Parallel Processing Summary:");
    println!("  Total time: {elapsed:?}");
    println!("  Success: {success}");
    println!("  Failures: {failures}");
    println!(
        "  Average time per document: {:?}",
        elapsed / paths.len() as u32
    );

    // Calculate speedup
    println!(
        "  Speedup: ~{:.1}x (depends on CPU cores)",
        paths.len() as f64 / elapsed.as_secs_f64()
    );

    // Display total characters processed
    let total_chars: usize = results.iter().filter_map(|r| r.as_ref().ok()).sum();
    println!("  Total characters extracted: {total_chars}");

    Ok(())
}
