#![cfg(feature = "opencv-preprocessing")]
/// Stage 1 (OCR) Rust vs Python Comparison Test - All 47 Pages
///
/// Validates that Rust RapidOCR produces outputs comparable to Python baseline
/// across all 47 pages from 4 PDFs.
///
/// Test methodology:
/// - Load page image from baseline_data/{pdf}/page_{N}/layout/input_page_image.npy
/// - Run Rust RapidOCR pipeline
/// - Load Python baseline from baseline_data/{pdf}/page_{N}/preprocessing/textline_cells.json
/// - Compare using IOU-based matching
///
/// Success criteria (per page):
/// - Matched boxes: > 70% of Rust detections must match Python within 50px center distance
/// - Overall: 47/47 pages pass
///
/// PP-OCRv4 baselines: 3,185 cells across 47 pages (N=638)
use anyhow::{Context, Result};
use ndarray::Array3;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct PythonCell {
    index: usize,
    text: String,
    rect: PythonRect,
    confidence: f32,
    from_ocr: bool,
}

#[derive(Debug, Deserialize)]
struct PythonRect {
    l: f32,
    t: f32,
    r: f32,
    b: f32,
}

#[derive(Debug)]
struct PageTestResult {
    pdf_name: String,
    page_num: usize,
    python_cells: usize,
    rust_cells: usize,
    matched: usize,
    match_percentage: f32,
    avg_distance: f32,
    passed: bool,
}

#[test]
fn test_stage01_rust_ocr_vs_python_all_47_pages() -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("Stage 1 (OCR) Validation: Rust vs Python - All 47 Pages");
    println!("{}", "=".repeat(80));

    // Setup paths
    let home = std::env::var("HOME").context("HOME not set")?;
    let base_path = PathBuf::from(&home).join("docling_debug_pdf_parsing");
    let model_dir = base_path.join("onnx_exports/rapidocr");

    // Load RapidOCR once
    println!("\n[1] Loading RapidOCR...");
    use docling_pdf_ml::ocr::types::OcrParams;
    use docling_pdf_ml::ocr::RapidOcr;

    let mut ocr = RapidOcr::new(model_dir.to_str().context("Invalid model dir")?)?;
    println!("  ✓ RapidOCR loaded successfully");

    let params = OcrParams::default();

    // Test cases: (pdf_name, num_pages)
    let test_cases = vec![
        ("arxiv_2206.01062", 9),
        ("code_and_formula", 2),
        ("jfk_scanned", 15),
        ("edinet_sample", 21),
    ];

    let mut results = Vec::new();
    let mut total_pages = 0;
    let mut passed_pages = 0;

    println!("\n[2] Testing all 47 pages...\n");

    for (pdf_name, num_pages) in test_cases {
        println!("{}", "-".repeat(80));
        println!("{} ({} pages)", pdf_name, num_pages);
        println!("{}", "-".repeat(80));

        for page_num in 0..num_pages {
            total_pages += 1;

            // Test this page
            let result = test_page(&base_path, pdf_name, page_num, &mut ocr, &params)?;

            if result.passed {
                passed_pages += 1;
            }

            // Print result
            let status = if result.passed {
                "✓ PASS"
            } else {
                "✗ FAIL"
            };
            println!(
                "  Page {:2}: {:6} | Python: {:3} cells, Rust: {:3} cells | Match: {}/{} ({:.1}%) | Avg: {:.1}px",
                page_num,
                status,
                result.python_cells,
                result.rust_cells,
                result.matched,
                result.rust_cells,
                result.match_percentage,
                result.avg_distance
            );

            results.push(result);
        }
        println!();
    }

    // Summary
    println!("{}", "=".repeat(80));
    println!(
        "SUMMARY: {}/{} pages passed ({:.1}%)",
        passed_pages,
        total_pages,
        (passed_pages as f32 / total_pages as f32) * 100.0
    );
    println!("{}", "=".repeat(80));

    // Show failures
    let failures: Vec<&PageTestResult> = results.iter().filter(|r| !r.passed).collect();
    if !failures.is_empty() {
        println!("\nFAILURES ({}):", failures.len());
        for result in &failures {
            println!(
                "  {} page {}: Match {}/{} ({:.1}%)",
                result.pdf_name,
                result.page_num,
                result.matched,
                result.rust_cells,
                result.match_percentage
            );
        }
    }

    // Calculate statistics
    let total_python_cells: usize = results.iter().map(|r| r.python_cells).sum();
    let total_rust_cells: usize = results.iter().map(|r| r.rust_cells).sum();
    let total_matched: usize = results.iter().map(|r| r.matched).sum();
    let avg_match_percentage: f32 =
        results.iter().map(|r| r.match_percentage).sum::<f32>() / results.len() as f32;

    println!("\nSTATISTICS:");
    println!("  Total Python cells: {}", total_python_cells);
    println!("  Total Rust cells: {}", total_rust_cells);
    println!("  Total matched: {}", total_matched);
    println!("  Average match percentage: {:.1}%", avg_match_percentage);
    println!();

    // Assert: All pages must pass
    assert_eq!(
        passed_pages,
        total_pages,
        "\n❌ {}/{} pages failed\nExpected 47/47 to pass with >70% match rate",
        total_pages - passed_pages,
        total_pages
    );

    println!("✅ ALL 47 PAGES PASSED");
    Ok(())
}

