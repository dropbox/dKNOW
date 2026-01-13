//! Archive document backend for ZIP, TAR, 7Z, and RAR formats
//!
//! This module provides archive extraction and document listing capabilities
//! for various archive formats supported by the docling-archive crate.

// Clippy pedantic allows:
// - Unit struct &self convention
// - Timestamp casts are safe for reasonable values
#![allow(clippy::trivially_copy_pass_by_ref)]
#![allow(clippy::cast_possible_wrap)]

use crate::traits::{BackendOptions, DocumentBackend};
use crate::utils::{create_list_item, create_section_header, create_text_item};
use docling_archive::{
    extract_7z_from_path, extract_rar_from_path, extract_tar_from_path, extract_zip_from_path,
    ExtractedFile,
};
use docling_core::{
    content::{CoordOrigin, DocItem, ProvenanceItem},
    DoclingError, Document, DocumentMetadata, InputFormat,
};
use std::path::Path;

/// Archive backend for processing archive files
///
/// Supports:
/// - ZIP (.zip)
/// - TAR (.tar, .tar.gz, .tgz, .tar.bz2, .tbz2)
/// - 7Z (.7z)
/// - RAR (.rar, including multi-volume RAR files)
///
/// Extracts and lists all files contained in the archive with metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArchiveBackend {
    format: InputFormat,
}

#[allow(clippy::cast_precision_loss)] // Archive sizes don't need full precision for display
#[allow(clippy::cast_possible_truncation)] // File sizes on 32-bit systems are limited anyway
#[allow(clippy::cast_possible_wrap)] // Unix timestamps fit in i64 for centuries
impl ArchiveBackend {
    /// Create a new archive backend for the specified format
    ///
    /// # Errors
    ///
    /// Returns an error if the format is not an archive format.
    #[inline]
    #[must_use = "creating a backend that is not used is a waste of resources"]
    pub fn new(format: InputFormat) -> Result<Self, DoclingError> {
        if !format.is_archive() {
            return Err(DoclingError::FormatError(format!(
                "Format {format:?} is not an archive format"
            )));
        }
        Ok(Self { format })
    }

    /// Format byte size as human-readable string (KB, MB, or bytes)
    #[inline]
    fn format_size_human(size: usize) -> String {
        if size >= 1_000_000 {
            format!("{:.2} MB", size as f64 / 1_000_000.0)
        } else if size >= 1_000 {
            format!("{:.2} KB", size as f64 / 1_000.0)
        } else {
            format!("{size} bytes")
        }
    }

    /// Count files by extension/type
    #[inline]
    fn count_file_types(files: &[ExtractedFile]) -> std::collections::HashMap<String, usize> {
        let mut type_counts = std::collections::HashMap::new();
        for file in files {
            let file_path = std::path::Path::new(&file.name);
            let extension = file_path.extension().and_then(|e| e.to_str()).map_or_else(
                || {
                    if file.name.starts_with('.') && !file.name.starts_with("./") {
                        "dotfile".to_string()
                    } else {
                        "no extension".to_string()
                    }
                },
                ToString::to_string,
            );
            *type_counts.entry(extension.to_lowercase()).or_insert(0) += 1;
        }
        type_counts
    }

    /// Build type summary strings (e.g., "3 TXT files", "1 dotfile")
    fn build_type_summary(type_counts: &std::collections::HashMap<String, usize>) -> Vec<String> {
        let mut sorted_types: Vec<_> = type_counts.iter().collect();
        sorted_types.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));

        sorted_types
            .into_iter()
            .map(|(ext, count)| {
                let ext_upper = ext.to_uppercase();
                let type_label = if ext == "dotfile" {
                    if *count == 1 {
                        "dotfile".to_string()
                    } else {
                        "dotfiles".to_string()
                    }
                } else if ext == "no extension" {
                    if *count == 1 {
                        "file with no extension".to_string()
                    } else {
                        "files with no extension".to_string()
                    }
                } else if *count == 1 {
                    format!("{ext_upper} file")
                } else {
                    format!("{ext_upper} files")
                };
                format!("{count} {type_label}")
            })
            .collect()
    }

    /// Calculate compression info string
    #[inline]
    fn compression_info_string(
        compressed_size: Option<usize>,
        total_size: usize,
        is_compressed: bool,
    ) -> String {
        if !is_compressed {
            return String::new();
        }
        if let Some(compressed) = compressed_size {
            if compressed > 0 && total_size > 0 {
                let ratio = (total_size as f64) / (compressed as f64);
                return format!(" [Compression ratio: {ratio:.1}x]");
            }
        }
        String::new()
    }

    /// Build file description string
    #[inline]
    fn build_file_description(num_files: usize, type_summary: &[String]) -> String {
        if num_files == 1 && !type_summary.is_empty() {
            type_summary[0].clone()
        } else {
            let file_word = if num_files == 1 { "file" } else { "files" };
            let type_breakdown = if type_summary.is_empty() {
                String::new()
            } else {
                format!(" ({})", type_summary.join(", "))
            };
            format!("{num_files} {file_word}{type_breakdown}")
        }
    }

    /// Build TAR archive summary string
    #[inline]
    fn build_tar_summary(
        file_description: &str,
        archive_size: Option<usize>,
        total_size: usize,
        compression_info: &str,
    ) -> String {
        archive_size.map_or_else(
            || {
                let byte_word = if total_size == 1 { "byte" } else { "bytes" };
                format!("{file_description}, {total_size} {byte_word} total{compression_info}")
            },
            |archive_size| {
                let overhead = archive_size.saturating_sub(total_size);
                let archive_word = if archive_size == 1 { "byte" } else { "bytes" };
                let content_word = if total_size == 1 { "byte" } else { "bytes" };
                format!(
                    "{file_description}, {archive_size} {archive_word} total ({total_size} {content_word} content + {overhead} bytes TAR overhead){compression_info}"
                )
            },
        )
    }

    /// Build non-TAR archive summary string
    #[inline]
    fn build_default_summary(
        file_description: &str,
        total_size: usize,
        compression_info: &str,
    ) -> String {
        let byte_word = if total_size == 1 { "byte" } else { "bytes" };
        format!("{file_description}, {total_size} {byte_word} total{compression_info}")
    }

    /// Extract files from archive based on format
    fn extract_archive(&self, path: &Path) -> Result<Vec<ExtractedFile>, DoclingError> {
        let files = match self.format {
            InputFormat::Zip => extract_zip_from_path(path),
            InputFormat::Tar => extract_tar_from_path(path),
            InputFormat::SevenZ => extract_7z_from_path(path),
            InputFormat::Rar => extract_rar_from_path(path),
            _ => {
                return Err(DoclingError::FormatError(format!(
                    "Unsupported archive format: {:?}",
                    self.format
                )))
            }
        };

        files.map_err(|e| DoclingError::BackendError(format!("Failed to extract archive: {e}")))
    }

    /// Detect TAR compression type from file path
    #[inline]
    fn detect_tar_compression(path: &Path) -> &'static str {
        let extension = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_lowercase();

        match extension.as_str() {
            "tgz" => "TAR.GZ",
            "gz" => {
                // Check if it's .tar.gz
                if let Some(stem) = path.file_stem() {
                    if std::path::Path::new(stem)
                        .extension()
                        .is_some_and(|e| e.eq_ignore_ascii_case("tar"))
                    {
                        return "TAR.GZ";
                    }
                }
                "TAR"
            }
            "tbz2" | "tbz" => "TAR.BZ2",
            "bz2" => {
                // Check if it's .tar.bz2
                if let Some(stem) = path.file_stem() {
                    if std::path::Path::new(stem)
                        .extension()
                        .is_some_and(|e| e.eq_ignore_ascii_case("tar"))
                    {
                        return "TAR.BZ2";
                    }
                }
                "TAR"
            }
            _ => "TAR",
        }
    }

    /// Generate `DocItems` from archive contents
    fn generate_docitems(
        files: &[ExtractedFile],
        archive_name: &str,
        format: InputFormat,
        archive_path: &Path,
    ) -> Vec<DocItem> {
        let mut doc_items = Vec::new();

        // Title header
        doc_items.push(create_section_header(
            doc_items.len(),
            format!("Archive Contents: {archive_name}"),
            1,
            Self::create_provenance(1),
        ));

        if files.is_empty() {
            doc_items.push(create_text_item(
                doc_items.len(),
                "(Empty archive)".to_string(),
                Self::create_provenance(1),
            ));
            return doc_items;
        }

        let archive_size_bytes = std::fs::metadata(archive_path)
            .ok()
            .map(|m| m.len() as usize);

        // Add Format Information section for TAR/7Z archives
        Self::add_format_info_section(&mut doc_items, format, archive_path, archive_size_bytes);

        // Build file statistics
        let num_files = files.len();
        let total_size: usize = files.iter().map(|f| f.size).sum();
        let type_counts = Self::count_file_types(files);
        let type_summary = Self::build_type_summary(&type_counts);
        let file_description = Self::build_file_description(num_files, &type_summary);

        // Calculate compression info
        let is_compressed = format == InputFormat::SevenZ
            || (format == InputFormat::Tar && Self::detect_tar_compression(archive_path) != "TAR");
        let compression_info =
            Self::compression_info_string(archive_size_bytes, total_size, is_compressed);

        // Add summary section
        Self::add_summary_section(
            &mut doc_items,
            format,
            &file_description,
            archive_size_bytes,
            total_size,
            &compression_info,
        );

        // Add contents section
        Self::add_contents_section(&mut doc_items, files);

        doc_items
    }

    /// Add format information section for TAR/7Z archives
    fn add_format_info_section(
        doc_items: &mut Vec<DocItem>,
        format: InputFormat,
        archive_path: &Path,
        archive_size_bytes: Option<usize>,
    ) {
        if format != InputFormat::Tar && format != InputFormat::SevenZ {
            return;
        }

        doc_items.push(create_section_header(
            doc_items.len(),
            "Format Information".to_string(),
            2,
            Self::create_provenance(1),
        ));

        let format_type = if format == InputFormat::Tar {
            Self::detect_tar_compression(archive_path)
        } else {
            "7Z"
        };

        let format_text = archive_size_bytes.map_or_else(
            || format!("Format: {format_type}"),
            |archive_size| {
                format!(
                    "Format: {format_type}, Archive size: {}",
                    Self::format_size_human(archive_size)
                )
            },
        );

        doc_items.push(create_text_item(
            doc_items.len(),
            format_text,
            Self::create_provenance(1),
        ));
    }

    /// Add archive summary section
    fn add_summary_section(
        doc_items: &mut Vec<DocItem>,
        format: InputFormat,
        file_description: &str,
        archive_size_bytes: Option<usize>,
        total_size: usize,
        compression_info: &str,
    ) {
        doc_items.push(create_section_header(
            doc_items.len(),
            "Archive Summary".to_string(),
            2,
            Self::create_provenance(1),
        ));

        let summary = if format == InputFormat::Tar {
            Self::build_tar_summary(
                file_description,
                archive_size_bytes,
                total_size,
                compression_info,
            )
        } else {
            Self::build_default_summary(file_description, total_size, compression_info)
        };

        doc_items.push(create_text_item(
            doc_items.len(),
            summary,
            Self::create_provenance(1),
        ));
    }

    /// Add contents section with file listings
    fn add_contents_section(doc_items: &mut Vec<DocItem>, files: &[ExtractedFile]) {
        doc_items.push(create_section_header(
            doc_items.len(),
            "Contents".to_string(),
            2,
            Self::create_provenance(1),
        ));

        for file in files {
            let file_text = format!("{} ({} bytes)", file.name, file.size);
            doc_items.push(create_list_item(
                doc_items.len(),
                file_text,
                "- ".to_string(),
                false,
                Self::create_provenance(1),
                None,
            ));
        }
    }

    /// Create provenance metadata for archive content
    ///
    /// Returns a Vec containing a single `ProvenanceItem` for the given page.
    /// This is the standard format expected by `DocItem` creation functions.
    #[inline]
    fn create_provenance(page_no: usize) -> Vec<ProvenanceItem> {
        vec![crate::utils::create_default_provenance(
            page_no,
            CoordOrigin::TopLeft,
        )]
    }

    /// Generate markdown from `DocItems`
    fn docitems_to_markdown(doc_items: &[DocItem]) -> String {
        let mut markdown = String::new();
        let mut last_was_list_item = false;

        for item in doc_items {
            match item {
                DocItem::Text { text, .. } => {
                    // Add blank line after list section if previous was list item
                    if last_was_list_item {
                        markdown.push('\n');
                    }
                    markdown.push_str(text);
                    markdown.push_str("\n\n");
                    last_was_list_item = false;
                }
                DocItem::SectionHeader { text, level, .. } => {
                    // Add blank line after list section if previous was list item
                    if last_was_list_item {
                        markdown.push('\n');
                    }
                    markdown.push_str(&"#".repeat(*level));
                    markdown.push(' ');
                    markdown.push_str(text);
                    markdown.push_str("\n\n");
                    last_was_list_item = false;
                }
                DocItem::ListItem { text, marker, .. } => {
                    markdown.push_str(marker);
                    markdown.push_str(text);
                    markdown.push('\n');
                    last_was_list_item = true;
                }
                _ => {
                    // No other types expected for archives
                    last_was_list_item = false;
                }
            }
        }

        // Add final blank line if document ends with list
        if last_was_list_item {
            markdown.push('\n');
        }

        markdown
    }
}

