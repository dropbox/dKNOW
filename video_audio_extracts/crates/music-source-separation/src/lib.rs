//! Music source separation module using Demucs/Spleeter via ONNX Runtime
//!
//! This module provides music source separation to isolate vocals, drums, bass,
//! and other instruments from audio tracks. Useful for karaoke, remixing, audio analysis,
//! and music production workflows.
//!
//! # Features
//! - Separates music into multiple stems (vocals, drums, bass, other, etc.)
//! - Supports Demucs and Spleeter architectures (user-provided ONNX models)
//! - Configurable stem selection (separate all or specific stems only)
//! - High-quality audio output (44.1kHz sampling rate)
//! - Hardware acceleration via ONNX Runtime (CoreML on macOS, CUDA on Linux)
//!
//! # Model Details
//! - Model: User-provided Demucs or Spleeter ONNX model
//! - Input: Audio waveform (44.1kHz mono or stereo)
//! - Output: Separated audio waveforms for each stem
//! - Typical stems: vocals, drums, bass, other (Demucs 4-stem)
//!
//! # Example
//! ```no_run
//! use video_audio_music_source_separation::{MusicSourceSeparator, SourceSeparationConfig};
//!
//! # fn main() -> anyhow::Result<()> {
//! let config = SourceSeparationConfig::default();
//! let mut separator = MusicSourceSeparator::new(
//!     "models/music-source-separation/demucs.onnx",
//!     "models/music-source-separation/stems.txt",
//!     config
//! )?;
//!
//! let audio_samples = vec![0.0f32; 44100];  // 1 second at 44.1kHz
//! let results = separator.separate(&audio_samples)?;
//!
//! for result in results {
//!     println!("{}: {} samples", result.stem_name, result.audio.len());
//! }
//! # Ok(())
//! # }
//! ```

pub mod plugin;
pub mod stft;

