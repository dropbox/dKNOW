//! Data models for LLM extraction and comparison.
//!
//! This module defines the core data structures used throughout the crate:
//!
//! - [`BBox`] - Bounding box coordinates for element location
//! - [`DocItemLabel`] - Element type classification (title, paragraph, table, etc.)
//! - [`ExtractedElement`] - A single extracted document element
//! - [`PageExtraction`] - Extraction results for one page
//! - [`PageGroundTruth`] - Ensemble-merged ground truth for one page
//! - [`DocumentGroundTruth`] - Complete document ground truth
//! - [`LlmExtractionResult`] - Single LLM extraction with metadata (tokens, cost, latency)
//! - [`ComparisonResult`] - Comparison between Rust output and ground truth
//!
//! ## LLM Client Submodules
//!
//! - [`openai`] - `OpenAI` API client for GPT-4o and o1 models
//! - [`bedrock`] - AWS Bedrock client for Claude models

pub mod bedrock;
pub mod openai;

use serde::{Deserialize, Serialize};

/// Bounding box in PDF coordinates (origin bottom-left).
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
pub struct BBox {
    /// Left edge x-coordinate
    pub l: f64,
    /// Top edge y-coordinate
    pub t: f64,
    /// Right edge x-coordinate
    pub r: f64,
    /// Bottom edge y-coordinate
    pub b: f64,
}

impl BBox {
    /// Calculate the area of the bounding box
    #[inline]
    #[must_use = "returns bounding box area"]
    pub fn area(&self) -> f64 {
        (self.r - self.l).abs() * (self.t - self.b).abs()
    }

    /// Compute intersection over union with another bbox.
    #[inline]
    #[must_use = "computes intersection over union"]
    pub fn iou(&self, other: &Self) -> f64 {
        let x_left = self.l.max(other.l);
        let y_bottom = self.b.max(other.b);
        let x_right = self.r.min(other.r);
        let y_top = self.t.min(other.t);

        if x_right < x_left || y_top < y_bottom {
            return 0.0;
        }

        let intersection = (x_right - x_left) * (y_top - y_bottom);
        let union = self.area() + other.area() - intersection;

        if union == 0.0 {
            0.0
        } else {
            intersection / union
        }
    }
}

/// Element labels matching Docling's `DocItem` types.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DocItemLabel {
    /// Document title (main heading)
    Title,
    /// Section or subsection header
    SectionHeader,
    /// Body text paragraph
    Paragraph,
    /// Generic text content (default)
    #[default]
    Text,
    /// List item (bulleted or numbered)
    ListItem,
    /// Tabular data
    Table,
    /// Image or figure
    Picture,
    /// Figure or table caption
    Caption,
    /// Footnote text
    Footnote,
    /// Mathematical formula or equation
    Formula,
    /// Running page header
    PageHeader,
    /// Running page footer
    PageFooter,
    /// Source code block
    Code,
    /// Interactive checkbox element
    Checkbox,
    /// Bibliographic reference or citation
    Reference,
}

impl std::fmt::Display for DocItemLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Title => write!(f, "title"),
            Self::SectionHeader => write!(f, "section_header"),
            Self::Paragraph => write!(f, "paragraph"),
            Self::Text => write!(f, "text"),
            Self::ListItem => write!(f, "list_item"),
            Self::Table => write!(f, "table"),
            Self::Picture => write!(f, "picture"),
            Self::Caption => write!(f, "caption"),
            Self::Footnote => write!(f, "footnote"),
            Self::Formula => write!(f, "formula"),
            Self::PageHeader => write!(f, "page_header"),
            Self::PageFooter => write!(f, "page_footer"),
            Self::Code => write!(f, "code"),
            Self::Checkbox => write!(f, "checkbox"),
            Self::Reference => write!(f, "reference"),
        }
    }
}

/// Table data for table elements.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TableData {
    /// Table contents as a 2D grid of cell strings
    pub rows: Vec<Vec<String>>,
    /// Number of rows in the table
    pub num_rows: usize,
    /// Number of columns in the table
    pub num_cols: usize,
}

/// A single extracted element from a page.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ExtractedElement {
    /// Element type classification
    pub label: DocItemLabel,
    /// Text content of the element
    pub text: String,
    /// Bounding box location on page (if available)
    pub bbox: Option<BBox>,
    /// Extraction confidence score (0.0-1.0)
    pub confidence: f64,
    /// Table-specific data (for table elements only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_data: Option<TableData>,
}

/// Extraction result for a single page.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PageExtraction {
    /// 1-based page number
    pub page_number: u32,
    /// Extracted elements on this page
    pub elements: Vec<ExtractedElement>,
    /// Indices defining element reading order
    pub reading_order: Vec<usize>,
}

/// Ground truth with confidence for a single page.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PageGroundTruth {
    /// 1-based page number
    pub page_number: u32,
    /// Ground truth elements on this page
    pub elements: Vec<ExtractedElement>,
    /// Indices defining element reading order
    pub reading_order: Vec<usize>,
    /// How many LLMs agreed on each element
    pub agreement_scores: Vec<f64>,
    /// Source model for each element (for debugging)
    pub sources: Vec<String>,
}

/// Full document ground truth.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DocumentGroundTruth {
    /// Source document filename
    pub filename: String,
    /// Ground truth for each page
    pub pages: Vec<PageGroundTruth>,
    /// Total element count across all pages
    pub total_elements: usize,
    /// Average confidence score across all elements
    pub avg_confidence: f64,
}

/// LLM extraction result with metadata.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LlmExtractionResult {
    /// Model identifier (e.g., "gpt-4o", "claude-3-opus")
    pub model: String,
    /// 1-based page number that was processed
    pub page_number: u32,
    /// Extracted page content
    pub extraction: PageExtraction,
    /// Number of input tokens consumed
    pub input_tokens: u32,
    /// Number of output tokens generated
    pub output_tokens: u32,
    /// Cost in USD for this extraction
    pub cost_usd: f64,
    /// Processing latency in milliseconds
    pub latency_ms: u64,
}

/// Cost tracking for a full PDF.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PdfCostReport {
    /// Source PDF filename
    pub pdf_name: String,
    /// Number of pages processed
    pub num_pages: usize,
    /// Per-model cost breakdown
    pub model_costs: std::collections::HashMap<String, ModelCost>,
    /// Total cost across all models
    pub total_cost_usd: f64,
}

/// Token and cost breakdown for a single model.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ModelCost {
    /// Total input tokens consumed
    pub input_tokens: u32,
    /// Total output tokens generated
    pub output_tokens: u32,
    /// Total cost in USD
    pub cost_usd: f64,
}

/// Comparison result between Rust output and ground truth.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ComparisonResult {
    /// Source document filename
    pub filename: String,
    /// Overall accuracy percentage (0-100)
    pub accuracy_percent: f64,
    /// Text content similarity score (0.0-1.0)
    pub text_similarity: f64,
    /// Document structure similarity score (0.0-1.0)
    pub structure_similarity: f64,
    /// Elements present in ground truth but missing from output
    pub missing_elements: Vec<String>,
    /// Elements present in output but not in ground truth
    pub extra_elements: Vec<String>,
    /// Elements with incorrect label classification
    pub label_mismatches: Vec<LabelMismatch>,
}

/// A mismatch between expected and actual element labels.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LabelMismatch {
    /// Truncated text content for identification
    pub text_preview: String,
    /// Label from ground truth
    pub expected_label: DocItemLabel,
    /// Label from Rust output
    pub actual_label: DocItemLabel,
}
