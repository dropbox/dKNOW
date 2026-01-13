//! Server state management

use crate::corrections::{Correction, CorrectionTracker};
use serde_json::{json, Value};
use std::collections::HashMap;
use uuid::Uuid;

struct LoadedDocument {
    path: String,
    page_count: usize,
    corrections: CorrectionTracker,
    elements: HashMap<usize, Vec<Element>>,
}

#[derive(Debug, Clone, PartialEq)]
struct Element {
    id: u32,
    label: String,
    confidence: f32,
    bbox: BBox,
    text: Option<String>,
    reading_order: i32,
    deleted: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct BBox {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

#[derive(Default)]
pub struct ServerState {
    documents: HashMap<String, LoadedDocument>,
}

impl ServerState {
    #[inline]
    #[must_use = "creates empty server state"]
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
        }
    }

    #[must_use = "this function returns a Result that should be handled"]
    pub fn load_pdf(&mut self, path: &str) -> Result<Value, String> {
        let doc_id = format!(
            "doc_{}",
            Uuid::new_v4().to_string().split('-').next().unwrap()
        );
        if !std::path::Path::new(path).exists() {
            return Err(format!("File not found: {path}"));
        }
        let doc = LoadedDocument {
            path: path.to_string(),
            page_count: 1,
            corrections: CorrectionTracker::new(),
            elements: HashMap::new(),
        };
        self.documents.insert(doc_id.clone(), doc);
        Ok(json!({"success": true, "document_id": doc_id, "page_count": 1, "path": path}))
    }

    #[must_use = "this function returns a Result that should be handled"]
    pub fn get_page_image(
        &self,
        doc_id: &str,
        page: usize,
        _stage: &str,
        _labels: bool,
        _conf: bool,
    ) -> Result<Value, String> {
        let doc = self
            .documents
            .get(doc_id)
            .ok_or_else(|| format!("Document not found: {doc_id}"))?;
        if page >= doc.page_count {
            return Err(format!("Page {page} out of range"));
        }
        Ok(json!({"success": true, "page": page, "width": 612, "height": 792, "note": "stub"}))
    }

    #[must_use = "this function returns a Result that should be handled"]
    pub fn list_elements(&self, doc_id: &str, page: usize, min_conf: f32) -> Result<Value, String> {
        let doc = self
            .documents
            .get(doc_id)
            .ok_or_else(|| format!("Document not found: {doc_id}"))?;
        if page >= doc.page_count {
            return Err(format!("Page {page} out of range"));
        }
        let elements: Vec<Value> = doc.elements.get(&page).map(|els| {
            els.iter().filter(|e| !e.deleted && e.confidence >= min_conf).map(|e| {
                json!({"id": e.id, "label": e.label, "confidence": e.confidence, "bbox": {"x": e.bbox.x, "y": e.bbox.y, "width": e.bbox.width, "height": e.bbox.height}, "text": e.text, "reading_order": e.reading_order})
            }).collect()
        }).unwrap_or_default();
        Ok(json!({"elements": elements, "total": elements.len(), "page": page}))
    }

    #[must_use = "this function returns a Result that should be handled"]
    pub fn correct_bbox(
        &mut self,
        doc_id: &str,
        page: usize,
        elem_id: u32,
        new_bbox: Value,
    ) -> Result<Value, String> {
        let doc = self
            .documents
            .get_mut(doc_id)
            .ok_or_else(|| format!("Document not found: {doc_id}"))?;
        let x = new_bbox
            .get("x")
            .and_then(serde_json::Value::as_f64)
            .ok_or("Invalid bbox")? as f32;
        let y = new_bbox
            .get("y")
            .and_then(serde_json::Value::as_f64)
            .ok_or("Invalid bbox")? as f32;
        let w = new_bbox
            .get("width")
            .and_then(serde_json::Value::as_f64)
            .ok_or("Invalid bbox")? as f32;
        let h = new_bbox
            .get("height")
            .and_then(serde_json::Value::as_f64)
            .ok_or("Invalid bbox")? as f32;
        let elements = doc.elements.entry(page).or_default();
        let element = elements
            .iter_mut()
            .find(|e| e.id == elem_id)
            .ok_or_else(|| format!("Element {elem_id} not found"))?;
        let orig = element.bbox;
        element.bbox = BBox {
            x,
            y,
            width: w,
            height: h,
        };
        let corr_id = doc.corrections.add(Correction::BBox {
            page,
            element_id: elem_id,
            original: json!({"x": orig.x, "y": orig.y, "width": orig.width, "height": orig.height}),
            corrected: new_bbox,
        });
        Ok(json!({"success": true, "correction_id": corr_id, "element_id": elem_id}))
    }

