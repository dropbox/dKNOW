#![cfg(feature = "pytorch")]
mod common;

use ndarray::ArrayD;
use std::path::PathBuf;
/// Test if encoder output tensor has correct memory layout
///
/// Check if the tensor is contiguous and has the right format
use tch::{Device, Tensor};

// Helper function to convert ndarray to tch::Tensor
fn numpy_to_tensor(arr: &ArrayD<f32>, device: Device) -> Tensor {
    let shape: Vec<i64> = arr.shape().iter().map(|&x| x as i64).collect();
    let data: Vec<f32> = arr.iter().copied().collect();
    Tensor::from_slice(&data).to(device).reshape(&shape)
}

#[test]
#[ignore]
fn test_encoder_output_contiguous() {
    println!("\n{}", "=".repeat(80));
    println!("TEST ENCODER OUTPUT MEMORY LAYOUT");
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

    println!("Input tensor:");
    println!("  Shape: {:?}", input_tensor.size());
    println!("  Is contiguous: {}", input_tensor.is_contiguous());

    // Run encoder
    println!("\nRunning ResNet18 encoder...");
    let encoder_out = model.encoder.forward(&input_tensor);

    println!("Encoder output (BHWC format):");
    println!("  Shape: {:?}", encoder_out.size());
    println!("  Is contiguous: {}", encoder_out.is_contiguous());

    // Convert to BCHW
    let encoder_bchw = encoder_out.permute([0, 3, 1, 2]);

    println!("After permute to BCHW:");
    println!("  Shape: {:?}", encoder_bchw.size());
    println!("  Is contiguous: {}", encoder_bchw.is_contiguous());

    // Try making it contiguous
    let encoder_bchw_contig = encoder_bchw.contiguous();
    println!("After .contiguous():");
    println!("  Shape: {:?}", encoder_bchw_contig.size());
    println!("  Is contiguous: {}", encoder_bchw_contig.is_contiguous());

    // Test conv with both versions
    let basic_block = &model.tag_transformer.input_filter.block0;
    let (ds_conv, _) = basic_block
        .downsample
        .as_ref()
        .expect("Downsample should exist");

    println!("\n{}", "-".repeat(80));
    println!("TESTING CONV WITH NON-CONTIGUOUS INPUT");
    println!("{}", "-".repeat(80));
    let output_non_contig = encoder_bchw.apply(ds_conv);
    print!("Output [0,:10,0,0]: [");
    for i in 0..10 {
        print!("{:7.3}", output_non_contig.double_value(&[0, i, 0, 0]));
        if i < 9 {
            print!(", ");
        }
    }
    println!("]");
    println!("Expected:            [ -2.951,  -5.572,   3.078,  -5.745,  -3.301,   6.457,   4.396,   1.514,  -6.602,   0.334]");

    println!("\n{}", "-".repeat(80));
    println!("TESTING CONV WITH CONTIGUOUS INPUT");
    println!("{}", "-".repeat(80));
    let output_contig = encoder_bchw_contig.apply(ds_conv);
    print!("Output [0,:10,0,0]: [");
    for i in 0..10 {
        print!("{:7.3}", output_contig.double_value(&[0, i, 0, 0]));
        if i < 9 {
            print!(", ");
        }
    }
    println!("]");
    println!("Expected:            [ -2.951,  -5.572,   3.078,  -5.745,  -3.301,   6.457,   4.396,   1.514,  -6.602,   0.334]");

    println!("\n{}", "=".repeat(80));
    if !encoder_bchw.is_contiguous() {
        println!("⚠️  ISSUE FOUND: Tensor is not contiguous after permute!");
        println!("This may cause incorrect conv results if tch-rs requires contiguous input.");
    } else {
        println!("✓ Tensor is contiguous");
    }
    println!("{}", "=".repeat(80));
}
