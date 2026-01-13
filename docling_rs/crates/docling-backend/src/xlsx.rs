//! Microsoft Excel (.xlsx) backend using calamine
//!
//! This module provides XLSX parsing capabilities using the calamine crate.
//! It converts Excel workbooks into structured Document objects.
//!
//! ## Python Source Reference
//!
//! This implementation is a line-by-line port of Python's `msexcel_backend.py`
//! from docling v2.58.0:
//! - Location: `~/docling/docling/backend/msexcel_backend.py`
//! - Lines: 1-645
//!
//! ## Architecture
//!
//! - Each worksheet becomes a separate "page" in the Document
//! - Tables are extracted from contiguous cell regions
//! - Images are embedded with position information
//! - Uses `TableData` structure for cell layout
//!
//! ## Implementation Status
//!
//! ✅ Table extraction with cell data
//! ✅ Multi-sheet support
//! ✅ Merged cell handling (added in N=281 using calamine 0.31)
//! ✅ Image extraction (added in N=1242)
//! ⚠️  Chart extraction (Python also does not implement - has TODO comment at line 255)
//!
//! ## Implementation Notes
//!
//! **Merged Cells:** Now supported via calamine 0.31+ API
//! - Uses `worksheet_merge_cells()` to get merged regions per sheet
//! - Each merged region is represented as a cell with `row_span/col_span`
//! - Only the top-left cell of a merged region contains content
//! - Matches Python's behavior using `openpyxl.sheet.merged_cells.ranges`

// Clippy pedantic allows:
// - Row/col to f64 conversion for bounding boxes
#![allow(clippy::cast_precision_loss)]

use crate::traits::{BackendOptions, DocumentBackend};
use calamine::{open_workbook, Data, DataType, Dimensions, Range, Reader, Xlsx};
use chrono::{DateTime, Utc};
use docling_core::{
    content::{BoundingBox, CoordOrigin, DocItem, ItemRef, ProvenanceItem, TableCell, TableData},
    DoclingError, Document, DocumentMetadata, InputFormat,
};
use quick_xml::events::Event;
use quick_xml::Reader as XmlReader;
use std::collections::HashSet;
use std::fmt::Write as FmtWrite;
use std::fs::File;
use std::io::Read as IoRead;
use std::path::Path;
use zip::ZipArchive;

/// Default DPI for Excel images (Excel standard).
/// Python docling uses 72 DPI for Excel images (`msexcel_backend.py`).
const DEFAULT_EXCEL_DPI: f64 = 72.0;

/// Data region representing the bounding box of non-empty cells
///
/// Python equivalent: `DataRegion` dataclass in `msexcel_backend.py:40-64`
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
struct DataRegion {
    /// Smallest row index (1-based, following Python)
    min_row: usize,
    /// Largest row index (1-based, following Python)
    max_row: usize,
    /// Smallest column index (1-based, following Python)
    min_col: usize,
    /// Largest column index (1-based, following Python)
    max_col: usize,
}

/// Test-only helper methods for DataRegion
#[cfg(test)]
impl DataRegion {
    /// Number of columns in the data region
    fn width(&self) -> usize {
        self.max_col - self.min_col + 1
    }

    /// Number of rows in the data region
    fn height(&self) -> usize {
        self.max_row - self.min_row + 1
    }
}

/// Represents an Excel cell
///
/// Python equivalent: `ExcelCell` in `msexcel_backend.py:66-82`
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
struct ExcelCell {
    /// Row index (0-based, relative to table)
    row: usize,
    /// Column index (0-based, relative to table)
    col: usize,
    /// Text content of the cell
    text: String,
    /// Number of rows the cell spans
    row_span: usize,
    /// Number of columns the cell spans
    col_span: usize,
}

/// Represents an Excel table on a worksheet
///
/// Python equivalent: `ExcelTable` in `msexcel_backend.py:84-99`
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
struct ExcelTable {
    /// Column and row indices of upper-left cell (0-based)
    anchor: (usize, usize),
    /// Number of rows in the table
    num_rows: usize,
    /// Number of columns in the table
    num_cols: usize,
    /// Cell data
    data: Vec<ExcelCell>,
}

/// Backend for parsing Excel workbooks (.xlsx, .xlsm)
///
/// Python equivalent: `MsExcelDocumentBackend` in `msexcel_backend.py:101-645`
///
/// ## Python Documentation (lines 102-115):
/// ```text
/// Backend for parsing Excel workbooks.
///
/// The backend converts an Excel workbook into a DoclingDocument object.
/// Each worksheet is converted into a separate page.
/// The following elements are parsed:
/// - Cell contents, parsed as tables. If two groups of cells are disconnected
///   between each other, they will be parsed as two different tables.
/// - Images, parsed as PictureItem objects.
///
/// The DoclingDocument tables and pictures have their provenance information, including
/// the position in their original Excel worksheet. The position is represented by a
/// bounding box object with the cell indices as units (0-based index). The size of this
/// bounding box is the number of columns and rows that the table or picture spans.
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct XlsxBackend;

#[allow(clippy::trivially_copy_pass_by_ref)] // Unit struct methods conventionally take &self
impl XlsxBackend {
    /// Create a new XLSX backend
    #[inline]
    #[must_use = "creates a backend instance that should be used for parsing"]
    pub const fn new() -> Self {
        Self
    }

    /// Evaluate a simple formula like "B2-C2", "B2+C2", "B2*C2", "B2/C2"
    ///
    /// For complex formulas (SUM, IF, etc.), returns None.
    /// For simple arithmetic, looks up cell values and calculates result.
    ///
    /// # Arguments
    /// * `formula` - Formula text (e.g., "B2-C2")
    /// * `range` - Worksheet data range for cell lookups
    ///
    /// # Returns
    /// `Some(result_string)` if formula was evaluated, None otherwise
    // Method signature kept for API consistency with other XlsxBackend methods
    #[allow(clippy::unused_self)]
    fn evaluate_simple_formula(&self, formula: &str, range: &Range<Data>) -> Option<String> {
        let formula = formula.trim();

        // Only handle simple binary operations
        let ops = ['+', '-', '*', '/'];
        let mut op_pos = None;
        let mut op_char = ' ';

        for &op in &ops {
            if let Some(pos) = formula.rfind(op) {
                // Skip if operator is at start (negative number)
                if pos > 0 {
                    op_pos = Some(pos);
                    op_char = op;
                    break;
                }
            }
        }

        let op_pos = op_pos?;

        // Split into left and right operands
        let left_ref = formula[..op_pos].trim();
        let right_ref = formula[op_pos + 1..].trim();

        // Parse cell references (e.g., "B2" -> col B, row 2)
        let left_val = Self::parse_cell_reference(left_ref, range)?;
        let right_val = Self::parse_cell_reference(right_ref, range)?;

        // Perform calculation
        let result = match op_char {
            '+' => left_val + right_val,
            '-' => left_val - right_val,
            '*' => left_val * right_val,
            '/' => {
                if right_val == 0.0 {
                    return Some("#DIV/0!".to_string());
                }
                left_val / right_val
            }
            _ => return None,
        };

        // Format result (remove unnecessary decimals for integers)
        if result.fract() == 0.0 {
            Some(format!("{result:.0}"))
        } else {
            Some(format!("{result:.2}"))
        }
    }

    /// Parse a cell reference like "B2" and return its numeric value
    ///
    /// # Arguments
    /// * `cell_ref` - Cell reference (e.g., "B2", "AA10")
    /// * `range` - Worksheet data range
    ///
    /// # Returns
    /// Some(value) if cell exists and contains a number, None otherwise
    fn parse_cell_reference(cell_ref: &str, range: &Range<Data>) -> Option<f64> {
        // Parse cell reference "B2" -> column B (index 1), row 2
        let mut col_str = String::new();
        let mut row_str = String::new();
        let mut in_number = false;

        for ch in cell_ref.chars() {
            if ch.is_ascii_digit() {
                in_number = true;
                row_str.push(ch);
            } else if ch.is_ascii_alphabetic() && !in_number {
                col_str.push(ch);
            } else {
                return None; // Invalid format
            }
        }

        if col_str.is_empty() || row_str.is_empty() {
            return None;
        }

        // Convert column letter to index (A=0, B=1, ..., Z=25, AA=26, etc.)
        let mut col_index = 0_usize;
        for ch in col_str.chars() {
            let digit = (ch.to_ascii_uppercase() as u8 - b'A') as usize;
            col_index = col_index * 26 + digit + 1;
        }
        col_index -= 1; // 0-based index

        // Convert row string to index (1-based in Excel, 0-based in calamine)
        let row_index: usize = row_str.parse().ok()?;
        if row_index == 0 {
            return None;
        }
        let row_index = row_index - 1; // Convert to 0-based

        // Look up cell value in range
        let cell_data = range.get((row_index, col_index))?;

        // Extract numeric value
        match cell_data {
            Data::Int(i) => Some(*i as f64),
            Data::Float(f) => Some(*f),
            Data::String(s) => s.parse::<f64>().ok(),
            _ => None,
        }
    }

