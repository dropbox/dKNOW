//! Visual quality tests using LLM vision comparison
//!
//! These tests compare original documents against parser output visually
//! using OpenAI's vision API. They catch layout, formatting, and rendering
//! issues that text-based comparison cannot detect.
//!
//! # Running Tests
//!
//! ```bash
//! # Set API key
//! export OPENAI_API_KEY="your-api-key-here"
//!
//! # Run all visual tests
//! cargo test --test visual_quality_tests -- --nocapture
//!
//! # Run single test
//! cargo test --test visual_quality_tests test_visual_docx -- --exact --nocapture
//! ```
//!
//! # Requirements
//!
//! - `OPENAI_API_KEY` environment variable
//! - `LibreOffice` (`soffice`) for document-to-PDF and HTML/markdown-to-PDF conversion
//! - `pdftoppm` (from `poppler-utils`) for PDF-to-PNG conversion
//!
//! # Cost
//!
//! Each visual test costs ~$0.01-0.02 (gpt-4o with high-detail images)

use anyhow::Result;
use docling_quality_verifier::{VerificationConfig, VisualQualityReport, VisualTester};
use std::env;
use std::fs;
use std::path::Path;

/// Helper to check if test environment is set up
fn check_visual_test_requirements() -> Result<()> {
    // Check API key
    env::var("OPENAI_API_KEY")
        .map_err(|_| anyhow::anyhow!("OPENAI_API_KEY not set - skipping visual test"))?;

    // Check LibreOffice (used for both document and HTML→PDF conversion)
    which::which("soffice")
        .map_err(|_| anyhow::anyhow!("LibreOffice (soffice) not found - install it"))?;

    // Check pdftoppm (for PDF→PNG conversion)
    which::which("pdftoppm")
        .map_err(|_| anyhow::anyhow!("pdftoppm not found - install poppler-utils"))?;

    Ok(())
}

/// Get path to test corpus (from project root)
fn test_corpus_path(format: &str, filename: &str) -> String {
    // Use absolute path since cargo test working directory varies
    let project_root = std::env::var("CARGO_MANIFEST_DIR")
        .map(|p| {
            Path::new(&p)
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .to_path_buf()
        })
        .unwrap_or_else(|_| std::env::current_dir().unwrap());
    project_root
        .join(format!("test-corpus/{format}/{filename}"))
        .to_str()
        .unwrap()
        .to_string()
}

/// Print visual quality report
fn print_visual_report(report: &VisualQualityReport, format_name: &str) {
    println!("\n=== Visual Quality Report: {format_name} ===");
    println!("Overall Score: {:.1}%", report.overall_score * 100.0);
    println!("  Layout:       {:.1}%", report.layout_score * 100.0);
    println!("  Formatting:   {:.1}%", report.formatting_score * 100.0);
    println!("  Tables:       {:.1}%", report.tables_score * 100.0);
    println!("  Completeness: {:.1}%", report.completeness_score * 100.0);
    println!("  Structure:    {:.1}%", report.structure_score * 100.0);

    if !report.issues.is_empty() {
        println!("\nIssues Found:");
        for issue in &report.issues {
            println!("  - {issue}");
        }
    }

    if !report.strengths.is_empty() {
        println!("\nStrengths:");
        for strength in &report.strengths {
            println!("  + {strength}");
        }
    }

    println!();
}

/// Assert minimum visual quality score
fn assert_visual_quality(report: &VisualQualityReport, min_score: f64, format_name: &str) {
    assert!(
        report.overall_score >= min_score,
        "Visual quality too low for {}: {:.1}% < {:.1}% (required)\nIssues: {:?}",
        format_name,
        report.overall_score * 100.0,
        min_score * 100.0,
        report.issues
    );
}

