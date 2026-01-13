//! Semantic embeddings extraction module
//!
//! Provides multimodal embedding extraction:
//! - Vision embeddings (CLIP)
//! - Text embeddings (Sentence-Transformers)
//! - Audio embeddings (CLAP)

pub mod plugin;

use anyhow::{Context, Result};
use fftw::array::AlignedVec;
use fftw::plan::*;
use fftw::types::*;
use image::DynamicImage;
use ndarray::{Array2, Array4};
use ort::{
    session::Session,
    value::{TensorRef, Value},
};
use serde::{Deserialize, Serialize};
use tokenizers::Tokenizer;
use tracing::{debug, info};

/// Vision embedding configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionEmbeddingConfig {
    /// Model variant to use
    pub model: CLIPModel,
    /// Path to the ONNX model file
    pub model_path: String,
    /// Whether to normalize embeddings to unit length
    pub normalize: bool,
    /// Input image size (default: 224x224 for CLIP)
    pub image_size: u32,
}

impl Default for VisionEmbeddingConfig {
    fn default() -> Self {
        Self {
            model: CLIPModel::VitB32,
            model_path: "models/embeddings/clip_vit_b32.onnx".to_string(),
            normalize: true,
            image_size: 224,
        }
    }
}

/// CLIP model variants
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CLIPModel {
    /// ViT-B/32: 512-dim embeddings, 149M params
    VitB32,
    /// ViT-L/14: 768-dim embeddings, 428M params
    VitL14,
}

impl CLIPModel {
    /// Get the embedding dimension for this model
    #[must_use]
    pub fn embedding_dim(&self) -> usize {
        match self {
            CLIPModel::VitB32 => 512,
            CLIPModel::VitL14 => 768,
        }
    }
}

/// Text embedding configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextEmbeddingConfig {
    /// Model name (e.g., "all-MiniLM-L6-v2", "all-mpnet-base-v2")
    pub model: String,
    /// Path to the ONNX model file
    pub model_path: String,
    /// Whether to normalize embeddings to unit length
    pub normalize: bool,
    /// Maximum sequence length
    pub max_length: usize,
}

impl Default for TextEmbeddingConfig {
    fn default() -> Self {
        Self {
            model: "all-MiniLM-L6-v2".to_string(),
            model_path: "models/embeddings/all_minilm_l6_v2.onnx".to_string(),
            normalize: true,
            max_length: 256,
        }
    }
}

/// Audio embedding configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioEmbeddingConfig {
    /// Model name (e.g., "laion/clap-htsat-fused")
    pub model: String,
    /// Path to the ONNX model file
    pub model_path: String,
    /// Whether to normalize embeddings to unit length
    pub normalize: bool,
    /// Sample rate for audio processing
    pub sample_rate: u32,
}

impl Default for AudioEmbeddingConfig {
    fn default() -> Self {
        Self {
            model: "laion/clap-htsat-fused".to_string(),
            model_path: "models/embeddings/clap.onnx".to_string(),
            normalize: true,
            sample_rate: 48000,
        }
    }
}

/// Vision embeddings extractor
pub struct VisionEmbeddings {
    session: Session,
    config: VisionEmbeddingConfig,
}

impl VisionEmbeddings {
    /// Create a new vision embeddings extractor
    #[allow(dead_code)]
    pub fn new(config: VisionEmbeddingConfig) -> Result<Self> {
        info!("Loading vision embedding model from: {}", config.model_path);

        let session = Session::builder()
            .context("Failed to create session builder")?
            .commit_from_file(&config.model_path)
            .with_context(|| format!("Failed to load ONNX model from {}", config.model_path))?;

        Ok(Self { session, config })
    }

