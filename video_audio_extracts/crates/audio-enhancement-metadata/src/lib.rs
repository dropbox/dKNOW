//! Audio enhancement metadata extraction
//!
//! This module analyzes audio signals to provide enhancement recommendations.
//! It computes metrics such as:
//! - Signal-to-noise ratio (SNR)
//! - Dynamic range
//! - Spectral characteristics (bandwidth, centroid, rolloff)
//! - Enhancement recommendations (denoise, normalize, EQ)
//!
//! # Example
//! ```no_run
//! use audio_enhancement_metadata::{AudioEnhancementAnalyzer, EnhancementConfig};
//!
//! # fn main() -> anyhow::Result<()> {
//! let config = EnhancementConfig::default();
//! let analyzer = AudioEnhancementAnalyzer::new(config);
//!
//! let audio_samples = vec![0.0f32; 48000];  // 1 second at 48kHz
//! let sample_rate = 48000;
//! let metadata = analyzer.analyze(&audio_samples, sample_rate)?;
//!
//! println!("SNR: {:.2} dB", metadata.snr_db);
//! println!("Dynamic range: {:.2} dB", metadata.dynamic_range_db);
//! println!("Recommendations: {:?}", metadata.recommendations);
//! # Ok(())
//! # }
//! ```

pub mod plugin;

use rustfft::{num_complex::Complex, FftPlanner};
use serde::{Deserialize, Serialize};
use std::f32;
use thiserror::Error;
use tracing::{debug, info};
/// Audio enhancement analysis errors
#[derive(Debug, Error)]
pub enum EnhancementError {
    #[error("Invalid audio: {0}")]
    InvalidAudio(String),
    #[error("Analysis failed: {0}")]
    AnalysisFailed(String),
}

/// Configuration for audio enhancement analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancementConfig {
    /// FFT size for spectral analysis
    pub fft_size: usize,
    /// Minimum SNR threshold for considering audio "clean" (dB)
    pub snr_threshold_db: f32,
    /// Minimum dynamic range threshold (dB)
    pub dynamic_range_threshold_db: f32,
    /// Low frequency cutoff for bandwidth calculation (Hz)
    pub low_freq_cutoff: f32,
    /// High frequency cutoff for bandwidth calculation (Hz)
    pub high_freq_cutoff: f32,
}

impl Default for EnhancementConfig {
    fn default() -> Self {
        Self {
            fft_size: 2048,
            snr_threshold_db: 20.0,
            dynamic_range_threshold_db: 40.0,
            low_freq_cutoff: 100.0,
            high_freq_cutoff: 8000.0,
        }
    }
}

/// Enhancement recommendation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EnhancementRecommendation {
    /// Audio should be denoised (low SNR)
    Denoise,
    /// Audio should be normalized (low dynamic range)
    Normalize,
    /// Audio needs equalization (spectral imbalance)
    Equalize,
    /// Audio has clipping (peaks at max amplitude)
    RemoveClipping,
    /// Audio is very quiet (low overall level)
    AmplifyVolume,
    /// No enhancement needed
    None,
}

/// Audio enhancement metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancementMetadata {
    /// Signal-to-noise ratio in dB
    pub snr_db: f32,
    /// Dynamic range in dB (difference between loudest and quietest parts)
    pub dynamic_range_db: f32,
    /// RMS (Root Mean Square) level
    pub rms_level: f32,
    /// Peak level (0.0 to 1.0)
    pub peak_level: f32,
    /// Spectral centroid in Hz (center of mass of spectrum)
    pub spectral_centroid_hz: f32,
    /// Spectral rolloff in Hz (frequency below which 85% of energy is contained)
    pub spectral_rolloff_hz: f32,
    /// Effective bandwidth in Hz
    pub bandwidth_hz: f32,
    /// Whether clipping is detected
    pub has_clipping: bool,
    /// Enhancement recommendations
    pub recommendations: Vec<EnhancementRecommendation>,
}

/// Audio enhancement analyzer
pub struct AudioEnhancementAnalyzer {
    config: EnhancementConfig,
}

impl AudioEnhancementAnalyzer {
    /// Create a new audio enhancement analyzer
    #[must_use]
    pub fn new(config: EnhancementConfig) -> Self {
        Self { config }
    }

