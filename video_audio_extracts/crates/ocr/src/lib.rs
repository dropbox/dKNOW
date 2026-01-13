//! OCR module using Tesseract 5.x
//!
//! This module provides optical character recognition (OCR) capabilities using
//! Tesseract 5.x, an open-source OCR engine developed by Google.
//!
//! # Features
//! - Single-stage OCR (combined detection and recognition)
//! - 100+ language support via Tesseract
//! - Word and character-level bounding boxes
//! - Confidence scores for each text region
//! - Multiple page segmentation modes
//!
//! # Example
//! ```no_run
//! use video_audio_ocr::{OCRDetector, OCRConfig, OCRError};
//! use image::RgbImage;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let config = OCRConfig::default();
//! let detector = OCRDetector::new(config)?;
//!
//! // Create or load an RGB image
//! let img = RgbImage::new(100, 100);
//! let text_regions = detector.detect_text(&img)?;
//!
//! for region in text_regions {
//!     println!("Text: {} ({:.2}%)", region.text, region.confidence * 100.0);
//! }
//! # Ok(())
//! }
//! ```

pub mod plugin;

use image::RgbImage;
use leptess::{LepTess, Variable};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::debug;

/// Configuration for OCR processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OCRConfig {
    /// Tesseract language codes (e.g., "eng", "eng+fra")
    pub language: String,
    /// Minimum confidence threshold (0-100)
    pub min_confidence: i32,
    /// Page segmentation mode (see Tesseract PSM)
    pub page_segmentation_mode: u32,
    /// Character dictionary (not used by Tesseract, kept for compatibility)
    #[serde(default)]
    pub character_dict: Vec<String>,
    /// Detection threshold (not used by Tesseract, kept for compatibility)
    #[serde(default = "default_detection_threshold")]
    pub detection_threshold: f32,
    /// Recognition threshold (mapped to min_confidence)
    #[serde(default = "default_recognition_threshold")]
    pub recognition_threshold: f32,
}

fn default_detection_threshold() -> f32 {
    0.3
}

fn default_recognition_threshold() -> f32 {
    0.5
}

impl Default for OCRConfig {
    fn default() -> Self {
        Self {
            language: "eng".to_string(),
            min_confidence: 50, // 50% minimum confidence
            page_segmentation_mode: 3, // PSM_AUTO (fully automatic)
            character_dict: Vec::new(),
            detection_threshold: default_detection_threshold(),
            recognition_threshold: default_recognition_threshold(),
        }
    }
}

/// Bounding box with normalized coordinates (0.0-1.0)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Detected text region with content and location
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextRegion {
    /// Recognized text content
    pub text: String,
    /// Recognition confidence score (0.0-1.0)
    pub confidence: f32,
    /// Bounding box with normalized coordinates (0.0-1.0)
    pub bbox: BBox,
}

/// Errors that can occur during OCR processing
#[derive(Error, Debug)]
pub enum OCRError {
    #[error("Failed to initialize Tesseract: {0}")]
    InitError(String),

    #[error("Failed to run OCR: {0}")]
    RecognitionError(String),

    #[error("Invalid image dimensions: {0}")]
    InvalidImageDimensions(String),

    #[error("Processing error: {0}")]
    ProcessingError(#[from] video_audio_common::ProcessingError),
}

/// OCR detector using Tesseract
pub struct OCRDetector {
    config: OCRConfig,
}

impl OCRDetector {
    /// Create a new OCR detector
    pub fn new(config: OCRConfig) -> Result<Self, OCRError> {
        // Verify Tesseract can initialize with the specified language
        let _test_init = LepTess::new(None, &config.language).map_err(|e| {
            OCRError::InitError(format!(
                "Failed to initialize Tesseract with language '{}': {}. \
                 Make sure language data is installed (e.g., 'brew install tesseract-lang')",
                config.language, e
            ))
        })?;

        Ok(Self { config })
    }

