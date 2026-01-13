//! CAD and Engineering Format Processing
//!
//! This module provides functions for processing CAD and engineering file formats:
//! - STL (`STereoLithography`) - 3D mesh format
//! - OBJ (Wavefront Object) - 3D mesh format
//! - GLTF/GLB (GL Transmission Format) - Modern 3D format
//! - DXF (Drawing Exchange Format) - `AutoCAD` interchange format
//! - IFC (Industry Foundation Classes) - Future

use crate::error::Result;
use std::path::Path;

/// Process STL file and return markdown representation
///
/// # Errors
/// Returns an error if the file cannot be parsed as valid STL.
#[must_use = "this function returns the extracted markdown content"]
pub fn process_stl<P: AsRef<Path>>(path: P) -> Result<String> {
    let path = path.as_ref();

    // Parse STL file using docling-cad
    let mesh = docling_cad::StlParser::parse_file(path)?;

    // Convert to markdown
    let markdown = docling_cad::stl_to_markdown(&mesh);

    Ok(markdown)
}

/// Process OBJ file and return markdown representation
///
/// # Errors
/// Returns an error if the file cannot be parsed as valid OBJ.
#[must_use = "this function returns the extracted markdown content"]
pub fn process_obj<P: AsRef<Path>>(path: P) -> Result<String> {
    let path = path.as_ref();

    // Parse OBJ file using docling-cad
    let mesh = docling_cad::ObjParser::parse_file(path)?;

    // Convert to markdown
    let markdown = docling_cad::obj_to_markdown(&mesh);

    Ok(markdown)
}

/// Process GLTF/GLB file and return markdown representation
///
/// # Errors
/// Returns an error if the file cannot be parsed as valid GLTF/GLB.
#[must_use = "this function returns the extracted markdown content"]
pub fn process_gltf<P: AsRef<Path>>(path: P) -> Result<String> {
    let path = path.as_ref();

    // Parse GLTF/GLB file using docling-cad
    let model = docling_cad::GltfParser::parse_file(path)?;

    // Convert to markdown
    let markdown = docling_cad::gltf_to_markdown(&model);

    Ok(markdown)
}

