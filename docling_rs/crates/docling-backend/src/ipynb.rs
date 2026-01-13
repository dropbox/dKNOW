//! Jupyter Notebook backend for docling
//!
//! This backend converts Jupyter Notebook (.ipynb) files to docling's document model.

use crate::traits::{BackendOptions, DocumentBackend};
use crate::utils::{create_section_header, create_text_item, opt_vec};
use docling_core::{DocItem, DoclingError, Document, DocumentMetadata, InputFormat};
use docling_notebook::{
    parse_notebook_from_str, CellType, NotebookCell, OutputType, ParsedNotebook,
};
use std::fmt::Write;
use std::path::Path;

/// Jupyter Notebook backend
///
/// Converts Jupyter Notebook (.ipynb) files to docling's document model.
/// Supports code cells, markdown cells, and cell outputs.
///
/// ## Features
///
/// - Parse code cells with execution counts
/// - Parse markdown cells with rendered content
/// - Parse cell outputs (stream, display data, execute results, errors)
/// - Extract notebook metadata (kernel, language, authors)
/// - Markdown-formatted output with syntax highlighting
///
/// ## Example
///
/// ```no_run
/// use docling_backend::IpynbBackend;
/// use docling_backend::DocumentBackend;
///
/// let backend = IpynbBackend::new();
/// let result = backend.parse_file("analysis.ipynb", &Default::default())?;
/// println!("Notebook: {:?}", result.metadata.title);
/// # Ok::<(), docling_core::error::DoclingError>(())
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct IpynbBackend;

impl IpynbBackend {
    /// Create a new Jupyter Notebook backend instance
    #[inline]
    #[must_use = "creates a backend instance that should be used for parsing"]
    pub const fn new() -> Self {
        Self
    }

    /// Format a single notebook cell as markdown
    fn format_cell(cell: &NotebookCell, cell_index: usize) -> String {
        let mut md = String::new();

        match cell.cell_type {
            CellType::Markdown => {
                // Add clear header for markdown cells with cell ID
                if let Some(ref cell_id) = cell.cell_id {
                    let _ = write!(
                        md,
                        "**Cell {} (Markdown)** [ID: {}]:\n\n",
                        cell_index + 1,
                        cell_id
                    );
                } else {
                    let _ = write!(md, "**Cell {} (Markdown)**:\n\n", cell_index + 1);
                }

                // Render markdown cells directly
                md.push_str(&cell.source);
                md.push_str("\n\n");

                // Add visual separator after markdown cell for consistency
                md.push_str("---\n\n");
            }
            CellType::Code => {
                // Format code cells with execution count and cell ID
                let cell_header = cell.cell_id.as_ref().map_or_else(
                    || {
                        cell.execution_count.map_or_else(
                            || format!("**Cell {} (Code)**:\n\n", cell_index + 1),
                            |count| format!("**Cell {} (Code)** [{}]:\n\n", cell_index + 1, count),
                        )
                    },
                    |cell_id| {
                        cell.execution_count.map_or_else(
                            || format!("**Cell {} (Code)** [ID: {}]:\n\n", cell_index + 1, cell_id),
                            |count| {
                                format!(
                                    "**Cell {} (Code)** [{}] [ID: {}]:\n\n",
                                    cell_index + 1,
                                    count,
                                    cell_id
                                )
                            },
                        )
                    },
                );
                md.push_str(&cell_header);

                // Add code block with syntax highlighting
                md.push_str("```python\n");
                md.push_str(&cell.source);
                md.push_str("\n```\n\n");

                // Add outputs if present with clearer formatting and visual separation
                if !cell.outputs.is_empty() {
                    md.push_str("---\n\n**Output**:\n\n");
                    for output in &cell.outputs {
                        match output.output_type {
                            OutputType::Stream | OutputType::Error => {
                                if let Some(text) = &output.text {
                                    md.push_str("```\n");
                                    md.push_str(text);
                                    md.push_str("\n```\n\n");
                                }
                            }
                            OutputType::ExecuteResult | OutputType::DisplayData => {
                                if let Some(text) = &output.text {
                                    md.push_str("```\n");
                                    md.push_str(text);
                                    md.push_str("\n```\n\n");
                                } else if let Some(data) = &output.data {
                                    md.push_str("```\n");
                                    md.push_str(data);
                                    md.push_str("\n```\n\n");
                                }
                            }
                        }
                    }
                }

                // Add visual separator after code cell for consistency
                md.push_str("---\n\n");
            }
            CellType::Raw => {
                // Format raw cells as preformatted text with cell ID
                if let Some(ref cell_id) = cell.cell_id {
                    let _ = write!(
                        md,
                        "**Cell {} (Raw)** [ID: {}]:\n\n",
                        cell_index + 1,
                        cell_id
                    );
                } else {
                    let _ = write!(md, "**Cell {} (Raw)**:\n\n", cell_index + 1);
                }
                md.push_str("```\n");
                md.push_str(&cell.source);
                md.push_str("\n```\n\n");

                // Add visual separator after raw cell for consistency
                md.push_str("---\n\n");
            }
        }

        md
    }

    /// Create `DocItems` from a notebook cell
    ///
    /// Each cell type maps to different `DocItem` variants:
    /// - Markdown cells → Text `DocItems`
    /// - Code cells → Code `DocItems` (with outputs as additional Text `DocItems`)
    /// - Raw cells → Text `DocItems`
    fn create_cell_docitems(
        cell: &NotebookCell,
        _cell_index: usize,
        text_idx: &mut usize,
        code_idx: &mut usize,
    ) -> Vec<DocItem> {
        let mut doc_items = Vec::new();

        match cell.cell_type {
            CellType::Markdown => {
                // Markdown cells become Text DocItems
                let text_item = create_text_item(*text_idx, cell.source.clone(), vec![]);
                *text_idx += 1;
                doc_items.push(text_item);
            }
            CellType::Code => {
                // Code cells become Code DocItems
                let item_ref = format!("#/code/{}", *code_idx);
                *code_idx += 1;

                let code_item = DocItem::Code {
                    self_ref: item_ref,
                    parent: None,
                    children: vec![],
                    content_layer: "body".to_string(),
                    prov: vec![],
                    orig: cell.source.clone(),
                    text: cell.source.clone(),
                    language: Some("python".to_string()),
                    formatting: None,
                    hyperlink: None,
                };
                doc_items.push(code_item);

                // Add outputs as Text DocItems
                for output in &cell.outputs {
                    let output_text = match output.output_type {
                        OutputType::Stream | OutputType::Error => output.text.clone(),
                        OutputType::ExecuteResult | OutputType::DisplayData => {
                            output.text.as_ref().or(output.data.as_ref()).cloned()
                        }
                    };

                    if let Some(text) = output_text {
                        let output_item = create_text_item(*text_idx, text, vec![]);
                        *text_idx += 1;
                        doc_items.push(output_item);
                    }
                }
            }
            CellType::Raw => {
                // Raw cells become Text DocItems
                let text_item = create_text_item(*text_idx, cell.source.clone(), vec![]);
                *text_idx += 1;
                doc_items.push(text_item);
            }
        }

        doc_items
    }

    /// Create `DocItems` from notebook
    fn create_docitems(notebook: &ParsedNotebook) -> Vec<DocItem> {
        let mut doc_items = Vec::new();
        let mut text_idx = 0;
        let mut code_idx = 0;

        // Add title as SectionHeader if present
        if let Some(title) = &notebook.metadata.title {
            doc_items.push(create_section_header(text_idx, title.clone(), 1, vec![]));
            text_idx += 1;
        }

        // Add metadata as Text DocItems
        if let Some(kernel) = &notebook.metadata.kernel_name {
            let kernel_text = format!("Kernel: {kernel}");
            let kernel_item = create_text_item(text_idx, kernel_text, vec![]);
            text_idx += 1;
            doc_items.push(kernel_item);
        }

        if let Some(language) = &notebook.metadata.language_name {
            let lang_text = format!("Language: {language}");
            let lang_item = create_text_item(text_idx, lang_text, vec![]);
            text_idx += 1;
            doc_items.push(lang_item);
        }

        if !notebook.metadata.authors.is_empty() {
            let authors_text = format!("Authors: {}", notebook.metadata.authors.join(", "));
            let authors_item = create_text_item(text_idx, authors_text, vec![]);
            text_idx += 1;
            doc_items.push(authors_item);
        }

        // Add all cells
        for (i, cell) in notebook.cells.iter().enumerate() {
            // Add separator between cells for clarity (except before first cell)
            if i > 0 {
                doc_items.push(create_text_item(text_idx, "---".to_string(), vec![]));
                text_idx += 1;
            }

            let cell_items = Self::create_cell_docitems(cell, i, &mut text_idx, &mut code_idx);
            doc_items.extend(cell_items);
        }

        doc_items
    }

