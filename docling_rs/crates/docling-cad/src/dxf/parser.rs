//! DXF file parser
//!
//! Parses DXF (Drawing Exchange Format) files using the dxf crate.
//! DXF is a CAD data file format developed by Autodesk for enabling data interoperability.

use anyhow::{Context, Result};
use dxf::entities::EntityType;
use dxf::enums::AcadVersion;
use dxf::Drawing;
use std::collections::HashMap;
use std::path::Path;

/// DXF drawing data
#[derive(Debug, Clone, PartialEq)]
pub struct DxfDrawing {
    /// Drawing name (from file name)
    pub name: Option<String>,
    /// DXF version (e.g., "AC1024" for `AutoCAD` 2010)
    pub version: String,
    /// DXF version code as it appears in the file (e.g., "AC1024")
    pub version_code: String,
    /// Total entity count
    pub entity_count: usize,
    /// Entity type counts
    pub entity_types: EntityTypeCounts,
    /// Text content extracted from TEXT and MTEXT entities
    pub text_content: Vec<String>,
    /// Layer names
    pub layer_names: Vec<String>,
    /// Bounding box (if calculable)
    pub bbox: Option<BoundingBox>,
    /// Key header variables from the DXF file
    pub header_vars: DxfHeaderVars,
}

/// Key DXF header variables
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DxfHeaderVars {
    /// $ACADVER - `AutoCAD` drawing database version number
    pub acad_ver: Option<String>,
    /// $ACADMAINTVER - Maintenance version number
    pub acad_maint_ver: Option<i16>,
    /// $DWGCODEPAGE - Drawing code page
    pub dwg_codepage: Option<String>,
    /// $LASTSAVEDBY - Name of the last user who saved the file
    pub last_saved_by: Option<String>,
    /// $INSUNITS - Default drawing units for `AutoCAD` `DesignCenter` blocks
    pub ins_units: Option<i16>,
    /// $MEASUREMENT - Units format for automatic scaling (0=English, 1=Metric)
    pub measurement: Option<i16>,
    /// $EXTMIN - Drawing extents minimum point (X, Y, Z)
    pub ext_min: Option<(f64, f64, f64)>,
    /// $EXTMAX - Drawing extents maximum point (X, Y, Z)
    pub ext_max: Option<(f64, f64, f64)>,
    /// $LIMMIN - Drawing limits minimum point (X, Y)
    pub lim_min: Option<(f64, f64)>,
    /// $LIMMAX - Drawing limits maximum point (X, Y)
    pub lim_max: Option<(f64, f64)>,
    /// $ORTHOMODE - Ortho mode on/off
    pub ortho_mode: Option<bool>,
    /// $REGENMODE - REGENAUTO mode on/off
    pub regen_mode: Option<bool>,
    /// $FILLMODE - Fill mode on/off
    pub fill_mode: Option<bool>,
    /// $QTEXTMODE - Quick text mode on/off
    pub qtext_mode: Option<bool>,
    /// $MIRRTEXT - Mirror text on/off
    pub mirror_text: Option<bool>,
    /// $LTSCALE - Global line type scale
    pub ltscale: Option<f64>,
    /// $ATTMODE - Attribute visibility (0=none, 1=normal, 2=all)
    pub att_mode: Option<i16>,
    /// $TEXTSIZE - Default text height
    pub text_size: Option<f64>,
    /// $TRACEWID - Default trace width
    pub trace_wid: Option<f64>,
    /// $TEXTSTYLE - Current text style name
    pub text_style: Option<String>,
    /// $CLAYER - Current layer name
    pub clayer: Option<String>,
    /// $INSBASE - Insertion base point (X, Y, Z)
    pub ins_base: Option<(f64, f64, f64)>,
    /// $CELTYPE - Current entity line type
    pub celtype: Option<String>,
    /// $DIM* - Dimension style variables (parsed from raw file)
    /// Map of variable name (without $ prefix) to string value
    pub dim_vars: HashMap<String, String>,
}

/// Entity type counts
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct EntityTypeCounts {
    /// Number of LINE entities
    pub lines: usize,
    /// Number of CIRCLE entities
    pub circles: usize,
    /// Number of ARC entities
    pub arcs: usize,
    /// Number of POLYLINE/LWPOLYLINE entities
    pub polylines: usize,
    /// Number of TEXT entities
    pub text: usize,
    /// Number of MTEXT (multiline text) entities
    pub mtext: usize,
    /// Number of POINT entities
    pub points: usize,
    /// Number of SPLINE entities
    pub splines: usize,
    /// Number of ELLIPSE entities
    pub ellipses: usize,
    /// Number of DIMENSION entities
    pub dimensions: usize,
    /// Number of BLOCK entities
    pub blocks: usize,
    /// Number of INSERT (block reference) entities
    pub inserts: usize,
    /// Number of other/unclassified entities
    pub other: usize,
}

