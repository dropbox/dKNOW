#![cfg(feature = "pytorch")]
use std::path::PathBuf;
/// Verify that conv weights are loaded correctly from SafeTensors
///
/// Compare the actual weight values loaded in Rust with expected values from SafeTensors
use tch::{Device, Tensor};

#[test]
#[ignore]
fn verify_downsample_conv_weights() {
    println!("\n{}", "=".repeat(80));
    println!("VERIFY DOWNSAMPLE CONV WEIGHT LOADING");
    println!("{}", "=".repeat(80));

    let device = Device::Cpu;
    let home = std::env::var("HOME").unwrap();

    // Load model
    println!("\nLoading model...");
    let model_dir = PathBuf::from(&home)
        .join(".cache/huggingface/hub/models--ds4sd--docling-models/snapshots/fc0f2d45e2218ea24bce5045f58a389aed16dc23/model_artifacts/tableformer/accurate");
    let model =
        docling_pdf_ml::models::table_structure::TableStructureModel::load(&model_dir, device)
            .expect("Failed to load model");

    // Get the downsample conv layer
    let basic_block = &model.tag_transformer.input_filter.block0;
    let (ds_conv, _ds_bn) = basic_block
        .downsample
        .as_ref()
        .expect("Downsample should exist");

    // Access the conv weight tensor
    // We need to get the actual weight tensor from the conv layer
    // Unfortunately tch-rs doesn't provide direct access, so let's check via forward pass

    println!("\nExpected from SafeTensors:");
    println!("Key: _tag_transformer._input_filter.0.downsample.0.weight");
    println!("Shape: [512, 256, 1, 1]");
    println!("First 10 values from channel 0:");
    println!("[-0.4564, 0.1250, 0.1157, 0.0290, 0.0386, 0.1293, -0.1035, -0.0513, 0.0237, 0.1610]");

    println!("\nTo verify weights are correct, we'll do a manual forward pass");
    println!("with a known input and compare with expected output.");

    // Create a simple test input: ones for all channels
    let test_input = Tensor::ones([1, 256, 1, 1], (tch::Kind::Float, device));

    println!("\nTest input: all ones, shape [1, 256, 1, 1]");

    // Run through the conv
    let output = test_input.apply(ds_conv);
    println!("Output shape: {:?}", output.size());

    // The sum of all weights for output channel 0 should match the output value
    print!("First 10 output channels: [");
    for i in 0..10 {
        print!("{:.4}", output.double_value(&[0, i, 0, 0]));
        if i < 9 {
            print!(", ");
        }
    }
    println!("]");

    println!("\n{}", "=".repeat(80));
    println!("If weights are loaded correctly, we can compute expected values.");
    println!("Sum of all 256 input weights for each output channel = output value");
    println!("{}", "=".repeat(80));
}
