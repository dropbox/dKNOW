//! Visualization rendering module
//!
//! Renders PDF pages with bounding box overlays for AI visual testing.
//! Generates PNG screenshots with element labels and JSON sidecars.

use crate::{DlvizElement, DlvizLabel, DlvizStage};
use ab_glyph::{FontRef, PxScale};
use image::{ImageBuffer, Rgba, RgbaImage};
use imageproc::drawing::{draw_hollow_rect_mut, draw_text_mut};
use imageproc::rect::Rect;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Color mapping for each label type (RGBA)
#[inline]
const fn label_color(label: DlvizLabel) -> Rgba<u8> {
    match label {
        DlvizLabel::SectionHeader => Rgba([66, 135, 245, 255]), // Blue
        DlvizLabel::Text => Rgba([128, 128, 128, 255]),         // Gray
        DlvizLabel::Table | DlvizLabel::CheckboxSelected => Rgba([76, 175, 80, 255]), // Green
        DlvizLabel::Picture => Rgba([255, 193, 7, 255]),        // Yellow
        DlvizLabel::Caption => Rgba([255, 152, 0, 255]),        // Orange
        DlvizLabel::Formula => Rgba([244, 67, 54, 255]),        // Red
        DlvizLabel::Title => Rgba([156, 39, 176, 255]),         // Purple
        DlvizLabel::PageHeader | DlvizLabel::PageFooter => Rgba([158, 158, 158, 255]), // Light Gray
        DlvizLabel::Footnote => Rgba([121, 85, 72, 255]),       // Brown
        DlvizLabel::ListItem => Rgba([0, 188, 212, 255]),       // Cyan
        DlvizLabel::Code => Rgba([96, 125, 139, 255]),          // Blue Gray
        DlvizLabel::CheckboxUnselected => Rgba([189, 189, 189, 255]), // Gray
        DlvizLabel::DocumentIndex => Rgba([63, 81, 181, 255]),  // Indigo
        DlvizLabel::Form => Rgba([233, 30, 99, 255]),           // Pink
        DlvizLabel::KeyValueRegion => Rgba([255, 87, 34, 255]), // Deep Orange
    }
}

/// Get label name as string
#[inline]
const fn label_name(label: DlvizLabel) -> &'static str {
    match label {
        DlvizLabel::Caption => "caption",
        DlvizLabel::Footnote => "footnote",
        DlvizLabel::Formula => "formula",
        DlvizLabel::ListItem => "list",
        DlvizLabel::PageFooter => "footer",
        DlvizLabel::PageHeader => "header",
        DlvizLabel::Picture => "picture",
        DlvizLabel::SectionHeader => "section",
        DlvizLabel::Table => "table",
        DlvizLabel::Text => "text",
        DlvizLabel::Title => "title",
        DlvizLabel::Code => "code",
        DlvizLabel::CheckboxSelected | DlvizLabel::CheckboxUnselected => "checkbox",
        DlvizLabel::DocumentIndex => "index",
        DlvizLabel::Form => "form",
        DlvizLabel::KeyValueRegion => "kv",
    }
}

/// Visualization rendering options
#[derive(Debug, Clone, PartialEq)]
pub struct RenderOptions {
    /// Line thickness for bounding boxes
    pub line_thickness: u32,
    /// Font scale for labels
    pub font_scale: f32,
    /// Whether to show confidence values
    pub show_confidence: bool,
    /// Whether to show reading order numbers
    pub show_reading_order: bool,
    /// Background label color opacity (0-255)
    pub label_bg_opacity: u8,
}

impl Default for RenderOptions {
    #[inline]
    fn default() -> Self {
        Self {
            line_thickness: 2,
            font_scale: 14.0,
            show_confidence: true,
            show_reading_order: true,
            label_bg_opacity: 200,
        }
    }
}

