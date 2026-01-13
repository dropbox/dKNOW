#![cfg(feature = "opencv-preprocessing")]
use docling_pdf_ml::preprocessing::rapidocr::rapidocr_rec_preprocess;
/// RapidOCR Recognition Preprocessing - Phase 2 Validation
///
/// Phase 2: Validate preprocessing pipeline matches Python baseline.
///
/// Success criteria (from N=83 threshold decision):
/// - Max pixel difference < 0.02
///
/// Related files:
/// - Python baseline: ml_model_inputs/rapid_ocr_isolated/rec_preprocessed_input.npy
/// - Raw cropped boxes: ml_model_inputs/rapid_ocr/cropped_text_box_{i}.npy
/// - Rust implementation: src/preprocessing/rapidocr.rs::rapidocr_rec_preprocess()
///
/// Context:
/// - Detection preprocessing validated at N=83 (max diff 0.0157)
/// - Classification preprocessing validated at N=89 (max diff < 0.02 with OpenCV)
/// - Recognition uses same algorithm as classification but max_width=320 instead of 192
use ndarray::{Array3, Array4, ArrayView3};
use npyz::NpyFile;
use std::fs::File;
use std::io::BufReader;

const MAX_DIFF_THRESHOLD: f32 = 0.02; // From N=83 decision

/// Load numpy .npy file as Array3<u8>
fn load_npy_u8(path: &str) -> Array3<u8> {
    let file = File::open(path).unwrap_or_else(|_| panic!("Failed to open {}", path));
    let reader = BufReader::new(file);
    let npy = NpyFile::new(reader).expect("Failed to parse .npy file");

    // Read shape
    let shape = npy.shape().to_vec();
    assert_eq!(shape.len(), 3, "Expected 3D array (H, W, C)");

    // Read data
    let data: Vec<u8> = npy.into_vec().expect("Failed to read .npy data");

    // Create ndarray (cast u64 to usize)
    Array3::from_shape_vec(
        (shape[0] as usize, shape[1] as usize, shape[2] as usize),
        data,
    )
    .expect("Failed to create Array3 from data")
}

/// Load numpy .npy file as Array4<f32>
fn load_npy_f32(path: &str) -> Array4<f32> {
    let file = File::open(path).unwrap_or_else(|_| panic!("Failed to open {}", path));
    let reader = BufReader::new(file);
    let npy = NpyFile::new(reader).expect("Failed to parse .npy file");

    // Read shape
    let shape = npy.shape().to_vec();
    assert_eq!(shape.len(), 4, "Expected 4D array (B, C, H, W)");

    // Read data
    let data: Vec<f32> = npy.into_vec().expect("Failed to read .npy data");

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
    .expect("Failed to create Array4 from data")
}

