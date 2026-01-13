//! # docling-llm-verify
//!
//! LLM ensemble verification library for PDF extraction quality assessment.
//!
//! This crate provides tools to use multiple Large Language Models (LLMs) to create
//! high-confidence ground truth for PDF document extraction, and compare Rust extraction
//! output against this ground truth.
//!
//! ## Overview
//!
//! The verification workflow:
//! 1. Render PDF pages to PNG images
//! 2. Send each page image to multiple LLMs for extraction
//! 3. Merge extractions using an ensemble algorithm (IoU-based alignment, majority voting)
//! 4. Compare Rust docling output against the ensemble ground truth
//! 5. Generate detailed comparison reports
//!
//! ## Supported LLM Providers
//!
//! - **`OpenAI`**: GPT-4o (vision), o1 (reasoning with vision)
//! - **AWS Bedrock**: Claude Opus 4.5, Claude Sonnet 3.5 v2
//!
//! ## Example Usage
//!
//! ```no_run
//! use docling_llm_verify::{
//!     PdfRenderer, merge_extractions, compare_outputs, generate_report,
//!     models::openai::{OpenAIClient, OpenAIModel},
//! };
//! use std::path::Path;
//!
//! # async fn example() -> anyhow::Result<()> {
//! // 1. Render PDF pages
//! let renderer = PdfRenderer::new()?;
//! let pages = renderer.render_pages(Path::new("document.pdf"), 150)?;
//!
//! // 2. Extract with multiple LLMs
//! let client = OpenAIClient::new(std::env::var("OPENAI_API_KEY")?);
//! let mut all_extractions = Vec::new();
//!
//! for page in &pages {
//!     let gpt4o = client.extract_page(
//!         OpenAIModel::Gpt4o,
//!         &page.png_data,
//!         page.page_number
//!     ).await?;
//!     all_extractions.push(gpt4o);
//! }
//!
//! // 3. Merge into ground truth
//! let ground_truth = merge_extractions(&all_extractions);
//!
//! // 4. Compare against Rust output
//! // let comparison = compare_outputs(&document_gt, &rust_markdown);
//!
//! // 5. Generate report
//! // let report = generate_report(&[comparison]);
//! # Ok(())
//! # }
//! ```
//!
//! ## Modules
//!
//! - [`ensemble`] - Merging algorithm for combining multiple LLM extractions
//! - [`models`] - Data types and LLM client implementations
//! - [`output`] - Comparison, reporting, and ground truth persistence
//! - [`pdf`] - PDF rendering to PNG images using pdfium
//!
//! ## Element Labels
//!
//! The extraction uses Docling's `DocItem` label taxonomy:
//! - `Title` - Main document title
//! - `SectionHeader` - Section/subsection headings
//! - `Paragraph` - Body text paragraphs
//! - `ListItem` - Bulleted or numbered list items
//! - `Table` - Tabular data with row/column structure
//! - `Picture` - Images, diagrams, charts
//! - `Caption` - Figure or table captions
//! - `Footnote` - Bottom-of-page references
//! - `Formula` - Mathematical equations
//! - `Code` - Programming code blocks
//!
//! ## Cost Estimation
//!
//! The crate tracks token usage and costs per LLM provider:
//! - GPT-4o: ~$2.50/1M input, ~$10/1M output tokens
//! - o1: ~$15/1M input, ~$60/1M output tokens
//! - Claude Opus 4.5: ~$15/1M input, ~$75/1M output tokens
//! - Claude Sonnet 3.5 v2: ~$3/1M input, ~$15/1M output tokens
//!
//! Typical cost per PDF page: $0.01 - $0.05 depending on model and page complexity.

pub mod ensemble;
pub mod models;
pub mod output;
pub mod pdf;

pub use ensemble::merge_extractions;
pub use models::*;
pub use output::{compare_outputs, generate_markdown, generate_report, save_ground_truth};
pub use pdf::PdfRenderer;
