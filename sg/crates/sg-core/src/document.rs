//! Document text extraction for various file formats
//!
//! This module provides text extraction for document formats that cannot be
//! read as plain text (e.g., PDFs, Office documents). It's enabled via the
//! `document-processing` feature.
//!
//! Also provides frontmatter extraction for markdown files with YAML/TOML
//! metadata headers.

#[cfg(feature = "document-processing")]
use anyhow::Context;
use anyhow::Result;
use std::path::Path;

/// Markdown frontmatter metadata
#[derive(Default, Debug)]
pub struct Frontmatter {
    pub title: Option<String>,
    pub description: Option<String>,
    pub author: Option<String>,
    pub date: Option<String>,
    pub tags: Vec<String>,
    pub categories: Vec<String>,
    /// Raw content after frontmatter
    pub content: String,
}

/// Extract YAML or TOML frontmatter from markdown content
///
/// Supports two formats:
/// - YAML: delimited by `---` at start and end
/// - TOML: delimited by `+++` at start and end
///
/// Returns the parsed frontmatter metadata and the content without frontmatter.
pub fn extract_frontmatter(content: &str) -> Frontmatter {
    let content = content.trim_start();

    // Try YAML frontmatter (---)
    if content.starts_with("---") {
        if let Some(fm) = parse_yaml_frontmatter(content) {
            return fm;
        }
    }

    // Try TOML frontmatter (+++)
    if content.starts_with("+++") {
        if let Some(fm) = parse_toml_frontmatter(content) {
            return fm;
        }
    }

    // No frontmatter found
    Frontmatter {
        content: content.to_string(),
        ..Default::default()
    }
}

/// Parse YAML frontmatter delimited by ---
fn parse_yaml_frontmatter(content: &str) -> Option<Frontmatter> {
    // Find the closing ---
    let after_first = &content[3..];
    let end_pos = after_first.find("\n---")?;
    let yaml_content = &after_first[..end_pos];
    let remaining_content = &after_first[end_pos + 4..].trim_start();

    let mut fm = Frontmatter {
        content: remaining_content.to_string(),
        ..Default::default()
    };

    // Parse simple key-value pairs from YAML
    for line in yaml_content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim().to_lowercase();
            let value = value.trim().trim_matches(|c| c == '"' || c == '\'');

            match key.as_str() {
                "title" => fm.title = Some(value.to_string()),
                "description" | "summary" | "excerpt" => fm.description = Some(value.to_string()),
                "author" | "authors" => fm.author = Some(value.to_string()),
                "date" | "created" | "published" => fm.date = Some(value.to_string()),
                "tags" => fm.tags = parse_yaml_list(value),
                "categories" | "category" => fm.categories = parse_yaml_list(value),
                _ => {}
            }
        }
    }

    Some(fm)
}

/// Parse TOML frontmatter delimited by +++
fn parse_toml_frontmatter(content: &str) -> Option<Frontmatter> {
    // Find the closing +++
    let after_first = &content[3..];
    let end_pos = after_first.find("\n+++")?;
    let toml_content = &after_first[..end_pos];
    let remaining_content = &after_first[end_pos + 4..].trim_start();

    let mut fm = Frontmatter {
        content: remaining_content.to_string(),
        ..Default::default()
    };

    // Parse simple key-value pairs from TOML
    for line in toml_content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim().to_lowercase();
            let value = value.trim().trim_matches(|c| c == '"' || c == '\'');

            match key.as_str() {
                "title" => fm.title = Some(value.to_string()),
                "description" | "summary" => fm.description = Some(value.to_string()),
                "author" => fm.author = Some(value.to_string()),
                "date" => fm.date = Some(value.to_string()),
                "tags" => fm.tags = parse_toml_array(value),
                "categories" => fm.categories = parse_toml_array(value),
                _ => {}
            }
        }
    }

    Some(fm)
}

/// Parse a YAML inline list like `[tag1, tag2]` or flow style
fn parse_yaml_list(value: &str) -> Vec<String> {
    let value = value.trim();
    if value.starts_with('[') && value.ends_with(']') {
        // Inline array format: [tag1, tag2]
        value[1..value.len() - 1]
            .split(',')
            .map(|s| s.trim().trim_matches(|c| c == '"' || c == '\'').to_string())
            .filter(|s| !s.is_empty())
            .collect()
    } else if !value.is_empty() {
        // Single value
        vec![value.to_string()]
    } else {
        vec![]
    }
}