#[test]
fn test_rapidocr_rec_preprocessing_phase2() {
    println!("\n=== RapidOCR Recognition Preprocessing Phase 2 ===");
    println!("Testing: Rust preprocessing vs Python baseline");
    println!("Threshold: max diff < {}\n", MAX_DIFF_THRESHOLD);

    // Load Python baseline (3 preprocessed boxes)
    let baseline_path = "ml_model_inputs/rapid_ocr_isolated/rec_preprocessed_input.npy";
    println!("Loading Python baseline: {}", baseline_path);
    let baseline = load_npy_f32(baseline_path);
    println!("  Shape: {:?}, dtype: f32", baseline.dim());
    assert_eq!(
        baseline.dim(),
        (3, 3, 48, 320),
        "Baseline should be (3, 3, 48, 320)"
    );

    // Process each of the 3 cropped text boxes
    let mut max_diffs = Vec::new();

    for i in 0..3 {
        println!("\n--- Box {} ---", i);

        // Load raw cropped box
        let crop_path = format!("ml_model_inputs/rapid_ocr/cropped_text_box_{}.npy", i);
        println!("Loading raw crop: {}", crop_path);
        let crop = load_npy_u8(&crop_path);
        println!("  Shape: {:?}, dtype: u8", crop.dim());

        // Run Rust preprocessing
        println!("Running Rust preprocessing...");
        let rust_output = rapidocr_rec_preprocess(&crop);
        println!("  Output shape: {:?}", rust_output.dim());
        assert_eq!(
            rust_output.dim(),
            (3, 48, 320),
            "Output should be (3, 48, 320)"
        );

        // Get Python baseline for this box
        let python_output = baseline.index_axis(ndarray::Axis(0), i);

        // Debug: print some values
        println!(
            "  Rust first 5 values, C0, row 0: {} {} {} {} {}",
            rust_output[[0, 0, 0]],
            rust_output[[0, 0, 1]],
            rust_output[[0, 0, 2]],
            rust_output[[0, 0, 3]],
            rust_output[[0, 0, 4]]
        );
        println!(
            "  Python first 5 values, C0, row 0: {} {} {} {} {}",
            python_output[[0, 0, 0]],
            python_output[[0, 0, 1]],
            python_output[[0, 0, 2]],
            python_output[[0, 0, 3]],
            python_output[[0, 0, 4]]
        );

        // Compare outputs
        let (max_diff, max_loc) =
            calculate_max_diff_with_location(&rust_output.view(), &python_output);
        println!(
            "  Max pixel difference: {:.6} at location {:?}",
            max_diff, max_loc
        );
        println!(
            "    Rust value at max: {:.6}",
            rust_output[[max_loc.0, max_loc.1, max_loc.2]]
        );
        println!(
            "    Python value at max: {:.6}",
            python_output[[max_loc.0, max_loc.1, max_loc.2]]
        );

        // Debug: Show surrounding pixels at max location
        if max_diff >= MAX_DIFF_THRESHOLD {
            let (c, h, w) = max_loc;
            println!("  Surrounding pixels at ({}, {}, {}±2):", c, h, w);
            for x in (w.saturating_sub(2))..=(w + 2).min(319) {
                println!(
                    "    Rust[{}, {}, {}] = {:.6}, Python = {:.6}",
                    c,
                    h,
                    x,
                    rust_output[[c, h, x]],
                    python_output[[c, h, x]]
                );
            }
        }

        max_diffs.push(max_diff);

        // Check threshold
        if max_diff < MAX_DIFF_THRESHOLD {
            println!("  ✓ PASS: {} < {}", max_diff, MAX_DIFF_THRESHOLD);
        } else {
            println!("  ✗ FAIL: {} >= {}", max_diff, MAX_DIFF_THRESHOLD);
            panic!(
                "Box {} preprocessing failed: max diff {:.6} >= threshold {}",
                i, max_diff, MAX_DIFF_THRESHOLD
            );
        }
    }

    // Summary
    println!("\n=== Summary ===");
    for (i, diff) in max_diffs.iter().enumerate() {
        println!("Box {}: max diff {:.6}", i, diff);
    }
    let overall_max = max_diffs.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    println!("\nOverall max diff: {:.6}", overall_max);
    println!("Threshold: {}", MAX_DIFF_THRESHOLD);

    if overall_max < MAX_DIFF_THRESHOLD {
        println!("\n✓ Phase 2 Recognition Preprocessing: PASS");
    } else {
        println!("\n✗ Phase 2 Recognition Preprocessing: FAIL");
        panic!(
            "Phase 2 failed: max diff {:.6} >= {}",
            overall_max, MAX_DIFF_THRESHOLD
        );
    }
}

/// Calculate maximum absolute difference between two tensors
fn calculate_max_diff_with_location(
    a: &ArrayView3<f32>,
    b: &ArrayView3<f32>,
) -> (f32, (usize, usize, usize)) {
    assert_eq!(a.dim(), b.dim(), "Tensors must have same shape");

    let mut max_diff = 0.0f32;
    let mut max_loc = (0, 0, 0);

    let (c, h, w) = a.dim();
    for ci in 0..c {
        for hi in 0..h {
            for wi in 0..w {
                let val_a = a[[ci, hi, wi]];
                let val_b = b[[ci, hi, wi]];
                let diff = (val_a - val_b).abs();
                if diff > max_diff {
                    max_diff = diff;
                    max_loc = (ci, hi, wi);
                }
            }
        }
    }
    (max_diff, max_loc)
}

/// Calculate maximum absolute difference between two tensors
fn calculate_max_diff(a: &ArrayView3<f32>, b: &ArrayView3<f32>) -> f32 {
    calculate_max_diff_with_location(a, b).0
}