/// 2D/3D bounding box
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundingBox {
    /// Minimum X coordinate
    pub min_x: f64,
    /// Minimum Y coordinate
    pub min_y: f64,
    /// Minimum Z coordinate
    pub min_z: f64,
    /// Maximum X coordinate
    pub max_x: f64,
    /// Maximum Y coordinate
    pub max_y: f64,
    /// Maximum Z coordinate
    pub max_z: f64,
}

impl BoundingBox {
    #[inline]
    const fn new() -> Self {
        Self {
            min_x: f64::INFINITY,
            min_y: f64::INFINITY,
            min_z: f64::INFINITY,
            max_x: f64::NEG_INFINITY,
            max_y: f64::NEG_INFINITY,
            max_z: f64::NEG_INFINITY,
        }
    }

    #[inline]
    const fn update(&mut self, x: f64, y: f64, z: f64) {
        self.min_x = self.min_x.min(x);
        self.min_y = self.min_y.min(y);
        self.min_z = self.min_z.min(z);
        self.max_x = self.max_x.max(x);
        self.max_y = self.max_y.max(y);
        self.max_z = self.max_z.max(z);
    }

    #[inline]
    const fn is_valid(&self) -> bool {
        self.min_x.is_finite() && self.max_x.is_finite()
    }

    /// Get bounding box width (X dimension)
    ///
    /// # Returns
    ///
    /// Width of the bounding box in DXF drawing units.
    #[inline]
    #[must_use = "width returns bounding box X dimension"]
    pub const fn width(&self) -> f64 {
        self.max_x - self.min_x
    }

    /// Get bounding box height (Y dimension)
    ///
    /// # Returns
    ///
    /// Height of the bounding box in DXF drawing units.
    #[inline]
    #[must_use = "height returns bounding box Y dimension"]
    pub const fn height(&self) -> f64 {
        self.max_y - self.min_y
    }

    /// Get bounding box depth (Z dimension)
    ///
    /// # Returns
    ///
    /// Depth of the bounding box in DXF drawing units.
    #[inline]
    #[must_use = "depth returns bounding box Z dimension"]
    pub const fn depth(&self) -> f64 {
        self.max_z - self.min_z
    }
}

/// DXF parser
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct DxfParser;

impl DxfParser {
    /// Parse DXF file from path
    ///
    /// Reads and parses a DXF (Drawing Exchange Format) file, extracting entities,
    /// layers, text content, header variables, and bounding box information.
    /// Supports various DXF versions from `AutoCAD` R12 onwards.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the DXF file
    ///
    /// # Returns
    ///
    /// Parsed DXF drawing with entity counts, text content, layers, and metadata.
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be opened or contains invalid DXF data.
    #[must_use = "parsing produces a result that should be handled"]
    pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<DxfDrawing> {
        let path = path.as_ref();
        let drawing = Drawing::load_file(path)
            .with_context(|| format!("Failed to load DXF file: {}", path.display()))?;

        let name = Self::extract_name_from_path(path);

        // Parse variables from raw file (dxf crate doesn't expose all of them)
        let raw_content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read DXF file: {}", path.display()))?;
        let dim_vars = Self::extract_dim_variables_from_str(&raw_content);
        let (ins_base, celtype) = Self::extract_misc_header_vars_from_str(&raw_content);

        Self::parse_drawing(&drawing, name, dim_vars, ins_base, celtype)
    }

    /// Parse DXF from string
    ///
    /// Parses DXF data from a string, extracting entities, layers, text content,
    /// header variables, and bounding box information.
    ///
    /// # Arguments
    ///
    /// * `content` - DXF file content as string
    /// * `name` - Optional name for the drawing (typically filename without extension)
    ///
    /// # Returns
    ///
    /// Parsed DXF drawing with entity counts, text content, layers, and metadata.
    ///
    /// # Errors
    ///
    /// Returns error if the string contains invalid DXF syntax.
    #[must_use = "parsing produces a result that should be handled"]
    pub fn parse_str(content: &str, name: Option<String>) -> Result<DxfDrawing> {
        let drawing = Drawing::load(&mut std::io::Cursor::new(content.as_bytes()))
            .context("Failed to parse DXF string")?;

        // Parse variables from raw content
        let dim_vars = Self::extract_dim_variables_from_str(content);
        let (ins_base, celtype) = Self::extract_misc_header_vars_from_str(content);

        Self::parse_drawing(&drawing, name, dim_vars, ins_base, celtype)
    }

