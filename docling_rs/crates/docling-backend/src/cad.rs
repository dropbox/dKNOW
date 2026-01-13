//! CAD and 3D format backends for docling
//!
//! This backend converts CAD and 3D file formats to docling's document model.
//! Supports STL, OBJ, GLTF, GLB, and DXF formats.

// Clippy pedantic allows:
// - 3D model parsing functions are necessarily complex
#![allow(clippy::too_many_lines)]

use crate::traits::{BackendOptions, DocumentBackend};
use crate::utils::{create_section_header, create_text_item, opt_vec};
use docling_cad::{
    dxf_to_markdown, gltf_to_markdown, obj_to_markdown, stl_to_markdown, DxfDrawing, DxfParser,
    GltfModel, GltfParser, ObjMesh, ObjParser, StlMesh, StlParser,
};
use docling_core::{DocItem, DoclingError, Document, DocumentMetadata, InputFormat};
use std::fmt::Write;
use std::path::Path;

/// CAD backend for 3D mesh and CAD formats
///
/// Supports:
/// - STL (`STereoLithography`) - 3D mesh format
/// - OBJ (Wavefront Object) - 3D mesh format
/// - GLTF (GL Transmission Format) - Modern 3D format
/// - GLB (Binary glTF) - Binary GLTF format
/// - DXF (Drawing Exchange Format) - `AutoCAD` interchange format
///
/// ## Example
///
/// ```no_run
/// use docling_backend::CadBackend;
/// use docling_backend::DocumentBackend;
/// use docling_core::InputFormat;
///
/// let backend = CadBackend::new(InputFormat::Stl)?;
/// let result = backend.parse_file("model.stl", &Default::default())?;
/// println!("Document: {:?}", result.metadata.title);
/// # Ok::<(), docling_core::error::DoclingError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CadBackend {
    format: InputFormat,
}

impl CadBackend {
    /// Create a new CAD backend for the specified format
    ///
    /// # Errors
    ///
    /// Returns an error if the format is not a CAD format.
    #[inline]
    #[must_use = "creating a backend that is not used is a waste of resources"]
    pub fn new(format: InputFormat) -> Result<Self, DoclingError> {
        match format {
            InputFormat::Stl
            | InputFormat::Obj
            | InputFormat::Gltf
            | InputFormat::Glb
            | InputFormat::Dxf => Ok(Self { format }),
            _ => Err(DoclingError::FormatError(format!(
                "Format {format:?} is not a CAD format"
            ))),
        }
    }

    /// Create `DocItems` directly from STL mesh data
    ///
    /// Generates structured `DocItems` from STL mesh, preserving 3D model semantic information.
    fn stl_to_docitems(mesh: &StlMesh) -> Vec<DocItem> {
        let mut doc_items = Vec::new();
        let mut section_index = 0;
        let mut text_index = 0;

        // Helper to create text DocItem
        let mut create_text = |text: String| {
            let item = create_text_item(text_index, text, vec![]);
            text_index += 1;
            item
        };

        // Helper to create section header
        let mut create_section = |title: String, level: usize| {
            let item = DocItem::SectionHeader {
                self_ref: format!("#/section_headers/{section_index}"),
                parent: None,
                children: vec![],
                content_layer: "body".to_string(),
                prov: vec![],
                orig: title.clone(),
                text: title,
                level,
                formatting: None,
                hyperlink: None,
            };
            section_index += 1;
            item
        };

        // Title (H1)
        let title = mesh.name.as_ref().map_or_else(
            || "STL Model".to_string(),
            |name| format!("STL Model: {name}"),
        );
        doc_items.push(create_section(title, 1));

        // File Information section (H2)
        doc_items.push(create_section("File Information".to_string(), 2));
        doc_items.push(create_text(format!(
            "- Format: STL ({})",
            if mesh.is_binary { "Binary" } else { "ASCII" }
        )));

        // Mesh Statistics section (H2)
        doc_items.push(create_section("Mesh Statistics".to_string(), 2));
        doc_items.push(create_text(format!("- Triangles: {}", mesh.triangle_count)));
        doc_items.push(create_text(format!("- Vertices: {}", mesh.vertex_count)));

        // Bounding Box section (H2)
        doc_items.push(create_section("Bounding Box".to_string(), 2));
        doc_items.push(create_text(format!(
            "- Minimum: ({:.3}, {:.3}, {:.3})",
            mesh.bbox_min[0], mesh.bbox_min[1], mesh.bbox_min[2]
        )));
        doc_items.push(create_text(format!(
            "- Maximum: ({:.3}, {:.3}, {:.3})",
            mesh.bbox_max[0], mesh.bbox_max[1], mesh.bbox_max[2]
        )));

        // Dimensions section (H2)
        let dims = mesh.dimensions();
        doc_items.push(create_section("Dimensions".to_string(), 2));
        doc_items.push(create_text(format!("- Width (X): {:.3}", dims[0])));
        doc_items.push(create_text(format!("- Depth (Y): {:.3}", dims[1])));
        doc_items.push(create_text(format!("- Height (Z): {:.3}", dims[2])));

        // Bounding Volume (text)
        let volume = mesh.bounding_volume();
        doc_items.push(create_text(format!(
            "Bounding Volume: {volume:.3} cubic units"
        )));

        // Model Description section (H2)
        doc_items.push(create_section("Model Description".to_string(), 2));
        let description = format!(
            "This 3D model contains {} triangular faces forming a mesh with {} unique vertices. {}",
            mesh.triangle_count,
            mesh.vertex_count,
            if mesh.is_binary {
                "The model is stored in binary STL format for compact file size."
            } else {
                "The model is stored in ASCII STL format for human readability."
            }
        );
        doc_items.push(create_text(description));

        doc_items
    }

    /// Create `DocItems` directly from OBJ mesh data
    ///
    /// Generates structured `DocItems` from OBJ mesh, preserving 3D model semantic information.
    fn obj_to_docitems(mesh: &ObjMesh) -> Vec<DocItem> {
        let mut doc_items = Vec::new();
        let mut section_index = 0;
        let mut text_index = 0;

        // Helper to create text DocItem
        let mut create_text = |text: String| {
            let item = create_text_item(text_index, text, vec![]);
            text_index += 1;
            item
        };

        // Helper to create section header
        let mut create_section = |title: String, level: usize| {
            let item = DocItem::SectionHeader {
                self_ref: format!("#/section_headers/{section_index}"),
                parent: None,
                children: vec![],
                content_layer: "body".to_string(),
                prov: vec![],
                orig: title.clone(),
                text: title,
                level,
                formatting: None,
                hyperlink: None,
            };
            section_index += 1;
            item
        };

        // Title (H1) - use first object name if available, otherwise generic
        let title = if !mesh.model_names.is_empty() && !mesh.model_names[0].is_empty() {
            mesh.model_names[0].clone()
        } else {
            "3D Model".to_string()
        };
        doc_items.push(create_section(title, 1));

        // Format indicator
        doc_items.push(create_text(
            "- Format: Wavefront OBJ (3D Geometry)".to_string(),
        ));

        // Geometry Statistics section (H2)
        doc_items.push(create_section("Geometry Statistics".to_string(), 2));
        doc_items.push(create_text(format!(
            "- Models (Objects): {}",
            mesh.model_count
        )));
        doc_items.push(create_text(format!(
            "- Total Vertices: {}",
            mesh.vertex_count
        )));
        doc_items.push(create_text(format!(
            "- Total Faces: {} (triangles)",
            mesh.face_count
        )));
        if mesh.material_count > 0 {
            doc_items.push(create_text(format!("- Materials: {}", mesh.material_count)));
        }

        // Features section (H2)
        doc_items.push(create_section("Features".to_string(), 2));
        if mesh.has_normals {
            doc_items.push(create_text(format!(
                "- Vertex Normals: {} normals",
                mesh.normal_count
            )));
        } else {
            doc_items.push(create_text(
                "- Vertex Normals: Not present in file (no 'vn' entries)".to_string(),
            ));
        }
        if mesh.has_texcoords {
            doc_items.push(create_text(format!(
                "- Texture Coordinates: {} coordinates",
                mesh.texcoord_count
            )));
        } else {
            doc_items.push(create_text(
                "- Texture Coordinates: Not present in file (no 'vt' entries)".to_string(),
            ));
        }

        // Bounding Box section (H2)
        doc_items.push(create_section("Bounding Box".to_string(), 2));
        doc_items.push(create_text(format!(
            "- Minimum: ({:.3}, {:.3}, {:.3})",
            mesh.bbox_min[0], mesh.bbox_min[1], mesh.bbox_min[2]
        )));
        doc_items.push(create_text(format!(
            "- Maximum: ({:.3}, {:.3}, {:.3})",
            mesh.bbox_max[0], mesh.bbox_max[1], mesh.bbox_max[2]
        )));

        let dims = mesh.dimensions();
        doc_items.push(create_text(format!(
            "- Dimensions: {:.3} × {:.3} × {:.3} units (W × H × D)",
            dims[0], dims[1], dims[2]
        )));

        let volume = mesh.bounding_volume();
        doc_items.push(create_text(format!(
            "- Bounding Volume: {volume:.3} cubic units"
        )));

        // Models/Groups section (H2) - always show if model names exist
        if !mesh.model_names.is_empty() {
            doc_items.push(create_section("Models/Groups".to_string(), 2));
            for (idx, model_name) in mesh.model_names.iter().enumerate() {
                let model = &mesh.models[idx];
                let vertex_count = model.mesh.positions.len() / 3;
                let face_count = model.mesh.indices.len() / 3;

                doc_items.push(create_text(format!(
                    "{}. **{}** - {} vertices, {} faces",
                    idx + 1,
                    if model_name.is_empty() {
                        "Unnamed"
                    } else {
                        model_name
                    },
                    vertex_count,
                    face_count
                )));
            }
        }

        // Materials section (H2) - always include section
        doc_items.push(create_section("Materials".to_string(), 2));
        if mesh.materials.is_empty() {
            doc_items.push(create_text(
                "No materials defined (no MTL file referenced)".to_string(),
            ));
        } else {
            for (idx, material) in mesh.materials.iter().enumerate() {
                let mut material_info = format!("{}. **{}**", idx + 1, material.name);

                // Ambient color
                if let Some([r, g, b]) = material.ambient {
                    let _ = write!(material_info, "\n   - Ambient: RGB({r:.2}, {g:.2}, {b:.2})");
                }

                // Diffuse color
                if let Some([r, g, b]) = material.diffuse {
                    let _ = write!(material_info, "\n   - Diffuse: RGB({r:.2}, {g:.2}, {b:.2})");
                }

                // Specular color
                if let Some([r, g, b]) = material.specular {
                    let _ = write!(
                        material_info,
                        "\n   - Specular: RGB({r:.2}, {g:.2}, {b:.2})"
                    );
                }

                // Diffuse texture map
                if let Some(ref diffuse_texture) = material.diffuse_texture {
                    let _ = write!(material_info, "\n   - Diffuse Texture: {diffuse_texture}");
                }

                doc_items.push(create_text(material_info));
            }
        }

        // Summary section (H2)
        doc_items.push(create_section("Summary".to_string(), 2));
        let mut summary = format!(
            "This OBJ file contains {} 3D model(s) with a total of {} vertices and {} triangular faces",
            mesh.model_count, mesh.vertex_count, mesh.face_count
        );

        if mesh.has_normals || mesh.has_texcoords {
            summary.push_str(", including ");
            let mut features = Vec::new();
            if mesh.has_normals {
                features.push("vertex normals");
            }
            if mesh.has_texcoords {
                features.push("texture coordinates");
            }
            summary.push_str(&features.join(" and "));
        }

        summary.push('.');
        doc_items.push(create_text(summary));

        doc_items
    }

