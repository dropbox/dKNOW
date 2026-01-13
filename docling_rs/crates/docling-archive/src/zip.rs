//! ZIP archive extraction and processing
//!
//! This module provides functionality for extracting files from ZIP archives
//! and processing them recursively.

use crate::error::ArchiveError;
use log::warn;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Component, Path, PathBuf};
use zip::ZipArchive;

/// Sanitize a path to prevent path traversal attacks (e.g., ../../../etc/passwd)
///
/// This function removes:
/// - Parent directory references (..)
/// - Current directory references (.)
/// - Absolute path prefixes (/)
/// - Drive letters (C:\)
///
/// Returns None if the path is entirely invalid (e.g., just "..")
#[inline]
fn sanitize_path(path: &str) -> Option<PathBuf> {
    let path = Path::new(path);
    let mut sanitized = PathBuf::new();

    for component in path.components() {
        // Only keep normal path components
        // Skip: parent refs (..), current dir (.), root (/), drive prefixes (C:\)
        if let Component::Normal(part) = component {
            sanitized.push(part);
        }
    }

    // Return None if path is empty after sanitization
    if sanitized.as_os_str().is_empty() {
        None
    } else {
        Some(sanitized)
    }
}

use crate::MAX_FILE_SIZE;

/// Extracted file from a ZIP archive
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ExtractedFile {
    /// Original filename within the archive
    pub name: String,
    /// Path of the file (may include directory structure)
    pub path: PathBuf,
    /// Uncompressed file size in bytes
    pub size: usize,
    /// File contents as bytes
    pub contents: Vec<u8>,
}

/// Information about a file in a ZIP archive (without extracting contents)
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct FileInfo {
    /// Filename within the archive
    pub name: String,
    /// Uncompressed file size
    pub size: u64,
    /// Compressed file size
    pub compressed_size: u64,
    /// Whether the file is encrypted
    pub is_encrypted: bool,
}

/// Extract all files from a ZIP archive
///
/// This function opens a ZIP archive and extracts all files (excluding directories)
/// into memory. Files that exceed size limits are skipped with a warning.
///
/// # Arguments
///
/// * `path` - Path to the ZIP archive file
///
/// # Returns
///
/// A vector of `ExtractedFile` structs containing file metadata and contents
///
/// # Errors
///
/// Returns `ArchiveError` if:
/// - Archive cannot be opened
/// - Archive is invalid or corrupted
/// - Archive is password-protected
/// - Archive exceeds size limits
///
/// # Panics
///
/// Should not panic in practice. Uses `.expect()` for file size conversion
/// but sizes are pre-checked against `MAX_FILE_SIZE` (100MB < `usize::MAX`).
///
/// # Examples
///
/// ```no_run
/// use docling_archive::zip::extract_zip_from_path;
/// use std::path::Path;
///
/// let files = extract_zip_from_path(Path::new("archive.zip")).unwrap();
/// for file in files {
///     println!("Extracted: {} ({} bytes)", file.name, file.size);
/// }
/// ```
#[must_use = "this function returns extracted files that should be processed"]
pub fn extract_zip_from_path(path: &Path) -> Result<Vec<ExtractedFile>, ArchiveError> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut archive = ZipArchive::new(reader)?;

    let mut files = Vec::new();

    for i in 0..archive.len() {
        let mut zip_file = archive.by_index(i)?;

        // Skip directories
        if zip_file.is_dir() {
            continue;
        }

        // Check if file is encrypted
        if zip_file.encrypted() {
            return Err(ArchiveError::PasswordProtected);
        }

        let raw_name = zip_file.name().to_string();
        let size = zip_file.size();

        // SECURITY: Sanitize path to prevent path traversal attacks
        // Malicious archives can contain entries like "../../../etc/passwd"
        let Some(sanitized_path) = sanitize_path(&raw_name) else {
            warn!("Skipping invalid path: {raw_name} (path traversal attempt or empty)");
            continue;
        };
        let name = sanitized_path.to_string_lossy().to_string();

        // Check file size limit
        if size > MAX_FILE_SIZE {
            warn!("Skipping large file: {name} ({size} bytes exceeds {MAX_FILE_SIZE} bytes limit)");
            continue;
        }

        // Read file contents
        // Safe: size already checked against MAX_FILE_SIZE (100MB < usize::MAX)
        let mut contents = Vec::with_capacity(
            size.try_into()
                .expect("size within bounds after MAX_FILE_SIZE check"),
        );
        zip_file.read_to_end(&mut contents)?;

        files.push(ExtractedFile {
            name: name.clone(),
            path: sanitized_path,
            size: contents.len(),
            contents,
        });
    }

    Ok(files)
}