    /// Extract embeddings from images using a pre-loaded session (static method for caching)
    pub fn extract_embeddings_with_session(
        session: &mut Session,
        config: &VisionEmbeddingConfig,
        images: &[DynamicImage],
    ) -> Result<Vec<Vec<f32>>> {
        if images.is_empty() {
            return Ok(Vec::new());
        }

        debug!(
            "Extracting vision embeddings for {} images using {:?}",
            images.len(),
            config.model
        );

        // Preprocess images
        let input_tensor = Self::preprocess_images_static(config, images)?;

        // Convert to ort::TensorRef
        let pixel_values = TensorRef::from_array_view(input_tensor.view())
            .context("Failed to convert input to ort::TensorRef")?;

        // CLIP model requires text inputs even for vision-only inference
        // Provide dummy text tokens: [BOS, "a", "photo", "of", "a", "photo", EOS]
        // Match batch size with pixel_values
        let batch_size = images.len();
        let dummy_tokens = vec![49406, 320, 2368, 539, 320, 2368, 49407];
        let mut input_ids_vec = Vec::with_capacity(batch_size * 7);
        for _ in 0..batch_size {
            input_ids_vec.extend_from_slice(&dummy_tokens);
        }
        let dummy_input_ids =
            ndarray::Array2::<i64>::from_shape_vec((batch_size, 7), input_ids_vec)
                .context("Failed to create dummy input_ids")?;
        let dummy_attention_mask = ndarray::Array2::<i64>::ones((batch_size, 7));

        let input_ids = Value::from_array(dummy_input_ids)
            .context("Failed to convert input_ids to ort::Value")?;
        let attention_mask = Value::from_array(dummy_attention_mask)
            .context("Failed to convert attention_mask to ort::Value")?;

        // Run inference with named inputs
        let outputs = session
            .run(ort::inputs![
                "pixel_values" => pixel_values,
                "input_ids" => input_ids,
                "attention_mask" => attention_mask,
            ])
            .context("Failed to run ONNX inference")?;

        // Extract image_embeds from output (index 3 in output list)
        let (_shape, embeddings_data) = outputs
            .get("image_embeds")
            .context("Failed to get image_embeds from outputs")?
            .try_extract_tensor::<f32>()
            .context("Failed to extract embeddings tensor")?;

        // Convert to Vec<Vec<f32>>
        let embedding_dim = config.model.embedding_dim();
        let mut embeddings = Vec::with_capacity(images.len());

        for i in 0..images.len() {
            let start = i * embedding_dim;
            let end = start + embedding_dim;
            let mut embedding = embeddings_data[start..end].to_vec();

            // Normalize if configured
            if config.normalize {
                normalize_vector(&mut embedding);
            }

            embeddings.push(embedding);
        }

        debug!("Extracted {} vision embeddings", embeddings.len());
        Ok(embeddings)
    }

    /// Extract embeddings from images
    ///
    /// # Arguments
    /// * `images` - Input images to extract embeddings from
    ///
    /// # Returns
    /// Vector of embeddings, one per image (512-dim for ViT-B/32, 768-dim for ViT-L/14)
    #[allow(dead_code)]
    pub fn extract_embeddings(&mut self, images: &[DynamicImage]) -> Result<Vec<Vec<f32>>> {
        Self::extract_embeddings_with_session(&mut self.session, &self.config, images)
    }

    /// Preprocess images for CLIP model (static version)
    fn preprocess_images_static(
        config: &VisionEmbeddingConfig,
        images: &[DynamicImage],
    ) -> Result<Array4<f32>> {
        let size = config.image_size;
        let batch_size = images.len();

        // Create tensor with shape [batch_size, 3, height, width]
        let mut tensor = Array4::<f32>::zeros((batch_size, 3, size as usize, size as usize));

        for (i, img) in images.iter().enumerate() {
            // Resize image
            let img = img.resize_exact(size, size, image::imageops::FilterType::Lanczos3);
            let img = img.to_rgb8();

            // Convert to tensor format (NCHW) with ImageNet normalization
            // mean = [0.485, 0.456, 0.406], std = [0.229, 0.224, 0.225]
            for y in 0..size as usize {
                for x in 0..size as usize {
                    let pixel = img.get_pixel(x as u32, y as u32);
                    tensor[[i, 0, y, x]] = (f32::from(pixel[0]) / 255.0 - 0.485) / 0.229;
                    tensor[[i, 1, y, x]] = (f32::from(pixel[1]) / 255.0 - 0.456) / 0.224;
                    tensor[[i, 2, y, x]] = (f32::from(pixel[2]) / 255.0 - 0.406) / 0.225;
                }
            }
        }

        Ok(tensor)
    }

