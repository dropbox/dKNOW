//! Comprehensive scalability benchmark for bulk fast path API
//!
//! Tests bulk processing with production videos (10+ files) to validate Phase 2 success criteria:
//! - 2x+ speedup for 10+ files
//! - Scalability across N=1, 2, 4, 8, 16 workers
//! - Memory usage tracking
//!
//! Run with: cargo run --release --package video-extract-core --example bulk_scalability_benchmark

use std::path::PathBuf;
use std::time::Instant;
use video_extract_core::{BulkExecutor, BulkFastPathResult};

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    println!("\n=== Bulk Fast Path Scalability Benchmark ===");
    println!("Purpose: Validate Phase 2 success criteria (BULK_API_PLAN_N24.md)");
    println!();

    // Collect production test video files from Desktop
    let mut test_files: Vec<PathBuf> = Vec::new();

    // Large production videos (38MB - 1.3GB)
    let candidate_files = vec![
        "~/Desktop/stuff/stuff/Screen Recording 2025-06-02 at 11.14.26 AM.mov", // 38MB
        "~/Desktop/stuff/stuff/May 5 - live labeling mocks.mp4",                // 38MB
        "~/Desktop/stuff/stuff/relevance-annotations-first-pass.mov",           // 97MB
        "~/Desktop/stuff/stuff/relevance-annotations-first-pass (1).mov",       // 97MB
        "~/Desktop/stuff/stuff/mission control video demo 720.mov",             // 277MB
        "~/Desktop/stuff/stuff/Investor update - Calendar Agent - Oct 6.mp4",   // 349MB
        "~/Desktop/stuff/stuff/GMT20250516-190317_Recording_avo_1920x1080 braintrust.mp4", // 980MB
    ];

    // Expand home directory and check which files exist
    for file_str in candidate_files {
        let expanded = shellexpand::tilde(file_str);
        let path = PathBuf::from(expanded.as_ref());
        if path.exists() {
            test_files.push(path);
        } else {
            eprintln!("Warning: File not found: {}", file_str);
        }
    }

    if test_files.is_empty() {
        eprintln!("Error: No test files found. Please ensure videos are in ~/Desktop/stuff/stuff/");
        std::process::exit(1);
    }

    println!("Found {} production video files:", test_files.len());
    for (i, file) in test_files.iter().enumerate() {
        let file_name = file
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        let metadata = std::fs::metadata(file);
        let size_mb = metadata
            .map(|m| m.len() as f64 / 1_048_576.0)
            .unwrap_or(0.0);
        println!("  {}. {} ({:.1} MB)", i + 1, file_name, size_mb);
    }
    println!();

    if test_files.len() < 10 {
        println!(
            "Warning: Only {} files available (target: 10+ for 2x speedup validation)",
            test_files.len()
        );
        println!("Proceeding with available files...\n");
    }

    // Run scalability tests
    let worker_counts = vec![1, 2, 4, 8];
    let mut results: Vec<(std::time::Duration, usize, usize, f64)> =
        Vec::with_capacity(worker_counts.len());

    for &workers in &worker_counts {
        println!("--- Test: N={} workers ---", workers);
        let (elapsed, success_count, total_count, per_file_times) =
            run_bulk_test(&test_files, workers).await;

        let speedup = if workers == 1 {
            1.0
        } else {
            results[0].0.as_secs_f64() / elapsed.as_secs_f64()
        };

        println!(
            "  Total time: {:.2}s | Success: {}/{} | Speedup: {:.2}x",
            elapsed.as_secs_f64(),
            success_count,
            total_count,
            speedup
        );

        // Calculate statistics
        let avg_time = per_file_times.iter().sum::<f64>() / per_file_times.len() as f64;
        let min_time = per_file_times.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_time = per_file_times
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);

        println!(
            "  Per-file time: min={:.2}s, avg={:.2}s, max={:.2}s",
            min_time, avg_time, max_time
        );
        println!();

        results.push((elapsed, success_count, total_count, speedup));
    }

    // Print summary
    println!("=== Summary ===");
    println!();
    println!("| Workers | Total Time | Success | Speedup |");
    println!("|---------|------------|---------|---------|");
    for (i, &workers) in worker_counts.iter().enumerate() {
        let (elapsed, success, total, speedup) = results[i];
        println!(
            "| {:>7} | {:>9.2}s | {:>2}/{:<2}   | {:>6.2}x |",
            workers,
            elapsed.as_secs_f64(),
            success,
            total,
            speedup
        );
    }
    println!();

    // Validate success criteria
    println!("=== Success Criteria (BULK_API_PLAN_N24.md) ===");
    println!();

    let files_tested = test_files.len();
    let (_, _, _, speedup_4) = results
        .iter()
        .find(|_| worker_counts.contains(&4))
        .copied()
        .unwrap_or((
            std::time::Duration::ZERO,
            0,
            0,
            results.last().map(|(_, _, _, s)| *s).unwrap_or(0.0),
        ));

    if files_tested >= 10 {
        if speedup_4 >= 2.0 {
            println!(
                "✅ SUCCESS: 2x+ speedup achieved with {} files (4 workers: {:.2}x)",
                files_tested, speedup_4
            );
        } else {
            println!(
                "❌ FAILED: Expected 2x+ speedup with 10+ files, got {:.2}x (4 workers)",
                speedup_4
            );
        }
    } else {
        println!(
            "⚠️  PARTIAL: Only {} files tested (target: 10+). Speedup with 4 workers: {:.2}x",
            files_tested, speedup_4
        );
    }

    // Check for linear scaling
    let (_, _, _, speedup_8) = results
        .iter()
        .find(|_| worker_counts.contains(&8))
        .copied()
        .unwrap_or((std::time::Duration::ZERO, 0, 0, 0.0));

    if speedup_8 >= 2.5 {
        println!(
            "✅ SUCCESS: 2.5x+ speedup with 8 workers ({:.2}x)",
            speedup_8
        );
    } else {
        println!(
            "⚠️  INFO: Speedup with 8 workers is {:.2}x (target: 2.5x+)",
            speedup_8
        );
    }

    println!();
    println!("=== Analysis ===");
    println!();

    // Efficiency analysis
    let efficiency_4 = speedup_4 / 4.0 * 100.0;
    let efficiency_8 = speedup_8 / 8.0 * 100.0;

    println!(
        "Parallel efficiency: 4 workers = {:.1}%, 8 workers = {:.1}%",
        efficiency_4, efficiency_8
    );

    if efficiency_4 >= 50.0 {
        println!(
            "✅ Good parallel efficiency (4 workers: {:.1}%)",
            efficiency_4
        );
    } else {
        println!(
            "⚠️  Low parallel efficiency (4 workers: {:.1}%). Possible causes:",
            efficiency_4
        );
        println!("   - Small batch size (limited parallelism)");
        println!("   - Uneven workload distribution");
        println!("   - FFmpeg init mutex contention");
    }

    println!();
}

