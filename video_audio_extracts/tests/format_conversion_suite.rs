//! Format Conversion Test Suite
//!
//! Automated tests that verify format conversion works for various input formats.
//!
//! Run: cargo test --test format_conversion_suite -- --ignored --test-threads=1
//!
//! These tests verify the format_conversion plugin can handle different input formats
//! and convert them successfully. This tests INPUT format support, ensuring all video
//! and audio formats are accepted and processed.
//!
//! Goals:
//! - Test all input video formats are accepted (MP4, MOV, AVI, MKV, WebM, FLV, etc.)
//! - Test all input audio formats are accepted (WAV, MP3, FLAC, AAC, etc.)
//! - Test various conversion presets (web, mobile, archive, compatible, etc.)
//! - Verify output files are valid (can be read by FFprobe)
//! - Measure conversion speed and compression ratios
//!
//! Note: All tests use the "web" preset (H.264/AAC/MP4) for consistency. The focus
//! is on testing INPUT format support, not output configuration options.

use std::process::Command;
use std::path::Path;

mod common;

/// Helper: Run format conversion and verify the output
/// Uses "web" preset (H.264/AAC/MP4) by default for all tests
fn run_and_verify_conversion(
    test_name: &str,
    input_file: &str,
    preset: &str,
) {
    println!("\n=== Format Conversion Test: {} ===", test_name);
    println!("Input: {}", input_file);
    println!("Preset: {}", preset);

    // 1. Clean up any previous debug output
    let _ = std::fs::remove_dir_all("./debug_output");

    // 2. Build operation string
    let ops = format!("format-conversion:preset={}", preset);

    // 3. Run video-extract
    let output = Command::new("./target/release/video-extract")
        .args(["debug", "--ops", &ops, input_file])
        .output()
        .expect("Failed to run video-extract");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!(
            "video-extract failed for {}\nstderr: {}",
            test_name, stderr
        );
    }

    // 4. Find output JSON (format: stage_00_format_conversion.json)
    let output_json = "./debug_output/stage_00_format_conversion.json";

    assert!(
        Path::new(output_json).exists(),
        "Output JSON not found: {}",
        output_json
    );

    // 5. Parse the JSON to get output file path
    let json_content = std::fs::read_to_string(output_json)
        .expect("Failed to read output JSON");

    let result: serde_json::Value = serde_json::from_str(&json_content)
        .expect("Failed to parse JSON");

    let output_path = result["output_path"]
        .as_str()
        .expect("Missing output_path in JSON");

    let compression_ratio = result["compression_ratio"]
        .as_f64()
        .expect("Missing compression_ratio in JSON");

    let input_size = result["input_size"]
        .as_u64()
        .expect("Missing input_size in JSON");

    let output_size = result["output_size"]
        .as_u64()
        .expect("Missing output_size in JSON");

    println!("Output path: {}", output_path);
    println!("Input size: {} bytes", input_size);
    println!("Output size: {} bytes", output_size);
    println!("Compression ratio: {:.2}%", compression_ratio * 100.0);

    // 6. Verify output file exists
    assert!(
        Path::new(output_path).exists(),
        "Converted output file not found: {}",
        output_path
    );

    // 7. Verify output file is valid using ffprobe
    let ffprobe_output = Command::new("ffprobe")
        .args([
            "-v", "error",
            "-show_entries", "format=format_name",
            "-of", "default=noprint_wrappers=1:nokey=1",
            output_path,
        ])
        .output()
        .expect("Failed to run ffprobe");

    assert!(
        ffprobe_output.status.success(),
        "ffprobe failed to read converted file: {}",
        String::from_utf8_lossy(&ffprobe_output.stderr)
    );

    let detected_format = String::from_utf8_lossy(&ffprobe_output.stdout);
    println!("Detected format: {}", detected_format.trim());

    // 8. Verify the container is valid (ffprobe succeeded is enough - it means the file is valid)
    assert!(
        !detected_format.trim().is_empty(),
        "ffprobe returned empty format for {}",
        output_path
    );

    // 9. Clean up
    let _ = std::fs::remove_file(output_path);
    let _ = std::fs::remove_dir_all("./debug_output");

    println!("✅ Test {} passed!", test_name);
}

// ============================================================================
// VIDEO FORMAT CONVERSION TESTS
// ============================================================================

#[test]
#[ignore]
fn convert_avi() {
    run_and_verify_conversion(
        "convert_avi",
        "test_edge_cases/format_test_avi.avi",
        "web",
    );
}

