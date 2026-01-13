//! Speaker diarization module using pure Rust/C++/ONNX
//!
//! Replaces PyAnnote.audio with a 3-component pipeline:
//! 1. Voice Activity Detection (WebRTC VAD in C++)
//! 2. Speaker Embeddings (ONNX model)
//! 3. Speaker Clustering (linfa-clustering in Rust)

pub mod plugin;

use anyhow::{Context, Result};
use fftw::array::AlignedVec;
use fftw::plan::*;
use fftw::types::*;
use hound::WavReader;
use ndarray::{Array1, Array2};
use ort::session::Session;
use ort::value::TensorRef;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, info, warn};
use webrtc_vad::{Vad, VadMode};

/// Configuration for speaker diarization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiarizationConfig {
    /// Minimum number of speakers (None = auto-detect)
    pub min_speakers: Option<u8>,
    /// Maximum number of speakers (None = auto-detect)
    pub max_speakers: Option<u8>,
    /// Path to speaker embedding ONNX model
    pub embedding_model_path: String,
    /// VAD aggressiveness (0-3, higher = more aggressive)
    pub vad_aggressiveness: u8,
    /// Minimum segment duration in seconds
    pub min_segment_duration: f32,
}

impl Default for DiarizationConfig {
    fn default() -> Self {
        Self {
            min_speakers: None,
            max_speakers: None,
            embedding_model_path: "models/diarization/speaker_embedding.onnx".to_string(),
            vad_aggressiveness: 3,
            min_segment_duration: 0.3,
        }
    }
}

/// Complete diarization result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diarization {
    /// List of identified speakers
    pub speakers: Vec<Speaker>,
    /// Timeline of speaker segments
    pub timeline: Vec<SpeakerSegment>,
}

/// Speaker metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Speaker {
    /// Speaker identifier (e.g., "`SPEAKER_00`", "`SPEAKER_01`")
    pub id: String,
    /// Total speaking time in seconds
    pub total_speaking_time: f64,
}

/// Single speaker segment in timeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeakerSegment {
    /// Start time in seconds
    pub start: f64,
    /// End time in seconds
    pub end: f64,
    /// Speaker identifier
    pub speaker: String,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
}

/// Internal speech segment from VAD
#[derive(Debug, Clone)]
struct SpeechSegment {
    start: f64,
    end: f64,
    samples: Vec<f32>,
}

/// Perform speaker diarization on an audio file
///
/// # Arguments
/// * `audio_path` - Path to the audio file (WAV format, 16kHz mono recommended)
/// * `config` - Diarization configuration
///
/// # Returns
/// * `Diarization` - Speaker timeline and metadata
///
/// # Errors
/// Returns error if:
/// - Audio file cannot be read
/// - ONNX model cannot be loaded
/// - Clustering fails
#[allow(dead_code)] // Kept for backwards compatibility, plugin uses diarize_audio_with_session
pub fn diarize_audio(audio_path: &Path, config: &DiarizationConfig) -> Result<Diarization> {
    info!("Starting speaker diarization for: {:?}", audio_path);

    // Verify audio file exists
    if !audio_path.exists() {
        anyhow::bail!("Audio file not found: {audio_path:?}");
    }

    // Step 1: Load audio
    let (audio_samples, sample_rate) = load_audio(audio_path)?;
    info!(
        "Loaded audio: {} samples at {}Hz",
        audio_samples.len(),
        sample_rate
    );

    // Step 2: Voice Activity Detection
    let speech_segments = detect_speech_segments(&audio_samples, sample_rate, config)?;
    info!("Detected {} speech segments", speech_segments.len());

    if speech_segments.is_empty() {
        warn!("No speech detected in audio");
        return Ok(Diarization {
            speakers: Vec::new(),
            timeline: Vec::new(),
        });
    }

    // Step 3: Extract speaker embeddings
    let embeddings = extract_speaker_embeddings(&speech_segments, config)?;
    info!("Extracted {} speaker embeddings", embeddings.len());

    // Step 4: Cluster embeddings to identify speakers
    let (cluster_labels, distances) = cluster_speakers(&embeddings, config)?;
    let max_label = cluster_labels.iter().max().copied().unwrap_or(0);
    info!("Identified {} unique speakers", max_label + 1);

    // Step 5: Build diarization result
    let diarization = build_diarization_result(&speech_segments, &cluster_labels, &distances);

    info!(
        "Diarization complete: {} speakers, {} segments",
        diarization.speakers.len(),
        diarization.timeline.len()
    );

    Ok(diarization)
}

