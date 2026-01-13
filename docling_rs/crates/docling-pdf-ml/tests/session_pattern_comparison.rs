/// Session Pattern Comparison Test
///
/// Tests whether with_intra_threads() affects ONNX inference results.
/// Phase 1 test (passes) uses: Session::builder().commit_from_file()
/// Production code (fails) uses: Session::builder().with_intra_threads().commit_from_file()
mod common;

use common::baseline_loaders::load_numpy;
use ort::session::Session;
use std::path::PathBuf;

#[test]
fn test_session_patterns_produce_same_results() {
    println!("\n================================================================================");
    println!("Session Pattern Comparison Test");
    println!("================================================================================");
    println!("Testing if with_intra_threads() affects results");
    println!();

    // Load ONNX model path
    let model_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("onnx_exports/layout_optimum/model.onnx");

    if !model_path.exists() {
        println!("⚠️  Skipping test - ONNX model not found at {model_path:?}");
        return;
    }

    // Load the known-working Phase 1 tensor
    let preprocessed_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("ml_model_inputs/layout_predictor/page_0_preprocessed_input.npy");

    if !preprocessed_path.exists() {
        println!("⚠️  Skipping test - preprocessed tensor not found");
        return;
    }

    let preprocessed = load_numpy(&preprocessed_path).expect("load tensor");
    println!("✓ Loaded tensor shape: {:?}", preprocessed.shape());

    let shape = preprocessed.shape().to_vec();
    let data = preprocessed.into_raw_vec_and_offset().0;

    // Test 1: Simple session (Phase 1 pattern)
    println!("\n--- Test 1: Simple Session (Phase 1 pattern) ---");
    let (simple_min, simple_max, simple_text, simple_sh) = {
        let mut session = Session::builder()
            .expect("builder")
            .commit_from_file(&model_path)
            .expect("load");

        let input_value =
            ort::value::Value::from_array((shape.as_slice(), data.clone())).expect("input");
        let outputs = session
            .run(ort::inputs!["pixel_values" => input_value])
            .expect("run");

        let (_, logits_data) = outputs["logits"].try_extract_tensor::<f32>().unwrap();

        let min = logits_data.iter().copied().fold(f32::INFINITY, f32::min);
        let max = logits_data
            .iter()
            .copied()
            .fold(f32::NEG_INFINITY, f32::max);

        let num_queries = 300;
        let num_classes = 17;
        let text_max = (0..num_queries)
            .map(|q| logits_data[q * num_classes + 9])
            .fold(f32::NEG_INFINITY, f32::max);
        let sh_max = (0..num_queries)
            .map(|q| logits_data[q * num_classes + 7])
            .fold(f32::NEG_INFINITY, f32::max);

        println!("  Logits range: [{min:.6}, {max:.6}]");
        println!(
            "  Text max logit: {:.6}, conf: {:.6}",
            text_max,
            1.0 / (1.0 + (-text_max).exp())
        );
        println!(
            "  SH max logit: {:.6}, conf: {:.6}",
            sh_max,
            1.0 / (1.0 + (-sh_max).exp())
        );

        (min, max, text_max, sh_max)
    };

    // Test 2: With intra_threads (production pattern)
    println!("\n--- Test 2: With intra_threads(4) (production pattern) ---");
    let (threads_min, threads_max, threads_text, threads_sh) = {
        let mut session = Session::builder()
            .expect("builder")
            .with_intra_threads(4)
            .expect("threads")
            .commit_from_file(&model_path)
            .expect("load");

        let input_value =
            ort::value::Value::from_array((shape.as_slice(), data.clone())).expect("input");
        let outputs = session
            .run(ort::inputs!["pixel_values" => input_value])
            .expect("run");

        let (_, logits_data) = outputs["logits"].try_extract_tensor::<f32>().unwrap();

        let min = logits_data.iter().copied().fold(f32::INFINITY, f32::min);
        let max = logits_data
            .iter()
            .copied()
            .fold(f32::NEG_INFINITY, f32::max);

        let num_queries = 300;
        let num_classes = 17;
        let text_max = (0..num_queries)
            .map(|q| logits_data[q * num_classes + 9])
            .fold(f32::NEG_INFINITY, f32::max);
        let sh_max = (0..num_queries)
            .map(|q| logits_data[q * num_classes + 7])
            .fold(f32::NEG_INFINITY, f32::max);

        println!("  Logits range: [{min:.6}, {max:.6}]");
        println!(
            "  Text max logit: {:.6}, conf: {:.6}",
            text_max,
            1.0 / (1.0 + (-text_max).exp())
        );
        println!(
            "  SH max logit: {:.6}, conf: {:.6}",
            sh_max,
            1.0 / (1.0 + (-sh_max).exp())
        );

        (min, max, text_max, sh_max)
    };

    // Compare results
    println!("\n================================================================================");
    println!("Comparison Results");
    println!("================================================================================");

    let min_diff = (simple_min - threads_min).abs();
    let max_diff = (simple_max - threads_max).abs();
    let text_diff = (simple_text - threads_text).abs();
    let sh_diff = (simple_sh - threads_sh).abs();

    println!("Min logit diff: {min_diff:.10}");
    println!("Max logit diff: {max_diff:.10}");
    println!("Text max diff:  {text_diff:.10}");
    println!("SH max diff:    {sh_diff:.10}");

    // Tolerance for numerical precision
    let tolerance = 1e-3;

    let all_match = min_diff < tolerance
        && max_diff < tolerance
        && text_diff < tolerance
        && sh_diff < tolerance;

    if all_match {
        println!("\n✅ Both session patterns produce the same results");
        println!("   The with_intra_threads() call does NOT affect accuracy");
    } else {
        println!("\n❌ Session patterns produce DIFFERENT results!");
        println!("   The with_intra_threads() call DOES affect accuracy");
    }

    println!("================================================================================\n");

    assert!(
        all_match,
        "Session patterns should produce same results within tolerance"
    );
}
