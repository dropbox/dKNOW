use docling_pdf_ml::pipeline_modular::{
    types::{BBox, LabeledCluster, LabeledClusters, OCRCells, TextCell},
    ModularPipeline,
};
/// Debug test for arxiv page 0 orchestrator pipeline
///
/// This test dumps all intermediate stage outputs to help debug
/// why Rust produces 27 clusters but Python produces 29.
use std::path::PathBuf;

/// Label mapping from class IDs to labels
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
        _ => format!("unknown_{}", class_id),
    }
}

#[test]
fn debug_arxiv_page0_stages() {
    // Load Stage 3 clusters
    let stage3_path =
        PathBuf::from("baseline_data/arxiv_2206.01062/page_0/layout/stage3_hf_postprocessed.json");
    let content = std::fs::read_to_string(&stage3_path).unwrap();
    let data: serde_json::Value = serde_json::from_str(&content).unwrap();

    let scores = data["scores"].as_array().unwrap();
    let labels = data["labels"].as_array().unwrap();
    let boxes = data["boxes"].as_array().unwrap();

    let mut clusters = Vec::new();
    for (idx, ((score, label_id), bbox_arr)) in scores
        .iter()
        .zip(labels.iter())
        .zip(boxes.iter())
        .enumerate()
    {
        let score_val = score.as_f64().unwrap();
        let label_id_val = label_id.as_i64().unwrap() as i32;
        let bbox_vals = bbox_arr.as_array().unwrap();

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

    let stage3_clusters = LabeledClusters { clusters };
    println!(
        "Stage 3 (input): {} clusters",
        stage3_clusters.clusters.len()
    );

    // Load OCR cells
    let cells_path =
        PathBuf::from("baseline_data/arxiv_2206.01062/page_0/preprocessing/textline_cells.json");
    let content = std::fs::read_to_string(&cells_path).unwrap();
    let data: serde_json::Value = serde_json::from_str(&content).unwrap();
    let cells_array = data.as_array().unwrap();

    let mut cells = Vec::new();
    for cell_data in cells_array {
        let text = cell_data["text"].as_str().unwrap().to_string();
        let rect = &cell_data["rect"];
        let bbox = BBox::new(
            rect["l"].as_f64().unwrap(),
            rect["t"].as_f64().unwrap(),
            rect["r"].as_f64().unwrap(),
            rect["b"].as_f64().unwrap(),
        );
        let confidence = cell_data.get("confidence").and_then(|v| v.as_f64());

        cells.push(TextCell {
            text,
            bbox,
            confidence,
        });
    }

    let ocr_cells = OCRCells { cells };
    println!("OCR cells: {}", ocr_cells.cells.len());

    // Manually run pipeline stages to see counts
    use docling_pdf_ml::pipeline_modular::{
        Stage04CellAssigner, Stage05EmptyRemover, Stage06OrphanCreator,
    };

    let stage04 = Stage04CellAssigner::new();
    let stage05 = Stage05EmptyRemover::new();
    let stage06 = Stage06OrphanCreator::new();

    // Stage 4
    let stage4_result = stage04.process(stage3_clusters, ocr_cells.clone());
    let stage4_with_cells = stage4_result
        .clusters
        .iter()
        .filter(|c| !c.cells.is_empty())
        .count();
    println!(
        "Stage 4 (cell assignment): {} clusters ({} with cells, {} empty)",
        stage4_result.clusters.len(),
        stage4_with_cells,
        stage4_result.clusters.len() - stage4_with_cells
    );

    // Stage 5
    let stage5_result = stage05.process(stage4_result);
    let stage5_with_cells = stage5_result
        .clusters
        .iter()
        .filter(|c| !c.cells.is_empty())
        .count();
    println!(
        "Stage 5 (empty removal):   {} clusters ({} with cells, {} empty special)",
        stage5_result.clusters.len(),
        stage5_with_cells,
        stage5_result.clusters.len() - stage5_with_cells
    );

    // Stage 6
    let stage6_result = stage06.process(stage5_result, ocr_cells.clone());
    let stage6_with_cells = stage6_result
        .clusters
        .iter()
        .filter(|c| !c.cells.is_empty())
        .count();
    println!(
        "Stage 6 (orphans):         {} clusters ({} with cells, {} empty)",
        stage6_result.clusters.len(),
        stage6_with_cells,
        stage6_result.clusters.len() - stage6_with_cells
    );

    // Now run full pipeline for final result
    let debug_dir = PathBuf::from("temp_debug_arxiv_page0");
    std::fs::create_dir_all(&debug_dir).unwrap();

    let pipeline = ModularPipeline::with_debug_output(debug_dir.clone());

    // Reload inputs (pipeline took ownership)
    let stage3_path =
        PathBuf::from("baseline_data/arxiv_2206.01062/page_0/layout/stage3_hf_postprocessed.json");
    let content = std::fs::read_to_string(&stage3_path).unwrap();
    let data: serde_json::Value = serde_json::from_str(&content).unwrap();
    let scores = data["scores"].as_array().unwrap();
    let labels = data["labels"].as_array().unwrap();
    let boxes = data["boxes"].as_array().unwrap();
    let mut clusters = Vec::new();
    for (idx, ((score, label_id), bbox_arr)) in scores
        .iter()
        .zip(labels.iter())
        .zip(boxes.iter())
        .enumerate()
    {
        let score_val = score.as_f64().unwrap();
        let label_id_val = label_id.as_i64().unwrap() as i32;
        let bbox_vals = bbox_arr.as_array().unwrap();
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
    let stage3_clusters = LabeledClusters { clusters };

    let cells_path =
        PathBuf::from("baseline_data/arxiv_2206.01062/page_0/preprocessing/textline_cells.json");
    let content = std::fs::read_to_string(&cells_path).unwrap();
    let data: serde_json::Value = serde_json::from_str(&content).unwrap();
    let cells_array = data.as_array().unwrap();
    let mut cells = Vec::new();
    for cell_data in cells_array {
        let text = cell_data["text"].as_str().unwrap().to_string();
        let rect = &cell_data["rect"];
        let bbox = BBox::new(
            rect["l"].as_f64().unwrap(),
            rect["t"].as_f64().unwrap(),
            rect["r"].as_f64().unwrap(),
            rect["b"].as_f64().unwrap(),
        );
        let confidence = cell_data.get("confidence").and_then(|v| v.as_f64());
        cells.push(TextCell {
            text,
            bbox,
            confidence,
        });
    }
    let ocr_cells = OCRCells { cells };

    let result = pipeline.process_stages_4_to_8(stage3_clusters, ocr_cells);

    println!("\nRust output: {} clusters", result.clusters.len());
    println!("Python output: 29 clusters");
    println!("Difference: {} clusters", 29 - result.clusters.len() as i32);

    println!("\nIntermediate outputs saved to: {}", debug_dir.display());
    println!("Compare with Python outputs in: baseline_data_modular/arxiv_2206.01062/page_0/");

    // Print cluster IDs and labels for comparison
    println!("\nRust cluster IDs:");
    for (i, c) in result.clusters.iter().enumerate() {
        println!(
            "  [{:2}] ID {:2}: {:20} @ ({:.1}, {:.1}, {:.1}, {:.1}) - {} cells",
            i,
            c.id,
            c.label,
            c.bbox.l,
            c.bbox.t,
            c.bbox.r,
            c.bbox.b,
            c.cells.len()
        );
    }

    // Don't fail the test, just report
    println!(
        "\n⚠️  Cluster count mismatch: Rust={}, Python=29",
        result.clusters.len()
    );
}
