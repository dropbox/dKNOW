//! Main quality verifier implementation

// Clippy pedantic allows:
// - Score percentages (0-100) are safely cast to u8
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]

use crate::client::OpenAIClient;
use crate::config::VerificationConfig;
use crate::types::{CategoryScores, QualityCategory, QualityFinding, QualityReport, Severity};
use anyhow::{Context, Result};
use docling_core::InputFormat;
use serde::{Deserialize, Serialize};

/// LLM-based quality verifier
///
/// Uses `OpenAI` models to perform semantic quality analysis of parser outputs
#[derive(Debug, Clone)]
pub struct LLMQualityVerifier {
    client: OpenAIClient,
    config: VerificationConfig,
}

/// Internal structure for parsing LLM JSON response
#[derive(Debug, Clone, Deserialize, Serialize)]
struct LLMResponse {
    #[serde(alias = "score", alias = "quality_score")]
    overall_score: Option<f64>,
    #[serde(default, alias = "scores")]
    category_scores: Option<LLMCategoryScores>,
    #[serde(default)]
    findings: Vec<LLMFinding>,
    #[serde(default)]
    reasoning: Option<String>,
}

/// Flexible category scores that accept both u8 (0-100) and f64 (0.0-1.0) from LLM
#[derive(Debug, Clone, Deserialize, Serialize)]
struct LLMCategoryScores {
    #[serde(default, deserialize_with = "deserialize_optional_score")]
    completeness: Option<u8>,
    #[serde(default, deserialize_with = "deserialize_optional_score")]
    accuracy: Option<u8>,
    #[serde(default, deserialize_with = "deserialize_optional_score")]
    structure: Option<u8>,
    #[serde(default, deserialize_with = "deserialize_optional_score")]
    formatting: Option<u8>,
    #[serde(default, deserialize_with = "deserialize_optional_score")]
    metadata: Option<u8>,
}

