//! # Model Comparison Benchmark
//!
//! Compares different layout detection models on the same test set.
//!
//! ## Supported Models
//!
//! | Model | Architecture | Published mAP | Expected Speed |
//! |-------|--------------|---------------|----------------|
//! | RT-DETR (ours) | DETR | 78% | 60ms |
//! | DocLayout-YOLO | YOLOv10 | 79.7% | <10ms |
//! | PP-DocLayout-L | RT-DETR | 90.4% | 13.4ms |
//! | PP-DocLayout-S | RT-DETR | 75% | 8.1ms |
//!
//! ## Class Mapping
//!
//! DocLayNet 11 classes (used by DocLayout-YOLO):
//! - Caption, Footnote, Formula, List-item, Page-footer, Page-header,
//!   Picture, Section-header, Table, Text, Title
//!
//! Our RT-DETR 17 classes:
//! - Caption, Footnote, Formula, List-item, Page-Footer, Page-Header,
//!   Picture, Section-Header, Table, Text, Title,
//!   Document Index, Code, Checkbox-Selected, Checkbox-Unselected, Form, Key-Value Region
//!
//! Core 11 classes match directly.
//!
//! ## Run
//!
//! ```bash
//! # Quick comparison test
//! cargo test --release --test model_comparison_benchmark test_model_info -- --nocapture
//!
//! # Full benchmark (requires models)
//! cargo test --release --test model_comparison_benchmark -- --ignored --nocapture
//! ```

mod common;

use common::baseline_loaders::load_numpy_u8;
use docling_pdf_ml::baseline::LayoutCluster;
use docling_pdf_ml::models::layout_predictor::LayoutPredictorModel;
use ndarray::{Array3, Ix3};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

// Use stub Device when pytorch is disabled
#[cfg(not(feature = "pytorch"))]
use docling_pdf_ml::pipeline::Device;
#[cfg(feature = "pytorch")]
use tch::Device;