/// Process DXF file and return markdown representation
///
/// # Errors
/// Returns an error if the file cannot be parsed as valid DXF.
#[must_use = "this function returns the extracted markdown content"]
pub fn process_dxf<P: AsRef<Path>>(path: P) -> Result<String> {
    let path = path.as_ref();

    // Parse DXF file using docling-cad
    let drawing = docling_cad::DxfParser::parse_file(path)?;

    // Convert to markdown
    let markdown = docling_cad::dxf_to_markdown(&drawing);

    Ok(markdown)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_stl_basic() {
        // Test with our generated test files
        let test_file = "test-corpus/cad/stl/simple_cube.stl";
        if std::path::Path::new(test_file).exists() {
            let result = process_stl(test_file);
            assert!(result.is_ok());

            let markdown = result.unwrap();
            assert!(markdown.contains("# STL Model"));
            assert!(markdown.contains("**Triangles**"));
        }
    }

    #[test]
    fn test_process_obj_basic() {
        // Test with OBJ test files
        let test_file = "test-corpus/cad/obj/simple_cube.obj";
        if std::path::Path::new(test_file).exists() {
            let result = process_obj(test_file);
            assert!(result.is_ok());

            let markdown = result.unwrap();
            assert!(markdown.contains("# 3D Model"));
            assert!(markdown.contains("Wavefront OBJ"));
        }
    }

    #[test]
    fn test_process_gltf_basic() {
        // Test with GLTF test files
        let test_file = "test-corpus/cad/gltf/simple_triangle.gltf";
        if std::path::Path::new(test_file).exists() {
            let result = process_gltf(test_file);
            assert!(result.is_ok());

            let markdown = result.unwrap();
            assert!(markdown.contains("# 3D Model"));
            assert!(markdown.contains("glTF"));
        }
    }

    #[test]
    fn test_process_glb_basic() {
        // Test with GLB binary test files
        let test_file = "test-corpus/cad/gltf/box.glb";
        if std::path::Path::new(test_file).exists() {
            let result = process_gltf(test_file);
            assert!(result.is_ok());

            let markdown = result.unwrap();
            assert!(markdown.contains("# 3D Model"));
            assert!(markdown.contains("GLB"));
        }
    }

    #[test]
    fn test_process_dxf_basic() {
        // Test with DXF test files
        let test_file = "test-corpus/cad/dxf/simple_drawing.dxf";
        if std::path::Path::new(test_file).exists() {
            let result = process_dxf(test_file);
            assert!(result.is_ok());

            let markdown = result.unwrap();
            assert!(markdown.contains("# DXF Drawing"));
            assert!(markdown.contains("Drawing Exchange Format"));
        }
    }

    #[test]
    fn test_process_stl_nonexistent_file() {
        // Test error handling for missing file
        let result = process_stl("nonexistent_file.stl");
        assert!(result.is_err());
    }

    #[test]
    fn test_process_obj_nonexistent_file() {
        // Test error handling for missing OBJ file
        let result = process_obj("nonexistent_file.obj");
        assert!(result.is_err());
    }

    #[test]
    fn test_process_gltf_nonexistent_file() {
        // Test error handling for missing GLTF file
        let result = process_gltf("nonexistent_file.gltf");
        assert!(result.is_err());
    }

    #[test]
    fn test_process_dxf_nonexistent_file() {
        // Test error handling for missing DXF file
        let result = process_dxf("nonexistent_file.dxf");
        assert!(result.is_err());
    }

    #[test]
    fn test_process_glb_binary_format() {
        // Test GLB binary format (different from text GLTF)
        // Both use same process_gltf function
        let glb_test = "test-corpus/cad/gltf/model.glb";
        let gltf_test = "test-corpus/cad/gltf/model.gltf";

        // Both should be processable by the same function
        // This tests format flexibility
        if std::path::Path::new(glb_test).exists() {
            let result = process_gltf(glb_test);
            assert!(result.is_ok());
        }

        if std::path::Path::new(gltf_test).exists() {
            let result = process_gltf(gltf_test);
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_stl_ascii_vs_binary() {
        // STL supports both ASCII and binary formats
        // Both should be parseable
        let ascii_stl = "test-corpus/cad/stl/ascii_cube.stl";
        let binary_stl = "test-corpus/cad/stl/binary_cube.stl";

        if std::path::Path::new(ascii_stl).exists() {
            let result = process_stl(ascii_stl);
            assert!(result.is_ok(), "ASCII STL should parse successfully");
        }

        if std::path::Path::new(binary_stl).exists() {
            let result = process_stl(binary_stl);
            assert!(result.is_ok(), "Binary STL should parse successfully");
        }
    }

    #[test]
    fn test_obj_with_materials() {
        // OBJ files can reference MTL material files
        // Parser should handle this gracefully
        let obj_with_mtl = "test-corpus/cad/obj/textured_model.obj";

        if std::path::Path::new(obj_with_mtl).exists() {
            let result = process_obj(obj_with_mtl);
            assert!(result.is_ok());

            let markdown = result.unwrap();
            // Should mention materials if present
            assert!(markdown.contains("# 3D Model") || markdown.contains("Wavefront OBJ"));
        }
    }

    #[test]
    fn test_dxf_version_compatibility() {
        // DXF has many versions (R12, R14, R2000, R2007, etc.)
        // Test that parser handles different versions gracefully
        let modern_dxf = "test-corpus/cad/dxf/r2007_drawing.dxf";
        let legacy_dxf = "test-corpus/cad/dxf/r12_drawing.dxf";

        if std::path::Path::new(modern_dxf).exists() {
            let result = process_dxf(modern_dxf);
            // Should either succeed or return version error
            // We just verify it handles it without panicking
            let _ = result;
        }

        if std::path::Path::new(legacy_dxf).exists() {
            let result = process_dxf(legacy_dxf);
            let _ = result;
        }
    }

    #[test]
    fn test_cad_format_markdown_structure() {
        // Test that markdown output has expected structure
        // Using any available test file
        let test_files = [
            "test-corpus/cad/stl/simple_cube.stl",
            "test-corpus/cad/obj/simple_cube.obj",
            "test-corpus/cad/gltf/simple_triangle.gltf",
            "test-corpus/cad/dxf/simple_drawing.dxf",
        ];

        for test_file in &test_files {
            if std::path::Path::new(test_file).exists() {
                let result = if test_file.ends_with(".stl") {
                    process_stl(test_file)
                } else if test_file.ends_with(".obj") {
                    process_obj(test_file)
                } else if test_file.ends_with(".gltf") {
                    process_gltf(test_file)
                } else if test_file.ends_with(".dxf") {
                    process_dxf(test_file)
                } else {
                    continue;
                };

                if let Ok(markdown) = result {
                    // All should start with heading
                    assert!(markdown.starts_with('#'));
                    // All should end with newline
                    assert!(markdown.ends_with('\n'));
                    // Should have some content beyond just heading
                    assert!(markdown.len() > 20);
                }
            }
        }
    }
}
