//! Output Validation Framework
//!
//! Validates that plugin outputs are structurally correct and semantically valid.
//! Does NOT require golden outputs - validates against expected schema and value ranges.
//!
//! This addresses the concern: "How do I know that the outputs from the binaries are good?"
//! without the maintenance burden of golden output files.

#![allow(dead_code)]

use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }
}

impl ValidationResult {
    pub fn error(&mut self, msg: String) {
        self.errors.push(msg);
        self.valid = false;
    }

    pub fn warn(&mut self, msg: String) {
        self.warnings.push(msg);
    }

    #[allow(dead_code)]
    pub fn merge(&mut self, other: ValidationResult) {
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
        self.valid = self.valid && other.valid;
    }
}

/// Validate keyframes output
pub fn validate_keyframes(output: &Value) -> ValidationResult {
    let mut result = ValidationResult::default();

    // Parse as array of keyframes
    let keyframes = match output.as_array() {
        Some(arr) => arr,
        None => {
            result.error("Keyframes output must be an array".to_string());
            return result;
        }
    };

    if keyframes.is_empty() {
        result.warn("No keyframes extracted (may be valid for very short videos)".to_string());
        return result;
    }

    for (i, kf) in keyframes.iter().enumerate() {
        let kf_obj = match kf.as_object() {
            Some(obj) => obj,
            None => {
                result.error(format!("Keyframe {} must be an object", i));
                continue;
            }
        };

        // Required fields
        if !kf_obj.contains_key("frame_number") {
            result.error(format!("Keyframe {}: missing frame_number", i));
        }
        if !kf_obj.contains_key("timestamp") {
            result.error(format!("Keyframe {}: missing timestamp", i));
        }
        if !kf_obj.contains_key("hash") {
            result.error(format!("Keyframe {}: missing hash", i));
        }
        if !kf_obj.contains_key("sharpness") {
            result.error(format!("Keyframe {}: missing sharpness", i));
        }
        if !kf_obj.contains_key("thumbnail_paths") {
            result.error(format!("Keyframe {}: missing thumbnail_paths", i));
        }

        // Validate frame_number
        if kf_obj
            .get("frame_number")
            .and_then(|v| v.as_u64())
            .is_none()
        {
            result.error(format!(
                "Keyframe {}: frame_number must be a non-negative integer",
                i
            ));
        }

        // Validate timestamp
        if let Some(timestamp) = kf_obj.get("timestamp").and_then(|v| v.as_f64()) {
            if timestamp < 0.0 {
                result.error(format!("Keyframe {}: timestamp must be non-negative", i));
            }
        } else {
            result.error(format!("Keyframe {}: timestamp must be a number", i));
        }

        // Validate hash (warn if 0, as this is intentional in fast mode but worth noting)
        if let Some(hash) = kf_obj.get("hash").and_then(|v| v.as_u64()) {
            if hash == 0 {
                result.warn(format!(
                    "Keyframe {}: hash is 0 (expected in fast mode, but verify)",
                    i
                ));
            }
        } else {
            result.error(format!(
                "Keyframe {}: hash must be a non-negative integer",
                i
            ));
        }

        // Validate sharpness (warn if 0.0, as this is intentional in fast mode but worth noting)
        if let Some(sharpness) = kf_obj.get("sharpness").and_then(|v| v.as_f64()) {
            if sharpness == 0.0 {
                result.warn(format!(
                    "Keyframe {}: sharpness is 0.0 (expected in fast mode, but verify)",
                    i
                ));
            }
            if sharpness < 0.0 {
                result.error(format!("Keyframe {}: sharpness must be non-negative", i));
            }
        } else {
            result.error(format!("Keyframe {}: sharpness must be a number", i));
        }

        // Validate thumbnail_paths
        if let Some(paths) = kf_obj.get("thumbnail_paths").and_then(|v| v.as_object()) {
            if paths.is_empty() {
                result.warn(format!("Keyframe {}: no thumbnail paths", i));
            }
            for (size, path) in paths {
                if let Some(path_str) = path.as_str() {
                    if path_str.is_empty() {
                        result.error(format!(
                            "Keyframe {}: thumbnail path for {} is empty",
                            i, size
                        ));
                    }
                    // Note: We don't check if files exist on disk, as tests may not save thumbnails
                } else {
                    result.error(format!(
                        "Keyframe {}: thumbnail path for {} must be a string",
                        i, size
                    ));
                }
            }
        } else {
            result.error(format!("Keyframe {}: thumbnail_paths must be an object", i));
        }
    }

    result
}

/// Validate object detection output
pub fn validate_object_detection(output: &Value) -> ValidationResult {
    let mut result = ValidationResult::default();

    // Parse as array directly (not wrapped in object)
    let detections_arr = match output.as_array() {
        Some(arr) => arr,
        None => {
            result.error("Object detection output must be an array".to_string());
            return result;
        }
    };

    if detections_arr.is_empty() {
        result.warn("No objects detected (may be valid for images without objects)".to_string());
        return result;
    }

    for (i, detection) in detections_arr.iter().enumerate() {
        let det_obj = match detection.as_object() {
            Some(obj) => obj,
            None => {
                result.error(format!("Detection {} must be an object", i));
                continue;
            }
        };

        // Validate confidence
        if let Some(confidence) = det_obj.get("confidence").and_then(|v| v.as_f64()) {
            if !(0.0..=1.0).contains(&confidence) {
                result.error(format!(
                    "Detection {}: confidence must be in [0, 1], got {}",
                    i, confidence
                ));
            }
        } else {
            result.error(format!("Detection {}: missing or invalid confidence", i));
        }

        // Validate bbox (allow small floating-point tolerance: -0.01 to 1.01)
        if let Some(bbox) = det_obj.get("bbox").and_then(|v| v.as_object()) {
            for coord in &["x", "y", "width", "height"] {
                if let Some(val) = bbox.get(*coord).and_then(|v| v.as_f64()) {
                    if !(-0.01..=1.01).contains(&val) {
                        result.error(format!(
                            "Detection {}: bbox {} must be in [-0.01, 1.01] (normalized with tolerance), got {}",
                            i, coord, val
                        ));
                    }
                } else {
                    result.error(format!(
                        "Detection {}: bbox missing or invalid {}",
                        i, coord
                    ));
                }
            }
        } else {
            result.error(format!("Detection {}: missing or invalid bbox", i));
        }

        // Validate class_id (COCO has 80 classes, but we'll be lenient)
        if let Some(class_id) = det_obj.get("class_id").and_then(|v| v.as_u64()) {
            if class_id > 1000 {
                result.warn(format!(
                    "Detection {}: class_id {} seems unusually high",
                    i, class_id
                ));
            }
        } else {
            result.error(format!("Detection {}: missing or invalid class_id", i));
        }

        // Validate class_name
        if let Some(class_name) = det_obj.get("class_name").and_then(|v| v.as_str()) {
            if class_name.is_empty() {
                result.error(format!("Detection {}: class_name should not be empty", i));
            }
        } else {
            result.error(format!("Detection {}: missing or invalid class_name", i));
        }
    }

    result
}

/// Validate face detection output
pub fn validate_face_detection(output: &Value) -> ValidationResult {
    let mut result = ValidationResult::default();

    // Parse as array directly (not wrapped in object)
    let faces_arr = match output.as_array() {
        Some(arr) => arr,
        None => {
            result.error("Face detection output must be an array".to_string());
            return result;
        }
    };

    if faces_arr.is_empty() {
        result.warn("No faces detected (may be valid for images without faces)".to_string());
        return result;
    }

    for (i, face) in faces_arr.iter().enumerate() {
        let face_obj = match face.as_object() {
            Some(obj) => obj,
            None => {
                result.error(format!("Face {} must be an object", i));
                continue;
            }
        };

        // Validate confidence
        if let Some(confidence) = face_obj.get("confidence").and_then(|v| v.as_f64()) {
            if !(0.0..=1.0).contains(&confidence) {
                result.error(format!(
                    "Face {}: confidence must be in [0, 1], got {}",
                    i, confidence
                ));
            }
        } else {
            result.error(format!("Face {}: missing or invalid confidence", i));
        }

        // Validate bbox (format: {x1, y1, x2, y2})
        if let Some(bbox) = face_obj.get("bbox").and_then(|v| v.as_object()) {
            for coord in &["x1", "y1", "x2", "y2"] {
                if let Some(val) = bbox.get(*coord).and_then(|v| v.as_f64()) {
                    if !(0.0..=1.0).contains(&val) {
                        result.error(format!(
                            "Face {}: bbox {} must be in [0, 1] (normalized), got {}",
                            i, coord, val
                        ));
                    }
                } else {
                    result.error(format!("Face {}: bbox missing or invalid {}", i, coord));
                }
            }
        } else {
            result.error(format!("Face {}: missing or invalid bbox", i));
        }

        // Validate landmarks (RetinaFace has 5 landmarks, but may be null)
        if let Some(landmarks_value) = face_obj.get("landmarks") {
            if landmarks_value.is_null() {
                // Landmarks can be null (not computed)
                result.warn(format!("Face {}: landmarks are null (not computed)", i));
            } else if let Some(landmarks) = landmarks_value.as_array() {
                if landmarks.len() != 5 {
                    result.warn(format!(
                        "Face {}: expected 5 landmarks (RetinaFace), got {}",
                        i,
                        landmarks.len()
                    ));
                }
                for (j, landmark) in landmarks.iter().enumerate() {
                    if let Some(lm_arr) = landmark.as_array() {
                        if lm_arr.len() != 2 {
                            result.error(format!("Face {}: landmark {} must have [x, y]", i, j));
                        }
                        for (k, coord) in lm_arr.iter().enumerate() {
                            if let Some(val) = coord.as_f64() {
                                if !(0.0..=1.0).contains(&val) {
                                    result.error(format!(
                                        "Face {}: landmark {} coord {} must be in [0, 1], got {}",
                                        i, j, k, val
                                    ));
                                }
                            } else {
                                result.error(format!(
                                    "Face {}: landmark {} coord {} must be a number",
                                    i, j, k
                                ));
                            }
                        }
                    } else {
                        result.error(format!("Face {}: landmark {} must be an array", i, j));
                    }
                }
            } else {
                result.error(format!("Face {}: landmarks must be an array or null", i));
            }
        } else {
            result.error(format!("Face {}: missing landmarks field", i));
        }
    }

    result
}

/// Validate transcription output
pub fn validate_transcription(output: &Value) -> ValidationResult {
    let mut result = ValidationResult::default();

    // Validate text field
    if let Some(text) = output.get("text").and_then(|v| v.as_str()) {
        if text.is_empty() {
            result.warn("Transcription text is empty (may be valid for silent audio)".to_string());
        }
    } else {
        result.error("Transcription output must have 'text' field".to_string());
    }

    // Validate language field
    if let Some(language) = output.get("language").and_then(|v| v.as_str()) {
        if language.len() != 2 {
            result.warn(format!(
                "Language code '{}' is not a 2-letter code",
                language
            ));
        }
    } else {
        result.error("Transcription output must have 'language' field".to_string());
    }

    // Validate segments array
    if let Some(segments) = output.get("segments").and_then(|v| v.as_array()) {
        if segments.is_empty() {
            result.warn("No transcription segments (may be valid for silent audio)".to_string());
        }

        let mut last_end = 0.0;
        for (i, segment) in segments.iter().enumerate() {
            let seg_obj = match segment.as_object() {
                Some(obj) => obj,
                None => {
                    result.error(format!("Segment {} must be an object", i));
                    continue;
                }
            };

            // Validate timestamps
            if let Some(start) = seg_obj.get("start").and_then(|v| v.as_f64()) {
                if start < 0.0 {
                    result.error(format!(
                        "Segment {}: start timestamp must be non-negative",
                        i
                    ));
                }
                if start < last_end {
                    result.warn(format!(
                        "Segment {}: start timestamp {} < previous end {}",
                        i, start, last_end
                    ));
                }
            } else {
                result.error(format!("Segment {}: missing or invalid start timestamp", i));
            }

            if let Some(end) = seg_obj.get("end").and_then(|v| v.as_f64()) {
                if end < 0.0 {
                    result.error(format!("Segment {}: end timestamp must be non-negative", i));
                }
                if let Some(start) = seg_obj.get("start").and_then(|v| v.as_f64()) {
                    if end < start {
                        result.error(format!("Segment {}: end {} < start {}", i, end, start));
                    }
                }
                last_end = end;
            } else {
                result.error(format!("Segment {}: missing or invalid end timestamp", i));
            }

            // Validate text
            if let Some(text) = seg_obj.get("text").and_then(|v| v.as_str()) {
                if text.is_empty() {
                    result.warn(format!("Segment {}: text is empty", i));
                }
            } else {
                result.error(format!("Segment {}: missing or invalid text", i));
            }
        }
    } else {
        result.error("Transcription output must have 'segments' array".to_string());
    }

    result
}

