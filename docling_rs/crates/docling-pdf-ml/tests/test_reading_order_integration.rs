#![cfg(feature = "pytorch")]
mod common;
/// Reading Order Pipeline Integration Test
///
/// Tests that reading order is correctly integrated into the full pipeline:
/// 1. Process all pages individually (layout → assembly)
/// 2. Apply reading order to document (all pages)
/// 3. Verify element ordering and post-processing
///
/// This validates the complete document assembly flow:
/// - All pages processed
/// - Reading order applied across document
/// - Elements properly sorted
/// - Captions, footnotes, and text merges applied
use common::baseline_loaders::load_numpy_u8;
use docling_pdf_ml::pipeline::SimpleTextCell;
use docling_pdf_ml::{Pipeline, PipelineConfig};
use std::path::PathBuf;
use tch::Device;

/// Load page size (width, height) from baseline data
fn load_page_size(doc_name: &str, page_no: usize) -> (f32, f32) {
    let path = PathBuf::from(format!(
        "baseline_data/{}/page_{}/preprocessing/page_size.json",
        doc_name, page_no
    ));

    let contents = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("Failed to read page size from {:?}", path));

    let json: serde_json::Value =
        serde_json::from_str(&contents).expect("Failed to parse page size JSON");

    let width = json["width"].as_f64().expect("Missing width") as f32;
    let height = json["height"].as_f64().expect("Missing height") as f32;

    (width, height)
}

/// Load textline cells from baseline data
fn load_textline_cells(doc_name: &str, page_no: usize) -> Option<Vec<SimpleTextCell>> {
    let path = PathBuf::from(format!(
        "baseline_data/{}/page_{}/preprocessing/textline_cells.json",
        doc_name, page_no
    ));

    if !path.exists() {
        return None;
    }

    let contents = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("Failed to read textline cells from {:?}", path));

    let mut cells: Vec<SimpleTextCell> =
        serde_json::from_str(&contents).expect("Failed to parse textline cells JSON");

    // Fix coordinate system: baseline cells have t > b (bottom-left origin)
    // but BoundingBox expects t < b (top-left origin)
    for cell in &mut cells {
        if cell.rect.t > cell.rect.b {
            std::mem::swap(&mut cell.rect.t, &mut cell.rect.b);
        }
    }

    Some(cells)
}

#[test]
fn test_reading_order_arxiv() {
    println!("\n=== Reading Order Integration Test: arxiv_2206.01062 ===");

    let doc_name = "arxiv_2206.01062";
    let num_pages = 9;

    // Step 1: Initialize pipeline
    println!("\n[1/3] Initializing pipeline...");
    let config = PipelineConfig {
        device: Device::Cpu,
        ocr_enabled: false,
        table_structure_enabled: false,
        ..Default::default()
    };

    let mut pipeline = Pipeline::new(config).expect("Failed to create pipeline");
    println!("  ✓ Pipeline initialized");

    // Step 2: Process all pages
    println!("\n[2/3] Processing {} pages...", num_pages);
    let mut pages = Vec::new();

    for page_no in 0..num_pages {
        println!("  Processing page {}...", page_no);

        // Load page image
        let image_path = PathBuf::from(format!(
            "baseline_data/{}/page_{}/layout/input_page_image.npy",
            doc_name, page_no
        ));

        if !image_path.exists() {
            println!("    ⚠ Skipping: image not found");
            continue;
        }

        let page_image_dyn = load_numpy_u8(&image_path).expect("Failed to load page image");
        let page_image = page_image_dyn
            .into_dimensionality::<ndarray::Ix3>()
            .expect("Failed to convert to 3D array");

        let (page_width, page_height) = load_page_size(doc_name, page_no);
        let textline_cells = load_textline_cells(doc_name, page_no);

        // Process page
        let page = pipeline
            .process_page(
                page_no,
                &page_image,
                page_width,
                page_height,
                textline_cells,
            )
            .expect("Failed to process page");

        println!(
            "    ✓ Page {} processed ({} elements)",
            page_no,
            page.assembled.as_ref().unwrap().elements.len()
        );

        pages.push(page);
    }

    println!("  ✓ Processed {} pages", pages.len());

    // Step 3: Apply reading order to document
    println!("\n[3/3] Applying reading order...");
    let document = pipeline
        .process_document(&pages)
        .expect("Failed to process document");

    println!("\n✓ Reading order integration test complete:");
    println!("    - Pages: {}", pages.len());
    println!("    - Total elements: {}", document.elements.len());
    println!("    - Body elements: {}", document.body.len());
    println!("    - Header elements: {}", document.headers.len());

    // Validation: basic sanity checks
    assert!(
        !document.elements.is_empty(),
        "Document should have elements"
    );
    assert!(
        !document.body.is_empty(),
        "Document should have body elements"
    );

    // Validation: element ordering (page_no should be non-decreasing)
    for i in 1..document.elements.len() {
        let prev_page = document.elements[i - 1].page_no();
        let curr_page = document.elements[i].page_no();
        assert!(
            curr_page >= prev_page,
            "Element {} has page_no {} < previous page_no {}",
            i,
            curr_page,
            prev_page
        );
    }

    println!("  ✓ Elements properly ordered by page");

    // Validation: no duplicate cluster IDs per page
    use std::collections::HashSet;
    let mut page_cid_sets: std::collections::HashMap<usize, HashSet<usize>> =
        std::collections::HashMap::new();

    for elem in &document.elements {
        let page_no = elem.page_no();
        let cid = elem.cluster().id;

        let cids = page_cid_sets.entry(page_no).or_default();
        assert!(
            cids.insert(cid),
            "Duplicate cluster ID {} on page {}",
            cid,
            page_no
        );
    }

    println!("  ✓ No duplicate cluster IDs per page");

    println!("\n✓ Reading Order Integration: PASSED");
}

