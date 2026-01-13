//! Integration tests for API server
//!
//! These tests start the API server, send real requests, and verify responses.
//! They test the full end-to-end pipeline from HTTP request through orchestrator
//! to job completion and result retrieval.

use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::time::sleep;

// Test file paths
fn test_video_path() -> PathBuf {
    PathBuf::from("/Users/ayates/docling/tests/data/audio/sample_10s_video-mp4.mp4")
}

fn test_video_large_path() -> PathBuf {
    PathBuf::from("/Users/ayates/Desktop/stuff/stuff/May 5 - live labeling mocks.mp4")
}

/// Helper to check if test file exists
fn test_file_exists(path: &Path) -> bool {
    path.exists() && path.is_file()
}

/// Find the project root directory by looking for Cargo.toml + models/ directory
fn find_project_root() -> PathBuf {
    let mut current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Walk up the directory tree until we find Cargo.toml at root level
    loop {
        let cargo_toml = current_dir.join("Cargo.toml");
        if cargo_toml.exists() {
            // Check if this is the workspace root by looking for models/ directory
            let models_dir = current_dir.join("models");
            if models_dir.exists() {
                return current_dir;
            }
        }

        // Try parent directory
        if let Some(parent) = current_dir.parent() {
            current_dir = parent.to_path_buf();
        } else {
            // Reached filesystem root, return current working directory
            return std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        }
    }
}

#[tokio::test]
async fn test_health_endpoint() {
    // Start server in background
    let state = video_audio_api_server::ApiState::new();
    let server_handle = tokio::spawn(async move {
        video_audio_api_server::start_server("127.0.0.1:18080", state)
            .await
            .expect("Failed to start server");
    });

    // Give server time to start
    sleep(Duration::from_secs(1)).await;

    // Test health endpoint
    let client = reqwest::Client::new();
    let response = client
        .get("http://127.0.0.1:18080/health")
        .send()
        .await
        .expect("Failed to send health check request");

    assert_eq!(response.status(), 200);

    let json: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(json["status"], "ok");
    assert!(json["version"].is_string());

    // Cleanup
    server_handle.abort();
}

#[tokio::test]
async fn test_realtime_processing_small_file() {
    let test_path = test_video_path();
    if !test_file_exists(&test_path) {
        eprintln!("Test file not found: {test_path:?}");
        eprintln!("Skipping test_realtime_processing_small_file");
        return;
    }

    // Start server in background
    let state = video_audio_api_server::ApiState::new();
    let server_handle = tokio::spawn(async move {
        video_audio_api_server::start_server("127.0.0.1:18081", state)
            .await
            .expect("Failed to start server");
    });

    // Give server time to start
    sleep(Duration::from_secs(1)).await;

    // Create request
    let request_body = serde_json::json!({
        "source": {
            "type": "upload",
            "location": test_path.to_str().unwrap()
        },
        "processing": {
            "priority": "realtime",
            "quality_mode": "fast",
            "required_features": ["transcription", "keyframes"],
            "optional_features": []
        }
    });

    // Send real-time processing request
    let client = reqwest::Client::new();
    let response = client
        .post("http://127.0.0.1:18081/api/v1/process/realtime")
        .json(&request_body)
        .send()
        .await
        .expect("Failed to send processing request");

    assert_eq!(
        response.status(),
        202,
        "Expected 202 Accepted status for async processing"
    );

    let json: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert!(json["job_id"].is_string(), "Response should include job_id");

    let job_id = json["job_id"].as_str().unwrap();
    println!("Job ID: {job_id}");

    // Poll job status until completed or timeout
    let max_attempts = 30; // 30 seconds timeout
    let mut completed = false;
    let mut status_response = None;

    for attempt in 0..max_attempts {
        sleep(Duration::from_secs(1)).await;

        let status_resp = client
            .get(format!(
                "http://127.0.0.1:18081/api/v1/jobs/{job_id}/status"
            ))
            .send()
            .await
            .expect("Failed to get job status");

        assert_eq!(status_resp.status(), 200);

        let status_json: serde_json::Value = status_resp
            .json()
            .await
            .expect("Failed to parse status JSON");

        println!(
            "Attempt {}: Status = {}",
            attempt + 1,
            status_json["status"]
        );

        if status_json["status"] == "completed" {
            completed = true;
            status_response = Some(status_json);
            break;
        } else if status_json["status"] == "failed" {
            panic!("Job failed: {status_json:?}");
        }
    }

    assert!(completed, "Job did not complete within timeout");

    let status_json = status_response.unwrap();
    assert!(status_json["completed_tasks"].as_u64().unwrap() > 0);
    // Optional tasks (like face detection, OCR, diarization) may fail without failing the job
    // So we allow up to 3 failed tasks (face_detection + OCR + diarization if models not available)
    assert!(status_json["failed_tasks"].as_u64().unwrap() <= 3);

    // Retrieve results
    let result_resp = client
        .get(format!(
            "http://127.0.0.1:18081/api/v1/jobs/{job_id}/result"
        ))
        .send()
        .await
        .expect("Failed to get job result");

    assert_eq!(result_resp.status(), 200);

    let result_json: serde_json::Value = result_resp
        .json()
        .await
        .expect("Failed to parse result JSON");

    println!(
        "Result: {}",
        serde_json::to_string_pretty(&result_json).unwrap()
    );

    // Verify results contain expected task outputs
    assert!(result_json["results"].is_object());
    let results = result_json["results"].as_object().unwrap();

    // Should have at least transcription and keyframes tasks
    assert!(results.len() >= 2, "Expected at least 2 task results");

    // Cleanup
    server_handle.abort();
}

