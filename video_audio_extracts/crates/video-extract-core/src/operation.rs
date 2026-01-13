//! Operation specifications and data source definitions

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Defines what output is desired and how to produce it
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputSpec {
    /// Pipeline source(s) - nested OutputSpecs to execute first
    pub sources: Vec<OutputSpec>,

    /// The operation to perform
    pub operation: Operation,
}

impl OutputSpec {
    /// Create a new OutputSpec with no sources
    pub fn new(operation: Operation) -> Self {
        Self {
            sources: Vec::with_capacity(2), // Most operations have 0-2 sources
            operation,
        }
    }

    /// Add a source to this OutputSpec
    pub fn with_source(mut self, source: OutputSpec) -> Self {
        self.sources.push(source);
        self
    }
}

/// The operation to perform on media
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Operation {
    // ────────── Data Sources ──────────
    /// Raw input file/URL/S3
    DataSource(DataSource),

    // ────────── Extraction ──────────
    /// Extract audio track (PCM for ML)
    Audio { sample_rate: u32, channels: u8 },

    /// Extract video frames at specified FPS
    Frames { fps: f32, format: PixelFormat },

    /// Extract keyframes (scene changes)
    Keyframes {
        max_frames: Option<u32>,
        min_interval_sec: f32,
    },

    // ────────── Analysis ──────────
    /// Transcribe speech to text
    Transcription {
        language: Option<String>,
        model: WhisperModel,
    },

    /// Speaker diarization (who spoke when)
    Diarization { num_speakers: Option<u32> },

    /// Voice Activity Detection (detect speech segments)
    VoiceActivityDetection {
        aggressiveness: VadAggressiveness,
        min_segment_duration: f32,
    },

    /// Detect objects in frames
    ObjectDetection {
        model: ObjectDetectionModel,
        confidence_threshold: f32,
        classes: Option<Vec<String>>,
    },

    /// Detect faces in frames
    FaceDetection {
        min_size: u32,
        include_landmarks: bool,
    },

    /// Extract text via OCR
    OCR { languages: Vec<String> },

    /// Detect scene changes
    SceneDetection {
        threshold: f32,
        keyframes_only: bool,
    },

    /// Extract embedded subtitles
    SubtitleExtraction {
        track_index: Option<usize>,
        language: Option<String>,
    },

    /// Classify audio events (speech, music, environmental sounds)
    AudioClassification {
        confidence_threshold: Option<f32>,
        top_k: Option<usize>,
    },

    /// Classify acoustic scenes (indoor/outdoor, room size, environment type)
    AcousticSceneClassification {
        confidence_threshold: Option<f32>,
    },

    /// Detect profane language in text (requires transcription)
    ProfanityDetection {
        min_severity: ProfanitySeverity,
        context_words: usize,
    },

    /// Detect duplicate/near-duplicate media using perceptual hashing
    DuplicateDetection {
        algorithm: DuplicateHashAlgorithm,
        hash_size: u32,
        threshold: f32,
    },

    /// Select best thumbnail from keyframes
    SmartThumbnail {
        min_quality: Option<f32>,
        preferred_resolution: Option<String>,
    },

    /// Recognize actions and activities in video
    ActionRecognition {
        min_segment_duration: Option<f64>,
        confidence_threshold: Option<f32>,
        scene_change_threshold: Option<f32>,
    },

    /// Track objects across video frames
    MotionTracking {
        high_confidence_threshold: Option<f32>,
        low_confidence_threshold: Option<f32>,
        detection_threshold_high: Option<f32>,
        detection_threshold_low: Option<f32>,
        max_age: Option<u32>,
        min_hits: Option<u32>,
    },

    /// Estimate human pose (17 COCO keypoints)
    PoseEstimation {
        model: PoseEstimationModel,
        confidence_threshold: f32,
        keypoint_threshold: f32,
    },

    /// Assess image quality (aesthetic and technical quality scores 1-10)
    ImageQualityAssessment { include_distribution: bool },

    /// Detect emotions from facial expressions (7 basic emotions)
    EmotionDetection { include_probabilities: bool },

    /// Audio enhancement metadata (SNR, dynamic range, spectral analysis)
    AudioEnhancementMetadata {},

    /// Classify camera shot types (close-up, medium, wide, aerial)
    ShotClassification {},

    /// Content moderation (NSFW detection)
    ContentModeration {
        include_categories: bool,
        nsfw_threshold: f32,
    },

    /// Detect brand logos in images
    LogoDetection {
        confidence_threshold: f32,
        logo_classes: Option<Vec<String>>,
    },

    /// Separate music into stems (vocals, drums, bass, other)
    MusicSourceSeparation { stems: Option<Vec<String>> },

    /// Estimate depth from single images (monocular depth estimation)
    DepthEstimation {
        input_size: u32,
        normalize: bool,
        resize_to_original: bool,
    },

    /// Generate natural language captions from images/video
    CaptionGeneration {
        max_length: usize,
        use_beam_search: bool,
        num_beams: usize,
    },

    // ────────── Embeddings ──────────
    /// Vision embeddings (CLIP)
    VisionEmbeddings { model: VisionModel },

    /// Text embeddings (Sentence-Transformers)
    TextEmbeddings { model: TextModel },

    /// Audio embeddings (CLAP)
    AudioEmbeddings { model: AudioModel },

    // ────────── Fusion ──────────
    /// Cross-modal temporal fusion
    Fusion {
        align_modalities: bool,
        extract_entities: bool,
        build_relationships: bool,
    },

    // ────────── Metadata ──────────
    /// Extract video/audio metadata
    Metadata { include_streams: bool },

    // ────────── Utility ──────────
    /// Convert media file to different format/codec/container
    FormatConversion {
        preset: Option<String>,
        video_codec: Option<String>,
        audio_codec: Option<String>,
        container: Option<String>,
        video_bitrate: Option<String>,
        audio_bitrate: Option<String>,
        width: Option<u32>,
        height: Option<u32>,
        crf: Option<u32>,
        output_file: Option<String>,
    },
}

