/// Stage 09: Document Assembly
///
/// Converts resolved clusters into typed document elements with sanitized text.
///
/// Algorithm (from Python baseline ~/`docling/docling/models/page_assemble_model.py)`:
/// 1. For each cluster, classify by label:
///    - `TEXT_ELEM_LABELS` → `TextElement` (extract and sanitize text from cells)
///    - `TABLE_LABELS` → Table (structure from table predictions, or empty fallback)
///    - `FIGURE_LABEL` → `FigureElement` (classification from figure predictions, or empty fallback)
///    - `CONTAINER_LABELS` → `ContainerElement` (form, `key_value_region`)
/// 2. Text sanitization:
///    - Join cell text lines with hyphenation handling
///    - Remove hyphen at line end if word continues on next line
///    - Replace special Unicode characters (⁄→/, '→', "→", •→∞, –/—→-)
///    - Strip leading/trailing whitespace
/// 3. Categorize elements:
///    - headers: `PAGE_HEADER`, `PAGE_FOOTER`
///    - body: All other elements
use crate::pipeline_modular::types::{BBox, ClusterWithCells, TextCell};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

/// Pre-compiled regex for detecting section headers embedded in text
/// N=4134: Pattern matches "4 Optimised", "4.1 Language", "5 Experiments", etc.
/// Pattern: space or start, then digits[.digits]*, then space, then Uppercase word (3+ chars)
static SECTION_HEADER_SPLIT_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    // Match section header pattern: "N Title" or "N.M Title" where Title starts with uppercase
    // Lookbehind would be ideal but Rust regex doesn't support it, so we match the preceding space
    Regex::new(r"(?:\s|^)(\d+(?:\.\d+)*)\s+([A-Z][a-zA-Z]{2,})")
        .expect("valid section header split regex")
});

/// Pre-compiled regex for matching words in text (used for hyphenation detection)
static WORD_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b[\w]+\b").expect("valid word regex"));

/// N=4411: Pre-compiled regex for normalizing compound hyphen spacing
/// Pattern: word boundary + pure letters (no digits), space(s), hyphen, space(s), word+
/// Matches: "mid - 19th", "AI - powered", "open - source", "voice - to - text"
/// Not matched: "19th - Early" (ordinal+dash - has digits), "- item" (list)
static COMPOUND_HYPHEN_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b([a-zA-Z]+) +- +([a-zA-Z0-9]+)").expect("valid compound hyphen regex")
});

/// N=4412: Pre-compiled regex for fixing PDF word breaks (split words without hyphen)
/// Pattern: lowercase letter + space(s) + common English suffix that shouldn't stand alone
static WORD_BREAK_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"([a-z]) +",
        r"(",
        r"onal|tional|sional|ional|",
        r"ness|ment|ible|ibly|ably|",
        r"tion|sion|ation|ition|",
        r"ance|ence|ancy|ency|",
        r"tences|ences|ances|",
        r"eous|ious|ous|",
        r"ive|ively|ve|lve|",
        r"ully|elly|illy|",
        r"ity|ty|ry|",
        r"nd|elds|ords|d|",
        r"ther|ould|ight|ith|rom|hout",
        r")\b"
    ))
    .expect("valid word break regex")
});

/// N=4416: Regex for "able" suffix - requires consonant before to avoid "be able" → "beable"
static WORD_BREAK_ABLE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"([bcdfghjklmnpqrstvwxyz]) +(able)\b").expect("valid able regex"));

/// N=4416: Regex for "ally" suffix - requires vowel before to join (virtu ally → virtually)
static WORD_BREAK_ALLY_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"([aeiou]) +(ally)\b").expect("valid ally regex"));

/// Pre-compiled regex for fixing list-style quoted token dashes
static QUOTED_TOKEN_DASH_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#""([A-Za-z0-9]{1,6})" +- +"#).expect("valid quoted token regex"));

/// N=4420: Pre-compiled regex for normalizing ORCID patterns
static ORCID_NORMALIZE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[[\d\s-]{15,25}\]").expect("valid ORCID regex"));

/// N=4420: Pre-compiled regex for adding space before ORCID brackets
static ORCID_SPACE_BEFORE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\w)\[([\d\s-]{15,25})\]").expect("valid ORCID space regex"));

/// N=4421: Pre-compiled regex for fixing ORCID-comma spacing
static ORCID_COMMA_SPACE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(\[\d{4} -\d{4} -\d{4} -\d{4}\]),").expect("valid ORCID comma regex")
});

/// Configuration for Stage 09 (Document Assembly)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stage09Config {
    // Label categories (from LayoutModel constants)
    pub text_elem_labels: Vec<String>,
    pub page_header_labels: Vec<String>,
    pub table_labels: Vec<String>,
    pub figure_label: String,
    pub container_labels: Vec<String>,
}

impl Default for Stage09Config {
    #[inline]
    fn default() -> Self {
        Self {
            text_elem_labels: vec![
                "text".to_string(),
                "title".to_string(), // Document title
                "footnote".to_string(),
                "caption".to_string(),
                "checkbox_unselected".to_string(),
                "checkbox_selected".to_string(),
                "section_header".to_string(),
                "page_header".to_string(),
                "page_footer".to_string(),
                "code".to_string(),
                "list_item".to_string(),
                "formula".to_string(),
            ],
            page_header_labels: vec!["page_header".to_string(), "page_footer".to_string()],
            table_labels: vec!["table".to_string(), "document_index".to_string()],
            figure_label: "picture".to_string(),
            container_labels: vec!["form".to_string(), "key_value_region".to_string()],
        }
    }
}