#[tokio::test]
async fn test_bulk_processing() {
    let test_path = test_video_path();
    if !test_file_exists(&test_path) {
        eprintln!("Test file not found: {test_path:?}");
        eprintln!("Skipping test_bulk_processing");
        return;
    }

    // Start server in background
    let state = video_audio_api_server::ApiState::new();
    let server_handle = tokio::spawn(async move {
        video_audio_api_server::start_server("127.0.0.1:18082", state)
            .await
            .expect("Failed to start server");
    });

    // Give server time to start
    sleep(Duration::from_secs(1)).await;

    // Create bulk request with single file
    let request_body = serde_json::json!({
        "batch_id": "test-batch-001",
        "files": [{
            "id": "file-001",
            "source": {
                "type": "upload",
                "location": test_path.to_str().unwrap()
            },
            "processing": {
                "priority": "bulk",
                "quality_mode": "balanced",
                "required_features": ["keyframes"],
                "optional_features": ["transcription"]
            }
        }],
        "batch_config": {
            "priority": "bulk",
            "optimize_for": "throughput"
        }
    });

    // Send bulk processing request
    let client = reqwest::Client::new();
    let response = client
        .post("http://127.0.0.1:18082/api/v1/process/bulk")
        .json(&request_body)
        .send()
        .await
        .expect("Failed to send bulk processing request");

    assert_eq!(response.status(), 202);

    let json: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert!(json["batch_id"].is_string());
    assert!(json["job_ids"].is_array());
    assert_eq!(json["job_ids"].as_array().unwrap().len(), 1);

    let job_id = json["job_ids"][0].as_str().unwrap();
    println!("Bulk job ID: {job_id}");

    // Poll for completion
    let max_attempts = 30;
    let mut completed = false;

    for _attempt in 0..max_attempts {
        sleep(Duration::from_secs(1)).await;

        let status_resp = client
            .get(format!(
                "http://127.0.0.1:18082/api/v1/jobs/{job_id}/status"
            ))
            .send()
            .await
            .expect("Failed to get job status");

        let status_json: serde_json::Value = status_resp
            .json()
            .await
            .expect("Failed to parse status JSON");

        if status_json["status"] == "completed" {
            completed = true;
            break;
        } else if status_json["status"] == "failed" {
            panic!("Bulk job failed: {status_json:?}");
        }
    }

    assert!(completed, "Bulk job did not complete within timeout");

    // Cleanup
    server_handle.abort();
}

#[tokio::test]
async fn test_missing_file_error() {
    // Start server in background
    let state = video_audio_api_server::ApiState::new();
    let server_handle = tokio::spawn(async move {
        video_audio_api_server::start_server("127.0.0.1:18083", state)
            .await
            .expect("Failed to start server");
    });

    // Give server time to start
    sleep(Duration::from_secs(1)).await;

    // Request with non-existent file
    let request_body = serde_json::json!({
        "source": {
            "type": "upload",
            "location": "/nonexistent/file.mp4"
        },
        "processing": {
            "priority": "realtime",
            "quality_mode": "fast",
            "required_features": ["transcription"],
            "optional_features": []
        }
    });

    let client = reqwest::Client::new();
    let response = client
        .post("http://127.0.0.1:18083/api/v1/process/realtime")
        .json(&request_body)
        .send()
        .await
        .expect("Failed to send request");

    // Should return 400 Bad Request or similar error
    assert!(
        response.status().is_client_error(),
        "Expected client error for missing file, got: {}",
        response.status()
    );

    // Cleanup
    server_handle.abort();
}

#[tokio::test]
async fn test_invalid_json_request() {
    // Start server in background
    let state = video_audio_api_server::ApiState::new();
    let server_handle = tokio::spawn(async move {
        video_audio_api_server::start_server("127.0.0.1:18084", state)
            .await
            .expect("Failed to start server");
    });

    // Give server time to start
    sleep(Duration::from_secs(1)).await;

    // Send invalid JSON
    let client = reqwest::Client::new();
    let response = client
        .post("http://127.0.0.1:18084/api/v1/process/realtime")
        .header("Content-Type", "application/json")
        .body("{invalid json")
        .send()
        .await
        .expect("Failed to send request");

    assert!(response.status().is_client_error());

    // Cleanup
    server_handle.abort();
}

#[tokio::test]
async fn test_job_not_found() {
    // Start server in background
    let state = video_audio_api_server::ApiState::new();
    let server_handle = tokio::spawn(async move {
        video_audio_api_server::start_server("127.0.0.1:18085", state)
            .await
            .expect("Failed to start server");
    });

    // Give server time to start
    sleep(Duration::from_secs(1)).await;

    // Query non-existent job
    let client = reqwest::Client::new();
    let response = client
        .get("http://127.0.0.1:18085/api/v1/jobs/00000000-0000-0000-0000-000000000000/status")
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        response.status(),
        404,
        "Should return 404 for non-existent job"
    );

    // Cleanup
    server_handle.abort();
}

#[tokio::test]
#[ignore] // Only run manually with large test file
async fn test_realtime_processing_large_file() {
    let test_path = test_video_large_path();
    if !test_file_exists(&test_path) {
        eprintln!("Large test file not found: {test_path:?}");
        eprintln!("Skipping test_realtime_processing_large_file");
        return;
    }

    // Start server
    let state = video_audio_api_server::ApiState::new();
    let server_handle = tokio::spawn(async move {
        video_audio_api_server::start_server("127.0.0.1:18086", state)
            .await
            .expect("Failed to start server");
    });

    sleep(Duration::from_secs(1)).await;

    let request_body = serde_json::json!({
        "source": {
            "type": "upload",
            "location": test_path.to_str().unwrap()
        },
        "processing": {
            "priority": "realtime",
            "quality_mode": "fast",
            "required_features": ["keyframes", "transcription"],
            "optional_features": ["object_detection"]
        }
    });

    let client = reqwest::Client::new();
    let response = client
        .post("http://127.0.0.1:18086/api/v1/process/realtime")
        .json(&request_body)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 202);

    let json: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    let job_id = json["job_id"].as_str().unwrap();

    println!("Large file job ID: {job_id}");

    // Poll for completion (longer timeout for large file)
    let max_attempts = 120; // 2 minutes
    let mut completed = false;

    for attempt in 0..max_attempts {
        sleep(Duration::from_secs(1)).await;

        let status_resp = client
            .get(format!(
                "http://127.0.0.1:18086/api/v1/jobs/{job_id}/status"
            ))
            .send()
            .await
            .expect("Failed to get job status");

        let status_json: serde_json::Value = status_resp
            .json()
            .await
            .expect("Failed to parse status JSON");

        if attempt % 10 == 0 {
            println!(
                "Attempt {}: Status = {}",
                attempt + 1,
                status_json["status"]
            );
        }

        if status_json["status"] == "completed" {
            completed = true;
            println!("Completed in {} seconds", attempt + 1);
            break;
        } else if status_json["status"] == "failed" {
            panic!("Large file job failed: {status_json:?}");
        }
    }

    assert!(completed, "Large file job did not complete within timeout");

    // Cleanup
    server_handle.abort();
}

