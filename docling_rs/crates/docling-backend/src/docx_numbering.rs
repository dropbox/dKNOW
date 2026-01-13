//! DOCX Numbering Definitions Parser
//!
//! Parses `word/numbering.xml` to extract list numbering format information.
//!
//! ## Python Reference
//! `~/docling/docling/backend/msword_backend.py:387-470`
//!
//! ## XML Structure
//! ```xml
//! <w:numbering>
//!   <w:abstractNum w:abstractNumId="0">
//!     <w:lvl w:ilvl="0">
//!       <w:numFmt w:val="decimal"/>  <!-- 1, 2, 3 -->
//!       <w:start w:val="1"/>
//!     </w:lvl>
//!   </w:abstractNum>
//!   <w:num w:numId="1">
//!     <w:abstractNumId w:val="0"/>
//!   </w:num>
//! </w:numbering>
//! ```

// Clippy pedantic allows:
// - XML numbering parsing is necessarily complex
#![allow(clippy::too_many_lines)]

use quick_xml::events::Event;
use quick_xml::Reader;
use std::collections::HashMap;
use std::io::{Read as IoRead, Seek};
use zip::ZipArchive;

/// Numbering format types
///
/// Maps to `<w:numFmt w:val="..."/>` values
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum NumFormat {
    /// Decimal: 1, 2, 3
    Decimal,
    /// Lower Roman: i, ii, iii
    LowerRoman,
    /// Upper Roman: I, II, III
    UpperRoman,
    /// Lower Letter: a, b, c
    LowerLetter,
    /// Upper Letter: A, B, C
    UpperLetter,
    /// Decimal with leading zero: 01, 02, 03
    DecimalZero,
    /// Bullet (non-numbered) - default for unspecified lists
    #[default]
    Bullet,
}

impl NumFormat {
    /// Parse from XML w:val attribute
    ///
    /// Python reference: msword_backend.py:459-466
    #[inline]
    #[must_use = "returns the parsed number format"]
    pub fn parse_format(s: &str) -> Self {
        match s {
            "decimal" => Self::Decimal,
            "lowerRoman" => Self::LowerRoman,
            "upperRoman" => Self::UpperRoman,
            "lowerLetter" => Self::LowerLetter,
            "upperLetter" => Self::UpperLetter,
            "decimalZero" => Self::DecimalZero,
            _ => Self::Bullet,
        }
    }

    /// Check if this format is numbered (not bullet)
    #[inline]
    #[must_use = "returns whether this format uses numbers instead of bullets"]
    pub const fn is_numbered(&self) -> bool {
        !matches!(self, Self::Bullet)
    }
}

impl std::fmt::Display for NumFormat {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Decimal => write!(f, "decimal"),
            Self::LowerRoman => write!(f, "lower_roman"),
            Self::UpperRoman => write!(f, "upper_roman"),
            Self::LowerLetter => write!(f, "lower_letter"),
            Self::UpperLetter => write!(f, "upper_letter"),
            Self::DecimalZero => write!(f, "decimal_zero"),
            Self::Bullet => write!(f, "bullet"),
        }
    }
}

impl std::str::FromStr for NumFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Normalize: lowercase and remove underscores
        let normalized: String = s.to_lowercase().chars().filter(|c| *c != '_').collect();
        match normalized.as_str() {
            // Display format (snake_case)
            "decimal" => Ok(Self::Decimal),
            "lowerroman" | "roman" => Ok(Self::LowerRoman),
            "upperroman" => Ok(Self::UpperRoman),
            "lowerletter" | "letter" => Ok(Self::LowerLetter),
            "upperletter" => Ok(Self::UpperLetter),
            "decimalzero" => Ok(Self::DecimalZero),
            "bullet" | "none" => Ok(Self::Bullet),
            _ => Err(format!(
                "unknown number format: '{s}' (expected: decimal, lower_roman, upper_roman, \
                lower_letter, upper_letter, decimal_zero, bullet)"
            )),
        }
    }
}

/// Level definition within abstract numbering
///
/// Corresponds to `<w:lvl w:ilvl="0">` in numbering.xml
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct LevelDefinition {
    /// Indentation level (0, 1, 2, ...)
    pub ilvl: i32,
    /// Number format (decimal, roman, letter, bullet)
    pub num_fmt: NumFormat,
    /// Starting value (usually 1)
    pub start_val: i32,
    /// Level text pattern (e.g., "%1.%2." for hierarchical numbering)
    /// Python reference: msword_backend.py:459-466
    pub lvl_text: Option<String>,
}

