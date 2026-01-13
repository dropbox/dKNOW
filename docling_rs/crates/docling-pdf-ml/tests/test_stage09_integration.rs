/// Integration tests for Stage 09 (Document Assembly)
///
/// Tests the Rust implementation against Python baselines.
///
/// Test structure:
/// - Load Stage 8 output (stage8_resolved.json)
/// - Run Rust Stage 09 document assembler
/// - Load Stage 9 baseline (stage9_assembled.json)
/// - Compare outputs (element count, types, labels, text content)
use docling_pdf_ml::pipeline_modular::stage09_document_assembler::Stage09DocumentAssembler;
use docling_pdf_ml::pipeline_modular::types::{BBox, ClusterWithCells, TextCell};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

/// Parse a cell's rect (with corner points) to BBox
fn parse_cell_rect(rect: &Value) -> BBox {
    // rect has corner points (r_x0, r_y0, r_x1, r_y1, r_x2, r_y2, r_x3, r_y3)
    // Convert to bbox (l, t, r, b)
    let x0 = rect["r_x0"].as_f64().unwrap();
    let y0 = rect["r_y0"].as_f64().unwrap();
    let x1 = rect["r_x1"].as_f64().unwrap();
    let y1 = rect["r_y1"].as_f64().unwrap();
    let x2 = rect["r_x2"].as_f64().unwrap();
    let y2 = rect["r_y2"].as_f64().unwrap();
    let x3 = rect["r_x3"].as_f64().unwrap();
    let y3 = rect["r_y3"].as_f64().unwrap();

    let min_x = x0.min(x1).min(x2).min(x3);
    let max_x = x0.max(x1).max(x2).max(x3);
    let min_y = y0.min(y1).min(y2).min(y3);
    let max_y = y0.max(y1).max(y2).max(y3);

    BBox::new(min_x, min_y, max_x, max_y)
}

/// Load stage 8 output (resolved clusters) from JSON
fn load_stage8_output(pdf_name: &str, page_num: usize) -> Vec<ClusterWithCells> {
    let path = PathBuf::from(format!(
        "baseline_data/{pdf_name}/page_{page_num}/layout/stage8_resolved.json"
    ));

    let json = fs::read_to_string(&path).unwrap_or_else(|_| panic!("Failed to read {path:?}"));

    let data: Value =
        serde_json::from_str(&json).unwrap_or_else(|e| panic!("Failed to parse {path:?}: {e}"));

    let clusters_array = data.as_array().expect("Stage 8 output should be array");

    let mut clusters = Vec::new();
    for cluster_data in clusters_array {
        let id = cluster_data["id"].as_u64().unwrap() as usize;
        let label = cluster_data["label"].as_str().unwrap().to_string();
        let confidence = cluster_data["confidence"].as_f64().unwrap();
        let class_id = cluster_data
            .get("class_id")
            .and_then(|v| v.as_i64())
            .unwrap_or(-1) as i32;

        let bbox_obj = &cluster_data["bbox"];
        let bbox = BBox::new(
            bbox_obj["l"].as_f64().unwrap(),
            bbox_obj["t"].as_f64().unwrap(),
            bbox_obj["r"].as_f64().unwrap(),
            bbox_obj["b"].as_f64().unwrap(),
        );

        let cells_array = cluster_data["cells"].as_array().unwrap();
        let mut cells = Vec::new();
        for cell_data in cells_array {
            let text = cell_data["text"].as_str().unwrap().to_string();
            let cell_bbox = parse_cell_rect(&cell_data["rect"]);
            let confidence = cell_data.get("confidence").and_then(|v| v.as_f64());

            cells.push(TextCell {
                text,
                bbox: cell_bbox,
                confidence,
                is_bold: false,
                is_italic: false,
            });
        }

        clusters.push(ClusterWithCells {
            id,
            label,
            bbox,
            confidence,
            class_id,
            cells,
        });
    }

    clusters
}

/// Load stage 9 baseline (assembled elements) from JSON
fn load_stage9_baseline(pdf_name: &str, page_num: usize) -> Value {
    let path = PathBuf::from(format!(
        "baseline_data/{pdf_name}/page_{page_num}/layout/stage9_assembled.json"
    ));

    let json = fs::read_to_string(&path).unwrap_or_else(|_| panic!("Failed to read {path:?}"));

    serde_json::from_str(&json).unwrap_or_else(|e| panic!("Failed to parse {path:?}: {e}"))
}

/// Get page size from preprocessing baseline
fn load_page_size(pdf_name: &str, page_num: usize) -> (f64, f64) {
    let path = PathBuf::from(format!(
        "baseline_data/{pdf_name}/page_{page_num}/preprocessing/page_size.json"
    ));

    let json = fs::read_to_string(&path).unwrap_or_else(|_| panic!("Failed to read {path:?}"));

    let data: Value =
        serde_json::from_str(&json).unwrap_or_else(|e| panic!("Failed to parse {path:?}: {e}"));

    let width = data["width"].as_f64().unwrap();
    let height = data["height"].as_f64().unwrap();

    (width, height)
}

