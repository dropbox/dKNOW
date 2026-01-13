//! Shared type definitions for `DoclingViz` FFI bridge
//!
//! This module contains additional types used by the FFI layer.

use crate::{DlvizBBox, DlvizLabel};

/// RGBA color for visualization.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct DlvizColor {
    /// Red channel (0-255).
    pub r: u8,
    /// Green channel (0-255).
    pub g: u8,
    /// Blue channel (0-255).
    pub b: u8,
    /// Alpha channel (0-255, 255 = fully opaque).
    pub a: u8,
}

impl DlvizColor {
    /// Create a new color
    #[inline]
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Create an opaque color
    #[inline]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::new(r, g, b, 255)
    }
}

/// Label color scheme for visualization
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct LabelColors;

impl LabelColors {
    /// Get color for a label
    #[inline]
    pub const fn color_for(label: DlvizLabel) -> DlvizColor {
        match label {
            DlvizLabel::Caption => DlvizColor::rgb(255, 165, 0), // Orange
            DlvizLabel::Footnote => DlvizColor::rgb(128, 128, 128), // Gray
            DlvizLabel::Formula => DlvizColor::rgb(0, 255, 255), // Cyan
            DlvizLabel::ListItem => DlvizColor::rgb(144, 238, 144), // Light green
            DlvizLabel::PageFooter | DlvizLabel::PageHeader => {
                DlvizColor::rgb(192, 192, 192) // Silver
            }
            DlvizLabel::Picture => DlvizColor::rgb(255, 0, 255), // Magenta
            DlvizLabel::SectionHeader => DlvizColor::rgb(0, 0, 255), // Blue
            DlvizLabel::Table => DlvizColor::rgb(0, 255, 0),     // Green
            DlvizLabel::Text => DlvizColor::rgb(255, 255, 0),    // Yellow
            DlvizLabel::Title => DlvizColor::rgb(255, 0, 0),     // Red
            DlvizLabel::Code => DlvizColor::rgb(128, 0, 128),    // Purple
            DlvizLabel::CheckboxSelected => DlvizColor::rgb(0, 128, 0), // Dark green
            DlvizLabel::CheckboxUnselected => DlvizColor::rgb(128, 0, 0), // Dark red
            DlvizLabel::DocumentIndex => DlvizColor::rgb(0, 128, 128), // Teal
            DlvizLabel::Form => DlvizColor::rgb(255, 192, 203),  // Pink
            DlvizLabel::KeyValueRegion => DlvizColor::rgb(255, 215, 0), // Gold
        }
    }
}

/// Conversion utilities for docling-core types
pub mod convert {
    use super::*;

    /// Convert a [`DlvizBBox`] to a rect tuple (x, y, width, height)
    #[inline]
    pub const fn bbox_to_rect(bbox: &DlvizBBox) -> (f32, f32, f32, f32) {
        (bbox.x, bbox.y, bbox.width, bbox.height)
    }

    /// Create [`DlvizBBox`] from coordinates
    #[inline]
    pub const fn rect_to_bbox(x: f32, y: f32, width: f32, height: f32) -> DlvizBBox {
        DlvizBBox {
            x,
            y,
            width,
            height,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_creation() {
        let c = DlvizColor::rgb(255, 128, 0);
        assert_eq!(c.r, 255);
        assert_eq!(c.g, 128);
        assert_eq!(c.b, 0);
        assert_eq!(c.a, 255);
    }

    #[test]
    fn test_label_colors() {
        let title_color = LabelColors::color_for(DlvizLabel::Title);
        assert_eq!(title_color.r, 255);
        assert_eq!(title_color.g, 0);
        assert_eq!(title_color.b, 0);
    }

    #[test]
    fn test_bbox_conversion() {
        let bbox = convert::rect_to_bbox(10.0, 20.0, 100.0, 50.0);
        let (x, y, w, h) = convert::bbox_to_rect(&bbox);
        assert_eq!((x, y, w, h), (10.0, 20.0, 100.0, 50.0));
    }
}
