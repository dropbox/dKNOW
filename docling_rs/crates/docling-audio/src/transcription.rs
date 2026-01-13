//! Audio transcription using Whisper
//!
//! This module provides audio-to-text transcription using the Whisper model.
//! Requires the `transcription` feature to be enabled.
//!
//! ## Setup
//!
//! 1. Download a Whisper model (GGML format):
//!    ```bash
//!    # Recommended: Base English model (142MB, good accuracy)
//!    curl -L https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin \
//!      -o ~/.cache/whisper/ggml-base.en.bin
//!
//!    # Fast: Tiny English model (75MB, lower accuracy)
//!    curl -L https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin \
//!      -o ~/.cache/whisper/ggml-tiny.en.bin
//!    ```
//!
//! 2. Set model path in environment or config
//!
//! ## Example
//!
//! ```no_run
//! use docling_audio::transcribe_audio;
//!
//! // Transcribe with default config
//! let transcript = transcribe_audio("meeting.wav", None)?;
//! println!("{}", transcript.text);
//!
//! // Transcribe with custom config
//! use docling_audio::TranscriptionConfig;
//! let config = TranscriptionConfig {
//!     model_path: Some("~/.cache/whisper/ggml-base.en.bin".into()),
//!     language: Some("en".to_string()),
//!     ..Default::default()
//! };
//! let transcript = transcribe_audio("meeting.wav", Some(&config))?;
//! # Ok::<(), docling_audio::AudioError>(())
//! ```

use crate::error::{AudioError, Result};
use crate::mp3::read_mp3_samples;
use crate::wav::read_wav_samples;
use std::path::{Path, PathBuf};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

/// Configuration for audio transcription
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct TranscriptionConfig {
    /// Path to Whisper model file (GGML format)
    /// If None, uses default: ~/.cache/whisper/ggml-base.en.bin
    pub model_path: Option<PathBuf>,

    /// Language code (e.g., "en", "es", "fr")
    /// If None, auto-detects language
    pub language: Option<String>,

    /// Number of threads to use for transcription
    /// If None, uses system default
    pub num_threads: Option<usize>,

    /// Translate to English (for non-English audio)
    pub translate: bool,

    /// Print progress to stderr
    pub print_progress: bool,

    /// Print timestamps for each segment
    pub print_timestamps: bool,
}

impl Default for TranscriptionConfig {
    #[inline]
    fn default() -> Self {
        Self {
            model_path: None,
            language: Some("en".to_string()),
            num_threads: None,
            translate: false,
            print_progress: false,
            print_timestamps: false,
        }
    }
}

/// Result of audio transcription
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TranscriptionResult {
    /// Full transcribed text
    pub text: String,

    /// Detected language (ISO 639-1 code)
    pub language: String,

    /// Confidence scores for each segment (0.0 to 1.0)
    pub segments: Vec<TranscriptionSegment>,
}

/// A segment of transcribed audio with timing information
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TranscriptionSegment {
    /// Start time in seconds
    pub start: f64,

    /// End time in seconds
    pub end: f64,

    /// Transcribed text for this segment
    pub text: String,
}

/// Transcribe audio file to text
///
/// This function transcribes an audio file (WAV or MP3) to text using the Whisper model.
/// The audio is automatically resampled to 16kHz mono (required by Whisper).
///
/// # Arguments
///
/// * `path` - Path to the audio file (WAV or MP3)
/// * `config` - Optional transcription configuration
///
/// # Returns
///
/// Returns `TranscriptionResult` containing the transcribed text and metadata.
///
/// # Errors
///
/// Returns `AudioError` if:
/// - File cannot be read
/// - Audio format is invalid
/// - Model file not found
/// - Transcription fails
///
/// # Examples
///
/// ```no_run
/// use docling_audio::transcribe_audio;
///
/// let transcript = transcribe_audio("meeting.wav", None)?;
/// println!("Transcript: {}", transcript.text);
/// println!("Language: {}", transcript.language);
/// # Ok::<(), docling_audio::AudioError>(())
/// ```
#[must_use = "this function returns transcription results that should be processed"]
pub fn transcribe_audio<P: AsRef<Path>>(
    path: P,
    config: Option<&TranscriptionConfig>,
) -> Result<TranscriptionResult> {
    let path = path.as_ref();
    let default_config = TranscriptionConfig::default();
    let config = config.unwrap_or(&default_config);

    // Read audio samples (auto-detects format)
    let (samples, original_sample_rate) = read_audio_samples(path)?;

    // Resample to 16kHz if needed (Whisper requires 16kHz)
    let samples = if original_sample_rate == 16000 {
        samples
    } else {
        resample_audio(&samples, original_sample_rate, 16000)?
    };

    // Load Whisper model
    let model_path = get_model_path(config)?;
    let params = WhisperContextParameters::default();
    let ctx = WhisperContext::new_with_params(&model_path, params).map_err(|e| {
        AudioError::transcription_failed(format!("Failed to load Whisper model: {e}"))
    })?;

    // Configure transcription parameters
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

    if let Some(lang) = &config.language {
        params.set_language(Some(lang));
    }

    if let Some(threads) = config.num_threads {
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        params.set_n_threads(threads as i32);
    }

    params.set_translate(config.translate);
    params.set_print_progress(config.print_progress);
    params.set_print_timestamps(config.print_timestamps);

    // Run transcription
    let mut state = ctx.create_state().map_err(|e| {
        AudioError::transcription_failed(format!("Failed to create Whisper state: {e}"))
    })?;

    state
        .full(params, &samples)
        .map_err(|e| AudioError::transcription_failed(format!("Transcription failed: {e}")))?;

    // Extract results
    let num_segments = state.full_n_segments();

    let mut full_text = String::new();
    let mut segments = Vec::new();

    for i in 0..num_segments {
        let segment = state.get_segment(i).ok_or_else(|| {
            AudioError::transcription_failed(format!("Failed to get segment {i}"))
        })?;

        let segment_text = segment.to_str_lossy().map_err(|e| {
            AudioError::transcription_failed(format!("Failed to get segment text: {e}"))
        })?;

        // Convert centiseconds to seconds (i64 to f64 precision loss is acceptable for timestamps)
        #[allow(clippy::cast_precision_loss)]
        let start_time = segment.start_timestamp() as f64 / 100.0;
        #[allow(clippy::cast_precision_loss)]
        let end_time = segment.end_timestamp() as f64 / 100.0;

        full_text.push_str(&segment_text);
        full_text.push(' ');

        segments.push(TranscriptionSegment {
            start: start_time,
            end: end_time,
            text: segment_text.trim().to_string(),
        });
    }

    Ok(TranscriptionResult {
        text: full_text.trim().to_string(),
        language: config.language.clone().unwrap_or_else(|| "en".to_string()),
        segments,
    })
}