    /// Parse a loaded Drawing
    #[allow(clippy::unnecessary_wraps, reason = "Result kept for API consistency")]
    #[allow(clippy::too_many_lines)] // Complex DXF parsing - keeping together for clarity
    fn parse_drawing(
        drawing: &Drawing,
        name: Option<String>,
        dim_vars: HashMap<String, String>,
        ins_base: Option<(f64, f64, f64)>,
        celtype: Option<String>,
    ) -> Result<DxfDrawing> {
        // Extract version code (raw string like "AC1024")
        let version_code = Self::get_version_code(drawing.header.version);
        // Human-readable version (e.g., "AutoCAD R2010")
        let version = format!("{:?}", drawing.header.version);

        // Extract key header variables
        // Note: dxf crate uses Rust field names (without $prefix)
        let header_vars = DxfHeaderVars {
            acad_ver: Some(version_code.clone()),
            acad_maint_ver: Some(drawing.header.maintenance_version),
            dwg_codepage: Some(drawing.header.drawing_code_page.clone()),
            last_saved_by: Some(drawing.header.last_saved_by.clone()),
            ins_units: Some(drawing.header.default_drawing_units as i16),
            // $MEASUREMENT field name varies - leaving as None for now
            measurement: None,
            // Drawing extents (bounding box from header)
            ext_min: Some((
                drawing.header.minimum_drawing_extents.x,
                drawing.header.minimum_drawing_extents.y,
                drawing.header.minimum_drawing_extents.z,
            )),
            ext_max: Some((
                drawing.header.maximum_drawing_extents.x,
                drawing.header.maximum_drawing_extents.y,
                drawing.header.maximum_drawing_extents.z,
            )),
            // Drawing limits (2D limits)
            lim_min: Some((
                drawing.header.minimum_drawing_limits.x,
                drawing.header.minimum_drawing_limits.y,
            )),
            lim_max: Some((
                drawing.header.maximum_drawing_limits.x,
                drawing.header.maximum_drawing_limits.y,
            )),
            // Drawing modes and settings
            ortho_mode: Some(drawing.header.draw_orthogonal_lines),
            regen_mode: Some(drawing.header.use_regen_mode),
            fill_mode: Some(drawing.header.fill_mode_on),
            qtext_mode: Some(drawing.header.use_quick_text_mode),
            mirror_text: Some(drawing.header.mirror_text),
            ltscale: Some(drawing.header.line_type_scale),
            att_mode: Some(drawing.header.attribute_visibility as i16),
            text_size: Some(drawing.header.default_text_height),
            trace_wid: Some(drawing.header.trace_width),
            text_style: Some(drawing.header.text_style.clone()),
            clayer: Some(drawing.header.current_layer.clone()),
            ins_base,
            celtype,
            dim_vars,
        };

        // Count entities and extract text
        let mut entity_types = EntityTypeCounts::default();
        let mut text_content = Vec::new();
        let mut bbox = BoundingBox::new();

        for entity in drawing.entities() {
            match entity.specific {
                EntityType::Line(ref line) => {
                    entity_types.lines += 1;
                    bbox.update(line.p1.x, line.p1.y, line.p1.z);
                    bbox.update(line.p2.x, line.p2.y, line.p2.z);
                }
                EntityType::Circle(ref circle) => {
                    entity_types.circles += 1;
                    bbox.update(circle.center.x, circle.center.y, circle.center.z);
                }
                EntityType::Arc(ref arc) => {
                    entity_types.arcs += 1;
                    bbox.update(arc.center.x, arc.center.y, arc.center.z);
                }
                EntityType::Polyline(_) | EntityType::LwPolyline(_) => {
                    entity_types.polylines += 1;
                }
                EntityType::Text(ref text) => {
                    entity_types.text += 1;
                    if !text.value.is_empty() {
                        text_content.push(text.value.clone());
                    }
                    bbox.update(text.location.x, text.location.y, text.location.z);
                }
                EntityType::MText(ref mtext) => {
                    entity_types.mtext += 1;
                    if !mtext.text.is_empty() {
                        text_content.push(mtext.text.clone());
                    }
                    bbox.update(
                        mtext.insertion_point.x,
                        mtext.insertion_point.y,
                        mtext.insertion_point.z,
                    );
                }
                EntityType::ModelPoint(ref point) => {
                    entity_types.points += 1;
                    bbox.update(point.location.x, point.location.y, point.location.z);
                }
                EntityType::Spline(ref spline) => {
                    entity_types.splines += 1;
                    for cp in &spline.control_points {
                        bbox.update(cp.x, cp.y, cp.z);
                    }
                }
                EntityType::Ellipse(ref ellipse) => {
                    entity_types.ellipses += 1;
                    bbox.update(ellipse.center.x, ellipse.center.y, ellipse.center.z);
                }
                EntityType::RotatedDimension(_)
                | EntityType::RadialDimension(_)
                | EntityType::DiameterDimension(_)
                | EntityType::AngularThreePointDimension(_)
                | EntityType::OrdinateDimension(_) => {
                    entity_types.dimensions += 1;
                }
                EntityType::Insert(_) => {
                    entity_types.inserts += 1;
                }
                _ => {
                    entity_types.other += 1;
                }
            }
        }

        // Extract layer names
        let layer_names: Vec<String> = drawing.layers().map(|layer| layer.name.clone()).collect();

        // Count blocks (must be done BEFORE calculating entity_count)
        entity_types.blocks = drawing.blocks().count();

        // Calculate total entity count from ENTITIES section only
        // (Blocks are in BLOCKS section and counted separately)
        let entity_count = entity_types.lines
            + entity_types.circles
            + entity_types.arcs
            + entity_types.polylines
            + entity_types.text
            + entity_types.mtext
            + entity_types.points
            + entity_types.splines
            + entity_types.ellipses
            + entity_types.dimensions
            + entity_types.inserts
            + entity_types.other;

        Ok(DxfDrawing {
            name,
            version,
            version_code,
            entity_count,
            entity_types,
            text_content,
            layer_names,
            bbox: bbox.is_valid().then_some(bbox),
            header_vars,
        })
    }

