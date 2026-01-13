//! MCP Tool implementations

use crate::state::ServerState;
use serde_json::{json, Value};

/// List all available tools with their schemas
#[must_use = "returns list of available MCP tools"]
pub fn list_tools() -> Vec<Value> {
    vec![
        json!({
            "name": "docling_load_pdf",
            "description": "Load a PDF file for extraction analysis and correction",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Absolute path to PDF file" }
                },
                "required": ["path"]
            }
        }),
        json!({
            "name": "docling_get_page_image",
            "description": "Render a PDF page with ML detection overlays",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "document_id": { "type": "string" },
                    "page": { "type": "integer" },
                    "stage": { "type": "string", "enum": ["raw", "layout", "reading_order"] },
                    "show_labels": { "type": "boolean", "default": true },
                    "show_confidence": { "type": "boolean", "default": true }
                },
                "required": ["document_id", "page"]
            }
        }),
        json!({
            "name": "docling_list_elements",
            "description": "List all detected elements on a page",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "document_id": { "type": "string" },
                    "page": { "type": "integer" },
                    "min_confidence": { "type": "number", "default": 0.0 }
                },
                "required": ["document_id", "page"]
            }
        }),
        json!({
            "name": "docling_correct_bbox",
            "description": "Correct the bounding box of a detected element",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "document_id": { "type": "string" },
                    "page": { "type": "integer" },
                    "element_id": { "type": "integer" },
                    "new_bbox": { "type": "object" }
                },
                "required": ["document_id", "page", "element_id", "new_bbox"]
            }
        }),
        json!({
            "name": "docling_correct_label",
            "description": "Change the label of a detected element",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "document_id": { "type": "string" },
                    "page": { "type": "integer" },
                    "element_id": { "type": "integer" },
                    "new_label": { "type": "string" }
                },
                "required": ["document_id", "page", "element_id", "new_label"]
            }
        }),
        json!({
            "name": "docling_add_element",
            "description": "Add a missed element",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "document_id": { "type": "string" },
                    "page": { "type": "integer" },
                    "label": { "type": "string" },
                    "bbox": { "type": "object" },
                    "text": { "type": "string" }
                },
                "required": ["document_id", "page", "label", "bbox"]
            }
        }),
        json!({
            "name": "docling_delete_element",
            "description": "Delete a false positive detection",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "document_id": { "type": "string" },
                    "page": { "type": "integer" },
                    "element_id": { "type": "integer" }
                },
                "required": ["document_id", "page", "element_id"]
            }
        }),
        json!({
            "name": "docling_save_corrections",
            "description": "Save corrections to golden training set",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "document_id": { "type": "string" },
                    "output_format": { "type": "string", "enum": ["json", "coco", "yolo"] },
                    "output_path": { "type": "string" }
                },
                "required": ["document_id"]
            }
        }),
        json!({
            "name": "docling_get_correction_summary",
            "description": "Get summary of all corrections made",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "document_id": { "type": "string" }
                },
                "required": ["document_id"]
            }
        }),
    ]
}

