/// EPUB (Electronic Publication) format parser
///
/// Supports EPUB 2.0.1 and EPUB 3.x formats
/// Uses the `epub` crate for high-level parsing
use std::io::Read;
use std::path::Path;
use std::sync::LazyLock;

use epub::doc::EpubDoc;
use quick_xml::events::Event;
use quick_xml::Reader;
use regex::Regex;

use crate::error::{EbookError, Result};
use crate::types::{Chapter, EbookMetadata, PageTarget, ParsedEbook, TocEntry};

// =============================================================================
// Pre-compiled regex patterns using std::sync::LazyLock (Rust 1.80+)
// =============================================================================

// -- TOC label normalization patterns --
static RE_CHAPTER_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)CHAPTER\s*([IVXLCDM]+)\.?$").expect("valid chapter pattern"));
static RE_STANDALONE_CHAPTER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?i)CHAPTER\s*([IVXLCDM]+)\.?$").expect("valid standalone chapter pattern")
});
static RE_ROMAN_ONLY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([IVXLCDM]+)\.?$").expect("valid roman numeral pattern"));

// -- Image tag conversion patterns --
static RE_IMG_TAG: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"<img[^>]+src=["']([^"']+)["'][^>]*/?>"#).expect("valid img regex")
});
static RE_ALT_ATTR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"alt=["']([^"']*)["']"#).expect("valid alt regex"));

/// Parse an EPUB file from path
///
/// # Errors
///
/// Returns an error if:
/// - The EPUB file cannot be opened or is invalid
/// - Internal EPUB resources cannot be read
#[must_use = "this function returns a parsed ebook that should be processed"]
pub fn parse_epub<P: AsRef<Path>>(path: P) -> Result<ParsedEbook> {
    let path_str = path.as_ref().to_string_lossy().to_string();

    // Open EPUB using epub crate
    let mut doc = EpubDoc::new(&path_str)
        .map_err(|e| EbookError::EpubError(format!("Failed to open EPUB: {e}")))?;

    // Extract metadata
    let metadata = extract_metadata(&doc);

    // Extract table of contents
    let toc = extract_toc(&doc);

    // Extract page list from toc.ncx (if available)
    let page_list = extract_page_list(path.as_ref())?;

    // Extract chapters in spine order
    let chapters = extract_chapters(&mut doc);

    Ok(ParsedEbook {
        metadata,
        body_title: None, // EPUB doesn't have body title like FB2
        chapters,
        toc,
        page_list,
    })
}

/// Extract metadata from EPUB
fn extract_metadata(doc: &EpubDoc<std::io::BufReader<std::fs::File>>) -> EbookMetadata {
    let mut metadata = EbookMetadata::new();

    // Title
    metadata.title = doc.mdata("title").map(|m| m.value.clone());

    // Creators (authors)
    if let Some(creator) = doc.mdata("creator").map(|m| m.value.clone()) {
        metadata.creators.push(creator);
    }

    // Language
    metadata.language = doc.mdata("language").map(|m| m.value.clone());

    // Identifier
    metadata.identifier = doc.mdata("identifier").map(|m| m.value.clone());

    // Publisher
    metadata.publisher = doc.mdata("publisher").map(|m| m.value.clone());

    // Date
    metadata.date = doc.mdata("date").map(|m| m.value.clone());

    // Description
    metadata.description = doc.mdata("description").map(|m| m.value.clone());

    // Subject
    if let Some(subject) = doc.mdata("subject").map(|m| m.value.clone()) {
        metadata.subjects.push(subject);
    }

    // Rights
    metadata.rights = doc.mdata("rights").map(|m| m.value.clone());

    // Contributor
    if let Some(contributor) = doc.mdata("contributor").map(|m| m.value.clone()) {
        metadata.contributors.push(contributor);
    }

    metadata
}

/// Extract table of contents from EPUB
fn extract_toc(doc: &EpubDoc<std::io::BufReader<std::fs::File>>) -> Vec<TocEntry> {
    let mut toc = Vec::new();

    // Get table of contents from EPUB
    // The epub crate provides toc() method that returns Vec<NavPoint>
    for (spine_order, nav_point) in doc.toc.iter().enumerate() {
        let entry = extract_toc_entry(nav_point, Some(spine_order));
        toc.push(entry);
    }

    toc
}

/// Recursively extract TOC entry and its children from `NavPoint`
fn extract_toc_entry(nav_point: &epub::doc::NavPoint, play_order: Option<usize>) -> TocEntry {
    // Normalize TOC label for consistent formatting
    let normalized_label = normalize_toc_label(&nav_point.label);

    let mut entry = TocEntry {
        label: normalized_label,
        href: nav_point.content.to_string_lossy().to_string(),
        play_order,
        children: Vec::new(),
    };

    // Recursively extract nested children
    for child_nav_point in &nav_point.children {
        let child_entry = extract_toc_entry(child_nav_point, None);
        entry.children.push(child_entry);
    }

    entry
}

/// Extract chapters in reading order from EPUB
fn extract_chapters(doc: &mut EpubDoc<std::io::BufReader<std::fs::File>>) -> Vec<Chapter> {
    let mut chapters = Vec::new();

    // Reset to first chapter
    doc.set_current_chapter(0);

    let mut spine_order = 0;

    // Iterate through all chapters in spine order
    #[allow(
        clippy::while_let_loop,
        reason = "guard-based break not compatible with while-let"
    )]
    loop {
        // Get current chapter content
        let Some((content, _media_type)) = doc.get_current_str() else {
            break; // No more chapters
        };

        // Get current chapter path
        let href = doc.get_current_path().map_or_else(
            || format!("chapter_{spine_order}.xhtml"),
            |p| p.to_string_lossy().to_string(),
        );

        // Try to extract chapter title from content (first h1/h2 tag)
        let title = extract_title_from_html(&content);

        let chapter = Chapter {
            title,
            content,
            href,
            spine_order,
        };

        chapters.push(chapter);

        // Move to next chapter
        if !doc.go_next() {
            break;
        }

        spine_order += 1;
    }

    chapters
}

