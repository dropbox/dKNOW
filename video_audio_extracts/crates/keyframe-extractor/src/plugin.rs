//! Plugin wrapper for keyframe extraction module

use crate::{extract_keyframes, KeyframeExtractor};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tracing::{debug, info};
use video_extract_core::plugin::PluginData;
use video_extract_core::{
    Context, Operation, Plugin, PluginConfig, PluginError, PluginRequest, PluginResponse,
};

/// Keyframe extraction plugin implementation
pub struct KeyframePlugin {
    config: PluginConfig,
    temp_dir: PathBuf,
}

impl KeyframePlugin {
    /// Create a new keyframe extraction plugin
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
        let temp_dir = PathBuf::from("/tmp/video-extract/keyframes");
        std::fs::create_dir_all(&temp_dir)?;

        Ok(Self::new(config, temp_dir))
    }

    /// Convert Operation::Keyframes parameters to KeyframeExtractor config
    fn build_extractor_config(
        max_frames: Option<u32>,
        min_interval_sec: f32,
        output_dir: PathBuf,
        input_path: &Path,
    ) -> KeyframeExtractor {
        // Use FFmpeg CLI for MXF/MOV/GXF files (N=61: MXF decoder issues, N=131: MOV frame 0 corruption, N=252: GXF decoder issues)
        // FFmpeg CLI method works correctly, while C FFI decoder has issues with these formats
        let use_ffmpeg_cli = input_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| {
                let ext_lower = ext.to_lowercase();
                ext_lower == "mxf" || ext_lower == "mov" || ext_lower == "gxf"
            })
            .unwrap_or(false);

        KeyframeExtractor {
            interval: f64::from(min_interval_sec),
            max_keyframes: max_frames.map(|f| f as usize).unwrap_or(500),
            similarity_threshold: 10,
            thumbnail_sizes: vec![(640, 480)], // Single resolution for speed
            output_dir,
            use_ffmpeg_cli, // Use FFmpeg CLI for MXF/MOV, C FFI decoder for others
        }
    }

    /// Generate output directory for keyframes
    fn output_dir(&self, input_path: &Path) -> PathBuf {
        let stem = input_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("video");
        self.temp_dir.join(stem).join("keyframes")
    }
}

#[async_trait]
impl Plugin for KeyframePlugin {
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
        let (max_frames, min_interval_sec) = match &request.operation {
            Operation::Keyframes {
                max_frames,
                min_interval_sec,
            } => (*max_frames, *min_interval_sec),
            _ => {
                return Err(PluginError::InvalidInput(
                    "Expected Keyframes operation".to_string(),
                ))
            }
        };

        if ctx.verbose {
            info!(
                "Extracting keyframes: max={:?}, interval={:.1}s",
                max_frames, min_interval_sec
            );
        }

        // Get input file path
        let input_path = match &request.input {
            PluginData::FilePath(path) => path.clone(),
            PluginData::Bytes(_) => {
                return Err(PluginError::UnsupportedFormat(
                    "Bytes input not yet supported, use file path".to_string(),
                ))
            }
            _ => {
                return Err(PluginError::InvalidInput(
                    "Expected file path or bytes".to_string(),
                ))
            }
        };

        debug!("Extracting keyframes from: {}", input_path.display());

        // Build extractor config
        let output_dir = self.output_dir(&input_path);
        let extractor_config =
            Self::build_extractor_config(max_frames, min_interval_sec, output_dir, &input_path);

        // Debug: Log which decoder path we're using
        if ctx.verbose {
            info!("Using FFmpeg CLI decoder: {}", extractor_config.use_ffmpeg_cli);
        }

        // Extract keyframes
        let keyframes = extract_keyframes(&input_path, extractor_config).map_err(|e| {
            PluginError::ExecutionFailed(format!("Keyframe extraction failed: {}", e))
        })?;

        let duration = start.elapsed();

        if ctx.verbose {
            info!(
                "Keyframe extraction complete in {:?}: {} keyframes extracted",
                duration,
                keyframes.len()
            );
        }

        // Serialize keyframes to JSON
        let json = serde_json::to_value(&keyframes).map_err(PluginError::Serialization)?;

        Ok(PluginResponse {
            output: PluginData::Json(json),
            duration,
            warnings: vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;
    use video_extract_core::plugin::{CacheConfig, PerformanceConfig, RuntimeConfig};

    fn create_test_config() -> PluginConfig {
        PluginConfig {
            name: "keyframes".to_string(),
            description: "Test keyframe extraction plugin".to_string(),
            inputs: vec!["mp4".to_string(), "mov".to_string()],
            outputs: vec!["Keyframes".to_string()],
            config: RuntimeConfig {
                max_file_size_mb: 10000,
                requires_gpu: false,
                experimental: false,
            },
            performance: PerformanceConfig {
                avg_processing_time_per_gb: "30s".to_string(),
                memory_per_file_mb: 512,
                supports_streaming: false,
            },
            cache: CacheConfig {
                enabled: true,
                version: 3,
                invalidate_before: SystemTime::UNIX_EPOCH,
            },
        }
    }

    #[test]
    fn test_plugin_creation() {
        let config = create_test_config();
        let plugin = KeyframePlugin::new(config, "/tmp");

        assert_eq!(plugin.name(), "keyframes");
        assert!(plugin.supports_input("mp4"));
        assert!(plugin.supports_input("mov"));
        assert!(plugin.produces_output("Keyframes"));
    }

    #[test]
    fn test_extractor_config_building() {
        let output_dir = PathBuf::from("/tmp/test");
        let input_path = PathBuf::from("/tmp/test.mp4");
        let config = KeyframePlugin::build_extractor_config(Some(100), 1.5, output_dir.clone(), &input_path);

        assert_eq!(config.max_keyframes, 100);
        assert!((config.interval - 1.5).abs() < 0.01);
        assert_eq!(config.output_dir, output_dir);
        assert!(!config.use_ffmpeg_cli); // MP4 should use C FFI decoder
    }

    #[test]
    fn test_mxf_uses_ffmpeg_cli() {
        let output_dir = PathBuf::from("/tmp/test");
        let input_path = PathBuf::from("/tmp/test.mxf");
        let config = KeyframePlugin::build_extractor_config(Some(100), 1.5, output_dir, &input_path);

        assert!(config.use_ffmpeg_cli); // MXF should use FFmpeg CLI
    }

    #[test]
    fn test_gxf_uses_ffmpeg_cli() {
        let output_dir = PathBuf::from("/tmp/test");
        let input_path = PathBuf::from("/tmp/test.gxf");
        let config = KeyframePlugin::build_extractor_config(Some(100), 1.5, output_dir, &input_path);

        assert!(config.use_ffmpeg_cli); // GXF should use FFmpeg CLI
    }
}