    /// Extract drawing name from file path
    fn extract_name_from_path(path: &Path) -> Option<String> {
        path.file_stem()
            .and_then(|s| s.to_str())
            .map(std::string::ToString::to_string)
    }

    /// Get the raw version code string from `AcadVersion` enum
    /// Maps enum variants to their raw DXF $ACADVER values (e.g., "AC1024" for R2010")
    #[inline]
    fn get_version_code(version: AcadVersion) -> String {
        use AcadVersion::{
            Version_1_0, Version_1_2, Version_1_40, Version_2_05, Version_2_10, Version_2_21,
            Version_2_22, Version_2_5, Version_2_6, R10, R11, R12, R13, R14, R2000, R2004, R2007,
            R2010, R2013, R2018, R9,
        };
        match version {
            Version_1_0 => "MC0.0",
            Version_1_2 => "AC1.2",
            Version_1_40 => "AC1.40",
            Version_2_05 => "AC1.50",
            Version_2_10 => "AC2.10",
            Version_2_21 => "AC2.21",
            Version_2_22 => "AC2.22",
            Version_2_5 => "AC1002",
            Version_2_6 => "AC1003",
            R9 => "AC1004",
            R10 => "AC1006",
            R11 | R12 => "AC1009",
            R13 => "AC1012",
            R14 => "AC1014",
            R2000 => "AC1015",
            R2004 => "AC1018",
            R2007 => "AC1021",
            R2010 => "AC1024",
            R2013 => "AC1027",
            R2018 => "AC1032",
        }
        .to_string()
    }

    /// Extract misc header variables not exposed by dxf crate
    /// Returns: (`ins_base`, celtype)
    fn extract_misc_header_vars_from_str(
        content: &str,
    ) -> (Option<(f64, f64, f64)>, Option<String>) {
        let lines: Vec<&str> = content.lines().collect();
        let mut ins_base = None;
        let mut celtype = None;

        let mut i = 0;
        while i < lines.len() {
            let line = lines[i].trim();

            if line == "$INSBASE" && i + 6 < lines.len() {
                // Format: $INSBASE, 10, x_value, 20, y_value, 30, z_value
                if lines[i + 1].trim() == "10"
                    && lines[i + 3].trim() == "20"
                    && lines[i + 5].trim() == "30"
                {
                    if let (Ok(x), Ok(y), Ok(z)) = (
                        lines[i + 2].trim().parse::<f64>(),
                        lines[i + 4].trim().parse::<f64>(),
                        lines[i + 6].trim().parse::<f64>(),
                    ) {
                        ins_base = Some((x, y, z));
                    }
                }
            } else if line == "$CELTYPE" && i + 2 < lines.len() {
                // Format: $CELTYPE, group_code, value
                celtype = Some(lines[i + 2].trim().to_string());
            }

            i += 1;
        }

        (ins_base, celtype)
    }

