use crate::error::Result;
use jupyter_protocol::media::MediaType;
use nbformat::v4::{Cell, Notebook, Output};
use std::fs;
use std::path::Path;

/// Parsed Jupyter Notebook content
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct ParsedNotebook {
    /// Notebook-level metadata
    pub metadata: NotebookMetadata,
    /// List of cells in the notebook
    pub cells: Vec<NotebookCell>,
}

/// Notebook-level metadata
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct NotebookMetadata {
    /// Kernel name (e.g., "python3", "ir")
    pub kernel_name: Option<String>,
    /// Programming language name (e.g., "python", "R")
    pub language_name: Option<String>,
    /// List of author names
    pub authors: Vec<String>,
    /// Notebook title if specified
    pub title: Option<String>,
}

/// Individual notebook cell
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct NotebookCell {
    /// Unique cell identifier
    pub cell_id: Option<String>,
    /// Type of cell (code, markdown, raw)
    pub cell_type: CellType,
    /// Cell source content
    pub source: String,
    /// Execution count for code cells
    pub execution_count: Option<i32>,
    /// Cell outputs (for executed code cells)
    pub outputs: Vec<CellOutputData>,
}

/// Type of notebook cell
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum CellType {
    /// Executable code cell
    #[default]
    Code,
    /// Markdown documentation cell
    Markdown,
    /// Raw text cell (no formatting)
    Raw,
}

impl std::fmt::Display for CellType {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Code => "code",
            Self::Markdown => "markdown",
            Self::Raw => "raw",
        };
        write!(f, "{s}")
    }
}

impl std::str::FromStr for CellType {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "code" => Ok(Self::Code),
            "markdown" | "md" => Ok(Self::Markdown),
            "raw" | "text" => Ok(Self::Raw),
            _ => Err(format!(
                "Unknown cell type '{s}'. Expected: code, markdown, raw"
            )),
        }
    }
}

/// Cell output data
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct CellOutputData {
    /// Type of output (`stream`, `display_data`, `execute_result`, `error`)
    pub output_type: OutputType,
    /// Text content of the output
    pub text: Option<String>,
    /// Additional data (e.g., base64-encoded images)
    pub data: Option<String>,
}

/// Type of cell output
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum OutputType {
    /// Stream output (stdout/stderr)
    #[default]
    Stream,
    /// Rich display data (images, HTML, etc.)
    DisplayData,
    /// Result of code execution
    ExecuteResult,
    /// Error traceback
    Error,
}

impl std::fmt::Display for OutputType {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Stream => "stream",
            Self::DisplayData => "display_data",
            Self::ExecuteResult => "execute_result",
            Self::Error => "error",
        };
        write!(f, "{s}")
    }
}

impl std::str::FromStr for OutputType {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "stream" | "stdout" | "stderr" => Ok(Self::Stream),
            "display_data" | "displaydata" | "display" => Ok(Self::DisplayData),
            "execute_result" | "executeresult" | "result" => Ok(Self::ExecuteResult),
            "error" | "traceback" => Ok(Self::Error),
            _ => Err(format!(
                "Unknown output type '{s}'. Expected: stream, display_data, execute_result, error"
            )),
        }
    }
}

/// Parse a Jupyter Notebook from a file path
///
/// # Errors
///
/// Returns an error if:
/// - The file cannot be read (I/O error)
/// - The notebook JSON is malformed
#[must_use = "this function returns a parsed notebook that should be processed"]
pub fn parse_notebook<P: AsRef<Path>>(path: P) -> Result<ParsedNotebook> {
    let content = fs::read_to_string(path)?;
    parse_notebook_from_str(&content)
}

/// Parse a Jupyter Notebook from a string
///
/// # Errors
///
/// Returns an error if the notebook JSON is malformed.
#[must_use = "this function returns a parsed notebook that should be processed"]
pub fn parse_notebook_from_str(content: &str) -> Result<ParsedNotebook> {
    // Parse using nbformat crate
    let notebook: Notebook = serde_json::from_str(content)?;

    // Extract metadata
    let metadata = extract_metadata(&notebook);

    // Extract cells
    let cells = extract_cells(&notebook);

    Ok(ParsedNotebook { metadata, cells })
}

/// Extract notebook metadata
fn extract_metadata(notebook: &Notebook) -> NotebookMetadata {
    let kernel_name = notebook
        .metadata
        .kernelspec
        .as_ref()
        .map(|ks| ks.name.clone());

    let language_name = notebook
        .metadata
        .language_info
        .as_ref()
        .map(|li| li.name.clone());

    let authors = notebook
        .metadata
        .authors
        .as_ref()
        .map(|authors| authors.iter().map(|a| a.name.clone()).collect())
        .unwrap_or_default();

    let title = notebook
        .metadata
        .additional
        .get("title")
        .and_then(|v| v.as_str())
        .map(String::from);

    NotebookMetadata {
        kernel_name,
        language_name,
        authors,
        title,
    }
}

