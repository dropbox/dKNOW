/// `DoclingDocument` JSON schema structs for validation
///
/// This module defines Rust types that match the `DoclingDocument` v1.8.0 JSON schema
/// from `docling_core`. Used for parsing official Python baseline outputs and comparing
/// with Rust pipeline results.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// `DoclingDocument` schema name
///
/// The official schema name used in `DoclingDocument` JSON output.
pub const DOCLING_DOCUMENT_SCHEMA_NAME: &str = "DoclingDocument";

/// `DoclingDocument` schema version (v1.8.0)
///
/// This constant represents the official schema version that this implementation
/// targets for compatibility with Python Docling baselines.
pub const DOCLING_DOCUMENT_SCHEMA_VERSION: &str = "1.8.0";

/// Coordinate system origin for bounding boxes.
///
/// Specifies whether coordinate values are measured from the top-left
/// or bottom-left corner of the page.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CoordOrigin {
    /// Origin at top-left corner (y increases downward).
    ///
    /// This is the default coordinate system matching image coordinates.
    #[default]
    Topleft,
    /// Origin at bottom-left corner (y increases upward).
    ///
    /// This matches traditional PDF coordinate systems.
    Bottomleft,
}

impl fmt::Display for CoordOrigin {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Topleft => write!(f, "top-left"),
            Self::Bottomleft => write!(f, "bottom-left"),
        }
    }
}

impl std::str::FromStr for CoordOrigin {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Normalize: lowercase and remove hyphens/underscores
        let normalized: String = s
            .to_lowercase()
            .chars()
            .filter(|c| *c != '-' && *c != '_')
            .collect();
        match normalized.as_str() {
            "topleft" | "top" | "tl" => Ok(Self::Topleft),
            "bottomleft" | "bottom" | "bl" => Ok(Self::Bottomleft),
            _ => Err(format!(
                "unknown coord origin: '{s}' (expected: top-left, bottom-left)"
            )),
        }
    }
}

/// Bounding box for document elements.
///
/// Defines a rectangular region using left, top, right, and bottom coordinates.
/// The coordinate interpretation depends on the `coord_origin` field.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BoundingBox {
    /// Left edge x-coordinate.
    pub l: f64,
    /// Top edge y-coordinate.
    pub t: f64,
    /// Right edge x-coordinate.
    pub r: f64,
    /// Bottom edge y-coordinate.
    pub b: f64,
    /// Coordinate system origin for interpreting the coordinates.
    pub coord_origin: CoordOrigin,
}

/// Provenance information linking content to its source location.
///
/// Tracks where a piece of content originated in the source document,
/// including page number, bounding box, and character span.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ProvenanceItem {
    /// Zero-indexed page number where the content appears.
    pub page_no: i32,
    /// Bounding box of the content on the page.
    pub bbox: BoundingBox,
    /// Character span (start, end) in the extracted text.
    pub charspan: (usize, usize),
}

/// JSON reference to another item in the document.
///
/// Uses JSON Pointer notation (RFC 6901) to reference items by path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefItem {
    /// JSON reference path (e.g., `"#/texts/0"`).
    #[serde(rename = "$ref")]
    pub cref: String,
}

/// Content layer classification for document elements.
///
/// Categorizes elements by their role in document structure.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContentLayer {
    /// Main document content (default).
    #[default]
    Body,
    /// Page furniture like headers, footers, and page numbers.
    Furniture,
    /// Background elements (watermarks, decorations).
    Background,
    /// Invisible or hidden content.
    Invisible,
    /// Annotations and notes.
    Notes,
}

impl fmt::Display for ContentLayer {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Body => write!(f, "Body"),
            Self::Furniture => write!(f, "Furniture"),
            Self::Background => write!(f, "Background"),
            Self::Invisible => write!(f, "Invisible"),
            Self::Notes => write!(f, "Notes"),
        }
    }
}

impl std::str::FromStr for ContentLayer {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "body" | "main" | "content" => Ok(Self::Body),
            "furniture" | "page" => Ok(Self::Furniture),
            "background" | "bg" => Ok(Self::Background),
            "invisible" | "hidden" => Ok(Self::Invisible),
            "notes" | "annotation" | "annotations" => Ok(Self::Notes),
            _ => Err(format!(
                "unknown content layer: '{s}' (expected: body, furniture, background, invisible, notes)"
            )),
        }
    }
}

