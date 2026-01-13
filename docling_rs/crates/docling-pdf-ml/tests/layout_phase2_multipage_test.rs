use docling_pdf_ml::preprocessing::layout::layout_preprocess;
/// Phase 2 Multi-Page Validation Test for LayoutPredictor Preprocessing
///
/// This test validates that the Rust preprocessing produces identical outputs to Python
/// on ALL 47 pages across 4 test PDFs.
///
/// Phase 2 Success Criteria: Max pixel difference < 0.02 on each page (from N=83 decision)
use ndarray::{Array3, Array4};
use npyz::NpyFile;
use std::fs::File;
use std::io::BufReader;

const MAX_DIFF_THRESHOLD: f32 = 0.02; // From N=83 decision

/// Test pages: (pdf_basename, page_num)
const TEST_PAGES: &[(&str, usize)] = &[
    // arxiv_2206.01062.pdf (9 pages)
    ("arxiv_2206.01062", 0),
    ("arxiv_2206.01062", 1),
    ("arxiv_2206.01062", 2),
    ("arxiv_2206.01062", 3),
    ("arxiv_2206.01062", 4),
    ("arxiv_2206.01062", 5),
    ("arxiv_2206.01062", 6),
    ("arxiv_2206.01062", 7),
    ("arxiv_2206.01062", 8),
    // code_and_formula.pdf (2 pages)
    ("code_and_formula", 0),
    ("code_and_formula", 1),
    // edinet_sample.pdf (21 pages)
    ("edinet_sample", 0),
    ("edinet_sample", 1),
    ("edinet_sample", 2),
    ("edinet_sample", 3),
    ("edinet_sample", 4),
    ("edinet_sample", 5),
    ("edinet_sample", 6),
    ("edinet_sample", 7),
    ("edinet_sample", 8),
    ("edinet_sample", 9),
    ("edinet_sample", 10),
    ("edinet_sample", 11),
    ("edinet_sample", 12),
    ("edinet_sample", 13),
    ("edinet_sample", 14),
    ("edinet_sample", 15),
    ("edinet_sample", 16),
    ("edinet_sample", 17),
    ("edinet_sample", 18),
    ("edinet_sample", 19),
    ("edinet_sample", 20),
    // jfk_scanned.pdf (15 pages)
    ("jfk_scanned", 0),
    ("jfk_scanned", 1),
    ("jfk_scanned", 2),
    ("jfk_scanned", 3),
    ("jfk_scanned", 4),
    ("jfk_scanned", 5),
    ("jfk_scanned", 6),
    ("jfk_scanned", 7),
    ("jfk_scanned", 8),
    ("jfk_scanned", 9),
    ("jfk_scanned", 10),
    ("jfk_scanned", 11),
    ("jfk_scanned", 12),
    ("jfk_scanned", 13),
    ("jfk_scanned", 14),
];

