//! Output formatting and comparison utilities.
//!
//! This module provides functions for:
//!
//! - **Ground Truth Persistence**: Save/load ground truth as JSON
//! - **Markdown Generation**: Convert ground truth to markdown format
//! - **Output Comparison**: Compare Rust extraction against ground truth
//! - **Report Generation**: Create detailed comparison reports
//!
//! ## Comparison Metrics
//!
//! The comparison evaluates three dimensions:
//!
//! 1. **Accuracy** (element-level): Percentage of ground truth elements found in Rust output
//! 2. **Text Similarity**: Character-level Jaccard similarity between normalized texts
//! 3. **Structure Similarity**: Combined from:
//!    - Label distribution similarity (40%)
//!    - Label sequence LCS ratio (40%)
//!    - Structural element preservation (20%)
//!
//! ## Example
//!
//! ```no_run
//! use docling_llm_verify::{compare_outputs, generate_report, save_ground_truth};
//! use docling_llm_verify::DocumentGroundTruth;
//! use std::path::Path;
//!
//! # fn example(ground_truth: &DocumentGroundTruth) -> anyhow::Result<()> {
//! // Save ground truth
//! save_ground_truth(ground_truth, Path::new("output/"))?;
//!
//! // Compare against Rust output
//! let rust_markdown = "# Title\n\nParagraph text...";
//! let comparison = compare_outputs(ground_truth, rust_markdown);
//!
//! // Generate report
//! let report = generate_report(&[comparison]);
//! println!("{}", report);
//! # Ok(())
//! # }
//! ```
//!
//! ## Output Files
//!
//! `save_ground_truth` creates:
//! - `document.json` - Full document ground truth
//! - `page_N.json` - Per-page extraction data
//! - `document.md` - Rendered markdown
//! - `confidence.json` - Per-element confidence scores

// Clippy pedantic allows:
// - Percentage calculations use f64 from usize
#![allow(clippy::cast_precision_loss)]

use crate::models::{
    ComparisonResult, DocItemLabel, DocumentGroundTruth, ExtractedElement, PdfCostReport,
};
use anyhow::Result;
use std::collections::HashMap;
use std::fmt::Write;
use std::path::Path;

/// Save ground truth to JSON file.
///
/// # Errors
///
/// Returns an error if file operations fail or serialization fails.
#[must_use = "this function returns a Result that should be handled"]
pub fn save_ground_truth(ground_truth: &DocumentGroundTruth, output_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(output_dir)?;

    // Save full document
    let doc_path = output_dir.join("document.json");
    let doc_json = serde_json::to_string_pretty(ground_truth)?;
    std::fs::write(&doc_path, doc_json)?;

    // Save per-page files
    for page in &ground_truth.pages {
        let page_path = output_dir.join(format!("page_{}.json", page.page_number));
        let page_json = serde_json::to_string_pretty(page)?;
        std::fs::write(&page_path, page_json)?;
    }

    // Generate markdown
    let md_path = output_dir.join("document.md");
    let markdown = generate_markdown(ground_truth);
    std::fs::write(&md_path, markdown)?;

    // Save confidence scores
    let conf_path = output_dir.join("confidence.json");
    let confidence: Vec<Vec<f64>> = ground_truth
        .pages
        .iter()
        .map(|p| p.agreement_scores.clone())
        .collect();
    let conf_json = serde_json::to_string_pretty(&confidence)?;
    std::fs::write(&conf_path, conf_json)?;

    Ok(())
}

/// Generate markdown from ground truth.
#[must_use = "generates markdown from ground truth"]
pub fn generate_markdown(ground_truth: &DocumentGroundTruth) -> String {
    let mut md = String::new();

    for page in &ground_truth.pages {
        if page.page_number > 1 {
            md.push_str("\n---\n\n");
        }

        for (i, elem) in page.elements.iter().enumerate() {
            let idx = if i < page.reading_order.len() {
                page.reading_order[i]
            } else {
                i
            };

            let elem = if idx < page.elements.len() {
                &page.elements[idx]
            } else {
                elem
            };

            md.push_str(&format_element_as_markdown(elem));
            md.push('\n');
        }
    }

    md
}

