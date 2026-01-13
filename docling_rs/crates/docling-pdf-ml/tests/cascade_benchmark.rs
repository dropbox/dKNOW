//! # Cascade Layout Benchmark
//!
//! Compares performance of different cascade routing modes:
//! - `AlwaysML`: Always use RT-DETR model (~60ms/page) - baseline
//! - `Auto`: Route simple pages to heuristics (~1ms), complex to ML
//! - `AlwaysHeuristic`: Always use heuristics (~1ms/page) - fastest but least accurate
//! - `Conservative`: Heuristics only for definitely simple pages
//!
//! ## Expected Results
//!
//! - Documents with many simple pages (books, reports): Auto mode 5-10x faster
//! - Documents with complex pages (tables, figures): Auto mode similar to AlwaysML
//! - Academic papers: Mixed results depending on page complexity
//!
//! ## Run
//!
//! ```bash
//! # Quick test (single page)
//! cargo test --release --features pytorch --test cascade_benchmark test_cascade_single_page
//!
//! # Full benchmark (ignored by default - slow)
//! cargo test --release --features pytorch --test cascade_benchmark -- --ignored --nocapture
//! ```

#![cfg(feature = "pytorch")]
mod common;

use common::baseline_loaders::load_numpy_u8;
use docling_pdf_ml::{CascadeMode, Pipeline, PipelineConfigBuilder, SimpleTextCell};
use std::fs::File;
use std::path::Path;
use std::time::Instant;
use tch::Device;

/// Load textline cells from baseline data (same as smoke_test_performance.rs)
fn load_textline_cells(pdf_name: &str, page_no: usize) -> Option<Vec<SimpleTextCell>> {
    let baseline_dir = format!("baseline_data/{}/page_{}/preprocessing", pdf_name, page_no);
    let cells_path = format!("{}/textline_cells.json", baseline_dir);

    let file = File::open(&cells_path).ok()?;
    let cells: Vec<SimpleTextCell> = serde_json::from_reader(file).ok()?;
    Some(cells)
}

/// Result of benchmarking a single mode
#[derive(Debug, Clone)]
struct ModeResult {
    mode: CascadeMode,
    avg_ms: f64,
    min_ms: f64,
    max_ms: f64,
    heuristic_pct: f64,
    speedup_vs_ml: f64,
}

/// Benchmark a single cascade mode on one page
fn benchmark_mode(
    mode: CascadeMode,
    pdf_name: &str,
    page_no: usize,
    page_width: f32,
    page_height: f32,
    iterations: usize,
) -> ModeResult {
    // Load page data
    let image_path_str = format!(
        "baseline_data/{}/page_{}/layout/input_page_image.npy",
        pdf_name, page_no
    );
    let image_path = Path::new(&image_path_str);
    let page_image_dyn = load_numpy_u8(image_path)
        .unwrap_or_else(|_| panic!("Failed to load {} page {} image", pdf_name, page_no));

    let page_image = page_image_dyn
        .into_dimensionality::<ndarray::Ix3>()
        .expect("Failed to convert to 3D array");

    let textline_cells = load_textline_cells(pdf_name, page_no);

    // Initialize pipeline with specified cascade mode
    let config = PipelineConfigBuilder::new()
        .device(Device::Cpu)
        .ocr_enabled(false)
        .table_structure_enabled(false)
        .cascade_mode(mode)
        .build()
        .expect("Failed to build config");

    let mut pipeline = Pipeline::new(config).expect("Failed to create pipeline");

    // Warmup (2 iterations)
    for _ in 0..2 {
        let _ = pipeline.process_page(
            page_no,
            &page_image,
            page_width,
            page_height,
            textline_cells.clone(),
        );
    }
    pipeline.reset_cascade_stats();

    // Benchmark
    let mut times = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let start = Instant::now();
        let _ = pipeline
            .process_page(
                page_no,
                &page_image,
                page_width,
                page_height,
                textline_cells.clone(),
            )
            .expect("Failed to process page");
        times.push(start.elapsed().as_secs_f64() * 1000.0);
    }

    // Compute statistics
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let avg_ms = times.iter().sum::<f64>() / times.len() as f64;
    let min_ms = times.first().copied().unwrap();
    let max_ms = times.last().copied().unwrap();

    let stats = pipeline.cascade_stats();
    let heuristic_pct = stats.heuristic_percentage();

    ModeResult {
        mode,
        avg_ms,
        min_ms,
        max_ms,
        heuristic_pct,
        speedup_vs_ml: 1.0, // Calculated later relative to AlwaysML
    }
}

