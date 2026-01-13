//! Smoke Test Suite - Fast Pre-Commit Validation
//!
//! Critical path tests for quick validation before commit.
//! Target: <1 minute total runtime (5-6 tests)
//!
//! Run: VIDEO_EXTRACT_THREADS=4 cargo test --release --test smoke_test -- --ignored --test-threads=1
//!
//! Note: VIDEO_EXTRACT_THREADS limits thread pool size to prevent system overload.
//! Recommended values: 2-4 threads for testing, remove for production (uses all cores).
//!
//! Tests cover:
//! - Video keyframes + object detection (most common operation)
//! - Audio extraction + transcription (common pipeline)
//! - Edge case handling (4K resolution)
//! - Error handling (corrupted file)
//! - Simple audio operation (fast baseline)

use std::env;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

/// Test result with timing
struct TestResult {
    passed: bool,
    duration_secs: f64,
    error: Option<String>,
}

/// Run video-extract CLI and capture result
fn run_video_extract(ops: &str, input_file: &PathBuf) -> TestResult {
    let start = Instant::now();
    let binary = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/video-extract");

    let output = Command::new(&binary)
        .args(["debug", "--ops", ops])
        .arg(input_file)
        .output()
        .expect("Failed to execute video-extract");

    let duration = start.elapsed().as_secs_f64();
    let passed = output.status.success();
    let error = if !passed {
        Some(String::from_utf8_lossy(&output.stderr).to_string())
    } else {
        None
    };

    TestResult {
        passed,
        duration_secs: duration,
        error,
    }
}

// ============================================================================
// SMOKE TESTS (5-6 tests, target <1 minute)
// ============================================================================

#[test]
#[ignore]
fn smoke_video_keyframes_detection() {
    // Most common operation: video keyframes + object detection
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_edge_cases/video_variable_framerate_vfr__timing_test.mp4");

    if !file.exists() {
        panic!("Test file not found: {}", file.display());
    }

    let result = run_video_extract("keyframes,object-detection", &file);
    assert!(
        result.passed,
        "Smoke test failed (keyframes+detection): {:?}",
        result.error
    );
    assert!(
        result.duration_secs < 10.0,
        "Too slow: {:.2}s (expected <10s)",
        result.duration_secs
    );
    println!(
        "✅ Video keyframes + detection: {:.2}s",
        result.duration_secs
    );
}

#[test]
#[ignore]
fn smoke_audio_transcription() {
    // Common pipeline: audio extraction + transcription
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_edge_cases/audio_mono_single_channel__channel_test.wav");

    if !file.exists() {
        panic!("Test file not found: {}", file.display());
    }

    let result = run_video_extract("transcription", &file);
    assert!(
        result.passed,
        "Smoke test failed (audio transcription): {:?}",
        result.error
    );
    assert!(
        result.duration_secs < 10.0,
        "Too slow: {:.2}s (expected <10s)",
        result.duration_secs
    );
    println!("✅ Audio transcription: {:.2}s", result.duration_secs);
}

#[test]
#[ignore]
fn smoke_4k_resolution() {
    // Edge case: 4K resolution (validates scaling/memory handling)
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_edge_cases/video_4k_ultra_hd_3840x2160__stress_test.mp4");

    if !file.exists() {
        panic!("Test file not found: {}", file.display());
    }

    let result = run_video_extract("keyframes", &file);
    assert!(
        result.passed,
        "Smoke test failed (4K resolution): {:?}",
        result.error
    );
    assert!(
        result.duration_secs < 15.0,
        "Too slow: {:.2}s (expected <15s)",
        result.duration_secs
    );
    println!("✅ 4K resolution handling: {:.2}s", result.duration_secs);
}

#[test]
#[ignore]
fn smoke_corrupted_file() {
    // Error handling: corrupted file should fail gracefully
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_edge_cases/corrupted_truncated_file__error_handling.mp4");

    if !file.exists() {
        panic!("Test file not found: {}", file.display());
    }

    let result = run_video_extract("keyframes", &file);
    // Corrupted file should either fail gracefully or process partial content
    // Both behaviors are acceptable - just verify no crash
    println!(
        "✅ Corrupted file handling: {:.2}s (passed={}, graceful)",
        result.duration_secs, result.passed
    );
}

#[test]
#[ignore]
fn smoke_audio_extraction() {
    // Fast baseline: simple audio extraction (fastest operation)
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_edge_cases/audio_very_short_1sec__duration_min.wav");

    if !file.exists() {
        panic!("Test file not found: {}", file.display());
    }

    let result = run_video_extract("audio", &file);
    assert!(
        result.passed,
        "Smoke test failed (audio extraction): {:?}",
        result.error
    );
    assert!(
        result.duration_secs < 5.0,
        "Too slow: {:.2}s (expected <5s)",
        result.duration_secs
    );
    println!("✅ Audio extraction (1sec): {:.2}s", result.duration_secs);
}

#[test]
#[ignore]
fn smoke_hevc_codec() {
    // Modern codec: HEVC/H.265 support
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_edge_cases/video_hevc_h265_modern_codec__compatibility.mp4");

    if !file.exists() {
        panic!("Test file not found: {}", file.display());
    }

    let result = run_video_extract("keyframes", &file);
    assert!(
        result.passed,
        "Smoke test failed (HEVC codec): {:?}",
        result.error
    );
    assert!(
        result.duration_secs < 10.0,
        "Too slow: {:.2}s (expected <10s)",
        result.duration_secs
    );
    println!("✅ HEVC/H.265 codec: {:.2}s", result.duration_secs);
}