/// Parse a TOML array like `["tag1", "tag2"]`
fn parse_toml_array(value: &str) -> Vec<String> {
    let value = value.trim();
    if value.starts_with('[') && value.ends_with(']') {
        value[1..value.len() - 1]
            .split(',')
            .map(|s| s.trim().trim_matches(|c| c == '"' || c == '\'').to_string())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        vec![]
    }
}

/// Format frontmatter metadata as a markdown header for search
///
/// This prepends metadata to improve search relevance for markdown files
/// with frontmatter.
pub fn format_frontmatter_header(fm: &Frontmatter) -> String {
    let mut header = String::new();

    if let Some(title) = &fm.title {
        header.push_str(&format!("# {title}\n\n"));
    }

    if let Some(description) = &fm.description {
        header.push_str(&format!("{description}\n\n"));
    }

    if let Some(author) = &fm.author {
        header.push_str(&format!("**Author:** {author}\n"));
    }

    if let Some(date) = &fm.date {
        header.push_str(&format!("**Date:** {date}\n"));
    }

    if !fm.tags.is_empty() {
        header.push_str(&format!("**Tags:** {}\n", fm.tags.join(", ")));
    }

    if !fm.categories.is_empty() {
        header.push_str(&format!("**Categories:** {}\n", fm.categories.join(", ")));
    }

    if !header.is_empty() {
        header.push('\n');
    }

    header
}

/// Check if a file is a markdown file that might have frontmatter
pub fn is_markdown_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| matches!(e.to_lowercase().as_str(), "md" | "markdown" | "mdx"))
        .unwrap_or(false)
}

/// Process markdown content to extract and format frontmatter
///
/// Returns the content with frontmatter metadata prepended as a header.
/// If no frontmatter is found, returns the original content unchanged.
pub fn process_markdown_frontmatter(content: &str) -> String {
    let fm = extract_frontmatter(content);

    // If no metadata was extracted, return content as-is
    if fm.title.is_none()
        && fm.description.is_none()
        && fm.author.is_none()
        && fm.date.is_none()
        && fm.tags.is_empty()
        && fm.categories.is_empty()
    {
        return fm.content;
    }

    let header = format_frontmatter_header(&fm);
    format!("{}{}", header, fm.content)
}

/// Extract text content from a document file
///
/// Currently supports:
/// - PDF files (via docling-backend, pure Rust)
/// - DOCX files (via docx-lite)
/// - XLSX files (via calamine)
/// - PPTX files (via pptx-to-md)
/// - EPUB files (via epub)
///
/// Returns the extracted text content.
#[cfg(feature = "document-processing")]
pub fn extract_document_text(path: &Path) -> Result<String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "pdf" => extract_pdf_text(path),
        "docx" => extract_docx_text(path),
        "pptx" => extract_pptx_text(path),
        "xlsx" | "xls" | "xlsm" | "xlsb" | "ods" => extract_spreadsheet_text(path),
        "epub" => extract_epub_text(path),
        _ => anyhow::bail!("Unsupported document format: {ext}"),
    }
}

/// PDF document metadata
#[cfg(feature = "document-processing")]
#[derive(Default)]
struct PdfMetadata {
    title: Option<String>,
    author: Option<String>,
    subject: Option<String>,
    keywords: Option<String>,
}

/// Extract metadata from a PDF file using lopdf
#[cfg(feature = "document-processing")]
fn extract_pdf_metadata(path: &Path) -> PdfMetadata {
    use lopdf::Document;

    let Ok(doc) = Document::load(path) else {
        tracing::debug!("Failed to load PDF for metadata: {}", path.display());
        return PdfMetadata::default();
    };

    // Get the document info dictionary
    let info = match doc.trailer.get(b"Info") {
        Ok(info_ref) => {
            // as_reference() returns (u32, u16) object ID tuple
            if let Ok(ref_id) = info_ref.as_reference() {
                doc.get_object(ref_id).ok()
            } else {
                None
            }
        }
        Err(_) => None,
    };

    let Some(lopdf::Object::Dictionary(info_dict)) = info else {
        tracing::debug!("No Info dictionary in PDF: {}", path.display());
        return PdfMetadata::default();
    };

    // Helper to extract string from info dict
    let get_string = |key: &[u8]| -> Option<String> {
        info_dict.get(key).ok().and_then(|obj| {
            match obj {
                lopdf::Object::String(bytes, _) => {
                    // Try UTF-8 first, then Latin-1
                    String::from_utf8(bytes.clone())
                        .ok()
                        .or_else(|| Some(bytes.iter().map(|&b| b as char).collect()))
                }
                _ => None,
            }
        })
    };

    PdfMetadata {
        title: get_string(b"Title"),
        author: get_string(b"Author"),
        subject: get_string(b"Subject"),
        keywords: get_string(b"Keywords"),
    }
}

