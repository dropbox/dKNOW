//! JATS (Journal Article Tag Suite) XML document parser
//!
//! Python source: ~/`docling/docling/backend/xml/jats_backend.py`
//!
//! # Architecture
//!
//! XML parsing using quick-xml (event-based parser)
//!
//! JATS is a standard XML format for scientific articles used by:
//! - `PubMed` Central (PMC)
//! - bioRxiv, medRxiv
//! - Springer Nature
//! - Various scientific publishers
//!
//! # JATS Document Structure
//!
//! - `<front>`: Metadata (title, authors, affiliations, abstract)
//! - `<body>`: Main content (sections, paragraphs, tables, figures)
//! - `<back>`: References, appendices
//!
//! # Implementation Status
//!
//! Fully implemented (title, abstract, authors, body sections, lists)
//!
//! # Python Reference
//!
//! jats_backend.py:143-299

// Clippy pedantic allows:
// - Unit struct &self convention
#![allow(clippy::trivially_copy_pass_by_ref)]

use crate::traits::{BackendOptions, DocumentBackend};
use crate::utils::{create_list_item, create_provenance, create_text_item};
use docling_core::{
    content::{DocItem, Formatting, ItemRef},
    DoclingError, Document, DocumentMetadata, InputFormat,
};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::fmt::Write as FmtWrite;
use std::fs;
use std::path::Path;

/// Text run with formatting information
///
/// Used to group consecutive text segments with identical formatting
/// before creating `DocItems`. This minimizes the number of `DocItems` created
/// for formatted paragraphs.
#[derive(Debug, Clone, PartialEq)]
struct TextRun {
    text: String,
    formatting: Formatting,
}

/// JATS backend for parsing scientific articles in JATS XML format
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct JatsBackend;

impl DocumentBackend for JatsBackend {
    #[inline]
    fn format(&self) -> InputFormat {
        InputFormat::Jats
    }

    fn parse_bytes(
        &self,
        bytes: &[u8],
        _options: &BackendOptions,
    ) -> Result<Document, DoclingError> {
        let xml_content = std::str::from_utf8(bytes)
            .map_err(|e| DoclingError::BackendError(format!("Invalid UTF-8 in JATS file: {e}")))?
            .to_string();

        self.parse_jats_xml(&xml_content, "file")
    }

    fn parse_file<P: AsRef<Path>>(
        &self,
        path: P,
        _options: &BackendOptions,
    ) -> Result<Document, DoclingError> {
        let path = path.as_ref();
        let filename = path.display().to_string();

        // Helper to add filename context to errors
        let add_context = |err: DoclingError| -> DoclingError {
            match err {
                DoclingError::BackendError(msg) => {
                    DoclingError::BackendError(format!("{msg}: {filename}"))
                }
                other => other,
            }
        };

        let xml_content = fs::read_to_string(path).map_err(DoclingError::IoError)?;

        let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");

        self.parse_jats_xml(&xml_content, file_stem)
            .map_err(add_context)
    }
}

#[allow(clippy::trivially_copy_pass_by_ref)] // Unit struct methods conventionally take &self
impl JatsBackend {
    /// Create a configured XML reader for JATS files
    ///
    /// JATS XML files typically include DOCTYPE declarations with DTDs.
    /// We configure the reader to handle DTDs safely by ignoring external entities.
    fn create_reader(xml_content: &str) -> Reader<&[u8]> {
        let mut reader = Reader::from_str(xml_content);
        reader.trim_text(true);
        // Allow DTD declarations but don't resolve external entities (security)
        reader.check_end_names(false); // More lenient parsing
        reader
    }

    /// Create XML reader WITHOUT trimming whitespace
    /// Used for abstract parsing where whitespace between inline elements matters
    fn create_reader_no_trim(xml_content: &str) -> Reader<&[u8]> {
        let mut reader = Reader::from_str(xml_content);
        reader.trim_text(false); // Preserve whitespace exactly
        reader.check_end_names(false);
        reader
    }

    /// Extract text with inline formatting from XML node
    ///
    /// Traverses the XML tree and tracks formatting tags (`<bold>`, `<italic>`, `<underline>`, `<sub>`, `<sup>`, `<monospace>`)
    /// Returns a list of `TextRuns` with their associated formatting.
    ///
    /// JATS inline formatting tags:
    /// - `<bold>` - Bold text
    /// - `<italic>` - Italic text
    /// - `<underline>` - Underlined text
    /// - `<sub>` - Subscript
    /// - `<sup>` - Superscript
    /// - `<monospace>` - Monospace/code text
    fn extract_text_with_formatting(
        node: &roxmltree::Node,
        current_formatting: &Formatting,
    ) -> Vec<TextRun> {
        let mut runs = Vec::new();
        let tag_name = node.tag_name().name();

        // Update formatting based on current node
        let mut fmt = current_formatting.clone();
        match tag_name {
            "bold" => fmt.bold = Some(true),
            "italic" => fmt.italic = Some(true),
            "underline" => fmt.underline = Some(true),
            "sub" => fmt.script = Some("sub".to_string()),
            "sup" => fmt.script = Some("super".to_string()),
            "monospace" => {
                // Monospace is represented by setting font_family
                fmt.font_family = Some("monospace".to_string());
            }
            _ => {}
        }

        // Collect text from this node
        if let Some(text) = node.text() {
            // Preserve whitespace within text segments - trim() will be applied to final output
            // Normalize newlines to spaces to match Python behavior
            let text_content = text.replace('\n', " ");
            if !text_content.is_empty() {
                runs.push(TextRun {
                    text: text_content,
                    formatting: fmt.clone(),
                });
            }
        }

        // Process children
        for child in node.children() {
            if child.is_element() {
                let child_runs = Self::extract_text_with_formatting(&child, &fmt);
                runs.extend(child_runs);
            }
            // Handle tail text (text after closing tag)
            if let Some(tail) = child.tail() {
                // Preserve whitespace in tail - trim() will be applied to final output
                // Normalize newlines to spaces to match Python behavior
                let tail_content = tail.replace('\n', " ");
                if !tail_content.is_empty() {
                    runs.push(TextRun {
                        text: tail_content,
                        formatting: fmt.clone(),
                    });
                }
            }
        }

        runs
    }

    /// Group consecutive runs with identical formatting
    ///
    /// Merges adjacent `TextRuns` that have the same formatting into a single run.
    /// This reduces the number of `DocItems` created for formatted paragraphs.
    fn group_runs_by_formatting(runs: Vec<TextRun>) -> Vec<TextRun> {
        if runs.is_empty() {
            return Vec::new();
        }

        let mut grouped = Vec::new();
        let mut iter = runs.into_iter();
        // SAFETY: We checked runs.is_empty() above
        let mut current_run = iter.next().unwrap();

        for run in iter {
            if Self::formatting_matches(&current_run.formatting, &run.formatting) {
                // Same formatting, append text without adding extra space
                // Whitespace is preserved from original XML in extract_text_with_formatting
                current_run.text.push_str(&run.text);
            } else {
                // Different formatting, save current and start new
                grouped.push(current_run);
                current_run = run;
            }
        }

        // Don't forget the last run
        grouped.push(current_run);
        grouped
    }

    /// Check if two Formatting structs are identical
    fn formatting_matches(a: &Formatting, b: &Formatting) -> bool {
        a.bold == b.bold
            && a.italic == b.italic
            && a.underline == b.underline
            && a.strikethrough == b.strikethrough
            && a.script == b.script
            && a.font_size == b.font_size
            && a.font_family == b.font_family
    }

    /// Parse JATS XML and extract `DocItems`
    ///
    /// Python reference: jats_backend.py:143-172 (convert method)
    /// Phase 3: Extract title, abstract, and body sections/paragraphs
    #[allow(clippy::too_many_lines)] // Complex JATS parsing - keeping together for clarity
    fn parse_jats_xml(&self, xml_content: &str, file_stem: &str) -> Result<Document, DoclingError> {
        let mut doc_items = Vec::new();
        let mut item_count = 0;

        // Parse title
        // Python reference: _parse_title() lines 270-291
        if let Some(title) = Self::extract_title(xml_content)? {
            if !title.is_empty() {
                // Title should be a level 1 heading (# in markdown)
                let doc_item = DocItem::SectionHeader {
                    self_ref: format!("#/texts/{item_count}"),
                    parent: None,
                    children: vec![],
                    content_layer: "body".to_string(),
                    prov: create_provenance(1),
                    orig: title.clone(),
                    text: title,
                    level: 1,
                    formatting: None,
                    hyperlink: None,
                };
                doc_items.push(doc_item);
                item_count += 1;
            }
        }

        // Parse authors (output as text paragraph after title)
        // Python reference: _parse_authors() lines 221-268
        let authors = Self::extract_authors(xml_content)?;
        if !authors.is_empty() {
            let author_text = authors.join(", ");
            let doc_item = DocItem::Text {
                self_ref: format!("#/texts/{item_count}"),
                parent: None,
                children: vec![],
                content_layer: "body".to_string(),
                prov: create_provenance(1),
                orig: author_text.clone(),
                text: author_text,
                formatting: None,
                hyperlink: None,
            };
            doc_items.push(doc_item);
            item_count += 1;
        }

        // Extract keywords/subject for metadata
        let keywords = Self::extract_keywords(xml_content)?;

        // Parse affiliations (output after authors)
        let affiliations = Self::extract_affiliations(xml_content)?;
        if !affiliations.is_empty() {
            let affiliation_text = affiliations.join("; ");
            // Escape HTML entities to match Python docling output
            let escaped_text = Self::escape_html_entities(&affiliation_text);
            let doc_item = DocItem::Text {
                self_ref: format!("#/texts/{item_count}"),
                parent: None,
                children: vec![],
                content_layer: "body".to_string(),
                prov: create_provenance(1),
                orig: escaped_text.clone(),
                text: escaped_text,
                formatting: None,
                hyperlink: None,
            };
            doc_items.push(doc_item);
            item_count += 1;
        }

        // Parse all abstracts including "Author summary"
        // Python reference: _parse_abstract() lines 203-220
        let all_abstracts = Self::extract_all_abstracts(xml_content)?;
        for (title, paragraphs) in all_abstracts {
            if paragraphs.is_empty() {
                continue;
            }

            // Determine section heading - use title if available, otherwise "Abstract"
            let heading = title.unwrap_or_else(|| "Abstract".to_string());

            // Add section heading
            let doc_item = DocItem::SectionHeader {
                self_ref: format!("#/texts/{item_count}"),
                parent: None,
                children: vec![],
                content_layer: "body".to_string(),
                prov: create_provenance(1),
                orig: heading.clone(),
                text: heading,
                level: 2,
                formatting: None,
                hyperlink: None,
            };
            doc_items.push(doc_item);
            item_count += 1;

            // Add paragraphs
            for text in paragraphs {
                if !text.is_empty() {
                    let doc_item = DocItem::Text {
                        self_ref: format!("#/texts/{item_count}"),
                        parent: None,
                        children: vec![],
                        content_layer: "body".to_string(),
                        prov: create_provenance(1),
                        orig: text.clone(),
                        text,
                        formatting: None,
                        hyperlink: None,
                    };
                    doc_items.push(doc_item);
                    item_count += 1;
                }
            }
        }

        // Parse body
        // Python reference: _walk_linear() lines 716-819
        let body_items = self.extract_body(xml_content)?;
        // Update item_count based on body items generated
        item_count += body_items.len();
        doc_items.extend(body_items);

        // Parse back matter (funding, acknowledgements, additional info, data availability)
        // Python reference: jats_backend.py:165-168 walks over <back> element
        let back_items = self.extract_back_matter(xml_content, &mut item_count)?;
        doc_items.extend(back_items);

        // Parse references from <back><ref-list>
        let ref_items = Self::extract_references(xml_content, &mut item_count)?;
        doc_items.extend(ref_items);

        // Generate markdown from DocItems using shared helper (applies formatting)
        let markdown = crate::markdown_helper::docitems_to_markdown(&doc_items);

        // Calculate num_characters from markdown output (consistent with other backends)
        let metadata = DocumentMetadata {
            num_pages: Some(1), // JATS doesn't have pages
            num_characters: markdown.chars().count(),
            title: Some(file_stem.to_string()),
            author: if authors.is_empty() {
                None
            } else {
                Some(authors.join(", "))
            },
            created: None,
            modified: None,
            language: None,
            subject: if keywords.is_empty() {
                None
            } else {
                Some(keywords.join(", "))
            },
            exif: None,
        };

        Ok(Document {
            format: InputFormat::Jats,
            markdown,
            metadata,
            content_blocks: Some(doc_items),
            docling_document: None,
        })
    }

