//! Short-Time Fourier Transform (STFT) and inverse STFT (iSTFT)
//!
//! This module provides STFT/iSTFT implementations for audio signal processing.
//! Used for converting between time-domain waveforms and frequency-domain spectrograms.
//!
//! # STFT (Short-Time Fourier Transform)
//! Converts time-domain audio into time-frequency representation (spectrogram).
//! - Input: Audio waveform (samples)
//! - Output: Complex spectrogram (real + imaginary components)
//!
//! # iSTFT (Inverse STFT)
//! Converts time-frequency representation back to time-domain audio.
//! - Input: Complex spectrogram
//! - Output: Audio waveform (samples)
//!
//! # Example
//! ```no_run
//! use music_source_separation::stft::{StftConfig, stft, istft};
//!
//! let audio = vec![0.0f32; 44100]; // 1 second at 44.1kHz
//! let config = StftConfig::default();
//!
//! // Forward STFT
//! let spectrogram = stft(&audio, &config);
//!
//! // Inverse STFT (should reconstruct original audio)
//! let reconstructed = istft(&spectrogram, &config, audio.len());
//! ```

use rustfft::{num_complex::Complex, FftPlanner};
use std::f32::consts::PI;
use thiserror::Error;
use tracing::debug;

/// STFT/iSTFT errors
#[derive(Debug, Error)]
pub enum StftError {
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("Audio too short: minimum {min} samples required, got {actual}")]
    AudioTooShort { min: usize, actual: usize },
    #[error("Spectrogram processing failed: {0}")]
    ProcessingFailed(String),
}

/// STFT configuration
#[derive(Debug, Clone)]
pub struct StftConfig {
    /// FFT size (window size in samples)
    pub n_fft: usize,
    /// Hop length (stride between windows in samples)
    pub hop_length: usize,
    /// Window type
    pub window: WindowType,
    /// Center the STFT windows (pad input with n_fft/2 zeros on each side)
    pub center: bool,
}

/// Window function types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowType {
    /// Hann window (raised cosine)
    Hann,
    /// Hamming window
    Hamming,
    /// Rectangular window (no windowing)
    Rectangular,
}

impl Default for StftConfig {
    fn default() -> Self {
        Self {
            n_fft: 2048,
            hop_length: 512,
            window: WindowType::Hann,
            center: true,
        }
    }
}

impl StftConfig {
    /// Create config for Demucs model (44.1kHz audio)
    ///
    /// Based on Demucs htdemucs_6s.onnx requirements:
    /// - Input waveform: [1, 2, 343980] (~7.8s at 44.1kHz)
    /// - Input spectrogram: [1, 4, 2048, 336]
    /// - 2048 frequency bins, 336 time frames
    #[must_use]
    pub fn demucs() -> Self {
        // Demucs uses:
        // - n_fft = 4096 (2048 complex bins → 2048 magnitude bins)
        // - hop_length = 1024 (4x overlap)
        // - 343980 samples / 1024 hop = 336 frames
        Self {
            n_fft: 4096,
            hop_length: 1024,
            window: WindowType::Hann,
            center: true,
        }
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), StftError> {
        if self.n_fft == 0 {
            return Err(StftError::InvalidConfig("n_fft must be > 0".to_string()));
        }
        if self.hop_length == 0 {
            return Err(StftError::InvalidConfig(
                "hop_length must be > 0".to_string(),
            ));
        }
        if self.hop_length > self.n_fft {
            return Err(StftError::InvalidConfig(
                "hop_length must be <= n_fft".to_string(),
            ));
        }
        Ok(())
    }
}

/// Generate window function
fn generate_window(size: usize, window_type: WindowType) -> Vec<f32> {
    match window_type {
        WindowType::Hann => (0..size)
            .map(|i| 0.5 * (1.0 - (2.0 * PI * i as f32 / size as f32).cos()))
            .collect(),
        WindowType::Hamming => (0..size)
            .map(|i| 0.54 - 0.46 * (2.0 * PI * i as f32 / size as f32).cos())
            .collect(),
        WindowType::Rectangular => vec![1.0; size],
    }
}