/// Extract title from HTML content (first h1 or h2 tag)
fn extract_title_from_html(html: &str) -> Option<String> {
    use scraper::{Html, Selector};

    let document = Html::parse_document(html);

    // Try h1 first
    if let Ok(h1_selector) = Selector::parse("h1") {
        if let Some(element) = document.select(&h1_selector).next() {
            let title = element
                .text()
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string();
            if !title.is_empty() {
                return Some(title);
            }
        }
    }

    // Try h2
    if let Ok(h2_selector) = Selector::parse("h2") {
        if let Some(element) = document.select(&h2_selector).next() {
            let title = element
                .text()
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string();
            if !title.is_empty() {
                return Some(title);
            }
        }
    }

    // Try title tag
    if let Ok(title_selector) = Selector::parse("title") {
        if let Some(element) = document.select(&title_selector).next() {
            let title = element
                .text()
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string();
            if !title.is_empty() {
                return Some(title);
            }
        }
    }

    None
}

/// Normalize TOC label for consistent formatting
///
/// Handles common issues in EPUB TOC labels:
/// - Extracts chapter numbers from mixed content (e.g., "Text. CHAPTER II." → "Chapter II")
/// - Normalizes capitalization (e.g., "CHAPTER IV" → "Chapter IV", "chapter i" → "Chapter I")
/// - Standardizes punctuation (e.g., "CHAPTER XIII" → "Chapter XIII", "Chapter XIII." → "Chapter XIII")
/// - Fixes spacing issues (e.g., "CHAPTERXXVII" → "Chapter XXVII")
/// - Normalizes title-like entries (e.g., "PRIDE. and PREJUDICE" → "Pride and Prejudice")
fn normalize_toc_label(label: &str) -> String {
    let trimmed = label.trim();

    // Pattern 1: Extract chapter number from mixed content
    // Examples: "Covering a screen. CHAPTER VIII." → "Chapter VIII"
    if let Some(caps) = RE_CHAPTER_PATTERN.captures(trimmed) {
        let chapter_num = &caps[1];
        return format!("Chapter {}", chapter_num.to_uppercase());
    }

    // Pattern 2: Standalone chapter number
    // Examples: "CHAPTER IV." → "Chapter IV", "CHAPTERXXVII" → "Chapter XXVII"
    if let Some(caps) = RE_STANDALONE_CHAPTER.captures(trimmed) {
        let chapter_num = &caps[1];
        return format!("Chapter {}", chapter_num.to_uppercase());
    }

    // Pattern 3: Just roman numerals
    // Examples: "I.", "XLVI." → "Chapter I", "Chapter XLVI"
    if let Some(caps) = RE_ROMAN_ONLY.captures(trimmed) {
        let chapter_num = &caps[1];
        // Only convert if it looks like a chapter number (I-LXX reasonable range)
        if is_reasonable_chapter_number(chapter_num) {
            return format!("Chapter {}", chapter_num.to_uppercase());
        }
    }

    // Pattern 4: All caps title (e.g., "PRIDE. and PREJUDICE" → "Pride and Prejudice")
    // Only apply to entries that are likely titles (not chapter numbers)
    // Precision loss acceptable: character counts are small (title text), well within f32 range
    #[allow(clippy::cast_precision_loss)]
    let uppercase_ratio = if trimmed.chars().filter(|c| c.is_alphabetic()).count() > 3 {
        trimmed.chars().filter(|c| c.is_uppercase()).count() as f32
            / trimmed.chars().filter(|c| c.is_alphabetic()).count() as f32
    } else {
        0.0
    };
    if uppercase_ratio > 0.7 {
        return normalize_title_case(trimmed);
    }

    // Default: return as-is with basic cleanup
    trimmed.to_string()
}

