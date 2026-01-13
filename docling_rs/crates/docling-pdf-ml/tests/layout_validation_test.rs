#![cfg(feature = "pytorch")]
mod common;
use common::baseline_loaders::{load_json, load_numpy_u8};
use docling_pdf_ml::baseline::LayoutCluster;
use docling_pdf_ml::models::layout_predictor::LayoutPredictorModel;
use std::path::PathBuf;
use tch::Device;

#[test]
fn test_layout_predictor_matches_baseline() {
    // Load ONNX model
    let model_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("onnx_exports/layout_predictor_rtdetr.onnx");

    if !model_path.exists() {
        println!("Skipping test - ONNX model not found at {:?}", model_path);
        return;
    }

    let mut model = LayoutPredictorModel::load(&model_path, Device::Cpu)
        .expect("Failed to load LayoutPredictor model");

    // Load baseline input image
    let input_image_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("baseline_data/arxiv_2206.01062/page_0/layout/input_page_image.npy");

    if !input_image_path.exists() {
        println!(
            "Skipping test - input image not found at {:?}",
            input_image_path
        );
        return;
    }

    let input_image_dyn = load_numpy_u8(&input_image_path).expect("Failed to load input image");
    let input_image = input_image_dyn
        .into_dimensionality::<ndarray::Ix3>()
        .expect("Failed to convert to 3D array");

    println!("Input image shape: {:?}", input_image.shape());

    // Load baseline expected output (ONNX Runtime output from Python, not PyTorch!)
    let expected_output_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("baseline_data/arxiv_2206.01062/page_0/layout/output_onnx_clusters.json");

    let mut expected_clusters: Vec<LayoutCluster> =
        load_json(&expected_output_path).expect("Failed to load expected output");

    // Assign IDs to expected clusters (they don't have IDs in the JSON)
    for (i, cluster) in expected_clusters.iter_mut().enumerate() {
        cluster.id = i as i32;
    }

    println!(
        "Expected {} clusters from baseline (ONNX Runtime output)",
        expected_clusters.len()
    );

    // Run Rust inference
    let actual_clusters = model.infer(&input_image).expect("Failed to run inference");

    println!("Got {} clusters from Rust inference", actual_clusters.len());

    // Compare results
    compare_clusters(&expected_clusters, &actual_clusters);
}

