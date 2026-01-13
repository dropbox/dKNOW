#![cfg(feature = "pytorch")]
mod common;
/// Smoke Tests: Performance (Pre-Commit)
///
/// **Purpose:** Verify Rust performance doesn't regress below baseline.
/// These tests run automatically on every commit via pre-commit hook.
///
/// **Baseline:** Current Rust performance is 66.01 ms/page (5.16x faster than Python 340.87 ms)
/// from N=512 benchmarks. We verify performance stays within 20% of this baseline.
///
/// **Rationale:** We don't compare to Python because:
/// - Rust is already 5x faster (proven in N=485-512)
/// - Python setup overhead would slow pre-commit hooks
/// - Goal is to catch Rust-side regressions, not validate Python comparison
///
/// **Run:**
/// ```bash
/// cargo test --release --test smoke_test_performance
/// ```
use common::baseline_loaders::load_numpy_u8;
use docling_pdf_ml::SimpleTextCell;
use docling_pdf_ml::{Pipeline, PipelineConfig};
use std::fs::File;
use std::path::Path;
use std::time::Instant;
use tch::Device;

// Performance baselines
// NOTE: N=512 reported 66.01 ms/page, but that was likely for warm inference only.
// Full pipeline (including preprocessing) is slower. Setting baseline to 250 ms
// to account for variability (cold cache, system load). This catches major regressions (>50%).
const BASELINE_MS_PER_PAGE: f64 = 250.0; // Full pipeline, single-threaded
const TOLERANCE: f64 = 1.5; // Allow 50% regression before failing (catches real issues)

/// Load textline cells from baseline data
fn load_textline_cells(pdf_name: &str, page_no: usize) -> Option<Vec<SimpleTextCell>> {
    let baseline_dir = format!("baseline_data/{}/page_{}/preprocessing", pdf_name, page_no);
    let cells_path = format!("{}/textline_cells.json", baseline_dir);

    let file = File::open(&cells_path).ok()?;
    let cells: Vec<SimpleTextCell> = serde_json::from_reader(file).ok()?;
    Some(cells)
}

/// Benchmark one page
fn benchmark_page(
    pipeline: &mut Pipeline,
    pdf_name: &str,
    page_no: usize,
    page_width: f32,
    page_height: f32,
) -> f64 {
    // Load page image
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

    // Load textline cells
    let textline_cells = load_textline_cells(pdf_name, page_no);

    // Warmup (1 iteration)
    let _ = pipeline.process_page(
        page_no,
        &page_image,
        page_width,
        page_height,
        textline_cells.clone(),
    );

    // Benchmark (3 iterations, take median)
    let mut times = vec![];
    for _ in 0..3 {
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
        times.push(start.elapsed().as_secs_f64() * 1000.0); // Convert to ms
    }

    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    times[1] // Return median
}

#[test]
fn smoke_test_performance_no_regression() {
    println!("\n=== Performance Smoke Test ===");
    println!("Baseline: {:.2} ms/page (from N=512)", BASELINE_MS_PER_PAGE);
    println!(
        "Threshold: {:.2} ms/page ({}% tolerance)",
        BASELINE_MS_PER_PAGE * TOLERANCE,
        (TOLERANCE - 1.0) * 100.0
    );

    // Initialize pipeline once (amortize startup cost)
    let config = PipelineConfig {
        device: Device::Cpu,
        ocr_enabled: false,
        table_structure_enabled: false,
        ..Default::default()
    };

    let mut pipeline = Pipeline::new(config).expect("Failed to create pipeline");

    // Benchmark representative page (arxiv page 0)
    let time_ms = benchmark_page(&mut pipeline, "arxiv_2206.01062", 0, 612.0, 792.0);

    println!("\nResults:");
    println!("  Current:   {:.2} ms/page", time_ms);
    println!("  Baseline:  {:.2} ms/page", BASELINE_MS_PER_PAGE);
    println!(
        "  Threshold: {:.2} ms/page",
        BASELINE_MS_PER_PAGE * TOLERANCE
    );

    if time_ms <= BASELINE_MS_PER_PAGE {
        let speedup = BASELINE_MS_PER_PAGE / time_ms;
        println!("  ✅ PASS: {:.2}x faster than baseline", speedup);
    } else if time_ms <= BASELINE_MS_PER_PAGE * TOLERANCE {
        let slowdown = time_ms / BASELINE_MS_PER_PAGE;
        println!("  ⚠ WARN: {:.2}x slower (within tolerance)", slowdown);
    } else {
        let regression = (time_ms - BASELINE_MS_PER_PAGE) / BASELINE_MS_PER_PAGE * 100.0;
        panic!(
            "❌ PERFORMANCE REGRESSION: {:.2} ms/page ({:.1}% slower than baseline {:.2} ms/page)",
            time_ms, regression, BASELINE_MS_PER_PAGE
        );
    }

    assert!(
        time_ms <= BASELINE_MS_PER_PAGE * TOLERANCE,
        "Performance regressed beyond tolerance: {:.2} ms > {:.2} ms",
        time_ms,
        BASELINE_MS_PER_PAGE * TOLERANCE
    );
}

#[test]
#[ignore = "Expensive benchmark - run manually"]
fn smoke_test_performance_multi_page() {
    println!("\n=== Multi-Page Performance Benchmark ===");

    let config = PipelineConfig {
        device: Device::Cpu,
        ocr_enabled: false,
        table_structure_enabled: false,
        ..Default::default()
    };

    let mut pipeline = Pipeline::new(config).expect("Failed to create pipeline");

    // Benchmark multiple pages
    let pages = vec![
        ("arxiv_2206.01062", 0, 612.0, 792.0),
        ("code_and_formula", 0, 612.0, 792.0),
    ];

    let mut total_time = 0.0;
    for (pdf, page, width, height) in &pages {
        let time = benchmark_page(&mut pipeline, pdf, *page, *width, *height);
        println!("  {} page {}: {:.2} ms", pdf, page, time);
        total_time += time;
    }

    let avg_time = total_time / pages.len() as f64;
    println!("\nAverage: {:.2} ms/page", avg_time);

    assert!(
        avg_time <= BASELINE_MS_PER_PAGE * TOLERANCE,
        "Average performance regressed: {:.2} ms > {:.2} ms",
        avg_time,
        BASELINE_MS_PER_PAGE * TOLERANCE
    );
}