/// Read audio samples from file (auto-detects format)
fn read_audio_samples<P: AsRef<Path>>(path: P) -> Result<(Vec<f32>, u32)> {
    let path = path.as_ref();

    // Detect format by extension
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| AudioError::invalid_format(path, "No file extension"))?;

    match extension.to_lowercase().as_str() {
        "wav" => read_wav_samples(path),
        "mp3" => read_mp3_samples(path),
        _ => Err(AudioError::unsupported_format(extension)),
    }
}

/// Resample audio from source sample rate to target sample rate
fn resample_audio(samples: &[f32], from_rate: u32, to_rate: u32) -> Result<Vec<f32>> {
    // Use rubato for high-quality resampling
    use rubato::{
        Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
    };

    if from_rate == to_rate {
        return Ok(samples.to_vec());
    }

    let params = SincInterpolationParameters {
        sinc_len: 256,
        f_cutoff: 0.95,
        interpolation: SincInterpolationType::Linear,
        oversampling_factor: 256,
        window: WindowFunction::BlackmanHarris2,
    };

    let mut resampler = SincFixedIn::<f32>::new(
        f64::from(to_rate) / f64::from(from_rate),
        2.0, // max_resample_ratio_relative (for safety)
        params,
        samples.len(),
        1, // num_channels (mono)
    )
    .map_err(|e| AudioError::resampling_failed(format!("Failed to create resampler: {e}")))?;

    let waves_in = vec![samples.to_vec()];
    let mut waves_out = resampler
        .process(&waves_in, None)
        .map_err(|e| AudioError::resampling_failed(format!("Resampling failed: {e}")))?;

    Ok(waves_out.remove(0))
}

/// Get Whisper model path from config or default location
fn get_model_path(config: &TranscriptionConfig) -> Result<String> {
    if let Some(path) = &config.model_path {
        let path_str = path.to_string_lossy().to_string();

        // Expand ~ to home directory
        let expanded_path = path_str
            .strip_prefix("~/")
            .map(|stripped| {
                std::env::var_os("HOME").map_or_else(
                    || path_str.clone(),
                    |home| {
                        PathBuf::from(home)
                            .join(stripped)
                            .to_string_lossy()
                            .to_string()
                    },
                )
            })
            .unwrap_or(path_str);

        if !Path::new(&expanded_path).exists() {
            return Err(AudioError::model_not_found(&expanded_path));
        }

        return Ok(expanded_path);
    }

    // Try default locations
    let default_paths = [
        "~/.cache/whisper/ggml-base.en.bin",
        "~/.cache/whisper/ggml-tiny.en.bin",
        "./models/ggml-base.en.bin",
        "./models/ggml-tiny.en.bin",
    ];

    for path in &default_paths {
        let expanded = path.strip_prefix("~/").map_or_else(
            || PathBuf::from(*path),
            |stripped| {
                std::env::var_os("HOME").map_or_else(
                    || PathBuf::from(*path),
                    |home| PathBuf::from(home).join(stripped),
                )
            },
        );

        if expanded.exists() {
            return Ok(expanded.to_string_lossy().to_string());
        }
    }

    Err(AudioError::model_not_found(
        "No Whisper model found. Please download a model:\n\
        curl -L https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin \\\n\
          -o ~/.cache/whisper/ggml-base.en.bin",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resample_audio() {
        // Test resampling from 44100Hz to 16000Hz
        let samples: Vec<f32> = vec![0.0; 44100]; // 1 second of silence at 44100Hz
        let resampled = resample_audio(&samples, 44100, 16000).unwrap();

        // Should have approximately 16000 samples
        // usize to f32 cast is acceptable for test comparison
        #[allow(clippy::cast_precision_loss)]
        let diff = (resampled.len() as f32 - 16000.0).abs();
        assert!(diff < 100.0);
    }

    #[test]
    fn test_resample_audio_same_rate() {
        // Test that resampling to same rate is a no-op
        let samples: Vec<f32> = vec![0.0; 16000];
        let resampled = resample_audio(&samples, 16000, 16000).unwrap();
        assert_eq!(samples.len(), resampled.len());
    }
}