/// Model information for comparison
#[derive(Debug, Clone)]
struct ModelInfo {
    /// Model name
    name: String,
    /// Path to model file
    path: PathBuf,
    /// Model format (ONNX or PyTorch)
    format: ModelFormat,
    /// Expected mAP from published results
    published_map: f32,
    /// Expected inference time (ms)
    expected_latency_ms: f32,
    /// Input resolution (height, width)
    input_resolution: (u32, u32),
    /// Number of classes
    num_classes: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ModelFormat {
    Onnx,
    PyTorch,
}

/// DocLayNet 11 classes (standard document layout classes)
const DOCLAYNET_CLASSES: &[&str] = &[
    "Caption",
    "Footnote",
    "Formula",
    "List-item",
    "Page-footer",
    "Page-header",
    "Picture",
    "Section-header",
    "Table",
    "Text",
    "Title",
];

/// Our RT-DETR 17 classes (superset of DocLayNet + form elements)
const RTDETR_CLASSES: &[&str] = &[
    "Caption",             // 0
    "Footnote",            // 1
    "Formula",             // 2
    "List-item",           // 3
    "Page-Footer",         // 4
    "Page-Header",         // 5
    "Picture",             // 6
    "Section-Header",      // 7
    "Table",               // 8
    "Text",                // 9
    "Title",               // 10
    "Document Index",      // 11
    "Code",                // 12
    "Checkbox-Selected",   // 13
    "Checkbox-Unselected", // 14
    "Form",                // 15
    "Key-Value Region",    // 16
];

/// Get available models for comparison
fn get_available_models() -> Vec<ModelInfo> {
    let mut models = Vec::new();

    // RT-DETR ONNX (our current model)
    let rtdetr_paths = [
        "onnx_exports/layout_optimum/model.onnx",
        "crates/docling-pdf-ml/onnx_exports/layout_optimum/model.onnx",
    ];
    for path in &rtdetr_paths {
        let p = PathBuf::from(path);
        if p.exists() {
            models.push(ModelInfo {
                name: "RT-DETR (FP32)".to_string(),
                path: p,
                format: ModelFormat::Onnx,
                published_map: 78.0,
                expected_latency_ms: 60.0,
                input_resolution: (640, 640),
                num_classes: 17,
            });
            break;
        }
    }

    // RT-DETR INT8 (quantized)
    let rtdetr_int8_paths = [
        "onnx_exports/layout_optimum/model_int8.onnx",
        "crates/docling-pdf-ml/onnx_exports/layout_optimum/model_int8.onnx",
    ];
    for path in &rtdetr_int8_paths {
        let p = PathBuf::from(path);
        if p.exists() {
            models.push(ModelInfo {
                name: "RT-DETR (INT8)".to_string(),
                path: p,
                format: ModelFormat::Onnx,
                published_map: 77.5,       // ~0.5% loss from quantization
                expected_latency_ms: 34.0, // ~1.78x faster
                input_resolution: (640, 640),
                num_classes: 17,
            });
            break;
        }
    }

    // DocLayout-YOLO DocLayNet (if downloaded)
    let yolo_doclaynet_paths = [
        "models/doclayout_yolo_doclaynet.onnx",
        "crates/docling-pdf-ml/models/doclayout_yolo_doclaynet.onnx",
        "onnx_exports/doclayout_yolo/model.onnx",
    ];
    for path in &yolo_doclaynet_paths {
        let p = PathBuf::from(path);
        if p.exists() {
            models.push(ModelInfo {
                name: "DocLayout-YOLO-DocLayNet".to_string(),
                path: p,
                format: ModelFormat::Onnx,
                published_map: 79.7,
                expected_latency_ms: 10.0,
                input_resolution: (1120, 1120),
                num_classes: 11,
            });
            break;
        }
    }

    // DocLayout-YOLO DocStructBench (pre-exported ONNX from HuggingFace)
    let yolo_docstruct_paths = [
        "models/doclayout_yolo_docstructbench_imgsz1024.onnx",
        "crates/docling-pdf-ml/models/doclayout_yolo_docstructbench_imgsz1024.onnx",
    ];
    for path in &yolo_docstruct_paths {
        let p = PathBuf::from(path);
        if p.exists() {
            models.push(ModelInfo {
                name: "DocLayout-YOLO-DocStructBench".to_string(),
                path: p,
                format: ModelFormat::Onnx,
                published_map: 85.0, // Different benchmark
                expected_latency_ms: 10.0,
                input_resolution: (1024, 1024),
                num_classes: 10, // DocStructBench has 10 classes
            });
            break;
        }
    }

    // PP-DocLayout (if downloaded)
    let pp_paths = [
        "models/pp_doclayout_l.onnx",
        "onnx_exports/pp_doclayout/model.onnx",
    ];
    for path in &pp_paths {
        let p = PathBuf::from(path);
        if p.exists() {
            models.push(ModelInfo {
                name: "PP-DocLayout-L".to_string(),
                path: p,
                format: ModelFormat::Onnx,
                published_map: 90.4,
                expected_latency_ms: 13.4,
                input_resolution: (800, 800), // Approximate
                num_classes: 23,              // PP-DocLayout has more classes
            });
            break;
        }
    }

    models
}

/// Load a test page image for benchmarking (u8 format)
fn load_test_image() -> Array3<u8> {
    let baseline_paths = [
        "baseline_data/redp5110/page_0/layout/input_page_image.npy",
        "baseline_data/2305.03393v1/page_0/layout/input_page_image.npy",
    ];

    for path in &baseline_paths {
        if let Ok(img) = load_numpy_u8(Path::new(path)) {
            if let Ok(arr3) = img.into_dimensionality::<Ix3>() {
                eprintln!("Loaded test image from: {path}");
                return arr3;
            }
        }
    }

    eprintln!("Warning: No baseline data found, using synthetic 792x612 test image");
    Array3::<u8>::zeros((792, 612, 3))
}

/// Benchmark result for a single model
#[derive(Debug)]
struct BenchmarkResult {
    model_name: String,
    mean_latency_ms: f64,
    std_latency_ms: f64,
    min_latency_ms: f64,
    max_latency_ms: f64,
    cluster_count: usize,
    class_distribution: HashMap<String, usize>,
}

/// Run benchmark for a model
fn benchmark_model(
    model: &mut LayoutPredictorModel,
    image: &Array3<u8>,
    model_name: &str,
    iterations: usize,
) -> BenchmarkResult {
    // Warmup
    for _ in 0..3 {
        let _ = model.infer(image);
    }

    // Benchmark
    let mut times = Vec::with_capacity(iterations);
    let mut last_clusters = Vec::new();

    for _ in 0..iterations {
        let start = Instant::now();
        let result = model.infer(image);
        times.push(start.elapsed().as_secs_f64() * 1000.0);

        if let Ok(clusters) = result {
            last_clusters = clusters;
        }
    }

    // Calculate stats
    let mean = times.iter().sum::<f64>() / times.len() as f64;
    let variance = times.iter().map(|t| (t - mean).powi(2)).sum::<f64>() / times.len() as f64;
    let std = variance.sqrt();
    let min = times.iter().copied().fold(f64::INFINITY, f64::min);
    let max = times.iter().copied().fold(f64::NEG_INFINITY, f64::max);

    // Count classes
    let mut class_distribution = HashMap::new();
    for cluster in &last_clusters {
        *class_distribution.entry(cluster.label.clone()).or_insert(0) += 1;
    }

    BenchmarkResult {
        model_name: model_name.to_string(),
        mean_latency_ms: mean,
        std_latency_ms: std,
        min_latency_ms: min,
        max_latency_ms: max,
        cluster_count: last_clusters.len(),
        class_distribution,
    }
}

/// Print benchmark comparison table
fn print_comparison_table(results: &[BenchmarkResult]) {
    println!("\n## Model Comparison Results\n");
    println!(
        "| Model | Mean (ms) | Std (ms) | Min (ms) | Max (ms) | Clusters | Speedup vs RT-DETR |"
    );
    println!(
        "|-------|-----------|----------|----------|----------|----------|-------------------|"
    );

    let baseline_latency = results
        .iter()
        .find(|r| r.model_name.contains("RT-DETR") && r.model_name.contains("FP32"))
        .map(|r| r.mean_latency_ms)
        .unwrap_or(results[0].mean_latency_ms);

    for result in results {
        let speedup = baseline_latency / result.mean_latency_ms;
        println!(
            "| {} | {:.2} | {:.2} | {:.2} | {:.2} | {} | {:.2}x |",
            result.model_name,
            result.mean_latency_ms,
            result.std_latency_ms,
            result.min_latency_ms,
            result.max_latency_ms,
            result.cluster_count,
            speedup,
        );
    }

    println!("\n## Class Distribution\n");
    for result in results {
        println!("### {}", result.model_name);
        let mut classes: Vec<_> = result.class_distribution.iter().collect();
        classes.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
        for (class, count) in classes {
            println!("  - {class}: {count}");
        }
        println!();
    }
}

// ============================================================================
// Tests
// ============================================================================

/// Test: Show available models
#[test]
fn test_model_info() {
    println!("\n## Available Layout Detection Models\n");

    let models = get_available_models();

    if models.is_empty() {
        println!("No models found. Download models to these locations:");
        println!("  - RT-DETR: onnx_exports/layout_optimum/model.onnx");
        println!("  - DocLayout-YOLO: models/doclayout_yolo_doclaynet.onnx");
        println!("  - PP-DocLayout: models/pp_doclayout_l.onnx");
        return;
    }

    println!("| Model | Format | Classes | Resolution | Published mAP | Expected Latency |");
    println!("|-------|--------|---------|------------|---------------|------------------|");

    for model in &models {
        println!(
            "| {} | {:?} | {} | {}x{} | {:.1}% | {:.1}ms |",
            model.name,
            model.format,
            model.num_classes,
            model.input_resolution.0,
            model.input_resolution.1,
            model.published_map,
            model.expected_latency_ms,
        );
    }

    println!("\n## Class Comparison\n");
    println!("### DocLayNet Classes (11)");
    for (i, class) in DOCLAYNET_CLASSES.iter().enumerate() {
        println!("  {i}: {class}");
    }
    println!("\n### RT-DETR Classes (17)");
    for (i, class) in RTDETR_CLASSES.iter().enumerate() {
        println!("  {i}: {class}");
    }
}

/// Test: Quick benchmark of available models
#[test]
#[ignore = "Benchmark test - run with: cargo test --release --test model_comparison_benchmark -- --ignored --nocapture"]
fn test_benchmark_all_models() {
    let models = get_available_models();

    if models.is_empty() {
        eprintln!("No models available for benchmarking");
        return;
    }

    let image = load_test_image();
    let iterations = 10;
    let mut results = Vec::new();

    for model_info in &models {
        eprintln!("\nBenchmarking: {}", model_info.name);

        match LayoutPredictorModel::load(&model_info.path, Device::Cpu) {
            Ok(mut model) => {
                let result = benchmark_model(&mut model, &image, &model_info.name, iterations);
                results.push(result);
            }
            Err(e) => {
                eprintln!("Failed to load {}: {}", model_info.name, e);
            }
        }
    }

    if !results.is_empty() {
        print_comparison_table(&results);
    }
}

/// Test: Compare RT-DETR FP32 vs INT8
#[test]
#[ignore = "Benchmark test - requires model weights"]
fn test_compare_fp32_vs_int8() {
    let models = get_available_models();

    let fp32 = models.iter().find(|m| m.name.contains("FP32"));
    let int8 = models.iter().find(|m| m.name.contains("INT8"));

    if fp32.is_none() || int8.is_none() {
        eprintln!("Need both FP32 and INT8 models for comparison");
        eprintln!("FP32: {:?}", fp32.map(|m| &m.path));
        eprintln!("INT8: {:?}", int8.map(|m| &m.path));
        return;
    }

    let image = load_test_image();
    let iterations = 20;
    let mut results = Vec::new();

    for model_info in [fp32.unwrap(), int8.unwrap()] {
        eprintln!("\nBenchmarking: {}", model_info.name);
        let mut model = LayoutPredictorModel::load(&model_info.path, Device::Cpu)
            .expect("Failed to load model");
        let result = benchmark_model(&mut model, &image, &model_info.name, iterations);
        results.push(result);
    }

    print_comparison_table(&results);

    // Verify INT8 is faster
    if results.len() == 2 {
        let speedup = results[0].mean_latency_ms / results[1].mean_latency_ms;
        println!("\nINT8 speedup: {speedup:.2}x");
        assert!(speedup > 1.5, "INT8 should be at least 1.5x faster");

        // Verify cluster counts are similar (accuracy preserved)
        // Skip accuracy check if using synthetic data (both produce ~0 clusters)
        if results[0].cluster_count > 5 || results[1].cluster_count > 5 {
            let count_diff =
                (results[0].cluster_count as i32 - results[1].cluster_count as i32).abs();
            let tolerance = std::cmp::max((results[0].cluster_count as f32 * 0.1) as i32, 2);
            println!("Cluster count difference: {count_diff} (tolerance: {tolerance})");
            assert!(
                count_diff <= tolerance,
                "INT8 should have similar cluster count"
            );
        } else {
            println!(
                "Note: Synthetic test image used (low cluster count: {}/{}), skipping accuracy check",
                results[0].cluster_count, results[1].cluster_count
            );
        }
    }
}

/// Test: Validate RT-DETR model exists and loads
#[test]
fn test_rtdetr_model_loads() {
    let models = get_available_models();

    let rtdetr = models.iter().find(|m| m.name.contains("RT-DETR"));

    if let Some(model_info) = rtdetr {
        eprintln!("Loading RT-DETR from: {:?}", model_info.path);
        let model = LayoutPredictorModel::load(&model_info.path, Device::Cpu);
        assert!(model.is_ok(), "RT-DETR model should load successfully");
    } else {
        eprintln!("RT-DETR model not found at expected paths");
        eprintln!("  - onnx_exports/layout_optimum/model.onnx");
        eprintln!("  - crates/docling-pdf-ml/onnx_exports/layout_optimum/model.onnx");
    }
}

/// Test: Validate DocLayout-YOLO wrapper works directly
#[test]
fn test_doclayout_yolo_wrapper() {
    use docling_pdf_ml::models::layout_predictor::doclayout_yolo::DocLayoutYolo;

    let yolo_paths = [
        "models/doclayout_yolo_doclaynet.onnx",
        "crates/docling-pdf-ml/models/doclayout_yolo_doclaynet.onnx",
    ];

    let mut model_opt = None;
    for path in &yolo_paths {
        let p = Path::new(path);
        if p.exists() {
            if let Ok(m) = DocLayoutYolo::load(p) {
                eprintln!("Loaded DocLayout-YOLO from: {path}");
                model_opt = Some(m);
                break;
            }
        }
    }

    let mut model = match model_opt {
        Some(m) => m,
        None => {
            eprintln!("DocLayout-YOLO model not found, skipping test");
            return;
        }
    };

    // Create a realistic test image (white background, like a document)
    let mut image = Array3::<u8>::from_elem((792, 612, 3), 255);

    // Add some grayscale variation (simulated text blocks)
    for y in 100..150 {
        for x in 50..550 {
            image[[y, x, 0]] = 30;
            image[[y, x, 1]] = 30;
            image[[y, x, 2]] = 30;
        }
    }
    for y in 200..250 {
        for x in 50..550 {
            image[[y, x, 0]] = 30;
            image[[y, x, 1]] = 30;
            image[[y, x, 2]] = 30;
        }
    }

    let result = model.infer(&image);
    assert!(result.is_ok(), "YOLO inference failed: {:?}", result.err());

    let clusters = result.unwrap();
    eprintln!("DocLayout-YOLO detected {} clusters", clusters.len());

    // Print detected clusters
    for (i, cluster) in clusters.iter().take(10).enumerate() {
        eprintln!(
            "  [{:2}] {:<15} conf={:.3} bbox=({:.0},{:.0},{:.0},{:.0})",
            i,
            cluster.label,
            cluster.confidence,
            cluster.bbox.l,
            cluster.bbox.t,
            cluster.bbox.r,
            cluster.bbox.b
        );
    }

    // Verify class labels are from DocLayNet schema
    for cluster in &clusters {
        assert!(
            DOCLAYNET_CLASSES.contains(&cluster.label.as_str()),
            "Unknown class label: {}. Expected one of {:?}",
            cluster.label,
            DOCLAYNET_CLASSES
        );
    }
}

/// Test: Compare YOLO and RT-DETR outputs on same image
#[test]
#[ignore = "Benchmark - run with: cargo test --release --test model_comparison_benchmark test_compare_yolo_vs_rtdetr -- --ignored --nocapture"]
fn test_compare_yolo_vs_rtdetr() {
    use docling_pdf_ml::models::layout_predictor::doclayout_yolo::DocLayoutYolo;

    println!("\n=== DocLayout-YOLO vs RT-DETR Comparison ===\n");

    // Load both models
    let mut yolo = match DocLayoutYolo::load(Path::new("models/doclayout_yolo_doclaynet.onnx")) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Failed to load DocLayout-YOLO: {e}");
            return;
        }
    };

    let rtdetr_paths = [
        "onnx_exports/layout_optimum/model.onnx",
        "crates/docling-pdf-ml/onnx_exports/layout_optimum/model.onnx",
    ];
    let mut rtdetr = None;
    for path in &rtdetr_paths {
        let p = PathBuf::from(path);
        if p.exists() {
            if let Ok(m) = LayoutPredictorModel::load(&p, Device::Cpu) {
                rtdetr = Some(m);
                break;
            }
        }
    }
    let mut rtdetr = match rtdetr {
        Some(m) => m,
        None => {
            eprintln!("RT-DETR model not found");
            return;
        }
    };

    // Load real test image if available, otherwise use realistic synthetic
    let image = load_test_image();
    let (h, w, _) = image.dim();
    println!("Test image size: {w}x{h}");

    // Warmup runs (first run includes JIT compilation overhead)
    println!("Running warmup...");
    for _ in 0..3 {
        let _ = yolo.infer(&image);
        let _ = rtdetr.infer(&image);
    }

    // Benchmark multiple runs for stable timing
    let iterations = 5;
    let mut yolo_times = Vec::new();
    let mut rtdetr_times = Vec::new();
    let mut yolo_result = Vec::new();
    let mut rtdetr_result = Vec::new();

    for _ in 0..iterations {
        let start = Instant::now();
        yolo_result = yolo.infer(&image).expect("YOLO inference failed");
        yolo_times.push(start.elapsed().as_secs_f64() * 1000.0);

        let start = Instant::now();
        rtdetr_result = rtdetr.infer(&image).expect("RT-DETR inference failed");
        rtdetr_times.push(start.elapsed().as_secs_f64() * 1000.0);
    }

    let yolo_avg = yolo_times.iter().sum::<f64>() / iterations as f64;
    let rtdetr_avg = rtdetr_times.iter().sum::<f64>() / iterations as f64;

    println!("\n## Performance Comparison\n");
    println!("| Model | Time (ms) | Clusters |");
    println!("|-------|-----------|----------|");
    println!(
        "| DocLayout-YOLO | {:.2} | {} |",
        yolo_avg,
        yolo_result.len()
    );
    println!("| RT-DETR | {:.2} | {} |", rtdetr_avg, rtdetr_result.len());

    let speedup = rtdetr_avg / yolo_avg;
    println!("\nYOLO speedup: {speedup:.1}x faster than RT-DETR");

    // Compare class distributions
    println!("\n## Class Distribution Comparison\n");

    let mut yolo_classes: HashMap<String, usize> = HashMap::new();
    for cluster in &yolo_result {
        *yolo_classes.entry(cluster.label.clone()).or_insert(0) += 1;
    }

    let mut rtdetr_classes: HashMap<String, usize> = HashMap::new();
    for cluster in &rtdetr_result {
        *rtdetr_classes.entry(cluster.label.clone()).or_insert(0) += 1;
    }

    // Collect all unique classes
    let mut all_classes: Vec<String> = yolo_classes
        .keys()
        .chain(rtdetr_classes.keys())
        .cloned()
        .collect();
    all_classes.sort();
    all_classes.dedup();

    println!("| Class | YOLO | RT-DETR | Match |");
    println!("|-------|------|---------|-------|");
    for class in &all_classes {
        let yolo_count = *yolo_classes.get(class).unwrap_or(&0);
        let rtdetr_count = *rtdetr_classes.get(class).unwrap_or(&0);
        let match_symbol = if yolo_count == rtdetr_count {
            "✓"
        } else {
            ""
        };
        println!("| {class} | {yolo_count} | {rtdetr_count} | {match_symbol} |");
    }

    // Summary
    let common_classes: usize = DOCLAYNET_CLASSES.len();
    println!("\n## Summary\n");
    println!("- YOLO model: 11 DocLayNet classes");
    println!("- RT-DETR model: 17 classes (11 DocLayNet + 6 form classes)");
    println!("- Common classes: {common_classes}");
    println!("- YOLO total detections: {}", yolo_result.len());
    println!("- RT-DETR total detections: {}", rtdetr_result.len());

    // Note: YOLO wrapper now uses SIMD-accelerated preprocessing via `image` crate.
    // Previous naive bilinear resize took ~490ms, now using optimized resize.
    //
    // Target: <30ms total (preprocessing + inference)
    println!("\n## Status\n");
    println!("YOLO preprocessing optimized with `image` crate SIMD resize.");
    println!("Both models successfully ran inference on the test image.");
}