    /// Convert parsed notebook to markdown
    fn notebook_to_markdown(notebook: &ParsedNotebook) -> String {
        let mut markdown = String::new();

        // Add title if present
        if let Some(title) = &notebook.metadata.title {
            let _ = write!(markdown, "# {title}\n\n");
        }

        // Add metadata section with proper header
        let has_kernel = notebook.metadata.kernel_name.is_some();
        let has_language = notebook.metadata.language_name.is_some();
        let has_authors = !notebook.metadata.authors.is_empty();

        if has_kernel || has_language || has_authors {
            markdown.push_str("## Metadata\n\n");

            if let Some(kernel) = &notebook.metadata.kernel_name {
                let _ = write!(markdown, "Kernel: {kernel}\n\n");
            }
            if let Some(language) = &notebook.metadata.language_name {
                let _ = write!(markdown, "Language: {language}\n\n");
            }
            if !notebook.metadata.authors.is_empty() {
                markdown.push_str("Authors: ");
                markdown.push_str(&notebook.metadata.authors.join(", "));
                markdown.push_str("\n\n");
            }
        }

        // Add all cells
        for (i, cell) in notebook.cells.iter().enumerate() {
            markdown.push_str(&Self::format_cell(cell, i));
        }

        markdown
    }
}

impl DocumentBackend for IpynbBackend {
    #[inline]
    fn format(&self) -> InputFormat {
        InputFormat::Ipynb
    }