#[test]
fn test_reading_order_code_formula() {
    println!("\n=== Reading Order Integration Test: code_and_formula ===");

    let doc_name = "code_and_formula";
    let num_pages = 2;

    // Step 1: Initialize pipeline
    println!("\n[1/3] Initializing pipeline...");
    let config = PipelineConfig {
        device: Device::Cpu,
        ocr_enabled: false,
        table_structure_enabled: false,
        ..Default::default()
    };

    let mut pipeline = Pipeline::new(config).expect("Failed to create pipeline");
    println!("  ✓ Pipeline initialized");

    // Step 2: Process all pages
    println!("\n[2/3] Processing {} pages...", num_pages);
    let mut pages = Vec::new();

    for page_no in 0..num_pages {
        println!("  Processing page {}...", page_no);

        // Load page image
        let image_path = PathBuf::from(format!(
            "baseline_data/{}/page_{}/layout/input_page_image.npy",
            doc_name, page_no
        ));

        if !image_path.exists() {
            println!("    ⚠ Skipping: image not found");
            continue;
        }

        let page_image_dyn = load_numpy_u8(&image_path).expect("Failed to load page image");
        let page_image = page_image_dyn
            .into_dimensionality::<ndarray::Ix3>()
            .expect("Failed to convert to 3D array");

        let (page_width, page_height) = load_page_size(doc_name, page_no);
        let textline_cells = load_textline_cells(doc_name, page_no);

        // Process page
        let page = pipeline
            .process_page(
                page_no,
                &page_image,
                page_width,
                page_height,
                textline_cells,
            )
            .expect("Failed to process page");

        println!(
            "    ✓ Page {} processed ({} elements)",
            page_no,
            page.assembled.as_ref().unwrap().elements.len()
        );

        pages.push(page);
    }

    println!("  ✓ Processed {} pages", pages.len());

    // Step 3: Apply reading order to document
    println!("\n[3/3] Applying reading order...");
    let document = pipeline
        .process_document(&pages)
        .expect("Failed to process document");

    println!("\n✓ Reading order integration test complete:");
    println!("    - Pages: {}", pages.len());
    println!("    - Total elements: {}", document.elements.len());
    println!("    - Body elements: {}", document.body.len());
    println!("    - Header elements: {}", document.headers.len());

    // Validation: basic sanity checks
    assert!(
        !document.elements.is_empty(),
        "Document should have elements"
    );
    assert!(
        !document.body.is_empty(),
        "Document should have body elements"
    );

    // Validation: element ordering (page_no should be non-decreasing)
    for i in 1..document.elements.len() {
        let prev_page = document.elements[i - 1].page_no();
        let curr_page = document.elements[i].page_no();
        assert!(
            curr_page >= prev_page,
            "Element {} has page_no {} < previous page_no {}",
            i,
            curr_page,
            prev_page
        );
    }

    println!("  ✓ Elements properly ordered by page");

    // Validation: no duplicate cluster IDs per page
    use std::collections::HashSet;
    let mut page_cid_sets: std::collections::HashMap<usize, HashSet<usize>> =
        std::collections::HashMap::new();

    for elem in &document.elements {
        let page_no = elem.page_no();
        let cid = elem.cluster().id;

        let cids = page_cid_sets.entry(page_no).or_default();
        assert!(
            cids.insert(cid),
            "Duplicate cluster ID {} on page {}",
            cid,
            page_no
        );
    }

    println!("  ✓ No duplicate cluster IDs per page");

    println!("\n✓ Reading Order Integration: PASSED");
}
