//! # Main Pipeline - End-to-End PDF Document Parsing
//!
//! This module provides the primary public API for PDF parsing via the [`Pipeline`] struct.
//! It orchestrates the complete document processing workflow from raw PDF to structured output.
//!
//! ## Pipeline Stages
//!
//! The pipeline executes the following stages in order:
//!
//! ### Stage 0: OCR (Optional)
//! - **Stage 0.0-0.3:** Optical Character Recognition for scanned documents
//! - **Implementation:** `ocr` module (RapidOCR: DbNet, AngleNet, CrnnNet)
//! - **Output:** Text cells with bounding boxes and confidence scores
//!
//! ### Stage 1: Layout Detection
//! - **Stage 1.0:** Preprocessing (resize, normalize, pad to 640×640)
//! - **Stage 1.1-1.4:** Layout ML model (ONNX or PyTorch backend)
//! - **Stage 1.5-1.8:** Postprocessing (NMS, confidence filtering, label assignment)
//! - **Implementation:** `layout_postprocessor.rs`
//! - **Output:** Labeled layout clusters (Text, Table, Picture, etc.)
//!
//! ### Stage 3: Assembly (Delegated to pipeline_modular)
//! - **Stage 3.0-3.5:** Document assembly substages
//! - **Implementation:** `pipeline_modular::ModularPipeline` (called from `executor.rs:1035`)
//! - **Stages:**
//!   - 3.0: Cell Assignment (assign OCR cells to clusters)
//!   - 3.1: Empty Removal (filter clusters without content)
//!   - 3.2: Orphan Creation (create clusters for unassigned cells)
//!   - 3.3: BBox Adjustment (expand cluster bounds to cell bounds)
//!   - 3.4: Overlap Resolution (merge overlapping clusters)
//!   - 3.5: Assembly (convert to document elements)
//! - **Output:** Page elements with assigned text cells
//!
//! ### Stage 4: Post-Processing
//! - **Stage 4.0:** TableFormer (optional table structure extraction)
//! - **Stage 4.1:** Reading order determination
//! - **Implementation:** `table_inference.rs`, `reading_order.rs`
//! - **Output:** Ordered page elements with table structures
//!
//! ### Stage 6: Export
//! - **Stage 6.0:** DoclingDocument JSON export
//! - **Implementation:** `docling_export.rs`
//! - **Output:** Structured JSON following DoclingDocument schema
//!
//! ## Module Organization
//!
//! - `executor`: Main `Pipeline` struct and `process_page()` orchestration
//! - `data_structures`: Core types (BoundingBox, Cluster, PageElement, etc.)
//! - `layout_postprocessor`: Layout ML postprocessing (NMS, filtering)
//! - `page_assembly`: Page assembly logic
//! - `reading_order`: Reading order determination and caption/footnote assignment
//! - `table_inference`: TableFormer integration
//! - `docling_export`: DoclingDocument JSON export
//!
//! ## Relationship with pipeline_modular
//!
//! The main pipeline delegates stages 3.0-3.5 to `pipeline_modular::ModularPipeline`
//! to maintain clean separation between ML stages and assembly logic. This design:
//! - Enables independent testing of assembly substages
//! - Provides clear input/output boundaries for each substage
//! - Mirrors the Python docling architecture
//!
//! The executor calls `ModularPipeline::process_stages_4_to_8()` at line 1035 to
//! process stages 3.0-3.5, then continues with stages 4.0, 4.1, and 6.0.
//!
//! ## Usage
//!
//! See the top-level crate documentation for usage examples.

// Internal modules (visible within crate, not to external users)
pub(crate) mod data_structures;
pub(crate) mod docling_export;
pub(crate) mod executor;
pub(crate) mod layout_postprocessor;
pub(crate) mod page_assembly;
pub(crate) mod reading_order;
// table_inference requires PyTorch (tch-rs) for TableFormer model
#[cfg(feature = "pytorch")]
pub(crate) mod table_inference;

// ============================================================================
// Public API Exports
// ============================================================================
//
// Explicit exports for the public API. Only types needed by library users
// are exported here. Internal types (Cluster, LayoutPrediction, etc.) are
// kept private.

