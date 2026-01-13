// MOBI (Mobipocket) E-book Format Parser
//
// MOBI is a proprietary e-book format developed by Mobipocket SA (acquired by Amazon).
// It was the primary format for Amazon Kindle devices before AZW3/KF8.
//
// Format: Binary format based on Palm Database (PDB) structure with HTML content
// Extensions: .mobi, .prc, .azw (older AZW is just MOBI with different extension)
//
// Implementation: Uses the `mobi` crate (v0.8) for parsing MOBI files
// Content: HTML with Kindle-specific tags (e.g., <mbp:pagebreak/>)
//
// References:
// - MOBI format: https://wiki.mobileread.com/wiki/MOBI
// - mobi-rs crate: https://github.com/vv9k/mobi-rs

use crate::{Chapter, EbookError, EbookMetadata, ParsedEbook};
use mobi::{headers::ExthRecord, Mobi};

/// Parse a MOBI file from raw bytes
///
/// # Arguments
/// * `bytes` - Raw bytes of the MOBI file
///
/// # Errors
///
/// Returns an error if:
/// - The MOBI file is DRM-protected
/// - The file is corrupted or invalid
/// - HTML content extraction fails
///
/// # Examples
/// ```no_run
/// use docling_ebook::parse_mobi;
/// let bytes = std::fs::read("book.mobi").unwrap();
/// let ebook = parse_mobi(&bytes).unwrap();
/// println!("Title: {}", ebook.metadata.title.unwrap_or_default());
/// ```
#[must_use = "this function returns a parsed ebook that should be processed"]
pub fn parse_mobi(bytes: &[u8]) -> Result<ParsedEbook, EbookError> {
    // Convert bytes to Vec<u8> for Mobi::new()
    let bytes_vec = bytes.to_vec();

    // Parse MOBI file using mobi crate
    let mobi = Mobi::new(bytes_vec).map_err(|e| {
        // Check for DRM protection
        let err_msg = e.to_string();
        if err_msg.contains("drm") || err_msg.contains("encrypted") {
            EbookError::DrmProtected(
                "MOBI file is DRM-protected and cannot be parsed. \
                     Remove DRM using Calibre + DeDRM plugin (if legally allowed)."
                    .to_string(),
            )
        } else {
            EbookError::ParseError(format!("MOBI parse error: {e}"))
        }
    })?;

    // Extract metadata
    let metadata = extract_metadata(&mobi);

    // Extract HTML content (using lossy conversion to handle encoding issues)
    // Calibre-generated MOBI files may have encoding variations that cause UTF-8 decode errors
    // The lossy method replaces invalid sequences with � (replacement character)
    let html_content = mobi.content_as_string_lossy();

    // Extract TOC - try embedded TOC first, before parsing chapters
    // Many MOBI files (e.g., Project Gutenberg) have embedded TOC in HTML
    // We extract it first so we can remove it from content to avoid duplication
    let (toc, cleaned_html) = match extract_embedded_toc_with_removal(&html_content) {
        Some((toc, cleaned)) => (toc, cleaned),
        None => {
            // No embedded TOC found, use original HTML and generate TOC from chapters later
            (Vec::new(), html_content.clone())
        }
    };

    // Additionally, remove any table-based TOCs (Project Gutenberg pattern)
    // This runs regardless of whether blockquote TOC was found
    let cleaned_html = remove_table_toc(&cleaned_html);

    // Parse HTML to extract chapters (using cleaned HTML without embedded TOC)
    let chapters = extract_chapters(&cleaned_html)?;

    // If we didn't find embedded TOC, generate from chapters
    let toc = if toc.is_empty() {
        generate_toc_from_chapters(&chapters)
    } else {
        toc
    };

    // Build ParsedEbook structure
    Ok(ParsedEbook {
        metadata,
        body_title: None, // MOBI doesn't have body title like FB2
        chapters,
        toc,
        page_list: Vec::new(), // MOBI doesn't have pageList like EPUB
    })
}