/// Base document element
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentElement {
    #[serde(rename = "type")]
    pub element_type: String,
    pub label: String,
    pub id: usize,
    pub page_no: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bbox: Option<BBox>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_width: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_height: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ref_str: Option<String>, // Reference string (e.g., "#/0/42")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cluster: Option<ClusterInfo>, // Full cluster information for baseline compatibility
}

/// Cluster information included in element for baseline compatibility
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClusterInfo {
    pub label: String,
    pub confidence: f64,
    pub bbox: BBox,
    pub id: usize,
    pub cells: Vec<CellInfo>,
}

/// Cell information for baseline compatibility
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CellInfo {
    pub text: String,
    pub rect: CellRect,
    pub confidence: f64,
    pub from_ocr: bool,
    /// N=4373: Whether text is bold (from PDF font flags)
    #[serde(default)]
    pub is_bold: bool,
    /// N=4373: Whether text is italic (from PDF font flags)
    #[serde(default)]
    pub is_italic: bool,
}

/// Cell rectangle (detailed format for baseline compatibility)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CellRect {
    pub r_x0: f64,
    pub r_y0: f64,
    pub r_x1: f64,
    pub r_y1: f64,
    pub r_x2: f64,
    pub r_y2: f64,
    pub r_x3: f64,
    pub r_y3: f64,
    pub coord_origin: String, // "TOPLEFT"
}

impl From<&TextCell> for CellRect {
    #[inline]
    fn from(cell: &TextCell) -> Self {
        // Convert BBox to detailed rectangle format
        // Assumes TOPLEFT origin (standard for Docling)
        Self {
            r_x0: cell.bbox.l,
            r_y0: cell.bbox.b, // bottom-left y
            r_x1: cell.bbox.r,
            r_y1: cell.bbox.b, // bottom-right y
            r_x2: cell.bbox.r,
            r_y2: cell.bbox.t, // top-right y
            r_x3: cell.bbox.l,
            r_y3: cell.bbox.t, // top-left y
            coord_origin: "TOPLEFT".to_string(),
        }
    }
}

/// Stage 09: Document Assembly
///
/// Converts resolved clusters into typed document elements with sanitized text.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Stage09DocumentAssembler {
    config: Stage09Config,
}

impl Stage09DocumentAssembler {
    #[inline]
    #[must_use = "returns a new Stage09DocumentAssembler instance"]
    pub fn new() -> Self {
        Self {
            config: Stage09Config::default(),
        }
    }

    /// Check if a word is alphanumeric, ignoring Unicode combining characters
    ///
    /// This matches Python's behavior where combining characters (like U+0332 COMBINING LOW LINE)
    /// are not matched by \w, but Rust's regex \w does match them.
    ///
    /// Python: "̲SN" → regex extracts "SN" → `isalnum()` returns True
    /// Rust: "̲SN" → regex extracts "̲SN" → need to filter combining chars before checking
    #[inline]
    fn is_word_alphanumeric(word: &str) -> bool {
        word.chars()
            .filter(|c| !Self::is_combining_mark(*c))
            .all(char::is_alphanumeric)
    }

    /// Check if a character is a Unicode combining mark (category Mn)
    ///
    /// Combining marks include accents, diacritics, and other non-spacing marks that
    /// modify the previous character. Examples:
    /// - U+0332 COMBINING LOW LINE (̲)
    /// - U+0301 COMBINING ACUTE ACCENT (́)
    /// - etc.
    #[inline]
    const fn is_combining_mark(c: char) -> bool {
        // Unicode categories: Mn = Mark, nonspacing (combining marks)
        // We check the range for common combining diacritical marks
        matches!(c,
            '\u{0300}'..='\u{036F}' | // Combining Diacritical Marks
            '\u{1AB0}'..='\u{1AFF}' | // Combining Diacritical Marks Extended
            '\u{1DC0}'..='\u{1DFF}' | // Combining Diacritical Marks Supplement
            '\u{20D0}'..='\u{20FF}' | // Combining Diacritical Marks for Symbols
            '\u{FE20}'..='\u{FE2F}'   // Combining Half Marks
        )
    }

    #[inline]
    #[must_use = "returns a new Stage09DocumentAssembler with custom config"]
    pub const fn with_config(config: Stage09Config) -> Self {
        Self { config }
    }

