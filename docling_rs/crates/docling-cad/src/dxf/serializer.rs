//! DXF markdown serializer
//!
//! Converts DXF drawing data to markdown format for document processing.

use super::parser::{DxfDrawing, DxfHeaderVars};
use std::fmt::Write;

// ============================================================================
// Helper functions for to_markdown - extracted to reduce complexity
// ============================================================================

/// Format coordinate value (scientific notation for large values)
#[inline]
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

/// Write title section
fn write_title(md: &mut String, name: Option<&str>) {
    if let Some(name) = name {
        let _ = writeln!(md, "# DXF Drawing: {name}");
        md.push('\n');
    } else {
        md.push_str("# DXF Drawing\n\n");
    }
}

/// Write file information section
fn write_file_info(md: &mut String, drawing: &DxfDrawing) {
    md.push_str("## File Information\n\n");
    md.push_str("- **Format**: DXF (Drawing Exchange Format)\n");
    let _ = writeln!(
        md,
        "- **DXF Version**: {} (AutoCAD {})",
        drawing.version_code,
        drawing.version.replace('R', "")
    );
    let _ = writeln!(md, "- **Total Entities**: {}", drawing.entity_count);
}

/// Write header variables section
fn write_header_vars(md: &mut String, vars: &DxfHeaderVars) {
    md.push_str("\n## Header Variables\n\n");

    // Version info
    if let Some(ref acad_ver) = vars.acad_ver {
        let _ = writeln!(md, "- **$ACADVER**: {acad_ver}");
    }
    if let Some(acad_maint_ver) = vars.acad_maint_ver {
        let _ = writeln!(md, "- **$ACADMAINTVER**: {acad_maint_ver}");
    }

    // String variables
    write_opt_str_var(md, "$DWGCODEPAGE", vars.dwg_codepage.as_deref());
    write_opt_str_var(md, "$LASTSAVEDBY", vars.last_saved_by.as_deref());

    // Numeric variables
    if let Some(ins_units) = vars.ins_units {
        let _ = writeln!(md, "- **$INSUNITS**: {ins_units}");
    }
    if let Some(measurement) = vars.measurement {
        let units = if measurement == 0 {
            "English"
        } else {
            "Metric"
        };
        let _ = writeln!(md, "- **$MEASUREMENT**: {measurement} ({units})");
    }

    // Mode flags
    write_bool_var(md, "$ORTHOMODE", vars.ortho_mode);
    write_bool_var(md, "$REGENMODE", vars.regen_mode);
    write_bool_var(md, "$FILLMODE", vars.fill_mode);
    write_bool_var(md, "$QTEXTMODE", vars.qtext_mode);
    write_bool_var(md, "$MIRRTEXT", vars.mirror_text);

    // Float variables
    if let Some(ltscale) = vars.ltscale {
        let _ = writeln!(md, "- **$LTSCALE**: {ltscale:.3}");
    }
    if let Some(att_mode) = vars.att_mode {
        let mode_str = match att_mode {
            0 => "0 (none)",
            1 => "1 (normal)",
            2 => "2 (all)",
            _ => &format!("{att_mode}"),
        };
        let _ = writeln!(md, "- **$ATTMODE**: {mode_str}");
    }
    if let Some(text_size) = vars.text_size {
        let _ = writeln!(md, "- **$TEXTSIZE**: {text_size:.3}");
    }
    if let Some(trace_wid) = vars.trace_wid {
        let _ = writeln!(md, "- **$TRACEWID**: {trace_wid:.3}");
    }

    // More string variables
    write_opt_str_var(md, "$TEXTSTYLE", vars.text_style.as_deref());
    write_opt_str_var(md, "$CLAYER", vars.clayer.as_deref());

    if let Some((x, y, z)) = vars.ins_base {
        let _ = writeln!(md, "- **$INSBASE**: ({x:.3}, {y:.3}, {z:.3})");
    }
    write_opt_str_var(md, "$CELTYPE", vars.celtype.as_deref());
}

/// Helper to write optional string variable
#[inline]
fn write_opt_str_var(md: &mut String, name: &str, value: Option<&str>) {
    if let Some(v) = value {
        if !v.is_empty() {
            let _ = writeln!(md, "- **{name}**: {v}");
        }
    }
}

/// Helper to write boolean variable
#[inline]
fn write_bool_var(md: &mut String, name: &str, value: Option<bool>) {
    if let Some(v) = value {
        let _ = writeln!(md, "- **{name}**: {}", if v { "1 (on)" } else { "0 (off)" });
    }
}