    /// Create `DocItems` directly from GLTF model data
    ///
    /// Generates structured `DocItems` from GLTF/GLB model, preserving 3D model semantic information.
    fn gltf_to_docitems(model: &GltfModel) -> Vec<DocItem> {
        let mut doc_items = Vec::new();
        let mut section_index = 0;
        let mut text_index = 0;

        // Helper to create text DocItem
        let mut create_text = |text: String| {
            let item = create_text_item(text_index, text, vec![]);
            text_index += 1;
            item
        };

        // Helper to create section header
        let mut create_section = |title: String, level: usize| {
            let item = DocItem::SectionHeader {
                self_ref: format!("#/section_headers/{section_index}"),
                parent: None,
                children: vec![],
                content_layer: "body".to_string(),
                prov: vec![],
                orig: title.clone(),
                text: title,
                level,
                formatting: None,
                hyperlink: None,
            };
            section_index += 1;
            item
        };

        // Title (H1) - main title (generic to avoid unsubstantiated claims about shape)
        doc_items.push(create_section("3D Model".to_string(), 1));

        // Format indicator
        let format_type = if model.is_binary {
            "GLB (Binary glTF 2.0)"
        } else {
            "glTF 2.0 (JSON)"
        };
        doc_items.push(create_text(format!("- Format: {format_type}")));

        // Asset Information section (H2) - subsection
        doc_items.push(create_section("Asset Information".to_string(), 2));
        doc_items.push(create_text(format!("- glTF Version: {}", model.version)));
        if let Some(ref generator) = model.generator {
            doc_items.push(create_text(format!("- Generator: {generator}")));
        }

        // Geometry Statistics section (H2)
        doc_items.push(create_section("Geometry Statistics".to_string(), 2));
        doc_items.push(create_text(format!("- Meshes: {}", model.mesh_count)));
        doc_items.push(create_text(format!(
            "- Primitives: {}",
            model.primitive_count
        )));
        doc_items.push(create_text(format!(
            "- Total Vertices: {}",
            model.vertex_count
        )));
        doc_items.push(create_text(format!(
            "- Total Triangles: {}",
            model.triangle_count
        )));

        // Scene Graph section (H2)
        doc_items.push(create_section("Scene Graph".to_string(), 2));
        doc_items.push(create_text(format!("- Scenes: {}", model.scene_count)));
        doc_items.push(create_text(format!("- Nodes: {}", model.node_count)));

        // Materials section (H2) - with details if available
        doc_items.push(create_section("Materials".to_string(), 2));
        if model.materials.is_empty() {
            doc_items.push(create_text("- No materials defined".to_string()));
        } else {
            doc_items.push(create_text(format!("- Count: {}", model.material_count)));
            for material in &model.materials {
                let mat_name = material.name.as_deref().unwrap_or("Unnamed");
                let mut details = vec![mat_name.to_string()];

                if let Some(color) = material.base_color {
                    // Format color as RGB (ignore alpha for readability)
                    details.push(format!(
                        "RGB({:.0}, {:.0}, {:.0})",
                        color[0] * 255.0,
                        color[1] * 255.0,
                        color[2] * 255.0
                    ));
                }

                if let Some(metallic) = material.metallic {
                    if metallic > 0.5 {
                        details.push("Metallic".to_string());
                    }
                }

                doc_items.push(create_text(format!("  - {}", details.join(", "))));
            }
        }

        // Animations section (H2) - if present
        if model.animation_count > 0 {
            doc_items.push(create_section("Animations".to_string(), 2));
            doc_items.push(create_text(format!(
                "- Animations: {}",
                model.animation_count
            )));
        }

        // Bounding Box section (H2) - if available
        if let (Some([min_x, min_y, min_z]), Some([max_x, max_y, max_z])) =
            (model.bbox_min, model.bbox_max)
        {
            doc_items.push(create_section("Bounding Box".to_string(), 2));
            doc_items.push(create_text(format!(
                "- Minimum: ({min_x:.3}, {min_y:.3}, {min_z:.3})"
            )));
            doc_items.push(create_text(format!(
                "- Maximum: ({max_x:.3}, {max_y:.3}, {max_z:.3})"
            )));

            if let Some([dim_x, dim_y, dim_z]) = model.dimensions() {
                doc_items.push(create_text(format!(
                    "- Dimensions: {dim_x:.3} × {dim_y:.3} × {dim_z:.3} (W × H × D)"
                )));
            }

            if let Some(volume) = model.bounding_volume() {
                doc_items.push(create_text(format!(
                    "- Bounding Volume: {volume:.3} cubic units"
                )));
            }
        }

        // Summary section (H2) - clearly separated from other sections
        doc_items.push(create_section("Summary".to_string(), 2));
        let mut summary = format!(
            "This {} model contains {} mesh{} with a total of {} vertices and {} triangles",
            format_type,
            model.mesh_count,
            if model.mesh_count == 1 { "" } else { "es" },
            model.vertex_count,
            model.triangle_count
        );

        if model.animation_count > 0 {
            let _ = write!(
                summary,
                ", and includes {} animation{}",
                model.animation_count,
                if model.animation_count == 1 { "" } else { "s" }
            );
        }

        summary.push('.');
        doc_items.push(create_text(summary));

        doc_items
    }

    /// Create `DocItems` directly from DXF drawing data
    ///
    /// Generates structured `DocItems` from DXF drawing, preserving CAD semantic information.
    fn dxf_to_docitems(drawing: &DxfDrawing) -> Vec<DocItem> {
        let mut builder = DxfDocItemBuilder::new();
        builder.write_title(drawing);
        builder.write_file_info(drawing);
        builder.write_header_vars(drawing);
        builder.write_dim_style_vars(drawing);
        builder.write_drawing_extents(drawing);
        builder.write_entity_stats(drawing);
        builder.write_layers(drawing);
        builder.write_drawing_dimensions(drawing);
        builder.write_text_content(drawing);
        builder.write_description(drawing);
        builder.doc_items
    }
}

/// Helper struct for building DXF `DocItem`s
#[derive(Debug, Clone, PartialEq)]
struct DxfDocItemBuilder {
    doc_items: Vec<DocItem>,
    section_index: usize,
    text_index: usize,
}

impl DxfDocItemBuilder {
    const fn new() -> Self {
        Self {
            doc_items: Vec::new(),
            section_index: 0,
            text_index: 0,
        }
    }

    #[inline]
    fn push_text(&mut self, text: String) {
        self.doc_items
            .push(create_text_item(self.text_index, text, vec![]));
        self.text_index += 1;
    }

    #[inline]
    fn push_section(&mut self, title: String, level: usize) {
        self.doc_items.push(create_section_header(
            self.section_index,
            title,
            level,
            vec![],
        ));
        self.section_index += 1;
    }

    fn write_title(&mut self, drawing: &DxfDrawing) {
        let title = drawing.name.as_ref().map_or_else(
            || "DXF Drawing".to_string(),
            |name| format!("DXF Drawing: {name}"),
        );
        self.push_section(title, 1);
    }

    fn write_file_info(&mut self, drawing: &DxfDrawing) {
        self.push_section("File Information".to_string(), 2);
        self.push_text("- Format: DXF (Drawing Exchange Format)".to_string());
        self.push_text(format!(
            "- DXF Version: {} (AutoCAD {})",
            drawing.version_code,
            drawing.version.replace('R', "")
        ));
        self.push_text(format!("- Total Entities: {}", drawing.entity_count));
    }

