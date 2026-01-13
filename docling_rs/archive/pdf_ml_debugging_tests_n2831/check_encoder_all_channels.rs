#![cfg(feature = "pytorch")]
mod common;

use ndarray::ArrayD;
use std::path::PathBuf;
/// Check if encoder output matches Python for ALL 256 channels, not just first 10
///
/// Maybe the issue is that some channels beyond the first 10 are wrong
use tch::{Device, Tensor};

// Helper function to convert ndarray to tch::Tensor
fn numpy_to_tensor(arr: &ArrayD<f32>, device: Device) -> Tensor {
    let shape: Vec<i64> = arr.shape().iter().map(|&x| x as i64).collect();
    let data: Vec<f32> = arr.iter().copied().collect();
    Tensor::from_slice(&data).to(device).reshape(&shape)
}

#[test]
#[ignore]
fn check_all_encoder_channels() {
    println!("\n{}", "=".repeat(80));
    println!("CHECK ALL 256 ENCODER OUTPUT CHANNELS");
    println!("{}", "=".repeat(80));

    let device = Device::Cpu;
    let home = std::env::var("HOME").unwrap();
    let base_path =
        PathBuf::from(&home).join("docling_debug_pdf_parsing/ml_model_inputs/tableformer");

    // Load model
    println!("\nLoading model...");
    let model_dir = PathBuf::from(&home)
        .join(".cache/huggingface/hub/models--ds4sd--docling-models/snapshots/fc0f2d45e2218ea24bce5045f58a389aed16dc23/model_artifacts/tableformer/accurate");
    let model =
        docling_pdf_ml::models::table_structure::TableStructureModel::load(&model_dir, device)
            .expect("Failed to load model");

    // Load input
    println!("Loading input...");
    let input_path = base_path.join("table_0_preprocessed_input.npy");
    let input_arr =
        common::baseline_loaders::load_numpy(&input_path).expect("Failed to load input");
    let input_tensor = numpy_to_tensor(&input_arr, device);

    // Load expected encoder output
    println!("Loading Python encoder output...");
    let encoder_output_path = base_path.join("table_0_encoder_output.npy");
    let expected_arr = common::baseline_loaders::load_numpy(&encoder_output_path)
        .expect("Failed to load expected encoder output");
    let expected_tensor = numpy_to_tensor(&expected_arr, device);

    println!(
        "Expected encoder output shape: {:?}",
        expected_tensor.size()
    );

    // Run Rust encoder
    println!("Running Rust encoder...");
    let encoder_out = model.encoder.forward(&input_tensor);

    println!("Rust encoder output shape (BHWC): {:?}", encoder_out.size());

    // Convert to BCHW
    let encoder_bchw = encoder_out.permute([0, 3, 1, 2]);
    println!("After permute to BCHW: {:?}", encoder_bchw.size());

    // Check ALL 256 channels at position [0,:,0,0]
    println!("\nChecking all 256 channels at position [0,:,0,0]:");
    let mut max_diff = 0.0_f64;
    let mut mismatches = 0;

    for c in 0..256 {
        let rust_val = encoder_bchw.double_value(&[0, c, 0, 0]);
        let python_val = expected_tensor.double_value(&[0, c, 0, 0]);
        let diff = (rust_val - python_val).abs();

        if diff > 1e-5 {
            mismatches += 1;
            if mismatches <= 10 {
                println!(
                    "  Channel {:3}: Rust={:10.6}, Python={:10.6}, diff={:e}",
                    c, rust_val, python_val, diff
                );
            }
        }

        if diff > max_diff {
            max_diff = diff;
        }
    }

    println!("\n{}", "=".repeat(80));
    println!("RESULT:");
    println!("  Total mismatches: {}/256", mismatches);
    println!("  Max difference: {}", max_diff);
    if max_diff < 1e-5 {
        println!("  ✅ ENCODER OUTPUT MATCHES PYTHON");
    } else {
        println!("  ❌ ENCODER OUTPUT DIFFERS FROM PYTHON");
    }
    println!("{}", "=".repeat(80));
}
