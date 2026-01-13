/// Comprehensive metadata extraction for all 23 output types
/// Implements USER DIRECTIVE from N=253: "What about the metadata? we need those, too!"
/// Returns detailed JSON with type_specific fields per OUTPUT_METADATA_SPECIFICATION.md
use serde_json::json;
use std::path::Path;

/// Extract comprehensive metadata for any operation type
pub fn extract_comprehensive_metadata(operation: &str, output_dir: &Path) -> Option<String> {
    // Extract first operation from pipeline (e.g., "audio;transcription" -> "transcription")
    let first_op = operation.split(';').next_back().unwrap_or(operation);

    match first_op {
        op if op.contains("keyframes") => extract_keyframes_metadata(output_dir),
        op if op.contains("transcription") => extract_transcription_metadata(output_dir),
        op if op.contains("object-detection") => extract_object_detection_metadata(output_dir),
        op if op.contains("face-detection") => extract_face_detection_metadata(output_dir),
        op if op.contains("audio") => extract_audio_metadata(output_dir),
        op if op.contains("ocr") => extract_ocr_metadata(output_dir),
        op if op.contains("scene-detection") => extract_scene_detection_metadata(output_dir),
        op if op.contains("diarization") => extract_diarization_metadata(output_dir),
        op if op.contains("pose-estimation") => extract_pose_estimation_metadata(output_dir),
        op if op.contains("emotion-detection") => extract_emotion_detection_metadata(output_dir),
        op if op.contains("vision-embeddings") => extract_vision_embeddings_metadata(output_dir),
        op if op.contains("audio-embeddings") => extract_audio_embeddings_metadata(output_dir),
        op if op.contains("text-embeddings") => extract_text_embeddings_metadata(output_dir),
        op if op.contains("audio-classification") => {
            extract_audio_classification_metadata(output_dir)
        }
        op if op.contains("audio-enhancement") => extract_audio_enhancement_metadata(output_dir),
        op if op.contains("subtitle-extraction") => extract_subtitle_metadata(output_dir),
        op if op.contains("format-conversion") => extract_format_conversion_metadata(output_dir),
        op if op.contains("metadata-extraction") => {
            extract_metadata_extraction_metadata(output_dir)
        }
        op if op.contains("image-quality") => extract_image_quality_metadata(output_dir),
        op if op.contains("shot-classification") => {
            extract_shot_classification_metadata(output_dir)
        }
        op if op.contains("action-recognition") => extract_action_recognition_metadata(output_dir),
        op if op.contains("smart-thumbnail") => extract_smart_thumbnail_metadata(output_dir),
        op if op.contains("motion-tracking") => extract_motion_tracking_metadata(output_dir),
        op if op.contains("content-moderation") => extract_content_moderation_metadata(output_dir),
        op if op.contains("logo-detection") => extract_logo_detection_metadata(output_dir),
        op if op.contains("caption-generation") => extract_caption_generation_metadata(output_dir),
        op if op.contains("depth-estimation") => extract_depth_estimation_metadata(output_dir),
        _ => extract_generic_metadata(first_op, output_dir),
    }
}

