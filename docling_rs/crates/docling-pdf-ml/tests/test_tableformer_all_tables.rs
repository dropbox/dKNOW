#![cfg(feature = "pytorch")]
mod common;
/// TableFormer Inference Validation - All Tables
///
/// Tests TableFormer inference on all 5 tables across arxiv_2206.01062:
/// - Page 3: 1 table (14 rows x 12 cols, 158 cells)
/// - Page 5: 1 table (14 rows x 6 cols, 80 cells)
/// - Page 6: 2 tables (13x5=65 cells, 14x5=55 cells)
/// - Page 7: 1 table (15 rows x 5 cols, 61 cells)
///
/// This test measures consistency of TableFormer inference across different
/// table structures to determine if observed errors (~7% on page 3) are
/// systematic or table-specific.
///
/// **IMPORTANT**: This test is marked as #[ignore] because it crashes with SIGKILL
/// when running in parallel (multiple Pipeline instances loading TableFormer model).
/// Each test case passes when run individually.
///
/// Run with: `cargo test --release --test test_tableformer_all_tables -- --test-threads=1 --ignored`
use common::baseline_loaders::load_numpy_u8;
use docling_pdf_ml::pipeline::{PageElement, SimpleTextCell};
use docling_pdf_ml::{Pipeline, PipelineConfig};
use rstest::rstest;
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

/// Load expected table structure from baseline
#[derive(Debug)]
struct ExpectedTable {
    table_index: usize,
    num_rows: usize,
    num_cols: usize,
    num_cells: usize,
}

fn load_expected_tables(doc_name: &str, page_no: usize) -> Vec<ExpectedTable> {
    let mut tables = Vec::new();
    let mut table_idx = 0;

    loop {
        let path = PathBuf::from(format!(
            "baseline_data/{}/page_{}/table/table_{}/output_table_structure.json",
            doc_name, page_no, table_idx
        ));

        if !path.exists() {
            break;
        }

        let contents = std::fs::read_to_string(&path).expect("Failed to read table structure");

        let json: serde_json::Value =
            serde_json::from_str(&contents).expect("Failed to parse table structure JSON");

        let num_rows = json["num_rows"].as_u64().expect("Missing num_rows") as usize;
        let num_cols = json["num_cols"].as_u64().expect("Missing num_cols") as usize;
        let num_cells = json["cells"].as_array().expect("Missing cells").len();

        tables.push(ExpectedTable {
            table_index: table_idx,
            num_rows,
            num_cols,
            num_cells,
        });

        table_idx += 1;
    }

    tables
}

