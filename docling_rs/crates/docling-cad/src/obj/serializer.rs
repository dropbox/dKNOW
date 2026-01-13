//! OBJ to Markdown serialization
//!
//! Converts parsed OBJ mesh data into a human-readable markdown format
//! suitable for document processing.

use std::fmt::Write;

use super::ObjMesh;

/// Convert OBJ mesh to markdown format
///
/// Generates a markdown document containing:
/// - Model name and file metadata
/// - Geometry statistics (vertices, faces, models)
/// - Bounding box and dimensions
/// - Material information
/// - Model list
///
/// # Arguments
/// * `mesh` - Parsed OBJ mesh data
///
/// # Returns
/// Markdown-formatted string
#[must_use = "serialization returns markdown string"]
#[allow(clippy::too_many_lines)] // Complex OBJ serialization - keeping together for clarity
pub fn to_markdown(mesh: &ObjMesh) -> String {
    let mut output = String::new();

    // Title - use original title from first comment or filename (no "3D Model:" prefix)
    // LLM Quality (N=2180): Match original document title format exactly
    let _ = writeln!(output, "# {}", mesh.name);

    // Format indicator
    output.push_str("- Format: Wavefront OBJ (3D Geometry)\n\n");

    // Geometry statistics
    output.push_str("## Geometry Statistics\n\n");
    let _ = writeln!(output, "- **Models (Objects):** {}", mesh.model_count);
    let _ = writeln!(output, "- **Total Vertices:** {}", mesh.vertex_count);
    let _ = writeln!(output, "- **Total Faces:** {} (triangles)", mesh.face_count);

    if mesh.material_count > 0 {
        let _ = writeln!(output, "- **Materials:** {}", mesh.material_count);
    }

    output.push('\n');

    // Features
    output.push_str("## Features\n\n");
    if mesh.has_normals {
        let _ = writeln!(
            output,
            "- **Vertex Normals:** {} normals",
            mesh.normal_count
        );
    } else {
        output.push_str("- **Vertex Normals:** Not present in file (no 'vn' entries)\n");
    }
    if mesh.has_texcoords {
        let _ = writeln!(
            output,
            "- **Texture Coordinates:** {} coordinates",
            mesh.texcoord_count
        );
    } else {
        output.push_str("- **Texture Coordinates:** Not present in file (no 'vt' entries)\n");
    }
    output.push('\n');

    // Bounding box
    output.push_str("## Bounding Box\n\n");
    let _ = writeln!(
        output,
        "- **Minimum:** ({:.3}, {:.3}, {:.3})",
        mesh.bbox_min[0], mesh.bbox_min[1], mesh.bbox_min[2]
    );
    let _ = writeln!(
        output,
        "- **Maximum:** ({:.3}, {:.3}, {:.3})",
        mesh.bbox_max[0], mesh.bbox_max[1], mesh.bbox_max[2]
    );

    let dims = mesh.dimensions();
    let _ = writeln!(
        output,
        "- **Dimensions:** {:.3} x {:.3} x {:.3} (W x H x D)",
        dims[0], dims[1], dims[2]
    );

    let volume = mesh.bounding_volume();
    let _ = writeln!(output, "- **Bounding Volume:** {volume:.3} cubic units");
    output.push('\n');

    // Model list
    if mesh.model_count > 1 {
        output.push_str("## Models\n\n");
        for (idx, model_name) in mesh.model_names.iter().enumerate() {
            let model = &mesh.models[idx];
            let vertex_count = model.mesh.positions.len() / 3;
            let face_count = model.mesh.indices.len() / 3;

            let _ = writeln!(
                output,
                "{}. {} - {} vertices, {} faces",
                idx + 1,
                if model_name.is_empty() {
                    "Unnamed"
                } else {
                    model_name
                },
                vertex_count,
                face_count
            );
        }
        output.push('\n');
    }

    // Materials (if present)
    if !mesh.materials.is_empty() {
        output.push_str("## Materials\n\n");
        for (idx, material) in mesh.materials.iter().enumerate() {
            let _ = writeln!(output, "{}. {}", idx + 1, material.name);

            // Ambient color
            if let Some([r, g, b]) = material.ambient {
                let _ = writeln!(output, "   - Ambient: RGB({r:.2}, {g:.2}, {b:.2})");
            }

            // Diffuse color
            if let Some([r, g, b]) = material.diffuse {
                let _ = writeln!(output, "   - Diffuse: RGB({r:.2}, {g:.2}, {b:.2})");
            }

            // Specular color
            if let Some([r, g, b]) = material.specular {
                let _ = writeln!(output, "   - Specular: RGB({r:.2}, {g:.2}, {b:.2})");
            }

            // Diffuse texture map
            if let Some(ref diffuse_texture) = material.diffuse_texture {
                let _ = writeln!(output, "   - Diffuse Texture: {diffuse_texture}");
            }
        }
        output.push('\n');
    }

    // Summary
    output.push_str("## Summary\n\n");
    let _ = writeln!(
        output,
        "This OBJ file contains {} 3D model(s) with a total of {} vertices and {} triangular faces",
        mesh.model_count, mesh.vertex_count, mesh.face_count
    );

    if mesh.has_normals || mesh.has_texcoords {
        output.push_str(", including ");
        let mut features = Vec::new();
        if mesh.has_normals {
            features.push("vertex normals");
        }
        if mesh.has_texcoords {
            features.push("texture coordinates");
        }
        output.push_str(&features.join(" and "));
    }

    output.push_str(".\n");

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::obj::ObjParser;

    fn create_simple_obj() -> &'static str {
        r"# Simple cube
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 1.0 1.0 0.0
v 0.0 1.0 0.0
v 0.0 0.0 1.0
v 1.0 0.0 1.0
v 1.0 1.0 1.0
v 0.0 1.0 1.0

f 1 2 3
f 1 3 4
f 5 7 6
f 5 8 7
f 4 3 7
f 4 7 8
f 1 6 2
f 1 5 6
f 2 6 7
f 2 7 3
f 1 4 8
f 1 8 5
"
    }

    #[test]
    fn test_to_markdown_basic() {
        let obj_content = create_simple_obj();
        let temp_dir = std::env::temp_dir();
        let obj_path = temp_dir.join("test_markdown.obj");

        std::fs::write(&obj_path, obj_content).expect("Failed to write test OBJ");

        let mesh = ObjParser::parse_file(&obj_path).expect("Failed to parse OBJ");
        let markdown = to_markdown(&mesh);

        // Check that markdown contains key information
        assert!(markdown.contains("# Simple cube")); // Title from first comment
        assert!(markdown.contains("Wavefront OBJ"));
        assert!(markdown.contains('8')); // 8 vertices
        assert!(markdown.contains("12")); // 12 faces
        assert!(markdown.contains("Bounding Box"));
        assert!(markdown.contains("Dimensions"));

        let _ = std::fs::remove_file(&obj_path);
    }

    #[test]
    fn test_to_markdown_with_normals() {
        let obj_content = r"# Triangle with normals
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 0.5 1.0 0.0
vn 0.0 0.0 1.0
vn 0.0 0.0 1.0
vn 0.0 0.0 1.0
f 1//1 2//2 3//3
";
        let temp_dir = std::env::temp_dir();
        let obj_path = temp_dir.join("test_normals_md.obj");

        std::fs::write(&obj_path, obj_content).expect("Failed to write test OBJ");

        let mesh = ObjParser::parse_file(&obj_path).expect("Failed to parse OBJ");
        let markdown = to_markdown(&mesh);

        assert!(markdown.contains("Vertex Normals:** 3 normals"));
        assert!(markdown.contains("vertex normals"));

        let _ = std::fs::remove_file(&obj_path);
    }

    #[test]
    fn test_to_markdown_with_texcoords() {
        let obj_content = r"# Triangle with texture coordinates
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 0.5 1.0 0.0
vt 0.0 0.0
vt 1.0 0.0
vt 0.5 1.0
f 1/1 2/2 3/3
";
        let temp_dir = std::env::temp_dir();
        let obj_path = temp_dir.join("test_texcoords_md.obj");

        std::fs::write(&obj_path, obj_content).expect("Failed to write test OBJ");

        let mesh = ObjParser::parse_file(&obj_path).expect("Failed to parse OBJ");
        let markdown = to_markdown(&mesh);

        assert!(markdown.contains("Texture Coordinates:** 3 coordinates"));
        assert!(markdown.contains("texture coordinates"));

        let _ = std::fs::remove_file(&obj_path);
    }
}