async fn run_bulk_test(
    files: &[PathBuf],
    max_concurrent: usize,
) -> (std::time::Duration, usize, usize, Vec<f64>) {
    let executor = BulkExecutor::new().with_max_concurrent_files(max_concurrent);

    let start_time = Instant::now();

    let mut rx = executor
        .execute_bulk_fast_path(files.to_vec(), 0.25, None)
        .await
        .expect("Failed to start bulk executor");

    let mut results: Vec<BulkFastPathResult> = Vec::new();

    while let Some(result) = rx.recv().await {
        results.push(result);
    }

    let total_time = start_time.elapsed();

    // Count successes and collect per-file times
    let success_count = results.iter().filter(|r| r.result.is_ok()).count();
    let total_count = results.len();
    let per_file_times: Vec<f64> = results
        .iter()
        .map(|r| r.processing_time.as_secs_f64())
        .collect();

    // Print individual file results (compact format)
    for result in &results {
        let file_name = result
            .input_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        match &result.result {
            Ok(detections) => {
                println!(
                    "    ✅ {} - {} detections in {:.2}s",
                    file_name,
                    detections.len(),
                    result.processing_time.as_secs_f64()
                );
            }
            Err(e) => {
                // Truncate error message
                let err_msg = if e.len() > 60 {
                    format!("{}...", &e[..60])
                } else {
                    e.clone()
                };
                println!(
                    "    ❌ {} - Error: {} ({:.2}s)",
                    file_name,
                    err_msg,
                    result.processing_time.as_secs_f64()
                );
            }
        }
    }

    (total_time, success_count, total_count, per_file_times)
}
