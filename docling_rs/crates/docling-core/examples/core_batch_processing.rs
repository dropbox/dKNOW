//! Batch document conversion example
//!
//! This example demonstrates processing multiple documents
//! and collecting conversion statistics.
//!
//! Usage:
//! ```bash
//! cargo run --example batch_processing -- file1.pdf file2.docx file3.html
//! ```

use docling_backend::DocumentConverter;
use docling_core::DoclingError;
use std::env;
use std::time::Duration;

struct ConversionStats {
    total: usize,
    successful: usize,
    failed: usize,
    total_time: Duration,
    total_chars: usize,
}

impl ConversionStats {
    fn new() -> Self {
        Self {
            total: 0,
            successful: 0,
            failed: 0,
            total_time: Duration::ZERO,
            total_chars: 0,
        }
    }

    fn print_summary(&self) {
        println!("\n=== Conversion Summary ===");
        println!("Total documents: {}", self.total);
        println!("Successful: {}", self.successful);
        println!("Failed: {}", self.failed);
        println!(
            "Success rate: {:.1}%",
            (self.successful as f64 / self.total as f64) * 100.0
        );
        println!("Total time: {:.2}s", self.total_time.as_secs_f64());
        println!(
            "Average time: {:.2}s/doc",
            self.total_time.as_secs_f64() / self.total as f64
        );
        println!("Total characters extracted: {}", self.total_chars);
        println!(
            "Throughput: {:.0} chars/sec",
            self.total_chars as f64 / self.total_time.as_secs_f64()
        );
    }
}

fn main() -> Result<(), DoclingError> {
    // Get file paths from command line
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <file1> <file2> ...", args[0]);
        eprintln!("Example: {} doc1.pdf doc2.docx doc3.html", args[0]);
        std::process::exit(1);
    }
    let file_paths = &args[1..];

    println!("Processing {} documents...\n", file_paths.len());

    // Create converter (reuse for all documents)
    let converter = DocumentConverter::new()?;

    let mut stats = ConversionStats::new();

    // Process each document
    for file_path in file_paths {
        stats.total += 1;
        println!(
            "[{}/{}] Processing: {}",
            stats.total,
            file_paths.len(),
            file_path
        );

        match converter.convert(file_path) {
            Ok(result) => {
                stats.successful += 1;
                stats.total_time += result.latency;
                stats.total_chars += result.document.metadata.num_characters;

                println!(
                    "  ✓ Success: {} chars in {:?}",
                    result.document.metadata.num_characters, result.latency
                );

                // Optionally save to file
                // std::fs::write(
                //     format!("{}.md", file_path),
                //     &result.document.markdown
                // )?;
            }
            Err(e) => {
                stats.failed += 1;
                eprintln!("  ✗ Failed: {e}");
            }
        }
    }

    stats.print_summary();

    Ok(())
}
