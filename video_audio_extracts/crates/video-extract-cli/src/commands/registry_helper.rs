//! Shared plugin registry helper
//!
//! Provides a centralized function to register all available plugins.
//! This eliminates code duplication across debug, performance, bulk, and plugins commands.

use anyhow::{Context, Result};
use audio_enhancement_metadata::plugin::AudioEnhancementMetadataPlugin;
use format_conversion::plugin::FormatConversionPlugin;
use shot_classification::plugin::ShotClassificationPlugin;
use std::sync::Arc;
use transcription::plugin::TranscriptionPlugin;
use video_audio_acoustic_scene_classification::plugin::AcousticSceneClassificationPlugin;
use video_audio_action_recognition::plugin::ActionRecognitionPlugin;
use video_audio_caption_generation::plugin::CaptionGenerationPlugin;
use video_audio_classification::plugin::AudioClassificationPlugin;
use video_audio_content_moderation::plugin::ContentModerationPlugin;
use video_audio_depth_estimation::plugin::DepthEstimationPlugin;
use video_audio_diarization::plugin::DiarizationPlugin;
use video_audio_duplicate_detection::plugin::DuplicateDetectionPlugin;
use video_audio_embeddings::plugin::EmbeddingsPlugin;
use video_audio_emotion_detection::plugin::EmotionDetectionPlugin;
use video_audio_extractor::plugin::AudioExtractionPlugin;
use video_audio_face_detection::plugin::FaceDetectionPlugin;
use video_audio_image_quality::plugin::ImageQualityPlugin;
use video_audio_keyframe::plugin::KeyframePlugin;
use video_audio_logo_detection::plugin::LogoDetectionPlugin;
use video_audio_metadata::plugin::MetadataExtractionPlugin;
use video_audio_motion_tracking::plugin::MotionTrackingPlugin;
use video_audio_music_source_separation::plugin::MusicSourceSeparationPlugin;
use video_audio_object_detection::plugin::ObjectDetectionPlugin;
use video_audio_ocr::plugin::OCRPlugin;
use video_audio_pose_estimation::plugin::PoseEstimationPlugin;
use video_audio_profanity_detection::plugin::ProfanityDetectionPlugin;
use video_audio_scene::plugin::SceneDetectionPlugin;
use video_audio_smart_thumbnail::plugin::SmartThumbnailPlugin;
use video_audio_subtitle::plugin::SubtitleExtractionPlugin;
use video_audio_voice_activity_detection::plugin::VoiceActivityDetectionPlugin;
use video_extract_core::Registry;