/// Test: Benchmark YOLO with detailed performance breakdown
#[test]
#[ignore = "Benchmark - run with: cargo test --release --test model_comparison_benchmark test_yolo_preprocessing_performance -- --ignored --nocapture"]
fn test_yolo_preprocessing_performance() {
    use docling_pdf_ml::models::layout_predictor::doclayout_yolo::DocLayoutYolo;

    println!("\n=== YOLO Performance Breakdown ===\n");

    // Load YOLO model
    let yolo_paths = [
        "models/doclayout_yolo_doclaynet.onnx",
        "crates/docling-pdf-ml/models/doclayout_yolo_doclaynet.onnx",
    ];

    let mut model = None;
    for path in &yolo_paths {
        let p = Path::new(path);
        if p.exists() {
            if let Ok(m) = DocLayoutYolo::load(p) {
                eprintln!("Loaded DocLayout-YOLO from: {path}");
                model = Some(m);
                break;
            }
        }
    }

    let mut model = match model {
        Some(m) => m,
        None => {
            eprintln!("DocLayout-YOLO model not found");
            return;
        }
    };

    // Load test image
    let image = load_test_image();
    let (h, w, _) = image.dim();
    println!("Test image size: {w}x{h}");

    // Warmup
    println!("Running warmup...");
    for _ in 0..3 {
        let _ = model.infer(&image);
    }

    // Benchmark with detailed timing
    let iterations = 10;
    let mut preprocess_times = Vec::new();
    let mut inference_times = Vec::new();
    let mut postprocess_times = Vec::new();
    let mut total_times = Vec::new();

    println!("Benchmarking {iterations} iterations with detailed timing...\n");
    for i in 0..iterations {
        let start = Instant::now();
        let result = model.infer_with_timing(&image);
        let total = start.elapsed().as_secs_f64() * 1000.0;

        match result {
            Ok((_clusters, pre, inf, post)) => {
                preprocess_times.push(pre);
                inference_times.push(inf);
                postprocess_times.push(post);
                total_times.push(total);
                if i == 0 {
                    println!(
                        "Iter {i}: pre={pre:.1}ms, infer={inf:.1}ms, post={post:.1}ms, total={total:.1}ms"
                    );
                }
            }
            Err(e) => {
                eprintln!("Iteration {i} failed: {e:?}");
            }
        }
    }

    // Calculate averages
    let avg_preprocess = preprocess_times.iter().sum::<f64>() / iterations as f64;
    let avg_inference = inference_times.iter().sum::<f64>() / iterations as f64;
    let avg_postprocess = postprocess_times.iter().sum::<f64>() / iterations as f64;
    let avg_total = total_times.iter().sum::<f64>() / iterations as f64;

    println!("\n## Performance Breakdown (average of {iterations} runs)\n");
    println!("| Stage | Time (ms) | % of Total |");
    println!("|-------|-----------|------------|");
    println!(
        "| Preprocessing | {:.2} | {:.1}% |",
        avg_preprocess,
        avg_preprocess / avg_total * 100.0
    );
    println!(
        "| ONNX Inference | {:.2} | {:.1}% |",
        avg_inference,
        avg_inference / avg_total * 100.0
    );
    println!(
        "| Post-processing | {:.2} | {:.1}% |",
        avg_postprocess,
        avg_postprocess / avg_total * 100.0
    );
    println!("| **Total** | **{avg_total:.2}** | 100% |");

    println!("\n## Key Findings\n");
    if avg_inference > avg_preprocess * 5.0 {
        println!(
            "- **ONNX inference is the bottleneck** ({:.1}% of time)",
            avg_inference / avg_total * 100.0
        );
        println!("- Preprocessing is optimized ({avg_preprocess:.2}ms)");
        println!("- Model runs 1120x1120 input on CPU - this is inherently slow");
        println!("- Published YOLO ~10ms speeds require GPU (CUDA/CoreML/Metal)");
    } else if avg_preprocess > avg_inference {
        println!(
            "- **Preprocessing is the bottleneck** ({:.1}% of time)",
            avg_preprocess / avg_total * 100.0
        );
        println!("- Consider fast_image_resize crate or GPU preprocessing");
    } else {
        println!("- Both preprocessing and inference take similar time");
        println!("- Further optimization requires GPU acceleration");
    }

    println!("\n## Comparison\n");
    println!("- RT-DETR (640x640, CPU): ~206ms");
    println!("- DocLayout-YOLO (1120x1120, CPU): ~{avg_total:.0}ms");
    let rtdetr_time = 206.0;
    if avg_total < rtdetr_time {
        println!(
            "- YOLO is {:.1}x faster than RT-DETR",
            rtdetr_time / avg_total
        );
    } else {
        println!(
            "- YOLO is {:.1}x slower than RT-DETR (larger input: 1120² vs 640²)",
            avg_total / rtdetr_time
        );
        println!("- This is expected: 1120² / 640² = 3.06x more pixels to process");
    }
}