    /// Preprocess images for CLIP model (instance method wrapper)
    #[allow(dead_code)]
    fn preprocess_images(&self, images: &[DynamicImage]) -> Result<Array4<f32>> {
        Self::preprocess_images_static(&self.config, images)
    }
}

/// Text embeddings extractor
pub struct TextEmbeddings {
    session: Session,
    tokenizer: Tokenizer,
    config: TextEmbeddingConfig,
}

impl TextEmbeddings {
    /// Create a new text embeddings extractor
    #[allow(dead_code)]
    pub fn new(config: TextEmbeddingConfig) -> Result<Self> {
        info!("Loading text embedding model from: {}", config.model_path);

        let session = Session::builder()
            .context("Failed to create session builder")?
            .commit_from_file(&config.model_path)
            .with_context(|| format!("Failed to load ONNX model from {}", config.model_path))?;

        // Load tokenizer from the same directory as the model
        use std::path::Path;
        let model_path = Path::new(&config.model_path);
        let tokenizer_path = model_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Invalid model path"))?
            .join("tokenizer_minilm/tokenizer.json");

        info!("Loading tokenizer from: {}", tokenizer_path.display());
        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {e}"))?;

        Ok(Self {
            session,
            tokenizer,
            config,
        })
    }

    /// Extract embeddings from texts using a pre-loaded session and tokenizer (static method for caching)
    pub fn extract_embeddings_with_session(
        session: &mut Session,
        tokenizer: &Tokenizer,
        config: &TextEmbeddingConfig,
        texts: &[String],
    ) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        debug!(
            "Extracting text embeddings for {} texts using {}",
            texts.len(),
            config.model
        );

        let mut result_embeddings = Vec::with_capacity(texts.len());

        // Process each text
        for text in texts {
            // Tokenize
            let encoding = tokenizer
                .encode(text.as_str(), true)
                .map_err(|e| anyhow::anyhow!("Tokenization failed: {e}"))?;

            let tokens = encoding.get_ids();
            let attention_mask = encoding.get_attention_mask();

            // Prepare inputs as i64 arrays
            let mut input_ids = Vec::with_capacity(tokens.len());
            input_ids.extend(tokens.iter().map(|&t| i64::from(t)));
            let mut attention_mask_i64 = Vec::with_capacity(attention_mask.len());
            attention_mask_i64.extend(attention_mask.iter().map(|&m| i64::from(m)));

            let seq_len = input_ids.len();

            // Create input tensors [1, seq_len]
            let input_ids_array = Array2::from_shape_vec((1, seq_len), input_ids)
                .context("Failed to create input_ids array")?;
            let attention_mask_array = Array2::from_shape_vec((1, seq_len), attention_mask_i64)
                .context("Failed to create attention_mask array")?;

            // Create token_type_ids (all zeros for single sentence input)
            let token_type_ids = vec![0i64; seq_len];
            let token_type_ids_array = Array2::from_shape_vec((1, seq_len), token_type_ids)
                .context("Failed to create token_type_ids array")?;

            // Convert to ort::TensorRef
            let input_ids_tensor = TensorRef::from_array_view(input_ids_array.view())
                .context("Failed to convert input_ids to ort::TensorRef")?;
            let attention_mask_tensor = TensorRef::from_array_view(attention_mask_array.view())
                .context("Failed to convert attention_mask to ort::TensorRef")?;
            let token_type_ids_tensor = TensorRef::from_array_view(token_type_ids_array.view())
                .context("Failed to convert token_type_ids to ort::TensorRef")?;

            // Run inference with all three inputs (some models require token_type_ids)
            let outputs = session
                .run(ort::inputs![
                    input_ids_tensor,
                    attention_mask_tensor,
                    token_type_ids_tensor
                ])
                .context("Failed to run ONNX inference")?;

            // Extract last_hidden_state [1, seq_len, hidden_dim]
            let (shape, hidden_states) = outputs[0]
                .try_extract_tensor::<f32>()
                .context("Failed to extract hidden states tensor")?;

            debug!("Hidden states shape: {:?}", shape);

            // Apply mean pooling over sequence dimension
            // Weights by attention mask to ignore padding tokens
            let hidden_dim = shape[2] as usize;
            let mut pooled = vec![0.0f32; hidden_dim];
            let mut sum_mask = 0.0f32;

            for (i, &mask_val_u32) in attention_mask.iter().enumerate().take(seq_len) {
                let mask_val = mask_val_u32 as f32;
                sum_mask += mask_val;

                for (j, pooled_val) in pooled.iter_mut().enumerate().take(hidden_dim) {
                    let idx = i * hidden_dim + j;
                    *pooled_val += hidden_states[idx] * mask_val;
                }
            }

            // Normalize by sum of attention mask
            if sum_mask > 0.0 {
                for val in &mut pooled {
                    *val /= sum_mask;
                }
            }

            // L2 normalization if configured
            if config.normalize {
                normalize_vector(&mut pooled);
            }

            result_embeddings.push(pooled);
        }

