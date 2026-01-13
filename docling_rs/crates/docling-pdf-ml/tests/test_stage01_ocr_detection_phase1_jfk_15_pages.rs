/// Stage 0.1: OCR Detection Phase 1 Validation - All 15 JFK Pages
///
/// **OCR Model:** `PP-OCRv4` (`PaddleOCR` v4) - `DbNet` detection model
///
/// **Purpose:** Validates that Rust ONNX inference for OCR Detection produces
/// IDENTICAL outputs to Python for all 15 JFK scanned pages.
///
/// **Methodology:**
/// - For each page, load Python baseline probability map
/// - Run Rust Detection ONNX inference
/// - Compare: `max_diff < 1e-4` (numerical precision tolerance ONLY)
/// - SUCCESS = 15/15 pages pass
///
/// **Baseline Extraction:**
/// - Script: `extract_ocr_detection_jfk_15_pages_CORRECT.py`
/// - Python env: `venv_ocr` (Python 3.12 + `rapidocr-onnxruntime`)
/// - Generated: 2025-11-15 (N=640)
///
/// **Baseline Files (per page):**
/// ```
/// baseline_data/jfk_scanned/page_{N}/ocr/detection_phase1/
///   ├── preprocessed_input.npy        # (1, 3, H', W') f32
///   └── python_probability_map.npy    # (1, 1, H', W') f32 - STAGE 0.1 OUTPUT
/// ```
///
/// **Success Criteria:**
/// - All 15 pages: `max_diff < 1e-4`
/// - No exceptions allowed
/// - This proves Stage 0.1 is 100% correct
use anyhow::{Context, Result};
use ndarray::Array4;
use ort::session::Session;
use std::path::PathBuf;

#[test]
fn test_stage01_ocr_detection_phase1_jfk_15_pages() -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("Stage 0.1: OCR Detection Phase 1 - All 15 JFK Pages");
    println!("{}", "=".repeat(80));
    println!("Goal: Prove Rust ONNX = Python ONNX for all 15 pages (max_diff < 1e-4)");
    println!();

    // Setup paths
    let home = std::env::var("HOME").context("HOME not set")?;
    let base_path = PathBuf::from(&home).join("docling_debug_pdf_parsing");
    let model_dir = base_path.join("onnx_exports/rapidocr");

    // Load ONNX model once
    println!("[1] Loading Detection ONNX model...");
    let model_path = model_dir.join("ch_PP-OCRv4_det_infer.onnx");
    let mut session = Session::builder()
        .context("Failed to create session builder")?
        .commit_from_file(&model_path)
        .context("Failed to load ONNX model")?;
    println!("  ✓ Model loaded\n");

    // Test all 15 pages
    let mut results = Vec::new();

    for page_num in 0..15 {
        println!("[Page {page_num}]");

        let baseline_dir = base_path.join(format!(
            "baseline_data/jfk_scanned/page_{page_num}/ocr/detection_phase1"
        ));

        // Load preprocessed input
        let input_path = baseline_dir.join("preprocessed_input.npy");
        let input_tensor: Array4<f32> = ndarray_npy::read_npy(&input_path)
            .with_context(|| format!("Failed to load input: {}", input_path.display()))?;
        println!("  Input: {:?}", input_tensor.shape());

        // Load Python baseline output
        let python_output_path = baseline_dir.join("python_probability_map.npy");
        let python_output: Array4<f32> = ndarray_npy::read_npy(&python_output_path)
            .with_context(|| format!("Failed to load output: {}", python_output_path.display()))?;
        println!("  Python output: {:?}", python_output.shape());

        // Run Rust ONNX inference
        let rust_output = run_detection_inference(&mut session, input_tensor.view())?;
        println!("  Rust output: {:?}", rust_output.shape());

        // Verify shapes match
        assert_eq!(
            rust_output.shape(),
            python_output.shape(),
            "Page {page_num}: Output shape mismatch"
        );

        // Compare outputs
        let max_diff = compute_max_abs_diff(rust_output.view(), python_output.view());
        let mean_diff = compute_mean_abs_diff(rust_output.view(), python_output.view());

        println!("  Max diff:  {max_diff:.10}");
        println!("  Mean diff: {mean_diff:.10}");

        let passed = max_diff < 1e-4;
        let status = if passed { "✓ PASS" } else { "✗ FAIL" };
        println!("  {status}\n");

        results.push((page_num, max_diff, passed));
    }

    // Summary
    println!("{}", "=".repeat(80));
    let passed_count = results.iter().filter(|(_, _, p)| *p).count();
    println!("SUMMARY: {passed_count}/15 pages passed");
    println!("{}", "=".repeat(80));

    // Show failures
    let failures: Vec<_> = results.iter().filter(|(_, _, p)| !*p).collect();
    if !failures.is_empty() {
        println!("\nFAILURES:");
        for (page, max_diff, _) in failures {
            println!("  Page {page}: max_diff = {max_diff:.10}");
        }
    }

    // Assert all passed
    assert_eq!(
        passed_count,
        15,
        "\n❌ {}/15 pages failed\nExpected 100% match (max_diff < 1e-4) for all pages",
        15 - passed_count
    );

    println!("\n✅ ALL 15 JFK PAGES PASSED");
    println!("Stage 0.1 (OCR Detection) is VALIDATED\n");
    Ok(())
}

// Helper functions

fn run_detection_inference(
    session: &mut Session,
    input: ndarray::ArrayView4<f32>,
) -> Result<Array4<f32>> {
    // Convert ndarray to raw vec and shape
    let shape = input.shape().to_vec();
    let data = input.to_owned().into_raw_vec_and_offset().0;

    // Prepare input tensor
    let input_value = ort::value::Value::from_array((shape.as_slice(), data))
        .context("Failed to create input value")?;

    // Run inference (get input name from session)
    let input_name = session.inputs[0].name.clone();
    let output_name = session.outputs[0].name.clone();

    let outputs = session
        .run(ort::inputs![input_name.as_str() => input_value])
        .context("Failed to run inference")?;

    // Extract output
    let (output_shape, output_data) = outputs[output_name.as_str()]
        .try_extract_tensor::<f32>()
        .context("Failed to extract output tensor")?;

    // Convert back to ndarray
    if output_shape.len() != 4 {
        anyhow::bail!("Expected 4D output, got shape {output_shape:?}");
    }

    let array = Array4::from_shape_vec(
        (
            output_shape[0] as usize,
            output_shape[1] as usize,
            output_shape[2] as usize,
            output_shape[3] as usize,
        ),
        output_data.to_vec(),
    )
    .context("Failed to create output array")?;

    Ok(array)
}

fn compute_max_abs_diff(a: ndarray::ArrayView4<f32>, b: ndarray::ArrayView4<f32>) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).abs())
        .fold(0.0f32, f32::max)
}

fn compute_mean_abs_diff(a: ndarray::ArrayView4<f32>, b: ndarray::ArrayView4<f32>) -> f32 {
    let sum: f32 = a.iter().zip(b.iter()).map(|(x, y)| (x - y).abs()).sum();
    sum / (a.len() as f32)
}