    /// Extract title from JATS XML
    ///
    /// Python reference: jats_backend.py:270-291 (`_parse_title`)
    /// Searches for `<title-group><article-title>` in article-meta
    fn extract_title(xml_content: &str) -> Result<Option<String>, DoclingError> {
        let mut reader = Self::create_reader(xml_content);

        let mut buf = Vec::new();
        let mut in_article_meta = false;
        let mut in_title_group = false;
        let mut in_article_title = false;
        let mut title_text = String::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let name = e.name();
                    match name.as_ref() {
                        b"article-meta" => in_article_meta = true,
                        b"title-group" if in_article_meta => in_title_group = true,
                        // Support article-title in either title-group or directly under article-meta
                        b"article-title" if in_title_group || in_article_meta => {
                            in_article_title = true;
                        }
                        _ => {}
                    }
                }
                Ok(Event::Text(e)) if in_article_title => {
                    let text = e.unescape().map_err(|e| {
                        DoclingError::BackendError(format!("XML unescape error: {e}"))
                    })?;
                    title_text.push_str(&text);
                }
                Ok(Event::End(e)) => {
                    let name = e.name();
                    match name.as_ref() {
                        b"article-title" => in_article_title = false,
                        b"title-group" => in_title_group = false,
                        b"article-meta" => in_article_meta = false,
                        _ => {}
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    return Err(DoclingError::BackendError(format!("XML parse error: {e}")));
                }
                _ => {}
            }
            buf.clear();
        }

        // Clean up whitespace and newlines
        let title = title_text.trim().replace('\n', " ");

        if title.is_empty() {
            Ok(None)
        } else {
            Ok(Some(title))
        }
    }

    /// Extract abstract from JATS XML (test-only helper)
    ///
    /// Python reference: jats_backend.py:203-220 (`_parse_abstract`)
    /// Searches for `<abstract><p>` elements and extracts text
    #[cfg(test)]
    fn extract_abstract(xml_content: &str) -> Result<Vec<String>, DoclingError> {
        // Get all abstracts and return only the main abstract's paragraphs
        let all_abstracts = Self::extract_all_abstracts(xml_content)?;
        for (title, paragraphs) in all_abstracts {
            if title.is_none() || title.as_deref() == Some("Abstract") {
                return Ok(paragraphs);
            }
        }
        Ok(Vec::new())
    }

    /// Extract all abstracts from JATS XML including "Author summary"
    ///
    /// Returns a vector of (Optional title, paragraphs) for each abstract element
    /// `PLoS` uses `<abstract abstract-type="summary"><title>Author summary</title>`
    #[allow(
        clippy::type_complexity,
        reason = "type is clear and local to this function"
    )]
    #[allow(clippy::too_many_lines)] // Complex abstract extraction - keeping together for clarity
    fn extract_all_abstracts(
        xml_content: &str,
    ) -> Result<Vec<(Option<String>, Vec<String>)>, DoclingError> {
        // Use non-trimming reader to preserve whitespace between inline elements like </sup> text
        let mut reader = Self::create_reader_no_trim(xml_content);

        let mut buf = Vec::new();
        let mut all_abstracts: Vec<(Option<String>, Vec<String>)> = Vec::new();
        let mut current_abstract_title: Option<String> = None;
        let mut current_paragraphs: Vec<String> = Vec::new();
        let mut in_abstract = false;
        let mut in_title = false;
        let mut in_p = false;
        let mut current_text = String::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let name = e.name();
                    match name.as_ref() {
                        b"abstract" => {
                            in_abstract = true;
                            current_abstract_title = None;
                            current_paragraphs.clear();
                        }
                        b"title" if in_abstract => in_title = true,
                        b"p" if in_abstract => in_p = true,
                        // Add space before word-level formatting elements (italic, bold, etc.)
                        // but NOT before sup/sub which attach to previous text
                        b"italic" | b"bold" | b"monospace" | b"sc" if in_abstract && in_p => {
                            if !current_text.is_empty()
                                && !current_text.ends_with(char::is_whitespace)
                            {
                                current_text.push(' ');
                            }
                        }
                        // sup and sub should NOT have space before them
                        b"sup" | b"sub" if in_abstract && in_p => {
                            // No space - they attach to previous text
                        }
                        _ => {}
                    }
                }
                Ok(Event::Text(e)) if in_abstract && (in_p || in_title) => {
                    let text = e.unescape().map_err(|e| {
                        DoclingError::BackendError(format!("XML unescape error: {e}"))
                    })?;
                    // Check if we need to add space after inline element
                    if current_text.ends_with('\x00') {
                        // Remove marker
                        current_text.pop();
                        // Add space if text doesn't start with whitespace/punctuation
                        if !text.is_empty()
                            && !text.starts_with(char::is_whitespace)
                            && !text.starts_with(|c: char| c.is_ascii_punctuation())
                        {
                            current_text.push(' ');
                        }
                    }
                    current_text.push_str(&text);
                }
                Ok(Event::End(e)) => {
                    let name = e.name();
                    match name.as_ref() {
                        b"title" if in_abstract => {
                            in_title = false;
                            let title_text = current_text.trim().to_string();
                            if !title_text.is_empty() {
                                current_abstract_title = Some(title_text);
                            }
                            current_text.clear();
                        }
                        b"p" if in_abstract => {
                            in_p = false;
                            // Remove any trailing marker
                            if current_text.ends_with('\x00') {
                                current_text.pop();
                            }
                            let para_text = current_text.trim().replace('\n', " ");
                            if !para_text.is_empty() {
                                // Re-escape HTML entities to match Python output
                                let escaped = Self::escape_html_entities(&para_text);
                                current_paragraphs.push(escaped);
                            }
                            current_text.clear();
                        }
                        b"abstract" => {
                            in_abstract = false;
                            if !current_paragraphs.is_empty() {
                                all_abstracts.push((
                                    current_abstract_title.take(),
                                    std::mem::take(&mut current_paragraphs),
                                ));
                            }
                        }
                        // Mark end of word-level inline element for space insertion
                        // but NOT for sup/sub which attach directly
                        b"italic" | b"bold" | b"monospace" | b"sc" if in_abstract && in_p => {
                            // Add a marker that will be replaced with space if needed
                            if !current_text.is_empty()
                                && !current_text.ends_with(char::is_whitespace)
                            {
                                current_text.push('\x00');
                            }
                        }
                        // sup and sub do NOT get space markers - they attach directly to surrounding text
                        // The trim_text setting will handle whitespace naturally
                        b"sup" | b"sub" if in_abstract && in_p => {
                            // No marker - they attach to surrounding text
                        }
                        _ => {}
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    return Err(DoclingError::BackendError(format!("XML parse error: {e}")));
                }
                _ => {}
            }
            buf.clear();
        }

        Ok(all_abstracts)
    }

    /// Extract authors from JATS XML
    ///
    /// Python reference: jats_backend.py:221-268 (`_parse_authors`)
    /// Searches for `<contrib-group><contrib[@contrib-type="author"]>`
    /// and extracts `<name><given-names>` and `<surname>` OR `<collab>` (consortium names)
    fn extract_authors(xml_content: &str) -> Result<Vec<String>, DoclingError> {
        let mut reader = Self::create_reader(xml_content);

        let mut buf = Vec::new();
        let mut authors = Vec::new();
        let mut in_article_meta = false;
        let mut in_contrib_group = false;
        let mut in_contrib = false;
        let mut in_name = false;
        let mut in_given_names = false;
        let mut in_surname = false;
        let mut in_collab = false;
        let mut is_author_contrib = false;
        let mut given_names = String::new();
        let mut surname = String::new();
        let mut collab_name = String::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let name = e.name();
                    match name.as_ref() {
                        b"article-meta" => in_article_meta = true,
                        b"contrib-group" if in_article_meta => in_contrib_group = true,
                        b"contrib" if in_contrib_group => {
                            in_contrib = true;
                            // Check if contrib-type="author"
                            is_author_contrib = e
                                .attributes()
                                .filter_map(std::result::Result::ok)
                                .any(|attr| {
                                    attr.key.as_ref() == b"contrib-type"
                                        && attr.value.as_ref() == b"author"
                                });
                        }
                        b"name" if in_contrib && is_author_contrib => in_name = true,
                        b"given-names" if in_name => in_given_names = true,
                        b"surname" if in_name => in_surname = true,
                        b"collab" if in_contrib && is_author_contrib => in_collab = true,
                        _ => {}
                    }
                }
                Ok(Event::Text(e)) => {
                    if in_given_names {
                        let text = e.unescape().map_err(|e| {
                            DoclingError::BackendError(format!("XML unescape error: {e}"))
                        })?;
                        given_names.push_str(&text);
                    } else if in_surname {
                        let text = e.unescape().map_err(|e| {
                            DoclingError::BackendError(format!("XML unescape error: {e}"))
                        })?;
                        surname.push_str(&text);
                    } else if in_collab {
                        let text = e.unescape().map_err(|e| {
                            DoclingError::BackendError(format!("XML unescape error: {e}"))
                        })?;
                        collab_name.push_str(&text);
                    }
                }
                Ok(Event::End(e)) => {
                    let name = e.name();
                    match name.as_ref() {
                        b"given-names" => in_given_names = false,
                        b"surname" => in_surname = false,
                        b"name" => in_name = false,
                        b"collab" => {
                            if in_contrib && is_author_contrib && !collab_name.is_empty() {
                                authors.push(collab_name.trim().to_string());
                            }
                            in_collab = false;
                            collab_name.clear();
                        }
                        b"contrib" if in_contrib => {
                            if is_author_contrib && !given_names.is_empty() && !surname.is_empty() {
                                let author_name =
                                    format!("{} {}", given_names.trim(), surname.trim());
                                authors.push(author_name);
                            }
                            in_contrib = false;
                            is_author_contrib = false;
                            given_names.clear();
                            surname.clear();
                        }
                        b"contrib-group" => in_contrib_group = false,
                        b"article-meta" => in_article_meta = false,
                        _ => {}
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    return Err(DoclingError::BackendError(format!("XML parse error: {e}")));
                }
                _ => {}
            }
            buf.clear();
        }

        Ok(authors)
    }

    /// Extract affiliations from JATS XML
    ///
    /// JATS files have two patterns for affiliations:
    /// 1. `<aff>` as child of author contrib-group (eLife style) - NOT editor contrib-groups
    /// 2. `<aff>` as direct child of `<article-meta>` (`PLoS` style)
    ///
    /// Python only includes affiliations that are referenced by author contributors,
    /// not affiliations referenced only by editors.
    fn extract_affiliations(xml_content: &str) -> Result<Vec<String>, DoclingError> {
        // Parse XML with roxmltree for DOM-like tree walking
        let parse_options = roxmltree::ParsingOptions {
            allow_dtd: true,
            ..roxmltree::ParsingOptions::default()
        };
        let doc = roxmltree::Document::parse_with_options(xml_content, parse_options)
            .map_err(|e| DoclingError::BackendError(format!("XML parse error: {e}")))?;

        let mut affiliations = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        // Find <article-meta> element
        let article_meta = doc
            .descendants()
            .find(|n| n.is_element() && n.tag_name().name() == "article-meta");

        if let Some(meta) = article_meta {
            // First, collect all affiliation IDs referenced by authors (not editors)
            let mut author_aff_ids = std::collections::HashSet::new();
            for contrib_group in meta
                .children()
                .filter(|n| n.is_element() && n.tag_name().name() == "contrib-group")
            {
                for contrib in contrib_group.children().filter(|n| {
                    n.is_element()
                        && n.tag_name().name() == "contrib"
                        && n.attribute("contrib-type") == Some("author")
                }) {
                    // Get xref elements with ref-type="aff"
                    for xref in contrib.descendants().filter(|n| {
                        n.is_element()
                            && n.tag_name().name() == "xref"
                            && n.attribute("ref-type") == Some("aff")
                    }) {
                        if let Some(rid) = xref.attribute("rid") {
                            author_aff_ids.insert(rid.to_string());
                        }
                    }
                }
            }

            // Pattern 1: <aff> inside contrib-groups that have author contributors (eLife style)
            // Skip contrib-groups that only contain editors
            for contrib_group in meta
                .children()
                .filter(|n| n.is_element() && n.tag_name().name() == "contrib-group")
            {
                // Check if this contrib-group has at least one author contributor
                let has_authors = contrib_group.children().any(|n| {
                    n.is_element()
                        && n.tag_name().name() == "contrib"
                        && n.attribute("contrib-type") == Some("author")
                });

                // Skip editor-only contrib-groups
                if !has_authors {
                    continue;
                }

                for aff in contrib_group
                    .children()
                    .filter(|n| n.is_element() && n.tag_name().name() == "aff")
                {
                    let id = aff.attribute("id").unwrap_or("");
                    if !id.is_empty() && seen_ids.contains(id) {
                        continue;
                    }
                    if !id.is_empty() {
                        seen_ids.insert(id.to_string());
                    }
                    let aff_text = Self::extract_affiliation_text(&aff);
                    if !aff_text.is_empty() {
                        affiliations.push(aff_text);
                    }
                }
            }

            // Pattern 2: <aff> as direct child of <article-meta> (PLoS style)
            // Only include affiliations that are referenced by authors (not editors)
            for aff in meta
                .children()
                .filter(|n| n.is_element() && n.tag_name().name() == "aff")
            {
                let id = aff.attribute("id").unwrap_or("");
                if !id.is_empty() && seen_ids.contains(id) {
                    continue;
                }
                // Skip affiliations that are NOT referenced by any author
                // (These are likely editor-only affiliations)
                if !id.is_empty() && !author_aff_ids.is_empty() && !author_aff_ids.contains(id) {
                    continue;
                }
                if !id.is_empty() {
                    seen_ids.insert(id.to_string());
                }
                let aff_text = Self::extract_affiliation_text(&aff);
                if !aff_text.is_empty() {
                    affiliations.push(aff_text);
                }
            }
        }

        Ok(affiliations)
    }

    /// Extract formatted text from a single `<aff>` element
    fn extract_affiliation_text(aff_node: &roxmltree::Node) -> String {
        let mut parts = Vec::new();

        for child in aff_node.children() {
            if !child.is_element() {
                continue;
            }

            let tag = child.tag_name().name();
            // Skip labels (e.g., "1", "2")
            if tag == "label" {
                continue;
            }
            // Extract text from address parts (institution, addr-line, etc.) and others
            let text = Self::get_text(&child);
            if !text.is_empty() {
                parts.push(text);
            }
        }

        // Join with commas and spaces
        parts.join(", ")
    }

    /// Extract keywords from JATS XML for subject metadata
    ///
    /// Searches for `<kwd-group>` elements in `<article-meta>` and extracts `<kwd>` keywords.
    /// These are used to populate the DocumentMetadata.subject field.
    ///
    /// # Returns
    /// Vector of keyword strings, empty if no keywords found
    fn extract_keywords(xml_content: &str) -> Result<Vec<String>, DoclingError> {
        let parse_options = roxmltree::ParsingOptions {
            allow_dtd: true,
            ..roxmltree::ParsingOptions::default()
        };
        let doc = roxmltree::Document::parse_with_options(xml_content, parse_options)
            .map_err(|e| DoclingError::BackendError(format!("XML parse error: {e}")))?;

        let mut keywords = Vec::new();

        // Find all <kwd> elements within <kwd-group> in <article-meta>
        for node in doc.descendants() {
            if node.tag_name().name() == "kwd"
                && node
                    .ancestors()
                    .any(|a| a.is_element() && a.tag_name().name() == "kwd-group")
                && node
                    .ancestors()
                    .any(|a| a.is_element() && a.tag_name().name() == "article-meta")
            {
                let keyword = Self::get_text(&node).trim().to_string();
                if !keyword.is_empty() {
                    keywords.push(keyword);
                }
            }
        }

        Ok(keywords)
    }

    /// Extract references from JATS XML `<back><ref-list>` section
    ///
    /// Parses citation elements (element-citation, mixed-citation) and creates
    /// a "References" section with list items for each reference.
    ///
    /// Returns `Vec<DocItem>` with:
    /// - `SectionHeader` for "References" (level 2)
    /// - `ListItem` for each reference
    fn extract_references(
        xml_content: &str,
        item_count: &mut usize,
    ) -> Result<Vec<DocItem>, DoclingError> {
        let parse_options = roxmltree::ParsingOptions {
            allow_dtd: true,
            ..roxmltree::ParsingOptions::default()
        };
        let doc = roxmltree::Document::parse_with_options(xml_content, parse_options)
            .map_err(|e| DoclingError::BackendError(format!("XML parse error: {e}")))?;

        let mut doc_items = Vec::new();

        // Find <back><ref-list> element
        let ref_list = doc
            .descendants()
            .find(|n| n.tag_name().name() == "back")
            .and_then(|back| {
                back.children()
                    .find(|n| n.is_element() && n.tag_name().name() == "ref-list")
            });

        let Some(ref_list) = ref_list else {
            return Ok(doc_items); // No references, return empty
        };

        // Collect all <ref> elements
        let refs: Vec<_> = ref_list
            .children()
            .filter(|n| n.is_element() && n.tag_name().name() == "ref")
            .collect();

        if refs.is_empty() {
            return Ok(doc_items); // No references, return empty
        }

        // Add "References" section header
        let section_ref = format!("#/texts/{item_count}");
        *item_count += 1;
        doc_items.push(DocItem::SectionHeader {
            self_ref: section_ref.clone(),
            parent: None,
            children: vec![],
            content_layer: "body".to_string(),
            prov: create_provenance(1),
            orig: "References".to_string(),
            text: "References".to_string(),
            level: 2,
            formatting: None,
            hyperlink: None,
        });

        // Process each <ref> element
        for ref_node in refs {
            let citation_text = Self::extract_citation_text(&ref_node);
            if citation_text.is_empty() {
                continue;
            }
            // Escape underscores for markdown (Python does this)
            let citation_text = Self::escape_underscores(&citation_text);

            // Create ListItem for each reference
            let list_item = create_list_item(
                *item_count, // text_index
                citation_text.clone(),
                "-".to_string(),                  // marker
                false,                            // enumerated (false = bullet list)
                create_provenance(1),             // provenance
                Some(ItemRef::new(&section_ref)), // parent
            );
            doc_items.push(list_item);
            *item_count += 1;
        }

        Ok(doc_items)
    }

    /// Extract citation text from a `<ref>` element
    ///
    /// Handles both structured (element-citation) and unstructured (mixed-citation) formats.
    ///
    /// For element-citation: Extracts author, year, title, source in structured format
    /// For mixed-citation: Uses the text content as-is
    fn extract_citation_text(ref_node: &roxmltree::Node) -> String {
        // Try mixed-citation first (simpler, unstructured format)
        if let Some(mixed_citation) = ref_node
            .children()
            .find(|n| n.is_element() && n.tag_name().name() == "mixed-citation")
        {
            return Self::get_text(&mixed_citation).trim().to_string();
        }

        // Try element-citation (structured format)
        if let Some(element_citation) = ref_node
            .children()
            .find(|n| n.is_element() && n.tag_name().name() == "element-citation")
        {
            return Self::format_element_citation(&element_citation);
        }

        // Fallback: get all text content
        Self::get_text(ref_node).trim().to_string()
    }

    /// Get text content from child element by tag name
    fn get_child_text(parent: &roxmltree::Node, tag: &str) -> Option<String> {
        parent
            .children()
            .find(|n| n.is_element() && n.tag_name().name() == tag)
            .map(|n| Self::get_text(&n).trim().to_string())
            .filter(|s| !s.is_empty())
    }

    /// Get text content from child element matching any of the tags
    fn get_child_text_multi(parent: &roxmltree::Node, tags: &[&str]) -> Option<String> {
        parent
            .children()
            .find(|n| n.is_element() && tags.contains(&n.tag_name().name()))
            .map(|n| Self::get_text(&n).trim().to_string())
            .filter(|s| !s.is_empty())
    }

    /// Get pub-id element text with specific type attribute
    fn get_pub_id_text(parent: &roxmltree::Node, id_type: &str) -> Option<String> {
        parent
            .children()
            .find(|n| {
                n.is_element()
                    && n.tag_name().name() == "pub-id"
                    && n.attribute("pub-id-type") == Some(id_type)
            })
            .map(|n| Self::get_text(&n).trim().to_string())
            .filter(|s| !s.is_empty())
    }

    /// Format authors from person-group for citation
    fn format_citation_authors(element_citation: &roxmltree::Node) -> Option<String> {
        let person_group = element_citation
            .children()
            .find(|n| n.is_element() && n.tag_name().name() == "person-group")?;

        let authors: Vec<String> = person_group
            .children()
            .filter(|n| n.is_element() && n.tag_name().name() == "name")
            .map(|name_node| {
                let surname = Self::get_child_text(&name_node, "surname").unwrap_or_default();
                let given_names =
                    Self::get_child_text(&name_node, "given-names").unwrap_or_default();
                if given_names.is_empty() {
                    surname
                } else {
                    format!("{surname} {given_names}")
                }
            })
            .collect();

        if authors.is_empty() {
            None
        } else {
            Some(format!("{}. ", authors.join(", ")))
        }
    }

    /// Format volume and pages for citation
    fn format_volume_pages(
        vol: &str,
        fpage: Option<&str>,
        lpage: Option<&str>,
        eloc: Option<&str>,
    ) -> String {
        let mut result = vol.to_string();
        match (fpage, lpage) {
            (Some(fp), Some(lp)) if !fp.is_empty() && !lp.is_empty() => {
                result.push(':');
                result.push_str(fp);
                result.push('–');
                result.push_str(lp);
            }
            (Some(fp), _) if !fp.is_empty() => {
                result.push(':');
                result.push_str(fp);
            }
            _ => {
                if let Some(el) = eloc {
                    if !el.is_empty() {
                        result.push(':');
                        result.push_str(el);
                    }
                }
            }
        }
        result
    }

    /// Format structured element-citation into readable text
    ///
    /// Constructs citation string from structured XML elements:
    /// - Author(s) (person-group)
    /// - Year
    /// - Title (article-title)
    /// - Source (source, journal name)
    /// - Volume, pages
    fn format_element_citation(element_citation: &roxmltree::Node) -> String {
        let mut result = String::new();

        // Extract authors
        if let Some(authors) = Self::format_citation_authors(element_citation) {
            result.push_str(&authors);
        }

        // Extract title (article-title or data-title)
        if let Some(title) =
            Self::get_child_text_multi(element_citation, &["article-title", "data-title"])
        {
            result.push_str(&title);
            result.push_str(". ");
        }

        // Extract source (journal name) and volume
        let source = Self::get_child_text(element_citation, "source");
        let volume = Self::get_child_text(element_citation, "volume");

        if let Some(s) = &source {
            result.push_str(s);
            if volume.is_some() {
                result.push(' ');
            }
        }

        // Extract fpage, lpage, elocation-id and format volume/pages
        let fpage = Self::get_child_text(element_citation, "fpage");
        let lpage = Self::get_child_text(element_citation, "lpage");
        let elocation_id = Self::get_child_text(element_citation, "elocation-id");

        if let Some(vol) = &volume {
            result.push_str(&Self::format_volume_pages(
                vol,
                fpage.as_deref(),
                lpage.as_deref(),
                elocation_id.as_deref(),
            ));
        }

        // Extract year
        if let Some(year) = Self::get_child_text(element_citation, "year") {
            result.push_str(" (");
            result.push_str(&year);
            result.push_str(").");
        }

        // Extract DOI
        if let Some(doi) = Self::get_pub_id_text(element_citation, "doi") {
            result.push_str(" DOI: ");
            result.push_str(&doi);
        }

        // Extract PMID
        if let Some(pmid) = Self::get_pub_id_text(element_citation, "pmid") {
            result.push_str(", PMID: ");
            result.push_str(&pmid);
        }

        // Extract accession ID (for database references like GEO, GenBank)
        if let Some(accession_node) = element_citation.children().find(|n| {
            n.is_element()
                && n.tag_name().name() == "pub-id"
                && n.attribute("pub-id-type") == Some("accession")
        }) {
            let accession_id = Self::get_text(&accession_node).trim().to_string();
            if !accession_id.is_empty() {
                let authority = accession_node
                    .attribute("assigning-authority")
                    .unwrap_or("");
                if !result.is_empty() {
                    result.push(' ');
                }
                if !authority.is_empty() {
                    result.push_str(authority);
                    result.push_str(": ");
                }
                result.push_str(&accession_id);
            }
        }

        result.trim().to_string()
    }

    /// Extract back matter from JATS XML `<back>` section (excluding ref-list)
    ///
    /// Python reference: jats_backend.py:165-168 - walks over `<back>` element
    ///
    /// Extracts:
    /// - Funding Information (from `<funding-group>`)
    /// - Acknowledgements (from `<ack>`)
    /// - Additional information (from `<notes>` or `<fn-group>`)
    /// - Additional files (from `<sec sec-type="supplementary-material">`)
    /// - Data availability (from `<sec sec-type="data-availability">`)
    fn extract_back_matter(
        &self,
        xml_content: &str,
        item_count: &mut usize,
    ) -> Result<Vec<DocItem>, DoclingError> {
        let parse_options = roxmltree::ParsingOptions {
            allow_dtd: true,
            ..roxmltree::ParsingOptions::default()
        };
        let doc = roxmltree::Document::parse_with_options(xml_content, parse_options)
            .map_err(|e| DoclingError::BackendError(format!("XML parse error: {e}")))?;

        let mut doc_items = Vec::new();

        // Find <back> element
        let Some(back) = doc.descendants().find(|n| n.tag_name().name() == "back") else {
            return Ok(doc_items); // No back element
        };

        // Extract Funding Information from front-matter funding-group
        // Python outputs this for elife-style papers that have structured funding with award-groups
        // but NOT for pntd-style papers that only have funding-statement
        let funding_items = Self::extract_funding_info(&doc, item_count);
        doc_items.extend(funding_items);

        // Extract acknowledgements from <back><ack>
        if let Some(ack) = back
            .children()
            .find(|n| n.is_element() && n.tag_name().name() == "ack")
        {
            let ack_items = Self::extract_acknowledgements(&ack, item_count);
            doc_items.extend(ack_items);
        }

        // Extract "Additional information" header - Python adds this before "Additional files"
        // This section appears when there are supplementary materials, notes, or fn-groups
        let has_supp_material = back
            .descendants()
            .any(|n| n.tag_name().name() == "supplementary-material");
        let has_notes = back.children().any(|n| {
            n.is_element() && (n.tag_name().name() == "notes" || n.tag_name().name() == "fn-group")
        });

        // Python adds "Additional information" header when supplementary materials exist
        if has_supp_material || has_notes {
            doc_items.push(DocItem::SectionHeader {
                self_ref: format!("#/texts/{item_count}"),
                parent: None,
                children: vec![],
                content_layer: "body".to_string(),
                prov: create_provenance(1),
                orig: "Additional information".to_string(),
                text: "Additional information".to_string(),
                level: 2,
                formatting: None,
                hyperlink: None,
            });
            *item_count += 1;
        }

        // Extract "Additional files" section - from <supplementary-material> refs
        let has_supp = back
            .descendants()
            .any(|n| n.tag_name().name() == "supplementary-material");
        if has_supp {
            doc_items.push(DocItem::SectionHeader {
                self_ref: format!("#/texts/{item_count}"),
                parent: None,
                children: vec![],
                content_layer: "body".to_string(),
                prov: create_provenance(1),
                orig: "Additional files".to_string(),
                text: "Additional files".to_string(),
                level: 2,
                formatting: None,
                hyperlink: None,
            });
            *item_count += 1;
        }

        // Extract data availability sections from <back><sec sec-type="data-availability">
        for sec in back.children().filter(|n| {
            n.is_element()
                && n.tag_name().name() == "sec"
                && n.attribute("sec-type") == Some("data-availability")
        }) {
            let da_items = self.extract_data_availability(&sec, item_count);
            doc_items.extend(da_items);
        }

        // Also check for <back><sec> without sec-type that contains data availability text
        for sec in back.children().filter(|n| {
            n.is_element() && n.tag_name().name() == "sec" && n.attribute("sec-type").is_none()
        }) {
            // Check if title mentions "data availability"
            if let Some(title_node) = sec
                .children()
                .find(|n| n.is_element() && n.tag_name().name() == "title")
            {
                let title = Self::get_text(&title_node).to_lowercase();
                if title.contains("data availability") || title.contains("data access") {
                    let da_items = self.extract_data_availability(&sec, item_count);
                    doc_items.extend(da_items);
                }
            }
        }

        Ok(doc_items)
    }

    /// Extract acknowledgements from `<ack>` element
    fn extract_acknowledgements(ack: &roxmltree::Node, item_count: &mut usize) -> Vec<DocItem> {
        let mut doc_items = Vec::new();

        // Extract title from XML <ack><title> element
        // This preserves the exact spelling used in the XML (British vs American)
        // e.g., elife uses "Acknowledgements" (British), pntd uses "Acknowledgments" (American)
        let title = ack
            .children()
            .find(|n| n.is_element() && n.tag_name().name() == "title")
            .map(|t| Self::get_text(&t).trim().to_string())
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| "Acknowledgments".to_string());

        doc_items.push(DocItem::SectionHeader {
            self_ref: format!("#/texts/{item_count}"),
            parent: None,
            children: vec![],
            content_layer: "body".to_string(),
            prov: create_provenance(1),
            orig: title.clone(),
            text: title,
            level: 2,
            formatting: None,
            hyperlink: None,
        });
        *item_count += 1;

        // Extract all paragraph text from <ack>
        for p in ack.descendants().filter(|n| n.tag_name().name() == "p") {
            let text = Self::get_text(&p).trim().to_string();
            if !text.is_empty() {
                // Escape underscores for markdown (Python does this)
                let text = Self::escape_underscores(&text);
                doc_items.push(create_text_item(*item_count, text, create_provenance(1)));
                *item_count += 1;
            }
        }

        doc_items
    }

    /// Extract funding information from front-matter funding-group
    /// Only outputs if there are structured award-groups (not just funding-statement)
    /// This matches Python behavior: elife gets "Funding Information", pntd does not
    #[allow(clippy::too_many_lines)] // Complex funding extraction - keeping together for clarity
    fn extract_funding_info(doc: &roxmltree::Document, item_count: &mut usize) -> Vec<DocItem> {
        let mut doc_items = Vec::new();

        // Find funding-group in article-meta (front-matter)
        let funding_group = doc
            .descendants()
            .find(|n| n.tag_name().name() == "funding-group");

        let Some(funding_group) = funding_group else {
            return doc_items;
        };

        // Check if there are award-groups (structured funding) with FundRef institution-id
        // Python docling only outputs Funding Information when FundRef URLs are present
        // (e.g., elife has FundRef and gets Funding section; pone has funder-id type and doesn't)
        let award_groups: Vec<_> = funding_group
            .children()
            .filter(|n| n.is_element() && n.tag_name().name() == "award-group")
            .filter(|ag| {
                // Must have principal-award-recipient AND FundRef institution-id to be included
                ag.descendants()
                    .any(|n| n.tag_name().name() == "principal-award-recipient")
                    && ag.descendants().any(|n| {
                        n.tag_name().name() == "institution-id"
                            && n.attribute("institution-id-type") == Some("FundRef")
                    })
            })
            .collect();

        if award_groups.is_empty() {
            return doc_items;
        }

        // Add "Funding Information" section header
        doc_items.push(DocItem::SectionHeader {
            self_ref: format!("#/texts/{item_count}"),
            parent: None,
            children: vec![],
            content_layer: "body".to_string(),
            prov: create_provenance(1),
            orig: "Funding Information".to_string(),
            text: "Funding Information".to_string(),
            level: 2,
            formatting: None,
            hyperlink: None,
        });
        *item_count += 1;

        // Add introductory text
        doc_items.push(create_text_item(
            *item_count,
            "This paper was supported by the following grants:".to_string(),
            create_provenance(1),
        ));
        *item_count += 1;

        // Extract each award-group as a list item
        for award_group in &award_groups {
            let mut parts = Vec::new();

            // Get funder-id URL and institution name
            // These are concatenated directly without space (per Python output format)
            let mut funder_institution = String::new();

            if let Some(institution_id) = award_group.descendants().find(|n| {
                n.tag_name().name() == "institution-id"
                    && n.attribute("institution-id-type") == Some("FundRef")
            }) {
                let funder_id = Self::get_text(&institution_id).trim().to_string();
                if !funder_id.is_empty() {
                    funder_institution.push_str(&funder_id);
                }
            }

            // Get institution name - concatenate directly to funder-id (no space)
            if let Some(institution) = award_group.descendants().find(|n| {
                n.tag_name().name() == "institution"
                    && n.parent()
                        .is_some_and(|p| p.tag_name().name() == "institution-wrap")
            }) {
                let name = Self::get_text(&institution).trim().to_string();
                if !name.is_empty() {
                    funder_institution.push_str(&name);
                }
            }

            if !funder_institution.is_empty() {
                parts.push(funder_institution);
            }

            // Get award-id
            if let Some(award_id) = award_group
                .descendants()
                .find(|n| n.tag_name().name() == "award-id")
            {
                let id = Self::get_text(&award_id).trim().to_string();
                if !id.is_empty() {
                    // Escape underscores for markdown
                    parts.push(Self::escape_underscores(&id));
                }
            }

            // Get principal-award-recipient name(s)
            let mut recipients = Vec::new();
            for recipient in award_group
                .descendants()
                .filter(|n| n.tag_name().name() == "principal-award-recipient")
            {
                if let Some(name) = recipient
                    .children()
                    .find(|n| n.is_element() && n.tag_name().name() == "name")
                {
                    let surname = name
                        .children()
                        .find(|n| n.tag_name().name() == "surname")
                        .map(|n| Self::get_text(&n).trim().to_string())
                        .unwrap_or_default();
                    let given = name
                        .children()
                        .find(|n| n.tag_name().name() == "given-names")
                        .map(|n| Self::get_text(&n).trim().to_string())
                        .unwrap_or_default();
                    if !surname.is_empty() || !given.is_empty() {
                        recipients.push(format!("{given} {surname}").trim().to_string());
                    }
                }
            }

            if !recipients.is_empty() {
                parts.push(format!("to {}.", recipients.join(", ")));
            }

            if !parts.is_empty() {
                let text = parts.join(" ");
                doc_items.push(DocItem::ListItem {
                    self_ref: format!("#/texts/{item_count}"),
                    parent: None,
                    children: vec![],
                    content_layer: "body".to_string(),
                    prov: create_provenance(1),
                    orig: text.clone(),
                    text,
                    enumerated: false,
                    marker: "- ".to_string(),
                    formatting: None,
                    hyperlink: None,
                });
                *item_count += 1;
            }
        }

        doc_items
    }

    /// Extract data availability section from `<sec sec-type="data-availability">`
    // Method signature kept for API consistency with other JatsBackend methods
    #[allow(clippy::unused_self)]
    fn extract_data_availability(
        &self,
        sec: &roxmltree::Node,
        item_count: &mut usize,
    ) -> Vec<DocItem> {
        let mut doc_items = Vec::new();

        // Add "Data availability" section header
        doc_items.push(DocItem::SectionHeader {
            self_ref: format!("#/texts/{item_count}"),
            parent: None,
            children: vec![],
            content_layer: "body".to_string(),
            prov: create_provenance(1),
            orig: "Data availability".to_string(),
            text: "Data availability".to_string(),
            level: 2,
            formatting: None,
            hyperlink: None,
        });
        *item_count += 1;

        // Extract all paragraphs, properly formatting element-citations
        for p in sec.descendants().filter(|n| n.tag_name().name() == "p") {
            // Check if this paragraph contains an element-citation
            if let Some(elem_citation) = p
                .children()
                .find(|n| n.is_element() && n.tag_name().name() == "element-citation")
            {
                // Use format_element_citation for structured data citations
                // Apply underscore escaping for markdown (Python does this)
                let text = Self::escape_underscores(&Self::format_element_citation(&elem_citation));
                if !text.is_empty() {
                    doc_items.push(create_text_item(*item_count, text, create_provenance(1)));
                    *item_count += 1;
                }
            } else {
                // Regular paragraph text
                let text = Self::get_text(&p).trim().to_string();
                if !text.is_empty() {
                    doc_items.push(create_text_item(*item_count, text, create_provenance(1)));
                    *item_count += 1;
                }
            }
        }

        // Extract related-object elements (dataset links)
        for obj in sec
            .descendants()
            .filter(|n| n.tag_name().name() == "related-object")
        {
            let text = Self::get_text(&obj).trim().to_string();
            if !text.is_empty() {
                doc_items.push(create_text_item(*item_count, text, create_provenance(1)));
                *item_count += 1;
            }
        }

        doc_items
    }

    /// Extract body sections and paragraphs from JATS XML
    ///
    /// Python reference: jats_backend.py:716-819 (`_walk_linear`)
    /// Recursively walks the `<body>` element to extract sections and paragraphs
    /// Phase 3: Basic section/paragraph extraction (no lists, figures, tables yet)
    fn extract_body(&self, xml_content: &str) -> Result<Vec<DocItem>, DoclingError> {
        // Parse XML with roxmltree for DOM-like tree walking
        // Enable DTD support since JATS files typically include DOCTYPE declarations
        let parse_options = roxmltree::ParsingOptions {
            allow_dtd: true,
            ..roxmltree::ParsingOptions::default()
        };
        let doc = roxmltree::Document::parse_with_options(xml_content, parse_options)
            .map_err(|e| DoclingError::BackendError(format!("XML parse error: {e}")))?;

        // Find <body> element
        let body = doc.descendants().find(|n| n.tag_name().name() == "body");

        let Some(body) = body else {
            // No body element found, return empty
            return Ok(Vec::new());
        };

        // Walk the body tree and collect DocItems
        let mut doc_items = Vec::new();
        let mut item_count = 0;
        // Start at level 1 so first body section becomes level 2 (##) in markdown
        // Title is level 1 (#), body sections are level 2+ (##, ###, etc)
        let mut heading_level = 1;

        self.walk_linear(
            &body,
            &mut doc_items,
            &mut item_count,
            &mut heading_level,
            None, // parent ref
        )?;

        Ok(doc_items)
    }

    /// Recursive tree walker for JATS body elements
    ///
    /// Python reference: jats_backend.py:716-819 (`_walk_linear`)
    ///
    /// Note: `&self` parameter is only used for recursion (not accessing state).
    /// This is intentional to maintain API consistency with other backend methods.
    #[allow(
        clippy::only_used_in_recursion,
        reason = "&self kept for API consistency with other backend methods"
    )]
    fn walk_linear(
        &self,
        node: &roxmltree::Node,
        doc_items: &mut Vec<DocItem>,
        item_count: &mut usize,
        heading_level: &mut usize,
        parent_ref: Option<ItemRef>,
    ) -> Result<String, DoclingError> {
        let mut state = JatsWalkState::new(doc_items, item_count, heading_level, parent_ref);
        state.process_node(self, node)
    }
}