/// Write dimension style variables section
fn write_dim_vars(md: &mut String, vars: &DxfHeaderVars) {
    if vars.dim_vars.is_empty() {
        return;
    }

    md.push_str("\n## Dimension Style Variables\n\n");
    let _ = writeln!(
        md,
        "Complete list of {} dimension style variables from the HEADER section:",
        vars.dim_vars.len()
    );
    md.push('\n');

    let mut dim_vars: Vec<(&String, &String)> = vars.dim_vars.iter().collect();
    dim_vars.sort_by_key(|(k, _)| *k);

    for (name, value) in dim_vars {
        if value.is_empty() {
            let _ = writeln!(md, "- **${name}**: (empty)");
        } else {
            let _ = writeln!(md, "- **${name}**: {value}");
        }
    }
}

/// Write drawing extents section
fn write_extents(md: &mut String, drawing: &DxfDrawing) {
    md.push_str("\n## Drawing Extents\n\n");

    // Check for sentinel values
    let has_sentinel = check_sentinel_values(&drawing.header_vars);

    // Header section extents
    md.push_str("### From HEADER Section\n\n");
    if let Some((x, y, z)) = drawing.header_vars.ext_min {
        let _ = writeln!(
            md,
            "- **$EXTMIN**: ({}, {}, {})",
            format_coord(x),
            format_coord(y),
            format_coord(z)
        );
    }
    if let Some((x, y, z)) = drawing.header_vars.ext_max {
        let _ = writeln!(
            md,
            "- **$EXTMAX**: ({}, {}, {})",
            format_coord(x),
            format_coord(y),
            format_coord(z)
        );
    }

    // Calculated extents if header has sentinel values
    if has_sentinel {
        if let Some(ref bbox) = drawing.bbox {
            md.push_str("\n### Calculated From Entities\n\n");
            md.push_str("*Note: Header values are sentinel/uninitialized. Actual extents calculated from entity geometry:*\n\n");
            let _ = writeln!(
                md,
                "- **EXTMIN (actual)**: ({:.3}, {:.3}, {:.3})",
                bbox.min_x, bbox.min_y, bbox.min_z
            );
            let _ = writeln!(
                md,
                "- **EXTMAX (actual)**: ({:.3}, {:.3}, {:.3})",
                bbox.max_x, bbox.max_y, bbox.max_z
            );
        }
    }

    // Drawing limits
    md.push_str("\n### Drawing Limits\n\n");
    if let Some((x, y)) = drawing.header_vars.lim_min {
        let _ = writeln!(md, "- **$LIMMIN**: ({x:.3}, {y:.3})");
    }
    if let Some((x, y)) = drawing.header_vars.lim_max {
        let _ = writeln!(md, "- **$LIMMAX**: ({x:.3}, {y:.3})");
    }
}