#[test]
#[ignore]
fn convert_mp4() {
    run_and_verify_conversion(
        "convert_mp4",
        "test_edge_cases/video_single_frame_only__minimal.mp4",
        "web",
    );
}

#[test]
#[ignore]
fn convert_mp4_mobile_preset() {
    run_and_verify_conversion(
        "convert_mp4_mobile",
        "test_edge_cases/video_tiny_64x64_resolution__scaling_test.mp4",
        "mobile",
    );
}

#[test]
#[ignore]
fn convert_mp4_archive_preset() {
    run_and_verify_conversion(
        "convert_mp4_archive",
        "test_edge_cases/video_variable_framerate_vfr__timing_test.mp4",
        "archive",
    );
}

#[test]
#[ignore]
fn convert_flv() {
    run_and_verify_conversion(
        "convert_flv",
        "test_edge_cases/format_test_flv.flv",
        "web",
    );
}

#[test]
#[ignore]
fn convert_3gp() {
    run_and_verify_conversion(
        "convert_3gp",
        "test_edge_cases/format_test_3gp.3gp",
        "web",
    );
}

#[test]
#[ignore]
fn convert_wmv() {
    run_and_verify_conversion(
        "convert_wmv",
        "test_edge_cases/format_test_wmv.wmv",
        "web",
    );
}

#[test]
#[ignore]
fn convert_ogv() {
    run_and_verify_conversion(
        "convert_ogv",
        "test_edge_cases/format_test_ogv.ogv",
        "web",
    );
}

#[test]
#[ignore]
fn convert_m4v() {
    run_and_verify_conversion(
        "convert_m4v",
        "test_edge_cases/format_test_m4v.m4v",
        "web",
    );
}

// ============================================================================
// AUDIO FORMAT CONVERSION TESTS
// ============================================================================

#[test]
#[ignore]
fn convert_wav() {
    run_and_verify_conversion(
        "convert_wav",
        "test_edge_cases/audio_mono_single_channel__channel_test.wav",
        "audioonly",
    );
}

#[test]
#[ignore]
fn convert_mp3() {
    run_and_verify_conversion(
        "convert_mp3",
        "test_edge_cases/audio_lowquality_16kbps__compression_test.mp3",
        "audioonly",
    );
}

#[test]
#[ignore]
fn convert_ogg() {
    run_and_verify_conversion(
        "convert_ogg",
        "test_edge_cases/format_test_ogg.ogg",
        "audioonly",
    );
}

#[test]
#[ignore]
fn convert_opus() {
    run_and_verify_conversion(
        "convert_opus",
        "test_edge_cases/format_test_opus.opus",
        "audioonly",
    );
}

// ============================================================================
// PRESET CONVERSION TESTS
// ============================================================================

#[test]
#[ignore]
fn convert_with_web_preset() {
    run_and_verify_conversion(
        "preset_web",
        "test_edge_cases/video_4k_ultra_hd_3840x2160__stress_test.mp4",
        "web",
    );
}

#[test]
#[ignore]
fn convert_with_mobile_preset() {
    run_and_verify_conversion(
        "preset_mobile",
        "test_edge_cases/video_variable_framerate_vfr__timing_test.mp4",
        "mobile",
    );
}

#[test]
#[ignore]
fn convert_with_archive_preset() {
    run_and_verify_conversion(
        "preset_archive",
        "test_edge_cases/format_test_avi.avi",
        "archive",
    );
}

#[test]
#[ignore]
fn convert_with_compatible_preset() {
    run_and_verify_conversion(
        "preset_compatible",
        "test_edge_cases/format_test_avi.avi",
        "compatible",
    );
}

#[test]
#[ignore]
fn convert_with_lowbandwidth_preset() {
    run_and_verify_conversion(
        "preset_lowbandwidth",
        "test_edge_cases/video_4k_ultra_hd_3840x2160__stress_test.mp4",
        "lowbandwidth",
    );
}

#[test]
#[ignore]
fn convert_with_copy_preset() {
    run_and_verify_conversion(
        "preset_copy",
        "test_edge_cases/format_test_avi.avi",
        "copy",
    );
}

// ============================================================================
// ADDITIONAL VIDEO FORMAT CONVERSION TESTS
// ============================================================================

#[test]
#[ignore]
fn convert_mov() {
    run_and_verify_conversion(
        "convert_mov",
        "test_edge_cases/video_no_audio_stream__error_test.mov",
        "web",
    );
}

#[test]
#[ignore]
fn convert_mkv() {
    run_and_verify_conversion(
        "convert_mkv",
        "test_files_wikimedia/mkv/action-recognition/01_h264_from_mp4.mkv",
        "web",
    );
}