/// Deserialize optional score that can be either u8 (0-100) or f64 (0.0-1.0)
fn deserialize_optional_score<'de, D>(deserializer: D) -> Result<Option<u8>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    use serde_json::Value;

    let value = Value::deserialize(deserializer)?;

    match value {
        Value::Null => Ok(None),
        // Already an integer 0-100
        Value::Number(n) if n.is_u64() => {
            let score = n.as_u64().unwrap();
            if score <= 100 {
                Ok(Some(score as u8))
            } else {
                Err(D::Error::custom(format!(
                    "Score {score} out of range 0-100"
                )))
            }
        }
        // Float 0.0-1.0, convert to 0-100
        Value::Number(n) if n.is_f64() => {
            let score_float = n.as_f64().unwrap();
            if (0.0..=1.0).contains(&score_float) {
                Ok(Some((score_float * 100.0).round() as u8))
            } else if (0.0..=100.0).contains(&score_float) {
                // Already in 0-100 range but as float
                Ok(Some(score_float.round() as u8))
            } else {
                Err(D::Error::custom(format!(
                    "Score {score_float} out of range 0.0-1.0 or 0-100"
                )))
            }
        }
        _ => Err(D::Error::custom("Expected number or null for score")),
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct LLMFinding {
    category: String,
    severity: String,
    description: String,
    location: Option<String>,
}

/// Helper function to check if format is binary (not text-based)
///
/// Binary formats cannot be read as UTF-8 text. When verifying
/// standalone output, we skip reading the input file for these formats
/// and only validate the markdown output structure.
const fn is_binary_format(format: InputFormat) -> bool {
    matches!(
        format,
        // Archives (always binary)
        InputFormat::Zip | InputFormat::Tar | InputFormat::SevenZ | InputFormat::Rar |
        // Images (always binary)
        InputFormat::Png | InputFormat::Jpeg | InputFormat::Tiff | InputFormat::Webp |
        InputFormat::Bmp | InputFormat::Gif | InputFormat::Heif | InputFormat::Avif |
        InputFormat::Dicom |
        // E-books (compressed archives)
        InputFormat::Epub | InputFormat::Mobi |
        // Office formats (ZIP-based)
        InputFormat::Docx | InputFormat::Pptx | InputFormat::Xlsx |
        InputFormat::Odt | InputFormat::Ods | InputFormat::Odp |
        InputFormat::Pages | InputFormat::Numbers | InputFormat::Key | InputFormat::Vsdx |
        // Compressed formats
        InputFormat::Kmz |
        // 3D binary formats
        InputFormat::Glb | InputFormat::Stl |
        // Other binary
        InputFormat::Msg | InputFormat::Mpp | InputFormat::One | InputFormat::Mdb |
        InputFormat::Pub | InputFormat::Idml |
        // PDF
        InputFormat::Pdf |
        // Audio/Video
        InputFormat::Wav | InputFormat::Mp3 | InputFormat::Mp4 | InputFormat::Mkv |
        InputFormat::Mov | InputFormat::Avi
    )
}

impl LLMQualityVerifier {
    /// Create a new quality verifier
    ///
    /// # Errors
    /// Returns an error if the `OpenAI` client cannot be initialized.
    #[must_use = "this function returns a verifier that should be used"]
    pub fn new(config: VerificationConfig) -> Result<Self> {
        let client = OpenAIClient::new()?;
        Ok(Self { client, config })
    }

    /// Create a verifier with default configuration
    ///
    /// # Errors
    /// Returns an error if the `OpenAI` client cannot be initialized.
    #[must_use = "this function returns a verifier that should be used"]
    pub fn with_defaults() -> Result<Self> {
        Self::new(VerificationConfig::default())
    }

    /// Create a verifier from environment variables
    ///
    /// # Errors
    /// Returns an error if the `OpenAI` client cannot be initialized.
    #[must_use = "this function returns a verifier that should be used"]
    pub fn from_env() -> Result<Self> {
        Self::new(VerificationConfig::from_env())
    }

    /// Compare two parser outputs for semantic equivalence (Mode 2)
    ///
    /// This is the primary verification mode used during parser migration.
    /// Compares expected output (from Python baseline) with actual output
    /// (from Rust parser) and returns detailed quality assessment.
    ///
    /// # Arguments
    ///
    /// * `expected` - Expected output (baseline from production parser)
    /// * `actual` - Actual output (from new parser implementation)
    /// * `format` - Document format being parsed
    ///
    /// # Returns
    ///
    /// Quality report with score, findings, and pass/fail status
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use docling_quality_verifier::{LLMQualityVerifier, VerificationConfig};
    /// # use docling_core::InputFormat;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let verifier = LLMQualityVerifier::with_defaults()?;
    /// let expected = std::fs::read_to_string("expected.md")?;
    /// let actual = std::fs::read_to_string("actual.md")?;
    ///
    /// let report = verifier.compare_outputs(&expected, &actual, InputFormat::Docx).await?;
    ///
    /// if report.passed {
    ///     println!("✅ Quality check passed: {:.1}%", report.score * 100.0);
    /// } else {
    ///     println!("❌ Quality check failed: {:.1}%", report.score * 100.0);
    ///     for finding in report.findings {
    ///         println!("  [{:?}] {}", finding.severity, finding.description);
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    /// Returns an error if the LLM API call fails or response parsing fails.
    #[must_use = "this function returns a quality report that should be processed"]
    pub async fn compare_outputs(
        &self,
        expected: &str,
        actual: &str,
        format: InputFormat,
    ) -> Result<QualityReport> {
        // Truncate outputs if too long (to stay within token limits)
        let expected_truncated = Self::truncate_text(expected, 8000);
        let actual_truncated = Self::truncate_text(actual, 8000);

        let prompt = self.build_comparison_prompt(expected_truncated, actual_truncated, format);

        let response = self
            .client
            .chat_completion(&self.config.model, &prompt, self.config.max_tokens)
            .await
            .context("Failed to get LLM response")?;

        // Debug logging to diagnose LLM test issues
        if std::env::var("DEBUG_LLM_TESTS").is_ok() {
            eprintln!("\n=== LLM REQUEST (verify_format) ===");
            eprintln!("{prompt}");
            eprintln!("\n=== LLM RESPONSE ===");
            eprintln!("{response}");
            eprintln!("=== END LLM DEBUG ===\n");
        }

        self.parse_llm_response(&response)
            .context("Failed to parse LLM response")
    }

    /// Standalone verification for formats without ground truth (Mode 3)
    ///
    /// Validates parser output against the original input document when no
    /// Python baseline exists. LLM reads both input and output to assess quality.
    ///
    /// # Arguments
    ///
    /// * `input_file` - Path to original document
    /// * `output` - Parser output (markdown)
    /// * `format` - Document format
    ///
    /// # Returns
    ///
    /// Quality report with score 0.0-1.0. Lower threshold (0.75) used since
    /// no authoritative baseline exists.
    ///
    /// # Errors
    /// Returns an error if file reading fails or the LLM API call fails.
    #[must_use = "this function returns a quality report that should be processed"]
    pub async fn verify_standalone(
        &self,
        input_file: &std::path::Path,
        output: &str,
        format: InputFormat,
    ) -> Result<QualityReport> {
        use std::fs;

        // For binary formats, skip input file reading and only validate output structure
        let prompt_section = if is_binary_format(format) {
            format!(
                "NOTE: Input file is binary format ({format}). Validate only the parser output structure."
            )
        } else {
            // Read original file for text formats
            let input_content = fs::read_to_string(input_file)
                .context("Failed to read input file for standalone verification")?;
            let truncated = Self::truncate_text(&input_content, 4000);
            format!("ORIGINAL INPUT DOCUMENT:\n```\n{truncated}\n```")
        };

        let output_truncated = Self::truncate_text(output, 4000);

        let prompt = format!(
            r#"Evaluate if this parser output accurately represents the input document.

IMPORTANT: This is a document CONVERSION system. The goal is to convert the input
document TO markdown format, not to preserve the original format's structure or syntax.

- For XML-based formats (SVG, KML, GPX, etc.): Loss of XML tags/structure is EXPECTED and CORRECT
- For binary formats: Conversion to text representation is EXPECTED and CORRECT
- Focus on: Content preservation, logical organization, semantic correctness
- NOT focus on: Original format syntax, exact formatting, structural preservation

FORMAT: {format}

{prompt_section}

PARSER OUTPUT:
```
{output_truncated}
```

Evaluate quality on these dimensions:

1. **Completeness** (0-100): Is all important content from input present in output?
   - Check: All text, images, tables, metadata extracted
   - NOT check: Original format syntax preserved

2. **Accuracy** (0-100): Is the content semantically correct?
   - Check: Meaning preserved, no hallucinations
   - NOT check: Exact character-by-character match

3. **Structure** (0-100): Is the document's CONTENT organization preserved in markdown format?
   - Check: Logical sections, headings hierarchy, content flow maintained
   - NOT check: Original format's syntax (XML tags, indentation, binary structure)

4. **Formatting** (0-100): Are tables/lists/content formatted correctly IN MARKDOWN?
   - Check: Markdown tables render correctly, lists are structured, content is readable
   - NOT check: Exact replication of original format's rendering

5. **Metadata** (0-100): Are titles/headers/authors preserved?
   - Check: Document metadata extracted and included
   - NOT check: Original format's metadata syntax

For each category with score < 95:
- List specific issues
- Assign severity: "critical", "major", "minor", or "info"
- Provide location if identifiable

Calculate overall_score as weighted average:
- completeness: 30%
- accuracy: 30%
- structure: 20%
- formatting: 15%
- metadata: 5%

Return JSON ONLY:
{{
  "overall_score": 0.0-1.0,
  "category_scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Specific issue description",
      "location": "Section X" (optional)
    }}
  ],
  "reasoning": "Brief assessment"
}}
"#
        );

        let response = self
            .client
            .chat_completion(&self.config.model, &prompt, self.config.max_tokens)
            .await
            .context("Failed to get LLM response for standalone verification")?;

        // Debug logging to diagnose LLM test issues
        if std::env::var("DEBUG_LLM_TESTS").is_ok() {
            eprintln!("\n=== LLM REQUEST (verify_standalone) ===");
            eprintln!("{prompt}");
            eprintln!("\n=== LLM RESPONSE ===");
            eprintln!("{response}");
            eprintln!("=== END LLM DEBUG ===\n");
        }

        self.parse_llm_response(&response)
            .context("Failed to parse standalone verification response")
    }

    /// Truncate text to approximate token limit
    ///
    /// Uses character count as rough approximation (1 token ≈ 4 chars)
    fn truncate_text(text: &str, max_chars: usize) -> &str {
        if text.len() <= max_chars {
            text
        } else {
            let truncate_at = text
                .char_indices()
                .nth(max_chars)
                .map_or(text.len(), |(i, _)| i);
            &text[..truncate_at]
        }
    }

    /// Build comparison prompt for Mode 2
    fn build_comparison_prompt(&self, expected: &str, actual: &str, format: InputFormat) -> String {
        format!(
            r#"Compare two document parser outputs for semantic equivalence.

IMPORTANT: Both outputs are markdown conversions from the same {format} input document.
Focus on whether the actual output preserves the same CONTENT and ORGANIZATION as expected,
not whether it matches the original {format} format syntax.

FORMAT: {format}

EXPECTED OUTPUT (baseline from production parser):
```
{expected}
```

ACTUAL OUTPUT (from new parser implementation):
```
{actual}
```

Evaluate quality on these dimensions:

1. **Completeness** (0-100): Are all sections/pages/elements present?
   - Check for missing paragraphs, tables, lists, headings
   - Verify page count and structure
   - Score 100 if everything present, 0 if major content missing

2. **Accuracy** (0-100): Is content semantically correct?
   - Compare text content for equivalence (exact match not required)
   - Check numbers, data, technical terms
   - Score 100 if semantically identical, 0 if major errors

3. **Structure** (0-100): Is document hierarchy preserved?
   - Heading levels (H1, H2, H3)
   - Section organization
   - Document outline
   - Score 100 if structure matches, 0 if completely wrong

4. **Formatting** (0-100): Are tables/lists/code blocks correct?
   - Table structure and data
   - List formatting (ordered/unordered)
   - Code blocks, quotes
   - Score 100 if formatting correct, 0 if garbled

5. **Metadata** (0-100): Are titles/authors/dates correct?
   - Document title
   - Author information
   - Dates and timestamps
   - Score 100 if metadata matches, 0 if missing/wrong

For each category with score < 95:
- List specific issues found
- Assign severity: "critical" (unusable), "major" (significant problem), "minor" (small difference), "info" (acceptable variation)
- Provide location if identifiable (e.g., "Page 2", "Table 3")

Calculate overall_score as weighted average:
- completeness: 30%
- accuracy: 30%
- structure: 20%
- formatting: 15%
- metadata: 5%

Return JSON ONLY (no markdown, no explanation):
{{
  "overall_score": 0.0-1.0,
  "category_scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Specific issue description",
      "location": "Page 2, Table 3" (optional)
    }}
  ],
  "reasoning": "Brief 1-2 sentence explanation of overall assessment"
}}

