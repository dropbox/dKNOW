//! `OpenAI` API client for GPT-4o and o1 vision models.
//!
//! This module provides an async client for `OpenAI`'s vision-capable models
//! to extract document elements from PDF page images.
//!
//! ## Supported Models
//!
//! - **GPT-4o**: Fast, cost-effective vision model with JSON mode support
//! - **o1**: Advanced reasoning model with vision capabilities
//!
//! ## Example
//!
//! ```no_run
//! use docling_llm_verify::models::openai::{OpenAIClient, OpenAIModel};
//!
//! # async fn example() -> anyhow::Result<()> {
//! let client = OpenAIClient::new(std::env::var("OPENAI_API_KEY")?);
//!
//! // Extract from a page image
//! let page_png: &[u8] = &[]; // Your PNG data
//! let result = client.extract_page(OpenAIModel::Gpt4o, page_png, 1).await?;
//!
//! println!("Model: {}", result.model);
//! println!("Elements: {}", result.extraction.elements.len());
//! println!("Cost: ${:.4}", result.cost_usd);
//! println!("Latency: {}ms", result.latency_ms);
//! # Ok(())
//! # }
//! ```
//!
//! ## Cost Information
//!
//! | Model | Input (per 1M tokens) | Output (per 1M tokens) |
//! |-------|----------------------|------------------------|
//! | GPT-4o | $2.50 | $10.00 |
//! | o1 | $15.00 | $60.00 |
//!
//! ## API Differences
//!
//! - GPT-4o: Supports `response_format: json_object`, temperature control
//! - o1: Uses `max_completion_tokens`, requires temperature=1, no JSON mode

// Clippy pedantic allows:
// - Timestamp calculations
#![allow(clippy::cast_possible_truncation)]

use anyhow::{Context, Result};
use base64::Engine;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use super::{DocItemLabel, ExtractedElement, LlmExtractionResult, PageExtraction};

/// `OpenAI` chat completion request for GPT-4o.
#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_completion_tokens: Option<u32>,
    temperature: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
}

#[derive(Debug, Serialize)]
struct ResponseFormat {
    r#type: String,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: Vec<Content>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum Content {
    Text { r#type: String, text: String },
    Image { r#type: String, image_url: ImageUrl },
}

#[derive(Debug, Serialize)]
struct ImageUrl {
    url: String,
    detail: String,
}

/// `OpenAI` chat completion response.
#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
    usage: Usage,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Debug, Deserialize)]
struct ResponseMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

/// LLM response format for page extraction.
#[derive(Debug, Deserialize)]
struct ExtractionResponse {
    page_number: u32,
    elements: Vec<ElementResponse>,
    reading_order: Vec<usize>,
}

#[derive(Debug, Deserialize)]
struct ElementResponse {
    label: String,
    text: String,
    bbox: Option<BBoxResponse>,
    confidence: f64,
    table_data: Option<TableDataResponse>,
}

#[derive(Debug, Deserialize)]
struct BBoxResponse {
    l: f64,
    t: f64,
    r: f64,
    b: f64,
}

#[derive(Debug, Deserialize)]
struct TableDataResponse {
    rows: Vec<Vec<String>>,
    num_rows: usize,
    num_cols: usize,
}

/// `OpenAI` model variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OpenAIModel {
    /// GPT-4o - Fast vision model with JSON mode support
    Gpt4o,
    /// o1 - Advanced reasoning model with vision
    O1,
}

impl OpenAIModel {
    /// Get the `OpenAI` API model identifier string
    #[inline]
    #[must_use = "returns OpenAI model identifier"]
    pub const fn model_id(&self) -> &str {
        match self {
            Self::Gpt4o => "gpt-4o",
            Self::O1 => "o1",
        }
    }

    /// Cost per 1M tokens (input, output).
    #[inline]
    #[must_use = "returns input/output token costs"]
    pub const fn cost_per_million(&self) -> (f64, f64) {
        match self {
            Self::Gpt4o => (2.50, 10.00), // $2.50/1M input, $10/1M output
            Self::O1 => (15.00, 60.00),   // $15/1M input, $60/1M output
        }
    }

    /// Calculate total API cost for a given number of tokens
    #[inline]
    #[must_use = "calculates total API cost"]
    pub fn calculate_cost(&self, input_tokens: u32, output_tokens: u32) -> f64 {
        let (input_rate, output_rate) = self.cost_per_million();
        (f64::from(input_tokens) * input_rate / 1_000_000.0)
            + (f64::from(output_tokens) * output_rate / 1_000_000.0)
    }
}

impl std::fmt::Display for OpenAIModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.model_id())
    }
}