    /// Detect and recognize text in an RGB image
    pub fn detect_text(&self, image: &RgbImage) -> Result<Vec<TextRegion>, OCRError> {
        let (width, height) = image.dimensions();

        if width == 0 || height == 0 {
            return Err(OCRError::InvalidImageDimensions(format!(
                "Image dimensions must be non-zero (got {}x{})",
                width, height
            )));
        }

        // Initialize Tesseract
        let mut lt = LepTess::new(None, &self.config.language).map_err(|e| {
            OCRError::InitError(format!("Failed to initialize Tesseract: {}", e))
        })?;

        // Set page segmentation mode
        lt.set_variable(
            Variable::TesseditPagesegMode,
            &self.config.page_segmentation_mode.to_string(),
        )
        .map_err(|e| OCRError::InitError(format!("Failed to set PSM: {}", e)))?;

        // Encode image to PNG in memory (leptess expects encoded image data)
        let mut png_buf = std::io::Cursor::new(Vec::new());
        image
            .write_to(&mut png_buf, image::ImageFormat::Png)
            .map_err(|e| {
                OCRError::RecognitionError(format!("Failed to encode image to PNG: {}", e))
            })?;

        // Set image from memory (leptess will decode the PNG)
        lt.set_image_from_mem(png_buf.get_ref()).map_err(|e| {
            OCRError::RecognitionError(format!("Failed to set image from memory: {}", e))
        })?;

        // Get text boxes at word level
        // Note: get_component_boxes() returns None if no text is detected (e.g., blank image)
        // This is not an error, just means the image has no text
        let boxes = match lt
            .get_component_boxes(leptess::capi::TessPageIteratorLevel_RIL_WORD, true)
        {
            Some(boxes) => boxes,
            None => return Ok(Vec::new()), // No text detected, return empty list
        };

        let mut text_regions = Vec::new();

        // Iterate through each detected word box
        for bbox in &boxes {
            // Get bounding box geometry
            let geom = bbox.get_geometry();

            // Set rectangle to restrict OCR to this box
            lt.set_rectangle(geom.x, geom.y, geom.w, geom.h);

            // Get text for this region
            let text = lt.get_utf8_text().unwrap_or_default().trim().to_string();

            if text.is_empty() {
                continue;
            }

            // Get confidence for this region (0-100 scale)
            let confidence = lt.mean_text_conf() as f32 / 100.0; // Convert 0-100 to 0.0-1.0

            // Filter by minimum confidence
            if (confidence * 100.0) as i32 >= self.config.min_confidence {
                // Normalize bounding box coordinates (0.0-1.0)
                let x = geom.x as f32 / width as f32;
                let y = geom.y as f32 / height as f32;
                let w = geom.w as f32 / width as f32;
                let h = geom.h as f32 / height as f32;

                debug!(
                    "OCR found text '{}' with confidence {:.2}% at ({:.3}, {:.3}, {:.3}, {:.3})",
                    text,
                    confidence * 100.0,
                    x,
                    y,
                    w,
                    h
                );

                text_regions.push(TextRegion {
                    text,
                    confidence,
                    bbox: BBox {
                        x,
                        y,
                        width: w,
                        height: h,
                    },
                });
            }
        }

        Ok(text_regions)
    }

    /// Compatibility method for plugin.rs that expects ONNX sessions
    ///
    /// This method provides a compatible interface with the old PaddleOCR implementation
    /// which used separate detection and recognition ONNX sessions. Tesseract uses a
    /// single-stage approach, so these sessions are ignored.
    ///
    /// # Arguments
    /// * `_det_session` - Detection session (ignored, kept for API compatibility)
    /// * `_rec_session` - Recognition session (ignored, kept for API compatibility)
    /// * `config` - OCR configuration
    /// * `image` - RGB image to process
    pub fn detect_text_with_sessions<S>(
        _det_session: &mut S,
        _rec_session: &mut S,
        config: &OCRConfig,
        image: &RgbImage,
    ) -> Result<Vec<TextRegion>, OCRError> {
        // Create a temporary detector with the provided config
        let detector = OCRDetector::new(config.clone())?;
        detector.detect_text(image)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};

    #[test]
    fn test_ocr_config_default() {
        let config = OCRConfig::default();
        assert_eq!(config.language, "eng");
        assert_eq!(config.min_confidence, 50);
        assert_eq!(config.page_segmentation_mode, 3);
    }

    #[test]
    fn test_detector_creation() {
        let config = OCRConfig::default();
        let result = OCRDetector::new(config);
        // This test will pass if Tesseract is installed with English data
        assert!(
            result.is_ok(),
            "Failed to create OCR detector. Make sure Tesseract is installed with English language data."
        );
    }

    #[test]
    fn test_detector_invalid_language() {
        let config = OCRConfig {
            language: "invalid_lang_xyz".to_string(),
            ..Default::default()
        };
        let result = OCRDetector::new(config);
        assert!(result.is_err(), "Should fail with invalid language");
    }

    #[test]
    fn test_detect_text_empty_image() {
        let config = OCRConfig::default();
        let detector = OCRDetector::new(config).expect("Failed to create detector");

        // Create a small solid color image (no text)
        let img = RgbImage::from_pixel(100, 100, Rgb([255, 255, 255]));
        let result = detector.detect_text(&img);

        // Should succeed but return no text regions (or very few low-confidence ones)
        assert!(result.is_ok());
        let regions = result.unwrap();
        // White image may have 0 regions or some noise detections filtered by confidence
        assert!(regions.len() < 5, "White image should have few or no text regions");
    }

    #[test]
    fn test_text_region_serialization() {
        let region = TextRegion {
            text: "HELLO".to_string(),
            confidence: 0.95,
            bbox: BBox {
                x: 0.1,
                y: 0.2,
                width: 0.3,
                height: 0.4,
            },
        };

        let json = serde_json::to_string(&region).expect("Failed to serialize");
        let deserialized: TextRegion =
            serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(region, deserialized);
    }
}
