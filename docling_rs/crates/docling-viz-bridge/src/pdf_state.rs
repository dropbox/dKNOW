//! PDF document state management
//!
//! This module manages the state of loaded PDF documents for visualization.
//! It conditionally compiles based on the `pdf-render` feature.

use std::path::PathBuf;

#[cfg(feature = "pdf-render")]
use crate::pipeline_integration::PipelineRunner;

/// Cached page metadata
#[derive(Debug, Clone, PartialEq)]
pub struct PageMetadata {
    /// Width in points
    pub width: f32,
    /// Height in points
    pub height: f32,
}

/// Internal PDF document state
#[cfg(feature = "pdf-render")]
pub struct PdfState {
    /// Path to the loaded PDF file
    pub path: PathBuf,
    /// Raw PDF bytes (owned for lifetime management)
    pdf_bytes: Vec<u8>,
    /// Cached page metadata
    page_metadata: Vec<PageMetadata>,
    /// ML pipeline runner for inference
    pipeline_runner: PipelineRunner,
}

#[cfg(feature = "pdf-render")]
impl PdfState {
    /// Load a PDF from the given path
    #[must_use = "loading a PDF returns a result that should be handled"]
    pub fn load(path: &str) -> Result<Self, String> {
        use pdfium_render::prelude::*;

        // Read the PDF file into memory
        let pdf_bytes = std::fs::read(path)
            .map_err(|e| format!("Failed to read PDF file '{}': {}", path, e))?;

        // Initialize pdfium - try system library first, then bundled
        let pdfium = Pdfium::new(
            Pdfium::bind_to_system_library()
                .or_else(|_| {
                    Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
                })
                .map_err(|e| format!("Failed to bind pdfium library: {e}"))?,
        );

        // Load document from bytes to extract metadata
        // Note: We use a separate binding for metadata extraction because
        // the document borrows the slice, and we want to keep pdf_bytes owned
        let bytes_for_metadata = pdf_bytes.clone();
        let document = pdfium
            .load_pdf_from_byte_slice(&bytes_for_metadata, None)
            .map_err(|e| format!("Failed to parse PDF: {e}"))?;

        // Cache page metadata
        let page_metadata: Vec<PageMetadata> = document
            .pages()
            .iter()
            .map(|page| PageMetadata {
                width: page.width().value,
                height: page.height().value,
            })
            .collect();

        // Drop document before returning to release borrow
        drop(document);

        // Initialize pipeline runner (may fail if models not available)
        let pipeline_runner = PipelineRunner::new();

        Ok(Self {
            path: PathBuf::from(path),
            pdf_bytes,
            page_metadata,
            pipeline_runner,
        })
    }

    /// Check if ML pipeline is available
    #[inline]
    #[must_use = "returns whether ML pipeline is available"]
    pub fn has_ml_pipeline(&self) -> bool {
        self.pipeline_runner.is_available()
    }

    /// Get ML pipeline initialization error (if any)
    #[inline]
    #[must_use = "returns the ML pipeline initialization error if any"]
    pub fn ml_init_error(&self) -> Option<&str> {
        self.pipeline_runner.init_error()
    }

    /// Run ML pipeline on a specific page
    ///
    /// Renders the page and processes it through the ML pipeline,
    /// capturing stage snapshots for visualization.
    #[must_use = "running ML pipeline returns a result that should be handled"]
    pub fn run_ml_pipeline(&mut self, page_num: usize, scale: f32) -> Result<(), String> {
        // Get page dimensions
        let (page_width_pts, page_height_pts) = self
            .page_size(page_num)
            .ok_or_else(|| format!("Page {page_num} not found"))?;

        // Extract text cells from PDF for element population
        let text_cells = self.extract_text_cells(page_num);
        if text_cells.is_none() || text_cells.as_ref().is_some_and(|c| c.is_empty()) {
            log::warn!(
                "Page {}: No text cells available and OCR is disabled. \
                 Document elements may have empty text. \
                 Enable OCR with .ocr_enabled(true) for scanned documents.",
                page_num
            );
        }

        // Render the page to get the image
        let (width, height, rgba_data) = self
            .render_page(page_num, scale)
            .ok_or_else(|| format!("Failed to render page {page_num}"))?;

        // Process through ML pipeline with text cells
        self.pipeline_runner.process_page(
            page_num,
            &rgba_data,
            width,
            height,
            page_width_pts,
            page_height_pts,
            text_cells.as_deref(),
        )
    }