#[test]
#[ignore]
fn convert_webm() {
    run_and_verify_conversion(
        "convert_webm",
        "test_edge_cases/video_single_frame_only__minimal.webm",
        "web",
    );
}

#[test]
#[ignore]
fn convert_mxf() {
    run_and_verify_conversion(
        "convert_mxf",
        "test_files_wikimedia/mxf/action-recognition/C0023S01.mxf",
        "web",
    );
}

#[test]
#[ignore]
fn convert_ts() {
    run_and_verify_conversion(
        "convert_ts",
        "test_files_streaming_hls_dash/hls_01_basic/segment_000.ts",
        "web",
    );
}

#[test]
#[ignore]
fn convert_vob() {
    run_and_verify_conversion(
        "convert_vob",
        "test_files_wikimedia/vob/action-recognition/03_test.vob",
        "web",
    );
}

#[test]
#[ignore]
fn convert_rm() {
    run_and_verify_conversion(
        "convert_rm",
        "test_files_wikimedia/rm/action-recognition/05_sample_1280x720.rm",
        "web",
    );
}

#[test]
#[ignore]
fn convert_asf() {
    run_and_verify_conversion(
        "convert_asf",
        "test_files_wikimedia/asf/action-recognition/02_elephant.asf",
        "web",
    );
}

#[test]
#[ignore]
fn convert_dv() {
    run_and_verify_conversion(
        "convert_dv",
        "test_files_wikimedia/dv/action-recognition/01_shots0000.dv",
        "web",
    );
}

#[test]
#[ignore]
fn convert_f4v() {
    run_and_verify_conversion(
        "convert_f4v",
        "test_files_video_formats_dpx_gxf_f4v/01_f4v_h264.f4v",
        "web",
    );
}

// ============================================================================
// ADDITIONAL AUDIO FORMAT CONVERSION TESTS
// ============================================================================

#[test]
#[ignore]
fn convert_flac() {
    run_and_verify_conversion(
        "convert_flac",
        "test_files_wikimedia/flac/audio-classification/04_Aina_zilizo_hatarini.flac.flac",
        "audioonly",
    );
}

#[test]
#[ignore]
fn convert_m4a() {
    run_and_verify_conversion(
        "convert_m4a",
        "test_files_wikimedia/alac/audio-classification/01_rodzaje_sygnalow.m4a",
        "audioonly",
    );
}

#[test]
#[ignore]
fn convert_wma() {
    run_and_verify_conversion(
        "convert_wma",
        "test_files_wikimedia/wma/audio-classification/01_bangles.wma",
        "audioonly",
    );
}

#[test]
#[ignore]
fn convert_amr() {
    run_and_verify_conversion(
        "convert_amr",
        "test_files_wikimedia/amr/audio-classification/01_sample.amr",
        "audioonly",
    );
}

#[test]
#[ignore]
fn convert_aac() {
    run_and_verify_conversion(
        "convert_aac",
        "test_files_local/sample_10s_audio-aac.aac",
        "audioonly",
    );
}

#[test]
#[ignore]
fn convert_ac3() {
    run_and_verify_conversion(
        "convert_ac3",
        "test_files_wikimedia/ac3/audio-classification/04_test.ac3",
        "audioonly",
    );
}

#[test]
#[ignore]
fn convert_dts() {
    run_and_verify_conversion(
        "convert_dts",
        "test_files_wikimedia/dts/audio-classification/03_test.dts",
        "audioonly",
    );
}

#[test]
#[ignore]
fn convert_ape() {
    run_and_verify_conversion(
        "convert_ape",
        "test_files_wikimedia/ape/audio-classification/01_concret_vbAccelerator.ape",
        "audioonly",
    );
}

#[test]
#[ignore]
fn convert_tta() {
    run_and_verify_conversion(
        "convert_tta",
        "test_files_legacy_audio/tta/03_test.tta",
        "audioonly",
    );
}

#[test]
#[ignore]
fn convert_wv() {
    run_and_verify_conversion(
        "convert_wv",
        "test_files_wikimedia/wavpack/audio-classification/01_premsa_version.wv",
        "audioonly",
    );
}

#[test]
#[ignore]
fn convert_mpc() {
    run_and_verify_conversion(
        "convert_mpc",
        "test_files_audio_formats_musepack/01_pumpkin.mpc",
        "audioonly",
    );
}

#[test]
#[ignore]
fn convert_au() {
    run_and_verify_conversion(
        "convert_au",
        "test_files_legacy_audio/au/garelka.au",
        "audioonly",
    );
}
