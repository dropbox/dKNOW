//! PDF page representation

use crate::annotation::PdfPageAnnotations;
use crate::document::PdfDocumentInner;
use crate::error::{PdfError, Result};
use crate::form::PdfPageFormFields;
use crate::page_object::{PdfPageObject, PdfPageObjects};
use crate::render::{PdfBitmap, PdfRenderConfig, PixelFormat};
use crate::structure::PdfStructTree;
use crate::text::PdfPageText;
use pdfium_sys::*;
use std::sync::Arc;

/// Metrics for a text block extracted from a PDF page.
///
/// Contains measurements about spacing, line heights, and indentation
/// that are useful for detecting text structure (paragraphs, columns, etc.).
///
/// # Example
///
/// ```no_run
/// use pdfium_render_fast::Pdfium;
///
/// let pdfium = Pdfium::new()?;
/// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
/// let page = doc.page(0)?;
///
/// for block in page.extract_text_blocks_with_metrics() {
///     println!("Block: {} lines, avg spacing: {:.1}pt",
///         block.line_count, block.avg_line_spacing);
///     if block.first_line_indent > 20.0 {
///         println!("  -> Likely a new paragraph (indented)");
///     }
/// }
/// # Ok::<(), pdfium_render_fast::PdfError>(())
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct TextBlockMetrics {
    /// Bounding box of the text block (left, bottom, right, top) in points.
    pub bounds: (f32, f32, f32, f32),
    /// Number of lines in this text block.
    pub line_count: usize,
    /// Average height of lines in points.
    pub avg_line_height: f32,
    /// Average vertical spacing between lines (leading) in points.
    pub avg_line_spacing: f32,
    /// Horizontal indent of the first line relative to the block's left edge.
    pub first_line_indent: f32,
    /// Average horizontal spacing between adjacent characters in points.
    pub avg_char_spacing: f32,
    /// Average horizontal spacing between words in points.
    pub avg_word_spacing: f32,
}

impl TextBlockMetrics {
    /// Get the width of the text block.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// for block in page.extract_text_blocks_with_metrics() {
    ///     println!("Block width: {:.1}pt", block.width());
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn width(&self) -> f32 {
        (self.bounds.2 - self.bounds.0).abs()
    }

    /// Get the height of the text block.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// for block in page.extract_text_blocks_with_metrics() {
    ///     println!("Block height: {:.1}pt", block.height());
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn height(&self) -> f32 {
        (self.bounds.3 - self.bounds.1).abs()
    }

    /// Get the area of the text block in square points.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// for block in page.extract_text_blocks_with_metrics() {
    ///     println!("Block area: {:.1}pt²", block.area());
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn area(&self) -> f32 {
        self.width() * self.height()
    }

    /// Get the center point of the text block.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// for block in page.extract_text_blocks_with_metrics() {
    ///     let (cx, cy) = block.center();
    ///     println!("Block center: ({:.1}, {:.1})", cx, cy);
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn center(&self) -> (f32, f32) {
        (
            (self.bounds.0 + self.bounds.2) / 2.0,
            (self.bounds.1 + self.bounds.3) / 2.0,
        )
    }

    /// Check if this block is a single line.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// for block in page.extract_text_blocks_with_metrics() {
    ///     if block.is_single_line() {
    ///         println!("Found single-line block");
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn is_single_line(&self) -> bool {
        self.line_count == 1
    }

    /// Check if this block has multiple lines.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// for block in page.extract_text_blocks_with_metrics() {
    ///     if block.is_multi_line() {
    ///         println!("Found multi-line block with {} lines", block.line_count);
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn is_multi_line(&self) -> bool {
        self.line_count > 1
    }

    /// Check if the first line is indented (likely a paragraph start).
    ///
    /// Returns true if first_line_indent is greater than the given threshold.
    ///
    /// # Arguments
    ///
    /// * `threshold` - Minimum indent in points to consider as indented (default: 10.0)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// for block in page.extract_text_blocks_with_metrics() {
    ///     if block.is_indented(15.0) {
    ///         println!("Paragraph start detected");
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn is_indented(&self, threshold: f32) -> bool {
        self.first_line_indent > threshold
    }

    /// Check if this block has tight line spacing (ratio < 1.5).
    ///
    /// Tight spacing typically indicates tightly-formatted content
    /// like lists or tables.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// for block in page.extract_text_blocks_with_metrics() {
    ///     if block.has_tight_spacing() {
    ///         println!("Tightly spaced block");
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn has_tight_spacing(&self) -> bool {
        if self.avg_line_height <= 0.0 {
            return false;
        }
        let ratio = (self.avg_line_height + self.avg_line_spacing) / self.avg_line_height;
        ratio < 1.5
    }

    /// Check if this block has loose line spacing (ratio > 2.0).
    ///
    /// Loose spacing typically indicates double-spaced content
    /// or blocks with extra vertical separation.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// for block in page.extract_text_blocks_with_metrics() {
    ///     if block.has_loose_spacing() {
    ///         println!("Loosely spaced block (double-spaced?)");
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn has_loose_spacing(&self) -> bool {
        if self.avg_line_height <= 0.0 {
            return false;
        }
        let ratio = (self.avg_line_height + self.avg_line_spacing) / self.avg_line_height;
        ratio > 2.0
    }

    /// Get the line spacing ratio (total line pitch / line height).
    ///
    /// - ~1.0-1.2: Very tight (single-spaced, no leading)
    /// - ~1.2-1.5: Normal single-spaced
    /// - ~1.5-2.0: 1.5-line spacing
    /// - ~2.0+: Double-spaced or more
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// for block in page.extract_text_blocks_with_metrics() {
    ///     if let Some(ratio) = block.line_spacing_ratio() {
    ///         println!("Line spacing ratio: {:.2}", ratio);
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn line_spacing_ratio(&self) -> Option<f32> {
        if self.avg_line_height <= 0.0 {
            return None;
        }
        Some((self.avg_line_height + self.avg_line_spacing) / self.avg_line_height)
    }

    /// Get the aspect ratio (width / height) of the block.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// for block in page.extract_text_blocks_with_metrics() {
    ///     println!("Block aspect ratio: {:.2}", block.aspect_ratio());
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn aspect_ratio(&self) -> f32 {
        if self.height() <= 0.0 {
            return 0.0;
        }
        self.width() / self.height()
    }

    /// Check if a point is inside this text block.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// for block in page.extract_text_blocks_with_metrics() {
    ///     if block.contains_point(100.0, 500.0) {
    ///         println!("Point is inside this block");
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn contains_point(&self, x: f32, y: f32) -> bool {
        x >= self.bounds.0 && x <= self.bounds.2 && y >= self.bounds.1 && y <= self.bounds.3
    }

    /// Check if this block overlaps with another.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// let blocks = page.extract_text_blocks_with_metrics();
    /// for i in 0..blocks.len() {
    ///     for j in i+1..blocks.len() {
    ///         if blocks[i].overlaps(&blocks[j]) {
    ///             println!("Blocks {} and {} overlap", i, j);
    ///         }
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn overlaps(&self, other: &TextBlockMetrics) -> bool {
        !(self.bounds.2 < other.bounds.0  // self is left of other
          || self.bounds.0 > other.bounds.2  // self is right of other
          || self.bounds.3 < other.bounds.1  // self is below other
          || self.bounds.1 > other.bounds.3) // self is above other
    }
}

/// Type of text decoration (underline, strikethrough, overline).
///
/// Text decorations in PDFs are typically implemented as path objects
/// (horizontal lines) rather than text properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextDecorationType {
    /// Underline - line below the text baseline
    Underline,
    /// Strikethrough - line through the middle of the text
    Strikethrough,
    /// Overline - line above the text
    Overline,
}

impl TextDecorationType {
    /// Check if this is an underline decoration.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::{Pdfium, TextDecorationType};
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// for decoration in page.extract_text_decorations() {
    ///     if decoration.decoration_type.is_underline() {
    ///         println!("Found underline");
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn is_underline(&self) -> bool {
        matches!(self, TextDecorationType::Underline)
    }

    /// Check if this is a strikethrough decoration.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::{Pdfium, TextDecorationType};
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// for decoration in page.extract_text_decorations() {
    ///     if decoration.decoration_type.is_strikethrough() {
    ///         println!("Found strikethrough");
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn is_strikethrough(&self) -> bool {
        matches!(self, TextDecorationType::Strikethrough)
    }

    /// Check if this is an overline decoration.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::{Pdfium, TextDecorationType};
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// for decoration in page.extract_text_decorations() {
    ///     if decoration.decoration_type.is_overline() {
    ///         println!("Found overline");
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn is_overline(&self) -> bool {
        matches!(self, TextDecorationType::Overline)
    }
}

/// A text decoration (underline, strikethrough, or overline) found on a page.
///
/// Text decorations in PDFs are typically implemented as path objects
/// (horizontal lines) positioned relative to text, rather than as
/// text properties. This struct captures detected decorations.
///
/// # Example
///
/// ```no_run
/// use pdfium_render_fast::Pdfium;
///
/// let pdfium = Pdfium::new()?;
/// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
/// let page = doc.page(0)?;
///
/// for decoration in page.extract_text_decorations() {
///     println!("{:?} at ({:.1}, {:.1}) - ({:.1}, {:.1}), thickness: {:.1}pt",
///         decoration.decoration_type,
///         decoration.bounds.0, decoration.bounds.1,
///         decoration.bounds.2, decoration.bounds.3,
///         decoration.thickness);
/// }
/// # Ok::<(), pdfium_render_fast::PdfError>(())
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct TextDecoration {
    /// Type of decoration
    pub decoration_type: TextDecorationType,
    /// Bounding box (left, bottom, right, top) in points
    pub bounds: (f32, f32, f32, f32),
    /// Line thickness in points
    pub thickness: f32,
    /// Color as (R, G, B, A)
    pub color: (u8, u8, u8, u8),
}

impl TextDecoration {
    /// Get the width of the decoration line.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// for decoration in page.extract_text_decorations() {
    ///     println!("Decoration width: {:.1}pt", decoration.width());
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn width(&self) -> f32 {
        (self.bounds.2 - self.bounds.0).abs()
    }

    /// Get the height of the decoration line.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// for decoration in page.extract_text_decorations() {
    ///     println!("Decoration height: {:.1}pt", decoration.height());
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn height(&self) -> f32 {
        (self.bounds.3 - self.bounds.1).abs()
    }

    /// Get the center y-coordinate of the decoration.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// for decoration in page.extract_text_decorations() {
    ///     println!("Decoration center Y: {:.1}pt", decoration.center_y());
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn center_y(&self) -> f32 {
        (self.bounds.1 + self.bounds.3) / 2.0
    }

    /// Check if this decoration is near a specific y-coordinate.
    ///
    /// # Arguments
    ///
    /// * `y` - The y-coordinate to check
    /// * `tolerance` - Maximum distance to consider "near"
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// let baseline = 100.0;
    /// for decoration in page.extract_text_decorations() {
    ///     if decoration.is_near_y(baseline, 5.0) {
    ///         println!("Decoration near baseline");
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn is_near_y(&self, y: f32, tolerance: f32) -> bool {
        (self.center_y() - y).abs() <= tolerance
    }

    /// Check if the decoration color is not transparent.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// for decoration in page.extract_text_decorations() {
    ///     if decoration.is_visible() {
    ///         println!("Visible decoration");
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn is_visible(&self) -> bool {
        self.color.3 > 0
    }
}

/// Analysis of mathematical characters on a PDF page.
///
/// This struct contains counts of various categories of mathematical
/// characters based on Unicode ranges. Useful for detecting whether
/// a page contains mathematical or technical content.
///
/// # Example
///
/// ```no_run
/// use pdfium_render_fast::Pdfium;
///
/// let pdfium = Pdfium::new()?;
/// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
/// let page = doc.page(0)?;
///
/// let analysis = page.analyze_math_chars();
/// println!("Math operators: {}", analysis.math_operators);
/// println!("Greek letters: {}", analysis.greek_letters);
/// println!("Total chars: {}", analysis.total_chars);
///
/// if analysis.math_ratio() > 0.05 {
///     println!("This page likely contains mathematical content");
/// }
/// # Ok::<(), pdfium_render_fast::PdfError>(())
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MathCharAnalysis {
    /// Count of mathematical operators (∑, ∫, ∂, etc.)
    /// Unicode ranges: U+2200-U+22FF, U+2A00-U+2AFF
    pub math_operators: usize,
    /// Count of mathematical alphanumeric symbols (𝑎, 𝑏, 𝒜, etc.)
    /// Unicode range: U+1D400-U+1D7FF
    pub math_alphanumerics: usize,
    /// Count of Greek letters (α, β, γ, etc.)
    /// Unicode range: U+0370-U+03FF
    pub greek_letters: usize,
    /// Count of arrow symbols (→, ←, ⇒, etc.)
    /// Unicode ranges: U+2190-U+21FF, U+27F0-U+27FF
    pub arrows: usize,
    /// Count of superscript characters (⁰¹²³⁴⁵⁶⁷⁸⁹ⁿ etc.)
    /// Unicode range: U+2070-U+207F
    pub superscripts: usize,
    /// Count of subscript characters (₀₁₂₃₄₅₆₇₈₉ etc.)
    /// Unicode range: U+2080-U+209F
    pub subscripts: usize,
    /// Total number of characters analyzed.
    pub total_chars: usize,
}

impl MathCharAnalysis {
    /// Create a new empty analysis.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the total count of mathematical characters.
    ///
    /// This includes math operators, math alphanumerics, greek letters,
    /// arrows, superscripts, and subscripts.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// let analysis = page.analyze_math_chars();
    /// println!("Total math chars: {}", analysis.math_char_count());
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn math_char_count(&self) -> usize {
        self.math_operators
            + self.math_alphanumerics
            + self.greek_letters
            + self.arrows
            + self.superscripts
            + self.subscripts
    }

    /// Get the ratio of mathematical characters to total characters.
    ///
    /// Returns 0.0 if there are no characters.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// let analysis = page.analyze_math_chars();
    /// if analysis.math_ratio() > 0.10 {
    ///     println!("Heavy math content (>10% math chars)");
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn math_ratio(&self) -> f32 {
        if self.total_chars == 0 {
            0.0
        } else {
            self.math_char_count() as f32 / self.total_chars as f32
        }
    }

    /// Check if this page has significant mathematical content.
    ///
    /// Returns true if math characters make up more than 5% of total chars.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// let analysis = page.analyze_math_chars();
    /// if analysis.has_significant_math() {
    ///     println!("Page contains mathematical content");
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn has_significant_math(&self) -> bool {
        self.math_ratio() > 0.05
    }

    /// Check if this page has any Greek letters.
    ///
    /// Greek letters are commonly used in mathematical notation.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// let analysis = page.analyze_math_chars();
    /// if analysis.has_greek() {
    ///     println!("Page uses Greek letters");
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn has_greek(&self) -> bool {
        self.greek_letters > 0
    }

    /// Check if this page has any mathematical operators.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// let analysis = page.analyze_math_chars();
    /// if analysis.has_operators() {
    ///     println!("Page has math operators");
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn has_operators(&self) -> bool {
        self.math_operators > 0
    }

    /// Check if this page has any sub/superscripts.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// let analysis = page.analyze_math_chars();
    /// if analysis.has_scripts() {
    ///     println!("Page has superscripts/subscripts");
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn has_scripts(&self) -> bool {
        self.superscripts > 0 || self.subscripts > 0
    }
}

/// Check if a character is a mathematical operator.
///
/// Covers Unicode ranges:
/// - Mathematical Operators: U+2200-U+22FF
/// - Supplemental Mathematical Operators: U+2A00-U+2AFF
/// - Miscellaneous Mathematical Symbols-A: U+27C0-U+27EF
/// - Miscellaneous Mathematical Symbols-B: U+2980-U+29FF
fn is_math_operator(c: char) -> bool {
    let code = c as u32;
    (0x2200..=0x22FF).contains(&code)      // Mathematical Operators
        || (0x2A00..=0x2AFF).contains(&code)   // Supplemental Mathematical Operators
        || (0x27C0..=0x27EF).contains(&code)   // Misc Math Symbols-A
        || (0x2980..=0x29FF).contains(&code) // Misc Math Symbols-B
}

/// Check if a character is a mathematical alphanumeric symbol.
///
/// Covers Unicode range: U+1D400-U+1D7FF
/// Includes: bold, italic, script, fraktur, double-struck, sans-serif,
/// monospace mathematical letters and digits.
fn is_math_alphanumeric(c: char) -> bool {
    let code = c as u32;
    (0x1D400..=0x1D7FF).contains(&code)
}

/// Check if a character is a Greek letter.
///
/// Covers Unicode range: U+0370-U+03FF (Greek and Coptic)
fn is_greek_letter(c: char) -> bool {
    let code = c as u32;
    (0x0370..=0x03FF).contains(&code)
}

/// Check if a character is an arrow symbol.
///
/// Covers Unicode ranges:
/// - Arrows: U+2190-U+21FF
/// - Supplemental Arrows-A: U+27F0-U+27FF
/// - Supplemental Arrows-B: U+2900-U+297F
fn is_arrow(c: char) -> bool {
    let code = c as u32;
    (0x2190..=0x21FF).contains(&code)      // Arrows
        || (0x27F0..=0x27FF).contains(&code)   // Supplemental Arrows-A
        || (0x2900..=0x297F).contains(&code) // Supplemental Arrows-B
}

/// Check if a character is a Unicode superscript.
///
/// Covers: ⁰¹²³⁴⁵⁶⁷⁸⁹⁺⁻⁼⁽⁾ⁿⁱ
/// Unicode range: U+2070-U+207F
fn is_unicode_superscript(c: char) -> bool {
    let code = c as u32;
    (0x2070..=0x207F).contains(&code) || c == '¹' || c == '²' || c == '³' // Latin-1 Supplement superscripts
}

/// Check if a character is a Unicode subscript.
///
/// Covers: ₀₁₂₃₄₅₆₇₈₉₊₋₌₍₎ₐₑₒₓₔₕₖₗₘₙₚₛₜ
/// Unicode range: U+2080-U+209F
fn is_unicode_subscript(c: char) -> bool {
    let code = c as u32;
    (0x2080..=0x209F).contains(&code)
}

/// Information about font usage on a page.
///
/// Contains statistics about how a font is used on a page, including
/// whether it appears to be a mathematical or monospace font.
///
/// # Example
///
/// ```no_run
/// use pdfium_render_fast::Pdfium;
///
/// let pdfium = Pdfium::new()?;
/// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
/// let page = doc.page(0)?;
///
/// for font_info in page.extract_font_usage() {
///     println!("Font: {} ({} chars, {:.1}% coverage)",
///         font_info.name, font_info.char_count, font_info.coverage * 100.0);
///     if font_info.is_math_font {
///         println!("  -> Mathematical font detected");
///     }
/// }
/// # Ok::<(), pdfium_render_fast::PdfError>(())
/// ```
#[derive(Debug, Clone)]
pub struct FontUsageInfo {
    /// Name of the font.
    pub name: String,
    /// Whether this appears to be a mathematical font.
    pub is_math_font: bool,
    /// Whether this appears to be a monospace font.
    pub is_monospace: bool,
    /// Number of characters using this font.
    pub char_count: usize,
    /// Fraction of page characters using this font (0.0 to 1.0).
    pub coverage: f32,
}

impl FontUsageInfo {
    /// Check if this font has significant coverage (>10% of page).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// for font in page.extract_font_usage() {
    ///     if font.has_significant_coverage() {
    ///         println!("Major font: {}", font.name);
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn has_significant_coverage(&self) -> bool {
        self.coverage > 0.10
    }

    /// Check if this is a code/programming font.
    ///
    /// Returns true if the font is monospace and has significant coverage.
    pub fn is_code_font(&self) -> bool {
        self.is_monospace && self.coverage > 0.05
    }
}

/// A text block that appears centered on the page.
///
/// Centered blocks are detected based on symmetric margins relative to page width.
/// Common uses include titles, headings, and centered formulas.
///
/// # Fields
///
/// * `bounds` - Bounding box (left, bottom, right, top) in page coordinates
/// * `text` - The text content of the block
/// * `margin_left` - Distance from left page edge to block left edge
/// * `margin_right` - Distance from block right edge to right page edge
/// * `margin_symmetry` - Measure of how centered: 0.0 = perfect center, higher = less centered
///
/// # Example
///
/// ```no_run
/// use pdfium_render_fast::Pdfium;
///
/// let pdfium = Pdfium::new()?;
/// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
/// let page = doc.page(0)?;
///
/// for block in page.extract_centered_blocks(10.0) {
///     println!("Centered: \"{}\" (symmetry: {:.1})", block.text, block.margin_symmetry);
/// }
/// # Ok::<(), pdfium_render_fast::PdfError>(())
/// ```
#[derive(Debug, Clone)]
pub struct CenteredBlock {
    /// Bounding box (left, bottom, right, top) in page coordinates.
    pub bounds: (f32, f32, f32, f32),
    /// The text content of the centered block.
    pub text: String,
    /// Distance from left page edge to block left edge.
    pub margin_left: f32,
    /// Distance from block right edge to right page edge.
    pub margin_right: f32,
    /// Measure of centering: 0.0 = perfect center, higher values = less centered.
    pub margin_symmetry: f32,
}

impl CenteredBlock {
    /// Create a new centered block.
    pub fn new(
        bounds: (f32, f32, f32, f32),
        text: String,
        margin_left: f32,
        margin_right: f32,
    ) -> Self {
        let margin_symmetry = (margin_left - margin_right).abs();
        Self {
            bounds,
            text,
            margin_left,
            margin_right,
            margin_symmetry,
        }
    }

    /// Check if this block is nearly perfectly centered.
    ///
    /// Returns true if margin_symmetry is less than the given tolerance.
    pub fn is_perfectly_centered(&self, tolerance: f32) -> bool {
        self.margin_symmetry < tolerance
    }

    /// Get the center X position of this block.
    pub fn center_x(&self) -> f32 {
        (self.bounds.0 + self.bounds.2) / 2.0
    }

    /// Get the width of this block.
    pub fn width(&self) -> f32 {
        self.bounds.2 - self.bounds.0
    }

    /// Get the height of this block.
    pub fn height(&self) -> f32 {
        self.bounds.3 - self.bounds.1
    }

    /// Check if this is likely a title (short, large margins).
    pub fn is_likely_title(&self) -> bool {
        // Likely a title if short text and significant margins
        self.text.len() < 100 && self.margin_left > 50.0 && self.margin_right > 50.0
    }
}

/// Type of bracket surrounding a reference.
///
/// Used to distinguish between different citation/reference styles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BracketType {
    /// Square brackets: `[1]`, `[ref]`
    Square,
    /// Parentheses: (1), (ref)
    Paren,
    /// Superscript number without brackets: ¹, ²
    Superscript,
    /// Angle brackets: `<1>`, `<ref>`
    Angle,
}

impl BracketType {
    /// Get the opening bracket character for this type.
    pub fn open_char(&self) -> char {
        match self {
            BracketType::Square => '[',
            BracketType::Paren => '(',
            BracketType::Superscript => '\u{0000}', // No bracket
            BracketType::Angle => '<',
        }
    }

    /// Get the closing bracket character for this type.
    pub fn close_char(&self) -> char {
        match self {
            BracketType::Square => ']',
            BracketType::Paren => ')',
            BracketType::Superscript => '\u{0000}', // No bracket
            BracketType::Angle => '>',
        }
    }
}

/// Position of a reference relative to surrounding text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferencePosition {
    /// Reference appears within the text flow.
    Inline,
    /// Reference appears at the end of a line.
    LineEnd,
    /// Reference appears at the start of a line.
    LineStart,
}

/// A detected bracketed reference (citation, footnote, etc.).
///
/// Bracketed references are common in academic papers and documentation.
/// Examples: `[1]`, `[ref]`, (see note 5), superscript numbers.
///
/// # Example
///
/// ```no_run
/// use pdfium_render_fast::Pdfium;
///
/// let pdfium = Pdfium::new()?;
/// let doc = pdfium.load_pdf_from_file("paper.pdf", None)?;
/// let page = doc.page(0)?;
///
/// for reference in page.extract_bracketed_references() {
///     println!("Reference: {} (type: {:?})", reference.text, reference.bracket_type);
/// }
/// # Ok::<(), pdfium_render_fast::PdfError>(())
/// ```
#[derive(Debug, Clone)]
pub struct BracketedReference {
    /// The text content of the reference (including brackets).
    pub text: String,
    /// Bounding box (left, bottom, right, top) in page coordinates.
    pub bounds: (f32, f32, f32, f32),
    /// Type of bracket used.
    pub bracket_type: BracketType,
    /// Position relative to surrounding text.
    pub position: ReferencePosition,
}

impl BracketedReference {
    /// Create a new bracketed reference.
    pub fn new(
        text: String,
        bounds: (f32, f32, f32, f32),
        bracket_type: BracketType,
        position: ReferencePosition,
    ) -> Self {
        Self {
            text,
            bounds,
            bracket_type,
            position,
        }
    }

    /// Get the inner text without brackets.
    pub fn inner_text(&self) -> &str {
        let t = self.text.trim();
        match self.bracket_type {
            BracketType::Superscript => t,
            _ => {
                // Remove opening and closing brackets
                if t.len() >= 2 {
                    &t[1..t.len() - 1]
                } else {
                    t
                }
            }
        }
    }

    /// Check if this is a numeric reference (e.g., `[1]`, `[23]`).
    pub fn is_numeric(&self) -> bool {
        self.inner_text()
            .chars()
            .all(|c| c.is_ascii_digit() || c == ',' || c == '-' || c == ' ')
    }

    /// Check if this is a range reference (e.g., `[1-5]`, `[3,7]`).
    pub fn is_range(&self) -> bool {
        let inner = self.inner_text();
        inner.contains('-') || inner.contains(',')
    }

    /// Get the width of this reference.
    pub fn width(&self) -> f32 {
        self.bounds.2 - self.bounds.0
    }

    /// Get the height of this reference.
    pub fn height(&self) -> f32 {
        self.bounds.3 - self.bounds.1
    }
}

/// Position of a script character relative to baseline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptPosition {
    /// Superscript (above baseline).
    Super,
    /// Subscript (below baseline).
    Sub,
}

impl ScriptPosition {
    /// Check if this is a superscript position.
    pub fn is_super(&self) -> bool {
        matches!(self, ScriptPosition::Super)
    }

    /// Check if this is a subscript position.
    pub fn is_sub(&self) -> bool {
        matches!(self, ScriptPosition::Sub)
    }
}

/// A character in a superscript or subscript position.
///
/// Used to represent individual characters that are raised (superscript)
/// or lowered (subscript) relative to the baseline text.
#[derive(Debug, Clone)]
pub struct ScriptChar {
    /// The Unicode character.
    pub char: char,
    /// Whether this is a superscript or subscript.
    pub position: ScriptPosition,
    /// Bounding box (left, bottom, right, top) in page coordinates.
    pub bounds: (f32, f32, f32, f32),
    /// Text rise value from PDF (positive = super, negative = sub).
    pub rise: f32,
}

impl ScriptChar {
    /// Create a new script character.
    pub fn new(
        char: char,
        position: ScriptPosition,
        bounds: (f32, f32, f32, f32),
        rise: f32,
    ) -> Self {
        Self {
            char,
            position,
            bounds,
            rise,
        }
    }

    /// Get the width of this character.
    pub fn width(&self) -> f32 {
        self.bounds.2 - self.bounds.0
    }

    /// Get the height of this character.
    pub fn height(&self) -> f32 {
        self.bounds.3 - self.bounds.1
    }
}

