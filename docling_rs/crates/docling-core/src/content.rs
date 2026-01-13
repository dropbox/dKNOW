//! Content block types for structured document representation
//!
//! This module defines the content block types that represent the structured
//! content of a document, matching Python's `docling_core.types.doc` types.

use serde::{Deserialize, Serialize};

/// Label for content items, matching Python's `DocItemLabel` enum
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentLabel {
    /// Regular paragraph text
    Paragraph,
    /// Section header/heading
    SectionHeader,
    /// Title (document or section)
    Title,
    /// Table
    Table,
    /// Picture/image
    Picture,
    /// Chart/graph
    Chart,
    /// List item
    ListItem,
    /// Code block
    Code,
    /// Caption (for tables, figures)
    Caption,
    /// Footnote
    Footnote,
    /// Formula/equation
    Formula,
    /// Page header
    PageHeader,
    /// Page footer
    PageFooter,
    /// Reference/citation
    Reference,
    /// Form element
    Form,
    /// Checkbox (selected)
    CheckboxSelected,
    /// Checkbox (unselected)
    CheckboxUnselected,
    /// Key-value region
    KeyValueRegion,
    /// Generic text (default)
    #[default]
    Text,
}

impl std::fmt::Display for ContentLabel {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Paragraph => "paragraph",
            Self::SectionHeader => "section_header",
            Self::Title => "title",
            Self::Table => "table",
            Self::Picture => "picture",
            Self::Chart => "chart",
            Self::ListItem => "list_item",
            Self::Code => "code",
            Self::Caption => "caption",
            Self::Footnote => "footnote",
            Self::Formula => "formula",
            Self::PageHeader => "page_header",
            Self::PageFooter => "page_footer",
            Self::Reference => "reference",
            Self::Form => "form",
            Self::CheckboxSelected => "checkbox_selected",
            Self::CheckboxUnselected => "checkbox_unselected",
            Self::KeyValueRegion => "key_value_region",
            Self::Text => "text",
        };
        write!(f, "{s}")
    }
}

impl std::str::FromStr for ContentLabel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "paragraph" => Ok(Self::Paragraph),
            "section_header" | "sectionheader" | "section-header" => Ok(Self::SectionHeader),
            "title" => Ok(Self::Title),
            "table" => Ok(Self::Table),
            "picture" => Ok(Self::Picture),
            "chart" => Ok(Self::Chart),
            "list_item" | "listitem" | "list-item" => Ok(Self::ListItem),
            "code" => Ok(Self::Code),
            "caption" => Ok(Self::Caption),
            "footnote" => Ok(Self::Footnote),
            "formula" => Ok(Self::Formula),
            "page_header" | "pageheader" | "page-header" => Ok(Self::PageHeader),
            "page_footer" | "pagefooter" | "page-footer" => Ok(Self::PageFooter),
            "reference" => Ok(Self::Reference),
            "form" => Ok(Self::Form),
            "checkbox_selected" | "checkboxselected" | "checkbox-selected" => {
                Ok(Self::CheckboxSelected)
            }
            "checkbox_unselected" | "checkboxunselected" | "checkbox-unselected" => {
                Ok(Self::CheckboxUnselected)
            }
            "key_value_region" | "keyvalueregion" | "key-value-region" => Ok(Self::KeyValueRegion),
            "text" => Ok(Self::Text),
            _ => Err(format!("unknown content label: '{s}'")),
        }
    }
}

/// Coordinate origin for bounding boxes
///
/// Defaults to `TopLeft` (standard screen coordinate system).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CoordOrigin {
    /// Origin at bottom-left (PDF coordinate system)
    #[serde(rename = "BOTTOMLEFT")]
    BottomLeft,
    /// Origin at top-left (most image formats)
    #[default]
    #[serde(rename = "TOPLEFT")]
    TopLeft,
}

impl std::fmt::Display for CoordOrigin {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::BottomLeft => "bottom_left",
            Self::TopLeft => "top_left",
        };
        write!(f, "{s}")
    }
}

impl std::str::FromStr for CoordOrigin {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "bottom_left" | "bottomleft" | "bottom-left" => Ok(Self::BottomLeft),
            "top_left" | "topleft" | "top-left" => Ok(Self::TopLeft),
            _ => Err(format!("unknown coordinate origin: '{s}'")),
        }
    }
}

// =============================================================================
// PDF Page Dimension Constants
// =============================================================================

/// PDF points per inch (PostScript standard: 1 inch = 72 points).
pub const PDF_POINTS_PER_INCH: f64 = 72.0;

/// US Letter page width in PDF points (8.5 inches × 72 dpi = 612 points).
pub const US_LETTER_WIDTH: f64 = 612.0;

/// US Letter page height in PDF points (11 inches × 72 dpi = 792 points).
pub const US_LETTER_HEIGHT: f64 = 792.0;

/// A4 page width in PDF points (210mm × 72 / 25.4 ≈ 595 points).
pub const A4_WIDTH: f64 = 595.0;

/// A4 page height in PDF points (297mm × 72 / 25.4 ≈ 842 points).
pub const A4_HEIGHT: f64 = 842.0;

/// Size dimensions
/// Python: Size in `docling_core.types.doc.base`
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Size {
    /// Width in points (default: 0.0)
    pub width: f64,
    /// Height in points (default: 0.0)
    pub height: f64,
}

impl Size {
    /// US Letter page size (8.5" × 11" = 612 × 792 points).
    pub const US_LETTER: Self = Self::new(US_LETTER_WIDTH, US_LETTER_HEIGHT);

    /// A4 page size (210mm × 297mm ≈ 595 × 842 points).
    pub const A4: Self = Self::new(A4_WIDTH, A4_HEIGHT);

    /// Creates a new Size with the given width and height.
    ///
    /// # Arguments
    ///
    /// * `width` - Width in points
    /// * `height` - Height in points
    #[inline]
    #[must_use = "creates a new Size with width and height"]
    pub const fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }

    /// Returns the size as a tuple (width, height).
    ///
    /// # Returns
    ///
    /// Tuple of (width, height) in points.
    #[inline]
    #[must_use = "returns the size as a tuple (width, height)"]
    pub const fn as_tuple(&self) -> (f64, f64) {
        (self.width, self.height)
    }
}

/// Bounding box coordinates matching Python's format
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct BoundingBox {
    /// Left coordinate
    pub l: f64,
    /// Top coordinate
    pub t: f64,
    /// Right coordinate
    pub r: f64,
    /// Bottom coordinate
    pub b: f64,
    /// Coordinate origin (BOTTOMLEFT or TOPLEFT)
    pub coord_origin: CoordOrigin,
}

impl BoundingBox {
    /// Creates a new `BoundingBox` with the given coordinates.
    ///
    /// # Arguments
    ///
    /// * `l` - Left coordinate
    /// * `t` - Top coordinate
    /// * `r` - Right coordinate
    /// * `b` - Bottom coordinate
    /// * `coord_origin` - Coordinate system origin (BOTTOMLEFT or TOPLEFT)
    #[inline]
    #[must_use = "creates a new BoundingBox with coordinates"]
    pub const fn new(l: f64, t: f64, r: f64, b: f64, coord_origin: CoordOrigin) -> Self {
        Self {
            l,
            t,
            r,
            b,
            coord_origin,
        }
    }

    /// Returns the width of the bounding box.
    ///
    /// # Returns
    ///
    /// Width in points (right - left).
    #[inline]
    #[must_use = "returns the width of the bounding box"]
    pub const fn width(&self) -> f64 {
        self.r - self.l
    }

    /// Returns the height of the bounding box.
    ///
    /// # Returns
    ///
    /// Height in points (absolute value of top - bottom).
    #[inline]
    #[must_use = "returns the height of the bounding box"]
    pub const fn height(&self) -> f64 {
        // Note: f64::abs() is not const, so we use manual absolute value
        let diff = self.t - self.b;
        if diff < 0.0 {
            -diff
        } else {
            diff
        }
    }