        debug!("Extracted {} text embeddings", result_embeddings.len());
        Ok(result_embeddings)
    }

    /// Extract embeddings from texts
    ///
    /// # Arguments
    /// * `texts` - Input texts to extract embeddings from
    ///
    /// # Returns
    /// Vector of embeddings, one per text (384-dim for MiniLM-L6-v2, 768-dim for mpnet-base-v2)
    #[allow(dead_code)]
    pub fn extract_embeddings(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        Self::extract_embeddings_with_session(
            &mut self.session,
            &self.tokenizer,
            &self.config,
            texts,
        )
    }
}

/// Audio embeddings extractor
pub struct AudioEmbeddings {
    session: Session,
    config: AudioEmbeddingConfig,
}

impl AudioEmbeddings {
    /// Create a new audio embeddings extractor
    #[allow(dead_code)]
    pub fn new(config: AudioEmbeddingConfig) -> Result<Self> {
        info!("Loading audio embedding model from: {}", config.model_path);

        let session = Session::builder()
            .context("Failed to create session builder")?
            .commit_from_file(&config.model_path)
            .with_context(|| format!("Failed to load ONNX model from {}", config.model_path))?;

        Ok(Self { session, config })
    }

    /// Extract embeddings from audio clips using a pre-loaded session (static method for caching)
    pub fn extract_embeddings_with_session(
        session: &mut Session,
        config: &AudioEmbeddingConfig,
        audio_clips: &[Vec<f32>],
    ) -> Result<Vec<Vec<f32>>> {
        if audio_clips.is_empty() {
            return Ok(Vec::new());
        }

        debug!(
            "Extracting audio embeddings for {} clips using {}",
            audio_clips.len(),
            config.model
        );

        let mut result_embeddings = Vec::with_capacity(audio_clips.len());

        // Process each audio clip
        for audio in audio_clips {
            // Preprocess audio
            let input_features = Self::preprocess_audio_static(config, audio)?;

            // Convert to ort::TensorRef
            let input_tensor = TensorRef::from_array_view(input_features.view())
                .context("Failed to convert input_features to ort::TensorRef")?;

            // Run ONNX inference with named input (CLAP model expects "input_features")
            let outputs = session
                .run(ort::inputs!["input_features" => input_tensor])
                .context("Failed to run ONNX inference")?;

            // Extract pooler_output [1, 768]
            let (_shape, embedding_data) = outputs[0]
                .try_extract_tensor::<f32>()
                .context("Failed to extract embedding tensor")?;

            let mut embedding = embedding_data.to_vec();

            // Normalize if configured
            if config.normalize {
                normalize_vector(&mut embedding);
            }

            result_embeddings.push(embedding);
        }

        debug!("Extracted {} audio embeddings", result_embeddings.len());
        Ok(result_embeddings)
    }

    /// Extract embeddings from audio clips
    ///
    /// # Arguments
    /// * `audio_clips` - Input audio clips as raw PCM samples at the configured sample rate
    ///
    /// # Returns
    /// Vector of embeddings, one per audio clip (768-dim for CLAP)
    #[allow(dead_code)]
    pub fn extract_embeddings(&mut self, audio_clips: &[Vec<f32>]) -> Result<Vec<Vec<f32>>> {
        Self::extract_embeddings_with_session(&mut self.session, &self.config, audio_clips)
    }

