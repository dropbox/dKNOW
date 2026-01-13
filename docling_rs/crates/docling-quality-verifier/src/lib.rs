//! LLM-based quality verification for document parser outputs
//!
//! This crate provides automated quality assessment of parser outputs using
//! `OpenAI`'s language models to detect semantic differences that traditional
//! string comparison would miss.
//!
//! # Features
//!
//! - **Comparative Analysis**: Compare expected vs actual outputs for semantic equivalence
//! - **Quality Scoring**: 0.0-1.0 score with detailed category breakdown
//! - **Actionable Findings**: Specific issues with severity and location
//! - **Cost Efficient**: Uses gpt-4o-mini for low-cost verification (~$0.05/month)
//!
//! # Example
//!
//! ```no_run
//! use docling_quality_verifier::{LLMQualityVerifier, VerificationConfig};
//! use docling_core::InputFormat;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let verifier = LLMQualityVerifier::new(VerificationConfig {
//!         model: "gpt-4o-mini".to_string(),
//!         quality_threshold: 0.85,
//!         detailed_diagnostics: true,
//!         max_tokens: 4096,
//!     })?;
//!
//!     let expected = std::fs::read_to_string("expected.md")?;
//!     let actual = std::fs::read_to_string("actual.md")?;
//!
//!     let report = verifier.compare_outputs(
//!         &expected,
//!         &actual,
//!         InputFormat::Docx
//!     ).await?;
//!
//!     println!("Quality Score: {:.1}%", report.score * 100.0);
//!     println!("Status: {}", if report.passed { "PASS" } else { "FAIL" });
//!
//!     for finding in report.findings {
//!         println!("[{:?}] {}", finding.severity, finding.description);
//!     }
//!
//!     Ok(())
//! }
//! ```

pub mod client;
pub mod config;
pub mod types;
pub mod verifier;
pub mod visual;

pub use client::OpenAIClient;
pub use config::VerificationConfig;
pub use types::{QualityCategory, QualityFinding, QualityReport, Severity, VisualQualityReport};
pub use verifier::LLMQualityVerifier;
pub use visual::VisualTester;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = VerificationConfig {
            model: "gpt-4o-mini".to_string(),
            quality_threshold: 0.85,
            detailed_diagnostics: true,
            max_tokens: 4096,
        };

        assert_eq!(config.model, "gpt-4o-mini");
        assert_eq!(config.quality_threshold, 0.85);
        assert!(config.detailed_diagnostics);
    }
}
