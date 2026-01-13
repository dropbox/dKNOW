//! GLTF/GLB to Markdown serialization
//!
//! Converts parsed GLTF model data into a human-readable markdown format
//! suitable for document processing.

use std::fmt::Write;

use super::{GltfModel, MaterialInfo};

/// Get format type string for model
#[inline]
const fn format_type(model: &GltfModel) -> &'static str {
    if model.is_binary {
        "GLB (Binary glTF 2.0)"
    } else {
        "glTF 2.0 (JSON)"
    }
}

/// Format asset information section
fn format_asset_info(output: &mut String, model: &GltfModel) {
    let format_str = format_type(model);
    output.push_str("## Asset Information\n\n");
    let _ = writeln!(output, "- Format: {format_str}");
    let _ = writeln!(output, "- glTF Version: {}", model.version);
    if let Some(ref generator) = model.generator {
        let _ = writeln!(output, "- Generator: {generator}");
    }
    output.push('\n');
}

/// Format geometry statistics section
fn format_geometry_stats(output: &mut String, model: &GltfModel) {
    output.push_str("## Geometry Statistics\n\n");
    let _ = writeln!(output, "- Meshes: {}", model.mesh_count);
    if !model.mesh_names.is_empty() {
        output.push_str("- Mesh Names:\n");
        for mesh_name in &model.mesh_names {
            let _ = writeln!(output, "  - {mesh_name}");
        }
    }
    let _ = writeln!(output, "- Primitives: {}", model.primitive_count);
    let _ = writeln!(output, "- Total Vertices: {}", model.vertex_count);
    let _ = writeln!(output, "- Total Triangles: {}", model.triangle_count);
    output.push('\n');
}

/// Format data structure section
fn format_data_structure(output: &mut String, model: &GltfModel) {
    output.push_str("## Data Structure\n\n");
    let _ = writeln!(output, "- Accessors: {}", model.accessor_count);
    let _ = writeln!(output, "- Buffer Views: {}", model.buffer_view_count);
    let _ = writeln!(output, "- Buffers: {}", model.buffer_count);
    output.push('\n');
}

/// Format scene graph section
fn format_scene_graph(output: &mut String, model: &GltfModel) {
    output.push_str("## Scene Graph\n\n");
    let _ = writeln!(output, "- Scenes: {}", model.scene_count);
    let _ = writeln!(output, "- Nodes: {}", model.node_count);
    if !model.node_names.is_empty() {
        output.push_str("- Node Names:\n");
        for node_name in &model.node_names {
            let _ = writeln!(output, "  - {node_name}");
        }
    }
    output.push('\n');
}

/// Format a single material
fn format_material(output: &mut String, index: usize, material: &MaterialInfo) {
    let default_name = format!("Material {index}");
    let mat_name = material.name.as_deref().unwrap_or(&default_name);
    let _ = writeln!(output, "### {mat_name}\n");

    if let Some(base_color) = material.base_color {
        let _ = writeln!(
            output,
            "- Base Color: ({:.3}, {:.3}, {:.3}, {:.3})",
            base_color[0], base_color[1], base_color[2], base_color[3]
        );
    }
    if let Some(metallic) = material.metallic {
        let _ = writeln!(output, "- Metallic: {metallic:.3}");
    }
    if let Some(roughness) = material.roughness {
        let _ = writeln!(output, "- Roughness: {roughness:.3}");
    }

    let _ = writeln!(output, "- Alpha Mode: {}", material.alpha_mode);
    let _ = writeln!(
        output,
        "- Double Sided: {}",
        if material.double_sided { "Yes" } else { "No" }
    );

    let mut textures = Vec::new();
    if material.has_base_color_texture {
        textures.push("Base Color");
    }
    if material.has_normal_texture {
        textures.push("Normal");
    }
    if material.has_emissive_texture {
        textures.push("Emissive");
    }
    if !textures.is_empty() {
        let _ = writeln!(output, "- Textures: {}", textures.join(", "));
    }
    output.push('\n');
}

/// Format materials section
fn format_materials(output: &mut String, model: &GltfModel) {
    if model.materials.is_empty() {
        return;
    }
    output.push_str("## Materials\n\n");
    let _ = writeln!(output, "- Count: {}", model.material_count);
    output.push('\n');
    for (i, material) in model.materials.iter().enumerate() {
        format_material(output, i, material);
    }
}

/// Format animations section
fn format_animations(output: &mut String, model: &GltfModel) {
    if model.animations.is_empty() {
        return;
    }
    output.push_str("## Animations\n\n");
    let _ = writeln!(output, "- Count: {}", model.animation_count);
    output.push('\n');
    for (i, animation) in model.animations.iter().enumerate() {
        let default_name = format!("Animation {i}");
        let anim_name = animation.name.as_deref().unwrap_or(&default_name);
        let _ = writeln!(output, "### {anim_name}\n");
        let _ = writeln!(output, "- Channels: {}", animation.channel_count);
        let _ = writeln!(output, "- Samplers: {}", animation.sampler_count);
        output.push('\n');
    }
}