/// Short-Time Fourier Transform (STFT)
///
/// Converts time-domain audio into complex spectrogram.
///
/// # Arguments
/// * `audio` - Input audio waveform (mono)
/// * `config` - STFT configuration
///
/// # Returns
/// Complex spectrogram: Vec of frames, each frame is Vec of Complex<f32>
/// - Outer vec: time frames (length = (audio.len() + n_fft/2 + hop_length - 1) / hop_length if center=true)
/// - Inner vec: frequency bins (length = n_fft/2 + 1, only positive frequencies)
///
/// # Errors
/// Returns error if configuration is invalid or audio is too short
pub fn stft(audio: &[f32], config: &StftConfig) -> Result<Vec<Vec<Complex<f32>>>, StftError> {
    config.validate()?;

    let n_fft = config.n_fft;
    let hop_length = config.hop_length;

    // Pad audio if center=true
    let audio_padded = if config.center {
        let pad_length = n_fft / 2;
        let mut padded = vec![0.0; pad_length];
        padded.extend_from_slice(audio);
        padded.extend(vec![0.0; pad_length]);
        padded
    } else {
        audio.to_vec()
    };

    // Check minimum length
    if audio_padded.len() < n_fft {
        return Err(StftError::AudioTooShort {
            min: n_fft,
            actual: audio_padded.len(),
        });
    }

    // Generate window
    let window = generate_window(n_fft, config.window);

    // Calculate number of frames
    let num_frames = (audio_padded.len() - n_fft) / hop_length + 1;
    debug!(
        "STFT: {} samples → {} frames (n_fft={}, hop={})",
        audio.len(),
        num_frames,
        n_fft,
        hop_length
    );

    // Create FFT planner
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(n_fft);

    // Process each frame
    let mut spectrogram = Vec::with_capacity(num_frames);
    for frame_idx in 0..num_frames {
        let start = frame_idx * hop_length;
        let end = start + n_fft;

        // Extract frame and apply window
        let mut buffer: Vec<Complex<f32>> = audio_padded[start..end]
            .iter()
            .zip(&window)
            .map(|(&sample, &win)| Complex::new(sample * win, 0.0))
            .collect();

        // Compute FFT
        fft.process(&mut buffer);

        // Store only positive frequencies (n_fft/2 bins for Demucs)
        // Real signals have symmetric FFT, so we only need first half
        // NOTE: Standard is n_fft/2 + 1, but Demucs expects exactly n_fft/2 (2048)
        let num_bins = n_fft / 2;
        spectrogram.push(buffer[..num_bins].to_vec());
    }

    Ok(spectrogram)
}

