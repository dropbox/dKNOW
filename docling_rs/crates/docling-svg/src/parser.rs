//! SVG document parser
//!
//! Parses SVG files (XML format) to extract text content and shapes

use crate::element::{SvgShape, SvgTextElement};
use crate::error::{Result, SvgError};
use crate::metadata::SvgMetadata;
use quick_xml::events::Event;
use quick_xml::Reader;
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Parsed SVG document
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SvgDocument {
    /// Document metadata
    pub metadata: SvgMetadata,

    /// Text elements extracted from the SVG
    pub text_elements: Vec<SvgTextElement>,

    /// Shape elements extracted from the SVG
    pub shapes: Vec<SvgShape>,
}

/// Parse SVG file from path
///
/// # Errors
///
/// Returns an error if:
/// - The file cannot be opened or read (`SvgError::Io`)
/// - The content is not valid XML (`SvgError::XmlError`)
#[must_use = "parsing produces a result that should be handled"]
pub fn parse_svg(path: &Path) -> Result<SvgDocument> {
    let mut file = File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    parse_svg_str(&content)
}

/// Helper struct for attribute parsing
#[derive(Debug, Clone, PartialEq, Eq)]
struct AttrMap {
    attrs: std::collections::HashMap<String, String>,
}

impl AttrMap {
    #[inline]
    fn from_event(e: &quick_xml::events::BytesStart<'_>) -> Self {
        let mut attrs = std::collections::HashMap::new();
        for attr in e.attributes().flatten() {
            let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
            let value = String::from_utf8_lossy(&attr.value).to_string();
            attrs.insert(key, value);
        }
        Self { attrs }
    }

    #[inline]
    fn get(&self, key: &str) -> Option<&str> {
        self.attrs.get(key).map(String::as_str)
    }

    #[inline]
    fn get_f64(&self, key: &str) -> f64 {
        self.get(key).and_then(|v| v.parse().ok()).unwrap_or(0.0)
    }

    #[inline]
    fn get_f64_opt(&self, key: &str) -> Option<f64> {
        self.get(key).and_then(|v| v.parse().ok())
    }

    #[inline]
    fn get_string(&self, key: &str) -> Option<String> {
        self.get(key).map(str::to_string)
    }
}

/// Parse SVG metadata from svg element attributes
#[inline]
fn parse_svg_metadata(attrs: &AttrMap, metadata: &mut SvgMetadata) {
    metadata.width = attrs.get_string("width");
    metadata.height = attrs.get_string("height");
    metadata.viewbox = attrs.get_string("viewBox");
}

/// Parse text element from attributes
#[inline]
fn parse_text_element(attrs: &AttrMap) -> SvgTextElement {
    let mut elem = SvgTextElement::new(String::new());
    elem.x = attrs.get_f64_opt("x");
    elem.y = attrs.get_f64_opt("y");
    elem.font_family = attrs.get_string("font-family");
    elem.font_size = attrs
        .get("font-size")
        .and_then(|v| v.trim_end_matches("px").trim().parse().ok());
    elem.id = attrs.get_string("id");
    elem.class = attrs.get_string("class");
    elem
}

/// Parse circle shape from attributes
#[inline]
fn parse_circle(attrs: &AttrMap) -> SvgShape {
    SvgShape::Circle {
        cx: attrs.get_f64("cx"),
        cy: attrs.get_f64("cy"),
        r: attrs.get_f64("r"),
        fill: attrs.get_string("fill"),
        stroke: attrs.get_string("stroke"),
        id: attrs.get_string("id"),
    }
}

/// Parse rectangle shape from attributes
#[inline]
fn parse_rect(attrs: &AttrMap) -> SvgShape {
    SvgShape::Rect {
        x: attrs.get_f64("x"),
        y: attrs.get_f64("y"),
        width: attrs.get_f64("width"),
        height: attrs.get_f64("height"),
        fill: attrs.get_string("fill"),
        stroke: attrs.get_string("stroke"),
        id: attrs.get_string("id"),
    }
}

/// Parse ellipse shape from attributes
#[inline]
fn parse_ellipse(attrs: &AttrMap) -> SvgShape {
    SvgShape::Ellipse {
        cx: attrs.get_f64("cx"),
        cy: attrs.get_f64("cy"),
        rx: attrs.get_f64("rx"),
        ry: attrs.get_f64("ry"),
        fill: attrs.get_string("fill"),
        stroke: attrs.get_string("stroke"),
        id: attrs.get_string("id"),
    }
}