/// Format PDF metadata as a markdown header
#[cfg(feature = "document-processing")]
fn format_pdf_metadata(metadata: &PdfMetadata) -> String {
    let mut header = String::new();

    if let Some(title) = &metadata.title {
        let title = title.trim();
        if !title.is_empty() {
            header.push_str(&format!("# {title}\n\n"));
        }
    }

    if let Some(author) = &metadata.author {
        let author = author.trim();
        if !author.is_empty() {
            header.push_str(&format!("**Author:** {author}\n"));
        }
    }

    if let Some(subject) = &metadata.subject {
        let subject = subject.trim();
        if !subject.is_empty() {
            header.push_str(&format!("**Subject:** {subject}\n"));
        }
    }

    if let Some(keywords) = &metadata.keywords {
        let keywords = keywords.trim();
        if !keywords.is_empty() {
            header.push_str(&format!("**Keywords:** {keywords}\n"));
        }
    }

    if !header.is_empty() {
        header.push('\n');
    }

    header
}

/// Minimum characters for considering PDF text extraction successful.
/// PDFs with less text may be scanned and eligible for OCR fallback.
#[cfg(feature = "document-processing")]
const PDF_MIN_TEXT_THRESHOLD: usize = 50;

/// Extract text from a PDF file
///
/// Tries extraction methods in order:
/// 1. docling-backend (layout-aware, best quality)
/// 2. pdf_extract (pure Rust fallback)
/// 3. OCR via docling-ocr (for scanned PDFs, requires `ocr` feature)
///
/// Prepends document metadata (title, author, subject) to improve search relevance.
#[cfg(feature = "document-processing")]
fn extract_pdf_text(path: &Path) -> Result<String> {
    // Extract metadata first (fast, doesn't require full text extraction)
    let metadata = extract_pdf_metadata(path);
    let metadata_header = format_pdf_metadata(&metadata);

    // Try docling-backend first (better layout-aware extraction)
    match try_docling_pdf(path) {
        Ok(text) if text.trim().len() >= PDF_MIN_TEXT_THRESHOLD => {
            tracing::debug!("PDF extracted with docling-backend: {} chars", text.len());
            // Also extract tables
            let tables = try_extract_pdf_tables(path);
            return Ok(format!("{metadata_header}{text}{tables}"));
        }
        Ok(text) if !text.trim().is_empty() => {
            tracing::debug!(
                "docling-backend returned minimal text ({} chars), trying fallback",
                text.trim().len()
            );
        }
        Ok(_) => {
            tracing::debug!("docling-backend returned empty text, trying fallback");
        }
        Err(e) => {
            tracing::debug!("docling-backend failed: {}, trying fallback", e);
        }
    }

    // Fallback to pdf_extract (pure Rust, no external libs)
    let text = pdf_extract::extract_text(path)
        .with_context(|| format!("Failed to extract text from PDF: {}", path.display()))?;

    // If we got enough text, return it
    if text.trim().len() >= PDF_MIN_TEXT_THRESHOLD {
        tracing::debug!(
            "PDF extracted with pdf_extract fallback: {} chars",
            text.len()
        );
        // Also extract tables
        let tables = try_extract_pdf_tables(path);
        return Ok(format!("{metadata_header}{text}{tables}"));
    }

    // Text is minimal - might be a scanned PDF. Try OCR if available.
    #[cfg(feature = "ocr")]
    {
        tracing::debug!(
            "PDF text too short ({} chars), attempting OCR fallback",
            text.trim().len()
        );
        match try_ocr_pdf(path) {
            Ok(ocr_text) if !ocr_text.trim().is_empty() => {
                tracing::debug!("PDF extracted with OCR: {} chars", ocr_text.len());
                // Also extract tables
                let tables = try_extract_pdf_tables(path);
                return Ok(format!("{metadata_header}{ocr_text}{tables}"));
            }
            Ok(_) => {
                tracing::debug!("OCR returned empty text");
            }
            Err(e) => {
                tracing::debug!("OCR failed: {}", e);
            }
        }
    }

    // Return whatever text we have (even if minimal)
    tracing::debug!(
        "PDF extracted with pdf_extract fallback: {} chars",
        text.len()
    );
    // Also extract tables
    let tables = try_extract_pdf_tables(path);
    Ok(format!("{metadata_header}{text}{tables}"))
}

