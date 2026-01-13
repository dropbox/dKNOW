/// Audio extraction module
///
/// Implements audio extraction from video/audio files with configurable format,
/// sample rate, channels, and optional EBU R128 normalization.
pub mod plugin;

use std::path::{Path, PathBuf};
use std::process::Command;
use video_audio_common::{ProcessingError, Result};
use video_audio_decoder::c_ffi::FormatContext;

/// Audio output format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioFormat {
    /// PCM (uncompressed) - for ML models
    PCM,
    /// FLAC (lossless compression) - for storage
    FLAC,
    /// M4A (AAC compression) - for storage
    M4A,
    /// MP3 (lossy compression) - for compatibility
    MP3,
}

impl AudioFormat {
    /// Get file extension for this format
    #[must_use]
    pub fn extension(&self) -> &str {
        match self {
            AudioFormat::PCM => "wav",
            AudioFormat::FLAC => "flac",
            AudioFormat::M4A => "m4a",
            AudioFormat::MP3 => "mp3",
        }
    }

    /// Get `FFmpeg` codec name for this format
    fn codec_name(&self) -> &str {
        match self {
            AudioFormat::PCM => "pcm_s16le",
            AudioFormat::FLAC => "flac",
            AudioFormat::M4A => "aac",
            AudioFormat::MP3 => "libmp3lame",
        }
    }
}

/// Audio extraction configuration
#[derive(Debug, Clone)]
pub struct AudioConfig {
    /// Target sample rate (16000 for ML, 48000 for storage)
    pub sample_rate: u32,
    /// Number of channels (1 for mono, 2 for stereo)
    pub channels: u8,
    /// Output audio format
    pub format: AudioFormat,
    /// Apply EBU R128 normalization to -23 LUFS
    pub normalize: bool,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: 16000,
            channels: 1,
            format: AudioFormat::PCM,
            normalize: false,
        }
    }
}

impl AudioConfig {
    /// Create config optimized for ML models (16kHz mono PCM)
    #[must_use]
    pub fn for_ml() -> Self {
        Self {
            sample_rate: 16000,
            channels: 1,
            format: AudioFormat::PCM,
            normalize: true,
        }
    }

    /// Create config optimized for storage (48kHz stereo FLAC)
    #[must_use]
    pub fn for_storage() -> Self {
        Self {
            sample_rate: 48000,
            channels: 2,
            format: AudioFormat::FLAC,
            normalize: false,
        }
    }
}

/// Check if a file has an audio stream using C FFI (zero-overhead, no process spawn)
///
/// # Arguments
/// * `input_path` - Path to input video or audio file
///
/// # Returns
/// `Ok(true)` if audio stream exists, `Ok(false)` if no audio stream
///
/// # Errors
/// Returns error if file cannot be opened
fn has_audio_stream_cffi(input_path: &Path) -> Result<bool> {
    let format_ctx = FormatContext::open(input_path)?;
    match format_ctx.find_audio_stream() {
        Ok(_) => Ok(true),
        Err(ProcessingError::FFmpegError(msg)) if msg.contains("av_find_best_stream failed") => {
            Ok(false)
        }
        Err(e) => Err(e),
    }
}

/// Extract audio from video/audio file
///
/// Uses C FFI for PCM/WAV format without normalization (zero process spawn overhead).
/// Falls back to FFmpeg CLI for compressed formats (FLAC, M4A, MP3) or when normalization is required.
///
/// # Arguments
/// * `input_path` - Path to input video or audio file
/// * `output_path` - Path for output audio file (extension will match config.format)
/// * `config` - Audio extraction configuration
///
/// # Returns
/// Path to the extracted audio file
///
/// # Errors
/// Returns error if:
/// - Input file has no audio stream
/// - Audio processing fails
/// - Output file cannot be written
pub fn extract_audio(
    input_path: &Path,
    output_path: &Path,
    config: &AudioConfig,
) -> Result<PathBuf> {
    // First, check if input has audio stream using C FFI (no process spawn)
    if !has_audio_stream_cffi(input_path)? {
        return Err(ProcessingError::FFmpegError(format!(
            "No audio stream found in input file: {}",
            input_path.display()
        )));
    }

    let output_path_with_ext = output_path.with_extension(config.format.extension());

    // Use C FFI path for PCM/WAV without normalization (FAST - no process spawn)
    if config.format == AudioFormat::PCM && !config.normalize {
        video_audio_decoder::c_ffi::extract_audio_to_wav(
            input_path,
            &output_path_with_ext,
            config.sample_rate,
            config.channels as u32,
        )?;
        return Ok(output_path_with_ext);
    }

    // Fall back to FFmpeg CLI for:
    // - Compressed formats (FLAC, M4A, MP3) which require encoders
    // - Normalization (loudnorm filter not yet implemented in C FFI)
    extract_audio_ffmpeg_cli(input_path, &output_path_with_ext, config)
}

/// Extract audio using FFmpeg CLI (supports all formats and normalization)
fn extract_audio_ffmpeg_cli(
    input_path: &Path,
    output_path: &Path,
    config: &AudioConfig,
) -> Result<PathBuf> {
    // Build FFmpeg command
    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-i")
        .arg(input_path)
        .arg("-vn") // No video
        .arg("-acodec")
        .arg(config.format.codec_name())
        .arg("-ar")
        .arg(config.sample_rate.to_string())
        .arg("-ac")
        .arg(config.channels.to_string());

    // Add bitrate for compressed formats
    match config.format {
        AudioFormat::M4A | AudioFormat::MP3 => {
            cmd.arg("-b:a").arg("128k");
        }
        _ => {}
    }

    // Add normalization filter if requested
    if config.normalize {
        cmd.arg("-af").arg("loudnorm=I=-23:LRA=7:TP=-2");
    }

    cmd.arg("-y") // Overwrite output
        .arg(output_path);

    // Execute FFmpeg
    let output = cmd
        .output()
        .map_err(|e| ProcessingError::FFmpegError(format!("Failed to execute FFmpeg: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ProcessingError::FFmpegError(format!(
            "FFmpeg failed: {stderr}"
        )));
    }

    // Verify output file exists
    if !output_path.exists() {
        return Err(ProcessingError::FFmpegError(
            "Output file was not created".to_string(),
        ));
    }

    Ok(output_path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_format_extension() {
        assert_eq!(AudioFormat::PCM.extension(), "wav");
        assert_eq!(AudioFormat::FLAC.extension(), "flac");
        assert_eq!(AudioFormat::M4A.extension(), "m4a");
        assert_eq!(AudioFormat::MP3.extension(), "mp3");
    }

    #[test]
    fn test_audio_config_default() {
        let config = AudioConfig::default();
        assert_eq!(config.sample_rate, 16000);
        assert_eq!(config.channels, 1);
        assert_eq!(config.format, AudioFormat::PCM);
        assert!(!config.normalize);
    }

    #[test]
    fn test_audio_config_for_ml() {
        let config = AudioConfig::for_ml();
        assert_eq!(config.sample_rate, 16000);
        assert_eq!(config.channels, 1);
        assert_eq!(config.format, AudioFormat::PCM);
        assert!(config.normalize);
    }

    #[test]
    fn test_audio_config_for_storage() {
        let config = AudioConfig::for_storage();
        assert_eq!(config.sample_rate, 48000);
        assert_eq!(config.channels, 2);
        assert_eq!(config.format, AudioFormat::FLAC);
        assert!(!config.normalize);
    }
}
