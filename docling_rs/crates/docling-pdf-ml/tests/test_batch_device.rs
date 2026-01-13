#![cfg(feature = "pytorch")]
// Test batch processing with different devices to identify MPS crash
//
// This test reproduces the batch processing crash and tests if it's device-specific
//
// NOTE: This test requires model weights in models/model4_rt_detr_v2 which are not
// included in the repository. The test is ignored by default to prevent CI failures.
// To run: cargo test --release -- --ignored test_batch_device

use docling_pdf_ml::models::layout_predictor::{InferenceBackend, LayoutPredictorModel};
use image::ImageReader;
use ndarray::Array3;
use std::path::Path;

#[test]
#[ignore = "Requires model weights not in repository"]
fn test_batch_with_cpu() {
    // Force CPU device
    std::env::set_var("LIBTORCH_USE_PYTORCH", "1");
    std::env::set_var("LIBTORCH_BYPASS_VERSION_CHECK", "1");

    // Load model with CPU device
    let mut predictor = LayoutPredictorModel::load_with_backend(
        Path::new("models/model4_rt_detr_v2"),
        tch::Device::Cpu, // Force CPU
        InferenceBackend::PyTorch,
    )
    .expect("Failed to load model");

    // Load 2 test images
    let img1 = ImageReader::open("test_data/arxiv_2206.01062/page_0000.png")
        .expect("Failed to open test image 1")
        .decode()
        .expect("Failed to decode image 1")
        .to_rgb8();

    let img2 = ImageReader::open("test_data/arxiv_2206.01062/page_0001.png")
        .expect("Failed to open test image 2")
        .decode()
        .expect("Failed to decode image 2")
        .to_rgb8();

    // Convert to Array3
    let (w1, h1) = img1.dimensions();
    let data1: Vec<u8> = img1.into_raw();
    let arr1 = Array3::from_shape_vec((h1 as usize, w1 as usize, 3), data1)
        .expect("Failed to create array 1");

    let (w2, h2) = img2.dimensions();
    let data2: Vec<u8> = img2.into_raw();
    let arr2 = Array3::from_shape_vec((h2 as usize, w2 as usize, 3), data2)
        .expect("Failed to create array 2");

    // Test batch inference with CPU
    println!("Testing batch inference on CPU device...");
    let result = predictor.infer_batch(&[arr1, arr2]);

    match result {
        Ok(clusters_batch) => {
            println!("✓ CPU batch inference SUCCESS!");
            println!("  Page 1: {} clusters", clusters_batch[0].len());
            println!("  Page 2: {} clusters", clusters_batch[1].len());
            assert_eq!(clusters_batch.len(), 2);
        }
        Err(e) => {
            panic!("✗ CPU batch inference FAILED: {}", e);
        }
    }
}

#[test]
#[ignore = "Requires model weights not in repository"]
#[cfg(target_os = "macos")]
fn test_batch_with_mps() {
    // Test with MPS device (expected to crash)
    std::env::set_var("LIBTORCH_USE_PYTORCH", "1");
    std::env::set_var("LIBTORCH_BYPASS_VERSION_CHECK", "1");

    // Load model with MPS device
    let mut predictor = LayoutPredictorModel::load_with_backend(
        Path::new("models/model4_rt_detr_v2"),
        tch::Device::Mps, // Force MPS
        InferenceBackend::PyTorch,
    )
    .expect("Failed to load model");

    // Load 2 test images
    let img1 = ImageReader::open("test_data/arxiv_2206.01062/page_0000.png")
        .expect("Failed to open test image 1")
        .decode()
        .expect("Failed to decode image 1")
        .to_rgb8();

    let img2 = ImageReader::open("test_data/arxiv_2206.01062/page_0001.png")
        .expect("Failed to open test image 2")
        .decode()
        .expect("Failed to decode image 2")
        .to_rgb8();

    // Convert to Array3
    let (w1, h1) = img1.dimensions();
    let data1: Vec<u8> = img1.into_raw();
    let arr1 = Array3::from_shape_vec((h1 as usize, w1 as usize, 3), data1)
        .expect("Failed to create array 1");

    let (w2, h2) = img2.dimensions();
    let data2: Vec<u8> = img2.into_raw();
    let arr2 = Array3::from_shape_vec((h2 as usize, w2 as usize, 3), data2)
        .expect("Failed to create array 2");

    // Test batch inference with MPS (may crash)
    println!("Testing batch inference on MPS device...");
    let result = predictor.infer_batch(&[arr1, arr2]);

    match result {
        Ok(clusters_batch) => {
            println!("✓ MPS batch inference SUCCESS!");
            println!("  Page 1: {} clusters", clusters_batch[0].len());
            println!("  Page 2: {} clusters", clusters_batch[1].len());
            assert_eq!(clusters_batch.len(), 2);
        }
        Err(e) => {
            println!("✗ MPS batch inference FAILED: {}", e);
            // Don't panic - this is expected to fail/crash
        }
    }
}
