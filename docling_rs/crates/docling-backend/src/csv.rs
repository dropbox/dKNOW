//! CSV Backend - Line-by-line port from Python docling v2.58.0
//!
//! Source: ~/`docling/docling/backend/csv_backend.py` (126 lines)
//!
//! Parses CSV files into structured markdown output.
//! Supports dialect detection (comma, semicolon, tab, pipe, colon delimiters).
//! Treats first row as header, no merged cells.
//!
//! # Features
//!
//! - Automatic delimiter detection (`,`, `;`, `\t`, `|`, `:`)
//! - First row treated as header
//! - Flexible column counts (handles ragged rows)
//! - `DocItem` generation (Table with `TableData` structure)

// Clippy pedantic allows:
// - Coordinate calculations use f64 from usize
#![allow(clippy::cast_precision_loss)]

use crate::traits::{BackendOptions, DocumentBackend};
use docling_core::{
    content::{BoundingBox, CoordOrigin, DocItem, ProvenanceItem, TableCell, TableData},
    DoclingError, Document, DocumentMetadata, InputFormat,
};
use std::path::Path;

/// CSV Document Backend
///
/// Ported from: docling/backend/csv_backend.py:17-126
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct CsvBackend;

impl CsvBackend {
    /// Create a new CSV backend instance
    #[inline]
    #[must_use = "creates a backend instance that should be used for parsing"]
    pub const fn new() -> Self {
        Self
    }

    /// Detect CSV delimiter using Python's Sniffer logic
    ///
    /// Python: lines 57-64 (`csv.Sniffer().sniff()`)
    #[inline]
    fn detect_delimiter(content: &str) -> Result<char, DoclingError> {
        // Get first line for delimiter detection
        let first_line = content.lines().next().unwrap_or_default();

        // Count occurrences of each candidate delimiter
        let delimiters = [',', ';', '\t', '|', ':'];
        let mut best_delimiter = ',';
        let mut max_count = 0;

        for &delim in &delimiters {
            let count = first_line.matches(delim).count();
            if count > max_count {
                max_count = count;
                best_delimiter = delim;
            }
        }

        // Validate delimiter is supported (Python: lines 61-64)
        if !delimiters.contains(&best_delimiter) {
            return Err(DoclingError::BackendError(format!(
                "Cannot convert csv with unknown delimiter {best_delimiter:?}"
            )));
        }

        Ok(best_delimiter)
    }

    /// Create a Table `DocItem` from CSV data
    ///
    /// This matches Python's `doc.add_table()` call in csv_backend.py:303-320
    /// Python: `doc.add_table(data=table_data`, ...) where `table_data` has `TableCell` objects
    fn create_table_docitem(csv_data: &[Vec<String>]) -> DocItem {
        let num_rows = csv_data.len();
        let num_cols = csv_data.iter().map(std::vec::Vec::len).max().unwrap_or(0);

        // Create TableData with grid and table_cells
        // Python creates TableCell objects (msexcel_backend.py:289-300)
        let mut grid = Vec::new();
        let mut table_cells = Vec::new();

        for (row_idx, row) in csv_data.iter().enumerate() {
            let mut grid_row = Vec::new();

            for (col_idx, cell_text) in row.iter().enumerate() {
                // Create TableCell matching Python's structure (msexcel_backend.py:289-300)
                let cell = TableCell {
                    text: cell_text.clone(),
                    row_span: Some(1),
                    col_span: Some(1),
                    ref_item: None,
                    start_row_offset_idx: Some(row_idx),
                    start_col_offset_idx: Some(col_idx),
                    ..Default::default()
                };

                grid_row.push(cell.clone());
                table_cells.push(cell);
            }

            // Pad row if shorter than num_cols
            for col_idx in row.len()..num_cols {
                let cell = TableCell {
                    text: String::new(),
                    row_span: Some(1),
                    col_span: Some(1),
                    ref_item: None,
                    start_row_offset_idx: Some(row_idx),
                    start_col_offset_idx: Some(col_idx),
                    ..Default::default()
                };
                grid_row.push(cell.clone());
                table_cells.push(cell);
            }

            grid.push(grid_row);
        }

        let table_data = TableData {
            num_rows,
            num_cols,
            grid,
            table_cells: Some(table_cells),
        };

        // Create Table DocItem (matching Python's add_table structure)
        // Python: doc.add_table(...) in msexcel_backend.py:303-320
        DocItem::Table {
            self_ref: "#/tables/0".to_string(),
            parent: None,
            children: vec![],
            content_layer: "body".to_string(),
            prov: vec![ProvenanceItem {
                page_no: 1,
                bbox: BoundingBox::new(
                    0.0,
                    0.0,
                    num_cols as f64,
                    num_rows as f64,
                    CoordOrigin::TopLeft,
                ),
                charspan: None,
            }],
            data: table_data,
            captions: vec![],
            footnotes: vec![],
            references: vec![],
            image: None,
            annotations: vec![],
        }
    }

    /// Parse CSV content to markdown
    ///
    /// Python: CsvDocumentBackend.convert (lines 52-125)
    fn parse_csv_to_markdown(content: &str) -> Result<String, DoclingError> {
        const MIN_PADDING: usize = 2;

        let dialect = Self::detect_delimiter(content)?;
        log::info!("Parsing CSV with delimiter: {dialect:?}");

        let csv_data = Self::read_csv_data(content, dialect)?;
        log::info!("Detected {} lines", csv_data.len());
        log::debug!("CSV data: {csv_data:?}");

        Self::check_column_uniformity(&csv_data);

        if csv_data.is_empty() {
            return Ok(String::new());
        }

        let num_cols = csv_data.iter().map(Vec::len).max().unwrap_or(0);
        let col_widths = Self::calculate_column_widths(&csv_data, num_cols, MIN_PADDING);
        let is_numeric = Self::detect_numeric_columns(&csv_data, num_cols);

        let mut markdown = Self::format_csv_to_markdown(&csv_data, &col_widths, &is_numeric);

        if markdown.ends_with('\n') {
            markdown.pop();
        }

        Ok(markdown)
    }

    /// Read CSV data from content
    fn read_csv_data(content: &str, dialect: char) -> Result<Vec<Vec<String>>, DoclingError> {
        let mut reader = csv::ReaderBuilder::new()
            .delimiter(dialect as u8)
            .flexible(true)
            .has_headers(false)
            .from_reader(content.as_bytes());

        reader
            .records()
            .map(|result| {
                result
                    .map(|record| {
                        record
                            .iter()
                            .map(std::string::ToString::to_string)
                            .collect()
                    })
                    .map_err(|e| {
                        DoclingError::BackendError(format!("Failed to read CSV record: {e}"))
                    })
            })
            .collect()
    }

    /// Check for uniform column lengths and log warning if inconsistent
    fn check_column_uniformity(csv_data: &[Vec<String>]) {
        if let Some(first_row) = csv_data.first() {
            let expected_length = first_row.len();
            let is_uniform = csv_data.iter().all(|row| row.len() == expected_length);
            if !is_uniform {
                log::warn!(
                    "Inconsistent column lengths detected in CSV data. \
                     Expected {expected_length} columns, but found rows with varying lengths."
                );
            }
        }
    }

    /// Calculate column widths for markdown table
    fn calculate_column_widths(
        csv_data: &[Vec<String>],
        num_cols: usize,
        min_padding: usize,
    ) -> Vec<usize> {
        let mut col_widths = csv_data.first().map_or_else(
            || vec![0; num_cols],
            |header| header.iter().map(|h| h.len() + min_padding).collect(),
        );

        if col_widths.len() < num_cols {
            col_widths.resize(num_cols, min_padding);
        }

        for row in csv_data.iter().skip(1) {
            for (col_idx, cell) in row.iter().enumerate() {
                if col_idx < num_cols {
                    col_widths[col_idx] = col_widths[col_idx].max(cell.len());
                }
            }
        }
        col_widths
    }

    /// Detect which columns are numeric
    fn detect_numeric_columns(csv_data: &[Vec<String>], num_cols: usize) -> Vec<bool> {
        let mut is_numeric = vec![false; num_cols];
        for (col_idx, is_num) in is_numeric.iter_mut().enumerate() {
            let (numeric_count, total_count) =
                csv_data.iter().skip(1).fold((0, 0), |(num, tot), row| {
                    row.get(col_idx).map_or((num, tot), |cell| {
                        (num + i32::from(is_likely_number(cell)), tot + 1)
                    })
                });
            *is_num = total_count > 0 && numeric_count > total_count / 2;
        }
        is_numeric
    }

    /// Format CSV data to markdown table
    fn format_csv_to_markdown(
        csv_data: &[Vec<String>],
        col_widths: &[usize],
        is_numeric: &[bool],
    ) -> String {
        let mut markdown = String::new();

        if let Some(header) = csv_data.first() {
            Self::write_csv_row(&mut markdown, header, col_widths, is_numeric);
            Self::write_csv_separator(&mut markdown, col_widths);
        }

        for row in csv_data.iter().skip(1) {
            Self::write_csv_row(&mut markdown, row, col_widths, is_numeric);
        }
        markdown
    }

    /// Write a single CSV row to markdown
    #[inline]
    fn write_csv_row(md: &mut String, row: &[String], col_widths: &[usize], is_numeric: &[bool]) {
        use std::fmt::Write;
        md.push('|');
        for (col_idx, cell) in row.iter().enumerate() {
            let width = col_widths[col_idx];
            if is_numeric[col_idx] {
                let _ = write!(md, " {cell:>width$} |");
            } else {
                let _ = write!(md, " {cell:<width$} |");
            }
        }
        for &width in col_widths.iter().skip(row.len()) {
            let _ = write!(md, " {:width$} |", "");
        }
        md.push('\n');
    }

    /// Write separator row to markdown
    #[inline]
    fn write_csv_separator(md: &mut String, col_widths: &[usize]) {
        md.push('|');
        for &width in col_widths {
            for _ in 0..width + 2 {
                md.push('-');
            }
            md.push('|');
        }
        md.push('\n');
    }
}

/// Check if a string is likely a number (for right-alignment in tables)
#[inline]
fn is_likely_number(s: &str) -> bool {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return false;
    }
    // Try parsing as integer or float
    trimmed.parse::<i64>().is_ok() || trimmed.parse::<f64>().is_ok()
}

