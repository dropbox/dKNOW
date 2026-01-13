//! Integration with docling-pdf-ml pipeline for real ML inference
//!
//! This module connects the FFI bridge to the actual ML pipeline,
//! converting between internal pipeline types and FFI-safe types.

use crate::DlvizStage;
use std::collections::HashMap;

/// Stage snapshot with owned data.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StageSnapshotData {
    /// Processing stage that produced this snapshot.
    pub stage: DlvizStage,
    /// Detected elements at this stage.
    pub elements: Vec<crate::DlvizElement>,
    /// Text cells extracted from PDF.
    pub cells: Vec<crate::DlvizTextCell>,
    /// Processing time for this stage in milliseconds.
    pub processing_time_ms: f64,
}

/// Processed page with stage snapshots.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProcessedPage {
    /// Page number (0-indexed).
    pub page_no: usize,
    /// Snapshots from each processing stage.
    pub snapshots: Vec<StageSnapshotData>,
    /// Element ID → text content mapping.
    pub element_texts: HashMap<u32, String>,
}

/// Text cell extracted from PDF
///
/// Represents a text segment with its bounding box and content.
/// Used to pass native PDF text to the ML pipeline for element population.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TextCell {
    /// Cell index
    pub index: usize,
    /// Text content
    pub text: String,
    /// Left x-coordinate (in points)
    pub l: f32,
    /// Top y-coordinate (in points, origin at top-left)
    pub t: f32,
    /// Right x-coordinate (in points)
    pub r: f32,
    /// Bottom y-coordinate (in points, origin at top-left)
    pub b: f32,
    /// Confidence score (1.0 for native PDF text)
    pub confidence: f32,
    /// Whether the text came from OCR
    pub from_ocr: bool,
}

#[cfg(feature = "pdf-ml")]
mod ml_integration {
    use super::*;
    use crate::{DlvizBBox, DlvizElement, DlvizLabel};
    use docling_pdf_ml::{
        BoundingBox, DocItemLabel, Page, PageElement, Pipeline, PipelineConfigBuilder,
    };
    use ndarray::Array3;
    use std::time::Instant;

    /// Convert DocItemLabel to DlvizLabel
    #[inline]
    pub fn convert_label(label: &DocItemLabel) -> DlvizLabel {
        match label {
            DocItemLabel::Caption => DlvizLabel::Caption,
            DocItemLabel::Footnote => DlvizLabel::Footnote,
            DocItemLabel::Formula => DlvizLabel::Formula,
            DocItemLabel::ListItem => DlvizLabel::ListItem,
            DocItemLabel::PageFooter => DlvizLabel::PageFooter,
            DocItemLabel::PageHeader => DlvizLabel::PageHeader,
            DocItemLabel::Picture | DocItemLabel::Figure => DlvizLabel::Picture,
            DocItemLabel::SectionHeader => DlvizLabel::SectionHeader,
            DocItemLabel::Table => DlvizLabel::Table,
            DocItemLabel::Text => DlvizLabel::Text,
            DocItemLabel::Title => DlvizLabel::Title,
            DocItemLabel::Code => DlvizLabel::Code,
            DocItemLabel::CheckboxSelected => DlvizLabel::CheckboxSelected,
            DocItemLabel::CheckboxUnselected => DlvizLabel::CheckboxUnselected,
            DocItemLabel::DocumentIndex => DlvizLabel::DocumentIndex,
            DocItemLabel::Form => DlvizLabel::Form,
            DocItemLabel::KeyValueRegion => DlvizLabel::KeyValueRegion,
        }
    }

    /// Convert BoundingBox to DlvizBBox
    #[inline]
    pub fn convert_bbox(bbox: &BoundingBox) -> DlvizBBox {
        DlvizBBox {
            x: bbox.l,
            y: bbox.t,
            width: bbox.r - bbox.l,
            height: bbox.b - bbox.t,
        }
    }