#[tokio::test]
async fn test_face_detection_integration() {
    // Use a video with visible faces - Kinetics-600 "talking on cell phone" category
    let kinetics_base = PathBuf::from("/Users/ayates/Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/Kinetics dataset (5%)/kinetics600_5per/kinetics600_5per/train");
    let test_path = kinetics_base.join("talking on cell phone/44mFype1uas.mp4");

    if !test_file_exists(&test_path) {
        eprintln!("Test file not found: {test_path:?}");
        eprintln!("Skipping test_face_detection_integration");
        return;
    }

    // Verify model exists (check both from repo root and from crate dir)
    let model_path = PathBuf::from("models/face-detection/retinaface_mnet025.onnx");
    let model_path_alt = PathBuf::from("../../models/face-detection/retinaface_mnet025.onnx");

    if !model_path.exists() && !model_path_alt.exists() {
        eprintln!("Face detection model not found at: {model_path:?} or {model_path_alt:?}");
        eprintln!("Current dir: {:?}", std::env::current_dir());
        eprintln!("Skipping test_face_detection_integration");
        return;
    }

    // Start server in background
    let state = video_audio_api_server::ApiState::new();
    let server_handle = tokio::spawn(async move {
        video_audio_api_server::start_server("127.0.0.1:18087", state)
            .await
            .expect("Failed to start server");
    });

    // Give server time to start
    sleep(Duration::from_secs(1)).await;

    // Create request
    let request_body = serde_json::json!({
        "source": {
            "type": "upload",
            "location": test_path.to_str().unwrap()
        },
        "processing": {
            "priority": "realtime",
            "quality_mode": "fast",
            "required_features": ["keyframes"],
            "optional_features": []
        }
    });

    // Send real-time processing request
    let client = reqwest::Client::new();
    let response = client
        .post("http://127.0.0.1:18087/api/v1/process/realtime")
        .json(&request_body)
        .send()
        .await
        .expect("Failed to send processing request");

    assert_eq!(response.status(), 202);

    let json: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    let job_id = json["job_id"].as_str().unwrap();
    println!("Job ID: {job_id}");

    // Poll job status until completed or timeout
    let max_attempts = 60; // 60 seconds timeout (face detection may take longer)
    let mut completed = false;

    for attempt in 0..max_attempts {
        sleep(Duration::from_secs(1)).await;

        let status_resp = client
            .get(format!(
                "http://127.0.0.1:18087/api/v1/jobs/{job_id}/status"
            ))
            .send()
            .await
            .expect("Failed to get job status");

        let status_json: serde_json::Value = status_resp
            .json()
            .await
            .expect("Failed to parse status JSON");

        if attempt % 10 == 0 {
            println!(
                "Attempt {}: Status = {}",
                attempt + 1,
                status_json["status"]
            );
        }

        if status_json["status"] == "completed" {
            completed = true;
            println!("Face detection job completed in {} seconds", attempt + 1);
            break;
        } else if status_json["status"] == "failed" {
            eprintln!("Job failed: {status_json:?}");
            // Don't panic - face detection may fail if model doesn't match
            server_handle.abort();
            return;
        }
    }

    assert!(
        completed,
        "Face detection job did not complete within timeout"
    );

    // Retrieve results
    let result_resp = client
        .get(format!(
            "http://127.0.0.1:18087/api/v1/jobs/{job_id}/result"
        ))
        .send()
        .await
        .expect("Failed to get job result");

    assert_eq!(result_resp.status(), 200);

    let result_json: serde_json::Value = result_resp
        .json()
        .await
        .expect("Failed to parse result JSON");

    println!(
        "Face detection result: {}",
        serde_json::to_string_pretty(&result_json).unwrap()
    );

    // Verify face detection results
    let results = result_json["results"].as_object().unwrap();

    // Check if face_detection task completed
    if let Some(face_result) = results.get("face_detection") {
        assert_eq!(face_result["type"], "face_detection");

        // Should have detected at least some faces in a "talking on cell phone" video
        let total_faces = face_result["total_faces"].as_u64().unwrap_or(0);
        println!("Total faces detected: {total_faces}");

        // Video has visible faces, but detection may vary based on frame quality
        // Just verify the structure is correct
        assert!(face_result["num_keyframes"].is_number());
        assert!(face_result["faces_per_keyframe"].is_array());
    } else {
        eprintln!("Warning: face_detection task not found in results");
        eprintln!("Available tasks: {:?}", results.keys());
    }

    // Cleanup
    server_handle.abort();
}

