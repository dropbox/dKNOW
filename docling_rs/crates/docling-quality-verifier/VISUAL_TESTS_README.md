# Visual Quality Tests

## Overview

Visual quality tests compare document parser outputs against original documents using LLM vision models. This catches layout, formatting, and rendering issues that text-based comparison cannot detect.

## Architecture

```
Original Document → LibreOffice → PDF → pdftoppm → PNG (Ground Truth)
                                                      ↓
                                                   GPT-4o Vision
                                                      ↑
Parser Output (Markdown) → wkhtmltopdf → PDF → pdftoppm → PNG (Our Result)
```

## Test Flow

1. **Convert original document to PDF** using LibreOffice (DOCX, PPTX, XLSX, etc.)
2. **Parse document to markdown** using docling-core Rust backend
3. **Convert markdown to PDF** using wkhtmltopdf
4. **Convert both PDFs to PNG images** using pdftoppm (first page only)
5. **Compare images visually** using OpenAI GPT-4o vision API
6. **Generate quality report** with scores and detailed findings

## Requirements

### System Dependencies

Install these tools on your system:

```bash
# macOS
brew install poppler      # Provides pdftoppm
brew install wkhtmltopdf  # HTML/markdown to PDF
brew install libreoffice  # Document to PDF (DOCX, PPTX, XLSX)

# Ubuntu/Debian
sudo apt-get install poppler-utils wkhtmltopdf libreoffice

# Verify installation
which pdftoppm wkhtmltopdf soffice
```

### API Key

Set your OpenAI API key:

```bash
export OPENAI_API_KEY="your-api-key-here"
```

### Test Corpus

Ensure test files exist:

```bash
ls test-corpus/docx/word_sample.docx
ls test-corpus/pptx/powerpoint_sample.pptx
ls test-corpus/xlsx/excel_sample.xlsx
ls test-corpus/html/example.html
```

If missing, copy from Python docling repository:

```bash
cp ~/docling/tests/data/docx/*.docx test-corpus/docx/
cp ~/docling/tests/data/pptx/*.pptx test-corpus/pptx/
cp ~/docling/tests/data/xlsx/*.xlsx test-corpus/xlsx/
cp ~/docling/tests/data/html/*.html test-corpus/html/
```

## Running Tests

### All Visual Tests

```bash
cargo test -p docling-quality-verifier --test visual_quality_tests -- --nocapture --ignored
```

### Single Test (DOCX)

```bash
cargo test -p docling-quality-verifier --test visual_quality_tests test_visual_docx -- --exact --nocapture --ignored
```

### Other Format Tests

```bash
# PPTX
cargo test -p docling-quality-verifier --test visual_quality_tests test_visual_pptx -- --exact --nocapture --ignored

# XLSX
cargo test -p docling-quality-verifier --test visual_quality_tests test_visual_xlsx -- --exact --nocapture --ignored

# HTML
cargo test -p docling-quality-verifier --test visual_quality_tests test_visual_html -- --exact --nocapture --ignored
```

## Test Output

Example output for DOCX test:

```
Testing DOCX: ../../test-corpus/docx/word_sample.docx
Converting DOCX to PDF...
  Original PDF: 45234 bytes
Parsing DOCX to markdown...
  Markdown: 2341 chars
Converting markdown to PDF...
  Output PDF: 38921 bytes
Comparing PDFs visually with GPT-4o...

=== Visual Quality Report: DOCX (word_sample.docx) ===
Overall Score: 87.5%
  Layout:       90.0%
  Formatting:   85.0%
  Tables:       88.0%
  Completeness: 90.0%
  Structure:    85.0%

Issues Found:
  - Table borders slightly thinner than original
  - Font size in headings slightly smaller

Strengths:
  + All content preserved accurately
  + Table structure maintained well
  + Paragraph spacing consistent
```

## Cost Estimation

Visual tests use GPT-4o with high-detail images:

- **Per test:** ~$0.01-0.02 USD
- **4 format tests:** ~$0.04-0.08 USD
- **Full suite (20+ tests):** ~$0.20-0.40 USD

Vision API pricing: https://openai.com/pricing

## Quality Thresholds

Tests assert minimum visual quality scores:

| Format | Minimum Score | Reasoning |
|--------|--------------|-----------|
| DOCX   | 75%          | Good formatting preservation expected |
| PPTX   | 70%          | Slide layouts are complex, lower threshold |
| XLSX   | 75%          | Tables should render well |
| HTML   | 80%          | HTML → Markdown → HTML should be near-perfect |

## Troubleshooting

### Test Skipped: "OPENAI_API_KEY not set"

Set your API key:
```bash
export OPENAI_API_KEY="sk-..."
```

### Test Skipped: "LibreOffice (soffice) not found"

Install LibreOffice:
```bash
brew install libreoffice  # macOS
sudo apt-get install libreoffice  # Ubuntu
```

### Test Skipped: "wkhtmltopdf not found"

Install wkhtmltopdf:
```bash
brew install wkhtmltopdf  # macOS
sudo apt-get install wkhtmltopdf  # Ubuntu
```

### Test Skipped: "pdftoppm not found"

Install poppler-utils:
```bash
brew install poppler  # macOS
sudo apt-get install poppler-utils  # Ubuntu
```

### Test Skipped: "Test corpus not found"

Copy test files from Python docling repository:
```bash
cp ~/docling/tests/data/docx/*.docx test-corpus/docx/
```

### API Error: Rate Limit Exceeded

Wait 60 seconds between test runs or use a higher rate limit tier.

### API Error: Invalid Request

Check that:
1. API key is valid and not expired
2. Base64 image encoding is working
3. PNG files are valid (check with `file output.png`)

## Implementation Details

### Visual Quality Report Schema

```rust
pub struct VisualQualityReport {
    pub overall_score: f64,      // 0.0-1.0 (weighted average)
    pub layout_score: f64,       // 30% weight
    pub formatting_score: f64,   // 25% weight
    pub tables_score: f64,       // 20% weight
    pub completeness_score: f64, // 15% weight
    pub structure_score: f64,    // 10% weight
    pub issues: Vec<String>,     // Specific problems found
    pub strengths: Vec<String>,  // Things done well
}
```

### Scoring Weights

The overall score is computed as:

```
overall = layout*0.30 + formatting*0.25 + tables*0.20 +
          completeness*0.15 + structure*0.10
```

### LLM Prompt Engineering

The vision API receives:
- System prompt: Quality verification assistant role
- User prompt: Structured comparison request with evaluation criteria
- Two images: Original document (ground truth) and parser output
- Response format: JSON object with scores and findings

### Limitations

1. **First page only**: Currently compares only the first page of documents
2. **Single-page focus**: Multi-page documents are not fully tested
3. **LLM variability**: Vision API scores may vary slightly between runs
4. **Cost**: Each test costs ~$0.01-0.02 USD in API fees

### Future Improvements

- Multi-page comparison (compare all pages, aggregate scores)
- Per-page scoring (identify which pages have issues)
- Screenshot caching (avoid regenerating PDFs/PNGs for same input)
- Faster models (consider gpt-4o-mini with vision when available)
- Local vision models (avoid API costs entirely)

## Integration with CI/CD

### GitHub Actions Example

```yaml
name: Visual Quality Tests

on:
  push:
    branches: [main]
  schedule:
    - cron: '0 0 * * 0'  # Weekly on Sunday

jobs:
  visual-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y poppler-utils wkhtmltopdf libreoffice

      - name: Run visual tests
        env:
          OPENAI_API_KEY: ${{ secrets.OPENAI_API_KEY }}
        run: |
          cargo test -p docling-quality-verifier \
            --test visual_quality_tests \
            -- --nocapture --ignored
```

### Cost Control

To avoid excessive API costs:
1. Run visual tests **weekly** (not on every commit)
2. Run on **main branch only** (not PRs)
3. Set **budget alerts** in OpenAI dashboard
4. Cache results for unchanged parser implementations

## References

- OpenAI Vision API: https://platform.openai.com/docs/guides/vision
- GPT-4o Model: https://openai.com/index/hello-gpt-4o/
- LibreOffice Headless: https://wiki.documentfoundation.org/Faq/General/007
- wkhtmltopdf: https://wkhtmltopdf.org/
- Poppler PDF Tools: https://poppler.freedesktop.org/