fn compare_clusters(expected: &[LayoutCluster], actual: &[LayoutCluster]) {
    println!("\n=== Comparing Clusters (ONNX vs ONNX) ===");
    println!("Expected: {} clusters", expected.len());
    println!("Actual:   {} clusters", actual.len());

    // Check cluster count - should be exact match for ONNX vs ONNX
    // (comparing Python ONNX Runtime vs Rust ONNX Runtime)
    let count_match_percentage = (actual.len() as f64 / expected.len() as f64) * 100.0;
    println!("Match percentage: {:.1}%", count_match_percentage);

    // Require 95% match rate (tighter than 85% since both use ONNX Runtime)
    const MIN_MATCH_PERCENTAGE: f64 = 95.0;

    if count_match_percentage < MIN_MATCH_PERCENTAGE {
        println!("\n❌ MISMATCH: Cluster count differs significantly!");
        println!(
            "   Expected {} clusters, got {} ({:.1}% match)",
            expected.len(),
            actual.len(),
            count_match_percentage
        );
        println!("   Required: {:.1}% match", MIN_MATCH_PERCENTAGE);

        // Print all expected clusters
        println!("\nExpected clusters:");
        for cluster in expected {
            println!(
                "  [{:2}] {:20} conf={:.6} bbox=({:.2}, {:.2}, {:.2}, {:.2})",
                cluster.id,
                cluster.label,
                cluster.confidence,
                cluster.bbox.l,
                cluster.bbox.t,
                cluster.bbox.r,
                cluster.bbox.b
            );
        }

        // Print all actual clusters
        println!("\nActual clusters:");
        for cluster in actual {
            println!(
                "  [{:2}] {:20} conf={:.6} bbox=({:.2}, {:.2}, {:.2}, {:.2})",
                cluster.id,
                cluster.label,
                cluster.confidence,
                cluster.bbox.l,
                cluster.bbox.t,
                cluster.bbox.r,
                cluster.bbox.b
            );
        }

        panic!(
            "Cluster count match {:.1}% < required {:.1}%",
            count_match_percentage, MIN_MATCH_PERCENTAGE
        );
    }

    if expected.len() != actual.len() {
        println!(
            "\n⚠️  NOTE: Cluster counts differ by {} (borderline cases near threshold)",
            (expected.len() as i32 - actual.len() as i32).abs()
        );
    }

    // Match clusters by bbox similarity (IoU > 0.9) rather than strict ordering
    // This handles cases where FP precision causes slightly different confidence ordering
    let bbox_tolerance = 1.5; // pixels (allow FP differences between Python/Rust ONNX Runtime)
    let confidence_tolerance = 0.25; // relative error (25% - Python/Rust ONNX Runtime may differ)
    let iou_threshold = 0.9; // IoU threshold for matching clusters

    let mut matched_actual = vec![false; actual.len()];
    let mut mismatches = Vec::new();
    let mut unmatched_expected = Vec::new();

    for (exp_idx, exp) in expected.iter().enumerate() {
        // Find best matching actual cluster by bbox IoU
        let mut best_match_idx = None;
        let mut best_iou = 0.0;

        for (act_idx, act) in actual.iter().enumerate() {
            if matched_actual[act_idx] {
                continue; // Already matched
            }

            // Compute IoU
            let intersect_l = exp.bbox.l.max(act.bbox.l);
            let intersect_t = exp.bbox.t.max(act.bbox.t);
            let intersect_r = exp.bbox.r.min(act.bbox.r);
            let intersect_b = exp.bbox.b.min(act.bbox.b);

            if intersect_r > intersect_l && intersect_b > intersect_t {
                let intersect_area = (intersect_r - intersect_l) * (intersect_b - intersect_t);
                let exp_area = (exp.bbox.r - exp.bbox.l) * (exp.bbox.b - exp.bbox.t);
                let act_area = (act.bbox.r - act.bbox.l) * (act.bbox.b - act.bbox.t);
                let union_area = exp_area + act_area - intersect_area;
                let iou = intersect_area / union_area;

                if iou > best_iou {
                    best_iou = iou;
                    best_match_idx = Some(act_idx);
                }
            }
        }

        if let Some(act_idx) = best_match_idx {
            if best_iou >= iou_threshold {
                matched_actual[act_idx] = true;
                let act = &actual[act_idx];

                // Compare matched clusters
                let mut cluster_issues = Vec::new();

                // Compare label
                if exp.label != act.label {
                    cluster_issues.push(format!("Label: '{}' != '{}'", exp.label, act.label));
                }

                // Compare confidence (relative error)
                let conf_rel_error =
                    (exp.confidence - act.confidence).abs() / exp.confidence.max(1e-10);
                if conf_rel_error > confidence_tolerance {
                    cluster_issues.push(format!(
                        "Confidence: {:.6} != {:.6} (rel_error={:.6})",
                        exp.confidence, act.confidence, conf_rel_error
                    ));
                }

                // Compare bounding box coordinates
                let bbox_diffs = [
                    ("l", exp.bbox.l, act.bbox.l),
                    ("t", exp.bbox.t, act.bbox.t),
                    ("r", exp.bbox.r, act.bbox.r),
                    ("b", exp.bbox.b, act.bbox.b),
                ];

                for (coord_name, exp_val, act_val) in &bbox_diffs {
                    let diff = (exp_val - act_val).abs();
                    if diff > bbox_tolerance {
                        cluster_issues.push(format!(
                            "BBox.{}: {:.2} != {:.2} (diff={:.4})",
                            coord_name, exp_val, act_val, diff
                        ));
                    }
                }

                if !cluster_issues.is_empty() {
                    mismatches.push((exp_idx, exp, act, cluster_issues));
                }
            } else {
                unmatched_expected.push((exp_idx, exp, best_iou));
            }
        } else {
            unmatched_expected.push((exp_idx, exp, 0.0));
        }
    }

    let unmatched_actual: Vec<_> = actual
        .iter()
        .enumerate()
        .filter(|(i, _)| !matched_actual[*i])
        .collect();

    // Report results
    let matched_count = expected.len() - unmatched_expected.len();
    let match_rate = (matched_count as f64 / expected.len() as f64) * 100.0;

    println!("\n=== Matching Results ===");
    println!(
        "Matched: {}/{} clusters ({:.1}%)",
        matched_count,
        expected.len(),
        match_rate
    );

    if !unmatched_expected.is_empty() {
        println!(
            "\n⚠️  {} expected clusters unmatched:",
            unmatched_expected.len()
        );
        for (idx, cluster, iou) in &unmatched_expected {
            println!(
                "  [{}] {} conf={:.6} bbox=({:.2}, {:.2}, {:.2}, {:.2}) best_iou={:.3}",
                idx,
                cluster.label,
                cluster.confidence,
                cluster.bbox.l,
                cluster.bbox.t,
                cluster.bbox.r,
                cluster.bbox.b,
                iou
            );
        }
    }

    if !unmatched_actual.is_empty() {
        println!(
            "\n⚠️  {} actual clusters unmatched:",
            unmatched_actual.len()
        );
        for (idx, cluster) in &unmatched_actual {
            println!(
                "  [{}] {} conf={:.6} bbox=({:.2}, {:.2}, {:.2}, {:.2})",
                idx,
                cluster.label,
                cluster.confidence,
                cluster.bbox.l,
                cluster.bbox.t,
                cluster.bbox.r,
                cluster.bbox.b
            );
        }
    }

    // Success criteria: >= 95% match rate and no mismatches in matched clusters
    const MIN_MATCH_RATE: f64 = 95.0;

    if mismatches.is_empty() && match_rate >= MIN_MATCH_RATE {
        println!(
            "\n✅ SUCCESS: {} clusters matched within tolerance!",
            matched_count
        );
        println!(
            "   - Match rate: {:.1}% (required: {:.1}%)",
            match_rate, MIN_MATCH_RATE
        );
        println!("   - IoU threshold: {:.2}", iou_threshold);
        println!("   - Bbox tolerance: {} pixels", bbox_tolerance);
        println!(
            "   - Confidence tolerance: {} relative error",
            confidence_tolerance
        );
        if !unmatched_expected.is_empty() {
            println!(
                "   - Note: {} expected clusters unmatched (borderline cases near threshold)",
                unmatched_expected.len()
            );
        }
        if !unmatched_actual.is_empty() {
            println!(
                "   - Note: {} extra clusters in Rust output (borderline cases)",
                unmatched_actual.len()
            );
        }
    } else {
        if match_rate < MIN_MATCH_RATE {
            println!(
                "\n❌ FAIL: Match rate {:.1}% < required {:.1}%",
                match_rate, MIN_MATCH_RATE
            );
        }
        if !mismatches.is_empty() {
            println!("\n❌ MISMATCHES in {} matched clusters:", mismatches.len());
            for (idx, exp, act, issues) in &mismatches {
                println!("\nExpected[{}]:", idx);
                println!(
                    "  Expected: {:20} conf={:.6} bbox=({:.2}, {:.2}, {:.2}, {:.2})",
                    exp.label, exp.confidence, exp.bbox.l, exp.bbox.t, exp.bbox.r, exp.bbox.b
                );
                println!(
                    "  Actual:   {:20} conf={:.6} bbox=({:.2}, {:.2}, {:.2}, {:.2})",
                    act.label, act.confidence, act.bbox.l, act.bbox.t, act.bbox.r, act.bbox.b
                );
                println!("  Issues:");
                for issue in issues {
                    println!("    - {}", issue);
                }
            }
        }
        panic!("Validation failed: match_rate={:.1}% (need {:.1}%), {} mismatches, {} unmatched expected",
            match_rate, MIN_MATCH_RATE, mismatches.len(), unmatched_expected.len());
    }
}
