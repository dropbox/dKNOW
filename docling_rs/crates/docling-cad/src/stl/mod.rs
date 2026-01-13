//! STL (`STereoLithography`) format parser module
//!
//! STL is a 3D mesh format widely used in 3D printing and CAD.
//! Supports both ASCII and binary variants.

pub mod parser;
pub mod serializer;

pub use parser::{StlMesh, StlParser};
pub use serializer::to_markdown;
