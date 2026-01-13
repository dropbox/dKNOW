//! File summary extraction and storage
//!
//! Provides one-line summaries for files to help AI tools understand
//! file contents without opening them. Uses tiered storage:
//! - Tier 1: Extended attributes (xattr) - fast, file-local
//! - Tier 2: SQLite database - universal fallback
//! - Tier 3: Extract from file (first line for text, MIME for binary)

use crate::file_types::is_text_file as check_text_file;
use crate::storage::DB;
use anyhow::{Context, Result};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// The xattr key used for storing summaries
pub const XATTR_KEY: &str = "user.sg.summary";

/// Maximum summary length in characters
pub const MAX_SUMMARY_LEN: usize = 200;

/// Source of a file summary
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SummarySource {
    /// First line of a text file
    FirstLine,
    /// MIME type detection for binary files
    Mime,
    /// LLM-generated summary (model name stored)
    Llm(String),
    /// From extended attribute
    Xattr,
    /// From SQLite database
    Sqlite,
}

impl SummarySource {
    /// Convert to string for storage
    pub fn as_str(&self) -> &str {
        match self {
            SummarySource::FirstLine => "firstline",
            SummarySource::Mime => "mime",
            SummarySource::Llm(_) => "llm",
            SummarySource::Xattr => "xattr",
            SummarySource::Sqlite => "sqlite",
        }
    }

    /// Parse from storage string
    pub fn from_str(s: &str, model: Option<&str>) -> Self {
        match s {
            "firstline" => SummarySource::FirstLine,
            "mime" => SummarySource::Mime,
            "llm" => SummarySource::Llm(model.unwrap_or("unknown").to_string()),
            "xattr" => SummarySource::Xattr,
            "sqlite" => SummarySource::Sqlite,
            _ => SummarySource::FirstLine,
        }
    }
}

/// A resolved file summary with metadata
#[derive(Debug, Clone)]
pub struct FileSummary {
    /// The summary text
    pub summary: String,
    /// Where the summary came from
    pub source: SummarySource,
    /// True if the file changed since the summary was generated
    pub is_stale: bool,
}

impl FileSummary {
    /// Create a new summary
    pub fn new(summary: String, source: SummarySource) -> Self {
        Self {
            summary,
            source,
            is_stale: false,
        }
    }

    /// Mark as stale
    pub fn stale(mut self) -> Self {
        self.is_stale = true;
        self
    }
}

/// Read summary from extended attribute.
///
/// Returns None if xattr is not set or not supported.
#[cfg(unix)]
pub fn read_xattr(path: &Path) -> Option<String> {
    match xattr::get(path, XATTR_KEY) {
        Ok(Some(data)) => String::from_utf8(data).ok(),
        Ok(None) => None,
        Err(_) => None,
    }
}

#[cfg(not(unix))]
pub fn read_xattr(_path: &Path) -> Option<String> {
    None // xattr not supported on Windows
}

/// Write summary to extended attribute.
///
/// Returns true if successful, false if xattr is not supported or write failed.
#[cfg(unix)]
pub fn write_xattr(path: &Path, summary: &str) -> bool {
    xattr::set(path, XATTR_KEY, summary.as_bytes()).is_ok()
}

#[cfg(not(unix))]
pub fn write_xattr(_path: &Path, _summary: &str) -> bool {
    false
}

/// Remove summary from extended attribute.
#[cfg(unix)]
pub fn remove_xattr(path: &Path) -> bool {
    xattr::remove(path, XATTR_KEY).is_ok()
}

#[cfg(not(unix))]
pub fn remove_xattr(_path: &Path) -> bool {
    false
}

/// Check if xattr is supported for a path.
#[cfg(unix)]
pub fn xattr_supported(path: &Path) -> bool {
    xattr::list(path).is_ok()
}

#[cfg(not(unix))]
pub fn xattr_supported(_path: &Path) -> bool {
    false
}

