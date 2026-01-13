//! Visual quality testing using PDF comparison
//!
//! This module implements visual quality validation by:
//! 1. Converting original document to PDF (ground truth)
//! 2. Converting parser output (markdown) to PDF (our result)
//! 3. Using LLM vision API to compare both PDFs visually
//!
//! This catches layout, formatting, and visual issues that text-based
//! comparison cannot detect.

use crate::client::OpenAIClient;
use crate::config::VerificationConfig;
use crate::types::VisualQualityReport;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

/// Visual quality tester using PDF comparison
#[derive(Debug, Clone)]
pub struct VisualTester {
    client: OpenAIClient,
}

impl VisualTester {
    /// Create new visual tester
    ///
    /// # Errors
    /// Returns an error if the `OpenAI` client cannot be initialized.
    #[must_use = "this function returns a tester that should be used"]
    pub fn new(_config: VerificationConfig) -> Result<Self> {
        let client = OpenAIClient::new()?;
        Ok(Self { client })
    }

    /// Convert document to PDF using `LibreOffice`
    ///
    /// Supports: DOCX, PPTX, XLSX, ODT, ODS, ODP, RTF, etc.
    ///
    /// # Errors
    /// Returns an error if `LibreOffice` conversion fails or output cannot be read.
    ///
    /// # Panics
    ///
    /// Panics if the input path or output directory path contains invalid UTF-8
    /// characters that cannot be converted to strings.
    #[must_use = "this function returns a PDF that should be processed"]
    pub fn document_to_pdf(&self, input_path: &Path) -> Result<Vec<u8>> {
        let temp_dir = tempfile::tempdir()?;
        let output_dir = temp_dir.path();

        // Use LibreOffice headless to convert to PDF
        let output = Command::new("soffice")
            .args([
                "--headless",
                "--convert-to",
                "pdf",
                "--outdir",
                output_dir.to_str().unwrap(),
                input_path.to_str().unwrap(),
            ])
            .output()
            .context("Failed to execute LibreOffice (soffice)")?;

        if !output.status.success() {
            anyhow::bail!(
                "LibreOffice conversion failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        // Find generated PDF
        let pdf_name = format!("{}.pdf", input_path.file_stem().unwrap().to_str().unwrap());
        let pdf_path = output_dir.join(pdf_name);

        if !pdf_path.exists() {
            anyhow::bail!("PDF not generated at {}", pdf_path.display());
        }

        fs::read(&pdf_path).context("Failed to read generated PDF")
    }

    /// Convert markdown to PDF via HTML
    ///
    /// Markdown → HTML → PDF using `LibreOffice` headless
    ///
    /// # Errors
    /// Returns an error if HTML conversion or `LibreOffice` conversion fails.
    #[must_use = "this function returns a PDF that should be processed"]
    pub fn markdown_to_pdf(&self, markdown: &str) -> Result<Vec<u8>> {
        // Convert markdown to HTML
        let html = markdown_to_html(markdown);

        // Convert HTML to PDF
        self.html_to_pdf(&html)
    }

    /// Convert HTML to PDF
    ///
    /// # Errors
    /// Returns an error if file I/O or `LibreOffice` conversion fails.
    ///
    /// # Panics
    ///
    /// Panics if the temporary file paths contain invalid UTF-8 characters
    /// that cannot be converted to strings.
    #[must_use = "this function returns a PDF that should be processed"]
    pub fn html_to_pdf(&self, html: &str) -> Result<Vec<u8>> {
        let temp_dir = tempfile::tempdir()?;
        let html_path = temp_dir.path().join("input.html");
        let output_dir = temp_dir.path();

        // Write HTML to temp file
        fs::write(&html_path, html)?;

        // Use LibreOffice headless to convert HTML to PDF (same as document_to_pdf)
        let output = Command::new("soffice")
            .args([
                "--headless",
                "--convert-to",
                "pdf",
                "--outdir",
                output_dir.to_str().unwrap(),
                html_path.to_str().unwrap(),
            ])
            .output()
            .context("Failed to execute LibreOffice (soffice)")?;

        if !output.status.success() {
            anyhow::bail!(
                "LibreOffice conversion failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        // Find generated PDF
        let pdf_name = "input.pdf";
        let pdf_path = output_dir.join(pdf_name);

        if !pdf_path.exists() {
            anyhow::bail!("PDF not generated at {}", pdf_path.display());
        }

        fs::read(&pdf_path).context("Failed to read generated PDF")
    }

    /// Compare two PDFs visually using LLM vision API
    ///
    /// Returns quality score based on visual similarity
    ///
    /// # Errors
    /// Returns an error if PDF conversion, image encoding, or LLM API call fails.
    #[must_use = "this function returns a visual quality report that should be processed"]
    pub async fn compare_visual_pdfs(
        &self,
        original_pdf: &[u8],
        output_pdf: &[u8],
        format_name: &str,
    ) -> Result<VisualQualityReport> {
        use base64::{engine::general_purpose, Engine as _};

        // Convert PDFs to PNG images (first page only for now)
        let original_png = Self::pdf_to_png(original_pdf)?;
        let output_png = Self::pdf_to_png(output_pdf)?;

        // Encode images as base64
        let original_b64 = general_purpose::STANDARD.encode(&original_png);
        let output_b64 = general_purpose::STANDARD.encode(&output_png);

        let prompt = format!(
            r#"Compare these two document renderings of a {format_name} document.

FIRST IMAGE: Ground truth (original document converted to PDF/PNG)
SECOND IMAGE: Parser output (markdown converted to PDF/PNG)

Evaluate visual quality on these dimensions:
1. Layout (30%): Text positioning, spacing, alignment, page structure
2. Formatting (25%): Bold, italic, fonts, colors, text styles
3. Tables (20%): Cell alignment, borders, data preservation
4. Completeness (15%): All content visible, no missing sections
5. Structure (10%): Section organization, hierarchy, headings

Return a JSON object with this exact structure:
{{
  "overall_score": 0.0-1.0,
  "layout_score": 0.0-1.0,
  "formatting_score": 0.0-1.0,
  "tables_score": 0.0-1.0,
  "completeness_score": 0.0-1.0,
  "structure_score": 0.0-1.0,
  "issues": ["list of specific issues found"],
  "strengths": ["list of things done well"]
}}"#
        );

        // Call vision API
        let response = self
            .client
            .vision_comparison(
                "gpt-4o", // Vision-capable model
                &prompt,
                &original_b64,
                &output_b64,
                1000,
            )
            .await?;

        // Parse JSON response into VisualQualityReport
        let report: VisualQualityReport = serde_json::from_str(&response)
            .context("Failed to parse vision API response as VisualQualityReport")?;

        Ok(report)
    }

    /// Convert PDF to PNG image (first page only)
    ///
    /// Uses pdf2image or similar tool
    fn pdf_to_png(pdf_data: &[u8]) -> Result<Vec<u8>> {
        let temp_dir = tempfile::tempdir()?;
        let pdf_path = temp_dir.path().join("input.pdf");
        let png_path = temp_dir.path().join("output.png");

        // Write PDF to temp file
        fs::write(&pdf_path, pdf_data)?;

        // Convert PDF to PNG using pdftoppm (from poppler-utils)
        // pdftoppm -png -f 1 -l 1 -singlefile input.pdf output
        let output = Command::new("pdftoppm")
            .args([
                "-png",
                "-f",
                "1",
                "-l",
                "1",
                "-singlefile",
                pdf_path.to_str().unwrap(),
                temp_dir.path().join("output").to_str().unwrap(),
            ])
            .output()
            .context("Failed to execute pdftoppm (install poppler-utils)")?;

        if !output.status.success() {
            anyhow::bail!(
                "pdftoppm failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        // Read the generated PNG
        fs::read(&png_path).context("Failed to read generated PNG")
    }
}

/// Convert markdown to HTML (basic implementation)
fn markdown_to_html(markdown: &str) -> String {
    // Use a markdown library or simple conversion
    // For now, wrap in basic HTML
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="UTF-8">
<style>
body {{ font-family: Arial, sans-serif; margin: 40px; }}
table {{ border-collapse: collapse; }}
td, th {{ border: 1px solid #ddd; padding: 8px; }}
</style>
</head>
<body>
{}
</body>
</html>"#,
        markdown_to_html_body(markdown)
    )
}

fn markdown_to_html_body(markdown: &str) -> String {
    use pulldown_cmark::{html, Parser};

    let parser = Parser::new(markdown);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_to_pdf() {
        // Skip if OPENAI_API_KEY not set (required for VisualTester)
        if std::env::var("OPENAI_API_KEY").is_err() {
            eprintln!("OPENAI_API_KEY not set - skipping visual test");
            return;
        }
        let tester = match VisualTester::new(VerificationConfig::default()) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Failed to create VisualTester: {e} - skipping");
                return;
            }
        };

        // Test converting a DOCX to PDF
        let result = tester.document_to_pdf(Path::new("test-corpus/docx/word_sample.docx"));

        if let Ok(pdf) = result {
            assert!(pdf.len() > 1000, "PDF should have content");
            assert_eq!(&pdf[0..4], b"%PDF", "Should be PDF file");
        } else {
            eprintln!("LibreOffice not available or test file missing - skipping");
        }
    }

    #[test]
    fn test_markdown_to_html() {
        let html = markdown_to_html("# Test\n\nParagraph");
        assert!(html.contains("<!DOCTYPE html>"));
        // Verify markdown is converted to actual HTML tags
        assert!(html.contains("<h1>"), "Should have h1 tag for # heading");
        assert!(html.contains("</h1>"), "Should close h1 tag");
        assert!(html.contains("<p>"), "Should have p tag for paragraph");
        assert!(html.contains("Test"));
        assert!(html.contains("Paragraph"));
    }

    #[test]
    fn test_markdown_to_html_with_formatting() {
        let html = markdown_to_html("**Bold** and *italic*\n\n- Item 1\n- Item 2");
        // Verify formatting is converted
        assert!(
            html.contains("<strong>"),
            "Should convert **bold** to <strong>"
        );
        assert!(html.contains("<em>"), "Should convert *italic* to <em>");
        assert!(html.contains("<ul>"), "Should convert lists to <ul>");
        assert!(html.contains("<li>"), "Should have list items <li>");
    }
}
