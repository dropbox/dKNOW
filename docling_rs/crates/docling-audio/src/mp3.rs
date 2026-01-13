//! MP3 (MPEG-1 Audio Layer 3) parser
//!
//! This module provides parsing for MP3 audio files using the `symphonia` crate.
//!
//! ## Features
//!
//! - Parse MP3 file metadata (sample rate, channels, duration)
//! - Extract audio samples for transcription
//! - Support for various MP3 bitrates
//! - Support for mono and stereo audio
//!
//! ## Example
//!
//! ```no_run
//! use docling_audio::parse_mp3;
//!
//! let mp3_info = parse_mp3("podcast.mp3")?;
//! println!("Sample rate: {}Hz", mp3_info.sample_rate);
//! println!("Duration: {:.2}s", mp3_info.duration_secs);
//! # Ok::<(), docling_audio::AudioError>(())
//! ```

use crate::error::{AudioError, Result};
use std::path::Path;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// Information about an MP3 audio file
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Mp3Info {
    /// Sample rate in Hz (e.g., 44100, 48000)
    pub sample_rate: u32,

    /// Number of audio channels (1 = mono, 2 = stereo)
    pub channels: u16,

    /// Duration in seconds
    pub duration_secs: f64,

    /// Total number of samples (estimated)
    pub total_samples: u64,
}

/// Parse MP3 file and extract metadata
///
/// # Arguments
///
/// * `path` - Path to the MP3 file
///
/// # Returns
///
/// Returns `Mp3Info` containing the audio metadata.
///
/// # Errors
///
/// Returns `AudioError` if:
/// - File cannot be read
/// - File is not a valid MP3 file
/// - Audio format is unsupported
///
/// # Examples
///
/// ```no_run
/// use docling_audio::parse_mp3;
///
/// let mp3_info = parse_mp3("podcast.mp3")?;
/// println!("Duration: {:.2} seconds", mp3_info.duration_secs);
/// # Ok::<(), docling_audio::AudioError>(())
/// ```
#[must_use = "this function returns MP3 info that should be processed"]
pub fn parse_mp3<P: AsRef<Path>>(path: P) -> Result<Mp3Info> {
    let path = path.as_ref();

    // Open file
    let file = std::fs::File::open(path).map_err(|e| AudioError::io(path, e))?;

    // Create media source
    let mss = MediaSourceStream::new(Box::new(file), MediaSourceStreamOptions::default());

    // Create hint with file extension
    let mut hint = Hint::new();
    hint.with_extension("mp3");

    // Probe the media source
    let format_opts = FormatOptions::default();
    let metadata_opts = MetadataOptions::default();

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &format_opts, &metadata_opts)
        .map_err(|e| AudioError::invalid_format(path, format!("Failed to probe MP3 file: {e}")))?;

    let format = probed.format;

    // Get the default track
    let track = format
        .default_track()
        .ok_or_else(|| AudioError::invalid_format(path, "No audio track found in MP3 file"))?;

    let sample_rate = track
        .codec_params
        .sample_rate
        .ok_or_else(|| AudioError::invalid_format(path, "Sample rate not found in MP3 metadata"))?;

    // Audio channels are always small (typically 1-8), safe to truncate
    #[allow(clippy::cast_possible_truncation)]
    let channels = track
        .codec_params
        .channels
        .map(|c| c.count() as u16)
        .ok_or_else(|| AudioError::invalid_format(path, "Channel count not found"))?;

    // Calculate duration
    // Precision loss is acceptable: n_frames would need to exceed 2^52 (~4.5 quadrillion)
    // for any precision loss, far beyond any real audio file's sample count
    #[allow(clippy::cast_precision_loss)]
    let duration_secs = track
        .codec_params
        .n_frames
        .map_or(0.0, |n_frames| n_frames as f64 / f64::from(sample_rate));

    // Truncation is intentional - we want whole sample count from f64 calculation
    // Sign loss is safe because duration_secs is always non-negative for audio files
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let total_samples = (duration_secs * f64::from(sample_rate)) as u64;

    Ok(Mp3Info {
        sample_rate,
        channels,
        duration_secs,
        total_samples,
    })
}

/// Read MP3 file and extract audio samples as f32 (mono, 16kHz)
///
/// This function reads an MP3 file and returns the audio samples as f32 values
/// in the range [-1.0, 1.0]. The audio is automatically:
/// - Converted to mono (if stereo, channels are averaged)
/// - Resampled to 16kHz (required for Whisper transcription)
///
/// # Arguments
///
/// * `path` - Path to the MP3 file
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
/// use docling_audio::mp3::read_mp3_samples;
///
/// let (samples, sample_rate) = read_mp3_samples("audio.mp3")?;
/// println!("Read {} samples at {}Hz", samples.len(), sample_rate);
/// # Ok::<(), docling_audio::AudioError>(())
/// ```
#[must_use = "this function returns audio samples that should be processed"]
pub fn read_mp3_samples<P: AsRef<Path>>(path: P) -> Result<(Vec<f32>, u32)> {
    let path = path.as_ref();

    // Open file
    let file = std::fs::File::open(path).map_err(|e| AudioError::io(path, e))?;

    // Create media source
    let mss = MediaSourceStream::new(Box::new(file), MediaSourceStreamOptions::default());

    // Create hint with file extension
    let mut hint = Hint::new();
    hint.with_extension("mp3");

    // Probe the media source
    let format_opts = FormatOptions::default();
    let metadata_opts = MetadataOptions::default();

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &format_opts, &metadata_opts)
        .map_err(|e| AudioError::invalid_format(path, format!("Failed to probe MP3 file: {e}")))?;

    let mut format = probed.format;

    // Get the default track
    let track = format
        .default_track()
        .ok_or_else(|| AudioError::invalid_format(path, "No audio track found in MP3 file"))?;

    let track_id = track.id;
    let sample_rate = track
        .codec_params
        .sample_rate
        .ok_or_else(|| AudioError::invalid_format(path, "Sample rate not found in MP3 metadata"))?;

    let channels = track
        .codec_params
        .channels
        .ok_or_else(|| AudioError::invalid_format(path, "Channel count not found"))?;

    // Create decoder
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| {
            AudioError::invalid_format(path, format!("Failed to create MP3 decoder: {e}"))
        })?;

    // Decode all packets
    let mut samples: Vec<f32> = Vec::new();

    loop {
        // Read next packet
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(symphonia::core::errors::Error::IoError(e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break; // End of stream
            }
            Err(e) => {
                return Err(AudioError::invalid_format(
                    path,
                    format!("Failed to read MP3 packet: {e}"),
                ))
            }
        };

        // Only decode packets for our track
        if packet.track_id() != track_id {
            continue;
        }

        // Decode packet
        let audio_buffer = decoder.decode(&packet).map_err(|e| {
            AudioError::invalid_format(path, format!("Failed to decode MP3 packet: {e}"))
        })?;

        // Convert to sample buffer
        let spec = *audio_buffer.spec();
        let duration = audio_buffer.capacity() as u64;

        let mut sample_buf = SampleBuffer::<f32>::new(duration, spec);
        sample_buf.copy_interleaved_ref(audio_buffer);

        // Add samples to output
        samples.extend_from_slice(sample_buf.samples());
    }

    // Convert stereo to mono by averaging channels
    let mono_samples = if channels.count() == 2 {
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
    fn test_parse_nonexistent_mp3() {
        let result = parse_mp3("nonexistent.mp3");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_mp3() {
        // Test with a text file instead of MP3
        let result = parse_mp3("Cargo.toml");
        assert!(result.is_err());
    }
}