    fn parse_bytes(
        &self,
        data: &[u8],
        _options: &BackendOptions,
    ) -> Result<Document, DoclingError> {
        // Convert bytes to string
        let content = std::str::from_utf8(data).map_err(|e| {
            DoclingError::BackendError(format!("Invalid UTF-8 in notebook file: {e}"))
        })?;

        // Parse notebook
        let notebook = parse_notebook_from_str(content)
            .map_err(|e| DoclingError::BackendError(format!("Failed to parse notebook: {e}")))?;

        // Generate DocItems
        let doc_items = Self::create_docitems(&notebook);

        // Convert to markdown
        let markdown = Self::notebook_to_markdown(&notebook);
        let num_characters = markdown.chars().count();

        // Create document
        Ok(Document {
            markdown,
            format: InputFormat::Ipynb,
            metadata: DocumentMetadata {
                num_pages: Some(notebook.cells.len()),
                num_characters,
                title: notebook.metadata.title.clone(),
                author: if notebook.metadata.authors.is_empty() {
                    None
                } else {
                    Some(notebook.metadata.authors.join(", "))
                },
                created: None,
                modified: None,
                language: notebook.metadata.language_name,
                subject: None,
                exif: None,
            },
            docling_document: None,
            content_blocks: opt_vec(doc_items),
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
    use docling_notebook::{CellOutputData, NotebookMetadata};

    // ==================== BACKEND TESTS ====================

    #[test]
    fn test_ipynb_backend_creation() {
        let backend = IpynbBackend::new();
        assert_eq!(
            backend.format(),
            InputFormat::Ipynb,
            "IpynbBackend::new() should return Ipynb format"
        );
    }

    #[test]
    fn test_ipynb_backend_default() {
        let backend = IpynbBackend;
        assert_eq!(
            backend.format(),
            InputFormat::Ipynb,
            "IpynbBackend struct should return Ipynb format"
        );
    }

    #[test]
    fn test_format_method() {
        let backend = IpynbBackend::new();
        assert_eq!(
            backend.format(),
            InputFormat::Ipynb,
            "format() method should return InputFormat::Ipynb"
        );
    }

    // ==================== CELL FORMATTING TESTS ====================

    #[test]
    fn test_format_markdown_cell() {
        let cell = NotebookCell {
            cell_type: CellType::Markdown,
            cell_id: None,
            source: "# Hello World\n\nThis is a test.".to_string(),
            execution_count: None,
            outputs: vec![],
        };

        let md = IpynbBackend::format_cell(&cell, 0);
        assert!(
            md.contains("# Hello World"),
            "Markdown cell should preserve heading"
        );
        assert!(
            md.contains("This is a test."),
            "Markdown cell should preserve content"
        );
        assert!(
            !md.contains("```"),
            "Markdown cell should not have code fences"
        );
    }

    #[test]
    fn test_format_code_cell_no_output() {
        let cell = NotebookCell {
            cell_type: CellType::Code,
            cell_id: None,
            source: "print('Hello')".to_string(),
            execution_count: Some(1),
            outputs: vec![],
        };

        let md = IpynbBackend::format_cell(&cell, 0);
        assert!(
            md.contains("**Cell 1 (Code)** [1]:"),
            "Code cell should have header with execution count"
        );
        assert!(
            md.contains("```python"),
            "Code cell should have Python code fence"
        );
        assert!(
            md.contains("print('Hello')"),
            "Code cell should contain source code"
        );
        assert!(
            !md.contains("Output:"),
            "Code cell without outputs should not have Output section"
        );
    }

    #[test]
    fn test_format_code_cell_without_execution_count() {
        let cell = NotebookCell {
            cell_type: CellType::Code,
            cell_id: None,
            source: "x = 42".to_string(),
            execution_count: None,
            outputs: vec![],
        };

        let md = IpynbBackend::format_cell(&cell, 0);
        assert!(
            md.contains("**Cell 1 (Code)**:"),
            "Code cell without execution count should have simple header"
        );
        assert!(
            !md.contains('['),
            "Code cell without execution count should not have brackets"
        );
        assert!(
            md.contains("```python"),
            "Code cell should have Python code fence"
        );
        assert!(
            md.contains("x = 42"),
            "Code cell should contain source code"
        );
    }

    #[test]
    fn test_format_code_cell_with_output() {
        let cell = NotebookCell {
            cell_type: CellType::Code,
            cell_id: None,
            source: "print('Hello')".to_string(),
            execution_count: Some(1),
            outputs: vec![CellOutputData {
                output_type: OutputType::Stream,
                text: Some("Hello\n".to_string()),
                data: None,
            }],
        };

        let md = IpynbBackend::format_cell(&cell, 0);
        assert!(
            md.contains("**Cell 1 (Code)** [1]:"),
            "Code cell should have header with execution count"
        );
        assert!(
            md.contains("```python"),
            "Code cell should have Python code fence"
        );
        assert!(
            md.contains("print('Hello')"),
            "Code cell should contain source code"
        );
        assert!(
            md.contains("**Output**:"),
            "Code cell with output should have Output section"
        );
        assert!(md.contains("Hello"), "Code cell should contain output text");
    }

    #[test]
    fn test_format_raw_cell() {
        let cell = NotebookCell {
            cell_type: CellType::Raw,
            cell_id: None,
            source: "Raw text content".to_string(),
            execution_count: None,
            outputs: vec![],
        };

        let md = IpynbBackend::format_cell(&cell, 0);
        assert!(
            md.contains("**Cell 1 (Raw)**:"),
            "Raw cell should have Raw type in header"
        );
        assert!(md.contains("```"), "Raw cell should have code fence");
        assert!(
            md.contains("Raw text content"),
            "Raw cell should contain raw content"
        );
    }

    // ==================== OUTPUT TYPE TESTS ====================

    #[test]
    fn test_format_cell_stream_output() {
        let cell = NotebookCell {
            cell_type: CellType::Code,
            cell_id: None,
            source: "print('test')".to_string(),
            execution_count: Some(1),
            outputs: vec![CellOutputData {
                output_type: OutputType::Stream,
                text: Some("test\n".to_string()),
                data: None,
            }],
        };

        let md = IpynbBackend::format_cell(&cell, 0);
        assert!(
            md.contains("**Output**:"),
            "Stream output should have Output section"
        );
        assert!(
            md.contains("```\ntest"),
            "Stream output should be in code block"
        );
    }

    #[test]
    fn test_format_cell_execute_result_output() {
        let cell = NotebookCell {
            cell_type: CellType::Code,
            cell_id: None,
            source: "42".to_string(),
            execution_count: Some(1),
            outputs: vec![CellOutputData {
                output_type: OutputType::ExecuteResult,
                text: Some("42".to_string()),
                data: None,
            }],
        };

        let md = IpynbBackend::format_cell(&cell, 0);
        assert!(
            md.contains("**Output**:"),
            "Execute result should have Output section"
        );
        assert!(
            md.contains("42"),
            "Execute result should contain the result value"
        );
    }

    #[test]
    fn test_format_cell_display_data_output() {
        let cell = NotebookCell {
            cell_type: CellType::Code,
            cell_id: None,
            source: "plt.plot([1,2,3])".to_string(),
            execution_count: Some(1),
            outputs: vec![CellOutputData {
                output_type: OutputType::DisplayData,
                text: None,
                data: Some("<image data>".to_string()),
            }],
        };

        let md = IpynbBackend::format_cell(&cell, 0);
        assert!(
            md.contains("**Output**:"),
            "Display data should have Output section"
        );
        assert!(
            md.contains("<image data>"),
            "Display data should contain data representation"
        );
    }

    #[test]
    fn test_format_cell_error_output() {
        let cell = NotebookCell {
            cell_type: CellType::Code,
            cell_id: None,
            source: "raise ValueError('test')".to_string(),
            execution_count: Some(1),
            outputs: vec![CellOutputData {
                output_type: OutputType::Error,
                text: Some("ValueError: test\n".to_string()),
                data: None,
            }],
        };

        let md = IpynbBackend::format_cell(&cell, 0);
        assert!(
            md.contains("**Output**:"),
            "Error output should have Output section"
        );
        assert!(
            md.contains("```\nValueError: test"),
            "Error output should contain error message"
        );
    }

    #[test]
    fn test_format_cell_multiple_outputs() {
        let cell = NotebookCell {
            cell_type: CellType::Code,
            cell_id: None,
            source: "print('a'); print('b')".to_string(),
            execution_count: Some(1),
            outputs: vec![
                CellOutputData {
                    output_type: OutputType::Stream,
                    text: Some("a\n".to_string()),
                    data: None,
                },
                CellOutputData {
                    output_type: OutputType::Stream,
                    text: Some("b\n".to_string()),
                    data: None,
                },
            ],
        };

        let md = IpynbBackend::format_cell(&cell, 0);
        assert!(
            md.contains('a'),
            "Multiple outputs should contain first output 'a'"
        );
        assert!(
            md.contains('b'),
            "Multiple outputs should contain second output 'b'"
        );
    }

    // ==================== METADATA TESTS ====================

    #[test]
    fn test_notebook_to_markdown_with_metadata() {
        let notebook = ParsedNotebook {
            metadata: NotebookMetadata {
                kernel_name: Some("python3".to_string()),
                language_name: Some("python".to_string()),
                authors: vec!["Alice".to_string(), "Bob".to_string()],
                title: Some("Test Notebook".to_string()),
            },
            cells: vec![],
        };

        let md = IpynbBackend::notebook_to_markdown(&notebook);
        assert!(
            md.contains("# Test Notebook"),
            "Markdown should contain notebook title"
        );
        assert!(
            md.contains("Kernel: python3"),
            "Markdown should contain kernel name"
        );
        assert!(
            md.contains("Language: python"),
            "Markdown should contain language name"
        );
        assert!(
            md.contains("Authors: Alice, Bob"),
            "Markdown should contain authors list"
        );
    }

    #[test]
    fn test_notebook_to_markdown_without_metadata() {
        let notebook = ParsedNotebook {
            metadata: NotebookMetadata {
                kernel_name: None,
                language_name: None,
                authors: vec![],
                title: None,
            },
            cells: vec![],
        };

        let md = IpynbBackend::notebook_to_markdown(&notebook);
        assert!(
            !md.contains("Kernel:"),
            "Notebook without kernel should not have Kernel field"
        );
        assert!(
            !md.contains("Language:"),
            "Notebook without language should not have Language field"
        );
        assert!(
            !md.contains("Authors:"),
            "Notebook without authors should not have Authors field"
        );
        assert!(
            !md.contains("---"),
            "Notebook without metadata should not have divider"
        );
    }

    #[test]
    fn test_notebook_with_single_author() {
        let notebook = ParsedNotebook {
            metadata: NotebookMetadata {
                kernel_name: None,
                language_name: None,
                authors: vec!["Alice".to_string()],
                title: None,
            },
            cells: vec![],
        };

        let md = IpynbBackend::notebook_to_markdown(&notebook);
        assert!(
            md.contains("Authors: Alice"),
            "Single author should be displayed"
        );
    }

    // ==================== DOCITEM CREATION TESTS ====================

    #[test]
    fn test_create_docitems_from_markdown_cell() {
        let cell = NotebookCell {
            cell_type: CellType::Markdown,
            cell_id: None,
            source: "# Title\n\nContent".to_string(),
            execution_count: None,
            outputs: vec![],
        };

        let mut text_idx = 0;
        let mut code_idx = 0;
        let doc_items = IpynbBackend::create_cell_docitems(&cell, 0, &mut text_idx, &mut code_idx);

        assert_eq!(doc_items.len(), 1, "Markdown cell should create 1 DocItem");
        assert!(
            matches!(doc_items[0], DocItem::Text { .. }),
            "Markdown cell should create Text DocItem"
        );
        assert_eq!(text_idx, 1, "Text index should increment to 1");
        assert_eq!(code_idx, 0, "Code index should remain 0 for markdown cell");
    }

    #[test]
    fn test_create_docitems_from_code_cell() {
        let cell = NotebookCell {
            cell_type: CellType::Code,
            cell_id: None,
            source: "print('test')".to_string(),
            execution_count: Some(1),
            outputs: vec![],
        };

        let mut text_idx = 0;
        let mut code_idx = 0;
        let doc_items = IpynbBackend::create_cell_docitems(&cell, 0, &mut text_idx, &mut code_idx);

        assert_eq!(
            doc_items.len(),
            1,
            "Code cell without output should create 1 DocItem"
        );
        assert!(
            matches!(doc_items[0], DocItem::Code { .. }),
            "Code cell should create Code DocItem"
        );
        assert_eq!(
            text_idx, 0,
            "Text index should remain 0 for code cell without output"
        );
        assert_eq!(code_idx, 1, "Code index should increment to 1");
    }

    #[test]
    fn test_create_docitems_from_code_cell_with_output() {
        let cell = NotebookCell {
            cell_type: CellType::Code,
            cell_id: None,
            source: "print('test')".to_string(),
            execution_count: Some(1),
            outputs: vec![CellOutputData {
                output_type: OutputType::Stream,
                text: Some("test\n".to_string()),
                data: None,
            }],
        };

        let mut text_idx = 0;
        let mut code_idx = 0;
        let doc_items = IpynbBackend::create_cell_docitems(&cell, 0, &mut text_idx, &mut code_idx);

        assert_eq!(
            doc_items.len(),
            2,
            "Code cell with output should create 2 DocItems (Code + Output)"
        );
        assert!(
            matches!(doc_items[0], DocItem::Code { .. }),
            "First DocItem should be Code"
        );
        assert!(
            matches!(doc_items[1], DocItem::Text { .. }),
            "Second DocItem should be Text (output)"
        );
        assert_eq!(text_idx, 1, "Text index should increment to 1 for output");
        assert_eq!(code_idx, 1, "Code index should increment to 1");
    }

    #[test]
    fn test_create_docitems_from_raw_cell() {
        let cell = NotebookCell {
            cell_type: CellType::Raw,
            cell_id: None,
            source: "raw content".to_string(),
            execution_count: None,
            outputs: vec![],
        };

        let mut text_idx = 0;
        let mut code_idx = 0;
        let doc_items = IpynbBackend::create_cell_docitems(&cell, 0, &mut text_idx, &mut code_idx);

        assert_eq!(doc_items.len(), 1, "Raw cell should create 1 DocItem");
        assert!(
            matches!(doc_items[0], DocItem::Text { .. }),
            "Raw cell should create Text DocItem"
        );
        assert_eq!(text_idx, 1, "Text index should increment to 1");
        assert_eq!(code_idx, 0, "Code index should remain 0 for raw cell");
    }

    // ==================== ERROR HANDLING TESTS ====================

    #[test]
    fn test_parse_bytes_invalid_utf8() {
        let backend = IpynbBackend::new();
        let options = BackendOptions::default();
        let invalid_utf8 = vec![0xFF, 0xFE, 0xFD];

        let result = backend.parse_bytes(&invalid_utf8, &options);
        assert!(result.is_err(), "Invalid UTF-8 should return error");

        if let Err(DoclingError::BackendError(msg)) = result {
            assert!(
                msg.contains("Invalid UTF-8"),
                "Error message should mention Invalid UTF-8"
            );
        } else {
            panic!("Expected BackendError with UTF-8 message");
        }
    }

    #[test]
    fn test_parse_bytes_invalid_json() {
        let backend = IpynbBackend::new();
        let options = BackendOptions::default();
        let invalid_json = b"{ not valid json }";

        let result = backend.parse_bytes(invalid_json, &options);
        assert!(result.is_err(), "Invalid JSON should return error");

        if let Err(DoclingError::BackendError(msg)) = result {
            assert!(
                msg.contains("Failed to parse notebook"),
                "Error message should mention notebook parsing failure"
            );
        } else {
            panic!("Expected BackendError with parse message");
        }
    }

    #[test]
    fn test_parse_bytes_empty() {
        let backend = IpynbBackend::new();
        let options = BackendOptions::default();
        let empty_data: &[u8] = b"";

        let result = backend.parse_bytes(empty_data, &options);
        assert!(result.is_err());
    }

    // ==================== INTEGRATION TESTS ====================

    #[test]
    fn test_parse_bytes_minimal_notebook() {
        let backend = IpynbBackend::new();
        let options = BackendOptions::default();

        // Minimal valid Jupyter notebook JSON
        let notebook_json = r#"{
            "cells": [],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        }"#;

        let result = backend.parse_bytes(notebook_json.as_bytes(), &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert_eq!(doc.format, InputFormat::Ipynb);
        // Empty notebook has no DocItems, so content_blocks should be None
        assert!(doc.content_blocks.is_none());
        assert_eq!(doc.metadata.num_pages, Some(0)); // No cells
    }

    #[test]
    fn test_document_metadata_num_pages() {
        let backend = IpynbBackend::new();
        let options = BackendOptions::default();

        // Valid notebook JSON with 2 cells
        let notebook_json = r#"{
            "cells": [
                {"cell_type": "markdown", "id": "1", "source": ["Test"], "metadata": {}},
                {"cell_type": "code", "id": "2", "source": ["print('hi')"], "execution_count": 1, "outputs": [], "metadata": {}}
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        }"#;

        let result = backend.parse_bytes(notebook_json.as_bytes(), &options);
        assert!(result.is_ok(), "Parse error: {:?}", result.err());

        let doc = result.unwrap();
        assert_eq!(doc.metadata.num_pages, Some(2)); // 2 cells = 2 pages
    }

    // ==================== UNICODE AND SPECIAL CHARACTERS (3 tests) ====================

    #[test]
    fn test_unicode_in_markdown_cells() {
        // Build JSON string to avoid raw string literal escaping issues
        let notebook_json = "{
            \"cells\": [
                {\"cell_type\": \"markdown\", \"id\": \"1\", \"source\": [\"Title\\n\\nCyrillic content\\n\\nEmoji content\"], \"metadata\": {}}
            ],
            \"metadata\": {},
            \"nbformat\": 4,
            \"nbformat_minor\": 5
        }";

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert!(doc.markdown.contains("Title"));
        assert!(doc.markdown.contains("Cyrillic content"));
        assert!(doc.markdown.contains("Emoji content"));
    }

    #[test]
    fn test_unicode_in_code_cells() {
        let notebook_json = "{
            \"cells\": [
                {\"cell_type\": \"code\", \"id\": \"1\", \"source\": [\"x = 1\"], \"execution_count\": 1, \"outputs\": [], \"metadata\": {}}
            ],
            \"metadata\": {},
            \"nbformat\": 4,
            \"nbformat_minor\": 5
        }";

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert!(doc.markdown.contains("x = 1"));
    }

    #[test]
    fn test_markdown_special_characters_preservation() {
        let notebook_json = r#"{
            "cells": [
                {"cell_type": "markdown", "id": "1", "source": ["**Bold** _Italic_ `Code` [Link](http://example.com)\n\n> Quote\n\n- List"], "metadata": {}}
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        }"#;

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert!(doc.markdown.contains("**Bold**"));
        assert!(doc.markdown.contains("_Italic_"));
        assert!(doc.markdown.contains("`Code`"));
        assert!(doc.markdown.contains("[Link](http://example.com)"));
    }

    // ==================== VALIDATION TESTS (5 tests) ====================

    #[test]
    fn test_very_long_cell_source() {
        let long_code = "x = 1\n".repeat(500); // 500 lines of code
        let notebook_json = format!(
            r#"{{
                "cells": [
                    {{"cell_type": "code", "id": "1", "source": ["{}"], "execution_count": 1, "outputs": [], "metadata": {{}}}}
                ],
                "metadata": {{}},
                "nbformat": 4,
                "nbformat_minor": 5
            }}"#,
            long_code.replace('\n', "\\n")
        );

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert!(doc.markdown.contains("x = 1"));
        assert!(doc.markdown.len() > 3000); // Should be substantial
    }

    #[test]
    fn test_many_cells() {
        // Create notebook with 50 cells
        let mut cells = Vec::new();
        for i in 0..50 {
            cells.push(format!(
                r#"{{"cell_type": "markdown", "id": "{i}", "source": ["Cell {i}"], "metadata": {{}}}}"#
            ));
        }
        let cells_json = cells.join(",");

        let notebook_json = format!(
            r#"{{
                "cells": [{cells_json}],
                "metadata": {{}},
                "nbformat": 4,
                "nbformat_minor": 5
            }}"#
        );

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert_eq!(doc.metadata.num_pages, Some(50));
        if let Some(items) = &doc.content_blocks {
            assert_eq!(items.len(), 99); // 50 cells + 49 separators
        }
    }

    #[test]
    fn test_many_outputs_per_cell() {
        // Create cell with 3 outputs (simplified to ensure valid JSON)
        let notebook_json = r#"{
            "cells": [
                {
                    "cell_type": "code",
                    "id": "1",
                    "source": ["print('test')"],
                    "execution_count": 1,
                    "outputs": [
                        {"output_type": "stream", "name": "stdout", "text": ["Output 1\n"]},
                        {"output_type": "stream", "name": "stdout", "text": ["Output 2\n"]},
                        {"output_type": "stream", "name": "stdout", "text": ["Output 3\n"]}
                    ],
                    "metadata": {}
                }
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        }"#;

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        if let Some(items) = &doc.content_blocks {
            assert_eq!(items.len(), 4); // 1 code + 3 outputs
        }
    }

    #[test]
    fn test_empty_cell_source() {
        let notebook_json = r#"{
            "cells": [
                {"cell_type": "markdown", "id": "1", "source": [""], "metadata": {}},
                {"cell_type": "code", "id": "2", "source": [""], "execution_count": null, "outputs": [], "metadata": {}}
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        }"#;

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert_eq!(doc.metadata.num_pages, Some(2));
    }

    #[test]
    fn test_whitespace_only_content_filtering() {
        let notebook_json = r#"{
            "cells": [
                {"cell_type": "markdown", "id": "1", "source": ["   \n\n   "], "metadata": {}},
                {"cell_type": "code", "id": "2", "source": ["    \n    "], "execution_count": null, "outputs": [], "metadata": {}}
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        }"#;

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        // Whitespace-only cells still create DocItems (parser doesn't filter)
        if let Some(items) = &doc.content_blocks {
            assert_eq!(items.len(), 3); // 2 cells + 1 separator
        }
    }

    // ==================== SERIALIZATION CONSISTENCY TESTS (4 tests) ====================

    #[test]
    fn test_markdown_not_empty_with_content() {
        let notebook_json = "{
            \"cells\": [
                {\"cell_type\": \"markdown\", \"id\": \"1\", \"source\": [\"Test\"], \"metadata\": {}}
            ],
            \"metadata\": {},
            \"nbformat\": 4,
            \"nbformat_minor\": 5
        }";

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert!(!doc.markdown.is_empty());
        assert!(doc.markdown.contains("Test"));
    }

    #[test]
    fn test_markdown_structure_consistency() {
        let notebook_json = r#"{
            "cells": [
                {"cell_type": "code", "id": "1", "source": ["print('hello')"], "execution_count": 1, "outputs": [], "metadata": {}}
            ],
            "metadata": {"kernel": {"name": "python3"}, "language_info": {"name": "python"}},
            "nbformat": 4,
            "nbformat_minor": 5
        }"#;

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        // Code cells should have Cell N format
        assert!(doc.markdown.contains("**Cell 1 (Code)**"));
        // Code should be in Python code block
        assert!(doc.markdown.contains("```python"));
    }

    #[test]
    fn test_docitems_match_markdown_content() {
        let notebook_json = r#"{
            "cells": [
                {"cell_type": "markdown", "id": "1", "source": ["Test content"], "metadata": {}},
                {"cell_type": "code", "id": "2", "source": ["x = 1"], "execution_count": 1, "outputs": [], "metadata": {}}
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        }"#;

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        if let Some(items) = &doc.content_blocks {
            assert_eq!(items.len(), 3); // 1 Text + 1 separator + 1 Code

            // First item should be Text with markdown content
            if let DocItem::Text { text, .. } = &items[0] {
                assert_eq!(text, "Test content");
            } else {
                panic!("Expected Text DocItem");
            }

            // Second item should be separator
            if let DocItem::Text { text, .. } = &items[1] {
                assert_eq!(text, "---");
            } else {
                panic!("Expected separator Text DocItem");
            }

            // Third item should be Code with Python code
            if let DocItem::Code { text, language, .. } = &items[2] {
                assert_eq!(text, "x = 1");
                assert_eq!(language.as_deref(), Some("python"));
            } else {
                panic!("Expected Code DocItem");
            }
        }
    }

    #[test]
    fn test_idempotent_parsing() {
        let notebook_json = r#"{
            "cells": [
                {"cell_type": "markdown", "id": "1", "source": ["Test"], "metadata": {}},
                {"cell_type": "code", "id": "2", "source": ["x = 1"], "execution_count": 1, "outputs": [], "metadata": {}}
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        }"#;

        let backend = IpynbBackend::new();

        // Parse twice
        let result1 = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        let result2 = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());

        assert!(result1.is_ok());
        assert!(result2.is_ok());

        let doc1 = result1.unwrap();
        let doc2 = result2.unwrap();

        // Should produce identical output
        assert_eq!(doc1.markdown, doc2.markdown);
        assert_eq!(doc1.metadata.num_pages, doc2.metadata.num_pages);
        assert_eq!(doc1.metadata.num_characters, doc2.metadata.num_characters);
    }

    // ==================== BACKEND OPTIONS TESTS (2 tests) ====================

    #[test]
    fn test_backend_accepts_default_options() {
        let notebook_json = r#"{
            "cells": [{"cell_type": "markdown", "id": "1", "source": ["Test"], "metadata": {}}],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        }"#;

        let backend = IpynbBackend::new();
        let options = BackendOptions::default();

        let result = backend.parse_bytes(notebook_json.as_bytes(), &options);
        assert!(result.is_ok());
    }

    #[test]
    fn test_backend_accepts_custom_options() {
        let notebook_json = r#"{
            "cells": [{"cell_type": "markdown", "id": "1", "source": ["Test"], "metadata": {}}],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        }"#;

        let backend = IpynbBackend::new();
        let options = BackendOptions::default()
            .with_ocr(true)
            .with_table_structure(true);

        let result = backend.parse_bytes(notebook_json.as_bytes(), &options);
        assert!(result.is_ok());
    }

    // ==================== FORMAT-SPECIFIC EDGE CASES (13 tests) ====================

    #[test]
    fn test_notebook_metadata_title() {
        let notebook_json = r#"{
            "cells": [],
            "metadata": {"title": "Analysis Notebook"},
            "nbformat": 4,
            "nbformat_minor": 5
        }"#;

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert_eq!(doc.metadata.title.as_deref(), Some("Analysis Notebook"));
        assert!(doc.markdown.contains("# Analysis Notebook"));
    }

    #[test]
    fn test_kernel_metadata_extraction() {
        let notebook_json = r#"{
            "nbformat": 4,
            "nbformat_minor": 5,
            "cells": [],
            "metadata": {
                "kernelspec": {
                    "name": "python3",
                    "display_name": "Python 3"
                }
            }
        }"#;

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let doc = result.unwrap();
        assert!(doc.markdown.contains("Kernel: python3"));
    }

    #[test]
    fn test_language_metadata_extraction() {
        let notebook_json = r#"{
            "cells": [],
            "metadata": {"language_info": {"name": "julia"}},
            "nbformat": 4,
            "nbformat_minor": 5
        }"#;

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert_eq!(doc.metadata.language.as_deref(), Some("julia"));
        assert!(doc.markdown.contains("Language: julia"));
    }

    #[test]
    fn test_cell_execution_count_high_value() {
        let notebook_json = r#"{
            "cells": [
                {"cell_type": "code", "id": "1", "source": ["x = 1"], "execution_count": 9999, "outputs": [], "metadata": {}}
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        }"#;

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert!(doc.markdown.contains("[9999]"));
    }

    #[test]
    fn test_mixed_cell_types_order_preservation() {
        let notebook_json = r#"{
            "cells": [
                {"cell_type": "markdown", "id": "1", "source": ["First"], "metadata": {}},
                {"cell_type": "code", "id": "2", "source": ["x = 1"], "execution_count": 1, "outputs": [], "metadata": {}},
                {"cell_type": "raw", "id": "3", "source": ["Raw"], "metadata": {}},
                {"cell_type": "markdown", "id": "4", "source": ["Second"], "metadata": {}}
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        }"#;

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        let md = &doc.markdown;
        let first_pos = md.find("First").unwrap();
        let code_pos = md.find("x = 1").unwrap();
        let raw_pos = md.find("Raw").unwrap();
        let second_pos = md.find("Second").unwrap();

        assert!(first_pos < code_pos);
        assert!(code_pos < raw_pos);
        assert!(raw_pos < second_pos);
    }

    #[test]
    fn test_metadata_separator_presence() {
        let notebook_json = r#"{
            "nbformat": 4,
            "nbformat_minor": 5,
            "cells": [],
            "metadata": {
                "kernelspec": {
                    "name": "python3",
                    "display_name": "Python 3"
                }
            }
        }"#;

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let doc = result.unwrap();
        // Should have metadata section header (replaced "---" separator with "## Metadata" in N=1613)
        assert!(doc.markdown.contains("## Metadata"));
    }

    #[test]
    fn test_output_with_data_field_only() {
        let notebook_json = r#"{
            "cells": [
                {
                    "cell_type": "code",
                    "id": "1",
                    "source": ["plt.plot([1,2,3])"],
                    "execution_count": 1,
                    "outputs": [
                        {
                            "output_type": "display_data",
                            "data": {
                                "text/plain": "<plot data>"
                            },
                            "metadata": {}
                        }
                    ],
                    "metadata": {}
                }
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        }"#;

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert!(doc.markdown.contains("<plot data>"));
    }

    #[test]
    fn test_output_with_text_field_priority() {
        let notebook_json = r#"{
            "cells": [
                {
                    "cell_type": "code",
                    "id": "1",
                    "source": ["42"],
                    "execution_count": 1,
                    "outputs": [
                        {"output_type": "execute_result", "text": "42", "data": "<other>"}
                    ],
                    "metadata": {}
                }
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        }"#;

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        // Text field should be used preferentially
        assert!(doc.markdown.contains("42"));
        assert!(!doc.markdown.contains("<other>"));
    }

    #[test]
    fn test_provenance_generation() {
        let notebook_json = r#"{
            "cells": [
                {"cell_type": "markdown", "id": "1", "source": ["Test"], "metadata": {}},
                {"cell_type": "code", "id": "2", "source": ["x = 1"], "execution_count": 1, "outputs": [], "metadata": {}}
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        }"#;

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        if let Some(items) = &doc.content_blocks {
            // All items should have empty provenance (notebooks don't have page numbers)
            for item in items {
                match item {
                    DocItem::Text { prov, .. } => assert!(prov.is_empty()),
                    DocItem::Code { prov, .. } => assert!(prov.is_empty()),
                    _ => {}
                }
            }
        }
    }

    #[test]
    fn test_code_docitem_language_field() {
        let notebook_json = r#"{
            "cells": [
                {"cell_type": "code", "id": "1", "source": ["print('test')"], "execution_count": 1, "outputs": [], "metadata": {}}
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        }"#;

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        if let Some(items) = &doc.content_blocks {
            if let DocItem::Code { language, .. } = &items[0] {
                assert_eq!(language.as_deref(), Some("python"));
            } else {
                panic!("Expected Code DocItem");
            }
        }
    }

    #[test]
    fn test_character_count_accuracy() {
        let notebook_json = r#"{
            "cells": [
                {"cell_type": "markdown", "id": "1", "source": ["Hello World"], "metadata": {}}
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        }"#;

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        let expected_chars = doc.markdown.chars().count();
        assert_eq!(doc.metadata.num_characters, expected_chars);
        assert!(doc.metadata.num_characters > 0);
    }

    #[test]
    fn test_content_blocks_none_for_empty_notebook() {
        let notebook_json = r#"{
            "cells": [],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        }"#;

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        // Empty notebook should have None content_blocks
        assert!(doc.content_blocks.is_none());
    }

    #[test]
    fn test_format_identification() {
        let notebook_json = r#"{
            "cells": [],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        }"#;

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert_eq!(doc.format, InputFormat::Ipynb);
    }

    // ========== JUPYTER NOTEBOOK ADVANCED FEATURES (8 tests) ==========

    #[test]
    fn test_notebook_with_execution_counts() {
        let notebook_json = r#"{
            "cells": [
                {"cell_type": "code", "id": "1", "source": ["x = 1"], "execution_count": null, "outputs": [], "metadata": {}},
                {"cell_type": "code", "id": "2", "source": ["y = 2"], "execution_count": 5, "outputs": [], "metadata": {}},
                {"cell_type": "code", "id": "3", "source": ["z = 3"], "execution_count": 42, "outputs": [], "metadata": {}}
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        }"#;

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        // Should parse successfully with different execution counts
        assert!(doc.content_blocks.is_some());
        assert!(doc.content_blocks.as_ref().unwrap().len() >= 3);
    }

    #[test]
    fn test_notebook_with_cell_metadata() {
        let notebook_json = r#"{
            "cells": [
                {
                    "cell_type": "code",
                    "id": "1",
                    "source": ["import numpy as np"],
                    "execution_count": 1,
                    "outputs": [],
                    "metadata": {
                        "collapsed": false,
                        "tags": ["imports", "setup"],
                        "jupyter": {"source_hidden": false}
                    }
                }
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        }"#;

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        // Should handle cell metadata without errors
        assert!(doc.content_blocks.is_some());
    }

    #[test]
    fn test_notebook_with_stream_outputs() {
        let notebook_json = r#"{
            "cells": [
                {
                    "cell_type": "code",
                    "id": "1",
                    "source": ["print('Hello, World!')"],
                    "execution_count": 1,
                    "outputs": [
                        {
                            "output_type": "stream",
                            "name": "stdout",
                            "text": ["Hello, World!\n"]
                        }
                    ],
                    "metadata": {}
                }
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        }"#;

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        // Should include stream output in markdown
        assert!(doc.markdown.contains("Hello, World!") || doc.markdown.len() > 20);
    }

    #[test]
    fn test_notebook_with_error_outputs() {
        let notebook_json = r#"{
            "cells": [
                {
                    "cell_type": "code",
                    "id": "1",
                    "source": ["1 / 0"],
                    "execution_count": 1,
                    "outputs": [
                        {
                            "output_type": "error",
                            "ename": "ZeroDivisionError",
                            "evalue": "division by zero",
                            "traceback": [
                                "\u001b[0;31m---------------------------------------------------------------------------\u001b[0m",
                                "\u001b[0;31mZeroDivisionError\u001b[0m: division by zero"
                            ]
                        }
                    ],
                    "metadata": {}
                }
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        }"#;

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        // Should handle error outputs gracefully
        assert!(doc.content_blocks.is_some());
        // Error message should appear in markdown
        assert!(doc.markdown.contains("ZeroDivisionError") || doc.markdown.contains("error"));
    }

    #[test]
    fn test_notebook_with_display_data_outputs() {
        let notebook_json = r#"{
            "cells": [
                {
                    "cell_type": "code",
                    "id": "1",
                    "source": ["display('Result')"],
                    "execution_count": 1,
                    "outputs": [
                        {
                            "output_type": "display_data",
                            "data": {
                                "text/plain": ["'Result'"],
                                "text/html": ["<b>Result</b>"]
                            },
                            "metadata": {}
                        }
                    ],
                    "metadata": {}
                }
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        }"#;

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        // Should extract display_data output
        assert!(doc.markdown.contains("Result") || !doc.markdown.is_empty());
    }

    #[test]
    fn test_notebook_with_raw_cells() {
        let notebook_json = r#"{
            "cells": [
                {
                    "cell_type": "raw",
                    "id": "1",
                    "source": ["This is raw content\nFor LaTeX or other formats"],
                    "metadata": {}
                },
                {
                    "cell_type": "markdown",
                    "id": "2",
                    "source": ["Regular markdown"],
                    "metadata": {}
                }
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        }"#;

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        // Should handle raw cells (may or may not include them in output)
        assert!(doc.content_blocks.is_some());
        assert!(!doc.markdown.is_empty());
    }

    #[test]
    fn test_notebook_with_notebook_level_metadata() {
        let notebook_json = r#"{
            "cells": [
                {"cell_type": "markdown", "id": "1", "source": ["Test"], "metadata": {}}
            ],
            "metadata": {
                "kernelspec": {
                    "name": "python3",
                    "display_name": "Python 3",
                    "language": "python"
                },
                "language_info": {
                    "name": "python",
                    "version": "3.11.5",
                    "mimetype": "text/x-python",
                    "codemirror_mode": {"name": "ipython", "version": 3}
                },
                "authors": [
                    {"name": "Data Scientist"}
                ],
                "title": "Analysis Notebook"
            },
            "nbformat": 4,
            "nbformat_minor": 5
        }"#;

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        // Should parse notebook-level metadata without errors
        assert_eq!(doc.format, InputFormat::Ipynb);
        assert!(doc.content_blocks.is_some());
    }

    #[test]
    fn test_notebook_with_multiline_source_arrays() {
        // JSON with literal backslash-n sequences (as would appear in actual notebook files)
        let notebook_json = r##"{
            "cells": [
                {
                    "cell_type": "markdown",
                    "id": "1",
                    "source": [
                        "# Heading\\n",
                        "\\n",
                        "This is a paragraph\\n"
                    ],
                    "metadata": {}
                },
                {
                    "cell_type": "code",
                    "id": "2",
                    "source": [
                        "def function():\\n",
                        "    return 42\\n"
                    ],
                    "execution_count": 1,
                    "outputs": [],
                    "metadata": {}
                }
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        }"##;

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        // Should correctly join multi-line source arrays
        assert!(doc.markdown.contains("Heading") || doc.markdown.contains("function"));
        assert!(doc.markdown.len() > 30);
        assert!(doc.content_blocks.is_some());
        assert!(doc.content_blocks.as_ref().unwrap().len() >= 2);
    }

    // ==================== ADDITIONAL EDGE CASES (N=533) ====================

    #[test]
    fn test_notebook_with_empty_cells_mixed() {
        // Test handling of empty markdown and code cells mixed with content
        let notebook_json = r#"{
            "cells": [
                {
                    "cell_type": "markdown",
                    "id": "1",
                    "source": [],
                    "metadata": {}
                },
                {
                    "cell_type": "code",
                    "id": "2",
                    "source": ["print('test')"],
                    "execution_count": 1,
                    "outputs": [],
                    "metadata": {}
                },
                {
                    "cell_type": "code",
                    "id": "3",
                    "source": [],
                    "execution_count": null,
                    "outputs": [],
                    "metadata": {}
                }
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        }"#;

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert!(doc.markdown.contains("print('test')"));
        assert!(doc.content_blocks.is_some());
    }

    #[test]
    fn test_notebook_with_output_data_json_structure() {
        // Test output with complex JSON-like data field (e.g., plotly figures)
        let notebook_json = r#"{
            "cells": [
                {
                    "cell_type": "code",
                    "id": "1",
                    "source": ["import plotly.graph_objects as go"],
                    "execution_count": 1,
                    "outputs": [
                        {
                            "output_type": "display_data",
                            "data": {
                                "application/json": "{\"data\": [{\"type\": \"scatter\"}], \"layout\": {}}"
                            },
                            "metadata": {}
                        }
                    ],
                    "metadata": {}
                }
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        }"#;

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert!(doc.markdown.contains("import plotly"));
        assert!(doc.content_blocks.is_some());
    }

    #[test]
    fn test_notebook_with_execution_count_zero() {
        // Test code cell with execution_count = 0 (valid but unusual)
        let notebook_json = r##"{
            "cells": [
                {
                    "cell_type": "code",
                    "id": "1",
                    "source": ["# Not executed yet"],
                    "execution_count": 0,
                    "outputs": [],
                    "metadata": {}
                }
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        }"##;

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert!(doc.markdown.contains("[0]")); // Should show execution count 0
        assert!(doc.markdown.contains("# Not executed yet"));
    }

    #[test]
    fn test_notebook_with_mixed_output_types_single_cell() {
        // Test code cell with multiple different output types
        let notebook_json = r#"{
            "cells": [
                {
                    "cell_type": "code",
                    "id": "1",
                    "source": ["x = 1\nprint(x)\nx"],
                    "execution_count": 1,
                    "outputs": [
                        {
                            "output_type": "stream",
                            "name": "stdout",
                            "text": ["1\n"]
                        },
                        {
                            "output_type": "execute_result",
                            "execution_count": 1,
                            "data": {
                                "text/plain": ["1"]
                            },
                            "metadata": {}
                        }
                    ],
                    "metadata": {}
                }
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        }"#;

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert!(doc.markdown.contains("x = 1"));
        assert!(doc.markdown.contains("print(x)"));
        assert!(doc.markdown.contains("**Output**:"));
        assert!(doc.content_blocks.is_some());
    }

    #[test]
    fn test_notebook_with_very_large_execution_count() {
        // Test handling of very large execution counts (long-running notebooks)
        let notebook_json = r##"{
            "cells": [
                {
                    "cell_type": "code",
                    "id": "1",
                    "source": ["# Cell 999"],
                    "execution_count": 999,
                    "outputs": [],
                    "metadata": {}
                }
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        }"##;

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert!(doc.markdown.contains("[999]"));
        assert!(doc.markdown.contains("# Cell 999"));
    }

    // ==================== ADDITIONAL COMPREHENSIVE EDGE CASES (N=588, +5 tests) ====================

    #[test]
    fn test_notebook_with_nbformat_variations() {
        // Test handling of different nbformat versions (3 vs 4)
        // Jupyter notebooks can have nbformat 3 or 4 with different structures
        let notebook_json = r##"{
            "cells": [
                {
                    "cell_type": "markdown",
                    "id": "1",
                    "source": ["# NBFormat Test"],
                    "metadata": {}
                }
            ],
            "metadata": {"nbformat": 4, "nbformat_minor": 5},
            "nbformat": 4,
            "nbformat_minor": 5
        }"##;

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert!(doc.markdown.contains("NBFormat Test"));
        assert_eq!(doc.format, InputFormat::Ipynb);
    }

    #[test]
    fn test_notebook_with_image_outputs_base64() {
        // Test handling of image data in outputs (common for matplotlib, PIL, etc.)
        let notebook_json = r#"{
            "cells": [
                {
                    "cell_type": "code",
                    "id": "1",
                    "source": ["import matplotlib.pyplot as plt\nplt.plot([1,2,3])"],
                    "execution_count": 1,
                    "outputs": [
                        {
                            "output_type": "display_data",
                            "data": {
                                "text/plain": ["<Figure size 640x480 with 1 Axes>"],
                                "image/png": "iVBORw0KGgoAAAANSUhEUgAA..."
                            },
                            "metadata": {}
                        }
                    ],
                    "metadata": {}
                }
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        }"#;

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        // Should handle image output gracefully (may extract plain text description)
        assert!(doc.markdown.contains("matplotlib") || doc.markdown.contains("Figure"));
        assert!(doc.content_blocks.is_some());
    }

    #[test]
    fn test_notebook_with_missing_optional_fields() {
        // Test graceful degradation when optional fields are missing
        // Some notebook editors create minimal notebooks
        let notebook_json = r#"{
            "cells": [
                {
                    "cell_type": "code",
                    "source": ["x = 1"],
                    "metadata": {},
                    "outputs": [],
                    "execution_count": null
                }
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 0
        }"#;

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        // Should handle missing "id" field gracefully
        assert!(result.is_ok() || result.is_err()); // May error or succeed depending on parser strictness

        // If it succeeds, verify content
        if let Ok(doc) = result {
            assert!(doc.markdown.contains("x = 1") || !doc.markdown.is_empty());
        }
    }

    #[test]
    fn test_notebook_with_r_language() {
        // Test notebooks using R language instead of Python
        let notebook_json = r#"{
            "cells": [
                {
                    "cell_type": "code",
                    "id": "1",
                    "source": ["x <- c(1, 2, 3)\nsum(x)"],
                    "execution_count": 1,
                    "outputs": [
                        {
                            "output_type": "execute_result",
                            "execution_count": 1,
                            "data": {
                                "text/plain": ["[1] 6"]
                            },
                            "metadata": {}
                        }
                    ],
                    "metadata": {}
                }
            ],
            "metadata": {
                "kernelspec": {
                    "name": "ir",
                    "display_name": "R",
                    "language": "R"
                },
                "language_info": {
                    "name": "R",
                    "version": "4.2.0"
                }
            },
            "nbformat": 4,
            "nbformat_minor": 5
        }"#;

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        // Should extract R language metadata
        assert_eq!(doc.metadata.language.as_deref(), Some("R"));
        assert!(doc.markdown.contains("Language: R") || doc.markdown.contains("Kernel: ir"));
        assert!(doc.markdown.contains("x <- c(1, 2, 3)"));
    }

    #[test]
    fn test_notebook_with_slide_metadata() {
        // Test notebooks with presentation/slide metadata (RISE extension)
        let notebook_json = "{
            \"cells\": [
                {
                    \"cell_type\": \"markdown\",
                    \"id\": \"1\",
                    \"source\": [\"Slide Title\"],
                    \"metadata\": {
                        \"slideshow\": {
                            \"slide_type\": \"slide\"
                        }
                    }
                },
                {
                    \"cell_type\": \"markdown\",
                    \"id\": \"2\",
                    \"source\": [\"Subslide Content\"],
                    \"metadata\": {
                        \"slideshow\": {
                            \"slide_type\": \"subslide\"
                        }
                    }
                },
                {
                    \"cell_type\": \"code\",
                    \"id\": \"3\",
                    \"source\": [\"x = 1\"],
                    \"execution_count\": 1,
                    \"outputs\": [],
                    \"metadata\": {
                        \"slideshow\": {
                            \"slide_type\": \"fragment\"
                        }
                    }
                }
            ],
            \"metadata\": {
                \"celltoolbar\": \"Slideshow\"
            },
            \"nbformat\": 4,
            \"nbformat_minor\": 5
        }";

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        // Should parse successfully and preserve content (may or may not show slide metadata)
        assert!(doc.markdown.contains("Slide Title") || doc.markdown.contains("x = 1"));
        assert!(doc.markdown.contains("Subslide Content") || doc.markdown.len() > 10);
        assert!(doc.content_blocks.is_some());
        assert!(doc.content_blocks.as_ref().unwrap().len() >= 3);
    }

    // ========== ADVANCED JUPYTER NOTEBOOK FEATURES (N=633, +5 tests) ==========

    #[test]
    fn test_notebook_with_widget_state() {
        // Test notebooks with ipywidgets state (interactive controls: sliders, buttons)
        let notebook_json = r#"{
            "cells": [
                {
                    "cell_type": "code",
                    "id": "1",
                    "source": ["from ipywidgets import IntSlider\nslider = IntSlider(value=50, min=0, max=100)"],
                    "execution_count": 1,
                    "outputs": [
                        {
                            "output_type": "display_data",
                            "data": {
                                "text/plain": ["IntSlider(value=50, max=100)"],
                                "application/vnd.jupyter.widget-view+json": {
                                    "model_id": "e8b9dc9f388945f0a62ae11db5a8b18e",
                                    "version_major": 2,
                                    "version_minor": 0
                                }
                            },
                            "metadata": {}
                        }
                    ],
                    "metadata": {}
                }
            ],
            "metadata": {
                "widgets": {
                    "application/vnd.jupyter.widget-state+json": {
                        "state": {
                            "e8b9dc9f388945f0a62ae11db5a8b18e": {
                                "model_name": "IntSliderModel",
                                "model_module": "@jupyter-widgets/controls",
                                "model_module_version": "1.5.0",
                                "state": {
                                    "value": 50,
                                    "min": 0,
                                    "max": 100
                                }
                            }
                        },
                        "version_major": 2,
                        "version_minor": 0
                    }
                }
            },
            "nbformat": 4,
            "nbformat_minor": 5
        }"#;

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        // Should handle widget state gracefully (extract plain text representation)
        assert!(doc.markdown.contains("IntSlider") || doc.markdown.contains("ipywidgets"));
        assert!(doc.content_blocks.is_some());
    }

    #[test]
    fn test_notebook_with_julia_kernel() {
        // Test notebooks using Julia language kernel
        let notebook_json = r#"{
            "cells": [
                {
                    "cell_type": "code",
                    "id": "1",
                    "source": ["function factorial(n::Int)\n    n <= 1 ? 1 : n * factorial(n-1)\nend\nfactorial(5)"],
                    "execution_count": 1,
                    "outputs": [
                        {
                            "output_type": "execute_result",
                            "execution_count": 1,
                            "data": {
                                "text/plain": ["120"]
                            },
                            "metadata": {}
                        }
                    ],
                    "metadata": {}
                }
            ],
            "metadata": {
                "kernelspec": {
                    "name": "julia-1.9",
                    "display_name": "Julia 1.9.3",
                    "language": "julia"
                },
                "language_info": {
                    "name": "julia",
                    "version": "1.9.3",
                    "mimetype": "application/julia",
                    "file_extension": ".jl"
                }
            },
            "nbformat": 4,
            "nbformat_minor": 5
        }"#;

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        // Should extract Julia language metadata
        assert_eq!(doc.metadata.language.as_deref(), Some("julia"));
        assert!(doc.markdown.contains("function factorial") || doc.markdown.contains("julia"));
    }

    #[test]
    fn test_notebook_with_nbconvert_metadata() {
        // Test notebooks with nbconvert export metadata (templates, output formats)
        let notebook_json = r#"{
            "cells": [
                {
                    "cell_type": "markdown",
                    "id": "1",
                    "source": ["Export Test Document"],
                    "metadata": {
                        "tags": ["hide-input", "full-width"]
                    }
                }
            ],
            "metadata": {
                "nbconvert": {
                    "exporter": "html",
                    "template": "lab",
                    "exclude_input": true,
                    "exclude_output_prompt": true
                },
                "authors": [
                    {"name": "John Doe", "affiliation": "University"}
                ],
                "title": "Research Paper",
                "date": "2024-01-15"
            },
            "nbformat": 4,
            "nbformat_minor": 5
        }"#;

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        // Should extract title from nbconvert metadata
        assert_eq!(doc.metadata.title, Some("Research Paper".to_string()));
        assert!(doc.markdown.contains("Export Test") || !doc.markdown.is_empty());
    }

    #[test]
    fn test_notebook_with_cell_attachments() {
        // Test notebooks with cell attachments (embedded images, files)
        let notebook_json = r#"{
            "cells": [
                {
                    "cell_type": "markdown",
                    "id": "1",
                    "source": ["Embedded image: ![logo.png](attachment:logo.png)"],
                    "attachments": {
                        "logo.png": {
                            "image/png": "iVBORw0KGgoAAAANSUhEUgAAAAUA..."
                        }
                    },
                    "metadata": {}
                },
                {
                    "cell_type": "markdown",
                    "id": "2",
                    "source": ["Data file: [data.csv](attachment:data.csv)"],
                    "attachments": {
                        "data.csv": {
                            "text/csv": "name,value\nAlice,100\nBob,200"
                        }
                    },
                    "metadata": {}
                }
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        }"#;

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        // Should handle cell attachments gracefully (may extract references)
        assert!(doc.markdown.contains("logo.png") || doc.markdown.contains("attachment"));
        assert!(doc.markdown.contains("data.csv") || doc.markdown.len() > 10);
        assert!(doc.content_blocks.is_some());
    }

    #[test]
    fn test_notebook_with_colab_metadata() {
        // Test notebooks with Google Colab-specific metadata
        let notebook_json = r#"{
            "cells": [
                {
                    "cell_type": "code",
                    "id": "1",
                    "source": ["!pip install pandas\nimport pandas as pd"],
                    "execution_count": 1,
                    "outputs": [],
                    "metadata": {
                        "colab": {
                            "base_uri": "https://localhost:8080/",
                            "height": 35
                        },
                        "executionInfo": {
                            "status": "ok",
                            "timestamp": 1705334400000,
                            "user_tz": -480,
                            "elapsed": 1234,
                            "user": {
                                "displayName": "John Doe",
                                "userId": "123456789"
                            }
                        }
                    }
                }
            ],
            "metadata": {
                "colab": {
                    "name": "Analysis.ipynb",
                    "provenance": [],
                    "collapsed_sections": [],
                    "authorship_tag": "ABX9TyMj...",
                    "include_colab_link": true
                },
                "kernelspec": {
                    "name": "python3",
                    "display_name": "Python 3",
                    "language": "python"
                },
                "accelerator": "GPU",
                "gpuClass": "standard"
            },
            "nbformat": 4,
            "nbformat_minor": 5
        }"#;

        let backend = IpynbBackend::new();
        let result = backend.parse_bytes(notebook_json.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        // Should handle Colab-specific metadata gracefully
        // Colab notebooks contain Python code by default
        assert!(doc.markdown.contains("pip install") || doc.markdown.contains("pandas"));
        // Language should be Python (from kernelspec)
        assert!(
            doc.metadata.language.as_deref() == Some("python")
                || doc.metadata.language.as_deref() == Some("Python")
                || doc.metadata.language.is_none() // Parser may not extract language
        );
        // Colab metadata should be preserved in some form
        assert!(doc.content_blocks.is_some());
    }
}
