//! Audio transcription using Whisper
//!
//! This module provides audio transcription capabilities using OpenAI's Whisper model
//! via candle-transformers. It supports common audio formats (MP3, WAV, FLAC, OGG, etc.)
//! and can extract audio from video files for transcription.
//!
//! # Usage
//!
//! ```ignore
//! use sg_core::whisper::Transcriber;
//!
//! let transcriber = Transcriber::new()?;
//! let text = transcriber.transcribe_file("audio.mp3")?;
//! println!("{}", text);
//! ```
//!
//! # Requirements
//!
//! - Enable the `audio-transcription` feature in Cargo.toml
//! - Whisper model weights will be downloaded from HuggingFace on first use

use anyhow::{bail, Context, Result};
use candle_core::{Device, IndexOp, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::whisper::{self as m, audio, Config};
use hf_hub::{api::sync::Api, Repo, RepoType};
use rubato::{FftFixedIn, Resampler};
use std::path::Path;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use tokenizers::Tokenizer;
use tracing::{debug, info};

/// Whisper model size variants
#[derive(Debug, Clone, Copy, Default)]
pub enum WhisperModel {
    /// Tiny model (~39M parameters) - fastest, lowest quality
    Tiny,
    /// Base model (~74M parameters) - good balance
    #[default]
    Base,
    /// Small model (~244M parameters) - better quality
    Small,
    /// Medium model (~769M parameters) - high quality
    Medium,
    /// Large model (~1.5B parameters) - best quality, slowest
    LargeV3,
}

impl WhisperModel {
    fn repo_id(&self) -> &'static str {
        match self {
            WhisperModel::Tiny => "openai/whisper-tiny",
            WhisperModel::Base => "openai/whisper-base",
            WhisperModel::Small => "openai/whisper-small",
            WhisperModel::Medium => "openai/whisper-medium",
            WhisperModel::LargeV3 => "openai/whisper-large-v3",
        }
    }
}

/// Whisper-based audio transcriber
pub struct Transcriber {
    model: m::model::Whisper,
    tokenizer: Tokenizer,
    mel_filters: Vec<f32>,
    device: Device,
    config: Config,
}

impl Transcriber {
    /// Create a new transcriber with the default (Base) model
    pub fn new() -> Result<Self> {
        Self::with_model(WhisperModel::default())
    }

    /// Create a transcriber with a specific model size
    pub fn with_model(model_size: WhisperModel) -> Result<Self> {
        let device = Device::Cpu;

        info!("Loading Whisper {:?} model...", model_size);

        // Download model from HuggingFace
        let api = Api::new()?;
        let repo = api.repo(Repo::new(model_size.repo_id().to_string(), RepoType::Model));

        let config_path = repo.get("config.json")?;
        let tokenizer_path = repo.get("tokenizer.json")?;
        let model_path = repo.get("model.safetensors")?;
        let mel_path = repo.get("mel_filters.npz")?;

        // Load config
        let config: Config = serde_json::from_reader(std::fs::File::open(&config_path)?)?;

        // Load tokenizer
        let tokenizer =
            Tokenizer::from_file(&tokenizer_path).map_err(|e| anyhow::anyhow!("{e}"))?;

        // Load mel filters
        let mel_filters = Self::load_mel_filters(&mel_path, config.num_mel_bins)?;

        // Load model
        let vb = unsafe { VarBuilder::from_mmaped_safetensors(&[model_path], m::DTYPE, &device)? };
        let model = m::model::Whisper::load(&vb, config.clone())?;

        info!("Whisper model loaded successfully");

        Ok(Self {
            model,
            tokenizer,
            mel_filters,
            device,
            config,
        })
    }

