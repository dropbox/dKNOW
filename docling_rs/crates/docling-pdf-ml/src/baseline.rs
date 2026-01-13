/// Internal baseline data structures used during PDF processing.
///
/// These types are used internally for converting between ML model outputs
/// and the pipeline's data structures. They are not part of the public API.
use serde::{Deserialize, Serialize};

/// Bounding box in baseline format (f64 coordinates).
///
/// Used internally for baseline data interchange. Production code uses
/// [`crate::pipeline::data_structures::BoundingBox`] with f32 coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
pub struct BBox {
    pub l: f64,
    pub t: f64,
    pub r: f64,
    pub b: f64,
}

/// OCR text cell in baseline format.
///
/// Used internally for OCR model outputs before conversion to pipeline format.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct OcrCell {
    pub text: String,
    pub bbox: BBox,
    #[serde(default)]
    pub confidence: Option<f64>,
}

/// Layout cluster in baseline format.
///
/// Used internally to convert ML model outputs to pipeline [`Cluster`](crate::pipeline::data_structures::Cluster) types.
/// The layout predictor returns results in this format which are then converted
/// to the pipeline's internal representation.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct LayoutCluster {
    #[serde(default)]
    pub id: i32,
    pub label: String,
    pub confidence: f64,
    pub bbox: BBox,
}

/// Raw predictor output format (flattened bbox).
///
/// Intermediate format from ML postprocessing before conversion to [`LayoutCluster`].
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct RawPrediction {
    pub l: f64,
    pub t: f64,
    pub r: f64,
    pub b: f64,
    pub label: String,
    pub confidence: f64,
}

impl RawPrediction {
    /// Convert to `LayoutCluster` with assigned ID.
    ///
    /// Normalizes label format (lowercase, underscores instead of spaces/hyphens).
    #[inline]
    #[must_use = "returns a new LayoutCluster, does not modify self"]
    pub fn to_cluster(self, id: i32) -> LayoutCluster {
        LayoutCluster {
            id,
            label: self.label.to_lowercase().replace([' ', '-'], "_"),
            confidence: self.confidence,
            bbox: BBox {
                l: self.l,
                t: self.t,
                r: self.r,
                b: self.b,
            },
        }
    }
}

/// Result of layout quality validation.
///
/// Used to detect broken ML model output early in the pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutValidationResult {
    /// Layout is valid - normal ML output
    Valid,
    /// Warning - potential issue but not definitive
    Warning(String),
    /// Error - definitely broken output
    Error(String),
}

