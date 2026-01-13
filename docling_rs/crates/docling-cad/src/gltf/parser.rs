//! GLTF/GLB file parser
//!
//! Parses GLTF 2.0 files (both .gltf JSON and .glb binary) using the gltf crate.

use anyhow::{Context, Result};
use std::path::Path;

/// Animation information
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct AnimationInfo {
    /// Animation name (if specified)
    pub name: Option<String>,
    /// Number of channels (animated properties)
    pub channel_count: usize,
    /// Number of samplers (keyframe data)
    pub sampler_count: usize,
}

/// Material information
#[allow(clippy::struct_excessive_bools)] // Bools represent distinct texture/rendering flags
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MaterialInfo {
    /// Material name (if specified)
    pub name: Option<String>,
    /// Base color factor [R, G, B, A]
    pub base_color: Option<[f32; 4]>,
    /// Metallic factor (0.0 = dielectric, 1.0 = metallic)
    pub metallic: Option<f32>,
    /// Roughness factor (0.0 = smooth, 1.0 = rough)
    pub roughness: Option<f32>,
    /// Alpha rendering mode (Opaque, Mask, Blend)
    pub alpha_mode: String,
    /// Whether material is double-sided
    pub double_sided: bool,
    /// Whether material has base color texture
    pub has_base_color_texture: bool,
    /// Whether material has normal map
    pub has_normal_texture: bool,
    /// Whether material has emissive texture
    pub has_emissive_texture: bool,
}

/// GLTF model data
#[derive(Debug, Clone, PartialEq)]
pub struct GltfModel {
    /// Model name (from file path or asset metadata)
    pub name: Option<String>,
    /// Number of meshes
    pub mesh_count: usize,
    /// Total number of primitives (submeshes) across all meshes
    pub primitive_count: usize,
    /// Total vertex count (approximate, summed across all accessors)
    pub vertex_count: usize,
    /// Total triangle count (approximate)
    pub triangle_count: usize,
    /// Number of nodes in scene graph
    pub node_count: usize,
    /// Number of scenes
    pub scene_count: usize,
    /// Number of materials
    pub material_count: usize,
    /// Number of animations
    pub animation_count: usize,
    /// Number of accessors (data access descriptors)
    pub accessor_count: usize,
    /// Number of buffer views (data layout descriptors)
    pub buffer_view_count: usize,
    /// Number of buffers (binary data containers)
    pub buffer_count: usize,
    /// Detailed animation information
    pub animations: Vec<AnimationInfo>,
    /// Detailed material information
    pub materials: Vec<MaterialInfo>,
    /// Names of meshes in the model
    pub mesh_names: Vec<String>,
    /// Names of nodes in the scene graph
    pub node_names: Vec<String>,
    /// Bounding box minimum (across all meshes)
    pub bbox_min: Option<[f32; 3]>,
    /// Bounding box maximum (across all meshes)
    pub bbox_max: Option<[f32; 3]>,
    /// Whether file is binary GLB format
    pub is_binary: bool,
    /// GLTF asset generator (if available)
    pub generator: Option<String>,
    /// GLTF version
    pub version: String,
}

impl GltfModel {
    /// Get mesh dimensions (width, height, depth)
    ///
    /// Returns the dimensions of the mesh's bounding box as `[width, height, depth]`.
    /// Returns `None` if bounding box information is not available.
    ///
    /// # Returns
    ///
    /// Optional array of three floats representing the width (X), height (Y), and depth (Z).
    #[inline]
    #[must_use = "dimensions returns optional width/height/depth array"]
    pub fn dimensions(&self) -> Option<[f32; 3]> {
        match (self.bbox_min, self.bbox_max) {
            (Some(min), Some(max)) => Some([max[0] - min[0], max[1] - min[1], max[2] - min[2]]),
            _ => None,
        }
    }