#[tokio::test]
#[ignore = "Requires OPENAI_API_KEY and external tools"]
async fn test_visual_docx() -> Result<()> {
    // Check requirements
    if let Err(e) = check_visual_test_requirements() {
        eprintln!("Skipping test: {e}");
        return Ok(());
    }

    // Create visual tester
    let config = VerificationConfig {
        model: "gpt-4o".to_string(), // Vision-capable model
        quality_threshold: 0.85,
        detailed_diagnostics: true,
        max_tokens: 4096,
    };
    let tester = VisualTester::new(config)?;

    // Test file: word_sample.docx
    // Use absolute path since cargo test working directory varies
    let project_root = std::env::var("CARGO_MANIFEST_DIR")
        .map(|p| {
            Path::new(&p)
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .to_path_buf()
        })
        .unwrap_or_else(|_| std::env::current_dir().unwrap());
    let docx_path = project_root.join("test-corpus/docx/word_sample.docx");
    let docx_path_str = docx_path.to_str().unwrap();

    if !docx_path.exists() {
        eprintln!("Test corpus not found: {}", docx_path.display());
        eprintln!("Project root: {}", project_root.display());
        eprintln!("Current dir: {:?}", std::env::current_dir());
        eprintln!("Run: cp ~/docling/tests/data/docx/*.docx test-corpus/docx/");
        return Ok(());
    }

    // Verify file actually exists
    assert!(
        docx_path.exists(),
        "Test file must exist: {}",
        docx_path.display()
    );

    println!("Testing DOCX: {}", docx_path.display());

    // Step 1: Convert original DOCX to PDF
    println!("Converting DOCX to PDF...");
    let original_pdf = tester.document_to_pdf(&docx_path)?;
    println!("  Original PDF: {} bytes", original_pdf.len());

    // Step 2: Parse DOCX to markdown using docling-core
    println!("Parsing DOCX to markdown...");
    let markdown = parse_docx_to_markdown(docx_path_str)?;
    println!("  Markdown: {} chars", markdown.len());

    // Step 3: Convert markdown to PDF
    println!("Converting markdown to PDF...");
    let output_pdf = tester.markdown_to_pdf(&markdown)?;
    println!("  Output PDF: {} bytes", output_pdf.len());

    // Step 4: Compare visually using LLM vision
    println!("Comparing PDFs visually with GPT-4o...");
    let report = tester
        .compare_visual_pdfs(&original_pdf, &output_pdf, "DOCX")
        .await?;

    // Print report
    print_visual_report(&report, "DOCX (word_sample.docx)");

    // Assert minimum quality
    assert_visual_quality(&report, 0.75, "DOCX");

    Ok(())
}

#[tokio::test]
#[ignore = "Requires OPENAI_API_KEY and external tools"]
async fn test_visual_pptx() -> Result<()> {
    // Check requirements
    if let Err(e) = check_visual_test_requirements() {
        eprintln!("Skipping test: {e}");
        return Ok(());
    }

    let config = VerificationConfig {
        model: "gpt-4o".to_string(),
        quality_threshold: 0.75, // Lower threshold for PPTX (formatting is harder)
        detailed_diagnostics: true,
        max_tokens: 4096,
    };
    let tester = VisualTester::new(config)?;

    let pptx_path = test_corpus_path("pptx", "powerpoint_sample.pptx");
    if !Path::new(&pptx_path).exists() {
        eprintln!("Test corpus not found: {pptx_path}");
        return Ok(());
    }

    println!("Testing PPTX: {pptx_path}");

    let original_pdf = tester.document_to_pdf(Path::new(&pptx_path))?;
    let markdown = parse_pptx_to_markdown(&pptx_path)?;
    let output_pdf = tester.markdown_to_pdf(&markdown)?;

    let report = tester
        .compare_visual_pdfs(&original_pdf, &output_pdf, "PPTX")
        .await?;

    print_visual_report(&report, "PPTX (powerpoint_sample.pptx)");
    assert_visual_quality(&report, 0.70, "PPTX");

    Ok(())
}

#[tokio::test]
#[ignore = "Requires OPENAI_API_KEY and external tools"]
async fn test_visual_xlsx() -> Result<()> {
    // Check requirements
    if let Err(e) = check_visual_test_requirements() {
        eprintln!("Skipping test: {e}");
        return Ok(());
    }

    let config = VerificationConfig {
        model: "gpt-4o".to_string(),
        quality_threshold: 0.80,
        detailed_diagnostics: true,
        max_tokens: 4096,
    };
    let tester = VisualTester::new(config)?;

    let xlsx_path = test_corpus_path("xlsx", "xlsx_01.xlsx");
    if !Path::new(&xlsx_path).exists() {
        eprintln!("Test corpus not found: {xlsx_path}");
        return Ok(());
    }

    println!("Testing XLSX: {xlsx_path}");

    let original_pdf = tester.document_to_pdf(Path::new(&xlsx_path))?;
    let markdown = parse_xlsx_to_markdown(&xlsx_path)?;
    let output_pdf = tester.markdown_to_pdf(&markdown)?;

    let report = tester
        .compare_visual_pdfs(&original_pdf, &output_pdf, "XLSX")
        .await?;

    print_visual_report(&report, "XLSX (excel_sample.xlsx)");
    assert_visual_quality(&report, 0.75, "XLSX");

    Ok(())
}

