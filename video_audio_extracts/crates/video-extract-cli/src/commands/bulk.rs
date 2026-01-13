//! Bulk mode command implementation - maximum throughput, parallel processing

use super::registry_helper::register_all_plugins;
use crate::parser::parse_ops_string;
use anyhow::{Context as _, Result};
use clap::Args;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tracing::{info, warn};
use video_extract_core::operation::{
    AudioModel, ObjectDetectionModel, TextModel, VadAggressiveness, VisionModel, WhisperModel,
};
use video_extract_core::{BulkExecutor, Operation, Pipeline, Registry};

#[derive(Args)]
pub struct BulkCommand {
    /// Input media file paths (multiple files or glob pattern)
    #[arg(value_name = "FILES", required = true)]
    inputs: Vec<PathBuf>,

    /// Operations to perform
    /// Available: audio, transcription, keyframes, object-detection, face-detection, ocr, diarization, scene-detection, vision-embeddings, text-embeddings, audio-embeddings
    /// Sequential: --ops "audio,transcription" or --ops "audio;transcription"
    /// Parallel: --ops "[audio,keyframes]"
    /// Mixed: --ops "keyframes;[object-detection,ocr]"
    #[arg(short, long)]
    ops: String,

    /// Output directory for results (optional)
    #[arg(long)]
    output_dir: Option<PathBuf>,

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

    /// Maximum concurrent files to process
    #[arg(long)]
    max_concurrent: Option<usize>,

    /// Output format: text (default) or jsonl (JSON lines)
    #[arg(long, default_value = "text")]
    format: String,
}

impl BulkCommand {
    pub async fn execute(self) -> Result<()> {
        info!("=== Video Extract Bulk Mode ===");
        info!("Total input files: {}", self.inputs.len());
        info!("Operations: {}", self.ops);

        // Verify input files exist
        let valid_inputs: Vec<PathBuf> = self
            .inputs
            .iter()
            .filter(|path| {
                if path.exists() {
                    true
                } else {
                    warn!("Skipping non-existent file: {}", path.display());
                    false
                }
            })
            .cloned()
            .collect();

        if valid_inputs.is_empty() {
            anyhow::bail!("No valid input files found");
        }

        info!("Valid input files: {}", valid_inputs.len());

        // Parse operations string (supports parallel syntax: "[a,b];[c,d]")
        let parsed_ops =
            parse_ops_string(&self.ops).context("Failed to parse operations string")?;

        // Create registry and register plugins
        let mut registry = Registry::new();
        self.register_plugins(&mut registry)?;

        // Build pipeline from operations
        let pipeline = self.build_pipeline(&registry, &parsed_ops)?;

        info!("Pipeline: {} stages", pipeline.stages.len());

        // Create executor
        let mut executor = BulkExecutor::new();
        if let Some(max_concurrent) = self.max_concurrent {
            executor = executor.with_max_concurrent_files(max_concurrent);
        }

        // Execute pipeline on all files
        let start_time = Instant::now();
        let mut rx = executor
            .execute_bulk(&pipeline, valid_inputs.clone())
            .await
            .context("Failed to start bulk execution")?;

        // Process results as they complete
        let output_jsonl = self.format == "jsonl";
        let completed = Arc::new(AtomicUsize::new(0));
        let failed = Arc::new(AtomicUsize::new(0));
        let total_files = valid_inputs.len();

        while let Some(result) = rx.recv().await {
            let completed_count = completed.fetch_add(1, Ordering::SeqCst) + 1;

            match result.result {
                Ok(exec_result) => {
                    if output_jsonl {
                        println!(
                            "{}",
                            serde_json::json!({
                                "type": "success",
                                "file": result.input_path.display().to_string(),
                                "processing_time_ms": result.processing_time.as_millis(),
                                "total_duration_ms": exec_result.total_duration.as_millis(),
                                "stages": exec_result.intermediates.len(),
                                "warnings": exec_result.warnings,
                            })
                        );
                    } else {
                        info!(
                            "✓ [{}/{}] {} - {:.2}s",
                            completed_count,
                            total_files,
                            result.input_path.display(),
                            result.processing_time.as_secs_f64()
                        );
                    }
                }
                Err(error) => {
                    failed.fetch_add(1, Ordering::SeqCst);
                    if output_jsonl {
                        println!(
                            "{}",
                            serde_json::json!({
                                "type": "error",
                                "file": result.input_path.display().to_string(),
                                "error": error,
                            })
                        );
                    } else {
                        warn!(
                            "✗ [{}/{}] {} - FAILED: {}",
                            completed_count,
                            total_files,
                            result.input_path.display(),
                            error
                        );
                    }
                }
            }
        }

        let total_time = start_time.elapsed();
        let completed_count = completed.load(Ordering::SeqCst);
        let failed_count = failed.load(Ordering::SeqCst);
        let throughput = completed_count as f64 / total_time.as_secs_f64();

        if output_jsonl {
            println!(
                "{}",
                serde_json::json!({
                    "type": "summary",
                    "total_files": total_files,
                    "completed": completed_count,
                    "failed": failed_count,
                    "total_time_s": total_time.as_secs_f64(),
                    "throughput_files_per_sec": throughput,
                })
            );
        } else {
            info!("=== Bulk Processing Complete ===");
            info!("Total files: {}", total_files);
            info!("Completed: {}", completed_count);
            info!("Failed: {}", failed_count);
            info!("Total time: {:.2}s", total_time.as_secs_f64());
            info!("Throughput: {:.2} files/sec", throughput);
        }

        Ok(())
    }