    /// Get snapshot for a specific page and stage
    #[inline]
    #[must_use = "returns the snapshot data for a page and stage"]
    pub fn get_ml_snapshot(
        &self,
        page_num: usize,
        stage: crate::DlvizStage,
    ) -> Option<&crate::pipeline_integration::StageSnapshotData> {
        self.pipeline_runner.get_snapshot(page_num, stage)
    }

    /// Check if a page has been processed by ML pipeline
    #[inline]
    #[must_use = "returns whether the page has been processed by ML"]
    pub fn has_ml_page(&self, page_num: usize) -> bool {
        self.pipeline_runner.has_page(page_num)
    }

    /// Clear all ML pipeline results
    #[inline]
    pub fn clear_ml_results(&mut self) {
        self.pipeline_runner.clear();
    }

    /// Get text content of an element from ML pipeline results
    ///
    /// # Arguments
    /// - `page_num`: Page number (0-indexed)
    /// - `element_id`: Element ID from snapshot
    ///
    /// # Returns
    /// The text content if available, or None if element not found or has no text
    #[inline]
    #[must_use = "returns the text content of an element"]
    pub fn get_ml_element_text(&self, page_num: usize, element_id: u32) -> Option<&str> {
        self.pipeline_runner.get_element_text(page_num, element_id)
    }

    /// Get the number of pages in the document
    #[inline]
    #[must_use = "returns the number of pages in the document"]
    pub fn page_count(&self) -> usize {
        self.page_metadata.len()
    }

    /// Get the document filename
    #[inline]
    #[must_use = "returns the document filename"]
    pub fn document_path(&self) -> Option<&str> {
        self.path.file_name()?.to_str()
    }

    /// Get page dimensions in points
    #[inline]
    #[must_use = "returns the page dimensions in points"]
    pub fn page_size(&self, page_num: usize) -> Option<(f32, f32)> {
        self.page_metadata
            .get(page_num)
            .map(|m| (m.width, m.height))
    }

    /// Extract text cells from a page
    ///
    /// Returns a vector of text cells with their bounding boxes and text content.
    /// Coordinates are in PDF points with origin at top-left.
    pub fn extract_text_cells(
        &self,
        page_num: usize,
    ) -> Option<Vec<crate::pipeline_integration::TextCell>> {
        use pdfium_render::prelude::*;

        // Re-bind pdfium for this operation
        let pdfium = Pdfium::new(
            Pdfium::bind_to_system_library()
                .or_else(|_| {
                    Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
                })
                .ok()?,
        );

        let document = pdfium
            .load_pdf_from_byte_slice(&self.pdf_bytes, None)
            .ok()?;
        let page = document.pages().get(page_num as u16).ok()?;
        let page_height = page.height().value;

        // Get text segments from the page
        let text = page.text().ok()?;
        let segments = text.segments();

        let mut cells = Vec::new();
        for (idx, segment) in segments.iter().enumerate() {
            let text_content = segment.text();
            if text_content.trim().is_empty() {
                continue;
            }

            let bounds = segment.bounds();
            // pdfium-render PdfRect uses left, top, right, bottom
            // Convert to top-left origin coordinates
            let l = bounds.left().value;
            let t = page_height - bounds.top().value; // Convert from bottom-left to top-left
            let r = bounds.right().value;
            let b = page_height - bounds.bottom().value; // Convert from bottom-left to top-left

            cells.push(crate::pipeline_integration::TextCell {
                index: idx,
                text: text_content,
                l,
                t: t.min(b), // Ensure t < b (top is smaller y)
                r,
                b: t.max(b),     // Ensure b > t (bottom is larger y)
                confidence: 1.0, // Native PDF text has high confidence
                from_ocr: false,
            });
        }

        log::debug!(
            "Extracted {} text cells from page {}",
            cells.len(),
            page_num
        );

        Some(cells)
    }