    /// Preprocess audio to mel-spectrogram features (pure Rust implementation) - static version
    ///
    /// CLAP expects mel-spectrograms with the following parameters:
    /// - Sample rate: 48000 Hz
    /// - FFT window size: 1024
    /// - Hop length: 480
    /// - Mel filterbanks: 64
    /// - Window function: Hann
    ///
    /// Output shape: [1, 1, `time_frames`, 64]
    fn preprocess_audio_static(
        config: &AudioEmbeddingConfig,
        audio: &[f32],
    ) -> Result<Array4<f32>> {
        // CLAP audio preprocessing parameters
        const FFT_SIZE: usize = 1024;
        const HOP_LENGTH: usize = 480;
        const N_MELS: usize = 64;
        const EXPECTED_DURATION_SEC: f32 = 10.0; // CLAP expects 10 second clips

        let sample_rate = config.sample_rate as usize;
        let expected_samples = (sample_rate as f32 * EXPECTED_DURATION_SEC) as usize;

        // Pad or trim audio to expected length
        let mut audio_padded = audio.to_vec();
        if audio_padded.len() < expected_samples {
            audio_padded.resize(expected_samples, 0.0);
        } else {
            audio_padded.truncate(expected_samples);
        }

        // Compute STFT (Short-Time Fourier Transform) using FFTW
        // Create FFTW plan (optimizes for hardware, reusable)
        let mut plan: C2CPlan32 = C2CPlan::aligned(
            &[FFT_SIZE],
            Sign::Forward,
            Flag::MEASURE, // FFTW auto-optimizes for hardware
        )
        .context("Failed to create FFTW plan")?;

        // Calculate number of frames
        let n_frames = (audio_padded.len() - FFT_SIZE) / HOP_LENGTH + 1;

        // Create Hann window
        let mut window = Vec::with_capacity(FFT_SIZE);
        for i in 0..FFT_SIZE {
            window.push(
                0.5 - 0.5
                    * ((2.0 * std::f32::consts::PI * i as f32) / (FFT_SIZE as f32 - 1.0)).cos(),
            );
        }

        // Compute power spectrogram
        let mut spectrogram = Vec::with_capacity(n_frames * FFT_SIZE / 2);

        // Allocate FFTW-aligned buffers (reuse across frames)
        let mut input = AlignedVec::new(FFT_SIZE);
        let mut output = AlignedVec::new(FFT_SIZE);

        for frame_idx in 0..n_frames {
            let start = frame_idx * HOP_LENGTH;
            let end = start + FFT_SIZE;

            if end > audio_padded.len() {
                break;
            }

            // Apply window and copy to FFTW input buffer
            for (i, (&sample, &window_val)) in
                audio_padded[start..end].iter().zip(&window).enumerate()
            {
                input[i] = c32::new(sample * window_val, 0.0);
            }

            // Compute FFT (SIMD-optimized)
            plan.c2c(&mut input, &mut output)
                .context("FFT computation failed")?;

            // Compute power spectrum (magnitude squared) for positive frequencies
            for complex_val in output.iter().take(FFT_SIZE / 2) {
                let magnitude = complex_val.norm();
                spectrogram.push(magnitude * magnitude);
            }
        }

        // Apply mel filterbank
        let mel_filterbank = create_mel_filterbank(N_MELS, FFT_SIZE / 2, sample_rate);
        let mel_spec = apply_mel_filterbank(&spectrogram, &mel_filterbank, n_frames, FFT_SIZE / 2);

        // Convert to log scale (log10(max(mel_spec, 1e-10)))
        let mut log_mel = Vec::with_capacity(mel_spec.len());
        log_mel.extend(mel_spec.iter().map(|x| (x.max(1e-10)).log10()));

        // Normalize to roughly [-1, 1] range (CLAP uses layer normalization, but we approximate)
        let mean: f32 = log_mel.iter().sum::<f32>() / log_mel.len() as f32;
        let variance: f32 =
            log_mel.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / log_mel.len() as f32;
        let std = variance.sqrt() + 1e-8;

        for val in &mut log_mel {
            *val = (*val - mean) / std;
        }

        // CLAP model expects [batch_size, 4, 1001, 64]
        // The "4" dimension is for fusion architecture (likely 4 frequency bands or feature maps)
        // We replicate the mel-spectrogram across 4 channels and ensure exactly 1001 frames
        let actual_frames = log_mel.len() / N_MELS;

        // Pad or trim to exactly 1001 frames
        const EXPECTED_FRAMES: usize = 1001;
        let mut mel_data = log_mel;
        if actual_frames < EXPECTED_FRAMES {
            // Pad with zeros
            mel_data.resize(EXPECTED_FRAMES * N_MELS, 0.0);
        } else if actual_frames > EXPECTED_FRAMES {
            // Trim to expected size
            mel_data.truncate(EXPECTED_FRAMES * N_MELS);
        }

        // Replicate across 4 channels: [1, 4, 1001, 64]
        let mut replicated_data = Vec::with_capacity(4 * EXPECTED_FRAMES * N_MELS);
        for _ in 0..4 {
            replicated_data.extend_from_slice(&mel_data);
        }

        let array = Array4::from_shape_vec((1, 4, EXPECTED_FRAMES, N_MELS), replicated_data)
            .context("Failed to create mel-spectrogram array")?;

        debug!("Created mel-spectrogram with shape: {:?}", array.shape());

        Ok(array)
    }