/// Format bounding box section
fn format_bounding_box(output: &mut String, model: &GltfModel) {
    let (Some([min_x, min_y, min_z]), Some([max_x, max_y, max_z])) =
        (model.bbox_min, model.bbox_max)
    else {
        return;
    };
    output.push_str("## Bounding Box\n\n");
    let _ = writeln!(output, "- Minimum: ({min_x:.3}, {min_y:.3}, {min_z:.3})");
    let _ = writeln!(output, "- Maximum: ({max_x:.3}, {max_y:.3}, {max_z:.3})");
    if let Some([dim_x, dim_y, dim_z]) = model.dimensions() {
        let _ = writeln!(
            output,
            "- Dimensions (X, Y, Z): {dim_x:.3}, {dim_y:.3}, {dim_z:.3}"
        );
    }
    if let Some(volume) = model.bounding_volume() {
        let _ = writeln!(output, "- Bounding Volume: {volume:.3} cubic units");
    }
    output.push('\n');
}

/// Format summary section
fn format_summary(output: &mut String, model: &GltfModel) {
    let format_str = format_type(model);
    output.push_str("## Summary\n\n");
    let mesh_plural = if model.mesh_count == 1 { "" } else { "es" };
    let vertex_word = if model.vertex_count == 1 {
        "vertex"
    } else {
        "vertices"
    };
    let triangle_word = if model.triangle_count == 1 {
        "triangle"
    } else {
        "triangles"
    };
    let _ = write!(
        output,
        "This {} model contains {} mesh{} with a total of {} {} and {} {}",
        format_str,
        model.mesh_count,
        mesh_plural,
        model.vertex_count,
        vertex_word,
        model.triangle_count,
        triangle_word
    );
    if model.animation_count > 0 {
        let anim_plural = if model.animation_count == 1 { "" } else { "s" };
        let _ = write!(
            output,
            ", and includes {} animation{}",
            model.animation_count, anim_plural
        );
    }
    output.push_str(".\n");
}