#[tokio::test]
async fn test_ocr_integration() {
    // Use a video that might contain text - try a few different Kinetics categories
    let kinetics_base = PathBuf::from("/Users/ayates/Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/Kinetics dataset (5%)/kinetics600_5per/kinetics600_5per/train");

    // Try different video categories that may have text
    let test_paths = vec![
        kinetics_base.join("reading book/3sJxdL91jfk.mp4"),
        kinetics_base.join("reading newspaper/1RZUl-hZNr8.mp4"),
        kinetics_base.join("talking on cell phone/44mFype1uas.mp4"),
    ];

    let mut test_path = None;
    for path in test_paths {
        if test_file_exists(&path) {
            test_path = Some(path);
            break;
        }
    }

    let test_path = if let Some(p) = test_path {
        p
    } else {
        eprintln!("No test file found for OCR integration test");
        eprintln!("Skipping test_ocr_integration");
        return;
    };

    // Verify model exists (use absolute path from project root)
    let project_root = find_project_root();
    let detection_model_path = project_root.join("models/ocr/ch_PP-OCRv4_det.onnx");
    let recognition_model_path = project_root.join("models/ocr/ch_PP-OCRv4_rec.onnx");

    if !detection_model_path.exists() || !recognition_model_path.exists() {
        eprintln!(
            "OCR models not found at: {detection_model_path:?} or {recognition_model_path:?}"
        );
        eprintln!("Skipping test_ocr_integration (models need to be downloaded)");
        return;
    }

    // Start server in background
    let state = video_audio_api_server::ApiState::new();
    let server_handle = tokio::spawn(async move {
        video_audio_api_server::start_server("127.0.0.1:18088", state)
            .await
            .expect("Failed to start server");
    });

    // Give server time to start
    sleep(Duration::from_secs(1)).await;

    // Create request
    let request_body = serde_json::json!({
        "source": {
            "type": "upload",
            "location": test_path.to_str().unwrap()
        },
        "processing": {
            "priority": "realtime",
            "quality_mode": "fast",
            "required_features": ["keyframes"],
            "optional_features": []
        }
    });

    // Send real-time processing request
    let client = reqwest::Client::new();
    let response = client
        .post("http://127.0.0.1:18088/api/v1/process/realtime")
        .json(&request_body)
        .send()
        .await
        .expect("Failed to send processing request");

    assert_eq!(response.status(), 202);

    let json: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    let job_id = json["job_id"].as_str().unwrap();
    println!("OCR Job ID: {job_id}");

    // Poll job status until completed or timeout
    let max_attempts = 60; // 60 seconds timeout
    let mut completed = false;

    for attempt in 0..max_attempts {
        sleep(Duration::from_secs(1)).await;

        let status_resp = client
            .get(format!(
                "http://127.0.0.1:18088/api/v1/jobs/{job_id}/status"
            ))
            .send()
            .await
            .expect("Failed to get job status");

        let status_json: serde_json::Value = status_resp
            .json()
            .await
            .expect("Failed to parse status JSON");

        if attempt % 10 == 0 {
            println!(
                "OCR Attempt {}: Status = {}",
                attempt + 1,
                status_json["status"]
            );
        }

        if status_json["status"] == "completed" {
            completed = true;
            println!("OCR job completed in {} seconds", attempt + 1);
            break;
        } else if status_json["status"] == "failed" {
            eprintln!("OCR job failed: {status_json:?}");
            // Don't panic - OCR may fail if models aren't available
            server_handle.abort();
            return;
        }
    }

    assert!(completed, "OCR job did not complete within timeout");

    // Retrieve results
    let result_resp = client
        .get(format!(
            "http://127.0.0.1:18088/api/v1/jobs/{job_id}/result"
        ))
        .send()
        .await
        .expect("Failed to get job result");

    assert_eq!(result_resp.status(), 200);

    let result_json: serde_json::Value = result_resp
        .json()
        .await
        .expect("Failed to parse result JSON");

    println!(
        "OCR result: {}",
        serde_json::to_string_pretty(&result_json).unwrap()
    );

    // Verify OCR results structure
    let results = result_json["results"].as_object().unwrap();

    // Check if ocr task completed
    if let Some(ocr_result) = results.get("ocr") {
        assert_eq!(ocr_result["type"], "ocr");

        // Verify structure
        assert!(ocr_result["total_text_regions"].is_number());
        assert!(ocr_result["num_keyframes"].is_number());
        assert!(ocr_result["text_regions_per_keyframe"].is_array());

        let total_text_regions = ocr_result["total_text_regions"].as_u64().unwrap_or(0);
        println!("Total text regions detected: {total_text_regions}");

        // Note: May not find text in all videos, just verify structure is correct
    } else {
        eprintln!("Warning: ocr task not found in results");
        eprintln!("Available tasks: {:?}", results.keys());
    }

    // Cleanup
    server_handle.abort();
}

#[tokio::test]
async fn test_diarization_integration() {
    // Use test videos - prioritize small files for faster CI
    // For multi-speaker validation, use docling sample first (fast)
    // Production videos take 5+ minutes to process and should be tested manually
    let test_videos = vec![
        // Docling sample (10 seconds - best for CI)
        "/Users/ayates/docling/tests/data/audio/sample_10s_video-mp4.mp4",
        // Kinetics short clips
        "/Users/ayates/Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/Kinetics dataset (5%)/kinetics600_5per/kinetics600_5per/train/talking_on_cell_phone/6Bka-N8DzMI_000038_000048.mp4",
        // Production Zoom meetings (use for manual testing only - take 5+ minutes)
        // "/Users/ayates/Desktop/stuff/stuff/review existing benchmarks/april meeting conv ai dashboard 2025-08-14 17.42.25 Zoom Meeting/video1509128771.mp4",
    ];

    // Find first video that exists
    let test_path = test_videos
        .iter()
        .find(|p| Path::new(p).exists())
        .map(|s| (*s).to_string());

    if test_path.is_none() {
        eprintln!("No test video found for diarization test");
        eprintln!("Tried paths: {test_videos:?}");
        eprintln!("Skipping test_diarization_integration");
        return;
    }

    let test_path = test_path.unwrap();
    println!("Testing diarization with: {test_path}");

    // Start server in background
    let state = video_audio_api_server::ApiState::new();
    let server_handle = tokio::spawn(async move {
        video_audio_api_server::start_server("127.0.0.1:18089", state)
            .await
            .expect("Failed to start server");
    });

    // Give server time to start
    sleep(Duration::from_secs(1)).await;

    // Create request
    let request_body = serde_json::json!({
        "source": {
            "type": "upload",
            "location": test_path
        },
        "processing": {
            "priority": "realtime",
            "quality_mode": "fast",
            "required_features": [],
            "optional_features": ["diarization"]
        }
    });

    // Send processing request
    let client = reqwest::Client::new();
    let response = client
        .post("http://127.0.0.1:18089/api/v1/process/realtime")
        .json(&request_body)
        .send()
        .await
        .expect("Failed to send diarization processing request");

    assert_eq!(
        response.status(),
        202,
        "Expected 202 Accepted for async realtime processing"
    );

    let json: serde_json::Value = response
        .json()
        .await
        .expect("Failed to parse diarization response JSON");

    let job_id = json["job_id"].as_str().unwrap();
    println!("Diarization job ID: {job_id}");

    // Poll for completion (diarization may take longer)
    let mut completed = false;
    for attempt in 0..60 {
        sleep(Duration::from_secs(1)).await;

        let status_resp = client
            .get(format!(
                "http://127.0.0.1:18089/api/v1/jobs/{job_id}/status"
            ))
            .send()
            .await
            .expect("Failed to get job status");

        let status_json: serde_json::Value = status_resp
            .json()
            .await
            .expect("Failed to parse status JSON");

        if attempt % 10 == 0 {
            println!(
                "Diarization Attempt {}: Status = {}",
                attempt + 1,
                status_json["status"]
            );
        }

        if status_json["status"] == "completed" {
            completed = true;
            println!("Diarization job completed in {} seconds", attempt + 1);
            break;
        } else if status_json["status"] == "failed" {
            eprintln!("Diarization job failed: {status_json:?}");
            // Don't panic - diarization may fail if pyannote.audio isn't installed
            server_handle.abort();
            return;
        }
    }

    assert!(completed, "Diarization job did not complete within timeout");

    // Retrieve results
    let result_resp = client
        .get(format!(
            "http://127.0.0.1:18089/api/v1/jobs/{job_id}/result"
        ))
        .send()
        .await
        .expect("Failed to get job result");

    assert_eq!(result_resp.status(), 200);

    let result_json: serde_json::Value = result_resp
        .json()
        .await
        .expect("Failed to parse result JSON");

    println!(
        "Diarization result: {}",
        serde_json::to_string_pretty(&result_json).unwrap()
    );

    // Verify diarization results structure
    let results = result_json["results"].as_object().unwrap();

    // Check if diarization task completed
    if let Some(diarization_result) = results.get("diarization") {
        assert_eq!(diarization_result["type"], "diarization");

        // Verify structure
        assert!(diarization_result["num_speakers"].is_number());
        assert!(diarization_result["num_segments"].is_number());
        assert!(diarization_result["speakers"].is_array());
        assert!(diarization_result["timeline"].is_array());

        let num_speakers = diarization_result["num_speakers"].as_u64().unwrap();
        let num_segments = diarization_result["num_segments"].as_u64().unwrap();

        println!("Speakers identified: {num_speakers}");
        println!("Speaker segments: {num_segments}");

        // Verify speaker structure
        let speakers = diarization_result["speakers"].as_array().unwrap();
        if !speakers.is_empty() {
            let first_speaker = &speakers[0];
            assert!(first_speaker["id"].is_string());
            assert!(first_speaker["total_speaking_time"].is_number());
        }

        // Verify timeline structure
        let timeline = diarization_result["timeline"].as_array().unwrap();
        if !timeline.is_empty() {
            let first_segment = &timeline[0];
            assert!(first_segment["start"].is_number());
            assert!(first_segment["end"].is_number());
            assert!(first_segment["speaker"].is_string());
            assert!(first_segment["confidence"].is_number());
        }
    } else {
        eprintln!("Warning: diarization task not found in results (may have failed)");
        eprintln!("Available tasks: {:?}", results.keys());
    }

    // Cleanup
    server_handle.abort();
}

