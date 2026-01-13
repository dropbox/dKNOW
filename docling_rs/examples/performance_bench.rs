//! Performance Benchmarking Example
//!
//! This example demonstrates how to measure conversion performance,
//! including throughput, latency, and resource usage.
//!
//! Run with:
//! ```bash
//! cargo run --release --example performance_bench -- path/to/document.pdf
//! ```

use docling_backend::DocumentConverter;
use docling_core::Result;
use std::env;
use std::time::{Duration, Instant};

fn main() -> Result<()> {
    // Get input file from command-line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <path-to-document> [iterations]", args[0]);
        eprintln!("\nExample:");
        eprintln!("  {} document.pdf", args[0]);
        eprintln!("  {} document.pdf 10", args[0]);
        std::process::exit(1);
    }

    let input_path = &args[1];
    let iterations = if args.len() > 2 {
        args[2].parse().unwrap_or(5)
    } else {
        5
    };

    println!("Performance Benchmarking");
    println!("Document: {input_path}");
    println!("Iterations: {iterations}");
    println!();

    // Warmup run (Python initialization overhead)
    println!("=== Warmup Run ===");
    {
        let converter = DocumentConverter::new()?;
        let result = converter.convert(input_path)?;
        println!("✓ Warmup complete");
        println!("  Pages: {:?}", result.document.metadata.num_pages);
        println!(
            "  Characters: {:?}",
            result.document.metadata.num_characters
        );
        println!();
    }

    // Benchmark: Multiple runs with same converter instance
    println!("=== Benchmark: Reusing Converter Instance ===");
    let reuse_stats = benchmark_with_reuse(input_path, iterations)?;
    display_stats(&reuse_stats);
    println!();

    // Benchmark: Creating new converter each time
    println!("=== Benchmark: Creating New Converter Each Time ===");
    let new_stats = benchmark_with_new_converter(input_path, iterations)?;
    display_stats(&new_stats);
    println!();

    // Comparison
    println!("=== Performance Comparison ===");
    println!("Reusing converter:");
    println!("  Average: {:?}", reuse_stats.average());
    println!("  Median: {:?}", reuse_stats.median());
    println!();
    println!("Creating new converter:");
    println!("  Average: {:?}", new_stats.average());
    println!("  Median: {:?}", new_stats.median());
    println!();
    println!(
        "Speedup from reusing converter: {:.2}x",
        new_stats.average().as_secs_f64() / reuse_stats.average().as_secs_f64()
    );
    println!();

    // Recommendations
    println!("=== Performance Recommendations ===");
    println!("✓ Reuse DocumentConverter instance when possible");
    println!("✓ Process documents in batches to amortize initialization cost");
    println!("✓ Use release build for production (--release flag)");
    println!("✓ Consider parallel processing for large batches (see batch_processing.rs)");
    println!("✓ Monitor memory usage for very large documents");

    Ok(())
}

/// Benchmark by reusing the same converter instance
fn benchmark_with_reuse(path: &str, iterations: usize) -> Result<BenchmarkStats> {
    let converter = DocumentConverter::new()?;
    let mut times = Vec::new();
    let mut total_chars = 0;

    for i in 1..=iterations {
        let start = Instant::now();
        let result = converter.convert(path)?;
        let elapsed = start.elapsed();

        times.push(elapsed);
        total_chars = result.document.metadata.num_characters;

        println!("  Run {i}: {elapsed:?}");
    }

    Ok(BenchmarkStats { times, total_chars })
}

/// Benchmark by creating a new converter for each conversion
fn benchmark_with_new_converter(path: &str, iterations: usize) -> Result<BenchmarkStats> {
    let mut times = Vec::new();
    let mut total_chars = 0;

    for i in 1..=iterations {
        let start = Instant::now();

        let converter = DocumentConverter::new()?;
        let result = converter.convert(path)?;

        let elapsed = start.elapsed();

        times.push(elapsed);
        total_chars = result.document.metadata.num_characters;

        println!("  Run {i}: {elapsed:?}");
    }

    Ok(BenchmarkStats { times, total_chars })
}

/// Statistics for benchmark results
struct BenchmarkStats {
    times: Vec<Duration>,
    total_chars: usize,
}

impl BenchmarkStats {
    fn average(&self) -> Duration {
        let total: Duration = self.times.iter().sum();
        total / self.times.len() as u32
    }

    fn median(&self) -> Duration {
        let mut sorted = self.times.clone();
        sorted.sort();
        sorted[sorted.len() / 2]
    }

    fn min(&self) -> Duration {
        *self.times.iter().min().unwrap()
    }

    fn max(&self) -> Duration {
        *self.times.iter().max().unwrap()
    }

    fn std_dev(&self) -> f64 {
        let avg = self.average().as_secs_f64();
        let variance: f64 = self
            .times
            .iter()
            .map(|t| {
                let diff = t.as_secs_f64() - avg;
                diff * diff
            })
            .sum::<f64>()
            / self.times.len() as f64;
        variance.sqrt()
    }

    fn throughput(&self) -> f64 {
        if self.average().as_secs_f64() == 0.0 {
            0.0
        } else {
            self.total_chars as f64 / self.average().as_secs_f64()
        }
    }
}

/// Display benchmark statistics
fn display_stats(stats: &BenchmarkStats) {
    println!("Statistics:");
    println!("  Runs: {}", stats.times.len());
    println!("  Average: {:?}", stats.average());
    println!("  Median: {:?}", stats.median());
    println!("  Min: {:?}", stats.min());
    println!("  Max: {:?}", stats.max());
    println!("  Std Dev: {:.3}s", stats.std_dev());
    println!("  Throughput: {:.0} chars/sec", stats.throughput());

    // Performance analysis
    let coefficient_of_variation = stats.std_dev() / stats.average().as_secs_f64();
    if coefficient_of_variation < 0.05 {
        println!("  Consistency: Excellent (CV < 5%)");
    } else if coefficient_of_variation < 0.10 {
        println!("  Consistency: Good (CV < 10%)");
    } else if coefficient_of_variation < 0.20 {
        println!("  Consistency: Fair (CV < 20%)");
    } else {
        println!("  Consistency: Poor (CV >= 20%) - Results highly variable");
    }
}