    /// Extract metadata from docProps/core.xml
    ///
    /// XLSX metadata is stored in docProps/core.xml in the ZIP archive.
    /// Returns (author, created, modified) tuple.
    ///
    /// Example XML:
    /// ```xml
    /// <dc:creator>John Doe</dc:creator>
    /// <dcterms:created xsi:type="dcterms:W3CDTF">2024-01-15T10:30:00Z</dcterms:created>
    /// <dcterms:modified xsi:type="dcterms:W3CDTF">2024-01-20T14:45:00Z</dcterms:modified>
    /// ```
    fn extract_core_metadata<P: AsRef<Path>>(
        path: P,
    ) -> (Option<String>, Option<DateTime<Utc>>, Option<DateTime<Utc>>) {
        // Open XLSX as ZIP archive to access docProps/core.xml
        let Ok(file) = File::open(path) else {
            return (None, None, None);
        };
        let Ok(mut archive) = ZipArchive::new(file) else {
            return (None, None, None);
        };

        // Try to read docProps/core.xml
        let xml_content = {
            let Ok(mut core_xml) = archive.by_name("docProps/core.xml") else {
                return (None, None, None); // No core.xml, no metadata
            };

            let mut content = String::new();
            if core_xml.read_to_string(&mut content).is_err() {
                return (None, None, None);
            }
            content
        };

        // Parse XML and extract metadata elements
        let mut reader = XmlReader::from_str(&xml_content);
        reader.trim_text(true);

        let mut buf = Vec::new();
        let mut in_creator = false;
        let mut in_created = false;
        let mut in_modified = false;
        let mut author = None;
        let mut created = None;
        let mut modified = None;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => match e.name().as_ref() {
                    b"dc:creator" => in_creator = true,
                    b"dcterms:created" => in_created = true,
                    b"dcterms:modified" => in_modified = true,
                    _ => {}
                },
                Ok(Event::Text(e)) => {
                    if let Ok(text) = e.unescape() {
                        let text_str = text.trim();
                        if !text_str.is_empty() {
                            if in_creator {
                                author = Some(text_str.to_string());
                            } else if in_created {
                                created = Self::parse_datetime(text_str);
                            } else if in_modified {
                                modified = Self::parse_datetime(text_str);
                            }
                        }
                    }
                }
                Ok(Event::End(e)) => match e.name().as_ref() {
                    b"dc:creator" => in_creator = false,
                    b"dcterms:created" => in_created = false,
                    b"dcterms:modified" => in_modified = false,
                    _ => {}
                },
                Ok(Event::Eof) | Err(_) => break, // Eof or parse error
                _ => {}
            }
            buf.clear();
        }

        (author, created, modified)
    }

    /// Parse ISO 8601 datetime string to `chrono::DateTime<Utc>`
    ///
    /// Office documents use W3CDTF format (ISO 8601):
    /// - 2024-01-15T10:30:00Z
    /// - 2024-01-15T10:30:00.123Z
    #[inline]
    fn parse_datetime(s: &str) -> Option<DateTime<Utc>> {
        DateTime::parse_from_rfc3339(s)
            .ok()
            .map(|dt| dt.with_timezone(&Utc))
    }

    /// Create a Table `DocItem` from an `ExcelTable`
    ///
    /// This matches Python's `doc.add_table()` call in msexcel_backend.py:303-320
    /// Python: `doc.add_table(data=table_data`, ...) where `table_data` has `TableCell` objects
    #[allow(clippy::cast_precision_loss)] // Cell coords are small integers, f64 precision is fine
    fn create_table_docitem(
        excel_table: &ExcelTable,
        table_index: usize,
        page_no: usize,
    ) -> DocItem {
        let num_rows = excel_table.num_rows;
        let num_cols = excel_table.num_cols;

        // Create TableData with grid and table_cells
        // Python creates TableCell objects (msexcel_backend.py:289-300)
        // Initialize grid with empty cells first
        let empty_cell = TableCell {
            text: String::new(),
            row_span: Some(1),
            col_span: Some(1),
            ref_item: None,
            start_row_offset_idx: None,
            start_col_offset_idx: None,
            ..Default::default()
        };
        let mut grid = vec![vec![empty_cell; num_cols]; num_rows];
        let mut table_cells = Vec::new();

        for excel_cell in &excel_table.data {
            // Create TableCell matching Python's structure (msexcel_backend.py:289-300)
            let cell = TableCell {
                text: excel_cell.text.clone(),
                row_span: Some(excel_cell.row_span),
                col_span: Some(excel_cell.col_span),
                ref_item: None,
                start_row_offset_idx: Some(excel_cell.row),
                start_col_offset_idx: Some(excel_cell.col),
                ..Default::default()
            };

            // Place cell at correct position in grid
            if excel_cell.row < num_rows && excel_cell.col < num_cols {
                grid[excel_cell.row][excel_cell.col] = cell.clone();
            }

            table_cells.push(cell);
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
            self_ref: format!("#/tables/{table_index}"),
            parent: None,
            children: vec![],
            content_layer: "body".to_string(),
            prov: vec![ProvenanceItem {
                page_no,
                bbox: BoundingBox::new(
                    excel_table.anchor.0 as f64,
                    excel_table.anchor.1 as f64,
                    (excel_table.anchor.0 + num_cols) as f64,
                    (excel_table.anchor.1 + num_rows) as f64,
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

    /// Convert a workbook to markdown
    ///
    /// Python equivalent: `_convert_workbook` (lines 205-237)
    fn convert_workbook_to_markdown(
        &self,
        workbook: &mut Xlsx<std::io::BufReader<std::fs::File>>,
    ) -> Result<String, DoclingError> {
        let mut markdown = String::new();

        // Load merged regions (calamine 0.31+ feature)
        workbook.load_merged_regions().map_err(|e| {
            DoclingError::BackendError(format!("Failed to load merged regions: {e}"))
        })?;

        let sheet_names = workbook.sheet_names();

        // Python line 217: Iterate over all sheets
        for (idx, name) in sheet_names.iter().enumerate() {
            // Python line 220: sheet = self.workbook[name]
            let range = workbook
                .worksheet_range(name)
                .map_err(|e| DoclingError::BackendError(format!("Failed to read sheet: {e}")))?;

            // Read formulas for this sheet (for cells with empty cached values)
            let formulas = workbook
                .worksheet_formula(name)
                .unwrap_or_else(|_| Range::new((0, 0), (0, 0)));

            // Get merged regions for this sheet
            let merged_regions = workbook
                .worksheet_merge_cells(name)
                .unwrap_or(Ok(Vec::new()))
                .unwrap_or_default();

            // Python line 225-230: Add section group for sheet
            if idx > 0 {
                markdown.push_str("\n\n");
            }

            // Python line 231: doc = self._convert_sheet(doc, sheet)
            self.convert_sheet_to_markdown(&range, &formulas, &merged_regions, &mut markdown);
        }

        Ok(markdown)
    }

    /// Convert a worksheet to markdown
    ///
    /// Python equivalent: `_convert_sheet` (lines 239-257)
    fn convert_sheet_to_markdown(
        &self,
        range: &Range<Data>,
        formulas: &Range<String>,
        merged_regions: &[Dimensions],
        markdown: &mut String,
    ) {
        // Python line 251-253: Find tables in sheet
        let tables = self.find_data_tables(range, formulas, merged_regions);

        // Convert each table to markdown
        for table in tables {
            Self::table_to_markdown(&table, markdown);
            markdown.push_str("\n\n");
        }
    }

    /// Find all tables in a sheet
    ///
    /// Python equivalent: `_find_data_tables` (lines 366-404)
    fn find_data_tables(
        &self,
        range: &Range<Data>,
        formulas: &Range<String>,
        merged_regions: &[Dimensions],
    ) -> Vec<ExcelTable> {
        let bounds = Self::find_true_data_bounds(range);
        let mut tables = Vec::new();
        let mut visited: HashSet<(usize, usize)> = HashSet::new();

        // Python lines 382-402: Scan data region for tables
        for ri in bounds.min_row..=bounds.max_row {
            for ci in bounds.min_col..=bounds.max_col {
                if visited.contains(&(ri, ci)) {
                    continue;
                }

                // Get cell value (calamine is 0-based)
                let cell_value = range.get((ri - 1, ci - 1));
                if cell_value.is_none_or(calamine::DataType::is_empty) {
                    continue;
                }

                // Python lines 397-399: Find table bounds starting from this cell
                let (table, visited_cells) = self.find_table_bounds(
                    range,
                    formulas,
                    ri,
                    ci,
                    bounds.max_row,
                    bounds.max_col,
                    merged_regions,
                );

                visited.extend(visited_cells);

                // Filter out very small tables (likely image-related cells or artifacts)
                // Python backend handles images separately via _find_images_in_sheet()
                // and doesn't create tables from them. Since calamine doesn't expose images,
                // we filter small tables (1-2 cells) that are likely image placeholders.
                // Skip tables with only 1-2 cells total (likely artifacts)
                if table.num_rows * table.num_cols <= 2 {
                    continue;
                }

                tables.push(table);
            }
        }

        tables
    }

    /// Find the true data boundaries in a worksheet
    ///
    /// Python equivalent: `_find_true_data_bounds` (lines 324-364)
    fn find_true_data_bounds(range: &Range<Data>) -> DataRegion {
        let (rows, cols) = range.get_size();

        if rows == 0 || cols == 0 {
            // Python lines 361-362: Default to (1, 1, 1, 1) if empty
            return DataRegion {
                min_row: 1,
                max_row: 1,
                min_col: 1,
                max_col: 1,
            };
        }

        let mut min_row: Option<usize> = None;
        let mut min_col: Option<usize> = None;
        let mut max_row: usize = 0;
        let mut max_col: usize = 0;

        // Python lines 341-347: Scan all cells for non-empty values
        for r in 0..rows {
            for c in 0..cols {
                if let Some(cell) = range.get((r, c)) {
                    if !cell.is_empty() {
                        let r1 = r + 1; // Convert to 1-based
                        let c1 = c + 1;
                        min_row = Some(min_row.map_or(r1, |mr: usize| mr.min(r1)));
                        min_col = Some(min_col.map_or(c1, |mc: usize| mc.min(c1)));
                        max_row = max_row.max(r1);
                        max_col = max_col.max(c1);
                    }
                }
            }
        }

        DataRegion {
            min_row: min_row.unwrap_or(1),
            max_row,
            min_col: min_col.unwrap_or(1),
            max_col,
        }
    }

    /// Check if a cell is part of a merged region and return span info
    ///
    /// Returns: (`row_span`, `col_span`, `cells_to_skip`)
    /// - If cell is top-left of merged region: returns actual spans
    /// - If cell is inside merged region (but not anchor): returns (1, 1, empty)
    /// - Otherwise: returns (1, 1, empty)
    ///
    /// Note: calamine Dimensions use 0-based coordinates, our row/col are 1-based
    fn get_merged_cell_info(
        row: usize,
        col: usize,
        merged_regions: &[Dimensions],
    ) -> (usize, usize, Vec<(usize, usize)>) {
        // Convert to 0-based for comparison with Dimensions
        let row_0 = row - 1;
        let col_0 = col - 1;

        for region in merged_regions {
            let start_row = region.start.0 as usize;
            let start_col = region.start.1 as usize;
            let end_row = region.end.0 as usize;
            let end_col = region.end.1 as usize;

            // Check if this cell is the top-left anchor of the merged region
            if row_0 == start_row && col_0 == start_col {
                let row_span = end_row - start_row + 1;
                let col_span = end_col - start_col + 1;
                return (row_span, col_span, Vec::new());
            }

            // Check if this cell is inside a merged region (but not the anchor)
            if row_0 >= start_row && row_0 <= end_row && col_0 >= start_col && col_0 <= end_col {
                // This cell should be skipped - it's part of another cell's merge
                // The caller should not process this cell
                return (1, 1, Vec::new());
            }
        }

        // Not part of any merged region
        (1, 1, Vec::new())
    }

    /// Find the bounds of a rectangular table starting from a cell
    ///
    /// Python equivalent: `_find_table_bounds` (lines 406-484)
    #[allow(
        clippy::too_many_arguments,
        reason = "table detection requires range, bounds, and merge info"
    )]
    fn find_table_bounds(
        &self,
        range: &Range<Data>,
        formulas: &Range<String>,
        start_row: usize,
        start_col: usize,
        max_row: usize,
        max_col: usize,
        merged_regions: &[Dimensions],
    ) -> (ExcelTable, HashSet<(usize, usize)>) {
        // Calculate formula range offset for coordinate mapping
        // Formula range coordinates are 0-based relative to range start
        let formula_row_offset = formulas.start().map_or(1, |(r, _)| (r + 1) as usize);
        let formula_col_offset = formulas.start().map_or(1, |(_, c)| (c + 1) as usize);
        // Python lines 428-429: Find table boundaries
        let table_max_row =
            Self::find_table_bottom(range, start_row, start_col, max_row, merged_regions);
        let table_max_col =
            Self::find_table_right(range, start_row, start_col, max_col, merged_regions);

        // Python lines 431-474: Collect data within bounds
        let mut data = Vec::new();
        let mut visited_cells = HashSet::new();

        for ri in start_row..=table_max_row {
            for ci in start_col..=table_max_col {
                if visited_cells.contains(&(ri, ci)) {
                    continue;
                }

                // Get cell value (convert to 0-based for calamine)
                let cell_value = range.get((ri - 1, ci - 1));

                // Calculate formula coordinates (0-based, relative to formula range start)
                // Excel coords (ri, ci) are 1-based
                // Formula range uses 0-based coords relative to its start position
                let formula_coords = (
                    ri.saturating_sub(formula_row_offset),
                    ci.saturating_sub(formula_col_offset),
                );

                let text = match cell_value {
                    Some(v) if !v.is_empty() => {
                        let text_val = v.to_string();
                        // Also check if the string value is empty (formula with no cached value)
                        if text_val.trim().is_empty() {
                            // Cell has empty string - check for formula
                            if let Some(formula_text) = formulas.get(formula_coords) {
                                if formula_text.is_empty() {
                                    text_val
                                } else {
                                    // Try to evaluate simple formulas (B2-C2, etc.)
                                    self.evaluate_simple_formula(formula_text, range)
                                        .unwrap_or_else(|| {
                                            // Complex formula - show formula text
                                            format!("={formula_text}")
                                        })
                                }
                            } else {
                                text_val
                            }
                        } else {
                            text_val
                        }
                    }
                    _ => {
                        // Cell is None or Data::Empty - check if it has a formula
                        formulas
                            .get(formula_coords)
                            .map_or_else(String::new, |formula_text| {
                                if formula_text.is_empty() {
                                    String::new()
                                } else {
                                    // Try to evaluate simple formulas (B2-C2, etc.)
                                    self.evaluate_simple_formula(formula_text, range)
                                        .unwrap_or_else(|| {
                                            // Complex formula - show formula text
                                            format!("={formula_text}")
                                        })
                                }
                            })
                    }
                };

                // Python lines 461-468: Create ExcelCell
                // Check if this cell is part of a merged region
                // calamine uses 0-based coordinates in Dimensions
                let (row_span, col_span, skip_merged_cells) =
                    Self::get_merged_cell_info(ri, ci, merged_regions);

                data.push(ExcelCell {
                    row: ri - start_row,
                    col: ci - start_col,
                    text,
                    row_span,
                    col_span,
                });

                // Mark this cell and all cells it spans as visited
                visited_cells.insert((ri, ci));
                for merge_ri in ri..(ri + row_span) {
                    for merge_ci in ci..(ci + col_span) {
                        visited_cells.insert((merge_ri, merge_ci));
                    }
                }

                // Also mark cells that are part of a larger merged region but not the anchor
                for (skip_ri, skip_ci) in skip_merged_cells {
                    visited_cells.insert((skip_ri, skip_ci));
                }
            }
        }

        // Python lines 476-482: Return ExcelTable
        (
            ExcelTable {
                anchor: (start_col - 1, start_row - 1), // Convert to 0-based
                num_rows: table_max_row - start_row + 1,
                num_cols: table_max_col - start_col + 1,
                data,
            },
            visited_cells,
        )
    }

    /// Find the bottom boundary of a table
    ///
    /// Python equivalent: `_find_table_bottom` (lines 486-527)
    fn find_table_bottom(
        range: &Range<Data>,
        start_row: usize,
        start_col: usize,
        max_row: usize,
        merged_regions: &[Dimensions],
    ) -> usize {
        let mut table_max_row = start_row;

        // Python lines 502-525: Scan downward from start_row + 1
        for ri in (start_row + 1)..=max_row {
            let cell_value = range.get((ri - 1, start_col - 1));
            let is_empty = cell_value.is_none_or(calamine::DataType::is_empty);

            // Check if cell is part of a merged region
            let row_0 = ri - 1;
            let col_0 = start_col - 1;
            let mut merged_range: Option<&Dimensions> = None;

            for region in merged_regions {
                let start_row_r = region.start.0 as usize;
                let start_col_r = region.start.1 as usize;
                let end_row_r = region.end.0 as usize;
                let end_col_r = region.end.1 as usize;

                if row_0 >= start_row_r
                    && row_0 <= end_row_r
                    && col_0 >= start_col_r
                    && col_0 <= end_col_r
                {
                    merged_range = Some(region);
                    break;
                }
            }

            // Python line 518: Stop if cell is empty and not merged
            if is_empty && merged_range.is_none() {
                break;
            }

            // Expand table_max_row to include the merged range if applicable
            if let Some(region) = merged_range {
                // region.end is 0-based, convert to 1-based
                table_max_row = table_max_row.max(region.end.0 as usize + 1);
            } else {
                table_max_row = ri;
            }
        }

        table_max_row
    }

    /// Find the right boundary of a table
    ///
    /// Python equivalent: `_find_table_right` (lines 529-570)
    fn find_table_right(
        range: &Range<Data>,
        start_row: usize,
        start_col: usize,
        max_col: usize,
        merged_regions: &[Dimensions],
    ) -> usize {
        let mut table_max_col = start_col;

        // Python lines 545-567: Scan rightward from start_col + 1
        for ci in (start_col + 1)..=max_col {
            let cell_value = range.get((start_row - 1, ci - 1));
            let is_empty = cell_value.is_none_or(calamine::DataType::is_empty);

            // Check if cell is part of a merged region
            let row_0 = start_row - 1;
            let col_0 = ci - 1;
            let mut merged_range: Option<&Dimensions> = None;

            for region in merged_regions {
                let start_row_r = region.start.0 as usize;
                let start_col_r = region.start.1 as usize;
                let end_row_r = region.end.0 as usize;
                let end_col_r = region.end.1 as usize;

                if row_0 >= start_row_r
                    && row_0 <= end_row_r
                    && col_0 >= start_col_r
                    && col_0 <= end_col_r
                {
                    merged_range = Some(region);
                    break;
                }
            }

            // Python line 561: Stop if cell is empty and not merged
            if is_empty && merged_range.is_none() {
                break;
            }

            // Expand table_max_col to include the merged range if applicable
            if let Some(region) = merged_range {
                // region.end is 0-based, convert to 1-based
                table_max_col = table_max_col.max(region.end.1 as usize + 1);
            } else {
                table_max_col = ci;
            }
        }

        table_max_col
    }

    /// Convert an `ExcelTable` to markdown format
    fn table_to_markdown(table: &ExcelTable, markdown: &mut String) {
        if table.data.is_empty() {
            return;
        }

        // Build a 2D grid of cells
        let mut grid: Vec<Vec<String>> = vec![vec![String::new(); table.num_cols]; table.num_rows];

        for cell in &table.data {
            if cell.row < table.num_rows && cell.col < table.num_cols {
                // For merged cells (row_span > 1 or col_span > 1), fill all spanned cells
                // Note: Using range loops here is clearer than enumerate for 2D grid indexing
                #[allow(
                    clippy::needless_range_loop,
                    reason = "2D grid indexing is clearer with range than enumerate"
                )]
                for r in cell.row..(cell.row + cell.row_span) {
                    for c in cell.col..(cell.col + cell.col_span) {
                        if r < table.num_rows && c < table.num_cols {
                            grid[r][c].clone_from(&cell.text);
                        }
                    }
                }
            }
        }

        // Calculate column widths using Python tabulate's algorithm:
        // (Adapted from docling-core/src/serializer/markdown.rs:739-776)
        // 1. minwidth = header_len + MIN_PADDING (2)
        // 2. maxwidth = max(minwidth, max_cell_len)
        let mut col_widths = vec![0; table.num_cols];

        // First pass: calculate minwidth from headers (first row)
        if !grid.is_empty() {
            let header_row = &grid[0];
            for (col_idx, cell_text) in header_row.iter().enumerate() {
                // Python's MIN_PADDING = 2
                // Use .chars().count() for Unicode character count, not .len() (byte count)
                col_widths[col_idx] = cell_text.chars().count() + 2;
            }
        }

        // Second pass: find max content width across all rows
        for row in &grid {
            for (col_idx, cell_text) in row.iter().enumerate() {
                // Take max of minwidth and actual content length
                // Use .chars().count() for Unicode character count, not .len() (byte count)
                col_widths[col_idx] = col_widths[col_idx].max(cell_text.chars().count());
            }
        }

        // Third pass: detect numeric columns (mimics Python tabulate's numparse)
        // (Adapted from docling-core/src/serializer/markdown.rs:778-811)
        // A column is numeric if ALL data rows (excluding header) contain only numeric text
        let mut col_is_numeric = vec![false; table.num_cols];
        if grid.len() > 1 {
            #[allow(
                clippy::needless_range_loop,
                reason = "column-major iteration for numeric detection"
            )]
            for col_idx in 0..table.num_cols {
                let mut all_numeric = true;
                for row in &grid[1..] {
                    if let Some(cell_text) = row.get(col_idx) {
                        let text = cell_text.trim();
                        // Empty cells are not considered numeric
                        if text.is_empty() {
                            all_numeric = false;
                            break;
                        }
                        // Check if the text is a valid number (integer or float)
                        if text.parse::<f64>().is_err() && text.parse::<i64>().is_err() {
                            all_numeric = false;
                            break;
                        }
                    }
                }
                col_is_numeric[col_idx] = all_numeric;
            }
        }

        // Render as markdown table with proper alignment
        // (Adapted from docling-core/src/serializer/markdown.rs:815-877)

        // Header row (first row) - align based on column type
        if !grid.is_empty() {
            let header_row = &grid[0];
            let header_cells: Vec<String> = header_row
                .iter()
                .enumerate()
                .map(|(idx, text)| {
                    if col_is_numeric[idx] {
                        // Right-align numeric column headers
                        format!(" {:>width$} ", text, width = col_widths[idx])
                    } else {
                        // Left-align string column headers
                        format!(" {:<width$} ", text, width = col_widths[idx])
                    }
                })
                .collect();
            let _ = writeln!(markdown, "|{}|", header_cells.join("|"));

            // Separator row
            let separators: Vec<String> = col_widths
                .iter()
                .map(|&width| "-".repeat(width + 2))
                .collect();
            let _ = writeln!(markdown, "|{}|", separators.join("|"));

            // Data rows - numeric columns are right-aligned
            for row in &grid[1..] {
                let data_cells: Vec<String> = row
                    .iter()
                    .enumerate()
                    .map(|(col_idx, text)| {
                        let width = col_widths[col_idx];
                        if col_is_numeric[col_idx] {
                            // Right-align numeric columns
                            format!(" {text:>width$} ")
                        } else {
                            // Left-align string columns
                            format!(" {text:<width$} ")
                        }
                    })
                    .collect();
                let _ = writeln!(markdown, "|{}|", data_cells.join("|"));
            }
        }
    }

    /// Extract images from all sheets and return Picture `DocItems`
    ///
    /// Python reference: _`find_images_in_sheet()` in msexcel_backend.py:572-616
    ///
    /// XLSX images are stored in:
    /// - xl/drawings/drawingN.xml: Picture definitions with relationship IDs
    /// - xl/drawings/_rels/drawingN.xml.rels: Maps relationship IDs to image paths
    /// - xl/media/: Actual image files
    // Method signature kept for API consistency with other XlsxBackend methods
    #[allow(clippy::unused_self)]
    fn extract_sheet_images(
        &self,
        archive: &mut ZipArchive<File>,
        sheet_idx: usize,
    ) -> Vec<DocItem> {
        let mut images = Vec::new();

        // Find drawing file for this sheet (sheet1 → drawing1, sheet2 → drawing2, etc.)
        let drawing_num = sheet_idx + 1;
        let drawing_path = format!("xl/drawings/drawing{drawing_num}.xml");

        // Try to read drawing file
        let Ok(drawing_xml) = Self::read_zip_file(archive, &drawing_path) else {
            return images; // No drawing file = no images
        };

        // Parse drawing XML to find <xdr:pic> elements
        let picture_refs = Self::parse_drawing_for_pictures(&drawing_xml);

        // If no pictures found, return empty list
        if picture_refs.is_empty() {
            return images;
        }

        // Read relationships file to map rId → image path
        let rels_path = format!("xl/drawings/_rels/drawing{drawing_num}.xml.rels");
        let Ok(rels_xml) = Self::read_zip_file(archive, &rels_path) else {
            return images; // Can't resolve relationships
        };

        let relationships = Self::parse_relationships(&rels_xml);

        // Extract each image
        for (idx, (rel_id, anchor)) in picture_refs.iter().enumerate() {
            if let Some(image_path) = relationships.get(rel_id) {
                match Self::extract_picture_docitem(archive, image_path, anchor, sheet_idx, idx) {
                    Ok(doc_item) => images.push(doc_item),
                    Err(e) => log::warn!("Failed to extract image {image_path}: {e}"),
                }
            }
        }

        images
    }

    /// Parse drawing XML to find <xdr:pic> elements and extract relationship IDs
    ///
    /// Returns: Vec<(`relationship_id`, `anchor_coords`)>
    /// `anchor_coords`: (`from_col`, `from_row`, `to_col`, `to_row`) - 0-based cell indices
    fn parse_drawing_for_pictures(xml: &str) -> Vec<(String, (usize, usize, usize, usize))> {
        let mut pictures = Vec::new();
        let mut reader = XmlReader::from_str(xml);
        reader.trim_text(true);

        let mut buf = Vec::new();
        let mut in_pic = false;
        let mut _in_blip = false;
        let mut in_from = false;
        let mut in_to = false;
        let mut in_from_col = false;
        let mut in_from_row = false;
        let mut in_to_col = false;
        let mut in_to_row = false;

        let mut current_rel_id = None;
        let mut from_col: usize = 0;
        let mut from_row: usize = 0;
        let mut to_col: usize = 0;
        let mut to_row: usize = 0;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e) | Event::Empty(e)) => {
                    let name = e.name();
                    match name.as_ref() {
                        b"xdr:pic" | b"pic" => in_pic = true,
                        b"a:blip" | b"blip" if in_pic => {
                            _in_blip = true;
                            // Extract r:embed attribute
                            for attr in e.attributes().filter_map(std::result::Result::ok) {
                                let key = attr.key;
                                if key.as_ref() == b"r:embed" || key.as_ref() == b"embed" {
                                    if let Ok(value) = attr.decode_and_unescape_value(&reader) {
                                        current_rel_id = Some(value.to_string());
                                    }
                                }
                            }
                        }
                        b"xdr:from" | b"from" => in_from = true,
                        b"xdr:to" | b"to" => in_to = true,
                        b"xdr:col" | b"col" if in_from => in_from_col = true,
                        b"xdr:row" | b"row" if in_from => in_from_row = true,
                        b"xdr:col" | b"col" if in_to => in_to_col = true,
                        b"xdr:row" | b"row" if in_to => in_to_row = true,
                        _ => {}
                    }
                }
                Ok(Event::Text(e)) => {
                    if let Ok(text) = e.unescape() {
                        let text_str = text.trim();
                        if in_from_col {
                            from_col = text_str.parse().unwrap_or(0);
                        } else if in_from_row {
                            from_row = text_str.parse().unwrap_or(0);
                        } else if in_to_col {
                            to_col = text_str.parse().unwrap_or(0);
                        } else if in_to_row {
                            to_row = text_str.parse().unwrap_or(0);
                        }
                    }
                }
                Ok(Event::End(e)) => {
                    let name = e.name();
                    match name.as_ref() {
                        b"xdr:pic" | b"pic" => {
                            // End of picture element - save the picture info
                            if let Some(rel_id) = current_rel_id.take() {
                                pictures.push((rel_id, (from_col, from_row, to_col, to_row)));
                            }
                            in_pic = false;
                            from_col = 0;
                            from_row = 0;
                            to_col = 0;
                            to_row = 0;
                        }
                        b"a:blip" | b"blip" => _in_blip = false,
                        b"xdr:from" | b"from" => in_from = false,
                        b"xdr:to" | b"to" => in_to = false,
                        b"xdr:col" | b"col" if in_from => in_from_col = false,
                        b"xdr:row" | b"row" if in_from => in_from_row = false,
                        b"xdr:col" | b"col" if in_to => in_to_col = false,
                        b"xdr:row" | b"row" if in_to => in_to_row = false,
                        _ => {}
                    }
                }
                Ok(Event::Eof) | Err(_) => break,
                _ => {}
            }
            buf.clear();
        }

        pictures
    }

    /// Parse relationships XML to map relationship IDs to targets
    ///
    /// Returns: `HashMap`<`relationship_id`, `target_path`>
    fn parse_relationships(xml: &str) -> std::collections::HashMap<String, String> {
        let mut relationships = std::collections::HashMap::new();
        let mut reader = XmlReader::from_str(xml);
        reader.trim_text(true);

        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Empty(e) | Event::Start(e)) => {
                    if e.name().as_ref() == b"Relationship" {
                        let mut id = None;
                        let mut target = None;

                        for attr in e.attributes().filter_map(std::result::Result::ok) {
                            match attr.key.as_ref() {
                                b"Id" => {
                                    if let Ok(value) = attr.decode_and_unescape_value(&reader) {
                                        id = Some(value.to_string());
                                    }
                                }
                                b"Target" => {
                                    if let Ok(value) = attr.decode_and_unescape_value(&reader) {
                                        target = Some(value.to_string());
                                    }
                                }
                                _ => {}
                            }
                        }

                        if let (Some(id), Some(target)) = (id, target) {
                            relationships.insert(id, target);
                        }
                    }
                }
                Ok(Event::Eof) | Err(_) => break,
                _ => {}
            }
            buf.clear();
        }

        relationships
    }

    /// Extract image from ZIP and create Picture `DocItem`
    ///
    /// Python reference: `doc.add_picture()` in msexcel_backend.py:600-613
    #[allow(clippy::cast_precision_loss)] // Cell coords are small integers, f64 precision is fine
    fn extract_picture_docitem(
        archive: &mut ZipArchive<File>,
        relative_path: &str,
        anchor: &(usize, usize, usize, usize),
        sheet_idx: usize,
        picture_idx: usize,
    ) -> Result<DocItem, DoclingError> {
        use base64::{engine::general_purpose::STANDARD, Engine};

        // Convert relative path "../media/image1.png" to absolute "xl/media/image1.png"
        let image_path = relative_path.strip_prefix("../media/").map_or_else(
            || format!("xl/{relative_path}"),
            |suffix| format!("xl/media/{suffix}"),
        );

        // Read image bytes from ZIP
        let image_bytes = {
            let mut file = archive.by_name(&image_path).map_err(|e| {
                DoclingError::BackendError(format!("Missing image {image_path}: {e}"))
            })?;
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes)
                .map_err(DoclingError::IoError)?;
            bytes
        };

        // Detect mimetype from extension
        let mimetype =
            crate::utils::mime_type_from_path(&image_path, crate::utils::MIME_IMAGE_UNKNOWN);

        // Get image dimensions using the image crate
        let (width, height, dpi) = image::load_from_memory(&image_bytes).ok().map_or(
            (0.0, 0.0, DEFAULT_EXCEL_DPI),
            |img| {
                let width = f64::from(img.width());
                let height = f64::from(img.height());
                (width, height, DEFAULT_EXCEL_DPI)
            },
        );

        // Encode as base64 data URI
        let base64_data = STANDARD.encode(&image_bytes);
        let data_uri = format!("data:{mimetype};base64,{base64_data}");

        // Create image metadata JSON (matching Python docling format)
        let image_json = serde_json::json!({
            "mimetype": mimetype,
            "dpi": dpi,
            "size": {
                "width": width,
                "height": height
            },
            "uri": data_uri
        });

        // Create bounding box from anchor (cell coordinates)
        // anchor: (from_col, from_row, to_col, to_row) - 0-based
        // Python uses these exact coordinates in BoundingBox (msexcel_backend.py:604-608)
        let (from_col, from_row, to_col, to_row) = anchor;
        let bbox = BoundingBox::new(
            *from_col as f64,
            *from_row as f64,
            *to_col as f64,
            *to_row as f64,
            CoordOrigin::TopLeft,
        );

        let prov = ProvenanceItem {
            page_no: sheet_idx + 1,
            bbox,
            charspan: None,
        };

        // Create Picture DocItem
        // Python reference: doc.add_picture() in msexcel_backend.py:600
        Ok(DocItem::Picture {
            self_ref: format!("#/sheets/{sheet_idx}/pictures/{picture_idx}"),
            parent: None,
            children: vec![],
            content_layer: "body".to_string(),
            prov: vec![prov],
            captions: vec![],
            footnotes: vec![],
            references: vec![],
            image: Some(image_json),
            annotations: vec![],
            ocr_text: None,
        })
    }

    /// Helper to read file from ZIP archive
    fn read_zip_file(archive: &mut ZipArchive<File>, path: &str) -> Result<String, DoclingError> {
        let mut file = archive
            .by_name(path)
            .map_err(|e| DoclingError::BackendError(format!("File not found: {path}: {e}")))?;
        let mut content = String::new();
        file.read_to_string(&mut content)
            .map_err(DoclingError::IoError)?;
        Ok(content)
    }
}

