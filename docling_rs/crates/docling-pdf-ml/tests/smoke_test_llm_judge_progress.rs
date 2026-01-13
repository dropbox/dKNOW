/// Smoke Test: LLM Judge Progress Tracker
///
/// This test tracks progress toward 100% correctness using LLM judge validation.
/// It is NON-BLOCKING (always passes) but reports current state and what needs fixing.
///
/// SUCCESS CRITERIA (to reach 100%):
/// - Test Coverage: 42/42 pages judged (not just 11)
/// - LLM Pass Rate: 42/42 pages score >= 95
/// - Content Complete: All missing content fixed
///
/// This test reminds workers what work remains.
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// LLM Judge result structure
#[derive(Debug)]
struct JudgeResult {
    pdf: String,
    page: usize,
    score: f32,
    verdict: String,
    missing: Vec<String>,
}

/// Load all judge results from judge_results/*.json
fn load_judge_results() -> Vec<JudgeResult> {
    let mut results = Vec::new();

    let judge_dir = Path::new("judge_results");
    if !judge_dir.exists() {
        return results;
    }

    if let Ok(entries) = fs::read_dir(judge_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                        // Parse filename: {pdf}_page_{N}.json
                        let filename = path.file_stem().unwrap().to_str().unwrap();
                        let parts: Vec<&str> = filename.split("_page_").collect();

                        if parts.len() == 2 {
                            let pdf = parts[0].to_string();
                            let page = parts[1].parse::<usize>().unwrap_or(0);
                            let score = json["score"].as_f64().unwrap_or(0.0) as f32;
                            let verdict = json["verdict"].as_str().unwrap_or("UNKNOWN").to_string();

                            let missing = json["missing_sections"]
                                .as_array()
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|v| v.as_str())
                                        .map(|s| s.to_string())
                                        .collect()
                                })
                                .unwrap_or_default();

                            results.push(JudgeResult {
                                pdf,
                                page,
                                score,
                                verdict,
                                missing,
                            });
                        }
                    }
                }
            }
        }
    }

    results.sort_by_key(|r| (r.pdf.clone(), r.page));
    results
}

#[test]
fn smoke_test_llm_judge_progress() {
    println!("\n{}", "=".repeat(80));
    println!("LLM JUDGE PROGRESS TRACKER (Non-Blocking Smoke Test)");
    println!("{}", "=".repeat(80));

    let results = load_judge_results();

    if results.is_empty() {
        println!("\n⚠️  NO LLM JUDGE RESULTS FOUND");
        println!("Run: python3 judge_all_pages.py");
        println!("\nTest: PASS (non-blocking)\n");
        return;
    }

    // Group by PDF
    let mut by_pdf: HashMap<String, Vec<&JudgeResult>> = HashMap::new();
    for result in &results {
        by_pdf.entry(result.pdf.clone()).or_default().push(result);
    }

    // Expected pages per PDF
    let expected_pages: HashMap<&str, usize> = [
        ("arxiv_2206.01062", 9),
        ("code_and_formula", 2),
        ("edinet_sample", 21),
        ("jfk_scanned", 10),
    ]
    .iter()
    .copied()
    .collect();

    println!("\n📊 PROGRESS GRID");
    println!("{}", "=".repeat(80));

    let mut total_tested = 0;
    let mut total_expected = 0;
    let mut total_passing = 0;
    let mut sum_scores = 0.0;

    for (pdf_name, expected_count) in expected_pages.iter() {
        let tested = by_pdf.get(*pdf_name).map(|v| v.len()).unwrap_or(0);
        let passing = by_pdf
            .get(*pdf_name)
            .map(|v| v.iter().filter(|r| r.verdict == "PASS").count())
            .unwrap_or(0);

        let avg_score = by_pdf
            .get(*pdf_name)
            .map(|v| {
                let sum: f32 = v.iter().map(|r| r.score).sum();
                if v.is_empty() {
                    0.0
                } else {
                    sum / v.len() as f32
                }
            })
            .unwrap_or(0.0);

        let coverage_pct = (tested as f32 / *expected_count as f32 * 100.0) as usize;
        let pass_pct = if tested > 0 {
            (passing as f32 / tested as f32 * 100.0) as usize
        } else {
            0
        };

        let status = if tested == *expected_count && passing == tested {
            "✅"
        } else if tested == 0 {
            "❌"
        } else {
            "⚠️ "
        };

        println!(
            "{status} {pdf_name:25} {tested:2}/{expected_count:2} tested ({coverage_pct:3}%)  |  {passing:2}/{tested:2} pass ({pass_pct:3}%)  |  Avg: {avg_score:5.1}/100"
        );

        total_tested += tested;
        total_expected += expected_count;
        total_passing += passing;
        sum_scores += avg_score * tested as f32;
    }

    let overall_avg = if total_tested > 0 {
        sum_scores / total_tested as f32
    } else {
        0.0
    };

    println!("{}", "-".repeat(80));
    println!(
        "   {:25} {:2}/{:2} tested ({:3}%)  |  {:2}/{:2} pass ({:3}%)  |  Avg: {:5.1}/100",
        "TOTAL",
        total_tested,
        total_expected,
        (total_tested as f32 / total_expected as f32 * 100.0) as usize,
        total_passing,
        total_tested,
        (total_passing as f32 / total_tested as f32 * 100.0) as usize,
        overall_avg
    );

    // Detailed failures
    let failures: Vec<_> = results.iter().filter(|r| r.verdict != "PASS").collect();

    if !failures.is_empty() {
        println!("\n📋 ISSUES TO FIX ({} pages need work):", failures.len());
        println!("{}", "=".repeat(80));

        for result in failures.iter().take(10) {
            println!(
                "\n❌ {}/page_{}: {:.0}/100",
                result.pdf, result.page, result.score
            );
            if !result.missing.is_empty() {
                println!("   Missing:");
                for item in result.missing.iter().take(3) {
                    println!("     - {item}");
                }
                if result.missing.len() > 3 {
                    println!("     ... and {} more", result.missing.len() - 3);
                }
            }
        }

        if failures.len() > 10 {
            println!(
                "\n   ... and {} more pages with issues",
                failures.len() - 10
            );
        }
    }

    println!("\n{}", "=".repeat(80));
    println!("TARGET: 42/42 pages score >= 95 (currently {total_passing}/42)");
    println!("{}", "=".repeat(80));

    println!("\n✅ Test: PASS (non-blocking - for information only)");
    println!("   Note: This test always passes but shows progress toward 100%\n");

    // Always pass (non-blocking)
    // No assertion needed - test passes by completing successfully
}
