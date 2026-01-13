/// Types for IDML (`InDesign` Markup Language) document representation
use serde::{Deserialize, Serialize};

/// Represents a complete IDML document
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IdmlDocument {
    /// Document metadata
    pub metadata: Metadata,
    /// List of stories (text flows) in the document
    pub stories: Vec<Story>,
}

/// Document metadata extracted from designmap.xml
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Metadata {
    /// Document title
    pub title: Option<String>,
    /// Document author/creator
    pub author: Option<String>,
}

/// Represents a story (text flow) in IDML
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Story {
    /// Story ID (e.g., "u1000")
    pub id: String,
    /// List of paragraphs in reading order
    pub paragraphs: Vec<Paragraph>,
}

/// Represents a paragraph with style and text content
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Paragraph {
    /// Paragraph style name (e.g., "Heading1", "`BodyText`")
    pub style: Option<String>,
    /// Text content of the paragraph
    pub text: String,
}

impl IdmlDocument {
    /// Create a new empty IDML document
    #[inline]
    #[must_use = "creates a new empty IDML document"]
    pub fn new() -> Self {
        Self {
            metadata: Metadata::default(),
            stories: Vec::new(),
        }
    }

    /// Create a new IDML document with metadata
    #[inline]
    #[must_use = "creates a new IDML document with the provided metadata"]
    pub const fn with_metadata(metadata: Metadata) -> Self {
        Self {
            metadata,
            stories: Vec::new(),
        }
    }

    /// Add a story to the document
    #[inline]
    pub fn add_story(&mut self, story: Story) {
        self.stories.push(story);
    }
}

impl Story {
    /// Create a new story with ID
    #[inline]
    #[must_use = "creates a new story with the given ID"]
    pub const fn new(id: String) -> Self {
        Self {
            id,
            paragraphs: Vec::new(),
        }
    }

    /// Add a paragraph to the story
    #[inline]
    pub fn add_paragraph(&mut self, paragraph: Paragraph) {
        self.paragraphs.push(paragraph);
    }
}

impl Paragraph {
    /// Create a new paragraph with text
    #[inline]
    #[must_use = "creates a new paragraph with the given text"]
    pub const fn new(text: String) -> Self {
        Self { style: None, text }
    }

    /// Create a new paragraph with style and text
    #[inline]
    #[must_use = "creates a new paragraph with the given style and text"]
    pub const fn with_style(style: String, text: String) -> Self {
        Self {
            style: Some(style),
            text,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_idml_document_creation() {
        let doc = IdmlDocument::new();
        assert!(doc.stories.is_empty());
        assert!(doc.metadata.title.is_none());
        assert!(doc.metadata.author.is_none());
    }

    #[test]
    fn test_idml_document_with_metadata() {
        let metadata = Metadata {
            title: Some("Test Document".to_string()),
            author: Some("Test Author".to_string()),
        };
        let doc = IdmlDocument::with_metadata(metadata);
        assert_eq!(doc.metadata.title, Some("Test Document".to_string()));
        assert_eq!(doc.metadata.author, Some("Test Author".to_string()));
    }

    #[test]
    fn test_story_creation() {
        let mut story = Story::new("u1000".to_string());
        assert_eq!(story.id, "u1000");
        assert!(story.paragraphs.is_empty());

        let para = Paragraph::new("Test paragraph".to_string());
        story.add_paragraph(para);
        assert_eq!(story.paragraphs.len(), 1);
    }

    #[test]
    fn test_paragraph_with_style() {
        let para = Paragraph::with_style("Heading1".to_string(), "Test heading".to_string());
        assert_eq!(para.style, Some("Heading1".to_string()));
        assert_eq!(para.text, "Test heading");
    }

    #[test]
    fn test_idml_document_default() {
        let default = IdmlDocument::default();
        let new = IdmlDocument::new();
        assert_eq!(default, new);
    }
}