    #[must_use = "this function returns a Result that should be handled"]
    pub fn correct_label(
        &mut self,
        doc_id: &str,
        page: usize,
        elem_id: u32,
        new_label: &str,
    ) -> Result<Value, String> {
        let doc = self
            .documents
            .get_mut(doc_id)
            .ok_or_else(|| format!("Document not found: {doc_id}"))?;
        let elements = doc.elements.entry(page).or_default();
        let element = elements
            .iter_mut()
            .find(|e| e.id == elem_id)
            .ok_or_else(|| format!("Element {elem_id} not found"))?;
        let orig = element.label.clone();
        element.label = new_label.to_string();
        let corr_id = doc.corrections.add(Correction::Label {
            page,
            element_id: elem_id,
            original: orig.clone(),
            corrected: new_label.to_string(),
        });
        Ok(json!({"success": true, "correction_id": corr_id, "from": orig, "to": new_label}))
    }

    #[must_use = "this function returns a Result that should be handled"]
    pub fn add_element(
        &mut self,
        doc_id: &str,
        page: usize,
        label: &str,
        bbox: Value,
        text: Option<&str>,
    ) -> Result<Value, String> {
        let doc = self
            .documents
            .get_mut(doc_id)
            .ok_or_else(|| format!("Document not found: {doc_id}"))?;
        let x = bbox
            .get("x")
            .and_then(serde_json::Value::as_f64)
            .ok_or("Invalid bbox")? as f32;
        let y = bbox
            .get("y")
            .and_then(serde_json::Value::as_f64)
            .ok_or("Invalid bbox")? as f32;
        let w = bbox
            .get("width")
            .and_then(serde_json::Value::as_f64)
            .ok_or("Invalid bbox")? as f32;
        let h = bbox
            .get("height")
            .and_then(serde_json::Value::as_f64)
            .ok_or("Invalid bbox")? as f32;
        let elements = doc.elements.entry(page).or_default();
        let new_id = elements.iter().map(|e| e.id).max().unwrap_or(0) + 1;
        elements.push(Element {
            id: new_id,
            label: label.to_string(),
            confidence: 1.0,
            bbox: BBox {
                x,
                y,
                width: w,
                height: h,
            },
            text: text.map(ToString::to_string),
            reading_order: -1,
            deleted: false,
        });
        let corr_id = doc.corrections.add(Correction::Add {
            page,
            element_id: new_id,
            label: label.to_string(),
            bbox,
            text: text.map(ToString::to_string),
        });
        Ok(json!({"success": true, "correction_id": corr_id, "element_id": new_id}))
    }

    #[must_use = "this function returns a Result that should be handled"]
    pub fn delete_element(
        &mut self,
        doc_id: &str,
        page: usize,
        elem_id: u32,
    ) -> Result<Value, String> {
        let doc = self
            .documents
            .get_mut(doc_id)
            .ok_or_else(|| format!("Document not found: {doc_id}"))?;
        let elements = doc.elements.entry(page).or_default();
        let element = elements
            .iter_mut()
            .find(|e| e.id == elem_id)
            .ok_or_else(|| format!("Element {elem_id} not found"))?;
        if element.deleted {
            return Err(format!("Element {elem_id} already deleted"));
        }
        element.deleted = true;
        let corr_id = doc.corrections.add(Correction::Delete {
            page,
            element_id: elem_id,
            original_label: element.label.clone(),
        });
        Ok(json!({"success": true, "correction_id": corr_id, "element_id": elem_id}))
    }

