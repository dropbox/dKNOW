//! OCR fallback for scanned documents
//!
//! This module provides OCR (Optical Character Recognition) support for extracting
//! text from scanned PDFs and images. It uses:
//! - `pdfium-render` to render PDF pages to images
//! - `docling-ocr` with PaddleOCR models for text recognition
//!
//! # Usage
//!
//! OCR is enabled via the `ocr` feature flag. The OCR engine requires ONNX models
//! to be downloaded to the docling-ocr assets directory.
//!
//! # Environment Variables
//!
//! - `DOCLING_OCR_ASSETS`: Path to OCR model assets directory
//!
//! # Example
//!
//! ```ignore
//! use sg_core::ocr::ocr_pdf;
//!
//! // OCR a scanned PDF
//! let text = ocr_pdf("/path/to/scanned.pdf")?;
//! println!("Extracted: {}", text);
//! ```

use anyhow::{Context, Result};
use std::path::Path;

use docling_ocr::OcrEngine;
use image::DynamicImage;
use pdfium_render::prelude::*;

/// Minimum characters threshold for considering text extraction successful.
/// PDFs with less text than this are considered "scanned" and eligible for OCR.
pub const MIN_TEXT_THRESHOLD: usize = 50;

/// Default DPI for rendering PDF pages to images for OCR.
/// Higher DPI = better OCR quality but slower processing.
pub const DEFAULT_OCR_DPI: f32 = 300.0;

/// OCR engine wrapper with lazy initialization
pub struct PdfOcrEngine {
    ocr: OcrEngine,
    pdfium: Pdfium,
    dpi: f32,
}

impl PdfOcrEngine {
    /// Create a new PDF OCR engine
    ///
    /// This initializes both pdfium (for PDF rendering) and the OCR engine
    /// (for text recognition).
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - pdfium library cannot be loaded
    /// - OCR models cannot be found
    pub fn new() -> Result<Self> {
        Self::with_dpi(DEFAULT_OCR_DPI)
    }

    /// Create a new PDF OCR engine with custom DPI
    ///
    /// # Arguments
    ///
    /// * `dpi` - Resolution for rendering PDF pages (default: 300)
    pub fn with_dpi(dpi: f32) -> Result<Self> {
        // Initialize pdfium
        let pdfium = Pdfium::new(
            Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
                .or_else(|_| Pdfium::bind_to_system_library())
                .context("Failed to bind pdfium library")?,
        );

        // Initialize OCR engine
        let ocr = OcrEngine::new().context("Failed to initialize OCR engine")?;

        Ok(Self { ocr, pdfium, dpi })
    }

    /// OCR a PDF file, returning the extracted text
    ///
    /// This renders each page to an image and performs OCR on it.
    /// Pages are processed sequentially and text is combined with page separators.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the PDF file
    ///
    /// # Returns
    ///
    /// The OCR'd text with pages separated by double newlines.
    pub fn ocr_pdf(&mut self, path: &Path) -> Result<String> {
        let document = self
            .pdfium
            .load_pdf_from_file(path, None)
            .with_context(|| format!("Failed to load PDF: {}", path.display()))?;

        let mut all_text = String::new();
        let page_count = document.pages().len();

        tracing::debug!(
            "OCR: processing {} pages from {}",
            page_count,
            path.display()
        );

        for (page_idx, page) in document.pages().iter().enumerate() {
            // Render page to image
            let image = self.render_page_to_image(&page)?;

            // OCR the page
            match self.ocr.recognize(&image) {
                Ok(result) => {
                    let page_text = result.text();
                    if !page_text.trim().is_empty() {
                        if !all_text.is_empty() {
                            all_text.push_str("\n\n");
                        }
                        all_text.push_str(&page_text);
                    }
                    tracing::trace!(
                        "OCR page {}/{}: {} chars, confidence {:.2}",
                        page_idx + 1,
                        page_count,
                        page_text.len(),
                        result.avg_confidence
                    );
                }
                Err(e) => {
                    tracing::warn!("OCR failed for page {}/{}: {}", page_idx + 1, page_count, e);
                }
            }
        }

        Ok(all_text)
    }