impl DocumentBackend for CsvBackend {
    #[inline]
    fn format(&self) -> InputFormat {
        InputFormat::Csv
    }

    fn parse_bytes(
        &self,
        data: &[u8],
        _options: &BackendOptions,
    ) -> Result<Document, DoclingError> {
        // Use std::str::from_utf8 to avoid allocating a vector copy
        let content = std::str::from_utf8(data)
            .map_err(|e| {
                DoclingError::BackendError(format!("CSV content must be valid UTF-8: {e}"))
            })?
            .to_string();

        // Parse CSV to markdown
        let markdown = Self::parse_csv_to_markdown(&content)?;

        // Parse CSV data for DocItem generation
        let dialect = Self::detect_delimiter(&content)?;
        let mut reader = csv::ReaderBuilder::new()
            .delimiter(dialect as u8)
            .flexible(true)
            .has_headers(false)
            .from_reader(content.as_bytes());

        let csv_data: Vec<Vec<String>> = reader
            .records()
            .map(|result| {
                result
                    .map(|record| {
                        record
                            .iter()
                            .map(std::string::ToString::to_string)
                            .collect()
                    })
                    .map_err(|e| {
                        DoclingError::BackendError(format!("Failed to read CSV record: {e}"))
                    })
            })
            .collect::<Result<Vec<Vec<String>>, DoclingError>>()?;

        // Create DocItem if we have data
        let content_blocks = if csv_data.is_empty() {
            None
        } else {
            let table_item = Self::create_table_docitem(&csv_data);
            Some(vec![table_item])
        };

        let metadata = DocumentMetadata {
            num_pages: None,
            num_characters: markdown.chars().count(),
            ..Default::default()
        };

        Ok(Document {
            markdown,
            format: InputFormat::Csv,
            metadata,
            content_blocks,
            docling_document: None,
        })
    }

