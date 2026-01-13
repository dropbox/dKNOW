/// Shared types for modular pipeline stages
///
/// These types mirror the Python `docling_modular` types and use serde for JSON serialization
/// to enable cross-language testing and validation.
use serde::{Deserialize, Serialize};

/// Bounding box with left, top, right, bottom coordinates
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BBox {
    pub l: f64, // left (x_min)
    pub t: f64, // top (y_min)
    pub r: f64, // right (x_max)
    pub b: f64, // bottom (y_max)
}

impl BBox {
    /// Create a new bounding box
    #[inline]
    #[must_use = "returns a new BBox instance"]
    pub const fn new(l: f64, t: f64, r: f64, b: f64) -> Self {
        Self { l, t, r, b }
    }

    /// Calculate area of the bounding box
    ///
    /// N=373: Inlined for performance (called in nested loops)
    /// N=592: Use `abs()` to match Python's `BoundingBox.area()`
    /// Python: `abs(self.r - self.l) * abs(self.b - self.t)`
    #[inline]
    #[must_use = "returns the bounding box area"]
    pub fn area(&self) -> f64 {
        let width = (self.r - self.l).abs();
        let height = (self.b - self.t).abs();
        width * height
    }

    /// Calculate intersection area with another bbox
    ///
    /// N=373: Inlined for performance (called in nested loops)
    #[inline]
    #[must_use = "returns the intersection area"]
    pub fn intersection_area(&self, other: &Self) -> f64 {
        let x_left = self.l.max(other.l);
        let y_top = self.t.max(other.t);
        let x_right = self.r.min(other.r);
        let y_bottom = self.b.min(other.b);

        let width = (x_right - x_left).max(0.0);
        let height = (y_bottom - y_top).max(0.0);

        width * height
    }

    /// Calculate intersection-over-self ratio (used for cell assignment)
    /// This is `intersection_area` / self.area, NOT `IoU`!
    ///
    /// N=373: Inlined for performance (called in nested loops during cell assignment)
    #[inline]
    #[must_use = "returns the intersection-over-self ratio"]
    pub fn intersection_over_self(&self, other: &Self) -> f64 {
        let intersection = self.intersection_area(other);
        let self_area = self.area();

        if self_area > 0.0 {
            intersection / self_area
        } else {
            0.0
        }
    }

    /// Calculate Intersection over Union (`IoU`) with another bbox
    ///
    /// N=373: Inlined for performance (called in overlap resolution)
    #[inline]
    #[must_use = "returns the Intersection over Union value"]
    pub fn iou(&self, other: &Self) -> f64 {
        let intersection = self.intersection_area(other);
        let union = self.area() + other.area() - intersection;

        if union > 0.0 {
            intersection / union
        } else {
            0.0
        }
    }
}

/// Text cell from OCR (preprocessing stage)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextCell {
    pub text: String,
    pub bbox: BBox,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    /// N=4373: Whether text is bold (from PDF font flags)
    #[serde(default)]
    pub is_bold: bool,
    /// N=4373: Whether text is italic (from PDF font flags)
    #[serde(default)]
    pub is_italic: bool,
}

/// Labeled cluster from Stage 3 (HF post-processing)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LabeledCluster {
    pub id: usize,
    pub label: String,
    pub bbox: BBox,
    pub confidence: f64,
    pub class_id: i32,
}

/// Container for labeled clusters (Stage 3 output)
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LabeledClusters {
    pub clusters: Vec<LabeledCluster>,
}

/// Container for OCR cells (preprocessing output)
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct OCRCells {
    pub cells: Vec<TextCell>,
}

/// Cluster with assigned cells (Stage 4 output)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClusterWithCells {
    pub id: usize,
    pub label: String,
    pub bbox: BBox,
    pub confidence: f64,
    pub class_id: i32,
    pub cells: Vec<TextCell>,
}

/// Container for clusters with cells (Stage 4 output)
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ClustersWithCells {
    pub clusters: Vec<ClusterWithCells>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bbox_area() {
        let bbox = BBox::new(0.0, 0.0, 10.0, 5.0);
        assert_eq!(bbox.area(), 50.0);
    }

    #[test]
    fn test_bbox_intersection() {
        let bbox1 = BBox::new(0.0, 0.0, 10.0, 10.0);
        let bbox2 = BBox::new(5.0, 5.0, 15.0, 15.0);

        assert_eq!(bbox1.intersection_area(&bbox2), 25.0);
        assert_eq!(bbox2.intersection_area(&bbox1), 25.0);
    }

    #[test]
    fn test_bbox_intersection_over_self() {
        let bbox1 = BBox::new(0.0, 0.0, 10.0, 10.0); // Area = 100
        let bbox2 = BBox::new(5.0, 5.0, 15.0, 15.0); // Area = 100

        // Intersection = 25
        assert_eq!(bbox1.intersection_over_self(&bbox2), 0.25); // 25 / 100
        assert_eq!(bbox2.intersection_over_self(&bbox1), 0.25); // 25 / 100
    }

    #[test]
    fn test_bbox_no_intersection() {
        let bbox1 = BBox::new(0.0, 0.0, 5.0, 5.0);
        let bbox2 = BBox::new(10.0, 10.0, 15.0, 15.0);

        assert_eq!(bbox1.intersection_area(&bbox2), 0.0);
        assert_eq!(bbox1.intersection_over_self(&bbox2), 0.0);
    }

    #[test]
    fn test_bbox_serialization() {
        let bbox = BBox::new(1.5, 2.5, 3.5, 4.5);
        let json = serde_json::to_string(&bbox).unwrap();
        let deserialized: BBox = serde_json::from_str(&json).unwrap();

        assert_eq!(bbox, deserialized);
    }
}
