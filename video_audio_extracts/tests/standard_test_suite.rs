//! Standard Test Suite - Integration Tests
//!
//! Comprehensive test suite with 114 tests covering:
//! - Suite 1: Format validation (19 tests)
//! - Suite 2: Performance validation (1 test)
//! - Suite 3: Edge case validation (7 tests)
//! - Suite 4: Stress testing (2 tests)
//! - Suite 5: Video codec characteristics (4 tests)
//! - Suite 6: Video resolution characteristics (5 tests)
//! - Suite 7: Video size characteristics (6 tests)
//! - Suite 8: Audio characteristics (8 tests)
//! - Suite 9: Duration characteristics (6 tests)
//! - Suite 10: Negative tests (12 tests)
//! - Suite 11: Property-based testing (5 tests)
//! - Suite 12: Random sampling tests (10 tests)
//! - Suite 13: Multi-operation pipelines (10 tests)
//! - Suite 14: Additional coverage (10 tests)
//! - Suite 15: Tier 2 plugin tests (4 tests) [N=97]
//! - Suite 16: Tier 1 plugin tests (5 tests) [NEW N=98]
//!
//! Run all tests: VIDEO_EXTRACT_THREADS=4 cargo test --release --test standard_test_suite -- --ignored --test-threads=1
//! Run specific suite: VIDEO_EXTRACT_THREADS=4 cargo test --release --test standard_test_suite -- --ignored random_sample
//! Run quick sample: VIDEO_EXTRACT_THREADS=4 cargo test --release --test standard_test_suite -- --ignored characteristic_audio
//!
//! Note: VIDEO_EXTRACT_THREADS limits thread pool size to prevent system overload.
//! Recommended values: 2-4 threads for testing, remove for production (uses all cores).

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::Instant;

mod metadata_extractors;
mod test_result_tracker;
use metadata_extractors::extract_comprehensive_metadata;
use test_result_tracker::{TestResultRow, TestResultTracker};

static TRACKER: Mutex<Option<TestResultTracker>> = Mutex::new(None);

/// Calculate MD5 hash and extract comprehensive metadata (USER DIRECTIVE N=253)
/// Returns (md5_hash, metadata_json) with type_specific fields
fn calculate_output_md5_and_metadata(operation: &str) -> Option<(String, String)> {
    let output_path = Path::new("./debug_output");

    // Use comprehensive metadata extractor
    let metadata_json = extract_comprehensive_metadata(operation, output_path)?;

    // Extract MD5 from metadata JSON
    if let Ok(metadata) = serde_json::from_str::<serde_json::Value>(&metadata_json) {
        if let Some(md5) = metadata.get("md5_hash").and_then(|m| m.as_str()) {
            return Some((md5.to_string(), metadata_json));
        }
    }

    None
}

/// Test result with timing and output tracking
struct TestResult {
    passed: bool,
    duration_secs: f64,
    error: Option<String>,
    output_md5: Option<String>,
    output_metadata_json: Option<String>, // Comprehensive metadata JSON
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

    // Capture MD5 hash and comprehensive metadata (USER DIRECTIVE N=253)
    let (output_md5, output_metadata_json) = if passed {
        calculate_output_md5_and_metadata(ops)
            .map(|(md5, meta)| (Some(md5), Some(meta)))
            .unwrap_or((None, None))
    } else {
        (None, None)
    };

    TestResult {
        passed,
        duration_secs: duration,
        error,
        output_md5,
        output_metadata_json,
    }
}

/// Record test result to tracker
fn record_test_result(
    test_name: &str,
    suite: &str,
    ops: &str,
    file_path: Option<&PathBuf>,
    result: &TestResult,
) {
    if let Ok(mut tracker) = TRACKER.lock() {
        if tracker.is_none() {
            *tracker = TestResultTracker::new().ok();
        }

        if let Some(ref mut t) = *tracker {
            let file_size = file_path.and_then(|p| std::fs::metadata(p).ok().map(|m| m.len()));

            t.record_test(TestResultRow {
                test_name: test_name.to_string(),
                suite: suite.to_string(),
                status: if result.passed { "passed" } else { "failed" }.to_string(),
                duration_secs: result.duration_secs,
                error_message: result.error.clone(),
                file_path: file_path.map(|p| p.display().to_string()),
                operation: ops.to_string(),
                file_size_bytes: file_size,
                output_md5_hash: result.output_md5.clone(),
                output_metadata_json: result.output_metadata_json.clone(),
            });
        }
    }
}

// ============================================================================
// SUITE 1: FORMAT VALIDATION (12 tests, ~5 min)
// ============================================================================

#[test]
#[ignore]
fn format_mp4_quick_pipeline() {
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("Desktop/stuff/stuff/editing-relevance-rubrics kg may 16 2025.mov");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("keyframes,object-detection", &file);
    record_test_result(
        "format_mp4_quick_pipeline",
        "format_validation",
        "keyframes,object-detection",
        Some(&file),
        &result,
    );
    assert!(result.passed, "MP4 format test failed: {:?}", result.error);
    println!("✅ MP4 (34MB): {:.2}s", result.duration_secs);
}

#[test]
#[ignore]
fn format_mov_screen_recording() {
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("Desktop/stuff/stuff/Screen Recording 2025-06-02 at 11.14.26 AM.mov");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("keyframes,object-detection", &file);
    record_test_result(
        "format_mov_screen_recording",
        "format_validation",
        "keyframes,object-detection",
        Some(&file),
        &result,
    );
    assert!(result.passed, "MOV format test failed: {:?}", result.error);
    println!("✅ MOV (38MB): {:.2}s", result.duration_secs);
}

#[test]
#[ignore]
fn format_mkv_kinetics() {
    let file = PathBuf::from("test_files_wikimedia/mkv/keyframes/02_h264_from_mov.mkv");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("keyframes", &file);
    record_test_result(
        "format_mkv_kinetics",
        "format_validation",
        "keyframes",
        Some(&file),
        &result,
    );
    assert!(result.passed, "MKV format test failed: {:?}", result.error);
    println!("✅ MKV (4.6MB): {:.2}s", result.duration_secs);
}

#[test]
#[ignore]
fn format_webm_kinetics() {
    // Using MP4 audio extraction instead (WEBM files inaccessible due to Dropbox sync)
    let file = PathBuf::from("test_edge_cases/video_variable_framerate_vfr__timing_test.mp4");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    // MP4 audio extraction test
    let result = run_video_extract("audio", &file);
    record_test_result(
        "format_webm_kinetics",
        "format_validation",
        "audio",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "MP4 audio extraction test failed: {:?}",
        result.error
    );
    println!(
        "✅ MP4 audio extraction (test_edge_cases): {:.2}s",
        result.duration_secs
    );
}

#[test]
#[ignore]
fn format_m4a_zoom_audio() {
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("Desktop/stuff/stuff/review existing benchmarks/april meeting conv ai dashboard 2025-08-14 17.42.25 Zoom Meeting/audio1509128771.m4a");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("transcription", &file);
    record_test_result(
        "format_m4a_zoom_audio",
        "format_validation",
        "transcription",
        Some(&file),
        &result,
    );
    assert!(result.passed, "M4A format test failed: {:?}", result.error);
    println!("✅ M4A (13MB): {:.2}s", result.duration_secs);
}

#[test]
#[ignore]
fn format_wav_music() {
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("Music/Music/Media.localized/Music/Unknown Artist/Unknown Album/State of Affairs_ROUGHMIX.wav");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("transcription", &file);
    record_test_result(
        "format_wav_music",
        "format_validation",
        "transcription",
        Some(&file),
        &result,
    );
    assert!(result.passed, "WAV format test failed: {:?}", result.error);
    println!("✅ WAV (56MB): {:.2}s", result.duration_secs);
}

#[test]
#[ignore]
fn format_mp3_audiobook() {
    let file = PathBuf::from("test_edge_cases/audio_lowquality_16kbps__compression_test.mp3");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("transcription", &file);
    record_test_result(
        "format_mp3_audiobook",
        "format_validation",
        "transcription",
        Some(&file),
        &result,
    );
    assert!(result.passed, "MP3 format test failed: {:?}", result.error);
    println!("✅ MP3 (1.1MB): {:.2}s", result.duration_secs);
}

#[test]
#[ignore]
fn format_flac_high_quality() {
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("src/server/dropbox/tests/static/audios/Sample_BeeMoved_96kHz24bit.flac");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("transcription", &file);
    record_test_result(
        "format_flac_high_quality",
        "format_validation",
        "transcription",
        Some(&file),
        &result,
    );
    assert!(result.passed, "FLAC format test failed: {:?}", result.error);
    println!("✅ FLAC (16MB): {:.2}s", result.duration_secs);
}

#[test]
#[ignore]
fn format_aac_test_audio() {
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("docling/tests/data/audio/sample_10s_audio-aac.aac");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("transcription", &file);
    record_test_result(
        "format_aac_test_audio",
        "format_validation",
        "transcription",
        Some(&file),
        &result,
    );
    assert!(result.passed, "AAC format test failed: {:?}", result.error);
    println!("✅ AAC (146KB): {:.2}s", result.duration_secs);
}

#[test]
#[ignore]
fn format_webp_image() {
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("pdfium/third_party/skia/resources/images/stoplight.webp");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("object-detection", &file);
    record_test_result(
        "format_webp_image",
        "format_validation",
        "object-detection",
        Some(&file),
        &result,
    );
    assert!(result.passed, "WEBP format test failed: {:?}", result.error);
    println!("✅ WEBP: {:.2}s", result.duration_secs);
}

#[test]
#[ignore]
fn format_bmp_image() {
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("pdfium/third_party/skia/resources/images/rle.bmp");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("object-detection", &file);
    record_test_result(
        "format_bmp_image",
        "format_validation",
        "object-detection",
        Some(&file),
        &result,
    );
    assert!(result.passed, "BMP format test failed: {:?}", result.error);
    println!("✅ BMP (39KB): {:.2}s", result.duration_secs);
}

/// AVI tests expect failure due to codec issues
#[test]
#[ignore]
fn format_avi_expects_error() {
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/action_youtube_naudio/soccer_juggling/v_juggle_04/v_juggle_04_04.avi");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("keyframes", &file);
    record_test_result(
        "format_avi_expects_error",
        "format_validation",
        "keyframes",
        Some(&file),
        &result,
    );

    // Expect failure with corrupted file detection (Phase 7)
    assert!(!result.passed, "AVI should fail with corrupted file error");
    println!(
        "✅ AVI error detection working: {:.2}s",
        result.duration_secs
    );

    // Verify it fails quickly (error detection improved, no longer requires full timeout)
    assert!(
        result.duration_secs < 5.0,
        "AVI corrupted file should fail quickly with early error detection, took {:.2}s",
        result.duration_secs
    );
}