/// Element info for JSON sidecar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElementInfo {
    /// Unique element identifier.
    pub id: u32,
    /// Element type label (e.g., "text", "table", "figure").
    pub label: String,
    /// Model confidence score (0.0 to 1.0).
    pub confidence: f32,
    /// Bounding box coordinates.
    pub bbox: BBoxInfo,
    /// Optional text content of the element.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Reading order index (-1 if not assigned).
    pub reading_order: i32,
}

/// Bounding box info for JSON.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct BBoxInfo {
    /// Left edge x-coordinate in page units.
    pub x: f32,
    /// Top edge y-coordinate in page units.
    pub y: f32,
    /// Width of the bounding box.
    pub width: f32,
    /// Height of the bounding box.
    pub height: f32,
}

/// Page size info for JSON.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct PageSizeInfo {
    /// Page width in page units.
    pub width: f32,
    /// Page height in page units.
    pub height: f32,
}

/// Statistics for JSON sidecar.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ElementStatistics {
    /// Count of elements grouped by label type.
    pub by_label: std::collections::HashMap<String, usize>,
    /// Average confidence score across all elements.
    pub avg_confidence: f32,
    /// Number of elements with confidence below threshold.
    pub low_confidence_count: usize,
}

/// JSON sidecar data for visualization metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VisualizationSidecar {
    /// Source PDF filename.
    pub pdf: String,
    /// Page number (0-indexed).
    pub page: usize,
    /// Page dimensions.
    pub page_size: PageSizeInfo,
    /// Processing stage name (e.g., "layout", "ocr").
    pub stage: String,
    /// Time taken to render visualization in milliseconds.
    pub render_time_ms: f64,
    /// Total number of elements.
    pub element_count: usize,
    /// List of detected elements.
    pub elements: Vec<ElementInfo>,
    /// Aggregate statistics for this page.
    pub statistics: ElementStatistics,
}

/// Render visualization with bounding box overlays
///
/// Takes an RGBA buffer from PDF rendering and draws element bounding boxes
/// with labels and reading order numbers.
///
/// # Panics
///
/// Panics if:
/// - The page buffer cannot be converted to an `RgbaImage` (invalid size or format)
/// - The embedded `DejaVuSansMono` font cannot be loaded (should never happen in practice)
pub fn render_visualization(
    page_buffer: &[u8],
    page_width: u32,
    page_height: u32,
    page_height_pts: f32,
    scale: f32,
    elements: &[DlvizElement],
    options: &RenderOptions,
) -> RgbaImage {
    // Create image from buffer
    let mut img: RgbaImage = ImageBuffer::from_raw(page_width, page_height, page_buffer.to_vec())
        .expect("Failed to create image from buffer");

    // Load embedded font using ab_glyph
    let font_data = include_bytes!("../assets/DejaVuSansMono.ttf");
    let font = FontRef::try_from_slice(font_data).expect("Failed to load font");
    let font_scale = PxScale::from(options.font_scale);

    // Draw each element
    for element in elements {
        let color = label_color(element.label);

        // Convert PDF coordinates (origin bottom-left) to image coordinates (origin top-left)
        // PDF: y=0 at bottom, Image: y=0 at top
        let x = (element.bbox.x * scale) as i32;
        let y = ((page_height_pts - element.bbox.y - element.bbox.height) * scale) as i32;
        let w = (element.bbox.width * scale) as u32;
        let h = (element.bbox.height * scale) as u32;

        // Clamp to image bounds
        let x = x.max(0) as u32;
        let y = y.max(0) as u32;
        let w = w.min(page_width.saturating_sub(x));
        let h = h.min(page_height.saturating_sub(y));

        if w > 0 && h > 0 {
            // Draw bounding box with line thickness
            for t in 0..options.line_thickness {
                let inner_w = w.saturating_sub(2 * t);
                let inner_h = h.saturating_sub(2 * t);
                if inner_w > 0 && inner_h > 0 {
                    let rect = Rect::at((x + t) as i32, (y + t) as i32).of_size(inner_w, inner_h);
                    draw_hollow_rect_mut(&mut img, rect, color);
                }
            }

            // Draw label text
            let label_text = if options.show_reading_order && element.reading_order >= 0 {
                if options.show_confidence {
                    format!(
                        "{} [{}] {:.0}%",
                        element.reading_order,
                        label_name(element.label),
                        element.confidence * 100.0
                    )
                } else {
                    format!("{} [{}]", element.reading_order, label_name(element.label))
                }
            } else if options.show_confidence {
                format!(
                    "[{}] {:.0}%",
                    label_name(element.label),
                    element.confidence * 100.0
                )
            } else {
                format!("[{}]", label_name(element.label))
            };

            // Draw label background
            let text_y = if y < 20 {
                y + h + 2
            } else {
                y.saturating_sub(18)
            };
            let bg_w = (label_text.len() * 8) as u32;
            let bg_h = 16u32;

            // Fill background rectangle manually
            let bg_color = Rgba([color.0[0], color.0[1], color.0[2], options.label_bg_opacity]);
            for py in text_y..(text_y + bg_h).min(page_height) {
                for px in x..(x + bg_w).min(page_width) {
                    img.put_pixel(px, py, bg_color);
                }
            }

            // Draw text
            draw_text_mut(
                &mut img,
                Rgba([255, 255, 255, 255]),
                x as i32 + 2,
                text_y as i32 + 1,
                font_scale,
                &font,
                &label_text,
            );
        }
    }

    img
}

