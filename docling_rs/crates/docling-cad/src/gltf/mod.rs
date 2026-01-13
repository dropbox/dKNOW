//! GLTF/GLB 3D model format support
//!
//! Provides parsing and markdown serialization for:
//! - GLTF 2.0 (JSON) - `.gltf` files
//! - GLB (Binary glTF) - `.glb` files
//!
//! # Features
//! - Full glTF 2.0 support via `gltf` crate
//! - Mesh geometry extraction (vertices, triangles)
//! - Scene graph information (nodes, scenes)
//! - Material and animation metadata
//! - Bounding box calculation
//! - Binary GLB format support
//!
//! # Example
//! ```no_run
//! use docling_cad::gltf::{GltfParser, to_markdown};
//!
//! let model = GltfParser::parse_file("model.gltf").unwrap();
//! let markdown = to_markdown(&model);
//! println!("{}", markdown);
//! ```

mod parser;
mod serializer;

pub use parser::{AnimationInfo, GltfModel, GltfParser, MaterialInfo};
pub use serializer::to_markdown;