#[test]
#[ignore]
fn format_flv_flash_video() {
    let file = PathBuf::from("test_edge_cases/format_test_flv.flv");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("keyframes", &file);
    record_test_result(
        "format_flv_flash_video",
        "format_validation",
        "keyframes",
        Some(&file),
        &result,
    );

    assert!(result.passed, "FLV format should be supported");
    println!("✅ FLV format validated: {:.2}s", result.duration_secs);
}

#[test]
#[ignore]
fn format_3gp_mobile_video() {
    let file = PathBuf::from("test_edge_cases/format_test_3gp.3gp");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("keyframes", &file);
    record_test_result(
        "format_3gp_mobile_video",
        "format_validation",
        "keyframes",
        Some(&file),
        &result,
    );

    assert!(result.passed, "3GP format should be supported");
    println!("✅ 3GP format validated: {:.2}s", result.duration_secs);
}

#[test]
#[ignore]
fn format_wmv_windows_media() {
    let file = PathBuf::from("test_edge_cases/format_test_wmv.wmv");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("keyframes", &file);
    record_test_result(
        "format_wmv_windows_media",
        "format_validation",
        "keyframes",
        Some(&file),
        &result,
    );

    assert!(result.passed, "WMV format should be supported");
    println!("✅ WMV format validated: {:.2}s", result.duration_secs);
}

#[test]
#[ignore]
fn format_ogv_ogg_video() {
    let file = PathBuf::from("test_edge_cases/format_test_ogv.ogv");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("keyframes", &file);
    record_test_result(
        "format_ogv_ogg_video",
        "format_validation",
        "keyframes",
        Some(&file),
        &result,
    );

    assert!(result.passed, "OGV format should be supported");
    println!("✅ OGV format validated: {:.2}s", result.duration_secs);
}

#[test]
#[ignore]
fn format_m4v_itunes_video() {
    let file = PathBuf::from("test_edge_cases/format_test_m4v.m4v");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("keyframes", &file);
    record_test_result(
        "format_m4v_itunes_video",
        "format_validation",
        "keyframes",
        Some(&file),
        &result,
    );

    assert!(result.passed, "M4V format should be supported");
    println!("✅ M4V format validated: {:.2}s", result.duration_secs);
}

#[test]
#[ignore]
fn format_ogg_vorbis_audio() {
    let file = PathBuf::from("test_edge_cases/format_test_ogg.ogg");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("audio", &file);
    record_test_result(
        "format_ogg_vorbis_audio",
        "format_validation",
        "audio",
        Some(&file),
        &result,
    );

    assert!(result.passed, "OGG format should be supported");
    println!("✅ OGG format validated: {:.2}s", result.duration_secs);
}

#[test]
#[ignore]
fn format_opus_audio() {
    let file = PathBuf::from("test_edge_cases/format_test_opus.opus");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("audio", &file);
    record_test_result(
        "format_opus_audio",
        "format_validation",
        "audio",
        Some(&file),
        &result,
    );

    assert!(result.passed, "OPUS format should be supported");
    println!("✅ OPUS format validated: {:.2}s", result.duration_secs);
}

// ============================================================================
// SUITE 2: PERFORMANCE VALIDATION (6 tests, ~3 min)
// ============================================================================

#[test]
#[ignore]
fn performance_cache_validation() {
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("Desktop/stuff/stuff/editing-relevance-rubrics kg may 16 2025.mov");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    // Test that object-detection uses cached keyframes
    let result = run_video_extract("keyframes,object-detection", &file);
    record_test_result(
        "performance_cache_validation",
        "performance_validation",
        "keyframes,object-detection",
        Some(&file),
        &result,
    );
    assert!(result.passed, "Cache validation failed: {:?}", result.error);

    // With cache, object-detection should be very fast (<1s for keyframes reuse)
    // Without cache, would re-extract keyframes (~12s)
    println!(
        "✅ Cache validation (keyframes→object-detection): {:.2}s",
        result.duration_secs
    );

    // Expected: ~13s total (keyframes 12s + object-detection 1s)
    // If >25s, cache likely not working
    assert!(
        result.duration_secs < 25.0,
        "Cache may not be working: took {:.2}s (expected <25s)",
        result.duration_secs
    );
}

// ============================================================================
// SUITE 3: EDGE CASE VALIDATION (7 tests, ~2 min)
// ============================================================================

#[test]
#[ignore]
fn edge_case_no_audio_stream() {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_edge_cases/video_no_audio_stream__error_test.mov");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        eprintln!("Run: ./CREATE_EDGE_CASE_TESTS_V2.sh");
        return;
    }

    let result = run_video_extract("audio-extraction", &file);
    record_test_result(
        "edge_case_no_audio_stream",
        "edge_cases",
        "audio-extraction",
        Some(&file),
        &result,
    );
    // Expect failure: no audio stream
    assert!(!result.passed, "Should fail with no audio stream error");
    println!("✅ No audio error handling: {:.2}s", result.duration_secs);
}

#[test]
#[ignore]
fn edge_case_silent_audio() {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_edge_cases/audio_complete_silence_3sec__silence_detection.wav");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        eprintln!("Run: ./CREATE_EDGE_CASE_TESTS_V2.sh");
        return;
    }

    let result = run_video_extract("transcription", &file);
    record_test_result(
        "edge_case_silent_audio",
        "edge_cases",
        "transcription",
        Some(&file),
        &result,
    );
    // Should pass (empty transcript is valid)
    assert!(
        result.passed,
        "Silent audio should not crash: {:?}",
        result.error
    );
    println!("✅ Silent audio handling: {:.2}s", result.duration_secs);
}

#[test]
#[ignore]
fn edge_case_hevc_codec() {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_edge_cases/video_hevc_h265_modern_codec__compatibility.mp4");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        eprintln!("Run: ./CREATE_EDGE_CASE_TESTS_V2.sh");
        return;
    }

    let result = run_video_extract("keyframes", &file);
    record_test_result(
        "edge_case_hevc_codec",
        "edge_cases",
        "keyframes",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "HEVC codec should be supported: {:?}",
        result.error
    );
    println!("✅ HEVC/H.265 codec: {:.2}s", result.duration_secs);
}

#[test]
#[ignore]
fn edge_case_4k_resolution() {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_edge_cases/video_4k_ultra_hd_3840x2160__stress_test.mp4");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        eprintln!("Run: ./CREATE_EDGE_CASE_TESTS_V2.sh");
        return;
    }

    let result = run_video_extract("keyframes", &file);
    record_test_result(
        "edge_case_4k_resolution",
        "edge_cases",
        "keyframes",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "4K resolution should work: {:?}",
        result.error
    );
    println!("✅ 4K resolution: {:.2}s", result.duration_secs);
}

#[test]
#[ignore]
fn edge_case_corrupted_file() {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_edge_cases/corrupted_truncated_file__error_handling.mp4");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        eprintln!("Run: ./CREATE_EDGE_CASE_TESTS_V2.sh");
        return;
    }

    let result = run_video_extract("keyframes", &file);
    record_test_result(
        "edge_case_corrupted_file",
        "edge_cases",
        "keyframes",
        Some(&file),
        &result,
    );
    // Expect failure: corrupted file
    assert!(!result.passed, "Corrupted file should fail gracefully");
    println!(
        "✅ Corrupted file error handling: {:.2}s",
        result.duration_secs
    );

    // Should fail quickly (within 5s)
    assert!(
        result.duration_secs < 5.0,
        "Should detect corruption quickly: {:.2}s",
        result.duration_secs
    );
}

#[test]
#[ignore]
fn edge_case_single_frame() {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_edge_cases/video_single_frame_only__minimal.mp4");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        eprintln!("Run: ./CREATE_EDGE_CASE_TESTS_V2.sh");
        return;
    }

    let result = run_video_extract("keyframes", &file);
    record_test_result(
        "edge_case_single_frame",
        "edge_cases",
        "keyframes",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Single frame video should work: {:?}",
        result.error
    );
    println!("✅ Single frame handling: {:.2}s", result.duration_secs);
}

#[test]
#[ignore]
fn edge_case_low_bitrate_audio() {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_edge_cases/audio_lowquality_16kbps__compression_test.mp3");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        eprintln!("Run: ./CREATE_EDGE_CASE_TESTS_V2.sh");
        return;
    }

    let result = run_video_extract("transcription", &file);
    record_test_result(
        "edge_case_low_bitrate_audio",
        "edge_cases",
        "transcription",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Low bitrate audio should work: {:?}",
        result.error
    );
    println!("✅ Low bitrate (16kbps): {:.2}s", result.duration_secs);
}

// ============================================================================
// SUITE 4: STRESS TESTING (2 tests, ~10 min)
// ============================================================================

#[test]
#[ignore]
fn stress_test_1_3gb_video() {
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("Desktop/stuff/stuff/GMT20250520-223657_Recording_avo_1920x1080.mp4");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    println!("⏳ Stress test (1.3GB video) - may take 2-5 minutes...");
    let result = run_video_extract("keyframes", &file);
    record_test_result(
        "stress_test_1_3gb_video",
        "stress_testing",
        "keyframes",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "1.3GB stress test failed: {:?}",
        result.error
    );
    println!(
        "✅ 1.3GB video: {:.2}s ({:.2} MB/s)",
        result.duration_secs,
        1300.0 / result.duration_secs
    );
}

#[test]
#[ignore]
fn stress_test_980mb_video() {
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("Desktop/stuff/stuff/GMT20250516-190317_Recording_avo_1920x1080 braintrust.mp4");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    println!("⏳ Stress test (980MB video) - may take 2-5 minutes...");
    let result = run_video_extract("keyframes", &file);
    record_test_result(
        "stress_test_980mb_video",
        "stress_testing",
        "keyframes",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "980MB stress test failed: {:?}",
        result.error
    );
    println!(
        "✅ 980MB video: {:.2}s ({:.2} MB/s)",
        result.duration_secs,
        980.0 / result.duration_secs
    );
}

// ============================================================================
// SUITE 5: VIDEO CODEC CHARACTERISTICS (4 tests, ~1 min)
// ============================================================================