/// Validate layout cluster quality to detect ML model failures.
///
/// This catches issues like:
/// - All elements having the same label (indicates model not differentiating)
/// - All elements having confidence=1.0 (indicates non-ML source like PDF text cells)
/// - No semantic structure in large documents (missing titles/headers)
///
/// # Arguments
/// * `clusters` - Layout clusters from ML inference
///
/// # Returns
/// `LayoutValidationResult` indicating valid output, warning, or error.
///
/// # Examples
/// ```ignore
/// let result = validate_layout_clusters(&clusters);
/// match result {
///     LayoutValidationResult::Error(msg) => {
///         log::error!("Layout validation failed: {}", msg);
///         return Err(DoclingError::ValidationError { message: msg });
///     }
///     LayoutValidationResult::Warning(msg) => {
///         log::warn!("Layout validation warning: {}", msg);
///     }
///     LayoutValidationResult::Valid => {}
/// }
/// ```
#[must_use = "validation result should be checked"]
pub fn validate_layout_clusters(clusters: &[LayoutCluster]) -> LayoutValidationResult {
    use std::collections::HashSet;

    // Empty is valid (might be blank page)
    if clusters.is_empty() {
        return LayoutValidationResult::Valid;
    }

    // Few elements (≤5) don't trigger errors - might be a simple page
    let min_elements_for_validation = 5;
    if clusters.len() <= min_elements_for_validation {
        return LayoutValidationResult::Valid;
    }

    // Check 1: Are all elements the same label?
    let unique_labels: HashSet<&str> = clusters.iter().map(|c| c.label.as_str()).collect();

    if unique_labels.len() == 1 {
        let only_label = unique_labels.iter().next().unwrap();
        return LayoutValidationResult::Error(format!(
            "All {} elements labeled '{}'. ML model likely failed to differentiate document structure. \
             Check INT8 quantization or model weights.",
            clusters.len(),
            only_label
        ));
    }

    // Check 2: Are all confidences exactly 1.0?
    let all_confidence_one = clusters
        .iter()
        .all(|c| (c.confidence - 1.0).abs() < f64::EPSILON);

    if all_confidence_one {
        return LayoutValidationResult::Error(format!(
            "All {} elements have confidence=1.00. This indicates PDF native text cells \
             were used instead of ML inference. Check model loading.",
            clusters.len()
        ));
    }

    // Check 3: For large documents, expect some semantic structure
    let has_title = unique_labels.contains("title");
    let has_section = unique_labels.contains("section_header");
    let has_semantic_structure = has_title || has_section;

    let large_doc_threshold = 15;
    if clusters.len() > large_doc_threshold && !has_semantic_structure {
        // This is just a warning - some documents legitimately have no titles
        return LayoutValidationResult::Warning(format!(
            "Document has {} elements but no title or section_header detected. \
             This may indicate ML model quality issues.",
            clusters.len()
        ));
    }

    LayoutValidationResult::Valid
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cluster(label: &str, confidence: f64) -> LayoutCluster {
        LayoutCluster {
            id: 0,
            label: label.to_string(),
            confidence,
            bbox: BBox {
                l: 0.0,
                t: 0.0,
                r: 100.0,
                b: 100.0,
            },
        }
    }

    #[test]
    fn test_validate_layout_clusters_valid() {
        // Mix of labels with varying confidence = valid
        let clusters = vec![
            make_cluster("title", 0.96),
            make_cluster("section_header", 0.92),
            make_cluster("text", 0.94),
            make_cluster("table", 0.89),
            make_cluster("picture", 0.95),
            make_cluster("text", 0.91),
        ];
        assert_eq!(
            validate_layout_clusters(&clusters),
            LayoutValidationResult::Valid
        );
    }

    #[test]
    fn test_validate_layout_clusters_all_text_error() {
        // All elements labeled "text" = error (likely ML failure)
        let clusters = vec![
            make_cluster("text", 0.9),
            make_cluster("text", 0.85),
            make_cluster("text", 0.92),
            make_cluster("text", 0.88),
            make_cluster("text", 0.9),
            make_cluster("text", 0.9),
        ];
        match validate_layout_clusters(&clusters) {
            LayoutValidationResult::Error(msg) => {
                assert!(msg.contains("All 6 elements labeled 'text'"));
            }
            _ => panic!("Expected Error, got Valid or Warning"),
        }
    }

    #[test]
    fn test_validate_layout_clusters_all_confidence_1_error() {
        // All elements with confidence 1.0 = error (native text cells, not ML)
        // Use different labels so we don't trigger "all same label" check first
        let clusters = vec![
            make_cluster("text", 1.0),
            make_cluster("title", 1.0),
            make_cluster("section_header", 1.0),
            make_cluster("table", 1.0),
            make_cluster("picture", 1.0),
            make_cluster("caption", 1.0),
        ];
        match validate_layout_clusters(&clusters) {
            LayoutValidationResult::Error(msg) => {
                assert!(msg.contains("confidence=1.00"));
            }
            _ => panic!("Expected Error, got Valid or Warning"),
        }
    }

    #[test]
    fn test_validate_layout_clusters_empty_valid() {
        // Empty is valid (blank page)
        assert_eq!(validate_layout_clusters(&[]), LayoutValidationResult::Valid);
    }

    #[test]
    fn test_validate_layout_clusters_few_elements_no_error() {
        // Few elements (<=5) don't trigger errors even if all same
        let clusters = vec![
            make_cluster("text", 1.0),
            make_cluster("text", 1.0),
            make_cluster("text", 1.0),
        ];
        assert_eq!(
            validate_layout_clusters(&clusters),
            LayoutValidationResult::Valid
        );
    }

    #[test]
    fn test_validate_layout_clusters_large_doc_warning() {
        // Large doc without title/section_header = warning
        // Use mix of labels (text, table, picture, etc.) but no title or section_header
        let labels = ["text", "table", "picture", "caption", "list_item"];
        let clusters: Vec<LayoutCluster> = (0..20)
            .map(|i| make_cluster(labels[i % labels.len()], 0.8 + (i as f64 * 0.01)))
            .collect();
        match validate_layout_clusters(&clusters) {
            LayoutValidationResult::Warning(msg) => {
                assert!(msg.contains("no title or section_header"));
            }
            _ => panic!("Expected Warning"),
        }
    }
}
