/// Common types for e-book parsing
use serde::{Deserialize, Serialize};

/// E-book metadata extracted from EPUB, MOBI, etc.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EbookMetadata {
    /// Book title
    pub title: Option<String>,

    /// Authors/creators
    pub creators: Vec<String>,

    /// Language code (e.g., "en", "fr")
    pub language: Option<String>,

    /// Unique identifier (ISBN, UUID, etc.)
    pub identifier: Option<String>,

    /// Publisher name
    pub publisher: Option<String>,

    /// Publication date
    pub date: Option<String>,

    /// Book description/summary
    pub description: Option<String>,

    /// Subject categories/tags
    pub subjects: Vec<String>,

    /// Rights/copyright information
    pub rights: Option<String>,

    /// Contributors (editors, illustrators, etc.)
    pub contributors: Vec<String>,
}

impl EbookMetadata {
    /// Creates a new empty `EbookMetadata` instance.
    #[inline]
    #[must_use = "creates empty metadata"]
    pub const fn new() -> Self {
        Self {
            title: None,
            creators: Vec::new(),
            language: None,
            identifier: None,
            publisher: None,
            date: None,
            description: None,
            subjects: Vec::new(),
            rights: None,
            contributors: Vec::new(),
        }
    }
}

/// Table of contents entry
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TocEntry {
    /// Entry label/title
    pub label: String,

    /// Target content file (href)
    pub href: String,

    /// Play order (for EPUB 2 NCX)
    pub play_order: Option<usize>,

    /// Nested sub-entries (for hierarchical TOC)
    pub children: Vec<TocEntry>,
}

/// Page target entry from EPUB pageList
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PageTarget {
    /// Page label (e.g., "1", "{vii}", "A-1")
    pub label: String,

    /// Target content file (href with anchor)
    pub href: String,

    /// Page type: "front", "normal", or "special"
    pub page_type: Option<String>,

    /// Play order in reading sequence
    pub play_order: Option<usize>,
}

impl TocEntry {
    /// Creates a new table of contents entry with the given label and href.
    #[inline]
    #[must_use = "creates TOC entry with label and href"]
    pub const fn new(label: String, href: String) -> Self {
        Self {
            label,
            href,
            play_order: None,
            children: Vec::new(),
        }
    }
}

/// E-book chapter/section
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Chapter {
    /// Chapter title (if available)
    pub title: Option<String>,

    /// Chapter content (HTML/XHTML or plain text)
    pub content: String,

    /// File path within e-book
    pub href: String,

    /// Chapter order in spine
    pub spine_order: usize,
}

impl Chapter {
    /// Creates a new chapter with the given content, href, and spine order.
    #[inline]
    #[must_use = "creates chapter with content"]
    pub const fn new(content: String, href: String, spine_order: usize) -> Self {
        Self {
            title: None,
            content,
            href,
            spine_order,
        }
    }
}

/// Parsed e-book structure
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ParsedEbook {
    /// E-book metadata
    pub metadata: EbookMetadata,

    /// Body title (FB2 format) - title page content, may include subtitle
    /// This is separate from metadata.title and can have additional information
    pub body_title: Option<String>,

    /// Chapters in reading order
    pub chapters: Vec<Chapter>,

    /// Table of contents
    pub toc: Vec<TocEntry>,

    /// Page list (EPUB pageList for illustrations/page markers)
    pub page_list: Vec<PageTarget>,
}

impl ParsedEbook {
    /// Creates a new parsed e-book with the given metadata.
    #[inline]
    #[must_use = "creates empty ebook structure"]
    pub const fn new(metadata: EbookMetadata) -> Self {
        Self {
            metadata,
            body_title: None,
            chapters: Vec::new(),
            toc: Vec::new(),
            page_list: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ebook_metadata_default() {
        let default = EbookMetadata::default();
        let new = EbookMetadata::new();
        assert_eq!(default, new);
        assert!(default.title.is_none());
        assert!(default.creators.is_empty());
        assert!(default.language.is_none());
        assert!(default.subjects.is_empty());
    }
}
