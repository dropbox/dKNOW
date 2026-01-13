//! PDF rendering to PNG images using pdfium.
//!
//! This module provides PDF page rendering for LLM vision models.
//!
//! ## Usage
//!
//! ```no_run
//! use docling_llm_verify::PdfRenderer;
//! use std::path::Path;
//!
//! # fn example() -> anyhow::Result<()> {
//! let renderer = PdfRenderer::new()?;
//!
//! // Render all pages at 150 DPI
//! let pages = renderer.render_pages(Path::new("document.pdf"), 150)?;
//!
//! for page in &pages {
//!     println!("Page {}: {}x{} pts, {} bytes PNG",
//!         page.page_number,
//!         page.width_pts,
//!         page.height_pts,
//!         page.size()
//!     );
//! }
//!
//! // Or render a single page
//! let page_3 = renderer.render_page(Path::new("document.pdf"), 3, 300)?;
//!
//! // Get page count
//! let count = renderer.page_count(Path::new("document.pdf"))?;
//! # Ok(())
//! # }
//! ```
//!
//! ## DPI Recommendations
//!
//! - **72 DPI**: Fast, small files, suitable for quick previews
//! - **150 DPI**: Good balance for LLM vision models (recommended)
//! - **300 DPI**: High quality, larger files, better for small text
//!
//! Higher DPI means more tokens consumed by vision models and higher costs.

// Clippy pedantic allows:
// - DPI and dimension calculations involve various cast types
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]

use anyhow::{Context, Result};
use image::ImageFormat;
use pdfium_render::prelude::*;

/// PDF points per inch - standard PostScript/PDF unit conversion factor.
///
/// PDF dimensions are specified in "points" where 1 inch = 72 points.
/// Used to convert between PDF points and pixel dimensions at a given DPI.
const PDF_POINTS_PER_INCH: f32 = 72.0;
use std::path::Path;

/// Render a PDF page to PNG at the specified DPI.
#[derive(Debug)]
pub struct PdfRenderer {
    pdfium: Pdfium,
}

impl PdfRenderer {
    /// Create a new PDF renderer.
    ///
    /// # Errors
    ///
    /// This function currently never returns an error.
    #[must_use = "this returns a Result that should be handled"]
    pub fn new() -> Result<Self> {
        let pdfium = Pdfium::default();
        Ok(Self { pdfium })
    }

    /// Render all pages of a PDF to PNG images.
    ///
    /// # Errors
    ///
    /// Returns an error if the PDF cannot be loaded or rendering fails.
    #[must_use = "this function returns rendered pages that should be processed"]
    pub fn render_pages(&self, pdf_path: &Path, dpi: u32) -> Result<Vec<PageImage>> {
        let document = self
            .pdfium
            .load_pdf_from_file(pdf_path, None)
            .context("Failed to load PDF")?;

        let page_count = document.pages().len() as usize;
        let mut pages = Vec::with_capacity(page_count);

        for (i, page) in document.pages().iter().enumerate() {
            let page_num = (i + 1) as u32;

            // Get page dimensions
            let width = page.width().value;
            let height = page.height().value;

            // Render at specified DPI
            let render_config = PdfRenderConfig::new()
                .set_target_width((width * dpi as f32 / PDF_POINTS_PER_INCH) as i32)
                .set_target_height((height * dpi as f32 / PDF_POINTS_PER_INCH) as i32);

            let bitmap = page
                .render_with_config(&render_config)
                .context(format!("Failed to render page {page_num}"))?;

            // Convert to PNG bytes
            let image = bitmap.as_image();

            let mut png_bytes = Vec::new();
            image
                .write_to(&mut std::io::Cursor::new(&mut png_bytes), ImageFormat::Png)
                .context("Failed to encode PNG")?;

            pages.push(PageImage {
                page_number: page_num,
                width_pts: width,
                height_pts: height,
                png_data: png_bytes,
            });
        }

        Ok(pages)
    }

    /// Render a single page.
    ///
    /// # Errors
    ///
    /// Returns an error if the PDF cannot be loaded, page not found, or rendering fails.
    #[must_use = "this function returns a rendered page that should be processed"]
    pub fn render_page(&self, pdf_path: &Path, page_num: u32, dpi: u32) -> Result<PageImage> {
        let document = self
            .pdfium
            .load_pdf_from_file(pdf_path, None)
            .context("Failed to load PDF")?;

        let page_index = (page_num - 1) as u16;
        let page = document
            .pages()
            .get(page_index)
            .context(format!("Page {page_num} not found"))?;

        let width = page.width().value;
        let height = page.height().value;

        let render_config = PdfRenderConfig::new()
            .set_target_width((width * dpi as f32 / PDF_POINTS_PER_INCH) as i32)
            .set_target_height((height * dpi as f32 / PDF_POINTS_PER_INCH) as i32);

        let bitmap = page
            .render_with_config(&render_config)
            .context(format!("Failed to render page {page_num}"))?;

        let image = bitmap.as_image();

        let mut png_bytes = Vec::new();
        image
            .write_to(&mut std::io::Cursor::new(&mut png_bytes), ImageFormat::Png)
            .context("Failed to encode PNG")?;

        Ok(PageImage {
            page_number: page_num,
            width_pts: width,
            height_pts: height,
            png_data: png_bytes,
        })
    }

    /// Get the number of pages in a PDF.
    ///
    /// # Errors
    ///
    /// Returns an error if the PDF cannot be loaded.
    #[must_use = "this function returns a page count that should be used"]
    pub fn page_count(&self, pdf_path: &Path) -> Result<usize> {
        let document = self
            .pdfium
            .load_pdf_from_file(pdf_path, None)
            .context("Failed to load PDF")?;

        Ok(document.pages().len() as usize)
    }
}

/// Rendered page image with metadata.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PageImage {
    /// 1-based page number
    pub page_number: u32,
    /// Page width in PDF points (1/72 inch)
    pub width_pts: f32,
    /// Page height in PDF points (1/72 inch)
    pub height_pts: f32,
    /// PNG image data
    pub png_data: Vec<u8>,
}

impl PageImage {
    /// Size in bytes.
    #[inline]
    #[must_use = "returns PNG data size in bytes"]
    pub const fn size(&self) -> usize {
        self.png_data.len()
    }
}
