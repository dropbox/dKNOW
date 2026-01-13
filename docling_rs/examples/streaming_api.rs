//! Streaming API Example
//!
//! This example demonstrates efficient batch processing using the streaming API,
//! with progress reporting and error recovery.
//!
//! Run with:
//! ```bash
//! cargo run --example streaming_api -- path/to/documents/*.pdf
//! ```

use docling_backend::DocumentConverter;
use docling_core::Result;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

fn main() -> Result<()> {
    // Get input files from command-line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <path-to-documents...>", args[0]);
        eprintln!("\nExample:");
        eprintln!("  {} documents/*.pdf", args[0]);
        eprintln!("  {} file1.pdf file2.docx file3.html", args[0]);
        std::process::exit(1);
    }

    let input_paths: Vec<PathBuf> = args[1..].iter().map(PathBuf::from).collect();

    println!("Streaming API - Batch Document Processing");
    println!("Processing {} documents", input_paths.len());
    println!();

    // Create output directory
    let output_dir = PathBuf::from("output");
    fs::create_dir_all(&output_dir)?;
    println!("Output directory: {}", output_dir.display());
    println!();

    // Process documents with streaming approach
    stream_convert_documents(&input_paths, &output_dir)?;

    Ok(())
}

/// Stream processing: Process documents one at a time with progress reporting
fn stream_convert_documents(inputs: &[PathBuf], output_dir: &Path) -> Result<()> {
    let start = Instant::now();

    // Create converter once and reuse (important for performance)
    let converter = DocumentConverter::new()?;

    let mut stats = BatchStats::new(inputs.len());

    println!("=== Starting Batch Conversion ===");
    println!();

    // Process each document sequentially with progress reporting
    for (i, input_path) in inputs.iter().enumerate() {
        let file_name = input_path.file_name().unwrap().to_string_lossy();

        // Progress indicator
        let progress = (i + 1) as f64 / inputs.len() as f64 * 100.0;
        print!(
            "[{:>3.0}%] [{}/{}] Processing {}... ",
            progress,
            i + 1,
            inputs.len(),
            file_name
        );

        let doc_start = Instant::now();

        match converter.convert(input_path) {
            Ok(result) => {
                // Generate output filename
                let output_filename = input_path
                    .file_stem()
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
                    + ".md";
                let output_path = output_dir.join(&output_filename);

                // Save output
                match fs::write(&output_path, &result.document.markdown) {
                    Ok(_) => {
                        let doc_elapsed = doc_start.elapsed();
                        println!(
                            "✓ {} chars, {:?}",
                            result.document.metadata.num_characters, doc_elapsed
                        );

                        stats.record_success(result.document.metadata.num_characters, doc_elapsed);
                    }
                    Err(e) => {
                        println!("✗ Write failed: {e}");
                        stats.record_failure();
                    }
                }
            }
            Err(e) => {
                println!("✗ Conversion failed: {e}");
                stats.record_failure();

                // Optionally: implement error recovery or skip
                // For now, continue to next document
            }
        }
    }

    let total_elapsed = start.elapsed();

    // Display comprehensive summary
    println!();
    println!("=== Batch Conversion Summary ===");
    println!("Total time: {total_elapsed:?}");
    println!("Success: {} / {}", stats.success_count, stats.total_count);
    println!("Failures: {}", stats.failure_count);
    println!("Success rate: {:.1}%", stats.success_rate());
    println!();

    if stats.success_count > 0 {
        println!("Performance Metrics:");
        println!("  Total characters: {}", stats.total_characters);
        println!("  Average per document: {:?}", stats.average_time());
        println!("  Throughput: {:.0} chars/sec", stats.throughput());
        println!(
            "  Documents per minute: {:.1}",
            stats.docs_per_minute(total_elapsed)
        );
    }

    println!();
    println!("Output files saved to: {}", output_dir.display());

    Ok(())
}

/// Statistics tracker for batch processing
struct BatchStats {
    total_count: usize,
    success_count: usize,
    failure_count: usize,
    total_characters: usize,
    total_processing_time: std::time::Duration,
}

impl BatchStats {
    fn new(total_count: usize) -> Self {
        Self {
            total_count,
            success_count: 0,
            failure_count: 0,
            total_characters: 0,
            total_processing_time: std::time::Duration::ZERO,
        }
    }

    fn record_success(&mut self, chars: usize, elapsed: std::time::Duration) {
        self.success_count += 1;
        self.total_characters += chars;
        self.total_processing_time += elapsed;
    }

    fn record_failure(&mut self) {
        self.failure_count += 1;
    }

    fn success_rate(&self) -> f64 {
        if self.total_count == 0 {
            0.0
        } else {
            (self.success_count as f64 / self.total_count as f64) * 100.0
        }
    }

    fn average_time(&self) -> std::time::Duration {
        if self.success_count == 0 {
            std::time::Duration::ZERO
        } else {
            self.total_processing_time / self.success_count as u32
        }
    }

    fn throughput(&self) -> f64 {
        if self.total_processing_time.as_secs_f64() == 0.0 {
            0.0
        } else {
            self.total_characters as f64 / self.total_processing_time.as_secs_f64()
        }
    }

    fn docs_per_minute(&self, total_elapsed: std::time::Duration) -> f64 {
        if total_elapsed.as_secs_f64() == 0.0 {
            0.0
        } else {
            (self.success_count as f64 / total_elapsed.as_secs_f64()) * 60.0
        }
    }
}
