use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// US Letter page width in PDF points (8.5 inches × 72 points/inch)
pub const US_LETTER_WIDTH_F32: f32 = 612.0;

/// US Letter page height in PDF points (11 inches × 72 points/inch)
pub const US_LETTER_HEIGHT_F32: f32 = 792.0;

/// Coordinate origin for bounding boxes
///
/// Specifies the origin point for coordinate systems used in bounding boxes.
/// PDF documents can use either top-left or bottom-left origin.
///
/// Defaults to `TopLeft` which is the standard screen coordinate system.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CoordOrigin {
    /// Top-left origin (y increases downward)
    #[default]
    #[serde(rename = "TOPLEFT")]
    TopLeft,
    /// Bottom-left origin (y increases upward)
    #[serde(rename = "BOTTOMLEFT")]
    BottomLeft,
}

impl fmt::Display for CoordOrigin {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TopLeft => write!(f, "top-left"),
            Self::BottomLeft => write!(f, "bottom-left"),
        }
    }
}

impl std::str::FromStr for CoordOrigin {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Normalize: lowercase and replace underscores/spaces with hyphens
        let normalized: String = s
            .to_lowercase()
            .chars()
            .map(|c| if c == '_' || c == ' ' { '-' } else { c })
            .collect();

        match normalized.as_str() {
            "topleft" | "top-left" | "tl" => Ok(Self::TopLeft),
            "bottomleft" | "bottom-left" | "bl" => Ok(Self::BottomLeft),
            _ => Err(format!(
                "unknown coord origin: '{s}' (expected: top-left, bottom-left)"
            )),
        }
    }
}

/// Bounding box with coordinates
///
/// Represents a rectangular region on a page with left, top, right, and bottom coordinates.
/// The coordinate system is specified by `coord_origin`.
///
/// # Examples
///
/// ```
/// use docling_pdf_ml::{BoundingBox, CoordOrigin};
///
/// let bbox = BoundingBox {
///     l: 100.0,
///     t: 200.0,
///     r: 300.0,
///     b: 400.0,
///     coord_origin: CoordOrigin::TopLeft,
/// };
///
/// let area = bbox.area();
/// assert_eq!(area, 200.0 * 200.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BoundingBox {
    /// Left x-coordinate
    pub l: f32,
    /// Top y-coordinate
    pub t: f32,
    /// Right x-coordinate
    pub r: f32,
    /// Bottom y-coordinate
    pub b: f32,
    /// Coordinate origin (default: `TopLeft`)
    #[serde(default = "default_coord_origin")]
    pub coord_origin: CoordOrigin,
}

#[inline]
fn default_coord_origin() -> CoordOrigin {
    CoordOrigin::default()
}

impl BoundingBox {
    /// Convert to bottom-left origin coordinate system
    ///
    /// Important: t and b represent TOP and BOTTOM edges respectively in both coordinate systems.
    /// When converting from TOPLEFT to BOTTOMLEFT:
    ///   - TOP edge: smaller y in TOPLEFT → larger y in BOTTOMLEFT
    ///   - BOTTOM edge: larger y in TOPLEFT → smaller y in BOTTOMLEFT
    ///
    /// Result: In BOTTOMLEFT, t > b (top is higher/further from origin)
    #[inline]
    #[must_use = "returns the bounding box converted to bottom-left origin"]
    pub fn to_bottom_left_origin(&self, page_height: f32) -> Self {
        match self.coord_origin {
            CoordOrigin::BottomLeft => *self,
            CoordOrigin::TopLeft => Self {
                l: self.l,
                t: page_height - self.t, // Top edge: smaller y_top → larger y_bottom
                r: self.r,
                b: page_height - self.b, // Bottom edge: larger y_bot → smaller y_bottom
                coord_origin: CoordOrigin::BottomLeft,
            },
        }
    }

    /// Calculate area of bounding box (handles inverted coordinates)
    #[inline]
    #[must_use = "returns the area of the bounding box"]
    pub fn area(&self) -> f32 {
        (self.r - self.l).abs() * (self.b - self.t).abs()
    }

    /// Calculate intersection area with another bounding box (handles inverted coordinates)
    #[inline]
    #[must_use = "returns the intersection area with another bounding box"]
    pub fn intersection_area(&self, other: &Self) -> f32 {
        // Normalize coordinates (handle inverted t/b or l/r)
        let self_l = self.l.min(self.r);
        let self_r = self.l.max(self.r);
        let self_t = self.t.min(self.b);
        let self_b = self.t.max(self.b);

        let other_l = other.l.min(other.r);
        let other_r = other.l.max(other.r);
        let other_t = other.t.min(other.b);
        let other_b = other.t.max(other.b);

        let x_overlap = (self_r.min(other_r) - self_l.max(other_l)).max(0.0);
        let y_overlap = (self_b.min(other_b) - self_t.max(other_t)).max(0.0);
        x_overlap * y_overlap
    }

    /// Calculate intersection-over-self ratio (overlap fraction from self's perspective)
    #[inline]
    #[must_use = "returns the overlap fraction relative to this box's area"]
    pub fn intersection_over_self(&self, other: &Self) -> f32 {
        let self_area = self.area();
        if self_area <= 0.0 {
            return 0.0;
        }
        let intersection = self.intersection_area(other);
        intersection / self_area
    }

    /// Calculate intersection-over-union (`IoU`) ratio
    #[inline]
    #[must_use = "returns the IoU ratio with another bounding box"]
    pub fn intersection_over_union(&self, other: &Self) -> f32 {
        let intersection = self.intersection_area(other);
        let union = self.area() + other.area() - intersection;
        if union <= 0.0 {
            return 0.0;
        }
        intersection / union
    }

    /// Check if this bbox overlaps with another bbox by at least `min_overlap` ratio
    #[inline]
    #[must_use = "returns whether boxes overlap by at least the minimum ratio"]
    pub fn overlaps(&self, other: &Self, min_overlap: f32) -> bool {
        self.intersection_over_self(other) >= min_overlap
    }

    /// Check if this bbox overlaps horizontally with another bbox
    #[inline]
    #[must_use = "returns whether boxes overlap horizontally"]
    pub fn overlaps_horizontally(&self, other: &Self) -> bool {
        !(self.r <= other.l || other.r <= self.l)
    }