/// A cluster of base text with associated superscript/subscript characters.
///
/// This represents common patterns like:
/// - Variable with subscript: x₀, xᵢ, aₙ
/// - Variable with superscript: x², x³, eⁿ
/// - Chemical formulas: H₂O, CO₂
/// - Mathematical expressions: xⁿ⁺¹
///
/// # Example
///
/// ```no_run
/// use pdfium_render_fast::Pdfium;
///
/// let pdfium = Pdfium::new()?;
/// let doc = pdfium.load_pdf_from_file("paper.pdf", None)?;
/// let page = doc.page(0)?;
///
/// for cluster in page.extract_script_clusters() {
///     println!("Base: {}, scripts: {:?}",
///         cluster.base_text,
///         cluster.scripts.iter().map(|s| s.char).collect::<String>());
/// }
/// # Ok::<(), pdfium_render_fast::PdfError>(())
/// ```
#[derive(Debug, Clone)]
pub struct ScriptCluster {
    /// The base text (normal position).
    pub base_text: String,
    /// Bounding box of the base text (left, bottom, right, top).
    pub base_bounds: (f32, f32, f32, f32),
    /// Script characters (superscripts and subscripts).
    pub scripts: Vec<ScriptChar>,
}

impl ScriptCluster {
    /// Create a new script cluster.
    pub fn new(
        base_text: String,
        base_bounds: (f32, f32, f32, f32),
        scripts: Vec<ScriptChar>,
    ) -> Self {
        Self {
            base_text,
            base_bounds,
            scripts,
        }
    }

    /// Get all superscript characters.
    pub fn superscripts(&self) -> Vec<&ScriptChar> {
        self.scripts
            .iter()
            .filter(|s| s.position.is_super())
            .collect()
    }

    /// Get all subscript characters.
    pub fn subscripts(&self) -> Vec<&ScriptChar> {
        self.scripts
            .iter()
            .filter(|s| s.position.is_sub())
            .collect()
    }

    /// Check if this cluster has any superscripts.
    pub fn has_superscripts(&self) -> bool {
        self.scripts.iter().any(|s| s.position.is_super())
    }

    /// Check if this cluster has any subscripts.
    pub fn has_subscripts(&self) -> bool {
        self.scripts.iter().any(|s| s.position.is_sub())
    }

    /// Get the script text as a string.
    pub fn script_text(&self) -> String {
        self.scripts.iter().map(|s| s.char).collect()
    }

    /// Get the combined text (base + scripts).
    pub fn full_text(&self) -> String {
        format!("{}{}", self.base_text, self.script_text())
    }

    /// Get the width of the base text.
    pub fn base_width(&self) -> f32 {
        self.base_bounds.2 - self.base_bounds.0
    }

    /// Get the height of the base text.
    pub fn base_height(&self) -> f32 {
        self.base_bounds.3 - self.base_bounds.1
    }
}

/// Writing direction detected in a PDF page.
///
/// PDF supports both horizontal (left-to-right) and vertical (top-to-bottom, right-to-left)
/// writing directions, commonly used in Japanese, Chinese, and Korean documents.
///
/// # Example
///
/// ```no_run
/// use pdfium_render_fast::{Pdfium, WritingDirection};
///
/// let pdfium = Pdfium::new()?;
/// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
/// let page = doc.page(0)?;
///
/// let info = page.detect_writing_direction();
/// match info.primary_direction {
///     WritingDirection::Horizontal => println!("Standard horizontal text"),
///     WritingDirection::VerticalRTL => println!("Vertical Japanese/Chinese text"),
///     WritingDirection::Mixed => println!("Mixed directions"),
/// }
/// # Ok::<(), pdfium_render_fast::PdfError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum WritingDirection {
    /// Horizontal writing (left-to-right, standard Western/Latin text).
    #[default]
    Horizontal,
    /// Vertical writing, right-to-left column order (Japanese/Chinese traditional).
    VerticalRTL,
    /// Mixed horizontal and vertical content on the same page.
    Mixed,
}

impl WritingDirection {
    /// Check if this is horizontal writing direction.
    pub fn is_horizontal(&self) -> bool {
        matches!(self, WritingDirection::Horizontal)
    }

    /// Check if this is vertical writing direction (RTL columns).
    pub fn is_vertical(&self) -> bool {
        matches!(self, WritingDirection::VerticalRTL)
    }

    /// Check if this is mixed writing direction.
    pub fn is_mixed(&self) -> bool {
        matches!(self, WritingDirection::Mixed)
    }
}

/// Information about writing direction detected in a PDF page.
///
/// Contains the primary direction and detailed metrics about the distribution
/// of horizontal vs vertical text on the page.
///
/// # Example
///
/// ```no_run
/// use pdfium_render_fast::Pdfium;
///
/// let pdfium = Pdfium::new()?;
/// let doc = pdfium.load_pdf_from_file("japanese.pdf", None)?;
/// let page = doc.page(0)?;
///
/// let info = page.detect_writing_direction();
/// println!("Primary: {:?}", info.primary_direction);
/// println!("Vertical ratio: {:.1}%", info.vertical_ratio * 100.0);
/// println!("Vertical regions: {}", info.vertical_regions.len());
/// # Ok::<(), pdfium_render_fast::PdfError>(())
/// ```
#[derive(Debug, Clone)]
pub struct WritingDirectionInfo {
    /// The primary writing direction (most common on the page).
    pub primary_direction: WritingDirection,
    /// Ratio of characters in vertical text (0.0 to 1.0).
    pub vertical_ratio: f32,
    /// Ratio of characters in horizontal text (0.0 to 1.0).
    pub horizontal_ratio: f32,
    /// Bounding boxes of detected vertical text regions (left, bottom, right, top).
    pub vertical_regions: Vec<(f32, f32, f32, f32)>,
}

impl WritingDirectionInfo {
    /// Create writing direction info indicating purely horizontal text.
    pub fn horizontal() -> Self {
        Self {
            primary_direction: WritingDirection::Horizontal,
            vertical_ratio: 0.0,
            horizontal_ratio: 1.0,
            vertical_regions: Vec::new(),
        }
    }

    /// Create writing direction info indicating purely vertical text.
    pub fn vertical_rtl(regions: Vec<(f32, f32, f32, f32)>) -> Self {
        Self {
            primary_direction: WritingDirection::VerticalRTL,
            vertical_ratio: 1.0,
            horizontal_ratio: 0.0,
            vertical_regions: regions,
        }
    }

    /// Check if the page has any vertical text.
    pub fn has_vertical_text(&self) -> bool {
        self.vertical_ratio > 0.0
    }

    /// Check if the page is predominantly vertical text.
    pub fn is_predominantly_vertical(&self) -> bool {
        self.vertical_ratio > 0.5
    }

    /// Check if the page is predominantly horizontal text.
    pub fn is_predominantly_horizontal(&self) -> bool {
        self.horizontal_ratio > 0.5
    }

    /// Get the number of detected vertical text regions.
    pub fn vertical_region_count(&self) -> usize {
        self.vertical_regions.len()
    }
}

impl Default for WritingDirectionInfo {
    fn default() -> Self {
        Self::horizontal()
    }
}

/// A ruby annotation (furigana) pairing base text with reading aid text.
///
/// Ruby text is small text placed above (horizontal writing) or beside (vertical writing)
/// base characters to indicate pronunciation. This is common in Japanese for placing
/// hiragana readings above kanji characters.
///
/// # Example
///
/// ```no_run
/// use pdfium_render_fast::Pdfium;
///
/// let pdfium = Pdfium::new()?;
/// let doc = pdfium.load_pdf_from_file("japanese.pdf", None)?;
/// let page = doc.page(0)?;
///
/// for ruby in page.extract_ruby_annotations() {
///     println!("Base: {}, Ruby: {}", ruby.base_text, ruby.ruby_text);
///     println!("  Size ratio: {:.2}", ruby.size_ratio);
/// }
/// # Ok::<(), pdfium_render_fast::PdfError>(())
/// ```
#[derive(Debug, Clone)]
pub struct RubyAnnotation {
    /// The base text (usually kanji or other characters needing reading aid).
    pub base_text: String,
    /// The ruby text (reading aid, usually hiragana in Japanese).
    pub ruby_text: String,
    /// Bounding box of the base text (left, bottom, right, top).
    pub base_bounds: (f32, f32, f32, f32),
    /// Bounding box of the ruby text (left, bottom, right, top).
    pub ruby_bounds: (f32, f32, f32, f32),
    /// Font size of the ruby text in points.
    pub ruby_font_size: f32,
    /// Ratio of ruby font size to base font size (typically 0.3-0.6).
    pub size_ratio: f32,
}

impl RubyAnnotation {
    /// Create a new ruby annotation.
    pub fn new(
        base_text: String,
        ruby_text: String,
        base_bounds: (f32, f32, f32, f32),
        ruby_bounds: (f32, f32, f32, f32),
        ruby_font_size: f32,
        size_ratio: f32,
    ) -> Self {
        Self {
            base_text,
            ruby_text,
            base_bounds,
            ruby_bounds,
            ruby_font_size,
            size_ratio,
        }
    }

    /// Get the width of the base text.
    pub fn base_width(&self) -> f32 {
        self.base_bounds.2 - self.base_bounds.0
    }

    /// Get the height of the base text.
    pub fn base_height(&self) -> f32 {
        self.base_bounds.3 - self.base_bounds.1
    }

    /// Get the width of the ruby text.
    pub fn ruby_width(&self) -> f32 {
        self.ruby_bounds.2 - self.ruby_bounds.0
    }

    /// Get the height of the ruby text.
    pub fn ruby_height(&self) -> f32 {
        self.ruby_bounds.3 - self.ruby_bounds.1
    }

    /// Check if the ruby is positioned above the base (horizontal writing).
    pub fn is_above(&self) -> bool {
        self.ruby_bounds.1 > self.base_bounds.3 - 1.0
    }

    /// Check if the ruby is positioned to the right of base (vertical writing).
    pub fn is_right_of(&self) -> bool {
        self.ruby_bounds.0 > self.base_bounds.2 - 1.0
    }

    /// Get the combined text (base with ruby in parentheses).
    pub fn combined_text(&self) -> String {
        format!("{}({})", self.base_text, self.ruby_text)
    }
}

/// Analysis of Japanese character types in text.
///
/// Breaks down text into character categories for identifying Japanese content
/// and analyzing writing system composition.
///
/// # Example
///
/// ```no_run
/// use pdfium_render_fast::Pdfium;
///
/// let pdfium = Pdfium::new()?;
/// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
/// let page = doc.page(0)?;
///
/// let analysis = page.analyze_japanese_chars();
/// if analysis.has_japanese() {
///     println!("Hiragana: {}", analysis.hiragana_count);
///     println!("Katakana: {}", analysis.katakana_count);
///     println!("Kanji: {}", analysis.kanji_count);
///     println!("Japanese ratio: {:.1}%", analysis.japanese_ratio() * 100.0);
/// }
/// # Ok::<(), pdfium_render_fast::PdfError>(())
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JapaneseCharAnalysis {
    /// Count of hiragana characters (あいうえお, etc.)
    pub hiragana_count: usize,
    /// Count of katakana characters (アイウエオ, etc.)
    pub katakana_count: usize,
    /// Count of CJK unified ideographs (kanji)
    pub kanji_count: usize,
    /// Count of fullwidth ASCII characters (Ａ-Ｚ, ０-９, etc.)
    pub fullwidth_ascii: usize,
    /// Count of halfwidth katakana characters (ｱ-ﾝ)
    pub halfwidth_katakana: usize,
    /// Count of Japanese punctuation (。、「」etc.)
    pub japanese_punctuation: usize,
    /// Total character count analyzed
    pub total_chars: usize,
}

impl JapaneseCharAnalysis {
    /// Create an empty analysis.
    pub fn new() -> Self {
        Self::default()
    }

    /// Analyze a single character and update counts.
    pub fn analyze_char(&mut self, ch: char) {
        self.total_chars += 1;

        match ch {
            // Hiragana (U+3040-U+309F)
            '\u{3040}'..='\u{309F}' => self.hiragana_count += 1,

            // Katakana (U+30A0-U+30FF)
            '\u{30A0}'..='\u{30FF}' => self.katakana_count += 1,

            // Halfwidth Katakana (U+FF65-U+FF9F)
            '\u{FF65}'..='\u{FF9F}' => self.halfwidth_katakana += 1,

            // Katakana Phonetic Extensions (U+31F0-U+31FF)
            '\u{31F0}'..='\u{31FF}' => self.katakana_count += 1,

            // CJK Unified Ideographs (Kanji) - main block
            '\u{4E00}'..='\u{9FFF}' => self.kanji_count += 1,

            // CJK Extension A
            '\u{3400}'..='\u{4DBF}' => self.kanji_count += 1,

            // CJK Extension B-G (rare kanji in higher planes)
            '\u{20000}'..='\u{2A6DF}'
            | '\u{2A700}'..='\u{2B739}'
            | '\u{2B740}'..='\u{2B81D}'
            | '\u{2B820}'..='\u{2CEA1}'
            | '\u{2CEB0}'..='\u{2EBE0}'
            | '\u{30000}'..='\u{3134A}' => {
                self.kanji_count += 1;
            }

            // CJK Compatibility Ideographs
            '\u{F900}'..='\u{FAFF}' => self.kanji_count += 1,

            // Japanese punctuation (CJK Symbols and Punctuation U+3000-U+303F)
            // Includes: 、。〃「」『』【】〒〓〔〕〖〗〘〙〚〛〜〝〞〟〰々〱〲〳〴〵〆〇
            '\u{3000}'..='\u{303F}' => self.japanese_punctuation += 1,

            // General punctuation that appears in Japanese text (outside other ranges)
            '…' | '‥' => self.japanese_punctuation += 1,

            // Fullwidth ASCII (U+FF01-U+FF5E) - includes fullwidth punctuation
            '\u{FF01}'..='\u{FF5E}' => self.fullwidth_ascii += 1,

            _ => {}
        }
    }

    /// Get total Japanese character count (hiragana + katakana + kanji).
    pub fn japanese_char_count(&self) -> usize {
        self.hiragana_count + self.katakana_count + self.kanji_count + self.halfwidth_katakana
    }

    /// Get ratio of Japanese characters to total characters.
    pub fn japanese_ratio(&self) -> f32 {
        if self.total_chars == 0 {
            0.0
        } else {
            self.japanese_char_count() as f32 / self.total_chars as f32
        }
    }

    /// Check if text contains any Japanese characters.
    pub fn has_japanese(&self) -> bool {
        self.japanese_char_count() > 0
    }

    /// Check if text is predominantly Japanese (> 50% Japanese chars).
    pub fn is_predominantly_japanese(&self) -> bool {
        self.japanese_ratio() > 0.5
    }

    /// Check if text contains hiragana.
    pub fn has_hiragana(&self) -> bool {
        self.hiragana_count > 0
    }

    /// Check if text contains katakana.
    pub fn has_katakana(&self) -> bool {
        self.katakana_count > 0 || self.halfwidth_katakana > 0
    }

    /// Check if text contains kanji.
    pub fn has_kanji(&self) -> bool {
        self.kanji_count > 0
    }

    /// Get kana count (hiragana + katakana).
    pub fn kana_count(&self) -> usize {
        self.hiragana_count + self.katakana_count + self.halfwidth_katakana
    }

    /// Merge another analysis into this one.
    pub fn merge(&mut self, other: &JapaneseCharAnalysis) {
        self.hiragana_count += other.hiragana_count;
        self.katakana_count += other.katakana_count;
        self.kanji_count += other.kanji_count;
        self.fullwidth_ascii += other.fullwidth_ascii;
        self.halfwidth_katakana += other.halfwidth_katakana;
        self.japanese_punctuation += other.japanese_punctuation;
        self.total_chars += other.total_chars;
    }
}

/// Helper function to check if a character is hiragana.
pub fn is_hiragana(ch: char) -> bool {
    matches!(ch, '\u{3040}'..='\u{309F}')
}

/// Helper function to check if a character is katakana.
pub fn is_katakana(ch: char) -> bool {
    matches!(ch, '\u{30A0}'..='\u{30FF}' | '\u{31F0}'..='\u{31FF}' | '\u{FF65}'..='\u{FF9F}')
}

/// Helper function to check if a character is kanji.
pub fn is_kanji(ch: char) -> bool {
    matches!(ch,
        '\u{4E00}'..='\u{9FFF}' |
        '\u{3400}'..='\u{4DBF}' |
        '\u{20000}'..='\u{2A6DF}' |
        '\u{2A700}'..='\u{2B739}' |
        '\u{2B740}'..='\u{2B81D}' |
        '\u{2B820}'..='\u{2CEA1}' |
        '\u{2CEB0}'..='\u{2EBE0}' |
        '\u{30000}'..='\u{3134A}' |
        '\u{F900}'..='\u{FAFF}'
    )
}

/// Helper function to check if a character is Japanese (hiragana, katakana, or kanji).
pub fn is_japanese_char(ch: char) -> bool {
    is_hiragana(ch) || is_katakana(ch) || is_kanji(ch)
}

/// Type of Japanese punctuation mark.
///
/// Classifies Japanese punctuation into semantic categories for text analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JPunctType {
    /// Period (。 U+3002, ． U+FF0E)
    Period,
    /// Comma (、 U+3001, ， U+FF0C)
    Comma,
    /// Opening quote marks (「『〈《【〔〖)
    QuoteOpen,
    /// Closing quote marks (」』〉》】〕〗)
    QuoteClose,
    /// Middle dot/interpunct (・ U+30FB)
    MiddleDot,
    /// Long vowel mark (ー U+30FC)
    LongVowel,
    /// Wave dash and similar (〜 U+301C, ～ U+FF5E)
    WaveDash,
    /// Ideographic space (　 U+3000)
    IdeographicSpace,
    /// Repetition marks (々 U+3005, 〃 U+3003)
    Repetition,
    /// Other Japanese punctuation
    Other,
}

impl JPunctType {
    /// Classify a character into a punctuation type.
    pub fn classify(ch: char) -> Option<Self> {
        match ch {
            // Period
            '。' | '．' => Some(JPunctType::Period),

            // Comma
            '、' | '，' => Some(JPunctType::Comma),

            // Opening quotes
            '「' | '『' | '〈' | '《' | '【' | '〔' | '〖' | '〘' | '〚' | '〝' => {
                Some(JPunctType::QuoteOpen)
            }

            // Closing quotes
            '」' | '』' | '〉' | '》' | '】' | '〕' | '〗' | '〙' | '〛' | '〞' | '〟' => {
                Some(JPunctType::QuoteClose)
            }

            // Middle dot
            '・' => Some(JPunctType::MiddleDot),

            // Long vowel
            'ー' => Some(JPunctType::LongVowel),

            // Wave dash
            '〜' | '～' => Some(JPunctType::WaveDash),

            // Ideographic space
            '\u{3000}' => Some(JPunctType::IdeographicSpace),

            // Repetition marks
            '々' | '〃' => Some(JPunctType::Repetition),

            // Other CJK symbols (U+3000-U+303F minus what we classified above)
            '\u{3001}'..='\u{303F}' => Some(JPunctType::Other),

            // Fullwidth punctuation
            '！' | '？' | '：' | '；' | '（' | '）' => Some(JPunctType::Other),

            _ => None,
        }
    }

    /// Check if this is an opening bracket/quote type.
    pub fn is_opening(&self) -> bool {
        matches!(self, JPunctType::QuoteOpen)
    }

    /// Check if this is a closing bracket/quote type.
    pub fn is_closing(&self) -> bool {
        matches!(self, JPunctType::QuoteClose)
    }
}

/// A Japanese punctuation character with its position and classification.
///
/// # Example
///
/// ```no_run
/// use pdfium_render_fast::Pdfium;
///
/// let pdfium = Pdfium::new()?;
/// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
/// let page = doc.page(0)?;
///
/// for punct in page.extract_japanese_punctuation() {
///     println!("'{}' at ({:.1}, {:.1}), type: {:?}",
///         punct.char, punct.bounds.0, punct.bounds.1, punct.punct_type);
/// }
/// # Ok::<(), pdfium_render_fast::PdfError>(())
/// ```
#[derive(Debug, Clone)]
pub struct JapanesePunctuation {
    /// The punctuation character.
    pub char: char,
    /// Bounding box (left, bottom, right, top) in points.
    pub bounds: (f32, f32, f32, f32),
    /// Classification of the punctuation type.
    pub punct_type: JPunctType,
    /// True if this appears to be a vertical writing variant.
    pub is_vertical_variant: bool,
}

impl JapanesePunctuation {
    /// Create a new Japanese punctuation record.
    pub fn new(
        ch: char,
        bounds: (f32, f32, f32, f32),
        punct_type: JPunctType,
        is_vertical_variant: bool,
    ) -> Self {
        Self {
            char: ch,
            bounds,
            punct_type,
            is_vertical_variant,
        }
    }

    /// Get the width of the character.
    pub fn width(&self) -> f32 {
        self.bounds.2 - self.bounds.0
    }

    /// Get the height of the character.
    pub fn height(&self) -> f32 {
        self.bounds.3 - self.bounds.1
    }

    /// Get the center x-coordinate.
    pub fn center_x(&self) -> f32 {
        (self.bounds.0 + self.bounds.2) / 2.0
    }

    /// Get the center y-coordinate.
    pub fn center_y(&self) -> f32 {
        (self.bounds.1 + self.bounds.3) / 2.0
    }
}

/// Type of emphasis mark (傍点) used in Japanese text.
///
/// Japanese uses small marks placed above (horizontal) or beside (vertical) characters
/// to provide emphasis, similar to italics or bold in Western typography.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EmphasisMarkType {
    /// Solid dot (●, most common)
    Dot,
    /// Hollow circle (○)
    Circle,
    /// Triangle (▲ or △)
    Triangle,
    /// Sesame/gomaten (﹅ filled, ﹆ open) - shaped like a sesame seed
    Sesame,
    /// Other or unidentified mark type
    Other,
}

impl EmphasisMarkType {
    /// Try to classify a character as an emphasis mark type.
    ///
    /// Returns `Some(EmphasisMarkType)` if the character is a recognized emphasis mark,
    /// or `None` if not an emphasis mark.
    pub fn classify(ch: char) -> Option<Self> {
        match ch {
            // Solid dots (not including sesame which has its own category)
            '●' | '•' | '·' | '∙' => Some(EmphasisMarkType::Dot),
            // Hollow circles (not including sesame)
            '○' | '◯' | '◦' => Some(EmphasisMarkType::Circle),
            // Triangles (filled and hollow, pointing various directions)
            '▲' | '△' | '▼' | '▽' | '◆' | '◇' => Some(EmphasisMarkType::Triangle),
            // Sesame marks (filled: ﹅ U+FE45, open: ﹆ U+FE46)
            '\u{FE45}' | '\u{FE46}' => Some(EmphasisMarkType::Sesame),
            _ => None,
        }
    }

    /// Check if this emphasis mark type is filled (solid).
    pub fn is_filled(&self) -> bool {
        matches!(self, EmphasisMarkType::Dot | EmphasisMarkType::Triangle)
    }
}

/// An emphasis mark (傍点) paired with the character it emphasizes.
///
/// Japanese text uses emphasis marks above or beside characters to highlight
/// important words, similar to bold/italic in Western text. These marks are
/// typically small dots or circles positioned above each emphasized character.
///
/// # Example
///
/// ```no_run
/// use pdfium_render_fast::Pdfium;
///
/// let pdfium = Pdfium::new()?;
/// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
/// let page = doc.page(0)?;
///
/// for mark in page.extract_emphasis_marks() {
///     println!("Character '{}' has {:?} emphasis mark",
///         mark.base_char, mark.mark_type);
/// }
/// # Ok::<(), pdfium_render_fast::PdfError>(())
/// ```
#[derive(Debug, Clone)]
pub struct EmphasisMark {
    /// The base character being emphasized.
    pub base_char: char,
    /// Bounding box of the base character (left, bottom, right, top) in points.
    pub base_bounds: (f32, f32, f32, f32),
    /// Bounding box of the emphasis mark (left, bottom, right, top) in points.
    pub mark_bounds: (f32, f32, f32, f32),
    /// The type of emphasis mark.
    pub mark_type: EmphasisMarkType,
}

impl EmphasisMark {
    /// Create a new emphasis mark.
    pub fn new(
        base_char: char,
        base_bounds: (f32, f32, f32, f32),
        mark_bounds: (f32, f32, f32, f32),
        mark_type: EmphasisMarkType,
    ) -> Self {
        Self {
            base_char,
            base_bounds,
            mark_bounds,
            mark_type,
        }
    }

    /// Get the center x-coordinate of the base character.
    pub fn base_center_x(&self) -> f32 {
        (self.base_bounds.0 + self.base_bounds.2) / 2.0
    }

    /// Get the center y-coordinate of the base character.
    pub fn base_center_y(&self) -> f32 {
        (self.base_bounds.1 + self.base_bounds.3) / 2.0
    }

    /// Get the center x-coordinate of the emphasis mark.
    pub fn mark_center_x(&self) -> f32 {
        (self.mark_bounds.0 + self.mark_bounds.2) / 2.0
    }

    /// Get the center y-coordinate of the emphasis mark.
    pub fn mark_center_y(&self) -> f32 {
        (self.mark_bounds.1 + self.mark_bounds.3) / 2.0
    }

    /// Check if the mark appears to be in horizontal text (mark above base).
    pub fn is_horizontal_layout(&self) -> bool {
        // Mark is above base if mark's center y is greater than base's center y
        self.mark_center_y() > self.base_center_y()
    }

    /// Check if the mark appears to be in vertical text (mark beside base).
    pub fn is_vertical_layout(&self) -> bool {
        // Mark is beside base in vertical text (typically to the right)
        !self.is_horizontal_layout()
    }
}

/// A point where horizontal and vertical grid lines intersect.
///
/// Used for detecting table structure in PDFs by analyzing where
/// visible lines cross to form a grid.
#[derive(Debug, Clone)]
pub struct GridIntersection {
    /// The intersection point (x, y) in page coordinates.
    pub point: (f32, f32),
    /// Index of the horizontal line at this intersection (if any).
    pub horizontal_line_idx: Option<usize>,
    /// Index of the vertical line at this intersection (if any).
    pub vertical_line_idx: Option<usize>,
}

impl GridIntersection {
    /// Create a new grid intersection.
    pub fn new(
        point: (f32, f32),
        horizontal_line_idx: Option<usize>,
        vertical_line_idx: Option<usize>,
    ) -> Self {
        Self {
            point,
            horizontal_line_idx,
            vertical_line_idx,
        }
    }

    /// Get the x-coordinate of this intersection.
    pub fn x(&self) -> f32 {
        self.point.0
    }

    /// Get the y-coordinate of this intersection.
    pub fn y(&self) -> f32 {
        self.point.1
    }

    /// Check if this is a true intersection (both lines present).
    pub fn is_full_intersection(&self) -> bool {
        self.horizontal_line_idx.is_some() && self.vertical_line_idx.is_some()
    }
}

/// Analysis of grid lines on a page, useful for table detection.
///
/// Provides information about horizontal and vertical lines,
/// their intersections, and the cells they form.
///
/// # Example
///
/// ```no_run
/// use pdfium_render_fast::Pdfium;
///
/// let pdfium = Pdfium::new()?;
/// let doc = pdfium.load_pdf_from_file("table.pdf", None)?;
/// let page = doc.page(0)?;
///
/// let grid = page.analyze_grid_lines();
/// println!("Found {} intersections forming {} potential cells",
///     grid.intersections.len(), grid.cell_bounds.len());
/// # Ok::<(), pdfium_render_fast::PdfError>(())
/// ```
#[derive(Debug, Clone)]
pub struct GridAnalysis {
    /// All detected intersections of horizontal and vertical lines.
    pub intersections: Vec<GridIntersection>,
    /// Y-coordinates of horizontal lines (row separators).
    pub row_separators: Vec<f32>,
    /// X-coordinates of vertical lines (column separators).
    pub column_separators: Vec<f32>,
    /// Bounds of detected cells (left, bottom, right, top).
    pub cell_bounds: Vec<(f32, f32, f32, f32)>,
}

impl GridAnalysis {
    /// Create a new empty grid analysis.
    pub fn new() -> Self {
        Self {
            intersections: Vec::new(),
            row_separators: Vec::new(),
            column_separators: Vec::new(),
            cell_bounds: Vec::new(),
        }
    }

    /// Get the number of rows detected.
    pub fn row_count(&self) -> usize {
        if self.row_separators.len() > 1 {
            self.row_separators.len() - 1
        } else {
            0
        }
    }