/// Generate JSON sidecar for visualization
pub fn generate_sidecar(
    pdf_name: &str,
    page_num: usize,
    page_size: (f32, f32),
    stage: DlvizStage,
    render_time_ms: f64,
    elements: &[DlvizElement],
    element_texts: &[Option<String>],
) -> VisualizationSidecar {
    let mut by_label: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut total_confidence = 0.0;
    let mut low_confidence_count = 0;

    let element_infos: Vec<ElementInfo> = elements
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let label_str = label_name(e.label).to_string();
            *by_label.entry(label_str.clone()).or_insert(0) += 1;
            total_confidence += e.confidence;
            if e.confidence < 0.7 {
                low_confidence_count += 1;
            }

            ElementInfo {
                id: e.id,
                label: label_str,
                confidence: e.confidence,
                bbox: BBoxInfo {
                    x: e.bbox.x,
                    y: e.bbox.y,
                    width: e.bbox.width,
                    height: e.bbox.height,
                },
                text: element_texts.get(i).cloned().flatten(),
                reading_order: e.reading_order,
            }
        })
        .collect();

    let avg_confidence = if elements.is_empty() {
        0.0
    } else {
        total_confidence / elements.len() as f32
    };

    let stage_name = match stage {
        DlvizStage::RawPdf => "raw_pdf",
        DlvizStage::OcrDetection => "ocr_detection",
        DlvizStage::OcrRecognition => "ocr_recognition",
        DlvizStage::LayoutDetection => "layout_detection",
        DlvizStage::CellAssignment => "cell_assignment",
        DlvizStage::EmptyClusterRemoval => "empty_cluster_removal",
        DlvizStage::OrphanDetection => "orphan_detection",
        DlvizStage::BBoxAdjust1 => "bbox_adjust_1",
        DlvizStage::BBoxAdjust2 => "bbox_adjust_2",
        DlvizStage::FinalAssembly => "final_assembly",
        DlvizStage::ReadingOrder => "reading_order",
    };

    VisualizationSidecar {
        pdf: pdf_name.to_string(),
        page: page_num,
        page_size: PageSizeInfo {
            width: page_size.0,
            height: page_size.1,
        },
        stage: stage_name.to_string(),
        render_time_ms,
        element_count: elements.len(),
        elements: element_infos,
        statistics: ElementStatistics {
            by_label,
            avg_confidence,
            low_confidence_count,
        },
    }
}