/// Test: Compare different input resolutions for RT-DETR
///
/// Tests speed/accuracy tradeoff:
/// - 640x640 (Full): Baseline accuracy
/// - 512x512 (Medium): ~1.56x faster, ~1-2% accuracy loss
/// - 448x448 (Fast): ~2.04x faster, ~3-5% accuracy loss
#[test]
#[ignore = "Benchmark - run with: cargo test --release --test model_comparison_benchmark test_resolution_comparison -- --ignored --nocapture"]
fn test_resolution_comparison() {
    use docling_pdf_ml::preprocessing::layout::LayoutResolution;

    println!("\n=== RT-DETR Resolution Comparison ===\n");

    let rtdetr_paths = [
        "onnx_exports/layout_optimum/model_int8.onnx", // Use INT8 for faster testing
        "crates/docling-pdf-ml/onnx_exports/layout_optimum/model_int8.onnx",
        "onnx_exports/layout_optimum/model.onnx",
        "crates/docling-pdf-ml/onnx_exports/layout_optimum/model.onnx",
    ];

    let mut model_path = None;
    for path in &rtdetr_paths {
        let p = PathBuf::from(path);
        if p.exists() {
            model_path = Some(p);
            eprintln!("Using model: {path}");
            break;
        }
    }

    let model_path = match model_path {
        Some(p) => p,
        None => {
            eprintln!("RT-DETR model not found");
            return;
        }
    };

    // Load test image
    let image = load_test_image();
    let (h, w, _) = image.dim();
    println!("Test image size: {w}x{h}");

    // Load model
    let mut model =
        LayoutPredictorModel::load(&model_path, Device::Cpu).expect("Failed to load model");

    // Warmup
    println!("Running warmup...");
    for _ in 0..3 {
        let _ = model.infer(&image);
    }

    let iterations = 10;
    let resolutions = [
        (LayoutResolution::Full, "640x640 (Full)"),
        (LayoutResolution::Medium, "512x512 (Medium)"),
        (LayoutResolution::Fast, "448x448 (Fast)"),
    ];

    let mut results: Vec<(String, f64, usize, f64)> = Vec::new(); // (name, time, clusters, expected_speedup)

    for (resolution, name) in &resolutions {
        let mut times = Vec::new();
        let mut last_clusters = Vec::new();

        for _ in 0..iterations {
            let start = Instant::now();
            let clusters = model
                .infer_with_resolution(&image, *resolution)
                .expect("Inference failed");
            times.push(start.elapsed().as_secs_f64() * 1000.0);
            last_clusters = clusters;
        }

        let avg_time = times.iter().sum::<f64>() / iterations as f64;
        let expected_speedup = resolution.expected_speedup();
        results.push((
            name.to_string(),
            avg_time,
            last_clusters.len(),
            expected_speedup,
        ));

        println!(
            "{}: {:.2}ms, {} clusters (expected speedup: {:.2}x)",
            name,
            avg_time,
            last_clusters.len(),
            expected_speedup
        );
    }

    // Calculate actual speedups
    println!("\n## Results\n");
    println!(
        "| Resolution | Time (ms) | Clusters | Expected Speedup | Actual Speedup | Accuracy Delta |"
    );
    println!(
        "|------------|-----------|----------|------------------|----------------|----------------|"
    );

    let baseline_time = results[0].1;
    let baseline_clusters = results[0].2;

    for (name, time, clusters, expected_speedup) in &results {
        let actual_speedup = baseline_time / time;
        let cluster_delta = *clusters as i32 - baseline_clusters as i32;
        let delta_str = if cluster_delta >= 0 {
            format!("+{cluster_delta}")
        } else {
            cluster_delta.to_string()
        };

        println!(
            "| {name} | {time:.2} | {clusters} | {expected_speedup:.2}x | {actual_speedup:.2}x | {delta_str} |"
        );
    }

    println!("\n## Analysis\n");

    // Check if speedups match expected
    for (name, time, _clusters, expected_speedup) in &results {
        let actual_speedup = baseline_time / time;
        let speedup_ratio = actual_speedup / expected_speedup;

        if speedup_ratio < 0.8 {
            println!(
                "⚠ {name}: Actual speedup ({actual_speedup:.2}x) is lower than expected ({expected_speedup:.2}x)"
            );
            println!("  This may indicate preprocessing overhead dominating inference time.");
        } else if speedup_ratio > 1.2 {
            println!(
                "✓ {name}: Actual speedup ({actual_speedup:.2}x) exceeds expected ({expected_speedup:.2}x)"
            );
        } else {
            println!(
                "✓ {name}: Actual speedup ({actual_speedup:.2}x) matches expected ({expected_speedup:.2}x)"
            );
        }
    }

    // Recommendation
    println!("\n## Recommendation\n");
    let fast_time = results[2].1;
    let fast_clusters = results[2].2;
    let cluster_loss_pct = if baseline_clusters > 0 {
        (1.0 - fast_clusters as f64 / baseline_clusters as f64) * 100.0
    } else {
        0.0
    };

    if fast_time < 80.0 && cluster_loss_pct < 10.0 {
        println!("Consider using 448x448 (Fast) resolution for interactive use cases:");
        println!("  - Time: {fast_time:.2}ms (vs {baseline_time:.2}ms baseline)");
        println!("  - Cluster count change: {:.1}%", -cluster_loss_pct);
        println!(
            "  - Target <100ms: {}",
            if fast_time < 100.0 {
                "✓ ACHIEVED"
            } else {
                "✗ NOT MET"
            }
        );
    } else if results[1].1 < 80.0 {
        println!("Consider using 512x512 (Medium) resolution as a balanced choice.");
    } else {
        println!("Current INT8 model at {baseline_time:.2}ms is already optimized for CPU.");
    }
}

