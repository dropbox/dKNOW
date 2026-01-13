//! OBJ file parser
//!
//! Parses OBJ files (Wavefront Object format) using the tobj crate.
//! OBJ is a text-based format for 3D models with vertex positions,
//! normals, texture coordinates, and material references.

use anyhow::{Context, Result};
use std::path::Path;

/// OBJ mesh data
#[derive(Debug, Clone)]
pub struct ObjMesh {
    /// Model name
    pub name: String,
    /// Number of models (objects) in the file
    pub model_count: usize,
    /// Number of unique vertices across all models
    pub vertex_count: usize,
    /// Number of faces (triangles) across all models
    pub face_count: usize,
    /// Number of materials
    pub material_count: usize,
    /// Bounding box minimum
    pub bbox_min: [f32; 3],
    /// Bounding box maximum
    pub bbox_max: [f32; 3],
    /// Whether normals are present
    pub has_normals: bool,
    /// Whether texture coordinates are present
    pub has_texcoords: bool,
    /// Number of vertex normals across all models
    pub normal_count: usize,
    /// Number of texture coordinates across all models
    pub texcoord_count: usize,
    /// Model names
    pub model_names: Vec<String>,
    /// Raw tobj models
    pub models: Vec<tobj::Model>,
    /// Raw tobj materials
    pub materials: Vec<tobj::Material>,
}

impl ObjMesh {
    /// Calculate bounding box from all model vertices
    ///
    /// Computes the axis-aligned bounding box encompassing all vertices
    /// across all models in the OBJ file. Used internally during parsing.
    ///
    /// # Arguments
    ///
    /// * `models` - Slice of tobj models to compute bounding box from
    ///
    /// # Returns
    ///
    /// Tuple of (min, max) coordinates as `[x, y, z]` arrays.
    /// Returns `([0,0,0], [0,0,0])` if no vertices found.
    pub(crate) fn calculate_bbox(models: &[tobj::Model]) -> ([f32; 3], [f32; 3]) {
        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];
        let mut found_any = false;

        for model in models {
            let positions = &model.mesh.positions;
            // Positions are stored as flat array: [x1, y1, z1, x2, y2, z2, ...]
            for chunk in positions.chunks(3) {
                if chunk.len() == 3 {
                    found_any = true;
                    for i in 0..3 {
                        min[i] = min[i].min(chunk[i]);
                        max[i] = max[i].max(chunk[i]);
                    }
                }
            }
        }

        if !found_any {
            return ([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]);
        }

        (min, max)
    }

    /// Get mesh dimensions (width, height, depth)
    ///
    /// Returns the dimensions of the mesh's bounding box as `[width, height, depth]`.
    /// Calculated by subtracting the minimum bounding box coordinates from the maximum.
    ///
    /// # Returns
    ///
    /// Array of three floats representing the width (X), height (Y), and depth (Z) of the mesh.
    #[inline]
    #[must_use = "dimensions returns width/height/depth array"]
    pub fn dimensions(&self) -> [f32; 3] {
        [
            self.bbox_max[0] - self.bbox_min[0],
            self.bbox_max[1] - self.bbox_min[1],
            self.bbox_max[2] - self.bbox_min[2],
        ]
    }

    /// Get mesh volume (bounding box volume, not actual volume)
    ///
    /// Returns the volume of the mesh's axis-aligned bounding box.
    /// Note: This is NOT the actual mesh volume, just the bounding box volume.
    /// For actual mesh volume, you would need to integrate over all faces.
    ///
    /// # Returns
    ///
    /// Volume in cubic units (width × height × depth).
    #[inline]
    #[must_use = "bounding_volume returns volume in cubic units"]
    pub fn bounding_volume(&self) -> f32 {
        let dims = self.dimensions();
        dims[0] * dims[1] * dims[2]
    }
}

/// OBJ parser
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct ObjParser;