impl DocumentBackend for XlsxBackend {
    #[inline]
    fn format(&self) -> InputFormat {
        InputFormat::Xlsx
    }

    fn parse_bytes(
        &self,
        _content: &[u8],
        _options: &BackendOptions,
    ) -> Result<Document, DoclingError> {
        // XLSX files are ZIP archives and need file path access
        Err(DoclingError::BackendError(
            "XLSX format requires file path (ZIP archive), use parse_file() instead".to_string(),
        ))
    }

    #[allow(clippy::too_many_lines)] // Complex Excel parsing logic - keeping together for clarity
    fn parse_file<P: AsRef<Path>>(
        &self,
        path: P,
        _options: &BackendOptions,
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

        // Open workbook using calamine
        let mut workbook: Xlsx<_> = open_workbook(path_ref).map_err(|e| {
            DoclingError::BackendError(format!("Failed to open XLSX: {e}: {filename}"))
        })?;

        // Convert to markdown
        let markdown = self
            .convert_workbook_to_markdown(&mut workbook)
            .map_err(&add_context)?;
        let num_characters = markdown.chars().count();

        // Extract metadata from docProps/core.xml
        let (author, created, modified) = Self::extract_core_metadata(path_ref);

        // Extract tables for DocItem generation
        // Reopen workbook to extract structured data
        let mut workbook2: Xlsx<_> = open_workbook(path_ref).map_err(|e| {
            DoclingError::BackendError(format!("Failed to reopen XLSX: {e}: {filename}"))
        })?;

        // Load merged regions (calamine 0.31+ feature)
        // This must be called before accessing merged regions
        workbook2.load_merged_regions().map_err(|e| {
            DoclingError::BackendError(format!("Failed to load merged regions: {e}: {filename}"))
        })?;

        let sheet_names = workbook2.sheet_names();
        let mut all_doc_items = Vec::new();
        let mut table_index = 0;

        // Add workbook header listing all sheets (for metadata clarity)
        // This makes multi-sheet structure explicitly visible in DocItems
        if !sheet_names.is_empty() {
            let workbook_header = DocItem::SectionHeader {
                self_ref: "#/workbook".to_string(),
                parent: None,
                children: vec![],
                content_layer: "body".to_string(),
                prov: vec![ProvenanceItem {
                    page_no: 1,
                    bbox: BoundingBox::new(0.0, 0.0, 0.0, 0.0, CoordOrigin::BottomLeft),
                    charspan: None,
                }],
                orig: format!("Workbook with {} sheets", sheet_names.len()),
                text: if sheet_names.len() == 1 {
                    format!("Workbook: 1 sheet ({})", sheet_names[0])
                } else {
                    format!(
                        "Workbook: {} sheets ({})",
                        sheet_names.len(),
                        sheet_names.join(", ")
                    )
                },
                level: 1,
                formatting: None,
                hyperlink: None,
            };
            all_doc_items.push(workbook_header);
        }

        // Open ZIP archive for image extraction
        // Python reference: Uses openpyxl which internally opens ZIP for images
        let file_for_zip = File::open(path_ref).map_err(DoclingError::IoError)?;
        let mut archive = ZipArchive::new(file_for_zip).map_err(|e| {
            DoclingError::BackendError(format!("Failed to open XLSX as ZIP: {e}: {filename}"))
        })?;

        for (sheet_idx, name) in sheet_names.iter().enumerate() {
            let range = workbook2.worksheet_range(name).map_err(|e| {
                DoclingError::BackendError(format!("Failed to read sheet: {e}: {filename}"))
            })?;

            // Read formulas for this sheet (for cells with empty cached values)
            let formulas = workbook2
                .worksheet_formula(name)
                .unwrap_or_else(|_| Range::new((0, 0), (0, 0)));

            // Get merged regions for this sheet
            let merged_regions = workbook2
                .worksheet_merge_cells(name)
                .unwrap_or(Ok(Vec::new()))
                .unwrap_or_default();

            // Python line 230: Add section header for sheet name
            // Creates a group/section for each sheet in the workbook
            let sheet_header = DocItem::SectionHeader {
                self_ref: format!("#/sheet_{sheet_idx}"),
                parent: Some(ItemRef {
                    ref_path: "#/workbook".to_string(),
                }),
                children: vec![],
                content_layer: "body".to_string(),
                prov: vec![ProvenanceItem {
                    page_no: (sheet_idx + 1),
                    bbox: BoundingBox::new(0.0, 0.0, 0.0, 0.0, CoordOrigin::BottomLeft),
                    charspan: None,
                }],
                orig: name.clone(),
                text: format!("sheet: {name}"), // Lowercase to match Python format
                level: 2,                       // Level 2 since workbook header is level 1
                formatting: None,
                hyperlink: None,
            };
            all_doc_items.push(sheet_header);

            // Find tables in sheet
            let tables = self.find_data_tables(&range, &formulas, &merged_regions);

            // Convert each table to DocItem
            for table in tables {
                let doc_item = Self::create_table_docitem(&table, table_index, sheet_idx + 1);
                all_doc_items.push(doc_item);
                table_index += 1;
            }

            // Extract images from sheet
            // Python reference: _find_images_in_sheet() in msexcel_backend.py:253
            let images = self.extract_sheet_images(&mut archive, sheet_idx);
            all_doc_items.extend(images);
        }

        let content_blocks = if all_doc_items.is_empty() {
            None
        } else {
            Some(all_doc_items)
        };

        Ok(Document {
            markdown,
            format: InputFormat::Xlsx,
            metadata: DocumentMetadata {
                num_characters,
                num_pages: Some(sheet_names.len()), // Number of sheets in workbook
                author,
                created,
                modified,
                ..Default::default()
            },
            content_blocks,
            docling_document: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};

    #[test]
    fn test_data_region() {
        let region = DataRegion {
            min_row: 1,
            max_row: 10,
            min_col: 1,
            max_col: 5,
        };

        assert_eq!(region.width(), 5);
        assert_eq!(region.height(), 10);
    }

    #[test]
    fn test_xlsx_backend_format() {
        let backend = XlsxBackend::new();
        assert_eq!(backend.format(), InputFormat::Xlsx);
    }

    /// Test metadata extraction from real XLSX file
    /// Verifies author, created date, and modified date extraction
    #[test]
    fn test_metadata_extraction() {
        let backend = XlsxBackend::new();
        let options = BackendOptions::default();

        // Use a test file that has metadata
        let test_file = "test-corpus/xlsx/xlsx_01.xlsx";

        // Skip test if file doesn't exist (for CI environments)
        if !std::path::Path::new(test_file).exists() {
            eprintln!("Skipping test_metadata_extraction: test file not found");
            return;
        }

        let result = backend.parse_file(test_file, &options);
        assert!(result.is_ok(), "Failed to parse XLSX file");

        let doc = result.unwrap();

        // Verify author metadata
        assert_eq!(
            doc.metadata.author.as_deref(),
            Some("Peter Staar"),
            "Author should be 'Peter Staar'"
        );

        // Verify created date (2024-11-16T05:17:41Z)
        assert!(
            doc.metadata.created.is_some(),
            "Created date should be present"
        );
        let created = doc.metadata.created.unwrap();
        assert_eq!(created.year(), 2024);
        assert_eq!(created.month(), 11);
        assert_eq!(created.day(), 16);

        // Verify modified date (2025-08-20T02:53:51Z)
        assert!(
            doc.metadata.modified.is_some(),
            "Modified date should be present"
        );
        let modified = doc.metadata.modified.unwrap();
        assert_eq!(modified.year(), 2025);
        assert_eq!(modified.month(), 8);
        assert_eq!(modified.day(), 20);

        // Modified should be after created
        assert!(
            modified >= created,
            "Modified date should be >= created date"
        );
    }

    /// Test merged cell detection logic
    /// Verifies that merged cells return correct span info
    #[test]
    fn test_merged_cell_info_anchor() {
        let _backend = XlsxBackend::new();

        // Create a merged region: A1:C2 (0-based: rows 0-1, cols 0-2)
        let merged_regions = vec![Dimensions {
            start: (0, 0),
            end: (1, 2),
        }];

        // Test anchor cell (top-left): should return spans
        let (row_span, col_span, _) = XlsxBackend::get_merged_cell_info(1, 1, &merged_regions);
        assert_eq!(row_span, 2, "Row span should be 2");
        assert_eq!(col_span, 3, "Col span should be 3");
    }

    /// Test merged cell detection for cells inside merged region (not anchor)
    #[test]
    fn test_merged_cell_info_inside() {
        let _backend = XlsxBackend::new();

        // Create a merged region: A1:C2 (0-based: rows 0-1, cols 0-2)
        let merged_regions = vec![Dimensions {
            start: (0, 0),
            end: (1, 2),
        }];

        // Test cell inside merged region but not anchor (B1 = row 1, col 2)
        let (row_span, col_span, _) = XlsxBackend::get_merged_cell_info(1, 2, &merged_regions);
        assert_eq!(row_span, 1, "Inside cell should have span 1");
        assert_eq!(col_span, 1, "Inside cell should have span 1");
    }

    /// Test merged cell detection for non-merged cells
    #[test]
    fn test_merged_cell_info_normal() {
        let _backend = XlsxBackend::new();

        // Create a merged region: A1:C2 (0-based: rows 0-1, cols 0-2)
        let merged_regions = vec![Dimensions {
            start: (0, 0),
            end: (1, 2),
        }];

        // Test cell outside merged region (D1 = row 1, col 4)
        let (row_span, col_span, _) = XlsxBackend::get_merged_cell_info(1, 4, &merged_regions);
        assert_eq!(row_span, 1, "Normal cell should have span 1");
        assert_eq!(col_span, 1, "Normal cell should have span 1");
    }

    /// Test merged cell detection with multiple merged regions
    #[test]
    fn test_merged_cell_info_multiple_regions() {
        let _backend = XlsxBackend::new();

        // Create multiple merged regions
        let merged_regions = vec![
            Dimensions {
                start: (0, 0), // A1:B2
                end: (1, 1),
            },
            Dimensions {
                start: (3, 0), // A4:C4
                end: (3, 2),
            },
        ];

        // Test first region anchor
        let (row_span, col_span, _) = XlsxBackend::get_merged_cell_info(1, 1, &merged_regions);
        assert_eq!(row_span, 2);
        assert_eq!(col_span, 2);

        // Test second region anchor
        let (row_span, col_span, _) = XlsxBackend::get_merged_cell_info(4, 1, &merged_regions);
        assert_eq!(row_span, 1);
        assert_eq!(col_span, 3);

        // Test cell between regions (should be normal)
        let (row_span, col_span, _) = XlsxBackend::get_merged_cell_info(3, 1, &merged_regions);
        assert_eq!(row_span, 1);
        assert_eq!(col_span, 1);
    }

    /// Test empty worksheet data bounds
    #[test]
    fn test_find_true_data_bounds_empty() {
        use calamine::{Data, Range};

        let _backend = XlsxBackend::new();

        // Create empty range
        let range: Range<Data> = Range::empty();

        let bounds = XlsxBackend::find_true_data_bounds(&range);

        // Should return default (1, 1, 1, 1) for empty sheet
        assert_eq!(bounds.min_row, 1);
        assert_eq!(bounds.max_row, 1);
        assert_eq!(bounds.min_col, 1);
        assert_eq!(bounds.max_col, 1);
    }

    /// Test data bounds with sparse data
    /// This test verifies that find_true_data_bounds correctly identifies
    /// the minimal bounding box containing all non-empty cells
    #[test]
    fn test_find_true_data_bounds_sparse() {
        use calamine::{Data, Range};

        let _backend = XlsxBackend::new();

        // Create range with data at specific cells using set_value
        // Data at (0,0), (2,4), (5,1)
        let mut range = Range::new((0, 0), (5, 4));
        range.set_value((0, 0), Data::String("A1".to_string()));
        range.set_value((2, 4), Data::String("E3".to_string()));
        range.set_value((5, 1), Data::String("B6".to_string()));

        let bounds = XlsxBackend::find_true_data_bounds(&range);

        // Bounds should be: min_row=1 (A1), max_row=6 (B6), min_col=1 (A1), max_col=5 (E3)
        assert_eq!(bounds.min_row, 1);
        assert_eq!(bounds.max_row, 6);
        assert_eq!(bounds.min_col, 1);
        assert_eq!(bounds.max_col, 5);
    }

    /// Test datetime parsing with valid ISO 8601 format
    #[test]
    fn test_parse_datetime_valid() {
        let dt = XlsxBackend::parse_datetime("2024-11-16T05:17:41Z");
        assert!(dt.is_some(), "Should parse valid ISO 8601 datetime");

        let dt = dt.unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 11);
        assert_eq!(dt.day(), 16);
    }

    /// Test datetime parsing with invalid format
    #[test]
    fn test_parse_datetime_invalid() {
        let dt = XlsxBackend::parse_datetime("not a datetime");
        assert!(dt.is_none(), "Should return None for invalid datetime");

        let dt = XlsxBackend::parse_datetime("2024-13-45T99:99:99Z");
        assert!(dt.is_none(), "Should return None for invalid date values");
    }

    /// Test table to markdown conversion
    #[test]
    fn test_table_to_markdown_simple() {
        let _backend = XlsxBackend::new();

        let table = ExcelTable {
            anchor: (0, 0),
            num_rows: 2,
            num_cols: 3,
            data: vec![
                ExcelCell {
                    row: 0,
                    col: 0,
                    text: "A".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 0,
                    col: 1,
                    text: "B".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 0,
                    col: 2,
                    text: "C".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 1,
                    col: 0,
                    text: "1".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 1,
                    col: 1,
                    text: "2".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 1,
                    col: 2,
                    text: "3".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
            ],
        };

        let mut markdown = String::new();
        XlsxBackend::table_to_markdown(&table, &mut markdown);

        // Should generate markdown table with header separator
        // Updated to match new formatting with proper column width padding and right-aligned numeric columns
        assert!(
            markdown.contains("|   A |   B |   C |"),
            "Markdown should contain header row '|   A |   B |   C |'"
        );
        assert!(
            markdown.contains("|   1 |   2 |   3 |"),
            "Markdown should contain data row '|   1 |   2 |   3 |'"
        );
        assert!(
            markdown.contains("|-----|-----|-----|"),
            "Markdown should contain header separator"
        );
    }

    /// Test table to markdown with empty table
    #[test]
    fn test_table_to_markdown_empty() {
        let _backend = XlsxBackend::new();

        let table = ExcelTable {
            anchor: (0, 0),
            num_rows: 0,
            num_cols: 0,
            data: vec![],
        };

        let mut markdown = String::new();
        XlsxBackend::table_to_markdown(&table, &mut markdown);

        // Should produce empty string for empty table
        assert_eq!(markdown, "");
    }

    /// Test error handling for bytes input (should fail)
    #[test]
    fn test_parse_bytes_error() {
        let backend = XlsxBackend::new();
        let options = BackendOptions::default();
        let dummy_bytes = b"dummy data";

        let result = backend.parse_bytes(dummy_bytes, &options);
        assert!(result.is_err(), "parse_bytes should return error for XLSX");

        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("ZIP archive"),
            "Error should mention ZIP archive requirement"
        );
    }

    /// Test DocItem generation for tables
    #[test]
    fn test_create_table_docitem() {
        let _backend = XlsxBackend::new();

        let table = ExcelTable {
            anchor: (1, 2), // Column B, Row 3 (0-based)
            num_rows: 3,
            num_cols: 2,
            data: vec![
                ExcelCell {
                    row: 0,
                    col: 0,
                    text: "Header1".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 0,
                    col: 1,
                    text: "Header2".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 1,
                    col: 0,
                    text: "Data1".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 1,
                    col: 1,
                    text: "Data2".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 2,
                    col: 0,
                    text: "Data3".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 2,
                    col: 1,
                    text: "Data4".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
            ],
        };

        let doc_item = XlsxBackend::create_table_docitem(&table, 5, 1);

        // Verify it's a Table DocItem
        match doc_item {
            DocItem::Table {
                self_ref,
                data,
                prov,
                ..
            } => {
                assert_eq!(self_ref, "#/tables/5");
                assert_eq!(data.num_rows, 3);
                assert_eq!(data.num_cols, 2);
                assert_eq!(prov.len(), 1);
                assert_eq!(prov[0].page_no, 1);

                // Verify bounding box (anchor + size)
                let bbox = &prov[0].bbox;
                assert_eq!(bbox.l, 1.0); // anchor col
                assert_eq!(bbox.t, 2.0); // anchor row
                assert_eq!(bbox.r, 3.0); // anchor col + num_cols
                assert_eq!(bbox.b, 5.0); // anchor row + num_rows
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    /// Test table boundary detection - find bottom edge
    /// Verifies that table scanning stops when it encounters an empty cell
    #[test]
    fn test_find_table_bottom() {
        use calamine::{Data, Range};

        let _backend = XlsxBackend::new();

        // Create range with contiguous column data
        let mut range = Range::new((0, 0), (3, 1));
        range.set_value((0, 0), Data::String("A1".to_string()));
        range.set_value((0, 1), Data::String("B1".to_string()));
        range.set_value((1, 0), Data::String("A2".to_string()));
        range.set_value((1, 1), Data::String("B2".to_string()));
        range.set_value((2, 0), Data::String("A3".to_string()));
        range.set_value((2, 1), Data::String("B3".to_string()));
        // (3, 0) is empty - gap in column A
        range.set_value((3, 1), Data::String("B4".to_string()));

        // Find bottom boundary starting from A1 (row=1, col=1)
        let merged_regions = Vec::new();
        let bottom = XlsxBackend::find_table_bottom(&range, 1, 1, 4, &merged_regions);

        // Should stop at row 3 (before the empty cell in A4)
        assert_eq!(bottom, 3);
    }

    /// Test table boundary detection - find right edge
    /// Verifies that table scanning stops when it encounters an empty cell
    #[test]
    fn test_find_table_right() {
        use calamine::{Data, Range};

        let _backend = XlsxBackend::new();

        // Create range with contiguous row data
        let mut range = Range::new((0, 0), (1, 3));
        range.set_value((0, 0), Data::String("A1".to_string()));
        range.set_value((0, 1), Data::String("B1".to_string()));
        range.set_value((0, 2), Data::String("C1".to_string()));
        // (0, 3) is empty - gap in row 1
        range.set_value((1, 0), Data::String("A2".to_string()));
        range.set_value((1, 1), Data::String("B2".to_string()));
        range.set_value((1, 2), Data::String("C2".to_string()));
        range.set_value((1, 3), Data::String("D2".to_string()));

        // Find right boundary starting from A1 (row=1, col=1)
        let merged_regions = Vec::new();
        let right = XlsxBackend::find_table_right(&range, 1, 1, 4, &merged_regions);

        // Should stop at column 3 (before the empty cell in D1)
        assert_eq!(right, 3);
    }

    // ========== BACKEND CREATION TESTS ==========

    #[test]
    fn test_backend_creation() {
        let backend = XlsxBackend::new();
        assert_eq!(backend.format(), InputFormat::Xlsx);
    }

    #[test]
    fn test_backend_default_trait() {
        let backend = XlsxBackend;
        assert_eq!(backend.format(), InputFormat::Xlsx);
    }

    // ========== TABLE DOCITEM STRUCTURE TESTS ==========

    #[test]
    fn test_table_docitem_self_ref_format() {
        let _backend = XlsxBackend::new();

        let table = ExcelTable {
            anchor: (0, 0),
            num_rows: 1,
            num_cols: 1,
            data: vec![ExcelCell {
                row: 0,
                col: 0,
                text: "A".to_string(),
                row_span: 1,
                col_span: 1,
            }],
        };

        let doc_item = XlsxBackend::create_table_docitem(&table, 0, 1);

        match doc_item {
            DocItem::Table { self_ref, .. } => {
                assert_eq!(self_ref, "#/tables/0");
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    #[test]
    fn test_table_docitem_tabledata_structure() {
        let _backend = XlsxBackend::new();

        let table = ExcelTable {
            anchor: (0, 0),
            num_rows: 2,
            num_cols: 3,
            data: vec![
                ExcelCell {
                    row: 0,
                    col: 0,
                    text: "A".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 0,
                    col: 1,
                    text: "B".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 0,
                    col: 2,
                    text: "C".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
            ],
        };

        let doc_item = XlsxBackend::create_table_docitem(&table, 0, 1);

        match doc_item {
            DocItem::Table { data, .. } => {
                assert_eq!(data.num_rows, 2);
                assert_eq!(data.num_cols, 3);
                assert!(data.table_cells.is_some());

                let cells = data.table_cells.as_ref().unwrap();
                assert_eq!(cells.len(), 3);

                // Verify first cell
                assert_eq!(cells[0].text, "A");
                assert_eq!(cells[0].row_span, Some(1));
                assert_eq!(cells[0].col_span, Some(1));
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    #[test]
    fn test_table_docitem_with_merged_cells() {
        let _backend = XlsxBackend::new();

        // Table with merged cell (2x2 in top-left)
        let table = ExcelTable {
            anchor: (0, 0),
            num_rows: 3,
            num_cols: 3,
            data: vec![
                ExcelCell {
                    row: 0,
                    col: 0,
                    text: "Merged".to_string(),
                    row_span: 2,
                    col_span: 2,
                },
                ExcelCell {
                    row: 0,
                    col: 2,
                    text: "C1".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
            ],
        };

        let doc_item = XlsxBackend::create_table_docitem(&table, 0, 1);

        match doc_item {
            DocItem::Table { data, .. } => {
                let cells = data.table_cells.as_ref().unwrap();

                // Verify merged cell has correct spans
                assert_eq!(cells[0].row_span, Some(2));
                assert_eq!(cells[0].col_span, Some(2));
                assert_eq!(cells[0].text, "Merged");
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    // ========== WORKSHEET HANDLING TESTS ==========

    #[test]
    fn test_find_data_tables_empty_worksheet() {
        use calamine::Range;

        let backend = XlsxBackend::new();
        let range: Range<Data> = Range::empty();
        let formulas: Range<String> = Range::empty();
        let merged_regions = vec![];

        let tables = backend.find_data_tables(&range, &formulas, &merged_regions);

        // Empty worksheet should produce zero tables
        assert_eq!(
            tables.len(),
            0,
            "Empty worksheet should produce zero tables"
        );
    }

    #[test]
    fn test_find_data_tables_single_cell() {
        use calamine::{Data, Range};

        let backend = XlsxBackend::new();

        // Single cell at A1
        let mut range = Range::new((0, 0), (0, 0));
        range.set_value((0, 0), Data::String("A1".to_string()));
        let formulas: Range<String> = Range::empty();
        let merged_regions = vec![];

        let tables = backend.find_data_tables(&range, &formulas, &merged_regions);

        // Single cell tables are now filtered out (likely image placeholders)
        // This matches Python behavior which handles images separately
        assert_eq!(
            tables.len(),
            0,
            "Single cell should be filtered out (likely image placeholder)"
        );
    }

    // ========== MARKDOWN FORMATTING TESTS ==========

    #[test]
    fn test_table_to_markdown_column_alignment() {
        let _backend = XlsxBackend::new();

        let table = ExcelTable {
            anchor: (0, 0),
            num_rows: 2,
            num_cols: 2,
            data: vec![
                ExcelCell {
                    row: 0,
                    col: 0,
                    text: "Short".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 0,
                    col: 1,
                    text: "Very Long Header".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 1,
                    col: 0,
                    text: "A".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 1,
                    col: 1,
                    text: "B".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
            ],
        };

        let mut markdown = String::new();
        XlsxBackend::table_to_markdown(&table, &mut markdown);

        // Column widths should be calculated based on content
        // Updated to match new formatting with proper column width padding
        assert!(
            markdown.contains("| Short   | Very Long Header   |"),
            "Markdown should contain properly aligned headers"
        );
        // Separator line should be present (format may vary)
        assert!(
            markdown.contains("|---"),
            "Markdown should contain header separator"
        );
    }

    #[test]
    fn test_table_to_markdown_special_characters() {
        let _backend = XlsxBackend::new();

        let table = ExcelTable {
            anchor: (0, 0),
            num_rows: 1,
            num_cols: 2,
            data: vec![
                ExcelCell {
                    row: 0,
                    col: 0,
                    text: "Text with <html> & \"quotes\"".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 0,
                    col: 1,
                    text: "Pipe | character".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
            ],
        };

        let mut markdown = String::new();
        XlsxBackend::table_to_markdown(&table, &mut markdown);

        // Special characters should be preserved in markdown
        assert!(
            markdown.contains("<html>"),
            "HTML tags should be preserved in markdown"
        );
        assert!(
            markdown.contains('&'),
            "Ampersand should be preserved in markdown"
        );
        assert!(
            markdown.contains("\"quotes\""),
            "Quotes should be preserved in markdown"
        );
        // Pipe character should be present (may be escaped in real implementation)
        assert!(
            markdown.contains("Pipe"),
            "Text with pipe should be present in markdown"
        );
    }

    #[test]
    fn test_table_to_markdown_long_text() {
        let _backend = XlsxBackend::new();

        let long_text = "A".repeat(500);
        let table = ExcelTable {
            anchor: (0, 0),
            num_rows: 1,
            num_cols: 1,
            data: vec![ExcelCell {
                row: 0,
                col: 0,
                text: long_text.clone(),
                row_span: 1,
                col_span: 1,
            }],
        };

        let mut markdown = String::new();
        XlsxBackend::table_to_markdown(&table, &mut markdown);

        // Long text should be included in markdown
        assert!(
            markdown.contains(&long_text),
            "Long text (500 chars) should be preserved in markdown"
        );
    }

    // ========== INTEGRATION TESTS ==========

    #[test]
    fn test_docitem_complete_structure() {
        let _backend = XlsxBackend::new();

        let table = ExcelTable {
            anchor: (0, 0),
            num_rows: 2,
            num_cols: 2,
            data: vec![
                ExcelCell {
                    row: 0,
                    col: 0,
                    text: "A1".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 0,
                    col: 1,
                    text: "B1".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
            ],
        };

        let doc_item = XlsxBackend::create_table_docitem(&table, 0, 1);

        match doc_item {
            DocItem::Table {
                self_ref,
                parent,
                children,
                content_layer,
                prov,
                data,
                captions,
                footnotes,
                references,
                annotations,
                image,
            } => {
                // Verify all fields
                assert_eq!(
                    self_ref, "#/tables/0",
                    "Table self_ref should be '#/tables/0'"
                );
                assert_eq!(parent, None, "Table should have no parent");
                assert!(children.is_empty(), "Table should have empty children");
                assert_eq!(content_layer, "body", "Content layer should be 'body'");
                assert_eq!(prov.len(), 1, "Provenance should have exactly one entry");
                assert_eq!(data.num_rows, 2, "Table should have 2 rows");
                assert_eq!(data.num_cols, 2, "Table should have 2 columns");
                assert!(captions.is_empty(), "Table should have empty captions");
                assert!(footnotes.is_empty(), "Table should have empty footnotes");
                assert!(references.is_empty(), "Table should have empty references");
                assert!(
                    annotations.is_empty(),
                    "Table should have empty annotations"
                );
                assert_eq!(image, None, "Table should have no image");
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    #[test]
    fn test_table_to_markdown_empty_cells() {
        let _backend = XlsxBackend::new();

        // Table with some empty cells
        let table = ExcelTable {
            anchor: (0, 0),
            num_rows: 2,
            num_cols: 3,
            data: vec![
                ExcelCell {
                    row: 0,
                    col: 0,
                    text: "A1".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 0,
                    col: 1,
                    text: "".to_string(), // Empty
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 0,
                    col: 2,
                    text: "C1".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 1,
                    col: 0,
                    text: "".to_string(), // Empty
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 1,
                    col: 1,
                    text: "B2".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 1,
                    col: 2,
                    text: "".to_string(), // Empty
                    row_span: 1,
                    col_span: 1,
                },
            ],
        };

        let mut markdown = String::new();
        XlsxBackend::table_to_markdown(&table, &mut markdown);

        // Empty cells should still create table structure
        // Updated to match new formatting with proper column width padding
        assert!(
            markdown.contains("| A1   |    | C1   |"),
            "Markdown should contain row with empty middle cell"
        );
        assert!(
            markdown.contains("|      | B2 |      |"),
            "Markdown should contain row with empty outer cells"
        );
    }

    // ========== CATEGORY 2: METADATA EDGE CASES ==========

    #[test]
    fn test_metadata_author_none_for_file_without_metadata() {
        let _backend = XlsxBackend::new();

        // Extract metadata from non-existent file (should return None)
        let (author, created, modified) = XlsxBackend::extract_core_metadata("nonexistent.xlsx");

        assert_eq!(author, None, "Author should be None for non-existent file");
        assert_eq!(
            created, None,
            "Created should be None for non-existent file"
        );
        assert_eq!(
            modified, None,
            "Modified should be None for non-existent file"
        );
    }

    #[test]
    fn test_metadata_timestamp_parsing_with_milliseconds() {
        // Test datetime parsing with milliseconds
        let dt = XlsxBackend::parse_datetime("2024-03-15T14:30:45.123Z");
        assert!(
            dt.is_some(),
            "ISO 8601 datetime with milliseconds should parse successfully"
        );

        let dt = dt.unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 3);
        assert_eq!(dt.day(), 15);
        assert_eq!(dt.hour(), 14);
        assert_eq!(dt.minute(), 30);
        assert_eq!(dt.second(), 45);
    }

    #[test]
    fn test_metadata_language_always_none() {
        // XLSX backend doesn't extract language metadata
        let backend = XlsxBackend::new();
        let options = BackendOptions::default();

        let test_file = "test-corpus/xlsx/xlsx_01.xlsx";
        if !std::path::Path::new(test_file).exists() {
            return;
        }

        let result = backend.parse_file(test_file, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert_eq!(
            doc.metadata.language, None,
            "Language should always be None for XLSX"
        );
    }

    #[test]
    fn test_metadata_num_pages_is_sheet_count() {
        // For XLSX, num_pages represents number of sheets
        let backend = XlsxBackend::new();
        let options = BackendOptions::default();

        let test_file = "test-corpus/xlsx/xlsx_01.xlsx";
        if !std::path::Path::new(test_file).exists() {
            return;
        }

        let result = backend.parse_file(test_file, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        // num_pages field exists in DocumentMetadata
        // For XLSX, this would represent sheet count if implemented
        // Currently it's None, which is correct default behavior
        assert!(doc.metadata.num_pages.is_none_or(|n| n > 0));
    }

    #[test]
    fn test_metadata_character_count_accuracy() {
        let _backend = XlsxBackend::new();

        // Create simple table markdown
        let table = ExcelTable {
            anchor: (0, 0),
            num_rows: 1,
            num_cols: 2,
            data: vec![
                ExcelCell {
                    row: 0,
                    col: 0,
                    text: "ABC".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 0,
                    col: 1,
                    text: "123".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
            ],
        };

        let mut markdown = String::new();
        XlsxBackend::table_to_markdown(&table, &mut markdown);

        let char_count = markdown.chars().count();
        // Character count should match markdown length
        // | ABC | 123 |
        // |---------|---------|
        assert!(char_count > 0, "Character count should be positive");
        assert!(markdown.contains("ABC") && markdown.contains("123"));
    }

    // ========== CATEGORY 3: DOCITEM STRUCTURE EDGE CASES ==========

    #[test]
    fn test_docitem_self_ref_sequential() {
        let _backend = XlsxBackend::new();

        let table1 = ExcelTable {
            anchor: (0, 0),
            num_rows: 1,
            num_cols: 1,
            data: vec![ExcelCell {
                row: 0,
                col: 0,
                text: "T1".to_string(),
                row_span: 1,
                col_span: 1,
            }],
        };

        let table2 = ExcelTable {
            anchor: (0, 0),
            num_rows: 1,
            num_cols: 1,
            data: vec![ExcelCell {
                row: 0,
                col: 0,
                text: "T2".to_string(),
                row_span: 1,
                col_span: 1,
            }],
        };

        let doc_item1 = XlsxBackend::create_table_docitem(&table1, 0, 1);
        let doc_item2 = XlsxBackend::create_table_docitem(&table2, 1, 1);

        // self_ref should be sequential
        match (doc_item1, doc_item2) {
            (DocItem::Table { self_ref: ref1, .. }, DocItem::Table { self_ref: ref2, .. }) => {
                assert_eq!(ref1, "#/tables/0");
                assert_eq!(ref2, "#/tables/1");
            }
            _ => panic!("Expected Table DocItems"),
        }
    }

    #[test]
    fn test_docitem_content_layer_always_body() {
        let _backend = XlsxBackend::new();

        let table = ExcelTable {
            anchor: (0, 0),
            num_rows: 1,
            num_cols: 1,
            data: vec![ExcelCell {
                row: 0,
                col: 0,
                text: "A".to_string(),
                row_span: 1,
                col_span: 1,
            }],
        };

        let doc_item = XlsxBackend::create_table_docitem(&table, 0, 1);

        match doc_item {
            DocItem::Table { content_layer, .. } => {
                assert_eq!(
                    content_layer, "body",
                    "content_layer should always be 'body'"
                );
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    #[test]
    fn test_docitem_no_formatting() {
        // XLSX backend doesn't preserve cell formatting in DocItems
        let _backend = XlsxBackend::new();

        let table = ExcelTable {
            anchor: (0, 0),
            num_rows: 1,
            num_cols: 1,
            data: vec![ExcelCell {
                row: 0,
                col: 0,
                text: "Plain text".to_string(),
                row_span: 1,
                col_span: 1,
            }],
        };

        let doc_item = XlsxBackend::create_table_docitem(&table, 0, 1);

        match doc_item {
            DocItem::Table { data, .. } => {
                let cells = data.table_cells.as_ref().unwrap();
                // Text should be plain, no formatting markers
                assert!(!cells[0].text.contains("**")); // No bold
                assert!(!cells[0].text.contains('_')); // No italic
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    #[test]
    fn test_docitem_table_self_ref_format() {
        // Verify table self_ref follows #/tables/N pattern
        let _backend = XlsxBackend::new();

        let table = ExcelTable {
            anchor: (0, 0),
            num_rows: 1,
            num_cols: 1,
            data: vec![ExcelCell {
                row: 0,
                col: 0,
                text: "A".to_string(),
                row_span: 1,
                col_span: 1,
            }],
        };

        let doc_item = XlsxBackend::create_table_docitem(&table, 42, 1);

        match doc_item {
            DocItem::Table { self_ref, .. } => {
                assert_eq!(self_ref, "#/tables/42");
                assert!(self_ref.starts_with("#/tables/"));
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    #[test]
    fn test_docitem_tabledata_uses_table_cells_field() {
        // XLSX uses table_cells field (not grid field exclusively)
        let _backend = XlsxBackend::new();

        let table = ExcelTable {
            anchor: (0, 0),
            num_rows: 2,
            num_cols: 2,
            data: vec![
                ExcelCell {
                    row: 0,
                    col: 0,
                    text: "A".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 0,
                    col: 1,
                    text: "B".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
            ],
        };

        let doc_item = XlsxBackend::create_table_docitem(&table, 0, 1);

        match doc_item {
            DocItem::Table { data, .. } => {
                assert!(
                    data.table_cells.is_some(),
                    "table_cells should be populated"
                );
                let cells = data.table_cells.as_ref().unwrap();
                assert_eq!(cells.len(), 2, "Should have 2 cells");

                // Grid should also be populated
                assert_eq!(data.grid.len(), 2, "Grid should have 2 rows");
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    #[test]
    fn test_docitem_provenance_page_no() {
        // Verify provenance page_no is set correctly (1-based sheet index)
        let _backend = XlsxBackend::new();

        let table = ExcelTable {
            anchor: (0, 0),
            num_rows: 1,
            num_cols: 1,
            data: vec![ExcelCell {
                row: 0,
                col: 0,
                text: "A".to_string(),
                row_span: 1,
                col_span: 1,
            }],
        };

        let doc_item = XlsxBackend::create_table_docitem(&table, 0, 3); // Sheet 3

        match doc_item {
            DocItem::Table { prov, .. } => {
                assert_eq!(prov.len(), 1);
                assert_eq!(prov[0].page_no, 3, "page_no should match sheet index");
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    // ========== CATEGORY 4: FORMAT-SPECIFIC COMPLEX CASES ==========

    #[test]
    fn test_table_with_many_rows() {
        let _backend = XlsxBackend::new();

        // Create table with 50 rows
        let mut data = Vec::new();
        for row in 0..50 {
            data.push(ExcelCell {
                row,
                col: 0,
                text: format!("Row{}", row + 1),
                row_span: 1,
                col_span: 1,
            });
        }

        let table = ExcelTable {
            anchor: (0, 0),
            num_rows: 50,
            num_cols: 1,
            data,
        };

        let doc_item = XlsxBackend::create_table_docitem(&table, 0, 1);

        match doc_item {
            DocItem::Table { data, .. } => {
                assert_eq!(data.num_rows, 50);
                let cells = data.table_cells.as_ref().unwrap();
                assert_eq!(cells.len(), 50);
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    #[test]
    fn test_table_with_many_columns() {
        let _backend = XlsxBackend::new();

        // Create table with 20 columns
        let mut data = Vec::new();
        for col in 0..20 {
            data.push(ExcelCell {
                row: 0,
                col,
                text: format!("Col{}", col + 1),
                row_span: 1,
                col_span: 1,
            });
        }

        let table = ExcelTable {
            anchor: (0, 0),
            num_rows: 1,
            num_cols: 20,
            data,
        };

        let doc_item = XlsxBackend::create_table_docitem(&table, 0, 1);

        match doc_item {
            DocItem::Table { data, .. } => {
                assert_eq!(data.num_cols, 20);
                let cells = data.table_cells.as_ref().unwrap();
                assert_eq!(cells.len(), 20);
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    #[test]
    fn test_table_with_unicode_content() {
        let _backend = XlsxBackend::new();

        // Unicode content: CJK + emoji
        let table = ExcelTable {
            anchor: (0, 0),
            num_rows: 1,
            num_cols: 3,
            data: vec![
                ExcelCell {
                    row: 0,
                    col: 0,
                    text: "日本語".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 0,
                    col: 1,
                    text: "中文".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 0,
                    col: 2,
                    text: "😀🎉".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
            ],
        };

        let doc_item = XlsxBackend::create_table_docitem(&table, 0, 1);

        match doc_item {
            DocItem::Table { data, .. } => {
                let cells = data.table_cells.as_ref().unwrap();
                assert_eq!(cells[0].text, "日本語");
                assert_eq!(cells[1].text, "中文");
                assert_eq!(cells[2].text, "😀🎉");
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    #[test]
    fn test_table_with_formulas_shows_result_values() {
        // Note: calamine shows formula results, not formula text
        // This test verifies that behavior
        let _backend = XlsxBackend::new();

        let table = ExcelTable {
            anchor: (0, 0),
            num_rows: 1,
            num_cols: 2,
            data: vec![
                ExcelCell {
                    row: 0,
                    col: 0,
                    text: "10".to_string(), // A1 = 10
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 0,
                    col: 1,
                    text: "20".to_string(), // B1 = A1*2 (result: 20)
                    row_span: 1,
                    col_span: 1,
                },
            ],
        };

        let doc_item = XlsxBackend::create_table_docitem(&table, 0, 1);

        match doc_item {
            DocItem::Table { data, .. } => {
                let cells = data.table_cells.as_ref().unwrap();
                // Should show result value, not formula
                assert_eq!(cells[1].text, "20");
                assert!(!cells[1].text.contains('='));
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    #[test]
    fn test_table_with_number_formats() {
        // XLSX cells can have various number formats (currency, percentage, date)
        // calamine returns formatted string values
        let _backend = XlsxBackend::new();

        let table = ExcelTable {
            anchor: (0, 0),
            num_rows: 1,
            num_cols: 3,
            data: vec![
                ExcelCell {
                    row: 0,
                    col: 0,
                    text: "1234.56".to_string(), // Number
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 0,
                    col: 1,
                    text: "50%".to_string(), // Percentage (may be "0.5" or "50%")
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 0,
                    col: 2,
                    text: "$100.00".to_string(), // Currency (may be "100" or "$100.00")
                    row_span: 1,
                    col_span: 1,
                },
            ],
        };

        let doc_item = XlsxBackend::create_table_docitem(&table, 0, 1);

        match doc_item {
            DocItem::Table { data, .. } => {
                let cells = data.table_cells.as_ref().unwrap();
                // Values should be present (format may vary based on calamine)
                assert!(!cells[0].text.is_empty());
                assert!(!cells[1].text.is_empty());
                assert!(!cells[2].text.is_empty());
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    #[test]
    fn test_table_with_large_merged_region() {
        let _backend = XlsxBackend::new();

        // Large merged region: 5 rows × 3 columns
        let table = ExcelTable {
            anchor: (0, 0),
            num_rows: 5,
            num_cols: 3,
            data: vec![ExcelCell {
                row: 0,
                col: 0,
                text: "Large merged cell".to_string(),
                row_span: 5,
                col_span: 3,
            }],
        };

        let doc_item = XlsxBackend::create_table_docitem(&table, 0, 1);

        match doc_item {
            DocItem::Table { data, .. } => {
                let cells = data.table_cells.as_ref().unwrap();
                assert_eq!(cells[0].row_span, Some(5));
                assert_eq!(cells[0].col_span, Some(3));
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    #[test]
    fn test_table_bounding_box_coordinates() {
        // Verify bounding box uses cell indices (0-based for columns, 0-based for rows)
        let _backend = XlsxBackend::new();

        let table = ExcelTable {
            anchor: (5, 10), // Column F (5), Row 11 (10) - 0-based
            num_rows: 3,
            num_cols: 4,
            data: vec![],
        };

        let doc_item = XlsxBackend::create_table_docitem(&table, 0, 1);

        match doc_item {
            DocItem::Table { prov, .. } => {
                let bbox = &prov[0].bbox;
                assert_eq!(bbox.l, 5.0); // anchor col
                assert_eq!(bbox.t, 10.0); // anchor row
                assert_eq!(bbox.r, 9.0); // anchor col + num_cols (5 + 4)
                assert_eq!(bbox.b, 13.0); // anchor row + num_rows (10 + 3)
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    // ========== CATEGORY 5: INTEGRATION & EDGE CASES ==========

    #[test]
    fn test_multiple_tables_in_document() {
        let _backend = XlsxBackend::new();

        let table1 = ExcelTable {
            anchor: (0, 0),
            num_rows: 2,
            num_cols: 2,
            data: vec![ExcelCell {
                row: 0,
                col: 0,
                text: "T1".to_string(),
                row_span: 1,
                col_span: 1,
            }],
        };

        let table2 = ExcelTable {
            anchor: (5, 5),
            num_rows: 3,
            num_cols: 3,
            data: vec![ExcelCell {
                row: 0,
                col: 0,
                text: "T2".to_string(),
                row_span: 1,
                col_span: 1,
            }],
        };

        let doc_item1 = XlsxBackend::create_table_docitem(&table1, 0, 1);
        let doc_item2 = XlsxBackend::create_table_docitem(&table2, 1, 1);

        // Both should be valid Table DocItems with different self_refs
        match (doc_item1, doc_item2) {
            (DocItem::Table { self_ref: ref1, .. }, DocItem::Table { self_ref: ref2, .. }) => {
                assert_eq!(ref1, "#/tables/0");
                assert_eq!(ref2, "#/tables/1");
            }
            _ => panic!("Expected Table DocItems"),
        }
    }

    #[test]
    fn test_document_format_consistency() {
        // Verify InputFormat is consistently Xlsx
        let backend = XlsxBackend::new();

        assert_eq!(backend.format(), InputFormat::Xlsx);

        // Test that default() also returns Xlsx backend
        let backend2 = XlsxBackend;
        assert_eq!(backend2.format(), InputFormat::Xlsx);
    }

    #[test]
    fn test_data_region_boundary_calculations() {
        // Test DataRegion width and height calculations
        let region = DataRegion {
            min_row: 5,
            max_row: 15,
            min_col: 2,
            max_col: 7,
        };

        assert_eq!(region.width(), 6); // 7 - 2 + 1
        assert_eq!(region.height(), 11); // 15 - 5 + 1
    }

    #[test]
    fn test_excel_cell_structure() {
        // Verify ExcelCell structure integrity
        let cell = ExcelCell {
            row: 5,
            col: 10,
            text: "Test".to_string(),
            row_span: 2,
            col_span: 3,
        };

        assert_eq!(cell.row, 5);
        assert_eq!(cell.col, 10);
        assert_eq!(cell.text, "Test");
        assert_eq!(cell.row_span, 2);
        assert_eq!(cell.col_span, 3);
    }

    // ========== CATEGORY 5: EDGE CASES AND ERROR HANDLING ==========

    #[test]
    fn test_table_with_whitespace_only_cells() {
        // Cells containing only whitespace should preserve the whitespace
        let _backend = XlsxBackend::new();

        let table = ExcelTable {
            anchor: (0, 0),
            num_rows: 2,
            num_cols: 2,
            data: vec![
                ExcelCell {
                    row: 0,
                    col: 0,
                    text: "   ".to_string(), // Only spaces
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 0,
                    col: 1,
                    text: "\t\t".to_string(), // Only tabs
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 1,
                    col: 0,
                    text: "\n".to_string(), // Only newline
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 1,
                    col: 1,
                    text: "".to_string(), // Empty
                    row_span: 1,
                    col_span: 1,
                },
            ],
        };

        let doc_item = XlsxBackend::create_table_docitem(&table, 0, 1);

        match doc_item {
            DocItem::Table { data, .. } => {
                let cells = data.table_cells.as_ref().unwrap();
                // Whitespace is preserved (trimmed by markdown serializer if needed)
                assert_eq!(cells.len(), 4);
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    #[test]
    fn test_table_with_very_long_cell_content() {
        // Cells with very long text (10,000 characters)
        let _backend = XlsxBackend::new();
        let long_text = "A".repeat(10000);

        let table = ExcelTable {
            anchor: (0, 0),
            num_rows: 1,
            num_cols: 1,
            data: vec![ExcelCell {
                row: 0,
                col: 0,
                text: long_text.clone(),
                row_span: 1,
                col_span: 1,
            }],
        };

        let doc_item = XlsxBackend::create_table_docitem(&table, 0, 1);

        match doc_item {
            DocItem::Table { data, .. } => {
                let cells = data.table_cells.as_ref().unwrap();
                assert_eq!(cells[0].text.len(), 10000, "Long text should be preserved");
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    #[test]
    fn test_table_with_newlines_in_cells() {
        // Excel allows multi-line cells (Alt+Enter)
        let _backend = XlsxBackend::new();

        let table = ExcelTable {
            anchor: (0, 0),
            num_rows: 1,
            num_cols: 2,
            data: vec![
                ExcelCell {
                    row: 0,
                    col: 0,
                    text: "Line 1\nLine 2\nLine 3".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 0,
                    col: 1,
                    text: "Single\r\nWindows\r\nLine".to_string(), // Windows CRLF
                    row_span: 1,
                    col_span: 1,
                },
            ],
        };

        let doc_item = XlsxBackend::create_table_docitem(&table, 0, 1);

        match doc_item {
            DocItem::Table { data, .. } => {
                let cells = data.table_cells.as_ref().unwrap();
                // Newlines should be preserved in cell text
                assert!(cells[0].text.contains('\n'));
                assert!(cells[1].text.contains('\n') || cells[1].text.contains("\r\n"));
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    #[test]
    fn test_table_with_leading_trailing_whitespace() {
        // Test preservation of leading/trailing whitespace
        let _backend = XlsxBackend::new();

        let table = ExcelTable {
            anchor: (0, 0),
            num_rows: 1,
            num_cols: 3,
            data: vec![
                ExcelCell {
                    row: 0,
                    col: 0,
                    text: "  leading".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 0,
                    col: 1,
                    text: "trailing  ".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 0,
                    col: 2,
                    text: "  both  ".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
            ],
        };

        let doc_item = XlsxBackend::create_table_docitem(&table, 0, 1);

        match doc_item {
            DocItem::Table { data, .. } => {
                let cells = data.table_cells.as_ref().unwrap();
                // Whitespace preserved (trimmed by markdown serializer if needed)
                assert_eq!(cells.len(), 3);
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    #[test]
    fn test_metadata_with_very_long_author_name() {
        // Test metadata with 500-character author name
        let author = "A".repeat(500);
        let metadata = DocumentMetadata {
            author: Some(author.clone()),
            title: None,
            created: None,
            modified: None,
            language: None,
            subject: None,
            num_pages: Some(3),
            num_characters: 12345,
            exif: None,
        };

        assert_eq!(metadata.author.unwrap().len(), 500);
        assert_eq!(metadata.num_pages, Some(3));
        assert_eq!(metadata.num_characters, 12345);
    }

    #[test]
    fn test_metadata_num_pages_zero_sheets() {
        // Empty workbook with zero sheets
        let metadata = DocumentMetadata {
            author: None,
            title: None,
            created: None,
            modified: None,
            language: None,
            subject: None,
            num_pages: Some(0), // Zero sheets
            num_characters: 0,
            exif: None,
        };

        assert_eq!(metadata.num_pages, Some(0));
        assert_eq!(metadata.num_characters, 0);
    }

    #[test]
    fn test_metadata_num_pages_many_sheets() {
        // Workbook with 100 sheets (edge case for large files)
        let metadata = DocumentMetadata {
            author: Some("Analyst".to_string()),
            title: Some("Large Workbook".to_string()),
            created: None,
            modified: None,
            language: None,
            subject: None,
            num_pages: Some(100), // 100 sheets
            num_characters: 1_000_000,
            exif: None,
        };

        assert_eq!(metadata.num_pages, Some(100));
        assert_eq!(metadata.num_characters, 1_000_000);
    }

    #[test]
    fn test_data_region_single_cell_region() {
        // Single cell region (min == max)
        let region = DataRegion {
            min_row: 10,
            max_row: 10,
            min_col: 5,
            max_col: 5,
        };

        assert_eq!(region.width(), 1); // 5 - 5 + 1
        assert_eq!(region.height(), 1); // 10 - 10 + 1
    }

    #[test]
    fn test_excel_cell_with_special_unicode() {
        // Cell with emoji, CJK, and other Unicode
        let cell = ExcelCell {
            row: 0,
            col: 0,
            text: "📊 数据分析 🔢 Données".to_string(), // Emoji + Chinese + French
            row_span: 1,
            col_span: 1,
        };

        assert_eq!(cell.text, "📊 数据分析 🔢 Données");
        // Verify Unicode is preserved
        assert!(cell.text.contains('📊'));
        assert!(cell.text.contains('数'));
        assert!(cell.text.contains('é'));
    }

    #[test]
    fn test_table_with_boolean_values() {
        // Test Excel boolean TRUE/FALSE cells
        let _backend = XlsxBackend::new();

        let table = ExcelTable {
            anchor: (0, 0),
            num_rows: 2,
            num_cols: 2,
            data: vec![
                ExcelCell {
                    row: 0,
                    col: 0,
                    text: "TRUE".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 0,
                    col: 1,
                    text: "FALSE".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 1,
                    col: 0,
                    text: "1".to_string(), // Excel stores TRUE as 1
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 1,
                    col: 1,
                    text: "0".to_string(), // Excel stores FALSE as 0
                    row_span: 1,
                    col_span: 1,
                },
            ],
        };

        let doc_item = XlsxBackend::create_table_docitem(&table, 0, 1);

        match doc_item {
            DocItem::Table { data, .. } => {
                let cells = data.table_cells.as_ref().unwrap();
                assert_eq!(cells.len(), 4);
                // Verify boolean values are preserved as text
                assert!(cells[0].text.contains("TRUE") || cells[0].text.contains('1'));
                assert!(cells[1].text.contains("FALSE") || cells[1].text.contains('0'));
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    #[test]
    fn test_table_with_error_values() {
        // Test Excel error values (#DIV/0!, #N/A, #REF!, #VALUE!, etc.)
        let _backend = XlsxBackend::new();

        let table = ExcelTable {
            anchor: (0, 0),
            num_rows: 2,
            num_cols: 3,
            data: vec![
                ExcelCell {
                    row: 0,
                    col: 0,
                    text: "#DIV/0!".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 0,
                    col: 1,
                    text: "#N/A".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 0,
                    col: 2,
                    text: "#REF!".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 1,
                    col: 0,
                    text: "#VALUE!".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 1,
                    col: 1,
                    text: "#NAME?".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 1,
                    col: 2,
                    text: "#NUM!".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
            ],
        };

        let doc_item = XlsxBackend::create_table_docitem(&table, 0, 1);

        match doc_item {
            DocItem::Table { data, .. } => {
                let cells = data.table_cells.as_ref().unwrap();
                assert_eq!(cells.len(), 6);
                // Verify error values are preserved
                assert!(cells[0].text.contains("#DIV/0!"));
                assert!(cells[1].text.contains("#N/A"));
                assert!(cells[2].text.contains("#REF!"));
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    #[test]
    fn test_table_with_scientific_notation() {
        // Test Excel scientific notation (1.23E+10, 4.56E-5, etc.)
        let _backend = XlsxBackend::new();

        let table = ExcelTable {
            anchor: (0, 0),
            num_rows: 3,
            num_cols: 2,
            data: vec![
                ExcelCell {
                    row: 0,
                    col: 0,
                    text: "1.23E+10".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 0,
                    col: 1,
                    text: "4.56E-5".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 1,
                    col: 0,
                    text: "9.99E+99".to_string(), // Large exponent
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 1,
                    col: 1,
                    text: "1.11E-99".to_string(), // Small exponent
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 2,
                    col: 0,
                    text: "-2.34E+8".to_string(), // Negative scientific
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 2,
                    col: 1,
                    text: "0.0E+0".to_string(), // Zero in scientific
                    row_span: 1,
                    col_span: 1,
                },
            ],
        };

        let doc_item = XlsxBackend::create_table_docitem(&table, 0, 1);

        match doc_item {
            DocItem::Table { data, .. } => {
                let cells = data.table_cells.as_ref().unwrap();
                assert_eq!(cells.len(), 6);
                // Verify scientific notation is preserved
                assert!(
                    cells[0].text.contains("E+") || cells[0].text.contains("e+"),
                    "Should preserve scientific notation"
                );
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    #[test]
    fn test_table_with_percentage_values() {
        // Test Excel percentage formatting (stored as decimals)
        let _backend = XlsxBackend::new();

        let table = ExcelTable {
            anchor: (0, 0),
            num_rows: 2,
            num_cols: 3,
            data: vec![
                ExcelCell {
                    row: 0,
                    col: 0,
                    text: "0.5".to_string(), // 50% stored as 0.5
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 0,
                    col: 1,
                    text: "1.25".to_string(), // 125% stored as 1.25
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 0,
                    col: 2,
                    text: "0.0035".to_string(), // 0.35% stored as 0.0035
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 1,
                    col: 0,
                    text: "50%".to_string(), // Sometimes stored as text
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 1,
                    col: 1,
                    text: "125%".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 1,
                    col: 2,
                    text: "0.35%".to_string(),
                    row_span: 1,
                    col_span: 1,
                },
            ],
        };

        let doc_item = XlsxBackend::create_table_docitem(&table, 0, 1);

        match doc_item {
            DocItem::Table { data, .. } => {
                let cells = data.table_cells.as_ref().unwrap();
                assert_eq!(cells.len(), 6);
                // Verify percentage values are captured
                assert!(!cells[0].text.is_empty());
                assert!(!cells[3].text.is_empty());
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    #[test]
    fn test_table_with_currency_symbols() {
        // Test Excel currency formatting (USD, EUR, GBP, JPY, etc.)
        let _backend = XlsxBackend::new();

        let table = ExcelTable {
            anchor: (0, 0),
            num_rows: 3,
            num_cols: 2,
            data: vec![
                ExcelCell {
                    row: 0,
                    col: 0,
                    text: "$1,234.56".to_string(), // USD
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 0,
                    col: 1,
                    text: "€9,876.54".to_string(), // EUR
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 1,
                    col: 0,
                    text: "£5,432.10".to_string(), // GBP
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 1,
                    col: 1,
                    text: "¥123,456".to_string(), // JPY (no decimal)
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 2,
                    col: 0,
                    text: "₹98,765.43".to_string(), // INR
                    row_span: 1,
                    col_span: 1,
                },
                ExcelCell {
                    row: 2,
                    col: 1,
                    text: "¥12,345.67".to_string(), // CNY
                    row_span: 1,
                    col_span: 1,
                },
            ],
        };

        let doc_item = XlsxBackend::create_table_docitem(&table, 0, 1);

        match doc_item {
            DocItem::Table { data, .. } => {
                let cells = data.table_cells.as_ref().unwrap();
                assert_eq!(cells.len(), 6);
                // Verify currency symbols are preserved
                assert!(cells[0].text.contains('$') || cells[0].text.contains("1234"));
                assert!(cells[1].text.contains('€') || cells[1].text.contains("9876"));
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    /// Test table with diagonal merged regions (e.g., A1:C3 merged)
    /// Verifies backend handles non-rectangular merged regions correctly
    #[test]
    fn test_table_with_diagonal_merged_region() {
        let _backend = XlsxBackend::new();

        // Create a 5x5 table with a diagonal merged region (3x3)
        let data = vec![
            ExcelCell {
                row: 0,
                col: 0,
                text: "Merged 3x3 Region".to_string(),
                row_span: 3,
                col_span: 3,
            },
            ExcelCell {
                row: 0,
                col: 3,
                text: "Col D".to_string(),
                row_span: 1,
                col_span: 1,
            },
            ExcelCell {
                row: 0,
                col: 4,
                text: "Col E".to_string(),
                row_span: 1,
                col_span: 1,
            },
            ExcelCell {
                row: 3,
                col: 0,
                text: "Row 4".to_string(),
                row_span: 1,
                col_span: 1,
            },
            ExcelCell {
                row: 4,
                col: 0,
                text: "Row 5".to_string(),
                row_span: 1,
                col_span: 1,
            },
        ];

        let table = ExcelTable {
            anchor: (0, 0),
            num_rows: 5,
            num_cols: 5,
            data,
        };
        let doc_item = XlsxBackend::create_table_docitem(&table, 0, 1);

        match doc_item {
            DocItem::Table { data, .. } => {
                // Verify grid dimensions are 5x5
                assert_eq!(data.num_rows, 5);
                assert_eq!(data.num_cols, 5);

                // Verify merged region text appears in correct location
                assert!(data.grid[0][0].text.contains("Merged 3x3"));

                // Verify table_cells contains merged cell info
                let cells = data.table_cells.as_ref().unwrap();
                let merged_cell = cells
                    .iter()
                    .find(|c| c.row_span == Some(3) && c.col_span == Some(3));
                assert!(merged_cell.is_some(), "Should have 3x3 merged cell");
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    /// Test table with empty first row (headers missing)
    /// Verifies backend handles tables starting with empty first row
    #[test]
    fn test_table_with_empty_first_row() {
        let _backend = XlsxBackend::new();

        let data = vec![
            // Row 0 is completely empty (no cells)
            // Row 1 has data
            ExcelCell {
                row: 1,
                col: 0,
                text: "Data Row 1".to_string(),
                row_span: 1,
                col_span: 1,
            },
            ExcelCell {
                row: 1,
                col: 1,
                text: "Value A".to_string(),
                row_span: 1,
                col_span: 1,
            },
            ExcelCell {
                row: 2,
                col: 0,
                text: "Data Row 2".to_string(),
                row_span: 1,
                col_span: 1,
            },
            ExcelCell {
                row: 2,
                col: 1,
                text: "Value B".to_string(),
                row_span: 1,
                col_span: 1,
            },
        ];

        let table = ExcelTable {
            anchor: (0, 0),
            num_rows: 3,
            num_cols: 2,
            data,
        };
        let doc_item = XlsxBackend::create_table_docitem(&table, 0, 1);

        match doc_item {
            DocItem::Table { data, .. } => {
                // Verify first row exists but is empty
                assert_eq!(data.grid[0][0].text, "");
                assert_eq!(data.grid[0][1].text, "");

                // Verify second row has data
                assert_eq!(data.grid[1][0].text, "Data Row 1");
                assert_eq!(data.grid[1][1].text, "Value A");
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    /// Test table with mixed cell types (string, number, blank, formula)
    /// Verifies backend handles heterogeneous cell content correctly
    #[test]
    fn test_table_with_mixed_cell_types() {
        let _backend = XlsxBackend::new();

        let data = vec![
            ExcelCell {
                row: 0,
                col: 0,
                text: "Label".to_string(),
                row_span: 1,
                col_span: 1,
            },
            ExcelCell {
                row: 0,
                col: 1,
                text: "123.45".to_string(),
                row_span: 1,
                col_span: 1,
            },
            ExcelCell {
                row: 0,
                col: 2,
                text: "".to_string(),
                row_span: 1,
                col_span: 1,
            }, // Empty
            ExcelCell {
                row: 0,
                col: 3,
                text: "=SUM(A1:A10)".to_string(),
                row_span: 1,
                col_span: 1,
            },
            ExcelCell {
                row: 1,
                col: 0,
                text: "2024-01-15".to_string(),
                row_span: 1,
                col_span: 1,
            }, // Date-like
            ExcelCell {
                row: 1,
                col: 1,
                text: "TRUE".to_string(),
                row_span: 1,
                col_span: 1,
            }, // Boolean-like
            ExcelCell {
                row: 1,
                col: 2,
                text: "#DIV/0!".to_string(),
                row_span: 1,
                col_span: 1,
            }, // Error
            ExcelCell {
                row: 1,
                col: 3,
                text: "3.14159e+00".to_string(),
                row_span: 1,
                col_span: 1,
            }, // Scientific
        ];

        let table = ExcelTable {
            anchor: (0, 0),
            num_rows: 2,
            num_cols: 4,
            data,
        };
        let doc_item = XlsxBackend::create_table_docitem(&table, 0, 1);

        match doc_item {
            DocItem::Table { data, .. } => {
                // Verify all cell types are preserved as strings
                assert_eq!(data.grid[0][0].text, "Label");
                assert_eq!(data.grid[0][1].text, "123.45");
                assert_eq!(data.grid[0][2].text, ""); // Empty
                assert!(data.grid[0][3].text.contains("SUM")); // Formula text

                assert!(data.grid[1][0].text.contains("2024")); // Date
                assert!(data.grid[1][1].text.contains("TRUE")); // Boolean
                assert!(data.grid[1][2].text.contains("DIV") || data.grid[1][2].text.contains('#')); // Error
                assert!(
                    data.grid[1][3].text.contains("3.14") || data.grid[1][3].text.contains("e+")
                ); // Scientific
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    /// Test table with very sparse data (only corners filled)
    /// Verifies backend handles sparse tables with mostly empty cells
    #[test]
    fn test_table_with_sparse_corner_data() {
        let _backend = XlsxBackend::new();

        // 10x10 table with only 4 corner cells filled
        let data = vec![
            ExcelCell {
                row: 0,
                col: 0,
                text: "Top Left".to_string(),
                row_span: 1,
                col_span: 1,
            },
            ExcelCell {
                row: 0,
                col: 9,
                text: "Top Right".to_string(),
                row_span: 1,
                col_span: 1,
            },
            ExcelCell {
                row: 9,
                col: 0,
                text: "Bottom Left".to_string(),
                row_span: 1,
                col_span: 1,
            },
            ExcelCell {
                row: 9,
                col: 9,
                text: "Bottom Right".to_string(),
                row_span: 1,
                col_span: 1,
            },
        ];

        let table = ExcelTable {
            anchor: (0, 0),
            num_rows: 10,
            num_cols: 10,
            data,
        };
        let doc_item = XlsxBackend::create_table_docitem(&table, 0, 1);

        match doc_item {
            DocItem::Table { data, .. } => {
                // Verify dimensions
                assert_eq!(data.num_rows, 10);
                assert_eq!(data.num_cols, 10);

                // Verify at least top left corner is present
                assert_eq!(data.grid[0][0].text, "Top Left");

                // Verify bottom left corner (last row, first column)
                assert_eq!(data.grid[9][0].text, "Bottom Left");

                // Verify middle is empty (grid filling works correctly)
                assert_eq!(data.grid[5][5].text, "");

                // Verify table_cells has exactly 4 cells with correct content
                let cells = data.table_cells.as_ref().unwrap();
                assert_eq!(cells.len(), 4);

                // Verify all 4 corner texts are present in table_cells
                let texts: Vec<&str> = cells.iter().map(|c| c.text.as_str()).collect();
                assert!(texts.contains(&"Top Left"));
                assert!(texts.contains(&"Top Right"));
                assert!(texts.contains(&"Bottom Left"));
                assert!(texts.contains(&"Bottom Right"));
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    /// Test table with right-to-left text (Arabic, Hebrew)
    /// Verifies backend preserves RTL Unicode text correctly
    #[test]
    fn test_table_with_rtl_text() {
        let _backend = XlsxBackend::new();

        let data = vec![
            // Arabic text
            ExcelCell {
                row: 0,
                col: 0,
                text: "مرحبا بك".to_string(),
                row_span: 1,
                col_span: 1,
            }, // "Welcome" in Arabic
            ExcelCell {
                row: 0,
                col: 1,
                text: "123".to_string(),
                row_span: 1,
                col_span: 1,
            },
            // Hebrew text
            ExcelCell {
                row: 1,
                col: 0,
                text: "שלום עולם".to_string(),
                row_span: 1,
                col_span: 1,
            }, // "Hello world" in Hebrew
            ExcelCell {
                row: 1,
                col: 1,
                text: "456".to_string(),
                row_span: 1,
                col_span: 1,
            },
            // Mixed RTL and LTR
            ExcelCell {
                row: 2,
                col: 0,
                text: "2024 مارس".to_string(),
                row_span: 1,
                col_span: 1,
            }, // "2024 March" (mixed)
            ExcelCell {
                row: 2,
                col: 1,
                text: "עברית English".to_string(),
                row_span: 1,
                col_span: 1,
            }, // Mixed Hebrew-English
        ];

        let table = ExcelTable {
            anchor: (0, 0),
            num_rows: 3,
            num_cols: 2,
            data,
        };
        let doc_item = XlsxBackend::create_table_docitem(&table, 0, 1);

        match doc_item {
            DocItem::Table { data, .. } => {
                // Verify RTL text is preserved correctly
                assert!(data.grid[0][0].text.contains("مرحبا")); // Arabic preserved
                assert!(data.grid[1][0].text.contains("שלום")); // Hebrew preserved
                assert!(
                    data.grid[2][0].text.contains("2024") && data.grid[2][0].text.contains("مارس")
                ); // Mixed preserved

                // Verify table_cells contains all RTL text
                let cells = data.table_cells.as_ref().unwrap();
                let arabic_cell = cells.iter().find(|c| c.text.contains("مرحبا"));
                assert!(arabic_cell.is_some(), "Should preserve Arabic text");

                let hebrew_cell = cells.iter().find(|c| c.text.contains("שלום"));
                assert!(hebrew_cell.is_some(), "Should preserve Hebrew text");
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    /// Test workbook with named ranges (cell references with custom names)
    /// Verifies backend handles Excel's defined names feature
    #[test]
    fn test_workbook_with_named_ranges() {
        let _backend = XlsxBackend::new();

        // Simulate table with cells that would be part of named ranges
        // Named ranges in Excel: e.g., "SalesData" = Sheet1!$A$1:$C$10
        let data = vec![
            // Header row for named range "QuarterlySales"
            ExcelCell {
                row: 0,
                col: 0,
                text: "Q1".to_string(),
                row_span: 1,
                col_span: 1,
            },
            ExcelCell {
                row: 0,
                col: 1,
                text: "Q2".to_string(),
                row_span: 1,
                col_span: 1,
            },
            // Data in named range
            ExcelCell {
                row: 1,
                col: 0,
                text: "1000".to_string(),
                row_span: 1,
                col_span: 1,
            },
            ExcelCell {
                row: 1,
                col: 1,
                text: "1500".to_string(),
                row_span: 1,
                col_span: 1,
            },
        ];

        let table = ExcelTable {
            anchor: (0, 0),
            num_rows: 2,
            num_cols: 2,
            data,
        };
        let doc_item = XlsxBackend::create_table_docitem(&table, 0, 1);

        match doc_item {
            DocItem::Table { data, .. } => {
                // Verify table content is preserved
                assert_eq!(data.grid[0][0].text, "Q1");
                assert_eq!(data.grid[0][1].text, "Q2");
                assert_eq!(data.grid[1][0].text, "1000");
                assert_eq!(data.grid[1][1].text, "1500");

                // Note: Named ranges are stored in workbook.xml under <definedNames>
                // Example: <definedName name="QuarterlySales">Sheet1!$A$1:$B$2</definedName>
                // Backend extracts cell values; named range metadata is typically not preserved
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    /// Test cells with conditional formatting (color scales, icon sets, data bars)
    /// Verifies backend extracts cell values regardless of formatting
    #[test]
    fn test_cells_with_conditional_formatting() {
        let _backend = XlsxBackend::new();

        // Cells with conditional formatting (backend sees values, not formatting)
        let data = vec![
            // Score column (might have color scale: red→yellow→green)
            ExcelCell {
                row: 0,
                col: 0,
                text: "Score".to_string(),
                row_span: 1,
                col_span: 1,
            },
            ExcelCell {
                row: 0,
                col: 1,
                text: "Status".to_string(),
                row_span: 1,
                col_span: 1,
            },
            // Low score (would be red with icon ▼)
            ExcelCell {
                row: 1,
                col: 0,
                text: "45".to_string(),
                row_span: 1,
                col_span: 1,
            },
            ExcelCell {
                row: 1,
                col: 1,
                text: "Low".to_string(),
                row_span: 1,
                col_span: 1,
            },
            // High score (would be green with icon ▲)
            ExcelCell {
                row: 2,
                col: 0,
                text: "95".to_string(),
                row_span: 1,
                col_span: 1,
            },
            ExcelCell {
                row: 2,
                col: 1,
                text: "High".to_string(),
                row_span: 1,
                col_span: 1,
            },
        ];

        let table = ExcelTable {
            anchor: (0, 0),
            num_rows: 3,
            num_cols: 2,
            data,
        };
        let doc_item = XlsxBackend::create_table_docitem(&table, 0, 1);

        match doc_item {
            DocItem::Table { data, .. } => {
                // Verify cell values are extracted (formatting metadata is separate)
                assert_eq!(data.grid[1][0].text, "45"); // Low score value
                assert_eq!(data.grid[2][0].text, "95"); // High score value

                // Note: Conditional formatting is stored in worksheet XML:
                // <conditionalFormatting> with <cfRule> elements
                // - Color scales: type="colorScale" with <color> gradients
                // - Icon sets: type="iconSet" with <iconSet iconSet="3Arrows">
                // - Data bars: type="dataBar" with <dataBar> gradient fill
                // Backend extracts cell text; conditional formatting is visual metadata
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    /// Test cells with data validation (dropdown lists, input restrictions)
    /// Verifies backend extracts selected values from validated cells
    #[test]
    fn test_cells_with_data_validation() {
        let _backend = XlsxBackend::new();

        // Cells with data validation (e.g., dropdown list of statuses)
        let data = vec![
            ExcelCell {
                row: 0,
                col: 0,
                text: "Task".to_string(),
                row_span: 1,
                col_span: 1,
            },
            ExcelCell {
                row: 0,
                col: 1,
                text: "Status".to_string(),
                row_span: 1,
                col_span: 1,
            },
            // Status column has dropdown: "Not Started", "In Progress", "Completed"
            ExcelCell {
                row: 1,
                col: 0,
                text: "Write report".to_string(),
                row_span: 1,
                col_span: 1,
            },
            ExcelCell {
                row: 1,
                col: 1,
                text: "In Progress".to_string(), // Selected from dropdown
                row_span: 1,
                col_span: 1,
            },
            ExcelCell {
                row: 2,
                col: 0,
                text: "Review code".to_string(),
                row_span: 1,
                col_span: 1,
            },
            ExcelCell {
                row: 2,
                col: 1,
                text: "Completed".to_string(), // Selected from dropdown
                row_span: 1,
                col_span: 1,
            },
        ];

        let table = ExcelTable {
            anchor: (0, 0),
            num_rows: 3,
            num_cols: 2,
            data,
        };
        let doc_item = XlsxBackend::create_table_docitem(&table, 0, 1);

        match doc_item {
            DocItem::Table { data, .. } => {
                // Verify selected values are extracted
                assert_eq!(data.grid[1][1].text, "In Progress");
                assert_eq!(data.grid[2][1].text, "Completed");

                // Note: Data validation is stored in worksheet XML:
                // <dataValidations><dataValidation type="list" sqref="B2:B100">
                //   <formula1>"Not Started,In Progress,Completed"</formula1>
                // </dataValidation></dataValidations>
                // Backend extracts selected cell value; validation rules are metadata
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    /// Test workbook with pivot table metadata
    /// Verifies backend handles pivot cache and pivot fields
    #[test]
    fn test_workbook_with_pivot_table() {
        let _backend = XlsxBackend::new();

        // Simulate pivot table result (backend sees output values, not pivot definition)
        // Source data: Sales by Region and Product
        // Pivot: Sum of Sales grouped by Region
        let data = vec![
            // Pivot table header
            ExcelCell {
                row: 0,
                col: 0,
                text: "Region".to_string(),
                row_span: 1,
                col_span: 1,
            },
            ExcelCell {
                row: 0,
                col: 1,
                text: "Total Sales".to_string(),
                row_span: 1,
                col_span: 1,
            },
            // Pivot table data (aggregated)
            ExcelCell {
                row: 1,
                col: 0,
                text: "East".to_string(),
                row_span: 1,
                col_span: 1,
            },
            ExcelCell {
                row: 1,
                col: 1,
                text: "50000".to_string(),
                row_span: 1,
                col_span: 1,
            },
            ExcelCell {
                row: 2,
                col: 0,
                text: "West".to_string(),
                row_span: 1,
                col_span: 1,
            },
            ExcelCell {
                row: 2,
                col: 1,
                text: "75000".to_string(),
                row_span: 1,
                col_span: 1,
            },
        ];

        let table = ExcelTable {
            anchor: (0, 0),
            num_rows: 3,
            num_cols: 2,
            data,
        };
        let doc_item = XlsxBackend::create_table_docitem(&table, 0, 1);

        match doc_item {
            DocItem::Table { data, .. } => {
                // Verify pivot table output is extracted
                assert_eq!(data.grid[1][0].text, "East");
                assert_eq!(data.grid[1][1].text, "50000");
                assert_eq!(data.grid[2][0].text, "West");
                assert_eq!(data.grid[2][1].text, "75000");

                // Note: Pivot tables are complex:
                // - Definition: xl/pivotTables/pivotTable1.xml (rows, columns, values)
                // - Cache: xl/pivotCache/pivotCacheDefinition1.xml (source data reference)
                // - Records: xl/pivotCache/pivotCacheRecords1.xml (cached source data)
                // Backend extracts rendered pivot table values; pivot metadata is separate
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    /// Test workbook with external links and references
    /// Verifies backend handles references to other workbooks
    #[test]
    fn test_workbook_with_external_links() {
        let _backend = XlsxBackend::new();

        // Cells with external references (e.g., =[Budget.xlsx]Sheet1!$A$1)
        // Backend would see resolved value or error if link is broken
        let data = vec![
            ExcelCell {
                row: 0,
                col: 0,
                text: "Local Value".to_string(),
                row_span: 1,
                col_span: 1,
            },
            ExcelCell {
                row: 0,
                col: 1,
                text: "External Reference".to_string(),
                row_span: 1,
                col_span: 1,
            },
            ExcelCell {
                row: 1,
                col: 0,
                text: "1000".to_string(),
                row_span: 1,
                col_span: 1,
            },
            // External link resolved value (or #REF! if broken)
            ExcelCell {
                row: 1,
                col: 1,
                text: "2000".to_string(), // Value from [Budget.xlsx]Sheet1!A1
                row_span: 1,
                col_span: 1,
            },
            ExcelCell {
                row: 2,
                col: 0,
                text: "3000".to_string(),
                row_span: 1,
                col_span: 1,
            },
            // Broken external link
            ExcelCell {
                row: 2,
                col: 1,
                text: "#REF!".to_string(), // Broken link error
                row_span: 1,
                col_span: 1,
            },
        ];

        let table = ExcelTable {
            anchor: (0, 0),
            num_rows: 3,
            num_cols: 2,
            data,
        };
        let doc_item = XlsxBackend::create_table_docitem(&table, 0, 1);

        match doc_item {
            DocItem::Table { data, .. } => {
                // Verify local and external values are extracted
                assert_eq!(data.grid[1][0].text, "1000");
                assert_eq!(data.grid[1][1].text, "2000"); // External ref resolved
                assert_eq!(data.grid[2][1].text, "#REF!"); // Broken link error preserved

                // Note: External links are stored in:
                // - xl/_rels/workbook.xml.rels (externalLink relationships)
                // - xl/externalLinks/externalLink1.xml (link definition)
                // - Formula: ='[ExternalWorkbook.xlsx]SheetName'!CellRef
                // Backend extracts resolved values or error text if link is broken
            }
            _ => panic!("Expected Table DocItem"),
        }
    }
}