/// Test: Compare CPU vs CoreML (ANE) execution for RT-DETR
#[test]
#[ignore = "Benchmark - run with: cargo test --release --test model_comparison_benchmark test_cpu_vs_coreml -- --ignored --nocapture"]
fn test_cpu_vs_coreml() {
    println!("\n=== RT-DETR CPU vs CoreML Comparison ===\n");

    let rtdetr_paths = [
        "onnx_exports/layout_optimum/model.onnx",
        "crates/docling-pdf-ml/onnx_exports/layout_optimum/model.onnx",
    ];

    let mut model_path = None;
    for path in &rtdetr_paths {
        let p = PathBuf::from(path);
        if p.exists() {
            model_path = Some(p);
            break;
        }
    }

    let model_path = match model_path {
        Some(p) => p,
        None => {
            eprintln!("RT-DETR model not found");
            return;
        }
    };

    // Load test image
    let image = load_test_image();
    let (h, w, _) = image.dim();
    println!("Test image size: {w}x{h}");

    // Benchmark CPU
    println!("\n## CPU Backend\n");
    let mut cpu_model =
        LayoutPredictorModel::load(&model_path, Device::Cpu).expect("Failed to load CPU model");

    // Warmup
    for _ in 0..3 {
        let _ = cpu_model.infer(&image);
    }

    let iterations = 10;
    let mut cpu_times = Vec::new();
    for _ in 0..iterations {
        let start = Instant::now();
        let _ = cpu_model.infer(&image);
        cpu_times.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    let cpu_avg = cpu_times.iter().sum::<f64>() / iterations as f64;
    println!("CPU average: {cpu_avg:.2}ms");

    // Benchmark CoreML (MPS maps to CoreML in ONNX Runtime)
    println!("\n## CoreML Backend (via ONNX Runtime)\n");

    // Drop CPU model first to release memory
    drop(cpu_model);

    let coreml_result = LayoutPredictorModel::load(&model_path, Device::Mps);
    match coreml_result {
        Ok(mut coreml_model) => {
            // Warmup
            for _ in 0..3 {
                let _ = coreml_model.infer(&image);
            }

            let mut coreml_times = Vec::new();
            for _ in 0..iterations {
                let start = Instant::now();
                let _ = coreml_model.infer(&image);
                coreml_times.push(start.elapsed().as_secs_f64() * 1000.0);
            }
            let coreml_avg = coreml_times.iter().sum::<f64>() / iterations as f64;
            println!("CoreML average: {coreml_avg:.2}ms");

            println!("\n## Summary\n");
            println!("| Backend | Time (ms) | Speedup |");
            println!("|---------|-----------|---------|");
            println!("| CPU | {cpu_avg:.2} | 1.00x |");
            println!(
                "| CoreML/ANE | {:.2} | {:.2}x |",
                coreml_avg,
                cpu_avg / coreml_avg
            );

            let target = 10.0;
            if coreml_avg <= target {
                println!("\n✓ TARGET ACHIEVED: {coreml_avg:.2}ms <= {target}ms target");
            } else {
                println!("\n⚠ Below target: {coreml_avg:.2}ms > {target}ms target");
            }
        }
        Err(e) => {
            eprintln!("Failed to load CoreML model: {e}");
            eprintln!(
                "This is expected on non-Apple Silicon or if CoreML provider is not available"
            );
            println!("\n## Summary\n");
            println!("CPU average: {cpu_avg:.2}ms");
            println!("CoreML: Not available on this system");
        }
    }
}

/// Test: Find optimal thread count for ONNX Runtime
///
/// ONNX Runtime performance varies with thread count.
/// Too few: underutilizes CPU
/// Too many: overhead from thread synchronization
#[test]
#[ignore = "Benchmark - run with: cargo test --release --test model_comparison_benchmark test_thread_optimization -- --ignored --nocapture"]
fn test_thread_optimization() {
    println!("\n=== RT-DETR Thread Optimization ===\n");

    let rtdetr_paths = [
        "onnx_exports/layout_optimum/model_int8.onnx",
        "crates/docling-pdf-ml/onnx_exports/layout_optimum/model_int8.onnx",
        "onnx_exports/layout_optimum/model.onnx",
        "crates/docling-pdf-ml/onnx_exports/layout_optimum/model.onnx",
    ];

    let mut model_path = None;
    for path in &rtdetr_paths {
        let p = PathBuf::from(path);
        if p.exists() {
            model_path = Some(p);
            eprintln!("Using model: {path}");
            break;
        }
    }

    let model_path = match model_path {
        Some(p) => p,
        None => {
            eprintln!("RT-DETR model not found");
            return;
        }
    };

    // Load test image
    let image = load_test_image();
    let (h, w, _) = image.dim();
    println!("Test image size: {w}x{h}");

    // Get system info
    let available_parallelism = std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(8);
    println!("Available parallelism: {available_parallelism} (logical cores)");

    let thread_counts = [1, 2, 4, 6, 8];
    let iterations = 10;
    let mut results: Vec<(usize, f64)> = Vec::new();

    for &threads in &thread_counts {
        if threads > available_parallelism {
            continue;
        }

        // Set thread count via environment variable
        std::env::set_var("LAYOUT_ONNX_THREADS", threads.to_string());

        // Load model with new thread configuration
        let mut model = match LayoutPredictorModel::load(&model_path, Device::Cpu) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("Failed to load model with {threads} threads: {e}");
                continue;
            }
        };

        // Warmup
        for _ in 0..3 {
            let _ = model.infer(&image);
        }

        // Benchmark
        let mut times = Vec::new();
        for _ in 0..iterations {
            let start = Instant::now();
            let _ = model.infer(&image);
            times.push(start.elapsed().as_secs_f64() * 1000.0);
        }

        let avg_time = times.iter().sum::<f64>() / iterations as f64;
        results.push((threads, avg_time));

        println!("{threads} threads: {avg_time:.2}ms");
    }

    // Clear environment variable
    std::env::remove_var("LAYOUT_ONNX_THREADS");

    // Find optimal
    println!("\n## Results\n");
    println!("| Threads | Time (ms) | Speedup vs 1 Thread |");
    println!("|---------|-----------|---------------------|");

    let baseline_time = results
        .iter()
        .find(|(t, _)| *t == 1)
        .map(|(_, time)| *time)
        .unwrap_or(results[0].1);

    let mut best_threads = 1;
    let mut best_time = baseline_time;

    for (threads, time) in &results {
        let speedup = baseline_time / time;
        println!("| {threads} | {time:.2} | {speedup:.2}x |");

        if *time < best_time {
            best_time = *time;
            best_threads = *threads;
        }
    }

    println!("\n## Recommendation\n");
    println!("Optimal thread count: {best_threads} ({best_time:.2}ms)");

    if best_threads > 1 {
        let speedup = baseline_time / best_time;
        println!("Speedup over single-threaded: {speedup:.2}x");
    }

    // Check if current default is optimal
    let default_threads = (available_parallelism / 2).clamp(1, 8);
    if let Some((_, default_time)) = results.iter().find(|(t, _)| *t == default_threads) {
        let diff_pct = (default_time - best_time) / best_time * 100.0;
        if diff_pct.abs() < 5.0 {
            println!("Default ({default_threads} threads) is within 5% of optimal.");
        } else {
            println!(
                "Consider changing default from {default_threads} to {best_threads} threads ({diff_pct:.1}% improvement)."
            );
        }
    }
}

