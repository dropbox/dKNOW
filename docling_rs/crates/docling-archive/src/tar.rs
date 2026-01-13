//! TAR archive extraction and processing
//!
//! This module provides functionality for extracting files from TAR archives
//! (uncompressed, gzip, and bzip2 compressed) and processing them recursively.

use crate::error::ArchiveError;
use bzip2::read::BzDecoder;
use flate2::read::GzDecoder;
use log::warn;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Component, Path, PathBuf};
use tar::Archive;

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
fn sanitize_path(path: &Path) -> Option<PathBuf> {
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

/// Gzip magic bytes (RFC 1952)
/// First two bytes of any gzip-compressed data
const GZIP_MAGIC: [u8; 2] = [0x1f, 0x8b];

/// Bzip2 magic bytes
/// First two bytes 'B' 'Z' (0x42 0x5a)
const BZIP2_MAGIC: [u8; 2] = [0x42, 0x5a];

// Re-export types from zip module for consistency
pub use crate::zip::{ExtractedFile, FileInfo};

/// Compression type detected for TAR archives
///
/// Defaults to `None` (uncompressed TAR).
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum TarCompression {
    /// Uncompressed TAR
    #[default]
    None,
    /// Gzip compressed (.tar.gz, .tgz)
    Gzip,
    /// Bzip2 compressed (.tar.bz2, .tbz2)
    Bzip2,
}

impl TarCompression {
    /// Detect compression from file extension
    #[inline]
    #[must_use = "returns the detected compression type"]
    pub fn from_extension(path: &Path) -> Self {
        let extension = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        match extension.as_str() {
            "tgz" => Self::Gzip,
            "gz" => {
                // Check if it's .tar.gz
                if let Some(stem) = path.file_stem() {
                    if std::path::Path::new(stem)
                        .extension()
                        .is_some_and(|e| e.eq_ignore_ascii_case("tar"))
                    {
                        return Self::Gzip;
                    }
                }
                Self::None
            }
            "tbz2" | "tbz" => Self::Bzip2,
            "bz2" => {
                // Check if it's .tar.bz2
                if let Some(stem) = path.file_stem() {
                    if std::path::Path::new(stem)
                        .extension()
                        .is_some_and(|e| e.eq_ignore_ascii_case("tar"))
                    {
                        return Self::Bzip2;
                    }
                }
                Self::None
            }
            _ => Self::None,
        }
    }

    /// Detect compression from file magic bytes
    ///
    /// Gzip: 0x1f 0x8b (`GZIP_MAGIC`)
    /// Bzip2: 0x42 0x5a 'B' 'Z' (`BZIP2_MAGIC`)
    #[inline]
    #[must_use = "returns the detected compression type"]
    pub fn from_magic_bytes(bytes: &[u8]) -> Self {
        if bytes.len() < 4 {
            return Self::None;
        }

        // Check for gzip magic bytes
        if bytes[0] == GZIP_MAGIC[0] && bytes[1] == GZIP_MAGIC[1] {
            return Self::Gzip;
        }

        // Check for bzip2 magic bytes
        if bytes[0] == BZIP2_MAGIC[0] && bytes[1] == BZIP2_MAGIC[1] {
            return Self::Bzip2;
        }

        Self::None
    }
}

impl std::fmt::Display for TarCompression {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::None => "none",
            Self::Gzip => "gzip",
            Self::Bzip2 => "bzip2",
        };
        write!(f, "{s}")
    }
}

impl std::str::FromStr for TarCompression {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "none" | "uncompressed" | "plain" | "tar" => Ok(Self::None),
            "gzip" | "gz" | "tgz" => Ok(Self::Gzip),
            "bzip2" | "bz2" | "tbz2" | "tbz" => Ok(Self::Bzip2),
            _ => Err(format!(
                "Unknown TAR compression '{s}'. Expected: none, gzip, bzip2"
            )),
        }
    }
}

