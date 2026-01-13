//! STL file parser
//!
//! Parses STL files (both ASCII and binary) using the `stl_io` crate.

use anyhow::{Context, Result};
use std::fs::File;
use std::path::Path;
use stl_io::{IndexedMesh, Vector};

/// STL mesh data
#[derive(Debug, Clone, PartialEq)]
pub struct StlMesh {
    /// Mesh name (from 'solid `<name>`' header)
    pub name: Option<String>,
    /// Number of triangles
    pub triangle_count: usize,
    /// Number of unique vertices
    pub vertex_count: usize,
    /// Bounding box minimum
    pub bbox_min: [f32; 3],
    /// Bounding box maximum
    pub bbox_max: [f32; 3],
    /// Whether file is binary or ASCII
    pub is_binary: bool,
    /// Raw indexed mesh (vertices + triangles)
    pub mesh: IndexedMesh,
}

impl StlMesh {
    /// Calculate bounding box from vertices
    fn calculate_bbox(vertices: &[Vector<f32>]) -> ([f32; 3], [f32; 3]) {
        if vertices.is_empty() {
            return ([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]);
        }

        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];

        for vertex in vertices {
            for i in 0..3 {
                min[i] = min[i].min(vertex[i]);
                max[i] = max[i].max(vertex[i]);
            }
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
    /// For actual mesh volume, you would need to integrate over all triangles.
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

/// STL parser
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct StlParser;

impl StlParser {
    /// Parse STL file from path
    ///
    /// Reads and parses an STL file (both ASCII and binary formats supported).
    /// Automatically detects the file format and extracts mesh data including
    /// vertices, triangles, bounding box, and metadata.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the STL file
    ///
    /// # Returns
    ///
    /// Parsed STL mesh with vertex data, triangle count, bounding box, and format info.
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be opened or contains invalid STL data.
    #[must_use = "parsing produces a result that should be handled"]
    pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<StlMesh> {
        let path = path.as_ref();
        let mut file = File::open(path)
            .with_context(|| format!("Failed to open STL file: {}", path.display()))?;

        // Try to read as binary first
        let mesh = stl_io::read_stl(&mut file)
            .with_context(|| format!("Failed to parse STL file: {}", path.display()))?;

        let name = Self::extract_name_from_path(path);
        let triangle_count = mesh.faces.len();
        let vertex_count = mesh.vertices.len();
        let (bbox_min, bbox_max) = StlMesh::calculate_bbox(&mesh.vertices);

        // Detect if binary or ASCII
        // stl_io handles both automatically, but we can infer from file size
        let is_binary = Self::is_likely_binary(path, triangle_count)?;

        Ok(StlMesh {
            name,
            triangle_count,
            vertex_count,
            bbox_min,
            bbox_max,
            is_binary,
            mesh,
        })
    }

    /// Parse STL from string (ASCII only)
    ///
    /// Parses STL data from a string containing ASCII STL format.
    /// Binary STL format is not supported for string input.
    ///
    /// # Arguments
    ///
    /// * `content` - STL file content as string (ASCII format)
    ///
    /// # Returns
    ///
    /// Parsed STL mesh with vertex data, triangle count, and bounding box.
    ///
    /// # Errors
    ///
    /// Returns error if the string contains invalid STL syntax.
    #[must_use = "parsing produces a result that should be handled"]
    pub fn parse_str(content: &str) -> Result<StlMesh> {
        let mut cursor = std::io::Cursor::new(content.as_bytes());
        let mesh = stl_io::read_stl(&mut cursor).context("Failed to parse STL string (ASCII)")?;

        let name = Self::extract_name_from_header(content);
        let triangle_count = mesh.faces.len();
        let vertex_count = mesh.vertices.len();
        let (bbox_min, bbox_max) = StlMesh::calculate_bbox(&mesh.vertices);

        Ok(StlMesh {
            name,
            triangle_count,
            vertex_count,
            bbox_min,
            bbox_max,
            is_binary: false, // String input is always ASCII
            mesh,
        })
    }

    /// Extract mesh name from file path
    fn extract_name_from_path(path: &Path) -> Option<String> {
        path.file_stem()
            .and_then(|s| s.to_str())
            .map(std::string::ToString::to_string)
    }

    /// Extract mesh name from ASCII STL header
    fn extract_name_from_header(content: &str) -> Option<String> {
        // ASCII STL starts with: solid <name>
        content
            .lines()
            .next()
            .and_then(|line| {
                line.strip_prefix("solid ")
                    .map(|name| name.trim().to_string())
            })
            .filter(|name| !name.is_empty())
    }

    /// Detect if file is binary or ASCII STL format
    ///
    /// Uses a robust detection method based on file content:
    /// - ASCII STL files start with "solid " (6 bytes)
    /// - Binary STL files start with an 80-byte header (arbitrary binary data)
    ///
    /// We also validate using file size as a secondary check:
    /// - Binary STL: exactly 80 + 4 + (50 * `triangle_count`) bytes
    /// - ASCII STL: variable size, typically 200-300 bytes per triangle
    fn is_likely_binary(path: &Path, triangle_count: usize) -> Result<bool> {
        use std::io::Read;

        let mut file = File::open(path)?;
        let mut header = [0u8; 6];

        // Try to read first 6 bytes
        let bytes_read = file.read(&mut header)?;
        if bytes_read < 5 {
            // File too small, assume ASCII
            return Ok(false);
        }

        // ASCII STL files start with "solid " (including the space)
        // Note: Some ASCII files might start with "solid" without a space, but this is rare
        if &header[0..5] == b"solid" {
            // Could be ASCII, but some binary files also start with "solid" in the 80-byte header
            // Use file size as secondary check
            let metadata = std::fs::metadata(path)?;
            // Safe conversion: files larger than usize::MAX are treated as binary
            let file_size: usize = metadata.len().try_into().unwrap_or(usize::MAX);

            // Binary STL: exactly 80 + 4 + (50 * triangle_count) bytes
            let expected_binary_size = 80 + 4 + (50 * triangle_count);

            // If file size matches binary format exactly (within 10 bytes), it's binary
            // Otherwise, it's ASCII
            // Wrap safe: file sizes >2^63-1 bytes (9 exabytes) are unrealistic for STL files
            #[allow(clippy::cast_possible_wrap)]
            let size_diff = (file_size as i64 - expected_binary_size as i64).abs();
            if size_diff < 10 {
                return Ok(true); // File size matches binary format
            }

            // File starts with "solid" but size doesn't match binary - it's ASCII
            return Ok(false);
        }

        // Doesn't start with "solid" - must be binary
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_parse_str() {
        let mesh = StlParser::parse_str(SIMPLE_CUBE).unwrap();
        assert_eq!(mesh.triangle_count, 2);
        assert!(mesh.vertex_count >= 3); // At least 3 unique vertices
        assert_eq!(mesh.name, Some("test_cube".to_string()));
        assert!(!mesh.is_binary);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_bounding_box() {
        let mesh = StlParser::parse_str(SIMPLE_CUBE).unwrap();
        // Cube from (0,0,0) to (1,1,0)
        assert_eq!(mesh.bbox_min, [0.0, 0.0, 0.0]);
        assert_eq!(mesh.bbox_max, [1.0, 1.0, 0.0]);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_dimensions() {
        let mesh = StlParser::parse_str(SIMPLE_CUBE).unwrap();
        let dims = mesh.dimensions();
        assert_eq!(dims, [1.0, 1.0, 0.0]);
    }

    #[test]
    fn test_extract_name_from_header() {
        let name = StlParser::extract_name_from_header("solid my_model\n  facet...");
        assert_eq!(name, Some("my_model".to_string()));

        let name = StlParser::extract_name_from_header("solid \n  facet...");
        assert_eq!(name, None);
    }
}