/// Check if a Roman numeral is in a reasonable range for chapter numbers (I-LXX / 1-70)
#[inline]
fn is_reasonable_chapter_number(roman: &str) -> bool {
    // Simple heuristic: Chapter numbers typically don't exceed LXX (70)
    // This avoids false positives on other Roman numeral uses
    let roman_upper = roman.to_uppercase();
    let reasonable_chapters = [
        "I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X", "XI", "XII", "XIII", "XIV",
        "XV", "XVI", "XVII", "XVIII", "XIX", "XX", "XXI", "XXII", "XXIII", "XXIV", "XXV", "XXVI",
        "XXVII", "XXVIII", "XXIX", "XXX", "XXXI", "XXXII", "XXXIII", "XXXIV", "XXXV", "XXXVI",
        "XXXVII", "XXXVIII", "XXXIX", "XL", "XLI", "XLII", "XLIII", "XLIV", "XLV", "XLVI", "XLVII",
        "XLVIII", "XLIX", "L", "LI", "LII", "LIII", "LIV", "LV", "LVI", "LVII", "LVIII", "LIX",
        "LX", "LXI", "LXII", "LXIII", "LXIV", "LXV", "LXVI", "LXVII", "LXVIII", "LXIX", "LXX",
    ];
    reasonable_chapters.contains(&roman_upper.as_str())
}