impl std::str::FromStr for OpenAIModel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "gpt-4o" | "gpt4o" | "4o" => Ok(Self::Gpt4o),
            "o1" => Ok(Self::O1),
            _ => Err(format!(
                "unknown OpenAI model '{s}'. Valid options: gpt-4o, gpt4o, 4o, o1"
            )),
        }
    }
}

/// HTTP client for `OpenAI` API requests
#[derive(Debug, Clone)]
pub struct OpenAIClient {
    /// Reqwest HTTP client
    client: Client,
    /// `OpenAI` API key
    api_key: String,
}

impl OpenAIClient {
    /// Create a new `OpenAI` client with the given API key
    #[must_use = "creates OpenAI client with API key"]
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }

    /// Extract document elements from a page image.
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails or response parsing fails.
    pub async fn extract_page(
        &self,
        model: OpenAIModel,
        page_image: &[u8],
        page_number: u32,
    ) -> Result<LlmExtractionResult> {
        let start = Instant::now();

        // Encode image as base64
        let image_b64 = base64::engine::general_purpose::STANDARD.encode(page_image);
        let image_url = format!("data:image/png;base64,{image_b64}");

        // Build the prompt
        let system_prompt = EXTRACTION_PROMPT;

        let messages = vec![Message {
            role: "user".to_string(),
            content: vec![
                Content::Text {
                    r#type: "text".to_string(),
                    text: format!(
                        "{system_prompt}\n\nExtract all content from page {page_number}. Return valid JSON only."
                    ),
                },
                Content::Image {
                    r#type: "image_url".to_string(),
                    image_url: ImageUrl {
                        url: image_url,
                        detail: "high".to_string(),
                    },
                },
            ],
        }];

        // o1 uses max_completion_tokens, GPT-4o uses max_tokens
        let (max_tokens, max_completion_tokens) = match model {
            OpenAIModel::O1 => (None, Some(4096)),
            OpenAIModel::Gpt4o => (Some(4096), None),
        };

        // o1 doesn't support response_format or temperature
        let (temperature, response_format) = match model {
            OpenAIModel::O1 => (1.0, None), // o1 requires temperature=1
            OpenAIModel::Gpt4o => (
                0.0,
                Some(ResponseFormat {
                    r#type: "json_object".to_string(),
                }),
            ),
        };

        let request = ChatRequest {
            model: model.model_id().to_string(),
            messages,
            max_tokens,
            max_completion_tokens,
            temperature,
            response_format,
        };

        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send request to OpenAI")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("OpenAI API error ({status}): {error_text}");
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .context("Failed to parse OpenAI response")?;

        let latency_ms = start.elapsed().as_millis() as u64;

        // Parse the extraction response - handle markdown-wrapped JSON (o1 doesn't use json_object)
        let content = &chat_response.choices[0].message.content;
        let json_content = extract_json(content);
        let extraction_response: ExtractionResponse =
            serde_json::from_str(&json_content).context("Failed to parse extraction JSON")?;

        // Convert to our types
        let extraction = convert_extraction(extraction_response);
        let cost = model.calculate_cost(
            chat_response.usage.prompt_tokens,
            chat_response.usage.completion_tokens,
        );

        Ok(LlmExtractionResult {
            model: model.model_id().to_string(),
            page_number,
            extraction,
            input_tokens: chat_response.usage.prompt_tokens,
            output_tokens: chat_response.usage.completion_tokens,
            cost_usd: cost,
            latency_ms,
        })
    }
}

fn convert_extraction(response: ExtractionResponse) -> PageExtraction {
    let elements = response
        .elements
        .into_iter()
        .map(|e| {
            let label = parse_label(&e.label);
            ExtractedElement {
                label,
                text: e.text,
                bbox: e.bbox.map(|b| super::BBox {
                    l: b.l,
                    t: b.t,
                    r: b.r,
                    b: b.b,
                }),
                confidence: e.confidence,
                table_data: e.table_data.map(|t| super::TableData {
                    rows: t.rows,
                    num_rows: t.num_rows,
                    num_cols: t.num_cols,
                }),
            }
        })
        .collect();

    PageExtraction {
        page_number: response.page_number,
        elements,
        reading_order: response.reading_order,
    }
}

/// Extract JSON from response, handling markdown code blocks.
fn extract_json(text: &str) -> String {
    let text = text.trim();

    // Handle ```json ... ``` wrapper
    if text.starts_with("```") {
        // Find the end of the first line (e.g., "```json\n")
        if let Some(start) = text.find('\n') {
            let after_first_line = &text[start + 1..];
            // Find closing ```
            if let Some(end) = after_first_line.rfind("```") {
                return after_first_line[..end].trim().to_string();
            }
        }
    }

    // Try to find JSON object directly
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            return text[start..=end].to_string();
        }
    }

    text.to_string()
}

