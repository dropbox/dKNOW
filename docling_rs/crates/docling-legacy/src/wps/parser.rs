//! Parser for Kingsoft WPS Writer (.wps) documents using `LibreOffice`
//!
//! This module converts WPS files to DOCX format using `LibreOffice`'s
//! headless mode.

use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

/// Backend for Kingsoft WPS Writer document parsing
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct WpsBackend;

impl WpsBackend {
    /// Convert a WPS file to DOCX format using `LibreOffice`
    ///
    /// ## Arguments
    ///
    /// * `path` - Path to the .wps file
    ///
    /// ## Errors
    ///
    /// Returns an error if:
    /// - File does not exist or is not a .wps file
    /// - `LibreOffice` (`soffice`) is not installed
    /// - Conversion fails
    #[must_use = "conversion produces a result that should be handled"]
    pub fn convert_to_docx(path: &Path) -> Result<Vec<u8>> {
        // Verify file exists
        if !path.exists() {
            anyhow::bail!("File not found: {}", path.display());
        }

        // Verify file extension
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext.to_lowercase() != "wps" {
            anyhow::bail!("Expected .wps file, got: .{ext}");
        }

        // Create temp directory for output
        let temp_dir = TempDir::new().context("Failed to create temporary directory")?;

        // Run LibreOffice conversion
        let output = Command::new("soffice")
            .arg("--headless")
            .arg("--convert-to")
            .arg("docx:MS Word 2007 XML")
            .arg("--outdir")
            .arg(temp_dir.path())
            .arg(path)
            .output()
            .context(
                "Failed to execute LibreOffice (soffice). \
                 Ensure LibreOffice is installed and accessible.",
            )?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("LibreOffice conversion failed: {stderr}");
        }

        // Find the output DOCX file
        let input_stem = path
            .file_stem()
            .context("Invalid input filename")?
            .to_string_lossy();
        let docx_path = temp_dir.path().join(format!("{input_stem}.docx"));

        if !docx_path.exists() {
            anyhow::bail!("Converted DOCX not found at {}", docx_path.display());
        }

        // Read DOCX bytes
        let docx_bytes = std::fs::read(&docx_path).context("Failed to read converted DOCX")?;

        Ok(docx_bytes)
    }

    /// Check if `LibreOffice` is available
    #[must_use = "checks if LibreOffice is installed"]
    pub fn is_libreoffice_available() -> bool {
        Command::new("soffice")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_convert_file_not_found() {
        let path = Path::new("/nonexistent/path/document.wps");
        let result = WpsBackend::convert_to_docx(path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("File not found"));
    }

    #[test]
    fn test_convert_wrong_extension() {
        let mut temp = NamedTempFile::with_suffix(".txt").unwrap();
        temp.write_all(b"test content").unwrap();
        temp.flush().unwrap();

        let result = WpsBackend::convert_to_docx(temp.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Expected .wps"));
    }

    #[test]
    fn test_libreoffice_available_check() {
        // Just verify the check doesn't panic
        let available = WpsBackend::is_libreoffice_available();
        println!("LibreOffice available: {available}");
    }

    #[test]
    fn test_convert_invalid_wps() {
        // Create temp file with .wps extension but invalid content
        let mut temp = NamedTempFile::with_suffix(".wps").unwrap();
        temp.write_all(b"not a valid wps file").unwrap();
        temp.flush().unwrap();

        // Only test if LibreOffice is available
        if WpsBackend::is_libreoffice_available() {
            let result = WpsBackend::convert_to_docx(temp.path());
            // LibreOffice may error or produce empty file for invalid input
            let _ = result;
        }
    }
}
