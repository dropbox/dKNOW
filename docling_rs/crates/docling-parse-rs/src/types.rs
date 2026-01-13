//! Types for parsed PDF content
//!
//! These types represent the JSON structures returned by docling-parse C API.
//! The schema is based on actual JSON output analysis (see reports/DOCLING_PARSE_JSON_SCHEMA_2025-10-23-20-48.md)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Top-level result from docling-parse
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DoclingParseResult {
    /// Document metadata
    pub info: DoclingInfo,

    /// Parsed pages
    pub pages: Vec<DoclingParsePage>,

    /// Annotations (optional)
    #[serde(default)]
    pub annotations: Option<DoclingAnnotations>,

    /// Performance timings (optional)
    #[serde(default)]
    pub timings: Option<HashMap<String, serde_json::Value>>,
}

/// Document information
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DoclingInfo {
    /// Number of pages
    #[serde(rename = "#-pages")]
    pub num_pages: usize,

    /// Source filename
    pub filename: String,
}

/// Annotations (typically null/empty)
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DoclingAnnotations {
    pub form: Option<serde_json::Value>,
    pub language: Option<serde_json::Value>,
    pub meta_xml: Option<serde_json::Value>,
    pub table_of_contents: Option<serde_json::Value>,
}

/// A single parsed page
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DoclingParsePage {
    /// Page number (0-indexed)
    pub page_number: usize,

    /// Original (unsanitized) page data
    pub original: OriginalPageData,

    /// Sanitized page data (optional, only after post-processing)
    #[serde(default)]
    pub sanitized: Option<SanitizedPageData>,

    /// Page annotations (optional)
    #[serde(default)]
    pub annotations: Option<serde_json::Value>,

    /// Performance timings (optional)
    #[serde(default)]
    pub timings: Option<HashMap<String, serde_json::Value>>,
}

/// Original (raw) page data from PDF
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OriginalPageData {
    /// Page dimensions and bounding boxes
    pub dimension: PageDimension,

    /// Text line cells (merged horizontal text)
    pub line_cells: CellData,

    /// Word-level cells
    pub word_cells: CellData,

    /// Character-level cells (if available)
    #[serde(default)]
    pub char_cells: Option<CellData>,
}

/// Sanitized (post-processed) page data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SanitizedPageData {
    /// Updated dimensions after sanitization
    pub dimension: PageDimension,
    // Additional fields may be added after sanitization
}

/// Page dimension information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageDimension {
    /// Page rotation angle in degrees
    pub angle: f64,

    /// Main bounding box [x0, y0, x1, y1]
    pub bbox: [f64; 4],

    /// Page height
    pub height: f64,

    /// Page width
    pub width: f64,

    /// Page boundary type ("crop_box", "media_box", etc.)
    pub page_boundary: String,

    /// All rectangle types
    pub rectangles: PageRectangles,
}

/// All PDF page boundary rectangles
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PageRectangles {
    /// Art box [x0, y0, x1, y1]
    #[serde(rename = "art-bbox")]
    pub art_bbox: [f64; 4],

    /// Bleed box [x0, y0, x1, y1]
    #[serde(rename = "bleed-bbox")]
    pub bleed_bbox: [f64; 4],

    /// Crop box [x0, y0, x1, y1]
    #[serde(rename = "crop-bbox")]
    pub crop_bbox: [f64; 4],

    /// Media box [x0, y0, x1, y1]
    #[serde(rename = "media-bbox")]
    pub media_bbox: [f64; 4],

    /// Trim box [x0, y0, x1, y1]
    #[serde(rename = "trim-bbox")]
    pub trim_bbox: [f64; 4],
}

/// Container for cell data arrays
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CellData {
    /// Array of cell arrays
    /// Each cell is a 21-element array: [coords(12), text(1), metadata(8)]
    pub data: Vec<CellArray>,
}

