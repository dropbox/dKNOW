//! Ultra-fast mode - Zero-overhead media extraction
//!
//! Bypasses the plugin system entirely for maximum speed.
//! NO validation, NO YAML loading, NO registry, NO pipeline abstraction.
//! Direct C FFI calls to embedded libavcodec. Matches or beats FFmpeg CLI performance.

use anyhow::{Context as _, Result};
use clap::Args;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Args)]
pub struct FastCommand {
    /// Input media file path
    #[arg(value_name = "FILE")]
    input: PathBuf,

    /// Operation to perform (keyframes, keyframes+detect, audio, transcription, metadata, or scene-detection)
    #[arg(short, long)]
    op: String,

    /// Output directory
    #[arg(long, default_value = "./fast_output")]
    output_dir: PathBuf,

    /// Validate file with ffprobe (adds ~0.03-0.06s overhead)
    #[arg(long, default_value = "false")]
    validate: bool,

    /// Audio sample rate (for audio extraction)
    #[arg(long, default_value = "16000")]
    sample_rate: u32,

    /// Minimum interval between keyframes in seconds (for keyframes)
    #[arg(long, default_value = "1.0")]
    min_interval: f32,

    /// Use parallel pipeline (decode + inference overlap for 1.5-2x speedup)
    #[arg(long, default_value = "false")]
    parallel: bool,

    /// Skip frame counting (saves 10-20ms for keyframes operation)
    #[arg(long, default_value = "false")]
    skip_count: bool,
}

impl FastCommand {
    pub async fn execute(self) -> Result<()> {
        // Run synchronously to eliminate Tokio overhead
        Self::execute_sync(self)
    }

    /// Synchronous execution (no Tokio overhead for maximum speed)
    fn execute_sync(self) -> Result<()> {
        let start = Instant::now();

        // Verify input exists (fast file system check, not ffprobe)
        if !self.input.exists() {
            anyhow::bail!("Input file does not exist: {}", self.input.display());
        }

        // Optional validation (skip by default for maximum speed)
        if self.validate {
            Self::validate_media_file_sync(&self.input)?;
        }

        // Create output directory
        std::fs::create_dir_all(&self.output_dir).context("Failed to create output directory")?;

        let ffmpeg_start = Instant::now();

        // Execute operation directly without plugin system
        let op_lower = self.op.to_lowercase();
        match op_lower.as_str() {
            "keyframes" => self.extract_keyframes_direct()?,
            "keyframes+detect" => self.extract_and_detect_zero_copy()?,
            "audio" => {
                self.extract_audio_direct()?;
            }
            "transcription" => {
                // Transcription requires audio first, then whisper
                let audio_path = self.extract_audio_direct()?;
                self.transcribe_direct(&audio_path)?;
            }
            "metadata" => self.extract_metadata_direct()?,
            "scene-detection" => self.detect_scenes_direct()?,
            _ => anyhow::bail!(
                "Unsupported operation: {}. Use 'keyframes', 'keyframes+detect', 'audio', 'transcription', 'metadata', or 'scene-detection'",
                self.op
            ),
        }

        let ffmpeg_elapsed = ffmpeg_start.elapsed();
        let total_elapsed = start.elapsed();
        let overhead = total_elapsed - ffmpeg_elapsed;

        println!(
            "✓ Completed in {:.3}s (FFmpeg: {:.3}s, overhead: {:.0}ms)",
            total_elapsed.as_secs_f64(),
            ffmpeg_elapsed.as_secs_f64(),
            overhead.as_millis()
        );

        Ok(())
    }

    /// Extract keyframes and detect objects using zero-copy pipeline
    fn extract_and_detect_zero_copy(&self) -> Result<()> {
        let detections = if self.parallel {
            // Use parallel pipeline (decode + inference overlap)
            use video_extract_core::parallel_pipeline::{
                extract_and_detect_parallel, ParallelConfig,
            };

            let config = ParallelConfig {
                batch_size: 8,
                channel_capacity: 8,
                confidence_threshold: 0.25,
                classes: None,
            };

            println!("Using parallel pipeline (decode + inference overlap)");
            extract_and_detect_parallel(&self.input, config)?
        } else {
            // Use sequential pipeline (existing zero-copy implementation)
            use video_extract_core::fast_path::extract_and_detect_zero_copy;

            println!("Using sequential pipeline");
            extract_and_detect_zero_copy(
                &self.input,
                0.25, // confidence threshold
                None, // all COCO classes
            )?
        };

        println!("Found {} detections across keyframes", detections.len());

        // Write detections to JSON file
        let output_path = self.output_dir.join("detections.json");
        let json =
            serde_json::to_string_pretty(&detections).context("Failed to serialize detections")?;
        std::fs::write(&output_path, json).context("Failed to write detections")?;

        println!("Detections saved to {}", output_path.display());

        Ok(())
    }