impl DocumentBackend for ArchiveBackend {
    #[inline]
    fn format(&self) -> InputFormat {
        self.format
    }

    fn parse_bytes(
        &self,
        _data: &[u8],
        _options: &BackendOptions,
    ) -> Result<Document, DoclingError> {
        // Archives cannot be parsed from bytes directly - they need file path for extraction
        Err(DoclingError::BackendError(
            "Archive parsing from bytes not supported. Use parse_file instead.".to_string(),
        ))
    }

    fn parse_file<P: AsRef<Path>>(
        &self,
        path: P,
        _options: &BackendOptions,
    ) -> Result<Document, DoclingError> {
        let path_ref = path.as_ref();
        let full_path = path_ref.display().to_string();
        let archive_name = path_ref
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("archive");

        // Helper to add filename context to errors
        let add_context = |err: DoclingError| -> DoclingError {
            match err {
                DoclingError::BackendError(msg) => {
                    DoclingError::BackendError(format!("{msg}: {full_path}"))
                }
                other => other,
            }
        };

        // Extract archive files
        let files = self.extract_archive(path_ref).map_err(add_context)?;

        // Generate DocItems
        let doc_items = Self::generate_docitems(&files, archive_name, self.format, path_ref);

        // Generate markdown from DocItems
        let markdown = Self::docitems_to_markdown(&doc_items);
        let num_characters = markdown.chars().count();

        // Get archive modification time (N=1693)
        let modified = path_ref
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| {
                use chrono::DateTime;
                DateTime::from_timestamp(
                    t.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() as i64,
                    0,
                )
            });

