//! Docling integration features for intelligent PDF extraction.
//!
//! This module provides high-level APIs designed to accelerate document
//! understanding by reducing ML inference requirements.
//!
//! # Overview
//!
//! These features are designed to complement docling's ML pipeline:
//!
//! - **Reading Order**: Logical text sequence without ML reconstruction
//! - **Font Clusters**: Semantic classification of text by typography
//! - **Layout Regions**: Column and block detection via geometry
//! - **Image Hints**: Content type classification to route processing
//! - **Document Classification**: Early routing to optimal pipeline
//!
//! # Example
//!
//! ```no_run
//! use pdfium_render_fast::{Pdfium, DoclingClassification};
//!
//! let pdfium = Pdfium::new()?;
//! let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
//!
//! // Get document classification for pipeline routing
//! let classification = DoclingClassification::analyze(&doc);
//! if classification.is_scanned {
//!     // Use OCR pipeline
//! } else if classification.is_tagged {
//!     // Use structure tree extraction (skip ML)
//! } else {
//!     // Use standard ML pipeline
//! }
//! # Ok::<(), pdfium_render_fast::PdfError>(())
//! ```

use crate::{PdfChar, PdfDocument, PdfFormType, PdfPage, Result};
use std::collections::HashMap;

// ============================================================================
// Reading Order
// ============================================================================

/// A text segment in reading order.
#[derive(Debug, Clone)]
pub struct ReadingOrderSegment {
    /// The text content
    pub text: String,
    /// Bounding box (left, top, right, bottom) in page coordinates
    pub bounds: (f32, f32, f32, f32),
    /// Semantic role if known (from structure tree)
    pub role: Option<String>,
    /// Order in reading sequence (0-based)
    pub order: usize,
}

