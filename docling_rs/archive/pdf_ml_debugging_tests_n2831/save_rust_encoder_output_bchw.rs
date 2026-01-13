#![cfg(feature = "pytorch")]
mod common;

use ndarray::{ArrayD, IxDyn};
use std::path::PathBuf;
/// Save Rust encoder output in BCHW format for comparison with Python
///
/// This will save the full 256-channel encoder output so we can compare channel-by-channel
use tch::{Device, Tensor};

// Helper function to convert ndarray to tch::Tensor
fn numpy_to_tensor(arr: &ArrayD<f32>, device: Device) -> Tensor {
    let shape: Vec<i64> = arr.shape().iter().map(|&x| x as i64).collect();
    let data: Vec<f32> = arr.iter().copied().collect();
    Tensor::from_slice(&data).to(device).reshape(&shape)
}

// Helper function to convert tch::Tensor to ndarray
fn tensor_to_numpy(tensor: &Tensor) -> ArrayD<f32> {
    let shape: Vec<usize> = tensor.size().iter().map(|&x| x as usize).collect();
    // Flatten the tensor to 1D for conversion
    let flat_tensor = tensor.contiguous().view([-1]);
    let flat_data = Vec::<f32>::try_from(flat_tensor).expect("Failed to convert tensor to vec");
    ArrayD::from_shape_vec(IxDyn(&shape), flat_data).expect("Failed to create ndarray")
}

#[test]
#[ignore]
fn save_rust_encoder_output() {
    println!("\n{}", "=".repeat(80));
    println!("SAVE RUST ENCODER OUTPUT");
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

    // Run encoder
    println!("Running Rust encoder...");
    let encoder_out = model.encoder.forward(&input_tensor);

    println!("Encoder output (BHWC): {:?}", encoder_out.size());

    // Convert to BCHW
    let encoder_bchw = encoder_out.permute([0, 3, 1, 2]);
    println!("After permute to BCHW: {:?}", encoder_bchw.size());

    // Convert to ndarray and save as .npy
    let encoder_arr = tensor_to_numpy(&encoder_bchw);
    let output_path = base_path.join("rust_encoder_output_bchw.npy");
    common::baseline_loaders::save_numpy(&encoder_arr, &output_path)
        .expect("Failed to save numpy array");

    println!("\n✓ Saved Rust encoder output to: {:?}", output_path);
    println!("  Shape: {:?}", encoder_bchw.size());

    // Print first 10 channels at [0,:10,0,0]
    print!("  First 10 channels at [0,:10,0,0]: [");
    for c in 0..10 {
        let val = encoder_bchw.double_value(&[0, c, 0, 0]);
        print!("{:7.3}", val);
        if c < 9 {
            print!(", ");
        }
    }
    println!("]");

    println!("\n{}", "=".repeat(80));
    println!("Now run Python script to compare:");
    println!("  python3 compare_encoder_outputs.py");
    println!("{}", "=".repeat(80));
}
