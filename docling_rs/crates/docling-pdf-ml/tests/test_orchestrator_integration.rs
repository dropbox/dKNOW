//! Integration test for the Rust modular pipeline orchestrator
//!
//! This test validates that the Rust orchestrator produces the same outputs
//! as the Python modular pipeline orchestrator.

#![allow(
    clippy::doc_markdown,
    clippy::use_debug,
    clippy::cast_possible_truncation,
    clippy::option_if_let_else
)]
use std::path::PathBuf;

use docling_pdf_ml::pipeline_modular::{
    types::{
        BBox, ClusterWithCells, ClustersWithCells, LabeledCluster, LabeledClusters, OCRCells,
        TextCell,
    },
    ModularPipeline,
};

/// Label mapping from class IDs to labels
/// Source: src/pipeline/data_structures.rs DocItemLabel::from_class_id()
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
        11 => "document_index".to_string(),
        12 => "checkbox_selected".to_string(),
        13 => "code".to_string(),
        14 => "checkbox_unselected".to_string(),
        15 => "form".to_string(),
        16 => "key_value_region".to_string(),
        _ => format!("unknown_{class_id}"),
    }
}

/// Load Stage 3 clusters from baseline data
fn load_stage3_clusters(pdf_name: &str, page_num: usize) -> LabeledClusters {
    let path = PathBuf::from(format!(
        "baseline_data/{pdf_name}/page_{page_num}/layout/stage3_hf_postprocessed.json"
    ));

    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("Failed to load {path:?}: {e}"));

    let data: serde_json::Value =
        serde_json::from_str(&content).unwrap_or_else(|e| panic!("Failed to parse {path:?}: {e}"));

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

/// Load OCR cells from baseline data
fn load_ocr_cells(pdf_name: &str, page_num: usize) -> OCRCells {
    let path = PathBuf::from(format!(
        "baseline_data/{pdf_name}/page_{page_num}/preprocessing/textline_cells.json"
    ));

    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("Failed to load {path:?}: {e}"));

    let data: serde_json::Value =
        serde_json::from_str(&content).unwrap_or_else(|e| panic!("Failed to parse {path:?}: {e}"));

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
            confidence: None,
            is_bold: false,
            is_italic: false,
        });
    }

    OCRCells { cells }
}

/// Load Python modular pipeline output (Stage 08 final) from baseline_data_modular
fn load_python_stage08_output(pdf_name: &str, page_num: usize) -> Option<ClustersWithCells> {
    let path = PathBuf::from(format!(
        "baseline_data_modular/{pdf_name}/page_{page_num}/stage08_resolved_final.json"
    ));

    // Return None if file doesn't exist (not all PDFs have modular baselines)
    if !path.exists() {
        return None;
    }

    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("Failed to load {path:?}: {e}"));

    let data: serde_json::Value =
        serde_json::from_str(&content).unwrap_or_else(|e| panic!("Failed to parse {path:?}: {e}"));

    // Handle {"clusters": [...]} format
    let clusters_value = if let Some(obj) = data.as_object() {
        obj.get("clusters")
            .expect("Expected 'clusters' key in modular baseline")
            .clone()
    } else {
        panic!("Expected object with 'clusters' key in modular baseline");
    };

    let clusters: Vec<ClusterWithCells> = serde_json::from_value(clusters_value)
        .unwrap_or_else(|e| panic!("Failed to deserialize clusters from {path:?}: {e}"));

    // Skip pages with 0 clusters (incomplete/failed baselines)
    if clusters.is_empty() {
        return None;
    }

    Some(ClustersWithCells { clusters })
}