/// Compare all cascade modes
#[test]
fn test_cascade_single_page() {
    println!("\n=== Cascade Layout Benchmark (Single Page) ===\n");

    let pdf_name = "arxiv_2206.01062";
    let page_no = 0;
    let page_width = 612.0;
    let page_height = 792.0;
    let iterations = 5;

    let modes = [
        CascadeMode::AlwaysML,
        CascadeMode::Auto,
        CascadeMode::AlwaysHeuristic,
        CascadeMode::Conservative,
    ];

    let mut results = Vec::new();

    for mode in &modes {
        println!("Benchmarking {:?}...", mode);
        let result = benchmark_mode(
            *mode,
            pdf_name,
            page_no,
            page_width,
            page_height,
            iterations,
        );
        results.push(result);
    }

    // Calculate speedup relative to AlwaysML
    let ml_avg = results[0].avg_ms;
    for result in &mut results {
        result.speedup_vs_ml = ml_avg / result.avg_ms;
    }

    // Print results table
    println!("\n{:-^80}", " Results ");
    println!(
        "{:<18} {:>10} {:>10} {:>10} {:>12} {:>10}",
        "Mode", "Avg (ms)", "Min (ms)", "Max (ms)", "Heuristic %", "Speedup"
    );
    println!("{:-<80}", "");

    for result in &results {
        println!(
            "{:<18} {:>10.2} {:>10.2} {:>10.2} {:>11.1}% {:>9.2}x",
            format!("{:?}", result.mode),
            result.avg_ms,
            result.min_ms,
            result.max_ms,
            result.heuristic_pct,
            result.speedup_vs_ml,
        );
    }

    println!("{:-<80}", "");

    // Assertions
    // AlwaysHeuristic should be significantly faster than AlwaysML
    let ml_result = &results[0];
    let heuristic_result = &results[2];

    println!("\n=== Analysis ===");
    println!(
        "AlwaysML:        {:.2} ms/page (baseline)",
        ml_result.avg_ms
    );
    println!(
        "AlwaysHeuristic: {:.2} ms/page ({:.1}x faster)",
        heuristic_result.avg_ms, heuristic_result.speedup_vs_ml
    );

    if heuristic_result.speedup_vs_ml > 5.0 {
        println!("  Heuristics working correctly (>5x speedup)");
    } else {
        println!(
            "  WARNING: Expected >5x speedup from heuristics, got {:.1}x",
            heuristic_result.speedup_vs_ml
        );
    }

    // Auto mode should be between ML and heuristic
    let auto_result = &results[1];
    println!(
        "\nAuto mode: {:.2} ms/page ({:.1}x faster than ML)",
        auto_result.avg_ms, auto_result.speedup_vs_ml
    );
    println!(
        "  Routing: {:.1}% heuristic, {:.1}% ML",
        auto_result.heuristic_pct,
        100.0 - auto_result.heuristic_pct
    );

    // Verify heuristics are meaningfully faster
    assert!(
        heuristic_result.avg_ms < ml_result.avg_ms,
        "Heuristics should be faster than ML"
    );
}

/// Full benchmark across multiple pages (ignored by default - slow)
#[test]
#[ignore = "Slow benchmark - run manually with --ignored"]
fn test_cascade_multi_page_benchmark() {
    println!("\n=== Cascade Layout Benchmark (Multi-Page) ===\n");

    let test_pages = vec![("arxiv_2206.01062", 0, 612.0, 792.0)];

    let modes = [
        CascadeMode::AlwaysML,
        CascadeMode::Auto,
        CascadeMode::AlwaysHeuristic,
        CascadeMode::Conservative,
    ];

    let mut all_results: Vec<(String, Vec<ModeResult>)> = Vec::new();

    for (pdf_name, page_no, page_width, page_height) in &test_pages {
        println!("\n--- {} page {} ---", pdf_name, page_no);

        let mut results = Vec::new();
        for mode in &modes {
            println!("  {:?}...", mode);
            let result = benchmark_mode(*mode, pdf_name, *page_no, *page_width, *page_height, 10);
            results.push(result);
        }

        // Calculate speedup
        let ml_avg = results[0].avg_ms;
        for result in &mut results {
            result.speedup_vs_ml = ml_avg / result.avg_ms;
        }

        all_results.push((format!("{}_p{}", pdf_name, page_no), results));
    }

    // Print summary
    println!("\n{:=^100}", " SUMMARY ");
    println!(
        "{:<30} {:>15} {:>15} {:>15} {:>15}",
        "Page", "AlwaysML (ms)", "Auto (ms)", "AlwaysHeur (ms)", "Conservative (ms)"
    );
    println!("{:-<100}", "");

    for (page_name, results) in &all_results {
        println!(
            "{:<30} {:>15.2} {:>15.2} {:>15.2} {:>15.2}",
            page_name, results[0].avg_ms, results[1].avg_ms, results[2].avg_ms, results[3].avg_ms,
        );
    }

    // Calculate averages across all pages
    let num_pages = all_results.len() as f64;
    let mut avg_by_mode = [0.0f64; 4];
    for (_, results) in &all_results {
        for (i, result) in results.iter().enumerate() {
            avg_by_mode[i] += result.avg_ms;
        }
    }
    for avg in &mut avg_by_mode {
        *avg /= num_pages;
    }

    println!("{:-<100}", "");
    println!(
        "{:<30} {:>15.2} {:>15.2} {:>15.2} {:>15.2}",
        "AVERAGE", avg_by_mode[0], avg_by_mode[1], avg_by_mode[2], avg_by_mode[3]
    );

    let overall_speedup_auto = avg_by_mode[0] / avg_by_mode[1];
    let overall_speedup_heur = avg_by_mode[0] / avg_by_mode[2];
    println!("\n=== Overall Speedup vs AlwaysML ===");
    println!("  Auto:           {:.2}x", overall_speedup_auto);
    println!("  AlwaysHeuristic: {:.2}x", overall_speedup_heur);
}