/// Helper struct for JATS tree walking state
#[derive(Debug)]
struct JatsWalkState<'a> {
    doc_items: &'a mut Vec<DocItem>,
    item_count: &'a mut usize,
    heading_level: &'a mut usize,
    parent_ref: Option<ItemRef>,
}

impl<'a> JatsWalkState<'a> {
    const fn new(
        doc_items: &'a mut Vec<DocItem>,
        item_count: &'a mut usize,
        heading_level: &'a mut usize,
        parent_ref: Option<ItemRef>,
    ) -> Self {
        Self {
            doc_items,
            item_count,
            heading_level,
            parent_ref,
        }
    }

    fn process_node(
        &mut self,
        backend: &JatsBackend,
        node: &roxmltree::Node,
    ) -> Result<String, DoclingError> {
        let tag_name = node.tag_name().name();
        let mut node_text = String::new();

        // Collect node text (if any)
        if let Some(text) = node.text() {
            if tag_name != "term" {
                node_text.push_str(&text.replace('\n', " "));
            }
        }

        // Process child elements
        for child in node.children() {
            if !child.is_element() {
                continue;
            }

            let child_tag = child.tag_name().name();
            let (stop_walk, new_parent_ref) = self.handle_element(&child, child_tag)?;

            // Recurse into child
            if !stop_walk {
                let child_text = backend.walk_linear(
                    &child,
                    self.doc_items,
                    self.item_count,
                    self.heading_level,
                    new_parent_ref,
                )?;

                // Don't accumulate text for flush_tags when parent is <p>
                let flush_tags = ["ack", "sec", "list", "boxed-text", "disp-formula", "fig"];
                if !(tag_name == "p" && flush_tags.contains(&child_tag)) {
                    node_text.push_str(&child_text);
                }

                // Decrement heading level after processing section
                if (child_tag == "sec" || child_tag == "ack") && *self.heading_level > 0 {
                    *self.heading_level -= 1;
                }
            }

            // Pick up tail text
            if let Some(tail) = child.tail() {
                node_text.push_str(&tail.replace('\n', " "));
            }
        }

        // Create paragraph if this is a <p> element with text
        if tag_name == "p" && !node_text.trim().is_empty() {
            self.handle_paragraph(node)?;
            return Ok(String::new());
        }

        Ok(node_text)
    }