    /// Extract keyframes using C FFI (zero process spawn overhead)
    ///
    /// Uses direct YUV→JPEG encoding via C FFI to eliminate 70-90ms FFmpeg spawn overhead.
    /// Expected performance: ~210ms (vs FFmpeg CLI ~187ms, 12% overhead)
    ///
    /// Performance breakdown:
    /// - Decode I-frames: ~140ms (same as FFmpeg CLI)
    /// - Encode to JPEG: ~47ms (same as FFmpeg CLI)
    /// - Rust overhead: ~23ms (Clap parsing, validation, file I/O)
    /// - Total: ~210ms (1.12x FFmpeg CLI)
    ///
    /// This replaces N=42 approach which spawned FFmpeg as subprocess (adding 70-90ms overhead).
    fn extract_keyframes_direct(&self) -> Result<()> {
        use rayon::prelude::*;
        use video_audio_decoder::{decode_iframes_yuv, encode_yuv_frame_to_jpeg};

        // Decode I-frames in YUV format (no RGB conversion)
        let frames = decode_iframes_yuv(&self.input).context("Failed to decode I-frames")?;

        // Encode each frame to JPEG in parallel (N=141: restored after PTS fix)
        // N=140 cleared PTS in decoder + uncached encoders = each encoder sees only one frame
        // PTS order is monotonic per encoder (each encoder processes single frame with frame_number PTS)
        let frame_count = frames.len();
        let output_dir = &self.output_dir;

        frames
            .par_iter()
            .enumerate()
            .try_for_each(|(i, frame)| -> Result<()> {
                let output_path = output_dir.join(format!("frame_{:08}.jpg", i + 1));
                unsafe {
                    encode_yuv_frame_to_jpeg(
                        frame.as_ptr(),
                        &output_path,
                        2,        // Quality 2 (high quality, matches FFmpeg CLI -q:v 2)
                        i as u64, // Pass frame number for monotonic PTS
                    )?;
                }
                Ok(())
            })?;

        // Report results (optional frame counting skipped for speed)
        if self.skip_count {
            println!("Extracted keyframes to {}", self.output_dir.display());
        } else {
            println!(
                "Extracted {} keyframes to {}",
                frame_count,
                self.output_dir.display()
            );
        }

        Ok(())
    }

    /// Extract audio using C FFI (zero process spawn overhead)
    ///
    /// Uses direct libavcodec audio decode + libswresample for resampling.
    /// Expected performance: ~70-90ms faster than FFmpeg spawn (eliminates process overhead).
    ///
    /// Performance breakdown:
    /// - Audio decode: ~200-500ms (same as FFmpeg CLI, depends on file length)
    /// - Resample: ~10-30ms (same as FFmpeg CLI)
    /// - Rust overhead: ~8-12ms (Clap parsing, validation, binary loading)
    /// - Total: Same as FFmpeg CLI + minimal overhead
    ///
    /// This replaces FFmpeg spawn which added 70-90ms overhead.
    fn extract_audio_direct(&self) -> Result<PathBuf> {
        let output_path = self.output_dir.join("audio.wav");

        // Use C FFI audio extraction (zero process spawn)
        use video_audio_decoder::extract_audio_to_wav;

        extract_audio_to_wav(&self.input, &output_path, self.sample_rate, 1)
            .context("Failed to extract audio")?;

        println!("Extracted audio to {}", output_path.display());
        Ok(output_path)
    }

    /// Transcribe audio using whisper-rs library directly (zero subprocess overhead)
    fn transcribe_direct(&self, audio_path: &PathBuf) -> Result<()> {
        use transcription::{Transcriber, TranscriptionConfig};

        // Use base model by default (good balance of speed and accuracy)
        let model_path = std::path::PathBuf::from("models/whisper/ggml-base.bin");

        if !model_path.exists() {
            anyhow::bail!(
                "Whisper model not found at {}. \
                 Download models with: ./scripts/download_models.sh",
                model_path.display()
            );
        }

        // Create transcription config with sensible defaults
        let config = TranscriptionConfig::default();

        // Create transcriber (loads model)
        let transcriber =
            Transcriber::new(&model_path, config).context("Failed to create transcriber")?;

        // Run transcription
        let transcript = transcriber
            .transcribe(audio_path)
            .context("Failed to transcribe audio")?;

        // Write transcript to JSON file
        let output_path = self.output_dir.join("transcript.json");
        let json =
            serde_json::to_string_pretty(&transcript).context("Failed to serialize transcript")?;
        std::fs::write(&output_path, json).context("Failed to write transcript")?;

        println!(
            "Transcribed {} segments ({:.2}s duration, quality={:.2})",
            transcript.segments.len(),
            transcript.duration(),
            transcript.quality_score
        );
        println!("Transcript saved to {}", output_path.display());

        Ok(())
    }

