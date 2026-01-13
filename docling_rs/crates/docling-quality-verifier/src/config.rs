//! Configuration for quality verification

use serde::{Deserialize, Serialize};
use std::env;

/// Configuration for LLM quality verification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerificationConfig {
    /// `OpenAI` model to use (e.g., "gpt-4o-mini", "gpt-4o")
    pub model: String,

    /// Quality threshold (0.0-1.0)
    /// Reports with score below this threshold will be marked as failed
    pub quality_threshold: f64,

    /// Enable detailed diagnostics (includes LLM reasoning)
    pub detailed_diagnostics: bool,

    /// Maximum tokens for LLM response
    pub max_tokens: usize,
}

impl VerificationConfig {
    /// Create configuration from environment variables
    ///
    /// Environment variables:
    /// - `LLM_MODEL`: Model name (default: "gpt-4o-mini")
    /// - `LLM_QUALITY_THRESHOLD`: Threshold 0.0-1.0 (default: 0.85)
    /// - `LLM_DETAILED`: Enable detailed diagnostics (default: false)
    /// - `LLM_MAX_TOKENS`: Max tokens (default: 4096)
    #[must_use = "creates config from environment variables"]
    pub fn from_env() -> Self {
        let model = env::var("LLM_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());

        let quality_threshold = env::var("LLM_QUALITY_THRESHOLD")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.85);

        let detailed_diagnostics = env::var("LLM_DETAILED")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(false);

        let max_tokens = env::var("LLM_MAX_TOKENS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(4096);

        Self {
            model,
            quality_threshold,
            detailed_diagnostics,
            max_tokens,
        }
    }
}

impl Default for VerificationConfig {
    #[inline]
    fn default() -> Self {
        Self {
            model: "gpt-4o-mini".to_string(),
            quality_threshold: 0.85,
            detailed_diagnostics: false,
            max_tokens: 4096,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = VerificationConfig::default();
        assert_eq!(config.model, "gpt-4o-mini");
        assert_eq!(config.quality_threshold, 0.85);
        assert!(!config.detailed_diagnostics);
        assert_eq!(config.max_tokens, 4096);
    }

    #[test]
    fn test_config_from_env() {
        env::set_var("LLM_MODEL", "gpt-4o");
        env::set_var("LLM_QUALITY_THRESHOLD", "0.90");
        env::set_var("LLM_DETAILED", "true");
        env::set_var("LLM_MAX_TOKENS", "8192");

        let config = VerificationConfig::from_env();
        assert_eq!(config.model, "gpt-4o");
        assert_eq!(config.quality_threshold, 0.90);
        assert!(config.detailed_diagnostics);
        assert_eq!(config.max_tokens, 8192);

        // Clean up
        env::remove_var("LLM_MODEL");
        env::remove_var("LLM_QUALITY_THRESHOLD");
        env::remove_var("LLM_DETAILED");
        env::remove_var("LLM_MAX_TOKENS");
    }
}
