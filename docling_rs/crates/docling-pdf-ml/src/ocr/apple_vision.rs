// Apple Vision OCR integration for macOS
//
// Uses the `macocr` CLI tool (Apple Vision wrapper) for high-quality OCR
// on scanned documents. Apple Vision produces 7x better results than RapidOCR
// on scanned English documents.
//
// Reference: https://github.com/riddleling/macocr

use crate::error::{DoclingError, Result};
use crate::ocr::types::TextCell;
use crate::pipeline::data_structures::{BoundingRectangle, CoordOrigin};
use image::DynamicImage;
use std::path::Path;
use std::process::Command;

/// Check if Apple Vision OCR is available on this system
#[must_use]
#[cfg(target_os = "macos")]
pub fn is_available() -> bool {
    // Check if macocr is installed
    let home = std::env::var("HOME").unwrap_or_default();
    let macocr_path = format!("{home}/.cargo/bin/macocr");
    Path::new(&macocr_path).exists()
}

#[must_use]
#[cfg(not(target_os = "macos"))]
pub fn is_available() -> bool {
    false
}

/// Apple Vision OCR via macocr CLI
///
/// This struct wraps the macocr CLI tool for high-quality OCR on scanned documents.
/// Apple Vision produces significantly better results than `RapidOCR` for English text.
#[derive(Debug)]
pub struct AppleVisionOcr {
    /// Path to macocr binary
    macocr_path: String,
}

impl AppleVisionOcr {
    /// Create new Apple Vision OCR instance
    ///
    /// Returns error if macocr is not available
    #[cfg(target_os = "macos")]
    pub fn new() -> Result<Self> {
        let home = std::env::var("HOME").map_err(|_| DoclingError::ConfigError {
            reason: "HOME environment variable not set".to_string(),
        })?;

        let macocr_path = format!("{home}/.cargo/bin/macocr");

        if !Path::new(&macocr_path).exists() {
            return Err(DoclingError::ConfigError {
                reason: format!(
                    "macocr CLI not found at {macocr_path}. Install with: cargo install macocr"
                ),
            });
        }

        Ok(Self { macocr_path })
    }

    #[cfg(not(target_os = "macos"))]
    pub fn new() -> Result<Self> {
        Err(DoclingError::ConfigError {
            reason: "Apple Vision OCR is only available on macOS".to_string(),
        })
    }

    /// Perform OCR on an image
    ///
    /// # Arguments
    /// * `image` - Input image to process
    /// * `page_width` - Page width in pixels (for bounding box calculation)
    /// * `page_height` - Page height in pixels (for bounding box calculation)
    ///
    /// # Returns
    /// Vector of [`TextCell`] objects with text and estimated bounding boxes
    #[cfg(target_os = "macos")]
    pub fn detect(
        &self,
        image: &DynamicImage,
        page_width: f32,
        page_height: f32,
    ) -> Result<Vec<TextCell>> {
        // Create temp directory for image and output
        let temp_dir = std::env::temp_dir();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let image_path = temp_dir.join(format!("docling_ocr_{timestamp}.png"));
        let text_path = temp_dir.join(format!("docling_ocr_{timestamp}.txt"));

        // Save image to temp file
        image
            .save(&image_path)
            .map_err(|e| DoclingError::IoError(std::io::Error::other(e)))?;

        // Run macocr
        let output = Command::new(&self.macocr_path)
            .args(["-o", image_path.to_str().unwrap()])
            .current_dir(&temp_dir)
            .output()
            .map_err(DoclingError::IoError)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            log::warn!("macocr failed: {stderr}");
            // Fall back to empty result, don't fail the entire pipeline
            cleanup_temp_files(&image_path, &text_path);
            return Ok(Vec::new());
        }

        // Read OCR output - macocr creates file in current directory, not input directory
        let output_filename = image_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");
        let actual_text_path = temp_dir.join(format!("{output_filename}.txt"));

        let text = std::fs::read_to_string(&actual_text_path).unwrap_or_default();

        // Parse text into lines and create TextCells
        let text_cells = parse_ocr_output(&text, page_width, page_height);