    /// Convert PageElement to DlvizElement with reading order
    #[inline]
    pub fn convert_page_element(element: &PageElement, reading_order: usize) -> DlvizElement {
        match element {
            PageElement::Text(text) => DlvizElement {
                id: text.id as u32,
                bbox: convert_bbox(&text.cluster.bbox),
                label: convert_label(&text.label),
                confidence: text.cluster.confidence,
                reading_order: reading_order as i32,
            },
            PageElement::Table(table) => DlvizElement {
                id: table.id as u32,
                bbox: convert_bbox(&table.cluster.bbox),
                label: convert_label(&table.label),
                confidence: table.cluster.confidence,
                reading_order: reading_order as i32,
            },
            PageElement::Figure(figure) => DlvizElement {
                id: figure.id as u32,
                bbox: convert_bbox(&figure.cluster.bbox),
                label: convert_label(&figure.label),
                confidence: figure.cluster.confidence,
                reading_order: reading_order as i32,
            },
            PageElement::Container(container) => DlvizElement {
                id: container.id as u32,
                bbox: convert_bbox(&container.cluster.bbox),
                label: convert_label(&container.label),
                confidence: container.cluster.confidence,
                reading_order: reading_order as i32,
            },
        }
    }

    /// Extract text content from a PageElement
    #[inline]
    #[must_use = "returns the text content of an element if present"]
    pub fn extract_element_text(element: &PageElement) -> Option<String> {
        match element {
            PageElement::Text(text) => Some(text.text.clone()),
            PageElement::Table(table) => table.text.clone(),
            PageElement::Figure(figure) => figure.text.clone(),
            PageElement::Container(_) => None, // Containers don't have text
        }
    }

    /// Get the element ID from a PageElement
    #[inline]
    #[must_use = "returns the element ID"]
    pub fn get_element_id(element: &PageElement) -> u32 {
        match element {
            PageElement::Text(text) => text.id as u32,
            PageElement::Table(table) => table.id as u32,
            PageElement::Figure(figure) => figure.id as u32,
            PageElement::Container(container) => container.id as u32,
        }
    }

    /// Extract stage snapshots and element texts from a processed page
    ///
    /// Note: We currently only have access to the final assembled output through the public API.
    /// Intermediate stages (like raw layout detection) would require access to internal types.
    /// For visualization, we create a "layout detection" stage from the assembled elements
    /// (without reading order) and a "reading order" stage (with reading order).
    ///
    /// Returns (snapshots, element_texts) where element_texts maps element ID to text content.
    pub fn extract_snapshots(
        page: &Page,
        layout_time_ms: f64,
    ) -> (Vec<StageSnapshotData>, HashMap<u32, String>) {
        let mut snapshots = Vec::new();
        let mut element_texts = HashMap::new();

        // Stage 0: Raw PDF (no elements yet)
        snapshots.push(StageSnapshotData {
            stage: DlvizStage::RawPdf,
            elements: vec![],
            cells: vec![],
            processing_time_ms: 0.0,
        });

        // Extract from assembled output
        if let Some(ref assembled) = page.assembled {
            // Extract text from all elements
            for elem in &assembled.elements {
                let id = get_element_id(elem);
                if let Some(text) = extract_element_text(elem) {
                    element_texts.insert(id, text);
                }
            }

            // Stage 3: Layout Detection - elements without reading order
            // (this represents what was detected before reading order assignment)
            let layout_elements: Vec<DlvizElement> = assembled
                .elements
                .iter()
                .map(|elem| {
                    // Convert without reading order
                    let mut dlviz_elem = convert_page_element(elem, 0);
                    dlviz_elem.reading_order = -1; // Not assigned yet at this stage
                    dlviz_elem
                })
                .collect();

            snapshots.push(StageSnapshotData {
                stage: DlvizStage::LayoutDetection,
                elements: layout_elements,
                cells: vec![], // Text cells not available through public API
                processing_time_ms: layout_time_ms,
            });

            // Stage 10: Reading Order - final assembled output with reading order
            let final_elements: Vec<DlvizElement> = assembled
                .elements
                .iter()
                .enumerate()
                .map(|(idx, elem)| convert_page_element(elem, idx))
                .collect();

            snapshots.push(StageSnapshotData {
                stage: DlvizStage::ReadingOrder,
                elements: final_elements,
                cells: vec![],
                processing_time_ms: 0.0,
            });
        }

        (snapshots, element_texts)
    }

    /// Pipeline runner that processes PDFs and captures stage snapshots
    pub struct PipelineRunner {
        pipeline: Option<Pipeline>,
        processed_pages: Vec<ProcessedPage>,
        init_error: Option<String>,
    }

    impl Default for PipelineRunner {
        #[inline]
        fn default() -> Self {
            Self::new()
        }
    }