    /// Preprocess audio to mel-spectrogram features (instance method wrapper)
    #[allow(dead_code)]
    fn preprocess_audio(&self, audio: &[f32]) -> Result<Array4<f32>> {
        Self::preprocess_audio_static(&self.config, audio)
    }
}

/// Normalize a vector to unit length (L2 normalization)
fn normalize_vector(vec: &mut [f32]) {
    let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 1e-12 {
        for x in vec.iter_mut() {
            *x /= norm;
        }
    }
}

/// Create mel filterbank for mel-spectrogram computation
///
/// # Arguments
/// * `n_mels` - Number of mel filterbanks
/// * `n_fft_bins` - Number of FFT bins (`FFT_SIZE` / 2)
/// * `sample_rate` - Audio sample rate
///
/// # Returns
/// Vec of length `n_mels` * `n_fft_bins`, where filterbank[`mel_idx` * `n_fft_bins` + `freq_idx`]
/// is the weight for applying mel filter `mel_idx` to FFT bin `freq_idx`
fn create_mel_filterbank(n_mels: usize, n_fft_bins: usize, sample_rate: usize) -> Vec<f32> {
    // Mel scale conversion functions
    let hz_to_mel = |hz: f32| 2595.0 * (1.0 + hz / 700.0).log10();
    let mel_to_hz = |mel: f32| 700.0 * (10.0_f32.powf(mel / 2595.0) - 1.0);

    // Frequency range: 0 to Nyquist frequency
    let nyquist = (sample_rate / 2) as f32;

    // Convert frequency range to mel scale
    let mel_low = hz_to_mel(0.0);
    let mel_high = hz_to_mel(nyquist);

    // Create n_mels+2 equally spaced points in mel scale
    let mut mel_points: Vec<f32> = Vec::with_capacity(n_mels + 2);
    mel_points.extend(
        (0..=n_mels + 1)
            .map(|i| mel_low + (mel_high - mel_low) * (i as f32) / (n_mels + 1) as f32)
            .map(mel_to_hz),
    );

    // Convert mel points to FFT bin indices
    let mut bin_points: Vec<f32> = Vec::with_capacity(mel_points.len());
    bin_points.extend(
        mel_points
            .iter()
            .map(|hz| hz * (n_fft_bins as f32) / nyquist),
    );

    // Create filterbank matrix
    let mut filterbank = vec![0.0f32; n_mels * n_fft_bins];

    for mel_idx in 0..n_mels {
        let left = bin_points[mel_idx];
        let center = bin_points[mel_idx + 1];
        let right = bin_points[mel_idx + 2];

        for bin_idx in 0..n_fft_bins {
            let freq_bin = bin_idx as f32;

            // Triangular filter
            let weight = if freq_bin >= left && freq_bin <= center {
                (freq_bin - left) / (center - left)
            } else if freq_bin > center && freq_bin <= right {
                (right - freq_bin) / (right - center)
            } else {
                0.0
            };

            filterbank[mel_idx * n_fft_bins + bin_idx] = weight;
        }
    }

    filterbank
}

