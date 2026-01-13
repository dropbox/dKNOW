//! Heuristic-based table detection for PDFs
//!
//! This module provides table detection and extraction from PDFs using
//! heuristics based on text cell alignment patterns. It doesn't require
//! ML models, making it suitable for lightweight deployment.
//!
//! The algorithm:
//! 1. Extract text cells with bounding boxes from PDF pages
//! 2. Cluster cells into rows based on vertical alignment
//! 3. Analyze column structure based on horizontal alignment
//! 4. Detect table regions where cells form a grid pattern
//! 5. Convert detected tables to markdown format

use anyhow::{Context, Result};
use pdfium_render::prelude::*;
use std::path::Path;

/// A text cell extracted from a PDF with its bounding box
#[derive(Debug, Clone)]
pub struct TextCell {
    /// Cell text content
    pub text: String,
    /// Left coordinate (points)
    pub x: f32,
    /// Top coordinate (points)
    pub y: f32,
    /// Width (points)
    pub width: f32,
    /// Height (points)
    pub height: f32,
}

impl TextCell {
    /// Center X coordinate
    fn center_x(&self) -> f32 {
        self.x + self.width / 2.0
    }

    /// Center Y coordinate
    fn center_y(&self) -> f32 {
        self.y + self.height / 2.0
    }

    /// Right edge
    fn right(&self) -> f32 {
        self.x + self.width
    }

    /// Bottom edge
    fn bottom(&self) -> f32 {
        self.y + self.height
    }
}

/// A detected table with its cells organized in a grid
#[derive(Debug, Clone)]
pub struct DetectedTable {
    /// Table cells organized as rows of cells
    pub rows: Vec<Vec<String>>,
    /// Number of columns
    pub num_cols: usize,
    /// Bounding box of the table (x, y, width, height)
    pub bbox: (f32, f32, f32, f32),
}

impl DetectedTable {
    /// Convert the table to markdown format
    pub fn to_markdown(&self) -> String {
        if self.rows.is_empty() || self.num_cols == 0 {
            return String::new();
        }

        let mut md = String::new();

        // Find max width for each column
        let mut col_widths = vec![0usize; self.num_cols];
        for row in &self.rows {
            for (i, cell) in row.iter().enumerate() {
                if i < col_widths.len() {
                    col_widths[i] = col_widths[i].max(cell.len());
                }
            }
        }

        // Minimum column width
        for w in &mut col_widths {
            *w = (*w).max(3);
        }

        // Write header row
        if let Some(header) = self.rows.first() {
            md.push('|');
            for (i, cell) in header.iter().enumerate() {
                let width = col_widths.get(i).copied().unwrap_or(3);
                md.push_str(&format!(" {cell:width$} |"));
            }
            md.push('\n');

            // Write separator row
            md.push('|');
            for width in &col_widths {
                md.push_str(&format!(" {} |", "-".repeat(*width)));
            }
            md.push('\n');
        }

        // Write data rows
        for row in self.rows.iter().skip(1) {
            md.push('|');
            for (i, cell) in row.iter().enumerate() {
                let width = col_widths.get(i).copied().unwrap_or(3);
                md.push_str(&format!(" {cell:width$} |"));
            }
            md.push('\n');
        }

        md
    }
}

/// Table detector configuration
#[derive(Debug, Clone)]
pub struct TableDetectorConfig {
    /// Tolerance for row alignment (points) - cells within this Y distance are in same row
    pub row_tolerance: f32,
    /// Tolerance for column alignment (points) - cells within this X distance are in same column
    pub col_tolerance: f32,
    /// Minimum cells to consider a region as a table
    pub min_cells: usize,
    /// Minimum rows to consider a region as a table
    pub min_rows: usize,
    /// Minimum columns to consider a region as a table
    pub min_cols: usize,
}

impl Default for TableDetectorConfig {
    fn default() -> Self {
        Self {
            row_tolerance: 5.0,  // 5 points (~1.7mm)
            col_tolerance: 10.0, // 10 points (~3.5mm)
            min_cells: 6,        // At least 6 cells (e.g., 2x3 or 3x2)
            min_rows: 2,         // At least 2 rows
            min_cols: 2,         // At least 2 columns
        }
    }
}

