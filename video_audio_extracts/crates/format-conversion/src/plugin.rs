//! Plugin wrapper for format conversion module

use crate::{convert_format, AudioCodec, Container, ConversionConfig, Preset, VideoCodec};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tracing::{debug, info};
use video_extract_core::plugin::PluginData;
use video_extract_core::{
    Context, Operation, Plugin, PluginConfig, PluginError, PluginRequest, PluginResponse,
};

/// Format conversion plugin implementation
pub struct FormatConversionPlugin {
    config: PluginConfig,
    temp_dir: PathBuf,
}

impl FormatConversionPlugin {
    /// Create a new format conversion plugin
    pub fn new(config: PluginConfig, temp_dir: impl AsRef<Path>) -> Self {
        Self {
            config,
            temp_dir: temp_dir.as_ref().to_path_buf(),
        }
    }

    /// Load plugin from YAML configuration
    pub fn from_yaml(yaml_path: impl AsRef<Path>) -> Result<Self, PluginError> {
        let contents = std::fs::read_to_string(yaml_path.as_ref())?;
        let config: PluginConfig = serde_yaml::from_str(&contents)
            .map_err(|e| PluginError::ExecutionFailed(format!("Failed to parse YAML: {}", e)))?;

        // Default temp directory
        let temp_dir = PathBuf::from("/tmp/video-extract/format-conversion");
        std::fs::create_dir_all(&temp_dir)?;

        Ok(Self::new(config, temp_dir))
    }

    /// Generate output filename for converted media
    fn output_filename(&self, input_path: &Path, extension: &str) -> PathBuf {
        let stem = input_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("converted");
        self.temp_dir.join(format!("{}.{}", stem, extension))
    }
}

#[async_trait]
impl Plugin for FormatConversionPlugin {
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
        let (
            preset,
            video_codec,
            audio_codec,
            container,
            video_bitrate,
            audio_bitrate,
            width,
            height,
            crf,
            output_file,
        ) = match &request.operation {
            Operation::FormatConversion {
                preset,
                video_codec,
                audio_codec,
                container,
                video_bitrate,
                audio_bitrate,
                width,
                height,
                crf,
                output_file,
            } => (
                preset.clone(),
                video_codec.clone(),
                audio_codec.clone(),
                container.clone(),
                video_bitrate.clone(),
                audio_bitrate.clone(),
                *width,
                *height,
                *crf,
                output_file.clone(),
            ),
            _ => {
                return Err(PluginError::InvalidInput(
                    "Expected FormatConversion operation".to_string(),
                ))
            }
        };

        if ctx.verbose {
            info!(
                "Converting format (video: {:?}, audio: {:?}, container: {:?})",
                video_codec, audio_codec, container
            );
        }

        // Get input file path
        let input_path = match &request.input {
            PluginData::FilePath(path) => path.clone(),
            PluginData::Bytes(_) => {
                return Err(PluginError::UnsupportedFormat(
                    "Bytes input not yet supported for format conversion".to_string(),
                ))
            }
            _ => {
                return Err(PluginError::InvalidInput(
                    "Expected file path or bytes".to_string(),
                ))
            }
        };

        debug!("Converting format from: {}", input_path.display());

        // Build conversion config from preset or individual parameters
        let conversion_config = if let Some(preset_str) = preset {
            // Parse preset and convert to config
            let preset = parse_preset(&preset_str)?;
            let mut config = preset.to_config();

            // Override preset config with any explicitly provided parameters
            if let Some(vc) = video_codec.as_ref() {
                config.video_codec = Some(parse_video_codec(vc)?);
            }
            if let Some(ac) = audio_codec.as_ref() {
                config.audio_codec = Some(parse_audio_codec(ac)?);
            }
            // Container override (only if explicitly provided)
            if let Some(c) = container.as_ref() {
                config.container = parse_container(c)?;
            }
            // Optional overrides
            if video_bitrate.is_some() {
                config.video_bitrate = video_bitrate;
            }
            if audio_bitrate.is_some() {
                config.audio_bitrate = audio_bitrate;
            }
            if width.is_some() {
                config.width = width;
            }
            if height.is_some() {
                config.height = height;
            }
            if crf.is_some() {
                config.crf = crf;
            }

            config
        } else {
            // No preset - build config from individual parameters
            let video_codec_enum = video_codec
                .as_ref()
                .map(|s| parse_video_codec(s))
                .transpose()?;
            let audio_codec_enum = audio_codec
                .as_ref()
                .map(|s| parse_audio_codec(s))
                .transpose()?;
            // Default to MP4 container if not specified (when no preset is used)
            let container_enum = if let Some(c) = container.as_ref() {
                parse_container(c)?
            } else {
                Container::Mp4
            };

            ConversionConfig {
                video_codec: video_codec_enum,
                audio_codec: audio_codec_enum,
                container: container_enum,
                video_bitrate,
                audio_bitrate,
                width,
                height,
                crf,
            }
        };

        // Determine output path
        let output_path = if let Some(output) = output_file {
            PathBuf::from(output)
        } else {
            // Generate default output path in temp directory based on container
            self.output_filename(&input_path, conversion_config.container.extension())
        };

        // Convert format using FFmpeg
        let result =
            convert_format(&input_path, &output_path, &conversion_config).map_err(|e| {
                PluginError::ExecutionFailed(format!("Format conversion failed: {}", e))
            })?;

        let duration = start.elapsed();