/// Extract the first line of a text file.
///
/// Strips leading comment characters (// /* # -- ") for cleaner summaries.
/// Returns up to MAX_SUMMARY_LEN characters.
pub fn extract_first_line(path: &Path) -> Result<String> {
    let file = File::open(path).with_context(|| format!("Failed to open {}", path.display()))?;
    let reader = BufReader::new(file);

    // Read first non-empty line
    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();

        // Skip empty lines
        if trimmed.is_empty() {
            continue;
        }

        // Clean up common comment prefixes
        let cleaned = strip_comment_prefix(trimmed);

        // Truncate if needed
        let summary = if cleaned.len() > MAX_SUMMARY_LEN {
            format!("{}...", &cleaned[..MAX_SUMMARY_LEN - 3])
        } else {
            cleaned.to_string()
        };

        return Ok(summary);
    }

    Ok("[empty file]".to_string())
}

/// Extract first line from content string.
pub fn extract_first_line_from_content(content: &str) -> String {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let cleaned = strip_comment_prefix(trimmed);
        return if cleaned.len() > MAX_SUMMARY_LEN {
            format!("{}...", &cleaned[..MAX_SUMMARY_LEN - 3])
        } else {
            cleaned.to_string()
        };
    }

    "[empty file]".to_string()
}

/// Strip common comment prefixes from a line.
fn strip_comment_prefix(line: &str) -> &str {
    let line = line.trim();

    // Doc comments (Rust, JS, etc.)
    if let Some(rest) = line.strip_prefix("///") {
        return rest.trim();
    }
    if let Some(rest) = line.strip_prefix("//!") {
        return rest.trim();
    }

    // Single-line comments
    if let Some(rest) = line.strip_prefix("//") {
        return rest.trim();
    }
    if let Some(rest) = line.strip_prefix("#") {
        // Don't strip shebang
        if !line.starts_with("#!") {
            return rest.trim();
        }
    }
    if let Some(rest) = line.strip_prefix("--") {
        return rest.trim();
    }

    // Block comment start
    if let Some(rest) = line.strip_prefix("/*") {
        let rest = rest.trim();
        // Handle /** style
        if let Some(r) = rest.strip_prefix("*") {
            return r.trim();
        }
        return rest;
    }

    // Docstring quotes (Python)
    if let Some(rest) = line.strip_prefix("\"\"\"") {
        return rest.trim();
    }
    if let Some(rest) = line.strip_prefix("'''") {
        return rest.trim();
    }

    line
}

/// Detect MIME type for a file.
///
/// Uses magic bytes to detect file type, with extension fallback.
pub fn detect_mime(path: &Path) -> String {
    // Try to read first few bytes for magic detection
    if let Ok(file) = File::open(path) {
        let mut reader = BufReader::new(file);
        let mut buffer = [0u8; 8192];
        if let Ok(n) = std::io::Read::read(&mut reader, &mut buffer) {
            if let Some(kind) = infer::get(&buffer[..n]) {
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("bin");
                return format!("{} ({})", kind.mime_type(), ext.to_uppercase());
            }
        }
    }

    // Fallback to extension-based detection
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("binary");

    format!("Binary file ({})", ext.to_uppercase())
}

/// Detect MIME type from content bytes.
pub fn detect_mime_from_bytes(bytes: &[u8], ext: Option<&str>) -> String {
    if let Some(kind) = infer::get(bytes) {
        let ext_str = ext.unwrap_or("bin");
        return format!("{} ({})", kind.mime_type(), ext_str.to_uppercase());
    }

    let ext_str = ext.unwrap_or("binary");
    format!("Binary file ({})", ext_str.to_uppercase())
}

/// Check if a file is a text file (vs binary).
pub fn is_text_file(path: &Path) -> bool {
    check_text_file(path)
}

/// Get summary for a file, using tiered resolution.
///
/// Resolution order:
/// 1. Extended attribute (if available)
/// 2. SQLite database (if indexed)
/// 3. First line (text) or MIME type (binary)
pub fn get_summary(path: &Path, db: Option<&DB>) -> Result<FileSummary> {
    // Tier 1: Try xattr
    if let Some(summary) = read_xattr(path) {
        return Ok(FileSummary::new(summary, SummarySource::Xattr));
    }

    // Tier 2: Try SQLite
    if let Some(db) = db {
        if let Some(record) = db.get_summary_by_path(path)? {
            let source = SummarySource::from_str(&record.source, record.model.as_deref());
            let mut summary = FileSummary::new(record.summary, source);
            if record.is_stale {
                summary = summary.stale();
            }
            return Ok(summary);
        }
    }

    // Tier 3: Extract from file
    if is_text_file(path) {
        let summary = extract_first_line(path)?;
        Ok(FileSummary::new(summary, SummarySource::FirstLine))
    } else {
        let mime = detect_mime(path);
        Ok(FileSummary::new(mime, SummarySource::Mime))
    }
}