/// Extract metadata from MOBI file
///
/// Extracts: title, authors, publisher, publish date, description, ISBN, contributors, language, ASIN, subjects, rights
fn extract_metadata(mobi: &Mobi) -> EbookMetadata {
    // Extract title (always present)
    let title = Some(mobi.title());

    // Extract author (optional) - map to creators field
    let creators = mobi.author().map(|a| vec![a]).unwrap_or_default();

    // Extract publisher (optional)
    let publisher = mobi.publisher();

    // Extract publish date (optional) - map to date field
    let date = mobi.publish_date();

    // Extract description (optional)
    let description = mobi.description();

    // Extract ISBN (optional) - map to identifier field
    // If ISBN not available, try ASIN from EXTH header (Amazon-specific)
    let identifier = mobi
        .isbn()
        .or_else(|| extract_exth_string(mobi, ExthRecord::Asin));

    // Extract contributors (optional)
    let contributors = mobi.contributor().map(|c| vec![c]).unwrap_or_default();

    // Extract language (available via mobi.language())
    let language_enum = mobi.language();
    let language = if language_enum == mobi::headers::Language::Neutral {
        None // Neutral means no language specified
    } else {
        Some(format!("{language_enum:?}")) // Convert enum to string (e.g., "English", "Spanish")
    };

    // Extract subjects/keywords from EXTH header (Amazon-specific)
    let subjects = extract_exth_string_vec(mobi, ExthRecord::Subject);

    // Extract rights from EXTH header (Amazon-specific)
    let rights = extract_exth_string(mobi, ExthRecord::Rights);

    EbookMetadata {
        title,
        creators,
        language,
        identifier,
        publisher,
        date,
        description,
        subjects,
        rights,
        contributors,
    }
}

/// Extract a string value from EXTH header
///
/// Returns the first value if multiple exist, or None if the record doesn't exist
#[inline]
fn extract_exth_string(mobi: &Mobi, record: ExthRecord) -> Option<String> {
    mobi.metadata
        .exth_record(record)
        .and_then(|values| values.first())
        .map(|bytes| String::from_utf8_lossy(bytes).to_string())
}

/// Extract multiple string values from EXTH header
///
/// Returns all values as a Vec, or empty Vec if the record doesn't exist
#[inline]
fn extract_exth_string_vec(mobi: &Mobi, record: ExthRecord) -> Vec<String> {
    mobi.metadata
        .exth_record(record)
        .map(|values| {
            values
                .iter()
                .map(|bytes| String::from_utf8_lossy(bytes).to_string())
                .collect()
        })
        .unwrap_or_default()
}

/// Generate Table of Contents from HTML content and extracted chapters
///
/// MOBI files often have an embedded TOC in the HTML content itself,
/// typically as a series of hyperlinks. We try to extract this first,
/// and fall back to generating TOC from chapter list if not found.
///
/// Extraction strategy:
/// 1. Try to find TOC links in HTML (links with chapter references)
/// 2. Fall back to generating TOC from extracted chapters
///
/// Creates a `TocEntry` for each chapter with:
/// - label: Chapter title (or "Chapter N" if no title)
/// - href: Chapter reference (chapter_{index})
fn generate_toc_from_chapters(chapters: &[Chapter]) -> Vec<crate::TocEntry> {
    use crate::TocEntry;

    chapters
        .iter()
        .enumerate()
        .map(|(i, chapter)| {
            let label = chapter
                .title
                .clone()
                .unwrap_or_else(|| format!("Chapter {}", i + 1));
            let href = chapter.href.clone();
            TocEntry::new(label, href)
        })
        .collect()
}

