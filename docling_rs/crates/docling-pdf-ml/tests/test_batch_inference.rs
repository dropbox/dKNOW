#![cfg(feature = "pytorch")]
// Test batch inference produces same results as sequential inference

use docling_pdf_ml::models::layout_predictor::{InferenceBackend, LayoutPredictorModel};
use ndarray::Array3;
use std::path::PathBuf;
use tch::Device;

/// Load test images from baseline data
fn load_test_images() -> Vec<(usize, Array3<u8>)> {
    let mut images = Vec::new();

    // Load arxiv pages 0 and 1
    for page_no in 0..2 {
        let image_path = PathBuf::from(format!(
            "baseline_data/arxiv_2206.01062/stage0_page{}_image.npy",
            page_no
        ));

        if !image_path.exists() {
            eprintln!("Warning: Test image not found: {:?}", image_path);
            continue;
        }

        let image: Array3<u8> =
            ndarray_npy::read_npy(&image_path).expect("Failed to load test image");

        images.push((page_no, image));
    }

    images
}

#[test]
#[ignore = "Requires model weights and baseline data"]
fn test_batch_inference_matches_sequential() {
    // Set environment for PyTorch
    std::env::set_var("LIBTORCH_USE_PYTORCH", "1");
    std::env::set_var("LIBTORCH_BYPASS_VERSION_CHECK", "1");

    // Load model with PyTorch backend
    let model_path = PathBuf::from(
        std::env::var("HOME").unwrap() + "/.cache/huggingface/hub/models--PekingU--rtdetr_r50vd_coco_o365/snapshots/457857cec8ac28ddede40ecee9eed2beca321af8/model.safetensors"
    );
    if !model_path.exists() {
        eprintln!("Model not found: {:?}", model_path);
        eprintln!("Skipping test");
        return;
    }

    let device = Device::cuda_if_available();
    let mut model =
        LayoutPredictorModel::load_with_backend(&model_path, device, InferenceBackend::PyTorch)
            .expect("Failed to load model");

    // Load test images
    let test_images = load_test_images();
    if test_images.len() < 2 {
        eprintln!("Need at least 2 test images, found {}", test_images.len());
        eprintln!("Skipping test");
        return;
    }

    println!("Testing batch inference with {} images", test_images.len());

    // Run sequential inference
    println!("\n=== Sequential Inference ===");
    let mut sequential_results = Vec::new();
    for (page_no, image) in &test_images {
        let clusters = model.infer(image).expect("Sequential inference failed");
        println!("Page {}: {} clusters", page_no, clusters.len());
        sequential_results.push(clusters);
    }

    // Run batch inference
    println!("\n=== Batch Inference ===");
    let images_only: Vec<Array3<u8>> = test_images.iter().map(|(_, img)| img.clone()).collect();
    let batch_results = model
        .infer_batch(&images_only)
        .expect("Batch inference failed");

    // Compare results
    println!("\n=== Comparison ===");
    assert_eq!(
        batch_results.len(),
        sequential_results.len(),
        "Batch and sequential should return same number of results"
    );

    for (i, (batch_clusters, sequential_clusters)) in batch_results
        .iter()
        .zip(sequential_results.iter())
        .enumerate()
    {
        println!(
            "Page {}: batch={} clusters, sequential={} clusters",
            i,
            batch_clusters.len(),
            sequential_clusters.len()
        );

        assert_eq!(
            batch_clusters.len(),
            sequential_clusters.len(),
            "Page {}: cluster count mismatch",
            i
        );

        // Compare each cluster
        for (j, (batch_cluster, seq_cluster)) in batch_clusters
            .iter()
            .zip(sequential_clusters.iter())
            .enumerate()
        {
            // Check label
            assert_eq!(
                batch_cluster.label, seq_cluster.label,
                "Page {} cluster {}: label mismatch",
                i, j
            );

            // Check bbox (allow small floating point error)
            let bbox_diff = (batch_cluster.bbox.l - seq_cluster.bbox.l).abs()
                + (batch_cluster.bbox.t - seq_cluster.bbox.t).abs()
                + (batch_cluster.bbox.r - seq_cluster.bbox.r).abs()
                + (batch_cluster.bbox.b - seq_cluster.bbox.b).abs();

            assert!(
                bbox_diff < 0.01,
                "Page {} cluster {}: bbox diff {} > 0.01",
                i,
                j,
                bbox_diff
            );

            // Check confidence (allow small error)
            let conf_diff = (batch_cluster.confidence - seq_cluster.confidence).abs();
            assert!(
                conf_diff < 0.001,
                "Page {} cluster {}: confidence diff {} > 0.001",
                i,
                j,
                conf_diff
            );
        }
    }

    println!("\n✅ Batch inference matches sequential inference");
}