    /// Check if this bbox overlaps vertically with another bbox
    #[inline]
    #[must_use = "returns whether boxes overlap vertically"]
    pub fn overlaps_vertically(&self, other: &Self) -> bool {
        match self.coord_origin {
            CoordOrigin::BottomLeft => !(self.t <= other.b || other.t <= self.b),
            CoordOrigin::TopLeft => !(self.b <= other.t || other.b <= self.t),
        }
    }

    /// Check if this bbox is strictly left of another bbox (with epsilon tolerance)
    #[inline]
    #[must_use = "returns whether this box is strictly left of another"]
    pub fn is_strictly_left_of(&self, other: &Self, eps: f32) -> bool {
        (self.r + eps) < other.l
    }

    /// Check if this bbox is strictly above another bbox (with epsilon tolerance)
    #[inline]
    #[must_use = "returns whether this box is strictly above another"]
    pub fn is_strictly_above(&self, other: &Self, eps: f32) -> bool {
        match self.coord_origin {
            CoordOrigin::BottomLeft => (self.b + eps) > other.t,
            CoordOrigin::TopLeft => (self.b + eps) < other.t,
        }
    }

    /// Check if this bbox overlaps vertically with another bbox by at least iou ratio
    #[inline]
    #[must_use = "returns whether boxes overlap vertically by at least the IoU ratio"]
    pub fn overlaps_vertically_with_iou(&self, other: &Self, iou: f32) -> bool {
        let intersection_height = match self.coord_origin {
            CoordOrigin::BottomLeft => (self.t.min(other.t) - self.b.max(other.b)).max(0.0),
            CoordOrigin::TopLeft => (self.b.min(other.b) - self.t.max(other.t)).max(0.0),
        };

        let self_height = match self.coord_origin {
            CoordOrigin::BottomLeft => self.t - self.b,
            CoordOrigin::TopLeft => self.b - self.t,
        };

        let other_height = match other.coord_origin {
            CoordOrigin::BottomLeft => other.t - other.b,
            CoordOrigin::TopLeft => other.b - other.t,
        };

        let union_height = self_height + other_height - intersection_height;

        if union_height <= 0.0 {
            return false;
        }

        (intersection_height / union_height) >= iou
    }
}

/// Bounding rectangle with 4 corner points
///
/// Represents a quadrilateral region defined by 4 corner points, allowing for
/// rotated or skewed bounding boxes (unlike axis-aligned `BoundingBox`).
/// Points are ordered clockwise starting from top-left: (x0,y0) -> (x1,y1) -> (x2,y2) -> (x3,y3).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BoundingRectangle {
    /// X coordinate of corner 0 (top-left)
    pub r_x0: f32,
    /// Y coordinate of corner 0 (top-left)
    pub r_y0: f32,
    /// X coordinate of corner 1 (top-right)
    pub r_x1: f32,
    /// Y coordinate of corner 1 (top-right)
    pub r_y1: f32,
    /// X coordinate of corner 2 (bottom-right)
    pub r_x2: f32,
    /// Y coordinate of corner 2 (bottom-right)
    pub r_y2: f32,
    /// X coordinate of corner 3 (bottom-left)
    pub r_x3: f32,
    /// Y coordinate of corner 3 (bottom-left)
    pub r_y3: f32,
    /// Coordinate system origin (top-left or bottom-left)
    #[serde(default = "default_coord_origin")]
    pub coord_origin: CoordOrigin,
}

impl BoundingRectangle {
    /// Convert to axis-aligned `BoundingBox`
    #[inline]
    #[must_use = "returns an axis-aligned bounding box"]
    pub fn to_bbox(&self) -> BoundingBox {
        let min_x = self.r_x0.min(self.r_x1).min(self.r_x2).min(self.r_x3);
        let max_x = self.r_x0.max(self.r_x1).max(self.r_x2).max(self.r_x3);
        let min_y = self.r_y0.min(self.r_y1).min(self.r_y2).min(self.r_y3);
        let max_y = self.r_y0.max(self.r_y1).max(self.r_y2).max(self.r_y3);

        BoundingBox {
            l: min_x,
            t: min_y,
            r: max_x,
            b: max_y,
            coord_origin: self.coord_origin,
        }
    }
}

/// Text cell from OCR or PDF text extraction
///
/// Represents a single text unit (word, line, or paragraph) with its bounding
/// rectangle and metadata. Used as input to the assembly pipeline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextCell {
    /// Cell index for ordering and tracking
    #[serde(default)]
    pub index: usize,
    /// Text content of the cell
    pub text: String,
    /// Bounding rectangle (4-corner quadrilateral)
    pub rect: BoundingRectangle,
    /// OCR confidence score (0.0 to 1.0), None if from PDF text layer
    #[serde(default)]
    pub confidence: Option<f32>,
    /// Whether this cell was extracted via OCR (true) or PDF text layer (false)
    #[serde(default)]
    pub from_ocr: bool,
    /// N=4373: Whether text is bold (from PDF font flags)
    #[serde(default)]
    pub is_bold: bool,
    /// N=4373: Whether text is italic (from PDF font flags)
    #[serde(default)]
    pub is_italic: bool,
}

/// Simple text cell with LTRB rectangle (used for baseline loading)
///
/// Represents programmatic text extracted from PDF (e.g., from PDF text layers).
/// This is an input format for providing existing text cells to the pipeline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimpleTextCell {
    /// Cell index (optional, for tracking)
    #[serde(default)]
    pub index: usize,
    /// Text content
    pub text: String,
    /// Bounding box location (axis-aligned rectangle)
    pub rect: BoundingBox,
    /// Confidence score (0.0 to 1.0, defaults to 1.0 for PDF text)
    #[serde(default = "default_confidence")]
    pub confidence: f32,
    /// Whether this cell was extracted via OCR (true) or PDF text layer (false)
    #[serde(default)]
    pub from_ocr: bool,
    /// N=4373: Whether text is bold (from PDF font flags)
    #[serde(default)]
    pub is_bold: bool,
    /// N=4373: Whether text is italic (from PDF font flags)
    #[serde(default)]
    pub is_italic: bool,
}

#[inline]
const fn default_confidence() -> f32 {
    1.0
}

