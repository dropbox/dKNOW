//! Legacy document format backend for docling-core
//!
//! Processes legacy document formats (RTF, DOC, `WordPerfect`, WPS) into markdown documents.

use crate::error::{DoclingError, Result};
use std::path::Path;

/// Process an RTF file into markdown
///
/// Extracts text content from Rich Text Format (.rtf) files.
///
/// # Arguments
///
/// * `path` - Path to the RTF file
///
/// # Returns
///
/// Returns markdown document with extracted text.
///
/// # Errors
///
/// Returns an error if the file cannot be read or if RTF parsing fails.
///
/// # Examples
///
/// ```no_run
/// use docling_core::legacy::process_rtf;
///
/// let markdown = process_rtf("document.rtf")?;
/// println!("{}", markdown);
/// # Ok::<(), docling_core::error::DoclingError>(())
/// ```
#[must_use = "this function returns the extracted markdown content"]
pub fn process_rtf<P: AsRef<Path>>(path: P) -> Result<String> {
    // Parse RTF file using docling-legacy
    let doc = docling_legacy::RtfParser::parse_file(&path)
        .map_err(|e| DoclingError::ConversionError(format!("Failed to parse RTF file: {e}")))?;

    // Convert to markdown
    let markdown = docling_legacy::rtf_to_markdown(&doc);

    Ok(markdown)
}