    /// Get mesh bounding volume
    ///
    /// Returns the volume of the mesh's axis-aligned bounding box.
    /// Returns `None` if bounding box information is not available.
    /// Note: This is NOT the actual mesh volume, just the bounding box volume.
    ///
    /// # Returns
    ///
    /// Optional volume in cubic units (width × height × depth).
    #[inline]
    #[must_use = "bounding_volume returns optional volume in cubic units"]
    pub fn bounding_volume(&self) -> Option<f32> {
        self.dimensions().map(|dims| dims[0] * dims[1] * dims[2])
    }
}

/// GLTF parser
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct GltfParser;

impl GltfParser {
    /// Parse GLTF/GLB file from path
    ///
    /// Reads and parses a GLTF 2.0 file (both .gltf JSON and .glb binary formats).
    /// Automatically detects the file format and extracts mesh data, scene graph,
    /// materials, animations, and metadata.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the GLTF or GLB file
    ///
    /// # Returns
    ///
    /// Parsed GLTF model with mesh data, scene structure, material info, and metadata.
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be opened or contains invalid GLTF data.
    #[must_use = "parsing produces a result that should be handled"]
    pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<GltfModel> {
        let path = path.as_ref();

        // Determine if binary GLB format based on extension
        let is_binary = path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("glb"));

        // Load GLTF document
        let gltf = gltf::Gltf::open(path)
            .with_context(|| format!("Failed to parse GLTF file: {}", path.display()))?;

        let name = Self::extract_name_from_path(path);
        let mesh_count = gltf.meshes().count();
        let node_count = gltf.nodes().count();
        let scene_count = gltf.scenes().count();
        let material_count = gltf.materials().count();
        let animation_count = gltf.animations().count();
        let accessor_count = gltf.accessors().count();
        let buffer_view_count = gltf.views().count();
        let buffer_count = gltf.buffers().count();

        // Count primitives and estimate vertex/triangle counts
        // Also collect mesh names
        let mut primitive_count = 0;
        let mut vertex_count = 0;
        let mut triangle_count = 0;
        let mut mesh_names = Vec::new();

        for (i, mesh) in gltf.meshes().enumerate() {
            // Collect mesh name
            let mesh_name = mesh
                .name()
                .map_or_else(|| format!("Mesh {i}"), String::from);
            mesh_names.push(mesh_name);

            for primitive in mesh.primitives() {
                primitive_count += 1;

                // Get vertex count from POSITION accessor
                if let Some(accessor) = primitive.get(&gltf::Semantic::Positions) {
                    vertex_count += accessor.count();
                }

                // Estimate triangle count
                let index_count = primitive
                    .indices()
                    .map_or(vertex_count, |accessor| accessor.count()); // If no indices, assume indexed by vertex count

                // Most primitives are triangles (mode 4)
                let mode = primitive.mode();
                triangle_count += match mode {
                    gltf::mesh::Mode::Triangles => index_count / 3,
                    gltf::mesh::Mode::TriangleFan | gltf::mesh::Mode::TriangleStrip => {
                        if index_count >= 3 {
                            index_count - 2
                        } else {
                            0
                        }
                    }
                    _ => 0, // Lines, points, etc.
                };
            }
        }

        // Collect node names
        let node_names: Vec<String> = gltf
            .nodes()
            .enumerate()
            .map(|(i, node)| {
                node.name()
                    .map_or_else(|| format!("Node {i}"), String::from)
            })
            .collect();

        // Calculate bounding box across all meshes
        let (bbox_min, bbox_max) = Self::calculate_bounding_box(&gltf);

        // Extract detailed animation information
        let animations = Self::extract_animations(&gltf);

        // Extract detailed material information
        let materials = Self::extract_materials(&gltf);

        // Extract asset metadata from JSON document
        let document = gltf.document;
        let asset_json = &document.as_json().asset;
        let generator = asset_json.generator.clone();
        let version = asset_json.version.clone();

