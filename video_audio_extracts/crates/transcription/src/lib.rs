//! Transcription module using Whisper.cpp
//!
//! Provides speech-to-text transcription with word-level timestamps,
//! language detection, and configurable quality modes.

pub mod plugin;
pub mod spellcheck;

use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;
use tracing::{debug, info, warn};
use video_audio_common::ProcessingError;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

/// Transcription errors
#[derive(Debug, Error)]
pub enum TranscriptionError {
    #[error("Failed to load model: {0}")]
    ModelLoadError(String),

    #[error("Failed to create transcription context: {0}")]
    ContextError(String),

    #[error("Failed to load audio: {0}")]
    AudioLoadError(String),

    #[error("Transcription failed: {0}")]
    TranscriptionFailed(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Quality too low: confidence {confidence:.2} < threshold {threshold:.2}")]
    QualityTooLow { confidence: f32, threshold: f32 },

    #[error("Processing error: {0}")]
    ProcessingError(#[from] ProcessingError),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, TranscriptionError>;

/// Whisper model size selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WhisperModel {
    /// 39M parameters, fastest
    Tiny,
    /// 74M parameters
    Base,
    /// 244M parameters
    Small,
    /// 769M parameters
    Medium,
    /// 1.5B parameters, most accurate
    LargeV3,
}

impl WhisperModel {
    /// Get the model filename
    #[must_use]
    pub fn filename(&self) -> &'static str {
        match self {
            Self::Tiny => "ggml-tiny.bin",
            Self::Base => "ggml-base.bin",
            Self::Small => "ggml-small.bin",
            Self::Medium => "ggml-medium.bin",
            Self::LargeV3 => "ggml-large-v3.bin",
        }
    }

    /// Get the model filename for English-only variant (faster)
    #[must_use]
    pub fn filename_en(&self) -> &'static str {
        match self {
            Self::Tiny => "ggml-tiny.en.bin",
            Self::Base => "ggml-base.en.bin",
            Self::Small => "ggml-small.en.bin",
            Self::Medium => "ggml-medium.en.bin",
            Self::LargeV3 => "ggml-large-v3.bin", // No English-only variant for v3
        }
    }
}

/// Compute type for inference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ComputeType {
    /// 32-bit floating point (highest quality, slowest)
    Float32,
    /// 16-bit floating point (good quality, faster)
    Float16,
    /// 8-bit integer quantization (lower quality, fastest)
    Int8,
}

/// Transcription configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionConfig {
    /// Model size to use
    pub model_size: WhisperModel,

    /// Language code (e.g., "en", "es") or None for auto-detection
    pub language: Option<String>,

    /// Enable word-level timestamps
    pub word_timestamps: bool,

    /// Beam search size (1-10, higher = more accurate but slower)
    pub beam_size: u8,

    /// Temperature for sampling (0.0-1.0)
    pub temperature: f32,

    /// Compute type for inference
    pub compute_type: ComputeType,

    /// Use English-only model (faster for English audio)
    pub english_only: bool,

    /// Number of threads for CPU inference
    pub num_threads: usize,

    /// Enable translation to English
    pub translate: bool,

    /// Minimum confidence threshold (0.0-1.0)
    pub min_confidence: Option<f32>,

    /// Enable post-processing spell correction for proper nouns
    pub enable_spell_correction: bool,

    /// Spell correction threshold (0.0-1.0, higher = stricter matching)
    pub spell_correction_threshold: f64,
}

impl Default for TranscriptionConfig {
    fn default() -> Self {
        Self {
            model_size: WhisperModel::LargeV3,
            language: None,
            word_timestamps: true,
            beam_size: 5,
            temperature: 0.0,
            compute_type: ComputeType::Float16,
            english_only: false,
            num_threads: num_cpus::get(),
            translate: false,
            min_confidence: None,
            enable_spell_correction: true,
            spell_correction_threshold: 0.85,
        }
    }
}