impl Operation {
    /// Get the output type name for this operation
    pub fn output_type_name(&self) -> &'static str {
        match self {
            Operation::DataSource(_) => "DataSource",
            Operation::Audio { .. } => "Audio",
            Operation::Frames { .. } => "Frames",
            Operation::Keyframes { .. } => "Keyframes",
            Operation::Transcription { .. } => "Transcription",
            Operation::Diarization { .. } => "Diarization",
            Operation::VoiceActivityDetection { .. } => "VoiceActivityDetection",
            Operation::ObjectDetection { .. } => "ObjectDetection",
            Operation::FaceDetection { .. } => "FaceDetection",
            Operation::OCR { .. } => "OCR",
            Operation::SceneDetection { .. } => "SceneDetection",
            Operation::SubtitleExtraction { .. } => "SubtitleExtraction",
            Operation::AudioClassification { .. } => "AudioClassification",
            Operation::AcousticSceneClassification { .. } => "AcousticSceneClassification",
            Operation::ProfanityDetection { .. } => "ProfanityDetection",
            Operation::DuplicateDetection { .. } => "DuplicateDetection",
            Operation::SmartThumbnail { .. } => "SmartThumbnail",
            Operation::ActionRecognition { .. } => "ActionRecognition",
            Operation::MotionTracking { .. } => "MotionTracking",
            Operation::PoseEstimation { .. } => "PoseEstimation",
            Operation::ImageQualityAssessment { .. } => "ImageQualityAssessment",
            Operation::EmotionDetection { .. } => "EmotionDetection",
            Operation::AudioEnhancementMetadata { .. } => "AudioEnhancementMetadata",
            Operation::ShotClassification { .. } => "ShotClassification",
            Operation::ContentModeration { .. } => "ContentModeration",
            Operation::LogoDetection { .. } => "LogoDetection",
            Operation::MusicSourceSeparation { .. } => "MusicSourceSeparation",
            Operation::DepthEstimation { .. } => "DepthEstimation",
            Operation::CaptionGeneration { .. } => "CaptionGeneration",
            Operation::VisionEmbeddings { .. } => "VisionEmbeddings",
            Operation::TextEmbeddings { .. } => "TextEmbeddings",
            Operation::AudioEmbeddings { .. } => "AudioEmbeddings",
            Operation::Fusion { .. } => "Fusion",
            Operation::Metadata { .. } => "Metadata",
            Operation::FormatConversion { .. } => "FormatConversion",
        }
    }

    /// Get a short name for logging
    pub fn name(&self) -> &'static str {
        match self {
            Operation::DataSource(_) => "datasource",
            Operation::Audio { .. } => "audio",
            Operation::Frames { .. } => "frames",
            Operation::Keyframes { .. } => "keyframes",
            Operation::Transcription { .. } => "transcription",
            Operation::Diarization { .. } => "diarization",
            Operation::VoiceActivityDetection { .. } => "voiceactivitydetection",
            Operation::ObjectDetection { .. } => "objectdetection",
            Operation::FaceDetection { .. } => "facedetection",
            Operation::OCR { .. } => "ocr",
            Operation::SceneDetection { .. } => "scenedetection",
            Operation::SubtitleExtraction { .. } => "subtitleextraction",
            Operation::AudioClassification { .. } => "audioclassification",
            Operation::AcousticSceneClassification { .. } => "acousticsceneclassification",
            Operation::ProfanityDetection { .. } => "profanitydetection",
            Operation::DuplicateDetection { .. } => "duplicatedetection",
            Operation::SmartThumbnail { .. } => "smartthumbnail",
            Operation::ActionRecognition { .. } => "actionrecognition",
            Operation::MotionTracking { .. } => "motiontracking",
            Operation::PoseEstimation { .. } => "poseestimation",
            Operation::ImageQualityAssessment { .. } => "imagequalityassessment",
            Operation::EmotionDetection { .. } => "emotiondetection",
            Operation::AudioEnhancementMetadata { .. } => "audioenhancementmetadata",
            Operation::ShotClassification { .. } => "shotclassification",
            Operation::ContentModeration { .. } => "contentmoderation",
            Operation::LogoDetection { .. } => "logodetection",
            Operation::MusicSourceSeparation { .. } => "musicsourceseparation",
            Operation::DepthEstimation { .. } => "depthestimation",
            Operation::CaptionGeneration { .. } => "captiongeneration",
            Operation::VisionEmbeddings { .. } => "visionembeddings",
            Operation::TextEmbeddings { .. } => "textembeddings",
            Operation::AudioEmbeddings { .. } => "audioembeddings",
            Operation::Fusion { .. } => "fusion",
            Operation::Metadata { .. } => "metadata",
            Operation::FormatConversion { .. } => "formatconversion",
        }
    }
}