fn format_element_as_markdown(elem: &ExtractedElement) -> String {
    match elem.label {
        DocItemLabel::Title => format!("# {}\n", elem.text),
        DocItemLabel::SectionHeader => format!("## {}\n", elem.text),
        DocItemLabel::Paragraph | DocItemLabel::Text => format!("{}\n", elem.text),
        DocItemLabel::ListItem => format!("- {}\n", elem.text),
        DocItemLabel::Table => elem
            .table_data
            .as_ref()
            .map_or_else(|| format!("{}\n", elem.text), format_table),
        DocItemLabel::Picture => format!("![{}](image)\n", elem.text),
        DocItemLabel::Caption => format!("*{}*\n", elem.text),
        DocItemLabel::Footnote => format!("[^]: {}\n", elem.text),
        DocItemLabel::Formula => format!("$${}$$\n", elem.text),
        DocItemLabel::PageHeader | DocItemLabel::PageFooter => String::new(), // Skip
        DocItemLabel::Code => format!("```\n{}\n```\n", elem.text),
        DocItemLabel::Checkbox => format!("- [ ] {}\n", elem.text),
        DocItemLabel::Reference => format!("[{}]\n", elem.text),
    }
}

fn format_table(table: &crate::models::TableData) -> String {
    if table.rows.is_empty() {
        return String::new();
    }

    let mut md = String::new();

    // Header row
    if let Some(header) = table.rows.first() {
        md.push('|');
        for cell in header {
            let _ = write!(md, " {cell} |");
        }
        md.push('\n');

        // Separator
        md.push('|');
        for _ in header {
            md.push_str(" --- |");
        }
        md.push('\n');
    }

    // Data rows
    for row in table.rows.iter().skip(1) {
        md.push('|');
        for cell in row {
            let _ = write!(md, " {cell} |");
        }
        md.push('\n');
    }

    md
}

/// Compare Rust output against ground truth.
#[must_use = "compares outputs and returns metrics"]
pub fn compare_outputs(
    ground_truth: &DocumentGroundTruth,
    rust_markdown: &str,
) -> ComparisonResult {
    let gt_markdown = generate_markdown(ground_truth);

    // Text similarity using Levenshtein-like metric
    let text_similarity = compute_text_similarity(&gt_markdown, rust_markdown);

    // Count elements
    let gt_elements: Vec<&ExtractedElement> = ground_truth
        .pages
        .iter()
        .flat_map(|p| &p.elements)
        .collect();

    // Extract element-level metrics
    let mut missing_elements = Vec::new();
    let label_mismatches = Vec::new();

    for elem in &gt_elements {
        if !rust_markdown.contains(&elem.text) && elem.text.len() > 10 {
            missing_elements.push(truncate(&elem.text, 50));
        }
    }

    let accuracy = if gt_elements.is_empty() {
        100.0
    } else {
        let found = gt_elements
            .iter()
            .filter(|e| rust_markdown.contains(&e.text) || e.text.len() < 5)
            .count();
        (found as f64 / gt_elements.len() as f64) * 100.0
    };

    // Compute structure similarity
    let structure_similarity = compute_structure_similarity(ground_truth, rust_markdown);

    ComparisonResult {
        filename: ground_truth.filename.clone(),
        accuracy_percent: accuracy,
        text_similarity: text_similarity * 100.0,
        structure_similarity: structure_similarity * 100.0,
        missing_elements,
        extra_elements: vec![],
        label_mismatches,
    }
}

