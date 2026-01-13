#![cfg(feature = "pytorch")]
use std::path::PathBuf;
/// Test conv layer with known input (all ones) to debug weight loading
///
/// If weights are loaded correctly, output[i,j,0,0] = sum of all weights[i,j,:,:]
use tch::{Device, Tensor};

#[test]
#[ignore]
fn test_downsample_conv_with_ones() {
    println!("\n{}", "=".repeat(80));
    println!("TEST CONV WITH ONES INPUT");
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

    println!("\nConv layer: downsample conv (1x1, 256→512, stride=1, padding=0, bias=false)");

    // Create test input: all ones [1, 256, 1, 1]
    let test_input = Tensor::ones([1, 256, 1, 1], (tch::Kind::Float, device));
    println!("Test input: all ones, shape [1, 256, 1, 1]");

    // Run through the conv
    let output = test_input.apply(ds_conv);
    println!("Output shape: {:?}", output.size());

    // For a 1x1 conv with all-ones input and no bias:
    // output[0,c,0,0] = sum of weight[c,:,0,0] across all 256 input channels

    println!("\nRust output [0,:10,0,0]:");
    for i in 0..10 {
        let val = output.double_value(&[0, i, 0, 0]);
        println!("  output[0,{:3},0,0] = {:12.6}", i, val);
    }

    println!("\n{}", "=".repeat(80));
    println!("Now run the Python equivalent to compare:");
    println!("  python3 test_conv_with_ones.py");
    println!("{}", "=".repeat(80));
}
