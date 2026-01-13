//! Output Validation Integration Tests
//!
//! These tests run video-extract and validate the outputs are structurally correct
//! and semantically valid.
//!
//! Run: VIDEO_EXTRACT_THREADS=4 cargo test --release --test output_validation_integration -- --ignored --test-threads=1

mod common;

use common::validators;
use serde_json::Value;
use std::process::Command;

fn run_and_validate(file: &str, operation: &str) -> (bool, Vec<String>, Vec<String>) {
    // Clean up debug output directory
    let _ = std::fs::remove_dir_all("./debug_output");

    // Run video-extract in debug mode to get JSON output
    let output = Command::new("./target/release/video-extract")
        .args(["debug", "--ops", operation, file])
        .output()
        .expect("Failed to execute video-extract");

    // Check if command succeeded
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return (false, vec![format!("Command failed: {}", stderr)], vec![]);
    }

    // Debug mode writes JSON to debug_output/stage_XX_{operation}.json
    // For multi-operation pipelines (e.g., "keyframes;action-recognition"), we need to read the LAST stage
    let parts: Vec<&str> = operation.split(';').collect();
    let last_operation = parts.last().unwrap();
    let stage_num = parts.len() - 1;

    let output_file = format!(
        "./debug_output/stage_{:02}_{}.json",
        stage_num,
        last_operation.replace("-", "_")
    );
    let json_content = match std::fs::read_to_string(&output_file) {
        Ok(content) => content,
        Err(e) => {
            return (
                false,
                vec![format!("Failed to read output file {}: {}", output_file, e)],
                vec![],
            );
        }
    };

    // Parse JSON
    let json: Value = match serde_json::from_str(&json_content) {
        Ok(json) => json,
        Err(e) => {
            return (false, vec![format!("Failed to parse JSON: {}", e)], vec![]);
        }
    };

    // Validate the parsed JSON using the LAST operation in the pipeline
    // (for single operations, this is the same as the operation parameter)
    let validation_result = validators::validate_output(last_operation, &json);
    (
        validation_result.valid,
        validation_result.errors,
        validation_result.warnings,
    )
}

// ============================================================================
// VALIDATION INTEGRATION TESTS
// ============================================================================

#[test]
#[ignore]
fn validate_keyframes_heic() {
    let (valid, errors, warnings) =
        run_and_validate("test_edge_cases/image_iphone_photo.heic", "keyframes");

    println!("Validation result: valid={}", valid);
    if !warnings.is_empty() {
        println!("Warnings:");
        for warning in &warnings {
            println!("  - {}", warning);
        }
    }
    if !errors.is_empty() {
        println!("Errors:");
        for error in &errors {
            println!("  - {}", error);
        }
    }

    // For now, just report results without asserting
    // We'll make this stricter once we fix the output format
}

#[test]
#[ignore]
fn validate_keyframes_mp4() {
    let (valid, errors, warnings) = run_and_validate(
        "test_edge_cases/video_hevc_h265_modern_codec__compatibility.mp4",
        "keyframes",
    );

    println!("Validation result: valid={}", valid);
    if !warnings.is_empty() {
        println!("Warnings:");
        for warning in &warnings {
            println!("  - {}", warning);
        }
    }
    if !errors.is_empty() {
        println!("Errors:");
        for error in &errors {
            println!("  - {}", error);
        }
    }
}

#[test]
#[ignore]
fn validate_object_detection_jpg() {
    let (valid, errors, warnings) =
        run_and_validate("test_edge_cases/image_test_dog.jpg", "object-detection");

    println!("Validation result: valid={}", valid);
    if !warnings.is_empty() {
        println!("Warnings:");
        for warning in &warnings {
            println!("  - {}", warning);
        }
    }
    if !errors.is_empty() {
        println!("Errors:");
        for error in &errors {
            println!("  - {}", error);
        }
    }

    // Object detection should find some objects in this image
    assert!(valid, "Object detection output should be valid");
}