/// Try to extract PDF text using OCR
#[cfg(all(feature = "document-processing", feature = "ocr"))]
fn try_ocr_pdf(path: &Path) -> Result<String> {
    use crate::ocr::ocr_pdf;
    ocr_pdf(path)
}

/// Try to extract PDF text using docling-backend
#[cfg(feature = "document-processing")]
fn try_docling_pdf(path: &Path) -> Result<String> {
    let converter = docling_backend::RustDocumentConverter::new()
        .with_context(|| "Failed to create document converter")?;
    let result = converter
        .convert(path)
        .with_context(|| format!("Failed to convert PDF: {}", path.display()))?;

    Ok(result.document.markdown)
}

/// Try to extract tables from a PDF
///
/// Uses heuristic-based table detection to find and extract tables.
/// Returns markdown-formatted tables, or empty string if none found.
#[cfg(feature = "document-processing")]
fn try_extract_pdf_tables(path: &Path) -> String {
    use crate::table_detector;

    match table_detector::extract_tables_as_markdown(path) {
        Ok(tables) if !tables.is_empty() => {
            tracing::debug!("Extracted {} chars of table content from PDF", tables.len());
            format!("\n\n## Extracted Tables\n\n{tables}")
        }
        Ok(_) => {
            tracing::debug!("No tables found in PDF");
            String::new()
        }
        Err(e) => {
            tracing::debug!("Table extraction failed: {}", e);
            String::new()
        }
    }
}

/// Extract text from a DOCX file using docx-lite
#[cfg(feature = "document-processing")]
fn extract_docx_text(path: &Path) -> Result<String> {
    let text = docx_lite::extract_text(path)
        .with_context(|| format!("Failed to extract text from DOCX: {}", path.display()))?;

    Ok(text)
}

/// Extract text from a PowerPoint presentation using pptx-to-md
///
/// Converts slides to markdown format for semantic search.
#[cfg(feature = "document-processing")]
fn extract_pptx_text(path: &Path) -> Result<String> {
    use pptx_to_md::{ParserConfig, PptxContainer};

    let config = ParserConfig::builder()
        .extract_images(false) // Don't extract images for text search
        .include_slide_comment(true) // Include slide numbers as context
        .build();

    let mut container = PptxContainer::open(path, config)
        .with_context(|| format!("Failed to open PPTX: {}", path.display()))?;

    let slides = container
        .parse_all()
        .with_context(|| format!("Failed to parse PPTX slides: {}", path.display()))?;

    let mut all_text = String::new();

    for slide in slides {
        if let Some(md_content) = slide.convert_to_md() {
            all_text.push_str(&md_content);
            all_text.push_str("\n\n");
        }
    }

    Ok(all_text)
}

/// Extract text from a spreadsheet file using calamine
///
/// Supports: XLSX, XLS, XLSM, XLSB, ODS
#[cfg(feature = "document-processing")]
fn extract_spreadsheet_text(path: &Path) -> Result<String> {
    use calamine::{open_workbook_auto, Data, Reader};

    let mut workbook = open_workbook_auto(path)
        .with_context(|| format!("Failed to open spreadsheet: {}", path.display()))?;

    let mut all_text = String::new();

    for sheet_name in workbook.sheet_names().to_vec() {
        if let Ok(range) = workbook.worksheet_range(&sheet_name) {
            // Add sheet name as a header
            all_text.push_str(&format!("## {sheet_name}\n\n"));

            for row in range.rows() {
                let row_text: Vec<String> = row
                    .iter()
                    .map(|cell| match cell {
                        Data::Empty => String::new(),
                        Data::String(s) => s.clone(),
                        Data::Float(f) => f.to_string(),
                        Data::Int(i) => i.to_string(),
                        Data::Bool(b) => b.to_string(),
                        Data::Error(e) => format!("#ERR:{e:?}"),
                        Data::DateTime(dt) => dt.to_string(),
                        Data::DateTimeIso(s) => s.clone(),
                        Data::DurationIso(s) => s.clone(),
                    })
                    .collect();

                // Skip empty rows
                if row_text.iter().all(|s| s.is_empty()) {
                    continue;
                }

                // Join cells with tabs, add newline
                all_text.push_str(&row_text.join("\t"));
                all_text.push('\n');
            }

            all_text.push('\n');
        }
    }

    Ok(all_text)
}

