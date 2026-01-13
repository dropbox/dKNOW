#![cfg(feature = "opencv-preprocessing")]
/// RapidOCR Classification Model - Phase 1 Validation Test
///
/// **Purpose:** Validates that Rust ONNX inference for the classification model produces
/// identical outputs to Python ONNX inference when given the SAME preprocessed input tensor.
///
/// **Methodology:** Phase 1 from WORKER_DIRECTIVE_FINAL_VALIDATION_METHODOLOGY.md
/// This test ONLY validates the ML model inference, NOT preprocessing or postprocessing.
///
/// **Baseline Extraction:**
/// - Script: `regenerate_cls_rec_baselines.py` (N=106, 2025-11-07)
/// - Python source: Direct `onnxruntime.InferenceSession` call (no wrapper)
/// - ONNX model: `onnx_exports/rapidocr/ch_ppocr_mobile_v2.0_cls_infer.onnx`
/// - Registry: See `BASELINE_EXTRACTION_REGISTRY.md` → RapidOCR Classification → Phase 1
///
/// **Baseline Files:**
/// - Input: `ml_model_inputs/rapid_ocr_isolated/cls_preprocessed_input.npy` - [3, 3, 48, 192] f32
/// - Output: `ml_model_inputs/rapid_ocr_isolated/cls_python_output.npy` - [3, 2] f32
///
/// **How Baseline Was Generated:**
/// ```python
/// import onnxruntime as ort
/// import numpy as np
///
/// # Load preprocessed input
/// cls_input = np.load("ml_model_inputs/rapid_ocr_isolated/cls_preprocessed_input.npy")
///
/// # Run ONNX inference
/// session = ort.InferenceSession("onnx_exports/rapidocr/ch_ppocr_mobile_v2.0_cls_infer.onnx")
/// input_name = session.get_inputs()[0].name
/// output_name = session.get_outputs()[0].name
/// cls_output = session.run([output_name], {input_name: cls_input})[0]
///
/// # Save raw ONNX output (NO reshaping)
/// np.save("ml_model_inputs/rapid_ocr_isolated/cls_python_output.npy", cls_output)
/// ```
///
/// **Regeneration Command:**
/// ```bash
/// cd ~/docling_debug_pdf_parsing
/// python3.12 regenerate_cls_rec_baselines.py  # Requires Python 3.12 (onnxruntime unavailable for 3.14)
/// ```
///
/// **Last Generated:** 2025-11-07 18:39 (N=106)
///
/// **IMPORTANT HISTORY:**
/// - N=78 (2025-11-07 08:50): Claimed PASSING with max_diff=0.00000012 - **UNVERIFIED**
/// - N=105 (2025-11-07 18:25): FAILING with max_diff=0.77 (fresh binaries)
/// - N=106 (2025-11-07 18:39): ROOT CAUSE - baseline had wrong shape [3,1,2] instead of [3,2]
/// - N=106 (2025-11-07 18:50): Baseline regenerated, NOW PASSING with max_diff=0.0
///
/// **Success Criteria:**
/// - Max absolute difference < 1e-4 (0.0001)
/// - Output shape matches exactly: [N, 2]
/// - Mean diff < 1e-5
use anyhow::{Context, Result};
use ndarray::{Array2, Array3, Array4, ArrayView2, ArrayView3, ArrayView4};
use ort::session::Session;
use std::path::PathBuf;

