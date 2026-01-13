//! AWS Bedrock client for Claude models.
//!
//! This module provides an async client for AWS Bedrock's Claude models
//! to extract document elements from PDF page images.
//!
//! ## Supported Models
//!
//! - **Claude Opus 4.5**: Highest quality, best for complex documents
//! - **Claude Sonnet 3.5 v2**: Good balance of speed and quality
//!
//! ## Authentication
//!
//! Uses default AWS credentials chain:
//! 1. Environment variables (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`)
//! 2. AWS credentials file (`~/.aws/credentials`)
//! 3. IAM role (for EC2/Lambda)
//!
//! ## Example
//!
//! ```no_run
//! use docling_llm_verify::models::bedrock::{BedrockClient, ClaudeModel};
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Use default region from AWS config
//! let client = BedrockClient::new().await?;
//!
//! // Or specify a region
//! let client = BedrockClient::new_with_region("us-west-2").await?;
//!
//! // Extract from a page image
//! let page_png: &[u8] = &[]; // Your PNG data
//! let result = client.extract_page(ClaudeModel::ClaudeSonnet35V2, page_png, 1).await?;
//!
//! println!("Model: {}", result.model);
//! println!("Elements: {}", result.extraction.elements.len());
//! println!("Cost: ${:.4}", result.cost_usd);
//! # Ok(())
//! # }
//! ```
//!
//! ## Cost Information
//!
//! | Model | Input (per 1M tokens) | Output (per 1M tokens) |
//! |-------|----------------------|------------------------|
//! | Claude Opus 4.5 | $15.00 | $75.00 |
//! | Claude Sonnet 3.5 v2 | $3.00 | $15.00 |
//!
//! ## Model IDs
//!
//! - Opus 4.5: `global.anthropic.claude-opus-4-5-20251101-v1:0` (global inference)
//! - Sonnet 3.5 v2: `us.anthropic.claude-3-5-sonnet-20241022-v2:0` (US inference)

// Clippy pedantic allows:
// - Timestamp and token count calculations
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]

use anyhow::{anyhow, Context, Result};
use aws_sdk_bedrockruntime::{
    primitives::Blob,
    types::{ContentBlock, ConversationRole, ImageBlock, ImageFormat, ImageSource, Message},
    Client,
};
use serde::Deserialize;
use std::time::Instant;

use super::{DocItemLabel, ExtractedElement, LlmExtractionResult, PageExtraction};

/// Claude model variants available on Bedrock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClaudeModel {
    /// Claude Opus 4.5 - Best for document understanding
    ClaudeOpus45,
    /// Claude Sonnet 3.5 v2 - Good balance of speed/quality
    ClaudeSonnet35V2,
}

impl ClaudeModel {
    /// Get the AWS Bedrock model identifier string
    #[inline]
    #[must_use = "returns AWS Bedrock model identifier"]
    pub const fn model_id(&self) -> &str {
        match self {
            // Use global inference profile for Opus 4.5
            Self::ClaudeOpus45 => "global.anthropic.claude-opus-4-5-20251101-v1:0",
            // Use US inference profile for Sonnet 3.5 v2
            Self::ClaudeSonnet35V2 => "us.anthropic.claude-3-5-sonnet-20241022-v2:0",
        }
    }

    /// Get the human-readable model name for display purposes
    #[inline]
    #[must_use = "returns human-readable model name"]
    pub const fn display_name(&self) -> &str {
        match self {
            Self::ClaudeOpus45 => "claude-opus-4.5",
            Self::ClaudeSonnet35V2 => "claude-sonnet-3.5-v2",
        }
    }

