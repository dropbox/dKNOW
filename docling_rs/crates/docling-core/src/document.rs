//! Core document types
//!
//! This module defines the main `Document` type and associated metadata.

use crate::content::{ContentBlock, DocItem, ItemRef};
use crate::error::Result;
use crate::format::InputFormat;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

/// Core document structure containing converted content and metadata.
///
/// Represents a converted document with markdown output, structured content,
/// and associated metadata. This is the primary result type from document conversion.
///
/// # Examples
///
/// ## Basic Document Access
///
/// ```rust,ignore
/// // Note: DocumentConverter is in docling-backend crate
/// use docling_backend::DocumentConverter;
///
/// let converter = DocumentConverter::new()?;
/// let result = converter.convert("document.pdf")?;
/// let doc = result.document;
///
/// // Access markdown output
/// println!("Markdown:\n{}", doc.markdown);
///
/// // Check metadata
/// println!("Format: {:?}", doc.format);
/// println!("Characters: {}", doc.metadata.num_characters);
/// # Ok::<(), docling_core::DoclingError>(())
/// ```
///
/// ## Structured Content
///
/// ```rust,ignore
/// // Note: DocumentConverter is in docling-backend crate
/// use docling_backend::DocumentConverter;
///
/// let converter = DocumentConverter::new()?;
/// let result = converter.convert("report.pdf")?;
///
/// if result.document.has_structured_content() {
///     if let Some(blocks) = result.document.blocks() {
///         println!("Document has {} content blocks", blocks.len());
///         for (i, block) in blocks.iter().enumerate() {
///             println!("Block {}: {:?}", i, block);
///         }
///     }
/// }
/// # Ok::<(), docling_core::DoclingError>(())
/// ```
///
/// ## Creating Documents
///
/// ```rust
/// use docling_core::{Document, InputFormat};
///
/// // Create a simple document from markdown
/// let doc = Document::from_markdown(
///     "# Hello World\n\nThis is a test.".to_string(),
///     InputFormat::Md
/// );
///
/// assert_eq!(doc.metadata.num_characters, 30);
/// assert_eq!(doc.format, InputFormat::Md);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    /// Markdown representation of the document
    pub markdown: String,

    /// Input format of the original document
    pub format: InputFormat,

    /// Document metadata
    pub metadata: DocumentMetadata,

    /// Structured content blocks (optional - for structured extraction)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_blocks: Option<Vec<ContentBlock>>,

    /// Full `DoclingDocument` structure (optional - for PDF/ML backends)
    /// This provides access to the complete document structure including:
    /// - Table data with full cell metadata
    /// - Picture metadata
    /// - Hierarchical document structure (body, groups)
    /// - Page information
    /// - Provenance data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docling_document: Option<Box<DoclingDocument>>,
}

/// Document metadata containing information about the source document.
///
/// Includes details such as page count, title, author, dates, and character count.
/// All fields except `num_characters` are optional as not all formats provide
/// metadata.
///
/// # Examples
///
/// ```rust,ignore
/// // Note: DocumentConverter is in docling-backend crate
/// use docling_backend::DocumentConverter;
///
/// let converter = DocumentConverter::new()?;
/// let result = converter.convert("document.pdf")?;
/// let metadata = &result.document.metadata;
///
/// // Always available
/// println!("Characters: {}", metadata.num_characters);
///
/// // Optional fields
/// if let Some(pages) = metadata.num_pages {
///     println!("Pages: {}", pages);
/// }
/// if let Some(title) = &metadata.title {
///     println!("Title: {}", title);
/// }
/// if let Some(author) = &metadata.author {
///     println!("Author: {}", author);
/// }
/// # Ok::<(), docling_core::DoclingError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DocumentMetadata {
    /// Number of pages (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_pages: Option<usize>,

    /// Total character count
    #[serde(default)]
    pub num_characters: usize,

    /// Document title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Document author(s)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    /// Creation date
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<chrono::DateTime<chrono::Utc>>,

    /// Last modified date
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified: Option<chrono::DateTime<chrono::Utc>>,

    /// Language (ISO 639-1 code)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,

    /// Document subject/description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,

    /// EXIF metadata (for image formats)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exif: Option<ExifMetadata>,
}