/// Extract text from an EPUB ebook
///
/// Reads the EPUB file and extracts text content from all chapters/sections.
/// Returns the extracted text with chapter titles as markdown headers.
#[cfg(feature = "document-processing")]
fn extract_epub_text(path: &Path) -> Result<String> {
    use epub::doc::EpubDoc;

    let mut doc =
        EpubDoc::new(path).with_context(|| format!("Failed to open EPUB: {}", path.display()))?;

    let mut all_text = String::new();

    // Extract metadata as header
    if let Some(title) = doc.get_title() {
        all_text.push_str(&format!("# {title}\n\n"));
    }
    if let Some(creator) = doc.mdata("creator") {
        all_text.push_str(&format!("**Author:** {}\n\n", creator.value));
    }

    // Extract text from each spine item (chapter/section)
    let num_chapters = doc.get_num_chapters();
    for chapter in 0..num_chapters {
        doc.set_current_chapter(chapter);
        // Get the resource content as string
        if let Some((content, _mime)) = doc.get_current_str() {
            // Strip HTML tags to get plain text
            let text = strip_html_tags(&content);
            let text = text.trim();

            if !text.is_empty() {
                all_text.push_str(text);
                all_text.push_str("\n\n");
            }
        }
    }

    Ok(all_text)
}

/// Strip HTML tags from a string, preserving text content
#[cfg(feature = "document-processing")]
fn strip_html_tags(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut in_script = false;
    let mut in_style = false;

    let chars: Vec<char> = html.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        if c == '<' {
            in_tag = true;

            // Check for script/style start
            let remaining: String = chars[i..].iter().collect();
            let remaining_lower = remaining.to_lowercase();
            if remaining_lower.starts_with("<script") {
                in_script = true;
            } else if remaining_lower.starts_with("<style") {
                in_style = true;
            } else if remaining_lower.starts_with("</script") {
                in_script = false;
            } else if remaining_lower.starts_with("</style") {
                in_style = false;
            }

            // Add newlines for block elements
            if (remaining_lower.starts_with("<p")
                || remaining_lower.starts_with("<div")
                || remaining_lower.starts_with("<br")
                || remaining_lower.starts_with("<h1")
                || remaining_lower.starts_with("<h2")
                || remaining_lower.starts_with("<h3")
                || remaining_lower.starts_with("<h4")
                || remaining_lower.starts_with("<h5")
                || remaining_lower.starts_with("<h6")
                || remaining_lower.starts_with("<li"))
                && !result.ends_with('\n')
                && !result.is_empty()
            {
                result.push('\n');
            }
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag && !in_script && !in_style {
            // Handle HTML entities
            if c == '&' {
                let remaining: String = chars[i..].iter().take(10).collect();
                if remaining.starts_with("&nbsp;") {
                    result.push(' ');
                    i += 5;
                } else if remaining.starts_with("&amp;") {
                    result.push('&');
                    i += 4;
                } else if remaining.starts_with("&lt;") {
                    result.push('<');
                    i += 3;
                } else if remaining.starts_with("&gt;") {
                    result.push('>');
                    i += 3;
                } else if remaining.starts_with("&quot;") {
                    result.push('"');
                    i += 5;
                } else if remaining.starts_with("&apos;") {
                    result.push('\'');
                    i += 5;
                } else {
                    result.push(c);
                }
            } else {
                result.push(c);
            }
        }

        i += 1;
    }

    // Normalize whitespace
    let mut normalized = String::with_capacity(result.len());
    let mut last_was_whitespace = true;
    let mut consecutive_newlines = 0;

    for c in result.chars() {
        if c == '\n' {
            consecutive_newlines += 1;
            if consecutive_newlines <= 2 {
                normalized.push('\n');
            }
            last_was_whitespace = true;
        } else if c.is_whitespace() {
            consecutive_newlines = 0;
            if !last_was_whitespace {
                normalized.push(' ');
                last_was_whitespace = true;
            }
        } else {
            consecutive_newlines = 0;
            normalized.push(c);
            last_was_whitespace = false;
        }
    }

    normalized
}

