//! SVG element structures (text and shapes)

use std::fmt::Write;

/// A text element extracted from SVG
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SvgTextElement {
    /// Text content
    pub content: String,

    /// X coordinate (if specified)
    pub x: Option<f64>,

    /// Y coordinate (if specified)
    pub y: Option<f64>,

    /// Font family (if specified)
    pub font_family: Option<String>,

    /// Font size (if specified)
    pub font_size: Option<f64>,

    /// Element ID (if specified)
    pub id: Option<String>,

    /// CSS class (if specified)
    pub class: Option<String>,
}

impl SvgTextElement {
    /// Create a new text element with content
    #[inline]
    #[must_use = "creates text element with content"]
    pub const fn new(content: String) -> Self {
        Self {
            content,
            x: None,
            y: None,
            font_family: None,
            font_size: None,
            id: None,
            class: None,
        }
    }
}

/// SVG shape elements
#[derive(Debug, Clone, PartialEq)]
pub enum SvgShape {
    /// Circle element
    Circle {
        /// Center X coordinate
        cx: f64,
        /// Center Y coordinate
        cy: f64,
        /// Circle radius
        r: f64,
        /// Fill color (CSS color value)
        fill: Option<String>,
        /// Stroke color (CSS color value)
        stroke: Option<String>,
        /// Element ID attribute
        id: Option<String>,
    },
    /// Rectangle element
    Rect {
        /// Top-left X coordinate
        x: f64,
        /// Top-left Y coordinate
        y: f64,
        /// Rectangle width
        width: f64,
        /// Rectangle height
        height: f64,
        /// Fill color (CSS color value)
        fill: Option<String>,
        /// Stroke color (CSS color value)
        stroke: Option<String>,
        /// Element ID attribute
        id: Option<String>,
    },
    /// Ellipse element
    Ellipse {
        /// Center X coordinate
        cx: f64,
        /// Center Y coordinate
        cy: f64,
        /// Horizontal radius
        rx: f64,
        /// Vertical radius
        ry: f64,
        /// Fill color (CSS color value)
        fill: Option<String>,
        /// Stroke color (CSS color value)
        stroke: Option<String>,
        /// Element ID attribute
        id: Option<String>,
    },
    /// Path element (simplified - just the 'd' attribute)
    Path {
        /// SVG path data string (d attribute)
        d: String,
        /// Fill color (CSS color value)
        fill: Option<String>,
        /// Stroke color (CSS color value)
        stroke: Option<String>,
        /// Element ID attribute
        id: Option<String>,
    },
    /// Line element
    Line {
        /// Start point X coordinate
        x1: f64,
        /// Start point Y coordinate
        y1: f64,
        /// End point X coordinate
        x2: f64,
        /// End point Y coordinate
        y2: f64,
        /// Stroke color (CSS color value)
        stroke: Option<String>,
        /// Element ID attribute
        id: Option<String>,
    },
    /// Polyline element (connected line segments)
    Polyline {
        /// List of (x, y) coordinate pairs
        points: Vec<(f64, f64)>,
        /// Fill color (CSS color value)
        fill: Option<String>,
        /// Stroke color (CSS color value)
        stroke: Option<String>,
        /// Element ID attribute
        id: Option<String>,
    },
    /// Polygon element (closed shape)
    Polygon {
        /// List of (x, y) vertex coordinate pairs
        points: Vec<(f64, f64)>,
        /// Fill color (CSS color value)
        fill: Option<String>,
        /// Stroke color (CSS color value)
        stroke: Option<String>,
        /// Element ID attribute
        id: Option<String>,
    },
}

/// Helper to append style attributes to description
#[inline]
fn append_style_attrs(
    desc: &mut String,
    fill: Option<&str>,
    stroke: Option<&str>,
    id: Option<&str>,
) {
    if let Some(f) = fill {
        let _ = write!(desc, ", fill {f}");
    }
    if let Some(s) = stroke {
        let _ = write!(desc, ", stroke {s}");
    }
    if let Some(i) = id {
        let _ = write!(desc, " (id: {i})");
    }
}

/// Helper for stroke-only shapes (Line)
#[inline]
fn append_stroke_and_id(desc: &mut String, stroke: Option<&str>, id: Option<&str>) {
    if let Some(s) = stroke {
        let _ = write!(desc, ", stroke {s}");
    }
    if let Some(i) = id {
        let _ = write!(desc, " (id: {i})");
    }
}

