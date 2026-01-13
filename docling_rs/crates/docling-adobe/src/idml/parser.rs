/// IDML parser for `InDesign` Markup Language files
use std::fs::File;
use std::io::Read;
use std::path::Path;

use quick_xml::events::Event;
use quick_xml::Reader;
use zip::ZipArchive;

use super::types::{IdmlDocument, Metadata, Paragraph, Story};
use crate::error::{IdmlError, Result};

/// Parser for IDML (`InDesign` Markup Language) documents
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct IdmlParser;

/// Represents parsed designmap.xml structure
#[derive(Debug, Clone, PartialEq, Eq)]
struct DesignMap {
    story_paths: Vec<String>,
    metadata: Metadata,
}

impl IdmlParser {
    /// Parse an IDML file from a path
    ///
    /// # Arguments
    /// * `path` - Path to the .idml file
    ///
    /// # Returns
    /// * `Result<IdmlDocument>` - Parsed IDML document or error
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file cannot be opened (`IdmlError::IoError`)
    /// - The file is not a valid ZIP archive (`IdmlError::ParseError`)
    #[must_use = "parsing produces a result that should be handled"]
    pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<IdmlDocument> {
        let file = File::open(path.as_ref())
            .map_err(|e| IdmlError::IoError(format!("Failed to open IDML file: {e}")))?;

        Self::parse_archive(file)
    }

    /// Parse an IDML archive from a file
    fn parse_archive(file: File) -> Result<IdmlDocument> {
        let mut archive = ZipArchive::new(file)
            .map_err(|e| IdmlError::ParseError(format!("Failed to read ZIP archive: {e}")))?;

        // Parse designmap.xml to get story paths and metadata
        let designmap = Self::parse_designmap(&mut archive)?;

        // Create document with metadata
        let mut document = IdmlDocument::with_metadata(designmap.metadata);

        // Parse each story
        for story_path in designmap.story_paths {
            match Self::parse_story(&mut archive, &story_path) {
                Ok(story) => document.add_story(story),
                Err(e) => {
                    // Log warning but continue with other stories
                    log::warn!("Failed to parse IDML story {story_path}: {e}");
                }
            }
        }

        Ok(document)
    }

    /// Parse designmap.xml to extract story paths and metadata
    fn parse_designmap(archive: &mut ZipArchive<File>) -> Result<DesignMap> {
        let mut file = archive
            .by_name("designmap.xml")
            .map_err(|e| IdmlError::ParseError(format!("designmap.xml not found: {e}")))?;

        let mut content = String::new();
        file.read_to_string(&mut content)
            .map_err(|e| IdmlError::IoError(format!("Failed to read designmap.xml: {e}")))?;

        let mut reader = Reader::from_str(&content);
        reader.trim_text(true);

        let mut story_paths = Vec::new();
        let mut metadata = Metadata::default();
        let mut in_title = false;
        let mut in_creator = false;

        loop {
            match reader.read_event() {
                Ok(Event::Start(e) | Event::Empty(e)) => {
                    match e.name().as_ref() {
                        b"idPkg:Story" => {
                            // Extract src attribute
                            for attr in e.attributes().flatten() {
                                if attr.key.as_ref() == b"src" {
                                    if let Ok(value) = std::str::from_utf8(&attr.value) {
                                        story_paths.push(value.to_string());
                                    }
                                }
                            }
                        }
                        b"Title" => in_title = true,
                        b"Creator" => in_creator = true,
                        _ => {}
                    }
                }
                Ok(Event::Text(e)) => {
                    let text = e.unescape().unwrap_or_default().to_string();
                    if in_title && !text.trim().is_empty() {
                        metadata.title = Some(text.trim().to_string());
                    } else if in_creator && !text.trim().is_empty() {
                        metadata.author = Some(text.trim().to_string());
                    }
                }
                Ok(Event::End(e)) => match e.name().as_ref() {
                    b"Title" => in_title = false,
                    b"Creator" => in_creator = false,
                    _ => {}
                },
                Ok(Event::Eof) => break,
                Err(e) => {
                    return Err(IdmlError::ParseError(format!(
                        "XML parse error in designmap.xml: {e}"
                    )))
                }
                _ => {}
            }
        }

        Ok(DesignMap {
            story_paths,
            metadata,
        })
    }