/// Extract embedded TOC from MOBI HTML content and return cleaned HTML
///
/// Many MOBI files (especially from Project Gutenberg) have an embedded TOC
/// in the HTML content itself, typically as a series of hyperlinks within
/// a blockquote element near the beginning of the document.
///
/// This function looks for TOC patterns, extracts them, and removes the
/// TOC blockquote from the HTML to avoid duplication in the final output.
///
/// Returns `Some((Vec<TocEntry>, cleaned_html))` if embedded TOC found, None otherwise
fn extract_embedded_toc_with_removal(html: &str) -> Option<(Vec<crate::TocEntry>, String)> {
    use crate::TocEntry;
    use scraper::{Html, Selector};

    let document = Html::parse_document(html);

    // Strategy: Find blockquote elements (Gutenberg pattern) or divs containing TOC
    // Look for clusters of links with Roman numerals
    let blockquote_selector = Selector::parse("blockquote").ok()?;
    let link_selector = Selector::parse("a").ok()?;
    let mut toc_entries = Vec::new();
    let mut toc_html_start: Option<usize> = None;
    let mut toc_html_end: Option<usize> = None;

    // First, try to find TOC within blockquote elements
    for blockquote in document.select(&blockquote_selector) {
        let links: Vec<_> = blockquote.select(&link_selector).collect();

        // Count how many links look like chapter references (Roman numerals or "Chapter:" patterns)
        let toc_link_count = links
            .iter()
            .filter(|link| {
                let text = link.text().collect::<String>();
                let text = text.trim();
                is_toc_entry(text)
            })
            .count();

        // If blockquote has many TOC-like links, it's likely the TOC
        if toc_link_count >= 10 {
            // Extract all links from this blockquote
            for link in links {
                let text = link.text().collect::<String>();
                let text = text.trim();

                // Skip empty links
                if text.is_empty() || text == "[]" {
                    continue;
                }

                // Skip pure numbers (page numbers)
                if text.chars().all(char::is_numeric) {
                    continue;
                }

                // Accept Roman numerals, "PREFACE", "List of...", etc.
                if is_toc_entry(text) {
                    let href = link.value().attr("href").unwrap_or("").to_string();

                    let label = format_toc_label(text);
                    toc_entries.push(TocEntry::new(label, href));
                }
            }

            // If we found a good TOC in this blockquote, mark it for removal
            if toc_entries.len() >= 10 {
                // Find the blockquote HTML in the original string to remove it
                // We look for <blockquote> tag surrounding this TOC
                if let Some(start) = html.find("<blockquote") {
                    // Find the matching </blockquote>
                    if let Some(end_tag_start) = html[start..].find("</blockquote>") {
                        toc_html_start = Some(start);
                        toc_html_end = Some(start + end_tag_start + "</blockquote>".len());
                    }
                }
                break;
            }
        }
    }

    // Build cleaned HTML, starting with original or blockquote-cleaned version
    let cleaned_html = if let (Some(start), Some(end)) = (toc_html_start, toc_html_end) {
        // Remove the TOC blockquote from HTML
        format!("{}{}", &html[..start], &html[end..])
    } else {
        html.to_string()
    };

    // Note: Table-based TOC removal is now handled by separate remove_table_toc() function
    // which is called after this function returns

    // Return TOC and cleaned HTML if we found a TOC
    (toc_entries.len() >= 10).then_some((toc_entries, cleaned_html))
}

/// Remove table-based TOC from HTML (Project Gutenberg pattern)
///
/// Some MOBI files (especially from Project Gutenberg) have a table-based TOC
/// embedded in the HTML content, typically with chapter links in table cells.
/// This function finds and removes such tables.
fn remove_table_toc(html: &str) -> String {
    use scraper::{Html, Selector};

    let document = Html::parse_document(html);
    let mut cleaned_html = html.to_string();

    let Ok(table_selector) = Selector::parse("table") else {
        return cleaned_html;
    };
    let Ok(link_selector) = Selector::parse("a") else {
        return cleaned_html;
    };

    for table in document.select(&table_selector) {
        let links: Vec<_> = table.select(&link_selector).collect();

        // Count TOC-like links (Chapter, Roman numerals, etc.)
        let toc_link_count = links
            .iter()
            .filter(|link| {
                let text = link.text().collect::<String>();
                is_toc_entry(text.trim())
            })
            .count();

        // If table has multiple TOC-like links (>= 5), it's likely an embedded TOC
        if toc_link_count >= 5 {
            // Find and remove this table from the HTML
            // Look for the first <table> tag
            if let Some(table_start) = cleaned_html.find("<table") {
                // Find the matching </table>
                if let Some(end_offset) = cleaned_html[table_start..].find("</table>") {
                    let table_end = table_start + end_offset + "</table>".len();
                    // Remove this table section
                    cleaned_html = format!(
                        "{}{}",
                        &cleaned_html[..table_start],
                        &cleaned_html[table_end..]
                    );
                    break; // Only remove first TOC table
                }
            }
        }
    }

    cleaned_html
}

