use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::encoding::is_valid_text_encoding;

const INDEXABLE_EXTENSIONS: &[&str] = &[
    "rs", "py", "js", "ts", "tsx", "jsx", "go", "c", "cpp", "h", "hpp", "java", "kt", "swift",
    "rb", "php", "cs", "fs", "ex", "exs", "clj", "cljs", "scala", "hs", "ml", "mli", "lua", "pl",
    "pm", "sh", "bash", "zsh", "fish", "ps1", "bat", "cmd", "sql", "json", "yaml", "yml", "toml",
    "xml", "html", "css", "scss", "sass", "md", "rst", "txt", "cmake",
];

/// Document file extensions that require special text extraction (not plain text)
/// These are supported when the `document-processing` feature is enabled.
/// - PDF: docling-core
/// - DOCX: docx-lite
/// - PPTX: pptx-to-md
/// - XLSX/XLS/XLSM/XLSB/ODS: calamine
/// - EPUB: epub
const DOCUMENT_EXTENSIONS: &[&str] = &[
    "pdf", "docx", "pptx", "xlsx", "xls", "xlsm", "xlsb", "ods", "epub",
];

/// Audio file extensions that can be transcribed with Whisper.
/// Supported when the `audio-transcription` feature is enabled.
const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "wav", "flac", "m4a", "ogg", "opus", "wma", "aac", "aiff", "webm",
];

/// Video file extensions that can have audio extracted and transcribed.
/// Supported when the `audio-transcription` feature is enabled.
const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "mkv", "avi", "mov", "wmv", "flv", "webm", "m4v", "mpeg", "mpg",
];

/// Image file extensions that can be embedded with CLIP for visual search.
/// Supported when the `clip` feature is enabled.
const IMAGE_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "bmp", "webp", "tiff", "tif", "ico", "heic", "heif", "avif",
];

const INDEXABLE_FILENAMES: &[&str] = &[
    "dockerfile",
    "makefile",
    "license",
    "copying",
    "readme",
    "changelog",
];

/// Common dotfiles that are useful to index (config files, ignore patterns, etc.)
/// Excludes .env files which may contain secrets.
const INDEXABLE_DOTFILES: &[&str] = &[
    ".gitignore",
    ".gitattributes",
    ".gitmodules",
    ".dockerignore",
    ".editorconfig",
    ".eslintrc",
    ".eslintrc.json",
    ".eslintrc.js",
    ".eslintrc.yml",
    ".prettierrc",
    ".prettierrc.json",
    ".prettierrc.js",
    ".prettierrc.yml",
    ".prettierignore",
    ".npmrc",
    ".yarnrc",
    ".nvmrc",
    ".node-version",
    ".python-version",
    ".ruby-version",
    ".tool-versions",
    ".clang-format",
    ".clang-tidy",
    ".rustfmt.toml",
    ".clippy.toml",
    ".cargo/config.toml",
    ".cargo/config",
    ".mailmap",
    ".codeowners",
    ".env.example",
    ".env.sample",
    ".env.template",
];

pub fn is_indexable_path(path: &Path) -> bool {
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if is_indexable_extension(ext) {
            return true;
        }
        // Include document files when document-processing feature is enabled
        #[cfg(feature = "document-processing")]
        if is_document_extension(ext) {
            return true;
        }
        // Include audio/video files when audio-transcription feature is enabled
        #[cfg(feature = "audio-transcription")]
        if is_audio_extension(ext) || is_video_extension(ext) {
            return true;
        }
        // Include image files when clip feature is enabled
        #[cfg(feature = "clip")]
        if is_image_extension(ext) {
            return true;
        }
    }

    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        if is_indexable_filename(name) {
            return true;
        }
        if is_indexable_dotfile(path) {
            return true;
        }
    }

    false
}

fn is_indexable_extension(ext: &str) -> bool {
    let ext = ext.to_ascii_lowercase();
    INDEXABLE_EXTENSIONS.iter().any(|value| *value == ext)
}

/// Check if extension is a document type requiring special extraction
fn is_document_extension(ext: &str) -> bool {
    let ext = ext.to_ascii_lowercase();
    DOCUMENT_EXTENSIONS.iter().any(|value| *value == ext)
}

/// Check if extension is an audio file type
fn is_audio_extension(ext: &str) -> bool {
    let ext = ext.to_ascii_lowercase();
    AUDIO_EXTENSIONS.iter().any(|value| *value == ext)
}

/// Check if extension is a video file type
fn is_video_extension(ext: &str) -> bool {
    let ext = ext.to_ascii_lowercase();
    VIDEO_EXTENSIONS.iter().any(|value| *value == ext)
}

/// Check if extension is an image file type
fn is_image_extension(ext: &str) -> bool {
    let ext = ext.to_ascii_lowercase();
    IMAGE_EXTENSIONS.iter().any(|value| *value == ext)
}

/// Check if a file requires document text extraction (not plain text read)
///
/// Returns true for PDFs and other document formats that need special processing.
/// These files require the `document-processing` feature to be enabled for indexing.
pub fn is_document_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(is_document_extension)
        .unwrap_or(false)
}

/// Check if a file is a plain text file that can be read directly
pub fn is_text_file(path: &Path) -> bool {
    is_indexable_path(path)
        && !is_document_file(path)
        && !is_audio_file(path)
        && !is_video_file(path)
        && !is_image_file(path)
}