/// List files in a ZIP archive without extracting contents
///
/// This is a lightweight operation that reads the ZIP central directory
/// to enumerate files without decompressing them.
///
/// # Arguments
///
/// * `path` - Path to the ZIP archive file
///
/// # Returns
///
/// A vector of `FileInfo` structs containing file metadata
///
/// # Errors
///
/// Returns `ArchiveError` if:
/// - Archive cannot be opened
/// - Archive is invalid or corrupted
///
/// # Examples
///
/// ```no_run
/// use docling_archive::zip::list_zip_contents;
/// use std::path::Path;
///
/// let files = list_zip_contents(Path::new("archive.zip")).unwrap();
/// for file in files {
///     println!("{} - {} bytes (compressed: {} bytes)",
///              file.name, file.size, file.compressed_size);
/// }
/// ```
#[must_use = "this function returns archive file listing that should be processed"]
pub fn list_zip_contents(path: &Path) -> Result<Vec<FileInfo>, ArchiveError> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut archive = ZipArchive::new(reader)?;

    let mut files = Vec::new();

    for i in 0..archive.len() {
        let zip_file = archive.by_index(i)?;

        // Skip directories
        if zip_file.is_dir() {
            continue;
        }

        files.push(FileInfo {
            name: zip_file.name().to_string(),
            size: zip_file.size(),
            compressed_size: zip_file.compressed_size(),
            is_encrypted: zip_file.encrypted(),
        });
    }

    Ok(files)
}