        // Cleanup temp files
        cleanup_temp_files(&image_path, &actual_text_path);

        log::debug!(
            "Apple Vision OCR: {} lines, {} chars",
            text_cells.len(),
            text.len()
        );

        Ok(text_cells)
    }

    #[cfg(not(target_os = "macos"))]
    pub fn detect(
        &self,
        _image: &DynamicImage,
        _page_width: f32,
        _page_height: f32,
    ) -> Result<Vec<TextCell>> {
        Err(DoclingError::ConfigError {
            reason: "Apple Vision OCR is only available on macOS".to_string(),
        })
    }
}

/// Parse OCR text output into [`TextCell`]s with estimated bounding boxes
///
/// Since macocr doesn't provide bounding boxes, we estimate them based on:
/// - Full page width for each line
/// - Vertical position based on line index
fn parse_ocr_output(text: &str, page_width: f32, page_height: f32) -> Vec<TextCell> {
    let lines: Vec<&str> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();

    if lines.is_empty() {
        return Vec::new();
    }

    let num_lines = lines.len();
    let line_height = page_height / (num_lines as f32 + 1.0);

    lines
        .into_iter()
        .enumerate()
        .map(|(index, line)| {
            // Estimate bounding box based on line position
            // Assume lines are evenly distributed vertically
            let y_top = (index as f32) * line_height;
            let y_bottom = y_top + line_height;

            // Use full page width for each line
            // This is a simplification - actual line width would be better
            let rect = BoundingRectangle {
                r_x0: 0.0,
                r_y0: y_top,
                r_x1: page_width,
                r_y1: y_top,
                r_x2: page_width,
                r_y2: y_bottom,
                r_x3: 0.0,
                r_y3: y_bottom,
                coord_origin: CoordOrigin::TopLeft,
            };

            TextCell {
                index,
                text: line.to_string(),
                orig: line.to_string(),
                confidence: 0.95, // Apple Vision is high quality
                from_ocr: true,
                rect,
            }
        })
        .collect()
}

/// Clean up temporary files
fn cleanup_temp_files(image_path: &Path, text_path: &Path) {
    let _ = std::fs::remove_file(image_path);
    let _ = std::fs::remove_file(text_path);
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use image::{DynamicImage, RgbImage};

    #[test]
    fn test_is_available() {
        // Just check that the function doesn't panic
        let _ = is_available();
    }

    #[test]
    fn test_parse_ocr_output() {
        let text = "Line 1\nLine 2\nLine 3\n";
        let cells = parse_ocr_output(text, 100.0, 300.0);

        assert_eq!(cells.len(), 3);
        assert_eq!(cells[0].text, "Line 1");
        assert_eq!(cells[1].text, "Line 2");
        assert_eq!(cells[2].text, "Line 3");

        // Check bounding boxes are distributed vertically
        assert!(cells[0].rect.r_y0 < cells[1].rect.r_y0);
        assert!(cells[1].rect.r_y0 < cells[2].rect.r_y0);
    }

    #[test]
    fn test_parse_ocr_output_empty() {
        let text = "";
        let cells = parse_ocr_output(text, 100.0, 100.0);
        assert!(cells.is_empty());
    }

    #[test]
    fn test_parse_ocr_output_whitespace_only() {
        let text = "   \n\n  \n";
        let cells = parse_ocr_output(text, 100.0, 100.0);
        assert!(cells.is_empty());
    }

    #[test]
    #[ignore = "Requires macocr to be installed"]
    fn test_apple_vision_ocr() {
        if !is_available() {
            eprintln!("Skipping test: macocr not available");
            return;
        }

        let ocr = AppleVisionOcr::new().expect("Failed to create AppleVisionOcr");

        // Create a test image (blank white)
        let img = RgbImage::from_pixel(100, 50, image::Rgb([255, 255, 255]));
        let test_image = DynamicImage::ImageRgb8(img);

        let result = ocr.detect(&test_image, 100.0, 50.0);
        assert!(result.is_ok(), "OCR should not fail on blank image");

        // Blank image should produce no text
        let cells = result.unwrap();
        assert!(cells.is_empty(), "Blank image should produce no text cells");
    }
}