/// Convert GLTF model to markdown format
///
/// Generates a markdown document containing:
/// - Model name and format
/// - Geometry statistics (meshes, primitives, vertices, triangles)
/// - Scene graph information (nodes, scenes)
/// - Material and animation counts
/// - Bounding box and dimensions
/// - Asset metadata (generator, version)
///
/// # Arguments
/// * `model` - Parsed GLTF model data
///
/// # Returns
/// Markdown-formatted string
#[must_use = "serialization returns markdown string"]
pub fn to_markdown(model: &GltfModel) -> String {
    let mut output = String::new();

    // Title
    let name = model.name.as_deref().unwrap_or("Unnamed Model");
    let _ = writeln!(output, "# 3D Model: {name}");

    format_asset_info(&mut output, model);
    format_geometry_stats(&mut output, model);
    format_data_structure(&mut output, model);
    format_scene_graph(&mut output, model);
    format_materials(&mut output, model);
    format_animations(&mut output, model);
    format_bounding_box(&mut output, model);
    format_summary(&mut output, model);

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gltf::{AnimationInfo, GltfModel, MaterialInfo};

    #[test]
    fn test_to_markdown_basic() {
        let model = GltfModel {
            name: Some("test_cube".to_string()),
            mesh_count: 1,
            primitive_count: 1,
            vertex_count: 8,
            triangle_count: 12,
            node_count: 1,
            scene_count: 1,
            material_count: 1,
            animation_count: 0,
            accessor_count: 2,
            buffer_view_count: 2,
            buffer_count: 2,
            animations: vec![],
            materials: vec![MaterialInfo {
                name: Some("Default Material".to_string()),
                base_color: Some([0.8, 0.8, 0.8, 1.0]),
                metallic: Some(0.5),
                roughness: Some(0.5),
                alpha_mode: "Opaque".to_string(),
                double_sided: false,
                has_base_color_texture: false,
                has_normal_texture: false,
                has_emissive_texture: false,
            }],
            mesh_names: vec!["Cube".to_string()],
            node_names: vec!["RootNode".to_string()],
            bbox_min: Some([-0.5, -0.5, -0.5]),
            bbox_max: Some([0.5, 0.5, 0.5]),
            is_binary: false,
            generator: Some("Test Generator".to_string()),
            version: "2.0".to_string(),
        };

        let markdown = to_markdown(&model);

        assert!(markdown.contains("# 3D Model: test_cube"));
        assert!(markdown.contains("## Asset Information"));
        assert!(markdown.contains("- Format: glTF 2.0 (JSON)"));
        assert!(markdown.contains("Meshes: 1"));
        assert!(markdown.contains("Total Vertices: 8"));
        assert!(markdown.contains("Total Triangles: 12"));
        assert!(markdown.contains("Generator: Test Generator"));
        assert!(markdown.contains("Scenes: 1"));
        assert!(markdown.contains("Nodes: 1"));
        assert!(markdown.contains("## Materials"));
        assert!(markdown.contains("### Default Material"));
        assert!(markdown.contains("- Base Color:"));
    }

    #[test]
    fn test_to_markdown_binary() {
        let model = GltfModel {
            name: Some("binary_model".to_string()),
            mesh_count: 2,
            primitive_count: 3,
            vertex_count: 100,
            triangle_count: 50,
            node_count: 5,
            scene_count: 1,
            material_count: 2,
            animation_count: 1,
            accessor_count: 4,
            buffer_view_count: 3,
            buffer_count: 1,
            animations: vec![AnimationInfo {
                name: Some("Rotate".to_string()),
                channel_count: 3,
                sampler_count: 3,
            }],
            materials: vec![
                MaterialInfo {
                    name: Some("Material 1".to_string()),
                    base_color: Some([1.0, 0.0, 0.0, 1.0]),
                    metallic: Some(0.0),
                    roughness: Some(0.8),
                    alpha_mode: "Opaque".to_string(),
                    double_sided: false,
                    has_base_color_texture: true,
                    has_normal_texture: false,
                    has_emissive_texture: false,
                },
                MaterialInfo {
                    name: Some("Material 2".to_string()),
                    base_color: Some([0.0, 1.0, 0.0, 1.0]),
                    metallic: Some(1.0),
                    roughness: Some(0.2),
                    alpha_mode: "Opaque".to_string(),
                    double_sided: true,
                    has_base_color_texture: false,
                    has_normal_texture: true,
                    has_emissive_texture: false,
                },
            ],
            mesh_names: vec!["Mesh 0".to_string(), "Mesh 1".to_string()],
            node_names: vec![
                "Node 0".to_string(),
                "Node 1".to_string(),
                "Node 2".to_string(),
                "Node 3".to_string(),
                "Node 4".to_string(),
            ],
            bbox_min: None,
            bbox_max: None,
            is_binary: true,
            generator: None,
            version: "2.0".to_string(),
        };

        let markdown = to_markdown(&model);

        assert!(markdown.contains("## Asset Information"));
        assert!(markdown.contains("- Format: GLB (Binary glTF 2.0)"));
        assert!(markdown.contains("Meshes: 2"));
        assert!(markdown.contains("## Animations"));
        assert!(markdown.contains("### Rotate"));
        assert!(!markdown.contains("## Bounding Box"));
    }

    #[test]
    fn test_to_markdown_with_animations() {
        let model = GltfModel {
            name: Some("animated_model".to_string()),
            mesh_count: 1,
            primitive_count: 1,
            vertex_count: 50,
            triangle_count: 25,
            node_count: 10,
            scene_count: 1,
            material_count: 1,
            animation_count: 3,
            accessor_count: 5,
            buffer_view_count: 3,
            buffer_count: 2,
            animations: vec![
                AnimationInfo {
                    name: Some("Walk".to_string()),
                    channel_count: 4,
                    sampler_count: 4,
                },
                AnimationInfo {
                    name: Some("Run".to_string()),
                    channel_count: 4,
                    sampler_count: 4,
                },
                AnimationInfo {
                    name: None,
                    channel_count: 2,
                    sampler_count: 2,
                },
            ],
            materials: vec![MaterialInfo {
                name: None,
                base_color: Some([0.5, 0.5, 0.5, 1.0]),
                metallic: Some(0.0),
                roughness: Some(1.0),
                alpha_mode: "Opaque".to_string(),
                double_sided: false,
                has_base_color_texture: false,
                has_normal_texture: false,
                has_emissive_texture: false,
            }],
            mesh_names: vec!["Character".to_string()],
            node_names: (0..10).map(|i| format!("Node {i}")).collect(),
            bbox_min: Some([0.0, 0.0, 0.0]),
            bbox_max: Some([1.0, 1.0, 1.0]),
            is_binary: false,
            generator: None,
            version: "2.0".to_string(),
        };

        let markdown = to_markdown(&model);

        assert!(markdown.contains("- Count: 3"));
        assert!(markdown.contains("### Walk"));
        assert!(markdown.contains("### Run"));
        assert!(markdown.contains("includes 3 animations"));
    }

    #[test]
    fn test_to_markdown_no_name() {
        let model = GltfModel {
            name: None,
            mesh_count: 1,
            primitive_count: 1,
            vertex_count: 10,
            triangle_count: 5,
            node_count: 1,
            scene_count: 1,
            material_count: 0,
            animation_count: 0,
            accessor_count: 1,
            buffer_view_count: 1,
            buffer_count: 1,
            animations: vec![],
            materials: vec![],
            mesh_names: vec!["Mesh 0".to_string()],
            node_names: vec!["Node 0".to_string()],
            bbox_min: None,
            bbox_max: None,
            is_binary: false,
            generator: None,
            version: "2.0".to_string(),
        };

        let markdown = to_markdown(&model);

        assert!(markdown.contains("# 3D Model: Unnamed Model"));
    }
}