impl SimpleTextCell {
    /// Convert to full `TextCell` with `BoundingRectangle`
    #[inline]
    #[must_use = "returns a full TextCell with BoundingRectangle"]
    pub fn to_text_cell(&self) -> TextCell {
        // Convert simple LTRB rect to 4-corner rect
        TextCell {
            index: self.index,
            text: self.text.clone(),
            rect: BoundingRectangle {
                r_x0: self.rect.l,
                r_y0: self.rect.t,
                r_x1: self.rect.r,
                r_y1: self.rect.t,
                r_x2: self.rect.r,
                r_y2: self.rect.b,
                r_x3: self.rect.l,
                r_y3: self.rect.b,
                coord_origin: self.rect.coord_origin,
            },
            confidence: Some(self.confidence),
            from_ocr: self.from_ocr,
            is_bold: self.is_bold,
            is_italic: self.is_italic,
        }
    }

    /// Get bounding box (for overlap calculations)
    #[inline]
    #[must_use = "returns a reference to the bounding box"]
    pub const fn bbox(&self) -> &BoundingBox {
        &self.rect
    }
}

/// Document item label enum
///
/// Classifies document elements detected by the layout predictor.
/// Each label represents a distinct type of content in the document.
///
/// # Examples
///
/// ```
/// use docling_pdf_ml::DocItemLabel;
///
/// let label = DocItemLabel::Text;
/// assert!(label.is_text_element());
/// assert!(!label.is_table());
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DocItemLabel {
    /// Regular body text paragraph
    #[default]
    #[serde(rename = "text", alias = "Text")]
    Text,
    /// Section or chapter heading
    #[serde(rename = "section_header", alias = "Section-header")]
    SectionHeader,
    /// Running header at top of page
    #[serde(rename = "page_header", alias = "Page-header")]
    PageHeader,
    /// Running footer at bottom of page
    #[serde(rename = "page_footer", alias = "Page-footer")]
    PageFooter,
    /// Document or section title
    #[serde(rename = "title", alias = "Title")]
    Title,
    /// Caption for figures, tables, or other elements
    #[serde(rename = "caption", alias = "Caption")]
    Caption,
    /// Footnote or endnote text
    #[serde(rename = "footnote", alias = "Footnote")]
    Footnote,
    /// Tabular data structure
    #[serde(rename = "table", alias = "Table")]
    Table,
    /// Vector graphic or diagram
    #[serde(rename = "figure", alias = "Figure")]
    Figure,
    /// Raster image or photograph
    #[serde(rename = "picture", alias = "Picture")]
    Picture,
    /// Mathematical formula or equation
    #[serde(rename = "formula", alias = "Formula")]
    Formula,
    /// Item in a bulleted or numbered list
    #[serde(rename = "list_item", alias = "List-item")]
    ListItem,
    /// Source code or preformatted text
    #[serde(rename = "code", alias = "Code")]
    Code,
    /// Checked/selected checkbox
    #[serde(rename = "checkbox_selected")]
    CheckboxSelected,
    /// Unchecked/unselected checkbox
    #[serde(rename = "checkbox_unselected")]
    CheckboxUnselected,
    /// Form field or input area
    #[serde(rename = "form")]
    Form,
    /// Key-value pair region (e.g., form labels)
    #[serde(
        rename = "key_value_region",
        alias = "key-value region",
        alias = "Key-Value Region"
    )]
    KeyValueRegion,
    /// Table of contents or index
    #[serde(rename = "document_index")]
    DocumentIndex,
}

impl DocItemLabel {
    /// Check if this label is a text element
    ///
    /// Matches Python's `TEXT_ELEM_LABELS` (including Title)
    #[inline]
    #[must_use = "returns whether this label is a text element"]
    pub const fn is_text_element(&self) -> bool {
        matches!(
            self,
            Self::Text
                | Self::Title  // N=2396: Add Title to text elements
                | Self::SectionHeader
                | Self::Caption
                | Self::Footnote
                | Self::ListItem
                | Self::Code
                | Self::PageHeader
                | Self::PageFooter
                | Self::CheckboxSelected
                | Self::CheckboxUnselected
                | Self::Formula
        )
    }

    /// Check if this label is a page header
    #[inline]
    #[must_use = "returns whether this label is a page header or footer"]
    pub const fn is_page_header(&self) -> bool {
        matches!(self, Self::PageHeader | Self::PageFooter)
    }

    /// Check if this label is a table
    ///
    /// Matches Python's `TABLE_LABELS` (Table, `DocumentIndex`)
    #[inline]
    #[must_use = "returns whether this label is a table"]
    pub const fn is_table(&self) -> bool {
        matches!(self, Self::Table | Self::DocumentIndex)
    }

    /// Check if this label is a figure
    #[inline]
    #[must_use = "returns whether this label is a figure"]
    pub const fn is_figure(&self) -> bool {
        matches!(self, Self::Figure | Self::Picture)
    }

    /// Check if this label is a container (wrapper type)
    ///
    /// Matches Python's `CONTAINER_LABELS` (Form, `KeyValueRegion` only)
    #[inline]
    #[must_use = "returns whether this label is a container"]
    pub const fn is_container(&self) -> bool {
        matches!(self, Self::Form | Self::KeyValueRegion)
    }

