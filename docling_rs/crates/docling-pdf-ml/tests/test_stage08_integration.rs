/// Integration tests for Stage 08 (Overlap Resolver)
///
/// ⚠️ WARNING: These standalone Stage 08 tests are DEPRECATED and misleading.
///
/// **Why these tests are problematic:**
///
/// Stage 08 (overlap resolution) runs in an **iterative loop** with Stage 07 (bbox adjustment).
/// The modular Python baseline `stage08_resolved_final.json` is generated **after the
/// iteration loop completes**, not after a single Stage 08 pass.
///
/// **The issue:**
/// Similar to Stage 07, the baselines are post-iteration outputs, making standalone
/// Stage 08 tests misleading. Stage 08's output depends on the iteration context.
///
/// **Correct testing approach:**
/// - Use `tests/test_orchestrator_integration.rs` which tests Stages 04-09 together
/// - Orchestrator test: ✅ PASSES 100% (26/26 pages)
/// - End-to-end test: ✅ PASSES
///
/// **Status:**
/// These tests are marked as #[ignore] to prevent false failures. They are kept
/// for historical reference only. Use the orchestrator test for validation.
///
/// Test structure (for reference only):
/// - Load Stage 8 input (stage8_adjusted.json)
/// - Run Rust Stage 08 overlap resolver
/// - Load Stage 8 output baseline (stage8_resolved.json)
/// - Compare outputs (cluster count, bboxes, labels, cells)
use docling_pdf_ml::pipeline_modular::stage08_overlap_resolver::Stage08OverlapResolver;
use docling_pdf_ml::pipeline_modular::types::{
    BBox, ClusterWithCells, ClustersWithCells, TextCell,
};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

/// Bbox comparison tolerance (pixels)
const BBOX_TOLERANCE: f64 = 0.01;

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

/// Load stage 8 input (adjusted clusters) from JSON
/// Supports both modular and old baseline formats, with fallback
fn load_stage8_input_from_path(path: PathBuf) -> ClustersWithCells {
    let json = fs::read_to_string(&path).unwrap_or_else(|_| panic!("Failed to read {path:?}"));

    let data: Value =
        serde_json::from_str(&json).unwrap_or_else(|e| panic!("Failed to parse {path:?}: {e}"));

    // Handle both formats: {"clusters": [...]} or [...]
    let clusters_array = if let Some(obj) = data.as_object() {
        obj.get("clusters")
            .expect("JSON object should have 'clusters' key")
            .as_array()
            .expect("'clusters' should be array")
    } else {
        data.as_array()
            .expect("JSON should be array or object with 'clusters' key")
    };

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

            // Handle both rect (old format) and bbox (modular format)
            let cell_bbox = if let Some(rect) = cell_data.get("rect") {
                parse_cell_rect(rect)
            } else if let Some(bbox_obj) = cell_data.get("bbox") {
                BBox::new(
                    bbox_obj["l"].as_f64().unwrap(),
                    bbox_obj["t"].as_f64().unwrap(),
                    bbox_obj["r"].as_f64().unwrap(),
                    bbox_obj["b"].as_f64().unwrap(),
                )
            } else {
                panic!("Cell should have 'rect' or 'bbox' field");
            };

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

    ClustersWithCells { clusters }
}

/// Load stage 8 output (resolved clusters) baseline from JSON
/// Reuses the same loader as input since format is identical
fn load_stage8_baseline_from_path(path: PathBuf) -> ClustersWithCells {
    load_stage8_input_from_path(path)
}

/// Deprecated: kept for compatibility, use load_stage8_baseline_from_path instead
fn _load_stage8_baseline_old(pdf_name: &str, page_num: usize) -> ClustersWithCells {
    let path = PathBuf::from(format!(
        "baseline_data_modular/{pdf_name}/page_{page_num}/stage08_resolved_final.json"
    ));

    let json = fs::read_to_string(&path).unwrap_or_else(|_| panic!("Failed to read {path:?}"));

    let data: Value =
        serde_json::from_str(&json).unwrap_or_else(|e| panic!("Failed to parse {path:?}: {e}"));

    // Handle both formats: {"clusters": [...]} or [...]
    let clusters_array = if let Some(obj) = data.as_object() {
        obj.get("clusters")
            .expect("JSON object should have 'clusters' key")
            .as_array()
            .expect("'clusters' should be array")
    } else {
        data.as_array()
            .expect("JSON should be array or object with 'clusters' key")
    };

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

            // Handle both rect (old format) and bbox (modular format)
            let cell_bbox = if let Some(rect) = cell_data.get("rect") {
                parse_cell_rect(rect)
            } else if let Some(bbox_obj) = cell_data.get("bbox") {
                BBox::new(
                    bbox_obj["l"].as_f64().unwrap(),
                    bbox_obj["t"].as_f64().unwrap(),
                    bbox_obj["r"].as_f64().unwrap(),
                    bbox_obj["b"].as_f64().unwrap(),
                )
            } else {
                panic!("Cell should have 'rect' or 'bbox' field");
            };

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

    ClustersWithCells { clusters }
}

