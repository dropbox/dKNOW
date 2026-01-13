//! Quality Metrics Dashboard CLI
//!
//! Display aggregated statistics from corpus batch review runs.
//! Helps track model performance over time.
//!
//! Usage:
//!   dlviz-metrics ./flagged/           # Display metrics from batch review output
//!   dlviz-metrics ./flagged/ --json    # Output as JSON
//!   dlviz-metrics ./flagged/ --compare ./previous_flagged/  # Compare with previous run

use clap::Parser;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "dlviz-metrics")]
#[command(about = "Quality Metrics Dashboard - Display aggregated ML detection statistics")]
struct Args {
    /// Directory containing corpus_stats.json and flagged_pages.json
    input_dir: PathBuf,

    /// Output as JSON instead of formatted text
    #[arg(long)]
    json: bool,

    /// Compare with a previous run directory
    #[arg(long)]
    compare: Option<PathBuf>,

    /// Show per-document breakdown
    #[arg(long)]
    verbose: bool,

    /// Sort documents by (confidence, flagged, processing_time, name)
    #[arg(long, default_value = "confidence")]
    sort_by: String,

    /// Show only top N problematic documents
    #[arg(long)]
    top_issues: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct CorpusStats {
    total_documents: usize,
    documents_with_flags: usize,
    total_pages: usize,
    total_flagged_pages: usize,
    total_elements: usize,
    total_low_confidence: usize,
    avg_confidence: f64,
    min_confidence: f64,
    label_counts: HashMap<String, usize>,
    total_processing_time_ms: f64,
    documents: Vec<DocumentStats>,
    config: BatchConfig,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code, reason = "fields needed for deserialization")]
struct DocumentStats {
    filename: String,
    total_pages: usize,
    flagged_pages: usize,
    total_elements: usize,
    low_confidence_elements: usize,
    overlap_pages: usize,
    empty_text_elements: usize,
    avg_confidence: f64,
    min_confidence: f64,
    label_counts: HashMap<String, usize>,
    processing_time_ms: f64,
    flagged_page_list: Vec<usize>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code, reason = "fields needed for deserialization")]
struct BatchConfig {
    min_confidence_threshold: f64,
    flag_overlapping: bool,
    flag_empty_text: bool,
    overlap_threshold: f64,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code, reason = "fields needed for deserialization")]
struct FlaggedPage {
    document: String,
    page: usize,
    reasons: Vec<FlagReason>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code, reason = "fields needed for deserialization")]
struct FlagReason {
    element_id: usize,
    reason: String,
    confidence: f64,
    label: String,
}

#[derive(Debug, Serialize)]
struct MetricsReport {
    summary: SummaryMetrics,
    label_distribution: Vec<LabelMetric>,
    problem_areas: ProblemAreas,
    performance: PerformanceMetrics,
    #[serde(skip_serializing_if = "Option::is_none")]
    comparison: Option<ComparisonMetrics>,
}

#[derive(Debug, Serialize)]
struct SummaryMetrics {
    total_documents: usize,
    total_pages: usize,
    total_elements: usize,
    avg_confidence: f64,
    min_confidence: f64,
    flagged_page_rate: f64,
    low_confidence_rate: f64,
}

#[derive(Debug, Serialize)]
struct LabelMetric {
    label: String,
    count: usize,
    percentage: f64,
}

#[derive(Debug, Serialize)]
struct ProblemAreas {
    documents_with_issues: usize,
    most_common_flag_reasons: Vec<(String, usize)>,
    low_confidence_labels: Vec<(String, usize)>,
    worst_documents: Vec<DocumentIssue>,
}

#[derive(Debug, Serialize)]
struct DocumentIssue {
    filename: String,
    avg_confidence: f64,
    flagged_pages: usize,
    low_confidence_elements: usize,
}