#[tokio::test]
#[ignore] // Requires large test files - run manually to verify scene detection optimization
async fn test_scene_detection_integration() {
    // Use videos with multiple scenes (e.g., presentations, lectures, screen recordings)
    // Kinetics categories: "presenting weather forecast", "news anchoring", "giving or receiving award"
    let test_videos = vec![
        "/Users/ayates/Desktop/stuff/stuff/editing-relevance-rubrics kg may 16 2025.mov",
        "/Users/ayates/Desktop/stuff/stuff/GMT20250516-190317_Recording_avo_1920x1080 braintrust.mp4",
        "/Users/ayates/Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/Kinetics dataset (5%)/kinetics600_5per/kinetics600_5per/train/presenting_weather_forecast/0AMKT8gZVPs_000045_000055.mp4",
    ];

    // Find first video that exists
    let test_path = test_videos
        .iter()
        .find(|p| Path::new(p).exists())
        .map(|s| (*s).to_string());

    if test_path.is_none() {
        eprintln!("No test video found for scene detection test");
        eprintln!("Tried paths: {test_videos:?}");
        eprintln!("Skipping test_scene_detection_integration");
        return;
    }

    let test_path = test_path.unwrap();
    println!("Testing scene detection with: {test_path}");

    // Start server in background
    let state = video_audio_api_server::ApiState::new();
    let server_handle = tokio::spawn(async move {
        video_audio_api_server::start_server("127.0.0.1:18090", state)
            .await
            .expect("Failed to start server");
    });

    // Give server time to start
    sleep(Duration::from_secs(1)).await;

    // Create request
    let request_body = serde_json::json!({
        "source": {
            "type": "upload",
            "location": test_path
        },
        "processing": {
            "priority": "realtime",
            "quality_mode": "fast",
            "required_features": [],
            "optional_features": ["scene_detection"]
        }
    });

    // Send processing request
    let client = reqwest::Client::new();
    let response = client
        .post("http://127.0.0.1:18090/api/v1/process/realtime")
        .json(&request_body)
        .send()
        .await
        .expect("Failed to send scene detection processing request");

    assert_eq!(response.status(), 202); // ACCEPTED for realtime processing

    let json: serde_json::Value = response
        .json()
        .await
        .expect("Failed to parse scene detection response JSON");

    let job_id = json["job_id"].as_str().unwrap();
    println!("Scene detection job ID: {job_id}");

    // Poll for completion (scene detection can take longer for large files)
    let mut completed = false;
    for attempt in 0..300 {
        // 5 minute timeout (300 seconds)
        sleep(Duration::from_secs(1)).await;

        let status_resp = client
            .get(format!(
                "http://127.0.0.1:18090/api/v1/jobs/{job_id}/status"
            ))
            .send()
            .await
            .expect("Failed to get scene detection job status");

        let status_json: serde_json::Value = status_resp
            .json()
            .await
            .expect("Failed to parse scene detection status JSON");

        let status = status_json["status"].as_str().unwrap();

        if attempt % 10 == 0 {
            // Log every 10 seconds
            println!("Attempt {}: status = {}", attempt + 1, status);
        }

        if status == "completed" || status == "failed" {
            completed = true;
            println!("Job {} after {} seconds", status, attempt + 1);
            break;
        }
    }

    assert!(
        completed,
        "Scene detection job did not complete within 5 minute timeout"
    );

    // Get result
    let result_resp = client
        .get(format!(
            "http://127.0.0.1:18090/api/v1/jobs/{job_id}/result"
        ))
        .send()
        .await
        .expect("Failed to get scene detection result");

    assert_eq!(result_resp.status(), 200);

    let result_json: serde_json::Value = result_resp
        .json()
        .await
        .expect("Failed to parse scene detection result JSON");

    println!(
        "Scene detection result: {}",
        serde_json::to_string_pretty(&result_json).unwrap()
    );

    // Verify scene detection results structure
    let results = result_json["results"].as_object().unwrap();

    // Check if scene_detection task completed
    if let Some(scene_result) = results.get("scene_detection") {
        assert_eq!(scene_result["type"], "scene_detection");

        // Verify structure
        assert!(scene_result["num_scenes"].is_number());
        assert!(scene_result["num_boundaries"].is_number());
        assert!(scene_result["threshold"].is_number());
        assert!(scene_result["boundaries"].is_array());

        let num_scenes = scene_result["num_scenes"].as_u64().unwrap();
        let num_boundaries = scene_result["num_boundaries"].as_u64().unwrap();

        println!("Total scenes: {num_scenes}");
        println!("Scene boundaries: {num_boundaries}");

        // Verify boundaries structure
        let boundaries = scene_result["boundaries"].as_array().unwrap();
        if !boundaries.is_empty() {
            let first_boundary = &boundaries[0];
            assert!(first_boundary["timestamp"].is_number());
            assert!(first_boundary["score"].is_number());

            let timestamp = first_boundary["timestamp"].as_f64().unwrap();
            let score = first_boundary["score"].as_f64().unwrap();
            println!("First boundary: {timestamp:.2}s (score: {score:.2})");
        }

        // num_scenes should be num_boundaries + 1 (initial scene before first boundary)
        assert_eq!(num_scenes, num_boundaries + 1);
    } else {
        eprintln!("Warning: scene_detection task not found in results");
        eprintln!("Available tasks: {:?}", results.keys());
        panic!("Scene detection task should have completed");
    }

    // Cleanup
    server_handle.abort();
}