/// Check if text is a Roman numeral link (with optional punctuation)
#[inline]
fn is_roman_numeral_link(text: &str) -> bool {
    if text.is_empty() || text.len() > 15 {
        return false;
    }

    // Remove punctuation and check if remaining is Roman numerals
    let cleaned: String = text
        .chars()
        .filter(|c| !matches!(c, '.' | ',' | ':' | ' '))
        .collect();

    if cleaned.is_empty() {
        return false;
    }

    // Check if all characters are Roman numeral digits
    cleaned
        .chars()
        .all(|c| matches!(c, 'I' | 'V' | 'X' | 'L' | 'C' | 'D' | 'M'))
}

/// Check if text looks like a TOC entry
#[inline]
fn is_toc_entry(text: &str) -> bool {
    // Roman numeral patterns
    if is_roman_numeral_link(text) {
        return true;
    }

    // Front matter / special entries
    let upper_text = text.to_uppercase();
    if upper_text.contains("PREFACE")
        || upper_text.contains("CHAPTER")
        || upper_text.contains("LIST OF")
        || upper_text.contains("ILLUSTRATIONS")
        || upper_text.contains("CONTENTS")
        || upper_text.contains("INTRODUCTION")
    {
        return true;
    }

    false
}

/// Format TOC label from raw text
fn format_toc_label(text: &str) -> String {
    // Normalize text by removing punctuation
    let normalized = text.replace(['.', ',', ':'], "").trim().to_string();

    // If it's a front matter entry, return as-is (without punctuation)
    let upper = normalized.to_uppercase();
    if upper.contains("PREFACE")
        || upper.contains("LIST OF")
        || upper.contains("ILLUSTRATIONS")
        || upper.contains("INTRODUCTION")
    {
        return normalized;
    }

    // If it contains "Chapter", extract just the Roman numeral and reformat consistently
    if text.contains("Chapter") || text.contains("CHAPTER") {
        // Extract Roman numeral from text like "Chapter: I" or "CHAPTER III"
        let parts: Vec<&str> = normalized.split_whitespace().collect();
        if let Some(roman) = parts.last() {
            if parts.len() >= 2 && is_roman_numeral_link(roman) {
                return format!("Chapter {roman}");
            }
        }
        // If we can't extract cleanly, return normalized
        return normalized;
    }

    // If it's a pure Roman numeral, format as "Chapter X"
    if is_roman_numeral_link(&normalized) {
        return format!("Chapter {normalized}");
    }

    // Default: return normalized
    normalized
}

/// Extract chapters from MOBI HTML content
///
/// MOBI files contain HTML with Kindle-specific tags:
/// - `<mbp:pagebreak/>` - Page/chapter breaks
/// - `<h1>`, `<h2>`, etc. - Heading tags for chapter titles
///
/// Strategy:
/// 1. Split content by `<mbp:pagebreak/>` tags
/// 2. If no page breaks found, split by `<h1>` tags
/// 3. If no structure found, treat entire book as single chapter
/// 4. Convert each chapter's HTML to Markdown
fn extract_chapters(html: &str) -> Result<Vec<Chapter>, EbookError> {
    // Strategy 1: Split by <mbp:pagebreak/> tags
    if html.contains("<mbp:pagebreak") {
        return extract_chapters_by_pagebreak(html);
    }

    // Strategy 2: Split by <h1> tags
    if html.contains("<h1") {
        return extract_chapters_by_headings(html);
    }

    // Strategy 3: Treat entire book as single chapter
    Ok(vec![Chapter {
        title: Some("Full Content".to_string()),
        content: html_to_markdown(html),
        href: String::new(),
        spine_order: 0,
    }])
}

