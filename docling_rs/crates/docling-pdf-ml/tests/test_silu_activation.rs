#![cfg(feature = "pytorch")]
mod common;
/// Test SiLU activation matching between Rust and Python
use common::baseline_loaders::load_numpy;
use std::path::PathBuf;
use tch::Tensor;

#[test]
fn test_silu_activation() {
    println!("\n=== SiLU Activation Test ===\n");

    // Load test input
    let input_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_silu_input.npy");
    let input_data = load_numpy(&input_path).expect("Failed to load test input");
    let input_vec = input_data.into_raw_vec_and_offset().0;

    // Convert to tensor
    let input_tensor = Tensor::from_slice(&input_vec);

    // Apply SiLU
    let output_tensor = input_tensor.silu();

    // Convert to vec
    let output_vec: Vec<f32> =
        Vec::try_from(&output_tensor).expect("Failed to convert output to vec");

    // Load Python output
    let python_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_silu_output_python.npy");
    let python_data = load_numpy(&python_path).expect("Failed to load Python output");
    let python_vec = python_data.into_raw_vec_and_offset().0;

    // Compare
    println!("Input values:");
    for (i, val) in input_vec.iter().enumerate() {
        println!("  [{}]: {:.6}", i, val);
    }
    println!();

    println!("SiLU outputs:");
    println!("  idx | Rust        | Python      | Diff");
    println!("  ----|-------------|-------------|-------------");
    for i in 0..output_vec.len() {
        let diff = (output_vec[i] - python_vec[i]).abs();
        let status = if diff < 1e-6 { "✅" } else { "❌" };
        println!(
            "  {:3} | {:11.6} | {:11.6} | {:11.6} {}",
            i, output_vec[i], python_vec[i], diff, status
        );
    }
    println!();

    // Check max diff
    let max_diff = output_vec
        .iter()
        .zip(python_vec.iter())
        .map(|(r, p)| (r - p).abs())
        .fold(0.0f32, f32::max);

    println!("Max diff: {:.6e}", max_diff);

    if max_diff < 1e-6 {
        println!("✅ SiLU activation matches Python");
    } else {
        println!("❌ SiLU activation diverges from Python");
        panic!("SiLU test failed");
    }
}