/// Extract all files from a TAR archive
///
/// This function opens a TAR archive (with automatic compression detection)
/// and extracts all files (excluding directories) into memory. Files that
/// exceed size limits are skipped with a warning.
///
/// Supported compression formats:
/// - Uncompressed (.tar)
/// - Gzip (.tar.gz, .tgz)
/// - Bzip2 (.tar.bz2, .tbz2)
///
/// # Arguments
///
/// * `path` - Path to the TAR archive file
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
/// - Unsupported compression format
/// - Archive exceeds size limits
///
/// # Examples
///
/// ```no_run
/// use docling_archive::tar::extract_tar_from_path;
/// use std::path::Path;
///
/// let files = extract_tar_from_path(Path::new("archive.tar.gz")).unwrap();
/// for file in files {
///     println!("Extracted: {} ({} bytes)", file.name, file.size);
/// }
/// ```
#[must_use = "this function returns extracted files that should be processed"]
pub fn extract_tar_from_path(path: &Path) -> Result<Vec<ExtractedFile>, ArchiveError> {
    // Detect compression from extension
    let compression = TarCompression::from_extension(path);

    let file = File::open(path)?;
    let reader = BufReader::new(file);

    // Create appropriate archive based on compression
    let mut files = Vec::new();

    match compression {
        TarCompression::None => {
            let mut archive = Archive::new(reader);
            extract_entries(&mut archive, &mut files)?;
        }
        TarCompression::Gzip => {
            let gz = GzDecoder::new(reader);
            let mut archive = Archive::new(gz);
            extract_entries(&mut archive, &mut files)?;
        }
        TarCompression::Bzip2 => {
            let bz = BzDecoder::new(reader);
            let mut archive = Archive::new(bz);
            extract_entries(&mut archive, &mut files)?;
        }
    }

    Ok(files)
}

/// Helper function to extract entries from a TAR archive
fn extract_entries<R: Read>(
    archive: &mut Archive<R>,
    files: &mut Vec<ExtractedFile>,
) -> Result<(), ArchiveError> {
    for entry in archive.entries()? {
        let mut entry = entry?;

        // Get entry metadata
        let header = entry.header();
        let entry_type = header.entry_type();

        // Skip directories, symlinks, and other special files
        if !entry_type.is_file() {
            continue;
        }

        let raw_path = entry.path()?.to_path_buf();
        let size = entry.header().size()?;

        // SECURITY: Sanitize path to prevent path traversal attacks
        // Malicious TAR files can contain entries like "../../../etc/passwd"
        let Some(sanitized_path) = sanitize_path(&raw_path) else {
            let raw_name = raw_path.to_string_lossy();
            warn!("Skipping invalid path: {raw_name} (path traversal attempt or empty)");
            continue;
        };
        let name = sanitized_path.to_string_lossy().to_string();

        // Skip macOS resource fork files (._filename)
        if let Some(file_name) = sanitized_path.file_name() {
            if let Some(name_str) = file_name.to_str() {
                if name_str.starts_with("._") {
                    continue;
                }
            }
        }

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
        entry.read_to_end(&mut contents)?;

        files.push(ExtractedFile {
            name: name.clone(),
            path: sanitized_path,
            size: contents.len(),
            contents,
        });
    }

    Ok(())
}

/// List files in a TAR archive without extracting contents
///
/// This is a lightweight operation that reads the TAR headers
/// to enumerate files without decompressing their contents.
///
/// # Arguments
///
/// * `path` - Path to the TAR archive file
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
/// use docling_archive::tar::list_tar_contents;
/// use std::path::Path;
///
/// let files = list_tar_contents(Path::new("archive.tar.gz")).unwrap();
/// for file in files {
///     println!("{} - {} bytes", file.name, file.size);
/// }
/// ```
#[must_use = "this function returns archive file listing that should be processed"]
pub fn list_tar_contents(path: &Path) -> Result<Vec<FileInfo>, ArchiveError> {
    // Detect compression from extension
    let compression = TarCompression::from_extension(path);

    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut files = Vec::new();

    match compression {
        TarCompression::None => {
            let mut archive = Archive::new(reader);
            list_entries(&mut archive, &mut files)?;
        }
        TarCompression::Gzip => {
            let gz = GzDecoder::new(reader);
            let mut archive = Archive::new(gz);
            list_entries(&mut archive, &mut files)?;
        }
        TarCompression::Bzip2 => {
            let bz = BzDecoder::new(reader);
            let mut archive = Archive::new(bz);
            list_entries(&mut archive, &mut files)?;
        }
    }

    Ok(files)
}