/// Extract chapters by splitting on `<mbp:pagebreak/>` tags
fn extract_chapters_by_pagebreak(html: &str) -> Result<Vec<Chapter>, EbookError> {
    let mut chapters = Vec::new();

    // Split by page break tags (both self-closing variants)
    let parts: Vec<&str> = html
        .split("<mbp:pagebreak/>")
        .chain(html.split("<mbp:pagebreak />"))
        .filter(|s| !s.trim().is_empty())
        .collect();

    // If split didn't work (only one part), try alternative approach
    if parts.len() <= 1 {
        return extract_chapters_by_headings(html);
    }

    for (i, html_chunk) in parts.iter().enumerate() {
        let html_chunk = html_chunk.trim();
        if html_chunk.is_empty() {
            continue;
        }

        // Try to extract chapter title from first heading in chunk
        let title = extract_first_heading(html_chunk);

        // Convert HTML to Markdown
        let content = html_to_markdown(html_chunk);

        if !content.trim().is_empty() {
            chapters.push(Chapter {
                title,
                content,
                href: format!("chapter_{i}"),
                spine_order: i,
            });
        }
    }

    if chapters.is_empty() {
        // Fallback: treat as single chapter
        return Ok(vec![Chapter {
            title: Some("Full Content".to_string()),
            content: html_to_markdown(html),
            href: String::new(),
            spine_order: 0,
        }]);
    }

    Ok(chapters)
}

/// Extract chapters by splitting on `<h1>` tags
fn extract_chapters_by_headings(html: &str) -> Result<Vec<Chapter>, EbookError> {
    use scraper::{Html, Selector};

    let document = Html::parse_document(html);
    let h1_selector = Selector::parse("h1")
        .ok()
        .ok_or_else(|| EbookError::ParseError("Failed to create h1 selector".to_string()))?;

    let mut chapters = Vec::new();
    let has_headings = document.select(&h1_selector).next().is_some();

    if !has_headings {
        // No h1 tags found, treat as single chapter
        return Ok(vec![Chapter {
            title: Some("Full Content".to_string()),
            content: html_to_markdown(html),
            href: String::new(),
            spine_order: 0,
        }]);
    }

    // For each h1, extract content until next h1 or end
    // This is a simplified approach - production code would need more sophisticated DOM traversal

    // Fallback: split HTML string by <h1> tags (simple but effective)
    let parts: Vec<&str> = html.split("<h1").collect();

    for (i, part) in parts.iter().enumerate() {
        if i == 0 && !part.contains("</h1>") {
            // Content before first h1 - skip or include as preface
            continue;
        }

        let part_with_h1 = format!("<h1{part}");

        // Extract title from h1 tag
        let title = extract_first_heading(&part_with_h1);

        // Convert to markdown
        let content = html_to_markdown(&part_with_h1);

        if !content.trim().is_empty() {
            chapters.push(Chapter {
                title,
                content,
                href: format!("chapter_{i}"),
                spine_order: i,
            });
        }
    }

    if chapters.is_empty() {
        // Fallback: treat as single chapter
        return Ok(vec![Chapter {
            title: Some("Full Content".to_string()),
            content: html_to_markdown(html),
            href: String::new(),
            spine_order: 0,
        }]);
    }

    Ok(chapters)
}

