//! # Docling Rust - Production-Grade PDF Parsing Library
//!
//! A high-performance, ML-based PDF parsing library ported from Python Docling to native Rust.
//! Provides document structure extraction including layout detection, table parsing, and reading order
//! determination with 100% output validation against the Python baseline.
//!
//! ## Features
//!
//! - **Layout Detection**: Document structure recognition (text, tables, figures, headers, etc.)
//! - **Table Parsing**: Accurate table structure extraction with row/column detection
//! - **Reading Order**: Intelligent reading order determination for multi-column layouts
//! - **Production Ready**: Type-safe error handling, comprehensive documentation, stable API
//! - **Performance**: Zero-copy operations, efficient memory usage, reusable pipeline instances
//!
//! ## Quick Start
//!
//! ```no_run
//! use docling_pdf_ml::{Pipeline, Result};
//! use ndarray::Array3;
//!
//! # fn main() -> Result<()> {
//! // Create pipeline with default configuration (CPU, OCR enabled, table parsing enabled)
//! let mut pipeline = Pipeline::with_defaults()?;
//!
//! // Or use convenience constructors:
//! // let mut pipeline = Pipeline::cpu()?;      // CPU-only
//! // let mut pipeline = Pipeline::gpu(0)?;     // First GPU
//!
//! // Process a page (page_image should be HWC format, u8, range [0, 255])
//! # let page_image = Array3::<u8>::zeros((792, 612, 3));
//! let page = pipeline.process_page(
//!     0,              // page number
//!     &page_image,    // page image as ndarray
//!     612.0,          // width in points
//!     792.0,          // height in points
//!     None,           // optional textline cells
//! )?;
//!
//! // Access results
//! if let Some(assembled) = page.assembled {
//!     for element in assembled.elements {
//!         match element {
//!             docling_pdf_ml::PageElement::Text(text) => {
//!                 println!("Text: {}", text.text);
//!             }
//!             docling_pdf_ml::PageElement::Table(table) => {
//!                 println!("Table: {} rows x {} cols", table.num_rows, table.num_cols);
//!             }
//!             docling_pdf_ml::PageElement::Figure(figure) => {
//!                 println!("Figure at ({:.1}, {:.1})", figure.cluster.bbox.l, figure.cluster.bbox.t);
//!             }
//!             _ => {}
//!         }
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Configuration
//!
//! Use [`PipelineConfigBuilder`] for custom configuration:
//!
//! ```no_run
//! use docling_pdf_ml::{Pipeline, PipelineConfigBuilder, Device};
//!
//! # fn main() -> docling_pdf_ml::Result<()> {
//! // Option 1: Use configuration presets
//! let config = PipelineConfigBuilder::minimal()  // Fast, minimal features
//!     .device(Device::Cpu)
//!     .build()?;
//!
//! // Option 2: Customize step-by-step
//! let config = PipelineConfigBuilder::new()
//!     .device(Device::Cuda(0))          // Use GPU
//!     .table_structure_enabled(true)    // Enable table parsing
//!     .ocr_enabled(false)               // Disable OCR
//!     .build()?;
//!
//! // Option 3: Start from preset and customize
//! let config = PipelineConfigBuilder::fast()     // Start with fast preset
//!     .device(Device::Cuda(0))                   // But use GPU
//!     .build()?;
//!
//! let mut pipeline = Pipeline::new(config)?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Configuration Presets
//!
//! - **`minimal()`**: Maximum speed, minimum features (no OCR, no table parsing)
//! - **`fast()`**: Balanced speed and features (OCR enabled, no table parsing)
//! - **`complete()`**: All features enabled (OCR, table parsing, code/formula enrichment)
//!
//! ## Error Handling
//!
//! All public APIs return [`Result<T>`] with specific error types:
//!
//! ```no_run
//! use docling_pdf_ml::{Pipeline, DoclingError};
//!
//! # fn example(mut pipeline: Pipeline, page_image: ndarray::Array3<u8>) {
//! match pipeline.process_page(0, &page_image, 612.0, 792.0, None) {
//!     Ok(page) => log::debug!("Success"),
//!     Err(DoclingError::InferenceError { model_name, .. }) => {
//!         log::warn!("Inference failed for {}", model_name);
//!     }
//!     Err(e) => log::warn!("Error: {}", e),
//! }
//! # }
//! ```
//!
//! ## Performance Notes
//!
//! - **Model Loading**: Pipeline initialization is expensive (1-3 seconds). Reuse pipeline instances
//!   for multiple pages to amortize this cost.
//! - **Thread Safety**: Pipeline is NOT thread-safe. Create one pipeline per thread for concurrent
//!   processing.
//! - **Memory**: Page images should be reused when possible. The pipeline uses zero-copy operations
//!   internally where feasible.
//!
//! ## Cascade Layout Routing
//!
//! For better performance on documents with simple pages, use [`CascadeMode`] to route pages
//! through different detection backends based on complexity:
//!
//! ```no_run
//! use docling_pdf_ml::{PipelineConfigBuilder, CascadeMode};
//!
//! # fn main() -> docling_pdf_ml::Result<()> {
//! // Auto mode: use heuristics for simple pages, ML for complex (5-10x speedup)
//! let config = PipelineConfigBuilder::new()
//!     .cascade_mode(CascadeMode::Auto)
//!     .build()?;
//!
//! // On macOS with Apple Silicon, use CoreML for best performance (7x faster)
//! #[cfg(feature = "coreml")]
//! let config = PipelineConfigBuilder::new()
//!     .cascade_mode(CascadeMode::AutoWithCoreML)
//!     .build()?;
//! # Ok(())
//! # }
//! ```
//!
//! **Cascade Modes:**
//! - **`AlwaysML`** (default): Always use RT-DETR ML model (~120ms per page)
//! - **`Auto`**: Use heuristics for simple pages, ML for complex (5-10x speedup on simple docs)
//! - **`AutoWithCoreML`** (macOS): Use `CoreML` for standard pages (7x faster than ONNX CPU)
//! - **`Conservative`**: Use heuristics only for definitely simple pages
//!
//! ### Monitoring Cascade Performance
//!
//! Use [`CascadeStats`] to monitor how pages are being routed and measure performance gains:
//!
//! ```no_run
//! use docling_pdf_ml::{Pipeline, PipelineConfigBuilder, CascadeMode, CascadeStats};
//!
//! # fn main() -> docling_pdf_ml::Result<()> {
//! let config = PipelineConfigBuilder::new()
//!     .cascade_mode(CascadeMode::Auto)
//!     .build()?;
//! let mut pipeline = Pipeline::new(config)?;
//!
//! // Process pages...
//! # let page_image = ndarray::Array3::<u8>::zeros((792, 612, 3));
//! # pipeline.process_page(0, &page_image, 612.0, 792.0, None)?;
//!
//! // Get cascade statistics
//! let stats: &CascadeStats = pipeline.cascade_stats();
//! println!("Heuristic path: {} pages", stats.heuristic_count);
//! println!("ML path: {} pages", stats.ml_count);
//! println!("CoreML path: {} pages", stats.coreml_count);
//! println!("Speedup factor: {:.1}x", stats.speedup_factor());
//! println!("Time saved: {:.0}ms", stats.estimated_time_saved_ms());
//! println!("Fast path usage: {:.1}%", stats.fast_path_percentage());
//!
//! // Reset stats if needed (e.g., for per-document tracking)
//! pipeline.reset_cascade_stats();
//! # Ok(())
//! # }
//! ```
//!
//! **Key Statistics:**
//! - **`speedup_factor()`**: Overall speedup vs. using ML for all pages (>1.0 = faster)
//! - **`estimated_time_saved_ms()`**: Total processing time saved
//! - **`fast_path_percentage()`**: Percentage of pages using fast paths
//!
//! ## ML Models
//!
//! The library uses three core ML models:
//! 1. **`LayoutPredictor`** (ONNX): Detects document structure
//! 2. **`TableFormer`** (PyTorch): Parses table structure
//! 3. **`RapidOCR`** (ONNX): Text extraction (optional)
//!
//! Models are loaded from local paths or cache directory at initialization.

