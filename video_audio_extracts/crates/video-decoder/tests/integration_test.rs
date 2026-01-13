/// Integration tests for video decoder module
use std::path::PathBuf;
use video_audio_decoder::{decode_video, DecoderConfig, FrameFilter, PixelFormat};

fn test_video_path(filename: &str) -> PathBuf {
    PathBuf::from("/Users/ayates/docling/tests/data/audio").join(filename)
}

#[test]
fn test_decode_mp4_all_frames() {
    let path = test_video_path("sample_10s_video-mp4.mp4");

    let config = DecoderConfig {
        output_format: PixelFormat::YUV420P,
        frame_filter: None,
    };

    let frames = decode_video(&path, &config).expect("Failed to decode video");

    // 10 second video at ~24fps should have around 240 frames
    assert!(
        frames.len() > 200,
        "Expected at least 200 frames, got {}",
        frames.len()
    );
    assert!(
        frames.len() < 300,
        "Expected less than 300 frames, got {}",
        frames.len()
    );

    // Check first frame
    let first_frame = &frames[0];
    assert_eq!(first_frame.frame_number, 0);
    assert_eq!(first_frame.width, 320);
    assert_eq!(first_frame.height, 320);
    assert_eq!(first_frame.format, PixelFormat::YUV420P);

    // YUV420P data size: width * height * 1.5 (Y + U/4 + V/4)
    let expected_size = (320 * 320 * 3) / 2;
    assert_eq!(first_frame.data.len(), expected_size);

    // Check timestamps are increasing
    for i in 1..frames.len() {
        assert!(
            frames[i].timestamp >= frames[i - 1].timestamp,
            "Timestamps should be monotonically increasing"
        );
    }
}

#[test]
fn test_decode_mov_all_frames() {
    let path = test_video_path("sample_10s_video-quicktime.mov");

    let config = DecoderConfig {
        output_format: PixelFormat::YUV420P,
        frame_filter: None,
    };

    let frames = decode_video(&path, &config).expect("Failed to decode video");

    // Should have similar frame count to MP4
    assert!(
        frames.len() > 200,
        "Expected at least 200 frames, got {}",
        frames.len()
    );

    // Check dimensions
    let first_frame = &frames[0];
    assert_eq!(first_frame.width, 320);
    assert_eq!(first_frame.height, 320);
}

#[test]
fn test_decode_avi_all_frames() {
    let path = test_video_path("sample_10s_video-avi.avi");

    let config = DecoderConfig {
        output_format: PixelFormat::YUV420P,
        frame_filter: None,
    };

    let frames = decode_video(&path, &config).expect("Failed to decode video");

    // AVI should also decode successfully
    assert!(
        frames.len() > 200,
        "Expected at least 200 frames, got {}",
        frames.len()
    );
}

#[test]
fn test_decode_rgb24_format() {
    let path = test_video_path("sample_10s_video-mp4.mp4");

    let config = DecoderConfig {
        output_format: PixelFormat::RGB24,
        frame_filter: None,
    };

    let frames = decode_video(&path, &config).expect("Failed to decode video");

    assert!(!frames.is_empty());

    let first_frame = &frames[0];
    assert_eq!(first_frame.format, PixelFormat::RGB24);

    // RGB24 data size: width * height * 3
    let expected_size = 320 * 320 * 3;
    assert_eq!(first_frame.data.len(), expected_size);
}

#[test]
fn test_filter_every_nth_frame() {
    let path = test_video_path("sample_10s_video-mp4.mp4");

    let config = DecoderConfig {
        output_format: PixelFormat::YUV420P,
        frame_filter: Some(FrameFilter::EveryNth(10)),
    };

    let frames = decode_video(&path, &config).expect("Failed to decode video");

    // Should extract every 10th frame, so ~24 frames from 240 total
    assert!(
        frames.len() > 15,
        "Expected at least 15 frames, got {}",
        frames.len()
    );
    assert!(
        frames.len() < 35,
        "Expected less than 35 frames, got {}",
        frames.len()
    );

    // Check frame numbers are multiples of 10
    for frame in &frames {
        assert_eq!(
            frame.frame_number % 10,
            0,
            "Frame number should be multiple of 10"
        );
    }
}

#[test]
fn test_filter_iframes_only() {
    let path = test_video_path("sample_10s_video-mp4.mp4");

    let config = DecoderConfig {
        output_format: PixelFormat::YUV420P,
        frame_filter: Some(FrameFilter::IFramesOnly),
    };

    let frames = decode_video(&path, &config).expect("Failed to decode video");

    // Should have significantly fewer frames (keyframes only)
    assert!(!frames.is_empty(), "Should have at least one keyframe");
    assert!(
        frames.len() < 50,
        "Should have less than 50 keyframes, got {}",
        frames.len()
    );

    // All frames should be keyframes
    for frame in &frames {
        assert!(
            frame.is_keyframe,
            "All extracted frames should be keyframes"
        );
    }
}

#[test]
fn test_filter_timestamps() {
    let path = test_video_path("sample_10s_video-mp4.mp4");

    // Extract frames at specific timestamps
    let timestamps = vec![0.0, 2.5, 5.0, 7.5, 9.5];

    let config = DecoderConfig {
        output_format: PixelFormat::YUV420P,
        frame_filter: Some(FrameFilter::Timestamps(timestamps.clone())),
    };

    let frames = decode_video(&path, &config).expect("Failed to decode video");

    // Should extract approximately 5 frames (one per timestamp)
    assert!(
        frames.len() >= 4,
        "Expected at least 4 frames, got {}",
        frames.len()
    );
    assert!(
        frames.len() <= 6,
        "Expected at most 6 frames, got {}",
        frames.len()
    );

    // Check timestamps are close to requested
    for (i, frame) in frames.iter().enumerate() {
        let closest_target = timestamps
            .iter()
            .map(|&ts| (ts - frame.timestamp).abs())
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();

        assert!(
            closest_target < 0.2,
            "Frame {} timestamp {} is not close to any requested timestamp",
            i,
            frame.timestamp
        );
    }
}

#[test]
fn test_decoder_with_frame_filtering() {
    let path = test_video_path("sample_10s_video-mp4.mp4");

    // Test decoder with frame filtering (every 30th frame)
    let config = DecoderConfig {
        output_format: PixelFormat::YUV420P,
        frame_filter: Some(FrameFilter::EveryNth(30)),
    };

    let frames = decode_video(&path, &config).expect("Failed to decode video");

    assert!(
        !frames.is_empty(),
        "Decoder should work with frame filtering"
    );
}

#[test]
fn test_error_on_audio_only_file() {
    let path = test_video_path("sample_10s_audio-mp3.mp3");

    let config = DecoderConfig::default();

    let result = decode_video(&path, &config);

    assert!(result.is_err(), "Should error on audio-only file");
}

#[test]
fn test_empty_filter_timestamps() {
    let path = test_video_path("sample_10s_video-mp4.mp4");

    let config = DecoderConfig {
        output_format: PixelFormat::YUV420P,
        frame_filter: Some(FrameFilter::Timestamps(vec![])),
    };

    let frames = decode_video(&path, &config).expect("Failed to decode video");

    // Empty timestamps should return no frames
    assert_eq!(frames.len(), 0, "Empty timestamps should return no frames");
}