/// Test: Quality comparison between CoreML, ONNX YOLO, and RT-DETR on real PDFs
///
/// Compares detection quality using IoU and label agreement metrics.
#[test]
#[ignore = "Benchmark - run with: cargo test --release --test model_comparison_benchmark test_quality_comparison_coreml_vs_onnx_vs_rtdetr -- --ignored --nocapture --features coreml"]
fn test_quality_comparison_coreml_vs_onnx_vs_rtdetr() {
    #[cfg(feature = "coreml")]
    use docling_pdf_ml::models::layout_predictor::coreml_backend::DocLayoutYoloCoreML;
    use docling_pdf_ml::models::layout_predictor::doclayout_yolo::DocLayoutYolo;

    println!("\n=== Quality Comparison: CoreML vs ONNX YOLO vs RT-DETR ===\n");

    // Load ONNX YOLO
    let onnx_yolo_paths = [
        "models/doclayout_yolo_doclaynet.onnx",
        "crates/docling-pdf-ml/models/doclayout_yolo_doclaynet.onnx",
    ];
    let mut onnx_yolo = None;
    for path in &onnx_yolo_paths {
        let p = Path::new(path);
        if p.exists() {
            if let Ok(m) = DocLayoutYolo::load(p) {
                eprintln!("Loaded ONNX YOLO from: {path}");
                onnx_yolo = Some(m);
                break;
            }
        }
    }

    // Load CoreML YOLO
    #[cfg(feature = "coreml")]
    let mut coreml_yolo = None;
    #[cfg(feature = "coreml")]
    {
        let coreml_paths = [
            "models/doclayout_yolo_doclaynet_fixed.mlmodel",
            "models/doclayout_yolo_doclaynet.mlmodel",
        ];
        for path in &coreml_paths {
            let p = Path::new(path);
            if p.exists() {
                if let Ok(m) = DocLayoutYoloCoreML::load(p) {
                    eprintln!("Loaded CoreML YOLO from: {}", path);
                    coreml_yolo = Some(m);
                    break;
                }
            }
        }
    }

    // Load RT-DETR
    let rtdetr_paths = [
        "onnx_exports/layout_optimum/model.onnx",
        "crates/docling-pdf-ml/onnx_exports/layout_optimum/model.onnx",
    ];
    let mut rtdetr = None;
    for path in &rtdetr_paths {
        let p = PathBuf::from(path);
        if p.exists() {
            if let Ok(m) = LayoutPredictorModel::load(&p, Device::Cpu) {
                eprintln!("Loaded RT-DETR from: {path}");
                rtdetr = Some(m);
                break;
            }
        }
    }

    // Check what models we have
    let have_onnx = onnx_yolo.is_some();
    #[cfg(feature = "coreml")]
    let have_coreml = coreml_yolo.is_some();
    #[cfg(not(feature = "coreml"))]
    let have_coreml = false;
    let have_rtdetr = rtdetr.is_some();

    println!("\n## Models Available\n");
    println!("- ONNX YOLO: {}", if have_onnx { "✓" } else { "✗" });
    println!("- CoreML YOLO: {}", if have_coreml { "✓" } else { "✗" });
    println!("- RT-DETR: {}", if have_rtdetr { "✓" } else { "✗" });

    if !have_onnx && !have_coreml && !have_rtdetr {
        eprintln!("No models available for comparison");
        return;
    }

    // Load test image
    let image = load_test_image();
    let (h, w, _) = image.dim();
    println!("\n## Test Image\n");
    println!("Size: {w}x{h}");

    // Run inference on each model
    println!("\n## Running Inference...\n");

    let mut onnx_result: Vec<LayoutCluster> = Vec::new();
    #[allow(unused_mut, reason = "CoreML feature conditionally assigns to this")]
    let mut coreml_result: Vec<LayoutCluster> = Vec::new();
    let mut rtdetr_result: Vec<LayoutCluster> = Vec::new();

    let mut onnx_time = 0.0;
    #[allow(unused_mut, reason = "CoreML feature conditionally assigns to this")]
    let mut coreml_time = 0.0;
    let mut rtdetr_time = 0.0;

    // Warmup and run
    if let Some(ref mut model) = onnx_yolo {
        for _ in 0..2 {
            let _ = model.infer(&image);
        }
        let start = Instant::now();
        onnx_result = model.infer(&image).unwrap_or_default();
        onnx_time = start.elapsed().as_secs_f64() * 1000.0;
    }

    #[cfg(feature = "coreml")]
    if let Some(ref mut model) = coreml_yolo {
        for _ in 0..2 {
            let _ = model.infer(&image);
        }
        let start = Instant::now();
        coreml_result = model.infer(&image).unwrap_or_default();
        coreml_time = start.elapsed().as_secs_f64() * 1000.0;
    }

    if let Some(ref mut model) = rtdetr {
        for _ in 0..2 {
            let _ = model.infer(&image);
        }
        let start = Instant::now();
        rtdetr_result = model.infer(&image).unwrap_or_default();
        rtdetr_time = start.elapsed().as_secs_f64() * 1000.0;
    }

    // Print results
    println!("## Performance\n");
    println!("| Model | Time (ms) | Detections |");
    println!("|-------|-----------|------------|");
    if have_onnx {
        println!("| ONNX YOLO | {:.1} | {} |", onnx_time, onnx_result.len());
    }
    if have_coreml {
        println!(
            "| CoreML YOLO | {:.1} | {} |",
            coreml_time,
            coreml_result.len()
        );
    }
    if have_rtdetr {
        println!("| RT-DETR | {:.1} | {} |", rtdetr_time, rtdetr_result.len());
    }

    // Compare CoreML vs ONNX (same model, should have very similar results)
    #[cfg(feature = "coreml")]
    if have_coreml && have_onnx {
        println!("\n## CoreML vs ONNX YOLO Quality\n");
        let (agreement, avg_iou) = compare_detections(&coreml_result, &onnx_result, 0.5);
        println!("- Label agreement: {:.1}%", agreement * 100.0);
        println!("- Average IoU (matched boxes): {:.3}", avg_iou);
        if agreement > 0.9 && avg_iou > 0.8 {
            println!("✓ CoreML and ONNX produce consistent results");
        } else {
            println!("⚠ Significant difference between CoreML and ONNX");
        }
    }

    // Compare YOLO vs RT-DETR (different models, expect different results)
    if have_onnx && have_rtdetr {
        println!("\n## ONNX YOLO vs RT-DETR Quality\n");
        let (agreement, avg_iou) = compare_detections(&onnx_result, &rtdetr_result, 0.5);
        println!("- Label agreement: {:.1}%", agreement * 100.0);
        println!("- Average IoU (matched boxes): {avg_iou:.3}");
        println!("Note: Different model architectures may produce different detections.");
    }

    // Print class distributions
    println!("\n## Class Distribution\n");

    if have_onnx {
        println!("### ONNX YOLO");
        print_class_distribution(&onnx_result);
    }
    #[cfg(feature = "coreml")]
    if have_coreml {
        println!("### CoreML YOLO");
        print_class_distribution(&coreml_result);
    }
    if have_rtdetr {
        println!("### RT-DETR");
        print_class_distribution(&rtdetr_result);
    }

    // Summary
    println!("\n## Summary\n");
    if have_coreml && have_onnx {
        let speedup = onnx_time / coreml_time;
        println!("CoreML speedup over ONNX: {speedup:.1}x");
        assert!(speedup > 1.5, "Expected CoreML to be faster than ONNX CPU");
    }
}