/// Validate OCR output
pub fn validate_ocr(output: &Value) -> ValidationResult {
    let mut result = ValidationResult::default();

    // Support two formats:
    // 1. Flat array: [{text, confidence, bbox}, ...]
    // 2. Nested format: {text_regions: [{text, confidence, bbox}, ...]}
    let regions_arr = if let Some(arr) = output.as_array() {
        // Format 1: Direct array (current plugin output)
        arr
    } else if let Some(arr) = output.get("text_regions").and_then(|v| v.as_array()) {
        // Format 2: Nested object (old format)
        arr
    } else {
        result.error("OCR output must be an array or have 'text_regions' array".to_string());
        return result;
    };

    if regions_arr.is_empty() {
        result.warn("No text regions detected (may be valid for images without text)".to_string());
        return result;
    }

    for (i, region) in regions_arr.iter().enumerate() {
        let region_obj = match region.as_object() {
            Some(obj) => obj,
            None => {
                result.error(format!("Text region {} must be an object", i));
                continue;
            }
        };

        // Validate text
        if let Some(text) = region_obj.get("text").and_then(|v| v.as_str()) {
            if text.is_empty() {
                result.warn(format!("Text region {}: text is empty", i));
            }
        } else {
            result.error(format!("Text region {}: missing or invalid text", i));
        }

        // Validate confidence
        if let Some(confidence) = region_obj.get("confidence").and_then(|v| v.as_f64()) {
            if !(0.0..=1.0).contains(&confidence) {
                result.error(format!(
                    "Text region {}: confidence must be in [0, 1], got {}",
                    i, confidence
                ));
            }
        } else {
            result.error(format!("Text region {}: missing or invalid confidence", i));
        }

        // Validate bbox - support two formats:
        // 1. Rectangle format: {x, y, width, height}
        // 2. Corner points format: {top_left, top_right, bottom_left, bottom_right}
        if let Some(bbox) = region_obj.get("bbox").and_then(|v| v.as_object()) {
            // Check if it's corner points format
            let is_corner_format = bbox.contains_key("top_left")
                || bbox.contains_key("top_right")
                || bbox.contains_key("bottom_left")
                || bbox.contains_key("bottom_right");

            if is_corner_format {
                // Validate corner points format (current OCR output)
                for corner in &["top_left", "top_right", "bottom_left", "bottom_right"] {
                    if let Some(point) = bbox.get(*corner).and_then(|v| v.as_array()) {
                        if point.len() != 2 {
                            result.error(format!(
                                "Text region {}: bbox {} must have 2 coordinates, got {}",
                                i, corner, point.len()
                            ));
                        }
                        for (idx, coord) in point.iter().enumerate() {
                            if let Some(val) = coord.as_f64() {
                                if !(0.0..=1.0).contains(&val) {
                                    result.error(format!(
                                        "Text region {}: bbox {}[{}] must be in [0, 1], got {}",
                                        i, corner, idx, val
                                    ));
                                }
                            }
                        }
                    } else {
                        result.error(format!(
                            "Text region {}: bbox missing or invalid {}",
                            i, corner
                        ));
                    }
                }
            } else {
                // Validate rectangle format (old format)
                for coord in &["x", "y", "width", "height"] {
                    if let Some(val) = bbox.get(*coord).and_then(|v| v.as_f64()) {
                        if !(0.0..=1.0).contains(&val) {
                            result.error(format!(
                                "Text region {}: bbox {} must be in [0, 1] (normalized), got {}",
                                i, coord, val
                            ));
                        }
                    } else {
                        result.error(format!(
                            "Text region {}: bbox missing or invalid {}",
                            i, coord
                        ));
                    }
                }
            }
        } else {
            result.error(format!("Text region {}: missing or invalid bbox", i));
        }
    }

    result
}

/// Validate embedding output (vision, audio, or text embeddings)
pub fn validate_embeddings(output: &Value, expected_dim: Option<usize>) -> ValidationResult {
    let mut result = ValidationResult::default();

    // Support three formats:
    // 1. Video format: {count: N, embeddings: [[vec1], [vec2], ...]}
    // 2. Image/audio format: [[vec1], [vec2], ...] (flat array)
    // 3. Old format: {embedding: [vec]}
    let embedding_arr = if let Some(embeddings) = output.get("embeddings").and_then(|v| v.as_array()) {
        // Format 1: Object with embeddings field
        if embeddings.is_empty() {
            result.warn("No embeddings generated (may be valid for empty input)".to_string());
            return result;
        }
        // Validate first embedding (assume rest have same structure)
        match embeddings[0].as_array() {
            Some(arr) => arr,
            None => {
                result.error("Embeddings array must contain arrays of numbers".to_string());
                return result;
            }
        }
    } else if let Some(flat_arr) = output.as_array() {
        // Format 2: Flat array [[vec1], [vec2], ...]
        if flat_arr.is_empty() {
            result.warn("No embeddings generated (may be valid for empty input)".to_string());
            return result;
        }
        // Validate first embedding
        match flat_arr[0].as_array() {
            Some(arr) => arr,
            None => {
                result.error("Flat embeddings array must contain arrays of numbers".to_string());
                return result;
            }
        }
    } else if let Some(arr) = output.get("embedding").and_then(|v| v.as_array()) {
        // Format 3: Old format with single embedding
        arr
    } else {
        result.error("Embedding output must be [[vec]], {embeddings: [[vec]]}, or {embedding: [vec]}".to_string());
        return result;
    };

    if embedding_arr.is_empty() {
        result.error("Embedding array is empty".to_string());
        return result;
    }

    // Check dimension
    if let Some(expected) = expected_dim {
        if embedding_arr.len() != expected {
            result.error(format!(
                "Embedding dimension mismatch: expected {}, got {}",
                expected,
                embedding_arr.len()
            ));
        }
    }

    // Validate values
    let mut has_nan = false;
    let mut has_inf = false;
    let mut sum_sq = 0.0;

    for (i, val) in embedding_arr.iter().enumerate() {
        if let Some(v) = val.as_f64() {
            if v.is_nan() {
                has_nan = true;
            }
            if v.is_infinite() {
                has_inf = true;
            }
            sum_sq += v * v;
        } else {
            result.error(format!("Embedding value at index {} must be a number", i));
        }
    }

    if has_nan {
        result.error("Embedding contains NaN values".to_string());
    }
    if has_inf {
        result.error("Embedding contains infinite values".to_string());
    }

    // Check L2 norm (many models normalize embeddings to unit length)
    let norm = sum_sq.sqrt();
    if !(0.9..=1.1).contains(&norm) {
        result.warn(format!(
            "Embedding L2 norm is {:.3}, expected ~1.0 for normalized embeddings",
            norm
        ));
    }

    result
}

/// Validate scene detection output
pub fn validate_scene_detection(output: &Value) -> ValidationResult {
    let mut result = ValidationResult::default();

    // Validate boundaries array
    let boundaries = match output.get("boundaries").and_then(|v| v.as_array()) {
        Some(arr) => arr,
        None => {
            result.error("Scene detection output must have 'boundaries' array".to_string());
            return result;
        }
    };

    // Validate scenes array
    let scenes = match output.get("scenes").and_then(|v| v.as_array()) {
        Some(arr) => arr,
        None => {
            result.error("Scene detection output must have 'scenes' array".to_string());
            return result;
        }
    };

    // Validate num_scenes matches scenes array length
    if let Some(num_scenes) = output.get("num_scenes").and_then(|v| v.as_u64()) {
        if num_scenes != scenes.len() as u64 {
            result.error(format!(
                "num_scenes {} does not match scenes array length {}",
                num_scenes,
                scenes.len()
            ));
        }
    } else {
        result.error("Scene detection output must have 'num_scenes' field".to_string());
    }

    // Validate config object
    if let Some(config) = output.get("config").and_then(|v| v.as_object()) {
        // Validate threshold
        if let Some(threshold) = config.get("threshold").and_then(|v| v.as_f64()) {
            if threshold < 0.0 {
                result.error(format!("Config threshold must be non-negative, got {}", threshold));
            }
        } else {
            result.error("Config must have 'threshold' field".to_string());
        }

        // Validate min_scene_duration
        if let Some(duration) = config.get("min_scene_duration").and_then(|v| v.as_f64()) {
            if duration < 0.0 {
                result.error(format!(
                    "Config min_scene_duration must be non-negative, got {}",
                    duration
                ));
            }
        } else {
            result.error("Config must have 'min_scene_duration' field".to_string());
        }

        // Validate keyframes_only
        if config.get("keyframes_only").and_then(|v| v.as_bool()).is_none() {
            result.error("Config must have 'keyframes_only' boolean field".to_string());
        }
    } else {
        result.error("Scene detection output must have 'config' object".to_string());
    }

    // Validate boundaries
    for (i, boundary) in boundaries.iter().enumerate() {
        let boundary_obj = match boundary.as_object() {
            Some(obj) => obj,
            None => {
                result.error(format!("Boundary {} must be an object", i));
                continue;
            }
        };

        // Validate timestamp
        if let Some(timestamp) = boundary_obj.get("timestamp").and_then(|v| v.as_f64()) {
            if timestamp < 0.0 {
                result.error(format!("Boundary {}: timestamp must be non-negative", i));
            }
        } else {
            result.error(format!("Boundary {}: missing or invalid timestamp", i));
        }

        // Validate frame_number
        if boundary_obj.get("frame_number").and_then(|v| v.as_u64()).is_none() {
            result.error(format!("Boundary {}: missing or invalid frame_number", i));
        }

        // Validate score
        if let Some(score) = boundary_obj.get("score").and_then(|v| v.as_f64()) {
            if score < 0.0 {
                result.error(format!("Boundary {}: score must be non-negative", i));
            }
        } else {
            result.error(format!("Boundary {}: missing or invalid score", i));
        }
    }

    // Validate scenes
    let mut last_end_time = 0.0;
    for (i, scene) in scenes.iter().enumerate() {
        let scene_obj = match scene.as_object() {
            Some(obj) => obj,
            None => {
                result.error(format!("Scene {} must be an object", i));
                continue;
            }
        };

        // Validate timestamps
        if let Some(start_time) = scene_obj.get("start_time").and_then(|v| v.as_f64()) {
            if start_time < 0.0 {
                result.error(format!("Scene {}: start_time must be non-negative", i));
            }
            if i > 0 && start_time < last_end_time {
                result.warn(format!(
                    "Scene {}: start_time {} < previous end_time {}",
                    i, start_time, last_end_time
                ));
            }
        } else {
            result.error(format!("Scene {}: missing or invalid start_time", i));
        }

        if let Some(end_time) = scene_obj.get("end_time").and_then(|v| v.as_f64()) {
            if end_time < 0.0 {
                result.error(format!("Scene {}: end_time must be non-negative", i));
            }
            if let Some(start_time) = scene_obj.get("start_time").and_then(|v| v.as_f64()) {
                if end_time < start_time {
                    result.error(format!(
                        "Scene {}: end_time {} < start_time {}",
                        i, end_time, start_time
                    ));
                }
            }
            last_end_time = end_time;
        } else {
            result.error(format!("Scene {}: missing or invalid end_time", i));
        }

        // Validate frame numbers
        if scene_obj.get("start_frame").and_then(|v| v.as_u64()).is_none() {
            result.error(format!("Scene {}: missing or invalid start_frame", i));
        }

        if scene_obj.get("end_frame").and_then(|v| v.as_u64()).is_none() {
            result.error(format!("Scene {}: missing or invalid end_frame", i));
        }

        // Validate frame_count matches frame range
        if let (Some(start_frame), Some(end_frame), Some(frame_count)) = (
            scene_obj.get("start_frame").and_then(|v| v.as_u64()),
            scene_obj.get("end_frame").and_then(|v| v.as_u64()),
            scene_obj.get("frame_count").and_then(|v| v.as_u64()),
        ) {
            let expected_count = end_frame - start_frame;
            if frame_count != expected_count {
                result.error(format!(
                    "Scene {}: frame_count {} does not match end_frame - start_frame = {}",
                    i, frame_count, expected_count
                ));
            }
        } else if scene_obj.get("frame_count").is_none() {
            result.error(format!("Scene {}: missing frame_count", i));
        }

        // Validate score
        if let Some(score) = scene_obj.get("score").and_then(|v| v.as_f64()) {
            if score < 0.0 {
                result.error(format!("Scene {}: score must be non-negative", i));
            }
        } else {
            result.error(format!("Scene {}: missing or invalid score", i));
        }
    }

    result
}

