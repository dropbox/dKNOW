//! Correction tracking

use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Correction {
    BBox {
        page: usize,
        element_id: u32,
        original: Value,
        corrected: Value,
    },
    Label {
        page: usize,
        element_id: u32,
        original: String,
        corrected: String,
    },
    Add {
        page: usize,
        element_id: u32,
        label: String,
        bbox: Value,
        text: Option<String>,
    },
    Delete {
        page: usize,
        element_id: u32,
        original_label: String,
    },
}

impl std::fmt::Display for Correction {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BBox {
                page, element_id, ..
            } => write!(f, "bbox correction on page {page}, element {element_id}"),
            Self::Label {
                page,
                element_id,
                original,
                corrected,
            } => write!(
                f,
                "label change on page {page}, element {element_id}: {original} -> {corrected}"
            ),
            Self::Add {
                page,
                element_id,
                label,
                ..
            } => write!(f, "add {label} on page {page}, element {element_id}"),
            Self::Delete {
                page,
                element_id,
                original_label,
            } => write!(
                f,
                "delete {original_label} on page {page}, element {element_id}"
            ),
        }
    }
}

/// Internal struct that tracks corrections with unique IDs.
/// The ID allows retrieval and removal via `CorrectionTracker` methods.
#[allow(
    dead_code,
    reason = "internal struct used by CorrectionTracker, fields accessed via methods"
)]
struct TrackedCorrection {
    id: String,
    correction: Correction,
}

#[derive(Default)]
pub struct CorrectionTracker {
    corrections: Vec<TrackedCorrection>,
}

impl CorrectionTracker {
    #[inline]
    #[must_use = "correction tracker is created but not used"]
    pub const fn new() -> Self {
        Self {
            corrections: Vec::new(),
        }
    }

    #[must_use = "correction ID is returned but not used"]
    pub fn add(&mut self, correction: Correction) -> String {
        let id = format!(
            "corr_{}",
            Uuid::new_v4().to_string().split('-').next().unwrap()
        );
        self.corrections.push(TrackedCorrection {
            id: id.clone(),
            correction,
        });
        id
    }

    #[inline]
    #[must_use = "correction count is returned but not used"]
    pub const fn count(&self) -> usize {
        self.corrections.len()
    }

    /// Get a correction by its ID
    #[inline]
    #[allow(dead_code, reason = "API for future MCP edit/undo features")]
    #[must_use = "looked up correction is returned but not used"]
    pub fn get_by_id(&self, id: &str) -> Option<&Correction> {
        self.corrections
            .iter()
            .find(|c| c.id == id)
            .map(|c| &c.correction)
    }

    /// Remove a correction by its ID, returning it if found
    #[inline]
    #[allow(dead_code, reason = "API for future MCP undo feature")]
    #[must_use = "removed correction is returned but not used"]
    pub fn remove_by_id(&mut self, id: &str) -> Option<Correction> {
        if let Some(pos) = self.corrections.iter().position(|c| c.id == id) {
            Some(self.corrections.remove(pos).correction)
        } else {
            None
        }
    }

    /// Get all correction IDs
    #[inline]
    #[allow(dead_code, reason = "API for future MCP listing feature")]
    #[must_use = "correction IDs are returned but not used"]
    pub fn ids(&self) -> Vec<&str> {
        self.corrections.iter().map(|c| c.id.as_str()).collect()
    }