    /// OCR a single image file
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the image file (PNG, JPEG, etc.)
    pub fn ocr_image(&mut self, path: &Path) -> Result<String> {
        let image = image::open(path)
            .with_context(|| format!("Failed to open image: {}", path.display()))?;

        let result = self.ocr.recognize(&image).context("OCR failed")?;

        Ok(result.text())
    }

    /// OCR a DynamicImage directly
    pub fn ocr_dynamic_image(&mut self, image: &DynamicImage) -> Result<String> {
        let result = self.ocr.recognize(image).context("OCR failed")?;
        Ok(result.text())
    }

    /// Render a PDF page to a DynamicImage
    fn render_page_to_image(&self, page: &PdfPage) -> Result<DynamicImage> {
        // Calculate dimensions based on DPI
        let width = page.width();
        let height = page.height();
        let scale = self.dpi / 72.0; // PDF points are 72 per inch

        let pixel_width = (width.value * scale) as i32;
        let pixel_height = (height.value * scale) as i32;

        // Render to bitmap
        let bitmap = page
            .render_with_config(
                &PdfRenderConfig::new()
                    .set_target_width(pixel_width)
                    .set_target_height(pixel_height)
                    .render_form_data(true)
                    .render_annotations(true),
            )
            .context("Failed to render PDF page")?;

        // Convert to DynamicImage (bitmap.as_image() returns DynamicImage)
        Ok(bitmap.as_image())
    }
}

/// OCR a PDF file using the default settings
///
/// This is a convenience function that creates a temporary OCR engine,
/// processes the PDF, and returns the text.
///
/// For processing multiple PDFs, create a `PdfOcrEngine` instance and
/// reuse it for better performance.
///
/// # Arguments
///
/// * `path` - Path to the PDF file
///
/// # Returns
///
/// The OCR'd text, or an error if OCR fails.
///
/// # Example
///
/// ```ignore
/// let text = ocr_pdf("/path/to/scanned.pdf")?;
/// ```
pub fn ocr_pdf<P: AsRef<Path>>(path: P) -> Result<String> {
    let mut engine = PdfOcrEngine::new()?;
    engine.ocr_pdf(path.as_ref())
}

/// Check if text is likely from a scanned document (too short for real content)
///
/// This helps decide whether OCR should be attempted on a PDF.
///
/// # Arguments
///
/// * `text` - The extracted text to check
/// * `threshold` - Minimum character count (default: MIN_TEXT_THRESHOLD)
pub fn is_likely_scanned(text: &str, threshold: Option<usize>) -> bool {
    let threshold = threshold.unwrap_or(MIN_TEXT_THRESHOLD);
    text.trim().len() < threshold
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_likely_scanned() {
        assert!(is_likely_scanned("", None));
        assert!(is_likely_scanned("   ", None));
        assert!(is_likely_scanned("short text", None));
        assert!(!is_likely_scanned(&"a".repeat(100), None));
        assert!(is_likely_scanned(&"a".repeat(100), Some(200)));
    }

    #[test]
    fn test_min_text_threshold() {
        assert_eq!(MIN_TEXT_THRESHOLD, 50);
    }

    #[test]
    fn test_default_ocr_dpi() {
        assert_eq!(DEFAULT_OCR_DPI, 300.0);
    }

    // Note: Integration tests require pdfium library and OCR models
    // Run with: cargo test --features ocr -- --ignored

    #[test]
    #[ignore]
    fn test_ocr_engine_creation() {
        let engine = PdfOcrEngine::new();
        // May fail if pdfium or OCR models not available
        if engine.is_err() {
            println!(
                "OCR engine creation failed (expected without models): {:?}",
                engine.err()
            );
        }
    }
}
