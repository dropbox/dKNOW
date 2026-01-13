//! Microsoft Word 97-2003 (.doc) document processing
//!
//! This module handles .doc files by converting them to DOCX format first,
//! then processing the DOCX with the Python backend.

use crate::{DoclingError, Result};
use docling_legacy::DocBackend;
use std::path::{Path, PathBuf};

/// Convert a .doc file to .docx format for processing
///
/// This function converts .doc → .docx using platform-specific tools:
/// - **macOS:** Uses native `textutil` (built-in)
/// - **Linux/Windows:** Returns error with instructions (`LibreOffice` support coming)
///
/// Returns the path to the converted DOCX file (temporary file, caller should clean up).
///
/// # Errors
///
/// Returns an error if:
/// - The file cannot be read or has an invalid signature
/// - The conversion tool is not available on the platform
/// - The conversion process fails
#[must_use = "this function returns the path to the converted DOCX file"]
pub fn convert_doc_to_docx(path: &Path) -> Result<PathBuf> {
    DocBackend::convert_doc_to_docx(path)
        .map_err(|e| DoclingError::ConversionError(format!("DOC→DOCX conversion failed: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    #[cfg(target_os = "macos")]
    fn test_convert_doc_invalid_signature() {
        // Create temp file with invalid signature
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"NOT_A_DOC_FILE").unwrap();
        let temp_path = temp.path().with_extension("doc");
        std::fs::copy(temp.path(), &temp_path).unwrap();

        let result = convert_doc_to_docx(&temp_path);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid .doc file signature"));

        std::fs::remove_file(&temp_path).ok();
    }

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn test_convert_doc_platform_not_supported() {
        use std::path::PathBuf;
        let path = PathBuf::from("test.doc");
        let result = convert_doc_to_docx(&path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("macOS"));
    }

    #[test]
    fn test_convert_doc_nonexistent_file() {
        // Test error handling for missing file (works on all platforms)
        let result = convert_doc_to_docx(Path::new("/nonexistent/path/to/document.doc"));
        assert!(result.is_err());
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_convert_doc_empty_file() {
        // Test empty .doc file
        let temp = NamedTempFile::new().unwrap();
        let temp_path = temp.path().with_extension("doc");
        std::fs::copy(temp.path(), &temp_path).unwrap();

        let result = convert_doc_to_docx(&temp_path);
        assert!(result.is_err());

        std::fs::remove_file(&temp_path).ok();
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_convert_doc_wrong_extension() {
        // Test file with .doc extension but wrong content
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"Plain text file, not a DOC").unwrap();
        let temp_path = temp.path().with_extension("doc");
        std::fs::copy(temp.path(), &temp_path).unwrap();

        let result = convert_doc_to_docx(&temp_path);
        assert!(result.is_err());

        std::fs::remove_file(&temp_path).ok();
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_convert_doc_pdf_masquerading() {
        // Test PDF file with .doc extension (common misuse)
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"%PDF-1.4\n").unwrap();
        let temp_path = temp.path().with_extension("doc");
        std::fs::copy(temp.path(), &temp_path).unwrap();

        let result = convert_doc_to_docx(&temp_path);
        assert!(result.is_err());

        std::fs::remove_file(&temp_path).ok();
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_convert_doc_docx_masquerading() {
        // Test DOCX file with .doc extension
        let mut temp = NamedTempFile::new().unwrap();
        // DOCX files are ZIP archives starting with PK signature
        temp.write_all(b"PK\x03\x04").unwrap();
        let temp_path = temp.path().with_extension("doc");
        std::fs::copy(temp.path(), &temp_path).unwrap();

        let result = convert_doc_to_docx(&temp_path);
        // Should fail because it's not a valid .doc file
        assert!(result.is_err());

        std::fs::remove_file(&temp_path).ok();
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_convert_doc_path_with_spaces() {
        // Test file path with spaces
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"Invalid doc content").unwrap();
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("test file with spaces.doc");
        std::fs::copy(temp.path(), &temp_path).unwrap();

        let result = convert_doc_to_docx(&temp_path);
        // Should fail due to invalid content, but path handling should work
        assert!(result.is_err());

        std::fs::remove_file(&temp_path).ok();
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_convert_doc_special_characters_in_path() {
        // Test file path with special characters
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"Invalid doc content").unwrap();
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("test-file_v1.0.doc");
        std::fs::copy(temp.path(), &temp_path).unwrap();

        let result = convert_doc_to_docx(&temp_path);
        // Should fail due to invalid content, but path handling should work
        assert!(result.is_err());

        std::fs::remove_file(&temp_path).ok();
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_convert_doc_rtf_masquerading() {
        // Test RTF file with .doc extension
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"{\\rtf1\\ansi\\deff0").unwrap();
        let temp_path = temp.path().with_extension("doc");
        std::fs::copy(temp.path(), &temp_path).unwrap();

        let result = convert_doc_to_docx(&temp_path);
        // Should fail because it's not a valid .doc file
        assert!(result.is_err());

        std::fs::remove_file(&temp_path).ok();
    }

    #[test]
    fn test_convert_doc_function_signature() {
        // Test that convert_doc_to_docx returns correct types
        // This test doesn't actually convert, just checks API
        let result = convert_doc_to_docx(Path::new("/nonexistent.doc"));
        // Should return Result type
        assert!(result.is_err());
        // Error message should be informative
        let err = result.unwrap_err();
        let err_str = err.to_string();
        assert!(!err_str.is_empty());
    }
}
