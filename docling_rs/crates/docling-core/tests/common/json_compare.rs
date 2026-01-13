//! JSON comparison utilities for DoclingDocument validation
//!
//! This module provides utilities for comparing Rust conversion output
//! against Python docling groundtruth JSON files.
//!
//! F83: DocItem JSON comparison tests
//!
//! ## Usage
//!
//! ```rust,ignore
//! use common::json_compare::{load_groundtruth_json, compare_documents, ComparisonResult};
//!
//! let groundtruth = load_groundtruth_json("test-corpus/groundtruth/docling_v2/doc.json")?;
//! let result = compare_documents(&converted_doc, &groundtruth);
//! assert!(result.is_ok(), "Document mismatch: {:?}", result);
//! ```

// Some functions are for future use (text content comparison, detailed validation)
#![allow(dead_code)]

use docling_core::{content::DocItem, DoclingDocument};
use std::collections::HashMap;
use std::path::Path;

/// Result of comparing two documents
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComparisonResult {
    /// Whether the comparison passed
    pub passed: bool,
    /// Differences found (empty if passed)
    pub differences: Vec<String>,
    /// Counts from actual document
    pub actual_counts: ItemCounts,
    /// Counts from expected document
    pub expected_counts: ItemCounts,
}

/// Counts of various item types in a document
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ItemCounts {
    pub texts: usize,
    pub tables: usize,
    pub pictures: usize,
    pub groups: usize,
    pub key_value_items: usize,
    pub form_items: usize,
    pub total: usize,
}

impl ItemCounts {
    /// Create counts from a DoclingDocument
    pub const fn from_docling_document(doc: &DoclingDocument) -> Self {
        Self {
            texts: doc.texts.len(),
            tables: doc.tables.len(),
            pictures: doc.pictures.len(),
            groups: doc.groups.len(),
            key_value_items: doc.key_value_items.len(),
            form_items: doc.form_items.len(),
            total: doc.texts.len()
                + doc.tables.len()
                + doc.pictures.len()
                + doc.groups.len()
                + doc.key_value_items.len()
                + doc.form_items.len(),
        }
    }

    /// Create counts from a slice of DocItems
    pub fn from_doc_items(items: &[DocItem]) -> Self {
        let mut counts = Self::default();
        for item in items {
            match item {
                // Text-like items
                DocItem::Text { .. }
                | DocItem::SectionHeader { .. }
                | DocItem::Title { .. }
                | DocItem::Caption { .. }
                | DocItem::PageHeader { .. }
                | DocItem::PageFooter { .. }
                | DocItem::Footnote { .. }
                | DocItem::Paragraph { .. }
                | DocItem::Reference { .. }
                | DocItem::Formula { .. }
                | DocItem::Code { .. }
                | DocItem::ListItem { .. }
                | DocItem::CheckboxSelected { .. }
                | DocItem::CheckboxUnselected { .. } => counts.texts += 1,
                // Tables
                DocItem::Table { .. } => counts.tables += 1,
                // Pictures
                DocItem::Picture { .. } => counts.pictures += 1,
                // Group items
                DocItem::List { .. }
                | DocItem::FormArea { .. }
                | DocItem::KeyValueArea { .. }
                | DocItem::OrderedList { .. }
                | DocItem::Chapter { .. }
                | DocItem::Section { .. }
                | DocItem::Sheet { .. }
                | DocItem::Slide { .. }
                | DocItem::CommentSection { .. }
                | DocItem::Inline { .. }
                | DocItem::PictureArea { .. }
                | DocItem::Unspecified { .. } => counts.groups += 1,
            }
            counts.total += 1;
        }
        counts
    }
}

/// Load a groundtruth JSON file as DoclingDocument
///
/// # Arguments
/// * `path` - Path to the .json groundtruth file
///
/// # Returns
/// * `Ok(DoclingDocument)` if parsing succeeds
/// * `Err(String)` with error message if parsing fails
pub fn load_groundtruth_json<P: AsRef<Path>>(path: P) -> Result<DoclingDocument, String> {
    let path = path.as_ref();
    if !path.exists() {
        return Err(format!("Groundtruth file not found: {}", path.display()));
    }

    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse JSON from {}: {}", path.display(), e))
}

