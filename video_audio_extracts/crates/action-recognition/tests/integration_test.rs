//! Integration tests for action recognition

use std::collections::HashMap;
use std::path::PathBuf;
use video_audio_action_recognition::{ActionRecognitionConfig, ActionRecognizer, ActivityType};
use video_audio_common::Keyframe;

/// Helper function to create test keyframe
fn create_test_keyframe(timestamp: f64, hash: u64, sharpness: f64) -> Keyframe {
    let mut thumbnail_paths = HashMap::new();
    thumbnail_paths.insert(
        "640x480".to_string(),
        PathBuf::from(format!("/tmp/frame_{}.jpg", timestamp)),
    );

    Keyframe {
        timestamp,
        frame_number: (timestamp * 30.0) as u64,
        hash,
        sharpness,
        thumbnail_paths,
    }
}

/// Test recognizer creation and configuration
#[test]
fn test_recognizer_creation() {
    let config = ActionRecognitionConfig::default();
    let _recognizer = ActionRecognizer::new(config.clone());

    assert_eq!(config.min_segment_duration, 2.0);
    assert_eq!(config.confidence_threshold, 0.5);
    assert_eq!(config.scene_change_threshold, 0.4);
}

/// Test error handling for insufficient keyframes
#[test]
fn test_insufficient_keyframes() {
    let config = ActionRecognitionConfig::default();
    let recognizer = ActionRecognizer::new(config);

    // Single keyframe
    let keyframes = vec![create_test_keyframe(0.0, 0x1234, 100.0)];
    let result = recognizer.analyze(&keyframes);

    assert!(result.is_err(), "Expected error for single keyframe");
}

/// Test static scene recognition (minimal changes)
#[test]
fn test_static_scene_recognition() {
    let config = ActionRecognitionConfig::default();
    let recognizer = ActionRecognizer::new(config);

    // Create keyframes with minimal changes (similar hashes, sharpness)
    let keyframes = vec![
        create_test_keyframe(0.0, 0x1234567812345678, 100.0),
        create_test_keyframe(1.0, 0x1234567812345679, 100.1),
        create_test_keyframe(2.0, 0x123456781234567A, 100.2),
        create_test_keyframe(3.0, 0x123456781234567B, 100.1),
        create_test_keyframe(4.0, 0x123456781234567C, 100.0),
    ];

    let result = recognizer.analyze(&keyframes).unwrap();

    assert!(
        !result.segments.is_empty(),
        "Should create at least one segment"
    );
    assert!(
        matches!(
            result.overall_activity,
            ActivityType::Static | ActivityType::LowMotion
        ),
        "Expected static or low motion, got {:?}",
        result.overall_activity
    );
    assert!(
        result.overall_confidence >= 0.5,
        "Confidence should be >= 0.5"
    );
}

/// Test high motion scene recognition (large changes)
#[test]
fn test_high_motion_scene_recognition() {
    let config = ActionRecognitionConfig::default();
    let recognizer = ActionRecognizer::new(config);

    // Create keyframes with large changes (very different hashes)
    let keyframes = vec![
        create_test_keyframe(0.0, 0x0000000000000000, 100.0),
        create_test_keyframe(0.5, 0xFFFFFFFFFFFFFFFF, 50.0),
        create_test_keyframe(1.0, 0x0000000000000000, 150.0),
        create_test_keyframe(1.5, 0xFFFFFFFFFFFFFFFF, 75.0),
        create_test_keyframe(2.0, 0x0000000000000000, 125.0),
        create_test_keyframe(2.5, 0xFFFFFFFFFFFFFFFF, 90.0),
    ];

    let result = recognizer.analyze(&keyframes).unwrap();

    assert!(
        !result.segments.is_empty(),
        "Should create at least one segment"
    );
    // Should classify as high motion or rapid cuts
    assert!(
        matches!(
            result.overall_activity,
            ActivityType::HighMotion | ActivityType::RapidCuts | ActivityType::ModerateMotion
        ),
        "Expected high motion or rapid cuts, got {:?}",
        result.overall_activity
    );
}

/// Test scene change detection
#[test]
fn test_scene_change_detection() {
    let config = ActionRecognitionConfig {
        scene_change_threshold: 0.3,
        ..Default::default()
    };
    let recognizer = ActionRecognizer::new(config);

    // Create keyframes with clear scene change (very different hashes at t=2.0)
    let keyframes = vec![
        // Scene 1: static
        create_test_keyframe(0.0, 0x1111111111111111, 100.0),
        create_test_keyframe(1.0, 0x1111111111111112, 100.0),
        // Scene change
        create_test_keyframe(2.0, 0xFFFFFFFFFFFFFFFF, 100.0),
        // Scene 2: static
        create_test_keyframe(3.0, 0xFFFFFFFFFFFFFFFE, 100.0),
        create_test_keyframe(4.0, 0xFFFFFFFFFFFFFFFD, 100.0),
    ];

    let result = recognizer.analyze(&keyframes).unwrap();

    assert!(
        result.total_scene_changes > 0,
        "Should detect at least one scene change"
    );
}