/// Extract text in reading order from a page.
///
/// This function analyzes character positions to determine logical reading order,
/// grouping characters into lines and ordering them top-to-bottom, left-to-right.
///
/// # Arguments
///
/// * `page` - The PDF page to extract from
///
/// # Returns
///
/// A vector of text segments in reading order.
pub fn extract_reading_order(page: &PdfPage) -> Result<Vec<ReadingOrderSegment>> {
    let text = page.text()?;
    let chars: Vec<PdfChar> = text.chars().collect();

    if chars.is_empty() {
        return Ok(Vec::new());
    }

    // Group characters into lines based on vertical position
    let lines = group_into_lines(&chars);

    // Sort lines by vertical position (top to bottom)
    let mut sorted_lines: Vec<_> = lines.into_iter().collect();
    sorted_lines.sort_by(|a, b| {
        // Compare by top coordinate (descending - PDF coordinates have Y=0 at bottom)
        b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
    });

    // Build reading order segments
    let mut segments = Vec::new();
    for (order, (line_chars, _top)) in sorted_lines.into_iter().enumerate() {
        if line_chars.is_empty() {
            continue;
        }

        // Sort characters left to right within line
        let mut sorted_chars = line_chars;
        sorted_chars.sort_by(|a, b| {
            a.left
                .partial_cmp(&b.left)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Build text and bounds
        let text: String = sorted_chars.iter().map(|c| c.unicode).collect();
        let left = sorted_chars
            .iter()
            .map(|c| c.left as f32)
            .fold(f32::MAX, f32::min);
        let right = sorted_chars
            .iter()
            .map(|c| c.right as f32)
            .fold(f32::MIN, f32::max);
        let top = sorted_chars
            .iter()
            .map(|c| c.top as f32)
            .fold(f32::MIN, f32::max);
        let bottom = sorted_chars
            .iter()
            .map(|c| c.bottom as f32)
            .fold(f32::MAX, f32::min);

        segments.push(ReadingOrderSegment {
            text,
            bounds: (left, top, right, bottom),
            role: None,
            order,
        });
    }

    Ok(segments)
}

/// Group characters into lines based on vertical position
fn group_into_lines(chars: &[PdfChar]) -> Vec<(Vec<PdfChar>, f64)> {
    let mut lines: Vec<(Vec<PdfChar>, f64)> = Vec::new();
    let tolerance = 3.0; // Vertical tolerance for same line

    for ch in chars {
        let char_mid_y = (ch.top + ch.bottom) / 2.0;

        // Find existing line with similar y position
        let existing_line = lines
            .iter_mut()
            .find(|(_, line_y)| (char_mid_y - *line_y).abs() < tolerance);

        if let Some((line_chars, _)) = existing_line {
            line_chars.push(ch.clone());
        } else {
            lines.push((vec![ch.clone()], char_mid_y));
        }
    }

    lines
}

// ============================================================================
// Font Analysis
// ============================================================================

/// Semantic role inferred from font characteristics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontSemanticRole {
    /// Document or section title (typically largest font)
    Title,
    /// Section headers (second largest font)
    SectionHeader,
    /// Main body text (most common font size)
    Body,
    /// Footnotes or captions (smaller than body)
    Footnote,
    /// Code or monospace text
    Code,
    /// Could not determine role
    Unknown,
}

/// A cluster of text with similar font characteristics.
#[derive(Debug, Clone)]
pub struct FontCluster {
    /// Inferred semantic role
    pub role: FontSemanticRole,
    /// Font size in points
    pub font_size: f32,
    /// Whether this appears to be monospace font
    pub is_monospace: bool,
    /// Number of characters with this font
    pub char_count: usize,
    /// Percentage of page covered (0.0 to 1.0)
    pub coverage: f32,
}

/// Analyze font usage patterns to classify text semantically.
///
/// This function clusters text by font size and characteristics,
/// then assigns semantic roles based on relative sizes and frequency.
pub fn analyze_font_clusters(page: &PdfPage) -> Result<Vec<FontCluster>> {
    let text = page.text()?;
    let chars: Vec<PdfChar> = text.chars().collect();

    if chars.is_empty() {
        return Ok(Vec::new());
    }

    // Group characters by font size (rounded to nearest 0.5pt)
    let mut size_groups: HashMap<i32, Vec<&PdfChar>> = HashMap::new();
    for ch in &chars {
        let size_key = (ch.font_size * 2.0).round() as i32; // 0.5pt buckets
        size_groups.entry(size_key).or_default().push(ch);
    }

    // Find most common font size (body text)
    let body_size = size_groups
        .iter()
        .max_by_key(|(_, chars)| chars.len())
        .map(|(size, _)| *size)
        .unwrap_or(24); // 12pt default

    let total_chars = chars.len();
    let mut clusters = Vec::new();

    for (size_key, group_chars) in size_groups {
        let font_size = size_key as f32 / 2.0;
        let char_count = group_chars.len();
        let coverage = char_count as f32 / total_chars as f32;

        // Check for monospace (uniform character widths)
        let is_monospace = check_monospace(&group_chars);

        // Determine semantic role
        let role = if is_monospace {
            FontSemanticRole::Code
        } else if size_key > body_size + 8 {
            // >4pt larger = title
            FontSemanticRole::Title
        } else if size_key > body_size + 2 {
            // >1pt larger = header
            FontSemanticRole::SectionHeader
        } else if size_key < body_size - 4 {
            // >2pt smaller = footnote
            FontSemanticRole::Footnote
        } else {
            FontSemanticRole::Body
        };

        clusters.push(FontCluster {
            role,
            font_size,
            is_monospace,
            char_count,
            coverage,
        });
    }

    // Sort by font size descending
    clusters.sort_by(|a, b| {
        b.font_size
            .partial_cmp(&a.font_size)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(clusters)
}

/// Check if characters appear to be monospace
fn check_monospace(chars: &[&PdfChar]) -> bool {
    if chars.len() < 5 {
        return false;
    }

    // Get widths of non-space characters
    let widths: Vec<f64> = chars
        .iter()
        .filter(|c| !c.unicode.is_whitespace())
        .map(|c| c.right - c.left)
        .collect();

    if widths.len() < 3 {
        return false;
    }

    // Check if widths are uniform (within 10% tolerance)
    let avg_width: f64 = widths.iter().sum::<f64>() / widths.len() as f64;
    if avg_width < 0.1 {
        return false;
    }

    let variance = widths
        .iter()
        .map(|w| ((w - avg_width) / avg_width).abs())
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(1.0);

    variance < 0.1
}

// ============================================================================
// Layout Detection
// ============================================================================

/// Type of layout region detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutRegionType {
    /// Main text column
    TextColumn,
    /// Header or footer area
    HeaderFooter,
    /// Figure or image area
    Figure,
    /// Side column or margin notes
    Sidebar,
    /// Margin area (empty or annotations)
    Margin,
}

/// A detected layout region on the page.
#[derive(Debug, Clone)]
pub struct LayoutRegion {
    /// Type of region
    pub region_type: LayoutRegionType,
    /// Bounding box (left, top, right, bottom)
    pub bounds: (f32, f32, f32, f32),
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Column index if multi-column (0 for single column)
    pub column_index: usize,
}

/// Detect layout regions on a page.
///
/// Analyzes text positioning to identify columns, headers, footers, and margins.
pub fn detect_layout_regions(page: &PdfPage) -> Result<Vec<LayoutRegion>> {
    let text = page.text()?;
    let chars: Vec<PdfChar> = text.chars().collect();

    let page_width = page.width() as f32;
    let page_height = page.height() as f32;

    let mut regions = Vec::new();

    if chars.is_empty() {
        // No text - return page as margin
        regions.push(LayoutRegion {
            region_type: LayoutRegionType::Margin,
            bounds: (0.0, page_height, page_width, 0.0),
            confidence: 1.0,
            column_index: 0,
        });
        return Ok(regions);
    }

    // Get text bounds
    let text_left = chars.iter().map(|c| c.left as f32).fold(f32::MAX, f32::min);
    let text_right = chars
        .iter()
        .map(|c| c.right as f32)
        .fold(f32::MIN, f32::max);
    let text_top = chars.iter().map(|c| c.top as f32).fold(f32::MIN, f32::max);
    let text_bottom = chars
        .iter()
        .map(|c| c.bottom as f32)
        .fold(f32::MAX, f32::min);

    // Detect header region (top 10% with text)
    let header_threshold = page_height * 0.9;
    let header_chars: Vec<_> = chars
        .iter()
        .filter(|c| c.top as f32 > header_threshold)
        .collect();
    if !header_chars.is_empty() {
        regions.push(LayoutRegion {
            region_type: LayoutRegionType::HeaderFooter,
            bounds: (0.0, page_height, page_width, header_threshold),
            confidence: 0.8,
            column_index: 0,
        });
    }

    // Detect footer region (bottom 10%)
    let footer_threshold = page_height * 0.1;
    let footer_chars: Vec<_> = chars
        .iter()
        .filter(|c| (c.bottom as f32) < footer_threshold)
        .collect();
    if !footer_chars.is_empty() {
        regions.push(LayoutRegion {
            region_type: LayoutRegionType::HeaderFooter,
            bounds: (0.0, footer_threshold, page_width, 0.0),
            confidence: 0.8,
            column_index: 0,
        });
    }

    // Detect columns by analyzing horizontal distribution
    let mid_x = page_width / 2.0;
    let left_chars: Vec<_> = chars
        .iter()
        .filter(|c| {
            (c.right as f32) < mid_x - 20.0
                && (c.top as f32) <= header_threshold
                && (c.bottom as f32) >= footer_threshold
        })
        .collect();
    let right_chars: Vec<_> = chars
        .iter()
        .filter(|c| {
            (c.left as f32) > mid_x + 20.0
                && (c.top as f32) <= header_threshold
                && (c.bottom as f32) >= footer_threshold
        })
        .collect();

    // Check for two-column layout
    if !left_chars.is_empty() && !right_chars.is_empty() {
        // Two columns detected
        let left_right = left_chars
            .iter()
            .map(|c| c.right as f32)
            .fold(f32::MIN, f32::max);
        let right_left = right_chars
            .iter()
            .map(|c| c.left as f32)
            .fold(f32::MAX, f32::min);

        // Left column
        regions.push(LayoutRegion {
            region_type: LayoutRegionType::TextColumn,
            bounds: (text_left, header_threshold, left_right, footer_threshold),
            confidence: 0.9,
            column_index: 0,
        });

        // Right column
        regions.push(LayoutRegion {
            region_type: LayoutRegionType::TextColumn,
            bounds: (right_left, header_threshold, text_right, footer_threshold),
            confidence: 0.9,
            column_index: 1,
        });
    } else {
        // Single column
        regions.push(LayoutRegion {
            region_type: LayoutRegionType::TextColumn,
            bounds: (
                text_left,
                text_top.min(header_threshold),
                text_right,
                text_bottom.max(footer_threshold),
            ),
            confidence: 0.95,
            column_index: 0,
        });
    }

    // Add margin regions
    if text_left > page_width * 0.1 {
        regions.push(LayoutRegion {
            region_type: LayoutRegionType::Margin,
            bounds: (0.0, page_height, text_left, 0.0),
            confidence: 0.7,
            column_index: 0,
        });
    }

    if text_right < page_width * 0.9 {
        regions.push(LayoutRegion {
            region_type: LayoutRegionType::Margin,
            bounds: (text_right, page_height, page_width, 0.0),
            confidence: 0.7,
            column_index: 0,
        });
    }

    Ok(regions)
}

// ============================================================================
// Image Content Hints
// ============================================================================

/// Hint about the type of content in an image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageContentHint {
    /// Photographic content
    Photo,
    /// Line art or diagram
    Diagram,
    /// Chart or graph
    Chart,
    /// Scanned text (needs OCR)
    ScannedText,
    /// Logo or icon
    Logo,
    /// Cannot determine type
    Unknown,
}

// ============================================================================
// Document Classification
// ============================================================================

/// Type of document detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentType {
    /// Academic article or paper
    Article,
    /// Book or long-form document
    Book,
    /// Presentation slides
    Slides,
    /// Form with fields to fill
    Form,
    /// Invoice or receipt
    Invoice,
    /// Letter or correspondence
    Letter,
    /// Technical documentation
    Technical,
    /// Could not determine type
    Unknown,
}

