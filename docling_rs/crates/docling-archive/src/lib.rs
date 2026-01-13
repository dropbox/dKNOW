//! Archive format support for docling
//!
//! This crate provides functionality for extracting and processing archive formats
//! including ZIP, TAR, 7Z, and RAR. It enables recursive document extraction from
//! archives, making it easy to process collections of documents packaged in archives.
//!
//! # Features
//!
//! - **ZIP archives**: Full support for ZIP file extraction
//! - **TAR archives**: Support for TAR, TAR.GZ, and TAR.BZ2 extraction
//! - **7Z archives**: Support for 7Z file extraction with multiple compression methods
//! - **RAR archives**: Support for RAR4 and RAR5 file extraction (including multi-volume)
//! - **Recursive processing**: Handle nested archives (ZIP within ZIP, RAR within RAR, etc.)
//! - **Streaming extraction**: Memory-efficient processing of large archives
//! - **Error handling**: Graceful handling of encrypted, corrupted, or oversized archives
//!
//! # Usage
//!
//! ## Extract all files from a ZIP archive
//!
//! ```no_run
//! use docling_archive::zip::extract_zip_from_path;
//! use std::path::Path;
//!
//! let files = extract_zip_from_path(Path::new("archive.zip")).unwrap();
//! for file in files {
//!     println!("Extracted: {} ({} bytes)", file.name, file.size);
//!     // Process file.contents...
//! }
//! ```
//!
//! ## Extract all files from a TAR archive
//!
//! ```no_run
//! use docling_archive::tar::extract_tar_from_path;
//! use std::path::Path;
//!
//! // Works with .tar, .tar.gz, .tgz, .tar.bz2
//! let files = extract_tar_from_path(Path::new("archive.tar.gz")).unwrap();
//! for file in files {
//!     println!("Extracted: {} ({} bytes)", file.name, file.size);
//! }
//! ```
//!
//! ## List archive contents without extraction
//!
//! ```no_run
//! use docling_archive::zip::list_zip_contents;
//! use std::path::Path;
//!
//! let files = list_zip_contents(Path::new("archive.zip")).unwrap();
//! for file in files {
//!     println!("{} - {} bytes", file.name, file.size);
//! }
//! ```
//!
//! ## Stream processing for large archives
//!
//! ```no_run
//! use docling_archive::zip::extract_zip_streaming;
//! use std::path::Path;
//!
//! extract_zip_streaming(Path::new("large_archive.zip"), |file| {
//!     println!("Processing: {}", file.name);
//!     // Process file contents...
//!     Ok(())
//! }).unwrap();
//! ```

pub mod error;
pub mod rar;
pub mod sevenz;
pub mod tar;
pub mod zip;

// =============================================================================
// Archive Constants
// =============================================================================

/// Maximum size for a single file within an archive (100 MB).
///
/// Files exceeding this limit are skipped during extraction to prevent
/// memory exhaustion from zip bombs or excessively large files.
pub const MAX_FILE_SIZE: u64 = 100_000_000;

/// Maximum nesting depth for recursive archive extraction.
///
/// Limits how deeply nested archives can be extracted (e.g., ZIP within ZIP).
/// Prevents infinite recursion from malicious or corrupted archives.
pub const MAX_NESTING_DEPTH: usize = 10;

// Re-export commonly used types
pub use error::ArchiveError;
pub use rar::{extract_rar_from_path, extract_rar_streaming, list_rar_contents};
pub use sevenz::{extract_7z_from_path, extract_7z_streaming, list_7z_contents};
pub use tar::{extract_tar_from_path, extract_tar_streaming, list_tar_contents};
pub use zip::{extract_zip_from_path, extract_zip_streaming, list_zip_contents};
pub use zip::{ExtractedFile, FileInfo};