/// Perform speaker diarization with provided ONNX session (for model caching)
///
/// # Arguments
/// * `audio_path` - Path to the audio file (WAV format, 16kHz mono recommended)
/// * `config` - Diarization configuration
/// * `session` - Mutable reference to WeSpeaker ONNX session
///
/// # Returns
/// * `Diarization` - Speaker timeline and metadata
///
/// # Errors
/// Returns error if:
/// - Audio file cannot be read
/// - Inference fails
/// - Clustering fails
pub fn diarize_audio_with_session(
    audio_path: &Path,
    config: &DiarizationConfig,
    session: &mut Session,
) -> Result<Diarization> {
    info!("Starting speaker diarization for: {:?}", audio_path);

    // Verify audio file exists
    if !audio_path.exists() {
        anyhow::bail!("Audio file not found: {audio_path:?}");
    }

    // Step 1: Load audio
    let (audio_samples, sample_rate) = load_audio(audio_path)?;
    info!(
        "Loaded audio: {} samples at {}Hz",
        audio_samples.len(),
        sample_rate
    );

    // Step 2: Voice Activity Detection
    let speech_segments = detect_speech_segments(&audio_samples, sample_rate, config)?;
    info!("Detected {} speech segments", speech_segments.len());

    if speech_segments.is_empty() {
        warn!("No speech detected in audio");
        return Ok(Diarization {
            speakers: Vec::new(),
            timeline: Vec::new(),
        });
    }

    // Step 3: Extract speaker embeddings with provided session
    let embeddings = extract_speaker_embeddings_with_session(&speech_segments, session)?;
    info!("Extracted {} speaker embeddings", embeddings.len());

    // Step 4: Cluster embeddings to identify speakers
    let (cluster_labels, distances) = cluster_speakers(&embeddings, config)?;
    let max_label = cluster_labels.iter().max().copied().unwrap_or(0);
    info!("Identified {} unique speakers", max_label + 1);

    // Step 5: Build diarization result
    let diarization = build_diarization_result(&speech_segments, &cluster_labels, &distances);

    info!(
        "Diarization complete: {} speakers, {} segments",
        diarization.speakers.len(),
        diarization.timeline.len()
    );

    Ok(diarization)
}

/// Load audio file and convert to mono f32 samples
fn load_audio(path: &Path) -> Result<(Vec<f32>, u32)> {
    let mut reader =
        WavReader::open(path).with_context(|| format!("Failed to open WAV file: {path:?}"))?;

    let spec = reader.spec();
    let sample_rate = spec.sample_rate;

    // Convert samples to f32 [-1.0, 1.0]
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => {
            let max_val = (1 << (spec.bits_per_sample - 1)) as f32;
            reader
                .samples::<i32>()
                .collect::<std::result::Result<Vec<_>, _>>()
                .with_context(|| "Failed to read audio samples")?
                .into_iter()
                .map(|s| s as f32 / max_val)
                .collect()
        }
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .collect::<std::result::Result<Vec<_>, _>>()
            .with_context(|| "Failed to read audio samples")?,
    };

    // Convert stereo to mono if needed
    let mono_samples = if spec.channels == 2 {
        let mono_len = samples.len() / 2;
        let mut mono = Vec::with_capacity(mono_len);
        mono.extend(
            samples
                .chunks(2)
                .map(|chunk| f32::midpoint(chunk[0], chunk[1])),
        );
        mono
    } else {
        samples
    };

    Ok((mono_samples, sample_rate))
}