#[derive(Debug, Serialize)]
struct PerformanceMetrics {
    total_processing_time_secs: f64,
    avg_time_per_page_ms: f64,
    pages_per_second: f64,
    slowest_document: Option<String>,
    slowest_time_secs: f64,
}

#[derive(Debug, Serialize)]
struct ComparisonMetrics {
    confidence_delta: f64,
    flagged_rate_delta: f64,
    low_confidence_delta: f64,
    improved: bool,
    details: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Load corpus stats
    let stats_path = args.input_dir.join("corpus_stats.json");
    if !stats_path.exists() {
        eprintln!("Error: corpus_stats.json not found in {:?}", args.input_dir);
        eprintln!("Run dlviz-batch-review first to generate statistics.");
        std::process::exit(1);
    }

    let stats: CorpusStats = serde_json::from_reader(std::fs::File::open(&stats_path)?)?;

    // Load flagged pages
    let flagged_path = args.input_dir.join("flagged_pages.json");
    let flagged: Vec<FlaggedPage> = if flagged_path.exists() {
        serde_json::from_reader(std::fs::File::open(&flagged_path)?)?
    } else {
        Vec::new()
    };

    // Load comparison data if specified
    let comparison_stats: Option<CorpusStats> = if let Some(ref compare_dir) = args.compare {
        let compare_path = compare_dir.join("corpus_stats.json");
        if compare_path.exists() {
            Some(serde_json::from_reader(std::fs::File::open(
                &compare_path,
            )?)?)
        } else {
            eprintln!("Warning: No corpus_stats.json in comparison directory");
            None
        }
    } else {
        None
    };

    // Build metrics report
    let report = build_metrics_report(&stats, &flagged, comparison_stats.as_ref(), &args);

    // Output
    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_formatted_report(&report, &stats, &args);
    }

    Ok(())
}