/// EXIF metadata extracted from image files.
///
/// Contains photographic metadata embedded in JPEG, TIFF, and other image formats.
/// This includes camera information, capture settings, GPS coordinates, and timestamps.
///
/// # Examples
///
/// ```rust,ignore
/// // Note: DocumentConverter is in docling-backend crate
/// use docling_backend::DocumentConverter;
///
/// let converter = DocumentConverter::new()?;
/// let result = converter.convert("photo.jpg")?;
///
/// if let Some(exif) = &result.document.metadata.exif {
///     if let Some(datetime) = exif.datetime {
///         println!("Photo taken: {}", datetime);
///     }
///     if let Some(make) = &exif.camera_make {
///         println!("Camera: {}", make);
///     }
///     if let (Some(lat), Some(lon)) = (exif.gps_latitude, exif.gps_longitude) {
///         println!("Location: {}, {}", lat, lon);
///     }
/// }
/// # Ok::<(), docling_core::DoclingError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ExifMetadata {
    /// Date and time when the image was captured (EXIF `DateTimeOriginal` or `DateTime`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datetime: Option<chrono::DateTime<chrono::Utc>>,

    /// Camera manufacturer (e.g., "Canon", "Nikon", "Apple")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera_make: Option<String>,

    /// Camera model (e.g., "Canon EOS 5D Mark IV", "iPhone 12 Pro")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera_model: Option<String>,

    /// GPS latitude in decimal degrees (positive = North, negative = South)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gps_latitude: Option<f64>,

    /// GPS longitude in decimal degrees (positive = East, negative = West)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gps_longitude: Option<f64>,

    /// GPS altitude in meters above sea level
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gps_altitude: Option<f64>,

    /// Image orientation (1-8, following EXIF specification)
    /// 1 = normal, 3 = 180°, 6 = 90° CW, 8 = 90° CCW
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orientation: Option<u32>,

    /// Software used to create or process the image
    #[serde(skip_serializing_if = "Option::is_none")]
    pub software: Option<String>,

    /// Exposure time in seconds (e.g., 0.00625 = 1/160s)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exposure_time: Option<f64>,

    /// F-number (aperture value, e.g., 2.8 = f/2.8)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub f_number: Option<f64>,

    /// ISO speed rating (e.g., 100, 400, 1600)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iso_speed: Option<u32>,

    /// Focal length in millimeters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focal_length: Option<f64>,

    /// HDR color primaries (e.g., "BT.2020", "Display P3", "sRGB")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hdr_color_primaries: Option<String>,

    /// HDR transfer characteristics (e.g., "PQ" for HDR10, "HLG" for Hybrid Log-Gamma)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hdr_transfer_characteristics: Option<String>,

    /// Maximum content light level in nits (cd/m²)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hdr_max_content_light_level: Option<u32>,

    /// Maximum frame-average light level in nits (cd/m²)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hdr_max_frame_average_light_level: Option<u32>,

    /// Mastering display max luminance in nits
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hdr_mastering_display_max_luminance: Option<u32>,

    /// Mastering display min luminance in 0.0001 nits
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hdr_mastering_display_min_luminance: Option<u32>,
}

impl Document {
    /// Returns the markdown representation of the document.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// // Note: DocumentConverter is in docling-backend crate
    /// use docling_backend::DocumentConverter;
    ///
    /// let converter = DocumentConverter::new()?;
    /// let result = converter.convert("document.pdf")?;
    ///
    /// let markdown = result.document.to_markdown();
    /// println!("{}", markdown);
    /// # Ok::<(), docling_core::DoclingError>(())
    /// ```
    #[inline]
    #[must_use = "returns the markdown representation"]
    pub fn to_markdown(&self) -> &str {
        &self.markdown
    }

    /// Creates a simple document from markdown text.
    ///
    /// This is useful for testing or when you need to create a `Document`
    /// from existing markdown without conversion.
    ///
    /// # Parameters
    ///
    /// - `markdown`: The markdown text content
    /// - `format`: The original format of the document
    ///
    /// # Examples
    ///
    /// ```rust
    /// use docling_core::{Document, InputFormat};
    ///
    /// let markdown = "# Title\n\nSome content.".to_string();
    /// let doc = Document::from_markdown(markdown, InputFormat::Md);
    ///
    /// assert_eq!(doc.to_markdown(), "# Title\n\nSome content.");
    /// assert_eq!(doc.format, InputFormat::Md);
    /// assert_eq!(doc.metadata.num_characters, 22);
    /// ```
    #[inline]
    #[must_use = "creates a document from markdown text"]
    pub fn from_markdown(markdown: String, format: InputFormat) -> Self {
        let num_characters = markdown.chars().count();
        Self {
            markdown,
            format,
            metadata: DocumentMetadata {
                num_pages: None,
                num_characters,
                title: None,
                author: None,
                subject: None,
                created: None,
                modified: None,
                language: None,
                exif: None,
            },
            content_blocks: None,
            docling_document: None,
        }
    }