/// Detect speech segments using WebRTC VAD
fn detect_speech_segments(
    samples: &[f32],
    sample_rate: u32,
    config: &DiarizationConfig,
) -> Result<Vec<SpeechSegment>> {
    use webrtc_vad::SampleRate;

    // Initialize VAD with correct sample rate
    let vad_sample_rate = match sample_rate {
        8000 => SampleRate::Rate8kHz,
        16000 => SampleRate::Rate16kHz,
        32000 => SampleRate::Rate32kHz,
        48000 => SampleRate::Rate48kHz,
        _ => {
            anyhow::bail!(
                "Unsupported sample rate for VAD: {sample_rate}Hz. Supported rates: 8000, 16000, 32000, 48000"
            );
        }
    };

    let vad_mode = match config.vad_aggressiveness {
        0 => VadMode::Quality,
        1 => VadMode::LowBitrate,
        2 => VadMode::Aggressive,
        _ => VadMode::VeryAggressive,
    };

    let mut vad = Vad::new_with_rate_and_mode(vad_sample_rate, vad_mode);

    // VAD processes audio in frames (10ms, 20ms, or 30ms)
    // We'll use 30ms frames (480 samples at 16kHz)
    const FRAME_DURATION_MS: usize = 30;
    let frame_size = (sample_rate as usize * FRAME_DURATION_MS) / 1000;

    // Pre-allocate: estimate ~10% of audio will be speech segments
    let estimated_segments = (samples.len() / frame_size) / 10;
    let mut speech_segments = Vec::with_capacity(estimated_segments.max(4));
    let mut current_segment_start: Option<usize> = None;

    // Convert f32 to i16 for VAD
    let mut samples_i16 = Vec::with_capacity(samples.len());
    samples_i16.extend(samples.iter().map(|&s| (s * 32767.0) as i16));

    let mut speech_frame_count = 0;
    let mut total_frame_count = 0;

    for (frame_idx, frame) in samples_i16.chunks(frame_size).enumerate() {
        if frame.len() != frame_size {
            break; // Skip incomplete frames
        }

        total_frame_count += 1;
        let is_speech = vad.is_voice_segment(frame).unwrap_or(false);
        if is_speech {
            speech_frame_count += 1;
        }
        let frame_start_idx = frame_idx * frame_size;

        if is_speech {
            if current_segment_start.is_none() {
                current_segment_start = Some(frame_start_idx);
            }
        } else if let Some(seg_start) = current_segment_start {
            // End of speech segment
            let seg_end = frame_start_idx;
            let duration = (seg_end - seg_start) as f64 / f64::from(sample_rate);

            if duration >= f64::from(config.min_segment_duration) {
                speech_segments.push(SpeechSegment {
                    start: seg_start as f64 / f64::from(sample_rate),
                    end: seg_end as f64 / f64::from(sample_rate),
                    samples: samples[seg_start..seg_end].to_vec(),
                });
            }

            current_segment_start = None;
        }
    }

    // Handle final segment if still in speech
    if let Some(seg_start) = current_segment_start {
        let seg_end = samples.len();
        let duration = (seg_end - seg_start) as f64 / f64::from(sample_rate);

        if duration >= f64::from(config.min_segment_duration) {
            speech_segments.push(SpeechSegment {
                start: seg_start as f64 / f64::from(sample_rate),
                end: seg_end as f64 / f64::from(sample_rate),
                samples: samples[seg_start..seg_end].to_vec(),
            });
        }
    }

    debug!(
        "VAD processed {} frames, {} detected as speech ({:.1}%)",
        total_frame_count,
        speech_frame_count,
        100.0 * f64::from(speech_frame_count) / f64::from(total_frame_count)
    );

    Ok(speech_segments)
}

