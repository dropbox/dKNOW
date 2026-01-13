//! LLM Ensemble PDF Verification CLI
//!
//! Extract ground truth from PDFs using ensemble of vision models.

// Clippy pedantic allows:
// - Cost and timing calculations use f64 from usize/u64
#![allow(clippy::cast_precision_loss)]

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use docling_llm_verify::{
    merge_extractions,
    models::{
        bedrock::{BedrockClient, ClaudeModel},
        openai::{OpenAIClient, OpenAIModel},
        ModelCost, PdfCostReport,
    },
    output, DocumentGroundTruth, LlmExtractionResult, PdfRenderer,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Available LLM providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ValueEnum)]
enum Provider {
    /// Claude via AWS Bedrock
    Claude,
    /// `OpenAI` GPT-4o
    Gpt4o,
    /// `OpenAI` o1
    O1,
    /// All providers (Claude + GPT-4o + o1)
    All,
}

#[derive(Parser)]
#[command(name = "docling-llm-verify")]
#[command(about = "LLM ensemble verification for PDF extraction quality")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Extract ground truth from a single PDF
    Extract {
        /// Path to PDF file
        #[arg(short, long)]
        pdf: PathBuf,

        /// Output directory for ground truth files
        #[arg(short, long)]
        output: PathBuf,

        /// DPI for rendering (default: 150)
        #[arg(long, default_value = "150")]
        dpi: u32,

        /// LLM providers to use (default: all)
        #[arg(long, value_enum, default_value = "all")]
        providers: Provider,
    },

    /// Extract ground truth from all PDFs in a directory
    ExtractAll {
        /// Directory containing PDF files
        #[arg(short, long)]
        input_dir: PathBuf,

        /// Output directory for ground truth files
        #[arg(short, long)]
        output_dir: PathBuf,

        /// DPI for rendering (default: 150)
        #[arg(long, default_value = "150")]
        dpi: u32,
    },

    /// Compare Rust output against ground truth
    Compare {
        /// Path to ground truth JSON
        #[arg(long)]
        ground_truth: PathBuf,

        /// Path to Rust markdown output
        #[arg(long)]
        rust_output: PathBuf,
    },

    /// Show cost estimate for extraction
    Estimate {
        /// Path to PDF file or directory
        #[arg(short, long)]
        path: PathBuf,

        /// DPI for rendering (default: 150)
        #[arg(long, default_value = "150")]
        dpi: u32,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env().add_directive(
                "docling_llm_verify=info"
                    .parse()
                    .expect("directive is compile-time constant"),
            ),
        )
        .init();

    let args = Args::parse();

    match args.command {
        Command::Extract {
            pdf,
            output,
            dpi,
            providers,
        } => {
            extract_single(&pdf, &output, dpi, providers).await?;
        }
        Command::ExtractAll {
            input_dir,
            output_dir,
            dpi,
        } => {
            extract_all(&input_dir, &output_dir, dpi).await?;
        }
        Command::Compare {
            ground_truth,
            rust_output,
        } => {
            compare(&ground_truth, &rust_output)?;
        }
        Command::Estimate { path, dpi } => {
            estimate(&path, dpi)?;
        }
    }

    Ok(())
}

/// Clients initialized for extraction
#[derive(Debug, Clone)]
struct ExtractorClients {
    openai: Option<OpenAIClient>,
    bedrock: Option<BedrockClient>,
    use_gpt4o: bool,
    use_o1: bool,
}

/// Initialize extraction clients based on provider selection
async fn init_clients(providers: Provider) -> Result<ExtractorClients> {
    let use_claude = matches!(providers, Provider::Claude | Provider::All);
    let use_gpt4o = matches!(providers, Provider::Gpt4o | Provider::All);
    let use_o1 = matches!(providers, Provider::O1 | Provider::All);

    let openai = if use_gpt4o || use_o1 {
        let api_key =
            std::env::var("OPENAI_API_KEY").context("OPENAI_API_KEY not set. Run: source .env")?;
        Some(OpenAIClient::new(api_key))
    } else {
        None
    };

    let bedrock = if use_claude {
        Some(BedrockClient::new().await?)
    } else {
        None
    };

    Ok(ExtractorClients {
        openai,
        bedrock,
        use_gpt4o,
        use_o1,
    })
}

/// Get provider description string
const fn provider_description(providers: Provider) -> &'static str {
    match providers {
        Provider::Claude => "Claude Opus 4.5",
        Provider::Gpt4o => "GPT-4o",
        Provider::O1 => "o1",
        Provider::All => "Claude Opus 4.5 + GPT-4o + o1",
    }
}

