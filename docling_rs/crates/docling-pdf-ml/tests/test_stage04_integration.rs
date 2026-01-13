/// Integration tests for Stage 04 (Cell Assignment)
///
/// These tests validate that the Rust implementation produces identical results
/// to the Python baseline for all test pages.
///
/// Test data location: baseline_data/{pdf_name}/page_{N}/
/// - Input: layout/stage3_hf_postprocessed.json (labeled clusters)
/// - Input: preprocessing/textline_cells.json (OCR cells)
/// - Baseline: layout/stage5_with_cells.json (clusters with assigned cells)
use docling_pdf_ml::pipeline_modular::{
    BBox, ClusterWithCells, ClustersWithCells, LabeledCluster, LabeledClusters, OCRCells,
    Stage04CellAssigner, Stage04Config, TextCell,
};
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Label mapping from class IDs to labels (same as Python)
/// Source: docling_modular/generate_baselines.py LABEL_MAP
fn class_id_to_label(class_id: i32) -> String {
    match class_id {
        0 => "caption".to_string(),
        1 => "footnote".to_string(),
        2 => "formula".to_string(),
        3 => "list_item".to_string(),
        4 => "page_footer".to_string(),
        5 => "page_header".to_string(),
        6 => "picture".to_string(),
        7 => "section_header".to_string(),
        8 => "table".to_string(),
        9 => "text".to_string(),
        10 => "title".to_string(),
        11 => "checkbox_selected".to_string(),
        12 => "checkbox_unselected".to_string(),
        13 => "code".to_string(),
        16 => "key-value region".to_string(),
        _ => format!("unknown_{class_id}"),
    }
}