/// Inverse Short-Time Fourier Transform (iSTFT)
///
/// Converts complex spectrogram back to time-domain audio using overlap-add.
///
/// # Arguments
/// * `spectrogram` - Complex spectrogram (frames × frequency bins)
/// * `config` - STFT configuration (must match forward STFT)
/// * `length` - Desired output length (to remove padding)
///
/// # Returns
/// Reconstructed audio waveform
///
/// # Errors
/// Returns error if configuration is invalid or spectrogram is malformed
pub fn istft(
    spectrogram: &[Vec<Complex<f32>>],
    config: &StftConfig,
    length: Option<usize>,
) -> Result<Vec<f32>, StftError> {
    config.validate()?;

    if spectrogram.is_empty() {
        return Err(StftError::ProcessingFailed(
            "Empty spectrogram".to_string(),
        ));
    }

    let n_fft = config.n_fft;
    let hop_length = config.hop_length;
    let num_frames = spectrogram.len();

    // Validate spectrogram shape (n_fft/2 bins for Demucs)
    let expected_bins = n_fft / 2;
    if spectrogram[0].len() != expected_bins {
        return Err(StftError::ProcessingFailed(format!(
            "Expected {} frequency bins, got {}",
            expected_bins,
            spectrogram[0].len()
        )));
    }

    debug!(
        "iSTFT: {} frames × {} bins → audio (n_fft={}, hop={})",
        num_frames,
        expected_bins,
        n_fft,
        hop_length
    );

    // Generate window
    let window = generate_window(n_fft, config.window);

    // Create inverse FFT planner
    let mut planner = FftPlanner::new();
    let ifft = planner.plan_fft_inverse(n_fft);

    // Calculate output length
    let output_length = (num_frames - 1) * hop_length + n_fft;
    let mut output = vec![0.0f32; output_length];
    let mut window_sum = vec![0.0f32; output_length];

    // Process each frame
    for (frame_idx, frame) in spectrogram.iter().enumerate() {
        // Reconstruct full FFT buffer (mirror negative frequencies)
        let mut buffer: Vec<Complex<f32>> = Vec::with_capacity(n_fft);
        buffer.extend_from_slice(frame);

        // Add Nyquist bin (set to zero for Demucs which omits it)
        buffer.push(Complex::new(0.0, 0.0));

        // Add mirrored negative frequencies (conjugate symmetry)
        // For real signals: X[k] = conj(X[N-k])
        // Skip DC (k=0) bin, mirror bins 1..(n_fft/2)
        for i in (1..(n_fft / 2)).rev() {
            buffer.push(Complex::new(frame[i].re, -frame[i].im)); // Complex conjugate
        }

        // Compute inverse FFT
        ifft.process(&mut buffer);

        // Overlap-add with windowing
        let start = frame_idx * hop_length;
        for (i, &window_val) in window.iter().enumerate() {
            let pos = start + i;
            if pos < output_length {
                // rustfft inverse FFT doesn't normalize, so divide by n_fft
                output[pos] += buffer[i].re * window_val / n_fft as f32;
                window_sum[pos] += window_val * window_val;
            }
        }
    }

    // Normalize by window sum (for overlap-add reconstruction)
    for i in 0..output_length {
        if window_sum[i] > 1e-8 {
            output[i] /= window_sum[i];
        }
    }

    // Remove padding if center=true
    let final_output = if config.center {
        let pad_length = n_fft / 2;
        if output.len() < 2 * pad_length {
            return Err(StftError::ProcessingFailed(
                "Output too short to remove padding".to_string(),
            ));
        }
        output[pad_length..(output.len() - pad_length)].to_vec()
    } else {
        output
    };

    // Trim to desired length
    let final_output = if let Some(len) = length {
        final_output[..len.min(final_output.len())].to_vec()
    } else {
        final_output
    };

    Ok(final_output)
}

/// Compute magnitude spectrogram from complex spectrogram
pub fn magnitude_spectrogram(spectrogram: &[Vec<Complex<f32>>]) -> Vec<Vec<f32>> {
    spectrogram
        .iter()
        .map(|frame| {
            frame
                .iter()
                .map(|c| (c.re * c.re + c.im * c.im).sqrt())
                .collect()
        })
        .collect()
}

