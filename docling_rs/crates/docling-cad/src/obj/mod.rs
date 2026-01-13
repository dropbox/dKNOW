//! OBJ file format support
//!
//! Wavefront OBJ is a widely-used 3D geometry format that stores vertices, faces,
//! normals, texture coordinates, and materials. This module provides parsing
//! and serialization for OBJ files.

mod parser;
mod serializer;

pub use parser::{ObjMesh, ObjParser};
pub use serializer::to_markdown;