    /// Convert label to Python-style string (lowercase with underscores)
    ///
    /// This matches the format used in modular pipeline baselines and Python code.
    ///
    /// # Examples
    /// ```
    /// use docling_pdf_ml::DocItemLabel;
    ///
    /// assert_eq!(DocItemLabel::Text.to_python_string(), "text");
    /// assert_eq!(DocItemLabel::SectionHeader.to_python_string(), "section_header");
    /// assert_eq!(DocItemLabel::KeyValueRegion.to_python_string(), "key_value_region");
    /// ```
    #[inline]
    #[must_use = "returns the Python-style string representation"]
    pub const fn to_python_string(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::SectionHeader => "section_header",
            Self::PageHeader => "page_header",
            Self::PageFooter => "page_footer",
            Self::Title => "title",
            Self::Caption => "caption",
            Self::Footnote => "footnote",
            Self::Table => "table",
            Self::Figure => "figure",
            Self::Picture => "picture",
            Self::Formula => "formula",
            Self::ListItem => "list_item",
            Self::Code => "code",
            Self::CheckboxSelected => "checkbox_selected",
            Self::CheckboxUnselected => "checkbox_unselected",
            Self::Form => "form",
            Self::KeyValueRegion => "key_value_region",
            Self::DocumentIndex => "document_index",
        }
    }

    /// Convert numeric label ID to `DocItemLabel` enum
    ///
    /// Maps label IDs from ONNX model outputs to `DocItemLabel` variants.
    /// This mapping is derived from the `HuggingFace` model config and verified
    /// against baseline `stage4_final_clusters.json` files.
    ///
    /// # Arguments
    /// * `label_id` - Numeric label ID from model output (0-16)
    ///
    /// # Returns
    /// * `Some(DocItemLabel)` - If `label_id` is valid
    /// * `None` - If `label_id` is unknown or reserved
    ///
    /// # Examples
    /// ```
    /// use docling_pdf_ml::DocItemLabel;
    ///
    /// assert_eq!(DocItemLabel::from_id(0), Some(DocItemLabel::Caption));
    /// assert_eq!(DocItemLabel::from_id(9), Some(DocItemLabel::Text));
    /// assert_eq!(DocItemLabel::from_id(999), None);
    /// ```
    #[must_use = "returns the label for a numeric ID if valid"]
    pub const fn from_id(label_id: i32) -> Option<Self> {
        // Maps label IDs from ML model to DocItemLabel enum
        // Reference: Empirically verified from baseline_data stage3_hf_postprocessed.json
        // Example: code_and_formula page 0: ID 13 appears twice as "code" labels
        // Note: This does NOT match docling-core enum order - ML model uses different IDs
        match label_id {
            0 => Some(Self::Caption),
            1 => Some(Self::Footnote),
            2 => Some(Self::Formula),
            3 => Some(Self::ListItem),
            4 => Some(Self::PageFooter),
            5 => Some(Self::PageHeader),
            6 => Some(Self::Picture),
            7 => Some(Self::SectionHeader),
            8 => Some(Self::Table),
            9 => Some(Self::Text),
            10 => Some(Self::Title),
            11 => Some(Self::DocumentIndex),
            12 => Some(Self::CheckboxSelected),
            13 => Some(Self::Code),
            14 => Some(Self::CheckboxUnselected),
            15 => Some(Self::Form),
            16 => Some(Self::KeyValueRegion),
            _ => None, // Unknown label ID
        }
    }
}

impl fmt::Display for DocItemLabel {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text => write!(f, "Text"),
            Self::SectionHeader => write!(f, "Section Header"),
            Self::PageHeader => write!(f, "Page Header"),
            Self::PageFooter => write!(f, "Page Footer"),
            Self::Title => write!(f, "Title"),
            Self::Caption => write!(f, "Caption"),
            Self::Footnote => write!(f, "Footnote"),
            Self::Table => write!(f, "Table"),
            Self::Figure => write!(f, "Figure"),
            Self::Picture => write!(f, "Picture"),
            Self::Formula => write!(f, "Formula"),
            Self::ListItem => write!(f, "List Item"),
            Self::Code => write!(f, "Code"),
            Self::CheckboxSelected => write!(f, "Checkbox (Selected)"),
            Self::CheckboxUnselected => write!(f, "Checkbox (Unselected)"),
            Self::Form => write!(f, "Form"),
            Self::KeyValueRegion => write!(f, "Key-Value Region"),
            Self::DocumentIndex => write!(f, "Document Index"),
        }
    }
}

impl std::str::FromStr for DocItemLabel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Normalize: lowercase and remove spaces/hyphens/underscores for matching
        let normalized: String = s
            .to_lowercase()
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '-' && *c != '_')
            .collect();

        match normalized.as_str() {
            "text" => Ok(Self::Text),
            "sectionheader" | "section" => Ok(Self::SectionHeader),
            "pageheader" => Ok(Self::PageHeader),
            "pagefooter" => Ok(Self::PageFooter),
            "title" => Ok(Self::Title),
            "caption" => Ok(Self::Caption),
            "footnote" => Ok(Self::Footnote),
            "table" => Ok(Self::Table),
            "figure" => Ok(Self::Figure),
            "picture" | "image" => Ok(Self::Picture),
            "formula" | "equation" => Ok(Self::Formula),
            "listitem" | "list" | "item" => Ok(Self::ListItem),
            "code" => Ok(Self::Code),
            "checkboxselected" | "checkbox(selected)" | "selectedcheckbox" => {
                Ok(Self::CheckboxSelected)
            }
            "checkboxunselected" | "checkbox(unselected)" | "unselectedcheckbox" => {
                Ok(Self::CheckboxUnselected)
            }
            "form" => Ok(Self::Form),
            "keyvalueregion" | "keyvalue" | "kv" => Ok(Self::KeyValueRegion),
            "documentindex" | "index" | "toc" => Ok(Self::DocumentIndex),
            _ => Err(format!(
                "unknown doc item label: '{s}' (expected: text, section_header, page_header, \
                 page_footer, title, caption, footnote, table, figure, picture, formula, \
                 list_item, code, checkbox_selected, checkbox_unselected, form, \
                 key_value_region, document_index)"
            )),
        }
    }
}

/// Layout cluster (from layout predictor)
///
/// Represents a detected document element with its classification, location,
/// and associated text content. Clusters are the primary output of the layout
/// predictor model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cluster {
    /// Unique cluster identifier
    #[serde(default)]
    pub id: usize,
    /// Classification label (text, table, figure, etc.)
    pub label: DocItemLabel,
    /// Bounding box location on page
    pub bbox: BoundingBox,
    /// Confidence score from ML model (0.0 to 1.0)
    pub confidence: f32,
    /// Text cells contained within this cluster
    #[serde(default)]
    pub cells: Vec<TextCell>,
    /// Child clusters (for hierarchical structures)
    #[serde(default)]
    pub children: Vec<Cluster>,
}

/// Layout prediction result from the ML model
///
/// Contains the detected document elements (clusters) for a page.
/// Each cluster represents a detected region with its classification,
/// bounding box, and confidence score.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutPrediction {
    /// Detected document elements (text regions, tables, figures, etc.)
    pub clusters: Vec<Cluster>,
}

/// Table cell
///
/// Represents a single cell in a parsed table with its text content,
/// location, and position within the table grid.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableCell {
    /// Text content of the cell
    pub text: String,
    /// Bounding box location
    pub bbox: BoundingBox,
    /// Number of rows this cell spans
    pub row_span: usize,
    /// Number of columns this cell spans
    pub col_span: usize,
    /// Starting row index
    pub start_row_offset_idx: usize,
    /// Ending row index
    pub end_row_offset_idx: usize,
    /// Starting column index
    pub start_col_offset_idx: usize,
    /// Ending column index
    pub end_col_offset_idx: usize,
    /// Whether this cell is a column header (ched tag)
    #[serde(default)]
    pub column_header: bool,
    /// Whether this cell is a row header (rhed tag)
    #[serde(default)]
    pub row_header: bool,
    /// Whether the text came from OCR (F72: Table-OCR linkage)
    #[serde(default)]
    pub from_ocr: bool,
    /// OCR confidence score (F72: Table-OCR linkage)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
}