/// Register all available plugins into the provided registry
///
/// This function registers all 32 plugins:
/// - audio_extraction
/// - transcription
/// - keyframes
/// - object_detection
/// - face_detection
/// - ocr
/// - diarization
/// - voice_activity_detection (Quick-Win Feature #1, N=268)
/// - acoustic_scene_classification (Quick-Win Feature #2, N=269)
/// - profanity_detection (Quick-Win Feature, N=270)
/// - scene_detection
/// - vision_embeddings
/// - text_embeddings
/// - audio_embeddings
/// - duplicate_detection (N=177)
/// - subtitle_extraction (Tier 1, N=72)
/// - audio_classification (Tier 1, N=73)
/// - smart_thumbnail (Tier 1, N=74)
/// - action_recognition (Tier 1, N=75)
/// - motion_tracking (Tier 1, N=77)
/// - pose_estimation (Tier 2, N=92)
/// - image_quality_assessment (Tier 2, N=93)
/// - emotion_detection (Tier 2, N=94-95)
/// - audio_enhancement_metadata (Tier 2, N=96)
/// - shot_classification (Tier 2, N=99)
/// - metadata_extraction (Tier 4, N=168)
/// - content_moderation (Tier 3, N=170)
/// - logo_detection (Tier 3, N=171)
/// - music_source_separation (Tier 3, N=172)
/// - depth_estimation (Tier 3, N=173)
/// - caption_generation (Tier 3, N=175)
/// - format_conversion (Tier 4, N=179)
pub fn register_all_plugins(registry: &mut Registry) -> Result<()> {
    // Register audio extraction plugin
    let audio_plugin = Arc::new(
        AudioExtractionPlugin::from_yaml("config/plugins/audio_extraction.yaml")
            .context("Failed to load audio extraction plugin")?,
    );
    registry.register(audio_plugin);

    // Register transcription plugin
    let transcription_plugin = Arc::new(
        TranscriptionPlugin::from_yaml("config/plugins/transcription.yaml")
            .context("Failed to load transcription plugin")?,
    );
    registry.register(transcription_plugin);

    // Register keyframe plugin
    let keyframe_plugin = Arc::new(
        KeyframePlugin::from_yaml("config/plugins/keyframes.yaml")
            .context("Failed to load keyframe plugin")?,
    );
    registry.register(keyframe_plugin);

    // Register ONNX plugins
    let object_detection_plugin = Arc::new(
        ObjectDetectionPlugin::from_yaml("config/plugins/object_detection.yaml")
            .context("Failed to load object detection plugin")?,
    );
    registry.register(object_detection_plugin);

    let face_detection_plugin = Arc::new(
        FaceDetectionPlugin::from_yaml("config/plugins/face_detection.yaml")
            .context("Failed to load face detection plugin")?,
    );
    registry.register(face_detection_plugin);

    let ocr_plugin = Arc::new(
        OCRPlugin::from_yaml("config/plugins/ocr.yaml").context("Failed to load OCR plugin")?,
    );
    registry.register(ocr_plugin);

    let diarization_plugin = Arc::new(
        DiarizationPlugin::from_yaml("config/plugins/diarization.yaml")
            .context("Failed to load diarization plugin")?,
    );
    registry.register(diarization_plugin);

    let vad_plugin = Arc::new(
        VoiceActivityDetectionPlugin::from_yaml("config/plugins/voice_activity_detection.yaml")
            .context("Failed to load voice activity detection plugin")?,
    );
    registry.register(vad_plugin);

    let scene_detection_plugin = Arc::new(
        SceneDetectionPlugin::from_yaml("config/plugins/scene_detection.yaml")
            .context("Failed to load scene detection plugin")?,
    );
    registry.register(scene_detection_plugin);

    // Register embeddings plugins (vision, text, audio)
    let vision_embeddings_plugin = Arc::new(
        EmbeddingsPlugin::from_yaml("config/plugins/vision_embeddings.yaml")
            .context("Failed to load vision embeddings plugin")?,
    );
    registry.register(vision_embeddings_plugin);

    let text_embeddings_plugin = Arc::new(
        EmbeddingsPlugin::from_yaml("config/plugins/text_embeddings.yaml")
            .context("Failed to load text embeddings plugin")?,
    );
    registry.register(text_embeddings_plugin);

    let audio_embeddings_plugin = Arc::new(
        EmbeddingsPlugin::from_yaml("config/plugins/audio_embeddings.yaml")
            .context("Failed to load audio embeddings plugin")?,
    );
    registry.register(audio_embeddings_plugin);

    // Register Tier 1 feature plugins (N=72-77, integrated N=80)
    let subtitle_plugin = Arc::new(
        SubtitleExtractionPlugin::from_yaml("config/plugins/subtitle_extraction.yaml")
            .context("Failed to load subtitle extraction plugin")?,
    );
    registry.register(subtitle_plugin);

    let audio_classification_plugin = Arc::new(
        AudioClassificationPlugin::from_yaml("config/plugins/audio_classification.yaml")
            .context("Failed to load audio classification plugin")?,
    );
    registry.register(audio_classification_plugin);

    let acoustic_scene_plugin = Arc::new(
        AcousticSceneClassificationPlugin::from_yaml(
            "config/plugins/acoustic_scene_classification.yaml",
        )
        .context("Failed to load acoustic scene classification plugin")?,
    );
    registry.register(acoustic_scene_plugin);

    let profanity_detection_plugin = Arc::new(
        ProfanityDetectionPlugin::from_yaml("config/plugins/profanity_detection.yaml")
            .context("Failed to load profanity detection plugin")?,
    );
    registry.register(profanity_detection_plugin);

    let duplicate_detection_plugin = Arc::new(
        DuplicateDetectionPlugin::from_yaml("config/plugins/duplicate_detection.yaml")
            .context("Failed to load duplicate detection plugin")?,
    );
    registry.register(duplicate_detection_plugin);

    let smart_thumbnail_plugin = Arc::new(
        SmartThumbnailPlugin::from_yaml("config/plugins/smart_thumbnail.yaml")
            .context("Failed to load smart thumbnail plugin")?,
    );
    registry.register(smart_thumbnail_plugin);

    let action_recognition_plugin = Arc::new(
        ActionRecognitionPlugin::from_yaml("config/plugins/action_recognition.yaml")
            .context("Failed to load action recognition plugin")?,
    );
    registry.register(action_recognition_plugin);

    let motion_tracking_plugin = Arc::new(
        MotionTrackingPlugin::from_yaml("config/plugins/motion_tracking.yaml")
            .context("Failed to load motion tracking plugin")?,
    );
    registry.register(motion_tracking_plugin);

    // Register Tier 2 feature plugins (N=92-94)
    let pose_estimation_plugin = Arc::new(
        PoseEstimationPlugin::from_yaml("config/plugins/pose_estimation.yaml")
            .context("Failed to load pose estimation plugin")?,
    );
    registry.register(pose_estimation_plugin);

    let image_quality_plugin = Arc::new(
        ImageQualityPlugin::from_yaml("config/plugins/image_quality_assessment.yaml")
            .context("Failed to load image quality assessment plugin")?,
    );
    registry.register(image_quality_plugin);

    let emotion_detection_plugin = Arc::new(
        EmotionDetectionPlugin::from_yaml("config/plugins/emotion_detection.yaml")
            .context("Failed to load emotion detection plugin")?,
    );
    registry.register(emotion_detection_plugin);

    let audio_enhancement_metadata_plugin = Arc::new(
        AudioEnhancementMetadataPlugin::from_yaml("config/plugins/audio_enhancement_metadata.yaml")
            .context("Failed to load audio enhancement metadata plugin")?,
    );
    registry.register(audio_enhancement_metadata_plugin);

    let shot_classification_plugin = Arc::new(
        ShotClassificationPlugin::from_yaml("config/plugins/shot_classification.yaml")
            .context("Failed to load shot classification plugin")?,
    );
    registry.register(shot_classification_plugin);

    // Register metadata extraction plugin (Tier 4, N=168)
    let metadata_extraction_plugin = Arc::new(
        MetadataExtractionPlugin::from_yaml("config/plugins/metadata_extraction.yaml")
            .context("Failed to load metadata extraction plugin")?,
    );
    registry.register(metadata_extraction_plugin);

    // Register content moderation plugin (Tier 3, N=170)
    let content_moderation_plugin = Arc::new(
        ContentModerationPlugin::from_yaml("config/plugins/content_moderation.yaml")
            .context("Failed to load content moderation plugin")?,
    );
    registry.register(content_moderation_plugin);

    // Register logo detection plugin (Tier 3, N=171)
    let logo_detection_plugin = Arc::new(
        LogoDetectionPlugin::from_yaml("config/plugins/logo_detection.yaml")
            .context("Failed to load logo detection plugin")?,
    );
    registry.register(logo_detection_plugin);

    // Register music source separation plugin (Tier 3, N=172)
    let music_source_separation_plugin = Arc::new(
        MusicSourceSeparationPlugin::from_yaml("config/plugins/music_source_separation.yaml")
            .context("Failed to load music source separation plugin")?,
    );
    registry.register(music_source_separation_plugin);

    // Register depth estimation plugin (Tier 3, N=173)
    let depth_estimation_plugin = Arc::new(
        DepthEstimationPlugin::from_yaml("config/plugins/depth_estimation.yaml")
            .context("Failed to load depth estimation plugin")?,
    );
    registry.register(depth_estimation_plugin);

    // Register caption generation plugin (Tier 3, N=175)
    let caption_generation_plugin = Arc::new(
        CaptionGenerationPlugin::from_yaml("config/plugins/caption_generation.yaml")
            .context("Failed to load caption generation plugin")?,
    );
    registry.register(caption_generation_plugin);

    // Register format conversion plugin (Tier 4, N=179)
    let format_conversion_plugin = Arc::new(
        FormatConversionPlugin::from_yaml("config/plugins/format_conversion.yaml")
            .context("Failed to load format conversion plugin")?,
    );
    registry.register(format_conversion_plugin);

    Ok(())
}