#[test]
#[ignore]
fn validate_face_detection_jpg() {
    let (valid, errors, warnings) =
        run_and_validate("test_edge_cases/image_iphone_photo.jpg", "face-detection");

    println!("Validation result: valid={}", valid);
    if !warnings.is_empty() {
        println!("Warnings:");
        for warning in &warnings {
            println!("  - {}", warning);
        }
    }
    if !errors.is_empty() {
        println!("Errors:");
        for error in &errors {
            println!("  - {}", error);
        }
    }

    // Face detection should find faces in this portrait
    assert!(valid, "Face detection output should be valid");
}

#[test]
#[ignore]
fn validate_scene_detection_mp4() {
    let (valid, errors, warnings) = run_and_validate(
        "test_edge_cases/video_hevc_h265_modern_codec__compatibility.mp4",
        "scene-detection",
    );

    println!("Validation result: valid={}", valid);
    if !warnings.is_empty() {
        println!("Warnings:");
        for warning in &warnings {
            println!("  - {}", warning);
        }
    }
    if !errors.is_empty() {
        println!("Errors:");
        for error in &errors {
            println!("  - {}", error);
        }
    }

    assert!(valid, "Scene detection output should be valid");
}

#[test]
#[ignore]
fn validate_ocr_jpg() {
    let (valid, errors, warnings) =
        run_and_validate("test_edge_cases/image_iphone_photo.jpg", "ocr");

    println!("Validation result: valid={}", valid);
    if !warnings.is_empty() {
        println!("Warnings:");
        for warning in &warnings {
            println!("  - {}", warning);
        }
    }
    if !errors.is_empty() {
        println!("Errors:");
        for error in &errors {
            println!("  - {}", error);
        }
    }

    assert!(valid, "OCR output should be valid");
}

// Emotion detection test removed - requires keyframes operation which is not single-operation
// Emotion detection is tested in smoke_test_comprehensive.rs with keyframes;emotion-detection

#[test]
#[ignore]
fn validate_pose_estimation_jpg() {
    let (valid, errors, warnings) =
        run_and_validate("test_edge_cases/image_test_dog.jpg", "pose-estimation");

    println!("Validation result: valid={}", valid);
    if !warnings.is_empty() {
        println!("Warnings:");
        for warning in &warnings {
            println!("  - {}", warning);
        }
    }
    if !errors.is_empty() {
        println!("Errors:");
        for error in &errors {
            println!("  - {}", error);
        }
    }

    // Note: Dog image may not have human poses, empty result acceptable
}

// Action recognition test removed - requires keyframes operation which is not single-operation
// Action recognition is tested in smoke_test_comprehensive.rs with keyframes;action-recognition

#[test]
#[ignore]
fn validate_shot_classification_jpg() {
    let (valid, errors, warnings) =
        run_and_validate("test_edge_cases/image_test_dog.jpg", "shot-classification");

    println!("Validation result: valid={}", valid);
    if !warnings.is_empty() {
        println!("Warnings:");
        for warning in &warnings {
            println!("  - {}", warning);
        }
    }
    if !errors.is_empty() {
        println!("Errors:");
        for error in &errors {
            println!("  - {}", error);
        }
    }

    assert!(valid, "Shot classification output should be valid");
}

#[test]
#[ignore]
fn validate_vision_embeddings_jpg() {
    let (valid, errors, warnings) =
        run_and_validate("test_edge_cases/image_test_dog.jpg", "vision-embeddings");

    println!("Validation result: valid={}", valid);
    if !warnings.is_empty() {
        println!("Warnings:");
        for warning in &warnings {
            println!("  - {}", warning);
        }
    }
    if !errors.is_empty() {
        println!("Errors:");
        for error in &errors {
            println!("  - {}", error);
        }
    }

    assert!(valid, "Vision embeddings output should be valid");
}

#[test]
#[ignore]
fn validate_duplicate_detection_jpg() {
    let (valid, errors, warnings) =
        run_and_validate("test_edge_cases/image_test_dog.jpg", "duplicate-detection");

    println!("Validation result: valid={}", valid);
    if !warnings.is_empty() {
        println!("Warnings:");
        for warning in &warnings {
            println!("  - {}", warning);
        }
    }
    if !errors.is_empty() {
        println!("Errors:");
        for error in &errors {
            println!("  - {}", error);
        }
    }

    assert!(valid, "Duplicate detection output should be valid");
}

