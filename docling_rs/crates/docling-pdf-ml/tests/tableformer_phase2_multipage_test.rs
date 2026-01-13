#![cfg(feature = "pytorch")]
/// Test: TableFormer Phase 2 Multi-Page - Preprocessing Validation on All 30 Tables
///
/// Purpose: Validate Rust preprocessing matches Python baseline on ALL tables.
///
/// Following N=120-121 pattern for LayoutPredictor multi-page validation:
/// - Load raw table image (Phase 2 input)
/// - Run Rust preprocessing
/// - Compare with Python preprocessed output
/// - Report individual failures, total pass rate
///
/// Success Criteria:
/// ✅ Max pixel difference < 0.02 (per CLAUDE.md Phase 2 threshold, adjusted in N=83)
///
/// Expected: 30/30 tables passing (100%)
///
/// Run with:
/// cargo test --release --test tableformer_phase2_multipage_test -- --nocapture
mod common;
use common::baseline_loaders::{load_numpy, load_numpy_u8_as_f32};
use docling_pdf_ml::preprocessing::tableformer::tableformer_preprocess;
use ndarray::{Array3, Array4, ArrayD};
use std::path::PathBuf;

// Test pages and table counts (same as Phase 1)
static TEST_TABLES: &[(&str, &str, usize)] = &[
    ("arxiv_2206.01062", "page_03", 1),
    ("arxiv_2206.01062", "page_05", 1),
    ("arxiv_2206.01062", "page_06", 2),
    ("arxiv_2206.01062", "page_07", 1),
    ("edinet_sample", "page_01", 1),
    ("edinet_sample", "page_04", 3),
    ("edinet_sample", "page_05", 2),
    ("edinet_sample", "page_06", 2),
    ("edinet_sample", "page_08", 1),
    ("edinet_sample", "page_09", 1),
    ("edinet_sample", "page_10", 1),
    ("edinet_sample", "page_11", 1),
    ("edinet_sample", "page_12", 1),
    ("edinet_sample", "page_13", 2),
    ("edinet_sample", "page_14", 5),
    ("edinet_sample", "page_15", 2),
    ("edinet_sample", "page_16", 2),
    ("edinet_sample", "page_17", 1),
];

// Helper: Calculate max absolute difference between two 4D arrays
fn max_difference_4d(a: &Array4<f32>, b: &Array4<f32>) -> f32 {
    assert_eq!(a.shape(), b.shape(), "Arrays must have same shape");
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).abs())
        .fold(0.0f32, f32::max)
}

#[derive(Default)]
struct TestResult {
    pass: bool,
    max_diff: f32,
}

fn test_single_table(
    base_path: &PathBuf,
    pdf_name: &str,
    page_name: &str,
    table_idx: usize,
) -> Result<TestResult, String> {
    let table_dir = base_path
        .join(pdf_name)
        .join(page_name)
        .join(format!("table_{}", table_idx));

    if !table_dir.exists() {
        return Err(format!(
            "Table directory not found: {}",
            table_dir.display()
        ));
    }

    // Load raw image (uint8)
    let raw_image_path = table_dir.join(format!("table_{}_raw_image.npy", table_idx));
    let raw_image_arr: ArrayD<f32> = load_numpy_u8_as_f32(&raw_image_path)
        .map_err(|e| format!("Failed to load raw image: {}", e))?;

    // Convert to u8 Array3
    let shape = raw_image_arr.shape();
    if shape.len() != 3 {
        return Err(format!("Expected 3D raw image, got {:?}", shape));
    }
    let (h, w, c) = (shape[0], shape[1], shape[2]);
    let raw_image =
        Array3::<u8>::from_shape_vec((h, w, c), raw_image_arr.iter().map(|&x| x as u8).collect())
            .map_err(|e| format!("Failed to reshape raw image: {}", e))?;

    // Load expected preprocessed output
    let preprocessed_path = table_dir.join(format!("table_{}_preprocessed_input.npy", table_idx));
    let expected_preprocessed_arr: ArrayD<f32> = load_numpy(&preprocessed_path)
        .map_err(|e| format!("Failed to load preprocessed output: {}", e))?;

    // Convert to Array4
    let shape = expected_preprocessed_arr.shape();
    if shape.len() != 4 {
        return Err(format!("Expected 4D preprocessed, got {:?}", shape));
    }
    let (b, c, h_out, w_out) = (shape[0], shape[1], shape[2], shape[3]);
    let expected_preprocessed = Array4::<f32>::from_shape_vec(
        (b, c, h_out, w_out),
        expected_preprocessed_arr.iter().copied().collect(),
    )
    .map_err(|e| format!("Failed to reshape preprocessed: {}", e))?;

    // Run Rust preprocessing
    let rust_preprocessed = tableformer_preprocess(&raw_image);

    // Compare
    if rust_preprocessed.shape() != expected_preprocessed.shape() {
        return Err(format!(
            "Shape mismatch: Rust {:?} vs Expected {:?}",
            rust_preprocessed.shape(),
            expected_preprocessed.shape()
        ));
    }

    let max_diff = max_difference_4d(&rust_preprocessed, &expected_preprocessed);
    let pass = max_diff < 0.02;

    Ok(TestResult { pass, max_diff })
}

#[test]
fn test_tableformer_phase2_all_tables() {
    println!("\n{}", "=".repeat(80));
    println!("Phase 2 Multi-Page Validation: TableFormer Preprocessing");
    println!("{}", "=".repeat(80));
    println!("Goal: Prove Rust preprocessing = Python preprocessing on ALL 30 tables");
    println!("Method: Same raw image → compare preprocessed tensors");
    println!("Success: Max diff < 0.02 pixels (N=83 threshold)");
    println!();

    // Setup
    let home = std::env::var("HOME").unwrap();
    let base_path =
        PathBuf::from(&home).join("docling_debug_pdf_parsing/ml_model_inputs_multipage");

    // Test all tables
    let mut total_tested = 0;
    let mut total_passed = 0;
    let mut total_failed = 0;

    for &(pdf_name, page_name, table_count) in TEST_TABLES {
        for table_idx in 0..table_count {
            total_tested += 1;

            print!(
                "[{:2}/{:2}] Testing {}/{}/table_{}... ",
                total_tested, 30, pdf_name, page_name, table_idx
            );

            match test_single_table(&base_path, pdf_name, page_name, table_idx) {
                Ok(result) => {
                    if result.pass {
                        println!("✅ PASS (max diff: {:.6})", result.max_diff);
                        total_passed += 1;
                    } else {
                        println!("❌ FAIL");
                        println!("      Max diff: {:.10} >= 0.0200000000", result.max_diff);
                        total_failed += 1;
                    }
                }
                Err(e) => {
                    println!("⚠️  ERROR: {}", e);
                    total_failed += 1;
                }
            }
        }
    }

    println!();
    println!("{}", "=".repeat(80));
    println!("Phase 2 Multi-Page Validation Results");
    println!("{}", "=".repeat(80));
    println!("Passed: {}/{}", total_passed, total_tested);
    println!("Failed: {}/{}", total_failed, total_tested);
    println!();

    if total_failed == 0 {
        println!("✅ ALL TABLES PASSED");
        println!("   Rust TableFormer preprocessing = Python baseline on all 30 tables");
        println!("{}", "=".repeat(80));
    } else {
        println!("❌ SOME TABLES FAILED");
        println!("   {} tables have preprocessing differences", total_failed);
        println!("   Next: Debug failed tables individually");
        println!("{}", "=".repeat(80));
    }

    assert_eq!(
        total_failed, 0,
        "{} tables failed Phase 2 validation",
        total_failed
    );
}
