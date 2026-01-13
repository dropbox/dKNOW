/// CoreML vs CPU Execution Provider Test
///
/// Tests whether CoreML execution provider produces different results than CPU.
mod common;

use common::baseline_loaders::load_numpy;
use ort::execution_providers::CoreMLExecutionProvider;
use ort::session::Session;
use std::path::PathBuf;

#[test]
fn test_coreml_vs_cpu() {
    println!("\n================================================================================");
    println!("CoreML vs CPU Execution Provider Test");
    println!("================================================================================");
    println!();

    let model_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("onnx_exports/layout_optimum/model.onnx");

    if !model_path.exists() {
        println!("⚠️  Skipping - ONNX model not found");
        return;
    }

    // Use the Rust-preprocessed tensor from multi_page.pdf
    let tensor_path = PathBuf::from("/tmp/rust_preprocessed_tensor.npy");

    if !tensor_path.exists() {
        println!("⚠️  Skipping - Rust tensor not found at /tmp/rust_preprocessed_tensor.npy");
        return;
    }

    let tensor = load_numpy(&tensor_path).expect("load tensor");
    let shape = tensor.shape().to_vec();
    let data = tensor.into_raw_vec_and_offset().0;

    // Test 1: CPU execution provider (default)
    println!("\n--- Test 1: CPU execution provider ---");
    let (cpu_min, cpu_max, cpu_text, cpu_sh) = {
        let mut session = Session::builder()
            .expect("builder")
            .commit_from_file(&model_path)
            .expect("load");

        let input = ort::value::Value::from_array((shape.as_slice(), data.clone())).expect("input");
        let outputs = session
            .run(ort::inputs!["pixel_values" => input])
            .expect("run");
        let (_, logits) = outputs["logits"].try_extract_tensor::<f32>().unwrap();

        let min = logits.iter().copied().fold(f32::INFINITY, f32::min);
        let max = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let text = (0..300)
            .map(|q| logits[q * 17 + 9])
            .fold(f32::NEG_INFINITY, f32::max);
        let sh = (0..300)
            .map(|q| logits[q * 17 + 7])
            .fold(f32::NEG_INFINITY, f32::max);

        (min, max, text, sh)
    };

    println!("  Logits range: [{cpu_min:.6}, {cpu_max:.6}]");
    println!(
        "  Text max logit: {:.6}, conf: {:.6}",
        cpu_text,
        1.0 / (1.0 + (-cpu_text).exp())
    );
    println!(
        "  SH max logit: {:.6}, conf: {:.6}",
        cpu_sh,
        1.0 / (1.0 + (-cpu_sh).exp())
    );

    // Test 2: CoreML execution provider
    println!("\n--- Test 2: CoreML execution provider ---");
    let (coreml_min, coreml_max, coreml_text, coreml_sh) = {
        let coreml_result = Session::builder()
            .expect("builder")
            .with_execution_providers([CoreMLExecutionProvider::default().build()])
            .and_then(|b| b.commit_from_file(&model_path));

        match coreml_result {
            Ok(mut session) => {
                let input =
                    ort::value::Value::from_array((shape.as_slice(), data.clone())).expect("input");
                let outputs = session
                    .run(ort::inputs!["pixel_values" => input])
                    .expect("run");
                let (_, logits) = outputs["logits"].try_extract_tensor::<f32>().unwrap();

                let min = logits.iter().copied().fold(f32::INFINITY, f32::min);
                let max = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
                let text = (0..300)
                    .map(|q| logits[q * 17 + 9])
                    .fold(f32::NEG_INFINITY, f32::max);
                let sh = (0..300)
                    .map(|q| logits[q * 17 + 7])
                    .fold(f32::NEG_INFINITY, f32::max);

                (min, max, text, sh)
            }
            Err(e) => {
                println!("  ⚠️  CoreML provider not available: {e}");
                println!("  Skipping CoreML comparison");
                return;
            }
        }
    };

    println!("  Logits range: [{coreml_min:.6}, {coreml_max:.6}]");
    println!(
        "  Text max logit: {:.6}, conf: {:.6}",
        coreml_text,
        1.0 / (1.0 + (-coreml_text).exp())
    );
    println!(
        "  SH max logit: {:.6}, conf: {:.6}",
        coreml_sh,
        1.0 / (1.0 + (-coreml_sh).exp())
    );

    // Compare
    println!("\n--- Comparison ---");
    println!(
        "  Logits range diff: min={:.6}, max={:.6}",
        (cpu_min - coreml_min).abs(),
        (cpu_max - coreml_max).abs()
    );
    println!("  Text logit diff:  {:.6}", (cpu_text - coreml_text).abs());
    println!("  SH logit diff:    {:.6}", (cpu_sh - coreml_sh).abs());

    let text_conf_diff =
        (1.0 / (1.0 + (-cpu_text).exp()) - 1.0 / (1.0 + (-coreml_text).exp())).abs();
    let sh_conf_diff = (1.0 / (1.0 + (-cpu_sh).exp()) - 1.0 / (1.0 + (-coreml_sh).exp())).abs();

    println!("  Text conf diff:   {text_conf_diff:.6}");
    println!("  SH conf diff:     {sh_conf_diff:.6}");

    // Check if CoreML produces significantly different results
    let max_logit_diff = (cpu_max - coreml_max).abs();
    if max_logit_diff > 1.0 {
        println!(
            "\n❌ CoreML produces SIGNIFICANTLY different results! Max diff: {max_logit_diff:.6}"
        );
    } else {
        println!("\n✅ CoreML and CPU produce similar results");
    }

    println!(
        "\n================================================================================\n"
    );
}