    /// Returns the area of the bounding box.
    ///
    /// # Returns
    ///
    /// Area in square points (width × height).
    #[inline]
    #[must_use = "returns the area of the bounding box"]
    pub const fn area(&self) -> f64 {
        self.width() * self.height()
    }

    /// Creates a `BoundingBox` from tuple coordinates.
    ///
    /// Converts from tuple format (left, bottom, right, top) used in Python.
    ///
    /// # Arguments
    ///
    /// * `tuple` - Coordinates as (left, bottom, right, top)
    /// * `coord_origin` - Coordinate system origin
    ///
    /// # Returns
    ///
    /// `BoundingBox` with the given coordinates.
    ///
    /// Python equivalent: `BoundingBox.from_tuple()` in `docling_core/types/doc/base.py`
    #[inline]
    #[must_use = "creates a BoundingBox from tuple coordinates"]
    pub const fn from_tuple(tuple: (f64, f64, f64, f64), coord_origin: CoordOrigin) -> Self {
        let (l, b, r, t) = tuple;
        Self::new(l, t, r, b, coord_origin)
    }

    /// Converts the bounding box to tuple format.
    ///
    /// Returns coordinates in the format (left, bottom, right, top) used in Python.
    ///
    /// # Returns
    ///
    /// Tuple of (left, bottom, right, top) coordinates.
    ///
    /// Python equivalent: `as_tuple()` in `docling_core/types/doc/base.py`
    #[inline]
    #[must_use = "converts the bounding box to tuple format"]
    pub const fn as_tuple(&self) -> (f64, f64, f64, f64) {
        (self.l, self.b, self.r, self.t)
    }

    /// Converts bounding box to top-left origin coordinate system.
    ///
    /// Transforms coordinates from bottom-left origin to top-left origin if needed.
    /// If already in top-left origin, returns a copy unchanged.
    ///
    /// # Arguments
    ///
    /// * `page_height` - Height of the page for coordinate transformation
    ///
    /// # Returns
    ///
    /// `BoundingBox` with coordinates in top-left origin system.
    ///
    /// Python equivalent: `to_top_left_origin()` in `docling_core/types/doc/base.py`
    #[inline]
    #[must_use = "converts bounding box to top-left origin"]
    pub fn to_top_left_origin(&self, page_height: f64) -> Self {
        if self.coord_origin == CoordOrigin::TopLeft {
            return *self;
        }

        Self {
            l: self.l,
            t: page_height - self.t,
            r: self.r,
            b: page_height - self.b,
            coord_origin: CoordOrigin::TopLeft,
        }
    }

    /// Converts bounding box to bottom-left origin coordinate system.
    ///
    /// Transforms coordinates from top-left origin to bottom-left origin if needed.
    /// If already in bottom-left origin, returns a copy unchanged.
    ///
    /// # Arguments
    ///
    /// * `page_height` - Height of the page for coordinate transformation
    ///
    /// # Returns
    ///
    /// `BoundingBox` with coordinates in bottom-left origin system.
    ///
    /// Python equivalent: `to_bottom_left_origin()` in `docling_core/types/doc/base.py`
    #[inline]
    #[must_use = "converts bounding box to bottom-left origin"]
    pub fn to_bottom_left_origin(&self, page_height: f64) -> Self {
        if self.coord_origin == CoordOrigin::BottomLeft {
            return *self;
        }

        Self {
            l: self.l,
            t: page_height - self.t,
            r: self.r,
            b: page_height - self.b,
            coord_origin: CoordOrigin::BottomLeft,
        }
    }
}

/// JSON pointer reference (e.g., {"$ref": "#/texts/0"})
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ItemRef {
    /// JSON pointer path to the referenced item (e.g., "#/texts/0")
    #[serde(rename = "$ref")]
    pub ref_path: String,
}

impl ItemRef {
    /// Creates a new `ItemRef` with the given reference path.
    ///
    /// # Arguments
    ///
    /// * `ref_path` - JSON pointer reference path (e.g., "#/texts/0")
    ///
    /// # Returns
    ///
    /// New `ItemRef` with the given path.
    #[inline]
    #[must_use = "creates a new ItemRef with the given path"]
    pub fn new(ref_path: impl Into<String>) -> Self {
        Self {
            ref_path: ref_path.into(),
        }
    }
}

/// Provenance information for content (page, bbox, charspan)
/// Matches Python's `ProvenanceItem`
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ProvenanceItem {
    /// Page number (1-indexed in Python)
    pub page_no: usize,
    /// Bounding box
    pub bbox: BoundingBox,
    /// Character span [start, end]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub charspan: Option<Vec<usize>>,
}

impl ProvenanceItem {
    /// Creates a default `ProvenanceItem` for the given page number and coordinate origin.
    ///
    /// This is the standard helper for creating placeholder provenance when exact
    /// coordinates are not available (e.g., for metadata, document-level content).
    /// Uses a unit bounding box of (0,0)-(1,1).
    ///
    /// # Arguments
    ///
    /// * `page_no` - The page number (0-indexed internally, displayed as 1-indexed)
    /// * `coord_origin` - The coordinate origin (`TopLeft` or `BottomLeft`)
    #[inline]
    #[must_use = "creates default provenance for a page"]
    pub const fn default_for_page(page_no: usize, coord_origin: CoordOrigin) -> Self {
        Self {
            page_no,
            bbox: BoundingBox::new(0.0, 0.0, 1.0, 1.0, coord_origin),
            charspan: None,
        }
    }
}

/// Text formatting information (default: all fields None)
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Formatting {
    /// Whether text is bold
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
    /// Whether text is italic
    #[serde(skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
    /// Whether text is underlined
    #[serde(skip_serializing_if = "Option::is_none")]
    pub underline: Option<bool>,
    /// Whether text has strikethrough
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strikethrough: Option<bool>,
    /// Whether text is inline code (rendered with backticks in markdown)
    /// Applies to HTML: `<code>`, `<kbd>`, `<samp>`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<bool>,
    /// Script type (superscript, subscript, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
    /// Font size in points
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f32>,
    /// Font family name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
}

/// Table cell data
///
/// Supports both simple cells (text only) and rich cells (with ref to group/list).
/// When ref is present (`RichTableCell` in Python), the cell contains structured content
/// like lists that should be serialized recursively rather than using the collapsed text.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TableCell {
    /// Cell text content
    pub text: String,
    /// Number of rows this cell spans
    #[serde(skip_serializing_if = "Option::is_none")]
    pub row_span: Option<usize>,
    /// Number of columns this cell spans
    #[serde(skip_serializing_if = "Option::is_none")]
    pub col_span: Option<usize>,
    /// Reference to rich content (e.g., list group)
    /// Corresponds to Python's RichTableCell.ref field
    #[serde(skip_serializing_if = "Option::is_none", rename = "ref")]
    pub ref_item: Option<ItemRef>,
    /// Row/col indices for reconstructing grid from flat `table_cells` list
    /// Starting row index in the table grid
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_row_offset_idx: Option<usize>,
    /// Starting column index in the table grid
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_col_offset_idx: Option<usize>,
    /// Whether this cell is a column header (ched tag in `TableFormer`)
    /// Used by markdown serializer to render header row distinctly
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub column_header: bool,
    /// Whether this cell is a row header (rhed tag in `TableFormer`)
    /// Used by markdown serializer to identify row header columns
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub row_header: bool,
    /// Whether the text came from OCR (indicates potential lower confidence)
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub from_ocr: bool,
    /// OCR confidence score (0.0 to 1.0) for cells from OCR
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
    /// Cell bounding box (Issue #2: Cell-level provenance)
    /// Preserves spatial location for debugging and visualization
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bbox: Option<BoundingBox>,
}

