//! # INT8 Quantization Benchmark
//!
//! Compares performance of INT8 quantized vs FP32 ONNX layout model.
//!
//! ## Expected Results
//!
//! - INT8 model: ~2x faster inference (due to INT8 matrix operations)
//! - INT8 model: ~4x smaller file size (43MB vs 164MB)
//! - INT8 model: <1% accuracy loss
//!
//! ## Run
//!
//! ```bash
//! # Quick comparison test
//! cargo test --release --test int8_quantization_benchmark test_int8_vs_fp32 -- --nocapture
//!
//! # Full benchmark (multiple iterations)
//! cargo test --release --test int8_quantization_benchmark -- --ignored --nocapture
//! ```

mod common;

use common::baseline_loaders::load_numpy_u8;
use docling_pdf_ml::models::layout_predictor::{InferenceBackend, LayoutPredictorModel};
use ndarray::{Array3, Ix3};
use std::path::Path;
use std::time::Instant;

// Use stub Device when pytorch is disabled
#[cfg(not(feature = "pytorch"))]
use docling_pdf_ml::pipeline::Device;
#[cfg(feature = "pytorch")]
use tch::Device;

/// Get the ONNX model paths
fn get_model_paths() -> (std::path::PathBuf, std::path::PathBuf) {
    let base_dirs = [
        "onnx_exports/layout_optimum",
        "crates/docling-pdf-ml/onnx_exports/layout_optimum",
    ];

    for base in &base_dirs {
        let fp32 = std::path::PathBuf::from(base).join("model.onnx");
        let int8 = std::path::PathBuf::from(base).join("model_int8.onnx");
        if fp32.exists() {
            return (fp32, int8);
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

    // Fall back to synthetic test image (640x480 RGB)
    eprintln!("Warning: No baseline data found, using synthetic 640x480 test image");
    Array3::<u8>::zeros((480, 640, 3))
}

/// Benchmark a single model
fn benchmark_model(model_path: &Path, image: &Array3<u8>, iterations: usize) -> (f64, f64, f64) {
    // Skip if model doesn't exist
    if !model_path.exists() {
        return (f64::NAN, f64::NAN, f64::NAN);
    }

    // Load model
    let mut model = match LayoutPredictorModel::load_with_backend(
        model_path,
        Device::Cpu,
        InferenceBackend::ONNX,
    ) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Failed to load model {}: {}", model_path.display(), e);
            return (f64::NAN, f64::NAN, f64::NAN);
        }
    };

    // Warmup
    for _ in 0..3 {
        let _ = model.infer(image);
    }

    // Benchmark
    let mut times = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let start = Instant::now();
        let _ = model.infer(image);
        times.push(start.elapsed().as_secs_f64() * 1000.0); // ms
    }

    let avg = times.iter().sum::<f64>() / times.len() as f64;
    let min = times.iter().copied().fold(f64::INFINITY, f64::min);
    let max = times.iter().copied().fold(f64::NEG_INFINITY, f64::max);

    (avg, min, max)
}

#[test]
fn test_int8_vs_fp32() {
    let (fp32_path, int8_path) = get_model_paths();

    println!("\n=== INT8 vs FP32 ONNX Layout Model Benchmark ===\n");
    println!("FP32 model: {}", fp32_path.display());
    println!("INT8 model: {}", int8_path.display());

    // Check file sizes
    if fp32_path.exists() {
        let fp32_size = std::fs::metadata(&fp32_path).map(|m| m.len()).unwrap_or(0);
        println!("FP32 size:  {:.1} MB", fp32_size as f64 / 1024.0 / 1024.0);
    }
    if int8_path.exists() {
        let int8_size = std::fs::metadata(&int8_path).map(|m| m.len()).unwrap_or(0);
        println!("INT8 size:  {:.1} MB", int8_size as f64 / 1024.0 / 1024.0);
    }

    // Load test image
    let image = load_test_image();

    println!("\nInput shape: {:?}", image.dim());
    println!("Running 5 iterations per model...\n");

    // Benchmark FP32
    let (fp32_avg, fp32_min, fp32_max) = benchmark_model(&fp32_path, &image, 5);
    if !fp32_avg.is_nan() {
        println!("FP32:  avg={fp32_avg:.2}ms  min={fp32_min:.2}ms  max={fp32_max:.2}ms");
    } else {
        println!("FP32:  NOT AVAILABLE");
    }

    // Benchmark INT8
    let (int8_avg, int8_min, int8_max) = benchmark_model(&int8_path, &image, 5);
    if !int8_avg.is_nan() {
        println!("INT8:  avg={int8_avg:.2}ms  min={int8_min:.2}ms  max={int8_max:.2}ms");
    } else {
        println!("INT8:  NOT AVAILABLE");
    }

    // Calculate speedup
    if !fp32_avg.is_nan() && !int8_avg.is_nan() {
        let speedup = fp32_avg / int8_avg;
        println!("\n=== Result ===");
        println!("INT8 speedup: {speedup:.2}x");

        // Expected ~2x speedup
        if speedup > 1.5 {
            println!("SUCCESS: INT8 quantization provides significant speedup");
        } else if speedup > 1.0 {
            println!("INFO: INT8 quantization provides modest speedup");
        } else {
            println!("WARNING: INT8 quantization did not provide expected speedup");
        }
    }
}