    /// Extract $DIM* variables from DXF content string
    fn extract_dim_variables_from_str(content: &str) -> HashMap<String, String> {
        let mut dim_vars = HashMap::new();
        let lines: Vec<&str> = content.lines().collect();

        let mut i = 0;
        while i < lines.len() {
            let line = lines[i].trim();

            // Check if this is a $DIM variable
            if line.starts_with("$DIM") {
                let var_name = line.trim_start_matches('$');

                // DXF format for header variables:
                // Line N: $VARIABLE_NAME
                // Line N+1: group_code (e.g., "280")
                // Line N+2: value (e.g., "2")
                // Line N+3: next variable or "  9" (section separator)

                // Read group code and value pairs
                // Most dimension variables have a single group code/value pair
                let mut j = i + 1;
                let mut value_str = String::new();
                let mut found_value = false;

                while j + 1 < lines.len() {
                    let group_code = lines[j].trim();
                    let value = lines[j + 1].trim();

                    // Check if we've reached the next variable or section end
                    if group_code.starts_with('$') || value.starts_with('$') {
                        break;
                    }

                    // Group code 9 precedes variable names (e.g., "  9" followed by "$DIMSCALE")
                    // If we see group code 9 with a $ variable name, we've hit the next variable
                    if group_code == "9" && value.starts_with('$') {
                        break;
                    }

                    // Store the value (use first value, even if empty)
                    if !found_value {
                        value_str = value.to_string();
                        found_value = true;
                    }

                    j += 2; // Skip to next group code/value pair
                }

                // Store the variable (even if empty - DXF spec allows empty values)
                if found_value {
                    dim_vars.insert(var_name.to_string(), value_str);
                }

                i = j; // Skip to where we left off
            } else {
                i += 1;
            }
        }

        dim_vars
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIMPLE_DXF: &str = r"0
SECTION
2
HEADER
9
$ACADVER
1
AC1015
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
CIRCLE
8
0
10
5.0
20
5.0
30
0.0
40
2.5
0
TEXT
8
0
10
1.0
20
1.0
30
0.0
40
0.5
1
Test Drawing
0
ENDSEC
0
EOF
";

    #[test]
    fn test_parse_simple_dxf() {
        let drawing = DxfParser::parse_str(SIMPLE_DXF, Some("test".to_string())).unwrap();
        assert_eq!(drawing.name, Some("test".to_string()));
        assert_eq!(drawing.entity_types.lines, 1);
        assert_eq!(drawing.entity_types.circles, 1);
        assert_eq!(drawing.entity_types.text, 1);
        assert_eq!(drawing.text_content.len(), 1);
        assert_eq!(drawing.text_content[0], "Test Drawing");
    }

    #[test]
    fn test_bounding_box() {
        let drawing = DxfParser::parse_str(SIMPLE_DXF, None).unwrap();
        assert!(drawing.bbox.is_some());
        let bbox = drawing.bbox.unwrap();
        assert!(bbox.min_x <= 0.0);
        assert!(bbox.max_x >= 10.0);
    }

    #[test]
    fn test_entity_count() {
        let drawing = DxfParser::parse_str(SIMPLE_DXF, None).unwrap();
        assert_eq!(drawing.entity_count, 3); // 1 line + 1 circle + 1 text
    }

    #[test]
    fn test_floor_plan_dim_vars() {
        let test_file = "../../test-corpus/cad/dxf/floor_plan.dxf";
        let drawing = DxfParser::parse_file(test_file).unwrap();

        // Check for dimension variables
        println!("Total dim vars: {}", drawing.header_vars.dim_vars.len());

        // Check specific variables (including ones that were previously missing)
        assert!(
            drawing.header_vars.dim_vars.contains_key("DIMLTYPE"),
            "$DIMLTYPE missing"
        );
        assert!(
            drawing.header_vars.dim_vars.contains_key("DIMLTEX1"),
            "$DIMLTEX1 missing"
        );
        assert!(
            drawing.header_vars.dim_vars.contains_key("DIMLTEX2"),
            "$DIMLTEX2 missing"
        );
        // Variables that were missing before the fix (N=2192)
        assert!(
            drawing.header_vars.dim_vars.contains_key("DIMASSOC"),
            "$DIMASSOC missing"
        );
        assert!(
            drawing.header_vars.dim_vars.contains_key("DIMSCALE"),
            "$DIMSCALE missing"
        );

        // Print EXTMIN/EXTMAX
        if let Some((x, y, z)) = drawing.header_vars.ext_min {
            println!("$EXTMIN: ({x}, {y}, {z})");
        }
        if let Some((x, y, z)) = drawing.header_vars.ext_max {
            println!("$EXTMAX: ({x}, {y}, {z})");
        }
    }
}