/// Table data structure
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TableData {
    /// Number of rows in the table
    pub num_rows: usize,
    /// Number of columns in the table
    pub num_cols: usize,
    /// Grid of cells (row-major order) - refs are stripped in this view
    pub grid: Vec<Vec<TableCell>>,
    /// Flat list of table cells with rich content refs preserved
    /// Python creates grid from `table_cells`, stripping refs for convenience
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_cells: Option<Vec<TableCell>>,
}

/// `DocItem` - structured content item matching Python's `DocItem`
///
/// This represents items in the texts/groups/tables/pictures arrays
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "label", rename_all = "snake_case")]
pub enum DocItem {
    /// Regular text
    Text {
        /// Self-reference path (e.g., "#/texts/0")
        self_ref: String,
        /// Reference to parent item
        #[serde(skip_serializing_if = "Option::is_none")]
        parent: Option<ItemRef>,
        /// References to child items
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        children: Vec<ItemRef>,
        /// Content layer identifier (e.g., "body", "furniture")
        content_layer: String,
        /// Provenance information (page, bbox, charspan)
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        prov: Vec<ProvenanceItem>,
        /// Original text
        orig: String,
        /// Sanitized text
        text: String,
        /// Text formatting (bold, italic, etc.)
        #[serde(skip_serializing_if = "Option::is_none", default)]
        formatting: Option<Formatting>,
        /// Hyperlink URL if this text is a link
        #[serde(skip_serializing_if = "Option::is_none", default)]
        hyperlink: Option<String>,
    },

    /// Section header
    #[serde(rename = "section_header")]
    SectionHeader {
        /// Self-reference path (e.g., "#/texts/0")
        self_ref: String,
        /// Reference to parent item
        #[serde(skip_serializing_if = "Option::is_none")]
        parent: Option<ItemRef>,
        /// References to child items
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        children: Vec<ItemRef>,
        /// Content layer identifier (e.g., "body", "furniture")
        content_layer: String,
        /// Provenance information (page, bbox, charspan)
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        prov: Vec<ProvenanceItem>,
        /// Original text
        orig: String,
        /// Sanitized text
        text: String,
        /// Heading level (1-100)
        level: usize,
        /// Text formatting (bold, italic, etc.)
        #[serde(skip_serializing_if = "Option::is_none", default)]
        formatting: Option<Formatting>,
        /// Hyperlink URL if this text is a link
        #[serde(skip_serializing_if = "Option::is_none", default)]
        hyperlink: Option<String>,
    },

    /// List item
    #[serde(rename = "list_item")]
    ListItem {
        /// Self-reference path (e.g., "#/texts/0")
        self_ref: String,
        /// Reference to parent item
        #[serde(skip_serializing_if = "Option::is_none")]
        parent: Option<ItemRef>,
        /// References to child items
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        children: Vec<ItemRef>,
        /// Content layer identifier (e.g., "body", "furniture")
        content_layer: String,
        /// Provenance information (page, bbox, charspan)
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        prov: Vec<ProvenanceItem>,
        /// Original text
        orig: String,
        /// Sanitized text
        text: String,
        /// Whether this is enumerated (default: false = unordered/bulleted list)
        #[serde(default)]
        enumerated: bool,
        /// List marker (default: empty string)
        #[serde(default)]
        marker: String,
        /// Text formatting (bold, italic, etc.)
        #[serde(skip_serializing_if = "Option::is_none", default)]
        formatting: Option<Formatting>,
        /// Hyperlink URL if this text is a link
        #[serde(skip_serializing_if = "Option::is_none", default)]
        hyperlink: Option<String>,
    },

    /// List group
    List {
        /// Self-reference path (e.g., "#/groups/0")
        self_ref: String,
        /// Reference to parent item
        #[serde(skip_serializing_if = "Option::is_none")]
        parent: Option<ItemRef>,
        /// References to child items
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        children: Vec<ItemRef>,
        /// Content layer identifier (e.g., "body", "furniture")
        content_layer: String,
        /// Group name/identifier
        name: String,
    },

    /// Form area group (generic group with form fields)
    #[serde(rename = "form_area")]
    FormArea {
        /// Self-reference path (e.g., "#/groups/0")
        self_ref: String,
        /// Reference to parent item
        #[serde(skip_serializing_if = "Option::is_none")]
        parent: Option<ItemRef>,
        /// References to child items
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        children: Vec<ItemRef>,
        /// Content layer identifier (e.g., "body", "furniture")
        content_layer: String,
        /// Group name/identifier
        name: String,
    },

    /// Key-value area group
    #[serde(rename = "key_value_area")]
    KeyValueArea {
        /// Self-reference path (e.g., "#/groups/0")
        self_ref: String,
        /// Reference to parent item
        #[serde(skip_serializing_if = "Option::is_none")]
        parent: Option<ItemRef>,
        /// References to child items
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        children: Vec<ItemRef>,
        /// Content layer identifier (e.g., "body", "furniture")
        content_layer: String,
        /// Group name/identifier
        name: String,
    },

    /// Ordered list group
    #[serde(rename = "ordered_list")]
    OrderedList {
        /// Self-reference path (e.g., "#/groups/0")
        self_ref: String,
        /// Reference to parent item
        #[serde(skip_serializing_if = "Option::is_none")]
        parent: Option<ItemRef>,
        /// References to child items
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        children: Vec<ItemRef>,
        /// Content layer identifier (e.g., "body", "furniture")
        content_layer: String,
        /// Group name/identifier
        name: String,
    },

    /// Chapter group
    Chapter {
        /// Self-reference path (e.g., "#/groups/0")
        self_ref: String,
        /// Reference to parent item
        #[serde(skip_serializing_if = "Option::is_none")]
        parent: Option<ItemRef>,
        /// References to child items
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        children: Vec<ItemRef>,
        /// Content layer identifier (e.g., "body", "furniture")
        content_layer: String,
        /// Group name/identifier
        name: String,
    },

    /// Section group
    Section {
        /// Self-reference path (e.g., "#/groups/0")
        self_ref: String,
        /// Reference to parent item
        #[serde(skip_serializing_if = "Option::is_none")]
        parent: Option<ItemRef>,
        /// References to child items
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        children: Vec<ItemRef>,
        /// Content layer identifier (e.g., "body", "furniture")
        content_layer: String,
        /// Group name/identifier
        name: String,
    },

    /// Sheet group (for spreadsheets)
    Sheet {
        /// Self-reference path (e.g., "#/groups/0")
        self_ref: String,
        /// Reference to parent item
        #[serde(skip_serializing_if = "Option::is_none")]
        parent: Option<ItemRef>,
        /// References to child items
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        children: Vec<ItemRef>,
        /// Content layer identifier (e.g., "body", "furniture")
        content_layer: String,
        /// Group name/identifier
        name: String,
    },

    /// Slide group (for presentations)
    Slide {
        /// Self-reference path (e.g., "#/groups/0")
        self_ref: String,
        /// Reference to parent item
        #[serde(skip_serializing_if = "Option::is_none")]
        parent: Option<ItemRef>,
        /// References to child items
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        children: Vec<ItemRef>,
        /// Content layer identifier (e.g., "body", "furniture")
        content_layer: String,
        /// Group name/identifier
        name: String,
    },

    /// Comment section group
    #[serde(rename = "comment_section")]
    CommentSection {
        /// Self-reference path (e.g., "#/groups/0")
        self_ref: String,
        /// Reference to parent item
        #[serde(skip_serializing_if = "Option::is_none")]
        parent: Option<ItemRef>,
        /// References to child items
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        children: Vec<ItemRef>,
        /// Content layer identifier (e.g., "body", "furniture")
        content_layer: String,
        /// Group name/identifier
        name: String,
    },