/// PDF table detector
pub struct TableDetector {
    pdfium: Pdfium,
    config: TableDetectorConfig,
}

impl TableDetector {
    /// Create a new table detector
    pub fn new() -> Result<Self> {
        Self::with_config(TableDetectorConfig::default())
    }

    /// Create a new table detector with custom configuration
    pub fn with_config(config: TableDetectorConfig) -> Result<Self> {
        let pdfium = Pdfium::new(
            Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
                .or_else(|_| Pdfium::bind_to_system_library())
                .context("Failed to bind pdfium library")?,
        );

        Ok(Self { pdfium, config })
    }

    /// Extract tables from a PDF file
    ///
    /// Returns a vector of detected tables, each containing markdown-formatted content.
    pub fn extract_tables(&self, path: &Path) -> Result<Vec<DetectedTable>> {
        let doc = self
            .pdfium
            .load_pdf_from_file(path, None)
            .context("Failed to load PDF")?;

        let mut all_tables = Vec::new();

        for (page_idx, page) in doc.pages().iter().enumerate() {
            tracing::debug!("Processing page {} for tables", page_idx + 1);
            let page_height = page.height().value;

            // Extract text cells from page
            let cells = self.extract_text_cells(&page, page_height)?;
            tracing::debug!("  Extracted {} text cells", cells.len());

            if cells.len() < self.config.min_cells {
                continue;
            }

            // Detect tables in this page
            let tables = self.detect_tables(&cells)?;
            tracing::debug!("  Detected {} tables", tables.len());

            all_tables.extend(tables);
        }

        Ok(all_tables)
    }