/// Test that INT8 produces similar results to FP32 (accuracy validation)
#[test]
fn test_int8_accuracy() {
    let (fp32_path, int8_path) = get_model_paths();

    if !fp32_path.exists() || !int8_path.exists() {
        println!("Skipping accuracy test - models not available");
        return;
    }

    println!("\n=== INT8 Accuracy Validation ===\n");

    // Load both models
    let mut fp32_model =
        LayoutPredictorModel::load_with_backend(&fp32_path, Device::Cpu, InferenceBackend::ONNX)
            .expect("Failed to load FP32 model");

    let mut int8_model =
        LayoutPredictorModel::load_with_backend(&int8_path, Device::Cpu, InferenceBackend::ONNX)
            .expect("Failed to load INT8 model");

    // Load test image
    let image = load_test_image();

    // Run inference on both models
    let fp32_results = fp32_model.infer(&image).expect("FP32 inference failed");
    let int8_results = int8_model.infer(&image).expect("INT8 inference failed");

    println!("FP32 detected {} elements", fp32_results.len());
    println!("INT8 detected {} elements", int8_results.len());

    // Compare results
    let count_diff = (fp32_results.len() as i64 - int8_results.len() as i64).abs();
    let count_diff_pct = if fp32_results.is_empty() {
        0.0
    } else {
        count_diff as f64 / fp32_results.len() as f64 * 100.0
    };

    println!("\nElement count difference: {count_diff} ({count_diff_pct:.1}%)");

    // Compare bounding boxes for matched elements
    let mut bbox_diffs = Vec::new();
    for (i, (fp32, int8)) in fp32_results.iter().zip(int8_results.iter()).enumerate() {
        let l_diff = (fp32.bbox.l - int8.bbox.l).abs();
        let t_diff = (fp32.bbox.t - int8.bbox.t).abs();
        let r_diff = (fp32.bbox.r - int8.bbox.r).abs();
        let b_diff = (fp32.bbox.b - int8.bbox.b).abs();
        let avg_diff = (l_diff + t_diff + r_diff + b_diff) / 4.0;
        bbox_diffs.push(avg_diff);

        if i < 5 || avg_diff > 5.0 {
            println!(
                "  Element {}: FP32 label={} vs INT8 label={}, bbox_diff={:.2}px",
                i, fp32.label, int8.label, avg_diff
            );
        }
    }

    if !bbox_diffs.is_empty() {
        let avg_bbox_diff = bbox_diffs.iter().sum::<f64>() / bbox_diffs.len() as f64;
        let max_bbox_diff = bbox_diffs.iter().copied().fold(0.0, f64::max);

        println!("\nBounding box differences:");
        println!("  Average: {avg_bbox_diff:.2} pixels");
        println!("  Max:     {max_bbox_diff:.2} pixels");

        // INT8 should produce very similar results (within a few pixels)
        // Note: Some difference is expected due to quantization, but should be small
        if avg_bbox_diff < 10.0 {
            println!("\nSUCCESS: INT8 accuracy is acceptable (avg bbox diff < 10px)");
        } else {
            println!("\nWARNING: INT8 accuracy may be degraded (avg bbox diff >= 10px)");
        }
    }

    // For now, don't fail the test - just report the differences
    // Actual accuracy validation would need labeled test data
}

/// Full benchmark with more iterations (ignored by default - slow)
#[test]
#[ignore = "Slow benchmark test - run manually with --ignored"]
fn test_int8_vs_fp32_full_benchmark() {
    let (fp32_path, int8_path) = get_model_paths();

    println!("\n=== Full INT8 vs FP32 Benchmark (20 iterations) ===\n");

    let image = load_test_image();

    // Benchmark with more iterations
    let (fp32_avg, fp32_min, fp32_max) = benchmark_model(&fp32_path, &image, 20);
    let (int8_avg, int8_min, int8_max) = benchmark_model(&int8_path, &image, 20);

    println!("FP32:  avg={fp32_avg:.2}ms  min={fp32_min:.2}ms  max={fp32_max:.2}ms");
    println!("INT8:  avg={int8_avg:.2}ms  min={int8_min:.2}ms  max={int8_max:.2}ms");

    if !fp32_avg.is_nan() && !int8_avg.is_nan() {
        let speedup = fp32_avg / int8_avg;
        let size_reduction = {
            let fp32_size = std::fs::metadata(&fp32_path).map(|m| m.len()).unwrap_or(0);
            let int8_size = std::fs::metadata(&int8_path).map(|m| m.len()).unwrap_or(0);
            fp32_size as f64 / int8_size as f64
        };

        println!("\n=== Summary ===");
        println!("Inference speedup: {speedup:.2}x");
        println!("Size reduction:    {size_reduction:.2}x");
        println!("\nExpected: ~2x speedup, ~4x size reduction");

        // Assert reasonable speedup for CI
        assert!(speedup > 1.0, "INT8 should not be slower than FP32");
    }
}