// Core pipeline API
pub use executor::{OcrBackend, PageTiming, Pipeline, PipelineConfig, PipelineConfigBuilder};

// Output types (what users get from process_page)
pub use data_structures::{
    AssembledUnit,    // Assembled page
    ContainerElement, // Container element
    FigureElement,    // Picture/figure
    Page,             // Complete page results
    PageElement,      // Enum of element types
    TableElement,     // Table structure
    TextElement,      // Text block
};

// Data structures used in API
pub use data_structures::{
    BoundingBox,       // Bounding box coordinates
    BoundingRectangle, // Rectangle representation
    CoordOrigin,       // Coordinate system
    DocItemLabel,      // Element labels
    SimpleTextCell,    // Text cell for OCR
    TableCell,         // Table cell
};

// Internal types (used within crate, hidden from external docs)
#[doc(hidden)]
pub use data_structures::{
    Cluster,                  // Layout cluster (internal ML structure)
    LayoutPrediction,         // Layout ML prediction result
    PagePredictions,          // All predictions for a page
    Size,                     // Size struct
    TableStructurePrediction, // Table structure prediction
    TextCell,                 // Text cell with full info
};

// Internal processors (used by tests, hidden from external docs)
#[doc(hidden)]
pub use reading_order::{ReadingOrderConfig, ReadingOrderPredictor};

// Export functionality (internal, but used by tests and examples)
#[doc(hidden)]
pub use docling_export::{to_docling_document, to_docling_document_multi};

/// Apply document-level caption and footnote assignments
///
/// This function should be called AFTER all pages have been processed
/// but BEFORE converting to `DoclingDocument`. It:
/// 1. Collects all elements from all pages into a single vector
/// 2. Computes caption assignments (matching captions to tables/figures/code)
/// 3. Applies the assignments back to the page elements
///
/// # Arguments
/// * `pages` - Mutable slice of Pages with assembled elements
///
/// # Example
/// ```ignore
/// // Process all pages first
/// let pages = process_all_pages(...);
///
/// // Apply document-level assignments
/// apply_document_level_assignments(&mut pages);
///
/// // Now convert to DoclingDocument
/// let doc = to_docling_document_multi(&pages, ...);
/// ```
pub fn apply_document_level_assignments(pages: &mut [Page]) {
    use std::collections::HashMap;

    // Collect all elements from all pages into a single sorted vector
    let mut all_elements: Vec<PageElement> = Vec::new();
    for page in pages.iter() {
        if let Some(assembled) = &page.assembled {
            all_elements.extend(assembled.elements.iter().cloned());
        }
    }

    if all_elements.is_empty() {
        return;
    }

    // Create reading order processor and compute caption assignments
    let reading_order = ReadingOrderPredictor::new(ReadingOrderConfig::default());
    let caption_assignments = reading_order.predict_to_captions(&all_elements);

    if caption_assignments.is_empty() {
        return;
    }

    log::debug!(
        "Applying {} document-level caption assignments",
        caption_assignments.len()
    );

    // Build CID-to-caption map for efficient lookup
    let cid_to_captions: HashMap<usize, Vec<usize>> = caption_assignments;

    // Apply assignments to elements in each page
    for page in pages.iter_mut() {
        if let Some(ref mut assembled) = page.assembled {
            for element in &mut assembled.elements {
                let cid = element.cluster().id;
                if let Some(caption_cids) = cid_to_captions.get(&cid) {
                    match element {
                        PageElement::Text(ref mut text_elem) => {
                            if text_elem.label == DocItemLabel::Code {
                                text_elem.captions.clone_from(caption_cids);
                            }
                        }
                        PageElement::Table(ref mut table_elem) => {
                            table_elem.captions.clone_from(caption_cids);
                        }
                        PageElement::Figure(ref mut fig_elem) => {
                            fig_elem.captions.clone_from(caption_cids);
                        }
                        PageElement::Container(_) => {}
                    }
                }
            }
        }
    }
}

// ============================================================================
// Device Enum (for non-pytorch builds)
// ============================================================================
//
// When the pytorch feature is disabled, we provide a stub Device enum
// that mirrors tch::Device interface for API compatibility.