/// Check if header extents have sentinel values
#[inline]
fn check_sentinel_values(vars: &DxfHeaderVars) -> bool {
    if let (Some((min_x, min_y, min_z)), Some((max_x, max_y, max_z))) = (vars.ext_min, vars.ext_max)
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

/// Write entity statistics table
fn write_entity_stats(md: &mut String, drawing: &DxfDrawing) {
    md.push_str("\n## Entity Statistics\n\n");
    let _ = writeln!(
        md,
        "Breakdown of {} entities from the ENTITIES section:\n",
        drawing.entity_count
    );

    md.push_str("| Entity Type | Count |\n");
    md.push_str("|-------------|-------|\n");

    let types = &drawing.entity_types;
    write_entity_row(md, "Lines", types.lines);
    write_entity_row(md, "Circles", types.circles);
    write_entity_row(md, "Arcs", types.arcs);
    write_entity_row(md, "Polylines", types.polylines);
    write_entity_row(md, "Text", types.text);
    write_entity_row(md, "MText", types.mtext);
    write_entity_row(md, "Points", types.points);
    write_entity_row(md, "Splines", types.splines);
    write_entity_row(md, "Ellipses", types.ellipses);
    write_entity_row(md, "Dimensions", types.dimensions);
    write_entity_row(md, "Blocks", types.blocks);
    write_entity_row(md, "Block Inserts", types.inserts);
    write_entity_row(md, "Other", types.other);
}

/// Write entity table row if count > 0
#[inline]
fn write_entity_row(md: &mut String, name: &str, count: usize) {
    if count > 0 {
        let _ = writeln!(md, "| {name} | {count} |");
    }
}

/// Write layers section
fn write_layers(md: &mut String, layer_names: &[String]) {
    if layer_names.is_empty() {
        return;
    }

    md.push_str("\n## Layers\n\n");
    md.push_str("Layer organization from the TABLES section:\n\n");
    let _ = writeln!(md, "- **Count**: {}", layer_names.len());
    if layer_names.len() <= 20 {
        md.push_str("- **Names**: ");
        md.push_str(&layer_names.join(", "));
        md.push('\n');
    }
}

/// Write drawing dimensions from bounding box
fn write_dimensions(md: &mut String, drawing: &DxfDrawing) {
    if let Some(ref bbox) = drawing.bbox {
        md.push_str("\n## Drawing Dimensions\n\n");
        let _ = writeln!(md, "- **Width** (X): {:.3}", bbox.width());
        let _ = writeln!(md, "- **Height** (Y): {:.3}", bbox.height());
        if bbox.depth().abs() > 0.001 {
            let _ = writeln!(md, "- **Depth** (Z): {:.3}", bbox.depth());
        }
    }
}

/// Write text content section
fn write_text_content(md: &mut String, text_content: &[String]) {
    if text_content.is_empty() {
        return;
    }

    md.push_str("\n## Text Content\n\n");
    for (i, text) in text_content.iter().enumerate() {
        if i < 50 {
            let _ = writeln!(md, "{}. {}", i + 1, text);
        }
    }
    if text_content.len() > 50 {
        let _ = writeln!(
            md,
            "\n*... and {} more text entities*",
            text_content.len() - 50
        );
    }
}

/// Write drawing description section
fn write_description(md: &mut String, drawing: &DxfDrawing) {
    md.push_str("\n## Drawing Description\n\n");

    let _ = write!(
        md,
        "This CAD drawing contains {} entities across {} layers. ",
        drawing.entity_count,
        drawing.layer_names.len()
    );

    let types = &drawing.entity_types;
    if types.text + types.mtext > 0 {
        let _ = write!(
            md,
            "The drawing includes {} text annotations. ",
            types.text + types.mtext
        );
    }

    if types.dimensions > 0 {
        let _ = write!(md, "There are {} dimension annotations. ", types.dimensions);
    }

    let _ = writeln!(
        md,
        "The drawing format is DXF version {} (AutoCAD {}).",
        drawing.version_code,
        drawing.version.replace('R', "")
    );
}

// ============================================================================
// Main API
// ============================================================================

/// Convert DXF drawing to markdown representation
#[must_use = "serialization returns markdown string"]
pub fn to_markdown(drawing: &DxfDrawing) -> String {
    let mut md = String::new();

    write_title(&mut md, drawing.name.as_deref());
    write_file_info(&mut md, drawing);
    write_header_vars(&mut md, &drawing.header_vars);
    write_dim_vars(&mut md, &drawing.header_vars);
    write_extents(&mut md, drawing);
    write_entity_stats(&mut md, drawing);
    write_layers(&mut md, &drawing.layer_names);
    write_dimensions(&mut md, drawing);
    write_text_content(&mut md, &drawing.text_content);
    write_description(&mut md, drawing);

    md
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dxf::parser::DxfParser;

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
    fn test_to_markdown() {
        let drawing = DxfParser::parse_str(SIMPLE_DXF, Some("test".to_string())).unwrap();
        let md = to_markdown(&drawing);

        assert!(md.contains("# DXF Drawing: test"));
        assert!(md.contains("**Format**: DXF"));
        // Entity stats are now in table format after N=2298 improvements
        assert!(md.contains("| Lines | 1 |"));
        assert!(md.contains("| Circles | 1 |"));
        assert!(md.contains("| Text | 1 |"));
        assert!(md.contains("Test Drawing"));
    }

    #[test]
    fn test_markdown_structure() {
        let drawing = DxfParser::parse_str(SIMPLE_DXF, Some("test".to_string())).unwrap();
        let md = to_markdown(&drawing);

        // Check for expected sections
        assert!(md.contains("## File Information"));
        assert!(md.contains("## Header Variables"));
        assert!(md.contains("## Drawing Extents"));
        assert!(md.contains("## Entity Statistics"));
        assert!(md.contains("## Drawing Dimensions"));
        assert!(md.contains("## Text Content"));
        assert!(md.contains("## Drawing Description"));
    }

    #[test]
    fn test_floor_plan_serialization() {
        let test_file = "../../test-corpus/cad/dxf/floor_plan.dxf";
        let drawing = DxfParser::parse_file(test_file).unwrap();
        let md = to_markdown(&drawing);

        println!("Generated markdown:\n{md}");

        // Check that dimension variables are included
        assert!(md.contains("$DIMLTYPE"), "Missing $DIMLTYPE in output");
        assert!(md.contains("$DIMLTEX1"), "Missing $DIMLTEX1 in output");
        assert!(md.contains("$DIMLTEX2"), "Missing $DIMLTEX2 in output");
        // Variables that were missing before the fix (N=2192)
        assert!(md.contains("$DIMASSOC"), "Missing $DIMASSOC in output");
        assert!(md.contains("$DIMSCALE"), "Missing $DIMSCALE in output");

        // Check that EXTMIN/EXTMAX are in scientific notation
        // Should be "1e+20" or "1e20", not "100000000000000000000.000"
        assert!(
            md.contains("1e+20") || md.contains("1e20"),
            "EXTMIN/EXTMAX should be in scientific notation, got:\n{md}"
        );
    }
}
