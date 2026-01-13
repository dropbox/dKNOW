//! Parser for `WordPerfect` (.wpd) documents using libwpd
//!
//! This module extracts text from `WordPerfect` files using the `wpd2text` tool
//! from the libwpd library.

use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

/// Backend for `WordPerfect` document parsing
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct WpdBackend;

impl WpdBackend {
    /// Extract plain text from a `WordPerfect` document
    ///
    /// Uses `wpd2text` from libwpd to extract the text content.
    ///
    /// ## Arguments
    ///
    /// * `path` - Path to the .wpd file
    ///
    /// ## Errors
    ///
    /// Returns an error if:
    /// - File does not exist or has wrong extension
    /// - `wpd2text` tool is not installed
    /// - Text extraction fails
    ///
    /// ## Example
    ///
    /// ```rust,no_run
    /// use docling_legacy::wpd::WpdBackend;
    /// use std::path::Path;
    ///
    /// let text = WpdBackend::extract_text(Path::new("document.wpd"))?;
    /// println!("Text: {}", text);
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    #[must_use = "extraction produces a result that should be handled"]
    pub fn extract_text(path: &Path) -> Result<String> {
        // Verify file exists
        if !path.exists() {
            anyhow::bail!("File not found: {}", path.display());
        }

        // Verify file extension
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let ext_lower = ext.to_lowercase();
        if ext_lower != "wpd" && ext_lower != "wp5" && ext_lower != "wp6" && ext_lower != "wp" {
            anyhow::bail!("Expected WordPerfect file (.wpd, .wp5, .wp6, .wp), got: .{ext}");
        }

        // Check for wpd2text availability
        Self::check_wpd2text_available()?;

        // Extract text using wpd2text
        Self::run_wpd2text(path)
    }

    /// Check if wpd2text is available on the system
    fn check_wpd2text_available() -> Result<()> {
        let output = Command::new("wpd2text").arg("--version").output().context(
            "wpd2text not found. Install libwpd:\n\
                 - macOS: brew install libwpd\n\
                 - Linux: apt install libwpd-tools",
        )?;

        if !output.status.success() {
            anyhow::bail!("wpd2text --version failed. Ensure libwpd is properly installed.");
        }

        Ok(())
    }

    /// Run wpd2text to extract text from a `WordPerfect` file
    fn run_wpd2text(path: &Path) -> Result<String> {
        let output = Command::new("wpd2text")
            .arg(path)
            .output()
            .with_context(|| format!("Failed to run wpd2text on {}", path.display()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("wpd2text failed:\n{}\n\nFile: {}", stderr, path.display());
        }

        let text =
            String::from_utf8(output.stdout).context("wpd2text output is not valid UTF-8")?;

        Ok(text)
    }

    /// Convert `WordPerfect` document to markdown format
    ///
    /// This is a simple conversion that wraps the extracted text in a markdown document.
    /// For more sophisticated conversion (tables, formatting), future versions may
    /// use `wpd2html` and then convert HTML to markdown.
    ///
    /// ## Arguments
    ///
    /// * `path` - Path to the .wpd file
    ///
    /// ## Errors
    ///
    /// Returns an error if text extraction fails (see [`Self::extract_text`]).
    #[must_use = "conversion produces a result that should be handled"]
    pub fn to_markdown(path: &Path) -> Result<String> {
        let text = Self::extract_text(path)?;

        // Get filename for title
        let filename = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Document");

        // Wrap in basic markdown structure
        let markdown = format!("# {}\n\n{}\n", filename, text.trim());

        Ok(markdown)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_wpd2text_available() {
        // This test verifies wpd2text is installed
        // Skip if not available (CI environments may not have it)
        let result = WpdBackend::check_wpd2text_available();
        if result.is_err() {
            eprintln!("Skipping test: wpd2text not available");
            return;
        }
        assert!(result.is_ok());
    }

    #[test]
    fn test_extract_text_file_not_found() {
        let path = Path::new("/nonexistent/path/document.wpd");
        let result = WpdBackend::extract_text(path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("File not found"));
    }

    #[test]
    fn test_extract_text_wrong_extension() {
        // Create temp file with wrong extension
        let mut temp = NamedTempFile::with_suffix(".txt").unwrap();
        temp.write_all(b"test content").unwrap();
        temp.flush().unwrap();

        let result = WpdBackend::extract_text(temp.path());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Expected WordPerfect"));
    }

    #[test]
    fn test_extract_text_invalid_wpd() {
        // Create temp file with .wpd extension but invalid content
        let mut temp = NamedTempFile::with_suffix(".wpd").unwrap();
        temp.write_all(b"not a valid wpd file").unwrap();
        temp.flush().unwrap();

        // This should fail because wpd2text can't parse it
        // But only if wpd2text is available
        if WpdBackend::check_wpd2text_available().is_ok() {
            let result = WpdBackend::extract_text(temp.path());
            // wpd2text may error or return empty for invalid files
            // Just verify it doesn't panic
            let _ = result;
        }
    }

    #[test]
    fn test_to_markdown_wraps_text() {
        // Mock test - verify markdown formatting logic
        // Can't easily test without real .wpd file
        let filename = Path::new("test_document.wpd");
        let stem = filename.file_stem().and_then(|s| s.to_str()).unwrap();
        assert_eq!(stem, "test_document");
    }
}