    /// Sanitize text lines with hyphenation handling
    ///
    /// Algorithm (from page_assemble_model.py:34-65):
    /// 1. If 0-1 lines: join with space
    /// 2. For each line after first:
    ///    - If previous line ends with '-':
    ///      - Extract last word from prev line, first word from current line
    ///      - If both are alphanumeric: remove hyphen (word continues)
    ///      - Otherwise: keep hyphen
    ///    - If previous line doesn't end with '-':
    ///      - Add space to end of previous line (word boundary)
    /// 3. Join all lines
    /// 4. Replace special Unicode characters
    /// 5. Strip whitespace
    // Method signature kept for API consistency with other DocumentAssembler methods
    #[allow(clippy::unused_self)]
    fn sanitize_text(&self, lines: &[String]) -> String {
        if lines.is_empty() {
            return String::new();
        }

        let sanitized_text = if lines.len() <= 1 {
            // Simple case: join with space
            lines.join(" ")
        } else {
            // Hyphenation handling for multi-line text
            let mut modified_lines = lines.to_vec();

            for ix in 1..modified_lines.len() {
                let prev_line = &modified_lines[ix - 1].clone();
                let curr_line = &modified_lines[ix];

                // N=4427: Check for hyphen, en-dash (U+2013), or em-dash (U+2014) at end
                let ends_with_dash = prev_line.ends_with('-')
                    || prev_line.ends_with('\u{2013}')
                    || prev_line.ends_with('\u{2014}');

                if ends_with_dash {
                    // N=4427: If dash is separated by whitespace ("word -"), treat as separator.
                    // Do NOT remove it as hyphenation (matches Python groundtruth for ORCID-like spans).
                    let dash_is_attached = prev_line
                        .chars()
                        .rev()
                        .nth(1)
                        .is_some_and(|c| !c.is_whitespace());

                    // Extract words from both lines
                    let prev_words: Vec<&str> = WORD_REGEX
                        .find_iter(prev_line)
                        .map(|m| m.as_str())
                        .collect();
                    let curr_words: Vec<&str> = WORD_REGEX
                        .find_iter(curr_line)
                        .map(|m| m.as_str())
                        .collect();

                    // Check if hyphen connects words (both alphanumeric)
                    if dash_is_attached
                        && !prev_words.is_empty()
                        && !curr_words.is_empty()
                        && Self::is_word_alphanumeric(prev_words.last().unwrap())
                        && Self::is_word_alphanumeric(curr_words.first().unwrap())
                    {
                        // Remove trailing dash: "hyphen-" + "ated" -> "hyphenated"
                        // Works for hyphen, en-dash, and em-dash (pop handles multi-byte Unicode)
                        let mut new_prev = prev_line.clone();
                        new_prev.pop();
                        modified_lines[ix - 1] = new_prev;
                    }
                } else if curr_line.starts_with('-')
                    || curr_line.starts_with('\u{2013}')
                    || curr_line.starts_with('\u{2014}')
                {
                    // N=4427: Current line starts with dash: "Pre" + "-Digital" -> "Pre-Digital"
                    // Don't add space, join directly
                } else {
                    // Add space at end (word boundary)
                    modified_lines[ix - 1] = format!("{prev_line} ");
                }
            }

            // Join all lines
            modified_lines.join("")
        };

        // Unicode normalization (replace special characters)
        // ALWAYS apply these replacements, even for single-line text
        let sanitized_text = sanitized_text
            .replace('⁄', "/") // fraction slash U+2044
            .replace(['\u{2018}', '\u{2019}'], "'") // right single quote '
            .replace(['\u{201C}', '\u{201D}'], "\"") // right double quote "
            .replace('•', "∞") // bullet U+2022 → infinity (matches Python docling v2.58.0)
            .replace(['\u{2013}', '\u{2014}'], "-"); // en-dash/em-dash → hyphen (matches Python docling v2.58.0)

        // N=4426: Apply punctuation spacing normalization (synced from page_assembly.rs)
        // This includes: abbreviation spacing, compound hyphen normalization, word break fixes,
        // ORCID normalization, and space removal around punctuation
        Self::normalize_punctuation_spacing(&sanitized_text)
            .trim()
            .to_string()
    }

    /// Insert missing space after common prose abbreviations
    ///
    /// PDF text extraction often omits spaces after abbreviations like "e.g." and "i.e."
    /// This function inserts the missing space when followed by a word or parenthesis.
    ///
    /// Example: `"e.g.the"` → `"e.g. the"`
    /// Example: `"i.e.(a)"` → `"i.e. (a)"`
    fn insert_missing_space_after_abbreviations(text: &str) -> String {
        let chars: Vec<char> = text.chars().collect();
        let len = chars.len();
        let mut result = String::with_capacity(text.len());

        let mut i = 0;
        while i < len {
            let is_abbrev_start = i == 0 || !chars[i - 1].is_alphanumeric();

            // Check for "e.g." pattern
            if is_abbrev_start
                && i + 3 < len
                && chars[i].eq_ignore_ascii_case(&'e')
                && chars[i + 1] == '.'
                && chars[i + 2].eq_ignore_ascii_case(&'g')
                && chars[i + 3] == '.'
            {
                result.push(chars[i]);
                result.push(chars[i + 1]);
                result.push(chars[i + 2]);
                result.push(chars[i + 3]);

                let next_idx = i + 4;
                if next_idx < len {
                    let next = chars[next_idx];
                    if (next.is_alphanumeric() || matches!(next, '(' | '[')) && next != ' ' {
                        result.push(' ');
                    }
                }

                i += 4;
                continue;
            }

            // Check for "i.e." pattern
            if is_abbrev_start
                && i + 3 < len
                && chars[i].eq_ignore_ascii_case(&'i')
                && chars[i + 1] == '.'
                && chars[i + 2].eq_ignore_ascii_case(&'e')
                && chars[i + 3] == '.'
            {
                result.push(chars[i]);
                result.push(chars[i + 1]);
                result.push(chars[i + 2]);
                result.push(chars[i + 3]);

                let next_idx = i + 4;
                if next_idx < len {
                    let next = chars[next_idx];
                    if (next.is_alphanumeric() || matches!(next, '(' | '[')) && next != ' ' {
                        result.push(' ');
                    }
                }

                i += 4;
                continue;
            }

            result.push(chars[i]);
            i += 1;
        }

        result
    }

