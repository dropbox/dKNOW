#![cfg(feature = "pytorch")]
mod common;

use ndarray::ArrayD;
use std::path::PathBuf;
/// Trace BasicBlock forward pass layer-by-layer to find divergence
///
/// This test instruments the BasicBlock forward pass to print intermediate
/// values after EVERY operation, allowing direct comparison with Python baseline.
///
/// Python baseline values (from test_basic_block_forward.py):
/// Input [0,:10,0,0]:         [2.268, 2.546, 1.498, 0.498, 0.700, 0.0, 0.0, 2.848, 0.0, 1.059]
/// After downsample conv:     [-2.951, -5.572, 3.078, -5.745, -3.301, 6.457, 4.396, 1.514, -6.602, 0.334]
/// After downsample bn:       [-0.191, -0.296, 0.387, -1.022, -1.299, 0.474, -0.124, 0.036, -0.198, 0.538]
/// After main conv1:          [-2.951, -5.572, 3.078, -5.745, -3.301, 6.457, 4.396, 1.514, -6.602, 0.334]
/// After main bn1:            [0.192, -0.609, -0.850, -0.382, -0.372, -0.022, 0.202, 0.267, 0.545, -0.255]
/// After main relu:           [0.192, 0.0, 0.0, 0.0, 0.0, 0.0, 0.202, 0.267, 0.545, 0.0]
/// After main conv2:          [5.096, 13.562, -4.979, 5.796, 2.816, -1.820, 3.038, -2.045, -1.425, 1.899]
/// After main bn2:            [0.489, 0.470, -0.055, -0.292, -0.209, -0.962, -0.212, -0.002, -0.755, -0.056]
/// After add residual:        [0.298, 0.174, 0.331, -1.314, -1.508, -0.488, -0.336, 0.034, -0.954, 0.482]
/// After final relu:          [0.298, 0.174, 0.331, 0.0, 0.0, 0.0, 0.0, 0.034, 0.0, 0.482]
use tch::{Device, Tensor};

// Helper function to convert ndarray to tch::Tensor
fn numpy_to_tensor(arr: &ArrayD<f32>, device: Device) -> Tensor {
    let shape: Vec<i64> = arr.shape().iter().map(|&x| x as i64).collect();
    let data: Vec<f32> = arr.iter().copied().collect();
    Tensor::from_slice(&data).to(device).reshape(&shape)
}

// Helper to print first 10 channels at position [0,:10,0,0]
fn print_channels(tensor: &Tensor, label: &str) {
    print!("{:30} [", label);
    for i in 0..10 {
        let val = tensor.double_value(&[0, i, 0, 0]);
        print!("{:7.3}", val);
        if i < 9 {
            print!(", ");
        }
    }
    println!("]");
}

#[test]
#[ignore]
fn trace_basicblock_forward() {
    println!("\n{}", "=".repeat(80));
    println!("BASICBLOCK LAYER-BY-LAYER TRACE");
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
    println!("Running ResNet18 encoder...");
    let encoder_out = model.encoder.forward(&input_tensor);

    // Convert to BCHW
    let encoder_bchw = encoder_out.permute([0, 3, 1, 2]);

    println!("\n{}", "-".repeat(80));
    println!("INPUT TO BASICBLOCK");
    println!("{}", "-".repeat(80));
    print_channels(&encoder_bchw, "Input [0,:10,0,0]:");

    println!("\nPython expected:               [  2.268,   2.546,   1.498,   0.498,   0.700,   0.000,   0.000,   2.848,   0.000,   1.059]");

    // Get the first BasicBlock from input_filter
    let basic_block = &model.tag_transformer.input_filter.block0;

    // Manual forward pass with tracing
    println!("\n{}", "-".repeat(80));
    println!("DOWNSAMPLE PATH");
    println!("{}", "-".repeat(80));

    let identity = if let Some((ds_conv, ds_bn)) = &basic_block.downsample {
        let mut residual = encoder_bchw.apply(ds_conv);
        print_channels(&residual, "After downsample conv:");
        println!("Python expected:               [ -2.951,  -5.572,   3.078,  -5.745,  -3.301,   6.457,   4.396,   1.514,  -6.602,   0.334]");

        residual = residual.apply_t(ds_bn, false);
        print_channels(&residual, "After downsample bn:");
        println!("Python expected:               [ -0.191,  -0.296,   0.387,  -1.022,  -1.299,   0.474,  -0.124,   0.036,  -0.198,   0.538]");

        residual
    } else {
        encoder_bchw.shallow_clone()
    };

    println!("\n{}", "-".repeat(80));
    println!("MAIN PATH");
    println!("{}", "-".repeat(80));

    // Main path
    let mut out = encoder_bchw.apply(&basic_block.conv1);
    print_channels(&out, "After conv1:");
    println!("Python expected:               [ -2.951,  -5.572,   3.078,  -5.745,  -3.301,   6.457,   4.396,   1.514,  -6.602,   0.334]");

    out = out.apply_t(&basic_block.bn1, false);
    print_channels(&out, "After bn1:");
    println!("Python expected:               [  0.192,  -0.609,  -0.850,  -0.382,  -0.372,  -0.022,   0.202,   0.267,   0.545,  -0.255]");

    out = out.relu();
    print_channels(&out, "After relu:");
    println!("Python expected:               [  0.192,   0.000,   0.000,   0.000,   0.000,   0.000,   0.202,   0.267,   0.545,   0.000]");

    out = out.apply(&basic_block.conv2);
    print_channels(&out, "After conv2:");
    println!("Python expected:               [  5.096,  13.562,  -4.979,   5.796,   2.816,  -1.820,   3.038,  -2.045,  -1.425,   1.899]");

    out = out.apply_t(&basic_block.bn2, false);
    print_channels(&out, "After bn2:");
    println!("Python expected:               [  0.489,   0.470,  -0.055,  -0.292,  -0.209,  -0.962,  -0.212,  -0.002,  -0.755,  -0.056]");

    println!("\n{}", "-".repeat(80));
    println!("RESIDUAL CONNECTION");
    println!("{}", "-".repeat(80));

    out += identity;
    print_channels(&out, "After add residual:");
    println!("Python expected:               [  0.298,   0.174,   0.331,  -1.314,  -1.508,  -0.488,  -0.336,   0.034,  -0.954,   0.482]");

    out = out.relu();
    print_channels(&out, "After final relu:");
    println!("Python expected:               [  0.298,   0.174,   0.331,   0.000,   0.000,   0.000,   0.000,   0.034,   0.000,   0.482]");

    println!("\n{}", "=".repeat(80));
    println!("COMPARISON SUMMARY");
    println!("{}", "=".repeat(80));
    println!("Compare each layer's Rust output with Python expected values above.");
    println!("The divergence point will reveal the bug.");
    println!("{}", "=".repeat(80));
}