/// Extract from page using Claude Opus 4.5
async fn extract_with_claude(
    client: &BedrockClient,
    png_data: &[u8],
    page_number: u32,
    extractions: &mut Vec<LlmExtractionResult>,
    total_cost: &mut f64,
    model_costs: &mut HashMap<String, ModelCost>,
) {
    match client
        .extract_page(ClaudeModel::ClaudeOpus45, png_data, page_number)
        .await
    {
        Ok(result) => {
            info!(
                "  Claude Opus 4.5: {} elements, {:.3}s, ${:.4}",
                result.extraction.elements.len(),
                result.latency_ms as f64 / 1000.0,
                result.cost_usd
            );
            *total_cost += result.cost_usd;
            update_model_cost(model_costs, &result);
            extractions.push(result);
        }
        Err(e) => warn!("  Claude Opus 4.5 failed: {}", e),
    }
}

/// Extract from page using GPT-4o or o1
#[allow(
    clippy::too_many_arguments,
    reason = "LLM extraction requires client, model, image, and tracking state"
)]
async fn extract_with_openai(
    client: &OpenAIClient,
    model: OpenAIModel,
    model_name: &str,
    png_data: &[u8],
    page_number: u32,
    extractions: &mut Vec<LlmExtractionResult>,
    total_cost: &mut f64,
    model_costs: &mut HashMap<String, ModelCost>,
) {
    match client.extract_page(model, png_data, page_number).await {
        Ok(result) => {
            info!(
                "  {}: {} elements, {:.3}s, ${:.4}",
                model_name,
                result.extraction.elements.len(),
                result.latency_ms as f64 / 1000.0,
                result.cost_usd
            );
            *total_cost += result.cost_usd;
            update_model_cost(model_costs, &result);
            extractions.push(result);
        }
        Err(e) => warn!("  {} failed: {}", model_name, e),
    }
}

/// Build document ground truth from page extractions
fn build_document_ground_truth(
    filename: String,
    all_page_gt: Vec<docling_llm_verify::PageGroundTruth>,
) -> DocumentGroundTruth {
    let total_elements: usize = all_page_gt.iter().map(|p| p.elements.len()).sum();
    let all_scores: Vec<f64> = all_page_gt
        .iter()
        .flat_map(|p| &p.agreement_scores)
        .copied()
        .collect();
    let avg_confidence = if all_scores.is_empty() {
        0.0
    } else {
        all_scores.iter().sum::<f64>() / all_scores.len() as f64
    };

    DocumentGroundTruth {
        filename,
        pages: all_page_gt,
        total_elements,
        avg_confidence,
    }
}

#[allow(clippy::future_not_send)] // PdfRenderer uses pdfium which is not Send
async fn extract_single(
    pdf_path: &Path,
    output_dir: &Path,
    dpi: u32,
    providers: Provider,
) -> Result<()> {
    let clients = init_clients(providers).await?;
    let renderer = PdfRenderer::new()?;
    let filename = pdf_path.file_name().map_or_else(
        || "unknown.pdf".to_string(),
        |n| n.to_string_lossy().to_string(),
    );

    info!("Processing: {}", filename);
    info!("Using providers: {}", provider_description(providers));

    let pages = renderer.render_pages(pdf_path, dpi)?;
    info!("Rendered {} pages at {} DPI", pages.len(), dpi);

    let mut all_page_gt = Vec::new();
    let mut total_cost = 0.0;
    let mut model_costs: HashMap<String, ModelCost> = HashMap::new();

    for page in &pages {
        info!(
            "Extracting page {} ({} KB)",
            page.page_number,
            page.size() / 1024
        );
        let mut extractions: Vec<LlmExtractionResult> = Vec::new();

        if let Some(ref client) = clients.bedrock {
            extract_with_claude(
                client,
                &page.png_data,
                page.page_number,
                &mut extractions,
                &mut total_cost,
                &mut model_costs,
            )
            .await;
        }

        if let Some(ref client) = clients.openai {
            if clients.use_gpt4o {
                extract_with_openai(
                    client,
                    OpenAIModel::Gpt4o,
                    "GPT-4o",
                    &page.png_data,
                    page.page_number,
                    &mut extractions,
                    &mut total_cost,
                    &mut model_costs,
                )
                .await;
            }
            if clients.use_o1 {
                extract_with_openai(
                    client,
                    OpenAIModel::O1,
                    "o1",
                    &page.png_data,
                    page.page_number,
                    &mut extractions,
                    &mut total_cost,
                    &mut model_costs,
                )
                .await;
            }
        }

        let page_gt = merge_extractions(&extractions);
        info!(
            "  Merged: {} elements, avg agreement: {:.1}%",
            page_gt.elements.len(),
            page_gt.agreement_scores.iter().sum::<f64>()
                / page_gt.agreement_scores.len().max(1) as f64
                * 100.0
        );
        all_page_gt.push(page_gt);
    }

    let ground_truth = build_document_ground_truth(filename.clone(), all_page_gt);

    let pdf_output_dir = output_dir.join(pdf_path.file_stem().unwrap_or_default());
    output::save_ground_truth(&ground_truth, &pdf_output_dir)?;

    let cost_report = PdfCostReport {
        pdf_name: filename,
        num_pages: pages.len(),
        model_costs,
        total_cost_usd: total_cost,
    };
    output::save_cost_report(&cost_report, &pdf_output_dir.join("cost.json"))?;

    info!("Saved ground truth to: {:?}", pdf_output_dir);
    info!("Total cost: ${:.4}", total_cost);

    Ok(())
}