/// Compare two cluster outputs
/// Note: Clusters may be in different order, so we sort by bbox before comparing
fn compare_clusters(
    rust_output: &ClustersWithCells,
    python_output: &ClustersWithCells,
    pdf_name: &str,
    page_num: usize,
) {
    // Compare cluster counts
    assert_eq!(
        rust_output.clusters.len(),
        python_output.clusters.len(),
        "{pdf_name} page {page_num}: Cluster count mismatch"
    );

    // Sort both outputs by bbox (l, t, r, b) for consistent comparison
    let mut rust_sorted = rust_output.clusters.clone();
    let mut python_sorted = python_output.clusters.clone();

    rust_sorted.sort_by(|a, b| {
        a.bbox
            .l
            .partial_cmp(&b.bbox.l)
            .unwrap()
            .then(a.bbox.t.partial_cmp(&b.bbox.t).unwrap())
            .then(a.bbox.r.partial_cmp(&b.bbox.r).unwrap())
            .then(a.bbox.b.partial_cmp(&b.bbox.b).unwrap())
    });

    python_sorted.sort_by(|a, b| {
        a.bbox
            .l
            .partial_cmp(&b.bbox.l)
            .unwrap()
            .then(a.bbox.t.partial_cmp(&b.bbox.t).unwrap())
            .then(a.bbox.r.partial_cmp(&b.bbox.r).unwrap())
            .then(a.bbox.b.partial_cmp(&b.bbox.b).unwrap())
    });

    // Compare each cluster
    for (i, (rust_cluster, python_cluster)) in
        rust_sorted.iter().zip(python_sorted.iter()).enumerate()
    {
        // Compare labels
        assert_eq!(
            rust_cluster.label, python_cluster.label,
            "{} page {}, cluster {}: Label mismatch\n  Rust bbox: {:?}\n  Python bbox: {:?}",
            pdf_name, page_num, i, rust_cluster.bbox, python_cluster.bbox
        );

        // Compare bboxes (allow small floating-point differences)
        let bbox_diff = (rust_cluster.bbox.l - python_cluster.bbox.l)
            .abs()
            .max((rust_cluster.bbox.t - python_cluster.bbox.t).abs())
            .max((rust_cluster.bbox.r - python_cluster.bbox.r).abs())
            .max((rust_cluster.bbox.b - python_cluster.bbox.b).abs());

        assert!(
            bbox_diff < 0.01,
            "{} page {}, cluster {}: Bbox diff {} > 0.01\n  Rust: {:?}\n  Python: {:?}",
            pdf_name,
            page_num,
            i,
            bbox_diff,
            rust_cluster.bbox,
            python_cluster.bbox
        );

        // Compare cell counts
        assert_eq!(
            rust_cluster.cells.len(),
            python_cluster.cells.len(),
            "{pdf_name} page {page_num}, cluster {i}: Cell count mismatch"
        );
    }

    println!(
        "✅ {} page {}: PASS ({} clusters, perfect match)",
        pdf_name,
        page_num,
        rust_output.clusters.len()
    );
}

#[test]
fn test_orchestrator_arxiv_page0() {
    // Load inputs
    let stage3_clusters = load_stage3_clusters("arxiv_2206.01062", 0);
    let ocr_cells = load_ocr_cells("arxiv_2206.01062", 0);

    // Run Rust orchestrator
    let pipeline = ModularPipeline::new();
    let rust_output = pipeline.process_stages_4_to_8(stage3_clusters, ocr_cells);

    // Load Python output
    let python_output = load_python_stage08_output("arxiv_2206.01062", 0)
        .expect("Python baseline should exist for arxiv page 0");

    // Compare
    compare_clusters(&rust_output, &python_output, "arxiv_2206.01062", 0);
}

#[test]
fn test_orchestrator_code_and_formula_page0() {
    // Load inputs
    let stage3_clusters = load_stage3_clusters("code_and_formula", 0);
    let ocr_cells = load_ocr_cells("code_and_formula", 0);

    // Run Rust orchestrator
    let pipeline = ModularPipeline::new();
    let rust_output = pipeline.process_stages_4_to_8(stage3_clusters, ocr_cells);

    // Load Python output
    let python_output = load_python_stage08_output("code_and_formula", 0)
        .expect("Python baseline should exist for code_and_formula page 0");

    // Compare
    compare_clusters(&rust_output, &python_output, "code_and_formula", 0);
}