/// Compare two bboxes with tolerance
fn bboxes_equal(a: &BBox, b: &BBox, tolerance: f64) -> bool {
    (a.l - b.l).abs() < tolerance
        && (a.t - b.t).abs() < tolerance
        && (a.r - b.r).abs() < tolerance
        && (a.b - b.b).abs() < tolerance
}

/// Calculate max bbox difference
fn max_bbox_diff(a: &BBox, b: &BBox) -> f64 {
    let diffs = [
        (a.l - b.l).abs(),
        (a.t - b.t).abs(),
        (a.r - b.r).abs(),
        (a.b - b.b).abs(),
    ];
    diffs.iter().copied().fold(f64::NEG_INFINITY, f64::max)
}

/// Test Stage 08 on a single page
fn test_stage08_page(pdf_name: &str, page_num: usize) -> Result<String, String> {
    // Determine which paths to use (prefer modular, fallback to old)
    let modular_base_path =
        PathBuf::from(format!("baseline_data_modular/{pdf_name}/page_{page_num}"));
    let old_base_path = PathBuf::from(format!("baseline_data/{pdf_name}/page_{page_num}"));

    // Input paths
    let stage7_modular_path = modular_base_path.join("stage07_adjusted_final.json");
    let stage8_old_input_path = old_base_path.join("layout/stage8_adjusted.json");

    // Baseline paths
    let stage8_modular_path = modular_base_path.join("stage08_resolved_final.json");
    let stage8_old_baseline_path = old_base_path.join("layout/stage8_resolved.json");

    // Determine which paths to use
    let input_path = if stage7_modular_path.exists() {
        stage7_modular_path
    } else if stage8_old_input_path.exists() {
        stage8_old_input_path
    } else {
        return Err(format!(
            "Missing input files for {pdf_name} page {page_num}"
        ));
    };

    let baseline_path = if stage8_modular_path.exists() {
        stage8_modular_path
    } else if stage8_old_baseline_path.exists() {
        stage8_old_baseline_path
    } else {
        return Err(format!(
            "Missing baseline files for {pdf_name} page {page_num}"
        ));
    };

    // Load Stage 8 input
    let input = load_stage8_input_from_path(input_path);

    // Run Rust Stage 08
    let resolver = Stage08OverlapResolver::new();
    let output = resolver.process(input);

    // Load Python baseline
    let baseline = load_stage8_baseline_from_path(baseline_path);

    // Compare cluster counts
    if output.clusters.len() != baseline.clusters.len() {
        return Err(format!(
            "Cluster count mismatch: Rust={}, Python={}",
            output.clusters.len(),
            baseline.clusters.len()
        ));
    }

    // Sort both outputs by ID for comparison (Python sorts by cell index, then position)
    let mut rust_clusters = output.clusters;
    let mut python_clusters = baseline.clusters;
    rust_clusters.sort_by_key(|c| c.id);
    python_clusters.sort_by_key(|c| c.id);

    // Compare each cluster
    let mut max_diff: f64 = 0.0;
    for (i, (rust_cluster, python_cluster)) in
        rust_clusters.iter().zip(python_clusters.iter()).enumerate()
    {
        // Compare ID
        if rust_cluster.id != python_cluster.id {
            return Err(format!(
                "Cluster {} ID mismatch: Rust={}, Python={}",
                i, rust_cluster.id, python_cluster.id
            ));
        }

        // Compare label
        if rust_cluster.label != python_cluster.label {
            return Err(format!(
                "Cluster {} label mismatch: Rust={}, Python={}",
                i, rust_cluster.label, python_cluster.label
            ));
        }

        // Compare bbox
        let diff = max_bbox_diff(&rust_cluster.bbox, &python_cluster.bbox);
        max_diff = max_diff.max(diff);

        if !bboxes_equal(&rust_cluster.bbox, &python_cluster.bbox, BBOX_TOLERANCE) {
            return Err(format!(
                "Cluster {} bbox mismatch (max diff: {:.6}): Rust={:?}, Python={:?}",
                i, diff, rust_cluster.bbox, python_cluster.bbox
            ));
        }

        // Compare cell count
        if rust_cluster.cells.len() != python_cluster.cells.len() {
            return Err(format!(
                "Cluster {} cell count mismatch: Rust={}, Python={}",
                i,
                rust_cluster.cells.len(),
                python_cluster.cells.len()
            ));
        }
    }

    Ok(format!(
        "✅ {} page {} PASS ({} clusters, max bbox diff: {:.6} px)",
        pdf_name,
        page_num,
        rust_clusters.len(),
        max_diff
    ))
}