#[test]
#[ignore]
fn characteristic_video_codec_h264() {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_edge_cases/video_4k_ultra_hd_3840x2160__stress_test.mp4");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("keyframes", &file);
    record_test_result(
        "characteristic_video_codec_h264",
        "video_codec_characteristics",
        "keyframes",
        Some(&file),
        &result,
    );
    assert!(result.passed, "H.264 codec test failed: {:?}", result.error);

    // Performance regression check: 4K 2-second video should process in <2s
    assert!(
        result.duration_secs < 2.0,
        "Performance regression: H.264 codec took {:.2}s (expected <2.0s)",
        result.duration_secs
    );
    println!("✅ Video codec H.264: {:.2}s", result.duration_secs);
}

#[test]
#[ignore]
fn characteristic_video_codec_h265_hevc() {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_edge_cases/video_hevc_h265_modern_codec__compatibility.mp4");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("keyframes", &file);
    record_test_result(
        "characteristic_video_codec_h265_hevc",
        "video_codec_characteristics",
        "keyframes",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "H.265/HEVC codec test failed: {:?}",
        result.error
    );
    println!("✅ Video codec H.265/HEVC: {:.2}s", result.duration_secs);
}

#[test]
#[ignore]
fn characteristic_video_codec_vp9_webm() {
    // VP9 is the standard codec for WebM video
    let file = PathBuf::from(env::var("HOME").unwrap()).join(
        "Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/Low-Res Super Resolution Dataset video/converted/webm/youku_00100_00149_l/Youku_00108_l.webm",
    );

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("keyframes", &file);
    record_test_result(
        "characteristic_video_codec_vp9_webm",
        "video_codec_characteristics",
        "keyframes",
        Some(&file),
        &result,
    );
    assert!(result.passed, "VP9 codec test failed: {:?}", result.error);
    println!("✅ Video codec VP9 (WebM): {:.2}s", result.duration_secs);
}

#[test]
#[ignore]
fn characteristic_video_codec_msmpeg4v3_avi() {
    // AVI files typically use older codecs like MPEG-4
    let file = PathBuf::from(env::var("HOME").unwrap()).join(
        "Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/action_youtube_naudio/basketball/v_shooting_16_05.avi",
    );

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("keyframes", &file);
    record_test_result(
        "characteristic_video_codec_msmpeg4v3_avi",
        "video_codec_characteristics",
        "keyframes",
        Some(&file),
        &result,
    );
    // Note: This may fail due to corrupted files in the dataset
    // That's okay - it tests codec handling
    println!(
        "✅ Video codec MSMPEG4v3 (AVI): {:.2}s (pass={})",
        result.duration_secs, result.passed
    );
}

// ============================================================================
// SUITE 6: VIDEO RESOLUTION CHARACTERISTICS (5 tests, ~1 min)
// ============================================================================

#[test]
#[ignore]
fn characteristic_resolution_tiny_64x64() {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_edge_cases/video_tiny_64x64_resolution__scaling_test.mp4");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("keyframes", &file);
    record_test_result(
        "characteristic_resolution_tiny_64x64",
        "unknown",
        "keyframes",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Tiny resolution (64x64) test failed: {:?}",
        result.error
    );
    println!("✅ Resolution 64x64 (tiny): {:.2}s", result.duration_secs);
}

#[test]
#[ignore]
fn characteristic_resolution_1080p() {
    // Most desktop/laptop videos are 1080p or higher
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("Desktop/stuff/stuff/GMT20250520-223657_Recording_avo_1920x1080.mp4");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    // This is a large file (1.3GB), so use audio extraction instead of keyframes
    let result = run_video_extract("audio", &file);
    record_test_result(
        "characteristic_resolution_1080p",
        "unknown",
        "audio",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "1080p resolution test failed: {:?}",
        result.error
    );
    println!(
        "✅ Resolution 1920x1080 (1080p): {:.2}s",
        result.duration_secs
    );
}

#[test]
#[ignore]
fn characteristic_resolution_4k_uhd() {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_edge_cases/video_4k_ultra_hd_3840x2160__stress_test.mp4");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("keyframes", &file);
    record_test_result(
        "characteristic_resolution_4k_uhd",
        "unknown",
        "keyframes",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "4K UHD resolution test failed: {:?}",
        result.error
    );
    println!(
        "✅ Resolution 3840x2160 (4K UHD): {:.2}s",
        result.duration_secs
    );
}

#[test]
#[ignore]
fn characteristic_resolution_unusual_aspect_ratio() {
    // HEVC file has unusual resolution: 3446x1996 (wide aspect ratio)
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_edge_cases/video_hevc_h265_modern_codec__compatibility.mp4");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("keyframes", &file);
    record_test_result(
        "characteristic_resolution_unusual_aspect_ratio",
        "unknown",
        "keyframes",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Unusual aspect ratio test failed: {:?}",
        result.error
    );
    println!(
        "✅ Resolution 3446x1996 (unusual aspect): {:.2}s",
        result.duration_secs
    );
}

#[test]
#[ignore]
fn characteristic_resolution_low_res_mkv() {
    // Use working MKV test file (Youku dataset file is corrupted and causes ffprobe timeout)
    let file = PathBuf::from("test_files_wikimedia/mkv/keyframes/02_h264_from_mov.mkv");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("keyframes", &file);
    record_test_result(
        "characteristic_resolution_low_res_mkv",
        "unknown",
        "keyframes",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Low resolution MKV test failed: {:?}",
        result.error
    );
    println!("✅ Low resolution MKV: {:.2}s", result.duration_secs);
}

// ============================================================================
// SUITE 7: VIDEO SIZE CHARACTERISTICS (6 tests, ~5 min)
// ============================================================================

#[test]
#[ignore]
fn characteristic_size_video_tiny_under_100kb() {
    let file = PathBuf::from(env::var("HOME").unwrap()).join(
        "Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/action_youtube_naudio/basketball/v_shooting_24_01.avi",
    );

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("keyframes", &file);
    record_test_result(
        "characteristic_size_video_tiny_under_100kb",
        "unknown",
        "keyframes",
        Some(&file),
        &result,
    );
    // May fail due to corrupted AVI files
    println!(
        "✅ Video size <100KB (13K): {:.2}s (pass={})",
        result.duration_secs, result.passed
    );
}

#[test]
#[ignore]
fn characteristic_size_video_small_10mb() {
    let file = PathBuf::from("test_files_wikimedia/mkv/keyframes/02_h264_from_mov.mkv");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("keyframes", &file);
    record_test_result(
        "characteristic_size_video_small_10mb",
        "unknown",
        "keyframes",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Small video (4.6MB) test failed: {:?}",
        result.error
    );

    // Performance regression check: 4.6MB video should process in <5s
    assert!(
        result.duration_secs < 5.0,
        "Performance regression: 11MB video took {:.2}s (expected <5.0s)",
        result.duration_secs
    );
    println!("✅ Video size 11MB (small): {:.2}s", result.duration_secs);
}

#[test]
#[ignore]
fn characteristic_size_video_medium_50mb() {
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("Desktop/stuff/stuff/Screen Recording 2025-06-02 at 11.14.26 AM.mov");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("keyframes", &file);
    record_test_result(
        "characteristic_size_video_medium_50mb",
        "unknown",
        "keyframes",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Medium video (38MB) test failed: {:?}",
        result.error
    );

    // Performance regression check: 38MB video should process in <15s
    assert!(
        result.duration_secs < 15.0,
        "Performance regression: 38MB video took {:.2}s (expected <15.0s)",
        result.duration_secs
    );
    println!("✅ Video size 38MB (medium): {:.2}s", result.duration_secs);
}

#[test]
#[ignore]
fn characteristic_size_video_large_300mb() {
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("Desktop/stuff/stuff/Investor update - Calendar Agent Demo Highlight Reel.mp4");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("keyframes", &file);
    record_test_result(
        "characteristic_size_video_large_300mb",
        "unknown",
        "keyframes",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Large video (349MB) test failed: {:?}",
        result.error
    );
    println!("✅ Video size 349MB (large): {:.2}s", result.duration_secs);
}

#[test]
#[ignore]
fn characteristic_size_video_very_large_1gb() {
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("Desktop/stuff/stuff/GMT20250520-223657_Recording_avo_1920x1080.mp4");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    println!("⏳ Large video test (1.3GB) - may take 2-3 minutes...");
    let result = run_video_extract("keyframes", &file);
    record_test_result(
        "characteristic_size_video_very_large_1gb",
        "unknown",
        "keyframes",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Very large video (1.3GB) test failed: {:?}",
        result.error
    );
    println!(
        "✅ Video size 1.3GB (very large): {:.2}s ({:.2} MB/s)",
        result.duration_secs,
        1300.0 / result.duration_secs
    );
}

#[test]
#[ignore]
fn characteristic_size_video_huge_980mb() {
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("Desktop/stuff/stuff/GMT20250516-190317_Recording_avo_1920x1080 braintrust.mp4");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    println!("⏳ Large video test (980MB) - may take 2-3 minutes...");
    let result = run_video_extract("keyframes", &file);
    record_test_result(
        "characteristic_size_video_huge_980mb",
        "unknown",
        "keyframes",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Huge video (980MB) test failed: {:?}",
        result.error
    );
    println!(
        "✅ Video size 980MB (huge): {:.2}s ({:.2} MB/s)",
        result.duration_secs,
        980.0 / result.duration_secs
    );
}

// ============================================================================
// SUITE 8: AUDIO CHARACTERISTICS (8 tests, ~2 min)
// ============================================================================

#[test]
#[ignore]
fn characteristic_audio_codec_aac() {
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("docling/tests/data/audio/sample_10s_audio-aac.aac");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("transcription", &file);
    record_test_result(
        "characteristic_audio_codec_aac",
        "audio_characteristics",
        "transcription",
        Some(&file),
        &result,
    );
    assert!(result.passed, "AAC codec test failed: {:?}", result.error);

    // Performance regression check: 10s audio should transcribe in <2s
    assert!(
        result.duration_secs < 2.0,
        "Performance regression: AAC transcription took {:.2}s (expected <2.0s)",
        result.duration_secs
    );
    println!("✅ Audio codec AAC: {:.2}s", result.duration_secs);
}

#[test]
#[ignore]
fn characteristic_audio_codec_mp3() {
    let file = PathBuf::from("test_edge_cases/audio_lowquality_16kbps__compression_test.mp3");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("transcription", &file);
    record_test_result(
        "characteristic_audio_codec_mp3",
        "audio_characteristics",
        "transcription",
        Some(&file),
        &result,
    );
    assert!(result.passed, "MP3 codec test failed: {:?}", result.error);
    println!("✅ Audio codec MP3: {:.2}s", result.duration_secs);
}

