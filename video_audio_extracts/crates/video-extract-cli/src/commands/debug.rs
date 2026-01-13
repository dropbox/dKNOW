//! Debug mode command implementation

use super::registry_helper::register_all_plugins;
use crate::parser::parse_ops_string;
use anyhow::{Context as _, Result};
use clap::Args;
use std::path::{Path, PathBuf};
use tracing::info;
use video_extract_core::operation::{
    AudioModel, ObjectDetectionModel, PoseEstimationModel, TextModel, VadAggressiveness,
    VisionModel, WhisperModel,
};
use video_extract_core::plugin::PluginData;
use video_extract_core::{
    DebugExecutor, Operation, PerformanceExecutor, Pipeline, Registry, StreamingResult,
};

#[derive(Args)]
pub struct DebugCommand {
    /// Input media file path
    #[arg(value_name = "FILE")]
    input: PathBuf,

    /// Operations to perform
    /// Available: audio, transcription, keyframes, object-detection, face-detection, ocr, diarization, scene-detection, vision-embeddings, text-embeddings, audio-embeddings
    /// Sequential: --ops "audio,transcription" or --ops "audio;transcription"
    /// Parallel: --ops "[audio,keyframes]"
    /// Mixed: --ops "keyframes;[object-detection,ocr]"
    #[arg(short, long)]
    ops: String,

    /// Output directory for intermediate results
    #[arg(long, default_value = "./debug_output")]
    output_dir: PathBuf,

    /// Whisper model for transcription
    #[arg(long, default_value = "large-v3")]
    model: String,

    /// Audio sample rate (Hz)
    #[arg(long, default_value = "16000")]
    sample_rate: u32,

    /// Audio channels
    #[arg(long, default_value = "1")]
    channels: u8,

    /// Max keyframes to extract
    #[arg(long)]
    max_frames: Option<u32>,

    /// Minimum interval between keyframes (seconds)
    #[arg(long, default_value = "1.0")]
    min_interval: f32,
}

impl DebugCommand {
    pub async fn execute(self) -> Result<()> {
        info!("=== Video Extract Debug Mode ===");
        info!("Input: {}", self.input.display());
        info!("Operations: {}", self.ops);
        info!("Output directory: {}", self.output_dir.display());

        // Verify input file exists
        if !self.input.exists() {
            anyhow::bail!("Input file does not exist: {}", self.input.display());
        }

        // Validate file with ffprobe (detect corrupted/malformed files early)
        Self::validate_media_file(&self.input).await?;

        // Parse operations string (supports parallel syntax: "[a,b];[c,d]")
        let parsed_ops =
            parse_ops_string(&self.ops).context("Failed to parse operations string")?;

        // Create registry and register plugins
        let mut registry = Registry::new();
        self.register_plugins(&mut registry)?;

        // Build pipeline from operations
        let pipeline = self.build_pipeline(&registry, &parsed_ops)?;

        info!("Pipeline: {} stages", pipeline.stages.len());
        for (idx, stage) in pipeline.stages.iter().enumerate() {
            info!(
                "  Stage {}: {} ({} → {})",
                idx + 1,
                stage.plugin.name(),
                stage.input_type,
                stage.output_type
            );
        }

        // Detect if pipeline has parallel groups (multiple stages with same input type)
        let has_parallel_groups = parsed_ops.iter().any(|group| group.len() > 1);

        // Execute pipeline
        let initial_input = PluginData::FilePath(self.input.clone());
        let result = if has_parallel_groups {
            // Use PerformanceExecutor for parallel execution
            info!("Detected parallel groups - using PerformanceExecutor for parallel execution");
            let executor = PerformanceExecutor::new().with_cache(); // Enable in-memory result caching

            // Execute and collect results from streaming channel
            let mut rx = executor
                .execute_streaming(&pipeline, initial_input)
                .await
                .context("Pipeline execution failed")?;

            // Collect streaming results until final result
            let mut final_result = None;
            while let Some(stream_result) = rx.recv().await {
                match stream_result {
                    StreamingResult::Complete(stage_result) => {
                        info!(
                            "Completed: Stage {} ({}) - {:.2}s",
                            stage_result.stage_index + 1,
                            stage_result.plugin_name,
                            stage_result.duration.as_secs_f64()
                        );
                    }
                    StreamingResult::Final(exec_result) => {
                        final_result = Some(exec_result);
                        break;
                    }
                    StreamingResult::Partial { .. } => {
                        // Ignore partial results in debug mode
                    }
                }
            }

            final_result.context("No final result received from executor")?
        } else {
            // Use DebugExecutor for sequential execution with intermediate outputs
            let executor = DebugExecutor::new()
                .with_cache() // Enable in-memory result caching (2.77x speedup expected)
                .with_output_dir(self.output_dir.clone());

            executor
                .execute(&pipeline, initial_input)
                .await
                .context("Pipeline execution failed")?
        };

        // Display results
        info!("=== Execution Complete ===");
        info!("Total time: {:.2}s", result.total_duration.as_secs_f64());
        info!("Stages executed: {}", result.intermediates.len());

        for stage_result in &result.intermediates {
            info!(
                "  Stage {}: {} - {:.2}s",
                stage_result.stage_index + 1,
                stage_result.plugin_name,
                stage_result.duration.as_secs_f64()
            );
        }

        if !result.warnings.is_empty() {
            info!("Warnings: {}", result.warnings.len());
            for warning in &result.warnings {
                info!("  - {}", warning);
            }
        }

        // Display final output
        match &result.output {
            PluginData::Json(value) => {
                info!("Final output (JSON):");
                println!("{}", serde_json::to_string_pretty(value)?);
            }
            PluginData::FilePath(path) => {
                info!("Final output: {}", path.display());
            }
            PluginData::Bytes(bytes) => {
                info!("Final output: {} bytes", bytes.len());
            }
            PluginData::Multiple(items) => {
                info!("Final output: {} items", items.len());
            }
        }

        info!(
            "Intermediate results saved to: {}",
            self.output_dir.display()
        );

        Ok(())
    }