    /// Get the number of columns detected.
    pub fn column_count(&self) -> usize {
        if self.column_separators.len() > 1 {
            self.column_separators.len() - 1
        } else {
            0
        }
    }

    /// Get the number of cells detected.
    pub fn cell_count(&self) -> usize {
        self.cell_bounds.len()
    }

    /// Check if this appears to be a valid table grid.
    ///
    /// Returns true if there are at least 2 rows and 2 columns.
    pub fn is_valid_table(&self) -> bool {
        self.row_count() >= 2 && self.column_count() >= 2
    }

    /// Get the bounds of the entire grid (if any).
    pub fn bounds(&self) -> Option<(f32, f32, f32, f32)> {
        if self.row_separators.is_empty() || self.column_separators.is_empty() {
            return None;
        }

        let min_x = self
            .column_separators
            .iter()
            .cloned()
            .fold(f32::INFINITY, f32::min);
        let max_x = self
            .column_separators
            .iter()
            .cloned()
            .fold(f32::NEG_INFINITY, f32::max);
        let min_y = self
            .row_separators
            .iter()
            .cloned()
            .fold(f32::INFINITY, f32::min);
        let max_y = self
            .row_separators
            .iter()
            .cloned()
            .fold(f32::NEG_INFINITY, f32::max);

        Some((min_x, min_y, max_x, max_y))
    }
}

impl Default for GridAnalysis {
    fn default() -> Self {
        Self::new()
    }
}

/// Type of column alignment detected in text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AlignmentType {
    /// Left-aligned text.
    Left,
    /// Right-aligned text.
    Right,
    /// Center-aligned text.
    Center,
    /// Decimal-aligned (numbers aligned on decimal point).
    Decimal,
}

impl AlignmentType {
    /// Check if this alignment is for numeric content.
    pub fn is_numeric_alignment(&self) -> bool {
        matches!(self, AlignmentType::Right | AlignmentType::Decimal)
    }
}

/// A detected column of aligned text.
///
/// Used for detecting table-like structures even without visible grid lines,
/// by analyzing text alignment patterns.
#[derive(Debug, Clone)]
pub struct AlignedColumn {
    /// X-coordinate of the alignment point.
    pub x_position: f32,
    /// Type of alignment detected.
    pub alignment: AlignmentType,
    /// Indices of text lines that participate in this column.
    pub line_indices: Vec<usize>,
    /// Confidence score (0.0 to 1.0) for this alignment detection.
    pub confidence: f32,
}

impl AlignedColumn {
    /// Create a new aligned column.
    pub fn new(
        x_position: f32,
        alignment: AlignmentType,
        line_indices: Vec<usize>,
        confidence: f32,
    ) -> Self {
        Self {
            x_position,
            alignment,
            line_indices,
            confidence: confidence.clamp(0.0, 1.0),
        }
    }

    /// Get the number of lines in this column.
    pub fn line_count(&self) -> usize {
        self.line_indices.len()
    }

    /// Check if this is a high-confidence detection.
    pub fn is_high_confidence(&self) -> bool {
        self.confidence >= 0.8
    }
}

/// Orientation of a whitespace gap in a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GapOrientation {
    /// Horizontal gap (separates content vertically).
    Horizontal,
    /// Vertical gap (separates content horizontally).
    Vertical,
}

/// A whitespace gap detected between content regions.
///
/// Used for detecting table-like structures without visible borders
/// by analyzing the spacing between text blocks.
#[derive(Debug, Clone)]
pub struct WhitespaceGap {
    /// Bounding box of the gap (left, bottom, right, top) in points.
    pub bounds: (f32, f32, f32, f32),
    /// Orientation of this gap.
    pub orientation: GapOrientation,
    /// Size of the gap (width for vertical, height for horizontal).
    pub gap_size: f32,
}

impl WhitespaceGap {
    /// Create a new whitespace gap.
    pub fn new(bounds: (f32, f32, f32, f32), orientation: GapOrientation) -> Self {
        let gap_size = match orientation {
            GapOrientation::Horizontal => bounds.3 - bounds.1, // height
            GapOrientation::Vertical => bounds.2 - bounds.0,   // width
        };
        Self {
            bounds,
            orientation,
            gap_size,
        }
    }

    /// Get the width of the gap region.
    pub fn width(&self) -> f32 {
        self.bounds.2 - self.bounds.0
    }

    /// Get the height of the gap region.
    pub fn height(&self) -> f32 {
        self.bounds.3 - self.bounds.1
    }

    /// Check if this is a vertical gap.
    pub fn is_vertical(&self) -> bool {
        self.orientation == GapOrientation::Vertical
    }

    /// Check if this is a horizontal gap.
    pub fn is_horizontal(&self) -> bool {
        self.orientation == GapOrientation::Horizontal
    }
}

/// Analysis of whitespace gaps forming a potential table grid.
///
/// Detects table-like structures by finding consistent horizontal
/// and vertical gaps that could represent row and column separators.
#[derive(Debug, Clone)]
pub struct GapMatrix {
    /// Horizontal gaps (potential row separators).
    pub horizontal_gaps: Vec<WhitespaceGap>,
    /// Vertical gaps (potential column separators).
    pub vertical_gaps: Vec<WhitespaceGap>,
    /// Estimated (rows, columns) based on gap pattern.
    pub potential_cells: (usize, usize),
}

impl GapMatrix {
    /// Create a new empty gap matrix.
    pub fn new() -> Self {
        Self {
            horizontal_gaps: Vec::new(),
            vertical_gaps: Vec::new(),
            potential_cells: (0, 0),
        }
    }

    /// Check if there are any gaps detected.
    pub fn has_gaps(&self) -> bool {
        !self.horizontal_gaps.is_empty() || !self.vertical_gaps.is_empty()
    }

    /// Get the total number of gaps.
    pub fn gap_count(&self) -> usize {
        self.horizontal_gaps.len() + self.vertical_gaps.len()
    }

    /// Check if the gaps suggest a table-like structure.
    pub fn suggests_table(&self) -> bool {
        self.potential_cells.0 >= 2 && self.potential_cells.1 >= 2
    }
}

impl Default for GapMatrix {
    fn default() -> Self {
        Self::new()
    }
}

/// Pattern of alternating row backgrounds in a table.
///
/// Many tables use alternating colored backgrounds (zebra striping)
/// for readability. This struct captures such patterns.
#[derive(Debug, Clone)]
pub struct AlternatingPattern {
    /// Bounds of each detected row (left, bottom, right, top).
    pub row_bounds: Vec<(f32, f32, f32, f32)>,
    /// Background color of each row (None if transparent).
    pub colors: Vec<Option<(u8, u8, u8, u8)>>,
    /// Whether the colors actually alternate.
    pub is_alternating: bool,
    /// Period of the alternation (typically 2 for zebra stripes).
    pub period: usize,
}

impl AlternatingPattern {
    /// Create a new alternating pattern.
    pub fn new(
        row_bounds: Vec<(f32, f32, f32, f32)>,
        colors: Vec<Option<(u8, u8, u8, u8)>>,
    ) -> Self {
        let (is_alternating, period) = Self::detect_alternation(&colors);
        Self {
            row_bounds,
            colors,
            is_alternating,
            period,
        }
    }

    /// Detect if colors alternate and with what period.
    fn detect_alternation(colors: &[Option<(u8, u8, u8, u8)>]) -> (bool, usize) {
        if colors.len() < 2 {
            return (false, 0);
        }

        // Check for period-2 alternation (most common: zebra stripes)
        if colors.len() >= 4 {
            let even_colors: Vec<_> = colors.iter().step_by(2).collect();
            let odd_colors: Vec<_> = colors.iter().skip(1).step_by(2).collect();

            // Check if all even rows have same color and all odd rows have same color
            let even_same = even_colors.windows(2).all(|w| w[0] == w[1]);
            let odd_same = odd_colors.windows(2).all(|w| w[0] == w[1]);

            if even_same && odd_same && even_colors.first() != odd_colors.first() {
                return (true, 2);
            }
        }

        (false, 0)
    }

    /// Get the number of rows in the pattern.
    pub fn row_count(&self) -> usize {
        self.row_bounds.len()
    }

    /// Check if this is a zebra stripe pattern (period 2).
    pub fn is_zebra_stripe(&self) -> bool {
        self.is_alternating && self.period == 2
    }
}

/// A region containing primarily numeric content.
///
/// Useful for identifying data columns in tables that contain
/// numbers, percentages, currency values, etc.
#[derive(Debug, Clone)]
pub struct NumericRegion {
    /// Bounding box of the region (left, bottom, right, top) in points.
    pub bounds: (f32, f32, f32, f32),
    /// Ratio of numeric characters to total characters (0.0 to 1.0).
    pub numeric_ratio: f32,
    /// Whether decimal points were found.
    pub has_decimals: bool,
    /// Whether currency symbols were found ($, €, ¥, £, etc.).
    pub has_currency: bool,
    /// Whether percentage signs were found.
    pub has_percentages: bool,
    /// Detected alignment of numbers in this region.
    pub alignment: AlignmentType,
}

impl NumericRegion {
    /// Create a new numeric region.
    pub fn new(
        bounds: (f32, f32, f32, f32),
        numeric_ratio: f32,
        has_decimals: bool,
        has_currency: bool,
        has_percentages: bool,
        alignment: AlignmentType,
    ) -> Self {
        Self {
            bounds,
            numeric_ratio: numeric_ratio.clamp(0.0, 1.0),
            has_decimals,
            has_currency,
            has_percentages,
            alignment,
        }
    }

    /// Check if this region is primarily numeric (>50% numeric characters).
    pub fn is_primarily_numeric(&self) -> bool {
        self.numeric_ratio > 0.5
    }

    /// Check if this appears to be a financial data column.
    pub fn is_financial(&self) -> bool {
        self.has_currency || (self.has_decimals && self.numeric_ratio > 0.6)
    }

    /// Check if this appears to be a percentage column.
    pub fn is_percentage_column(&self) -> bool {
        self.has_percentages && self.numeric_ratio > 0.4
    }

    /// Get the width of this region.
    pub fn width(&self) -> f32 {
        self.bounds.2 - self.bounds.0
    }

    /// Get the height of this region.
    pub fn height(&self) -> f32 {
        self.bounds.3 - self.bounds.1
    }
}

/// A cluster of related text blocks grouped by proximity.
///
/// Text clustering helps identify logical groupings of text that may
/// represent paragraphs, columns, or other layout structures.
#[derive(Debug, Clone)]
pub struct TextCluster {
    /// Bounding box of the cluster (left, bottom, right, top) in points.
    pub bounds: (f32, f32, f32, f32),
    /// Total character count in this cluster.
    pub char_count: usize,
    /// Number of text lines in this cluster.
    pub line_count: usize,
    /// Gap above this cluster to the nearest content.
    pub gap_above: f32,
    /// Gap below this cluster to the nearest content.
    pub gap_below: f32,
    /// Gap to the left of this cluster.
    pub gap_left: f32,
    /// Gap to the right of this cluster.
    pub gap_right: f32,
}

impl TextCluster {
    /// Create a new text cluster.
    pub fn new(bounds: (f32, f32, f32, f32), char_count: usize, line_count: usize) -> Self {
        Self {
            bounds,
            char_count,
            line_count,
            gap_above: 0.0,
            gap_below: 0.0,
            gap_left: 0.0,
            gap_right: 0.0,
        }
    }

    /// Get the width of this cluster.
    pub fn width(&self) -> f32 {
        self.bounds.2 - self.bounds.0
    }

    /// Get the height of this cluster.
    pub fn height(&self) -> f32 {
        self.bounds.3 - self.bounds.1
    }

    /// Get the center point of this cluster.
    pub fn center(&self) -> (f32, f32) {
        (
            (self.bounds.0 + self.bounds.2) / 2.0,
            (self.bounds.1 + self.bounds.3) / 2.0,
        )
    }

    /// Check if this cluster is isolated (significant gaps on all sides).
    pub fn is_isolated(&self, min_gap: f32) -> bool {
        self.gap_above >= min_gap
            && self.gap_below >= min_gap
            && self.gap_left >= min_gap
            && self.gap_right >= min_gap
    }
}

/// A line with its detected indentation level.
#[derive(Debug, Clone)]
pub struct IndentedLine {
    /// Index of this line in the page.
    pub line_index: usize,
    /// Indentation in points from the base margin.
    pub indent_px: f32,
    /// Indentation level (0 = no indent, 1 = first level, etc.).
    pub indent_level: usize,
    /// Bounding box of this line (left, bottom, right, top) in points.
    pub bounds: (f32, f32, f32, f32),
}

impl IndentedLine {
    /// Create a new indented line.
    pub fn new(
        line_index: usize,
        indent_px: f32,
        indent_level: usize,
        bounds: (f32, f32, f32, f32),
    ) -> Self {
        Self {
            line_index,
            indent_px,
            indent_level,
            bounds,
        }
    }

    /// Check if this line is indented.
    pub fn is_indented(&self) -> bool {
        self.indent_level > 0
    }
}

/// Analysis of indentation patterns on a page.
///
/// Detects indentation levels used for lists, quotes, code blocks, etc.
#[derive(Debug, Clone)]
pub struct IndentationAnalysis {
    /// The base left margin in points.
    pub base_margin: f32,
    /// The typical increment between indent levels in points.
    pub indent_increment: f32,
    /// Lines with their detected indentation.
    pub lines: Vec<IndentedLine>,
    /// Maximum indent level found.
    pub max_level: usize,
}

impl IndentationAnalysis {
    /// Create a new indentation analysis.
    pub fn new() -> Self {
        Self {
            base_margin: 0.0,
            indent_increment: 0.0,
            lines: Vec::new(),
            max_level: 0,
        }
    }

    /// Get count of indented lines.
    pub fn indented_line_count(&self) -> usize {
        self.lines.iter().filter(|l| l.is_indented()).count()
    }

    /// Check if this page has significant indentation structure.
    pub fn has_indentation(&self) -> bool {
        self.max_level > 0 && self.indented_line_count() > 0
    }

    /// Get lines at a specific indent level.
    pub fn lines_at_level(&self, level: usize) -> Vec<&IndentedLine> {
        self.lines
            .iter()
            .filter(|l| l.indent_level == level)
            .collect()
    }
}

impl Default for IndentationAnalysis {
    fn default() -> Self {
        Self::new()
    }
}

/// Type of list marker detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ListMarkerType {
    /// Bullet point (•, ◦, ▪, etc.)
    Bullet,
    /// Dash or hyphen (-, –, —)
    Dash,
    /// Asterisk (*)
    Asterisk,
    /// Number followed by period (1., 2., etc.)
    NumberDot,
    /// Number in parentheses ((1), (2), etc.) or followed by paren (1), 2))
    NumberParen,
    /// Letter followed by period (a., b., A., B., etc.)
    LetterDot,
    /// Letter in parentheses ((a), (b), etc.) or followed by paren
    LetterParen,
    /// Roman numeral (i., ii., I., II., etc.)
    Roman,
    /// Custom or unrecognized marker
    Custom,
}

impl ListMarkerType {
    /// Check if this is a numbered list type.
    pub fn is_numbered(&self) -> bool {
        matches!(
            self,
            ListMarkerType::NumberDot
                | ListMarkerType::NumberParen
                | ListMarkerType::LetterDot
                | ListMarkerType::LetterParen
                | ListMarkerType::Roman
        )
    }

    /// Check if this is a bullet/unordered list type.
    pub fn is_bullet(&self) -> bool {
        matches!(
            self,
            ListMarkerType::Bullet | ListMarkerType::Dash | ListMarkerType::Asterisk
        )
    }

    /// Detect marker type from text.
    pub fn detect(text: &str) -> Option<Self> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return None;
        }

        // Check for bullet characters
        if trimmed.len() == 1 {
            let ch = trimmed.chars().next().unwrap();
            match ch {
                '•' | '◦' | '▪' | '▫' | '●' | '○' | '■' | '□' | '►' | '▸' => {
                    return Some(ListMarkerType::Bullet)
                }
                '-' | '–' | '—' => return Some(ListMarkerType::Dash),
                '*' => return Some(ListMarkerType::Asterisk),
                _ => {}
            }
        }

        // Check for numbered patterns
        // Number followed by dot: "1." "12." etc.
        if trimmed.ends_with('.')
            && trimmed[..trimmed.len() - 1]
                .chars()
                .all(|c| c.is_ascii_digit())
        {
            return Some(ListMarkerType::NumberDot);
        }

        // Number followed by paren: "1)" "12)" etc.
        if trimmed.ends_with(')')
            && trimmed[..trimmed.len() - 1]
                .chars()
                .all(|c| c.is_ascii_digit())
        {
            return Some(ListMarkerType::NumberParen);
        }

        // Letter followed by dot: "a." "B." etc.
        if trimmed.len() == 2 && trimmed.ends_with('.') {
            let first = trimmed.chars().next().unwrap();
            if first.is_ascii_alphabetic() {
                return Some(ListMarkerType::LetterDot);
            }
        }

        // Letter followed by paren: "a)" "B)" etc.
        if trimmed.len() == 2 && trimmed.ends_with(')') {
            let first = trimmed.chars().next().unwrap();
            if first.is_ascii_alphabetic() {
                return Some(ListMarkerType::LetterParen);
            }
        }

        // Roman numerals: i., ii., iii., iv., v., I., II., etc.
        let roman_chars = ['i', 'v', 'x', 'I', 'V', 'X'];
        if let Some(prefix) = trimmed.strip_suffix('.') {
            if !prefix.is_empty() && prefix.chars().all(|c| roman_chars.contains(&c)) {
                return Some(ListMarkerType::Roman);
            }
        }

        None
    }
}

/// A detected list marker and its position.
#[derive(Debug, Clone)]
pub struct ListMarker {
    /// Type of list marker.
    pub marker_type: ListMarkerType,
    /// The actual marker text.
    pub marker_text: String,
    /// Bounding box of the marker (left, bottom, right, top) in points.
    pub marker_bounds: (f32, f32, f32, f32),
    /// X-coordinate where the content following the marker starts.
    pub content_start_x: f32,
}

impl ListMarker {
    /// Create a new list marker.
    pub fn new(
        marker_type: ListMarkerType,
        marker_text: String,
        marker_bounds: (f32, f32, f32, f32),
        content_start_x: f32,
    ) -> Self {
        Self {
            marker_type,
            marker_text,
            marker_bounds,
            content_start_x,
        }
    }

    /// Get the width of the marker.
    pub fn width(&self) -> f32 {
        self.marker_bounds.2 - self.marker_bounds.0
    }

    /// Get the gap between marker and content.
    pub fn marker_content_gap(&self) -> f32 {
        self.content_start_x - self.marker_bounds.2
    }
}

/// A detected column gutter (vertical whitespace between columns).
#[derive(Debug, Clone)]
pub struct ColumnGutter {
    /// X-coordinate of the gutter center.
    pub x_position: f32,
    /// Top Y-coordinate of the gutter region.
    pub top_y: f32,
    /// Bottom Y-coordinate of the gutter region.
    pub bottom_y: f32,
    /// Width of the gutter in points.
    pub width: f32,
    /// Confidence score (0.0-1.0) for this gutter detection.
    pub confidence: f32,
}

impl ColumnGutter {
    /// Create a new column gutter.
    pub fn new(x_position: f32, top_y: f32, bottom_y: f32, width: f32, confidence: f32) -> Self {
        Self {
            x_position,
            top_y,
            bottom_y,
            width,
            confidence: confidence.clamp(0.0, 1.0),
        }
    }

    /// Get the height of this gutter.
    pub fn height(&self) -> f32 {
        self.top_y - self.bottom_y
    }

    /// Check if this is a high-confidence gutter.
    pub fn is_high_confidence(&self) -> bool {
        self.confidence >= 0.7
    }
}

/// Analysis of column layout on a page.
#[derive(Debug, Clone)]
pub struct ColumnLayout {
    /// Detected column gutters.
    pub gutters: Vec<ColumnGutter>,
    /// Number of columns detected (gutters.len() + 1, or 1 if no gutters).
    pub column_count: usize,
    /// Bounding boxes for each column (left, bottom, right, top).
    pub column_bounds: Vec<(f32, f32, f32, f32)>,
}

impl ColumnLayout {
    /// Create a new column layout.
    pub fn new() -> Self {
        Self {
            gutters: Vec::new(),
            column_count: 1,
            column_bounds: Vec::new(),
        }
    }

    /// Check if this page has multiple columns.
    pub fn is_multi_column(&self) -> bool {
        self.column_count > 1
    }

    /// Get the average gutter width.
    pub fn average_gutter_width(&self) -> Option<f32> {
        if self.gutters.is_empty() {
            None
        } else {
            let sum: f32 = self.gutters.iter().map(|g| g.width).sum();
            Some(sum / self.gutters.len() as f32)
        }
    }
}

impl Default for ColumnLayout {
    fn default() -> Self {
        Self::new()
    }
}

/// A cell in a content density heatmap.
#[derive(Debug, Clone)]
pub struct DensityCell {
    /// Bounding box of this cell (left, bottom, right, top).
    pub bounds: (f32, f32, f32, f32),
    /// Text content density (0.0-1.0, ratio of area covered by text).
    pub text_density: f32,
    /// Image coverage (0.0-1.0, ratio of area covered by images).
    pub image_coverage: f32,
    /// Line coverage (0.0-1.0, ratio of area covered by lines/paths).
    pub line_coverage: f32,
}

impl DensityCell {
    /// Create a new density cell.
    pub fn new(
        bounds: (f32, f32, f32, f32),
        text_density: f32,
        image_coverage: f32,
        line_coverage: f32,
    ) -> Self {
        Self {
            bounds,
            text_density: text_density.clamp(0.0, 1.0),
            image_coverage: image_coverage.clamp(0.0, 1.0),
            line_coverage: line_coverage.clamp(0.0, 1.0),
        }
    }

    /// Get the width of this cell.
    pub fn width(&self) -> f32 {
        self.bounds.2 - self.bounds.0
    }

    /// Get the height of this cell.
    pub fn height(&self) -> f32 {
        self.bounds.3 - self.bounds.1
    }

    /// Get total content coverage.
    pub fn total_coverage(&self) -> f32 {
        (self.text_density + self.image_coverage + self.line_coverage).min(1.0)
    }

    /// Check if this cell is mostly empty.
    pub fn is_empty(&self) -> bool {
        self.total_coverage() < 0.1
    }

    /// Check if this cell is primarily text.
    pub fn is_text_dominant(&self) -> bool {
        self.text_density > self.image_coverage && self.text_density > self.line_coverage
    }
}

/// A content density heatmap for a page.
#[derive(Debug, Clone)]
pub struct DensityMap {
    /// Grid dimensions (rows, columns).
    pub grid_size: (usize, usize),
    /// 2D grid of density cells (indexed by row then column).
    pub cells: Vec<Vec<DensityCell>>,
}

impl DensityMap {
    /// Create a new empty density map.
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            grid_size: (rows, cols),
            cells: Vec::new(),
        }
    }

    /// Get the cell at the given row and column.
    pub fn cell(&self, row: usize, col: usize) -> Option<&DensityCell> {
        self.cells.get(row).and_then(|r| r.get(col))
    }

    /// Get total number of cells.
    pub fn cell_count(&self) -> usize {
        self.grid_size.0 * self.grid_size.1
    }

    /// Get count of empty cells.
    pub fn empty_cell_count(&self) -> usize {
        self.cells.iter().flatten().filter(|c| c.is_empty()).count()
    }

    /// Get average text density across all cells.
    pub fn average_text_density(&self) -> f32 {
        let cells: Vec<&DensityCell> = self.cells.iter().flatten().collect();
        if cells.is_empty() {
            return 0.0;
        }
        let sum: f32 = cells.iter().map(|c| c.text_density).sum();
        sum / cells.len() as f32
    }
}

/// Check if a font name indicates a known mathematical font.
///
/// This function recognizes common mathematical fonts by name pattern.
///
/// # Arguments
///
/// * `name` - The font name to check
///
/// # Returns
///
/// `true` if the font name matches a known mathematical font pattern.
///
/// # Example
///
/// ```no_run
/// use pdfium_render_fast::is_known_math_font;
///
/// assert!(is_known_math_font("CMMI10"));  // Computer Modern Math Italic
/// assert!(is_known_math_font("Symbol"));
/// assert!(is_known_math_font("STIXMath"));
/// assert!(!is_known_math_font("Arial"));
/// ```
pub fn is_known_math_font(name: &str) -> bool {
    let lower = name.to_lowercase();

    // Computer Modern (TeX) fonts
    if lower.starts_with("cm") {
        // CMMI (math italic), CMSY (symbols), CMEX (extensions)
        return lower.contains("mi") || lower.contains("sy") || lower.contains("ex");
    }

    // Check for specific math font names
    let math_patterns = [
        "math",
        "symbol",
        "stix",
        "euler",
        "fraktur",
        "script",
        "blackboard",
        "msbm",
        "msam",
        "rsfs",
        "eufm",
        "eusm",
        "euex",
        "mtsy",
        "mtex",
        "cmex",
        "cmsy",
        "cmmi",
        "cambria math",
        "asana math",
        "xits math",
        "latinmodern",
        "libertinus math",
        "dejavu math",
        "lucida math",
    ];

    for pattern in &math_patterns {
        if lower.contains(pattern) {
            return true;
        }
    }

    false
}

/// Check if a font name indicates a monospace/fixed-width font.
///
/// # Arguments
///
/// * `name` - The font name to check
///
/// # Returns
///
/// `true` if the font name matches a known monospace font pattern.
fn is_known_monospace_font(name: &str) -> bool {
    let lower = name.to_lowercase();

    let mono_patterns = [
        "mono",
        "courier",
        "consolas",
        "menlo",
        "monaco",
        "inconsolata",
        "fira code",
        "source code",
        "jetbrains",
        "hack",
        "anonymous",
        "ubuntu mono",
        "droid sans mono",
        "dejavu sans mono",
        "liberation mono",
        "lucida console",
        "terminal",
        "fixed",
        "typewriter",
        "cmtt", // Computer Modern Typewriter
    ];

    for pattern in &mono_patterns {
        if lower.contains(pattern) {
            return true;
        }
    }

    false
}

/// Content from a page that may be either raw JPEG or rendered bitmap.
///
/// Used by [`PdfPage::smart_render()`] to automatically choose the fastest path:
/// - Scanned pages: Returns raw JPEG bytes (545x faster)
/// - Normal pages: Returns rendered bitmap
///
/// # Example
///
/// ```no_run
/// use pdfium_render_fast::{Pdfium, PdfRenderConfig, ScannedPageContent};
///
/// let pdfium = Pdfium::new()?;
/// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
/// let page = doc.page(0)?;
///
/// let config = PdfRenderConfig::new().set_target_dpi(300.0);
/// match page.smart_render(&config)? {
///     ScannedPageContent::Jpeg(data) => {
///         std::fs::write("page.jpg", &data)?;
///     }
///     ScannedPageContent::Rendered(bitmap) => {
///         bitmap.save_as_png("page.png")?;
///     }
/// }
/// # Ok::<(), pdfium_render_fast::PdfError>(())
/// ```
pub enum ScannedPageContent {
    /// Raw JPEG data extracted directly from the PDF (scanned page).
    /// This bypasses rendering entirely for massive speedup.
    Jpeg(Vec<u8>),
    /// Rendered bitmap for pages that need actual rendering.
    Rendered(PdfBitmap),
}

impl ScannedPageContent {
    /// Returns true if this is raw JPEG data (scanned page).
    pub fn is_jpeg(&self) -> bool {
        matches!(self, ScannedPageContent::Jpeg(_))
    }

    /// Returns true if this is a rendered bitmap.
    pub fn is_rendered(&self) -> bool {
        matches!(self, ScannedPageContent::Rendered(_))
    }

    /// Get the JPEG data if this is a scanned page.
    pub fn as_jpeg(&self) -> Option<&[u8]> {
        match self {
            ScannedPageContent::Jpeg(data) => Some(data),
            _ => None,
        }
    }