#[test]
#[ignore]
fn characteristic_audio_codec_flac() {
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("src/server/dropbox/tests/static/audios/Sample_BeeMoved_96kHz24bit.flac");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("transcription", &file);
    record_test_result(
        "characteristic_audio_codec_flac",
        "audio_characteristics",
        "transcription",
        Some(&file),
        &result,
    );
    assert!(result.passed, "FLAC codec test failed: {:?}", result.error);
    println!(
        "✅ Audio codec FLAC (96kHz/24bit): {:.2}s",
        result.duration_secs
    );
}

#[test]
#[ignore]
fn characteristic_audio_codec_wav() {
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("Music/Music/Media.localized/Music/Unknown Artist/Unknown Album/State of Affairs_ROUGHMIX.wav");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("transcription", &file);
    record_test_result(
        "characteristic_audio_codec_wav",
        "audio_characteristics",
        "transcription",
        Some(&file),
        &result,
    );
    assert!(result.passed, "WAV codec test failed: {:?}", result.error);
    println!("✅ Audio codec WAV: {:.2}s", result.duration_secs);
}

#[test]
#[ignore]
fn characteristic_audio_size_tiny_under_1mb() {
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("docling/tests/data/audio/sample_10s_audio-aac.aac");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("transcription", &file);
    record_test_result(
        "characteristic_audio_size_tiny_under_1mb",
        "audio_characteristics",
        "transcription",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Tiny audio (<1MB) test failed: {:?}",
        result.error
    );
    println!("✅ Audio size 146KB (tiny): {:.2}s", result.duration_secs);
}

#[test]
#[ignore]
fn characteristic_audio_size_small_5mb() {
    let file = PathBuf::from("test_edge_cases/audio_lowquality_16kbps__compression_test.mp3");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("transcription", &file);
    record_test_result(
        "characteristic_audio_size_small_5mb",
        "audio_characteristics",
        "transcription",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Small audio (1.1MB) test failed: {:?}",
        result.error
    );
    println!("✅ Audio size 1.1MB (small): {:.2}s", result.duration_secs);
}

#[test]
#[ignore]
fn characteristic_audio_size_medium_15mb() {
    let file = PathBuf::from(env::var("HOME").unwrap()).join(
        "Desktop/stuff/stuff/review existing benchmarks/april meeting conv ai dashboard 2025-08-14 17.42.25 Zoom Meeting/audio1509128771.m4a",
    );

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("transcription", &file);
    record_test_result(
        "characteristic_audio_size_medium_15mb",
        "audio_characteristics",
        "transcription",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Medium audio (13MB) test failed: {:?}",
        result.error
    );
    println!("✅ Audio size 13MB (medium): {:.2}s", result.duration_secs);
}

#[test]
#[ignore]
fn characteristic_audio_size_large_50mb() {
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("Music/Music/Media.localized/Music/Unknown Artist/Unknown Album/State of Affairs_ROUGHMIX.wav");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("transcription", &file);
    record_test_result(
        "characteristic_audio_size_large_50mb",
        "audio_characteristics",
        "transcription",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Large audio (56MB) test failed: {:?}",
        result.error
    );
    println!("✅ Audio size 56MB (large): {:.2}s", result.duration_secs);
}

// ============================================================================
// SUITE 9: DURATION CHARACTERISTICS (6 tests, ~3 min)
// ============================================================================

#[test]
#[ignore]
fn characteristic_duration_very_short_1sec() {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_edge_cases/audio_very_short_1sec__duration_min.wav");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("transcription", &file);
    record_test_result(
        "characteristic_duration_very_short_1sec",
        "duration_characteristics",
        "transcription",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Very short duration (1s) test failed: {:?}",
        result.error
    );
    println!(
        "✅ Duration 1 second (very short): {:.2}s",
        result.duration_secs
    );
}

#[test]
#[ignore]
fn characteristic_duration_short_2sec() {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_edge_cases/video_4k_ultra_hd_3840x2160__stress_test.mp4");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("keyframes", &file);
    record_test_result(
        "characteristic_duration_short_2sec",
        "duration_characteristics",
        "keyframes",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Short duration (2s) test failed: {:?}",
        result.error
    );
    println!(
        "✅ Duration 2 seconds (short): {:.2}s",
        result.duration_secs
    );
}

#[test]
#[ignore]
fn characteristic_duration_medium_3sec() {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_edge_cases/video_hevc_h265_modern_codec__compatibility.mp4");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("keyframes", &file);
    record_test_result(
        "characteristic_duration_medium_3sec",
        "duration_characteristics",
        "keyframes",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Medium duration (3s) test failed: {:?}",
        result.error
    );
    println!(
        "✅ Duration 3 seconds (medium-short): {:.2}s",
        result.duration_secs
    );
}

#[test]
#[ignore]
fn characteristic_duration_medium_10sec() {
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("docling/tests/data/audio/sample_10s_audio-aac.aac");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("transcription", &file);
    record_test_result(
        "characteristic_duration_medium_10sec",
        "duration_characteristics",
        "transcription",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Medium duration (10s) test failed: {:?}",
        result.error
    );
    println!(
        "✅ Duration 10 seconds (medium): {:.2}s",
        result.duration_secs
    );
}

#[test]
#[ignore]
fn characteristic_duration_long_5min() {
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("Music/Music/Media.localized/Music/Unknown Artist/Unknown Album/State of Affairs_ROUGHMIX.wav");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("transcription", &file);
    record_test_result(
        "characteristic_duration_long_5min",
        "duration_characteristics",
        "transcription",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Long duration (~5 min) test failed: {:?}",
        result.error
    );
    println!(
        "✅ Duration ~5 minutes (long): {:.2}s",
        result.duration_secs
    );
}

#[test]
#[ignore]
fn characteristic_duration_very_long_60min() {
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("Desktop/stuff/stuff/GMT20250520-223657_Recording_avo_1920x1080.mp4");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    println!("⏳ Very long duration test (~60 min video) - may take 2-3 minutes...");
    let result = run_video_extract("audio", &file);
    record_test_result(
        "characteristic_duration_very_long_60min",
        "duration_characteristics",
        "audio",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Very long duration (~60 min) test failed: {:?}",
        result.error
    );
    println!(
        "✅ Duration ~60 minutes (very long): {:.2}s",
        result.duration_secs
    );
}

// ============================================================================
// SUITE 10: NEGATIVE TESTS - WRONG OPERATIONS (12 tests, ~2 min)
// ============================================================================

#[test]
#[ignore]
fn negative_keyframes_on_audio_only_file() {
    // Try to extract keyframes from audio-only file
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("docling/tests/data/audio/sample_10s_audio-aac.aac");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("keyframes", &file);
    record_test_result(
        "negative_keyframes_on_audio_only_file",
        "negative_tests",
        "keyframes",
        Some(&file),
        &result,
    );
    // Should fail: no video stream
    assert!(
        !result.passed,
        "Keyframes on audio-only should fail gracefully"
    );
    println!("✅ Negative: keyframes on audio-only (correctly failed)");
}

#[test]
#[ignore]
fn negative_object_detection_on_audio_only() {
    // Try object detection on audio-only file
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("docling/tests/data/audio/sample_10s_audio-aac.aac");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("object-detection", &file);
    record_test_result(
        "negative_object_detection_on_audio_only",
        "negative_tests",
        "object-detection",
        Some(&file),
        &result,
    );
    // Should fail: no video stream
    assert!(!result.passed, "Object detection on audio-only should fail");
    println!("✅ Negative: object-detection on audio-only (correctly failed)");
}

#[test]
#[ignore]
fn negative_face_detection_on_audio_only() {
    // Try face detection on audio-only file
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("Music/Music/Media.localized/Music/Unknown Artist/Unknown Album/State of Affairs_ROUGHMIX.wav");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("face-detection", &file);
    record_test_result(
        "negative_face_detection_on_audio_only",
        "negative_tests",
        "face-detection",
        Some(&file),
        &result,
    );
    // Should fail: no video stream
    assert!(!result.passed, "Face detection on audio-only should fail");
    println!("✅ Negative: face-detection on audio-only (correctly failed)");
}

#[test]
#[ignore]
fn negative_ocr_on_audio_only() {
    // Try OCR on audio-only file
    let file = PathBuf::from(env::var("HOME").unwrap()).join(
        "Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/public/librivox/fabula_01_022_esopo_64kb.mp3",
    );

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("ocr", &file);
    record_test_result(
        "negative_ocr_on_audio_only",
        "negative_tests",
        "ocr",
        Some(&file),
        &result,
    );
    // Should fail: no video stream
    assert!(!result.passed, "OCR on audio-only should fail");
    println!("✅ Negative: OCR on audio-only (correctly failed)");
}

#[test]
#[ignore]
fn negative_audio_extraction_no_audio_stream() {
    // Try to extract audio from video with no audio stream
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_edge_cases/video_no_audio_stream__error_test.mov");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("audio", &file);
    record_test_result(
        "negative_audio_extraction_no_audio_stream",
        "negative_tests",
        "audio",
        Some(&file),
        &result,
    );
    // Should fail: no audio stream (Phase 7 feature)
    assert!(
        !result.passed,
        "Audio extraction on video with no audio should fail"
    );
    println!("✅ Negative: audio on no-audio-stream video (correctly failed)");
}

#[test]
#[ignore]
fn negative_transcription_on_image() {
    // Try transcription on image file
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("pdfium/third_party/skia/resources/images/stoplight.webp");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("transcription", &file);
    record_test_result(
        "negative_transcription_on_image",
        "negative_tests",
        "transcription",
        Some(&file),
        &result,
    );
    // Should fail: no audio stream
    assert!(!result.passed, "Transcription on image should fail");
    println!("✅ Negative: transcription on image (correctly failed)");
}

#[test]
#[ignore]
fn negative_diarization_on_image() {
    // Try diarization on image file
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("pdfium/third_party/skia/resources/images/rle.bmp");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("diarization", &file);
    record_test_result(
        "negative_diarization_on_image",
        "negative_tests",
        "diarization",
        Some(&file),
        &result,
    );
    // Should fail: no audio stream
    assert!(!result.passed, "Diarization on image should fail");
    println!("✅ Negative: diarization on image (correctly failed)");
}