/// Check if a file is an audio file that can be transcribed
///
/// Returns true for audio formats supported by Whisper transcription.
/// These files require the `audio-transcription` feature to be enabled for indexing.
pub fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(is_audio_extension)
        .unwrap_or(false)
}

/// Check if a file is a video file that can have its audio transcribed
///
/// Returns true for video formats from which audio can be extracted.
/// These files require the `audio-transcription` feature to be enabled for indexing.
pub fn is_video_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(is_video_extension)
        .unwrap_or(false)
}

/// Check if a file is a media file (audio or video)
pub fn is_media_file(path: &Path) -> bool {
    is_audio_file(path) || is_video_file(path)
}

/// Check if a file is an image that can be embedded with CLIP
///
/// Returns true for image formats supported by CLIP visual embedding.
/// These files require the `clip` feature to be enabled for indexing.
pub fn is_image_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(is_image_extension)
        .unwrap_or(false)
}

fn is_indexable_filename(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    if INDEXABLE_FILENAMES.iter().any(|value| *value == name) {
        return true;
    }

    INDEXABLE_FILENAMES.iter().any(|value| {
        if name.len() <= value.len() {
            return false;
        }
        if !name.starts_with(value) {
            return false;
        }

        matches!(name.as_bytes()[value.len()], b'.' | b'-' | b'_')
    })
}

/// Check if a path matches a known dotfile pattern
fn is_indexable_dotfile(path: &Path) -> bool {
    // Check for exact dotfile matches
    for pattern in INDEXABLE_DOTFILES {
        // Handle patterns with subdirectories (e.g., ".cargo/config.toml")
        if pattern.contains('/') {
            if path.ends_with(Path::new(pattern)) {
                return true;
            }
        } else if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.eq_ignore_ascii_case(pattern) {
                return true;
            }
        }
    }

    false
}

/// Detected file type from magic bytes
#[derive(Debug, Clone, PartialEq)]
pub enum DetectedFileType {
    /// Plain text file (UTF-8 or ASCII)
    Text,
    /// PDF document
    Pdf,
    /// Microsoft Word document (DOCX)
    Docx,
    /// Microsoft Excel spreadsheet (XLSX)
    Xlsx,
    /// Microsoft PowerPoint presentation (PPTX)
    Pptx,
    /// EPUB ebook
    Epub,
    /// Archive format (ZIP, tar, gzip, etc.)
    Archive,
    /// Image file (PNG, JPEG, GIF, etc.)
    Image,
    /// Audio file (MP3, WAV, FLAC, etc.)
    Audio,
    /// Video file (MP4, MKV, AVI, etc.)
    Video,
    /// Executable binary
    Executable,
    /// Unknown binary format
    Binary,
    /// Could not determine type
    Unknown,
}

/// Detect file type by reading magic bytes from the file
///
/// This reads up to 8KB from the file to detect its type using magic bytes.
/// Returns `Unknown` if the file cannot be read or type cannot be determined.
pub fn detect_file_type(path: &Path) -> DetectedFileType {
    let Ok(mut file) = File::open(path) else {
        return DetectedFileType::Unknown;
    };

    // Read first 8KB for magic byte detection
    let mut buffer = [0u8; 8192];
    let Ok(bytes_read) = file.read(&mut buffer) else {
        return DetectedFileType::Unknown;
    };

    if bytes_read == 0 {
        return DetectedFileType::Text; // Empty files are considered text
    }

    detect_file_type_from_buffer(&buffer[..bytes_read])
}

/// Detect file type from a buffer of bytes
///
/// Uses the `infer` crate for magic byte detection, with additional
/// heuristics for text detection.
pub fn detect_file_type_from_buffer(buffer: &[u8]) -> DetectedFileType {
    // First, try magic byte detection with infer crate
    if let Some(file_type) = infer::get(buffer) {
        let mime = file_type.mime_type();

        // Map MIME types to our enum
        return match mime {
            "application/pdf" => DetectedFileType::Pdf,
            "application/epub+zip" => DetectedFileType::Epub,
            // Office documents (detected as ZIP, need content check)
            "application/zip" => detect_office_or_archive(buffer),
            // Archives
            "application/gzip"
            | "application/x-tar"
            | "application/x-bzip2"
            | "application/x-xz"
            | "application/x-7z-compressed"
            | "application/x-rar-compressed"
            | "application/zstd" => DetectedFileType::Archive,
            // Images
            _ if mime.starts_with("image/") => DetectedFileType::Image,
            // Audio
            _ if mime.starts_with("audio/") => DetectedFileType::Audio,
            // Video
            _ if mime.starts_with("video/") => DetectedFileType::Video,
            // Executables
            "application/x-executable"
            | "application/x-mach-binary"
            | "application/x-dosexec"
            | "application/vnd.microsoft.portable-executable" => DetectedFileType::Executable,
            // Other binary formats
            _ => DetectedFileType::Binary,
        };
    }

    // No magic bytes matched - check if it's text
    if is_likely_text(buffer) {
        DetectedFileType::Text
    } else {
        DetectedFileType::Binary
    }
}