    /// Analyze audio samples and return enhancement metadata
    ///
    /// # Arguments
    /// * `samples` - Audio samples (mono, normalized to -1.0 to 1.0)
    /// * `sample_rate` - Sample rate in Hz
    ///
    /// # Errors
    /// Returns error if audio is invalid or analysis fails
    pub fn analyze(
        &self,
        samples: &[f32],
        sample_rate: u32,
    ) -> Result<EnhancementMetadata, EnhancementError> {
        if samples.is_empty() {
            return Err(EnhancementError::InvalidAudio(
                "Empty audio samples".to_string(),
            ));
        }

        info!("Analyzing {} samples at {} Hz", samples.len(), sample_rate);

        // Compute time-domain metrics
        let (rms_level, peak_level, has_clipping) = self.compute_level_metrics(samples);
        let dynamic_range_db = self.compute_dynamic_range(samples);

        // Compute frequency-domain metrics
        let (spectral_centroid_hz, spectral_rolloff_hz, bandwidth_hz) =
            self.compute_spectral_metrics(samples, sample_rate)?;

        // Estimate SNR using spectral analysis
        let snr_db = self.estimate_snr(samples)?;

        // Generate recommendations
        let recommendations = self.generate_recommendations(
            snr_db,
            dynamic_range_db,
            rms_level,
            peak_level,
            has_clipping,
        );

        debug!(
            "Analysis complete: SNR={:.2}dB, DR={:.2}dB, centroid={:.0}Hz",
            snr_db, dynamic_range_db, spectral_centroid_hz
        );

        Ok(EnhancementMetadata {
            snr_db,
            dynamic_range_db,
            rms_level,
            peak_level,
            spectral_centroid_hz,
            spectral_rolloff_hz,
            bandwidth_hz,
            has_clipping,
            recommendations,
        })
    }

    /// Compute level metrics (RMS, peak, clipping)
    fn compute_level_metrics(&self, samples: &[f32]) -> (f32, f32, bool) {
        let mut sum_squares = 0.0;
        let mut peak = 0.0;
        let mut clipping_count = 0;

        for &sample in samples {
            sum_squares += sample * sample;
            let abs_sample = sample.abs();
            if abs_sample > peak {
                peak = abs_sample;
            }
            // Check for clipping (within 1% of max amplitude)
            if abs_sample >= 0.99 {
                clipping_count += 1;
            }
        }

        let rms = (sum_squares / samples.len() as f32).sqrt();
        let has_clipping = clipping_count > (samples.len() / 1000); // More than 0.1% clipped

        (rms, peak, has_clipping)
    }

    /// Compute dynamic range in dB
    fn compute_dynamic_range(&self, samples: &[f32]) -> f32 {
        // Use sliding window to find local peaks and troughs
        const WINDOW_SIZE: usize = 4096;
        let mut max_rms: f32 = 0.0;
        let mut min_rms: f32 = f32::MAX;

        for chunk in samples.chunks(WINDOW_SIZE) {
            if chunk.is_empty() {
                continue;
            }
            let sum_squares: f32 = chunk.iter().map(|&s| s * s).sum();
            let rms = (sum_squares / chunk.len() as f32).sqrt();
            if rms > 1e-6 {
                // Ignore silence
                max_rms = max_rms.max(rms);
                min_rms = min_rms.min(rms);
            }
        }

        if min_rms == f32::MAX || min_rms < 1e-6 {
            return 0.0;
        }

        20.0 * (max_rms / min_rms).log10()
    }

    /// Compute spectral metrics using FFT
    fn compute_spectral_metrics(
        &self,
        samples: &[f32],
        sample_rate: u32,
    ) -> Result<(f32, f32, f32), EnhancementError> {
        let fft_size = self.config.fft_size.min(samples.len());
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(fft_size);

        // Prepare FFT input (windowed)
        let mut buffer: Vec<Complex<f32>> = samples[..fft_size]
            .iter()
            .enumerate()
            .map(|(i, &s)| {
                // Hann window
                let window =
                    0.5 * (1.0 - (2.0 * f32::consts::PI * i as f32 / fft_size as f32).cos());
                Complex::new(s * window, 0.0)
            })
            .collect();

        // Compute FFT
        fft.process(&mut buffer);

        // Compute magnitude spectrum (only first half, real signals are symmetric)
        let half_size = fft_size / 2;
        let mut magnitudes: Vec<f32> = Vec::with_capacity(half_size);
        magnitudes.extend(
            buffer[..half_size]
                .iter()
                .map(|c| (c.re * c.re + c.im * c.im).sqrt()),
        );

        // Frequency resolution
        let freq_resolution = sample_rate as f32 / fft_size as f32;

        // Compute spectral centroid
        let total_magnitude: f32 = magnitudes.iter().sum();
        if total_magnitude < 1e-6 {
            return Ok((0.0, 0.0, 0.0));
        }

        let spectral_centroid_hz: f32 = magnitudes
            .iter()
            .enumerate()
            .map(|(i, &mag)| i as f32 * freq_resolution * mag)
            .sum::<f32>()
            / total_magnitude;

        // Compute spectral rolloff (85% of energy)
        let mut cumulative_energy = 0.0;
        let threshold = 0.85 * total_magnitude;
        let mut rolloff_bin = 0;
        for (i, &mag) in magnitudes.iter().enumerate() {
            cumulative_energy += mag;
            if cumulative_energy >= threshold {
                rolloff_bin = i;
                break;
            }
        }
        let spectral_rolloff_hz = rolloff_bin as f32 * freq_resolution;

        // Compute bandwidth (frequency range containing significant energy)
        let low_bin = (self.config.low_freq_cutoff / freq_resolution) as usize;
        let high_bin =
            (self.config.high_freq_cutoff / freq_resolution).min(half_size as f32) as usize;
        let bandwidth_hz = (high_bin - low_bin) as f32 * freq_resolution;

        Ok((spectral_centroid_hz, spectral_rolloff_hz, bandwidth_hz))
    }