/// Extract cells from notebook
fn extract_cells(notebook: &Notebook) -> Vec<NotebookCell> {
    let mut cells = Vec::new();

    for cell in &notebook.cells {
        match cell {
            Cell::Code {
                id,
                source,
                execution_count,
                outputs,
                ..
            } => {
                let source_text = source.join("");
                let cell_outputs = extract_outputs(outputs);
                cells.push(NotebookCell {
                    cell_id: Some(id.to_string()),
                    cell_type: CellType::Code,
                    source: source_text,
                    execution_count: *execution_count,
                    outputs: cell_outputs,
                });
            }
            Cell::Markdown { id, source, .. } => {
                let source_text = source.join("");
                cells.push(NotebookCell {
                    cell_id: Some(id.to_string()),
                    cell_type: CellType::Markdown,
                    source: source_text,
                    execution_count: None,
                    outputs: Vec::new(),
                });
            }
            Cell::Raw { id, source, .. } => {
                let source_text = source.join("");
                cells.push(NotebookCell {
                    cell_id: Some(id.to_string()),
                    cell_type: CellType::Raw,
                    source: source_text,
                    execution_count: None,
                    outputs: Vec::new(),
                });
            }
        }
    }

    cells
}

/// Extract outputs from code cell
fn extract_outputs(outputs: &[Output]) -> Vec<CellOutputData> {
    let mut result = Vec::new();

    for output in outputs {
        match output {
            Output::Stream { text, .. } => {
                result.push(CellOutputData {
                    output_type: OutputType::Stream,
                    text: Some(text.0.clone()),
                    data: None,
                });
            }
            Output::DisplayData(display_data) => {
                // Try to extract text/plain representation from media content
                let text =
                    display_data
                        .data
                        .content
                        .iter()
                        .find_map(|media_type| match media_type {
                            MediaType::Plain(s) => Some(s.clone()),
                            _ => None,
                        });

                result.push(CellOutputData {
                    output_type: OutputType::DisplayData,
                    text,
                    data: None,
                });
            }
            Output::ExecuteResult(execute_result) => {
                // Try to extract text/plain representation from media content
                let text =
                    execute_result
                        .data
                        .content
                        .iter()
                        .find_map(|media_type| match media_type {
                            MediaType::Plain(s) => Some(s.clone()),
                            _ => None,
                        });

                result.push(CellOutputData {
                    output_type: OutputType::ExecuteResult,
                    text,
                    data: None,
                });
            }
            Output::Error(error_output) => {
                let text = format!(
                    "{}: {}\n{}",
                    error_output.ename,
                    error_output.evalue,
                    error_output.traceback.join("\n")
                );

                result.push(CellOutputData {
                    output_type: OutputType::Error,
                    text: Some(text),
                    data: None,
                });
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_notebook() {
        let notebook_json = r##"{
            "nbformat": 4,
            "nbformat_minor": 5,
            "metadata": {
                "kernelspec": {
                    "name": "python3",
                    "display_name": "Python 3"
                },
                "language_info": {
                    "name": "python",
                    "version": "3.9.0"
                }
            },
            "cells": [
                {
                    "id": "cell-1",
                    "cell_type": "markdown",
                    "metadata": {},
                    "source": ["# Hello World\n", "This is a test notebook."]
                },
                {
                    "id": "cell-2",
                    "cell_type": "code",
                    "metadata": {},
                    "execution_count": 1,
                    "source": ["print(\"Hello, World!\")"],
                    "outputs": [
                        {
                            "output_type": "stream",
                            "name": "stdout",
                            "text": ["Hello, World!\n"]
                        }
                    ]
                }
            ]
        }"##;

        let result = parse_notebook_from_str(notebook_json);
        assert!(
            result.is_ok(),
            "Failed to parse notebook: {:?}",
            result.err()
        );

        let notebook = result.unwrap();
        assert_eq!(notebook.cells.len(), 2);
        assert_eq!(notebook.cells[0].cell_type, CellType::Markdown);
        assert_eq!(notebook.cells[1].cell_type, CellType::Code);
        assert_eq!(notebook.metadata.kernel_name, Some("python3".to_string()));
    }

    #[test]
    fn test_extract_code_output() {
        let notebook_json = r#"{
            "nbformat": 4,
            "nbformat_minor": 5,
            "metadata": {},
            "cells": [
                {
                    "id": "cell-1",
                    "cell_type": "code",
                    "metadata": {},
                    "execution_count": 1,
                    "source": ["2 + 2"],
                    "outputs": [
                        {
                            "output_type": "execute_result",
                            "execution_count": 1,
                            "data": {
                                "text/plain": "4"
                            },
                            "metadata": {}
                        }
                    ]
                }
            ]
        }"#;

        let notebook = parse_notebook_from_str(notebook_json).unwrap();
        assert_eq!(notebook.cells.len(), 1);
        assert_eq!(notebook.cells[0].outputs.len(), 1);
        assert_eq!(
            notebook.cells[0].outputs[0].output_type,
            OutputType::ExecuteResult
        );
        assert_eq!(notebook.cells[0].outputs[0].text, Some("4".to_string()));
    }

    #[test]
    fn test_error_output() {
        let notebook_json = r#"{
            "nbformat": 4,
            "nbformat_minor": 5,
            "metadata": {},
            "cells": [
                {
                    "id": "cell-1",
                    "cell_type": "code",
                    "metadata": {},
                    "execution_count": 1,
                    "source": ["1 / 0"],
                    "outputs": [
                        {
                            "output_type": "error",
                            "ename": "ZeroDivisionError",
                            "evalue": "division by zero",
                            "traceback": [
                                "Traceback (most recent call last):",
                                "  File \"<stdin>\", line 1, in <module>",
                                "ZeroDivisionError: division by zero"
                            ]
                        }
                    ]
                }
            ]
        }"#;

        let notebook = parse_notebook_from_str(notebook_json).unwrap();
        assert_eq!(notebook.cells.len(), 1);
        assert_eq!(notebook.cells[0].outputs.len(), 1);
        assert_eq!(notebook.cells[0].outputs[0].output_type, OutputType::Error);
        assert!(notebook.cells[0].outputs[0]
            .text
            .as_ref()
            .unwrap()
            .contains("ZeroDivisionError"));
    }

    #[test]
    fn test_cell_type_display() {
        assert_eq!(format!("{}", CellType::Code), "code");
        assert_eq!(format!("{}", CellType::Markdown), "markdown");
        assert_eq!(format!("{}", CellType::Raw), "raw");
    }

    #[test]
    fn test_output_type_display() {
        assert_eq!(format!("{}", OutputType::Stream), "stream");
        assert_eq!(format!("{}", OutputType::DisplayData), "display_data");
        assert_eq!(format!("{}", OutputType::ExecuteResult), "execute_result");
        assert_eq!(format!("{}", OutputType::Error), "error");
    }

    #[test]
    fn test_cell_type_from_str() {
        // Exact matches
        assert_eq!("code".parse::<CellType>().unwrap(), CellType::Code);
        assert_eq!("markdown".parse::<CellType>().unwrap(), CellType::Markdown);
        assert_eq!("raw".parse::<CellType>().unwrap(), CellType::Raw);

        // Short aliases
        assert_eq!("md".parse::<CellType>().unwrap(), CellType::Markdown);
        assert_eq!("text".parse::<CellType>().unwrap(), CellType::Raw);

        // Case insensitive
        assert_eq!("CODE".parse::<CellType>().unwrap(), CellType::Code);
        assert_eq!("Markdown".parse::<CellType>().unwrap(), CellType::Markdown);

        // Invalid
        assert!("invalid".parse::<CellType>().is_err());
    }

    #[test]
    fn test_cell_type_roundtrip() {
        for cell_type in [CellType::Code, CellType::Markdown, CellType::Raw] {
            let s = cell_type.to_string();
            let parsed: CellType = s.parse().unwrap();
            assert_eq!(parsed, cell_type);
        }
    }

    #[test]
    fn test_output_type_from_str() {
        // Exact matches
        assert_eq!("stream".parse::<OutputType>().unwrap(), OutputType::Stream);
        assert_eq!(
            "display_data".parse::<OutputType>().unwrap(),
            OutputType::DisplayData
        );
        assert_eq!(
            "execute_result".parse::<OutputType>().unwrap(),
            OutputType::ExecuteResult
        );
        assert_eq!("error".parse::<OutputType>().unwrap(), OutputType::Error);

        // Short aliases
        assert_eq!("stdout".parse::<OutputType>().unwrap(), OutputType::Stream);
        assert_eq!("stderr".parse::<OutputType>().unwrap(), OutputType::Stream);
        assert_eq!(
            "display".parse::<OutputType>().unwrap(),
            OutputType::DisplayData
        );
        assert_eq!(
            "result".parse::<OutputType>().unwrap(),
            OutputType::ExecuteResult
        );
        assert_eq!(
            "traceback".parse::<OutputType>().unwrap(),
            OutputType::Error
        );

        // Case insensitive
        assert_eq!("STREAM".parse::<OutputType>().unwrap(), OutputType::Stream);
        assert_eq!(
            "Display_Data".parse::<OutputType>().unwrap(),
            OutputType::DisplayData
        );

        // Hyphen variant
        assert_eq!(
            "display-data".parse::<OutputType>().unwrap(),
            OutputType::DisplayData
        );
        assert_eq!(
            "execute-result".parse::<OutputType>().unwrap(),
            OutputType::ExecuteResult
        );

        // Invalid
        assert!("invalid".parse::<OutputType>().is_err());
    }

    #[test]
    fn test_output_type_roundtrip() {
        for output_type in [
            OutputType::Stream,
            OutputType::DisplayData,
            OutputType::ExecuteResult,
            OutputType::Error,
        ] {
            let s = output_type.to_string();
            let parsed: OutputType = s.parse().unwrap();
            assert_eq!(parsed, output_type);
        }
    }
}