/// Load Stage 3 clusters from JSON
fn load_stage3_clusters(path: &Path) -> LabeledClusters {
    let json_str = fs::read_to_string(path).expect("Failed to read stage3 JSON");
    let data: Value = serde_json::from_str(&json_str).expect("Failed to parse stage3 JSON");

    let scores = data["scores"].as_array().expect("scores should be array");
    let labels = data["labels"].as_array().expect("labels should be array");
    let boxes = data["boxes"].as_array().expect("boxes should be array");

    let mut clusters = Vec::new();
    for (idx, ((score, label_id), bbox_arr)) in scores
        .iter()
        .zip(labels.iter())
        .zip(boxes.iter())
        .enumerate()
    {
        let score_val = score.as_f64().expect("score should be f64");
        let label_id_val = label_id.as_i64().expect("label should be i64") as i32;
        let bbox_vals = bbox_arr.as_array().expect("bbox should be array");

        let bbox = BBox::new(
            bbox_vals[0].as_f64().unwrap(),
            bbox_vals[1].as_f64().unwrap(),
            bbox_vals[2].as_f64().unwrap(),
            bbox_vals[3].as_f64().unwrap(),
        );

        clusters.push(LabeledCluster {
            id: idx,
            label: class_id_to_label(label_id_val),
            bbox,
            confidence: score_val,
            class_id: label_id_val,
        });
    }

    LabeledClusters { clusters }
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

        // N=599: NORMALIZE coordinates - Python modular baseline does min/max normalization
        // Source: docling_modular/generate_baselines.py:95-99
        // This ensures l<r and t<b regardless of coordinate system
        let l = rect["l"].as_f64().unwrap();
        let t = rect["t"].as_f64().unwrap();
        let r = rect["r"].as_f64().unwrap();
        let b = rect["b"].as_f64().unwrap();

        let bbox = BBox::new(
            l.min(r), // l = min(l, r)
            t.min(b), // t = min(t, b)
            r.max(l), // r = max(l, r)
            b.max(t), // b = max(t, b)
        );

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

/// Load Stage 4 baseline (modular format)
fn load_stage4_modular_baseline(path: &Path) -> ClustersWithCells {
    let json_str = fs::read_to_string(path).expect("Failed to read stage4 modular JSON");
    let data: Value = serde_json::from_str(&json_str).expect("Failed to parse stage4 modular JSON");

    // Modular format: {"clusters": [...]}
    let clusters_array = data["clusters"]
        .as_array()
        .expect("modular JSON should have 'clusters' array");

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

        // Load cells (modular format: bbox instead of rect)
        let cells_array = cluster_data["cells"]
            .as_array()
            .expect("cells should be array");
        let mut cells = Vec::new();

        for cell_data in cells_array {
            let text = cell_data["text"]
                .as_str()
                .expect("text should be string")
                .to_string();

            // Modular format uses bbox {l, t, r, b}
            let bbox_obj = &cell_data["bbox"];
            let cell_bbox = BBox::new(
                bbox_obj["l"].as_f64().unwrap(),
                bbox_obj["t"].as_f64().unwrap(),
                bbox_obj["r"].as_f64().unwrap(),
                bbox_obj["b"].as_f64().unwrap(),
            );

            cells.push(TextCell {
                text,
                bbox: cell_bbox,
                confidence: cell_data.get("confidence").and_then(|v| v.as_f64()),
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

    ClustersWithCells { clusters }
}

/// Load Stage 5 baseline (which contains Stage 4 output - clusters with cells)
/// This is the old monolithic format
fn load_stage5_baseline(path: &Path) -> ClustersWithCells {
    let json_str = fs::read_to_string(path).expect("Failed to read stage5 JSON");
    let data: Value = serde_json::from_str(&json_str).expect("Failed to parse stage5 JSON");

    let clusters_array = data.as_array().expect("stage5 should be array");

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
        let cells_array = cluster_data["cells"]
            .as_array()
            .expect("cells should be array");
        let mut cells = Vec::new();

        for cell_data in cells_array {
            let text = cell_data["text"]
                .as_str()
                .expect("text should be string")
                .to_string();

            // Stage 5 cells have rotated rect format (r_x0, r_y0, etc.)
            let rect = &cell_data["rect"];
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

            let cell_bbox = BBox::new(
                xs.iter().copied().fold(f64::INFINITY, f64::min),
                ys.iter().copied().fold(f64::INFINITY, f64::min),
                xs.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                ys.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            );

            cells.push(TextCell {
                text,
                bbox: cell_bbox,
                confidence: cell_data.get("confidence").and_then(|v| v.as_f64()),
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

    ClustersWithCells { clusters }
}

/// Compare cell assignments between result and baseline
fn compare_cell_assignments(
    result: &ClustersWithCells,
    baseline: &ClustersWithCells,
    page_name: &str,
) -> bool {
    // Check cluster count
    if result.clusters.len() != baseline.clusters.len() {
        eprintln!(
            "  ❌ {}: Cluster count mismatch: {} vs {}",
            page_name,
            result.clusters.len(),
            baseline.clusters.len()
        );
        return false;
    }

    // Sort clusters by ID for comparison
    let mut result_clusters = result.clusters.clone();
    let mut baseline_clusters = baseline.clusters.clone();
    result_clusters.sort_by_key(|c| c.id);
    baseline_clusters.sort_by_key(|c| c.id);

    // Compare each cluster's cell assignments
    let mut mismatches = 0;
    for (res_cluster, base_cluster) in result_clusters.iter().zip(baseline_clusters.iter()) {
        // Check IDs match
        if res_cluster.id != base_cluster.id {
            eprintln!("  ❌ {page_name}: Cluster ID mismatch");
            return false;
        }

        // Compare cell counts
        if res_cluster.cells.len() != base_cluster.cells.len() {
            eprintln!(
                "  ⚠️  Cluster {} ({}): {} cells vs {} baseline",
                res_cluster.id,
                res_cluster.label,
                res_cluster.cells.len(),
                base_cluster.cells.len()
            );
            mismatches += 1;
            continue;
        }

        // Compare cell texts (order may differ, so use sets)
        let res_texts: HashSet<_> = res_cluster.cells.iter().map(|c| &c.text).collect();
        let base_texts: HashSet<_> = base_cluster.cells.iter().map(|c| &c.text).collect();

        if res_texts != base_texts {
            eprintln!(
                "  ⚠️  Cluster {} ({}): Cell text mismatch",
                res_cluster.id, res_cluster.label
            );
            eprintln!(
                "      Only in result: {}",
                res_texts.difference(&base_texts).count()
            );
            eprintln!(
                "      Only in baseline: {}",
                base_texts.difference(&res_texts).count()
            );
            mismatches += 1;
        }
    }

    if mismatches > 0 {
        eprintln!("  ❌ {page_name}: {mismatches} clusters with cell assignment mismatches");
        return false;
    }

    println!("  ✅ {page_name}: All cell assignments match baseline");
    true
}

/// Test helper to run Stage 04 on a single page
fn test_page(pdf_name: &str, page_num: usize) -> bool {
    let base_path = PathBuf::from(format!("baseline_data/{pdf_name}/page_{page_num}"));
    let modular_base_path =
        PathBuf::from(format!("baseline_data_modular/{pdf_name}/page_{page_num}"));

    // Check if input files exist
    let stage3_path = base_path.join("layout/stage3_hf_postprocessed.json");
    let cells_path = base_path.join("preprocessing/textline_cells.json");

    // Prefer modular baseline, fall back to old baseline
    let stage4_modular_path = modular_base_path.join("stage04_cell_assignments.json");
    let stage5_path = base_path.join("layout/stage5_with_cells.json");

    if !stage3_path.exists() || !cells_path.exists() {
        eprintln!("  ⏸️  {page_num}: Missing input files, skipping",);
        return true; // Skip, don't fail
    }

    if !stage4_modular_path.exists() && !stage5_path.exists() {
        eprintln!("  ⏸️  {page_num}: Missing baseline files, skipping",);
        return true; // Skip, don't fail
    }

    // Load inputs
    let stage3_clusters = load_stage3_clusters(&stage3_path);
    let ocr_cells = load_ocr_cells(&cells_path);

    // Run Stage 4
    let assigner = Stage04CellAssigner::with_config(Stage04Config {
        min_overlap: 0.2,
        skip_text_clusters: false,
    });
    let result = assigner.process(stage3_clusters, ocr_cells);

    // Load baseline (prefer modular)
    let baseline = if stage4_modular_path.exists() {
        load_stage4_modular_baseline(&stage4_modular_path)
    } else {
        load_stage5_baseline(&stage5_path)
    };

    // Compare
    compare_cell_assignments(&result, &baseline, &format!("{pdf_name} page {page_num}"))
}

#[test]
fn test_stage04_arxiv_all_pages() {
    println!("\nTesting Stage 04 on arxiv_2206.01062 (9 pages)");
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
fn test_stage04_code_and_formula_all_pages() {
    println!("\nTesting Stage 04 on code_and_formula (2 pages)");
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
fn test_stage04_edinet_all_pages() {
    println!("\nTesting Stage 04 on edinet_sample (21 pages)");
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
fn test_stage04_jfk_all_pages() {
    println!("\nTesting Stage 04 on jfk_scanned (15 pages)");
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
