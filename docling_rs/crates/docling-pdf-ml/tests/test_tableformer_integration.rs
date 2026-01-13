#![cfg(feature = "pytorch")]
mod common;
/// Test TableFormer inference integration end-to-end
///
/// This test verifies that TableFormer inference works correctly
/// when integrated into the pipeline (not loading from baseline).
use common::baseline_loaders::load_numpy_u8;
use docling_pdf_ml::pipeline::PageElement;
use docling_pdf_ml::{Pipeline, PipelineConfig};
use rstest::rstest;
use std::path::PathBuf;
use tch::Device;

// Test data: (page_no, expected_num_tables, expected_rows, expected_cols)
// From baseline data analysis:
//   - Page 3: 1 table (14 rows, 12 cols)
//   - Page 5: 1 table
//   - Page 6: 2 tables
//   - Page 7: 1 table
#[rstest]
#[case(3, 1, 14, 12)]
#[case(5, 1, 0, 0)] // Don't verify structure (baseline may vary)
#[case(6, 2, 0, 0)] // Don't verify structure (baseline may vary)
#[case(7, 1, 0, 0)] // Don't verify structure (baseline may vary)
fn test_tableformer_inference_arxiv(
    #[case] page_no: usize,
    #[case] expected_num_tables: usize,
    #[case] expected_rows: usize,
    #[case] expected_cols: usize,
) {
    println!(
        "\n=== TableFormer Inference Test: arxiv page {} ===",
        page_no
    );

    let doc_name = "arxiv_2206.01062";

    // Load page image
    let image_path = PathBuf::from(format!(
        "baseline_data/{}/page_{}/layout/input_page_image.npy",
        doc_name, page_no
    ));
    let page_image_dyn = load_numpy_u8(&image_path).expect("Failed to load image");
    let page_image = page_image_dyn
        .into_dimensionality::<ndarray::Ix3>()
        .expect("Failed to convert to 3D array");

    // Load page dimensions
    let size_path = PathBuf::from(format!(
        "baseline_data/{}/page_{}/preprocessing/page_size.json",
        doc_name, page_no
    ));
    let size_json = std::fs::read_to_string(&size_path).expect("Failed to read page size");
    let json: serde_json::Value = serde_json::from_str(&size_json).expect("Failed to parse JSON");
    let page_width = json["width"].as_f64().unwrap() as f32;
    let page_height = json["height"].as_f64().unwrap() as f32;

    // Create pipeline with TableFormer ENABLED
    let config = PipelineConfig {
        device: Device::Cpu,
        ocr_enabled: false,
        table_structure_enabled: true, // ENABLE TableFormer inference
        ..Default::default()
    };

    let mut pipeline = Pipeline::new(config).expect("Failed to create pipeline");

    // Process page with TableFormer inference
    println!("Processing page with TableFormer inference...");
    let page = pipeline
        .process_page(
            page_no,
            &page_image,
            page_width,
            page_height,
            None, // No textline cells
        )
        .expect("Failed to process page");

    let assembled = page.assembled.expect("Page should have assembled data");
    println!("Elements: {}", assembled.elements.len());

    // Find table elements
    let mut table_count = 0;
    for element in &assembled.elements {
        if let PageElement::Table(table) = element {
            table_count += 1;
            println!("\n📊 Table Element {} (ID={}):", table_count, table.id);
            println!("    num_rows: {}", table.num_rows);
            println!("    num_cols: {}", table.num_cols);
            println!("    table_cells: {}", table.table_cells.len());

            // Verify table structure is populated (not empty fallback)
            assert!(table.num_rows > 0, "Table should have rows");
            assert!(table.num_cols > 0, "Table should have columns");
            assert!(!table.table_cells.is_empty(), "Table should have cells");

            // Verify expected structure (if provided)
            if expected_rows > 0 {
                println!(
                    "    Expected: {} rows, {} cols",
                    expected_rows, expected_cols
                );
                assert_eq!(table.num_rows, expected_rows, "Row count mismatch");
                assert_eq!(table.num_cols, expected_cols, "Column count mismatch");
            }
        }
    }

    assert_eq!(
        table_count, expected_num_tables,
        "Expected {} tables on page {}, found {}",
        expected_num_tables, page_no, table_count
    );
    println!(
        "\n✓ TableFormer inference integration works! ({} tables found)",
        table_count
    );
}