impl TranscriptionConfig {
    /// Create a fast preset optimized for speed
    #[must_use]
    pub fn fast() -> Self {
        Self {
            model_size: WhisperModel::Tiny,
            beam_size: 1,
            compute_type: ComputeType::Int8,
            num_threads: num_cpus::get() / 2,
            ..Default::default()
        }
    }

    /// Create a balanced preset for good quality and reasonable speed
    #[must_use]
    pub fn balanced() -> Self {
        Self {
            model_size: WhisperModel::Small,
            beam_size: 5,
            compute_type: ComputeType::Float16,
            ..Default::default()
        }
    }

    /// Create an accurate preset optimized for quality
    #[must_use]
    pub fn accurate() -> Self {
        Self {
            model_size: WhisperModel::Medium,
            beam_size: 10,
            compute_type: ComputeType::Float16,
            ..Default::default()
        }
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        if self.beam_size == 0 || self.beam_size > 10 {
            return Err(TranscriptionError::InvalidConfig(
                "beam_size must be between 1 and 10".to_string(),
            ));
        }

        if !(0.0..=1.0).contains(&self.temperature) {
            return Err(TranscriptionError::InvalidConfig(
                "temperature must be between 0.0 and 1.0".to_string(),
            ));
        }

        if let Some(threshold) = self.min_confidence {
            if !(0.0..=1.0).contains(&threshold) {
                return Err(TranscriptionError::InvalidConfig(
                    "min_confidence must be between 0.0 and 1.0".to_string(),
                ));
            }
        }

        Ok(())
    }
}

/// Word timing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordTiming {
    /// Word text
    pub word: String,
    /// Start time in seconds
    pub start: f64,
    /// End time in seconds
    pub end: f64,
    /// Confidence score (0.0-1.0)
    pub probability: f32,
}

/// Transcript segment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptSegment {
    /// Segment start time in seconds
    pub start: f64,
    /// Segment end time in seconds
    pub end: f64,
    /// Segment text
    pub text: String,
    /// Word-level timings (if enabled)
    pub words: Vec<WordTiming>,
    /// Probability that this segment contains no speech
    pub no_speech_prob: f32,
}

/// Complete transcript with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transcript {
    /// Full transcribed text
    pub text: String,
    /// Detected or specified language code
    pub language: String,
    /// Language detection confidence
    pub language_probability: f32,
    /// Transcript segments with timestamps
    pub segments: Vec<TranscriptSegment>,
    /// Overall quality score (0.0-1.0)
    pub quality_score: f32,
}

impl Transcript {
    /// Calculate average confidence across all words
    #[must_use]
    pub fn average_confidence(&self) -> f32 {
        let total_words: usize = self.segments.iter().map(|s| s.words.len()).sum();
        if total_words == 0 {
            return 0.0;
        }

        let total_prob: f32 = self
            .segments
            .iter()
            .flat_map(|s| &s.words)
            .map(|w| w.probability)
            .sum();

        total_prob / total_words as f32
    }

    /// Get total duration
    #[must_use]
    pub fn duration(&self) -> f64 {
        self.segments.last().map_or(0.0, |s| s.end)
    }
}

/// Transcription engine using Whisper.cpp
pub struct Transcriber {
    context: WhisperContext,
    config: TranscriptionConfig,
}

impl Transcriber {
    /// Create a new transcriber with the specified configuration
    ///
    /// # Arguments
    /// * `model_path` - Path to the Whisper model file
    /// * `config` - Transcription configuration
    pub fn new(model_path: impl AsRef<Path>, config: TranscriptionConfig) -> Result<Self> {
        config.validate()?;

        let model_path = model_path.as_ref();
        if !model_path.exists() {
            return Err(TranscriptionError::ModelLoadError(format!(
                "Model file not found: {}",
                model_path.display()
            )));
        }

        info!("Loading Whisper model from {}", model_path.display());

        let ctx_params = WhisperContextParameters::default();
        let context = WhisperContext::new_with_params(
            model_path.to_str().ok_or_else(|| {
                TranscriptionError::ModelLoadError("Invalid path encoding".to_string())
            })?,
            ctx_params,
        )
        .map_err(|e| TranscriptionError::ModelLoadError(e.to_string()))?;

        info!(
            "Whisper model loaded successfully (model_size={:?})",
            config.model_size
        );

        Ok(Self { context, config })
    }