/// Validate metadata extraction output
pub fn validate_metadata_extraction(output: &Value) -> ValidationResult {
    let mut result = ValidationResult::default();

    // Validate format object
    if let Some(format) = output.get("format").and_then(|v| v.as_object()) {
        // Validate format_name
        if format.get("format_name").and_then(|v| v.as_str()).is_none() {
            result.error("Format must have 'format_name' string field".to_string());
        }

        // Validate duration
        if let Some(duration) = format.get("duration").and_then(|v| v.as_f64()) {
            if duration < 0.0 {
                result.error(format!("Format duration must be non-negative, got {}", duration));
            }
        } else {
            result.warn("Format missing 'duration' field (may be valid for images)".to_string());
        }

        // Validate size
        if let Some(size) = format.get("size").and_then(|v| v.as_u64()) {
            if size == 0 {
                result.warn("Format size is 0 (unusual but may be valid)".to_string());
            }
        } else {
            result.error("Format must have 'size' field".to_string());
        }

        // Validate bit_rate (optional, warn if missing for video/audio)
        if format.get("bit_rate").is_none() {
            result.warn("Format missing 'bit_rate' field (expected for video/audio)".to_string());
        }

        // Validate nb_streams
        if let Some(nb_streams) = format.get("nb_streams").and_then(|v| v.as_u64()) {
            if nb_streams == 0 {
                result.warn("Format has 0 streams (unusual)".to_string());
            }
        } else {
            result.error("Format must have 'nb_streams' field".to_string());
        }
    } else {
        result.error("Metadata output must have 'format' object".to_string());
    }

    // Validate video_stream (null or object)
    if let Some(video_stream) = output.get("video_stream") {
        if !video_stream.is_null() {
            if let Some(stream) = video_stream.as_object() {
                // Validate codec_name
                if stream.get("codec_name").and_then(|v| v.as_str()).is_none() {
                    result.error("Video stream must have 'codec_name' field".to_string());
                }

                // Validate dimensions
                if let Some(width) = stream.get("width").and_then(|v| v.as_u64()) {
                    if width == 0 {
                        result.error("Video stream width must be > 0".to_string());
                    }
                } else {
                    result.error("Video stream must have 'width' field".to_string());
                }

                if let Some(height) = stream.get("height").and_then(|v| v.as_u64()) {
                    if height == 0 {
                        result.error("Video stream height must be > 0".to_string());
                    }
                } else {
                    result.error("Video stream must have 'height' field".to_string());
                }

                // Validate fps
                if let Some(fps) = stream.get("fps").and_then(|v| v.as_f64()) {
                    if fps <= 0.0 {
                        result.error(format!("Video stream fps must be > 0, got {}", fps));
                    }
                    if fps > 1000.0 {
                        result.warn(format!("Video stream fps {} seems unusually high", fps));
                    }
                } else {
                    result.error("Video stream must have 'fps' field".to_string());
                }
            } else {
                result.error("Video stream must be an object or null".to_string());
            }
        }
    } else {
        result.error("Metadata output must have 'video_stream' field (null if no video)".to_string());
    }

    // Validate audio_stream (null or object)
    if let Some(audio_stream) = output.get("audio_stream") {
        if !audio_stream.is_null() {
            if let Some(stream) = audio_stream.as_object() {
                // Validate codec_name
                if stream.get("codec_name").and_then(|v| v.as_str()).is_none() {
                    result.error("Audio stream must have 'codec_name' field".to_string());
                }

                // Validate sample_rate
                if let Some(sample_rate) = stream.get("sample_rate").and_then(|v| v.as_u64()) {
                    if sample_rate == 0 {
                        result.error("Audio stream sample_rate must be > 0".to_string());
                    }
                } else {
                    result.error("Audio stream must have 'sample_rate' field".to_string());
                }

                // Validate channels
                if let Some(channels) = stream.get("channels").and_then(|v| v.as_u64()) {
                    if channels == 0 {
                        result.error("Audio stream channels must be > 0".to_string());
                    }
                    if channels > 32 {
                        result.warn(format!("Audio stream channels {} seems unusually high", channels));
                    }
                } else {
                    result.error("Audio stream must have 'channels' field".to_string());
                }
            } else {
                result.error("Audio stream must be an object or null".to_string());
            }
        }
    } else {
        result.error("Metadata output must have 'audio_stream' field (null if no audio)".to_string());
    }

    // Validate config object
    if output.get("config").and_then(|v| v.as_object()).is_none() {
        result.warn("Metadata output missing 'config' object".to_string());
    }

    result
}

/// Validate duplicate-detection output
pub fn validate_duplicate_detection(output: &Value) -> ValidationResult {
    let mut result = ValidationResult::default();

    // Validate top-level fields
    if !output.is_object() {
        result.error("Duplicate detection output must be an object".to_string());
        return result;
    }

    let obj = output.as_object().unwrap();

    // Required fields
    if !obj.contains_key("algorithm") {
        result.error("Missing 'algorithm' field".to_string());
    }
    if !obj.contains_key("hash_size") {
        result.error("Missing 'hash_size' field".to_string());
    }
    if !obj.contains_key("perceptual_hash") {
        result.error("Missing 'perceptual_hash' field".to_string());
    }
    if !obj.contains_key("threshold") {
        result.error("Missing 'threshold' field".to_string());
    }

    // Validate algorithm
    if let Some(algorithm) = obj.get("algorithm").and_then(|v| v.as_str()) {
        if !["Gradient", "Mean", "DoubleGradient", "VertGradient", "Blockhash"].contains(&algorithm) {
            result.warn(format!("Unknown algorithm: {}", algorithm));
        }
    } else {
        result.error("'algorithm' must be a string".to_string());
    }

    // Validate hash_size
    if let Some(hash_size) = obj.get("hash_size").and_then(|v| v.as_u64()) {
        if hash_size == 0 || hash_size > 64 {
            result.warn(format!("Unusual hash_size: {}", hash_size));
        }
    } else {
        result.error("'hash_size' must be a positive integer".to_string());
    }

    // Validate threshold
    if let Some(threshold) = obj.get("threshold").and_then(|v| v.as_f64()) {
        if !(0.0..=1.0).contains(&threshold) {
            result.error(format!("Threshold must be in [0.0, 1.0], got {}", threshold));
        }
    } else {
        result.error("'threshold' must be a number".to_string());
    }

    // Validate perceptual_hash object
    if let Some(hash_obj) = obj.get("perceptual_hash").and_then(|v| v.as_object()) {
        if !hash_obj.contains_key("hash") {
            result.error("'perceptual_hash.hash' field is missing".to_string());
        }
        if !hash_obj.contains_key("media_type") {
            result.error("'perceptual_hash.media_type' field is missing".to_string());
        }

        // Validate hash is base64 string
        if let Some(hash_str) = hash_obj.get("hash").and_then(|v| v.as_str()) {
            if hash_str.is_empty() {
                result.error("'perceptual_hash.hash' cannot be empty".to_string());
            }
        } else {
            result.error("'perceptual_hash.hash' must be a string".to_string());
        }

        // Validate media_type
        if let Some(media_type) = hash_obj.get("media_type").and_then(|v| v.as_str()) {
            if !["Video", "Image", "Audio"].contains(&media_type) {
                result.warn(format!("Unknown media_type: {}", media_type));
            }
        } else {
            result.error("'perceptual_hash.media_type' must be a string".to_string());
        }
    } else {
        result.error("'perceptual_hash' must be an object".to_string());
    }

    result
}

/// Validate smart-thumbnail output
pub fn validate_smart_thumbnail(output: &Value) -> ValidationResult {
    let mut result = ValidationResult::default();

    if !output.is_object() {
        result.error("Smart thumbnail output must be an object".to_string());
        return result;
    }

    let obj = output.as_object().unwrap();

    // Required fields
    if !obj.contains_key("keyframe") {
        result.error("Missing 'keyframe' field".to_string());
    }
    if !obj.contains_key("quality_score") {
        result.error("Missing 'quality_score' field".to_string());
    }
    if !obj.contains_key("scores") {
        result.error("Missing 'scores' field".to_string());
    }

    // Validate quality_score
    if let Some(score) = obj.get("quality_score").and_then(|v| v.as_f64()) {
        if !(0.0..=1.0).contains(&score) {
            result.warn(format!("Quality score outside [0, 1]: {}", score));
        }
    } else {
        result.error("'quality_score' must be a number".to_string());
    }

    // Validate keyframe (should match keyframe structure)
    if let Some(kf) = obj.get("keyframe").and_then(|v| v.as_object()) {
        if !kf.contains_key("frame_number") {
            result.error("keyframe missing 'frame_number'".to_string());
        }
        if !kf.contains_key("timestamp") {
            result.error("keyframe missing 'timestamp'".to_string());
        }
        if !kf.contains_key("thumbnail_paths") {
            result.error("keyframe missing 'thumbnail_paths'".to_string());
        }
    } else {
        result.error("'keyframe' must be an object".to_string());
    }

    // Validate scores object
    if let Some(scores) = obj.get("scores").and_then(|v| v.as_object()) {
        let expected_scores = ["sharpness", "brightness_contrast", "colorfulness", "composition", "face_presence"];
        for score_name in expected_scores {
            if !scores.contains_key(score_name) {
                result.warn(format!("scores missing '{}' field", score_name));
            }
        }

        // Validate face_presence is boolean
        if let Some(face) = scores.get("face_presence") {
            if !face.is_boolean() {
                result.error("scores.face_presence must be a boolean".to_string());
            }
        }

        // Validate numeric scores are in [0, 1] range (except composition which may differ)
        for score_name in &["sharpness", "brightness_contrast", "colorfulness"] {
            if let Some(score_val) = scores.get(*score_name).and_then(|v| v.as_f64()) {
                if !(0.0..=1.0).contains(&score_val) {
                    result.warn(format!("scores.{} outside [0, 1]: {}", score_name, score_val));
                }
            }
        }
    } else {
        result.error("'scores' must be an object".to_string());
    }

    result
}

/// Validate voice-activity-detection output
pub fn validate_voice_activity_detection(output: &Value) -> ValidationResult {
    let mut result = ValidationResult::default();

    if !output.is_object() {
        result.error("Voice activity detection output must be an object".to_string());
        return result;
    }

    let obj = output.as_object().unwrap();

    // Required fields
    if !obj.contains_key("segments") {
        result.error("Missing 'segments' field".to_string());
    }
    if !obj.contains_key("total_duration") {
        result.error("Missing 'total_duration' field".to_string());
    }
    if !obj.contains_key("total_voice_duration") {
        result.error("Missing 'total_voice_duration' field".to_string());
    }
    if !obj.contains_key("voice_percentage") {
        result.error("Missing 'voice_percentage' field".to_string());
    }

    // Validate segments array
    if let Some(segments) = obj.get("segments").and_then(|v| v.as_array()) {
        if segments.is_empty() {
            result.warn("No voice segments detected".to_string());
        }

        for (i, seg) in segments.iter().enumerate() {
            let seg_obj = match seg.as_object() {
                Some(obj) => obj,
                None => {
                    result.error(format!("Segment {} must be an object", i));
                    continue;
                }
            };

            // Required segment fields
            if !seg_obj.contains_key("start") {
                result.error(format!("Segment {}: missing 'start'", i));
            }
            if !seg_obj.contains_key("end") {
                result.error(format!("Segment {}: missing 'end'", i));
            }
            if !seg_obj.contains_key("duration") {
                result.error(format!("Segment {}: missing 'duration'", i));
            }
            if !seg_obj.contains_key("confidence") {
                result.error(format!("Segment {}: missing 'confidence'", i));
            }

            // Validate timestamps
            let start = seg_obj.get("start").and_then(|v| v.as_f64());
            let end = seg_obj.get("end").and_then(|v| v.as_f64());
            let duration = seg_obj.get("duration").and_then(|v| v.as_f64());

            if let (Some(s), Some(e)) = (start, end) {
                if s < 0.0 {
                    result.error(format!("Segment {}: start time cannot be negative", i));
                }
                if e <= s {
                    result.error(format!("Segment {}: end time must be > start time", i));
                }
            }

            if let Some(d) = duration {
                if d <= 0.0 {
                    result.error(format!("Segment {}: duration must be positive", i));
                }
            }

            // Validate confidence
            if let Some(conf) = seg_obj.get("confidence").and_then(|v| v.as_f64()) {
                if !(0.0..=1.0).contains(&conf) {
                    result.error(format!("Segment {}: confidence must be in [0, 1]", i));
                }
            }
        }
    } else {
        result.error("'segments' must be an array".to_string());
    }

    // Validate total_duration
    if let Some(total_dur) = obj.get("total_duration").and_then(|v| v.as_f64()) {
        if total_dur <= 0.0 {
            result.error("'total_duration' must be positive".to_string());
        }
    } else {
        result.error("'total_duration' must be a number".to_string());
    }

    // Validate total_voice_duration
    if let Some(voice_dur) = obj.get("total_voice_duration").and_then(|v| v.as_f64()) {
        if voice_dur < 0.0 {
            result.error("'total_voice_duration' cannot be negative".to_string());
        }

        // Check against total_duration
        if let Some(total_dur) = obj.get("total_duration").and_then(|v| v.as_f64()) {
            if voice_dur > total_dur {
                result.error("'total_voice_duration' cannot exceed 'total_duration'".to_string());
            }
        }
    } else {
        result.error("'total_voice_duration' must be a number".to_string());
    }

    // Validate voice_percentage
    if let Some(percentage) = obj.get("voice_percentage").and_then(|v| v.as_f64()) {
        if !(0.0..=1.0).contains(&percentage) {
            result.error(format!("'voice_percentage' must be in [0, 1], got {}", percentage));
        }
    } else {
        result.error("'voice_percentage' must be a number".to_string());
    }

    result
}