/// Document item label indicating the semantic type of content.
///
/// These labels are assigned by the ML layout detection model and
/// determine how content is processed and serialized.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocItemLabel {
    /// Document title (main heading).
    Title,
    /// Table of contents or document index.
    DocumentIndex,
    /// Section or chapter heading.
    SectionHeader,
    /// Regular paragraph text.
    Paragraph,
    /// Tabular data.
    Table,
    /// Image or figure.
    Picture,
    /// Chart or graph visualization.
    Chart,
    /// Mathematical formula or equation.
    Formula,
    /// Source code or code snippet.
    Code,
    /// Selected/checked checkbox.
    CheckboxSelected,
    /// Unselected/unchecked checkbox.
    CheckboxUnselected,
    /// Generic text (fallback label).
    Text,
    /// Item in a list (bulleted or numbered).
    ListItem,
    /// Reference or citation.
    Reference,
    /// Page header (repeating content at top).
    PageHeader,
    /// Page footer (repeating content at bottom).
    PageFooter,
    /// Caption for a figure or table.
    Caption,
    /// Footnote text.
    Footnote,
    /// Key-value pair region (forms, metadata).
    KeyValueRegion,
    /// Form field or input area.
    Form,
    /// Empty or unspecified label.
    #[serde(rename = "")]
    EmptyValue,
}

impl fmt::Display for DocItemLabel {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Title => write!(f, "Title"),
            Self::DocumentIndex => write!(f, "Document Index"),
            Self::SectionHeader => write!(f, "Section Header"),
            Self::Paragraph => write!(f, "Paragraph"),
            Self::Table => write!(f, "Table"),
            Self::Picture => write!(f, "Picture"),
            Self::Chart => write!(f, "Chart"),
            Self::Formula => write!(f, "Formula"),
            Self::Code => write!(f, "Code"),
            Self::CheckboxSelected => write!(f, "Checkbox (Selected)"),
            Self::CheckboxUnselected => write!(f, "Checkbox (Unselected)"),
            Self::Text => write!(f, "Text"),
            Self::ListItem => write!(f, "List Item"),
            Self::Reference => write!(f, "Reference"),
            Self::PageHeader => write!(f, "Page Header"),
            Self::PageFooter => write!(f, "Page Footer"),
            Self::Caption => write!(f, "Caption"),
            Self::Footnote => write!(f, "Footnote"),
            Self::KeyValueRegion => write!(f, "Key-Value Region"),
            Self::Form => write!(f, "Form"),
            Self::EmptyValue => write!(f, "(empty)"),
        }
    }
}

impl std::str::FromStr for DocItemLabel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Normalize: remove hyphens/underscores, lowercase
        let normalized: String = s
            .chars()
            .filter(|c| *c != '-' && *c != '_' && *c != ' ')
            .collect::<String>()
            .to_lowercase();

        match normalized.as_str() {
            "title" => Ok(Self::Title),
            "documentindex" | "docindex" | "index" => Ok(Self::DocumentIndex),
            "sectionheader" | "section" | "header" => Ok(Self::SectionHeader),
            "paragraph" | "para" | "p" => Ok(Self::Paragraph),
            "table" | "tbl" => Ok(Self::Table),
            "picture" | "image" | "img" | "pic" => Ok(Self::Picture),
            "chart" | "graph" => Ok(Self::Chart),
            "formula" | "math" | "equation" => Ok(Self::Formula),
            "code" | "codeblock" => Ok(Self::Code),
            "checkboxselected" | "checked" | "selected" => Ok(Self::CheckboxSelected),
            "checkboxunselected" | "unchecked" | "unselected" => Ok(Self::CheckboxUnselected),
            "text" | "txt" => Ok(Self::Text),
            "listitem" | "li" | "item" => Ok(Self::ListItem),
            "reference" | "ref" | "citation" => Ok(Self::Reference),
            "pageheader" | "pagehead" => Ok(Self::PageHeader),
            "pagefooter" | "pagefoot" => Ok(Self::PageFooter),
            "caption" | "cap" | "figcaption" => Ok(Self::Caption),
            "footnote" | "note" => Ok(Self::Footnote),
            "keyvalueregion" | "keyvalue" | "kv" | "kvregion" => Ok(Self::KeyValueRegion),
            "form" | "formfield" => Ok(Self::Form),
            "" | "empty" | "emptyvalue" | "(empty)" => Ok(Self::EmptyValue),
            _ => Err(format!("Unknown DocItemLabel: '{s}'")),
        }
    }
}