    /// Transcribe audio from a file using a pre-loaded WhisperContext (for model caching)
    ///
    /// # Arguments
    /// * `context` - Pre-loaded WhisperContext (shared via Arc)
    /// * `audio_path` - Path to audio file (must be 16kHz mono WAV/PCM)
    /// * `config` - Transcription configuration
    pub fn transcribe_with_context(
        context: &WhisperContext,
        audio_path: impl AsRef<Path>,
        config: &TranscriptionConfig,
    ) -> Result<Transcript> {
        config.validate()?;

        let audio_path = audio_path.as_ref();

        debug!("Loading audio from {}", audio_path.display());

        // Load audio samples
        let audio_samples = Self::load_audio_samples_impl(audio_path)?;

        debug!(
            "Loaded {} audio samples ({:.2}s)",
            audio_samples.len(),
            audio_samples.len() as f64 / 16000.0
        );

        // Create transcription parameters
        let params = Self::create_params_impl(config)?;

        // Run transcription
        info!("Starting transcription...");
        let mut state = context
            .create_state()
            .map_err(|e| TranscriptionError::ContextError(e.to_string()))?;

        state
            .full(params, &audio_samples)
            .map_err(|e| TranscriptionError::TranscriptionFailed(e.to_string()))?;

        // Detect language and probability if language was auto-detected
        let (detected_language, language_probability) = if config.language.is_none() {
            let detected_lang_id = state.full_lang_id_from_state();
            let lang_str = whisper_rs::get_lang_str(detected_lang_id)
                .unwrap_or("en")
                .to_string();

            let probability = match state.lang_detect(0, config.num_threads) {
                Ok((_lang_id, lang_probs)) => lang_probs
                    .get(detected_lang_id as usize)
                    .copied()
                    .unwrap_or(1.0),
                Err(e) => {
                    debug!("Failed to detect language probability: {}", e);
                    1.0
                }
            };

            (lang_str, probability)
        } else {
            let lang = config.language.clone().unwrap_or_else(|| {
                warn!("Language config missing in else branch (should not happen), defaulting to 'en'");
                "en".to_string()
            });
            (lang, 1.0)
        };

        // Extract results
        let transcript =
            Self::extract_transcript_impl(&state, config, detected_language, language_probability)?;

        info!(
            "Transcription complete: {} segments, {:.2}s duration, quality={:.2}",
            transcript.segments.len(),
            transcript.duration(),
            transcript.quality_score
        );

        // Check quality threshold
        if let Some(threshold) = config.min_confidence {
            if transcript.quality_score < threshold {
                return Err(TranscriptionError::QualityTooLow {
                    confidence: transcript.quality_score,
                    threshold,
                });
            }
        }

        Ok(transcript)
    }

