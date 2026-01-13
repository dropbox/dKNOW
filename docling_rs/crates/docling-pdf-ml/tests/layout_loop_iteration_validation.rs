use serde::Deserialize;
/// Loop Iteration Validation: Stages 7-8 Per-Iteration Testing
///
/// This test validates that the Stages 7-8 loop produces identical outputs at EACH
/// iteration, not just the final converged state.
///
/// Loop structure:
/// ```
/// for iteration in 0..3:
///     if converged: break
///     Stage 7: Adjust bboxes  → save stage7_iteration{N}_adjusted.json
///     Stage 8: Resolve overlaps → save stage8_iteration{N}_resolved.json
/// ```
///
/// Test strategy:
/// 1. Load Stage 6 output (input to loop)
/// 2. Run Rust ModularPipeline (which executes the loop)
/// 3. Compare Rust's per-iteration outputs with Python's per-iteration baselines
/// 4. Validate convergence happens at same iteration
///
/// Baselines location: baseline_data_modular/{pdf}/page_{N}/
///   - stage7_iteration1_adjusted.json
///   - stage8_iteration1_resolved.json
///   - stage7_iteration2_adjusted.json (if needed)
///   - stage8_iteration2_resolved.json (if needed)
///   - loop_convergence_info.json
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
struct ConvergenceInfo {
    converged_at_iteration: usize,
    final_cluster_count: usize,
    converged_early: bool,
}

#[derive(Debug, Deserialize, Clone)]
struct BaselineCluster {
    id: usize,
    label: String,
    confidence: f64,
    bbox: BaselineBbox,
    #[serde(default)]
    cells: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize, Clone)]
struct BaselineBbox {
    l: f64,
    t: f64,
    r: f64,
    b: f64,
}

fn load_convergence_info(pdf_name: &str, page_no: usize) -> Option<ConvergenceInfo> {
    let path = PathBuf::from(format!(
        "baseline_data_modular/{pdf_name}/page_{page_no}/loop_convergence_info.json"
    ));

    if !path.exists() {
        return None;
    }

    let contents = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&contents).ok()
}

fn load_baseline_clusters(path: &Path) -> Vec<BaselineCluster> {
    let contents =
        fs::read_to_string(path).unwrap_or_else(|_| panic!("Failed to read {}", path.display()));
    serde_json::from_str(&contents)
        .unwrap_or_else(|e| panic!("Failed to parse {}: {}", path.display(), e))
}

#[test]
#[ignore = "Requires instrumented pipeline"]
fn test_loop_iteration_validation_arxiv_page_0() {
    println!("\n{}", "=".repeat(80));
    println!("Loop Iteration Validation: arxiv_2206.01062 page 0");
    println!("{}", "=".repeat(80));

    let pdf_name = "arxiv_2206.01062";
    let page_no = 0;

    // Load convergence info
    let conv_info = load_convergence_info(pdf_name, page_no).expect("Convergence info not found");

    println!("\n  Python loop behavior:");
    println!(
        "    Converged at: iteration {}",
        conv_info.converged_at_iteration
    );
    println!(
        "    Final count: {} clusters",
        conv_info.final_cluster_count
    );
    println!("    Early convergence: {}", conv_info.converged_early);

    // Check iteration files exist
    println!("\n  Checking baseline files:");
    for iteration in 1..=3 {
        let stage7_path = PathBuf::from(format!(
            "baseline_data_modular/{pdf_name}/page_{page_no}/stage7_iteration{iteration}_adjusted.json"
        ));

        let stage8_path = PathBuf::from(format!(
            "baseline_data_modular/{pdf_name}/page_{page_no}/stage8_iteration{iteration}_resolved.json"
        ));

        if stage7_path.exists() && stage8_path.exists() {
            let stage7_clusters = load_baseline_clusters(&stage7_path);
            let stage8_clusters = load_baseline_clusters(&stage8_path);
            println!(
                "    Iteration {}: ✅ Stage 7: {} clusters, Stage 8: {} clusters",
                iteration,
                stage7_clusters.len(),
                stage8_clusters.len()
            );
        } else if iteration <= conv_info.converged_at_iteration {
            println!("    Iteration {iteration}: ❌ Missing baseline files");
            panic!("Expected baseline files for iteration {iteration} but not found");
        } else {
            println!("    Iteration {iteration}: — Not executed (converged earlier)");
        }
    }

    println!(
        "\n  ⚠️  NOTE: Rust ModularPipeline needs instrumentation to save per-iteration outputs"
    );
    println!("     Current implementation only exposes final output via process_stages_4_to_8()");
    println!("     Need to add:");
    println!("       - pub fn process_stages_4_to_8_with_iteration_trace()");
    println!("       - Returns: Vec<(iteration_no, stage7_output, stage8_output)>");
    println!("\n  Once instrumented, this test will:");
    println!("    1. Run Rust ModularPipeline with iteration tracing");
    println!("    2. Compare each iteration's Stage 7 output with Python baseline");
    println!("    3. Compare each iteration's Stage 8 output with Python baseline");
    println!("    4. Verify Rust converges at same iteration as Python");
}

#[test]
fn test_baseline_files_exist() {
    println!("\n{}", "=".repeat(80));
    println!("Checking Per-Iteration Baseline Availability");
    println!("{}", "=".repeat(80));

    let test_pages = vec![
        ("arxiv_2206.01062", vec![0, 1, 2, 3, 4, 5, 6, 7, 8]),
        ("code_and_formula", vec![0, 1]),
    ];

    let mut total_pages = 0;
    let mut pages_with_baselines = 0;
    let mut total_iterations = 0;

    for (pdf_name, pages) in test_pages {
        for page_no in pages {
            total_pages += 1;

            if let Some(conv_info) = load_convergence_info(pdf_name, page_no) {
                pages_with_baselines += 1;

                // Check how many iterations were executed
                let iterations_executed = conv_info.converged_at_iteration;
                total_iterations += iterations_executed;

                // Verify iteration files exist
                for iteration in 1..=iterations_executed {
                    let stage7_path = PathBuf::from(format!(
                        "baseline_data_modular/{pdf_name}/page_{page_no}/stage7_iteration{iteration}_adjusted.json"
                    ));

                    let stage8_path = PathBuf::from(format!(
                        "baseline_data_modular/{pdf_name}/page_{page_no}/stage8_iteration{iteration}_resolved.json"
                    ));

                    assert!(
                        stage7_path.exists(),
                        "Missing Stage 7 iteration {iteration} for {pdf_name}/page_{page_no}"
                    );

                    assert!(
                        stage8_path.exists(),
                        "Missing Stage 8 iteration {iteration} for {pdf_name}/page_{page_no}"
                    );
                }
            }
        }
    }

    println!("\n  Pages with baselines: {pages_with_baselines}/{total_pages}");
    println!("  Total iterations: {total_iterations}");
    println!(
        "  Average iterations per page: {:.2}",
        total_iterations as f32 / pages_with_baselines as f32
    );

    println!("\n  ✅ All per-iteration baseline files exist");
    println!("\n  Next: Instrument Rust ModularPipeline to save per-iteration outputs");
}