/// Abstract numbering definition
///
/// Corresponds to `<w:abstractNum w:abstractNumId="...">` in numbering.xml
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AbstractNum {
    /// Abstract numbering ID
    pub abstract_num_id: i32,
    /// Map ilvl → `LevelDefinition`
    pub levels: HashMap<i32, LevelDefinition>,
}

/// All numbering definitions from numbering.xml
///
/// Python reference: msword_backend.py:387-470
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NumberingDefinitions {
    /// Map numId → abstractNumId
    num_map: HashMap<i32, i32>,
    /// Map abstractNumId → `AbstractNum`
    abstract_nums: HashMap<i32, AbstractNum>,
}

impl NumberingDefinitions {
    /// Create empty numbering definitions
    #[inline]
    #[must_use = "creates an empty numbering definitions instance"]
    pub fn empty() -> Self {
        Self {
            num_map: HashMap::new(),
            abstract_nums: HashMap::new(),
        }
    }

    /// Get level definition for numId and ilvl
    ///
    /// Python reference: msword_backend.py:410-455
    #[inline]
    #[must_use = "returns the level definition if found"]
    pub fn get_level(&self, num_id: i32, ilvl: i32) -> Option<&LevelDefinition> {
        // numId → abstractNumId
        let abstract_num_id = self.num_map.get(&num_id)?;
        // abstractNumId → AbstractNum
        let abstract_num = self.abstract_nums.get(abstract_num_id)?;
        // ilvl → LevelDefinition
        abstract_num.levels.get(&ilvl)
    }

    /// Check if list is numbered (not bullet)
    ///
    /// Python reference: msword_backend.py:387-472 (_`is_numbered_list`)
    #[inline]
    #[must_use = "returns whether this list uses numbers instead of bullets"]
    pub fn is_numbered(&self, num_id: i32, ilvl: i32) -> bool {
        self.get_level(num_id, ilvl)
            .is_some_and(|level| level.num_fmt.is_numbered())
    }
}

/// Extract attribute value as i32 by key from XML element
#[inline]
fn get_attr_i32(e: &quick_xml::events::BytesStart<'_>, key: &[u8]) -> Option<i32> {
    e.attributes()
        .flatten()
        .find(|a| a.key.as_ref() == key)
        .and_then(|a| std::str::from_utf8(&a.value).ok()?.parse::<i32>().ok())
}

/// Extract attribute value as String by key from XML element
#[inline]
fn get_attr_string(e: &quick_xml::events::BytesStart<'_>, key: &[u8]) -> Option<String> {
    e.attributes()
        .flatten()
        .find(|a| a.key.as_ref() == key)
        .and_then(|a| std::str::from_utf8(&a.value).ok().map(str::to_string))
}

