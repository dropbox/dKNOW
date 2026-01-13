//! Duplicate Detection Plugin
//!
//! Provides perceptual hashing for duplicate/near-duplicate detection

use crate::{DuplicateDetectionConfig, DuplicateDetector, HashAlgorithm, PerceptualHash};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Instant;
use tracing::{debug, info};
use video_extract_core::plugin::PluginData;
use video_extract_core::{
    Context, Operation, Plugin, PluginConfig, PluginError, PluginRequest, PluginResponse,
};

/// Duplicate detection plugin
pub struct DuplicateDetectionPlugin {
    config: PluginConfig,
    detection_config: DuplicateDetectionConfig,
}

impl DuplicateDetectionPlugin {
    /// Create new duplicate detection plugin with configuration
    pub fn new(config: PluginConfig, detection_config: DuplicateDetectionConfig) -> Self {
        Self {
            config,
            detection_config,
        }
    }

    /// Load plugin from YAML configuration
    pub fn from_yaml(yaml_path: impl AsRef<Path>) -> Result<Self, PluginError> {
        let contents = std::fs::read_to_string(yaml_path.as_ref())?;
        let yaml: serde_yaml::Value = serde_yaml::from_str(&contents)
            .map_err(|e| PluginError::ExecutionFailed(format!("Failed to parse YAML: {}", e)))?;

        // Parse standard plugin config
        let plugin_config: PluginConfig = serde_yaml::from_value(yaml.clone()).map_err(|e| {
            PluginError::ExecutionFailed(format!("Failed to parse plugin config: {}", e))
        })?;

        // Parse duplicate_detection section or use defaults
        let detection_config = if let Some(dd_config) = yaml.get("duplicate_detection") {
            serde_yaml::from_value(dd_config.clone()).unwrap_or_default()
        } else {
            DuplicateDetectionConfig::default()
        };

        Ok(Self::new(plugin_config, detection_config))
    }
}

#[async_trait]
impl Plugin for DuplicateDetectionPlugin {
    fn name(&self) -> &str {
        &self.config.name
    }

    fn config(&self) -> &PluginConfig {
        &self.config
    }

    fn supports_input(&self, input_type: &str) -> bool {
        self.config.inputs.iter().any(|s| s == input_type)
    }

    fn produces_output(&self, output_type: &str) -> bool {
        self.config.outputs.iter().any(|s| s == output_type)
    }