    /// Inline group (for inline content grouping)
    Inline {
        /// Self-reference path (e.g., "#/groups/0")
        self_ref: String,
        /// Reference to parent item
        #[serde(skip_serializing_if = "Option::is_none")]
        parent: Option<ItemRef>,
        /// References to child items
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        children: Vec<ItemRef>,
        /// Content layer identifier (e.g., "body", "furniture")
        content_layer: String,
        /// Group name/identifier
        name: String,
    },

    /// Picture area group
    #[serde(rename = "picture_area")]
    PictureArea {
        /// Self-reference path (e.g., "#/groups/0")
        self_ref: String,
        /// Reference to parent item
        #[serde(skip_serializing_if = "Option::is_none")]
        parent: Option<ItemRef>,
        /// References to child items
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        children: Vec<ItemRef>,
        /// Content layer identifier (e.g., "body", "furniture")
        content_layer: String,
        /// Group name/identifier
        name: String,
    },

    /// Unspecified group
    Unspecified {
        /// Self-reference path (e.g., "#/groups/0")
        self_ref: String,
        /// Reference to parent item
        #[serde(skip_serializing_if = "Option::is_none")]
        parent: Option<ItemRef>,
        /// References to child items
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        children: Vec<ItemRef>,
        /// Content layer identifier (e.g., "body", "furniture")
        content_layer: String,
        /// Group name/identifier
        name: String,
    },

    /// Table (also handles `document_index` with table structure)
    #[serde(alias = "document_index")]
    Table {
        /// Self-reference path (e.g., "#/tables/0")
        self_ref: String,
        /// Reference to parent item
        #[serde(skip_serializing_if = "Option::is_none")]
        parent: Option<ItemRef>,
        /// References to child items
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        children: Vec<ItemRef>,
        /// Content layer identifier (e.g., "body", "furniture")
        content_layer: String,
        /// Provenance information (page, bbox, charspan)
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        prov: Vec<ProvenanceItem>,
        /// Table data (cells, dimensions)
        data: TableData,
        /// References to caption items
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        captions: Vec<ItemRef>,
        /// References to footnote items
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        footnotes: Vec<ItemRef>,
        /// References to reference items
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        references: Vec<ItemRef>,
        /// Image representation of the table
        #[serde(skip_serializing_if = "Option::is_none")]
        image: Option<serde_json::Value>, // Generic for now
        /// Table annotations (e.g., bounding boxes for cells)
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        annotations: Vec<serde_json::Value>, // Generic for now
    },

    /// Picture
    Picture {
        /// Self-reference path (e.g., "#/pictures/0")
        self_ref: String,
        /// Reference to parent item
        #[serde(skip_serializing_if = "Option::is_none")]
        parent: Option<ItemRef>,
        /// References to child items
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        children: Vec<ItemRef>,
        /// Content layer identifier (e.g., "body", "furniture")
        content_layer: String,
        /// Provenance information (page, bbox, charspan)
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        prov: Vec<ProvenanceItem>,
        /// References to caption items
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        captions: Vec<ItemRef>,
        /// References to footnote items
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        footnotes: Vec<ItemRef>,
        /// References to reference items
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        references: Vec<ItemRef>,
        /// Image data (embedded or reference)
        #[serde(skip_serializing_if = "Option::is_none")]
        image: Option<serde_json::Value>, // Generic for now
        /// Picture annotations (e.g., object detection results)
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        annotations: Vec<serde_json::Value>, // Generic for now
        /// OCR text extracted from the figure/picture content
        /// This captures text that appears within charts, diagrams, graphs, etc.
        /// Rendered as `<!-- figure-ocr: ... -->` in markdown output
        #[serde(skip_serializing_if = "Option::is_none")]
        ocr_text: Option<String>,
    },

    /// Page footer
    #[serde(rename = "page_footer")]
    PageFooter {
        /// Self-reference path (e.g., "#/texts/0")
        self_ref: String,
        /// Reference to parent item
        #[serde(skip_serializing_if = "Option::is_none")]
        parent: Option<ItemRef>,
        /// References to child items
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        children: Vec<ItemRef>,
        /// Content layer identifier (e.g., "body", "furniture")
        content_layer: String,
        /// Provenance information (page, bbox, charspan)
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        prov: Vec<ProvenanceItem>,
        /// Original text
        orig: String,
        /// Sanitized text
        text: String,
        /// Text formatting (bold, italic, etc.)
        #[serde(skip_serializing_if = "Option::is_none", default)]
        formatting: Option<Formatting>,
        /// Hyperlink URL if this text is a link
        #[serde(skip_serializing_if = "Option::is_none", default)]
        hyperlink: Option<String>,
    },

    /// Page header
    #[serde(rename = "page_header")]
    PageHeader {
        /// Self-reference path (e.g., "#/texts/0")
        self_ref: String,
        /// Reference to parent item
        #[serde(skip_serializing_if = "Option::is_none")]
        parent: Option<ItemRef>,
        /// References to child items
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        children: Vec<ItemRef>,
        /// Content layer identifier (e.g., "body", "furniture")
        content_layer: String,
        /// Provenance information (page, bbox, charspan)
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        prov: Vec<ProvenanceItem>,
        /// Original text
        orig: String,
        /// Sanitized text
        text: String,
        /// Text formatting (bold, italic, etc.)
        #[serde(skip_serializing_if = "Option::is_none", default)]
        formatting: Option<Formatting>,
        /// Hyperlink URL if this text is a link
        #[serde(skip_serializing_if = "Option::is_none", default)]
        hyperlink: Option<String>,
    },

    /// Paragraph
    Paragraph {
        /// Self-reference path (e.g., "#/texts/0")
        self_ref: String,
        /// Reference to parent item
        #[serde(skip_serializing_if = "Option::is_none")]
        parent: Option<ItemRef>,
        /// References to child items
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        children: Vec<ItemRef>,
        /// Content layer identifier (e.g., "body", "furniture")
        content_layer: String,
        /// Provenance information (page, bbox, charspan)
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        prov: Vec<ProvenanceItem>,
        /// Original text
        orig: String,
        /// Sanitized text
        text: String,
        /// Text formatting (bold, italic, etc.)
        #[serde(skip_serializing_if = "Option::is_none", default)]
        formatting: Option<Formatting>,
        /// Hyperlink URL if this text is a link
        #[serde(skip_serializing_if = "Option::is_none", default)]
        hyperlink: Option<String>,
    },

    /// Caption
    Caption {
        /// Self-reference path (e.g., "#/texts/0")
        self_ref: String,
        /// Reference to parent item
        #[serde(skip_serializing_if = "Option::is_none")]
        parent: Option<ItemRef>,
        /// References to child items
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        children: Vec<ItemRef>,
        /// Content layer identifier (e.g., "body", "furniture")
        content_layer: String,
        /// Provenance information (page, bbox, charspan)
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        prov: Vec<ProvenanceItem>,
        /// Original text
        orig: String,
        /// Sanitized text
        text: String,
        /// Text formatting (bold, italic, etc.)
        #[serde(skip_serializing_if = "Option::is_none", default)]
        formatting: Option<Formatting>,
        /// Hyperlink URL if this text is a link
        #[serde(skip_serializing_if = "Option::is_none", default)]
        hyperlink: Option<String>,
    },

    /// Title
    Title {
        /// Self-reference path (e.g., "#/texts/0")
        self_ref: String,
        /// Reference to parent item
        #[serde(skip_serializing_if = "Option::is_none")]
        parent: Option<ItemRef>,
        /// References to child items
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        children: Vec<ItemRef>,
        /// Content layer identifier (e.g., "body", "furniture")
        content_layer: String,
        /// Provenance information (page, bbox, charspan)
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        prov: Vec<ProvenanceItem>,
        /// Original text
        orig: String,
        /// Sanitized text
        text: String,
        /// Text formatting (bold, italic, etc.)
        #[serde(skip_serializing_if = "Option::is_none", default)]
        formatting: Option<Formatting>,
        /// Hyperlink URL if this text is a link
        #[serde(skip_serializing_if = "Option::is_none", default)]
        hyperlink: Option<String>,
    },