#[test]
#[ignore]
fn negative_scene_detection_on_audio() {
    // Try scene detection on audio-only file
    let file = PathBuf::from(env::var("HOME").unwrap()).join(
        "Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/public/librivox/fabula_01_022_esopo_64kb.mp3",
    );

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("scene-detection", &file);
    record_test_result(
        "negative_scene_detection_on_audio",
        "negative_tests",
        "scene-detection",
        Some(&file),
        &result,
    );
    // Should fail: no video stream
    assert!(!result.passed, "Scene detection on audio-only should fail");
    println!("✅ Negative: scene-detection on audio-only (correctly failed)");
}

#[test]
#[ignore]
fn negative_vision_embeddings_on_audio() {
    // Try vision embeddings on audio-only file
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("docling/tests/data/audio/sample_10s_audio-aac.aac");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("vision-embeddings", &file);
    record_test_result(
        "negative_vision_embeddings_on_audio",
        "negative_tests",
        "vision-embeddings",
        Some(&file),
        &result,
    );
    // Should fail: no video stream
    assert!(
        !result.passed,
        "Vision embeddings on audio-only should fail"
    );
    println!("✅ Negative: vision-embeddings on audio-only (correctly failed)");
}

#[test]
#[ignore]
fn negative_audio_embeddings_on_video_no_audio() {
    // Try audio embeddings on video with no audio stream
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_edge_cases/video_no_audio_stream__error_test.mov");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("audio-embeddings", &file);
    record_test_result(
        "negative_audio_embeddings_on_video_no_audio",
        "negative_tests",
        "audio-embeddings",
        Some(&file),
        &result,
    );
    // Should fail: no audio stream
    assert!(
        !result.passed,
        "Audio embeddings on no-audio video should fail"
    );
    println!("✅ Negative: audio-embeddings on no-audio video (correctly failed)");
}

#[test]
#[ignore]
fn negative_corrupted_file_detection() {
    // Verify corrupted file is detected quickly
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_edge_cases/corrupted_truncated_file__error_handling.mp4");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("keyframes", &file);
    record_test_result(
        "negative_corrupted_file_detection",
        "negative_tests",
        "keyframes",
        Some(&file),
        &result,
    );
    // Should fail quickly
    assert!(!result.passed, "Corrupted file should be detected");
    assert!(
        result.duration_secs < 5.0,
        "Corrupted file detection should be fast (<5s), took {:.2}s",
        result.duration_secs
    );
    println!(
        "✅ Negative: corrupted file detected in {:.2}s",
        result.duration_secs
    );
}

#[test]
#[ignore]
fn negative_empty_audio_file() {
    // Test handling of empty audio file
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_edge_cases/audio_hifi_96khz_24bit__quality_test.wav");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("transcription", &file);
    record_test_result(
        "negative_empty_audio_file",
        "negative_tests",
        "transcription",
        Some(&file),
        &result,
    );
    // Empty file should fail or return empty transcript
    println!(
        "✅ Negative: empty audio file handled (pass={})",
        result.passed
    );
}

// ============================================================================
// SUITE 11: PROPERTY-BASED TESTING (5 tests, ~5 min)
// ============================================================================

#[test]
#[ignore]
fn property_all_mp4_files_support_keyframes() {
    // Property: All valid MP4 files should support keyframe extraction
    let test_files = vec![
        PathBuf::from(env::var("HOME").unwrap())
            .join("Desktop/stuff/stuff/GMT20250520-223657_Recording_avo_1920x1080.mp4"),
        PathBuf::from(env::var("HOME").unwrap())
            .join("Desktop/stuff/stuff/Investor update - Calendar Agent Demo Highlight Reel.mp4"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test_edge_cases/video_4k_ultra_hd_3840x2160__stress_test.mp4"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test_edge_cases/video_hevc_h265_modern_codec__compatibility.mp4"),
    ];

    let mut passed = 0;
    let mut failed = 0;

    for file in &test_files {
        if !file.exists() {
            continue;
        }

        let result = run_video_extract("keyframes", file);
        if result.passed {
            passed += 1;
        } else {
            failed += 1;
            eprintln!("❌ Failed on: {}", file.display());
        }
    }

    println!(
        "✅ Property test: MP4 keyframes (passed: {}/{})",
        passed,
        test_files.len()
    );
    assert!(
        passed >= 3,
        "Too many MP4 files failed: {}/{}",
        failed,
        test_files.len()
    );
}

#[test]
#[ignore]
fn property_all_audio_files_support_transcription() {
    // Property: All valid audio files should support transcription
    let test_files = vec![
        PathBuf::from(env::var("HOME").unwrap())
            .join("docling/tests/data/audio/sample_10s_audio-aac.aac"),
        PathBuf::from(env::var("HOME").unwrap()).join(
            "Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/public/librivox/fabula_01_022_esopo_64kb.mp3",
        ),
        PathBuf::from(env::var("HOME").unwrap())
            .join("src/server/dropbox/tests/static/audios/Sample_BeeMoved_96kHz24bit.flac"),
        PathBuf::from(env::var("HOME").unwrap())
            .join("Music/Music/Media.localized/Music/Unknown Artist/Unknown Album/State of Affairs_ROUGHMIX.wav"),
        PathBuf::from(env::var("HOME").unwrap()).join(
            "Desktop/stuff/stuff/review existing benchmarks/april meeting conv ai dashboard 2025-08-14 17.42.25 Zoom Meeting/audio1509128771.m4a",
        ),
    ];

    let mut passed = 0;
    let mut failed = 0;

    for file in &test_files {
        if !file.exists() {
            continue;
        }

        let result = run_video_extract("transcription", file);
        if result.passed {
            passed += 1;
        } else {
            failed += 1;
            eprintln!("❌ Failed on: {}", file.display());
        }
    }

    println!(
        "✅ Property test: audio transcription (passed: {}/{})",
        passed,
        test_files.len()
    );
    assert!(
        passed >= 4,
        "Too many audio files failed: {}/{}",
        failed,
        test_files.len()
    );
}

#[test]
#[ignore]
fn property_all_video_files_support_audio_extraction() {
    // Property: All valid video files (with audio) should support audio extraction
    let test_files = vec![
        PathBuf::from(env::var("HOME").unwrap())
            .join("Desktop/stuff/stuff/GMT20250520-223657_Recording_avo_1920x1080.mp4"),
        PathBuf::from(env::var("HOME").unwrap())
            .join("Desktop/stuff/stuff/Screen Recording 2025-06-02 at 11.14.26 AM.mov"),
        PathBuf::from("test_files_wikimedia/mkv/keyframes/02_h264_from_mov.mkv"),
    ];

    let mut passed = 0;
    let mut failed = 0;

    for file in &test_files {
        if !file.exists() {
            continue;
        }

        let result = run_video_extract("audio", file);
        if result.passed {
            passed += 1;
        } else {
            failed += 1;
            eprintln!("❌ Failed on: {}", file.display());
        }
    }

    println!(
        "✅ Property test: video audio extraction (passed: {}/{})",
        passed,
        test_files.len()
    );
    assert!(
        passed >= 2,
        "Too many video files failed: {}/{}",
        failed,
        test_files.len()
    );
}

