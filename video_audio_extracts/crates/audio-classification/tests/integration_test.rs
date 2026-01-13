//! Integration tests for audio classification

use video_audio_classification::{AudioClassificationConfig, AudioClassifier};

/// Helper function to get model path
fn get_model_path() -> std::path::PathBuf {
    std::env::current_dir()
        .expect("Failed to get current directory")
        .ancestors()
        .find(|p| p.join("Cargo.toml").exists() && p.join("crates").exists())
        .expect("Failed to find project root")
        .join("models/audio-classification/yamnet.onnx")
}

/// Test YAMNet model loading
#[test]
#[ignore] // Requires model file
fn test_yamnet_model_loading() {
    let model_path = get_model_path();
    let config = AudioClassificationConfig::default();

    let classifier = AudioClassifier::new(model_path, config);
    assert!(
        classifier.is_ok(),
        "Failed to load YAMNet model: {:?}",
        classifier.err()
    );
}

/// Test audio classification with sine wave (should classify as tone/music)
#[test]
#[ignore] // Requires model file and test audio
fn test_classify_sine_wave() {
    let model_path = get_model_path();
    let config = AudioClassificationConfig {
        confidence_threshold: 0.1,
        top_k: 5,
        segment_duration: 3.0,
    };

    let mut classifier =
        AudioClassifier::new(model_path, config).expect("Failed to load YAMNet model");

    // Generate 3 seconds of 440Hz sine wave (16kHz sample rate)
    let sample_rate = 16000;
    let duration_sec = 3.0;
    let frequency = 440.0; // A4 note
    let num_samples = (sample_rate as f32 * duration_sec) as usize;

    let mut audio = Vec::with_capacity(num_samples);
    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let sample = (2.0 * std::f32::consts::PI * frequency * t).sin() * 0.5;
        audio.push(sample);
    }

    let results = classifier.classify(&audio).expect("Classification failed");

    // Should get 1 segment (3 seconds)
    assert_eq!(results.len(), 1, "Expected 1 segment");

    let segment = &results[0];
    assert_eq!(segment.start_time, 0.0);
    assert!((segment.end_time - 3.0).abs() < 0.01);

    // Check that we got some results
    assert!(
        !segment.results.is_empty(),
        "Expected at least one classification result"
    );

    // Print results for debugging
    println!("Sine wave classification results:");
    for result in &segment.results {
        println!("  {}: {:.2}%", result.class_name, result.confidence * 100.0);
    }

    // Sine wave should be classified as some kind of tone/music/sound
    // (not asserting specific class as YAMNet classes are diverse)
}

/// Test audio classification with silence (should have low confidence or silence-related classes)
#[test]
#[ignore] // Requires model file
fn test_classify_silence() {
    let model_path = get_model_path();
    let config = AudioClassificationConfig {
        confidence_threshold: 0.05, // Low threshold to catch any predictions
        top_k: 5,
        segment_duration: 3.0,
    };

    let mut classifier =
        AudioClassifier::new(model_path, config).expect("Failed to load YAMNet model");

    // Generate 3 seconds of silence (16kHz sample rate)
    let sample_rate = 16000;
    let duration_sec = 3.0;
    let num_samples = (sample_rate as f32 * duration_sec) as usize;
    let audio = vec![0.0f32; num_samples];

    let results = classifier.classify(&audio).expect("Classification failed");

    // Print results for debugging
    println!("Silence classification results:");
    if results.is_empty() {
        println!("  No classifications above threshold (expected for silence)");
    } else {
        for segment in &results {
            for result in &segment.results {
                println!("  {}: {:.2}%", result.class_name, result.confidence * 100.0);
            }
        }
    }

    // Silence typically produces low confidence scores or specific silence classes
    // We don't assert specific behavior as it depends on the model's training
}

/// Test audio classification with multiple segments
#[test]
#[ignore] // Requires model file
fn test_classify_multiple_segments() {
    let model_path = get_model_path();
    let config = AudioClassificationConfig::default();

    let mut classifier =
        AudioClassifier::new(model_path, config).expect("Failed to load YAMNet model");

    // Generate 6 seconds of audio (should produce 2 segments)
    let sample_rate = 16000;
    let duration_sec = 6.0;
    let num_samples = (sample_rate as f32 * duration_sec) as usize;

    // First 3 seconds: 440Hz tone
    // Second 3 seconds: 880Hz tone
    let mut audio = Vec::with_capacity(num_samples);
    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let frequency = if t < 3.0 { 440.0 } else { 880.0 };
        let sample = (2.0 * std::f32::consts::PI * frequency * t).sin() * 0.5;
        audio.push(sample);
    }

    let results = classifier.classify(&audio).expect("Classification failed");

    // Should get 2 segments
    assert_eq!(
        results.len(),
        2,
        "Expected 2 segments for 6 seconds of audio"
    );

    // Check segment timing
    assert_eq!(results[0].start_time, 0.0);
    assert!((results[0].end_time - 3.0).abs() < 0.01);
    assert!((results[1].start_time - 3.0).abs() < 0.01);
    assert!((results[1].end_time - 6.0).abs() < 0.01);

    println!("Multi-segment classification results:");
    for (i, segment) in results.iter().enumerate() {
        println!(
            "  Segment {}: {:.2}s - {:.2}s",
            i + 1,
            segment.start_time,
            segment.end_time
        );
        for result in &segment.results {
            println!(
                "    {}: {:.2}%",
                result.class_name,
                result.confidence * 100.0
            );
        }
    }
}

/// Test configuration variants
#[test]
fn test_classification_configs() {
    let default_config = AudioClassificationConfig::default();
    assert_eq!(default_config.confidence_threshold, 0.3);
    assert_eq!(default_config.top_k, 5);
    assert_eq!(default_config.segment_duration, 3.0);

    let high_conf = AudioClassificationConfig::high_confidence();
    assert_eq!(high_conf.confidence_threshold, 0.5);
    assert_eq!(high_conf.top_k, 3);

    let comprehensive = AudioClassificationConfig::comprehensive();
    assert_eq!(comprehensive.confidence_threshold, 0.1);
    assert_eq!(comprehensive.top_k, 10);
}

/// Test invalid audio length handling
#[test]
#[ignore] // Requires model file
fn test_invalid_audio_length() {
    let model_path = get_model_path();
    let config = AudioClassificationConfig::default();

    let mut classifier =
        AudioClassifier::new(model_path, config).expect("Failed to load YAMNet model");

    // Audio too short (less than 3 seconds = 48000 samples at 16kHz)
    let audio = vec![0.0f32; 1000];

    let result = classifier.classify(&audio);
    assert!(
        result.is_err(),
        "Expected error for audio shorter than 3 seconds"
    );
}