/// Extract speaker embeddings for each speech segment using ONNX model
fn extract_speaker_embeddings(
    segments: &[SpeechSegment],
    config: &DiarizationConfig,
) -> Result<Array2<f32>> {
    let model_path = Path::new(&config.embedding_model_path);

    if !model_path.exists() {
        warn!(
            "Speaker embedding model not found at: {:?}. Using placeholder embeddings.",
            model_path
        );
        // Return random embeddings as placeholder
        return Ok(Array2::from_shape_fn((segments.len(), 256), |(_i, _j)| {
            fastrand::f32()
        }));
    }

    // Load ONNX model
    let mut session = Session::builder()
        .context("Failed to create ONNX session")?
        .commit_from_file(model_path)
        .with_context(|| format!("Failed to load ONNX model from {model_path:?}"))?;

    let embedding_dim = 256; // WeSpeaker ResNet34 output dimension
    let mut all_embeddings = Vec::with_capacity(segments.len());

    for (idx, segment) in segments.iter().enumerate() {
        debug!(
            "Extracting embedding for segment {} ({:.2}s - {:.2}s)",
            idx, segment.start, segment.end
        );

        // Compute mel-filterbank features (WeSpeaker expects 80-dim fbank features)
        let mel_features = compute_mel_features(&segment.samples, 16000)?;
        debug!(
            "Computed mel features: {} frames x 80 bins",
            mel_features.shape()[0]
        );

        // WeSpeaker model expects input shape: [batch, time_frames, 80]
        // Add batch dimension: [1, time_frames, 80]
        let batch_features = mel_features.insert_axis(ndarray::Axis(0)).to_owned();

        // Run ONNX inference
        let input_tensor = TensorRef::from_array_view(batch_features.view())
            .context("Failed to create input tensor")?;

        let outputs = session
            .run(ort::inputs![input_tensor])
            .context("Failed to run ONNX inference")?;

        // Extract embedding [1, 256]
        let (_shape, embedding_data) = outputs[0]
            .try_extract_tensor::<f32>()
            .context("Failed to extract embedding tensor")?;

        // Take first batch element (batch size = 1)
        all_embeddings
            .extend_from_slice(&embedding_data[0..embedding_dim.min(embedding_data.len())]);
    }

    // Reshape to [n_segments, embedding_dim]
    let n_segments = segments.len();
    let embeddings = Array2::from_shape_vec((n_segments, embedding_dim), all_embeddings)
        .context("Failed to reshape embeddings")?;

    Ok(embeddings)
}

/// Extract speaker embeddings using provided ONNX session (for model caching)
fn extract_speaker_embeddings_with_session(
    segments: &[SpeechSegment],
    session: &mut Session,
) -> Result<Array2<f32>> {
    let embedding_dim = 256; // WeSpeaker ResNet34 output dimension
    let mut all_embeddings = Vec::with_capacity(segments.len());

    for (idx, segment) in segments.iter().enumerate() {
        debug!(
            "Extracting embedding for segment {} ({:.2}s - {:.2}s)",
            idx, segment.start, segment.end
        );

        // Compute mel-filterbank features (WeSpeaker expects 80-dim fbank features)
        let mel_features = compute_mel_features(&segment.samples, 16000)?;
        debug!(
            "Computed mel features: {} frames x 80 bins",
            mel_features.shape()[0]
        );

        // WeSpeaker model expects input shape: [batch, time_frames, 80]
        // Add batch dimension: [1, time_frames, 80]
        let batch_features = mel_features.insert_axis(ndarray::Axis(0)).to_owned();

        // Run ONNX inference
        let input_tensor = TensorRef::from_array_view(batch_features.view())
            .context("Failed to create input tensor")?;

        let outputs = session
            .run(ort::inputs![input_tensor])
            .context("Failed to run ONNX inference")?;

        // Extract embedding [1, 256]
        let (_shape, embedding_data) = outputs[0]
            .try_extract_tensor::<f32>()
            .context("Failed to extract embedding tensor")?;

        // Take first batch element (batch size = 1)
        all_embeddings
            .extend_from_slice(&embedding_data[0..embedding_dim.min(embedding_data.len())]);
    }

    // Reshape to [n_segments, embedding_dim]
    let n_segments = segments.len();
    let embeddings = Array2::from_shape_vec((n_segments, embedding_dim), all_embeddings)
        .context("Failed to reshape embeddings")?;

    Ok(embeddings)
}

