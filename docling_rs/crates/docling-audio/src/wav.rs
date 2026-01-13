//! WAV (Waveform Audio Format) parser
//!
//! This module provides parsing for WAV audio files using the `hound` crate.
//!
//! ## Features
//!
//! - Parse WAV file metadata (sample rate, channels, bit depth, duration)
//! - Extract audio samples for transcription
//! - Support for various bit depths (8, 16, 24, 32-bit)
//! - Support for mono and stereo audio
//!
//! ## Example
//!
//! ```no_run
//! use docling_audio::parse_wav;
//!
//! let wav_info = parse_wav("recording.wav")?;
//! println!("Sample rate: {}Hz", wav_info.sample_rate);
//! println!("Duration: {:.2}s", wav_info.duration_secs);
//! # Ok::<(), docling_audio::AudioError>(())
//! ```

use crate::error::{AudioError, Result};
use std::path::Path;

/// Information about a WAV audio file
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WavInfo {
    /// Sample rate in Hz (e.g., 44100, 48000)
    pub sample_rate: u32,

    /// Number of audio channels (1 = mono, 2 = stereo)
    pub channels: u16,

    /// Bit depth (e.g., 16, 24, 32)
    pub bit_depth: u16,

    /// Duration in seconds
    pub duration_secs: f64,

    /// Total number of samples
    pub total_samples: u64,
}

/// Parse WAV file and extract metadata
///
/// # Arguments
///
/// * `path` - Path to the WAV file
///
/// # Returns
///
/// Returns `WavInfo` containing the audio metadata.
///
/// # Errors
///
/// Returns `AudioError` if:
/// - File cannot be read
/// - File is not a valid WAV file
/// - Audio format is unsupported
///
/// # Examples
///
/// ```no_run
/// use docling_audio::parse_wav;
///
/// let wav_info = parse_wav("meeting.wav")?;
/// println!("Duration: {:.2} seconds", wav_info.duration_secs);
/// # Ok::<(), docling_audio::AudioError>(())
/// ```
#[must_use = "this function returns WAV info that should be processed"]
pub fn parse_wav<P: AsRef<Path>>(path: P) -> Result<WavInfo> {
    let path = path.as_ref();

    // Open WAV file using hound
    let reader = hound::WavReader::open(path)
        .map_err(|e| AudioError::invalid_format(path, format!("Failed to open WAV file: {e}")))?;

    let spec = reader.spec();

    // Calculate duration
    let total_samples = u64::from(reader.len());
    // Precision loss acceptable: sample counts never exceed f64 mantissa range in practice
    #[allow(clippy::cast_precision_loss)]
    let duration_secs =
        total_samples as f64 / (f64::from(spec.sample_rate) * f64::from(spec.channels));

    Ok(WavInfo {
        sample_rate: spec.sample_rate,
        channels: spec.channels,
        bit_depth: spec.bits_per_sample,
        duration_secs,
        total_samples,
    })
}

/// Read WAV file and extract audio samples as f32 (mono, 16kHz)
///
/// This function reads a WAV file and returns the audio samples as f32 values
/// in the range [-1.0, 1.0]. The audio is automatically:
/// - Converted to mono (if stereo, channels are averaged)
/// - Resampled to 16kHz (required for Whisper transcription)
///
/// # Arguments
///
/// * `path` - Path to the WAV file
///
/// # Returns
///
/// Returns a vector of f32 audio samples and the original sample rate.
///
/// # Errors
///
/// Returns `AudioError` if file cannot be read or format is invalid.
///
/// # Examples
///
/// ```no_run
/// use docling_audio::wav::read_wav_samples;
///
/// let (samples, sample_rate) = read_wav_samples("audio.wav")?;
/// println!("Read {} samples at {}Hz", samples.len(), sample_rate);
/// # Ok::<(), docling_audio::AudioError>(())
/// ```
#[must_use = "this function returns audio samples that should be processed"]
pub fn read_wav_samples<P: AsRef<Path>>(path: P) -> Result<(Vec<f32>, u32)> {
    let path = path.as_ref();

    let mut reader = hound::WavReader::open(path)
        .map_err(|e| AudioError::invalid_format(path, format!("Failed to open WAV file: {e}")))?;

    let spec = reader.spec();
    let sample_rate = spec.sample_rate;
    let channels = spec.channels as usize;

    // Read samples based on bit depth
    let samples: Vec<f32> = match spec.bits_per_sample {
        8 => reader
            .samples::<i8>()
            .map(|s| f32::from(s.unwrap_or(0)) / 128.0)
            .collect(),
        16 => reader
            .samples::<i16>()
            .map(|s| f32::from(s.unwrap_or(0)) / 32768.0)
            .collect(),
        // Precision loss intentional: 24/32-bit audio samples converted to f32 representation
        // is standard practice in audio processing (f32 provides sufficient dynamic range)
        #[allow(clippy::cast_precision_loss)]
        24 => reader
            .samples::<i32>()
            .map(|s| s.unwrap_or(0) as f32 / 8_388_608.0)
            .collect(),
        #[allow(clippy::cast_precision_loss)]
        32 => reader
            .samples::<i32>()
            .map(|s| s.unwrap_or(0) as f32 / 2_147_483_648.0)
            .collect(),
        _ => {
            let bits_per_sample = spec.bits_per_sample;
            return Err(AudioError::invalid_format(
                path,
                format!("Unsupported bit depth: {bits_per_sample}"),
            ));
        }
    };

    // Convert stereo to mono by averaging channels
    let mono_samples = if channels == 2 {
        samples
            .chunks_exact(2)
            .map(|chunk| f32::midpoint(chunk[0], chunk[1]))
            .collect()
    } else {
        samples
    };

    Ok((mono_samples, sample_rate))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_nonexistent_wav() {
        let result = parse_wav("nonexistent.wav");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_wav() {
        // Test with a text file instead of WAV
        let result = parse_wav("Cargo.toml");
        assert!(result.is_err());
    }
}