/// Group label indicating the type of content grouping.
///
/// Groups organize related document items into logical units
/// (e.g., list items into a list, key-value pairs into a region).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroupLabel {
    /// Unspecified or unknown group type (default).
    #[default]
    Unspecified,
    /// Unordered (bulleted) list.
    List,
    /// Ordered (numbered) list.
    OrderedList,
    /// Inline grouping (text that flows together).
    Inline,
    /// Key-value pair region (metadata, forms).
    KeyValueArea,
    /// Form input area.
    FormArea,
}

impl fmt::Display for GroupLabel {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unspecified => write!(f, "Unspecified"),
            Self::List => write!(f, "List"),
            Self::OrderedList => write!(f, "Ordered List"),
            Self::Inline => write!(f, "Inline"),
            Self::KeyValueArea => write!(f, "Key-Value Area"),
            Self::FormArea => write!(f, "Form Area"),
        }
    }
}

impl std::str::FromStr for GroupLabel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Normalize: remove hyphens/underscores, lowercase
        let normalized: String = s
            .chars()
            .filter(|c| *c != '-' && *c != '_' && *c != ' ')
            .collect::<String>()
            .to_lowercase();

        match normalized.as_str() {
            "unspecified" | "unknown" | "none" | "" => Ok(Self::Unspecified),
            "list" | "ul" | "unorderedlist" => Ok(Self::List),
            "orderedlist" | "ol" | "numbered" | "numlist" => Ok(Self::OrderedList),
            "inline" | "span" => Ok(Self::Inline),
            "keyvaluearea" | "kvarea" | "keyvalue" | "kv" => Ok(Self::KeyValueArea),
            "formarea" | "form" => Ok(Self::FormArea),
            _ => Err(format!("Unknown GroupLabel: '{s}'")),
        }
    }
}

/// Base structure for all document tree nodes.
///
/// Provides the tree hierarchy through parent/children references
/// and tracks content layer classification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeItemBase {
    /// JSON Pointer reference to this item (e.g., `"#/texts/0"`).
    pub self_ref: String,
    /// Reference to parent node (None for root items).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<RefItem>,
    /// References to child nodes.
    #[serde(default)]
    pub children: Vec<RefItem>,
    /// Content layer classification (body, furniture, etc.).
    pub content_layer: ContentLayer,
}

/// Container for grouping related document items.
///
/// Groups organize items like list items or key-value pairs
/// into logical units.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupItem {
    /// Base node properties (refs, layer).
    #[serde(flatten)]
    pub base: NodeItemBase,
    /// Group name/identifier.
    pub name: String,
    /// Type of group (`list`, `ordered_list`, etc.).
    pub label: GroupLabel,
}

/// Text content item with semantic label.
///
/// Represents extracted text with classification (title, paragraph, etc.)
/// and optional metadata for specific label types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextItem {
    /// Base node properties (refs, layer).
    #[serde(flatten)]
    pub base: NodeItemBase,
    /// Semantic label (`title`, `paragraph`, `section_header`, etc.).
    pub label: DocItemLabel,
    /// Original extracted text (before normalization).
    pub orig: String,
    /// Normalized/cleaned text.
    pub text: String,
    /// Provenance information linking to source locations.
    #[serde(default)]
    pub prov: Vec<ProvenanceItem>,
    /// Heading level (1-6) for section headers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<i32>,
    /// Whether list item is enumerated (numbered vs bulleted).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enumerated: Option<bool>,
    /// List item marker (e.g., "•", "1.", "a)").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub marker: Option<String>,
    /// Programming language for code blocks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_language: Option<String>,
    /// N=4378: Whether text is bold (from PDF font flags).
    #[serde(default)]
    pub is_bold: bool,
    /// N=4378: Whether text is italic (from PDF font flags).
    #[serde(default)]
    pub is_italic: bool,
}

/// Individual cell within a table.
///
/// Contains cell content, span information, and header classification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableCell {
    /// Cell text content.
    pub text: String,
    /// Number of rows this cell spans.
    pub row_span: i32,
    /// Number of columns this cell spans.
    pub col_span: i32,
    /// Starting row index (0-based).
    pub start_row_offset_idx: i32,
    /// Ending row index (exclusive).
    pub end_row_offset_idx: i32,
    /// Starting column index (0-based).
    pub start_col_offset_idx: i32,
    /// Ending column index (exclusive).
    pub end_col_offset_idx: i32,
    /// Whether this cell is a column header.
    #[serde(default)]
    pub column_header: bool,
    /// Whether this cell is a row header.
    #[serde(default)]
    pub row_header: bool,
    /// Bounding box of the cell on the page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bbox: Option<BoundingBox>,
    /// Whether text was extracted via OCR.
    #[serde(default)]
    pub from_ocr: bool,
    /// OCR confidence score (0.0-1.0).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
}