    /// Transcribe audio from a file
    ///
    /// # Arguments
    /// * `audio_path` - Path to audio file (must be 16kHz mono WAV/PCM)
    ///
    /// # Returns
    /// Complete transcript with segments and word-level timings
    pub fn transcribe(&self, audio_path: impl AsRef<Path>) -> Result<Transcript> {
        let audio_path = audio_path.as_ref();

        debug!("Loading audio from {}", audio_path.display());

        // Load audio samples
        // Note: whisper-rs expects 16kHz mono f32 samples
        let audio_samples = Self::load_audio_samples_impl(audio_path)?;

        debug!(
            "Loaded {} audio samples ({:.2}s)",
            audio_samples.len(),
            audio_samples.len() as f64 / 16000.0
        );

        // Create transcription parameters
        let params = Self::create_params_impl(&self.config)?;

        // Run transcription
        info!("Starting transcription...");
        let mut state = self
            .context
            .create_state()
            .map_err(|e| TranscriptionError::ContextError(e.to_string()))?;

        state
            .full(params, &audio_samples)
            .map_err(|e| TranscriptionError::TranscriptionFailed(e.to_string()))?;

        // Detect language and probability if language was auto-detected
        let (detected_language, language_probability) = if self.config.language.is_none() {
            // Get detected language ID
            let detected_lang_id = state.full_lang_id_from_state();

            // Convert language ID to language string
            let lang_str = whisper_rs::get_lang_str(detected_lang_id)
                .unwrap_or("en")
                .to_string();

            // Try to get language probabilities
            // Note: lang_detect can be called after full() since mel data is still in state
            let probability = match state.lang_detect(0, self.config.num_threads) {
                Ok((_lang_id, lang_probs)) => {
                    // Extract probability for the detected language
                    lang_probs
                        .get(detected_lang_id as usize)
                        .copied()
                        .unwrap_or(1.0)
                }
                Err(e) => {
                    // If language detection fails, use 1.0 as fallback
                    debug!("Failed to detect language probability: {}", e);
                    1.0
                }
            };

            (lang_str, probability)
        } else {
            // Language was explicitly specified by user, so probability is 1.0 (certain)
            // Note: Logic guarantees config.language is Some here, but use defensive handling
            let lang = self.config.language.clone().unwrap_or_else(|| {
                warn!("Language config missing in else branch (should not happen), defaulting to 'en'");
                "en".to_string()
            });
            (lang, 1.0)
        };

        // Extract results
        let transcript =
            self.extract_transcript(&state, detected_language, language_probability)?;

        info!(
            "Transcription complete: {} segments, {:.2}s duration, quality={:.2}",
            transcript.segments.len(),
            transcript.duration(),
            transcript.quality_score
        );

        // Check quality threshold
        if let Some(threshold) = self.config.min_confidence {
            if transcript.quality_score < threshold {
                return Err(TranscriptionError::QualityTooLow {
                    confidence: transcript.quality_score,
                    threshold,
                });
            }
        }

        Ok(transcript)
    }