/// Apply mel filterbank to power spectrogram
///
/// # Arguments
/// * `spectrogram` - Power spectrogram [`n_frames` * `n_fft_bins`]
/// * `filterbank` - Mel filterbank [`n_mels` * `n_fft_bins`]
/// * `n_frames` - Number of time frames
/// * `n_fft_bins` - Number of FFT bins
///
/// # Returns
/// Mel spectrogram [`n_frames` * `n_mels`]
fn apply_mel_filterbank(
    spectrogram: &[f32],
    filterbank: &[f32],
    n_frames: usize,
    n_fft_bins: usize,
) -> Vec<f32> {
    let n_mels = filterbank.len() / n_fft_bins;
    let mut mel_spec = vec![0.0f32; n_frames * n_mels];

    for frame_idx in 0..n_frames {
        for mel_idx in 0..n_mels {
            let mut sum = 0.0f32;
            for bin_idx in 0..n_fft_bins {
                let spec_val = spectrogram[frame_idx * n_fft_bins + bin_idx];
                let filter_val = filterbank[mel_idx * n_fft_bins + bin_idx];
                sum += spec_val * filter_val;
            }
            mel_spec[frame_idx * n_mels + mel_idx] = sum;
        }
    }

    mel_spec
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clip_model_embedding_dim() {
        assert_eq!(CLIPModel::VitB32.embedding_dim(), 512);
        assert_eq!(CLIPModel::VitL14.embedding_dim(), 768);
    }

    #[test]
    fn test_normalize_vector() {
        let mut vec = vec![3.0, 4.0];
        normalize_vector(&mut vec);
        assert!((vec[0] - 0.6).abs() < 1e-6);
        assert!((vec[1] - 0.8).abs() < 1e-6);
    }

    #[test]
    fn test_default_configs() {
        let vision_config = VisionEmbeddingConfig::default();
        assert_eq!(vision_config.image_size, 224);
        assert!(vision_config.normalize);

        let text_config = TextEmbeddingConfig::default();
        assert_eq!(text_config.model, "all-MiniLM-L6-v2");
        assert!(text_config.normalize);

        let audio_config = AudioEmbeddingConfig::default();
        assert_eq!(audio_config.sample_rate, 48000);
        assert!(audio_config.normalize);
    }

    #[test]
    #[ignore] // Requires ONNX model and Python dependencies
    fn test_audio_embeddings_real() {
        // Create a simple sine wave (1 second at 48kHz)
        let sample_rate = 48000;
        let duration = 1.0;
        let frequency = 440.0; // A4 note
        let num_samples = (sample_rate as f32 * duration) as usize;

        let mut audio: Vec<f32> = Vec::with_capacity(num_samples);
        audio.extend((0..num_samples).map(|i| {
            let t = i as f32 / sample_rate as f32;
            (2.0 * std::f32::consts::PI * frequency * t).sin()
        }));

        // Create audio embeddings extractor
        let config = AudioEmbeddingConfig::default();
        let mut extractor = AudioEmbeddings::new(config).expect("Failed to create AudioEmbeddings");

        // Extract embeddings
        let embeddings = extractor
            .extract_embeddings(&[audio])
            .expect("Failed to extract embeddings");

        // Verify we got one embedding
        assert_eq!(embeddings.len(), 1);

        // Verify embedding dimension (768 for CLAP)
        assert_eq!(embeddings[0].len(), 768);

        // Verify embedding is not all zeros
        let sum: f32 = embeddings[0].iter().sum();
        assert!(sum.abs() > 1e-6, "Embedding should not be all zeros");

        // Verify embedding is normalized (L2 norm should be ~1.0)
        let norm: f32 = embeddings[0].iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-4, "Embedding should be normalized");
    }
}
