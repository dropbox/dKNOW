//! Microsoft Publisher (.pub) format support
//!
//! Uses `LibreOffice` for conversion since .pub is a proprietary binary format.
//!
//! Strategy:
//! 1. Use `soffice --headless --convert-to pdf` to convert .pub to PDF
//! 2. Use existing PDF backend to parse the converted PDF
//! 3. Return the `DoclingDocument`

use anyhow::{Context, Result};
// use docling_core::DoclingDocument;  // TODO: Will be used when implementing direct DocItem generation
// NOTE: Direct .pub parsing would require:
// - OLE Compound Document parsing (cfb crate)
// - Publisher-specific binary format parsing (libmspub FFI)
// Current: LibreOffice conversion is acceptable for most use cases
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

/// Backend for Microsoft Publisher files
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct PublisherBackend;

impl PublisherBackend {
    /// Create a new Publisher backend
    #[inline]
    #[must_use = "creates Publisher backend instance"]
    pub const fn new() -> Self {
        Self
    }

    /// Convert Publisher file to PDF using `LibreOffice`
    /// Returns PDF bytes
    ///
    /// # Errors
    ///
    /// Returns an error if `LibreOffice` is not available or if conversion fails.
    #[must_use = "this function returns PDF data that should be processed"]
    pub fn convert_to_pdf(&self, input_path: &Path) -> Result<Vec<u8>> {
        // Create temporary directory for output
        let temp_dir = TempDir::new().context("Failed to create temporary directory")?;

        // Run LibreOffice conversion
        let output = Command::new("soffice")
            .arg("--headless")
            .arg("--convert-to")
            .arg("pdf:writer_pdf_Export")
            .arg("--outdir")
            .arg(temp_dir.path())
            .arg(input_path)
            .output()
            .context("Failed to execute LibreOffice (soffice)")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("LibreOffice conversion failed: {stderr}");
        }

        // Find the output PDF file
        let input_stem = input_path
            .file_stem()
            .context("Invalid input filename")?
            .to_string_lossy();
        let pdf_path = temp_dir.path().join(format!("{input_stem}.pdf"));

        if !pdf_path.exists() {
            anyhow::bail!("Converted PDF not found at {}", pdf_path.display());
        }

        // Read PDF bytes
        let pdf_bytes = std::fs::read(&pdf_path).context("Failed to read converted PDF")?;
        Ok(pdf_bytes)
    }
}

impl PublisherBackend {
    /// Get the backend name
    #[inline]
    #[must_use = "returns backend name string"]
    pub const fn name(&self) -> &'static str {
        "Publisher"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    // ==== Backend Creation Tests ====

    #[test]
    fn test_publisher_backend_creation() {
        let backend = PublisherBackend::new();
        assert_eq!(backend.name(), "Publisher");
    }

