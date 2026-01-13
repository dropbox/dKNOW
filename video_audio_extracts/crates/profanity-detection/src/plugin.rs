//! Plugin wrapper for profanity detection module

use crate::{ProfanityConfig, ProfanityDetector, ProfanityMatch, Severity};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;
use tracing::{debug, info};
use video_extract_core::operation::ProfanitySeverity;
use video_extract_core::plugin::PluginData;
use video_extract_core::{
    Context, Operation, Plugin, PluginConfig, PluginError, PluginRequest, PluginResponse,
};

/// Profanity detection results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfanityResults {
    /// Total number of profane words detected
    pub total_matches: usize,
    /// Matches grouped by severity
    pub by_severity: HashMap<Severity, Vec<ProfanityMatch>>,
    /// All detected profanity matches
    pub matches: Vec<ProfanityMatch>,
    /// Profanity rate (matches per minute)
    pub profanity_rate: f64,
    /// Most severe level detected
    pub max_severity: Option<Severity>,
}

impl ProfanityResults {
    /// Create results from detected matches
    pub fn from_matches(matches: Vec<ProfanityMatch>, duration_seconds: Option<f64>) -> Self {
        let total_matches = matches.len();

        // Group by severity (max 4 levels: Mild, Moderate, Strong, Severe)
        let mut by_severity: HashMap<Severity, Vec<ProfanityMatch>> = HashMap::with_capacity(4);
        let mut max_severity: Option<Severity> = None;

        for m in &matches {
            by_severity.entry(m.severity).or_default().push(m.clone());

            // Track maximum severity
            max_severity = match max_severity {
                None => Some(m.severity),
                Some(current) => {
                    if Self::severity_level(m.severity) > Self::severity_level(current) {
                        Some(m.severity)
                    } else {
                        Some(current)
                    }
                }
            };
        }

        // Calculate profanity rate (per minute)
        let profanity_rate = if let Some(duration) = duration_seconds {
            let minutes = duration / 60.0;
            if minutes > 0.0 {
                total_matches as f64 / minutes
            } else {
                0.0
            }
        } else {
            0.0
        };

        Self {
            total_matches,
            by_severity,
            matches,
            profanity_rate,
            max_severity,
        }
    }

    fn severity_level(severity: Severity) -> u8 {
        match severity {
            Severity::Mild => 1,
            Severity::Moderate => 2,
            Severity::Strong => 3,
            Severity::Severe => 4,
        }
    }
}

/// Profanity Detection plugin implementation
pub struct ProfanityDetectionPlugin {
    config: PluginConfig,
}

impl ProfanityDetectionPlugin {
    /// Create a new profanity detection plugin
    pub fn new(config: PluginConfig) -> Self {
        Self { config }
    }

    /// Load plugin from YAML configuration
    pub fn from_yaml(yaml_path: impl AsRef<Path>) -> Result<Self, PluginError> {
        let contents = std::fs::read_to_string(yaml_path.as_ref())?;
        let config: PluginConfig = serde_yaml::from_str(&contents)
            .map_err(|e| PluginError::ExecutionFailed(format!("Failed to parse YAML: {}", e)))?;

        Ok(Self::new(config))
    }

    /// Convert ProfanitySeverity (from Operation enum) to Severity (from library)
    fn convert_severity(sev: &ProfanitySeverity) -> Severity {
        match sev {
            ProfanitySeverity::Mild => Severity::Mild,
            ProfanitySeverity::Moderate => Severity::Moderate,
            ProfanitySeverity::Strong => Severity::Strong,
            ProfanitySeverity::Severe => Severity::Severe,
        }
    }
}

#[async_trait]
impl Plugin for ProfanityDetectionPlugin {
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
        let (min_severity, context_words) = match &request.operation {
            Operation::ProfanityDetection {
                min_severity,
                context_words,
            } => (Self::convert_severity(min_severity), *context_words),
            _ => {
                return Err(PluginError::ExecutionFailed(
                    "Invalid operation for profanity detection plugin".to_string(),
                ));
            }
        };

        debug!(
            "Profanity detection: min_severity={:?}, context_words={}",
            min_severity, context_words
        );

        // Create detector with configured parameters
        let detector_config = ProfanityConfig {
            min_severity,
            word_boundaries: true,
            context_words,
        };
        let detector = ProfanityDetector::with_config(detector_config);

        // Extract text from Transcription JSON input
        let (text, duration_seconds) = match &request.input {
            PluginData::Json(json) => {
                // Extract full text from transcription
                let text = json
                    .get("text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        PluginError::InvalidInput(
                            "Transcription JSON must have a 'text' field".to_string(),
                        )
                    })?
                    .to_string();

                // Extract duration if available (for profanity rate calculation)
                let duration = json.get("duration").and_then(|v| v.as_f64());

                (text, duration)
            }
            _ => {
                return Err(PluginError::InvalidInput(
                    "ProfanityDetection only supports Transcription JSON input".to_string(),
                ));
            }
        };

        if ctx.verbose {
            info!(
                "Analyzing {} characters of transcribed text for profanity",
                text.len()
            );
        }

        // Detect profanity in the text
        let matches = detector.detect_in_text(&text);

        debug!("Found {} profanity matches", matches.len());

        // Create results with match details
        let results = ProfanityResults::from_matches(matches, duration_seconds);
        let elapsed = start.elapsed();

        if ctx.verbose {
            info!(
                "Profanity detection completed: {} matches (max severity: {:?}) in {:?}",
                results.total_matches, results.max_severity, elapsed
            );
        }

        Ok(PluginResponse {
            output: PluginData::Json(serde_json::to_value(results).unwrap()),
            duration: elapsed,
            warnings: Vec::new(),
        })
    }
}
