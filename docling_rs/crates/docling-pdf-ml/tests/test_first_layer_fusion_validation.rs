#![cfg(feature = "pytorch")]
// Test: First Layer Fusion Validation
//
// Validates that batch norm fusion works correctly for a single ConvNormLayer
// by comparing against Python baseline that uses separate conv + batch norm.
//
// This test isolates fusion correctness from accumulated floating point errors
// that may occur when passing through the entire network.
//
// Test approach:
// 1. Load model with batch norm fusion (N=500 approach)
// 2. Extract input to first lateral_conv layer from Python baseline
// 3. Run through first ConvNormLayer with fusion
// 4. Compare output against Python baseline (lateral_conv_0_after_bn.npy)
// 5. If match: fusion works, divergence is from accumulation
// 6. If no match: fusion has a bug

use docling_pdf_ml::models::layout_predictor::pytorch_backend::{
    encoder::RTDetrV2ConvNormLayer, weights,
};
use ndarray::{Array, Ix4};
use ndarray_npy::ReadNpyExt;
use std::fs::File;
use std::path::PathBuf;
use tch::{nn, Device, Kind, Tensor};

const TOLERANCE: f64 = 1e-3; // Same tolerance as other validation tests

#[test]
fn test_first_layer_fusion_validation() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n{}", "=".repeat(80));
    println!("First Layer Fusion Validation");
    println!("{}", "=".repeat(80));
    println!();
    println!("Goal: Validate that fusion works correctly for a single ConvNormLayer");
    println!("by comparing against Python baseline (separate conv + batch norm).");
    println!();

    // Use CPU for consistent results
    let device = Device::Cpu;

    // Baseline directory
    let baseline_dir =
        PathBuf::from("baseline_data/arxiv_2206.01062/page_0/layout/pytorch_intermediate");

    println!("1. Loading Python Baseline Data");
    println!("{}", "-".repeat(80));

    // Load input to first lateral_conv
    let input_path = baseline_dir.join("lateral_conv_0_input_verified_v2.npy");
    if !input_path.exists() {
        println!("⚠️  Skipping test: baseline file not found");
        println!("   Expected: {:?}", input_path);
        return Ok(());
    }

    let input_file = File::open(&input_path)?;
    let input_arr = Array::<f32, Ix4>::read_npy(input_file)?;
    let input = Tensor::from_slice(input_arr.as_slice().unwrap())
        .reshape([
            input_arr.shape()[0] as i64,
            input_arr.shape()[1] as i64,
            input_arr.shape()[2] as i64,
            input_arr.shape()[3] as i64,
        ])
        .to(device);

    println!("   ✓ Input loaded: {:?}", input.size());

    // Load expected output (after conv + batch norm in Python)
    let output_path = baseline_dir.join("lateral_conv_0_after_bn.npy");
    if !output_path.exists() {
        println!("⚠️  Skipping test: baseline output file not found");
        println!("   Expected: {:?}", output_path);
        return Ok(());
    }

    let output_file = File::open(&output_path)?;
    let output_arr = Array::<f32, Ix4>::read_npy(output_file)?;
    let expected_output = Tensor::from_slice(output_arr.as_slice().unwrap())
        .reshape([
            output_arr.shape()[0] as i64,
            output_arr.shape()[1] as i64,
            output_arr.shape()[2] as i64,
            output_arr.shape()[3] as i64,
        ])
        .to(device);

    println!("   ✓ Expected output loaded: {:?}", expected_output.size());

    println!();
    println!("2. Creating Model with Fusion");
    println!("{}", "-".repeat(80));

    // Set environment for PyTorch
    std::env::set_var("LIBTORCH_USE_PYTORCH", "1");
    std::env::set_var("LIBTORCH_BYPASS_VERSION_CHECK", "1");

    // Get model path
    let model_path = match weights::get_model_path() {
        Ok(path) => path,
        Err(e) => {
            println!("⚠️  Skipping test: {}", e);
            return Ok(());
        }
    };

    // Create a single ConvNormLayer matching lateral_conv[0] configuration
    // From model: lateral_conv[0] is a 1x1 conv with 256 input/output channels
    let mut vs = nn::VarStore::new(device);
    let root = vs.root() / "model" / "encoder" / "lateral_convs" / "0";

    // Note: N=500 sets bias=true for fusion
    let layer = RTDetrV2ConvNormLayer::new(
        &root,
        256,     // in_channels
        256,     // out_channels
        1,       // kernel_size
        1,       // stride
        Some(0), // padding
        None,    // activation (None for lateral_conv)
    );

    println!("   ✓ ConvNormLayer created");

    // Load weights with fusion
    weights::load_weights_into(&mut vs, &model_path)?;
    vs.freeze();

    println!("   ✓ Weights loaded (with fusion)");

    println!();
    println!("3. Running Forward Pass");
    println!("{}", "-".repeat(80));

    // Run forward pass through fused ConvNormLayer
    let rust_output = tch::no_grad(|| layer.forward(&input));

    println!("   ✓ Forward pass complete");
    println!("   Rust output shape: {:?}", rust_output.size());

    println!();
    println!("4. Comparing Outputs");
    println!("{}", "=".repeat(80));

    // Flatten tensors for comparison
    let rust_flat = rust_output.flatten(0, -1);
    let expected_flat = expected_output.flatten(0, -1);

    // Compute statistics
    let diff = &rust_flat - &expected_flat;
    let abs_diff = diff.abs();

    let max_diff: f64 = abs_diff.max().try_into()?;
    let mean_diff: f64 = abs_diff.mean(Kind::Float).try_into()?;

    // Get ranges for context
    let rust_min: f64 = rust_flat.min().try_into()?;
    let rust_max: f64 = rust_flat.max().try_into()?;
    let expected_min: f64 = expected_flat.min().try_into()?;
    let expected_max: f64 = expected_flat.max().try_into()?;

    println!("--- First Layer Output ---");
    println!(
        "   Python (conv+bn): [{:.6}, {:.6}]",
        expected_min, expected_max
    );
    println!("   Rust (fused):     [{:.6}, {:.6}]", rust_min, rust_max);
    println!();
    println!(
        "   Max diff:  {:.6e} (tolerance: {:.3e})",
        max_diff, TOLERANCE
    );
    println!("   Mean diff: {:.6e}", mean_diff);

    println!();
    println!("{}", "=".repeat(80));

    if max_diff < TOLERANCE {
        println!("✅ PASS: First layer fusion produces correct output!");
        println!();
        println!("Conclusion: Fusion works correctly for a single layer.");
        println!("If end-to-end test fails, divergence is from accumulated errors");
        println!("through the network, not from fusion itself.");
        Ok(())
    } else {
        println!("❌ FAIL: First layer fusion output diverges from baseline");
        println!();
        println!("Max diff: {:.6e} >= {:.3e}", max_diff, TOLERANCE);
        println!();
        println!("Conclusion: Fusion has a bug. The formula may be correct in isolation");
        println!("(unit test passes), but something is wrong with how it's applied in");
        println!("the actual model. Possible issues:");
        println!("  1. Weight loading bug (fused weights not getting to the right place)");
        println!("  2. Fusion formula applied incorrectly for this layer configuration");
        println!("  3. Forward pass issue (batch norm not actually being skipped)");

        Err(format!(
            "First layer fusion validation failed: max diff {:.6e}",
            max_diff
        )
        .into())
    }
}