    /// Load audio samples from file
    ///
    /// Converts any audio/video file to 16kHz mono f32 samples required by Whisper.
    /// Uses audio-extractor to handle format conversion via `FFmpeg`, then reads WAV samples.
    /// Internal static helper for loading audio samples
    fn load_audio_samples_impl(audio_path: &Path) -> Result<Vec<f32>> {
        use video_audio_extractor::{extract_audio, AudioConfig, AudioFormat};

        // Create temp directory for intermediate WAV file
        let temp_dir = std::env::temp_dir();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or(std::time::Duration::from_secs(0))
            .as_millis();
        let temp_wav = temp_dir.join(format!("whisper_audio_{timestamp}.wav"));

        // Extract audio to 16kHz mono PCM WAV
        let config = AudioConfig {
            sample_rate: 16000,
            channels: 1,
            format: AudioFormat::PCM,
            normalize: false, // Whisper handles normalization internally
        };

        debug!("Extracting audio to temp WAV: {}", temp_wav.display());

        let wav_path = extract_audio(audio_path, &temp_wav, &config).map_err(|e| {
            TranscriptionError::AudioLoadError(format!("Failed to extract audio: {e}"))
        })?;

        // Read WAV samples using hound
        let mut reader = hound::WavReader::open(&wav_path).map_err(|e| {
            TranscriptionError::AudioLoadError(format!("Failed to open WAV file: {e}"))
        })?;

        let spec = reader.spec();

        // Verify format matches expectations
        if spec.sample_rate != 16000 {
            return Err(TranscriptionError::AudioLoadError(format!(
                "Expected 16kHz sample rate, got {}Hz",
                spec.sample_rate
            )));
        }
        if spec.channels != 1 {
            return Err(TranscriptionError::AudioLoadError(format!(
                "Expected mono audio, got {} channels",
                spec.channels
            )));
        }

        // Read samples and convert to f32
        // Pre-allocate Vec with known duration (from WAV spec)
        let num_samples = reader.len() as usize;
        let samples: Result<Vec<f32>> = match spec.sample_format {
            hound::SampleFormat::Int => {
                // Convert integer samples to f32 in [-1.0, 1.0]
                let bits = spec.bits_per_sample;
                let max_val = (1 << (bits - 1)) as f32;

                let mut samples = Vec::with_capacity(num_samples);
                for sample in reader.samples::<i32>() {
                    let sample = sample.map_err(|e| {
                        TranscriptionError::AudioLoadError(format!("Failed to read sample: {e}"))
                    })?;
                    samples.push(sample as f32 / max_val);
                }
                Ok(samples)
            }
            hound::SampleFormat::Float => {
                // Already in f32 format
                let mut samples = Vec::with_capacity(num_samples);
                for sample in reader.samples::<f32>() {
                    let sample = sample.map_err(|e| {
                        TranscriptionError::AudioLoadError(format!("Failed to read sample: {e}"))
                    })?;
                    samples.push(sample);
                }
                Ok(samples)
            }
        };

        let samples = samples?;

        // Clean up temp file
        if let Err(e) = std::fs::remove_file(&wav_path) {
            warn!("Failed to remove temp WAV file: {}", e);
        }

        debug!(
            "Loaded {} samples ({:.2}s) from {}",
            samples.len(),
            samples.len() as f64 / 16000.0,
            audio_path.display()
        );

        Ok(samples)
    }

    /// Create Whisper parameters from config
    #[allow(dead_code)]
    fn create_params(&self) -> Result<FullParams<'_, '_>> {
        Self::create_params_impl(&self.config)
    }

    /// Internal static helper for creating transcription parameters
    fn create_params_impl(config: &TranscriptionConfig) -> Result<FullParams<'_, '_>> {
        let strategy = if config.beam_size > 1 {
            SamplingStrategy::BeamSearch {
                beam_size: i32::from(config.beam_size),
                patience: 1.0,
            }
        } else {
            SamplingStrategy::Greedy { best_of: 1 }
        };

        let mut params = FullParams::new(strategy);

        // Set number of threads
        params.set_n_threads(config.num_threads as i32);

        // Set language
        if let Some(ref lang) = config.language {
            params.set_language(Some(lang.as_str()));
        }

        // Enable/disable translation
        params.set_translate(config.translate);

        // Enable/disable timestamps
        params.set_token_timestamps(config.word_timestamps);

        // Set temperature
        params.set_temperature(config.temperature);

        // Print progress to stderr
        params.set_print_progress(false);
        params.set_print_realtime(false);

        Ok(params)
    }

    /// Extract transcript from Whisper state
    fn extract_transcript(
        &self,
        state: &whisper_rs::WhisperState,
        detected_language: String,
        language_probability: f32,
    ) -> Result<Transcript> {
        Self::extract_transcript_impl(state, &self.config, detected_language, language_probability)
    }