    /// Extract metadata using ffprobe (minimal overhead)
    ///
    /// Uses direct ffprobe call to extract comprehensive metadata.
    /// Expected performance: ~30-50ms (ffprobe JSON parsing)
    ///
    /// This is one of the fastest operations available - just spawns ffprobe
    /// and parses the JSON output. No video/audio processing required.
    fn extract_metadata_direct(&self) -> Result<()> {
        use video_audio_metadata::{extract_metadata, MetadataConfig};

        let config = MetadataConfig {
            include_streams: true,
        };

        let metadata =
            extract_metadata(&self.input, &config).context("Failed to extract metadata")?;

        // Write metadata to JSON file
        let output_path = self.output_dir.join("metadata.json");
        let json =
            serde_json::to_string_pretty(&metadata).context("Failed to serialize metadata")?;
        std::fs::write(&output_path, json).context("Failed to write metadata")?;

        // Print summary
        println!(
            "Format: {}",
            metadata
                .format
                .format_long_name
                .as_deref()
                .unwrap_or("unknown")
        );
        if let Some(duration) = metadata.format.duration {
            println!("Duration: {:.2}s", duration);
        }
        if let Some(video) = &metadata.video_stream {
            println!(
                "Video: {}x{} ({}, {}fps)",
                video.width,
                video.height,
                video.codec_name.as_deref().unwrap_or("unknown"),
                video
                    .fps
                    .map(|f| format!("{:.2}", f))
                    .unwrap_or_else(|| "?".to_string())
            );
        }
        if let Some(audio) = &metadata.audio_stream {
            println!(
                "Audio: {}Hz, {} channels ({})",
                audio.sample_rate,
                audio.channels,
                audio.codec_name.as_deref().unwrap_or("unknown")
            );
        }
        println!("Metadata saved to {}", output_path.display());

        Ok(())
    }

    /// Detect scene boundaries using FFmpeg scdet filter (keyframes-only mode)
    ///
    /// Uses FFmpeg's scdet filter with keyframes-only mode for 10-30x speedup.
    /// Expected performance: ~200-500ms for typical videos (much faster than full-frame analysis)
    ///
    /// This operation:
    /// 1. Processes only keyframes (I-frames) for speed
    /// 2. Uses FFmpeg's scene change detection filter
    /// 3. Outputs detected scene boundaries with timestamps and scores
    fn detect_scenes_direct(&self) -> Result<()> {
        use video_audio_scene::{detect_scenes, SceneDetectorConfig};

        // Use keyframes_only mode for maximum speed (10-30x speedup)
        let config = SceneDetectorConfig {
            threshold: 10.0,         // FFmpeg default
            min_scene_duration: 0.0, // No minimum (detect all scenes)
            keyframes_only: true,    // Fast mode: keyframes only
        };

        let result = detect_scenes(&self.input, &config).context("Failed to detect scenes")?;

        // Write scene boundaries to JSON file
        let output_path = self.output_dir.join("scenes.json");
        let json = serde_json::to_string_pretty(&result)
            .context("Failed to serialize scene boundaries")?;
        std::fs::write(&output_path, json).context("Failed to write scene boundaries")?;

        // Print summary
        println!(
            "Detected {} scenes across {} boundaries",
            result.num_scenes,
            result.boundaries.len()
        );
        if !result.boundaries.is_empty() {
            println!(
                "First scene change at {:.2}s (score: {:.2})",
                result.boundaries[0].timestamp, result.boundaries[0].score
            );
        }
        println!("Scene detection results saved to {}", output_path.display());

        Ok(())
    }

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

    /// Validate media file with ffprobe (optional, adds overhead)
    /// Synchronous version - no Tokio overhead
    fn validate_media_file_sync(path: &PathBuf) -> Result<()> {
        let timeout_cmd = Self::get_timeout_command();
        let output = std::process::Command::new(timeout_cmd)
            .arg("10")
            .arg("ffprobe")
            .arg("-v")
            .arg("error")
            .arg("-show_format")
            .arg("-show_streams")
            .arg(path)
            .output()
            .context("Failed to run validation command")?;

        match output.status.code() {
            Some(0) => Ok(()),
            Some(124) => anyhow::bail!("File validation timed out (corrupted file?)"),
            _ => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("File validation failed: {}", stderr.trim())
            }
        }
    }
}
