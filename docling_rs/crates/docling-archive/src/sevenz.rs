//! 7Z archive extraction and processing
//!
//! This module provides functionality for extracting files from 7Z archives
//! and processing them recursively. Supports multiple compression methods
//! including LZMA, LZMA2, BZIP2, DEFLATE, and ZSTD.

use crate::error::ArchiveError;
use crate::zip::{ExtractedFile, FileInfo};
use log::warn;
use sevenz_rust::{Password, SevenZReader};
use std::fs::File;
use std::io::BufReader;
use std::path::{Component, Path, PathBuf};

use crate::MAX_FILE_SIZE;

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

/// Extract all files from a 7Z archive
///
/// This function opens a 7Z archive and extracts all files (excluding directories)
/// into memory. Files that exceed size limits are skipped with a warning.
///
/// Supports multiple compression methods:
/// - LZMA (default 7Z compression)
/// - LZMA2 (modern multi-threaded variant)
/// - BZIP2
/// - DEFLATE (ZIP-compatible)
/// - ZSTD (high-speed compression)
/// - Copy (uncompressed)
///
/// # Arguments
///
/// * `path` - Path to the 7Z archive file
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
/// - Archive is password-protected (encrypted)
/// - Archive exceeds size limits
///
/// # Examples
///
/// ```no_run
/// use docling_archive::sevenz::extract_7z_from_path;
/// use std::path::Path;
///
/// let files = extract_7z_from_path(Path::new("archive.7z")).unwrap();
/// for file in files {
///     println!("Extracted: {} ({} bytes)", file.name, file.size);
/// }
/// ```
#[must_use = "this function returns extracted files that should be processed"]
pub fn extract_7z_from_path(path: &Path) -> Result<Vec<ExtractedFile>, ArchiveError> {
    let file = File::open(path)?;
    let len = file.metadata()?.len();

    let reader = BufReader::new(file);

    // Create 7Z reader with empty password (will error if password required)
    let password = Password::empty();
    let mut sz = SevenZReader::new(reader, len, password).map_err(|e| {
        let err_str = e.to_string();
        if err_str.contains("password") || err_str.contains("encrypted") {
            ArchiveError::PasswordProtected
        } else {
            ArchiveError::Other(format!("7Z error: {e}"))
        }
    })?;

    let mut files = Vec::new();
    let mut extraction_failed = false;

    sz.for_each_entries(|entry, reader| {
        // Skip directories
        if entry.is_directory() {
            return Ok(true); // continue iteration
        }

        let raw_name = entry.name().to_string();
        let size = entry.size();

        // SECURITY: Sanitize path to prevent path traversal attacks
        // Malicious 7Z files can contain entries like "../../../etc/passwd"
        let Some(sanitized_path) = sanitize_path(&raw_name) else {
            warn!("Skipping invalid path: {raw_name} (path traversal attempt or empty)");
            return Ok(true); // continue iteration
        };
        let name = sanitized_path.to_string_lossy().to_string();

        // Check file size limit
        if size > MAX_FILE_SIZE {
            warn!(
                "Skipping large file in 7Z: {name} ({size} bytes exceeds {MAX_FILE_SIZE} bytes limit)"
            );
            return Ok(true); // continue iteration
        }

        // Read file contents
        let mut contents = Vec::new();
        match reader.read_to_end(&mut contents) {
            Ok(_) => {
                files.push(ExtractedFile {
                    name,
                    path: sanitized_path,
                    size: contents.len(),
                    contents,
                });
                Ok(true) // continue iteration
            }
            Err(e) => {
                warn!("Failed to extract file {name} from 7Z: {e}");
                extraction_failed = true;
                Ok(true) // continue despite error
            }
        }
    })
    .map_err(|e| {
        let err_str = e.to_string();
        if err_str.contains("password") || err_str.contains("encrypted") {
            ArchiveError::PasswordProtected
        } else {
            ArchiveError::Other(format!("7Z extraction error: {e}"))
        }
    })?;

    if extraction_failed && files.is_empty() {
        return Err(ArchiveError::Other(
            "Failed to extract any files from 7Z archive".to_string(),
        ));
    }

    Ok(files)
}

/// List files in a 7Z archive without extracting contents
///
/// This is a lightweight operation that reads the 7Z archive headers
/// to enumerate files without decompressing them.
///
/// # Arguments
///
/// * `path` - Path to the 7Z archive file
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
/// use docling_archive::sevenz::list_7z_contents;
/// use std::path::Path;
///
/// let files = list_7z_contents(Path::new("archive.7z")).unwrap();
/// for file in files {
///     println!("{} - {} bytes", file.name, file.size);
/// }
/// ```
#[must_use = "this function returns archive file listing that should be processed"]
pub fn list_7z_contents(path: &Path) -> Result<Vec<FileInfo>, ArchiveError> {
    let file = File::open(path)?;
    let len = file.metadata()?.len();

    let reader = BufReader::new(file);

    let password = Password::empty();
    let mut sz = SevenZReader::new(reader, len, password).map_err(|e| {
        let err_str = e.to_string();
        if err_str.contains("password") || err_str.contains("encrypted") {
            ArchiveError::PasswordProtected
        } else {
            ArchiveError::Other(format!("7Z error: {e}"))
        }
    })?;

    let mut file_infos = Vec::new();

    sz.for_each_entries(|entry, _reader| {
        // Skip directories
        if entry.is_directory() {
            return Ok(true);
        }

        let raw_name = entry.name().to_string();
        let size = entry.size();

        // Sanitize path for consistency (also protects against malicious filenames in listing)
        let name = match sanitize_path(&raw_name) {
            Some(p) => p.to_string_lossy().to_string(),
            None => {
                // Skip invalid paths
                return Ok(true);
            }
        };

        // Note: 7Z format doesn't easily expose compressed size per file
        // For solid archives, multiple files share compression blocks
        let compressed_size = 0; // Not available from API

        file_infos.push(FileInfo {
            name,
            size,
            compressed_size,
            is_encrypted: false, // Would have errored earlier if encrypted
        });

        Ok(true)
    })
    .map_err(|e| ArchiveError::Other(format!("7Z listing error: {e}")))?;

    Ok(file_infos)
}

