//! SVG (Scalable Vector Graphics) parser for docling
//!
//! This crate provides parsing capabilities for SVG files, extracting text content
//! and metadata from SVG documents.
//!
//! ## Supported Features
//!
//! - **Text extraction** - Extracts text from `<text>` and `<tspan>` elements
//! - **Metadata** - Parses `<title>`, `<desc>`, viewBox, width/height
//! - **Shapes** - Recognizes `<rect>`, `<circle>`, `<ellipse>`, `<line>`, `<polygon>`, `<polyline>`, `<path>`
//!
//! ## Examples
//!
//! Parse an SVG file:
//!
//! ```rust,no_run
//! use docling_svg::parse_svg;
//! use std::path::Path;
//!
//! let doc = parse_svg(Path::new("diagram.svg"))?;
//! println!("Title: {:?}", doc.metadata.title);
//! println!("Text elements: {}", doc.text_elements.len());
//! # Ok::<(), docling_svg::SvgError>(())
//! ```
//!
//! Parse from a string:
//!
//! ```rust
//! use docling_svg::parse_svg_str;
//!
//! let svg = r#"<svg viewBox="0 0 100 100"><text x="10" y="20">Hello SVG</text></svg>"#;
//! let doc = parse_svg_str(svg)?;
//! assert_eq!(doc.text_elements.len(), 1);
//! assert_eq!(doc.text_elements[0].content, "Hello SVG");
//! # Ok::<(), docling_svg::SvgError>(())
//! ```
//!
//! ## Format Details
//!
//! SVG is an XML-based vector image format. This parser focuses on extracting
//! human-readable content (text) rather than rendering the graphics. Shapes
//! are recognized but not rendered.

pub mod element;
pub mod error;
pub mod metadata;
pub mod parser;

// Re-export main types
pub use element::SvgTextElement;
pub use error::{Result, SvgError};
pub use metadata::SvgMetadata;
pub use parser::{parse_svg, parse_svg_str, SvgDocument};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        let svg = r"<svg><text>Test</text></svg>";
        let doc = parse_svg_str(svg).expect("Failed to parse");
        assert_eq!(doc.text_elements.len(), 1);
    }
}