/// Validate emotion-detection output
pub fn validate_emotion_detection(output: &Value) -> ValidationResult {
    let mut result = ValidationResult::default();

    if !output.is_object() {
        result.error("Emotion detection output must be an object".to_string());
        return result;
    }

    let obj = output.as_object().unwrap();

    // Required field
    if !obj.contains_key("emotions") {
        result.error("Missing 'emotions' field".to_string());
        return result;
    }

    // Validate emotions array
    if let Some(emotions) = obj.get("emotions").and_then(|v| v.as_array()) {
        if emotions.is_empty() {
            result.warn("No emotions detected (may be valid for videos without faces)".to_string());
            return result;
        }

        for (i, emo) in emotions.iter().enumerate() {
            let emo_obj = match emo.as_object() {
                Some(obj) => obj,
                None => {
                    result.error(format!("Emotion {} must be an object", i));
                    continue;
                }
            };

            // Required fields
            if !emo_obj.contains_key("timestamp") {
                result.error(format!("Emotion {}: missing 'timestamp'", i));
            }
            if !emo_obj.contains_key("emotion") {
                result.error(format!("Emotion {}: missing 'emotion'", i));
            }
            if !emo_obj.contains_key("confidence") {
                result.error(format!("Emotion {}: missing 'confidence'", i));
            }
            if !emo_obj.contains_key("probabilities") {
                result.error(format!("Emotion {}: missing 'probabilities'", i));
            }

            // Validate timestamp
            if let Some(ts) = emo_obj.get("timestamp").and_then(|v| v.as_f64()) {
                if ts < 0.0 {
                    result.error(format!("Emotion {}: timestamp cannot be negative", i));
                }
            } else {
                result.error(format!("Emotion {}: 'timestamp' must be a number", i));
            }

            // Validate emotion label
            if let Some(emotion) = emo_obj.get("emotion").and_then(|v| v.as_str()) {
                let valid_emotions = ["angry", "disgust", "fear", "happy", "sad", "surprise", "neutral"];
                if !valid_emotions.contains(&emotion) {
                    result.warn(format!("Emotion {}: unknown emotion label '{}'", i, emotion));
                }
            } else {
                result.error(format!("Emotion {}: 'emotion' must be a string", i));
            }

            // Validate confidence
            if let Some(conf) = emo_obj.get("confidence").and_then(|v| v.as_f64()) {
                if !(0.0..=1.0).contains(&conf) {
                    result.error(format!("Emotion {}: confidence must be in [0, 1]", i));
                }
            } else {
                result.error(format!("Emotion {}: 'confidence' must be a number", i));
            }

            // Validate probabilities array
            if let Some(probs) = emo_obj.get("probabilities").and_then(|v| v.as_array()) {
                if probs.len() != 7 {
                    result.warn(format!("Emotion {}: expected 7 probabilities (for 7 emotions), got {}", i, probs.len()));
                }

                let mut prob_sum = 0.0;
                for (j, prob_val) in probs.iter().enumerate() {
                    if let Some(p) = prob_val.as_f64() {
                        if !(0.0..=1.0).contains(&p) {
                            result.error(format!("Emotion {}: probability[{}] must be in [0, 1]", i, j));
                        }
                        prob_sum += p;
                    } else {
                        result.error(format!("Emotion {}: probability[{}] must be a number", i, j));
                    }
                }

                // Check probabilities sum to approximately 1.0
                if (prob_sum - 1.0).abs() > 0.01 {
                    result.warn(format!("Emotion {}: probabilities sum to {}, expected ~1.0", i, prob_sum));
                }
            } else {
                result.error(format!("Emotion {}: 'probabilities' must be an array", i));
            }
        }
    } else {
        result.error("'emotions' must be an array".to_string());
    }

    result
}

/// Validate image-quality-assessment output
pub fn validate_image_quality_assessment(output: &Value) -> ValidationResult {
    let mut result = ValidationResult::default();

    // Helper function to validate a single assessment object
    let validate_assessment = |assess_obj: &serde_json::Map<String, Value>, index: Option<usize>, result: &mut ValidationResult| {
        let idx_str = index.map(|i| format!("Assessment {}", i)).unwrap_or_else(|| "Assessment".to_string());

        // Required fields
        if !assess_obj.contains_key("mean_score") {
            result.error(format!("{}: missing 'mean_score'", idx_str));
        }
        if !assess_obj.contains_key("std_score") {
            result.error(format!("{}: missing 'std_score'", idx_str));
        }

        // Validate mean_score (typically 0-10 scale for NIMA)
        if let Some(mean) = assess_obj.get("mean_score").and_then(|v| v.as_f64()) {
            if !(0.0..=10.0).contains(&mean) {
                result.warn(format!("{}: mean_score outside expected [0, 10] range: {}", idx_str, mean));
            }
        } else {
            result.error(format!("{}: 'mean_score' must be a number", idx_str));
        }

        // Validate std_score (standard deviation, should be >= 0)
        if let Some(std) = assess_obj.get("std_score").and_then(|v| v.as_f64()) {
            if std < 0.0 {
                result.error(format!("{}: std_score cannot be negative", idx_str));
            }
        } else {
            result.error(format!("{}: 'std_score' must be a number", idx_str));
        }
    };

    // Output can be either a single object (for single image) or array (for video with keyframes)
    if let Some(obj) = output.as_object() {
        // Single assessment object
        validate_assessment(obj, None, &mut result);
    } else if let Some(assessments) = output.as_array() {
        // Array of assessments
        if assessments.is_empty() {
            result.warn("No quality assessments returned".to_string());
            return result;
        }

        for (i, assess) in assessments.iter().enumerate() {
            if let Some(assess_obj) = assess.as_object() {
                validate_assessment(assess_obj, Some(i), &mut result);
            } else {
                result.error(format!("Assessment {} must be an object", i));
            }
        }
    } else {
        result.error("Image quality assessment output must be an object or array".to_string());
    }

    result
}

/// Validate subtitle-extraction output
pub fn validate_subtitle_extraction(output: &Value) -> ValidationResult {
    let mut result = ValidationResult::default();

    if !output.is_object() {
        result.error("Subtitle extraction output must be an object".to_string());
        return result;
    }

    let obj = output.as_object().unwrap();

    // Required fields
    if !obj.contains_key("tracks") {
        result.error("Missing 'tracks' field".to_string());
    }
    if !obj.contains_key("total_entries") {
        result.error("Missing 'total_entries' field".to_string());
    }

    // Validate total_entries
    if let Some(total) = obj.get("total_entries").and_then(|v| v.as_u64()) {
        if total == 0 {
            result.warn("No subtitle entries extracted".to_string());
        }
    } else {
        result.error("'total_entries' must be a non-negative integer".to_string());
    }

    // Validate tracks array
    if let Some(tracks) = obj.get("tracks").and_then(|v| v.as_array()) {
        if tracks.is_empty() {
            result.warn("No subtitle tracks found".to_string());
            return result;
        }

        for (i, track) in tracks.iter().enumerate() {
            let track_obj = match track.as_object() {
                Some(obj) => obj,
                None => {
                    result.error(format!("Track {} must be an object", i));
                    continue;
                }
            };

            // Required track fields
            if !track_obj.contains_key("index") {
                result.error(format!("Track {}: missing 'index'", i));
            }
            if !track_obj.contains_key("codec") {
                result.error(format!("Track {}: missing 'codec'", i));
            }
            if !track_obj.contains_key("language") {
                result.error(format!("Track {}: missing 'language'", i));
            }
            if !track_obj.contains_key("is_default") {
                result.error(format!("Track {}: missing 'is_default'", i));
            }
            if !track_obj.contains_key("entries") {
                result.error(format!("Track {}: missing 'entries'", i));
            }

            // Validate index
            if track_obj.get("index").and_then(|v| v.as_u64()).is_none() {
                result.error(format!("Track {}: 'index' must be a non-negative integer", i));
            }

            // Validate is_default is boolean
            if let Some(is_default) = track_obj.get("is_default") {
                if !is_default.is_boolean() {
                    result.error(format!("Track {}: 'is_default' must be a boolean", i));
                }
            }

            // Validate entries array
            if let Some(entries) = track_obj.get("entries").and_then(|v| v.as_array()) {
                for (j, entry) in entries.iter().enumerate() {
                    let entry_obj = match entry.as_object() {
                        Some(obj) => obj,
                        None => {
                            result.error(format!("Track {}, Entry {}: must be an object", i, j));
                            continue;
                        }
                    };

                    // Required entry fields
                    if !entry_obj.contains_key("start_time") {
                        result.error(format!("Track {}, Entry {}: missing 'start_time'", i, j));
                    }
                    if !entry_obj.contains_key("end_time") {
                        result.error(format!("Track {}, Entry {}: missing 'end_time'", i, j));
                    }
                    if !entry_obj.contains_key("text") {
                        result.error(format!("Track {}, Entry {}: missing 'text'", i, j));
                    }
                    if !entry_obj.contains_key("track_index") {
                        result.error(format!("Track {}, Entry {}: missing 'track_index'", i, j));
                    }

                    // Validate timestamps
                    let start = entry_obj.get("start_time").and_then(|v| v.as_f64());
                    let end = entry_obj.get("end_time").and_then(|v| v.as_f64());

                    if let (Some(s), Some(e)) = (start, end) {
                        if s < 0.0 {
                            result.error(format!("Track {}, Entry {}: start_time cannot be negative", i, j));
                        }
                        if e <= s {
                            result.error(format!("Track {}, Entry {}: end_time must be > start_time", i, j));
                        }
                    }

                    // Validate text
                    if let Some(text) = entry_obj.get("text").and_then(|v| v.as_str()) {
                        if text.is_empty() {
                            result.warn(format!("Track {}, Entry {}: empty subtitle text", i, j));
                        }
                    } else {
                        result.error(format!("Track {}, Entry {}: 'text' must be a string", i, j));
                    }
                }
            } else {
                result.error(format!("Track {}: 'entries' must be an array", i));
            }
        }
    } else {
        result.error("'tracks' must be an array".to_string());
    }

    result
}