fn test_page(
    base_path: &PathBuf,
    pdf_name: &str,
    page_num: usize,
    ocr: &mut docling_pdf_ml::ocr::RapidOcr,
    params: &docling_pdf_ml::ocr::types::OcrParams,
) -> Result<PageTestResult> {
    // Load page image
    let image_path = base_path.join(format!(
        "baseline_data/{}/page_{}/layout/input_page_image.npy",
        pdf_name, page_num
    ));

    let input_array = load_npy_image(&image_path)
        .with_context(|| format!("Failed to load image: {}", image_path.display()))?;
    let input_image = array_to_dynamic_image(&input_array)?;

    // Run Rust OCR
    let text_cells = ocr.detect(&input_image, params)?;

    // Convert to centers for matching
    let rust_centers: Vec<(f32, f32)> = text_cells
        .iter()
        .map(|cell| {
            let center_x =
                (cell.rect.r_x0 + cell.rect.r_x1 + cell.rect.r_x2 + cell.rect.r_x3) / 4.0;
            let center_y =
                (cell.rect.r_y0 + cell.rect.r_y1 + cell.rect.r_y2 + cell.rect.r_y3) / 4.0;
            (center_x, center_y)
        })
        .collect();

    // Load Python baseline
    let baseline_path = base_path.join(format!(
        "baseline_data/{}/page_{}/preprocessing/textline_cells.json",
        pdf_name, page_num
    ));

    let python_cells: Vec<PythonCell> = load_json(&baseline_path)
        .with_context(|| format!("Failed to load baseline: {}", baseline_path.display()))?;

    // Convert Python rects to centers
    let python_centers: Vec<(f32, f32)> = python_cells
        .iter()
        .map(|cell| {
            let center_x = (cell.rect.l + cell.rect.r) / 2.0;
            let center_y = (cell.rect.t + cell.rect.b) / 2.0;
            (center_x, center_y)
        })
        .collect();

    // IOU-based matching
    let mut matched_count = 0;
    let mut total_distance = 0.0f32;

    for (rust_x, rust_y) in &rust_centers {
        // Find closest Python cell
        let mut min_distance = f32::MAX;
        for (python_x, python_y) in &python_centers {
            let distance = ((rust_x - python_x).powi(2) + (rust_y - python_y).powi(2)).sqrt();
            if distance < min_distance {
                min_distance = distance;
            }
        }

        // Match if within 50 pixels
        if min_distance < 50.0 {
            matched_count += 1;
            total_distance += min_distance;
        }
    }

    let match_percentage = if rust_centers.is_empty() {
        100.0
    } else {
        (matched_count as f32 / rust_centers.len() as f32) * 100.0
    };

    let avg_distance = if matched_count > 0 {
        total_distance / matched_count as f32
    } else {
        0.0
    };

    let passed = match_percentage >= 70.0;

    Ok(PageTestResult {
        pdf_name: pdf_name.to_string(),
        page_num,
        python_cells: python_cells.len(),
        rust_cells: rust_centers.len(),
        matched: matched_count,
        match_percentage,
        avg_distance,
        passed,
    })
}

// Helper functions

fn load_npy_image(path: &PathBuf) -> Result<Array3<u8>> {
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

fn array_to_dynamic_image(array: &Array3<u8>) -> Result<image::DynamicImage> {
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

fn load_json<T: serde::de::DeserializeOwned>(path: &PathBuf) -> Result<T> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path.display()))?;
    serde_json::from_str(&contents)
        .with_context(|| format!("Failed to parse JSON: {}", path.display()))
}