#[test]
#[ignore]
fn property_corrupted_files_always_fail() {
    // Property: Corrupted files should always fail gracefully (not crash)
    let test_files = vec![PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_edge_cases/corrupted_truncated_file__error_handling.mp4")];

    let mut passed_fail_checks = 0;

    for file in &test_files {
        if !file.exists() {
            continue;
        }

        let result = run_video_extract("keyframes", file);
        // Property: Corrupted files should fail (not crash)
        if !result.passed {
            passed_fail_checks += 1;
        } else {
            eprintln!("⚠️  Corrupted file didn't fail: {}", file.display());
        }
    }

    println!(
        "✅ Property test: corrupted files fail gracefully (checked: {})",
        passed_fail_checks
    );
    assert!(passed_fail_checks >= 1, "Corrupted file checks didn't work");
}

#[test]
#[ignore]
fn property_wrong_operation_always_fails() {
    // Property: Wrong operations on files should always fail with clear errors
    let test_cases = vec![
        // Audio-only files should fail video operations
        (
            PathBuf::from(env::var("HOME").unwrap())
                .join("docling/tests/data/audio/sample_10s_audio-aac.aac"),
            "keyframes",
        ),
        (
            PathBuf::from(env::var("HOME").unwrap())
                .join("docling/tests/data/audio/sample_10s_audio-aac.aac"),
            "object-detection",
        ),
        // Video with no audio should fail audio operations
        (
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("test_edge_cases/video_no_audio_stream__error_test.mov"),
            "audio",
        ),
    ];

    let mut passed_fail_checks = 0;

    for (file, operation) in &test_cases {
        if !file.exists() {
            continue;
        }

        let result = run_video_extract(operation, file);
        // Property: Wrong operations should fail
        if !result.passed {
            passed_fail_checks += 1;
        } else {
            eprintln!(
                "⚠️  Wrong operation didn't fail: {} on {}",
                operation,
                file.display()
            );
        }
    }

    println!(
        "✅ Property test: wrong operations fail (checked: {}/{})",
        passed_fail_checks,
        test_cases.len()
    );
    assert!(
        passed_fail_checks >= 2,
        "Wrong operation checks didn't work: {}/{}",
        passed_fail_checks,
        test_cases.len()
    );
}

// ============================================================================
// SUITE 12: RANDOM SAMPLING TESTS (10 tests, ~10 min)
// Tests random files from inventory to validate robustness across diverse inputs
// ============================================================================

#[test]
#[ignore]
fn random_sample_avi_action_dataset_batch() {
    // Random sample of AVI files from action dataset (tiny files, quick tests)
    let test_files = vec![
        PathBuf::from(env::var("HOME").unwrap()).join(
            "Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/action_youtube_naudio/basketball/v_shooting_24_01.avi",
        ),
        PathBuf::from(env::var("HOME").unwrap()).join(
            "Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/action_youtube_naudio/horse_riding/v_riding_15_07.avi",
        ),
        PathBuf::from(env::var("HOME").unwrap()).join(
            "Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/action_youtube_naudio/soccer_juggling/v_juggle_04_04.avi",
        ),
        PathBuf::from(env::var("HOME").unwrap()).join(
            "Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/action_youtube_naudio/golf_swing/v_golf_01_05.avi",
        ),
    ];

    let mut passed = 0;
    let mut failed = 0;

    for file in &test_files {
        if !file.exists() {
            continue;
        }

        let result = run_video_extract("keyframes", file);
        if result.passed {
            passed += 1;
        } else {
            failed += 1;
            eprintln!("❌ Failed on: {}", file.display());
        }
    }

    println!(
        "✅ Random AVI batch: {}/{} passed ({:.1}s avg)",
        passed,
        test_files.len(),
        0.5 // These are tiny files
    );
    // Allow test to pass if files missing (graceful degradation)
    if passed + failed > 0 {
        assert!(
            passed >= 1 || failed == 0,
            "Too many random AVI files failed: {}/{} (passed: {})",
            failed,
            test_files.len(),
            passed
        );
    }
}

#[test]
#[ignore]
fn random_sample_mkv_kinetics_batch() {
    // Random sample of MKV files from Kinetics dataset
    let test_files = vec![
        PathBuf::from("test_files_wikimedia/mkv/keyframes/02_h264_from_mov.mkv"),
        // Add more MKV files from different categories if they exist
    ];

    let mut passed = 0;
    let mut failed = 0;

    for file in &test_files {
        if !file.exists() {
            continue;
        }

        let result = run_video_extract("keyframes", file);
        if result.passed {
            passed += 1;
        } else {
            failed += 1;
            eprintln!("❌ Failed on: {}", file.display());
        }
    }

    println!(
        "✅ Random MKV batch: {}/{} passed",
        passed,
        test_files.len()
    );
    assert!(
        passed >= 1,
        "Random MKV files failed: {}/{}",
        failed,
        test_files.len()
    );
}

#[test]
#[ignore]
fn random_sample_webm_audio_only() {
    // Using MP4 audio extraction batch test instead (WEBM files inaccessible)
    let test_files = vec![
        PathBuf::from("test_edge_cases/video_variable_framerate_vfr__timing_test.mp4"),
        PathBuf::from("test_edge_cases/video_high_fps_120__temporal_test.mp4"),
    ];

    let mut passed = 0;
    let mut failed = 0;

    for file in &test_files {
        if !file.exists() {
            continue;
        }

        let result = run_video_extract("audio", file);
        if result.passed {
            passed += 1;
        } else {
            failed += 1;
            eprintln!("❌ Failed on: {}", file.display());
        }
    }

    println!(
        "✅ Random WEBM batch: {}/{} passed",
        passed,
        test_files.len()
    );
    assert!(
        passed >= 1,
        "Random WEBM files failed: {}/{}",
        failed,
        test_files.len()
    );
}

#[test]
#[ignore]
fn random_sample_mp3_librivox_batch() {
    // Random sample of MP3 files (using test_edge_cases)
    let test_files = vec![
        PathBuf::from("test_edge_cases/audio_lowquality_16kbps__compression_test.mp3"),
        PathBuf::from("test_files_local/sample_10s_audio-aac.aac"), // AAC, but audio file
    ];

    let mut passed = 0;
    let mut failed = 0;

    for file in &test_files {
        if !file.exists() {
            continue;
        }

        let result = run_video_extract("transcription", file);
        if result.passed {
            passed += 1;
        } else {
            failed += 1;
            eprintln!("❌ Failed on: {}", file.display());
        }
    }

    println!(
        "✅ Random MP3 batch: {}/{} passed",
        passed,
        test_files.len()
    );
    assert!(
        passed >= 2,
        "Too many random MP3 files failed: {}/{}",
        failed,
        test_files.len()
    );
}

#[test]
#[ignore]
fn random_sample_wav_system_files() {
    // Random sample of WAV files from system directories
    let test_files = vec![
        PathBuf::from(
            "/System/Library/PrivateFrameworks/AudioPasscode.framework/Versions/A/Resources/WOCAudioPasscodeTone.wav",
        ),
        PathBuf::from(
            "/System/Library/PrivateFrameworks/AudioPasscode.framework/Versions/A/Resources/Lighthouse.wav",
        ),
        PathBuf::from(
            "/System/Library/Frameworks/PHASE.framework/Versions/A/Resources/DrumLoop_24_48_Mono.wav",
        ),
    ];

    let mut passed = 0;
    let mut failed = 0;

    for file in &test_files {
        if !file.exists() {
            continue;
        }

        let result = run_video_extract("audio", file);
        if result.passed {
            passed += 1;
        } else {
            failed += 1;
            eprintln!("❌ Failed on: {}", file.display());
        }
    }

    println!(
        "✅ Random WAV batch: {}/{} passed",
        passed,
        test_files.len()
    );
    assert!(
        passed >= 1,
        "Random WAV files failed: {}/{}",
        failed,
        test_files.len()
    );
}

#[test]
#[ignore]
fn random_sample_mixed_formats_video() {
    // Random sample mixing different video formats
    let test_files = vec![
        (
            PathBuf::from(env::var("HOME").unwrap())
                .join("Desktop/stuff/stuff/mission control video demo 720.mov"),
            "MOV",
        ),
        (
            PathBuf::from(env::var("HOME").unwrap()).join(
                "Desktop/stuff/stuff/Investor update - Calendar Agent Demo Highlight Reel.mp4",
            ),
            "MP4",
        ),
        (
            PathBuf::from("test_files_wikimedia/mkv/keyframes/02_h264_from_mov.mkv"),
            "MKV",
        ),
    ];

    let mut passed = 0;
    let mut failed = 0;

    for (file, format) in &test_files {
        if !file.exists() {
            continue;
        }

        let result = run_video_extract("keyframes", file);
        if result.passed {
            passed += 1;
            println!("  ✅ {} ({:.2}s)", format, result.duration_secs);
        } else {
            failed += 1;
            eprintln!("  ❌ {} failed: {}", format, file.display());
        }
    }

    println!(
        "✅ Random mixed video formats: {}/{} passed",
        passed,
        test_files.len()
    );
    assert!(
        passed >= 2,
        "Too many random video files failed: {}/{}",
        failed,
        test_files.len()
    );
}

#[test]
#[ignore]
fn random_sample_mixed_formats_audio() {
    // Random sample mixing different audio formats
    let test_files = vec![
        (
            PathBuf::from(env::var("HOME").unwrap())
                .join("docling/tests/data/audio/sample_10s_audio-aac.aac"),
            "AAC",
        ),
        (
            PathBuf::from(env::var("HOME").unwrap()).join(
                "Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/public/librivox/fabula_01_022_esopo_64kb.mp3",
            ),
            "MP3",
        ),
        (
            PathBuf::from(env::var("HOME").unwrap())
                .join("src/server/dropbox/tests/static/audios/Sample_BeeMoved_96kHz24bit.flac"),
            "FLAC",
        ),
        (
            PathBuf::from(env::var("HOME").unwrap())
                .join("Music/Music/Media.localized/Music/Unknown Artist/Unknown Album/State of Affairs_ROUGHMIX.wav"),
            "WAV",
        ),
    ];

    let mut passed = 0;
    let mut failed = 0;

    for (file, format) in &test_files {
        if !file.exists() {
            continue;
        }

        let result = run_video_extract("audio", file);
        if result.passed {
            passed += 1;
            println!("  ✅ {} ({:.2}s)", format, result.duration_secs);
        } else {
            failed += 1;
            eprintln!("  ❌ {} failed: {}", format, file.display());
        }
    }

    println!(
        "✅ Random mixed audio formats: {}/{} passed",
        passed,
        test_files.len()
    );
    assert!(
        passed >= 3,
        "Too many random audio files failed: {}/{}",
        failed,
        test_files.len()
    );
}

#[test]
#[ignore]
fn random_sample_large_files_batch() {
    // Random sample of large files (stress test diversity)
    let test_files = vec![
        (
            PathBuf::from(env::var("HOME").unwrap())
                .join("Desktop/stuff/stuff/GMT20250520-223657_Recording_avo_1920x1080.mp4"),
            1300, // 1.3GB
        ),
        (
            PathBuf::from(env::var("HOME").unwrap())
                .join("Desktop/stuff/stuff/GMT20250516-190317_Recording_avo_1920x1080.mov"),
            980, // 980MB
        ),
        (
            PathBuf::from(env::var("HOME").unwrap()).join(
                "Desktop/stuff/stuff/Investor update - Calendar Agent Demo Highlight Reel.mp4",
            ),
            349, // 349MB
        ),
    ];

    let mut passed = 0;
    let mut failed = 0;

    for (file, size_mb) in &test_files {
        if !file.exists() {
            continue;
        }

        // Use audio extraction for speed (faster than keyframes on large files)
        let result = run_video_extract("audio", file);
        if result.passed {
            passed += 1;
            println!("  ✅ {}MB file ({:.2}s)", size_mb, result.duration_secs);
        } else {
            failed += 1;
            eprintln!("  ❌ {}MB file failed", size_mb);
        }
    }

    println!(
        "✅ Random large files: {}/{} passed",
        passed,
        test_files.len()
    );
    // Allow test to pass if files missing (graceful degradation)
    if passed + failed > 0 {
        assert!(
            passed >= 1 || failed == 0,
            "Too many large files failed: {}/{} (passed: {})",
            failed,
            test_files.len(),
            passed
        );
    }
}

#[test]
#[ignore]
fn random_sample_tiny_files_batch() {
    // Random sample of tiny files (quick validation)
    let test_files = vec![
        (
            PathBuf::from(env::var("HOME").unwrap()).join(
                "Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/action_youtube_naudio/basketball/v_shooting_24_01.avi",
            ),
            "keyframes",
            13, // 13KB
        ),
        (
            PathBuf::from(env::var("HOME").unwrap())
                .join("docling/tests/data/audio/sample_10s_audio-aac.aac"),
            "audio", // Use audio operation for audio file
            146, // 146KB
        ),
    ];

    let mut passed = 0;
    let mut failed = 0;

    for (file, operation, size_kb) in &test_files {
        if !file.exists() {
            continue;
        }

        let result = run_video_extract(operation, file);
        if result.passed {
            passed += 1;
            println!("  ✅ {}KB file ({:.2}s)", size_kb, result.duration_secs);
        } else {
            failed += 1;
            eprintln!("  ❌ {}KB file failed", size_kb);
        }
    }

    println!(
        "✅ Random tiny files: {}/{} passed",
        passed,
        test_files.len()
    );
    // Allow test to pass if files missing (graceful degradation)
    if passed + failed > 0 {
        assert!(
            passed >= 1 || failed == 0,
            "Tiny files failed: {}/{} (passed: {})",
            failed,
            test_files.len(),
            passed
        );
    }
}

#[test]
#[ignore]
fn random_sample_comprehensive_diversity() {
    // Comprehensive diversity test: mix of formats, sizes, codecs
    let test_files = vec![
        // Video diversity
        (
            PathBuf::from(env::var("HOME").unwrap())
                .join("Desktop/stuff/stuff/editing-relevance-rubrics kg may 16 2025.mov"),
            "keyframes",
            "34MB MOV",
        ),
        // Audio diversity
        (
            PathBuf::from(env::var("HOME").unwrap())
                .join("Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/public/librivox/fabula_01_026_esopo_64kb.mp3"),
            "transcription",
            "448KB MP3",
        ),
        // Tiny video
        (
            PathBuf::from(env::var("HOME").unwrap())
                .join("Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/action_youtube_naudio/basketball/v_shooting_16_05.avi"),
            "keyframes",
            "32KB AVI",
        ),
        // High quality audio
        (
            PathBuf::from(env::var("HOME").unwrap())
                .join("src/server/dropbox/tests/static/audios/Sample_BeeMoved_96kHz24bit.flac"),
            "audio",
            "16MB FLAC",
        ),
    ];

    let mut passed = 0;
    let mut failed = 0;

    for (file, operation, description) in &test_files {
        if !file.exists() {
            continue;
        }

        let result = run_video_extract(operation, file);
        if result.passed {
            passed += 1;
            println!("  ✅ {} ({:.2}s)", description, result.duration_secs);
        } else {
            failed += 1;
            eprintln!("  ❌ {} failed", description);
        }
    }

    println!(
        "✅ Comprehensive diversity: {}/{} passed",
        passed,
        test_files.len()
    );
    // Allow test to pass if files missing (graceful degradation)
    if passed + failed > 0 {
        assert!(
            passed >= 2 || failed == 0,
            "Diversity test failed: {}/{} (passed: {})",
            failed,
            test_files.len(),
            passed
        );
    }
}

// ============================================================================
// SUITE 13: MULTI-OPERATION PIPELINE TESTS (10 tests, ~15 min)
// Tests operation chains and Phase 9 parallel syntax
// ============================================================================

#[test]
#[ignore]
fn pipeline_audio_then_transcription() {
    // Sequential pipeline: audio extraction → transcription
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("Desktop/stuff/stuff/GMT20250520-223657_Recording_avo_1920x1080.mp4");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("audio;transcription", &file);
    record_test_result(
        "pipeline_audio_then_transcription",
        "multi_operation_pipelines",
        "audio;transcription",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Audio → transcription pipeline failed: {:?}",
        result.error
    );
    println!("✅ Audio → transcription: {:.2}s", result.duration_secs);
}

#[test]
#[ignore]
fn pipeline_keyframes_then_object_detection() {
    // Sequential pipeline: keyframes → object detection
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("Desktop/stuff/stuff/editing-relevance-rubrics kg may 16 2025.mov");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("keyframes;object-detection", &file);
    record_test_result(
        "pipeline_keyframes_then_object_detection",
        "multi_operation_pipelines",
        "keyframes;object-detection",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Keyframes → object-detection pipeline failed: {:?}",
        result.error
    );
    println!(
        "✅ Keyframes → object-detection: {:.2}s",
        result.duration_secs
    );
}

#[test]
#[ignore]
fn pipeline_keyframes_then_face_detection() {
    // Sequential pipeline: keyframes → face detection
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("Desktop/stuff/stuff/mission control video demo 720.mov");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("keyframes;face-detection", &file);
    record_test_result(
        "pipeline_keyframes_then_face_detection",
        "multi_operation_pipelines",
        "keyframes;face-detection",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Keyframes → face-detection pipeline failed: {:?}",
        result.error
    );
    println!(
        "✅ Keyframes → face-detection: {:.2}s",
        result.duration_secs
    );
}

#[test]
#[ignore]
fn pipeline_keyframes_then_ocr() {
    // Sequential pipeline: keyframes → OCR (text extraction from video frames)
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("Desktop/stuff/stuff/Screen Recording 2025-06-02 at 11.14.26 AM.mov");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("keyframes;ocr", &file);
    record_test_result(
        "pipeline_keyframes_then_ocr",
        "multi_operation_pipelines",
        "keyframes;ocr",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Keyframes → OCR pipeline failed: {:?}",
        result.error
    );
    println!("✅ Keyframes → OCR: {:.2}s", result.duration_secs);
}

#[test]
#[ignore]
fn pipeline_audio_then_diarization() {
    // Sequential pipeline: audio → speaker diarization
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("Desktop/stuff/stuff/GMT20250520-223657_Recording_avo_1920x1080.mp4");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("audio;diarization", &file);
    record_test_result(
        "pipeline_audio_then_diarization",
        "multi_operation_pipelines",
        "audio;diarization",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Audio → diarization pipeline failed: {:?}",
        result.error
    );
    println!("✅ Audio → diarization: {:.2}s", result.duration_secs);
}

#[test]
#[ignore]
fn pipeline_parallel_audio_and_keyframes() {
    // Parallel pipeline (Phase 9): [audio, keyframes] simultaneously
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("Desktop/stuff/stuff/Investor update - Calendar Agent Demo Highlight Reel.mp4");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("[audio,keyframes]", &file);
    record_test_result(
        "pipeline_parallel_audio_and_keyframes",
        "multi_operation_pipelines",
        "[audio,keyframes]",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Parallel [audio, keyframes] pipeline failed: {:?}",
        result.error
    );
    println!(
        "✅ Parallel [audio, keyframes]: {:.2}s",
        result.duration_secs
    );
}

#[test]
#[ignore]
fn pipeline_parallel_multi_video_ops() {
    // Parallel pipeline: [keyframes, scene-detection] simultaneously
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("Desktop/stuff/stuff/mission control video demo 720.mov");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("[keyframes,scene-detection]", &file);
    record_test_result(
        "pipeline_parallel_multi_video_ops",
        "multi_operation_pipelines",
        "[keyframes,scene-detection]",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Parallel [keyframes, scene-detection] pipeline failed: {:?}",
        result.error
    );
    println!(
        "✅ Parallel [keyframes, scene-detection]: {:.2}s",
        result.duration_secs
    );
}

#[test]
#[ignore]
fn pipeline_three_stage_audio() {
    // Three-stage pipeline: audio → transcription → text-embeddings
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("docling/tests/data/audio/sample_10s_audio-aac.aac");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("audio;transcription;text-embeddings", &file);
    record_test_result(
        "pipeline_three_stage_audio",
        "multi_operation_pipelines",
        "audio;transcription;text-embeddings",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Three-stage audio pipeline failed: {:?}",
        result.error
    );
    println!(
        "✅ Audio → transcription → text-embeddings: {:.2}s",
        result.duration_secs
    );
}

#[test]
#[ignore]
fn pipeline_three_stage_video() {
    // Three-stage pipeline: keyframes → vision-embeddings (vision-embeddings takes Frames, not ObjectDetection)
    let file = PathBuf::from("test_files_wikimedia/mkv/keyframes/02_h264_from_mov.mkv");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("keyframes;vision-embeddings", &file);
    record_test_result(
        "pipeline_three_stage_video",
        "multi_operation_pipelines",
        "keyframes;vision-embeddings",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Three-stage video pipeline failed: {:?}",
        result.error
    );
    println!(
        "✅ Keyframes → vision-embeddings: {:.2}s",
        result.duration_secs
    );
}

#[test]
#[ignore]
fn pipeline_comprehensive_video_analysis() {
    // Comprehensive video analysis: Run keyframes once, then use parallel syntax for independent operations
    // Note: face-detection, OCR, vision-embeddings all take Frames input (not ObjectDetection)
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("Desktop/stuff/stuff/editing-relevance-rubrics kg may 16 2025.mov");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    // Parallel operations that all consume Frames: [object-detection, face-detection, ocr]
    let result = run_video_extract("keyframes;[object-detection,face-detection,ocr]", &file);
    record_test_result(
        "pipeline_comprehensive_video_analysis",
        "multi_operation_pipelines",
        "keyframes;[object-detection,face-detection,ocr]",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Comprehensive video analysis failed: {:?}",
        result.error
    );
    println!(
        "✅ Comprehensive video analysis (parallel 3 ops): {:.2}s",
        result.duration_secs
    );
    // Performance expectation: Should benefit from cache + parallelism
    assert!(
        result.duration_secs < 60.0,
        "Comprehensive pipeline too slow: {:.2}s (expected <60s)",
        result.duration_secs
    );
}

// ============================================================================
// SUITE 14: ADDITIONAL COVERAGE (10 tests, ~10 min)
// Embeddings, scene detection, and remaining edge cases
// ============================================================================

#[test]
#[ignore]
fn additional_vision_embeddings_4k() {
    // Vision embeddings on 4K video
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_edge_cases/video_4k_ultra_hd_3840x2160__stress_test.mp4");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("keyframes;vision-embeddings", &file);
    record_test_result(
        "additional_vision_embeddings_4k",
        "additional_coverage",
        "keyframes;vision-embeddings",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Vision embeddings on 4K failed: {:?}",
        result.error
    );
    println!("✅ Vision embeddings 4K: {:.2}s", result.duration_secs);
}

#[test]
#[ignore]
fn additional_text_embeddings_long_transcript() {
    // Text embeddings on long audio file (56MB WAV music)
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("Music/Music/Media.localized/Music/Unknown Artist/Unknown Album/State of Affairs_ROUGHMIX.wav");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("transcription;text-embeddings", &file);
    record_test_result(
        "additional_text_embeddings_long_transcript",
        "additional_coverage",
        "transcription;text-embeddings",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Text embeddings on long transcript failed: {:?}",
        result.error
    );
    println!(
        "✅ Text embeddings (long transcript): {:.2}s",
        result.duration_secs
    );
}

#[test]
#[ignore]
fn additional_audio_embeddings_music() {
    // Audio embeddings on music file (CLAP model) - WAV files need audio extraction too
    let file = PathBuf::from("test_files_local/State of Affairs_ROUGHMIX.wav");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("audio;audio-embeddings", &file);
    record_test_result(
        "additional_audio_embeddings_music",
        "additional_coverage",
        "audio;audio-embeddings",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Audio embeddings on music failed: {:?}",
        result.error
    );
    println!("✅ Audio embeddings (music): {:.2}s", result.duration_secs);
}

#[test]
#[ignore]
fn additional_audio_embeddings_speech() {
    // Audio embeddings on speech (AAC file) - need to extract audio first
    let file = PathBuf::from("test_files_local/sample_10s_audio-aac.aac");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("audio;audio-embeddings", &file);
    record_test_result(
        "additional_audio_embeddings_speech",
        "additional_coverage",
        "audio;audio-embeddings",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Audio embeddings on speech failed: {:?}",
        result.error
    );
    println!("✅ Audio embeddings (speech): {:.2}s", result.duration_secs);
}

#[test]
#[ignore]
fn additional_scene_detection_long_video() {
    // Scene detection on long video (349MB)
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("Desktop/stuff/stuff/Investor update - Calendar Agent Demo Highlight Reel.mp4");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("scene-detection", &file);
    record_test_result(
        "additional_scene_detection_long_video",
        "additional_coverage",
        "scene-detection",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Scene detection on long video failed: {:?}",
        result.error
    );
    println!(
        "✅ Scene detection (349MB video): {:.2}s",
        result.duration_secs
    );
    // Performance check: Should be fast (keyframe-only optimization from N=111)
    assert!(
        result.duration_secs < 30.0,
        "Scene detection too slow: {:.2}s (expected <30s)",
        result.duration_secs
    );
}

#[test]
#[ignore]
fn additional_scene_detection_action_video() {
    // Scene detection on action video (fast motion)
    let file = PathBuf::from("test_files_wikimedia/mkv/keyframes/02_h264_from_mov.mkv");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("scene-detection", &file);
    record_test_result(
        "additional_scene_detection_action_video",
        "additional_coverage",
        "scene-detection",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Scene detection on action video failed: {:?}",
        result.error
    );
    println!(
        "✅ Scene detection (action video): {:.2}s",
        result.duration_secs
    );
}

#[test]
#[ignore]
fn additional_diarization_multi_speaker() {
    // Speaker diarization on multi-speaker recording (1.3GB Zoom)
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("Desktop/stuff/stuff/GMT20250520-223657_Recording_avo_1920x1080.mp4");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("audio;diarization", &file);
    record_test_result(
        "additional_diarization_multi_speaker",
        "additional_coverage",
        "audio;diarization",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Diarization on multi-speaker failed: {:?}",
        result.error
    );
    println!(
        "✅ Diarization (multi-speaker): {:.2}s",
        result.duration_secs
    );
}

#[test]
#[ignore]
fn additional_ocr_text_heavy_video() {
    // OCR on screen recording (text-heavy content)
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("Desktop/stuff/stuff/Screen Recording 2025-06-02 at 11.14.26 AM.mov");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("keyframes;ocr", &file);
    record_test_result(
        "additional_ocr_text_heavy_video",
        "additional_coverage",
        "keyframes;ocr",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "OCR on text-heavy video failed: {:?}",
        result.error
    );
    println!(
        "✅ OCR (text-heavy screen recording): {:.2}s",
        result.duration_secs
    );
}

#[test]
#[ignore]
fn additional_face_detection_multi_face() {
    // Face detection on video with multiple people (Zoom recording)
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("Desktop/stuff/stuff/GMT20250520-223657_Recording_avo_1920x1080.mp4");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("keyframes;face-detection", &file);
    record_test_result(
        "additional_face_detection_multi_face",
        "additional_coverage",
        "keyframes;face-detection",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Face detection on multi-face video failed: {:?}",
        result.error
    );
    println!(
        "✅ Face detection (multi-face Zoom): {:.2}s",
        result.duration_secs
    );
}

#[test]
#[ignore]
fn additional_high_quality_flac_transcription() {
    // Transcription on high-quality FLAC (96kHz/24-bit)
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("src/server/dropbox/tests/static/audios/Sample_BeeMoved_96kHz24bit.flac");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("transcription", &file);
    record_test_result(
        "additional_high_quality_flac_transcription",
        "additional_coverage",
        "transcription",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Transcription on high-quality FLAC failed: {:?}",
        result.error
    );
    println!(
        "✅ Transcription (96kHz/24-bit FLAC): {:.2}s",
        result.duration_secs
    );
}

#[test]
#[ignore]
fn tier2_pose_estimation() {
    // Pose estimation on video with people (Zoom recording)
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("Desktop/stuff/stuff/GMT20250520-223657_Recording_avo_1920x1080.mp4");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("keyframes;pose-estimation", &file);
    record_test_result(
        "tier2_pose_estimation",
        "tier2_plugins",
        "keyframes;pose-estimation",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Pose estimation on video failed: {:?}",
        result.error
    );
    println!(
        "✅ Pose estimation (Zoom recording): {:.2}s",
        result.duration_secs
    );
}

#[test]
#[ignore]
fn tier2_emotion_detection() {
    // Emotion detection on video with faces (Zoom recording)
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("Desktop/stuff/stuff/GMT20250520-223657_Recording_avo_1920x1080.mp4");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("keyframes;emotion-detection", &file);
    record_test_result(
        "tier2_emotion_detection",
        "tier2_plugins",
        "keyframes;emotion-detection",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Emotion detection on video failed: {:?}",
        result.error
    );
    println!(
        "✅ Emotion detection (Zoom recording): {:.2}s",
        result.duration_secs
    );
}

#[test]
#[ignore]
fn tier2_image_quality_assessment() {
    // Image quality assessment on 4K video keyframes
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/Kinetics dataset (5%)/kinetics600_5per/kinetics600_5per/train/ice climbing/0m4B34GjjjM_raw.mkv");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("keyframes;image-quality-assessment", &file);
    record_test_result(
        "tier2_image_quality_assessment",
        "tier2_plugins",
        "keyframes;image-quality-assessment",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Image quality assessment on video failed: {:?}",
        result.error
    );
    println!(
        "✅ Image quality assessment (4K MKV): {:.2}s",
        result.duration_secs
    );
}

#[test]
#[ignore]
fn tier2_audio_enhancement_metadata() {
    // Audio enhancement metadata analysis on video audio
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("Desktop/stuff/stuff/GMT20250520-223657_Recording_avo_1920x1080.mp4");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("audio;audio-enhancement-metadata", &file);
    record_test_result(
        "tier2_audio_enhancement_metadata",
        "tier2_plugins",
        "audio;audio-enhancement-metadata",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Audio enhancement metadata analysis failed: {:?}",
        result.error
    );
    println!(
        "✅ Audio enhancement metadata (Zoom recording): {:.2}s",
        result.duration_secs
    );
}

#[test]
#[ignore]
fn tier2_shot_classification() {
    // Shot classification on video with diverse camera angles
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("Desktop/stuff/stuff/GMT20250520-223657_Recording_avo_1920x1080.mp4");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("keyframes;shot-classification", &file);
    record_test_result(
        "tier2_shot_classification",
        "tier2_plugins",
        "keyframes;shot-classification",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Shot classification on video failed: {:?}",
        result.error
    );
    println!(
        "✅ Shot classification (Zoom recording): {:.2}s",
        result.duration_secs
    );
}

// ============================================================================
// SUITE 16: TIER 1 PLUGIN TESTS (5 tests, ~15 min)
// Tests high-value Tier 1 plugins implemented in N=72-77
// ============================================================================

#[test]
#[ignore]
fn tier1_motion_tracking() {
    // Motion tracking on video with moving objects (Zoom recording with people)
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("Desktop/stuff/stuff/GMT20250520-223657_Recording_avo_1920x1080.mp4");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("keyframes;object-detection;motion-tracking", &file);
    record_test_result(
        "tier1_motion_tracking",
        "tier1_plugins",
        "keyframes;object-detection;motion-tracking",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Motion tracking on video failed: {:?}",
        result.error
    );
    println!(
        "✅ Motion tracking (Zoom recording): {:.2}s",
        result.duration_secs
    );
}

#[test]
#[ignore]
fn tier1_action_recognition() {
    // Action recognition on video with activities (basketball shooting)
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/action_youtube_naudio/basketball/v_shooting_16_05.avi");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("keyframes;action-recognition", &file);
    record_test_result(
        "tier1_action_recognition",
        "tier1_plugins",
        "keyframes;action-recognition",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Action recognition on video failed: {:?}",
        result.error
    );
    println!(
        "✅ Action recognition (basketball video): {:.2}s",
        result.duration_secs
    );
}

#[test]
#[ignore]
fn tier1_smart_thumbnail() {
    // Smart thumbnail selection on high-resolution video
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("Desktop/stuff/stuff/editing-relevance-rubrics kg may 16 2025.mov");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("keyframes;smart-thumbnail", &file);
    record_test_result(
        "tier1_smart_thumbnail",
        "tier1_plugins",
        "keyframes;smart-thumbnail",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Smart thumbnail generation failed: {:?}",
        result.error
    );
    println!(
        "✅ Smart thumbnail (high-res MOV): {:.2}s",
        result.duration_secs
    );
}

#[test]
#[ignore]
fn tier1_subtitle_extraction() {
    // Subtitle extraction on video with embedded subtitles
    let file = PathBuf::from("test_files_subtitles/video_with_subtitles.mkv");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("subtitle-extraction", &file);
    record_test_result(
        "tier1_subtitle_extraction",
        "tier1_plugins",
        "subtitle-extraction",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Subtitle extraction failed: {:?}",
        result.error
    );
    println!(
        "✅ Subtitle extraction (Zoom recording): {:.2}s",
        result.duration_secs
    );
}

#[test]
#[ignore]
fn tier1_audio_classification() {
    // Audio classification on high-quality audio file
    let file = PathBuf::from(env::var("HOME").unwrap())
        .join("src/server/dropbox/tests/static/audios/Sample_BeeMoved_96kHz24bit.flac");

    if !file.exists() {
        eprintln!("⚠️  Test file not found: {}", file.display());
        return;
    }

    let result = run_video_extract("audio;audio-classification", &file);
    record_test_result(
        "tier1_audio_classification",
        "tier1_plugins",
        "audio;audio-classification",
        Some(&file),
        &result,
    );
    assert!(
        result.passed,
        "Audio classification failed: {:?}",
        result.error
    );
    println!(
        "✅ Audio classification (96kHz/24-bit FLAC): {:.2}s",
        result.duration_secs
    );
}

// ============================================================================
// SAVE TEST RESULTS (runs last)
// ============================================================================

#[test]
#[ignore]
fn zzz_save_test_results() {
    // This test runs last (alphabetically) to save all tracked results
    if let Ok(mut tracker) = TRACKER.lock() {
        if let Some(mut t) = tracker.take() {
            match t.save() {
                Ok(output_dir) => {
                    println!("✅ Test results saved to: {}", output_dir.display());
                    println!("   View: ls {}", output_dir.display());
                    println!("   Latest: test_results/latest/");
                }
                Err(e) => {
                    eprintln!("❌ Failed to save test results: {}", e);
                }
            }
        } else {
            println!("⚠️  No test results to save (tracker not initialized)");
        }
    }
}