    /// Get the bitmap if this is a rendered page.
    pub fn as_bitmap(&self) -> Option<&PdfBitmap> {
        match self {
            ScannedPageContent::Rendered(bitmap) => Some(bitmap),
            _ => None,
        }
    }

    /// Get the raw JPEG data if this is a scanned page, or None for rendered pages.
    ///
    /// For scanned pages, this returns the raw JPEG bytes.
    /// For rendered pages, use `as_bitmap()` and save to a file instead.
    pub fn into_jpeg(self) -> Option<Vec<u8>> {
        match self {
            ScannedPageContent::Jpeg(data) => Some(data),
            ScannedPageContent::Rendered(_) => None,
        }
    }

    /// Save to a file, choosing format based on content type.
    ///
    /// Scanned pages are saved as JPEG, rendered pages as PNG.
    pub fn save<P: AsRef<std::path::Path>>(&self, path: P) -> Result<()> {
        match self {
            ScannedPageContent::Jpeg(data) => {
                std::fs::write(path, data).map_err(|e| PdfError::IoError {
                    message: format!("Failed to write JPEG: {}", e),
                })
            }
            ScannedPageContent::Rendered(bitmap) => bitmap.save_as_png(path),
        }
    }
}

/// A rectangular box defining page boundaries.
///
/// PDF defines several page boxes that control different aspects of rendering:
/// - **MediaBox**: Physical medium size (paper)
/// - **CropBox**: Region to display/print (default: MediaBox)
/// - **BleedBox**: Region for production print bleed (default: CropBox)
/// - **TrimBox**: Finished page dimensions (default: CropBox)
/// - **ArtBox**: Meaningful content boundaries (default: CropBox)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PdfPageBox {
    /// Left boundary (in points)
    pub left: f32,
    /// Bottom boundary (in points)
    pub bottom: f32,
    /// Right boundary (in points)
    pub right: f32,
    /// Top boundary (in points)
    pub top: f32,
}

impl PdfPageBox {
    /// Create a new page box.
    pub fn new(left: f32, bottom: f32, right: f32, top: f32) -> Self {
        Self {
            left,
            bottom,
            right,
            top,
        }
    }

    /// Get the width of the box.
    pub fn width(&self) -> f32 {
        (self.right - self.left).abs()
    }

    /// Get the height of the box.
    pub fn height(&self) -> f32 {
        (self.top - self.bottom).abs()
    }

    /// Check if the box is valid (has positive area).
    pub fn is_valid(&self) -> bool {
        self.width() > 0.0 && self.height() > 0.0
    }
}

impl Default for PdfPageBox {
    fn default() -> Self {
        Self::new(0.0, 0.0, 612.0, 792.0) // US Letter
    }
}

/// Transformation matrix for page transforms.
///
/// The matrix represents a 2D affine transformation:
/// ```text
/// | a  b  0 |
/// | c  d  0 |
/// | e  f  1 |
/// ```
///
/// Transformed point: (x', y') = (a*x + c*y + e, b*x + d*y + f)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PdfMatrix {
    /// Scale/rotate coefficient
    pub a: f32,
    /// Rotate/shear coefficient
    pub b: f32,
    /// Rotate/shear coefficient
    pub c: f32,
    /// Scale/rotate coefficient
    pub d: f32,
    /// Horizontal translation
    pub e: f32,
    /// Vertical translation
    pub f: f32,
}

impl PdfMatrix {
    /// Create a new transformation matrix.
    pub fn new(a: f32, b: f32, c: f32, d: f32, e: f32, f: f32) -> Self {
        Self { a, b, c, d, e, f }
    }

    /// Create an identity matrix (no transformation).
    pub fn identity() -> Self {
        Self::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0)
    }

    /// Create a translation matrix.
    pub fn translation(x: f32, y: f32) -> Self {
        Self::new(1.0, 0.0, 0.0, 1.0, x, y)
    }

    /// Create a uniform scale matrix.
    pub fn scale(factor: f32) -> Self {
        Self::scale_xy(factor, factor)
    }

    /// Create a non-uniform scale matrix.
    pub fn scale_xy(x: f32, y: f32) -> Self {
        Self::new(x, 0.0, 0.0, y, 0.0, 0.0)
    }

    /// Create a rotation matrix (angle in radians).
    pub fn rotation(radians: f32) -> Self {
        let cos = radians.cos();
        let sin = radians.sin();
        Self::new(cos, sin, -sin, cos, 0.0, 0.0)
    }

    /// Create a rotation matrix from degrees.
    pub fn rotation_degrees(degrees: f32) -> Self {
        Self::rotation(degrees.to_radians())
    }
}

impl Default for PdfMatrix {
    fn default() -> Self {
        Self::identity()
    }
}

/// Clipping rectangle for page transforms.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PdfClipRect {
    /// Left boundary
    pub left: f32,
    /// Top boundary
    pub top: f32,
    /// Right boundary
    pub right: f32,
    /// Bottom boundary
    pub bottom: f32,
}

impl PdfClipRect {
    /// Create a new clipping rectangle.
    pub fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    /// Create from a page box.
    pub fn from_page_box(page_box: &PdfPageBox) -> Self {
        Self::new(page_box.left, page_box.top, page_box.right, page_box.bottom)
    }
}

/// Page rotation in 90-degree increments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PdfPageRotation {
    /// No rotation (0 degrees)
    #[default]
    None = 0,
    /// 90 degrees clockwise
    Clockwise90 = 1,
    /// 180 degrees
    Rotated180 = 2,
    /// 270 degrees clockwise (90 degrees counter-clockwise)
    Clockwise270 = 3,
}

impl PdfPageRotation {
    /// Create rotation from raw PDFium value (0-3).
    pub fn from_raw(value: i32) -> Self {
        match value {
            0 => Self::None,
            1 => Self::Clockwise90,
            2 => Self::Rotated180,
            3 => Self::Clockwise270,
            _ => Self::None,
        }
    }

    /// Get the raw PDFium rotation value.
    pub fn as_raw(&self) -> i32 {
        *self as i32
    }

    /// Get the rotation in degrees (0, 90, 180, or 270).
    pub fn as_degrees(&self) -> u16 {
        match self {
            Self::None => 0,
            Self::Clockwise90 => 90,
            Self::Rotated180 => 180,
            Self::Clockwise270 => 270,
        }
    }
}

/// A page in a PDF document.
///
/// Provides access to page dimensions, text content, and rendering.
pub struct PdfPage {
    handle: FPDF_PAGE,
    doc_inner: Arc<PdfDocumentInner>,
    index: usize,
}

// SAFETY: Page handles are safe to send between threads
unsafe impl Send for PdfPage {}

impl PdfPage {
    pub(crate) fn new(handle: FPDF_PAGE, doc_inner: Arc<PdfDocumentInner>, index: usize) -> Self {
        Self {
            handle,
            doc_inner,
            index,
        }
    }

    /// Get the page index (0-based).
    pub fn index(&self) -> usize {
        self.index
    }

    /// Get the raw page handle.
    pub fn handle(&self) -> FPDF_PAGE {
        self.handle
    }

    /// Get the page width in points (1/72 inch).
    pub fn width(&self) -> f64 {
        unsafe { FPDF_GetPageWidth(self.handle) }
    }

    /// Get the page height in points (1/72 inch).
    pub fn height(&self) -> f64 {
        unsafe { FPDF_GetPageHeight(self.handle) }
    }

    /// Get the page size as (width, height) in points.
    pub fn size(&self) -> (f64, f64) {
        (self.width(), self.height())
    }

    /// Get the page size in pixels at a given DPI.
    pub fn size_at_dpi(&self, dpi: f64) -> (u32, u32) {
        let scale = dpi / 72.0;
        let width = (self.width() * scale).round() as u32;
        let height = (self.height() * scale).round() as u32;
        (width, height)
    }

    /// Get the page rotation.
    ///
    /// # Returns
    ///
    /// The rotation of this page as a `PdfPageRotation` enum.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::{Pdfium, PdfPageRotation};
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// match page.rotation() {
    ///     PdfPageRotation::None => println!("No rotation"),
    ///     PdfPageRotation::Clockwise90 => println!("Rotated 90°"),
    ///     PdfPageRotation::Rotated180 => println!("Rotated 180°"),
    ///     PdfPageRotation::Clockwise270 => println!("Rotated 270°"),
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn rotation(&self) -> PdfPageRotation {
        let raw = unsafe { FPDFPage_GetRotation(self.handle) };
        PdfPageRotation::from_raw(raw)
    }

    /// Get the rotation in degrees (0, 90, 180, or 270).
    pub fn rotation_degrees(&self) -> u16 {
        self.rotation().as_degrees()
    }

    /// Set the page rotation.
    ///
    /// # Arguments
    ///
    /// * `rotation` - The rotation to apply to this page
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::{Pdfium, PdfPageRotation};
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let mut page = doc.page(0)?;
    ///
    /// // Rotate page 90 degrees clockwise
    /// page.set_rotation(PdfPageRotation::Clockwise90);
    ///
    /// // Save the rotated document
    /// doc.save_to_file("rotated.pdf", None)?;
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn set_rotation(&mut self, rotation: PdfPageRotation) {
        unsafe {
            FPDFPage_SetRotation(self.handle, rotation.as_raw());
        }
    }

    /// Set the page rotation from degrees.
    ///
    /// # Arguments
    ///
    /// * `degrees` - The rotation angle (must be 0, 90, 180, or 270)
    ///
    /// # Returns
    ///
    /// `Ok(())` if the rotation was valid and applied,
    /// `Err` if the degrees value is not a valid rotation (0, 90, 180, or 270).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let mut page = doc.page(0)?;
    ///
    /// // Rotate page 180 degrees
    /// page.set_rotation_degrees(180)?;
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn set_rotation_degrees(&mut self, degrees: u16) -> Result<()> {
        let rotation = match degrees {
            0 => PdfPageRotation::None,
            90 => PdfPageRotation::Clockwise90,
            180 => PdfPageRotation::Rotated180,
            270 => PdfPageRotation::Clockwise270,
            _ => {
                return Err(PdfError::InvalidInput {
                    message: format!(
                        "Invalid rotation degrees: {}. Must be 0, 90, 180, or 270.",
                        degrees
                    ),
                })
            }
        };
        self.set_rotation(rotation);
        Ok(())
    }

    /// Get the text content of this page.
    ///
    /// # Returns
    ///
    /// A `PdfPageText` object for accessing text content, or an error if
    /// text extraction fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// let text = page.text()?;
    /// println!("Text: {}", text.all());
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn text(&self) -> Result<PdfPageText> {
        let text_page = unsafe { FPDFText_LoadPage(self.handle) };
        if text_page.is_null() {
            return Err(PdfError::TextExtractionFailed {
                reason: "Failed to load text page".to_string(),
            });
        }
        Ok(PdfPageText::new(text_page))
    }

    /// Render the page to a bitmap using default settings (300 DPI, BGRA).
    ///
    /// # Returns
    ///
    /// A `PdfBitmap` containing the rendered page.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// let bitmap = page.render()?;
    /// bitmap.save_as_png("page.png")?;
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn render(&self) -> Result<PdfBitmap> {
        self.render_with_config(&PdfRenderConfig::default())
    }

    /// Render the page to a bitmap with custom settings.
    ///
    /// # Arguments
    ///
    /// * `config` - Render configuration (DPI, pixel format, etc.)
    ///
    /// # Returns
    ///
    /// A `PdfBitmap` containing the rendered page.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::{Pdfium, PdfRenderConfig, PixelFormat};
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// let config = PdfRenderConfig::new()
    ///     .set_target_dpi(150.0)
    ///     .set_pixel_format(PixelFormat::Bgr);
    ///
    /// let bitmap = page.render_with_config(&config)?;
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn render_with_config(&self, config: &PdfRenderConfig) -> Result<PdfBitmap> {
        let (width, height) = self.size_at_dpi(config.dpi());
        let format = config.pixel_format();

        // Create bitmap
        let fpdf_format = format.to_fpdf_format();
        let bitmap = unsafe { FPDFBitmap_Create(width as i32, height as i32, fpdf_format) };

        if bitmap.is_null() {
            return Err(PdfError::BitmapCreationFailed {
                reason: "FPDFBitmap_Create returned null".to_string(),
            });
        }

        // Fill with white background
        unsafe {
            FPDFBitmap_FillRect(bitmap, 0, 0, width as i32, height as i32, 0xFFFFFFFF);
        }

        // Render the page
        let flags = FPDF_ANNOT as i32 | FPDF_PRINTING as i32;
        unsafe {
            FPDF_RenderPageBitmap(
                bitmap,
                self.handle,
                0,
                0,
                width as i32,
                height as i32,
                0, // rotation
                flags,
            );

            // Render form elements if form handle exists
            let form_handle = self.doc_inner.form_handle;
            if !form_handle.is_null() {
                FPDF_FFLDraw(
                    form_handle,
                    bitmap,
                    self.handle,
                    0,
                    0,
                    width as i32,
                    height as i32,
                    0,
                    flags,
                );
            }
        }

        Ok(PdfBitmap::new(bitmap, width, height, format))
    }

    /// Check if this page appears to be a scanned image (single full-page JPEG).
    ///
    /// Scanned pages can use the JPEG fast path for 545x speedup.
    pub fn is_scanned(&self) -> bool {
        unsafe {
            let obj_count = FPDFPage_CountObjects(self.handle);
            if obj_count != 1 {
                return false;
            }

            let obj = FPDFPage_GetObject(self.handle, 0);
            if obj.is_null() {
                return false;
            }

            let obj_type = FPDFPageObj_GetType(obj);
            if obj_type != FPDF_PAGEOBJ_IMAGE as i32 {
                return false;
            }

            // Check if the image is JPEG
            let image_bitmap = FPDFImageObj_GetBitmap(obj);
            if image_bitmap.is_null() {
                return false;
            }

            // Get image dimensions and compare to page size
            let img_width = FPDFBitmap_GetWidth(image_bitmap);
            let img_height = FPDFBitmap_GetHeight(image_bitmap);
            FPDFBitmap_Destroy(image_bitmap);

            // Check if image covers most of the page (>90%)
            let page_width = self.width() as i32;
            let page_height = self.height() as i32;

            img_width >= (page_width * 9 / 10) && img_height >= (page_height * 9 / 10)
        }
    }

    /// Extract JPEG data if this page is a scanned image.
    ///
    /// Returns the raw JPEG bytes if the page contains a single full-page JPEG image.
    /// Returns None if the page is not a simple scanned image.
    ///
    /// This method provides 545x speedup for scanned PDFs by bypassing rendering.
    pub fn extract_jpeg_if_scanned(&self) -> Option<Vec<u8>> {
        if !self.is_scanned() {
            return None;
        }

        unsafe {
            let obj = FPDFPage_GetObject(self.handle, 0);
            if obj.is_null() {
                return None;
            }

            // Get raw image data
            let filter_count = FPDFImageObj_GetImageFilterCount(obj);

            // Check if this is a DCT (JPEG) encoded image
            if filter_count > 0 {
                let mut filter_buf = vec![0u8; 32];
                let filter_len = FPDFImageObj_GetImageFilter(
                    obj,
                    0,
                    filter_buf.as_mut_ptr() as *mut _,
                    filter_buf.len() as u64,
                );

                if filter_len > 0 {
                    let filter_name =
                        String::from_utf8_lossy(&filter_buf[..filter_len as usize - 1]);
                    if filter_name == "DCTDecode" {
                        // Get raw JPEG data
                        let data_len = FPDFImageObj_GetImageDataRaw(obj, std::ptr::null_mut(), 0);
                        if data_len > 0 {
                            let mut data = vec![0u8; data_len as usize];
                            FPDFImageObj_GetImageDataRaw(
                                obj,
                                data.as_mut_ptr() as *mut _,
                                data_len,
                            );
                            return Some(data);
                        }
                    }
                }
            }

            // Fallback: get decoded data and check if it's small enough to be JPEG
            let decoded_len = FPDFImageObj_GetImageDataDecoded(obj, std::ptr::null_mut(), 0);
            if decoded_len > 0 && decoded_len < 50_000_000 {
                let mut data = vec![0u8; decoded_len as usize];
                FPDFImageObj_GetImageDataDecoded(obj, data.as_mut_ptr() as *mut _, decoded_len);

                // Check for JPEG magic bytes
                if data.len() >= 2 && data[0] == 0xFF && data[1] == 0xD8 {
                    return Some(data);
                }
            }
        }

        None
    }

    /// Check if this page is a JPEG-encoded scanned image.
    ///
    /// This is more specific than `is_scanned()` - it checks if the image
    /// is specifically JPEG encoded, which allows for faster extraction.
    pub fn is_jpeg_scanned(&self) -> bool {
        if !self.is_scanned() {
            return false;
        }

        unsafe {
            let obj = FPDFPage_GetObject(self.handle, 0);
            if obj.is_null() {
                return false;
            }

            let filter_count = FPDFImageObj_GetImageFilterCount(obj);
            if filter_count > 0 {
                let mut filter_buf = vec![0u8; 32];
                let filter_len = FPDFImageObj_GetImageFilter(
                    obj,
                    0,
                    filter_buf.as_mut_ptr() as *mut _,
                    filter_buf.len() as u64,
                );

                if filter_len > 0 {
                    let filter_name =
                        String::from_utf8_lossy(&filter_buf[..filter_len as usize - 1]);
                    return filter_name == "DCTDecode";
                }
            }
        }

        false
    }

    /// Smart render that automatically chooses the fastest path.
    ///
    /// For scanned pages with JPEG content, extracts the raw JPEG (545x faster).
    /// For normal pages, renders with the given configuration.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::{Pdfium, PdfRenderConfig, ScannedPageContent};
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// let config = PdfRenderConfig::new().set_target_dpi(300.0);
    /// let content = page.smart_render(&config)?;
    ///
    /// if content.is_jpeg() {
    ///     println!("Extracted JPEG directly (fast path)");
    /// } else {
    ///     println!("Rendered bitmap (normal path)");
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn smart_render(&self, config: &PdfRenderConfig) -> Result<ScannedPageContent> {
        // Try JPEG fast path first
        if let Some(jpeg_data) = self.extract_jpeg_if_scanned() {
            return Ok(ScannedPageContent::Jpeg(jpeg_data));
        }

        // Fall back to normal rendering
        let bitmap = self.render_with_config(config)?;
        Ok(ScannedPageContent::Rendered(bitmap))
    }

    /// Get annotations on this page.
    ///
    /// Returns an iterator over all annotations (highlights, notes, links, etc.).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::{Pdfium, PdfAnnotationType};
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// for annot in page.annotations() {
    ///     println!("Annotation type: {:?}", annot.annotation_type());
    ///     if let Ok(rect) = annot.rect() {
    ///         println!("  at ({:.1}, {:.1})", rect.left, rect.top);
    ///     }
    ///     if let Some(contents) = annot.contents() {
    ///         println!("  contents: {}", contents);
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn annotations(&self) -> PdfPageAnnotations {
        PdfPageAnnotations::new(self.handle)
    }

    /// Get the number of annotations on this page.
    pub fn annotation_count(&self) -> i32 {
        unsafe { FPDFPage_GetAnnotCount(self.handle) }
    }

    /// Get form fields on this page.
    ///
    /// Returns an iterator over form fields (text boxes, checkboxes, etc.).
    /// Only returns fields if the document has an AcroForm.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::{Pdfium, PdfFormFieldType};
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("form.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// for field in page.form_fields() {
    ///     println!("Field: {} ({:?})", field.name, field.field_type);
    ///     if field.field_type == PdfFormFieldType::TextField {
    ///         println!("  Value: {}", field.value);
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn form_fields(&self) -> PdfPageFormFields<'_> {
        PdfPageFormFields::new(self.doc_inner.form_handle, self.handle)
    }

    /// Check if this page has any form fields.
    pub fn has_form_fields(&self) -> bool {
        self.form_fields().next().is_some()
    }

    /// Get a mutable form field editor by annotation index.
    ///
    /// This allows you to modify form field values (text, checkbox state, selections).
    ///
    /// # Arguments
    ///
    /// * `annot_index` - The annotation index (0-based)
    ///
    /// # Returns
    ///
    /// `Some(editor)` if a form field exists at that index,
    /// `None` if the annotation index is out of bounds or not a form field.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("form.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// // Edit the first form field
    /// if let Some(mut editor) = page.form_field_editor(0) {
    ///     let _ = editor.set_text_value("New value");
    /// }
    ///
    /// doc.save_to_file("modified.pdf", None)?;
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn form_field_editor(&self, annot_index: i32) -> Option<crate::form::PdfFormFieldEditor> {
        if annot_index < 0 || annot_index >= self.annotation_count() {
            return None;
        }
        crate::form::PdfFormFieldEditor::new(self.doc_inner.form_handle, self.handle, annot_index)
    }

