//! Pure Rust LaTeX format support
//!
//! Strategy:
//! 1. Regex-based parsing for common LaTeX constructs
//! 2. Extract document structure: title, sections, paragraphs, lists, tables
//! 3. Generate `DocItems` directly (no Python bridge, no Pandoc, no external deps)
//! 4. Handle common LaTeX commands and formatting
//!
//! Note: This is a pragmatic solution. LaTeX is Turing-complete and full parsing
//! would require a complete TeX engine. This handles common academic documents.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use docling_core::{
    content::{CoordOrigin, DocItem, ProvenanceItem},
    Document, DocumentMetadata, InputFormat,
};
use regex::Regex;
use std::path::Path;
use std::sync::LazyLock;

// =============================================================================
// Pre-compiled regex patterns using std::sync::LazyLock (Rust 1.80+)
// Replaces lazy_static! macro with standard library equivalent
// =============================================================================

// -- Text cleaning patterns --
static RE_COMMENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)%.*$").expect("valid comment regex"));
static RE_DOCUMENT_CMD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\\(?:documentclass|usepackage)(?:\[[^\]]*\])?\{[^}]*\}").expect("valid cmd regex")
});
static RE_BOLD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\\textbf\{([^}]+)\}").expect("valid bold regex"));
static RE_ITALIC: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\\(?:textit|emph)\{([^}]+)\}").expect("valid italic regex"));
static RE_CODE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\\texttt\{([^}]+)\}").expect("valid code regex"));
static RE_UNDERLINE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\\underline\{([^}]+)\}").expect("valid underline regex"));
static RE_CITE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\\(?:cite|ref|label)\{[^}]+\}").expect("valid cite regex"));
static RE_SIMPLE_CMD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\\[a-zA-Z]+\*?\s*").expect("valid simple cmd regex"));
static RE_WHITESPACE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[ \t]+").expect("valid whitespace regex"));

// -- Section patterns --
static RE_SECTION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\\section\{([^}]+)\}").expect("valid section regex"));
static RE_SUBSECTION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\\subsection\{([^}]+)\}").expect("valid subsection regex"));
static RE_SUBSUBSECTION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\\subsubsection\{([^}]+)\}").expect("valid subsubsection regex"));
static RE_PARAGRAPH: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\\paragraph\{([^}]+)\}").expect("valid paragraph regex"));

// -- Metadata patterns --
static RE_TITLE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\\title\{([^}]+)\}").expect("valid title regex"));
static RE_AUTHOR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\\author\{([^}]+)\}").expect("valid author regex"));
static RE_DATE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\\date\{([^}]+)\}").expect("valid date regex"));

// -- Resume patterns --
static RE_RESUME_SUBHEADING: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\\resumeSubheading\s*\{").expect("valid subheading regex"));
static RE_RESUME_PROJECT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\\resumeProjectHeading\s*\{").expect("valid project regex"));

// -- List patterns --
static RE_ITEMIZE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)\\begin\{itemize\}(.*?)\\end\{itemize\}").expect("valid itemize regex")
});
static RE_ENUMERATE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)\\begin\{enumerate\}(.*?)\\end\{enumerate\}").expect("valid enumerate regex")
});
static RE_RESUME_LIST: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)\\resumeItemListStart(.*?)\\resumeItemListEnd")
        .expect("valid resume list regex")
});
static RE_RESUME_ITEM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\\resumeItem\{([^}]+)\}").expect("valid resume item regex"));

// -- Table patterns --
static RE_TABULAR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)\\begin\{tabular\*?\}(?:\{[^}]*\})?\{[^}]*\}(.*?)\\end\{tabular\*?\}")
        .expect("valid tabular regex")
});

// -- Center block patterns --
static RE_CENTER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)\\begin\{center\}(.*?)\\end\{center\}").expect("valid center regex")
});

// -- Body text extraction patterns --
static RE_REMOVE_ENV: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?s)\\begin\{(?:figure|table|equation|align|itemize|enumerate|tabular\*?|center)\}.*?\\end\{(?:figure|table|equation|align|itemize|enumerate|tabular\*?|center)\}"
    ).expect("valid env regex")
});
static RE_SECTION_CMD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)\\(?:section|subsection|subsubsection|paragraph)\{[^}]+\}")
        .expect("valid section cmd regex")
});
// -- Paragraph extraction patterns --
static RE_RESUME_LIST_PARA: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)\\resumeItemListStart.*?\\resumeItemListEnd")
        .expect("valid resume list para regex")
});
static RE_RESUME_SUBHEADING_LIST: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)\\resumeSubHeadingListStart.*?\\resumeSubHeadingListEnd")
        .expect("valid resume subheading list regex")
});
static RE_RESUME_SUBHEADING_PARA: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)\\resumeSubheading\s*\{[^}]+\}\s*\{[^}]+\}\s*\{[^}]+\}\s*\{[^}]+\}")
        .expect("valid resume subheading para regex")
});
static RE_RESUME_PROJECT_PARA: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)\\resumeProjectHeading\s*\{[^}]+\}\s*\{[^}]+\}")
        .expect("valid resume project para regex")
});
static RE_DOC_BEGIN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s).*?\\begin\{document\}").expect("valid doc begin regex"));
static RE_DOC_END: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)\\end\{document\}.*").expect("valid doc end regex"));