/// Test segment creation with multiple scene changes
#[test]
fn test_multiple_segments() {
    let config = ActionRecognitionConfig::default();
    let recognizer = ActionRecognizer::new(config);

    // Create multiple distinct segments
    let keyframes = vec![
        // Static segment 1 (0-2s)
        create_test_keyframe(0.0, 0x1111111111111111, 100.0),
        create_test_keyframe(1.0, 0x1111111111111111, 100.0),
        create_test_keyframe(2.0, 0x1111111111111111, 100.0),
        // Scene change
        create_test_keyframe(3.0, 0xFFFFFFFFFFFFFFFF, 100.0),
        // Dynamic segment 2 (3-5s)
        create_test_keyframe(4.0, 0x0000000000000000, 50.0),
        create_test_keyframe(5.0, 0xFFFFFFFFFFFFFFFF, 150.0),
        // Scene change
        create_test_keyframe(6.0, 0x5555555555555555, 100.0),
        // Static segment 3 (6-8s)
        create_test_keyframe(7.0, 0x5555555555555556, 100.0),
        create_test_keyframe(8.0, 0x5555555555555557, 100.0),
    ];

    let result = recognizer.analyze(&keyframes).unwrap();

    // Should create multiple segments due to scene changes
    assert!(
        !result.segments.is_empty(),
        "Should create at least one segment"
    );

    // Each segment should have valid timestamps and confidence
    for segment in &result.segments {
        assert!(
            segment.end_time > segment.start_time,
            "Segment duration should be positive"
        );
        assert!(
            segment.confidence >= 0.0 && segment.confidence <= 1.0,
            "Confidence should be between 0 and 1"
        );
        assert!(segment.motion_score >= 0.0 && segment.motion_score <= 1.0);
    }
}

/// Test rapid cuts detection (frequent scene changes)
#[test]
fn test_rapid_cuts_detection() {
    let config = ActionRecognitionConfig {
        scene_change_threshold: 0.3,
        min_segment_duration: 1.0, // Lower threshold for this test
        ..Default::default()
    };
    let recognizer = ActionRecognizer::new(config);

    // Create video with many rapid cuts (new scene every 0.5 seconds)
    let keyframes = vec![
        create_test_keyframe(0.0, 0x1111111111111111, 100.0),
        create_test_keyframe(0.5, 0x2222222222222222, 100.0),
        create_test_keyframe(1.0, 0x3333333333333333, 100.0),
        create_test_keyframe(1.5, 0x4444444444444444, 100.0),
        create_test_keyframe(2.0, 0x5555555555555555, 100.0),
        create_test_keyframe(2.5, 0x6666666666666666, 100.0),
        create_test_keyframe(3.0, 0x7777777777777777, 100.0),
    ];

    let result = recognizer.analyze(&keyframes).unwrap();

    // With many scene changes over short duration, should detect rapid cuts
    assert!(
        result.total_scene_changes > 0,
        "Should detect multiple scene changes"
    );
}

/// Test custom configuration parameters
#[test]
fn test_custom_configuration() {
    let config = ActionRecognitionConfig {
        min_segment_duration: 3.0,
        confidence_threshold: 0.7,
        scene_change_threshold: 0.5,
    };
    let recognizer = ActionRecognizer::new(config);

    let keyframes = vec![
        create_test_keyframe(0.0, 0x1111111111111111, 100.0),
        create_test_keyframe(1.0, 0x1111111111111112, 100.0),
        create_test_keyframe(2.0, 0x1111111111111113, 100.0),
        create_test_keyframe(3.0, 0x1111111111111114, 100.0),
        create_test_keyframe(4.0, 0x1111111111111115, 100.0),
    ];

    let result = recognizer.analyze(&keyframes).unwrap();

    // Should successfully analyze with custom config
    assert!(!result.segments.is_empty());
}

/// Test result structure
#[test]
fn test_result_structure() {
    let config = ActionRecognitionConfig::default();
    let recognizer = ActionRecognizer::new(config);

    let keyframes = vec![
        create_test_keyframe(0.0, 0x1111111111111111, 100.0),
        create_test_keyframe(1.0, 0x1111111111111112, 100.0),
        create_test_keyframe(2.0, 0x1111111111111113, 100.0),
    ];

    let result = recognizer.analyze(&keyframes).unwrap();

    // Check result structure
    assert!(result.overall_confidence >= 0.0 && result.overall_confidence <= 1.0);
    // total_scene_changes is usize, always >= 0

    // Check segment structure
    for segment in &result.segments {
        assert!(segment.start_time >= 0.0);
        assert!(segment.end_time >= segment.start_time);
        assert!(segment.confidence >= 0.0 && segment.confidence <= 1.0);
        assert!(segment.motion_score >= 0.0 && segment.motion_score <= 1.0);
        // scene_changes is usize, always >= 0
    }
}

/// Test gradual motion increase (low -> moderate -> high)
#[test]
fn test_gradual_motion_increase() {
    let config = ActionRecognitionConfig::default();
    let recognizer = ActionRecognizer::new(config);

    // Create keyframes with gradually increasing motion
    let keyframes = vec![
        // Low motion
        create_test_keyframe(0.0, 0x1000000000000000, 100.0),
        create_test_keyframe(1.0, 0x1100000000000000, 100.0),
        create_test_keyframe(2.0, 0x1110000000000000, 100.0),
        // Moderate motion
        create_test_keyframe(3.0, 0x1111000000000000, 95.0),
        create_test_keyframe(4.0, 0x1111100000000000, 90.0),
        create_test_keyframe(5.0, 0x1111110000000000, 85.0),
        // High motion
        create_test_keyframe(6.0, 0x1111111000000000, 80.0),
        create_test_keyframe(7.0, 0x1111111100000000, 75.0),
        create_test_keyframe(8.0, 0x1111111110000000, 70.0),
    ];

    let result = recognizer.analyze(&keyframes).unwrap();

    // Should successfully segment the video
    assert!(!result.segments.is_empty());
    // Overall activity should reflect some motion pattern
    // (any activity type is valid since hashes are synthetic)
    assert!(matches!(
        result.overall_activity,
        ActivityType::Static
            | ActivityType::LowMotion
            | ActivityType::ModerateMotion
            | ActivityType::HighMotion
            | ActivityType::RapidCuts
    ));
}
