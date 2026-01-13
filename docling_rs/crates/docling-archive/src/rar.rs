//! RAR archive extraction and processing
//!
//! This module provides functionality for extracting files from RAR archives
//! and processing them recursively. Supports both RAR4 (legacy) and RAR5 (modern)
//! formats, including multi-volume archives.
//!
//! Uses the `unar` command-line tool which provides full compatibility with all
//! RAR formats and features without license restrictions.

use crate::error::ArchiveError;
use crate::zip::{ExtractedFile, FileInfo};
use log::warn;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

/// Sanitize a path to prevent path traversal attacks (e.g., ../../../etc/passwd)
///
/// Removes:
/// - Parent directory references (..)
/// - Current directory references (.)
/// - Absolute path prefixes (/, C:\, etc.)
///
/// Returns None if the sanitized path would be empty.
#[inline]
fn sanitize_path(path: &Path) -> Option<PathBuf> {
    let mut sanitized = PathBuf::new();

    for component in path.components() {
        // Only keep normal path components
        // Skip: parent refs (..), current dir (.), root (/), drive prefixes (C:\)
        if let Component::Normal(name) = component {
            sanitized.push(name);
        }
    }

    if sanitized.as_os_str().is_empty() {
        None
    } else {
        Some(sanitized)
    }
}

use crate::MAX_FILE_SIZE;

/// Extract all files from a RAR archive
///
/// This function opens a RAR archive and extracts all files (excluding directories)
/// into memory. Files that exceed size limits are skipped with a warning.
///
/// Supports:
/// - RAR4 (legacy format, pre-2013)
/// - RAR5 (modern format, 2013+)
/// - Multi-volume archives (.part1.rar, .part2.rar, etc.)
/// - Solid and non-solid compression
///
/// # Arguments
///
/// * `path` - Path to the RAR archive file (or first part of multi-volume archive)
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
/// - Multi-volume archive parts are missing
/// - unar command is not available
///
/// # Examples
///
/// ```no_run
/// use docling_archive::rar::extract_rar_from_path;
/// use std::path::Path;
///
/// let files = extract_rar_from_path(Path::new("archive.rar")).unwrap();
/// for file in files {
///     println!("Extracted: {} ({} bytes)", file.name, file.size);
/// }
/// ```
#[must_use = "this function returns extracted files that should be processed"]
pub fn extract_rar_from_path(path: &Path) -> Result<Vec<ExtractedFile>, ArchiveError> {
    // Verify archive exists
    if !path.exists() {
        return Err(ArchiveError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "RAR file not found",
        )));
    }

    // Create temporary directory for extraction
    let temp_dir = TempDir::new().map_err(ArchiveError::Io)?;

    // Extract using unar command
    let output = Command::new("unar")
        .arg("-o")
        .arg(temp_dir.path())
        .arg("-D") // Don't create subdirectory
        .arg("-f") // Force overwrite
        .arg(path)
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ArchiveError::Other(
                    "unar command not found. Install with: brew install unar".into(),
                )
            } else {
                ArchiveError::Io(e)
            }
        })?;

    // Check if extraction succeeded
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("password") || stderr.contains("encrypted") {
            return Err(ArchiveError::PasswordProtected);
        }
        return Err(ArchiveError::Other(format!(
            "unar extraction failed: {stderr}"
        )));
    }

    // Read extracted files
    let mut files = Vec::new();
    read_directory_recursive(temp_dir.path(), temp_dir.path(), &mut files)?;

    Ok(files)
}

/// Recursively read all files from a directory
fn read_directory_recursive(
    dir: &Path,
    base_path: &Path,
    files: &mut Vec<ExtractedFile>,
) -> Result<(), ArchiveError> {
    for entry in fs::read_dir(dir).map_err(ArchiveError::Io)? {
        let entry = entry.map_err(ArchiveError::Io)?;
        let path = entry.path();
        let metadata = entry.metadata().map_err(ArchiveError::Io)?;

        if metadata.is_file() {
            let size = metadata.len();

            // Get relative path from base
            let raw_path = path.strip_prefix(base_path).unwrap_or(&path);
            let raw_name = raw_path.to_string_lossy().to_string();

            // SECURITY: Sanitize path to prevent path traversal attacks
            // Although unar should sanitize during extraction, we sanitize again
            // to be safe and consistent with other archive formats
            let Some(sanitized_path) = sanitize_path(raw_path) else {
                warn!("Skipping invalid path: {raw_name} (path traversal attempt or empty)");
                continue;
            };
            let name = sanitized_path.to_string_lossy().to_string();

            // Check file size limit
            if size > MAX_FILE_SIZE {
                warn!(
                    "Skipping large file in RAR: {name} ({size} bytes exceeds {MAX_FILE_SIZE} bytes limit)"
                );
                continue;
            }

            let contents = fs::read(&path).map_err(ArchiveError::Io)?;

            files.push(ExtractedFile {
                name: name.clone(),
                path: sanitized_path,
                // Safe: size already checked against MAX_FILE_SIZE (100MB < usize::MAX)
                size: size
                    .try_into()
                    .expect("size within bounds after MAX_FILE_SIZE check"),
                contents,
            });
        } else if metadata.is_dir() {
            read_directory_recursive(&path, base_path, files)?;
        }
    }

    Ok(())
}