/// Extract files from ZIP archive using a streaming approach
///
/// This function processes files one-at-a-time instead of loading all into memory,
/// which is more memory-efficient for large archives.
///
/// # Arguments
///
/// * `path` - Path to the ZIP archive file
/// * `processor` - Callback function to process each extracted file
///
/// # Errors
///
/// Returns `ArchiveError` if archive operations fail, or propagates errors
/// from the processor callback.
///
/// # Panics
///
/// Should not panic in practice. Uses `.expect()` for file size conversion
/// but sizes are pre-checked against `MAX_FILE_SIZE` (100MB < `usize::MAX`).
///
/// # Examples
///
/// ```no_run
/// use docling_archive::zip::extract_zip_streaming;
/// use std::path::Path;
///
/// extract_zip_streaming(Path::new("archive.zip"), |file| {
///     println!("Processing: {}", file.name);
///     // Process file contents...
///     Ok(())
/// }).unwrap();
/// ```
#[must_use = "this function returns a Result that should be checked for errors"]
pub fn extract_zip_streaming<F>(path: &Path, mut processor: F) -> Result<(), ArchiveError>
where
    F: FnMut(ExtractedFile) -> Result<(), Box<dyn std::error::Error>>,
{
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut archive = ZipArchive::new(reader)?;

    for i in 0..archive.len() {
        let mut zip_file = archive.by_index(i)?;

        // Skip directories
        if zip_file.is_dir() {
            continue;
        }

        // Check if encrypted
        if zip_file.encrypted() {
            return Err(ArchiveError::PasswordProtected);
        }

        let raw_name = zip_file.name().to_string();
        let size = zip_file.size();

        // SECURITY: Sanitize path to prevent path traversal attacks
        let Some(sanitized_path) = sanitize_path(&raw_name) else {
            warn!("Skipping invalid path: {raw_name} (path traversal attempt or empty)");
            continue;
        };
        let name = sanitized_path.to_string_lossy().to_string();

        // Check file size limit
        if size > MAX_FILE_SIZE {
            warn!("Skipping large file: {name} ({size} bytes exceeds {MAX_FILE_SIZE} bytes limit)");
            continue;
        }

        // Read file contents
        // Safe: size already checked against MAX_FILE_SIZE (100MB < usize::MAX)
        let mut contents = Vec::with_capacity(
            size.try_into()
                .expect("size within bounds after MAX_FILE_SIZE check"),
        );
        zip_file.read_to_end(&mut contents)?;

        let extracted_file = ExtractedFile {
            name: name.clone(),
            path: sanitized_path,
            size: contents.len(),
            contents,
        };

        // Process file immediately (contents dropped after this)
        processor(extracted_file).map_err(|e| std::io::Error::other(e.to_string()))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    use zip::write::{FileOptions, ZipWriter};

    /// Helper: Create a simple test ZIP file
    fn create_test_zip() -> Result<NamedTempFile, Box<dyn std::error::Error>> {
        let temp_file = NamedTempFile::new()?;
        let mut zip = ZipWriter::new(temp_file.reopen()?);

        // Add test files
        let options: FileOptions<()> = FileOptions::default();

        zip.start_file("file1.txt", options)?;
        zip.write_all(b"Hello from file 1")?;

        zip.start_file("file2.txt", options)?;
        zip.write_all(b"Hello from file 2")?;

        zip.start_file("subdir/file3.txt", options)?;
        zip.write_all(b"Hello from subdirectory")?;

        zip.finish()?;

        Ok(temp_file)
    }

    #[test]
    #[allow(clippy::similar_names)]
    fn test_extract_zip_basic() {
        let temp_zip = create_test_zip().expect("Failed to create test ZIP");
        let files = extract_zip_from_path(temp_zip.path()).expect("Failed to extract ZIP");

        assert_eq!(files.len(), 3, "Should extract 3 files");

        // Check first file
        let file1 = files
            .iter()
            .find(|f| f.name == "file1.txt")
            .expect("file1.txt not found");
        assert_eq!(file1.contents, b"Hello from file 1");
        assert_eq!(file1.size, 17);

        // Check subdirectory file
        let file3 = files
            .iter()
            .find(|f| f.name == "subdir/file3.txt")
            .expect("subdir/file3.txt not found");
        assert_eq!(file3.contents, b"Hello from subdirectory");
    }

    #[test]
    #[allow(clippy::similar_names)]
    fn test_list_zip_contents() {
        let temp_zip = create_test_zip().expect("Failed to create test ZIP");
        let files = list_zip_contents(temp_zip.path()).expect("Failed to list ZIP contents");

        assert_eq!(files.len(), 3, "Should list 3 files");

        // Check file info
        let file1 = files
            .iter()
            .find(|f| f.name == "file1.txt")
            .expect("file1.txt not found");
        assert_eq!(file1.size, 17);
        assert!(!file1.is_encrypted);
    }

    #[test]
    fn test_extract_zip_streaming() {
        let temp_zip = create_test_zip().expect("Failed to create test ZIP");
        let mut count = 0;

        extract_zip_streaming(temp_zip.path(), |file| {
            count += 1;
            assert!(!file.contents.is_empty());
            Ok(())
        })
        .expect("Failed to stream ZIP");

        assert_eq!(count, 3, "Should process 3 files");
    }

    #[test]
    fn test_nonexistent_file() {
        let result = extract_zip_from_path(Path::new("nonexistent.zip"));
        assert!(result.is_err(), "Should fail for nonexistent file");
    }
}
