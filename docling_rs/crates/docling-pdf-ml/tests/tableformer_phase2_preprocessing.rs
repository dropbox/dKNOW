#![cfg(feature = "pytorch")]
/// Test: TableFormer Phase 2 - Preprocessing Validation
///
/// Purpose: Validate that Rust preprocessing produces identical outputs to Python baseline.
///
/// This follows the 3-phase validation methodology from WORKER_DIRECTIVE_FINAL_VALIDATION_METHODOLOGY.md:
///
/// Phase 1: ML Model Isolated (✅ COMPLETE - passing)
/// Phase 2 (THIS TEST): Preprocessing validation
/// - Load raw table image (SAME input for both Python and Rust)
/// - Run Rust preprocessing on raw image
/// - Compare Rust preprocessed output with Python preprocessed output
/// - Success criteria: max diff < 0.02 pixels (CLAUDE.md)
///
/// Phase 3 (Later): End-to-end validation
///
/// Test Strategy:
/// 1. Load raw table image: table_0_raw_image.npy [225, 418, 3] uint8
/// 2. Load Python preprocessed output: table_0_preprocessed_input.npy [1, 3, 448, 448] float32
/// 3. Run Rust preprocessing on raw image
/// 4. Compare outputs within tolerance
///
/// Success Criteria:
/// ✅ Max pixel difference < 0.02 (adjusted from 0.01 per N=83 decision)
///
/// Run with:
/// cargo test --release --test tableformer_phase2_preprocessing -- --ignored --nocapture
mod common;
use common::baseline_loaders::{load_numpy, load_numpy_u8_as_f32};
use docling_pdf_ml::preprocessing::tableformer::tableformer_preprocess;
use ndarray::{Array3, Array4, ArrayD};
use std::path::PathBuf;

// Calculate max absolute difference between two arrays
fn max_difference_4d(a: &Array4<f32>, b: &Array4<f32>) -> f32 {
    assert_eq!(a.shape(), b.shape(), "Arrays must have same shape");
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).abs())
        .fold(0.0f32, f32::max)
}

