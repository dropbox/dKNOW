#![cfg(feature = "pytorch")]
// Test PyTorch backend integration with LayoutPredictorModel
//
// This test validates that the PyTorch backend can be loaded and used
// for inference through the LayoutPredictorModel interface.

use docling_pdf_ml::models::layout_predictor::pytorch_backend::weights;
use docling_pdf_ml::models::layout_predictor::{InferenceBackend, LayoutPredictorModel};
use ndarray::Array3;
use ndarray_npy::ReadNpyExt;
use std::fs::File;
use std::path::PathBuf;
use tch::Device;

#[test]
fn test_pytorch_backend_load_and_infer() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n{}", "=".repeat(80));
    println!("PyTorch Backend Integration Test");
    println!("{}", "=".repeat(80));
    println!();

    // Set environment for PyTorch
    std::env::set_var("LIBTORCH_USE_PYTORCH", "1");
    std::env::set_var("LIBTORCH_BYPASS_VERSION_CHECK", "1");

    // Use CPU for consistent results
    let device = Device::Cpu;

    // Get model path from HuggingFace cache
    let model_path = match weights::get_model_path() {
        Ok(path) => path,
        Err(e) => {
            println!("⚠️  Skipping test: {}", e);
            println!("   To fix: huggingface-cli download docling-project/docling-layout-heron");
            return Ok(());
        }
    };

    println!("1. Loading PyTorch Backend");
    println!("{}", "-".repeat(80));
    println!("   Model path: {:?}", model_path);

    // Load model with PyTorch backend
    let mut model =
        LayoutPredictorModel::load_with_backend(&model_path, device, InferenceBackend::PyTorch)?;

    println!("   ✓ Model loaded successfully");
    println!();

    // Load test image from baseline data
    println!("2. Loading Test Image");
    println!("{}", "-".repeat(80));

    let baseline_dir = PathBuf::from("baseline_data/arxiv_2206.01062/page_0/layout");
    let image_path = baseline_dir.join("stage0_image.npy");

    if !image_path.exists() {
        println!("⚠️  Skipping inference test: stage0_image.npy not found");
        println!("   Model loading succeeded, which is the primary goal of this test");
        return Ok(());
    }

    let file = File::open(&image_path)?;
    let image = Array3::<u8>::read_npy(file)?;
    println!("   Image shape: {:?}", image.shape());
    println!();

    // Run inference
    println!("3. Running Inference");
    println!("{}", "-".repeat(80));
    println!("   This may take a few seconds...");

    let clusters = model.infer(&image)?;

    println!("   ✓ Inference complete");
    println!("   Detected {} clusters", clusters.len());
    println!();

    // Validate outputs
    println!("4. Validating Outputs");
    println!("{}", "-".repeat(80));

    // Check that we got some clusters
    assert!(!clusters.is_empty(), "Expected at least 1 cluster");
    assert!(
        clusters.len() < 500,
        "Expected fewer than 500 clusters (sanity check)"
    );

    // Check first cluster has valid properties
    let first = &clusters[0];
    assert!(
        first.bbox.l >= 0.0 && first.bbox.l <= 1.0,
        "Bbox L should be normalized [0, 1]"
    );
    assert!(
        first.bbox.t >= 0.0 && first.bbox.t <= 1.0,
        "Bbox T should be normalized [0, 1]"
    );
    assert!(
        first.bbox.r >= 0.0 && first.bbox.r <= 1.0,
        "Bbox R should be normalized [0, 1]"
    );
    assert!(
        first.bbox.b >= 0.0 && first.bbox.b <= 1.0,
        "Bbox B should be normalized [0, 1]"
    );
    assert!(
        first.confidence >= 0.0 && first.confidence <= 1.0,
        "Confidence should be [0, 1]"
    );

    println!(
        "   ✓ Cluster count: {} (within expected range)",
        clusters.len()
    );
    println!(
        "   ✓ First cluster bbox: [{:.3}, {:.3}, {:.3}, {:.3}]",
        first.bbox.l, first.bbox.t, first.bbox.r, first.bbox.b
    );
    println!("   ✓ First cluster label: {:?}", first.label);
    println!("   ✓ First cluster confidence: {:.3}", first.confidence);
    println!();

    println!("{}", "=".repeat(80));
    println!("✓ PyTorch Backend Integration Test PASSED");
    println!("{}", "=".repeat(80));

    Ok(())
}
