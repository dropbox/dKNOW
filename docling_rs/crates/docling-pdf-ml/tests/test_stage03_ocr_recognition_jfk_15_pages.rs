/// Stage 0.3: OCR Recognition Phase 1 Validation - All 15 JFK Pages (642 boxes)
///
/// **OCR Model:** PP-OCRv4 Recognition (CrnnNet)
///
/// **Purpose:** Validates that Rust ONNX inference for Recognition produces
/// IDENTICAL outputs to Python for all 642 text boxes across 15 JFK pages.
///
/// **Success Criteria:**
/// - All 642 boxes: max_diff < 1e-4
use anyhow::{Context, Result};
use ndarray::Array3;
use ort::session::Session;
use std::path::PathBuf;

#[test]
fn test_stage03_ocr_recognition_jfk_15_pages() -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("Stage 0.3: OCR Recognition - All 15 JFK Pages (642 boxes)");
    println!("{}", "=".repeat(80));
    println!("Goal: Prove Rust ONNX = Python ONNX for all boxes (max_diff < 1e-4)");
    println!();

    // Setup
    let home = std::env::var("HOME").context("HOME not set")?;
    let base_path = PathBuf::from(&home).join("docling_debug_pdf_parsing");
    let model_path = base_path.join("onnx_exports/rapidocr/ch_PP-OCRv4_rec_infer.onnx");

    println!("[1] Loading Recognition ONNX model...");
    let mut session = Session::builder()?.commit_from_file(&model_path)?;
    println!("  ✓ Model loaded\n");

    let mut total_boxes = 0;
    let mut passed_boxes = 0;
    let mut failed_boxes = 0;

    for page_num in 0..15 {
        println!("[Page {page_num}]");

        let baseline_dir = base_path.join(format!(
            "baseline_data/jfk_scanned/page_{page_num}/ocr/recognition_phase1"
        ));

        // Count boxes
        let box_files: Vec<_> = std::fs::read_dir(&baseline_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().to_str().unwrap().contains("_logits.npy"))
            .collect();

        let num_boxes = box_files.len();
        println!("  Boxes: {num_boxes}");

        let mut page_passed = 0;
        let mut page_failed = 0;

        for box_idx in 0..num_boxes {
            // Load preprocessed input (3, 48, W) where W varies
            let input_path = baseline_dir.join(format!("box_{box_idx:03}_preprocessed.npy"));
            let input_tensor: Array3<f32> = ndarray_npy::read_npy(&input_path)?;

            // Add batch dimension
            let input_batch = input_tensor.insert_axis(ndarray::Axis(0));

            // Load Python logits (1, seq_len, 6625)
            let logits_path = baseline_dir.join(format!("box_{box_idx:03}_logits.npy"));
            let python_logits: ndarray::ArrayD<f32> = ndarray_npy::read_npy(&logits_path)?;

            // Run Rust
            let rust_logits = run_recognition_inference(&mut session, input_batch.view())?;

            // Compare
            let max_diff = compute_max_abs_diff(&rust_logits, &python_logits);

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

    println!("{}", "=".repeat(80));
    println!("SUMMARY:");
    println!("  Total boxes: {total_boxes}");
    println!("  Passed: {passed_boxes}");
    println!("  Failed: {failed_boxes}");
    println!("{}", "=".repeat(80));

    if failed_boxes > 0 {
        anyhow::bail!("\n❌ {failed_boxes}/{total_boxes} boxes failed");
    }

    println!("\n✅ ALL 642 BOXES MATCHED");
    println!("Stage 0.3 (OCR Recognition) is VALIDATED\n");
    Ok(())
}

fn run_recognition_inference(
    session: &mut Session,
    input: ndarray::ArrayView4<f32>,
) -> Result<ndarray::ArrayD<f32>> {
    let shape = input.shape().to_vec();
    let data = input.to_owned().into_raw_vec_and_offset().0;

    let input_value = ort::value::Value::from_array((shape.as_slice(), data))?;
    let input_name = session.inputs[0].name.clone();
    let output_name = session.outputs[0].name.clone();

    let outputs = session.run(ort::inputs![input_name.as_str() => input_value])?;

    let (output_shape, output_data) = outputs[output_name.as_str()].try_extract_tensor::<f32>()?;

    // Create dynamic array with actual output shape
    let array = ndarray::ArrayD::from_shape_vec(
        output_shape.iter().map(|&x| x as usize).collect::<Vec<_>>(),
        output_data.to_vec(),
    )?;

    Ok(array)
}

fn compute_max_abs_diff(a: &ndarray::ArrayD<f32>, b: &ndarray::ArrayD<f32>) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).abs())
        .fold(0.0f32, f32::max)
}