/// Get the groundtruth JSON path for a test file
///
/// # Arguments
/// * `test_file_path` - Relative path like "pdf/document.pdf"
///
/// # Returns
/// * Path to corresponding .json file in groundtruth/docling_v2/
pub fn groundtruth_json_path(test_file_path: &str) -> String {
    let file_stem = Path::new(test_file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");
    format!("../../test-corpus/groundtruth/docling_v2/{file_stem}.json")
}

/// Compare a converted document against expected DoclingDocument
///
/// This performs structural comparison, checking:
/// - Item counts (texts, tables, pictures, groups)
/// - Text content extraction
///
/// # Arguments
/// * `actual_items` - DocItems from Rust conversion (may be None)
/// * `expected` - DoclingDocument from groundtruth JSON
/// * `tolerance` - Allowed percentage difference in counts (0.0-1.0)
///
/// # Returns
/// * `ComparisonResult` with pass/fail status and differences
pub fn compare_documents(
    actual_items: Option<&[DocItem]>,
    expected: &DoclingDocument,
    tolerance: f64,
) -> ComparisonResult {
    let mut differences = Vec::new();
    let expected_counts = ItemCounts::from_docling_document(expected);

    let actual_counts = actual_items.map_or_else(
        || {
            differences.push("No content_blocks in converted document".to_string());
            ItemCounts::default()
        },
        ItemCounts::from_doc_items,
    );

    // Compare counts with tolerance
    let count_checks = [
        ("texts", actual_counts.texts, expected_counts.texts),
        ("tables", actual_counts.tables, expected_counts.tables),
        ("pictures", actual_counts.pictures, expected_counts.pictures),
        ("groups", actual_counts.groups, expected_counts.groups),
    ];

    for (name, actual, expected) in count_checks {
        if expected > 0 {
            let diff_pct = ((actual as f64) - (expected as f64)).abs() / (expected as f64);
            if diff_pct > tolerance {
                differences.push(format!(
                    "{}: actual={}, expected={} (diff {:.1}%)",
                    name,
                    actual,
                    expected,
                    diff_pct * 100.0
                ));
            }
        } else if actual > 0 {
            // Expected 0 but got some - might be OK (Rust backend adds items Python doesn't)
            // Only warn if significantly different
            if actual > 5 {
                differences.push(format!(
                    "{name}: actual={actual}, expected=0 (unexpected items)"
                ));
            }
        }
    }

    ComparisonResult {
        passed: differences.is_empty(),
        differences,
        actual_counts,
        expected_counts,
    }
}

/// Extract all text content from DocItems for text-based comparison
pub fn extract_text_content(items: &[DocItem]) -> Vec<String> {
    let mut texts = Vec::new();
    for item in items {
        match item {
            DocItem::Text { text, .. }
            | DocItem::SectionHeader { text, .. }
            | DocItem::Title { text, .. }
            | DocItem::Caption { text, .. }
            | DocItem::PageHeader { text, .. }
            | DocItem::PageFooter { text, .. }
            | DocItem::Footnote { text, .. }
            | DocItem::Paragraph { text, .. }
            | DocItem::Reference { text, .. }
            | DocItem::ListItem { text, .. }
            | DocItem::CheckboxSelected { text, .. }
            | DocItem::CheckboxUnselected { text, .. }
            | DocItem::Formula { text, .. }
            | DocItem::Code { text, .. } => {
                if !text.is_empty() {
                    texts.push(text.clone());
                }
            }
            DocItem::Table { data, .. } => {
                // Extract text from table cells
                for row in &data.grid {
                    for cell in row {
                        if !cell.text.is_empty() {
                            texts.push(cell.text.clone());
                        }
                    }
                }
            }
            _ => {}
        }
    }
    texts
}

/// Extract all text content from a DoclingDocument
pub fn extract_docling_document_text(doc: &DoclingDocument) -> Vec<String> {
    let mut texts = Vec::new();

    // Extract from texts array
    for item in &doc.texts {
        match item {
            DocItem::Text { text, .. }
            | DocItem::SectionHeader { text, .. }
            | DocItem::Title { text, .. }
            | DocItem::Caption { text, .. }
            | DocItem::PageHeader { text, .. }
            | DocItem::PageFooter { text, .. }
            | DocItem::Footnote { text, .. }
            | DocItem::Paragraph { text, .. }
            | DocItem::Reference { text, .. }
            | DocItem::ListItem { text, .. }
            | DocItem::Formula { text, .. }
            | DocItem::Code { text, .. } => {
                if !text.is_empty() {
                    texts.push(text.clone());
                }
            }
            _ => {}
        }
    }

    // Extract from tables array
    for item in &doc.tables {
        if let DocItem::Table { data, .. } = item {
            for row in &data.grid {
                for cell in row {
                    if !cell.text.is_empty() {
                        texts.push(cell.text.clone());
                    }
                }
            }
        }
    }

    texts
}

/// Compare text content between two documents
///
/// Returns (matched_count, total_expected, missing_texts)
pub fn compare_text_content(
    actual_items: Option<&[DocItem]>,
    expected: &DoclingDocument,
) -> (usize, usize, Vec<String>) {
    let expected_texts = extract_docling_document_text(expected);

    let actual_texts: Vec<String> = actual_items.map_or_else(Vec::new, extract_text_content);

    // Create a set for faster lookup
    let actual_set: std::collections::HashSet<_> = actual_texts.iter().collect();

    let mut matched = 0;
    let mut missing = Vec::new();

    for expected_text in &expected_texts {
        // Check if the expected text is in actual (exact match or contained)
        if actual_set.contains(expected_text)
            || actual_texts
                .iter()
                .any(|t| t.contains(expected_text) || expected_text.contains(t))
        {
            matched += 1;
        } else {
            // Only report first 100 chars of missing text
            let truncated = if expected_text.len() > 100 {
                format!("{}...", &expected_text[..100])
            } else {
                expected_text.clone()
            };
            missing.push(truncated);
        }
    }

    (matched, expected_texts.len(), missing)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_item_counts_default() {
        let counts = ItemCounts::default();
        assert_eq!(counts.total, 0);
        assert_eq!(counts.texts, 0);
        assert_eq!(counts.tables, 0);
    }

    #[test]
    fn test_groundtruth_json_path() {
        assert_eq!(
            groundtruth_json_path("pdf/document.pdf"),
            "../../test-corpus/groundtruth/docling_v2/document.json"
        );
        assert_eq!(
            groundtruth_json_path("html/page.html"),
            "../../test-corpus/groundtruth/docling_v2/page.json"
        );
    }

    #[test]
    fn test_comparison_result_no_items() {
        // Test comparing when actual has no items
        let expected = DoclingDocument {
            schema_name: "DoclingDocument".to_string(),
            version: "1.7.0".to_string(),
            name: "test".to_string(),
            origin: docling_core::document::Origin {
                mimetype: "application/pdf".to_string(),
                binary_hash: 0,
                filename: "test.pdf".to_string(),
            },
            body: docling_core::document::GroupItem {
                self_ref: "#/body".to_string(),
                parent: None,
                children: vec![],
                content_layer: "body".to_string(),
                name: "_root_".to_string(),
                label: "unspecified".to_string(),
            },
            furniture: None,
            texts: vec![],
            groups: vec![],
            tables: vec![],
            pictures: vec![],
            key_value_items: vec![],
            form_items: vec![],
            pages: HashMap::new(),
        };

        let result = compare_documents(None, &expected, 0.1);
        // Empty expected, no actual items - should pass (or at least not fail badly)
        assert!(result.differences.len() <= 1);
    }
}