#[test]
#[ignore = "Requires model weights"]
fn test_batch_inference_performance() {
    use std::time::Instant;

    // Set environment for PyTorch
    std::env::set_var("LIBTORCH_USE_PYTORCH", "1");
    std::env::set_var("LIBTORCH_BYPASS_VERSION_CHECK", "1");
    std::env::set_var("PROFILE_MODEL", "1"); // Enable profiling

    // Load model with PyTorch backend
    let model_path = PathBuf::from(
        std::env::var("HOME").unwrap() + "/.cache/huggingface/hub/models--PekingU--rtdetr_r50vd_coco_o365/snapshots/457857cec8ac28ddede40ecee9eed2beca321af8/model.safetensors"
    );
    if !model_path.exists() {
        eprintln!("Model not found: {:?}", model_path);
        eprintln!("Skipping test");
        return;
    }

    let device = Device::cuda_if_available();
    let mut model =
        LayoutPredictorModel::load_with_backend(&model_path, device, InferenceBackend::PyTorch)
            .expect("Failed to load model");

    // Load test images
    let test_images = load_test_images();
    if test_images.len() < 2 {
        eprintln!("Need at least 2 test images, found {}", test_images.len());
        eprintln!("Skipping test");
        return;
    }

    let batch_size = test_images.len();
    println!("Performance test with batch size: {}", batch_size);

    // Warmup
    println!("\n=== Warmup ===");
    for (_, image) in &test_images {
        let _ = model.infer(image);
    }

    // Benchmark sequential inference
    println!("\n=== Sequential Inference (5 iterations) ===");
    let mut sequential_times = Vec::new();
    for iter in 0..5 {
        let start = Instant::now();
        for (page_no, image) in &test_images {
            let clusters = model.infer(image).unwrap();
            if iter == 0 {
                println!("  Page {}: {} clusters", page_no, clusters.len());
            }
        }
        let elapsed = start.elapsed();
        sequential_times.push(elapsed);
        println!(
            "  Iteration {}: {:.2} ms total ({:.2} ms/page)",
            iter,
            elapsed.as_secs_f64() * 1000.0,
            elapsed.as_secs_f64() * 1000.0 / batch_size as f64
        );
    }

    // Benchmark batch inference
    println!("\n=== Batch Inference (5 iterations) ===");
    let images_only: Vec<Array3<u8>> = test_images.iter().map(|(_, img)| img.clone()).collect();

    let mut batch_times = Vec::new();
    for iter in 0..5 {
        let start = Instant::now();
        let batch_results = model.infer_batch(&images_only).unwrap();
        let elapsed = start.elapsed();
        batch_times.push(elapsed);

        if iter == 0 {
            for (i, clusters) in batch_results.iter().enumerate() {
                println!("  Page {}: {} clusters", i, clusters.len());
            }
        }

        println!(
            "  Iteration {}: {:.2} ms total ({:.2} ms/page)",
            iter,
            elapsed.as_secs_f64() * 1000.0,
            elapsed.as_secs_f64() * 1000.0 / batch_size as f64
        );
    }

    // Calculate statistics
    let seq_mean = sequential_times
        .iter()
        .map(|d| d.as_secs_f64())
        .sum::<f64>()
        / sequential_times.len() as f64;
    let batch_mean =
        batch_times.iter().map(|d| d.as_secs_f64()).sum::<f64>() / batch_times.len() as f64;

    let seq_min = sequential_times
        .iter()
        .map(|d| d.as_secs_f64())
        .fold(f64::INFINITY, f64::min);
    let batch_min = batch_times
        .iter()
        .map(|d| d.as_secs_f64())
        .fold(f64::INFINITY, f64::min);

    println!("\n=== Performance Summary ===");
    println!("Sequential:");
    println!(
        "  Mean: {:.2} ms total ({:.2} ms/page)",
        seq_mean * 1000.0,
        seq_mean * 1000.0 / batch_size as f64
    );
    println!(
        "  Min:  {:.2} ms total ({:.2} ms/page)",
        seq_min * 1000.0,
        seq_min * 1000.0 / batch_size as f64
    );

    println!("Batch:");
    println!(
        "  Mean: {:.2} ms total ({:.2} ms/page)",
        batch_mean * 1000.0,
        batch_mean * 1000.0 / batch_size as f64
    );
    println!(
        "  Min:  {:.2} ms total ({:.2} ms/page)",
        batch_min * 1000.0,
        batch_min * 1000.0 / batch_size as f64
    );

    println!("Speedup:");
    println!("  Mean: {:.2}x", seq_mean / batch_mean);
    println!("  Min:  {:.2}x", seq_min / batch_min);

    // We expect at least some improvement from batching
    // Conservative check: batch should not be slower than sequential
    assert!(batch_mean <= seq_mean * 1.1,
        "Batch inference should not be significantly slower than sequential (batch: {:.2} ms, seq: {:.2} ms)",
        batch_mean * 1000.0, seq_mean * 1000.0
    );
}