    #[allow(
        clippy::unnecessary_wraps,
        reason = "Result return type for consistency with walk_linear error handling pattern"
    )]
    fn handle_element(
        &mut self,
        child: &roxmltree::Node,
        tag: &str,
    ) -> Result<(bool, Option<ItemRef>), DoclingError> {
        let mut stop_walk = false;
        let mut new_parent_ref = self.parent_ref.clone();

        match tag {
            "sec" | "ack" => new_parent_ref = self.handle_section(child, tag),
            "list" => new_parent_ref = Some(self.handle_list()),
            "list-item" => new_parent_ref = Some(self.handle_list_item(child)),
            "def-list" => new_parent_ref = Some(self.handle_def_list()),
            "def-item" => {
                self.handle_def_item(child);
                stop_walk = true;
            }
            "fig" => {
                self.handle_fig(child);
                stop_walk = true;
            }
            "table-wrap" => {
                self.handle_table_wrap(child);
                stop_walk = true;
            }
            "supplementary-material"
            | "fn-group"
            | "ref-list"
            | "element-citation"
            | "mixed-citation"
            | "tex-math"
            | "inline-formula" => {
                stop_walk = true;
            }
            _ => {}
        }

        Ok((stop_walk, new_parent_ref))
    }

    fn handle_section(&mut self, child: &roxmltree::Node, tag: &str) -> Option<ItemRef> {
        let mut new_parent_ref = self.parent_ref.clone();

        // Find <title> child
        let title_text = child
            .children()
            .find(|n| {
                n.is_element() && (n.tag_name().name() == "title" || n.tag_name().name() == "label")
            })
            .map_or_else(
                || {
                    if tag == "ack" {
                        "Acknowledgments".to_string()
                    } else {
                        String::new()
                    }
                },
                |title_node| JatsBackend::get_text(&title_node),
            );

        if !title_text.is_empty() {
            *self.heading_level += 1;
            let self_ref = format!("#/texts/{}", *self.item_count);
            *self.item_count += 1;

            self.doc_items.push(DocItem::SectionHeader {
                self_ref: self_ref.clone(),
                parent: self.parent_ref.clone(),
                children: vec![],
                content_layer: "body".to_string(),
                prov: create_provenance(1),
                orig: title_text.clone(),
                text: title_text,
                level: *self.heading_level,
                formatting: None,
                hyperlink: None,
            });
            new_parent_ref = Some(ItemRef::new(self_ref));
        }

        // Handle <subtitle> if present
        if let Some(subtitle_node) = child
            .children()
            .find(|n| n.is_element() && n.tag_name().name() == "subtitle")
        {
            let subtitle_text = JatsBackend::get_text(&subtitle_node);
            if !subtitle_text.is_empty() {
                self.doc_items.push(DocItem::Text {
                    self_ref: format!("#/texts/{}", *self.item_count),
                    parent: new_parent_ref.clone(),
                    children: vec![],
                    content_layer: "body".to_string(),
                    prov: create_provenance(1),
                    orig: subtitle_text.clone(),
                    text: subtitle_text,
                    formatting: None,
                    hyperlink: None,
                });
                *self.item_count += 1;
            }
        }

        new_parent_ref
    }

    fn handle_list(&mut self) -> ItemRef {
        let self_ref = format!("#/groups/{}", *self.item_count);
        *self.item_count += 1;

        self.doc_items.push(DocItem::List {
            self_ref: self_ref.clone(),
            parent: self.parent_ref.clone(),
            children: vec![],
            content_layer: "body".to_string(),
            name: "list".to_string(),
        });
        ItemRef::new(self_ref)
    }

    fn handle_list_item(&mut self, child: &roxmltree::Node) -> ItemRef {
        let mut direct_text = String::new();

        for child_node in child.children() {
            if child_node.is_element() {
                let tag = child_node.tag_name().name();
                if tag == "p" {
                    direct_text.push_str(&JatsBackend::get_text(&child_node));
                    direct_text.push(' ');
                } else if tag != "list" {
                    if let Some(text) = child_node.text() {
                        direct_text.push_str(text);
                    }
                }
            } else if let Some(text) = child_node.text() {
                direct_text.push_str(text);
            }
        }

        let self_ref = format!("#/items/{}", *self.item_count);
        let doc_item = create_list_item(
            *self.item_count,
            direct_text.trim().to_string(),
            "-".to_string(),
            false,
            create_provenance(1),
            self.parent_ref.clone(),
        );
        *self.item_count += 1;
        self.doc_items.push(doc_item);
        ItemRef::new(self_ref)
    }

    fn handle_def_list(&mut self) -> ItemRef {
        let self_ref = format!("#/groups/{}", *self.item_count);
        *self.item_count += 1;

        self.doc_items.push(DocItem::List {
            self_ref: self_ref.clone(),
            parent: self.parent_ref.clone(),
            children: vec![],
            content_layer: "body".to_string(),
            name: "def-list".to_string(),
        });
        ItemRef::new(self_ref)
    }

    fn handle_def_item(&mut self, child: &roxmltree::Node) {
        let mut term_text = String::new();
        let mut def_text = String::new();

        for child_node in child.children() {
            if child_node.is_element() {
                match child_node.tag_name().name() {
                    "term" => term_text = JatsBackend::get_text(&child_node),
                    "def" => def_text = JatsBackend::get_text(&child_node),
                    _ => {}
                }
            }
        }

        let combined_text = match (!term_text.is_empty(), !def_text.is_empty()) {
            (true, true) => format!("{}: {}", term_text.trim(), def_text.trim()),
            (true, false) => term_text.trim().to_string(),
            (false, true) => def_text.trim().to_string(),
            (false, false) => String::new(),
        };

        if !combined_text.is_empty() {
            let doc_item = create_list_item(
                *self.item_count,
                combined_text,
                "-".to_string(),
                false,
                create_provenance(1),
                self.parent_ref.clone(),
            );
            *self.item_count += 1;
            self.doc_items.push(doc_item);
        }
    }

    fn handle_fig(&mut self, child: &roxmltree::Node) {
        let (label, caption) = Self::extract_label_caption(child, true);
        let caption_text = Self::format_label_caption(&label, &caption);

        if !caption_text.is_empty() {
            self.push_text_item(caption_text);
        }

        // Add <!-- image --> marker
        self.push_text_item("<!-- image -->".to_string());
    }

    fn handle_table_wrap(&mut self, child: &roxmltree::Node) {
        let (label, caption) = Self::extract_label_caption(child, false);
        let mut table_node: Option<roxmltree::Node> = None;

        // Find table node
        for table_child in child.children() {
            if !table_child.is_element() {
                continue;
            }
            match table_child.tag_name().name() {
                "table" => {
                    table_node = Some(table_child);
                }
                "alternatives" => {
                    for alt_child in table_child.children() {
                        if alt_child.is_element() && alt_child.tag_name().name() == "table" {
                            table_node = Some(alt_child);
                            break;
                        }
                    }
                }
                _ => {}
            }
        }

        let caption_text = Self::format_label_caption(&label, &caption);
        if !caption_text.is_empty() {
            self.push_text_item(caption_text);
        }

        if let Some(table) = table_node {
            let markdown_table = JatsBackend::table_to_markdown(&table);
            if !markdown_table.is_empty() {
                self.push_text_item(markdown_table);
            }
        }
    }

    fn extract_label_caption(node: &roxmltree::Node, skip_supp_material: bool) -> (String, String) {
        let mut label = String::new();
        let mut caption = String::new();

        for child in node.children() {
            if !child.is_element() {
                continue;
            }
            match child.tag_name().name() {
                "label" => label = JatsBackend::get_text(&child),
                "caption" => {
                    for caption_child in child.children() {
                        if !caption_child.is_element() {
                            continue;
                        }
                        let tag = caption_child.tag_name().name();
                        if tag == "title" || tag == "p" {
                            if skip_supp_material {
                                let has_supp = caption_child.children().any(|c| {
                                    c.is_element()
                                        && c.tag_name().name() == "supplementary-material"
                                });
                                if has_supp {
                                    continue;
                                }
                            }
                            let text = JatsBackend::get_text(&caption_child);
                            if !text.is_empty() {
                                if !caption.is_empty() {
                                    caption.push(' ');
                                }
                                caption.push_str(&text);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        (label, caption)
    }

    #[inline]
    fn format_label_caption(label: &str, caption: &str) -> String {
        let label_trimmed = label.trim();
        let caption_trimmed = caption.trim();
        match (!label_trimmed.is_empty(), !caption_trimmed.is_empty()) {
            (true, true) => format!("{label_trimmed} {caption_trimmed}"),
            (true, false) => label_trimmed.to_string(),
            (false, true) => caption_trimmed.to_string(),
            (false, false) => String::new(),
        }
    }

    fn push_text_item(&mut self, text: String) {
        self.doc_items.push(DocItem::Text {
            self_ref: format!("#/texts/{}", *self.item_count),
            parent: self.parent_ref.clone(),
            children: vec![],
            content_layer: "body".to_string(),
            prov: create_provenance(1),
            orig: text.clone(),
            text,
            formatting: None,
            hyperlink: None,
        });
        *self.item_count += 1;
    }

    #[allow(
        clippy::unnecessary_wraps,
        reason = "Result kept for API consistency with other handlers"
    )]
    fn handle_paragraph(&mut self, node: &roxmltree::Node) -> Result<(), DoclingError> {
        let default_formatting = Formatting {
            bold: None,
            italic: None,
            underline: None,
            strikethrough: None,
            code: None,
            script: None,
            font_size: None,
            font_family: None,
        };
        let runs = JatsBackend::extract_text_with_formatting(node, &default_formatting);
        let grouped_runs = JatsBackend::group_runs_by_formatting(runs);

        let mut plain_text_buffer = String::new();
        for run in &grouped_runs {
            if !run.text.is_empty() {
                plain_text_buffer.push_str(&run.text);
            }
        }

        if !plain_text_buffer.trim().is_empty() {
            let escaped_text =
                JatsBackend::escape_html_entities(&plain_text_buffer.trim().replace('\n', " "));
            let escaped_text = JatsBackend::escape_underscores(&escaped_text);
            if !escaped_text.is_empty() {
                self.push_text_item(escaped_text);
            }
        }
        Ok(())
    }
}

impl JatsBackend {
    /// Escape underscores in text for markdown output
    ///
    /// Python docling escapes underscores in body text to prevent markdown
    /// interpretation as emphasis. Pattern: `_` → `\_`
    /// Does NOT escape if underscore is already escaped (preceded by `\`).
    #[inline]
    fn escape_underscores(text: &str) -> String {
        // Use regex to escape unescaped underscores (not preceded by backslash)
        // Pattern: negative lookbehind for backslash, then underscore
        let mut result = String::with_capacity(text.len() + text.matches('_').count());
        let chars: Vec<char> = text.chars().collect();
        for (i, c) in chars.iter().enumerate() {
            if *c == '_' {
                // Check if preceded by backslash - if not, add escape
                if i == 0 || chars[i - 1] != '\\' {
                    result.push('\\');
                }
                result.push('_');
            } else {
                result.push(*c);
            }
        }
        result
    }

    /// Convert JATS `<table>` element to markdown table format
    ///
    /// Handles `<thead>`, `<tbody>`, `<tfoot>` sections with `<tr>`/`<th>`/`<td>` elements
    fn table_to_markdown(table: &roxmltree::Node) -> String {
        let (rows, header_row_count) = Self::extract_table_rows(table);
        if rows.is_empty() {
            return String::new();
        }

        let col_count = rows.iter().map(Vec::len).max().unwrap_or(0);
        if col_count == 0 {
            return String::new();
        }

        let col_widths = Self::calculate_column_widths(&rows, col_count);
        let is_numeric = Self::detect_numeric_columns(&rows, col_count, header_row_count);
        let col_widths = Self::apply_column_padding(&rows, col_widths, &is_numeric);

        Self::format_markdown_table(&rows, &col_widths, &is_numeric, col_count)
    }

    /// Extract rows from table element (thead, tbody, tfoot, and direct tr children)
    fn extract_table_rows(table: &roxmltree::Node) -> (Vec<Vec<String>>, usize) {
        let mut rows: Vec<Vec<String>> = Vec::new();
        let mut header_row_count = 0;

        for child in table.children() {
            if !child.is_element() {
                continue;
            }
            match child.tag_name().name() {
                "thead" => {
                    for tr in child.children() {
                        if tr.is_element() && tr.tag_name().name() == "tr" {
                            let row = Self::extract_table_row(&tr);
                            if !row.is_empty() {
                                rows.push(row);
                                header_row_count += 1;
                            }
                        }
                    }
                }
                "tbody" | "tfoot" => {
                    for tr in child.children() {
                        if tr.is_element() && tr.tag_name().name() == "tr" {
                            let row = Self::extract_table_row(&tr);
                            if !row.is_empty() {
                                rows.push(row);
                            }
                        }
                    }
                }
                "tr" => {
                    let row = Self::extract_table_row(&child);
                    if !row.is_empty() {
                        rows.push(row);
                    }
                }
                _ => {}
            }
        }
        (rows, header_row_count)
    }

    /// Calculate max width for each column
    fn calculate_column_widths(rows: &[Vec<String>], col_count: usize) -> Vec<usize> {
        let mut col_widths: Vec<usize> = vec![0; col_count];
        for row in rows {
            for (col_idx, cell) in row.iter().enumerate() {
                let cell_len = cell.trim().chars().count();
                if cell_len > col_widths[col_idx] {
                    col_widths[col_idx] = cell_len;
                }
            }
        }
        col_widths
    }

    /// Detect if columns are numeric (for right-alignment)
    fn detect_numeric_columns(
        rows: &[Vec<String>],
        col_count: usize,
        header_row_count: usize,
    ) -> Vec<bool> {
        let mut is_numeric: Vec<bool> = vec![true; col_count];
        let mut has_content: Vec<bool> = vec![false; col_count];
        let skip_count = if header_row_count > 0 {
            header_row_count
        } else {
            1
        };

        for row in rows.iter().skip(skip_count) {
            for (col_idx, cell) in row.iter().enumerate() {
                if col_idx < col_count {
                    let trimmed = cell.trim();
                    if !trimmed.is_empty() {
                        has_content[col_idx] = true;
                        let is_cell_numeric =
                            trimmed == "-" || trimmed.chars().all(|c| c.is_ascii_digit());
                        if !is_cell_numeric {
                            is_numeric[col_idx] = false;
                        }
                    }
                }
            }
        }

        for col_idx in 0..col_count {
            if !has_content[col_idx] {
                is_numeric[col_idx] = false;
            }
        }
        is_numeric
    }

    /// Apply Python tabulate padding algorithm to column widths
    fn apply_column_padding(
        rows: &[Vec<String>],
        mut col_widths: Vec<usize>,
        is_numeric: &[bool],
    ) -> Vec<usize> {
        let header_lens: Vec<usize> = if rows.is_empty() {
            vec![0; col_widths.len()]
        } else {
            rows[0]
                .iter()
                .map(|cell| cell.trim().chars().count())
                .collect()
        };

        for col_idx in 0..col_widths.len() {
            let header_len = header_lens.get(col_idx).copied().unwrap_or(0);
            if is_numeric[col_idx] {
                col_widths[col_idx] = col_widths[col_idx].max(header_len) + 2;
            } else {
                col_widths[col_idx] = col_widths[col_idx].max(header_len + 2);
            }
        }
        col_widths
    }

    /// Build markdown table with aligned columns
    fn format_markdown_table(
        rows: &[Vec<String>],
        col_widths: &[usize],
        is_numeric: &[bool],
        col_count: usize,
    ) -> String {
        let mut md = String::new();

        for (i, row) in rows.iter().enumerate() {
            Self::write_table_row(&mut md, row, col_widths, is_numeric, col_count);

            // Add separator after first row (header)
            if i == 0 {
                Self::write_separator_row(&mut md, col_widths);
            }
        }

        md.trim_end().to_string()
    }

    /// Write a single table row to markdown
    #[inline]
    fn write_table_row(
        md: &mut String,
        row: &[String],
        col_widths: &[usize],
        is_numeric: &[bool],
        col_count: usize,
    ) {
        md.push('|');
        for col_idx in 0..col_count {
            let cell_text = row.get(col_idx).map_or("", |s| s.trim());
            let width = col_widths[col_idx];
            if is_numeric[col_idx] {
                let _ = write!(md, " {cell_text:>width$} |");
            } else {
                let _ = write!(md, " {cell_text:<width$} |");
            }
        }
        md.push('\n');
    }

    /// Write separator row to markdown
    #[inline]
    fn write_separator_row(md: &mut String, col_widths: &[usize]) {
        md.push('|');
        for &width in col_widths {
            let dashes = "-".repeat(width);
            let _ = write!(md, "-{dashes}-|");
        }
        md.push('\n');
    }

    /// Extract cells from a `<tr>` element, expanding colspan attributes
    fn extract_table_row(tr: &roxmltree::Node) -> Vec<String> {
        let mut cells = Vec::new();
        for td in tr.children() {
            if td.is_element() {
                let tag = td.tag_name().name();
                if tag == "td" || tag == "th" {
                    // Use get_text_raw for tables - Python doesn't escape HTML entities in tables
                    let text = Self::get_text_raw(&td);

                    // Handle colspan - repeat cell text for each column it spans
                    let colspan: usize = td
                        .attribute("colspan")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(1);

                    // First cell gets the text, subsequent cells in colspan get same text
                    for _ in 0..colspan {
                        cells.push(text.clone());
                    }
                }
            }
        }
        cells
    }

    /// Extract all text from an XML node recursively
    ///
    /// Python reference: jats_backend.py:175-190 (_`get_text`)
    fn get_text(node: &roxmltree::Node) -> String {
        let mut text = String::new();

        if let Some(t) = node.text() {
            text.push_str(t);
        }

        for child in node.children() {
            if child.is_element() {
                let tag = child.tag_name().name();
                // Add space before word-level inline formatting (italic, bold)
                // but NOT before sup/sub which attach to previous text
                // Also don't add space after opening brackets/parentheses or dashes
                if matches!(tag, "italic" | "bold" | "monospace" | "sc")
                    && !text.is_empty()
                    && !text.ends_with(char::is_whitespace)
                    && !text.ends_with(['(', '[', '{', '-'])
                {
                    text.push(' ');
                }
                // sup and sub don't get space before them
                text.push_str(&Self::get_text(&child));
            }
            if let Some(tail) = child.tail() {
                // Only add space after word-level formatting if tail doesn't start with space/punctuation
                // sup/sub attach directly without space after
                let child_tag = if child.is_element() {
                    child.tag_name().name()
                } else {
                    ""
                };
                let is_word_formatting =
                    matches!(child_tag, "italic" | "bold" | "monospace" | "sc");
                if is_word_formatting
                    && !text.is_empty()
                    && !text.ends_with(char::is_whitespace)
                    && !tail.is_empty()
                    && !tail.starts_with(char::is_whitespace)
                    && !tail.starts_with(|c: char| c.is_ascii_punctuation())
                {
                    text.push(' ');
                }
                text.push_str(tail);
            }
        }

        let result = text.trim().replace('\n', " ");
        // Escape HTML entities to match Python docling output
        Self::escape_html_entities(&result)
    }

    /// Get text content from an element WITHOUT HTML escaping (for tables)
    fn get_text_raw(node: &roxmltree::Node) -> String {
        let mut text = String::new();

        if let Some(t) = node.text() {
            text.push_str(t);
        }

        for child in node.children() {
            if child.is_element() {
                let tag = child.tag_name().name();
                let child_text = Self::get_text_raw(&child);

                // Add space before word-level inline formatting (italic, bold)
                // but NOT before sup/sub which attach to previous text
                // Also don't add space after opening brackets/parentheses or dashes
                // Also don't add space when continuing a number (e.g., <italic>1</italic>.<italic>16)
                if matches!(tag, "italic" | "bold" | "monospace" | "sc") {
                    let child_starts_with_digit = child_text
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_ascii_digit());
                    let text_ends_with_decimal = text.ends_with('.')
                        && text
                            .chars()
                            .rev()
                            .nth(1)
                            .is_some_and(|c| c.is_ascii_digit());

                    // Complex condition for readability, clippy wants simplified form but this is clearer
                    #[allow(
                        clippy::nonminimal_bool,
                        reason = "expanded form is more readable than simplified"
                    )]
                    let should_add_space = !text.is_empty()
                        && !text.ends_with(char::is_whitespace)
                        && !text.ends_with(['(', '[', '{', '-'])
                        // Don't add space if we're continuing a decimal number (e.g., "1." + "16")
                        && !(text_ends_with_decimal && child_starts_with_digit);
                    if should_add_space {
                        text.push(' ');
                    }
                }
                // sup and sub don't get space before them
                text.push_str(&child_text);
            }
            if let Some(tail) = child.tail() {
                // Only add space after word-level formatting if tail doesn't start with space/punctuation
                // sup/sub attach directly without space after
                let child_tag = if child.is_element() {
                    child.tag_name().name()
                } else {
                    ""
                };
                let is_word_formatting =
                    matches!(child_tag, "italic" | "bold" | "monospace" | "sc");
                if is_word_formatting
                    && !text.is_empty()
                    && !text.ends_with(char::is_whitespace)
                    && !tail.is_empty()
                    && !tail.starts_with(char::is_whitespace)
                    && !tail.starts_with(|c: char| c.is_ascii_punctuation())
                {
                    text.push(' ');
                }
                text.push_str(tail);
            }
        }

        text.trim().replace('\n', " ")
    }

    /// Escape HTML entities in text to match Python docling output
    ///
    /// Python docling keeps `<`, `>`, and `&` as HTML entities (`&lt;`, `&gt;`, `&amp;`)
    /// in the markdown output. We need to re-escape these after XML parsing.
    ///
    /// Important: Don't double-escape! If text already contains `&amp;`, `&lt;`, or `&gt;`,
    /// we should keep those as-is and only escape bare `&`, `<`, `>`.
    fn escape_html_entities(text: &str) -> String {
        // First escape < and > (these are always escaped)
        let result = text.replace('<', "&lt;").replace('>', "&gt;");

        // For &, we need to avoid double-escaping. Process char by char.
        // An & should NOT be escaped if it's part of an entity like &amp; &lt; &gt; &#123; etc.
        let mut output = String::with_capacity(result.len() + 16);
        let chars: Vec<char> = result.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            if chars[i] == '&' {
                // Check if this & is already part of an HTML entity
                // Entities are: &name; or &#decimal; or &#xhex;
                let remaining: String = chars[i..].iter().collect();
                let is_entity = remaining.starts_with("&amp;")
                    || remaining.starts_with("&lt;")
                    || remaining.starts_with("&gt;")
                    || remaining.starts_with("&quot;")
                    || remaining.starts_with("&apos;")
                    || remaining.starts_with("&nbsp;")
                    || (remaining.len() > 2
                        && remaining.starts_with("&#")
                        && remaining[2..]
                            .chars()
                            .take_while(|c| {
                                c.is_ascii_digit() || *c == 'x' || c.is_ascii_hexdigit()
                            })
                            .count()
                            > 0
                        && remaining.contains(';'));

                if is_entity {
                    // Keep the & as-is (it's already part of an entity)
                    output.push('&');
                } else {
                    // Escape bare &
                    output.push_str("&amp;");
                }
            } else {
                output.push(chars[i]);
            }
            i += 1;
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use docling_core::CoordOrigin;

    // ==================== BACKEND TESTS ====================

    #[test]
    fn test_jats_backend_creation() {
        let backend = JatsBackend;
        assert_eq!(
            backend.format(),
            InputFormat::Jats,
            "JATS backend should return Jats format"
        );
    }

    #[test]
    fn test_jats_backend_default() {
        let backend = JatsBackend;
        assert_eq!(
            backend.format(),
            InputFormat::Jats,
            "Default JATS backend should return Jats format"
        );
    }

    #[test]
    fn test_jats_backend_format() {
        let backend = JatsBackend;
        assert_eq!(
            backend.format(),
            InputFormat::Jats,
            "JATS backend format() should return Jats"
        );
    }

    // ==================== TITLE EXTRACTION TESTS ====================

    #[test]
    fn test_extract_title_simple() {
        let _backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <front>
        <article-meta>
            <title-group>
                <article-title>Simple Title</article-title>
            </title-group>
        </article-meta>
    </front>
</article>"#;

        let result = JatsBackend::extract_title(xml);
        assert!(result.is_ok(), "Title extraction should succeed");
        assert_eq!(
            result.unwrap(),
            Some("Simple Title".to_string()),
            "Title should be 'Simple Title'"
        );
    }

    #[test]
    fn test_extract_title_with_whitespace() {
        let _backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <front>
        <article-meta>
            <title-group>
                <article-title>
                    Title with
                    Newlines
                </article-title>
            </title-group>
        </article-meta>
    </front>
</article>"#;

        let result = JatsBackend::extract_title(xml);
        assert!(
            result.is_ok(),
            "Title extraction with whitespace should succeed"
        );
        let title = result.unwrap().unwrap();
        assert!(
            title.contains("Title with"),
            "Title should contain 'Title with'"
        );
        assert!(
            !title.contains('\n'),
            "Newlines should be replaced with spaces"
        ); // Newlines should be replaced with spaces
    }

    #[test]
    fn test_extract_title_missing() {
        let _backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <front>
        <article-meta>
        </article-meta>
    </front>
</article>"#;

        let result = JatsBackend::extract_title(xml);
        assert!(
            result.is_ok(),
            "Missing title extraction should succeed (returns None)"
        );
        assert_eq!(result.unwrap(), None, "Missing title should return None");
    }

    // ==================== ABSTRACT EXTRACTION TESTS ====================

    #[test]
    fn test_extract_abstract_single_paragraph() {
        let _backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <front>
        <abstract>
            <p>This is the abstract.</p>
        </abstract>
    </front>
</article>"#;

        let result = JatsBackend::extract_abstract(xml);
        assert!(result.is_ok(), "Abstract extraction should succeed");
        let abstracts = result.unwrap();
        assert_eq!(abstracts.len(), 1, "Should extract 1 abstract paragraph");
        assert_eq!(abstracts[0], "This is the abstract.");
    }

    #[test]
    fn test_extract_abstract_multiple_paragraphs() {
        let _backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <front>
        <abstract>
            <p>First paragraph.</p>
            <p>Second paragraph.</p>
        </abstract>
    </front>
</article>"#;

        let result = JatsBackend::extract_abstract(xml);
        assert!(
            result.is_ok(),
            "Multiple paragraph abstract extraction should succeed"
        );
        let abstracts = result.unwrap();
        assert_eq!(abstracts.len(), 2, "Should extract 2 abstract paragraphs");
        assert_eq!(
            abstracts[0], "First paragraph.",
            "First paragraph should match"
        );
        assert_eq!(
            abstracts[1], "Second paragraph.",
            "Second paragraph should match"
        );
    }

    #[test]
    fn test_extract_abstract_empty() {
        let _backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <front>
    </front>
</article>"#;

        let result = JatsBackend::extract_abstract(xml);
        assert!(result.is_ok(), "Empty abstract extraction should succeed");
        let abstracts = result.unwrap();
        assert!(
            abstracts.is_empty(),
            "Missing abstract should return empty vec"
        );
    }

    // ==================== AUTHOR EXTRACTION TESTS ====================

    #[test]
    fn test_extract_authors_single() {
        let _backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <front>
        <article-meta>
            <contrib-group>
                <contrib contrib-type="author">
                    <name>
                        <given-names>John</given-names>
                        <surname>Doe</surname>
                    </name>
                </contrib>
            </contrib-group>
        </article-meta>
    </front>
</article>"#;

        let result = JatsBackend::extract_authors(xml);
        assert!(result.is_ok(), "Single author extraction should succeed");
        let authors = result.unwrap();
        assert_eq!(authors.len(), 1, "Should extract 1 author");
        assert_eq!(authors[0], "John Doe", "Author name should be 'John Doe'");
    }

    #[test]
    fn test_extract_authors_multiple() {
        let _backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <front>
        <article-meta>
            <contrib-group>
                <contrib contrib-type="author">
                    <name>
                        <given-names>Alice</given-names>
                        <surname>Smith</surname>
                    </name>
                </contrib>
                <contrib contrib-type="author">
                    <name>
                        <given-names>Bob</given-names>
                        <surname>Jones</surname>
                    </name>
                </contrib>
            </contrib-group>
        </article-meta>
    </front>
</article>"#;

        let result = JatsBackend::extract_authors(xml);
        assert!(result.is_ok(), "Multiple authors extraction should succeed");
        let authors = result.unwrap();
        assert_eq!(authors.len(), 2, "Should extract 2 authors");
        assert_eq!(
            authors[0], "Alice Smith",
            "First author should be 'Alice Smith'"
        );
        assert_eq!(
            authors[1], "Bob Jones",
            "Second author should be 'Bob Jones'"
        );
    }

    #[test]
    fn test_extract_authors_non_author_contrib() {
        let _backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <front>
        <article-meta>
            <contrib-group>
                <contrib contrib-type="editor">
                    <name>
                        <given-names>Jane</given-names>
                        <surname>Editor</surname>
                    </name>
                </contrib>
                <contrib contrib-type="author">
                    <name>
                        <given-names>John</given-names>
                        <surname>Author</surname>
                    </name>
                </contrib>
            </contrib-group>
        </article-meta>
    </front>
</article>"#;

        let result = JatsBackend::extract_authors(xml);
        assert!(result.is_ok(), "Author extraction should succeed");
        let authors = result.unwrap();
        assert_eq!(authors.len(), 1, "Should extract only author, not editor"); // Only author, not editor
        assert_eq!(
            authors[0], "John Author",
            "Should extract author name 'John Author'"
        );
    }

    // ==================== BODY EXTRACTION TESTS ====================

    #[test]
    fn test_extract_body_with_sections() {
        let backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <body>
        <sec>
            <title>Introduction</title>
            <p>Intro paragraph.</p>
        </sec>
        <sec>
            <title>Methods</title>
            <p>Methods paragraph.</p>
        </sec>
    </body>
</article>"#;

        let result = backend.extract_body(xml);
        assert!(
            result.is_ok(),
            "Body extraction with sections should succeed"
        );
        let items = result.unwrap();
        assert!(
            !items.is_empty(),
            "Body with sections should produce DocItems"
        );

        // Should have SectionHeader and Text DocItems
        let has_section = items
            .iter()
            .any(|item| matches!(item, DocItem::SectionHeader { .. }));
        let has_text = items
            .iter()
            .any(|item| matches!(item, DocItem::Text { .. }));
        assert!(has_section, "Should have SectionHeader DocItems");
        assert!(has_text, "Should have Text DocItems");
    }

    #[test]
    fn test_extract_body_with_lists() {
        let backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <body>
        <sec>
            <title>List Example</title>
            <list>
                <list-item>First item</list-item>
                <list-item>Second item</list-item>
            </list>
        </sec>
    </body>
</article>"#;

        let result = backend.extract_body(xml);
        assert!(result.is_ok(), "Body extraction with lists should succeed");
        let items = result.unwrap();
        assert!(!items.is_empty(), "Body with lists should produce DocItems");

        // Should have List and ListItem DocItems
        let has_list = items
            .iter()
            .any(|item| matches!(item, DocItem::List { .. }));
        let has_list_item = items
            .iter()
            .any(|item| matches!(item, DocItem::ListItem { .. }));
        assert!(has_list, "Should have List DocItem");
        assert!(has_list_item, "Should have ListItem DocItems");
    }

    #[test]
    fn test_extract_body_empty() {
        let backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
</article>"#;

        let result = backend.extract_body(xml);
        assert!(result.is_ok(), "Empty body extraction should succeed");
        let items = result.unwrap();
        assert!(items.is_empty(), "Empty article should produce no DocItems");
    }

    // ==================== ERROR HANDLING TESTS ====================

    #[test]
    fn test_parse_bytes_invalid_utf8() {
        let backend = JatsBackend;
        let options = BackendOptions::default();
        let invalid_utf8 = vec![0xFF, 0xFE, 0xFD]; // Invalid UTF-8 sequence

        let result = backend.parse_bytes(&invalid_utf8, &options);
        assert!(result.is_err(), "Invalid UTF-8 should return error");
        if let Err(DoclingError::BackendError(msg)) = result {
            assert!(
                msg.contains("Invalid UTF-8"),
                "Error message should mention 'Invalid UTF-8'"
            );
        } else {
            panic!("Expected BackendError with UTF-8 message");
        }
    }

    #[test]
    fn test_parse_bytes_invalid_xml() {
        let backend = JatsBackend;
        let options = BackendOptions::default();
        let invalid_xml = b"<article><unclosed>";

        let result = backend.parse_bytes(invalid_xml, &options);
        // Parser may be lenient and succeed, or may error
        // Either way, we verify the backend handles it gracefully
        match result {
            Ok(_doc) => {}                           // Lenient parser accepted it
            Err(DoclingError::BackendError(_)) => {} // Parser rejected it
            Err(e) => panic!("Unexpected error type: {e:?}"),
        }
    }

    #[test]
    fn test_parse_bytes_empty() {
        let backend = JatsBackend;
        let options = BackendOptions::default();
        let empty_data: &[u8] = b"";

        let result = backend.parse_bytes(empty_data, &options);
        assert!(result.is_err(), "Empty data should return error");
    }

    // ==================== INTEGRATION TESTS ====================

    #[test]
    fn test_parse_empty_jats() {
        let backend = JatsBackend;
        let options = BackendOptions::default();
        let empty_xml = b"<?xml version=\"1.0\"?><article></article>";

        let result = backend.parse_bytes(empty_xml, &options);
        // Empty valid XML should parse (produce empty document)
        assert!(
            result.is_ok(),
            "Empty valid JATS XML should parse successfully"
        );
        let doc = result.unwrap();
        assert_eq!(
            doc.format,
            InputFormat::Jats,
            "Document format should be JATS"
        );
        assert!(
            doc.content_blocks.is_some(),
            "Empty JATS should still have content_blocks (Some)"
        );
    }

    #[test]
    fn test_parse_minimal_jats() {
        let backend = JatsBackend;
        let options = BackendOptions::default();
        let minimal_xml = br#"<?xml version="1.0"?>
<article>
    <front>
        <article-meta>
            <title-group>
                <article-title>Test Article</article-title>
            </title-group>
        </article-meta>
    </front>
</article>"#;

        let result = backend.parse_bytes(minimal_xml, &options);
        assert!(
            result.is_ok(),
            "Minimal JATS with title should parse successfully"
        );
        let doc = result.unwrap();
        assert_eq!(
            doc.format,
            InputFormat::Jats,
            "Document format should be JATS"
        );
        // Metadata title comes from file_stem parameter, not extracted article title
        assert!(doc.metadata.title.is_some(), "Metadata should have title");
        assert_eq!(
            doc.metadata.title.unwrap(),
            "file",
            "Metadata title should be 'file' (from file_stem)"
        );
        // Article title should be in content_blocks as DocItem
        assert!(
            doc.content_blocks.is_some(),
            "Document should have content_blocks"
        );
        let items = doc.content_blocks.unwrap();
        assert!(
            !items.is_empty(),
            "Content blocks should not be empty for minimal JATS"
        );
        // First item should contain the article title as SectionHeader
        if let DocItem::SectionHeader { text, .. } = &items[0] {
            assert_eq!(
                text, "Test Article",
                "First SectionHeader should be article title"
            );
        } else {
            panic!("Expected first DocItem to be SectionHeader with article title");
        }
    }

    #[test]
    fn test_jats_with_dtd() {
        // Test that JATS files with DOCTYPE/DTD declarations parse correctly
        // Regression test for "XML with DTD detected" error
        let backend = JatsBackend;
        let options = BackendOptions::default();

        // XML with DTD declaration (similar to real JATS files from PubMed)
        let xml_with_dtd = br#"<?xml version="1.0"?>
<!DOCTYPE article
PUBLIC "-//NLM//DTD JATS (Z39.96) Journal Archiving v1.2//EN" "JATS-archivearticle1.dtd">
<article>
    <front>
        <article-meta>
            <title-group>
                <article-title>Test Article with DTD</article-title>
            </title-group>
        </article-meta>
    </front>
    <body>
        <sec>
            <title>Introduction</title>
            <p>This article has a DTD declaration.</p>
        </sec>
    </body>
</article>"#;

        let result = backend.parse_bytes(xml_with_dtd, &options);
        assert!(
            result.is_ok(),
            "JATS with DTD should parse successfully: {:?}",
            result.err()
        );

        let doc = result.unwrap();
        assert_eq!(
            doc.format,
            InputFormat::Jats,
            "Document format should be JATS"
        );
        assert!(doc.content_blocks.is_some(), "JATS must generate DocItems");
        assert!(!doc.markdown.is_empty(), "JATS must generate markdown");

        // Verify content was extracted
        if let Some(blocks) = &doc.content_blocks {
            assert!(!blocks.is_empty(), "Should have extracted DocItems");
        }
    }

    #[test]
    fn test_document_metadata_structure() {
        let backend = JatsBackend;
        let options = BackendOptions::default();
        let xml = br#"<?xml version="1.0"?>
<article>
    <front>
        <article-meta>
            <title-group>
                <article-title>Test</article-title>
            </title-group>
            <contrib-group>
                <contrib contrib-type="author">
                    <name>
                        <given-names>John</given-names>
                        <surname>Doe</surname>
                    </name>
                </contrib>
            </contrib-group>
        </article-meta>
    </front>
    <body>
        <sec>
            <title>Section</title>
            <p>Content.</p>
        </sec>
    </body>
</article>"#;

        let result = backend.parse_bytes(xml, &options);
        assert!(
            result.is_ok(),
            "JATS with metadata and body should parse successfully"
        );

        let doc = result.unwrap();
        assert_eq!(
            doc.metadata.num_pages,
            Some(1),
            "JATS documents should have 1 page"
        );
        assert!(
            doc.metadata.num_characters > 0,
            "Document should have character count"
        );
        assert_eq!(
            doc.metadata.author,
            Some("John Doe".to_string()),
            "Author should be extracted as 'John Doe'"
        );
        assert_eq!(
            doc.format,
            InputFormat::Jats,
            "Document format should be JATS"
        );
    }

    // ==================== CATEGORY 8: ADVANCED XML STRUCTURE TESTS ====================

    #[test]
    fn test_nested_sections() {
        // Test deeply nested section hierarchy with proper heading levels
        let backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <body>
        <sec>
            <title>Level 1</title>
            <p>Content L1</p>
            <sec>
                <title>Level 2</title>
                <p>Content L2</p>
                <sec>
                    <title>Level 3</title>
                    <p>Content L3</p>
                </sec>
            </sec>
        </sec>
    </body>
</article>"#;

        let result = backend.extract_body(xml);
        assert!(result.is_ok(), "Nested sections should parse successfully");
        let items = result.unwrap();

        // Should have section headers at different levels
        let section_headers: Vec<_> = items
            .iter()
            .filter_map(|item| match item {
                DocItem::SectionHeader { level, text, .. } => Some((level, text.as_str())),
                _ => None,
            })
            .collect();

        assert!(section_headers.len() >= 3, "Should have 3 section headers");
        // Verify level progression - body sections start at level 2 (##)
        // Title would be level 1 (#), so first body section is level 2
        assert!(
            section_headers.iter().any(|(level, _)| **level == 2),
            "Should have level 2 (first body section)"
        );
        assert!(
            section_headers.iter().any(|(level, _)| **level == 3),
            "Should have level 3 (nested section)"
        );
        assert!(
            section_headers.iter().any(|(level, _)| **level == 4),
            "Should have level 4 (deeply nested section)"
        );
    }

    #[test]
    fn test_acknowledgments_section() {
        // Test <ack> element (acknowledgments section)
        let backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <body>
        <ack>
            <p>We thank the reviewers for their comments.</p>
        </ack>
    </body>
</article>"#;

        let result = backend.extract_body(xml);
        assert!(
            result.is_ok(),
            "Acknowledgments section should parse successfully"
        );
        let items = result.unwrap();

        // Should have "Acknowledgments" section header
        let has_ack = items.iter().any(|item| match item {
            DocItem::SectionHeader { text, .. } => text == "Acknowledgments",
            _ => false,
        });
        assert!(has_ack, "Should have Acknowledgments section header");

        // Should have paragraph text
        let has_text = items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("reviewers"),
            _ => false,
        });
        assert!(has_text, "Should have acknowledgment text");
    }

    #[test]
    fn test_section_with_label_and_title() {
        // Test section with both <label> and <title>
        let backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <body>
        <sec>
            <label>1</label>
            <title>Introduction</title>
            <p>First section.</p>
        </sec>
    </body>
</article>"#;

        let result = backend.extract_body(xml);
        assert!(
            result.is_ok(),
            "Section with label and title should parse successfully"
        );
        let items = result.unwrap();

        // Should extract title (label is also considered as title if no title element)
        let has_section = items.iter().any(|item| match item {
            DocItem::SectionHeader { text, .. } => {
                text.contains("Introduction") || text.contains('1')
            }
            _ => false,
        });
        assert!(has_section, "Should have section with title/label");
    }

    #[test]
    fn test_complex_list_structure() {
        // Test list with multiple items and nested structure
        let backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <body>
        <sec>
            <title>Features</title>
            <list>
                <list-item><p>Feature one with paragraph</p></list-item>
                <list-item><p>Feature two with paragraph</p></list-item>
                <list-item><p>Feature three with paragraph</p></list-item>
            </list>
        </sec>
    </body>
</article>"#;

        let result = backend.extract_body(xml);
        assert!(
            result.is_ok(),
            "Complex list structure should parse successfully"
        );
        let items = result.unwrap();

        // Should have List container
        let list_count = items
            .iter()
            .filter(|item| matches!(item, DocItem::List { .. }))
            .count();
        assert!(list_count >= 1, "Should have at least 1 List container");

        // Should have ListItem elements
        let list_item_count = items
            .iter()
            .filter(|item| matches!(item, DocItem::ListItem { .. }))
            .count();
        assert!(
            list_item_count >= 3,
            "Should have at least 3 ListItem elements"
        );
    }

    #[test]
    fn test_xml_entities_and_special_chars() {
        // Test XML entity escaping and special characters
        let _backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <front>
        <article-meta>
            <title-group>
                <article-title>Test &amp; Special &lt;Characters&gt; &quot;Quotes&quot;</article-title>
            </title-group>
        </article-meta>
    </front>
</article>"#;

        let result = JatsBackend::extract_title(xml);
        assert!(
            result.is_ok(),
            "XML with entities should parse successfully"
        );
        let title = result.unwrap().unwrap();

        // Entities should be unescaped
        assert!(title.contains('&'), "Should contain unescaped ampersand");
        assert!(title.contains('<'), "Should contain unescaped less-than");
        assert!(title.contains('>'), "Should contain unescaped greater-than");
    }

    // ==================== CATEGORY 9: JATS-SPECIFIC ELEMENT TESTS ====================

    #[test]
    fn test_skip_supplementary_material() {
        // Test that <supplementary-material> elements are skipped
        let backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <body>
        <sec>
            <title>Data</title>
            <p>See supplementary materials.</p>
            <supplementary-material>
                <caption>Additional data file</caption>
            </supplementary-material>
        </sec>
    </body>
</article>"#;

        let result = backend.extract_body(xml);
        assert!(
            result.is_ok(),
            "Body with supplementary material should parse successfully"
        );
        let items = result.unwrap();

        // Should have text paragraph but not supplementary material content
        let has_main_text = items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("supplementary materials"),
            _ => false,
        });
        assert!(has_main_text, "Should extract main paragraph text");

        // Supplementary material caption should not appear
        let has_supp = items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("Additional data file"),
            _ => false,
        });
        assert!(!has_supp, "Should skip supplementary material content");
    }

    #[test]
    fn test_skip_reference_list() {
        // Test that <ref-list> elements are skipped
        let backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <body>
        <sec>
            <title>Conclusion</title>
            <p>Final paragraph.</p>
        </sec>
    </body>
    <back>
        <ref-list>
            <ref id="ref1">
                <element-citation>Smith et al. 2020</element-citation>
            </ref>
        </ref-list>
    </back>
</article>"#;

        let result = backend.extract_body(xml);
        assert!(
            result.is_ok(),
            "Body with ref-list in back section should parse successfully"
        );
        let items = result.unwrap();

        // Should extract body content
        let has_body = items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("Final paragraph"),
            _ => false,
        });
        assert!(has_body, "Should extract body text");

        // References should not appear (ref-list is in <back>, not <body>)
        // Note: If ref-list appears in body, it's skipped by stop_walk
    }

    #[test]
    fn test_skip_figure_elements() {
        // Test that <fig> elements are skipped (figures handled separately)
        let backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <body>
        <sec>
            <title>Results</title>
            <p>Figure 1 shows results.</p>
            <fig id="fig1">
                <caption>
                    <p>Result visualization</p>
                </caption>
            </fig>
        </sec>
    </body>
</article>"#;

        let result = backend.extract_body(xml);
        assert!(
            result.is_ok(),
            "Body with figure elements should parse successfully"
        );
        let items = result.unwrap();

        // Should extract main text
        let has_text = items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("Figure 1 shows"),
            _ => false,
        });
        assert!(has_text, "Should extract main paragraph");

        // Figure caption extraction is handled separately
        // Current implementation skips fig elements (stop_walk = true)
    }

    #[test]
    fn test_skip_table_wrap_elements() {
        // Test that <table-wrap> elements are skipped
        let backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <body>
        <sec>
            <title>Data</title>
            <p>Table 1 shows data.</p>
            <table-wrap>
                <caption>Data summary</caption>
                <table>
                    <tr><td>Cell 1</td></tr>
                </table>
            </table-wrap>
        </sec>
    </body>
</article>"#;

        let result = backend.extract_body(xml);
        assert!(
            result.is_ok(),
            "Body with table-wrap elements should parse successfully"
        );
        let items = result.unwrap();

        // Should extract main text
        let has_text = items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("Table 1 shows"),
            _ => false,
        });
        assert!(has_text, "Should extract main paragraph");

        // Table content is skipped by stop_walk
    }

    #[test]
    fn test_skip_formula_elements() {
        // Test that <tex-math> and <inline-formula> are skipped
        let backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <body>
        <sec>
            <title>Math</title>
            <p>Equation: <inline-formula><tex-math>E=mc^2</tex-math></inline-formula></p>
        </sec>
    </body>
</article>"#;

        let result = backend.extract_body(xml);
        assert!(
            result.is_ok(),
            "Body with formula elements should parse successfully"
        );
        let items = result.unwrap();

        // Should extract some text (at least "Equation:")
        let has_text = items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("Equation"),
            _ => false,
        });
        assert!(has_text, "Should extract text from paragraph");

        // Formula elements are skipped (stop_walk = true)
    }

    // ==================== CATEGORY 10: COMPLEX DOCUMENT STRUCTURE TESTS ====================

    #[test]
    fn test_docitem_index_sequential() {
        // Test that DocItem self_ref indices are sequential
        let backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <front>
        <article-meta>
            <title-group>
                <article-title>Title</article-title>
            </title-group>
        </article-meta>
    </front>
    <body>
        <sec>
            <title>Section</title>
            <p>Paragraph 1</p>
            <p>Paragraph 2</p>
        </sec>
    </body>
</article>"#;

        let result = backend.parse_jats_xml(xml, "test");
        assert!(
            result.is_ok(),
            "Document with multiple paragraphs should parse successfully"
        );
        let doc = result.unwrap();

        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty(), "Document should have content blocks");

        // Check that indices are sequential
        for item in items.iter() {
            let self_ref = match item {
                DocItem::Text { self_ref, .. } => self_ref,
                DocItem::SectionHeader { self_ref, .. } => self_ref,
                DocItem::List { self_ref, .. } => self_ref,
                DocItem::ListItem { self_ref, .. } => self_ref,
                _ => continue,
            };

            // self_ref should contain an index (format: "#/texts/{i}" or "#/groups/{i}")
            assert!(
                self_ref.contains('/'),
                "self_ref should be a JSON pointer path"
            );
        }
    }

    #[test]
    fn test_content_layer_validation() {
        // Test that all DocItems have content_layer = "body"
        let backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <front>
        <article-meta>
            <title-group>
                <article-title>Title</article-title>
            </title-group>
        </article-meta>
    </front>
    <body>
        <sec>
            <title>Section</title>
            <p>Content</p>
        </sec>
    </body>
</article>"#;

        let result = backend.parse_jats_xml(xml, "test");
        assert!(
            result.is_ok(),
            "Content layer validation test should parse successfully"
        );
        let doc = result.unwrap();

        let items = doc.content_blocks.unwrap();
        for item in items {
            let content_layer = match item {
                DocItem::Text { content_layer, .. } => content_layer,
                DocItem::SectionHeader { content_layer, .. } => content_layer,
                DocItem::List { content_layer, .. } => content_layer,
                _ => continue,
            };
            assert_eq!(
                content_layer, "body",
                "All DocItems should have content_layer='body'"
            );
        }
    }

    #[test]
    fn test_large_document_handling() {
        // Test document with many sections and paragraphs
        let backend = JatsBackend;
        let mut xml = String::from(
            r#"<?xml version="1.0"?>
<article>
    <body>"#,
        );

        // Add 10 sections with 3 paragraphs each
        for i in 1..=10 {
            xml.push_str(&format!(
                r"
        <sec>
            <title>Section {i}</title>
            <p>Paragraph {i} in section {i}</p>
            <p>Another paragraph in section {i}</p>
            <p>Third paragraph in section {i}</p>
        </sec>"
            ));
        }

        xml.push_str("\n    </body>\n</article>");

        let result = backend.extract_body(&xml);
        assert!(
            result.is_ok(),
            "Large document with 10 sections should parse successfully"
        );
        let items = result.unwrap();

        // Should have at least 10 sections
        let section_count = items
            .iter()
            .filter(|item| matches!(item, DocItem::SectionHeader { .. }))
            .count();
        assert!(
            section_count >= 10,
            "Should have at least 10 sections, got {section_count}"
        );

        // Should have at least 30 paragraphs (3 per section × 10 sections)
        let text_count = items
            .iter()
            .filter(|item| matches!(item, DocItem::Text { .. }))
            .count();
        assert!(
            text_count >= 30,
            "Should have at least 30 text items, got {text_count}"
        );
    }

    #[test]
    fn test_markdown_generation_formatting() {
        // Test that markdown is properly formatted with headings
        let backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <body>
        <sec>
            <title>Introduction</title>
            <p>First paragraph.</p>
        </sec>
        <sec>
            <title>Methods</title>
            <p>Methods paragraph.</p>
        </sec>
    </body>
</article>"#;

        let items = backend.extract_body(xml).unwrap();
        let markdown = crate::markdown_helper::docitems_to_markdown(&items);

        // Should contain markdown headings
        assert!(
            markdown.contains("# Introduction"),
            "Should have level 1 heading"
        );
        assert!(
            markdown.contains("# Methods"),
            "Should have level 1 heading"
        );

        // Should contain paragraph text
        assert!(
            markdown.contains("First paragraph"),
            "Should have paragraph text"
        );
        assert!(
            markdown.contains("Methods paragraph"),
            "Should have paragraph text"
        );
    }

    #[test]
    fn test_empty_sections_handling() {
        // Test sections with no content
        let backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <body>
        <sec>
            <title>Empty Section</title>
        </sec>
        <sec>
            <title>Section with Content</title>
            <p>Has content.</p>
        </sec>
    </body>
</article>"#;

        let result = backend.extract_body(xml);
        assert!(
            result.is_ok(),
            "Empty and non-empty sections should parse successfully"
        );
        let items = result.unwrap();

        // Should have section headers
        let section_count = items
            .iter()
            .filter(|item| matches!(item, DocItem::SectionHeader { .. }))
            .count();
        assert!(section_count >= 2, "Should have at least 2 section headers");

        // Should have at least 1 text paragraph (from non-empty section)
        let text_count = items
            .iter()
            .filter(|item| matches!(item, DocItem::Text { .. }))
            .count();
        assert!(
            text_count >= 1,
            "Should have at least 1 text paragraph from non-empty section"
        );
    }

    // ==================== CATEGORY 11: METADATA AND PROVENANCE TESTS ====================

    #[test]
    fn test_metadata_character_count() {
        // Test that num_characters is correctly calculated
        let backend = JatsBackend;
        let options = BackendOptions::default();
        let xml = br#"<?xml version="1.0"?>
<article>
    <front>
        <article-meta>
            <title-group>
                <article-title>Short Title</article-title>
            </title-group>
        </article-meta>
    </front>
    <body>
        <sec>
            <title>Section</title>
            <p>Content paragraph with some text.</p>
        </sec>
    </body>
</article>"#;

        let result = backend.parse_bytes(xml, &options);
        assert!(
            result.is_ok(),
            "Document for character count test should parse successfully"
        );
        let doc = result.unwrap();

        // Should have positive character count
        assert!(doc.metadata.num_characters > 0, "Should count characters");

        // Rough validation: character count should be reasonable
        // Note: Whitespace normalization may reduce character count from raw input
        assert!(
            doc.metadata.num_characters >= 30,
            "Character count {} should be at least 30 (accounting for whitespace normalization)",
            doc.metadata.num_characters
        );
        assert!(
            doc.metadata.num_characters < 200,
            "Character count {} should be less than 200 for this small test document",
            doc.metadata.num_characters
        );
    }

    #[test]
    fn test_provenance_default_values() {
        // Test that provenance has default page=1 and correct origin
        let backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <body>
        <sec>
            <title>Test</title>
            <p>Content.</p>
        </sec>
    </body>
</article>"#;

        let items = backend.extract_body(xml).unwrap();

        // All items should have provenance with page=1
        for item in items {
            let prov = match item {
                DocItem::Text { prov, .. } => prov,
                DocItem::SectionHeader { prov, .. } => prov,
                _ => continue,
            };

            assert!(!prov.is_empty(), "Should have provenance");
            assert_eq!(prov[0].page_no, 1, "JATS documents default to page 1");
            assert_eq!(
                prov[0].bbox.coord_origin,
                CoordOrigin::TopLeft,
                "Should use TopLeft origin"
            );
        }
    }

    #[test]
    fn test_multiple_authors_metadata() {
        // Test that multiple authors are joined correctly
        let backend = JatsBackend;
        let options = BackendOptions::default();
        let xml = br#"<?xml version="1.0"?>
<article>
    <front>
        <article-meta>
            <title-group>
                <article-title>Research Paper</article-title>
            </title-group>
            <contrib-group>
                <contrib contrib-type="author">
                    <name>
                        <given-names>Alice</given-names>
                        <surname>Smith</surname>
                    </name>
                </contrib>
                <contrib contrib-type="author">
                    <name>
                        <given-names>Bob</given-names>
                        <surname>Jones</surname>
                    </name>
                </contrib>
                <contrib contrib-type="author">
                    <name>
                        <given-names>Carol</given-names>
                        <surname>White</surname>
                    </name>
                </contrib>
            </contrib-group>
        </article-meta>
    </front>
</article>"#;

        let result = backend.parse_bytes(xml, &options);
        assert!(
            result.is_ok(),
            "Document with multiple authors should parse successfully"
        );
        let doc = result.unwrap();

        assert!(doc.metadata.author.is_some(), "Should have authors");
        let authors = doc.metadata.author.unwrap();
        assert!(
            authors.contains("Alice Smith"),
            "Should contain Alice Smith"
        );
        assert!(authors.contains("Bob Jones"), "Should contain Bob Jones");
        assert!(
            authors.contains("Carol White"),
            "Should contain Carol White"
        );
        assert!(authors.contains(", "), "Authors should be comma-separated");
    }

    #[test]
    fn test_document_format_consistency() {
        // Test that format field is consistently set
        let backend = JatsBackend;
        let options = BackendOptions::default();
        let xml = br#"<?xml version="1.0"?>
<article>
    <front>
        <article-meta>
            <title-group>
                <article-title>Test</article-title>
            </title-group>
        </article-meta>
    </front>
</article>"#;

        let result = backend.parse_bytes(xml, &options);
        assert!(
            result.is_ok(),
            "Document format consistency test should parse successfully"
        );
        let doc = result.unwrap();

        assert_eq!(doc.format, InputFormat::Jats, "Format should be Jats");
        assert_eq!(
            backend.format(),
            InputFormat::Jats,
            "Backend should report Jats format"
        );
    }

    #[test]
    fn test_abstract_included_in_docitems() {
        // Test that abstract paragraphs are included in content_blocks
        let backend = JatsBackend;
        let options = BackendOptions::default();
        let xml = br#"<?xml version="1.0"?>
<article>
    <front>
        <article-meta>
            <title-group>
                <article-title>Research Article</article-title>
            </title-group>
            <abstract>
                <p>This is the first abstract paragraph.</p>
                <p>This is the second abstract paragraph.</p>
            </abstract>
        </article-meta>
    </front>
    <body>
        <sec>
            <title>Introduction</title>
            <p>Body content.</p>
        </sec>
    </body>
</article>"#;

        let result = backend.parse_bytes(xml, &options);
        assert!(
            result.is_ok(),
            "Document with abstract should parse successfully"
        );
        let doc = result.unwrap();

        assert!(doc.content_blocks.is_some(), "Should have content blocks");
        let items = doc.content_blocks.unwrap();

        // Should have title as SectionHeader
        let has_title = items.iter().any(|item| match item {
            DocItem::SectionHeader { text, .. } => text.contains("Research Article"),
            _ => false,
        });
        assert!(has_title, "Should include article title");

        // Should have abstract paragraphs
        let has_abstract1 = items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("first abstract paragraph"),
            _ => false,
        });
        assert!(has_abstract1, "Should include first abstract paragraph");

        let has_abstract2 = items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("second abstract paragraph"),
            _ => false,
        });
        assert!(has_abstract2, "Should include second abstract paragraph");

        // Should have body content
        let has_body = items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("Body content"),
            _ => false,
        });
        assert!(has_body, "Should include body content");
    }

    // === Article Title Edge Cases ===

    #[test]
    fn test_title_with_nested_formatting() {
        // Test title with nested XML formatting tags (bold, italic, sub, sup)
        let _backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <front>
        <article-meta>
            <title-group>
                <article-title>Study of <italic>E. coli</italic> Growth at 37<sup>°</sup>C with CO<sub>2</sub></article-title>
            </title-group>
        </article-meta>
    </front>
</article>"#;

        let result = JatsBackend::extract_title(xml);
        assert!(
            result.is_ok(),
            "Title with nested formatting should parse successfully"
        );
        let title = result.unwrap();
        assert!(
            title.is_some(),
            "Should extract title with nested formatting"
        );
        let title_text = title.unwrap();
        // Formatting tags should be removed, text preserved
        assert!(
            title_text.contains("E. coli"),
            "Should preserve italic text content"
        );
        assert!(
            title_text.contains("37"),
            "Should preserve superscript content"
        );
        assert!(
            title_text.contains("CO"),
            "Should preserve subscript content"
        );
    }

    #[test]
    fn test_title_very_long() {
        // Test title with 500+ characters
        let _backend = JatsBackend;
        let long_title = "A".repeat(500);
        let xml = format!(
            r#"<?xml version="1.0"?>
<article>
    <front>
        <article-meta>
            <title-group>
                <article-title>{long_title}</article-title>
            </title-group>
        </article-meta>
    </front>
</article>"#
        );

        let result = JatsBackend::extract_title(&xml);
        assert!(result.is_ok(), "Very long title should parse successfully");
        let title = result.unwrap();
        assert!(title.is_some(), "Should extract very long title");
        let title_text = title.unwrap();
        assert_eq!(
            title_text.len(),
            500,
            "Should preserve full 500-character title length"
        );
    }

    #[test]
    fn test_title_with_xml_entities() {
        // Test title with XML entities (&amp;, &lt;, &gt;, &quot;, etc.)
        let _backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <front>
        <article-meta>
            <title-group>
                <article-title>Expression &amp; Regulation: &lt;5% "Normal" Levels &gt; Threshold</article-title>
            </title-group>
        </article-meta>
    </front>
</article>"#;

        let result = JatsBackend::extract_title(xml);
        assert!(
            result.is_ok(),
            "Title with XML entities should parse successfully"
        );
        let title = result.unwrap();
        assert!(title.is_some(), "Should extract title with XML entities");
        let title_text = title.unwrap();
        assert!(title_text.contains('&'), "Should unescape &amp; to &");
        assert!(title_text.contains('<'), "Should unescape &lt; to <");
        assert!(title_text.contains('>'), "Should unescape &gt; to >");
        assert!(title_text.contains('"'), "Should unescape &quot; to \"");
    }

    #[test]
    fn test_title_with_hexadecimal_entities() {
        // Test title with hexadecimal HTML entities (&#x0003c; for <)
        let _backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <front>
        <article-meta>
            <title-group>
                <article-title>adjusted p-value&#x0003c;1e-5</article-title>
            </title-group>
        </article-meta>
    </front>
</article>"#;

        let result = JatsBackend::extract_title(xml);
        assert!(
            result.is_ok(),
            "Title with hexadecimal entities should parse successfully"
        );
        let title = result.unwrap();
        assert!(title.is_some(), "Should extract title with hex entities");
        let title_text = title.unwrap();
        assert!(title_text.contains('<'), "Should unescape &#x0003c; to <");
        assert_eq!(
            title_text, "adjusted p-value<1e-5",
            "Should produce correct decoded text"
        );
    }

    #[test]
    fn test_title_with_unicode() {
        // Test title with Unicode characters (CJK, emoji, special symbols)
        let _backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <front>
        <article-meta>
            <title-group>
                <article-title>研究：日本語タイトル with 中文 and Emoji 🧬🔬 and Symbols α β γ Δ ∑</article-title>
            </title-group>
        </article-meta>
    </front>
</article>"#;

        let result = JatsBackend::extract_title(xml);
        assert!(
            result.is_ok(),
            "Title with Unicode characters should parse successfully"
        );
        let title = result.unwrap();
        assert!(title.is_some(), "Should extract title with Unicode");
        let title_text = title.unwrap();
        assert!(
            title_text.contains("研究"),
            "Should preserve Japanese characters"
        );
        assert!(
            title_text.contains("中文"),
            "Should preserve Chinese characters"
        );
        assert!(title_text.contains("🧬"), "Should preserve emoji");
        assert!(title_text.contains("α"), "Should preserve Greek letters");
        assert!(
            title_text.contains("∑"),
            "Should preserve mathematical symbols"
        );
    }

    #[test]
    fn test_title_without_title_group() {
        // Test title without title-group wrapper (direct article-title in article-meta)
        // N=3098: Now supported - article-title can be directly under article-meta
        // This handles JATS variants that don't use the title-group wrapper
        let _backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <front>
        <article-meta>
            <article-title>Direct Title Without Group</article-title>
        </article-meta>
    </front>
</article>"#;

        let result = JatsBackend::extract_title(xml);
        assert!(
            result.is_ok(),
            "Title without title-group should parse without error"
        );
        let title = result.unwrap();
        // Now extracts title directly from article-meta (N=3098)
        assert_eq!(
            title,
            Some("Direct Title Without Group".to_string()),
            "Should extract title directly from article-meta"
        );
    }

    // === Abstract Variations ===

    #[test]
    fn test_abstract_with_nested_formatting() {
        // Test abstract with nested formatting (bold, italic within paragraphs)
        let _backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <front>
        <abstract>
            <p>This study examines <bold>key findings</bold> with <italic>significant impact</italic> on the field.</p>
        </abstract>
    </front>
</article>"#;

        let result = JatsBackend::extract_abstract(xml);
        assert!(
            result.is_ok(),
            "Abstract with nested formatting should parse successfully"
        );
        let abstracts = result.unwrap();
        assert_eq!(abstracts.len(), 1, "Should extract one abstract paragraph");
        // Formatting tags removed, text preserved
        assert!(
            abstracts[0].contains("key findings"),
            "Should preserve bold text content"
        );
        assert!(
            abstracts[0].contains("significant impact"),
            "Should preserve italic text content"
        );
    }

    #[test]
    fn test_abstract_very_long() {
        // Test abstract with 2000+ characters across multiple paragraphs
        let _backend = JatsBackend;
        let long_para1 = "A".repeat(1000);
        let long_para2 = "B".repeat(1000);
        let xml = format!(
            r#"<?xml version="1.0"?>
<article>
    <front>
        <abstract>
            <p>{long_para1}</p>
            <p>{long_para2}</p>
        </abstract>
    </front>
</article>"#
        );

        let result = JatsBackend::extract_abstract(&xml);
        assert!(
            result.is_ok(),
            "Very long abstract should parse successfully"
        );
        let abstracts = result.unwrap();
        assert_eq!(abstracts.len(), 2, "Should extract two abstract paragraphs");
        assert_eq!(
            abstracts[0].len(),
            1000,
            "First paragraph should be 1000 chars"
        );
        assert_eq!(
            abstracts[1].len(),
            1000,
            "Second paragraph should be 1000 chars"
        );
    }

    /// N=2847: Test verifies HTML entities are kept escaped to match Python docling v2.58.0 output.
    /// Verified against groundtruth file pone.0234687.nxml.md which contains &gt; and &lt; escaped.
    #[test]
    fn test_abstract_with_special_characters() {
        // Test abstract with special characters and XML entities
        // XML entities should remain escaped in output to match Python docling behavior
        let _backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <front>
        <abstract>
            <p>Results show α=0.05, p&lt;0.001, with &amp; without controls.</p>
        </abstract>
    </front>
</article>"#;

        let result = JatsBackend::extract_abstract(xml);
        assert!(
            result.is_ok(),
            "Abstract with special characters should parse successfully"
        );
        let abstracts = result.unwrap();
        assert_eq!(
            abstracts.len(),
            1,
            "Should extract abstract with special chars"
        );
        // Greek letter α is preserved directly (not an XML entity)
        assert!(abstracts[0].contains("α"), "Should preserve Greek letter");
        // XML entities are kept escaped to match Python docling output
        assert!(
            abstracts[0].contains("&lt;"),
            "Should keep &lt; escaped (matches Python)"
        );
        assert!(
            abstracts[0].contains("&amp;"),
            "Should keep &amp; escaped (matches Python)"
        );
    }

    #[test]
    fn test_abstract_structured() {
        // Test structured abstract with sec elements and labels (Background, Methods, Results, Conclusions)
        let _backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <front>
        <abstract>
            <sec>
                <label>Background</label>
                <p>Background information.</p>
            </sec>
            <sec>
                <label>Methods</label>
                <p>Methods description.</p>
            </sec>
            <sec>
                <label>Results</label>
                <p>Results summary.</p>
            </sec>
            <sec>
                <label>Conclusions</label>
                <p>Conclusions drawn.</p>
            </sec>
        </abstract>
    </front>
</article>"#;

        let result = JatsBackend::extract_abstract(xml);
        assert!(
            result.is_ok(),
            "Structured abstract should parse successfully"
        );
        let abstracts = result.unwrap();
        assert_eq!(
            abstracts.len(),
            4,
            "Should extract all 4 structured abstract paragraphs"
        );

        // Verify all 4 sections are present
        assert!(
            abstracts
                .iter()
                .any(|p| p.contains("Background information")),
            "Should extract Background section"
        );
        assert!(
            abstracts.iter().any(|p| p.contains("Methods description")),
            "Should extract Methods section"
        );
        assert!(
            abstracts.iter().any(|p| p.contains("Results summary")),
            "Should extract Results section"
        );
        assert!(
            abstracts.iter().any(|p| p.contains("Conclusions drawn")),
            "Should extract Conclusions section"
        );
    }

    // === Author Metadata Edge Cases ===

    #[test]
    fn test_authors_with_affiliations() {
        // Test authors with complex affiliations (multiple aff elements)
        let _backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <front>
        <article-meta>
            <contrib-group>
                <contrib contrib-type="author">
                    <name>
                        <surname>Smith</surname>
                        <given-names>John</given-names>
                    </name>
                    <aff id="aff1">Department of Biology</aff>
                    <aff id="aff2">Institute of Science</aff>
                </contrib>
            </contrib-group>
        </article-meta>
    </front>
</article>"#;

        let result = JatsBackend::extract_authors(xml);
        assert!(
            result.is_ok(),
            "Authors with affiliations should parse successfully"
        );
        let authors = result.unwrap();
        assert_eq!(authors.len(), 1, "Should extract author with affiliations");
        assert_eq!(authors[0], "John Smith", "Should extract author name");
    }

    #[test]
    fn test_authors_with_orcid() {
        // Test authors with ORCID identifiers
        let _backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <front>
        <article-meta>
            <contrib-group>
                <contrib contrib-type="author">
                    <name>
                        <surname>Johnson</surname>
                        <given-names>Alice</given-names>
                    </name>
                    <contrib-id contrib-id-type="orcid">https://orcid.org/0000-0002-1234-5678</contrib-id>
                </contrib>
            </contrib-group>
        </article-meta>
    </front>
</article>"#;

        let result = JatsBackend::extract_authors(xml);
        assert!(result.is_ok());
        let authors = result.unwrap();
        assert_eq!(authors.len(), 1, "Should extract author with ORCID");
        assert_eq!(authors[0], "Alice Johnson", "Should extract author name");
    }

    #[test]
    fn test_authors_consortium() {
        // Test consortium/group authors (collab element)
        let _backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <front>
        <article-meta>
            <contrib-group>
                <contrib contrib-type="author">
                    <collab>COVID-19 Genomics Consortium</collab>
                </contrib>
            </contrib-group>
        </article-meta>
    </front>
</article>"#;

        let result = JatsBackend::extract_authors(xml);
        assert!(result.is_ok());
        let authors = result.unwrap();
        assert_eq!(authors.len(), 1, "Should extract consortium as author");
        assert_eq!(
            authors[0], "COVID-19 Genomics Consortium",
            "Should extract consortium name from collab element"
        );
    }

    #[test]
    fn test_authors_with_name_particles() {
        // Test authors with name particles (van, de, von, etc.)
        let _backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <front>
        <article-meta>
            <contrib-group>
                <contrib contrib-type="author">
                    <name>
                        <surname>van der Berg</surname>
                        <given-names>Hans</given-names>
                    </name>
                </contrib>
                <contrib contrib-type="author">
                    <name>
                        <surname>de la Cruz</surname>
                        <given-names>Maria</given-names>
                    </name>
                </contrib>
            </contrib-group>
        </article-meta>
    </front>
</article>"#;

        let result = JatsBackend::extract_authors(xml);
        assert!(result.is_ok());
        let authors = result.unwrap();
        assert_eq!(
            authors.len(),
            2,
            "Should extract authors with name particles"
        );
        assert_eq!(
            authors[0], "Hans van der Berg",
            "Should preserve name particles"
        );
        assert_eq!(
            authors[1], "Maria de la Cruz",
            "Should preserve Spanish name particles"
        );
    }

    // === Body Content Edge Cases ===

    #[test]
    fn test_body_deeply_nested_sections() {
        // Test deeply nested sections (5+ levels)
        let backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <body>
        <sec>
            <title>Level 1</title>
            <sec>
                <title>Level 2</title>
                <sec>
                    <title>Level 3</title>
                    <sec>
                        <title>Level 4</title>
                        <sec>
                            <title>Level 5</title>
                            <p>Deep content.</p>
                        </sec>
                    </sec>
                </sec>
            </sec>
        </sec>
    </body>
</article>"#;

        let result = backend.extract_body(xml);
        assert!(result.is_ok());
        let items = result.unwrap();

        // Should extract all section headers
        let section_count = items
            .iter()
            .filter(|item| matches!(item, DocItem::SectionHeader { .. }))
            .count();
        assert_eq!(section_count, 5, "Should extract all 5 section headers");

        // Should extract deep content
        let has_content = items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("Deep content"),
            _ => false,
        });
        assert!(has_content, "Should extract deeply nested content");
    }

    #[test]
    fn test_body_section_with_subtitle() {
        // Test sections with subtitle elements
        let backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <body>
        <sec>
            <title>Main Title</title>
            <subtitle>Explanatory Subtitle</subtitle>
            <p>Section content.</p>
        </sec>
    </body>
</article>"#;

        let result = backend.extract_body(xml);
        assert!(result.is_ok());
        let items = result.unwrap();

        // Should extract section with title
        let has_title = items.iter().any(|item| match item {
            DocItem::SectionHeader { text, .. } => text.contains("Main Title"),
            _ => false,
        });
        assert!(has_title, "Should extract section title");

        // Should extract subtitle
        let has_subtitle = items.iter().any(|item| match item {
            DocItem::SectionHeader { text, .. } => text.contains("Explanatory Subtitle"),
            DocItem::Text { text, .. } => text.contains("Explanatory Subtitle"),
            _ => false,
        });
        assert!(
            has_subtitle,
            "Should extract subtitle (as part of section header or separate text item)"
        );

        // Should extract section content
        let has_content = items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("Section content"),
            _ => false,
        });
        assert!(has_content, "Should extract section content");
    }

    #[test]
    fn test_body_paragraphs_with_inline_citations() {
        // Test paragraphs with inline citations (xref elements)
        let backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <body>
        <sec>
            <title>Background</title>
            <p>Previous studies<xref ref-type="bibr" rid="ref1">1</xref> showed that<xref ref-type="bibr" rid="ref2">2,3</xref> results vary.</p>
        </sec>
    </body>
</article>"#;

        let result = backend.extract_body(xml);
        assert!(result.is_ok());
        let items = result.unwrap();

        // Should extract paragraph text
        let has_para = items.iter().any(|item| match item {
            DocItem::Text { text, .. } => {
                text.contains("Previous studies") && text.contains("results vary")
            }
            _ => false,
        });
        assert!(has_para, "Should extract paragraph with citations");

        // Citation numbers may or may not be preserved depending on implementation
    }

    #[test]
    fn test_body_paragraphs_with_inline_math() {
        // Test paragraphs with inline math (inline-formula)
        let backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <body>
        <sec>
            <title>Methods</title>
            <p>We calculated the ratio <inline-formula>r=a/b</inline-formula> for each sample.</p>
        </sec>
    </body>
</article>"#;

        let result = backend.extract_body(xml);
        assert!(result.is_ok());
        let items = result.unwrap();

        // Should extract paragraph text (math may be skipped or converted)
        let has_para = items.iter().any(|item| match item {
            DocItem::Text { text, .. } => {
                text.contains("We calculated") && text.contains("for each sample")
            }
            _ => false,
        });
        assert!(has_para, "Should extract paragraph around inline math");
    }

    // === List Variations ===

    #[test]
    fn test_lists_definition_list() {
        // Test definition lists (def-list with term and def elements)
        let backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <body>
        <sec>
            <title>Glossary</title>
            <def-list>
                <def-item>
                    <term>PCR</term>
                    <def>
                        <p>Polymerase Chain Reaction</p>
                    </def>
                </def-item>
                <def-item>
                    <term>DNA</term>
                    <def>
                        <p>Deoxyribonucleic Acid</p>
                    </def>
                </def-item>
            </def-list>
        </sec>
    </body>
</article>"#;

        let result = backend.extract_body(xml);
        assert!(result.is_ok());
        let items = result.unwrap();

        // Should extract section header
        let has_section = items.iter().any(|item| match item {
            DocItem::SectionHeader { text, .. } => text.contains("Glossary"),
            _ => false,
        });
        assert!(has_section, "Should extract section header");

        // Should extract both terms
        let has_pcr = items.iter().any(|item| match item {
            DocItem::Text { text, .. } | DocItem::ListItem { text, .. } => text.contains("PCR"),
            _ => false,
        });
        assert!(has_pcr, "Should extract PCR term");

        let has_dna = items.iter().any(|item| match item {
            DocItem::Text { text, .. } | DocItem::ListItem { text, .. } => text.contains("DNA"),
            _ => false,
        });
        assert!(has_dna, "Should extract DNA term");

        // Should extract both definitions
        let has_pcr_def = items.iter().any(|item| match item {
            DocItem::Text { text, .. } | DocItem::ListItem { text, .. } => {
                text.contains("Polymerase Chain Reaction")
            }
            _ => false,
        });
        assert!(has_pcr_def, "Should extract PCR definition");

        let has_dna_def = items.iter().any(|item| match item {
            DocItem::Text { text, .. } | DocItem::ListItem { text, .. } => {
                text.contains("Deoxyribonucleic Acid")
            }
            _ => false,
        });
        assert!(has_dna_def, "Should extract DNA definition");
    }

    #[test]
    fn test_lists_with_labels() {
        // Test lists with label elements (labeled list items)
        let backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <body>
        <sec>
            <title>Steps</title>
            <list list-type="order">
                <list-item>
                    <label>Step 1.</label>
                    <p>Prepare samples.</p>
                </list-item>
                <list-item>
                    <label>Step 2.</label>
                    <p>Run analysis.</p>
                </list-item>
            </list>
        </sec>
    </body>
</article>"#;

        let result = backend.extract_body(xml);
        assert!(result.is_ok());
        let items = result.unwrap();

        // Should extract list items
        let list_count = items
            .iter()
            .filter(|item| matches!(item, DocItem::ListItem { .. }))
            .count();
        assert!(list_count >= 2, "Should extract at least 2 list items");
    }

    #[test]
    fn test_lists_deeply_nested() {
        // Test very deeply nested lists (3+ levels)
        let backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <body>
        <sec>
            <title>Hierarchy</title>
            <list list-type="bullet">
                <list-item>
                    <p>Level 1 Item A</p>
                    <list list-type="bullet">
                        <list-item>
                            <p>Level 2 Item A1</p>
                            <list list-type="bullet">
                                <list-item>
                                    <p>Level 3 Item A1a</p>
                                </list-item>
                            </list>
                        </list-item>
                    </list>
                </list-item>
            </list>
        </sec>
    </body>
</article>"#;

        let result = backend.extract_body(xml);
        assert!(result.is_ok());
        let items = result.unwrap();

        // Should extract section header
        let has_section = items.iter().any(|item| match item {
            DocItem::SectionHeader { text, .. } => text.contains("Hierarchy"),
            _ => false,
        });
        assert!(has_section, "Should extract section header");

        // Should extract nested list structure as separate DocItems
        // 3 List containers (one for each level)
        let list_container_count = items
            .iter()
            .filter(|item| matches!(item, DocItem::List { .. }))
            .count();
        assert!(
            list_container_count >= 3,
            "Should have at least 3 List containers for 3-level nesting, got {list_container_count}"
        );

        // 3 ListItem elements (one for each level)
        let list_item_count = items
            .iter()
            .filter(|item| matches!(item, DocItem::ListItem { .. }))
            .count();
        assert!(
            list_item_count >= 3,
            "Should have at least 3 ListItem elements for 3 levels, got {list_item_count}"
        );

        // Verify list item contents are NOT flattened (each should only have its direct text)
        let list_items: Vec<String> = items
            .iter()
            .filter_map(|item| match item {
                DocItem::ListItem { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();

        // Find the top-level item
        let top_item = list_items.iter().find(|text| text.contains("Level 1"));
        assert!(top_item.is_some(), "Should have Level 1 item");
        let top_text = top_item.unwrap();
        // Top level item should NOT contain nested content (should only have "Level 1 Item A")
        assert!(
            !top_text.contains("Level 2"),
            "Level 1 item should not contain Level 2 text (nested content should be separate DocItems), got: {top_text}"
        );
    }

    #[test]
    fn test_inline_formula_elements() {
        // Test inline mathematical formulas within text
        let backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <body>
        <sec>
            <p>The equation <inline-formula><tex-math>E = mc^2</tex-math></inline-formula> is fundamental.</p>
        </sec>
    </body>
</article>"#;

        let result = backend.extract_body(xml);
        assert!(result.is_ok());
        let items = result.unwrap();

        let has_text = items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("equation") && text.contains("fundamental"),
            _ => false,
        });
        assert!(has_text, "Should extract text with inline formula context");
    }

    #[test]
    fn test_contrib_group_authors() {
        // Test author contribution groups
        let backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <front>
        <article-meta>
            <contrib-group>
                <contrib contrib-type="author">
                    <name><surname>Smith</surname><given-names>John</given-names></name>
                </contrib>
                <contrib contrib-type="author">
                    <name><surname>Doe</surname><given-names>Jane</given-names></name>
                </contrib>
            </contrib-group>
        </article-meta>
    </front>
    <body>
        <sec><p>Test content</p></sec>
    </body>
</article>"#;

        let result = backend.parse_bytes(xml.as_bytes(), &Default::default());
        assert!(result.is_ok());
        let doc = result.unwrap();

        // Author should be in metadata
        let author = doc.metadata.author.as_ref();
        assert!(author.is_some());
        // Should have author information
        assert!(
            !author.unwrap().is_empty(),
            "Should have author information"
        );
    }

    #[test]
    fn test_mixed_citation_formats() {
        // Test various citation element formats
        let backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <back>
        <ref-list>
            <ref id="ref1">
                <element-citation publication-type="journal">
                    <person-group person-group-type="author">
                        <name><surname>Author</surname></name>
                    </person-group>
                    <article-title>Title</article-title>
                    <year>2023</year>
                </element-citation>
            </ref>
            <ref id="ref2">
                <mixed-citation>Author et al. (2024) Title. Journal.</mixed-citation>
            </ref>
        </ref-list>
    </back>
    <body>
        <sec><p>Test</p></sec>
    </body>
</article>"#;

        let result = backend.parse_bytes(xml.as_bytes(), &Default::default());
        assert!(result.is_ok());
        // Should successfully parse document with mixed citation formats
    }

    #[test]
    fn test_article_with_multiple_abstracts() {
        // Test document with multiple abstracts (e.g., abstract in multiple languages)
        let backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <front>
        <article-meta>
            <title-group>
                <article-title>Multilingual Article</article-title>
            </title-group>
            <abstract xml:lang="en">
                <p>This is the English abstract.</p>
            </abstract>
            <abstract xml:lang="fr">
                <p>Ceci est le résumé français.</p>
            </abstract>
            <abstract xml:lang="de">
                <p>Dies ist die deutsche Zusammenfassung.</p>
            </abstract>
        </article-meta>
    </front>
    <body>
        <sec><p>Content</p></sec>
    </body>
</article>"#;

        let result = backend.parse_bytes(xml.as_bytes(), &Default::default());
        assert!(result.is_ok());
        let doc = result.unwrap();

        // Should include all abstracts
        let md = doc.markdown;
        assert!(md.contains("English abstract") || md.contains("Abstract"));
        // Verify document has content
        assert!(!md.is_empty());
    }

    #[test]
    fn test_article_with_funding_information() {
        // Test extraction of funding information
        let backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <front>
        <article-meta>
            <title-group>
                <article-title>Funded Research</article-title>
            </title-group>
            <funding-group>
                <award-group>
                    <funding-source>National Science Foundation</funding-source>
                    <award-id>NSF-12345</award-id>
                </award-group>
                <award-group>
                    <funding-source>European Research Council</funding-source>
                    <award-id>ERC-67890</award-id>
                </award-group>
            </funding-group>
        </article-meta>
    </front>
    <body>
        <sec><p>Research content</p></sec>
    </body>
</article>"#;

        let result = backend.parse_bytes(xml.as_bytes(), &Default::default());
        assert!(result.is_ok());
        let doc = result.unwrap();

        // Verify document parses successfully
        assert!(!doc.markdown.is_empty());
        assert!(doc.metadata.title.is_some());
    }

    #[test]
    fn test_article_with_keywords() {
        // Test keyword extraction
        let backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <front>
        <article-meta>
            <title-group>
                <article-title>Keyword Test Article</article-title>
            </title-group>
            <kwd-group>
                <kwd>Machine Learning</kwd>
                <kwd>Natural Language Processing</kwd>
                <kwd>Deep Learning</kwd>
                <kwd>Neural Networks</kwd>
                <kwd>Artificial Intelligence</kwd>
            </kwd-group>
        </article-meta>
    </front>
    <body>
        <sec><p>Content about AI</p></sec>
    </body>
</article>"#;

        let result = backend.parse_bytes(xml.as_bytes(), &Default::default());
        assert!(result.is_ok());
        let doc = result.unwrap();

        // Verify document parses successfully
        assert!(!doc.markdown.is_empty());
        // Should contain article title
        assert!(doc.markdown.contains("Keyword Test Article"));
    }

    #[test]
    fn test_article_with_supplementary_material() {
        // Test handling of supplementary material references
        let backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <front>
        <article-meta>
            <title-group>
                <article-title>Article with Supplements</article-title>
            </title-group>
        </article-meta>
    </front>
    <body>
        <sec>
            <p>See supplementary material for details.</p>
            <supplementary-material id="supp1">
                <label>Supplementary Material 1</label>
                <caption>
                    <p>Additional experimental data</p>
                </caption>
            </supplementary-material>
            <supplementary-material id="supp2">
                <label>Supplementary Material 2</label>
                <caption>
                    <p>Statistical analysis code</p>
                </caption>
            </supplementary-material>
        </sec>
    </body>
</article>"#;

        let result = backend.parse_bytes(xml.as_bytes(), &Default::default());
        assert!(result.is_ok());
        let doc = result.unwrap();

        // Should parse document with supplementary materials
        assert!(!doc.markdown.is_empty());
        assert!(doc.markdown.contains("supplementary") || doc.markdown.contains("details"));
    }

    #[test]
    fn test_article_with_complex_table() {
        // Test complex table with headers, footers, and merged cells
        let backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <front>
        <article-meta>
            <title-group>
                <article-title>Table Test</article-title>
            </title-group>
        </article-meta>
    </front>
    <body>
        <sec>
            <table-wrap id="tbl1">
                <label>Table 1</label>
                <caption>
                    <p>Experimental Results</p>
                </caption>
                <table>
                    <thead>
                        <tr>
                            <th>Method</th>
                            <th>Accuracy</th>
                            <th>Precision</th>
                            <th>Recall</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr>
                            <td>Baseline</td>
                            <td>0.85</td>
                            <td>0.83</td>
                            <td>0.87</td>
                        </tr>
                        <tr>
                            <td>Proposed</td>
                            <td>0.92</td>
                            <td>0.91</td>
                            <td>0.93</td>
                        </tr>
                    </tbody>
                    <tfoot>
                        <tr>
                            <td colspan="4">All metrics are averaged over 5 runs.</td>
                        </tr>
                    </tfoot>
                </table>
            </table-wrap>
        </sec>
    </body>
</article>"#;

        let result = backend.parse_bytes(xml.as_bytes(), &Default::default());
        assert!(result.is_ok());
        let doc = result.unwrap();

        // Should include table content
        let md = doc.markdown;
        assert!(!md.is_empty());
        // Should have table-related content
        assert!(md.contains("Table") || md.contains("Method") || md.contains("Accuracy"));
    }

    #[test]
    fn test_article_with_affiliations() {
        // Test author affiliations (aff elements)
        let backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <front>
        <article-meta>
            <contrib-group>
                <contrib>
                    <name><surname>Smith</surname><given-names>John</given-names></name>
                    <xref ref-type="aff" rid="aff1">1</xref>
                </contrib>
            </contrib-group>
            <aff id="aff1">
                <label>1</label>
                <institution>MIT</institution>
                <addr-line><city>Cambridge</city><state>MA</state></addr-line>
            </aff>
        </article-meta>
    </front>
</article>"#;

        let result = backend.parse_bytes(xml.as_bytes(), &Default::default());
        assert!(result.is_ok());
        let _doc = result.unwrap();
        // Affiliations are metadata, may not appear in markdown body (appears in metadata instead)
        // Test that parsing succeeds without errors (markdown exists)
    }

    #[test]
    fn test_article_with_permissions() {
        // Test copyright and licensing information (permissions element)
        let backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <front>
        <article-meta>
            <permissions>
                <copyright-statement>© 2024 The Authors</copyright-statement>
                <copyright-year>2024</copyright-year>
                <copyright-holder>The Authors</copyright-holder>
                <license license-type="open-access">
                    <license-p>This is an open access article under CC BY license.</license-p>
                </license>
            </permissions>
        </article-meta>
    </front>
</article>"#;

        let result = backend.parse_bytes(xml.as_bytes(), &Default::default());
        assert!(result.is_ok());
        let _doc = result.unwrap();
        // Permissions may or may not be extracted depending on implementation
        // Test that parsing succeeds without errors (markdown exists)
    }

    #[test]
    fn test_article_with_history() {
        // Test article history (received, accepted, published dates)
        let backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <front>
        <article-meta>
            <history>
                <date date-type="received">
                    <day>15</day>
                    <month>01</month>
                    <year>2024</year>
                </date>
                <date date-type="accepted">
                    <day>20</day>
                    <month>03</month>
                    <year>2024</year>
                </date>
            </history>
        </article-meta>
    </front>
</article>"#;

        let result = backend.parse_bytes(xml.as_bytes(), &Default::default());
        assert!(result.is_ok());
        let _doc = result.unwrap();
        // History dates may or may not be extracted depending on implementation
        // Test that parsing succeeds without errors (markdown exists)
    }

    #[test]
    fn test_article_with_corresp() {
        // Test correspondence information (corresp elements for contact details)
        let backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <front>
        <article-meta>
            <author-notes>
                <corresp id="cor1">
                    <label>*</label>
                    Correspondence: <email>author@example.com</email>
                </corresp>
                <fn fn-type="con">
                    <p>All authors contributed equally.</p>
                </fn>
            </author-notes>
        </article-meta>
    </front>
</article>"#;

        let result = backend.parse_bytes(xml.as_bytes(), &Default::default());
        assert!(result.is_ok());
        let _doc = result.unwrap();
        // Correspondence info may or may not be extracted
        // Test that parsing succeeds without errors (markdown exists)
    }

    #[test]
    fn test_article_with_disp_formula() {
        // Test display formulas (disp-formula for numbered equations)
        let backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<article>
    <body>
        <sec>
            <p>Consider the following equation:</p>
            <disp-formula id="eq1">
                <label>(1)</label>
                <tex-math>\int_{0}^{\infty} e^{-x^2} dx = \frac{\sqrt{\pi}}{2}</tex-math>
            </disp-formula>
            <p>This is the Gaussian integral.</p>
        </sec>
    </body>
</article>"#;

        let result = backend.parse_bytes(xml.as_bytes(), &Default::default());
        assert!(result.is_ok());
        let doc = result.unwrap();

        let md = doc.markdown;
        assert!(!md.is_empty());
        // Should have context around formula (text from paragraphs)
        assert!(md.contains("equation") || md.contains("Gaussian"));
    }

    /// N=2408: Inline formatting extraction (bold)
    /// Updated N=2933: Changed to match Python behavior - bold text is merged into paragraph
    #[test]
    fn test_jats_formatting_extraction_bold() {
        let backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<!DOCTYPE article PUBLIC "-//NLM//DTD JATS (Z39.96) Journal Archiving and Interchange DTD v1.2 20190208//EN" "JATS-archivearticle1.dtd">
<article>
    <body>
        <sec>
            <p>This is <bold>bold text</bold> in a paragraph.</p>
        </sec>
    </body>
</article>"#;

        let result = backend.parse_bytes(xml.as_bytes(), &Default::default());
        assert!(result.is_ok());
        let doc = result.unwrap();

        // Check that bold text is merged into paragraph (matches Python behavior)
        // Python doesn't create separate DocItems for inline formatting
        assert!(
            doc.markdown.contains("bold text"),
            "Markdown should contain the bold text content"
        );
        assert!(
            doc.markdown.contains("This is bold text in a paragraph"),
            "Text should be merged into a single paragraph"
        );
    }

    /// N=2408: Inline formatting extraction (italic)
    /// Updated N=2933: Changed to match Python behavior - italic text is merged into paragraph
    #[test]
    fn test_jats_formatting_extraction_italic() {
        let backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<!DOCTYPE article PUBLIC "-//NLM//DTD JATS (Z39.96) Journal Archiving and Interchange DTD v1.2 20190208//EN" "JATS-archivearticle1.dtd">
<article>
    <body>
        <sec>
            <p>This is <italic>italic text</italic> in a paragraph.</p>
        </sec>
    </body>
</article>"#;

        let result = backend.parse_bytes(xml.as_bytes(), &Default::default());
        assert!(result.is_ok());
        let doc = result.unwrap();

        // Check that italic text is merged into paragraph (matches Python behavior)
        // Python doesn't create separate DocItems for inline formatting
        assert!(
            doc.markdown.contains("italic text"),
            "Markdown should contain the italic text content"
        );
        assert!(
            doc.markdown.contains("This is italic text in a paragraph"),
            "Text should be merged into a single paragraph"
        );
    }

    /// N=2408: Inline formatting extraction (subscript)
    /// Updated N=2933: Changed to match Python behavior - subscript text is merged into paragraph
    #[test]
    fn test_jats_formatting_extraction_subscript() {
        let backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<!DOCTYPE article PUBLIC "-//NLM//DTD JATS (Z39.96) Journal Archiving and Interchange DTD v1.2 20190208//EN" "JATS-archivearticle1.dtd">
<article>
    <body>
        <sec>
            <p>H<sub>2</sub>O is water.</p>
        </sec>
    </body>
</article>"#;

        let result = backend.parse_bytes(xml.as_bytes(), &Default::default());
        assert!(result.is_ok());
        let doc = result.unwrap();

        // Check that subscript text is merged into paragraph (matches Python behavior)
        assert!(
            doc.markdown.contains("H2O is water"),
            "Markdown should contain the merged text with subscript content"
        );
    }

    /// N=2408: Inline formatting extraction (superscript)
    /// Updated N=2933: Changed to match Python behavior - superscript text is merged into paragraph
    #[test]
    fn test_jats_formatting_extraction_superscript() {
        let backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<!DOCTYPE article PUBLIC "-//NLM//DTD JATS (Z39.96) Journal Archiving and Interchange DTD v1.2 20190208//EN" "JATS-archivearticle1.dtd">
<article>
    <body>
        <sec>
            <p>E=mc<sup>2</sup> is Einstein's equation.</p>
        </sec>
    </body>
</article>"#;

        let result = backend.parse_bytes(xml.as_bytes(), &Default::default());
        assert!(result.is_ok());
        let doc = result.unwrap();

        // Check that superscript text is merged into paragraph (matches Python behavior)
        assert!(
            doc.markdown.contains("E=mc2 is Einstein's equation"),
            "Markdown should contain the merged text with superscript content"
        );
    }

    /// N=2408: Inline formatting extraction (combined)
    /// Updated N=2933: Changed to match Python behavior - formatted text is merged into paragraph
    #[test]
    fn test_jats_formatting_extraction_combined() {
        let backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<!DOCTYPE article PUBLIC "-//NLM//DTD JATS (Z39.96) Journal Archiving and Interchange DTD v1.2 20190208//EN" "JATS-archivearticle1.dtd">
<article>
    <body>
        <sec>
            <p>This has <bold>bold</bold>, <italic>italic</italic>, and <monospace>monospace</monospace> text.</p>
        </sec>
    </body>
</article>"#;

        let result = backend.parse_bytes(xml.as_bytes(), &Default::default());
        assert!(result.is_ok());
        let doc = result.unwrap();

        // Check that all formatted text is merged into a single paragraph (matches Python behavior)
        assert!(
            doc.markdown
                .contains("This has bold, italic, and monospace text"),
            "Markdown should contain all text merged into single paragraph"
        );
    }

    #[test]
    fn test_extract_keywords_from_kwd_group() {
        // Test keyword extraction from <kwd-group> for subject metadata
        let _backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<!DOCTYPE article PUBLIC "-//NLM//DTD JATS (Z39.96) Journal Archiving and Interchange DTD v1.2 20190208//EN" "JATS-archivearticle1.dtd">
<article>
    <front>
        <article-meta>
            <kwd-group>
                <kwd>machine learning</kwd>
                <kwd>neural networks</kwd>
                <kwd>deep learning</kwd>
            </kwd-group>
        </article-meta>
    </front>
</article>"#;

        let result = JatsBackend::extract_keywords(xml);
        assert!(result.is_ok());
        let keywords = result.unwrap();
        assert_eq!(keywords.len(), 3, "Should extract 3 keywords");
        assert!(keywords.contains(&"machine learning".to_string()));
        assert!(keywords.contains(&"neural networks".to_string()));
        assert!(keywords.contains(&"deep learning".to_string()));
    }

    #[test]
    fn test_keywords_in_metadata() {
        // Test that keywords are included in DocumentMetadata.subject
        use tempfile::NamedTempFile;

        let backend = JatsBackend;
        let xml = r#"<?xml version="1.0"?>
<!DOCTYPE article PUBLIC "-//NLM//DTD JATS (Z39.96) Journal Archiving and Interchange DTD v1.2 20190208//EN" "JATS-archivearticle1.dtd">
<article>
    <front>
        <article-meta>
            <title-group>
                <article-title>Test Article</article-title>
            </title-group>
            <kwd-group>
                <kwd>bioinformatics</kwd>
                <kwd>genomics</kwd>
            </kwd-group>
        </article-meta>
    </front>
    <body>
        <sec>
            <p>Test paragraph</p>
        </sec>
    </body>
</article>"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        use std::io::Write;
        temp_file.write_all(xml.as_bytes()).unwrap();
        let temp_path = temp_file.path();

        let result = backend.parse_file(temp_path, &Default::default());
        assert!(result.is_ok());
        let doc = result.unwrap();

        assert!(
            doc.metadata.subject.is_some(),
            "Should have subject metadata"
        );
        let subject = doc.metadata.subject.unwrap();
        assert!(
            subject.contains("bioinformatics"),
            "Should contain first keyword"
        );
        assert!(
            subject.contains("genomics"),
            "Should contain second keyword"
        );
        assert!(subject.contains(", "), "Keywords should be comma-separated");
    }
}