/// Compute power spectrogram from complex spectrogram
pub fn power_spectrogram(spectrogram: &[Vec<Complex<f32>>]) -> Vec<Vec<f32>> {
    spectrogram
        .iter()
        .map(|frame| frame.iter().map(|c| c.re * c.re + c.im * c.im).collect())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    #[test]
    fn test_stft_config_validation() {
        let valid = StftConfig::default();
        assert!(valid.validate().is_ok());

        let invalid_nfft = StftConfig {
            n_fft: 0,
            ..Default::default()
        };
        assert!(invalid_nfft.validate().is_err());

        let invalid_hop = StftConfig {
            hop_length: 0,
            ..Default::default()
        };
        assert!(invalid_hop.validate().is_err());

        let invalid_hop_gt_nfft = StftConfig {
            n_fft: 512,
            hop_length: 1024,
            ..Default::default()
        };
        assert!(invalid_hop_gt_nfft.validate().is_err());
    }

    #[test]
    fn test_window_generation() {
        // Test Hann window
        let hann = generate_window(4, WindowType::Hann);
        assert_eq!(hann.len(), 4);
        assert!((hann[0] - 0.0).abs() < 1e-6); // Start at 0
        assert!((hann[2] - 1.0).abs() < 1e-6); // Peak at center

        // Test rectangular window
        let rect = generate_window(4, WindowType::Rectangular);
        assert_eq!(rect.len(), 4);
        assert!(rect.iter().all(|&x| (x - 1.0).abs() < 1e-6));
    }

    #[test]
    fn test_stft_istft_roundtrip() {
        // Create a simple sine wave
        let sample_rate = 16000.0;
        let duration = 1.0; // 1 second
        let freq = 440.0; // A4 note
        let num_samples = (sample_rate * duration) as usize;

        let audio: Vec<f32> = (0..num_samples)
            .map(|i| (2.0 * PI * freq * i as f32 / sample_rate).sin())
            .collect();

        // STFT configuration
        let config = StftConfig {
            n_fft: 512,
            hop_length: 128,
            window: WindowType::Hann,
            center: true,
        };

        // Forward STFT
        let spectrogram = stft(&audio, &config).expect("STFT failed");
        assert!(!spectrogram.is_empty());
        assert_eq!(spectrogram[0].len(), config.n_fft / 2 + 1);

        // Inverse STFT
        let reconstructed = istft(&spectrogram, &config, Some(audio.len())).expect("iSTFT failed");
        assert_eq!(reconstructed.len(), audio.len());

        // Check reconstruction error (should be very small)
        // Allow some error due to windowing and edge effects
        let mut max_error: f32 = 0.0;
        let mut mean_error: f32 = 0.0;
        for (i, (&original, &recon)) in audio.iter().zip(&reconstructed).enumerate() {
            // Skip edges (first/last 10% of signal) where windowing causes issues
            if i < num_samples / 10 || i > num_samples * 9 / 10 {
                continue;
            }
            let error = (original - recon).abs();
            max_error = max_error.max(error);
            mean_error += error;
        }
        mean_error /= (num_samples * 8 / 10) as f32;

        // Reconstruction should be very accurate in the middle
        assert!(
            mean_error < 0.01,
            "Mean reconstruction error too high: {}",
            mean_error
        );
        assert!(
            max_error < 0.1,
            "Max reconstruction error too high: {}",
            max_error
        );
    }

    #[test]
    fn test_stft_too_short() {
        let audio = vec![0.0f32; 100]; // Too short
        let config = StftConfig {
            n_fft: 512,
            hop_length: 128,
            center: false, // No padding
            ..Default::default()
        };

        let result = stft(&audio, &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_demucs_config() {
        let config = StftConfig::demucs();
        assert_eq!(config.n_fft, 4096);
        assert_eq!(config.hop_length, 1024);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_magnitude_spectrogram() {
        let complex_spec = vec![vec![
            Complex::new(3.0, 4.0), // magnitude = 5.0
            Complex::new(0.0, 0.0), // magnitude = 0.0
            Complex::new(1.0, 0.0), // magnitude = 1.0
        ]];

        let mag_spec = magnitude_spectrogram(&complex_spec);
        assert_eq!(mag_spec.len(), 1);
        assert_eq!(mag_spec[0].len(), 3);
        assert!((mag_spec[0][0] - 5.0).abs() < 1e-6);
        assert!((mag_spec[0][1] - 0.0).abs() < 1e-6);
        assert!((mag_spec[0][2] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_power_spectrogram() {
        let complex_spec = vec![vec![
            Complex::new(3.0, 4.0), // power = 25.0
            Complex::new(0.0, 0.0), // power = 0.0
            Complex::new(1.0, 0.0), // power = 1.0
        ]];

        let pow_spec = power_spectrogram(&complex_spec);
        assert_eq!(pow_spec.len(), 1);
        assert_eq!(pow_spec[0].len(), 3);
        assert!((pow_spec[0][0] - 25.0).abs() < 1e-6);
        assert!((pow_spec[0][1] - 0.0).abs() < 1e-6);
        assert!((pow_spec[0][2] - 1.0).abs() < 1e-6);
    }
}
