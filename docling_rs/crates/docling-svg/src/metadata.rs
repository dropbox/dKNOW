//! SVG metadata structures

/// SVG document metadata
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct SvgMetadata {
    /// SVG title (from `<title>` element)
    pub title: Option<String>,

    /// SVG description (from `<desc>` element)
    pub description: Option<String>,

    /// SVG width (from width attribute on `<svg>` element)
    pub width: Option<String>,

    /// SVG height (from height attribute on `<svg>` element)
    pub height: Option<String>,

    /// SVG viewBox (from viewBox attribute)
    pub viewbox: Option<String>,
}

impl SvgMetadata {
    /// Create a new empty metadata object
    #[inline]
    #[must_use = "creates empty metadata object"]
    pub const fn new() -> Self {
        Self {
            title: None,
            description: None,
            width: None,
            height: None,
            viewbox: None,
        }
    }
}