/// Backend for LaTeX files
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct LatexBackend;

#[allow(clippy::trivially_copy_pass_by_ref)] // Unit struct methods conventionally take &self
impl LatexBackend {
    /// Create a new LaTeX backend
    ///
    /// # Errors
    ///
    /// This function currently never fails and is infallible.
    /// The `Result` return type is for API consistency.
    #[inline]
    #[must_use = "this returns a Result that should be handled"]
    pub const fn new() -> Result<Self> {
        Ok(Self)
    }

    /// Clean LaTeX text by removing commands and normalizing whitespace
    fn clean_latex_text(text: &str) -> String {
        let mut result = text.to_string();

        // Remove comments (% to end of line)
        result = RE_COMMENT.replace_all(&result, "").to_string();

        // Remove document class and packages
        result = RE_DOCUMENT_CMD.replace_all(&result, "").to_string();

        // Remove document environment markers
        result = result.replace(r"\begin{document}", "");
        result = result.replace(r"\end{document}", "");
        result = result.replace(r"\maketitle", "");

        // Handle line breaks (\\) - convert to actual newlines BEFORE other processing
        // This preserves structure in multi-line list items
        result = result.replace(r"\\", "\n");

        // Handle math mode separators: $|$ → | (common in resumes for contact info)
        result = result.replace("$|$", " | ");

        // Handle formatting commands - convert to markdown equivalents
        result = RE_BOLD.replace_all(&result, "**$1**").to_string();
        result = RE_ITALIC.replace_all(&result, "*$1*").to_string();
        result = RE_CODE.replace_all(&result, "`$1`").to_string();

        // Underline doesn't have markdown equivalent, just preserve text
        result = RE_UNDERLINE.replace_all(&result, "$1").to_string();

        // Handle citations and references
        result = RE_CITE.replace_all(&result, "").to_string();

        // Remove remaining simple commands (no arguments)
        result = RE_SIMPLE_CMD.replace_all(&result, " ").to_string();

        // Remove curly braces
        result = result.replace(['{', '}'], "");

        // Normalize whitespace BUT preserve newlines
        // Replace multiple spaces with single space, but keep newlines
        result = RE_WHITESPACE.replace_all(&result, " ").to_string();

        result.trim().to_string()
    }

    /// Extract sections from LaTeX document
    // Method signature kept for API consistency with other LaTeXBackend methods
    #[allow(clippy::unused_self)]
    fn extract_sections(&self, source: &str) -> Vec<(String, String, usize)> {
        let mut sections = Vec::new();

        // Process each section type with pre-compiled regex
        let section_patterns: &[(&Regex, usize)] = &[
            (&RE_SECTION, 1),
            (&RE_SUBSECTION, 2),
            (&RE_SUBSUBSECTION, 3),
            (&RE_PARAGRAPH, 4),
        ];

        for (re, level) in section_patterns {
            for cap in re.captures_iter(source) {
                if let Some(title_match) = cap.get(1) {
                    let title = Self::clean_latex_text(title_match.as_str());
                    // Use the START of \section command, not just the title content
                    if let Some(full_match) = cap.get(0) {
                        let line_no = source[..full_match.start()].matches('\n').count() + 1;
                        sections.push((title, "#".repeat(*level), line_no));
                    }
                }
            }
        }

        // Sort by line number
        sections.sort_by_key(|(_, _, line)| *line);
        sections
    }

    /// Extract title, author, date from LaTeX preamble
    // Method signature kept for API consistency with other LaTeXBackend methods
    #[allow(clippy::unused_self)]
    fn extract_metadata(&self, source: &str) -> (Option<String>, Option<String>, Option<String>) {
        let title = RE_TITLE
            .captures(source)
            .and_then(|cap| cap.get(1))
            .map(|m| Self::clean_latex_text(m.as_str()));

        let author = RE_AUTHOR
            .captures(source)
            .and_then(|cap| cap.get(1))
            .map(|m| Self::clean_latex_text(m.as_str()));

        let date = RE_DATE
            .captures(source)
            .and_then(|cap| cap.get(1))
            .map(|m| Self::clean_latex_text(m.as_str()));

        (title, author, date)
    }

    /// Parse LaTeX date string into `DateTime<Utc>`
    /// Supports multiple formats commonly used in LaTeX:
    /// - ISO 8601 full: 2025-01-15T10:30:00Z
    /// - Date only: 2025-01-15
    /// - Year only: 2025
    /// - Month and year: January 2025, Jan 2025
    fn parse_date(date_str: &str) -> Option<DateTime<Utc>> {
        // Try full ISO 8601 format first
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(date_str) {
            return Some(dt.with_timezone(&Utc));
        }

        // Try date-only format (YYYY-MM-DD)
        if let Ok(naive_date) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
            let naive_datetime = naive_date.and_hms_opt(0, 0, 0)?;
            return Some(chrono::DateTime::from_naive_utc_and_offset(
                naive_datetime,
                Utc,
            ));
        }