{detailed_instruction}"#,
            format = format,
            expected = expected,
            actual = actual,
            detailed_instruction = if self.config.detailed_diagnostics {
                "Include detailed reasoning."
            } else {
                "Keep reasoning brief."
            }
        )
    }

    /// Custom verification with user-provided prompt
    ///
    /// Allows direct interaction with LLM for specialized validation tasks.
    /// User provides custom prompt, LLM returns JSON following standard format.
    ///
    /// # Arguments
    ///
    /// * `prompt` - Custom prompt for LLM (must request JSON response)
    ///
    /// # Returns
    ///
    /// Quality report parsed from LLM's JSON response
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use docling_quality_verifier::LLMQualityVerifier;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let verifier = LLMQualityVerifier::with_defaults()?;
    /// let prompt = "Evaluate this JSON for completeness...";
    /// let report = verifier.custom_verification(prompt).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    /// Returns an error if the LLM API call fails or response parsing fails.
    #[must_use = "this function returns a quality report that should be processed"]
    pub async fn custom_verification(&self, prompt: &str) -> Result<QualityReport> {
        let response = self
            .client
            .chat_completion(&self.config.model, prompt, self.config.max_tokens)
            .await
            .context("Failed to get LLM response for custom verification")?;

        // Debug logging to diagnose LLM test issues
        if std::env::var("DEBUG_LLM_TESTS").is_ok() {
            eprintln!("\n=== LLM REQUEST (custom_verification) ===");
            eprintln!("{prompt}");
            eprintln!("\n=== LLM RESPONSE ===");
            eprintln!("{response}");
            eprintln!("=== END LLM DEBUG ===\n");
        }

        self.parse_llm_response(&response)
            .context("Failed to parse custom verification response")
    }

    /// Parse LLM JSON response into `QualityReport`
    fn parse_llm_response(&self, response: &str) -> Result<QualityReport> {
        let llm_response: LLMResponse =
            serde_json::from_str(response).context("Failed to parse LLM response as JSON")?;

        // Get category scores or use defaults
        let category_scores = llm_response.category_scores.map_or(
            CategoryScores {
                completeness: 0,
                accuracy: 0,
                structure: 0,
                formatting: 0,
                metadata: 0,
            },
            |scores| CategoryScores {
                completeness: scores.completeness.unwrap_or(0),
                accuracy: scores.accuracy.unwrap_or(0),
                structure: scores.structure.unwrap_or(0),
                formatting: scores.formatting.unwrap_or(0),
                metadata: scores.metadata.unwrap_or(0),
            },
        );

        // Get overall score or compute from category scores
        let overall_score = if let Some(score) = llm_response.overall_score {
            // Validate overall_score is in range
            if !(0.0..=1.0).contains(&score) {
                anyhow::bail!("Invalid overall_score: {score} (must be 0.0-1.0)");
            }
            score
        } else {
            // Compute average of category scores
            // Divide by 500 to get 0.0-1.0 scale (5 categories * 100)
            (f64::from(category_scores.completeness)
                + f64::from(category_scores.accuracy)
                + f64::from(category_scores.structure)
                + f64::from(category_scores.formatting)
                + f64::from(category_scores.metadata))
                / 500.0
        };

        // Convert LLM findings to typed findings
        let findings: Result<Vec<QualityFinding>> = llm_response
            .findings
            .into_iter()
            .map(|f| {
                let category = match f.category.as_str() {
                    "completeness" => QualityCategory::Completeness,
                    "accuracy" => QualityCategory::Accuracy,
                    "structure" => QualityCategory::Structure,
                    "formatting" => QualityCategory::Formatting,
                    "metadata" => QualityCategory::Metadata,
                    _ => anyhow::bail!("Invalid category: {}", f.category),
                };

                let severity = match f.severity.as_str() {
                    "critical" => Severity::Critical,
                    "major" => Severity::Major,
                    "minor" => Severity::Minor,
                    "info" => Severity::Info,
                    _ => anyhow::bail!("Invalid severity: {}", f.severity),
                };

                Ok(QualityFinding {
                    category,
                    severity,
                    description: f.description,
                    location: f.location,
                })
            })
            .collect();

        let reasoning = if self.config.detailed_diagnostics {
            llm_response.reasoning
        } else {
            None
        };

        Ok(QualityReport::new(
            overall_score,
            self.config.quality_threshold,
            category_scores,
            findings?,
            reasoning,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn test_truncate_text() {
        let text = "Hello World!";
        assert_eq!(LLMQualityVerifier::truncate_text(text, 5), "Hello");
        assert_eq!(LLMQualityVerifier::truncate_text(text, 100), text);
    }

    #[test]
    #[serial]
    fn test_parse_llm_response() {
        let config = VerificationConfig::default();
        let verifier = LLMQualityVerifier {
            client: OpenAIClient::new().unwrap_or_else(|_| {
                std::env::set_var("OPENAI_API_KEY", "test");
                OpenAIClient::new().unwrap()
            }),
            config,
        };

        let response = r#"{
            "overall_score": 0.92,
            "category_scores": {
                "completeness": 95,
                "accuracy": 90,
                "structure": 92,
                "formatting": 88,
                "metadata": 100
            },
            "findings": [
                {
                    "category": "formatting",
                    "severity": "minor",
                    "description": "Table spacing slightly different",
                    "location": "Table 2"
                }
            ],
            "reasoning": "Output is highly accurate with minor formatting differences."
        }"#;

        let report = verifier.parse_llm_response(response).unwrap();
        assert_eq!(report.score, 0.92);
        assert!(report.passed); // 0.92 > 0.85 threshold
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.category_scores.completeness, 95);
    }

    #[test]
    #[serial]
    fn test_parse_llm_response_invalid_score() {
        let config = VerificationConfig::default();
        let verifier = LLMQualityVerifier {
            client: OpenAIClient::new().unwrap_or_else(|_| {
                std::env::set_var("OPENAI_API_KEY", "test");
                OpenAIClient::new().unwrap()
            }),
            config,
        };

        let response = r#"{
            "overall_score": 1.5,
            "category_scores": {
                "completeness": 95,
                "accuracy": 90,
                "structure": 92,
                "formatting": 88,
                "metadata": 100
            },
            "findings": [],
            "reasoning": "Test"
        }"#;

        let result = verifier.parse_llm_response(response);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid overall_score"));
    }
}