/// Parse numbering.xml from DOCX archive
///
/// Python reference: msword_backend.py:404-455
///
/// ## XML Structure
/// ```xml
/// <w:numbering xmlns:w="...">
///   <w:abstractNum w:abstractNumId="0">
///     <w:lvl w:ilvl="0">
///       <w:start w:val="1"/>
///       <w:numFmt w:val="decimal"/>
///     </w:lvl>
///     <w:lvl w:ilvl="1">
///       <w:numFmt w:val="lowerLetter"/>
///     </w:lvl>
///   </w:abstractNum>
///   <w:num w:numId="1">
///     <w:abstractNumId w:val="0"/>
///   </w:num>
/// </w:numbering>
/// ```
///
/// # Errors
///
/// Returns an error if the XML parsing fails or if the file cannot be read.
#[must_use = "this function returns numbering definitions that should be used for list formatting"]
pub fn parse_numbering_xml<R: IoRead + Seek>(
    zip_archive: &mut ZipArchive<R>,
) -> Result<NumberingDefinitions, Box<dyn std::error::Error>> {
    // Try to open word/numbering.xml
    let Ok(mut xml_file) = zip_archive.by_name("word/numbering.xml") else {
        // numbering.xml is optional - document may not have lists
        return Ok(NumberingDefinitions::empty());
    };

    let mut xml_content = String::new();
    xml_file.read_to_string(&mut xml_content)?;
    drop(xml_file); // Release borrow

    let mut reader = Reader::from_str(&xml_content);
    reader.trim_text(true);

    let mut num_map: HashMap<i32, i32> = HashMap::new();
    let mut abstract_nums: HashMap<i32, AbstractNum> = HashMap::new();

    // State tracking
    let mut current_abstract_num_id: Option<i32> = None;
    let mut current_abstract_num: Option<AbstractNum> = None;
    let mut current_ilvl: Option<i32> = None;
    let mut current_level: Option<LevelDefinition> = None;

    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e) | Event::Empty(ref e)) => {
                let tag_name = e.name();

                match tag_name.as_ref() {
                    b"w:abstractNum" => {
                        // Start abstract numbering definition
                        // <w:abstractNum w:abstractNumId="0">
                        if let Some(id) = get_attr_i32(e, b"w:abstractNumId") {
                            current_abstract_num_id = Some(id);
                            current_abstract_num = Some(AbstractNum {
                                abstract_num_id: id,
                                levels: HashMap::new(),
                            });
                        }
                    }
                    b"w:num" => {
                        // Numbering instance: <w:num w:numId="1">
                        // Store numId temporarily, will link to abstractNumId when we see it
                        current_ilvl = get_attr_i32(e, b"w:numId");
                    }
                    b"w:abstractNumId" => {
                        // Link numId → abstractNumId
                        // <w:abstractNumId w:val="0"/>
                        if let (Some(num_id), Some(abstract_num_id)) =
                            (current_ilvl.take(), get_attr_i32(e, b"w:val"))
                        {
                            num_map.insert(num_id, abstract_num_id);
                        }
                    }
                    b"w:lvl" => {
                        // Level definition: <w:lvl w:ilvl="0">
                        if let Some(ilvl) = get_attr_i32(e, b"w:ilvl") {
                            current_ilvl = Some(ilvl);
                            current_level = Some(LevelDefinition {
                                ilvl,
                                num_fmt: NumFormat::Bullet, // Default
                                start_val: 1,               // Default
                                lvl_text: None,             // Will be set from w:lvlText
                            });
                        }
                    }
                    b"w:lvlText" => {
                        // Level text pattern: <w:lvlText w:val="%1.%2."/>
                        if let Some(ref mut level) = current_level {
                            level.lvl_text = get_attr_string(e, b"w:val");
                        }
                    }
                    b"w:start" => {
                        // Starting value: <w:start w:val="1"/>
                        if let Some(ref mut level) = current_level {
                            if let Some(start) = get_attr_i32(e, b"w:val") {
                                level.start_val = start;
                            }
                        }
                    }
                    b"w:numFmt" => {
                        // Number format: <w:numFmt w:val="decimal"/>
                        if let Some(ref mut level) = current_level {
                            if let Some(val) = get_attr_string(e, b"w:val") {
                                level.num_fmt = NumFormat::parse_format(&val);
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                let tag_name = e.name();
                match tag_name.as_ref() {
                    b"w:lvl" => {
                        // Finish level definition
                        if let (Some(ref mut abstract_num), Some(level)) =
                            (current_abstract_num.as_mut(), current_level.take())
                        {
                            abstract_num.levels.insert(level.ilvl, level);
                        }
                        current_ilvl = None;
                    }
                    b"w:abstractNum" => {
                        // Finish abstract numbering definition
                        if let (Some(id), Some(abstract_num)) =
                            (current_abstract_num_id.take(), current_abstract_num.take())
                        {
                            abstract_nums.insert(id, abstract_num);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(format!("Error parsing numbering.xml: {e}").into());
            }
            _ => {}
        }

        buf.clear();
    }

    Ok(NumberingDefinitions {
        num_map,
        abstract_nums,
    })
}

/// List counter tracker
///
/// Tracks counters per (numId, ilvl) pair for numbered lists.
///
/// Python reference: msword_backend.py:372-386
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ListCounters {
    /// Map (numId, ilvl) → current counter value
    counters: HashMap<(i32, i32), i32>,
}

impl ListCounters {
    /// Create new counter tracker
    #[inline]
    #[must_use = "creates a new counter tracker instance"]
    pub fn new() -> Self {
        Self {
            counters: HashMap::new(),
        }
    }

    /// Reset all counters for a specific numId
    ///
    /// Python: `_reset_list_counters_for_new_sequence` (line 380)
    pub fn reset_for_sequence(&mut self, num_id: i32) {
        self.counters.retain(|(nid, _), _| *nid != num_id);
    }

    /// Reset counters for levels deeper than the given ilvl
    /// When we encounter ilvl=1 after ilvl=2, we reset ilvl=2 counter
    pub fn reset_deeper_levels(&mut self, num_id: i32, ilvl: i32) {
        self.counters
            .retain(|(nid, lvl), _| *nid != num_id || *lvl <= ilvl);
    }

    /// Get counter value and increment
    ///
    /// Python: `_get_list_counter` (line 372)
    /// Returns current counter, then increments for next call
    #[inline]
    #[must_use = "returns the counter value before incrementing"]
    pub fn get_and_increment(&mut self, num_id: i32, ilvl: i32) -> i32 {
        let counter = self.counters.entry((num_id, ilvl)).or_insert(0);
        *counter += 1;
        *counter
    }

    /// Get current counter value without incrementing
    #[inline]
    #[must_use = "returns the current counter value"]
    pub fn get_current(&self, num_id: i32, ilvl: i32) -> i32 {
        *self.counters.get(&(num_id, ilvl)).unwrap_or(&0)
    }
}

/// Generate list marker string and enumerated flag
///
/// Python reference: msword_backend.py:1172-1175, 1195-1197
///
/// Returns (marker, enumerated):
/// - Numbered lists: ("1.", true), ("1.1", true), ("i.", true), etc.
/// - Bullet lists: ("", false)
#[must_use = "returns the list marker and enumeration status"]
pub fn generate_marker(
    numbering: &NumberingDefinitions,
    counters: &mut ListCounters,
    num_id: i32,
    ilvl: i32,
) -> (String, bool) {
    let Some(level_def) = numbering.get_level(num_id, ilvl) else {
        return (String::new(), false); // Unknown format, treat as bullet
    };

    if level_def.num_fmt.is_numbered() {
        // Reset deeper levels when moving to a shallower level
        counters.reset_deeper_levels(num_id, ilvl);

        // Initialize any skipped intermediate levels to 1
        // This handles cases like jumping from ilvl=0 to ilvl=2:
        // levels 1 gets initialized to 1, so "%1.%2.%3" becomes "2.1.1" not "2.0.1"
        for intermediate_level in 0..ilvl {
            if counters.get_current(num_id, intermediate_level) == 0 {
                // Initialize intermediate level - value not needed, just the side effect
                let _ = counters.get_and_increment(num_id, intermediate_level);
            }
        }

        // Increment counter for this level
        let counter = counters.get_and_increment(num_id, ilvl);

        // Check if we have a lvlText pattern for hierarchical numbering
        level_def.lvl_text.as_ref().map_or_else(
            || {
                // Simple marker without lvlText
                let marker = match level_def.num_fmt {
                    NumFormat::Decimal => format!("{counter}."),
                    NumFormat::LowerRoman => format!("{}.", to_lower_roman(counter)),
                    NumFormat::UpperRoman => format!("{}.", to_upper_roman(counter)),
                    NumFormat::LowerLetter => format!("{}.", to_lower_letter(counter)),
                    NumFormat::UpperLetter => format!("{}.", to_upper_letter(counter)),
                    NumFormat::DecimalZero => format!("{counter:02}."),
                    NumFormat::Bullet => String::new(),
                };
                (marker, true) // enumerated = true
            },
            |lvl_text| {
                // Format using lvlText pattern (e.g., "%1.%2." → "1.1.")
                let marker = format_lvl_text(lvl_text, numbering, counters, num_id, ilvl, counter);
                // Keep the period for list items - docx.rs strips it for headings only
                (marker, true)
            },
        )
    } else {
        // Bullet list - marker is empty in DocItem
        // (serializer adds "- " or "* " when rendering markdown)
        (String::new(), false) // enumerated = false
    }
}

/// Format hierarchical marker using lvlText pattern
///
/// Pattern like "%1.%2." means: `level0_counter.level1_counter`.
/// %1 = counter at ilvl 0, %2 = counter at ilvl 1, etc.
fn format_lvl_text(
    lvl_text: &str,
    numbering: &NumberingDefinitions,
    counters: &ListCounters,
    num_id: i32,
    current_ilvl: i32,
    current_counter: i32,
) -> String {
    let mut result = lvl_text.to_string();

    // Replace %1, %2, %3, etc. with actual counter values
    for level in 0..=current_ilvl {
        let placeholder = format!("%{}", level + 1);
        if result.contains(&placeholder) {
            let counter_val = if level == current_ilvl {
                current_counter
            } else {
                counters.get_current(num_id, level)
            };

            // Get format for this level
            let formatted = numbering.get_level(num_id, level).map_or_else(
                || counter_val.to_string(),
                |level_def| match level_def.num_fmt {
                    NumFormat::LowerRoman => to_lower_roman(counter_val),
                    NumFormat::UpperRoman => to_upper_roman(counter_val),
                    NumFormat::LowerLetter => to_lower_letter(counter_val),
                    NumFormat::UpperLetter => to_upper_letter(counter_val),
                    NumFormat::DecimalZero => format!("{counter_val:02}"),
                    _ => counter_val.to_string(), // Decimal and others
                },
            );

            result = result.replace(&placeholder, &formatted);
        }
    }

    result
}

/// Convert number to lowercase Roman numerals
#[inline]
fn to_lower_roman(n: i32) -> String {
    to_roman(n).to_lowercase()
}

/// Convert number to uppercase Roman numerals
#[inline]
fn to_upper_roman(n: i32) -> String {
    to_roman(n)
}

/// Convert number to Roman numerals (uppercase)
#[inline]
fn to_roman(mut n: i32) -> String {
    if n <= 0 {
        return String::new();
    }

    let values = [1000, 900, 500, 400, 100, 90, 50, 40, 10, 9, 5, 4, 1];
    let numerals = [
        "M", "CM", "D", "CD", "C", "XC", "L", "XL", "X", "IX", "V", "IV", "I",
    ];

    let mut result = String::new();
    for (i, &value) in values.iter().enumerate() {
        while n >= value {
            result.push_str(numerals[i]);
            n -= value;
        }
    }
    result
}

/// Convert number to lowercase letter (a, b, c, ..., z, aa, ab, ...)
#[inline]
fn to_lower_letter(n: i32) -> String {
    to_letter(n, 'a')
}

/// Convert number to uppercase letter (A, B, C, ..., Z, AA, AB, ...)
#[inline]
fn to_upper_letter(n: i32) -> String {
    to_letter(n, 'A')
}

/// Convert number to letter sequence
#[inline]
fn to_letter(mut n: i32, base: char) -> String {
    if n <= 0 {
        return String::new();
    }

    let mut result = String::new();
    while n > 0 {
        n -= 1; // Convert to 0-based
                // n % 26 is always 0-25 when n > 0, so cast to u8 is safe
        #[allow(clippy::cast_sign_loss)]
        let c = ((n % 26) as u8 + base as u8) as char;
        result.insert(0, c);
        n /= 26;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_num_format_from_str() {
        assert_eq!(NumFormat::parse_format("decimal"), NumFormat::Decimal);
        assert_eq!(NumFormat::parse_format("lowerRoman"), NumFormat::LowerRoman);
        assert_eq!(NumFormat::parse_format("upperRoman"), NumFormat::UpperRoman);
        assert_eq!(
            NumFormat::parse_format("lowerLetter"),
            NumFormat::LowerLetter
        );
        assert_eq!(
            NumFormat::parse_format("upperLetter"),
            NumFormat::UpperLetter
        );
        assert_eq!(
            NumFormat::parse_format("decimalZero"),
            NumFormat::DecimalZero
        );
        assert_eq!(NumFormat::parse_format("bullet"), NumFormat::Bullet);
        assert_eq!(NumFormat::parse_format("unknown"), NumFormat::Bullet);
    }

    #[test]
    fn test_num_format_is_numbered() {
        assert!(NumFormat::Decimal.is_numbered());
        assert!(NumFormat::LowerRoman.is_numbered());
        assert!(NumFormat::UpperRoman.is_numbered());
        assert!(NumFormat::LowerLetter.is_numbered());
        assert!(NumFormat::UpperLetter.is_numbered());
        assert!(NumFormat::DecimalZero.is_numbered());
        assert!(!NumFormat::Bullet.is_numbered());
    }

    #[test]
    fn test_num_format_from_str_trait() {
        use std::str::FromStr;

        // Display format (snake_case)
        assert_eq!(NumFormat::from_str("decimal").unwrap(), NumFormat::Decimal);
        assert_eq!(
            NumFormat::from_str("lower_roman").unwrap(),
            NumFormat::LowerRoman
        );
        assert_eq!(
            NumFormat::from_str("upper_roman").unwrap(),
            NumFormat::UpperRoman
        );
        assert_eq!(
            NumFormat::from_str("lower_letter").unwrap(),
            NumFormat::LowerLetter
        );
        assert_eq!(
            NumFormat::from_str("upper_letter").unwrap(),
            NumFormat::UpperLetter
        );
        assert_eq!(
            NumFormat::from_str("decimal_zero").unwrap(),
            NumFormat::DecimalZero
        );
        assert_eq!(NumFormat::from_str("bullet").unwrap(), NumFormat::Bullet);

        // Aliases
        assert_eq!(NumFormat::from_str("roman").unwrap(), NumFormat::LowerRoman);
        assert_eq!(
            NumFormat::from_str("letter").unwrap(),
            NumFormat::LowerLetter
        );
        assert_eq!(NumFormat::from_str("none").unwrap(), NumFormat::Bullet);

        // Case insensitive
        assert_eq!(NumFormat::from_str("DECIMAL").unwrap(), NumFormat::Decimal);
        assert_eq!(
            NumFormat::from_str("LowerRoman").unwrap(),
            NumFormat::LowerRoman
        );

        // Error case
        assert!(NumFormat::from_str("unknown_format").is_err());
    }

    #[test]
    fn test_num_format_roundtrip() {
        use std::str::FromStr;

        for fmt in [
            NumFormat::Decimal,
            NumFormat::LowerRoman,
            NumFormat::UpperRoman,
            NumFormat::LowerLetter,
            NumFormat::UpperLetter,
            NumFormat::DecimalZero,
            NumFormat::Bullet,
        ] {
            let s = fmt.to_string();
            let parsed = NumFormat::from_str(&s).unwrap();
            assert_eq!(fmt, parsed, "roundtrip failed for {s}");
        }
    }

    #[test]
    fn test_empty_numbering_definitions() {
        let defs = NumberingDefinitions::empty();
        assert!(defs.get_level(1, 0).is_none());
        assert!(!defs.is_numbered(1, 0));
    }

    #[test]
    fn test_counter_management() {
        let mut counters = ListCounters::new();

        // First call should return 1
        assert_eq!(counters.get_and_increment(1, 0), 1);
        // Second call should return 2
        assert_eq!(counters.get_and_increment(1, 0), 2);
        // Third call should return 3
        assert_eq!(counters.get_and_increment(1, 0), 3);

        // Different ilvl should have separate counter
        assert_eq!(counters.get_and_increment(1, 1), 1);
        assert_eq!(counters.get_and_increment(1, 1), 2);

        // Different numId should have separate counter
        assert_eq!(counters.get_and_increment(2, 0), 1);

        // Reset sequence for numId=1
        counters.reset_for_sequence(1);
        assert_eq!(counters.get_and_increment(1, 0), 1); // Back to 1
        assert_eq!(counters.get_and_increment(1, 1), 1); // Also reset

        // numId=2 should be unaffected
        assert_eq!(counters.get_and_increment(2, 0), 2);
    }

    #[test]
    fn test_to_roman() {
        assert_eq!(to_roman(1), "I");
        assert_eq!(to_roman(2), "II");
        assert_eq!(to_roman(3), "III");
        assert_eq!(to_roman(4), "IV");
        assert_eq!(to_roman(5), "V");
        assert_eq!(to_roman(9), "IX");
        assert_eq!(to_roman(10), "X");
        assert_eq!(to_roman(40), "XL");
        assert_eq!(to_roman(50), "L");
        assert_eq!(to_roman(90), "XC");
        assert_eq!(to_roman(100), "C");
        assert_eq!(to_roman(400), "CD");
        assert_eq!(to_roman(500), "D");
        assert_eq!(to_roman(900), "CM");
        assert_eq!(to_roman(1000), "M");
        assert_eq!(to_roman(1994), "MCMXCIV");
    }

    #[test]
    fn test_to_lower_roman() {
        assert_eq!(to_lower_roman(1), "i");
        assert_eq!(to_lower_roman(2), "ii");
        assert_eq!(to_lower_roman(3), "iii");
        assert_eq!(to_lower_roman(4), "iv");
    }

    #[test]
    fn test_to_letter() {
        assert_eq!(to_lower_letter(1), "a");
        assert_eq!(to_lower_letter(2), "b");
        assert_eq!(to_lower_letter(26), "z");
        assert_eq!(to_lower_letter(27), "aa");
        assert_eq!(to_lower_letter(28), "ab");

        assert_eq!(to_upper_letter(1), "A");
        assert_eq!(to_upper_letter(2), "B");
        assert_eq!(to_upper_letter(26), "Z");
        assert_eq!(to_upper_letter(27), "AA");
    }

    #[test]
    fn test_generate_marker_decimal() {
        let mut defs = NumberingDefinitions::empty();
        let mut abstract_num = AbstractNum {
            abstract_num_id: 0,
            levels: HashMap::new(),
        };
        abstract_num.levels.insert(
            0,
            LevelDefinition {
                ilvl: 0,
                num_fmt: NumFormat::Decimal,
                start_val: 1,
                lvl_text: None,
            },
        );
        defs.abstract_nums.insert(0, abstract_num);
        defs.num_map.insert(1, 0);

        let mut counters = ListCounters::new();

        let (marker1, enum1) = generate_marker(&defs, &mut counters, 1, 0);
        assert_eq!(marker1, "1.");
        assert!(enum1);

        let (marker2, enum2) = generate_marker(&defs, &mut counters, 1, 0);
        assert_eq!(marker2, "2.");
        assert!(enum2);
    }

    #[test]
    fn test_generate_marker_bullet() {
        let mut defs = NumberingDefinitions::empty();
        let mut abstract_num = AbstractNum {
            abstract_num_id: 0,
            levels: HashMap::new(),
        };
        abstract_num.levels.insert(
            0,
            LevelDefinition {
                ilvl: 0,
                num_fmt: NumFormat::Bullet,
                start_val: 1,
                lvl_text: None,
            },
        );
        defs.abstract_nums.insert(0, abstract_num);
        defs.num_map.insert(20, 0);

        let mut counters = ListCounters::new();

        let (marker, enumerated) = generate_marker(&defs, &mut counters, 20, 0);
        assert_eq!(marker, "");
        assert!(!enumerated);
    }

    #[test]
    fn test_generate_marker_roman() {
        let mut defs = NumberingDefinitions::empty();
        let mut abstract_num = AbstractNum {
            abstract_num_id: 0,
            levels: HashMap::new(),
        };
        abstract_num.levels.insert(
            0,
            LevelDefinition {
                ilvl: 0,
                num_fmt: NumFormat::LowerRoman,
                start_val: 1,
                lvl_text: None,
            },
        );
        defs.abstract_nums.insert(0, abstract_num);
        defs.num_map.insert(1, 0);

        let mut counters = ListCounters::new();

        let (marker1, _) = generate_marker(&defs, &mut counters, 1, 0);
        assert_eq!(marker1, "i.");

        let (marker2, _) = generate_marker(&defs, &mut counters, 1, 0);
        assert_eq!(marker2, "ii.");
    }

    #[test]
    fn test_num_format_display() {
        assert_eq!(format!("{}", NumFormat::Decimal), "decimal");
        assert_eq!(format!("{}", NumFormat::LowerRoman), "lower_roman");
        assert_eq!(format!("{}", NumFormat::UpperRoman), "upper_roman");
        assert_eq!(format!("{}", NumFormat::LowerLetter), "lower_letter");
        assert_eq!(format!("{}", NumFormat::UpperLetter), "upper_letter");
        assert_eq!(format!("{}", NumFormat::DecimalZero), "decimal_zero");
        assert_eq!(format!("{}", NumFormat::Bullet), "bullet");
    }

    #[test]
    fn test_list_counters_default() {
        let default = ListCounters::default();
        let new = ListCounters::new();
        assert_eq!(default, new);
    }
}
