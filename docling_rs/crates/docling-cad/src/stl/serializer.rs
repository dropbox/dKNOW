//! STL markdown serializer
//!
//! Converts STL mesh data to markdown format for document processing.

use std::fmt::Write;

use super::parser::StlMesh;

/// Convert STL mesh to markdown representation
#[must_use = "serialization returns markdown string"]
pub fn to_markdown(mesh: &StlMesh) -> String {
    let mut md = String::new();

    // Title
    if let Some(ref name) = mesh.name {
        let _ = writeln!(md, "# STL Model: {name}\n");
    } else {
        md.push_str("# STL Model\n\n");
    }

    // Format type
    md.push_str("## File Information\n\n");
    let _ = writeln!(
        md,
        "- **Format**: STL ({})",
        if mesh.is_binary { "Binary" } else { "ASCII" }
    );

    // Mesh statistics
    md.push_str("\n## Mesh Statistics\n\n");
    let _ = writeln!(md, "- **Triangles**: {}", mesh.triangle_count);
    let _ = writeln!(md, "- **Vertices**: {}", mesh.vertex_count);

    // Bounding box
    md.push_str("\n## Bounding Box\n\n");
    let _ = writeln!(
        md,
        "- **Minimum**: ({:.3}, {:.3}, {:.3})",
        mesh.bbox_min[0], mesh.bbox_min[1], mesh.bbox_min[2]
    );
    let _ = writeln!(
        md,
        "- **Maximum**: ({:.3}, {:.3}, {:.3})",
        mesh.bbox_max[0], mesh.bbox_max[1], mesh.bbox_max[2]
    );

    // Dimensions
    let dims = mesh.dimensions();
    md.push_str("\n## Dimensions\n\n");
    let _ = writeln!(md, "- **Width** (X): {:.3}", dims[0]);
    let _ = writeln!(md, "- **Depth** (Y): {:.3}", dims[1]);
    let _ = writeln!(md, "- **Height** (Z): {:.3}", dims[2]);

    // Bounding volume
    let volume = mesh.bounding_volume();
    let _ = writeln!(md, "\n**Bounding Volume**: {volume:.3} cubic units");

    // Additional metadata
    md.push_str("\n## Model Description\n\n");
    let _ = write!(
        md,
        "This 3D model contains {} triangular faces forming a mesh with {} unique vertices. ",
        mesh.triangle_count, mesh.vertex_count
    );

    if mesh.is_binary {
        md.push_str("The model is stored in binary STL format for compact file size.");
    } else {
        md.push_str("The model is stored in ASCII STL format for human readability.");
    }

    md.push('\n');
    md
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stl::parser::StlParser;

    const SIMPLE_CUBE: &str = r"solid test_cube
  facet normal 0.0 0.0 1.0
    outer loop
      vertex 0.0 0.0 0.0
      vertex 1.0 0.0 0.0
      vertex 1.0 1.0 0.0
    endloop
  endfacet
  facet normal 0.0 0.0 1.0
    outer loop
      vertex 0.0 0.0 0.0
      vertex 1.0 1.0 0.0
      vertex 0.0 1.0 0.0
    endloop
  endfacet
endsolid test_cube
";

    #[test]
    fn test_to_markdown() {
        let mesh = StlParser::parse_str(SIMPLE_CUBE).unwrap();
        let md = to_markdown(&mesh);

        assert!(md.contains("# STL Model: test_cube"));
        assert!(md.contains("**Format**: STL (ASCII)"));
        assert!(md.contains("**Triangles**: 2"));
        assert!(md.contains("## Bounding Box"));
        assert!(md.contains("## Dimensions"));
    }

    #[test]
    fn test_markdown_structure() {
        let mesh = StlParser::parse_str(SIMPLE_CUBE).unwrap();
        let md = to_markdown(&mesh);

        // Check for expected sections
        assert!(md.contains("## File Information"));
        assert!(md.contains("## Mesh Statistics"));
        assert!(md.contains("## Bounding Box"));
        assert!(md.contains("## Dimensions"));
        assert!(md.contains("## Model Description"));
    }
}