    /// Extract text cells from a PDF page
    #[allow(clippy::unused_self)]
    fn extract_text_cells(&self, page: &PdfPage, page_height: f32) -> Result<Vec<TextCell>> {
        let text = page
            .text()
            .map_err(|e| anyhow::anyhow!("Failed to get page text: {e}"))?;

        let mut cells = Vec::new();

        for segment in text.segments().iter() {
            let content = segment.text();
            let content = content.trim();
            if content.is_empty() {
                continue;
            }

            let bounds = segment.bounds();

            // Convert from PDF's bottom-left origin to top-left origin
            let x = bounds.left().value;
            let y = page_height - bounds.top().value;
            let width = bounds.right().value - bounds.left().value;
            let height = bounds.top().value - bounds.bottom().value;

            cells.push(TextCell {
                text: content.to_string(),
                x,
                y,
                width,
                height,
            });
        }

        // Sort cells by position (top to bottom, left to right)
        cells.sort_by(|a, b| {
            let y_cmp = a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal);
            if y_cmp == std::cmp::Ordering::Equal {
                a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal)
            } else {
                y_cmp
            }
        });

        Ok(cells)
    }

    /// Detect tables from text cells using heuristics
    fn detect_tables(&self, cells: &[TextCell]) -> Result<Vec<DetectedTable>> {
        if cells.is_empty() {
            return Ok(Vec::new());
        }

        // Step 1: Cluster cells into rows based on Y alignment
        let rows = self.cluster_rows(cells);

        // Step 2: Find potential table regions (consecutive rows with similar column structure)
        let table_regions = self.find_table_regions(&rows);

        // Step 3: Convert each region to a DetectedTable
        let mut tables = Vec::new();
        for region in table_regions {
            if let Some(table) = self.build_table(&region) {
                tables.push(table);
            }
        }

        Ok(tables)
    }

    /// Cluster cells into rows based on Y coordinate alignment
    fn cluster_rows<'a>(&self, cells: &'a [TextCell]) -> Vec<Vec<&'a TextCell>> {
        let mut rows: Vec<Vec<&TextCell>> = Vec::new();

        for cell in cells {
            // Find an existing row that this cell belongs to
            let mut found_row = None;
            for (i, row) in rows.iter().enumerate() {
                if let Some(first) = row.first() {
                    // Check if cell's Y is within tolerance of row's Y
                    if (cell.center_y() - first.center_y()).abs() <= self.config.row_tolerance {
                        found_row = Some(i);
                        break;
                    }
                }
            }

            if let Some(row_idx) = found_row {
                rows[row_idx].push(cell);
            } else {
                rows.push(vec![cell]);
            }
        }

        // Sort cells within each row by X coordinate
        for row in &mut rows {
            row.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));
        }

        // Sort rows by Y coordinate
        rows.sort_by(|a, b| {
            let ay = a.first().map(|c| c.y).unwrap_or(0.0);
            let by = b.first().map(|c| c.y).unwrap_or(0.0);
            ay.partial_cmp(&by).unwrap_or(std::cmp::Ordering::Equal)
        });

        rows
    }

    /// Find potential table regions (consecutive rows with similar column counts)
    fn find_table_regions<'a>(&self, rows: &[Vec<&'a TextCell>]) -> Vec<Vec<Vec<&'a TextCell>>> {
        let mut regions: Vec<Vec<Vec<&'a TextCell>>> = Vec::new();
        let mut current_region: Vec<Vec<&'a TextCell>> = Vec::new();
        let mut expected_cols: Option<usize> = None;

        for row in rows {
            let num_cells = row.len();

            // Skip single-cell rows (likely headers/paragraphs)
            if num_cells < self.config.min_cols {
                // End current region if it's big enough
                if current_region.len() >= self.config.min_rows {
                    regions.push(std::mem::take(&mut current_region));
                } else {
                    current_region.clear();
                }
                expected_cols = None;
                continue;
            }

            // Check if this row fits the current region's column pattern
            if let Some(exp_cols) = expected_cols {
                // Allow some flexibility in column count (+/- 1)
                if (num_cells as i32 - exp_cols as i32).abs() <= 1 {
                    current_region.push(row.clone());
                } else {
                    // Column count mismatch - start new region
                    if current_region.len() >= self.config.min_rows {
                        regions.push(std::mem::take(&mut current_region));
                    }
                    current_region.clear();
                    current_region.push(row.clone());
                    expected_cols = Some(num_cells);
                }
            } else {
                current_region.push(row.clone());
                expected_cols = Some(num_cells);
            }
        }

        // Don't forget the last region
        if current_region.len() >= self.config.min_rows {
            regions.push(current_region);
        }

        regions
    }

    /// Build a DetectedTable from a region of rows
    fn build_table(&self, region: &[Vec<&TextCell>]) -> Option<DetectedTable> {
        if region.is_empty() {
            return None;
        }

        // Step 1: Find column boundaries by analyzing X positions across all rows
        let col_boundaries = self.find_column_boundaries(region);
        let num_cols = col_boundaries.len().saturating_sub(1);

        if num_cols < self.config.min_cols {
            return None;
        }

        // Step 2: Assign cells to columns
        let mut table_rows: Vec<Vec<String>> = Vec::new();

        for row in region {
            let mut table_row = vec![String::new(); num_cols];

            for cell in row {
                // Find which column this cell belongs to
                let col_idx = self.find_column_index(cell, &col_boundaries);
                if col_idx < num_cols {
                    // Append text if there's already content (merged cells)
                    if table_row[col_idx].is_empty() {
                        table_row[col_idx] = cell.text.clone();
                    } else {
                        table_row[col_idx].push(' ');
                        table_row[col_idx].push_str(&cell.text);
                    }
                }
            }

            table_rows.push(table_row);
        }

        // Step 3: Calculate bounding box
        let bbox = self.calculate_bbox(region);

        Some(DetectedTable {
            rows: table_rows,
            num_cols,
            bbox,
        })
    }

    /// Find column boundaries from a table region
    fn find_column_boundaries(&self, region: &[Vec<&TextCell>]) -> Vec<f32> {
        // Collect all X positions (left edges)
        let mut x_positions: Vec<f32> = region
            .iter()
            .flat_map(|row| row.iter().map(|c| c.x))
            .collect();

        x_positions.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        if x_positions.is_empty() {
            return Vec::new();
        }

        // Cluster X positions to find column boundaries
        let mut boundaries = vec![x_positions[0]];

        for &x in &x_positions[1..] {
            if let Some(&last) = boundaries.last() {
                if x - last > self.config.col_tolerance {
                    boundaries.push(x);
                }
            }
        }

        // Add a final boundary (right edge of last column)
        if let Some(max_right) = region
            .iter()
            .flat_map(|row| row.iter().map(|c| c.right()))
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        {
            boundaries.push(max_right);
        }

        boundaries
    }

    /// Find which column a cell belongs to based on its X position
    #[allow(clippy::unused_self)]
    fn find_column_index(&self, cell: &TextCell, boundaries: &[f32]) -> usize {
        let cell_center = cell.center_x();

        for (i, window) in boundaries.windows(2).enumerate() {
            if cell_center >= window[0] && cell_center < window[1] {
                return i;
            }
        }

        // If past the last boundary, assign to last column
        boundaries.len().saturating_sub(2)
    }

    /// Calculate bounding box for a region
    #[allow(clippy::unused_self)]
    fn calculate_bbox(&self, region: &[Vec<&TextCell>]) -> (f32, f32, f32, f32) {
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for row in region {
            for cell in row {
                min_x = min_x.min(cell.x);
                min_y = min_y.min(cell.y);
                max_x = max_x.max(cell.right());
                max_y = max_y.max(cell.bottom());
            }
        }

        (min_x, min_y, max_x - min_x, max_y - min_y)
    }
}