    /// Get an iterator over mutable form field editors.
    ///
    /// This allows batch modification of form fields.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::{Pdfium, PdfFormFieldType};
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("form.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// // Clear all text fields
    /// for mut editor in page.form_field_editors() {
    ///     if editor.field_type().is_text() {
    ///         let _ = editor.set_text_value("");
    ///     }
    /// }
    ///
    /// doc.save_to_file("cleared.pdf", None)?;
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn form_field_editors(&self) -> crate::form::PdfPageFormFieldEditors {
        crate::form::PdfPageFormFieldEditors::new(self.doc_inner.form_handle, self.handle)
    }

    /// Get all page objects (images, text, paths, etc.).
    ///
    /// Page objects are the graphical elements that make up a PDF page.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::{Pdfium, PdfPageObjectType};
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// let objects = page.objects();
    /// println!("Page has {} objects", objects.count());
    /// println!("  Images: {}", objects.count_of_type(PdfPageObjectType::Image));
    /// println!("  Text: {}", objects.count_of_type(PdfPageObjectType::Text));
    /// println!("  Paths: {}", objects.count_of_type(PdfPageObjectType::Path));
    ///
    /// // Get all images
    /// for img in objects.images() {
    ///     if let Some((w, h)) = img.image_size() {
    ///         println!("Image {}x{} at {:?}", w, h, img.bounds());
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn objects(&self) -> PdfPageObjects {
        PdfPageObjects::new(self.handle, self.doc_inner.handle)
    }

    /// Get the number of page objects.
    pub fn object_count(&self) -> usize {
        self.objects().count()
    }

    /// Get only content objects (non-artifacts) from the page.
    ///
    /// This filters out artifacts (headers, footers, watermarks, etc.)
    /// to return only the document's actual content objects.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// // Get only content objects (excludes headers, footers, etc.)
    /// for obj in page.content_objects() {
    ///     println!("Content object at {:?}", obj.bounds());
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn content_objects(&self) -> Vec<PdfPageObject> {
        self.objects()
            .iter()
            .filter(|obj| !obj.is_artifact())
            .collect()
    }

    /// Get only artifact objects from the page.
    ///
    /// Artifacts are decorative elements not part of the document's logical
    /// content, such as headers, footers, page numbers, and watermarks.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// // Get only artifact objects
    /// for artifact in page.artifact_objects() {
    ///     if let Some(artifact_type) = artifact.artifact_type() {
    ///         println!("Found {:?} artifact", artifact_type);
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn artifact_objects(&self) -> Vec<PdfPageObject> {
        self.objects()
            .iter()
            .filter(|obj| obj.is_artifact())
            .collect()
    }

    /// Count the number of artifact objects on this page.
    pub fn artifact_count(&self) -> usize {
        self.objects()
            .iter()
            .filter(|obj| obj.is_artifact())
            .count()
    }

    /// Count the number of content (non-artifact) objects on this page.
    pub fn content_object_count(&self) -> usize {
        self.objects()
            .iter()
            .filter(|obj| !obj.is_artifact())
            .count()
    }

    // ========================================================================
    // Line/Separator Extraction
    // ========================================================================

    /// Extract all lines from the page.
    ///
    /// Scans all path objects on the page and extracts line segments.
    /// This includes simple lines (MoveTo + LineTo) as well as line segments
    /// from more complex paths like polylines and rectangles.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// let lines = page.extract_lines();
    /// println!("Found {} lines on page", lines.len());
    ///
    /// for line in &lines {
    ///     if line.is_horizontal {
    ///         println!("Horizontal separator at y={:.1}", line.start.1);
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn extract_lines(&self) -> Vec<crate::page_object::ExtractedLine> {
        let mut lines = Vec::new();

        for obj in self.objects().paths() {
            if let Some(path_lines) = obj.extract_lines() {
                lines.extend(path_lines);
            }
        }

        lines
    }

    /// Extract only horizontal lines from the page.
    ///
    /// Returns lines where the horizontal extent is significantly greater
    /// than the vertical extent (ratio > 5:1 or vertical delta < 1pt).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// // Find horizontal separators
    /// for line in page.extract_horizontal_lines() {
    ///     println!("Horizontal line at y={:.1}, length={:.1}",
    ///         line.y_position().unwrap_or(0.0), line.length());
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn extract_horizontal_lines(&self) -> Vec<crate::page_object::ExtractedLine> {
        self.extract_lines()
            .into_iter()
            .filter(|line| line.is_horizontal)
            .collect()
    }

    /// Extract only vertical lines from the page.
    ///
    /// Returns lines where the vertical extent is significantly greater
    /// than the horizontal extent (ratio > 5:1 or horizontal delta < 1pt).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// // Find vertical borders/column separators
    /// for line in page.extract_vertical_lines() {
    ///     println!("Vertical line at x={:.1}, length={:.1}",
    ///         line.x_position().unwrap_or(0.0), line.length());
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn extract_vertical_lines(&self) -> Vec<crate::page_object::ExtractedLine> {
        self.extract_lines()
            .into_iter()
            .filter(|line| line.is_vertical)
            .collect()
    }

    /// Get the count of line segments on this page.
    pub fn line_count(&self) -> usize {
        self.extract_lines().len()
    }

    // ========================================================================
    // Colored Region Extraction
    // ========================================================================

    /// Extract all colored (filled) regions from the page.
    ///
    /// Scans all path objects on the page and extracts filled shapes.
    /// The `is_behind_text` field is determined by z-order: regions that
    /// appear before any text object are considered "behind text".
    ///
    /// Colored regions are useful for detecting:
    /// - Page backgrounds
    /// - Table cell backgrounds
    /// - Highlighted text areas
    /// - Decorative boxes
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// let regions = page.extract_colored_regions();
    /// println!("Found {} colored regions", regions.len());
    ///
    /// for region in &regions {
    ///     if region.is_behind_text {
    ///         println!("Background region at {:?}", region.bounds);
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn extract_colored_regions(&self) -> Vec<crate::page_object::ColoredRegion> {
        use crate::page_object::PdfPageObjectType;

        let objects = self.objects();
        let mut regions = Vec::new();

        // Find the index of the first text object to determine z-order
        let first_text_index = objects.iter().position(|obj| obj.is_text());

        for obj in objects.iter() {
            if obj.object_type() == PdfPageObjectType::Path {
                // Determine if this path is behind text (appears before first text in z-order)
                let is_behind_text = match first_text_index {
                    Some(text_idx) => obj.index() < text_idx,
                    None => true, // No text on page, all paths are "behind" (vacuously true)
                };

                if let Some(region) = obj.extract_colored_region(is_behind_text) {
                    regions.push(region);
                }
            }
        }

        regions
    }

    /// Extract colored regions that appear behind text.
    ///
    /// Returns only regions where `is_behind_text` is true, which typically
    /// means they are background fills or table cell backgrounds.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// for region in page.extract_background_regions() {
    ///     if let Some(fill) = region.fill_color {
    ///         println!("Background color: RGBA({}, {}, {}, {})", fill.0, fill.1, fill.2, fill.3);
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn extract_background_regions(&self) -> Vec<crate::page_object::ColoredRegion> {
        self.extract_colored_regions()
            .into_iter()
            .filter(|r| r.is_behind_text)
            .collect()
    }

    /// Extract colored regions that appear in front of text.
    ///
    /// Returns only regions where `is_behind_text` is false, which typically
    /// means they are overlays, highlights, or annotations.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// for region in page.extract_foreground_regions() {
    ///     println!("Foreground region at {:?}", region.bounds);
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn extract_foreground_regions(&self) -> Vec<crate::page_object::ColoredRegion> {
        self.extract_colored_regions()
            .into_iter()
            .filter(|r| !r.is_behind_text)
            .collect()
    }

    /// Get the count of colored regions on this page.
    pub fn colored_region_count(&self) -> usize {
        self.extract_colored_regions().len()
    }

    /// Check if the page has a colored background (full-page filled region).
    ///
    /// Returns true if there's a filled region that covers most of the page
    /// (>90% area) and appears behind text.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// if page.has_page_background() {
    ///     println!("Page has a colored background");
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn has_page_background(&self) -> bool {
        let page_area = self.width() * self.height();
        if page_area <= 0.0 {
            return false;
        }

        self.extract_background_regions().iter().any(|r| {
            let region_area = r.area() as f64;
            region_area > page_area * 0.9
        })
    }

    /// Get the dominant background color if the page has a colored background.
    ///
    /// Returns the fill color of the largest background region that covers
    /// most of the page, or None if no such region exists.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// if let Some((r, g, b, a)) = page.page_background_color() {
    ///     println!("Page background: RGBA({}, {}, {}, {})", r, g, b, a);
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn page_background_color(&self) -> Option<(u8, u8, u8, u8)> {
        let page_area = self.width() * self.height();
        if page_area <= 0.0 {
            return None;
        }

        // Find the largest background region
        self.extract_background_regions()
            .into_iter()
            .filter(|r| r.area() as f64 > page_area * 0.5) // At least 50% of page
            .max_by(|a, b| {
                a.area()
                    .partial_cmp(&b.area())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .and_then(|r| r.fill_color)
    }

    // ========================================================================
    // Text Block Metrics Extraction
    // ========================================================================

    /// Extract text blocks with detailed spacing metrics.
    ///
    /// Groups characters into lines and lines into blocks, computing metrics
    /// such as line heights, spacing, and indentation. This is useful for
    /// detecting paragraph structure, column layouts, and text formatting.
    ///
    /// The algorithm:
    /// 1. Groups characters into lines by vertical position (within tolerance)
    /// 2. Groups lines into blocks by detecting large vertical gaps
    /// 3. Computes per-block metrics for layout analysis
    ///
    /// # Returns
    ///
    /// A vector of `TextBlockMetrics` for each detected text block on the page.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// let blocks = page.extract_text_blocks_with_metrics();
    /// println!("Found {} text blocks", blocks.len());
    ///
    /// for (i, block) in blocks.iter().enumerate() {
    ///     println!("Block {}: {} lines, avg line height: {:.1}pt",
    ///         i, block.line_count, block.avg_line_height);
    ///     if block.first_line_indent > 20.0 {
    ///         println!("  -> Paragraph (indented first line)");
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn extract_text_blocks_with_metrics(&self) -> Vec<TextBlockMetrics> {
        // Get text page
        let text_page = match self.text() {
            Ok(t) => t,
            Err(_) => return Vec::new(),
        };

        // Collect all characters with their positions
        let chars: Vec<_> = text_page.chars().collect();
        if chars.is_empty() {
            return Vec::new();
        }

        // Group characters into lines by vertical position
        // Characters on the same line have similar y-coordinates
        let line_tolerance = 3.0; // Points tolerance for same line
        let mut lines: Vec<Vec<&crate::text::PdfChar>> = Vec::new();

        for ch in &chars {
            // Skip whitespace characters for line grouping
            if ch.unicode.is_whitespace() {
                continue;
            }

            // Find a line this character belongs to
            let mut found_line = false;
            for line in &mut lines {
                if !line.is_empty() {
                    let line_y = line[0].bottom;
                    if (ch.bottom - line_y).abs() < line_tolerance {
                        line.push(ch);
                        found_line = true;
                        break;
                    }
                }
            }

            if !found_line {
                lines.push(vec![ch]);
            }
        }

        if lines.is_empty() {
            return Vec::new();
        }

        // Sort lines by vertical position (top to bottom, so higher y first)
        lines.sort_by(|a, b| {
            let y_a = a.first().map(|c| c.bottom).unwrap_or(0.0);
            let y_b = b.first().map(|c| c.bottom).unwrap_or(0.0);
            y_b.partial_cmp(&y_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Sort characters within each line by horizontal position
        for line in &mut lines {
            line.sort_by(|a, b| {
                a.left
                    .partial_cmp(&b.left)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }

        // Compute line heights and find gap threshold for block detection
        let mut line_heights: Vec<f64> = Vec::new();
        let mut line_gaps: Vec<f64> = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            let height = line
                .iter()
                .map(|c| c.top - c.bottom)
                .fold(0.0f64, |a, b| a.max(b));
            line_heights.push(height);

            if i > 0 {
                let prev_line = &lines[i - 1];
                let prev_bottom = prev_line
                    .iter()
                    .map(|c| c.bottom)
                    .fold(f64::MAX, |a, b| a.min(b));
                let curr_top = line.iter().map(|c| c.top).fold(f64::MIN, |a, b| a.max(b));
                let gap = prev_bottom - curr_top;
                if gap > 0.0 {
                    line_gaps.push(gap);
                }
            }
        }

        // Determine block separation threshold
        // Use 1.5x the average line height as threshold
        let avg_line_height = if line_heights.is_empty() {
            12.0 // Default
        } else {
            line_heights.iter().sum::<f64>() / line_heights.len() as f64
        };
        let block_gap_threshold = avg_line_height * 1.5;

        // Group lines into blocks
        let mut blocks: Vec<Vec<usize>> = Vec::new(); // Each block is a list of line indices
        let mut current_block: Vec<usize> = vec![0];

        for i in 1..lines.len() {
            let prev_line = &lines[i - 1];
            let curr_line = &lines[i];

            let prev_bottom = prev_line
                .iter()
                .map(|c| c.bottom)
                .fold(f64::MAX, |a, b| a.min(b));
            let curr_top = curr_line
                .iter()
                .map(|c| c.top)
                .fold(f64::MIN, |a, b| a.max(b));
            let gap = prev_bottom - curr_top;

            if gap > block_gap_threshold {
                // Start new block
                blocks.push(current_block);
                current_block = vec![i];
            } else {
                current_block.push(i);
            }
        }
        blocks.push(current_block);

        // Compute metrics for each block
        let mut result = Vec::with_capacity(blocks.len());

        for block_lines in &blocks {
            if block_lines.is_empty() {
                continue;
            }

            // Collect all characters in this block
            let mut block_chars: Vec<&crate::text::PdfChar> = Vec::new();
            for &line_idx in block_lines {
                block_chars.extend(lines[line_idx].iter());
            }

            if block_chars.is_empty() {
                continue;
            }

            // Compute bounds
            let left = block_chars
                .iter()
                .map(|c| c.left)
                .fold(f64::MAX, |a, b| a.min(b)) as f32;
            let bottom = block_chars
                .iter()
                .map(|c| c.bottom)
                .fold(f64::MAX, |a, b| a.min(b)) as f32;
            let right = block_chars
                .iter()
                .map(|c| c.right)
                .fold(f64::MIN, |a, b| a.max(b)) as f32;
            let top = block_chars
                .iter()
                .map(|c| c.top)
                .fold(f64::MIN, |a, b| a.max(b)) as f32;

            // Line count
            let line_count = block_lines.len();

            // Average line height
            let block_line_heights: Vec<f64> = block_lines
                .iter()
                .map(|&idx| {
                    lines[idx]
                        .iter()
                        .map(|c| c.top - c.bottom)
                        .fold(0.0f64, |a, b| a.max(b))
                })
                .collect();
            let avg_line_height = if block_line_heights.is_empty() {
                0.0
            } else {
                (block_line_heights.iter().sum::<f64>() / block_line_heights.len() as f64) as f32
            };

            // Average line spacing
            let mut block_line_spacings: Vec<f64> = Vec::new();
            for i in 1..block_lines.len() {
                let prev_idx = block_lines[i - 1];
                let curr_idx = block_lines[i];
                let prev_line = &lines[prev_idx];
                let curr_line = &lines[curr_idx];

                let prev_bottom = prev_line
                    .iter()
                    .map(|c| c.bottom)
                    .fold(f64::MAX, |a, b| a.min(b));
                let curr_top = curr_line
                    .iter()
                    .map(|c| c.top)
                    .fold(f64::MIN, |a, b| a.max(b));
                let spacing = prev_bottom - curr_top;
                if spacing > 0.0 {
                    block_line_spacings.push(spacing);
                }
            }
            let avg_line_spacing = if block_line_spacings.is_empty() {
                0.0
            } else {
                (block_line_spacings.iter().sum::<f64>() / block_line_spacings.len() as f64) as f32
            };

            // First line indent
            let first_line_idx = block_lines[0];
            let first_line = &lines[first_line_idx];
            let first_line_left = first_line
                .iter()
                .map(|c| c.left)
                .fold(f64::MAX, |a, b| a.min(b)) as f32;
            let first_line_indent = (first_line_left - left).max(0.0);

            // Average character spacing (gap between adjacent characters on same line)
            let mut char_spacings: Vec<f64> = Vec::new();
            for &line_idx in block_lines {
                let line = &lines[line_idx];
                for i in 1..line.len() {
                    let gap = line[i].left - line[i - 1].right;
                    if gap > 0.0 && gap < 20.0 {
                        // Filter out unreasonably large gaps
                        char_spacings.push(gap);
                    }
                }
            }
            let avg_char_spacing = if char_spacings.is_empty() {
                0.0
            } else {
                (char_spacings.iter().sum::<f64>() / char_spacings.len() as f64) as f32
            };

            // Average word spacing (gaps larger than typical char spacing)
            // Use 3x average char spacing as threshold for word boundary
            let word_threshold = if avg_char_spacing > 0.0 {
                (avg_char_spacing * 3.0) as f64
            } else {
                5.0 // Default threshold
            };
            let word_spacings: Vec<f64> = char_spacings
                .iter()
                .filter(|&&gap| gap > word_threshold)
                .copied()
                .collect();
            let avg_word_spacing = if word_spacings.is_empty() {
                0.0
            } else {
                (word_spacings.iter().sum::<f64>() / word_spacings.len() as f64) as f32
            };

            result.push(TextBlockMetrics {
                bounds: (left, bottom, right, top),
                line_count,
                avg_line_height,
                avg_line_spacing,
                first_line_indent,
                avg_char_spacing,
                avg_word_spacing,
            });
        }

        result
    }

    /// Get the count of text blocks on this page.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// println!("Page has {} text blocks", page.text_block_count());
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn text_block_count(&self) -> usize {
        self.extract_text_blocks_with_metrics().len()
    }

    // ========================================================================
    // Text Decoration Detection
    // ========================================================================

    /// Extract text decorations (underlines, strikethroughs, overlines) from the page.
    ///
    /// Text decorations in PDFs are typically implemented as path objects (horizontal
    /// lines) rather than text properties. This method detects horizontal lines that
    /// appear near text and classifies them based on their vertical position:
    /// - **Underline**: Line below the text baseline
    /// - **Strikethrough**: Line through the middle of the text
    /// - **Overline**: Line above the text top
    ///
    /// # Returns
    ///
    /// A vector of `TextDecoration` containing the decoration type, bounds,
    /// thickness, and color.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// let decorations = page.extract_text_decorations();
    /// for decoration in &decorations {
    ///     println!("{:?}: {:.1}pt wide, color: {:?}",
    ///         decoration.decoration_type,
    ///         decoration.width(),
    ///         decoration.color);
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn extract_text_decorations(&self) -> Vec<TextDecoration> {
        // Get horizontal lines from the page
        let lines = self.extract_horizontal_lines();
        if lines.is_empty() {
            return Vec::new();
        }

        // Get text characters to determine line positions relative to text
        let text = match self.text() {
            Ok(t) => t,
            Err(_) => return Vec::new(),
        };

        let chars: Vec<_> = text.chars().collect();
        if chars.is_empty() {
            // No text to compare against - return lines as potential decorations
            return lines
                .iter()
                .map(|line| TextDecoration {
                    decoration_type: TextDecorationType::Underline, // Default
                    bounds: line.bounds(),
                    thickness: line.thickness,
                    color: line.color,
                })
                .collect();
        }

        let mut decorations = Vec::new();

        for line in &lines {
            let line_bounds = line.bounds();

            // Skip lines that are too long (likely page dividers, not text decorations)
            let line_width = line_bounds.2 - line_bounds.0;
            if line_width > self.width() as f32 * 0.8 {
                continue;
            }

            // Find characters that overlap horizontally with this line
            let line_y = (line_bounds.1 + line_bounds.3) / 2.0;
            let overlapping_chars: Vec<_> = chars
                .iter()
                .filter(|ch| (ch.right as f32) > line_bounds.0 && (ch.left as f32) < line_bounds.2)
                .collect();

            if overlapping_chars.is_empty() {
                continue; // No text overlap, skip this line
            }

            // Compute text metrics for overlapping characters
            let avg_bottom = overlapping_chars
                .iter()
                .map(|ch| ch.bottom as f32)
                .sum::<f32>()
                / overlapping_chars.len() as f32;
            let avg_top = overlapping_chars
                .iter()
                .map(|ch| ch.top as f32)
                .sum::<f32>()
                / overlapping_chars.len() as f32;
            let avg_height = avg_top - avg_bottom;
            let mid_y = (avg_bottom + avg_top) / 2.0;

            // Classify the decoration based on its y-position relative to text
            let tolerance = avg_height * 0.15; // 15% of text height
            let decoration_type = if line_y < avg_bottom - tolerance {
                // Line is below the baseline
                TextDecorationType::Underline
            } else if (line_y - mid_y).abs() < tolerance {
                // Line is near the middle of the text
                TextDecorationType::Strikethrough
            } else if line_y > avg_top + tolerance {
                // Line is above the text
                TextDecorationType::Overline
            } else if line_y < mid_y {
                // Between bottom and middle - likely underline
                TextDecorationType::Underline
            } else {
                // Between middle and top - likely overline or strikethrough
                TextDecorationType::Strikethrough
            };

            decorations.push(TextDecoration {
                decoration_type,
                bounds: line_bounds,
                thickness: line.thickness,
                color: line.color,
            });
        }

        decorations
    }

    /// Get the count of text decorations on this page.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// println!("Page has {} text decorations", page.text_decoration_count());
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn text_decoration_count(&self) -> usize {
        self.extract_text_decorations().len()
    }

    /// Check if the page has any text decorations.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// if page.has_text_decorations() {
    ///     println!("Page has underlines, strikethroughs, or overlines");
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn has_text_decorations(&self) -> bool {
        !self.extract_text_decorations().is_empty()
    }

    /// Get underlines from this page.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// for underline in page.underlines() {
    ///     println!("Underline: {:.1}pt wide", underline.width());
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn underlines(&self) -> Vec<TextDecoration> {
        self.extract_text_decorations()
            .into_iter()
            .filter(|d| d.decoration_type.is_underline())
            .collect()
    }

    /// Get strikethroughs from this page.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// for strikethrough in page.strikethroughs() {
    ///     println!("Strikethrough: {:.1}pt wide", strikethrough.width());
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn strikethroughs(&self) -> Vec<TextDecoration> {
        self.extract_text_decorations()
            .into_iter()
            .filter(|d| d.decoration_type.is_strikethrough())
            .collect()
    }

    /// Get overlines from this page.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// for overline in page.overlines() {
    ///     println!("Overline: {:.1}pt wide", overline.width());
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn overlines(&self) -> Vec<TextDecoration> {
        self.extract_text_decorations()
            .into_iter()
            .filter(|d| d.decoration_type.is_overline())
            .collect()
    }

    // ========================================================================
    // Invisible Text Layer Detection
    // ========================================================================

    /// Check if this page has an invisible text layer.
    ///
    /// Invisible text is commonly used in scanned PDFs where an OCR layer
    /// is placed over the scanned image to enable text selection and search.
    /// The text is rendered with render mode 3 (invisible).
    ///
    /// This method checks all text objects on the page and returns true
    /// if any are rendered with invisible mode.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("scanned.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// if page.has_invisible_text_layer() {
    ///     println!("This page has an OCR text layer");
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn has_invisible_text_layer(&self) -> bool {
        self.objects()
            .text_objects()
            .iter()
            .any(|t| t.is_invisible_text())
    }

    /// Extract the invisible text layer from this page.
    ///
    /// Returns the combined text from all invisible text objects on this page.
    /// This is useful for extracting the OCR layer from scanned PDFs.
    ///
    /// Returns `None` if there is no invisible text on this page.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("scanned.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// if let Some(ocr_text) = page.extract_invisible_text() {
    ///     println!("OCR text: {}", ocr_text);
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn extract_invisible_text(&self) -> Option<String> {
        // Get the text page handle for text extraction
        let text_page = self.text().ok()?;

        let invisible_texts: Vec<String> = self
            .objects()
            .text_objects()
            .into_iter()
            .filter(|t| t.is_invisible_text())
            .filter_map(|t| t.text_content(&text_page))
            .collect();

        if invisible_texts.is_empty() {
            None
        } else {
            Some(invisible_texts.join(""))
        }
    }

    /// Count the number of invisible text objects on this page.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("scanned.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// let count = page.invisible_text_object_count();
    /// println!("Page has {} invisible text objects", count);
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn invisible_text_object_count(&self) -> usize {
        self.objects()
            .text_objects()
            .iter()
            .filter(|t| t.is_invisible_text())
            .count()
    }

    /// Get all invisible text objects from this page.
    ///
    /// Returns a vector of text objects that use invisible rendering mode.
    /// This is useful for analyzing the OCR layer in detail.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("scanned.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// let invisible_objects = page.invisible_text_objects();
    /// println!("Found {} invisible text objects", invisible_objects.len());
    ///
    /// // To get text from objects, use text_content with a text page:
    /// let text_page = page.text()?;
    /// for obj in &invisible_objects {
    ///     if let Some(text) = obj.text_content(&text_page) {
    ///         println!("Invisible text: {}", text);
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn invisible_text_objects(&self) -> Vec<PdfPageObject> {
        self.objects()
            .text_objects()
            .into_iter()
            .filter(|t| t.is_invisible_text())
            .collect()
    }

    /// Get all visible text objects from this page.
    ///
    /// Returns a vector of text objects that are not invisible.
    /// This filters out OCR layers and returns only normally rendered text.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// let visible_objects = page.visible_text_objects();
    /// println!("Found {} visible text objects", visible_objects.len());
    ///
    /// // To get text from objects, use text_content with a text page:
    /// let text_page = page.text()?;
    /// for obj in &visible_objects {
    ///     if let Some(text) = obj.text_content(&text_page) {
    ///         println!("Visible text: {}", text);
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn visible_text_objects(&self) -> Vec<PdfPageObject> {
        self.objects()
            .text_objects()
            .into_iter()
            .filter(|t| !t.is_invisible_text())
            .collect()
    }

    // ========================================================================
    // Mathematical Character Analysis
    // ========================================================================

    /// Analyze mathematical characters on this page.
    ///
    /// This method scans all characters on the page and categorizes them
    /// by Unicode ranges to detect mathematical content. Useful for
    /// identifying pages with equations, formulas, or technical notation.
    ///
    /// # Categories Detected
    ///
    /// - **Math operators**: ∑, ∫, ∂, ∞, ±, ≠, ≤, ≥, etc.
    /// - **Math alphanumerics**: 𝑎, 𝑏, 𝒜, 𝔸, etc. (mathematical fonts)
    /// - **Greek letters**: α, β, γ, δ, θ, π, Σ, Ω, etc.
    /// - **Arrows**: →, ←, ↑, ↓, ⇒, ⟶, etc.
    /// - **Superscripts**: ⁰¹²³⁴⁵⁶⁷⁸⁹ⁿ (Unicode superscripts)
    /// - **Subscripts**: ₀₁₂₃₄₅₆₇₈₉ₐₑₒₓ (Unicode subscripts)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("paper.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// let analysis = page.analyze_math_chars();
    /// println!("Math operators: {}", analysis.math_operators);
    /// println!("Greek letters: {}", analysis.greek_letters);
    /// println!("Math ratio: {:.1}%", analysis.math_ratio() * 100.0);
    ///
    /// if analysis.has_significant_math() {
    ///     println!("This page contains significant mathematical content");
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn analyze_math_chars(&self) -> MathCharAnalysis {
        let mut analysis = MathCharAnalysis::new();

        // Get text from page
        let text = match self.text() {
            Ok(t) => t,
            Err(_) => return analysis,
        };

        // Analyze each character
        for ch in text.chars() {
            analysis.total_chars += 1;

            let c = ch.unicode;

            if is_math_operator(c) {
                analysis.math_operators += 1;
            } else if is_math_alphanumeric(c) {
                analysis.math_alphanumerics += 1;
            } else if is_greek_letter(c) {
                analysis.greek_letters += 1;
            } else if is_arrow(c) {
                analysis.arrows += 1;
            } else if is_unicode_superscript(c) {
                analysis.superscripts += 1;
            } else if is_unicode_subscript(c) {
                analysis.subscripts += 1;
            }
        }

        analysis
    }

    /// Get the count of mathematical characters on this page.
    ///
    /// This is a convenience method that returns the total count of
    /// mathematical characters without full analysis details.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// let count = page.math_char_count();
    /// println!("Found {} mathematical characters", count);
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn math_char_count(&self) -> usize {
        self.analyze_math_chars().math_char_count()
    }

    /// Check if this page has significant mathematical content.
    ///
    /// Returns true if mathematical characters make up more than 5%
    /// of the total characters on the page.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// if page.has_math_content() {
    ///     println!("This page contains mathematical content");
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn has_math_content(&self) -> bool {
        self.analyze_math_chars().has_significant_math()
    }

    // ========================================================================
    // Font Usage Analysis
    // ========================================================================

    /// Extract font usage information from this page.
    ///
    /// This method analyzes all text on the page and returns statistics
    /// about each font used, including whether it's a mathematical or
    /// monospace font.
    ///
    /// # Returns
    ///
    /// A vector of `FontUsageInfo` structs sorted by character count (descending).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// for font in page.extract_font_usage() {
    ///     println!("Font: {} - {} chars ({:.1}%)",
    ///         font.name, font.char_count, font.coverage * 100.0);
    ///     if font.is_math_font {
    ///         println!("  -> Math font!");
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn extract_font_usage(&self) -> Vec<FontUsageInfo> {
        use std::collections::HashMap;

        let text = match self.text() {
            Ok(t) => t,
            Err(_) => return vec![],
        };

        let total_chars = text.chars().count();
        if total_chars == 0 {
            return vec![];
        }

        // Count characters per font
        let mut font_counts: HashMap<String, usize> = HashMap::new();

        for obj in self.objects().text_objects() {
            if let Some(font) = obj.text_font() {
                let font_name = font.base_name().unwrap_or_else(|| "Unknown".to_string());
                // Get approximate char count from bounds
                // This is a rough estimate based on object presence
                *font_counts.entry(font_name).or_insert(0) += 1;
            }
        }

        // If no font info from objects, try to estimate from text
        if font_counts.is_empty() {
            // Return a single "Unknown" font entry
            return vec![FontUsageInfo {
                name: "Unknown".to_string(),
                is_math_font: false,
                is_monospace: false,
                char_count: total_chars,
                coverage: 1.0,
            }];
        }

        // Convert to FontUsageInfo
        let total_objects: usize = font_counts.values().sum();
        let mut results: Vec<FontUsageInfo> = font_counts
            .into_iter()
            .map(|(name, count)| {
                let is_math = is_known_math_font(&name);
                let is_mono = is_known_monospace_font(&name);
                let coverage = count as f32 / total_objects as f32;

                FontUsageInfo {
                    name,
                    is_math_font: is_math,
                    is_monospace: is_mono,
                    char_count: count,
                    coverage,
                }
            })
            .collect();

        // Sort by character count (descending)
        results.sort_by(|a, b| b.char_count.cmp(&a.char_count));

        results
    }

    /// Get the count of distinct fonts used on this page.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// println!("Page uses {} distinct fonts", page.font_count());
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn font_count(&self) -> usize {
        self.extract_font_usage().len()
    }

    /// Check if this page uses any mathematical fonts.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// if page.has_math_fonts() {
    ///     println!("Page uses mathematical fonts");
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn has_math_fonts(&self) -> bool {
        self.extract_font_usage().iter().any(|f| f.is_math_font)
    }

    /// Get the names of all fonts used on this page.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// println!("Fonts: {:?}", page.font_names());
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn font_names(&self) -> Vec<String> {
        self.extract_font_usage()
            .into_iter()
            .map(|f| f.name)
            .collect()
    }

    // ========================================================================
    // Centered Block Detection (Feature 13)
    // ========================================================================

    /// Extract text blocks that appear centered on the page.
    ///
    /// A block is considered "centered" if its left and right margins are
    /// approximately equal (within the given tolerance).
    ///
    /// # Arguments
    ///
    /// * `tolerance` - Maximum allowed difference between left and right margins
    ///   for a block to be considered centered. Smaller values are stricter.
    ///   Common values: 5.0 (strict), 10.0 (normal), 20.0 (loose).
    ///
    /// # Returns
    ///
    /// A vector of `CenteredBlock` structs, sorted by margin_symmetry (most centered first).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// // Find blocks centered within 10 point tolerance
    /// let centered = page.extract_centered_blocks(10.0);
    ///
    /// for block in centered {
    ///     println!("Centered text: \"{}\"", block.text);
    ///     println!("  Margins: L={:.1} R={:.1} (symmetry: {:.1})",
    ///         block.margin_left, block.margin_right, block.margin_symmetry);
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn extract_centered_blocks(&self, tolerance: f32) -> Vec<CenteredBlock> {
        let (page_width, _page_height) = self.size();
        let page_width = page_width as f32;

        let text = match self.text() {
            Ok(t) => t,
            Err(_) => return vec![],
        };

        // Get text as lines with bounds
        // We'll approximate by iterating through words and grouping by Y position
        let words = text.words();
        if words.is_empty() {
            return vec![];
        }

        // Group words into lines based on Y position
        let mut lines: Vec<(f32, f32, f32, f32, String)> = vec![]; // (left, bottom, right, top, text)
        let mut current_line_y: Option<f32> = None;
        let mut current_line_left = f32::MAX;
        let mut current_line_right = f32::MIN;
        let mut current_line_bottom = f32::MAX;
        let mut current_line_top = f32::MIN;
        let mut current_line_text = String::new();

        for word in words {
            let word_bottom = word.bottom as f32;
            let line_threshold = 5.0; // Points - words within this Y are same line

            let same_line =
                current_line_y.is_some_and(|y| (word_bottom - y).abs() < line_threshold);

            if same_line {
                // Add to current line
                current_line_left = current_line_left.min(word.left as f32);
                current_line_right = current_line_right.max(word.right as f32);
                current_line_bottom = current_line_bottom.min(word.bottom as f32);
                current_line_top = current_line_top.max(word.top as f32);
                if !current_line_text.is_empty() {
                    current_line_text.push(' ');
                }
                current_line_text.push_str(&word.text);
            } else {
                // Save previous line if exists
                if current_line_y.is_some() && !current_line_text.is_empty() {
                    lines.push((
                        current_line_left,
                        current_line_bottom,
                        current_line_right,
                        current_line_top,
                        current_line_text.clone(),
                    ));
                }

                // Start new line
                current_line_y = Some(word_bottom);
                current_line_left = word.left as f32;
                current_line_right = word.right as f32;
                current_line_bottom = word.bottom as f32;
                current_line_top = word.top as f32;
                current_line_text = word.text.clone();
            }
        }

        // Don't forget the last line
        if current_line_y.is_some() && !current_line_text.is_empty() {
            lines.push((
                current_line_left,
                current_line_bottom,
                current_line_right,
                current_line_top,
                current_line_text,
            ));
        }

        // Find centered lines
        let mut centered_blocks: Vec<CenteredBlock> = vec![];

        for (left, bottom, right, top, text) in lines {
            let margin_left = left;
            let margin_right = page_width - right;

            let margin_symmetry = (margin_left - margin_right).abs();

            // Check if centered within tolerance
            if margin_symmetry <= tolerance {
                // Also require meaningful margins (not full-width text)
                let min_margin = 20.0; // At least 20 points from edges
                if margin_left >= min_margin && margin_right >= min_margin {
                    centered_blocks.push(CenteredBlock::new(
                        (left, bottom, right, top),
                        text,
                        margin_left,
                        margin_right,
                    ));
                }
            }
        }

        // Sort by symmetry (most centered first)
        centered_blocks.sort_by(|a, b| {
            a.margin_symmetry
                .partial_cmp(&b.margin_symmetry)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        centered_blocks
    }

    /// Get count of centered blocks with given tolerance.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// println!("Found {} centered blocks", page.centered_block_count(10.0));
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn centered_block_count(&self, tolerance: f32) -> usize {
        self.extract_centered_blocks(tolerance).len()
    }

    /// Check if page has any centered content.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// if page.has_centered_content(15.0) {
    ///     println!("Page has centered content (likely title or heading)");
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn has_centered_content(&self, tolerance: f32) -> bool {
        !self.extract_centered_blocks(tolerance).is_empty()
    }

    // ========================================================================
    // Bracketed Reference Detection (Feature 14)
    // ========================================================================

    /// Extract bracketed references (citations, footnotes) from the page.
    ///
    /// Detects patterns like `[1]`, `[ref]`, (note 5), superscript numbers, etc.
    ///
    /// # Returns
    ///
    /// A vector of detected `BracketedReference` structs.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("paper.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// let refs = page.extract_bracketed_references();
    /// for r in refs {
    ///     println!("Found reference: {} at ({:.1}, {:.1})",
    ///         r.text, r.bounds.0, r.bounds.1);
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn extract_bracketed_references(&self) -> Vec<BracketedReference> {
        let text = match self.text() {
            Ok(t) => t,
            Err(_) => return vec![],
        };

        let all_text = text.all();
        let chars: Vec<_> = text.chars().collect();

        if chars.is_empty() {
            return vec![];
        }

        let mut references = vec![];

        // Detect square bracket references: [1], [1,2], [1-5], [ref]
        self.find_bracket_references(
            &all_text,
            &chars,
            '[',
            ']',
            BracketType::Square,
            &mut references,
        );

        // Detect parenthetical references: (1), (ref)
        self.find_bracket_references(
            &all_text,
            &chars,
            '(',
            ')',
            BracketType::Paren,
            &mut references,
        );

        // Detect angle bracket references: <1>, <ref>
        self.find_bracket_references(
            &all_text,
            &chars,
            '<',
            '>',
            BracketType::Angle,
            &mut references,
        );

        // Detect Unicode superscript numbers: ¹, ², ³, etc.
        self.find_superscript_references(&chars, &mut references);

        references
    }

    /// Helper to find bracketed references with specific bracket characters.
    fn find_bracket_references(
        &self,
        all_text: &str,
        chars: &[crate::text::PdfChar],
        open: char,
        close: char,
        bracket_type: BracketType,
        references: &mut Vec<BracketedReference>,
    ) {
        let (page_width, _) = self.size();
        let page_width = page_width as f32;

        let mut i = 0;
        while i < all_text.len() {
            if let Some(start) = all_text[i..].find(open) {
                let abs_start = i + start;
                if let Some(end_offset) = all_text[abs_start..].find(close) {
                    let abs_end = abs_start + end_offset + 1;

                    // Extract the bracketed content
                    let ref_text: String = all_text[abs_start..abs_end].to_string();

                    // Skip if too long (probably not a reference)
                    if ref_text.len() > 20 {
                        i = abs_start + 1;
                        continue;
                    }

                    // Get bounds from character positions
                    let (bounds, position) = self
                        .get_reference_bounds_and_position(abs_start, abs_end, chars, page_width);

                    if let Some(bounds) = bounds {
                        references.push(BracketedReference::new(
                            ref_text,
                            bounds,
                            bracket_type,
                            position,
                        ));
                    }

                    i = abs_end;
                } else {
                    i = abs_start + 1;
                }
            } else {
                break;
            }
        }
    }

    /// Helper to find Unicode superscript number references.
    fn find_superscript_references(
        &self,
        chars: &[crate::text::PdfChar],
        references: &mut Vec<BracketedReference>,
    ) {
        let (page_width, _) = self.size();
        let page_width = page_width as f32;

        // Unicode superscript digits: ⁰¹²³⁴⁵⁶⁷⁸⁹
        const SUPERSCRIPT_DIGITS: &[char] = &['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];

        let mut i = 0;
        while i < chars.len() {
            if SUPERSCRIPT_DIGITS.contains(&chars[i].unicode) {
                // Found start of superscript sequence
                let start = i;
                let mut end = i;

                // Collect consecutive superscript digits
                while end < chars.len() && SUPERSCRIPT_DIGITS.contains(&chars[end].unicode) {
                    end += 1;
                }

                // Build the text
                let ref_text: String = chars[start..end].iter().map(|c| c.unicode).collect();

                // Get bounds
                let left = chars[start].left as f32;
                let right = chars[end - 1].right as f32;
                let bottom = chars[start..end]
                    .iter()
                    .map(|c| c.bottom as f32)
                    .fold(f32::MAX, f32::min);
                let top = chars[start..end]
                    .iter()
                    .map(|c| c.top as f32)
                    .fold(f32::MIN, f32::max);

                // Determine position
                let position = if left < 50.0 {
                    ReferencePosition::LineStart
                } else if right > page_width - 50.0 {
                    ReferencePosition::LineEnd
                } else {
                    ReferencePosition::Inline
                };

                references.push(BracketedReference::new(
                    ref_text,
                    (left, bottom, right, top),
                    BracketType::Superscript,
                    position,
                ));

                i = end;
            } else {
                i += 1;
            }
        }
    }

    /// Helper to get bounds and position for a reference.
    fn get_reference_bounds_and_position(
        &self,
        _start_idx: usize,
        _end_idx: usize,
        chars: &[crate::text::PdfChar],
        page_width: f32,
    ) -> (Option<(f32, f32, f32, f32)>, ReferencePosition) {
        // Use character bounds if available
        // Note: Character indices in text string may not directly map to PdfChar indices
        // For now, use a simpler approach - just get the first/last chars as approximation
        if chars.is_empty() {
            return (None, ReferencePosition::Inline);
        }

        // Since we can't reliably map string indices to char indices,
        // return approximate bounds based on first char position
        let first = &chars[0];
        let left = first.left as f32;
        let right = first.right as f32;
        let bottom = first.bottom as f32;
        let top = first.top as f32;

        let position = if left < 50.0 {
            ReferencePosition::LineStart
        } else if right > page_width - 50.0 {
            ReferencePosition::LineEnd
        } else {
            ReferencePosition::Inline
        };

        (Some((left, bottom, right, top)), position)
    }

    /// Get count of bracketed references on this page.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("paper.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// println!("Found {} references on page", page.reference_count());
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn reference_count(&self) -> usize {
        self.extract_bracketed_references().len()
    }

    /// Check if page has any bracketed references.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("paper.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// if page.has_references() {
    ///     println!("Page contains citations or references");
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn has_references(&self) -> bool {
        !self.extract_bracketed_references().is_empty()
    }

    /// Get only square bracket references [like this].
    pub fn square_bracket_references(&self) -> Vec<BracketedReference> {
        self.extract_bracketed_references()
            .into_iter()
            .filter(|r| r.bracket_type == BracketType::Square)
            .collect()
    }

    /// Get only numeric references (e.g., `[1]`, `[23]`, (5)).
    pub fn numeric_references(&self) -> Vec<BracketedReference> {
        self.extract_bracketed_references()
            .into_iter()
            .filter(|r| r.is_numeric())
            .collect()
    }

    // ========================================================================
    // Script Cluster Detection (Feature 15)
    // ========================================================================

    /// Extract subscript/superscript clusters from the page.
    ///
    /// Identifies base characters with associated raised (superscript) or
    /// lowered (subscript) characters, based on text rise values.
    ///
    /// # Returns
    ///
    /// A vector of `ScriptCluster` structs containing base text and script characters.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("paper.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// for cluster in page.extract_script_clusters() {
    ///     println!("Base: '{}', Scripts: '{}'",
    ///         cluster.base_text, cluster.script_text());
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn extract_script_clusters(&self) -> Vec<ScriptCluster> {
        let text = match self.text() {
            Ok(t) => t,
            Err(_) => return vec![],
        };

        let chars: Vec<_> = text.chars().collect();
        if chars.is_empty() {
            return vec![];
        }

        let mut clusters = vec![];

        // Threshold for rise detection (in points)
        // Typical superscripts have rise > 2.0, subscripts < -2.0
        let rise_threshold = 2.0;

        let mut i = 0;
        while i < chars.len() {
            let ch = &chars[i];

            // Check if this is a script character (non-zero rise)
            if ch.text_rise().abs() > rise_threshold {
                // This is a script character, find its base
                // Look back to find the preceding base character
                if i > 0 {
                    let base_idx = i - 1;
                    let base = &chars[base_idx];

                    // Only if base is on baseline (near-zero rise)
                    if base.text_rise().abs() <= rise_threshold {
                        // Start collecting scripts
                        let mut scripts = vec![];
                        let mut script_end = i;

                        while script_end < chars.len() {
                            let sc = &chars[script_end];
                            if sc.text_rise().abs() > rise_threshold {
                                let position = if sc.text_rise() > 0.0 {
                                    ScriptPosition::Super
                                } else {
                                    ScriptPosition::Sub
                                };
                                scripts.push(ScriptChar::new(
                                    sc.unicode,
                                    position,
                                    (
                                        sc.left as f32,
                                        sc.bottom as f32,
                                        sc.right as f32,
                                        sc.top as f32,
                                    ),
                                    sc.text_rise(),
                                ));
                                script_end += 1;
                            } else {
                                break;
                            }
                        }

                        if !scripts.is_empty() {
                            clusters.push(ScriptCluster::new(
                                base.unicode.to_string(),
                                (
                                    base.left as f32,
                                    base.bottom as f32,
                                    base.right as f32,
                                    base.top as f32,
                                ),
                                scripts,
                            ));
                        }

                        i = script_end;
                        continue;
                    }
                }
            }
            i += 1;
        }

        clusters
    }

    /// Get count of script clusters on this page.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("paper.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// println!("Found {} script clusters", page.script_cluster_count());
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn script_cluster_count(&self) -> usize {
        self.extract_script_clusters().len()
    }

    /// Check if page has any script clusters.
    pub fn has_script_clusters(&self) -> bool {
        !self.extract_script_clusters().is_empty()
    }

    /// Get clusters that have superscripts.
    pub fn superscript_clusters(&self) -> Vec<ScriptCluster> {
        self.extract_script_clusters()
            .into_iter()
            .filter(|c| c.has_superscripts())
            .collect()
    }

    /// Get clusters that have subscripts.
    pub fn subscript_clusters(&self) -> Vec<ScriptCluster> {
        self.extract_script_clusters()
            .into_iter()
            .filter(|c| c.has_subscripts())
            .collect()
    }

    // ========================================================================
    // Writing Direction Detection (Feature 16 - Japanese text support)
    // ========================================================================

    /// Detect the writing direction of text on this page.
    ///
    /// Analyzes character positions to determine if text flows horizontally
    /// (left-to-right) or vertically (top-to-bottom, right-to-left columns).
    /// This is commonly used for Japanese, Chinese, and Korean documents.
    ///
    /// # Algorithm
    ///
    /// 1. Examines consecutive character pairs
    /// 2. Classifies movement as horizontal (x increases) or vertical (y decreases)
    /// 3. Groups vertical characters into regions
    /// 4. Returns the primary direction based on character counts
    ///
    /// # Returns
    ///
    /// A `WritingDirectionInfo` containing:
    /// - `primary_direction`: The dominant writing direction
    /// - `vertical_ratio`: Proportion of characters in vertical text (0.0-1.0)
    /// - `horizontal_ratio`: Proportion of characters in horizontal text (0.0-1.0)
    /// - `vertical_regions`: Bounding boxes of vertical text areas
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::{Pdfium, WritingDirection};
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// let info = page.detect_writing_direction();
    /// match info.primary_direction {
    ///     WritingDirection::Horizontal => println!("Standard horizontal text"),
    ///     WritingDirection::VerticalRTL => println!("Vertical Japanese/Chinese text"),
    ///     WritingDirection::Mixed => println!("Contains both directions"),
    /// }
    ///
    /// if info.has_vertical_text() {
    ///     println!("Found {} vertical regions", info.vertical_region_count());
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn detect_writing_direction(&self) -> WritingDirectionInfo {
        let text = match self.text() {
            Ok(t) => t,
            Err(_) => return WritingDirectionInfo::horizontal(),
        };

        let chars: Vec<_> = text.chars().collect();
        if chars.len() < 2 {
            return WritingDirectionInfo::horizontal();
        }

        // Analyze character flow direction
        let mut horizontal_count = 0usize;
        let mut vertical_count = 0usize;

        // Track current vertical region
        let mut current_vertical_region: Option<(f32, f32, f32, f32)> = None;
        let mut vertical_regions: Vec<(f32, f32, f32, f32)> = Vec::new();

        for i in 0..chars.len().saturating_sub(1) {
            let curr = &chars[i];
            let next = &chars[i + 1];

            // Skip control characters and very small chars
            if curr.unicode.is_control() || next.unicode.is_control() {
                continue;
            }

            let dx = (next.left - curr.left) as f32;
            let dy = (next.bottom - curr.bottom) as f32;

            // Use character height as reference for vertical detection
            let char_height = curr.height() as f32;
            let min_move = char_height * 0.3; // Minimum movement threshold

            // Detect writing direction from character flow
            // Vertical: y decreases significantly (top to bottom), x stays similar
            // Horizontal: x increases significantly (left to right), y stays similar
            let is_vertical_move = dy < -min_move && dx.abs() < char_height;
            let is_horizontal_move = dx > min_move && dy.abs() < char_height * 0.5;

            if is_vertical_move {
                vertical_count += 1;

                // Character bounds for region tracking
                let char_bounds = (
                    curr.left as f32,
                    curr.bottom as f32,
                    curr.right as f32,
                    curr.top as f32,
                );

                // Extend or start vertical region
                if let Some(ref mut region) = current_vertical_region {
                    // Extend region to include this character
                    region.0 = region.0.min(curr.left as f32);
                    region.1 = region.1.min(curr.bottom as f32);
                    region.2 = region.2.max(curr.right as f32);
                    region.3 = region.3.max(curr.top as f32);
                } else {
                    current_vertical_region = Some(char_bounds);
                }
            } else if is_horizontal_move {
                horizontal_count += 1;

                // End current vertical region if we encounter horizontal text
                if let Some(region) = current_vertical_region.take() {
                    // Only add region if it has meaningful size
                    if region.3 - region.1 > char_height * 2.0 {
                        vertical_regions.push(region);
                    }
                }
            } else {
                // Neutral movement (same line, same column, or diagonal)
                // Continue current region if we're in one
            }
        }

        // Don't forget the last vertical region
        if let Some(region) = current_vertical_region {
            let char_height = if !chars.is_empty() {
                chars[0].height() as f32
            } else {
                12.0
            };
            if region.3 - region.1 > char_height * 2.0 {
                vertical_regions.push(region);
            }
        }

        let total = (horizontal_count + vertical_count).max(1) as f32;
        let vertical_ratio = vertical_count as f32 / total;
        let horizontal_ratio = horizontal_count as f32 / total;

        // Determine primary direction
        let primary_direction = if vertical_ratio > 0.7 {
            WritingDirection::VerticalRTL
        } else if horizontal_ratio > 0.7 {
            WritingDirection::Horizontal
        } else if vertical_ratio > 0.1 && horizontal_ratio > 0.1 {
            WritingDirection::Mixed
        } else {
            WritingDirection::Horizontal
        };

        WritingDirectionInfo {
            primary_direction,
            vertical_ratio,
            horizontal_ratio,
            vertical_regions,
        }
    }

    /// Check if this page contains vertical text.
    ///
    /// A convenience method that returns true if any vertical text is detected.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// if page.has_vertical_text() {
    ///     println!("This page contains vertical text (Japanese/Chinese layout)");
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn has_vertical_text(&self) -> bool {
        self.detect_writing_direction().has_vertical_text()
    }

    /// Check if this page is primarily vertical text.
    ///
    /// Returns true if more than 50% of the text flow is vertical.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("novel.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// if page.is_vertical_text_page() {
    ///     println!("This is a vertical text page");
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn is_vertical_text_page(&self) -> bool {
        self.detect_writing_direction().is_predominantly_vertical()
    }

    /// Get the primary writing direction of this page.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::{Pdfium, WritingDirection};
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// let direction = page.writing_direction();
    /// println!("Writing direction: {:?}", direction);
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn writing_direction(&self) -> WritingDirection {
        self.detect_writing_direction().primary_direction
    }

    // ========================================================================
    // Ruby Text (Furigana) Extraction (Feature 17 - Japanese text support)
    // ========================================================================

    /// Extract ruby annotations (furigana) from this page.
    ///
    /// Ruby text is small reading aid text placed above (horizontal) or beside (vertical)
    /// base characters in Japanese/Chinese text. This method detects pairs of base text
    /// and their ruby annotations based on font size and spatial relationships.
    ///
    /// # Algorithm
    ///
    /// 1. Groups characters by font size
    /// 2. Identifies small text (< 60% of median font size) as potential ruby
    /// 3. Matches ruby text to nearby larger text (above or to the right)
    /// 4. Returns paired annotations with position and size information
    ///
    /// # Returns
    ///
    /// A vector of `RubyAnnotation` containing base text and ruby text pairs.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("japanese.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// for ruby in page.extract_ruby_annotations() {
    ///     println!("{} ({})", ruby.base_text, ruby.ruby_text);
    ///     println!("  Size ratio: {:.2}", ruby.size_ratio);
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn extract_ruby_annotations(&self) -> Vec<RubyAnnotation> {
        let text = match self.text() {
            Ok(t) => t,
            Err(_) => return Vec::new(),
        };

        let chars: Vec<_> = text.chars().collect();
        if chars.len() < 2 {
            return Vec::new();
        }

        // Collect font sizes to determine the median
        let mut font_sizes: Vec<f64> = chars
            .iter()
            .filter(|c| !c.unicode.is_control() && !c.unicode.is_whitespace())
            .map(|c| c.font_size)
            .filter(|&s| s > 0.1)
            .collect();

        if font_sizes.is_empty() {
            return Vec::new();
        }

        font_sizes.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median_size = font_sizes[font_sizes.len() / 2];

        // Ruby text is typically much smaller than base text (30-60% of base)
        let ruby_threshold = median_size * 0.65;
        let min_ruby_size = median_size * 0.2; // Ruby shouldn't be too small

        // Group characters by whether they could be ruby or base
        let mut potential_ruby: Vec<(usize, &crate::text::PdfChar)> = Vec::new();
        let mut base_chars: Vec<(usize, &crate::text::PdfChar)> = Vec::new();

        for (i, ch) in chars.iter().enumerate() {
            if ch.unicode.is_control() || ch.unicode.is_whitespace() {
                continue;
            }
            if ch.font_size < 0.1 {
                continue;
            }

            if ch.font_size < ruby_threshold && ch.font_size > min_ruby_size {
                potential_ruby.push((i, ch));
            } else if ch.font_size >= ruby_threshold {
                base_chars.push((i, ch));
            }
        }

        let mut annotations = Vec::new();

        // Group consecutive potential ruby characters
        let mut ruby_groups: Vec<Vec<(usize, &crate::text::PdfChar)>> = Vec::new();
        let mut current_group: Vec<(usize, &crate::text::PdfChar)> = Vec::new();

        for (i, ch) in potential_ruby {
            if let Some((last_i, last_ch)) = current_group.last() {
                // Check if this character is consecutive (horizontally or vertically)
                let dx = (ch.left - last_ch.right).abs();
                let dy = (ch.bottom - last_ch.bottom).abs();
                let char_size = ch.font_size;

                // Allow small gaps
                if dx < char_size * 2.0 && dy < char_size * 1.5 && i == last_i + 1 {
                    current_group.push((i, ch));
                } else {
                    if !current_group.is_empty() {
                        ruby_groups.push(std::mem::take(&mut current_group));
                    }
                    current_group.push((i, ch));
                }
            } else {
                current_group.push((i, ch));
            }
        }
        if !current_group.is_empty() {
            ruby_groups.push(current_group);
        }

        // Match ruby groups with base text
        for ruby_group in ruby_groups {
            if ruby_group.is_empty() {
                continue;
            }

            // Get ruby bounds and text
            let ruby_text: String = ruby_group.iter().map(|(_, ch)| ch.unicode).collect();
            let ruby_left = ruby_group
                .iter()
                .map(|(_, c)| c.left)
                .fold(f64::MAX, f64::min);
            let ruby_bottom = ruby_group
                .iter()
                .map(|(_, c)| c.bottom)
                .fold(f64::MAX, f64::min);
            let ruby_right = ruby_group
                .iter()
                .map(|(_, c)| c.right)
                .fold(f64::MIN, f64::max);
            let ruby_top = ruby_group
                .iter()
                .map(|(_, c)| c.top)
                .fold(f64::MIN, f64::max);
            let ruby_font_size = ruby_group[0].1.font_size;

            // Find matching base text (text that is below/left of ruby and similar width)
            let mut best_match: Option<(f64, Vec<(usize, &crate::text::PdfChar)>)> = None;

            // Try to find base text below (horizontal writing) or to the left (vertical)
            for (base_idx, base_char) in &base_chars {
                // Check if this base char is positioned correctly relative to ruby
                // For horizontal: base should be below ruby
                // For vertical: base should be to the left of ruby

                let base_center_x = (base_char.left + base_char.right) / 2.0;
                let ruby_center_x = (ruby_left + ruby_right) / 2.0;

                // Horizontal case: ruby is above base
                let is_above = ruby_bottom > base_char.top - base_char.font_size * 0.3
                    && ruby_top < base_char.top + base_char.font_size * 2.0
                    && (base_center_x - ruby_center_x).abs() < base_char.font_size * 2.0;

                // Vertical case: ruby is to the right of base
                let base_center_y = (base_char.bottom + base_char.top) / 2.0;
                let ruby_center_y = (ruby_bottom + ruby_top) / 2.0;
                let is_right = ruby_left > base_char.right - base_char.font_size * 0.3
                    && ruby_right < base_char.right + base_char.font_size * 2.0
                    && (base_center_y - ruby_center_y).abs() < base_char.font_size * 2.0;

                if is_above || is_right {
                    let distance = if is_above {
                        (ruby_bottom - base_char.top).abs() + (base_center_x - ruby_center_x).abs()
                    } else {
                        (ruby_left - base_char.right).abs() + (base_center_y - ruby_center_y).abs()
                    };

                    if best_match.is_none() || distance < best_match.as_ref().unwrap().0 {
                        best_match = Some((distance, vec![(*base_idx, *base_char)]));
                    }
                }
            }

            // If we found a match, create the annotation
            if let Some((_, base_group)) = best_match {
                let base_text: String = base_group.iter().map(|(_, ch)| ch.unicode).collect();
                let base_left = base_group
                    .iter()
                    .map(|(_, c)| c.left)
                    .fold(f64::MAX, f64::min);
                let base_bottom = base_group
                    .iter()
                    .map(|(_, c)| c.bottom)
                    .fold(f64::MAX, f64::min);
                let base_right = base_group
                    .iter()
                    .map(|(_, c)| c.right)
                    .fold(f64::MIN, f64::max);
                let base_top = base_group
                    .iter()
                    .map(|(_, c)| c.top)
                    .fold(f64::MIN, f64::max);
                let base_font_size = base_group[0].1.font_size;

                let size_ratio = if base_font_size > 0.0 {
                    ruby_font_size / base_font_size
                } else {
                    0.5
                };

                // Only include if size ratio is reasonable for ruby (typically 0.3-0.6)
                if size_ratio > 0.2 && size_ratio < 0.75 {
                    annotations.push(RubyAnnotation::new(
                        base_text,
                        ruby_text,
                        (
                            base_left as f32,
                            base_bottom as f32,
                            base_right as f32,
                            base_top as f32,
                        ),
                        (
                            ruby_left as f32,
                            ruby_bottom as f32,
                            ruby_right as f32,
                            ruby_top as f32,
                        ),
                        ruby_font_size as f32,
                        size_ratio as f32,
                    ));
                }
            }
        }

        annotations
    }

    /// Check if this page has any ruby annotations.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// if page.has_ruby_annotations() {
    ///     println!("This page contains ruby (furigana) text");
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn has_ruby_annotations(&self) -> bool {
        !self.extract_ruby_annotations().is_empty()
    }

    /// Get count of ruby annotations on this page.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// println!("Found {} ruby annotations", page.ruby_annotation_count());
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn ruby_annotation_count(&self) -> usize {
        self.extract_ruby_annotations().len()
    }

    // ========================================================================
    // Japanese Character Analysis (Feature 18 - Japanese text support)
    // ========================================================================

    /// Analyze Japanese character types on this page.
    ///
    /// Counts hiragana, katakana, kanji, and other Japanese character categories
    /// to help identify Japanese content and analyze writing system composition.
    ///
    /// # Returns
    ///
    /// A `JapaneseCharAnalysis` containing character counts by category.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// let analysis = page.analyze_japanese_chars();
    /// if analysis.has_japanese() {
    ///     println!("Japanese content detected:");
    ///     println!("  Hiragana: {}", analysis.hiragana_count);
    ///     println!("  Katakana: {}", analysis.katakana_count);
    ///     println!("  Kanji: {}", analysis.kanji_count);
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn analyze_japanese_chars(&self) -> JapaneseCharAnalysis {
        let text = match self.text() {
            Ok(t) => t,
            Err(_) => return JapaneseCharAnalysis::new(),
        };
        text.japanese_char_analysis()
    }

    /// Check if this page contains any Japanese characters.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// if page.has_japanese_text() {
    ///     println!("This page contains Japanese text");
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn has_japanese_text(&self) -> bool {
        self.analyze_japanese_chars().has_japanese()
    }

    /// Check if this page is predominantly Japanese text.
    ///
    /// Returns true if more than 50% of characters are Japanese.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// if page.is_japanese_page() {
    ///     println!("This is a Japanese document page");
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn is_japanese_page(&self) -> bool {
        self.analyze_japanese_chars().is_predominantly_japanese()
    }

    // ========================================================================
    // Japanese Punctuation Detection (Feature 19 - Japanese text support)
    // ========================================================================

    /// Extract Japanese punctuation marks from this page.
    ///
    /// Identifies and classifies Japanese punctuation characters including:
    /// - Periods (。) and commas (、)
    /// - Quote marks (「」『』etc.)
    /// - Middle dots (・), long vowels (ー)
    /// - Wave dashes (〜), repetition marks (々)
    ///
    /// # Returns
    ///
    /// A vector of `JapanesePunctuation` with character, position, and type.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::{Pdfium, JPunctType};
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// for punct in page.extract_japanese_punctuation() {
    ///     match punct.punct_type {
    ///         JPunctType::Period => println!("Period at ({:.1}, {:.1})", punct.bounds.0, punct.bounds.1),
    ///         JPunctType::QuoteOpen => println!("Opening quote: '{}'", punct.char),
    ///         _ => {}
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn extract_japanese_punctuation(&self) -> Vec<JapanesePunctuation> {
        let text = match self.text() {
            Ok(t) => t,
            Err(_) => return Vec::new(),
        };

        let chars: Vec<_> = text.chars().collect();
        let direction = self.detect_writing_direction();
        let is_vertical = direction.primary_direction == WritingDirection::VerticalRTL;

        let mut punctuation = Vec::new();

        for ch in chars {
            if let Some(punct_type) = JPunctType::classify(ch.unicode) {
                let bounds = (
                    ch.left as f32,
                    ch.bottom as f32,
                    ch.right as f32,
                    ch.top as f32,
                );

                // Detect vertical variants based on aspect ratio and page direction
                // Vertical punctuation (。、) often has different shapes in vertical text
                let is_vertical_variant = if is_vertical {
                    // In vertical text, punctuation might be rotated or positioned differently
                    true
                } else {
                    // In horizontal text, check if aspect ratio suggests vertical variant
                    let width = ch.width() as f32;
                    let height = ch.height() as f32;
                    // Some punctuation is taller than wide in vertical mode
                    height > width * 1.5
                };

                punctuation.push(JapanesePunctuation::new(
                    ch.unicode,
                    bounds,
                    punct_type,
                    is_vertical_variant,
                ));
            }
        }

        punctuation
    }

    /// Check if this page has any Japanese punctuation.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// if page.has_japanese_punctuation() {
    ///     println!("This page contains Japanese punctuation");
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn has_japanese_punctuation(&self) -> bool {
        !self.extract_japanese_punctuation().is_empty()
    }

    /// Get count of Japanese punctuation marks on this page.
    pub fn japanese_punctuation_count(&self) -> usize {
        self.extract_japanese_punctuation().len()
    }

    /// Extract emphasis marks (傍点) from this page.
    ///
    /// Emphasis marks are small marks (dots, circles, etc.) placed above or beside
    /// characters in Japanese text to emphasize them. This method finds such marks
    /// by detecting small mark characters positioned near larger base characters.
    ///
    /// # Algorithm
    ///
    /// 1. Identify potential mark characters (●, ○, •, etc.)
    /// 2. For each mark, find nearby base characters
    /// 3. Match marks to base characters based on position alignment
    ///
    /// # Returns
    ///
    /// Vector of `EmphasisMark` structs pairing marks with their base characters.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// for mark in page.extract_emphasis_marks() {
    ///     println!("Character '{}' has {:?} emphasis",
    ///         mark.base_char, mark.mark_type);
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn extract_emphasis_marks(&self) -> Vec<EmphasisMark> {
        let text = match self.text() {
            Ok(t) => t,
            Err(_) => return Vec::new(),
        };

        let chars: Vec<_> = text.chars().collect();
        if chars.is_empty() {
            return Vec::new();
        }

        // Collect all characters with their info
        struct CharInfo {
            ch: char,
            bounds: (f32, f32, f32, f32),
            center_x: f32,
            center_y: f32,
            height: f32,
            is_mark: Option<EmphasisMarkType>,
        }

        let char_infos: Vec<CharInfo> = chars
            .iter()
            .map(|ch| {
                let bounds = (
                    ch.left as f32,
                    ch.bottom as f32,
                    ch.right as f32,
                    ch.top as f32,
                );
                let center_x = (bounds.0 + bounds.2) / 2.0;
                let center_y = (bounds.1 + bounds.3) / 2.0;
                let height = bounds.3 - bounds.1;
                let is_mark = EmphasisMarkType::classify(ch.unicode);
                CharInfo {
                    ch: ch.unicode,
                    bounds,
                    center_x,
                    center_y,
                    height,
                    is_mark,
                }
            })
            .collect();

        // Calculate average character height for non-marks
        let non_mark_heights: Vec<f32> = char_infos
            .iter()
            .filter(|c| c.is_mark.is_none() && c.height > 0.0)
            .map(|c| c.height)
            .collect();

        let avg_height = if non_mark_heights.is_empty() {
            12.0 // Default assumption
        } else {
            non_mark_heights.iter().sum::<f32>() / non_mark_heights.len() as f32
        };

        // Emphasis marks are typically much smaller than regular characters
        let mark_size_threshold = avg_height * 0.6;

        let mut results = Vec::new();

        // Find potential emphasis marks (small marks above/beside characters)
        for (i, info) in char_infos.iter().enumerate() {
            if let Some(mark_type) = &info.is_mark {
                // This character is a potential emphasis mark
                // Check if it's small enough to be an emphasis mark
                if info.height >= mark_size_threshold {
                    continue; // Too large to be an emphasis mark
                }

                // Look for a base character nearby (typically below for horizontal text)
                // Search within a reasonable distance
                let search_range = avg_height * 2.0;

                let mut best_match: Option<(usize, f32)> = None;

                for (j, base_info) in char_infos.iter().enumerate() {
                    if i == j {
                        continue;
                    }
                    // Skip other marks
                    if base_info.is_mark.is_some() {
                        continue;
                    }
                    // Skip whitespace and punctuation as base characters
                    if base_info.ch.is_whitespace() || base_info.ch.is_ascii_punctuation() {
                        continue;
                    }

                    // Check horizontal alignment (centers should be close)
                    let x_diff = (info.center_x - base_info.center_x).abs();
                    if x_diff > avg_height * 0.5 {
                        continue; // Not horizontally aligned
                    }

                    // Check vertical distance (mark should be above or to the side)
                    let y_diff = info.center_y - base_info.center_y;

                    // For horizontal text: mark is above (positive y_diff)
                    // For vertical text: mark might be to the right
                    let distance = (x_diff * x_diff + y_diff * y_diff).sqrt();

                    if distance < search_range {
                        // Prefer marks that are above the character
                        let score = if y_diff > 0.0 {
                            distance // Mark is above - good
                        } else {
                            distance * 2.0 // Mark is below - less likely
                        };

                        if best_match.is_none() || score < best_match.unwrap().1 {
                            best_match = Some((j, score));
                        }
                    }
                }

                if let Some((base_idx, _)) = best_match {
                    let base_info = &char_infos[base_idx];
                    results.push(EmphasisMark::new(
                        base_info.ch,
                        base_info.bounds,
                        info.bounds,
                        *mark_type,
                    ));
                }
            }
        }

        results
    }

    /// Check if this page has any emphasis marks.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// if page.has_emphasis_marks() {
    ///     println!("This page contains emphasis marks");
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn has_emphasis_marks(&self) -> bool {
        !self.extract_emphasis_marks().is_empty()
    }

    /// Get count of emphasis marks on this page.
    pub fn emphasis_mark_count(&self) -> usize {
        self.extract_emphasis_marks().len()
    }

    /// Analyze grid lines on this page for table detection.
    ///
    /// This method extracts horizontal and vertical lines, finds their
    /// intersections, and identifies potential table cells.
    ///
    /// # Returns
    ///
    /// A `GridAnalysis` struct containing:
    /// - `intersections`: Points where lines cross
    /// - `row_separators`: Y-coordinates of horizontal lines
    /// - `column_separators`: X-coordinates of vertical lines
    /// - `cell_bounds`: Bounding boxes of detected cells
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("table.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// let grid = page.analyze_grid_lines();
    /// if grid.is_valid_table() {
    ///     println!("Found {}x{} table", grid.row_count(), grid.column_count());
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn analyze_grid_lines(&self) -> GridAnalysis {
        use crate::page_object::ExtractedLine;

        let lines = self.extract_lines();
        if lines.is_empty() {
            return GridAnalysis::new();
        }

        // Separate horizontal and vertical lines
        let h_lines: Vec<(usize, &ExtractedLine)> = lines
            .iter()
            .enumerate()
            .filter(|(_, l)| l.is_horizontal)
            .collect();
        let v_lines: Vec<(usize, &ExtractedLine)> = lines
            .iter()
            .enumerate()
            .filter(|(_, l)| l.is_vertical)
            .collect();

        if h_lines.is_empty() || v_lines.is_empty() {
            return GridAnalysis::new();
        }

        // Collect unique Y positions for row separators (from horizontal lines)
        let mut row_ys: Vec<f32> = Vec::new();
        for (_, line) in &h_lines {
            let y = (line.start.1 + line.end.1) / 2.0;
            // Check if this Y is unique (within tolerance)
            if !row_ys.iter().any(|&ry| (ry - y).abs() < 2.0) {
                row_ys.push(y);
            }
        }
        row_ys.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        // Collect unique X positions for column separators (from vertical lines)
        let mut col_xs: Vec<f32> = Vec::new();
        for (_, line) in &v_lines {
            let x = (line.start.0 + line.end.0) / 2.0;
            // Check if this X is unique (within tolerance)
            if !col_xs.iter().any(|&cx| (cx - x).abs() < 2.0) {
                col_xs.push(x);
            }
        }
        col_xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        // Find intersections
        let mut intersections = Vec::new();
        let tolerance = 5.0; // Points tolerance for intersection detection

        for (h_idx, h_line) in &h_lines {
            let h_y = (h_line.start.1 + h_line.end.1) / 2.0;
            let h_min_x = h_line.start.0.min(h_line.end.0);
            let h_max_x = h_line.start.0.max(h_line.end.0);

            for (v_idx, v_line) in &v_lines {
                let v_x = (v_line.start.0 + v_line.end.0) / 2.0;
                let v_min_y = v_line.start.1.min(v_line.end.1);
                let v_max_y = v_line.start.1.max(v_line.end.1);

                // Check if lines actually intersect
                // Vertical line's X must be within horizontal line's X range
                // Horizontal line's Y must be within vertical line's Y range
                if v_x >= h_min_x - tolerance
                    && v_x <= h_max_x + tolerance
                    && h_y >= v_min_y - tolerance
                    && h_y <= v_max_y + tolerance
                {
                    intersections.push(GridIntersection::new(
                        (v_x, h_y),
                        Some(*h_idx),
                        Some(*v_idx),
                    ));
                }
            }
        }

        // Generate cell bounds from consecutive row and column separators
        let mut cell_bounds = Vec::new();
        for i in 0..col_xs.len().saturating_sub(1) {
            for j in 0..row_ys.len().saturating_sub(1) {
                cell_bounds.push((
                    col_xs[i],     // left
                    row_ys[j],     // bottom
                    col_xs[i + 1], // right
                    row_ys[j + 1], // top
                ));
            }
        }

        GridAnalysis {
            intersections,
            row_separators: row_ys,
            column_separators: col_xs,
            cell_bounds,
        }
    }

    /// Check if this page has a grid structure (potential table).
    pub fn has_grid_lines(&self) -> bool {
        let grid = self.analyze_grid_lines();
        grid.is_valid_table()
    }

    /// Detect column alignments in text on this page.
    ///
    /// Analyzes text blocks to find columns with consistent alignment patterns
    /// (left, right, center, or decimal alignment).
    ///
    /// # Arguments
    ///
    /// * `tolerance` - Maximum deviation in points for text to be considered aligned.
    ///   Typical values: 2.0 for strict alignment, 5.0 for looser matching.
    ///
    /// # Returns
    ///
    /// Vector of `AlignedColumn` structs describing detected alignment patterns.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// let columns = page.detect_column_alignments(3.0);
    /// for col in &columns {
    ///     println!("{:?} alignment at x={:.1}, {} lines, confidence: {:.1}%",
    ///         col.alignment, col.x_position, col.line_count(), col.confidence * 100.0);
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn detect_column_alignments(&self, tolerance: f32) -> Vec<AlignedColumn> {
        let blocks = self.extract_text_blocks_with_metrics();
        if blocks.is_empty() {
            return Vec::new();
        }

        // Collect left edges, right edges, and centers of all blocks
        let mut left_edges: Vec<(f32, usize)> = Vec::new();
        let mut right_edges: Vec<(f32, usize)> = Vec::new();
        let mut centers: Vec<(f32, usize)> = Vec::new();

        for (idx, block) in blocks.iter().enumerate() {
            let left = block.bounds.0;
            let right = block.bounds.2;
            let center = (left + right) / 2.0;

            left_edges.push((left, idx));
            right_edges.push((right, idx));
            centers.push((center, idx));
        }

        let mut columns = Vec::new();

        // Find left-aligned columns
        columns.extend(Self::find_aligned_positions(
            &left_edges,
            tolerance,
            AlignmentType::Left,
        ));

        // Find right-aligned columns
        columns.extend(Self::find_aligned_positions(
            &right_edges,
            tolerance,
            AlignmentType::Right,
        ));

        // Find center-aligned columns
        columns.extend(Self::find_aligned_positions(
            &centers,
            tolerance,
            AlignmentType::Center,
        ));

        // Sort by x_position for consistent output
        columns.sort_by(|a, b| {
            a.x_position
                .partial_cmp(&b.x_position)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        columns
    }

    /// Helper function to find aligned positions.
    fn find_aligned_positions(
        edges: &[(f32, usize)],
        tolerance: f32,
        alignment: AlignmentType,
    ) -> Vec<AlignedColumn> {
        use std::collections::HashMap;

        // Group edges by position (within tolerance)
        let mut groups: HashMap<i32, Vec<usize>> = HashMap::new();
        let bucket_size = tolerance.max(1.0);

        for &(pos, idx) in edges {
            let bucket = (pos / bucket_size).round() as i32;
            groups.entry(bucket).or_default().push(idx);
        }

        // Convert groups to AlignedColumn if they have multiple lines
        let total_lines = edges.len();
        groups
            .into_iter()
            .filter(|(_, indices)| indices.len() >= 2) // Need at least 2 aligned items
            .map(|(_bucket, indices)| {
                let avg_pos: f32 = edges
                    .iter()
                    .filter(|(_, idx)| indices.contains(idx))
                    .map(|(pos, _)| pos)
                    .sum::<f32>()
                    / indices.len() as f32;

                let confidence = (indices.len() as f32 / total_lines as f32).min(1.0);

                AlignedColumn::new(avg_pos, alignment, indices, confidence)
            })
            .collect()
    }

    /// Get the number of detected column alignments.
    pub fn column_alignment_count(&self, tolerance: f32) -> usize {
        self.detect_column_alignments(tolerance).len()
    }

    /// Analyze whitespace gaps on this page to detect table-like structures.
    ///
    /// This method identifies consistent horizontal and vertical gaps between
    /// text blocks that could indicate a table layout without visible borders.
    ///
    /// # Arguments
    ///
    /// * `min_gap` - Minimum gap size in points to consider as a separator.
    ///   Typical values: 5.0 for dense text, 10.0 for normal spacing.
    ///
    /// # Returns
    ///
    /// A `GapMatrix` containing horizontal gaps, vertical gaps, and potential
    /// cell count estimates.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("data.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// let gaps = page.analyze_whitespace_gaps(8.0);
    /// if gaps.suggests_table() {
    ///     println!("Detected ~{} rows x {} cols of data",
    ///         gaps.potential_cells.0, gaps.potential_cells.1);
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn analyze_whitespace_gaps(&self, min_gap: f32) -> GapMatrix {
        let blocks = self.extract_text_blocks_with_metrics();
        if blocks.len() < 2 {
            return GapMatrix::new();
        }

        // Get page bounds
        let page_width = self.width() as f32;
        let page_height = self.height() as f32;

        // Collect all block bounds
        let mut block_bounds: Vec<(f32, f32, f32, f32)> = blocks.iter().map(|b| b.bounds).collect();

        // Sort by Y position (bottom to top)
        block_bounds.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        // Find horizontal gaps (between rows of text)
        let mut horizontal_gaps = Vec::new();
        for i in 0..block_bounds.len() - 1 {
            let current = &block_bounds[i];
            let next = &block_bounds[i + 1];

            // Gap between current top and next bottom
            let gap_size = next.1 - current.3;
            if gap_size >= min_gap {
                horizontal_gaps.push(WhitespaceGap::new(
                    (0.0, current.3, page_width, next.1),
                    GapOrientation::Horizontal,
                ));
            }
        }

        // Sort blocks by X position (left to right)
        block_bounds.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        // Find vertical gaps (between columns of text)
        let mut vertical_gaps = Vec::new();
        for i in 0..block_bounds.len() - 1 {
            let current = &block_bounds[i];
            let next = &block_bounds[i + 1];

            // Gap between current right and next left
            let gap_size = next.0 - current.2;
            if gap_size >= min_gap {
                vertical_gaps.push(WhitespaceGap::new(
                    (current.2, 0.0, next.0, page_height),
                    GapOrientation::Vertical,
                ));
            }
        }

        // Estimate potential cells
        let potential_rows = horizontal_gaps.len() + 1;
        let potential_cols = vertical_gaps.len() + 1;

        GapMatrix {
            horizontal_gaps,
            vertical_gaps,
            potential_cells: (potential_rows, potential_cols),
        }
    }

    /// Detect alternating row backgrounds (zebra striping) on this page.
    ///
    /// Looks for colored regions that alternate in a pattern, typically
    /// indicating table rows with visual separation.
    ///
    /// # Returns
    ///
    /// `Some(AlternatingPattern)` if a pattern is detected, `None` otherwise.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("table.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// if let Some(pattern) = page.detect_alternating_backgrounds() {
    ///     if pattern.is_zebra_stripe() {
    ///         println!("Found zebra-striped table with {} rows", pattern.row_count());
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn detect_alternating_backgrounds(&self) -> Option<AlternatingPattern> {
        let regions = self.extract_colored_regions();
        if regions.len() < 4 {
            return None;
        }

        // Filter to regions that span horizontally (potential row backgrounds)
        let page_width = self.width() as f32;
        let horizontal_regions: Vec<_> = regions
            .iter()
            .filter(|r| {
                let width = r.bounds.2 - r.bounds.0;
                width > page_width * 0.5 && r.fill_color.is_some()
            })
            .collect();

        if horizontal_regions.len() < 4 {
            return None;
        }

        // Sort by Y position
        let mut sorted_regions = horizontal_regions;
        sorted_regions.sort_by(|a, b| {
            a.bounds
                .1
                .partial_cmp(&b.bounds.1)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Extract bounds and colors
        let row_bounds: Vec<_> = sorted_regions.iter().map(|r| r.bounds).collect();
        let colors: Vec<_> = sorted_regions.iter().map(|r| r.fill_color).collect();

        let pattern = AlternatingPattern::new(row_bounds, colors);

        if pattern.is_alternating {
            Some(pattern)
        } else {
            None
        }
    }

    /// Detect regions with primarily numeric content.
    ///
    /// Identifies areas of the page containing numbers, currency values,
    /// percentages, etc. Useful for finding data columns in tables.
    ///
    /// # Returns
    ///
    /// Vector of `NumericRegion` structs describing detected numeric areas.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("financial.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// for region in page.detect_numeric_regions() {
    ///     if region.is_financial() {
    ///         println!("Financial data at x={:.1}", region.bounds.0);
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn detect_numeric_regions(&self) -> Vec<NumericRegion> {
        let blocks = self.extract_text_blocks_with_metrics();
        let text = match self.text() {
            Ok(t) => t,
            Err(_) => return Vec::new(),
        };

        let mut regions = Vec::new();

        for block in &blocks {
            // Get text content within this block's bounds
            let block_text = text.text_in_rect(
                block.bounds.0 as f64,
                block.bounds.1 as f64,
                block.bounds.2 as f64,
                block.bounds.3 as f64,
            );

            if block_text.is_empty() {
                continue;
            }

            // Analyze character composition
            let total_chars = block_text.chars().filter(|c| !c.is_whitespace()).count();
            if total_chars == 0 {
                continue;
            }

            let numeric_chars = block_text.chars().filter(|c| c.is_ascii_digit()).count();

            let has_decimals = block_text.contains('.') || block_text.contains(',');
            let has_currency = block_text
                .chars()
                .any(|c| matches!(c, '$' | '€' | '£' | '¥' | '₹' | '₽' | '₩' | '฿'));
            let has_percentages = block_text.contains('%');

            let numeric_ratio = numeric_chars as f32 / total_chars as f32;

            // Only include regions with significant numeric content
            if numeric_ratio > 0.3 || has_currency || has_percentages {
                // Determine alignment based on text block position
                let alignment = if has_currency || has_decimals {
                    AlignmentType::Right
                } else {
                    AlignmentType::Left
                };

                regions.push(NumericRegion::new(
                    block.bounds,
                    numeric_ratio,
                    has_decimals,
                    has_currency,
                    has_percentages,
                    alignment,
                ));
            }
        }

        regions
    }

    /// Get count of numeric regions detected on this page.
    pub fn numeric_region_count(&self) -> usize {
        self.detect_numeric_regions().len()
    }

    /// Cluster text blocks by proximity into logical groups.
    ///
    /// Groups nearby text blocks that are likely to be related content
    /// (e.g., paragraphs, columns, sidebars).
    ///
    /// # Arguments
    ///
    /// * `gap_threshold` - Minimum gap in points to separate clusters.
    ///   Typical values: 10.0 for tight clustering, 30.0 for loose grouping.
    ///
    /// # Returns
    ///
    /// Vector of `TextCluster` structs representing grouped content.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// for cluster in page.cluster_text_blocks(20.0) {
    ///     println!("Cluster: {} chars, {} lines",
    ///         cluster.char_count, cluster.line_count);
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn cluster_text_blocks(&self, _gap_threshold: f32) -> Vec<TextCluster> {
        let blocks = self.extract_text_blocks_with_metrics();
        if blocks.is_empty() {
            return Vec::new();
        }

        let page_height = self.height() as f32;
        let page_width = self.width() as f32;

        // Start with each block as its own cluster
        let mut clusters: Vec<TextCluster> = blocks
            .iter()
            .map(|b| {
                // Estimate char count from area (rough approximation)
                let area = b.width() * b.height();
                let char_count = (area / 50.0).max(1.0) as usize; // ~50 sq pt per char
                TextCluster::new(b.bounds, char_count, b.line_count)
            })
            .collect();

        // Calculate gaps for each cluster
        for i in 0..clusters.len() {
            let bounds = clusters[i].bounds;

            // Gap above (to page top or nearest block above)
            let mut gap_above = page_height - bounds.3;
            for (j, cluster) in clusters.iter().enumerate() {
                if i != j && cluster.bounds.1 > bounds.3 {
                    let gap = cluster.bounds.1 - bounds.3;
                    if gap < gap_above {
                        gap_above = gap;
                    }
                }
            }

            // Gap below (to page bottom or nearest block below)
            let mut gap_below = bounds.1;
            for (j, cluster) in clusters.iter().enumerate() {
                if i != j && cluster.bounds.3 < bounds.1 {
                    let gap = bounds.1 - cluster.bounds.3;
                    if gap < gap_below {
                        gap_below = gap;
                    }
                }
            }

            // Gap left (to page left or nearest block)
            let mut gap_left = bounds.0;
            for (j, cluster) in clusters.iter().enumerate() {
                if i != j && cluster.bounds.2 < bounds.0 {
                    let gap = bounds.0 - cluster.bounds.2;
                    if gap < gap_left {
                        gap_left = gap;
                    }
                }
            }

            // Gap right (to page right or nearest block)
            let mut gap_right = page_width - bounds.2;
            for (j, cluster) in clusters.iter().enumerate() {
                if i != j && cluster.bounds.0 > bounds.2 {
                    let gap = cluster.bounds.0 - bounds.2;
                    if gap < gap_right {
                        gap_right = gap;
                    }
                }
            }

            clusters[i].gap_above = gap_above;
            clusters[i].gap_below = gap_below;
            clusters[i].gap_left = gap_left;
            clusters[i].gap_right = gap_right;
        }

        clusters
    }

    /// Get count of text clusters on this page.
    pub fn text_cluster_count(&self, gap_threshold: f32) -> usize {
        self.cluster_text_blocks(gap_threshold).len()
    }

    /// Analyze indentation patterns on this page.
    ///
    /// Detects indentation levels used in the document, useful for identifying
    /// lists, nested content, code blocks, and other structured text.
    ///
    /// # Returns
    ///
    /// An `IndentationAnalysis` struct containing detected indentation patterns.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// let indent = page.analyze_indentation();
    /// if indent.has_indentation() {
    ///     println!("Found {} indent levels, {} indented lines",
    ///         indent.max_level, indent.indented_line_count());
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn analyze_indentation(&self) -> IndentationAnalysis {
        let blocks = self.extract_text_blocks_with_metrics();
        if blocks.is_empty() {
            return IndentationAnalysis::new();
        }

        // Collect left positions of all blocks
        let mut left_positions: Vec<f32> = blocks.iter().map(|b| b.bounds.0).collect();
        left_positions.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        // Find base margin (minimum left position)
        let base_margin = *left_positions.first().unwrap_or(&0.0);

        // Collect unique indent positions (within 5pt tolerance)
        let mut indent_levels: Vec<f32> = Vec::new();
        for &pos in &left_positions {
            let relative = pos - base_margin;
            if relative > 5.0 {
                // Only count if different from existing levels
                let is_new = !indent_levels.iter().any(|&l| (l - relative).abs() < 5.0);
                if is_new {
                    indent_levels.push(relative);
                }
            }
        }
        indent_levels.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        // Estimate indent increment
        let indent_increment = if indent_levels.len() >= 2 {
            indent_levels[1] - indent_levels[0]
        } else if !indent_levels.is_empty() {
            indent_levels[0]
        } else {
            20.0 // Default assumption
        };

        // Build indented lines
        let mut lines = Vec::new();
        for (idx, block) in blocks.iter().enumerate() {
            let indent_px = block.bounds.0 - base_margin;
            let indent_level = if indent_px > 5.0 && indent_increment > 0.0 {
                ((indent_px / indent_increment).round() as usize).max(1)
            } else {
                0
            };

            lines.push(IndentedLine::new(
                idx,
                indent_px,
                indent_level,
                block.bounds,
            ));
        }

        let max_level = lines.iter().map(|l| l.indent_level).max().unwrap_or(0);

        IndentationAnalysis {
            base_margin,
            indent_increment,
            lines,
            max_level,
        }
    }

    /// Extract list markers from this page.
    ///
    /// Detects bullet points, numbered lists, and other list markers.
    ///
    /// # Returns
    ///
    /// Vector of `ListMarker` structs describing detected markers.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// for marker in page.extract_list_markers() {
    ///     println!("{:?}: '{}'", marker.marker_type, marker.marker_text);
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn extract_list_markers(&self) -> Vec<ListMarker> {
        let text = match self.text() {
            Ok(t) => t,
            Err(_) => return Vec::new(),
        };

        let chars: Vec<_> = text.chars().collect();
        if chars.is_empty() {
            return Vec::new();
        }

        let mut markers = Vec::new();
        let mut i = 0;

        while i < chars.len() {
            let ch = &chars[i];

            // Check for single-character bullet markers
            if let Some(marker_type) = ListMarkerType::detect(&ch.unicode.to_string()) {
                // Found a potential marker - look for following content
                let marker_bounds = (
                    ch.left as f32,
                    ch.bottom as f32,
                    ch.right as f32,
                    ch.top as f32,
                );

                // Find content start (skip whitespace after marker)
                let mut content_start_x = marker_bounds.2;
                let mut j = i + 1;
                while j < chars.len() && chars[j].unicode.is_whitespace() {
                    j += 1;
                }
                if j < chars.len() {
                    content_start_x = chars[j].left as f32;
                }

                markers.push(ListMarker::new(
                    marker_type,
                    ch.unicode.to_string(),
                    marker_bounds,
                    content_start_x,
                ));
            }

            // Check for multi-character markers (numbers, letters with punctuation)
            // Look for patterns like "1." "a)" "ii." at start of what looks like a line
            if i == 0 || (i > 0 && chars[i - 1].unicode == '\n') {
                // Start of potential line - check for marker pattern
                let mut potential_marker = String::new();
                let mut j = i;
                let mut end_j = i;

                // Collect potential marker text (up to 5 chars or until space)
                while j < chars.len() && j - i < 5 {
                    let c = chars[j].unicode;
                    if c.is_whitespace() {
                        break;
                    }
                    potential_marker.push(c);
                    end_j = j;
                    j += 1;
                }

                if potential_marker.len() >= 2 {
                    if let Some(marker_type) = ListMarkerType::detect(&potential_marker) {
                        let marker_bounds = (
                            chars[i].left as f32,
                            chars[i].bottom as f32,
                            chars[end_j].right as f32,
                            chars[i].top as f32,
                        );

                        // Find content start
                        let mut content_start_x = marker_bounds.2;
                        while j < chars.len() && chars[j].unicode.is_whitespace() {
                            j += 1;
                        }
                        if j < chars.len() {
                            content_start_x = chars[j].left as f32;
                        }

                        markers.push(ListMarker::new(
                            marker_type,
                            potential_marker,
                            marker_bounds,
                            content_start_x,
                        ));
                    }
                }
            }

            i += 1;
        }

        markers
    }

    /// Get count of list markers on this page.
    pub fn list_marker_count(&self) -> usize {
        self.extract_list_markers().len()
    }

    /// Detect column gutters to analyze multi-column layout.
    ///
    /// Analyzes vertical whitespace patterns to identify column separations.
    ///
    /// # Returns
    ///
    /// A `ColumnLayout` struct describing detected columns and gutters.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// let layout = page.detect_column_gutters();
    /// if layout.is_multi_column() {
    ///     println!("Found {} columns", layout.column_count);
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn detect_column_gutters(&self) -> ColumnLayout {
        let page_width = self.width() as f32;
        let page_height = self.height() as f32;

        let blocks = self.extract_text_blocks_with_metrics();
        if blocks.is_empty() {
            return ColumnLayout::new();
        }

        // Analyze vertical gaps to find potential gutters
        // Group blocks by their x-position ranges
        let mut left_edges: Vec<f32> = blocks.iter().map(|b| b.bounds.0).collect();
        let mut right_edges: Vec<f32> = blocks.iter().map(|b| b.bounds.2).collect();
        left_edges.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        right_edges.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        // Look for vertical gaps that span most of the page height
        let min_gutter_width = 20.0; // Minimum 20pt gutter
        let min_gutter_height_ratio = 0.5; // Must span at least 50% of page
        let min_gutter_height = page_height * min_gutter_height_ratio;

        let mut gutters = Vec::new();

        // Check for gaps between text blocks
        // Sort blocks by x-position
        let mut sorted_blocks: Vec<_> = blocks.iter().collect();
        sorted_blocks.sort_by(|a, b| {
            a.bounds
                .0
                .partial_cmp(&b.bounds.0)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Find vertical gaps between content regions
        for i in 0..sorted_blocks.len() {
            let right_of_current = sorted_blocks[i].bounds.2;

            // Check gap to next block(s)
            for block in sorted_blocks.iter().skip(i + 1) {
                let left_of_next = block.bounds.0;
                let gap = left_of_next - right_of_current;

                if gap >= min_gutter_width {
                    // Check if this gap spans enough vertical space
                    // Find vertical extent of blocks on each side
                    let mut left_min_y = page_height;
                    let mut left_max_y = 0.0f32;
                    let mut right_min_y = page_height;
                    let mut right_max_y = 0.0f32;

                    for b in &blocks {
                        if b.bounds.2 <= right_of_current + 5.0 {
                            left_min_y = left_min_y.min(b.bounds.1);
                            left_max_y = left_max_y.max(b.bounds.3);
                        }
                        if b.bounds.0 >= left_of_next - 5.0 {
                            right_min_y = right_min_y.min(b.bounds.1);
                            right_max_y = right_max_y.max(b.bounds.3);
                        }
                    }

                    let combined_min = left_min_y.min(right_min_y);
                    let combined_max = left_max_y.max(right_max_y);
                    let span = combined_max - combined_min;

                    if span >= min_gutter_height {
                        let x_pos = (right_of_current + left_of_next) / 2.0;
                        let confidence = (span / page_height).min(1.0) * (gap / 50.0).min(1.0);

                        // Check if we already have a gutter near this position
                        let already_exists = gutters.iter().any(|g: &ColumnGutter| {
                            (g.x_position - x_pos).abs() < min_gutter_width
                        });

                        if !already_exists {
                            gutters.push(ColumnGutter::new(
                                x_pos,
                                combined_max,
                                combined_min,
                                gap,
                                confidence,
                            ));
                        }
                    }
                }
            }
        }

        // Sort gutters by x-position
        gutters.sort_by(|a, b| {
            a.x_position
                .partial_cmp(&b.x_position)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Calculate column bounds
        let column_count = gutters.len() + 1;
        let mut column_bounds = Vec::new();

        let page_left = 0.0;
        let page_bottom = 0.0;
        let page_top = page_height;

        if gutters.is_empty() {
            // Single column - use full page width
            column_bounds.push((page_left, page_bottom, page_width, page_top));
        } else {
            // Multiple columns - split at gutters
            let mut prev_x = page_left;
            for gutter in &gutters {
                let gutter_left = gutter.x_position - gutter.width / 2.0;
                column_bounds.push((prev_x, page_bottom, gutter_left, page_top));
                prev_x = gutter.x_position + gutter.width / 2.0;
            }
            // Last column
            column_bounds.push((prev_x, page_bottom, page_width, page_top));
        }

        ColumnLayout {
            gutters,
            column_count,
            column_bounds,
        }
    }

    /// Compute a content density heatmap for this page.
    ///
    /// Divides the page into a grid and computes content density for each cell.
    ///
    /// # Arguments
    ///
    /// * `rows` - Number of rows in the grid (minimum 1)
    /// * `cols` - Number of columns in the grid (minimum 1)
    ///
    /// # Returns
    ///
    /// A `DensityMap` with content density analysis for each cell.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// let map = page.compute_density_map(10, 10);
    /// println!("Average text density: {:.1}%", map.average_text_density() * 100.0);
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn compute_density_map(&self, rows: usize, cols: usize) -> DensityMap {
        let rows = rows.max(1);
        let cols = cols.max(1);

        let page_width = self.width() as f32;
        let page_height = self.height() as f32;

        let cell_width = page_width / cols as f32;
        let cell_height = page_height / rows as f32;

        // Get text blocks for text density
        let text_blocks = self.extract_text_blocks_with_metrics();

        // Get page objects for image/line density
        let objects = self.objects();
        let image_bounds: Vec<(f32, f32, f32, f32)> = objects
            .iter()
            .filter(|o| o.is_image())
            .filter_map(|o| o.bounds())
            .map(|b| (b.left, b.bottom, b.right, b.top))
            .collect();

        let line_bounds: Vec<(f32, f32, f32, f32)> = objects
            .iter()
            .filter(|o| o.is_path())
            .filter_map(|o| o.bounds())
            .map(|b| (b.left, b.bottom, b.right, b.top))
            .collect();

        let mut cells = Vec::new();

        for row in 0..rows {
            let mut row_cells = Vec::new();
            for col in 0..cols {
                let left = col as f32 * cell_width;
                let bottom = row as f32 * cell_height;
                let right = left + cell_width;
                let top = bottom + cell_height;
                let cell_area = cell_width * cell_height;

                // Calculate text overlap
                let mut text_area = 0.0f32;
                for block in &text_blocks {
                    let overlap = Self::rect_overlap((left, bottom, right, top), block.bounds);
                    text_area += overlap;
                }
                let text_density = (text_area / cell_area).min(1.0);

                // Calculate image overlap
                let mut image_area = 0.0f32;
                for bounds in &image_bounds {
                    let overlap = Self::rect_overlap((left, bottom, right, top), *bounds);
                    image_area += overlap;
                }
                let image_coverage = (image_area / cell_area).min(1.0);

                // Calculate line overlap
                let mut line_area = 0.0f32;
                for bounds in &line_bounds {
                    let overlap = Self::rect_overlap((left, bottom, right, top), *bounds);
                    line_area += overlap;
                }
                let line_coverage = (line_area / cell_area).min(1.0);

                row_cells.push(DensityCell::new(
                    (left, bottom, right, top),
                    text_density,
                    image_coverage,
                    line_coverage,
                ));
            }
            cells.push(row_cells);
        }

        DensityMap {
            grid_size: (rows, cols),
            cells,
        }
    }

    /// Calculate the overlap area between two rectangles.
    fn rect_overlap(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> f32 {
        let (a_left, a_bottom, a_right, a_top) = a;
        let (b_left, b_bottom, b_right, b_top) = b;

        let overlap_left = a_left.max(b_left);
        let overlap_bottom = a_bottom.max(b_bottom);
        let overlap_right = a_right.min(b_right);
        let overlap_top = a_top.min(b_top);

        if overlap_right > overlap_left && overlap_top > overlap_bottom {
            (overlap_right - overlap_left) * (overlap_top - overlap_bottom)
        } else {
            0.0
        }
    }

    // ========================================================================
    // Page Boxes (MediaBox, CropBox, BleedBox, TrimBox, ArtBox)
    // ========================================================================

    /// Get the MediaBox of this page.
    ///
    /// The MediaBox defines the physical medium size (paper dimensions).
    /// All other page boxes default to MediaBox if not explicitly set.
    ///
    /// # Returns
    ///
    /// The MediaBox if defined, or None if not set.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// if let Some(media_box) = page.media_box() {
    ///     println!("Paper size: {:.0}x{:.0} points", media_box.width(), media_box.height());
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn media_box(&self) -> Option<PdfPageBox> {
        let mut left = 0.0f32;
        let mut bottom = 0.0f32;
        let mut right = 0.0f32;
        let mut top = 0.0f32;
        let result = unsafe {
            FPDFPage_GetMediaBox(self.handle, &mut left, &mut bottom, &mut right, &mut top)
        };
        if result != 0 {
            Some(PdfPageBox::new(left, bottom, right, top))
        } else {
            None
        }
    }

    /// Set the MediaBox of this page.
    ///
    /// # Arguments
    ///
    /// * `page_box` - The new MediaBox dimensions
    pub fn set_media_box(&mut self, page_box: &PdfPageBox) {
        unsafe {
            FPDFPage_SetMediaBox(
                self.handle,
                page_box.left,
                page_box.bottom,
                page_box.right,
                page_box.top,
            );
        }
    }

    /// Get the CropBox of this page.
    ///
    /// The CropBox defines the region to which page contents should be clipped
    /// during display or printing. Defaults to MediaBox if not set.
    pub fn crop_box(&self) -> Option<PdfPageBox> {
        let mut left = 0.0f32;
        let mut bottom = 0.0f32;
        let mut right = 0.0f32;
        let mut top = 0.0f32;
        let result = unsafe {
            FPDFPage_GetCropBox(self.handle, &mut left, &mut bottom, &mut right, &mut top)
        };
        if result != 0 {
            Some(PdfPageBox::new(left, bottom, right, top))
        } else {
            None
        }
    }

    /// Set the CropBox of this page.
    pub fn set_crop_box(&mut self, page_box: &PdfPageBox) {
        unsafe {
            FPDFPage_SetCropBox(
                self.handle,
                page_box.left,
                page_box.bottom,
                page_box.right,
                page_box.top,
            );
        }
    }

    /// Get the BleedBox of this page.
    ///
    /// The BleedBox defines the region for production printing bleed area.
    /// Defaults to CropBox if not set.
    pub fn bleed_box(&self) -> Option<PdfPageBox> {
        let mut left = 0.0f32;
        let mut bottom = 0.0f32;
        let mut right = 0.0f32;
        let mut top = 0.0f32;
        let result = unsafe {
            FPDFPage_GetBleedBox(self.handle, &mut left, &mut bottom, &mut right, &mut top)
        };
        if result != 0 {
            Some(PdfPageBox::new(left, bottom, right, top))
        } else {
            None
        }
    }

    /// Set the BleedBox of this page.
    pub fn set_bleed_box(&mut self, page_box: &PdfPageBox) {
        unsafe {
            FPDFPage_SetBleedBox(
                self.handle,
                page_box.left,
                page_box.bottom,
                page_box.right,
                page_box.top,
            );
        }
    }

    /// Get the TrimBox of this page.
    ///
    /// The TrimBox defines the intended finished page dimensions after trimming.
    /// Defaults to CropBox if not set.
    pub fn trim_box(&self) -> Option<PdfPageBox> {
        let mut left = 0.0f32;
        let mut bottom = 0.0f32;
        let mut right = 0.0f32;
        let mut top = 0.0f32;
        let result = unsafe {
            FPDFPage_GetTrimBox(self.handle, &mut left, &mut bottom, &mut right, &mut top)
        };
        if result != 0 {
            Some(PdfPageBox::new(left, bottom, right, top))
        } else {
            None
        }
    }

    /// Set the TrimBox of this page.
    pub fn set_trim_box(&mut self, page_box: &PdfPageBox) {
        unsafe {
            FPDFPage_SetTrimBox(
                self.handle,
                page_box.left,
                page_box.bottom,
                page_box.right,
                page_box.top,
            );
        }
    }

    /// Get the ArtBox of this page.
    ///
    /// The ArtBox defines the meaningful content boundaries.
    /// Defaults to CropBox if not set.
    pub fn art_box(&self) -> Option<PdfPageBox> {
        let mut left = 0.0f32;
        let mut bottom = 0.0f32;
        let mut right = 0.0f32;
        let mut top = 0.0f32;
        let result = unsafe {
            FPDFPage_GetArtBox(self.handle, &mut left, &mut bottom, &mut right, &mut top)
        };
        if result != 0 {
            Some(PdfPageBox::new(left, bottom, right, top))
        } else {
            None
        }
    }

    /// Set the ArtBox of this page.
    pub fn set_art_box(&mut self, page_box: &PdfPageBox) {
        unsafe {
            FPDFPage_SetArtBox(
                self.handle,
                page_box.left,
                page_box.bottom,
                page_box.right,
                page_box.top,
            );
        }
    }

    /// Get the bounding box of all page content.
    ///
    /// Returns the smallest rectangle that encloses all page objects.
    pub fn bounding_box(&self) -> Option<PdfPageBox> {
        let mut rect = FS_RECTF {
            left: 0.0,
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
        };
        let result = unsafe { FPDF_GetPageBoundingBox(self.handle, &mut rect) };
        if result != 0 {
            // Note: FS_RECTF uses (left, top, right, bottom), PdfPageBox uses (left, bottom, right, top)
            Some(PdfPageBox::new(
                rect.left,
                rect.bottom,
                rect.right,
                rect.top,
            ))
        } else {
            None
        }
    }

    // ========================================================================
    // Page Transforms
    // ========================================================================

    /// Apply a transformation matrix and/or clipping rectangle to the page.
    ///
    /// This modifies all page content by applying the transformation.
    ///
    /// # Arguments
    ///
    /// * `matrix` - Optional transformation matrix (scale, rotate, translate)
    /// * `clip` - Optional clipping rectangle
    ///
    /// # Returns
    ///
    /// `true` if the transform was applied successfully.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::{Pdfium, PdfMatrix, PdfClipRect};
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let mut page = doc.page(0)?;
    ///
    /// // Scale page to 50%
    /// let matrix = PdfMatrix::scale(0.5);
    /// page.transform(Some(&matrix), None);
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn transform(&mut self, matrix: Option<&PdfMatrix>, clip: Option<&PdfClipRect>) -> bool {
        if matrix.is_none() && clip.is_none() {
            return false;
        }

        let fs_matrix = matrix.map(|m| FS_MATRIX {
            a: m.a,
            b: m.b,
            c: m.c,
            d: m.d,
            e: m.e,
            f: m.f,
        });

        let fs_clip = clip.map(|c| FS_RECTF {
            left: c.left,
            top: c.top,
            right: c.right,
            bottom: c.bottom,
        });

        let matrix_ptr = fs_matrix
            .as_ref()
            .map(|m| m as *const FS_MATRIX)
            .unwrap_or(std::ptr::null());
        let clip_ptr = fs_clip
            .as_ref()
            .map(|c| c as *const FS_RECTF)
            .unwrap_or(std::ptr::null());

        unsafe { FPDFPage_TransFormWithClip(self.handle, matrix_ptr, clip_ptr) != 0 }
    }

    // ========================================================================
    // Thumbnails
    // ========================================================================

    /// Check if this page has an embedded thumbnail.
    pub fn has_thumbnail(&self) -> bool {
        unsafe { FPDFPage_GetDecodedThumbnailData(self.handle, std::ptr::null_mut(), 0) > 0 }
    }

    /// Get the decoded thumbnail data for this page.
    ///
    /// Returns the thumbnail image data in its decoded form (typically bitmap data).
    /// Returns None if no thumbnail exists.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// if let Some(data) = page.thumbnail_data() {
    ///     println!("Thumbnail: {} bytes", data.len());
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn thumbnail_data(&self) -> Option<Vec<u8>> {
        let size =
            unsafe { FPDFPage_GetDecodedThumbnailData(self.handle, std::ptr::null_mut(), 0) };
        if size == 0 {
            return None;
        }

        let mut buffer = vec![0u8; size as usize];
        let actual = unsafe {
            FPDFPage_GetDecodedThumbnailData(self.handle, buffer.as_mut_ptr() as *mut _, size)
        };
        if actual == 0 {
            return None;
        }

        buffer.truncate(actual as usize);
        Some(buffer)
    }

    /// Get the raw (compressed) thumbnail data for this page.
    ///
    /// Returns the original thumbnail data as stored in the PDF.
    /// This may be compressed (e.g., JPEG, PNG) unlike `thumbnail_data()`.
    pub fn raw_thumbnail_data(&self) -> Option<Vec<u8>> {
        let size = unsafe { FPDFPage_GetRawThumbnailData(self.handle, std::ptr::null_mut(), 0) };
        if size == 0 {
            return None;
        }

        let mut buffer = vec![0u8; size as usize];
        let actual = unsafe {
            FPDFPage_GetRawThumbnailData(self.handle, buffer.as_mut_ptr() as *mut _, size)
        };
        if actual == 0 {
            return None;
        }

        buffer.truncate(actual as usize);
        Some(buffer)
    }

    /// Get the thumbnail as a bitmap.
    ///
    /// Returns a bitmap object that can be saved as PNG/JPEG.
    /// Returns None if no thumbnail exists.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// if let Some(thumb) = page.thumbnail_bitmap() {
    ///     thumb.save_as_png("thumbnail.png")?;
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn thumbnail_bitmap(&self) -> Option<PdfBitmap> {
        let bitmap = unsafe { FPDFPage_GetThumbnailAsBitmap(self.handle) };
        if bitmap.is_null() {
            return None;
        }

        let width = unsafe { FPDFBitmap_GetWidth(bitmap) } as u32;
        let height = unsafe { FPDFBitmap_GetHeight(bitmap) } as u32;
        let format_code = unsafe { FPDFBitmap_GetFormat(bitmap) };

        // Use if/else to avoid clippy warnings about non-uppercase constant patterns
        let format_u32 = format_code as u32;
        let format = if format_u32 == FPDFBitmap_Gray {
            PixelFormat::Gray
        } else if format_u32 == FPDFBitmap_BGR {
            PixelFormat::Bgr
        } else {
            // BGRx, BGRA, and unknown formats all treated as BGRA
            PixelFormat::Bgra
        };

        Some(PdfBitmap::new(bitmap, width, height, format))
    }

    // ========================================================================
    // Structure Tree (Accessibility)
    // ========================================================================

    /// Get the structure tree for this page.
    ///
    /// The structure tree provides accessibility information about the page's
    /// logical structure (headings, paragraphs, tables, figures, etc.).
    ///
    /// Returns None if the page has no structure tree (untagged PDF).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// if let Some(tree) = page.structure_tree() {
    ///     println!("Page has {} structure elements", tree.child_count());
    ///     for elem in tree.children() {
    ///         if let Some(elem_type) = elem.element_type() {
    ///             println!("  - {} element", elem_type);
    ///         }
    ///     }
    /// } else {
    ///     println!("Page is not tagged (no structure tree)");
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn structure_tree(&self) -> Option<PdfStructTree> {
        let tree = unsafe { FPDF_StructTree_GetForPage(self.handle) };
        PdfStructTree::from_handle(tree)
    }

    /// Check if this page has a structure tree.
    ///
    /// Tagged PDFs have structure trees that provide accessibility information.
    pub fn has_structure_tree(&self) -> bool {
        let tree = unsafe { FPDF_StructTree_GetForPage(self.handle) };
        if tree.is_null() {
            false
        } else {
            // Close the tree since we're just checking
            unsafe { FPDF_StructTree_Close(tree) };
            true
        }
    }

    // ========================================================================
    // Page Flatten (Annotations → Static Content)
    // ========================================================================

    /// Flatten annotations and form fields into the page content.
    ///
    /// This converts interactive elements (annotations, form fields) into
    /// static page content that cannot be edited. This is useful for:
    /// - Archiving filled forms
    /// - Printing forms with filled values
    /// - Removing interactivity for distribution
    ///
    /// # Arguments
    ///
    /// * `mode` - Flatten mode (Display or Print)
    ///
    /// # Returns
    ///
    /// The result of the flatten operation:
    /// - `FlattenResult::Success` - Annotations were flattened
    /// - `FlattenResult::NothingToDo` - No annotations/forms to flatten
    /// - `FlattenResult::Fail` - Operation failed
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::{Pdfium, FlattenMode, FlattenResult};
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("form.pdf", None)?;
    ///
    /// // Flatten all pages
    /// for page in doc.pages() {
    ///     match page.flatten(FlattenMode::Display) {
    ///         FlattenResult::Success => println!("Page {} flattened", page.index()),
    ///         FlattenResult::NothingToDo => println!("Page {} has nothing to flatten", page.index()),
    ///         FlattenResult::Fail => eprintln!("Failed to flatten page {}", page.index()),
    ///     }
    /// }
    ///
    /// // Save flattened document
    /// doc.save_to_file("form_flattened.pdf", None)?;
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn flatten(&self, mode: crate::document::FlattenMode) -> crate::document::FlattenResult {
        let result = unsafe { FPDFPage_Flatten(self.handle, mode.to_raw()) };
        crate::document::FlattenResult::from_raw(result)
    }

    /// Flatten annotations for display (all visible annotations).
    ///
    /// Convenience method equivalent to `flatten(FlattenMode::Display)`.
    pub fn flatten_for_display(&self) -> crate::document::FlattenResult {
        self.flatten(crate::document::FlattenMode::Display)
    }

    /// Flatten annotations for print (only print-visible annotations).
    ///
    /// Convenience method equivalent to `flatten(FlattenMode::Print)`.
    pub fn flatten_for_print(&self) -> crate::document::FlattenResult {
        self.flatten(crate::document::FlattenMode::Print)
    }

    // ========================================================================
    // Page Object Insertion/Removal
    // ========================================================================

    /// Insert a page object into this page.
    ///
    /// The object is added at the end of the page's content stream.
    /// After inserting objects, call `generate_content()` to update the page.
    ///
    /// # Arguments
    ///
    /// * `object` - The page object to insert (consumes ownership)
    ///
    /// # Returns
    ///
    /// `Ok(())` on success.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::{Pdfium, StandardFont};
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.create_new_document()?;
    ///
    /// // Create a text object
    /// let font = doc.load_standard_font(StandardFont::Helvetica)?;
    /// let mut text = doc.create_text_object(&font, 12.0)?;
    /// text.set_text("Hello, World!")?;
    /// text.transform(1.0, 0.0, 0.0, 1.0, 72.0, 720.0);
    ///
    /// // Insert into page
    /// let mut page = doc.new_page(0, 612.0, 792.0)?;
    /// page.insert_object(text)?;
    /// page.generate_content()?;
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn insert_object(&mut self, object: crate::document::PdfNewPageObject) -> Result<()> {
        let handle = object.into_handle();
        unsafe {
            FPDFPage_InsertObject(self.handle, handle);
        }
        Ok(())
    }

    /// Insert a page object at a specific index.
    ///
    /// Objects are rendered in order, so lower indices appear behind higher ones.
    ///
    /// # Arguments
    ///
    /// * `object` - The page object to insert
    /// * `index` - Position in the object list (0 = first/back-most)
    ///
    /// # Returns
    ///
    /// `Ok(())` on success.
    pub fn insert_object_at_index(
        &mut self,
        object: crate::document::PdfNewPageObject,
        index: usize,
    ) -> Result<()> {
        let handle = object.into_handle();
        unsafe {
            FPDFPage_InsertObjectAtIndex(self.handle, handle, index);
        }
        Ok(())
    }

    /// Remove a page object by index.
    ///
    /// After removing objects, call `generate_content()` to update the page.
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the object to remove
    ///
    /// # Returns
    ///
    /// `true` if the object was removed, `false` if the index was invalid.
    pub fn remove_object(&mut self, index: usize) -> bool {
        let object = unsafe { FPDFPage_GetObject(self.handle, index as i32) };
        if object.is_null() {
            return false;
        }
        let result = unsafe { FPDFPage_RemoveObject(self.handle, object) };
        if result != 0 {
            // Object was removed from page but we still own it - destroy it
            unsafe {
                FPDFPageObj_Destroy(object);
            }
            true
        } else {
            false
        }
    }

    // ========================================================================
    // Content Generation
    // ========================================================================

    /// Generate the page content stream.
    ///
    /// This must be called after inserting or removing page objects for the
    /// changes to take effect. The content stream is the internal PDF data
    /// that describes what to draw on the page.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or an error if content generation fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.create_new_document()?;
    ///
    /// // Create a page and add some content
    /// let mut page = doc.new_page(0, 612.0, 792.0)?;
    ///
    /// // Add objects...
    ///
    /// // Generate the content stream
    /// page.generate_content()?;
    ///
    /// // Now save the document
    /// doc.save_to_file("output.pdf", None)?;
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn generate_content(&mut self) -> Result<()> {
        let success = unsafe { FPDFPage_GenerateContent(self.handle) };
        if success == 0 {
            return Err(PdfError::InvalidInput {
                message: "Failed to generate page content".to_string(),
            });
        }
        Ok(())
    }

    /// Delete the page from the document.
    ///
    /// **Warning**: After calling this, the page handle becomes invalid.
    /// Do not use this page object after deletion.
    ///
    /// # Arguments
    ///
    /// * `preserve_handle` - If true, only marks for deletion (page still renders)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    ///
    /// // Delete page 3 (index 2)
    /// if doc.page_count() > 2 {
    ///     doc.delete_page(2);
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn delete_from_document(&self) {
        unsafe {
            FPDFPage_Delete(self.doc_inner.handle, self.index as i32);
        }
    }

    /// Insert a clip path into this page.
    ///
    /// The clip path will constrain rendering to the defined region.
    /// All subsequent content on the page will be clipped to this path.
    ///
    /// # Arguments
    ///
    /// * `clip_path` - The clip path to insert
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::{Pdfium, PdfClipPath};
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// // Create a rectangular clip path and insert it
    /// let clip = PdfClipPath::new_rect(100.0, 100.0, 400.0, 500.0)?;
    /// page.insert_clip_path(&clip);
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn insert_clip_path(&self, clip_path: &crate::document::PdfClipPath) {
        unsafe {
            FPDFPage_InsertClipPath(self.handle, clip_path.handle());
        }
    }

    /// Transform the page with a transformation matrix and clip rectangle.
    ///
    /// Applies a transformation matrix and optionally clips the result to
    /// a rectangular region. This is useful for resizing, rotating, or
    /// repositioning page content.
    ///
    /// # Arguments
    ///
    /// * `matrix` - The transformation matrix (a, b, c, d, e, f)
    /// * `clip_rect` - Optional clip rectangle (left, bottom, right, top)
    ///
    /// # Returns
    ///
    /// `true` if the transformation was successful, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// // Scale page content by 50% with a clip rectangle
    /// let matrix = (0.5, 0.0, 0.0, 0.5, 0.0, 0.0);
    /// let clip = Some((0.0, 0.0, 300.0, 400.0));
    /// page.transform_with_clip(matrix, clip);
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn transform_with_clip(
        &self,
        matrix: (f32, f32, f32, f32, f32, f32),
        clip_rect: Option<(f32, f32, f32, f32)>,
    ) -> bool {
        let fs_matrix = FS_MATRIX {
            a: matrix.0,
            b: matrix.1,
            c: matrix.2,
            d: matrix.3,
            e: matrix.4,
            f: matrix.5,
        };

        let clip = clip_rect.map(|(l, b, r, t)| FS_RECTF {
            left: l,
            bottom: b,
            right: r,
            top: t,
        });

        unsafe {
            FPDFPage_TransFormWithClip(
                self.handle,
                &fs_matrix,
                clip.as_ref().map_or(std::ptr::null(), |c| c as *const _),
            ) != 0
        }
    }
}

impl Drop for PdfPage {
    fn drop(&mut self) {
        unsafe {
            let form_handle = self.doc_inner.form_handle;
            if !form_handle.is_null() {
                FORM_DoPageAAction(self.handle, form_handle, FPDFPAGE_AACTION_CLOSE as i32);
                FORM_OnBeforeClosePage(self.handle, form_handle);
            }
            FPDF_ClosePage(self.handle);
        }
    }
}