/// Parse path shape from attributes
#[inline]
fn parse_path(attrs: &AttrMap) -> Option<SvgShape> {
    let d = attrs.get_string("d").unwrap_or_default();
    if d.is_empty() {
        return None;
    }
    Some(SvgShape::Path {
        d,
        fill: attrs.get_string("fill"),
        stroke: attrs.get_string("stroke"),
        id: attrs.get_string("id"),
    })
}

/// Parse line shape from attributes
#[inline]
fn parse_line(attrs: &AttrMap) -> SvgShape {
    SvgShape::Line {
        x1: attrs.get_f64("x1"),
        y1: attrs.get_f64("y1"),
        x2: attrs.get_f64("x2"),
        y2: attrs.get_f64("y2"),
        stroke: attrs.get_string("stroke"),
        id: attrs.get_string("id"),
    }
}

/// Parse polyline shape from attributes
#[inline]
fn parse_polyline(attrs: &AttrMap) -> Option<SvgShape> {
    let points_str = attrs.get("points").unwrap_or("");
    let points = parse_points(points_str);
    if points.is_empty() {
        return None;
    }
    Some(SvgShape::Polyline {
        points,
        fill: attrs.get_string("fill"),
        stroke: attrs.get_string("stroke"),
        id: attrs.get_string("id"),
    })
}

/// Parse polygon shape from attributes
#[inline]
fn parse_polygon(attrs: &AttrMap) -> Option<SvgShape> {
    let points_str = attrs.get("points").unwrap_or("");
    let points = parse_points(points_str);
    if points.is_empty() {
        return None;
    }
    Some(SvgShape::Polygon {
        points,
        fill: attrs.get_string("fill"),
        stroke: attrs.get_string("stroke"),
        id: attrs.get_string("id"),
    })
}

/// Parse shape from tag name and attributes
#[inline]
fn parse_shape(tag_name: &str, attrs: &AttrMap) -> Option<SvgShape> {
    match tag_name {
        "circle" => Some(parse_circle(attrs)),
        "rect" => Some(parse_rect(attrs)),
        "ellipse" => Some(parse_ellipse(attrs)),
        "path" => parse_path(attrs),
        "line" => Some(parse_line(attrs)),
        "polyline" => parse_polyline(attrs),
        "polygon" => parse_polygon(attrs),
        _ => None,
    }
}

/// Parser state for SVG parsing
#[derive(Debug, Clone, Default, PartialEq)]
struct SvgParseState {
    in_text: bool,
    in_title: bool,
    in_desc: bool,
    current_text: Option<SvgTextElement>,
    text_content: String,
}