    /// Render a page to an RGBA image buffer
    ///
    /// Returns (width, height, rgba_bytes) or None on error
    #[must_use = "returns the rendered page as RGBA buffer"]
    pub fn render_page(&self, page_num: usize, scale: f32) -> Option<(u32, u32, Vec<u8>)> {
        use pdfium_render::prelude::*;

        // Re-bind pdfium for this operation
        let pdfium = Pdfium::new(
            Pdfium::bind_to_system_library()
                .or_else(|_| {
                    Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
                })
                .ok()?,
        );

        let document = pdfium
            .load_pdf_from_byte_slice(&self.pdf_bytes, None)
            .ok()?;
        let page = document.pages().get(page_num as u16).ok()?;

        let width = (page.width().value * scale) as i32;
        let height = (page.height().value * scale) as i32;

        let bitmap = page
            .render_with_config(
                &PdfRenderConfig::new()
                    .set_target_width(width)
                    .set_target_height(height),
            )
            .ok()?;

        let rgba = bitmap.as_rgba_bytes();
        Some((width as u32, height as u32, rgba))
    }

    /// Get raw PDF bytes (for passing to ML pipeline)
    #[inline]
    #[must_use = "returns the raw PDF bytes"]
    pub fn pdf_bytes(&self) -> &[u8] {
        &self.pdf_bytes
    }
}

/// Stub state when pdf-render is not available.
#[cfg(not(feature = "pdf-render"))]
pub struct PdfState {
    /// Path to the PDF file.
    pub path: PathBuf,
}

#[cfg(not(feature = "pdf-render"))]
impl PdfState {
    /// # Errors
    ///
    /// Returns an error when `pdf-render` feature is not enabled.
    #[inline]
    #[must_use = "this function returns a Result that should be checked for errors"]
    pub fn load(path: &str) -> Result<Self, String> {
        Err(format!(
            "PDF rendering not available. Build with --features pdf-render. Path: {path}"
        ))
    }

    /// Returns the number of pages (always 0 for stub).
    #[inline]
    #[must_use = "returns the page count"]
    pub const fn page_count(&self) -> usize {
        0
    }

    /// Returns the document filename.
    #[inline]
    #[must_use = "returns the document filename"]
    pub fn document_path(&self) -> Option<&str> {
        self.path.file_name()?.to_str()
    }

    /// Returns the page dimensions (always None for stub).
    #[inline]
    #[must_use = "returns the page dimensions in points"]
    pub const fn page_size(&self, _page_num: usize) -> Option<(f32, f32)> {
        None
    }

    /// Renders a page (always None for stub).
    #[inline]
    #[must_use = "returns the rendered page as RGBA buffer"]
    pub const fn render_page(&self, _page_num: usize, _scale: f32) -> Option<(u32, u32, Vec<u8>)> {
        None
    }

    /// Returns the raw PDF bytes (empty for stub).
    #[inline]
    #[must_use = "returns the raw PDF bytes"]
    pub const fn pdf_bytes(&self) -> &[u8] {
        &[]
    }

    /// Returns whether ML pipeline is available (always false for stub).
    #[inline]
    #[must_use = "returns whether ML pipeline is available"]
    pub const fn has_ml_pipeline(&self) -> bool {
        false
    }

    /// Returns the ML pipeline initialization error.
    #[inline]
    #[must_use = "returns the ML pipeline initialization error if any"]
    pub const fn ml_init_error(&self) -> Option<&str> {
        Some("PDF rendering not available")
    }

    /// # Errors
    ///
    /// Returns an error when `pdf-render` feature is not enabled.
    #[inline]
    #[must_use = "this function returns a Result that should be checked for errors"]
    pub fn run_ml_pipeline(&mut self, _page_num: usize, _scale: f32) -> Result<(), String> {
        Err("PDF rendering not available".into())
    }

    /// Gets ML snapshot for a page and stage (always None for stub).
    #[inline]
    #[must_use = "returns the snapshot data for a page and stage"]
    pub const fn get_ml_snapshot(
        &self,
        _page_num: usize,
        _stage: crate::DlvizStage,
    ) -> Option<&crate::pipeline_integration::StageSnapshotData> {
        None
    }

    /// Returns whether page has been processed by ML (always false for stub).
    #[inline]
    #[must_use = "returns whether the page has been processed by ML"]
    pub const fn has_ml_page(&self, _page_num: usize) -> bool {
        false
    }

    /// Clears ML results (no-op for stub).
    #[inline]
    pub const fn clear_ml_results(&mut self) {}

    /// Gets text content of an element (always None for stub).
    #[inline]
    #[must_use = "returns the text content of an element"]
    pub const fn get_ml_element_text(&self, _page_num: usize, _element_id: u32) -> Option<&str> {
        None
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(not(feature = "pdf-render"))]
    fn test_stub_state() {
        use super::*;
        let result = PdfState::load("test.pdf");
        assert!(result.is_err());
    }
}
