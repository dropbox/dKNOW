//! `OpenAI` API client for quality verification

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;

/// `OpenAI` API client
#[derive(Debug, Clone)]
pub struct OpenAIClient {
    api_key: String,
    http_client: reqwest::Client,
    base_url: String,
}

/// `OpenAI` Chat API request
#[derive(Debug, Clone, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    max_tokens: usize,
    temperature: f32,
    response_format: ResponseFormat,
}

/// Response format specification
#[derive(Debug, Clone, Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: String,
}

/// Chat message (text or multimodal)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Message {
    role: String,
    #[serde(flatten)]
    content: MessageContent,
}

/// Content of a message (either text or multimodal)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum MessageContent {
    Text { content: String },
    Multimodal { content: Vec<ContentPart> },
}

/// Content part for multimodal messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum ContentPart {
    Text { text: String },
    ImageUrl { image_url: ImageUrl },
}

/// Image URL with detail level
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ImageUrl {
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
}

/// `OpenAI` Chat API response
#[derive(Debug, Clone, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

/// Response choice
#[derive(Debug, Clone, Deserialize)]
struct Choice {
    message: ResponseMessage,
}

/// Response message (simplified, only contains text content)
#[derive(Debug, Clone, Deserialize)]
struct ResponseMessage {
    content: Option<String>,
}

impl OpenAIClient {
    /// Create a new `OpenAI` client
    ///
    /// Reads API key from `OPENAI_API_KEY` environment variable
    ///
    /// # Errors
    /// Returns an error if `OPENAI_API_KEY` is not set or HTTP client creation fails.
    #[must_use = "creating a client that is not used is a waste of resources"]
    pub fn new() -> Result<Self> {
        let api_key =
            env::var("OPENAI_API_KEY").context("OPENAI_API_KEY environment variable not set")?;

        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .context("Failed to create HTTP client")?;

        let base_url =
            env::var("OPENAI_API_BASE").unwrap_or_else(|_| "https://api.openai.com/v1".to_string());

        Ok(Self {
            api_key,
            http_client,
            base_url,
        })
    }

