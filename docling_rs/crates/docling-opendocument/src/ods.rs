//! `OpenDocument` Spreadsheet (ODS) format parser
//!
//! Parses .ods files (`OpenDocument` Spreadsheet format used by `LibreOffice` Calc).
//! Uses the `calamine` crate for robust ODS parsing.

use crate::error::Result;
use calamine::{open_workbook, Data, Ods, Range, Reader};
use std::fmt::Write;
use std::path::Path;

/// Represents a single spreadsheet sheet with structured data
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct OdsSheet {
    /// Sheet name
    pub name: String,
    /// Table data as rows of cells
    /// Each row is a vector of cell texts
    pub rows: Vec<Vec<String>>,
    /// Number of rows
    pub row_count: usize,
    /// Number of columns
    pub col_count: usize,
}

/// Parsed ODS spreadsheet content
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct OdsDocument {
    /// Spreadsheet text content (all cells concatenated)
    pub text: String,
    /// Sheet names
    pub sheet_names: Vec<String>,
    /// Number of sheets
    pub sheet_count: usize,
    /// Total number of cells with data
    pub cell_count: usize,
    /// Number of rows across all sheets
    pub row_count: usize,
}

impl OdsDocument {
    /// Create a new empty ODS document
    #[inline]
    #[must_use = "creates empty ODS document"]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add text content
    #[inline]
    pub fn add_text(&mut self, text: &str) {
        if !self.text.is_empty() && !self.text.ends_with('\n') {
            self.text.push('\n');
        }
        self.text.push_str(text);
    }

    /// Add a sheet header
    #[inline]
    pub fn add_sheet_header(&mut self, sheet_name: &str) {
        if !self.text.is_empty() {
            self.text.push_str("\n\n");
        }
        let _ = writeln!(self.text, "## Sheet: {sheet_name}");
    }

    /// Add a table row
    #[inline]
    pub fn add_row(&mut self, cells: &[String]) {
        let row_text = cells.join(" | ");
        self.add_text(&row_text);
        self.row_count += 1;
    }
}

/// Parse ODS file from a path
///
/// # Errors
///
/// Returns an error if the file cannot be opened (I/O error) or if the ODS content
/// is invalid (not a valid spreadsheet format or corrupted file).
#[must_use = "this function returns a parsed ODS document that should be processed"]
pub fn parse_ods_file<P: AsRef<Path>>(path: P) -> Result<OdsDocument> {
    let mut workbook = open_workbook::<Ods<_>, _>(path)?;
    let mut doc = OdsDocument::new();

    // Get all sheet names
    doc.sheet_names.clone_from(&workbook.sheet_names());
    doc.sheet_count = doc.sheet_names.len();

    // Process each sheet
    for sheet_name in doc.sheet_names.clone() {
        if let Ok(range) = workbook.worksheet_range(&sheet_name) {
            process_sheet(&mut doc, &sheet_name, &range);
        }
    }

    Ok(doc)
}

/// Process a single sheet
fn process_sheet(doc: &mut OdsDocument, sheet_name: &str, range: &Range<Data>) {
    // Add sheet header
    doc.add_sheet_header(sheet_name);

    // Get dimensions
    let (row_count, col_count) = range.get_size();
    if row_count == 0 || col_count == 0 {
        return;
    }

    // Collect all rows first
    let mut all_rows = Vec::new();
    for row_idx in 0..row_count {
        let mut row_cells = Vec::new();
        let mut has_data = false;

        for col_idx in 0..col_count {
            if let Some(cell) = range.get((row_idx, col_idx)) {
                let cell_text = format_cell(cell);
                if !cell_text.is_empty() {
                    has_data = true;
                    doc.cell_count += 1;
                }
                row_cells.push(cell_text);
            } else {
                row_cells.push(String::new());
            }
        }

        // Only add row if it has data
        if has_data {
            all_rows.push(row_cells);
        }
    }

    // Format as markdown table if we have rows
    if !all_rows.is_empty() {
        // First row as header
        let header = format!("| {} |", all_rows[0].join(" | "));
        doc.add_text(&header);

        // Header separator
        let separator = format!("|{}|", vec![" --- "; col_count].join("|"));
        doc.add_text(&separator);
        doc.row_count += 1;

        // Remaining rows as data
        for row_cells in all_rows.iter().skip(1) {
            let row_text = format!("| {} |", row_cells.join(" | "));
            doc.add_text(&row_text);
            doc.row_count += 1;
        }

        // Single row case - still treat first row as header
        if all_rows.len() == 1 {
            doc.row_count += 1;
        }
    }
}

