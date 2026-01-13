// OCR data structures
//
// Port of OcrStruct.h from RapidOcrOnnx

use crate::pipeline::data_structures::BoundingRectangle;

// =============================================================================
// OCR Postprocessing Constants
// =============================================================================

/// Long side threshold for filtering elongated text boxes.
///
/// Text boxes with (`long_side` / `short_side`) > 3.0 are considered artifacts.
/// Reference: DbNet.cpp:64-65
pub const OCR_LONG_SIDE_THRESH: f32 = 3.0;

/// Maximum number of contour candidates to process.
///
/// Limits processing time for images with many detected regions.
/// Reference: DbNet.cpp:73-74
pub const OCR_MAX_CANDIDATES: usize = 1000;

/// OCR model input height (shared by `AngleNet` and `CrnnNet`).
///
/// Both angle classification and text recognition models expect height=48.
/// Reference:
/// - AngleNet.h:36 (dstHeight = 48)
/// - CrnnNet.cpp:125-148 (resize to height 48)
pub const OCR_MODEL_HEIGHT: usize = 48;

/// `AngleNet` model input width.
///
/// Angle classification model expects width=192 after aspect-ratio resize.
/// Reference: AngleNet.h:35 (dstWidth = 192)
pub const ANGLENET_WIDTH: u32 = 192;

/// `CrnnNet` (recognition) model max input width.
///
/// Recognition model expects max width=320 after aspect-ratio resize.
/// Reference: CrnnNet.cpp configuration
pub const CRNN_MAX_WIDTH: usize = 320;

/// Normalization divisor for OCR models (`AngleNet`, `CrnnNet`).
///
/// Formula: pixel / 127.5 - 1.0 maps [0, 255] to [-1, 1]
/// This is equivalent to (pixel / 255.0 - 0.5) / 0.5
///
/// Reference:
/// - AngleNet.h:33-34 (`meanValues[3]` = `{127.5, 127.5, 127.5}`)
/// - CrnnNet.cpp (same normalization for CRNN model)
pub const OCR_NORMALIZE_DIVISOR: f32 = 127.5;

/// Parameters for OCR detection
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DetectionParams {
    /// Global preprocessing: max side length before detection (default: 2000)
    /// If max(h, w) > `max_side_len`, scale down while preserving aspect ratio
    pub max_side_len: u32,
    /// Global preprocessing: min side length before detection (default: 30)
    /// If min(h, w) < `min_side_len`, scale up while preserving aspect ratio
    pub min_side_len: u32,
    /// Detection preprocessing: target side length (default: 736)
    /// Applied after global preprocessing, respecting `limit_type`
    pub limit_side_len: u32,
    /// Detection preprocessing: limit type (default: `"min"`)
    /// `"min"` = scale so min(h, w) >= `limit_side_len`
    /// `"max"` = scale so max(h, w) <= `limit_side_len`
    pub limit_type: LimitType,
    /// Minimum confidence score for detected text boxes (default: 0.5, matches Python `box_thresh`)
    pub box_score_thresh: f32,
    /// Binary threshold for detection mask (default: 0.3, matches Python thresh)
    pub box_thresh: f32,
    /// Expand detected boxes by this ratio (default: 1.6, matches Python `unclip_ratio`)
    pub unclip_ratio: f32,
}

/// Limit type for detection preprocessing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LimitType {
    /// Scale so min(h, w) >= `limit_side_len`
    #[default]
    Min,
    /// Scale so max(h, w) <= `limit_side_len`
    Max,
}

impl std::fmt::Display for LimitType {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Min => write!(f, "min"),
            Self::Max => write!(f, "max"),
        }
    }
}

impl std::str::FromStr for LimitType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "min" | "minimum" => Ok(Self::Min),
            "max" | "maximum" => Ok(Self::Max),
            _ => Err(format!("Unknown limit type '{s}'. Expected: min, max")),
        }
    }
}

impl Default for DetectionParams {
    #[inline]
    fn default() -> Self {
        Self {
            max_side_len: 2000,         // Python Global.max_side_len
            min_side_len: 30,           // Python Global.min_side_len
            limit_side_len: 736,        // Python Det.limit_side_len
            limit_type: LimitType::Min, // Python Det.limit_type = "min"
            box_score_thresh: 0.5,      // Python default: box_thresh = 0.5
            box_thresh: 0.3,            // Python default: thresh = 0.3
            unclip_ratio: 1.6,          // Python default: unclip_ratio = 1.6
        }
    }
}

/// Parameters for full OCR pipeline
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OcrParams {
    pub detection: DetectionParams,
    /// Minimum confidence score for final text results (default: 0.5, matches Python `text_score`)
    pub text_score: f32,
}

impl Default for OcrParams {
    #[inline]
    fn default() -> Self {
        Self {
            detection: DetectionParams::default(),
            text_score: 0.5, // Python default: text_score = 0.5
        }
    }
}

/// Detected text box from `DbNet`
///
/// Reference: OcrStruct.h:27-31
#[derive(Debug, Clone, PartialEq)]
pub struct TextBox {
    /// Four corners of the text box (clockwise from top-left)
    /// Each point is (x, y) in image coordinates
    pub corners: Vec<(f32, f32)>,
    /// Detection confidence score (0.0 - 1.0)
    pub score: f32,
}

/// Text rotation classification from `AngleNet`
///
/// Reference: OcrStruct.h:33-37
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Angle {
    /// Rotation index: 0 = 0°, 1 = 180°
    pub index: usize,
    /// Classification confidence score (0.0 - 1.0)
    pub score: f32,
}

/// Recognized text line from `CrnnNet`
///
/// Reference: OcrStruct.h:39-43
#[derive(Debug, Clone, PartialEq)]
pub struct TextLine {
    /// Recognized text string
    pub text: String,
    /// Per-character confidence scores
    pub char_scores: Vec<f32>,
}

/// Final OCR text cell (output format)
///
/// This matches the `TextCell` format used in the pipeline
#[derive(Debug, Clone, PartialEq)]
pub struct TextCell {
    /// Cell index
    pub index: usize,
    /// Recognized text
    pub text: String,
    /// Original text (same as text for OCR cells)
    pub orig: String,
    /// Overall confidence score (average of char scores)
    pub confidence: f32,
    /// Always true for OCR cells
    pub from_ocr: bool,
    /// Bounding rectangle in page coordinates
    pub rect: BoundingRectangle,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_limit_type_display() {
        assert_eq!(LimitType::Min.to_string(), "min");
        assert_eq!(LimitType::Max.to_string(), "max");
    }

    #[test]
    fn test_limit_type_from_str() {
        // Exact matches
        assert_eq!("min".parse::<LimitType>().unwrap(), LimitType::Min);
        assert_eq!("max".parse::<LimitType>().unwrap(), LimitType::Max);

        // Case insensitive
        assert_eq!("MIN".parse::<LimitType>().unwrap(), LimitType::Min);
        assert_eq!("Max".parse::<LimitType>().unwrap(), LimitType::Max);

        // Aliases
        assert_eq!("minimum".parse::<LimitType>().unwrap(), LimitType::Min);
        assert_eq!("maximum".parse::<LimitType>().unwrap(), LimitType::Max);

        // Invalid
        assert!("invalid".parse::<LimitType>().is_err());
    }

    #[test]
    fn test_limit_type_roundtrip() {
        for limit_type in [LimitType::Min, LimitType::Max] {
            let s = limit_type.to_string();
            let parsed: LimitType = s.parse().unwrap();
            assert_eq!(parsed, limit_type);
        }
    }
}