    #[must_use = "this function returns a Result that should be handled"]
    pub fn save_corrections(
        &self,
        doc_id: &str,
        format: &str,
        path: Option<&str>,
    ) -> Result<Value, String> {
        let doc = self
            .documents
            .get(doc_id)
            .ok_or_else(|| format!("Document not found: {doc_id}"))?;
        let output = path.map_or_else(
            || {
                let base = std::path::Path::new(&doc.path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("corrections");
                format!("{base}_corrections.{format}")
            },
            ToString::to_string,
        );
        let data = json!({"source_pdf": doc.path, "corrections_applied": doc.corrections.count()});
        std::fs::write(&output, serde_json::to_string_pretty(&data).unwrap())
            .map_err(|e| format!("Write failed: {e}"))?;
        Ok(json!({"success": true, "output_path": output, "format": format}))
    }

    #[must_use = "this function returns a Result that should be handled"]
    pub fn get_correction_summary(&self, doc_id: &str) -> Result<Value, String> {
        let doc = self
            .documents
            .get(doc_id)
            .ok_or_else(|| format!("Document not found: {doc_id}"))?;
        let summary = doc.corrections.summary();
        Ok(
            json!({"total": summary.total, "bbox": summary.bbox_count, "label": summary.label_count, "add": summary.add_count, "delete": summary.delete_count}),
        )
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
    fn test_new_is_empty() {
        let state = ServerState::new();
        assert!(state.documents.is_empty());
    }

    #[test]
    fn test_default_is_empty() {
        let state = ServerState::default();
        assert!(state.documents.is_empty());
    }

    #[test]
    fn test_load_pdf_file_not_found() {
        let mut state = ServerState::new();
        let result = state.load_pdf("/nonexistent/file.pdf");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("File not found"));
    }

    #[test]
    fn test_load_pdf_success() {
        let temp_file = create_temp_pdf();
        let mut state = ServerState::new();

        let result = state.load_pdf(temp_file.path().to_str().unwrap());
        assert!(result.is_ok());

        let value = result.unwrap();
        assert_eq!(value["success"], true);
        assert!(value["document_id"].as_str().unwrap().starts_with("doc_"));
        assert_eq!(value["page_count"], 1);
    }

    #[test]
    fn test_get_page_image_doc_not_found() {
        let state = ServerState::new();
        let result = state.get_page_image("invalid_doc", 0, "raw", false, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Document not found"));
    }

    #[test]
    fn test_get_page_image_page_out_of_range() {
        let temp_file = create_temp_pdf();
        let mut state = ServerState::new();
        let load_result = state.load_pdf(temp_file.path().to_str().unwrap()).unwrap();
        let doc_id = load_result["document_id"].as_str().unwrap();

        let result = state.get_page_image(doc_id, 10, "raw", false, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("out of range"));
    }

    #[test]
    fn test_get_page_image_success() {
        let temp_file = create_temp_pdf();
        let mut state = ServerState::new();
        let load_result = state.load_pdf(temp_file.path().to_str().unwrap()).unwrap();
        let doc_id = load_result["document_id"].as_str().unwrap();

        let result = state.get_page_image(doc_id, 0, "raw", true, true);
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["success"], true);
        assert_eq!(value["page"], 0);
    }

    #[test]
    fn test_list_elements_empty() {
        let temp_file = create_temp_pdf();
        let mut state = ServerState::new();
        let load_result = state.load_pdf(temp_file.path().to_str().unwrap()).unwrap();
        let doc_id = load_result["document_id"].as_str().unwrap();

        let result = state.list_elements(doc_id, 0, 0.0);
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["total"], 0);
        assert!(value["elements"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_add_element() {
        let temp_file = create_temp_pdf();
        let mut state = ServerState::new();
        let load_result = state.load_pdf(temp_file.path().to_str().unwrap()).unwrap();
        let doc_id = load_result["document_id"].as_str().unwrap();

        let bbox = json!({"x": 10.0, "y": 20.0, "width": 100.0, "height": 50.0});
        let result = state.add_element(doc_id, 0, "Text", bbox, Some("Hello world"));
        assert!(result.is_ok());

        let value = result.unwrap();
        assert_eq!(value["success"], true);
        assert_eq!(value["element_id"], 1);
        assert!(value["correction_id"]
            .as_str()
            .unwrap()
            .starts_with("corr_"));
    }

    #[test]
    fn test_add_and_list_elements() {
        let temp_file = create_temp_pdf();
        let mut state = ServerState::new();
        let load_result = state.load_pdf(temp_file.path().to_str().unwrap()).unwrap();
        let doc_id = load_result["document_id"].as_str().unwrap();

        // Add two elements
        let bbox1 = json!({"x": 10.0, "y": 20.0, "width": 100.0, "height": 50.0});
        state
            .add_element(doc_id, 0, "Text", bbox1, Some("First"))
            .unwrap();

        let bbox2 = json!({"x": 10.0, "y": 80.0, "width": 100.0, "height": 30.0});
        state
            .add_element(doc_id, 0, "Title", bbox2, Some("Second"))
            .unwrap();

        // List elements
        let result = state.list_elements(doc_id, 0, 0.0).unwrap();
        assert_eq!(result["total"], 2);
        let elements = result["elements"].as_array().unwrap();
        assert_eq!(elements.len(), 2);
        assert_eq!(elements[0]["label"], "Text");
        assert_eq!(elements[1]["label"], "Title");
    }

    #[test]
    fn test_correct_bbox() {
        let temp_file = create_temp_pdf();
        let mut state = ServerState::new();
        let load_result = state.load_pdf(temp_file.path().to_str().unwrap()).unwrap();
        let doc_id = load_result["document_id"].as_str().unwrap();

        // Add an element
        let bbox = json!({"x": 10.0, "y": 20.0, "width": 100.0, "height": 50.0});
        state.add_element(doc_id, 0, "Text", bbox, None).unwrap();

        // Correct its bbox
        let new_bbox = json!({"x": 15.0, "y": 25.0, "width": 110.0, "height": 55.0});
        let result = state.correct_bbox(doc_id, 0, 1, new_bbox);
        assert!(result.is_ok());

        let value = result.unwrap();
        assert_eq!(value["success"], true);
        assert_eq!(value["element_id"], 1);
    }

    #[test]
    fn test_correct_label() {
        let temp_file = create_temp_pdf();
        let mut state = ServerState::new();
        let load_result = state.load_pdf(temp_file.path().to_str().unwrap()).unwrap();
        let doc_id = load_result["document_id"].as_str().unwrap();

        // Add an element
        let bbox = json!({"x": 10.0, "y": 20.0, "width": 100.0, "height": 50.0});
        state.add_element(doc_id, 0, "Text", bbox, None).unwrap();

        // Correct its label
        let result = state.correct_label(doc_id, 0, 1, "Title");
        assert!(result.is_ok());

        let value = result.unwrap();
        assert_eq!(value["success"], true);
        assert_eq!(value["from"], "Text");
        assert_eq!(value["to"], "Title");
    }

    #[test]
    fn test_delete_element() {
        let temp_file = create_temp_pdf();
        let mut state = ServerState::new();
        let load_result = state.load_pdf(temp_file.path().to_str().unwrap()).unwrap();
        let doc_id = load_result["document_id"].as_str().unwrap();

        // Add an element
        let bbox = json!({"x": 10.0, "y": 20.0, "width": 100.0, "height": 50.0});
        state.add_element(doc_id, 0, "Text", bbox, None).unwrap();

        // Delete it
        let result = state.delete_element(doc_id, 0, 1);
        assert!(result.is_ok());

        let value = result.unwrap();
        assert_eq!(value["success"], true);
        assert_eq!(value["element_id"], 1);

        // Verify it doesn't appear in list
        let list_result = state.list_elements(doc_id, 0, 0.0).unwrap();
        assert_eq!(list_result["total"], 0);
    }

    #[test]
    fn test_delete_already_deleted() {
        let temp_file = create_temp_pdf();
        let mut state = ServerState::new();
        let load_result = state.load_pdf(temp_file.path().to_str().unwrap()).unwrap();
        let doc_id = load_result["document_id"].as_str().unwrap();

        // Add and delete an element
        let bbox = json!({"x": 10.0, "y": 20.0, "width": 100.0, "height": 50.0});
        state.add_element(doc_id, 0, "Text", bbox, None).unwrap();
        state.delete_element(doc_id, 0, 1).unwrap();

        // Try to delete again
        let result = state.delete_element(doc_id, 0, 1);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already deleted"));
    }

    #[test]
    fn test_get_correction_summary() {
        let temp_file = create_temp_pdf();
        let mut state = ServerState::new();
        let load_result = state.load_pdf(temp_file.path().to_str().unwrap()).unwrap();
        let doc_id = load_result["document_id"].as_str().unwrap();

        // Add element (1 add correction)
        let bbox = json!({"x": 10.0, "y": 20.0, "width": 100.0, "height": 50.0});
        state.add_element(doc_id, 0, "Text", bbox, None).unwrap();

        // Correct bbox (1 bbox correction)
        let new_bbox = json!({"x": 15.0, "y": 25.0, "width": 110.0, "height": 55.0});
        state.correct_bbox(doc_id, 0, 1, new_bbox).unwrap();

        // Correct label (1 label correction)
        state.correct_label(doc_id, 0, 1, "Title").unwrap();

        // Delete element (1 delete correction)
        state.delete_element(doc_id, 0, 1).unwrap();

        // Check summary
        let result = state.get_correction_summary(doc_id).unwrap();
        assert_eq!(result["total"], 4);
        assert_eq!(result["add"], 1);
        assert_eq!(result["bbox"], 1);
        assert_eq!(result["label"], 1);
        assert_eq!(result["delete"], 1);
    }

    #[test]
    fn test_list_elements_with_confidence_filter() {
        let temp_file = create_temp_pdf();
        let mut state = ServerState::new();
        let load_result = state.load_pdf(temp_file.path().to_str().unwrap()).unwrap();
        let doc_id = load_result["document_id"].as_str().unwrap();

        // Add element (confidence = 1.0)
        let bbox = json!({"x": 10.0, "y": 20.0, "width": 100.0, "height": 50.0});
        state.add_element(doc_id, 0, "Text", bbox, None).unwrap();

        // List with high confidence filter - should include
        let result = state.list_elements(doc_id, 0, 0.9).unwrap();
        assert_eq!(result["total"], 1);

        // List with impossible confidence filter - should exclude
        let result = state.list_elements(doc_id, 0, 1.1).unwrap();
        assert_eq!(result["total"], 0);
    }
}