#[test]
#[ignore = "Requires baseline data"]
fn test_tableformer_phase2_preprocessing() {
    println!("\n=== TableFormer Phase 2: Preprocessing Validation ===\n");
    println!(
        "Following WORKER_DIRECTIVE methodology: Same raw image → Compare preprocessed outputs\n"
    );

    // ========================================
    // Step 1: Load Baseline Data
    // ========================================
    println!("Step 1: Loading Python baseline data...");

    let home = std::env::var("HOME").unwrap();
    let base_path =
        PathBuf::from(&home).join("docling_debug_pdf_parsing/ml_model_inputs/tableformer");

    // Load raw table image (Phase 2 input) [225, 418, 3] uint8
    let raw_image_path = base_path.join("table_0_raw_image.npy");
    println!("  Loading raw table image: {}", raw_image_path.display());
    let raw_image_arr_f32: ArrayD<f32> =
        load_numpy_u8_as_f32(&raw_image_path).expect("Failed to load raw image");
    println!("  ✓ Raw image shape: {:?}", raw_image_arr_f32.shape());

    // Convert from f32 to u8 (numpy saves as float32 by default)
    let shape = raw_image_arr_f32.shape();
    assert_eq!(shape.len(), 3, "Raw image must be 3D (H, W, C)");
    let (h, w, c) = (shape[0], shape[1], shape[2]);
    let raw_image = Array3::<u8>::from_shape_vec(
        (h, w, c),
        raw_image_arr_f32.iter().map(|&x| x as u8).collect(),
    )
    .expect("Failed to reshape raw image");

    println!("  ✓ Reshaped to Array3: ({}, {}, {})", h, w, c);

    // Load Python preprocessed output (Phase 2 expected output) [1, 3, 448, 448] float32
    let preprocessed_path = base_path.join("table_0_preprocessed_input.npy");
    println!(
        "  Loading Python preprocessed output: {}",
        preprocessed_path.display()
    );
    let expected_preprocessed_arr: ArrayD<f32> =
        load_numpy(&preprocessed_path).expect("Failed to load preprocessed output");
    println!(
        "  ✓ Expected preprocessed shape: {:?}",
        expected_preprocessed_arr.shape()
    );

    // Convert to Array4
    let shape = expected_preprocessed_arr.shape();
    assert_eq!(shape.len(), 4, "Preprocessed must be 4D (B, C, H, W)");
    let (b, c, h_out, w_out) = (shape[0], shape[1], shape[2], shape[3]);
    let expected_preprocessed = Array4::<f32>::from_shape_vec(
        (b, c, h_out, w_out),
        expected_preprocessed_arr.iter().copied().collect(),
    )
    .expect("Failed to reshape preprocessed output");

    println!(
        "  ✓ Reshaped to Array4: ({}, {}, {}, {})",
        b, c, h_out, w_out
    );

    // ========================================
    // Step 2: Run Rust Preprocessing
    // ========================================
    println!("\nStep 2: Running Rust preprocessing on SAME raw image...");

    let rust_preprocessed = tableformer_preprocess(&raw_image);

    println!("  ✓ Rust preprocessing complete");
    println!(
        "    Rust preprocessed shape: {:?}",
        rust_preprocessed.shape()
    );

    // Verify shapes match
    assert_eq!(
        rust_preprocessed.shape(),
        expected_preprocessed.shape(),
        "Preprocessed shapes must match"
    );

    // ========================================
    // Step 3: Compare Outputs
    // ========================================
    println!("\nStep 3: Comparing preprocessed outputs...");

    let max_diff = max_difference_4d(&rust_preprocessed, &expected_preprocessed);

    println!("  Max pixel difference: {:.10}", max_diff);

    // Print some statistics
    let rust_min = rust_preprocessed
        .iter()
        .copied()
        .fold(f32::INFINITY, f32::min);
    let rust_max = rust_preprocessed
        .iter()
        .copied()
        .fold(f32::NEG_INFINITY, f32::max);
    let expected_min = expected_preprocessed
        .iter()
        .copied()
        .fold(f32::INFINITY, f32::min);
    let expected_max = expected_preprocessed
        .iter()
        .copied()
        .fold(f32::NEG_INFINITY, f32::max);

    println!("\n  Value ranges:");
    println!("    Rust:     min={:.6}, max={:.6}", rust_min, rust_max);
    println!(
        "    Expected: min={:.6}, max={:.6}",
        expected_min, expected_max
    );

    // Print first 10 values from each array for debugging
    println!("\n  First 10 values:");
    print!("    Rust:     [");
    for i in 0..10.min(rust_preprocessed.len()) {
        print!("{:.6}", rust_preprocessed.iter().nth(i).unwrap());
        if i < 9 {
            print!(", ");
        }
    }
    println!("]");

    print!("    Expected: [");
    for i in 0..10.min(expected_preprocessed.len()) {
        print!("{:.6}", expected_preprocessed.iter().nth(i).unwrap());
        if i < 9 {
            print!(", ");
        }
    }
    println!("]");

    // ========================================
    // Step 4: Validate
    // ========================================
    println!("\nStep 4: Validation...");

    // Threshold: 0.02 pixels (per CLAUDE.md Phase 2 threshold, adjusted in N=83)
    let threshold = 0.02;

    if max_diff < threshold {
        println!("  ✅ PREPROCESSING VALIDATION PASSED");
        println!(
            "    Max difference: {:.10} < {:.2} threshold",
            max_diff, threshold
        );
    } else {
        println!("  ❌ PREPROCESSING VALIDATION FAILED");
        println!(
            "    Max difference: {:.10} >= {:.2} threshold",
            max_diff, threshold
        );

        // Find location of max difference for debugging
        let mut max_idx = (0, 0, 0, 0);
        let mut max_val = 0.0f32;
        for b_idx in 0..b {
            for c_idx in 0..c {
                for h_idx in 0..h_out {
                    for w_idx in 0..w_out {
                        let diff = (rust_preprocessed[[b_idx, c_idx, h_idx, w_idx]]
                            - expected_preprocessed[[b_idx, c_idx, h_idx, w_idx]])
                        .abs();
                        if diff > max_val {
                            max_val = diff;
                            max_idx = (b_idx, c_idx, h_idx, w_idx);
                        }
                    }
                }
            }
        }

        println!(
            "\n    Max difference location: [{}, {}, {}, {}]",
            max_idx.0, max_idx.1, max_idx.2, max_idx.3
        );
        println!("      Rust:     {:.10}", rust_preprocessed[max_idx]);
        println!("      Expected: {:.10}", expected_preprocessed[max_idx]);

        panic!(
            "Preprocessing validation FAILED: max diff {:.10} >= {:.2}",
            max_diff, threshold
        );
    }

    // ========================================
    // Final Summary
    // ========================================
    println!("\n=== Phase 2 Validation Summary ===");
    println!("✅ TABLEFORMER PHASE 2 COMPLETE!");
    println!("\nResults:");
    println!(
        "  ✅ Preprocessing: max diff {:.10} < {:.2} threshold",
        max_diff, threshold
    );
    println!("\nPreprocessing is VALIDATED:");
    println!("  - Normalization: Correct");
    println!("  - Resize (448x448): Correct");
    println!("  - Transpose (CWH): Correct");
    println!("  - Scaling (/255): Correct");
    println!("\nNext: Phase 3 end-to-end validation or move to next model");
}