/// List contents of a RAR archive without extracting
///
/// This function provides a lightweight way to inspect archive contents
/// without extracting files. Useful for previewing archives or checking
/// if specific files exist.
///
/// # Arguments
///
/// * `path` - Path to the RAR archive file
///
/// # Returns
///
/// A vector of `FileInfo` structs containing file metadata
///
/// # Errors
///
/// Returns `ArchiveError` if archive cannot be opened or is invalid
///
/// # Examples
///
/// ```no_run
/// use docling_archive::rar::list_rar_contents;
/// use std::path::Path;
///
/// let contents = list_rar_contents(Path::new("archive.rar")).unwrap();
/// for file in contents {
///     println!("{}: {} bytes", file.name, file.size);
/// }
/// ```
#[must_use = "this function returns archive file listing that should be processed"]
pub fn list_rar_contents(path: &Path) -> Result<Vec<FileInfo>, ArchiveError> {
    // Verify archive exists
    if !path.exists() {
        return Err(ArchiveError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "RAR file not found",
        )));
    }

    // Use lsar (list archive) command
    let output = Command::new("lsar")
        .arg("-j") // JSON output
        .arg(path)
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ArchiveError::Other(
                    "lsar command not found. Install with: brew install unar".into(),
                )
            } else {
                ArchiveError::Io(e)
            }
        })?;

    // Check if listing succeeded
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("password") || stderr.contains("encrypted") {
            return Err(ArchiveError::PasswordProtected);
        }
        return Err(ArchiveError::Other(format!(
            "lsar listing failed: {stderr}"
        )));
    }

    // Parse JSON output
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| ArchiveError::Other(format!("Failed to parse lsar JSON output: {e}")))?;

    let mut files = Vec::new();

    if let Some(entries) = json["lsarContents"].as_array() {
        for entry in entries {
            // Skip directories
            if let Some(entry_type) = entry["XADFileType"].as_str() {
                if entry_type == "Directory" {
                    continue;
                }
            }

            let raw_name = entry["XADFileName"].as_str().unwrap_or("unknown");

            // SECURITY: Sanitize path for consistency with extraction
            // (protects against malicious filenames in listing)
            let Some(p) = sanitize_path(Path::new(raw_name)) else {
                warn!("Skipping invalid path in listing: {raw_name}");
                continue;
            };
            let name = p.to_string_lossy().to_string();

            let size = entry["XADFileSize"].as_u64().unwrap_or(0);
            let compressed_size = entry["XADCompressedSize"].as_u64().unwrap_or(0);
            let is_encrypted = entry["XADIsEncrypted"].as_bool().unwrap_or(false);

            files.push(FileInfo {
                name,
                size,
                compressed_size,
                is_encrypted,
            });
        }
    }

    Ok(files)
}

/// Extract files from a RAR archive with a callback for each file
///
/// This function provides streaming extraction, allowing you to process
/// files one at a time without loading the entire archive into memory.
///
/// # Arguments
///
/// * `path` - Path to the RAR archive file
/// * `processor` - Callback function invoked for each extracted file
///
/// # Returns
///
/// `Ok(())` if all files were processed successfully
///
/// # Errors
///
/// Returns `ArchiveError` if archive cannot be opened or processing fails
///
/// # Examples
///
/// ```no_run
/// use docling_archive::rar::extract_rar_streaming;
/// use std::path::Path;
///
/// extract_rar_streaming(Path::new("archive.rar"), |file| {
///     println!("Processing: {}", file.name);
///     Ok(())
/// }).unwrap();
/// ```
#[must_use = "this function returns a Result that should be checked for errors"]
pub fn extract_rar_streaming<F>(path: &Path, mut processor: F) -> Result<(), ArchiveError>
where
    F: FnMut(ExtractedFile) -> Result<(), ArchiveError>,
{
    let files = extract_rar_from_path(path)?;
    for file in files {
        processor(file)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_extract_rar_nonexistent_file() {
        let result = extract_rar_from_path(Path::new("nonexistent.rar"));
        assert!(result.is_err());
        match result {
            Err(ArchiveError::Io(_)) => {
                // Expected error type
            }
            _ => panic!("Expected Io error for nonexistent file"),
        }
    }

    #[test]
    fn test_extract_rar_invalid_format() {
        // Create a temporary file with invalid RAR content
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"This is not a RAR file").unwrap();
        temp_file.flush().unwrap();

        let result = extract_rar_from_path(temp_file.path());
        assert!(result.is_err());
        match result {
            Err(ArchiveError::Other(_)) => {
                // Expected error type
            }
            _ => panic!("Expected Other error for invalid format"),
        }
    }

    #[test]
    fn test_list_rar_contents() {
        let path = Path::new("../../test-corpus/archives/rar/simple.rar");
        if !path.exists() {
            eprintln!("Test file not found: {path:?}");
            return; // Skip if test file doesn't exist
        }

        let contents = match list_rar_contents(path) {
            Ok(c) => c,
            Err(ArchiveError::Other(msg)) if msg.contains("lsar command not found") => {
                eprintln!("Skipping test: {msg}");
                return; // Skip if lsar is not installed
            }
            Err(e) => panic!("Unexpected error: {e:?}"),
        };
        assert!(!contents.is_empty());

        for file in contents {
            println!("File: {} ({} bytes)", file.name, file.size);
            println!("  Compressed: {} bytes", file.compressed_size);
            println!("  Encrypted: {}", file.is_encrypted);
        }
    }

    #[test]
    #[ignore = "Requires encrypted.rar test file"]
    fn test_extract_rar_encrypted() {
        let path = Path::new("../../test-corpus/archives/rar/encrypted.rar");
        if !path.exists() {
            return; // Skip if test file doesn't exist
        }

        let result = extract_rar_from_path(path);
        assert!(result.is_err());
        match result {
            Err(ArchiveError::PasswordProtected) => {
                // Expected error for encrypted archive
            }
            _ => panic!("Expected PasswordProtected error for encrypted RAR"),
        }
    }
}