/// Keyframes: dimensions, sizes, JPEG quality, color profiles
fn extract_keyframes_metadata(output_dir: &Path) -> Option<String> {
    let json_path = output_dir.join("stage_00_keyframes.json");
    if !json_path.exists() {
        return None;
    }

    let data = std::fs::read(&json_path).ok()?;
    let md5_hash = format!("{:x}", md5::compute(&data));

    let json: serde_json::Value = serde_json::from_slice(&data).ok()?;

    // Handle both formats: array directly or wrapped in {"keyframes": [...]}
    let keyframes = if let Some(arr) = json.as_array() {
        arr
    } else {
        json.get("keyframes")?.as_array()?
    };

    let keyframe_count = keyframes.len();
    let mut dimensions = Vec::with_capacity(keyframe_count);
    let mut sizes = Vec::with_capacity(keyframe_count);
    let mut total_bytes = 0u64;

    // Extract dimensions and sizes from actual output files if they exist
    for kf in keyframes {
        if let Some(paths) = kf.get("thumbnail_paths").and_then(|p| p.as_object()) {
            for (_, path_value) in paths {
                if let Some(path_str) = path_value.as_str() {
                    let path = Path::new(path_str);

                    // Try to read image dimensions using image crate
                    if let Ok(img) = image::open(path) {
                        dimensions.push(json!({
                            "width": img.width(),
                            "height": img.height()
                        }));
                    }

                    // Get file size
                    if let Ok(meta) = std::fs::metadata(path) {
                        let size = meta.len();
                        sizes.push(size);
                        total_bytes += size;
                    }
                }
            }
        }
    }

    let size_summary = if !sizes.is_empty() {
        json!({
            "min_bytes": sizes.iter().min().unwrap(),
            "max_bytes": sizes.iter().max().unwrap(),
            "mean_bytes": total_bytes / sizes.len() as u64
        })
    } else {
        json!(null)
    };

    let metadata = json!({
        "output_type": "keyframes",
        "md5_hash": md5_hash,
        "primary_file": json_path.to_string_lossy(),
        "primary_file_size": data.len(),
        "type_specific": {
            "keyframe_count": keyframe_count,
            "dimensions": dimensions,
            "sizes": sizes,
            "total_bytes": total_bytes,
            "size_summary": size_summary
        }
    });

    Some(metadata.to_string())
}

/// Transcription: text length, language, confidence, segments
fn extract_transcription_metadata(output_dir: &Path) -> Option<String> {
    let json_path = output_dir.join("stage_01_transcription.json");
    if !json_path.exists() {
        return None;
    }

    let data = std::fs::read(&json_path).ok()?;
    let md5_hash = format!("{:x}", md5::compute(&data));

    let json: serde_json::Value = serde_json::from_slice(&data).ok()?;

    // Extract full text
    let full_text = json.get("text").and_then(|t| t.as_str()).unwrap_or("");
    let char_count = full_text.len();
    let word_count = full_text.split_whitespace().count();

    // Extract language
    let language = json
        .get("language")
        .and_then(|l| l.as_str())
        .map(|s| s.to_string());

    // Extract segments for statistics
    let segments = json.get("segments").and_then(|s| s.as_array());
    let segment_count = segments.map(|s| s.len()).unwrap_or(0);

    // Calculate confidence statistics if available
    let mut confidences = Vec::with_capacity(segment_count);
    if let Some(segs) = segments {
        for seg in segs {
            if let Some(conf) = seg.get("confidence").and_then(|c| c.as_f64()) {
                confidences.push(conf);
            }
        }
    }

    let confidence_stats = if !confidences.is_empty() {
        let sum: f64 = confidences.iter().sum();
        let mean = sum / confidences.len() as f64;
        let min = confidences.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = confidences
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);

        json!({
            "mean": mean,
            "min": min,
            "max": max,
            "count": confidences.len()
        })
    } else {
        json!(null)
    };

    let metadata = json!({
        "output_type": "transcription",
        "md5_hash": md5_hash,
        "primary_file": json_path.to_string_lossy(),
        "primary_file_size": data.len(),
        "type_specific": {
            "text_length_chars": char_count,
            "text_length_words": word_count,
            "language": language,
            "segment_count": segment_count,
            "confidence_stats": confidence_stats
        }
    });

    Some(metadata.to_string())
}