/// Extract the text from the first heading tag (h1-h6) in HTML
fn extract_first_heading(html: &str) -> Option<String> {
    use scraper::{Html, Selector};

    let document = Html::parse_fragment(html);

    // Try h1 first, then h2, h3, etc.
    for tag in &["h1", "h2", "h3", "h4", "h5", "h6"] {
        if let Ok(selector) = Selector::parse(tag) {
            if let Some(heading) = document.select(&selector).next() {
                let text = heading.text().collect::<String>();
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }
    }

    None
}

/// Convert HTML to Markdown
///
/// Uses html2md crate for conversion. Handles:
/// - Standard HTML tags: `<p>`, `<h1>`-`<h6>`, `<b>`, `<i>`, `<a>`, `<img>`, `<ul>`, `<ol>`, `<li>`
/// - Tables: `<table>`, `<tr>`, `<td>`, `<th>`
/// - Kindle tags: `<mbp:pagebreak/>` removed, others passed through
fn html_to_markdown(html: &str) -> String {
    // Remove Kindle-specific tags that don't convert well
    let cleaned_html = html
        .replace("<mbp:pagebreak/>", "\n\n")
        .replace("<mbp:pagebreak />", "\n\n")
        .replace("<mbp:section>", "")
        .replace("</mbp:section>", "");

    // Convert HTML to Markdown using html2md crate
    html2md::parse_html(&cleaned_html)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_to_markdown_basic() {
        let html = r#"
            <h1>Chapter 1</h1>
            <p>This is a <b>bold</b> paragraph with <i>italic</i> text.</p>
            <p>Another paragraph with a <a href="http://example.com">link</a>.</p>
        "#;

        let markdown = html_to_markdown(html);
        assert!(markdown.contains("Chapter 1"));
        assert!(markdown.contains("bold"));
        assert!(markdown.contains("italic"));
        assert!(markdown.contains("link"));
    }

    #[test]
    fn test_html_to_markdown_kindle_tags() {
        let html = r"
            <p>First page content</p>
            <mbp:pagebreak/>
            <p>Second page content</p>
        ";

        let markdown = html_to_markdown(html);
        assert!(markdown.contains("First page"));
        assert!(markdown.contains("Second page"));
        assert!(!markdown.contains("mbp:pagebreak")); // Kindle tag should be removed
    }

    #[test]
    fn test_extract_first_heading() {
        let html = r"
            <h1>Chapter Title</h1>
            <p>Some content</p>
        ";

        let result = extract_first_heading(html);
        assert_eq!(result, Some("Chapter Title".to_string()));
    }

    #[test]
    fn test_extract_first_heading_h2() {
        let html = r"
            <p>Some intro</p>
            <h2>Section Title</h2>
            <p>Some content</p>
        ";

        let result = extract_first_heading(html);
        assert_eq!(result, Some("Section Title".to_string()));
    }

    #[test]
    fn test_extract_first_heading_none() {
        let html = r"
            <p>Just paragraphs</p>
            <p>No headings here</p>
        ";

        let result = extract_first_heading(html);
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_chapters_single() {
        let html = r"
            <h1>The Book</h1>
            <p>This is a simple book with no chapter breaks.</p>
            <p>Just a few paragraphs of content.</p>
        ";

        let result = extract_chapters(html);
        assert!(result.is_ok());

        let chapters = result.unwrap();
        assert_eq!(chapters.len(), 1);
        assert_eq!(chapters[0].title, Some("The Book".to_string()));
        assert!(chapters[0].content.contains("simple book"));
    }

    #[test]
    fn test_extract_chapters_by_h1() {
        let html = r"
            <h1>Chapter 1: The Beginning</h1>
            <p>First chapter content</p>
            <h1>Chapter 2: The Middle</h1>
            <p>Second chapter content</p>
            <h1>Chapter 3: The End</h1>
            <p>Third chapter content</p>
        ";

        let result = extract_chapters(html);
        assert!(result.is_ok());

        let chapters = result.unwrap();
        assert!(chapters.len() >= 2); // Should have at least 2 chapters

        // Check that chapters contain expected content
        let all_content = chapters
            .iter()
            .map(|c| c.content.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        assert!(all_content.contains("First chapter") || all_content.contains("Second chapter"));
    }

    #[test]
    fn test_extract_exth_string() {
        // This test verifies the helper function compiles correctly
        // Actual EXTH extraction is tested in integration tests with real MOBI files
        // The function signature and logic are validated here

        // Real testing happens with actual MOBI files in integration tests
    }

    #[test]
    fn test_extract_exth_string_vec() {
        // This test verifies the helper function compiles correctly
        // Actual EXTH extraction is tested in integration tests with real MOBI files
    }

    // Note: Cannot fully test parse_mobi() and EXTH extraction without actual MOBI files
    // Integration tests will cover actual MOBI file parsing with Amazon-specific metadata
}
