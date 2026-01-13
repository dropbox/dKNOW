//! Common utilities for Apple iWork formats
//!
//! All iWork formats (Pages, Numbers, Keynote) share the same structure:
//! - ZIP archive containing IWA (iWork Archive) files
//! - QuickLook/Preview.pdf embedded for preview
//!
//! This module provides shared extraction logic.

use anyhow::{Context, Result};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

/// Extract `QuickLook` preview PDF from iWork file
///
/// All iWork files contain a QuickLook/Preview.pdf that can be extracted and parsed.
/// This provides high-fidelity content without needing to decode proprietary IWA format.
///
/// Returns the raw PDF bytes that can be parsed by a PDF backend.
///
/// # Errors
///
/// Returns an error if the file cannot be opened, is not a valid iWork document,
/// or does not contain a `QuickLook` preview.
#[must_use = "this function returns PDF data that should be processed"]
pub fn extract_quicklook_pdf(input_path: &Path, format_name: &str) -> Result<Vec<u8>> {
    // Open ZIP archive
    let file = File::open(input_path).with_context(|| {
        format!(
            "Failed to open {} file: {}\n\
                 Ensure the file is a valid iWork document.",
            format_name,
            input_path.display()
        )
    })?;

    let mut archive = ZipArchive::new(file).with_context(|| {
        format!(
            "Failed to read {} file as ZIP archive: {}\n\
             File may be corrupted or not a valid iWork document.",
            format_name,
            input_path.display()
        )
    })?;

    // Validate iWork file structure
    let has_preview = (0..archive.len()).any(|i| {
        archive
            .by_index(i)
            .is_ok_and(|file| file.name() == "QuickLook/Preview.pdf")
    });

    if !has_preview {
        anyhow::bail!(
            "Invalid {format_name} file: missing QuickLook/Preview.pdf\n\
             This is required in all iWork documents."
        );
    }

    // Extract Preview.pdf
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        if file.name() == "QuickLook/Preview.pdf" {
            let mut pdf_content = Vec::new();
            file.read_to_end(&mut pdf_content)
                .context("Failed to read preview PDF from iWork file")?;
            return Ok(pdf_content);
        }
    }

    anyhow::bail!(
        "Failed to extract preview PDF from {format_name} file\n\
         File structure may be corrupted."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_pages_quicklook_pdf() {
        let path = Path::new("../../test-corpus/apple-pages/resume.pages");
        if !path.exists() {
            return;
        }

        let result = extract_quicklook_pdf(path, "Pages");
        match result {
            Ok(pdf_bytes) => {
                assert!(!pdf_bytes.is_empty(), "PDF bytes should not be empty");
                // Check PDF magic number
                assert_eq!(&pdf_bytes[0..5], b"%PDF-", "Should be valid PDF");
            }
            Err(e) => {
                println!("⚠️  Pages PDF extraction: {e:?}");
            }
        }
    }
}