/// Object Detection: count, classes, bbox areas, confidence stats
fn extract_object_detection_metadata(output_dir: &Path) -> Option<String> {
    let json_path = output_dir.join("stage_01_object_detection.json");
    if !json_path.exists() {
        return None;
    }

    let data = std::fs::read(&json_path).ok()?;
    let md5_hash = format!("{:x}", md5::compute(&data));

    let json: serde_json::Value = serde_json::from_slice(&data).ok()?;

    // Extract detections array
    let detections = json.get("detections").and_then(|d| d.as_array());
    let detection_count = detections.map(|d| d.len()).unwrap_or(0);

    let mut classes = Vec::with_capacity(detection_count);
    let mut confidences = Vec::with_capacity(detection_count);
    let mut bbox_areas = Vec::with_capacity(detection_count);

    if let Some(dets) = detections {
        for det in dets {
            // Extract class
            if let Some(class) = det.get("class").and_then(|c| c.as_str()) {
                classes.push(class.to_string());
            }

            // Extract confidence
            if let Some(conf) = det.get("confidence").and_then(|c| c.as_f64()) {
                confidences.push(conf);
            }

            // Calculate bbox area if available
            if let Some(bbox) = det.get("bbox").and_then(|b| b.as_object()) {
                let width = bbox.get("width").and_then(|w| w.as_f64()).unwrap_or(0.0);
                let height = bbox.get("height").and_then(|h| h.as_f64()).unwrap_or(0.0);
                bbox_areas.push(width * height);
            }
        }
    }

    // Calculate confidence statistics
    let confidence_stats = if !confidences.is_empty() {
        let sum: f64 = confidences.iter().sum();
        let mean = sum / confidences.len() as f64;
        let min = confidences.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = confidences
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);

        json!({
            "mean": mean,
            "min": min,
            "max": max
        })
    } else {
        json!(null)
    };

    // Get unique classes
    let mut unique_classes = classes.clone();
    unique_classes.sort();
    unique_classes.dedup();

    let metadata = json!({
        "output_type": "object_detection",
        "md5_hash": md5_hash,
        "primary_file": json_path.to_string_lossy(),
        "primary_file_size": data.len(),
        "type_specific": {
            "detection_count": detection_count,
            "unique_classes": unique_classes,
            "confidence_stats": confidence_stats,
            "bbox_area_stats": if !bbox_areas.is_empty() {
                let sum: f64 = bbox_areas.iter().sum();
                json!({
                    "mean": sum / bbox_areas.len() as f64,
                    "min": bbox_areas.iter().cloned().fold(f64::INFINITY, f64::min),
                    "max": bbox_areas.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
                })
            } else {
                json!(null)
            }
        }
    });

    Some(metadata.to_string())
}

/// Face Detection: count, landmarks, bbox areas, confidence
fn extract_face_detection_metadata(output_dir: &Path) -> Option<String> {
    let json_path = output_dir.join("stage_01_face_detection.json");
    if !json_path.exists() {
        return None;
    }

    let data = std::fs::read(&json_path).ok()?;
    let md5_hash = format!("{:x}", md5::compute(&data));

    let json: serde_json::Value = serde_json::from_slice(&data).ok()?;

    // Extract faces array
    let faces = json.get("faces").and_then(|f| f.as_array());
    let face_count = faces.map(|f| f.len()).unwrap_or(0);

    let mut confidences = Vec::with_capacity(face_count);
    let mut bbox_areas = Vec::with_capacity(face_count);
    let mut landmark_counts = Vec::with_capacity(face_count);

    if let Some(face_arr) = faces {
        for face in face_arr {
            // Extract confidence
            if let Some(conf) = face.get("confidence").and_then(|c| c.as_f64()) {
                confidences.push(conf);
            }

            // Calculate bbox area
            if let Some(bbox) = face.get("bbox").and_then(|b| b.as_object()) {
                let width = bbox.get("width").and_then(|w| w.as_f64()).unwrap_or(0.0);
                let height = bbox.get("height").and_then(|h| h.as_f64()).unwrap_or(0.0);
                bbox_areas.push(width * height);
            }

            // Count landmarks
            if let Some(landmarks) = face.get("landmarks").and_then(|l| l.as_array()) {
                landmark_counts.push(landmarks.len());
            }
        }
    }

    let confidence_stats = if !confidences.is_empty() {
        let sum: f64 = confidences.iter().sum();
        json!({
            "mean": sum / confidences.len() as f64,
            "min": confidences.iter().cloned().fold(f64::INFINITY, f64::min),
            "max": confidences.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
        })
    } else {
        json!(null)
    };

    let metadata = json!({
        "output_type": "face_detection",
        "md5_hash": md5_hash,
        "primary_file": json_path.to_string_lossy(),
        "primary_file_size": data.len(),
        "type_specific": {
            "face_count": face_count,
            "confidence_stats": confidence_stats,
            "landmark_counts": landmark_counts,
            "bbox_area_stats": if !bbox_areas.is_empty() {
                let sum: f64 = bbox_areas.iter().sum();
                json!({
                    "mean": sum / bbox_areas.len() as f64,
                    "min": bbox_areas.iter().cloned().fold(f64::INFINITY, f64::min),
                    "max": bbox_areas.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
                })
            } else {
                json!(null)
            }
        }
    });

    Some(metadata.to_string())
}