        // Try date format with slashes (YYYY/MM/DD)
        if let Ok(naive_date) = chrono::NaiveDate::parse_from_str(date_str, "%Y/%m/%d") {
            let naive_datetime = naive_date.and_hms_opt(0, 0, 0)?;
            return Some(chrono::DateTime::from_naive_utc_and_offset(
                naive_datetime,
                Utc,
            ));
        }

        // Try "Month DD, YYYY" format (January 15, 2025)
        if let Ok(naive_date) = chrono::NaiveDate::parse_from_str(date_str, "%B %d, %Y") {
            let naive_datetime = naive_date.and_hms_opt(0, 0, 0)?;
            return Some(chrono::DateTime::from_naive_utc_and_offset(
                naive_datetime,
                Utc,
            ));
        }

        // Try "Month YYYY" format (January 2025)
        if let Ok(naive_date) = chrono::NaiveDate::parse_from_str(date_str, "%B %Y") {
            let naive_datetime = naive_date.and_hms_opt(0, 0, 0)?;
            return Some(chrono::DateTime::from_naive_utc_and_offset(
                naive_datetime,
                Utc,
            ));
        }

        // Try year-only format (YYYY)
        if let Ok(year) = date_str.parse::<i32>() {
            if (1000..=9999).contains(&year) {
                let naive_date = chrono::NaiveDate::from_ymd_opt(year, 1, 1)?;
                let naive_datetime = naive_date.and_hms_opt(0, 0, 0)?;
                return Some(chrono::DateTime::from_naive_utc_and_offset(
                    naive_datetime,
                    Utc,
                ));
            }
        }

