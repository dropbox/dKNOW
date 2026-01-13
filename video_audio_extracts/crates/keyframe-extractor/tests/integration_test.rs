use std::path::PathBuf;
use tempfile::TempDir;
use video_audio_keyframe::{extract_keyframes, KeyframeExtractor};

const TEST_VIDEO_DIR: &str = "/Users/ayates/docling/tests/data/audio";

fn test_video_path(filename: &str) -> PathBuf {
    PathBuf::from(TEST_VIDEO_DIR).join(filename)
}

#[test]
fn test_extract_keyframes_mp4() {
    let video_path = test_video_path("sample_10s_video-mp4.mp4");
    let temp_dir = TempDir::new().unwrap();

    let config = KeyframeExtractor {
        interval: 1.0,
        max_keyframes: 100,
        similarity_threshold: 10,
        thumbnail_sizes: vec![(320, 240)],
        output_dir: temp_dir.path().to_path_buf(),
        use_ffmpeg_cli: false,
    };

    let result = extract_keyframes(&video_path, config);
    assert!(result.is_ok(), "Failed to extract keyframes: {result:?}");

    let keyframes = result.unwrap();
    assert!(!keyframes.is_empty(), "No keyframes extracted");

    // Verify keyframe properties
    for keyframe in &keyframes {
        assert!(keyframe.timestamp >= 0.0);
        assert!(keyframe.hash > 0);
        assert!(keyframe.sharpness >= 0.0);
        assert!(!keyframe.thumbnail_paths.is_empty());

        // Verify thumbnail exists
        for (size_key, path) in &keyframe.thumbnail_paths {
            assert!(path.exists(), "Thumbnail not found: {path:?}");
            println!(
                "Keyframe at {:.2}s: hash={}, sharpness={:.2}, thumbnail={}",
                keyframe.timestamp, keyframe.hash, keyframe.sharpness, size_key
            );
        }
    }
}

#[test]
fn test_extract_keyframes_mov() {
    let video_path = test_video_path("sample_10s_video-quicktime.mov");
    let temp_dir = TempDir::new().unwrap();

    let config = KeyframeExtractor {
        interval: 2.0,
        max_keyframes: 50,
        similarity_threshold: 10,
        thumbnail_sizes: vec![(640, 480)],
        output_dir: temp_dir.path().to_path_buf(),
        use_ffmpeg_cli: false,
    };

    let result = extract_keyframes(&video_path, config);
    assert!(result.is_ok(), "Failed to extract keyframes: {result:?}");

    let keyframes = result.unwrap();
    assert!(!keyframes.is_empty(), "No keyframes extracted");
}

#[test]
fn test_extract_keyframes_avi() {
    let video_path = test_video_path("sample_10s_video-avi.avi");
    let temp_dir = TempDir::new().unwrap();

    let config = KeyframeExtractor {
        interval: 1.0,
        max_keyframes: 100,
        similarity_threshold: 10,
        thumbnail_sizes: vec![(320, 240)],
        output_dir: temp_dir.path().to_path_buf(),
        use_ffmpeg_cli: false,
    };

    let result = extract_keyframes(&video_path, config);
    assert!(result.is_ok(), "Failed to extract keyframes: {result:?}");

    let keyframes = result.unwrap();
    assert!(!keyframes.is_empty(), "No keyframes extracted");
}

#[test]
fn test_extract_keyframes_multi_resolution() {
    let video_path = test_video_path("sample_10s_video-mp4.mp4");
    let temp_dir = TempDir::new().unwrap();

    let config = KeyframeExtractor {
        interval: 2.0,
        max_keyframes: 100,
        similarity_threshold: 10,
        thumbnail_sizes: vec![(160, 120), (320, 240), (640, 480)],
        output_dir: temp_dir.path().to_path_buf(),
        use_ffmpeg_cli: false,
    };

    let result = extract_keyframes(&video_path, config);
    assert!(result.is_ok(), "Failed to extract keyframes: {result:?}");

    let keyframes = result.unwrap();
    assert!(!keyframes.is_empty(), "No keyframes extracted");

    // Verify all resolutions are generated
    for keyframe in &keyframes {
        assert_eq!(
            keyframe.thumbnail_paths.len(),
            3,
            "Expected 3 thumbnails, got {}",
            keyframe.thumbnail_paths.len()
        );
        assert!(keyframe.thumbnail_paths.contains_key("160x120"));
        assert!(keyframe.thumbnail_paths.contains_key("320x240"));
        assert!(keyframe.thumbnail_paths.contains_key("640x480"));
    }
}

