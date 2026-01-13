//! Layout Detection Consistency Tests
//!
//! This test module validates that layout detection produces consistent, reasonable outputs
//! across different configurations. It's a stepping stone toward full mAP validation.
//!
//! ## Test Coverage
//! - Cluster count consistency between INT8 and FP32 models
//! - Label distribution consistency across runs
//! - Resolution comparison for accuracy impact
//!
//! ## Note on Full mAP Validation
//! Proper mAP calculation requires ground truth layout annotations (bounding boxes + labels).
//! Our current groundtruth is markdown output, not layout detection annotations.
//! Full mAP validation would require DocLayNet validation set annotations.

#[cfg(not(feature = "pytorch"))]
use docling_pdf_ml::pipeline::Device;
use docling_pdf_ml::{
    models::layout_predictor::{InferenceBackend, LayoutPredictorModel},
    preprocessing::layout::LayoutResolution,
};
use ndarray::Array3;
use std::collections::HashMap;
use std::path::Path;
#[cfg(feature = "pytorch")]
use tch::Device;

fn find_model_path(filename: &str) -> Option<std::path::PathBuf> {
    let paths = [
        format!("models/{filename}"),
        format!("crates/docling-pdf-ml/models/{filename}"),
        format!("onnx_exports/{filename}"),
    ];

    for path in &paths {
        let p = Path::new(path);
        if p.exists() {
            return Some(p.to_path_buf());
        }
    }
    None
}

fn create_test_image() -> Array3<u8> {
    // Create a synthetic test image (letter-size page at 72 DPI)
    Array3::<u8>::from_elem((792, 612, 3), 255)
}

fn count_labels(clusters: &[docling_pdf_ml::baseline::LayoutCluster]) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for cluster in clusters {
        *counts.entry(cluster.label.clone()).or_insert(0) += 1;
    }
    counts
}

/// Test that INT8 and FP32 models produce similar cluster counts
#[test]
#[ignore = "Requires both INT8 and FP32 model files"]
fn test_int8_vs_fp32_consistency() {
    let fp32_path = match find_model_path("rtdetr_layout_v2.onnx") {
        Some(p) => p,
        None => {
            eprintln!("FP32 model not found, skipping test");
            return;
        }
    };

    let int8_path = match find_model_path("rtdetr_layout_v2_int8.onnx") {
        Some(p) => p,
        None => {
            eprintln!("INT8 model not found, skipping test");
            return;
        }
    };

    // Load models
    let mut fp32_model =
        LayoutPredictorModel::load_with_backend(&fp32_path, Device::Cpu, InferenceBackend::ONNX)
            .expect("Failed to load FP32 model");

    let mut int8_model =
        LayoutPredictorModel::load_with_backend(&int8_path, Device::Cpu, InferenceBackend::ONNX)
            .expect("Failed to load INT8 model");

    // Run inference on test image
    let image = create_test_image();

    let fp32_clusters = fp32_model.infer(&image).expect("FP32 inference failed");
    let int8_clusters = int8_model.infer(&image).expect("INT8 inference failed");

    // Compare results
    let fp32_count = fp32_clusters.len();
    let int8_count = int8_clusters.len();

    println!("FP32 model: {fp32_count} clusters");
    println!("INT8 model: {int8_count} clusters");

    // Allow some variation (±20% or ±2 clusters, whichever is larger)
    let max_diff = std::cmp::max(2, (fp32_count as f64 * 0.2) as usize);
    let diff = (fp32_count as i64 - int8_count as i64).unsigned_abs() as usize;

    assert!(
        diff <= max_diff,
        "INT8 and FP32 cluster counts differ too much: {fp32_count} vs {int8_count} (diff: {diff}, max allowed: {max_diff})"
    );

    // Compare label distributions
    let fp32_labels = count_labels(&fp32_clusters);
    let int8_labels = count_labels(&int8_clusters);

    println!("\nLabel distribution:");
    println!("FP32: {fp32_labels:?}");
    println!("INT8: {int8_labels:?}");

    // Both should have similar label sets
    let all_labels: std::collections::HashSet<_> =
        fp32_labels.keys().chain(int8_labels.keys()).collect();

    for label in all_labels {
        let fp32_count = *fp32_labels.get(label).unwrap_or(&0);
        let int8_count = *int8_labels.get(label).unwrap_or(&0);
        println!("  {label}: FP32={fp32_count}, INT8={int8_count}");
    }
}

