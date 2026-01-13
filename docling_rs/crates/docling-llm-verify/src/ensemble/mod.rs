//! Ensemble merging algorithm for LLM extractions.
//!
//! This module implements the core algorithm for combining document extractions
//! from multiple LLMs into high-confidence ground truth.
//!
//! ## Algorithm Overview
//!
//! 1. **Element Alignment**: Match elements across models using:
//!    - Bounding box `IoU` (Intersection over Union)
//!    - Text similarity (character-level Jaccard)
//!    - Label matching bonus
//!
//! 2. **Majority Voting**: For each aligned group:
//!    - Label: Most common label wins
//!    - Text: Exact match preferred, then 2/3 majority, else longest text
//!    - `BBox`: Average of all bounding boxes
//!
//! 3. **Confidence Scoring**: Combined from:
//!    - Base confidence from individual extractions (50%)
//!    - Text agreement confidence (30%)
//!    - Model agreement ratio (20%)
//!
//! ## Example
//!
//! ```
//! use docling_llm_verify::{merge_extractions, LlmExtractionResult};
//!
//! // Assuming you have extractions from multiple LLMs
//! let extractions: Vec<LlmExtractionResult> = vec![];
//! let ground_truth = merge_extractions(&extractions);
//! ```
//!
//! ## Thresholds
//!
//! - `IOU_THRESHOLD`: 0.5 - Minimum bbox overlap to consider elements matching
//! - Element similarity: 0.5 - Combined threshold for alignment

// Clippy pedantic allows:
// - Percentage/ratio calculations use f64 from usize
// - Short variable names in algorithms (x, y, i, j, etc.)
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::many_single_char_names)]

use crate::models::{
    BBox, DocItemLabel, ExtractedElement, LlmExtractionResult, PageGroundTruth, TableData,
};
use std::collections::HashMap;

/// Minimum `IoU` threshold for considering two elements as matching.
const IOU_THRESHOLD: f64 = 0.5;

/// Merge extractions from multiple LLMs into ground truth.
#[must_use = "merges LLM extractions into ground truth"]
pub fn merge_extractions(extractions: &[LlmExtractionResult]) -> PageGroundTruth {
    if extractions.is_empty() {
        return PageGroundTruth {
            page_number: 0,
            elements: vec![],
            reading_order: vec![],
            agreement_scores: vec![],
            sources: vec![],
        };
    }

    let page_number = extractions[0].page_number;

    // Align elements across models using bbox IoU
    let alignments = align_elements(extractions);

    let mut elements = Vec::new();
    let mut agreement_scores = Vec::new();
    let mut sources = Vec::new();
    let mut reading_order = Vec::new();

    for (i, alignment) in alignments.iter().enumerate() {
        let label = majority_vote_label(alignment);
        let (text, text_confidence) = consensus_text(alignment);
        let bbox = average_bbox(alignment);
        let table_data = merge_table_data(alignment);

        let num_agreeing = alignment.len();
        let total_models = extractions.len();
        let agreement = num_agreeing as f64 / total_models as f64;

        // Average confidence weighted by agreement
        let base_confidence =
            alignment.iter().map(|e| e.confidence).sum::<f64>() / alignment.len() as f64;

        let confidence =
            base_confidence.mul_add(0.5, text_confidence.mul_add(0.3, agreement * 0.2));

        elements.push(ExtractedElement {
            label,
            text,
            bbox,
            confidence,
            table_data,
        });

        agreement_scores.push(agreement);

        let source = alignment
            .iter()
            .map(|_| "ensemble")
            .collect::<Vec<_>>()
            .join(", ");
        sources.push(source);

        reading_order.push(i);
    }

    PageGroundTruth {
        page_number,
        elements,
        reading_order,
        agreement_scores,
        sources,
    }
}

/// Align elements across models by bbox overlap.
fn align_elements(extractions: &[LlmExtractionResult]) -> Vec<Vec<&ExtractedElement>> {
    if extractions.is_empty() {
        return vec![];
    }

    // Use the first model's elements as anchors
    let anchor = &extractions[0].extraction;

    let mut alignments: Vec<Vec<&ExtractedElement>> =
        anchor.elements.iter().map(|e| vec![e]).collect();

    // For each other model, match elements to anchors
    for extraction in extractions.iter().skip(1) {
        let mut used_indices: Vec<bool> = vec![false; extraction.extraction.elements.len()];

        for (anchor_idx, anchor_elem) in anchor.elements.iter().enumerate() {
            let mut best_match: Option<(usize, f64)> = None;

            for (idx, elem) in extraction.extraction.elements.iter().enumerate() {
                if used_indices[idx] {
                    continue;
                }

                let score = element_similarity(anchor_elem, elem);
                if score > IOU_THRESHOLD && best_match.is_none_or(|(_, s)| score > s) {
                    best_match = Some((idx, score));
                }
            }

            if let Some((idx, _)) = best_match {
                alignments[anchor_idx].push(&extraction.extraction.elements[idx]);
                used_indices[idx] = true;
            }
        }

        // Add unmatched elements as new groups
        for (idx, elem) in extraction.extraction.elements.iter().enumerate() {
            if !used_indices[idx] {
                alignments.push(vec![elem]);
            }
        }
    }

    alignments
}