        // Create document
        Ok(Document {
            markdown,
            format: self.format,
            metadata: DocumentMetadata {
                num_pages: None,
                num_characters,
                title: Some(archive_name.to_string()),
                author: None,
                created: None,
                modified,
                language: None,
                subject: None,
                exif: None,
            },
            content_blocks: Some(doc_items),
            docling_document: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_archive_backend_creation() {
        // Valid archive formats
        assert!(
            ArchiveBackend::new(InputFormat::Zip).is_ok(),
            "ZIP format should be supported"
        );
        assert!(
            ArchiveBackend::new(InputFormat::Tar).is_ok(),
            "TAR format should be supported"
        );
        assert!(
            ArchiveBackend::new(InputFormat::SevenZ).is_ok(),
            "7Z format should be supported"
        );
        assert!(
            ArchiveBackend::new(InputFormat::Rar).is_ok(),
            "RAR format should be supported"
        );

        // Invalid format
        assert!(
            ArchiveBackend::new(InputFormat::Pdf).is_err(),
            "PDF format should be rejected for archive backend"
        );
    }

    #[test]
    fn test_docitem_generation_empty() {
        let files = vec![];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "test.zip",
            InputFormat::Zip,
            Path::new("test.zip"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);
        assert!(markdown.contains("Empty archive"));
    }

    #[test]
    fn test_docitem_generation_with_files() {
        use std::path::PathBuf;
        let files = vec![
            ExtractedFile {
                name: "file1.txt".to_string(),
                path: PathBuf::from("file1.txt"),
                contents: vec![],
                size: 100,
            },
            ExtractedFile {
                name: "file2.txt".to_string(),
                path: PathBuf::from("file2.txt"),
                contents: vec![],
                size: 200,
            },
        ];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "test.zip",
            InputFormat::Zip,
            Path::new("test.zip"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);
        assert!(markdown.contains("2 files"));
        assert!(markdown.contains("300 bytes"));
        assert!(markdown.contains("file1.txt"));
        assert!(markdown.contains("file2.txt"));
    }

    // ========================================
    // CATEGORY 1: Metadata Tests (3 tests)
    // Archives don't have traditional metadata (no author, dates, etc.)
    // Instead, test document metadata generation (title, num_characters)
    // ========================================

    #[test]
    fn test_archive_metadata_title_from_filename() {
        // Archive title should be extracted from filename
        use std::path::PathBuf;
        let files = vec![ExtractedFile {
            name: "readme.txt".to_string(),
            path: PathBuf::from("readme.txt"),
            contents: vec![],
            size: 50,
        }];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "myarchive.zip",
            InputFormat::Zip,
            Path::new("myarchive.zip"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        // Verify title appears in header
        assert!(markdown.contains("Archive Contents: myarchive.zip"));

        // Verify metadata fields would be set correctly
        let num_chars = markdown.chars().count();
        assert!(
            num_chars > 0,
            "Markdown output should have non-zero character count"
        );
    }

    #[test]
    fn test_archive_metadata_character_count() {
        // Character count should match generated markdown length
        use std::path::PathBuf;
        let files = vec![
            ExtractedFile {
                name: "file1.txt".to_string(),
                path: PathBuf::from("file1.txt"),
                contents: vec![],
                size: 100,
            },
            ExtractedFile {
                name: "file2.txt".to_string(),
                path: PathBuf::from("file2.txt"),
                contents: vec![],
                size: 200,
            },
        ];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "test.zip",
            InputFormat::Zip,
            Path::new("test.zip"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        let char_count = markdown.chars().count();
        assert!(
            char_count > 100,
            "Markdown output should be substantial (>100 chars) for archive with files"
        ); // Should be substantial markdown output
        assert!(
            char_count < 1000,
            "Markdown output should not be excessively long (<1000 chars) for 2 files"
        ); // Should not be excessively long for 2 files
    }

    #[test]
    fn test_archive_metadata_empty_archive() {
        // Empty archive should have minimal character count
        let files = vec![];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "empty.zip",
            InputFormat::Zip,
            Path::new("empty.zip"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        let char_count = markdown.chars().count();
        assert!(
            char_count > 0,
            "Empty archive should still have header and message"
        ); // Should have header and "Empty archive" message
        assert!(
            char_count < 200,
            "Empty archive markdown should be brief (<200 chars)"
        ); // Should be brief for empty archive
        assert!(markdown.contains("Empty archive"));
    }

    // ========================================
    // CATEGORY 2: DocItem Generation Tests (3 tests)
    // Test DocItem types: SectionHeader, Text, ListItem
    // ========================================

    #[test]
    fn test_docitem_types_single_file() {
        // Single file archive should generate: SectionHeader (title) + Text (summary) + SectionHeader (contents) + ListItem (file)
        use std::path::PathBuf;
        let files = vec![ExtractedFile {
            name: "single.txt".to_string(),
            path: PathBuf::from("single.txt"),
            contents: vec![],
            size: 42,
        }];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "test.zip",
            InputFormat::Zip,
            Path::new("test.zip"),
        );

        assert_eq!(doc_items.len(), 5, "Single file archive should generate 5 DocItems: Title + Summary header + Summary text + Contents header + 1 file"); // Title + Summary header + Summary text + Contents header + 1 file

        // Verify DocItem types
        match &doc_items[0] {
            DocItem::SectionHeader { text, level, .. } => {
                assert_eq!(*level, 1, "Title header should be level 1");
                assert!(
                    text.contains("Archive Contents"),
                    "Title should contain 'Archive Contents'"
                );
            }
            _ => panic!("Expected SectionHeader for title"),
        }

        match &doc_items[1] {
            DocItem::SectionHeader { text, level, .. } => {
                assert_eq!(*level, 2, "Summary header should be level 2");
                assert_eq!(
                    text, "Archive Summary",
                    "Summary header text should be 'Archive Summary'"
                );
            }
            _ => panic!("Expected SectionHeader for summary header"),
        }

        match &doc_items[2] {
            DocItem::Text { text, .. } => {
                // After N=1893: single-file archives show type directly (e.g., "1 TXT file")
                assert!(
                    text.contains("1 TXT file") || text.contains("1 file"),
                    "Summary should mention 1 file or 1 TXT file"
                );
                assert!(
                    text.contains("42 bytes"),
                    "Summary should mention file size (42 bytes)"
                );
            }
            _ => panic!("Expected Text for summary text"),
        }

        match &doc_items[3] {
            DocItem::SectionHeader { text, level, .. } => {
                assert_eq!(*level, 2, "Contents header should be level 2");
                assert_eq!(
                    text, "Contents",
                    "Contents header text should be 'Contents'"
                );
            }
            _ => panic!("Expected SectionHeader for contents"),
        }

        match &doc_items[4] {
            DocItem::ListItem { text, marker, .. } => {
                assert_eq!(marker, "- ");
                assert!(text.contains("single.txt"));
                assert!(text.contains("42 bytes"));
            }
            _ => panic!("Expected ListItem for file"),
        }
    }

    #[test]
    fn test_docitem_types_multiple_files() {
        // Multiple files should generate proper DocItem sequence
        use std::path::PathBuf;
        let files = vec![
            ExtractedFile {
                name: "file1.txt".to_string(),
                path: PathBuf::from("file1.txt"),
                contents: vec![],
                size: 100,
            },
            ExtractedFile {
                name: "file2.txt".to_string(),
                path: PathBuf::from("file2.txt"),
                contents: vec![],
                size: 200,
            },
            ExtractedFile {
                name: "file3.txt".to_string(),
                path: PathBuf::from("file3.txt"),
                contents: vec![],
                size: 300,
            },
        ];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "test.zip",
            InputFormat::Zip,
            Path::new("test.zip"),
        );

        assert_eq!(doc_items.len(), 7, "Archive with 3 files should generate 7 DocItems: Title + Summary header + Summary text + Contents header + 3 files"); // Title + Summary header + Summary text + Contents header + 3 files

        // Verify summary contains correct totals
        match &doc_items[2] {
            DocItem::Text { text, .. } => {
                assert!(text.contains("3 files"));
                assert!(text.contains("600 bytes"));
            }
            _ => panic!("Expected Text for summary"),
        }

        // Verify all files are listed
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);
        assert!(markdown.contains("file1.txt"));
        assert!(markdown.contains("file2.txt"));
        assert!(markdown.contains("file3.txt"));
    }

    #[test]
    fn test_docitem_types_empty_archive() {
        // Empty archive should generate: SectionHeader + Text (empty message)
        let files = vec![];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "empty.zip",
            InputFormat::Zip,
            Path::new("empty.zip"),
        );

        assert_eq!(
            doc_items.len(),
            2,
            "Empty archive should generate 2 DocItems: Title + Empty message"
        ); // Title + Empty message

        match &doc_items[0] {
            DocItem::SectionHeader { text, .. } => {
                assert!(text.contains("Archive Contents"));
            }
            _ => panic!("Expected SectionHeader"),
        }

        match &doc_items[1] {
            DocItem::Text { text, .. } => {
                assert_eq!(text, "(Empty archive)");
            }
            _ => panic!("Expected Text for empty message"),
        }
    }

    // ========================================
    // CATEGORY 3: Format-Specific Features (4 tests)
    // Test archive-specific: file sizes, summaries, nested paths, emoji markers
    // ========================================

    #[test]
    fn test_archive_file_size_calculation() {
        // Total size should be sum of all file sizes
        use std::path::PathBuf;
        let files = vec![
            ExtractedFile {
                name: "small.txt".to_string(),
                path: PathBuf::from("small.txt"),
                contents: vec![],
                size: 10,
            },
            ExtractedFile {
                name: "medium.txt".to_string(),
                path: PathBuf::from("medium.txt"),
                contents: vec![],
                size: 100,
            },
            ExtractedFile {
                name: "large.txt".to_string(),
                path: PathBuf::from("large.txt"),
                contents: vec![],
                size: 1000,
            },
        ];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "test.zip",
            InputFormat::Zip,
            Path::new("test.zip"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        // Verify summary shows total size
        assert!(markdown.contains("1110 bytes total"));

        // Verify individual file sizes
        assert!(markdown.contains("small.txt (10 bytes)"));
        assert!(markdown.contains("medium.txt (100 bytes)"));
        assert!(markdown.contains("large.txt (1000 bytes)"));
    }

    #[test]
    fn test_archive_nested_paths() {
        // Archives often contain nested directory structures
        use std::path::PathBuf;
        let files = vec![
            ExtractedFile {
                name: "root.txt".to_string(),
                path: PathBuf::from("root.txt"),
                contents: vec![],
                size: 50,
            },
            ExtractedFile {
                name: "dir1/file1.txt".to_string(),
                path: PathBuf::from("dir1/file1.txt"),
                contents: vec![],
                size: 60,
            },
            ExtractedFile {
                name: "dir1/dir2/file2.txt".to_string(),
                path: PathBuf::from("dir1/dir2/file2.txt"),
                contents: vec![],
                size: 70,
            },
        ];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "test.zip",
            InputFormat::Zip,
            Path::new("test.zip"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        // Verify all paths are preserved in output
        assert!(markdown.contains("root.txt"));
        assert!(markdown.contains("dir1/file1.txt"));
        assert!(markdown.contains("dir1/dir2/file2.txt"));

        // Verify total size calculation
        assert!(markdown.contains("180 bytes total"));
    }

    #[test]
    fn test_archive_file_listings() {
        // Archive listings show plain file names without emoji
        use std::path::PathBuf;
        let files = vec![
            ExtractedFile {
                name: "document.txt".to_string(),
                path: PathBuf::from("document.txt"),
                contents: vec![],
                size: 100,
            },
            ExtractedFile {
                name: "image.png".to_string(),
                path: PathBuf::from("image.png"),
                contents: vec![],
                size: 200,
            },
        ];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "test.zip",
            InputFormat::Zip,
            Path::new("test.zip"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        // Verify files are listed without emoji
        assert!(markdown.contains("document.txt (100 bytes)"));
        assert!(markdown.contains("image.png (200 bytes)"));
    }

    #[test]
    fn test_archive_list_item_format() {
        // List items should have proper markdown format with "- " marker
        use std::path::PathBuf;
        let files = vec![ExtractedFile {
            name: "test.txt".to_string(),
            path: PathBuf::from("test.txt"),
            contents: vec![],
            size: 42,
        }];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "test.zip",
            InputFormat::Zip,
            Path::new("test.zip"),
        );

        // Find the ListItem
        let list_item = doc_items
            .iter()
            .find_map(|item| {
                if let DocItem::ListItem { text, marker, .. } = item {
                    Some((text, marker))
                } else {
                    None
                }
            })
            .expect("Should have ListItem");

        assert_eq!(list_item.1, "- ");
        assert!(list_item.0.contains("test.txt (42 bytes)"));

        // Verify markdown output
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);
        assert!(markdown.contains("- test.txt (42 bytes)\n"));
    }

    // ========================================
    // CATEGORY 4: Edge Cases (3 tests)
    // Test boundary conditions and special cases
    // ========================================

    #[test]
    fn test_archive_zero_byte_files() {
        // Zero-byte files should be handled gracefully
        use std::path::PathBuf;
        let files = vec![
            ExtractedFile {
                name: "empty1.txt".to_string(),
                path: PathBuf::from("empty1.txt"),
                contents: vec![],
                size: 0,
            },
            ExtractedFile {
                name: "empty2.txt".to_string(),
                path: PathBuf::from("empty2.txt"),
                contents: vec![],
                size: 0,
            },
        ];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "test.zip",
            InputFormat::Zip,
            Path::new("test.zip"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        // Verify summary shows 0 bytes total
        assert!(markdown.contains("2 files"));
        assert!(markdown.contains("0 bytes total"));

        // Verify individual files show 0 bytes
        assert!(markdown.contains("empty1.txt (0 bytes)"));
        assert!(markdown.contains("empty2.txt (0 bytes)"));
    }

    #[test]
    fn test_archive_large_file_count() {
        // Many files (100+) should be handled efficiently
        use std::path::PathBuf;
        let mut files = Vec::new();
        for i in 0..150 {
            files.push(ExtractedFile {
                name: format!("file{i}.txt"),
                path: PathBuf::from(format!("file{i}.txt")),
                contents: vec![],
                size: i as usize * 10,
            });
        }

        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "large.zip",
            InputFormat::Zip,
            Path::new("large.zip"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        // Verify summary shows 150 files
        assert!(markdown.contains("150 files"));

        // Verify total size: sum of 0+10+20+...+1490 = 10 * (0+1+2+...+149) = 10 * (149*150/2) = 111750
        assert!(markdown.contains("111750 bytes total"));

        // Verify all files listed (spot check a few)
        assert!(markdown.contains("file0.txt (0 bytes)"));
        assert!(markdown.contains("file50.txt (500 bytes)"));
        assert!(markdown.contains("file149.txt (1490 bytes)"));

        // Verify DocItems count: 1 title + 1 summary header + 1 summary text + 1 contents header + 150 files = 154
        assert_eq!(
            doc_items.len(),
            154,
            "Large archive should generate 154 DocItems: 4 headers/text + 150 file items"
        );
    }

    #[test]
    fn test_archive_special_characters_in_filenames() {
        // Filenames with spaces, unicode, special chars should be preserved
        use std::path::PathBuf;
        let files = vec![
            ExtractedFile {
                name: "file with spaces.txt".to_string(),
                path: PathBuf::from("file with spaces.txt"),
                contents: vec![],
                size: 100,
            },
            ExtractedFile {
                name: "文件.txt".to_string(), // Chinese characters
                path: PathBuf::from("文件.txt"),
                contents: vec![],
                size: 200,
            },
            ExtractedFile {
                name: "file-with-dashes_and_underscores.txt".to_string(),
                path: PathBuf::from("file-with-dashes_and_underscores.txt"),
                contents: vec![],
                size: 300,
            },
        ];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "test.zip",
            InputFormat::Zip,
            Path::new("test.zip"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        // Verify all special filenames are preserved
        assert!(markdown.contains("file with spaces.txt"));
        assert!(markdown.contains("文件.txt"));
        assert!(markdown.contains("file-with-dashes_and_underscores.txt"));

        // Verify summary
        assert!(markdown.contains("3 files"));
        assert!(markdown.contains("600 bytes total"));
    }

    // ========================================
    // CATEGORY 5: Backend Trait Implementation (2 tests)
    // ========================================

    #[test]
    fn test_backend_format_method() {
        // format() should return the format passed to new()
        let zip_backend = ArchiveBackend::new(InputFormat::Zip).unwrap();
        assert_eq!(
            zip_backend.format(),
            InputFormat::Zip,
            "ZIP backend format() should return Zip"
        );

        let tar_backend = ArchiveBackend::new(InputFormat::Tar).unwrap();
        assert_eq!(
            tar_backend.format(),
            InputFormat::Tar,
            "TAR backend format() should return Tar"
        );

        let sevenz_backend = ArchiveBackend::new(InputFormat::SevenZ).unwrap();
        assert_eq!(
            sevenz_backend.format(),
            InputFormat::SevenZ,
            "7Z backend format() should return SevenZ"
        );

        let rar_backend = ArchiveBackend::new(InputFormat::Rar).unwrap();
        assert_eq!(
            rar_backend.format(),
            InputFormat::Rar,
            "RAR backend format() should return Rar"
        );
    }

    #[test]
    fn test_parse_bytes_not_supported() {
        // parse_bytes should always return error for archives
        let backend = ArchiveBackend::new(InputFormat::Zip).unwrap();
        let result = backend.parse_bytes(b"fake zip data", &BackendOptions::default());

        assert!(
            result.is_err(),
            "parse_bytes should return error for archives"
        );
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Archive parsing from bytes not supported"));
        assert!(error_msg.contains("parse_file"));
    }

    // ========================================
    // CATEGORY 6: Markdown Formatting Details (4 tests)
    // ========================================

    #[test]
    fn test_markdown_structure_hierarchy() {
        // Verify markdown has proper hierarchical structure: # title, bold summary, ## contents, - list
        use std::path::PathBuf;
        let files = vec![ExtractedFile {
            name: "test.txt".to_string(),
            path: PathBuf::from("test.txt"),
            contents: vec![],
            size: 100,
        }];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "archive.zip",
            InputFormat::Zip,
            Path::new("archive.zip"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        // Verify structure order
        let title_pos = markdown.find("# Archive Contents:").unwrap();
        let summary_pos = markdown.find("## Archive Summary").unwrap();
        let contents_pos = markdown.find("## Contents").unwrap();
        let list_pos = markdown.find("- test.txt").unwrap();

        // Positions should be in order
        assert!(
            title_pos < summary_pos,
            "Title should appear before summary section"
        );
        assert!(
            summary_pos < contents_pos,
            "Summary section should appear before contents section"
        );
        assert!(
            contents_pos < list_pos,
            "Contents section should appear before file list"
        );
    }

    #[test]
    fn test_markdown_heading_levels() {
        // Archive title = level 1, Contents = level 2
        use std::path::PathBuf;
        let files = vec![ExtractedFile {
            name: "test.txt".to_string(),
            path: PathBuf::from("test.txt"),
            contents: vec![],
            size: 10,
        }];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "test.zip",
            InputFormat::Zip,
            Path::new("test.zip"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        // Level 1 heading (single #)
        assert!(markdown.contains("# Archive Contents: test.zip\n\n"));
        // Level 2 heading (double ##)
        assert!(markdown.contains("## Contents\n\n"));
    }

    #[test]
    fn test_markdown_summary_text() {
        // Summary text should be plain (no bold formatting)
        use std::path::PathBuf;
        let files = vec![ExtractedFile {
            name: "file.txt".to_string(),
            path: PathBuf::from("file.txt"),
            contents: vec![],
            size: 50,
        }];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "test.zip",
            InputFormat::Zip,
            Path::new("test.zip"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        // Summary should be plain text (no bold) with file type breakdown
        // After N=1870: Summary is now a section header followed by summary text
        // After N=1893: single-file archives show type directly (no redundancy)
        assert!(markdown.contains("## Archive Summary"));
        assert!(markdown.contains("1 TXT file, 50 bytes total"));
    }

    #[test]
    fn test_markdown_list_markers() {
        // List items should use "- " marker
        use std::path::PathBuf;
        let files = vec![
            ExtractedFile {
                name: "a.txt".to_string(),
                path: PathBuf::from("a.txt"),
                contents: vec![],
                size: 10,
            },
            ExtractedFile {
                name: "b.txt".to_string(),
                path: PathBuf::from("b.txt"),
                contents: vec![],
                size: 20,
            },
        ];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "test.zip",
            InputFormat::Zip,
            Path::new("test.zip"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        // Each file should have "- " prefix
        assert!(markdown.contains("- a.txt (10 bytes)\n"));
        assert!(markdown.contains("- b.txt (20 bytes)\n"));
    }

    // ========================================
    // CATEGORY 7: DocItem self_ref Format (2 tests)
    // ========================================

    #[test]
    fn test_docitem_self_ref_indices() {
        // self_ref should use correct format and sequential indices for each type
        use std::path::PathBuf;
        let files = vec![
            ExtractedFile {
                name: "file1.txt".to_string(),
                path: PathBuf::from("file1.txt"),
                contents: vec![],
                size: 10,
            },
            ExtractedFile {
                name: "file2.txt".to_string(),
                path: PathBuf::from("file2.txt"),
                contents: vec![],
                size: 20,
            },
        ];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "test.zip",
            InputFormat::Zip,
            Path::new("test.zip"),
        );

        // Expected: Title header (0), Summary header (1), Summary text (2), Contents header (3), File1 list (4), File2 list (5)
        assert_eq!(
            doc_items.len(),
            6,
            "Archive with 2 files should generate 6 DocItems"
        );

        // Verify self_ref format for each item type
        assert!(
            matches!(&doc_items[0], DocItem::SectionHeader { self_ref, .. } if self_ref == "#/headers/0"),
            "Title header self_ref should be #/headers/0"
        );
        assert!(
            matches!(&doc_items[1], DocItem::SectionHeader { self_ref, .. } if self_ref == "#/headers/1"),
            "Summary header self_ref should be #/headers/1"
        );
        assert!(
            matches!(&doc_items[2], DocItem::Text { self_ref, .. } if self_ref == "#/texts/2"),
            "Summary text self_ref should be #/texts/2"
        );
        assert!(
            matches!(&doc_items[3], DocItem::SectionHeader { self_ref, .. } if self_ref == "#/headers/3"),
            "Contents header self_ref should be #/headers/3"
        );
        assert!(
            matches!(&doc_items[4], DocItem::ListItem { self_ref, .. } if self_ref == "#/texts/4"),
            "First file list item self_ref should be #/texts/4"
        );
        assert!(
            matches!(&doc_items[5], DocItem::ListItem { self_ref, .. } if self_ref == "#/texts/5"),
            "Second file list item self_ref should be #/texts/5"
        );
    }

    #[test]
    fn test_docitem_self_ref_format_string() {
        // self_ref should use correct format for each DocItem type
        use std::path::PathBuf;
        let files = vec![ExtractedFile {
            name: "single.txt".to_string(),
            path: PathBuf::from("single.txt"),
            contents: vec![],
            size: 42,
        }];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "test.zip",
            InputFormat::Zip,
            Path::new("test.zip"),
        );

        // Check all self_ref values use correct format for their type
        for item in &doc_items {
            match item {
                DocItem::Text { self_ref, .. } => assert!(self_ref.starts_with("#/texts/")),
                DocItem::SectionHeader { self_ref, .. } => {
                    assert!(self_ref.starts_with("#/headers/"))
                }
                DocItem::ListItem { self_ref, .. } => {
                    assert!(self_ref.starts_with("#/texts/"))
                }
                _ => panic!("Unexpected DocItem type"),
            };
        }
    }

    // ========================================
    // CATEGORY 8: Provenance Information (2 tests)
    // ========================================

    #[test]
    fn test_provenance_page_numbers() {
        // All DocItems should have page_no = 1 for archives (single-page concept)
        use std::path::PathBuf;
        let files = vec![ExtractedFile {
            name: "test.txt".to_string(),
            path: PathBuf::from("test.txt"),
            contents: vec![],
            size: 10,
        }];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "test.zip",
            InputFormat::Zip,
            Path::new("test.zip"),
        );

        // Verify all provenance items have page_no = 1
        for item in &doc_items {
            let prov = match item {
                DocItem::Text { prov, .. } => prov,
                DocItem::SectionHeader { prov, .. } => prov,
                DocItem::ListItem { prov, .. } => prov,
                _ => panic!("Unexpected DocItem type"),
            };
            assert_eq!(
                prov.len(),
                1,
                "Each DocItem should have exactly 1 provenance entry"
            );
            assert_eq!(
                prov[0].page_no, 1,
                "Provenance page_no should be 1 for archives"
            );
        }
    }

    #[test]
    fn test_provenance_bounding_box() {
        // Provenance should have default full-page bbox (0,0 to 1,1)
        use std::path::PathBuf;
        let files = vec![ExtractedFile {
            name: "test.txt".to_string(),
            path: PathBuf::from("test.txt"),
            contents: vec![],
            size: 10,
        }];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "test.zip",
            InputFormat::Zip,
            Path::new("test.zip"),
        );

        // Check first DocItem's bbox
        let prov = match &doc_items[0] {
            DocItem::SectionHeader { prov, .. } => prov,
            _ => panic!("Expected SectionHeader"),
        };

        assert_eq!(prov[0].bbox.l, 0.0, "Bounding box left should be 0.0");
        assert_eq!(prov[0].bbox.t, 0.0, "Bounding box top should be 0.0");
        assert_eq!(prov[0].bbox.r, 1.0, "Bounding box right should be 1.0");
        assert_eq!(prov[0].bbox.b, 1.0, "Bounding box bottom should be 1.0");
        assert_eq!(
            prov[0].bbox.coord_origin,
            CoordOrigin::TopLeft,
            "Bounding box coord_origin should be TopLeft"
        );
    }

    // ========================================
    // CATEGORY 9: Summary Variations (3 tests)
    // ========================================

    #[test]
    fn test_summary_singular_file() {
        // Summary should say "1 files" (note: current code doesn't handle singular/plural)
        use std::path::PathBuf;
        let files = vec![ExtractedFile {
            name: "single.txt".to_string(),
            path: PathBuf::from("single.txt"),
            contents: vec![],
            size: 100,
        }];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "test.zip",
            InputFormat::Zip,
            Path::new("test.zip"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        // Current implementation uses grammatically correct "1 file" with type breakdown
        // After N=1870: Summary is now a section header followed by summary text
        // After N=1893: single-file archives show type directly (no redundancy)
        assert!(markdown.contains("## Archive Summary"));
        assert!(markdown.contains("1 TXT file, 100 bytes total"));
    }

    #[test]
    fn test_summary_very_large_sizes() {
        // Summary should handle very large byte counts (GB range)
        use std::path::PathBuf;
        let files = vec![
            ExtractedFile {
                name: "large1.bin".to_string(),
                path: PathBuf::from("large1.bin"),
                contents: vec![],
                size: 1_000_000_000, // 1 GB
            },
            ExtractedFile {
                name: "large2.bin".to_string(),
                path: PathBuf::from("large2.bin"),
                contents: vec![],
                size: 2_000_000_000, // 2 GB
            },
        ];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "huge.zip",
            InputFormat::Zip,
            Path::new("huge.zip"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        // Verify summary shows correct total: 3 GB = 3,000,000,000 bytes
        assert!(markdown.contains("2 files"));
        assert!(markdown.contains("3000000000 bytes total"));
    }

    #[test]
    fn test_summary_mixed_sizes() {
        // Summary with mix of small and large files
        use std::path::PathBuf;
        let files = vec![
            ExtractedFile {
                name: "tiny.txt".to_string(),
                path: PathBuf::from("tiny.txt"),
                contents: vec![],
                size: 1,
            },
            ExtractedFile {
                name: "small.txt".to_string(),
                path: PathBuf::from("small.txt"),
                contents: vec![],
                size: 100,
            },
            ExtractedFile {
                name: "medium.txt".to_string(),
                path: PathBuf::from("medium.txt"),
                contents: vec![],
                size: 10_000,
            },
            ExtractedFile {
                name: "large.bin".to_string(),
                path: PathBuf::from("large.bin"),
                contents: vec![],
                size: 1_000_000,
            },
        ];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "mixed.zip",
            InputFormat::Zip,
            Path::new("mixed.zip"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        // Total: 1 + 100 + 10,000 + 1,000,000 = 1,010,101
        assert!(markdown.contains("4 files"));
        assert!(markdown.contains("1010101 bytes total"));
    }

    // ========================================
    // CATEGORY 10: Filename Edge Cases (3 tests)
    // ========================================

    #[test]
    fn test_empty_filename() {
        // Empty filename should be handled gracefully
        use std::path::PathBuf;
        let files = vec![ExtractedFile {
            name: "".to_string(),
            path: PathBuf::from(""),
            contents: vec![],
            size: 50,
        }];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "test.zip",
            InputFormat::Zip,
            Path::new("test.zip"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        // Should still generate list item (with empty name)
        assert!(markdown.contains("1 file"));
        assert!(markdown.contains("50 bytes"));
        // Emoji and size should be present even with empty name
        assert!(markdown.contains(" (50 bytes)"));
    }

    #[test]
    fn test_very_long_filename() {
        // Very long filenames (255+ chars) should be preserved
        use std::path::PathBuf;
        let long_name = "a".repeat(300);
        let files = vec![ExtractedFile {
            name: format!("{long_name}.txt"),
            path: PathBuf::from(format!("{long_name}.txt")),
            contents: vec![],
            size: 100,
        }];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "test.zip",
            InputFormat::Zip,
            Path::new("test.zip"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        // Long filename should be present in output
        assert!(
            markdown.contains(&long_name),
            "Very long filename (300 chars) should be preserved in output"
        );
        assert!(markdown.contains(".txt (100 bytes)"));
    }

    #[test]
    fn test_filename_without_extension() {
        // Files without extensions should work
        use std::path::PathBuf;
        let files = vec![
            ExtractedFile {
                name: "README".to_string(),
                path: PathBuf::from("README"),
                contents: vec![],
                size: 1000,
            },
            ExtractedFile {
                name: "LICENSE".to_string(),
                path: PathBuf::from("LICENSE"),
                contents: vec![],
                size: 2000,
            },
        ];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "project.tar",
            InputFormat::Zip,
            Path::new("project.tar"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        // Files without extensions should be listed normally
        assert!(markdown.contains("README (1000 bytes)"));
        assert!(markdown.contains("LICENSE (2000 bytes)"));
        assert!(markdown.contains("2 files"));
        assert!(markdown.contains("3000 bytes total"));
    }

    // ========================================
    // CATEGORY 11: Integration Tests (2 tests)
    // ========================================

    #[test]
    fn test_full_docitems_structure_validation() {
        // Comprehensive validation of DocItems structure
        use std::path::PathBuf;
        let files = vec![
            ExtractedFile {
                name: "file1.txt".to_string(),
                path: PathBuf::from("file1.txt"),
                contents: vec![],
                size: 100,
            },
            ExtractedFile {
                name: "file2.txt".to_string(),
                path: PathBuf::from("file2.txt"),
                contents: vec![],
                size: 200,
            },
        ];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "archive.zip",
            InputFormat::Zip,
            Path::new("archive.zip"),
        );

        // Expected structure: Title SectionHeader + Summary SectionHeader + Summary Text + Contents SectionHeader + 2 ListItems = 6 items
        assert_eq!(
            doc_items.len(),
            6,
            "Archive with 2 files should generate 6 DocItems"
        );

        // Verify types
        assert!(
            matches!(&doc_items[0], DocItem::SectionHeader { .. }),
            "Item 0 should be SectionHeader (title)"
        );
        assert!(
            matches!(&doc_items[1], DocItem::SectionHeader { .. }),
            "Item 1 should be SectionHeader (summary header)"
        );
        assert!(
            matches!(&doc_items[2], DocItem::Text { .. }),
            "Item 2 should be Text (summary)"
        );
        assert!(
            matches!(&doc_items[3], DocItem::SectionHeader { .. }),
            "Item 3 should be SectionHeader (contents header)"
        );
        assert!(
            matches!(&doc_items[4], DocItem::ListItem { .. }),
            "Item 4 should be ListItem (first file)"
        );
        assert!(
            matches!(&doc_items[5], DocItem::ListItem { .. }),
            "Item 5 should be ListItem (second file)"
        );

        // Verify all have provenance
        for item in &doc_items {
            let prov = match item {
                DocItem::Text { prov, .. } => prov,
                DocItem::SectionHeader { prov, .. } => prov,
                DocItem::ListItem { prov, .. } => prov,
                _ => panic!("Unexpected type"),
            };
            assert!(
                !prov.is_empty(),
                "All DocItems should have non-empty provenance"
            );
        }

        // Verify all have content_layer = "body"
        for item in &doc_items {
            let layer = match item {
                DocItem::Text { content_layer, .. } => content_layer,
                DocItem::SectionHeader { content_layer, .. } => content_layer,
                DocItem::ListItem { content_layer, .. } => content_layer,
                _ => panic!("Unexpected type"),
            };
            assert_eq!(layer, "body");
        }
    }

    #[test]
    fn test_parse_bytes_always_fails() {
        // Verify parse_bytes fails for all supported archive formats
        let formats = vec![
            InputFormat::Zip,
            InputFormat::Tar,
            InputFormat::SevenZ,
            InputFormat::Rar,
        ];

        for format in formats {
            let backend = ArchiveBackend::new(format).unwrap();
            let result = backend.parse_bytes(b"dummy data", &BackendOptions::default());
            assert!(result.is_err(), "parse_bytes should fail for {format:?}");
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("Archive parsing from bytes not supported"),
                "Error message should mention bytes not supported"
            );
        }
    }

    // ========================================
    // CATEGORY 12: Additional Edge Cases (Target: 50 tests total)
    // ========================================

    /// Test can_handle method
    #[test]
    fn test_can_handle_archive_formats() {
        let zip_backend = ArchiveBackend::new(InputFormat::Zip).unwrap();
        assert!(
            zip_backend.can_handle(InputFormat::Zip),
            "ZIP backend should handle ZIP format"
        );
        assert!(
            !zip_backend.can_handle(InputFormat::Tar),
            "ZIP backend should not handle TAR format"
        );
        assert!(
            !zip_backend.can_handle(InputFormat::Pdf),
            "ZIP backend should not handle PDF format"
        );

        let tar_backend = ArchiveBackend::new(InputFormat::Tar).unwrap();
        assert!(
            tar_backend.can_handle(InputFormat::Tar),
            "TAR backend should handle TAR format"
        );
        assert!(
            !tar_backend.can_handle(InputFormat::Zip),
            "TAR backend should not handle ZIP format"
        );
    }

    /// Test BackendOptions passthrough (ignored but accepted)
    #[test]
    fn test_backend_options_ignored() {
        use std::path::PathBuf;
        let files = vec![ExtractedFile {
            name: "test.txt".to_string(),
            path: PathBuf::from("test.txt"),
            contents: vec![],
            size: 100,
        }];

        // Archive backend ignores all options
        let _options = BackendOptions::default()
            .with_ocr(true)
            .with_table_structure(true);

        // Should generate same output regardless of options
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "test.zip",
            InputFormat::Zip,
            Path::new("test.zip"),
        );
        assert_eq!(
            doc_items.len(),
            5,
            "Options should not affect DocItem count"
        ); // Title + Summary header + Summary text + Contents header + 1 file
    }

    /// Test content_blocks is always Some for non-empty archives
    #[test]
    fn test_content_blocks_populated() {
        use std::path::PathBuf;
        let files = vec![ExtractedFile {
            name: "test.txt".to_string(),
            path: PathBuf::from("test.txt"),
            contents: vec![],
            size: 100,
        }];

        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "test.zip",
            InputFormat::Zip,
            Path::new("test.zip"),
        );
        // Non-empty archives always generate DocItems
        assert!(
            !doc_items.is_empty(),
            "Non-empty archive should generate DocItems"
        );
        assert_eq!(
            doc_items.len(),
            5,
            "Single file archive should generate 5 DocItems"
        ); // Title + Summary header + Summary text + Contents header + 1 file
    }

    /// Test archive with directory entries (ending with /)
    #[test]
    fn test_archive_directory_entries() {
        use std::path::PathBuf;
        let files = vec![
            ExtractedFile {
                name: "folder/".to_string(),
                path: PathBuf::from("folder/"),
                contents: vec![],
                size: 0,
            },
            ExtractedFile {
                name: "folder/file.txt".to_string(),
                path: PathBuf::from("folder/file.txt"),
                contents: vec![],
                size: 100,
            },
        ];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "test.zip",
            InputFormat::Zip,
            Path::new("test.zip"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        // Directories should be listed
        assert!(markdown.contains("folder/"));
        assert!(markdown.contains("folder/file.txt"));
        assert!(markdown.contains("2 files"));
        assert!(markdown.contains("100 bytes total"));
    }

    /// Test archive name with special characters
    #[test]
    fn test_archive_name_special_chars() {
        use std::path::PathBuf;
        let files = vec![ExtractedFile {
            name: "test.txt".to_string(),
            path: PathBuf::from("test.txt"),
            contents: vec![],
            size: 10,
        }];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "archive_with-special_chars!@#.zip",
            InputFormat::Zip,
            Path::new("archive_with-special_chars!@#.zip"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        // Archive name should be preserved in title
        assert!(markdown.contains("Archive Contents: archive_with-special_chars!@#.zip"));
    }

    /// Test file with maximum size value
    #[test]
    fn test_file_maximum_size() {
        use std::path::PathBuf;
        let files = vec![ExtractedFile {
            name: "huge.bin".to_string(),
            path: PathBuf::from("huge.bin"),
            contents: vec![],
            size: usize::MAX,
        }];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "test.zip",
            InputFormat::Zip,
            Path::new("test.zip"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        // Should handle maximum usize value
        assert!(markdown.contains("huge.bin"));
        assert!(markdown.contains(&format!("{} bytes", usize::MAX)));
    }

    /// Test multiple archives of different formats
    #[test]
    fn test_multiple_format_backends() {
        // Each format should maintain its own identity
        let zip = ArchiveBackend::new(InputFormat::Zip).unwrap();
        let tar = ArchiveBackend::new(InputFormat::Tar).unwrap();
        let sevenz = ArchiveBackend::new(InputFormat::SevenZ).unwrap();
        let rar = ArchiveBackend::new(InputFormat::Rar).unwrap();

        assert_eq!(
            zip.format(),
            InputFormat::Zip,
            "ZIP backend should maintain ZIP identity"
        );
        assert_eq!(
            tar.format(),
            InputFormat::Tar,
            "TAR backend should maintain TAR identity"
        );
        assert_eq!(
            sevenz.format(),
            InputFormat::SevenZ,
            "7Z backend should maintain SevenZ identity"
        );
        assert_eq!(
            rar.format(),
            InputFormat::Rar,
            "RAR backend should maintain RAR identity"
        );
    }

    /// Test file path with backslashes (Windows-style)
    #[test]
    fn test_file_path_backslashes() {
        use std::path::PathBuf;
        let files = vec![ExtractedFile {
            name: "folder\\subfolder\\file.txt".to_string(),
            path: PathBuf::from("folder\\subfolder\\file.txt"),
            contents: vec![],
            size: 50,
        }];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "windows.zip",
            InputFormat::Zip,
            Path::new("windows.zip"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        // Backslashes should be preserved
        assert!(markdown.contains("folder\\subfolder\\file.txt"));
    }

    /// Test file with emoji in name
    #[test]
    fn test_file_emoji_in_name() {
        use std::path::PathBuf;
        let files = vec![ExtractedFile {
            name: "document.txt 🎉".to_string(),
            path: PathBuf::from("document.txt 🎉"),
            contents: vec![],
            size: 100,
        }];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "test.zip",
            InputFormat::Zip,
            Path::new("test.zip"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        // Emoji in filename should be preserved (but no emoji marker prefix)
        assert!(markdown.contains("document.txt 🎉 (100 bytes)"));
    }

    /// Test DocItem parent field is None
    #[test]
    fn test_docitem_parent_field() {
        use std::path::PathBuf;
        let files = vec![ExtractedFile {
            name: "test.txt".to_string(),
            path: PathBuf::from("test.txt"),
            contents: vec![],
            size: 10,
        }];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "test.zip",
            InputFormat::Zip,
            Path::new("test.zip"),
        );

        // All DocItems should have parent = None
        for item in &doc_items {
            let parent = match item {
                DocItem::Text { parent, .. } => parent,
                DocItem::SectionHeader { parent, .. } => parent,
                DocItem::ListItem { parent, .. } => parent,
                _ => panic!("Unexpected type"),
            };
            assert!(
                parent.is_none(),
                "Archive DocItems should have no parent reference"
            );
        }
    }

    /// Test DocItem children field is empty
    #[test]
    fn test_docitem_children_field() {
        use std::path::PathBuf;
        let files = vec![ExtractedFile {
            name: "test.txt".to_string(),
            path: PathBuf::from("test.txt"),
            contents: vec![],
            size: 10,
        }];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "test.zip",
            InputFormat::Zip,
            Path::new("test.zip"),
        );

        // All DocItems should have empty children Vec
        for item in &doc_items {
            let children = match item {
                DocItem::Text { children, .. } => children,
                DocItem::SectionHeader { children, .. } => children,
                DocItem::ListItem { children, .. } => children,
                _ => panic!("Unexpected type"),
            };
            assert!(
                children.is_empty(),
                "Archive DocItems should have empty children vec"
            );
        }
    }

    /// Test single file archive with nested path
    #[test]
    fn test_single_nested_file() {
        use std::path::PathBuf;
        let files = vec![ExtractedFile {
            name: "a/b/c/d/e/file.txt".to_string(),
            path: PathBuf::from("a/b/c/d/e/file.txt"),
            contents: vec![],
            size: 123,
        }];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "nested.tar.gz",
            InputFormat::Zip,
            Path::new("nested.tar.gz"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        // Deep nesting should be preserved
        assert!(markdown.contains("a/b/c/d/e/file.txt"));
        assert!(markdown.contains("123 bytes"));
    }

    /// Test file with dot prefix (hidden files)
    #[test]
    fn test_hidden_files() {
        use std::path::PathBuf;
        let files = vec![
            ExtractedFile {
                name: ".hidden".to_string(),
                path: PathBuf::from(".hidden"),
                contents: vec![],
                size: 10,
            },
            ExtractedFile {
                name: ".config/settings.json".to_string(),
                path: PathBuf::from(".config/settings.json"),
                contents: vec![],
                size: 20,
            },
        ];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "dotfiles.tar",
            InputFormat::Zip,
            Path::new("dotfiles.tar"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        // Hidden files (dot prefix) should be listed
        assert!(markdown.contains(".hidden"));
        assert!(markdown.contains(".config/settings.json"));
        assert!(markdown.contains("2 files"));
        assert!(markdown.contains("30 bytes total"));
    }

    /// Test ListItem marker field
    #[test]
    fn test_list_item_marker_format() {
        use std::path::PathBuf;
        let files = vec![ExtractedFile {
            name: "test.txt".to_string(),
            path: PathBuf::from("test.txt"),
            contents: vec![],
            size: 10,
        }];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "test.zip",
            InputFormat::Zip,
            Path::new("test.zip"),
        );

        // Find the ListItem and check marker
        let list_item = doc_items.iter().find_map(|item| {
            if let DocItem::ListItem { marker, .. } = item {
                Some(marker)
            } else {
                None
            }
        });

        // marker should be "- " (unordered list)
        assert_eq!(list_item, Some(&"- ".to_string()));
    }

    /// Test markdown output without trailing newlines on lists
    #[test]
    fn test_markdown_list_formatting() {
        use std::path::PathBuf;
        let files = vec![
            ExtractedFile {
                name: "a.txt".to_string(),
                path: PathBuf::from("a.txt"),
                contents: vec![],
                size: 10,
            },
            ExtractedFile {
                name: "b.txt".to_string(),
                path: PathBuf::from("b.txt"),
                contents: vec![],
                size: 20,
            },
        ];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "test.zip",
            InputFormat::Zip,
            Path::new("test.zip"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        // List items should end with single newline (not double)
        assert!(markdown.contains("- a.txt (10 bytes)\n"));
        assert!(markdown.contains("- b.txt (20 bytes)\n"));

        // Should NOT have double newlines after list items
        assert!(!markdown.contains("- a.txt (10 bytes)\n\n"));
    }

    /// Test document format field
    #[test]
    fn test_document_format_field() {
        let backend = ArchiveBackend::new(InputFormat::Zip).unwrap();
        assert_eq!(
            backend.format(),
            InputFormat::Zip,
            "Backend format should be Zip"
        );

        let backend = ArchiveBackend::new(InputFormat::Tar).unwrap();
        assert_eq!(
            backend.format(),
            InputFormat::Tar,
            "Backend format should be Tar"
        );
    }

    // ========================================
    // CATEGORY 8: Additional Edge Cases (2 tests)
    // ========================================

    /// Test archive with very deeply nested directory structure
    #[test]
    fn test_deeply_nested_directory_structure() {
        use std::path::PathBuf;
        let files = vec![
            ExtractedFile {
                name: "root.txt".to_string(),
                path: PathBuf::from("root.txt"),
                contents: vec![],
                size: 10,
            },
            ExtractedFile {
                name: "deep/path/a/b/c/d/e/f/g/h/i/j/file.txt".to_string(),
                path: PathBuf::from("deep/path/a/b/c/d/e/f/g/h/i/j/file.txt"),
                contents: vec![],
                size: 100,
            },
            ExtractedFile {
                name: "deep/path/a/b/c/other.txt".to_string(),
                path: PathBuf::from("deep/path/a/b/c/other.txt"),
                contents: vec![],
                size: 50,
            },
        ];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "nested.tar.gz",
            InputFormat::Zip,
            Path::new("nested.tar.gz"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        // Should list all files correctly
        assert!(markdown.contains("root.txt"));
        assert!(markdown.contains("deep/path/a/b/c/d/e/f/g/h/i/j/file.txt"));
        assert!(markdown.contains("deep/path/a/b/c/other.txt"));
        assert!(markdown.contains("3 files"));
        assert!(markdown.contains("160 bytes total"));

        // DocItems should include all files as list items
        let list_items: Vec<_> = doc_items
            .iter()
            .filter(|item| matches!(item, DocItem::ListItem { .. }))
            .collect();
        assert_eq!(list_items.len(), 3, "Should have 3 list items for 3 files");
    }

    /// Test archive with mixed file types and extensions
    #[test]
    fn test_mixed_file_types_and_extensions() {
        use std::path::PathBuf;
        let files = vec![
            ExtractedFile {
                name: "document.pdf".to_string(),
                path: PathBuf::from("document.pdf"),
                contents: vec![],
                size: 1024,
            },
            ExtractedFile {
                name: "image.png".to_string(),
                path: PathBuf::from("image.png"),
                contents: vec![],
                size: 2048,
            },
            ExtractedFile {
                name: "script.py".to_string(),
                path: PathBuf::from("script.py"),
                contents: vec![],
                size: 512,
            },
            ExtractedFile {
                name: "README.md".to_string(),
                path: PathBuf::from("README.md"),
                contents: vec![],
                size: 256,
            },
            ExtractedFile {
                name: "data.json".to_string(),
                path: PathBuf::from("data.json"),
                contents: vec![],
                size: 128,
            },
            ExtractedFile {
                name: "no_extension".to_string(),
                path: PathBuf::from("no_extension"),
                contents: vec![],
                size: 64,
            },
        ];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "mixed.zip",
            InputFormat::Zip,
            Path::new("mixed.zip"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        // Should list all different file types
        assert!(markdown.contains("document.pdf"));
        assert!(markdown.contains("image.png"));
        assert!(markdown.contains("script.py"));
        assert!(markdown.contains("README.md"));
        assert!(markdown.contains("data.json"));
        assert!(markdown.contains("no_extension"));

        // Should show correct file count and total size
        assert!(markdown.contains("6 files"));
        assert!(markdown.contains("4032 bytes total") || markdown.contains("4.03 KB"));

        // Each file should have its size displayed
        assert!(markdown.contains("1024 bytes") || markdown.contains("1.02 KB")); // PDF
        assert!(markdown.contains("2048 bytes") || markdown.contains("2.05 KB")); // PNG
        assert!(markdown.contains("512 bytes")); // Python
        assert!(markdown.contains("256 bytes")); // Markdown
        assert!(markdown.contains("128 bytes")); // JSON
        assert!(markdown.contains("64 bytes")); // No extension

        // All should be list items
        let list_items: Vec<_> = doc_items
            .iter()
            .filter(|item| matches!(item, DocItem::ListItem { .. }))
            .collect();
        assert_eq!(
            list_items.len(),
            6,
            "Should have 6 list items for 6 files with mixed extensions"
        );
    }

    // ========== EXTENDED ARCHIVE TESTS (N=487, +10 tests) ==========

    /// Test ZIP archive with maximum compression
    #[test]
    fn test_zip_maximum_compression() {
        use std::path::PathBuf;
        // Simulating highly compressed text file (e.g., log files, JSON)
        // 10MB original compressed to 500KB (20:1 ratio)
        let files = vec![ExtractedFile {
            name: "application.log".to_string(),
            path: PathBuf::from("application.log"),
            contents: vec![],
            size: 10_485_760, // 10MB uncompressed
        }];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "logs.zip",
            InputFormat::Zip,
            Path::new("logs.zip"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        // After N=1893: single-file archives show type directly (e.g., "1 LOG file")
        assert!(markdown.contains("application.log"));
        assert!(markdown.contains("10485760 bytes") || markdown.contains("10.0 MB"));
        assert!(markdown.contains("1 LOG file") || markdown.contains("1 file"));
    }

    /// Test TAR.GZ multi-volume archive (split archives)
    #[test]
    fn test_tar_gz_split_archive() {
        use std::path::PathBuf;
        // Simulating large dataset split across volumes
        let files = vec![
            ExtractedFile {
                name: "dataset_part1.dat".to_string(),
                path: PathBuf::from("dataset_part1.dat"),
                contents: vec![],
                size: 104_857_600, // 100MB
            },
            ExtractedFile {
                name: "dataset_part2.dat".to_string(),
                path: PathBuf::from("dataset_part2.dat"),
                contents: vec![],
                size: 104_857_600, // 100MB
            },
            ExtractedFile {
                name: "dataset_part3.dat".to_string(),
                path: PathBuf::from("dataset_part3.dat"),
                contents: vec![],
                size: 52_428_800, // 50MB
            },
        ];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "dataset.tar.gz",
            InputFormat::Zip,
            Path::new("dataset.tar.gz"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        assert!(markdown.contains("dataset_part1.dat"));
        assert!(markdown.contains("dataset_part2.dat"));
        assert!(markdown.contains("dataset_part3.dat"));
        assert!(markdown.contains("3 files"));
        assert!(markdown.contains("262144000 bytes") || markdown.contains("MB"));
    }

    /// Test 7Z archive with solid compression (interdependent files)
    #[test]
    fn test_7z_solid_compression() {
        use std::path::PathBuf;
        // Solid compression makes files interdependent for better compression
        let files = vec![
            ExtractedFile {
                name: "chapter1.txt".to_string(),
                path: PathBuf::from("book/chapter1.txt"),
                contents: vec![],
                size: 50000,
            },
            ExtractedFile {
                name: "chapter2.txt".to_string(),
                path: PathBuf::from("book/chapter2.txt"),
                contents: vec![],
                size: 52000,
            },
            ExtractedFile {
                name: "chapter3.txt".to_string(),
                path: PathBuf::from("book/chapter3.txt"),
                contents: vec![],
                size: 48000,
            },
        ];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "book.7z",
            InputFormat::Zip,
            Path::new("book.7z"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        assert!(markdown.contains("chapter1.txt"));
        assert!(markdown.contains("chapter2.txt"));
        assert!(markdown.contains("chapter3.txt"));
        assert!(markdown.contains("3 files"));
        assert!(markdown.contains("150000 bytes") || markdown.contains("KB"));
    }

    /// Test RAR multi-volume archive (part001.rar, part002.rar, etc.)
    #[test]
    fn test_rar_multivolume_parts() {
        use std::path::PathBuf;
        // RAR split across multiple parts
        let files = vec![ExtractedFile {
            name: "large_video.mkv".to_string(),
            path: PathBuf::from("large_video.mkv"),
            contents: vec![],
            size: 2_147_483_648, // 2GB video file
        }];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "video.part001.rar",
            InputFormat::Zip,
            Path::new("video.part001.rar"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        assert!(markdown.contains("large_video.mkv"));
        assert!(markdown.contains("2147483648 bytes") || markdown.contains("2.0 GB"));
        // After N=1893: single-file archives show type directly (e.g., "1 MKV file")
        assert!(markdown.contains("1 MKV file") || markdown.contains("1 file"));
    }

    /// Test archive with hidden files and dotfiles
    #[test]
    fn test_archive_with_hidden_files() {
        use std::path::PathBuf;
        let files = vec![
            ExtractedFile {
                name: ".gitignore".to_string(),
                path: PathBuf::from(".gitignore"),
                contents: vec![],
                size: 50,
            },
            ExtractedFile {
                name: ".env".to_string(),
                path: PathBuf::from(".env"),
                contents: vec![],
                size: 100,
            },
            ExtractedFile {
                name: ".config/settings.json".to_string(),
                path: PathBuf::from(".config/settings.json"),
                contents: vec![],
                size: 200,
            },
            ExtractedFile {
                name: "README.md".to_string(),
                path: PathBuf::from("README.md"),
                contents: vec![],
                size: 500,
            },
        ];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "project.zip",
            InputFormat::Zip,
            Path::new("project.zip"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        assert!(markdown.contains(".gitignore"));
        assert!(markdown.contains(".env"));
        assert!(markdown.contains(".config/settings.json"));
        assert!(markdown.contains("README.md"));
        assert!(markdown.contains("4 files"));
        assert!(markdown.contains("850 bytes"));
    }

    /// Test archive with symbolic links (stored as regular files)
    #[test]
    fn test_archive_with_symlinks() {
        use std::path::PathBuf;
        // Archives often store symlinks as regular text files or with metadata
        let files = vec![
            ExtractedFile {
                name: "data.txt".to_string(),
                path: PathBuf::from("data.txt"),
                contents: vec![],
                size: 1024,
            },
            ExtractedFile {
                name: "link_to_data.txt".to_string(), // Symlink stored
                path: PathBuf::from("link_to_data.txt"),
                contents: vec![],
                size: 8, // Symlinks are small (just the path)
            },
        ];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "with_symlinks.tar.gz",
            InputFormat::Zip,
            Path::new("with_symlinks.tar.gz"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        assert!(markdown.contains("data.txt"));
        assert!(markdown.contains("link_to_data.txt"));
        assert!(markdown.contains("2 files"));
        assert!(markdown.contains("1032 bytes"));
    }

    /// Test archive with international (Unicode) filenames
    #[test]
    fn test_archive_unicode_filenames() {
        use std::path::PathBuf;
        let files = vec![
            ExtractedFile {
                name: "文档.txt".to_string(), // Chinese
                path: PathBuf::from("文档.txt"),
                contents: vec![],
                size: 100,
            },
            ExtractedFile {
                name: "файл.pdf".to_string(), // Russian
                path: PathBuf::from("файл.pdf"),
                contents: vec![],
                size: 200,
            },
            ExtractedFile {
                name: "ファイル.jpg".to_string(), // Japanese
                path: PathBuf::from("ファイル.jpg"),
                contents: vec![],
                size: 300,
            },
            ExtractedFile {
                name: "مستند.docx".to_string(), // Arabic
                path: PathBuf::from("مستند.docx"),
                contents: vec![],
                size: 400,
            },
        ];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "international.zip",
            InputFormat::Zip,
            Path::new("international.zip"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        assert!(markdown.contains("文档.txt"));
        assert!(markdown.contains("файл.pdf"));
        assert!(markdown.contains("ファイル.jpg"));
        assert!(markdown.contains("مستند.docx"));
        assert!(markdown.contains("4 files"));
        assert!(markdown.contains("1000 bytes"));
    }

    /// Test archive with special characters in filenames
    #[test]
    fn test_archive_special_char_filenames() {
        use std::path::PathBuf;
        let files = vec![
            ExtractedFile {
                name: "file with spaces.txt".to_string(),
                path: PathBuf::from("file with spaces.txt"),
                contents: vec![],
                size: 100,
            },
            ExtractedFile {
                name: "file-with-dashes.txt".to_string(),
                path: PathBuf::from("file-with-dashes.txt"),
                contents: vec![],
                size: 200,
            },
            ExtractedFile {
                name: "file_with_underscores.txt".to_string(),
                path: PathBuf::from("file_with_underscores.txt"),
                contents: vec![],
                size: 300,
            },
            ExtractedFile {
                name: "file(with)parens.txt".to_string(),
                path: PathBuf::from("file(with)parens.txt"),
                contents: vec![],
                size: 400,
            },
        ];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "special_chars.tar",
            InputFormat::Zip,
            Path::new("special_chars.tar"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        assert!(markdown.contains("file with spaces.txt"));
        assert!(markdown.contains("file-with-dashes.txt"));
        assert!(markdown.contains("file_with_underscores.txt"));
        assert!(markdown.contains("file(with)parens.txt"));
        assert!(markdown.contains("4 files"));
        assert!(markdown.contains("1000 bytes"));
    }

    /// Test archive with very large file count (1500+ files)
    #[test]
    fn test_archive_very_large_file_count() {
        use std::path::PathBuf;
        // Simulating source code repository or photo collection
        let mut files = Vec::new();
        for i in 0..1500 {
            files.push(ExtractedFile {
                name: format!("file{i:04}.txt"),
                path: PathBuf::from(format!("file{i:04}.txt")),
                contents: vec![],
                size: 1024, // 1KB each
            });
        }

        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "large_collection.zip",
            InputFormat::Zip,
            Path::new("large_collection.zip"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        assert!(markdown.contains("file0000.txt"));
        assert!(markdown.contains("file1499.txt"));
        assert!(markdown.contains("1500 files"));
        assert!(markdown.contains("1536000 bytes") || markdown.contains("1.5 MB"));

        // Verify all files are list items
        let list_items: Vec<_> = doc_items
            .iter()
            .filter(|item| matches!(item, DocItem::ListItem { .. }))
            .collect();
        assert_eq!(
            list_items.len(),
            1500,
            "Should have 1500 list items for 1500 files"
        );
    }

    /// Test archive with multiple zero-byte files
    #[test]
    fn test_archive_multiple_zero_byte_files() {
        use std::path::PathBuf;
        let files = vec![
            ExtractedFile {
                name: "empty1.txt".to_string(),
                path: PathBuf::from("empty1.txt"),
                contents: vec![],
                size: 0,
            },
            ExtractedFile {
                name: "empty2.log".to_string(),
                path: PathBuf::from("empty2.log"),
                contents: vec![],
                size: 0,
            },
            ExtractedFile {
                name: "empty3.dat".to_string(),
                path: PathBuf::from("empty3.dat"),
                contents: vec![],
                size: 0,
            },
            ExtractedFile {
                name: "non_empty.txt".to_string(),
                path: PathBuf::from("non_empty.txt"),
                contents: vec![],
                size: 100,
            },
        ];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "with_empty_files.tar.gz",
            InputFormat::Zip,
            Path::new("with_empty_files.tar.gz"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        assert!(markdown.contains("empty1.txt"));
        assert!(markdown.contains("empty2.log"));
        assert!(markdown.contains("empty3.dat"));
        assert!(markdown.contains("non_empty.txt"));
        assert!(markdown.contains("4 files"));
        assert!(markdown.contains("100 bytes"));

        // Verify 0 byte files show correctly
        assert!(markdown.contains("0 bytes"));
    }

    /// Test archive with very deeply nested directory structure
    #[test]
    fn test_archive_very_deep_nesting() {
        use std::path::PathBuf;
        let files = vec![
            ExtractedFile {
                name: "a/b/c/d/e/f/g/h/i/j/deep.txt".to_string(),
                path: PathBuf::from("a/b/c/d/e/f/g/h/i/j/deep.txt"),
                contents: vec![],
                size: 50,
            },
            ExtractedFile {
                name: "root.txt".to_string(),
                path: PathBuf::from("root.txt"),
                contents: vec![],
                size: 25,
            },
        ];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "deep_nested.tar.gz",
            InputFormat::Zip,
            Path::new("deep_nested.tar.gz"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        assert!(markdown.contains("a/b/c/d/e/f/g/h/i/j/deep.txt"));
        assert!(markdown.contains("root.txt"));
        assert!(markdown.contains("2 files"));
    }

    /// Test archive with mixed directory and file structure
    #[test]
    fn test_archive_mixed_structure() {
        use std::path::PathBuf;
        let files = vec![
            ExtractedFile {
                name: "docs/manual.pdf".to_string(),
                path: PathBuf::from("docs/manual.pdf"),
                contents: vec![],
                size: 1000,
            },
            ExtractedFile {
                name: "src/main.rs".to_string(),
                path: PathBuf::from("src/main.rs"),
                contents: vec![],
                size: 500,
            },
            ExtractedFile {
                name: "src/lib.rs".to_string(),
                path: PathBuf::from("src/lib.rs"),
                contents: vec![],
                size: 300,
            },
            ExtractedFile {
                name: "README.md".to_string(),
                path: PathBuf::from("README.md"),
                contents: vec![],
                size: 200,
            },
        ];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "project.tar",
            InputFormat::Zip,
            Path::new("project.tar"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        assert!(markdown.contains("docs/manual.pdf"));
        assert!(markdown.contains("src/main.rs"));
        assert!(markdown.contains("src/lib.rs"));
        assert!(markdown.contains("README.md"));
        assert!(markdown.contains("4 files"));
        assert!(markdown.contains("2000 bytes") || markdown.contains("2.0 KiB"));
    }

    /// Test archive with only directories (no files)
    #[test]
    fn test_archive_empty_directories() {
        // Some archives might contain directory entries with no actual files
        let files: Vec<ExtractedFile> = vec![];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "empty_dirs.tar.gz",
            InputFormat::Zip,
            Path::new("empty_dirs.tar.gz"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        // Should indicate empty archive
        assert!(markdown.contains("(Empty archive)"));
    }

    // ========================================
    // CATEGORY 13: Advanced Archive Features (N=589, +5 tests)
    // ========================================

    /// Test archive with duplicate filenames in different directories
    #[test]
    fn test_archive_duplicate_filenames_different_paths() {
        use std::path::PathBuf;
        let files = vec![
            ExtractedFile {
                name: "config.json".to_string(),
                path: PathBuf::from("app1/config.json"),
                contents: vec![],
                size: 100,
            },
            ExtractedFile {
                name: "config.json".to_string(),
                path: PathBuf::from("app2/config.json"),
                contents: vec![],
                size: 150,
            },
            ExtractedFile {
                name: "config.json".to_string(),
                path: PathBuf::from("app3/config.json"),
                contents: vec![],
                size: 200,
            },
        ];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "multi_app.zip",
            InputFormat::Zip,
            Path::new("multi_app.zip"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        // All three should be listed with full paths preserved
        // Note: Implementation uses 'name' field which may be basename only
        // This tests that parser handles duplicate names correctly
        assert!(markdown.contains("config.json"));
        assert!(markdown.contains("3 files"));
        assert!(markdown.contains("450 bytes"));
    }

    /// Test archive with version numbering in filenames
    #[test]
    fn test_archive_with_version_numbering() {
        use std::path::PathBuf;
        let files = vec![
            ExtractedFile {
                name: "document_v1.0.0.pdf".to_string(),
                path: PathBuf::from("document_v1.0.0.pdf"),
                contents: vec![],
                size: 1024,
            },
            ExtractedFile {
                name: "document_v1.1.0.pdf".to_string(),
                path: PathBuf::from("document_v1.1.0.pdf"),
                contents: vec![],
                size: 1100,
            },
            ExtractedFile {
                name: "document_v2.0.0-beta.pdf".to_string(),
                path: PathBuf::from("document_v2.0.0-beta.pdf"),
                contents: vec![],
                size: 1200,
            },
        ];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "versions.zip",
            InputFormat::Zip,
            Path::new("versions.zip"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        // Version strings should be preserved
        assert!(markdown.contains("document_v1.0.0.pdf"));
        assert!(markdown.contains("document_v1.1.0.pdf"));
        assert!(markdown.contains("document_v2.0.0-beta.pdf"));
        assert!(markdown.contains("3 files"));
        assert!(markdown.contains("3324 bytes"));
    }

    /// Test archive with nested archives (archive within archive)
    #[test]
    fn test_archive_with_nested_archives() {
        use std::path::PathBuf;
        let files = vec![
            ExtractedFile {
                name: "inner.zip".to_string(),
                path: PathBuf::from("inner.zip"),
                contents: vec![],
                size: 50_000,
            },
            ExtractedFile {
                name: "data.tar.gz".to_string(),
                path: PathBuf::from("data.tar.gz"),
                contents: vec![],
                size: 75_000,
            },
            ExtractedFile {
                name: "backup.7z".to_string(),
                path: PathBuf::from("backup.7z"),
                contents: vec![],
                size: 100_000,
            },
            ExtractedFile {
                name: "README.txt".to_string(),
                path: PathBuf::from("README.txt"),
                contents: vec![],
                size: 500,
            },
        ];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "outer.zip",
            InputFormat::Zip,
            Path::new("outer.zip"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        // Nested archives should be listed as regular files
        assert!(markdown.contains("inner.zip"));
        assert!(markdown.contains("data.tar.gz"));
        assert!(markdown.contains("backup.7z"));
        assert!(markdown.contains("README.txt"));
        assert!(markdown.contains("4 files"));
        assert!(markdown.contains("225500 bytes"));
    }

    /// Test archive with filename containing dots (multiple extensions)
    #[test]
    fn test_archive_with_multiple_dots_in_names() {
        use std::path::PathBuf;
        let files = vec![
            ExtractedFile {
                name: "file.backup.2024.tar.gz".to_string(),
                path: PathBuf::from("file.backup.2024.tar.gz"),
                contents: vec![],
                size: 1000,
            },
            ExtractedFile {
                name: "config.test.local.json".to_string(),
                path: PathBuf::from("config.test.local.json"),
                contents: vec![],
                size: 200,
            },
            ExtractedFile {
                name: "data.v1.2.3.final.txt".to_string(),
                path: PathBuf::from("data.v1.2.3.final.txt"),
                contents: vec![],
                size: 300,
            },
        ];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "multi_dot.zip",
            InputFormat::Zip,
            Path::new("multi_dot.zip"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        // Files with multiple dots should be preserved exactly
        assert!(markdown.contains("file.backup.2024.tar.gz"));
        assert!(markdown.contains("config.test.local.json"));
        assert!(markdown.contains("data.v1.2.3.final.txt"));
        assert!(markdown.contains("3 files"));
        assert!(markdown.contains("1500 bytes"));
    }

    /// Test archive with unusual but valid characters in filenames
    #[test]
    fn test_archive_with_unusual_valid_characters() {
        use std::path::PathBuf;
        let files = vec![
            ExtractedFile {
                name: "file[1].txt".to_string(),
                path: PathBuf::from("file[1].txt"),
                contents: vec![],
                size: 100,
            },
            ExtractedFile {
                name: "file{2}.txt".to_string(),
                path: PathBuf::from("file{2}.txt"),
                contents: vec![],
                size: 150,
            },
            ExtractedFile {
                name: "file@#$%.txt".to_string(),
                path: PathBuf::from("file@#$%.txt"),
                contents: vec![],
                size: 200,
            },
            ExtractedFile {
                name: "file+plus.txt".to_string(),
                path: PathBuf::from("file+plus.txt"),
                contents: vec![],
                size: 50,
            },
            ExtractedFile {
                name: "file=equals.txt".to_string(),
                path: PathBuf::from("file=equals.txt"),
                contents: vec![],
                size: 75,
            },
        ];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "special.tar.gz",
            InputFormat::Zip,
            Path::new("special.tar.gz"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        // Unusual but valid characters should be preserved
        assert!(markdown.contains("file[1].txt"));
        assert!(markdown.contains("file{2}.txt"));
        assert!(markdown.contains("file@#$%.txt"));
        assert!(markdown.contains("file+plus.txt"));
        assert!(markdown.contains("file=equals.txt"));
        assert!(markdown.contains("5 files"));
        assert!(markdown.contains("575 bytes"));
    }

    // ========================================
    // CATEGORY 14: Advanced Compression and Format Features (N=619, +5 tests)
    // ========================================

    /// Test multi-volume RAR archive (split across multiple files)
    #[test]
    fn test_archive_multivolume_rar() {
        use std::path::PathBuf;
        // Multi-volume RAR files are split: archive.part1.rar, archive.part2.rar, etc.
        // When extracted, all parts combine to form complete file list
        let files = vec![
            ExtractedFile {
                name: "large_video.mp4".to_string(),
                path: PathBuf::from("large_video.mp4"),
                contents: vec![],
                size: 500_000_000, // 500 MB split across volumes
            },
            ExtractedFile {
                name: "README.txt".to_string(),
                path: PathBuf::from("README.txt"),
                contents: vec![],
                size: 1024,
            },
        ];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "backup.part1.rar",
            InputFormat::Zip,
            Path::new("backup.part1.rar"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        // Should list all extracted files from multi-volume set (title shows archive name)
        assert!(markdown.contains("backup.part1.rar"));
        assert!(markdown.contains("large_video.mp4"));
        assert!(markdown.contains("README.txt"));
        assert!(markdown.contains("2 files"));
        assert!(markdown.contains("500001024 bytes")); // 500 MB + 1 KB
    }

    /// Test TAR with modern compression algorithms (xz, zstd)
    #[test]
    fn test_archive_tar_modern_compression() {
        use std::path::PathBuf;
        // .tar.xz uses LZMA2 compression (better than gzip/bzip2)
        // .tar.zst uses Zstandard (Facebook's compression algorithm)
        let files = vec![
            ExtractedFile {
                name: "data.bin".to_string(),
                path: PathBuf::from("data.bin"),
                contents: vec![],
                size: 10_485_760, // 10 MB
            },
            ExtractedFile {
                name: "index.json".to_string(),
                path: PathBuf::from("index.json"),
                contents: vec![],
                size: 2048,
            },
        ];

        // Test .tar.xz
        let doc_items_xz = ArchiveBackend::generate_docitems(
            &files,
            "backup.tar.xz",
            InputFormat::Zip,
            Path::new("backup.tar.xz"),
        );
        let markdown_xz = ArchiveBackend::docitems_to_markdown(&doc_items_xz);
        assert!(markdown_xz.contains("backup.tar.xz"));
        assert!(markdown_xz.contains("10487808 bytes")); // 10 MB + 2 KB

        // Test .tar.zst
        let doc_items_zst = ArchiveBackend::generate_docitems(
            &files,
            "backup.tar.zst",
            InputFormat::Zip,
            Path::new("backup.tar.zst"),
        );
        let markdown_zst = ArchiveBackend::docitems_to_markdown(&doc_items_zst);
        assert!(markdown_zst.contains("backup.tar.zst"));
        assert!(markdown_zst.contains("10487808 bytes"));
    }

    /// Test archive with symbolic links
    #[test]
    fn test_archive_with_symbolic_links() {
        use std::path::PathBuf;
        // TAR and ZIP can store symbolic links
        // Symlinks have zero size but point to other files
        let files = vec![
            ExtractedFile {
                name: "actual_file.txt".to_string(),
                path: PathBuf::from("data/actual_file.txt"),
                contents: vec![],
                size: 1024,
            },
            ExtractedFile {
                name: "link_to_file.txt".to_string(),
                path: PathBuf::from("shortcuts/link_to_file.txt"),
                contents: vec![], // Symlinks typically have 0 size in listing
                size: 0,
            },
            ExtractedFile {
                name: "dir_link".to_string(),
                path: PathBuf::from("shortcuts/dir_link"),
                contents: vec![],
                size: 0, // Directory symlink
            },
        ];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "with_symlinks.tar.gz",
            InputFormat::Zip,
            Path::new("with_symlinks.tar.gz"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        // All entries should be listed, even zero-size symlinks
        assert!(markdown.contains("actual_file.txt"));
        assert!(markdown.contains("link_to_file.txt"));
        assert!(markdown.contains("dir_link"));
        assert!(markdown.contains("3 files"));
        assert!(markdown.contains("1024 bytes")); // Only actual file counts
    }

    /// Test archive with Unix file permissions and executable bits
    #[test]
    fn test_archive_with_file_permissions() {
        use std::path::PathBuf;
        // TAR preserves Unix permissions (rwxr-xr-x, etc.)
        // ZIP has limited permission support
        let files = vec![
            ExtractedFile {
                name: "script.sh".to_string(), // Executable
                path: PathBuf::from("bin/script.sh"),
                contents: vec![],
                size: 512,
            },
            ExtractedFile {
                name: "data.txt".to_string(), // Read-only
                path: PathBuf::from("data/data.txt"),
                contents: vec![],
                size: 1024,
            },
            ExtractedFile {
                name: "secret.key".to_string(), // Private (600)
                path: PathBuf::from("keys/secret.key"),
                contents: vec![],
                size: 256,
            },
        ];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "permissions.tar",
            InputFormat::Zip,
            Path::new("permissions.tar"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        // Files should be listed (permissions may not be shown in basic listing)
        assert!(markdown.contains("script.sh"));
        assert!(markdown.contains("data.txt"));
        assert!(markdown.contains("secret.key"));
        assert!(markdown.contains("3 files"));
        assert!(markdown.contains("1792 bytes")); // 512 + 1024 + 256
    }

    /// Test archive with varied timestamp formats and epochs
    #[test]
    fn test_archive_with_timestamp_variations() {
        use std::path::PathBuf;
        // Archives store modification times in different formats
        // ZIP: DOS time (2-second precision, limited range 1980-2107)
        // TAR: Unix timestamp (1-second precision, range 1970-2038 or beyond)
        // 7Z: Windows FILETIME (100-nanosecond precision)
        let files = vec![
            ExtractedFile {
                name: "ancient.txt".to_string(), // Before Unix epoch
                path: PathBuf::from("ancient.txt"),
                contents: vec![],
                size: 100,
            },
            ExtractedFile {
                name: "y2k.txt".to_string(), // Year 2000
                path: PathBuf::from("y2k.txt"),
                contents: vec![],
                size: 200,
            },
            ExtractedFile {
                name: "future.txt".to_string(), // Far future
                path: PathBuf::from("future.txt"),
                contents: vec![],
                size: 300,
            },
            ExtractedFile {
                name: "epoch.txt".to_string(), // Unix epoch (1970-01-01)
                path: PathBuf::from("epoch.txt"),
                contents: vec![],
                size: 400,
            },
        ];
        let doc_items = ArchiveBackend::generate_docitems(
            &files,
            "timestamps.7z",
            InputFormat::Zip,
            Path::new("timestamps.7z"),
        );
        let markdown = ArchiveBackend::docitems_to_markdown(&doc_items);

        // All files should be listed regardless of timestamp edge cases
        assert!(markdown.contains("ancient.txt"));
        assert!(markdown.contains("y2k.txt"));
        assert!(markdown.contains("future.txt"));
        assert!(markdown.contains("epoch.txt"));
        assert!(markdown.contains("4 files"));
        assert!(markdown.contains("1000 bytes")); // 100 + 200 + 300 + 400
    }
}