    /// Checks if the document has structured content blocks.
    ///
    /// Returns `true` if the document contains one or more content blocks,
    /// `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// // Note: DocumentConverter is in docling-backend crate
    /// use docling_backend::DocumentConverter;
    ///
    /// let converter = DocumentConverter::new()?;
    /// let result = converter.convert("document.pdf")?;
    ///
    /// if result.document.has_structured_content() {
    ///     println!("Document has structured content");
    /// } else {
    ///     println!("Document has only markdown");
    /// }
    /// # Ok::<(), docling_core::DoclingError>(())
    /// ```
    #[inline]
    #[must_use = "returns whether the document has structured content"]
    pub fn has_structured_content(&self) -> bool {
        self.content_blocks
            .as_ref()
            .is_some_and(|blocks| !blocks.is_empty())
    }

    /// Returns the structured content blocks if available.
    ///
    /// Returns `None` if the document has no structured content,
    /// or `Some(&[ContentBlock])` with the content blocks.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// // Note: DocumentConverter is in docling-backend crate
    /// use docling_backend::DocumentConverter;
    ///
    /// let converter = DocumentConverter::new()?;
    /// let result = converter.convert("document.pdf")?;
    ///
    /// if let Some(blocks) = result.document.blocks() {
    ///     for (i, block) in blocks.iter().enumerate() {
    ///         println!("Block {}: {:?}", i, block);
    ///     }
    /// }
    /// # Ok::<(), docling_core::DoclingError>(())
    /// ```
    #[inline]
    #[must_use = "returns the structured content blocks if available"]
    pub fn blocks(&self) -> Option<&[ContentBlock]> {
        self.content_blocks.as_deref()
    }

    /// Returns basic document statistics.
    ///
    /// Provides a quick overview of document content including:
    /// - Number of characters
    /// - Number of lines
    /// - Number of pages (if available)
    /// - Number of content blocks (if structured)
    /// - Word count estimate
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// // Note: DocumentConverter is in docling-backend crate
    /// use docling_backend::DocumentConverter;
    ///
    /// let converter = DocumentConverter::new()?;
    /// let result = converter.convert("document.pdf")?;
    ///
    /// let stats = result.document.stats();
    /// println!("Characters: {}", stats.num_characters);
    /// println!("Lines: {}", stats.num_lines);
    /// println!("Words (est): {}", stats.word_count);
    /// if let Some(pages) = stats.num_pages {
    ///     println!("Pages: {}", pages);
    /// }
    /// # Ok::<(), docling_core::DoclingError>(())
    /// ```
    #[must_use = "returns basic document statistics"]
    pub fn stats(&self) -> DocumentStats {
        let num_characters = self.metadata.num_characters;
        let num_lines = self.markdown.lines().count();
        let num_pages = self.metadata.num_pages;
        let num_blocks = self.content_blocks.as_ref().map(std::vec::Vec::len);

        // Estimate word count (split on whitespace and filter non-empty)
        let word_count = self
            .markdown
            .split_whitespace()
            .filter(|s| !s.is_empty())
            .count();

        DocumentStats {
            num_characters,
            num_lines,
            num_pages,
            num_blocks,
            word_count,
        }
    }

    /// Checks if the document is empty.
    ///
    /// Returns `true` if the document has no content (empty markdown),
    /// `false` otherwise. This is useful for filtering out failed conversions
    /// or skipping empty documents in batch processing.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use docling_core::{Document, InputFormat};
    ///
    /// let empty_doc = Document::from_markdown(String::new(), InputFormat::Md);
    /// assert!(empty_doc.is_empty());
    ///
    /// let doc_with_content = Document::from_markdown(
    ///     "# Hello World".to_string(),
    ///     InputFormat::Md
    /// );
    /// assert!(!doc_with_content.is_empty());
    /// ```
    #[inline]
    #[must_use = "returns whether the document is empty"]
    pub const fn is_empty(&self) -> bool {
        self.markdown.is_empty()
    }

    /// Returns the number of pages in the document, if available.
    ///
    /// This is a convenience method for accessing `self.metadata.num_pages`.
    /// Returns `None` for formats that don't have a page concept (e.g., HTML, CSV).
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// // Note: DocumentConverter is in docling-backend crate
    /// use docling_backend::DocumentConverter;
    ///
    /// let converter = DocumentConverter::new()?;
    /// let result = converter.convert("document.pdf")?;
    ///
    /// if let Some(pages) = result.document.page_count() {
    ///     println!("Document has {} pages", pages);
    /// } else {
    ///     println!("Page count not available for this format");
    /// }
    /// # Ok::<(), docling_core::DoclingError>(())
    /// ```
    #[inline]
    #[must_use = "returns the page count if available"]
    pub const fn page_count(&self) -> Option<usize> {
        self.metadata.num_pages
    }
}

