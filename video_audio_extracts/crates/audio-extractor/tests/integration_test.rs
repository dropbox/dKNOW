use std::path::Path;
use tempfile::TempDir;
use video_audio_extractor::{extract_audio, AudioConfig, AudioFormat};

#[test]
fn test_extract_audio_pcm() {
    let test_video = Path::new("/Users/ayates/docling/tests/data/audio/sample_10s_video-mp4.mp4");

    assert!(test_video.exists(), "Test video not found: {test_video:?}");

    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("output");

    let config = AudioConfig::for_ml(); // 16kHz mono PCM

    let result_path =
        extract_audio(test_video, &output_path, &config).expect("Failed to extract audio");

    assert!(result_path.exists(), "Output file not created");
    assert_eq!(result_path.extension().unwrap(), "wav");

    let metadata = std::fs::metadata(&result_path).unwrap();
    assert!(
        metadata.len() > 10000,
        "Output file too small: {} bytes",
        metadata.len()
    );

    println!(
        "Extracted audio to {:?}, size: {} bytes",
        result_path,
        metadata.len()
    );
}

#[test]
fn test_extract_audio_flac() {
    let test_video = Path::new("/Users/ayates/docling/tests/data/audio/sample_10s_video-mp4.mp4");

    assert!(test_video.exists(), "Test video not found: {test_video:?}");

    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("output");

    let config = AudioConfig::for_storage(); // 48kHz stereo FLAC

    let result_path =
        extract_audio(test_video, &output_path, &config).expect("Failed to extract audio");

    assert!(result_path.exists(), "Output file not created");
    assert_eq!(result_path.extension().unwrap(), "flac");

    println!("Extracted FLAC audio to {result_path:?}");
}

#[test]
fn test_extract_from_audio_only_mp3() {
    let test_audio = Path::new("/Users/ayates/docling/tests/data/audio/sample_10s.mp3");

    assert!(test_audio.exists(), "Test audio not found: {test_audio:?}");

    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("output");

    let config = AudioConfig::default();

    let result_path = extract_audio(test_audio, &output_path, &config)
        .expect("Failed to extract audio from audio file");

    assert!(result_path.exists(), "Output file not created");
    println!("Extracted from audio-only file to {result_path:?}");
}

#[test]
fn test_extract_wav() {
    let test_audio = Path::new("/Users/ayates/docling/tests/data/audio/sample_10s_audio-wav.wav");

    if !test_audio.exists() {
        eprintln!("Skipping test - file not found");
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("output");
    let config = AudioConfig::default();

    let result = extract_audio(test_audio, &output_path, &config);
    assert!(result.is_ok(), "Failed to extract from WAV");
}

#[test]
fn test_extract_video_avi() {
    let test_video = Path::new("/Users/ayates/docling/tests/data/audio/sample_10s_video-avi.avi");

    if !test_video.exists() {
        eprintln!("Skipping test - file not found");
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("output");
    let config = AudioConfig::default();

    let result = extract_audio(test_video, &output_path, &config);
    assert!(result.is_ok(), "Failed to extract from AVI");
}

#[test]
fn test_extract_with_normalization() {
    let test_audio = Path::new("/Users/ayates/docling/tests/data/audio/sample_10s.mp3");

    assert!(test_audio.exists(), "Test audio not found");

    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("output_normalized");

    let config = AudioConfig {
        sample_rate: 16000,
        channels: 1,
        format: AudioFormat::PCM,
        normalize: true,
    };

    let result_path = extract_audio(test_audio, &output_path, &config)
        .expect("Failed to extract with normalization");

    assert!(result_path.exists());
    println!("Extracted normalized audio");
}