    #[inline]
    #[must_use = "correction summary is returned but not used"]
    pub fn summary(&self) -> CorrectionSummary {
        let mut bbox = 0;
        let mut label = 0;
        let mut add = 0;
        let mut delete = 0;
        for t in &self.corrections {
            match &t.correction {
                Correction::BBox { .. } => {
                    bbox += 1;
                }
                Correction::Label { .. } => {
                    label += 1;
                }
                Correction::Add { .. } => {
                    add += 1;
                }
                Correction::Delete { .. } => {
                    delete += 1;
                }
            }
        }
        CorrectionSummary {
            total: self.corrections.len(),
            bbox_count: bbox,
            label_count: label,
            add_count: add,
            delete_count: delete,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CorrectionSummary {
    pub total: usize,
    pub bbox_count: usize,
    pub label_count: usize,
    pub add_count: usize,
    pub delete_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_bbox_correction(page: usize, element_id: u32) -> Correction {
        Correction::BBox {
            page,
            element_id,
            original: json!({"x": 0, "y": 0}),
            corrected: json!({"x": 10, "y": 10}),
        }
    }

    fn make_label_correction(page: usize, element_id: u32) -> Correction {
        Correction::Label {
            page,
            element_id,
            original: "Text".to_string(),
            corrected: "Title".to_string(),
        }
    }

    #[test]
    fn test_new_tracker_is_empty() {
        let tracker = CorrectionTracker::new();
        assert_eq!(tracker.count(), 0);
        assert!(tracker.ids().is_empty());
    }

    #[test]
    fn test_default_tracker_is_empty() {
        let tracker = CorrectionTracker::default();
        assert_eq!(tracker.count(), 0);
    }

    #[test]
    fn test_add_returns_id() {
        let mut tracker = CorrectionTracker::new();
        let id = tracker.add(make_bbox_correction(0, 1));
        assert!(id.starts_with("corr_"));
        assert_eq!(tracker.count(), 1);
    }

    #[test]
    fn test_get_by_id_returns_correction() {
        let mut tracker = CorrectionTracker::new();
        let id = tracker.add(make_bbox_correction(0, 1));

        let correction = tracker.get_by_id(&id);
        assert!(correction.is_some());
        assert!(matches!(
            correction.unwrap(),
            Correction::BBox {
                page: 0,
                element_id: 1,
                ..
            }
        ));
    }

    #[test]
    fn test_get_by_id_not_found() {
        let tracker = CorrectionTracker::new();
        assert!(tracker.get_by_id("nonexistent").is_none());
    }

    #[test]
    fn test_remove_by_id_removes_correction() {
        let mut tracker = CorrectionTracker::new();
        let id = tracker.add(make_bbox_correction(0, 1));
        assert_eq!(tracker.count(), 1);

        let removed = tracker.remove_by_id(&id);
        assert!(removed.is_some());
        assert_eq!(tracker.count(), 0);
        assert!(tracker.get_by_id(&id).is_none());
    }

    #[test]
    fn test_remove_by_id_not_found() {
        let mut tracker = CorrectionTracker::new();
        assert!(tracker.remove_by_id("nonexistent").is_none());
    }

    #[test]
    fn test_ids_returns_all_ids() {
        let mut tracker = CorrectionTracker::new();
        let id1 = tracker.add(make_bbox_correction(0, 1));
        let id2 = tracker.add(make_label_correction(0, 2));

        let ids = tracker.ids();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&id1.as_str()));
        assert!(ids.contains(&id2.as_str()));
    }

    #[test]
    fn test_summary_counts_types() {
        let mut tracker = CorrectionTracker::new();
        let _ = tracker.add(make_bbox_correction(0, 1));
        let _ = tracker.add(make_label_correction(0, 2));
        let _ = tracker.add(Correction::Add {
            page: 1,
            element_id: 3,
            label: "Table".to_string(),
            bbox: json!({"x": 0, "y": 0, "w": 100, "h": 50}),
            text: None,
        });
        let _ = tracker.add(Correction::Delete {
            page: 1,
            element_id: 4,
            original_label: "Text".to_string(),
        });

        let summary = tracker.summary();
        assert_eq!(summary.total, 4);
        assert_eq!(summary.bbox_count, 1);
        assert_eq!(summary.label_count, 1);
        assert_eq!(summary.add_count, 1);
        assert_eq!(summary.delete_count, 1);
    }

    #[test]
    fn test_correction_summary_default() {
        let summary = CorrectionSummary::default();
        assert_eq!(summary.total, 0);
        assert_eq!(summary.bbox_count, 0);
    }

    #[test]
    fn test_correction_partialeq() {
        let c1 = make_bbox_correction(0, 1);
        let c2 = make_bbox_correction(0, 1);
        let c3 = make_bbox_correction(0, 2);

        assert_eq!(c1, c2);
        assert_ne!(c1, c3);
    }

    #[test]
    fn test_correction_display() {
        let bbox = make_bbox_correction(0, 1);
        assert_eq!(format!("{bbox}"), "bbox correction on page 0, element 1");

        let label = make_label_correction(0, 2);
        assert_eq!(
            format!("{label}"),
            "label change on page 0, element 2: Text -> Title"
        );

        let add = Correction::Add {
            page: 1,
            element_id: 3,
            label: "Table".to_string(),
            bbox: json!({"x": 0, "y": 0, "w": 100, "h": 50}),
            text: None,
        };
        assert_eq!(format!("{add}"), "add Table on page 1, element 3");

        let delete = Correction::Delete {
            page: 2,
            element_id: 4,
            original_label: "Image".to_string(),
        };
        assert_eq!(format!("{delete}"), "delete Image on page 2, element 4");
    }
}