/// Validate pose-estimation output
pub fn validate_pose_estimation(output: &Value) -> ValidationResult {
    let mut result = ValidationResult::default();

    // Parse as array of pose detections
    let detections = match output.as_array() {
        Some(arr) => arr,
        None => {
            result.error("Pose estimation output must be an array".to_string());
            return result;
        }
    };

    if detections.is_empty() {
        result.warn("No people detected (may be valid for scenes without people)".to_string());
        return result;
    }

    for (i, detection) in detections.iter().enumerate() {
        let det_obj = match detection.as_object() {
            Some(obj) => obj,
            None => {
                result.error(format!("Detection {} must be an object", i));
                continue;
            }
        };

        // Required fields
        if !det_obj.contains_key("bbox") {
            result.error(format!("Detection {}: missing 'bbox'", i));
        }
        if !det_obj.contains_key("confidence") {
            result.error(format!("Detection {}: missing 'confidence'", i));
        }
        if !det_obj.contains_key("keypoints") {
            result.error(format!("Detection {}: missing 'keypoints'", i));
        }

        // Validate confidence
        if let Some(conf) = det_obj.get("confidence").and_then(|v| v.as_f64()) {
            if !(0.0..=1.0).contains(&conf) {
                result.error(format!("Detection {}: confidence must be in [0, 1]", i));
            }
        } else {
            result.error(format!("Detection {}: 'confidence' must be a number", i));
        }

        // Validate bounding box
        if let Some(bbox) = det_obj.get("bbox").and_then(|v| v.as_object()) {
            for field in &["x", "y", "width", "height"] {
                if let Some(val) = bbox.get(*field).and_then(|v| v.as_f64()) {
                    if !(0.0..=1.0).contains(&val) {
                        result.error(format!(
                            "Detection {}: bbox.{} must be normalized [0, 1]",
                            i, field
                        ));
                    }
                } else {
                    result.error(format!("Detection {}: bbox.{} must be a number", i, field));
                }
            }
        }

        // Validate keypoints array (17 keypoints for COCO format, but some models may output fewer)
        if let Some(keypoints) = det_obj.get("keypoints").and_then(|v| v.as_array()) {
            if keypoints.len() != 17 {
                result.warn(format!(
                    "Detection {}: expected 17 keypoints (COCO format), got {} (may be valid for different model formats)",
                    i,
                    keypoints.len()
                ));
            }

            for (j, kp) in keypoints.iter().enumerate() {
                let kp_obj = match kp.as_object() {
                    Some(obj) => obj,
                    None => {
                        result.error(format!("Detection {}, Keypoint {}: must be an object", i, j));
                        continue;
                    }
                };

                // Required keypoint fields
                if !kp_obj.contains_key("name") {
                    result.error(format!("Detection {}, Keypoint {}: missing 'name'", i, j));
                }
                if !kp_obj.contains_key("x") {
                    result.error(format!("Detection {}, Keypoint {}: missing 'x'", i, j));
                }
                if !kp_obj.contains_key("y") {
                    result.error(format!("Detection {}, Keypoint {}: missing 'y'", i, j));
                }
                if !kp_obj.contains_key("confidence") {
                    result.error(format!("Detection {}, Keypoint {}: missing 'confidence'", i, j));
                }

                // Validate keypoint confidence
                if let Some(conf) = kp_obj.get("confidence").and_then(|v| v.as_f64()) {
                    if !(0.0..=1.0).contains(&conf) {
                        result.error(format!(
                            "Detection {}, Keypoint {}: confidence must be in [0, 1]",
                            i, j
                        ));
                    }
                }

                // Validate keypoint coordinates (normalized 0-1)
                for coord in &["x", "y"] {
                    if let Some(val) = kp_obj.get(*coord).and_then(|v| v.as_f64()) {
                        if !(0.0..=1.0).contains(&val) {
                            result.error(format!(
                                "Detection {}, Keypoint {}: {} must be normalized [0, 1]",
                                i, j, coord
                            ));
                        }
                    }
                }
            }
        } else {
            result.error(format!("Detection {}: 'keypoints' must be an array", i));
        }
    }

    result
}

/// Validate action-recognition output
pub fn validate_action_recognition(output: &Value) -> ValidationResult {
    let mut result = ValidationResult::default();

    if !output.is_object() {
        result.error("Action recognition output must be an object".to_string());
        return result;
    }

    let obj = output.as_object().unwrap();

    // Required fields
    if !obj.contains_key("segments") {
        result.error("Missing 'segments' field".to_string());
    }
    if !obj.contains_key("overall_activity") {
        result.error("Missing 'overall_activity' field".to_string());
    }
    if !obj.contains_key("overall_confidence") {
        result.error("Missing 'overall_confidence' field".to_string());
    }
    if !obj.contains_key("total_scene_changes") {
        result.error("Missing 'total_scene_changes' field".to_string());
    }

    // Validate overall_activity (enum values)
    if let Some(activity) = obj.get("overall_activity").and_then(|v| v.as_str()) {
        let valid_activities = [
            "Static",
            "LowMotion",
            "ModerateMotion",
            "HighMotion",
            "RapidCuts",
        ];
        if !valid_activities.contains(&activity) {
            result.error(format!(
                "Invalid overall_activity '{}', must be one of: {:?}",
                activity, valid_activities
            ));
        }
    } else {
        result.error("'overall_activity' must be a string".to_string());
    }

    // Validate overall_confidence
    if let Some(conf) = obj.get("overall_confidence").and_then(|v| v.as_f64()) {
        if !(0.0..=1.0).contains(&conf) {
            result.error("'overall_confidence' must be in [0, 1]".to_string());
        }
    } else {
        result.error("'overall_confidence' must be a number".to_string());
    }

    // Validate total_scene_changes
    if obj
        .get("total_scene_changes")
        .and_then(|v| v.as_u64())
        .is_none()
    {
        result.error("'total_scene_changes' must be a non-negative integer".to_string());
    }

    // Validate segments array
    if let Some(segments) = obj.get("segments").and_then(|v| v.as_array()) {
        if segments.is_empty() {
            result.warn("No action segments detected".to_string());
            return result;
        }

        for (i, segment) in segments.iter().enumerate() {
            let seg_obj = match segment.as_object() {
                Some(obj) => obj,
                None => {
                    result.error(format!("Segment {} must be an object", i));
                    continue;
                }
            };

            // Required segment fields
            for field in &[
                "start_time",
                "end_time",
                "activity",
                "confidence",
                "motion_score",
                "scene_changes",
            ] {
                if !seg_obj.contains_key(*field) {
                    result.error(format!("Segment {}: missing '{}'", i, field));
                }
            }

            // Validate timestamps
            let start = seg_obj.get("start_time").and_then(|v| v.as_f64());
            let end = seg_obj.get("end_time").and_then(|v| v.as_f64());

            if let (Some(s), Some(e)) = (start, end) {
                if s < 0.0 {
                    result.error(format!("Segment {}: start_time cannot be negative", i));
                }
                if e <= s {
                    result.error(format!("Segment {}: end_time must be > start_time", i));
                }
            }

            // Validate confidence
            if let Some(conf) = seg_obj.get("confidence").and_then(|v| v.as_f64()) {
                if !(0.0..=1.0).contains(&conf) {
                    result.error(format!("Segment {}: confidence must be in [0, 1]", i));
                }
            }

            // Validate motion_score
            if let Some(score) = seg_obj.get("motion_score").and_then(|v| v.as_f64()) {
                if !(0.0..=1.0).contains(&score) {
                    result.error(format!("Segment {}: motion_score must be in [0, 1]", i));
                }
            }

            // Validate scene_changes
            if seg_obj
                .get("scene_changes")
                .and_then(|v| v.as_u64())
                .is_none()
            {
                result.error(format!(
                    "Segment {}: 'scene_changes' must be a non-negative integer",
                    i
                ));
            }
        }
    } else {
        result.error("'segments' must be an array".to_string());
    }

    result
}

/// Validate shot-classification output
pub fn validate_shot_classification(output: &Value) -> ValidationResult {
    let mut result = ValidationResult::default();

    if !output.is_object() {
        result.error("Shot classification output must be an object".to_string());
        return result;
    }

    let obj = output.as_object().unwrap();

    // Helper function to validate a single shot object
    let validate_shot = |shot_obj: &serde_json::Map<String, Value>,
                         idx: Option<usize>,
                         result: &mut ValidationResult| {
        let idx_str = idx
            .map(|i| format!("Shot {}", i))
            .unwrap_or_else(|| "Shot".to_string());

        // Required shot fields
        if !shot_obj.contains_key("shot_type") {
            result.error(format!("{}: missing 'shot_type'", idx_str));
        }
        if !shot_obj.contains_key("confidence") {
            result.error(format!("{}: missing 'confidence'", idx_str));
        }
        if !shot_obj.contains_key("metadata") {
            result.error(format!("{}: missing 'metadata'", idx_str));
        }

        // Validate shot_type (enum values)
        if let Some(shot_type) = shot_obj.get("shot_type").and_then(|v| v.as_str()) {
            let valid_types = [
                "extreme_closeup",
                "closeup",
                "medium",
                "medium_long",
                "long",
                "wide",
            ];
            if !valid_types.contains(&shot_type) {
                result.error(format!(
                    "{}: invalid shot_type '{}', must be one of: {:?}",
                    idx_str, shot_type, valid_types
                ));
            }
        } else {
            result.error(format!("{}: 'shot_type' must be a string", idx_str));
        }

        // Validate confidence
        if let Some(conf) = shot_obj.get("confidence").and_then(|v| v.as_f64()) {
            if !(0.0..=1.0).contains(&conf) {
                result.error(format!("{}: confidence must be in [0, 1]", idx_str));
            }
        } else {
            result.error(format!("{}: 'confidence' must be a number", idx_str));
        }

        // Validate metadata
        if let Some(metadata) = shot_obj.get("metadata").and_then(|v| v.as_object()) {
            // Required metadata fields
            for field in &["edge_density", "brightness", "contrast", "dominant_region"] {
                if !metadata.contains_key(*field) {
                    result.error(format!("{}: metadata missing '{}'", idx_str, field));
                }
            }

            // Validate numeric metadata fields
            for field in &["edge_density", "brightness", "contrast"] {
                if let Some(val) = metadata.get(*field).and_then(|v| v.as_f64()) {
                    if !(0.0..=1.0).contains(&val) {
                        result.error(format!(
                            "{}: metadata.{} must be in [0, 1]",
                            idx_str, field
                        ));
                    }
                } else {
                    result.error(format!("{}: metadata.{} must be a number", idx_str, field));
                }
            }

            // Validate dominant_region
            if let Some(region) = metadata.get("dominant_region").and_then(|v| v.as_str()) {
                let valid_regions = ["center", "edges", "top", "bottom", "left", "right"];
                if !valid_regions.contains(&region) {
                    result.warn(format!(
                        "{}: unusual dominant_region '{}', expected one of: {:?}",
                        idx_str, region, valid_regions
                    ));
                }
            }
        } else {
            result.error(format!("{}: 'metadata' must be an object", idx_str));
        }
    };

    // Output can be either:
    // 1. Array wrapper format (video with multiple keyframes): {shots: [...], frame_count: N}
    // 2. Single shot object (single image): {shot_type: "...", confidence: ..., metadata: {...}}
    if obj.contains_key("shots") && obj.contains_key("frame_count") {
        // Format 1: Array wrapper
        // Validate frame_count
        if obj.get("frame_count").and_then(|v| v.as_u64()).is_none() {
            result.error("'frame_count' must be a non-negative integer".to_string());
        }

        // Validate shots array
        if let Some(shots) = obj.get("shots").and_then(|v| v.as_array()) {
            if shots.is_empty() {
                result.warn("No shots classified (may be valid for empty input)".to_string());
                return result;
            }

            for (i, shot) in shots.iter().enumerate() {
                let shot_obj = match shot.as_object() {
                    Some(obj) => obj,
                    None => {
                        result.error(format!("Shot {} must be an object", i));
                        continue;
                    }
                };
                validate_shot(shot_obj, Some(i), &mut result);
            }
        } else {
            result.error("'shots' must be an array".to_string());
        }
    } else if obj.contains_key("shot_type") {
        // Format 2: Single shot object (for single image inputs)
        validate_shot(obj, None, &mut result);
    } else {
        result.error(
            "Shot classification output must have either {shots, frame_count} or {shot_type, confidence, metadata}".to_string()
        );
    }

    result
}

/// Validate content-moderation output
pub fn validate_content_moderation(output: &Value) -> ValidationResult {
    let mut result = ValidationResult::default();

    // Parse as array of moderation results (one per keyframe)
    let results = match output.as_array() {
        Some(arr) => arr,
        None => {
            result.error("Content moderation output must be an array".to_string());
            return result;
        }
    };

    if results.is_empty() {
        result.warn("No content moderation results (may be valid for empty input)".to_string());
        return result;
    }

    for (i, moderation) in results.iter().enumerate() {
        let mod_obj = match moderation.as_object() {
            Some(obj) => obj,
            None => {
                result.error(format!("Moderation result {} must be an object", i));
                continue;
            }
        };

        // Required fields
        if !mod_obj.contains_key("nsfw_score") {
            result.error(format!("Moderation result {}: missing 'nsfw_score'", i));
        }
        if !mod_obj.contains_key("is_safe") {
            result.error(format!("Moderation result {}: missing 'is_safe'", i));
        }

        // Validate nsfw_score
        if let Some(score) = mod_obj.get("nsfw_score").and_then(|v| v.as_f64()) {
            if !(0.0..=1.0).contains(&score) {
                result.error(format!(
                    "Moderation result {}: nsfw_score must be in [0, 1]",
                    i
                ));
            }
        } else {
            result.error(format!(
                "Moderation result {}: 'nsfw_score' must be a number",
                i
            ));
        }

        // Validate is_safe boolean
        if let Some(is_safe) = mod_obj.get("is_safe") {
            if !is_safe.is_boolean() {
                result.error(format!(
                    "Moderation result {}: 'is_safe' must be a boolean",
                    i
                ));
            }
        }

        // Optional: validate category_scores if present
        if let Some(categories) = mod_obj.get("category_scores").and_then(|v| v.as_object()) {
            for (category, score) in categories {
                if let Some(s) = score.as_f64() {
                    if !(0.0..=1.0).contains(&s) {
                        result.error(format!(
                            "Moderation result {}: category_scores.{} must be in [0, 1]",
                            i, category
                        ));
                    }
                } else {
                    result.error(format!(
                        "Moderation result {}: category_scores.{} must be a number",
                        i, category
                    ));
                }
            }
        }
    }

    result
}

