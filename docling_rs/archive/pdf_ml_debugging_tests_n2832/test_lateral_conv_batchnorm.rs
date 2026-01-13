#![cfg(feature = "pytorch")]
// Test lateral conv BatchNorm issue
// This test isolates the lateral_conv[0] forward pass to debug BatchNorm divergence

use docling_pdf_ml::models::layout_predictor::pytorch_backend::weights::load_weights_into;
use ndarray_npy::read_npy;
use std::path::Path;
use tch::nn;
use tch::{Device, IndexOp, Kind, Tensor};

#[test]
#[ignore] // Debug test for BatchNorm investigation - not a production test
fn test_lateral_conv_0_forward() {
    // Set env vars
    std::env::set_var(
        "DYLD_LIBRARY_PATH",
        "/opt/homebrew/lib/python3.14/site-packages/torch/lib",
    );
    std::env::set_var("DYLD_FALLBACK_LIBRARY_PATH", "/opt/homebrew/opt/llvm/lib");
    std::env::set_var("LIBTORCH_USE_PYTORCH", "1");
    std::env::set_var("LIBTORCH_BYPASS_VERSION_CHECK", "1");

    println!("\n=== Lateral Conv 0 BatchNorm Test ===\n");

    // Load input
    let input: ndarray::Array4<f32> =
        read_npy("test_lateral_conv_input.npy").expect("Failed to load input");
    let input_tensor = Tensor::from_slice(input.as_slice().unwrap())
        .reshape([1, 256, 20, 20])
        .to_kind(Kind::Float);

    println!(
        "Input: shape={:?}, mean={:.6}",
        input_tensor.size(),
        input_tensor.mean(Kind::Float).double_value(&[])
    );

    // Create VarStore and layers
    let mut vs = nn::VarStore::new(Device::Cpu);

    // Create Conv2d (1x1, no bias)
    let conv_config = nn::ConvConfig {
        stride: 1,
        padding: 0,
        bias: false,
        ..Default::default()
    };
    let conv = nn::conv2d(
        &(vs.root() / "model" / "encoder" / "lateral_convs" / "0" / "conv"),
        256,
        256,
        1,
        conv_config,
    );

    // Create BatchNorm2d
    let bn_config = nn::BatchNormConfig {
        cudnn_enabled: false,
        eps: 1e-5,
        momentum: 0.1,
        ..Default::default()
    };
    let bn = nn::batch_norm2d(
        &(vs.root() / "model" / "encoder" / "lateral_convs" / "0" / "norm"),
        256,
        bn_config,
    );

    println!("\nLayers created. Variables in VarStore:");
    {
        let variables = vs.variables_.lock().unwrap();
        for (key, var) in variables.named_variables.iter() {
            if key.contains("lateral_convs.0") {
                let shape = var.size();
                if shape.len() == 1 && shape[0] <= 5 {
                    let vals: Vec<f32> = Vec::try_from(var).unwrap_or_default();
                    println!("  {}: {:?}", key, vals);
                } else if shape.len() == 1 {
                    let sample = var.narrow(0, 0, 5);
                    let vals: Vec<f32> = Vec::try_from(&sample).unwrap_or_default();
                    println!("  {}: shape={:?}, first 5={:?}", key, shape, vals);
                } else {
                    println!("  {}: shape={:?}", key, shape);
                }
            }
        }
    }

    // Load weights
    let model_path = Path::new("/Users/ayates/.cache/huggingface/hub/models--ds4sd--docling-layout-heron/snapshots/bdb7099d742220552d703932cc0ce0a26a7a8da8/model.safetensors");
    println!("\nLoading weights from safetensors...");
    load_weights_into(&mut vs, model_path).expect("Failed to load weights");

    // Check loaded values
    println!("\nValues after loading:");
    {
        let variables = vs.variables_.lock().unwrap();
        for (key, var) in variables.named_variables.iter() {
            if key.contains("lateral_convs.0.norm") {
                let shape = var.size();
                if shape.len() == 1 {
                    let sample = var.narrow(0, 0, 5.min(shape[0]));
                    let vals: Vec<f32> = Vec::try_from(&sample).unwrap_or_default();
                    println!("  {}: first 5={:?}", key, vals);
                }
            }
        }
    }

    // Forward pass
    println!("\nRunning forward pass...");
    let after_conv = tch::no_grad(|| input_tensor.apply(&conv));
    println!(
        "After conv: mean={:.6}, sample={:?}",
        after_conv.mean(Kind::Float).double_value(&[]),
        Vec::<f32>::try_from(&after_conv.i((0, ..5, 0, 0))).unwrap()
    );

    let after_bn = tch::no_grad(|| {
        after_conv.apply_t(&bn, false) // training=false (eval mode)
    });
    println!(
        "After BN: mean={:.6}, sample={:?}",
        after_bn.mean(Kind::Float).double_value(&[]),
        Vec::<f32>::try_from(&after_bn.i((0, ..5, 0, 0))).unwrap()
    );

    // Load expected output
    let expected: ndarray::Array4<f32> =
        read_npy("test_lateral_conv_expected.npy").expect("Failed to load expected output");
    let expected_tensor = Tensor::from_slice(expected.as_slice().unwrap())
        .reshape([1, 256, 20, 20])
        .to_kind(Kind::Float);

    println!(
        "Expected: mean={:.6}, sample={:?}",
        expected_tensor.mean(Kind::Float).double_value(&[]),
        Vec::<f32>::try_from(&expected_tensor.i((0, ..5, 0, 0))).unwrap()
    );

    // Compare
    let diff = (&after_bn - &expected_tensor).abs();
    let max_diff = diff.max().double_value(&[]);
    let mean_diff = diff.mean(Kind::Float).double_value(&[]);

    println!("\nComparison:");
    println!("  Max diff: {:.6e}", max_diff);
    println!("  Mean diff: {:.6e}", mean_diff);

    if max_diff < 1e-3 {
        println!("\n✅ PASS: Output matches expected within tolerance");
    } else {
        println!("\n❌ FAIL: Output diverges from expected");
        println!("\nDEBUG: Check if running_mean/running_var are being used:");
        println!(
            "  - Python BN applies: (x - running_mean) / sqrt(running_var + eps) * weight + bias"
        );
        println!("  - Rust should use apply_t(&bn, false) for eval mode");
        println!("  - Check if running_mean/running_var are loaded correctly above");
        panic!("BatchNorm output diverges");
    }
}