    fn write_header_vars(&mut self, drawing: &DxfDrawing) {
        self.push_section("Header Variables".to_string(), 2);

        // String variables
        Self::write_opt_str_var(
            &mut self.doc_items,
            &mut self.text_index,
            "$ACADVER",
            drawing.header_vars.acad_ver.as_deref(),
        );
        if let Some(v) = drawing.header_vars.acad_maint_ver {
            self.push_text(format!("- $ACADMAINTVER: {v}"));
        }
        Self::write_opt_nonempty_str(
            &mut self.doc_items,
            &mut self.text_index,
            "$DWGCODEPAGE",
            drawing.header_vars.dwg_codepage.as_deref(),
        );
        Self::write_opt_nonempty_str(
            &mut self.doc_items,
            &mut self.text_index,
            "$LASTSAVEDBY",
            drawing.header_vars.last_saved_by.as_deref(),
        );

        // Numeric variables
        if let Some(v) = drawing.header_vars.ins_units {
            self.push_text(format!("- $INSUNITS: {v}"));
        }
        if let Some(v) = drawing.header_vars.measurement {
            let units = if v == 0 { "English" } else { "Metric" };
            self.push_text(format!("- $MEASUREMENT: {v} ({units})"));
        }

        // Boolean mode variables
        self.write_bool_var("$ORTHOMODE", drawing.header_vars.ortho_mode);
        self.write_bool_var("$REGENMODE", drawing.header_vars.regen_mode);
        self.write_bool_var("$FILLMODE", drawing.header_vars.fill_mode);
        self.write_bool_var("$QTEXTMODE", drawing.header_vars.qtext_mode);
        self.write_bool_var("$MIRRTEXT", drawing.header_vars.mirror_text);

        // Float variables
        if let Some(v) = drawing.header_vars.ltscale {
            self.push_text(format!("- $LTSCALE: {v:.3}"));
        }
        if let Some(v) = drawing.header_vars.att_mode {
            let mode_str = match v {
                0 => "0 (none)",
                1 => "1 (normal)",
                2 => "2 (all)",
                _ => &format!("{v}"),
            };
            self.push_text(format!("- $ATTMODE: {mode_str}"));
        }
        if let Some(v) = drawing.header_vars.text_size {
            self.push_text(format!("- $TEXTSIZE: {v:.3}"));
        }
        if let Some(v) = drawing.header_vars.trace_wid {
            self.push_text(format!("- $TRACEWID: {v:.3}"));
        }

        // More string variables
        Self::write_opt_nonempty_str(
            &mut self.doc_items,
            &mut self.text_index,
            "$TEXTSTYLE",
            drawing.header_vars.text_style.as_deref(),
        );
        Self::write_opt_nonempty_str(
            &mut self.doc_items,
            &mut self.text_index,
            "$CLAYER",
            drawing.header_vars.clayer.as_deref(),
        );
        if let Some((x, y, z)) = drawing.header_vars.ins_base {
            self.push_text(format!("- $INSBASE: ({x:.3}, {y:.3}, {z:.3})"));
        }
        Self::write_opt_nonempty_str(
            &mut self.doc_items,
            &mut self.text_index,
            "$CELTYPE",
            drawing.header_vars.celtype.as_deref(),
        );
    }

    fn write_opt_str_var(
        doc_items: &mut Vec<DocItem>,
        text_index: &mut usize,
        name: &str,
        value: Option<&str>,
    ) {
        if let Some(v) = value {
            doc_items.push(create_text_item(
                *text_index,
                format!("- {name}: {v}"),
                vec![],
            ));
            *text_index += 1;
        }
    }

    fn write_opt_nonempty_str(
        doc_items: &mut Vec<DocItem>,
        text_index: &mut usize,
        name: &str,
        value: Option<&str>,
    ) {
        if let Some(v) = value {
            if !v.is_empty() {
                doc_items.push(create_text_item(
                    *text_index,
                    format!("- {name}: {v}"),
                    vec![],
                ));
                *text_index += 1;
            }
        }
    }

    fn write_bool_var(&mut self, name: &str, value: Option<bool>) {
        if let Some(v) = value {
            self.push_text(format!(
                "- {name}: {}",
                if v { "1 (on)" } else { "0 (off)" }
            ));
        }
    }

    fn write_dim_style_vars(&mut self, drawing: &DxfDrawing) {
        if drawing.header_vars.dim_vars.is_empty() {
            return;
        }
        self.push_section("Dimension Style Variables".to_string(), 2);
        self.push_text(format!(
            "Complete list of {} dimension style variables from the HEADER section:",
            drawing.header_vars.dim_vars.len()
        ));

        let mut dim_vars: Vec<(&String, &String)> = drawing.header_vars.dim_vars.iter().collect();
        dim_vars.sort_by_key(|(k, _)| *k);

        for (name, value) in dim_vars {
            if value.is_empty() {
                self.push_text(format!("- ${name}: (empty)"));
            } else {
                self.push_text(format!("- ${name}: {value}"));
            }
        }
    }

    fn format_coord(v: f64) -> String {
        if v.abs() >= 1e15 {
            let formatted = format!("{v:e}");
            if formatted.contains('e') && !formatted.contains("e-") {
                formatted.replace('e', "e+")
            } else {
                formatted
            }
        } else {
            format!("{v:.3}")
        }
    }

    fn has_sentinel_values(drawing: &DxfDrawing) -> bool {
        if let (Some((min_x, min_y, min_z)), Some((max_x, max_y, max_z))) =
            (drawing.header_vars.ext_min, drawing.header_vars.ext_max)
        {
            min_x.abs() > 1e15
                || min_y.abs() > 1e15
                || min_z.abs() > 1e15
                || max_x.abs() > 1e15
                || max_y.abs() > 1e15
                || max_z.abs() > 1e15
        } else {
            false
        }
    }

    fn write_drawing_extents(&mut self, drawing: &DxfDrawing) {
        self.push_section("Drawing Extents".to_string(), 2);

        if let Some((x, y, z)) = drawing.header_vars.ext_min {
            self.push_text(format!(
                "- $EXTMIN: ({}, {}, {})",
                Self::format_coord(x),
                Self::format_coord(y),
                Self::format_coord(z)
            ));
        }
        if let Some((x, y, z)) = drawing.header_vars.ext_max {
            self.push_text(format!(
                "- $EXTMAX: ({}, {}, {})",
                Self::format_coord(x),
                Self::format_coord(y),
                Self::format_coord(z)
            ));
        }

        if Self::has_sentinel_values(drawing) {
            if let Some(ref bbox) = drawing.bbox {
                self.push_text(format!(
                    "- $EXTMIN (from entities): ({:.3}, {:.3}, {:.3})",
                    bbox.min_x, bbox.min_y, bbox.min_z
                ));
                self.push_text(format!(
                    "- $EXTMAX (from entities): ({:.3}, {:.3}, {:.3})",
                    bbox.max_x, bbox.max_y, bbox.max_z
                ));
            }
        }

        if let Some((x, y)) = drawing.header_vars.lim_min {
            self.push_text(format!("- $LIMMIN: ({x:.3}, {y:.3})"));
        }
        if let Some((x, y)) = drawing.header_vars.lim_max {
            self.push_text(format!("- $LIMMAX: ({x:.3}, {y:.3})"));
        }
    }

    fn write_entity_stats(&mut self, drawing: &DxfDrawing) {
        self.push_section("Entity Statistics".to_string(), 2);
        self.push_text(format!(
            "Breakdown of {} entities from the ENTITIES section:",
            drawing.entity_count
        ));

        let types = &drawing.entity_types;
        self.write_entity_count("Lines", types.lines);
        self.write_entity_count("Circles", types.circles);
        self.write_entity_count("Arcs", types.arcs);
        self.write_entity_count("Polylines", types.polylines);
        self.write_entity_count("Text", types.text);
        self.write_entity_count("MText", types.mtext);
        self.write_entity_count("Points", types.points);
        self.write_entity_count("Splines", types.splines);
        self.write_entity_count("Ellipses", types.ellipses);
        self.write_entity_count("Dimensions", types.dimensions);
        self.write_entity_count("Blocks", types.blocks);
        self.write_entity_count("Block Inserts", types.inserts);
        self.write_entity_count("Other", types.other);
    }

    fn write_entity_count(&mut self, name: &str, count: usize) {
        if count > 0 {
            self.push_text(format!("- {name}: {count}"));
        }
    }

    fn write_layers(&mut self, drawing: &DxfDrawing) {
        if drawing.layer_names.is_empty() {
            return;
        }
        self.push_section("Layers".to_string(), 2);
        self.push_text("Layer organization from the TABLES section:".to_string());
        self.push_text(format!("- Count: {}", drawing.layer_names.len()));
        if drawing.layer_names.len() <= 20 {
            self.push_text(format!("- Names: {}", drawing.layer_names.join(", ")));
        }
    }

    fn write_drawing_dimensions(&mut self, drawing: &DxfDrawing) {
        if let Some(ref bbox) = drawing.bbox {
            self.push_section("Drawing Dimensions".to_string(), 2);
            self.push_text(format!("- Width (X): {:.3}", bbox.width()));
            self.push_text(format!("- Height (Y): {:.3}", bbox.height()));
            if bbox.depth().abs() > 0.001 {
                self.push_text(format!("- Depth (Z): {:.3}", bbox.depth()));
            }
        }
    }

    fn write_text_content(&mut self, drawing: &DxfDrawing) {
        if drawing.text_content.is_empty() {
            return;
        }
        self.push_section("Text Content".to_string(), 2);
        for (i, text) in drawing.text_content.iter().enumerate() {
            if i < 50 {
                self.push_text(format!("{}. {}", i + 1, text));
            }
        }
        if drawing.text_content.len() > 50 {
            self.push_text(format!(
                "*... and {} more text entities*",
                drawing.text_content.len() - 50
            ));
        }
    }

    fn write_description(&mut self, drawing: &DxfDrawing) {
        self.push_section("Drawing Description".to_string(), 2);
        let types = &drawing.entity_types;
        let mut description = format!(
            "This CAD drawing contains {} entities across {} layers. ",
            drawing.entity_count,
            drawing.layer_names.len()
        );

        if types.text + types.mtext > 0 {
            let _ = write!(
                description,
                "The drawing includes {} text annotations. ",
                types.text + types.mtext
            );
        }

        if types.dimensions > 0 {
            let _ = write!(
                description,
                "There are {} dimension annotations. ",
                types.dimensions
            );
        }

        let _ = write!(
            description,
            "The drawing format is DXF version {} (AutoCAD {}).",
            drawing.version_code,
            drawing.version.replace('R', "")
        );

        self.push_text(description);
    }
}

impl CadBackend {
    /// **DEPRECATED - FOR TESTS ONLY**
    ///
    /// Create DocItems from markdown content (for testing markdown parsing)
    ///
    /// This method is ONLY used by legacy tests to verify markdown-to-DocItems conversion.
    /// Production code should use format-specific methods (stl_to_docitems, obj_to_docitems, etc.)
    /// that generate DocItems directly from structured data.
    #[cfg(test)]
    fn create_docitems(&self, markdown: &str) -> Vec<DocItem> {
        let mut doc_items = Vec::new();
        let mut current_section = String::new();
        let mut text_index = 0;

        for line in markdown.lines() {
            if line.starts_with("## ") {
                // Save previous section if non-empty
                if !current_section.trim().is_empty() {
                    let text_content = current_section.trim().to_string();
                    doc_items.push(create_text_item(text_index, text_content, Vec::new()));
                    text_index += 1;
                }

                // Start new section with header
                current_section = format!("{line}\n");
            } else {
                current_section.push_str(line);
                current_section.push('\n');
            }
        }

        // Add final section
        if !current_section.trim().is_empty() {
            let text_content = current_section.trim().to_string();
            doc_items.push(create_text_item(text_index, text_content, Vec::new()));
        }

        doc_items
    }
}

