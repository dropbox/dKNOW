//! IDML (`InDesign` Markup Language) parsing and serialization
//!
//! This module provides functionality to parse IDML files (`InDesign` documents)
//! and convert them to markdown format.
//!
//! # Examples
//!
//! ```no_run
//! use docling_adobe::idml::{IdmlParser, IdmlSerializer};
//!
//! let doc = IdmlParser::parse_file("document.idml").unwrap();
//! let markdown = IdmlSerializer::to_markdown(&doc);
//! println!("{}", markdown);
//! ```

/// IDML ZIP archive parser
pub mod parser;
/// IDML to markdown serializer
pub mod serializer;
/// IDML document type definitions
pub mod types;

pub use parser::IdmlParser;
pub use serializer::IdmlSerializer;
pub use types::{IdmlDocument, Metadata, Paragraph, Story};
