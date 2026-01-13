//! # Resolution Benchmark
//!
//! Compares inference performance at different input resolutions.
//!
//! ## Expected Results
//!
//! - 640x640 (Full): Baseline accuracy
//! - 512x512 (Medium): ~1.56x faster, ~1-2% accuracy loss
//! - 448x448 (Fast): ~2.04x faster, ~3-5% accuracy loss
//!
//! ## Run
//!
//! ```bash
//! # Quick comparison test
//! cargo test --release --test resolution_benchmark test_resolution_comparison -- --nocapture
//!
//! # Full benchmark with accuracy comparison
//! cargo test --release --test resolution_benchmark -- --ignored --nocapture
//! ```

mod common;

use common::baseline_loaders::load_numpy_u8;
use docling_pdf_ml::models::layout_predictor::{InferenceBackend, LayoutPredictorModel};
use docling_pdf_ml::preprocessing::layout::LayoutResolution;
use ndarray::{Array3, Ix3};
use std::path::Path;
use std::time::Instant;

// Use stub Device when pytorch is disabled
#[cfg(not(feature = "pytorch"))]
use docling_pdf_ml::pipeline::Device;
#[cfg(feature = "pytorch")]
use tch::Device;

/// Get the ONNX model path
fn get_model_path() -> std::path::PathBuf {
    let base_dirs = [
        "onnx_exports/layout_optimum",
        "crates/docling-pdf-ml/onnx_exports/layout_optimum",
    ];

    for base in &base_dirs {
        // Prefer INT8 model if available (from N=3483)
        let int8 = std::path::PathBuf::from(base).join("model_int8.onnx");
        if int8.exists() {
            return int8;
        }
        let fp32 = std::path::PathBuf::from(base).join("model.onnx");
        if fp32.exists() {
            return fp32;
        }
    }

    panic!("Could not find ONNX model directory");
}

/// Load a test page image for benchmarking (u8 format)
fn load_test_image() -> Array3<u8> {
    // Try to load from baseline data
    let baseline_paths = [
        "baseline_data/redp5110/page_0/layout/input_page_image.npy",
        "baseline_data/2305.03393v1/page_0/layout/input_page_image.npy",
    ];

    for path in &baseline_paths {
        if let Ok(img) = load_numpy_u8(Path::new(path)) {
            if let Ok(arr3) = img.into_dimensionality::<Ix3>() {
                return arr3;
            }
        }
    }

    // Fall back to synthetic test image (792x612 RGB - typical page size)
    eprintln!("Warning: No baseline data found, using synthetic 792x612 test image");
    Array3::<u8>::zeros((792, 612, 3))
}

/// Benchmark a model at a specific resolution
fn benchmark_resolution(
    model: &mut LayoutPredictorModel,
    image: &Array3<u8>,
    resolution: LayoutResolution,
    iterations: usize,
) -> (f64, f64, f64, usize) {
    // Warmup
    for _ in 0..3 {
        let _ = model.infer_with_resolution(image, resolution);
    }

    // Benchmark
    let mut times = Vec::with_capacity(iterations);
    let mut total_clusters = 0;

    for _ in 0..iterations {
        let start = Instant::now();
        let result = model.infer_with_resolution(image, resolution);
        times.push(start.elapsed().as_secs_f64() * 1000.0); // ms

        if let Ok(clusters) = result {
            total_clusters += clusters.len();
        }
    }

    let avg = times.iter().sum::<f64>() / times.len() as f64;
    let min = times.iter().copied().fold(f64::INFINITY, f64::min);
    let max = times.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let avg_clusters = total_clusters / iterations;

    (avg, min, max, avg_clusters)
}

#[test]
fn test_resolution_comparison() {
    let model_path = get_model_path();

    println!("\n=== Resolution Benchmark ===\n");
    println!("Model: {}", model_path.display());

    // Load model
    let mut model = match LayoutPredictorModel::load_with_backend(
        &model_path,
        Device::Cpu,
        InferenceBackend::ONNX,
    ) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Failed to load model: {e}");
            return;
        }
    };

    // Load test image
    let image = load_test_image();
    println!("Input shape: {:?}", image.dim());

    let resolutions = [
        (LayoutResolution::Full, "640x640 (Full)"),
        (LayoutResolution::Medium, "512x512 (Medium)"),
        (LayoutResolution::Fast, "448x448 (Fast)"),
    ];

    println!("\nRunning 5 iterations per resolution...\n");
    println!(
        "{:<20} {:>12} {:>12} {:>12} {:>10} {:>10}",
        "Resolution", "Avg (ms)", "Min (ms)", "Max (ms)", "Clusters", "Speedup"
    );
    println!("{}", "-".repeat(78));

    let mut baseline_avg = 0.0;
    for (resolution, name) in &resolutions {
        let (avg, min, max, clusters) = benchmark_resolution(&mut model, &image, *resolution, 5);

        if *resolution == LayoutResolution::Full {
            baseline_avg = avg;
        }

        let speedup = if baseline_avg > 0.0 && avg > 0.0 {
            baseline_avg / avg
        } else {
            1.0
        };

        println!("{name:<20} {avg:>12.2} {min:>12.2} {max:>12.2} {clusters:>10} {speedup:>9.2}x");
    }

    // Calculate expected vs actual speedup
    println!("\n=== Expected vs Actual Speedup ===\n");
    for (resolution, name) in &resolutions {
        let expected = resolution.expected_speedup();
        println!("{name}: expected {expected:.2}x");
    }

    println!("\nNote: Actual speedup may differ from theoretical due to:");
    println!("  - Fixed overhead (model loading, post-processing)");
    println!("  - Memory bandwidth limitations");
    println!("  - CPU cache effects");
}

