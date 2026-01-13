#![cfg(feature = "pytorch")]
mod common;
/// Pipeline OCR Integration Test
///
/// Tests that RapidOCR is properly integrated with the pipeline:
/// 1. Load pipeline with OCR enabled
/// 2. Process a page without providing textline_cells
/// 3. Verify OCR runs automatically
/// 4. Verify results are generated
use common::baseline_loaders::load_numpy_u8;
use docling_pdf_ml::{Pipeline, PipelineConfig};
use std::path::Path;
use tch::Device;

#[test]
fn test_pipeline_with_ocr_enabled() {
    println!("\n=== Pipeline OCR Integration Test ===");

    // 1. Load page image (HWC format, u8)
    println!("\n[1/3] Loading page image...");
    let image_path = Path::new("ml_model_inputs/layout_predictor/page_0_image.npy");
    let page_image_dyn = load_numpy_u8(image_path).expect("Failed to load page image");

    // Convert ArrayD<u8> to Array3<u8>
    let shape = page_image_dyn.shape().to_vec();
    assert_eq!(shape.len(), 3, "Image must be 3D (HWC format)");
    let page_image = page_image_dyn
        .into_dimensionality::<ndarray::Ix3>()
        .expect("Failed to convert to 3D array");

    println!("  ✓ Image loaded: shape={:?}", page_image.shape());

    // Page dimensions (from baseline)
    let page_width = 612.0; // arxiv page width
    let page_height = 792.0; // arxiv page height

    // 2. Initialize pipeline with OCR enabled
    println!("\n[2/3] Initializing pipeline with OCR enabled...");
    let config = PipelineConfig {
        device: Device::Cpu,
        ocr_enabled: true, // Enable OCR
        table_structure_enabled: false,
        ..Default::default()
    };

    let mut pipeline = Pipeline::new(config).expect("Failed to create pipeline");
    println!("  ✓ Pipeline initialized with OCR");

    // 3. Process page WITHOUT providing textline_cells
    // This should trigger RapidOCR automatically
    println!("\n[3/3] Processing page (OCR should run automatically)...");
    let page = pipeline
        .process_page(
            0,
            &page_image,
            page_width,
            page_height,
            None, // No textline_cells provided - OCR should run
        )
        .expect("Failed to process page");

    // 4. Verify results
    let assembled = page.assembled.expect("Page should have assembled data");
    println!("  ✓ Page processed successfully");
    println!("    - Elements: {}", assembled.elements.len());
    println!("    - Body: {}", assembled.body.len());
    println!("    - Headers: {}", assembled.headers.len());

    // Basic sanity checks
    assert!(
        !assembled.elements.is_empty(),
        "Should have detected at least some elements"
    );

    println!("\n✓ OCR integration test PASSED");
}

#[test]
fn test_pipeline_with_ocr_disabled() {
    println!("\n=== Pipeline OCR Disabled Test ===");

    // 1. Load page image
    println!("\n[1/3] Loading page image...");
    let image_path = Path::new("ml_model_inputs/layout_predictor/page_0_image.npy");
    let page_image_dyn = load_numpy_u8(image_path).expect("Failed to load page image");
    let page_image = page_image_dyn
        .into_dimensionality::<ndarray::Ix3>()
        .expect("Failed to convert to 3D array");
    println!("  ✓ Image loaded");

    let page_width = 612.0;
    let page_height = 792.0;

    // 2. Initialize pipeline with OCR DISABLED
    println!("\n[2/3] Initializing pipeline with OCR disabled...");
    let config = PipelineConfig {
        device: Device::Cpu,
        ocr_enabled: false, // Disable OCR
        table_structure_enabled: false,
        ..Default::default()
    };

    let mut pipeline = Pipeline::new(config).expect("Failed to create pipeline");
    println!("  ✓ Pipeline initialized WITHOUT OCR");

    // 3. Process page without textline_cells
    // OCR should NOT run (because it's disabled)
    println!("\n[3/3] Processing page (OCR should NOT run)...");
    let page = pipeline
        .process_page(
            0,
            &page_image,
            page_width,
            page_height,
            None, // No textline_cells - OCR is disabled so modular pipeline will be skipped
        )
        .expect("Failed to process page");

    // 4. Verify results - should have layout detection only
    let assembled = page.assembled.expect("Page should have assembled data");
    println!("  ✓ Page processed successfully");
    println!("    - Elements: {}", assembled.elements.len());

    // Without OCR and without textline_cells, the modular pipeline won't run
    // We should still have layout detection results
    assert!(
        page.predictions.layout.is_some(),
        "Should have layout predictions even without OCR"
    );

    println!("\n✓ OCR disabled test PASSED");
}

#[test]
fn test_pipeline_with_empty_cells_triggers_ocr() {
    println!("\n=== Pipeline OCR with Empty Cells Test ===");
    println!("Testing that Some([]) (empty cells) triggers OCR, not skips it");

    // 1. Load page image
    println!("\n[1/3] Loading page image...");
    let image_path = Path::new("ml_model_inputs/layout_predictor/page_0_image.npy");
    let page_image_dyn = load_numpy_u8(image_path).expect("Failed to load page image");
    let page_image = page_image_dyn
        .into_dimensionality::<ndarray::Ix3>()
        .expect("Failed to convert to 3D array");
    println!("  ✓ Image loaded");

    let page_width = 612.0;
    let page_height = 792.0;

    // 2. Initialize pipeline with OCR ENABLED
    println!("\n[2/3] Initializing pipeline with OCR enabled...");
    let config = PipelineConfig {
        device: Device::Cpu,
        ocr_enabled: true, // Enable OCR
        table_structure_enabled: false,
        ..Default::default()
    };

    let mut pipeline = Pipeline::new(config).expect("Failed to create pipeline");
    println!("  ✓ Pipeline initialized WITH OCR");

    // 3. Process page WITH empty textline_cells
    // This is the critical test: Some([]) should trigger OCR, not skip it
    println!("\n[3/3] Processing page with empty cells (OCR should run)...");
    let empty_cells = Some(vec![]); // Empty vector - should trigger OCR
    let page = pipeline
        .process_page(
            0,
            &page_image,
            page_width,
            page_height,
            empty_cells, // Pass empty cells - OCR should still run
        )
        .expect("Failed to process page");

    // 4. Verify OCR ran by checking assembled output
    println!("  ✓ Page processed successfully");

    // Verify we have assembled results
    let assembled = page.assembled.expect("Page should have assembled data");
    println!("    - Elements: {}", assembled.elements.len());

    // The key test: with empty cells passed in, OCR should run and produce text
    // If OCR didn't run, there would be no text content (layout detection alone doesn't extract text)
    assert!(
        !assembled.elements.is_empty(),
        "Should have detected elements after OCR"
    );

    // Check that at least some elements have text content (proves OCR ran and extracted text)
    let has_text = assembled
        .elements
        .iter()
        .any(|elem| !elem.text().is_empty());

    assert!(
        has_text,
        "OCR should have extracted text content (proves OCR ran with empty cells input)"
    );

    println!("    - Text extracted: OCR confirmed to have run");
    println!("\n✓ Empty cells triggers OCR test PASSED");
    println!("  (Bug fix verified: Some([]) now correctly triggers OCR)");
}