/// Post-process table cells to split merged numeric values into adjacent empty cells
///
/// Problem: TableFormer sometimes detects the correct number of columns but assigns
/// all OCR text to a single cell, leaving adjacent cells empty. This happens when
/// column boundaries are detected but text-to-cell matching fails.
///
/// Example:
/// - Detected: 10 columns with cells at positions 0-9
/// - Row data: cell\[5\] = "10.56 16.07 44.3 35.3 64.5 48.0 24.3 51.9 44.7"
/// - Cells 1-4 and 6-9 are empty
/// - Expected: Each numeric value in its own column
///
/// Fix: Detect rows where one cell has N space-separated numeric values and there
/// are N-1 empty cells. Redistribute values across those cells.
pub fn postprocess_table_cells(
    mut cells: Vec<TableCell>,
    num_rows: usize,
    num_cols: usize,
) -> Vec<TableCell> {
    log::debug!(
        "postprocess_table_cells: {} cells, {} rows, {} cols",
        cells.len(),
        num_rows,
        num_cols
    );

    // Skip if table is too small or has no cells
    if num_cols < 3 || cells.is_empty() {
        log::debug!("Skipping: table too small or empty");
        return cells;
    }

    // Group cells by row
    let mut rows_to_cells: HashMap<usize, Vec<usize>> = HashMap::new();
    for (idx, cell) in cells.iter().enumerate() {
        rows_to_cells
            .entry(cell.start_row_offset_idx)
            .or_default()
            .push(idx);
    }

    // Regex to detect multiple space-separated numeric values
    // Matches: "10.56 16.07 44.3" or "1 2 3 4 5" or "100.5 200.3"
    let numeric_pattern = regex::Regex::new(r"^[\d\.\-]+(?:\s+[\d\.\-]+)+$").expect("valid regex");

    // Regex to detect text prefix followed by numeric values
    // Matches: "Pythia-160M NeoX 29.64 38.10 33.0" → captures "Pythia-160M NeoX" + "29.64 38.10 33.0"
    // Text tokens: words starting with letter, may contain digits/hyphens (e.g., "Pythia-160M")
    // Numeric tokens: pure number-like strings
    let text_plus_numbers = regex::Regex::new(
        r"^([A-Za-z][\w\.\-]*(?:\s+[A-Za-z][\w\.\-]*)*)\s+([\d\.\-]+(?:\s+[\d\.\-]+)+)$",
    )
    .expect("valid regex");

    let mut modifications: Vec<(usize, Vec<String>)> = Vec::new();

    // Check each row for cells with multiple merged values
    for row in 0..num_rows {
        let Some(cell_indices) = rows_to_cells.get(&row) else {
            continue;
        };

        // Find cells in this row with multiple space-separated values
        for &cell_idx in cell_indices {
            let cell = &cells[cell_idx];
            let text = cell.text.trim();

            // Skip empty cells
            if text.is_empty() {
                continue;
            }

            // Check patterns: pure numeric OR text+numbers
            let pure_numeric = numeric_pattern.is_match(text);
            let text_numbers_match = text_plus_numbers.captures(text);

            if text.contains(' ') && text.len() > 10 {
                log::debug!(
                    "Row {} col {}: text='{}' pure_numeric={} text_numbers={}",
                    row,
                    cell.start_col_offset_idx,
                    &text[..text.len().min(60)],
                    pure_numeric,
                    text_numbers_match.is_some()
                );
            }

            // Parse values based on which pattern matched
            let values: Vec<String> = if pure_numeric {
                // Pure numeric: just split by whitespace
                text.split_whitespace().map(|s| s.to_string()).collect()
            } else if let Some(caps) = text_numbers_match {
                // Text + numbers: split text prefix into words, then numbers
                let text_part = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let num_part = caps.get(2).map(|m| m.as_str()).unwrap_or("");

                let mut vals: Vec<String> = text_part
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect();
                vals.extend(num_part.split_whitespace().map(|s| s.to_string()));
                vals
            } else {
                continue; // Neither pattern matched
            };

            let value_count = values.len();

            // Only process if multiple values (2+)
            if value_count < 2 {
                continue;
            }

            // Count empty cells in this row (both LEFT and RIGHT of source cell)
            let start_col = cell.start_col_offset_idx;
            let mut empty_left_indices: Vec<usize> = Vec::new();
            let mut empty_right_indices: Vec<usize> = Vec::new();

            for &other_idx in cell_indices {
                let other = &cells[other_idx];
                if other.text.trim().is_empty() && other.start_row_offset_idx == row {
                    if other.start_col_offset_idx < start_col {
                        empty_left_indices.push(other_idx);
                    } else if other.start_col_offset_idx > start_col {
                        empty_right_indices.push(other_idx);
                    }
                }
            }

            // Sort by column position (left ascending, right ascending)
            empty_left_indices.sort_by_key(|&idx| cells[idx].start_col_offset_idx);
            empty_right_indices.sort_by_key(|&idx| cells[idx].start_col_offset_idx);

            let total_empty = empty_left_indices.len() + empty_right_indices.len();
            let total_slots = total_empty + 1; // +1 for the source cell

            // Check if we have enough slots to redistribute all values
            if total_slots >= value_count {
                // Strategy: Fill from left to right, source cell gets one value
                // values is already Vec<String>
                let left_count = empty_left_indices.len();

                // Determine how many values go left vs right
                let values_for_left = left_count.min(value_count.saturating_sub(1));
                let source_value_idx = values_for_left;
                let values_for_right = value_count - values_for_left - 1;

                log::debug!(
                    "Row {}: Splitting cell at col {} with {} values: {} left, 1 source, {} right",
                    row,
                    start_col,
                    value_count,
                    values_for_left,
                    values_for_right
                );

                // Assign values to left cells
                for (i, &empty_idx) in empty_left_indices
                    .iter()
                    .rev()
                    .take(values_for_left)
                    .enumerate()
                {
                    let value_idx = values_for_left - 1 - i;
                    modifications.push((empty_idx, vec![values[value_idx].clone()]));
                }

                // Source cell gets the value at index values_for_left
                modifications.push((cell_idx, vec![values[source_value_idx].clone()]));

                // Assign values to right cells
                for (i, &empty_idx) in empty_right_indices
                    .iter()
                    .take(values_for_right)
                    .enumerate()
                {
                    let value_idx = source_value_idx + 1 + i;
                    modifications.push((empty_idx, vec![values[value_idx].clone()]));
                }
            }
        }
    }

    // Apply modifications
    for (idx, new_values) in modifications {
        if !new_values.is_empty() {
            cells[idx].text = new_values[0].clone();
        }
    }

    cells
}