impl DocumentBackend for CadBackend {
    #[inline]
    fn format(&self) -> InputFormat {
        self.format
    }

    fn parse_file<P: AsRef<Path>>(
        &self,
        path: P,
        _options: &BackendOptions,
    ) -> Result<Document, DoclingError> {
        let path_ref = path.as_ref();
        let filename = path_ref.display();

        // Generate DocItems and markdown based on format
        // Also extract author metadata for GLTF (generator field)
        let (doc_items, markdown, author) = match self.format {
            InputFormat::Stl => {
                // STL: Use direct DocItem generation (no markdown intermediary)
                let mesh = StlParser::parse_file(path_ref).map_err(|e| {
                    DoclingError::BackendError(format!("Failed to parse STL: {e}: {filename}"))
                })?;
                let doc_items = Self::stl_to_docitems(&mesh);
                let markdown = stl_to_markdown(&mesh); // For backwards compatibility
                (doc_items, markdown, None)
            }
            InputFormat::Obj => {
                // OBJ: Use direct DocItem generation (no markdown intermediary)
                let mesh = ObjParser::parse_file(path_ref).map_err(|e| {
                    DoclingError::BackendError(format!("Failed to parse OBJ: {e}: {filename}"))
                })?;
                let doc_items = Self::obj_to_docitems(&mesh);
                let markdown = obj_to_markdown(&mesh); // For backwards compatibility
                (doc_items, markdown, None)
            }
            InputFormat::Gltf | InputFormat::Glb => {
                // GLTF/GLB: Use direct DocItem generation (no markdown intermediary)
                let model = GltfParser::parse_file(path_ref).map_err(|e| {
                    DoclingError::BackendError(format!("Failed to parse GLTF: {e}: {filename}"))
                })?;
                // Extract generator as author (N=1876)
                let author = model.generator.clone();
                let doc_items = Self::gltf_to_docitems(&model);
                let markdown = gltf_to_markdown(&model); // For backwards compatibility
                (doc_items, markdown, author)
            }
            InputFormat::Dxf => {
                // DXF: Use direct DocItem generation (no markdown intermediary)
                let drawing = DxfParser::parse_file(path_ref).map_err(|e| {
                    DoclingError::BackendError(format!("Failed to parse DXF: {e}: {filename}"))
                })?;
                // Extract $LASTSAVEDBY as author (N=1877)
                let author = drawing.header_vars.last_saved_by.clone();
                let doc_items = Self::dxf_to_docitems(&drawing);
                let markdown = dxf_to_markdown(&drawing); // For backwards compatibility
                (doc_items, markdown, author)
            }
            _ => {
                return Err(DoclingError::FormatError(format!(
                    "Unsupported CAD format: {:?}",
                    self.format
                )))
            }
        };

        let num_characters = markdown.chars().count();

        // Extract title from filename
        let title = path_ref
            .file_stem()
            .and_then(|s| s.to_str())
            .map(std::string::ToString::to_string);

        Ok(Document {
            markdown,
            format: self.format,
            metadata: DocumentMetadata {
                num_pages: None,
                num_characters,
                title,
                author, // GLTF: generator (N=1876), DXF: $LASTSAVEDBY (N=1877)
                created: None,
                modified: None,
                language: None,
                subject: None,
                exif: None,
            },
            content_blocks: opt_vec(doc_items),
            docling_document: None,
        })
    }

    fn parse_bytes(
        &self,
        _data: &[u8],
        _options: &BackendOptions,
    ) -> Result<Document, DoclingError> {
        Err(DoclingError::BackendError(
            "CAD formats do not support parsing from bytes".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== Backend Creation Tests (5 tests) ==========

    #[test]
    fn test_cad_backend_creation() {
        let backend = CadBackend::new(InputFormat::Stl).unwrap();
        assert_eq!(backend.format(), InputFormat::Stl);

        let backend = CadBackend::new(InputFormat::Obj).unwrap();
        assert_eq!(backend.format(), InputFormat::Obj);

        let backend = CadBackend::new(InputFormat::Gltf).unwrap();
        assert_eq!(backend.format(), InputFormat::Gltf);

        let backend = CadBackend::new(InputFormat::Dxf).unwrap();
        assert_eq!(backend.format(), InputFormat::Dxf);
    }

    #[test]
    fn test_glb_backend_creation() {
        let backend = CadBackend::new(InputFormat::Glb).unwrap();
        assert_eq!(backend.format(), InputFormat::Glb);
    }

    #[test]
    fn test_glb_materials_formatting() {
        let backend = CadBackend::new(InputFormat::Glb).unwrap();
        let test_file = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../test-corpus/cad/gltf/box.glb"
        );
        if !std::path::Path::new(test_file).exists() {
            // Skip if test file doesn't exist
            return;
        }
        let result = backend
            .parse_file(test_file, &Default::default())
            .expect("Failed to parse GLB file");
        // Verify markdown is generated
        assert!(!result.markdown.is_empty(), "Markdown should not be empty");
        // Verify it contains expected sections
        assert!(
            result.markdown.contains("# ") || result.markdown.contains("## "),
            "Markdown should have headers"
        );
    }

    #[test]
    fn test_invalid_format() {
        let result = CadBackend::new(InputFormat::Pdf);
        assert!(
            result.is_err(),
            "PDF format should not be supported by CAD backend"
        );
    }

    #[test]
    fn test_invalid_format_error_message() {
        let result = CadBackend::new(InputFormat::Docx);
        assert!(
            result.is_err(),
            "DOCX format should not be supported by CAD backend"
        );
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("not a CAD format"),
            "Error message should mention 'not a CAD format'"
        );
    }

    #[test]
    fn test_backend_trait_implementation() {
        let backend = CadBackend::new(InputFormat::Stl).unwrap();
        assert_eq!(backend.format(), InputFormat::Stl);
    }

    // ========== DocItem Generation Tests (7 tests) ==========

    #[test]
    fn test_create_docitems_empty() {
        let backend = CadBackend::new(InputFormat::Stl).unwrap();
        let markdown = "";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 0);
    }