    /// Footnote
    Footnote {
        /// Self-reference path (e.g., "#/texts/0")
        self_ref: String,
        /// Reference to parent item
        #[serde(skip_serializing_if = "Option::is_none")]
        parent: Option<ItemRef>,
        /// References to child items
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        children: Vec<ItemRef>,
        /// Content layer identifier (e.g., "body", "furniture")
        content_layer: String,
        /// Provenance information (page, bbox, charspan)
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        prov: Vec<ProvenanceItem>,
        /// Original text
        orig: String,
        /// Sanitized text
        text: String,
        /// Text formatting (bold, italic, etc.)
        #[serde(skip_serializing_if = "Option::is_none", default)]
        formatting: Option<Formatting>,
        /// Hyperlink URL if this text is a link
        #[serde(skip_serializing_if = "Option::is_none", default)]
        hyperlink: Option<String>,
    },

    /// Reference
    Reference {
        /// Self-reference path (e.g., "#/texts/0")
        self_ref: String,
        /// Reference to parent item
        #[serde(skip_serializing_if = "Option::is_none")]
        parent: Option<ItemRef>,
        /// References to child items
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        children: Vec<ItemRef>,
        /// Content layer identifier (e.g., "body", "furniture")
        content_layer: String,
        /// Provenance information (page, bbox, charspan)
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        prov: Vec<ProvenanceItem>,
        /// Original text
        orig: String,
        /// Sanitized text
        text: String,
        /// Text formatting (bold, italic, etc.)
        #[serde(skip_serializing_if = "Option::is_none", default)]
        formatting: Option<Formatting>,
        /// Hyperlink URL if this text is a link
        #[serde(skip_serializing_if = "Option::is_none", default)]
        hyperlink: Option<String>,
    },

    /// Code
    Code {
        /// Self-reference path (e.g., "#/texts/0")
        self_ref: String,
        /// Reference to parent item
        #[serde(skip_serializing_if = "Option::is_none")]
        parent: Option<ItemRef>,
        /// References to child items
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        children: Vec<ItemRef>,
        /// Content layer identifier (e.g., "body", "furniture")
        content_layer: String,
        /// Provenance information (page, bbox, charspan)
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        prov: Vec<ProvenanceItem>,
        /// Original text
        orig: String,
        /// Sanitized text
        text: String,
        /// Programming language identifier
        #[serde(skip_serializing_if = "Option::is_none")]
        language: Option<String>,
        /// Text formatting (bold, italic, etc.)
        #[serde(skip_serializing_if = "Option::is_none", default)]
        formatting: Option<Formatting>,
        /// Hyperlink URL if this text is a link
        #[serde(skip_serializing_if = "Option::is_none", default)]
        hyperlink: Option<String>,
    },

    /// Formula
    Formula {
        /// Self-reference path (e.g., "#/texts/0")
        self_ref: String,
        /// Reference to parent item
        #[serde(skip_serializing_if = "Option::is_none")]
        parent: Option<ItemRef>,
        /// References to child items
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        children: Vec<ItemRef>,
        /// Content layer identifier (e.g., "body", "furniture")
        content_layer: String,
        /// Provenance information (page, bbox, charspan)
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        prov: Vec<ProvenanceItem>,
        /// Original text
        orig: String,
        /// Sanitized text
        text: String,
        /// Text formatting (bold, italic, etc.)
        #[serde(skip_serializing_if = "Option::is_none", default)]
        formatting: Option<Formatting>,
        /// Hyperlink URL if this text is a link
        #[serde(skip_serializing_if = "Option::is_none", default)]
        hyperlink: Option<String>,
    },

    /// Checkbox (selected)
    #[serde(rename = "checkbox_selected")]
    CheckboxSelected {
        /// Self-reference path (e.g., "#/texts/0")
        self_ref: String,
        /// Reference to parent item
        #[serde(skip_serializing_if = "Option::is_none")]
        parent: Option<ItemRef>,
        /// References to child items
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        children: Vec<ItemRef>,
        /// Content layer identifier (e.g., "body", "furniture")
        content_layer: String,
        /// Provenance information (page, bbox, charspan)
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        prov: Vec<ProvenanceItem>,
        /// Original text
        orig: String,
        /// Sanitized text
        text: String,
        /// Text formatting (bold, italic, etc.)
        #[serde(skip_serializing_if = "Option::is_none", default)]
        formatting: Option<Formatting>,
        /// Hyperlink URL if this text is a link
        #[serde(skip_serializing_if = "Option::is_none", default)]
        hyperlink: Option<String>,
    },

    /// Checkbox (unselected)
    #[serde(rename = "checkbox_unselected")]
    CheckboxUnselected {
        /// Self-reference path (e.g., "#/texts/0")
        self_ref: String,
        /// Reference to parent item
        #[serde(skip_serializing_if = "Option::is_none")]
        parent: Option<ItemRef>,
        /// References to child items
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        children: Vec<ItemRef>,
        /// Content layer identifier (e.g., "body", "furniture")
        content_layer: String,
        /// Provenance information (page, bbox, charspan)
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        prov: Vec<ProvenanceItem>,
        /// Original text
        orig: String,
        /// Sanitized text
        text: String,
        /// Text formatting (bold, italic, etc.)
        #[serde(skip_serializing_if = "Option::is_none", default)]
        formatting: Option<Formatting>,
        /// Hyperlink URL if this text is a link
        #[serde(skip_serializing_if = "Option::is_none", default)]
        hyperlink: Option<String>,
    },
}

impl DocItem {
    /// Get the self reference
    #[inline]
    #[must_use = "returns the self reference path"]
    pub fn self_ref(&self) -> &str {
        match self {
            Self::Text { self_ref, .. }
            | Self::SectionHeader { self_ref, .. }
            | Self::ListItem { self_ref, .. }
            | Self::List { self_ref, .. }
            | Self::FormArea { self_ref, .. }
            | Self::KeyValueArea { self_ref, .. }
            | Self::OrderedList { self_ref, .. }
            | Self::Chapter { self_ref, .. }
            | Self::Section { self_ref, .. }
            | Self::Sheet { self_ref, .. }
            | Self::Slide { self_ref, .. }
            | Self::CommentSection { self_ref, .. }
            | Self::Inline { self_ref, .. }
            | Self::PictureArea { self_ref, .. }
            | Self::Unspecified { self_ref, .. }
            | Self::Table { self_ref, .. }
            | Self::Picture { self_ref, .. }
            | Self::PageFooter { self_ref, .. }
            | Self::PageHeader { self_ref, .. }
            | Self::Paragraph { self_ref, .. }
            | Self::Caption { self_ref, .. }
            | Self::Title { self_ref, .. }
            | Self::Footnote { self_ref, .. }
            | Self::Reference { self_ref, .. }
            | Self::Code { self_ref, .. }
            | Self::Formula { self_ref, .. }
            | Self::CheckboxSelected { self_ref, .. }
            | Self::CheckboxUnselected { self_ref, .. } => self_ref,
        }
    }