/// Table structure data containing dimensions and cells.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableData {
    /// Total number of rows in the table.
    pub num_rows: i32,
    /// Total number of columns in the table.
    pub num_cols: i32,
    /// All cells in the table (row-major order).
    #[serde(default)]
    pub table_cells: Vec<TableCell>,
}

/// Complete table item with structure and provenance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableItem {
    /// Base node properties (refs, layer).
    #[serde(flatten)]
    pub base: NodeItemBase,
    /// Semantic label (always `Table`).
    pub label: DocItemLabel,
    /// Table structure and cell data.
    pub data: TableData,
    /// Provenance information linking to source locations.
    #[serde(default)]
    pub prov: Vec<ProvenanceItem>,
    /// References to caption items.
    #[serde(default)]
    pub captions: Vec<RefItem>,
}

/// Image or figure item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PictureItem {
    /// Base node properties (refs, layer).
    #[serde(flatten)]
    pub base: NodeItemBase,
    /// Semantic label (Picture, Chart, etc.).
    pub label: DocItemLabel,
    /// Provenance information linking to source locations.
    #[serde(default)]
    pub prov: Vec<ProvenanceItem>,
    /// References to caption items.
    #[serde(default)]
    pub captions: Vec<RefItem>,
    /// Additional annotations (generic JSON).
    #[serde(default)]
    pub annotations: Vec<serde_json::Value>,
    /// OCR text extracted from within the figure/picture content.
    /// This captures text from charts, diagrams, graphs, etc.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ocr_text: Option<String>,
}

/// Page dimensions in points.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Size {
    /// Page width in points.
    pub width: f64,
    /// Page height in points.
    pub height: f64,
}

/// Page metadata for a single document page.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PageItem {
    /// Zero-indexed page number.
    pub page_no: i32,
    /// Page dimensions.
    pub size: Size,
}

/// Source document origin metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentOrigin {
    /// MIME type of the source document.
    pub mimetype: String,
    /// Hash of the source document binary.
    pub binary_hash: u64,
    /// Original filename.
    pub filename: String,
    /// Optional URI/URL of the source.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
}

/// Complete `DoclingDocument` representing extracted document content.
///
/// This is the top-level structure containing all extracted content
/// organized into a tree hierarchy with typed items.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DoclingDocument {
    /// Schema name (typically `DoclingDocument`).
    pub schema_name: String,
    /// Schema version (e.g., "1.8.0").
    pub version: String,
    /// Document name/title.
    pub name: String,
    /// Source document metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<DocumentOrigin>,

    /// Main body content tree.
    pub body: GroupItem,
    /// Page furniture (headers, footers, page numbers).
    #[serde(default)]
    pub furniture: Option<GroupItem>,

    /// Additional content groups.
    #[serde(default)]
    pub groups: Vec<GroupItem>,
    /// All extracted text items.
    #[serde(default)]
    pub texts: Vec<TextItem>,
    /// All extracted tables.
    #[serde(default)]
    pub tables: Vec<TableItem>,
    /// All extracted images/figures.
    #[serde(default)]
    pub pictures: Vec<PictureItem>,
    /// Key-value items (generic JSON for extensibility).
    #[serde(default)]
    pub key_value_items: Vec<serde_json::Value>,
    /// Form items (generic JSON for extensibility).
    #[serde(default)]
    pub form_items: Vec<serde_json::Value>,

    /// Page metadata keyed by page number string.
    #[serde(default)]
    pub pages: HashMap<String, PageItem>,

    /// Total number of pages in the document.
    #[serde(default)]
    pub num_pages: i32,
    /// Pre-rendered markdown (if available).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<String>,
}

impl DoclingDocument {
    /// Load `DoclingDocument` from JSON file
    #[must_use = "this function returns a document that should be processed"]
    pub fn from_json_file(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let json_str = std::fs::read_to_string(path)?;
        let doc: Self = serde_json::from_str(&json_str)?;
        Ok(doc)
    }

    /// Get all text items in reading order from body
    #[inline]
    #[must_use = "returns the text items in reading order"]
    pub fn get_text_items_in_order(&self) -> Vec<&TextItem> {
        // Simple traversal - flatten all texts (can be improved with reading order)
        self.texts.iter().collect()
    }

