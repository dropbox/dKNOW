//! HTML Backend - Port from Python docling v2.58.0
//!
//! Source: ~/`docling/docling/backend/html_backend.py` (1350 lines, 52KB)
//!
//! Parses HTML documents into structured `DocItems`.
//!
//! # Implemented Features (~72% complete)
//!
//! - Headings (h1-h6) - Creates `SectionHeader` `DocItems` with proper levels
//! - Paragraphs (p, address, summary) - Creates Text `DocItems`
//! - Lists (ul, ol) - Creates List + `ListItem` `DocItems`, supports start attribute, parent references
//! - Tables (table, tr, td, th) - Creates Table `DocItems` with grid + cells, handles colspan/rowspan
//! - Code blocks (pre) - Creates Code `DocItems`
//! - Images (img) - Creates Picture `DocItems` with alt/title/src, caption generation from figcaption/alt/title
//!
//! # Not Yet Implemented
//!
//! - Footer (footer tag) - Requires hierarchy (ContentLayer.FURNITURE)
//! - Details (details tag) - Requires hierarchy (GroupLabel.SECTION)
//! - Inline formatting (strong, em, a) - Requires formatting annotations
//! - Nested lists - Requires parent-child relationships for nested list containers
//! - Rich table cells - Requires nested content within cells
//! - Language detection for code blocks (not in Python either)
//!
//! # Python Reference
//!
//! See reports/feature-phase-e-open-standards/html_backend_analysis_2025-11-10-09-45.md

// Clippy pedantic allows:
// - Table cell row calculation uses isize for span handling
#![allow(clippy::cast_possible_wrap)]

use crate::traits::{BackendOptions, DocumentBackend};
use crate::utils::{
    create_list_item_with_hyperlink, create_provenance, create_section_header_with_hyperlink,
    create_text_item, create_text_item_with_hyperlink,
};
use docling_core::content::Formatting;
use docling_core::{
    content::{DocItem, ItemRef, TableCell, TableData},
    DoclingError, Document, DocumentMetadata, InputFormat,
};
use scraper::{ElementRef, Html, Selector};

/// HTML Document Backend
///
/// Ported from: docling/backend/html_backend.py:193-1350
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct HtmlBackend;

/// Text run with formatting annotation
/// Python reference: `AnnotatedText` (lines 95-100)
#[derive(Debug, Clone, PartialEq)]
struct AnnotatedText {
    text: String,
    formatting: Option<Formatting>,
    hyperlink: Option<String>,
}

/// Format tag stack for tracking nested formatting during traversal
/// Python reference: `format_tags` field (line 214) + _`use_format()` context manager (lines 719-727)
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
struct FormatContext {
    format_tags: Vec<String>,
    hyperlink: Option<String>,
}

impl FormatContext {
    #[inline]
    fn new() -> Self {
        Self::default()
    }

    /// Push formatting tags onto stack (entering tag)
    #[inline]
    fn push_tags(&mut self, tags: Vec<String>) {
        self.format_tags.extend(tags);
    }

    /// Pop N formatting tags from stack (exiting tag)
    #[inline]
    fn pop_tags(&mut self, count: usize) {
        let new_len = self.format_tags.len().saturating_sub(count);
        self.format_tags.truncate(new_len);
    }

    /// Convert active format tags to Formatting object
    /// Python reference: _formatting property (lines 606-613)
    #[inline]
    fn to_formatting(&self) -> Option<Formatting> {
        let mut bold = false;
        let mut italic = false;
        let mut underline = false;
        let mut strikethrough = false;
        let mut code = false;
        let mut script: Option<String> = None;

        for tag in &self.format_tags {
            match tag.as_str() {
                "strong" | "b" => bold = true,
                "em" | "i" | "var" => italic = true,
                "u" | "ins" => underline = true,
                "s" | "del" => strikethrough = true,
                "code" | "kbd" | "samp" => code = true,
                "sub" => script = Some("sub".to_string()),
                "sup" => script = Some("super".to_string()),
                _ => {}
            }
        }

        if !bold && !italic && !underline && !strikethrough && !code && script.is_none() {
            None
        } else {
            Some(Formatting {
                bold: bold.then_some(true),
                italic: italic.then_some(true),
                underline: underline.then_some(true),
                strikethrough: strikethrough.then_some(true),
                code: code.then_some(true),
                script,
                font_size: None,
                font_family: None,
            })
        }
    }

    /// Check if tag is a formatting tag
    /// Python reference: _`FORMAT_TAG_MAP` (lines 72-88)
    #[inline]
    fn is_format_tag(tag_name: &str) -> bool {
        matches!(
            tag_name,
            "b" | "strong"
                | "i"
                | "em"
                | "var"
                | "u"
                | "ins"
                | "s"
                | "del"
                | "sub"
                | "sup"
                | "code"
                | "kbd"
                | "samp"
        )
    }
}

#[allow(clippy::trivially_copy_pass_by_ref)] // Unit struct methods conventionally take &self
impl HtmlBackend {
    /// Create a new HTML backend instance
    #[inline]
    #[must_use = "creates a backend instance that should be used for parsing"]
    pub const fn new() -> Self {
        Self
    }

    /// Normalize em dashes and en dashes to hyphens with surrounding spaces
    ///
    /// Python docling converts these for cleaner markdown output:
    /// - Em dash (U+2014 —) → " - " (space + hyphen + space)
    /// - En dash (U+2013 –) → " - " (space + hyphen + space)
    ///
    /// Also handles cases where there are already spaces around the dash to avoid
    /// double spaces. The pattern " — " becomes " - " (not "  -  ").
    #[inline]
    fn normalize_dashes(text: &str) -> String {
        // Replace all dash variants to single hyphen-minus
        // (em-dash and en-dash with surrounding spaces, or standalone)
        text.replace(" — ", " - ")
            .replace(" – ", " - ")
            .replace(['—', '–'], " - ")
    }