/// Transcribe audio file to text using Whisper
///
/// Requires the `audio-transcription` feature.
/// Returns the transcribed text content.
#[cfg(feature = "audio-transcription")]
pub fn transcribe_audio_file(path: &Path) -> Result<String> {
    use crate::whisper::Transcriber;

    tracing::info!("Transcribing audio: {}", path.display());
    let mut transcriber = Transcriber::new()?;
    transcriber.transcribe_file(path)
}

/// Transcribe audio file with a reusable transcriber
///
/// More efficient for batch processing - avoids model reload for each file.
/// Requires the `audio-transcription` feature.
#[cfg(feature = "audio-transcription")]
pub fn transcribe_audio_file_with_transcriber(
    path: &Path,
    transcriber: &mut crate::whisper::Transcriber,
) -> Result<String> {
    tracing::debug!("Transcribing audio: {}", path.display());
    transcriber.transcribe_file(path)
}

/// Image embedding result from CLIP
#[cfg(feature = "clip")]
pub struct ImageEmbedding {
    /// The embedding vector (512 dimensions for CLIP ViT-B/32)
    pub data: Vec<f32>,
    /// Path to the source image
    pub path: std::path::PathBuf,
}

/// Embed an image file using CLIP
///
/// Requires the `clip` feature.
/// Returns a 512-dimensional embedding suitable for cross-modal search
/// (searching images with text queries).
///
/// Note: CLIP embeddings are single-vector (512 dim) and use cosine similarity,
/// which is different from XTR text embeddings (multi-vector, MaxSim scoring).
/// They require separate index storage for proper search functionality.
#[cfg(feature = "clip")]
pub fn embed_image_file(path: &Path) -> Result<ImageEmbedding> {
    use crate::embedder_clip::ClipEmbedder;
    use candle_core::Device;

    tracing::info!("Embedding image: {}", path.display());

    let device = Device::Cpu;
    let mut embedder = ClipEmbedder::new(&device)?;
    let result = embedder.embed_image_file(path)?;

    Ok(ImageEmbedding {
        data: result.data,
        path: path.to_path_buf(),
    })
}

/// Embed an image file with a reusable CLIP embedder
///
/// More efficient for batch processing - avoids model reload for each image.
/// Requires the `clip` feature.
#[cfg(feature = "clip")]
pub fn embed_image_file_with_embedder(
    path: &Path,
    embedder: &mut crate::embedder_clip::ClipEmbedder,
) -> Result<ImageEmbedding> {
    tracing::debug!("Embedding image: {}", path.display());
    let result = embedder.embed_image_file(path)?;

    Ok(ImageEmbedding {
        data: result.data,
        path: path.to_path_buf(),
    })
}

/// Read file content, extracting text from documents if needed
///
/// This function automatically detects whether a file is a plain text file
/// or a document that requires special extraction. For documents, it uses
/// the appropriate extraction method.
///
/// For markdown files, frontmatter metadata (YAML/TOML) is extracted and
/// prepended to improve search relevance.
///
/// For audio/video files (when `audio-transcription` feature is enabled),
/// transcription is performed using Whisper.
///
/// When the `document-processing` feature is disabled, document files are skipped.
#[cfg(feature = "document-processing")]
pub fn read_file_content(path: &Path) -> Result<Option<String>> {
    // Handle audio/video files with transcription
    #[cfg(feature = "audio-transcription")]
    if crate::file_types::is_media_file(path) {
        match transcribe_audio_file(path) {
            Ok(content) => {
                // Format transcription with file metadata
                let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("audio");
                let formatted = format!("# Transcription: {filename}\n\n{content}\n");
                return Ok(Some(formatted));
            }
            Err(e) => {
                tracing::debug!("Failed to transcribe {}: {}", path.display(), e);
                return Ok(None);
            }
        }
    }

    if crate::file_types::is_document_file(path) {
        match extract_document_text(path) {
            Ok(content) => Ok(Some(content)),
            Err(e) => {
                tracing::debug!("Failed to extract document {}: {}", path.display(), e);
                Ok(None)
            }
        }
    } else {
        // Plain text file - read directly
        match std::fs::read_to_string(path) {
            Ok(content) => {
                // Process markdown frontmatter if applicable
                if is_markdown_file(path) {
                    Ok(Some(process_markdown_frontmatter(&content)))
                } else {
                    Ok(Some(content))
                }
            }
            Err(e) => {
                tracing::debug!("Failed to read {}: {}", path.display(), e);
                Ok(None)
            }
        }
    }
}