    /// Get total character count across all text items
    #[inline]
    #[must_use = "returns the total character count"]
    pub fn get_total_char_count(&self) -> usize {
        self.texts.iter().map(|t| t.text.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use log::trace;
    use std::path::PathBuf;

    #[test]
    fn test_coord_origin_display() {
        assert_eq!(CoordOrigin::Topleft.to_string(), "top-left");
        assert_eq!(CoordOrigin::Bottomleft.to_string(), "bottom-left");
    }

    #[test]
    fn test_coord_origin_from_str() {
        use std::str::FromStr;

        // Primary names
        assert_eq!(
            CoordOrigin::from_str("top-left").unwrap(),
            CoordOrigin::Topleft
        );
        assert_eq!(
            CoordOrigin::from_str("bottom-left").unwrap(),
            CoordOrigin::Bottomleft
        );

        // Aliases
        assert_eq!(
            CoordOrigin::from_str("topleft").unwrap(),
            CoordOrigin::Topleft
        );
        assert_eq!(
            CoordOrigin::from_str("top_left").unwrap(),
            CoordOrigin::Topleft
        );
        assert_eq!(CoordOrigin::from_str("top").unwrap(), CoordOrigin::Topleft);
        assert_eq!(CoordOrigin::from_str("tl").unwrap(), CoordOrigin::Topleft);
        assert_eq!(
            CoordOrigin::from_str("bottom").unwrap(),
            CoordOrigin::Bottomleft
        );
        assert_eq!(
            CoordOrigin::from_str("bl").unwrap(),
            CoordOrigin::Bottomleft
        );

        // Case insensitive
        assert_eq!(
            CoordOrigin::from_str("TOP-LEFT").unwrap(),
            CoordOrigin::Topleft
        );

        // Error case
        assert!(CoordOrigin::from_str("center").is_err());
    }

    #[test]
    fn test_coord_origin_roundtrip() {
        use std::str::FromStr;

        for origin in [CoordOrigin::Topleft, CoordOrigin::Bottomleft] {
            let s = origin.to_string();
            let parsed = CoordOrigin::from_str(&s).unwrap();
            assert_eq!(origin, parsed, "roundtrip failed for {s}");
        }
    }

    #[test]
    fn test_content_layer_from_str() {
        use std::str::FromStr;

        // Primary names
        assert_eq!(ContentLayer::from_str("body").unwrap(), ContentLayer::Body);
        assert_eq!(
            ContentLayer::from_str("furniture").unwrap(),
            ContentLayer::Furniture
        );
        assert_eq!(
            ContentLayer::from_str("background").unwrap(),
            ContentLayer::Background
        );
        assert_eq!(
            ContentLayer::from_str("invisible").unwrap(),
            ContentLayer::Invisible
        );
        assert_eq!(
            ContentLayer::from_str("notes").unwrap(),
            ContentLayer::Notes
        );

        // Aliases
        assert_eq!(ContentLayer::from_str("main").unwrap(), ContentLayer::Body);
        assert_eq!(
            ContentLayer::from_str("bg").unwrap(),
            ContentLayer::Background
        );
        assert_eq!(
            ContentLayer::from_str("hidden").unwrap(),
            ContentLayer::Invisible
        );
        assert_eq!(
            ContentLayer::from_str("annotation").unwrap(),
            ContentLayer::Notes
        );

        // Case insensitive
        assert_eq!(ContentLayer::from_str("BODY").unwrap(), ContentLayer::Body);

        // Error case
        assert!(ContentLayer::from_str("unknown").is_err());
    }

    #[test]
    fn test_content_layer_roundtrip() {
        use std::str::FromStr;

        for layer in [
            ContentLayer::Body,
            ContentLayer::Furniture,
            ContentLayer::Background,
            ContentLayer::Invisible,
            ContentLayer::Notes,
        ] {
            let s = layer.to_string();
            let parsed = ContentLayer::from_str(&s).unwrap();
            assert_eq!(layer, parsed, "roundtrip failed for {s}");
        }
    }

    #[test]
    fn test_parse_code_and_formula_baseline() {
        let baseline_path =
            PathBuf::from("baseline_data/code_and_formula/official_docling_document.json");

        if !baseline_path.exists() {
            trace!("Skipping test: baseline file not found at {baseline_path:?}");
            return;
        }

        let doc = DoclingDocument::from_json_file(&baseline_path)
            .expect("Failed to parse code_and_formula baseline");

        // Basic validation
        assert_eq!(doc.schema_name, DOCLING_DOCUMENT_SCHEMA_NAME);
        assert_eq!(doc.version, DOCLING_DOCUMENT_SCHEMA_VERSION);
        // Note: Old baseline files don't have num_pages field, skip validation

        // Check texts
        assert_eq!(doc.texts.len(), 16, "Expected 16 text items");

        // Check first text item
        let first_text = &doc.texts[0];
        assert_eq!(first_text.label, DocItemLabel::SectionHeader);
        assert_eq!(first_text.text, "JavaScript Code Example");

        trace!("✓ Successfully parsed code_and_formula baseline");
        trace!("  Texts: {}", doc.texts.len());
        trace!("  Tables: {}", doc.tables.len());
        trace!("  Pictures: {}", doc.pictures.len());
        trace!("  Pages: {}", doc.pages.len());
    }

    #[test]
    fn test_parse_all_baselines() {
        let baselines = vec![
            (
                "arxiv",
                "baseline_data/arxiv_2206.01062/official_docling_document.json",
            ),
            (
                "code_and_formula",
                "baseline_data/code_and_formula/official_docling_document.json",
            ),
            (
                "edinet",
                "baseline_data/edinet_sample/official_docling_document.json",
            ),
            (
                "jfk",
                "baseline_data/jfk_scanned/official_docling_document.json",
            ),
        ];

        for (name, path) in baselines {
            let baseline_path = PathBuf::from(path);

            if !baseline_path.exists() {
                trace!("Skipping {name}: baseline file not found");
                continue;
            }

            match DoclingDocument::from_json_file(&baseline_path) {
                Ok(doc) => {
                    trace!("✓ {name} baseline parsed successfully");
                    trace!("  Version: {}", doc.version);
                    trace!("  Pages: {} (0 if old baseline)", doc.num_pages);
                    trace!("  Texts: {}", doc.texts.len());
                    trace!("  Tables: {}", doc.tables.len());
                    trace!("  Pictures: {}", doc.pictures.len());
                }
                Err(e) => {
                    panic!("Failed to parse {name} baseline: {e}");
                }
            }
        }
    }

    #[test]
    fn test_content_layer_display() {
        assert_eq!(ContentLayer::Body.to_string(), "Body");
        assert_eq!(ContentLayer::Furniture.to_string(), "Furniture");
        assert_eq!(ContentLayer::Background.to_string(), "Background");
        assert_eq!(ContentLayer::Invisible.to_string(), "Invisible");
        assert_eq!(ContentLayer::Notes.to_string(), "Notes");
    }

    #[test]
    fn test_doc_item_label_display() {
        assert_eq!(DocItemLabel::Title.to_string(), "Title");
        assert_eq!(DocItemLabel::DocumentIndex.to_string(), "Document Index");
        assert_eq!(DocItemLabel::SectionHeader.to_string(), "Section Header");
        assert_eq!(DocItemLabel::Paragraph.to_string(), "Paragraph");
        assert_eq!(DocItemLabel::Table.to_string(), "Table");
        assert_eq!(DocItemLabel::Picture.to_string(), "Picture");
        assert_eq!(DocItemLabel::Chart.to_string(), "Chart");
        assert_eq!(DocItemLabel::Formula.to_string(), "Formula");
        assert_eq!(DocItemLabel::Code.to_string(), "Code");
        assert_eq!(
            DocItemLabel::CheckboxSelected.to_string(),
            "Checkbox (Selected)"
        );
        assert_eq!(
            DocItemLabel::CheckboxUnselected.to_string(),
            "Checkbox (Unselected)"
        );
        assert_eq!(DocItemLabel::Text.to_string(), "Text");
        assert_eq!(DocItemLabel::ListItem.to_string(), "List Item");
        assert_eq!(DocItemLabel::Reference.to_string(), "Reference");
        assert_eq!(DocItemLabel::PageHeader.to_string(), "Page Header");
        assert_eq!(DocItemLabel::PageFooter.to_string(), "Page Footer");
        assert_eq!(DocItemLabel::Caption.to_string(), "Caption");
        assert_eq!(DocItemLabel::Footnote.to_string(), "Footnote");
        assert_eq!(DocItemLabel::KeyValueRegion.to_string(), "Key-Value Region");
        assert_eq!(DocItemLabel::Form.to_string(), "Form");
        assert_eq!(DocItemLabel::EmptyValue.to_string(), "(empty)");
    }

    #[test]
    fn test_group_label_display() {
        assert_eq!(GroupLabel::Unspecified.to_string(), "Unspecified");
        assert_eq!(GroupLabel::List.to_string(), "List");
        assert_eq!(GroupLabel::OrderedList.to_string(), "Ordered List");
        assert_eq!(GroupLabel::Inline.to_string(), "Inline");
        assert_eq!(GroupLabel::KeyValueArea.to_string(), "Key-Value Area");
        assert_eq!(GroupLabel::FormArea.to_string(), "Form Area");
    }

    #[test]
    fn test_doc_item_label_from_str() {
        use std::str::FromStr;

        // Basic variants (lowercase)
        assert_eq!(
            DocItemLabel::from_str("title").unwrap(),
            DocItemLabel::Title
        );
        assert_eq!(
            DocItemLabel::from_str("table").unwrap(),
            DocItemLabel::Table
        );
        assert_eq!(
            DocItemLabel::from_str("paragraph").unwrap(),
            DocItemLabel::Paragraph
        );
        assert_eq!(
            DocItemLabel::from_str("picture").unwrap(),
            DocItemLabel::Picture
        );
        assert_eq!(DocItemLabel::from_str("code").unwrap(), DocItemLabel::Code);
        assert_eq!(DocItemLabel::from_str("text").unwrap(), DocItemLabel::Text);
        assert_eq!(
            DocItemLabel::from_str("caption").unwrap(),
            DocItemLabel::Caption
        );
        assert_eq!(
            DocItemLabel::from_str("footnote").unwrap(),
            DocItemLabel::Footnote
        );
        assert_eq!(DocItemLabel::from_str("form").unwrap(), DocItemLabel::Form);

        // Case insensitive
        assert_eq!(
            DocItemLabel::from_str("TITLE").unwrap(),
            DocItemLabel::Title
        );
        assert_eq!(
            DocItemLabel::from_str("Table").unwrap(),
            DocItemLabel::Table
        );

        // With separators (removed during normalization)
        assert_eq!(
            DocItemLabel::from_str("document_index").unwrap(),
            DocItemLabel::DocumentIndex
        );
        assert_eq!(
            DocItemLabel::from_str("section-header").unwrap(),
            DocItemLabel::SectionHeader
        );
        assert_eq!(
            DocItemLabel::from_str("list item").unwrap(),
            DocItemLabel::ListItem
        );
        assert_eq!(
            DocItemLabel::from_str("page_header").unwrap(),
            DocItemLabel::PageHeader
        );
        assert_eq!(
            DocItemLabel::from_str("page-footer").unwrap(),
            DocItemLabel::PageFooter
        );
        assert_eq!(
            DocItemLabel::from_str("key_value_region").unwrap(),
            DocItemLabel::KeyValueRegion
        );

        // Checkbox variants
        assert_eq!(
            DocItemLabel::from_str("checkbox_selected").unwrap(),
            DocItemLabel::CheckboxSelected
        );
        assert_eq!(
            DocItemLabel::from_str("checkbox-unselected").unwrap(),
            DocItemLabel::CheckboxUnselected
        );
        assert_eq!(
            DocItemLabel::from_str("checked").unwrap(),
            DocItemLabel::CheckboxSelected
        );
        assert_eq!(
            DocItemLabel::from_str("unchecked").unwrap(),
            DocItemLabel::CheckboxUnselected
        );

        // Aliases
        assert_eq!(
            DocItemLabel::from_str("para").unwrap(),
            DocItemLabel::Paragraph
        );
        assert_eq!(
            DocItemLabel::from_str("p").unwrap(),
            DocItemLabel::Paragraph
        );
        assert_eq!(
            DocItemLabel::from_str("img").unwrap(),
            DocItemLabel::Picture
        );
        assert_eq!(
            DocItemLabel::from_str("image").unwrap(),
            DocItemLabel::Picture
        );
        assert_eq!(
            DocItemLabel::from_str("math").unwrap(),
            DocItemLabel::Formula
        );
        assert_eq!(
            DocItemLabel::from_str("li").unwrap(),
            DocItemLabel::ListItem
        );
        assert_eq!(
            DocItemLabel::from_str("ref").unwrap(),
            DocItemLabel::Reference
        );
        assert_eq!(
            DocItemLabel::from_str("kv").unwrap(),
            DocItemLabel::KeyValueRegion
        );

        // Empty value
        assert_eq!(
            DocItemLabel::from_str("").unwrap(),
            DocItemLabel::EmptyValue
        );
        assert_eq!(
            DocItemLabel::from_str("empty").unwrap(),
            DocItemLabel::EmptyValue
        );

        // Invalid
        assert!(DocItemLabel::from_str("invalid_label").is_err());
        assert!(DocItemLabel::from_str("xyz").is_err());
    }

    #[test]
    fn test_doc_item_label_roundtrip() {
        use std::str::FromStr;

        // Test that all variants can roundtrip via snake_case format
        let variants = [
            ("title", DocItemLabel::Title),
            ("document_index", DocItemLabel::DocumentIndex),
            ("section_header", DocItemLabel::SectionHeader),
            ("paragraph", DocItemLabel::Paragraph),
            ("table", DocItemLabel::Table),
            ("picture", DocItemLabel::Picture),
            ("chart", DocItemLabel::Chart),
            ("formula", DocItemLabel::Formula),
            ("code", DocItemLabel::Code),
            ("checkbox_selected", DocItemLabel::CheckboxSelected),
            ("checkbox_unselected", DocItemLabel::CheckboxUnselected),
            ("text", DocItemLabel::Text),
            ("list_item", DocItemLabel::ListItem),
            ("reference", DocItemLabel::Reference),
            ("page_header", DocItemLabel::PageHeader),
            ("page_footer", DocItemLabel::PageFooter),
            ("caption", DocItemLabel::Caption),
            ("footnote", DocItemLabel::Footnote),
            ("key_value_region", DocItemLabel::KeyValueRegion),
            ("form", DocItemLabel::Form),
        ];

        for (s, expected) in variants {
            let parsed = DocItemLabel::from_str(s)
                .unwrap_or_else(|_| panic!("Failed to parse DocItemLabel '{s}'"));
            assert_eq!(parsed, expected, "Mismatch for '{s}'");
        }
    }

    #[test]
    fn test_group_label_from_str() {
        use std::str::FromStr;

        // Basic variants (lowercase)
        assert_eq!(
            GroupLabel::from_str("unspecified").unwrap(),
            GroupLabel::Unspecified
        );
        assert_eq!(GroupLabel::from_str("list").unwrap(), GroupLabel::List);
        assert_eq!(GroupLabel::from_str("inline").unwrap(), GroupLabel::Inline);

        // Case insensitive
        assert_eq!(GroupLabel::from_str("LIST").unwrap(), GroupLabel::List);
        assert_eq!(GroupLabel::from_str("Inline").unwrap(), GroupLabel::Inline);

        // With separators (removed during normalization)
        assert_eq!(
            GroupLabel::from_str("ordered_list").unwrap(),
            GroupLabel::OrderedList
        );
        assert_eq!(
            GroupLabel::from_str("key-value-area").unwrap(),
            GroupLabel::KeyValueArea
        );
        assert_eq!(
            GroupLabel::from_str("form area").unwrap(),
            GroupLabel::FormArea
        );

        // Aliases
        assert_eq!(
            GroupLabel::from_str("unknown").unwrap(),
            GroupLabel::Unspecified
        );
        assert_eq!(
            GroupLabel::from_str("none").unwrap(),
            GroupLabel::Unspecified
        );
        assert_eq!(GroupLabel::from_str("").unwrap(), GroupLabel::Unspecified);
        assert_eq!(GroupLabel::from_str("ul").unwrap(), GroupLabel::List);
        assert_eq!(GroupLabel::from_str("ol").unwrap(), GroupLabel::OrderedList);
        assert_eq!(
            GroupLabel::from_str("numbered").unwrap(),
            GroupLabel::OrderedList
        );
        assert_eq!(GroupLabel::from_str("span").unwrap(), GroupLabel::Inline);
        assert_eq!(
            GroupLabel::from_str("kv").unwrap(),
            GroupLabel::KeyValueArea
        );
        assert_eq!(GroupLabel::from_str("form").unwrap(), GroupLabel::FormArea);

        // Invalid
        assert!(GroupLabel::from_str("invalid_label").is_err());
        assert!(GroupLabel::from_str("xyz").is_err());
    }

    #[test]
    fn test_group_label_roundtrip() {
        use std::str::FromStr;

        // Test that all variants can roundtrip via snake_case format
        let variants = [
            ("unspecified", GroupLabel::Unspecified),
            ("list", GroupLabel::List),
            ("ordered_list", GroupLabel::OrderedList),
            ("inline", GroupLabel::Inline),
            ("key_value_area", GroupLabel::KeyValueArea),
            ("form_area", GroupLabel::FormArea),
        ];

        for (s, expected) in variants {
            let parsed = GroupLabel::from_str(s)
                .unwrap_or_else(|_| panic!("Failed to parse GroupLabel '{s}'"));
            assert_eq!(parsed, expected, "Mismatch for '{s}'");
        }
    }
}
