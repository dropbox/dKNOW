//! Jupyter Notebook format backend for docling-core
//!
//! Processes Jupyter Notebook (.ipynb) files into markdown documents.

use std::fmt::Write;
use std::path::Path;

use crate::error::{DoclingError, Result};

/// Process a Jupyter Notebook file into markdown
///
/// # Arguments
///
/// * `path` - Path to the .ipynb file
///
/// # Returns
///
/// Returns markdown document with notebook content.
///
/// # Errors
///
/// Returns an error if the file cannot be read or if notebook parsing fails.
///
/// # Examples
///
/// ```no_run
/// use docling_core::notebook::process_ipynb;
///
/// let markdown = process_ipynb("analysis.ipynb")?;
/// println!("{}", markdown);
/// # Ok::<(), docling_core::error::DoclingError>(())
/// ```
#[must_use = "this function returns the extracted markdown content"]
pub fn process_ipynb<P: AsRef<Path>>(path: P) -> Result<String> {
    let path = path.as_ref();

    // Parse notebook file
    let notebook = docling_notebook::parse_notebook(path)
        .map_err(|e| DoclingError::ConversionError(format!("Failed to parse notebook: {e}")))?;

    // Start building markdown output
    let mut markdown = String::new();

    // Add title
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("notebook.ipynb");

    let notebook_title = notebook
        .metadata
        .title
        .clone()
        .unwrap_or_else(|| filename.to_string());
    let _ = writeln!(markdown, "# {notebook_title}\n");

    // Add metadata section
    markdown.push_str("## Notebook Information\n\n");
    markdown.push_str("- **Format:** Jupyter Notebook\n");
    if let Some(kernel) = &notebook.metadata.kernel_name {
        let _ = writeln!(markdown, "- **Kernel:** {kernel}");
    }
    if let Some(language) = &notebook.metadata.language_name {
        let _ = writeln!(markdown, "- **Language:** {language}");
    }
    if !notebook.metadata.authors.is_empty() {
        markdown.push_str("- **Authors:** ");
        markdown.push_str(&notebook.metadata.authors.join(", "));
        markdown.push('\n');
    }
    let _ = writeln!(markdown, "- **Cells:** {}", notebook.cells.len());
    markdown.push('\n');

    // Add cells section
    markdown.push_str("---\n\n");

    for (i, cell) in notebook.cells.iter().enumerate() {
        match cell.cell_type {
            docling_notebook::CellType::Markdown => {
                // Markdown cells are rendered directly
                markdown.push_str(&cell.source);
                if !cell.source.ends_with('\n') {
                    markdown.push('\n');
                }
                markdown.push('\n');
            }
            docling_notebook::CellType::Code => {
                // Code cells are rendered in code blocks with their outputs
                let _ = writeln!(markdown, "**In [{}]:**\n", i + 1);

                // Add code
                markdown.push_str("```");
                if let Some(language) = &notebook.metadata.language_name {
                    markdown.push_str(language);
                }
                markdown.push('\n');
                markdown.push_str(&cell.source);
                if !cell.source.ends_with('\n') {
                    markdown.push('\n');
                }
                markdown.push_str("```\n\n");

                // Add outputs if present
                if !cell.outputs.is_empty() {
                    let _ = writeln!(markdown, "**Out [{}]:**\n", i + 1);
                    for output in &cell.outputs {
                        match output.output_type {
                            docling_notebook::OutputType::Stream
                            | docling_notebook::OutputType::ExecuteResult
                            | docling_notebook::OutputType::DisplayData => {
                                if let Some(text) = &output.text {
                                    markdown.push_str("```\n");
                                    markdown.push_str(text);
                                    if !text.ends_with('\n') {
                                        markdown.push('\n');
                                    }
                                    markdown.push_str("```\n\n");
                                }
                            }
                            docling_notebook::OutputType::Error => {
                                if let Some(text) = &output.text {
                                    markdown.push_str("**Error:**\n\n```\n");
                                    markdown.push_str(text);
                                    if !text.ends_with('\n') {
                                        markdown.push('\n');
                                    }
                                    markdown.push_str("```\n\n");
                                }
                            }
                        }
                    }
                }
            }
            docling_notebook::CellType::Raw => {
                // Raw cells are rendered as pre-formatted text
                if !cell.source.is_empty() {
                    markdown.push_str("```\n");
                    markdown.push_str(&cell.source);
                    if !cell.source.ends_with('\n') {
                        markdown.push('\n');
                    }
                    markdown.push_str("```\n\n");
                }
            }
        }
    }

    // If notebook is empty, add a note
    if notebook.cells.is_empty() {
        markdown.push_str("*This notebook is empty.*\n\n");
    }

    Ok(markdown)
}
