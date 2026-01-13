//! XPS page structures

/// XPS page content
#[derive(Debug, Clone, Default, PartialEq)]
pub struct XpsPage {
    /// Page number (1-indexed)
    pub number: usize,

    /// Page width (in XPS units, 1/96 inch)
    pub width: f64,

    /// Page height (in XPS units, 1/96 inch)
    pub height: f64,

    /// Text elements on the page
    pub text: Vec<XpsTextElement>,
}

/// Text element on XPS page
#[derive(Debug, Clone, Default, PartialEq)]
pub struct XpsTextElement {
    /// Text content
    pub content: String,

    /// X position (left edge)
    pub x: f64,

    /// Y position (top edge)
    pub y: f64,

    /// Font size (if available)
    pub font_size: Option<f64>,
}

impl XpsTextElement {
    /// Create new text element
    #[inline]
    #[must_use = "creates text element at position"]
    pub const fn new(content: String, x: f64, y: f64) -> Self {
        Self {
            content,
            x,
            y,
            font_size: None,
        }
    }
}