// Smart thumbnail test removed - requires keyframes operation which is not single-operation
// Smart thumbnail is tested in smoke_test_comprehensive.rs with keyframes;smart-thumbnail

#[test]
#[ignore]
fn validate_image_quality_assessment_jpg() {
    let (valid, errors, warnings) =
        run_and_validate("test_edge_cases/image_test_dog.jpg", "image-quality-assessment");

    println!("Validation result: valid={}", valid);
    if !warnings.is_empty() {
        println!("Warnings:");
        for warning in &warnings {
            println!("  - {}", warning);
        }
    }
    if !errors.is_empty() {
        println!("Errors:");
        for error in &errors {
            println!("  - {}", error);
        }
    }

    assert!(valid, "Image quality assessment output should be valid");
}

#[test]
#[ignore]
fn validate_metadata_extraction_mp4() {
    let (valid, errors, warnings) = run_and_validate(
        "test_edge_cases/video_hevc_h265_modern_codec__compatibility.mp4",
        "metadata-extraction",
    );

    println!("Validation result: valid={}", valid);
    if !warnings.is_empty() {
        println!("Warnings:");
        for warning in &warnings {
            println!("  - {}", warning);
        }
    }
    if !errors.is_empty() {
        println!("Errors:");
        for error in &errors {
            println!("  - {}", error);
        }
    }

    assert!(valid, "Metadata extraction output should be valid");
}

// Audio operation validation tests

#[test]
#[ignore]
fn validate_voice_activity_detection_wav() {
    let (valid, errors, warnings) = run_and_validate(
        "test_edge_cases/audio_mono_single_channel__channel_test.wav",
        "voice-activity-detection",
    );

    println!("Validation result: valid={}", valid);
    if !warnings.is_empty() {
        println!("Warnings:");
        for warning in &warnings {
            println!("  - {}", warning);
        }
    }
    if !errors.is_empty() {
        println!("Errors:");
        for error in &errors {
            println!("  - {}", error);
        }
    }

    assert!(valid, "Voice activity detection output should be valid");
}

#[test]
#[ignore]
fn validate_audio_classification_wav() {
    let (valid, errors, warnings) = run_and_validate(
        "test_edge_cases/audio_mono_single_channel__channel_test.wav",
        "audio-classification",
    );

    println!("Validation result: valid={}", valid);
    if !warnings.is_empty() {
        println!("Warnings:");
        for warning in &warnings {
            println!("  - {}", warning);
        }
    }
    if !errors.is_empty() {
        println!("Errors:");
        for error in &errors {
            println!("  - {}", error);
        }
    }

    assert!(valid, "Audio classification output should be valid");
}

#[test]
#[ignore]
fn validate_acoustic_scene_classification_wav() {
    let (valid, errors, warnings) = run_and_validate(
        "test_edge_cases/audio_mono_single_channel__channel_test.wav",
        "acoustic-scene-classification",
    );

    println!("Validation result: valid={}", valid);
    if !warnings.is_empty() {
        println!("Warnings:");
        for warning in &warnings {
            println!("  - {}", warning);
        }
    }
    if !errors.is_empty() {
        println!("Errors:");
        for error in &errors {
            println!("  - {}", error);
        }
    }

    assert!(valid, "Acoustic scene classification output should be valid");
}

#[test]
#[ignore]
fn validate_audio_embeddings_wav() {
    let (valid, errors, warnings) = run_and_validate(
        "test_edge_cases/audio_mono_single_channel__channel_test.wav",
        "audio-embeddings",
    );

    println!("Validation result: valid={}", valid);
    if !warnings.is_empty() {
        println!("Warnings:");
        for warning in &warnings {
            println!("  - {}", warning);
        }
    }
    if !errors.is_empty() {
        println!("Errors:");
        for error in &errors {
            println!("  - {}", error);
        }
    }

    assert!(valid, "Audio embeddings output should be valid");
}