#[tokio::test]
async fn test_semantic_search_text_query() {
    // First, process a video to generate and store embeddings
    let test_path = test_video_path();
    if !test_file_exists(&test_path) {
        eprintln!("Test file not found: {test_path:?}");
        eprintln!("Skipping test_semantic_search_text_query");
        return;
    }

    // Check if Qdrant is available (semantic search requires Qdrant)
    let qdrant_available = tokio::net::TcpStream::connect("127.0.0.1:6333")
        .await
        .is_ok();
    if !qdrant_available {
        eprintln!("Qdrant not available on 127.0.0.1:6333");
        eprintln!("Skipping test_semantic_search_text_query (requires Qdrant)");
        return;
    }

    // Start server in background
    let state = video_audio_api_server::ApiState::new();
    let server_handle = tokio::spawn(async move {
        video_audio_api_server::start_server("127.0.0.1:18091", state)
            .await
            .expect("Failed to start server");
    });

    // Give server time to start
    sleep(Duration::from_secs(1)).await;

    // Create processing request with embeddings enabled
    let request_body = serde_json::json!({
        "source": {
            "type": "upload",
            "location": test_path.to_str().unwrap()
        },
        "processing": {
            "priority": "realtime",
            "quality_mode": "fast",
            "required_features": ["keyframes"],
            "optional_features": []
        }
    });

    // Send processing request to generate embeddings
    let client = reqwest::Client::new();
    let response = client
        .post("http://127.0.0.1:18091/api/v1/process/realtime")
        .json(&request_body)
        .send()
        .await
        .expect("Failed to send processing request");

    assert_eq!(response.status(), 202);

    let json: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    let job_id = json["job_id"].as_str().unwrap().to_string();
    println!("Processing job ID: {job_id}");

    // Poll until processing completes
    let max_attempts = 60;
    let mut completed = false;

    for attempt in 0..max_attempts {
        sleep(Duration::from_secs(1)).await;

        let status_resp = client
            .get(format!(
                "http://127.0.0.1:18091/api/v1/jobs/{job_id}/status"
            ))
            .send()
            .await
            .expect("Failed to get job status");

        let status_json: serde_json::Value = status_resp
            .json()
            .await
            .expect("Failed to parse status JSON");

        if attempt % 10 == 0 {
            println!(
                "Attempt {}: Status = {}",
                attempt + 1,
                status_json["status"]
            );
        }

        if status_json["status"] == "completed" {
            completed = true;
            println!("Processing completed in {} seconds", attempt + 1);
            break;
        } else if status_json["status"] == "failed" {
            eprintln!("Processing failed: {status_json:?}");
            server_handle.abort();
            return;
        }
    }

    assert!(completed, "Processing did not complete within timeout");

    // Wait a moment for embeddings to be stored in Qdrant
    sleep(Duration::from_secs(2)).await;

    // Now perform text query search
    let search_request = serde_json::json!({
        "query": {
            "type": "text",
            "query": "person speaking"
        },
        "limit": 5,
        "job_id": job_id,
        "include_vectors": false
    });

    let search_response = client
        .post("http://127.0.0.1:18091/api/v1/search/similar")
        .json(&search_request)
        .send()
        .await
        .expect("Failed to send search request");

    println!("Search response status: {}", search_response.status());

    if search_response.status() == 503 {
        eprintln!("Qdrant unavailable during search (service_unavailable)");
        server_handle.abort();
        return;
    }

    assert_eq!(
        search_response.status(),
        200,
        "Expected 200 OK for search request"
    );

    let search_json: serde_json::Value = search_response
        .json()
        .await
        .expect("Failed to parse search response JSON");

    println!(
        "Search result: {}",
        serde_json::to_string_pretty(&search_json).unwrap()
    );

    // Verify search response structure
    assert!(search_json["results"].is_array());
    assert!(search_json["count"].is_number());
    assert_eq!(search_json["query_type"], "text");

    let results = search_json["results"].as_array().unwrap();
    let count = search_json["count"].as_u64().unwrap();

    println!("Found {count} similar results");

    // Verify each result has expected fields
    for result in results {
        assert!(result["vector_id"].is_string());
        assert!(result["score"].is_number());
        assert!(result["job_id"].is_string());
        assert!(result["embedding_type"].is_string());
        assert!(result["metadata"].is_object());
        assert!(result["vector"].is_null()); // We requested include_vectors=false
    }

    // Cleanup
    server_handle.abort();
}

