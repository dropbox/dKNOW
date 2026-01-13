//! # docling-notebook
//!
//! Jupyter Notebook (.ipynb) parsing library for docling-rs.
//!
//! This crate provides functionality to parse Jupyter Notebook files (nbformat 4.x)
//! and extract their contents including:
//! - Markdown cells (documentation)
//! - Code cells (with execution counts)
//! - Cell outputs (stream, display data, execute results, errors)
//! - Notebook metadata (kernel, language, authors)
//!
//! ## Example
//!
//! ```no_run
//! use docling_notebook::parse_notebook;
//!
//! let notebook = parse_notebook("example.ipynb")?;
//! for cell in &notebook.cells {
//!     println!("Cell type: {:?}", cell.cell_type);
//!     println!("Source: {}", cell.source);
//! }
//! # Ok::<(), docling_notebook::NotebookError>(())
//! ```

/// Error types for notebook parsing
pub mod error;
/// Jupyter notebook (ipynb) parser
pub mod ipynb;

pub use error::{NotebookError, Result};
pub use ipynb::{
    parse_notebook, parse_notebook_from_str, CellOutputData, CellType, NotebookCell,
    NotebookMetadata, OutputType, ParsedNotebook,
};