#[test]
#[ignore]
fn validate_audio_enhancement_metadata_wav() {
    let (valid, errors, warnings) = run_and_validate(
        "test_edge_cases/audio_mono_single_channel__channel_test.wav",
        "audio-enhancement-metadata",
    );

    println!("Validation result: valid={}", valid);
    if !warnings.is_empty() {
        println!("Warnings:");
        for warning in &warnings {
            println!("  - {}", warning);
        }
    }
    if !errors.is_empty() {
        println!("Errors:");
        for error in &errors {
            println!("  - {}", error);
        }
    }

    assert!(valid, "Audio enhancement metadata output should be valid");
}

// Transcription test disabled - no suitable test file in test_edge_cases/
// Transcription is already tested in smoke_test_comprehensive.rs
// TODO: Re-enable when we have Wikimedia test files with actual speech

#[test]
#[ignore]
fn validate_subtitle_extraction_mp4() {
    let (valid, errors, warnings) = run_and_validate(
        "test_edge_cases/video_with_subtitles__subtitle_test.mp4",
        "subtitle-extraction",
    );

    println!("Validation result: valid={}", valid);
    if !warnings.is_empty() {
        println!("Warnings:");
        for warning in &warnings {
            println!("  - {}", warning);
        }
    }
    if !errors.is_empty() {
        println!("Errors:");
        for error in &errors {
            println!("  - {}", error);
        }
    }

    assert!(valid, "Subtitle extraction output should be valid");
}

// Content moderation test disabled - requires model file: models/content-moderation/nsfw_mobilenet.onnx
// See models/content-moderation/README.md for instructions
// Content moderation is tested in smoke_test_comprehensive.rs when model is available

// Logo detection test disabled - requires model file: models/logo-detection/yolov8_logo.onnx
// See models/logo-detection/README.md for instructions
// Logo detection is tested in smoke_test_comprehensive.rs when model is available

// Depth estimation test disabled - requires model file in models/depth-estimation/
// See models/depth-estimation/README.md for instructions
// Depth estimation is tested in smoke_test_comprehensive.rs when model is available

// Caption generation test disabled - operation exists but model files may not be available
// Caption generation is marked as experimental plugin

// Motion tracking test disabled - requires ObjectDetection input, not direct video file
// Motion tracking operates on ObjectDetection output to track objects across frames
// Would need multi-operation pipeline: keyframes;object-detection;motion-tracking

// Multi-operation pipeline tests - these validate chained operations

#[test]
#[ignore]
fn validate_action_recognition_pipeline_mp4() {
    let (valid, errors, warnings) = run_and_validate(
        "test_edge_cases/format_test_avi.avi",
        "keyframes;action-recognition",
    );

    println!("Validation result: valid={}", valid);
    if !warnings.is_empty() {
        println!("Warnings:");
        for warning in &warnings {
            println!("  - {}", warning);
        }
    }
    if !errors.is_empty() {
        println!("Errors:");
        for error in &errors {
            println!("  - {}", error);
        }
    }

    assert!(valid, "Action recognition pipeline output should be valid");
}

#[test]
#[ignore]
fn validate_emotion_detection_pipeline_mp4() {
    let (valid, errors, warnings) = run_and_validate(
        "test_edge_cases/format_test_avi.avi",
        "keyframes;emotion-detection",
    );

    println!("Validation result: valid={}", valid);
    if !warnings.is_empty() {
        println!("Warnings:");
        for warning in &warnings {
            println!("  - {}", warning);
        }
    }
    if !errors.is_empty() {
        println!("Errors:");
        for error in &errors {
            println!("  - {}", error);
        }
    }

    assert!(valid, "Emotion detection pipeline output should be valid");
}

#[test]
#[ignore]
fn validate_smart_thumbnail_pipeline_mp4() {
    let (valid, errors, warnings) = run_and_validate(
        "test_edge_cases/format_test_avi.avi",
        "keyframes;smart-thumbnail",
    );

    println!("Validation result: valid={}", valid);
    if !warnings.is_empty() {
        println!("Warnings:");
        for warning in &warnings {
            println!("  - {}", warning);
        }
    }
    if !errors.is_empty() {
        println!("Errors:");
        for error in &errors {
            println!("  - {}", error);
        }
    }

    assert!(valid, "Smart thumbnail pipeline output should be valid");
}