/// Validate logo-detection output
pub fn validate_logo_detection(output: &Value) -> ValidationResult {
    let mut result = ValidationResult::default();

    // Parse as array of logo detections
    let detections = match output.as_array() {
        Some(arr) => arr,
        None => {
            result.error("Logo detection output must be an array".to_string());
            return result;
        }
    };

    if detections.is_empty() {
        result.warn("No logos detected (may be valid for content without logos)".to_string());
        return result;
    }

    for (i, detection) in detections.iter().enumerate() {
        let det_obj = match detection.as_object() {
            Some(obj) => obj,
            None => {
                result.error(format!("Detection {} must be an object", i));
                continue;
            }
        };

        // Required fields
        if !det_obj.contains_key("class_id") {
            result.error(format!("Detection {}: missing 'class_id'", i));
        }
        if !det_obj.contains_key("class_name") {
            result.error(format!("Detection {}: missing 'class_name'", i));
        }
        if !det_obj.contains_key("confidence") {
            result.error(format!("Detection {}: missing 'confidence'", i));
        }
        if !det_obj.contains_key("bbox") {
            result.error(format!("Detection {}: missing 'bbox'", i));
        }

        // Validate class_id
        if det_obj.get("class_id").and_then(|v| v.as_u64()).is_none() {
            result.error(format!(
                "Detection {}: 'class_id' must be a non-negative integer",
                i
            ));
        }

        // Validate class_name
        if det_obj.get("class_name").and_then(|v| v.as_str()).is_none() {
            result.error(format!("Detection {}: 'class_name' must be a string", i));
        }

        // Validate confidence
        if let Some(conf) = det_obj.get("confidence").and_then(|v| v.as_f64()) {
            if !(0.0..=1.0).contains(&conf) {
                result.error(format!("Detection {}: confidence must be in [0, 1]", i));
            }
        } else {
            result.error(format!("Detection {}: 'confidence' must be a number", i));
        }

        // Validate bounding box
        if let Some(bbox) = det_obj.get("bbox").and_then(|v| v.as_object()) {
            for field in &["x", "y", "width", "height"] {
                if let Some(val) = bbox.get(*field).and_then(|v| v.as_f64()) {
                    if !(0.0..=1.0).contains(&val) {
                        result.error(format!(
                            "Detection {}: bbox.{} must be normalized [0, 1]",
                            i, field
                        ));
                    }
                } else {
                    result.error(format!("Detection {}: bbox.{} must be a number", i, field));
                }
            }
        } else {
            result.error(format!("Detection {}: 'bbox' must be an object", i));
        }
    }

    result
}

/// Validate depth-estimation output
pub fn validate_depth_estimation(output: &Value) -> ValidationResult {
    let mut result = ValidationResult::default();

    // Parse as array of depth map results (one per keyframe)
    let depth_maps = match output.as_array() {
        Some(arr) => arr,
        None => {
            result.error("Depth estimation output must be an array".to_string());
            return result;
        }
    };

    if depth_maps.is_empty() {
        result.warn("No depth maps generated (may be valid for empty input)".to_string());
        return result;
    }

    for (i, depth_map) in depth_maps.iter().enumerate() {
        let map_obj = match depth_map.as_object() {
            Some(obj) => obj,
            None => {
                result.error(format!("Depth map {} must be an object", i));
                continue;
            }
        };

        // Required fields (depth estimation typically outputs file paths or raw data)
        // Check for common depth estimation output fields
        if !map_obj.contains_key("depth_map_path")
            && !map_obj.contains_key("depth_data")
            && !map_obj.contains_key("min_depth")
        {
            result.error(format!(
                "Depth map {}: expected at least one of: depth_map_path, depth_data, min_depth",
                i
            ));
        }

        // Validate min/max depth if present
        if let Some(min_depth) = map_obj.get("min_depth").and_then(|v| v.as_f64()) {
            if min_depth < 0.0 {
                result.error(format!("Depth map {}: min_depth cannot be negative", i));
            }
        }

        if let Some(max_depth) = map_obj.get("max_depth").and_then(|v| v.as_f64()) {
            if max_depth < 0.0 {
                result.error(format!("Depth map {}: max_depth cannot be negative", i));
            }

            if let Some(min_depth) = map_obj.get("min_depth").and_then(|v| v.as_f64()) {
                if max_depth <= min_depth {
                    result.error(format!(
                        "Depth map {}: max_depth must be > min_depth",
                        i
                    ));
                }
            }
        }

        // Validate frame_number if present
        if map_obj.get("frame_number").and_then(|v| v.as_u64()).is_none()
            && map_obj.contains_key("frame_number")
        {
            result.error(format!(
                "Depth map {}: 'frame_number' must be a non-negative integer",
                i
            ));
        }
    }

    result
}

/// Validate caption-generation output
pub fn validate_caption_generation(output: &Value) -> ValidationResult {
    let mut result = ValidationResult::default();

    // Parse as array of caption results (one per keyframe)
    let captions = match output.as_array() {
        Some(arr) => arr,
        None => {
            result.error("Caption generation output must be an array".to_string());
            return result;
        }
    };

    if captions.is_empty() {
        result.warn("No captions generated (may be valid for empty input)".to_string());
        return result;
    }

    for (i, caption) in captions.iter().enumerate() {
        let cap_obj = match caption.as_object() {
            Some(obj) => obj,
            None => {
                result.error(format!("Caption {} must be an object", i));
                continue;
            }
        };

        // Required fields
        if !cap_obj.contains_key("text") {
            result.error(format!("Caption {}: missing 'text'", i));
        }

        // Validate text
        if let Some(text) = cap_obj.get("text").and_then(|v| v.as_str()) {
            if text.is_empty() {
                result.warn(format!("Caption {}: empty caption text", i));
            }
            if text.len() > 500 {
                result.warn(format!(
                    "Caption {}: unusually long caption ({} characters)",
                    i,
                    text.len()
                ));
            }
        } else {
            result.error(format!("Caption {}: 'text' must be a string", i));
        }

        // Validate optional confidence field
        if let Some(conf) = cap_obj.get("confidence") {
            if let Some(c) = conf.as_f64() {
                if !(0.0..=1.0).contains(&c) {
                    result.error(format!("Caption {}: confidence must be in [0, 1]", i));
                }
            } else {
                result.error(format!("Caption {}: 'confidence' must be a number", i));
            }
        }

        // Validate optional frame_number field
        if cap_obj.get("frame_number").and_then(|v| v.as_u64()).is_none()
            && cap_obj.contains_key("frame_number")
        {
            result.error(format!(
                "Caption {}: 'frame_number' must be a non-negative integer",
                i
            ));
        }
    }

    result
}

/// Validate audio-classification output
pub fn validate_audio_classification(output: &Value) -> ValidationResult {
    let mut result = ValidationResult::default();

    // Parse as array of segment classifications
    let segments = match output.as_array() {
        Some(arr) => arr,
        None => {
            result.error("Audio classification output must be an array".to_string());
            return result;
        }
    };

    if segments.is_empty() {
        result.warn(
            "No audio classifications (may be valid for silent or very short audio)".to_string(),
        );
        return result;
    }

    for (i, segment) in segments.iter().enumerate() {
        let seg_obj = match segment.as_object() {
            Some(obj) => obj,
            None => {
                result.error(format!("Segment {} must be an object", i));
                continue;
            }
        };

        // Required fields
        if !seg_obj.contains_key("start_time") {
            result.error(format!("Segment {}: missing 'start_time'", i));
        }
        if !seg_obj.contains_key("end_time") {
            result.error(format!("Segment {}: missing 'end_time'", i));
        }
        if !seg_obj.contains_key("results") {
            result.error(format!("Segment {}: missing 'results'", i));
        }

        // Validate timestamps
        let start = seg_obj.get("start_time").and_then(|v| v.as_f64());
        let end = seg_obj.get("end_time").and_then(|v| v.as_f64());

        if let (Some(s), Some(e)) = (start, end) {
            if s < 0.0 {
                result.error(format!("Segment {}: start_time cannot be negative", i));
            }
            if e <= s {
                result.error(format!("Segment {}: end_time must be > start_time", i));
            }
        }

        // Validate results array (top-k classifications)
        if let Some(results) = seg_obj.get("results").and_then(|v| v.as_array()) {
            if results.is_empty() {
                result.warn(format!(
                    "Segment {}: no classification results (may indicate low confidence)",
                    i
                ));
            }

            for (j, class_result) in results.iter().enumerate() {
                let class_obj = match class_result.as_object() {
                    Some(obj) => obj,
                    None => {
                        result.error(format!("Segment {}, Result {}: must be an object", i, j));
                        continue;
                    }
                };

                // Required classification fields
                if !class_obj.contains_key("class_id") {
                    result.error(format!("Segment {}, Result {}: missing 'class_id'", i, j));
                }
                if !class_obj.contains_key("class_name") {
                    result.error(format!("Segment {}, Result {}: missing 'class_name'", i, j));
                }
                if !class_obj.contains_key("confidence") {
                    result.error(format!("Segment {}, Result {}: missing 'confidence'", i, j));
                }

                // Validate class_id (YAMNet has 521 classes: 0-520)
                if let Some(class_id) = class_obj.get("class_id").and_then(|v| v.as_u64()) {
                    if class_id > 520 {
                        result.warn(format!(
                            "Segment {}, Result {}: class_id {} exceeds expected YAMNet range (0-520)",
                            i, j, class_id
                        ));
                    }
                } else {
                    result.error(format!(
                        "Segment {}, Result {}: 'class_id' must be a non-negative integer",
                        i, j
                    ));
                }

                // Validate class_name
                if class_obj
                    .get("class_name")
                    .and_then(|v| v.as_str())
                    .is_none()
                {
                    result.error(format!(
                        "Segment {}, Result {}: 'class_name' must be a string",
                        i, j
                    ));
                }

                // Validate confidence
                if let Some(conf) = class_obj.get("confidence").and_then(|v| v.as_f64()) {
                    if !(0.0..=1.0).contains(&conf) {
                        result.error(format!(
                            "Segment {}, Result {}: confidence must be in [0, 1]",
                            i, j
                        ));
                    }
                } else {
                    result.error(format!(
                        "Segment {}, Result {}: 'confidence' must be a number",
                        i, j
                    ));
                }
            }
        } else {
            result.error(format!("Segment {}: 'results' must be an array", i));
        }
    }

    result
}

/// Validate diarization (speaker diarization) output
pub fn validate_diarization(output: &Value) -> ValidationResult {
    let mut result = ValidationResult::default();

    // Parse as object with speakers and timeline
    let obj = match output.as_object() {
        Some(o) => o,
        None => {
            result.error("Diarization output must be an object".to_string());
            return result;
        }
    };

    // Required fields
    if !obj.contains_key("speakers") {
        result.error("Diarization output missing 'speakers' field".to_string());
    }
    if !obj.contains_key("timeline") {
        result.error("Diarization output missing 'timeline' field".to_string());
    }

    // Validate speakers array
    if let Some(speakers) = obj.get("speakers").and_then(|v| v.as_array()) {
        if speakers.is_empty() {
            result.warn("No speakers detected (may be valid for silent audio)".to_string());
        }

        for (i, speaker) in speakers.iter().enumerate() {
            let spk_obj = match speaker.as_object() {
                Some(o) => o,
                None => {
                    result.error(format!("Speaker {} must be an object", i));
                    continue;
                }
            };

            // Required speaker fields
            if !spk_obj.contains_key("id") {
                result.error(format!("Speaker {}: missing 'id'", i));
            }
            if !spk_obj.contains_key("total_speaking_time") {
                result.error(format!("Speaker {}: missing 'total_speaking_time'", i));
            }

            // Validate speaker ID format (e.g., "SPEAKER_00")
            if let Some(id) = spk_obj.get("id").and_then(|v| v.as_str()) {
                if !id.starts_with("SPEAKER_") {
                    result.warn(format!(
                        "Speaker {}: ID '{}' doesn't follow expected 'SPEAKER_XX' format",
                        i, id
                    ));
                }
            } else {
                result.error(format!("Speaker {}: 'id' must be a string", i));
            }

            // Validate total_speaking_time
            if let Some(time) = spk_obj.get("total_speaking_time").and_then(|v| v.as_f64()) {
                if time < 0.0 {
                    result.error(format!(
                        "Speaker {}: total_speaking_time cannot be negative",
                        i
                    ));
                }
            } else {
                result.error(format!(
                    "Speaker {}: 'total_speaking_time' must be a number",
                    i
                ));
            }
        }
    } else {
        result.error("'speakers' must be an array".to_string());
    }

    // Validate timeline array
    if let Some(timeline) = obj.get("timeline").and_then(|v| v.as_array()) {
        if timeline.is_empty() && obj.get("speakers").and_then(|v| v.as_array()).is_some_and(|s| !s.is_empty()) {
            result.warn("Timeline is empty but speakers detected (unexpected)".to_string());
        }

        for (i, segment) in timeline.iter().enumerate() {
            let seg_obj = match segment.as_object() {
                Some(o) => o,
                None => {
                    result.error(format!("Timeline segment {} must be an object", i));
                    continue;
                }
            };

            // Required segment fields
            if !seg_obj.contains_key("start") {
                result.error(format!("Segment {}: missing 'start'", i));
            }
            if !seg_obj.contains_key("end") {
                result.error(format!("Segment {}: missing 'end'", i));
            }
            if !seg_obj.contains_key("speaker") {
                result.error(format!("Segment {}: missing 'speaker'", i));
            }
            if !seg_obj.contains_key("confidence") {
                result.error(format!("Segment {}: missing 'confidence'", i));
            }

            // Validate timestamps
            let start = seg_obj.get("start").and_then(|v| v.as_f64());
            let end = seg_obj.get("end").and_then(|v| v.as_f64());

            if let (Some(s), Some(e)) = (start, end) {
                if s < 0.0 {
                    result.error(format!("Segment {}: start cannot be negative", i));
                }
                if e <= s {
                    result.error(format!("Segment {}: end must be > start", i));
                }
            }

            // Validate speaker ID
            if seg_obj.get("speaker").and_then(|v| v.as_str()).is_none() {
                result.error(format!("Segment {}: 'speaker' must be a string", i));
            }

            // Validate confidence
            if let Some(conf) = seg_obj.get("confidence").and_then(|v| v.as_f64()) {
                if !(0.0..=1.0).contains(&conf) {
                    result.error(format!("Segment {}: confidence must be in [0, 1]", i));
                }
            } else {
                result.error(format!("Segment {}: 'confidence' must be a number", i));
            }
        }

        // Check timeline is sorted by start time
        let mut prev_start: Option<f64> = None;
        for (i, segment) in timeline.iter().enumerate() {
            if let Some(start) = segment.get("start").and_then(|v| v.as_f64()) {
                if let Some(prev) = prev_start {
                    if start < prev {
                        result.warn(format!(
                            "Timeline segment {} not sorted by start time (prev: {}, current: {})",
                            i, prev, start
                        ));
                    }
                }
                prev_start = Some(start);
            }
        }
    } else {
        result.error("'timeline' must be an array".to_string());
    }

    result
}