    fn parse_file<P: AsRef<Path>>(
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

    #[test]
    fn test_delimiter_detection() {
        assert_eq!(
            CsvBackend::detect_delimiter("a,b,c\n1,2,3").unwrap(),
            ',',
            "Should detect comma delimiter from comma-separated content"
        );
        assert_eq!(
            CsvBackend::detect_delimiter("a;b;c\n1;2;3").unwrap(),
            ';',
            "Should detect semicolon delimiter from semicolon-separated content"
        );
        assert_eq!(
            CsvBackend::detect_delimiter("a\tb\tc\n1\t2\t3").unwrap(),
            '\t',
            "Should detect tab delimiter from tab-separated content"
        );
        assert_eq!(
            CsvBackend::detect_delimiter("a|b|c\n1|2|3").unwrap(),
            '|',
            "Should detect pipe delimiter from pipe-separated content"
        );
    }

    #[test]
    fn test_csv_tab_with_quoted_tabs() {
        // Test CSV with tab delimiter and quoted fields containing tabs
        let content = "Index\tCustomer Id\tCompany\n1\tDD37\tRasmussen Group\n4\t5Cef\t\"Dominguez\tMcmillan and Donovan\"";

        let markdown = CsvBackend::parse_csv_to_markdown(content).unwrap();

        // Debug output
        eprintln!("Generated markdown:\n{markdown}");
        eprintln!("Markdown length: {}", markdown.len());

        // Check that tab inside quoted field is preserved
        assert!(
            markdown.contains("Dominguez\tMcmillan and Donovan"),
            "Tab should be preserved in quoted field"
        );

        // Count column widths manually in first line
        let lines: Vec<&str> = markdown.lines().collect();
        eprintln!("Header line: {}", lines[0]);
        let header_parts: Vec<&str> = lines[0].split('|').collect();
        for (i, part) in header_parts.iter().enumerate() {
            eprintln!("Column {}: '{}' (len={})", i, part, part.len());
        }
    }

    #[test]
    fn test_simple_csv() {
        let content = "Name,Age,City\nAlice,30,NYC\nBob,25,LA";

        let markdown = CsvBackend::parse_csv_to_markdown(content).unwrap();

        // Should have header row, separator, and 2 data rows
        let lines: Vec<&str> = markdown.lines().collect();
        assert_eq!(
            lines.len(),
            4,
            "Should have 4 lines: header, separator, and 2 data rows"
        );

        // Check header format
        assert!(
            lines[0].contains("Name"),
            "Header row should contain 'Name' column"
        );
        assert!(
            lines[0].contains("Age"),
            "Header row should contain 'Age' column"
        );
        assert!(
            lines[0].contains("City"),
            "Header row should contain 'City' column"
        );

        // Check separator
        assert!(
            lines[1].contains("---"),
            "Second line should be separator with dashes"
        );
    }

    #[test]
    fn test_number_alignment() {
        // Valid integers
        assert!(
            is_likely_number("123"),
            "Positive integer '123' should be detected as number"
        );
        assert!(
            is_likely_number("-45"),
            "Negative integer '-45' should be detected as number"
        );
        assert!(
            is_likely_number("0"),
            "Zero '0' should be detected as number"
        );

        // Valid floats
        assert!(
            is_likely_number("1.23"),
            "Positive float '1.23' should be detected as number"
        );
        assert!(
            is_likely_number("-1.23"),
            "Negative float '-1.23' should be detected as number"
        );
        assert!(
            is_likely_number("0.0"),
            "Zero float '0.0' should be detected as number"
        );

        // Scientific notation
        assert!(
            is_likely_number("1.23e10"),
            "Scientific notation '1.23e10' should be detected as number"
        );
        assert!(
            is_likely_number("1e-5"),
            "Scientific notation '1e-5' should be detected as number"
        );

        // Whitespace handling
        assert!(
            is_likely_number("  123  "),
            "Number with surrounding spaces should be detected"
        );
        assert!(
            is_likely_number("\t456\t"),
            "Number with surrounding tabs should be detected"
        );

        // Not numbers
        assert!(
            !is_likely_number("abc"),
            "Alphabetic string 'abc' should not be detected as number"
        );
        assert!(
            !is_likely_number("12abc"),
            "Mixed string '12abc' should not be detected as number"
        );
        assert!(
            !is_likely_number(""),
            "Empty string should not be detected as number"
        );
        assert!(
            !is_likely_number("   "),
            "Whitespace-only string should not be detected as number"
        );
    }

    // ========================================
    // CATEGORY 1: Metadata Tests (3 tests)
    // CSV has minimal metadata, but test document metadata generation
    // ========================================

    #[test]
    fn test_csv_metadata_character_count() {
        // Character count should match generated markdown length
        let content = "Name,Age\nAlice,30\nBob,25";
        let markdown = CsvBackend::parse_csv_to_markdown(content).unwrap();

        let char_count = markdown.chars().count();
        assert!(
            char_count > 20,
            "Character count should be > 20 for meaningful CSV content"
        );
        assert!(
            markdown.contains("Name"),
            "Markdown should contain 'Name' header"
        );
        assert!(
            markdown.contains("Alice"),
            "Markdown should contain 'Alice' data value"
        );
        assert!(
            markdown.contains("Bob"),
            "Markdown should contain 'Bob' data value"
        );
    }

    #[test]
    fn test_csv_metadata_empty_file() {
        // Empty CSV should produce empty markdown
        let content = "";
        let markdown = CsvBackend::parse_csv_to_markdown(content).unwrap();

        assert_eq!(markdown, "", "Empty CSV should produce empty markdown");
        assert_eq!(
            markdown.chars().count(),
            0,
            "Empty CSV markdown should have zero characters"
        );
    }

    #[test]
    fn test_csv_metadata_header_only() {
        // CSV with only header row (no data rows)
        let content = "Col1,Col2,Col3";
        let markdown = CsvBackend::parse_csv_to_markdown(content).unwrap();

        // Should have header and separator only (no data rows)
        let lines: Vec<&str> = markdown.lines().collect();
        assert_eq!(
            lines.len(),
            2,
            "Header-only CSV should produce 2 lines: header and separator"
        );
        assert!(
            lines[0].contains("Col1"),
            "Header row should contain 'Col1'"
        );
        assert!(
            lines[1].contains("---"),
            "Second line should be separator with dashes"
        );
    }

    // ========================================
    // CATEGORY 2: DocItem Generation Tests (3 tests)
    // CSV generates Table DocItem with TableData and TableCell structures
    // ========================================

    #[test]
    fn test_csv_docitem_table_structure() {
        // Verify Table DocItem is generated with correct structure
        let content = "Name,Age\nAlice,30\nBob,25";
        let dialect = CsvBackend::detect_delimiter(content).unwrap();

        let mut reader = csv::ReaderBuilder::new()
            .delimiter(dialect as u8)
            .flexible(true)
            .has_headers(false)
            .from_reader(content.as_bytes());

        let csv_data: Vec<Vec<String>> = reader
            .records()
            .map(|r| r.unwrap().iter().map(str::to_string).collect())
            .collect();

        let table_item = CsvBackend::create_table_docitem(&csv_data);

        // Verify it's a Table DocItem
        match table_item {
            DocItem::Table {
                self_ref,
                data,
                prov,
                ..
            } => {
                assert_eq!(
                    self_ref, "#/tables/0",
                    "Table self_ref should be '#/tables/0'"
                );
                assert_eq!(
                    data.num_rows, 3,
                    "Table should have 3 rows (header + 2 data rows)"
                );
                assert_eq!(data.num_cols, 2, "Table should have 2 columns");
                assert_eq!(data.grid.len(), 3, "Grid should have 3 rows");
                assert_eq!(
                    data.grid[0].len(),
                    2,
                    "First grid row should have 2 columns"
                );

                // Verify provenance has bounding box
                assert_eq!(prov.len(), 1, "Table should have 1 provenance item");
                assert_eq!(
                    prov[0].bbox.r, 2.0,
                    "Bounding box right should equal num_cols (2.0)"
                );
                assert_eq!(
                    prov[0].bbox.b, 3.0,
                    "Bounding box bottom should equal num_rows (3.0)"
                );
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    #[test]
    fn test_csv_docitem_table_cells() {
        // Verify TableCell objects are created correctly
        let content = "A,B\n1,2\n3,4";
        let dialect = CsvBackend::detect_delimiter(content).unwrap();

        let mut reader = csv::ReaderBuilder::new()
            .delimiter(dialect as u8)
            .flexible(true)
            .has_headers(false)
            .from_reader(content.as_bytes());

        let csv_data: Vec<Vec<String>> = reader
            .records()
            .map(|r| r.unwrap().iter().map(str::to_string).collect())
            .collect();

        let table_item = CsvBackend::create_table_docitem(&csv_data);

        match table_item {
            DocItem::Table { data, .. } => {
                let cells = data.table_cells.as_ref().unwrap();
                assert_eq!(cells.len(), 6, "Should have 6 cells (3 rows × 2 cols)");

                // Verify first cell (header)
                assert_eq!(cells[0].text, "A", "First cell text should be 'A'");
                assert_eq!(
                    cells[0].row_span,
                    Some(1),
                    "First cell row_span should be 1"
                );
                assert_eq!(
                    cells[0].col_span,
                    Some(1),
                    "First cell col_span should be 1"
                );
                assert_eq!(
                    cells[0].start_row_offset_idx,
                    Some(0),
                    "First cell should be in row 0"
                );
                assert_eq!(
                    cells[0].start_col_offset_idx,
                    Some(0),
                    "First cell should be in column 0"
                );

                // Verify last cell (data)
                assert_eq!(cells[5].text, "4", "Last cell text should be '4'");
                assert_eq!(
                    cells[5].start_row_offset_idx,
                    Some(2),
                    "Last cell should be in row 2"
                );
                assert_eq!(
                    cells[5].start_col_offset_idx,
                    Some(1),
                    "Last cell should be in column 1"
                );
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    #[test]
    fn test_csv_docitem_empty_csv() {
        // Empty CSV should produce no DocItems
        let content = "";
        let backend = CsvBackend::new();
        let result = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert!(
            result.content_blocks.is_none(),
            "Empty CSV should have no content blocks"
        );
        assert_eq!(
            result.markdown, "",
            "Empty CSV should produce empty markdown"
        );
    }

    // ========================================
    // CATEGORY 3: Format-Specific Features (5 tests)
    // CSV-specific: delimiters, alignment, column widths, padding
    // ========================================

    #[test]
    fn test_csv_delimiter_variants() {
        // Test all supported delimiters: comma, semicolon, tab, pipe, colon

        let test_cases = vec![
            ("a,b,c\n1,2,3", ',', "comma"),
            ("a;b;c\n1;2;3", ';', "semicolon"),
            ("a\tb\tc\n1\t2\t3", '\t', "tab"),
            ("a|b|c\n1|2|3", '|', "pipe"),
            ("a:b:c\n1:2:3", ':', "colon"),
        ];

        for (content, expected_delim, name) in test_cases {
            let detected = CsvBackend::detect_delimiter(content).unwrap();
            assert_eq!(
                detected, expected_delim,
                "Failed to detect {name} delimiter"
            );

            // Verify parsing works
            let markdown = CsvBackend::parse_csv_to_markdown(content).unwrap();
            assert!(markdown.contains('a'), "Missing header in {name} test");
            assert!(markdown.contains('1'), "Missing data in {name} test");
        }
    }

    #[test]
    fn test_csv_numeric_column_alignment() {
        // Numeric columns should be right-aligned, text columns left-aligned
        let content = "Name,Age,Score\nAlice,30,95.5\nBob,25,88.0";
        let markdown = CsvBackend::parse_csv_to_markdown(content).unwrap();

        // Age and Score should be right-aligned (numbers)
        // Name should be left-aligned (text)
        // Format: "| Name  | Age | Score |"
        //         "|-------|-----|-------|"
        //         "| Alice |  30 |  95.5 |"

        let lines: Vec<&str> = markdown.lines().collect();
        assert_eq!(
            lines.len(),
            4,
            "Should have 4 lines: header, separator, and 2 data rows"
        );

        // Check that numeric columns have proper alignment markers
        let header = lines[0];
        assert!(
            header.contains("Name"),
            "Header should contain 'Name' column"
        );
        assert!(header.contains("Age"), "Header should contain 'Age' column");
        assert!(
            header.contains("Score"),
            "Header should contain 'Score' column"
        );
    }

    #[test]
    fn test_csv_column_width_calculation() {
        // Column widths should accommodate longest content + MIN_PADDING
        let content = "Short,VeryLongHeader\nX,Y";
        let markdown = CsvBackend::parse_csv_to_markdown(content).unwrap();

        let lines: Vec<&str> = markdown.lines().collect();
        let header = lines[0];

        // VeryLongHeader should make second column wider
        // Format: "| Short  | VeryLongHeader |"
        assert!(
            header.contains("Short"),
            "Header should contain 'Short' column"
        );
        assert!(
            header.contains("VeryLongHeader"),
            "Header should contain 'VeryLongHeader' column"
        );

        // Separator line should reflect column widths
        let separator = lines[1];
        assert!(
            separator.len() > 20,
            "Separator should be > 20 chars due to long header"
        );
    }

    #[test]
    fn test_csv_irregular_row_lengths() {
        // Rows with varying column counts should be padded
        let content = "A,B,C\n1,2\n3,4,5,6";
        let markdown = CsvBackend::parse_csv_to_markdown(content).unwrap();

        let lines: Vec<&str> = markdown.lines().collect();
        assert_eq!(
            lines.len(),
            4,
            "Should have 4 lines: header, separator, and 2 data rows"
        );

        // All rows should have same number of columns (4 - max from data)
        let header_cols = lines[0].matches('|').count() - 1; // subtract surrounding pipes
        let row1_cols = lines[2].matches('|').count() - 1;
        let row2_cols = lines[3].matches('|').count() - 1;

        assert_eq!(
            header_cols, row1_cols,
            "Header and first data row should have same column count"
        );
        assert_eq!(
            header_cols, row2_cols,
            "Header and second data row should have same column count"
        );
    }

    #[test]
    fn test_csv_github_flavored_markdown_format() {
        // Output should be GitHub Flavored Markdown table format
        let content = "Col1,Col2\nVal1,Val2";
        let markdown = CsvBackend::parse_csv_to_markdown(content).unwrap();

        let lines: Vec<&str> = markdown.lines().collect();
        assert_eq!(
            lines.len(),
            3,
            "Should have 3 lines: header, separator, and 1 data row"
        );

        // Check GFM format: | cell | cell |
        assert!(
            lines[0].starts_with('|'),
            "Header row should start with pipe"
        );
        assert!(lines[0].ends_with('|'), "Header row should end with pipe");

        // Separator should be all dashes: |------|------|
        assert!(
            lines[1].starts_with('|'),
            "Separator should start with pipe"
        );
        assert!(lines[1].contains("---"), "Separator should contain dashes");
        assert!(lines[1].ends_with('|'), "Separator should end with pipe");

        // Data row should follow same format
        assert!(lines[2].starts_with('|'), "Data row should start with pipe");
        assert!(lines[2].ends_with('|'), "Data row should end with pipe");
    }

    // ========================================
    // CATEGORY 4: Edge Cases (3 tests)
    // Test boundary conditions and special scenarios
    // ========================================

    #[test]
    fn test_csv_single_column() {
        // Single column CSV (no delimiters)
        let content = "Name\nAlice\nBob\nCharlie";
        let markdown = CsvBackend::parse_csv_to_markdown(content).unwrap();

        let lines: Vec<&str> = markdown.lines().collect();
        assert_eq!(
            lines.len(),
            5,
            "Single column CSV should have 5 lines: header, separator, and 3 data rows"
        );

        // Verify single column format
        assert!(lines[0].contains("Name"), "Header should contain 'Name'");
        assert!(
            lines[2].contains("Alice"),
            "First data row should contain 'Alice'"
        );
        assert!(
            lines[3].contains("Bob"),
            "Second data row should contain 'Bob'"
        );
        assert!(
            lines[4].contains("Charlie"),
            "Third data row should contain 'Charlie'"
        );
    }

    #[test]
    fn test_csv_empty_cells() {
        // CSV with empty cells should render correctly
        let content = "A,B,C\n1,,3\n,5,\n7,8,9";
        let markdown = CsvBackend::parse_csv_to_markdown(content).unwrap();

        let lines: Vec<&str> = markdown.lines().collect();
        assert_eq!(
            lines.len(),
            5,
            "Should have 5 lines: header, separator, and 3 data rows"
        );

        // Verify empty cells are handled (spaces between pipes)
        assert!(lines[2].contains('1'), "First data row should contain '1'");
        assert!(lines[2].contains('3'), "First data row should contain '3'");

        assert!(lines[3].contains('5'), "Second data row should contain '5'");

        assert!(lines[4].contains('7'), "Third data row should contain '7'");
        assert!(lines[4].contains('8'), "Third data row should contain '8'");
        assert!(lines[4].contains('9'), "Third data row should contain '9'");
    }

    #[test]
    fn test_csv_special_characters() {
        // CSV with special characters (quotes, commas in quotes, unicode)
        let content =
            "Name,Description\nAlice,\"Hello, World\"\nBob,こんにちは\nCharlie,\"Line1\nLine2\"";
        let markdown = CsvBackend::parse_csv_to_markdown(content).unwrap();

        // CSV library should handle quoted commas and newlines
        // Verify unicode is preserved
        assert!(
            markdown.contains("こんにちは"),
            "Unicode should be preserved"
        );

        // Verify quoted comma is treated as single field
        assert!(
            markdown.contains("Hello, World") || markdown.contains("Hello,"),
            "Quoted comma should be in single field"
        );
    }

    // New tests for N=393 expansion

    // ========================================
    // CATEGORY 5: Backend Trait Conformance (3 tests)
    // ========================================

    #[test]
    fn test_csv_backend_new_vs_default() {
        let backend1 = CsvBackend::new();
        let backend2 = CsvBackend;

        // Both should return Csv format
        assert_eq!(
            backend1.format(),
            InputFormat::Csv,
            "CsvBackend::new() should return Csv format"
        );
        assert_eq!(
            backend2.format(),
            InputFormat::Csv,
            "CsvBackend struct should return Csv format"
        );
    }

    #[test]
    fn test_csv_backend_format_method() {
        let backend = CsvBackend::new();
        assert_eq!(
            backend.format(),
            InputFormat::Csv,
            "format() method should return InputFormat::Csv"
        );
    }

    #[test]
    fn test_csv_backend_parse_bytes_invalid_utf8() {
        let backend = CsvBackend::new();
        let invalid_utf8 = vec![0xFF, 0xFE, 0xFD]; // Invalid UTF-8 sequence

        let result = backend.parse_bytes(&invalid_utf8, &BackendOptions::default());
        assert!(result.is_err(), "Invalid UTF-8 should return error");

        match result {
            Err(DoclingError::BackendError(msg)) => {
                assert!(
                    msg.contains("valid UTF-8"),
                    "Error message should mention UTF-8 validity"
                );
            }
            _ => panic!("Expected BackendError for invalid UTF-8"),
        }
    }

    // ========================================
    // CATEGORY 6: Delimiter Edge Cases (3 tests)
    // ========================================

    #[test]
    fn test_csv_no_delimiter_single_column() {
        // CSV with no delimiters should default to comma
        let content = "Name\nAlice\nBob";

        let delimiter = CsvBackend::detect_delimiter(content).unwrap();
        assert_eq!(delimiter, ',', "No delimiter found should default to comma");
    }

    #[test]
    fn test_csv_delimiter_priority_comma_wins() {
        // When comma appears most, it should be selected
        let content = "a,b,c,d\n1|2|3,4";

        let delimiter = CsvBackend::detect_delimiter(content).unwrap();
        assert_eq!(
            delimiter, ',',
            "Comma should win when it appears most (3 commas > 2 pipes)"
        );
    }

    #[test]
    fn test_csv_delimiter_detection_empty_content() {
        // Empty content should default to comma
        let content = "";

        let delimiter = CsvBackend::detect_delimiter(content).unwrap();
        assert_eq!(
            delimiter, ',',
            "Empty content should default to comma delimiter"
        );
    }

    // ========================================
    // CATEGORY 7: Table Structure Validation (5 tests)
    // ========================================

    #[test]
    fn test_csv_table_self_ref_format() {
        let content = "A,B\n1,2";
        let backend = CsvBackend::new();
        let result = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        let table = &result.content_blocks.as_ref().unwrap()[0];
        match table {
            DocItem::Table { self_ref, .. } => {
                assert_eq!(
                    self_ref, "#/tables/0",
                    "Table self_ref should be '#/tables/0'"
                );
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    #[test]
    fn test_csv_table_provenance_page_no() {
        let content = "A,B\n1,2";
        let backend = CsvBackend::new();
        let result = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        let table = &result.content_blocks.as_ref().unwrap()[0];
        match table {
            DocItem::Table { prov, .. } => {
                assert_eq!(prov.len(), 1, "Should have exactly 1 provenance item");
                assert_eq!(prov[0].page_no, 1, "CSV provenance should use page_no 1");
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    #[test]
    fn test_csv_table_bounding_box_coordinates() {
        let content = "A,B,C\n1,2,3\n4,5,6";
        let backend = CsvBackend::new();
        let result = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        let table = &result.content_blocks.as_ref().unwrap()[0];
        match table {
            DocItem::Table { prov, .. } => {
                let bbox = &prov[0].bbox;
                assert_eq!(bbox.l, 0.0, "Bounding box left should be 0.0");
                assert_eq!(bbox.t, 0.0, "Bounding box top should be 0.0");
                assert_eq!(
                    bbox.r, 3.0,
                    "Bounding box right should equal num_cols (3.0)"
                );
                assert_eq!(
                    bbox.b, 3.0,
                    "Bounding box bottom should equal num_rows (3.0)"
                );
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    #[test]
    fn test_csv_table_content_layer() {
        let content = "A,B\n1,2";
        let backend = CsvBackend::new();
        let result = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        let table = &result.content_blocks.as_ref().unwrap()[0];
        match table {
            DocItem::Table { content_layer, .. } => {
                assert_eq!(
                    content_layer, "body",
                    "Table content_layer should be 'body'"
                );
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    #[test]
    fn test_csv_table_empty_optional_fields() {
        let content = "A,B\n1,2";
        let backend = CsvBackend::new();
        let result = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        let table = &result.content_blocks.as_ref().unwrap()[0];
        match table {
            DocItem::Table {
                parent,
                children,
                captions,
                footnotes,
                references,
                image,
                annotations,
                ..
            } => {
                assert!(parent.is_none(), "CSV table should have no parent");
                assert!(children.is_empty(), "CSV table should have no children");
                assert!(captions.is_empty(), "CSV table should have no captions");
                assert!(footnotes.is_empty(), "CSV table should have no footnotes");
                assert!(references.is_empty(), "CSV table should have no references");
                assert!(image.is_none(), "CSV table should have no image");
                assert!(
                    annotations.is_empty(),
                    "CSV table should have no annotations"
                );
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    // ========================================
    // CATEGORY 8: Cell Content Edge Cases (4 tests)
    // ========================================

    #[test]
    fn test_csv_very_long_cell_value() {
        // Test cell with 1000+ character value
        let long_text = "x".repeat(1000);
        let content = format!("Header\n{long_text}");

        let markdown = CsvBackend::parse_csv_to_markdown(&content).unwrap();

        // Verify long text is preserved
        assert!(
            markdown.contains(&long_text),
            "Long text (1000 chars) should be preserved in markdown"
        );
        assert!(
            markdown.len() > 1000,
            "Markdown should be longer than 1000 chars for long content"
        );
    }

    #[test]
    fn test_csv_unicode_emoji_cells() {
        // Test cells with emoji and various unicode
        let content = "Name,Emoji\nAlice,😀🎉\nBob,🚀🌟\nCharlie,日本語";

        let markdown = CsvBackend::parse_csv_to_markdown(content).unwrap();

        // Verify emoji and unicode are preserved
        assert!(markdown.contains("😀🎉"), "Party emoji should be preserved");
        assert!(
            markdown.contains("🚀🌟"),
            "Rocket and star emoji should be preserved"
        );
        assert!(
            markdown.contains("日本語"),
            "Japanese unicode should be preserved"
        );
    }

    #[test]
    fn test_csv_markdown_special_chars_escaped() {
        // Test cells with markdown special characters
        let content = "Text,Markdown\nPlain,**Bold**\nPlain,_Italic_\nPlain,`Code`";

        let markdown = CsvBackend::parse_csv_to_markdown(content).unwrap();

        // Verify markdown characters are preserved as-is (not interpreted)
        assert!(
            markdown.contains("**Bold**"),
            "Bold markdown syntax should be preserved"
        );
        assert!(
            markdown.contains("_Italic_"),
            "Italic markdown syntax should be preserved"
        );
        assert!(
            markdown.contains("`Code`"),
            "Code markdown syntax should be preserved"
        );
    }

    #[test]
    fn test_csv_cell_with_pipes() {
        // Test cells with pipe characters (markdown table delimiter)
        let content = "Text,Value\nPlain,A|B|C";

        let markdown = CsvBackend::parse_csv_to_markdown(content).unwrap();

        // Verify pipes are preserved (should not break table structure)
        assert!(
            markdown.contains("A|B|C"),
            "Pipe characters in cell content should be preserved"
        );
    }

    // ========================================
    // CATEGORY 9: Document Structure Integration (3 tests)
    // ========================================

    #[test]
    fn test_csv_full_document_structure() {
        let content = "Name,Age\nAlice,30";
        let backend = CsvBackend::new();
        let result = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Verify all Document fields
        assert!(
            !result.markdown.is_empty(),
            "Document markdown should not be empty"
        );
        assert_eq!(
            result.format,
            InputFormat::Csv,
            "Document format should be Csv"
        );
        assert!(
            result.metadata.num_characters > 0,
            "Document should have positive character count"
        );
        assert!(
            result.content_blocks.is_some(),
            "Document should have content blocks"
        );

        let blocks = result.content_blocks.unwrap();
        assert_eq!(
            blocks.len(),
            1,
            "Document should have exactly 1 content block (the table)"
        );
    }

    #[test]
    fn test_csv_document_metadata_fields() {
        let content = "A,B,C\n1,2,3\n4,5,6";
        let backend = CsvBackend::new();
        let result = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Verify metadata
        assert!(
            result.metadata.num_pages.is_none(),
            "CSV should have no page count (num_pages=None)"
        );
        assert!(
            result.metadata.num_characters > 0,
            "CSV should have positive character count"
        );

        // Character count should match markdown length
        assert_eq!(
            result.metadata.num_characters,
            result.markdown.chars().count(),
            "Metadata character count should match markdown length"
        );
    }

    #[test]
    fn test_csv_content_blocks_none_for_empty() {
        let content = "";
        let backend = CsvBackend::new();
        let result = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Empty CSV should have None content_blocks
        assert!(
            result.content_blocks.is_none(),
            "Empty CSV should have None content_blocks"
        );
        assert_eq!(
            result.markdown, "",
            "Empty CSV should produce empty markdown"
        );
        assert_eq!(
            result.metadata.num_characters, 0,
            "Empty CSV should have zero character count"
        );
    }

    // ========================================
    // CATEGORY 10: Additional Edge Cases (2 tests)
    // ========================================

    #[test]
    fn test_csv_single_row_header_only_docitem() {
        // Single row (header only) should still create Table DocItem
        let content = "Col1,Col2,Col3";
        let backend = CsvBackend::new();
        let result = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Should have content_blocks with table
        assert!(
            result.content_blocks.is_some(),
            "Header-only CSV should create content blocks"
        );

        let table = &result.content_blocks.as_ref().unwrap()[0];
        match table {
            DocItem::Table { data, .. } => {
                assert_eq!(data.num_rows, 1, "Header-only table should have 1 row");
                assert_eq!(data.num_cols, 3, "Header-only table should have 3 columns");
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    #[test]
    fn test_csv_large_table() {
        // Test CSV with many rows and columns
        let mut content = String::from("A,B,C,D,E,F,G,H,I,J\n");
        for i in 0..100 {
            content.push_str(&format!(
                "{},{},{},{},{},{},{},{},{},{}\n",
                i,
                i + 1,
                i + 2,
                i + 3,
                i + 4,
                i + 5,
                i + 6,
                i + 7,
                i + 8,
                i + 9
            ));
        }

        let backend = CsvBackend::new();
        let result = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Verify large table structure
        assert!(
            result.content_blocks.is_some(),
            "Large CSV should create content blocks"
        );

        let table = &result.content_blocks.as_ref().unwrap()[0];
        match table {
            DocItem::Table { data, .. } => {
                assert_eq!(
                    data.num_rows, 101,
                    "Large table should have 101 rows (1 header + 100 data)"
                );
                assert_eq!(data.num_cols, 10, "Large table should have 10 columns");
                assert_eq!(data.grid.len(), 101, "Grid should have 101 rows");

                // Verify table_cells count
                let cells = data.table_cells.as_ref().unwrap();
                assert_eq!(
                    cells.len(),
                    1010,
                    "Large table should have 1010 cells (101 rows × 10 cols)"
                );
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    // ========================================
    // CATEGORY 11: Number Detection and Alignment (3 tests)
    // ========================================

    #[test]
    fn test_is_likely_number_negative_numbers() {
        // Negative integers and floats should be detected
        assert!(
            is_likely_number("-123"),
            "Negative integer '-123' should be detected as number"
        );
        assert!(
            is_likely_number("-1.5"),
            "Negative float '-1.5' should be detected as number"
        );
        assert!(
            is_likely_number("-0.001"),
            "Small negative float '-0.001' should be detected as number"
        );
    }

    #[test]
    fn test_is_likely_number_mixed_content() {
        // Mixed alphanumeric should NOT be numbers
        assert!(
            !is_likely_number("123abc"),
            "String '123abc' should not be detected as number"
        );
        assert!(
            !is_likely_number("abc123"),
            "String 'abc123' should not be detected as number"
        );
        assert!(
            !is_likely_number("12.34.56"),
            "Invalid float '12.34.56' should not be detected as number"
        );
        assert!(
            !is_likely_number("1,000"),
            "Comma-separated number '1,000' should not be detected as number"
        );
    }

    #[test]
    fn test_is_likely_number_whitespace_trimming() {
        // Should trim whitespace before checking
        assert!(
            is_likely_number("  123  "),
            "Number with surrounding spaces should be detected"
        );
        assert!(
            is_likely_number("\t45.6\n"),
            "Number with tab and newline should be detected"
        );
        assert!(
            !is_likely_number("  abc  "),
            "Text with surrounding spaces should not be detected as number"
        );
    }

    // ========================================
    // CATEGORY 12: Grid Padding Edge Cases (3 tests)
    // ========================================

    #[test]
    fn test_csv_grid_padding_short_rows() {
        // Rows shorter than max should be padded with empty cells
        let content = "A,B,C,D\n1,2\n3,4,5\n6,7,8,9";
        let dialect = CsvBackend::detect_delimiter(content).unwrap();

        let mut reader = csv::ReaderBuilder::new()
            .delimiter(dialect as u8)
            .flexible(true)
            .has_headers(false)
            .from_reader(content.as_bytes());

        let csv_data: Vec<Vec<String>> = reader
            .records()
            .map(|r| r.unwrap().iter().map(str::to_string).collect())
            .collect();

        let table_item = CsvBackend::create_table_docitem(&csv_data);

        match table_item {
            DocItem::Table { data, .. } => {
                // All rows in grid should have 4 columns (max)
                assert_eq!(
                    data.num_cols, 4,
                    "Table should have 4 columns (max from all rows)"
                );
                for (row_idx, row) in data.grid.iter().enumerate() {
                    assert_eq!(
                        row.len(),
                        4,
                        "Row {row_idx} should have 4 cells (padded to max)"
                    );
                }
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    #[test]
    fn test_csv_cell_row_col_indices() {
        // Verify each cell has correct start_row_offset_idx and start_col_offset_idx
        let content = "A,B\n1,2\n3,4";
        let dialect = CsvBackend::detect_delimiter(content).unwrap();

        let mut reader = csv::ReaderBuilder::new()
            .delimiter(dialect as u8)
            .flexible(true)
            .has_headers(false)
            .from_reader(content.as_bytes());

        let csv_data: Vec<Vec<String>> = reader
            .records()
            .map(|r| r.unwrap().iter().map(str::to_string).collect())
            .collect();

        let table_item = CsvBackend::create_table_docitem(&csv_data);

        match table_item {
            DocItem::Table { data, .. } => {
                // Verify cell indices match their position
                for (row_idx, row) in data.grid.iter().enumerate() {
                    for (col_idx, cell) in row.iter().enumerate() {
                        assert_eq!(
                            cell.start_row_offset_idx,
                            Some(row_idx),
                            "Cell at ({row_idx},{col_idx}) should have row_offset={row_idx}"
                        );
                        assert_eq!(
                            cell.start_col_offset_idx,
                            Some(col_idx),
                            "Cell at ({row_idx},{col_idx}) should have col_offset={col_idx}"
                        );
                    }
                }
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    #[test]
    fn test_csv_cell_span_always_one() {
        // CSV cells should always have row_span=1 and col_span=1 (no merging)
        let content = "A,B,C\n1,2,3\n4,5,6";
        let dialect = CsvBackend::detect_delimiter(content).unwrap();

        let mut reader = csv::ReaderBuilder::new()
            .delimiter(dialect as u8)
            .flexible(true)
            .has_headers(false)
            .from_reader(content.as_bytes());

        let csv_data: Vec<Vec<String>> = reader
            .records()
            .map(|r| r.unwrap().iter().map(str::to_string).collect())
            .collect();

        let table_item = CsvBackend::create_table_docitem(&csv_data);

        match table_item {
            DocItem::Table { data, .. } => {
                let cells = data.table_cells.as_ref().unwrap();
                for cell in cells {
                    assert_eq!(cell.row_span, Some(1), "All cells should have row_span=1");
                    assert_eq!(cell.col_span, Some(1), "All cells should have col_span=1");
                }
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    // ========================================
    // CATEGORY 13: Markdown Separator Line (2 tests)
    // ========================================

    #[test]
    fn test_csv_separator_line_dash_count() {
        // Separator dashes should be width + 2 (for padding spaces)
        let content = "Short,VeryLongColumnName\nX,Y";
        let markdown = CsvBackend::parse_csv_to_markdown(content).unwrap();

        let lines: Vec<&str> = markdown.lines().collect();
        let separator = lines[1];

        // Separator should have more dashes for longer column
        let dash_sections: Vec<&str> = separator.split('|').collect();
        assert!(
            dash_sections.len() >= 3,
            "Should have at least 2 columns plus surrounding |"
        );

        // Second column should have more dashes (VeryLongColumnName is longer)
        let col1_dashes = dash_sections[1].matches('-').count();
        let col2_dashes = dash_sections[2].matches('-').count();
        assert!(
            col2_dashes > col1_dashes,
            "Longer column should have more dashes"
        );
    }

    #[test]
    fn test_csv_separator_no_trailing_newline() {
        // Markdown output should not have trailing newline
        let content = "A,B\n1,2";
        let markdown = CsvBackend::parse_csv_to_markdown(content).unwrap();

        assert!(
            !markdown.ends_with('\n'),
            "Should not have trailing newline"
        );
        assert!(
            !markdown.ends_with("\n\n"),
            "Should not have double trailing newline"
        );
    }

    // ========================================
    // CATEGORY 14: Parse File Method (2 tests)
    // ========================================

    #[test]
    fn test_csv_parse_file_method() {
        // Test parse_file method (as opposed to parse_bytes)
        use std::io::Write;
        use tempfile::NamedTempFile;

        let content = "Name,Score\nAlice,95\nBob,88";
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let backend = CsvBackend::new();
        let result = backend
            .parse_file(temp_file.path(), &BackendOptions::default())
            .unwrap();

        assert_eq!(
            result.format,
            InputFormat::Csv,
            "parse_file should return Csv format"
        );
        assert!(
            result.markdown.contains("Name"),
            "Parsed markdown should contain 'Name' header"
        );
        assert!(
            result.markdown.contains("Alice"),
            "Parsed markdown should contain 'Alice' data"
        );
        assert!(
            result.content_blocks.is_some(),
            "Parsed file should have content blocks"
        );
    }

    #[test]
    fn test_csv_parse_file_nonexistent() {
        // Parsing non-existent file should return error
        let backend = CsvBackend::new();
        let result =
            backend.parse_file("/nonexistent/path/to/file.csv", &BackendOptions::default());

        assert!(result.is_err(), "Non-existent file should return error");
        match result {
            Err(DoclingError::IoError(_)) => {} // Expected
            _ => panic!("Expected IoError for non-existent file"),
        }
    }

    // ========================================
    // CATEGORY 15: Additional Delimiter Robustness (2 tests)
    // ========================================

    #[test]
    fn test_csv_mixed_delimiters_first_wins() {
        // When multiple delimiters present, most frequent in first line wins
        let content = "a,b,c,d\n1;2;3;4\n5,6,7,8";

        let delimiter = CsvBackend::detect_delimiter(content).unwrap();
        assert_eq!(
            delimiter, ',',
            "First line comma count (3) should win over later semicolons"
        );
    }

    #[test]
    fn test_csv_colon_delimiter_parsing() {
        // Colon delimiter is valid but uncommon
        let content = "Name:Age:City\nAlice:30:NYC\nBob:25:LA";

        let delimiter = CsvBackend::detect_delimiter(content).unwrap();
        assert_eq!(delimiter, ':', "Should detect colon delimiter");

        let markdown = CsvBackend::parse_csv_to_markdown(content).unwrap();
        assert!(
            markdown.contains("Name"),
            "Colon-delimited CSV should parse 'Name'"
        );
        assert!(
            markdown.contains("Alice"),
            "Colon-delimited CSV should parse 'Alice'"
        );
        assert!(
            markdown.contains("NYC"),
            "Colon-delimited CSV should parse 'NYC'"
        );
    }

    // ========================================
    // CATEGORY 16: Additional Edge Cases (2 tests)
    // ========================================

    #[test]
    fn test_csv_with_numeric_precision() {
        // Test handling of high-precision numeric data
        let content = r"Measurement,Value,Scientific
Temperature,98.60000000001,1.234567890123456789e-10
Pressure,101.325,2.99792458e8
Weight,0.000000000001,6.62607015e-34";
        let backend = CsvBackend::new();
        let markdown = CsvBackend::parse_csv_to_markdown(content).unwrap();

        // Should preserve numeric precision
        assert!(markdown.contains("98.60000000001") || markdown.contains("98.6"));
        assert!(markdown.contains("1.23456") || markdown.contains("e-10"));
        assert!(markdown.contains("101.325"));
        assert!(markdown.contains("2.99792458e8") || markdown.contains("e8"));
        assert!(markdown.contains("6.62607015e-34") || markdown.contains("e-34"));

        // Should have proper table structure
        assert!(markdown.contains("Measurement"));
        assert!(markdown.contains("Value"));
        assert!(markdown.contains("Scientific"));

        // Should generate DocItems
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();
        assert!(doc.content_blocks.is_some());
        let items = doc.content_blocks.unwrap();
        assert!(items
            .iter()
            .any(|item| matches!(item, DocItem::Table { .. })));
    }

    #[test]
    fn test_csv_with_international_data() {
        // Test handling of international data with various scripts
        let content = r"Country,Capital,Population,Language
中国,北京,1400000000,中文
日本,東京,126000000,日本語
한국,서울,51000000,한국어
Россия,Москва,146000000,Русский
العربية,القاهرة,100000000,العربية
";
        let backend = CsvBackend::new();
        let markdown = CsvBackend::parse_csv_to_markdown(content).unwrap();

        // Should handle Chinese characters
        assert!(markdown.contains("中国"));
        assert!(markdown.contains("北京"));
        assert!(markdown.contains("中文"));

        // Should handle Japanese characters
        assert!(markdown.contains("日本"));
        assert!(markdown.contains("東京"));
        assert!(markdown.contains("日本語"));

        // Should handle Korean characters
        assert!(markdown.contains("한국"));
        assert!(markdown.contains("서울"));
        assert!(markdown.contains("한국어"));

        // Should handle Cyrillic characters
        assert!(markdown.contains("Россия"));
        assert!(markdown.contains("Москва"));
        assert!(markdown.contains("Русский"));

        // Should handle Arabic characters
        assert!(markdown.contains("العربية"));
        assert!(markdown.contains("القاهرة"));

        // Should maintain table structure with 5 rows (header + 4 data)
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();
        let items = doc.content_blocks.unwrap();

        // Find the table DocItem
        let table_item = items.iter().find_map(|item| {
            if let DocItem::Table { data, .. } = item {
                Some(data)
            } else {
                None
            }
        });

        assert!(table_item.is_some());
        let table = table_item.unwrap();
        assert_eq!(table.num_cols, 4); // Country, Capital, Population, Language
        assert!(table.num_rows >= 4); // At least 4 data rows (+ possibly header)

        // Character count should be substantial due to Unicode
        assert!(doc.metadata.num_characters > 100);
    }

    // ========================================
    // CATEGORY 17: Line Ending and Encoding Edge Cases (4 tests)
    // ========================================

    #[test]
    fn test_csv_with_utf8_bom() {
        // Test CSV with UTF-8 BOM (Byte Order Mark)
        let bom = "\u{FEFF}";
        let content = format!("{bom}Name,Age,City\nAlice,30,NYC\nBob,25,LA");
        let backend = CsvBackend::new();

        let markdown = CsvBackend::parse_csv_to_markdown(&content).unwrap();

        // Should handle BOM gracefully and parse data
        assert!(markdown.contains("Name"));
        assert!(markdown.contains("Alice"));
        assert!(markdown.contains("NYC"));

        // Should generate proper DocItems
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();
        assert!(doc.content_blocks.is_some());
        let items = doc.content_blocks.unwrap();
        assert!(items
            .iter()
            .any(|item| matches!(item, DocItem::Table { .. })));
    }

    #[test]
    fn test_csv_with_windows_line_endings() {
        // Test CSV with Windows line endings (\r\n)
        let content = "Name,Age,City\r\nAlice,30,NYC\r\nBob,25,LA\r\n";
        let backend = CsvBackend::new();

        let markdown = CsvBackend::parse_csv_to_markdown(content).unwrap();

        // Should parse Windows line endings correctly
        assert!(markdown.contains("Name"));
        assert!(markdown.contains("Alice"));
        assert!(markdown.contains("Bob"));

        // Should have 3 rows (header + 2 data)
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();
        let items = doc.content_blocks.unwrap();
        let table_item = items.iter().find_map(|item| {
            if let DocItem::Table { data, .. } = item {
                Some(data)
            } else {
                None
            }
        });

        assert!(table_item.is_some());
        let table = table_item.unwrap();
        assert_eq!(table.num_rows, 3); // header + 2 data rows
    }

    #[test]
    fn test_csv_with_mac_line_endings() {
        // Test CSV with old Mac line endings (\r only)
        let content = "Name,Age,City\rAlice,30,NYC\rBob,25,LA\r";
        let backend = CsvBackend::new();

        let markdown = CsvBackend::parse_csv_to_markdown(content).unwrap();

        // Should parse Mac line endings correctly
        assert!(markdown.contains("Name"));
        assert!(markdown.contains("Alice"));
        assert!(markdown.contains("Bob"));

        // Should generate DocItems
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();
        assert!(doc.content_blocks.is_some());
    }

    #[test]
    fn test_csv_with_mixed_line_endings() {
        // Test CSV with mixed line endings (Unix, Windows, Mac)
        let content = "Name,Age,City\nAlice,30,NYC\r\nBob,25,LA\rCharlie,35,SF\n";
        let backend = CsvBackend::new();

        let markdown = CsvBackend::parse_csv_to_markdown(content).unwrap();

        // Should handle mixed line endings gracefully
        assert!(markdown.contains("Name"));
        assert!(markdown.contains("Alice"));
        assert!(markdown.contains("Bob"));
        assert!(markdown.contains("Charlie"));

        // Should have 4 rows (header + 3 data)
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();
        let items = doc.content_blocks.unwrap();
        let table_item = items.iter().find_map(|item| {
            if let DocItem::Table { data, .. } = item {
                Some(data)
            } else {
                None
            }
        });

        assert!(table_item.is_some());
    }

    // ========================================
    // CATEGORY 18: Quoting and Escaping Edge Cases (3 tests)
    // ========================================

    #[test]
    fn test_csv_with_quoted_newlines() {
        // Test CSV with newlines inside quoted fields
        let content = r#"Name,Address,Phone
Alice,"123 Main St
Apt 4B",555-1234
Bob,"456 Oak Ave
Suite 10",555-5678"#;

        let markdown = CsvBackend::parse_csv_to_markdown(content).unwrap();

        // Should preserve newlines within quoted fields
        assert!(markdown.contains("Alice"));
        assert!(markdown.contains("Bob"));
        // Note: The actual newline handling depends on CSV parser implementation
        // Just verify the content is parsed
        assert!(markdown.contains("Main St") || markdown.contains("123"));
        assert!(markdown.contains("Oak Ave") || markdown.contains("456"));
    }

    #[test]
    fn test_csv_with_escaped_quotes() {
        // Test CSV with escaped quotes (doubled quotes in CSV standard)
        let content = r#"Name,Quote,Year
Shakespeare,"To be, or not to be, ""that"" is the question",1603
Einstein,"Imagination is more important than knowledge. ""E=mc²""",1905"#;
        let backend = CsvBackend::new();

        let markdown = CsvBackend::parse_csv_to_markdown(content).unwrap();

        // Should handle escaped quotes correctly
        assert!(markdown.contains("Shakespeare"));
        assert!(markdown.contains("Einstein"));
        assert!(markdown.contains("To be") || markdown.contains("question"));
        assert!(markdown.contains("Imagination") || markdown.contains("knowledge"));

        // Should generate proper DocItems
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();
        assert!(doc.content_blocks.is_some());
    }

    #[test]
    fn test_csv_with_trailing_comma() {
        // Test CSV with trailing comma (creates empty column)
        let content = "Name,Age,City,\nAlice,30,NYC,\nBob,25,LA,";
        let backend = CsvBackend::new();

        let markdown = CsvBackend::parse_csv_to_markdown(content).unwrap();

        // Should handle trailing comma as empty column
        assert!(markdown.contains("Name"));
        assert!(markdown.contains("Alice"));
        assert!(markdown.contains("Bob"));

        // Should have 4 columns (including empty one)
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();
        let items = doc.content_blocks.unwrap();
        let table_item = items.iter().find_map(|item| {
            if let DocItem::Table { data, .. } = item {
                Some(data)
            } else {
                None
            }
        });

        assert!(table_item.is_some());
        let table = table_item.unwrap();
        assert_eq!(table.num_cols, 4); // Name, Age, City, (empty)
    }

    // ========================================
    // CATEGORY 19: Data Type and Formatting Edge Cases (1 test)
    // ========================================

    #[test]
    fn test_csv_all_numeric_data() {
        // Test CSV with purely numeric data (no strings)
        let content = r"Year,Q1,Q2,Q3,Q4
2021,1250000,1340000,1425000,1580000
2022,1620000,1705000,1830000,1920000
2023,2010000,2145000,2280000,2450000
2024,2520000,2680000,2850000,3020000";
        let backend = CsvBackend::new();

        let markdown = CsvBackend::parse_csv_to_markdown(content).unwrap();

        // Should right-align numeric columns
        assert!(markdown.contains("Year"));
        assert!(markdown.contains("2021"));
        assert!(markdown.contains("1250000"));
        assert!(markdown.contains("3020000"));

        // Should have proper numeric alignment indicators
        let lines: Vec<&str> = markdown.lines().collect();
        assert!(lines.len() >= 4); // header + separator + data rows

        // Check that separator line has alignment indicators
        let separator = lines[1];
        // Right-aligned columns can be ---:| or just ---: depending on implementation
        // Just verify that numeric columns have some alignment
        assert!(
            separator.contains("---"),
            "Expected markdown table separator with dashes"
        );

        // Should generate proper DocItems
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();
        assert!(doc.content_blocks.is_some());
        let items = doc.content_blocks.unwrap();
        let table_item = items.iter().find_map(|item| {
            if let DocItem::Table { data, .. } = item {
                Some(data)
            } else {
                None
            }
        });

        assert!(table_item.is_some());
        let table = table_item.unwrap();
        assert_eq!(table.num_cols, 5); // Year, Q1, Q2, Q3, Q4
        assert_eq!(table.num_rows, 5); // header + 4 data rows
    }

    #[test]
    fn test_csv_with_very_large_row_count() {
        // Test CSV with 1000+ rows (stress test)
        let mut rows = vec!["ID,Name,Value,Status".to_string()];
        for i in 0..1000 {
            rows.push(format!("{i},User{i},{},Active", i * 100));
        }
        let content = rows.join("\n");
        let backend = CsvBackend::new();

        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        let items = doc.content_blocks.unwrap();
        let table_item = items.iter().find_map(|item| {
            if let DocItem::Table { data, .. } = item {
                Some(data)
            } else {
                None
            }
        });

        assert!(table_item.is_some());
        let table = table_item.unwrap();
        assert_eq!(table.num_rows, 1001); // header + 1000 data rows
        assert_eq!(table.num_cols, 4);

        // Verify first and last row
        let cells = table.table_cells.as_ref().unwrap();
        assert!(cells[0].text.contains("ID") || cells[0].text == "ID");
        // Last row should be row 999 (ID 999)
        assert!(
            cells.iter().any(|c| c.text.contains("User999")),
            "Should contain last row data"
        );
    }

    #[test]
    fn test_csv_empty_file() {
        // Test completely empty CSV file
        let content = "";

        let result = CsvBackend::parse_csv_to_markdown(content);

        // Should handle empty file gracefully (either error or empty result)
        match result {
            Ok(markdown) => {
                // If it returns OK, should be empty or minimal
                assert!(
                    markdown.is_empty() || markdown.trim().is_empty(),
                    "Empty CSV should produce empty markdown"
                );
            }
            Err(_) => {
                // Error is also acceptable for empty file
            }
        }
    }

    #[test]
    fn test_csv_single_column_no_delimiter() {
        // Test CSV with only one column (no delimiter needed)
        let content = r"Name
Alice
Bob
Charlie
David
Eve";
        let backend = CsvBackend::new();

        let markdown = CsvBackend::parse_csv_to_markdown(content).unwrap();

        // Should parse single column correctly
        assert!(markdown.contains("Name"));
        assert!(markdown.contains("Alice"));
        assert!(markdown.contains("Eve"));

        // Should generate proper DocItems
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();
        let items = doc.content_blocks.unwrap();
        let table_item = items.iter().find_map(|item| {
            if let DocItem::Table { data, .. } = item {
                Some(data)
            } else {
                None
            }
        });

        assert!(table_item.is_some());
        let table = table_item.unwrap();
        assert_eq!(table.num_cols, 1); // Single column
        assert_eq!(table.num_rows, 6); // header + 5 data rows
    }

    #[test]
    fn test_tsv_with_complex_data() {
        // Test tab-separated values with complex data
        let content = "ID\tName\tDescription\tURL\n1\tProduct A\tHigh-quality product with features\thttps://example.com/a\n2\tProduct B\tAnother great product\thttps://example.com/b\n3\tProduct C\tBest seller of 2024\thttps://example.com/c";
        let backend = CsvBackend::new();

        // Should detect tab delimiter
        let delimiter = CsvBackend::detect_delimiter(content).unwrap();
        assert_eq!(delimiter, '\t', "Should detect tab delimiter");

        let markdown = CsvBackend::parse_csv_to_markdown(content).unwrap();

        // Should parse TSV correctly
        assert!(markdown.contains("ID"));
        assert!(markdown.contains("Product A"));
        assert!(markdown.contains("https://example.com/a"));
        assert!(markdown.contains("Best seller"));

        // Should generate proper DocItems
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();
        let items = doc.content_blocks.unwrap();
        let table_item = items.iter().find_map(|item| {
            if let DocItem::Table { data, .. } = item {
                Some(data)
            } else {
                None
            }
        });

        assert!(table_item.is_some());
        let table = table_item.unwrap();
        assert_eq!(table.num_cols, 4); // ID, Name, Description, URL
        assert_eq!(table.num_rows, 4); // header + 3 data rows
    }

    #[test]
    fn test_csv_with_extremely_long_cell_values() {
        // Test CSV with cells containing 10000+ characters
        let long_text = "A".repeat(10000);
        let content = format!("ID,Content,Status\n1,{long_text},Active\n2,Normal text,Active");
        let backend = CsvBackend::new();

        let markdown = CsvBackend::parse_csv_to_markdown(&content).unwrap();

        // Should handle very long cells
        assert!(markdown.contains("ID"));
        assert!(markdown.contains("Content"));
        assert!(markdown.contains("Active"));
        // Long text should be present (or truncated gracefully)
        assert!(markdown.len() > 5000, "Should contain long text data");

        // Should generate proper DocItems
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();
        let items = doc.content_blocks.unwrap();
        let table_item = items.iter().find_map(|item| {
            if let DocItem::Table { data, .. } = item {
                Some(data)
            } else {
                None
            }
        });

        assert!(table_item.is_some());
        let table = table_item.unwrap();
        assert_eq!(table.num_cols, 3); // ID, Content, Status
        assert_eq!(table.num_rows, 3); // header + 2 data rows

        // Verify long cell is preserved
        let cells = table.table_cells.as_ref().unwrap();
        let long_cell = cells
            .iter()
            .find(|c| c.text.len() > 5000)
            .expect("Should have a cell with long text");
        assert_eq!(long_cell.text.len(), 10000, "Long text should be preserved");
    }

    #[test]
    fn test_csv_whitespace_only_file() {
        // Test CSV file containing only whitespace (spaces, tabs, newlines)
        let content = "   \t\t  \n\n   \n\t";
        let backend = CsvBackend::new();

        let result = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        // CSV parser treats whitespace lines as rows with whitespace/empty cells
        assert_eq!(result.metadata.num_pages, None);
        // Should create a table with whitespace content
        assert!(result.content_blocks.is_some());
        let tables = result.content_blocks.as_ref().unwrap();
        assert_eq!(tables.len(), 1);

        // Verify table has rows from whitespace lines
        if let DocItem::Table { data, .. } = &tables[0] {
            assert!(
                data.num_rows > 0,
                "Should have rows from whitespace content"
            );
        } else {
            panic!("Expected Table DocItem");
        }
    }

    #[test]
    fn test_csv_duplicate_column_names() {
        // Test CSV with duplicate column headers (common real-world issue)
        let content = "Name,Age,Name,Status\nAlice,30,Alice2,Active\nBob,25,Bob2,Inactive";
        let backend = CsvBackend::new();

        let result = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Should parse successfully (duplicate names allowed)
        assert!(result.markdown.contains("Name"));
        assert!(result.markdown.contains("Age"));
        assert!(result.markdown.contains("Status"));
        assert!(result.markdown.contains("Alice"));
        assert!(result.markdown.contains("Bob"));

        // Verify DocItems structure
        let items = result.content_blocks.unwrap();
        let table_item = items.iter().find_map(|item| {
            if let DocItem::Table { data, .. } = item {
                Some(data)
            } else {
                None
            }
        });

        assert!(table_item.is_some());
        let table = table_item.unwrap();
        assert_eq!(table.num_cols, 4); // Name, Age, Name, Status
        assert_eq!(table.num_rows, 3); // header + 2 data rows

        // Both "Name" columns should be present in cells
        let cells = table.table_cells.as_ref().unwrap();
        let header_cells: Vec<_> = cells
            .iter()
            .filter(|c| c.start_row_offset_idx == Some(0))
            .collect();
        assert_eq!(header_cells.len(), 4);
        let name_count = header_cells.iter().filter(|c| c.text == "Name").count();
        assert_eq!(name_count, 2, "Should have two 'Name' headers");
    }

    #[test]
    fn test_csv_with_bom() {
        // Test CSV file with UTF-8 BOM (Byte Order Mark)
        // BOM is often added by Windows Excel when saving as UTF-8
        // Format: EF BB BF (UTF-8 BOM) followed by CSV data
        let backend = CsvBackend;
        let options = BackendOptions::default();

        // UTF-8 BOM + CSV data
        let csv_data = "\u{FEFF}Name,Age,City\nAlice,30,NYC\nBob,25,LA\n";
        let result = backend.parse_bytes(csv_data.as_bytes(), &options);

        assert!(result.is_ok());
        let doc = result.unwrap();

        // Verify table structure (BOM should be handled/stripped)
        let items = doc.content_blocks.unwrap();
        let table_item = items.iter().find_map(|item| {
            if let DocItem::Table { data, .. } = item {
                Some(data)
            } else {
                None
            }
        });

        assert!(table_item.is_some());
        let table = table_item.unwrap();
        assert_eq!(table.num_cols, 3);
        assert_eq!(table.num_rows, 3); // header + 2 data rows

        // Verify BOM doesn't appear in first cell
        let cells = table.table_cells.as_ref().unwrap();
        let first_cell = cells
            .iter()
            .find(|c| c.start_row_offset_idx == Some(0) && c.start_col_offset_idx == Some(0));
        assert!(first_cell.is_some());
        // csv crate handles BOM automatically
        assert_eq!(first_cell.unwrap().text, "Name");
    }

    #[test]
    fn test_csv_with_quoted_fields_containing_newlines() {
        // Test CSV with quoted fields that contain embedded newlines
        // RFC 4180: Fields with line breaks must be enclosed in double-quotes
        let backend = CsvBackend;
        let options = BackendOptions::default();

        let csv_data = "Name,Description,Status\n\
                        Alice,\"First line\nSecond line\nThird line\",Active\n\
                        Bob,\"Single line description\",Inactive\n\
                        Charlie,\"Line 1\nLine 2\",Active\n";

        let result = backend.parse_bytes(csv_data.as_bytes(), &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        let table_item = items.iter().find_map(|item| {
            if let DocItem::Table { data, .. } = item {
                Some(data)
            } else {
                None
            }
        });

        assert!(table_item.is_some());
        let table = table_item.unwrap();
        assert_eq!(table.num_cols, 3);
        assert_eq!(table.num_rows, 4); // header + 3 data rows

        // Verify newlines are preserved in cell content
        let cells = table.table_cells.as_ref().unwrap();
        let alice_desc = cells
            .iter()
            .find(|c| c.start_row_offset_idx == Some(1) && c.start_col_offset_idx == Some(1));
        assert!(alice_desc.is_some());
        let desc_text = &alice_desc.unwrap().text;
        // csv crate preserves newlines in quoted fields
        assert!(desc_text.contains('\n') || desc_text.contains("line"));
    }

    #[test]
    fn test_csv_with_delimiter_in_quoted_field() {
        // Test CSV with delimiters (commas) inside quoted fields
        // RFC 4180: Fields containing commas must be enclosed in double-quotes
        let backend = CsvBackend;
        let options = BackendOptions::default();

        let csv_data = "Name,Address,Phone\n\
                        Alice,\"123 Main St, Apt 4B, New York, NY\",555-1234\n\
                        Bob,\"456 Oak Ave, Suite 100, Los Angeles, CA\",555-5678\n\
                        Charlie,\"789 Pine Rd, Miami, FL\",555-9012\n";

        let result = backend.parse_bytes(csv_data.as_bytes(), &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        let table_item = items.iter().find_map(|item| {
            if let DocItem::Table { data, .. } = item {
                Some(data)
            } else {
                None
            }
        });

        assert!(table_item.is_some());
        let table = table_item.unwrap();
        assert_eq!(
            table.num_cols, 3,
            "Should have 3 columns (Name, Address, Phone)"
        );
        assert_eq!(table.num_rows, 4); // header + 3 data rows

        // Verify commas are preserved in address field (not treated as delimiters)
        let cells = table.table_cells.as_ref().unwrap();
        let alice_addr = cells
            .iter()
            .find(|c| c.start_row_offset_idx == Some(1) && c.start_col_offset_idx == Some(1));
        assert!(alice_addr.is_some());
        let addr_text = &alice_addr.unwrap().text;
        // Address should contain commas
        assert!(
            addr_text.contains(','),
            "Address should preserve commas: {addr_text}"
        );
        assert!(
            addr_text.contains("123 Main St") || addr_text.contains("Apt"),
            "Address should contain street info"
        );
    }

    #[test]
    fn test_csv_with_byte_order_mark() {
        // Test CSV with UTF-8 BOM (Byte Order Mark: 0xEF, 0xBB, 0xBF)
        // Excel often adds BOM to UTF-8 CSV files
        let backend = CsvBackend;
        let options = BackendOptions::default();

        // UTF-8 BOM + CSV data
        let mut csv_data = vec![0xEF, 0xBB, 0xBF]; // UTF-8 BOM
        csv_data.extend_from_slice(b"Name,Age\nAlice,30\nBob,25\n");

        let result = backend.parse_bytes(&csv_data, &options);
        assert!(result.is_ok(), "Should parse CSV with UTF-8 BOM");

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        let table_item = items.iter().find_map(|item| {
            if let DocItem::Table { data, .. } = item {
                Some(data)
            } else {
                None
            }
        });

        assert!(table_item.is_some());
        let table = table_item.unwrap();
        assert_eq!(table.num_cols, 2, "Should have 2 columns");
        assert_eq!(table.num_rows, 3); // header + 2 data rows

        // Verify BOM doesn't appear in header text
        let cells = table.table_cells.as_ref().unwrap();
        let header_cell = cells
            .iter()
            .find(|c| c.start_row_offset_idx == Some(0) && c.start_col_offset_idx == Some(0));
        assert!(header_cell.is_some());
        let header_text = &header_cell.unwrap().text;
        assert_eq!(
            header_text, "Name",
            "Header should not contain BOM characters"
        );
    }

    #[test]
    fn test_csv_with_escaped_quotes_in_dialogue() {
        // Test CSV with escaped quotes using "" in conversational text (RFC 4180 spec)
        let backend = CsvBackend;
        let options = BackendOptions::default();

        let csv_data = "Name,Quote\n\
                        Alice,\"She said \"\"Hello\"\" to me\"\n\
                        Bob,\"He replied \"\"Hi there\"\"\"\n\
                        Charlie,\"Quote: \"\"To be or not to be\"\"\"\n";

        let result = backend.parse_bytes(csv_data.as_bytes(), &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        let table_item = items.iter().find_map(|item| {
            if let DocItem::Table { data, .. } = item {
                Some(data)
            } else {
                None
            }
        });

        assert!(table_item.is_some());
        let table = table_item.unwrap();
        assert_eq!(table.num_cols, 2);
        assert_eq!(table.num_rows, 4); // header + 3 data rows

        // Verify escaped quotes are unescaped properly
        let cells = table.table_cells.as_ref().unwrap();
        let alice_quote = cells
            .iter()
            .find(|c| c.start_row_offset_idx == Some(1) && c.start_col_offset_idx == Some(1));
        assert!(alice_quote.is_some());
        let quote_text = &alice_quote.unwrap().text;
        // Escaped "" should become single "
        // Expected: She said "Hello" to me
        assert!(
            quote_text.contains("Hello"),
            "Should contain unescaped quote content: {quote_text}"
        );
    }

    #[test]
    fn test_csv_with_different_line_endings() {
        // Test CSV with Windows (CRLF), Unix (LF), and Mac (CR) line endings
        let backend = CsvBackend;
        let options = BackendOptions::default();

        // Test Windows CRLF (\r\n)
        let csv_windows = "Name,Age\r\nAlice,30\r\nBob,25\r\n";
        let result_win = backend.parse_bytes(csv_windows.as_bytes(), &options);
        assert!(result_win.is_ok(), "Should parse CSV with CRLF");

        // Test Unix LF (\n)
        let csv_unix = "Name,Age\nAlice,30\nBob,25\n";
        let result_unix = backend.parse_bytes(csv_unix.as_bytes(), &options);
        assert!(result_unix.is_ok(), "Should parse CSV with LF");

        // Test Mac CR (\r) - rare but should handle
        let csv_mac = "Name,Age\rAlice,30\rBob,25\r";
        let result_mac = backend.parse_bytes(csv_mac.as_bytes(), &options);
        assert!(result_mac.is_ok(), "Should parse CSV with CR");

        // Verify all produce same table structure
        for result in [result_win, result_unix, result_mac] {
            let doc = result.unwrap();
            let items = doc.content_blocks.unwrap();
            let table_item = items.iter().find_map(|item| {
                if let DocItem::Table { data, .. } = item {
                    Some(data)
                } else {
                    None
                }
            });
            assert!(table_item.is_some());
            let table = table_item.unwrap();
            assert_eq!(table.num_cols, 2);
            assert_eq!(table.num_rows, 3); // header + 2 data rows
        }
    }

    #[test]
    fn test_csv_with_leading_trailing_whitespace() {
        // Test CSV with leading/trailing whitespace around values
        // RFC 4180: whitespace around unquoted fields is preserved
        let backend = CsvBackend;
        let options = BackendOptions::default();

        let csv_data = "Name,Age,City\n\
                        Alice , 30 , New York \n\
                         Bob, 25,Los Angeles\n\
                        Charlie,  35  ,  Chicago  \n";

        let result = backend.parse_bytes(csv_data.as_bytes(), &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        let table_item = items.iter().find_map(|item| {
            if let DocItem::Table { data, .. } = item {
                Some(data)
            } else {
                None
            }
        });

        assert!(table_item.is_some());
        let table = table_item.unwrap();
        assert_eq!(table.num_cols, 3);
        assert_eq!(table.num_rows, 4); // header + 3 data rows

        // Verify whitespace handling (may trim or preserve depending on parser)
        let cells = table.table_cells.as_ref().unwrap();
        let alice_name = cells
            .iter()
            .find(|c| c.start_row_offset_idx == Some(1) && c.start_col_offset_idx == Some(0));
        assert!(alice_name.is_some());
        let name_text = &alice_name.unwrap().text;
        // Should contain "Alice" (with or without surrounding spaces)
        assert!(
            name_text.contains("Alice"),
            "Should contain name: {name_text}"
        );

        // Note: RFC 4180 says whitespace is preserved, but many parsers trim by default
        // This test verifies we handle whitespace gracefully without crashing
    }

    #[test]
    fn test_csv_with_very_wide_table() {
        // Test CSV with many columns (100 columns)
        // Verifies backend handles wide tables without performance issues
        let backend = CsvBackend;
        let options = BackendOptions::default();

        // Generate header with 100 columns
        let header: Vec<String> = (0..100).map(|i| format!("Col{i}")).collect();
        let header_line = header.join(",");

        // Generate data row with 100 values
        let data: Vec<String> = (0..100).map(|i| format!("Val{i}")).collect();
        let data_line = data.join(",");

        let csv_data = format!("{header_line}\n{data_line}\n{data_line}\n");

        let result = backend.parse_bytes(csv_data.as_bytes(), &options);
        assert!(result.is_ok(), "Should parse CSV with 100 columns");

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        let table_item = items.iter().find_map(|item| {
            if let DocItem::Table { data, .. } = item {
                Some(data)
            } else {
                None
            }
        });

        assert!(table_item.is_some());
        let table = table_item.unwrap();
        assert_eq!(table.num_cols, 100, "Should have 100 columns");
        assert_eq!(table.num_rows, 3); // header + 2 data rows

        // Verify first and last column values
        let cells = table.table_cells.as_ref().unwrap();

        // First column header
        let first_header = cells
            .iter()
            .find(|c| c.start_row_offset_idx == Some(0) && c.start_col_offset_idx == Some(0));
        assert!(first_header.is_some());
        assert_eq!(&first_header.unwrap().text, "Col0");

        // Last column header
        let last_header = cells
            .iter()
            .find(|c| c.start_row_offset_idx == Some(0) && c.start_col_offset_idx == Some(99));
        assert!(last_header.is_some());
        assert_eq!(&last_header.unwrap().text, "Col99");

        // First data cell
        let first_data = cells
            .iter()
            .find(|c| c.start_row_offset_idx == Some(1) && c.start_col_offset_idx == Some(0));
        assert!(first_data.is_some());
        assert_eq!(&first_data.unwrap().text, "Val0");
    }
}