    /// Get the text content, if applicable
    #[inline]
    #[must_use = "returns the text content if applicable"]
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::Text { text, .. }
            | Self::SectionHeader { text, .. }
            | Self::ListItem { text, .. }
            | Self::PageFooter { text, .. }
            | Self::PageHeader { text, .. }
            | Self::Paragraph { text, .. }
            | Self::Caption { text, .. }
            | Self::Title { text, .. }
            | Self::Footnote { text, .. }
            | Self::Reference { text, .. }
            | Self::Code { text, .. }
            | Self::Formula { text, .. }
            | Self::CheckboxSelected { text, .. }
            | Self::CheckboxUnselected { text, .. } => Some(text),
            Self::List { .. }
            | Self::FormArea { .. }
            | Self::KeyValueArea { .. }
            | Self::OrderedList { .. }
            | Self::Chapter { .. }
            | Self::Section { .. }
            | Self::Sheet { .. }
            | Self::Slide { .. }
            | Self::CommentSection { .. }
            | Self::Inline { .. }
            | Self::PictureArea { .. }
            | Self::Unspecified { .. }
            | Self::Table { .. }
            | Self::Picture { .. } => None,
        }
    }

    /// Get provenance information
    #[inline]
    #[must_use = "returns the provenance information"]
    pub fn provenance(&self) -> &[ProvenanceItem] {
        match self {
            Self::Text { prov, .. }
            | Self::SectionHeader { prov, .. }
            | Self::ListItem { prov, .. }
            | Self::Table { prov, .. }
            | Self::Picture { prov, .. }
            | Self::PageFooter { prov, .. }
            | Self::PageHeader { prov, .. }
            | Self::Paragraph { prov, .. }
            | Self::Caption { prov, .. }
            | Self::Title { prov, .. }
            | Self::Footnote { prov, .. }
            | Self::Reference { prov, .. }
            | Self::Code { prov, .. }
            | Self::Formula { prov, .. }
            | Self::CheckboxSelected { prov, .. }
            | Self::CheckboxUnselected { prov, .. } => prov,
            Self::List { .. }
            | Self::FormArea { .. }
            | Self::KeyValueArea { .. }
            | Self::OrderedList { .. }
            | Self::Chapter { .. }
            | Self::Section { .. }
            | Self::Sheet { .. }
            | Self::Slide { .. }
            | Self::CommentSection { .. }
            | Self::Inline { .. }
            | Self::PictureArea { .. }
            | Self::Unspecified { .. } => &[],
        }
    }
}

impl std::fmt::Display for DocItem {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (variant, self_ref) = match self {
            Self::Text { self_ref, .. } => ("text", self_ref.as_str()),
            Self::SectionHeader { self_ref, .. } => ("section_header", self_ref.as_str()),
            Self::ListItem { self_ref, .. } => ("list_item", self_ref.as_str()),
            Self::List { self_ref, .. } => ("list", self_ref.as_str()),
            Self::FormArea { self_ref, .. } => ("form_area", self_ref.as_str()),
            Self::KeyValueArea { self_ref, .. } => ("key_value_area", self_ref.as_str()),
            Self::OrderedList { self_ref, .. } => ("ordered_list", self_ref.as_str()),
            Self::Chapter { self_ref, .. } => ("chapter", self_ref.as_str()),
            Self::Section { self_ref, .. } => ("section", self_ref.as_str()),
            Self::Sheet { self_ref, .. } => ("sheet", self_ref.as_str()),
            Self::Slide { self_ref, .. } => ("slide", self_ref.as_str()),
            Self::CommentSection { self_ref, .. } => ("comment_section", self_ref.as_str()),
            Self::Inline { self_ref, .. } => ("inline", self_ref.as_str()),
            Self::PictureArea { self_ref, .. } => ("picture_area", self_ref.as_str()),
            Self::Unspecified { self_ref, .. } => ("unspecified", self_ref.as_str()),
            Self::Table { self_ref, .. } => ("table", self_ref.as_str()),
            Self::Picture { self_ref, .. } => ("picture", self_ref.as_str()),
            Self::PageFooter { self_ref, .. } => ("page_footer", self_ref.as_str()),
            Self::PageHeader { self_ref, .. } => ("page_header", self_ref.as_str()),
            Self::Paragraph { self_ref, .. } => ("paragraph", self_ref.as_str()),
            Self::Caption { self_ref, .. } => ("caption", self_ref.as_str()),
            Self::Title { self_ref, .. } => ("title", self_ref.as_str()),
            Self::Footnote { self_ref, .. } => ("footnote", self_ref.as_str()),
            Self::Reference { self_ref, .. } => ("reference", self_ref.as_str()),
            Self::Code { self_ref, .. } => ("code", self_ref.as_str()),
            Self::Formula { self_ref, .. } => ("formula", self_ref.as_str()),
            Self::CheckboxSelected { self_ref, .. } => ("checkbox_selected", self_ref.as_str()),
            Self::CheckboxUnselected { self_ref, .. } => ("checkbox_unselected", self_ref.as_str()),
        };
        write!(f, "{variant} ({self_ref})")
    }
}

