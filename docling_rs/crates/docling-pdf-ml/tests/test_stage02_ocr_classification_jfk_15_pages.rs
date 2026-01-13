/// Stage 0.2: OCR Classification Phase 1 Validation - All 15 JFK Pages (642 boxes)
///
/// **OCR Model:** PP-OCR mobile v2.0 Classification (AngleNet)
///
/// **Purpose:** Validates that Rust ONNX inference for Classification produces
/// IDENTICAL outputs to Python for all 642 text boxes across 15 JFK pages.
///
/// **Methodology:**
/// - For each page, for each detected box:
///   - Load Python baseline logits
///   - Load preprocessed input
///   - Run Rust Classification ONNX inference
///   - Compare: max_diff < 1e-4 (numerical precision ONLY)
///
/// **Success Criteria:**
/// - All 642 boxes: max_diff < 1e-4
/// - This proves Stage 0.2 is 100% correct
use anyhow::{Context, Result};
use ndarray::Array3;
use ort::session::Session;
use std::path::PathBuf;

#[test]
fn test_stage02_ocr_classification_jfk_15_pages() -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("Stage 0.2: OCR Classification - All 15 JFK Pages (642 boxes)");
    println!("{}", "=".repeat(80));
    println!("Goal: Prove Rust ONNX = Python ONNX for all boxes (max_diff < 1e-4)");
    println!();

    // Setup paths
    let home = std::env::var("HOME").context("HOME not set")?;
    let base_path = PathBuf::from(&home).join("docling_debug_pdf_parsing");
    let model_path = base_path.join("onnx_exports/rapidocr/ch_ppocr_mobile_v2.0_cls_infer.onnx");

    // Load Classification ONNX model once
    println!("[1] Loading Classification ONNX model...");
    let mut session = Session::builder()
        .context("Failed to create session builder")?
        .commit_from_file(&model_path)
        .context("Failed to load ONNX model")?;
    println!("  ✓ Model loaded\n");

    // Test all 15 pages
    let mut total_boxes = 0;
    let mut passed_boxes = 0;
    let mut failed_boxes = 0;

    for page_num in 0..15 {
        println!("[Page {page_num}]");

        let baseline_dir = base_path.join(format!(
            "baseline_data/jfk_scanned/page_{page_num}/ocr/classification_phase1"
        ));

        // Count boxes for this page
        let box_files: Vec<_> = std::fs::read_dir(&baseline_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().to_str().unwrap().contains("_logits.npy"))
            .collect();

        let num_boxes = box_files.len();
        println!("  Boxes: {num_boxes}");

        let mut page_passed = 0;
        let mut page_failed = 0;

        // Test each box
        for box_idx in 0..num_boxes {
            // Load preprocessed input
            let input_path = baseline_dir.join(format!("box_{box_idx:03}_preprocessed.npy"));
            let input_tensor: Array3<f32> = ndarray_npy::read_npy(&input_path)
                .with_context(|| format!("Failed to load input: {}", input_path.display()))?;

            // Add batch dimension: (3, 48, 192) -> (1, 3, 48, 192)
            let input_batch = input_tensor.insert_axis(ndarray::Axis(0));

            // Load Python baseline logits (shape: (1, 2))
            let logits_path = baseline_dir.join(format!("box_{box_idx:03}_logits.npy"));
            let python_logits: ndarray::Array2<f32> = ndarray_npy::read_npy(&logits_path)
                .with_context(|| format!("Failed to load logits: {}", logits_path.display()))?;

            // Run Rust ONNX inference
            let rust_logits = run_classification_inference(&mut session, input_batch.view())?;

            // Compare (both are (1, 2) shape)
            let max_diff = compute_max_abs_diff_2d(rust_logits.view(), python_logits.view());

            if max_diff < 1e-4 {
                page_passed += 1;
            } else {
                if page_failed == 0 {
                    println!("  ✗ Box failures:");
                }
                println!("    Box {box_idx}: max_diff = {max_diff:.10}");
                page_failed += 1;
            }

            total_boxes += 1;
        }

        if page_failed == 0 {
            println!("  ✓ PASS - All {num_boxes} boxes match");
        } else {
            println!("  ✗ FAIL - {page_failed} boxes mismatch");
        }

        passed_boxes += page_passed;
        failed_boxes += page_failed;
        println!();
    }

    // Summary
    println!("{}", "=".repeat(80));
    println!("SUMMARY:");
    println!("  Total boxes: {total_boxes}");
    println!("  Passed: {passed_boxes}");
    println!("  Failed: {failed_boxes}");
    println!("{}", "=".repeat(80));

    if failed_boxes > 0 {
        anyhow::bail!(
            "\n❌ {failed_boxes}/{total_boxes} boxes failed\nExpected all {total_boxes} boxes to match (max_diff < 1e-4)"
        );
    }

    println!("\n✅ ALL 642 BOXES ACROSS 15 PAGES MATCHED");
    println!("Stage 0.2 (OCR Classification) is VALIDATED\n");
    Ok(())
}

fn run_classification_inference(
    session: &mut Session,
    input: ndarray::ArrayView4<f32>,
) -> Result<ndarray::Array2<f32>> {
    // Convert ndarray to raw vec and shape
    let shape = input.shape().to_vec();
    let data = input.to_owned().into_raw_vec_and_offset().0;

    // Prepare input tensor
    let input_value = ort::value::Value::from_array((shape.as_slice(), data))
        .context("Failed to create input value")?;

    // Run inference
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
    if output_shape.len() != 2 {
        anyhow::bail!("Expected 2D output, got shape {output_shape:?}");
    }

    // Classification output is (1, 2)
    let array = ndarray::Array2::from_shape_vec(
        (output_shape[0] as usize, output_shape[1] as usize),
        output_data.to_vec(),
    )
    .context("Failed to create output array")?;

    Ok(array)
}

fn compute_max_abs_diff_2d(a: ndarray::ArrayView2<f32>, b: ndarray::ArrayView2<f32>) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).abs())
        .fold(0.0f32, f32::max)
}