/// Text element (assembled)
///
/// Represents a text region on a page after assembly. Text elements include
/// paragraphs, headers, footers, captions, and other textual content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextElement {
    /// Classification label (Text, `SectionHeader`, Caption, etc.)
    pub label: DocItemLabel,
    /// Unique element identifier
    pub id: usize,
    /// Page number (0-indexed)
    pub page_no: usize,
    /// Extracted text content (sanitized)
    pub text: String,
    /// Original unsanitized text (before hyphenation removal, Unicode normalization, etc.)
    pub orig: String,
    /// Underlying layout cluster
    pub cluster: Cluster,
    /// Caption elements attached to this element (for Code blocks)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub captions: Vec<usize>,
    /// Footnote elements attached to this element (for Code blocks)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub footnotes: Vec<usize>,
    /// N=4373: Whether text is bold (majority of cells are bold)
    #[serde(default)]
    pub is_bold: bool,
    /// N=4373: Whether text is italic (majority of cells are italic)
    #[serde(default)]
    pub is_italic: bool,
}

/// Table element (assembled)
///
/// Represents a table with parsed structure including rows, columns, and cells.
/// Tables are detected by the layout predictor and structure is extracted by
/// the `TableFormer` model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableElement {
    /// Label (should be `DocItemLabel::Table`)
    pub label: DocItemLabel,
    /// Unique element identifier
    pub id: usize,
    /// Page number (0-indexed)
    pub page_no: usize,
    /// Optional concatenated text from all cells
    #[serde(default)]
    pub text: Option<String>,
    /// Underlying layout cluster
    pub cluster: Cluster,
    /// OTSL (Object Tag Sequence Language) representation
    #[serde(default)]
    pub otsl_seq: Vec<String>,
    /// Number of rows in table
    #[serde(default)]
    pub num_rows: usize,
    /// Number of columns in table
    #[serde(default)]
    pub num_cols: usize,
    /// Parsed table cells with positions
    #[serde(default)]
    pub table_cells: Vec<TableCell>,
    /// Caption elements attached to this table
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub captions: Vec<usize>,
    /// Footnote elements attached to this table
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub footnotes: Vec<usize>,
}

/// Figure element (assembled)
///
/// Represents a detected figure/image region on a page. Figures are
/// classified by the code/formula model into pictures, charts, or formulas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FigureElement {
    /// Label (Picture, Chart, or Formula)
    pub label: DocItemLabel,
    /// Unique element identifier
    pub id: usize,
    /// Page number (0-indexed)
    pub page_no: usize,
    /// Optional OCR text from the figure
    pub text: Option<String>,
    /// Underlying layout cluster
    pub cluster: Cluster,
    /// Classification result from code/formula model
    pub predicted_class: Option<String>,
    /// Classification confidence (0.0 to 1.0)
    pub confidence: Option<f32>,
    /// Caption elements attached to this figure
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub captions: Vec<usize>,
    /// Footnote elements attached to this figure
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub footnotes: Vec<usize>,
}

/// Container element (assembled)
///
/// Represents a container region that may hold other elements, such as
/// forms, key-value regions, or other grouped content areas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContainerElement {
    /// Classification label (Form, `KeyValueRegion`, etc.)
    pub label: DocItemLabel,
    /// Unique element identifier
    pub id: usize,
    /// Page number (0-indexed)
    pub page_no: usize,
    /// Underlying layout cluster
    pub cluster: Cluster,
}

/// Page element enum (union type)
///
/// Represents the different types of elements that can appear on a page
/// after document assembly. This is the primary output type for accessing
/// parsed document content.
///
/// # Examples
///
/// ```no_run
/// use docling_pdf_ml::PageElement;
///
/// # fn process_elements(elements: Vec<PageElement>) {
/// for element in elements {
///     match element {
///         PageElement::Text(text) => {
///             log::debug!("Text: {}", text.text);
///         }
///         PageElement::Table(table) => {
///             log::debug!("Table: {}x{}", table.num_rows, table.num_cols);
///         }
///         PageElement::Figure(figure) => {
///             log::debug!("Figure at {:?}", figure.cluster.bbox);
///         }
///         PageElement::Container(container) => {
///             log::debug!("Container: {:?}", container.label);
///         }
///     }
/// }
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PageElement {
    /// Text element (paragraphs, headers, lists, etc.)
    #[serde(rename = "TextElement")]
    Text(TextElement),
    /// Table with parsed structure
    #[serde(rename = "Table")]
    Table(TableElement),
    /// Figure or image
    #[serde(rename = "FigureElement")]
    Figure(FigureElement),
    /// Container element (forms, key-value regions)
    #[serde(rename = "ContainerElement")]
    Container(ContainerElement),
}

impl PageElement {
    /// Returns the element's unique identifier.
    #[inline]
    #[must_use = "returns the element's unique identifier"]
    pub const fn id(&self) -> usize {
        match self {
            Self::Text(e) => e.id,
            Self::Table(e) => e.id,
            Self::Figure(e) => e.id,
            Self::Container(e) => e.id,
        }
    }

    /// Returns the element's page number (0-indexed).
    #[inline]
    #[must_use = "returns the element's page number"]
    pub const fn page_no(&self) -> usize {
        match self {
            Self::Text(e) => e.page_no,
            Self::Table(e) => e.page_no,
            Self::Figure(e) => e.page_no,
            Self::Container(e) => e.page_no,
        }
    }

    /// Returns a reference to the underlying cluster (bounding box and label).
    #[inline]
    #[must_use = "returns a reference to the underlying cluster"]
    pub const fn cluster(&self) -> &Cluster {
        match self {
            Self::Text(e) => &e.cluster,
            Self::Table(e) => &e.cluster,
            Self::Figure(e) => &e.cluster,
            Self::Container(e) => &e.cluster,
        }
    }

    /// Get text content from the element (empty string for non-text elements)
    #[inline]
    #[must_use = "returns the text content of the element"]
    pub fn text(&self) -> &str {
        match self {
            Self::Text(e) => &e.text,
            Self::Table(e) => e.text.as_deref().unwrap_or(""),
            Self::Figure(_) | Self::Container(_) => "",
        }
    }