fn build_metrics_report(
    stats: &CorpusStats,
    flagged: &[FlaggedPage],
    comparison: Option<&CorpusStats>,
    args: &Args,
) -> MetricsReport {
    // Summary metrics
    let flagged_page_rate = if stats.total_pages > 0 {
        stats.total_flagged_pages as f64 / stats.total_pages as f64 * 100.0
    } else {
        0.0
    };

    let low_confidence_rate = if stats.total_elements > 0 {
        stats.total_low_confidence as f64 / stats.total_elements as f64 * 100.0
    } else {
        0.0
    };

    let summary = SummaryMetrics {
        total_documents: stats.total_documents,
        total_pages: stats.total_pages,
        total_elements: stats.total_elements,
        avg_confidence: stats.avg_confidence,
        min_confidence: stats.min_confidence,
        flagged_page_rate,
        low_confidence_rate,
    };

    // Label distribution
    let total_labels: usize = stats.label_counts.values().sum();
    let mut label_distribution: Vec<LabelMetric> = stats
        .label_counts
        .iter()
        .map(|(label, count)| LabelMetric {
            label: label.clone(),
            count: *count,
            percentage: if total_labels > 0 {
                *count as f64 / total_labels as f64 * 100.0
            } else {
                0.0
            },
        })
        .collect();
    label_distribution.sort_by(|a, b| b.count.cmp(&a.count));

    // Problem areas
    let mut flag_reasons: HashMap<String, usize> = HashMap::new();
    let mut low_conf_labels: HashMap<String, usize> = HashMap::new();

    for page in flagged {
        for reason in &page.reasons {
            // Extract reason type
            let reason_type = if reason.reason.contains("Low confidence") {
                "Low confidence".to_string()
            } else if reason.reason.contains("Overlap") {
                "Overlapping".to_string()
            } else if reason.reason.contains("Empty text") {
                "Empty text".to_string()
            } else {
                reason.reason.clone()
            };
            *flag_reasons.entry(reason_type).or_insert(0) += 1;

            // Track low confidence labels
            if reason.reason.contains("Low confidence") {
                *low_conf_labels.entry(reason.label.clone()).or_insert(0) += 1;
            }
        }
    }

    let mut most_common_flag_reasons: Vec<(String, usize)> = flag_reasons.into_iter().collect();
    most_common_flag_reasons.sort_by(|a, b| b.1.cmp(&a.1));

    let mut low_confidence_labels: Vec<(String, usize)> = low_conf_labels.into_iter().collect();
    low_confidence_labels.sort_by(|a, b| b.1.cmp(&a.1));

    // Worst documents
    let mut sorted_docs: Vec<&DocumentStats> = stats.documents.iter().collect();
    match args.sort_by.as_str() {
        "confidence" => sorted_docs.sort_by(|a, b| {
            a.avg_confidence
                .partial_cmp(&b.avg_confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
        "flagged" => sorted_docs.sort_by(|a, b| b.flagged_pages.cmp(&a.flagged_pages)),
        "processing_time" => sorted_docs.sort_by(|a, b| {
            b.processing_time_ms
                .partial_cmp(&a.processing_time_ms)
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
        "name" => sorted_docs.sort_by(|a, b| a.filename.cmp(&b.filename)),
        _ => {}
    }

    let limit = args.top_issues.unwrap_or(5);
    let worst_documents: Vec<DocumentIssue> = sorted_docs
        .iter()
        .take(limit)
        .map(|d| DocumentIssue {
            filename: d.filename.clone(),
            avg_confidence: d.avg_confidence,
            flagged_pages: d.flagged_pages,
            low_confidence_elements: d.low_confidence_elements,
        })
        .collect();

    let problem_areas = ProblemAreas {
        documents_with_issues: stats.documents_with_flags,
        most_common_flag_reasons,
        low_confidence_labels,
        worst_documents,
    };

    // Performance metrics
    let total_processing_time_secs = stats.total_processing_time_ms / 1000.0;
    let avg_time_per_page_ms = if stats.total_pages > 0 {
        stats.total_processing_time_ms / stats.total_pages as f64
    } else {
        0.0
    };
    let pages_per_second = if total_processing_time_secs > 0.0 {
        stats.total_pages as f64 / total_processing_time_secs
    } else {
        0.0
    };

    let slowest_doc = stats.documents.iter().max_by(|a, b| {
        a.processing_time_ms
            .partial_cmp(&b.processing_time_ms)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let performance = PerformanceMetrics {
        total_processing_time_secs,
        avg_time_per_page_ms,
        pages_per_second,
        slowest_document: slowest_doc.map(|d| d.filename.clone()),
        slowest_time_secs: slowest_doc
            .map(|d| d.processing_time_ms / 1000.0)
            .unwrap_or(0.0),
    };

    // Comparison metrics
    let comparison_metrics = comparison.map(|prev| {
        let confidence_delta = stats.avg_confidence - prev.avg_confidence;
        let prev_flagged_rate = if prev.total_pages > 0 {
            prev.total_flagged_pages as f64 / prev.total_pages as f64 * 100.0
        } else {
            0.0
        };
        let flagged_rate_delta = flagged_page_rate - prev_flagged_rate;

        let prev_low_conf_rate = if prev.total_elements > 0 {
            prev.total_low_confidence as f64 / prev.total_elements as f64 * 100.0
        } else {
            0.0
        };
        let low_confidence_delta = low_confidence_rate - prev_low_conf_rate;

        // Improved if confidence went up or flagged rate went down
        let improved = confidence_delta > 0.0 || flagged_rate_delta < 0.0;

        let details = format!(
            "Confidence: {:.2}% → {:.2}% ({:+.2}%), Flagged pages: {:.1}% → {:.1}% ({:+.1}%)",
            prev.avg_confidence * 100.0,
            stats.avg_confidence * 100.0,
            confidence_delta * 100.0,
            prev_flagged_rate,
            flagged_page_rate,
            flagged_rate_delta
        );

        ComparisonMetrics {
            confidence_delta,
            flagged_rate_delta,
            low_confidence_delta,
            improved,
            details,
        }
    });

    MetricsReport {
        summary,
        label_distribution,
        problem_areas,
        performance,
        comparison: comparison_metrics,
    }
}

fn print_formatted_report(report: &MetricsReport, stats: &CorpusStats, args: &Args) {
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║            QUALITY METRICS DASHBOARD                             ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
    println!();

    // Summary
    println!("┌─────────────────────────────────────────────────────────────────┐");
    println!("│ SUMMARY                                                         │");
    println!("├─────────────────────────────────────────────────────────────────┤");
    println!(
        "│  Documents: {:>6}    Pages: {:>6}    Elements: {:>8}      │",
        report.summary.total_documents, report.summary.total_pages, report.summary.total_elements
    );
    println!(
        "│  Avg Confidence: {:>6.2}%   Min: {:>6.2}%                         │",
        report.summary.avg_confidence * 100.0,
        report.summary.min_confidence * 100.0
    );
    println!(
        "│  Flagged Pages: {:>6.1}%   Low Confidence Elements: {:>6.1}%     │",
        report.summary.flagged_page_rate, report.summary.low_confidence_rate
    );
    println!("└─────────────────────────────────────────────────────────────────┘");
    println!();

    // Label distribution
    println!("┌─────────────────────────────────────────────────────────────────┐");
    println!("│ LABEL DISTRIBUTION                                              │");
    println!("├─────────────────────────────────────────────────────────────────┤");
    for label in &report.label_distribution {
        let bar_len = (label.percentage / 2.0).min(30.0) as usize;
        let bar = "█".repeat(bar_len);
        println!(
            "│  {:>15}: {:>6} ({:>5.1}%) {}",
            label.label, label.count, label.percentage, bar
        );
    }
    println!("└─────────────────────────────────────────────────────────────────┘");
    println!();

    // Problem areas
    println!("┌─────────────────────────────────────────────────────────────────┐");
    println!("│ PROBLEM AREAS                                                   │");
    println!("├─────────────────────────────────────────────────────────────────┤");
    println!(
        "│  Documents with issues: {:>4}                                    │",
        report.problem_areas.documents_with_issues
    );
    println!("│                                                                 │");
    println!("│  Most common flag reasons:                                      │");
    for (reason, count) in &report.problem_areas.most_common_flag_reasons {
        println!(
            "│    • {}: {}                                ",
            reason, count
        );
    }
    println!("│                                                                 │");
    println!("│  Labels with low confidence issues:                             │");
    for (label, count) in report.problem_areas.low_confidence_labels.iter().take(5) {
        println!(
            "│    • {}: {}                                ",
            label, count
        );
    }
    println!("│                                                                 │");
    println!(
        "│  Problematic documents (by {}):                 ",
        args.sort_by
    );
    for doc in &report.problem_areas.worst_documents {
        println!(
            "│    • {} (conf: {:.1}%, flagged: {}, low_conf: {})",
            doc.filename,
            doc.avg_confidence * 100.0,
            doc.flagged_pages,
            doc.low_confidence_elements
        );
    }
    println!("└─────────────────────────────────────────────────────────────────┘");
    println!();

    // Performance
    println!("┌─────────────────────────────────────────────────────────────────┐");
    println!("│ PERFORMANCE                                                     │");
    println!("├─────────────────────────────────────────────────────────────────┤");
    println!(
        "│  Total processing time: {:>8.1} seconds                        │",
        report.performance.total_processing_time_secs
    );
    println!(
        "│  Average time per page: {:>8.1} ms                             │",
        report.performance.avg_time_per_page_ms
    );
    println!(
        "│  Throughput: {:>8.2} pages/second                              │",
        report.performance.pages_per_second
    );
    if let Some(ref slowest) = report.performance.slowest_document {
        println!(
            "│  Slowest document: {} ({:.1}s)         ",
            slowest, report.performance.slowest_time_secs
        );
    }
    println!("└─────────────────────────────────────────────────────────────────┘");
    println!();

    // Comparison (if available)
    if let Some(ref cmp) = report.comparison {
        println!("┌─────────────────────────────────────────────────────────────────┐");
        println!("│ COMPARISON WITH PREVIOUS RUN                                    │");
        println!("├─────────────────────────────────────────────────────────────────┤");
        let status = if cmp.improved {
            "✓ IMPROVED"
        } else {
            "✗ REGRESSED"
        };
        println!(
            "│  Status: {}                                              │",
            status
        );
        println!("│  {}  │", cmp.details);
        println!("└─────────────────────────────────────────────────────────────────┘");
        println!();
    }

    // Verbose per-document breakdown
    if args.verbose {
        println!("┌─────────────────────────────────────────────────────────────────┐");
        println!("│ PER-DOCUMENT BREAKDOWN                                          │");
        println!("├─────────────────────────────────────────────────────────────────┤");
        for doc in &stats.documents {
            println!("│  {} ({} pages)", doc.filename, doc.total_pages);
            println!(
                "│    Confidence: {:.1}% avg, {:.1}% min",
                doc.avg_confidence * 100.0,
                doc.min_confidence * 100.0
            );
            println!(
                "│    Elements: {}, Low confidence: {}, Flagged pages: {}",
                doc.total_elements, doc.low_confidence_elements, doc.flagged_pages
            );
            println!(
                "│    Processing time: {:.1}s",
                doc.processing_time_ms / 1000.0
            );
            println!("│");
        }
        println!("└─────────────────────────────────────────────────────────────────┘");
    }

    // Config info
    println!("┌─────────────────────────────────────────────────────────────────┐");
    println!("│ BATCH REVIEW CONFIG                                             │");
    println!("├─────────────────────────────────────────────────────────────────┤");
    println!(
        "│  Confidence threshold: {:.0}%                                     │",
        stats.config.min_confidence_threshold * 100.0
    );
    println!(
        "│  Flag overlapping: {}                                           │",
        if stats.config.flag_overlapping {
            "yes"
        } else {
            "no "
        }
    );
    println!(
        "│  Flag empty text: {}                                            │",
        if stats.config.flag_empty_text {
            "yes"
        } else {
            "no "
        }
    );
    println!("└─────────────────────────────────────────────────────────────────┘");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_stats() -> CorpusStats {
        CorpusStats {
            total_documents: 3,
            documents_with_flags: 2,
            total_pages: 10,
            total_flagged_pages: 3,
            total_elements: 100,
            total_low_confidence: 20,
            avg_confidence: 0.85,
            min_confidence: 0.3,
            label_counts: {
                let mut map = HashMap::new();
                map.insert("Text".to_string(), 80);
                map.insert("Picture".to_string(), 15);
                map.insert("Table".to_string(), 5);
                map
            },
            total_processing_time_ms: 5000.0,
            documents: vec![
                DocumentStats {
                    filename: "doc1.pdf".to_string(),
                    total_pages: 5,
                    flagged_pages: 2,
                    total_elements: 50,
                    low_confidence_elements: 10,
                    overlap_pages: 0,
                    empty_text_elements: 0,
                    avg_confidence: 0.8,
                    min_confidence: 0.3,
                    label_counts: HashMap::new(),
                    processing_time_ms: 2500.0,
                    flagged_page_list: vec![0, 2],
                },
                DocumentStats {
                    filename: "doc2.pdf".to_string(),
                    total_pages: 3,
                    flagged_pages: 1,
                    total_elements: 30,
                    low_confidence_elements: 8,
                    overlap_pages: 0,
                    empty_text_elements: 0,
                    avg_confidence: 0.85,
                    min_confidence: 0.4,
                    label_counts: HashMap::new(),
                    processing_time_ms: 1500.0,
                    flagged_page_list: vec![1],
                },
                DocumentStats {
                    filename: "doc3.pdf".to_string(),
                    total_pages: 2,
                    flagged_pages: 0,
                    total_elements: 20,
                    low_confidence_elements: 2,
                    overlap_pages: 0,
                    empty_text_elements: 0,
                    avg_confidence: 0.95,
                    min_confidence: 0.7,
                    label_counts: HashMap::new(),
                    processing_time_ms: 1000.0,
                    flagged_page_list: vec![],
                },
            ],
            config: BatchConfig {
                min_confidence_threshold: 0.85,
                flag_overlapping: false,
                flag_empty_text: false,
                overlap_threshold: 0.5,
            },
        }
    }

    fn create_test_flagged() -> Vec<FlaggedPage> {
        vec![
            FlaggedPage {
                document: "doc1.pdf".to_string(),
                page: 0,
                reasons: vec![FlagReason {
                    element_id: 0,
                    reason: "Low confidence: 0.30 < 0.85".to_string(),
                    confidence: 0.3,
                    label: "Text".to_string(),
                }],
            },
            FlaggedPage {
                document: "doc1.pdf".to_string(),
                page: 2,
                reasons: vec![FlagReason {
                    element_id: 5,
                    reason: "Low confidence: 0.50 < 0.85".to_string(),
                    confidence: 0.5,
                    label: "Picture".to_string(),
                }],
            },
            FlaggedPage {
                document: "doc2.pdf".to_string(),
                page: 1,
                reasons: vec![FlagReason {
                    element_id: 3,
                    reason: "Low confidence: 0.40 < 0.85".to_string(),
                    confidence: 0.4,
                    label: "Text".to_string(),
                }],
            },
        ]
    }

    #[test]
    fn test_summary_metrics_calculation() {
        let stats = create_test_stats();
        let flagged = create_test_flagged();
        let args = Args {
            input_dir: PathBuf::from("."),
            json: false,
            compare: None,
            verbose: false,
            sort_by: "confidence".to_string(),
            top_issues: None,
        };

        let report = build_metrics_report(&stats, &flagged, None, &args);

        assert_eq!(report.summary.total_documents, 3);
        assert_eq!(report.summary.total_pages, 10);
        assert_eq!(report.summary.total_elements, 100);
        assert!((report.summary.avg_confidence - 0.85).abs() < 0.001);
        assert!((report.summary.min_confidence - 0.3).abs() < 0.001);
        assert!((report.summary.flagged_page_rate - 30.0).abs() < 0.001); // 3/10 = 30%
        assert!((report.summary.low_confidence_rate - 20.0).abs() < 0.001); // 20/100 = 20%
    }

    #[test]
    fn test_label_distribution() {
        let stats = create_test_stats();
        let flagged = create_test_flagged();
        let args = Args {
            input_dir: PathBuf::from("."),
            json: false,
            compare: None,
            verbose: false,
            sort_by: "confidence".to_string(),
            top_issues: None,
        };

        let report = build_metrics_report(&stats, &flagged, None, &args);

        assert_eq!(report.label_distribution.len(), 3);
        // Labels are sorted by count (descending)
        assert_eq!(report.label_distribution[0].label, "Text");
        assert_eq!(report.label_distribution[0].count, 80);
        assert!((report.label_distribution[0].percentage - 80.0).abs() < 0.001);
    }

    #[test]
    fn test_problem_areas() {
        let stats = create_test_stats();
        let flagged = create_test_flagged();
        let args = Args {
            input_dir: PathBuf::from("."),
            json: false,
            compare: None,
            verbose: false,
            sort_by: "confidence".to_string(),
            top_issues: None,
        };

        let report = build_metrics_report(&stats, &flagged, None, &args);

        assert_eq!(report.problem_areas.documents_with_issues, 2);
        // Check flag reasons
        assert!(!report.problem_areas.most_common_flag_reasons.is_empty());
        assert_eq!(
            report.problem_areas.most_common_flag_reasons[0].0,
            "Low confidence"
        );
        assert_eq!(report.problem_areas.most_common_flag_reasons[0].1, 3);
        // Check low confidence labels
        assert!(!report.problem_areas.low_confidence_labels.is_empty());
    }

    #[test]
    fn test_performance_metrics() {
        let stats = create_test_stats();
        let flagged = create_test_flagged();
        let args = Args {
            input_dir: PathBuf::from("."),
            json: false,
            compare: None,
            verbose: false,
            sort_by: "confidence".to_string(),
            top_issues: None,
        };

        let report = build_metrics_report(&stats, &flagged, None, &args);

        assert!((report.performance.total_processing_time_secs - 5.0).abs() < 0.001);
        assert!((report.performance.avg_time_per_page_ms - 500.0).abs() < 0.001); // 5000/10
        assert!((report.performance.pages_per_second - 2.0).abs() < 0.001); // 10/5
        assert_eq!(
            report.performance.slowest_document,
            Some("doc1.pdf".to_string())
        );
    }

    #[test]
    fn test_comparison_metrics() {
        let stats = create_test_stats();
        let mut prev_stats = create_test_stats();
        prev_stats.avg_confidence = 0.80; // Lower than current
        prev_stats.total_flagged_pages = 5; // Higher than current

        let flagged = create_test_flagged();
        let args = Args {
            input_dir: PathBuf::from("."),
            json: false,
            compare: None,
            verbose: false,
            sort_by: "confidence".to_string(),
            top_issues: None,
        };

        let report = build_metrics_report(&stats, &flagged, Some(&prev_stats), &args);

        assert!(report.comparison.is_some());
        let cmp = report.comparison.unwrap();
        assert!(cmp.improved); // Confidence went up, flagged rate went down
        assert!((cmp.confidence_delta - 0.05).abs() < 0.001); // 0.85 - 0.80
    }

    #[test]
    fn test_sort_by_confidence() {
        let stats = create_test_stats();
        let flagged = create_test_flagged();
        let args = Args {
            input_dir: PathBuf::from("."),
            json: false,
            compare: None,
            verbose: false,
            sort_by: "confidence".to_string(),
            top_issues: Some(5),
        };

        let report = build_metrics_report(&stats, &flagged, None, &args);

        // Documents should be sorted by confidence (ascending - worst first)
        assert_eq!(report.problem_areas.worst_documents[0].filename, "doc1.pdf");
        assert_eq!(report.problem_areas.worst_documents[1].filename, "doc2.pdf");
        assert_eq!(report.problem_areas.worst_documents[2].filename, "doc3.pdf");
    }

    #[test]
    fn test_top_issues_limit() {
        let stats = create_test_stats();
        let flagged = create_test_flagged();
        let args = Args {
            input_dir: PathBuf::from("."),
            json: false,
            compare: None,
            verbose: false,
            sort_by: "confidence".to_string(),
            top_issues: Some(2), // Only show top 2
        };

        let report = build_metrics_report(&stats, &flagged, None, &args);

        assert_eq!(report.problem_areas.worst_documents.len(), 2);
    }

    #[test]
    fn test_json_serialization() {
        let stats = create_test_stats();
        let flagged = create_test_flagged();
        let args = Args {
            input_dir: PathBuf::from("."),
            json: true,
            compare: None,
            verbose: false,
            sort_by: "confidence".to_string(),
            top_issues: None,
        };

        let report = build_metrics_report(&stats, &flagged, None, &args);
        let json = serde_json::to_string(&report).expect("JSON serialization failed");

        assert!(json.contains("total_documents"));
        assert!(json.contains("label_distribution"));
        assert!(json.contains("problem_areas"));
        assert!(json.contains("performance"));
    }
}