#[test]
fn test_orchestrator_all_pages() {
    let test_cases = vec![
        // arxiv (9 pages, has modular baselines)
        ("arxiv_2206.01062", 0),
        ("arxiv_2206.01062", 1),
        ("arxiv_2206.01062", 2),
        ("arxiv_2206.01062", 3),
        ("arxiv_2206.01062", 4),
        ("arxiv_2206.01062", 5),
        ("arxiv_2206.01062", 6),
        ("arxiv_2206.01062", 7),
        ("arxiv_2206.01062", 8),
        // code_and_formula (2 pages, has modular baselines)
        ("code_and_formula", 0),
        ("code_and_formula", 1),
        // jfk (15 pages, has modular baselines)
        ("jfk_scanned", 0),
        ("jfk_scanned", 1),
        ("jfk_scanned", 2),
        ("jfk_scanned", 3),
        ("jfk_scanned", 4),
        ("jfk_scanned", 5),
        ("jfk_scanned", 6),
        ("jfk_scanned", 7),
        ("jfk_scanned", 8),
        ("jfk_scanned", 9),
        ("jfk_scanned", 10),
        ("jfk_scanned", 11),
        ("jfk_scanned", 12),
        ("jfk_scanned", 13),
        ("jfk_scanned", 14),
    ];

    let mut passed = 0;
    let mut failed = Vec::new();

    for (pdf_name, page_num) in test_cases.iter() {
        // Load inputs
        let stage3_clusters = load_stage3_clusters(pdf_name, *page_num);
        let ocr_cells = load_ocr_cells(pdf_name, *page_num);

        // Load Python output (skip if not available)
        let python_output = match load_python_stage08_output(pdf_name, *page_num) {
            Some(output) => output,
            None => {
                println!("⏭️  {pdf_name} page {page_num}: SKIP (no modular baseline)");
                continue;
            }
        };

        // Run Rust orchestrator
        let pipeline = ModularPipeline::new();
        let rust_output = pipeline.process_stages_4_to_8(stage3_clusters, ocr_cells);

        // Compare
        match std::panic::catch_unwind(|| {
            compare_clusters(&rust_output, &python_output, pdf_name, *page_num);
        }) {
            Ok(_) => passed += 1,
            Err(e) => {
                let error_msg = if let Some(s) = e.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = e.downcast_ref::<&str>() {
                    s.to_string()
                } else {
                    "Unknown error".to_string()
                };
                failed.push(format!("{pdf_name} page {page_num}: {error_msg}"));
            }
        }
    }

    println!("\n=== Orchestrator Integration Test Summary ===");
    println!("Total: {} pages", passed + failed.len());
    println!(
        "Passed: {} ({:.1}%)",
        passed,
        100.0 * passed as f64 / (passed + failed.len()) as f64
    );
    println!(
        "Failed: {} ({:.1}%)",
        failed.len(),
        100.0 * failed.len() as f64 / (passed + failed.len()) as f64
    );

    if !failed.is_empty() {
        println!("\nFailures:");
        for failure in &failed {
            println!("  - {failure}");
        }
        panic!(
            "\nOrchestrator validation failed: {}/{} pages",
            failed.len(),
            passed + failed.len()
        );
    }
}

// REMOVED N=88: test_end_to_end_code_and_formula_page0
// Reason: Redundant with test_orchestrator_code_and_formula_page0 (which passed)
// - Both test stages 4-9 on code_and_formula page 0
// - test_end_to_end used old baseline structure (baseline_data/.../stage9_assembled.json)
// - test_orchestrator uses current modular baseline structure (baseline_data_modular/.../stage08_resolved_final.json)
// - Modular orchestrator test is the authoritative validation approach