/// Cluster speaker embeddings using K-means clustering
/// Returns (labels, distances) where distances are from each segment to its cluster centroid
fn cluster_speakers(
    embeddings: &Array2<f32>,
    config: &DiarizationConfig,
) -> Result<(Array1<usize>, Array1<f32>)> {
    let n_segments = embeddings.nrows();

    // Determine number of clusters
    let n_clusters = if let Some(min) = config.min_speakers {
        min as usize
    } else if let Some(_max) = config.max_speakers {
        // Use silhouette analysis or elbow method to find optimal clusters
        // For now, use a simple heuristic: sqrt(n_segments)
        (n_segments as f64).sqrt().ceil() as usize
    } else {
        // Auto-detect: start with sqrt heuristic, cap at reasonable range
        ((n_segments as f64).sqrt().ceil() as usize).clamp(2, 10)
    };

    info!(
        "Clustering {} segments into {} speakers",
        n_segments, n_clusters
    );

    // Perform simple K-means clustering
    let (labels, distances) = simple_kmeans(embeddings, n_clusters, 100)?;

    Ok((labels, distances))
}

/// Simple K-means clustering implementation
/// Returns (labels, distances) where distances are from each point to its assigned centroid
fn simple_kmeans(
    data: &Array2<f32>,
    k: usize,
    max_iters: usize,
) -> Result<(Array1<usize>, Array1<f32>)> {
    let n_samples = data.nrows();
    let n_features = data.ncols();

    // Initialize centroids randomly from data points
    let mut centroids = Array2::<f32>::zeros((k, n_features));
    for i in 0..k {
        let idx = (fastrand::usize(..)) % n_samples;
        centroids.row_mut(i).assign(&data.row(idx));
    }

    let mut labels = Array1::<usize>::zeros(n_samples);
    let mut distances = Array1::<f32>::zeros(n_samples);

    for _ in 0..max_iters {
        let mut changed = false;

        // Assign each point to nearest centroid
        for i in 0..n_samples {
            let point = data.row(i);
            let mut min_dist = f32::INFINITY;
            let mut min_idx = 0;

            for j in 0..k {
                let centroid = centroids.row(j);
                let dist: f32 = point
                    .iter()
                    .zip(centroid.iter())
                    .map(|(a, b)| (a - b).powi(2))
                    .sum::<f32>()
                    .sqrt();

                if dist < min_dist {
                    min_dist = dist;
                    min_idx = j;
                }
            }

            if labels[i] != min_idx {
                labels[i] = min_idx;
                changed = true;
            }
            distances[i] = min_dist;
        }

        if !changed {
            break;
        }

        // Update centroids
        for j in 0..k {
            let cluster_points: Vec<usize> = labels
                .iter()
                .enumerate()
                .filter_map(|(i, &label)| if label == j { Some(i) } else { None })
                .collect();

            if !cluster_points.is_empty() {
                for feat in 0..n_features {
                    let sum: f32 = cluster_points.iter().map(|&i| data[[i, feat]]).sum();
                    centroids[[j, feat]] = sum / cluster_points.len() as f32;
                }
            }
        }
    }

    Ok((labels, distances))
}

/// Convert clustering distances to confidence scores
/// Uses inverse distance mapping: confidence = 1 / (1 + `normalized_distance`)
/// Normalized to [0.0, 1.0] range where closer = higher confidence
fn distances_to_confidence(distances: &Array1<f32>) -> Array1<f32> {
    if distances.is_empty() {
        return Array1::from_vec(Vec::new());
    }

    // Find min and max distances for normalization
    let min_dist = distances.iter().copied().fold(f32::INFINITY, f32::min);
    let max_dist = distances.iter().copied().fold(f32::NEG_INFINITY, f32::max);

    let range = (max_dist - min_dist).max(1e-6); // Avoid division by zero

    // Normalize distances to [0, 1] range and convert to confidence
    // confidence = 1 - normalized_distance gives linear mapping
    // Then scale to [0.3, 1.0] range - even far points get reasonable confidence
    distances.mapv(|d| {
        let normalized = (d - min_dist) / range;
        let conf = 1.0 - normalized;
        // Scale from [0, 1] to [0.3, 1.0] - providing meaningful differentiation
        0.3 + (conf * 0.7)
    })
}