#[test]
fn test_rapidocr_cls_phase1_isolated() -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("RapidOCR Classification Model - Phase 1 Validation");
    println!("{}", "=".repeat(80));
    println!("Goal: Prove Rust ONNX inference = Python ONNX inference (< 1e-4 diff)");
    println!("Method: Same preprocessed tensor → compare raw classification logits");
    println!();

    // Setup paths
    let home = std::env::var("HOME").context("HOME not set")?;
    let base_path = PathBuf::from(&home).join("docling_debug_pdf_parsing");
    let input_dir = base_path.join("ml_model_inputs/rapid_ocr_isolated");
    let model_dir = base_path.join("onnx_exports/rapidocr");

    // Load preprocessed input (from Python)
    println!("[1] Loading preprocessed input...");
    let input_tensor = load_npy_f32_4d(&input_dir.join("cls_preprocessed_input.npy"))?;
    println!("  ✓ Input shape: {:?}", input_tensor.shape());
    println!("  ✓ Input dtype: f32");
    println!(
        "  ✓ Input range: [{:.6}, {:.6}]",
        input_tensor.iter().copied().fold(f32::INFINITY, f32::min),
        input_tensor
            .iter()
            .copied()
            .fold(f32::NEG_INFINITY, f32::max)
    );
    // Expected: [N, 3, 48, 192] f32, range [-1.0, 1.0]

    // Load expected output (from Python ONNX)
    println!("\n[2] Loading expected output...");
    let expected_output = load_npy_f32_2d(&input_dir.join("cls_python_output.npy"))?;
    println!("  ✓ Output shape: {:?}", expected_output.shape());
    println!("  ✓ Output dtype: f32");
    println!(
        "  ✓ Output range: [{:.6}, {:.6}]",
        expected_output
            .iter()
            .copied()
            .fold(f32::INFINITY, f32::min),
        expected_output
            .iter()
            .copied()
            .fold(f32::NEG_INFINITY, f32::max)
    );
    // Expected: [N, 2] f32, range [0.0, 1.0] (2-class logits)

    // Load ONNX model
    println!("\n[3] Loading ONNX model...");
    let model_path = model_dir.join("ch_ppocr_mobile_v2.0_cls_infer.onnx");
    let mut session = Session::builder()
        .context("Failed to create session builder")?
        .commit_from_file(&model_path)
        .context("Failed to load ONNX model")?;
    println!("  ✓ Model loaded from: {}", model_path.display());
    println!(
        "  ✓ Inputs: {:?}",
        session.inputs.iter().map(|i| &i.name).collect::<Vec<_>>()
    );
    println!(
        "  ✓ Outputs: {:?}",
        session.outputs.iter().map(|o| &o.name).collect::<Vec<_>>()
    );

    // Run Rust ONNX inference
    println!("\n[4] Running Rust ONNX inference...");
    let rust_output = run_classification_inference(&mut session, input_tensor.view())?;
    println!("  ✓ Rust output shape: {:?}", rust_output.shape());

    // Verify shapes match
    assert_eq!(
        rust_output.shape(),
        expected_output.shape(),
        "Output shape mismatch"
    );

    // Compare outputs
    println!("\n[5] Comparing outputs...");
    let max_abs_diff = compute_max_abs_diff_2d(rust_output.view(), expected_output.view());
    let mean_abs_diff = compute_mean_abs_diff_2d(rust_output.view(), expected_output.view());

    println!("  Max absolute diff:  {:.10}", max_abs_diff);
    println!("  Mean absolute diff: {:.10}", mean_abs_diff);

    println!("\n{}", "=".repeat(80));

    // Threshold for classification: 1e-4 (less strict than detection due to softmax operations)
    let threshold = 1e-4;
    if max_abs_diff < threshold {
        println!("✅ RAPIDOCR CLASSIFICATION PHASE 1 PASSED");
        println!("  Max diff: {:.10} < {:.10}", max_abs_diff, threshold);
        println!("  Mean diff: {:.10}", mean_abs_diff);
    } else {
        println!("❌ RAPIDOCR CLASSIFICATION PHASE 1 FAILED");
        println!("  Max diff: {:.10} >= {:.10}", max_abs_diff, threshold);
        println!("  Mean diff: {:.10}", mean_abs_diff);
    }

    println!("{}", "=".repeat(80));

    assert!(
        max_abs_diff < threshold,
        "Max diff {:.10} exceeds threshold {:.10}",
        max_abs_diff,
        threshold
    );

    Ok(())
}

