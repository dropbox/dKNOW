//! DXF (Drawing Exchange Format) parser module
//!
//! DXF is a CAD data file format developed by Autodesk for enabling data
//! interoperability between `AutoCAD` and other CAD applications.
//! Supports versions from R10 through 2018.

pub mod parser;
pub mod serializer;

pub use parser::{DxfDrawing, DxfParser};
pub use serializer::to_markdown;