impl ObjParser {
    /// Parse OBJ file from path
    ///
    /// Reads and parses a Wavefront OBJ file including vertices, faces, normals,
    /// texture coordinates, and material references. Automatically loads associated
    /// MTL (material) files referenced in the OBJ file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the OBJ file
    ///
    /// # Returns
    ///
    /// Parsed OBJ mesh with vertex data, face count, bounding box, and material info.
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be opened, contains invalid OBJ syntax,
    /// or references materials that cannot be loaded.
    #[must_use = "parsing produces a result that should be handled"]
    pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<ObjMesh> {
        let path = path.as_ref();

        // Load OBJ file with default options
        let load_options = tobj::LoadOptions {
            triangulate: true,   // Convert all polygons to triangles
            single_index: false, // Keep separate indices for positions/normals/texcoords
            ..Default::default()
        };

        let (models, materials) = tobj::load_obj(path, &load_options)
            .with_context(|| format!("Failed to parse OBJ file: {}", path.display()))?;

        let materials = materials.with_context(|| {
            format!(
                "Failed to load MTL materials for OBJ file: {}",
                path.display()
            )
        })?;

        // Extract title from first comment line, or fallback to filename
        let name = std::fs::read_to_string(path)
            .ok()
            .and_then(|content| {
                content
                    .lines()
                    .find(|line| line.trim().starts_with('#'))
                    .map(|line| line.trim().trim_start_matches('#').trim().to_string())
            })
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unnamed")
                    .to_string()
            });

        let model_count = models.len();

        // Count total vertices and faces
        let mut vertex_count = 0;
        let mut face_count = 0;
        let mut has_normals = false;
        let mut has_texcoords = false;
        let mut normal_count = 0;
        let mut texcoord_count = 0;
        let mut model_names = Vec::new();

        for model in &models {
            // Positions are stored as flat array [x1, y1, z1, x2, y2, z2, ...]
            vertex_count += model.mesh.positions.len() / 3;

            // Indices refer to vertices (triangulated, so 3 indices = 1 face)
            face_count += model.mesh.indices.len() / 3;

            // Normals are stored as flat array [nx1, ny1, nz1, nx2, ny2, nz2, ...]
            let model_normals = model.mesh.normals.len() / 3;
            normal_count += model_normals;
            has_normals = has_normals || model_normals > 0;

            // Texture coordinates are stored as flat array [u1, v1, u2, v2, ...]
            let model_texcoords = model.mesh.texcoords.len() / 2;
            texcoord_count += model_texcoords;
            has_texcoords = has_texcoords || model_texcoords > 0;

            model_names.push(model.name.clone());
        }

        let (bbox_min, bbox_max) = ObjMesh::calculate_bbox(&models);

        Ok(ObjMesh {
            name,
            model_count,
            vertex_count,
            face_count,
            material_count: materials.len(),
            bbox_min,
            bbox_max,
            has_normals,
            has_texcoords,
            normal_count,
            texcoord_count,
            model_names,
            models,
            materials,
        })
    }

    /// Parse OBJ from string
    ///
    /// **Not supported for OBJ files.** OBJ files often reference external MTL
    /// (material) files, which requires filesystem access. Use `parse_file` instead.
    ///
    /// # Arguments
    ///
    /// * `_data` - OBJ file content as string (unused)
    /// * `filename` - Original filename for error messages
    ///
    /// # Returns
    ///
    /// Always returns error suggesting to use `parse_file` instead.
    ///
    /// # Errors
    ///
    /// Always returns error as this operation is not supported.
    #[must_use = "parsing produces a result that should be handled"]
    pub fn parse_str(_data: &str, filename: &str) -> Result<ObjMesh> {
        // tobj doesn't have a direct parse_str API, so we need to write to a temp file
        // For now, we'll return an error suggesting to use parse_file instead
        anyhow::bail!(
            "parse_str not supported for OBJ files. \
             Please save to a file and use parse_file instead. \
             OBJ files often reference external MTL (material) files, \
             which requires filesystem access. Filename: {filename}"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_simple_obj() -> &'static str {
        // Simple cube OBJ (8 vertices, 12 triangular faces)
        r"# Simple cube
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 1.0 1.0 0.0
v 0.0 1.0 0.0
v 0.0 0.0 1.0
v 1.0 0.0 1.0
v 1.0 1.0 1.0
v 0.0 1.0 1.0

# Front face
f 1 2 3
f 1 3 4
# Back face
f 5 7 6
f 5 8 7
# Top face
f 4 3 7
f 4 7 8
# Bottom face
f 1 6 2
f 1 5 6
# Right face
f 2 6 7
f 2 7 3
# Left face
f 1 4 8
f 1 8 5
"
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_parse_simple_cube() {
        let obj_content = create_simple_obj();
        let temp_dir = std::env::temp_dir();
        let obj_path = temp_dir.join("test_cube.obj");

        std::fs::write(&obj_path, obj_content).expect("Failed to write test OBJ");

        let mesh = ObjParser::parse_file(&obj_path).expect("Failed to parse OBJ");

        assert_eq!(mesh.vertex_count, 8, "Should have 8 vertices");
        assert_eq!(mesh.face_count, 12, "Should have 12 triangular faces");
        assert_eq!(mesh.model_count, 1, "Should have 1 model");

        // Check that title is extracted from first comment
        assert_eq!(
            mesh.name, "Simple cube",
            "Should extract title from first comment line"
        );

        // Check bounding box (0,0,0) to (1,1,1)
        assert_eq!(mesh.bbox_min, [0.0, 0.0, 0.0]);
        assert_eq!(mesh.bbox_max, [1.0, 1.0, 1.0]);

        // Check dimensions
        let dims = mesh.dimensions();
        assert_eq!(dims, [1.0, 1.0, 1.0]);

        // Clean up
        let _ = std::fs::remove_file(&obj_path);
    }

    #[test]
    fn test_parse_with_normals() {
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
        let obj_path = temp_dir.join("test_normals.obj");

        std::fs::write(&obj_path, obj_content).expect("Failed to write test OBJ");

        let mesh = ObjParser::parse_file(&obj_path).expect("Failed to parse OBJ");

        assert_eq!(mesh.vertex_count, 3);
        assert_eq!(mesh.face_count, 1);
        assert!(mesh.has_normals, "Should have normals");

        let _ = std::fs::remove_file(&obj_path);
    }

    #[test]
    fn test_parse_with_texcoords() {
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
        let obj_path = temp_dir.join("test_texcoords.obj");

        std::fs::write(&obj_path, obj_content).expect("Failed to write test OBJ");

        let mesh = ObjParser::parse_file(&obj_path).expect("Failed to parse OBJ");

        assert_eq!(mesh.vertex_count, 3);
        assert_eq!(mesh.face_count, 1);
        assert!(mesh.has_texcoords, "Should have texture coordinates");

        let _ = std::fs::remove_file(&obj_path);
    }

    #[test]
    fn test_bounding_volume() {
        let obj_content = create_simple_obj();
        let temp_dir = std::env::temp_dir();
        let obj_path = temp_dir.join("test_volume.obj");

        std::fs::write(&obj_path, obj_content).expect("Failed to write test OBJ");

        let mesh = ObjParser::parse_file(&obj_path).expect("Failed to parse OBJ");

        // 1x1x1 cube should have volume of 1.0
        let volume = mesh.bounding_volume();
        assert!(
            (volume - 1.0).abs() < 0.001,
            "Volume should be ~1.0, got {volume}"
        );

        let _ = std::fs::remove_file(&obj_path);
    }

    #[test]
    fn test_parse_nonexistent_file() {
        let result = ObjParser::parse_file("/nonexistent/path/file.obj");
        assert!(result.is_err(), "Should fail for nonexistent file");
    }

    #[test]
    fn test_parse_str_not_supported() {
        let result = ObjParser::parse_str("v 0 0 0", "test.obj");
        assert!(result.is_err(), "parse_str should return error");
        assert!(result.unwrap_err().to_string().contains("not supported"));
    }
}