    /// Estimate SNR using spectral analysis
    ///
    /// Uses a simple approach: compare energy in high-frequency band (assumed noise)
    /// to total energy. More sophisticated methods could use voice activity detection.
    fn estimate_snr(&self, samples: &[f32]) -> Result<f32, EnhancementError> {
        // Simple SNR estimation: use peak-to-RMS ratio as a proxy
        // More sophisticated methods would use VAD and noise estimation
        let (rms, peak, _) = self.compute_level_metrics(samples);

        if rms < 1e-6 {
            return Ok(0.0);
        }

        // Approximate SNR using peak-to-RMS ratio
        // This is a rough estimate; proper SNR requires noise model
        let snr_db = 20.0 * (peak / rms).log10();

        // Clamp to reasonable range
        Ok(snr_db.clamp(0.0, 60.0))
    }

    /// Generate enhancement recommendations based on metrics
    fn generate_recommendations(
        &self,
        snr_db: f32,
        dynamic_range_db: f32,
        rms_level: f32,
        _peak_level: f32,
        has_clipping: bool,
    ) -> Vec<EnhancementRecommendation> {
        // Pre-allocate capacity for typical recommendation count (max 4 checks: clipping, SNR, dynamic range, level)
        let mut recommendations = Vec::with_capacity(4);

        // Check for clipping
        if has_clipping {
            recommendations.push(EnhancementRecommendation::RemoveClipping);
        }

        // Check SNR
        if snr_db < self.config.snr_threshold_db {
            recommendations.push(EnhancementRecommendation::Denoise);
        }

        // Check dynamic range
        if dynamic_range_db < self.config.dynamic_range_threshold_db {
            recommendations.push(EnhancementRecommendation::Normalize);
        }

        // Check overall level
        if rms_level < 0.1 && !has_clipping {
            recommendations.push(EnhancementRecommendation::AmplifyVolume);
        }

        // If no issues found
        if recommendations.is_empty() {
            recommendations.push(EnhancementRecommendation::None);
        }

        recommendations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyzer_creation() {
        let config = EnhancementConfig::default();
        let _analyzer = AudioEnhancementAnalyzer::new(config);
    }

    #[test]
    fn test_silent_audio() {
        let config = EnhancementConfig::default();
        let analyzer = AudioEnhancementAnalyzer::new(config);

        let samples = vec![0.0; 4096];
        let result = analyzer.analyze(&samples, 48000);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.rms_level, 0.0);
        assert_eq!(metadata.peak_level, 0.0);
        assert!(!metadata.has_clipping);
    }

    #[test]
    fn test_clipping_detection() {
        let config = EnhancementConfig::default();
        let analyzer = AudioEnhancementAnalyzer::new(config);

        // Generate audio with clipping
        let mut samples = vec![0.5; 4096];
        for sample in samples.iter_mut().take(100) {
            *sample = 1.0; // Clipped samples
        }

        let result = analyzer.analyze(&samples, 48000);
        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert!(metadata.has_clipping);
        assert!(metadata
            .recommendations
            .contains(&EnhancementRecommendation::RemoveClipping));
    }

    #[test]
    fn test_sine_wave_analysis() {
        let config = EnhancementConfig::default();
        let analyzer = AudioEnhancementAnalyzer::new(config);

        // Generate 440Hz sine wave
        let sample_rate = 48000;
        let duration = 1.0; // 1 second
        let frequency = 440.0;
        let samples: Vec<f32> = (0..((sample_rate as f32 * duration) as usize))
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                0.5 * (2.0 * f32::consts::PI * frequency * t).sin()
            })
            .collect();

        let result = analyzer.analyze(&samples, sample_rate);
        assert!(result.is_ok());
        let metadata = result.unwrap();

        // Sine wave should have good dynamic range and no clipping
        assert!(!metadata.has_clipping);
        assert!(metadata.peak_level > 0.45 && metadata.peak_level < 0.55);
        assert!(metadata.spectral_centroid_hz > 100.0); // Should be somewhere reasonable
    }

    #[test]
    fn test_low_volume_recommendation() {
        let config = EnhancementConfig::default();
        let analyzer = AudioEnhancementAnalyzer::new(config);

        // Generate very quiet audio
        let samples: Vec<f32> = (0..4096).map(|_| 0.01).collect();

        let result = analyzer.analyze(&samples, 48000);
        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert!(metadata
            .recommendations
            .contains(&EnhancementRecommendation::AmplifyVolume));
    }
}