    /// Parse a Story XML file to extract paragraphs
    fn parse_story(archive: &mut ZipArchive<File>, story_path: &str) -> Result<Story> {
        let mut file = archive.by_name(story_path).map_err(|e| {
            IdmlError::ParseError(format!("Story file {story_path} not found: {e}"))
        })?;

        let mut content = String::new();
        file.read_to_string(&mut content).map_err(|e| {
            IdmlError::IoError(format!("Failed to read story file {story_path}: {e}"))
        })?;

        // Extract story ID from path (e.g., "Stories/Story_u1000.xml" -> "u1000")
        let story_id = story_path
            .rsplit('_')
            .next()
            .and_then(|s| s.strip_suffix(".xml"))
            .unwrap_or("unknown")
            .to_string();

        let mut reader = Reader::from_str(&content);
        reader.trim_text(true);

        let mut story = Story::new(story_id);
        let mut current_paragraph: Option<(Option<String>, String)> = None;
        let mut in_content = false;
        let mut current_style: Option<String>;

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) => {
                    match e.name().as_ref() {
                        b"ParagraphStyleRange" => {
                            // Save previous paragraph if exists
                            if let Some((style, text)) = current_paragraph.take() {
                                if !text.trim().is_empty() {
                                    let para = style.map_or_else(
                                        || Paragraph::new(text.trim().to_string()),
                                        |s| Paragraph::with_style(s, text.trim().to_string()),
                                    );
                                    story.add_paragraph(para);
                                }
                            }

                            // Extract AppliedParagraphStyle attribute
                            current_style = None;
                            for attr in e.attributes().flatten() {
                                if attr.key.as_ref() == b"AppliedParagraphStyle" {
                                    if let Ok(value) = std::str::from_utf8(&attr.value) {
                                        // Extract style name (e.g., "ParagraphStyle/Heading1" -> "Heading1")
                                        let style_name = value
                                            .split('/')
                                            .next_back()
                                            .unwrap_or(value)
                                            .to_string();
                                        current_style = Some(style_name);
                                    }
                                }
                            }

                            // Start new paragraph
                            current_paragraph = Some((current_style.clone(), String::new()));
                        }
                        b"Content" => {
                            in_content = true;
                        }
                        _ => {}
                    }
                }
                Ok(Event::Text(e)) => {
                    if in_content {
                        let text = e.unescape().unwrap_or_default().to_string();
                        if let Some((_, ref mut para_text)) = current_paragraph {
                            para_text.push_str(&text);
                        }
                    }
                }
                Ok(Event::End(e)) => {
                    if e.name().as_ref() == b"Content" {
                        in_content = false;
                    }
                }
                Ok(Event::Eof) => {
                    // Save last paragraph if exists
                    if let Some((style, text)) = current_paragraph.take() {
                        if !text.trim().is_empty() {
                            let para = style.map_or_else(
                                || Paragraph::new(text.trim().to_string()),
                                |s| Paragraph::with_style(s, text.trim().to_string()),
                            );
                            story.add_paragraph(para);
                        }
                    }
                    break;
                }
                Err(e) => {
                    return Err(IdmlError::ParseError(format!(
                        "XML parse error in {story_path}: {e}"
                    )))
                }
                _ => {}
            }
        }

        Ok(story)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_document() {
        // This test requires the generated test file
        let test_file = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../test-corpus/adobe/idml/simple_document.idml"
        );

        if std::path::Path::new(test_file).exists() {
            let result = IdmlParser::parse_file(test_file);
            assert!(result.is_ok(), "Failed to parse simple_document.idml");

            let doc = result.unwrap();
            assert_eq!(doc.metadata.title, Some("Simple Letter".to_string()));
            assert_eq!(doc.metadata.author, Some("John Doe".to_string()));
            assert!(!doc.stories.is_empty(), "Document should have stories");

            // Check first story has paragraphs
            let story = &doc.stories[0];
            assert!(!story.paragraphs.is_empty(), "Story should have paragraphs");

            // Check heading
            let first_para = &story.paragraphs[0];
            assert_eq!(first_para.text, "Business Letter");
            assert_eq!(first_para.style, Some("Heading1".to_string()));
        }
    }

    #[test]
    fn test_parse_magazine_layout() {
        let test_file = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../test-corpus/adobe/idml/magazine_layout.idml"
        );

        if std::path::Path::new(test_file).exists() {
            let result = IdmlParser::parse_file(test_file);
            assert!(result.is_ok(), "Failed to parse magazine_layout.idml");

            let doc = result.unwrap();
            assert_eq!(
                doc.metadata.title,
                Some("Tech Magazine: AI Revolution".to_string())
            );
            // Magazine has 2 stories
            assert_eq!(doc.stories.len(), 2, "Magazine should have 2 stories");
        }
    }

    #[test]
    fn test_parse_technical_manual() {
        let test_file = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../test-corpus/adobe/idml/technical_manual.idml"
        );

        if std::path::Path::new(test_file).exists() {
            let result = IdmlParser::parse_file(test_file);
            assert!(result.is_ok(), "Failed to parse technical_manual.idml");

            let doc = result.unwrap();
            assert!(doc.metadata.title.is_some());
            assert!(!doc.stories.is_empty());
        }
    }
}