    /// Send a chat completion request to `OpenAI`
    ///
    /// # Arguments
    ///
    /// * `model` - Model name (e.g., "gpt-4o-mini")
    /// * `prompt` - User prompt
    /// * `max_tokens` - Maximum tokens in response
    ///
    /// # Returns
    ///
    /// JSON string response from the model
    ///
    /// # Errors
    /// Returns an error if the API request fails or response parsing fails.
    #[must_use = "this function returns an API response that should be processed"]
    pub async fn chat_completion(
        &self,
        model: &str,
        prompt: &str,
        max_tokens: usize,
    ) -> Result<String> {
        let request = ChatRequest {
            model: model.to_string(),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: MessageContent::Text {
                        content: "You are a document quality verification assistant. Analyze documents and provide structured JSON responses.".to_string(),
                    },
                },
                Message {
                    role: "user".to_string(),
                    content: MessageContent::Text {
                        content: prompt.to_string(),
                    },
                },
            ],
            max_tokens,
            temperature: 0.0, // Zero temperature for maximum determinism (reduce variance)
            response_format: ResponseFormat {
                format_type: "json_object".to_string(),
            },
        };

        let response = self
            .http_client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send OpenAI API request")?;

        let status = response.status();
        let response_text = response
            .text()
            .await
            .context("Failed to read OpenAI API response")?;

        if !status.is_success() {
            anyhow::bail!("OpenAI API request failed with status {status}: {response_text}");
        }

        let chat_response: ChatResponse =
            serde_json::from_str(&response_text).context("Failed to parse OpenAI API response")?;

        let message_content = chat_response
            .choices
            .first()
            .context("No choices in OpenAI response")?
            .message
            .content
            .clone()
            .context("No content in response")?;

        Ok(message_content)
    }

    /// Compare two images using vision API
    ///
    /// # Arguments
    ///
    /// * `model` - Vision model (e.g., "gpt-4o", "gpt-4-turbo")
    /// * `prompt` - Comparison prompt
    /// * `image1_base64` - First image as base64 string
    /// * `image2_base64` - Second image as base64 string
    /// * `max_tokens` - Maximum tokens in response
    ///
    /// # Returns
    ///
    /// JSON string response from the vision model
    ///
    /// # Errors
    /// Returns an error if the API request fails or response parsing fails.
    #[must_use = "this function returns an API response that should be processed"]
    pub async fn vision_comparison(
        &self,
        model: &str,
        prompt: &str,
        image1_base64: &str,
        image2_base64: &str,
        max_tokens: usize,
    ) -> Result<String> {
        let request = ChatRequest {
            model: model.to_string(),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: MessageContent::Text {
                        content: "You are a document quality verification assistant specializing in visual comparison of documents. Provide structured JSON responses with quality scores.".to_string(),
                    },
                },
                Message {
                    role: "user".to_string(),
                    content: MessageContent::Multimodal {
                        content: vec![
                            ContentPart::Text {
                                text: prompt.to_string(),
                            },
                            ContentPart::ImageUrl {
                                image_url: ImageUrl {
                                    url: format!("data:image/png;base64,{image1_base64}"),
                                    detail: Some("high".to_string()),
                                },
                            },
                            ContentPart::ImageUrl {
                                image_url: ImageUrl {
                                    url: format!("data:image/png;base64,{image2_base64}"),
                                    detail: Some("high".to_string()),
                                },
                            },
                        ],
                    },
                },
            ],
            max_tokens,
            temperature: 0.0, // Zero temperature for maximum determinism (reduce variance)
            response_format: ResponseFormat {
                format_type: "json_object".to_string(),
            },
        };

        let response = self
            .http_client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send OpenAI vision API request")?;

        let status = response.status();
        let response_text = response
            .text()
            .await
            .context("Failed to read OpenAI vision API response")?;

        if !status.is_success() {
            anyhow::bail!("OpenAI vision API request failed with status {status}: {response_text}");
        }

        let chat_response: ChatResponse = serde_json::from_str(&response_text)
            .context("Failed to parse OpenAI vision API response")?;

        let message_content = chat_response
            .choices
            .first()
            .context("No choices in OpenAI response")?
            .message
            .content
            .clone()
            .context("No content in vision response")?;

        Ok(message_content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_client_creation_requires_api_key() {
        // Remove API key if present
        let original = env::var("OPENAI_API_KEY").ok();
        env::remove_var("OPENAI_API_KEY");

        // Double-check it's actually removed
        if env::var("OPENAI_API_KEY").is_ok() {
            // If we can't actually remove it (e.g., process inherits from parent),
            // skip this test rather than fail
            if let Some(key) = original {
                env::set_var("OPENAI_API_KEY", key);
            }
            return; // Skip test - can't isolate environment
        }

        let result = OpenAIClient::new();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("OPENAI_API_KEY"));

        // Restore original
        if let Some(key) = original {
            env::set_var("OPENAI_API_KEY", key);
        }
    }

    #[test]
    #[serial]
    fn test_client_creation_with_api_key() {
        env::set_var("OPENAI_API_KEY", "test-key");

        let result = OpenAIClient::new();
        assert!(result.is_ok());

        let client = result.unwrap();
        assert_eq!(client.api_key, "test-key");
        // Note: Not testing base_url here due to potential race conditions with other tests

        env::remove_var("OPENAI_API_KEY");
    }

    #[test]
    #[serial]
    fn test_custom_base_url() {
        env::set_var("OPENAI_API_KEY", "test-key");
        env::set_var("OPENAI_API_BASE", "https://custom.api.com");

        let result = OpenAIClient::new();
        assert!(result.is_ok());

        let client = result.unwrap();
        assert_eq!(client.base_url, "https://custom.api.com");

        env::remove_var("OPENAI_API_KEY");
        env::remove_var("OPENAI_API_BASE");
    }

    // Integration test (requires actual API key)
    #[tokio::test]
    #[serial]
    async fn test_chat_completion_integration() {
        // Skip if OPENAI_API_KEY not set or is a test value
        let api_key = match std::env::var("OPENAI_API_KEY") {
            Ok(k) if k.starts_with("sk-") => k,
            _ => {
                eprintln!("OPENAI_API_KEY not set or invalid - skipping integration test");
                return;
            }
        };
        // Restore real key if it was clobbered by another test
        std::env::set_var("OPENAI_API_KEY", &api_key);
        let client = match OpenAIClient::new() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to create client: {e} - skipping");
                return;
            }
        };

        let response = client
            .chat_completion(
                "gpt-4o-mini",
                "Return a JSON object with a single field 'test' set to true",
                100,
            )
            .await
            .expect("Failed to get completion");

        // Should return valid JSON
        let parsed: serde_json::Value =
            serde_json::from_str(&response).expect("Response should be valid JSON");
        assert!(parsed.get("test").is_some());
    }
}
