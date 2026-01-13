#![cfg(feature = "opencv-preprocessing")]
/// Stage 0.1.1: OCR Detection Postprocessing - All 15 JFK Pages
///
/// **OCR Model:** PP-OCRv4 Detection Postprocessing (DbNet)
///
/// **Purpose:** Validates that Rust detection postprocessing produces
/// EXACTLY the same text box coordinates as Python for all 15 JFK pages.
///
/// **Methodology:**
/// - For each page:
///   - Load Python baseline boxes (642 total across 15 pages)
///   - Load probability map from Stage 0.1
///   - Run Rust postprocessing
///   - Compare: Every box must match exactly (integer coordinates)
///
/// **Success Criteria:**
/// - All 642 boxes across all 15 pages must match exactly
/// - Box coordinates are integers, so exact match expected
use anyhow::{Context, Result};
use image::GenericImageView;
use ndarray::Array4;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct PythonBoxes {
    num_boxes: usize,
    boxes: Vec<Vec<[i32; 2]>>, // Each box: 4 corners, each corner: [x, y]
    scores: Vec<f64>,
    original_shape: [usize; 2],
}

#[test]
fn test_stage01_1_ocr_det_postprocess_jfk_15_pages() -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("Stage 0.1.1: OCR Detection Postprocessing - All 15 JFK Pages");
    println!("{}", "=".repeat(80));
    println!("Goal: Prove Rust boxes = Python boxes (exact match for all 642 boxes)");
    println!();

    // Setup paths
    let home = std::env::var("HOME").context("HOME not set")?;
    let base_path = PathBuf::from(&home).join("docling_debug_pdf_parsing");

    // Test all 15 pages
    let mut total_boxes = 0;
    let mut total_matches = 0;
    let mut total_mismatches = 0;

    for page_num in 0..15 {
        println!("[Page {}]", page_num);

        // Load Python baseline boxes
        let boxes_path = base_path.join(format!(
            "baseline_data/jfk_scanned/page_{}/ocr/detection_postprocess/python_boxes.json",
            page_num
        ));
        let boxes_json = std::fs::read_to_string(&boxes_path)
            .with_context(|| format!("Failed to load boxes: {}", boxes_path.display()))?;
        let python_boxes: PythonBoxes = serde_json::from_str(&boxes_json)?;

        println!("  Python boxes: {}", python_boxes.num_boxes);

        // Load probability map from Stage 0.1
        let prob_map_path = base_path.join(format!(
            "baseline_data/jfk_scanned/page_{}/ocr/detection_phase1/python_probability_map.npy",
            page_num
        ));
        let prob_map: Array4<f32> = ndarray_npy::read_npy(&prob_map_path)?;
        println!("  Probability map: {:?}", prob_map.shape());

        // Load original image to get dimensions
        let image_path = base_path.join(format!(
            "baseline_data/jfk_scanned/page_{}/layout/input_page_image.npy",
            page_num
        ));
        let image_array = load_npy_image(&image_path)?;
        let orig_image = array_to_dynamic_image(&image_array)?;

        // Run Rust postprocessing
        use docling_pdf_ml::ocr::detection::DbNet;
        use docling_pdf_ml::ocr::types::DetectionParams;

        // Create dummy DbNet instance (we don't need the model, just postprocess)
        let params = DetectionParams::default();

        // Extract detection map
        let shape = prob_map.shape();
        let out_height = shape[2] as i32;
        let out_width = shape[3] as i32;
        let detection_map: Vec<f32> = prob_map.iter().copied().collect();

        // Calculate scale factors (same as in detect())
        let (orig_width, orig_height) = orig_image.dimensions();
        let preprocessed_width = out_width as f32;
        let preprocessed_height = out_height as f32;
        let scale_x = orig_width as f32 / preprocessed_width;
        let scale_y = orig_height as f32 / preprocessed_height;

        // Call postprocess
        let rust_boxes = DbNet::postprocess(
            &detection_map,
            out_width,
            out_height,
            &params,
            scale_x,
            scale_y,
            orig_width,
            orig_height,
        )?;

        println!("  Rust boxes: {}", rust_boxes.len());

        // DEBUG: Print first 3 Rust boxes (before reversal) for Page 0
        if page_num == 0 {
            println!("  DEBUG - First 3 Rust boxes (before reversal):");
            for (i, b) in rust_boxes.iter().take(3).enumerate() {
                let corners: Vec<[i32; 2]> = b
                    .corners
                    .iter()
                    .map(|&(x, y)| [x.round() as i32, y.round() as i32])
                    .collect();
                println!("    Rust box {}: {:?}", i, corners);
            }
        }

        // IMPORTANT: Rust boxes are in reverse order compared to Python
        // Reverse Rust boxes to match Python ordering
        let rust_boxes_reversed: Vec<_> = rust_boxes.iter().rev().collect();

        // Compare boxes
        if rust_boxes.len() != python_boxes.num_boxes {
            println!(
                "  ✗ Box count mismatch: Rust={}, Python={}",
                rust_boxes.len(),
                python_boxes.num_boxes
            );
            total_mismatches += 1;
            continue;
        }

        // Compare each box
        let mut page_mismatches = 0;
        for (i, (rust_box, python_box)) in rust_boxes_reversed
            .iter()
            .zip(python_boxes.boxes.iter())
            .enumerate()
        {
            // Rust box has corners as Vec<(f32, f32)>, Python has Vec<[i32; 2]>
            let rust_points: Vec<[i32; 2]> = rust_box
                .corners
                .iter()
                .map(|&(x, y)| [x.round() as i32, y.round() as i32])
                .collect();

            let python_points: Vec<[i32; 2]> = python_box.clone();

            if rust_points != python_points {
                if page_mismatches == 0 {
                    println!("  ✗ Box mismatches:");
                }
                println!(
                    "    Box {}: Rust {:?} != Python {:?}",
                    i, rust_points, python_points
                );
                page_mismatches += 1;
            }
        }

        if page_mismatches == 0 {
            println!("  ✓ PASS - All {} boxes match exactly", rust_boxes.len());
            total_matches += rust_boxes.len();
        } else {
            println!("  ✗ FAIL - {} boxes mismatch", page_mismatches);
            total_mismatches += page_mismatches;
        }

        total_boxes += rust_boxes.len();
        println!();
    }

    // Summary
    println!("{}", "=".repeat(80));
    println!("SUMMARY:");
    println!("  Total boxes: {}", total_boxes);
    println!("  Matches: {}", total_matches);
    println!("  Mismatches: {}", total_mismatches);
    println!("{}", "=".repeat(80));

    if total_mismatches > 0 {
        anyhow::bail!(
            "\n❌ {} boxes failed exact match\nExpected all {} boxes to match exactly",
            total_mismatches,
            total_boxes
        );
    }

    println!("\n✅ ALL 642 BOXES ACROSS 15 PAGES MATCHED EXACTLY");
    println!("Stage 0.1.1 (Detection Postprocessing) is VALIDATED\n");
    Ok(())
}

// Helper functions

fn load_npy_image(path: &PathBuf) -> Result<ndarray::Array3<u8>> {
    use ndarray_npy::NpzReader;
    use std::fs::File;

    if path.extension().and_then(|s| s.to_str()) == Some("npz") {
        let file = File::open(path).context("Failed to open .npz file")?;
        let mut npz = NpzReader::new(file).context("Failed to create NpzReader")?;
        npz.by_index(0).context("Failed to read array from .npz")
    } else {
        ndarray_npy::read_npy(path).context("Failed to read .npy file")
    }
}

fn array_to_dynamic_image(array: &ndarray::Array3<u8>) -> Result<image::DynamicImage> {
    let (height, width, channels) = array.dim();
    anyhow::ensure!(channels == 3, "Expected 3 channels (RGB), got {}", channels);

    let mut img_buf = image::RgbImage::new(width as u32, height as u32);
    for y in 0..height {
        for x in 0..width {
            let pixel = image::Rgb([array[[y, x, 0]], array[[y, x, 1]], array[[y, x, 2]]]);
            img_buf.put_pixel(x as u32, y as u32, pixel);
        }
    }

    Ok(image::DynamicImage::ImageRgb8(img_buf))
}
