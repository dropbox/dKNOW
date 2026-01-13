# docling-quality-verifier

LLM-based quality verification for document parser outputs.

## Overview

This crate provides automated semantic quality assessment of parser outputs using OpenAI's language models. Unlike traditional string comparison, it can detect:

- **Completeness issues**: Missing sections, pages, or elements
- **Accuracy problems**: Incorrect content or data
- **Structure defects**: Wrong document hierarchy
- **Formatting errors**: Garbled tables, lists, or code blocks
- **Metadata issues**: Missing or incorrect titles, authors, dates

## Features

- **Comparative Analysis (Mode 2)**: Compare expected vs actual outputs
- **Visual Quality Testing**: PDF-based visual comparison using GPT-4o vision API
- **Quality Scoring**: 0.0-1.0 score with detailed category breakdown (Completeness, Accuracy, Structure, Formatting, Metadata)
- **Actionable Findings**: Specific issues with severity levels (Critical, Major, Minor, Info)
- **Cost Efficient**: Uses gpt-4o-mini for text (~$0.05-0.10/month) or gpt-4o for vision (~$0.01-0.02/test)

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
docling-quality-verifier = "2.58.0"
docling-core = "2.58.0"
tokio = { version = "1.0", features = ["full"] }
```

## Usage

### Basic Example

```rust
use docling_quality_verifier::{LLMQualityVerifier, VerificationConfig};
use docling_core::InputFormat;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set API key
    std::env::set_var("OPENAI_API_KEY", "sk-...");

    // Create verifier
    let verifier = LLMQualityVerifier::new(VerificationConfig {
        model: "gpt-4o-mini".to_string(),
        quality_threshold: 0.85,
        detailed_diagnostics: true,
        max_tokens: 4096,
    })?;

    // Load outputs
    let expected = std::fs::read_to_string("expected_output.md")?;
    let actual = std::fs::read_to_string("actual_output.md")?;

    // Compare outputs
    let report = verifier.compare_outputs(
        &expected,
        &actual,
        InputFormat::Docx
    ).await?;

    // Check results
    println!("Quality Score: {:.1}%", report.score * 100.0);
    println!("Status: {}", if report.passed { "✅ PASS" } else { "❌ FAIL" });

    // Print findings
    for finding in report.findings {
        println!("[{:?}] {:?}: {}",
            finding.severity,
            finding.category,
            finding.description
        );
        if let Some(loc) = finding.location {
            println!("    Location: {}", loc);
        }
    }

    Ok(())
}
```

### Using Environment Variables

```rust
use docling_quality_verifier::LLMQualityVerifier;

// Configure via environment variables:
// - LLM_MODEL="gpt-4o-mini"
// - LLM_QUALITY_THRESHOLD="0.85"
// - LLM_DETAILED="true"
// - LLM_MAX_TOKENS="4096"
// - OPENAI_API_KEY="sk-..."

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let verifier = LLMQualityVerifier::from_env()?;

    // ... use verifier
    Ok(())
}
```

### Integration Testing

```rust
#[tokio::test]
async fn test_parser_quality() {
    let verifier = LLMQualityVerifier::default().unwrap();

    let result = parse_document("test.docx").unwrap();
    let expected = load_expected_output("test_expected.md").unwrap();

    // Traditional check (fast)
    if result.markdown == expected {
        return; // Perfect match
    }

    // LLM verification (if traditional fails)
    let quality = verifier.compare_outputs(
        &expected,
        &result.markdown,
        InputFormat::Docx
    ).await.unwrap();

    assert!(quality.score >= 0.85,
        "Quality too low: {:.1}% < 85%",
        quality.score * 100.0
    );
}
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `OPENAI_API_KEY` | OpenAI API key (required) | - |
| `LLM_MODEL` | Model name | `gpt-4o-mini` |
| `LLM_QUALITY_THRESHOLD` | Pass/fail threshold (0.0-1.0) | `0.85` |
| `LLM_DETAILED` | Enable detailed diagnostics | `false` |
| `LLM_MAX_TOKENS` | Max tokens in LLM response | `4096` |
| `OPENAI_API_BASE` | Custom API base URL | `https://api.openai.com/v1` |

## Quality Scoring

### Overall Score

Weighted average of category scores:
- **Completeness**: 30% (most important)
- **Accuracy**: 30% (most important)
- **Structure**: 20%
- **Formatting**: 15%
- **Metadata**: 5%

### Category Scores (0-100)

- **100**: Perfect match
- **95-99**: Excellent (minor acceptable differences)
- **85-94**: Good (some differences, semantically equivalent)
- **70-84**: Fair (noticeable issues, still usable)
- **Below 70**: Poor (significant problems)

### Severity Levels

- **Critical**: Unusable output (major content missing)
- **Major**: Significant problems (substantial content incorrect)
- **Minor**: Small differences (acceptable variations)
- **Info**: Informational (formatting preferences, style)

## Cost Analysis

Using **gpt-4o-mini** (recommended):
- Input: $0.15/1M tokens
- Output: $0.60/1M tokens

### Typical Usage

Per test (~2500 tokens total):
- Cost: ~$0.0006
- 97 tests: ~$0.06
- Monthly (10 benchmark runs): ~$0.60

### On-Failure Only

10% failure rate during development:
- Per run: ~$0.006
- Monthly: ~$0.06

**Recommendation**: Use `LLM_VERIFY_ON_FAIL=1` mode for cost-efficient development.

## Models

### gpt-4o-mini (Recommended)
- Cost: $0.15/$0.60 per 1M tokens
- Quality: Good for most cases
- Speed: Fast (~1-2 seconds per comparison)

### gpt-4o (High Accuracy)
- Cost: $2.50/$10.00 per 1M tokens
- Quality: Excellent for critical validation
- Speed: Moderate (~2-4 seconds)

## Limitations

- **Token Limits**: Large documents are truncated (8000 chars ≈ 2000 tokens)
- **API Costs**: Small but non-zero cost per verification
- **Latency**: ~1-2 seconds per verification (async)
- **API Dependency**: Requires OpenAI API access

## Visual Quality Testing

The visual testing module compares documents visually using PDF rendering:

```bash
# Set API key
export OPENAI_API_KEY="sk-..."

# Run visual tests (requires LibreOffice, wkhtmltopdf, pdftoppm)
cargo test --test visual_quality_tests -- --ignored --nocapture

# Run single format
cargo test --test visual_quality_tests test_visual_docx -- --exact --ignored --nocapture
```

### How It Works

1. **Original to PDF**: Convert original document (DOCX/PPTX/XLSX/HTML) to PDF
2. **Parser to Markdown**: Parse document using docling parser to markdown
3. **Markdown to PDF**: Convert markdown back to PDF
4. **Visual Comparison**: Use GPT-4o vision API to compare both PDFs visually

### Visual Quality Metrics

- **Layout**: Overall document structure and spacing (30% weight)
- **Formatting**: Text styles, fonts, and visual hierarchy (25% weight)
- **Tables**: Table structure and data accuracy (20% weight)
- **Completeness**: All content present and rendered (15% weight)
- **Structure**: Section organization and flow (10% weight)

### Requirements

```bash
# macOS
brew install poppler wkhtmltopdf libreoffice

# Linux
apt-get install poppler-utils wkhtmltopdf libreoffice
```

### Cost

Visual tests use GPT-4o with vision:
- Per test: ~$0.01-0.02
- 4 visual tests: ~$0.04-0.08
- Recommended for CI/CD quality gates

## Examples

See `crates/docling-examples/` for more examples.

## License

MIT