/// A single cell represented as a 21-element array
///
/// Array format:
/// - \[0-3\]: Bounding box (x0, y0, x1, y1)
/// - \[4-11\]: Quadrilateral points (x0, y0, x1, y1, x2, y2, x3, y3)
/// - \[12\]: Text content (string)
/// - \[13\]: Unknown integer (typically -1)
/// - \[14\]: Font size (float)
/// - \[15\]: Unknown string (typically empty)
/// - \[16\]: Font category ("STANDARD", "TYPE1", "TYPE3", "CID")
/// - \[17\]: Font ID (e.g., "/F38")
/// - \[18\]: Font name (e.g., "/CHJOZT+CMBX12")
/// - \[19\]: Unknown boolean
/// - \[20\]: Unknown boolean
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CellArray {
    /// Bounding box coordinates
    pub bbox: CellBBox,

    /// Quadrilateral points (for rotated text)
    pub quad_points: [f64; 8],

    /// Text content
    pub text: String,

    /// Font size
    pub font_size: f64,

    /// Font category
    pub font_category: String,

    /// Font ID (PDF reference)
    pub font_id: String,

    /// Font name (embedded font name)
    pub font_name: String,

    /// Unknown integer field (index 13)
    pub unknown_int: i32,

    /// Unknown string field (index 15)
    pub unknown_str: String,

    /// Unknown boolean fields (indices 19-20)
    pub unknown_bools: [bool; 2],
}

impl<'de> Deserialize<'de> for CellArray {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{Error, SeqAccess, Visitor};

        struct CellArrayVisitor;

        impl<'de> Visitor<'de> for CellArrayVisitor {
            type Value = CellArray;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a 21-element array")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                // Read 21 elements
                let x0 = seq
                    .next_element()?
                    .ok_or_else(|| Error::invalid_length(0, &self))?;
                let y0 = seq
                    .next_element()?
                    .ok_or_else(|| Error::invalid_length(1, &self))?;
                let x1 = seq
                    .next_element()?
                    .ok_or_else(|| Error::invalid_length(2, &self))?;
                let y1 = seq
                    .next_element()?
                    .ok_or_else(|| Error::invalid_length(3, &self))?;
                let qx0 = seq
                    .next_element()?
                    .ok_or_else(|| Error::invalid_length(4, &self))?;
                let qy0 = seq
                    .next_element()?
                    .ok_or_else(|| Error::invalid_length(5, &self))?;
                let qx1 = seq
                    .next_element()?
                    .ok_or_else(|| Error::invalid_length(6, &self))?;
                let qy1 = seq
                    .next_element()?
                    .ok_or_else(|| Error::invalid_length(7, &self))?;
                let qx2 = seq
                    .next_element()?
                    .ok_or_else(|| Error::invalid_length(8, &self))?;
                let qy2 = seq
                    .next_element()?
                    .ok_or_else(|| Error::invalid_length(9, &self))?;
                let qx3 = seq
                    .next_element()?
                    .ok_or_else(|| Error::invalid_length(10, &self))?;
                let qy3 = seq
                    .next_element()?
                    .ok_or_else(|| Error::invalid_length(11, &self))?;
                let text = seq
                    .next_element()?
                    .ok_or_else(|| Error::invalid_length(12, &self))?;
                let unknown_int = seq
                    .next_element()?
                    .ok_or_else(|| Error::invalid_length(13, &self))?;
                let font_size = seq
                    .next_element()?
                    .ok_or_else(|| Error::invalid_length(14, &self))?;
                let unknown_str = seq
                    .next_element()?
                    .ok_or_else(|| Error::invalid_length(15, &self))?;
                let font_category = seq
                    .next_element()?
                    .ok_or_else(|| Error::invalid_length(16, &self))?;
                let font_id = seq
                    .next_element()?
                    .ok_or_else(|| Error::invalid_length(17, &self))?;
                let font_name = seq
                    .next_element()?
                    .ok_or_else(|| Error::invalid_length(18, &self))?;
                let bool1 = seq
                    .next_element()?
                    .ok_or_else(|| Error::invalid_length(19, &self))?;
                let bool2 = seq
                    .next_element()?
                    .ok_or_else(|| Error::invalid_length(20, &self))?;

                Ok(CellArray {
                    bbox: CellBBox { x0, y0, x1, y1 },
                    quad_points: [qx0, qy0, qx1, qy1, qx2, qy2, qx3, qy3],
                    text,
                    font_size,
                    font_category,
                    font_id,
                    font_name,
                    unknown_int,
                    unknown_str,
                    unknown_bools: [bool1, bool2],
                })
            }
        }

        deserializer.deserialize_seq(CellArrayVisitor)
    }
}

/// Bounding box for a cell
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct CellBBox {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}