// Global allocator: jemalloc for better memory allocation patterns (N=153)
// NOTE (N=2315): Tested with/without jemalloc - crash is NOT caused by jemalloc
// The "foreign exceptions" crash happens at libtorch library load time, regardless of allocator
// Expected 2-5% improvement from reduced fragmentation and better cache behavior
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

// Error types (public API)
pub mod error;

// Internal modules (not part of public API)
#[doc(hidden)]
pub mod baseline; // Internal data structures (BBox, LayoutCluster) used for ML model conversions
#[doc(hidden)]
pub mod convert; // Convert pipeline output to docling-core format
#[doc(hidden)]
pub mod convert_to_core;
#[doc(hidden)]
pub mod docling_document; // DoclingDocument JSON schema (internal export format)
#[doc(hidden)]
pub mod models; // ML model internals (layout, table, code/formula)
                // OCR module: Pure Rust parts always available, OpenCV postprocessing requires feature
#[doc(hidden)]
pub mod ocr; // RapidOCR implementation (internal)
#[doc(hidden)]
pub mod pipeline_modular; // Modular assembly stages (internal, used by pipeline)
#[doc(hidden)]
pub mod preprocessing; // Image preprocessing internals (internal) // Convert pdf-ml DoclingDocument to core DoclingDocument