/// Audio Extraction: duration, sample rate, channels, bit depth
fn extract_audio_metadata(output_dir: &Path) -> Option<String> {
    let audio_path = output_dir.join("stage_00_audio_extraction.wav");
    if !audio_path.exists() {
        return None;
    }

    let data = std::fs::read(&audio_path).ok()?;
    let md5_hash = format!("{:x}", md5::compute(&data));
    let file_size = data.len();

    // Parse WAV header for metadata
    let (sample_rate, channels, bit_depth, duration_sec) = parse_wav_header(&data)?;

    let metadata = json!({
        "output_type": "audio_extraction",
        "md5_hash": md5_hash,
        "primary_file": audio_path.to_string_lossy(),
        "primary_file_size": file_size,
        "type_specific": {
            "duration_sec": duration_sec,
            "sample_rate": sample_rate,
            "channels": channels,
            "bit_depth": bit_depth,
            "format": "WAV",
            "codec": "pcm_s16le"
        }
    });

    Some(metadata.to_string())
}

/// Parse WAV header to extract audio metadata
/// Returns: (sample_rate, channels, bit_depth, duration_sec)
fn parse_wav_header(data: &[u8]) -> Option<(u32, u16, u16, f64)> {
    if data.len() < 44 {
        return None;
    }

    // Check for RIFF header
    if &data[0..4] != b"RIFF" || &data[8..12] != b"WAVE" {
        return None;
    }

    // Find fmt chunk
    let mut pos = 12;
    while pos + 8 <= data.len() {
        let chunk_id = &data[pos..pos + 4];
        let chunk_size =
            u32::from_le_bytes([data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]])
                as usize;

        if chunk_id == b"fmt " {
            if pos + 8 + chunk_size > data.len() {
                return None;
            }

            // Extract fmt chunk data
            let channels = u16::from_le_bytes([data[pos + 10], data[pos + 11]]);
            let sample_rate = u32::from_le_bytes([
                data[pos + 12],
                data[pos + 13],
                data[pos + 14],
                data[pos + 15],
            ]);
            let bit_depth = u16::from_le_bytes([data[pos + 22], data[pos + 23]]);

            // Calculate duration from data chunk
            let duration_sec = calculate_wav_duration(data, sample_rate, channels, bit_depth);

            return Some((sample_rate, channels, bit_depth, duration_sec));
        }

        pos += 8 + chunk_size;
    }

    None
}

/// Calculate WAV duration by finding data chunk
fn calculate_wav_duration(data: &[u8], sample_rate: u32, channels: u16, bit_depth: u16) -> f64 {
    let mut pos = 12;

    while pos + 8 <= data.len() {
        let chunk_id = &data[pos..pos + 4];
        let chunk_size =
            u32::from_le_bytes([data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]])
                as usize;

        if chunk_id == b"data" {
            let byte_rate = sample_rate * channels as u32 * bit_depth as u32 / 8;
            if byte_rate > 0 {
                return chunk_size as f64 / byte_rate as f64;
            } else {
                return 0.0;
            }
        }

        pos += 8 + chunk_size;
    }

    0.0
}

// Remaining 18 extractors (simplified versions for now)

fn extract_ocr_metadata(output_dir: &Path) -> Option<String> {
    extract_json_output_metadata("ocr", output_dir)
}

fn extract_scene_detection_metadata(output_dir: &Path) -> Option<String> {
    extract_json_output_metadata("scene_detection", output_dir)
}

fn extract_diarization_metadata(output_dir: &Path) -> Option<String> {
    extract_json_output_metadata("diarization", output_dir)
}

fn extract_pose_estimation_metadata(output_dir: &Path) -> Option<String> {
    extract_json_output_metadata("pose_estimation", output_dir)
}

fn extract_emotion_detection_metadata(output_dir: &Path) -> Option<String> {
    extract_json_output_metadata("emotion_detection", output_dir)
}

fn extract_vision_embeddings_metadata(output_dir: &Path) -> Option<String> {
    extract_json_output_metadata("vision_embeddings", output_dir)
}

fn extract_audio_embeddings_metadata(output_dir: &Path) -> Option<String> {
    extract_json_output_metadata("audio_embeddings", output_dir)
}