        Ok(GltfModel {
            name,
            mesh_count,
            primitive_count,
            vertex_count,
            triangle_count,
            node_count,
            scene_count,
            material_count,
            animation_count,
            accessor_count,
            buffer_view_count,
            buffer_count,
            animations,
            materials,
            mesh_names,
            node_names,
            bbox_min,
            bbox_max,
            is_binary,
            generator,
            version,
        })
    }

    /// Extract animation information
    fn extract_animations(gltf: &gltf::Gltf) -> Vec<AnimationInfo> {
        gltf.animations()
            .map(|animation| {
                let name = animation.name().map(String::from);
                let channel_count = animation.channels().count();
                let sampler_count = animation.samplers().count();

                AnimationInfo {
                    name,
                    channel_count,
                    sampler_count,
                }
            })
            .collect()
    }

    /// Extract material information
    fn extract_materials(gltf: &gltf::Gltf) -> Vec<MaterialInfo> {
        gltf.materials()
            .map(|material| {
                let name = material.name().map(String::from);

                // Get PBR metallic-roughness properties
                let pbr = material.pbr_metallic_roughness();
                let base_color = Some(pbr.base_color_factor());
                let metallic = Some(pbr.metallic_factor());
                let roughness = Some(pbr.roughness_factor());
                let has_base_color_texture = pbr.base_color_texture().is_some();

                // Get alpha mode
                let alpha_mode = match material.alpha_mode() {
                    gltf::material::AlphaMode::Opaque => "Opaque",
                    gltf::material::AlphaMode::Mask => "Mask",
                    gltf::material::AlphaMode::Blend => "Blend",
                }
                .to_string();

                // Get other properties
                let double_sided = material.double_sided();
                let has_normal_texture = material.normal_texture().is_some();
                let has_emissive_texture = material.emissive_texture().is_some();

                MaterialInfo {
                    name,
                    base_color,
                    metallic,
                    roughness,
                    alpha_mode,
                    double_sided,
                    has_base_color_texture,
                    has_normal_texture,
                    has_emissive_texture,
                }
            })
            .collect()
    }

    /// Extract name from file path
    fn extract_name_from_path(path: &Path) -> Option<String> {
        path.file_stem()
            .and_then(|name| name.to_str())
            .map(String::from)
    }

    /// Calculate bounding box across all mesh primitives
    // JSON numbers are parsed as f64, but 3D graphics uses f32. Precision loss is acceptable.
    #[allow(clippy::cast_possible_truncation)]
    fn calculate_bounding_box(gltf: &gltf::Gltf) -> (Option<[f32; 3]>, Option<[f32; 3]>) {
        let mut global_min: Option<[f32; 3]> = None;
        let mut global_max: Option<[f32; 3]> = None;

        for mesh in gltf.meshes() {
            for primitive in mesh.primitives() {
                if let Some(accessor) = primitive.get(&gltf::Semantic::Positions) {
                    // Parse min value from JSON array
                    if let Some(min_json) = accessor.min() {
                        if let Some(min_array) = min_json.as_array() {
                            if min_array.len() >= 3 {
                                // Convert JSON numbers to f32
                                if let (Some(x), Some(y), Some(z)) = (
                                    min_array[0].as_f64(),
                                    min_array[1].as_f64(),
                                    min_array[2].as_f64(),
                                ) {
                                    let min_vec = [x as f32, y as f32, z as f32];
                                    global_min = Some(global_min.map_or(min_vec, |current| {
                                        [
                                            current[0].min(min_vec[0]),
                                            current[1].min(min_vec[1]),
                                            current[2].min(min_vec[2]),
                                        ]
                                    }));
                                }
                            }
                        }
                    }

                    // Parse max value from JSON array
                    if let Some(max_json) = accessor.max() {
                        if let Some(max_array) = max_json.as_array() {
                            if max_array.len() >= 3 {
                                // Convert JSON numbers to f32
                                if let (Some(x), Some(y), Some(z)) = (
                                    max_array[0].as_f64(),
                                    max_array[1].as_f64(),
                                    max_array[2].as_f64(),
                                ) {
                                    let max_vec = [x as f32, y as f32, z as f32];
                                    global_max = Some(global_max.map_or(max_vec, |current| {
                                        [
                                            current[0].max(max_vec[0]),
                                            current[1].max(max_vec[1]),
                                            current[2].max(max_vec[2]),
                                        ]
                                    }));
                                }
                            }
                        }
                    }
                }
            }
        }

        (global_min, global_max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_triangle() {
        let path = "../../test-corpus/cad/gltf/simple_triangle.gltf";
        let model = GltfParser::parse_file(path).expect("Failed to parse simple_triangle.gltf");

        assert_eq!(model.name, Some("simple_triangle".to_string()));
        assert_eq!(model.mesh_count, 1);
        assert_eq!(model.primitive_count, 1);
        assert_eq!(model.vertex_count, 3);
        assert_eq!(model.triangle_count, 1);
        assert!(!model.is_binary);
        // Animations and materials may be empty or present depending on test file
        assert_eq!(model.animations.len(), model.animation_count);
        assert_eq!(model.materials.len(), model.material_count);
    }

    #[test]
    fn test_parse_simple_cube() {
        let path = "../../test-corpus/cad/gltf/simple_cube.gltf";
        let model = GltfParser::parse_file(path).expect("Failed to parse simple_cube.gltf");

        assert_eq!(model.name, Some("simple_cube".to_string()));
        assert_eq!(model.mesh_count, 1);
        assert_eq!(model.vertex_count, 8);
        assert_eq!(model.triangle_count, 12); // 6 faces * 2 triangles
        assert!(!model.is_binary);
        assert_eq!(model.animations.len(), model.animation_count);
        assert_eq!(model.materials.len(), model.material_count);
    }

    #[test]
    fn test_parse_box() {
        let path = "../../test-corpus/cad/gltf/box.gltf";
        let model = GltfParser::parse_file(path).expect("Failed to parse box.gltf");

        assert_eq!(model.name, Some("box".to_string()));
        assert!(model.mesh_count > 0);
        assert!(model.vertex_count > 0);
        assert!(!model.is_binary);
        assert_eq!(model.animations.len(), model.animation_count);
        assert_eq!(model.materials.len(), model.material_count);
    }

    #[test]
    fn test_parse_glb_binary() {
        let path = "../../test-corpus/cad/gltf/box.glb";
        let model = GltfParser::parse_file(path).expect("Failed to parse box.glb");

        assert_eq!(model.name, Some("box".to_string()));
        assert!(model.mesh_count > 0);
        assert!(model.is_binary);
        assert_eq!(model.animations.len(), model.animation_count);
        assert_eq!(model.materials.len(), model.material_count);
    }

    #[test]
    fn test_bounding_box() {
        let path = "../../test-corpus/cad/gltf/simple_cube.gltf";
        let model = GltfParser::parse_file(path).expect("Failed to parse simple_cube.gltf");

        assert!(model.bbox_min.is_some());
        assert!(model.bbox_max.is_some());

        if let (Some(min), Some(max)) = (model.bbox_min, model.bbox_max) {
            // Cube should have dimensions roughly 1x1x1
            let dims = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
            assert!(dims[0] > 0.9 && dims[0] < 1.1);
            assert!(dims[1] > 0.9 && dims[1] < 1.1);
            assert!(dims[2] > 0.9 && dims[2] < 1.1);
        }
    }

    #[test]
    fn test_dimensions() {
        let path = "../../test-corpus/cad/gltf/simple_cube.gltf";
        let model = GltfParser::parse_file(path).expect("Failed to parse simple_cube.gltf");

        let dims = model.dimensions().expect("Should have dimensions");
        assert!(dims[0] > 0.0);
        assert!(dims[1] > 0.0);
        assert!(dims[2] > 0.0);
    }

    #[test]
    fn test_bounding_volume() {
        let path = "../../test-corpus/cad/gltf/simple_cube.gltf";
        let model = GltfParser::parse_file(path).expect("Failed to parse simple_cube.gltf");

        let volume = model.bounding_volume().expect("Should have volume");
        assert!(volume > 0.0);
    }

    #[test]
    fn test_parse_nonexistent_file() {
        let result = GltfParser::parse_file("nonexistent.gltf");
        assert!(result.is_err());
    }
}