/// Get summary for content that's already loaded.
///
/// Used during indexing when we have the content in memory.
pub fn get_summary_from_content(content: &str, path: &Path) -> FileSummary {
    // Check if it looks like text
    let is_text = content.chars().take(1000).all(|c| !c.is_control() || c.is_whitespace());

    if is_text && !content.is_empty() {
        let summary = extract_first_line_from_content(content);
        FileSummary::new(summary, SummarySource::FirstLine)
    } else {
        // Binary content
        let mime = detect_mime_from_bytes(content.as_bytes(), path.extension().and_then(|e| e.to_str()));
        FileSummary::new(mime, SummarySource::Mime)
    }
}

/// Store a summary to both xattr (if supported) and SQLite.
pub fn store_summary(
    path: &Path,
    db: &DB,
    doc_id: u32,
    summary: &str,
    source: &SummarySource,
    hash: &str,
) -> Result<()> {
    // Try to write to xattr
    let storage_tier = if write_xattr(path, summary) {
        "xattr"
    } else {
        "sqlite"
    };

    // Always store in SQLite as backup
    let model = match source {
        SummarySource::Llm(m) => Some(m.as_str()),
        _ => None,
    };

    db.upsert_summary(doc_id, summary, source.as_str(), model, hash, storage_tier)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_strip_comment_prefix() {
        assert_eq!(strip_comment_prefix("// Hello world"), "Hello world");
        assert_eq!(strip_comment_prefix("/// Doc comment"), "Doc comment");
        assert_eq!(strip_comment_prefix("//! Module doc"), "Module doc");
        assert_eq!(strip_comment_prefix("# Python comment"), "Python comment");
        assert_eq!(strip_comment_prefix("-- SQL comment"), "SQL comment");
        assert_eq!(strip_comment_prefix("/* Block start"), "Block start");
        assert_eq!(strip_comment_prefix("/** JSDoc"), "JSDoc");
        assert_eq!(strip_comment_prefix("Normal line"), "Normal line");
        // Shebang should be preserved
        assert_eq!(strip_comment_prefix("#!/bin/bash"), "#!/bin/bash");
    }

    #[test]
    fn test_extract_first_line() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "// This is the summary").unwrap();
        writeln!(file, "fn main() {{}}").unwrap();

        let summary = extract_first_line(file.path()).unwrap();
        assert_eq!(summary, "This is the summary");
    }

    #[test]
    fn test_extract_first_line_skips_empty() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file).unwrap();
        writeln!(file).unwrap();
        writeln!(file, "// Actual content").unwrap();

        let summary = extract_first_line(file.path()).unwrap();
        assert_eq!(summary, "Actual content");
    }

    #[test]
    fn test_extract_first_line_from_content() {
        let content = "/// Module documentation\nfn main() {}";
        let summary = extract_first_line_from_content(content);
        assert_eq!(summary, "Module documentation");
    }

    #[test]
    fn test_summary_truncation() {
        let long_line = "x".repeat(300);
        let content = format!("// {long_line}");
        let summary = extract_first_line_from_content(&content);
        assert!(summary.len() <= MAX_SUMMARY_LEN);
        assert!(summary.ends_with("..."));
    }

    #[test]
    fn test_summary_source_roundtrip() {
        assert_eq!(
            SummarySource::from_str("firstline", None),
            SummarySource::FirstLine
        );
        assert_eq!(SummarySource::from_str("mime", None), SummarySource::Mime);
        assert_eq!(
            SummarySource::from_str("llm", Some("gpt-4")),
            SummarySource::Llm("gpt-4".to_string())
        );
    }
}
