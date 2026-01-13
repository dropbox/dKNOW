//! Voice Activity Detection (VAD) using WebRTC VAD
//!
//! Detects speech segments in audio streams using the WebRTC Voice Activity Detector.

pub mod plugin;

use anyhow::Result;
use log::debug;
use serde::{Deserialize, Serialize};
use webrtc_vad::{SampleRate, Vad, VadMode};

/// Configuration for Voice Activity Detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VadConfig {
    pub vad_aggressiveness: u8,
    pub min_segment_duration: f32,
    pub frame_duration_ms: usize,
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            vad_aggressiveness: 2,
            min_segment_duration: 0.3,
            frame_duration_ms: 30,
        }
    }
}

/// A detected speech segment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceSegment {
    pub start: f64,
    pub end: f64,
    pub duration: f64,
    pub confidence: f32,
}

/// VAD result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VadResult {
    pub segments: Vec<VoiceSegment>,
    pub total_voice_duration: f64,
    pub voice_percentage: f32,
    pub total_duration: f64,
}

/// Voice Activity Detector
pub struct VoiceActivityDetector {
    config: VadConfig,
}

impl Default for VoiceActivityDetector {
    fn default() -> Self {
        Self::new(VadConfig::default())
    }
}

impl VoiceActivityDetector {
    pub fn new(config: VadConfig) -> Self {
        Self { config }
    }

    pub fn detect(&self, samples: &[f32], sample_rate: u32) -> Result<VadResult> {
        let segments = self.detect_voice_segments(samples, sample_rate)?;
        let total_voice_duration: f64 = segments.iter().map(|s| s.duration).sum();
        let total_duration = samples.len() as f64 / sample_rate as f64;
        let voice_percentage = if total_duration > 0.0 {
            (total_voice_duration / total_duration) as f32
        } else {
            0.0
        };

        debug!(
            "VAD detected {} segments, {:.1}% voice",
            segments.len(),
            voice_percentage * 100.0
        );

        Ok(VadResult {
            segments,
            total_voice_duration,
            voice_percentage,
            total_duration,
        })
    }

    fn detect_voice_segments(
        &self,
        samples: &[f32],
        sample_rate: u32,
    ) -> Result<Vec<VoiceSegment>> {
        let vad_sample_rate = match sample_rate {
            8000 => SampleRate::Rate8kHz,
            16000 => SampleRate::Rate16kHz,
            32000 => SampleRate::Rate32kHz,
            48000 => SampleRate::Rate48kHz,
            _ => anyhow::bail!("Unsupported sample rate: {sample_rate}Hz"),
        };

        let vad_mode = match self.config.vad_aggressiveness {
            0 => VadMode::Quality,
            1 => VadMode::LowBitrate,
            2 => VadMode::Aggressive,
            _ => VadMode::VeryAggressive,
        };

        let mut vad = Vad::new_with_rate_and_mode(vad_sample_rate, vad_mode);
        let frame_size = (sample_rate as usize * self.config.frame_duration_ms) / 1000;
        // Pre-allocate: estimate ~10% of frames will be speech segments
        let estimated_segments = (samples.len() / frame_size) / 10;
        let mut voice_segments = Vec::with_capacity(estimated_segments.max(4));
        let mut current_start: Option<usize> = None;
        let mut samples_i16 = Vec::with_capacity(samples.len());
        samples_i16.extend(samples.iter().map(|&s| (s * 32767.0) as i16));

        for (idx, frame) in samples_i16.chunks(frame_size).enumerate() {
            if frame.len() != frame_size {
                break;
            }

            let frame_start = idx * frame_size;
            let is_voice = vad.is_voice_segment(frame).unwrap_or(false);

            if is_voice {
                if current_start.is_none() {
                    current_start = Some(frame_start);
                }
            } else if let Some(start) = current_start {
                let end = frame_start;
                let duration = (end - start) as f64 / sample_rate as f64;

                if duration >= self.config.min_segment_duration as f64 {
                    voice_segments.push(VoiceSegment {
                        start: start as f64 / sample_rate as f64,
                        end: end as f64 / sample_rate as f64,
                        duration,
                        confidence: 1.0,
                    });
                }
                current_start = None;
            }
        }

        if let Some(start) = current_start {
            let end = samples.len();
            let duration = (end - start) as f64 / sample_rate as f64;
            if duration >= self.config.min_segment_duration as f64 {
                voice_segments.push(VoiceSegment {
                    start: start as f64 / sample_rate as f64,
                    end: end as f64 / sample_rate as f64,
                    duration,
                    confidence: 1.0,
                });
            }
        }

        Ok(voice_segments)
    }
}