        None
    }

    /// Extract a LaTeX command argument (handles nested braces)
    /// Returns the content inside braces and the position after the closing brace
    fn extract_braced_arg(source: &str, start_pos: usize) -> Option<(String, usize)> {
        let bytes = source.as_bytes();
        if start_pos >= bytes.len() || bytes[start_pos] != b'{' {
            return None;
        }

        let mut depth = 0;
        let mut end_pos = start_pos;
        for (i, &byte) in bytes.iter().enumerate().skip(start_pos) {
            match byte {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        end_pos = i;
                        break;
                    }
                }
                _ => {}
            }
        }

        if depth != 0 {
            return None; // Unmatched braces
        }

        Some((source[start_pos + 1..end_pos].to_string(), end_pos + 1))
    }

    /// Extract resume headings (\resumeSubheading and \resumeProjectHeading)
    /// These are custom commands used in resume templates that expand to tabular* with two rows
    // Method signature kept for API consistency with other LaTeXBackend methods
    #[allow(clippy::unused_self)]
    fn extract_resume_headings(&self, source: &str) -> Vec<(Vec<String>, usize)> {
        let mut headings = Vec::new();

        // Match \resumeSubheading{arg1}{arg2}{arg3}{arg4}
        for cap in RE_RESUME_SUBHEADING.find_iter(source) {
            let line_no = source[..cap.start()].matches('\n').count() + 1;
            let mut pos = cap.end() - 1; // Position at '{'

            // Extract 4 arguments
            let args: Vec<String> = (0..4)
                .filter_map(|_| {
                    // Skip whitespace before next argument
                    while pos < source.len() && source.as_bytes()[pos].is_ascii_whitespace() {
                        pos += 1;
                    }
                    Self::extract_braced_arg(source, pos).map(|(content, next_pos)| {
                        pos = next_pos;
                        Self::clean_latex_text(&content)
                    })
                })
                .collect();

            if args.len() == 4 {
                let row1 = if args[1].is_empty() {
                    args[0].clone()
                } else {
                    format!("{} | {}", args[0], args[1])
                };
                let row2 = if args[3].is_empty() {
                    args[2].clone()
                } else {
                    format!("{} | {}", args[2], args[3])
                };
                headings.push((vec![row1, row2], line_no));
            }
        }

        // Match \resumeProjectHeading{arg1}{arg2}
        for cap in RE_RESUME_PROJECT.find_iter(source) {
            let line_no = source[..cap.start()].matches('\n').count() + 1;
            let mut pos = cap.end() - 1; // Position at '{'

            // Extract 2 arguments
            let args: Vec<String> = (0..2)
                .filter_map(|_| {
                    // Skip whitespace before next argument
                    while pos < source.len() && source.as_bytes()[pos].is_ascii_whitespace() {
                        pos += 1;
                    }
                    Self::extract_braced_arg(source, pos).map(|(content, next_pos)| {
                        pos = next_pos;
                        Self::clean_latex_text(&content)
                    })
                })
                .collect();

            if args.len() == 2 {
                let row = if args[1].is_empty() {
                    args[0].clone()
                } else {
                    format!("{} | {}", args[0], args[1])
                };
                headings.push((vec![row], line_no));
            }
        }

        headings.sort_by_key(|(_, line)| *line);
        headings
    }

    /// Extract lists (itemize, enumerate) from LaTeX
    fn extract_lists(&self, source: &str) -> Vec<(Vec<String>, String, usize)> {
        let mut lists = Vec::new();

        // Match itemize environments
        for cap in RE_ITEMIZE.captures_iter(source) {
            if let Some(content_match) = cap.get(1) {
                let content = content_match.as_str();
                let line_no = source[..cap.get(0).expect("group 0 always exists").start()]
                    .matches('\n')
                    .count()
                    + 1;
                let items = self.extract_list_items(content);
                if !items.is_empty() {
                    lists.push((items, "bullet".to_string(), line_no));
                }
            }
        }

        // Match enumerate environments
        for cap in RE_ENUMERATE.captures_iter(source) {
            if let Some(content_match) = cap.get(1) {
                let content = content_match.as_str();
                let line_no = source[..cap.get(0).expect("group 0 always exists").start()]
                    .matches('\n')
                    .count()
                    + 1;
                let items = self.extract_list_items(content);
                if !items.is_empty() {
                    lists.push((items, "number".to_string(), line_no));
                }
            }
        }

        // Match custom resume list commands: \resumeItemListStart...\resumeItemListEnd
        // These expand to itemize environments but are used in resume templates
        for cap in RE_RESUME_LIST.captures_iter(source) {
            if let Some(content_match) = cap.get(1) {
                let content = content_match.as_str();
                // Remove comments BEFORE extracting resume items
                let content_no_comments = RE_COMMENT.replace_all(content, "");
                let line_no = source[..cap.get(0).expect("group 0 always exists").start()]
                    .matches('\n')
                    .count()
                    + 1;
                let items: Vec<String> = RE_RESUME_ITEM
                    .captures_iter(&content_no_comments)
                    .filter_map(|item_cap| {
                        item_cap.get(1).map(|m| Self::clean_latex_text(m.as_str()))
                    })
                    .filter(|text| !text.is_empty())
                    .collect();
                if !items.is_empty() {
                    lists.push((items, "bullet".to_string(), line_no));
                }
            }
        }

        // Sort by line number
        lists.sort_by_key(|(_, _, line)| *line);
        lists
    }

    /// Extract individual items from list content
    // Method signature kept for API consistency with other LaTeXBackend methods
    #[allow(clippy::unused_self)]
    fn extract_list_items(&self, content: &str) -> Vec<String> {
        let mut items = Vec::new();

        // Split by \item and take all text until next \item or end
        let parts: Vec<&str> = content.split(r"\item").collect();

        for part in parts {
            let trimmed = part.trim();
            if !trimmed.is_empty() {
                let item_text = Self::clean_latex_text(trimmed);
                // Filter out environment parameters like [leftmargin=0.15in, label={}]
                // These start with '[' and end with ']'
                if !item_text.is_empty()
                    && !item_text.starts_with('[')
                    && !item_text.ends_with(']')
                    && item_text.len() > 3
                {
                    items.push(item_text);
                }
            }
        }

        items
    }

    /// Extract tables from LaTeX (tabular and tabular* environments)
    fn extract_tables(&self, source: &str) -> Vec<(Vec<Vec<String>>, usize)> {
        let mut tables = Vec::new();

        for cap in RE_TABULAR.captures_iter(source) {
            // Check if this table is in a commented-out section
            // Look for the line containing \begin{tabular} and check if it starts with %
            let table_start = cap.get(0).expect("group 0 always exists").start();
            let line_start_pos = source[..table_start].rfind('\n').map_or(0, |p| p + 1);
            let line_prefix = &source[line_start_pos..table_start];

            // Skip if line starts with % (comment)
            if line_prefix.trim_start().starts_with('%') {
                continue;
            }

            if let Some(content_match) = cap.get(1) {
                let content = content_match.as_str();
                let line_no = source[..content_match.start()].matches('\n').count() + 1;
                let rows = self.parse_table_content(content);
                if !rows.is_empty() {
                    tables.push((rows, line_no));
                }
            }
        }

        tables
    }

    /// Parse table content into rows and cells
    // Method signature kept for API consistency with other LaTeXBackend methods
    #[allow(clippy::unused_self)]
    fn parse_table_content(&self, content: &str) -> Vec<Vec<String>> {
        let mut rows = Vec::new();

        // Split by \\\\ (which represents \\ in LaTeX - row separator)
        let parts: Vec<&str> = content.split("\\\\").collect();

        for row_text in &parts {
            // Remove \hline commands and skip if empty
            let cleaned = row_text.replace(r"\hline", "");
            let trimmed = cleaned.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Split cells by & (column separator)
            let cells: Vec<String> = trimmed
                .split('&')
                .map(Self::clean_latex_text)
                .filter(|cell| !cell.is_empty())
                .collect();

            if !cells.is_empty() {
                rows.push(cells);
            }
        }

        rows
    }

    /// Extract paragraphs of text from LaTeX
    /// Extract center blocks (often used for headers/titles in resumes)
    // Method signature kept for API consistency with other LaTeXBackend methods
    #[allow(clippy::unused_self)]
    fn extract_center_blocks(&self, source: &str) -> Vec<(String, usize)> {
        let mut blocks = Vec::new();

        for cap in RE_CENTER.captures_iter(source) {
            if let Some(begin_match) = cap.get(0) {
                // Use the start of \begin{center} for line number, not content
                let line_no = source[..begin_match.start()].matches('\n').count() + 1;
                if let Some(content_match) = cap.get(1) {
                    let content = content_match.as_str();
                    let text = Self::clean_latex_text(content);
                    if !text.is_empty() {
                        blocks.push((text, line_no));
                    }
                }
            }
        }

        blocks
    }

    // Method signature kept for API consistency with other LaTeXBackend methods
    #[allow(clippy::unused_self)]
    fn extract_paragraphs(&self, source: &str) -> Vec<(String, usize)> {
        let mut paragraphs = Vec::new();

        let cleaned = RE_REMOVE_ENV.replace_all(source, "");
        let cleaned = RE_SECTION_CMD.replace_all(&cleaned, "");
        let cleaned = RE_RESUME_LIST_PARA.replace_all(&cleaned, "");
        let cleaned = RE_RESUME_SUBHEADING_LIST.replace_all(&cleaned, "");
        let cleaned = RE_RESUME_SUBHEADING_PARA.replace_all(&cleaned, "");
        let cleaned = RE_RESUME_PROJECT_PARA.replace_all(&cleaned, "");
        let cleaned = RE_DOC_BEGIN.replace(&cleaned, "");
        let cleaned = RE_DOC_END.replace(&cleaned, "");

        // Split by double newlines (paragraph breaks in LaTeX)
        for (idx, para) in cleaned.split("\n\n").enumerate() {
            let text = Self::clean_latex_text(para);
            if !text.is_empty() && text.len() > 20 && !text.starts_with('\\') {
                paragraphs.push((text, idx + 1));
            }
        }

        paragraphs
    }

    /// Parse LaTeX into structured `DocItems`
    #[allow(clippy::too_many_lines)] // Complex LaTeX parsing - keeping together for clarity
    fn parse_latex_to_doc_items(&self, source: &str) -> Vec<DocItem> {
        // Collect items with their line numbers for proper document-order sorting
        let mut items_with_line_nos: Vec<(DocItem, usize)> = Vec::new();

        // Extract only the document body (after \begin{document})
        // This skips preamble with custom command definitions
        let doc_start = source.find(r"\begin{document}").unwrap_or(0);
        let doc_end = source.find(r"\end{document}").unwrap_or(source.len());
        let document_body = &source[doc_start..doc_end];

        // Calculate line offset for document_body (line numbers relative to full source)
        let line_offset = source[..doc_start].matches('\n').count();

        // Extract center blocks first (often headers/contact info in resumes)
        let center_blocks = self.extract_center_blocks(document_body);
        for (text, line_no) in center_blocks {
            items_with_line_nos.push((
                DocItem::Text {
                    self_ref: String::new(), // Will be set after sorting
                    parent: None,
                    children: vec![],
                    content_layer: "body".to_string(),
                    prov: vec![create_default_provenance(line_no + line_offset)],
                    orig: text.clone(),
                    text,
                    formatting: None,
                    hyperlink: None,
                },
                line_no + line_offset,
            ));
        }

        // Extract sections (with line numbers for sorting)
        let sections = self.extract_sections(document_body);
        for (title, level, line_no) in sections {
            if !title.is_empty() {
                let formatted_title = format!("{level} {title}");
                items_with_line_nos.push((
                    DocItem::Text {
                        self_ref: String::new(), // Will be set after sorting
                        parent: None,
                        children: vec![],
                        content_layer: "body".to_string(),
                        prov: vec![create_default_provenance(line_no + line_offset)],
                        orig: formatted_title.clone(),
                        text: formatted_title,
                        formatting: None,
                        hyperlink: None,
                    },
                    line_no + line_offset,
                ));
            }
        }

        // Extract resume headings (\resumeSubheading, \resumeProjectHeading)
        let resume_headings = self.extract_resume_headings(document_body);
        for (lines, line_no) in resume_headings {
            for line_text in lines {
                if !line_text.is_empty() {
                    items_with_line_nos.push((
                        DocItem::Text {
                            self_ref: String::new(), // Will be set after sorting
                            parent: None,
                            children: vec![],
                            content_layer: "body".to_string(),
                            prov: vec![create_default_provenance(line_no + line_offset)],
                            orig: line_text.clone(),
                            text: line_text,
                            formatting: None,
                            hyperlink: None,
                        },
                        line_no + line_offset,
                    ));
                }
            }
        }

        // Extract lists from document body only (with line numbers for sorting)
        let lists = self.extract_lists(document_body);
        for (items, list_type, list_line_no) in lists {
            let enumerated = list_type == "number";

            // Add list items as ListItem DocItems (not Text!)
            for (idx, item_text) in items.iter().enumerate() {
                let marker = if enumerated {
                    format!("{}.", idx + 1)
                } else {
                    "•".to_string()
                };

                items_with_line_nos.push((
                    DocItem::ListItem {
                        self_ref: String::new(), // Will be set after sorting
                        parent: None,
                        children: vec![],
                        content_layer: "body".to_string(),
                        prov: vec![create_default_provenance(list_line_no + line_offset)],
                        orig: item_text.clone(),
                        text: item_text.clone(),
                        enumerated,
                        marker,
                        formatting: None,
                        hyperlink: None,
                    },
                    list_line_no + line_offset,
                ));
            }
        }

        // Extract tables from document body only (with line numbers for sorting)
        let tables = self.extract_tables(document_body);
        for (rows, line_no) in tables {
            use docling_core::content::{TableCell, TableData};

            let num_rows = rows.len();
            let num_cols = rows.iter().map(std::vec::Vec::len).max().unwrap_or(0);

            // Create grid of cells
            let grid: Vec<Vec<TableCell>> = rows
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|cell_text| TableCell {
                            text: cell_text.clone(),
                            row_span: None,
                            col_span: None,
                            ref_item: None,
                            start_row_offset_idx: None,
                            start_col_offset_idx: None,
                            column_header: false,
                            row_header: false,
                            from_ocr: false,
                            confidence: None,
                            bbox: None,
                        })
                        .collect()
                })
                .collect();

            items_with_line_nos.push((
                DocItem::Table {
                    self_ref: String::new(), // Will be set after sorting
                    parent: None,
                    children: vec![],
                    content_layer: "body".to_string(),
                    prov: vec![create_default_provenance(line_no + line_offset)],
                    data: TableData {
                        num_rows,
                        num_cols,
                        grid,
                        table_cells: None,
                    },
                    captions: vec![],
                    footnotes: vec![],
                    references: vec![],
                    image: None,
                    annotations: vec![],
                },
                line_no + line_offset,
            ));
        }

        // Extract paragraphs from document body only (with line numbers for sorting)
        let paragraphs = self.extract_paragraphs(document_body);
        for (text, line_no) in paragraphs {
            if !text.is_empty() {
                items_with_line_nos.push((
                    DocItem::Text {
                        self_ref: String::new(), // Will be set after sorting
                        parent: None,
                        children: vec![],
                        content_layer: "body".to_string(),
                        prov: vec![create_default_provenance(line_no + line_offset)],
                        orig: text.clone(),
                        text,
                        formatting: None,
                        hyperlink: None,
                    },
                    line_no + line_offset,
                ));
            }
        }

        // Sort all items by line number to preserve document order
        items_with_line_nos.sort_by_key(|(_, line_no)| *line_no);

        // Extract items and assign correct self_ref indices
        let mut doc_items = Vec::new();
        let mut text_count = 0;
        let mut table_count = 0;

        for (mut item, _) in items_with_line_nos {
            // Assign correct self_ref based on item type
            match &mut item {
                DocItem::Text { self_ref, .. } | DocItem::ListItem { self_ref, .. } => {
                    *self_ref = format!("#/texts/{text_count}");
                    text_count += 1;
                }
                DocItem::Table { self_ref, .. } => {
                    *self_ref = format!("#/tables/{table_count}");
                    table_count += 1;
                }
                _ => {}
            }
            doc_items.push(item);
        }

        // If we didn't extract anything useful, fall back to simple cleaning
        if doc_items.is_empty() {
            let full_text = Self::clean_latex_text(source);
            if !full_text.is_empty() {
                doc_items.push(DocItem::Text {
                    self_ref: "#/texts/0".to_string(),
                    parent: None,
                    children: vec![],
                    content_layer: "body".to_string(),
                    prov: vec![create_default_provenance(1)],
                    orig: full_text.clone(),
                    text: full_text,
                    formatting: None,
                    hyperlink: None,
                });
            }
        }

        // Items are already sorted by line number above (line 501)
        doc_items
    }

    /// Parse a LaTeX file
    ///
    /// # Errors
    ///
    /// Returns an error if the input file cannot be read.
    #[must_use = "this function returns a parsed document that should be processed"]
    pub fn parse(&mut self, input_path: &Path) -> Result<Document> {
        // Read the LaTeX file
        let source = std::fs::read_to_string(input_path)
            .with_context(|| format!("Failed to read LaTeX file: {}", input_path.display()))?;

        // Extract metadata
        let (title_from_doc, author, date) = self.extract_metadata(&source);

        // Parse LaTeX and generate DocItems
        let doc_items = self.parse_latex_to_doc_items(&source);

        // Determine title
        let title = title_from_doc
            .or_else(|| {
                // Try to extract from first section
                doc_items.iter().find_map(|item| match item {
                    DocItem::Text { text, .. } => text
                        .starts_with('#')
                        .then(|| text.trim_start_matches('#').trim().to_string()),
                    _ => None,
                })
            })
            .unwrap_or_else(|| {
                input_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("LaTeX Document")
                    .to_string()
            });

        // Generate markdown from DocItems
        let markdown = doc_items
            .iter()
            .filter_map(|item| {
                match item {
                    DocItem::Text { text, .. } => Some(text.clone()),
                    DocItem::ListItem { text, marker, .. } => {
                        // Format list item with marker
                        Some(format!("{marker} {text}"))
                    }
                    DocItem::Table { data, .. } => {
                        // Serialize table to markdown (GitHub format)
                        Self::serialize_table_simple(data)
                    }
                    _ => {
                        // Handle other DocItem types if needed
                        None
                    }
                }
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        // Create metadata
        // Parse date and populate created field if available
        let created = date.as_ref().and_then(|d| Self::parse_date(d));

        let metadata = DocumentMetadata {
            title: Some(title),
            author,
            subject: None,
            created,
            modified: None,
            num_pages: None,
            num_characters: markdown.len(),
            language: None,
            exif: None,
        };

        // Create Document with DocItems
        Ok(Document {
            markdown,
            format: InputFormat::Tex,
            content_blocks: Some(doc_items),
            metadata,
            docling_document: None,
        })
    }

    /// Get the backend name
    #[inline]
    #[must_use = "returns backend name string"]
    pub const fn name(&self) -> &'static str {
        "LaTeX (Pure Rust)"
    }

    /// Serialize a table to markdown format (simplified version)
    ///
    /// This creates GitHub-style markdown tables from `TableData`.
    /// More complex tables with spans, refs, etc. should use the full `MarkdownSerializer`.
    fn serialize_table_simple(data: &docling_core::content::TableData) -> Option<String> {
        if data.grid.is_empty() {
            return None;
        }

        let rows = &data.grid;
        let num_cols = data.num_cols;

        // Build table rows
        let mut md_lines = Vec::new();

        // First row is header
        if !rows.is_empty() {
            let header_cells: Vec<String> = rows[0]
                .iter()
                .take(num_cols)
                .map(|cell| cell.text.clone())
                .collect();
            md_lines.push(format!("| {} |", header_cells.join(" | ")));

            // Separator row
            let separators = vec!["---"; num_cols];
            md_lines.push(format!("|{}|", separators.join("|")));

            // Data rows
            for row in rows.iter().skip(1) {
                let row_cells: Vec<String> = row
                    .iter()
                    .take(num_cols)
                    .map(|cell| cell.text.clone())
                    .collect();
                md_lines.push(format!("| {} |", row_cells.join(" | ")));
            }
        }

        if md_lines.is_empty() {
            None
        } else {
            Some(md_lines.join("\n"))
        }
    }
}