#[test]
fn test_extract_keyframes_with_interval() {
    let video_path = test_video_path("sample_10s_video-mp4.mp4");
    let temp_dir = TempDir::new().unwrap();

    // Test with 2-second interval
    let config = KeyframeExtractor {
        interval: 2.0,
        max_keyframes: 100,
        similarity_threshold: 10,
        thumbnail_sizes: vec![(320, 240)],
        output_dir: temp_dir.path().to_path_buf(),
        use_ffmpeg_cli: false,
    };

    let result = extract_keyframes(&video_path, config);
    assert!(result.is_ok());

    let keyframes = result.unwrap();

    // Verify interval constraint
    for i in 1..keyframes.len() {
        let time_diff = keyframes[i].timestamp - keyframes[i - 1].timestamp;
        assert!(
            time_diff >= 2.0,
            "Keyframes too close: {time_diff:.2}s apart (expected >= 2.0s)"
        );
    }
}

#[test]
fn test_extract_keyframes_max_limit() {
    let video_path = test_video_path("sample_10s_video-mp4.mp4");
    let temp_dir = TempDir::new().unwrap();

    // Set max to 3 keyframes
    let config = KeyframeExtractor {
        interval: 0.1, // Very small interval
        max_keyframes: 3,
        similarity_threshold: 10,
        thumbnail_sizes: vec![(320, 240)],
        output_dir: temp_dir.path().to_path_buf(),
        use_ffmpeg_cli: false,
    };

    let result = extract_keyframes(&video_path, config);
    assert!(result.is_ok());

    let keyframes = result.unwrap();
    assert!(
        keyframes.len() <= 3,
        "Expected at most 3 keyframes, got {}",
        keyframes.len()
    );
}

#[test]
fn test_keyframe_deduplication() {
    let video_path = test_video_path("sample_10s_video-mp4.mp4");
    let temp_dir = TempDir::new().unwrap();

    // Use very low similarity threshold to enforce strict deduplication
    let config = KeyframeExtractor {
        interval: 0.1,
        max_keyframes: 100,
        similarity_threshold: 5, // Very strict
        thumbnail_sizes: vec![(320, 240)],
        output_dir: temp_dir.path().to_path_buf(),
        use_ffmpeg_cli: false,
    };

    let result = extract_keyframes(&video_path, config);
    assert!(result.is_ok());

    let keyframes = result.unwrap();

    // Verify no duplicate hashes
    let mut hashes = std::collections::HashSet::new();
    for keyframe in &keyframes {
        let inserted = hashes.insert(keyframe.hash);
        assert!(inserted, "Duplicate hash found: {}", keyframe.hash);
    }
}

#[test]
fn test_extract_keyframes_preview_preset() {
    let video_path = test_video_path("sample_10s_video-mp4.mp4");
    let temp_dir = TempDir::new().unwrap();

    let mut config = KeyframeExtractor::for_preview();
    config.output_dir = temp_dir.path().to_path_buf();

    let result = extract_keyframes(&video_path, config);
    assert!(result.is_ok());

    let keyframes = result.unwrap();
    assert!(!keyframes.is_empty());
}

#[test]
fn test_extract_keyframes_analysis_preset() {
    let video_path = test_video_path("sample_10s_video-mp4.mp4");
    let temp_dir = TempDir::new().unwrap();

    let mut config = KeyframeExtractor::for_analysis();
    config.output_dir = temp_dir.path().to_path_buf();

    let result = extract_keyframes(&video_path, config);
    assert!(result.is_ok());

    let keyframes = result.unwrap();
    assert!(!keyframes.is_empty());

    // Analysis preset should generate multiple thumbnail resolutions
    for keyframe in &keyframes {
        assert_eq!(
            keyframe.thumbnail_paths.len(),
            3,
            "Expected 3 thumbnail resolutions"
        );
    }
}