/// Main pipeline implementation providing end-to-end PDF parsing (Stages 0.0 → 6.0).
///
/// This is the **primary public API** for document processing. The pipeline orchestrates:
/// - Stage 0.0-0.3: OCR (optional)
/// - Stage 1.0-1.8: Layout detection ML + postprocessing
/// - Stage 3.0-3.5: Assembly substages
/// - Stage 4.0: TableFormer (optional table structure extraction)
/// - Stage 4.1: Reading order determination
/// - Stage 6.0: DoclingDocument export
pub mod pipeline;

// ============================================================================
// Public API Exports
// ============================================================================
//
// This section defines the minimal public API surface for the library.
// Everything else is internal implementation details.

pub use error::{DoclingError, Result};

// Core pipeline API
pub use pipeline::{
    Pipeline,              // Main pipeline struct
    PipelineConfig,        // Pipeline configuration
    PipelineConfigBuilder, // Configuration builder
};

// Output types (returned by Pipeline::process_page)
pub use pipeline::{
    AssembledUnit,    // Assembled page elements
    ContainerElement, // Container (section, list, etc.)
    FigureElement,    // Figure/Picture with bbox
    Page,             // Complete page with all processing results
    PageElement,      // Enum: Text | Table | Figure | Container
    TableElement,     // Table with structure (rows, cols, cells)
    TextElement,      // Text block with content and bbox
};

// Data structures used in outputs
pub use pipeline::{
    BoundingBox,       // Bounding box with coordinates
    BoundingRectangle, // Rectangle representation
    CoordOrigin,       // Coordinate system origin
    DocItemLabel,      // Element type labels
    SimpleTextCell,    // Text cell (input for OCR)
    TableCell,         // Table cell with position
};

// Export functionality (used by docling-backend)
pub use pipeline::apply_document_level_assignments;
pub use pipeline::to_docling_document_multi;

// Configuration types
pub use models::layout_predictor::InferenceBackend;
pub use pipeline::OcrBackend; // OCR backend selection (Auto, AppleVision, RapidOcr)
#[cfg(feature = "pytorch")]
pub use tch::Device; // Device selection (CPU, CUDA, MPS) // Backend selection (ONNX, PyTorch)

// Cascade layout routing (performance optimization)
pub use models::cascade_layout::{CascadeMode, CascadeStats};

// Re-export Device enum for non-pytorch builds (mirrors tch::Device)
#[cfg(not(feature = "pytorch"))]
pub use pipeline::Device;

// Device auto-detection (always use the best available device)
pub use pipeline::executor::{detect_best_batch_size, detect_best_device};