/// Read file content without document processing
///
/// When the `document-processing` feature is disabled, this only reads plain text files.
/// Document files are skipped with a debug log message.
///
/// For markdown files, frontmatter metadata (YAML/TOML) is extracted and
/// prepended to improve search relevance.
///
/// For audio/video files (when `audio-transcription` feature is enabled),
/// transcription is performed using Whisper.
#[cfg(not(feature = "document-processing"))]
pub fn read_file_content(path: &Path) -> Result<Option<String>> {
    // Handle audio/video files with transcription
    #[cfg(feature = "audio-transcription")]
    if crate::file_types::is_media_file(path) {
        match transcribe_audio_file(path) {
            Ok(content) => {
                let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("audio");
                let formatted = format!("# Transcription: {}\n\n{}\n", filename, content);
                return Ok(Some(formatted));
            }
            Err(e) => {
                tracing::debug!("Failed to transcribe {}: {}", path.display(), e);
                return Ok(None);
            }
        }
    }

    if crate::file_types::is_document_file(path) {
        tracing::debug!(
            "Skipping document file {} (document-processing feature not enabled)",
            path.display()
        );
        Ok(None)
    } else {
        // Plain text file - read directly
        match std::fs::read_to_string(path) {
            Ok(content) => {
                // Process markdown frontmatter if applicable
                if is_markdown_file(path) {
                    Ok(Some(process_markdown_frontmatter(&content)))
                } else {
                    Ok(Some(content))
                }
            }
            Err(e) => {
                tracing::debug!("Failed to read {}: {}", path.display(), e);
                Ok(None)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_read_file_content_text() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.txt");

        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, "Hello, world!").unwrap();

        let content = read_file_content(&file_path).unwrap().unwrap();
        assert!(content.contains("Hello, world!"));
    }

    #[test]
    fn test_read_file_content_nonexistent() {
        let path = Path::new("/nonexistent/file.txt");
        let result = read_file_content(path).unwrap();
        assert!(result.is_none());
    }

    #[test]
    #[cfg(not(feature = "document-processing"))]
    fn test_read_file_content_pdf_without_feature() {
        // Without the feature, PDF files should be skipped
        let path = Path::new("/tmp/test.pdf");
        let result = read_file_content(path).unwrap();
        assert!(result.is_none());
    }

    // Frontmatter extraction tests

    #[test]
    fn test_yaml_frontmatter_basic() {
        let content = r"---
title: My Blog Post
author: John Doe
date: 2024-01-15
---

# Introduction

This is the main content.
";
        let fm = extract_frontmatter(content);
        assert_eq!(fm.title.as_deref(), Some("My Blog Post"));
        assert_eq!(fm.author.as_deref(), Some("John Doe"));
        assert_eq!(fm.date.as_deref(), Some("2024-01-15"));
        assert!(fm.content.contains("# Introduction"));
        assert!(fm.content.contains("This is the main content."));
        // Frontmatter should be stripped from content
        assert!(!fm.content.contains("title:"));
    }

    #[test]
    fn test_yaml_frontmatter_with_tags() {
        let content = r"---
title: Tagged Post
tags: [rust, programming, cli]
categories: [development]
---

Content here.
";
        let fm = extract_frontmatter(content);
        assert_eq!(fm.title.as_deref(), Some("Tagged Post"));
        assert_eq!(fm.tags, vec!["rust", "programming", "cli"]);
        assert_eq!(fm.categories, vec!["development"]);
    }

    #[test]
    fn test_yaml_frontmatter_with_description() {
        let content = r"---
title: API Documentation
description: Complete guide to our REST API
---

API docs here.
";
        let fm = extract_frontmatter(content);
        assert_eq!(
            fm.description.as_deref(),
            Some("Complete guide to our REST API")
        );
    }

    #[test]
    fn test_yaml_frontmatter_summary_as_description() {
        let content = r"---
title: API Documentation
summary: Summary used as description
---

API docs here.
";
        let fm = extract_frontmatter(content);
        // summary is an alias for description
        assert_eq!(
            fm.description.as_deref(),
            Some("Summary used as description")
        );
    }

    #[test]
    fn test_toml_frontmatter() {
        let content = r#"+++
title = "TOML Post"
author = "Jane Smith"
date = "2024-02-20"
tags = ["toml", "config"]
+++

TOML formatted frontmatter content.
"#;
        let fm = extract_frontmatter(content);
        assert_eq!(fm.title.as_deref(), Some("TOML Post"));
        assert_eq!(fm.author.as_deref(), Some("Jane Smith"));
        assert_eq!(fm.date.as_deref(), Some("2024-02-20"));
        assert_eq!(fm.tags, vec!["toml", "config"]);
        assert!(fm.content.contains("TOML formatted frontmatter content."));
    }

    #[test]
    fn test_no_frontmatter() {
        let content = r"# Regular Markdown

No frontmatter here, just content.
";
        let fm = extract_frontmatter(content);
        assert!(fm.title.is_none());
        assert!(fm.author.is_none());
        assert!(fm.content.contains("# Regular Markdown"));
    }

    #[test]
    fn test_frontmatter_with_quoted_values() {
        let content = r#"---
title: "Quoted Title"
author: 'Single Quoted'
---

Content.
"#;
        let fm = extract_frontmatter(content);
        assert_eq!(fm.title.as_deref(), Some("Quoted Title"));
        assert_eq!(fm.author.as_deref(), Some("Single Quoted"));
    }

    #[test]
    fn test_format_frontmatter_header() {
        let fm = Frontmatter {
            title: Some("Test Title".to_string()),
            description: Some("A test description".to_string()),
            author: Some("Test Author".to_string()),
            date: Some("2024-01-01".to_string()),
            tags: vec!["tag1".to_string(), "tag2".to_string()],
            categories: vec!["cat1".to_string()],
            content: String::new(),
        };

        let header = format_frontmatter_header(&fm);
        assert!(header.contains("# Test Title"));
        assert!(header.contains("A test description"));
        assert!(header.contains("**Author:** Test Author"));
        assert!(header.contains("**Date:** 2024-01-01"));
        assert!(header.contains("**Tags:** tag1, tag2"));
        assert!(header.contains("**Categories:** cat1"));
    }

    #[test]
    fn test_process_markdown_frontmatter() {
        let content = r"---
title: Processed Post
author: Claude
---

The actual content.
";
        let processed = process_markdown_frontmatter(content);
        assert!(processed.contains("# Processed Post"));
        assert!(processed.contains("**Author:** Claude"));
        assert!(processed.contains("The actual content."));
        // Original frontmatter delimiters should be gone
        assert!(!processed.contains("---"));
    }

    #[test]
    fn test_process_markdown_no_frontmatter() {
        let content = "# Just Content\n\nNo frontmatter.";
        let processed = process_markdown_frontmatter(content);
        assert_eq!(processed, content);
    }

    #[test]
    fn test_is_markdown_file() {
        assert!(is_markdown_file(Path::new("/tmp/README.md")));
        assert!(is_markdown_file(Path::new("/tmp/doc.markdown")));
        assert!(is_markdown_file(Path::new("/tmp/page.mdx")));
        assert!(is_markdown_file(Path::new("/tmp/FILE.MD"))); // Case insensitive
        assert!(!is_markdown_file(Path::new("/tmp/code.rs")));
        assert!(!is_markdown_file(Path::new("/tmp/config.json")));
    }

    #[test]
    fn test_yaml_frontmatter_alternate_keys() {
        // Test alternate key names (last matching key wins)
        let content = r"---
authors: Multiple Authors
created: 2024-03-01
category: single
excerpt: This is an excerpt
---

Content.
";
        let fm = extract_frontmatter(content);
        // excerpt should work for description
        assert_eq!(fm.description.as_deref(), Some("This is an excerpt"));
        // authors should work for author
        assert_eq!(fm.author.as_deref(), Some("Multiple Authors"));
        // created should work for date
        assert_eq!(fm.date.as_deref(), Some("2024-03-01"));
        // category (singular) should work for categories
        assert_eq!(fm.categories, vec!["single"]);
    }

    #[test]
    fn test_read_file_content_markdown_with_frontmatter() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.md");

        let content = r"---
title: Test Document
author: Test Author
---

# Main Content

Some text here.
";
        std::fs::write(&file_path, content).unwrap();

        let result = read_file_content(&file_path).unwrap().unwrap();
        // Should have formatted title as header
        assert!(result.contains("# Test Document"));
        // Should have author metadata
        assert!(result.contains("**Author:** Test Author"));
        // Should have the main content
        assert!(result.contains("# Main Content"));
        assert!(result.contains("Some text here."));
    }
}