/// Call a tool with arguments
#[must_use = "this function returns a Result that should be handled"]
pub fn call_tool(state: &mut ServerState, name: &str, args: Value) -> Result<Value, String> {
    match name {
        "docling_load_pdf" => {
            let path = args
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or("Missing path")?;
            state.load_pdf(path)
        }
        "docling_get_page_image" => {
            let doc_id = args
                .get("document_id")
                .and_then(|v| v.as_str())
                .ok_or("Missing document_id")?;
            let page = args
                .get("page")
                .and_then(serde_json::Value::as_u64)
                .ok_or("Missing page")? as usize;
            let stage = args
                .get("stage")
                .and_then(|v| v.as_str())
                .unwrap_or("reading_order");
            let show_labels = args
                .get("show_labels")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(true);
            let show_confidence = args
                .get("show_confidence")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(true);
            state.get_page_image(doc_id, page, stage, show_labels, show_confidence)
        }
        "docling_list_elements" => {
            let doc_id = args
                .get("document_id")
                .and_then(|v| v.as_str())
                .ok_or("Missing document_id")?;
            let page = args
                .get("page")
                .and_then(serde_json::Value::as_u64)
                .ok_or("Missing page")? as usize;
            let min_conf = args
                .get("min_confidence")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0) as f32;
            state.list_elements(doc_id, page, min_conf)
        }
        "docling_correct_bbox" => {
            let doc_id = args
                .get("document_id")
                .and_then(|v| v.as_str())
                .ok_or("Missing document_id")?;
            let page = args
                .get("page")
                .and_then(serde_json::Value::as_u64)
                .ok_or("Missing page")? as usize;
            let elem_id = args
                .get("element_id")
                .and_then(serde_json::Value::as_u64)
                .ok_or("Missing element_id")? as u32;
            let bbox = args.get("new_bbox").ok_or("Missing new_bbox")?.clone();
            state.correct_bbox(doc_id, page, elem_id, bbox)
        }
        "docling_correct_label" => {
            let doc_id = args
                .get("document_id")
                .and_then(|v| v.as_str())
                .ok_or("Missing document_id")?;
            let page = args
                .get("page")
                .and_then(serde_json::Value::as_u64)
                .ok_or("Missing page")? as usize;
            let elem_id = args
                .get("element_id")
                .and_then(serde_json::Value::as_u64)
                .ok_or("Missing element_id")? as u32;
            let label = args
                .get("new_label")
                .and_then(|v| v.as_str())
                .ok_or("Missing new_label")?;
            state.correct_label(doc_id, page, elem_id, label)
        }
        "docling_add_element" => {
            let doc_id = args
                .get("document_id")
                .and_then(|v| v.as_str())
                .ok_or("Missing document_id")?;
            let page = args
                .get("page")
                .and_then(serde_json::Value::as_u64)
                .ok_or("Missing page")? as usize;
            let label = args
                .get("label")
                .and_then(|v| v.as_str())
                .ok_or("Missing label")?;
            let bbox = args.get("bbox").ok_or("Missing bbox")?.clone();
            let text = args.get("text").and_then(|v| v.as_str());
            state.add_element(doc_id, page, label, bbox, text)
        }
        "docling_delete_element" => {
            let doc_id = args
                .get("document_id")
                .and_then(|v| v.as_str())
                .ok_or("Missing document_id")?;
            let page = args
                .get("page")
                .and_then(serde_json::Value::as_u64)
                .ok_or("Missing page")? as usize;
            let elem_id = args
                .get("element_id")
                .and_then(serde_json::Value::as_u64)
                .ok_or("Missing element_id")? as u32;
            state.delete_element(doc_id, page, elem_id)
        }
        "docling_save_corrections" => {
            let doc_id = args
                .get("document_id")
                .and_then(|v| v.as_str())
                .ok_or("Missing document_id")?;
            let format = args
                .get("output_format")
                .and_then(|v| v.as_str())
                .unwrap_or("json");
            let path = args.get("output_path").and_then(|v| v.as_str());
            state.save_corrections(doc_id, format, path)
        }
        "docling_get_correction_summary" => {
            let doc_id = args
                .get("document_id")
                .and_then(|v| v.as_str())
                .ok_or("Missing document_id")?;
            state.get_correction_summary(doc_id)
        }
        _ => Err(format!("Unknown tool: {name}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_temp_pdf() -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"%PDF-1.4 test").unwrap();
        file
    }

    #[test]
    fn test_list_tools_returns_all_tools() {
        let tools = list_tools();
        assert_eq!(tools.len(), 9);

        let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
        assert!(names.contains(&"docling_load_pdf"));
        assert!(names.contains(&"docling_get_page_image"));
        assert!(names.contains(&"docling_list_elements"));
        assert!(names.contains(&"docling_correct_bbox"));
        assert!(names.contains(&"docling_correct_label"));
        assert!(names.contains(&"docling_add_element"));
        assert!(names.contains(&"docling_delete_element"));
        assert!(names.contains(&"docling_save_corrections"));
        assert!(names.contains(&"docling_get_correction_summary"));
    }

    #[test]
    fn test_list_tools_have_schemas() {
        let tools = list_tools();
        for tool in &tools {
            assert!(tool.get("name").is_some());
            assert!(tool.get("description").is_some());
            assert!(tool.get("inputSchema").is_some());
        }
    }

    #[test]
    fn test_call_tool_unknown_tool() {
        let mut state = ServerState::new();
        let result = call_tool(&mut state, "unknown_tool", json!({}));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown tool"));
    }

    #[test]
    fn test_call_tool_load_pdf_missing_path() {
        let mut state = ServerState::new();
        let result = call_tool(&mut state, "docling_load_pdf", json!({}));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Missing path"));
    }

    #[test]
    fn test_call_tool_load_pdf_success() {
        let temp_file = create_temp_pdf();
        let mut state = ServerState::new();
        let result = call_tool(
            &mut state,
            "docling_load_pdf",
            json!({"path": temp_file.path().to_str().unwrap()}),
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap()["success"], true);
    }

    #[test]
    fn test_call_tool_get_page_image_missing_params() {
        let mut state = ServerState::new();

        // Missing document_id
        let result = call_tool(&mut state, "docling_get_page_image", json!({"page": 0}));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Missing document_id"));

        // Missing page
        let result = call_tool(
            &mut state,
            "docling_get_page_image",
            json!({"document_id": "doc_123"}),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Missing page"));
    }

    #[test]
    fn test_call_tool_list_elements_missing_params() {
        let mut state = ServerState::new();

        let result = call_tool(&mut state, "docling_list_elements", json!({"page": 0}));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Missing document_id"));
    }

    #[test]
    fn test_call_tool_correct_bbox_missing_params() {
        let mut state = ServerState::new();

        let result = call_tool(&mut state, "docling_correct_bbox", json!({}));
        assert!(result.is_err());

        let result = call_tool(
            &mut state,
            "docling_correct_bbox",
            json!({"document_id": "doc_123", "page": 0, "element_id": 1}),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Missing new_bbox"));
    }

    #[test]
    fn test_call_tool_correct_label_missing_params() {
        let mut state = ServerState::new();

        let result = call_tool(
            &mut state,
            "docling_correct_label",
            json!({"document_id": "doc_123", "page": 0, "element_id": 1}),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Missing new_label"));
    }

    #[test]
    fn test_call_tool_add_element_missing_params() {
        let mut state = ServerState::new();

        let result = call_tool(
            &mut state,
            "docling_add_element",
            json!({"document_id": "doc_123", "page": 0, "label": "Text"}),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Missing bbox"));
    }

    #[test]
    fn test_call_tool_delete_element_missing_params() {
        let mut state = ServerState::new();

        let result = call_tool(
            &mut state,
            "docling_delete_element",
            json!({"document_id": "doc_123", "page": 0}),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Missing element_id"));
    }

    #[test]
    fn test_call_tool_save_corrections_missing_params() {
        let mut state = ServerState::new();

        let result = call_tool(&mut state, "docling_save_corrections", json!({}));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Missing document_id"));
    }

    #[test]
    fn test_call_tool_get_summary_missing_params() {
        let mut state = ServerState::new();

        let result = call_tool(&mut state, "docling_get_correction_summary", json!({}));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Missing document_id"));
    }

    #[test]
    fn test_call_tool_full_workflow() {
        let temp_file = create_temp_pdf();
        let mut state = ServerState::new();

        // Load PDF
        let load_result = call_tool(
            &mut state,
            "docling_load_pdf",
            json!({"path": temp_file.path().to_str().unwrap()}),
        )
        .unwrap();
        let doc_id = load_result["document_id"].as_str().unwrap();

        // Get page image
        let result = call_tool(
            &mut state,
            "docling_get_page_image",
            json!({"document_id": doc_id, "page": 0}),
        );
        assert!(result.is_ok());

        // Add element
        let add_result = call_tool(
            &mut state,
            "docling_add_element",
            json!({
                "document_id": doc_id,
                "page": 0,
                "label": "Text",
                "bbox": {"x": 10, "y": 20, "width": 100, "height": 50}
            }),
        )
        .unwrap();
        assert_eq!(add_result["element_id"], 1);

        // List elements
        let list_result = call_tool(
            &mut state,
            "docling_list_elements",
            json!({"document_id": doc_id, "page": 0}),
        )
        .unwrap();
        assert_eq!(list_result["total"], 1);

        // Correct bbox
        let result = call_tool(
            &mut state,
            "docling_correct_bbox",
            json!({
                "document_id": doc_id,
                "page": 0,
                "element_id": 1,
                "new_bbox": {"x": 15, "y": 25, "width": 110, "height": 55}
            }),
        );
        assert!(result.is_ok());

        // Correct label
        let result = call_tool(
            &mut state,
            "docling_correct_label",
            json!({
                "document_id": doc_id,
                "page": 0,
                "element_id": 1,
                "new_label": "Title"
            }),
        );
        assert!(result.is_ok());

        // Get summary
        let summary = call_tool(
            &mut state,
            "docling_get_correction_summary",
            json!({"document_id": doc_id}),
        )
        .unwrap();
        assert_eq!(summary["total"], 3); // add + bbox + label

        // Delete element
        let result = call_tool(
            &mut state,
            "docling_delete_element",
            json!({"document_id": doc_id, "page": 0, "element_id": 1}),
        );
        assert!(result.is_ok());
    }
}
