/// Tensor Comparison Test
///
/// Tests whether Rust-preprocessed tensor produces same results as Python-preprocessed tensor.
mod common;

use common::baseline_loaders::load_numpy;
use ort::session::Session;
use std::path::PathBuf;

#[test]
fn test_python_vs_rust_tensor() {
    println!("\n================================================================================");
    println!("Tensor Comparison Test");
    println!("================================================================================");
    println!("Testing Python-preprocessed vs Rust-preprocessed tensors");
    println!();

    // Load ONNX model
    let model_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("onnx_exports/layout_optimum/model.onnx");

    if !model_path.exists() {
        println!("⚠️  Skipping test - ONNX model not found");
        return;
    }

    // Load Python-preprocessed tensor (known working)
    let python_tensor_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("ml_model_inputs/layout_predictor/page_0_preprocessed_input.npy");

    if !python_tensor_path.exists() {
        println!("⚠️  Skipping test - Python tensor not found");
        return;
    }

    // Load Rust-preprocessed tensor (from DEBUG_ONNX)
    let rust_tensor_path = PathBuf::from("/tmp/rust_preprocessed_tensor.npy");

    if !rust_tensor_path.exists() {
        println!("⚠️  Skipping test - Rust tensor not found at /tmp/rust_preprocessed_tensor.npy");
        println!("   Run with DEBUG_ONNX=1 to generate it");
        return;
    }

    // Load session
    let mut session = Session::builder()
        .expect("builder")
        .commit_from_file(&model_path)
        .expect("load");

    // Test 1: Python tensor
    println!("\n--- Test 1: Python-preprocessed tensor ---");
    let python_tensor = load_numpy(&python_tensor_path).expect("load python tensor");
    println!("  Shape: {:?}", python_tensor.shape());
    println!(
        "  Stats: min={:.6}, max={:.6}, mean={:.6}",
        python_tensor.iter().copied().fold(f32::INFINITY, f32::min),
        python_tensor
            .iter()
            .copied()
            .fold(f32::NEG_INFINITY, f32::max),
        python_tensor.iter().sum::<f32>() / python_tensor.len() as f32
    );

    let py_shape = python_tensor.shape().to_vec();
    let py_data = python_tensor.into_raw_vec_and_offset().0;

    let (py_min, py_max, py_text, py_sh) = {
        let py_input = ort::value::Value::from_array((py_shape.as_slice(), py_data.clone()))
            .expect("py input");
        let py_outputs = session
            .run(ort::inputs!["pixel_values" => py_input])
            .expect("py run");
        let (_, py_logits) = py_outputs["logits"].try_extract_tensor::<f32>().unwrap();

        let min = py_logits.iter().copied().fold(f32::INFINITY, f32::min);
        let max = py_logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let text = (0..300)
            .map(|q| py_logits[q * 17 + 9])
            .fold(f32::NEG_INFINITY, f32::max);
        let sh = (0..300)
            .map(|q| py_logits[q * 17 + 7])
            .fold(f32::NEG_INFINITY, f32::max);

        (min, max, text, sh)
    };

    println!("  Logits range: [{py_min:.6}, {py_max:.6}]");
    println!(
        "  Text max logit: {:.6}, conf: {:.6}",
        py_text,
        1.0 / (1.0 + (-py_text).exp())
    );
    println!(
        "  SH max logit: {:.6}, conf: {:.6}",
        py_sh,
        1.0 / (1.0 + (-py_sh).exp())
    );

    // Test 2: Rust tensor
    println!("\n--- Test 2: Rust-preprocessed tensor ---");
    let rust_tensor = load_numpy(&rust_tensor_path).expect("load rust tensor");
    println!("  Shape: {:?}", rust_tensor.shape());
    println!(
        "  Stats: min={:.6}, max={:.6}, mean={:.6}",
        rust_tensor.iter().copied().fold(f32::INFINITY, f32::min),
        rust_tensor
            .iter()
            .copied()
            .fold(f32::NEG_INFINITY, f32::max),
        rust_tensor.iter().sum::<f32>() / rust_tensor.len() as f32
    );

    let rust_shape = rust_tensor.shape().to_vec();
    let rust_data = rust_tensor.into_raw_vec_and_offset().0;

    let rust_input = ort::value::Value::from_array((rust_shape.as_slice(), rust_data.clone()))
        .expect("rust input");
    let rust_outputs = session
        .run(ort::inputs!["pixel_values" => rust_input])
        .expect("rust run");
    let (_, rust_logits) = rust_outputs["logits"].try_extract_tensor::<f32>().unwrap();

    let rust_min = rust_logits.iter().copied().fold(f32::INFINITY, f32::min);
    let rust_max = rust_logits
        .iter()
        .copied()
        .fold(f32::NEG_INFINITY, f32::max);
    let rust_text = (0..300)
        .map(|q| rust_logits[q * 17 + 9])
        .fold(f32::NEG_INFINITY, f32::max);
    let rust_sh = (0..300)
        .map(|q| rust_logits[q * 17 + 7])
        .fold(f32::NEG_INFINITY, f32::max);

    println!("  Logits range: [{rust_min:.6}, {rust_max:.6}]");
    println!(
        "  Text max logit: {:.6}, conf: {:.6}",
        rust_text,
        1.0 / (1.0 + (-rust_text).exp())
    );
    println!(
        "  SH max logit: {:.6}, conf: {:.6}",
        rust_sh,
        1.0 / (1.0 + (-rust_sh).exp())
    );

    // Compare tensor values
    println!("\n--- Tensor Value Comparison ---");
    let max_diff: f32 = py_data
        .iter()
        .zip(rust_data.iter())
        .map(|(p, r)| (p - r).abs())
        .fold(0.0f32, f32::max);
    let mean_diff: f32 = py_data
        .iter()
        .zip(rust_data.iter())
        .map(|(p, r)| (p - r).abs())
        .sum::<f32>()
        / py_data.len() as f32;

    println!("  Max tensor diff:  {max_diff:.6}");
    println!("  Mean tensor diff: {mean_diff:.6}");

    // Compare logits
    println!("\n--- Logits Comparison ---");
    println!(
        "  Logits range diff: min={:.6}, max={:.6}",
        (py_min - rust_min).abs(),
        (py_max - rust_max).abs()
    );
    println!("  Text logit diff:  {:.6}", (py_text - rust_text).abs());
    println!("  SH logit diff:    {:.6}", (py_sh - rust_sh).abs());

    println!(
        "\n================================================================================\n"
    );
}