    /// Renumber cluster ID by adding an offset (for making globally unique IDs across pages)
    pub fn renumber_cluster_id(&mut self, offset: usize) {
        match self {
            Self::Text(e) => e.cluster.id += offset,
            Self::Table(e) => e.cluster.id += offset,
            Self::Figure(e) => e.cluster.id += offset,
            Self::Container(e) => e.cluster.id += offset,
        }
    }
}

impl fmt::Display for PageElement {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text(e) => write!(f, "Text[{}]: {}", e.label, e.text),
            Self::Table(e) => write!(f, "Table[{}x{}]", e.num_rows, e.num_cols),
            Self::Figure(e) => write!(f, "Figure[{}]", e.label),
            Self::Container(e) => write!(f, "Container[{}]", e.label),
        }
    }
}

/// Table structure prediction from the `TableFormer` model.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TableStructurePrediction {
    /// Map from table ID to its parsed structure (rows, columns, cells).
    pub table_map: HashMap<usize, TableElement>,
}

/// Figure classification prediction
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FigureClassificationPrediction {
    pub figure_count: usize,
    pub figure_map: HashMap<usize, FigureElement>,
}

/// Assembled unit (page assembly output)
///
/// Contains the final assembled document elements organized into body and header sections.
/// This is the main output structure after page assembly and reading order determination.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssembledUnit {
    /// All page elements in reading order
    pub elements: Vec<PageElement>,
    /// Body elements (main content)
    pub body: Vec<PageElement>,
    /// Header elements (page headers, footers)
    pub headers: Vec<PageElement>,
}

/// Page predictions (container for all ML outputs).
///
/// Contains the results of all ML models run on a page.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PagePredictions {
    /// Layout detection results (text blocks, tables, figures, etc.).
    pub layout: Option<LayoutPrediction>,
    /// Table structure prediction results (cell boundaries, spanning).
    pub tablestructure: Option<TableStructurePrediction>,
    /// Figure classification results.
    pub figures_classification: Option<FigureClassificationPrediction>,
}

/// Page size
///
/// Represents the dimensions of a page in points (1/72 inch).
/// Defaults to zero dimensions.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Size {
    /// Page width in points
    pub width: f32,
    /// Page height in points
    pub height: f32,
}

/// Page (main data structure)
///
/// Represents a parsed page with ML predictions and assembled content.
/// This is the primary output type returned by [`Pipeline::process_page`].
///
/// # Examples
///
/// ```no_run
/// use docling_pdf_ml::{Pipeline, PipelineConfig};
/// use ndarray::Array3;
///
/// # fn main() -> docling_pdf_ml::Result<()> {
/// # let mut pipeline = Pipeline::new(PipelineConfig::default())?;
/// # let page_image = Array3::<u8>::zeros((792, 612, 3));
/// let page = pipeline.process_page(0, &page_image, 612.0, 792.0, None)?;
///
/// // Access assembled content
/// if let Some(assembled) = page.assembled {
///     log::debug!("Found {} elements", assembled.elements.len());
/// }
///
/// // Access raw predictions
/// if let Some(layout) = page.predictions.layout {
///     log::debug!("Detected {} clusters", layout.clusters.len());
/// }
/// # Ok(())
/// # }
/// ```
///
/// [`Pipeline::process_page`]: crate::Pipeline::process_page
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Page {
    /// Page number (0-indexed)
    pub page_no: usize,
    /// Page dimensions
    pub size: Option<Size>,
    /// Raw ML model predictions
    pub predictions: PagePredictions,
    /// Assembled document elements (if assembly was successful)
    pub assembled: Option<AssembledUnit>,
}

impl Page {
    /// Create a new empty page
    #[inline]
    #[must_use = "returns a new empty page"]
    pub fn new(page_no: usize) -> Self {
        Self {
            page_no,
            size: None,
            predictions: PagePredictions::default(),
            assembled: None,
        }
    }