/// Document statistics returned by [`Document::stats()`].
///
/// Provides basic metrics about a converted document.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DocumentStats {
    /// Number of characters in the markdown output
    pub num_characters: usize,

    /// Number of lines in the markdown output
    pub num_lines: usize,

    /// Number of pages in the original document (if available)
    pub num_pages: Option<usize>,

    /// Number of structured content blocks (if available)
    pub num_blocks: Option<usize>,

    /// Estimated word count (splits on whitespace)
    pub word_count: usize,
}

// Note: DocumentFormat is re-exported from crate::format module as an alias to InputFormat

/// `DoclingDocument` - matches Python's `DoclingDocument` JSON structure
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DoclingDocument {
    /// Schema name (e.g., "`DoclingDocument`")
    pub schema_name: String,

    /// Schema version (e.g., "1.7.0")
    pub version: String,

    /// Document name
    pub name: String,

    /// Document origin information
    pub origin: Origin,

    /// Body content group
    pub body: GroupItem,

    /// Furniture content group (headers/footers)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub furniture: Option<GroupItem>,

    /// Text items
    #[serde(default)]
    pub texts: Vec<DocItem>,

    /// Group items (lists, etc.)
    #[serde(default)]
    pub groups: Vec<DocItem>,

    /// Table items
    #[serde(default)]
    pub tables: Vec<DocItem>,

    /// Picture items
    #[serde(default)]
    pub pictures: Vec<DocItem>,

    /// Key-value items
    #[serde(default)]
    pub key_value_items: Vec<DocItem>,

    /// Form items
    #[serde(default)]
    pub form_items: Vec<DocItem>,

    /// Pages information
    pub pages: HashMap<String, PageInfo>,
}

/// Document origin information
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Origin {
    /// MIME type
    pub mimetype: String,

    /// Binary hash
    pub binary_hash: u64,

    /// Filename
    pub filename: String,
}

/// Group item (for body/furniture/lists)
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GroupItem {
    /// Self reference
    pub self_ref: String,

    /// Parent reference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<ItemRef>,

    /// Children references
    #[serde(default)]
    pub children: Vec<ItemRef>,

    /// Content layer
    pub content_layer: String,

    /// Name
    pub name: String,

    /// Label
    pub label: String,
}

/// Page information
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct PageInfo {
    /// Page size
    pub size: PageSize,

    /// Page number
    pub page_no: usize,
}

/// Page size
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct PageSize {
    /// Width in points
    pub width: f64,

    /// Height in points
    pub height: f64,
}

impl DoclingDocument {
    /// Get all items from the document (texts, groups, tables, pictures)
    #[must_use = "returns all items from the document"]
    pub fn all_items(&self) -> Vec<&DocItem> {
        let mut items = Vec::new();
        items.extend(&self.texts);
        items.extend(&self.groups);
        items.extend(&self.tables);
        items.extend(&self.pictures);
        items
    }