fn parse_label(s: &str) -> DocItemLabel {
    match s.to_lowercase().as_str() {
        "title" => DocItemLabel::Title,
        "section_header" | "header" | "heading" => DocItemLabel::SectionHeader,
        "paragraph" | "para" => DocItemLabel::Paragraph,
        "list_item" | "list" | "bullet" => DocItemLabel::ListItem,
        "table" => DocItemLabel::Table,
        "picture" | "image" | "figure" => DocItemLabel::Picture,
        "caption" => DocItemLabel::Caption,
        "footnote" => DocItemLabel::Footnote,
        "formula" | "equation" | "math" => DocItemLabel::Formula,
        "page_header" => DocItemLabel::PageHeader,
        "page_footer" | "footer" => DocItemLabel::PageFooter,
        "code" => DocItemLabel::Code,
        "checkbox" => DocItemLabel::Checkbox,
        "reference" | "ref" | "citation" => DocItemLabel::Reference,
        _ => DocItemLabel::Text, // Default: text and others
    }
}

const EXTRACTION_PROMPT: &str = r#"You are an expert document extraction system. Extract ALL content from this document page with high precision.

OUTPUT JSON SCHEMA:
{
  "page_number": <int>,
  "elements": [
    {
      "label": "<title|section_header|paragraph|list_item|table|picture|caption|footnote|formula|page_header|page_footer|code|text>",
      "text": "<exact text content>",
      "bbox": {"l": <left>, "t": <top>, "r": <right>, "b": <bottom>} or null,
      "confidence": <0.0-1.0>,
      "table_data": {
        "rows": [["cell1", "cell2"], ["cell3", "cell4"]],
        "num_rows": <int>,
        "num_cols": <int>
      } // only for tables, null otherwise
    }
  ],
  "reading_order": [0, 1, 2, ...]
}

EXTRACTION RULES:
1. Extract EVERY text element visible on the page
2. Preserve exact spelling, punctuation, and spacing
3. For tables: extract ALL cells, preserve row/column structure
4. For figures/pictures: describe visual content briefly in text field
5. Bounding boxes should be relative coordinates (0-100 scale for both width and height)
6. Confidence: 1.0 = certain, 0.5 = uncertain (OCR artifacts, blurry text)
7. Reading order: natural document flow (left-to-right, top-to-bottom for Western text)
8. Label classification:
   - title: Main document title, usually largest text at top
   - section_header: Section/subsection headings (numbered or not)
   - paragraph: Body text paragraphs
   - list_item: Bulleted or numbered list items
   - table: Tabular data (must include table_data)
   - picture: Images, diagrams, charts
   - caption: Figure or table captions
   - footnote: Bottom-of-page references
   - formula: Mathematical equations
   - page_header/page_footer: Repeating header/footer content
   - code: Programming code blocks
   - text: Generic text that doesn't fit other categories

Return ONLY valid JSON. No markdown, no explanation."#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_model_display() {
        assert_eq!(format!("{}", OpenAIModel::Gpt4o), "gpt-4o");
        assert_eq!(format!("{}", OpenAIModel::O1), "o1");
    }

    #[test]
    fn test_openai_model_costs() {
        let (input, output) = OpenAIModel::Gpt4o.cost_per_million();
        assert!(input > 0.0);
        assert!(output > 0.0);
    }

    #[test]
    fn test_openai_model_from_str() {
        // Primary names
        assert_eq!("gpt-4o".parse::<OpenAIModel>().unwrap(), OpenAIModel::Gpt4o);
        assert_eq!("o1".parse::<OpenAIModel>().unwrap(), OpenAIModel::O1);

        // Short aliases
        assert_eq!("gpt4o".parse::<OpenAIModel>().unwrap(), OpenAIModel::Gpt4o);
        assert_eq!("4o".parse::<OpenAIModel>().unwrap(), OpenAIModel::Gpt4o);

        // Case insensitive
        assert_eq!("GPT-4O".parse::<OpenAIModel>().unwrap(), OpenAIModel::Gpt4o);
        assert_eq!("O1".parse::<OpenAIModel>().unwrap(), OpenAIModel::O1);

        // Invalid input
        assert!("claude".parse::<OpenAIModel>().is_err());
        assert!("invalid".parse::<OpenAIModel>().is_err());
    }

    #[test]
    fn test_openai_model_roundtrip() {
        // Round-trip test: display -> parse -> display
        for model in [OpenAIModel::Gpt4o, OpenAIModel::O1] {
            let display = model.model_id();
            let parsed: OpenAIModel = display.parse().unwrap();
            assert_eq!(parsed, model, "round-trip failed for {display}");
        }
    }
}