/// Helper function to list entries in a TAR archive
fn list_entries<R: Read>(
    archive: &mut Archive<R>,
    files: &mut Vec<FileInfo>,
) -> Result<(), ArchiveError> {
    for entry in archive.entries()? {
        let entry = entry?;

        // Get entry metadata
        let header = entry.header();
        let entry_type = header.entry_type();

        // Skip directories and special files
        if !entry_type.is_file() {
            continue;
        }

        let path = entry.path()?.to_path_buf();
        let name = path.to_string_lossy().to_string();
        let size = entry.header().size()?;

        // Skip macOS resource fork files (._filename)
        if let Some(file_name) = path.file_name() {
            if let Some(name_str) = file_name.to_str() {
                if name_str.starts_with("._") {
                    continue;
                }
            }
        }

        files.push(FileInfo {
            name,
            size,
            compressed_size: size, // TAR doesn't store separate compressed size
            is_encrypted: false,   // TAR doesn't support encryption
        });
    }

    Ok(())
}

/// Extract files from TAR archive using a streaming approach
///
/// This function processes files one-at-a-time instead of loading all into memory,
/// which is more memory-efficient for large archives.
///
/// # Arguments
///
/// * `path` - Path to the TAR archive file
/// * `processor` - Callback function to process each extracted file
///
/// # Errors
///
/// Returns `ArchiveError` if archive operations fail, or propagates errors
/// from the processor callback.
///
/// # Examples
///
/// ```no_run
/// use docling_archive::tar::extract_tar_streaming;
/// use std::path::Path;
///
/// extract_tar_streaming(Path::new("archive.tar.gz"), |file| {
///     println!("Processing: {}", file.name);
///     // Process file contents...
///     Ok(())
/// }).unwrap();
/// ```
#[must_use = "this function returns a Result that should be checked for errors"]
pub fn extract_tar_streaming<F>(path: &Path, mut processor: F) -> Result<(), ArchiveError>
where
    F: FnMut(ExtractedFile) -> Result<(), Box<dyn std::error::Error>>,
{
    // Detect compression from extension
    let compression = TarCompression::from_extension(path);

    let file = File::open(path)?;
    let reader = BufReader::new(file);

    match compression {
        TarCompression::None => {
            let mut archive = Archive::new(reader);
            stream_entries(&mut archive, &mut processor)?;
        }
        TarCompression::Gzip => {
            let gz = GzDecoder::new(reader);
            let mut archive = Archive::new(gz);
            stream_entries(&mut archive, &mut processor)?;
        }
        TarCompression::Bzip2 => {
            let bz = BzDecoder::new(reader);
            let mut archive = Archive::new(bz);
            stream_entries(&mut archive, &mut processor)?;
        }
    }

    Ok(())
}