/// Compare two sets of detections and return (label_agreement, avg_iou)
fn compare_detections(
    a: &[docling_pdf_ml::baseline::LayoutCluster],
    b: &[docling_pdf_ml::baseline::LayoutCluster],
    iou_threshold: f64,
) -> (f64, f64) {
    if a.is_empty() || b.is_empty() {
        return (0.0, 0.0);
    }

    let mut matched = 0;
    let mut label_matches = 0;
    let mut total_iou = 0.0;

    // For each detection in a, find best match in b
    for det_a in a {
        let mut best_iou = 0.0;
        let mut best_label_match = false;

        for det_b in b {
            let iou = compute_bbox_iou(&det_a.bbox, &det_b.bbox);
            if iou > best_iou {
                best_iou = iou;
                best_label_match = normalize_label(&det_a.label) == normalize_label(&det_b.label);
            }
        }

        if best_iou >= iou_threshold {
            matched += 1;
            total_iou += best_iou;
            if best_label_match {
                label_matches += 1;
            }
        }
    }

    let agreement = if matched > 0 {
        label_matches as f64 / matched as f64
    } else {
        0.0
    };
    let avg_iou = if matched > 0 {
        total_iou / matched as f64
    } else {
        0.0
    };

    (agreement, avg_iou)
}

/// Compute IoU between two bounding boxes
fn compute_bbox_iou(a: &docling_pdf_ml::baseline::BBox, b: &docling_pdf_ml::baseline::BBox) -> f64 {
    let x1 = a.l.max(b.l);
    let y1 = a.t.max(b.t);
    let x2 = a.r.min(b.r);
    let y2 = a.b.min(b.b);

    let intersection = (x2 - x1).max(0.0) * (y2 - y1).max(0.0);
    let area_a = (a.r - a.l) * (a.b - a.t);
    let area_b = (b.r - b.l) * (b.b - b.t);
    let union = area_a + area_b - intersection;

    if union > 0.0 {
        intersection / union
    } else {
        0.0
    }
}

/// Normalize label for comparison (handle case differences)
fn normalize_label(label: &str) -> String {
    // Map RT-DETR labels to YOLO labels where they differ in case
    match label {
        "Page-Footer" => "Page-footer".to_string(),
        "Page-Header" => "Page-header".to_string(),
        "Section-Header" => "Section-header".to_string(),
        other => other.to_string(),
    }
}

