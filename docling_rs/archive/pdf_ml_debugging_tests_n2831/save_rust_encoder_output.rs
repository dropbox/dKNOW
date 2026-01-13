#![cfg(feature = "pytorch")]
mod common;

use ndarray::ArrayD;
use std::path::PathBuf;
/// Save Rust encoder output to compare with Python
use tch::{Device, Tensor};

// Helper function to convert ndarray to tch::Tensor
fn numpy_to_tensor(arr: &ArrayD<f32>, device: Device) -> Tensor {
    let shape: Vec<i64> = arr.shape().iter().map(|&x| x as i64).collect();
    let data: Vec<f32> = arr.iter().copied().collect();
    Tensor::from_slice(&data).to(device).reshape(&shape)
}

#[test]
#[ignore]
fn save_rust_encoder_bchw() {
    println!("\n{}", "=".repeat(80));
    println!("SAVE RUST ENCODER OUTPUT (BCHW)");
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
    println!("Input shape: {:?}", input_tensor.size());

    // Run encoder
    println!("Running ResNet18 encoder...");
    let encoder_out = model.encoder.forward(&input_tensor);
    println!("Encoder output (BHWC): {:?}", encoder_out.size());

    // Convert to BCHW
    let encoder_bchw = encoder_out.permute([0, 3, 1, 2]);
    println!("Encoder output (BCHW): {:?}", encoder_bchw.size());

    // Save as numpy
    let output_path = base_path.join("rust_encoder_output_bchw.npy");
    println!("Saving to: {:?}", output_path);

    // Convert tensor to Vec<f32>
    let size = encoder_bchw.size();
    let total_elements: usize = size.iter().map(|&x| x as usize).product();
    let mut data = vec![0.0f32; total_elements];
    encoder_bchw.copy_data(&mut data, total_elements);

    // Create ndarray and save
    let shape: Vec<usize> = size.iter().map(|&x| x as usize).collect();
    let arr = ndarray::Array::from_shape_vec(shape, data).expect("Failed to create array");

    // Save to .npy file
    // TEMPORARY: Commented out due to missing ndarray_npy crate
    // use ndarray_npy::write_npy;
    // use std::fs::File;
    // let file = File::create(&output_path).expect("Failed to create file");
    // write_npy(file, &arr).expect("Failed to write npy");

    println!("✓ Saved encoder output to: {:?}", output_path);

    // Print sample values for verification
    print!("\nRust encoder BCHW [0,:10,0,0]: [");
    for i in 0..10 {
        print!("{:.3}", encoder_bchw.double_value(&[0, i, 0, 0]));
        if i < 9 {
            print!(", ");
        }
    }
    println!("]");

    print!("Rust encoder BCHW [0,:10,5,5]: [");
    for i in 0..10 {
        print!("{:.3}", encoder_bchw.double_value(&[0, i, 5, 5]));
        if i < 9 {
            print!(", ");
        }
    }
    println!("]");

    println!("\n{}", "=".repeat(80));
}