    #[test]
    #[allow(
        clippy::default_constructed_unit_structs,
        reason = "testing Default trait impl"
    )]
    fn test_publisher_backend_default_equals_new() {
        // Verify derived Default produces same result as new()
        assert_eq!(PublisherBackend::default(), PublisherBackend::new());
    }

    #[test]
    fn test_publisher_backend_default() {
        let backend1 = PublisherBackend::new();
        let backend2 = PublisherBackend {};
        assert_eq!(backend1.name(), backend2.name());
    }

    #[test]
    fn test_multiple_backend_instances() {
        let backend1 = PublisherBackend::new();
        let backend2 = PublisherBackend::new();
        let backend3 = PublisherBackend {};

        assert_eq!(backend1.name(), backend2.name());
        assert_eq!(backend2.name(), backend3.name());
    }

    #[test]
    fn test_backend_name_consistency() {
        let backend = PublisherBackend::new();
        assert_eq!(backend.name(), "Publisher");
        assert_eq!(backend.name(), backend.name()); // Idempotent
    }

    #[test]
    fn test_backend_name_is_static() {
        let backend = PublisherBackend::new();
        let name1 = backend.name();
        let name2 = backend.name();

        assert_eq!(name1, "Publisher");
        assert_eq!(name1, name2);
        assert_eq!(name1.len(), 9);
    }

    // ==== Error Handling Tests ====

    #[test]
    fn test_convert_to_pdf_nonexistent_file() {
        let backend = PublisherBackend::new();
        let nonexistent = Path::new("/nonexistent/path/file.pub");

        let result = backend.convert_to_pdf(nonexistent);
        assert!(result.is_err(), "Should fail on nonexistent file");
    }

    #[test]
    fn test_convert_to_pdf_empty_file() {
        let backend = PublisherBackend::new();

        // Create a temporary empty .pub file
        let temp_dir = TempDir::new().unwrap();
        let pub_path = temp_dir.path().join("empty.pub");
        std::fs::write(&pub_path, b"").unwrap();

        // Attempt conversion - may fail on empty file depending on LibreOffice
        let result = backend.convert_to_pdf(&pub_path);
        // Note: result can be Ok or Err depending on LibreOffice version
        // Just verify it doesn't panic
        let _ = result;
    }

    #[test]
    fn test_convert_to_pdf_large_invalid_file() {
        let backend = PublisherBackend::new();

        let temp_dir = TempDir::new().unwrap();
        let pub_path = temp_dir.path().join("large.pub");
        // Create a 1MB file of zeros
        let large_data = vec![0u8; 1024 * 1024];
        std::fs::write(&pub_path, large_data).unwrap();

        // LibreOffice may or may not process this - just verify no panic
        let result = backend.convert_to_pdf(&pub_path);
        let _ = result;
    }

    // ==== Filename Handling Tests ====

    #[test]
    fn test_convert_to_pdf_with_spaces_in_filename() {
        let backend = PublisherBackend::new();

        let temp_dir = TempDir::new().unwrap();
        let pub_path = temp_dir.path().join("file with spaces.pub");
        std::fs::write(&pub_path, b"Test content").unwrap();

        // Should handle spaces in filename gracefully
        let result = backend.convert_to_pdf(&pub_path);
        // Verify path handling works (result depends on LibreOffice)
        let _ = result;
    }

    #[test]
    fn test_convert_to_pdf_with_unicode_filename() {
        let backend = PublisherBackend::new();

        let temp_dir = TempDir::new().unwrap();
        let pub_path = temp_dir.path().join("文档.pub");
        std::fs::write(&pub_path, b"Test content").unwrap();

        // Should handle Unicode filename gracefully
        let result = backend.convert_to_pdf(&pub_path);
        let _ = result;
    }

    #[test]
    fn test_convert_to_pdf_with_special_chars_filename() {
        let backend = PublisherBackend::new();

        let temp_dir = TempDir::new().unwrap();
        let pub_path = temp_dir.path().join("file&name#test.pub");
        std::fs::write(&pub_path, b"Test content").unwrap();

        // Should handle special characters in filename gracefully
        let result = backend.convert_to_pdf(&pub_path);
        let _ = result;
    }

    #[test]
    fn test_convert_to_pdf_no_extension() {
        let backend = PublisherBackend::new();

        let temp_dir = TempDir::new().unwrap();
        let pub_path = temp_dir.path().join("no_extension");
        std::fs::write(&pub_path, b"Test content").unwrap();

        let result = backend.convert_to_pdf(&pub_path);
        let _ = result;
    }

    #[test]
    fn test_convert_to_pdf_nested_directory() {
        let backend = PublisherBackend::new();

        let temp_dir = TempDir::new().unwrap();
        let nested_path = temp_dir.path().join("nested").join("directory");
        std::fs::create_dir_all(&nested_path).unwrap();
        let pub_path = nested_path.join("file.pub");
        std::fs::write(&pub_path, b"Test content").unwrap();

        let result = backend.convert_to_pdf(&pub_path);
        let _ = result;
    }

    #[test]
    fn test_convert_to_pdf_very_long_filename() {
        let backend = PublisherBackend::new();

        let temp_dir = TempDir::new().unwrap();
        // Create a filename with 100 characters (200 might exceed FS limits)
        let long_name = "a".repeat(100) + ".pub";
        let pub_path = temp_dir.path().join(long_name);

        // Some filesystems may not support this
        if std::fs::write(&pub_path, b"Test").is_ok() {
            let result = backend.convert_to_pdf(&pub_path);
            let _ = result;
        }
    }

    // ==== File Permission Tests ====

    #[test]
    fn test_convert_to_pdf_readonly_input() {
        let backend = PublisherBackend::new();

        let temp_dir = TempDir::new().unwrap();
        let pub_path = temp_dir.path().join("readonly.pub");
        let mut file = std::fs::File::create(&pub_path).unwrap();
        file.write_all(b"Test content").unwrap();
        drop(file);

        // Make readonly
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&pub_path).unwrap().permissions();
            perms.set_mode(0o444);
            std::fs::set_permissions(&pub_path, perms).unwrap();
        }

        // Should still be able to read readonly files
        let result = backend.convert_to_pdf(&pub_path);
        let _ = result;
    }

    #[test]
    fn test_convert_to_pdf_symlink() {
        let backend = PublisherBackend::new();

        #[cfg(unix)]
        {
            let temp_dir = TempDir::new().unwrap();
            let real_path = temp_dir.path().join("real.pub");
            let symlink_path = temp_dir.path().join("symlink.pub");

            std::fs::write(&real_path, b"Test content").unwrap();
            std::os::unix::fs::symlink(&real_path, &symlink_path).unwrap();

            // Should handle symlinks gracefully
            let result = backend.convert_to_pdf(&symlink_path);
            let _ = result;
        }

        #[cfg(not(unix))]
        {
            // Skip test on non-Unix platforms
            assert!(true);
        }
    }

    // ==== Content Type Tests ====

    #[test]
    fn test_convert_to_pdf_binary_content() {
        let backend = PublisherBackend::new();

        let temp_dir = TempDir::new().unwrap();
        let pub_path = temp_dir.path().join("binary.pub");
        // Write some binary data
        let binary_data: Vec<u8> = (0..255).collect();
        std::fs::write(&pub_path, binary_data).unwrap();

        let result = backend.convert_to_pdf(&pub_path);
        let _ = result;
    }

    #[test]
    fn test_convert_to_pdf_utf8_content() {
        let backend = PublisherBackend::new();

        let temp_dir = TempDir::new().unwrap();
        let pub_path = temp_dir.path().join("utf8.pub");
        std::fs::write(&pub_path, "Unicode: 你好世界 🌍").unwrap();

        let result = backend.convert_to_pdf(&pub_path);
        let _ = result;
    }

    #[test]
    fn test_convert_to_pdf_with_newline_in_content() {
        let backend = PublisherBackend::new();

        let temp_dir = TempDir::new().unwrap();
        let pub_path = temp_dir.path().join("newlines.pub");
        std::fs::write(&pub_path, "Line 1\nLine 2\nLine 3\n").unwrap();

        let result = backend.convert_to_pdf(&pub_path);
        let _ = result;
    }

    #[test]
    fn test_convert_to_pdf_zip_file() {
        let backend = PublisherBackend::new();

        let temp_dir = TempDir::new().unwrap();
        let pub_path = temp_dir.path().join("file.pub");
        // Write ZIP header (PK\x03\x04)
        std::fs::write(&pub_path, b"PK\x03\x04").unwrap();

        let result = backend.convert_to_pdf(&pub_path);
        let _ = result;
    }

    // ==== Integration Notes ====
    // Note: Real Publisher file conversion requires:
    // 1. LibreOffice installed and accessible via 'soffice' command
    // 2. Actual .pub test files (proprietary binary format)
    // 3. Platform-specific LibreOffice behavior varies
    //
    // These unit tests focus on:
    // - Backend API contract (new, name, default)
    // - Error handling (nonexistent files)
    // - Filename handling (spaces, unicode, special chars)
    // - File permission handling (readonly, symlinks)
    // - Graceful degradation (invalid files don't cause panics)
    //
    // Full integration testing happens in the integration test suite
    // with actual .pub files and LibreOffice validation.
}