/// Test cascade stats tracking
#[test]
fn test_cascade_stats_tracking() {
    println!("\n=== Cascade Stats Tracking Test ===\n");

    // Load page data
    let pdf_name = "arxiv_2206.01062";
    let page_no = 0;
    let image_path_str = format!(
        "baseline_data/{}/page_{}/layout/input_page_image.npy",
        pdf_name, page_no
    );
    let page_image_dyn = load_numpy_u8(Path::new(&image_path_str)).expect("Failed to load image");
    let page_image = page_image_dyn
        .into_dimensionality::<ndarray::Ix3>()
        .expect("Failed to convert to 3D array");
    let textline_cells = load_textline_cells(pdf_name, page_no);

    // Test AlwaysML mode - should have 0 heuristic, 100% ML
    let config = PipelineConfigBuilder::new()
        .device(Device::Cpu)
        .ocr_enabled(false)
        .table_structure_enabled(false)
        .cascade_mode(CascadeMode::AlwaysML)
        .build()
        .expect("Failed to build config");

    let mut pipeline = Pipeline::new(config).expect("Failed to create pipeline");
    pipeline.reset_cascade_stats();

    // Process 3 pages
    for _ in 0..3 {
        let _ = pipeline.process_page(0, &page_image, 612.0, 792.0, textline_cells.clone());
    }

    let stats = pipeline.cascade_stats();
    println!("AlwaysML mode stats:");
    println!("  ML count: {}", stats.ml_count);
    println!("  Heuristic count: {}", stats.heuristic_count);
    println!("  Heuristic %: {:.1}%", stats.heuristic_percentage());
    println!("  Estimated speedup: {:.2}x", stats.speedup_factor());

    assert_eq!(stats.ml_count, 3, "AlwaysML should use ML for all pages");
    assert_eq!(stats.heuristic_count, 0, "AlwaysML should use 0 heuristics");
    assert!(
        (stats.speedup_factor() - 1.0).abs() < 0.01,
        "AlwaysML speedup should be ~1.0"
    );

    // Test AlwaysHeuristic mode - should have 100% heuristic, 0 ML
    let config = PipelineConfigBuilder::new()
        .device(Device::Cpu)
        .ocr_enabled(false)
        .table_structure_enabled(false)
        .cascade_mode(CascadeMode::AlwaysHeuristic)
        .build()
        .expect("Failed to build config");

    let mut pipeline = Pipeline::new(config).expect("Failed to create pipeline");
    pipeline.reset_cascade_stats();

    for _ in 0..3 {
        let _ = pipeline.process_page(0, &page_image, 612.0, 792.0, textline_cells.clone());
    }

    let stats = pipeline.cascade_stats();
    println!("\nAlwaysHeuristic mode stats:");
    println!("  ML count: {}", stats.ml_count);
    println!("  Heuristic count: {}", stats.heuristic_count);
    println!("  Heuristic %: {:.1}%", stats.heuristic_percentage());
    println!("  Estimated speedup: {:.2}x", stats.speedup_factor());

    assert_eq!(
        stats.heuristic_count, 3,
        "AlwaysHeuristic should use heuristics for all pages"
    );
    assert_eq!(stats.ml_count, 0, "AlwaysHeuristic should use 0 ML");
    assert!(
        stats.speedup_factor() > 50.0,
        "AlwaysHeuristic speedup should be >50x"
    );

    println!("\n All stats tracking tests passed!");
}