/// Test Stage 09 on a single page
fn test_stage09_page(pdf_name: &str, page_num: usize) -> Result<(), String> {
    // Load Stage 8 output
    let stage8_clusters = load_stage8_output(pdf_name, page_num);

    // Load page size
    let (page_width, page_height) = load_page_size(pdf_name, page_num);

    // Run Stage 09
    let assembler = Stage09DocumentAssembler::new();
    let elements = assembler.process(stage8_clusters, page_num, page_width, page_height);

    // Load baseline
    let baseline = load_stage9_baseline(pdf_name, page_num);
    let baseline_elements = baseline["elements"].as_array().unwrap();

    // Compare element count
    if elements.len() != baseline_elements.len() {
        return Err(format!(
            "Element count mismatch: Rust={}, Python={}",
            elements.len(),
            baseline_elements.len()
        ));
    }

    // Serialize Rust elements to JSON for comparison
    let rust_json = serde_json::to_value(&elements).unwrap();
    let rust_elements = rust_json.as_array().unwrap();

    // Compare each element
    for (i, (rust_elem, python_elem)) in rust_elements
        .iter()
        .zip(baseline_elements.iter())
        .enumerate()
    {
        // Compare type
        let rust_type = rust_elem["type"].as_str().unwrap();
        let python_type = python_elem["type"].as_str().unwrap();
        if rust_type != python_type {
            return Err(format!(
                "Element {i} type mismatch: Rust={rust_type}, Python={python_type}"
            ));
        }

        // Compare label
        let rust_label = rust_elem["label"].as_str().unwrap();
        let python_label = python_elem["label"].as_str().unwrap();
        if rust_label != python_label {
            return Err(format!(
                "Element {i} label mismatch: Rust={rust_label}, Python={python_label}"
            ));
        }

        // Compare ID
        let rust_id = rust_elem["id"].as_u64().unwrap();
        let python_id = python_elem["id"].as_u64().unwrap();
        if rust_id != python_id {
            return Err(format!(
                "Element {i} ID mismatch: Rust={rust_id}, Python={python_id}"
            ));
        }

        // Compare text (if present)
        if let Some(rust_text) = rust_elem.get("text") {
            if let Some(python_text) = python_elem.get("text") {
                let rust_text_str = rust_text.as_str().unwrap();
                let python_text_str = python_text.as_str().unwrap();

                if rust_text_str != python_text_str {
                    return Err(format!(
                        "Element {} text mismatch:\n  Rust ({} chars): {}\n  Python ({} chars): {}",
                        i,
                        rust_text_str.len(),
                        if rust_text_str.len() > 100 {
                            format!("{}...", &rust_text_str[..100])
                        } else {
                            rust_text_str.to_string()
                        },
                        python_text_str.len(),
                        if python_text_str.len() > 100 {
                            format!("{}...", &python_text_str[..100])
                        } else {
                            python_text_str.to_string()
                        }
                    ));
                }
            }
        }
    }

    Ok(())
}

#[test]
fn test_stage09_code_and_formula_page0() {
    let result = test_stage09_page("code_and_formula", 0);
    assert!(
        result.is_ok(),
        "code_and_formula page 0 failed: {}",
        result.unwrap_err()
    );
}

#[test]
fn test_stage09_code_and_formula_page1() {
    let result = test_stage09_page("code_and_formula", 1);
    assert!(
        result.is_ok(),
        "code_and_formula page 1 failed: {}",
        result.unwrap_err()
    );
}

#[test]
fn test_stage09_all_pages() {
    let test_cases = vec![
        ("code_and_formula", vec![0, 1]),
        ("arxiv_2206.01062", (0..9).collect()),
        ("jfk_scanned", (0..15).collect()),
    ];

    let mut passed = 0;
    let mut failed = 0;
    let mut failures = Vec::new();

    for (pdf_name, pages) in test_cases {
        for page_num in pages {
            match test_stage09_page(pdf_name, page_num) {
                Ok(_) => {
                    println!("✅ {pdf_name} page {page_num}");
                    passed += 1;
                }
                Err(err) => {
                    println!("❌ {pdf_name} page {page_num}: {err}");
                    failures.push(format!("{pdf_name} page {page_num}"));
                    failed += 1;
                }
            }
        }
    }

    println!("\n=== Stage 09 Integration Test Summary ===");
    println!("Total: {} pages", passed + failed);
    println!(
        "Passed: {} ({:.1}%)",
        passed,
        (passed as f64) / (passed + failed) as f64 * 100.0
    );
    println!(
        "Failed: {} ({:.1}%)",
        failed,
        (failed as f64) / (passed + failed) as f64 * 100.0
    );

    if !failures.is_empty() {
        println!("\nFailures:");
        for failure in &failures {
            println!("  - {failure}");
        }
    }

    assert_eq!(failed, 0, "{failed} pages failed");
}