fn extract_text_embeddings_metadata(output_dir: &Path) -> Option<String> {
    extract_json_output_metadata("text_embeddings", output_dir)
}

fn extract_audio_classification_metadata(output_dir: &Path) -> Option<String> {
    extract_json_output_metadata("audio_classification", output_dir)
}

fn extract_audio_enhancement_metadata(output_dir: &Path) -> Option<String> {
    extract_json_output_metadata("audio_enhancement_metadata", output_dir)
}

fn extract_subtitle_metadata(output_dir: &Path) -> Option<String> {
    extract_json_output_metadata("subtitle_extraction", output_dir)
}

fn extract_format_conversion_metadata(output_dir: &Path) -> Option<String> {
    extract_json_output_metadata("format_conversion", output_dir)
}

fn extract_metadata_extraction_metadata(output_dir: &Path) -> Option<String> {
    extract_json_output_metadata("metadata_extraction", output_dir)
}

fn extract_image_quality_metadata(output_dir: &Path) -> Option<String> {
    extract_json_output_metadata("image_quality_assessment", output_dir)
}

fn extract_shot_classification_metadata(output_dir: &Path) -> Option<String> {
    extract_json_output_metadata("shot_classification", output_dir)
}

fn extract_action_recognition_metadata(output_dir: &Path) -> Option<String> {
    extract_json_output_metadata("action_recognition", output_dir)
}

fn extract_smart_thumbnail_metadata(output_dir: &Path) -> Option<String> {
    extract_json_output_metadata("smart_thumbnail", output_dir)
}

fn extract_motion_tracking_metadata(output_dir: &Path) -> Option<String> {
    extract_json_output_metadata("motion_tracking", output_dir)
}

fn extract_content_moderation_metadata(output_dir: &Path) -> Option<String> {
    extract_json_output_metadata("content_moderation", output_dir)
}

fn extract_logo_detection_metadata(output_dir: &Path) -> Option<String> {
    extract_json_output_metadata("logo_detection", output_dir)
}

fn extract_caption_generation_metadata(output_dir: &Path) -> Option<String> {
    extract_json_output_metadata("caption_generation", output_dir)
}

fn extract_depth_estimation_metadata(output_dir: &Path) -> Option<String> {
    extract_json_output_metadata("depth_estimation", output_dir)
}

fn extract_generic_metadata(operation: &str, output_dir: &Path) -> Option<String> {
    let op_normalized = operation.replace("-", "_");
    extract_json_output_metadata(&op_normalized, output_dir)
}

/// Generic JSON output metadata extraction
/// Extracts basic metadata from JSON outputs (array length, object keys, file size)
fn extract_json_output_metadata(operation: &str, output_dir: &Path) -> Option<String> {
    // Try stage_01 first (plugins), then stage_00 (core operations)
    let json_path_01 = output_dir.join(format!("stage_01_{}.json", operation));
    let json_path_00 = output_dir.join(format!("stage_00_{}.json", operation));

    let json_path = if json_path_01.exists() {
        json_path_01
    } else if json_path_00.exists() {
        json_path_00
    } else {
        return None;
    };

    let data = std::fs::read(&json_path).ok()?;
    let md5_hash = format!("{:x}", md5::compute(&data));
    let file_size = data.len();

    // Parse JSON to extract basic statistics
    let json: serde_json::Value = serde_json::from_slice(&data).ok()?;

    let type_specific = match &json {
        serde_json::Value::Array(arr) => {
            json!({
                "array_length": arr.len()
            })
        }
        serde_json::Value::Object(obj) => {
            let mut stats = serde_json::Map::new();
            stats.insert("key_count".to_string(), json!(obj.len()));
            stats.insert("keys".to_string(), json!(obj.keys().collect::<Vec<_>>()));

            // If there's an array field, count it
            for (key, value) in obj {
                if let serde_json::Value::Array(arr) = value {
                    stats.insert(format!("{}_count", key), json!(arr.len()));
                }
            }

            json!(stats)
        }
        _ => json!({}),
    };

    let metadata = json!({
        "output_type": operation,
        "md5_hash": md5_hash,
        "primary_file": json_path.to_string_lossy(),
        "primary_file_size": file_size,
        "type_specific": type_specific
    });

    Some(metadata.to_string())
}