    /// Cost per 1M tokens (input, output) in USD.
    #[inline]
    #[must_use = "returns input/output token costs"]
    pub const fn cost_per_million(&self) -> (f64, f64) {
        match self {
            Self::ClaudeOpus45 => (15.00, 75.00),
            Self::ClaudeSonnet35V2 => (3.00, 15.00),
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

impl std::fmt::Display for ClaudeModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

impl std::str::FromStr for ClaudeModel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "claude-opus-4.5" | "opus-4.5" | "opus45" | "opus" => Ok(Self::ClaudeOpus45),
            "claude-sonnet-3.5-v2" | "sonnet-3.5-v2" | "sonnet35v2" | "sonnet" => {
                Ok(Self::ClaudeSonnet35V2)
            }
            _ => Err(format!(
                "unknown Claude model '{s}'. Valid options: claude-opus-4.5, opus, claude-sonnet-3.5-v2, sonnet"
            )),
        }
    }
}

/// AWS Bedrock client for Claude models.
#[derive(Debug, Clone)]
pub struct BedrockClient {
    client: Client,
}

impl BedrockClient {
    /// Create a new Bedrock client using default AWS credentials.
    ///
    /// # Errors
    ///
    /// Returns an error if AWS credentials cannot be loaded.
    pub async fn new() -> Result<Self> {
        let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let client = Client::new(&config);
        Ok(Self { client })
    }

    /// Create a new Bedrock client for a specific region.
    ///
    /// # Errors
    ///
    /// Returns an error if AWS credentials cannot be loaded or region is invalid.
    pub async fn new_with_region(region: &str) -> Result<Self> {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(region.to_string()))
            .load()
            .await;
        let client = Client::new(&config);
        Ok(Self { client })
    }

    /// Extract document elements from a page image using Claude.
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails or response parsing fails.
    pub async fn extract_page(
        &self,
        model: ClaudeModel,
        page_image: &[u8],
        page_number: u32,
    ) -> Result<LlmExtractionResult> {
        let start = Instant::now();

        // Build the message with image
        let image_block = ImageBlock::builder()
            .format(ImageFormat::Png)
            .source(ImageSource::Bytes(Blob::new(page_image.to_vec())))
            .build()
            .context("Failed to build image block")?;

        let prompt_text = format!(
            "{EXTRACTION_PROMPT}\n\nExtract all content from page {page_number}. Return valid JSON only."
        );

        let message = Message::builder()
            .role(ConversationRole::User)
            .content(ContentBlock::Image(image_block))
            .content(ContentBlock::Text(prompt_text))
            .build()
            .context("Failed to build message")?;

        // Call the Converse API
        let response = self
            .client
            .converse()
            .model_id(model.model_id())
            .messages(message)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Bedrock API error: {e:?}"))?;

        let latency_ms = start.elapsed().as_millis() as u64;

        // Extract the text response
        let output = response.output().context("No output in response")?;
        let message = output
            .as_message()
            .map_err(|_| anyhow!("Output is not a message"))?;
        let content = message.content();

        let text_content = content
            .iter()
            .find_map(|block| {
                if let ContentBlock::Text(text) = block {
                    Some(text.clone())
                } else {
                    None
                }
            })
            .context("No text content in response")?;

        // Parse the extraction response - handle markdown-wrapped JSON
        let json_content = extract_json(&text_content);
        let extraction_response: ExtractionResponse =
            serde_json::from_str(&json_content).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to parse extraction JSON from Claude: {}. First 500 chars: {}",
                    e,
                    &json_content[..json_content.len().min(500)]
                )
            })?;

        // Get token usage
        let usage = response.usage().context("No usage info")?;
        let input_tokens = usage.input_tokens() as u32;
        let output_tokens = usage.output_tokens() as u32;

        // Convert to our types
        let extraction = convert_extraction(extraction_response, page_number);
        let cost = model.calculate_cost(input_tokens, output_tokens);

        Ok(LlmExtractionResult {
            model: model.display_name().to_string(),
            page_number,
            extraction,
            input_tokens,
            output_tokens,
            cost_usd: cost,
            latency_ms,
        })
    }
}