/// Save visualization to PNG file
///
/// # Errors
///
/// Returns an error if the file cannot be saved.
#[must_use = "this function returns a Result that should be checked for errors"]
pub fn save_visualization(img: &RgbaImage, path: &Path) -> Result<(), String> {
    img.save(path)
        .map_err(|e| format!("Failed to save PNG: {e}"))
}

/// Save sidecar to JSON file
///
/// # Errors
///
/// Returns an error if serialization or file writing fails.
#[must_use = "this function returns a Result that should be checked for errors"]
pub fn save_sidecar(sidecar: &VisualizationSidecar, path: &Path) -> Result<(), String> {
    let json = serde_json::to_string_pretty(sidecar)
        .map_err(|e| format!("Failed to serialize sidecar: {e}"))?;
    std::fs::write(path, json).map_err(|e| format!("Failed to write sidecar: {e}"))
}

/// Validation result for layout quality
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutValidationResult {
    /// Layout output is valid with semantic labels
    Valid,
    /// Warning: output may have issues but is usable
    Warning(String),
    /// Error: layout output is invalid (likely ML model failure)
    Error(String),
}

/// Validate layout output quality to catch ML model failures
///
/// Detects issues like:
/// - All elements having the same label (often indicates fallback to heuristics)
/// - All elements having confidence 1.0 (indicates native text cells, not ML predictions)
///
/// # Arguments
/// * `elements` - Layout elements to validate
///
/// # Returns
/// * `LayoutValidationResult` - Valid, Warning, or Error with description
///
/// # Examples
/// ```ignore
/// let result = validate_layout_quality(&elements);
/// match result {
///     LayoutValidationResult::Error(msg) => {
///         log::error!("Layout validation failed: {}", msg);
///     }
///     LayoutValidationResult::Warning(msg) => {
///         log::warn!("Layout validation warning: {}", msg);
///     }
///     LayoutValidationResult::Valid => {}
/// }
/// ```
pub fn validate_layout_quality(elements: &[DlvizElement]) -> LayoutValidationResult {
    use std::collections::HashSet;

    // Empty is valid (might be blank page)
    if elements.is_empty() {
        return LayoutValidationResult::Valid;
    }

    // Check 1: Label variety - should have more than just "text"
    let unique_labels: HashSet<DlvizLabel> = elements.iter().map(|e| e.label).collect();
    if unique_labels.len() == 1 && unique_labels.contains(&DlvizLabel::Text) && elements.len() > 5 {
        return LayoutValidationResult::Error(format!(
            "All {} elements labeled 'text'. ML model may not be classifying correctly. \
             Expected semantic labels (title, section_header, etc.)",
            elements.len()
        ));
    }

    // Check 2: Confidence variety - ML predictions should vary
    let confidences: Vec<f32> = elements.iter().map(|e| e.confidence).collect();
    let all_same = confidences
        .iter()
        .all(|&c| (c - confidences[0]).abs() < 0.001);
    if all_same && confidences[0] > 0.99 && elements.len() > 5 {
        return LayoutValidationResult::Error(format!(
            "All {} elements have confidence={:.2}. \
             This suggests native text cells, not ML predictions.",
            elements.len(),
            confidences[0]
        ));
    }

    // Check 3: Academic/document papers should have title or section headers
    let has_title = unique_labels.contains(&DlvizLabel::Title);
    let has_section = unique_labels.contains(&DlvizLabel::SectionHeader);
    if elements.len() > 20 && !has_title && !has_section {
        return LayoutValidationResult::Warning(format!(
            "{} elements but no title or section_header detected. \
             Layout model may not be classifying semantic structure.",
            elements.len()
        ));
    }

    LayoutValidationResult::Valid
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_label_colors_are_distinct() {
        let labels = [
            DlvizLabel::SectionHeader,
            DlvizLabel::Text,
            DlvizLabel::Table,
            DlvizLabel::Picture,
            DlvizLabel::Caption,
            DlvizLabel::Formula,
            DlvizLabel::Title,
        ];

        let colors: Vec<_> = labels.iter().map(|l| label_color(*l)).collect();

        // Check that colors are distinct
        for i in 0..colors.len() {
            for j in (i + 1)..colors.len() {
                assert_ne!(
                    colors[i], colors[j],
                    "Labels {:?} and {:?} have same color",
                    labels[i], labels[j]
                );
            }
        }
    }

    #[test]
    fn test_label_names() {
        assert_eq!(label_name(DlvizLabel::Text), "text");
        assert_eq!(label_name(DlvizLabel::Table), "table");
        assert_eq!(label_name(DlvizLabel::SectionHeader), "section");
    }

    fn make_element(label: DlvizLabel, confidence: f32) -> DlvizElement {
        use crate::DlvizBBox;
        DlvizElement {
            id: 0,
            bbox: DlvizBBox {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 50.0,
            },
            label,
            confidence,
            reading_order: 0,
        }
    }

    #[test]
    fn test_validate_layout_quality_valid() {
        // Mix of labels with varying confidence = valid
        let elements = vec![
            make_element(DlvizLabel::Title, 0.96),
            make_element(DlvizLabel::Text, 0.87),
            make_element(DlvizLabel::Text, 0.92),
            make_element(DlvizLabel::SectionHeader, 0.89),
            make_element(DlvizLabel::Text, 0.85),
            make_element(DlvizLabel::Text, 0.91),
        ];
        assert_eq!(
            validate_layout_quality(&elements),
            LayoutValidationResult::Valid
        );
    }

    #[test]
    fn test_validate_layout_quality_all_text_error() {
        // All elements labeled "text" = error (likely ML failure)
        let elements = vec![
            make_element(DlvizLabel::Text, 0.9),
            make_element(DlvizLabel::Text, 0.9),
            make_element(DlvizLabel::Text, 0.9),
            make_element(DlvizLabel::Text, 0.9),
            make_element(DlvizLabel::Text, 0.9),
            make_element(DlvizLabel::Text, 0.9),
        ];
        match validate_layout_quality(&elements) {
            LayoutValidationResult::Error(msg) => {
                assert!(msg.contains("All 6 elements labeled 'text'"));
            }
            other => panic!("Expected Error, got {:?}", other),
        }
    }

    #[test]
    fn test_validate_layout_quality_all_confidence_1_error() {
        // All elements with confidence 1.0 = error (native text cells, not ML)
        let elements = vec![
            make_element(DlvizLabel::Text, 1.0),
            make_element(DlvizLabel::SectionHeader, 1.0),
            make_element(DlvizLabel::Title, 1.0),
            make_element(DlvizLabel::Text, 1.0),
            make_element(DlvizLabel::Text, 1.0),
            make_element(DlvizLabel::Text, 1.0),
        ];
        match validate_layout_quality(&elements) {
            LayoutValidationResult::Error(msg) => {
                assert!(msg.contains("confidence=1.00"));
            }
            other => panic!("Expected Error, got {:?}", other),
        }
    }

    #[test]
    fn test_validate_layout_quality_empty_valid() {
        // Empty is valid (blank page)
        assert_eq!(validate_layout_quality(&[]), LayoutValidationResult::Valid);
    }

    #[test]
    fn test_validate_layout_quality_few_elements_no_error() {
        // Few elements (<=5) don't trigger errors even if all same
        let elements = vec![
            make_element(DlvizLabel::Text, 1.0),
            make_element(DlvizLabel::Text, 1.0),
            make_element(DlvizLabel::Text, 1.0),
        ];
        assert_eq!(
            validate_layout_quality(&elements),
            LayoutValidationResult::Valid
        );
    }
}