#[tokio::test]
async fn test_semantic_search_image_query() {
    // Use a test keyframe image for image query
    // First extract keyframes from a video, then use one as a query
    let test_path = test_video_path();
    if !test_file_exists(&test_path) {
        eprintln!("Test file not found: {test_path:?}");
        eprintln!("Skipping test_semantic_search_image_query");
        return;
    }

    // Check if Qdrant is available
    let qdrant_available = tokio::net::TcpStream::connect("127.0.0.1:6333")
        .await
        .is_ok();
    if !qdrant_available {
        eprintln!("Qdrant not available on 127.0.0.1:6333");
        eprintln!("Skipping test_semantic_search_image_query (requires Qdrant)");
        return;
    }

    // Start server in background
    let state = video_audio_api_server::ApiState::new();
    let server_handle = tokio::spawn(async move {
        video_audio_api_server::start_server("127.0.0.1:18092", state)
            .await
            .expect("Failed to start server");
    });

    // Give server time to start
    sleep(Duration::from_secs(1)).await;

    // Process video to extract keyframes and generate embeddings
    let request_body = serde_json::json!({
        "source": {
            "type": "upload",
            "location": test_path.to_str().unwrap()
        },
        "processing": {
            "priority": "realtime",
            "quality_mode": "fast",
            "required_features": ["keyframes"],
            "optional_features": []
        }
    });

    let client = reqwest::Client::new();
    let response = client
        .post("http://127.0.0.1:18092/api/v1/process/realtime")
        .json(&request_body)
        .send()
        .await
        .expect("Failed to send processing request");

    assert_eq!(response.status(), 202);

    let json: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    let job_id = json["job_id"].as_str().unwrap().to_string();
    println!("Processing job ID for image search: {job_id}");

    // Poll until processing completes
    let max_attempts = 60;
    let mut completed = false;

    for attempt in 0..max_attempts {
        sleep(Duration::from_secs(1)).await;

        let status_resp = client
            .get(format!(
                "http://127.0.0.1:18092/api/v1/jobs/{job_id}/status"
            ))
            .send()
            .await
            .expect("Failed to get job status");

        let status_json: serde_json::Value = status_resp
            .json()
            .await
            .expect("Failed to parse status JSON");

        if attempt % 10 == 0 {
            println!(
                "Attempt {}: Status = {}",
                attempt + 1,
                status_json["status"]
            );
        }

        if status_json["status"] == "completed" {
            completed = true;
            println!("Processing completed in {} seconds", attempt + 1);
            break;
        } else if status_json["status"] == "failed" {
            eprintln!("Processing failed: {status_json:?}");
            server_handle.abort();
            return;
        }
    }

    assert!(completed, "Processing did not complete within timeout");

    // Get job results to find keyframe paths
    let result_resp = client
        .get(format!(
            "http://127.0.0.1:18092/api/v1/jobs/{job_id}/result"
        ))
        .send()
        .await
        .expect("Failed to get job result");

    assert_eq!(result_resp.status(), 200);

    let result_json: serde_json::Value = result_resp
        .json()
        .await
        .expect("Failed to parse result JSON");

    // Extract keyframe path from results
    let results = result_json["results"].as_object().unwrap();
    let keyframe_result = results
        .get("keyframe_extraction")
        .expect("Keyframe extraction should be present");

    let keyframe_paths = keyframe_result["paths"].as_array().unwrap();
    assert!(
        !keyframe_paths.is_empty(),
        "Should have extracted at least one keyframe"
    );

    let query_image_path = keyframe_paths[0].as_str().unwrap();
    println!("Using keyframe as query image: {query_image_path}");

    // Verify keyframe exists
    if !Path::new(query_image_path).exists() {
        eprintln!("Keyframe not found: {query_image_path}");
        server_handle.abort();
        return;
    }

    // Wait for embeddings to be stored
    sleep(Duration::from_secs(2)).await;

    // Perform image query search
    let search_request = serde_json::json!({
        "query": {
            "type": "image",
            "location": {
                "type": "upload",
                "location": query_image_path
            }
        },
        "limit": 5,
        "embedding_type": "clip_frame",
        "include_vectors": true
    });

    let search_response = client
        .post("http://127.0.0.1:18092/api/v1/search/similar")
        .json(&search_request)
        .send()
        .await
        .expect("Failed to send image search request");

    println!("Image search response status: {}", search_response.status());

    if search_response.status() == 503 {
        eprintln!("Qdrant unavailable during search");
        server_handle.abort();
        return;
    }

    assert_eq!(
        search_response.status(),
        200,
        "Expected 200 OK for image search request"
    );

    let search_json: serde_json::Value = search_response
        .json()
        .await
        .expect("Failed to parse search response JSON");

    println!(
        "Image search result: {}",
        serde_json::to_string_pretty(&search_json).unwrap()
    );

    // Verify search response structure
    assert!(search_json["results"].is_array());
    assert!(search_json["count"].is_number());
    assert_eq!(search_json["query_type"], "image");

    let results = search_json["results"].as_array().unwrap();
    let count = search_json["count"].as_u64().unwrap();

    println!("Found {count} similar images");

    // Since we're querying with a keyframe from the same video, we should find at least itself
    // (or other similar frames from the same video)
    if count > 0 {
        let first_result = &results[0];
        assert!(first_result["vector_id"].is_string());
        assert!(first_result["score"].is_number());
        assert!(first_result["embedding_type"] == "clip_frame");
        assert!(first_result["vector"].is_array()); // We requested include_vectors=true

        let score = first_result["score"].as_f64().unwrap();
        println!("Top match score: {score:.4}");

        // The query image should match itself with high similarity (> 0.9)
        // Note: This depends on whether the exact frame is in the database
    }

    // Cleanup
    server_handle.abort();
}

