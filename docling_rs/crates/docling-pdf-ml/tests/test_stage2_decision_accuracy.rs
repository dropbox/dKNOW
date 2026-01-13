mod common;

/// Stage 2 Decision Accuracy - Validate Decisions, Not Just Precision
///
/// Tests that catch amplification problems:
/// - Small errors near thresholds → wrong decisions
/// - Label scores differ by 0.02 → wrong label assigned
/// - NMS scores differ by 0.01 → different boxes kept
///
/// SUCCESS = Edit distance 0 (all decisions match Python)

// Function exists but test body is commented out pending ML pipeline implementation
#[allow(unused_imports)]
use common::baseline_loaders::load_layout_clusters_with_cells;
use std::collections::HashMap;

/// Test: Final cluster decisions match Python exactly
#[test]
#[ignore = "Requires baseline_data and Rust ML pipeline implementation"]
fn test_decision_final_clusters_arxiv_page0() {
    println!("\n{}", "=".repeat(80));
    println!("DECISION ACCURACY TEST: Final Clusters");
    println!("{}", "=".repeat(80));

    // Load Python baseline clusters
    // let python_clusters = load_layout_clusters_with_cells("arxiv_2206.01062", 0)
    //     .expect("Failed to load Python clusters");

    println!("\n✓ Python baseline:");
    // println!("   Clusters: {}", python_clusters.len());

    // Count by label
    // let mut python_labels: HashMap<String, usize> = HashMap::new();
    // for cluster in &python_clusters {
    //     *python_labels.entry(format!("{:?}", cluster.label)).or_default() += 1;
    // }

    // println!("   By label:");
    // for (label, count) in python_labels.iter() {
    //     println!("     {}: {}", label, count);
    // }

    // TODO: Run Rust ML pipeline, get final clusters
    // let rust_clusters = run_rust_ml_pipeline("arxiv_2206.01062", 0);

    println!("\n✗ Rust ML pipeline: NOT IMPLEMENTED YET");
    println!("   TODO: Run complete ML → get final clusters");

    // When implemented, validate:
    println!("\n📋 Decision Accuracy Checks (TODO):");
    println!("   1. Cluster count: Rust == Python (exact)");
    println!("   2. Label distribution: Same count of each label");
    println!("   3. Cluster matching: Each Rust cluster has Python match");
    println!("   4. Edit distance: 0 insertions/deletions/substitutions");

    println!("\n⚠️  Currently this test just documents requirements");
    println!("   Worker: Implement Rust ML pipeline integration");
}

/// Helper: Calculate edit distance for label sequences
fn calculate_label_edit_distance(rust_labels: &[String], python_labels: &[String]) -> usize {
    // Wagner-Fischer algorithm (dynamic programming)
    let m = rust_labels.len();
    let n = python_labels.len();

    let mut dp = vec![vec![0usize; n + 1]; m + 1];

    // Initialize
    for (i, row) in dp.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, val) in dp[0].iter_mut().enumerate() {
        *val = j;
    }

    // Fill matrix
    for i in 1..=m {
        for j in 1..=n {
            let cost = if rust_labels[i - 1] == python_labels[j - 1] {
                0
            } else {
                1
            };
            dp[i][j] = std::cmp::min(
                std::cmp::min(
                    dp[i - 1][j] + 1, // Deletion
                    dp[i][j - 1] + 1, // Insertion
                ),
                dp[i - 1][j - 1] + cost, // Substitution
            );
        }
    }

    dp[m][n]
}

#[test]
fn test_edit_distance_helper() {
    // Test the edit distance function
    let python = vec![
        "text".to_string(),
        "text".to_string(),
        "picture".to_string(),
    ];
    let rust = vec![
        "text".to_string(),
        "picture".to_string(),
        "text".to_string(),
    ];

    let distance = calculate_label_edit_distance(&rust, &python);
    println!("\nEdit distance test:");
    println!("  Python: {python:?}");
    println!("  Rust:   {rust:?}");
    println!("  Edit distance: {distance}");

    assert_eq!(distance, 2, "Should be 2 operations to transform");
}

/// Example: How decision test SHOULD work (template for worker)
#[test]
#[ignore = "Template test - not yet implemented"]
fn test_decision_accuracy_template() {
    println!("\n{}", "=".repeat(80));
    println!("DECISION ACCURACY TEST TEMPLATE");
    println!("{}", "=".repeat(80));

    // 1. Load Python final clusters
    // let python_clusters = load_layout_clusters_with_cells("arxiv_2206.01062", 0).unwrap();

    // let python_count = python_clusters.len();
    // let python_labels: Vec<String> = python_clusters
    //     .iter()
    //     .map(|c| format!("{:?}", c.label))
    //     .collect();

    let python_count = 26; // Simulated for now
    let python_labels: Vec<String> = vec![]; // Simulated

    println!("\n✓ Python: {python_count} clusters (simulated)");
    // println!("   Labels: {:?}", python_labels);

    // 2. Run Rust ML (TODO: implement this)
    // let rust_clusters = run_rust_ml_complete("arxiv_2206.01062", 0);
    // let rust_count = rust_clusters.len();
    // let rust_labels: Vec<String> = ...;

    // For now, simulate with example mismatch
    let rust_count = 27; // Simulated
    let rust_labels = ["text", "text", "picture", "picture", "picture", "text"] // 3 pictures vs 1
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    println!("\n✗ Rust: {rust_count} clusters (simulated)");
    println!("   Labels: {rust_labels:?}");

    // 3. Decision accuracy checks
    println!("\n📊 DECISION ACCURACY:");

    // 3a. Count match?
    let count_match = rust_count == python_count;
    println!(
        "   Cluster count: {} == {} ? {}",
        rust_count,
        python_count,
        if count_match { "✅" } else { "❌" }
    );

    // 3b. Label distribution
    let mut python_label_counts: HashMap<String, usize> = HashMap::new();
    for label in &python_labels {
        *python_label_counts.entry(label.clone()).or_default() += 1;
    }

    let mut rust_label_counts: HashMap<String, usize> = HashMap::new();
    for label in &rust_labels {
        *rust_label_counts.entry(label.clone()).or_default() += 1;
    }

    println!("\n   Label distribution:");
    for label in ["text", "picture", "table"] {
        let py_count = python_label_counts.get(label).unwrap_or(&0);
        let rust_count = rust_label_counts.get(label).unwrap_or(&0);
        let match_symbol = if py_count == rust_count { "✅" } else { "❌" };
        println!("     {label}: Python={py_count}, Rust={rust_count} {match_symbol}");
    }

    // 3c. Edit distance
    let edit_dist = calculate_label_edit_distance(&rust_labels, &python_labels);
    println!("\n   Label sequence edit distance: {edit_dist}");
    println!("     (0 = exact match, >0 = insertions/deletions/swaps)");

    // 3d. Verdict
    println!("\n{}", "=".repeat(80));
    if count_match && edit_dist == 0 {
        println!("✅ DECISION ACCURACY: PASS");
        println!("   All decisions match Python exactly");
    } else {
        println!("❌ DECISION ACCURACY: FAIL");
        println!("   Count diff: {:+}", rust_count - python_count);
        println!("   Edit distance: {edit_dist} operations needed");
        println!();
        println!("   This means:");
        println!("   - Numerical precision may be good (< 0.1% error)");
        println!("   - But decisions are WRONG (amplification!)");
        println!("   - Must fix thresholds, NMS, or classification logic");
    }
}