    /// Internal static helper for extracting transcript from Whisper state
    fn extract_transcript_impl(
        state: &whisper_rs::WhisperState,
        config: &TranscriptionConfig,
        detected_language: String,
        language_probability: f32,
    ) -> Result<Transcript> {
        let num_segments = state.full_n_segments();

        let mut segments = Vec::with_capacity(num_segments as usize);
        // Pre-allocate with estimated capacity (average ~50 chars per segment)
        let mut full_text = String::with_capacity((num_segments as usize) * 50);

        // Iterate through segments using the iterator API
        for segment in state.as_iter() {
            let start = segment.start_timestamp() as f64 / 1000.0; // Convert ms to seconds
            let end = segment.end_timestamp() as f64 / 1000.0;
            let segment_text = segment.to_string();

            // Extract word-level timings if enabled
            let words = if config.word_timestamps {
                Self::extract_word_timings_impl(&segment)?
            } else {
                Vec::new()
            };

            // Extract no-speech probability from Whisper segment
            let no_speech_prob = segment.no_speech_probability();

            segments.push(TranscriptSegment {
                start,
                end,
                text: segment_text.trim().to_string(),
                words,
                no_speech_prob,
            });

            full_text.push_str(&segment_text);
            full_text.push(' ');
        }

        let mut transcript = Transcript {
            text: full_text.trim().to_string(),
            language: detected_language,
            language_probability,
            segments,
            quality_score: 0.0, // Will be calculated below
        };

        // Apply spell correction if enabled
        if config.enable_spell_correction {
            let dict = spellcheck::ProperNounDictionary::new();
            transcript.text = dict.correct_text(&transcript.text, config.spell_correction_threshold);

            // Also correct segment text
            for segment in &mut transcript.segments {
                segment.text = dict.correct_text(&segment.text, config.spell_correction_threshold);
            }
        }

        // Calculate quality score
        let quality_score = Self::calculate_quality_score_impl(&transcript);

        Ok(Transcript {
            quality_score,
            ..transcript
        })
    }

    /// Extract word-level timings from a segment
    #[allow(dead_code)]
    fn extract_word_timings(
        &self,
        segment: &whisper_rs::WhisperSegment,
    ) -> Result<Vec<WordTiming>> {
        Self::extract_word_timings_impl(segment)
    }

    /// Internal static helper for extracting word-level timings
    fn extract_word_timings_impl(segment: &whisper_rs::WhisperSegment) -> Result<Vec<WordTiming>> {
        let num_tokens = segment.n_tokens();
        let mut words = Vec::with_capacity(num_tokens as usize);

        for token_idx in 0..num_tokens {
            let token = segment.get_token(token_idx).ok_or_else(|| {
                TranscriptionError::TranscriptionFailed(format!(
                    "Failed to get token at index {token_idx}"
                ))
            })?;

            let token_data = token.token_data();
            let token_text = token.to_string();

            // Skip special tokens
            let trimmed = token_text.trim();
            if trimmed.is_empty()
                || trimmed.starts_with("[_")
                || trimmed.starts_with("<|") {
                continue;
            }

            // Convert from centiseconds to seconds
            let start = token_data.t0 as f64 / 100.0;
            let end = token_data.t1 as f64 / 100.0;

            words.push(WordTiming {
                word: token_text.trim().to_string(),
                start,
                end,
                probability: token_data.p,
            });
        }

        Ok(words)
    }

    /// Calculate overall quality score for transcript
    #[allow(dead_code)]
    fn calculate_quality_score(&self, transcript: &Transcript) -> f32 {
        Self::calculate_quality_score_impl(transcript)
    }