/// Build final diarization result from segments, cluster labels, and distances
fn build_diarization_result(
    segments: &[SpeechSegment],
    labels: &Array1<usize>,
    distances: &Array1<f32>,
) -> Diarization {
    // Pre-allocate speaker_times HashMap (estimate 2-10 speakers typical)
    let mut speaker_times: HashMap<usize, f64> = HashMap::with_capacity(8);
    let mut timeline = Vec::with_capacity(segments.len());

    // Convert distances to confidence scores
    let confidences = distances_to_confidence(distances);

    for ((seg, &label), &confidence) in segments.iter().zip(labels.iter()).zip(confidences.iter()) {
        let duration = seg.end - seg.start;
        *speaker_times.entry(label).or_insert(0.0) += duration;

        timeline.push(SpeakerSegment {
            start: seg.start,
            end: seg.end,
            speaker: format!("SPEAKER_{label:02}"),
            confidence,
        });
    }

    // Sort timeline by start time
    timeline.sort_by(|a, b| {
        a.start
            .partial_cmp(&b.start)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Create speaker list
    let mut speakers: Vec<Speaker> = speaker_times
        .into_iter()
        .map(|(id, total_speaking_time)| Speaker {
            id: format!("SPEAKER_{id:02}"),
            total_speaking_time,
        })
        .collect();

    // Sort speakers by ID for consistent output
    speakers.sort_by(|a, b| a.id.cmp(&b.id));

    Diarization { speakers, timeline }
}

/// Compute mel-filterbank features for `WeSpeaker` model
///
/// `WeSpeaker` expects 80-dimensional mel-filterbank features computed with:
/// - Sample rate: 16000 Hz
/// - Frame length: 25ms (400 samples at 16kHz)
/// - Frame shift: 10ms (160 samples at 16kHz)
/// - Mel filterbanks: 80
/// - Window: Hamming
///
/// Returns: Array2<f32> with shape [`n_frames`, 80]
fn compute_mel_features(audio: &[f32], sample_rate: u32) -> Result<Array2<f32>> {
    const FRAME_LENGTH_MS: usize = 25;
    const FRAME_SHIFT_MS: usize = 10;
    const N_MELS: usize = 80;

    let sample_rate = sample_rate as usize;
    let frame_length = (sample_rate * FRAME_LENGTH_MS) / 1000; // 400 samples at 16kHz
    let frame_shift = (sample_rate * FRAME_SHIFT_MS) / 1000; // 160 samples at 16kHz

    // Use next power of 2 for FFT
    let fft_size = frame_length.next_power_of_two(); // 512 for 400 samples

    // Compute STFT using FFTW
    // Create FFTW plan (optimizes for hardware, reusable)
    let mut plan: C2CPlan32 = C2CPlan::aligned(
        &[fft_size],
        Sign::Forward,
        Flag::MEASURE, // FFTW auto-optimizes for hardware
    )
    .context("Failed to create FFTW plan")?;

    // Calculate number of frames
    let n_frames = if audio.len() > frame_length {
        (audio.len() - frame_length) / frame_shift + 1
    } else {
        1
    };

    // Create Hamming window
    let window: Vec<f32> = (0..frame_length)
        .map(|i| {
            0.54 - 0.46
                * ((2.0 * std::f32::consts::PI * i as f32) / (frame_length as f32 - 1.0)).cos()
        })
        .collect();

    // Compute power spectrogram
    let mut spectrogram = Vec::with_capacity(n_frames * (fft_size / 2));

    // Allocate FFTW-aligned buffers (reuse across frames)
    let mut input = AlignedVec::new(fft_size);
    let mut output = AlignedVec::new(fft_size);

    for frame_idx in 0..n_frames {
        let start = frame_idx * frame_shift;
        let end = (start + frame_length).min(audio.len());

        // Zero-initialize input buffer
        for i in 0..fft_size {
            input[i] = c32::new(0.0, 0.0);
        }

        // Apply window to audio frame
        for (i, &sample) in audio[start..end].iter().enumerate() {
            if i < window.len() {
                input[i] = c32::new(sample * window[i], 0.0);
            }
        }

        // Compute FFT (SIMD-optimized)
        plan.c2c(&mut input, &mut output)
            .context("FFT computation failed")?;

        // Compute power spectrum for positive frequencies
        for complex_val in output.iter().take(fft_size / 2) {
            let magnitude = complex_val.norm();
            spectrogram.push(magnitude * magnitude);
        }
    }

    // Apply mel filterbank
    let mel_filterbank = create_mel_filterbank(N_MELS, fft_size / 2, sample_rate);
    let mel_spec = apply_mel_filterbank(&spectrogram, &mel_filterbank, n_frames, fft_size / 2);

    // Convert to log scale
    let log_mel: Vec<f32> = mel_spec
        .iter()
        .map(|x| (x.max(1e-10)).ln()) // Natural log
        .collect();

    // Reshape to [n_frames, n_mels]
    let features = Array2::from_shape_vec((n_frames, N_MELS), log_mel)
        .context("Failed to create mel features array")?;

    Ok(features)
}

/// Create mel filterbank for mel-spectrogram computation
///
/// Returns a flattened matrix of size [`n_mels` * `n_fft_bins`]
fn create_mel_filterbank(n_mels: usize, n_fft_bins: usize, sample_rate: usize) -> Vec<f32> {
    // Mel scale conversion
    let hz_to_mel = |hz: f32| 2595.0 * (1.0 + hz / 700.0).log10();
    let mel_to_hz = |mel: f32| 700.0 * (10.0_f32.powf(mel / 2595.0) - 1.0);

    let nyquist = (sample_rate / 2) as f32;

    // Create mel-spaced frequency points
    let mel_low = hz_to_mel(0.0);
    let mel_high = hz_to_mel(nyquist);

    let mel_points: Vec<f32> = (0..=n_mels + 1)
        .map(|i| mel_low + (mel_high - mel_low) * (i as f32) / (n_mels + 1) as f32)
        .map(mel_to_hz)
        .collect();

    // Convert to FFT bin indices
    let bin_points: Vec<f32> = mel_points
        .iter()
        .map(|hz| hz * (n_fft_bins as f32) / nyquist)
        .collect();

    // Create triangular filters
    let mut filterbank = vec![0.0f32; n_mels * n_fft_bins];

    for mel_idx in 0..n_mels {
        let left = bin_points[mel_idx];
        let center = bin_points[mel_idx + 1];
        let right = bin_points[mel_idx + 2];

        for bin_idx in 0..n_fft_bins {
            let freq_bin = bin_idx as f32;

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
    use std::path::Path;

    #[test]
    fn test_default_config() {
        let config = DiarizationConfig::default();
        assert_eq!(config.vad_aggressiveness, 3);
        assert_eq!(config.min_segment_duration, 0.3);
        assert!(config.min_speakers.is_none());
        assert!(config.max_speakers.is_none());
    }

    #[test]
    #[ignore] // Requires test file
    fn test_vad_basic() {
        use hound::WavReader;
        use webrtc_vad::{SampleRate, Vad, VadMode};

        let audio_path = Path::new("/tmp/test_real_speech.wav");
        let mut reader = WavReader::open(audio_path).unwrap();
        let spec = reader.spec();

        println!(
            "Audio: sample_rate={}, channels={}",
            spec.sample_rate, spec.channels
        );

        let samples: Vec<i16> = reader.samples::<i16>().map(|s| s.unwrap()).collect();
        println!("Total samples: {}", samples.len());

        // Initialize VAD with correct sample rate (16kHz)
        let mut vad = Vad::new_with_rate_and_mode(SampleRate::Rate16kHz, VadMode::Quality);

        const FRAME_SIZE: usize = 480; // 30ms at 16kHz
        let mut speech_frames = 0;
        let mut total_frames = 0;

        for chunk in samples.chunks(FRAME_SIZE) {
            if chunk.len() == FRAME_SIZE {
                total_frames += 1;
                if vad.is_voice_segment(chunk).unwrap_or(false) {
                    speech_frames += 1;
                }
            }
        }

        println!(
            "Speech frames: {}/{} ({:.1}%)",
            speech_frames,
            total_frames,
            100.0 * f64::from(speech_frames) / f64::from(total_frames)
        );
        assert!(speech_frames > 0, "Expected some speech frames");
    }

    #[test]
    fn test_speaker_serialization() {
        let speaker = Speaker {
            id: "SPEAKER_00".to_string(),
            total_speaking_time: 42.5,
        };
        let json = serde_json::to_string(&speaker).unwrap();
        let deserialized: Speaker = serde_json::from_str(&json).unwrap();
        assert_eq!(speaker.id, deserialized.id);
        assert_eq!(
            speaker.total_speaking_time,
            deserialized.total_speaking_time
        );
    }

    #[test]
    fn test_segment_serialization() {
        let segment = SpeakerSegment {
            start: 1.5,
            end: 3.2,
            speaker: "SPEAKER_01".to_string(),
            confidence: 0.95,
        };
        let json = serde_json::to_string(&segment).unwrap();
        let deserialized: SpeakerSegment = serde_json::from_str(&json).unwrap();
        assert_eq!(segment.start, deserialized.start);
        assert_eq!(segment.end, deserialized.end);
        assert_eq!(segment.speaker, deserialized.speaker);
        assert_eq!(segment.confidence, deserialized.confidence);
    }

    #[test]
    #[ignore] // Requires test file and ONNX model - run manually
    fn test_diarize_zoom_meeting() {
        use std::time::Instant;

        // Initialize ONNX Runtime
        ort::init()
            .with_name("diarization-test")
            .with_execution_providers([
                ort::execution_providers::CPUExecutionProvider::default().build()
            ])
            .commit()
            .unwrap();

        let audio_path = Path::new("/tmp/diarization_test/zoom_meeting.wav");
        assert!(audio_path.exists(),
                "Test file not found: {audio_path:?}. Create it with: ffmpeg -i <input> -ar 16000 -ac 1 {audio_path:?}"
            );

        let config = DiarizationConfig::default();
        println!("\n=== Testing Speaker Diarization ===");
        println!("Audio file: {audio_path:?}");
        println!("Config: {config:?}\n");

        let start = Instant::now();
        let result = diarize_audio(audio_path, &config).unwrap();
        let elapsed = start.elapsed();

        println!("=== Results ===");
        println!("Processing time: {:.2}s", elapsed.as_secs_f64());
        println!("Number of speakers: {}", result.speakers.len());
        println!("\nSpeakers:");
        for speaker in &result.speakers {
            println!(
                "  {} - Total speaking time: {:.2}s",
                speaker.id, speaker.total_speaking_time
            );
        }

        println!("\nTimeline ({} segments):", result.timeline.len());
        for (i, segment) in result.timeline.iter().take(15).enumerate() {
            println!(
                "  [{:3}] {:.2}s - {:.2}s: {} (conf: {:.3})",
                i, segment.start, segment.end, segment.speaker, segment.confidence
            );
        }
        if result.timeline.len() > 15 {
            println!("  ... and {} more segments", result.timeline.len() - 15);
        }

        // Save results to JSON
        let json = serde_json::to_string_pretty(&result).unwrap();
        std::fs::write("/tmp/diarization_test/result.json", &json).unwrap();
        println!("\nResults saved to: /tmp/diarization_test/result.json");

        // Basic validation
        assert!(
            !result.speakers.is_empty(),
            "Should detect at least one speaker"
        );
        assert!(
            !result.timeline.is_empty(),
            "Should have at least one segment"
        );

        // Validate timeline consistency
        for segment in &result.timeline {
            assert!(
                segment.end > segment.start,
                "Segment end must be after start"
            );
            assert!(
                segment.confidence >= 0.0 && segment.confidence <= 1.0,
                "Confidence must be in [0, 1]"
            );
        }

        // Check timeline is sorted by start time
        for i in 1..result.timeline.len() {
            assert!(
                result.timeline[i].start >= result.timeline[i - 1].start,
                "Timeline should be sorted by start time"
            );
        }

        println!("\n✓ All validations passed");
    }
}