/// Print class distribution for a set of detections
fn print_class_distribution(detections: &[docling_pdf_ml::baseline::LayoutCluster]) {
    let mut class_counts: HashMap<String, usize> = HashMap::new();
    for det in detections {
        *class_counts.entry(det.label.clone()).or_insert(0) += 1;
    }
    let mut classes: Vec<_> = class_counts.iter().collect();
    classes.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    for (class, count) in classes {
        println!("  - {class}: {count}");
    }
}

/// Test: Cascade Layout Predictor Benchmark
///
/// Tests the 3-tier cascade approach:
/// 1. Heuristic: ~1ms (text-only pages)
/// 2. CoreML YOLO: ~74ms (ANE accelerated)
/// 3. RT-DETR: ~120ms (fallback for complex pages)
#[test]
#[ignore = "Benchmark - run with: cargo test --release --test model_comparison_benchmark test_cascade_layout_benchmark -- --ignored --nocapture --features coreml"]
fn test_cascade_layout_benchmark() {
    use docling_pdf_ml::CascadeStats;

    println!("\n=== Cascade Layout Predictor Benchmark ===\n");

    // Simulate different page types and their processing times
    // Based on N=3523 measurements
    struct SimulatedPage {
        name: &'static str,
        text_only: bool,      // Can use heuristic (1ms)
        simple_layout: bool,  // Can use YOLO (74ms CoreML)
        complex_layout: bool, // Needs RT-DETR (120ms)
    }

    let pages = vec![
        SimulatedPage {
            name: "Text paragraph",
            text_only: true,
            simple_layout: false,
            complex_layout: false,
        },
        SimulatedPage {
            name: "Simple table",
            text_only: false,
            simple_layout: true,
            complex_layout: false,
        },
        SimulatedPage {
            name: "Multi-column",
            text_only: false,
            simple_layout: true,
            complex_layout: false,
        },
        SimulatedPage {
            name: "Complex form",
            text_only: false,
            simple_layout: false,
            complex_layout: true,
        },
        SimulatedPage {
            name: "Title page",
            text_only: true,
            simple_layout: false,
            complex_layout: false,
        },
        SimulatedPage {
            name: "Image with caption",
            text_only: false,
            simple_layout: true,
            complex_layout: false,
        },
        SimulatedPage {
            name: "Mixed elements",
            text_only: false,
            simple_layout: false,
            complex_layout: true,
        },
        SimulatedPage {
            name: "Bibliography",
            text_only: true,
            simple_layout: false,
            complex_layout: false,
        },
        SimulatedPage {
            name: "Figure page",
            text_only: false,
            simple_layout: true,
            complex_layout: false,
        },
        SimulatedPage {
            name: "Index page",
            text_only: true,
            simple_layout: false,
            complex_layout: false,
        },
    ];

    // Timing constants (from N=3523 benchmarks)
    const HEURISTIC_MS: f64 = 1.0;
    const COREML_MS: f64 = 74.0;
    const RTDETR_MS: f64 = 120.0;
    const ONNX_YOLO_MS: f64 = 516.0;

    println!("## Page Types\n");
    println!("| Page | Type | Processing |");
    println!("|------|------|------------|");
    for page in &pages {
        let ptype = if page.text_only {
            "Text-only"
        } else if page.simple_layout {
            "Simple"
        } else {
            "Complex"
        };
        let proc = if page.text_only {
            "Heuristic"
        } else if page.simple_layout {
            "CoreML/YOLO"
        } else {
            "RT-DETR"
        };
        println!("| {} | {} | {} |", page.name, ptype, proc);
    }

    // Calculate cascade timing
    let mut stats = CascadeStats::default();
    let mut total_cascade_ms = 0.0;
    let mut total_rtdetr_only_ms = 0.0;
    let mut total_onnx_yolo_only_ms = 0.0;

    for page in &pages {
        total_rtdetr_only_ms += RTDETR_MS;
        total_onnx_yolo_only_ms += ONNX_YOLO_MS;

        if page.text_only {
            stats.heuristic_count += 1;
            stats.heuristic_time_us += (HEURISTIC_MS * 1000.0) as u64;
            total_cascade_ms += HEURISTIC_MS;
        } else if page.simple_layout {
            stats.coreml_count += 1;
            stats.coreml_time_us += (COREML_MS * 1000.0) as u64;
            total_cascade_ms += COREML_MS;
        } else {
            stats.ml_count += 1;
            stats.ml_time_us += (RTDETR_MS * 1000.0) as u64;
            total_cascade_ms += RTDETR_MS;
        }
    }

    println!("\n## Cascade Statistics\n");
    println!(
        "- Heuristic pages: {} ({:.0}%)",
        stats.heuristic_count,
        stats.heuristic_percentage()
    );
    println!(
        "- CoreML pages: {} ({:.0}%)",
        stats.coreml_count,
        stats.coreml_percentage()
    );
    println!(
        "- RT-DETR pages: {} ({:.0}%)",
        stats.ml_count,
        100.0 - stats.heuristic_percentage() - stats.coreml_percentage()
    );
    println!("- Fast path: {:.0}%", stats.fast_path_percentage());

    println!("\n## Performance Comparison\n");
    println!("| Approach | Total Time (ms) | Per-Page Avg | Speedup |");
    println!("|----------|-----------------|--------------|---------|");
    println!(
        "| RT-DETR only | {:.0} | {:.0} | 1.00x |",
        total_rtdetr_only_ms,
        total_rtdetr_only_ms / 10.0
    );
    println!(
        "| ONNX YOLO only | {:.0} | {:.0} | {:.2}x |",
        total_onnx_yolo_only_ms,
        total_onnx_yolo_only_ms / 10.0,
        total_rtdetr_only_ms / total_onnx_yolo_only_ms
    );
    println!(
        "| **Cascade (with CoreML)** | **{:.0}** | **{:.0}** | **{:.2}x** |",
        total_cascade_ms,
        total_cascade_ms / 10.0,
        total_rtdetr_only_ms / total_cascade_ms
    );

    let speedup = stats.speedup_factor();
    println!("\n## CascadeStats Calculations\n");
    println!("- Calculated speedup: {speedup:.2}x");
    println!(
        "- Estimated time saved: {:.0}ms",
        stats.estimated_time_saved_ms()
    );

    println!("\n## Key Insights\n");
    println!(
        "1. **Heuristic fast-path** (text-only pages): {:.0}x faster than RT-DETR",
        RTDETR_MS / HEURISTIC_MS
    );
    println!(
        "2. **CoreML ANE** (simple layouts): {:.1}x faster than RT-DETR",
        RTDETR_MS / COREML_MS
    );
    println!(
        "3. **Cascade speedup**: {:.1}x overall for mixed workloads",
        total_rtdetr_only_ms / total_cascade_ms
    );
    println!(
        "4. **Best case**: 100% text-only → {:.0}x speedup",
        RTDETR_MS / HEURISTIC_MS
    );
    println!("5. **Worst case**: 100% complex → 1.0x (fallback to RT-DETR)");

    // Validate cascade stats calculations
    assert!(
        speedup > 1.5,
        "Cascade should provide >1.5x speedup for mixed workload: {speedup:.2}"
    );
    assert!(
        stats.fast_path_percentage() > 70.0,
        "Fast path should handle >70% of pages"
    );

    println!("\n✓ Cascade benchmark complete");
}
