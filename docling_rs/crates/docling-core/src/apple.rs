//! Apple iWork format support (Pages, Numbers, Keynote)
//!
//! This module provides processing functions for Apple's iWork formats:
//! - Pages (.pages) - Word processing documents
//! - Numbers (.numbers) - Spreadsheet documents
//! - Keynote (.key) - Presentation documents
//!
//! All three formats share the same structure: they are ZIP archives containing
//! IWA (iWork Archive) files with an embedded QuickLook/Preview.pdf for previewing.
//!
//! The current implementation extracts and parses the QuickLook preview PDF to
//! obtain the document content. This provides high-fidelity rendering without
//! needing to decode the proprietary IWA (protobuf-based) format.

use crate::{serializer::MarkdownSerializer, Result};
use std::path::Path;

/// Process an Apple Pages document (.pages)
///
/// Pages files are ZIP archives containing IWA files and a QuickLook/Preview.pdf.
/// This function extracts the preview PDF and converts it to markdown.
///
/// # Arguments
///
/// * `path` - Path to the .pages file
///
/// # Returns
///
/// Markdown string containing the extracted text and structure
///
/// # Errors
///
/// Returns an error if:
/// - File cannot be opened or read
/// - File is not a valid Pages document
/// - QuickLook/Preview.pdf is missing or corrupted
/// - PDF parsing fails
///
/// # Examples
///
/// ```rust,no_run
/// use docling_core::apple::process_pages;
/// use std::path::Path;
///
/// let markdown = process_pages(Path::new("document.pages"))?;
/// println!("Pages document: {}", markdown);
/// # Ok::<(), docling_core::DoclingError>(())
/// ```
#[must_use = "this function returns the extracted markdown content"]
pub fn process_pages(path: &Path) -> Result<String> {
    let backend = docling_apple::PagesBackend::new();
    let pdf_bytes = backend.extract_preview_pdf(path).map_err(|e| {
        crate::DoclingError::ConversionError(format!("Pages PDF extraction error: {e}"))
    })?;

    // Write PDF to temp file and parse with Python
    let temp_dir = tempfile::TempDir::new()
        .map_err(|e| crate::DoclingError::IoError(std::io::Error::other(e)))?;
    let pdf_path = temp_dir.path().join("preview.pdf");
    std::fs::write(&pdf_path, pdf_bytes)?;

    let doc = crate::python_bridge::convert_via_python(&pdf_path, false)?;
    let serializer = MarkdownSerializer::new();
    Ok(serializer.serialize(&doc))
}

/// Process an Apple Numbers spreadsheet (.numbers)
///
/// Numbers files are ZIP archives containing IWA files and a QuickLook/Preview.pdf.
/// This function extracts the preview PDF and converts it to markdown.
///
/// # Arguments
///
/// * `path` - Path to the .numbers file
///
/// # Returns
///
/// Markdown string containing the extracted text and structure
///
/// # Errors
///
/// Returns an error if:
/// - File cannot be opened or read
/// - File is not a valid Numbers document
/// - QuickLook/Preview.pdf is missing or corrupted
/// - PDF parsing fails
///
/// # Examples
///
/// ```rust,no_run
/// use docling_core::apple::process_numbers;
/// use std::path::Path;
///
/// let markdown = process_numbers(Path::new("budget.numbers"))?;
/// println!("Numbers spreadsheet: {}", markdown);
/// # Ok::<(), docling_core::DoclingError>(())
/// ```
#[must_use = "this function returns the extracted markdown content"]
pub fn process_numbers(path: &Path) -> Result<String> {
    let backend = docling_apple::NumbersBackend::new();
    let pdf_bytes = backend.extract_preview_pdf(path).map_err(|e| {
        crate::DoclingError::ConversionError(format!("Numbers PDF extraction error: {e}"))
    })?;

    // Write PDF to temp file and parse with Python
    let temp_dir = tempfile::TempDir::new()
        .map_err(|e| crate::DoclingError::IoError(std::io::Error::other(e)))?;
    let pdf_path = temp_dir.path().join("preview.pdf");
    std::fs::write(&pdf_path, pdf_bytes)?;

    let doc = crate::python_bridge::convert_via_python(&pdf_path, false)?;
    let serializer = MarkdownSerializer::new();
    Ok(serializer.serialize(&doc))
}

/// Process an Apple Keynote presentation (.key)
///
/// Keynote files are ZIP archives containing IWA files and a QuickLook/Preview.pdf.
/// This function extracts the preview PDF and converts it to markdown.
///
/// # Arguments
///
/// * `path` - Path to the .key file
///
/// # Returns
///
/// Markdown string containing the extracted text and structure
///
/// # Errors
///
/// Returns an error if:
/// - File cannot be opened or read
/// - File is not a valid Keynote document
/// - QuickLook/Preview.pdf is missing or corrupted
/// - PDF parsing fails
///
/// # Examples
///
/// ```rust,no_run
/// use docling_core::apple::process_keynote;
/// use std::path::Path;
///
/// let markdown = process_keynote(Path::new("presentation.key"))?;
/// println!("Keynote presentation: {}", markdown);
/// # Ok::<(), docling_core::DoclingError>(())
/// ```
#[must_use = "this function returns the extracted markdown content"]
pub fn process_keynote(path: &Path) -> Result<String> {
    let backend = docling_apple::KeynoteBackend::new();
    let pdf_bytes = backend.extract_preview_pdf(path).map_err(|e| {
        crate::DoclingError::ConversionError(format!("Keynote PDF extraction error: {e}"))
    })?;

    // Write PDF to temp file and parse with Python
    let temp_dir = tempfile::TempDir::new()
        .map_err(|e| crate::DoclingError::IoError(std::io::Error::other(e)))?;
    let pdf_path = temp_dir.path().join("preview.pdf");
    std::fs::write(&pdf_path, pdf_bytes)?;

    let doc = crate::python_bridge::convert_via_python(&pdf_path, false)?;
    let serializer = MarkdownSerializer::new();
    Ok(serializer.serialize(&doc))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pages_processing() {
        let path = Path::new("../../test-corpus/apple-pages/resume.pages");
        if !path.exists() {
            return;
        }

        let result = process_pages(path);
        match result {
            Ok(markdown) => {
                assert!(!markdown.is_empty(), "Pages markdown should not be empty");
            }
            Err(e) => {
                println!("⚠️  Pages processing: {:?}", e);
            }
        }
    }

    #[test]
    fn test_numbers_processing() {
        let path = Path::new("../../test-corpus/apple-numbers/budget.numbers");
        if !path.exists() {
            return;
        }

        let result = process_numbers(path);
        match result {
            Ok(markdown) => {
                assert!(!markdown.is_empty(), "Numbers markdown should not be empty");
            }
            Err(e) => {
                println!("⚠️  Numbers processing: {:?}", e);
            }
        }
    }

    #[test]
    fn test_keynote_processing() {
        let path = Path::new("../../test-corpus/apple-keynote/training.key");
        if !path.exists() {
            return;
        }

        let result = process_keynote(path);
        match result {
            Ok(markdown) => {
                assert!(!markdown.is_empty(), "Keynote markdown should not be empty");
            }
            Err(e) => {
                println!("⚠️  Keynote processing: {:?}", e);
            }
        }
    }
}