/// Document classification for pipeline routing.
///
/// Provides metadata to help route documents to the optimal processing pipeline.
#[derive(Debug, Clone)]
pub struct DoclingClassification {
    /// True if document appears to be scanned (needs OCR)
    pub is_scanned: bool,
    /// True if document has structure tags (can skip ML)
    pub is_tagged: bool,
    /// True if document has form fields
    pub has_forms: bool,
    /// True if document is multi-column layout
    pub is_multi_column: bool,
    /// Detected document type
    pub document_type: DocumentType,
    /// Number of pages
    pub page_count: u32,
    /// Average text length per page
    pub avg_text_per_page: f32,
    /// Percentage of pages with images
    pub image_coverage: f32,
}

impl DoclingClassification {
    /// Analyze a document and return classification.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::{Pdfium, DoclingClassification};
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    ///
    /// let classification = DoclingClassification::analyze(&doc);
    /// println!("Scanned: {}, Tagged: {}", classification.is_scanned, classification.is_tagged);
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn analyze(doc: &PdfDocument) -> Self {
        let page_count = doc.page_count();
        let is_tagged = doc.is_tagged();
        let has_forms = doc.form_type() != PdfFormType::None;

        let mut total_text_chars = 0usize;
        let mut scanned_pages = 0usize;
        let mut multi_column_pages = 0usize;
        let mut pages_with_images = 0usize;

        // Sample first 10 pages for analysis
        let sample_count = page_count.min(10);
        for i in 0..sample_count {
            if let Ok(page) = doc.page(i) {
                // Check text content
                if let Ok(text) = page.text() {
                    let char_count = text.char_count();
                    total_text_chars += char_count;

                    // Check if scanned (has images but no/little text)
                    let objects = page.objects();
                    let image_count = objects.iter().filter(|o| o.is_image()).count();

                    if image_count > 0 && char_count < 100 {
                        scanned_pages += 1;
                    }

                    if image_count > 0 {
                        pages_with_images += 1;
                    }

                    // Check for multi-column (simplified)
                    if let Ok(regions) = detect_layout_regions(&page) {
                        let column_count = regions
                            .iter()
                            .filter(|r| r.region_type == LayoutRegionType::TextColumn)
                            .count();
                        if column_count > 1 {
                            multi_column_pages += 1;
                        }
                    }
                }
            }
        }

        let is_scanned = sample_count > 0 && scanned_pages > sample_count / 2;
        let is_multi_column = sample_count > 0 && multi_column_pages > sample_count / 2;
        let avg_text_per_page = if sample_count > 0 {
            total_text_chars as f32 / sample_count as f32
        } else {
            0.0
        };
        let image_coverage = if sample_count > 0 {
            pages_with_images as f32 / sample_count as f32
        } else {
            0.0
        };

        // Determine document type
        let document_type = Self::classify_type(
            page_count,
            has_forms,
            is_multi_column,
            avg_text_per_page,
            image_coverage,
        );

        DoclingClassification {
            is_scanned,
            is_tagged,
            has_forms,
            is_multi_column,
            document_type,
            page_count: page_count as u32,
            avg_text_per_page,
            image_coverage,
        }
    }