/// Detect if a ZIP file is actually an Office document or EPUB
fn detect_office_or_archive(buffer: &[u8]) -> DetectedFileType {
    // ZIP-based formats have specific signatures in their content
    // DOCX: contains word/document.xml
    // XLSX: contains xl/workbook.xml
    // PPTX: contains ppt/presentation.xml
    // EPUB: contains META-INF/container.xml and mimetype

    // For now, we check for the [Content_Types].xml marker that Office docs have
    // This is a simplified check - full detection would require parsing the ZIP
    if buffer.len() > 30 {
        // Look for "[Content_Types].xml" signature in the buffer
        let search_bytes = b"[Content_Types].xml";
        if buffer
            .windows(search_bytes.len())
            .any(|w| w == search_bytes)
        {
            // It's an Office document, but we can't easily tell which type
            // without parsing the ZIP. Default to Archive and let extension decide.
            return DetectedFileType::Archive;
        }

        // Check for EPUB mimetype marker
        if buffer.len() > 38 {
            // EPUB has "mimetypeapplication/epub+zip" at offset 30
            let mimetype_check = &buffer[30..buffer.len().min(60)];
            if mimetype_check
                .windows(9)
                .any(|w| w == b"mimetype\0" || w.starts_with(b"mimetype"))
            {
                return DetectedFileType::Epub;
            }
        }
    }

    DetectedFileType::Archive
}

/// Check if a buffer appears to be valid text content
///
/// Uses encoding detection to determine if content is text:
/// - Detects UTF-8, UTF-16, UTF-32 (with and without BOM)
/// - Detects legacy encodings (Latin-1, Windows-1252, etc.) via chardetng
/// - Checks for high ratio of printable characters after decoding
fn is_likely_text(buffer: &[u8]) -> bool {
    if buffer.is_empty() {
        return true;
    }

    // Use encoding detection to check for valid text
    // This handles UTF-8, UTF-16, UTF-32, Latin-1, Windows-1252, etc.
    if is_valid_text_encoding(buffer) {
        return true;
    }

    // Fallback: If encoding detection fails but no null bytes,
    // do a final printability check
    if buffer.contains(&0) {
        // Null bytes could be UTF-16/32, but if encoding detection
        // didn't catch it, it's likely binary
        return false;
    }

    // Try UTF-8 decoding with replacement and check printability
    let text = String::from_utf8_lossy(buffer);

    // Count printable vs control characters
    let mut printable = 0;
    let mut control = 0;
    let mut replacement = 0;

    for c in text.chars() {
        if c == '\u{FFFD}' {
            replacement += 1;
        } else if c.is_ascii_graphic() || c.is_ascii_whitespace() {
            printable += 1;
        } else if c.is_ascii_control() && c != '\n' && c != '\r' && c != '\t' {
            control += 1;
        } else if !c.is_ascii() {
            // Non-ASCII but valid UTF-8 (e.g., CJK, emoji)
            printable += 1;
        }
    }

    // If more than 10% replacement or control characters, likely binary
    let total = printable + control + replacement;
    if total > 0 {
        let bad_ratio = (control + replacement) as f64 / total as f64;
        if bad_ratio > 0.10 {
            return false;
        }
    }

    true
}

/// Check if a file is binary (not text-based)
///
/// Returns true if the file appears to be a binary format that cannot
/// be meaningfully indexed as text.
///
/// Note: Images are considered binary unless the `clip` feature is enabled,
/// in which case they can be indexed via CLIP embeddings.
pub fn is_binary_file(path: &Path) -> bool {
    let file_type = detect_file_type(path);

    // Images are indexable with CLIP feature
    #[cfg(feature = "clip")]
    if file_type == DetectedFileType::Image {
        return false;
    }

    matches!(
        file_type,
        DetectedFileType::Image
            | DetectedFileType::Audio
            | DetectedFileType::Video
            | DetectedFileType::Executable
            | DetectedFileType::Binary
            | DetectedFileType::Archive
    )
}

/// Check if a file should be indexed based on its actual content
///
/// This function reads the file to detect its type using magic bytes,
/// providing more reliable detection than extension-based checks alone.
///
/// Use this for files without extensions or when you want to verify
/// that a file's content matches its extension.
pub fn is_indexable_by_content(path: &Path) -> bool {
    let file_type = detect_file_type(path);

    match file_type {
        DetectedFileType::Text => true,
        DetectedFileType::Pdf => {
            #[cfg(feature = "document-processing")]
            return true;
            #[cfg(not(feature = "document-processing"))]
            return false;
        }
        DetectedFileType::Docx | DetectedFileType::Xlsx | DetectedFileType::Pptx => {
            #[cfg(feature = "document-processing")]
            return true;
            #[cfg(not(feature = "document-processing"))]
            return false;
        }
        DetectedFileType::Epub => {
            #[cfg(feature = "document-processing")]
            return true;
            #[cfg(not(feature = "document-processing"))]
            return false;
        }
        DetectedFileType::Audio | DetectedFileType::Video => {
            #[cfg(feature = "audio-transcription")]
            return true;
            #[cfg(not(feature = "audio-transcription"))]
            return false;
        }
        DetectedFileType::Image => {
            #[cfg(feature = "clip")]
            return true;
            #[cfg(not(feature = "clip"))]
            return false;
        }
        // Binary formats are not indexable
        DetectedFileType::Archive | DetectedFileType::Executable | DetectedFileType::Binary => {
            false
        }
        DetectedFileType::Unknown => {
            // Fall back to extension-based detection for unknown types
            is_indexable_path(path)
        }
    }
}