        if ctx.verbose {
            info!(
                "Format conversion complete in {:?} ({:.1}% size)",
                duration,
                result.compression_ratio * 100.0
            );
        }

        // Serialize result to JSON Value
        let result_value = serde_json::to_value(&result).map_err(|e| {
            PluginError::ExecutionFailed(format!("Failed to serialize result: {}", e))
        })?;

        Ok(PluginResponse {
            output: PluginData::Json(result_value),
            duration,
            warnings: vec![],
        })
    }
}

fn parse_video_codec(codec: &str) -> Result<VideoCodec, PluginError> {
    match codec.to_lowercase().as_str() {
        "h264" | "libx264" => Ok(VideoCodec::H264),
        "h265" | "hevc" | "libx265" => Ok(VideoCodec::H265),
        "vp9" | "libvpx-vp9" => Ok(VideoCodec::Vp9),
        "av1" | "libaom-av1" => Ok(VideoCodec::Av1),
        "copy" => Ok(VideoCodec::Copy),
        _ => Err(PluginError::UnsupportedFormat(format!(
            "Unsupported video codec: {}",
            codec
        ))),
    }
}

fn parse_audio_codec(codec: &str) -> Result<AudioCodec, PluginError> {
    match codec.to_lowercase().as_str() {
        "aac" => Ok(AudioCodec::Aac),
        "mp3" | "libmp3lame" => Ok(AudioCodec::Mp3),
        "opus" | "libopus" => Ok(AudioCodec::Opus),
        "flac" => Ok(AudioCodec::Flac),
        "copy" => Ok(AudioCodec::Copy),
        _ => Err(PluginError::UnsupportedFormat(format!(
            "Unsupported audio codec: {}",
            codec
        ))),
    }
}

fn parse_container(container: &str) -> Result<Container, PluginError> {
    match container.to_lowercase().as_str() {
        "mp4" => Ok(Container::Mp4),
        "mkv" => Ok(Container::Mkv),
        "webm" => Ok(Container::Webm),
        "mov" => Ok(Container::Mov),
        _ => Err(PluginError::UnsupportedFormat(format!(
            "Unsupported container: {}",
            container
        ))),
    }
}

fn parse_preset(preset: &str) -> Result<Preset, PluginError> {
    match preset.to_lowercase().as_str() {
        "web" => Ok(Preset::Web),
        "mobile" => Ok(Preset::Mobile),
        "archive" => Ok(Preset::Archive),
        "compatible" => Ok(Preset::Compatible),
        "webopen" | "web-open" => Ok(Preset::WebOpen),
        "lowbandwidth" | "low-bandwidth" => Ok(Preset::LowBandwidth),
        "audioonly" | "audio-only" => Ok(Preset::AudioOnly),
        "copy" => Ok(Preset::Copy),
        _ => Err(PluginError::UnsupportedFormat(format!(
            "Unsupported preset: {}. Available presets: web, mobile, archive, compatible, web-open, low-bandwidth, audio-only, copy",
            preset
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;
    use video_extract_core::plugin::{CacheConfig, PerformanceConfig, RuntimeConfig};

    #[test]
    fn test_plugin_creation() {
        let config = PluginConfig {
            name: "format-conversion".to_string(),
            description: "Convert media file formats".to_string(),
            inputs: vec!["MediaFile".to_string()],
            outputs: vec!["MediaFile".to_string()],
            config: RuntimeConfig {
                max_file_size_mb: 10000,
                requires_gpu: false,
                experimental: false,
            },
            performance: PerformanceConfig {
                avg_processing_time_per_gb: "60s".to_string(),
                memory_per_file_mb: 1024,
                supports_streaming: false,
            },
            cache: CacheConfig {
                enabled: false,
                version: 1,
                invalidate_before: SystemTime::UNIX_EPOCH,
            },
        };

        let plugin = FormatConversionPlugin::new(config, "/tmp/test");
        assert_eq!(plugin.name(), "format-conversion");
    }

    #[test]
    fn test_parse_video_codec() {
        assert!(matches!(
            parse_video_codec("h264").unwrap(),
            VideoCodec::H264
        ));
        assert!(matches!(
            parse_video_codec("libx264").unwrap(),
            VideoCodec::H264
        ));
        assert!(matches!(
            parse_video_codec("h265").unwrap(),
            VideoCodec::H265
        ));
        assert!(matches!(parse_video_codec("vp9").unwrap(), VideoCodec::Vp9));
        assert!(matches!(
            parse_video_codec("copy").unwrap(),
            VideoCodec::Copy
        ));
        assert!(parse_video_codec("invalid").is_err());
    }

    #[test]
    fn test_parse_audio_codec() {
        assert!(matches!(parse_audio_codec("aac").unwrap(), AudioCodec::Aac));
        assert!(matches!(parse_audio_codec("mp3").unwrap(), AudioCodec::Mp3));
        assert!(matches!(
            parse_audio_codec("opus").unwrap(),
            AudioCodec::Opus
        ));
        assert!(matches!(
            parse_audio_codec("copy").unwrap(),
            AudioCodec::Copy
        ));
        assert!(parse_audio_codec("invalid").is_err());
    }

    #[test]
    fn test_parse_container() {
        assert!(matches!(parse_container("mp4").unwrap(), Container::Mp4));
        assert!(matches!(parse_container("mkv").unwrap(), Container::Mkv));
        assert!(matches!(parse_container("webm").unwrap(), Container::Webm));
        assert!(parse_container("invalid").is_err());
    }
}
