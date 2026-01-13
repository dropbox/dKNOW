#![cfg(feature = "pytorch")]
mod common;

use ndarray::ArrayD;
use std::path::PathBuf;
/// Debug ResNet18 encoder layer-by-layer to find where Rust diverges from Python
///
/// Compare with Python output from debug_encoder_layer_by_layer.py
use tch::{Device, Tensor};

// Helper function to convert ndarray to tch::Tensor
fn numpy_to_tensor(arr: &ArrayD<f32>, device: Device) -> Tensor {
    let shape: Vec<i64> = arr.shape().iter().map(|&x| x as i64).collect();
    let data: Vec<f32> = arr.iter().copied().collect();
    Tensor::from_slice(&data).to(device).reshape(&shape)
}

#[test]
#[ignore]
fn test_encoder_layer_by_layer() {
    println!("\n{}", "=".repeat(80));
    println!("Rust Encoder Layer-by-Layer Debug");
    println!("{}", "=".repeat(80));

    let device = Device::Cpu;
    let home = std::env::var("HOME").unwrap();
    let base_path =
        PathBuf::from(&home).join("docling_debug_pdf_parsing/ml_model_inputs/tableformer");

    // Load preprocessed input
    println!("\nLoading preprocessed input...");
    let input_path = base_path.join("table_0_preprocessed_input.npy");
    let preprocessed_arr = common::baseline_loaders::load_numpy(&input_path)
        .expect("Failed to load preprocessed input");
    let input_tensor = numpy_to_tensor(&preprocessed_arr, device);
    println!("  ✓ Input shape: {:?}", input_tensor.size());
    print!("  First 10: [");
    for i in 0..10 {
        print!("{:.6}", input_tensor.double_value(&[0, 0, 0, i]));
        if i < 9 {
            print!(", ");
        }
    }
    println!("]");
    println!(
        "  Range: [{:.6}, {:.6}]",
        input_tensor.min().double_value(&[]),
        input_tensor.max().double_value(&[])
    );

    // Load model
    println!("\nLoading TableFormer model...");
    let model_dir = PathBuf::from(&home)
        .join(".cache/huggingface/hub/models--ds4sd--docling-models/snapshots/fc0f2d45e2218ea24bce5045f58a389aed16dc23/model_artifacts/tableformer/accurate");
    let model =
        docling_pdf_ml::models::table_structure::TableStructureModel::load(&model_dir, device)
            .expect("Failed to load model");
    println!("  ✓ Model loaded");

    // Manual forward pass through encoder with debug output at each stage
    println!("\n{}", "=".repeat(80));
    println!("Layer-by-Layer Forward Pass");
    println!("{}", "=".repeat(80));

    println!("\n[1] After conv1 (7x7, stride=2):");
    let mut x = input_tensor.apply(&model.encoder.conv1);
    println!("  Shape: {:?}", x.size());
    println!("  [0,0,0,0]: {:.10}", x.double_value(&[0, 0, 0, 0]));
    print!("  [0,:5,0,0]: [");
    for i in 0..5 {
        print!("{:.6}", x.double_value(&[0, i, 0, 0]));
        if i < 4 {
            print!(", ");
        }
    }
    println!("]");
    println!(
        "  Range: [{:.6}, {:.6}]",
        x.min().double_value(&[]),
        x.max().double_value(&[])
    );

    println!("\n[2] After bn1:");
    x = x.apply_t(&model.encoder.bn1, false); // train=false for inference
    println!("  Shape: {:?}", x.size());
    println!("  [0,0,0,0]: {:.10}", x.double_value(&[0, 0, 0, 0]));
    print!("  [0,:5,0,0]: [");
    for i in 0..5 {
        print!("{:.6}", x.double_value(&[0, i, 0, 0]));
        if i < 4 {
            print!(", ");
        }
    }
    println!("]");
    println!(
        "  Range: [{:.6}, {:.6}]",
        x.min().double_value(&[]),
        x.max().double_value(&[])
    );

    println!("\n[3] After relu:");
    x = x.relu();
    println!("  Shape: {:?}", x.size());
    println!("  [0,0,0,0]: {:.10}", x.double_value(&[0, 0, 0, 0]));
    print!("  [0,:5,0,0]: [");
    for i in 0..5 {
        print!("{:.6}", x.double_value(&[0, i, 0, 0]));
        if i < 4 {
            print!(", ");
        }
    }
    println!("]");
    println!(
        "  Range: [{:.6}, {:.6}]",
        x.min().double_value(&[]),
        x.max().double_value(&[])
    );

    println!("\n[4] After maxpool:");
    x = x.max_pool2d([3, 3], [2, 2], [1, 1], [1, 1], false);
    println!("  Shape: {:?}", x.size());
    println!("  [0,0,0,0]: {:.10}", x.double_value(&[0, 0, 0, 0]));
    print!("  [0,:5,0,0]: [");
    for i in 0..5 {
        print!("{:.6}", x.double_value(&[0, i, 0, 0]));
        if i < 4 {
            print!(", ");
        }
    }
    println!("]");
    println!(
        "  Range: [{:.6}, {:.6}]",
        x.min().double_value(&[]),
        x.max().double_value(&[])
    );

    println!("\n[5] After layer1 (2 BasicBlocks):");
    x = model.encoder.layer1_block0.forward(&x);
    x = model.encoder.layer1_block1.forward(&x);
    println!("  Shape: {:?}", x.size());
    println!("  [0,0,0,0]: {:.10}", x.double_value(&[0, 0, 0, 0]));
    print!("  [0,:5,0,0]: [");
    for i in 0..5 {
        print!("{:.6}", x.double_value(&[0, i, 0, 0]));
        if i < 4 {
            print!(", ");
        }
    }
    println!("]");
    println!(
        "  Range: [{:.6}, {:.6}]",
        x.min().double_value(&[]),
        x.max().double_value(&[])
    );

    println!("\n[6] After layer2 (2 BasicBlocks, downsample):");
    x = model.encoder.layer2_block0.forward(&x);
    x = model.encoder.layer2_block1.forward(&x);
    println!("  Shape: {:?}", x.size());
    println!("  [0,0,0,0]: {:.10}", x.double_value(&[0, 0, 0, 0]));
    print!("  [0,:5,0,0]: [");
    for i in 0..5 {
        print!("{:.6}", x.double_value(&[0, i, 0, 0]));
        if i < 4 {
            print!(", ");
        }
    }
    println!("]");
    println!(
        "  Range: [{:.6}, {:.6}]",
        x.min().double_value(&[]),
        x.max().double_value(&[])
    );

    println!("\n[7] After layer3 (2 BasicBlocks, downsample):");
    x = model.encoder.layer3_block0.forward(&x);
    x = model.encoder.layer3_block1.forward(&x);
    println!("  Shape: {:?}", x.size());
    println!("  [0,0,0,0]: {:.10}", x.double_value(&[0, 0, 0, 0]));
    print!("  [0,:5,0,0]: [");
    for i in 0..5 {
        print!("{:.6}", x.double_value(&[0, i, 0, 0]));
        if i < 4 {
            print!(", ");
        }
    }
    println!("]");
    println!(
        "  Range: [{:.6}, {:.6}]",
        x.min().double_value(&[]),
        x.max().double_value(&[])
    );

    println!("\n[8] After adaptive_pool (28, 28):");
    x = x.adaptive_avg_pool2d([28, 28]);
    println!("  Shape: {:?}", x.size());
    println!("  [0,0,0,0]: {:.10}", x.double_value(&[0, 0, 0, 0]));
    print!("  [0,:5,0,0]: [");
    for i in 0..5 {
        print!("{:.6}", x.double_value(&[0, i, 0, 0]));
        if i < 4 {
            print!(", ");
        }
    }
    println!("]");
    println!(
        "  Range: [{:.6}, {:.6}]",
        x.min().double_value(&[]),
        x.max().double_value(&[])
    );

    println!("\n[9] After permute to BHWC:");
    let encoder_out = x.permute([0, 2, 3, 1]);
    println!("  Shape: {:?}", encoder_out.size());
    println!(
        "  [0,0,0,0]: {:.10}",
        encoder_out.double_value(&[0, 0, 0, 0])
    );
    print!("  [0,0,0,:5]: [");
    for i in 0..5 {
        print!("{:.6}", encoder_out.double_value(&[0, 0, 0, i]));
        if i < 4 {
            print!(", ");
        }
    }
    println!("]");
    println!(
        "  Range: [{:.6}, {:.6}]",
        encoder_out.min().double_value(&[]),
        encoder_out.max().double_value(&[])
    );

    println!("\n{}", "=".repeat(80));
    println!("✓ Complete - Compare with Python output");
    println!("{}", "=".repeat(80));

    println!("\nExpected Python output:");
    println!("  After conv1: [-0.099, 0.020, -0.029, -0.590, 0.034]");
    println!("  After bn1:   [0.152, 0.043, 0.126, -0.062, 0.367]");
    println!("  After relu:  [0.152, 0.043, 0.126, 0.000, 0.367]");
    println!("  After maxpool: [0.174, 0.063, 0.371, 0.240, 0.631]");
    println!("  After layer1: [0.000, 1.707, 1.806, 2.600, 3.917]");
    println!("  After layer2: [0.647, 0.000, 0.000, 1.085, 0.107]");
    println!("  After layer3: [2.268, 2.546, 1.498, 0.498, 0.700]");
    println!("  After adaptive_pool: [2.268, 2.546, 1.498, 0.498, 0.700]");
    println!("  After permute (BHWC): [2.268, 2.546, 1.498, 0.498, 0.700]");

    // Don't assert - just print for comparison
}