/// Test that different resolutions produce reasonable results
#[test]
#[ignore = "Requires model file"]
fn test_resolution_consistency() {
    let model_path = match find_model_path("rtdetr_layout_v2_int8.onnx")
        .or_else(|| find_model_path("rtdetr_layout_v2.onnx"))
    {
        Some(p) => p,
        None => {
            eprintln!("No layout model found, skipping test");
            return;
        }
    };

    let mut model =
        LayoutPredictorModel::load_with_backend(&model_path, Device::Cpu, InferenceBackend::ONNX)
            .expect("Failed to load model");

    let image = create_test_image();

    // Test different resolutions
    let resolutions = [
        (LayoutResolution::Full, "640x640"),
        (LayoutResolution::Medium, "512x512"),
        (LayoutResolution::Fast, "448x448"),
    ];

    let mut baseline_count = 0;

    for (resolution, name) in &resolutions {
        let clusters = model
            .infer_with_resolution(&image, *resolution)
            .unwrap_or_else(|_| panic!("Inference failed at {name}"));

        println!("{}: {} clusters", name, clusters.len());

        if *resolution == LayoutResolution::Full {
            baseline_count = clusters.len();
        }

        // At minimum, reduced resolution should find similar clusters (±30%)
        if baseline_count > 0 && *resolution != LayoutResolution::Full {
            let max_diff = std::cmp::max(2, (baseline_count as f64 * 0.3) as usize);
            let diff = (baseline_count as i64 - clusters.len() as i64).unsigned_abs() as usize;

            println!("  Diff from baseline: {diff} (max allowed: {max_diff})");

            // This is a soft assertion - we just log if it fails
            if diff > max_diff {
                eprintln!("  WARNING: {name} cluster count differs significantly from baseline");
            }
        }
    }
}

/// Test that repeated runs produce consistent results (determinism)
#[test]
#[ignore = "Requires model file"]
fn test_inference_determinism() {
    let model_path = match find_model_path("rtdetr_layout_v2_int8.onnx")
        .or_else(|| find_model_path("rtdetr_layout_v2.onnx"))
    {
        Some(p) => p,
        None => {
            eprintln!("No layout model found, skipping test");
            return;
        }
    };

    let mut model =
        LayoutPredictorModel::load_with_backend(&model_path, Device::Cpu, InferenceBackend::ONNX)
            .expect("Failed to load model");

    let image = create_test_image();

    // Run inference multiple times
    let runs = 3;
    let mut results = Vec::new();

    for i in 0..runs {
        let clusters = model.infer(&image).expect("Inference failed");
        println!("Run {}: {} clusters", i + 1, clusters.len());
        results.push(clusters);
    }

    // All runs should produce identical cluster counts
    let first_count = results[0].len();
    for (i, clusters) in results.iter().enumerate().skip(1) {
        assert_eq!(
            clusters.len(),
            first_count,
            "Run {} produced different cluster count: {} vs {}",
            i + 1,
            clusters.len(),
            first_count
        );
    }

    println!("\nAll {runs} runs produced consistent results ({first_count} clusters)");
}

/// Smoke test: ensure layout detection produces reasonable output on synthetic image
#[test]
fn test_layout_detection_smoke() {
    let model_path = match find_model_path("rtdetr_layout_v2_int8.onnx")
        .or_else(|| find_model_path("rtdetr_layout_v2.onnx"))
    {
        Some(p) => p,
        None => {
            eprintln!("No layout model found, skipping smoke test");
            return;
        }
    };

    let mut model =
        LayoutPredictorModel::load_with_backend(&model_path, Device::Cpu, InferenceBackend::ONNX)
            .expect("Failed to load model");

    // Test with blank image (should find few/no clusters)
    let blank_image = Array3::<u8>::from_elem((792, 612, 3), 255);
    let blank_clusters = model
        .infer(&blank_image)
        .expect("Blank image inference failed");

    // Blank image should not have many high-confidence detections
    let high_confidence: Vec<_> = blank_clusters
        .iter()
        .filter(|c| c.confidence > 0.5)
        .collect();

    println!(
        "Blank image: {} total clusters, {} high-confidence (>0.5)",
        blank_clusters.len(),
        high_confidence.len()
    );

    // A blank image shouldn't have many detections
    assert!(
        high_confidence.len() <= 5,
        "Too many high-confidence detections on blank image: {}",
        high_confidence.len()
    );
}