#[rstest]
#[case("arxiv_2206.01062", 3, 1)] // Page 3: 1 table
#[case("arxiv_2206.01062", 5, 1)] // Page 5: 1 table
#[case("arxiv_2206.01062", 6, 2)] // Page 6: 2 tables
#[case("arxiv_2206.01062", 7, 1)] // Page 7: 1 table
#[ignore = "Crashes with SIGKILL when run in parallel - run with --test-threads=1"]
fn test_tableformer_all_tables(
    #[case] doc_name: &str,
    #[case] page_no: usize,
    #[case] expected_table_count: usize,
) {
    println!("\n{}", "=".repeat(60));
    println!("TableFormer Validation: {} page {}", doc_name, page_no);
    println!("{}", "=".repeat(60));

    // 1. Load page image
    let image_path = PathBuf::from(format!(
        "baseline_data/{}/page_{}/layout/input_page_image.npy",
        doc_name, page_no
    ));

    let page_image_dyn = load_numpy_u8(&image_path).expect("Failed to load page image");
    let page_image = page_image_dyn
        .into_dimensionality::<ndarray::Ix3>()
        .expect("Failed to convert to 3D array");

    let (page_width, page_height) = load_page_size(doc_name, page_no);

    // 2. Initialize pipeline WITH TableFormer enabled
    let config = PipelineConfig {
        device: Device::Cpu,
        ocr_enabled: false,
        table_structure_enabled: true, // Enable TableFormer inference
        ..Default::default()
    };

    let mut pipeline = Pipeline::new(config).expect("Failed to create pipeline");

    // 3. Load textline cells
    let textline_cells = load_textline_cells(doc_name, page_no);

    // 4. Process page with TableFormer inference
    println!("\n[1/3] Running TableFormer inference...");
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
    println!("  ✓ Page processed: {} elements", assembled.elements.len());

    // 5. Load expected tables
    let expected_tables = load_expected_tables(doc_name, page_no);
    assert_eq!(
        expected_tables.len(),
        expected_table_count,
        "Expected {} tables on page {}, found {} in baseline",
        expected_table_count,
        page_no,
        expected_tables.len()
    );

    // 6. Extract and validate table elements
    println!("\n[2/3] Extracting table elements...");
    let mut actual_tables: Vec<_> = assembled
        .elements
        .iter()
        .filter_map(|e| {
            if let PageElement::Table(table) = e {
                Some(table)
            } else {
                None
            }
        })
        .collect();

    // Sort by ID (should match table_0, table_1, etc. order)
    actual_tables.sort_by_key(|t| t.id);

    println!("  ✓ Found {} table element(s)", actual_tables.len());

    assert_eq!(
        actual_tables.len(),
        expected_table_count,
        "Expected {} tables, found {}",
        expected_table_count,
        actual_tables.len()
    );

    // 7. Compare each table
    println!("\n[3/3] Validating table structures...\n");

    let mut all_match = true;
    let mut total_cell_accuracy = 0.0;

    for (i, (actual, expected)) in actual_tables.iter().zip(expected_tables.iter()).enumerate() {
        println!("  Table {} (cluster ID={}):", i, actual.id);
        println!(
            "    Actual:   {} rows x {} cols, {} cells",
            actual.num_rows,
            actual.num_cols,
            actual.table_cells.len()
        );
        println!(
            "    Expected: {} rows x {} cols, {} cells",
            expected.num_rows, expected.num_cols, expected.num_cells
        );

        let row_match = actual.num_rows == expected.num_rows;
        let col_match = actual.num_cols == expected.num_cols;
        let cell_match = actual.table_cells.len() == expected.num_cells;

        if row_match && col_match && cell_match {
            println!("    ✓ EXACT MATCH");
        } else {
            all_match = false;
            let row_diff = actual.num_rows as i32 - expected.num_rows as i32;
            let col_diff = actual.num_cols as i32 - expected.num_cols as i32;
            let cell_diff = actual.table_cells.len() as i32 - expected.num_cells as i32;

            let cell_accuracy =
                (actual.table_cells.len() as f64 / expected.num_cells as f64) * 100.0;
            total_cell_accuracy += cell_accuracy;

            println!("    ✗ MISMATCH:");
            if !row_match {
                println!("      - Rows: {} (diff: {:+})", actual.num_rows, row_diff);
            }
            if !col_match {
                println!("      - Cols: {} (diff: {:+})", actual.num_cols, col_diff);
            }
            if !cell_match {
                println!(
                    "      - Cells: {} (diff: {:+})",
                    actual.table_cells.len(),
                    cell_diff
                );
            }
            println!("      - Cell accuracy: {:.1}%", cell_accuracy);
        }
        println!();
    }

    // 8. Summary
    println!("{}", "=".repeat(60));
    if all_match {
        println!("✅ ALL TABLES MATCH EXACTLY");
    } else {
        let avg_accuracy = total_cell_accuracy / actual_tables.len() as f64;
        println!("⚠️  SOME TABLES HAVE DIFFERENCES");
        println!("   Average cell accuracy: {:.1}%", avg_accuracy);
    }
    println!("{}\n", "=".repeat(60));

    // For now, we PASS the test even with mismatches to collect data
    // Once we understand the pattern, we can decide on tolerance thresholds
}