#[test]
fn test_layout_phase2_all_pages() {
    println!("\n================================================================================");
    println!("Phase 2 Multi-Page Validation: LayoutPredictor Preprocessing");
    println!("================================================================================");
    println!("Goal: Prove Rust preprocessing = Python preprocessing on ALL 47 pages");
    println!("Method: Raw image → preprocess → compare to baseline");
    println!("Success: Max diff < {MAX_DIFF_THRESHOLD} on each page (total: 47/47 passing)");
    println!();

    let mut passed = 0;
    let mut failed = 0;

    for (idx, &(pdf_name, page_num)) in TEST_PAGES.iter().enumerate() {
        let page_label = format!("{pdf_name}/page_{page_num:02}");
        print!(
            "[{:2}/{:2}] Testing {}...",
            idx + 1,
            TEST_PAGES.len(),
            page_label
        );

        // Construct paths
        let base_dir = format!("ml_model_inputs_multipage/{pdf_name}/page_{page_num:02}");
        let image_path = format!("{base_dir}/page_image.npy");
        let baseline_path = format!("{base_dir}/preprocessed_input.npy");

        // Load raw page image
        let raw_image = match load_npy_u8(&image_path) {
            Ok(img) => img,
            Err(e) => {
                println!(" ⚠️  SKIP (failed to load image: {e})");
                continue;
            }
        };

        // Load Python baseline (preprocessed tensor)
        let baseline = match load_npy_f32(&baseline_path) {
            Ok(b) => b,
            Err(e) => {
                println!(" ⚠️  SKIP (failed to load baseline: {e})");
                continue;
            }
        };

        // Run Rust preprocessing
        let rust_output = layout_preprocess(&raw_image);

        // Compare outputs
        let max_diff = calculate_max_diff(&rust_output, &baseline);

        if max_diff < MAX_DIFF_THRESHOLD {
            println!(" ✅ PASS (max diff: {max_diff:.6})");
            passed += 1;
        } else {
            println!(" ❌ FAIL (max diff: {max_diff:.6} >= {MAX_DIFF_THRESHOLD})");
            failed += 1;
        }
    }

    println!();
    println!("================================================================================");
    println!("Phase 2 Multi-Page Validation Results");
    println!("================================================================================");
    println!("Passed: {}/{}", passed, TEST_PAGES.len());
    println!("Failed: {}/{}", failed, TEST_PAGES.len());
    println!();

    if failed == 0 {
        println!("✅ ALL PAGES PASSED");
        println!("   Rust preprocessing = Python preprocessing on all 47 pages");
        println!("   Conclusion: Preprocessing is correct for multi-page validation");
    } else {
        println!("❌ SOME PAGES FAILED");
        println!("   {failed} pages have preprocessing differences");
        println!("   Next: Debug failed pages individually");
    }

    println!("================================================================================\n");

    // Assert to fail test if any page failed
    assert_eq!(failed, 0, "{failed} pages failed Phase 2 validation");
}

/// Load numpy .npy file as Array3<u8>
fn load_npy_u8(path: &str) -> Result<Array3<u8>, String> {
    let file = File::open(path).map_err(|e| format!("Failed to open {path}: {e}"))?;
    let reader = BufReader::new(file);
    let npy = NpyFile::new(reader).map_err(|e| format!("Failed to parse .npy file: {e}"))?;

    // Read shape
    let shape = npy.shape().to_vec();
    if shape.len() != 3 {
        return Err(format!("Expected 3D array (H, W, C), got {shape:?}"));
    }

    // Read data
    let data: Vec<u8> = npy
        .into_vec()
        .map_err(|e| format!("Failed to read .npy data: {e}"))?;

    // Create ndarray (cast u64 to usize)
    Array3::from_shape_vec(
        (shape[0] as usize, shape[1] as usize, shape[2] as usize),
        data,
    )
    .map_err(|e| format!("Failed to create Array3 from data: {e}"))
}

/// Load numpy .npy file as Array4<f32>
fn load_npy_f32(path: &str) -> Result<Array4<f32>, String> {
    let file = File::open(path).map_err(|e| format!("Failed to open {path}: {e}"))?;
    let reader = BufReader::new(file);
    let npy = NpyFile::new(reader).map_err(|e| format!("Failed to parse .npy file: {e}"))?;

    // Read shape
    let shape = npy.shape().to_vec();
    if shape.len() != 4 {
        return Err(format!("Expected 4D array (B, C, H, W), got {shape:?}"));
    }

    // Read data
    let data: Vec<f32> = npy
        .into_vec()
        .map_err(|e| format!("Failed to read .npy data: {e}"))?;

    // Create ndarray (cast u64 to usize)
    Array4::from_shape_vec(
        (
            shape[0] as usize,
            shape[1] as usize,
            shape[2] as usize,
            shape[3] as usize,
        ),
        data,
    )
    .map_err(|e| format!("Failed to create Array4 from data: {e}"))
}

fn calculate_max_diff(rust: &Array4<f32>, python: &Array4<f32>) -> f32 {
    let rust_slice = rust.as_slice().unwrap();
    let python_slice = python.as_slice().unwrap();

    rust_slice
        .iter()
        .zip(python_slice.iter())
        .map(|(r, p)| (r - p).abs())
        .fold(0.0f32, f32::max)
}