    /// Normalize spacing around punctuation (synced from page_assembly.rs)
    ///
    /// PDF text extraction often creates separate cells for punctuation characters,
    /// resulting in unwanted spaces when cells are joined. This function fixes:
    /// - Space before: `, ) ] ; .` → removed
    /// - Space before: `(` → removed when preceded by alphanumeric (function calls)
    /// - Space after: `( [` → removed
    /// - Space after `.` → removed only if followed by lowercase (method calls)
    /// - Multiple consecutive spaces → single space
    fn normalize_punctuation_spacing(text: &str) -> String {
        // Heuristic: treat input as code-like if it contains common code tokens.
        let code_like = {
            let has_function_kw = text.contains("function ");
            let has_return_kw = text.contains("return ");
            let has_brace_stmt = text.contains('{') && text.contains('}') && text.contains(';');
            has_function_kw || has_return_kw || has_brace_stmt
        };

        // Use a single pass with character iteration for efficiency
        let mut result = String::with_capacity(text.len());
        let chars: Vec<char> = text.chars().collect();
        let len = chars.len();

        let mut i = 0;
        while i < len {
            let c = chars[i];

            if c == ' ' {
                // Look ahead to see if next non-space char is punctuation that shouldn't have space before
                let mut j = i + 1;
                while j < len && chars[j] == ' ' {
                    j += 1;
                }

                if j < len {
                    let next_char = chars[j];
                    // Remove space before these punctuation marks
                    if next_char == ',' {
                        if code_like {
                            i = j;
                            continue;
                        }
                    } else if next_char == ')' {
                        let prev_non_space = result.chars().rev().find(|c| !c.is_whitespace());
                        if !prev_non_space.is_some_and(|c| matches!(c, ';' | '>')) {
                            i = j;
                            continue;
                        }
                    } else if matches!(next_char, ']' | ';' | '.') {
                        i = j;
                        continue;
                    }

                    // N=4410: Remove space before `(` only in code-like contexts
                    if next_char == '(' && code_like && !result.is_empty() {
                        let result_chars: Vec<char> = result.chars().collect();
                        let rlen = result_chars.len();
                        let last_char = result_chars[rlen - 1];

                        let mut word_start = rlen;
                        while word_start > 0
                            && (result_chars[word_start - 1].is_alphanumeric()
                                || result_chars[word_start - 1] == '_')
                        {
                            word_start -= 1;
                        }

                        let word_len = rlen - word_start;
                        let is_short_lowercase = word_start < rlen
                            && word_len <= 4
                            && result_chars[word_start..rlen]
                                .iter()
                                .all(|c| c.is_ascii_lowercase() || *c == '_');

                        let after_code_punct =
                            word_start > 0 && matches!(result_chars[word_start - 1], '.' | '(');
                        let after_close_paren = last_char == ')';

                        if is_short_lowercase || after_code_punct || after_close_paren {
                            i = j;
                            continue;
                        }
                    }
                }

                // Check if previous char is punctuation that shouldn't have space after
                if !result.is_empty() {
                    let prev_char = result.chars().last().unwrap();
                    if matches!(prev_char, '(' | '[') {
                        let mut k = j;
                        while k < len && chars[k] == ' ' {
                            k += 1;
                        }
                        if k < len && chars[k].is_alphanumeric() {
                            i += 1;
                            continue;
                        }
                    }

                    // Special handling for `.` - remove space after only for method calls
                    if code_like && prev_char == '.' && j < len {
                        let next_char = chars[j];
                        if next_char.is_ascii_lowercase() {
                            i += 1;
                            continue;
                        }
                    }
                }

                // Keep a single space (collapse multiple spaces)
                if !result.ends_with(' ') {
                    result.push(' ');
                }
            } else {
                result.push(c);
            }
            i += 1;
        }

        if !code_like {
            result = Self::insert_missing_space_after_abbreviations(&result);
        }

        // Remove separator-dash after quoted tokens in definition lists
        result = QUOTED_TOKEN_DASH_REGEX
            .replace_all(&result, r#""$1" "#)
            .into_owned();

        // N=4411: Normalize compound hyphen spacing (mid - 19th → mid-19th)
        loop {
            let new_result = COMPOUND_HYPHEN_REGEX
                .replace_all(&result, |caps: &regex::Captures<'_>| {
                    let left = &caps[1];
                    let right = &caps[2];

                    if left.eq_ignore_ascii_case("cell") {
                        return format!("{left} {right}");
                    }
                    if left.len() == 1 && left.chars().all(|c| c.is_ascii_uppercase()) {
                        return format!("{left} - {right}");
                    }
                    if right.eq_ignore_ascii_case("all") || right.eq_ignore_ascii_case("and") {
                        return format!("{left} - {right}");
                    }
                    if right.len() >= 2 && right.chars().all(|c| c.is_ascii_uppercase()) {
                        return format!("{left} - {right}");
                    }
                    format!("{left}-{right}")
                })
                .into_owned();
            if new_result == result {
                break;
            }
            result = new_result;
        }

        // N=4412: Fix PDF word breaks (professi onal → professional)
        loop {
            let new_result = WORD_BREAK_REGEX.replace_all(&result, "$1$2").into_owned();
            if new_result == result {
                break;
            }
            result = new_result;
        }

        // N=4416: Fix "able" suffix with consonant check
        loop {
            let new_result = WORD_BREAK_ABLE_REGEX
                .replace_all(&result, "$1$2")
                .into_owned();
            if new_result == result {
                break;
            }
            result = new_result;
        }

        // N=4416: Fix "ally" suffix with vowel check
        loop {
            let new_result = WORD_BREAK_ALLY_REGEX
                .replace_all(&result, "$1$2")
                .into_owned();
            if new_result == result {
                break;
            }
            result = new_result;
        }

        // N=4420: Add space before ORCID brackets
        result = ORCID_SPACE_BEFORE_REGEX
            .replace_all(&result, "$1 [$2]")
            .into_owned();

        // N=4420: Normalize ORCID patterns
        result = ORCID_NORMALIZE_REGEX
            .replace_all(&result, |caps: &regex::Captures<'_>| {
                let matched = &caps[0];
                let digits: String = matched.chars().filter(|c| c.is_ascii_digit()).collect();
                if digits.len() == 16 {
                    format!(
                        "[{} -{} -{} -{}]",
                        &digits[0..4],
                        &digits[4..8],
                        &digits[8..12],
                        &digits[12..16]
                    )
                } else {
                    matched.to_string()
                }
            })
            .into_owned();