/// Parse SVG from string content
///
/// # Errors
///
/// Returns an error if:
/// - The content is not valid XML (`SvgError::XmlError`)
/// - The SVG structure is invalid (`SvgError::InvalidStructure`)
#[must_use = "parsing produces a result that should be handled"]
pub fn parse_svg_str(content: &str) -> Result<SvgDocument> {
    let mut metadata = SvgMetadata::new();
    let mut text_elements = Vec::new();
    let mut shapes = Vec::new();
    let mut state = SvgParseState::default();

    let mut reader = Reader::from_str(content);
    reader.trim_text(true);
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e) | Event::Empty(e)) => {
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let attrs = AttrMap::from_event(&e);

                match tag_name.as_str() {
                    "svg" => parse_svg_metadata(&attrs, &mut metadata),
                    "title" => state.in_title = true,
                    "desc" => state.in_desc = true,
                    "text" => {
                        state.in_text = true;
                        state.current_text = Some(parse_text_element(&attrs));
                        state.text_content.clear();
                    }
                    _ => {
                        if let Some(shape) = parse_shape(&tag_name, &attrs) {
                            shapes.push(shape);
                        }
                    }
                }
            }
            Ok(Event::Text(e)) => {
                let text = e.unescape().unwrap_or_default().trim().to_string();
                if !text.is_empty() {
                    if state.in_title {
                        metadata.title = Some(text);
                    } else if state.in_desc {
                        metadata.description = Some(text);
                    } else if state.in_text {
                        state.text_content.push_str(&text);
                        state.text_content.push(' ');
                    }
                }
            }
            Ok(Event::End(e)) => {
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                match tag_name.as_str() {
                    "title" => state.in_title = false,
                    "desc" => state.in_desc = false,
                    "text" => {
                        state.in_text = false;
                        if let Some(mut elem) = state.current_text.take() {
                            elem.content = state.text_content.trim().to_string();
                            if !elem.content.is_empty() {
                                text_elements.push(elem);
                            }
                        }
                        state.text_content.clear();
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                log::warn!("XML parse error in SVG: {e}");
                return Err(SvgError::XmlError(e.to_string()));
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(SvgDocument {
        metadata,
        text_elements,
        shapes,
    })
}

/// Parse SVG points string (e.g., "10,20 30,40 50,60" or "10 20 30 40 50 60")
#[inline]
fn parse_points(points_str: &str) -> Vec<(f64, f64)> {
    let mut points = Vec::new();

    // Replace commas with spaces, then split by whitespace
    let normalized = points_str.replace(',', " ");
    let coords: Vec<f64> = normalized
        .split_whitespace()
        .filter_map(|s| s.parse::<f64>().ok())
        .collect();

    // Group coordinates into (x, y) pairs
    for chunk in coords.chunks(2) {
        if chunk.len() == 2 {
            points.push((chunk[0], chunk[1]));
        }
    }

    points
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_svg() {
        let svg = r#"<?xml version="1.0"?>
<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
    <title>Test SVG</title>
    <desc>A simple test SVG</desc>
    <text x="10" y="20" font-size="12">Hello World</text>
</svg>"#;

        let doc = parse_svg_str(svg).expect("Failed to parse SVG");

        assert_eq!(doc.metadata.title, Some("Test SVG".to_string()));
        assert_eq!(
            doc.metadata.description,
            Some("A simple test SVG".to_string())
        );
        assert_eq!(doc.metadata.width, Some("100".to_string()));
        assert_eq!(doc.metadata.height, Some("100".to_string()));

        assert_eq!(doc.text_elements.len(), 1);
        assert_eq!(doc.text_elements[0].content, "Hello World");
        assert_eq!(doc.text_elements[0].x, Some(10.0));
        assert_eq!(doc.text_elements[0].y, Some(20.0));
        assert_eq!(doc.text_elements[0].font_size, Some(12.0));
    }

    #[test]
    fn test_parse_svg_with_tspan() {
        let svg = r#"<?xml version="1.0"?>
<svg xmlns="http://www.w3.org/2000/svg">
    <text x="10" y="20">
        <tspan>Hello</tspan>
        <tspan> World</tspan>
    </text>
</svg>"#;

        let doc = parse_svg_str(svg).expect("Failed to parse SVG with tspan");

        assert_eq!(doc.text_elements.len(), 1);
        assert_eq!(doc.text_elements[0].content, "Hello World");
    }

    #[test]
    fn test_parse_svg_multiple_text() {
        let svg = r#"<?xml version="1.0"?>
<svg xmlns="http://www.w3.org/2000/svg">
    <text x="10" y="20">First</text>
    <text x="10" y="40">Second</text>
    <text x="10" y="60">Third</text>
</svg>"#;

        let doc = parse_svg_str(svg).expect("Failed to parse SVG with multiple text");

        assert_eq!(doc.text_elements.len(), 3);
        assert_eq!(doc.text_elements[0].content, "First");
        assert_eq!(doc.text_elements[1].content, "Second");
        assert_eq!(doc.text_elements[2].content, "Third");
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_parse_svg_with_shapes() {
        let svg = r##"<?xml version="1.0"?>
<svg xmlns="http://www.w3.org/2000/svg" width="200" height="200">
    <title>Shape Test</title>
    <circle cx="100" cy="100" r="50" fill="#FF0000"/>
    <rect x="10" y="10" width="80" height="60" fill="#00FF00" stroke="#000000"/>
    <ellipse cx="150" cy="150" rx="40" ry="20" fill="#0000FF"/>
    <path d="M 10 10 L 90 90" stroke="#FF00FF"/>
    <text x="100" y="110" font-size="12">Center</text>
</svg>"##;

        let doc = parse_svg_str(svg).expect("Failed to parse SVG with shapes");

        // Check shapes were parsed
        assert_eq!(doc.shapes.len(), 4, "Should parse 4 shapes");

        // Verify circle
        if let Some(shape) = doc.shapes.first() {
            match shape {
                crate::element::SvgShape::Circle {
                    cx, cy, r, fill, ..
                } => {
                    assert_eq!(*cx, 100.0);
                    assert_eq!(*cy, 100.0);
                    assert_eq!(*r, 50.0);
                    assert_eq!(fill.as_deref(), Some("#FF0000"));
                }
                _ => panic!("Expected Circle shape"),
            }
        }

        // Verify rect
        if let Some(shape) = doc.shapes.get(1) {
            match shape {
                crate::element::SvgShape::Rect {
                    x,
                    y,
                    width,
                    height,
                    ..
                } => {
                    assert_eq!(*x, 10.0);
                    assert_eq!(*y, 10.0);
                    assert_eq!(*width, 80.0);
                    assert_eq!(*height, 60.0);
                }
                _ => panic!("Expected Rect shape"),
            }
        }

        // Verify ellipse
        if let Some(shape) = doc.shapes.get(2) {
            match shape {
                crate::element::SvgShape::Ellipse { cx, cy, rx, ry, .. } => {
                    assert_eq!(*cx, 150.0);
                    assert_eq!(*cy, 150.0);
                    assert_eq!(*rx, 40.0);
                    assert_eq!(*ry, 20.0);
                }
                _ => panic!("Expected Ellipse shape"),
            }
        }

        // Verify path
        if let Some(shape) = doc.shapes.get(3) {
            match shape {
                crate::element::SvgShape::Path { d, .. } => {
                    assert_eq!(d, "M 10 10 L 90 90");
                }
                _ => panic!("Expected Path shape"),
            }
        }

        // Verify text still parsed
        assert_eq!(doc.text_elements.len(), 1);
        assert_eq!(doc.text_elements[0].content, "Center");
    }

    #[test]
    fn test_parse_file_if_exists() {
        let path = Path::new("../../test-corpus/svg/simple_icon.svg");
        if path.exists() {
            let doc = parse_svg(path).expect("Failed to parse SVG file");
            assert!(!doc.text_elements.is_empty() || doc.metadata.title.is_some());
            // Verify shapes are parsed from real file
            if !doc.shapes.is_empty() {
                println!("Found {} shapes in simple_icon.svg", doc.shapes.len());
            }
        }
    }

    #[test]
    fn test_simple_icon_content() {
        use crate::error::SvgError;

        let path = Path::new("../../test-corpus/svg/simple_icon.svg");
        if !path.exists() {
            return; // Skip if file doesn't exist
        }

        let doc = parse_svg(path).expect("Failed to parse simple_icon.svg");

        // Verify title extracted
        assert_eq!(doc.metadata.title, Some("Simple Icon".to_string()));

        // Verify description extracted
        assert_eq!(
            doc.metadata.description,
            Some("A simple SVG icon with text".to_string())
        );

        // Verify shapes extracted (circle)
        assert_eq!(doc.shapes.len(), 1);

        // Verify text extracted (ICON)
        assert_eq!(doc.text_elements.len(), 1);
        assert_eq!(doc.text_elements[0].content, "ICON");

        println!("\n✓ Title: {:?}", doc.metadata.title);
        println!("✓ Description: {:?}", doc.metadata.description);
        println!("✓ Shapes: {}", doc.shapes.len());
        println!("✓ Text elements: {}", doc.text_elements.len());

        // Now test the backend output
        let _svg_str = std::fs::read_to_string(path).map_err(SvgError::Io).unwrap();
        // Can't call backend here since it's in a different crate
        // Just verify parser output is correct
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_parse_line_shape() {
        let svg = r##"<?xml version="1.0"?>
<svg xmlns="http://www.w3.org/2000/svg" width="200" height="200">
    <line x1="10" y1="20" x2="100" y2="80" stroke="#FF0000" id="line1"/>
    <line x1="0" y1="0" x2="50" y2="50" stroke="#00FF00"/>
</svg>"##;

        let doc = parse_svg_str(svg).expect("Failed to parse SVG with line shapes");

        // Check lines were parsed
        assert_eq!(doc.shapes.len(), 2, "Should parse 2 line shapes");

        // Verify first line
        if let Some(shape) = doc.shapes.first() {
            match shape {
                crate::element::SvgShape::Line {
                    x1,
                    y1,
                    x2,
                    y2,
                    stroke,
                    id,
                } => {
                    assert_eq!(*x1, 10.0);
                    assert_eq!(*y1, 20.0);
                    assert_eq!(*x2, 100.0);
                    assert_eq!(*y2, 80.0);
                    assert_eq!(stroke.as_deref(), Some("#FF0000"));
                    assert_eq!(id.as_deref(), Some("line1"));
                }
                _ => panic!("Expected Line shape"),
            }
        }

        // Verify second line
        if let Some(shape) = doc.shapes.get(1) {
            match shape {
                crate::element::SvgShape::Line { x1, y1, x2, y2, .. } => {
                    assert_eq!(*x1, 0.0);
                    assert_eq!(*y1, 0.0);
                    assert_eq!(*x2, 50.0);
                    assert_eq!(*y2, 50.0);
                }
                _ => panic!("Expected Line shape"),
            }
        }
    }

    #[test]
    fn test_parse_polyline_shape() {
        let svg = r##"<?xml version="1.0"?>
<svg xmlns="http://www.w3.org/2000/svg" width="200" height="200">
    <polyline points="10,20 30,40 50,60 70,80" fill="none" stroke="#0000FF" id="poly1"/>
    <polyline points="0 0 100 0 100 100 0 100" stroke="#FF00FF"/>
</svg>"##;

        let doc = parse_svg_str(svg).expect("Failed to parse SVG with polyline shapes");

        // Check polylines were parsed
        assert_eq!(doc.shapes.len(), 2, "Should parse 2 polyline shapes");

        // Verify first polyline
        if let Some(shape) = doc.shapes.first() {
            match shape {
                crate::element::SvgShape::Polyline {
                    points,
                    fill,
                    stroke,
                    id,
                } => {
                    assert_eq!(points.len(), 4);
                    assert_eq!(points[0], (10.0, 20.0));
                    assert_eq!(points[1], (30.0, 40.0));
                    assert_eq!(points[2], (50.0, 60.0));
                    assert_eq!(points[3], (70.0, 80.0));
                    assert_eq!(fill.as_deref(), Some("none"));
                    assert_eq!(stroke.as_deref(), Some("#0000FF"));
                    assert_eq!(id.as_deref(), Some("poly1"));
                }
                _ => panic!("Expected Polyline shape"),
            }
        }

        // Verify second polyline (space-separated points format)
        if let Some(shape) = doc.shapes.get(1) {
            match shape {
                crate::element::SvgShape::Polyline { points, .. } => {
                    assert_eq!(points.len(), 4);
                    assert_eq!(points[0], (0.0, 0.0));
                    assert_eq!(points[1], (100.0, 0.0));
                    assert_eq!(points[2], (100.0, 100.0));
                    assert_eq!(points[3], (0.0, 100.0));
                }
                _ => panic!("Expected Polyline shape"),
            }
        }
    }

    #[test]
    fn test_parse_polygon_shape() {
        let svg = r##"<?xml version="1.0"?>
<svg xmlns="http://www.w3.org/2000/svg" width="200" height="200">
    <polygon points="100,10 150,100 50,100" fill="#FFFF00" stroke="#000000" id="triangle"/>
    <polygon points="20,20 80,20 80,80 20,80" fill="#00FFFF"/>
</svg>"##;

        let doc = parse_svg_str(svg).expect("Failed to parse SVG with polygon shapes");

        // Check polygons were parsed
        assert_eq!(doc.shapes.len(), 2, "Should parse 2 polygon shapes");

        // Verify first polygon (triangle)
        if let Some(shape) = doc.shapes.first() {
            match shape {
                crate::element::SvgShape::Polygon {
                    points,
                    fill,
                    stroke,
                    id,
                } => {
                    assert_eq!(points.len(), 3);
                    assert_eq!(points[0], (100.0, 10.0));
                    assert_eq!(points[1], (150.0, 100.0));
                    assert_eq!(points[2], (50.0, 100.0));
                    assert_eq!(fill.as_deref(), Some("#FFFF00"));
                    assert_eq!(stroke.as_deref(), Some("#000000"));
                    assert_eq!(id.as_deref(), Some("triangle"));
                }
                _ => panic!("Expected Polygon shape"),
            }
        }

        // Verify second polygon (square)
        if let Some(shape) = doc.shapes.get(1) {
            match shape {
                crate::element::SvgShape::Polygon { points, fill, .. } => {
                    assert_eq!(points.len(), 4);
                    assert_eq!(points[0], (20.0, 20.0));
                    assert_eq!(points[1], (80.0, 20.0));
                    assert_eq!(points[2], (80.0, 80.0));
                    assert_eq!(points[3], (20.0, 80.0));
                    assert_eq!(fill.as_deref(), Some("#00FFFF"));
                }
                _ => panic!("Expected Polygon shape"),
            }
        }
    }

    #[test]
    fn test_parse_all_shape_types() {
        let svg = r#"<?xml version="1.0"?>
<svg xmlns="http://www.w3.org/2000/svg" width="300" height="300">
    <title>Complete Shape Test</title>
    <circle cx="50" cy="50" r="25" fill="red"/>
    <rect x="100" y="100" width="50" height="30" fill="green"/>
    <ellipse cx="200" cy="200" rx="30" ry="20" fill="blue"/>
    <path d="M 10 10 L 50 50 L 10 50 Z" fill="yellow"/>
    <line x1="250" y1="10" x2="290" y2="50" stroke="purple"/>
    <polyline points="10,250 30,270 50,250 70,270" stroke="orange" fill="none"/>
    <polygon points="200,250 220,290 180,290" fill="cyan"/>
</svg>"#;

        let doc = parse_svg_str(svg).expect("Failed to parse SVG with all shape types");

        // Check all 7 shape types were parsed
        assert_eq!(doc.shapes.len(), 7, "Should parse 7 shapes (all types)");

        // Verify we have one of each type
        let mut has_circle = false;
        let mut has_rect = false;
        let mut has_ellipse = false;
        let mut has_path = false;
        let mut has_line = false;
        let mut has_polyline = false;
        let mut has_polygon = false;

        for shape in &doc.shapes {
            match shape {
                crate::element::SvgShape::Circle { .. } => has_circle = true,
                crate::element::SvgShape::Rect { .. } => has_rect = true,
                crate::element::SvgShape::Ellipse { .. } => has_ellipse = true,
                crate::element::SvgShape::Path { .. } => has_path = true,
                crate::element::SvgShape::Line { .. } => has_line = true,
                crate::element::SvgShape::Polyline { .. } => has_polyline = true,
                crate::element::SvgShape::Polygon { .. } => has_polygon = true,
            }
        }

        assert!(has_circle, "Should have Circle shape");
        assert!(has_rect, "Should have Rect shape");
        assert!(has_ellipse, "Should have Ellipse shape");
        assert!(has_path, "Should have Path shape");
        assert!(has_line, "Should have Line shape");
        assert!(has_polyline, "Should have Polyline shape");
        assert!(has_polygon, "Should have Polygon shape");

        // Verify metadata
        assert_eq!(doc.metadata.title, Some("Complete Shape Test".to_string()));
    }

    #[test]
    fn test_parse_points_helper() {
        // Test comma-separated format
        let points = parse_points("10,20 30,40 50,60");
        assert_eq!(points.len(), 3);
        assert_eq!(points[0], (10.0, 20.0));
        assert_eq!(points[1], (30.0, 40.0));
        assert_eq!(points[2], (50.0, 60.0));

        // Test space-separated format
        let points = parse_points("10 20 30 40 50 60");
        assert_eq!(points.len(), 3);
        assert_eq!(points[0], (10.0, 20.0));
        assert_eq!(points[1], (30.0, 40.0));
        assert_eq!(points[2], (50.0, 60.0));

        // Test mixed format
        let points = parse_points("10,20 30 40,50,60");
        assert_eq!(points.len(), 3);
        assert_eq!(points[0], (10.0, 20.0));
        assert_eq!(points[1], (30.0, 40.0));
        assert_eq!(points[2], (50.0, 60.0));

        // Test empty string
        let points = parse_points("");
        assert_eq!(points.len(), 0);

        // Test odd number of coordinates (should ignore last one)
        let points = parse_points("10,20 30");
        assert_eq!(points.len(), 1);
        assert_eq!(points[0], (10.0, 20.0));
    }
}