#[allow(clippy::future_not_send)] // Uses extract_single which contains non-Send PdfRenderer
async fn extract_all(input_dir: &Path, output_dir: &Path, dpi: u32) -> Result<()> {
    let mut total_cost = 0.0;

    // Find all PDFs
    let pdfs: Vec<PathBuf> = std::fs::read_dir(input_dir)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "pdf"))
        .collect();

    info!("Found {} PDFs", pdfs.len());

    for pdf_path in &pdfs {
        let pdf_name = pdf_path
            .file_stem()
            .map(|n| n.to_string_lossy())
            .unwrap_or_default();
        let pdf_output = output_dir.join(&*pdf_name);

        info!("\n=== Processing: {} ===", pdf_name);

        match extract_single(pdf_path, output_dir, dpi, Provider::All).await {
            Ok(()) => {
                // Read cost from saved file
                if let Ok(cost_json) = std::fs::read_to_string(pdf_output.join("cost.json")) {
                    if let Ok(cost) = serde_json::from_str::<PdfCostReport>(&cost_json) {
                        total_cost += cost.total_cost_usd;
                    }
                }
            }
            Err(e) => {
                warn!("Failed to process {}: {}", pdf_name, e);
            }
        }
    }

    info!("\n=== Complete ===");
    info!("Processed {} PDFs", pdfs.len());
    info!("Total cost: ${:.2}", total_cost);

    Ok(())
}

fn compare(ground_truth_path: &PathBuf, rust_output_path: &PathBuf) -> Result<()> {
    let gt_json = std::fs::read_to_string(ground_truth_path)?;
    let ground_truth: DocumentGroundTruth = serde_json::from_str(&gt_json)?;

    let rust_output = std::fs::read_to_string(rust_output_path)?;

    let result = output::compare_outputs(&ground_truth, &rust_output);

    println!("Comparison Results for: {}", result.filename);
    println!("  Accuracy: {:.1}%", result.accuracy_percent);
    println!("  Text Similarity: {:.1}%", result.text_similarity);

    if !result.missing_elements.is_empty() {
        println!("\nMissing Elements:");
        for elem in &result.missing_elements {
            println!("  - {elem}");
        }
    }

    Ok(())
}

fn estimate(path: &PathBuf, _dpi: u32) -> Result<()> {
    let renderer = PdfRenderer::new()?;

    let pdfs: Vec<PathBuf> = if path.is_dir() {
        std::fs::read_dir(path)?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "pdf"))
            .collect()
    } else {
        vec![path.clone()]
    };

    let mut total_pages = 0;
    println!("Cost Estimate (GPT-4o + o1):\n");
    println!("| File | Pages | Est. Cost |");
    println!("|------|-------|-----------|");

    for pdf in &pdfs {
        let page_count = renderer.page_count(pdf)?;
        total_pages += page_count;

        // Estimate: ~2000 input tokens/page (image), ~500 output tokens/page
        // GPT-4o: $2.50/1M in, $10/1M out
        // o1: $15/1M in, $60/1M out
        let gpt4o_cost = (page_count as f64 * 2000.0 * 2.50 / 1_000_000.0)
            + (page_count as f64 * 500.0 * 10.0 / 1_000_000.0);
        let o1_cost = (page_count as f64 * 2000.0 * 15.0 / 1_000_000.0)
            + (page_count as f64 * 500.0 * 60.0 / 1_000_000.0);
        let total = gpt4o_cost + o1_cost;

        println!(
            "| {} | {} | ${:.2} |",
            pdf.file_name()
                .map(|n| n.to_string_lossy())
                .unwrap_or_default(),
            page_count,
            total
        );
    }

    let total_gpt4o = (total_pages as f64 * 2000.0 * 2.50 / 1_000_000.0)
        + (total_pages as f64 * 500.0 * 10.0 / 1_000_000.0);
    let total_o1 = (total_pages as f64 * 2000.0 * 15.0 / 1_000_000.0)
        + (total_pages as f64 * 500.0 * 60.0 / 1_000_000.0);

    println!("\n**Total Pages:** {total_pages}");
    println!("**GPT-4o only:** ${total_gpt4o:.2}");
    println!("**GPT-4o + o1:** ${:.2}", total_gpt4o + total_o1);

    Ok(())
}

fn update_model_cost(costs: &mut HashMap<String, ModelCost>, result: &LlmExtractionResult) {
    let entry = costs.entry(result.model.clone()).or_insert(ModelCost {
        input_tokens: 0,
        output_tokens: 0,
        cost_usd: 0.0,
    });
    entry.input_tokens += result.input_tokens;
    entry.output_tokens += result.output_tokens;
    entry.cost_usd += result.cost_usd;
}