    impl PipelineRunner {
        /// Create a new pipeline runner
        ///
        /// Note: Pipeline creation may fail if models are not available.
        /// The error is captured and reported on first use.
        #[must_use = "returns a new pipeline runner instance"]
        pub fn new() -> Self {
            // Try to create the pipeline
            let (pipeline, init_error) = match PipelineConfigBuilder::fast()
                .ocr_enabled(false) // Disable OCR for viz (faster)
                .table_structure_enabled(false) // Disable TableFormer for viz
                .prefer_quantized(false) // N=4005: INT8 breaks classification - use FP32
                .build()
            {
                Ok(config) => match Pipeline::new(config) {
                    Ok(p) => (Some(p), None),
                    Err(e) => (None, Some(format!("Pipeline init failed: {e}"))),
                },
                Err(e) => (None, Some(format!("Config build failed: {e}"))),
            };

            Self {
                pipeline,
                processed_pages: Vec::new(),
                init_error,
            }
        }

        /// Check if pipeline is available
        #[inline]
        #[must_use = "returns whether the ML pipeline is available"]
        pub fn is_available(&self) -> bool {
            self.pipeline.is_some()
        }

        /// Get initialization error (if any)
        #[inline]
        #[must_use = "returns the initialization error message if pipeline failed to start"]
        pub fn init_error(&self) -> Option<&str> {
            self.init_error.as_deref()
        }

        /// Process a page image and capture stage snapshots
        ///
        /// # Arguments
        /// - `page_no`: Page number (0-indexed)
        /// - `image`: Page image as RGBA bytes (width x height x 4)
        /// - `width`: Image width in pixels
        /// - `height`: Image height in pixels
        /// - `page_width_pts`: Page width in points
        /// - `page_height_pts`: Page height in points
        /// - `text_cells`: Optional text cells extracted from PDF
        #[allow(
            clippy::too_many_arguments,
            reason = "page processing requires image data, dimensions, and PDF coordinates"
        )]
        pub fn process_page(
            &mut self,
            page_no: usize,
            image: &[u8],
            width: u32,
            height: u32,
            page_width_pts: f32,
            page_height_pts: f32,
            text_cells: Option<&[super::TextCell]>,
        ) -> Result<(), String> {
            let pipeline = self.pipeline.as_mut().ok_or_else(|| {
                self.init_error
                    .clone()
                    .unwrap_or_else(|| "Pipeline not initialized".into())
            })?;

            // Convert RGBA to RGB array for pipeline (HWC format)
            let mut rgb_data = Vec::with_capacity((width * height * 3) as usize);
            for pixel in image.chunks(4) {
                rgb_data.push(pixel[0]); // R
                rgb_data.push(pixel[1]); // G
                rgb_data.push(pixel[2]); // B
            }

            // Create ndarray from RGB data
            let page_image = Array3::from_shape_vec((height as usize, width as usize, 3), rgb_data)
                .map_err(|e| format!("Failed to create image array: {e}"))?;

            // Convert text cells to SimpleTextCell format for pipeline
            let simple_cells: Option<Vec<docling_pdf_ml::SimpleTextCell>> =
                text_cells.map(|cells| {
                    cells
                        .iter()
                        .map(|c| docling_pdf_ml::SimpleTextCell {
                            index: c.index,
                            text: c.text.clone(),
                            rect: docling_pdf_ml::pipeline::BoundingBox {
                                l: c.l,
                                t: c.t,
                                r: c.r,
                                b: c.b,
                                coord_origin: docling_pdf_ml::pipeline::CoordOrigin::TopLeft,
                            },
                            confidence: c.confidence,
                            from_ocr: c.from_ocr,
                        })
                        .collect()
                });

            log::debug!(
                "Processing page {} with {} text cells",
                page_no,
                simple_cells.as_ref().map_or(0, Vec::len)
            );

            // Process the page
            let start = Instant::now();
            let page = pipeline
                .process_page(
                    page_no,
                    &page_image,
                    page_width_pts,
                    page_height_pts,
                    simple_cells, // Pass text cells to pipeline
                )
                .map_err(|e| format!("Pipeline error: {e}"))?;
            let layout_time_ms = start.elapsed().as_secs_f64() * 1000.0;

            // Extract snapshots and element texts
            let (snapshots, element_texts) = extract_snapshots(&page, layout_time_ms);

            // Store or replace processed page
            let processed = ProcessedPage {
                page_no,
                snapshots,
                element_texts,
            };

            // Find and replace existing page or append
            if let Some(existing) = self
                .processed_pages
                .iter_mut()
                .find(|p| p.page_no == page_no)
            {
                *existing = processed;
            } else {
                self.processed_pages.push(processed);
            }

            Ok(())
        }

        /// Get snapshot for a specific page and stage
        #[inline]
        #[must_use = "returns the snapshot data for a specific page and stage"]
        pub fn get_snapshot(
            &self,
            page_no: usize,
            stage: DlvizStage,
        ) -> Option<&StageSnapshotData> {
            self.processed_pages
                .iter()
                .find(|p| p.page_no == page_no)?
                .snapshots
                .iter()
                .find(|s| s.stage == stage)
        }

        /// Get text content for a specific element
        ///
        /// # Arguments
        /// - `page_no`: Page number (0-indexed)
        /// - `element_id`: Element ID from snapshot
        ///
        /// # Returns
        /// The text content if available, or None if not found
        #[inline]
        #[must_use = "returns the text content of a specific element on a page"]
        pub fn get_element_text(&self, page_no: usize, element_id: u32) -> Option<&str> {
            self.processed_pages
                .iter()
                .find(|p| p.page_no == page_no)?
                .element_texts
                .get(&element_id)
                .map(|s| s.as_str())
        }

        /// Check if a page has been processed
        #[inline]
        #[must_use = "returns whether a page has been processed"]
        pub fn has_page(&self, page_no: usize) -> bool {
            self.processed_pages.iter().any(|p| p.page_no == page_no)
        }

        /// Clear all processed pages
        #[inline]
        pub fn clear(&mut self) {
            self.processed_pages.clear();
        }
    }
}