/// Validate acoustic_scene_classification output
pub fn validate_acoustic_scene_classification(output: &Value) -> ValidationResult {
    let mut result = ValidationResult::default();

    // Parse as array of scene detections
    let scenes = match output.as_array() {
        Some(arr) => arr,
        None => {
            result.error("Acoustic scene classification output must be an array".to_string());
            return result;
        }
    };

    if scenes.is_empty() {
        result.warn("No scenes detected (may be valid for silent or very short audio)".to_string());
        return result;
    }

    // Valid scene types from YAMNet (IDs 500-504)
    let valid_scene_types = [
        "InsideSmallRoom",
        "InsideLargeRoom",
        "InsidePublicSpace",
        "OutsideUrban",
        "OutsideRural",
    ];

    for (i, scene) in scenes.iter().enumerate() {
        let scene_obj = match scene.as_object() {
            Some(obj) => obj,
            None => {
                result.error(format!("Scene {} must be an object", i));
                continue;
            }
        };

        // Required fields
        if !scene_obj.contains_key("start_time") {
            result.error(format!("Scene {}: missing 'start_time'", i));
        }
        if !scene_obj.contains_key("end_time") {
            result.error(format!("Scene {}: missing 'end_time'", i));
        }
        if !scene_obj.contains_key("scene_type") {
            result.error(format!("Scene {}: missing 'scene_type'", i));
        }
        if !scene_obj.contains_key("scene_name") {
            result.error(format!("Scene {}: missing 'scene_name'", i));
        }
        if !scene_obj.contains_key("confidence") {
            result.error(format!("Scene {}: missing 'confidence'", i));
        }

        // Validate timestamps
        let start = scene_obj.get("start_time").and_then(|v| v.as_f64());
        let end = scene_obj.get("end_time").and_then(|v| v.as_f64());

        if let (Some(s), Some(e)) = (start, end) {
            if s < 0.0 {
                result.error(format!("Scene {}: start_time cannot be negative", i));
            }
            if e <= s {
                result.error(format!("Scene {}: end_time must be > start_time", i));
            }
        }

        // Validate scene_type (enum)
        if let Some(scene_type) = scene_obj.get("scene_type").and_then(|v| v.as_str()) {
            if !valid_scene_types.contains(&scene_type) {
                result.error(format!(
                    "Scene {}: invalid scene_type '{}' (expected one of: {:?})",
                    i, scene_type, valid_scene_types
                ));
            }
        } else {
            result.error(format!("Scene {}: 'scene_type' must be a string", i));
        }

        // Validate scene_name
        if scene_obj.get("scene_name").and_then(|v| v.as_str()).is_none() {
            result.error(format!("Scene {}: 'scene_name' must be a string", i));
        }

        // Validate confidence
        if let Some(conf) = scene_obj.get("confidence").and_then(|v| v.as_f64()) {
            if !(0.0..=1.0).contains(&conf) {
                result.error(format!("Scene {}: confidence must be in [0, 1]", i));
            }
        } else {
            result.error(format!("Scene {}: 'confidence' must be a number", i));
        }
    }

    result
}

/// Validate profanity_detection output
pub fn validate_profanity_detection(output: &Value) -> ValidationResult {
    let mut result = ValidationResult::default();

    // Parse as object with results
    let obj = match output.as_object() {
        Some(o) => o,
        None => {
            result.error("Profanity detection output must be an object".to_string());
            return result;
        }
    };

    // Required fields
    if !obj.contains_key("total_matches") {
        result.error("Profanity detection output missing 'total_matches'".to_string());
    }
    if !obj.contains_key("matches") {
        result.error("Profanity detection output missing 'matches'".to_string());
    }
    if !obj.contains_key("profanity_rate") {
        result.error("Profanity detection output missing 'profanity_rate'".to_string());
    }

    // Validate total_matches
    if let Some(total) = obj.get("total_matches").and_then(|v| v.as_u64()) {
        if total == 0 {
            // No profanity detected is valid
        }
    } else {
        result.error("'total_matches' must be a non-negative integer".to_string());
    }

    // Validate matches array
    if let Some(matches) = obj.get("matches").and_then(|v| v.as_array()) {
        let valid_severities = ["mild", "moderate", "strong", "severe"];

        for (i, match_item) in matches.iter().enumerate() {
            let match_obj = match match_item.as_object() {
                Some(o) => o,
                None => {
                    result.error(format!("Match {} must be an object", i));
                    continue;
                }
            };

            // Required match fields
            if !match_obj.contains_key("word") {
                result.error(format!("Match {}: missing 'word'", i));
            }
            if !match_obj.contains_key("severity") {
                result.error(format!("Match {}: missing 'severity'", i));
            }
            if !match_obj.contains_key("context") {
                result.error(format!("Match {}: missing 'context'", i));
            }

            // Validate word
            if match_obj.get("word").and_then(|v| v.as_str()).is_none() {
                result.error(format!("Match {}: 'word' must be a string", i));
            }

            // Validate severity (enum: mild, moderate, strong, severe)
            if let Some(severity) = match_obj.get("severity").and_then(|v| v.as_str()) {
                if !valid_severities.contains(&severity) {
                    result.error(format!(
                        "Match {}: invalid severity '{}' (expected: {:?})",
                        i, severity, valid_severities
                    ));
                }
            } else {
                result.error(format!("Match {}: 'severity' must be a string", i));
            }

            // Validate optional start/end times (for transcription)
            if let Some(start) = match_obj.get("start").and_then(|v| v.as_f64()) {
                if start < 0.0 {
                    result.error(format!("Match {}: 'start' cannot be negative", i));
                }
            }
            if let Some(end) = match_obj.get("end").and_then(|v| v.as_f64()) {
                if end < 0.0 {
                    result.error(format!("Match {}: 'end' cannot be negative", i));
                }
                if let Some(start) = match_obj.get("start").and_then(|v| v.as_f64()) {
                    if end <= start {
                        result.error(format!("Match {}: 'end' must be > 'start'", i));
                    }
                }
            }

            // Validate context
            if match_obj.get("context").and_then(|v| v.as_str()).is_none() {
                result.error(format!("Match {}: 'context' must be a string", i));
            }
        }
    } else {
        result.error("'matches' must be an array".to_string());
    }

    // Validate profanity_rate
    if let Some(rate) = obj.get("profanity_rate").and_then(|v| v.as_f64()) {
        if rate < 0.0 {
            result.error("'profanity_rate' cannot be negative".to_string());
        }
    } else {
        result.error("'profanity_rate' must be a number".to_string());
    }

    // Validate optional max_severity
    if let Some(max_sev) = obj.get("max_severity") {
        if !max_sev.is_null() {
            if let Some(sev_str) = max_sev.as_str() {
                let valid_severities = ["mild", "moderate", "strong", "severe"];
                if !valid_severities.contains(&sev_str) {
                    result.error(format!(
                        "Invalid max_severity '{}' (expected: {:?})",
                        sev_str, valid_severities
                    ));
                }
            } else {
                result.error("'max_severity' must be a string or null".to_string());
            }
        }
    }

    result
}

/// Validate motion-tracking output
pub fn validate_motion_tracking(output: &Value) -> ValidationResult {
    let mut result = ValidationResult::default();

    // Parse as array of tracks
    let tracks = match output.as_array() {
        Some(arr) => arr,
        None => {
            result.error("Motion tracking output must be an array".to_string());
            return result;
        }
    };

    if tracks.is_empty() {
        result.warn("No tracks detected (may be valid for static scenes)".to_string());
        return result;
    }

    for (i, track) in tracks.iter().enumerate() {
        let track_obj = match track.as_object() {
            Some(obj) => obj,
            None => {
                result.error(format!("Track {} must be an object", i));
                continue;
            }
        };

        // Required fields
        if !track_obj.contains_key("id") {
            result.error(format!("Track {}: missing 'id'", i));
        }
        if !track_obj.contains_key("class_id") {
            result.error(format!("Track {}: missing 'class_id'", i));
        }
        if !track_obj.contains_key("class_name") {
            result.error(format!("Track {}: missing 'class_name'", i));
        }
        if !track_obj.contains_key("detections") {
            result.error(format!("Track {}: missing 'detections'", i));
        }
        if !track_obj.contains_key("start_frame") {
            result.error(format!("Track {}: missing 'start_frame'", i));
        }
        if !track_obj.contains_key("end_frame") {
            result.error(format!("Track {}: missing 'end_frame'", i));
        }
        if !track_obj.contains_key("hits") {
            result.error(format!("Track {}: missing 'hits'", i));
        }
        if !track_obj.contains_key("age") {
            result.error(format!("Track {}: missing 'age'", i));
        }

        // Validate track ID
        if track_obj.get("id").and_then(|v| v.as_u64()).is_none() {
            result.error(format!("Track {}: 'id' must be a non-negative integer", i));
        }

        // Validate class_id
        if track_obj.get("class_id").and_then(|v| v.as_u64()).is_none() {
            result.error(format!("Track {}: 'class_id' must be a non-negative integer", i));
        }

        // Validate class_name
        if track_obj.get("class_name").and_then(|v| v.as_str()).is_none() {
            result.error(format!("Track {}: 'class_name' must be a string", i));
        }

        // Validate frame indices
        let start_frame = track_obj.get("start_frame").and_then(|v| v.as_u64());
        let end_frame = track_obj.get("end_frame").and_then(|v| v.as_u64());

        if let (Some(start), Some(end)) = (start_frame, end_frame) {
            if end < start {
                result.error(format!("Track {}: end_frame must be >= start_frame", i));
            }
        }

        // Validate detections array
        if let Some(detections) = track_obj.get("detections").and_then(|v| v.as_array()) {
            if detections.is_empty() {
                result.error(format!("Track {}: 'detections' array cannot be empty", i));
            }

            for (j, detection) in detections.iter().enumerate() {
                let det_obj = match detection.as_object() {
                    Some(o) => o,
                    None => {
                        result.error(format!("Track {}, Detection {}: must be an object", i, j));
                        continue;
                    }
                };

                // Required detection fields
                if !det_obj.contains_key("bbox") {
                    result.error(format!("Track {}, Detection {}: missing 'bbox'", i, j));
                }
                if !det_obj.contains_key("confidence") {
                    result.error(format!("Track {}, Detection {}: missing 'confidence'", i, j));
                }
                if !det_obj.contains_key("frame_idx") {
                    result.error(format!("Track {}, Detection {}: missing 'frame_idx'", i, j));
                }

                // Validate bbox
                if let Some(bbox) = det_obj.get("bbox").and_then(|v| v.as_object()) {
                    if !bbox.contains_key("x") || !bbox.contains_key("y")
                        || !bbox.contains_key("width") || !bbox.contains_key("height")
                    {
                        result.error(format!(
                            "Track {}, Detection {}: bbox missing required fields (x, y, width, height)",
                            i, j
                        ));
                    }

                    // Validate bbox values are in [0, 1] range (normalized coordinates)
                    for field in &["x", "y", "width", "height"] {
                        if let Some(val) = bbox.get(*field).and_then(|v| v.as_f64()) {
                            if !(0.0..=1.0).contains(&val) {
                                result.warn(format!(
                                    "Track {}, Detection {}: bbox.{} = {} outside [0, 1] (may be absolute coordinates)",
                                    i, j, field, val
                                ));
                            }
                        }
                    }
                }

                // Validate confidence
                if let Some(conf) = det_obj.get("confidence").and_then(|v| v.as_f64()) {
                    if !(0.0..=1.0).contains(&conf) {
                        result.error(format!(
                            "Track {}, Detection {}: confidence must be in [0, 1]",
                            i, j
                        ));
                    }
                }
            }
        } else {
            result.error(format!("Track {}: 'detections' must be an array", i));
        }

        // Validate hits and age
        if track_obj.get("hits").and_then(|v| v.as_u64()).is_none() {
            result.error(format!("Track {}: 'hits' must be a non-negative integer", i));
        }
        if track_obj.get("age").and_then(|v| v.as_u64()).is_none() {
            result.error(format!("Track {}: 'age' must be a non-negative integer", i));
        }
    }

    result
}