/// Compute similarity between two elements (bbox `IoU` + text similarity).
fn element_similarity(a: &ExtractedElement, b: &ExtractedElement) -> f64 {
    // BBox IoU component
    let bbox_score = match (&a.bbox, &b.bbox) {
        (Some(ba), Some(bb)) => ba.iou(bb),
        _ => 0.5, // Neutral if no bbox
    };

    // Text similarity component
    let text_score = text_similarity(&a.text, &b.text);

    // Label match bonus
    let label_bonus = if a.label == b.label { 0.2 } else { 0.0 };

    (bbox_score * 0.4) + (text_score * 0.4) + label_bonus
}

/// Simple text similarity using character overlap.
fn text_similarity(a: &str, b: &str) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }

    let a_chars: std::collections::HashSet<char> = a.chars().collect();
    let b_chars: std::collections::HashSet<char> = b.chars().collect();

    let intersection = a_chars.intersection(&b_chars).count();
    let union = a_chars.union(&b_chars).count();

    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

/// Majority vote on label.
fn majority_vote_label(elements: &[&ExtractedElement]) -> DocItemLabel {
    let mut counts: HashMap<DocItemLabel, usize> = HashMap::new();

    for elem in elements {
        *counts.entry(elem.label).or_default() += 1;
    }

    counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map_or(DocItemLabel::Text, |(label, _)| label)
}

/// Find consensus text with confidence.
fn consensus_text(elements: &[&ExtractedElement]) -> (String, f64) {
    if elements.is_empty() {
        return (String::new(), 0.0);
    }

    if elements.len() == 1 {
        return (elements[0].text.clone(), 0.8);
    }

    let texts: Vec<&str> = elements.iter().map(|e| e.text.as_str()).collect();

    // Check for exact agreement
    if texts.iter().all(|t| *t == texts[0]) {
        return (texts[0].to_owned(), 1.0);
    }

    // Check for 2/3 majority
    for t in &texts {
        let matches = texts.iter().filter(|x| *x == t).count();
        if matches >= 2 {
            return ((*t).to_owned(), 0.9);
        }
    }

    // No majority - use the longest text (most complete extraction)
    let longest = texts.iter().max_by_key(|t| t.len()).unwrap();
    ((*longest).to_owned(), 0.7)
}

/// Average bounding boxes.
fn average_bbox(elements: &[&ExtractedElement]) -> Option<BBox> {
    let bboxes: Vec<&BBox> = elements.iter().filter_map(|e| e.bbox.as_ref()).collect();

    if bboxes.is_empty() {
        return None;
    }

    let n = bboxes.len() as f64;
    let l = bboxes.iter().map(|b| b.l).sum::<f64>() / n;
    let t = bboxes.iter().map(|b| b.t).sum::<f64>() / n;
    let r = bboxes.iter().map(|b| b.r).sum::<f64>() / n;
    let b = bboxes.iter().map(|b| b.b).sum::<f64>() / n;

    Some(BBox { l, t, r, b })
}

/// Merge table data from multiple extractions.
fn merge_table_data(elements: &[&ExtractedElement]) -> Option<TableData> {
    let tables: Vec<&TableData> = elements
        .iter()
        .filter_map(|e| e.table_data.as_ref())
        .collect();

    if tables.is_empty() {
        return None;
    }

    // Use the table with the most cells
    let best = tables
        .iter()
        .max_by_key(|t| t.num_rows * t.num_cols)
        .unwrap();

    Some(TableData {
        rows: best.rows.clone(),
        num_rows: best.num_rows,
        num_cols: best.num_cols,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_text_similarity() {
        assert_eq!(text_similarity("hello", "hello"), 1.0);
        assert!(text_similarity("hello", "world") < 0.5);
        assert!(text_similarity("hello", "helo") > 0.8);
    }

    #[test]
    fn test_majority_vote_label() {
        let e1 = ExtractedElement {
            label: DocItemLabel::Paragraph,
            text: "test".into(),
            bbox: None,
            confidence: 1.0,
            table_data: None,
        };
        let e2 = ExtractedElement {
            label: DocItemLabel::Paragraph,
            text: "test".into(),
            bbox: None,
            confidence: 1.0,
            table_data: None,
        };
        let e3 = ExtractedElement {
            label: DocItemLabel::Text,
            text: "test".into(),
            bbox: None,
            confidence: 1.0,
            table_data: None,
        };

        let result = majority_vote_label(&[&e1, &e2, &e3]);
        assert_eq!(result, DocItemLabel::Paragraph);
    }
}