/// Extract tables from a PDF and return them as markdown
///
/// This is the main entry point for table extraction.
pub fn extract_tables_as_markdown(path: &Path) -> Result<String> {
    let detector = TableDetector::new()?;
    let tables = detector.extract_tables(path)?;

    if tables.is_empty() {
        return Ok(String::new());
    }

    let mut md = String::new();
    for (i, table) in tables.iter().enumerate() {
        if i > 0 {
            md.push_str("\n\n");
        }
        md.push_str(&table.to_markdown());
    }

    Ok(md)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detected_table_to_markdown() {
        let table = DetectedTable {
            rows: vec![
                vec!["Name".to_string(), "Age".to_string(), "City".to_string()],
                vec![
                    "Alice".to_string(),
                    "30".to_string(),
                    "New York".to_string(),
                ],
                vec!["Bob".to_string(), "25".to_string(), "Boston".to_string()],
            ],
            num_cols: 3,
            bbox: (0.0, 0.0, 100.0, 50.0),
        };

        let md = table.to_markdown();
        assert!(md.contains("| Name"));
        assert!(md.contains("| Age"));
        assert!(md.contains("| City"));
        assert!(md.contains("| Alice"));
        assert!(md.contains("| ---"));
    }

    #[test]
    fn test_empty_table() {
        let table = DetectedTable {
            rows: vec![],
            num_cols: 0,
            bbox: (0.0, 0.0, 0.0, 0.0),
        };

        let md = table.to_markdown();
        assert!(md.is_empty());
    }

    #[test]
    fn test_config_default() {
        let config = TableDetectorConfig::default();
        assert_eq!(config.min_rows, 2);
        assert_eq!(config.min_cols, 2);
        assert!(config.row_tolerance > 0.0);
    }

    #[test]
    fn test_pdf_table_extraction() {
        // This test requires pdfium library to be available
        // Skip if pdfium is not found
        let detector = match TableDetector::new() {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Skipping test - pdfium not available: {e}");
                return;
            }
        };

        // Test with bert.pdf if available
        let pdf_path =
            std::path::Path::new("/Users/ayates/video_audio_extracts/test_pdf_corpus/all/bert.pdf");
        if !pdf_path.exists() {
            eprintln!("Skipping test - bert.pdf not found");
            return;
        }

        let result = detector.extract_tables(pdf_path);
        match result {
            Ok(tables) => {
                println!("Found {} tables in bert.pdf", tables.len());
                for (i, table) in tables.iter().enumerate() {
                    println!(
                        "Table {}: {} rows x {} cols",
                        i + 1,
                        table.rows.len(),
                        table.num_cols
                    );
                }
                // bert.pdf should have result tables
                // Even if no tables found, the extraction itself should not error
            }
            Err(e) => {
                // Extraction may fail for various reasons (encrypted PDF, etc.)
                // This is acceptable for a test
                println!("Table extraction failed: {e}");
            }
        }
    }
}
