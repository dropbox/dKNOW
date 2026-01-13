use std::path::Path;
use video_audio_scene::{detect_scenes, SceneDetectorConfig};

#[test]
#[ignore] // Run manually: cargo test --package video-audio-scene --test integration_test -- --ignored
fn test_scene_detection_with_duration() {
    // Use a known test video (10 seconds)
    let video_path = Path::new("/Users/ayates/docling/tests/data/audio/sample_10s_video-mp4.mp4");

    if !video_path.exists() {
        eprintln!("Test video not found at {video_path:?}, skipping test");
        return;
    }

    let config = SceneDetectorConfig {
        threshold: 5.0, // Low threshold to detect some scenes
        min_scene_duration: 0.0,
        keyframes_only: false,
    };

    let result = detect_scenes(video_path, &config).expect("Scene detection failed");

    println!("Detected {} scenes", result.num_scenes);
    println!("Scene boundaries: {}", result.boundaries.len());

    for (i, scene) in result.scenes.iter().enumerate() {
        println!(
            "Scene {}: {:.2}s - {:.2}s (duration: {:.2}s, score: {:.2})",
            i,
            scene.start_time,
            scene.end_time,
            scene.end_time - scene.start_time,
            scene.score
        );
    }

    // Verify that if there are scenes, the last scene ends at approximately the video duration (9.99s)
    if let Some(last_scene) = result.scenes.last() {
        println!("Last scene end_time: {:.2}s", last_scene.end_time);

        // The video is 9.99 seconds, so last scene should end around that time
        // Allow some tolerance for FFmpeg duration parsing
        assert!(
            last_scene.end_time >= 9.0 && last_scene.end_time <= 10.5,
            "Last scene end_time should be around 9.99s (video duration), got {:.2}s",
            last_scene.end_time
        );

        // Verify it's not the old placeholder value (last_boundary + 1.0)
        if !result.boundaries.is_empty() {
            let last_boundary_time = result.boundaries.last().unwrap().timestamp;
            // If this assertion fails, we're still using the placeholder
            assert_ne!(
                last_scene.end_time,
                last_boundary_time + 1.0,
                "Last scene end_time appears to be placeholder value (last_boundary + 1.0)"
            );
        }
    }
}