/// Legacy `ContentBlock` type alias for backward compatibility
pub type ContentBlock = DocItem;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_item_creation() {
        let item = DocItem::Text {
            self_ref: "#/texts/0".to_string(),
            parent: Some(ItemRef::new("#/body")),
            children: vec![],
            content_layer: "body".to_string(),
            prov: vec![],
            orig: "Hello, world!".to_string(),
            text: "Hello, world!".to_string(),
            formatting: None,
            hyperlink: None,
        };

        assert_eq!(item.self_ref(), "#/texts/0");
        assert_eq!(item.text(), Some("Hello, world!"));
    }

    #[test]
    fn test_section_header_creation() {
        let item = DocItem::SectionHeader {
            self_ref: "#/texts/1".to_string(),
            parent: Some(ItemRef::new("#/body")),
            children: vec![],
            content_layer: "body".to_string(),
            prov: vec![],
            orig: "Introduction".to_string(),
            text: "Introduction".to_string(),
            level: 1,
            formatting: None,
            hyperlink: None,
        };

        assert_eq!(item.self_ref(), "#/texts/1");
        assert_eq!(item.text(), Some("Introduction"));
    }

    #[test]
    fn test_doc_item_serialization() {
        let item = DocItem::Text {
            self_ref: "#/texts/0".to_string(),
            parent: None,
            children: vec![],
            content_layer: "body".to_string(),
            prov: vec![],
            orig: "Test".to_string(),
            text: "Test".to_string(),
            formatting: None,
            hyperlink: None,
        };

        let json = serde_json::to_string(&item).unwrap();
        let deserialized: DocItem = serde_json::from_str(&json).unwrap();

        assert_eq!(item, deserialized);
    }

    #[test]
    fn test_bounding_box() {
        let bbox = BoundingBox::new(72.0, 766.0, 262.0, 756.0, CoordOrigin::BottomLeft);
        assert_eq!(bbox.width(), 190.0);
        assert_eq!(bbox.height(), 10.0);
        assert_eq!(bbox.area(), 1900.0);
    }

    #[test]
    fn test_item_ref() {
        let item_ref = ItemRef::new("#/texts/0");
        assert_eq!(item_ref.ref_path, "#/texts/0");

        let json = serde_json::to_string(&item_ref).unwrap();
        assert!(json.contains("$ref"));
    }

    #[test]
    fn test_bounding_box_coordinate_conversion() {
        // Test conversion from bottom-left to top-left origin
        let bbox = BoundingBox::new(100.0, 700.0, 200.0, 650.0, CoordOrigin::BottomLeft);
        let page_height = US_LETTER_HEIGHT;
        let top_left_bbox = bbox.to_top_left_origin(page_height);

        assert_eq!(top_left_bbox.coord_origin, CoordOrigin::TopLeft);
        assert_eq!(top_left_bbox.l, 100.0);
        assert_eq!(top_left_bbox.r, 200.0);
        // In top-left origin, t should be page_height - original_t
        assert_eq!(top_left_bbox.t, US_LETTER_HEIGHT - 700.0);
        assert_eq!(top_left_bbox.b, US_LETTER_HEIGHT - 650.0);

        // Converting back should return original
        let back_to_bottom_left = top_left_bbox.to_bottom_left_origin(page_height);
        assert_eq!(back_to_bottom_left.coord_origin, CoordOrigin::BottomLeft);
        assert_eq!(back_to_bottom_left.l, bbox.l);
        assert_eq!(back_to_bottom_left.t, bbox.t);
        assert_eq!(back_to_bottom_left.r, bbox.r);
        assert_eq!(back_to_bottom_left.b, bbox.b);
    }

    #[test]
    fn test_bounding_box_from_tuple() {
        let tuple = (10.0, 20.0, 30.0, 40.0); // (l, b, r, t)
        let bbox = BoundingBox::from_tuple(tuple, CoordOrigin::BottomLeft);

        assert_eq!(bbox.l, 10.0);
        assert_eq!(bbox.b, 20.0);
        assert_eq!(bbox.r, 30.0);
        assert_eq!(bbox.t, 40.0);

        let as_tuple = bbox.as_tuple();
        assert_eq!(as_tuple, tuple);
    }

    #[test]
    fn test_size_operations() {
        let size = Size::new(100.0, 200.0);
        assert_eq!(size.width, 100.0);
        assert_eq!(size.height, 200.0);
        assert_eq!(size.as_tuple(), (100.0, 200.0));
    }

    #[test]
    fn test_table_cell_with_spans() {
        let cell = TableCell {
            text: "Merged Cell".to_string(),
            row_span: Some(2),
            col_span: Some(3),
            ..Default::default()
        };

        assert_eq!(cell.text, "Merged Cell");
        assert_eq!(cell.row_span, Some(2));
        assert_eq!(cell.col_span, Some(3));
    }

    #[test]
    fn test_table_data_structure() {
        let cell1 = TableCell {
            text: "A".to_string(),
            ..Default::default()
        };
        let cell2 = TableCell {
            text: "B".to_string(),
            ..Default::default()
        };

        let table_data = TableData {
            num_rows: 1,
            num_cols: 2,
            grid: vec![vec![cell1.clone(), cell2.clone()]],
            table_cells: Some(vec![cell1, cell2]),
        };

        assert_eq!(table_data.num_rows, 1);
        assert_eq!(table_data.num_cols, 2);
        assert_eq!(table_data.grid.len(), 1);
        assert_eq!(table_data.grid[0].len(), 2);
    }

    #[test]
    fn test_provenance_item() {
        let bbox = BoundingBox::new(0.0, 100.0, 50.0, 90.0, CoordOrigin::BottomLeft);
        let prov = ProvenanceItem {
            page_no: 1,
            bbox,
            charspan: Some(vec![0, 10]),
        };

        assert_eq!(prov.page_no, 1);
        assert_eq!(prov.bbox.width(), 50.0);
        assert_eq!(prov.charspan, Some(vec![0, 10]));
    }

    #[test]
    fn test_formatting_options() {
        let formatting = Formatting {
            bold: Some(true),
            italic: Some(false),
            underline: Some(true),
            strikethrough: None,
            code: None,
            script: Some("superscript".to_string()),
            font_size: Some(12.0),
            font_family: Some("Arial".to_string()),
        };

        assert_eq!(formatting.bold, Some(true));
        assert_eq!(formatting.italic, Some(false));
        assert_eq!(formatting.underline, Some(true));
        assert_eq!(formatting.font_size, Some(12.0));
        assert_eq!(formatting.font_family, Some("Arial".to_string()));
    }

    #[test]
    fn test_list_item_with_marker() {
        let item = DocItem::ListItem {
            self_ref: "#/texts/5".to_string(),
            parent: Some(ItemRef::new("#/groups/0")),
            children: vec![],
            content_layer: "body".to_string(),
            prov: vec![],
            orig: "First item".to_string(),
            text: "First item".to_string(),
            enumerated: true,
            marker: "1.".to_string(),
            formatting: None,
            hyperlink: None,
        };

        assert_eq!(item.self_ref(), "#/texts/5");
        assert_eq!(item.text(), Some("First item"));

        if let DocItem::ListItem {
            enumerated, marker, ..
        } = item
        {
            assert!(enumerated);
            assert_eq!(marker, "1.");
        } else {
            panic!("Expected ListItem variant");
        }
    }

    #[test]
    fn test_code_with_language() {
        let item = DocItem::Code {
            self_ref: "#/texts/10".to_string(),
            parent: None,
            children: vec![],
            content_layer: "body".to_string(),
            prov: vec![],
            orig: "fn main() {}".to_string(),
            text: "fn main() {}".to_string(),
            language: Some("rust".to_string()),
            formatting: None,
            hyperlink: None,
        };

        assert_eq!(item.self_ref(), "#/texts/10");
        assert_eq!(item.text(), Some("fn main() {}"));

        if let DocItem::Code { language, .. } = item {
            assert_eq!(language, Some("rust".to_string()));
        } else {
            panic!("Expected Code variant");
        }
    }

    #[test]
    fn test_group_variants_no_text() {
        let list_item = DocItem::List {
            self_ref: "#/groups/0".to_string(),
            parent: None,
            children: vec![ItemRef::new("#/texts/0"), ItemRef::new("#/texts/1")],
            content_layer: "body".to_string(),
            name: "ordered_list_1".to_string(),
        };

        let table_item = DocItem::Table {
            self_ref: "#/tables/0".to_string(),
            parent: None,
            children: vec![],
            content_layer: "body".to_string(),
            prov: vec![],
            data: TableData {
                num_rows: 0,
                num_cols: 0,
                grid: vec![],
                table_cells: None,
            },
            captions: vec![],
            footnotes: vec![],
            references: vec![],
            image: None,
            annotations: vec![],
        };

        // Groups and tables should return None for text()
        assert_eq!(list_item.text(), None);
        assert_eq!(table_item.text(), None);

        // But should have self_ref
        assert_eq!(list_item.self_ref(), "#/groups/0");
        assert_eq!(table_item.self_ref(), "#/tables/0");
    }

    #[test]
    fn test_content_label_serialization() {
        let label = ContentLabel::Paragraph;
        let json = serde_json::to_string(&label).unwrap();
        assert_eq!(json, r#""paragraph""#);

        let deserialized: ContentLabel = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ContentLabel::Paragraph);
    }

    #[test]
    fn test_content_label_display() {
        assert_eq!(format!("{}", ContentLabel::Paragraph), "paragraph");
        assert_eq!(format!("{}", ContentLabel::SectionHeader), "section_header");
        assert_eq!(format!("{}", ContentLabel::Title), "title");
        assert_eq!(format!("{}", ContentLabel::Table), "table");
        assert_eq!(format!("{}", ContentLabel::Picture), "picture");
        assert_eq!(format!("{}", ContentLabel::Chart), "chart");
        assert_eq!(format!("{}", ContentLabel::ListItem), "list_item");
        assert_eq!(format!("{}", ContentLabel::Code), "code");
        assert_eq!(format!("{}", ContentLabel::Caption), "caption");
        assert_eq!(format!("{}", ContentLabel::Footnote), "footnote");
        assert_eq!(format!("{}", ContentLabel::Formula), "formula");
        assert_eq!(format!("{}", ContentLabel::PageHeader), "page_header");
        assert_eq!(format!("{}", ContentLabel::PageFooter), "page_footer");
        assert_eq!(format!("{}", ContentLabel::Reference), "reference");
        assert_eq!(format!("{}", ContentLabel::Form), "form");
        assert_eq!(
            format!("{}", ContentLabel::CheckboxSelected),
            "checkbox_selected"
        );
        assert_eq!(
            format!("{}", ContentLabel::CheckboxUnselected),
            "checkbox_unselected"
        );
        assert_eq!(
            format!("{}", ContentLabel::KeyValueRegion),
            "key_value_region"
        );
        assert_eq!(format!("{}", ContentLabel::Text), "text");
    }

    #[test]
    fn test_coord_origin_display() {
        assert_eq!(format!("{}", CoordOrigin::BottomLeft), "bottom_left");
        assert_eq!(format!("{}", CoordOrigin::TopLeft), "top_left");
    }

    #[test]
    fn test_coord_origin_from_str() {
        use std::str::FromStr;

        // Standard formats
        assert_eq!(
            CoordOrigin::from_str("bottom_left").unwrap(),
            CoordOrigin::BottomLeft
        );
        assert_eq!(
            CoordOrigin::from_str("top_left").unwrap(),
            CoordOrigin::TopLeft
        );

        // Alternative formats
        assert_eq!(
            CoordOrigin::from_str("bottomleft").unwrap(),
            CoordOrigin::BottomLeft
        );
        assert_eq!(
            CoordOrigin::from_str("topleft").unwrap(),
            CoordOrigin::TopLeft
        );
        assert_eq!(
            CoordOrigin::from_str("bottom-left").unwrap(),
            CoordOrigin::BottomLeft
        );
        assert_eq!(
            CoordOrigin::from_str("top-left").unwrap(),
            CoordOrigin::TopLeft
        );

        // Case insensitive
        assert_eq!(
            CoordOrigin::from_str("BOTTOM_LEFT").unwrap(),
            CoordOrigin::BottomLeft
        );
        assert_eq!(
            CoordOrigin::from_str("TOP_LEFT").unwrap(),
            CoordOrigin::TopLeft
        );

        // Invalid
        assert!(CoordOrigin::from_str("invalid").is_err());
        assert!(CoordOrigin::from_str("").is_err());
    }

    #[test]
    fn test_coord_origin_roundtrip() {
        use std::str::FromStr;

        for origin in [CoordOrigin::BottomLeft, CoordOrigin::TopLeft] {
            let s = origin.to_string();
            let parsed = CoordOrigin::from_str(&s).unwrap();
            assert_eq!(origin, parsed);
        }
    }

    #[test]
    fn test_doc_item_display() {
        let text = DocItem::Text {
            self_ref: "#/texts/0".to_string(),
            parent: None,
            children: vec![],
            content_layer: "body".to_string(),
            prov: vec![],
            orig: "Hello".to_string(),
            text: "Hello".to_string(),
            formatting: None,
            hyperlink: None,
        };
        assert_eq!(format!("{text}"), "text (#/texts/0)");

        let section = DocItem::SectionHeader {
            self_ref: "#/texts/1".to_string(),
            parent: None,
            children: vec![],
            content_layer: "body".to_string(),
            prov: vec![],
            orig: "Header".to_string(),
            text: "Header".to_string(),
            level: 1,
            formatting: None,
            hyperlink: None,
        };
        assert_eq!(format!("{section}"), "section_header (#/texts/1)");

        let table = DocItem::Table {
            self_ref: "#/tables/0".to_string(),
            parent: None,
            children: vec![],
            content_layer: "body".to_string(),
            prov: vec![],
            data: TableData {
                num_rows: 2,
                num_cols: 3,
                grid: vec![],
                table_cells: None,
            },
            captions: vec![],
            footnotes: vec![],
            references: vec![],
            image: None,
            annotations: vec![],
        };
        assert_eq!(format!("{table}"), "table (#/tables/0)");
    }

    #[test]
    fn test_content_label_from_str() {
        use std::str::FromStr;

        // Standard formats
        assert_eq!(
            ContentLabel::from_str("paragraph").unwrap(),
            ContentLabel::Paragraph
        );
        assert_eq!(
            ContentLabel::from_str("section_header").unwrap(),
            ContentLabel::SectionHeader
        );
        assert_eq!(
            ContentLabel::from_str("title").unwrap(),
            ContentLabel::Title
        );
        assert_eq!(
            ContentLabel::from_str("table").unwrap(),
            ContentLabel::Table
        );
        assert_eq!(
            ContentLabel::from_str("picture").unwrap(),
            ContentLabel::Picture
        );
        assert_eq!(
            ContentLabel::from_str("chart").unwrap(),
            ContentLabel::Chart
        );
        assert_eq!(
            ContentLabel::from_str("list_item").unwrap(),
            ContentLabel::ListItem
        );
        assert_eq!(ContentLabel::from_str("code").unwrap(), ContentLabel::Code);
        assert_eq!(
            ContentLabel::from_str("caption").unwrap(),
            ContentLabel::Caption
        );
        assert_eq!(
            ContentLabel::from_str("footnote").unwrap(),
            ContentLabel::Footnote
        );
        assert_eq!(
            ContentLabel::from_str("formula").unwrap(),
            ContentLabel::Formula
        );
        assert_eq!(
            ContentLabel::from_str("page_header").unwrap(),
            ContentLabel::PageHeader
        );
        assert_eq!(
            ContentLabel::from_str("page_footer").unwrap(),
            ContentLabel::PageFooter
        );
        assert_eq!(
            ContentLabel::from_str("reference").unwrap(),
            ContentLabel::Reference
        );
        assert_eq!(ContentLabel::from_str("form").unwrap(), ContentLabel::Form);
        assert_eq!(
            ContentLabel::from_str("checkbox_selected").unwrap(),
            ContentLabel::CheckboxSelected
        );
        assert_eq!(
            ContentLabel::from_str("checkbox_unselected").unwrap(),
            ContentLabel::CheckboxUnselected
        );
        assert_eq!(
            ContentLabel::from_str("key_value_region").unwrap(),
            ContentLabel::KeyValueRegion
        );
        assert_eq!(ContentLabel::from_str("text").unwrap(), ContentLabel::Text);

        // Alternative formats (compact)
        assert_eq!(
            ContentLabel::from_str("sectionheader").unwrap(),
            ContentLabel::SectionHeader
        );
        assert_eq!(
            ContentLabel::from_str("listitem").unwrap(),
            ContentLabel::ListItem
        );
        assert_eq!(
            ContentLabel::from_str("pageheader").unwrap(),
            ContentLabel::PageHeader
        );
        assert_eq!(
            ContentLabel::from_str("pagefooter").unwrap(),
            ContentLabel::PageFooter
        );

        // Alternative formats (kebab-case)
        assert_eq!(
            ContentLabel::from_str("section-header").unwrap(),
            ContentLabel::SectionHeader
        );
        assert_eq!(
            ContentLabel::from_str("list-item").unwrap(),
            ContentLabel::ListItem
        );
        assert_eq!(
            ContentLabel::from_str("page-header").unwrap(),
            ContentLabel::PageHeader
        );
        assert_eq!(
            ContentLabel::from_str("page-footer").unwrap(),
            ContentLabel::PageFooter
        );

        // Case insensitive
        assert_eq!(
            ContentLabel::from_str("PARAGRAPH").unwrap(),
            ContentLabel::Paragraph
        );
        assert_eq!(
            ContentLabel::from_str("Section_Header").unwrap(),
            ContentLabel::SectionHeader
        );

        // Invalid
        assert!(ContentLabel::from_str("invalid").is_err());
        assert!(ContentLabel::from_str("").is_err());
    }

    #[test]
    fn test_content_label_roundtrip() {
        use std::str::FromStr;

        // Test all variants roundtrip: Display -> FromStr -> original
        let all_labels = [
            ContentLabel::Paragraph,
            ContentLabel::SectionHeader,
            ContentLabel::Title,
            ContentLabel::Table,
            ContentLabel::Picture,
            ContentLabel::Chart,
            ContentLabel::ListItem,
            ContentLabel::Code,
            ContentLabel::Caption,
            ContentLabel::Footnote,
            ContentLabel::Formula,
            ContentLabel::PageHeader,
            ContentLabel::PageFooter,
            ContentLabel::Reference,
            ContentLabel::Form,
            ContentLabel::CheckboxSelected,
            ContentLabel::CheckboxUnselected,
            ContentLabel::KeyValueRegion,
            ContentLabel::Text,
        ];

        for label in all_labels {
            let s = label.to_string();
            let parsed = ContentLabel::from_str(&s).unwrap();
            assert_eq!(
                label, parsed,
                "Roundtrip failed for {label:?}: '{s}' -> {parsed:?}"
            );
        }
    }
}
