//! Parser for Microsoft Word 97-2003 (.doc) binary format
//!
//! This module converts .doc files to .docx using platform-specific tools.
//! The caller is responsible for parsing the resulting DOCX file.

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::NamedTempFile;

/// CFB (Compound File Binary) / OLE2 magic signature
///
/// All OLE-based Microsoft Office formats (DOC, XLS, PPT) start with these 8 bytes.
/// The signature is `D0 CF 11 E0 A1 B1 1A E1` - a mnemonic for "DOC FILE".
const CFB_MAGIC_SIGNATURE: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];

/// Converter for Microsoft Word 97-2003 binary format
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct DocBackend;

impl DocBackend {
    /// Convert a .doc file to .docx format
    ///
    /// This function performs the conversion only. The caller is responsible
    /// for parsing the resulting DOCX file using the appropriate backend.
    ///
    /// ## Platform Support
    ///
    /// - **macOS:** Uses native `textutil` command (built-in, no dependencies)
    /// - **Linux/Windows:** Returns error with conversion instructions (`LibreOffice` support future)
    ///
    /// ## Arguments
    ///
    /// * `path` - Path to the .doc file
    ///
    /// ## Errors
    ///
    /// Returns an error if:
    /// - File does not exist or is not a .doc file
    /// - File has invalid CFB/OLE2 signature
    /// - Platform conversion tool fails or is unavailable
    ///
    /// ## Example
    ///
    /// ```rust,no_run
    /// use docling_legacy::doc::DocBackend;
    /// use std::path::Path;
    ///
    /// let docx_path = DocBackend::convert_doc_to_docx(Path::new("report.doc"))?;
    /// // Now parse docx_path with DOCX backend
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    #[must_use = "conversion produces a result that should be handled"]
    pub fn convert_doc_to_docx(path: &Path) -> Result<PathBuf> {
        // Verify file exists
        if !path.exists() {
            anyhow::bail!("File not found: {}", path.display());
        }

        // Verify file extension
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext.to_lowercase() != "doc" {
            anyhow::bail!("Expected .doc file, got: .{ext}");
        }

        // Verify CFB signature (OLE2/CFB magic bytes)
        Self::verify_cfb_signature(path)?;

        // Convert .doc → .docx using platform-specific tool
        let temp_file = Self::convert_to_docx(path)?;

        // Return the path (temporary file will be cleaned up by caller)
        Ok(temp_file.into_temp_path().keep()?)
    }

    /// Verify that the file has a valid CFB signature
    ///
    /// CFB (Compound File Binary) signature: `D0 CF 11 E0 A1 B1 1A E1`
    fn verify_cfb_signature(path: &Path) -> Result<()> {
        let bytes =
            fs::read(path).with_context(|| format!("Failed to read file: {}", path.display()))?;

        if bytes.len() < 8 {
            anyhow::bail!("File too small to be a valid .doc file (< 8 bytes)");
        }

        let actual = &bytes[0..8];

        if actual != CFB_MAGIC_SIGNATURE {
            anyhow::bail!(
                "Invalid .doc file signature. Expected CFB signature {CFB_MAGIC_SIGNATURE:02X?}, got {actual:02X?}"
            );
        }

        Ok(())
    }

    /// Convert .doc to .docx using platform-specific tools
    ///
    /// Returns the path to the converted .docx file (temporary file)
    fn convert_to_docx(doc_path: &Path) -> Result<tempfile::NamedTempFile> {
        #[cfg(target_os = "macos")]
        {
            Self::convert_with_textutil(doc_path)
        }

        #[cfg(not(target_os = "macos"))]
        {
            anyhow::bail!(
                "DOC format conversion is currently only supported on macOS (using textutil).\n\
                 \n\
                 To convert .doc files on Linux/Windows:\n\
                 1. Use LibreOffice: soffice --headless --convert-to docx {}\n\
                 2. Then parse the resulting .docx file\n\
                 \n\
                 Alternatively, install LibreOffice and enable the 'libreoffice' feature flag (coming soon).",
                doc_path.display()
            )
        }
    }

    /// Convert .doc to .docx using macOS textutil
    #[cfg(target_os = "macos")]
    fn convert_with_textutil(doc_path: &Path) -> Result<tempfile::NamedTempFile> {
        // Create temporary file for output
        let temp_docx =
            NamedTempFile::new().context("Failed to create temporary file for DOCX conversion")?;

        // Run textutil conversion
        let output = Command::new("/usr/bin/textutil")
            .arg("-convert")
            .arg("docx")
            .arg(doc_path)
            .arg("-output")
            .arg(temp_docx.path())
            .output()
            .context("Failed to execute textutil command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!(
                "textutil conversion failed:\n{}\n\nCommand: textutil -convert docx {} -output {}",
                stderr,
                doc_path.display(),
                temp_docx.path().display()
            );
        }

        // Verify output file was created and has content
        let metadata = fs::metadata(temp_docx.path())
            .context("Failed to read converted DOCX file metadata")?;

        if metadata.len() == 0 {
            anyhow::bail!("textutil produced empty DOCX file. Input may be corrupted or invalid.");
        }

        Ok(temp_docx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_verify_cfb_signature_valid() {
        // Create temp file with valid CFB signature
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(&CFB_MAGIC_SIGNATURE).unwrap();
        temp.write_all(&[0x00; 512]).unwrap(); // Add some padding
        temp.flush().unwrap();

        let result = DocBackend::verify_cfb_signature(temp.path());
        assert!(result.is_ok(), "Valid CFB signature should pass");
    }

    #[test]
    fn test_verify_cfb_signature_invalid() {
        // Create temp file with invalid signature
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"INVALID_").unwrap();
        temp.flush().unwrap();

        let result = DocBackend::verify_cfb_signature(temp.path());
        assert!(result.is_err(), "Invalid signature should fail");
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid .doc file signature"));
    }

    #[test]
    fn test_verify_cfb_signature_too_small() {
        // Create temp file with < 8 bytes (partial CFB signature)
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(&CFB_MAGIC_SIGNATURE[..3]).unwrap();
        temp.flush().unwrap();

        let result = DocBackend::verify_cfb_signature(temp.path());
        assert!(result.is_err(), "File < 8 bytes should fail");
        assert!(result.unwrap_err().to_string().contains("too small"));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_textutil_available() {
        // Verify textutil command exists on macOS
        let output = Command::new("/usr/bin/textutil").arg("--help").output();
        assert!(output.is_ok(), "textutil should be available on macOS");
    }
}