    async fn execute(
        &self,
        ctx: &Context,
        request: &PluginRequest,
    ) -> Result<PluginResponse, PluginError> {
        let start = Instant::now();

        // Extract operation parameters
        let (hash_alg, hash_size, threshold) = match &request.operation {
            Operation::DuplicateDetection {
                algorithm,
                hash_size,
                threshold,
            } => (*algorithm, *hash_size, *threshold),
            _ => {
                return Err(PluginError::InvalidInput(
                    "Expected DuplicateDetection operation".to_string(),
                ))
            }
        };

        if ctx.verbose {
            info!(
                "Duplicate detection: algorithm={:?}, hash_size={}, threshold={}",
                hash_alg, hash_size, threshold
            );
        }

        // Handle JSON input (keyframes from upstream plugin)
        if let PluginData::Json(json) = &request.input {
            return handle_keyframes_json(json, hash_alg, hash_size, threshold, &self.detection_config, ctx.verbose, start).await;
        }

        // Get input file path
        let input_path = match &request.input {
            PluginData::FilePath(path) => path.clone(),
            PluginData::Bytes(_) => {
                return Err(PluginError::UnsupportedFormat(
                    "Bytes input not yet supported, use file path".to_string(),
                ))
            }
            _ => return Err(PluginError::InvalidInput("Expected file path or keyframes JSON".to_string())),
        };

        if ctx.verbose {
            debug!("Input file: {:?}", input_path);
        }

        // Convert DuplicateHashAlgorithm to internal HashAlgorithm
        let internal_algorithm = match hash_alg {
            video_extract_core::operation::DuplicateHashAlgorithm::Mean => HashAlgorithm::Mean,
            video_extract_core::operation::DuplicateHashAlgorithm::Gradient => {
                HashAlgorithm::Gradient
            }
            video_extract_core::operation::DuplicateHashAlgorithm::DCT => HashAlgorithm::DCT,
            video_extract_core::operation::DuplicateHashAlgorithm::Block => HashAlgorithm::Block,
            video_extract_core::operation::DuplicateHashAlgorithm::VertGradient => {
                HashAlgorithm::VertGradient
            }
            video_extract_core::operation::DuplicateHashAlgorithm::DoubleGradient => {
                HashAlgorithm::DoubleGradient
            }
        };

        // Create detector with operation parameters
        let mut detection_config = self.detection_config.clone();
        detection_config.hash_algorithm = internal_algorithm;
        detection_config.hash_size = hash_size;
        detection_config.similarity_threshold = threshold;

        // Save video_keyframes before moving detection_config
        let max_video_keyframes = detection_config.video_keyframes;

        let detector = DuplicateDetector::new(detection_config);

        // Detect media type and compute hash
        let file_ext = input_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        let hash = match file_ext.as_str() {
            // RAW camera formats - extract keyframe first via dcraw, then hash the JPEG
            "arw" | "cr2" | "dng" | "nef" | "raf" => {
                info!("RAW format detected - extracting keyframe first for hashing");

                // Create a temporary output directory for the extracted keyframe
                let temp_dir = tempfile::tempdir().map_err(|e| {
                    PluginError::ExecutionFailed(format!("Failed to create temp dir: {}", e))
                })?;

                // Extract keyframe using keyframe extractor (handles dcraw conversion)
                let config = video_audio_keyframe::KeyframeExtractor {
                    interval: 0.0, // Extract first frame immediately
                    max_keyframes: 1, // Only need 1 frame for duplicate detection
                    similarity_threshold: 0, // No deduplication needed
                    thumbnail_sizes: vec![(640, 480)], // Standard thumbnail size
                    output_dir: temp_dir.path().to_path_buf(),
                    use_ffmpeg_cli: false,
                };

                let keyframes = video_audio_keyframe::extract_keyframes(&input_path, config)
                    .map_err(|e| {
                        PluginError::ExecutionFailed(format!("Failed to extract keyframe from RAW: {}", e))
                    })?;

                if keyframes.is_empty() {
                    return Err(PluginError::ExecutionFailed(
                        "No keyframe extracted from RAW file".to_string(),
                    ));
                }

                // Get the thumbnail path for the first keyframe
                let thumbnail_key = "640x480"; // Match the size we configured
                let keyframe_path = keyframes[0].thumbnail_paths.get(thumbnail_key)
                    .ok_or_else(|| {
                        PluginError::ExecutionFailed(
                            format!("No thumbnail found for size {}", thumbnail_key)
                        )
                    })?;

                debug!("Loading extracted keyframe: {:?}", keyframe_path);
                let img_new = image::open(keyframe_path).map_err(|e| {
                    PluginError::ExecutionFailed(format!("Failed to load extracted keyframe: {}", e))
                })?;

                // Convert to img_hash's image 0.23 DynamicImage via raw pixels
                let rgba = img_new.to_rgba8();
                let (width, height) = rgba.dimensions();

                use img_hash::image::{ImageBuffer, Rgba};
                let img_old: ImageBuffer<Rgba<u8>, Vec<u8>> =
                    ImageBuffer::from_raw(width, height, rgba.into_raw()).ok_or_else(|| {
                        PluginError::ExecutionFailed("Failed to convert image buffer".to_string())
                    })?;
                let img = img_hash::image::DynamicImage::ImageRgba8(img_old);

                detector.hash_image(&img).map_err(|e| {
                    PluginError::ExecutionFailed(format!("Image hashing failed: {}", e))
                })?
            }

            // Standard image formats
            "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "ico" | "heic" | "heif" | "avif" => {
                debug!("Computing image perceptual hash");
                // Load with image 0.25 (has all codecs)
                let img_new = image::open(&input_path).map_err(|e| {
                    PluginError::ExecutionFailed(format!("Failed to load image: {}", e))
                })?;

                // Convert to img_hash's image 0.23 DynamicImage via raw pixels
                // This avoids codec issues between image 0.25 and image 0.23
                let rgba = img_new.to_rgba8();
                let (width, height) = rgba.dimensions();

                // Construct img_hash's DynamicImage from raw RGBA pixels
                use img_hash::image::{ImageBuffer, Rgba};
                let img_old: ImageBuffer<Rgba<u8>, Vec<u8>> =
                    ImageBuffer::from_raw(width, height, rgba.into_raw()).ok_or_else(|| {
                        PluginError::ExecutionFailed("Failed to convert image buffer".to_string())
                    })?;
                let img = img_hash::image::DynamicImage::ImageRgba8(img_old);

                detector.hash_image(&img).map_err(|e| {
                    PluginError::ExecutionFailed(format!("Image hashing failed: {}", e))
                })?
            }

            // Video formats
            "mp4" | "mov" | "mkv" | "webm" | "avi" | "ts" | "m2ts" | "mts" | "mpg" | "mpeg"
            | "mxf" | "vob" | "rm" | "rmvb" | "asf" | "dv" => {
                info!("Video format detected - extracting keyframes for multi-frame hashing");

                // Extract all keyframes using zero-copy decoder
                let raw_frames = video_audio_decoder::decode_iframes_zero_copy(&input_path)
                    .map_err(|e| {
                        PluginError::ExecutionFailed(format!("Failed to decode video: {}", e))
                    })?;

                if raw_frames.is_empty() {
                    return Err(PluginError::ExecutionFailed(
                        "No keyframes found in video".to_string(),
                    ));
                }

                // Limit to first N keyframes for performance (configurable via detection_config)
                let max_keyframes = max_video_keyframes.min(raw_frames.len());
                let frames_to_hash = &raw_frames[..max_keyframes];

                info!(
                    "Processing {} keyframes for video hash (total available: {}, max_keyframes={})",
                    frames_to_hash.len(),
                    raw_frames.len(),
                    max_keyframes
                );

                // Convert raw frames to img_hash DynamicImages
                // Pre-allocate keyframe_images Vec with frames_to_hash.len() capacity
                let mut keyframe_images: Vec<img_hash::image::DynamicImage> =
                    Vec::with_capacity(frames_to_hash.len());
                for (i, frame) in frames_to_hash.iter().enumerate() {
                    // Convert raw frame data to image (zero-copy: create slice from raw pointer)
                    // SAFETY: RawFrameBuffer keeps AVFrame alive via Drop, so data_ptr is valid
                    let data_slice = unsafe {
                        std::slice::from_raw_parts(
                            frame.data_ptr,
                            (frame.width * frame.height * 3) as usize,
                        )
                    };

                    // Create RGB image from raw data
                    let img_new =
                        image::RgbImage::from_vec(frame.width, frame.height, data_slice.to_vec())
                            .ok_or_else(|| {
                            PluginError::ExecutionFailed(format!(
                                "Failed to create image from frame {} data",
                                i
                            ))
                        })?;

                    // Convert to img_hash's image 0.23 DynamicImage via raw pixels
                    // Convert RGB to RGBA for compatibility
                    let rgba = image::DynamicImage::ImageRgb8(img_new).to_rgba8();
                    let (width, height) = rgba.dimensions();

                    // Construct img_hash's DynamicImage from raw RGBA pixels
                    use img_hash::image::{ImageBuffer, Rgba};
                    let img_old: ImageBuffer<Rgba<u8>, Vec<u8>> =
                        ImageBuffer::from_raw(width, height, rgba.into_raw()).ok_or_else(|| {
                            PluginError::ExecutionFailed(format!(
                                "Failed to convert image buffer for frame {}",
                                i
                            ))
                        })?;
                    keyframe_images.push(img_hash::image::DynamicImage::ImageRgba8(img_old));
                }

                // Hash all keyframes and concatenate (uses hash_video_keyframes from lib.rs)
                let hash = detector
                    .hash_video_keyframes(&keyframe_images)
                    .map_err(|e| {
                        PluginError::ExecutionFailed(format!("Video hashing failed: {}", e))
                    })?;

                hash
            }

            // Audio formats
            "wav" | "mp3" | "flac" | "m4a" | "ogg" | "aac" => {
                debug!("Audio format detected - extracting and hashing");

                // Extract audio to temporary WAV file
                let temp_dir = tempfile::tempdir().map_err(|e| {
                    PluginError::ExecutionFailed(format!("Failed to create temp dir: {}", e))
                })?;
                let temp_wav = temp_dir.path().join("audio.wav");

                // Extract audio using C FFI (16kHz mono for consistency with transcription)
                video_audio_decoder::c_ffi::extract_audio_to_wav(
                    &input_path,
                    &temp_wav,
                    16000, // 16kHz sample rate
                    1,     // Mono
                )
                .map_err(|e| {
                    PluginError::ExecutionFailed(format!("Audio extraction failed: {}", e))
                })?;

                // Read audio samples from WAV file
                let mut reader = hound::WavReader::open(&temp_wav).map_err(|e| {
                    PluginError::ExecutionFailed(format!("Failed to read WAV: {}", e))
                })?;

                let sample_rate = reader.spec().sample_rate;
                let samples: Vec<f32> = reader
                    .samples::<i16>()
                    .map(|s| s.unwrap_or(0) as f32 / 32768.0) // Convert i16 to f32 [-1.0, 1.0]
                    .collect();

                debug!(
                    "Extracted {} audio samples @ {} Hz for hashing",
                    samples.len(),
                    sample_rate
                );

                // Compute perceptual hash using spectrogram-based fingerprint
                detector.hash_audio(&samples, sample_rate).map_err(|e| {
                    PluginError::ExecutionFailed(format!("Audio hashing failed: {}", e))
                })?
            }

            _ => {
                return Err(PluginError::UnsupportedFormat(format!(
                    "Unsupported file format: {}",
                    file_ext
                )))
            }
        };

