/// Integration tests for Stage 06 (Orphan Cluster Creation)
///
/// These tests validate that the Rust implementation produces identical results
/// to the Python baseline for all test pages.
///
/// Test data location: `baseline_data/{pdf_name}/page_{N}/`
/// - Input: `layout/stage6_non_empty.json` (non-empty clusters from Stage 5)
/// - Input: `preprocessing/textline_cells.json` (all OCR cells)
/// - Baseline: `layout/stage7_with_orphans.json` (clusters with orphans added)
use docling_pdf_ml::pipeline_modular::{
    BBox, ClusterWithCells, ClustersWithCells, OCRCells, Stage06OrphanCreator, TextCell,
};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Load Stage 6 input (non-empty clusters from Stage 5)
/// Supports both old monolithic format (array) and new modular format ({"clusters": [...]})
fn load_stage6_input(path: &Path) -> ClustersWithCells {
    let json_str = fs::read_to_string(path).expect("Failed to read stage6 JSON");
    let data: Value = serde_json::from_str(&json_str).expect("Failed to parse stage6 JSON");

    // Handle both formats: {"clusters": [...]} (modular) or [...] (monolithic)
    let clusters_array = if let Some(obj) = data.as_object() {
        obj.get("clusters")
            .expect("JSON object should have 'clusters' key")
            .as_array()
            .expect("'clusters' should be array")
    } else {
        data.as_array()
            .expect("stage6 should be array or object with 'clusters'")
    };

    let mut clusters = Vec::new();
    for cluster_data in clusters_array {
        let id = cluster_data["id"].as_u64().expect("id should be u64") as usize;
        let label = cluster_data["label"]
            .as_str()
            .expect("label should be string")
            .to_string();
        let confidence = cluster_data["confidence"]
            .as_f64()
            .expect("confidence should be f64");
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

        // Load cells
        let mut cells = Vec::new();
        if let Some(cells_array) = cluster_data.get("cells").and_then(|v| v.as_array()) {
            for cell_data in cells_array {
                let text = cell_data["text"]
                    .as_str()
                    .expect("text should be string")
                    .to_string();

                // Handle both rect (old format) and bbox (modular format)
                let cell_bbox = if let Some(rect) = cell_data.get("rect") {
                    // Old format: rotated rect (r_x0, r_y0, etc.)
                    let xs = [
                        rect["r_x0"].as_f64().unwrap(),
                        rect["r_x1"].as_f64().unwrap(),
                        rect["r_x2"].as_f64().unwrap(),
                        rect["r_x3"].as_f64().unwrap(),
                    ];
                    let ys = [
                        rect["r_y0"].as_f64().unwrap(),
                        rect["r_y1"].as_f64().unwrap(),
                        rect["r_y2"].as_f64().unwrap(),
                        rect["r_y3"].as_f64().unwrap(),
                    ];

                    BBox::new(
                        xs.iter().copied().fold(f64::INFINITY, f64::min),
                        ys.iter().copied().fold(f64::INFINITY, f64::min),
                        xs.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                        ys.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                    )
                } else if let Some(bbox_obj) = cell_data.get("bbox") {
                    // New modular format: bbox {l, t, r, b}
                    BBox::new(
                        bbox_obj["l"].as_f64().unwrap(),
                        bbox_obj["t"].as_f64().unwrap(),
                        bbox_obj["r"].as_f64().unwrap(),
                        bbox_obj["b"].as_f64().unwrap(),
                    )
                } else {
                    panic!("Cell should have 'rect' or 'bbox' field");
                };

                cells.push(TextCell {
                    text,
                    bbox: cell_bbox,
                    confidence: cell_data.get("confidence").and_then(|v| v.as_f64()),
                    is_bold: false,
                    is_italic: false,
                });
            }
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

    ClustersWithCells { clusters }
}

/// Load OCR cells from preprocessing JSON
fn load_ocr_cells(path: &Path) -> OCRCells {
    let json_str = fs::read_to_string(path).expect("Failed to read cells JSON");
    let data: Value = serde_json::from_str(&json_str).expect("Failed to parse cells JSON");

    let cells_array = data.as_array().expect("cells should be array");

    let mut cells = Vec::new();
    for cell_data in cells_array {
        let text = cell_data["text"].as_str().expect("text should be string");
        let rect = &cell_data["rect"];

        let l = rect["l"].as_f64().unwrap();
        let t = rect["t"].as_f64().unwrap();
        let r = rect["r"].as_f64().unwrap();
        let b = rect["b"].as_f64().unwrap();

        // N=592: DO NOT normalize - Python filters invalid bboxes (area <= 0)
        let bbox = BBox::new(l, t, r, b);

        cells.push(TextCell {
            text: text.to_string(),
            bbox,
            confidence: cell_data.get("confidence").and_then(|v| v.as_f64()),
            is_bold: false,
            is_italic: false,
        });
    }

    OCRCells { cells }
}

/// Load Stage 7 baseline (clusters with orphans)
fn load_stage7_baseline(path: &Path) -> ClustersWithCells {
    // Same format as Stage 6 input
    load_stage6_input(path)
}

/// Compare cluster lists between result and baseline
fn compare_cluster_lists(
    result: &ClustersWithCells,
    baseline: &ClustersWithCells,
    page_name: &str,
) -> bool {
    // Check cluster count
    if result.clusters.len() != baseline.clusters.len() {
        eprintln!(
            "  ❌ {}: Cluster count mismatch: {} vs {} baseline",
            page_name,
            result.clusters.len(),
            baseline.clusters.len()
        );
        return false;
    }

    // Build ID maps for comparison
    let result_by_id: HashMap<_, _> = result.clusters.iter().map(|c| (c.id, c)).collect();
    let baseline_by_id: HashMap<_, _> = baseline.clusters.iter().map(|c| (c.id, c)).collect();

    // Check IDs match
    let result_ids: HashSet<_> = result_by_id.keys().copied().collect();
    let baseline_ids: HashSet<_> = baseline_by_id.keys().copied().collect();

    if result_ids != baseline_ids {
        let missing: Vec<_> = baseline_ids.difference(&result_ids).collect();
        let extra: Vec<_> = result_ids.difference(&baseline_ids).collect();

        if !missing.is_empty() {
            eprintln!("  ❌ {page_name}: Missing IDs: {missing:?}");
        }
        if !extra.is_empty() {
            eprintln!("  ❌ {page_name}: Extra IDs: {extra:?}");
        }
        return false;
    }

    // Compare each cluster
    for cluster_id in result_ids.iter() {
        let result_cluster = result_by_id[cluster_id];
        let baseline_cluster = baseline_by_id[cluster_id];

        // Check label
        if result_cluster.label != baseline_cluster.label {
            eprintln!(
                "  ❌ {}: Cluster {} label mismatch: {} vs {}",
                page_name, cluster_id, result_cluster.label, baseline_cluster.label
            );
            return false;
        }

        // Check cell count
        if result_cluster.cells.len() != baseline_cluster.cells.len() {
            eprintln!(
                "  ❌ {}: Cluster {} cell count mismatch: {} vs {}",
                page_name,
                cluster_id,
                result_cluster.cells.len(),
                baseline_cluster.cells.len()
            );
            return false;
        }

        // Compare cell texts (order may differ, so use sets)
        let result_texts: HashSet<_> = result_cluster.cells.iter().map(|c| &c.text).collect();
        let baseline_texts: HashSet<_> = baseline_cluster.cells.iter().map(|c| &c.text).collect();

        if result_texts != baseline_texts {
            eprintln!("  ❌ {page_name}: Cluster {cluster_id} cell text mismatch");
            return false;
        }
    }

    true
}

/// Test helper to run Stage 06 on a single page
fn test_page(pdf_name: &str, page_num: usize) -> bool {
    let base_path = PathBuf::from(format!("baseline_data/{pdf_name}/page_{page_num}"));
    let modular_base_path =
        PathBuf::from(format!("baseline_data_modular/{pdf_name}/page_{page_num}"));

    // Check input files (prefer modular, fallback to old)
    let stage5_modular_path = modular_base_path.join("stage05_non_empty.json");
    let stage6_old_path = base_path.join("layout/stage6_non_empty.json");
    let cells_path = base_path.join("preprocessing/textline_cells.json");

    // Check baseline files (prefer modular, fallback to old)
    let stage6_modular_path = modular_base_path.join("stage06_with_orphans.json");
    let stage7_old_path = base_path.join("layout/stage7_with_orphans.json");

    // Determine which paths to use
    let input_path = if stage5_modular_path.exists() {
        stage5_modular_path
    } else if stage6_old_path.exists() {
        stage6_old_path
    } else {
        eprintln!("  ⏸️  {page_num}: Missing input files, skipping");
        return true; // Skip, don't fail
    };

    if !cells_path.exists() {
        eprintln!("  ⏸️  {page_num}: Missing cells file, skipping");
        return true; // Skip, don't fail
    }

    let baseline_path = if stage6_modular_path.exists() {
        stage6_modular_path
    } else if stage7_old_path.exists() {
        stage7_old_path
    } else {
        eprintln!("  ⏸️  {page_num}: Missing baseline files, skipping");
        return true; // Skip, don't fail
    };

    // Load inputs
    let stage6_input = load_stage6_input(&input_path);
    let all_cells = load_ocr_cells(&cells_path);
    let input_count = stage6_input.clusters.len();

    // Run Stage 6
    let creator = Stage06OrphanCreator::new();
    let result = creator.process(stage6_input, all_cells);

    // Load baseline
    let baseline = load_stage7_baseline(&baseline_path);

    // Compare
    let matches = compare_cluster_lists(&result, &baseline, &format!("{pdf_name} page {page_num}"));

    if matches {
        let orphans_created = result.clusters.len() - input_count;
        println!(
            "  ✅ {page_num}: {} clusters ({} orphans created)",
            result.clusters.len(),
            orphans_created
        );
    }

    matches
}

#[test]
fn test_stage06_arxiv_all_pages() {
    println!("\nTesting Stage 06 on arxiv_2206.01062 (9 pages)");
    let mut passed = 0;
    let mut total = 0;

    for page_num in 0..9 {
        total += 1;
        if test_page("arxiv_2206.01062", page_num) {
            passed += 1;
        }
    }

    println!("\narxiv: {passed}/{total} pages passed");
    assert_eq!(passed, total, "All arxiv pages should pass");
}

#[test]
fn test_stage06_code_and_formula_all_pages() {
    println!("\nTesting Stage 06 on code_and_formula (2 pages)");
    let mut passed = 0;
    let mut total = 0;

    for page_num in 0..2 {
        total += 1;
        if test_page("code_and_formula", page_num) {
            passed += 1;
        }
    }

    println!("\ncode_and_formula: {passed}/{total} pages passed");
    assert_eq!(passed, total, "All code_and_formula pages should pass");
}

#[test]
fn test_stage06_edinet_all_pages() {
    println!("\nTesting Stage 06 on edinet_sample (21 pages)");
    let mut passed = 0;
    let mut total = 0;

    for page_num in 0..21 {
        total += 1;
        if test_page("edinet_sample", page_num) {
            passed += 1;
        }
    }

    println!("\nedinet: {passed}/{total} pages passed");
    assert_eq!(passed, total, "All edinet pages should pass");
}

#[test]
fn test_stage06_jfk_all_pages() {
    println!("\nTesting Stage 06 on jfk_scanned (15 pages)");
    let mut passed = 0;
    let mut total = 0;

    for page_num in 0..15 {
        total += 1;
        if test_page("jfk_scanned", page_num) {
            passed += 1;
        }
    }

    println!("\njfk: {passed}/{total} pages passed");
    assert_eq!(passed, total, "All jfk pages should pass");
}
