//! LLM-based quality verification for document conversion
//!
//! Uses OpenAI GPT-4 to semantically verify that parsed output
//! preserves essential content from the original document.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Configuration for OpenAI API
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LlmVerifierConfig {
    pub api_key: String,
    pub model: String, // e.g., "gpt-4", "gpt-4-turbo"
}

impl Default for LlmVerifierConfig {
    #[inline]
    fn default() -> Self {
        Self {
            api_key: std::env::var("OPENAI_API_KEY").unwrap_or_default(),
            model: "gpt-4".to_string(),
        }
    }
}

/// Result of LLM verification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerificationResult {
    /// Whether content is semantically equivalent
    pub is_equivalent: bool,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f64,
    /// Detailed explanation
    pub explanation: String,
    /// Issues found (if any)
    pub issues: Vec<String>,
    /// What was preserved well
    pub preserved: Vec<String>,
}

/// LLM-based document conversion verifier
#[derive(Debug, Clone)]
pub struct LlmVerifier {
    config: LlmVerifierConfig,
    client: reqwest::blocking::Client,
}

impl LlmVerifier {
    /// Create new LLM verifier with API key from environment
    #[must_use = "this returns a Result that should be handled"]
    pub fn new() -> Result<Self> {
        let config = LlmVerifierConfig::default();
        if config.api_key.is_empty() {
            anyhow::bail!(
                "OPENAI_API_KEY environment variable not set.\n\
                 Set it with: export OPENAI_API_KEY=sk-..."
            );
        }

        Ok(Self {
            config,
            client: reqwest::blocking::Client::new(),
        })
    }

    /// Verify that parsed markdown preserves content from original document
    ///
    /// # Arguments
    /// * `original_path` - Path to original document
    /// * `parsed_markdown` - Markdown output from conversion
    /// * `format_name` - Format name for context (e.g., "Pages", "LaTeX")
    pub fn verify_conversion(
        &self,
        original_path: &Path,
        parsed_markdown: &str,
        format_name: &str,
    ) -> Result<VerificationResult> {
        // Build prompt for LLM
        let prompt = self.build_verification_prompt(original_path, parsed_markdown, format_name)?;

        // Call OpenAI API
        let response = self.call_openai(&prompt)?;

        // Parse response
        self.parse_verification_response(&response)
    }

    fn build_verification_prompt(
        &self,
        original_path: &Path,
        parsed_markdown: &str,
        format_name: &str,
    ) -> Result<String> {
        Ok(format!(
            "You are a document conversion quality verifier. Analyze whether the parsed output \
             correctly preserves the essential content from the original document.\n\n\
             **Original Document:**\n\
             - Format: {}\n\
             - File: {}\n\
             - (You cannot see the original file, but the user will describe it or you should infer from the output)\n\n\
             **Parsed Output (Markdown):**\n\
             ```markdown\n{}\n```\n\n\
             **Task:**\n\
             Evaluate this conversion on the following criteria:\n\
             1. **Content Completeness:** Is all essential text content present?\n\
             2. **Structure Preservation:** Are sections, headings, lists preserved?\n\
             3. **Table Integrity:** Are tables (if any) correctly extracted?\n\
             4. **Formatting:** Is basic formatting (bold, italic, links) preserved?\n\
             5. **Metadata:** Is document title/structure clear?\n\n\
             **Respond in JSON format:**\n\
             {{\n  \
               \"is_equivalent\": true/false,\n  \
               \"confidence\": 0.0 to 1.0,\n  \
               \"explanation\": \"detailed analysis\",\n  \
               \"issues\": [\"list of problems found\"],\n  \
               \"preserved\": [\"list of what was preserved well\"]\n\
             }}\n\n\
             Focus on semantic equivalence, not exact formatting. \
             Consider that conversion chains may lose some styling but should preserve content.",
            format_name,
            original_path.display(),
            if parsed_markdown.len() > 4000 {
                &format!("{}...\n\n[truncated, {} total chars]", &parsed_markdown[..4000], parsed_markdown.len())
            } else {
                parsed_markdown
            }
        ))
    }

    fn call_openai(&self, prompt: &str) -> Result<String> {
        #[derive(Serialize)]
        struct ChatRequest {
            model: String,
            messages: Vec<Message>,
            temperature: f64,
        }

        #[derive(Serialize)]
        struct Message {
            role: String,
            content: String,
        }

        #[derive(Deserialize)]
        struct ChatResponse {
            choices: Vec<Choice>,
        }

        #[derive(Deserialize)]
        struct Choice {
            message: ResponseMessage,
        }

        #[derive(Deserialize)]
        struct ResponseMessage {
            content: String,
        }

        let request = ChatRequest {
            model: self.config.model.clone(),
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            temperature: 0.3, // Low temperature for consistent analysis
        };

        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .context("Failed to call OpenAI API")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            anyhow::bail!("OpenAI API error {}: {}", status, body);
        }

        let chat_response: ChatResponse = response
            .json()
            .context("Failed to parse OpenAI response")?;

        let content = chat_response
            .choices
            .first()
            .context("No response from OpenAI")?
            .message
            .content
            .clone();

        Ok(content)
    }

    fn parse_verification_response(&self, response: &str) -> Result<VerificationResult> {
        // Extract JSON from response (may be wrapped in markdown code blocks)
        let json_str = if let Some(start) = response.find('{') {
            if let Some(end) = response.rfind('}') {
                &response[start..=end]
            } else {
                response
            }
        } else {
            response
        };

        let result: VerificationResult = serde_json::from_str(json_str)
            .context("Failed to parse LLM response as JSON")?;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verifier_creation() {
        // Skip if no API key
        if std::env::var("OPENAI_API_KEY").is_err() {
            println!("⚠️  OPENAI_API_KEY not set, skipping test");
            return;
        }

        let verifier = LlmVerifier::new();
        assert!(verifier.is_ok(), "Should create verifier with valid API key");
    }

    #[test]
    fn test_prompt_building() {
        let config = LlmVerifierConfig {
            api_key: "test-key".to_string(),
            model: "gpt-4".to_string(),
        };
        let verifier = LlmVerifier {
            config,
            client: reqwest::blocking::Client::new(),
        };

        let prompt = verifier.build_verification_prompt(
            Path::new("test.pages"),
            "# Document\n\nSome content",
            "Pages",
        );

        assert!(prompt.is_ok());
        let prompt_text = prompt.unwrap();
        assert!(prompt_text.contains("Pages"));
        assert!(prompt_text.contains("Some content"));
    }
}
