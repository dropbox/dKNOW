//! Benchmark for bulk fast path API
//!
//! Run with: cargo run --release --package video-extract-core --example bulk_benchmark

use std::path::PathBuf;
use std::time::Instant;
use video_extract_core::BulkExecutor;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Collect test video files
    let test_files: Vec<PathBuf> = vec![
        "test_edge_cases/video_4k_ultra_hd_3840x2160__stress_test.mp4",
        "test_edge_cases/video_hevc_h265_modern_codec__compatibility.mp4",
        "test_edge_cases/video_high_fps_120__temporal_test.mp4",
        "test_edge_cases/video_no_audio_stream__error_test.mov",
        "test_edge_cases/video_single_frame_only__minimal.mp4",
        "test_edge_cases/video_tiny_64x64_resolution__scaling_test.mp4",
        "test_edge_cases/video_variable_framerate_vfr__timing_test.mp4",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect();

    println!("\n=== Bulk Fast Path Benchmark ===");
    println!("Total files: {}", test_files.len());
    println!();

    // Test 1: Sequential baseline (max_concurrent_files=1)
    println!("--- Test 1: Sequential Baseline (N=1) ---");
    let sequential_time = run_bulk_test(&test_files, 1).await;
    println!(
        "Sequential processing: {:.2}s ({:.2}s per file)\n",
        sequential_time.as_secs_f64(),
        sequential_time.as_secs_f64() / test_files.len() as f64
    );

    // Test 2: Parallel with 4 workers
    println!("--- Test 2: Parallel (N=4) ---");
    let parallel_4_time = run_bulk_test(&test_files, 4).await;
    let speedup_4 = sequential_time.as_secs_f64() / parallel_4_time.as_secs_f64();
    println!(
        "Parallel (4 workers): {:.2}s ({:.2}s per file)",
        parallel_4_time.as_secs_f64(),
        parallel_4_time.as_secs_f64() / test_files.len() as f64
    );
    println!("Speedup: {:.2}x\n", speedup_4);

    // Test 3: Parallel with 8 workers
    println!("--- Test 3: Parallel (N=8) ---");
    let parallel_8_time = run_bulk_test(&test_files, 8).await;
    let speedup_8 = sequential_time.as_secs_f64() / parallel_8_time.as_secs_f64();
    println!(
        "Parallel (8 workers): {:.2}s ({:.2}s per file)",
        parallel_8_time.as_secs_f64(),
        parallel_8_time.as_secs_f64() / test_files.len() as f64
    );
    println!("Speedup: {:.2}x\n", speedup_8);

    // Summary
    println!("=== Summary ===");
    println!("Sequential (N=1):  {:.2}s", sequential_time.as_secs_f64());
    println!(
        "Parallel (N=4):    {:.2}s ({:.2}x speedup)",
        parallel_4_time.as_secs_f64(),
        speedup_4
    );
    println!(
        "Parallel (N=8):    {:.2}s ({:.2}x speedup)",
        parallel_8_time.as_secs_f64(),
        speedup_8
    );
    println!();

    if speedup_4 >= 2.0 {
        println!("✅ SUCCESS: Achieved 2x+ speedup with 4 workers (goal: 2x)");
    } else {
        println!(
            "⚠️  WARNING: Speedup with 4 workers is {:.2}x (goal: 2x)",
            speedup_4
        );
    }

    if speedup_8 >= 2.5 {
        println!("✅ SUCCESS: Achieved 2.5x+ speedup with 8 workers (goal: 2.5x)");
    } else {
        println!(
            "⚠️  WARNING: Speedup with 8 workers is {:.2}x (goal: 2.5x)",
            speedup_8
        );
    }
}

async fn run_bulk_test(files: &[PathBuf], max_concurrent: usize) -> std::time::Duration {
    let executor = BulkExecutor::new().with_max_concurrent_files(max_concurrent);

    let start_time = Instant::now();

    let mut rx = executor
        .execute_bulk_fast_path(files.to_vec(), 0.25, None)
        .await
        .expect("Failed to start bulk executor");

    let mut results = Vec::with_capacity(files.len());

    while let Some(result) = rx.recv().await {
        results.push(result);
    }

    let total_time = start_time.elapsed();

    // Print results
    for result in &results {
        let file_name = result
            .input_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        match &result.result {
            Ok(detections) => {
                println!(
                    "  ✅ {} - {} detections in {:.2}s",
                    file_name,
                    detections.len(),
                    result.processing_time.as_secs_f64()
                );
            }
            Err(e) => {
                // Truncate error message
                let err_msg = if e.len() > 80 {
                    format!("{}...", &e[..80])
                } else {
                    e.clone()
                };
                println!(
                    "  ❌ {} - Error: {} (took {:.2}s)",
                    file_name,
                    err_msg,
                    result.processing_time.as_secs_f64()
                );
            }
        }
    }

    println!(
        "  Total wall time: {:.2}s (includes parallelism)\n",
        total_time.as_secs_f64()
    );

    total_time
}