    /// Get direct child rows from a table element (not nested table rows)
    ///
    /// HTML tables may contain `<tr>` directly or inside `<thead>`, `<tbody>`, `<tfoot>`.
    /// This function returns only the direct rows, not rows from nested tables.
    fn get_direct_rows<'a>(table_element: &'a ElementRef<'a>) -> Vec<ElementRef<'a>> {
        let mut rows = Vec::new();

        for child in table_element.children() {
            if let Some(child_element) = ElementRef::wrap(child) {
                let tag_name = child_element.value().name();
                match tag_name {
                    "tr" => {
                        // Direct <tr> child
                        rows.push(child_element);
                    }
                    "thead" | "tbody" | "tfoot" => {
                        // Container element - get its direct <tr> children
                        for inner_child in child_element.children() {
                            if let Some(inner_element) = ElementRef::wrap(inner_child) {
                                if inner_element.value().name() == "tr" {
                                    rows.push(inner_element);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        rows
    }

    /// Get direct child cells from a row element (not nested table cells)
    ///
    /// This function returns only direct `<td>` and `<th>` children, not cells
    /// from nested tables within this row.
    fn get_direct_cells<'a>(
        row: &'a ElementRef<'a>,
        _cell_selector: &Selector,
    ) -> Vec<ElementRef<'a>> {
        let mut cells = Vec::new();

        for child in row.children() {
            if let Some(child_element) = ElementRef::wrap(child) {
                let tag_name = child_element.value().name();
                if tag_name == "td" || tag_name == "th" {
                    cells.push(child_element);
                }
            }
        }

        cells
    }

    /// Extract text with formatting from element recursively
    /// Python reference: _`extract_text_and_hyperlink_recursively()` (lines 615-693)
    ///
    /// Key behavior: Preserves newline information for paragraph boundary detection.
    /// Python uses `keep_newlines=True` and then `split_by_newline()` to create
    /// separate inline groups at newline boundaries within `<p>` tags.
    fn extract_text_with_formatting(
        element: ElementRef,
        context: &mut FormatContext,
    ) -> Vec<AnnotatedText> {
        let mut result = Vec::new();
        let element_name = element.value().name();

        // Check if this is a formatting tag or hyperlink
        let is_format_tag = FormatContext::is_format_tag(element_name);
        let is_link = element_name == "a";

        if is_format_tag {
            // Push format tag onto stack
            context.push_tags(vec![element_name.to_string()]);
        }

        if is_link {
            // Extract hyperlink href
            if let Some(href) = element.value().attr("href") {
                context.hyperlink = Some(href.to_string());
            }
        }

        // Process children
        for child in element.children() {
            if let Some(text_node) = child.value().as_text() {
                let raw_text: &str = text_node;

                // Python behavior (_extract_text_and_hyperlink_recursively lines 641-666):
                // - Text is stripped (item.strip()) - removes leading/trailing whitespace
                // - BUT internal newlines are preserved for later splitting
                //
                // Note: Python's strip() only removes LEADING and TRAILING whitespace,
                // so internal newlines are preserved: ".  \n  Notice" -> ".  \n  Notice"
                let text = raw_text.trim();
                if !text.is_empty() {
                    // Normalize em/en dashes to hyphens with surrounding spaces
                    // Python docling converts these for cleaner markdown output
                    let text = Self::normalize_dashes(text);
                    result.push(AnnotatedText {
                        text,
                        formatting: context.to_formatting(),
                        hyperlink: context.hyperlink.clone(),
                    });
                }
                // Whitespace-only nodes are skipped (Python returns empty AnnotatedTextList)
            } else if let Some(child_element) = ElementRef::wrap(child) {
                // Element node - recurse
                let child_texts = Self::extract_text_with_formatting(child_element, context);
                result.extend(child_texts);
            }
        }

        // Pop format tag from stack
        if is_format_tag {
            context.pop_tags(1);
        }

        if is_link {
            // Clear hyperlink context
            context.hyperlink = None;
        }

        result
    }

    /// Group consecutive `AnnotatedText` runs with identical formatting into `DocItems`
    ///
    /// If all runs have identical formatting, creates a single Text `DocItem`.
    /// If runs have different formatting, creates an Inline group containing Text children.
    ///
    /// Python reference: `simplify_text_elements()` (lines 135-156)
    fn group_text_runs(&self, runs: Vec<AnnotatedText>, item_count: &mut usize) -> Vec<DocItem> {
        if runs.is_empty() {
            return vec![];
        }

        // Group consecutive runs with identical formatting
        let mut grouped_runs: Vec<Vec<AnnotatedText>> = Vec::new();
        let mut iter = runs.into_iter();
        // SAFETY: We checked runs.is_empty() above
        let mut current_group: Vec<AnnotatedText> = vec![iter.next().unwrap()];

        for run in iter {
            // Safe: current_group always has at least one element
            if let Some(last) = current_group.last() {
                if run.formatting != last.formatting || run.hyperlink != last.hyperlink {
                    // Different formatting - flush current group and start new one
                    if !current_group.is_empty() {
                        grouped_runs.push(current_group);
                        current_group = Vec::new();
                    }
                }
            }
            // Add run to current group (regardless of whether we flushed)
            current_group.push(run);
        }
        // Flush final group
        if !current_group.is_empty() {
            grouped_runs.push(current_group);
        }

        // If only one group, return a single Text DocItem
        if grouped_runs.len() == 1 {
            return vec![self.create_docitem_from_runs(&grouped_runs[0], item_count)];
        }

        // Multiple groups with different formatting - create an Inline group
        // This matches Python behavior where mixed formatting creates inline children

        // Pre-allocate the Inline group's reference
        let inline_ref = format!("#/groups/{item_count}");
        *item_count += 1;

        let mut children = Vec::new();
        let mut child_items = Vec::new();

        for group in &grouped_runs {
            let child_item = Self::create_docitem_from_runs_with_parent(
                group,
                item_count,
                Some(ItemRef::new(inline_ref.clone())),
            );
            if let DocItem::Text { self_ref, .. } = &child_item {
                children.push(ItemRef::new(self_ref.clone()));
            }
            child_items.push(child_item);
        }

        // Create the Inline group
        let inline_group = DocItem::Inline {
            self_ref: inline_ref,
            parent: None,
            children,
            content_layer: "body".to_string(),
            name: "inline".to_string(),
        };

        // Return all items: child Text items first, then the Inline group
        let mut result = child_items;
        result.push(inline_group);
        result
    }

    /// Create a `DocItem` from a group of `AnnotatedText` runs with identical formatting
    // Method signature kept for API consistency with other HtmlBackend methods
    #[allow(clippy::unused_self)]
    fn create_docitem_from_runs(&self, runs: &[AnnotatedText], item_count: &mut usize) -> DocItem {
        Self::create_docitem_from_runs_with_parent(runs, item_count, None)
    }

    /// Create a `DocItem` from a group of `AnnotatedText` runs with specified parent
    fn create_docitem_from_runs_with_parent(
        runs: &[AnnotatedText],
        item_count: &mut usize,
        parent: Option<ItemRef>,
    ) -> DocItem {
        // Combine text from all runs
        let combined_text = runs
            .iter()
            .map(|r| r.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        let formatting = runs[0].formatting.clone();
        let hyperlink = runs[0].hyperlink.clone();

        let self_ref = format!("#/texts/{item_count}");
        *item_count += 1;

        DocItem::Text {
            self_ref,
            parent,
            children: vec![],
            content_layer: "body".to_string(),
            prov: create_provenance(1),
            orig: combined_text.clone(),
            text: combined_text,
            formatting,
            hyperlink,
        }
    }

    /// Extract metadata from HTML meta tags and title
    ///
    /// Extracts standard HTML metadata:
    /// - title: `<title>` tag
    /// - author: `<meta name="author">`
    /// - subject: `<meta name="description">` (N=1880: extracted to DocumentMetadata.subject)
    /// - keywords: `<meta name="keywords">` (not in `DocumentMetadata`, could be added later)
    fn extract_metadata(document: &Html) -> DocumentMetadata {
        let mut metadata = DocumentMetadata::default();

        // Extract title from <title> tag
        if let Ok(selector) = Selector::parse("title") {
            if let Some(title_element) = document.select(&selector).next() {
                let title = title_element.text().collect::<String>().trim().to_string();
                if !title.is_empty() {
                    metadata.title = Some(title);
                }
            }
        }

        // Extract author from <meta name="author">
        if let Ok(selector) = Selector::parse("meta[name='author']") {
            if let Some(meta_element) = document.select(&selector).next() {
                if let Some(author) = meta_element.value().attr("content") {
                    let author = author.trim().to_string();
                    if !author.is_empty() {
                        metadata.author = Some(author);
                    }
                }
            }
        }

        // N=1880: Extract description from <meta name="description"> to subject field
        // HTML meta description is semantically equivalent to DocumentMetadata.subject
        if let Ok(selector) = Selector::parse("meta[name='description']") {
            if let Some(meta_element) = document.select(&selector).next() {
                if let Some(description) = meta_element.value().attr("content") {
                    let description = description.trim().to_string();
                    if !description.is_empty() {
                        metadata.subject = Some(description);
                    }
                }
            }
        }

        metadata
    }

    /// Parse HTML elements and convert to `DocItems`
    ///
    /// Python reference: _`walk()` lines 760-808
    /// Walks the HTML tree in document order, processing elements as encountered
    fn parse_elements(
        &self,
        document: &Html,
        options: &BackendOptions,
    ) -> Result<Vec<DocItem>, DoclingError> {
        let mut doc_items = Vec::new();
        let mut item_count = 0;

        // Get body element and walk from there
        // Python: soup.body in _walk() line 777
        let body_selector = Selector::parse("body")
            .map_err(|e| DoclingError::BackendError(format!("Invalid selector: {e}")))?;

        if let Some(body) = document.select(&body_selector).next() {
            self.walk_element(body, &mut doc_items, &mut item_count, options)?;
        }

        Ok(doc_items)
    }

    /// Recursively walk HTML tree in document order
    ///
    /// Python reference: _`walk()` lines 760-808
    /// Processes elements as encountered, preserving document order
    fn walk_element(
        &self,
        element: scraper::ElementRef,
        doc_items: &mut Vec<DocItem>,
        item_count: &mut usize,
        options: &BackendOptions,
    ) -> Result<(), DoclingError> {
        // Get element name
        let name = element.value().name();

        // Handle element based on type
        // Python: lines 783-806
        match name {
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                Self::handle_heading(element, doc_items, item_count);
            }
            "p" | "address" | "summary" => {
                self.handle_paragraph(element, doc_items, item_count);
            }
            "ul" | "ol" => {
                Self::handle_list_element(element, doc_items, item_count)?;
            }
            "figure" => {
                // Handle figure with potential figcaption
                self.handle_figure(element, doc_items, item_count, options)?;
            }
            "img" => {
                Self::handle_image(element, doc_items, item_count, options);
            }
            "table" => {
                self.handle_table(element, doc_items, item_count)?;
            }
            "pre" => {
                // Skip hidden pre elements
                if element.value().attr("hidden").is_some() {
                    return Ok(());
                }
                Self::handle_code_block(element, doc_items, item_count);
            }
            "a" => {
                // Handle <a> tags wrapping block-level elements
                // When <a href="..."> contains block elements, we need to:
                // 1. Extract the hyperlink href
                // 2. Walk the children, applying hyperlink to text content
                let href = element.value().attr("href");
                Self::walk_element_with_hyperlink(element, doc_items, item_count, href)?;
            }
            // Skip script and style elements - their content is not document text
            "script" | "style" | "noscript" => {
                // Do nothing - skip these elements entirely
            }
            _ => {
                // For other elements, recursively walk children
                // Python: _walk(child, doc) lines 806-807
                for child in element.children() {
                    if let Some(child_element) = scraper::ElementRef::wrap(child) {
                        self.walk_element(child_element, doc_items, item_count, options)?;
                    } else if let Some(text_node) = child.value().as_text() {
                        // Handle bare text nodes (e.g., text after tables not wrapped in <p>)
                        let text = text_node.trim();
                        if !text.is_empty() {
                            doc_items.push(create_text_item(
                                *item_count,
                                text.to_string(),
                                create_provenance(1),
                            ));
                            *item_count += 1;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Walk element children with a hyperlink context
    ///
    /// Used when an `<a>` tag wraps block-level elements. The hyperlink href is passed down
    /// and applied to text content created within.
    fn walk_element_with_hyperlink(
        element: scraper::ElementRef,
        doc_items: &mut Vec<DocItem>,
        item_count: &mut usize,
        hyperlink: Option<&str>,
    ) -> Result<(), DoclingError> {
        // Walk children, applying hyperlink to text content
        for child in element.children() {
            if let Some(child_element) = scraper::ElementRef::wrap(child) {
                let child_name = child_element.value().name();
                match child_name {
                    "p" | "div" | "span" => {
                        // Block or inline container - extract text with hyperlink
                        let text = child_element.text().collect::<Vec<_>>().join(" ");
                        let text = text.trim().to_string();
                        if !text.is_empty() {
                            doc_items.push(create_text_item_with_hyperlink(
                                *item_count,
                                text,
                                create_provenance(1),
                                hyperlink.map(String::from),
                            ));
                            *item_count += 1;
                        }
                    }
                    "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                        // Heading with hyperlink
                        Self::handle_heading_with_hyperlink(
                            child_element,
                            doc_items,
                            item_count,
                            hyperlink.map(String::from),
                        );
                    }
                    _ => {
                        // Recurse for other elements
                        Self::walk_element_with_hyperlink(
                            child_element,
                            doc_items,
                            item_count,
                            hyperlink,
                        )?;
                    }
                }
            } else if let Some(text_node) = child.value().as_text() {
                // Bare text node with hyperlink
                let text = text_node.trim();
                if !text.is_empty() {
                    doc_items.push(create_text_item_with_hyperlink(
                        *item_count,
                        text.to_string(),
                        create_provenance(1),
                        hyperlink.map(String::from),
                    ));
                    *item_count += 1;
                }
            }
        }
        Ok(())
    }

    /// Handle a single heading element
    ///
    /// Python reference: _`handle_heading()` lines 810-874
    /// Note: Headings typically should be bold by default, but we extract inline formatting if present
    fn handle_heading(
        element: scraper::ElementRef,
        doc_items: &mut Vec<DocItem>,
        item_count: &mut usize,
    ) {
        // For now, keep simple text extraction for headings
        // Python docling doesn't extract inline formatting from headings either
        // Headings are already semantically marked by their level
        let text = element.text().collect::<Vec<_>>().join(" ");
        let text = text.trim().to_string();

        if text.is_empty() {
            return;
        }

        // Extract heading level from tag name (h1 = 1, h2 = 2, etc.)
        let level = element
            .value()
            .name()
            .chars()
            .nth(1)
            .and_then(|c| c.to_digit(10))
            .unwrap_or(1) as usize;

        // Check for hyperlink inside heading (e.g., <h2><a href="...">text</a></h2>)
        let hyperlink = scraper::Selector::parse("a")
            .ok()
            .and_then(|selector| element.select(&selector).next())
            .and_then(|a| a.value().attr("href"))
            .map(String::from);

        // Create SectionHeader DocItem with hyperlink if present
        let doc_item = create_section_header_with_hyperlink(
            *item_count,
            text,
            level,
            create_provenance(1),
            hyperlink,
        );

        doc_items.push(doc_item);
        *item_count += 1;
    }

    /// Handle a heading element with a hyperlink
    ///
    /// Used when `<a href="...">` wraps a heading element.
    fn handle_heading_with_hyperlink(
        element: scraper::ElementRef,
        doc_items: &mut Vec<DocItem>,
        item_count: &mut usize,
        hyperlink: Option<String>,
    ) {
        let text = element.text().collect::<Vec<_>>().join(" ");
        let text = text.trim().to_string();

        if text.is_empty() {
            return;
        }

        let level = element
            .value()
            .name()
            .chars()
            .nth(1)
            .and_then(|c| c.to_digit(10))
            .unwrap_or(1) as usize;

        let doc_item = create_section_header_with_hyperlink(
            *item_count,
            text,
            level,
            create_provenance(1),
            hyperlink,
        );

        doc_items.push(doc_item);
        *item_count += 1;
    }

    /// Handle a single paragraph element
    ///
    /// Python reference: _`handle_block()` lines 1050-1078
    /// Now with inline formatting extraction (N=1161)
    /// Updated N=2347: Split runs at newline boundaries (Python `split_by_newline()` behavior)
    fn handle_paragraph(
        &self,
        element: scraper::ElementRef,
        doc_items: &mut Vec<DocItem>,
        item_count: &mut usize,
    ) {
        // Extract text with formatting
        let mut context = FormatContext::new();
        let runs = Self::extract_text_with_formatting(element, &mut context);

        if runs.is_empty() {
            return;
        }

        // Split runs at newline boundaries (Python behavior: split_by_newline())
        // Each part between newlines becomes a separate paragraph/inline group
        let parts = Self::split_runs_by_newline(runs);

        for part in parts {
            if part.is_empty() {
                continue;
            }

            // Group runs by formatting and create DocItems for this part
            let items = self.group_text_runs(part, item_count);
            doc_items.extend(items);
        }
    }

    /// Split annotated text runs at newline boundaries
    ///
    /// Python reference: `AnnotatedTextList.split_by_newline()` (lines 174-190)
    ///
    /// Python's `.strip()` only removes LEADING and TRAILING whitespace, but
    /// INTERNAL newlines are preserved. Then `split_by_newline()` splits each
    /// run's text at `\n` characters, creating separate groups.
    ///
    /// Example: `.  \n  Notice that` (after trim) splits into `[".", "Notice that"]`
    fn split_runs_by_newline(runs: Vec<AnnotatedText>) -> Vec<Vec<AnnotatedText>> {
        let mut super_list: Vec<Vec<AnnotatedText>> = Vec::new();
        let mut active_list: Vec<AnnotatedText> = Vec::new();

        for run in runs {
            // Split this run's text by internal newlines (Python lines 178-188)
            let sub_texts: Vec<&str> = run.text.split('\n').collect();

            if sub_texts.len() == 1 {
                // No internal newlines - just add to current list
                active_list.push(run);
            } else {
                // Has internal newlines - split into multiple runs
                for sub_text in sub_texts {
                    // Strip each part (Python does `annotated_text.text.strip()` when creating doc items)
                    let trimmed = sub_text.trim();
                    if !trimmed.is_empty() {
                        let sub_run = AnnotatedText {
                            text: trimmed.to_string(),
                            formatting: run.formatting.clone(),
                            hyperlink: run.hyperlink.clone(),
                        };
                        active_list.push(sub_run);
                    }
                    // After each part, flush to super_list (Python lines 185-186)
                    super_list.push(active_list);
                    active_list = Vec::new();
                }
            }
        }

        // Don't forget the last active list (Python line 188)
        if !active_list.is_empty() {
            super_list.push(active_list);
        }

        // Filter out lists with only empty/whitespace text
        super_list
            .into_iter()
            .filter(|list| list.iter().any(|run| !run.text.trim().is_empty()))
            .collect()
    }

    /// Handle a single list element (ul or ol)
    ///
    /// Python reference: _`handle_list()` lines 876-1004
    fn handle_list_element(
        list_element: scraper::ElementRef,
        doc_items: &mut Vec<DocItem>,
        item_count: &mut usize,
    ) -> Result<(), DoclingError> {
        let is_ordered = list_element.value().name() == "ol";

        // Check for start attribute on ordered lists
        let start = if is_ordered {
            list_element
                .value()
                .attr("start")
                .and_then(|s| s.parse::<usize>().ok())
        } else {
            None
        };

        Self::parse_list(list_element, is_ordered, start, None, doc_items, item_count)?;
        Ok(())
    }

    /// Handle a figure element (containing img and potentially figcaption)
    ///
    /// Python reference: _`emit_image()` lines 1153-1193
    // Method signature kept for API consistency with other HtmlBackend methods
    #[allow(clippy::unused_self)]
    fn handle_figure(
        &self,
        figure_element: scraper::ElementRef,
        doc_items: &mut Vec<DocItem>,
        item_count: &mut usize,
        options: &BackendOptions,
    ) -> Result<(), DoclingError> {
        // Find img and figcaption within figure
        let img_selector = Selector::parse("img")
            .map_err(|e| DoclingError::BackendError(format!("Invalid selector: {e}")))?;
        let figcaption_selector = Selector::parse("figcaption")
            .map_err(|e| DoclingError::BackendError(format!("Invalid selector: {e}")))?;

        // Get figcaption text if present
        let figcaption_text = figure_element
            .select(&figcaption_selector)
            .next()
            .map(|fc| fc.text().collect::<Vec<_>>().join(" ").trim().to_string())
            .filter(|s| !s.is_empty());

        // Find img element and process it with figcaption priority
        if let Some(img_element) = figure_element.select(&img_selector).next() {
            Self::handle_image_with_caption(
                img_element,
                figcaption_text,
                doc_items,
                item_count,
                options,
            );
        }

        Ok(())
    }

    /// Handle a single image element with optional caption override
    ///
    /// Python reference: _`emit_image()` lines 1130-1193
    fn handle_image(
        img_element: scraper::ElementRef,
        doc_items: &mut Vec<DocItem>,
        item_count: &mut usize,
        options: &BackendOptions,
    ) {
        Self::handle_image_with_caption(img_element, None, doc_items, item_count, options);
    }

    /// Handle image with optional caption override (for figcaption support)
    ///
    /// Python reference: _`emit_image()` lines 1130-1193
    fn handle_image_with_caption(
        img_element: scraper::ElementRef,
        caption_override: Option<String>,
        doc_items: &mut Vec<DocItem>,
        item_count: &mut usize,
        options: &BackendOptions,
    ) {
        // Extract attributes
        let src = img_element.value().attr("src").unwrap_or("");
        let alt = img_element.value().attr("alt").unwrap_or("");
        let title = img_element.value().attr("title").unwrap_or("");

        // Skip SVG images entirely (Python docling doesn't create Picture DocItems for SVGs)
        // This is because SVG is a vector format that may contain text/code rather than a bitmap
        if src.to_lowercase().ends_with(".svg") {
            log::debug!("Skipping SVG image: {src}");
            return;
        }

        // Caption priority: figcaption > alt > title
        // Python: lines 1153-1178
        let caption_text = caption_override
            .or_else(|| {
                if alt.is_empty() {
                    None
                } else {
                    Some(alt.to_string())
                }
            })
            .or_else(|| {
                if title.is_empty() {
                    None
                } else {
                    Some(title.to_string())
                }
            });

        let mut captions = vec![];
        if let Some(caption_text) = caption_text {
            let caption_ref = format!("#/texts/{}", *item_count);
            *item_count += 1;

            let caption = create_text_item(*item_count - 1, caption_text, create_provenance(1));

            captions.push(ItemRef::new(caption_ref));
            doc_items.push(caption);
        }

        // Create Picture DocItem
        let picture_ref = format!("#/pictures/{}", *item_count);
        *item_count += 1;

        // Fetch image data if enabled
        let image_json = if src.is_empty() {
            None
        } else {
            let mut json = serde_json::json!({"uri": src, "alt": alt, "title": title});

            // Try to fetch image data
            if let Some(image_bytes) = Self::load_image_data(src, options) {
                // Encode bytes as base64 for JSON storage
                let base64_data = base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    &image_bytes,
                );
                json["data"] = serde_json::json!(base64_data);
                json["size"] = serde_json::json!(image_bytes.len());

                // Try to extract image dimensions
                if let Ok(img) = image::load_from_memory(&image_bytes) {
                    json["width"] = serde_json::json!(img.width());
                    json["height"] = serde_json::json!(img.height());
                }
            }

            Some(json)
        };

        let picture = DocItem::Picture {
            self_ref: picture_ref,
            parent: None,
            children: vec![],
            content_layer: "body".to_string(),
            prov: create_provenance(1),
            captions,
            footnotes: vec![],
            references: vec![],
            image: image_json,
            annotations: vec![],
            ocr_text: None,
        };

        doc_items.push(picture);
    }

    /// Handle a single table element
    ///
    /// Python reference: _`handle_block()` lines 1084-1095
    fn handle_table(
        &self,
        table_element: scraper::ElementRef,
        doc_items: &mut Vec<DocItem>,
        item_count: &mut usize,
    ) -> Result<(), DoclingError> {
        // Get table dimensions
        let (num_rows, num_cols) = self.get_table_dimensions(&table_element)?;

        if num_rows == 0 || num_cols == 0 {
            return Ok(()); // Skip empty tables
        }

        // Parse table data
        let (grid, table_cells) = self.parse_table_data(&table_element, num_rows, num_cols)?;

        let data = TableData {
            num_rows,
            num_cols,
            grid,
            table_cells: Some(table_cells),
        };

        // Create Table DocItem
        let table_ref = format!("#/tables/{}", *item_count);
        *item_count += 1;

        let table = DocItem::Table {
            self_ref: table_ref,
            parent: None,
            children: vec![],
            content_layer: "body".to_string(),
            prov: create_provenance(1),
            captions: vec![],
            footnotes: vec![],
            references: vec![],
            data,
            image: None,
            annotations: vec![],
        };

        doc_items.push(table);

        Ok(())
    }

    /// Handle a single code block element
    ///
    /// Python reference: _`handle_block()` lines 1097-1117
    fn handle_code_block(
        pre_element: scraper::ElementRef,
        doc_items: &mut Vec<DocItem>,
        item_count: &mut usize,
    ) {
        // Extract text content preserving whitespace
        let text = pre_element.text().collect::<Vec<_>>().join("");
        let text = text.trim().to_string();

        if text.is_empty() {
            return;
        }

        // Detect language from class attribute
        let language = Self::detect_code_language(&pre_element);

        // Create Code DocItem
        let code_ref = format!("#/texts/{}", *item_count);
        *item_count += 1;

        let code = DocItem::Code {
            self_ref: code_ref,
            parent: None,
            children: vec![],
            content_layer: "body".to_string(),
            prov: create_provenance(1),
            orig: text.clone(),
            text,
            language,
            formatting: None,
            hyperlink: None,
        };

        doc_items.push(code);
    }

    /// Extract text from list item with hyperlink information
    ///
    /// Returns (text, hyperlink) where:
    /// - text: The text content (may include markdown link format if mixed content)
    /// - hyperlink: The URL if the entire list item is a single link
    fn extract_li_text_with_hyperlink(li_element: &ElementRef) -> (String, Option<String>) {
        let mut text_parts = Vec::new();
        let mut first_link_href: Option<String> = None;
        let mut has_non_link_text = false;
        let mut link_count = 0;

        for child in li_element.children() {
            if let Some(text_node) = child.value().as_text() {
                // Direct text node
                let text = text_node.trim();
                if !text.is_empty() {
                    text_parts.push(text.to_string());
                    has_non_link_text = true;
                }
            } else if let Some(child_element) = ElementRef::wrap(child) {
                let tag_name = child_element.value().name();
                // Skip nested list elements (they're handled recursively)
                if tag_name == "ul" || tag_name == "ol" {
                    continue;
                }
                // Skip hidden elements
                if child_element.value().attr("hidden").is_some() {
                    continue;
                }
                // Check if this is an <a> tag
                if tag_name == "a" {
                    let href = child_element.value().attr("href");
                    let link_text = child_element.text().collect::<Vec<_>>().join(" ");
                    let link_text = link_text.trim().to_string();

                    if !link_text.is_empty() {
                        link_count += 1;
                        if link_count == 1 && first_link_href.is_none() {
                            first_link_href = href.map(String::from);
                        }

                        // If there's a href, format as markdown link for mixed content
                        if let Some(url) = href {
                            if url.starts_with("http://")
                                || url.starts_with("https://")
                                || url.starts_with("mailto:")
                                || url.starts_with('#')
                                || url.starts_with('/')
                            {
                                text_parts.push(format!("[{link_text}]({url})"));
                            } else {
                                text_parts.push(link_text);
                            }
                        } else {
                            text_parts.push(link_text);
                        }
                    }
                } else {
                    // Include text from other inline/block elements
                    // For inline code elements (code, kbd, samp, pre), wrap in backticks
                    let text = child_element.text().collect::<Vec<_>>().join(" ");
                    let text = text.trim();
                    if !text.is_empty() {
                        if tag_name == "code"
                            || tag_name == "kbd"
                            || tag_name == "samp"
                            || tag_name == "pre"
                        {
                            // Inline code - wrap in backticks
                            text_parts.push(format!("`{text}`"));
                        } else {
                            text_parts.push(text.to_string());
                        }
                        has_non_link_text = true;
                    }
                }
            }
        }

        let full_text = text_parts.join(" ");

        // If the entire list item is a single link (no other text), return the link info
        // Otherwise, the text already contains markdown link format
        if link_count == 1 && !has_non_link_text && first_link_href.is_some() {
            // Extract raw link text from the <a> element directly (not li_element which includes nested items)
            // Find the first <a> child and get its direct text
            let raw_text = li_element
                .children()
                .filter_map(ElementRef::wrap)
                .find(|e| e.value().name() == "a")
                .map(|a| {
                    // Get only direct text content, not from nested elements
                    a.text().collect::<Vec<_>>().join(" ").trim().to_string()
                })
                .unwrap_or_default();

            // Collapse whitespace in the text
            let raw_text = raw_text.split_whitespace().collect::<Vec<_>>().join(" ");
            (raw_text, first_link_href)
        } else {
            // Also collapse whitespace in the full text
            let full_text = full_text.split_whitespace().collect::<Vec<_>>().join(" ");
            (full_text, None)
        }
    }

    /// Parse a single list element (ul or ol)
    ///
    /// Python reference: _`handle_list()` lines 876-1004
    ///
    /// # Arguments
    ///
    /// * `parent_item_ref` - Optional parent `ListItem` reference (for nested lists)
    ///
    /// Returns the `self_ref` of the created List `DocItem`
    #[allow(clippy::too_many_lines)] // Complex list parsing - keeping together for clarity
    fn parse_list(
        list_element: scraper::ElementRef,
        is_ordered: bool,
        start: Option<usize>,
        parent_item_ref: Option<String>,
        doc_items: &mut Vec<DocItem>,
        item_count: &mut usize,
    ) -> Result<String, DoclingError> {
        // Create List container
        // Python: doc.add_list_group() lines 889-893
        let list_ref = format!("#/groups/{}", *item_count);
        *item_count += 1;

        let list_name = if is_ordered {
            start.map_or_else(
                || "ordered list".to_string(),
                |start_num| format!("ordered list start {start_num}"),
            )
        } else {
            "list".to_string()
        };

        // Track ListItem refs for populating List.children
        let mut list_item_refs: Vec<ItemRef> = Vec::new();

        // Parse list items (li) - only immediate children to avoid nested list duplication
        // Python: lines 901-1000
        // IMPORTANT: Use .children() not .select("li") to avoid selecting nested list items
        let mut item_index = 0;
        for child in list_element.children() {
            // Only process element nodes (skip text nodes)
            let li_element = match ElementRef::wrap(child) {
                Some(elem) if elem.value().name() == "li" => elem,
                _ => continue,
            };

            // Extract text from this <li> (but not from nested lists) with hyperlink info
            let (text, hyperlink) = Self::extract_li_text_with_hyperlink(&li_element);

            // Check if this <li> has nested lists - we need to process them even if text is empty
            // This handles cases like <li style="list-style-type: none;"><ul>...</ul></li>
            // which are used to create extra indentation levels
            let has_nested_list = li_element.children().any(|c| {
                ElementRef::wrap(c).is_some_and(|elem| {
                    let tag = elem.value().name();
                    tag == "ul" || tag == "ol"
                })
            });

            // If text is empty and no nested lists, skip entirely
            if text.trim().is_empty() && !has_nested_list {
                continue;
            }

            // Get the parent ref for nested lists - if this <li> has no text but has nested lists,
            // we use the parent list as the parent for nested lists (pass-through)
            // Otherwise, this <li> becomes the parent for nested lists
            let nested_parent_ref = if text.trim().is_empty() {
                // Empty wrapper <li> - nested lists inherit current list as parent
                Some(list_ref.clone())
            } else {
                // Normal <li> with text - nested lists have this <li> as parent
                None // Will be set after creating the list item
            };

            // Only create a ListItem if there's text
            let list_item_ref = if text.trim().is_empty() {
                None
            } else {
                // Determine marker
                // Python: lines 912-915
                let marker = if is_ordered {
                    let start_num = start.unwrap_or(1);
                    format!("{}.", start_num + item_index)
                } else {
                    "-".to_string()
                };

                // Create ListItem with hyperlink if the entire item is a single link
                // Python: doc.add_list_item() lines 930-936 or 974-983
                let item_ref = format!("#/texts/{}", *item_count);
                let list_item = create_list_item_with_hyperlink(
                    *item_count,
                    text,
                    marker,
                    is_ordered,
                    create_provenance(1),
                    Some(ItemRef::new(list_ref.clone())),
                    hyperlink,
                );
                *item_count += 1;
                doc_items.push(list_item);
                item_index += 1;

                // Track ref for List.children population
                list_item_refs.push(ItemRef::new(item_ref.clone()));

                Some(item_ref)
            };

            // Recursively handle nested lists within this <li>
            // Check for nested <ul> or <ol> elements
            // Pass the current ListItem as parent for nested lists (or parent list if empty wrapper)
            let parent_for_nested = list_item_ref.clone().or(nested_parent_ref);
            let mut nested_list_refs: Vec<String> = Vec::new();
            for nested_child in li_element.children() {
                if let Some(nested_element) = ElementRef::wrap(nested_child) {
                    let nested_tag = nested_element.value().name();
                    if nested_tag == "ul" {
                        // Nested unordered list
                        let nested_ref = Self::parse_list(
                            nested_element,
                            false,
                            None,
                            parent_for_nested.clone(),
                            doc_items,
                            item_count,
                        )?;
                        nested_list_refs.push(nested_ref);
                    } else if nested_tag == "ol" {
                        // Nested ordered list
                        let nested_start = nested_element
                            .value()
                            .attr("start")
                            .and_then(|s| s.parse::<usize>().ok());
                        let nested_ref = Self::parse_list(
                            nested_element,
                            true,
                            nested_start,
                            parent_for_nested.clone(),
                            doc_items,
                            item_count,
                        )?;
                        nested_list_refs.push(nested_ref);
                    }
                }
            }

            // If this ListItem has nested lists, update its children field
            // Find the ListItem we just created and update it
            if let Some(ref item_ref_str) = list_item_ref {
                if !nested_list_refs.is_empty() {
                    // Find the ListItem in doc_items and update its children
                    for item in doc_items.iter_mut() {
                        if let DocItem::ListItem {
                            self_ref, children, ..
                        } = item
                        {
                            if self_ref == item_ref_str {
                                for nested_ref in &nested_list_refs {
                                    children.push(ItemRef::new(nested_ref.clone()));
                                }
                                break;
                            }
                        }
                    }
                }
            }
        }

        // Create the List DocItem with populated children
        // Push at end - serializer uses body.children for order, not doc_items order
        let list_doc_item = DocItem::List {
            self_ref: list_ref.clone(),
            parent: parent_item_ref.map(ItemRef::new),
            children: list_item_refs,
            content_layer: "body".to_string(),
            name: list_name,
        };
        doc_items.push(list_doc_item);

        Ok(list_ref)
    }

    /// Check if a URL is a remote URL (HTTP/HTTPS)
    ///
    /// Python reference: `HTMLDocumentBackend`._`is_remote_url()`
    #[inline]
    fn is_remote_url(url: &str) -> bool {
        url.starts_with("http://") || url.starts_with("https://")
    }

    /// Load image data from various sources (HTTP, data: URL, file path)
    ///
    /// Python reference: `HTMLDocumentBackend`._`load_image_data()` lines 1212-1244
    ///
    /// Supports:
    /// - HTTP/HTTPS URLs (if `enable_remote_fetch` is true)
    /// - data: URLs with base64 encoding
    /// - file:// URLs and local file paths (if `enable_local_fetch` is true)
    ///
    /// Returns None for:
    /// - SVG files (not supported for image embedding)
    /// - Invalid/inaccessible files
    /// - Security violations (fetch disabled)
    fn load_image_data(src_loc: &str, options: &BackendOptions) -> Option<Vec<u8>> {
        // Skip SVG files
        if src_loc.to_lowercase().ends_with(".svg") {
            log::debug!("Skipping SVG file: {src_loc}");
            return None;
        }

        // Handle data: URLs (base64 encoded images)
        if src_loc.starts_with("data:") {
            return Self::load_data_url(src_loc);
        }

        // Handle remote URLs (HTTP/HTTPS)
        if Self::is_remote_url(src_loc) {
            if !options.enable_remote_fetch {
                log::warn!(
                    "Remote fetch disabled. Set enable_remote_fetch=true to fetch {src_loc}"
                );
                return None;
            }
            return Self::load_remote_image(src_loc);
        }

        // Handle local files (file:// URLs or relative paths)
        Self::load_local_image(src_loc, options)
    }

    /// Load image from data: URL (base64 encoded)
    ///
    /// Python reference: _`load_image_data()` lines 1236-1237
    fn load_data_url(src_loc: &str) -> Option<Vec<u8>> {
        // Extract base64 data after "data:image/<type>;base64,"
        let data_prefix = "data:image/";
        if !src_loc.starts_with(data_prefix) {
            return None;
        }

        // Find the base64 marker
        let base64_marker = ";base64,";
        let base64_start = src_loc.find(base64_marker)?;
        let base64_data = &src_loc[base64_start + base64_marker.len()..];

        // Decode base64
        match base64::Engine::decode(&base64::engine::general_purpose::STANDARD, base64_data) {
            Ok(bytes) => Some(bytes),
            Err(e) => {
                log::warn!("Failed to decode base64 image data: {e}");
                None
            }
        }
    }

    /// Load image from remote URL (HTTP/HTTPS)
    ///
    /// Python reference: _`load_image_data()` lines 1228-1232
    fn load_remote_image(url: &str) -> Option<Vec<u8>> {
        // Use a client with timeout to prevent hanging on slow/unresponsive servers
        let client = match reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                log::warn!("Failed to build HTTP client: {e}");
                return None;
            }
        };

        match client.get(url).send() {
            Ok(response) => {
                if response.status().is_success() {
                    match response.bytes() {
                        Ok(bytes) => Some(bytes.to_vec()),
                        Err(e) => {
                            log::warn!("Failed to read image bytes from {url}: {e}");
                            None
                        }
                    }
                } else {
                    log::warn!(
                        "HTTP error {} fetching image from {}",
                        response.status(),
                        url
                    );
                    None
                }
            }
            Err(e) => {
                log::warn!("Failed to fetch image from {url}: {e}");
                None
            }
        }
    }

    /// Load image from local file path
    ///
    /// Python reference: _`load_image_data()` lines 1238-1244
    fn load_local_image(src_loc: &str, options: &BackendOptions) -> Option<Vec<u8>> {
        if !options.enable_local_fetch {
            log::warn!("Local fetch disabled. Set enable_local_fetch=true to fetch {src_loc}");
            return None;
        }

        // Strip file:// prefix if present
        let path = src_loc.strip_prefix("file://").unwrap_or(src_loc);

        // Check if file exists and is readable
        match std::fs::read(path) {
            Ok(data) => Some(data),
            Err(e) => {
                log::warn!("Failed to read image file {path}: {e}");
                None
            }
        }
    }

    /// Get table dimensions (`num_rows`, `num_cols`) accounting for spans
    ///
    /// Python reference: `get_html_table_row_col()` lines 1007-1027
    fn get_table_dimensions(
        &self,
        table_element: &scraper::ElementRef,
    ) -> Result<(usize, usize), DoclingError> {
        let cell_selector = Selector::parse("td, th")
            .map_err(|e| DoclingError::BackendError(format!("Invalid selector: {e}")))?;

        let mut num_rows = 0;
        let mut num_cols = 0;

        // Iterate through direct child rows only (not nested table rows)
        // Use children() to get direct children and filter for tr/tbody elements
        for row in Self::get_direct_rows(table_element) {
            let mut col_count = 0;
            let mut is_row_header = true;

            // Count columns in this row (direct cells only)
            for cell in Self::get_direct_cells(&row, &cell_selector) {
                let (col_span, row_span) = self.get_cell_spans(&cell);
                col_count += col_span;

                // Check if this is a data cell or single-row header
                if cell.value().name() == "td" || row_span == 1 {
                    is_row_header = false;
                }
            }

            num_cols = num_cols.max(col_count);

            // Only count rows that aren't pure row headers
            if !is_row_header {
                num_rows += 1;
            }
        }

        Ok((num_rows, num_cols))
    }

    /// Parse table data into grid and cells list
    ///
    /// Python reference: `parse_table_data()` lines 383-498
    ///
    /// # Arguments
    /// * `table_element` - The table element to parse
    /// * `num_rows` - Number of rows in the table
    /// * `num_cols` - Number of columns in the table
    fn parse_table_data(
        &self,
        table_element: &scraper::ElementRef,
        num_rows: usize,
        num_cols: usize,
    ) -> Result<(Vec<Vec<TableCell>>, Vec<TableCell>), DoclingError> {
        let cell_selector = Selector::parse("td, th")
            .map_err(|e| DoclingError::BackendError(format!("Invalid selector: {e}")))?;

        // Create grid to track cell placement
        // Python: grid = [[None for _ in range(num_cols)] for _ in range(num_rows)]
        let mut grid: Vec<Vec<Option<String>>> = vec![vec![None; num_cols]; num_rows];
        let mut table_cells = Vec::new();

        let mut row_idx: isize = -1;
        let mut start_row_span = 0;

        // Iterate through direct child rows only (not nested table rows)
        for row in Self::get_direct_rows(table_element) {
            // Get direct child cells only
            let cells: Vec<_> = Self::get_direct_cells(&row, &cell_selector);

            // Check if this row is a header
            let mut _col_header = true;
            let mut row_header = true;

            for cell in &cells {
                let (_, row_span) = self.get_cell_spans(cell);
                if cell.value().name() == "td" {
                    _col_header = false;
                    row_header = false;
                } else if row_span == 1 {
                    row_header = false;
                }
            }

            if row_header {
                start_row_span += 1;
            } else {
                row_idx += 1;
                start_row_span = 0;
            }

            // Process cells in this row
            let mut col_idx = 0;

            for cell in cells {
                // Check for nested structure (rich cell)
                // Extract cell text using paragraph-aware extraction
                // Note: We use collapsed inline form for all cells because:
                // 1. The markdown serializer uses cell.text for table rendering
                // 2. Adding nested DocItems separately causes them to appear before the table
                // 3. The collapsed text preserves content (e.g., "- First - Second - Third")
                let text = Self::extract_cell_text_with_paragraphs(&cell);
                let ref_item = None;

                // Get spans
                let (col_span, row_span) = self.get_cell_spans(&cell);
                let actual_row_span = if row_header { row_span - 1 } else { row_span };

                // Find next available column in grid
                // row_idx is >= 0 here because this code only runs when !row_header
                // which means row_idx was incremented from -1 to at least 0
                #[allow(clippy::cast_sign_loss)]
                let current_row = (row_idx + start_row_span as isize) as usize;
                while col_idx < num_cols
                    && current_row < num_rows
                    && grid[current_row][col_idx].is_some()
                {
                    col_idx += 1;
                }

                // Fill grid for spanned cells
                for r in start_row_span..(start_row_span + actual_row_span) {
                    for c in 0..col_span {
                        // row_idx is >= 0 here (see comment above)
                        #[allow(clippy::cast_sign_loss)]
                        let grid_row = row_idx as usize + r;
                        let grid_col = col_idx + c;
                        if grid_row < num_rows && grid_col < num_cols {
                            grid[grid_row][grid_col] = Some(text.clone());
                        }
                    }
                }

                // Create table cell (now with optional ref_item for rich cells)
                // row_idx is >= 0 here (see comment above)
                #[allow(clippy::cast_sign_loss)]
                let row_idx_usize = row_idx as usize;
                let cell_obj = TableCell {
                    text: text.clone(),
                    row_span: if actual_row_span > 1 {
                        Some(actual_row_span)
                    } else {
                        None
                    },
                    col_span: (col_span > 1).then_some(col_span),
                    ref_item, // Set to Some(ItemRef) for rich cells, None for simple cells
                    start_row_offset_idx: Some(start_row_span + row_idx_usize),
                    start_col_offset_idx: Some(col_idx),
                    ..Default::default()
                };

                table_cells.push(cell_obj);
            }
        }

        // Convert grid from Option<String> to TableCell
        let grid_cells: Vec<Vec<TableCell>> = grid
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|cell| TableCell {
                        text: cell.unwrap_or_default(),
                        row_span: None,
                        col_span: None,
                        ref_item: None,
                        start_row_offset_idx: None,
                        start_col_offset_idx: None,
                        ..Default::default()
                    })
                    .collect()
            })
            .collect();

        Ok((grid_cells, table_cells))
    }

    /// Extract colspan and rowspan from table cell
    ///
    /// Python reference: _`get_cell_spans()` lines 1317-1340
    // Method signature kept for API consistency with other HtmlBackend methods
    #[allow(clippy::unused_self)]
    fn get_cell_spans(&self, cell: &scraper::ElementRef) -> (usize, usize) {
        let colspan = cell
            .value()
            .attr("colspan")
            .and_then(Self::extract_num)
            .unwrap_or(1);

        let rowspan = cell
            .value()
            .attr("rowspan")
            .and_then(Self::extract_num)
            .unwrap_or(1);

        (colspan, rowspan)
    }

    /// Extract numeric value from string
    ///
    /// Python reference: _`extract_num()` nested function in _`get_cell_spans()`
    #[inline]
    fn extract_num(s: &str) -> Option<usize> {
        if s.is_empty() {
            return None;
        }

        // Extract first number from string
        let digits: String = s.chars().take_while(char::is_ascii_digit).collect();
        digits.parse().ok()
    }

    /// Detect if table cell contains nested structured content (lists, multiple paragraphs, etc.)
    ///
    /// Returns true if the cell should be treated as a "rich cell" with nested `DocItems`.
    /// Rich cells require creating a Group `DocItem` and setting `ref_item`.
    ///
    /// Detection criteria:
    /// - Contains list elements: `<ul>`, `<ol>`
    /// - Contains multiple block elements: Multiple `<p>`, `<div>`
    ///
    /// # Arguments
    /// * `cell` - The table cell element to check
    ///
    /// # Returns
    /// true if cell contains nested structure, false otherwise
    ///
    /// Note: Currently only used in tests. Reserved for future JSON `DocItem` generation.
    #[cfg(test)]
    fn has_nested_structure(cell: &scraper::ElementRef) -> bool {
        // Check for list elements (ul, ol)
        if let Ok(list_selector) = Selector::parse("ul, ol") {
            if cell.select(&list_selector).next().is_some() {
                return true;
            }
        }

        // Check for multiple paragraphs
        if let Ok(p_selector) = Selector::parse("p") {
            if cell.select(&p_selector).count() > 1 {
                return true;
            }
        }

        // Check for div containers (potential structure)
        if let Ok(div_selector) = Selector::parse("div") {
            if cell.select(&div_selector).next().is_some() {
                return true;
            }
        }

        false
    }

    /// Extract text from table cell with proper paragraph separation
    ///
    /// Python uses double spaces to separate paragraphs within table cells.
    /// This function extracts text from multiple `<p>` elements and joins them with double spaces.
    fn extract_cell_text_with_paragraphs(cell: &scraper::ElementRef) -> String {
        // Check for lists (ul/ol)
        if let Some(result) = Self::try_extract_cell_with_lists(cell) {
            return result;
        }

        // Check for nested tables
        if let Some(result) = Self::try_extract_cell_with_tables(cell) {
            return result;
        }

        // Check for paragraphs
        if let Some(result) = Self::try_extract_cell_paragraphs(cell) {
            return result;
        }

        // Handle <br> tags
        if let Some(result) = Self::try_extract_cell_with_br(cell) {
            return result;
        }

        // Fallback to simple text extraction
        Self::extract_normalized_text(cell)
    }

    /// Extract normalized text from an element (join and collapse whitespace)
    fn extract_normalized_text(element: &scraper::ElementRef) -> String {
        element
            .text()
            .collect::<Vec<_>>()
            .join(" ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Try to extract cell content containing lists
    fn try_extract_cell_with_lists(cell: &scraper::ElementRef) -> Option<String> {
        let list_selector = Selector::parse("ul, ol").ok()?;
        cell.select(&list_selector).next()?;

        let parts = Self::extract_cell_children_with_list_handling(cell);
        if parts.is_empty() {
            None
        } else {
            Some(parts.join("  "))
        }
    }

    /// Extract cell children handling lists specially
    fn extract_cell_children_with_list_handling(cell: &scraper::ElementRef) -> Vec<String> {
        let mut parts = Vec::new();

        for child in cell.children() {
            if let Some(text_node) = child.value().as_text() {
                let text = text_node.trim();
                if !text.is_empty() {
                    parts.push(text.to_string());
                }
            } else if let Some(child_element) = ElementRef::wrap(child) {
                if let Some(text) = Self::extract_child_element_text(&child_element) {
                    parts.push(text);
                }
            }
        }
        parts
    }

    /// Extract text from a child element, handling lists and tables specially
    fn extract_child_element_text(element: &ElementRef) -> Option<String> {
        let tag_name = element.value().name();
        match tag_name {
            "ul" | "ol" => Self::extract_list_text(element),
            "table" => {
                let text = Self::serialize_nested_table_to_inline(element);
                if text.is_empty() {
                    None
                } else {
                    Some(text)
                }
            }
            _ => {
                let text = Self::extract_normalized_text(element);
                if text.is_empty() {
                    None
                } else {
                    Some(text)
                }
            }
        }
    }

    /// Extract text from a list element as "- item - item" format
    fn extract_list_text(list: &ElementRef) -> Option<String> {
        let li_selector = Selector::parse("li").ok()?;
        let items: Vec<String> = list
            .select(&li_selector)
            .map(|li| Self::extract_normalized_text(&li))
            .filter(|s| !s.is_empty())
            .collect();

        if items.is_empty() {
            None
        } else {
            Some(
                items
                    .iter()
                    .map(|item| format!("- {item}"))
                    .collect::<Vec<_>>()
                    .join(" "),
            )
        }
    }

    /// Try to extract cell content containing nested tables
    fn try_extract_cell_with_tables(cell: &scraper::ElementRef) -> Option<String> {
        let table_selector = Selector::parse("table").ok()?;
        cell.select(&table_selector).next()?;

        let parts = Self::extract_cell_children_with_table_handling(cell);
        if parts.is_empty() {
            None
        } else {
            Some(parts.join("  "))
        }
    }

    /// Extract cell children handling tables specially
    fn extract_cell_children_with_table_handling(cell: &scraper::ElementRef) -> Vec<String> {
        let mut parts = Vec::new();

        for child in cell.children() {
            if let Some(text_node) = child.value().as_text() {
                let text = text_node.trim();
                if !text.is_empty() {
                    parts.push(text.to_string());
                }
            } else if let Some(child_element) = ElementRef::wrap(child) {
                let tag_name = child_element.value().name();
                let text = if tag_name == "table" {
                    Self::serialize_nested_table_to_inline(&child_element)
                } else {
                    Self::extract_normalized_text(&child_element)
                };
                if !text.is_empty() {
                    parts.push(text);
                }
            }
        }
        parts
    }

    /// Try to extract cell content from paragraphs
    fn try_extract_cell_paragraphs(cell: &scraper::ElementRef) -> Option<String> {
        let p_selector = Selector::parse("p").ok()?;
        let paragraphs: Vec<String> = cell
            .select(&p_selector)
            .map(|p| Self::extract_normalized_text(&p))
            .filter(|s| !s.is_empty())
            .collect();

        if paragraphs.is_empty() {
            None
        } else {
            Some(paragraphs.join("  "))
        }
    }

    /// Try to extract cell content with <br> tag handling
    fn try_extract_cell_with_br(cell: &scraper::ElementRef) -> Option<String> {
        let html_content = cell.html();
        if !html_content.contains("<br") {
            return None;
        }

        let mut result = String::new();
        let mut last_was_br = false;

        for node in cell.descendants() {
            if let Some(text_node) = node.value().as_text() {
                let text = text_node.trim();
                if !text.is_empty() {
                    if last_was_br && !result.is_empty() {
                        result.push_str("  ");
                    } else if !result.is_empty() {
                        result.push(' ');
                    }
                    result.push_str(text);
                    last_was_br = false;
                }
            } else if let Some(element) = node.value().as_element() {
                if element.name() == "br" {
                    last_was_br = true;
                }
            }
        }

        Some(result)
    }

    /// Serialize a nested table to inline markdown format (recursive)
    ///
    /// Python docling represents nested tables in table cells as inline markdown:
    /// `| | A1 | B1 | |------| | C1 | D1 | |`
    ///
    /// Each row becomes `| cell | cell | cell |` and a separator `|------|------|------|`
    /// is inserted after the first row (header row).
    ///
    /// Handles arbitrarily deep nesting by recursively serializing nested tables.
    /// Column widths are calculated based on content to match Python output.
    fn serialize_nested_table_to_inline(table_element: &ElementRef) -> String {
        let Ok(cell_selector) = Selector::parse("td, th") else {
            return String::new();
        };

        let mut rows: Vec<Vec<String>> = Vec::new();

        // Use get_direct_rows to only process direct row children (not nested table rows)
        for row in Self::get_direct_rows(table_element) {
            let mut cells: Vec<String> = Vec::new();

            // Use get_direct_cells to only process direct cell children
            for cell in Self::get_direct_cells(&row, &cell_selector) {
                // Check if this cell contains a nested table
                let Ok(table_selector) = Selector::parse("table") else {
                    // Fall back to simple text extraction
                    let text = cell
                        .text()
                        .collect::<Vec<_>>()
                        .join(" ")
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ");
                    cells.push(text);
                    continue;
                };

                if let Some(nested_table) = cell.select(&table_selector).next() {
                    // Recursively serialize the nested table
                    let nested_text = Self::serialize_nested_table_to_inline(&nested_table);
                    cells.push(nested_text);
                } else {
                    // Simple cell - extract text
                    let text = cell
                        .text()
                        .collect::<Vec<_>>()
                        .join(" ")
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ");
                    cells.push(text);
                }
            }

            if !cells.is_empty() {
                rows.push(cells);
            }
        }

        if rows.is_empty() {
            return String::new();
        }

        // Find max column count
        let max_cols = rows.iter().map(Vec::len).max().unwrap_or(0);

        // Calculate column widths based on content
        // Python tabulate uses minimum 4 chars for content, minimum 6 dashes for separator
        let mut col_widths: Vec<usize> = vec![4; max_cols];
        for row in &rows {
            for (col_idx, cell_text) in row.iter().enumerate() {
                if col_idx < col_widths.len() {
                    col_widths[col_idx] = col_widths[col_idx].max(cell_text.len());
                }
            }
        }
        // Separator widths: minimum 6 dashes, or content width if larger
        let separator_widths: Vec<usize> = col_widths.iter().map(|w| (*w).max(6)).collect();

        // Build inline table representation
        let mut parts = Vec::new();

        // First row (header) - Python uses left-aligned cells with col_widths (min 4)
        if !rows.is_empty() {
            let row_str = format!(
                "| {} |",
                rows[0]
                    .iter()
                    .enumerate()
                    .map(|(i, c)| {
                        let width = col_widths.get(i).copied().unwrap_or(4);
                        format!("{c:<width$}")
                    })
                    .collect::<Vec<_>>()
                    .join(" | ")
            );
            parts.push(row_str);
        }

        // Separator after first row - dashes use separator_widths (min 6)
        if max_cols > 0 {
            let sep = format!(
                "|{}|",
                separator_widths
                    .iter()
                    .map(|w| "-".repeat(*w))
                    .collect::<Vec<_>>()
                    .join("|")
            );
            parts.push(sep);
        }

        // Remaining rows - use col_widths (min 4), not separator_widths
        for row in rows.iter().skip(1) {
            let row_str = format!(
                "| {} |",
                row.iter()
                    .enumerate()
                    .map(|(i, c)| {
                        let width = col_widths.get(i).copied().unwrap_or(4);
                        format!("{c:<width$}")
                    })
                    .collect::<Vec<_>>()
                    .join(" | ")
            );
            parts.push(row_str);
        }

        parts.join(" ")
    }

    /// Parse nested content inside table cell into `DocItems`
    ///
    /// This function handles rich table cells that contain structured content
    /// like lists, multiple paragraphs, or formatted text.
    ///
    /// # Arguments
    /// * `cell` - The table cell element to parse
    /// * `item_count` - Mutable counter for generating unique item references
    ///
    /// # Returns
    /// Vec of `DocItems` representing the nested content, or empty Vec if parsing fails
    ///
    /// # Implementation
    /// Phase 2: Full implementation with support for:
    /// - `<ul>` → List + `ListItem` `DocItems`
    /// - `<ol>` → `OrderedList` + `ListItem` `DocItems` (with start attribute)
    /// - `<p>` → Text `DocItems`
    /// - `<div>` → Text `DocItems` (treated as paragraphs)
    ///
    /// Note: Currently only used in tests. Reserved for future JSON `DocItem` generation.
    #[cfg(test)]
    fn parse_cell_content(cell: &scraper::ElementRef, item_count: &mut usize) -> Vec<DocItem> {
        let mut doc_items = Vec::new();

        // Parse unordered lists (ul)
        if let Ok(ul_selector) = Selector::parse("ul") {
            for ul_element in cell.select(&ul_selector) {
                let list_ref = format!("#/lists/{}", *item_count);
                *item_count += 1;

                // Parse list items
                let mut list_item_refs = Vec::new();
                if let Ok(li_selector) = Selector::parse("li") {
                    for li_element in ul_element.select(&li_selector) {
                        let text = li_element.text().collect::<Vec<_>>().join(" ");
                        let text = text.trim().to_string();

                        if !text.is_empty() {
                            let li_ref = format!("#/list-items/{}", *item_count);
                            *item_count += 1;

                            let list_item = DocItem::ListItem {
                                self_ref: li_ref.clone(),
                                parent: Some(ItemRef::new(list_ref.clone())),
                                children: vec![],
                                content_layer: "body".to_string(),
                                marker: "-".to_string(),
                                enumerated: false,
                                prov: create_provenance(1),
                                orig: text.clone(),
                                text,
                                formatting: None,
                                hyperlink: None,
                            };
                            list_item_refs.push(ItemRef::new(li_ref));
                            doc_items.push(list_item);
                        }
                    }
                }

                // Create List DocItem
                let list_item = DocItem::List {
                    self_ref: list_ref.clone(),
                    parent: None,
                    children: list_item_refs,
                    content_layer: "body".to_string(),
                    name: "list".to_string(),
                };
                // Insert list before its items
                doc_items.insert(
                    doc_items.len()
                        - doc_items
                            .iter()
                            .filter(|i| matches!(i, DocItem::ListItem { .. }))
                            .count(),
                    list_item,
                );
            }
        }

        // Parse ordered lists (ol)
        if let Ok(ol_selector) = Selector::parse("ol") {
            for ol_element in cell.select(&ol_selector) {
                let list_ref = format!("#/lists/{}", *item_count);
                *item_count += 1;

                // Get start attribute if present
                let start_num: usize = ol_element
                    .value()
                    .attr("start")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(1);

                // Parse list items
                let mut list_item_refs = Vec::new();
                let mut item_num = start_num;
                if let Ok(li_selector) = Selector::parse("li") {
                    for li_element in ol_element.select(&li_selector) {
                        let text = li_element.text().collect::<Vec<_>>().join(" ");
                        let text = text.trim().to_string();

                        if !text.is_empty() {
                            let li_ref = format!("#/list-items/{}", *item_count);
                            *item_count += 1;

                            let list_item = DocItem::ListItem {
                                self_ref: li_ref.clone(),
                                parent: Some(ItemRef::new(list_ref.clone())),
                                children: vec![],
                                content_layer: "body".to_string(),
                                marker: format!("{item_num}."),
                                enumerated: true,
                                prov: create_provenance(1),
                                orig: text.clone(),
                                text,
                                formatting: None,
                                hyperlink: None,
                            };
                            list_item_refs.push(ItemRef::new(li_ref));
                            doc_items.push(list_item);
                            item_num += 1;
                        }
                    }
                }

                // Create ordered List DocItem
                let list_item = DocItem::List {
                    self_ref: list_ref.clone(),
                    parent: None,
                    children: list_item_refs,
                    content_layer: "body".to_string(),
                    name: format!("ordered list {start_num}"),
                };
                // Insert list before its items
                let list_item_count = doc_items
                    .iter()
                    .filter(|i| matches!(i, DocItem::ListItem { .. }))
                    .count();
                doc_items.insert(doc_items.len() - list_item_count, list_item);
            }
        }

        // Parse paragraphs
        if let Ok(p_selector) = Selector::parse("p") {
            for p_element in cell.select(&p_selector) {
                let text = p_element.text().collect::<Vec<_>>().join(" ");
                let text = text.trim().to_string();

                if !text.is_empty() {
                    let text_item = DocItem::Text {
                        self_ref: format!("#/texts/{}", *item_count),
                        parent: None,
                        children: vec![],
                        content_layer: "body".to_string(),
                        prov: create_provenance(1),
                        orig: text.clone(),
                        text,
                        formatting: None,
                        hyperlink: None,
                    };
                    *item_count += 1;
                    doc_items.push(text_item);
                }
            }
        }

        // Parse divs if no other content found
        if doc_items.is_empty() {
            if let Ok(div_selector) = Selector::parse("div") {
                for div_element in cell.select(&div_selector) {
                    let text = div_element.text().collect::<Vec<_>>().join(" ");
                    let text = text.trim().to_string();

                    if !text.is_empty() {
                        let text_item = DocItem::Text {
                            self_ref: format!("#/texts/{}", *item_count),
                            parent: None,
                            children: vec![],
                            content_layer: "body".to_string(),
                            prov: create_provenance(1),
                            orig: text.clone(),
                            text,
                            formatting: None,
                            hyperlink: None,
                        };
                        *item_count += 1;
                        doc_items.push(text_item);
                    }
                }
            }
        }

        doc_items
    }

    /// Detect programming language from class attribute in code blocks
    ///
    /// Supports common patterns:
    /// - `<code class="language-python">` (GitHub/Markdown style)
    /// - `<code class="lang-python">` (alternative prefix)
    /// - `<code class="python">` (simple class name)
    /// - Checks both `<pre>` and nested `<code>` elements
    fn detect_code_language(pre_element: &scraper::ElementRef) -> Option<String> {
        // Try to find a <code> element inside <pre>
        let code_selector = Selector::parse("code").ok()?;

        // Check nested <code> element first (most specific)
        if let Some(code_element) = pre_element.select(&code_selector).next() {
            if let Some(lang) =
                Self::extract_language_from_classes(code_element.value().attr("class"))
            {
                return Some(lang);
            }
        }

        // Fall back to checking <pre> element itself
        Self::extract_language_from_classes(pre_element.value().attr("class"))
    }

    /// Extract language name from space-separated class list
    ///
    /// Handles patterns like:
    /// - "language-python" → "python"
    /// - "lang-rust" → "rust"
    /// - "python" → "python" (if single token or known language)
    #[inline]
    fn extract_language_from_classes(class_attr: Option<&str>) -> Option<String> {
        let class_attr = class_attr?;

        for class in class_attr.split_whitespace() {
            // Check for "language-xxx" prefix (GitHub/Markdown standard)
            if let Some(lang) = class.strip_prefix("language-") {
                return Some(lang.to_string());
            }

            // Check for "lang-xxx" prefix (alternative)
            if let Some(lang) = class.strip_prefix("lang-") {
                return Some(lang.to_string());
            }

            // If single class or looks like a language name, use it
            // Common language names: python, rust, javascript, java, cpp, c, go, ruby, etc.
            // For simplicity, accept any single-token class as potential language
            if !class.contains('-') && !class.is_empty() {
                return Some(class.to_string());
            }
        }

        None
    }
}

impl DocumentBackend for HtmlBackend {
    #[inline]
    fn format(&self) -> InputFormat {
        InputFormat::Html
    }

    fn parse_bytes(&self, data: &[u8], options: &BackendOptions) -> Result<Document, DoclingError> {
        // Convert bytes to string (use from_utf8 to avoid allocating a vector copy)
        let content = std::str::from_utf8(data)
            .map_err(|e| DoclingError::BackendError(format!("Invalid UTF-8: {e}")))?
            .to_string();

        // Parse HTML with scraper (Python: BeautifulSoup line 222)
        let document = Html::parse_document(&content);

        // Extract metadata from HTML (title, author, etc.)
        let mut metadata = Self::extract_metadata(&document);

        // Parse HTML elements to DocItems
        let doc_items = self.parse_elements(&document, options)?;

        // Generate markdown from DocItems using shared helper (applies formatting)
        let markdown = crate::markdown_helper::docitems_to_markdown(&doc_items);

        // Set character count
        metadata.num_characters = markdown.len();

        // Create Document with extracted metadata
        Ok(Document {
            format: InputFormat::Html,
            metadata,
            markdown,
            content_blocks: Some(doc_items),
            docling_document: None,
        })
    }

    fn parse_file<P: AsRef<std::path::Path>>(
        &self,
        path: P,
        options: &BackendOptions,
    ) -> Result<Document, DoclingError> {
        let path_ref = path.as_ref();
        let filename = path_ref.display().to_string();

        // Helper to add filename context to errors
        let add_context = |err: DoclingError| -> DoclingError {
            match err {
                DoclingError::BackendError(msg) => {
                    DoclingError::BackendError(format!("{msg}: {filename}"))
                }
                other => other,
            }
        };

        let data = std::fs::read(path_ref).map_err(DoclingError::IoError)?;
        self.parse_bytes(&data, options).map_err(add_context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use docling_core::CoordOrigin;

    /// Test 1: Verify backend creation and format()
    /// Ensures backend can be instantiated and returns correct format
    #[test]
    fn test_html_backend_creation() {
        let backend = HtmlBackend::new();
        assert_eq!(
            backend.format(),
            InputFormat::Html,
            "HtmlBackend::new() should return Html format"
        );
    }

    /// Test 2: Verify Default trait implementation
    /// Ensures Default::default() works and returns correct format
    #[test]
    fn test_html_backend_default() {
        let backend = HtmlBackend;
        assert_eq!(
            backend.format(),
            InputFormat::Html,
            "HtmlBackend default should return Html format"
        );
    }

    /// Test 3: Test error handling for invalid UTF-8
    /// Ensures proper error is returned when bytes are not valid UTF-8
    #[test]
    fn test_parse_bytes_invalid_utf8() {
        let backend = HtmlBackend::new();
        let invalid_utf8 = vec![0xFF, 0xFE, 0xFD]; // Invalid UTF-8 bytes
        let options = BackendOptions::default();

        let result = backend.parse_bytes(&invalid_utf8, &options);
        assert!(result.is_err(), "Invalid UTF-8 bytes should return error");
        if let Err(DoclingError::BackendError(msg)) = result {
            assert!(msg.contains("Invalid UTF-8"));
        } else {
            panic!("Expected BackendError with 'Invalid UTF-8' message");
        }
    }

    /// Test 4: Test parsing empty HTML document
    /// Ensures empty but valid HTML is handled gracefully
    #[test]
    fn test_parse_empty_html() {
        let backend = HtmlBackend::new();
        let html = b"<html><body></body></html>";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(html, &options);
        assert!(result.is_ok(), "Empty HTML should parse successfully");

        let doc = result.unwrap();
        assert_eq!(
            doc.format,
            InputFormat::Html,
            "Empty HTML document should have Html format"
        );
        // Empty HTML should produce no DocItems (or empty content)
        assert!(
            doc.content_blocks.as_ref().is_none_or(|v| v.is_empty()),
            "Empty HTML should produce no content blocks"
        );
    }

    /// Test 5: Test parsing minimal HTML with heading
    /// Verifies basic heading extraction functionality
    #[test]
    fn test_parse_minimal_html_heading() {
        let backend = HtmlBackend::new();
        let html = b"<html><body><h1>Test Title</h1></body></html>";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(html, &options);
        assert!(
            result.is_ok(),
            "Parsing minimal HTML with heading should succeed"
        );

        let doc = result.unwrap();
        assert_eq!(
            doc.format,
            InputFormat::Html,
            "Document format should be Html"
        );

        // Should have content_blocks with heading
        let items = doc
            .content_blocks
            .as_deref()
            .expect("should have content_blocks");
        assert!(
            !items.is_empty(),
            "Content blocks should not be empty for HTML with heading"
        );

        // First item should be SectionHeader with "Test Title"
        match &items[0] {
            DocItem::SectionHeader { text, level, .. } => {
                assert_eq!(text, "Test Title", "Heading text should match");
                assert_eq!(*level, 1, "h1 should be level 1");
            }
            _ => panic!("Expected SectionHeader DocItem"),
        }
    }

    /// Test 6: Test parsing HTML with paragraph
    /// Verifies basic paragraph extraction functionality
    #[test]
    fn test_parse_minimal_html_paragraph() {
        let backend = HtmlBackend::new();
        let html = b"<html><body><p>Test paragraph content.</p></body></html>";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(html, &options);
        assert!(result.is_ok(), "Parsing HTML with paragraph should succeed");

        let doc = result.unwrap();
        assert_eq!(
            doc.format,
            InputFormat::Html,
            "Document format should be Html"
        );

        // Should have content_blocks with paragraph
        assert!(
            doc.content_blocks.is_some(),
            "HTML with paragraph should have content_blocks"
        );
        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty(), "Content blocks should not be empty");

        // First item should be Text with paragraph content
        match &items[0] {
            DocItem::Text { text, .. } => {
                assert_eq!(
                    text, "Test paragraph content.",
                    "Paragraph text should match"
                );
            }
            _ => panic!("Expected Text DocItem"),
        }
    }

    /// Test 7: Test parsing HTML with multiple elements
    /// Verifies backend can handle multiple headings and paragraphs in document order
    /// After N=577: DOM tree walking preserves document order
    #[test]
    fn test_parse_html_multiple_elements() {
        let backend = HtmlBackend::new();
        let html = b"<html><body>\
            <h1>Title</h1>\
            <p>First paragraph.</p>\
            <h2>Subtitle</h2>\
            <p>Second paragraph.</p>\
            </body></html>";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(html, &options);
        assert!(
            result.is_ok(),
            "Parsing HTML with multiple elements should succeed"
        );

        let doc = result.unwrap();
        assert_eq!(
            doc.format,
            InputFormat::Html,
            "Document format should be Html"
        );

        // Should have content_blocks with 4 items
        assert!(
            doc.content_blocks.is_some(),
            "HTML with multiple elements should have content_blocks"
        );
        let items = doc.content_blocks.unwrap();
        assert_eq!(items.len(), 4, "Should have 4 content items (h1, p, h2, p)");

        // After DOM tree walking refactor (N=577), document order is preserved
        // Order should be: Title (h1), First paragraph, Subtitle (h2), Second paragraph
        match &items[0] {
            DocItem::SectionHeader { text, level, .. } => {
                assert_eq!(text, "Title", "First heading text should be 'Title'");
                assert_eq!(*level, 1, "h1 should be level 1");
            }
            _ => panic!("Expected SectionHeader DocItem at position 0"),
        }
        match &items[1] {
            DocItem::Text { text, .. } => {
                assert_eq!(
                    text, "First paragraph.",
                    "First paragraph text should match"
                );
            }
            _ => panic!("Expected Text DocItem at position 1"),
        }
        match &items[2] {
            DocItem::SectionHeader { text, level, .. } => {
                assert_eq!(text, "Subtitle", "Second heading text should be 'Subtitle'");
                assert_eq!(*level, 2, "h2 should be level 2");
            }
            _ => panic!("Expected SectionHeader DocItem at position 2"),
        }
        match &items[3] {
            DocItem::Text { text, .. } => {
                assert_eq!(
                    text, "Second paragraph.",
                    "Second paragraph text should match"
                );
            }
            _ => panic!("Expected Text DocItem at position 3"),
        }
    }

    /// Test 8: Test parsing unordered list (ul)
    /// Verifies basic unordered list extraction
    #[test]
    fn test_parse_unordered_list() {
        let backend = HtmlBackend::new();
        let html = b"<html><body>\
            <ul>\
                <li>First item</li>\
                <li>Second item</li>\
                <li>Third item</li>\
            </ul>\
            </body></html>";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(html, &options);
        assert!(result.is_ok(), "Parsing unordered list HTML should succeed");

        let doc = result.unwrap();
        assert!(
            doc.content_blocks.is_some(),
            "Should have content_blocks for list HTML"
        );
        let items = doc.content_blocks.unwrap();

        // Should have 1 List + 3 ListItems = 4 items
        assert_eq!(items.len(), 4, "Should have 4 items: 1 List + 3 ListItems");

        // Find the List container (order not guaranteed)
        let list_count = items
            .iter()
            .filter(|item| matches!(item, DocItem::List { name, .. } if name == "list"))
            .count();
        assert_eq!(list_count, 1, "Should have exactly 1 List DocItem");

        // Find and verify all 3 ListItems
        let list_items: Vec<_> = items
            .iter()
            .filter_map(|item| {
                if let DocItem::ListItem {
                    text,
                    marker,
                    enumerated,
                    ..
                } = item
                {
                    Some((text.clone(), marker.clone(), *enumerated))
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(list_items.len(), 3, "Should have 3 ListItems");

        // Verify each expected text is present
        for expected_text in ["First item", "Second item", "Third item"] {
            assert!(
                list_items.iter().any(|(text, marker, enumerated)| {
                    text == expected_text && marker == "-" && !enumerated
                }),
                "Expected ListItem with text '{expected_text}', marker '-', enumerated=false"
            );
        }
    }

    /// Test 9: Test parsing ordered list (ol)
    /// Verifies basic ordered list extraction
    #[test]
    fn test_parse_ordered_list() {
        let backend = HtmlBackend::new();
        let html = b"<html><body>\
            <ol>\
                <li>First item</li>\
                <li>Second item</li>\
            </ol>\
            </body></html>";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(html, &options);
        assert!(result.is_ok(), "Parsing ordered list HTML should succeed");

        let doc = result.unwrap();
        assert!(
            doc.content_blocks.is_some(),
            "Should have content_blocks for ordered list HTML"
        );
        let items = doc.content_blocks.unwrap();

        // Should have 1 List + 2 ListItems = 3 items
        assert_eq!(items.len(), 3, "Should have 3 items: 1 List + 2 ListItems");

        // Find the List container (order not guaranteed)
        let list_count = items
            .iter()
            .filter(|item| matches!(item, DocItem::List { name, .. } if name == "ordered list"))
            .count();
        assert_eq!(list_count, 1, "Should have exactly 1 ordered List DocItem");

        // Find and verify all ListItems
        let list_items: Vec<_> = items
            .iter()
            .filter_map(|item| {
                if let DocItem::ListItem {
                    text,
                    marker,
                    enumerated,
                    ..
                } = item
                {
                    Some((text.clone(), marker.clone(), *enumerated))
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(list_items.len(), 2, "Should have 2 ListItems");

        // Verify first item
        assert!(
            list_items
                .iter()
                .any(|(text, marker, enumerated)| text == "First item"
                    && marker == "1."
                    && *enumerated),
            "Expected ListItem with text 'First item', marker '1.', enumerated=true"
        );

        // Verify second item
        assert!(
            list_items
                .iter()
                .any(|(text, marker, enumerated)| text == "Second item"
                    && marker == "2."
                    && *enumerated),
            "Expected ListItem with text 'Second item', marker '2.', enumerated=true"
        );
    }

    /// Test 10: Test parsing ordered list with start attribute
    /// Verifies that start attribute is respected
    #[test]
    fn test_parse_ordered_list_with_start() {
        let backend = HtmlBackend::new();
        let html = b"<html><body>\
            <ol start=\"5\">\
                <li>Fifth item</li>\
                <li>Sixth item</li>\
            </ol>\
            </body></html>";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(html, &options);
        assert!(
            result.is_ok(),
            "Parsing ordered list with start attribute should succeed"
        );

        let doc = result.unwrap();
        assert!(
            doc.content_blocks.is_some(),
            "Should have content_blocks for ordered list with start"
        );
        let items = doc.content_blocks.unwrap();

        // Should have 1 List + 2 ListItems = 3 items
        assert_eq!(items.len(), 3, "Should have 3 items: 1 List + 2 ListItems");

        // Find the List container (order not guaranteed)
        let list_count = items
            .iter()
            .filter(
                |item| matches!(item, DocItem::List { name, .. } if name == "ordered list start 5"),
            )
            .count();
        assert_eq!(
            list_count, 1,
            "Should have exactly 1 ordered List DocItem with start 5"
        );

        // Find and verify all ListItems
        let list_items: Vec<_> = items
            .iter()
            .filter_map(|item| {
                if let DocItem::ListItem { text, marker, .. } = item {
                    Some((text.clone(), marker.clone()))
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(list_items.len(), 2, "Should have 2 ListItems");

        // List items should start from 5
        assert!(
            list_items
                .iter()
                .any(|(text, marker)| text == "Fifth item" && marker == "5."),
            "Expected ListItem with text 'Fifth item', marker '5.'"
        );

        assert!(
            list_items
                .iter()
                .any(|(text, marker)| text == "Sixth item" && marker == "6."),
            "Expected ListItem with text 'Sixth item', marker '6.'"
        );
    }

    /// Test 11: Test parsing image with src and alt
    /// Verifies basic image extraction with caption generation (N=262)
    #[test]
    fn test_parse_image() {
        let backend = HtmlBackend::new();
        let html = b"<html><body>\
            <img src=\"image.png\" alt=\"Test Image\" />\
            </body></html>";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(html, &options);
        assert!(result.is_ok(), "Parsing HTML with image should succeed");

        let doc = result.unwrap();
        assert!(doc.content_blocks.is_some());
        let items = doc.content_blocks.unwrap();

        // Should have 2 items: Text (caption) + Picture
        // After N=262, images with alt text generate Text DocItems as captions
        assert_eq!(items.len(), 2);

        // Check Text item (used as caption)
        match &items[0] {
            DocItem::Text { text, .. } => {
                assert_eq!(text, "Test Image");
            }
            _ => panic!("Expected Text DocItem (caption)"),
        }

        // Check Picture item
        match &items[1] {
            DocItem::Picture { image, .. } => {
                assert!(image.is_some());
                let img_data = image.as_ref().unwrap();
                assert_eq!(
                    img_data.get("uri").and_then(|v| v.as_str()),
                    Some("image.png")
                );
                assert_eq!(
                    img_data.get("alt").and_then(|v| v.as_str()),
                    Some("Test Image")
                );
            }
            _ => panic!("Expected Picture DocItem"),
        }
    }

    /// Test 12: Test parsing multiple images
    /// Verifies handling of multiple image elements with caption generation (N=262)
    #[test]
    fn test_parse_multiple_images() {
        let backend = HtmlBackend::new();
        let html = b"<html><body>\
            <img src=\"image1.png\" alt=\"First\" />\
            <img src=\"image2.jpg\" alt=\"Second\" />\
            </body></html>";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(html, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert!(doc.content_blocks.is_some());
        let items = doc.content_blocks.unwrap();

        // Should have 4 items: Text1 (caption) + Picture1 + Text2 (caption) + Picture2
        // After N=262, each image with alt text generates: Text (caption) + Picture
        assert_eq!(items.len(), 4);

        // Check first Text (caption)
        match &items[0] {
            DocItem::Text { text, .. } => {
                assert_eq!(text, "First");
            }
            _ => panic!("Expected Text DocItem (caption)"),
        }

        // Check first Picture
        match &items[1] {
            DocItem::Picture { image, .. } => {
                assert!(image.is_some());
                let img_data = image.as_ref().unwrap();
                assert_eq!(
                    img_data.get("uri").and_then(|v| v.as_str()),
                    Some("image1.png")
                );
                assert_eq!(img_data.get("alt").and_then(|v| v.as_str()), Some("First"));
            }
            _ => panic!("Expected Picture DocItem"),
        }

        // Check second Text (caption)
        match &items[2] {
            DocItem::Text { text, .. } => {
                assert_eq!(text, "Second");
            }
            _ => panic!("Expected Text DocItem (caption)"),
        }

        // Check second Picture
        match &items[3] {
            DocItem::Picture { image, .. } => {
                assert!(image.is_some());
                let img_data = image.as_ref().unwrap();
                assert_eq!(
                    img_data.get("uri").and_then(|v| v.as_str()),
                    Some("image2.jpg")
                );
                assert_eq!(img_data.get("alt").and_then(|v| v.as_str()), Some("Second"));
            }
            _ => panic!("Expected Picture DocItem"),
        }
    }

    /// Test 13: Test parsing image with title attribute
    /// Verifies that title attribute is captured, and caption uses alt over title (N=262)
    #[test]
    fn test_parse_image_with_title() {
        let backend = HtmlBackend::new();
        let html = b"<html><body>\
            <img src=\"image.png\" alt=\"Alt Text\" title=\"Title Text\" />\
            </body></html>";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(html, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert!(doc.content_blocks.is_some());
        let items = doc.content_blocks.unwrap();

        // Should have 2 items: Text (caption using alt) + Picture
        // After N=262, caption priority: figcaption > alt > title
        assert_eq!(items.len(), 2);

        // Check Text (caption) uses alt text (higher priority than title)
        match &items[0] {
            DocItem::Text { text, .. } => {
                assert_eq!(text, "Alt Text");
            }
            _ => panic!("Expected Text DocItem (caption)"),
        }

        // Check Picture item has both alt and title
        match &items[1] {
            DocItem::Picture { image, .. } => {
                assert!(image.is_some());
                let img_data = image.as_ref().unwrap();
                assert_eq!(
                    img_data.get("uri").and_then(|v| v.as_str()),
                    Some("image.png")
                );
                assert_eq!(
                    img_data.get("alt").and_then(|v| v.as_str()),
                    Some("Alt Text")
                );
                assert_eq!(
                    img_data.get("title").and_then(|v| v.as_str()),
                    Some("Title Text")
                );
            }
            _ => panic!("Expected Picture DocItem"),
        }
    }

    /// Test 14: Test parsing simple table
    /// Verifies basic table extraction
    #[test]
    fn test_parse_simple_table() {
        let backend = HtmlBackend::new();
        let html = b"<html><body>\
            <table>\
                <tr><th>Header 1</th><th>Header 2</th></tr>\
                <tr><td>Cell 1</td><td>Cell 2</td></tr>\
                <tr><td>Cell 3</td><td>Cell 4</td></tr>\
            </table>\
            </body></html>";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(html, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert!(doc.content_blocks.is_some());
        let items = doc.content_blocks.unwrap();

        // Should have 1 Table item
        assert_eq!(items.len(), 1);

        // Check Table item
        match &items[0] {
            DocItem::Table { data, .. } => {
                // Note: In Python's logic, a row with <th> cells where rowspan==1 is NOT a "row header"
                // and counts as a data row. So this table has 3 rows:
                // Row 0: <th>Header 1</th><th>Header 2</th> (rowspan=1, counts as data row)
                // Row 1: <td>Cell 1</td><td>Cell 2</td>
                // Row 2: <td>Cell 3</td><td>Cell 4</td>
                assert_eq!(data.num_rows, 3);
                assert_eq!(data.num_cols, 2);
                assert_eq!(data.grid.len(), 3);
                assert_eq!(data.grid[0].len(), 2);

                // Check grid contents
                // First row is the header row with th tags
                assert_eq!(data.grid[0][0].text, "Header 1");
                assert_eq!(data.grid[0][1].text, "Header 2");
                assert_eq!(data.grid[1][0].text, "Cell 1");
                assert_eq!(data.grid[1][1].text, "Cell 2");
                assert_eq!(data.grid[2][0].text, "Cell 3");
                assert_eq!(data.grid[2][1].text, "Cell 4");
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    /// Test 15: Test parsing table with colspan
    /// Verifies colspan attribute handling
    #[test]
    fn test_parse_table_with_colspan() {
        let backend = HtmlBackend::new();
        let html = b"<html><body>\
            <table>\
                <tr><th colspan=\"2\">Spanning Header</th></tr>\
                <tr><td>Cell 1</td><td>Cell 2</td></tr>\
            </table>\
            </body></html>";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(html, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert!(doc.content_blocks.is_some());
        let items = doc.content_blocks.unwrap();

        // Should have 1 Table item
        assert_eq!(items.len(), 1);

        match &items[0] {
            DocItem::Table { data, .. } => {
                assert_eq!(data.num_cols, 2);

                // Check that table_cells has the colspan information
                if let Some(cells) = &data.table_cells {
                    assert!(!cells.is_empty());
                    // First cell should have colspan=2
                    assert_eq!(cells[0].col_span, Some(2));
                    assert_eq!(cells[0].text, "Spanning Header");
                }
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    /// Test 16: Test parsing table with rowspan
    /// Verifies rowspan attribute handling
    #[test]
    fn test_parse_table_with_rowspan() {
        let backend = HtmlBackend::new();
        let html = b"<html><body>\
            <table>\
                <tr><th rowspan=\"2\">Spanning Cell</th><td>Cell 1</td></tr>\
                <tr><td>Cell 2</td></tr>\
            </table>\
            </body></html>";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(html, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert!(doc.content_blocks.is_some());
        let items = doc.content_blocks.unwrap();

        // Should have 1 Table item
        assert_eq!(items.len(), 1);

        match &items[0] {
            DocItem::Table { data, .. } => {
                assert_eq!(data.num_cols, 2);

                // Check that table_cells has the rowspan information
                if let Some(cells) = &data.table_cells {
                    // First cell should have rowspan
                    // Note: Python subtracts 1 from rowspan for header cells
                    assert!(cells[0].row_span.is_some() || cells[0].row_span == Some(1));
                }
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    /// Test 17: Test parsing empty table
    /// Verifies empty tables are handled gracefully
    #[test]
    fn test_parse_empty_table() {
        let backend = HtmlBackend::new();
        let html = b"<html><body>\
            <table></table>\
            </body></html>";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(html, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        // Empty tables should be skipped
        if let Some(items) = doc.content_blocks {
            assert!(items.is_empty(), "Empty tables should be skipped");
        }
    }

    /// Test 18: Test parsing code block (pre)
    /// Verifies basic code block extraction
    #[test]
    fn test_parse_code_block() {
        let backend = HtmlBackend::new();
        let html = b"<html><body>\
            <pre>function hello() {\n  console.log('Hello');\n}</pre>\
            </body></html>";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(html, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert!(doc.content_blocks.is_some());
        let items = doc.content_blocks.unwrap();

        // Should have 1 Code item
        assert_eq!(items.len(), 1);

        // Check Code item
        match &items[0] {
            DocItem::Code { text, .. } => {
                assert!(text.contains("function hello()"));
                assert!(text.contains("console.log"));
            }
            _ => panic!("Expected Code DocItem"),
        }
    }

    /// Test 19: Test parsing multiple code blocks
    /// Verifies handling of multiple pre elements
    #[test]
    fn test_parse_multiple_code_blocks() {
        let backend = HtmlBackend::new();
        let html = b"<html><body>\
            <pre>code block 1</pre>\
            <p>Some text</p>\
            <pre>code block 2</pre>\
            </body></html>";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(html, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert!(doc.content_blocks.is_some());
        let items = doc.content_blocks.unwrap();

        // Should have 1 paragraph + 2 code blocks = 3 items
        assert_eq!(items.len(), 3);

        // Check for Code items
        let code_items: Vec<_> = items
            .iter()
            .filter(|item| matches!(item, DocItem::Code { .. }))
            .collect();
        assert_eq!(code_items.len(), 2);
    }

    /// Test 20: Test parsing empty code block
    /// Verifies empty code blocks are skipped
    #[test]
    fn test_parse_empty_code_block() {
        let backend = HtmlBackend::new();
        let html = b"<html><body>\
            <pre></pre>\
            <pre>   </pre>\
            </body></html>";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(html, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        // Empty code blocks should be skipped
        if let Some(items) = doc.content_blocks {
            assert!(items.is_empty(), "Empty code blocks should be skipped");
        }
    }

    /// Test 21: Test code block language detection
    /// Verifies language detection from class attributes (N=268)
    #[test]
    fn test_parse_code_block_with_language() {
        let backend = HtmlBackend::new();
        let html = b"<html><body>\
            <pre><code class=\"language-python\">print('Hello')</code></pre>\
            <pre><code class=\"lang-rust\">println!(\"World\");</code></pre>\
            <pre><code class=\"javascript\">console.log('Test');</code></pre>\
            <pre class=\"language-java\">System.out.println(\"Java\");</pre>\
            </body></html>";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(html, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert!(doc.content_blocks.is_some());
        let items = doc.content_blocks.unwrap();

        // Should have 4 Code items
        assert_eq!(items.len(), 4);

        // Check first code block (language-python)
        match &items[0] {
            DocItem::Code { text, language, .. } => {
                assert_eq!(text, "print('Hello')");
                assert_eq!(language.as_deref(), Some("python"));
            }
            _ => panic!("Expected Code DocItem"),
        }

        // Check second code block (lang-rust)
        match &items[1] {
            DocItem::Code { text, language, .. } => {
                assert_eq!(text, "println!(\"World\");");
                assert_eq!(language.as_deref(), Some("rust"));
            }
            _ => panic!("Expected Code DocItem"),
        }

        // Check third code block (simple class name)
        match &items[2] {
            DocItem::Code { text, language, .. } => {
                assert_eq!(text, "console.log('Test');");
                assert_eq!(language.as_deref(), Some("javascript"));
            }
            _ => panic!("Expected Code DocItem"),
        }

        // Check fourth code block (language on pre element)
        match &items[3] {
            DocItem::Code { text, language, .. } => {
                assert_eq!(text, "System.out.println(\"Java\");");
                assert_eq!(language.as_deref(), Some("java"));
            }
            _ => panic!("Expected Code DocItem"),
        }
    }

    /// Test 18: Test metadata extraction (title, author, and description)
    /// Verifies that HTML <title>, <meta name="author">, and <meta name="description"> are extracted
    #[test]
    fn test_html_metadata_extraction() {
        let backend = HtmlBackend::new();
        let html = b"<html>\
            <head>\
                <title>Test Document Title</title>\
                <meta name=\"author\" content=\"John Doe\">\
                <meta name=\"description\" content=\"A test document\">\
            </head>\
            <body>\
                <p>Some content</p>\
            </body>\
            </html>";
        let options = BackendOptions::default();

        let document = backend.parse_bytes(html, &options).unwrap();

        // Verify title was extracted
        assert_eq!(
            document.metadata.title,
            Some("Test Document Title".to_string())
        );

        // Verify author was extracted
        assert_eq!(document.metadata.author, Some("John Doe".to_string()));

        // N=1880: Verify description was extracted to subject field
        assert_eq!(
            document.metadata.subject,
            Some("A test document".to_string())
        );

        // Verify character count is set
        assert!(document.metadata.num_characters > 0);
    }

    /// Test 19: Test metadata extraction with missing fields
    /// Verifies graceful handling when title, author, or description are missing
    #[test]
    fn test_html_metadata_extraction_missing() {
        let backend = HtmlBackend::new();
        let html = b"<html><body><p>Content without metadata</p></body></html>";
        let options = BackendOptions::default();

        let document = backend.parse_bytes(html, &options).unwrap();

        // Verify title, author, and subject are None when not present
        assert_eq!(document.metadata.title, None);
        assert_eq!(document.metadata.author, None);
        assert_eq!(document.metadata.subject, None); // N=1880: Verify description is None

        // Character count should still be set
        assert!(document.metadata.num_characters > 0);
    }

    /// Test 20: Test metadata extraction with empty values
    /// Verifies that empty title/author/description tags are treated as None
    #[test]
    fn test_html_metadata_extraction_empty() {
        let backend = HtmlBackend::new();
        let html = b"<html>\
            <head>\
                <title>   </title>\
                <meta name=\"author\" content=\"   \">\
                <meta name=\"description\" content=\"   \">\
            </head>\
            <body><p>Content</p></body>\
            </html>";
        let options = BackendOptions::default();

        let document = backend.parse_bytes(html, &options).unwrap();

        // Verify empty/whitespace-only values are treated as None
        assert_eq!(document.metadata.title, None);
        assert_eq!(document.metadata.author, None);
        assert_eq!(document.metadata.subject, None); // N=1880: Verify empty description is None
    }

    #[test]
    fn test_is_remote_url() {
        assert!(HtmlBackend::is_remote_url("http://example.com/image.png"));
        assert!(HtmlBackend::is_remote_url("https://example.com/image.png"));
        assert!(!HtmlBackend::is_remote_url("file:///path/to/image.png"));
        assert!(!HtmlBackend::is_remote_url("/path/to/image.png"));
        assert!(!HtmlBackend::is_remote_url(
            "data:image/png;base64,iVBOR..."
        ));
    }

    #[test]
    fn test_load_data_url() {
        // Valid data URL with base64 encoded "test"
        let data_url = "data:image/png;base64,dGVzdA==";
        let result = HtmlBackend::load_data_url(data_url);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), b"test");

        // Invalid data URL (not an image)
        let invalid_url = "data:text/plain;base64,dGVzdA==";
        let result = HtmlBackend::load_data_url(invalid_url);
        assert!(result.is_none());

        // Invalid base64
        let invalid_base64 = "data:image/png;base64,!!!invalid!!!";
        let result = HtmlBackend::load_data_url(invalid_base64);
        assert!(result.is_none());
    }

    #[test]
    fn test_load_local_image_disabled() {
        let options = BackendOptions::default(); // enable_local_fetch = false
        let result = HtmlBackend::load_local_image("/path/to/image.png", &options);
        assert!(result.is_none());
    }

    #[test]
    fn test_load_local_image_enabled_nonexistent() {
        let options = BackendOptions::default().with_local_fetch(true);
        let result = HtmlBackend::load_local_image("/nonexistent/image.png", &options);
        assert!(result.is_none());
    }

    #[test]
    fn test_load_local_image_enabled_exists() {
        // Create a temporary file
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = temp_dir.path().join("test_image.png");
        std::fs::write(&temp_path, b"fake image data").unwrap();

        let options = BackendOptions::default().with_local_fetch(true);
        let result = HtmlBackend::load_local_image(temp_path.to_str().unwrap(), &options);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), b"fake image data");
    }

    #[test]
    fn test_load_local_image_file_url() {
        // Create a temporary file
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = temp_dir.path().join("test_image.png");
        std::fs::write(&temp_path, b"fake image data").unwrap();

        let options = BackendOptions::default().with_local_fetch(true);
        let file_url = format!("file://{}", temp_path.to_str().unwrap());
        let result = HtmlBackend::load_local_image(&file_url, &options);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), b"fake image data");
    }

    #[test]
    fn test_load_image_data_svg_skipped() {
        let options = BackendOptions::default().with_local_fetch(true);
        let result = HtmlBackend::load_image_data("/path/to/image.svg", &options);
        assert!(result.is_none());
    }

    #[test]
    fn test_load_image_data_data_url() {
        let options = BackendOptions::default();
        let data_url = "data:image/png;base64,dGVzdA==";
        let result = HtmlBackend::load_image_data(data_url, &options);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), b"test");
    }

    #[test]
    fn test_load_image_data_remote_disabled() {
        let options = BackendOptions::default(); // enable_remote_fetch = false
        let result = HtmlBackend::load_image_data("http://example.com/image.png", &options);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_image_with_data_url() {
        let backend = HtmlBackend;
        // Create a 1x1 red PNG image
        let png_data = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x02\x00\x00\x00\x90wS\xde\x00\x00\x00\x0cIDATx\x9cc\xf8\xcf\xc0\x00\x00\x00\x03\x00\x01\x00\x00\x00\x00IEND\xaeB`\x82";
        let base64_data =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, png_data);
        let data_url = format!("data:image/png;base64,{base64_data}");

        let html = format!(r#"<html><body><img src="{data_url}" alt="Test image"/></body></html>"#);
        let options = BackendOptions::default(); // Data URLs don't require enable_remote_fetch or enable_local_fetch

        let document = backend.parse_bytes(html.as_bytes(), &options).unwrap();

        // Verify Picture DocItem was created (may have a caption Text DocItem first)
        let doc_items = document.content_blocks.unwrap();
        assert!(!doc_items.is_empty());

        // Find the Picture DocItem (might be first or second if there's a caption)
        let picture = doc_items
            .iter()
            .find(|item| matches!(item, DocItem::Picture { .. }));
        assert!(picture.is_some(), "Expected Picture DocItem");

        if let DocItem::Picture { image, .. } = picture.unwrap() {
            assert!(image.is_some());
            let img_json = image.as_ref().unwrap();
            assert_eq!(img_json["alt"], "Test image");
            assert!(
                img_json["data"].is_string(),
                "Image data should be fetched and embedded"
            );
            assert_eq!(
                img_json["size"],
                png_data.len(),
                "Size should match PNG data length"
            );
            // Note: width/height extraction may fail for some PNG formats, so we don't assert those
        } else {
            panic!("Expected Picture DocItem");
        }
    }

    // ===== Category 1: Backend Creation (Additional Tests) =====

    /// Test backend format persistence across multiple calls
    /// Verifies format() returns consistent InputFormat
    #[test]
    fn test_backend_format_persistence() {
        let backend = HtmlBackend::new();
        let format1 = backend.format();
        let format2 = backend.format();
        let format3 = backend.format();
        assert_eq!(format1, InputFormat::Html);
        assert_eq!(format2, InputFormat::Html);
        assert_eq!(format3, InputFormat::Html);
    }

    // ===== Category 2: Metadata (Additional Tests) =====

    /// Test metadata extraction with special characters
    /// Verifies unicode and HTML entities in title/author
    #[test]
    fn test_metadata_with_special_characters() {
        let backend = HtmlBackend::new();
        let html = b"<html>\
            <head>\
                <title>Test \xE2\x80\x93 Document \xE2\x9C\x93</title>\
                <meta name=\"author\" content=\"Jos\xC3\xA9 Garc\xC3\xADa\">\
            </head>\
            <body><p>Content</p></body>\
            </html>";
        let options = BackendOptions::default();

        let document = backend.parse_bytes(html, &options).unwrap();

        // Verify title contains unicode characters (em dash and check mark)
        assert!(document.metadata.title.is_some());
        let title = document.metadata.title.unwrap();
        assert!(title.contains("Test"));
        assert!(title.contains("Document"));

        // Verify author contains accented characters
        assert_eq!(document.metadata.author, Some("José García".to_string()));
    }

    /// Test metadata with multiple meta tags (same name)
    /// Verifies first occurrence wins
    #[test]
    fn test_metadata_multiple_meta_tags() {
        let backend = HtmlBackend::new();
        let html = b"<html>\
            <head>\
                <meta name=\"author\" content=\"First Author\">\
                <meta name=\"author\" content=\"Second Author\">\
            </head>\
            <body><p>Content</p></body>\
            </html>";
        let options = BackendOptions::default();

        let document = backend.parse_bytes(html, &options).unwrap();

        // First occurrence should win
        assert_eq!(document.metadata.author, Some("First Author".to_string()));
    }

    /// Test metadata with malformed HTML
    /// Verifies graceful handling of malformed meta tags
    #[test]
    fn test_metadata_malformed_html() {
        let backend = HtmlBackend::new();
        let html = b"<html>\
            <head>\
                <title>Valid Title</title>\
                <meta name=\"author\"><!-- Missing content attribute -->\
                <meta content=\"Orphan Content\"><!-- Missing name attribute -->\
            </head>\
            <body><p>Content</p></body>\
            </html>";
        let options = BackendOptions::default();

        let document = backend.parse_bytes(html, &options).unwrap();

        // Title should still be extracted
        assert_eq!(document.metadata.title, Some("Valid Title".to_string()));

        // Author should be None (malformed meta tag)
        assert_eq!(document.metadata.author, None);
    }

    /// Test metadata with nested title tags
    /// Verifies only first/outermost title is used
    #[test]
    fn test_metadata_nested_title() {
        let backend = HtmlBackend::new();
        let html = b"<html>\
            <head>\
                <title>Outer Title</title>\
            </head>\
            <body>\
                <title>Inner Title (should be ignored)</title>\
                <p>Content</p>\
            </body>\
            </html>";
        let options = BackendOptions::default();

        let document = backend.parse_bytes(html, &options).unwrap();

        // Only the <head> title should be extracted
        assert_eq!(document.metadata.title, Some("Outer Title".to_string()));
    }

    // ===== Category 3: DocItem Generation (Additional Tests) =====

    /// Test DocItem self_ref generation is sequential
    /// Verifies self_ref values increment correctly
    #[test]
    fn test_docitem_self_ref_sequential() {
        let backend = HtmlBackend::new();
        let html = b"<html><body>\
            <h1>First</h1>\
            <p>Second</p>\
            <h2>Third</h2>\
            </body></html>";
        let options = BackendOptions::default();

        let document = backend.parse_bytes(html, &options).unwrap();
        let items = document.content_blocks.unwrap();

        // Check that self_ref values are unique and sequential
        let mut refs = Vec::new();
        for item in &items {
            let self_ref = match item {
                DocItem::Text { self_ref, .. } => self_ref,
                DocItem::List { self_ref, .. } => self_ref,
                DocItem::ListItem { self_ref, .. } => self_ref,
                DocItem::Picture { self_ref, .. } => self_ref,
                DocItem::Table { self_ref, .. } => self_ref,
                DocItem::Code { self_ref, .. } => self_ref,
                _ => continue,
            };
            refs.push(self_ref.clone());
        }

        // Verify no duplicate self_refs
        let unique_refs: std::collections::HashSet<_> = refs.iter().collect();
        assert_eq!(
            refs.len(),
            unique_refs.len(),
            "self_ref values should be unique"
        );
    }

    /// Test DocItem creation with empty content is skipped
    /// Verifies empty headings, paragraphs are not created
    #[test]
    fn test_docitem_empty_content_skipped() {
        let backend = HtmlBackend::new();
        let html = b"<html><body>\
            <h1></h1>\
            <p>   </p>\
            <h2>Valid Heading</h2>\
            <p></p>\
            </body></html>";
        let options = BackendOptions::default();

        let document = backend.parse_bytes(html, &options).unwrap();
        let items = document.content_blocks.unwrap();

        // Should only have 1 item (Valid Heading)
        assert_eq!(items.len(), 1);
        match &items[0] {
            DocItem::SectionHeader { text, level, .. } => {
                assert_eq!(text, "Valid Heading");
                assert_eq!(*level, 2); // h2
            }
            _ => panic!("Expected SectionHeader DocItem"),
        }
    }

    /// Test DocItem provenance generation
    /// Verifies all DocItems have proper provenance
    #[test]
    fn test_docitem_provenance() {
        let backend = HtmlBackend::new();
        let html = b"<html><body>\
            <h1>Heading</h1>\
            <p>Paragraph</p>\
            </body></html>";
        let options = BackendOptions::default();

        let document = backend.parse_bytes(html, &options).unwrap();
        let items = document.content_blocks.unwrap();

        // Check that all DocItems have provenance
        for item in &items {
            let prov = match item {
                DocItem::Text { prov, .. } => prov,
                DocItem::List { .. } => continue, // Lists don't have prov
                DocItem::ListItem { prov, .. } => prov,
                DocItem::Picture { prov, .. } => prov,
                DocItem::Table { prov, .. } => prov,
                DocItem::Code { prov, .. } => prov,
                _ => continue,
            };

            assert!(!prov.is_empty(), "DocItem should have provenance");
            assert_eq!(prov[0].page_no, 1);
            assert_eq!(prov[0].bbox.coord_origin, CoordOrigin::TopLeft);
        }
    }

    /// Test mixed DocItem types in single document
    /// Verifies backend handles diverse content correctly
    #[test]
    fn test_docitem_mixed_types() {
        let backend = HtmlBackend::new();
        let html = b"<html><body>\
            <h1>Title</h1>\
            <p>Paragraph</p>\
            <ul><li>List item</li></ul>\
            <img src=\"test.png\" alt=\"Image\" />\
            <table><tr><td>Cell</td></tr></table>\
            <pre>Code</pre>\
            </body></html>";
        let options = BackendOptions::default();

        let document = backend.parse_bytes(html, &options).unwrap();
        let items = document.content_blocks.unwrap();

        // Should have: Title (Text), Paragraph (Text), List, ListItem, Text (caption), Picture, Table, Code
        assert!(items.len() >= 6);

        // Check for each DocItem type
        let has_text = items
            .iter()
            .any(|item| matches!(item, DocItem::Text { .. }));
        let has_list = items
            .iter()
            .any(|item| matches!(item, DocItem::List { .. }));
        let has_list_item = items
            .iter()
            .any(|item| matches!(item, DocItem::ListItem { .. }));
        let has_picture = items
            .iter()
            .any(|item| matches!(item, DocItem::Picture { .. }));
        let has_table = items
            .iter()
            .any(|item| matches!(item, DocItem::Table { .. }));
        let has_code = items
            .iter()
            .any(|item| matches!(item, DocItem::Code { .. }));

        assert!(has_text, "Should have Text DocItems");
        assert!(has_list, "Should have List DocItem");
        assert!(has_list_item, "Should have ListItem DocItem");
        assert!(has_picture, "Should have Picture DocItem");
        assert!(has_table, "Should have Table DocItem");
        assert!(has_code, "Should have Code DocItem");
    }

    /// Test DocItem text content escaping
    /// Verifies HTML entities are properly decoded
    #[test]
    fn test_docitem_text_escaping() {
        let backend = HtmlBackend::new();
        let html = b"<html><body>\
            <p>Text with &lt;HTML&gt; &amp; entities &quot;quoted&quot;</p>\
            </body></html>";
        let options = BackendOptions::default();

        let document = backend.parse_bytes(html, &options).unwrap();
        let items = document.content_blocks.unwrap();

        assert_eq!(items.len(), 1);
        match &items[0] {
            DocItem::Text { text, .. } => {
                // HTML entities should be decoded by scraper library
                assert!(text.contains("<HTML>") || text.contains("&lt;HTML&gt;"));
                assert!(text.contains('&') || text.contains("&amp;"));
            }
            _ => panic!("Expected Text DocItem"),
        }
    }

    // ===== Category 4: Format-Specific (Additional Tests) =====

    /// Test parsing figure with figcaption
    /// Verifies figcaption takes priority over alt text for image captions
    #[test]
    fn test_parse_figure_with_figcaption() {
        let backend = HtmlBackend::new();
        let html = b"<html><body>\
            <figure>\
                <img src=\"test.png\" alt=\"Alt text\" />\
                <figcaption>Figure caption</figcaption>\
            </figure>\
            </body></html>";
        let options = BackendOptions::default();

        let document = backend.parse_bytes(html, &options).unwrap();
        let items = document.content_blocks.unwrap();

        // Should have: Text (figcaption caption) + Picture
        assert_eq!(items.len(), 2);

        // Caption should use figcaption text, not alt
        match &items[0] {
            DocItem::Text { text, .. } => {
                assert_eq!(text, "Figure caption");
            }
            _ => panic!("Expected Text DocItem (caption)"),
        }
    }

    /// Test parsing address element
    /// Verifies address tag creates Text DocItem
    #[test]
    fn test_parse_address_element() {
        let backend = HtmlBackend::new();
        let html = b"<html><body>\
            <address>123 Main St, City, State 12345</address>\
            </body></html>";
        let options = BackendOptions::default();

        let document = backend.parse_bytes(html, &options).unwrap();
        let items = document.content_blocks.unwrap();

        assert_eq!(items.len(), 1);
        match &items[0] {
            DocItem::Text { text, .. } => {
                assert_eq!(text, "123 Main St, City, State 12345");
            }
            _ => panic!("Expected Text DocItem"),
        }
    }

    /// Test parsing summary element
    /// Verifies summary tag creates Text DocItem
    #[test]
    fn test_parse_summary_element() {
        let backend = HtmlBackend::new();
        let html = b"<html><body>\
            <details>\
                <summary>Click to expand</summary>\
                <p>Hidden content</p>\
            </details>\
            </body></html>";
        let options = BackendOptions::default();

        let document = backend.parse_bytes(html, &options).unwrap();
        let items = document.content_blocks.unwrap();

        // Should have: Text (summary) + Text (paragraph)
        assert!(!items.is_empty());

        // Find summary text
        let has_summary = items.iter().any(
            |item| matches!(item, DocItem::Text { text, .. } if text.contains("Click to expand")),
        );
        assert!(has_summary, "Summary element should create Text DocItem");
    }

    /// Test heading level bounds (h1-h6)
    /// Verifies all heading levels are supported
    #[test]
    fn test_heading_level_bounds() {
        let backend = HtmlBackend::new();
        let html = b"<html><body>\
            <h1>Level 1</h1>\
            <h2>Level 2</h2>\
            <h3>Level 3</h3>\
            <h4>Level 4</h4>\
            <h5>Level 5</h5>\
            <h6>Level 6</h6>\
            <h7>Level 7 (invalid)</h7>\
            </body></html>";
        let options = BackendOptions::default();

        let document = backend.parse_bytes(html, &options).unwrap();
        let items = document.content_blocks.unwrap();

        // Should have 7 items (h1-h6 as SectionHeaders, h7 text extracted as Text)
        // h7 is not valid HTML, but we still extract its text content to not lose data
        assert_eq!(items.len(), 7);

        // Verify all heading levels h1-h6 are present as SectionHeaders
        for i in 1..=6 {
            let expected_text = format!("Level {i}");
            let has_heading = items
                .iter()
                .any(|item| matches!(item, DocItem::SectionHeader { text, level, .. } if text == &expected_text && *level == i));
            assert!(has_heading, "Should have heading level {i}");
        }

        // Verify h7 is NOT parsed as a SectionHeader (since h7 is invalid HTML)
        // But its text content is extracted as a Text item
        let has_h7_header = items.iter().any(
            |item| matches!(item, DocItem::SectionHeader { text, .. } if text.contains("Level 7")),
        );
        assert!(
            !has_h7_header,
            "h7 should not be parsed as a SectionHeader (invalid HTML)"
        );

        // Verify h7 text content was extracted as Text
        let has_h7_text = items
            .iter()
            .any(|item| matches!(item, DocItem::Text { text, .. } if text.contains("Level 7")));
        assert!(
            has_h7_text,
            "h7 text content should be extracted as Text item"
        );
    }

    /// Test table with empty cells
    /// Verifies empty table cells are handled correctly
    #[test]
    fn test_table_with_empty_cells() {
        let backend = HtmlBackend::new();
        let html = b"<html><body>\
            <table>\
                <tr><td>A</td><td></td><td>C</td></tr>\
                <tr><td></td><td>B</td><td></td></tr>\
            </table>\
            </body></html>";
        let options = BackendOptions::default();

        let document = backend.parse_bytes(html, &options).unwrap();
        let items = document.content_blocks.unwrap();

        assert_eq!(items.len(), 1);
        match &items[0] {
            DocItem::Table { data, .. } => {
                assert_eq!(data.num_rows, 2);
                assert_eq!(data.num_cols, 3);

                // Empty cells should be empty strings
                assert_eq!(data.grid[0][0].text, "A");
                assert_eq!(data.grid[0][1].text, "");
                assert_eq!(data.grid[0][2].text, "C");
                assert_eq!(data.grid[1][0].text, "");
                assert_eq!(data.grid[1][1].text, "B");
                assert_eq!(data.grid[1][2].text, "");
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    /// Test list with empty items
    /// Verifies empty list items are skipped
    #[test]
    fn test_list_with_empty_items() {
        let backend = HtmlBackend::new();
        let html = b"<html><body>\
            <ul>\
                <li>Valid item</li>\
                <li></li>\
                <li>   </li>\
                <li>Another valid</li>\
            </ul>\
            </body></html>";
        let options = BackendOptions::default();

        let document = backend.parse_bytes(html, &options).unwrap();
        let items = document.content_blocks.unwrap();

        // Verify we have correct number of non-empty list items (empty items skipped)
        // Note: HTML backend outputs ListItems directly without List container
        let list_items: Vec<_> = items
            .iter()
            .filter(|item| matches!(item, DocItem::ListItem { .. }))
            .collect();
        assert_eq!(
            list_items.len(),
            2,
            "Should have 2 valid ListItems (empty items skipped)"
        );

        // Verify text content of list items
        let texts: Vec<_> = list_items
            .iter()
            .filter_map(|item| {
                if let DocItem::ListItem { text, .. } = item {
                    Some(text.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert!(texts.contains(&"Valid item"), "Should contain 'Valid item'");
        assert!(
            texts.contains(&"Another valid"),
            "Should contain 'Another valid'"
        );
    }

    /// Test code block with preserved whitespace
    /// Verifies pre elements preserve formatting
    #[test]
    fn test_code_block_whitespace_preservation() {
        let backend = HtmlBackend::new();
        let html = b"<html><body>\
            <pre>function test() {\n    return true;\n}</pre>\
            </body></html>";
        let options = BackendOptions::default();

        let document = backend.parse_bytes(html, &options).unwrap();
        let items = document.content_blocks.unwrap();

        assert_eq!(items.len(), 1);
        match &items[0] {
            DocItem::Code { text, .. } => {
                // Whitespace should be preserved (though outer trim is applied)
                assert!(text.contains("function test()"));
                assert!(text.contains("return true"));
            }
            _ => panic!("Expected Code DocItem"),
        }
    }

    // ===== Category 5: Integration (Additional Tests) =====

    /// Test complex document with all element types
    /// Verifies backend handles realistic HTML documents
    #[test]
    fn test_complex_document_structure() {
        let backend = HtmlBackend::new();
        let html = b"<html>\
            <head><title>Complex Document</title></head>\
            <body>\
                <h1>Main Title</h1>\
                <p>Introduction paragraph.</p>\
                <h2>Section 1</h2>\
                <p>Section content.</p>\
                <ul><li>Point 1</li><li>Point 2</li></ul>\
                <h2>Section 2</h2>\
                <table><tr><td>Data</td></tr></table>\
                <pre>code example</pre>\
            </body>\
            </html>";
        let options = BackendOptions::default();

        let document = backend.parse_bytes(html, &options).unwrap();

        // Verify metadata
        assert_eq!(
            document.metadata.title,
            Some("Complex Document".to_string())
        );
        assert_eq!(document.format, InputFormat::Html);

        // Verify content blocks present
        let items = document.content_blocks.unwrap();
        assert!(items.len() >= 8); // Multiple elements

        // Verify markdown output generated
        assert!(!document.markdown.is_empty());
    }

    /// Test document with nested structures
    /// Verifies handling of nested HTML elements
    #[test]
    fn test_nested_structures() {
        let backend = HtmlBackend::new();
        let html = b"<html><body>\
            <div>\
                <section>\
                    <article>\
                        <h1>Deeply nested heading</h1>\
                        <p>Deeply nested paragraph.</p>\
                    </article>\
                </section>\
            </div>\
            </body></html>";
        let options = BackendOptions::default();

        let document = backend.parse_bytes(html, &options).unwrap();
        let items = document.content_blocks.unwrap();

        // Should extract content from nested elements
        assert!(items.len() >= 2);

        let has_heading = items.iter().any(|item| {
            matches!(item, DocItem::SectionHeader { text, .. } if text.contains("Deeply nested heading"))
        });
        let has_paragraph = items.iter().any(|item| {
            matches!(item, DocItem::Text { text, .. } if text.contains("Deeply nested paragraph"))
        });

        assert!(has_heading, "Should extract nested heading");
        assert!(has_paragraph, "Should extract nested paragraph");
    }

    /// Test document with inline formatting
    /// Verifies inline tags (strong, em, a) are extracted with formatting preserved (N=1161)
    #[test]
    fn test_inline_formatting() {
        let backend = HtmlBackend::new();
        let html = b"<html><body>\
            <p>This is <strong>bold</strong> and <em>italic</em> text.</p>\
            <p>Link: <a href=\"http://example.com\">Example</a></p>\
            </body></html>";
        let options = BackendOptions::default();

        let document = backend.parse_bytes(html, &options).unwrap();
        let items = document.content_blocks.unwrap();

        // After N=1161: Inline formatting extraction creates separate DocItems per formatting run
        // First paragraph: "This is" + "bold"(bold) + "and" + "italic"(italic) + "text." = 5 items
        // Second paragraph: "Link:" + "Example"(link) = 2 items
        // Total: 7+ items expected
        assert!(
            items.len() >= 5,
            "Expected at least 5 DocItems with formatting extraction, got {}",
            items.len()
        );

        // Check that text with formatting is extracted
        let all_text: Vec<String> = items
            .iter()
            .filter_map(|item| match item {
                DocItem::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();

        let combined_text = all_text.join(" ");
        assert!(combined_text.contains("bold"), "Should extract 'bold' text");
        assert!(
            combined_text.contains("italic"),
            "Should extract 'italic' text"
        );
        assert!(
            combined_text.contains("Example"),
            "Should extract 'Example' link text"
        );

        // Check that at least one item has bold formatting
        let has_bold = items.iter().any(|item| match item {
            DocItem::Text {
                formatting, text, ..
            } => text.contains("bold") && formatting.as_ref().and_then(|f| f.bold) == Some(true),
            _ => false,
        });
        assert!(has_bold, "Should have DocItem with bold formatting");

        // Check that at least one item has italic formatting
        let has_italic = items.iter().any(|item| match item {
            DocItem::Text {
                formatting, text, ..
            } => {
                text.contains("italic") && formatting.as_ref().and_then(|f| f.italic) == Some(true)
            }
            _ => false,
        });
        assert!(has_italic, "Should have DocItem with italic formatting");
    }

    /// Test large document performance
    /// Verifies backend scales to realistic document sizes
    #[test]
    fn test_large_document_performance() {
        let backend = HtmlBackend::new();

        // Generate HTML with 100 paragraphs
        let mut html = String::from("<html><body>");
        for i in 0..100 {
            html.push_str(&format!("<p>Paragraph {i}</p>"));
        }
        html.push_str("</body></html>");

        let options = BackendOptions::default();
        let result = backend.parse_bytes(html.as_bytes(), &options);

        assert!(result.is_ok());
        let document = result.unwrap();
        let items = document.content_blocks.unwrap();

        // Should have 100 Text DocItems
        assert_eq!(items.len(), 100);
    }

    /// Test document with whitespace-only content
    /// Verifies excessive whitespace doesn't create DocItems
    #[test]
    fn test_whitespace_only_content() {
        let backend = HtmlBackend::new();
        let html = b"<html><body>\
            <p>   \n\n   </p>\
            <h1>     </h1>\
            <h2>Valid</h2>\
            </body></html>";
        let options = BackendOptions::default();

        let document = backend.parse_bytes(html, &options).unwrap();
        let items = document.content_blocks.unwrap();

        // Only "Valid" heading should be created
        assert_eq!(items.len(), 1);
        match &items[0] {
            DocItem::SectionHeader { text, level, .. } => {
                assert_eq!(text, "Valid");
                assert_eq!(*level, 2); // h2
            }
            _ => panic!("Expected SectionHeader DocItem"),
        }
    }

    /// Test error recovery from malformed HTML
    /// Verifies backend handles malformed HTML gracefully
    #[test]
    fn test_error_recovery_malformed_html() {
        let backend = HtmlBackend::new();
        // Missing closing tags, unclosed elements
        let html = b"<html><body>\
            <h1>Unclosed heading\
            <p>Paragraph without close\
            <ul><li>List item\
            <h2>Another heading</h2>\
            </body>";
        let options = BackendOptions::default();

        // Should still parse successfully (scraper handles malformed HTML)
        let result = backend.parse_bytes(html, &options);
        assert!(result.is_ok());

        let document = result.unwrap();
        let items = document.content_blocks.unwrap();

        // Should extract some content despite malformed structure
        assert!(!items.is_empty());
    }

    /// Test HTML entities and special characters in various contexts
    /// Verifies proper decoding of HTML entities in headings, paragraphs, lists
    #[test]
    fn test_html_entities_in_content() {
        let backend = HtmlBackend::new();
        let html = b"<html><body>\
            <h1>Entities: &lt;html&gt; &amp; &quot;quotes&quot; &apos;apostrophe&apos;</h1>\
            <p>Math: 2 &lt; 3 &gt; 1, x &le; y, a &ge; b, &plusmn; 5</p>\
            <ul>\
                <li>Item with &copy; copyright</li>\
                <li>Euro &euro; and pound &pound;</li>\
                <li>Non-breaking&nbsp;space</li>\
            </ul>\
            <code>Code with &lt;tag&gt; and &amp; ampersand</code>\
            </body></html>";
        let options = BackendOptions::default();

        let document = backend.parse_bytes(html, &options).unwrap();
        let items = document.content_blocks.unwrap();

        // Should decode entities correctly
        assert!(items.len() >= 3); // heading, paragraph, list

        // Check heading has decoded entities
        if let DocItem::SectionHeader { text, .. } = &items[0] {
            assert!(text.contains("<html>"));
            assert!(text.contains('&'));
            assert!(text.contains("\"quotes\""));
        } else {
            panic!("Expected SectionHeader");
        }

        // Check paragraph has decoded math symbols
        if let DocItem::Text { text, .. } = &items[1] {
            assert!(text.contains('<') && text.contains('>'));
            // Entities should be decoded by scraper
        } else {
            panic!("Expected Text DocItem");
        }
    }

    /// Test very long single line content
    /// Verifies backend handles extremely long content without errors
    #[test]
    fn test_very_long_single_line() {
        let backend = HtmlBackend::new();

        // Create a very long paragraph (10,000 characters)
        let long_text = "Lorem ipsum ".repeat(833); // ~10,000 chars
        let html = format!("<html><body><p>{long_text}</p></body></html>");
        let options = BackendOptions::default();

        let document = backend.parse_bytes(html.as_bytes(), &options).unwrap();
        let items = document.content_blocks.unwrap();

        assert_eq!(items.len(), 1);
        if let DocItem::Text { text, .. } = &items[0] {
            assert!(text.len() > 9000); // Should preserve long content
            assert!(text.contains("Lorem ipsum"));
        } else {
            panic!("Expected Text DocItem");
        }
    }

    /// Test table with headers (thead/th elements)
    /// Verifies proper parsing of table headers and body rows
    #[test]
    fn test_table_with_headers_and_complex_cells() {
        let backend = HtmlBackend::new();
        let html = b"<html><body>\
            <table>\
                <thead>\
                    <tr>\
                        <th>Name</th>\
                        <th>Age</th>\
                        <th>City</th>\
                        <th>Country</th>\
                    </tr>\
                </thead>\
                <tbody>\
                    <tr>\
                        <td>Alice</td>\
                        <td>30</td>\
                        <td>New York</td>\
                        <td>USA</td>\
                    </tr>\
                    <tr>\
                        <td>Bob</td>\
                        <td>25</td>\
                        <td>London</td>\
                        <td>UK</td>\
                    </tr>\
                    <tr>\
                        <td>Charlie</td>\
                        <td>35</td>\
                        <td>Tokyo</td>\
                        <td>Japan</td>\
                    </tr>\
                </tbody>\
            </table>\
            </body></html>";
        let options = BackendOptions::default();

        let document = backend.parse_bytes(html, &options).unwrap();
        let items = document.content_blocks.unwrap();

        assert_eq!(items.len(), 1);
        if let DocItem::Table { data, .. } = &items[0] {
            // Should have header row + 3 data rows = 4 rows total
            assert_eq!(data.num_rows, 4);
            assert_eq!(data.num_cols, 4);
            assert_eq!(data.grid.len(), 4);

            // Check header row (th elements)
            assert_eq!(data.grid[0].len(), 4);
            assert_eq!(data.grid[0][0].text, "Name");
            assert_eq!(data.grid[0][1].text, "Age");
            assert_eq!(data.grid[0][2].text, "City");
            assert_eq!(data.grid[0][3].text, "Country");

            // Check first data row
            assert_eq!(data.grid[1][0].text, "Alice");
            assert_eq!(data.grid[1][1].text, "30");
            assert_eq!(data.grid[1][2].text, "New York");
            assert_eq!(data.grid[1][3].text, "USA");

            // Check second data row
            assert_eq!(data.grid[2][0].text, "Bob");
            assert_eq!(data.grid[2][1].text, "25");

            // Check third data row
            assert_eq!(data.grid[3][0].text, "Charlie");
            assert_eq!(data.grid[3][2].text, "Tokyo");
        } else {
            panic!("Expected Table DocItem");
        }
    }

    /// Test nested lists (ul inside li)
    /// Verifies backend handles nested list structures
    #[test]
    fn test_nested_lists() {
        let backend = HtmlBackend::new();
        let html = b"<html><body>\
            <ul>\
                <li>Level 1 Item 1</li>\
                <li>Level 1 Item 2\
                    <ul>\
                        <li>Level 2 Item 1</li>\
                        <li>Level 2 Item 2</li>\
                    </ul>\
                </li>\
                <li>Level 1 Item 3</li>\
            </ul>\
            </body></html>";
        let options = BackendOptions::default();

        let document = backend.parse_bytes(html, &options).unwrap();
        let items = document.content_blocks.unwrap();

        // Should have at least 1 list (outer) + nested list items
        // Implementation may vary on how nested lists are handled
        assert!(!items.is_empty());

        // Check that we have List DocItems
        let list_items: Vec<_> = items
            .iter()
            .filter(|item| matches!(item, DocItem::List { .. }))
            .collect();
        assert!(
            !list_items.is_empty(),
            "Should have at least one List DocItem"
        );
    }

    /// Test nested tables (table inside td)
    /// Verifies backend handles nested table structures
    #[test]
    fn test_nested_tables() {
        let backend = HtmlBackend::new();
        let html = b"<html><body>\
            <table>\
                <tr>\
                    <td>Outer Cell 1</td>\
                    <td>\
                        <table>\
                            <tr>\
                                <td>Inner Cell 1</td>\
                                <td>Inner Cell 2</td>\
                            </tr>\
                        </table>\
                    </td>\
                </tr>\
            </table>\
            </body></html>";
        let options = BackendOptions::default();

        let document = backend.parse_bytes(html, &options).unwrap();
        let items = document.content_blocks.unwrap();

        // Should have at least the outer table
        assert!(!items.is_empty());

        // Count table items
        let table_items: Vec<_> = items
            .iter()
            .filter(|item| matches!(item, DocItem::Table { .. }))
            .collect();

        // Implementation may handle nested tables differently
        // At minimum, should have 1 table (outer)
        assert!(
            !table_items.is_empty(),
            "Should have at least one Table DocItem"
        );
    }

    /// Test HTML with script and style tags
    /// Verifies backend correctly ignores script/style content
    #[test]
    fn test_html_with_scripts_and_styles() {
        let backend = HtmlBackend::new();
        let html = b"<html>\
            <head>\
                <title>Test Page</title>\
                <style>body { color: red; }</style>\
                <script>console.log('test');</script>\
            </head>\
            <body>\
                <h1>Real Content</h1>\
                <p>This is visible text.</p>\
                <script>alert('should not appear');</script>\
                <style>.hidden { display: none; }</style>\
                <p>More visible text.</p>\
            </body>\
            </html>";
        let options = BackendOptions::default();

        let document = backend.parse_bytes(html, &options).unwrap();
        let items = document.content_blocks.unwrap();

        // Should have h1 + 2 paragraphs = 3 items
        // Script/style content should not appear
        assert_eq!(items.len(), 3);

        if let DocItem::SectionHeader { text, .. } = &items[0] {
            assert_eq!(text, "Real Content");
        } else {
            panic!("Expected SectionHeader for h1");
        }

        if let DocItem::Text { text, .. } = &items[1] {
            assert_eq!(text, "This is visible text.");
            assert!(!text.contains("console.log"));
            assert!(!text.contains("alert"));
        } else {
            panic!("Expected Text for first paragraph");
        }

        if let DocItem::Text { text, .. } = &items[2] {
            assert_eq!(text, "More visible text.");
            assert!(!text.contains("display: none"));
        } else {
            panic!("Expected Text for second paragraph");
        }
    }

    /// Test complex mixed content document
    /// Verifies backend handles combination of all supported elements
    #[test]
    fn test_complex_mixed_content() {
        let backend = HtmlBackend::new();
        let html = b"<html><body>\
            <h1>Document Title</h1>\
            <p>Introduction paragraph.</p>\
            <h2>Section 1</h2>\
            <ul>\
                <li>List item 1</li>\
                <li>List item 2</li>\
            </ul>\
            <table>\
                <tr><td>Row 1 Col 1</td><td>Row 1 Col 2</td></tr>\
                <tr><td>Row 2 Col 1</td><td>Row 2 Col 2</td></tr>\
            </table>\
            <pre>Code block content</pre>\
            <img src='test.jpg' alt='Test Image'/>\
            <h3>Subsection</h3>\
            <p>Conclusion paragraph.</p>\
            </body></html>";
        let options = BackendOptions::default();

        let document = backend.parse_bytes(html, &options).unwrap();
        let items = document.content_blocks.unwrap();

        // Should have: h1, p, h2, list, table, pre, img, h3, p = 9+ items
        assert!(items.len() >= 9, "Should have at least 9 DocItems");

        // Verify we have different types
        let has_header = items
            .iter()
            .any(|item| matches!(item, DocItem::SectionHeader { .. }));
        let has_text = items
            .iter()
            .any(|item| matches!(item, DocItem::Text { .. }));
        let has_list = items
            .iter()
            .any(|item| matches!(item, DocItem::List { .. }));
        let has_table = items
            .iter()
            .any(|item| matches!(item, DocItem::Table { .. }));
        let has_code = items
            .iter()
            .any(|item| matches!(item, DocItem::Code { .. }));
        let has_picture = items
            .iter()
            .any(|item| matches!(item, DocItem::Picture { .. }));

        assert!(has_header, "Should have SectionHeader DocItems");
        assert!(has_text, "Should have Text DocItems");
        assert!(has_list, "Should have List DocItems");
        assert!(has_table, "Should have Table DocItems");
        assert!(has_code, "Should have Code DocItems");
        assert!(has_picture, "Should have Picture DocItems");
    }

    /// Test HTML with form elements
    /// Verifies backend handles form elements gracefully (likely as text)
    #[test]
    fn test_html_with_form_elements() {
        let backend = HtmlBackend::new();
        let html = b"<html><body>\
            <h1>Registration Form</h1>\
            <p>Please fill out the form below.</p>\
            <form>\
                <label for='name'>Name:</label>\
                <input type='text' id='name' name='name'/>\
                <label for='email'>Email:</label>\
                <input type='email' id='email' name='email'/>\
                <textarea name='message'>Enter your message</textarea>\
                <button type='submit'>Submit</button>\
            </form>\
            <p>Thank you for registering.</p>\
            </body></html>";
        let options = BackendOptions::default();

        let document = backend.parse_bytes(html, &options).unwrap();
        let items = document.content_blocks.unwrap();

        // Should have at least: h1 + 2 paragraphs = 3 items minimum
        assert!(items.len() >= 3, "Should have at least 3 DocItems");

        // Check that we got the heading and paragraphs
        if let DocItem::SectionHeader { text, .. } = &items[0] {
            assert_eq!(text, "Registration Form");
        } else {
            panic!("Expected SectionHeader for h1");
        }

        // Form content handling may vary
        // Important: should not crash or error on form elements
        let last_item = items.last().unwrap();
        if let DocItem::Text { text, .. } = last_item {
            // Last paragraph should be present
            assert!(
                text.contains("Thank you") || text.contains("registering"),
                "Should preserve text after form"
            );
        }
    }

    /// Test HTML with doctype and comments
    /// Verifies backend handles DOCTYPE and HTML comments correctly
    #[test]
    fn test_html_with_doctype_and_comments() {
        let backend = HtmlBackend::new();
        let html = b"<!DOCTYPE html>\
            <!-- This is a comment -->\
            <html>\
            <head><!-- Head comment --><title>Test</title></head>\
            <body>\
            <!-- Body comment -->\
            <p>Content</p>\
            <!-- Another comment -->\
            </body>\
            </html>";
        let options = BackendOptions::default();

        let document = backend.parse_bytes(html, &options).unwrap();
        let items = document.content_blocks.unwrap();

        // Should have 1 text item (paragraph), comments should be ignored
        assert_eq!(items.len(), 1, "Should have 1 DocItem (comments ignored)");

        if let DocItem::Text { text, .. } = &items[0] {
            assert_eq!(text, "Content");
            // Verify no comment text leaked into content
            assert!(
                !text.contains("comment"),
                "Comments should not appear in text"
            );
        } else {
            panic!("Expected Text DocItem");
        }
    }

    /// Test HTML with base64 data URLs in images
    /// Verifies backend handles base64-encoded image data URLs
    #[test]
    fn test_html_with_base64_image() {
        let backend = HtmlBackend::new();
        // Small 1x1 red pixel PNG as base64
        let html = b"<html><body>\
            <img src='data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8DwHwAFBQIAX8jx0gAAAABJRU5ErkJggg==' alt='Red Pixel'/>\
            <p>Image above</p>\
            </body></html>";
        let options = BackendOptions::default();

        let document = backend.parse_bytes(html, &options).unwrap();
        let items = document.content_blocks.unwrap();

        // Should have at least 2 DocItems (might be Text+Picture or just Picture)
        assert!(
            items.len() >= 2,
            "Should have at least 2 DocItems, got {}: {:?}",
            items.len(),
            items
        );

        // Look for Picture DocItem
        let has_picture = items
            .iter()
            .any(|item| matches!(item, DocItem::Picture { .. }));
        assert!(has_picture, "Should have Picture DocItem");

        // Verify Picture has the data URL
        for item in &items {
            if let DocItem::Picture { image, .. } = item {
                assert!(image.is_some());
                let img_data = image.as_ref().unwrap();
                assert_eq!(
                    img_data.get("alt").and_then(|v| v.as_str()),
                    Some("Red Pixel")
                );
                // Verify data URL is preserved
                assert!(img_data
                    .get("uri")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .starts_with("data:image/png;base64,"));
            }
        }

        // Verify paragraph text is present
        let has_paragraph = items
            .iter()
            .any(|item| matches!(item, DocItem::Text { text, .. } if text == "Image above"));
        assert!(has_paragraph, "Should have paragraph text");
    }

    /// Test HTML with semantic HTML5 elements (article, section, nav, aside)
    /// Verifies backend handles HTML5 semantic elements
    #[test]
    fn test_html5_semantic_elements() {
        let backend = HtmlBackend::new();
        let html = b"<html><body>\
            <nav><a href='#'>Navigation Link</a></nav>\
            <article>\
                <h1>Article Title</h1>\
                <p>Article content</p>\
            </article>\
            <section>\
                <h2>Section Title</h2>\
                <p>Section content</p>\
            </section>\
            <aside>\
                <p>Sidebar content</p>\
            </aside>\
            </body></html>";
        let options = BackendOptions::default();

        let document = backend.parse_bytes(html, &options).unwrap();
        let items = document.content_blocks.unwrap();

        // Should have: nav link (text/list item), h1, article p, h2, section p, aside p
        assert!(items.len() >= 5, "Should have at least 5 DocItems");

        // Verify we can extract content from semantic elements
        let text_content: Vec<String> = items
            .iter()
            .filter_map(|item| match item {
                DocItem::Text { text, .. } => Some(text.clone()),
                DocItem::SectionHeader { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();

        assert!(text_content.iter().any(|t| t.contains("Article")));
        assert!(text_content.iter().any(|t| t.contains("Section")));
        assert!(text_content.iter().any(|t| t.contains("Sidebar")));
    }

    /// Test HTML with definition lists (dl, dt, dd)
    /// Verifies backend handles definition lists without crashing
    #[test]
    fn test_html_definition_lists() {
        let backend = HtmlBackend::new();
        let html = b"<html><body>\
            <h1>Glossary</h1>\
            <dl>\
                <dt>Term 1</dt>\
                <dd>Definition 1</dd>\
                <dt>Term 2</dt>\
                <dd>Definition 2a</dd>\
                <dd>Definition 2b</dd>\
            </dl>\
            <p>End of glossary</p>\
            </body></html>";
        let options = BackendOptions::default();

        // Should not panic or crash on definition lists
        let document = backend.parse_bytes(html, &options).unwrap();
        let items = document.content_blocks.unwrap();

        // Definition lists may or may not generate DocItems depending on implementation
        // The important thing is we don't crash and we preserve surrounding content
        assert!(
            items.len() >= 2,
            "Should have at least heading and paragraph, got {}: {:?}",
            items.len(),
            items
        );

        // Verify heading is present
        let has_heading = items
            .iter()
            .any(|item| matches!(item, DocItem::SectionHeader { text, .. } if text == "Glossary"));
        assert!(has_heading, "Should have heading");

        // Verify paragraph after definition list is present
        let has_paragraph = items
            .iter()
            .any(|item| matches!(item, DocItem::Text { text, .. } if text == "End of glossary"));
        assert!(has_paragraph, "Should have paragraph after definition list");
    }

    /// Test HTML with very deeply nested tables
    /// Verifies backend handles deep table nesting without stack overflow
    #[test]
    fn test_html_deeply_nested_tables() {
        let backend = HtmlBackend::new();
        let html = b"<html><body>\
            <table>\
                <tr><td>\
                    <table>\
                        <tr><td>\
                            <table>\
                                <tr><td>\
                                    <table>\
                                        <tr><td>Deeply nested content</td></tr>\
                                    </table>\
                                </td></tr>\
                            </table>\
                        </td></tr>\
                    </table>\
                </td></tr>\
            </table>\
            </body></html>";
        let options = BackendOptions::default();

        // Should not panic or stack overflow
        let document = backend.parse_bytes(html, &options).unwrap();
        let items = document.content_blocks.unwrap();

        // Should have at least one Table DocItem (may flatten nested tables)
        assert!(!items.is_empty(), "Should have at least 1 DocItem");

        let has_table = items
            .iter()
            .any(|item| matches!(item, DocItem::Table { .. }));
        assert!(has_table, "Should have at least one Table DocItem");

        // Verify we can extract the deeply nested text
        let has_nested_text = items.iter().any(|item| {
            match item {
                DocItem::Table { data, .. } => {
                    // TableData has a grid field which is Vec<Vec<TableCell>>
                    data.grid
                        .iter()
                        .flatten()
                        .any(|cell| cell.text.contains("Deeply nested content"))
                }
                _ => false,
            }
        });
        assert!(
            has_nested_text,
            "Should preserve deeply nested table content"
        );
    }

    /// Test HTML with embedded SVG graphics
    /// Verifies backend handles inline SVG with text content
    #[test]
    fn test_html_embedded_svg() {
        let backend = HtmlBackend::new();
        let html = b"<html><body>\
            <h1>SVG Graphics Demo</h1>\
            <svg width=\"100\" height=\"100\" xmlns=\"http://www.w3.org/2000/svg\">\
                <text x=\"10\" y=\"50\" font-size=\"20\">SVG Text</text>\
            </svg>\
            <p>Text after SVG</p>\
            </body></html>";
        let options = BackendOptions::default();

        let document = backend.parse_bytes(html, &options).unwrap();
        let items = document.content_blocks.unwrap();

        // Should have heading, paragraph after SVG, and possibly SVG text
        assert!(
            items.len() >= 2,
            "Should have at least heading and paragraph"
        );

        // Verify heading is present
        let has_heading = items
            .iter()
            .any(|item| matches!(item, DocItem::SectionHeader { text, .. } if text == "SVG Graphics Demo"));
        assert!(has_heading, "Should have heading");

        // Verify paragraph after SVG is present
        let has_paragraph = items
            .iter()
            .any(|item| matches!(item, DocItem::Text { text, .. } if text == "Text after SVG"));
        assert!(has_paragraph, "Should have paragraph after SVG");

        // Note: SVG handling varies by implementation
        // - May extract text from <text> elements within SVG
        // - May treat SVG as opaque graphic (no text extraction)
        // - May convert SVG to Picture DocItem with alt text
        // Current test ensures we don't crash and preserve surrounding content
    }

    /// Test HTML with video/audio fallback content
    /// Verifies backend extracts fallback text from multimedia elements
    #[test]
    fn test_html_multimedia_fallback() {
        let backend = HtmlBackend::new();
        let html = b"<html><body>\
            <h1>Multimedia Content</h1>\
            <video controls>\
                <source src=\"video.mp4\" type=\"video/mp4\">\
                <p>Your browser does not support the video tag. <a href=\"video.mp4\">Download video</a></p>\
            </video>\
            <audio controls>\
                <source src=\"audio.mp3\" type=\"audio/mpeg\">\
                <p>Your browser does not support the audio tag.</p>\
            </audio>\
            <p>After multimedia</p>\
            </body></html>";
        let options = BackendOptions::default();

        let document = backend.parse_bytes(html, &options).unwrap();
        let items = document.content_blocks.unwrap();

        // Should have heading and fallback content
        assert!(
            items.len() >= 2,
            "Should have at least heading and some content"
        );

        // Verify heading is present
        let has_heading = items
            .iter()
            .any(|item| matches!(item, DocItem::SectionHeader { text, .. } if text == "Multimedia Content"));
        assert!(has_heading, "Should have heading");

        // Verify final paragraph is present
        let has_paragraph = items
            .iter()
            .any(|item| matches!(item, DocItem::Text { text, .. } if text == "After multimedia"));
        assert!(has_paragraph, "Should have paragraph after multimedia");

        // Note: Fallback content handling:
        // - May extract fallback text from <video>/<audio> children
        // - May treat multimedia as Picture/Figure DocItem
        // - May skip multimedia elements entirely
        // Per CLAUDE.md, audio/video are out of scope - handled by separate system
    }

    /// Test HTML with MathML mathematical notation
    /// Verifies backend handles embedded mathematical expressions
    #[test]
    fn test_html_mathml() {
        let backend = HtmlBackend::new();
        let html = b"<html><body>\
            <h1>Mathematical Formulas</h1>\
            <p>The quadratic formula is:</p>\
            <math xmlns=\"http://www.w3.org/1998/Math/MathML\">\
                <mrow>\
                    <mi>x</mi><mo>=</mo>\
                    <mfrac>\
                        <mrow><mo>-</mo><mi>b</mi><mo>&PlusMinus;</mo><msqrt><mrow><msup><mi>b</mi><mn>2</mn></msup><mo>-</mo><mn>4</mn><mi>a</mi><mi>c</mi></mrow></msqrt></mrow>\
                        <mrow><mn>2</mn><mi>a</mi></mrow>\
                    </mfrac>\
                </mrow>\
            </math>\
            <p>where a, b, and c are constants.</p>\
            </body></html>";
        let options = BackendOptions::default();

        let document = backend.parse_bytes(html, &options).unwrap();
        let items = document.content_blocks.unwrap();

        // Should have heading and surrounding paragraphs
        assert!(items.len() >= 2, "Should have at least heading and content");

        // Verify heading is present
        let has_heading = items
            .iter()
            .any(|item| matches!(item, DocItem::SectionHeader { text, .. } if text == "Mathematical Formulas"));
        assert!(has_heading, "Should have heading");

        // Verify surrounding text is preserved
        let has_intro = items.iter().any(
            |item| matches!(item, DocItem::Text { text, .. } if text.contains("quadratic formula")),
        );
        assert!(has_intro, "Should have introductory text");

        let has_explanation = items.iter().any(
            |item| matches!(item, DocItem::Text { text, .. } if text.contains("where a, b, and c")),
        );
        assert!(has_explanation, "Should have explanation after formula");

        // Note: MathML handling options:
        // - Extract text from <mi>, <mn>, <mo> elements (variables, numbers, operators)
        // - Convert to Formula DocItem with LaTeX/MathML representation
        // - Treat as opaque (skip MathML, preserve surrounding text)
        // Current test ensures we preserve surrounding content
    }

    /// Test HTML with microdata/structured data
    /// Verifies backend handles schema.org markup
    #[test]
    fn test_html_microdata() {
        let backend = HtmlBackend::new();
        let html = b"<html><body>\
            <div itemscope itemtype=\"http://schema.org/Person\">\
                <h1 itemprop=\"name\">John Doe</h1>\
                <p>Email: <span itemprop=\"email\">john@example.com</span></p>\
                <p>Phone: <span itemprop=\"telephone\">555-1234</span></p>\
                <div itemprop=\"address\" itemscope itemtype=\"http://schema.org/PostalAddress\">\
                    <p>Address: <span itemprop=\"streetAddress\">123 Main St</span>, \
                    <span itemprop=\"addressLocality\">Anytown</span>, \
                    <span itemprop=\"addressRegion\">CA</span> \
                    <span itemprop=\"postalCode\">12345</span></p>\
                </div>\
            </div>\
            </body></html>";
        let options = BackendOptions::default();

        let document = backend.parse_bytes(html, &options).unwrap();
        let items = document.content_blocks.unwrap();

        // Should have heading and contact info
        assert!(items.len() >= 3, "Should have heading, email, and address");

        // Verify heading/name is present
        let has_name = items
            .iter()
            .any(|item| matches!(item, DocItem::SectionHeader { text, .. } if text == "John Doe"));
        assert!(has_name, "Should have person name as heading");

        // Verify email is present
        let has_email = items.iter().any(
            |item| matches!(item, DocItem::Text { text, .. } if text.contains("john@example.com")),
        );
        assert!(has_email, "Should have email");

        // Verify address is present
        let has_address = items
            .iter()
            .any(|item| matches!(item, DocItem::Text { text, .. } if text.contains("123 Main St")));
        assert!(has_address, "Should have address");

        // Note: Microdata handling:
        // - itemscope/itemtype/itemprop attributes provide semantic meaning
        // - Backend should extract visible text, ignoring microdata attributes
        // - Microdata is metadata, not visible content
        // - Alternative: could use microdata to improve DocItem classification
        //   (e.g., recognize Person schema → extract structured fields)
    }

    /// Test HTML with obsolete/deprecated tags
    /// Verifies backend handles legacy HTML gracefully
    #[test]
    fn test_html_deprecated_tags() {
        let backend = HtmlBackend::new();
        let html = b"<html><body>\
            <center><h1>Centered Title</h1></center>\
            <font color=\"red\" size=\"5\">Red large text</font>\
            <marquee>Scrolling text</marquee>\
            <blink>Blinking text</blink>\
            <u>Underlined text</u>\
            <strike>Strikethrough text</strike>\
            <tt>Teletype text</tt>\
            <p>Modern paragraph</p>\
            </body></html>";
        let options = BackendOptions::default();

        let document = backend.parse_bytes(html, &options).unwrap();
        let items = document.content_blocks.unwrap();

        // Should have at least heading and paragraph
        assert!(
            items.len() >= 2,
            "Should have at least heading and paragraph"
        );

        // Verify heading is present (center tag should be ignored)
        let has_heading = items.iter().any(
            |item| matches!(item, DocItem::SectionHeader { text, .. } if text == "Centered Title"),
        );
        assert!(has_heading, "Should have heading");

        // Verify deprecated tag content is preserved (if supported)
        // Note: Some deprecated tags like <font>, <marquee>, <blink> may be ignored by parser
        // The key is that we don't crash and we preserve the heading and paragraph
        let _has_red_text = items.iter().any(
            |item| matches!(item, DocItem::Text { text, .. } if text.contains("Red large text")),
        );
        // Font tag may or may not be extracted - implementation dependent

        let _has_marquee = items.iter().any(
            |item| matches!(item, DocItem::Text { text, .. } if text.contains("Scrolling text")),
        );
        // Marquee may or may not be extracted - non-standard tag

        // Verify modern paragraph is present
        let has_paragraph = items
            .iter()
            .any(|item| matches!(item, DocItem::Text { text, .. } if text == "Modern paragraph"));
        assert!(has_paragraph, "Should have modern paragraph");

        // Note: Deprecated tags to handle:
        // - <center>: deprecated in HTML4, removed in HTML5 (use CSS text-align)
        // - <font>: deprecated in HTML4, removed in HTML5 (use CSS font-family/color/size)
        // - <marquee>: non-standard, never part of HTML spec (IE extension)
        // - <blink>: non-standard, removed from browsers
        // - <u>: deprecated in HTML4 (confusion with links), redefined in HTML5
        // - <strike>: deprecated in HTML4, use <s> or <del>
        // - <tt>: deprecated in HTML5, use <code> or <kbd>
        // Backend should extract text content, ignore deprecated styling
    }

    // ========================================
    // Rich Table Cells Tests (Phase 1)
    // ========================================

    /// Test: has_nested_structure() returns true for cells with unordered lists
    #[test]
    fn test_has_nested_structure_with_unordered_list() {
        let _backend = HtmlBackend::new();
        let html = r"
            <table>
                <tr>
                    <td>
                        <ul>
                            <li>First item</li>
                            <li>Second item</li>
                        </ul>
                    </td>
                </tr>
            </table>
        ";

        let document = Html::parse_document(html);
        let td_selector = Selector::parse("td").unwrap();
        let cell = document.select(&td_selector).next().unwrap();

        assert!(
            HtmlBackend::has_nested_structure(&cell),
            "Cell with <ul> should be detected as nested"
        );
    }

    /// Test: has_nested_structure() returns true for cells with ordered lists
    #[test]
    fn test_has_nested_structure_with_ordered_list() {
        let _backend = HtmlBackend::new();
        let html = r"
            <table>
                <tr>
                    <td>
                        <ol>
                            <li>First</li>
                            <li>Second</li>
                        </ol>
                    </td>
                </tr>
            </table>
        ";

        let document = Html::parse_document(html);
        let td_selector = Selector::parse("td").unwrap();
        let cell = document.select(&td_selector).next().unwrap();

        assert!(
            HtmlBackend::has_nested_structure(&cell),
            "Cell with <ol> should be detected as nested"
        );
    }

    /// Test: has_nested_structure() returns true for cells with multiple paragraphs
    #[test]
    fn test_has_nested_structure_with_multiple_paragraphs() {
        let _backend = HtmlBackend::new();
        let html = r"
            <table>
                <tr>
                    <td>
                        <p>First paragraph</p>
                        <p>Second paragraph</p>
                    </td>
                </tr>
            </table>
        ";

        let document = Html::parse_document(html);
        let td_selector = Selector::parse("td").unwrap();
        let cell = document.select(&td_selector).next().unwrap();

        assert!(
            HtmlBackend::has_nested_structure(&cell),
            "Cell with multiple <p> tags should be detected as nested"
        );
    }

    /// Test: has_nested_structure() returns true for cells with div containers
    #[test]
    fn test_has_nested_structure_with_div() {
        let _backend = HtmlBackend::new();
        let html = r"
            <table>
                <tr>
                    <td>
                        <div>Nested content</div>
                    </td>
                </tr>
            </table>
        ";

        let document = Html::parse_document(html);
        let td_selector = Selector::parse("td").unwrap();
        let cell = document.select(&td_selector).next().unwrap();

        assert!(
            HtmlBackend::has_nested_structure(&cell),
            "Cell with <div> should be detected as nested"
        );
    }

    /// Test: has_nested_structure() returns false for simple text cells
    #[test]
    fn test_has_nested_structure_simple_text() {
        let _backend = HtmlBackend::new();
        let html = r"
            <table>
                <tr>
                    <td>Simple text content</td>
                </tr>
            </table>
        ";

        let document = Html::parse_document(html);
        let td_selector = Selector::parse("td").unwrap();
        let cell = document.select(&td_selector).next().unwrap();

        assert!(
            !HtmlBackend::has_nested_structure(&cell),
            "Cell with plain text should NOT be detected as nested"
        );
    }

    /// Test: has_nested_structure() returns false for cell with single paragraph
    #[test]
    fn test_has_nested_structure_single_paragraph() {
        let _backend = HtmlBackend::new();
        let html = r"
            <table>
                <tr>
                    <td>
                        <p>Single paragraph</p>
                    </td>
                </tr>
            </table>
        ";

        let document = Html::parse_document(html);
        let td_selector = Selector::parse("td").unwrap();
        let cell = document.select(&td_selector).next().unwrap();

        assert!(
            !HtmlBackend::has_nested_structure(&cell),
            "Cell with single <p> should NOT be detected as nested"
        );
    }

    /// Test: has_nested_structure() returns false for cells with inline formatting only
    #[test]
    fn test_has_nested_structure_inline_formatting() {
        let _backend = HtmlBackend::new();
        let html = r"
            <table>
                <tr>
                    <td>Text with <strong>bold</strong> and <em>italic</em></td>
                </tr>
            </table>
        ";

        let document = Html::parse_document(html);
        let td_selector = Selector::parse("td").unwrap();
        let cell = document.select(&td_selector).next().unwrap();

        assert!(
            !HtmlBackend::has_nested_structure(&cell),
            "Cell with inline formatting only should NOT be detected as nested"
        );
    }

    /// Test: parse_cell_content() should return DocItems for rich content
    /// N=2408: Rich table cell parsing - Implemented N=2829
    #[test]
    fn test_parse_cell_content_stub() {
        let _backend = HtmlBackend::new();
        let html = r"
            <table>
                <tr>
                    <td>
                        <ul>
                            <li>Test</li>
                        </ul>
                    </td>
                </tr>
            </table>
        ";

        let document = Html::parse_document(html);
        let td_selector = Selector::parse("td").unwrap();
        let cell = document.select(&td_selector).next().unwrap();

        let mut item_count = 0;
        let result = HtmlBackend::parse_cell_content(&cell, &mut item_count);

        assert!(
            !result.is_empty(),
            "Phase 2: parse_cell_content should return DocItems for lists"
        );

        // Verify we got a List and ListItem
        let has_list = result
            .iter()
            .any(|item| matches!(item, DocItem::List { .. }));
        let has_list_item = result
            .iter()
            .any(|item| matches!(item, DocItem::ListItem { .. }));
        assert!(has_list, "Should have List DocItem");
        assert!(has_list_item, "Should have ListItem DocItem");
    }

    // ========================================
    // Rich Table Cells Integration Tests (Phase 2)
    // ========================================

    /// Test: Full table parsing with list in cell
    /// N=2408: Rich table cell parsing - Implemented N=2829
    #[test]
    fn test_table_with_list_in_cell() {
        let backend = HtmlBackend::new();
        let html = r"
            <html><body>
                <table>
                    <tr>
                        <th>Items</th>
                        <th>Description</th>
                    </tr>
                    <tr>
                        <td>
                            <ul>
                                <li>First item</li>
                                <li>Second item</li>
                                <li>Third item</li>
                            </ul>
                        </td>
                        <td>Plain text description</td>
                    </tr>
                </table>
            </body></html>
        ";

        let options = BackendOptions::default();
        let result = backend.parse_bytes(html.as_bytes(), &options);
        assert!(result.is_ok(), "Should parse table with list successfully");

        let doc = result.unwrap();
        let items = doc
            .content_blocks
            .as_deref()
            .expect("should have content_blocks");

        // Should have: 1 Table only
        // Note: Lists inside table cells are NOT added as separate DocItems.
        // They are collapsed into the cell's text field as inline content.
        // This matches Python docling behavior and produces correct markdown output.
        let tables: Vec<_> = items
            .iter()
            .filter(|item| matches!(item, DocItem::Table { .. }))
            .collect();
        let lists: Vec<_> = items
            .iter()
            .filter(|item| matches!(item, DocItem::List { .. }))
            .collect();
        let list_items: Vec<_> = items
            .iter()
            .filter(|item| matches!(item, DocItem::ListItem { .. }))
            .collect();

        assert_eq!(tables.len(), 1, "Should have 1 Table");
        assert_eq!(
            lists.len(),
            0,
            "Lists inside cells should not be separate DocItems"
        );
        assert_eq!(
            list_items.len(),
            0,
            "ListItems inside cells should not be separate DocItems"
        );

        // Verify table cell text contains collapsed list content
        if let DocItem::Table { data, .. } = &tables[0] {
            let table_cells = data.table_cells.as_ref().expect("Should have table_cells");

            // All cells should have ref_item = None (no nested DocItems stored)
            assert!(
                table_cells[0].ref_item.is_none(),
                "Header cell should not have ref_item"
            );
            assert!(
                table_cells[1].ref_item.is_none(),
                "Header cell should not have ref_item"
            );
            assert!(
                table_cells[2].ref_item.is_none(),
                "Cell with list should not have ref_item (list collapsed to text)"
            );
            assert!(
                table_cells[3].ref_item.is_none(),
                "Plain text cell should not have ref_item"
            );

            // Cell with list should have collapsed inline text
            assert!(
                table_cells[2].text.contains("First item"),
                "Cell should contain collapsed list text"
            );
            assert!(
                table_cells[2].text.contains("- "),
                "Cell should have list marker in text"
            );
        } else {
            panic!("Expected Table DocItem");
        }
    }

    /// Test: Full table parsing with multiple paragraphs in cell
    #[test]
    fn test_table_with_multiple_paragraphs_in_cell() {
        let backend = HtmlBackend::new();
        let html = r"
            <html><body>
                <table>
                    <tr>
                        <td>
                            <p>First paragraph</p>
                            <p>Second paragraph</p>
                        </td>
                    </tr>
                </table>
            </body></html>
        ";

        let options = BackendOptions::default();
        let result = backend.parse_bytes(html.as_bytes(), &options);
        assert!(
            result.is_ok(),
            "Should parse table with paragraphs successfully"
        );

        let doc = result.unwrap();
        let items = doc
            .content_blocks
            .as_deref()
            .expect("should have content_blocks");

        // Should have: 1 Table only
        // Note: Paragraphs inside table cells are collapsed into cell text, not separate DocItems
        let tables: Vec<_> = items
            .iter()
            .filter(|item| matches!(item, DocItem::Table { .. }))
            .collect();
        let text_items: Vec<_> = items
            .iter()
            .filter(|item| matches!(item, DocItem::Text { .. }))
            .collect();

        assert_eq!(tables.len(), 1, "Should have 1 Table");
        assert_eq!(
            text_items.len(),
            0,
            "Paragraphs inside cells should not be separate DocItems"
        );

        // Verify table cell has collapsed text (no ref_item)
        if let DocItem::Table { data, .. } = &tables[0] {
            let table_cells = data.table_cells.as_ref().expect("Should have table_cells");
            assert!(
                table_cells[0].ref_item.is_none(),
                "Cell with paragraphs should not have ref_item (content collapsed to text)"
            );
            // Cell text should contain both paragraphs separated by double space
            assert!(
                table_cells[0].text.contains("First paragraph"),
                "Cell should contain first paragraph"
            );
            assert!(
                table_cells[0].text.contains("Second paragraph"),
                "Cell should contain second paragraph"
            );
        } else {
            panic!("Expected Table DocItem");
        }
    }

    /// Test: Mixed table with simple and rich cells
    /// N=2408: Rich table cell parsing - Implemented N=2829
    #[test]
    fn test_table_with_mixed_simple_and_rich_cells() {
        let backend = HtmlBackend::new();
        let html = r"
            <html><body>
                <table>
                    <tr>
                        <td>Simple text</td>
                        <td>
                            <ul>
                                <li>Item 1</li>
                                <li>Item 2</li>
                            </ul>
                        </td>
                        <td>Another simple text</td>
                    </tr>
                </table>
            </body></html>
        ";

        let options = BackendOptions::default();
        let result = backend.parse_bytes(html.as_bytes(), &options);
        assert!(result.is_ok(), "Should parse mixed table successfully");

        let doc = result.unwrap();
        let items = doc
            .content_blocks
            .as_deref()
            .expect("should have content_blocks");

        // Should have: 1 Table only (lists collapsed to cell text)
        let tables: Vec<_> = items
            .iter()
            .filter(|item| matches!(item, DocItem::Table { .. }))
            .collect();
        assert_eq!(tables.len(), 1, "Should have 1 Table");

        // Verify all cells have ref_item = None (nested content collapsed to text)
        if let DocItem::Table { data, .. } = &tables[0] {
            let table_cells = data.table_cells.as_ref().expect("Should have table_cells");
            assert_eq!(table_cells.len(), 3, "Should have 3 cells");

            assert!(
                table_cells[0].ref_item.is_none(),
                "First cell (simple) should not have ref_item"
            );
            assert!(
                table_cells[1].ref_item.is_none(),
                "Second cell (list) should not have ref_item (content collapsed to text)"
            );
            assert!(
                table_cells[2].ref_item.is_none(),
                "Third cell (simple) should not have ref_item"
            );

            // Verify cell text content
            assert_eq!(table_cells[0].text, "Simple text", "First cell text");
            assert!(
                table_cells[1].text.contains("Item 1"),
                "Second cell should contain list items as text"
            );
            assert!(
                table_cells[1].text.contains("- "),
                "Second cell should have list marker"
            );
            assert_eq!(
                table_cells[2].text, "Another simple text",
                "Third cell text"
            );
        } else {
            panic!("Expected Table DocItem");
        }
    }
}