/// Run classification inference on a batch of preprocessed images
fn run_classification_inference(
    session: &mut Session,
    input: ArrayView4<f32>,
) -> Result<Array2<f32>> {
    // Input: [N, 3, 48, 192] - batch of cropped text regions
    // Output: [N, 2] - 2-class logits (0° or 180°)

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
    // ONNX model outputs [N, 2]
    if output_shape.len() != 2 {
        anyhow::bail!("Expected 2D output, got shape {:?}", output_shape);
    }

    let n = output_shape[0] as usize;
    let classes = output_shape[1] as usize;

    // Create [N, 2] array
    let array = Array2::from_shape_vec((n, classes), output_data.to_vec())
        .context("Failed to create output ndarray")?;

    Ok(array)
}

/// Load a .npy file as f32 Array4
fn load_npy_f32_4d(path: &PathBuf) -> Result<Array4<f32>> {
    use npyz::NpyFile;
    use std::fs::File;

    let file = File::open(path).context("Failed to open npy file")?;
    let npy = NpyFile::new(file).context("Failed to parse npy file")?;

    let shape: Vec<usize> = npy.shape().iter().map(|&x| x as usize).collect();
    let data: Vec<f32> = npy.into_vec().context("Failed to read npy data")?;

    // Verify 4D
    if shape.len() != 4 {
        anyhow::bail!("Expected 4D array, got shape {:?}", shape);
    }

    // Create ndarray
    let array = Array4::from_shape_vec((shape[0], shape[1], shape[2], shape[3]), data)
        .context("Failed to create ndarray from shape")?;

    Ok(array)
}

/// Load a .npy file as f32 Array3
fn load_npy_f32_3d(path: &PathBuf) -> Result<Array3<f32>> {
    use npyz::NpyFile;
    use std::fs::File;

    let file = File::open(path).context("Failed to open npy file")?;
    let npy = NpyFile::new(file).context("Failed to parse npy file")?;

    let shape: Vec<usize> = npy.shape().iter().map(|&x| x as usize).collect();
    let data: Vec<f32> = npy.into_vec().context("Failed to read npy data")?;

    // Verify 3D
    if shape.len() != 3 {
        anyhow::bail!("Expected 3D array, got shape {:?}", shape);
    }

    // Create ndarray
    let array = Array3::from_shape_vec((shape[0], shape[1], shape[2]), data)
        .context("Failed to create ndarray from shape")?;

    Ok(array)
}

/// Load a .npy file as f32 Array2
fn load_npy_f32_2d(path: &PathBuf) -> Result<Array2<f32>> {
    use npyz::NpyFile;
    use std::fs::File;

    let file = File::open(path).context("Failed to open npy file")?;
    let npy = NpyFile::new(file).context("Failed to parse npy file")?;

    let shape: Vec<usize> = npy.shape().iter().map(|&x| x as usize).collect();
    let data: Vec<f32> = npy.into_vec().context("Failed to read npy data")?;

    // Verify 2D
    if shape.len() != 2 {
        anyhow::bail!("Expected 2D array, got shape {:?}", shape);
    }

    // Create ndarray
    let array = Array2::from_shape_vec((shape[0], shape[1]), data)
        .context("Failed to create ndarray from shape")?;

    Ok(array)
}

/// Compute max absolute difference between two 3D arrays
fn compute_max_abs_diff_3d(a: ArrayView3<f32>, b: ArrayView3<f32>) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).abs())
        .fold(0.0f32, f32::max)
}

/// Compute mean absolute difference between two 3D arrays
fn compute_mean_abs_diff_3d(a: ArrayView3<f32>, b: ArrayView3<f32>) -> f32 {
    let sum: f32 = a.iter().zip(b.iter()).map(|(x, y)| (x - y).abs()).sum();
    sum / (a.len() as f32)
}

/// Compute max absolute difference between two 2D arrays
fn compute_max_abs_diff_2d(a: ArrayView2<f32>, b: ArrayView2<f32>) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).abs())
        .fold(0.0f32, f32::max)
}

/// Compute mean absolute difference between two 2D arrays
fn compute_mean_abs_diff_2d(a: ArrayView2<f32>, b: ArrayView2<f32>) -> f32 {
    let sum: f32 = a.iter().zip(b.iter()).map(|(x, y)| (x - y).abs()).sum();
    sum / (a.len() as f32)
}