        let elapsed = start.elapsed();

        if ctx.verbose {
            info!(
                "Duplicate detection completed in {:.3}s: {} bytes hash",
                elapsed.as_secs_f64(),
                hash.hash.len()
            );
        }

        // Return hash as JSON
        let output = DuplicateDetectionOutput {
            perceptual_hash: hash,
            algorithm: hash_alg,
            hash_size,
            threshold,
        };

        let json = serde_json::to_value(&output).map_err(|e| {
            PluginError::ExecutionFailed(format!("Failed to serialize output: {}", e))
        })?;

        Ok(PluginResponse {
            output: PluginData::Json(json),
            duration: elapsed,
            warnings: vec![],
        })
    }
}

/// Handle keyframes JSON input from upstream plugin (e.g., keyframes -> duplicate-detection)
async fn handle_keyframes_json(
    json: &serde_json::Value,
    hash_alg: video_extract_core::operation::DuplicateHashAlgorithm,
    hash_size: u32,
    threshold: f32,
    detection_config: &DuplicateDetectionConfig,
    verbose: bool,
    start: Instant,
) -> Result<PluginResponse, PluginError> {
    use video_audio_common::Keyframe;

    // Parse keyframes JSON
    let keyframes: Vec<Keyframe> = serde_json::from_value(json.clone()).map_err(|e| {
        PluginError::InvalidInput(format!("Failed to parse keyframes JSON: {}", e))
    })?;

    if keyframes.is_empty() {
        return Err(PluginError::ExecutionFailed(
            "No keyframes in input".to_string(),
        ));
    }

    if verbose {
        info!("Processing {} keyframes from upstream plugin", keyframes.len());
    }

    // Convert algorithm
    let internal_algorithm = match hash_alg {
        video_extract_core::operation::DuplicateHashAlgorithm::Mean => HashAlgorithm::Mean,
        video_extract_core::operation::DuplicateHashAlgorithm::Gradient => HashAlgorithm::Gradient,
        video_extract_core::operation::DuplicateHashAlgorithm::DCT => HashAlgorithm::DCT,
        video_extract_core::operation::DuplicateHashAlgorithm::Block => HashAlgorithm::Block,
        video_extract_core::operation::DuplicateHashAlgorithm::VertGradient => HashAlgorithm::VertGradient,
        video_extract_core::operation::DuplicateHashAlgorithm::DoubleGradient => HashAlgorithm::DoubleGradient,
    };

    // Create detector
    let mut config = detection_config.clone();
    config.hash_algorithm = internal_algorithm;
    config.hash_size = hash_size;
    config.similarity_threshold = threshold;
    let detector = DuplicateDetector::new(config);

    // Get first keyframe's first thumbnail path
    let keyframe = &keyframes[0];
    let thumbnail_path = keyframe
        .thumbnail_paths
        .values()
        .next()
        .ok_or_else(|| {
            PluginError::ExecutionFailed("No thumbnail paths in keyframe".to_string())
        })?;

    if verbose {
        debug!("Loading keyframe thumbnail: {:?}", thumbnail_path);
    }

    // Load and hash the thumbnail image
    let img_new = image::open(thumbnail_path).map_err(|e| {
        PluginError::ExecutionFailed(format!("Failed to load keyframe thumbnail: {}", e))
    })?;

    // Convert to img_hash's image 0.23 DynamicImage via raw pixels
    let rgba = img_new.to_rgba8();
    let (width, height) = rgba.dimensions();

    use img_hash::image::{ImageBuffer, Rgba};
    let img_old: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_raw(width, height, rgba.into_raw()).ok_or_else(|| {
            PluginError::ExecutionFailed("Failed to convert image buffer".to_string())
        })?;
    let img = img_hash::image::DynamicImage::ImageRgba8(img_old);

    let hash = detector.hash_image(&img).map_err(|e| {
        PluginError::ExecutionFailed(format!("Image hashing failed: {}", e))
    })?;

    let elapsed = start.elapsed();

    if verbose {
        info!(
            "Duplicate detection completed in {:.3}s: {} bytes hash",
            elapsed.as_secs_f64(),
            hash.hash.len()
        );
    }

    // Return hash as JSON
    let output = DuplicateDetectionOutput {
        perceptual_hash: hash,
        algorithm: hash_alg,
        hash_size,
        threshold,
    };

    let json = serde_json::to_value(&output).map_err(|e| {
        PluginError::ExecutionFailed(format!("Failed to serialize output: {}", e))
    })?;

    Ok(PluginResponse {
        output: PluginData::Json(json),
        duration: elapsed,
        warnings: vec![],
    })
}

/// Duplicate detection output
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DuplicateDetectionOutput {
    perceptual_hash: PerceptualHash,
    algorithm: video_extract_core::operation::DuplicateHashAlgorithm,
    hash_size: u32,
    threshold: f32,
}
// force rebuild