    fn register_plugins(&self, registry: &mut Registry) -> Result<()> {
        info!("Registering plugins...");

        // Register all plugins using shared helper
        register_all_plugins(registry)?;

        // Log registered plugins
        for plugin_name in registry.plugin_names() {
            info!("  ✓ {}", plugin_name);
        }

        Ok(())
    }

    fn build_pipeline(&self, registry: &Registry, parsed_ops: &[Vec<String>]) -> Result<Pipeline> {
        info!("Building pipeline...");

        if parsed_ops.is_empty() {
            anyhow::bail!("No operations specified. Use --ops to specify operations.");
        }

        // Determine input format from file extension
        let input_format = self
            .input
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| anyhow::anyhow!("Could not determine file extension"))?
            .to_string();

        info!("Input format: {}", input_format);

        let mut stages = Vec::with_capacity(parsed_ops.len());
        let mut current_input_type = input_format.clone();

        // Build stages supporting parallel groups
        // All operations in a parallel group get the SAME input type
        // PerformanceExecutor will detect parallelism via dependency analysis
        for (stage_idx, group) in parsed_ops.iter().enumerate() {
            if group.len() > 1 {
                info!(
                    "Stage {} has {} parallel operations (will run in parallel with PerformanceExecutor)",
                    stage_idx + 1,
                    group.len()
                );
            }

            // Save the input type for this group
            let group_input_type = current_input_type.clone();
            let mut group_output_types = Vec::with_capacity(group.len());

            for op_name in group {
                let operation = self.parse_operation(op_name)?;
                let output_type = operation.output_type_name().to_string();

                info!(
                    "Looking for plugin: {} -> {}",
                    group_input_type, output_type
                );

                // Find plugin that can do this transformation
                let plugin_name = match operation.output_type_name() {
                    "Audio" => "audio_extraction",
                    "Transcription" => "transcription",
                    "Keyframes" => "keyframes",
                    "ObjectDetection" => "object_detection",
                    "FaceDetection" => "face_detection",
                    "OCR" => "ocr",
                    "Diarization" => "diarization",
                    "VoiceActivityDetection" => "voice_activity_detection",
                    "SceneDetection" => "scene_detection",
                    "VisionEmbeddings" => "vision_embeddings",
                    "TextEmbeddings" => "text_embeddings",
                    "AudioEmbeddings" => "audio_embeddings",
                    "PoseEstimation" => "pose_estimation",
                    "ImageQualityAssessment" => "image_quality_assessment",
                    "EmotionDetection" => "emotion_detection",
                    "AudioEnhancementMetadata" => "audio_enhancement_metadata",
                    "MotionTracking" => "motion-tracking",
                    "ActionRecognition" => "action_recognition",
                    "SmartThumbnail" => "smart_thumbnail",
                    "SubtitleExtraction" => "subtitle_extraction",
                    "AudioClassification" => "audio_classification",
                    "AcousticSceneClassification" => "acoustic_scene_classification",
                    "ProfanityDetection" => "profanity_detection",
                    "DuplicateDetection" => "duplicate_detection",
                    "ShotClassification" => "shot_classification",
                    "Metadata" => "metadata_extraction",
                    "ContentModeration" => "content_moderation",
                    "LogoDetection" => "logo_detection",
                    "MusicSourceSeparation" => "music_source_separation",
                    "DepthEstimation" => "depth_estimation",
                    "CaptionGeneration" => "caption_generation",
                    "FormatConversion" => "format_conversion",
                    _ => anyhow::bail!("Unknown operation: {}", op_name),
                };

                let plugin = registry
                    .get_plugin(plugin_name)
                    .ok_or_else(|| anyhow::anyhow!("Plugin not found: {}", plugin_name))?;

                // Verify plugin supports the input type
                if !plugin.supports_input(&group_input_type) {
                    anyhow::bail!(
                        "Plugin {} does not support input type: {}",
                        plugin_name,
                        group_input_type
                    );
                }

                stages.push(video_extract_core::PipelineStage {
                    plugin,
                    input_type: group_input_type.clone(),
                    output_type: output_type.clone(),
                    operation: operation.clone(),
                });

                group_output_types.push(output_type);
            }

            // For next group, use the last output type from this group
            // This handles sequential pipelines correctly (single output per group)
            // For parallel groups, the last output type becomes the "primary" output
            // but PerformanceExecutor's output_map tracks all outputs by type
            if let Some(last_output) = group_output_types.last() {
                current_input_type = last_output.clone();
            }
        }