/// Check if a file has valid text content, useful for validating
/// that a file with a text extension actually contains text.
///
/// This reads the first 8KB of the file to check for binary content.
/// Returns false if the file appears to be binary despite its extension.
pub fn validate_text_file(path: &Path) -> bool {
    let file_type = detect_file_type(path);
    matches!(file_type, DetectedFileType::Text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_indexable_path_extensions() {
        assert!(is_indexable_path(Path::new("/tmp/main.rs")));
        assert!(is_indexable_path(Path::new("/tmp/README.md")));
        assert!(!is_indexable_path(Path::new("/tmp/archive.zip")));
    }

    #[test]
    fn test_is_indexable_path_filenames() {
        assert!(is_indexable_path(Path::new("/tmp/Dockerfile")));
        assert!(is_indexable_path(Path::new("/tmp/Dockerfile.dev")));
        assert!(is_indexable_path(Path::new("/tmp/Makefile")));
        assert!(is_indexable_path(Path::new("/tmp/Makefile.am")));
        assert!(is_indexable_path(Path::new("/tmp/LICENSE")));
        assert!(is_indexable_path(Path::new("/tmp/LICENSE-MIT")));
        assert!(is_indexable_path(Path::new("/tmp/README")));
        assert!(is_indexable_path(Path::new("/tmp/README_old")));
        assert!(is_indexable_path(Path::new("/tmp/CHANGELOG")));
        // Use non-indexable variant to verify separator match
        assert!(!is_indexable_path(Path::new("/tmp/Makefilebak")));
    }

    #[test]
    fn test_is_indexable_path_dotfiles() {
        // Git files
        assert!(is_indexable_path(Path::new("/tmp/.gitignore")));
        assert!(is_indexable_path(Path::new("/tmp/.gitattributes")));
        assert!(is_indexable_path(Path::new("/tmp/.gitmodules")));

        // Docker
        assert!(is_indexable_path(Path::new("/tmp/.dockerignore")));

        // Editor/formatter configs
        assert!(is_indexable_path(Path::new("/tmp/.editorconfig")));
        assert!(is_indexable_path(Path::new("/tmp/.eslintrc")));
        assert!(is_indexable_path(Path::new("/tmp/.eslintrc.json")));
        assert!(is_indexable_path(Path::new("/tmp/.prettierrc")));

        // Version managers
        assert!(is_indexable_path(Path::new("/tmp/.nvmrc")));
        assert!(is_indexable_path(Path::new("/tmp/.node-version")));
        assert!(is_indexable_path(Path::new("/tmp/.python-version")));
        assert!(is_indexable_path(Path::new("/tmp/.ruby-version")));
        assert!(is_indexable_path(Path::new("/tmp/.tool-versions")));

        // Rust configs
        assert!(is_indexable_path(Path::new("/tmp/.rustfmt.toml")));
        assert!(is_indexable_path(Path::new("/tmp/.clippy.toml")));
        assert!(is_indexable_path(Path::new(
            "/tmp/project/.cargo/config.toml"
        )));
        assert!(is_indexable_path(Path::new("/tmp/project/.cargo/config")));

        // Env examples (safe to index)
        assert!(is_indexable_path(Path::new("/tmp/.env.example")));
        assert!(is_indexable_path(Path::new("/tmp/.env.sample")));
        assert!(is_indexable_path(Path::new("/tmp/.env.template")));

        // Actual .env files should NOT be indexed (security)
        assert!(!is_indexable_path(Path::new("/tmp/.env")));
        assert!(!is_indexable_path(Path::new("/tmp/.env.local")));
        assert!(!is_indexable_path(Path::new("/tmp/.env.production")));

        // Random dotfiles should NOT be indexed
        assert!(!is_indexable_path(Path::new("/tmp/.DS_Store")));
        assert!(!is_indexable_path(Path::new("/tmp/.bash_history")));
    }

    #[test]
    fn test_is_indexable_extension_case_insensitive() {
        // Extensions should be case insensitive
        assert!(is_indexable_path(Path::new("/tmp/main.RS")));
        assert!(is_indexable_path(Path::new("/tmp/main.Rs")));
        assert!(is_indexable_path(Path::new("/tmp/main.rS")));
        assert!(is_indexable_path(Path::new("/tmp/README.MD")));
        assert!(is_indexable_path(Path::new("/tmp/config.JSON")));
        assert!(is_indexable_path(Path::new("/tmp/script.PY")));
    }

    #[test]
    fn test_is_indexable_filename_case_insensitive() {
        // Filenames should be case insensitive
        assert!(is_indexable_path(Path::new("/tmp/dockerfile")));
        assert!(is_indexable_path(Path::new("/tmp/DOCKERFILE")));
        assert!(is_indexable_path(Path::new("/tmp/DockerFile")));
        assert!(is_indexable_path(Path::new("/tmp/makefile")));
        assert!(is_indexable_path(Path::new("/tmp/MAKEFILE")));
        assert!(is_indexable_path(Path::new("/tmp/license")));
        assert!(is_indexable_path(Path::new("/tmp/readme")));
        assert!(is_indexable_path(Path::new("/tmp/changelog")));
        assert!(is_indexable_path(Path::new("/tmp/copying")));
    }

    #[test]
    fn test_is_indexable_path_edge_cases() {
        // Empty path components
        assert!(!is_indexable_path(Path::new("")));

        // Path with only extension-like name (no actual extension)
        assert!(!is_indexable_path(Path::new("/tmp/rs")));
        assert!(!is_indexable_path(Path::new("/tmp/py")));

        // Files that look like extensions but aren't
        assert!(!is_indexable_path(Path::new("/tmp/.rs")));

        // Very long paths should still work
        assert!(is_indexable_path(Path::new(
            "/very/long/path/to/some/deeply/nested/directory/main.rs"
        )));

        // Paths with spaces
        assert!(is_indexable_path(Path::new("/tmp/my file.rs")));
        assert!(is_indexable_path(Path::new("/path with spaces/main.py")));

        // Paths with special characters
        assert!(is_indexable_path(Path::new("/tmp/main-v2.rs")));
        assert!(is_indexable_path(Path::new("/tmp/main_v2.rs")));
        assert!(is_indexable_path(Path::new("/tmp/main.v2.rs")));
    }

    #[test]
    fn test_is_indexable_filename_separator_edge_cases() {
        // All valid separators after known filenames
        assert!(is_indexable_path(Path::new("/tmp/Dockerfile.prod")));
        assert!(is_indexable_path(Path::new("/tmp/Dockerfile-prod")));
        assert!(is_indexable_path(Path::new("/tmp/Dockerfile_prod")));

        // Makefile variants
        assert!(is_indexable_path(Path::new("/tmp/Makefile.in")));
        assert!(is_indexable_path(Path::new("/tmp/Makefile-test")));
        assert!(is_indexable_path(Path::new("/tmp/Makefile_backup")));

        // LICENSE variants
        assert!(is_indexable_path(Path::new("/tmp/LICENSE.txt")));
        assert!(is_indexable_path(Path::new("/tmp/LICENSE-APACHE")));

        // Invalid - no separator or wrong separator
        assert!(!is_indexable_path(Path::new("/tmp/Dockerfileprod")));
        assert!(!is_indexable_path(Path::new("/tmp/LICENSEfile")));
    }

    #[test]
    fn test_is_indexable_dotfile_case_insensitive() {
        // Dotfiles should be case insensitive
        assert!(is_indexable_path(Path::new("/tmp/.GITIGNORE")));
        assert!(is_indexable_path(Path::new("/tmp/.GitIgnore")));
        assert!(is_indexable_path(Path::new("/tmp/.EDITORCONFIG")));
        assert!(is_indexable_path(Path::new("/tmp/.EditorConfig")));
    }

    #[test]
    fn test_is_document_file() {
        // PDF files should be detected as document files
        assert!(is_document_file(Path::new("/tmp/report.pdf")));
        assert!(is_document_file(Path::new("/tmp/report.PDF")));
        assert!(is_document_file(Path::new("/tmp/report.Pdf")));

        // DOCX files should be detected as document files
        assert!(is_document_file(Path::new("/tmp/document.docx")));
        assert!(is_document_file(Path::new("/tmp/document.DOCX")));

        // PPTX files should be detected as document files
        assert!(is_document_file(Path::new("/tmp/presentation.pptx")));
        assert!(is_document_file(Path::new("/tmp/presentation.PPTX")));

        // Spreadsheet files should be detected as document files
        assert!(is_document_file(Path::new("/tmp/data.xlsx")));
        assert!(is_document_file(Path::new("/tmp/data.XLSX")));
        assert!(is_document_file(Path::new("/tmp/data.xls")));
        assert!(is_document_file(Path::new("/tmp/data.xlsm")));
        assert!(is_document_file(Path::new("/tmp/data.xlsb")));
        assert!(is_document_file(Path::new("/tmp/data.ods")));

        // EPUB ebook files should be detected as document files
        assert!(is_document_file(Path::new("/tmp/book.epub")));
        assert!(is_document_file(Path::new("/tmp/book.EPUB")));

        // Regular text files should NOT be document files
        assert!(!is_document_file(Path::new("/tmp/main.rs")));
        assert!(!is_document_file(Path::new("/tmp/README.md")));
        assert!(!is_document_file(Path::new("/tmp/config.json")));

        // Other non-document files
        assert!(!is_document_file(Path::new("/tmp/archive.zip")));
        assert!(!is_document_file(Path::new("/tmp/image.png")));
    }

    #[test]
    fn test_is_text_file() {
        // Regular code/text files are text files
        assert!(is_text_file(Path::new("/tmp/main.rs")));
        assert!(is_text_file(Path::new("/tmp/README.md")));

        // Document files are NOT text files (they need special extraction)
        assert!(!is_text_file(Path::new("/tmp/report.pdf")));
        assert!(!is_text_file(Path::new("/tmp/document.docx")));
        assert!(!is_text_file(Path::new("/tmp/data.xlsx")));

        // Non-indexable files are also not text files
        assert!(!is_text_file(Path::new("/tmp/archive.zip")));
    }

    #[test]
    #[cfg(feature = "document-processing")]
    fn test_documents_indexable_with_feature() {
        // With document-processing feature, documents should be indexable
        assert!(is_indexable_path(Path::new("/tmp/report.pdf")));
        assert!(is_indexable_path(Path::new("/tmp/report.PDF")));
        assert!(is_indexable_path(Path::new("/tmp/document.docx")));
        assert!(is_indexable_path(Path::new("/tmp/document.DOCX")));
        assert!(is_indexable_path(Path::new("/tmp/presentation.pptx")));
        assert!(is_indexable_path(Path::new("/tmp/presentation.PPTX")));
        assert!(is_indexable_path(Path::new("/tmp/data.xlsx")));
        assert!(is_indexable_path(Path::new("/tmp/data.xls")));
        assert!(is_indexable_path(Path::new("/tmp/data.ods")));
        assert!(is_indexable_path(Path::new("/tmp/book.epub")));
        assert!(is_indexable_path(Path::new("/tmp/book.EPUB")));
    }

    #[test]
    #[cfg(not(feature = "document-processing"))]
    fn test_documents_not_indexable_without_feature() {
        // Without document-processing feature, documents should NOT be indexable
        assert!(!is_indexable_path(Path::new("/tmp/report.pdf")));
        assert!(!is_indexable_path(Path::new("/tmp/report.PDF")));
        assert!(!is_indexable_path(Path::new("/tmp/document.docx")));
        assert!(!is_indexable_path(Path::new("/tmp/presentation.pptx")));
        assert!(!is_indexable_path(Path::new("/tmp/data.xlsx")));
        assert!(!is_indexable_path(Path::new("/tmp/book.epub")));
    }

    // Magic byte detection tests

    #[test]
    fn test_detect_text_content() {
        // Plain ASCII text
        let text = b"Hello, world!\nThis is a test file.";
        assert_eq!(detect_file_type_from_buffer(text), DetectedFileType::Text);

        // UTF-8 text with unicode
        let unicode = "Hello, 世界! 🎉 こんにちは".as_bytes();
        assert_eq!(
            detect_file_type_from_buffer(unicode),
            DetectedFileType::Text
        );

        // Empty content is text
        assert_eq!(detect_file_type_from_buffer(b""), DetectedFileType::Text);

        // Whitespace only
        assert_eq!(
            detect_file_type_from_buffer(b"   \n\t\r\n  "),
            DetectedFileType::Text
        );
    }

    #[test]
    fn test_detect_binary_content() {
        // Null bytes indicate binary
        let binary = b"hello\x00world";
        assert_eq!(
            detect_file_type_from_buffer(binary),
            DetectedFileType::Binary
        );

        // Random binary data
        let binary = &[0x00, 0x01, 0x02, 0x03, 0xff, 0xfe, 0xfd];
        assert_eq!(
            detect_file_type_from_buffer(binary),
            DetectedFileType::Binary
        );
    }

    #[test]
    fn test_detect_pdf_magic_bytes() {
        // PDF magic bytes: %PDF-
        let pdf_header = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3";
        assert_eq!(
            detect_file_type_from_buffer(pdf_header),
            DetectedFileType::Pdf
        );
    }

    #[test]
    fn test_detect_png_magic_bytes() {
        // PNG magic bytes
        let png_header = &[0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
        assert_eq!(
            detect_file_type_from_buffer(png_header),
            DetectedFileType::Image
        );
    }

    #[test]
    fn test_detect_jpeg_magic_bytes() {
        // JPEG magic bytes
        let jpeg_header = &[0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10, 0x4a, 0x46, 0x49, 0x46];
        assert_eq!(
            detect_file_type_from_buffer(jpeg_header),
            DetectedFileType::Image
        );
    }

    #[test]
    fn test_detect_zip_magic_bytes() {
        // ZIP magic bytes (PK..)
        let zip_header = &[0x50, 0x4b, 0x03, 0x04, 0x00, 0x00, 0x00, 0x00];
        assert_eq!(
            detect_file_type_from_buffer(zip_header),
            DetectedFileType::Archive
        );
    }

    #[test]
    fn test_detect_gzip_magic_bytes() {
        // gzip magic bytes
        let gzip_header = &[0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00];
        assert_eq!(
            detect_file_type_from_buffer(gzip_header),
            DetectedFileType::Archive
        );
    }

    #[test]
    fn test_is_likely_text() {
        // Valid text - ASCII
        assert!(is_likely_text(b"Hello, world!"));
        assert!(is_likely_text(b""));
        assert!(is_likely_text(b"\n\n\n"));

        // Valid text - UTF-8 with CJK
        assert!(is_likely_text("日本語テスト".as_bytes()));

        // Binary content - null bytes in wrong place
        assert!(!is_likely_text(b"hello\x00world"));

        // Random binary
        assert!(!is_likely_text(&[0x00, 0x01, 0x02, 0xff, 0xfe]));
    }

    #[test]
    fn test_is_likely_text_utf16() {
        // UTF-16 LE with BOM should be detected as text
        let utf16_le_bom = &[0xFF, 0xFE, b'H', 0x00, b'i', 0x00];
        assert!(is_likely_text(utf16_le_bom));

        // UTF-16 BE with BOM should be detected as text
        let utf16_be_bom = &[0xFE, 0xFF, 0x00, b'H', 0x00, b'i'];
        assert!(is_likely_text(utf16_be_bom));

        // UTF-16 LE without BOM (long enough for detection)
        let utf16_le_no_bom: Vec<u8> = vec![
            b'H', 0x00, b'e', 0x00, b'l', 0x00, b'l', 0x00, b'o', 0x00, b' ', 0x00, b'W', 0x00,
            b'o', 0x00, b'r', 0x00, b'l', 0x00, b'd', 0x00,
        ];
        assert!(is_likely_text(&utf16_le_no_bom));

        // UTF-16 BE without BOM
        let utf16_be_no_bom: Vec<u8> = vec![
            0x00, b'H', 0x00, b'e', 0x00, b'l', 0x00, b'l', 0x00, b'o', 0x00, b' ', 0x00, b'W',
            0x00, b'o', 0x00, b'r', 0x00, b'l', 0x00, b'd',
        ];
        assert!(is_likely_text(&utf16_be_no_bom));
    }

    #[test]
    fn test_is_likely_text_utf8_bom() {
        // UTF-8 with BOM
        let utf8_bom = &[0xEF, 0xBB, 0xBF, b'H', b'e', b'l', b'l', b'o'];
        assert!(is_likely_text(utf8_bom));
    }

    #[test]
    fn test_is_likely_text_latin1() {
        // ISO-8859-1 (Latin-1): "café" where é is 0xE9
        // This is not valid UTF-8 but should be detected as text via chardetng
        let latin1 = &[b'c', b'a', b'f', 0xE9];
        assert!(is_likely_text(latin1));
    }

    #[test]
    fn test_is_binary_file_with_real_files() {
        use std::io::Write;

        let dir = tempfile::tempdir().unwrap();

        // Create a text file
        let text_path = dir.path().join("test.txt");
        {
            let mut f = std::fs::File::create(&text_path).unwrap();
            f.write_all(b"Hello, world!").unwrap();
        }
        assert!(!is_binary_file(&text_path));
        assert!(validate_text_file(&text_path));

        // Create a binary file - use MP3 header (always binary regardless of features)
        let binary_path = dir.path().join("test.mp3");
        {
            let mut f = std::fs::File::create(&binary_path).unwrap();
            // MP3 ID3v2 header
            f.write_all(b"ID3\x04\x00\x00\x00\x00\x00\x00").unwrap();
        }
        assert!(is_binary_file(&binary_path));
        assert!(!validate_text_file(&binary_path));
    }

    #[test]
    fn test_is_indexable_by_content_text() {
        use std::io::Write;

        let dir = tempfile::tempdir().unwrap();

        // Create a text file without extension
        let text_path = dir.path().join("noextension");
        {
            let mut f = std::fs::File::create(&text_path).unwrap();
            f.write_all(b"This is a plain text file without extension")
                .unwrap();
        }
        assert!(is_indexable_by_content(&text_path));

        // Create a Rust source file
        let rs_path = dir.path().join("main.rs");
        {
            let mut f = std::fs::File::create(&rs_path).unwrap();
            f.write_all(b"fn main() { println!(\"Hello\"); }").unwrap();
        }
        assert!(is_indexable_by_content(&rs_path));
    }

    #[test]
    fn test_is_indexable_by_content_binary() {
        use std::io::Write;

        let dir = tempfile::tempdir().unwrap();

        // Create a binary file (fake image) - with clip feature, images ARE indexable
        let img_path = dir.path().join("image.png");
        {
            let mut f = std::fs::File::create(&img_path).unwrap();
            // PNG header
            f.write_all(&[0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a])
                .unwrap();
        }
        #[cfg(feature = "clip")]
        assert!(is_indexable_by_content(&img_path)); // Images indexable with CLIP
        #[cfg(not(feature = "clip"))]
        assert!(!is_indexable_by_content(&img_path));

        // Binary without extension - never indexable
        let binary_path = dir.path().join("binaryfile");
        {
            let mut f = std::fs::File::create(&binary_path).unwrap();
            f.write_all(&[0x00, 0x01, 0x02, 0xff, 0xfe]).unwrap();
        }
        assert!(!is_indexable_by_content(&binary_path));
    }

    #[test]
    fn test_detected_file_type_enum() {
        // Test that all variants are distinct
        assert_ne!(DetectedFileType::Text, DetectedFileType::Binary);
        assert_ne!(DetectedFileType::Pdf, DetectedFileType::Archive);
        assert_ne!(DetectedFileType::Image, DetectedFileType::Video);

        // Test clone and debug
        let ft = DetectedFileType::Text;
        let ft2 = ft.clone();
        assert_eq!(ft, ft2);
        assert_eq!(format!("{ft:?}"), "Text");
    }

    #[test]
    fn test_is_audio_file() {
        // Audio file extensions
        assert!(is_audio_file(Path::new("/tmp/song.mp3")));
        assert!(is_audio_file(Path::new("/tmp/audio.wav")));
        assert!(is_audio_file(Path::new("/tmp/music.flac")));
        assert!(is_audio_file(Path::new("/tmp/voice.m4a")));
        assert!(is_audio_file(Path::new("/tmp/sound.ogg")));
        assert!(is_audio_file(Path::new("/tmp/podcast.opus")));
        assert!(is_audio_file(Path::new("/tmp/track.aac")));
        assert!(is_audio_file(Path::new("/tmp/audio.aiff")));

        // Case insensitive
        assert!(is_audio_file(Path::new("/tmp/SONG.MP3")));
        assert!(is_audio_file(Path::new("/tmp/Audio.WAV")));

        // Non-audio files
        assert!(!is_audio_file(Path::new("/tmp/main.rs")));
        assert!(!is_audio_file(Path::new("/tmp/video.mp4")));
        assert!(!is_audio_file(Path::new("/tmp/image.png")));
        assert!(!is_audio_file(Path::new("/tmp/document.pdf")));
    }

    #[test]
    fn test_is_video_file() {
        // Video file extensions
        assert!(is_video_file(Path::new("/tmp/movie.mp4")));
        assert!(is_video_file(Path::new("/tmp/video.mkv")));
        assert!(is_video_file(Path::new("/tmp/clip.avi")));
        assert!(is_video_file(Path::new("/tmp/recording.mov")));
        assert!(is_video_file(Path::new("/tmp/stream.flv")));
        assert!(is_video_file(Path::new("/tmp/video.webm")));
        assert!(is_video_file(Path::new("/tmp/film.mpeg")));

        // Case insensitive
        assert!(is_video_file(Path::new("/tmp/MOVIE.MP4")));
        assert!(is_video_file(Path::new("/tmp/Video.MKV")));

        // Non-video files
        assert!(!is_video_file(Path::new("/tmp/main.rs")));
        assert!(!is_video_file(Path::new("/tmp/song.mp3")));
        assert!(!is_video_file(Path::new("/tmp/image.png")));
        assert!(!is_video_file(Path::new("/tmp/document.pdf")));
    }

    #[test]
    fn test_is_media_file() {
        // Audio files are media
        assert!(is_media_file(Path::new("/tmp/song.mp3")));
        assert!(is_media_file(Path::new("/tmp/audio.wav")));

        // Video files are media
        assert!(is_media_file(Path::new("/tmp/movie.mp4")));
        assert!(is_media_file(Path::new("/tmp/video.mkv")));

        // Non-media files
        assert!(!is_media_file(Path::new("/tmp/main.rs")));
        assert!(!is_media_file(Path::new("/tmp/image.png")));
        assert!(!is_media_file(Path::new("/tmp/document.pdf")));
    }

    #[test]
    #[cfg(feature = "audio-transcription")]
    fn test_audio_indexable_with_feature() {
        // With audio-transcription feature, audio/video should be indexable
        assert!(is_indexable_path(Path::new("/tmp/podcast.mp3")));
        assert!(is_indexable_path(Path::new("/tmp/voice.wav")));
        assert!(is_indexable_path(Path::new("/tmp/meeting.mp4")));
        assert!(is_indexable_path(Path::new("/tmp/lecture.mkv")));
    }

    #[test]
    #[cfg(not(feature = "audio-transcription"))]
    fn test_audio_not_indexable_without_feature() {
        // Without audio-transcription feature, audio/video should NOT be indexable
        assert!(!is_indexable_path(Path::new("/tmp/podcast.mp3")));
        assert!(!is_indexable_path(Path::new("/tmp/voice.wav")));
        assert!(!is_indexable_path(Path::new("/tmp/meeting.mp4")));
        assert!(!is_indexable_path(Path::new("/tmp/lecture.mkv")));
    }

    #[test]
    fn test_is_image_file() {
        // Image file extensions
        assert!(is_image_file(Path::new("/tmp/photo.png")));
        assert!(is_image_file(Path::new("/tmp/image.jpg")));
        assert!(is_image_file(Path::new("/tmp/photo.jpeg")));
        assert!(is_image_file(Path::new("/tmp/animation.gif")));
        assert!(is_image_file(Path::new("/tmp/bitmap.bmp")));
        assert!(is_image_file(Path::new("/tmp/modern.webp")));
        assert!(is_image_file(Path::new("/tmp/scan.tiff")));
        assert!(is_image_file(Path::new("/tmp/scan.tif")));
        assert!(is_image_file(Path::new("/tmp/icon.ico")));
        assert!(is_image_file(Path::new("/tmp/apple.heic")));
        assert!(is_image_file(Path::new("/tmp/apple.heif")));
        assert!(is_image_file(Path::new("/tmp/new.avif")));

        // Case insensitive
        assert!(is_image_file(Path::new("/tmp/PHOTO.PNG")));
        assert!(is_image_file(Path::new("/tmp/Image.JPG")));
        assert!(is_image_file(Path::new("/tmp/Photo.JPEG")));

        // Non-image files
        assert!(!is_image_file(Path::new("/tmp/main.rs")));
        assert!(!is_image_file(Path::new("/tmp/song.mp3")));
        assert!(!is_image_file(Path::new("/tmp/video.mp4")));
        assert!(!is_image_file(Path::new("/tmp/document.pdf")));
    }

    #[test]
    #[cfg(feature = "clip")]
    fn test_images_indexable_with_clip_feature() {
        // With clip feature, images should be indexable
        assert!(is_indexable_path(Path::new("/tmp/photo.png")));
        assert!(is_indexable_path(Path::new("/tmp/image.jpg")));
        assert!(is_indexable_path(Path::new("/tmp/photo.jpeg")));
        assert!(is_indexable_path(Path::new("/tmp/animation.gif")));
        assert!(is_indexable_path(Path::new("/tmp/modern.webp")));
    }

    #[test]
    #[cfg(not(feature = "clip"))]
    fn test_images_not_indexable_without_clip_feature() {
        // Without clip feature, images should NOT be indexable
        assert!(!is_indexable_path(Path::new("/tmp/photo.png")));
        assert!(!is_indexable_path(Path::new("/tmp/image.jpg")));
        assert!(!is_indexable_path(Path::new("/tmp/photo.jpeg")));
        assert!(!is_indexable_path(Path::new("/tmp/animation.gif")));
        assert!(!is_indexable_path(Path::new("/tmp/modern.webp")));
    }

    #[test]
    #[cfg(feature = "clip")]
    fn test_is_text_file_excludes_images_with_clip() {
        // Images are not text files even with clip feature
        assert!(!is_text_file(Path::new("/tmp/photo.png")));
        assert!(!is_text_file(Path::new("/tmp/image.jpg")));
    }
}
