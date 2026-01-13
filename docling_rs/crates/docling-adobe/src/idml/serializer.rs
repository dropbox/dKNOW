/// IDML serializer for converting IDML documents to markdown
use super::types::{IdmlDocument, Paragraph};
use std::fmt::Write;

/// Serializer for IDML documents to markdown format
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct IdmlSerializer;

impl IdmlSerializer {
    /// Convert an IDML document to markdown
    ///
    /// # Arguments
    /// * `doc` - The IDML document to serialize
    ///
    /// # Returns
    /// * `String` - Markdown representation of the document
    #[must_use = "returns the markdown representation of the document"]
    pub fn to_markdown(doc: &IdmlDocument) -> String {
        let mut markdown = String::new();

        // Add metadata header if present
        if doc.metadata.title.is_some() || doc.metadata.author.is_some() {
            markdown.push_str("---\n");

            if let Some(ref title) = doc.metadata.title {
                let _ = writeln!(markdown, "title: {title}");
            }

            if let Some(ref author) = doc.metadata.author {
                let _ = writeln!(markdown, "author: {author}");
            }

            markdown.push_str("---\n\n");
        }

        // Serialize each story
        for story in &doc.stories {
            // Serialize paragraphs
            for paragraph in &story.paragraphs {
                let para_md = Self::paragraph_to_markdown(paragraph);
                markdown.push_str(&para_md);
                markdown.push_str("\n\n");
            }
        }

        // Trim trailing whitespace
        markdown.trim_end().to_string()
    }

    /// Convert a paragraph to markdown based on its style
    #[inline]
    fn paragraph_to_markdown(paragraph: &Paragraph) -> String {
        match paragraph.style.as_deref() {
            Some("Heading1") => format!("# {}", paragraph.text),
            Some("Heading2") => format!("## {}", paragraph.text),
            Some("Heading3") => format!("### {}", paragraph.text),
            Some("Heading4") => format!("#### {}", paragraph.text),
            Some("Heading5") => format!("##### {}", paragraph.text),
            Some("Heading6") => format!("###### {}", paragraph.text),
            _ => paragraph.text.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::idml::types::{IdmlDocument, Metadata, Paragraph, Story};

    #[test]
    fn test_basic_markdown_generation() {
        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        story.add_paragraph(Paragraph::new("Simple paragraph".to_string()));
        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);
        assert_eq!(markdown, "Simple paragraph");
    }

    #[test]
    fn test_markdown_with_metadata() {
        let metadata = Metadata {
            title: Some("Test Document".to_string()),
            author: Some("Test Author".to_string()),
        };

        let mut doc = IdmlDocument::with_metadata(metadata);
        let mut story = Story::new("u1000".to_string());
        story.add_paragraph(Paragraph::new("Content".to_string()));
        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);
        assert!(markdown.contains("---"));
        assert!(markdown.contains("title: Test Document"));
        assert!(markdown.contains("author: Test Author"));
        assert!(markdown.contains("Content"));
    }

    #[test]
    fn test_heading_styles() {
        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        story.add_paragraph(Paragraph::with_style(
            "Heading1".to_string(),
            "Main Heading".to_string(),
        ));
        story.add_paragraph(Paragraph::with_style(
            "Heading2".to_string(),
            "Subheading".to_string(),
        ));
        story.add_paragraph(Paragraph::new("Body text".to_string()));

        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);
        assert!(markdown.contains("# Main Heading"));
        assert!(markdown.contains("## Subheading"));
        assert!(markdown.contains("Body text"));
    }

    #[test]
    fn test_multiple_paragraphs() {
        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        story.add_paragraph(Paragraph::new("First paragraph".to_string()));
        story.add_paragraph(Paragraph::new("Second paragraph".to_string()));
        story.add_paragraph(Paragraph::new("Third paragraph".to_string()));

        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);
        let paragraphs: Vec<&str> = markdown.split("\n\n").collect();
        assert_eq!(paragraphs.len(), 3);
        assert_eq!(paragraphs[0], "First paragraph");
        assert_eq!(paragraphs[1], "Second paragraph");
        assert_eq!(paragraphs[2], "Third paragraph");
    }

    #[test]
    fn test_multiple_stories() {
        let mut doc = IdmlDocument::new();

        let mut story1 = Story::new("u1000".to_string());
        story1.add_paragraph(Paragraph::new("Story 1 content".to_string()));
        doc.add_story(story1);

        let mut story2 = Story::new("u2000".to_string());
        story2.add_paragraph(Paragraph::new("Story 2 content".to_string()));
        doc.add_story(story2);

        let markdown = IdmlSerializer::to_markdown(&doc);
        assert!(markdown.contains("Story 1 content"));
        assert!(markdown.contains("Story 2 content"));
    }
}