    fn register_plugins(&self, registry: &mut Registry) -> Result<()> {
        // Register all plugins using shared helper
        register_all_plugins(registry)
    }

    fn build_pipeline(&self, registry: &Registry, parsed_ops: &[Vec<String>]) -> Result<Pipeline> {
        if parsed_ops.is_empty() {
            anyhow::bail!("No operations specified. Use --ops to specify operations.");
        }

        let input_format = self.get_input_format()?;
        let mut stages = Vec::with_capacity(parsed_ops.len());
        let mut current_input_type = input_format.clone();

        // Build stages supporting parallel groups
        // All operations in a parallel group get the SAME input type
        // PerformanceExecutor will detect parallelism via dependency analysis
        for (stage_idx, group) in parsed_ops.iter().enumerate() {
            if group.len() > 1 {
                info!(
                    "Stage {} has {} parallel operations (will run in parallel)",
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
                    "Metadata" => "metadata_extraction",
                    "ContentModeration" => "content_moderation",
                    "LogoDetection" => "logo_detection",
                    "MusicSourceSeparation" => "music_source_separation",
                    "DepthEstimation" => "depth_estimation",
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

    fn get_input_format(&self) -> Result<String> {
        // For bulk mode, we determine format from first input file
        if self.inputs.is_empty() {
            anyhow::bail!("No input files specified");
        }

        let first_input = &self.inputs[0];
        let extension = first_input
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| anyhow::anyhow!("Could not determine file extension"))?;

        Ok(extension.to_lowercase())
    }

    fn parse_operation(&self, op_name: &str) -> Result<Operation> {
        match op_name.to_lowercase().as_str() {
            "audio" => Ok(Operation::Audio {
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
                    _ => anyhow::bail!("Invalid whisper model: {}", self.model),
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
            "vision-embeddings" => Ok(Operation::VisionEmbeddings {
                model: VisionModel::ClipVitB32,
            }),
            "text-embeddings" => Ok(Operation::TextEmbeddings {
                model: TextModel::AllMiniLmL6V2,
            }),
            "audio-embeddings" => Ok(Operation::AudioEmbeddings {
                model: AudioModel::ClapHtsatFused,
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
            "music-source-separation" | "stem-separation" => Ok(Operation::MusicSourceSeparation {
                stems: None, // Extract all stems by default
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
}