        Ok(Pipeline { stages })
    }

    fn parse_operation(&self, op_name: &str) -> Result<Operation> {
        match op_name.to_lowercase().as_str() {
            "audio" | "audio-extraction" => Ok(Operation::Audio {
                sample_rate: self.sample_rate,
                channels: self.channels,
            }),
            "transcription" => {
                let model = match self.model.as_str() {
                    "tiny" => WhisperModel::Tiny,
                    "base" => WhisperModel::Base,
                    "small" => WhisperModel::Small,
                    "medium" => WhisperModel::Medium,
                    "large" | "large-v3" => WhisperModel::Large,
                    _ => anyhow::bail!("Unknown whisper model: {}", self.model),
                };
                Ok(Operation::Transcription {
                    language: None,
                    model,
                })
            }
            "keyframes" => Ok(Operation::Keyframes {
                max_frames: self.max_frames,
                min_interval_sec: self.min_interval,
            }),
            "object-detection" => Ok(Operation::ObjectDetection {
                model: ObjectDetectionModel::YoloV8x,
                confidence_threshold: 0.3,
                classes: None,
            }),
            "face-detection" => Ok(Operation::FaceDetection {
                min_size: 20,
                include_landmarks: true,
            }),
            "ocr" => Ok(Operation::OCR {
                languages: vec!["en".to_string()],
            }),
            "diarization" => Ok(Operation::Diarization { num_speakers: None }),
            "voice-activity-detection" | "vad" => Ok(Operation::VoiceActivityDetection {
                aggressiveness: VadAggressiveness::Aggressive,
                min_segment_duration: 0.3,
            }),
            "scene-detection" => Ok(Operation::SceneDetection {
                threshold: 0.4,
                keyframes_only: true,
            }),
            "vision-embeddings" => Ok(Operation::VisionEmbeddings {
                model: VisionModel::ClipVitB32,
            }),
            "text-embeddings" => Ok(Operation::TextEmbeddings {
                model: TextModel::AllMiniLmL6V2,
            }),
            "audio-embeddings" => Ok(Operation::AudioEmbeddings {
                model: AudioModel::ClapHtsatFused,
            }),
            "pose-estimation" => Ok(Operation::PoseEstimation {
                model: PoseEstimationModel::YoloV8xPose,
                confidence_threshold: 0.5,
                keypoint_threshold: 0.5,
            }),
            "image-quality-assessment" => Ok(Operation::ImageQualityAssessment {
                include_distribution: false,
            }),
            "emotion-detection" => Ok(Operation::EmotionDetection {
                include_probabilities: true,
            }),
            "audio-enhancement-metadata" => Ok(Operation::AudioEnhancementMetadata {}),
            "motion-tracking" => Ok(Operation::MotionTracking {
                high_confidence_threshold: None,
                low_confidence_threshold: None,
                detection_threshold_high: None,
                detection_threshold_low: None,
                max_age: None,
                min_hits: None,
            }),
            "action-recognition" => Ok(Operation::ActionRecognition {
                min_segment_duration: None,
                confidence_threshold: None,
                scene_change_threshold: None,
            }),
            "smart-thumbnail" => Ok(Operation::SmartThumbnail {
                min_quality: None,
                preferred_resolution: None,
            }),
            "subtitle-extraction" => Ok(Operation::SubtitleExtraction {
                track_index: None,
                language: None,
            }),
            "audio-classification" => Ok(Operation::AudioClassification {
                confidence_threshold: None,
                top_k: None,
            }),
            "acoustic-scene-classification" | "scene-classification" | "acoustic-scene" => {
                Ok(Operation::AcousticSceneClassification {
                    confidence_threshold: None, // Uses default 0.2 from AcousticSceneConfig
                })
            }
            "profanity-detection" | "profanity" => Ok(Operation::ProfanityDetection {
                min_severity: video_extract_core::operation::ProfanitySeverity::Mild,
                context_words: 3,
            }),
            "duplicate-detection" | "duplicate" | "dedup" => Ok(Operation::DuplicateDetection {
                algorithm: video_extract_core::operation::DuplicateHashAlgorithm::Gradient,
                hash_size: 8,
                threshold: 0.9,
            }),
            "shot-classification" => Ok(Operation::ShotClassification {}),
            "metadata" | "metadata-extraction" => Ok(Operation::Metadata {
                include_streams: true,
            }),
            "content-moderation" | "nsfw-detection" => Ok(Operation::ContentModeration {
                include_categories: false,
                nsfw_threshold: 0.5,
            }),
            "logo-detection" | "brand-detection" => Ok(Operation::LogoDetection {
                confidence_threshold: 0.50,
                logo_classes: None,
            }),
            "depth-estimation" | "depth" => Ok(Operation::DepthEstimation {
                input_size: 256,
                normalize: true,
                resize_to_original: false,
            }),
            "music-source-separation" | "music-separation" => Ok(Operation::MusicSourceSeparation {
                stems: None,
            }),
            "caption-generation" | "caption" | "image-caption" => Ok(Operation::CaptionGeneration {
                max_length: 50,
                use_beam_search: false,
                num_beams: 1,
            }),
            name if name.starts_with("format-conversion")
                || name.starts_with("transcode")
                || name.starts_with("convert") =>
            {
                // Parse format conversion parameters from operation spec
                // Example: format-conversion:container=mp4:video_codec=h264:audio_codec=aac:crf=23
                let parts: Vec<&str> = op_name.split(':').collect();
                let mut params: std::collections::HashMap<&str, &str> =
                    std::collections::HashMap::with_capacity(parts.len());
                for part in parts.iter().skip(1) {
                    if let Some((key, value)) = part.split_once('=') {
                        params.insert(key, value);
                    }
                }

                Ok(Operation::FormatConversion {
                    preset: params.get("preset").map(|&s| s.to_string()),
                    video_codec: params.get("video_codec").map(|&s| s.to_string()),
                    audio_codec: params.get("audio_codec").map(|&s| s.to_string()),
                    container: params.get("container").map(|&s| s.to_string()),
                    video_bitrate: params.get("video_bitrate").map(|&s| s.to_string()),
                    audio_bitrate: params.get("audio_bitrate").map(|&s| s.to_string()),
                    width: params.get("width").and_then(|s| s.parse().ok()),
                    height: params.get("height").and_then(|s| s.parse().ok()),
                    crf: params.get("crf").and_then(|s| s.parse().ok()),
                    output_file: params.get("output").map(|&s| s.to_string()),
                })
            }
            _ => anyhow::bail!("Unknown operation: {}", op_name),
        }
    }

    /// Validate media file with ffprobe before processing
    /// Detects corrupted or malformed files that would cause FFmpeg to hang
    /// Get the appropriate timeout command for the current platform
    fn get_timeout_command() -> &'static str {
        // Check for gtimeout (GNU coreutils on macOS)
        if std::process::Command::new("gtimeout")
            .arg("--version")
            .output()
            .is_ok()
        {
            return "gtimeout";
        }
        // Check for timeout (Linux, or GNU coreutils in PATH on macOS)
        if std::process::Command::new("timeout")
            .arg("--version")
            .output()
            .is_ok()
        {
            return "timeout";
        }
        // Fallback - will fail later with clear error if neither exists
        "timeout"
    }

    ///
    /// Uses system `timeout` command to enforce hard timeout at OS level
    /// (tokio timeout cannot interrupt blocking system calls in ffprobe's C code)
    async fn validate_media_file(path: &Path) -> Result<()> {
        // Check if file is a RAW camera format
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();

        let is_raw = matches!(
            extension.as_str(),
            "arw" | "cr2" | "dng" | "nef" | "raf" | "rw2" | "orf" | "pef" | "dcr" | "x3f"
        );

        if is_raw {
            // Validate RAW files with dcraw instead of ffprobe
            info!("Validating RAW camera file with dcraw...");
            let path_owned = path.to_owned();
            let output = tokio::task::spawn_blocking(move || {
                std::process::Command::new("dcraw")
                    .arg("-i") // Identify file (fast, no decode)
                    .arg(&path_owned)
                    .output()
            })
            .await??;

            if output.status.success() {
                info!("RAW file validation passed");
                Ok(())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!(
                    "RAW file validation failed: {}\n\
                     This file may be corrupted or in an unsupported RAW format.",
                    stderr.trim()
                );
            }
        } else {
            // Validate video/audio files with ffprobe
            info!("Validating media file with ffprobe...");

            // Use system timeout command to enforce hard limit
            // This is the only way to reliably timeout ffprobe when it hangs on corrupted files
            let path_owned = path.to_owned();
            let timeout_cmd = Self::get_timeout_command();
            let output_result = tokio::task::spawn_blocking(move || {
                std::process::Command::new(timeout_cmd)
                    .arg("10") // 10 second timeout
                    .arg("ffprobe")
                    .arg("-v")
                    .arg("error")
                    .arg("-show_format")
                    .arg("-show_streams")
                    .arg(&path_owned)
                    .output()
            })
            .await;

            match output_result {
                Ok(Ok(output)) => {
                    // timeout command exit codes:
                    // 0 = success
                    // 124 = timeout expired
                    // 125-127 = other errors
                    match output.status.code() {
                        Some(0) => {
                            info!("File validation passed");
                            Ok(())
                        }
                        Some(124) => {
                            anyhow::bail!(
                                "File validation timed out after 10 seconds.\n\
                                 This usually indicates a corrupted or malformed file that causes ffprobe to hang.\n\
                                 File: {}",
                                path.display()
                            );
                        }
                        Some(code) => {
                            let stderr = String::from_utf8_lossy(&output.stderr);
                            anyhow::bail!(
                                "File validation failed (exit code: {}): {}\n\
                                 This file may be corrupted or in an unsupported format.",
                                code,
                                stderr.trim()
                            );
                        }
                        None => {
                            anyhow::bail!("File validation process was terminated by signal");
                        }
                    }
                }
                Ok(Err(e)) => {
                    anyhow::bail!("Failed to run validation command: {}", e);
                }
                Err(e) => {
                    anyhow::bail!("Failed to spawn validation task: {}", e);
                }
            }
        }
    }
}
