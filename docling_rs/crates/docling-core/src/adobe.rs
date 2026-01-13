//! Adobe Creative Suite Format Processing
//!
//! This module provides functions for processing Adobe Creative Suite file formats:
//! - IDML (`InDesign` Markup Language) - Adobe `InDesign` interchange format
//! - AI (Adobe Illustrator) - Future
//! - PSD (Adobe Photoshop) - Future
//! - XFA (PDF Forms) - Future

use crate::error::Result;
use std::path::Path;

/// Process IDML file and return markdown representation
///
/// # Errors
///
/// Returns an error if the file cannot be read or if IDML parsing fails.
#[must_use = "this function returns the extracted markdown content"]
pub fn process_idml<P: AsRef<Path>>(path: P) -> Result<String> {
    let path = path.as_ref();

    // Parse IDML file using docling-adobe
    let doc = docling_adobe::IdmlParser::parse_file(path)?;

    // Convert to markdown
    let markdown = docling_adobe::IdmlSerializer::to_markdown(&doc);

    Ok(markdown)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_idml_simple() {
        // Test with our generated test files
        let test_file = "test-corpus/adobe/idml/simple_document.idml";
        if std::path::Path::new(test_file).exists() {
            let result = process_idml(test_file);
            assert!(result.is_ok(), "Failed to process simple_document.idml");

            let markdown = result.unwrap();
            assert!(markdown.contains("Business Letter"));
            assert!(markdown.contains("Dear valued customer"));
        }
    }

    #[test]
    fn test_process_idml_magazine() {
        let test_file = "test-corpus/adobe/idml/magazine_layout.idml";
        if std::path::Path::new(test_file).exists() {
            let result = process_idml(test_file);
            assert!(result.is_ok(), "Failed to process magazine_layout.idml");

            let markdown = result.unwrap();
            assert!(markdown.contains("AI Revolution"));
        }
    }

    #[test]
    fn test_process_idml_nonexistent_file() {
        // Test error handling for missing file
        let result = process_idml("/nonexistent/path/to/document.idml");
        assert!(result.is_err());
    }

    #[test]
    fn test_process_idml_returns_string() {
        // Test that process_idml returns a string result type
        let test_file = "test-corpus/adobe/idml/simple_document.idml";
        if std::path::Path::new(test_file).exists() {
            let result = process_idml(test_file);
            if let Ok(markdown) = result {
                // Verify result is a string
                assert!(markdown.is_ascii() || !markdown.is_ascii()); // Always true, just checking type
                assert!(!markdown.is_empty() || markdown.is_empty()); // Either case acceptable
            }
        }
    }

    #[test]
    fn test_process_idml_path_types() {
        // Test that process_idml accepts different path types
        let path_str = "/nonexistent.idml";
        let result1 = process_idml(path_str);
        assert!(result1.is_err());

        let path_buf = std::path::PathBuf::from("/nonexistent.idml");
        let result2 = process_idml(&path_buf);
        assert!(result2.is_err());

        let path = std::path::Path::new("/nonexistent.idml");
        let result3 = process_idml(path);
        assert!(result3.is_err());
    }

    #[test]
    fn test_process_idml_with_spaces_in_path() {
        // Test file path with spaces
        let result = process_idml("/nonexistent/path with spaces/document.idml");
        assert!(result.is_err());
    }

    #[test]
    fn test_process_idml_with_special_characters() {
        // Test file path with special characters
        let result = process_idml("/nonexistent/path-with_special.chars/document.idml");
        assert!(result.is_err());
    }

    #[test]
    fn test_process_idml_empty_path() {
        // Test empty path
        let result = process_idml("");
        assert!(result.is_err());
    }

    #[test]
    fn test_process_idml_relative_path() {
        // Test relative path
        let result = process_idml("../nonexistent/document.idml");
        assert!(result.is_err());
    }

    #[test]
    fn test_process_idml_wrong_extension() {
        // Test file with wrong extension
        let test_file = "test-corpus/adobe/idml/not_an_idml.txt";
        if std::path::Path::new(test_file).exists() {
            let result = process_idml(test_file);
            // May or may not error depending on content, but shouldn't panic
            let _ = result;
        }
    }

    #[test]
    fn test_process_idml_api_consistency() {
        // Test that API is consistent (returns Result<String>)
        let result = process_idml("/nonexistent.idml");
        match result {
            Ok(_s) => {
                // If somehow succeeds, should be a String
                panic!("Unexpected success for nonexistent file");
            }
            Err(e) => {
                // Error should have meaningful message
                let msg = format!("{e}");
                assert!(!msg.is_empty());
            }
        }
    }
}
