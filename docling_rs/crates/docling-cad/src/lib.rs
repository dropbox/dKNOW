//! docling-cad - CAD and engineering format support for docling
//!
//! This crate provides parsers and converters for CAD and engineering file formats:
//! - **STL** (`STereoLithography`) - 3D mesh format for 3D printing and CAD
//! - **OBJ** (Wavefront Object) - 3D mesh format for modeling and graphics
//! - **GLTF/GLB** (GL Transmission Format) - Modern 3D format for web/AR/VR
//! - **DXF** (Drawing Exchange Format) - `AutoCAD` interchange format
//!
//! ## Examples
//!
//! Parse a STL file:
//!
//! ```rust,no_run
//! use docling_cad::{StlParser, stl_to_markdown};
//!
//! let mesh = StlParser::parse_file("model.stl")?;
//! println!("Triangles: {}", mesh.triangle_count);
//! println!("Bounding box: {:?} to {:?}", mesh.bbox_min, mesh.bbox_max);
//!
//! // Convert to markdown
//! let markdown = stl_to_markdown(&mesh);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Parse a DXF drawing:
//!
//! ```rust,no_run
//! use docling_cad::{DxfParser, dxf_to_markdown};
//!
//! let drawing = DxfParser::parse_file("blueprint.dxf")?;
//! println!("Layers: {}", drawing.layer_names.len());
//! println!("Entities: {}", drawing.entity_count);
//!
//! // Convert to markdown
//! let markdown = dxf_to_markdown(&drawing);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Parse a GLTF/GLB model:
//!
//! ```rust,no_run
//! use docling_cad::{GltfParser, gltf_to_markdown};
//!
//! let model = GltfParser::parse_file("scene.glb")?;
//! println!("Meshes: {}", model.mesh_count);
//! println!("Materials: {}", model.material_count);
//!
//! let markdown = gltf_to_markdown(&model);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Future Support
//!
//! Planned formats:
//! - IFC (Industry Foundation Classes) - BIM format

pub mod dxf;
pub mod gltf;
pub mod obj;
pub mod stl;

// Re-export main types
pub use dxf::{to_markdown as dxf_to_markdown, DxfDrawing, DxfParser};
pub use gltf::{to_markdown as gltf_to_markdown, GltfModel, GltfParser};
pub use obj::{to_markdown as obj_to_markdown, ObjMesh, ObjParser};
pub use stl::{to_markdown as stl_to_markdown, StlMesh, StlParser};