/// Helper function to stream entries from a TAR archive
fn stream_entries<R: Read, F>(
    archive: &mut Archive<R>,
    processor: &mut F,
) -> Result<(), ArchiveError>
where
    F: FnMut(ExtractedFile) -> Result<(), Box<dyn std::error::Error>>,
{
    for entry in archive.entries()? {
        let mut entry = entry?;

        // Get entry metadata
        let header = entry.header();
        let entry_type = header.entry_type();

        // Skip directories and special files
        if !entry_type.is_file() {
            continue;
        }

        let raw_path = entry.path()?.to_path_buf();
        let size = entry.header().size()?;

        // SECURITY: Sanitize path to prevent path traversal attacks
        let Some(sanitized_path) = sanitize_path(&raw_path) else {
            let raw_name = raw_path.to_string_lossy();
            warn!("Skipping invalid path: {raw_name} (path traversal attempt or empty)");
            continue;
        };
        let name = sanitized_path.to_string_lossy().to_string();

        // Skip macOS resource fork files (._filename)
        if let Some(file_name) = sanitized_path.file_name() {
            if let Some(name_str) = file_name.to_str() {
                if name_str.starts_with("._") {
                    continue;
                }
            }
        }

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
        entry.read_to_end(&mut contents)?;

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
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use tar::Builder;
    use tempfile::NamedTempFile;

    /// Helper: Create a simple test TAR file (uncompressed)
    fn create_test_tar() -> Result<NamedTempFile, Box<dyn std::error::Error>> {
        let temp_file = NamedTempFile::new()?;
        let mut builder = Builder::new(temp_file.reopen()?);

        // Add test files using append_data (simpler API)
        let data1 = b"Hello from file 1";
        let mut header1 = tar::Header::new_gnu();
        header1.set_path("file1.txt")?;
        header1.set_size(data1.len() as u64);
        header1.set_mode(0o644);
        header1.set_cksum();
        builder.append(&header1, &data1[..])?;

        let data2 = b"Hello from file 2";
        let mut header2 = tar::Header::new_gnu();
        header2.set_path("file2.txt")?;
        header2.set_size(data2.len() as u64);
        header2.set_mode(0o644);
        header2.set_cksum();
        builder.append(&header2, &data2[..])?;

        let data3 = b"Hello from subdirectory";
        let mut header3 = tar::Header::new_gnu();
        header3.set_path("subdir/file3.txt")?;
        header3.set_size(data3.len() as u64);
        header3.set_mode(0o644);
        header3.set_cksum();
        builder.append(&header3, &data3[..])?;

        builder.finish()?;

        Ok(temp_file)
    }

    /// Helper: Create a gzip compressed TAR file
    fn create_test_tar_gz() -> Result<NamedTempFile, Box<dyn std::error::Error>> {
        use std::io::{Seek, SeekFrom, Write};

        // Create temp file with .tar.gz extension so compression detection works
        let mut temp_file = tempfile::Builder::new().suffix(".tar.gz").tempfile()?;

        // Create TAR in memory first
        let mut tar_data = Vec::new();
        {
            let mut builder = Builder::new(&mut tar_data);

            // Add test files
            let data1 = b"Hello from file 1";
            let mut header1 = tar::Header::new_gnu();
            header1.set_path("file1.txt")?;
            header1.set_size(data1.len() as u64);
            header1.set_mode(0o644);
            header1.set_cksum();
            builder.append(&header1, &data1[..])?;

            let data2 = b"Hello from file 2";
            let mut header2 = tar::Header::new_gnu();
            header2.set_path("file2.txt")?;
            header2.set_size(data2.len() as u64);
            header2.set_mode(0o644);
            header2.set_cksum();
            builder.append(&header2, &data2[..])?;

            builder.finish()?;
        }

        // Compress and write to file
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&tar_data)?;
        let compressed = encoder.finish()?;

        temp_file.write_all(&compressed)?;
        temp_file.flush()?;
        temp_file.as_file().sync_all()?;
        // Rewind to beginning for reading
        temp_file.seek(SeekFrom::Start(0))?;

        Ok(temp_file)
    }

    #[test]
    fn test_compression_detection() {
        assert_eq!(
            TarCompression::from_extension(Path::new("archive.tar")),
            TarCompression::None
        );
        assert_eq!(
            TarCompression::from_extension(Path::new("archive.tar.gz")),
            TarCompression::Gzip
        );
        assert_eq!(
            TarCompression::from_extension(Path::new("archive.tgz")),
            TarCompression::Gzip
        );
        assert_eq!(
            TarCompression::from_extension(Path::new("archive.tar.bz2")),
            TarCompression::Bzip2
        );
        assert_eq!(
            TarCompression::from_extension(Path::new("archive.tbz2")),
            TarCompression::Bzip2
        );
    }

    #[test]
    fn test_tar_compression_display() {
        assert_eq!(format!("{}", TarCompression::None), "none");
        assert_eq!(format!("{}", TarCompression::Gzip), "gzip");
        assert_eq!(format!("{}", TarCompression::Bzip2), "bzip2");
    }

    #[test]
    #[allow(clippy::similar_names)]
    fn test_extract_tar_basic() {
        let temp_tar = create_test_tar().expect("Failed to create test TAR");
        let files = extract_tar_from_path(temp_tar.path()).expect("Failed to extract TAR");

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
    fn test_extract_tar_gz() {
        let temp_tar = create_test_tar_gz().expect("Failed to create test TAR.GZ");

        // Keep temp file alive for the duration of the test
        let _keep_alive = &temp_tar;
        let path = temp_tar.path();

        let files = extract_tar_from_path(path).expect("Failed to extract TAR.GZ");

        assert_eq!(files.len(), 2, "Should extract 2 files");

        // Check first file
        let file1 = files
            .iter()
            .find(|f| f.name == "file1.txt")
            .expect("file1.txt not found");
        assert_eq!(file1.contents, b"Hello from file 1");
    }

    #[test]
    #[allow(clippy::similar_names)]
    fn test_list_tar_contents() {
        let temp_tar = create_test_tar().expect("Failed to create test TAR");
        let files = list_tar_contents(temp_tar.path()).expect("Failed to list TAR contents");

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
    fn test_extract_tar_streaming() {
        let temp_tar = create_test_tar().expect("Failed to create test TAR");
        let mut count = 0;

        extract_tar_streaming(temp_tar.path(), |file| {
            count += 1;
            assert!(!file.contents.is_empty());
            Ok(())
        })
        .expect("Failed to stream TAR");

        assert_eq!(count, 3, "Should process 3 files");
    }

    #[test]
    fn test_nonexistent_file() {
        let result = extract_tar_from_path(Path::new("nonexistent.tar"));
        assert!(result.is_err(), "Should fail for nonexistent file");
    }

    #[test]
    fn test_tar_compression_from_str() {
        // Exact matches
        assert_eq!(
            "none".parse::<TarCompression>().unwrap(),
            TarCompression::None
        );
        assert_eq!(
            "gzip".parse::<TarCompression>().unwrap(),
            TarCompression::Gzip
        );
        assert_eq!(
            "bzip2".parse::<TarCompression>().unwrap(),
            TarCompression::Bzip2
        );

        // Aliases
        assert_eq!(
            "uncompressed".parse::<TarCompression>().unwrap(),
            TarCompression::None
        );
        assert_eq!(
            "tar".parse::<TarCompression>().unwrap(),
            TarCompression::None
        );
        assert_eq!(
            "gz".parse::<TarCompression>().unwrap(),
            TarCompression::Gzip
        );
        assert_eq!(
            "tgz".parse::<TarCompression>().unwrap(),
            TarCompression::Gzip
        );
        assert_eq!(
            "bz2".parse::<TarCompression>().unwrap(),
            TarCompression::Bzip2
        );
        assert_eq!(
            "tbz2".parse::<TarCompression>().unwrap(),
            TarCompression::Bzip2
        );

        // Case insensitive
        assert_eq!(
            "GZIP".parse::<TarCompression>().unwrap(),
            TarCompression::Gzip
        );
        assert_eq!(
            "BZip2".parse::<TarCompression>().unwrap(),
            TarCompression::Bzip2
        );

        // Invalid
        assert!("invalid".parse::<TarCompression>().is_err());
        assert!("zip".parse::<TarCompression>().is_err());
    }

    #[test]
    fn test_tar_compression_roundtrip() {
        for compression in [
            TarCompression::None,
            TarCompression::Gzip,
            TarCompression::Bzip2,
        ] {
            let s = compression.to_string();
            let parsed: TarCompression = s.parse().unwrap();
            assert_eq!(parsed, compression);
        }
    }
}
