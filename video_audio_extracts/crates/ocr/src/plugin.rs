//! Plugin wrapper for OCR module

use crate::{OCRConfig, OCRDetector};
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tracing::{debug, info};
use video_extract_core::image_io::load_image;
use video_extract_core::plugin::PluginData;
use video_extract_core::{
    Context, Operation, Plugin, PluginConfig, PluginError, PluginRequest, PluginResponse,
};

/// OCR plugin implementation using Tesseract
pub struct OCRPlugin {
    config: PluginConfig,
    ocr_config: OCRConfig,
}

impl OCRPlugin {
    /// Create a new OCR plugin with Tesseract configuration
    pub fn new(config: PluginConfig) -> Self {
        // Default Tesseract configuration (English, 50% confidence threshold)
        let ocr_config = OCRConfig::default();

        Self {
            config,
            ocr_config,
        }
    }

    /// Create a new OCR plugin with custom Tesseract configuration
    pub fn with_ocr_config(config: PluginConfig, ocr_config: OCRConfig) -> Self {
        Self {
            config,
            ocr_config,
        }
    }

    /// Load plugin from YAML configuration
    pub fn from_yaml(yaml_path: impl AsRef<Path>) -> Result<Self, PluginError> {
        let contents = std::fs::read_to_string(yaml_path.as_ref())?;
        let config: PluginConfig = serde_yaml::from_str(&contents)
            .map_err(|e| PluginError::ExecutionFailed(format!("Failed to parse YAML: {}", e)))?;

        Ok(Self::new(config))
    }
}

#[async_trait]
impl Plugin for OCRPlugin {
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

        let languages = match &request.operation {
            Operation::OCR { languages } => languages.clone(),
            _ => {
                return Err(PluginError::InvalidInput(
                    "Expected OCR operation".to_string(),
                ))
            }
        };

        if ctx.verbose {
            info!("Running OCR with Tesseract, languages: {:?}", languages);
        }

        // Handle two input types: FilePath (direct image) or Keyframes JSON
        let image_paths: Vec<PathBuf> = match &request.input {
            PluginData::FilePath(path) => vec![path.clone()],
            PluginData::Json(json) => {
                // Parse Keyframes JSON to extract image paths
                #[derive(serde::Deserialize)]
                struct Keyframe {
                    thumbnail_paths: HashMap<String, PathBuf>,
                }

                let keyframes: Vec<Keyframe> =
                    serde_json::from_value(json.clone()).map_err(|e| {
                        PluginError::InvalidInput(format!("Failed to parse Keyframes JSON: {}", e))
                    })?;

                if keyframes.is_empty() {
                    return Err(PluginError::InvalidInput(
                        "Keyframes JSON is empty".to_string(),
                    ));
                }

                // Extract first thumbnail path from each keyframe
                keyframes
                    .iter()
                    .enumerate()
                    .filter_map(|(i, kf)| {
                        kf.thumbnail_paths.values().next().map(|p| {
                            debug!("Keyframe {}: Using thumbnail: {}", i, p.display());
                            p.clone()
                        })
                    })
                    .collect()
            }
            _ => {
                return Err(PluginError::UnsupportedFormat(
                    "OCR requires FilePath or Keyframes JSON input".to_string(),
                ))
            }
        };

        if image_paths.is_empty() {
            return Err(PluginError::InvalidInput(
                "No valid image paths found in input".to_string(),
            ));
        }

        debug!("Running OCR on {} image(s)", image_paths.len());

        // Map requested languages to Tesseract language code
        // Default to English if no languages specified or unrecognized
        let tesseract_lang = if languages.is_empty() {
            "eng".to_string()
        } else {
            // Map common language names to Tesseract codes
            // TODO: Expand this mapping for more languages
            let lang_code = match languages[0].to_lowercase().as_str() {
                "english" | "en" | "eng" => "eng",
                "chinese" | "zh" | "chi_sim" => "chi_sim",
                "spanish" | "es" | "spa" => "spa",
                "french" | "fr" | "fra" => "fra",
                "german" | "de" | "deu" => "deu",
                "japanese" | "ja" | "jpn" => "jpn",
                "korean" | "ko" | "kor" => "kor",
                _ => "eng", // Default to English for unrecognized languages
            };
            lang_code.to_string()
        };

        let ocr_config = OCRConfig {
            language: tesseract_lang.clone(),
            ..self.ocr_config.clone()
        };

        info!(
            "Using Tesseract with language '{}', min_confidence: {}%",
            ocr_config.language, ocr_config.min_confidence
        );

        // Process all images
        let text_regions = tokio::task::spawn_blocking(move || {
            // Create OCR detector (validates Tesseract installation and language data)
            let detector = OCRDetector::new(ocr_config).map_err(|e| {
                PluginError::ExecutionFailed(format!("Failed to create OCR detector: {}", e))
            })?;

            // Process each image and aggregate results
            // Pre-allocate all_regions Vec (estimate ~5 text regions per image on average)
            let mut all_regions = Vec::with_capacity(image_paths.len() * 5);
            for (idx, img_path) in image_paths.iter().enumerate() {
                // Load image with optimized I/O (mozjpeg for JPEG, 3-5x faster)
                let img = load_image(img_path).map_err(|e| {
                    PluginError::ExecutionFailed(format!(
                        "Failed to load image {} ({}): {}",
                        idx,
                        img_path.display(),
                        e
                    ))
                })?;

                let regions = detector.detect_text(&img).map_err(|e| {
                    PluginError::ExecutionFailed(format!(
                        "OCR failed for image {} ({}): {}",
                        idx,
                        img_path.display(),
                        e
                    ))
                })?;

                all_regions.extend(regions);
            }

            Ok::<_, PluginError>(all_regions)
        })
        .await
        .map_err(|e| PluginError::ExecutionFailed(format!("Task join error: {}", e)))??;

        let duration = start.elapsed();

        if ctx.verbose {
            info!(
                "OCR complete in {:?}: {} text regions detected",
                duration,
                text_regions.len()
            );
        }

        let json = serde_json::to_value(&text_regions).map_err(PluginError::Serialization)?;

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
            name: "ocr".to_string(),
            description: "Test OCR plugin".to_string(),
            inputs: vec!["jpg".to_string(), "png".to_string()],
            outputs: vec!["OCR".to_string()],
            config: RuntimeConfig {
                max_file_size_mb: 50,
                requires_gpu: false,
                experimental: false,
            },
            performance: PerformanceConfig {
                avg_processing_time_per_gb: "90s".to_string(),
                memory_per_file_mb: 512,
                supports_streaming: false,
            },
            cache: CacheConfig {
                enabled: true,
                version: 1,
                invalidate_before: SystemTime::UNIX_EPOCH,
            },
        }
    }

    #[test]
    fn test_plugin_creation() {
        let config = create_test_config();
        let plugin = OCRPlugin::new(config);
        assert_eq!(plugin.name(), "ocr");
    }

    #[test]
    fn test_plugin_with_custom_ocr_config() {
        let plugin_config = create_test_config();
        let ocr_config = OCRConfig {
            language: "eng".to_string(),
            min_confidence: 60,
            ..OCRConfig::default()
        };
        let plugin = OCRPlugin::with_ocr_config(plugin_config, ocr_config);
        assert_eq!(plugin.name(), "ocr");
        assert_eq!(plugin.ocr_config.min_confidence, 60);
    }
}