    /// Find an item by its reference path
    #[must_use = "finds an item by its reference path"]
    pub fn find_item(&self, ref_path: &str) -> Option<&DocItem> {
        self.all_items()
            .into_iter()
            .find(|item| item.self_ref() == ref_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_from_markdown() {
        let doc = Document::from_markdown("# Hello\n\nWorld".to_string(), InputFormat::Md);

        assert_eq!(doc.to_markdown(), "# Hello\n\nWorld");
        assert_eq!(doc.format, InputFormat::Md);
        assert!(!doc.has_structured_content());
    }

    #[test]
    fn test_document_with_structured_content() {
        use crate::content::{ContentBlock, ItemRef};

        let blocks = vec![
            ContentBlock::SectionHeader {
                self_ref: "#/texts/0".to_string(),
                parent: Some(ItemRef::new("#/body")),
                children: vec![],
                content_layer: "body".to_string(),
                prov: vec![],
                orig: "Introduction".to_string(),
                text: "Introduction".to_string(),
                level: 1,
                formatting: None,
                hyperlink: None,
            },
            ContentBlock::Text {
                self_ref: "#/texts/1".to_string(),
                parent: Some(ItemRef::new("#/body")),
                children: vec![],
                content_layer: "body".to_string(),
                prov: vec![],
                orig: "This is a paragraph.".to_string(),
                text: "This is a paragraph.".to_string(),
                formatting: None,
                hyperlink: None,
            },
        ];

        let doc = Document {
            markdown: "# Introduction\n\nThis is a paragraph.".to_string(),
            format: InputFormat::Pdf,
            metadata: DocumentMetadata::default(),
            content_blocks: Some(blocks),
            docling_document: None,
        };

        assert!(doc.has_structured_content());
        assert_eq!(doc.blocks().unwrap().len(), 2);
    }

    #[test]
    fn test_document_serialization() {
        let doc = Document::from_markdown("Test document".to_string(), InputFormat::Pdf);

        let json = serde_json::to_string(&doc).unwrap();
        let deserialized: Document = serde_json::from_str(&json).unwrap();

        assert_eq!(doc.markdown, deserialized.markdown);
        assert_eq!(doc.format, deserialized.format);
    }

    #[test]
    fn test_document_stats() {
        let markdown =
            "# Title\n\nThis is a test document with some words.\n\nAnother paragraph.".to_string();
        let num_chars = markdown.chars().count();
        let doc = Document {
            markdown,
            format: InputFormat::Md,
            metadata: DocumentMetadata {
                num_pages: Some(3_usize),
                num_characters: num_chars,
                ..Default::default()
            },
            content_blocks: Some(vec![ContentBlock::Text {
                self_ref: "#/texts/0".to_string(),
                parent: None,
                children: vec![],
                content_layer: "body".to_string(),
                prov: vec![],
                orig: "test".to_string(),
                text: "test".to_string(),
                formatting: None,
                hyperlink: None,
            }]),
            docling_document: None,
        };

        let stats = doc.stats();

        assert_eq!(stats.num_characters, num_chars);
        assert_eq!(stats.num_lines, 5);
        assert_eq!(stats.num_pages, Some(3_usize));
        assert_eq!(stats.num_blocks, Some(1));
        assert_eq!(stats.word_count, 12); // "Title", "This", "is", "a", "test", "document", "with", "some", "words", "Another", "paragraph"
    }

    #[test]
    fn test_document_stats_no_pages() {
        let doc = Document::from_markdown("Hello world".to_string(), InputFormat::Md);
        let stats = doc.stats();

        assert_eq!(stats.num_characters, 11);
        assert_eq!(stats.num_lines, 1);
        assert_eq!(stats.num_pages, None);
        assert_eq!(stats.num_blocks, None);
        assert_eq!(stats.word_count, 2);
    }

    #[test]
    fn test_docling_document_deserialization() {
        // This test verifies we can load Python's DoclingDocument JSON format
        // N=4322: Fixed path to use groundtruth (full DoclingDocument), not test-results (array)
        // Path relative to crates/docling-core
        let json_path = "../../test-corpus/groundtruth/docling_v2/multi_page.json";

        if !std::path::Path::new(json_path).exists() {
            eprintln!("Skipping test: {json_path} not found");
            return;
        }

        let json_content =
            std::fs::read_to_string(json_path).expect("Failed to read multi_page.json");

        let doc: DoclingDocument =
            serde_json::from_str(&json_content).expect("Failed to deserialize DoclingDocument");

        // Verify basic structure
        assert_eq!(doc.schema_name, "DoclingDocument");
        // Note: Schema version may vary (1.7.0 in file, but Python docling 2.58.0 may use 1.8.0)
        assert!(
            doc.version == "1.7.0" || doc.version == "1.8.0",
            "Version should be 1.7.0 or 1.8.0, got: {}",
            doc.version
        );
        assert_eq!(doc.name, "multi_page");
        assert_eq!(doc.origin.filename, "multi_page.pdf");

        // Verify we have content
        assert!(!doc.texts.is_empty(), "Should have text items");
        assert!(!doc.groups.is_empty(), "Should have group items");
        assert!(!doc.pages.is_empty(), "Should have page info");

        // Verify first text item
        if let Some(first_text) = doc.texts.first() {
            assert_eq!(first_text.self_ref(), "#/texts/0");
            assert_eq!(
                first_text.text(),
                Some("The Evolution of the Word Processor")
            );
        }

        // Verify we can find items by reference
        let item = doc.find_item("#/texts/0");
        assert!(item.is_some(), "Should find item by reference");
    }

    #[test]
    fn test_document_with_exif_metadata() {
        // Test EXIF metadata handling for image documents
        let exif = ExifMetadata {
            datetime: Some(chrono::Utc::now()),
            camera_make: Some("Canon".to_string()),
            camera_model: Some("EOS 5D Mark IV".to_string()),
            gps_latitude: Some(37.7749), // San Francisco
            gps_longitude: Some(-122.4194),
            gps_altitude: Some(52.0),
            orientation: Some(1),
            software: Some("Adobe Photoshop 2024".to_string()),
            exposure_time: Some(0.00625), // 1/160s
            f_number: Some(2.8),
            iso_speed: Some(400),
            focal_length: Some(50.0),
            hdr_color_primaries: None,
            hdr_transfer_characteristics: None,
            hdr_max_content_light_level: None,
            hdr_max_frame_average_light_level: None,
            hdr_mastering_display_max_luminance: None,
            hdr_mastering_display_min_luminance: None,
        };

        let metadata = DocumentMetadata {
            exif: Some(exif.clone()),
            ..Default::default()
        };

        let doc = Document {
            markdown: "![Photo](image.jpg)".to_string(),
            format: InputFormat::Jpeg,
            metadata,
            content_blocks: None,
            docling_document: None,
        };

        // Verify EXIF data is preserved
        assert!(doc.metadata.exif.is_some());
        let stored_exif = doc.metadata.exif.as_ref().unwrap();
        assert_eq!(stored_exif.camera_make, Some("Canon".to_string()));
        assert_eq!(stored_exif.camera_model, Some("EOS 5D Mark IV".to_string()));
        assert_eq!(stored_exif.gps_latitude, Some(37.7749));
        assert_eq!(stored_exif.iso_speed, Some(400));
        assert_eq!(stored_exif.focal_length, Some(50.0));

        // Verify serialization preserves EXIF
        let json = serde_json::to_string(&doc).unwrap();
        let deserialized: Document = serde_json::from_str(&json).unwrap();
        assert!(deserialized.metadata.exif.is_some());
        assert_eq!(
            deserialized.metadata.exif.as_ref().unwrap().camera_make,
            Some("Canon".to_string())
        );
    }

    #[test]
    fn test_document_metadata_complete() {
        // Test all metadata fields populated
        let metadata = DocumentMetadata {
            num_pages: Some(150),
            num_characters: 125_000,
            title: Some("Complete Technical Specification".to_string()),
            author: Some("Engineering Team".to_string()),
            created: Some(chrono::Utc::now()),
            modified: Some(chrono::Utc::now()),
            language: Some("en".to_string()),
            subject: Some("Technical documentation".to_string()),
            exif: None,
        };

        let doc = Document {
            markdown: "# Technical Specification\n\nContent here...".to_string(),
            format: InputFormat::Pdf,
            metadata: metadata.clone(),
            content_blocks: None,
            docling_document: None,
        };

        // Verify all metadata fields
        assert_eq!(doc.metadata.num_pages, Some(150));
        assert_eq!(doc.metadata.num_characters, 125_000);
        assert_eq!(
            doc.metadata.title,
            Some("Complete Technical Specification".to_string())
        );
        assert_eq!(doc.metadata.author, Some("Engineering Team".to_string()));
        assert!(doc.metadata.created.is_some());
        assert!(doc.metadata.modified.is_some());
        assert_eq!(doc.metadata.language, Some("en".to_string()));

        // Verify JSON round-trip with all fields
        let json = serde_json::to_string_pretty(&doc).unwrap();
        assert!(json.contains("Complete Technical Specification"));
        assert!(json.contains("Engineering Team"));

        let deserialized: Document = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.metadata.num_pages, Some(150));
        assert_eq!(
            deserialized.metadata.title,
            Some("Complete Technical Specification".to_string())
        );
    }

    #[test]
    fn test_document_empty_and_edge_cases() {
        // Test empty document
        let empty_doc = Document::from_markdown("".to_string(), InputFormat::Md);
        assert_eq!(empty_doc.markdown, "");
        assert_eq!(empty_doc.metadata.num_characters, 0);
        assert!(!empty_doc.has_structured_content());

        // Test document with only whitespace
        let whitespace_doc =
            Document::from_markdown("   \n\n\t\t  \n".to_string(), InputFormat::Md);
        assert!(whitespace_doc.metadata.num_characters > 0);

        // Test very long markdown
        let long_content = "A".repeat(1_000_000);
        let long_doc = Document::from_markdown(long_content.clone(), InputFormat::Md);
        assert_eq!(long_doc.markdown.len(), 1_000_000);
        assert_eq!(long_doc.metadata.num_characters, 1_000_000);

        // Test Unicode characters
        let unicode_doc = Document::from_markdown(
            "Hello 世界 🌍 Здравствуй مرحبا".to_string(),
            InputFormat::Md,
        );
        assert!(unicode_doc.markdown.contains("世界"));
        assert!(unicode_doc.markdown.contains("🌍"));
        assert!(unicode_doc.markdown.contains("مرحبا"));
    }

    #[test]
    fn test_document_structured_content_manipulation() {
        // Test document with multiple types of content blocks
        let blocks = vec![
            DocItem::SectionHeader {
                self_ref: "#/texts/0".to_string(),
                parent: Some(ItemRef::new("#/body")),
                children: vec![],
                text: "Chapter 1: Introduction".to_string(),
                level: 1,
                content_layer: "body".to_string(),
                prov: vec![],
                orig: "Chapter 1: Introduction".to_string(),
                formatting: None,
                hyperlink: None,
            },
            DocItem::Text {
                self_ref: "#/texts/1".to_string(),
                parent: Some(ItemRef::new("#/body")),
                children: vec![],
                text: "This is the introduction text.".to_string(),
                content_layer: "body".to_string(),
                prov: vec![],
                orig: "This is the introduction text.".to_string(),
                formatting: None,
                hyperlink: None,
            },
            DocItem::SectionHeader {
                self_ref: "#/texts/2".to_string(),
                parent: Some(ItemRef::new("#/body")),
                children: vec![],
                text: "Section 1.1".to_string(),
                level: 2,
                content_layer: "body".to_string(),
                prov: vec![],
                orig: "Section 1.1".to_string(),
                formatting: None,
                hyperlink: None,
            },
            DocItem::Text {
                self_ref: "#/texts/3".to_string(),
                parent: Some(ItemRef::new("#/body")),
                children: vec![],
                text: "Section content here.".to_string(),
                content_layer: "body".to_string(),
                prov: vec![],
                orig: "Section content here.".to_string(),
                formatting: None,
                hyperlink: None,
            },
        ];

        let doc = Document {
            markdown: "# Chapter 1: Introduction\n\nThis is the introduction text.\n\n## Section 1.1\n\nSection content here.".to_string(),
            format: InputFormat::Docx,
            metadata: DocumentMetadata::default(),
            content_blocks: Some(blocks),
            docling_document: None,
        };

        // Verify structured content access
        assert!(doc.has_structured_content());
        let retrieved_blocks = doc.blocks().unwrap();
        assert_eq!(retrieved_blocks.len(), 4);

        // Count headings vs text
        let headings: Vec<_> = retrieved_blocks
            .iter()
            .filter(|b| matches!(b, DocItem::SectionHeader { .. }))
            .collect();
        let text_blocks: Vec<_> = retrieved_blocks
            .iter()
            .filter(|b| matches!(b, DocItem::Text { .. }))
            .collect();

        assert_eq!(headings.len(), 2);
        assert_eq!(text_blocks.len(), 2);

        // Test cloning preserves structure
        let cloned_doc = doc.clone();
        assert_eq!(cloned_doc.blocks().unwrap().len(), 4);
    }

    #[test]
    fn test_document_format_variations() {
        // Test documents from different input formats
        let formats = vec![
            (InputFormat::Pdf, "PDF document content"),
            (InputFormat::Docx, "Word document content"),
            (InputFormat::Pptx, "PowerPoint slide content"),
            (InputFormat::Xlsx, "Excel table content"),
            (InputFormat::Html, "HTML page content"),
            (InputFormat::Md, "Markdown text content"),
            (InputFormat::Asciidoc, "AsciiDoc content"),
            (InputFormat::Jpeg, "Image description"),
            (InputFormat::Epub, "E-book chapter content"),
            (InputFormat::Rtf, "RTF formatted text"),
        ];

        for (format, content) in formats {
            let doc = Document::from_markdown(content.to_string(), format);

            assert_eq!(doc.format, format);
            assert_eq!(doc.markdown, content);
            assert_eq!(doc.metadata.num_characters, content.len());

            // Verify each format serializes correctly
            let json = serde_json::to_string(&doc).unwrap();
            let deserialized: Document = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized.format, format);
            assert_eq!(deserialized.markdown, content);
        }

        // Test format-specific metadata
        let mut pdf_doc = Document::from_markdown("PDF content".to_string(), InputFormat::Pdf);
        pdf_doc.metadata.num_pages = Some(42);
        assert_eq!(pdf_doc.metadata.num_pages, Some(42));

        let mut image_doc = Document::from_markdown("Image".to_string(), InputFormat::Jpeg);
        let exif = ExifMetadata {
            camera_make: Some("Sony".to_string()),
            ..Default::default()
        };
        image_doc.metadata.exif = Some(exif);
        assert!(image_doc.metadata.exif.is_some());
    }
}