/// Compute structure similarity between ground truth and Rust output.
///
/// This measures how similar the document structure is, considering:
/// 1. Element type distribution (are there similar counts of each label type?)
/// 2. Label sequence similarity (are elements in similar order?)
/// 3. Reading order preservation
fn compute_structure_similarity(ground_truth: &DocumentGroundTruth, rust_markdown: &str) -> f64 {
    // Get ground truth element labels
    let gt_labels: Vec<DocItemLabel> = ground_truth
        .pages
        .iter()
        .flat_map(|p| p.elements.iter().map(|e| e.label))
        .collect();

    if gt_labels.is_empty() {
        return 1.0; // Empty documents are structurally identical
    }

    // Infer structure from Rust markdown output
    let inferred_labels = infer_labels_from_markdown(rust_markdown);

    // Component 1: Label distribution similarity (Jaccard on label counts)
    let gt_counts = count_labels(&gt_labels);
    let rust_counts = count_labels(&inferred_labels);
    let distribution_similarity = compute_count_similarity(&gt_counts, &rust_counts);

    // Component 2: Label sequence similarity (longest common subsequence ratio)
    let sequence_similarity = compute_lcs_ratio(&gt_labels, &inferred_labels);

    // Component 3: Structural element ratio (tables, headers, lists preserved?)
    let structural_similarity = compute_structural_element_ratio(&gt_counts, &rust_counts);

    // Weighted combination: distribution and sequence matter most
    let similarity = distribution_similarity.mul_add(
        0.4,
        sequence_similarity.mul_add(0.4, 0.2 * structural_similarity),
    );

    similarity.clamp(0.0, 1.0)
}

/// Infer document element labels from markdown text.
fn infer_labels_from_markdown(markdown: &str) -> Vec<DocItemLabel> {
    let mut labels = Vec::new();

    for line in markdown.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let label = if trimmed.starts_with("# ") && !trimmed.starts_with("## ") {
            DocItemLabel::Title
        } else if trimmed.starts_with("## ") || trimmed.starts_with("### ") {
            DocItemLabel::SectionHeader
        } else if trimmed.starts_with("- [ ]") || trimmed.starts_with("- [x]") {
            DocItemLabel::Checkbox
        } else if trimmed.starts_with("- ")
            || trimmed.starts_with("* ")
            || trimmed.starts_with("1. ")
        {
            DocItemLabel::ListItem
        } else if trimmed.starts_with('|') && trimmed.ends_with('|') {
            DocItemLabel::Table
        } else if trimmed.starts_with("```") {
            DocItemLabel::Code
        } else if trimmed.starts_with("$$") || trimmed.starts_with('$') {
            DocItemLabel::Formula
        } else if trimmed.starts_with("![") {
            DocItemLabel::Picture
        } else if trimmed.starts_with('*') && trimmed.ends_with('*') && trimmed.len() > 2 {
            DocItemLabel::Caption
        } else if trimmed.starts_with("[^") {
            DocItemLabel::Footnote
        } else if trimmed.starts_with('[') && trimmed.ends_with(']') {
            DocItemLabel::Reference
        } else {
            DocItemLabel::Paragraph
        };

        labels.push(label);
    }

    labels
}

/// Count occurrences of each label type.
fn count_labels(labels: &[DocItemLabel]) -> HashMap<DocItemLabel, usize> {
    let mut counts = HashMap::new();
    for label in labels {
        *counts.entry(*label).or_insert(0) += 1;
    }
    counts
}

/// Compute similarity between two label count distributions.
fn compute_count_similarity(
    a: &HashMap<DocItemLabel, usize>,
    b: &HashMap<DocItemLabel, usize>,
) -> f64 {
    let all_labels: std::collections::HashSet<_> = a.keys().chain(b.keys()).collect();

    if all_labels.is_empty() {
        return 1.0;
    }

    let mut intersection = 0usize;
    let mut union = 0usize;

    for label in all_labels {
        let count_a = *a.get(label).unwrap_or(&0);
        let count_b = *b.get(label).unwrap_or(&0);
        intersection += count_a.min(count_b);
        union += count_a.max(count_b);
    }

    if union == 0 {
        1.0
    } else {
        intersection as f64 / union as f64
    }
}

/// Compute longest common subsequence ratio.
fn compute_lcs_ratio(a: &[DocItemLabel], b: &[DocItemLabel]) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }

    let lcs_len = lcs_length(a, b);
    let max_len = a.len().max(b.len());

    lcs_len as f64 / max_len as f64
}

/// Compute length of longest common subsequence.
fn lcs_length(a: &[DocItemLabel], b: &[DocItemLabel]) -> usize {
    let m = a.len();
    let n = b.len();

    // Use space-optimized LCS (only need two rows)
    let mut prev = vec![0usize; n + 1];
    let mut curr = vec![0usize; n + 1];

    for i in 1..=m {
        for j in 1..=n {
            if a[i - 1] == b[j - 1] {
                curr[j] = prev[j - 1] + 1;
            } else {
                curr[j] = prev[j].max(curr[j - 1]);
            }
        }
        std::mem::swap(&mut prev, &mut curr);
        curr.fill(0);
    }

    prev[n]
}