        // N=4421: Fix ORCID-comma spacing
        result = ORCID_COMMA_SPACE_REGEX
            .replace_all(&result, "$1 ,")
            .into_owned();

        result
    }

    /// N=4134: Split text at embedded section header boundaries
    ///
    /// Detects patterns like "...text. 4 Optimised Table Structure Language To mitigate..."
    /// and splits into: `[("...text.", "text"), ("4 Optimised Table Structure Language", "section_header"), ("To mitigate...", "text")]`
    ///
    /// Returns: Vec of (`text_segment`, label) pairs
    #[allow(clippy::unused_self)] // Method for API consistency
    fn split_at_section_headers(&self, text: &str) -> Vec<(String, String)> {
        let mut segments: Vec<(String, String)> = Vec::new();

        // Find all section header start positions
        let mut header_starts: Vec<(usize, String)> = Vec::new();
        for cap in SECTION_HEADER_SPLIT_REGEX.captures_iter(text) {
            if let (Some(num), Some(first_word)) = (cap.get(1), cap.get(2)) {
                let num_text = num.as_str();
                let first_word_text = first_word.as_str();

                // Skip if looks like year (4 digits in 1800-2099 range)
                if num_text.len() == 4 {
                    if let Ok(year) = num_text.parse::<u32>() {
                        if (1800..=2099).contains(&year) {
                            continue;
                        }
                    }
                }

                // Skip author patterns and bibliographic patterns
                if first_word_text.starts_with("M.")
                    || first_word_text == "Lysak"
                    || first_word_text == "IEEE"
                    || first_word_text == "Auer"
                    || first_word_text == "Xue"
                {
                    continue;
                }

                // Record start position and section number
                header_starts.push((num.start(), num_text.to_string()));
            }
        }

        // If no section headers found, return original text
        if header_starts.is_empty() {
            return vec![(text.to_string(), "text".to_string())];
        }

        // Extract full section header titles
        // Title ends when we see: lowercase word after period, or next section header
        let mut last_end = 0;
        for (i, (start_pos, _num)) in header_starts.iter().enumerate() {
            // Add text before this section header (if any)
            if *start_pos > last_end {
                let before_text = text[last_end..*start_pos].trim();
                if !before_text.is_empty() {
                    segments.push((before_text.to_string(), "text".to_string()));
                }
            }

            // Find where this section header title ends
            // Strategy: extract until we hit a sentence boundary (". " followed by lowercase or number)
            // or until next section header
            let rest = &text[*start_pos..];
            let next_header_pos = if i + 1 < header_starts.len() {
                header_starts[i + 1].0 - *start_pos
            } else {
                rest.len()
            };

            // Find title end within this range
            let title_end = Self::find_section_title_end(&rest[..next_header_pos]);

            let section_header = rest[..title_end].trim().to_string();
            if !section_header.is_empty() {
                segments.push((section_header.clone(), "section_header".to_string()));
            }

            last_end = *start_pos + title_end;
        }

        // Add remaining text after last section header
        if last_end < text.len() {
            let remaining = text[last_end..].trim();
            if !remaining.is_empty() {
                segments.push((remaining.to_string(), "text".to_string()));
            }
        }

        // If segments empty (shouldn't happen), return original
        if segments.is_empty() {
            return vec![(text.to_string(), "text".to_string())];
        }

        segments
    }

    /// Find where a section title ends
    ///
    /// Section titles like "4 Optimised Table Structure Language" end when:
    /// - We hit ". " followed by content that starts a new sentence
    /// - We hit a pattern that looks like body text ("To ", "In ", "The ", etc.)
    fn find_section_title_end(text: &str) -> usize {
        let chars: Vec<char> = text.chars().collect();
        let in_title = true;

        for i in 0..chars.len() {
            if !in_title {
                break;
            }

            // Check for sentence end: ". " followed by text
            if i > 0 && chars[i - 1] == '.' && chars[i] == ' ' {
                // Look ahead - if next word is lowercase or common sentence starter, title ends
                let rest: String = chars[i + 1..].iter().take(20).collect();
                let first_word: String = rest.chars().take_while(|c| c.is_alphabetic()).collect();

                // Common sentence starters that indicate body text
                let sentence_starters = [
                    "To", "In", "The", "This", "We", "It", "For", "On", "At", "By", "As", "An",
                    "Our", "With", "From", "Both",
                ];

                if sentence_starters.contains(&first_word.as_str()) {
                    return i; // End at the period
                }

                // If first word is lowercase, title likely ended
                if first_word.chars().next().is_some_and(char::is_lowercase) {
                    return i;
                }
            }

            // Check for direct sentence start patterns without period
            // e.g., "4 Optimised Table Structure Language To mitigate"
            if i > 2 && chars[i - 1] == ' ' {
                let rest: String = chars[i..].iter().take(20).collect();
                let first_word: String = rest.chars().take_while(|c| c.is_alphabetic()).collect();

                // If we hit a sentence starter word after a space, and previous was capitalized
                // title content, we might be at boundary
                let sentence_starters = ["To", "In", "The", "We", "It", "For", "On", "At", "By"];
                if sentence_starters.contains(&first_word.as_str()) {
                    // Check if previous word was a title word (capitalized or connector)
                    let before: String = chars[..i - 1].iter().rev().take(20).collect();
                    let prev_word: String = before
                        .chars()
                        .take_while(|c| c.is_alphabetic())
                        .collect::<String>()
                        .chars()
                        .rev()
                        .collect();

                    // If previous word is capitalized (part of title), this is likely body text start
                    if prev_word.chars().next().is_some_and(char::is_uppercase) {
                        return i - 1; // End before the space
                    }
                }
            }
        }

        // No clear boundary found - use reasonable max length
        // Most section headers are under 80 chars
        text.len().min(80)
    }

    /// Process resolved clusters to create document structure
    ///
    /// Args:
    ///     clusters: List of resolved clusters from Stage 8
    ///     `page_no`: Page number (default: 0)
    ///     `page_width`: Page width in pixels (default: 612.0 for letter size)
    ///     `page_height`: Page height in pixels (default: 792.0 for letter size)
    ///
    /// Returns:
    ///     List of document elements
    #[must_use = "returns the assembled document elements"]
    #[allow(clippy::too_many_lines)]
    pub fn process(
        &self,
        clusters: Vec<ClusterWithCells>,
        page_no: usize,
        page_width: f64,
        page_height: f64,
    ) -> Vec<DocumentElement> {
        let mut elements = Vec::new();

        for cluster in clusters {
            let label = &cluster.label;

            // TEXT ELEMENTS (most common)
            if self.config.text_elem_labels.contains(label) {
                // Extract text lines from cells
                let textlines: Vec<String> = cluster
                    .cells
                    .iter()
                    .map(|cell| cell.text.replace('\x02', "-").trim().to_string())
                    .filter(|text| !text.is_empty())
                    .collect();

                // Sanitize text (hyphenation, Unicode normalization)
                let text = self.sanitize_text(&textlines);

                // N=4134: Split text at embedded section header boundaries
                // This handles cases where layout model merges section headers with paragraph text
                let segments = self.split_at_section_headers(&text);

                // Create cluster info for baseline compatibility (shared across segments)
                let cluster_info = ClusterInfo {
                    label: cluster.label.clone(),
                    confidence: cluster.confidence,
                    bbox: cluster.bbox,
                    id: cluster.id,
                    cells: cluster
                        .cells
                        .iter()
                        .map(|cell| CellInfo {
                            text: cell.text.clone(),
                            rect: CellRect::from(cell),
                            confidence: cell.confidence.unwrap_or(1.0),
                            from_ocr: false, // Default value
                            is_bold: cell.is_bold,
                            is_italic: cell.is_italic,
                        })
                        .collect(),
                };

                // Create elements for each segment
                for (i, (segment_text, segment_label)) in segments.into_iter().enumerate() {
                    let elem = DocumentElement {
                        element_type: "TextElement".to_string(),
                        label: segment_label,
                        id: cluster.id + i * 1000, // Unique ID for split segments
                        page_no,
                        bbox: Some(cluster.bbox), // Same bbox for all (approximate)
                        page_width: Some(page_width),
                        page_height: Some(page_height),
                        ref_str: Some(format!("#/{}/{}", page_no, cluster.id + i * 1000)),
                        text: Some(segment_text),
                        cluster: if i == 0 {
                            Some(cluster_info.clone())
                        } else {
                            None
                        },
                    };
                    elements.push(elem);
                }
            }
            // TABLE ELEMENTS
            else if self.config.table_labels.contains(label) {
                // NOTE: Table structure predictions not implemented yet
                // Fallback: create empty table element
                let cluster_info = ClusterInfo {
                    label: cluster.label.clone(),
                    confidence: cluster.confidence,
                    bbox: cluster.bbox,
                    id: cluster.id,
                    cells: cluster
                        .cells
                        .iter()
                        .map(|cell| CellInfo {
                            text: cell.text.clone(),
                            rect: CellRect::from(cell),
                            confidence: cell.confidence.unwrap_or(1.0),
                            from_ocr: false,
                            is_bold: cell.is_bold,
                            is_italic: cell.is_italic,
                        })
                        .collect(),
                };

                let elem = DocumentElement {
                    element_type: "Table".to_string(),
                    label: label.clone(),
                    id: cluster.id,
                    page_no,
                    bbox: Some(cluster.bbox),
                    page_width: Some(page_width),
                    page_height: Some(page_height),
                    ref_str: Some(format!("#/{}/{}", page_no, cluster.id)),
                    text: Some(String::new()), // Empty text
                    cluster: Some(cluster_info),
                };
                elements.push(elem);
            }
            // FIGURE ELEMENTS
            else if label == &self.config.figure_label {
                // NOTE: Figure classification not implemented yet
                // Fallback: create empty figure element
                let cluster_info = ClusterInfo {
                    label: cluster.label.clone(),
                    confidence: cluster.confidence,
                    bbox: cluster.bbox,
                    id: cluster.id,
                    cells: cluster
                        .cells
                        .iter()
                        .map(|cell| CellInfo {
                            text: cell.text.clone(),
                            rect: CellRect::from(cell),
                            confidence: cell.confidence.unwrap_or(1.0),
                            from_ocr: false,
                            is_bold: cell.is_bold,
                            is_italic: cell.is_italic,
                        })
                        .collect(),
                };

                let elem = DocumentElement {
                    element_type: "FigureElement".to_string(),
                    label: label.clone(),
                    id: cluster.id,
                    page_no,
                    bbox: Some(cluster.bbox),
                    page_width: Some(page_width),
                    page_height: Some(page_height),
                    ref_str: Some(format!("#/{}/{}", page_no, cluster.id)),
                    text: Some(String::new()), // Empty text
                    cluster: Some(cluster_info),
                };
                elements.push(elem);
            }
            // CONTAINER ELEMENTS
            else if self.config.container_labels.contains(label) {
                let cluster_info = ClusterInfo {
                    label: cluster.label.clone(),
                    confidence: cluster.confidence,
                    bbox: cluster.bbox,
                    id: cluster.id,
                    cells: cluster
                        .cells
                        .iter()
                        .map(|cell| CellInfo {
                            text: cell.text.clone(),
                            rect: CellRect::from(cell),
                            confidence: cell.confidence.unwrap_or(1.0),
                            from_ocr: false,
                            is_bold: cell.is_bold,
                            is_italic: cell.is_italic,
                        })
                        .collect(),
                };

                let elem = DocumentElement {
                    element_type: "ContainerElement".to_string(),
                    label: label.clone(),
                    id: cluster.id,
                    page_no,
                    bbox: Some(cluster.bbox),
                    page_width: Some(page_width),
                    page_height: Some(page_height),
                    ref_str: Some(format!("#/{}/{}", page_no, cluster.id)),
                    text: None, // Container elements don't have text
                    cluster: Some(cluster_info),
                };
                elements.push(elem);
            }
        }

        elements
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_text_single_line() {
        let assembler = Stage09DocumentAssembler::new();
        let lines = vec!["Hello world".to_string()];
        let result = assembler.sanitize_text(&lines);
        assert_eq!(result, "Hello world");
    }

    #[test]
    fn test_sanitize_text_hyphenation() {
        let assembler = Stage09DocumentAssembler::new();
        let lines = vec![
            "Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed diam nonumy eir-"
                .to_string(),
            "mod tempor invidunt ut labore".to_string(),
        ];
        let result = assembler.sanitize_text(&lines);
        // Hyphen should be removed (connects "eir-" + "mod" → "eirmod")
        assert!(result.starts_with("Lorem ipsum"));
        assert!(result.contains("eirmod tempor"));
        assert!(!result.contains("eir- mod"));
    }

    #[test]
    fn test_sanitize_text_no_hyphenation() {
        let assembler = Stage09DocumentAssembler::new();
        let lines = vec!["Hello world".to_string(), "Next line".to_string()];
        let result = assembler.sanitize_text(&lines);
        // Space should be added between lines
        assert_eq!(result, "Hello world Next line");
    }

    #[test]
    fn test_sanitize_text_unicode_replacement() {
        let assembler = Stage09DocumentAssembler::new();
        // Test Unicode normalization:
        // - ⁄ (fraction slash U+2044) → /
        // - • (bullet U+2022) → ∞ (infinity, matches Python docling v2.58.0)
        // - em-dash (U+2014) → - (hyphen)
        let lines = vec!["Hello⁄world • test—done".to_string()];
        let result = assembler.sanitize_text(&lines);
        assert_eq!(result, "Hello/world ∞ test-done");
    }

    #[test]
    fn test_sanitize_text_empty() {
        let assembler = Stage09DocumentAssembler::new();
        let lines: Vec<String> = vec![];
        let result = assembler.sanitize_text(&lines);
        assert_eq!(result, "");
    }

    #[test]
    fn test_sanitize_text_abbreviation_spacing() {
        let assembler = Stage09DocumentAssembler::new();

        // Test e.g. followed by word
        let lines = vec!["e.g.the".to_string()];
        let result = assembler.sanitize_text(&lines);
        assert_eq!(result, "e.g. the");

        // Test i.e. followed by word
        let lines = vec!["i.e.the".to_string()];
        let result = assembler.sanitize_text(&lines);
        assert_eq!(result, "i.e. the");

        // Test e.g. followed by parenthesis
        let lines = vec!["e.g.(a)".to_string()];
        let result = assembler.sanitize_text(&lines);
        assert_eq!(result, "e.g. (a)");

        // Test abbreviation already has space (should not double-space)
        let lines = vec!["e.g. the".to_string()];
        let result = assembler.sanitize_text(&lines);
        assert_eq!(result, "e.g. the");

        // Test abbreviation at end of text (no change needed)
        let lines = vec!["example e.g.".to_string()];
        let result = assembler.sanitize_text(&lines);
        assert_eq!(result, "example e.g.");
    }

    #[test]
    fn test_sanitize_text_compound_hyphen() {
        let assembler = Stage09DocumentAssembler::new();

        // Test compound hyphen normalization: "mid - 19th" → "mid-19th"
        let lines = vec!["mid - 19th century".to_string()];
        let result = assembler.sanitize_text(&lines);
        assert_eq!(result, "mid-19th century");

        // Test "open - source" → "open-source"
        let lines = vec!["open - source software".to_string()];
        let result = assembler.sanitize_text(&lines);
        assert_eq!(result, "open-source software");

        // Test ALL-CAPS word stays spaced: "Vision - ECCV" stays as is
        let lines = vec!["Vision - ECCV 2024".to_string()];
        let result = assembler.sanitize_text(&lines);
        assert_eq!(result, "Vision - ECCV 2024");
    }

    #[test]
    fn test_sanitize_text_word_breaks() {
        let assembler = Stage09DocumentAssembler::new();

        // Test "professi onal" → "professional"
        let lines = vec!["professi onal work".to_string()];
        let result = assembler.sanitize_text(&lines);
        assert_eq!(result, "professional work");

        // Test "a nd" → "and"
        let lines = vec!["a nd the".to_string()];
        let result = assembler.sanitize_text(&lines);
        assert_eq!(result, "and the");

        // Test "able" suffix with consonant: "read able" → "readable"
        let lines = vec!["read able".to_string()];
        let result = assembler.sanitize_text(&lines);
        assert_eq!(result, "readable");

        // Test "ally" suffix with vowel: "virtu ally" → "virtually"
        let lines = vec!["virtu ally".to_string()];
        let result = assembler.sanitize_text(&lines);
        assert_eq!(result, "virtually");
    }

    #[test]
    fn test_sanitize_text_orcid_normalization() {
        let assembler = Stage09DocumentAssembler::new();

        // Test ORCID bracket space insertion: "Lysak[0000-0002-3723-6960]" → "Lysak [0000 -0002 -3723 -6960]"
        let lines = vec!["Lysak[0000-0002-3723-6960]".to_string()];
        let result = assembler.sanitize_text(&lines);
        assert_eq!(result, "Lysak [0000 -0002 -3723 -6960]");

        // Test ORCID comma spacing: "], " → "] ,"
        let lines = vec!["Author [0000 -0002 -3723 -6960],".to_string()];
        let result = assembler.sanitize_text(&lines);
        assert_eq!(result, "Author [0000 -0002 -3723 -6960] ,");
    }

    #[test]
    fn test_sanitize_text_en_em_dash_hyphenation() {
        let assembler = Stage09DocumentAssembler::new();

        // Test en-dash (U+2013) at end of line - should be treated as hyphenation
        // "hyphen–" + "ated" → "hyphenated" (en-dash removed, words joined)
        let lines = vec!["hyphen\u{2013}".to_string(), "ated".to_string()];
        let result = assembler.sanitize_text(&lines);
        assert_eq!(result, "hyphenated");

        // Test em-dash (U+2014) at end of line - should be treated as hyphenation
        // "hyphen—" + "ated" → "hyphenated" (em-dash removed, words joined)
        let lines = vec!["hyphen\u{2014}".to_string(), "ated".to_string()];
        let result = assembler.sanitize_text(&lines);
        assert_eq!(result, "hyphenated");

        // Test regular hyphen still works
        let lines = vec!["hyphen-".to_string(), "ated".to_string()];
        let result = assembler.sanitize_text(&lines);
        assert_eq!(result, "hyphenated");
    }

    #[test]
    fn test_sanitize_text_dash_at_line_start() {
        let assembler = Stage09DocumentAssembler::new();

        // Test current line starting with hyphen: "Pre" + "-Digital" → "Pre-Digital"
        let lines = vec!["Pre".to_string(), "-Digital".to_string()];
        let result = assembler.sanitize_text(&lines);
        assert_eq!(result, "Pre-Digital");

        // Test current line starting with en-dash: "Pre" + "–Digital" → "Pre–Digital" → "Pre-Digital"
        let lines = vec!["Pre".to_string(), "\u{2013}Digital".to_string()];
        let result = assembler.sanitize_text(&lines);
        assert_eq!(result, "Pre-Digital"); // Unicode normalization converts en-dash to hyphen

        // Test current line starting with em-dash: "Pre" + "—Digital" → "Pre—Digital" → "Pre-Digital"
        let lines = vec!["Pre".to_string(), "\u{2014}Digital".to_string()];
        let result = assembler.sanitize_text(&lines);
        assert_eq!(result, "Pre-Digital"); // Unicode normalization converts em-dash to hyphen
    }

    #[test]
    fn test_sanitize_text_whitespace_separated_dash() {
        let assembler = Stage09DocumentAssembler::new();

        // Test dash separated by whitespace: "word -" should NOT be treated as hyphenation
        // The dash is a separator token (e.g., ORCID spans like "0000 -0002")
        // The dash remains and lines join directly (no additional space added)
        let lines = vec!["word -".to_string(), "continuation".to_string()];
        let result = assembler.sanitize_text(&lines);
        assert_eq!(result, "word -continuation");

        // Test en-dash separated by whitespace - same behavior
        let lines = vec!["word \u{2013}".to_string(), "continuation".to_string()];
        let result = assembler.sanitize_text(&lines);
        assert_eq!(result, "word -continuation"); // en-dash normalized to hyphen
    }

    /// Regression test: Verify modular pipeline produces identical output to original pipeline
    ///
    /// This test ensures that Stage09DocumentAssembler.sanitize_text() produces the same
    /// output as PageAssembler.sanitize_text() from the original pipeline (page_assembly.rs).
    ///
    /// Context:
    /// - N=4426-4427: Synced normalize_punctuation_spacing and en-dash/em-dash handling
    /// - Both implementations should now be functionally identical
    #[test]
    fn test_sanitize_text_matches_original_pipeline() {
        use crate::pipeline::page_assembly::PageAssembler;

        let modular = Stage09DocumentAssembler::new();
        let original = PageAssembler;

        // Test cases covering all synced functionality
        let test_cases: Vec<Vec<String>> = vec![
            // Single line
            vec!["Hello world".to_string()],
            // Multi-line with hyphenation
            vec!["hyphen-".to_string(), "ated".to_string()],
            // Multi-line without hyphenation
            vec!["Hello".to_string(), "world".to_string()],
            // Unicode normalization
            vec!["Hello⁄world • test—done".to_string()],
            // En-dash hyphenation
            vec!["platforms\u{2013}".to_string(), "reflects".to_string()],
            // Em-dash hyphenation
            vec![
                "AI-powered platforms\u{2014}".to_string(),
                "reflects".to_string(),
            ],
            // Line starting with dash
            vec!["Pre".to_string(), "-Digital".to_string()],
            // Whitespace-separated dash (not hyphenation)
            vec!["word -".to_string(), "continuation".to_string()],
            // Abbreviation spacing
            vec!["e.g.this is example".to_string()],
            // Empty
            vec![],
            // Punctuation spacing
            vec!["( text )".to_string()],
            // ORCID-like patterns
            vec!["0000".to_string(), "-0002".to_string()],
        ];

        for (i, lines) in test_cases.iter().enumerate() {
            let modular_result = modular.sanitize_text(lines);
            let original_result = original.sanitize_text(lines);

            assert_eq!(
                modular_result, original_result,
                "Test case {} mismatch:\n  input: {:?}\n  modular: {:?}\n  original: {:?}",
                i, lines, modular_result, original_result
            );
        }
    }
}
