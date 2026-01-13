//! Microsoft Access (.mdb, .accdb) format support
//!
//! Access files are database files (Jet/ACE database format).
//! Strategy: Use mdbtools to extract schema and data, convert to text.

use anyhow::{Context, Result};
use docling_core::{
    content::{
        CoordOrigin, DocItem, ItemRef, ProvenanceItem, TableCell, TableData, US_LETTER_HEIGHT,
        US_LETTER_WIDTH,
    },
    document::{GroupItem, Origin, PageInfo, PageSize},
    BoundingBox, DoclingDocument,
};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

/// Backend for Microsoft Access files
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct AccessBackend;

impl AccessBackend {
    /// Create a new Access backend instance
    #[inline]
    #[must_use = "creates Access backend instance"]
    pub const fn new() -> Self {
        Self
    }

    /// Extract table names from database
    fn list_tables(input_path: &Path) -> Result<Vec<String>> {
        let output = Command::new("mdb-tables")
            .arg("-1") // One table per line
            .arg(input_path)
            .output()
            .context("Failed to execute mdb-tables (is mdbtools installed?)")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("mdb-tables failed: {stderr}");
        }

        let tables = String::from_utf8_lossy(&output.stdout);
        Ok(tables
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(std::string::ToString::to_string)
            .collect())
    }

    /// Export table data as CSV
    fn export_table(input_path: &Path, table_name: &str) -> Result<Vec<Vec<String>>> {
        let output = Command::new("mdb-export")
            .arg(input_path)
            .arg(table_name)
            .output()
            .context("Failed to execute mdb-export")?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let csv_data = String::from_utf8_lossy(&output.stdout);
        let mut rows = Vec::new();

        for line in csv_data.lines().take(50) {
            // Limit to 50 rows
            let cells: Vec<String> = line.split(',').map(|s| s.trim().to_string()).collect();
            rows.push(cells);
        }

        Ok(rows)
    }

    /// Parse Access database and generate `DocItems`
    ///
    /// Uses mdbtools to extract tables from .mdb/.accdb files
    ///
    /// # Errors
    ///
    /// Returns an error if the database file cannot be read or if table extraction fails.
    #[must_use = "this function returns a parsed document that should be processed"]
    #[allow(clippy::too_many_lines)] // Complex database extraction - keeping together for clarity
    pub fn parse(&self, input_path: &Path) -> Result<DoclingDocument> {
        let file_name = input_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        // List all tables
        let table_names = Self::list_tables(input_path).context("Failed to list tables")?;

        // Create DocItems - separate texts and tables
        let mut text_items = Vec::new();
        let mut table_items = Vec::new();
        let mut text_idx = 0;
        let mut table_idx = 0;
        let mut body_children = Vec::new();

        // Create default bounding box (full page, US Letter)
        let default_bbox = BoundingBox::new(
            0.0,
            0.0,
            US_LETTER_WIDTH,
            US_LETTER_HEIGHT,
            CoordOrigin::BottomLeft,
        );

        // Add title
        let title_text = format!("Microsoft Access Database: {file_name}");
        let title_ref = format!("#/texts/{text_idx}");
        body_children.push(ItemRef::new(&title_ref));
        text_items.push(DocItem::Title {
            self_ref: title_ref,
            parent: Some(ItemRef::new("#")),
            children: vec![],
            content_layer: "body".to_string(),
            prov: vec![ProvenanceItem {
                page_no: 1,
                bbox: default_bbox,
                charspan: None,
            }],
            orig: title_text.clone(),
            text: title_text,
            formatting: None,
            hyperlink: None,
        });
        text_idx += 1;

        // Add summary paragraph
        let summary = format!("Database contains {} tables.", table_names.len());
        let summary_ref = format!("#/texts/{text_idx}");
        body_children.push(ItemRef::new(&summary_ref));
        text_items.push(DocItem::SectionHeader {
            self_ref: summary_ref,
            parent: Some(ItemRef::new("#")),
            children: vec![],
            content_layer: "body".to_string(),
            prov: vec![ProvenanceItem {
                page_no: 1,
                bbox: default_bbox,
                charspan: None,
            }],
            orig: summary.clone(),
            text: summary,
            level: 1,
            formatting: None,
            hyperlink: None,
        });
        text_idx += 1;

        // Process each table
        for table_name in &table_names {
            // Add table heading
            let heading = format!("Table: {table_name}");
            let heading_ref = format!("#/texts/{text_idx}");
            body_children.push(ItemRef::new(&heading_ref));
            text_items.push(DocItem::SectionHeader {
                self_ref: heading_ref,
                parent: Some(ItemRef::new("#")),
                children: vec![],
                content_layer: "body".to_string(),
                prov: vec![ProvenanceItem {
                    page_no: 1,
                    bbox: default_bbox,
                    charspan: None,
                }],
                orig: heading.clone(),
                text: heading,
                level: 2,
                formatting: None,
                hyperlink: None,
            });
            text_idx += 1;

            // Export table data
            match Self::export_table(input_path, table_name) {
                Ok(rows) if !rows.is_empty() => {
                    // Build grid of TableCells
                    let mut grid = Vec::new();
                    for row in &rows {
                        let mut grid_row = Vec::new();
                        for cell_text in row {
                            grid_row.push(TableCell {
                                text: cell_text.clone(),
                                row_span: Some(1),
                                col_span: Some(1),
                                ref_item: None,
                                start_row_offset_idx: None,
                                start_col_offset_idx: None,
                                column_header: false,
                                row_header: false,
                                from_ocr: false,
                                bbox: None,
                                confidence: None,
                            });
                        }
                        grid.push(grid_row);
                    }

                    let table_data = TableData {
                        num_rows: rows.len(),
                        num_cols: rows.first().map_or(0, std::vec::Vec::len),
                        grid,
                        table_cells: None,
                    };

                    let table_ref = format!("#/tables/{table_idx}");
                    body_children.push(ItemRef::new(&table_ref));
                    table_items.push(DocItem::Table {
                        self_ref: table_ref,
                        parent: Some(ItemRef::new("#")),
                        children: vec![],
                        content_layer: "body".to_string(),
                        prov: vec![ProvenanceItem {
                            page_no: 1,
                            bbox: default_bbox,
                            charspan: None,
                        }],
                        data: table_data,
                        captions: vec![],
                        footnotes: vec![],
                        references: vec![],
                        image: None,
                        annotations: vec![],
                    });
                    table_idx += 1;
                }
                _ => {
                    // Add error message
                    let error_text = "Unable to export table data";
                    let error_ref = format!("#/texts/{text_idx}");
                    body_children.push(ItemRef::new(&error_ref));
                    text_items.push(DocItem::Text {
                        self_ref: error_ref,
                        parent: Some(ItemRef::new("#")),
                        children: vec![],
                        content_layer: "body".to_string(),
                        prov: vec![ProvenanceItem {
                            page_no: 1,
                            bbox: default_bbox,
                            charspan: None,
                        }],
                        orig: error_text.to_string(),
                        text: error_text.to_string(),
                        formatting: None,
                        hyperlink: None,
                    });
                    text_idx += 1;
                }
            }
        }

        // Create body group
        let body = GroupItem {
            self_ref: "#".to_string(),
            parent: None,
            children: body_children,
            content_layer: "body".to_string(),
            name: "body".to_string(),
            label: "body".to_string(),
        };

        // Create pages map
        let mut pages = HashMap::new();
        pages.insert(
            "1".to_string(),
            PageInfo {
                page_no: 1,
                size: PageSize {
                    width: US_LETTER_WIDTH,
                    height: US_LETTER_HEIGHT,
                },
            },
        );

        Ok(DoclingDocument {
            schema_name: "DoclingDocument".to_string(),
            version: "1.7.0".to_string(),
            name: file_name,
            origin: Origin {
                filename: input_path.to_string_lossy().to_string(),
                mimetype: "application/x-msaccess".to_string(),
                binary_hash: 0,
            },
            body,
            furniture: None,
            texts: text_items,
            tables: table_items,
            groups: vec![],
            pictures: vec![],
            key_value_items: vec![],
            form_items: vec![],
            pages,
        })
    }

    /// Get the backend name
    #[inline]
    #[must_use = "returns backend name string"]
    pub const fn name(&self) -> &'static str {
        "Access"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_access_backend_creation() {
        let backend = AccessBackend::new();
        assert_eq!(backend.name(), "Access");
    }

    #[test]
    #[allow(
        clippy::default_constructed_unit_structs,
        reason = "testing Default trait impl"
    )]
    fn test_access_backend_default_equals_new() {
        // Verify derived Default produces same result as new()
        assert_eq!(AccessBackend::default(), AccessBackend::new());
    }
}