/// Data source for input
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "source_type")]
pub enum DataSource {
    /// Local file path
    LocalFile {
        path: PathBuf,
        format_hint: Option<String>,
    },

    /// HTTP/HTTPS URL
    Url {
        url: String,
        format_hint: Option<String>,
    },

    /// S3 object
    S3 {
        bucket: String,
        key: String,
        format_hint: Option<String>,
    },

    /// Raw bytes with format hint
    Bytes {
        #[serde(with = "serde_bytes")]
        data: Vec<u8>,
        format_hint: String,
    },
}

impl DataSource {
    /// Get format hint or infer from path/URL
    pub fn format_hint(&self) -> Result<String, crate::error::PluginError> {
        match self {
            DataSource::LocalFile { path, format_hint } => {
                if let Some(hint) = format_hint {
                    Ok(hint.clone())
                } else {
                    path.extension()
                        .and_then(|ext| ext.to_str())
                        .map(|s| s.to_lowercase())
                        .ok_or_else(|| {
                            crate::error::PluginError::InvalidInput(
                                "Cannot determine file format from path".to_string(),
                            )
                        })
                }
            }
            DataSource::Url { url, format_hint } => {
                if let Some(hint) = format_hint {
                    Ok(hint.clone())
                } else {
                    // Try to extract extension from URL path
                    url.rsplit('/')
                        .next()
                        .and_then(|filename| filename.rsplit('.').next())
                        .map(|s| s.to_lowercase())
                        .ok_or_else(|| {
                            crate::error::PluginError::InvalidInput(
                                "Cannot determine format from URL".to_string(),
                            )
                        })
                }
            }
            DataSource::S3 {
                key, format_hint, ..
            } => {
                if let Some(hint) = format_hint {
                    Ok(hint.clone())
                } else {
                    key.rsplit('.')
                        .next()
                        .map(|s| s.to_lowercase())
                        .ok_or_else(|| {
                            crate::error::PluginError::InvalidInput(
                                "Cannot determine format from S3 key".to_string(),
                            )
                        })
                }
            }
            DataSource::Bytes { format_hint, .. } => Ok(format_hint.clone()),
        }
    }
}