/// Create default provenance - delegates to shared implementation
#[inline]
const fn create_default_provenance(page: usize) -> ProvenanceItem {
    ProvenanceItem::default_for_page(page, CoordOrigin::TopLeft)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(
        clippy::default_constructed_unit_structs,
        reason = "testing Default trait impl"
    )]
    fn test_latex_backend_default() {
        // Verify derived Default produces unit struct
        let backend = LatexBackend::default();
        // LatexBackend is a unit struct, so default() and new().unwrap() should be equal
        assert_eq!(backend, LatexBackend::new().unwrap());
    }

    #[test]
    fn test_clean_latex_text() {
        let _backend = LatexBackend::new().unwrap();

        let text = r"\section{Introduction} This is \textbf{bold} and \textit{italic} text.";
        let cleaned = LatexBackend::clean_latex_text(text);
        assert!(cleaned.contains("Introduction"));
        assert!(cleaned.contains("bold"));
        assert!(cleaned.contains("italic"));
    }

    #[test]
    fn test_parse_simple_latex() {
        let backend = LatexBackend::new().unwrap();

        let latex = r"\documentclass{article}
\begin{document}
\section{Test}
This is a test paragraph with some content.
\end{document}";

        let doc_items = backend.parse_latex_to_doc_items(latex);
        assert!(!doc_items.is_empty(), "Should extract at least one DocItem");

        // Check that we have a section
        let has_section = doc_items.iter().any(|item| {
            if let DocItem::Text { text, .. } = item {
                text.contains("Test")
            } else {
                false
            }
        });
        assert!(has_section, "Should extract section heading");
    }

    #[test]
    fn test_extract_metadata() {
        let backend = LatexBackend::new().unwrap();

        let latex = r"\title{My Document}
\author{John Doe}
\date{2025-01-15}
\begin{document}
Content here.
\end{document}";

        let (title, author, date) = backend.extract_metadata(latex);
        assert_eq!(title, Some("My Document".to_string()));
        assert_eq!(author, Some("John Doe".to_string()));
        assert_eq!(date, Some("2025-01-15".to_string()));
    }

    #[test]
    fn test_parse_list() {
        let backend = LatexBackend::new().unwrap();

        let latex = r"\documentclass{article}
\begin{document}
\section{Test}
\begin{itemize}
\item First item
\item Second item
\end{itemize}
\end{document}";

        let doc_items = backend.parse_latex_to_doc_items(latex);
        // Should have: 1 section + 2 list items
        assert!(
            doc_items.len() >= 3,
            "Should extract section and list items"
        );

        // Check that we have list items (now using ListItem DocItem type)
        let has_list_items = doc_items.iter().any(|item| {
            matches!(
                item,
                DocItem::ListItem {
                    enumerated: false,
                    ..
                }
            )
        });
        assert!(has_list_items, "Should extract list items");
    }

    #[test]
    fn test_parse_table() {
        let backend = LatexBackend::new().unwrap();

        let latex = r"\documentclass{article}
\begin{document}
\begin{tabular}{|c|c|}
\hline
Header 1 & Header 2 \\
\hline
Cell 1 & Cell 2 \\
\hline
\end{tabular}
\end{document}";

        let doc_items = backend.parse_latex_to_doc_items(latex);

        // Check that we have a table
        let has_table = doc_items
            .iter()
            .any(|item| matches!(item, DocItem::Table { .. }));
        assert!(has_table, "Should extract table");

        // Check table structure
        if let Some(DocItem::Table { data, .. }) = doc_items
            .iter()
            .find(|item| matches!(item, DocItem::Table { .. }))
        {
            assert_eq!(data.num_rows, 2, "Should have 2 rows");
            assert_eq!(data.num_cols, 2, "Should have 2 columns");
        }
    }

    #[test]
    fn test_formatting_preservation() {
        let text = r"This is \textbf{bold} and \textit{italic} text.";
        let cleaned = LatexBackend::clean_latex_text(text);
        assert!(
            cleaned.contains("**bold**"),
            "Should preserve bold as markdown"
        );
        assert!(
            cleaned.contains("*italic*"),
            "Should preserve italic as markdown"
        );
    }

    #[test]
    fn test_parse_date() {
        use chrono::Datelike;

        // Test ISO 8601 date format (most common in LaTeX)
        let dt = LatexBackend::parse_date("2025-01-15");
        assert!(dt.is_some(), "Should parse ISO date");
        let dt = dt.unwrap();
        assert_eq!(dt.year(), 2025);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 15);

        // Test date with slashes
        let dt = LatexBackend::parse_date("2025/01/15");
        assert!(dt.is_some(), "Should parse date with slashes");
        let dt = dt.unwrap();
        assert_eq!(dt.year(), 2025);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 15);

        // Test year-only format
        let dt = LatexBackend::parse_date("2025");
        assert!(dt.is_some(), "Should parse year-only");
        let dt = dt.unwrap();
        assert_eq!(dt.year(), 2025);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 1);

        // Test invalid date
        let dt = LatexBackend::parse_date("not a date");
        assert!(dt.is_none(), "Should return None for invalid date");
    }

    #[test]
    fn test_date_metadata_integration() {
        use chrono::Datelike;
        use std::io::Write;

        let mut backend = LatexBackend::new().unwrap();

        let latex = r"\documentclass{article}
\title{Test Document}
\author{Jane Smith}
\date{2025-01-15}
\begin{document}
Content
\end{document}";

        // Create a temporary file
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_latex_date.tex");
        {
            let mut file = std::fs::File::create(&temp_file).unwrap();
            file.write_all(latex.as_bytes()).unwrap();
        }

        let document = backend.parse(&temp_file).unwrap();

        // Clean up
        std::fs::remove_file(&temp_file).ok();

        // Verify metadata
        assert_eq!(document.metadata.title, Some("Test Document".to_string()));
        assert_eq!(document.metadata.author, Some("Jane Smith".to_string()));

        // Verify date was parsed into created field
        assert!(
            document.metadata.created.is_some(),
            "Date should be parsed into created field"
        );
        let created = document.metadata.created.unwrap();
        assert_eq!(created.year(), 2025);
        assert_eq!(created.month(), 1);
        assert_eq!(created.day(), 15);
    }

    #[test]
    fn test_resume_template_parsing() {
        let mut backend = LatexBackend::new().unwrap();
        let test_file = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../test-corpus/latex/resume_template.tex"
        );
        let input_path = std::path::Path::new(test_file);

        // Skip if file doesn't exist (CI environment)
        if !input_path.exists() {
            eprintln!("Skipping test - resume_template.tex not found");
            return;
        }

        let document = backend.parse(input_path).unwrap();

        println!("\n=== RESUME TEMPLATE ANALYSIS ===");
        println!("Markdown length: {} chars", document.markdown.len());

        if let Some(ref items) = document.content_blocks {
            println!("Total DocItems: {}", items.len());

            for (idx, item) in items.iter().enumerate() {
                match item {
                    DocItem::Text { text, .. } => {
                        println!("  [{idx}] Text: {text}");
                    }
                    DocItem::ListItem { text, marker, .. } => {
                        println!("  [{idx}] ListItem({marker}): {text}");
                    }
                    DocItem::Table { data, .. } => {
                        println!("  [{}] Table: {}x{}", idx, data.num_rows, data.num_cols);
                        for (row_idx, row) in data.grid.iter().enumerate() {
                            for (col_idx, cell) in row.iter().enumerate() {
                                println!("      [{},{}]: {}", row_idx, col_idx, cell.text);
                            }
                        }
                    }
                    _ => {}
                }
            }

            // Check for expected sections
            let markdown_lower = document.markdown.to_lowercase();
            println!("\n=== SECTION PRESENCE CHECK ===");
            println!("Has 'education': {}", markdown_lower.contains("education"));
            println!(
                "Has 'experience': {}",
                markdown_lower.contains("experience")
            );
            println!(
                "Has 'projects': {}",
                markdown_lower.contains("projects") || markdown_lower.contains("gitlytics")
            );
            println!(
                "Has 'technical skills': {}",
                markdown_lower.contains("technical skills")
                    || markdown_lower.contains("languages:")
            );
        }

        println!("\n=== FULL MARKDOWN ===");
        println!("{}", document.markdown);
        println!("=== END ===\n");
    }
}