    /// Internal static helper for calculating quality score
    fn calculate_quality_score_impl(transcript: &Transcript) -> f32 {
        let mut quality = 0.0;
        let mut total_duration = 0.0;

        for segment in &transcript.segments {
            let duration = segment.end - segment.start;
            if duration <= 0.0 {
                continue;
            }

            // Average word confidence
            let confidence = if segment.words.is_empty() {
                0.5 // Default if no word-level data
            } else {
                segment.words.iter().map(|w| w.probability).sum::<f32>()
                    / segment.words.len() as f32
            };

            // No-speech probability (lower is better)
            let speech_quality = 1.0 - segment.no_speech_prob;

            // Word density (2-4 words/sec is typical for speech)
            let word_density = segment.words.len() as f32 / duration as f32;
            let density_quality = if (2.0..=4.0).contains(&word_density) {
                1.0
            } else if word_density < 2.0 && word_density > 0.0 {
                word_density / 2.0 // Penalize sparse speech
            } else if word_density > 4.0 {
                4.0 / word_density // Penalize dense speech
            } else {
                0.5 // No words detected
            };

            let segment_quality = (confidence + speech_quality + density_quality) / 3.0;
            quality += segment_quality * duration as f32;
            total_duration += duration;
        }

        if total_duration > 0.0 {
            quality / total_duration as f32
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_whisper_model_filenames() {
        assert_eq!(WhisperModel::Tiny.filename(), "ggml-tiny.bin");
        assert_eq!(WhisperModel::Base.filename(), "ggml-base.bin");
        assert_eq!(WhisperModel::Small.filename(), "ggml-small.bin");
        assert_eq!(WhisperModel::Medium.filename(), "ggml-medium.bin");
        assert_eq!(WhisperModel::LargeV3.filename(), "ggml-large-v3.bin");
    }

    #[test]
    fn test_config_presets() {
        let fast = TranscriptionConfig::fast();
        assert_eq!(fast.model_size, WhisperModel::Tiny);
        assert_eq!(fast.beam_size, 1);
        assert_eq!(fast.compute_type, ComputeType::Int8);

        let balanced = TranscriptionConfig::balanced();
        assert_eq!(balanced.model_size, WhisperModel::Small);
        assert_eq!(balanced.beam_size, 5);

        let accurate = TranscriptionConfig::accurate();
        assert_eq!(accurate.model_size, WhisperModel::Medium);
        assert_eq!(accurate.beam_size, 10);
    }

    #[test]
    fn test_config_validation() {
        let mut config = TranscriptionConfig::default();
        assert!(config.validate().is_ok());

        config.beam_size = 0;
        assert!(config.validate().is_err());

        config.beam_size = 11;
        assert!(config.validate().is_err());

        config.beam_size = 5;
        config.temperature = -0.1;
        assert!(config.validate().is_err());

        config.temperature = 1.1;
        assert!(config.validate().is_err());

        config.temperature = 0.5;
        config.min_confidence = Some(1.5);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_transcript_average_confidence() {
        let transcript = Transcript {
            text: "Hello world".to_string(),
            language: "en".to_string(),
            language_probability: 1.0,
            segments: vec![TranscriptSegment {
                start: 0.0,
                end: 1.0,
                text: "Hello world".to_string(),
                words: vec![
                    WordTiming {
                        word: "Hello".to_string(),
                        start: 0.0,
                        end: 0.5,
                        probability: 0.9,
                    },
                    WordTiming {
                        word: "world".to_string(),
                        start: 0.5,
                        end: 1.0,
                        probability: 0.8,
                    },
                ],
                no_speech_prob: 0.1,
            }],
            quality_score: 0.85,
        };

        let avg_conf = transcript.average_confidence();
        assert!((avg_conf - 0.85).abs() < 0.01);
    }

    #[test]
    fn test_transcript_duration() {
        let transcript = Transcript {
            text: "Test".to_string(),
            language: "en".to_string(),
            language_probability: 1.0,
            segments: vec![
                TranscriptSegment {
                    start: 0.0,
                    end: 1.0,
                    text: "First".to_string(),
                    words: vec![],
                    no_speech_prob: 0.0,
                },
                TranscriptSegment {
                    start: 1.0,
                    end: 3.5,
                    text: "Second".to_string(),
                    words: vec![],
                    no_speech_prob: 0.0,
                },
            ],
            quality_score: 0.9,
        };

        assert_eq!(transcript.duration(), 3.5);
    }

    #[test]
    #[ignore] // Requires Whisper model to be downloaded
    fn test_audio_sample_loading() {
        use std::path::PathBuf;

        tracing_subscriber::fmt::init();

        // Test with WAV file from docling test suite
        let audio_path =
            PathBuf::from("/Users/ayates/docling/tests/data/audio/sample_10s_audio-wav.wav");

        if !audio_path.exists() {
            eprintln!(
                "Skipping test - audio file not found: {}",
                audio_path.display()
            );
            return;
        }

        // Download model if needed (skip test if model not available)
        let model_path = PathBuf::from("models/ggml-base.en.bin");
        if !model_path.exists() {
            eprintln!("Skipping test - model not found: {}", model_path.display());
            return;
        }

        let config = TranscriptionConfig::default();

        let transcriber =
            Transcriber::new(&model_path, config).expect("Failed to create transcriber");

        // This will test the full pipeline: audio loading + transcription
        let result = transcriber.transcribe(&audio_path);

        match result {
            Ok(transcript) => {
                println!("Transcription successful!");
                println!("Text: {}", transcript.text);
                println!("Language: {}", transcript.language);
                println!("Duration: {:.2}s", transcript.duration());
                println!("Segments: {}", transcript.segments.len());
                println!("Quality: {:.2}", transcript.quality_score);

                // Basic validation
                assert!(
                    !transcript.text.is_empty(),
                    "Transcript should not be empty"
                );
                assert!(transcript.duration() > 0.0, "Duration should be positive");
                assert!(
                    !transcript.segments.is_empty(),
                    "Should have at least one segment"
                );
            }
            Err(e) => {
                panic!("Transcription failed: {e}");
            }
        }
    }

    #[test]
    fn test_audio_sample_loading_formats() {
        use std::path::PathBuf;

        // Test that we can handle different audio formats by converting them to 16kHz mono WAV
        let test_files = vec![
            "/Users/ayates/docling/tests/data/audio/sample_10s_audio-wav.wav",
            "/Users/ayates/docling/tests/data/audio/sample_10s_audio-mp3.mp3",
        ];

        // We'll just test the audio extraction part without needing the Whisper model
        for file_path in test_files {
            let path = PathBuf::from(file_path);
            if !path.exists() {
                eprintln!("Skipping {file_path} - file not found");
                continue;
            }

            // Test audio extraction to WAV format
            use video_audio_extractor::{extract_audio, AudioConfig, AudioFormat};
            let temp_dir = std::env::temp_dir();
            let temp_wav = temp_dir.join("test_audio_extract.wav");

            let config = AudioConfig {
                sample_rate: 16000,
                channels: 1,
                format: AudioFormat::PCM,
                normalize: false,
            };

            let result = extract_audio(&path, &temp_wav, &config);
            assert!(
                result.is_ok(),
                "Failed to extract audio from {}: {:?}",
                file_path,
                result.err()
            );

            let wav_path = result.unwrap();
            assert!(wav_path.exists(), "WAV file should exist");

            // Verify we can read the WAV file
            let reader = hound::WavReader::open(&wav_path);
            assert!(
                reader.is_ok(),
                "Failed to open WAV file: {:?}",
                reader.err()
            );

            let mut reader = reader.unwrap();
            let spec = reader.spec();

            assert_eq!(spec.sample_rate, 16000, "Sample rate should be 16kHz");
            assert_eq!(spec.channels, 1, "Should be mono");

            // Read a few samples to verify
            let samples: Vec<f32> = match spec.sample_format {
                hound::SampleFormat::Int => {
                    let bits = spec.bits_per_sample;
                    let max_val = (1 << (bits - 1)) as f32;
                    reader
                        .samples::<i32>()
                        .take(100)
                        .map(|s| s.unwrap() as f32 / max_val)
                        .collect()
                }
                hound::SampleFormat::Float => reader
                    .samples::<f32>()
                    .take(100)
                    .map(|s| s.unwrap())
                    .collect(),
            };

            assert!(!samples.is_empty(), "Should have read some samples");
            println!(
                "Successfully loaded {} samples from {}",
                samples.len(),
                file_path
            );

            // Clean up
            let _ = std::fs::remove_file(&wav_path);
        }
    }
}
