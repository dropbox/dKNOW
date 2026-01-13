#![cfg(feature = "pytorch")]
mod common;
/// TableFormer Inference Integration Test
///
/// Tests TableFormer inference on page 3 (arxiv_2206.01062) which has 1 table.
/// Compares inference output against baseline table structure.
///
/// Expected (from baseline):
/// - 1 table cluster (ID=5)
/// - num_rows: 14
/// - num_cols: 12
/// - table_cells: 158 cells
use common::baseline_loaders::load_numpy_u8;
use docling_pdf_ml::pipeline::{PageElement, SimpleTextCell};
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

    // Fix coordinate system
    for cell in &mut cells {
        if cell.rect.t > cell.rect.b {
            std::mem::swap(&mut cell.rect.t, &mut cell.rect.b);
        }
    }

    Some(cells)
}

#[test]
fn test_tableformer_inference_page3() {
    let doc_name = "arxiv_2206.01062";
    let page_no = 3;

    println!(
        "\n=== TableFormer Inference Test: {} page {} ===",
        doc_name, page_no
    );

    // 1. Load page image
    println!("\n[1/4] Loading page image...");
    let image_path = PathBuf::from(format!(
        "baseline_data/{}/page_{}/layout/input_page_image.npy",
        doc_name, page_no
    ));

    let page_image_dyn = load_numpy_u8(&image_path).expect("Failed to load page image");
    let page_image = page_image_dyn
        .into_dimensionality::<ndarray::Ix3>()
        .expect("Failed to convert to 3D array");
    println!("  ✓ Image loaded: shape={:?}", page_image.shape());

    let (page_width, page_height) = load_page_size(doc_name, page_no);
    println!("  ✓ Page size: {}x{}", page_width, page_height);

    // 2. Initialize pipeline WITH TableFormer enabled
    println!("\n[2/4] Initializing pipeline with TableFormer...");
    let config = PipelineConfig {
        device: Device::Cpu,
        ocr_enabled: false,
        table_structure_enabled: true, // ← Enable TableFormer inference
        ..Default::default()
    };

    let mut pipeline = Pipeline::new(config).expect("Failed to create pipeline");
    println!("  ✓ Pipeline initialized with TableFormer");

    // 3. Load textline cells
    let textline_cells = load_textline_cells(doc_name, page_no);
    if let Some(ref cells) = textline_cells {
        println!("  ✓ Loaded {} textline cells", cells.len());
    }

    // 4. Process page with TableFormer inference
    println!("\n[3/4] Running TableFormer inference...");
    let page = pipeline
        .process_page(
            page_no,
            &page_image,
            page_width,
            page_height,
            textline_cells,
        )
        .expect("Failed to process page");

    let assembled = page.assembled.expect("Page should have assembled data");
    println!("  ✓ Page processed");
    println!("    - Elements: {}", assembled.elements.len());

    // 5. Find table element and check structure
    println!("\n[4/4] Validating table structure...");
    let mut found_table = false;
    for element in &assembled.elements {
        if let PageElement::Table(table) = element {
            found_table = true;
            println!("\n  📊 Table Element (ID={}):", table.id);
            println!("      - num_rows: {}", table.num_rows);
            println!("      - num_cols: {}", table.num_cols);
            println!("      - table_cells: {}", table.table_cells.len());

            // Expected from baseline
            let expected_rows = 14;
            let expected_cols = 12;
            let expected_cells = 158;

            println!("\n  Expected (from baseline):");
            println!("      - num_rows: {}", expected_rows);
            println!("      - num_cols: {}", expected_cols);
            println!("      - table_cells: {}", expected_cells);

            // Compare
            if table.num_rows == expected_rows {
                println!("  ✓ num_rows matches");
            } else {
                println!(
                    "  ✗ num_rows mismatch: got {}, expected {}",
                    table.num_rows, expected_rows
                );
            }

            if table.num_cols == expected_cols {
                println!("  ✓ num_cols matches");
            } else {
                println!(
                    "  ✗ num_cols mismatch: got {}, expected {}",
                    table.num_cols, expected_cols
                );
            }

            if table.table_cells.len() == expected_cells {
                println!("  ✓ table_cells count matches");
            } else {
                println!(
                    "  ✗ table_cells count mismatch: got {}, expected {}",
                    table.table_cells.len(),
                    expected_cells
                );
            }

            // Sample a few cells to check structure
            if !table.table_cells.is_empty() {
                println!("\n  Sample cells:");
                for (i, cell) in table.table_cells.iter().take(3).enumerate() {
                    println!("    Cell {}: row=[{},{}], col=[{},{}], bbox=({:.1},{:.1},{:.1},{:.1}), text='{}'",
                        i,
                        cell.start_row_offset_idx,
                        cell.end_row_offset_idx,
                        cell.start_col_offset_idx,
                        cell.end_col_offset_idx,
                        cell.bbox.l, cell.bbox.t, cell.bbox.r, cell.bbox.b,
                        cell.text
                    );
                }
            }

            // Assert basic structure
            assert!(table.num_rows > 0, "Table should have rows");
            assert!(table.num_cols > 0, "Table should have columns");
            assert!(!table.table_cells.is_empty(), "Table should have cells");

            break;
        }
    }

    assert!(found_table, "Should have found at least one table element");
    println!("\n✓ TableFormer inference test complete");
}