// Individual page tests for quick debugging
// ⚠️ DEPRECATED: Use test_orchestrator_integration.rs instead

#[test]
#[ignore = "Deprecated: Stage 08 is part of iterative loop, test via orchestrator instead"]
fn test_stage08_arxiv_page0() {
    match test_stage08_page("arxiv_2206.01062", 0) {
        Ok(msg) => println!("{msg}"),
        Err(e) => panic!("{}", e),
    }
}

#[test]
#[ignore = "Deprecated: Stage 08 is part of iterative loop, test via orchestrator instead"]
fn test_stage08_code_and_formula_page0() {
    match test_stage08_page("code_and_formula", 0) {
        Ok(msg) => println!("{msg}"),
        Err(e) => panic!("{}", e),
    }
}

#[test]
#[ignore = "Deprecated: Stage 08 is part of iterative loop, test via orchestrator instead"]
fn test_stage08_edinet_page0() {
    match test_stage08_page("edinet_sample", 0) {
        Ok(msg) => println!("{msg}"),
        Err(e) => panic!("{}", e),
    }
}

#[test]
#[ignore = "Deprecated: Stage 08 is part of iterative loop, test via orchestrator instead"]
fn test_stage08_jfk_page0() {
    match test_stage08_page("jfk_scanned", 0) {
        Ok(msg) => println!("{msg}"),
        Err(e) => panic!("{}", e),
    }
}

// Comprehensive test for all pages
// ⚠️ DEPRECATED: Use test_orchestrator_integration.rs instead

#[test]
#[ignore = "Deprecated: Stage 08 is part of iterative loop, test via orchestrator instead"]
fn test_stage08_all_pages() {
    let test_cases = [
        // arxiv_2206.01062 (9 pages)
        ("arxiv_2206.01062", 0),
        ("arxiv_2206.01062", 1),
        ("arxiv_2206.01062", 2),
        ("arxiv_2206.01062", 3),
        ("arxiv_2206.01062", 4),
        ("arxiv_2206.01062", 5),
        ("arxiv_2206.01062", 6),
        ("arxiv_2206.01062", 7),
        ("arxiv_2206.01062", 8),
        // code_and_formula (2 pages)
        ("code_and_formula", 0),
        ("code_and_formula", 1),
        // edinet_sample (21 pages)
        ("edinet_sample", 0),
        ("edinet_sample", 1),
        ("edinet_sample", 2),
        ("edinet_sample", 3),
        ("edinet_sample", 4),
        ("edinet_sample", 5),
        ("edinet_sample", 6),
        ("edinet_sample", 7),
        ("edinet_sample", 8),
        ("edinet_sample", 9),
        ("edinet_sample", 10),
        ("edinet_sample", 11),
        ("edinet_sample", 12),
        ("edinet_sample", 13),
        ("edinet_sample", 14),
        ("edinet_sample", 15),
        ("edinet_sample", 16),
        ("edinet_sample", 17),
        ("edinet_sample", 18),
        ("edinet_sample", 19),
        ("edinet_sample", 20),
        // jfk_scanned (15 pages)
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
    let mut failed = 0;
    let mut failures = Vec::new();

    for (pdf_name, page_num) in test_cases.iter() {
        match test_stage08_page(pdf_name, *page_num) {
            Ok(msg) => {
                println!("{msg}");
                passed += 1;
            }
            Err(e) => {
                eprintln!("❌ {pdf_name} page {page_num} FAIL: {e}");
                failures.push(format!("{pdf_name} page {page_num}: {e}"));
                failed += 1;
            }
        }
    }

    println!("\n=== Stage 08 Integration Test Summary ===");
    println!("Total: {} pages", test_cases.len());
    println!(
        "Passed: {} ({:.1}%)",
        passed,
        100.0 * passed as f64 / test_cases.len() as f64
    );
    println!(
        "Failed: {} ({:.1}%)",
        failed,
        100.0 * failed as f64 / test_cases.len() as f64
    );

    if !failures.is_empty() {
        println!("\nFailures:");
        for failure in &failures {
            println!("  - {failure}");
        }
        panic!(
            "\nStage 08 validation failed: {}/{} pages",
            failed,
            test_cases.len()
        );
    }

    println!("\n✅ Stage 08 validated on all {} pages", test_cases.len());
}