/// Device selection for ML model inference.
///
/// This is a stub for non-pytorch builds. When the `pytorch` feature is enabled,
/// the real `tch::Device` is re-exported instead.
#[cfg(not(feature = "pytorch"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Device {
    /// CPU inference
    #[default]
    Cpu,
    /// CUDA GPU inference (device index)
    Cuda(usize),
    /// Apple Metal Performance Shaders (MPS)
    Mps,
}

#[cfg(not(feature = "pytorch"))]
impl std::fmt::Display for Device {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cpu => write!(f, "cpu"),
            Self::Cuda(idx) => write!(f, "cuda:{idx}"),
            Self::Mps => write!(f, "mps"),
        }
    }
}

#[cfg(not(feature = "pytorch"))]
impl std::str::FromStr for Device {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lower = s.to_lowercase();
        match lower.as_str() {
            "cpu" => Ok(Self::Cpu),
            "mps" | "metal" => Ok(Self::Mps),
            s if s.starts_with("cuda") => {
                // Handle "cuda", "cuda:0", "cuda:1", etc.
                if s == "cuda" {
                    Ok(Self::Cuda(0))
                } else if let Some(idx_str) = s.strip_prefix("cuda:") {
                    idx_str.parse::<usize>().map(Self::Cuda).map_err(|_| {
                        format!("Invalid CUDA device index '{idx_str}'. Expected: cuda:N")
                    })
                } else {
                    Err(format!(
                        "Invalid device '{s}'. Expected: cpu, cuda, cuda:N, mps"
                    ))
                }
            }
            s if s.starts_with("gpu") => {
                // Handle "gpu", "gpu:0", etc. as alias for cuda
                if s == "gpu" {
                    Ok(Self::Cuda(0))
                } else if let Some(idx_str) = s.strip_prefix("gpu:") {
                    idx_str.parse::<usize>().map(Self::Cuda).map_err(|_| {
                        format!("Invalid GPU device index '{idx_str}'. Expected: gpu:N")
                    })
                } else {
                    Err(format!(
                        "Invalid device '{s}'. Expected: cpu, cuda, cuda:N, mps"
                    ))
                }
            }
            _ => Err(format!(
                "Unknown device '{s}'. Expected: cpu, cuda, cuda:N, mps"
            )),
        }
    }
}

#[cfg(test)]
#[cfg(not(feature = "pytorch"))]
mod tests {
    use super::*;

    #[test]
    fn test_device_display() {
        assert_eq!(Device::Cpu.to_string(), "cpu");
        assert_eq!(Device::Cuda(0).to_string(), "cuda:0");
        assert_eq!(Device::Cuda(1).to_string(), "cuda:1");
        assert_eq!(Device::Mps.to_string(), "mps");
    }

    #[test]
    fn test_device_from_str() {
        // Canonical forms
        assert_eq!("cpu".parse::<Device>().unwrap(), Device::Cpu);
        assert_eq!("cuda".parse::<Device>().unwrap(), Device::Cuda(0));
        assert_eq!("cuda:0".parse::<Device>().unwrap(), Device::Cuda(0));
        assert_eq!("cuda:1".parse::<Device>().unwrap(), Device::Cuda(1));
        assert_eq!("mps".parse::<Device>().unwrap(), Device::Mps);

        // Case insensitive
        assert_eq!("CPU".parse::<Device>().unwrap(), Device::Cpu);
        assert_eq!("CUDA:2".parse::<Device>().unwrap(), Device::Cuda(2));
        assert_eq!("MPS".parse::<Device>().unwrap(), Device::Mps);

        // Aliases
        assert_eq!("gpu".parse::<Device>().unwrap(), Device::Cuda(0));
        assert_eq!("gpu:0".parse::<Device>().unwrap(), Device::Cuda(0));
        assert_eq!("metal".parse::<Device>().unwrap(), Device::Mps);

        // Invalid
        assert!("invalid".parse::<Device>().is_err());
        assert!("cuda:abc".parse::<Device>().is_err());
        assert!("".parse::<Device>().is_err());
    }

    #[test]
    fn test_device_roundtrip() {
        for device in [Device::Cpu, Device::Cuda(0), Device::Cuda(1), Device::Mps] {
            let s = device.to_string();
            let parsed: Device = s.parse().unwrap();
            assert_eq!(parsed, device);
        }
    }
}