#[tokio::test]
#[ignore = "Requires OPENAI_API_KEY and external tools"]
async fn test_visual_html() -> Result<()> {
    // Check requirements
    if let Err(e) = check_visual_test_requirements() {
        eprintln!("Skipping test: {e}");
        return Ok(());
    }

    let config = VerificationConfig {
        model: "gpt-4o".to_string(),
        quality_threshold: 0.85,
        detailed_diagnostics: true,
        max_tokens: 4096,
    };
    let tester = VisualTester::new(config)?;

    let html_path = test_corpus_path("html", "example_01.html");
    if !Path::new(&html_path).exists() {
        eprintln!("Test corpus not found: {html_path}");
        return Ok(());
    }

    println!("Testing HTML: {html_path}");

    // For HTML, convert original HTML to PDF directly
    let original_html = fs::read_to_string(&html_path)?;
    let original_pdf = tester.html_to_pdf(&original_html)?;

    // Parse HTML to markdown
    let markdown = parse_html_to_markdown(&html_path)?;
    let output_pdf = tester.markdown_to_pdf(&markdown)?;

    let report = tester
        .compare_visual_pdfs(&original_pdf, &output_pdf, "HTML")
        .await?;

    print_visual_report(&report, "HTML (example.html)");
    assert_visual_quality(&report, 0.80, "HTML");

    Ok(())
}

// ============================================================================
// Parser Integration Helpers
// ============================================================================

/// Parse DOCX to markdown using docling
fn parse_docx_to_markdown(path: &str) -> Result<String> {
    use std::process::Command;

    // Use cargo run to execute docling binary from docling-cli package
    // --force overwrites existing output files
    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "-p",
            "docling-cli",
            "--bin",
            "docling",
            "--",
            "convert",
            "--force",
            path,
        ])
        .output()?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to parse DOCX: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // CLI writes to file, not stdout - read the output file
    let output_path = Path::new(path).with_extension("md");
    if output_path.exists() {
        Ok(fs::read_to_string(&output_path)?)
    } else {
        // Fall back to stdout if file doesn't exist
        Ok(String::from_utf8(output.stdout)?)
    }
}

/// Parse PPTX to markdown using docling
fn parse_pptx_to_markdown(path: &str) -> Result<String> {
    use std::process::Command;

    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "-p",
            "docling-cli",
            "--bin",
            "docling",
            "--",
            "convert",
            "--force",
            path,
        ])
        .output()?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to parse PPTX: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // CLI writes to file, not stdout - read the output file
    let output_path = Path::new(path).with_extension("md");
    if output_path.exists() {
        Ok(fs::read_to_string(&output_path)?)
    } else {
        Ok(String::from_utf8(output.stdout)?)
    }
}

/// Parse XLSX to markdown using docling
fn parse_xlsx_to_markdown(path: &str) -> Result<String> {
    use std::process::Command;

    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "-p",
            "docling-cli",
            "--bin",
            "docling",
            "--",
            "convert",
            "--force",
            path,
        ])
        .output()?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to parse XLSX: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // CLI writes to file, not stdout - read the output file
    let output_path = Path::new(path).with_extension("md");
    if output_path.exists() {
        Ok(fs::read_to_string(&output_path)?)
    } else {
        Ok(String::from_utf8(output.stdout)?)
    }
}

/// Parse HTML to markdown using docling
fn parse_html_to_markdown(path: &str) -> Result<String> {
    use std::process::Command;

    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "-p",
            "docling-cli",
            "--bin",
            "docling",
            "--",
            "convert",
            "--force",
            path,
        ])
        .output()?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to parse HTML: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // CLI writes to file, not stdout - read the output file
    let output_path = Path::new(path).with_extension("md");
    if output_path.exists() {
        Ok(fs::read_to_string(&output_path)?)
    } else {
        Ok(String::from_utf8(output.stdout)?)
    }
}
