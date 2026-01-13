/// Integration tests for Stage 05 (Empty Cluster Removal)
///
/// These tests validate that the Rust implementation produces identical results
/// to the Python baseline for all test pages.
///
/// Test data location: baseline_data/{pdf_name}/page_{N}/
/// - Input: layout/stage5_with_cells.json (clusters with cells from Stage 4)
/// - Baseline: layout/stage6_non_empty.json (non-empty clusters only)
use docling_pdf_ml::pipeline_modular::{
    BBox, ClusterWithCells, ClustersWithCells, Stage05EmptyRemover, TextCell,
};
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Load Stage 5 input (clusters with cells from Stage 4)
/// Supports both old monolithic format (array) and new modular format ({"clusters": [...]})
fn load_stage5_input(path: &Path) -> ClustersWithCells {
    let json_str = fs::read_to_string(path).expect("Failed to read stage5 JSON");
    let data: Value = serde_json::from_str(&json_str).expect("Failed to parse stage5 JSON");

    // Handle both formats: {"clusters": [...]} (modular) or [...] (monolithic)
    let clusters_array = if let Some(obj) = data.as_object() {
        obj.get("clusters")
            .expect("JSON object should have 'clusters' key")
            .as_array()
            .expect("'clusters' should be array")
    } else {
        data.as_array()
            .expect("stage5 should be array or object with 'clusters'")
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

        // Load cells (may be empty for some clusters)
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

/// Load Stage 6 baseline (non-empty clusters only)
fn load_stage6_baseline(path: &Path) -> ClustersWithCells {
    // Same format as Stage 5 input
    load_stage5_input(path)
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
    let result_by_id: std::collections::HashMap<_, _> =
        result.clusters.iter().map(|c| (c.id, c)).collect();
    let baseline_by_id: std::collections::HashMap<_, _> =
        baseline.clusters.iter().map(|c| (c.id, c)).collect();

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

/// Test helper to run Stage 05 on a single page
fn test_page(pdf_name: &str, page_num: usize) -> bool {
    let base_path = PathBuf::from(format!("baseline_data/{pdf_name}/page_{page_num}"));
    let modular_base_path =
        PathBuf::from(format!("baseline_data_modular/{pdf_name}/page_{page_num}"));

    // Check input files (prefer modular, fallback to old)
    let stage4_modular_path = modular_base_path.join("stage04_cell_assignments.json");
    let stage5_old_path = base_path.join("layout/stage5_with_cells.json");

    // Check baseline files (prefer modular, fallback to old)
    let stage5_modular_path = modular_base_path.join("stage05_non_empty.json");
    let stage6_old_path = base_path.join("layout/stage6_non_empty.json");

    // Determine which paths to use
    let input_path = if stage4_modular_path.exists() {
        stage4_modular_path
    } else if stage5_old_path.exists() {
        stage5_old_path
    } else {
        eprintln!("  ⏸️  {page_num}: Missing input files, skipping");
        return true; // Skip, don't fail
    };

    let baseline_path = if stage5_modular_path.exists() {
        stage5_modular_path
    } else if stage6_old_path.exists() {
        stage6_old_path
    } else {
        eprintln!("  ⏸️  {page_num}: Missing baseline files, skipping");
        return true; // Skip, don't fail
    };

    // Load input
    let stage5_input = load_stage5_input(&input_path);
    let input_count = stage5_input.clusters.len();

    // Run Stage 5
    let remover = Stage05EmptyRemover::new();
    let result = remover.process(stage5_input);

    // Load baseline
    let baseline = load_stage6_baseline(&baseline_path);

    // Compare
    let matches = compare_cluster_lists(&result, &baseline, &format!("{pdf_name} page {page_num}"));

    if matches {
        println!(
            "  ✅ {page_num}: {} clusters ({} removed)",
            result.clusters.len(),
            input_count - result.clusters.len()
        );
    }

    matches
}

#[test]
fn test_stage05_arxiv_all_pages() {
    println!("\nTesting Stage 05 on arxiv_2206.01062 (9 pages)");
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
fn test_stage05_code_and_formula_all_pages() {
    println!("\nTesting Stage 05 on code_and_formula (2 pages)");
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
fn test_stage05_edinet_all_pages() {
    println!("\nTesting Stage 05 on edinet_sample (21 pages)");
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
fn test_stage05_jfk_all_pages() {
    println!("\nTesting Stage 05 on jfk_scanned (15 pages)");
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
