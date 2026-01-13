//! Audio format support for `docling_rs`
//!
//! This crate provides audio file parsing and optional transcription support for:
//! - WAV (Waveform Audio Format)
//! - MP3 (MPEG-1 Audio Layer 3)
//!
//! ## Features
//!
//! - `transcription`: Enable audio-to-text transcription using Whisper
//!
//! ## Usage
//!
//! ### Basic Audio Parsing (Metadata Only)
//!
//! ```no_run
//! use docling_audio::{parse_wav, parse_mp3};
//!
//! // Parse WAV file
//! let audio_info = parse_wav("recording.wav")?;
//! println!("Duration: {:.2}s, Sample rate: {}Hz",
//!     audio_info.duration_secs, audio_info.sample_rate);
//!
//! // Parse MP3 file
//! let audio_info = parse_mp3("podcast.mp3")?;
//! println!("Duration: {:.2}s, Channels: {}",
//!     audio_info.duration_secs, audio_info.channels);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ### Audio Transcription (requires `transcription` feature)
//!
//! ```ignore
//! use docling_audio::transcribe_audio;
//!
//! // Transcribe audio file to text
//! let transcript = transcribe_audio("meeting.wav", None)?;
//! println!("Transcript: {}", transcript.text);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub mod error;
pub mod mp3;
pub mod wav;

#[cfg(feature = "transcription")]
pub mod transcription;

pub use error::{AudioError, Result};
pub use mp3::{parse_mp3, Mp3Info};
pub use wav::{parse_wav, WavInfo};

#[cfg(feature = "transcription")]
pub use transcription::{transcribe_audio, TranscriptionConfig, TranscriptionResult};

/// Unified audio information struct
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AudioInfo {
    /// Sample rate in Hz (e.g., 44100, 48000)
    pub sample_rate: u32,

    /// Number of audio channels (1 = mono, 2 = stereo)
    pub channels: u16,

    /// Duration in seconds
    pub duration_secs: f64,

    /// Bit depth (e.g., 16, 24) - None for compressed formats like MP3
    pub bit_depth: Option<u16>,

    /// Total number of samples
    pub total_samples: u64,

    /// Format name ("WAV", "MP3", etc.)
    pub format: String,
}