    fn classify_type(
        page_count: usize,
        has_forms: bool,
        is_multi_column: bool,
        avg_text_per_page: f32,
        image_coverage: f32,
    ) -> DocumentType {
        // Forms are easy to detect
        if has_forms {
            return DocumentType::Form;
        }

        // Slides: few pages, high image coverage, low text
        if page_count > 5 && image_coverage > 0.7 && avg_text_per_page < 200.0 {
            return DocumentType::Slides;
        }

        // Invoice: 1-3 pages, low text, structured layout
        if page_count <= 3 && avg_text_per_page < 500.0 {
            return DocumentType::Invoice;
        }

        // Letter: 1-2 pages, moderate text
        if page_count <= 2 && avg_text_per_page > 200.0 && avg_text_per_page < 2000.0 {
            return DocumentType::Letter;
        }

        // Article: multi-column, academic-style
        if is_multi_column && (2..=30).contains(&page_count) {
            return DocumentType::Article;
        }

        // Book: many pages
        if page_count > 50 {
            return DocumentType::Book;
        }

        // Technical: moderate pages, high text density
        if (5..=50).contains(&page_count) && avg_text_per_page > 1000.0 {
            return DocumentType::Technical;
        }

        DocumentType::Unknown
    }
}

impl PdfChar {
    /// Clone the character (needed for grouping operations)
    fn clone(&self) -> Self {
        PdfChar {
            index: self.index,
            unicode: self.unicode,
            left: self.left,
            bottom: self.bottom,
            right: self.right,
            top: self.top,
            font_size: self.font_size,
            angle: self.angle,
            origin_x: self.origin_x,
            origin_y: self.origin_y,
        }
    }
}