// ────────── Model Enums ──────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PixelFormat {
    RGB24,
    RGBA,
    BGR24,
    BGRA,
    YUV420P,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WhisperModel {
    Tiny,   // 39M params
    Base,   // 74M params
    Small,  // 244M params
    Medium, // 769M params
    Large,  // 1550M params
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VadAggressiveness {
    Quality,        // 0 - Least aggressive, best quality
    LowBitrate,     // 1 - Low bitrate
    Aggressive,     // 2 - Aggressive
    VeryAggressive, // 3 - Most aggressive, may drop some speech
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProfanitySeverity {
    Mild,     // damn, hell, crap
    Moderate, // ass, bitch, shit
    Strong,   // f-word, explicit terms
    Severe,   // slurs, hate speech
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DuplicateHashAlgorithm {
    Mean,           // Average Hash (aHash) - fast, simple
    Gradient,       // Gradient Hash - robust to color/brightness changes
    DCT,            // DCT Hash (pHash) - most accurate
    Block,          // Block Hash - good for varied content
    VertGradient,   // Vertical Gradient Hash (dHash)
    DoubleGradient, // Double Gradient Hash
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ObjectDetectionModel {
    YoloV8n, // Nano - fastest
    YoloV8s, // Small
    YoloV8m, // Medium
    YoloV8l, // Large
    YoloV8x, // Extra large - most accurate
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PoseEstimationModel {
    YoloV8nPose,     // Nano - fastest (FP32)
    YoloV8nPoseInt8, // Nano - quantized INT8 (3.6MB, 20-50% faster)
    YoloV8sPose,     // Small
    YoloV8mPose,     // Medium
    YoloV8lPose,     // Large
    YoloV8xPose,     // Extra large - most accurate
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VisionModel {
    ClipVitB32, // 512-dim embeddings
    ClipVitL14, // 768-dim embeddings
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TextModel {
    AllMiniLmL6V2,  // 384-dim, fast
    AllMpnetBaseV2, // 768-dim, accurate
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AudioModel {
    ClapHtsatFused, // 512-dim CLAP embeddings
}

mod serde_bytes {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(bytes)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Vec::<u8>::deserialize(deserializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_output_type_name() {
        let op = Operation::Transcription {
            language: Some("en".to_string()),
            model: WhisperModel::Base,
        };
        assert_eq!(op.output_type_name(), "Transcription");
    }

    #[test]
    fn test_data_source_format_hint_from_path() {
        let ds = DataSource::LocalFile {
            path: PathBuf::from("test.mp4"),
            format_hint: None,
        };
        assert_eq!(ds.format_hint().unwrap(), "mp4");
    }

    #[test]
    fn test_output_spec_with_source() {
        let audio_spec = OutputSpec::new(Operation::Audio {
            sample_rate: 16000,
            channels: 1,
        });

        let transcription_spec = OutputSpec::new(Operation::Transcription {
            language: None,
            model: WhisperModel::Base,
        })
        .with_source(audio_spec);

        assert_eq!(transcription_spec.sources.len(), 1);
    }
}