/// LLM response format for page extraction.
#[derive(Debug, Deserialize)]
struct ExtractionResponse {
    #[serde(default)]
    page_number: u32,
    elements: Vec<ElementResponse>,
    #[serde(default)]
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

fn convert_extraction(response: ExtractionResponse, page_number: u32) -> PageExtraction {
    let elements: Vec<ExtractedElement> = response
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

    let reading_order = if response.reading_order.is_empty() {
        (0..elements.len()).collect()
    } else {
        response.reading_order
    };

    PageExtraction {
        page_number: if response.page_number > 0 {
            response.page_number
        } else {
            page_number
        },
        elements,
        reading_order,
    }
}

/// Extract JSON from Claude's response, handling markdown code blocks.
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
    fn test_parse_label() {
        assert_eq!(parse_label("title"), DocItemLabel::Title);
        assert_eq!(parse_label("TITLE"), DocItemLabel::Title);
        assert_eq!(parse_label("section_header"), DocItemLabel::SectionHeader);
        assert_eq!(parse_label("heading"), DocItemLabel::SectionHeader);
        assert_eq!(parse_label("paragraph"), DocItemLabel::Paragraph);
        assert_eq!(parse_label("unknown"), DocItemLabel::Text);
    }

    #[test]
    fn test_model_ids() {
        assert!(ClaudeModel::ClaudeOpus45.model_id().contains("opus"));
        assert!(ClaudeModel::ClaudeSonnet35V2.model_id().contains("sonnet"));
    }

    #[test]
    fn test_claude_model_display() {
        assert_eq!(format!("{}", ClaudeModel::ClaudeOpus45), "claude-opus-4.5");
        assert_eq!(
            format!("{}", ClaudeModel::ClaudeSonnet35V2),
            "claude-sonnet-3.5-v2"
        );
    }

    #[test]
    fn test_claude_model_from_str() {
        // Primary names
        assert_eq!(
            "claude-opus-4.5".parse::<ClaudeModel>().unwrap(),
            ClaudeModel::ClaudeOpus45
        );
        assert_eq!(
            "claude-sonnet-3.5-v2".parse::<ClaudeModel>().unwrap(),
            ClaudeModel::ClaudeSonnet35V2
        );

        // Short aliases
        assert_eq!(
            "opus".parse::<ClaudeModel>().unwrap(),
            ClaudeModel::ClaudeOpus45
        );
        assert_eq!(
            "sonnet".parse::<ClaudeModel>().unwrap(),
            ClaudeModel::ClaudeSonnet35V2
        );
        assert_eq!(
            "opus-4.5".parse::<ClaudeModel>().unwrap(),
            ClaudeModel::ClaudeOpus45
        );
        assert_eq!(
            "sonnet-3.5-v2".parse::<ClaudeModel>().unwrap(),
            ClaudeModel::ClaudeSonnet35V2
        );
        assert_eq!(
            "opus45".parse::<ClaudeModel>().unwrap(),
            ClaudeModel::ClaudeOpus45
        );
        assert_eq!(
            "sonnet35v2".parse::<ClaudeModel>().unwrap(),
            ClaudeModel::ClaudeSonnet35V2
        );

        // Case insensitive
        assert_eq!(
            "OPUS".parse::<ClaudeModel>().unwrap(),
            ClaudeModel::ClaudeOpus45
        );
        assert_eq!(
            "Claude-Opus-4.5".parse::<ClaudeModel>().unwrap(),
            ClaudeModel::ClaudeOpus45
        );

        // Invalid input
        assert!("gpt-4".parse::<ClaudeModel>().is_err());
        assert!("invalid".parse::<ClaudeModel>().is_err());
    }

    #[test]
    fn test_claude_model_roundtrip() {
        // Round-trip test: display -> parse -> display
        for model in [ClaudeModel::ClaudeOpus45, ClaudeModel::ClaudeSonnet35V2] {
            let display = model.display_name();
            let parsed: ClaudeModel = display.parse().unwrap();
            assert_eq!(parsed, model, "round-trip failed for {display}");
        }
    }
}