/// Result of a document conversion operation
///
/// Contains the converted document and performance metrics.
/// This struct is returned by both Python-bridge and Rust-native converters.
///
/// # Examples
///
/// ```rust,no_run
/// # use docling_core::{Document, ConversionResult, InputFormat};
/// # use std::time::Duration;
/// let doc = Document::from_markdown("# Test".to_string(), InputFormat::Md);
/// let result = ConversionResult {
///     document: doc,
///     latency: Duration::from_millis(100),
/// };
///
/// println!("Conversion took {:?}", result.latency);
/// if let Some(pages) = result.document.metadata.num_pages {
///     println!("Pages: {}", pages);
/// }
/// # Ok::<(), docling_core::DoclingError>(())
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ConversionResult {
    /// The converted document with markdown and metadata
    pub document: Document,
    /// Time taken to perform the conversion
    pub latency: Duration,
}

impl ConversionResult {
    /// Save the markdown output to a file.
    ///
    /// This is a convenience method that writes the document's markdown
    /// representation to the specified file path.
    ///
    /// # Parameters
    ///
    /// - `path`: The file path to write to (will be created/overwritten)
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if the write operation fails.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use docling_core::{Document, ConversionResult, InputFormat};
    /// # use std::time::Duration;
    /// let doc = Document::from_markdown("# Test".to_string(), InputFormat::Md);
    /// let result = ConversionResult {
    ///     document: doc,
    ///     latency: Duration::from_millis(100),
    /// };
    ///
    /// // Save markdown to file
    /// result.save_markdown("output.md")?;
    /// # Ok::<(), docling_core::DoclingError>(())
    /// ```
    ///
    /// # Errors
    /// Returns an error if the file cannot be written.
    #[must_use = "this function returns a Result that should be checked for errors"]
    pub fn save_markdown<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        std::fs::write(path, &self.document.markdown)?;
        Ok(())
    }

    /// Save the document as JSON to a file.
    ///
    /// Serializes the entire document (including metadata and structured content)
    /// to JSON format and writes it to the specified file.
    ///
    /// # Parameters
    ///
    /// - `path`: The file path to write to (will be created/overwritten)
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if serialization or write fails.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use docling_core::{Document, ConversionResult, InputFormat};
    /// # use std::time::Duration;
    /// let doc = Document::from_markdown("# Test".to_string(), InputFormat::Md);
    /// let result = ConversionResult {
    ///     document: doc,
    ///     latency: Duration::from_millis(100),
    /// };
    ///
    /// // Save full document as JSON
    /// result.save_json("output.json")?;
    /// # Ok::<(), docling_core::DoclingError>(())
    /// ```
    ///
    /// # Errors
    /// Returns an error if serialization or file writing fails.
    #[must_use = "this function returns a Result that should be checked for errors"]
    pub fn save_json<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.document)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Save the document as YAML to a file.
    ///
    /// Serializes the entire document (including metadata and structured content)
    /// to YAML format and writes it to the specified file.
    ///
    /// # Parameters
    ///
    /// - `path`: The file path to write to (will be created/overwritten)
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if serialization or write fails.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use docling_core::{Document, ConversionResult, InputFormat};
    /// # use std::time::Duration;
    /// let doc = Document::from_markdown("# Test".to_string(), InputFormat::Md);
    /// let result = ConversionResult {
    ///     document: doc,
    ///     latency: Duration::from_millis(100),
    /// };
    ///
    /// // Save full document as YAML
    /// result.save_yaml("output.yaml")?;
    /// # Ok::<(), docling_core::DoclingError>(())
    /// ```
    ///
    /// # Errors
    /// Returns an error if serialization or file writing fails.
    #[must_use = "this function returns a Result that should be checked for errors"]
    pub fn save_yaml<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let yaml = serde_yaml::to_string(&self.document)?;
        std::fs::write(path, yaml)?;
        Ok(())
    }

    /// Get document statistics.
    ///
    /// This is a convenience method that delegates to `document.stats()`.
    ///
    /// # Returns
    ///
    /// Returns a `DocumentStats` struct with character count, line count, etc.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use docling_core::{Document, ConversionResult, InputFormat};
    /// # use std::time::Duration;
    /// let doc = Document::from_markdown("# Test".to_string(), InputFormat::Md);
    /// let result = ConversionResult {
    ///     document: doc,
    ///     latency: Duration::from_millis(100),
    /// };
    ///
    /// let stats = result.stats();
    /// println!("Characters: {}", stats.num_characters);
    /// # Ok::<(), docling_core::DoclingError>(())
    /// ```
    #[inline]
    #[must_use = "returns basic document statistics"]
    pub fn stats(&self) -> DocumentStats {
        self.document.stats()
    }
}