use ndarray::s;
use ort::{
    session::Session,
    value::Value,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use thiserror::Error;
use tracing::{debug, info};
use video_audio_common::ProcessingError;

/// Music source separation errors
#[derive(Debug, Error)]
pub enum SourceSeparationError {
    #[error("Model loading failed: {0}")]
    ModelLoad(String),
    #[error("Inference failed: {0}")]
    Inference(String),
    #[error("Invalid audio length: minimum {min} samples required, got {actual}")]
    InvalidAudioLength { min: usize, actual: usize },
    #[error("Stem map loading failed: {0}")]
    StemMapLoad(String),
    #[error("Audio processing failed: {0}")]
    AudioProcessing(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<SourceSeparationError> for ProcessingError {
    fn from(err: SourceSeparationError) -> Self {
        ProcessingError::Other(err.to_string())
    }
}

/// Configuration for music source separation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceSeparationConfig {
    /// Target sample rate for processing (Demucs typically uses 44100Hz)
    pub sample_rate: u32,
    /// Stems to extract (empty = all stems, otherwise filter by name)
    pub stems_filter: Vec<String>,
    /// Minimum segment length in samples (for processing efficiency)
    pub min_segment_length: usize,
}

impl Default for SourceSeparationConfig {
    fn default() -> Self {
        Self {
            sample_rate: 44100,
            stems_filter: vec![],      // Extract all stems by default
            min_segment_length: 44100, // 1 second minimum
        }
    }
}

impl SourceSeparationConfig {
    /// Create a config for vocals-only extraction (karaoke use case)
    #[must_use]
    pub fn vocals_only() -> Self {
        Self {
            sample_rate: 44100,
            stems_filter: vec!["vocals".to_string()],
            min_segment_length: 44100,
        }
    }

    /// Create a config for drums-only extraction (beat analysis use case)
    #[must_use]
    pub fn drums_only() -> Self {
        Self {
            sample_rate: 44100,
            stems_filter: vec!["drums".to_string()],
            min_segment_length: 44100,
        }
    }
}

/// Result of source separation for a single stem
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeparatedStem {
    /// Name of the stem (e.g., "vocals", "drums", "bass", "other")
    pub stem_name: String,
    /// Separated audio samples (44.1kHz, mono or stereo depending on model)
    pub audio: Vec<f32>,
    /// Number of channels (1 for mono, 2 for stereo)
    pub channels: usize,
}

/// Music source separator using ONNX Runtime
pub struct MusicSourceSeparator {
    _session: Session,
    _stem_names: Vec<String>,
    config: SourceSeparationConfig,
}

impl MusicSourceSeparator {
    /// Create a new music source separator
    ///
    /// # Arguments
    /// * `model_path` - Path to ONNX model file (e.g., demucs.onnx, spleeter.onnx)
    /// * `stem_names_path` - Path to stem names file (one stem name per line)
    /// * `config` - Source separation configuration
    ///
    /// # Errors
    /// Returns error if model or stem names file cannot be loaded
    pub fn new(
        model_path: impl AsRef<Path>,
        stem_names_path: impl AsRef<Path>,
        config: SourceSeparationConfig,
    ) -> Result<Self, SourceSeparationError> {
        let model_path = model_path.as_ref();
        let stem_names_path = stem_names_path.as_ref();

        // Load ONNX model
        info!(
            "Loading music source separation model from {:?}",
            model_path
        );
        let session = Session::builder()
            .map_err(|e| SourceSeparationError::ModelLoad(e.to_string()))?
            .commit_from_file(model_path)
            .map_err(|e| SourceSeparationError::ModelLoad(e.to_string()))?;

        // Load stem names from file
        info!("Loading stem names from {:?}", stem_names_path);
        let stem_names = Self::load_stem_names(stem_names_path)?;
        info!("Loaded {} stems: {:?}", stem_names.len(), stem_names);

        Ok(Self {
            _session: session,
            _stem_names: stem_names,
            config,
        })
    }

    /// Load stem names from text file (one name per line)
    fn load_stem_names(path: impl AsRef<Path>) -> Result<Vec<String>, SourceSeparationError> {
        let content = fs::read_to_string(path.as_ref())
            .map_err(|e| SourceSeparationError::StemMapLoad(format!("Cannot read file: {}", e)))?;

        let stems: Vec<String> = content
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(String::from)
            .collect();

        if stems.is_empty() {
            return Err(SourceSeparationError::StemMapLoad(
                "Stem names file is empty".to_string(),
            ));
        }

        Ok(stems)
    }

    /// Separate audio into multiple stems using Demucs
    ///
    /// # Arguments
    /// * `audio` - Audio samples (mono or stereo interleaved, at configured sample rate)
    ///
    /// # Returns
    /// Vector of separated stems, each containing audio samples and metadata
    ///
    /// # Errors
    /// Returns error if audio is too short or inference fails
    pub fn separate(&mut self, audio: &[f32]) -> Result<Vec<SeparatedStem>, SourceSeparationError> {
        use crate::stft::{StftConfig, stft, istft};

        if audio.len() < self.config.min_segment_length {
            return Err(SourceSeparationError::InvalidAudioLength {
                min: self.config.min_segment_length,
                actual: audio.len(),
            });
        }

        debug!(
            "Separating audio: {} samples at {}Hz",
            audio.len(),
            self.config.sample_rate
        );

        // Demucs htdemucs_6s.onnx expects:
        // Input 1: waveform [1, 2, 343980] - batch=1, channels=2 (stereo), samples=343980
        // Input 2: spectrogram [1, 4, 2048, 336] - batch=1, channels=4 (real+imag×2), freq_bins=2048, frames=336
        // Output: spectrograms [1, 6, 4, 2048, 336] - batch=1, stems=6, channels=4, freq_bins=2048, frames=336

        const TARGET_SAMPLES: usize = 343980; // ~7.8s at 44.1kHz
        const NUM_STEMS: usize = 6;

        // Step 1: Preprocess audio to stereo and pad/trim to target length
        let (left, right) = self.preprocess_audio_to_stereo(audio, TARGET_SAMPLES)?;

        // Step 2: Compute STFT for both channels
        let stft_config = StftConfig::demucs();
        debug!("Computing STFT with n_fft={}, hop_length={}", stft_config.n_fft, stft_config.hop_length);

        let stft_left = stft(&left, &stft_config)
            .map_err(|e| SourceSeparationError::AudioProcessing(format!("STFT left failed: {}", e)))?;
        let stft_right = stft(&right, &stft_config)
            .map_err(|e| SourceSeparationError::AudioProcessing(format!("STFT right failed: {}", e)))?;

        // Step 3: Convert complex spectrogram to [1, 4, 2048, 336] format
        // 4 channels = [left_real, left_imag, right_real, right_imag]
        let spectrogram_input = self.pack_stereo_spectrogram(&stft_left, &stft_right)?;
        debug!("Spectrogram shape: {:?}", spectrogram_input.shape());

        // Step 4: Prepare waveform input [1, 2, 343980]
        let waveform_input = self.pack_stereo_waveform(&left, &right);
        debug!("Waveform shape: {:?}", waveform_input.shape());

        // Step 5: Convert to ORT Value tensors
        let waveform_value = Value::from_array(waveform_input)
            .map_err(|e| SourceSeparationError::Inference(format!("Failed to create waveform tensor: {}", e)))?;
        let spectrogram_value = Value::from_array(spectrogram_input)
            .map_err(|e| SourceSeparationError::Inference(format!("Failed to create spectrogram tensor: {}", e)))?;

        // Step 6: Run ONNX inference
        debug!("Running Demucs inference...");
        let outputs = self._session.run(ort::inputs![
            "input" => waveform_value,
            "onnx::ReduceMean_1" => spectrogram_value
        ])
            .map_err(|e| SourceSeparationError::Inference(format!("ONNX inference failed: {}", e)))?;

        // Step 7: Extract output spectrograms [1, 6, 4, 2048, 336]
        let (output_shape, output_data) = outputs["output"]
            .try_extract_tensor::<f32>()
            .map_err(|e| SourceSeparationError::Inference(format!("Failed to extract output tensor: {}", e)))?;

        debug!("Output tensor shape: {:?}", output_shape);

        // Convert to ndarray (shape should be [1, 6, 4, 2048, 336])
        if output_shape.len() != 5 {
            return Err(SourceSeparationError::Inference(format!(
                "Expected 5D output tensor, got {}D", output_shape.len())));
        }

        let output_spectrograms = ndarray::ArrayView::from_shape(
            (output_shape[0] as usize, output_shape[1] as usize, output_shape[2] as usize,
             output_shape[3] as usize, output_shape[4] as usize),
            output_data
        ).map_err(|e| SourceSeparationError::Inference(format!("Failed to reshape output: {}", e)))?
            .to_owned();

        // Drop outputs to release borrow on self._session
        drop(outputs);

        debug!("Output spectrograms shape: {:?}", output_spectrograms.shape());

        // Step 8: Convert each stem's spectrogram back to waveform via iSTFT
        let mut stems = Vec::with_capacity(NUM_STEMS);
        for stem_idx in 0..NUM_STEMS {
            let stem_name = self._stem_names.get(stem_idx)
                .ok_or_else(|| SourceSeparationError::AudioProcessing(
                    format!("Stem {} not found in stem names", stem_idx)))?;

            // Skip if user filtered this stem
            if !self.config.stems_filter.is_empty()
                && !self.config.stems_filter.contains(stem_name) {
                continue;
            }

            debug!("Processing stem {}: {}", stem_idx, stem_name);

            // Extract this stem's spectrogram [4, 2048, 336]
            let stem_spec = output_spectrograms.slice(s![0, stem_idx, .., .., ..]);

            // Unpack to left/right complex spectrograms
            let (stft_left, stft_right) = self.unpack_stereo_spectrogram(&stem_spec)?;

            // iSTFT to reconstruct waveforms
            let left_audio = istft(&stft_left, &stft_config, Some(TARGET_SAMPLES))
                .map_err(|e| SourceSeparationError::AudioProcessing(format!("iSTFT left failed: {}", e)))?;
            let right_audio = istft(&stft_right, &stft_config, Some(TARGET_SAMPLES))
                .map_err(|e| SourceSeparationError::AudioProcessing(format!("iSTFT right failed: {}", e)))?;

            // Interleave stereo channels
            let interleaved = self.interleave_stereo(&left_audio, &right_audio);

            stems.push(SeparatedStem {
                stem_name: stem_name.clone(),
                audio: interleaved,
                channels: 2, // Stereo output
            });
        }

        info!("Separated {} stems successfully", stems.len());
        Ok(stems)
    }

    /// Preprocess audio to stereo and pad/trim to target length
    fn preprocess_audio_to_stereo(&self, audio: &[f32], target_samples: usize)
        -> Result<(Vec<f32>, Vec<f32>), SourceSeparationError> {
        // Assume input is stereo interleaved [L, R, L, R, ...]
        // If mono, duplicate to stereo
        let (mut left, mut right) = if audio.len() % 2 == 0 {
            // Stereo interleaved
            let left: Vec<f32> = audio.iter().step_by(2).copied().collect();
            let right: Vec<f32> = audio.iter().skip(1).step_by(2).copied().collect();
            (left, right)
        } else {
            // Mono - duplicate to stereo
            (audio.to_vec(), audio.to_vec())
        };

        // Pad or trim to target length
        left.resize(target_samples, 0.0);
        right.resize(target_samples, 0.0);

        Ok((left, right))
    }

    /// Pack stereo complex spectrogram into [1, 4, freq_bins, time_frames] format
    /// 4 channels = [left_real, left_imag, right_real, right_imag]
    fn pack_stereo_spectrogram(&self, left: &[Vec<rustfft::num_complex::Complex<f32>>],
                                 right: &[Vec<rustfft::num_complex::Complex<f32>>])
        -> Result<ndarray::Array4<f32>, SourceSeparationError> {
        use ndarray::Array4;

        let time_frames = left.len();
        let freq_bins = left[0].len();

        if right.len() != time_frames || right[0].len() != freq_bins {
            return Err(SourceSeparationError::AudioProcessing(
                "Left/right spectrogram size mismatch".to_string()));
        }

        let mut arr = Array4::<f32>::zeros((1, 4, freq_bins, time_frames));

        for t in 0..time_frames {
            for f in 0..freq_bins {
                arr[[0, 0, f, t]] = left[t][f].re;  // Left real
                arr[[0, 1, f, t]] = left[t][f].im;  // Left imaginary
                arr[[0, 2, f, t]] = right[t][f].re; // Right real
                arr[[0, 3, f, t]] = right[t][f].im; // Right imaginary
            }
        }

        Ok(arr)
    }

    /// Pack stereo waveform into [1, 2, samples] format
    fn pack_stereo_waveform(&self, left: &[f32], right: &[f32]) -> ndarray::Array3<f32> {
        use ndarray::Array3;

        let samples = left.len();
        let mut arr = Array3::<f32>::zeros((1, 2, samples));

        for i in 0..samples {
            arr[[0, 0, i]] = left[i];
            arr[[0, 1, i]] = right[i];
        }

        arr
    }

    /// Unpack [4, freq_bins, time_frames] spectrogram to left/right complex spectrograms
    fn unpack_stereo_spectrogram(&self, packed: &ndarray::ArrayView3<f32>)
        -> Result<(Vec<Vec<rustfft::num_complex::Complex<f32>>>,
                   Vec<Vec<rustfft::num_complex::Complex<f32>>>), SourceSeparationError> {
        use rustfft::num_complex::Complex;

        let freq_bins = packed.shape()[1];
        let time_frames = packed.shape()[2];

        let mut left = Vec::with_capacity(time_frames);
        let mut right = Vec::with_capacity(time_frames);

        for t in 0..time_frames {
            let mut left_frame = Vec::with_capacity(freq_bins);
            let mut right_frame = Vec::with_capacity(freq_bins);

            for f in 0..freq_bins {
                left_frame.push(Complex::new(packed[[0, f, t]], packed[[1, f, t]]));
                right_frame.push(Complex::new(packed[[2, f, t]], packed[[3, f, t]]));
            }

            left.push(left_frame);
            right.push(right_frame);
        }

        Ok((left, right))
    }

    /// Interleave stereo channels [L, R, L, R, ...]
    fn interleave_stereo(&self, left: &[f32], right: &[f32]) -> Vec<f32> {
        let mut interleaved = Vec::with_capacity(left.len() + right.len());
        for (l, r) in left.iter().zip(right.iter()) {
            interleaved.push(*l);
            interleaved.push(*r);
        }
        interleaved
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = SourceSeparationConfig::default();
        assert_eq!(config.sample_rate, 44100);
        assert!(config.stems_filter.is_empty());
        assert_eq!(config.min_segment_length, 44100);
    }

    #[test]
    fn test_vocals_only_config() {
        let config = SourceSeparationConfig::vocals_only();
        assert_eq!(config.stems_filter, vec!["vocals"]);
    }

    #[test]
    fn test_drums_only_config() {
        let config = SourceSeparationConfig::drums_only();
        assert_eq!(config.stems_filter, vec!["drums"]);
    }
}