#[cfg(feature = "pdf-ml")]
pub use ml_integration::*;

// Stub implementation when pdf-ml is not available
#[cfg(not(feature = "pdf-ml"))]
/// Stub pipeline runner when `pdf-ml` feature is disabled.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct PipelineRunner;

#[cfg(not(feature = "pdf-ml"))]
impl PipelineRunner {
    /// Creates a new stub pipeline runner.
    #[inline]
    #[must_use = "returns a new pipeline runner instance"]
    pub const fn new() -> Self {
        Self
    }

    /// Returns whether the ML pipeline is available (always false for stub).
    #[inline]
    #[must_use = "returns whether the ML pipeline is available"]
    pub const fn is_available(&self) -> bool {
        false
    }

    /// Returns the initialization error message.
    #[inline]
    #[must_use = "returns the initialization error message if pipeline failed to start"]
    pub const fn init_error(&self) -> Option<&str> {
        Some("PDF ML pipeline not available. Build with --features pdf-ml")
    }

    /// # Errors
    ///
    /// Returns an error when `pdf-ml` feature is not enabled.
    #[inline]
    #[allow(
        clippy::too_many_arguments,
        reason = "stub matches pdf-ml variant signature"
    )]
    pub fn process_page(
        &mut self,
        _page_no: usize,
        _image: &[u8],
        _width: u32,
        _height: u32,
        _page_width_pts: f32,
        _page_height_pts: f32,
        _text_cells: Option<&[TextCell]>,
    ) -> Result<(), String> {
        Err("PDF ML pipeline not available. Build with --features pdf-ml".into())
    }

    /// Gets snapshot data for a specific page and stage (always None for stub).
    #[inline]
    #[must_use = "returns the snapshot data for a specific page and stage"]
    pub const fn get_snapshot(
        &self,
        _page_no: usize,
        _stage: DlvizStage,
    ) -> Option<&StageSnapshotData> {
        None
    }

    /// Gets the text content of a specific element (always None for stub).
    #[inline]
    #[must_use = "returns the text content of a specific element on a page"]
    pub const fn get_element_text(&self, _page_no: usize, _element_id: u32) -> Option<&str> {
        None
    }

    /// Returns whether a page has been processed (always false for stub).
    #[inline]
    #[must_use = "returns whether a page has been processed"]
    pub const fn has_page(&self, _page_no: usize) -> bool {
        false
    }

    /// Clears all processed pages (no-op for stub).
    #[inline]
    pub const fn clear(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_runner_stub() {
        let _runner = PipelineRunner::new();
        #[cfg(not(feature = "pdf-ml"))]
        {
            assert!(!_runner.is_available());
            assert!(_runner.init_error().is_some());
        }
    }
}