/// Test accuracy at different resolutions (compare cluster counts and bbox similarity)
#[test]
fn test_resolution_accuracy() {
    let model_path = get_model_path();

    if !model_path.exists() {
        println!("Skipping accuracy test - model not available");
        return;
    }

    println!("\n=== Resolution Accuracy Comparison ===\n");

    // Load model
    let mut model =
        LayoutPredictorModel::load_with_backend(&model_path, Device::Cpu, InferenceBackend::ONNX)
            .expect("Failed to load model");

    // Load test image
    let image = load_test_image();

    // Get baseline results at full resolution
    let full_results = model
        .infer_with_resolution(&image, LayoutResolution::Full)
        .expect("Full resolution inference failed");

    let resolutions = [
        (LayoutResolution::Medium, "512x512 (Medium)"),
        (LayoutResolution::Fast, "448x448 (Fast)"),
    ];

    println!(
        "Baseline (640x640): {} clusters detected\n",
        full_results.len()
    );

    for (resolution, name) in &resolutions {
        let results = model
            .infer_with_resolution(&image, *resolution)
            .expect("Inference failed");

        let count_diff = (full_results.len() as i64 - results.len() as i64).abs();
        let count_diff_pct = if full_results.is_empty() {
            0.0
        } else {
            count_diff as f64 / full_results.len() as f64 * 100.0
        };

        println!("{}: {} clusters detected", name, results.len());
        println!("  Cluster count difference: {count_diff} ({count_diff_pct:.1}%)");

        // Compare bounding boxes for matched elements
        if !full_results.is_empty() && !results.is_empty() {
            let mut bbox_diffs = Vec::new();
            let mut label_mismatches = 0;

            for (full, reduced) in full_results.iter().zip(results.iter()) {
                if full.label != reduced.label {
                    label_mismatches += 1;
                }

                let l_diff = (full.bbox.l - reduced.bbox.l).abs();
                let t_diff = (full.bbox.t - reduced.bbox.t).abs();
                let r_diff = (full.bbox.r - reduced.bbox.r).abs();
                let b_diff = (full.bbox.b - reduced.bbox.b).abs();
                let avg_diff = (l_diff + t_diff + r_diff + b_diff) / 4.0;
                bbox_diffs.push(avg_diff);
            }

            if !bbox_diffs.is_empty() {
                let avg_bbox_diff = bbox_diffs.iter().sum::<f64>() / bbox_diffs.len() as f64;
                let max_bbox_diff = bbox_diffs.iter().copied().fold(0.0, f64::max);
                println!("  Bbox diff: avg={avg_bbox_diff:.2}px, max={max_bbox_diff:.2}px");
                println!(
                    "  Label mismatches: {} ({:.1}%)",
                    label_mismatches,
                    label_mismatches as f64 / bbox_diffs.len() as f64 * 100.0
                );
            }
        }
        println!();
    }
}

/// Full benchmark with more iterations (ignored by default - slow)
#[test]
#[ignore = "Slow benchmark test - run manually with --ignored"]
fn test_resolution_full_benchmark() {
    let model_path = get_model_path();

    println!("\n=== Full Resolution Benchmark (20 iterations) ===\n");

    let mut model =
        LayoutPredictorModel::load_with_backend(&model_path, Device::Cpu, InferenceBackend::ONNX)
            .expect("Failed to load model");

    let image = load_test_image();

    let resolutions = [
        (LayoutResolution::Full, "640x640"),
        (LayoutResolution::Medium, "512x512"),
        (LayoutResolution::Fast, "448x448"),
        (LayoutResolution::Custom(384), "384x384"),
        (LayoutResolution::Custom(320), "320x320"),
    ];

    println!(
        "{:<15} {:>12} {:>12} {:>12} {:>10}",
        "Resolution", "Avg (ms)", "Min (ms)", "Max (ms)", "Speedup"
    );
    println!("{}", "-".repeat(63));

    let mut baseline_avg = 0.0;
    for (resolution, name) in &resolutions {
        let (avg, min, max, _) = benchmark_resolution(&mut model, &image, *resolution, 20);

        if *resolution == LayoutResolution::Full {
            baseline_avg = avg;
        }

        let speedup = if baseline_avg > 0.0 && avg > 0.0 {
            baseline_avg / avg
        } else {
            1.0
        };

        println!("{name:<15} {avg:>12.2} {min:>12.2} {max:>12.2} {speedup:>9.2}x");
    }
}