/// Extract sheets with structured table data
/// Returns a vector of sheets, each containing structured row/column data
///
/// # Errors
///
/// Returns an error if the file cannot be opened (I/O error) or if the ODS content
/// is invalid (not a valid spreadsheet format or corrupted file).
#[must_use = "this function returns parsed ODS sheets that should be processed"]
pub fn parse_ods_sheets<P: AsRef<Path>>(path: P) -> Result<Vec<OdsSheet>> {
    let mut workbook = open_workbook::<Ods<_>, _>(path)?;
    let mut sheets = Vec::new();

    // Get all sheet names
    let sheet_names: Vec<String> = workbook.sheet_names();

    // Process each sheet
    for sheet_name in sheet_names {
        if let Ok(range) = workbook.worksheet_range(&sheet_name) {
            let sheet = extract_sheet_data(&sheet_name, &range);
            sheets.push(sheet);
        }
    }

    Ok(sheets)
}

/// Extract structured data from a single sheet
fn extract_sheet_data(sheet_name: &str, range: &Range<Data>) -> OdsSheet {
    let (row_count, col_count) = range.get_size();
    let mut rows = Vec::new();

    // Process each row
    for row_idx in 0..row_count {
        let mut row_cells = Vec::new();
        let mut has_data = false;

        for col_idx in 0..col_count {
            if let Some(cell) = range.get((row_idx, col_idx)) {
                let cell_text = format_cell(cell);
                if !cell_text.is_empty() {
                    has_data = true;
                }
                row_cells.push(cell_text);
            } else {
                row_cells.push(String::new());
            }
        }

        // Only add row if it has data
        if has_data {
            rows.push(row_cells);
        }
    }

    let row_count = rows.len();
    OdsSheet {
        name: sheet_name.to_string(),
        rows,
        row_count,
        col_count,
    }
}

/// Format a cell value as a string
#[inline]
fn format_cell(cell: &Data) -> String {
    match cell {
        Data::Int(i) => i.to_string(),
        Data::Float(f) => {
            // Format float, removing unnecessary trailing zeros
            let s = f.to_string();
            if s.contains('.') {
                s.trim_end_matches('0').trim_end_matches('.').to_string()
            } else {
                s
            }
        }
        Data::String(s) => s.clone(),
        Data::Bool(b) => b.to_string(),
        Data::DateTime(dt) => format!("{dt}"),
        Data::DateTimeIso(dt) => dt.clone(),
        Data::DurationIso(d) => d.clone(),
        Data::Error(e) => format!("ERROR: {e:?}"),
        Data::Empty => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ods_document_creation() {
        let mut doc = OdsDocument::new();
        assert_eq!(doc.text, "");
        doc.add_text("Sheet 1");
        assert_eq!(doc.text, "Sheet 1");
        doc.add_text("Row 1");
        assert_eq!(doc.text, "Sheet 1\nRow 1");
    }

    #[test]
    fn test_add_sheet_header() {
        let mut doc = OdsDocument::new();
        doc.add_sheet_header("Sheet1");
        assert!(doc.text.contains("## Sheet: Sheet1"));
    }

    #[test]
    fn test_add_row() {
        let mut doc = OdsDocument::new();
        doc.add_row(&["A1".to_string(), "B1".to_string(), "C1".to_string()]);
        // Note: add_row is now deprecated in favor of markdown table formatting
        // This test preserves the old pipe-separated behavior for backward compatibility
        assert_eq!(doc.text, "A1 | B1 | C1");
        assert_eq!(doc.row_count, 1);
    }

    #[test]
    fn test_format_cell_int() {
        let cell = Data::Int(42);
        assert_eq!(format_cell(&cell), "42");
    }

    #[test]
    fn test_format_cell_float() {
        #[allow(clippy::approx_constant, reason = "intentional test value, not PI")]
        let cell = Data::Float(3.14);
        assert_eq!(format_cell(&cell), "3.14");

        let cell2 = Data::Float(10.0);
        assert_eq!(format_cell(&cell2), "10");
    }

    #[test]
    fn test_format_cell_string() {
        let cell = Data::String("Hello".to_string());
        assert_eq!(format_cell(&cell), "Hello");
    }

    #[test]
    fn test_format_cell_bool() {
        let cell = Data::Bool(true);
        assert_eq!(format_cell(&cell), "true");
    }

    #[test]
    fn test_format_cell_empty() {
        let cell = Data::Empty;
        assert_eq!(format_cell(&cell), "");
    }
}