/// Normalize all-caps title to title case
/// Examples: "PRIDE. and PREJUDICE" → "Pride and Prejudice"
///           "THE FULL PROJECT GUTENBERG LICENSE" → "The Full Project Gutenberg License"
fn normalize_title_case(title: &str) -> String {
    let words: Vec<&str> = title.split_whitespace().collect();
    let mut result = Vec::new();

    for (i, word) in words.iter().enumerate() {
        // Remove trailing punctuation for processing
        // Don't add back punctuation in the middle of titles (e.g., "PRIDE. and" → "Pride and")
        let (clean_word, punct) = if word.ends_with('.') || word.ends_with(',') {
            let punct_str = &word[word.len() - 1..];
            // Only keep punctuation if it's the last word or looks like an abbreviation
            let keep_punct = i == words.len() - 1 || word.len() <= 3;
            (
                &word[..word.len() - 1],
                if keep_punct { punct_str } else { "" },
            )
        } else {
            (*word, "")
        };

        // Convert to title case
        // Keep small words lowercase unless they're first/last word
        let small_words = [
            "a", "an", "and", "as", "at", "but", "by", "for", "in", "of", "on", "or", "the", "to",
            "with",
        ];
        let lowercase = clean_word.to_lowercase();

        let converted =
            if i == 0 || i == words.len() - 1 || !small_words.contains(&lowercase.as_str()) {
                // Capitalize first letter, lowercase rest
                let mut chars = lowercase.chars();
                chars.next().map_or_else(String::new, |f| {
                    let mut capitalized = f.to_uppercase().collect::<String>();
                    capitalized.push_str(chars.as_str());
                    capitalized
                })
            } else {
                lowercase
            };

        result.push(format!("{converted}{punct}"));
    }

    result.join(" ")
}

/// Convert HTML content to plain text (for markdown output)
#[must_use = "converts HTML to plain text"]
pub fn html_to_text(html: &str) -> String {
    // Pre-process HTML to convert <img> tags to markdown image syntax
    // This preserves image references that would otherwise be stripped by html2text
    let html_with_images = convert_img_tags_to_markdown(html);

    // Use html2text for conversion
    html2text::from_read(html_with_images.as_bytes(), 80)
}

/// Convert HTML `<img>` tags to markdown image syntax
///
/// Extracts `src` and `alt` attributes from `<img>` tags and converts them to `![alt](src)` format.
/// This preserves image references during HTML-to-text conversion.
///
/// # Examples
/// ```ignore
/// // This is an internal function, not exported
/// let html = r#"<img src="cover.jpg" alt="Book Cover"/>"#;
/// let result = convert_img_tags_to_markdown(html);
/// assert!(result.contains("![Book Cover](cover.jpg)"));
/// ```
fn convert_img_tags_to_markdown(html: &str) -> String {
    RE_IMG_TAG
        .replace_all(html, |caps: &regex::Captures| {
            let src = &caps[1];

            // Try to extract alt attribute if present
            // caps.get(0) always exists for a match, but use as_str() on full match safely
            let full_match = caps.get(0).map_or("", |m| m.as_str());
            let alt = RE_ALT_ATTR
                .captures(full_match)
                .and_then(|c| c.get(1))
                .map_or("", |m| m.as_str());

            // Convert to markdown image syntax: ![alt](src)
            format!("![{alt}]({src})")
        })
        .to_string()
}

/// Normalize page label from EPUB pageList
///
/// Strips curly braces from page labels for cleaner display.
/// EPUB spec uses curly braces to indicate front matter page numbers (e.g., `{vii}` → `vii`)
///
/// # Examples
/// - `{vii}` → `vii` (front matter Roman numeral)
/// - `{ix}` → `ix`
/// - `1` → `1` (normal page number, unchanged)
/// - `A-1` → `A-1` (special page marker, unchanged)
#[inline]
fn normalize_page_label(label: &str) -> String {
    label.trim_matches(|c| c == '{' || c == '}').to_string()
}