/// Extract and process files from a 7Z archive with streaming callback
///
/// This function provides a memory-efficient way to process large 7Z archives
/// by invoking a callback for each extracted file. Files are processed one at
/// a time and not stored in memory simultaneously.
///
/// # Arguments
///
/// * `path` - Path to the 7Z archive file
/// * `processor` - Callback function invoked for each extracted file
///
/// # Returns
///
/// Returns `Ok(())` if all files were processed successfully
///
/// # Errors
///
/// Returns `ArchiveError` if:
/// - Archive cannot be opened
/// - Archive is invalid or corrupted
/// - The callback function returns an error
///
/// # Examples
///
/// ```no_run
/// use docling_archive::sevenz::extract_7z_streaming;
/// use std::path::Path;
///
/// extract_7z_streaming(Path::new("large.7z"), |file| {
///     println!("Processing: {}", file.name);
///     // Process file.contents...
///     Ok(())
/// }).unwrap();
/// ```
#[must_use = "this function returns a Result that should be checked for errors"]
pub fn extract_7z_streaming<F>(path: &Path, mut processor: F) -> Result<(), ArchiveError>
where
    F: FnMut(&ExtractedFile) -> Result<(), ArchiveError>,
{
    let file = File::open(path)?;
    let len = file.metadata()?.len();

    let reader = BufReader::new(file);

    let password = Password::empty();
    let mut sz = SevenZReader::new(reader, len, password).map_err(|e| {
        let err_str = e.to_string();
        if err_str.contains("password") || err_str.contains("encrypted") {
            ArchiveError::PasswordProtected
        } else {
            ArchiveError::Other(format!("7Z error: {e}"))
        }
    })?;

    sz.for_each_entries(|entry, reader| {
        // Skip directories
        if entry.is_directory() {
            return Ok(true);
        }

        let raw_name = entry.name().to_string();
        let size = entry.size();

        // SECURITY: Sanitize path to prevent path traversal attacks
        let Some(sanitized_path) = sanitize_path(&raw_name) else {
            warn!("Skipping invalid path: {raw_name} (path traversal attempt or empty)");
            return Ok(true);
        };
        let name = sanitized_path.to_string_lossy().to_string();

        // Check file size limit
        if size > MAX_FILE_SIZE {
            warn!(
                "Skipping large file in 7Z: {name} ({size} bytes exceeds {MAX_FILE_SIZE} bytes limit)"
            );
            return Ok(true);
        }

        // Read file contents
        let mut contents = Vec::new();
        reader.read_to_end(&mut contents)?;

        let extracted_file = ExtractedFile {
            name,
            path: sanitized_path,
            size: contents.len(),
            contents,
        };

        // Invoke callback
        processor(&extracted_file).map_err(|e| std::io::Error::other(e.to_string()))?;

        Ok(true)
    })
    .map_err(|e| ArchiveError::Other(format!("7Z streaming error: {e}")))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_7z_nonexistent_file() {
        let result = extract_7z_from_path(Path::new("nonexistent.7z"));
        assert!(result.is_err());
        // Should be an IO error for file not found
    }

    #[test]
    fn test_7z_invalid_format() {
        // Create a file with invalid 7Z format
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"not a real 7z file").unwrap();
        temp_file.flush().unwrap();

        let result = extract_7z_from_path(temp_file.path());
        assert!(result.is_err());
        // Should be an error (invalid 7Z format)
    }

    // Note: Creating actual 7Z test files programmatically requires using the
    // sevenz-rust compress feature, which we've disabled to reduce dependencies.
    // Instead, we rely on integration tests with real 7Z files from test corpus.

    #[test]
    fn test_extract_7z_real_file() {
        // Uses simple.7z from test corpus (relative path from crate root)
        let test_file = Path::new("../../test-corpus/archives/7z/simple.7z");
        if !test_file.exists() {
            eprintln!("Test file not found: {test_file:?}");
            return;
        }

        let files = extract_7z_from_path(test_file).unwrap();
        assert!(!files.is_empty(), "Should extract at least one file");

        for file in &files {
            println!("Extracted: {} ({} bytes)", file.name, file.size);
            // Note: Some archives may have empty files, so we don't assert size > 0
        }
    }

    #[test]
    fn test_list_7z_real_file() {
        // Uses simple.7z from test corpus (relative path from crate root)
        let test_file = Path::new("../../test-corpus/archives/7z/simple.7z");
        if !test_file.exists() {
            eprintln!("Test file not found: {test_file:?}");
            return;
        }

        let files = list_7z_contents(test_file).unwrap();
        assert!(!files.is_empty(), "Should list at least one file");

        for file in &files {
            println!("File: {} ({} bytes)", file.name, file.size);
        }
    }

    #[test]
    fn test_7z_streaming_real_file() {
        // Uses simple.7z from test corpus (relative path from crate root)
        let test_file = Path::new("../../test-corpus/archives/7z/simple.7z");
        if !test_file.exists() {
            eprintln!("Test file not found: {test_file:?}");
            return;
        }

        let mut file_count = 0;

        extract_7z_streaming(test_file, |file| {
            println!("Processing: {} ({} bytes)", file.name, file.size);
            file_count += 1;
            Ok(())
        })
        .unwrap();

        assert!(file_count > 0, "Should process at least one file");
    }
}