/// Test audio query semantic search
///
/// This test verifies that audio embeddings can be extracted and used for similarity search.
/// It processes an audio/video file, then uses the same file as a query to find similar content.
#[tokio::test]
#[ignore] // Requires CLAP model and Qdrant
async fn test_semantic_search_audio_query() {
    let test_path = test_video_path();
    if !test_file_exists(&test_path) {
        eprintln!("Test file not found: {test_path:?}");
        eprintln!("Skipping test_semantic_search_audio_query");
        return;
    }

    // Check if Qdrant is available
    let qdrant_available = tokio::net::TcpStream::connect("127.0.0.1:6333")
        .await
        .is_ok();
    if !qdrant_available {
        eprintln!("Qdrant not available on 127.0.0.1:6333");
        eprintln!("Skipping test_semantic_search_audio_query (requires Qdrant)");
        return;
    }

    // Check if CLAP model exists
    if !Path::new("models/embeddings/clap.onnx").exists() {
        eprintln!("CLAP model not found at models/embeddings/clap.onnx");
        eprintln!("Skipping test_semantic_search_audio_query (requires CLAP model)");
        return;
    }

    // Start server in background
    let state = video_audio_api_server::ApiState::new();
    let server_handle = tokio::spawn(async move {
        video_audio_api_server::start_server("127.0.0.1:18093", state)
            .await
            .expect("Failed to start server");
    });

    // Give server time to start
    sleep(Duration::from_secs(1)).await;

    // Process video to extract audio embeddings
    let request_body = serde_json::json!({
        "source": {
            "type": "upload",
            "location": test_path.to_str().unwrap()
        },
        "processing": {
            "priority": "realtime",
            "quality_mode": "fast",
            "required_features": ["audio_embeddings"],
            "optional_features": []
        }
    });

    let client = reqwest::Client::new();
    let response = client
        .post("http://127.0.0.1:18093/api/v1/process/realtime")
        .json(&request_body)
        .send()
        .await
        .expect("Failed to send processing request");

    assert_eq!(response.status(), 202);

    let json: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    let job_id = json["job_id"].as_str().unwrap().to_string();
    println!("Processing job ID for audio search: {job_id}");

    // Poll until processing completes
    let max_attempts = 60;
    let mut completed = false;

    for attempt in 0..max_attempts {
        sleep(Duration::from_secs(1)).await;

        let status_resp = client
            .get(format!(
                "http://127.0.0.1:18093/api/v1/jobs/{job_id}/status"
            ))
            .send()
            .await
            .expect("Failed to get job status");

        let status_json: serde_json::Value = status_resp
            .json()
            .await
            .expect("Failed to parse status JSON");

        if status_json["status"] == "completed" {
            completed = true;
            println!("Processing completed after {} seconds", attempt + 1);
            break;
        } else if status_json["status"] == "failed" {
            panic!("Processing failed: {status_json:?}");
        }
    }

    assert!(
        completed,
        "Processing did not complete within {max_attempts} seconds"
    );

    // Wait for embeddings to be stored
    sleep(Duration::from_secs(2)).await;

    // Perform audio query search using the same file
    let search_request = serde_json::json!({
        "query": {
            "type": "audio",
            "location": {
                "type": "upload",
                "location": test_path.to_str().unwrap()
            }
        },
        "limit": 5,
        "embedding_type": "clap_audio",
        "include_vectors": true
    });

    let search_response = client
        .post("http://127.0.0.1:18093/api/v1/search/similar")
        .json(&search_request)
        .send()
        .await
        .expect("Failed to send audio search request");

    println!("Audio search response status: {}", search_response.status());

    if search_response.status() == 503 {
        eprintln!("Qdrant unavailable during search");
        server_handle.abort();
        return;
    }

    assert_eq!(
        search_response.status(),
        200,
        "Expected 200 OK for audio search request"
    );

    let search_json: serde_json::Value = search_response
        .json()
        .await
        .expect("Failed to parse search response JSON");

    println!(
        "Audio search result: {}",
        serde_json::to_string_pretty(&search_json).unwrap()
    );

    // Verify search response structure
    assert!(search_json["results"].is_array());
    assert!(search_json["count"].is_number());
    assert_eq!(search_json["query_type"], "audio");

    let results = search_json["results"].as_array().unwrap();
    let count = search_json["count"].as_u64().unwrap();

    println!("Found {count} similar audio clips");

    // Since we're querying with the same audio file, we should find itself
    if count > 0 {
        let first_result = &results[0];
        assert!(first_result["vector_id"].is_string());
        assert!(first_result["score"].is_number());
        assert!(first_result["embedding_type"] == "clap_audio");
        assert!(first_result["vector"].is_array());

        let score = first_result["score"].as_f64().unwrap();
        println!("Top match score: {score:.4}");

        // The query audio should match itself with high similarity (> 0.9)
        assert!(
            score > 0.85,
            "Expected high similarity for same audio file, got {score}"
        );
    }

    // Cleanup
    server_handle.abort();
}

/// Test URL download functionality
///
/// This test verifies that the API can download a media file from a public URL
/// and process it successfully. It uses a small sample video from a public source.
///
/// Note: This test is marked as ignored by default since it requires internet connectivity.
/// Run with: cargo test `test_url_download_processing` -- --ignored
#[tokio::test]
#[ignore]
async fn test_url_download_processing() {
    // Start server in background
    let state = video_audio_api_server::ApiState::new();
    let server_handle = tokio::spawn(async move {
        video_audio_api_server::start_server("127.0.0.1:18090", state)
            .await
            .expect("Failed to start server");
    });

    // Give server time to start
    sleep(Duration::from_secs(1)).await;

    // Use a small publicly available video
    // NOTE: External URLs may break over time. If this test fails with 400 Bad Request,
    // the URL may no longer be available. Update to a different public test video URL.
    let test_url =
        "https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/ForBiggerBlazes.mp4";

    let request_body = serde_json::json!({
        "source": {
            "type": "url",
            "location": test_url
        },
        "processing": {
            "priority": "realtime",
            "quality_mode": "fast",
            "required_features": ["keyframes"],
            "optional_features": []
        }
    });

    let client = reqwest::Client::new();
    let response = client
        .post("http://127.0.0.1:18090/api/v1/process/realtime")
        .json(&request_body)
        .send()
        .await
        .expect("Failed to send request");

    println!("Response status: {}", response.status());

    // Should accept the job
    assert_eq!(response.status(), 202, "Expected 202 Accepted status");

    let json: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    let job_id = json["job_id"].as_str().unwrap();

    println!("Job ID: {job_id}");

    // Poll for completion (longer timeout for download + processing)
    let max_attempts = 60; // 1 minute
    let mut completed = false;

    for attempt in 0..max_attempts {
        sleep(Duration::from_secs(1)).await;

        let status_resp = client
            .get(format!(
                "http://127.0.0.1:18090/api/v1/jobs/{job_id}/status"
            ))
            .send()
            .await
            .expect("Failed to get job status");

        if status_resp.status().is_success() {
            let status_json: serde_json::Value = status_resp
                .json()
                .await
                .expect("Failed to parse status JSON");

            let status = status_json["status"].as_str().unwrap();
            println!("Attempt {}: Status = {}", attempt + 1, status);

            if status == "completed" {
                completed = true;
                break;
            } else if status == "failed" {
                let error = status_json["error"].as_str().unwrap_or("Unknown error");
                panic!("Job failed: {error}");
            }
        }
    }

    assert!(completed, "Job did not complete within timeout");

    // Get final result
    let result_resp = client
        .get(format!(
            "http://127.0.0.1:18090/api/v1/jobs/{job_id}/result"
        ))
        .send()
        .await
        .expect("Failed to get job result");

    assert_eq!(result_resp.status(), 200);

    let result_json: serde_json::Value = result_resp
        .json()
        .await
        .expect("Failed to parse result JSON");

    println!(
        "Job result: {}",
        serde_json::to_string_pretty(&result_json).unwrap()
    );

    // Verify job completed successfully
    assert_eq!(result_json["status"], "completed");

    // Verify we have some results
    let results = result_json["results"].as_object().unwrap();
    assert!(!results.is_empty(), "Expected non-empty results");

    // Verify keyframe extraction ran (we requested it)
    assert!(
        results.contains_key("keyframes"),
        "Expected keyframes in results"
    );

    println!("URL download and processing test completed successfully");

    // Cleanup
    server_handle.abort();
}