/// Compute ratio of structural elements (tables, headers, lists) preserved.
fn compute_structural_element_ratio(
    gt: &HashMap<DocItemLabel, usize>,
    rust: &HashMap<DocItemLabel, usize>,
) -> f64 {
    let structural_labels = [
        DocItemLabel::Title,
        DocItemLabel::SectionHeader,
        DocItemLabel::Table,
        DocItemLabel::ListItem,
        DocItemLabel::Picture,
    ];

    let mut gt_count = 0usize;
    let mut preserved = 0usize;

    for label in &structural_labels {
        let gt_n = *gt.get(label).unwrap_or(&0);
        let rust_n = *rust.get(label).unwrap_or(&0);
        gt_count += gt_n;
        preserved += gt_n.min(rust_n);
    }

    if gt_count == 0 {
        1.0
    } else {
        preserved as f64 / gt_count as f64
    }
}

fn compute_text_similarity(a: &str, b: &str) -> f64 {
    // Normalize whitespace
    let a_norm: String = a.split_whitespace().collect::<Vec<_>>().join(" ");
    let b_norm: String = b.split_whitespace().collect::<Vec<_>>().join(" ");

    if a_norm.is_empty() && b_norm.is_empty() {
        return 1.0;
    }

    // Character-level Jaccard similarity
    let a_chars: std::collections::HashSet<char> = a_norm.chars().collect();
    let b_chars: std::collections::HashSet<char> = b_norm.chars().collect();

    let intersection = a_chars.intersection(&b_chars).count();
    let union = a_chars.union(&b_chars).count();

    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

/// Generate comparison report.
#[must_use = "generates comparison report"]
pub fn generate_report(results: &[ComparisonResult]) -> String {
    let mut report = String::new();

    report.push_str("# LLM Ensemble Verification Report\n\n");
    let _ = writeln!(
        report,
        "Generated: {}\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    );

    // Summary table
    report.push_str("## Summary\n\n");
    report.push_str(
        "| File | Accuracy | Text Similarity | Structure Similarity | Missing Elements |\n",
    );
    report.push_str(
        "|------|----------|-----------------|---------------------|------------------|\n",
    );

    for r in results {
        let _ = writeln!(
            report,
            "| {} | {:.1}% | {:.1}% | {:.1}% | {} |",
            r.filename,
            r.accuracy_percent,
            r.text_similarity,
            r.structure_similarity,
            r.missing_elements.len()
        );
    }

    // Overall statistics
    let avg_accuracy =
        results.iter().map(|r| r.accuracy_percent).sum::<f64>() / results.len() as f64;
    let avg_text_similarity =
        results.iter().map(|r| r.text_similarity).sum::<f64>() / results.len() as f64;
    let avg_structure_similarity =
        results.iter().map(|r| r.structure_similarity).sum::<f64>() / results.len() as f64;

    let _ = writeln!(report, "\n**Average Accuracy:** {avg_accuracy:.1}%");
    let _ = writeln!(
        report,
        "**Average Text Similarity:** {avg_text_similarity:.1}%"
    );
    let _ = writeln!(
        report,
        "**Average Structure Similarity:** {avg_structure_similarity:.1}%"
    );

    // Detailed results
    report.push_str("\n## Detailed Results\n\n");

    for r in results {
        let _ = writeln!(report, "### {}\n", r.filename);
        let _ = writeln!(report, "- Accuracy: {:.1}%", r.accuracy_percent);
        let _ = writeln!(report, "- Text Similarity: {:.1}%", r.text_similarity);
        let _ = writeln!(
            report,
            "- Structure Similarity: {:.1}%",
            r.structure_similarity
        );

        if !r.missing_elements.is_empty() {
            report.push_str("\nMissing Elements:\n");
            for elem in &r.missing_elements {
                let _ = writeln!(report, "- {elem}");
            }
        }

        report.push('\n');
    }

    report
}

/// Save cost report.
///
/// # Errors
///
/// Returns an error if file writing or serialization fails.
#[must_use = "this function returns a Result that should be handled"]
pub fn save_cost_report(report: &PdfCostReport, output_path: &Path) -> Result<()> {
    let json = serde_json::to_string_pretty(report)?;
    std::fs::write(output_path, json)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_labels_from_markdown() {
        let markdown = r"# Title
## Section Header
Some paragraph text.
- List item 1
- List item 2
| Header |
| --- |
| Cell |
```
code block
```
";
        let labels = infer_labels_from_markdown(markdown);

        assert!(labels.contains(&DocItemLabel::Title));
        assert!(labels.contains(&DocItemLabel::SectionHeader));
        assert!(labels.contains(&DocItemLabel::Paragraph));
        assert!(labels.contains(&DocItemLabel::ListItem));
        assert!(labels.contains(&DocItemLabel::Table));
        assert!(labels.contains(&DocItemLabel::Code));
    }

    #[test]
    fn test_count_labels() {
        let labels = vec![
            DocItemLabel::Paragraph,
            DocItemLabel::Paragraph,
            DocItemLabel::Title,
            DocItemLabel::Table,
        ];
        let counts = count_labels(&labels);

        assert_eq!(*counts.get(&DocItemLabel::Paragraph).unwrap(), 2);
        assert_eq!(*counts.get(&DocItemLabel::Title).unwrap(), 1);
        assert_eq!(*counts.get(&DocItemLabel::Table).unwrap(), 1);
    }

    #[test]
    fn test_compute_count_similarity_identical() {
        let mut a = HashMap::new();
        a.insert(DocItemLabel::Paragraph, 5);
        a.insert(DocItemLabel::Title, 1);

        let similarity = compute_count_similarity(&a, &a);
        assert!((similarity - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_compute_count_similarity_different() {
        let mut a = HashMap::new();
        a.insert(DocItemLabel::Paragraph, 5);
        a.insert(DocItemLabel::Title, 1);

        let mut b = HashMap::new();
        b.insert(DocItemLabel::Paragraph, 3);
        b.insert(DocItemLabel::Table, 2);

        let similarity = compute_count_similarity(&a, &b);
        // Intersection: min(5,3) + min(1,0) + min(0,2) = 3 + 0 + 0 = 3
        // Union: max(5,3) + max(1,0) + max(0,2) = 5 + 1 + 2 = 8
        // Similarity: 3/8 = 0.375
        assert!((similarity - 0.375).abs() < 0.001);
    }

    #[test]
    fn test_lcs_length_identical() {
        let a = vec![
            DocItemLabel::Title,
            DocItemLabel::Paragraph,
            DocItemLabel::Table,
        ];
        let lcs = lcs_length(&a, &a);
        assert_eq!(lcs, 3);
    }

    #[test]
    fn test_lcs_length_partial() {
        let a = vec![
            DocItemLabel::Title,
            DocItemLabel::Paragraph,
            DocItemLabel::Table,
        ];
        let b = vec![
            DocItemLabel::Title,
            DocItemLabel::ListItem,
            DocItemLabel::Table,
        ];
        let lcs = lcs_length(&a, &b);
        // LCS: Title, Table = 2
        assert_eq!(lcs, 2);
    }

    #[test]
    fn test_lcs_length_empty() {
        let a: Vec<DocItemLabel> = vec![];
        let b = vec![DocItemLabel::Title];
        assert_eq!(lcs_length(&a, &b), 0);
        assert_eq!(lcs_length(&b, &a), 0);
    }

    #[test]
    fn test_compute_lcs_ratio() {
        let a = vec![DocItemLabel::Title, DocItemLabel::Paragraph];
        let b = vec![DocItemLabel::Title, DocItemLabel::Paragraph];
        let ratio = compute_lcs_ratio(&a, &b);
        assert!((ratio - 1.0).abs() < 0.001);

        let c = vec![DocItemLabel::Title, DocItemLabel::Table];
        let ratio2 = compute_lcs_ratio(&a, &c);
        // LCS = 1, max_len = 2, ratio = 0.5
        assert!((ratio2 - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_compute_structural_element_ratio_identical() {
        let mut counts = HashMap::new();
        counts.insert(DocItemLabel::Title, 1);
        counts.insert(DocItemLabel::Table, 2);
        counts.insert(DocItemLabel::ListItem, 3);

        let ratio = compute_structural_element_ratio(&counts, &counts);
        assert!((ratio - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_compute_structural_element_ratio_missing() {
        let mut gt = HashMap::new();
        gt.insert(DocItemLabel::Title, 1);
        gt.insert(DocItemLabel::Table, 2);

        let mut rust = HashMap::new();
        rust.insert(DocItemLabel::Title, 1);
        // Missing tables

        let ratio = compute_structural_element_ratio(&gt, &rust);
        // gt_count = 1 + 2 = 3, preserved = min(1,1) + min(2,0) = 1 + 0 = 1
        // ratio = 1/3 ≈ 0.333
        assert!((ratio - 0.333).abs() < 0.01);
    }

    #[test]
    fn test_compare_outputs_identical() {
        use crate::models::{DocumentGroundTruth, ExtractedElement, PageGroundTruth};

        let ground_truth = DocumentGroundTruth {
            filename: "test.pdf".to_string(),
            pages: vec![PageGroundTruth {
                page_number: 1,
                elements: vec![
                    ExtractedElement {
                        label: DocItemLabel::Title,
                        text: "Test Document Title".to_string(),
                        bbox: None,
                        confidence: 1.0,
                        table_data: None,
                    },
                    ExtractedElement {
                        label: DocItemLabel::Paragraph,
                        text: "This is a test paragraph with content.".to_string(),
                        bbox: None,
                        confidence: 1.0,
                        table_data: None,
                    },
                ],
                reading_order: vec![0, 1],
                agreement_scores: vec![1.0, 1.0],
                sources: vec!["test".to_string(), "test".to_string()],
            }],
            total_elements: 2,
            avg_confidence: 1.0,
        };

        // Markdown that matches the ground truth
        let rust_markdown = "# Test Document Title\n\nThis is a test paragraph with content.\n";

        let result = compare_outputs(&ground_truth, rust_markdown);

        // Should have high accuracy since all elements are found
        assert!(
            result.accuracy_percent >= 50.0,
            "Expected high accuracy, got {}",
            result.accuracy_percent
        );
        // Should have reasonable text similarity
        assert!(
            result.text_similarity > 0.0,
            "Expected positive text similarity"
        );
        // Should have structure similarity
        assert!(
            result.structure_similarity > 0.0,
            "Expected positive structure similarity"
        );
        // No missing elements (text is found)
        assert!(
            result.missing_elements.is_empty(),
            "Expected no missing elements"
        );
    }

    #[test]
    fn test_compare_outputs_missing_elements() {
        use crate::models::{DocumentGroundTruth, ExtractedElement, PageGroundTruth};

        let ground_truth = DocumentGroundTruth {
            filename: "test.pdf".to_string(),
            pages: vec![PageGroundTruth {
                page_number: 1,
                elements: vec![
                    ExtractedElement {
                        label: DocItemLabel::Title,
                        text: "Important Title That Should Appear".to_string(),
                        bbox: None,
                        confidence: 1.0,
                        table_data: None,
                    },
                    ExtractedElement {
                        label: DocItemLabel::Paragraph,
                        text: "Critical paragraph content that is missing.".to_string(),
                        bbox: None,
                        confidence: 1.0,
                        table_data: None,
                    },
                ],
                reading_order: vec![0, 1],
                agreement_scores: vec![1.0, 1.0],
                sources: vec!["test".to_string(), "test".to_string()],
            }],
            total_elements: 2,
            avg_confidence: 1.0,
        };

        // Markdown that doesn't contain the expected text
        let rust_markdown = "# Different Title\n\nCompletely different content here.\n";

        let result = compare_outputs(&ground_truth, rust_markdown);

        // Should have low accuracy since elements are missing
        assert!(result.accuracy_percent < 100.0, "Expected lower accuracy");
        // Should have missing elements reported
        assert!(
            !result.missing_elements.is_empty(),
            "Expected missing elements to be reported"
        );
    }
}