    /// Create a new page with layout predictions
    #[inline]
    #[must_use = "returns a new page with layout predictions"]
    pub fn with_layout(page_no: usize, clusters: Vec<Cluster>) -> Self {
        Self {
            page_no,
            size: None,
            predictions: PagePredictions {
                layout: Some(LayoutPrediction { clusters }),
                ..Default::default()
            },
            assembled: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coord_origin_display() {
        assert_eq!(CoordOrigin::TopLeft.to_string(), "top-left");
        assert_eq!(CoordOrigin::BottomLeft.to_string(), "bottom-left");
    }

    #[test]
    fn test_doc_item_label_display() {
        assert_eq!(DocItemLabel::Text.to_string(), "Text");
        assert_eq!(DocItemLabel::SectionHeader.to_string(), "Section Header");
        assert_eq!(DocItemLabel::PageHeader.to_string(), "Page Header");
        assert_eq!(DocItemLabel::PageFooter.to_string(), "Page Footer");
        assert_eq!(DocItemLabel::Title.to_string(), "Title");
        assert_eq!(DocItemLabel::Caption.to_string(), "Caption");
        assert_eq!(DocItemLabel::Footnote.to_string(), "Footnote");
        assert_eq!(DocItemLabel::Table.to_string(), "Table");
        assert_eq!(DocItemLabel::Figure.to_string(), "Figure");
        assert_eq!(DocItemLabel::Picture.to_string(), "Picture");
        assert_eq!(DocItemLabel::Formula.to_string(), "Formula");
        assert_eq!(DocItemLabel::ListItem.to_string(), "List Item");
        assert_eq!(DocItemLabel::Code.to_string(), "Code");
        assert_eq!(
            DocItemLabel::CheckboxSelected.to_string(),
            "Checkbox (Selected)"
        );
        assert_eq!(
            DocItemLabel::CheckboxUnselected.to_string(),
            "Checkbox (Unselected)"
        );
        assert_eq!(DocItemLabel::Form.to_string(), "Form");
        assert_eq!(DocItemLabel::KeyValueRegion.to_string(), "Key-Value Region");
        assert_eq!(DocItemLabel::DocumentIndex.to_string(), "Document Index");
    }

    #[test]
    fn test_coord_origin_from_str() {
        use std::str::FromStr;

        // Exact matches
        assert_eq!(
            CoordOrigin::from_str("top-left").unwrap(),
            CoordOrigin::TopLeft
        );
        assert_eq!(
            CoordOrigin::from_str("bottom-left").unwrap(),
            CoordOrigin::BottomLeft
        );

        // Case insensitive
        assert_eq!(
            CoordOrigin::from_str("TOP-LEFT").unwrap(),
            CoordOrigin::TopLeft
        );
        assert_eq!(
            CoordOrigin::from_str("BOTTOM-LEFT").unwrap(),
            CoordOrigin::BottomLeft
        );

        // Without hyphens
        assert_eq!(
            CoordOrigin::from_str("topleft").unwrap(),
            CoordOrigin::TopLeft
        );
        assert_eq!(
            CoordOrigin::from_str("bottomleft").unwrap(),
            CoordOrigin::BottomLeft
        );

        // Short aliases
        assert_eq!(CoordOrigin::from_str("tl").unwrap(), CoordOrigin::TopLeft);
        assert_eq!(
            CoordOrigin::from_str("bl").unwrap(),
            CoordOrigin::BottomLeft
        );

        // Error case
        assert!(CoordOrigin::from_str("invalid").is_err());
    }

    #[test]
    fn test_coord_origin_roundtrip() {
        use std::str::FromStr;

        // Verify roundtrip: value -> Display -> FromStr -> value
        for origin in [CoordOrigin::TopLeft, CoordOrigin::BottomLeft] {
            let s = origin.to_string();
            let parsed = CoordOrigin::from_str(&s).unwrap();
            assert_eq!(origin, parsed);
        }
    }

    #[test]
    fn test_doc_item_label_from_str() {
        use std::str::FromStr;

        // Exact matches (case insensitive)
        assert_eq!(DocItemLabel::from_str("text").unwrap(), DocItemLabel::Text);
        assert_eq!(
            DocItemLabel::from_str("title").unwrap(),
            DocItemLabel::Title
        );
        assert_eq!(
            DocItemLabel::from_str("table").unwrap(),
            DocItemLabel::Table
        );
        assert_eq!(
            DocItemLabel::from_str("figure").unwrap(),
            DocItemLabel::Figure
        );
        assert_eq!(DocItemLabel::from_str("code").unwrap(), DocItemLabel::Code);
        assert_eq!(DocItemLabel::from_str("form").unwrap(), DocItemLabel::Form);

        // Multi-word labels
        assert_eq!(
            DocItemLabel::from_str("section_header").unwrap(),
            DocItemLabel::SectionHeader
        );
        assert_eq!(
            DocItemLabel::from_str("page-header").unwrap(),
            DocItemLabel::PageHeader
        );
        assert_eq!(
            DocItemLabel::from_str("page footer").unwrap(),
            DocItemLabel::PageFooter
        );
        assert_eq!(
            DocItemLabel::from_str("list_item").unwrap(),
            DocItemLabel::ListItem
        );
        assert_eq!(
            DocItemLabel::from_str("key-value-region").unwrap(),
            DocItemLabel::KeyValueRegion
        );
        assert_eq!(
            DocItemLabel::from_str("document_index").unwrap(),
            DocItemLabel::DocumentIndex
        );

        // Aliases
        assert_eq!(
            DocItemLabel::from_str("image").unwrap(),
            DocItemLabel::Picture
        );
        assert_eq!(
            DocItemLabel::from_str("equation").unwrap(),
            DocItemLabel::Formula
        );
        assert_eq!(
            DocItemLabel::from_str("kv").unwrap(),
            DocItemLabel::KeyValueRegion
        );
        assert_eq!(
            DocItemLabel::from_str("toc").unwrap(),
            DocItemLabel::DocumentIndex
        );

        // Checkbox variants
        assert_eq!(
            DocItemLabel::from_str("checkbox_selected").unwrap(),
            DocItemLabel::CheckboxSelected
        );
        assert_eq!(
            DocItemLabel::from_str("checkbox_unselected").unwrap(),
            DocItemLabel::CheckboxUnselected
        );

        // Error case
        assert!(DocItemLabel::from_str("invalid_label").is_err());
    }

    #[test]
    fn test_doc_item_label_roundtrip() {
        use std::str::FromStr;

        // Test all variants for roundtrip parsing
        // Note: Display uses spaces, FromStr normalizes them away
        let variants = [
            DocItemLabel::Text,
            DocItemLabel::Title,
            DocItemLabel::Caption,
            DocItemLabel::Footnote,
            DocItemLabel::Table,
            DocItemLabel::Figure,
            DocItemLabel::Picture,
            DocItemLabel::Formula,
            DocItemLabel::Code,
            DocItemLabel::Form,
            // Multi-word labels need special handling since Display uses spaces
            // but FromStr normalizes them
        ];

        for label in variants {
            let s = label.to_string();
            let parsed = DocItemLabel::from_str(&s).unwrap();
            assert_eq!(label, parsed);
        }
    }

    #[test]
    fn test_page_element_display() {
        // Create a minimal TextElement for testing
        let text_elem = TextElement {
            label: DocItemLabel::Text,
            id: 1,
            page_no: 0,
            text: "Hello world".to_string(),
            orig: "Hello world".to_string(),
            cluster: Cluster {
                id: 1,
                label: DocItemLabel::Text,
                bbox: BoundingBox {
                    l: 0.0,
                    t: 0.0,
                    r: 100.0,
                    b: 20.0,
                    coord_origin: CoordOrigin::TopLeft,
                },
                confidence: 0.95,
                cells: vec![],
                children: vec![],
            },
            captions: vec![],
            footnotes: vec![],
            is_bold: false,
            is_italic: false,
        };
        let page_elem = PageElement::Text(text_elem);
        assert_eq!(page_elem.to_string(), "Text[Text]: Hello world");

        // Test Table display
        let table_elem = TableElement {
            label: DocItemLabel::Table,
            id: 2,
            page_no: 0,
            text: None,
            cluster: Cluster {
                id: 2,
                label: DocItemLabel::Table,
                bbox: BoundingBox {
                    l: 0.0,
                    t: 0.0,
                    r: 200.0,
                    b: 100.0,
                    coord_origin: CoordOrigin::TopLeft,
                },
                confidence: 0.9,
                cells: vec![],
                children: vec![],
            },
            otsl_seq: vec![],
            num_rows: 3,
            num_cols: 4,
            table_cells: vec![],
            captions: vec![],
            footnotes: vec![],
        };
        let page_elem = PageElement::Table(table_elem);
        assert_eq!(page_elem.to_string(), "Table[3x4]");
    }
}