/// Validate audio_enhancement_metadata output
pub fn validate_audio_enhancement_metadata(output: &Value) -> ValidationResult {
    let mut result = ValidationResult::default();

    // Parse as object with metadata fields
    let obj = match output.as_object() {
        Some(o) => o,
        None => {
            result.error("Audio enhancement metadata output must be an object".to_string());
            return result;
        }
    };

    // Required fields
    let required_fields = [
        "snr_db",
        "dynamic_range_db",
        "rms_level",
        "peak_level",
        "spectral_centroid_hz",
        "spectral_rolloff_hz",
        "bandwidth_hz",
        "has_clipping",
        "recommendations",
    ];

    for field in &required_fields {
        if !obj.contains_key(*field) {
            result.error(format!("Audio enhancement metadata missing '{}'", field));
        }
    }

    // Validate numeric fields
    if let Some(snr_db) = obj.get("snr_db").and_then(|v| v.as_f64()) {
        // SNR can be negative for very noisy audio
        if !(-50.0..=100.0).contains(&snr_db) {
            result.warn(format!(
                "snr_db = {} dB is outside typical range [-50, 100]",
                snr_db
            ));
        }
    } else {
        result.error("'snr_db' must be a number".to_string());
    }

    if let Some(dr_db) = obj.get("dynamic_range_db").and_then(|v| v.as_f64()) {
        if dr_db < 0.0 {
            result.error("'dynamic_range_db' cannot be negative".to_string());
        }
        if dr_db > 120.0 {
            result.warn(format!(
                "dynamic_range_db = {} dB exceeds typical maximum (120 dB)",
                dr_db
            ));
        }
    } else {
        result.error("'dynamic_range_db' must be a number".to_string());
    }

    if let Some(rms) = obj.get("rms_level").and_then(|v| v.as_f64()) {
        if !(0.0..=1.0).contains(&rms) {
            result.error(format!(
                "'rms_level' must be in [0, 1], got {}",
                rms
            ));
        }
    } else {
        result.error("'rms_level' must be a number".to_string());
    }

    if let Some(peak) = obj.get("peak_level").and_then(|v| v.as_f64()) {
        if !(0.0..=1.0).contains(&peak) {
            result.error(format!(
                "'peak_level' must be in [0, 1], got {}",
                peak
            ));
        }
    } else {
        result.error("'peak_level' must be a number".to_string());
    }

    // Validate spectral fields (Hz values)
    for field in &["spectral_centroid_hz", "spectral_rolloff_hz", "bandwidth_hz"] {
        if let Some(hz) = obj.get(*field).and_then(|v| v.as_f64()) {
            if hz < 0.0 {
                result.error(format!("'{}' cannot be negative", field));
            }
            if hz > 48000.0 {
                result.warn(format!(
                    "'{}' = {} Hz exceeds typical Nyquist limit (48000 Hz)",
                    field, hz
                ));
            }
        } else {
            result.error(format!("'{}' must be a number", field));
        }
    }

    // Validate has_clipping (boolean)
    if obj.get("has_clipping").and_then(|v| v.as_bool()).is_none() {
        result.error("'has_clipping' must be a boolean".to_string());
    }

    // Validate recommendations array
    if let Some(recs) = obj.get("recommendations").and_then(|v| v.as_array()) {
        let valid_recommendations = [
            "Denoise",
            "Normalize",
            "Equalize",
            "RemoveClipping",
            "AmplifyVolume",
            "None",
        ];

        for (i, rec) in recs.iter().enumerate() {
            if let Some(rec_str) = rec.as_str() {
                if !valid_recommendations.contains(&rec_str) {
                    result.error(format!(
                        "Recommendation {}: invalid value '{}' (expected: {:?})",
                        i, rec_str, valid_recommendations
                    ));
                }
            } else {
                result.error(format!("Recommendation {}: must be a string", i));
            }
        }
    } else {
        result.error("'recommendations' must be an array".to_string());
    }

    result
}

/// Validate music_source_separation output
pub fn validate_music_source_separation(output: &Value) -> ValidationResult {
    let mut result = ValidationResult::default();

    // Parse as array of separated stems
    let stems = match output.as_array() {
        Some(arr) => arr,
        None => {
            result.error("Music source separation output must be an array".to_string());
            return result;
        }
    };

    if stems.is_empty() {
        result.error("No stems separated (expected at least one stem)".to_string());
        return result;
    }

    // Common stem names (Demucs 4-stem model)
    let common_stems = ["vocals", "drums", "bass", "other"];

    for (i, stem) in stems.iter().enumerate() {
        let stem_obj = match stem.as_object() {
            Some(obj) => obj,
            None => {
                result.error(format!("Stem {} must be an object", i));
                continue;
            }
        };

        // Required fields
        if !stem_obj.contains_key("stem_name") {
            result.error(format!("Stem {}: missing 'stem_name'", i));
        }
        if !stem_obj.contains_key("audio") {
            result.error(format!("Stem {}: missing 'audio'", i));
        }
        if !stem_obj.contains_key("channels") {
            result.error(format!("Stem {}: missing 'channels'", i));
        }

        // Validate stem_name
        if let Some(stem_name) = stem_obj.get("stem_name").and_then(|v| v.as_str()) {
            if !common_stems.contains(&stem_name) {
                result.warn(format!(
                    "Stem {}: uncommon stem_name '{}' (typical: {:?})",
                    i, stem_name, common_stems
                ));
            }
        } else {
            result.error(format!("Stem {}: 'stem_name' must be a string", i));
        }

        // Validate audio array
        if let Some(audio) = stem_obj.get("audio").and_then(|v| v.as_array()) {
            if audio.is_empty() {
                result.warn(format!("Stem {}: audio array is empty (silent stem?)", i));
            }

            // Validate audio samples are numbers
            for (j, sample) in audio.iter().take(10).enumerate() {
                // Check first 10 samples
                if sample.as_f64().is_none() {
                    result.error(format!(
                        "Stem {}: audio sample {} must be a number",
                        i, j
                    ));
                    break;
                }
            }
        } else {
            result.error(format!("Stem {}: 'audio' must be an array", i));
        }

        // Validate channels
        if let Some(channels) = stem_obj.get("channels").and_then(|v| v.as_u64()) {
            if channels != 1 && channels != 2 {
                result.error(format!(
                    "Stem {}: channels must be 1 (mono) or 2 (stereo), got {}",
                    i, channels
                ));
            }
        } else {
            result.error(format!("Stem {}: 'channels' must be a positive integer", i));
        }
    }

    result
}

/// Main validation dispatcher - validates output based on operation type
pub fn validate_output(operation: &str, output: &Value) -> ValidationResult {
    match operation {
        "keyframes" => validate_keyframes(output),
        "object-detection" => validate_object_detection(output),
        "face-detection" => validate_face_detection(output),
        "transcription" => validate_transcription(output),
        "ocr" => validate_ocr(output),
        "vision-embeddings" => validate_embeddings(output, Some(512)), // CLIP-ViT-B/32 is 512-dim
        "audio-embeddings" => validate_embeddings(output, None),       // Don't know dimension
        "text-embeddings" => validate_embeddings(output, None),        // Don't know dimension
        "scene-detection" => validate_scene_detection(output),
        "metadata-extraction" => validate_metadata_extraction(output),
        "duplicate-detection" => validate_duplicate_detection(output),
        "smart-thumbnail" => validate_smart_thumbnail(output),
        "voice-activity-detection" => validate_voice_activity_detection(output),
        "emotion-detection" => validate_emotion_detection(output),
        "image-quality-assessment" => validate_image_quality_assessment(output),
        "subtitle-extraction" => validate_subtitle_extraction(output),
        "pose-estimation" => validate_pose_estimation(output),
        "action-recognition" => validate_action_recognition(output),
        "shot-classification" => validate_shot_classification(output),
        "content-moderation" => validate_content_moderation(output),
        "logo-detection" => validate_logo_detection(output),
        "depth-estimation" => validate_depth_estimation(output),
        "caption-generation" => validate_caption_generation(output),
        "audio-classification" => validate_audio_classification(output),
        "diarization" => validate_diarization(output),
        "acoustic-scene-classification" => validate_acoustic_scene_classification(output),
        "profanity-detection" => validate_profanity_detection(output),
        "motion-tracking" => validate_motion_tracking(output),
        "audio-enhancement-metadata" => validate_audio_enhancement_metadata(output),
        "music-source-separation" => validate_music_source_separation(output),
        _ => {
            let mut result = ValidationResult::default();
            result.warn(format!(
                "No validator implemented for operation '{}'",
                operation
            ));
            result
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_valid_keyframes() {
        let output = json!([
            {
                "frame_number": 0,
                "timestamp": 0.0,
                "hash": 12345,
                "sharpness": 0.8,
                "thumbnail_paths": {
                    "640x480": "/tmp/frame_0.jpg"
                }
            }
        ]);

        let result = validate_keyframes(&output);
        assert!(result.valid, "Errors: {:?}", result.errors);
        assert!(
            result.warnings.is_empty(),
            "Warnings: {:?}",
            result.warnings
        );
    }

    #[test]
    fn test_keyframes_with_zero_hash() {
        let output = json!([
            {
                "frame_number": 0,
                "timestamp": 0.0,
                "hash": 0,
                "sharpness": 0.0,
                "thumbnail_paths": {
                    "640x480": "/tmp/frame_0.jpg"
                }
            }
        ]);

        let result = validate_keyframes(&output);
        assert!(result.valid, "Should be valid but with warnings");
        assert_eq!(result.warnings.len(), 2); // hash=0 and sharpness=0.0 warnings
    }

    #[test]
    fn test_invalid_keyframes_negative_timestamp() {
        let output = json!([
            {
                "frame_number": 0,
                "timestamp": -1.0,
                "hash": 0,
                "sharpness": 0.0,
                "thumbnail_paths": {}
            }
        ]);

        let result = validate_keyframes(&output);
        assert!(!result.valid, "Should be invalid");
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_valid_object_detection() {
        let output = json!([
            {
                "confidence": 0.95,
                "bbox": {
                    "x": 0.1,
                    "y": 0.2,
                    "width": 0.3,
                    "height": 0.4
                },
                "class_id": 1,
                "class_name": "person"
            }
        ]);

        let result = validate_object_detection(&output);
        assert!(result.valid, "Errors: {:?}", result.errors);
    }

    #[test]
    fn test_invalid_object_detection_confidence() {
        let output = json!([
            {
                "confidence": 1.5, // Invalid: > 1.0
                "bbox": {
                    "x": 0.1,
                    "y": 0.2,
                    "width": 0.3,
                    "height": 0.4
                },
                "class_id": 1,
                "class_name": "person"
            }
        ]);

        let result = validate_object_detection(&output);
        assert!(!result.valid);
    }

    #[test]
    fn test_valid_embeddings() {
        // Create a normalized 512-dim embedding
        let embedding: Vec<f64> = (0..512).map(|i| (i as f64 / 512.0) * 0.01).collect();
        let norm = embedding.iter().map(|v| v * v).sum::<f64>().sqrt();
        let normalized: Vec<f64> = embedding.iter().map(|v| v / norm).collect();

        let output = json!({
            "embedding": normalized
        });

        let result = validate_embeddings(&output, Some(512));
        assert!(result.valid, "Errors: {:?}", result.errors);
    }
}