/// Extract page list from EPUB toc.ncx file
///
/// EPUB 2 uses toc.ncx with two navigation structures:
/// 1. navMap - Table of contents (chapters) - handled by epub crate
/// 2. pageList - Page markers/illustrations - manually extracted here
///
/// Returns empty vector if no pageList found (not an error - some EPUBs don't have one)
#[allow(clippy::too_many_lines)] // Complex EPUB navigation parsing - keeping together for clarity
fn extract_page_list(epub_path: &Path) -> Result<Vec<PageTarget>> {
    use std::io::BufReader;

    // Open EPUB as ZIP archive
    let file = std::fs::File::open(epub_path)
        .map_err(|e| EbookError::EpubError(format!("Failed to open EPUB file: {e}")))?;
    let reader = BufReader::new(file);
    let mut archive = zip::ZipArchive::new(reader)
        .map_err(|e| EbookError::EpubError(format!("Invalid EPUB (not a ZIP): {e}")))?;

    // Find toc.ncx file (typically in OEBPS/ or similar)
    let ncx_entry_idx = (0..archive.len()).find(|&i| {
        archive
            .by_index(i)
            .is_ok_and(|file| file.name().ends_with("toc.ncx"))
    });

    let Some(idx) = ncx_entry_idx else {
        // No toc.ncx found - return empty list (EPUB 3 may not have one)
        return Ok(Vec::new());
    };

    // Read toc.ncx content
    let mut ncx_file = archive
        .by_index(idx)
        .map_err(|e| EbookError::EpubError(format!("Failed to read toc.ncx: {e}")))?;
    let mut ncx_content = String::new();
    ncx_file
        .read_to_string(&mut ncx_content)
        .map_err(|e| EbookError::EpubError(format!("Failed to read toc.ncx content: {e}")))?;

    // Parse XML to extract pageList
    let mut reader = Reader::from_str(&ncx_content);
    reader.trim_text(true);

    let mut page_list = Vec::new();
    let mut in_page_list = false;
    let mut in_page_target = false;
    let mut in_nav_label = false;

    let mut current_label = String::new();
    let mut current_href = String::new();
    let mut current_type: Option<String> = None;
    let mut current_play_order: Option<usize> = None;

    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e) | Event::Empty(e)) => {
                let name = e.local_name();
                match name.as_ref() {
                    b"pageList" => in_page_list = true,
                    b"pageTarget" if in_page_list => {
                        in_page_target = true;
                        // Extract attributes
                        for attr in e.attributes().flatten() {
                            match attr.key.as_ref() {
                                b"type" => {
                                    if let Ok(value) = attr.unescape_value() {
                                        current_type = Some(value.to_string());
                                    }
                                }
                                b"playOrder" => {
                                    if let Ok(value) = attr.unescape_value() {
                                        current_play_order = value.parse::<usize>().ok();
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    b"navLabel" if in_page_target => in_nav_label = true,
                    b"content" if in_page_target => {
                        // Extract src attribute
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"src" {
                                if let Ok(value) = attr.unescape_value() {
                                    current_href = value.to_string();
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(e)) => {
                // Capture text inside navLabel > text
                if in_nav_label {
                    if let Ok(text) = e.unescape() {
                        current_label = normalize_page_label(text.trim());
                    }
                }
            }
            Ok(Event::End(e)) => {
                let name = e.local_name();
                match name.as_ref() {
                    b"pageList" => {
                        in_page_list = false;
                    }
                    b"pageTarget" => {
                        // Save completed pageTarget
                        if !current_label.is_empty() && !current_href.is_empty() {
                            page_list.push(PageTarget {
                                label: current_label.clone(),
                                href: current_href.clone(),
                                page_type: current_type.clone(),
                                play_order: current_play_order,
                            });
                        }
                        // Reset state
                        in_page_target = false;
                        current_label.clear();
                        current_href.clear();
                        current_type = None;
                        current_play_order = None;
                    }
                    b"navLabel" => in_nav_label = false,
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                // XML parse error - return empty list rather than failing
                log::warn!("Failed to parse pageList from toc.ncx: {e}");
                return Ok(Vec::new());
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(page_list)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_title_from_html() {
        let html = r"<html><body><h1>Chapter One</h1><p>Content</p></body></html>";
        let title = extract_title_from_html(html);
        assert_eq!(title, Some("Chapter One".to_string()));
    }

    #[test]
    fn test_extract_title_h2() {
        let html = r"<html><body><h2>Introduction</h2><p>Content</p></body></html>";
        let title = extract_title_from_html(html);
        assert_eq!(title, Some("Introduction".to_string()));
    }

    #[test]
    fn test_extract_title_none() {
        let html = r"<html><body><p>Content without heading</p></body></html>";
        let title = extract_title_from_html(html);
        assert_eq!(title, None);
    }

    #[test]
    fn test_html_to_text() {
        let html = r"<p>Hello <strong>world</strong>!</p>";
        let text = html_to_text(html);
        assert!(text.contains("Hello"));
        assert!(text.contains("world"));
    }

    #[test]
    fn test_normalize_toc_label_mixed_content() {
        // Test extraction of chapter numbers from mixed content
        assert_eq!(
            normalize_toc_label("I hope Mr. Bingley will like it. CHAPTER II."),
            "Chapter II"
        );
        assert_eq!(
            normalize_toc_label("He rode a black horse. CHAPTER III."),
            "Chapter III"
        );
        assert_eq!(
            normalize_toc_label("Covering a screen. CHAPTER VIII."),
            "Chapter VIII"
        );
    }

    #[test]
    fn test_normalize_toc_label_standalone_chapters() {
        // Test standalone chapter numbers with various formats
        assert_eq!(normalize_toc_label("CHAPTER IV."), "Chapter IV");
        assert_eq!(normalize_toc_label("CHAPTER V"), "Chapter V");
        assert_eq!(normalize_toc_label("Chapter I."), "Chapter I");
        assert_eq!(normalize_toc_label("CHAPTER XIII"), "Chapter XIII");
        assert_eq!(normalize_toc_label("CHAPTER XIV"), "Chapter XIV");
        assert_eq!(normalize_toc_label("CHAPTERXXVII"), "Chapter XXVII");
        assert_eq!(normalize_toc_label("CHAPTERXXVIII"), "Chapter XXVIII");
    }

    #[test]
    fn test_normalize_toc_label_title_case() {
        // Test all-caps title normalization
        assert_eq!(
            normalize_toc_label("PRIDE. and PREJUDICE"),
            "Pride and Prejudice"
        );
        assert_eq!(
            normalize_toc_label("THE FULL PROJECT GUTENBERG LICENSE"),
            "The Full Project Gutenberg License"
        );
    }

    #[test]
    fn test_normalize_toc_label_preserves_normal() {
        // Test that normal labels are preserved
        assert_eq!(normalize_toc_label("Preface"), "Preface");
        assert_eq!(normalize_toc_label("Introduction"), "Introduction");
    }

    #[test]
    fn test_normalize_page_label() {
        // Test curly brace stripping for front matter
        assert_eq!(normalize_page_label("{vii}"), "vii");
        assert_eq!(normalize_page_label("{ix}"), "ix");
        assert_eq!(normalize_page_label("{x}"), "x");

        // Test normal page numbers unchanged
        assert_eq!(normalize_page_label("1"), "1");
        assert_eq!(normalize_page_label("324"), "324");

        // Test special markers unchanged
        assert_eq!(normalize_page_label("A-1"), "A-1");
        assert_eq!(normalize_page_label("Plate 5"), "Plate 5");
    }

    #[test]
    fn test_convert_img_tags_to_markdown() {
        // Test basic img tag with src and alt
        let html = r#"<img src="cover.jpg" alt="Book Cover"/>"#;
        let result = convert_img_tags_to_markdown(html);
        assert!(
            result.contains("![Book Cover](cover.jpg)"),
            "Result was: {result}"
        );

        // Test img tag with empty alt
        let html2 =
            r#"<img src="7808070301428223795_cover.jpg" alt="" class="x-ebookmaker-wrapper"/>"#;
        let result2 = convert_img_tags_to_markdown(html2);
        assert!(
            result2.contains("![](7808070301428223795_cover.jpg)"),
            "Result was: {result2}"
        );

        // Test multiple img tags
        let html3 = r#"<p>Text <img src="img1.png" alt="First"/> more text <img src="img2.jpg" alt="Second"/></p>"#;
        let result3 = convert_img_tags_to_markdown(html3);
        assert!(result3.contains("![First](img1.png)"));
        assert!(result3.contains("![Second](img2.jpg)"));

        // Test img tag with single quotes
        let html4 = r"<img src='image.gif' alt='Test'/>";
        let result4 = convert_img_tags_to_markdown(html4);
        assert!(result4.contains("![Test](image.gif)"));
    }
}