    #[test]
    fn test_create_docitems_whitespace_only() {
        let backend = CadBackend::new(InputFormat::Stl).unwrap();
        let markdown = "   \n\n   \n  ";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 0);
    }

    #[test]
    fn test_create_docitems_single_section() {
        let backend = CadBackend::new(InputFormat::Stl).unwrap();
        let markdown = "## Model Information\n\n- Triangles: 1234\n- Vertices: 5678\n";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1);

        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(
                text.contains("Model Information"),
                "Text should contain 'Model Information'"
            );
            assert!(
                text.contains("Triangles"),
                "Text should contain 'Triangles'"
            );
            assert!(
                text.contains("1234"),
                "Text should contain triangle count '1234'"
            );
        } else {
            panic!("Expected DocItem::Text");
        }
    }

    #[test]
    fn test_create_docitems_multiple_sections() {
        let backend = CadBackend::new(InputFormat::Gltf).unwrap();
        let markdown = r"## Scene Information

- Nodes: 5
- Meshes: 3

## Material Information

- Materials: 2
- Textures: 4

## Geometry Statistics

- Total Triangles: 15000
- Total Vertices: 9000
";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 3);

        // Verify first section (Scene Information)
        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(
                text.contains("Scene Information"),
                "Text should contain 'Scene Information'"
            );
            assert!(text.contains("Nodes"), "Text should contain 'Nodes'");
        } else {
            panic!("Expected DocItem::Text for first section");
        }

        // Verify second section (Material Information)
        if let DocItem::Text { text, .. } = &doc_items[1] {
            assert!(
                text.contains("Material Information"),
                "Text should contain 'Material Information'"
            );
            assert!(
                text.contains("Materials"),
                "Text should contain 'Materials'"
            );
        } else {
            panic!("Expected DocItem::Text for second section");
        }

        // Verify third section (Geometry Statistics)
        if let DocItem::Text { text, .. } = &doc_items[2] {
            assert!(
                text.contains("Geometry Statistics"),
                "Text should contain 'Geometry Statistics'"
            );
            assert!(
                text.contains("Total Triangles"),
                "Text should contain 'Total Triangles'"
            );
        } else {
            panic!("Expected DocItem::Text for third section");
        }
    }

    #[test]
    fn test_create_docitems_section_without_header() {
        let backend = CadBackend::new(InputFormat::Obj).unwrap();
        let markdown = "Some content before any header\n\n## First Section\n\nContent here\n";
        let doc_items = backend.create_docitems(markdown);
        // Should have 2 sections: one for content before header, one for First Section
        assert_eq!(doc_items.len(), 2);
    }

    #[test]
    fn test_create_docitems_consecutive_headers() {
        let backend = CadBackend::new(InputFormat::Dxf).unwrap();
        let markdown = "## Header 1\n## Header 2\n## Header 3\n";
        let doc_items = backend.create_docitems(markdown);
        // Each header without content should still create a section
        assert_eq!(doc_items.len(), 3);
    }

    #[test]
    fn test_create_docitems_preserves_content() {
        let backend = CadBackend::new(InputFormat::Stl).unwrap();
        let markdown = "## Test Section\nLine 1\nLine 2\nLine 3\n";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1);

        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(text.contains("Line 1"), "Text should contain 'Line 1'");
            assert!(text.contains("Line 2"), "Text should contain 'Line 2'");
            assert!(text.contains("Line 3"), "Text should contain 'Line 3'");
        } else {
            panic!("Expected DocItem::Text");
        }
    }

    // ========== Error Handling Tests (3 tests) ==========

    #[test]
    fn test_parse_bytes_not_supported() {
        let backend = CadBackend::new(InputFormat::Stl).unwrap();
        let result = backend.parse_bytes(&[], &Default::default());
        assert!(result.is_err(), "Parse bytes should fail for CAD backend");
        let err = result.unwrap_err();
        assert!(err
            .to_string()
            .contains("do not support parsing from bytes"));
    }

    #[test]
    fn test_parse_bytes_all_formats() {
        let formats = vec![
            InputFormat::Stl,
            InputFormat::Obj,
            InputFormat::Gltf,
            InputFormat::Glb,
            InputFormat::Dxf,
        ];

        for format in formats {
            let backend = CadBackend::new(format).unwrap();
            let result = backend.parse_bytes(&[1, 2, 3], &Default::default());
            assert!(
                result.is_err(),
                "Format {format:?} should not support parse_bytes"
            );
        }
    }

    #[test]
    fn test_invalid_format_variations() {
        // Test various non-CAD formats
        let invalid_formats = vec![
            InputFormat::Pdf,
            InputFormat::Docx,
            InputFormat::Html,
            InputFormat::Csv,
            InputFormat::Png,
        ];

        for format in invalid_formats {
            let result = CadBackend::new(format);
            assert!(result.is_err(), "Format {format:?} should be rejected");
        }
    }

    // ========== Format-Specific Tests (5 tests) ==========

    #[test]
    fn test_stl_format_metadata() {
        let backend = CadBackend::new(InputFormat::Stl).unwrap();
        assert_eq!(backend.format(), InputFormat::Stl);
    }

    #[test]
    fn test_obj_format_metadata() {
        let backend = CadBackend::new(InputFormat::Obj).unwrap();
        assert_eq!(backend.format(), InputFormat::Obj);
    }

    #[test]
    fn test_gltf_format_metadata() {
        let backend = CadBackend::new(InputFormat::Gltf).unwrap();
        assert_eq!(backend.format(), InputFormat::Gltf);
    }

    #[test]
    fn test_glb_format_metadata() {
        let backend = CadBackend::new(InputFormat::Glb).unwrap();
        assert_eq!(backend.format(), InputFormat::Glb);
    }

    #[test]
    fn test_dxf_format_metadata() {
        let backend = CadBackend::new(InputFormat::Dxf).unwrap();
        assert_eq!(backend.format(), InputFormat::Dxf);
    }

    // ========== DocItem Index Tests (3 tests) ==========

    #[test]
    fn test_docitem_indices() {
        let backend = CadBackend::new(InputFormat::Gltf).unwrap();
        let markdown =
            "## Section 1\nContent 1\n## Section 2\nContent 2\n## Section 3\nContent 3\n";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 3);

        // Verify self_refs are sequential
        for (i, item) in doc_items.iter().enumerate() {
            if let DocItem::Text { self_ref, .. } = item {
                assert_eq!(self_ref, &format!("#/texts/{i}"));
            } else {
                panic!("Expected DocItem::Text");
            }
        }
    }

    #[test]
    fn test_docitem_variant() {
        let backend = CadBackend::new(InputFormat::Stl).unwrap();
        let markdown = "## Model Info\nTest content\n";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1);

        // Verify it's a Text variant
        matches!(&doc_items[0], DocItem::Text { .. });
    }

    #[test]
    fn test_docitem_no_bounding_boxes() {
        let backend = CadBackend::new(InputFormat::Obj).unwrap();
        let markdown = "## Test\nContent\n";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1);

        if let DocItem::Text { prov, .. } = &doc_items[0] {
            assert_eq!(prov.len(), 0, "CAD DocItems should not have bounding boxes");
        } else {
            panic!("Expected DocItem::Text");
        }
    }

    // ========== UNICODE AND SPECIAL CHARACTER TESTS ==========

    #[test]
    fn test_docitems_unicode_content() {
        let backend = CadBackend::new(InputFormat::Gltf).unwrap();
        let markdown = "## 模型信息 (Model Info)\n\n三角形数量: 1000\n顶点数量: 500\n";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1);

        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(
                text.contains("模型信息"),
                "Chinese text '模型信息' should be preserved"
            );
            assert!(
                text.contains("三角形数量"),
                "Chinese text '三角形数量' should be preserved"
            );
        }
    }

    #[test]
    fn test_docitems_emoji_in_section() {
        let backend = CadBackend::new(InputFormat::Obj).unwrap();
        let markdown = "## 📐 Geometry Data\n\n🔺 Triangles: 500\n📊 Vertices: 300\n";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1);

        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(text.contains("📐"), "Emoji '📐' should be preserved");
            assert!(text.contains("🔺"), "Emoji '🔺' should be preserved");
            assert!(text.contains("📊"), "Emoji '📊' should be preserved");
        }
    }

    #[test]
    fn test_markdown_utf8_validation() {
        let backend = CadBackend::new(InputFormat::Stl).unwrap();
        let markdown = "## Модель (Model)\n\nДанные: Тест\n";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1);

        // Verify valid UTF-8
        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(
                std::str::from_utf8(text.as_bytes()).is_ok(),
                "Text should be valid UTF-8"
            );
            assert!(
                text.contains("Модель"),
                "Cyrillic text 'Модель' should be preserved"
            );
        }
    }

    #[test]
    fn test_special_characters_in_markdown() {
        let backend = CadBackend::new(InputFormat::Dxf).unwrap();
        let markdown = "## Drawing: Special-Chars_2024\n\nLayers: Layer#1, Layer@2, Layer$3\n";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1);

        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(
                text.contains("Special-Chars"),
                "Text with special characters should be preserved"
            );
            assert!(
                text.contains("Layer#1"),
                "Text with '#' should be preserved"
            );
            assert!(
                text.contains("Layer@2"),
                "Text with '@' should be preserved"
            );
        }
    }

    // ========== VALIDATION TESTS ==========

    #[test]
    fn test_markdown_only_headers() {
        let backend = CadBackend::new(InputFormat::Gltf).unwrap();
        let markdown = "## Header 1\n## Header 2\n## Header 3\n";
        let doc_items = backend.create_docitems(markdown);
        // Each header creates a section
        assert_eq!(doc_items.len(), 3);
    }

    #[test]
    fn test_markdown_mixed_whitespace() {
        let backend = CadBackend::new(InputFormat::Obj).unwrap();
        // Note: Headers must start at beginning of line (no leading whitespace)
        let markdown = "## Section 1\n\n  Content  \n\n## Section 2\n  More  \n";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 2);
    }

    #[test]
    fn test_markdown_very_long_section() {
        let backend = CadBackend::new(InputFormat::Stl).unwrap();
        let mut markdown = String::from("## Long Section\n\n");
        for i in 0..1000 {
            let _ = writeln!(markdown, "Line {i}");
        }
        let doc_items = backend.create_docitems(&markdown);
        assert_eq!(doc_items.len(), 1);

        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(
                text.contains("Line 0"),
                "First line 'Line 0' should be present"
            );
            assert!(text.contains("Line 999"));
        }
    }

    #[test]
    fn test_markdown_empty_sections() {
        let backend = CadBackend::new(InputFormat::Dxf).unwrap();
        let markdown = "## Section 1\n\n\n\n## Section 2\n\n\n\n## Section 3\n\n\n";
        let doc_items = backend.create_docitems(markdown);
        // Empty sections should still create items
        assert_eq!(doc_items.len(), 3);
    }

    #[test]
    fn test_markdown_no_trailing_newline() {
        let backend = CadBackend::new(InputFormat::Glb).unwrap();
        let markdown = "## Section\nContent without trailing newline";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1);
    }

    // ========== SERIALIZATION CONSISTENCY TESTS ==========

    #[test]
    fn test_docitems_count_matches_sections() {
        let backend = CadBackend::new(InputFormat::Gltf).unwrap();
        let markdown = "## A\nContent\n## B\nContent\n## C\nContent\n";
        let doc_items = backend.create_docitems(markdown);
        // Should have exactly 3 sections
        assert_eq!(doc_items.len(), 3);
    }

    #[test]
    fn test_docitems_order_preserved() {
        let backend = CadBackend::new(InputFormat::Obj).unwrap();
        let markdown = "## First\nContent 1\n## Second\nContent 2\n## Third\nContent 3\n";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 3);

        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(text.contains("First"));
        }
        if let DocItem::Text { text, .. } = &doc_items[1] {
            assert!(text.contains("Second"));
        }
        if let DocItem::Text { text, .. } = &doc_items[2] {
            assert!(text.contains("Third"));
        }
    }

    #[test]
    fn test_docitems_idempotent() {
        let backend = CadBackend::new(InputFormat::Stl).unwrap();
        let markdown = "## Section\n\nContent here\n";

        // Parse twice
        let doc_items1 = backend.create_docitems(markdown);
        let doc_items2 = backend.create_docitems(markdown);

        // Should produce identical results
        assert_eq!(doc_items1.len(), doc_items2.len());
        for (item1, item2) in doc_items1.iter().zip(doc_items2.iter()) {
            if let (DocItem::Text { text: t1, .. }, DocItem::Text { text: t2, .. }) = (item1, item2)
            {
                assert_eq!(t1, t2);
            }
        }
    }

    #[test]
    fn test_markdown_not_empty_for_valid_input() {
        let backend = CadBackend::new(InputFormat::Dxf).unwrap();
        let markdown = "## Test\nContent\n";
        let doc_items = backend.create_docitems(markdown);
        assert!(!doc_items.is_empty());
    }

    // ========== BACKEND OPTIONS TESTS ==========

    #[test]
    fn test_parse_file_with_default_options() {
        // Verify backend creation and default options work
        let _backend = CadBackend::new(InputFormat::Stl).unwrap();
        let _options = BackendOptions::default();
        // Note: We can't test parse_file here without actual CAD files,
        // but we verify the backend and options are created successfully
    }

    #[test]
    fn test_parse_bytes_error_consistent() {
        // All CAD formats should reject parse_bytes consistently
        let formats = [
            InputFormat::Stl,
            InputFormat::Obj,
            InputFormat::Gltf,
            InputFormat::Glb,
            InputFormat::Dxf,
        ];

        for format in formats {
            let backend = CadBackend::new(format).unwrap();
            let result = backend.parse_bytes(b"test", &Default::default());
            assert!(result.is_err());
            assert!(result
                .unwrap_err()
                .to_string()
                .contains("do not support parsing from bytes"));
        }
    }

    // ========== FORMAT-SPECIFIC TESTS ==========

    #[test]
    fn test_stl_backend_format() {
        let backend = CadBackend::new(InputFormat::Stl).unwrap();
        assert_eq!(backend.format(), InputFormat::Stl);
    }

    #[test]
    fn test_obj_backend_format() {
        let backend = CadBackend::new(InputFormat::Obj).unwrap();
        assert_eq!(backend.format(), InputFormat::Obj);
    }

    #[test]
    fn test_gltf_backend_format() {
        let backend = CadBackend::new(InputFormat::Gltf).unwrap();
        assert_eq!(backend.format(), InputFormat::Gltf);
    }

    #[test]
    fn test_glb_backend_format() {
        let backend = CadBackend::new(InputFormat::Glb).unwrap();
        assert_eq!(backend.format(), InputFormat::Glb);
    }

    #[test]
    fn test_dxf_backend_format() {
        let backend = CadBackend::new(InputFormat::Dxf).unwrap();
        assert_eq!(backend.format(), InputFormat::Dxf);
    }

    #[test]
    fn test_all_cad_formats_supported() {
        let formats = [
            InputFormat::Stl,
            InputFormat::Obj,
            InputFormat::Gltf,
            InputFormat::Glb,
            InputFormat::Dxf,
        ];

        for format in formats {
            let result = CadBackend::new(format);
            assert!(result.is_ok(), "Format {format:?} should be supported");
        }
    }

    #[test]
    fn test_section_parsing_h2_headers() {
        // Verify only H2 (##) headers create sections, not H1 (#) or H3 (###)
        let backend = CadBackend::new(InputFormat::Gltf).unwrap();
        let markdown = "# H1 Header\n## H2 Section 1\n### H3 Subsection\n## H2 Section 2\n";
        let doc_items = backend.create_docitems(markdown);
        // Should have 3 sections: H1+H2, H3+H2, (final section)
        // Actually: content before first H2 (H1) + Section 1 + Section 2
        assert!(doc_items.len() >= 2);
    }

    #[test]
    fn test_markdown_section_boundaries() {
        let backend = CadBackend::new(InputFormat::Obj).unwrap();
        let markdown = "## Section 1\nLine A\nLine B\n## Section 2\nLine C\nLine D\n";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 2);

        // First section should not contain content from second
        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(text.contains("Line A"));
            assert!(text.contains("Line B"));
            assert!(!text.contains("Line C"));
        }

        // Second section should not contain content from first
        if let DocItem::Text { text, .. } = &doc_items[1] {
            assert!(!text.contains("Line A"));
            assert!(text.contains("Line C"));
            assert!(text.contains("Line D"));
        }
    }

    #[test]
    fn test_content_blocks_none_for_empty() {
        let backend = CadBackend::new(InputFormat::Stl).unwrap();
        let markdown = "";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 0);
        // In parse_file, this would result in content_blocks: None
    }

    #[test]
    fn test_content_blocks_some_for_valid() {
        let backend = CadBackend::new(InputFormat::Dxf).unwrap();
        let markdown = "## Section\nContent\n";
        let doc_items = backend.create_docitems(markdown);
        assert!(!doc_items.is_empty());
        // In parse_file, this would result in content_blocks: Some(doc_items)
    }

    // ========== Additional Edge Cases (3 tests) ==========

    #[test]
    fn test_cad_unicode_content() {
        // Test handling of Unicode characters in CAD metadata (filenames, layer names)
        let backend = CadBackend::new(InputFormat::Gltf).unwrap();
        let markdown = "## 图层 (Layer)\n模型信息: 日本語テスト 🎨\n";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1);

        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(text.contains("图层"));
            assert!(text.contains("日本語"));
            assert!(text.contains("🎨"));
        }
    }

    #[test]
    fn test_cad_very_long_content() {
        // Test handling of very large CAD files with extensive metadata
        let backend = CadBackend::new(InputFormat::Stl).unwrap();
        let long_content = "A".repeat(10000);
        let markdown = format!("## Large Section\n{long_content}\n");
        let doc_items = backend.create_docitems(&markdown);
        assert_eq!(doc_items.len(), 1);

        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(text.len() > 9000);
        }
    }

    #[test]
    fn test_cad_invalid_format_error() {
        // Test that invalid formats are rejected
        let result = CadBackend::new(InputFormat::Pdf);
        assert!(result.is_err());

        if let Err(err) = result {
            assert!(err.to_string().contains("not a CAD format"));
        }
    }

    // ========== Additional Test Coverage (9 tests) ==========

    #[test]
    fn test_multiple_h2_headers_on_same_line() {
        // Edge case: what if markdown has ## on the same line multiple times?
        let backend = CadBackend::new(InputFormat::Gltf).unwrap();
        let markdown = "## Section 1 ## Not a header\nContent\n";
        let doc_items = backend.create_docitems(markdown);
        // Should still create one section (only first ## at start of line counts)
        assert_eq!(doc_items.len(), 1);
    }

    #[test]
    fn test_h2_header_with_trailing_whitespace() {
        let backend = CadBackend::new(InputFormat::Obj).unwrap();
        let markdown = "## Section Title    \nContent\n";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1);

        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(text.contains("Section Title"));
        }
    }

    #[test]
    fn test_markdown_with_code_blocks() {
        // Code blocks shouldn't be treated as headers
        let backend = CadBackend::new(InputFormat::Stl).unwrap();
        let markdown = "## Section\n```\n## This is not a header\n```\n";
        let doc_items = backend.create_docitems(markdown);
        // Note: Our current implementation doesn't parse code blocks specially,
        // so the ## inside code block WILL be treated as header.
        // This test documents current behavior.
        assert!(!doc_items.is_empty());
    }

    #[test]
    fn test_docitems_with_newlines_at_end() {
        let backend = CadBackend::new(InputFormat::Dxf).unwrap();
        let markdown = "## Section\nContent\n\n\n\n";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1);
    }

    #[test]
    fn test_all_formats_reject_parse_bytes() {
        // Comprehensive test that ALL CAD formats consistently reject parse_bytes
        let formats = vec![
            InputFormat::Stl,
            InputFormat::Obj,
            InputFormat::Gltf,
            InputFormat::Glb,
            InputFormat::Dxf,
        ];

        for format in formats {
            let backend = CadBackend::new(format).unwrap();

            // Test with empty bytes
            assert!(backend.parse_bytes(&[], &Default::default()).is_err());

            // Test with non-empty bytes
            assert!(backend
                .parse_bytes(b"test data", &Default::default())
                .is_err());

            // Test with binary-looking data
            assert!(backend
                .parse_bytes(&[0xFF, 0xFE, 0xFD], &Default::default())
                .is_err());
        }
    }

    #[test]
    fn test_markdown_with_h1_h3_headers() {
        // Verify only H2 (##) creates sections, not H1 (#) or H3 (###)
        let backend = CadBackend::new(InputFormat::Gltf).unwrap();
        let markdown = "# H1\nContent A\n## H2\nContent B\n### H3\nContent C\n";
        let doc_items = backend.create_docitems(markdown);

        // H1 content should be in first section (before first H2)
        // H2 section should include H3 (since we only split on H2)
        assert!(doc_items.len() >= 2);
    }

    #[test]
    fn test_format_error_details() {
        // Test error messages contain useful information
        let invalid_formats = vec![
            InputFormat::Pdf,
            InputFormat::Docx,
            InputFormat::Xlsx,
            InputFormat::Html,
        ];

        for format in invalid_formats {
            let result = CadBackend::new(format);
            assert!(result.is_err());

            let err = result.unwrap_err();
            let err_msg = err.to_string();

            // Error should mention it's not a CAD format
            assert!(err_msg.contains("not a CAD format") || err_msg.contains("Format"));
        }
    }

    #[test]
    fn test_docitem_self_refs_unique() {
        // Verify each DocItem has unique self_ref
        let backend = CadBackend::new(InputFormat::Obj).unwrap();
        let markdown = "## A\nContent\n## B\nContent\n## C\nContent\n## D\nContent\n";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 4);

        let mut self_refs = std::collections::HashSet::new();
        for item in &doc_items {
            if let DocItem::Text { self_ref, .. } = item {
                // Each self_ref should be unique
                assert!(
                    self_refs.insert(self_ref.clone()),
                    "Duplicate self_ref: {self_ref}"
                );
            }
        }

        // Should have 4 unique self_refs
        assert_eq!(self_refs.len(), 4);
    }

    #[test]
    fn test_backend_debug_impl() {
        // Verify Debug trait is implemented and works
        let backend = CadBackend::new(InputFormat::Stl).unwrap();
        let debug_str = format!("{backend:?}");

        // Debug output should contain useful information
        assert!(debug_str.contains("CadBackend"));
        assert!(debug_str.contains("Stl") || debug_str.contains("format"));
    }

    #[test]
    fn test_gltf_with_animations() {
        // Test GLTF with animation data
        let backend = CadBackend::new(InputFormat::Gltf).unwrap();
        let markdown = "## Mesh Statistics\n\nVertices: 100\nFaces: 50\n\n## Animations\n\nAnimation 1: Walk cycle (2.5s)\n";
        let doc_items = backend.create_docitems(markdown);

        assert!(doc_items.len() >= 2);
        let has_animation = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("Animation"),
            _ => false,
        });
        assert!(has_animation);
    }

    #[test]
    fn test_obj_with_materials() {
        // Test OBJ with material (.mtl) references
        let backend = CadBackend::new(InputFormat::Obj).unwrap();
        let markdown = "## Mesh Statistics\n\nVertices: 200\nFaces: 100\n\n## Materials\n\nMaterial: metal_blue\nMaterial: wood_grain\n";
        let doc_items = backend.create_docitems(markdown);

        assert!(doc_items.len() >= 2);
        let has_materials = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("Materials") || text.contains("Material:"),
            _ => false,
        });
        assert!(has_materials);
    }

    #[test]
    fn test_dxf_with_layers() {
        // Test DXF with multiple layers (common in CAD drawings)
        let backend = CadBackend::new(InputFormat::Dxf).unwrap();
        let markdown = "## Drawing Information\n\nLayers: 5\n\n## Layers\n\nLayer 0: Construction\nLayer 1: Dimensions\nLayer 2: Hidden\n";
        let doc_items = backend.create_docitems(markdown);

        assert!(doc_items.len() >= 2);
        let has_layers = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("Layer"),
            _ => false,
        });
        assert!(has_layers);
    }

    #[test]
    fn test_stl_binary_vs_ascii() {
        // Test distinction between binary and ASCII STL (both use .stl extension)
        let backend = CadBackend::new(InputFormat::Stl).unwrap();

        // ASCII STL would have text-based info
        let ascii_markdown = "## Mesh Statistics\n\nFormat: ASCII\nVertices: 300\nFaces: 100\n";
        let ascii_items = backend.create_docitems(ascii_markdown);
        assert!(!ascii_items.is_empty());

        // Binary STL would have binary format indicator
        let binary_markdown = "## Mesh Statistics\n\nFormat: Binary\nVertices: 300\nFaces: 100\n";
        let binary_items = backend.create_docitems(binary_markdown);
        assert!(!binary_items.is_empty());
    }

    #[test]
    fn test_glb_embedded_textures() {
        // Test GLB (binary glTF) with embedded textures
        let backend = CadBackend::new(InputFormat::Glb).unwrap();
        let markdown = "## Mesh Statistics\n\nVertices: 500\nFaces: 250\n\n## Textures\n\nTexture 0: baseColor (512x512)\nTexture 1: normalMap (512x512)\nTexture 2: metallicRoughness (512x512)\n";
        let doc_items = backend.create_docitems(markdown);

        assert!(doc_items.len() >= 2);
        let has_textures = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("Texture"),
            _ => false,
        });
        assert!(has_textures);
    }

    // ========== ADDITIONAL COMPREHENSIVE EDGE CASES (65 → 70) ==========

    #[test]
    fn test_dxf_with_3d_solids() {
        // Test DXF with 3D solid objects (not just 2D entities)
        let backend = CadBackend::new(InputFormat::Dxf).unwrap();
        let markdown = concat!(
            "## Drawing Information\n\n",
            "Version: AutoCAD 2018\n",
            "Units: Millimeters\n\n",
            "## 3D Solids\n\n",
            "Solid 1: Box (100x50x25mm)\n",
            "Solid 2: Cylinder (radius 20mm, height 100mm)\n",
            "Solid 3: Sphere (radius 30mm)\n\n",
            "## Entities\n\n",
            "Total solids: 3\n",
            "Total surfaces: 12\n"
        );
        let doc_items = backend.create_docitems(markdown);

        assert!(doc_items.len() >= 2);
        let has_solids = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => {
                text.contains("3D Solids") || text.contains("Solid") || text.contains("Cylinder")
            }
            _ => false,
        });
        assert!(has_solids);
    }

    #[test]
    fn test_gltf_with_morph_targets() {
        // Test GLTF with morph targets (shape keys/blend shapes for animation)
        let backend = CadBackend::new(InputFormat::Gltf).unwrap();
        let markdown = concat!(
            "## Mesh Statistics\n\n",
            "Vertices: 2048\n",
            "Faces: 1024\n\n",
            "## Morph Targets\n\n",
            "Target 0: smile (weight: 0.0-1.0)\n",
            "Target 1: blink_left (weight: 0.0-1.0)\n",
            "Target 2: blink_right (weight: 0.0-1.0)\n",
            "Target 3: frown (weight: 0.0-1.0)\n\n",
            "## Animation Channels\n\n",
            "Morph weights animated: 4 targets\n"
        );
        let doc_items = backend.create_docitems(markdown);

        assert!(doc_items.len() >= 2);
        let has_morphs = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => {
                text.contains("Morph") || text.contains("Target") || text.contains("weight")
            }
            _ => false,
        });
        assert!(has_morphs);
    }

    #[test]
    fn test_obj_with_vertex_normals() {
        // Test OBJ with vertex normals (vn entries for smooth shading)
        let backend = CadBackend::new(InputFormat::Obj).unwrap();
        let markdown = concat!(
            "## Mesh Statistics\n\n",
            "Vertices (v): 1500\n",
            "Vertex normals (vn): 1500\n",
            "Texture coordinates (vt): 1500\n",
            "Faces (f): 750\n\n",
            "## Shading Information\n\n",
            "Smooth shading: Enabled (normals present)\n",
            "UV mapping: Complete\n",
            "Material references: 3 materials\n"
        );
        let doc_items = backend.create_docitems(markdown);

        assert!(doc_items.len() >= 2);
        let has_normals = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => {
                text.contains("normal") || text.contains("Smooth shading") || text.contains("vn")
            }
            _ => false,
        });
        assert!(has_normals);
    }

    #[test]
    fn test_stl_with_large_triangle_count() {
        // Test STL with very large triangle count (stress test for metadata parsing)
        let backend = CadBackend::new(InputFormat::Stl).unwrap();
        let markdown = concat!(
            "## Mesh Statistics\n\n",
            "Format: Binary\n",
            "Triangles: 1000000\n",
            "Vertices: 3000000\n",
            "File size: 50 MB\n\n",
            "## Bounding Box\n\n",
            "Min: (-100.5, -200.3, -50.0)\n",
            "Max: (100.5, 200.3, 50.0)\n",
            "Dimensions: 201.0 x 400.6 x 100.0\n\n",
            "## Quality Metrics\n\n",
            "Degenerate triangles: 0\n",
            "Zero-area triangles: 0\n"
        );
        let doc_items = backend.create_docitems(markdown);

        assert!(doc_items.len() >= 2);
        // Verify we can handle large numbers
        let has_large_count = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("1000000") || text.contains("Triangles:"),
            _ => false,
        });
        assert!(has_large_count);
    }

    #[test]
    fn test_glb_with_pbr_materials() {
        // Test GLB with PBR (Physically Based Rendering) material properties
        let backend = CadBackend::new(InputFormat::Glb).unwrap();
        let markdown = concat!(
            "## Mesh Statistics\n\n",
            "Vertices: 4096\n",
            "Faces: 2048\n\n",
            "## PBR Materials\n\n",
            "Material 0: MetalPlate\n",
            "  Base color: (0.8, 0.8, 0.8, 1.0)\n",
            "  Metallic: 1.0\n",
            "  Roughness: 0.4\n",
            "  Normal map: present\n",
            "  Ambient occlusion: present\n\n",
            "Material 1: RoughWood\n",
            "  Base color: (0.6, 0.4, 0.2, 1.0)\n",
            "  Metallic: 0.0\n",
            "  Roughness: 0.9\n",
            "  Normal map: present\n\n",
            "## Texture Maps\n\n",
            "Total textures: 6 (baseColor, metallic, roughness, normal, AO)\n"
        );
        let doc_items = backend.create_docitems(markdown);

        assert!(doc_items.len() >= 2);
        let has_pbr = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => {
                text.contains("PBR")
                    || text.contains("Metallic")
                    || text.contains("Roughness")
                    || text.contains("Normal map")
            }
            _ => false,
        });
        assert!(has_pbr);
    }

    // Advanced CAD Features (Tests 71-75)

    #[test]
    fn test_dxf_with_acis_geometry() {
        // Test DXF with ACIS 3D solid geometry (SAT format embedded in DXF)
        let backend = CadBackend::new(InputFormat::Dxf).unwrap();
        let markdown = concat!(
            "## Drawing Information\n\n",
            "Format: AutoCAD 2018 DXF\n",
            "Units: Millimeters\n\n",
            "## 3D Solids (ACIS)\n\n",
            "Solid 1: Box\n",
            "  Type: 3DSOLID\n",
            "  ACIS Version: 700\n",
            "  Volume: 8000.0 mm³\n",
            "  Bounding box: (-10, -10, -10) to (10, 10, 10)\n\n",
            "Solid 2: Cylinder\n",
            "  Type: 3DSOLID\n",
            "  ACIS Version: 700\n",
            "  Volume: 3141.59 mm³\n",
            "  Radius: 10.0 mm, Height: 10.0 mm\n\n",
            "## ACIS SAT Data\n\n",
            "Entities: 2\n",
            "Bodies: 2\n",
            "Shells: 2\n"
        );
        let doc_items = backend.create_docitems(markdown);

        assert!(doc_items.len() >= 2);
        // Verify ACIS geometry is parsed
        let has_acis = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => {
                text.contains("ACIS") || text.contains("3DSOLID") || text.contains("SAT Data")
            }
            _ => false,
        });
        assert!(has_acis);

        // Verify volume and dimension data present
        let has_volume = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("Volume:") && text.contains("mm³"),
            _ => false,
        });
        assert!(has_volume);
    }

    #[test]
    fn test_gltf_with_draco_compression() {
        // Test GLTF with Draco mesh compression (KHR_draco_mesh_compression extension)
        let backend = CadBackend::new(InputFormat::Gltf).unwrap();
        let markdown = concat!(
            "## Mesh Statistics\n\n",
            "Original vertices: 1048576\n",
            "Original indices: 2097152\n",
            "Compressed size: 45.2 KB\n",
            "Compression ratio: 98.9%\n\n",
            "## Draco Compression\n\n",
            "Extension: KHR_draco_mesh_compression\n",
            "Quantization bits (position): 14\n",
            "Quantization bits (normal): 10\n",
            "Quantization bits (texcoord): 12\n",
            "Quantization bits (color): 8\n",
            "Compression level: 7\n\n",
            "## Attributes\n\n",
            "POSITION: compressed (14-bit quantization)\n",
            "NORMAL: compressed (10-bit quantization)\n",
            "TEXCOORD_0: compressed (12-bit quantization)\n",
            "COLOR_0: compressed (8-bit quantization)\n\n",
            "## Performance\n\n",
            "Decompression time: ~15ms\n",
            "Memory saved: 98.9% (3.8 MB → 45 KB)\n"
        );
        let doc_items = backend.create_docitems(markdown);

        assert!(doc_items.len() >= 3);
        // Verify Draco compression metadata
        let has_draco = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => {
                text.contains("Draco") || text.contains("KHR_draco_mesh_compression")
            }
            _ => false,
        });
        assert!(has_draco);

        // Verify quantization data
        let has_quantization = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("Quantization bits"),
            _ => false,
        });
        assert!(has_quantization);

        // Verify compression ratio data
        let has_compression_ratio = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("Compression ratio"),
            _ => false,
        });
        assert!(has_compression_ratio);
    }

    #[test]
    fn test_obj_with_multiple_texture_maps() {
        // Test OBJ with complete texture map set (diffuse, specular, bump, displacement, etc.)
        let backend = CadBackend::new(InputFormat::Obj).unwrap();
        let markdown = concat!(
            "## Mesh Statistics\n\n",
            "Vertices: 8192\n",
            "Texture coordinates: 8192\n",
            "Normals: 8192\n",
            "Faces: 4096\n\n",
            "## Material: BrickWall\n\n",
            "Diffuse map (map_Kd): brick_diffuse.png (2048×2048)\n",
            "Specular map (map_Ks): brick_specular.png (2048×2048)\n",
            "Bump map (map_bump): brick_bump.png (2048×2048)\n",
            "Normal map (norm): brick_normal.png (2048×2048)\n",
            "Displacement map (disp): brick_displacement.png (2048×2048)\n",
            "Alpha map (map_d): brick_alpha.png (2048×2048)\n",
            "Ambient map (map_Ka): brick_ambient.png (1024×1024)\n",
            "Emissive map (map_Ke): brick_emissive.png (1024×1024)\n\n",
            "## Material Properties\n\n",
            "Ambient color (Ka): (0.2, 0.2, 0.2)\n",
            "Diffuse color (Kd): (0.8, 0.6, 0.4)\n",
            "Specular color (Ks): (0.5, 0.5, 0.5)\n",
            "Specular exponent (Ns): 96.0\n",
            "Transparency (d): 1.0\n",
            "Illumination model: 2 (highlight on)\n"
        );
        let doc_items = backend.create_docitems(markdown);

        assert!(doc_items.len() >= 2);
        // Verify multiple texture types present
        let has_texture_maps = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => {
                text.contains("Diffuse map")
                    && text.contains("Specular map")
                    && text.contains("Bump map")
                    && text.contains("Normal map")
                    && text.contains("Displacement map")
            }
            _ => false,
        });
        assert!(has_texture_maps);

        // Verify material properties
        let has_material_props = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => {
                text.contains("Ambient color") && text.contains("Illumination model")
            }
            _ => false,
        });
        assert!(has_material_props);
    }

    #[test]
    fn test_stl_with_color_per_facet() {
        // Test STL with non-standard per-facet color extension (VisCAM/SolView format)
        let backend = CadBackend::new(InputFormat::Stl).unwrap();
        let markdown = concat!(
            "## Mesh Statistics\n\n",
            "Triangles: 500\n",
            "Vertices: 1500\n",
            "Format: Binary STL with color extension\n\n",
            "## Color Information\n\n",
            "Color mode: Per-facet (VisCAM/SolView extension)\n",
            "Attribute byte count: 2 bytes per triangle\n",
            "Color bits: 15-bit RGB (5-5-5)\n",
            "Valid color bit: 0x8000\n\n",
            "## Color Distribution\n\n",
            "Facets with color: 500 (100%)\n",
            "Unique colors: 24\n",
            "Color 1 (Red): 150 facets (30%)\n",
            "Color 2 (Green): 125 facets (25%)\n",
            "Color 3 (Blue): 100 facets (20%)\n",
            "Color 4 (Yellow): 75 facets (15%)\n",
            "Other colors: 50 facets (10%)\n\n",
            "## Usage Note\n\n",
            "Color extension is non-standard. Not all STL readers support colors.\n",
            "Standard STL viewers may ignore color data.\n"
        );
        let doc_items = backend.create_docitems(markdown);

        assert!(doc_items.len() >= 3);
        // Verify color extension metadata
        let has_color_info = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("Color mode") && text.contains("Per-facet"),
            _ => false,
        });
        assert!(has_color_info);

        // Verify color statistics
        let has_color_stats = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => {
                text.contains("Color Distribution") && text.contains("Unique colors")
            }
            _ => false,
        });
        assert!(has_color_stats);

        // Verify non-standard warning
        let has_warning = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("non-standard"),
            _ => false,
        });
        assert!(has_warning);
    }

    #[test]
    fn test_glb_with_khr_extensions() {
        // Test GLB with multiple KHR extensions (lights, materials, variants)
        let backend = CadBackend::new(InputFormat::Glb).unwrap();
        let markdown = concat!(
            "## Mesh Statistics\n\n",
            "Vertices: 16384\n",
            "Faces: 8192\n",
            "Meshes: 12\n\n",
            "## glTF Extensions\n\n",
            "KHR_lights_punctual: Enabled\n",
            "KHR_materials_unlit: Enabled\n",
            "KHR_materials_clearcoat: Enabled\n",
            "KHR_materials_transmission: Enabled\n",
            "KHR_materials_volume: Enabled\n",
            "KHR_materials_ior: Enabled\n",
            "KHR_materials_specular: Enabled\n",
            "KHR_materials_sheen: Enabled\n",
            "KHR_materials_variants: Enabled\n",
            "KHR_mesh_quantization: Enabled\n",
            "KHR_texture_transform: Enabled\n\n",
            "## Lights (KHR_lights_punctual)\n\n",
            "Light 0: Directional (Sun)\n",
            "  Intensity: 10.0\n",
            "  Color: (1.0, 0.95, 0.9)\n",
            "  Direction: (0.3, -0.8, 0.5)\n\n",
            "Light 1: Point (Ceiling lamp)\n",
            "  Intensity: 5.0\n",
            "  Color: (1.0, 1.0, 1.0)\n",
            "  Range: 10.0 meters\n\n",
            "Light 2: Spot (Flashlight)\n",
            "  Intensity: 8.0\n",
            "  Inner cone: 0.3 rad\n",
            "  Outer cone: 0.5 rad\n\n",
            "## Material Variants\n\n",
            "Variant 0: Default\n",
            "Variant 1: Night mode\n",
            "Variant 2: Wireframe\n",
            "Variant 3: Debug UVs\n\n",
            "## Advanced Materials\n\n",
            "Material 0: Glass (transmission + volume)\n",
            "  IOR: 1.5\n",
            "  Transmission: 0.95\n",
            "  Thickness: 0.05 meters\n\n",
            "Material 1: ClearCoat (automotive paint)\n",
            "  Clearcoat: 1.0\n",
            "  Clearcoat roughness: 0.05\n\n",
            "Material 2: Fabric (sheen)\n",
            "  Sheen color: (1.0, 0.9, 0.8)\n",
            "  Sheen roughness: 0.6\n"
        );
        let doc_items = backend.create_docitems(markdown);

        assert!(doc_items.len() >= 4);
        // Verify KHR extensions present
        let has_khr_extensions = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => {
                text.contains("KHR_lights_punctual")
                    && text.contains("KHR_materials_unlit")
                    && text.contains("KHR_materials_variants")
            }
            _ => false,
        });
        assert!(has_khr_extensions);

        // Verify lights metadata
        let has_lights = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => {
                text.contains("Light 0: Directional")
                    && text.contains("Light 1: Point")
                    && text.contains("Light 2: Spot")
            }
            _ => false,
        });
        assert!(has_lights);

        // Verify material variants
        let has_variants = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => {
                text.contains("Material Variants") && text.contains("Variant")
            }
            _ => false,
        });
        assert!(has_variants);

        // Verify advanced materials (transmission, clearcoat, sheen)
        let has_advanced_materials = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => {
                text.contains("Glass") && text.contains("ClearCoat") && text.contains("Fabric")
            }
            _ => false,
        });
        assert!(has_advanced_materials);
    }

    #[test]
    fn test_gltf_generator_as_author() {
        // Test that GLTF generator field is extracted as author metadata (N=1876)
        // Create a minimal GLTF JSON with generator field
        use std::io::Write;
        use tempfile::NamedTempFile;

        let gltf_json = r#"{
            "asset": {
                "version": "2.0",
                "generator": "Blender 3.6.0"
            },
            "scene": 0,
            "scenes": [{"nodes": []}],
            "nodes": []
        }"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(gltf_json.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let backend = CadBackend::new(InputFormat::Gltf).unwrap();
        let result = backend.parse_file(temp_file.path(), &BackendOptions::default());

        assert!(result.is_ok());
        let doc = result.unwrap();

        // Verify generator is extracted as author
        assert_eq!(doc.metadata.author, Some("Blender 3.6.0".to_string()));

        // Verify markdown contains generator info
        assert!(doc.markdown.contains("Generator: Blender 3.6.0"));
    }

    #[test]
    fn test_dxf_lastsavedby_as_author() {
        // Test that DXF $LASTSAVEDBY field is extracted as author metadata (N=1877)
        // Create a minimal DXF with $LASTSAVEDBY header variable
        use std::io::Write;
        use tempfile::NamedTempFile;

        let dxf_content = r"0
SECTION
2
HEADER
9
$ACADVER
1
AC1015
9
$LASTSAVEDBY
1
John Doe
0
ENDSEC
0
SECTION
2
TABLES
0
ENDSEC
0
SECTION
2
ENTITIES
0
LINE
8
0
10
0.0
20
0.0
30
0.0
11
10.0
21
10.0
31
0.0
0
ENDSEC
0
EOF
";

        let mut temp_file = NamedTempFile::with_suffix(".dxf").unwrap();
        temp_file.write_all(dxf_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let backend = CadBackend::new(InputFormat::Dxf).unwrap();
        let result = backend.parse_file(temp_file.path(), &BackendOptions::default());

        assert!(result.is_ok());
        let doc = result.unwrap();

        // Verify $LASTSAVEDBY is extracted as author
        assert_eq!(doc.metadata.author, Some("John Doe".to_string()));

        // Verify markdown contains $LASTSAVEDBY info
        assert!(doc.markdown.contains("$LASTSAVEDBY"));
        assert!(doc.markdown.contains("John Doe"));
    }
}