    fn load_mel_filters(path: &Path, num_mel_bins: usize) -> Result<Vec<f32>> {
        // The mel filters are stored as numpy .npz file
        // We need to extract the "mel_80" array (or "mel_128" for large models)
        let file = std::fs::File::open(path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        let mel_key = if num_mel_bins == 128 {
            "mel_128.npy"
        } else {
            "mel_80.npy"
        };

        let mut npy_file = archive.by_name(mel_key)?;
        let mut data = Vec::new();
        std::io::Read::read_to_end(&mut npy_file, &mut data)?;

        // Parse numpy array (skip 128-byte header, then f32 values)
        // Simple parsing - real implementation should use ndarray-npy
        let header_len = data[8] as usize + data[9] as usize * 256;
        let offset = 10 + header_len;
        let floats: Vec<f32> = data[offset..]
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .collect();

        Ok(floats)
    }

    /// Transcribe an audio file to text
    pub fn transcribe_file(&mut self, path: impl AsRef<Path>) -> Result<String> {
        let path = path.as_ref();
        debug!("Transcribing audio file: {}", path.display());

        // Load and decode audio
        let samples = self.load_audio(path)?;

        // Transcribe
        self.transcribe_samples(&samples)
    }

    /// Load audio file and return mono 16kHz samples
    fn load_audio(&self, path: &Path) -> Result<Vec<f32>> {
        let file = std::fs::File::open(path)?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        // Create hint from file extension
        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        // Probe the file format
        let probed = symphonia::default::get_probe()
            .format(
                &hint,
                mss,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )
            .context("Failed to probe audio file")?;

        let mut format = probed.format;

        // Find the first audio track
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
            .context("No audio track found")?;

        let codec_params = track.codec_params.clone();
        let track_id = track.id;

        // Create decoder
        let mut decoder = symphonia::default::get_codecs()
            .make(&codec_params, &DecoderOptions::default())
            .context("Failed to create audio decoder")?;

        let sample_rate = codec_params.sample_rate.context("Unknown sample rate")? as usize;
        let channels = codec_params.channels.map(|c| c.count()).unwrap_or(1);

        let mut all_samples: Vec<f32> = Vec::new();

        // Decode all packets
        loop {
            let packet = match format.next_packet() {
                Ok(p) => p,
                Err(symphonia::core::errors::Error::IoError(e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    break;
                }
                Err(e) => bail!("Error reading packet: {e}"),
            };

            if packet.track_id() != track_id {
                continue;
            }

            let decoded = decoder.decode(&packet)?;
            let spec = *decoded.spec();
            let duration = decoded.capacity() as u64;

            let mut sample_buf = SampleBuffer::<f32>::new(duration, spec);
            sample_buf.copy_interleaved_ref(decoded);

            all_samples.extend(sample_buf.samples());
        }

        // Convert to mono if stereo
        let mono_samples = if channels > 1 {
            all_samples
                .chunks(channels)
                .map(|chunk| chunk.iter().sum::<f32>() / channels as f32)
                .collect()
        } else {
            all_samples
        };

        // Resample to 16kHz if needed
        let target_rate = 16000;
        if sample_rate != target_rate {
            debug!(
                "Resampling from {}Hz to {}Hz ({} samples)",
                sample_rate,
                target_rate,
                mono_samples.len()
            );
            self.resample(&mono_samples, sample_rate, target_rate)
        } else {
            Ok(mono_samples)
        }
    }

    /// Resample audio to target sample rate
    #[allow(clippy::unused_self)]
    fn resample(&self, samples: &[f32], from_rate: usize, to_rate: usize) -> Result<Vec<f32>> {
        let mut resampler = FftFixedIn::<f32>::new(from_rate, to_rate, 1024, 2, 1)?;

        let mut output = Vec::new();
        let mut input = samples.to_vec();

        // Process in chunks
        let chunk_size = resampler.input_frames_next();
        while input.len() >= chunk_size {
            let (chunk, rest) = input.split_at(chunk_size);
            let frames_in = vec![chunk.to_vec()];
            let frames_out = resampler.process(&frames_in, None)?;
            output.extend(&frames_out[0]);
            input = rest.to_vec();
        }

        // Process remaining samples with padding
        if !input.is_empty() {
            input.resize(chunk_size, 0.0);
            let frames_in = vec![input];
            let frames_out = resampler.process(&frames_in, None)?;
            output.extend(&frames_out[0]);
        }

        Ok(output)
    }

    /// Transcribe audio samples (mono, 16kHz)
    fn transcribe_samples(&mut self, samples: &[f32]) -> Result<String> {
        // Compute mel spectrogram
        let mel = audio::pcm_to_mel(&self.config, samples, &self.mel_filters);
        let mel_len = mel.len();

        let mel = Tensor::from_vec(
            mel,
            (
                1,
                self.config.num_mel_bins,
                mel_len / self.config.num_mel_bins,
            ),
            &self.device,
        )?;

        // Get language token (English)
        let language_token = self
            .tokenizer
            .token_to_id("<|en|>")
            .context("Language token not found")?;

        // Get special tokens
        let sot_token = self
            .tokenizer
            .token_to_id("<|startoftranscript|>")
            .context("SOT token not found")?;
        let transcribe_token = self
            .tokenizer
            .token_to_id("<|transcribe|>")
            .context("Transcribe token not found")?;
        let eot_token = self
            .tokenizer
            .token_to_id("<|endoftext|>")
            .context("EOT token not found")?;
        let notimestamps_token = self
            .tokenizer
            .token_to_id("<|notimestamps|>")
            .context("No timestamps token not found")?;

        // Initial tokens
        let mut tokens = vec![
            sot_token,
            language_token,
            transcribe_token,
            notimestamps_token,
        ];

        // Encode audio
        let audio_features = self.model.encoder.forward(&mel, true)?;

        // Decode tokens
        let max_tokens = 448; // Whisper's max tokens per segment
        let mut all_tokens = Vec::new();

        for _ in 0..max_tokens {
            let token_tensor = Tensor::new(&tokens[..], &self.device)?.unsqueeze(0)?;
            let logits = self
                .model
                .decoder
                .forward(&token_tensor, &audio_features, true)?;

            // Get last token's logits
            let seq_len = logits.dim(1)?;
            let logits = logits.i((0, seq_len - 1))?;

            // Greedy decoding
            let next_token = logits.argmax(0)?.to_scalar::<u32>()?;

            if next_token == eot_token {
                break;
            }

            tokens.push(next_token);
            all_tokens.push(next_token);
        }

        // Decode tokens to text
        let text = self
            .tokenizer
            .decode(&all_tokens, true)
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        Ok(text.trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_whisper_model_repos() {
        assert_eq!(WhisperModel::Tiny.repo_id(), "openai/whisper-tiny");
        assert_eq!(WhisperModel::Base.repo_id(), "openai/whisper-base");
        assert_eq!(WhisperModel::Small.repo_id(), "openai/whisper-small");
        assert_eq!(WhisperModel::Medium.repo_id(), "openai/whisper-medium");
        assert_eq!(WhisperModel::LargeV3.repo_id(), "openai/whisper-large-v3");
    }

    #[test]
    fn test_default_model() {
        let model = WhisperModel::default();
        assert!(matches!(model, WhisperModel::Base));
    }
}