impl SvgShape {
    /// Convert shape to human-readable markdown description
    #[must_use = "converts shape to markdown description"]
    pub fn to_markdown(&self) -> String {
        match self {
            Self::Circle {
                cx,
                cy,
                r,
                fill,
                stroke,
                id,
            } => {
                let mut desc = format!("Circle at ({cx}, {cy}), radius {r}");
                append_style_attrs(&mut desc, fill.as_deref(), stroke.as_deref(), id.as_deref());
                desc
            }
            Self::Rect {
                x,
                y,
                width,
                height,
                fill,
                stroke,
                id,
            } => {
                let mut desc = format!("Rectangle at ({x}, {y}), {width}×{height}");
                append_style_attrs(&mut desc, fill.as_deref(), stroke.as_deref(), id.as_deref());
                desc
            }
            Self::Ellipse {
                cx,
                cy,
                rx,
                ry,
                fill,
                stroke,
                id,
            } => {
                let mut desc = format!("Ellipse at ({cx}, {cy}), radii {rx}×{ry}");
                append_style_attrs(&mut desc, fill.as_deref(), stroke.as_deref(), id.as_deref());
                desc
            }
            Self::Path {
                d,
                fill,
                stroke,
                id,
            } => {
                let d_preview = if d.len() > 50 {
                    format!("{}...", &d[..50])
                } else {
                    d.clone()
                };
                let mut desc = format!("Path ({d_preview})");
                append_style_attrs(&mut desc, fill.as_deref(), stroke.as_deref(), id.as_deref());
                desc
            }
            Self::Line {
                x1,
                y1,
                x2,
                y2,
                stroke,
                id,
            } => {
                let mut desc = format!("Line from ({x1}, {y1}) to ({x2}, {y2})");
                append_stroke_and_id(&mut desc, stroke.as_deref(), id.as_deref());
                desc
            }
            Self::Polyline {
                points,
                fill,
                stroke,
                id,
            } => {
                let point_count = points.len();
                let mut desc = format!("Polyline with {point_count} points");
                if point_count > 0 {
                    let _ = write!(desc, " starting at ({}, {})", points[0].0, points[0].1);
                    if point_count > 1 {
                        let last = &points[point_count - 1];
                        let _ = write!(desc, " ending at ({}, {})", last.0, last.1);
                    }
                }
                append_style_attrs(&mut desc, fill.as_deref(), stroke.as_deref(), id.as_deref());
                desc
            }
            Self::Polygon {
                points,
                fill,
                stroke,
                id,
            } => {
                let point_count = points.len();
                let mut desc = format!("Polygon with {point_count} vertices");
                if point_count > 0 {
                    let _ = write!(desc, " starting at ({}, {})", points[0].0, points[0].1);
                }
                append_style_attrs(&mut desc, fill.as_deref(), stroke.as_deref(), id.as_deref());
                desc
            }
        }
    }
}

impl std::fmt::Display for SvgShape {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Circle { r, .. } => write!(f, "circle (r={r:.1})"),
            Self::Rect { width, height, .. } => write!(f, "rect ({width:.1}×{height:.1})"),
            Self::Ellipse { rx, ry, .. } => write!(f, "ellipse ({rx:.1}×{ry:.1})"),
            Self::Path { d, .. } => {
                let preview = if d.len() > 20 { &d[..20] } else { d };
                write!(f, "path ({preview}...)")
            }
            Self::Line { .. } => write!(f, "line"),
            Self::Polyline { points, .. } => write!(f, "polyline ({} pts)", points.len()),
            Self::Polygon { points, .. } => write!(f, "polygon ({} pts)", points.len()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_svg_shape_display() {
        let circle = SvgShape::Circle {
            cx: 10.0,
            cy: 20.0,
            r: 5.5,
            fill: None,
            stroke: None,
            id: None,
        };
        assert_eq!(format!("{circle}"), "circle (r=5.5)");

        let rect = SvgShape::Rect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 50.0,
            fill: None,
            stroke: None,
            id: None,
        };
        assert_eq!(format!("{rect}"), "rect (100.0×50.0)");

        let ellipse = SvgShape::Ellipse {
            cx: 50.0,
            cy: 50.0,
            rx: 30.0,
            ry: 20.0,
            fill: None,
            stroke: None,
            id: None,
        };
        assert_eq!(format!("{ellipse}"), "ellipse (30.0×20.0)");

        let path = SvgShape::Path {
            d: "M0 0 L10 10 Z".to_string(),
            fill: None,
            stroke: None,
            id: None,
        };
        assert_eq!(format!("{path}"), "path (M0 0 L10 10 Z...)");

        let line = SvgShape::Line {
            x1: 0.0,
            y1: 0.0,
            x2: 100.0,
            y2: 100.0,
            stroke: None,
            id: None,
        };
        assert_eq!(format!("{line}"), "line");

        let polyline = SvgShape::Polyline {
            points: vec![(0.0, 0.0), (10.0, 10.0), (20.0, 0.0)],
            fill: None,
            stroke: None,
            id: None,
        };
        assert_eq!(format!("{polyline}"), "polyline (3 pts)");

        let polygon = SvgShape::Polygon {
            points: vec![(0.0, 0.0), (10.0, 10.0), (20.0, 0.0), (10.0, -10.0)],
            fill: None,
            stroke: None,
            id: None,
        };
        assert_eq!(format!("{polygon}"), "polygon (4 pts)");
    }
}
