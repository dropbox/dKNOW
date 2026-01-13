//! # docling-latex
//!
//! LaTeX document parser for docling-rs.
//!
//! This crate provides parsing support for LaTeX (`.tex`) documents using a pure
//! Rust regex-based approach. No external dependencies (Pandoc, TeX engine) are
//! required.
//!
//! ## Supported Format
//!
//! | Format | Extension | Description |
//! |--------|-----------|-------------|
//! | LaTeX | `.tex` | TeX/LaTeX source documents |
//!
//! ## Supported Features
//!
//! ### Document Structure
//!
//! | Command | Description |
//! |---------|-------------|
//! | `\documentclass{}` | Document class detection |
//! | `\section{}` | Section headings (level 1) |
//! | `\subsection{}` | Subsection headings (level 2) |
//! | `\subsubsection{}` | Subsubsection headings (level 3) |
//! | `\paragraph{}` | Paragraph headings (level 4) |
//! | `\chapter{}` | Chapter headings (book class) |
//!
//! ### Metadata
//!
//! | Command | Description |
//! |---------|-------------|
//! | `\title{}` | Document title |
//! | `\author{}` | Author name(s) |
//! | `\date{}` | Publication date |
//! | `\abstract{}` | Abstract content |
//!
//! ### Text Formatting
//!
//! | Command | Output |
//! |---------|--------|
//! | `\textbf{}` | **Bold text** |
//! | `\textit{}` | *Italic text* |
//! | `\emph{}` | *Emphasized text* |
//! | `\texttt{}` | `Monospace text` |
//! | `\underline{}` | Underlined text |
//!
//! ### Environments
//!
//! | Environment | Description |
//! |-------------|-------------|
//! | `itemize` | Bulleted lists |
//! | `enumerate` | Numbered lists |
//! | `tabular` | Tables |
//! | `tabular*` | Tables with width |
//! | `figure` | Figure environments |
//! | `equation` | Math equations |
//! | `verbatim` | Preformatted text |
//!
//! ### Special Support
//!
//! - **Resume templates**: `\resumeSubheading`, `\resumeItem`
//! - **Bibliography**: `\cite{}`, `\bibliography{}`
//! - **Cross-references**: `\ref{}`, `\label{}`
//!
//! ## Quick Start
//!
//! ### Parse a LaTeX Document
//!
//! ```rust,no_run
//! use docling_latex::LatexBackend;
//! use std::path::Path;
//!
//! let mut backend = LatexBackend::new()?;
//! let doc = backend.parse(Path::new("paper.tex"))?;
//!
//! // Access metadata
//! println!("Title: {:?}", doc.metadata.title);
//! println!("Author: {:?}", doc.metadata.author);
//!
//! // Access content blocks (DocItems)
//! if let Some(blocks) = &doc.content_blocks {
//!     println!("Content blocks: {}", blocks.len());
//!     for block in blocks {
//!         println!("  {:?}", block);
//!     }
//! }
//! # Ok::<(), anyhow::Error>(())
//! ```
//!
//! ## Output Format
//!
//! The parser produces `DocItem` blocks that can be serialized to markdown:
//!
//! | LaTeX Element | `DocItem` Type |
//! |---------------|--------------|
//! | `\section{}` | `SectionHeader` (level 1) |
//! | `\subsection{}` | `SectionHeader` (level 2) |
//! | Body text | `Text` |
//! | `itemize`/`enumerate` | `List` |
//! | `tabular` | `Table` |
//! | `\includegraphics{}` | `Picture` |
//!
//! ## Parser Architecture
//!
//! The parser uses a multi-pass approach:
//!
//! 1. **Preprocessing**: Remove comments, normalize whitespace
//! 2. **Metadata extraction**: Find `\title`, `\author`, `\date`
//! 3. **Structure parsing**: Identify sections and environments
//! 4. **Content extraction**: Parse text, lists, tables
//! 5. **`DocItem` generation**: Create structured output
//!
//! ## Use Cases
//!
//! - **Academic papers**: Parse research papers and theses
//! - **Documentation**: Convert LaTeX docs to other formats
//! - **Resume processing**: Extract structured resume data
//! - **Content migration**: Move LaTeX content to web/markdown
//!
//! ## Limitations
//!
//! LaTeX is Turing-complete, so full parsing would require a complete TeX engine.
//! This parser handles common academic document patterns using regex-based
//! extraction. The following may not parse correctly:
//!
//! - **Custom macros**: User-defined commands
//! - **Complex conditionals**: `\if`, `\else` constructs
//! - **Programmatic content**: Counter manipulation, loops
//! - **External packages**: Package-specific commands
//! - **Math rendering**: Equations are extracted as-is
//!
//! ## Example Documents
//!
//! The parser works well with:
//!
//! - Standard `article`, `report`, `book` classes
//! - Common templates (IEEE, ACM, Springer)
//! - Resume templates (Jake's resume, etc.)
//! - Simple beamer presentations

pub mod latex;

pub use latex::LatexBackend;
