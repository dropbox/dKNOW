//! LLM DocItem Validation Tests
//!
//! These tests validate DocItem completeness - the REAL format!
//! NOT markdown output quality (which is inherently limited).
//!
//! # What These Tests Validate
//!
//! - Does DOCX parser extract ALL information to DocItems?
//! - Is JSON export 100% complete?
//! - Are all document features captured?
//!
//! # Running These Tests
//!
//! ```bash
//! source .env  # Has OPENAI_API_KEY
//! cargo test llm_docitem -- --ignored --nocapture
//! ```

use docling_apple::{KeynoteBackend, NumbersBackend, PagesBackend};
use docling_backend::{
    ArchiveBackend, AsciidocBackend, AvifBackend, BmpBackend, CadBackend, CsvBackend, DicomBackend,
    DocumentBackend, DocxBackend, EbooksBackend, EmailBackend, GifBackend, GpxBackend, HeifBackend,
    HtmlBackend, IcsBackend, IdmlBackend, IpynbBackend, JatsBackend, JpegBackend, KmlBackend,
    MarkdownBackend, OpenDocumentBackend, PngBackend, PptxBackend, RtfBackend, SrtBackend,
    SvgBackend, TiffBackend, WebpBackend, WebvttBackend, XlsxBackend, XpsBackend,
};
use docling_core::InputFormat;
use docling_latex::LatexBackend;
use docling_legacy::doc::DocBackend;
use docling_microsoft_extended::{ProjectBackend, VisioBackend};
use docling_quality_verifier::{LLMQualityVerifier, VerificationConfig};
use std::path::Path;

fn create_verifier() -> LLMQualityVerifier {
    LLMQualityVerifier::new(VerificationConfig {
        model: "gpt-4o".to_string(),
        quality_threshold: 0.95,
        detailed_diagnostics: true,
        max_tokens: 8192,
    })
    .expect("Failed to create verifier - check OPENAI_API_KEY")
}

/// Truncate large JSON for LLM context
///
/// GPT-4o has 128K token limit (~512K chars). For large documents (EPUB, JATS, MOBI),
/// the DocItem JSON can exceed this. This function intelligently truncates:
///
/// - Keep first 40K chars (beginning of document)
/// - Keep last 40K chars (end of document)
/// - Add summary in middle (DocItem count, truncated size)
///
/// This preserves structure visibility while fitting in context window.
fn truncate_json_for_llm(json: &str, max_chars: usize) -> String {
    if json.len() <= max_chars {
        return json.to_string();
    }

    // Calculate chunk sizes (40% each for start/end, 20% for summary)
    let chunk_size = max_chars * 4 / 10;

    // Use char-aware truncation to avoid panics on UTF-8 boundaries
    let start_chunk: String = json.chars().take(chunk_size).collect();
    let end_chunk: String = json
        .chars()
        .rev()
        .take(chunk_size)
        .collect::<String>()
        .chars()
        .rev()
        .collect();

    // Count DocItems (rough estimate by counting "label" fields)
    let docitem_count = json.matches("\"label\":").count();
    let truncated_chars =
        json.chars().count() - start_chunk.chars().count() - end_chunk.chars().count();

    format!(
        "{}\n\n... [TRUNCATED {} chars containing ~{} middle DocItems for LLM context limit] ...\n\n{}",
        start_chunk,
        truncated_chars,
        docitem_count / 2, // Rough estimate of middle items
        end_chunk
    )
}

/// Test DOCX DocItem completeness via JSON
///
/// MANDATORY TEST - NOT IGNORED
/// This validates parser extracts ALL information to DocItems (JSON)
/// NOT just markdown output quality
#[tokio::test]
async fn test_llm_docitem_docx() {
    let verifier = create_verifier();

    // Parse DOCX to DocItems
    let backend = DocxBackend;
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/docx/word_sample.docx"
            ),
            &Default::default(),
        )
        .expect("Failed to parse DOCX");

    // Export to JSON (the complete representation)
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", result.content_blocks.is_some());

    // Read original DOCX for comparison
    let docx_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/docx/word_sample.docx"
    ));

    // LLM validates: Does JSON contain ALL information from DOCX?
    let prompt = format!(
        r#"Analyze if this DocItem JSON contains complete information from the original DOCX.

ORIGINAL DOCUMENT: {}
(Open and analyze the DOCX structure)

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness:

1. **Completeness (0-100)**: All paragraphs, tables, images extracted?
2. **Accuracy (0-100)**: Content semantically correct?
3. **Structure (0-100)**: Headings, lists, sections preserved?
4. **Formatting (0-100)**: Tables, lists formatted correctly in DocItems?
5. **Metadata (0-100)**: Document properties, styles captured?

For each category, check:
- Is information extracted to DocItems?
- Is metadata preserved (types, styles, formatting)?
- Are relationships maintained?

For each category with score < 95:
- List specific issues
- Assign severity: "critical", "major", "minor", or "info"
- Provide location if identifiable

Calculate overall_score as weighted average:
- completeness: 30%
- accuracy: 30%
- structure: 20%
- formatting: 15%
- metadata: 5%

Return JSON ONLY (no markdown, no explanation):
{{
  "overall_score": 0.0-1.0,
  "category_scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        docx_path.display(),
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: DOCX ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Text Content: {}/100",
        quality.category_scores.completeness
    );
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Tables:       {}/100", quality.category_scores.accuracy);
    println!("  Images:       {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    if !quality.findings.is_empty() {
        println!("\nDocItem Gaps:");
        for finding in &quality.findings {
            println!("  - {}", finding.description);
        }
    }

    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}

/// Test CSV DocItem completeness
///
/// MANDATORY TEST - NOT IGNORED
/// This validates CSV parser extracts ALL data to DocItems (Table structure)
#[tokio::test]
async fn test_llm_docitem_csv() {
    let verifier = create_verifier();

    // Parse CSV to DocItems
    let backend = CsvBackend::new();
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/csv/csv-comma.csv"
            ),
            &Default::default(),
        )
        .expect("Failed to parse CSV");

    // Export to JSON (the complete representation)
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", result.content_blocks.is_some());

    // Read original CSV for comparison
    let csv_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/csv/csv-comma.csv"
    ));
    let csv_content = std::fs::read_to_string(csv_path).expect("Failed to read CSV");

    // LLM validates: Does JSON contain ALL information from CSV?
    let prompt = format!(
        r#"Analyze if this DocItem JSON contains complete information from the original CSV.

ORIGINAL CSV:
```csv
{}
```

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness:

1. **Completeness (0-100)**: All rows and columns extracted?
2. **Accuracy (0-100)**: Cell values semantically correct?
3. **Structure (0-100)**: Table structure preserved (rows × columns)?
4. **Formatting (0-100)**: Table formatted correctly in DocItems?
5. **Metadata (0-100)**: CSV metadata captured (delimiter, num_rows, num_cols)?

For each category, check:
- Is ALL CSV data extracted to Table DocItem?
- Are cell values preserved exactly?
- Is table grid structure correct?

For each category with score < 95:
- List specific issues
- Assign severity: "critical", "major", "minor", or "info"
- Provide location if identifiable

Calculate overall_score as weighted average:
- completeness: 30%
- accuracy: 30%
- structure: 20%
- formatting: 15%
- metadata: 5%

Return JSON ONLY (no markdown, no explanation):
{{
  "overall_score": 0.0-1.0,
  "category_scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        csv_content,
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: CSV ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Completeness: {}/100",
        quality.category_scores.completeness
    );
    println!("  Accuracy:     {}/100", quality.category_scores.accuracy);
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Formatting:   {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    if !quality.findings.is_empty() {
        println!("\nDocItem Gaps:");
        for finding in &quality.findings {
            println!("  - {}", finding.description);
        }
    }

    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}

/// Test PPTX DocItem completeness via JSON
///
/// MANDATORY TEST - NOT IGNORED
/// This validates parser extracts ALL information to DocItems (JSON)
#[tokio::test]
async fn test_llm_docitem_pptx() {
    let verifier = create_verifier();

    // Parse PPTX to DocItems
    // NOTE: Using powerpoint_sample.pptx (3 slides) for overall quality test
    // Image extraction is validated separately in backend unit test test_pptx_image_extraction
    let backend = PptxBackend;
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/pptx/powerpoint_sample.pptx"
            ),
            &Default::default(),
        )
        .expect("Failed to parse PPTX");

    // Export to JSON (the complete representation)
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", result.content_blocks.is_some());
    println!("Slide count: {}", result.metadata.num_pages.unwrap_or(0));

    // Read original PPTX for comparison
    let pptx_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/pptx/powerpoint_sample.pptx"
    ));

    // LLM validates: Does JSON contain ALL information from PPTX?
    let prompt = format!(
        r#"Analyze if this DocItem JSON contains complete information from the original PPTX.

ORIGINAL DOCUMENT: {}
(Open and analyze the PPTX structure - slides, text, shapes, images)

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness:

1. **Completeness (0-100)**: All slides, text boxes, shapes extracted?
2. **Accuracy (0-100)**: Content semantically correct?
3. **Structure (0-100)**: Slide order, layout, sections preserved?
4. **Formatting (0-100)**: Tables, lists, shapes represented in DocItems?
5. **Metadata (0-100)**: Document properties, slide metadata captured?

For each category, check:
- Is information extracted to DocItems?
- Is metadata preserved (types, styles, formatting)?
- Are relationships maintained?

For each category with score < 95:
- List specific issues
- Assign severity: "critical", "major", "minor", or "info"
- Provide location if identifiable

Calculate overall_score as weighted average:
- completeness: 30%
- accuracy: 30%
- structure: 20%
- formatting: 15%
- metadata: 5%

Return JSON ONLY (no markdown, no explanation):
{{
  "overall_score": 0.0-1.0,
  "category_scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        pptx_path.display(),
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: PPTX ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Completeness: {}/100",
        quality.category_scores.completeness
    );
    println!("  Accuracy:     {}/100", quality.category_scores.accuracy);
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Formatting:   {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    if !quality.findings.is_empty() {
        println!("\nDocItem Gaps:");
        for finding in &quality.findings {
            println!("  - {}", finding.description);
        }
    }

    assert!(
        quality.score >= 0.85,
        "DocItem completeness: {:.1}% (need 85%)",
        quality.score * 100.0
    );
}

/// Test XLSX DocItem completeness via JSON
///
/// MANDATORY TEST - NOT IGNORED
/// This validates parser extracts ALL information to DocItems (JSON)
#[tokio::test]
async fn test_llm_docitem_xlsx() {
    let verifier = create_verifier();

    // Parse XLSX to DocItems
    // NOTE: Using smaller file (xlsx_05) to avoid GPT-4o 128K context limit
    // xlsx_01.xlsx produces 249KB JSON (~147K tokens) which exceeds limit
    let backend = XlsxBackend;
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/xlsx/xlsx_05_financial_report.xlsx"
            ),
            &Default::default(),
        )
        .expect("Failed to parse XLSX");

    // Export to JSON (the complete representation)
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", result.content_blocks.is_some());

    // Read original XLSX for comparison
    let xlsx_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/xlsx/xlsx_05_financial_report.xlsx"
    ));

    // LLM validates: Does JSON contain ALL information from XLSX?
    let prompt = format!(
        r#"Analyze if this DocItem JSON contains complete information from the original XLSX.

ORIGINAL DOCUMENT: {}
(Open and analyze the XLSX structure - sheets, rows, columns, formulas)

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness:

1. **Completeness (0-100)**: All sheets, rows, columns, cells extracted?
2. **Accuracy (0-100)**: Cell values correct (numbers, text, formulas)?
3. **Structure (0-100)**: Sheet order, table structure preserved?
4. **Formatting (0-100)**: Tables, cell formatting represented in DocItems?
5. **Metadata (0-100)**: Sheet names, workbook properties captured?

For each category, check:
- Is information extracted to DocItems?
- Is metadata preserved (types, styles, formatting)?
- Are relationships maintained?

For each category with score < 95:
- List specific issues
- Assign severity: "critical", "major", "minor", or "info"
- Provide location if identifiable

Calculate overall_score as weighted average:
- completeness: 30%
- accuracy: 30%
- structure: 20%
- formatting: 15%
- metadata: 5%

Return JSON ONLY (no markdown, no explanation):
{{
  "overall_score": 0.0-1.0,
  "category_scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        xlsx_path.display(),
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: XLSX ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Completeness: {}/100",
        quality.category_scores.completeness
    );
    println!("  Accuracy:     {}/100", quality.category_scores.accuracy);
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Formatting:   {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    if !quality.findings.is_empty() {
        println!("\nDocItem Gaps:");
        for finding in &quality.findings {
            println!("  - {}", finding.description);
        }
    }

    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}

/// Test HTML DocItem completeness via JSON
///
/// MANDATORY TEST - NOT IGNORED
/// This validates HTML parser extracts ALL DOM information to DocItems
#[tokio::test]
async fn test_llm_docitem_html() {
    let verifier = create_verifier();

    // Parse HTML to DocItems
    let backend = HtmlBackend;
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/html/example_03.html"
            ),
            &Default::default(),
        )
        .expect("Failed to parse HTML");

    // Export to JSON (the complete representation)
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", result.content_blocks.is_some());

    // Read original HTML for comparison
    let html_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/html/example_03.html"
    ));
    let html_content = std::fs::read_to_string(html_path).expect("Failed to read HTML");

    // LLM validates: Does JSON contain ALL information from HTML?
    let prompt = format!(
        r#"Analyze if this DocItem JSON contains complete information from the original HTML.

ORIGINAL HTML:
```html
{}
```

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness:

1. **Completeness (0-100)**: All headings, paragraphs, lists, tables extracted?
2. **Accuracy (0-100)**: Content semantically correct?
3. **Structure (0-100)**: Document structure (heading levels, lists, sections) preserved?
4. **Formatting (0-100)**: Tables, lists formatted correctly in DocItems?
5. **Metadata (0-100)**: HTML metadata captured?

For each category, check:
- Is information extracted to DocItems?
- Is metadata preserved (types, styles, formatting)?
- Are relationships maintained?

For each category with score < 95:
- List specific issues
- Assign severity: "critical", "major", "minor", or "info"
- Provide location if identifiable

Calculate overall_score as weighted average:
- completeness: 30%
- accuracy: 30%
- structure: 20%
- formatting: 15%
- metadata: 5%

Return JSON ONLY (no markdown, no explanation):
{{
  "overall_score": 0.0-1.0,
  "category_scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        html_content,
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: HTML ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Completeness: {}/100",
        quality.category_scores.completeness
    );
    println!("  Accuracy:     {}/100", quality.category_scores.accuracy);
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Formatting:   {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    if !quality.findings.is_empty() {
        println!("\nDocItem Gaps:");
        for finding in &quality.findings {
            println!("  - {}", finding.description);
        }
    }

    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}

/// Test Markdown DocItem completeness via JSON
///
/// MANDATORY TEST - NOT IGNORED
/// This validates Markdown parser extracts ALL information to DocItems
#[tokio::test]
async fn test_llm_docitem_markdown() {
    let verifier = create_verifier();

    // Parse Markdown to DocItems
    let backend = MarkdownBackend;
    let result = backend
        .parse_file(
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../test-corpus/md/duck.md"),
            &Default::default(),
        )
        .expect("Failed to parse Markdown");

    // Export to JSON (the complete representation)
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", result.content_blocks.is_some());

    // Read original Markdown for comparison
    let md_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/md/duck.md"
    ));
    let md_content = std::fs::read_to_string(md_path).expect("Failed to read Markdown");

    // LLM validates: Does JSON contain ALL information from Markdown?
    let prompt = format!(
        r#"Analyze if this DocItem JSON contains complete information from the original Markdown.

ORIGINAL MARKDOWN:
```markdown
{}
```

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness:

1. **Completeness (0-100)**: All headings, paragraphs, lists, code blocks extracted?
2. **Accuracy (0-100)**: Content semantically correct?
3. **Structure (0-100)**: Document structure (heading levels, lists, sections) preserved?
4. **Formatting (0-100)**: Code blocks, lists, emphasis formatted correctly in DocItems?
5. **Metadata (0-100)**: Markdown metadata captured?

For each category, check:
- Is information extracted to DocItems?
- Is metadata preserved (types, styles, formatting)?
- Are relationships maintained?

For each category with score < 95:
- List specific issues
- Assign severity: "critical", "major", "minor", or "info"
- Provide location if identifiable

Calculate overall_score as weighted average:
- completeness: 30%
- accuracy: 30%
- structure: 20%
- formatting: 15%
- metadata: 5%

Return JSON ONLY (no markdown, no explanation):
{{
  "overall_score": 0.0-1.0,
  "category_scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        md_content,
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: Markdown ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Completeness: {}/100",
        quality.category_scores.completeness
    );
    println!("  Accuracy:     {}/100", quality.category_scores.accuracy);
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Formatting:   {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    if !quality.findings.is_empty() {
        println!("\nDocItem Gaps:");
        for finding in &quality.findings {
            println!("  - {}", finding.description);
        }
    }

    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}

/// Test AsciiDoc DocItem completeness via JSON
///
/// MANDATORY TEST - NOT IGNORED
/// This validates AsciiDoc parser extracts ALL information to DocItems
#[tokio::test]
async fn test_llm_docitem_asciidoc() {
    let verifier = create_verifier();

    // Parse AsciiDoc to DocItems
    let backend = AsciidocBackend;
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/asciidoc/test_02.asciidoc"
            ),
            &Default::default(),
        )
        .expect("Failed to parse AsciiDoc");

    // Export to JSON (the complete representation)
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", result.content_blocks.is_some());

    // Read original AsciiDoc for comparison
    let asciidoc_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/asciidoc/test_02.asciidoc"
    ));
    let asciidoc_content = std::fs::read_to_string(asciidoc_path).expect("Failed to read AsciiDoc");

    // LLM validates: Does JSON contain ALL information from AsciiDoc?
    let prompt = format!(
        r#"Analyze if this DocItem JSON contains complete information from the original AsciiDoc.

ORIGINAL ASCIIDOC:
```asciidoc
{}
```

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness:

1. **Completeness (0-100)**: All headings, paragraphs, lists, sections extracted?
2. **Accuracy (0-100)**: Content semantically correct?
3. **Structure (0-100)**: Document structure (heading levels, lists, sections) preserved?
4. **Formatting (0-100)**: Lists, code blocks formatted correctly in DocItems?
5. **Metadata (0-100)**: AsciiDoc metadata captured?

For each category, check:
- Is information extracted to DocItems?
- Is metadata preserved (types, styles, formatting)?
- Are relationships maintained?

For each category with score < 95:
- List specific issues
- Assign severity: "critical", "major", "minor", or "info"
- Provide location if identifiable

Calculate overall_score as weighted average:
- completeness: 30%
- accuracy: 30%
- structure: 20%
- formatting: 15%
- metadata: 5%

Return JSON ONLY (no markdown, no explanation):
{{
  "overall_score": 0.0-1.0,
  "category_scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        asciidoc_content,
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: AsciiDoc ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Completeness: {}/100",
        quality.category_scores.completeness
    );
    println!("  Accuracy:     {}/100", quality.category_scores.accuracy);
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Formatting:   {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    if !quality.findings.is_empty() {
        println!("\nDocItem Gaps:");
        for finding in &quality.findings {
            println!("  - {}", finding.description);
        }
    }

    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}

/// Test JATS DocItem completeness via JSON
///
/// MANDATORY TEST - NOT IGNORED
/// This validates JATS parser extracts ALL information to DocItems
#[tokio::test]
async fn test_llm_docitem_jats() {
    let verifier = create_verifier();

    // Parse JATS to DocItems
    // NOTE: Using smaller sample to avoid context limits (elife_sample_03 is 159KB)
    let backend = JatsBackend;
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/jats/elife_sample_02.nxml"
            ),
            &Default::default(),
        )
        .expect("Failed to parse JATS");

    // Export to JSON (the complete representation)
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", result.content_blocks.is_some());

    // Read original JATS for comparison
    let jats_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/jats/elife_sample_02.nxml"
    ));
    let jats_content = std::fs::read_to_string(jats_path).expect("Failed to read JATS");

    // LLM validates: Does JSON contain ALL information from JATS?
    let prompt = format!(
        r#"Analyze if this DocItem JSON contains complete information from the original JATS XML.

ORIGINAL DOCUMENT:
```xml
{}
```
(Analyze the JATS XML structure - article metadata, sections, paragraphs, citations)

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness:

1. **Completeness (0-100)**: All sections, paragraphs, figures, citations extracted?
2. **Accuracy (0-100)**: Content semantically correct?
3. **Structure (0-100)**: Article structure (sections, subsections) preserved?
4. **Formatting (0-100)**: Citations, figures, tables formatted correctly in DocItems?
5. **Metadata (0-100)**: Article metadata (authors, title, journal) captured?

For each category, check:
- Is information extracted to DocItems?
- Is metadata preserved (types, styles, formatting)?
- Are relationships maintained?

For each category with score < 95:
- List specific issues
- Assign severity: "critical", "major", "minor", or "info"
- Provide location if identifiable

Calculate overall_score as weighted average:
- completeness: 30%
- accuracy: 30%
- structure: 20%
- formatting: 15%
- metadata: 5%

Return JSON ONLY (no markdown, no explanation):
{{
  "overall_score": 0.0-1.0,
  "category_scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        jats_content,
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: JATS ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Completeness: {}/100",
        quality.category_scores.completeness
    );
    println!("  Accuracy:     {}/100", quality.category_scores.accuracy);
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Formatting:   {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    if !quality.findings.is_empty() {
        println!("\nDocItem Gaps:");
        for finding in &quality.findings {
            println!("  - {}", finding.description);
        }
    }

    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}

/// Test WebVTT DocItem completeness via JSON
///
/// MANDATORY TEST - NOT IGNORED
/// This validates WebVTT parser extracts ALL information to DocItems
#[tokio::test]
async fn test_llm_docitem_webvtt() {
    let verifier = create_verifier();

    // Parse WebVTT to DocItems
    let backend = WebvttBackend;
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/webvtt/webvtt_example_01.vtt"
            ),
            &Default::default(),
        )
        .expect("Failed to parse WebVTT");

    // Export to JSON (the complete representation)
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", result.content_blocks.is_some());

    // Read original WebVTT for comparison
    let webvtt_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/webvtt/webvtt_example_01.vtt"
    ));
    let webvtt_content = std::fs::read_to_string(webvtt_path).expect("Failed to read WebVTT");

    // LLM validates: Does JSON contain ALL information from WebVTT?
    let prompt = format!(
        r#"Analyze if this DocItem JSON contains complete information from the original WebVTT.

ORIGINAL WEBVTT:
```webvtt
{}
```

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness:

1. **Completeness (0-100)**: All captions, timestamps, cue identifiers extracted?
2. **Accuracy (0-100)**: Content and timestamps semantically correct?
3. **Structure (0-100)**: Cue order and timing preserved?
4. **Formatting (0-100)**: Caption formatting (voices, styles) represented in DocItems?
5. **Metadata (0-100)**: WebVTT metadata (headers, regions) captured?

For each category, check:
- Is information extracted to DocItems?
- Is metadata preserved (types, styles, formatting)?
- Are relationships maintained?

For each category with score < 95:
- List specific issues
- Assign severity: "critical", "major", "minor", or "info"
- Provide location if identifiable

Calculate overall_score as weighted average:
- completeness: 30%
- accuracy: 30%
- structure: 20%
- formatting: 15%
- metadata: 5%

Return JSON ONLY (no markdown, no explanation):
{{
  "overall_score": 0.0-1.0,
  "category_scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        webvtt_content,
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: WebVTT ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Completeness: {}/100",
        quality.category_scores.completeness
    );
    println!("  Accuracy:     {}/100", quality.category_scores.accuracy);
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Formatting:   {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    if !quality.findings.is_empty() {
        println!("\nDocItem Gaps:");
        for finding in &quality.findings {
            println!("  - {}", finding.description);
        }
    }

    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}

/// Test PNG DocItem completeness via JSON
///
/// MANDATORY TEST - NOT IGNORED
/// This validates PNG parser extracts text via OCR to DocItems
#[tokio::test]
async fn test_llm_docitem_png() {
    let verifier = create_verifier();

    // Parse PNG to DocItems (with OCR)
    let backend = PngBackend;
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/png/detail_pattern.png"
            ),
            &Default::default(),
        )
        .expect("Failed to parse PNG");

    // Export to JSON (the complete representation)
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", result.content_blocks.is_some());

    // Read original PNG for comparison
    let png_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/png/detail_pattern.png"
    ));

    // LLM validates: Does JSON contain text extracted from PNG?
    let prompt = format!(
        r#"Analyze if this DocItem JSON contains text extracted from the PNG image via OCR.

ORIGINAL IMAGE: {}
(Open and analyze the PNG - does it contain text?)

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness for IMAGE WITH TEXT:

1. **Completeness (0-100)**: All visible text extracted via OCR?
2. **Accuracy (0-100)**: OCR text semantically correct (spelling, words)?
3. **Structure (0-100)**: Text layout preserved (paragraphs, sections)?
4. **Formatting (0-100)**: Text structure represented in DocItems?
5. **Metadata (0-100)**: Image metadata (dimensions, format) captured?

For images WITHOUT visible text:
- Completeness: 100 (nothing to extract)
- Accuracy: 100 (no OCR errors possible)
- Structure: 100 (no structure to preserve)
- Formatting: 100 (no formatting to preserve)
- Metadata: Check dimensions, format are captured

For each category with score < 95:
- List specific issues
- Assign severity: "critical", "major", "minor", or "info"
- Provide location if identifiable

Calculate overall_score as weighted average:
- completeness: 30%
- accuracy: 30%
- structure: 20%
- formatting: 15%
- metadata: 5%

Return JSON ONLY (no markdown, no explanation):
{{
  "overall_score": 0.0-1.0,
  "category_scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        png_path.display(),
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: PNG ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Completeness: {}/100",
        quality.category_scores.completeness
    );
    println!("  Accuracy:     {}/100", quality.category_scores.accuracy);
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Formatting:   {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    if !quality.findings.is_empty() {
        println!("\nDocItem Gaps:");
        for finding in &quality.findings {
            println!("  - {}", finding.description);
        }
    }

    assert!(
        quality.score >= 0.90,
        "DocItem completeness: {:.1}% (need 90%, OCR may have minor errors)",
        quality.score * 100.0
    );
}

/// Test JPEG DocItem completeness via JSON
///
/// MANDATORY TEST - NOT IGNORED
/// This validates JPEG parser extracts text via OCR to DocItems
#[tokio::test]
async fn test_llm_docitem_jpeg() {
    let verifier = create_verifier();

    // Parse JPEG to DocItems (with OCR)
    let backend = JpegBackend;
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/jpeg/circles.jpg"
            ),
            &Default::default(),
        )
        .expect("Failed to parse JPEG");

    // Export to JSON (the complete representation)
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", result.content_blocks.is_some());

    // Read original JPEG for comparison
    let jpeg_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/jpeg/circles.jpg"
    ));

    // LLM validates: Does JSON contain text extracted from JPEG?
    let prompt = format!(
        r#"Analyze if this DocItem JSON contains text extracted from the JPEG image via OCR.

ORIGINAL IMAGE: {}
(Open and analyze the JPEG - does it contain text?)

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness for IMAGE WITH TEXT:

1. **Completeness (0-100)**: All visible text extracted via OCR?
2. **Accuracy (0-100)**: OCR text semantically correct (spelling, words)?
3. **Structure (0-100)**: Text layout preserved (paragraphs, sections)?
4. **Formatting (0-100)**: Text structure represented in DocItems?
5. **Metadata (0-100)**: Image metadata (dimensions, format) captured?

For images WITHOUT visible text:
- Completeness: 100 (nothing to extract)
- Accuracy: 100 (no OCR errors possible)
- Structure: 100 (no structure to preserve)
- Formatting: 100 (no formatting to preserve)
- Metadata: Check dimensions, format are captured

For each category with score < 95:
- List specific issues
- Assign severity: "critical", "major", "minor", or "info"
- Provide location if identifiable

Calculate overall_score as weighted average:
- completeness: 30%
- accuracy: 30%
- structure: 20%
- formatting: 15%
- metadata: 5%

Return JSON ONLY (no markdown, no explanation):
{{
  "overall_score": 0.0-1.0,
  "category_scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        jpeg_path.display(),
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: JPEG ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Completeness: {}/100",
        quality.category_scores.completeness
    );
    println!("  Accuracy:     {}/100", quality.category_scores.accuracy);
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Formatting:   {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    if !quality.findings.is_empty() {
        println!("\nDocItem Gaps:");
        for finding in &quality.findings {
            println!("  - {}", finding.description);
        }
    }

    assert!(
        quality.score >= 0.90,
        "DocItem completeness: {:.1}% (need 90%, OCR may have minor errors)",
        quality.score * 100.0
    );
}

/// Test TIFF DocItem completeness via JSON
///
/// MANDATORY TEST - NOT IGNORED
/// This validates TIFF parser extracts text via OCR to DocItems
#[tokio::test]
async fn test_llm_docitem_tiff() {
    let verifier = create_verifier();

    // Parse TIFF to DocItems (with OCR)
    let backend = TiffBackend;
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/tiff/grayscale.tiff"
            ),
            &Default::default(),
        )
        .expect("Failed to parse TIFF");

    // Export to JSON (the complete representation)
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", result.content_blocks.is_some());

    // Read original TIFF for comparison
    let tiff_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/tiff/grayscale.tiff"
    ));

    // LLM validates: Does JSON contain text extracted from TIFF?
    let prompt = format!(
        r#"Analyze if this DocItem JSON contains text extracted from the TIFF image via OCR.

ORIGINAL IMAGE: {}
(Open and analyze the TIFF - does it contain text?)

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness for IMAGE WITH TEXT:

1. **Completeness (0-100)**: All visible text extracted via OCR?
2. **Accuracy (0-100)**: OCR text semantically correct (spelling, words)?
3. **Structure (0-100)**: Text layout preserved (paragraphs, sections)?
4. **Formatting (0-100)**: Text structure represented in DocItems?
5. **Metadata (0-100)**: Image metadata (dimensions, format) captured?

For images WITHOUT visible text:
- Completeness: 100 (nothing to extract)
- Accuracy: 100 (no OCR errors possible)
- Structure: 100 (no structure to preserve)
- Formatting: 100 (no formatting to preserve)
- Metadata: Check dimensions, format are captured

For each category with score < 95:
- List specific issues
- Assign severity: "critical", "major", "minor", or "info"
- Provide location if identifiable

Calculate overall_score as weighted average:
- completeness: 30%
- accuracy: 30%
- structure: 20%
- formatting: 15%
- metadata: 5%

Return JSON ONLY (no markdown, no explanation):
{{
  "overall_score": 0.0-1.0,
  "category_scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        tiff_path.display(),
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: TIFF ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Completeness: {}/100",
        quality.category_scores.completeness
    );
    println!("  Accuracy:     {}/100", quality.category_scores.accuracy);
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Formatting:   {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    if !quality.findings.is_empty() {
        println!("\nDocItem Gaps:");
        for finding in &quality.findings {
            println!("  - {}", finding.description);
        }
    }

    assert!(
        quality.score >= 0.90,
        "DocItem completeness: {:.1}% (need 90%, OCR may have minor errors)",
        quality.score * 100.0
    );
}

/// Test WEBP DocItem completeness via JSON
///
/// MANDATORY TEST - NOT IGNORED
/// This validates WEBP parser extracts text via OCR to DocItems
#[tokio::test]
async fn test_llm_docitem_webp() {
    let verifier = create_verifier();

    // Parse WEBP to DocItems (with OCR)
    let backend = WebpBackend;
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/webp/sample_graphic.webp"
            ),
            &Default::default(),
        )
        .expect("Failed to parse WEBP");

    // Export to JSON (the complete representation)
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", result.content_blocks.is_some());

    // Read original WEBP for comparison
    let webp_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/webp/sample_graphic.webp"
    ));

    // LLM validates: Does JSON contain text extracted from WEBP?
    let prompt = format!(
        r#"Analyze if this DocItem JSON contains text extracted from the WEBP image via OCR.

ORIGINAL IMAGE: {}
(Open and analyze the WEBP - does it contain text?)

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness for IMAGE WITH TEXT:

1. **Completeness (0-100)**: All visible text extracted via OCR?
2. **Accuracy (0-100)**: OCR text semantically correct (spelling, words)?
3. **Structure (0-100)**: Text layout preserved (paragraphs, sections)?
4. **Formatting (0-100)**: Text structure represented in DocItems?
5. **Metadata (0-100)**: Image metadata (dimensions, format) captured?

For images WITHOUT visible text:
- Completeness: 100 (nothing to extract)
- Accuracy: 100 (no OCR errors possible)
- Structure: 100 (no structure to preserve)
- Formatting: 100 (no formatting to preserve)
- Metadata: Check dimensions, format are captured

For each category with score < 95:
- List specific issues
- Assign severity: "critical", "major", "minor", or "info"
- Provide location if identifiable

Calculate overall_score as weighted average:
- completeness: 30%
- accuracy: 30%
- structure: 20%
- formatting: 15%
- metadata: 5%

Return JSON ONLY (no markdown, no explanation):
{{
  "overall_score": 0.0-1.0,
  "category_scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        webp_path.display(),
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: WEBP ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Completeness: {}/100",
        quality.category_scores.completeness
    );
    println!("  Accuracy:     {}/100", quality.category_scores.accuracy);
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Formatting:   {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    if !quality.findings.is_empty() {
        println!("\nDocItem Gaps:");
        for finding in &quality.findings {
            println!("  - {}", finding.description);
        }
    }

    assert!(
        quality.score >= 0.90,
        "DocItem completeness: {:.1}% (need 90%, OCR may have minor errors)",
        quality.score * 100.0
    );
}

/// Test BMP DocItem completeness via JSON
///
/// MANDATORY TEST - NOT IGNORED
/// This validates BMP parser extracts text via OCR to DocItems
#[tokio::test]
async fn test_llm_docitem_bmp() {
    let verifier = create_verifier();

    // Parse BMP to DocItems (with OCR)
    let backend = BmpBackend;
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/bmp/gradient.bmp"
            ),
            &Default::default(),
        )
        .expect("Failed to parse BMP");

    // Export to JSON (the complete representation)
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", result.content_blocks.is_some());

    // Read original BMP for comparison
    let bmp_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/bmp/gradient.bmp"
    ));

    // LLM validates: Does JSON contain text extracted from BMP?
    let prompt = format!(
        r#"Analyze if this DocItem JSON contains text extracted from the BMP image via OCR.

ORIGINAL IMAGE: {}
(Open and analyze the BMP - does it contain text?)

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness for IMAGE WITH TEXT:

1. **Completeness (0-100)**: All visible text extracted via OCR?
2. **Accuracy (0-100)**: OCR text semantically correct (spelling, words)?
3. **Structure (0-100)**: Text layout preserved (paragraphs, sections)?
4. **Formatting (0-100)**: Text structure represented in DocItems?
5. **Metadata (0-100)**: Image metadata (dimensions, format) captured?

For images WITHOUT visible text:
- Completeness: 100 (nothing to extract)
- Accuracy: 100 (no OCR errors possible)
- Structure: 100 (no structure to preserve)
- Formatting: 100 (no formatting to preserve)
- Metadata: Check dimensions, format are captured

For each category with score < 95:
- List specific issues
- Assign severity: "critical", "major", "minor", or "info"
- Provide location if identifiable

Calculate overall_score as weighted average:
- completeness: 30%
- accuracy: 30%
- structure: 20%
- formatting: 15%
- metadata: 5%

Return JSON ONLY (no markdown, no explanation):
{{
  "overall_score": 0.0-1.0,
  "category_scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        bmp_path.display(),
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: BMP ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Completeness: {}/100",
        quality.category_scores.completeness
    );
    println!("  Accuracy:     {}/100", quality.category_scores.accuracy);
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Formatting:   {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    if !quality.findings.is_empty() {
        println!("\nDocItem Gaps:");
        for finding in &quality.findings {
            println!("  - {}", finding.description);
        }
    }

    assert!(
        quality.score >= 0.90,
        "DocItem completeness: {:.1}% (need 90%, OCR may have minor errors)",
        quality.score * 100.0
    );
}

/// Test ZIP DocItem completeness via JSON
///
/// MANDATORY TEST - NOT IGNORED
/// This validates ZIP parser extracts file list to DocItems
#[tokio::test]
async fn test_llm_docitem_zip() {
    let verifier = create_verifier();

    // Parse ZIP to DocItems
    let backend = ArchiveBackend::new(InputFormat::Zip).expect("Failed to create ZIP backend");
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/archives/zip/simple.zip"
            ),
            &Default::default(),
        )
        .expect("Failed to parse ZIP");

    // Export to JSON (the complete representation)
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", result.content_blocks.is_some());

    // Read original ZIP for comparison
    let zip_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/archives/zip/simple.zip"
    ));

    // LLM validates: Does JSON contain ALL information from ZIP?
    let prompt = format!(
        r#"Analyze if this DocItem JSON contains complete information from the original ZIP archive.

ORIGINAL ARCHIVE: {}
(Open and analyze the ZIP - list all contained files and metadata)

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness for ARCHIVE:

1. **Completeness (0-100)**: All files in archive listed?
2. **Accuracy (0-100)**: File names, sizes, paths correct?
3. **Structure (0-100)**: Directory hierarchy preserved?
4. **Formatting (0-100)**: Archive listing formatted correctly in DocItems?
5. **Metadata (0-100)**: Archive metadata (total files, sizes) captured?

For archives:
- Completeness: Check every file is listed (match file count)
- Accuracy: Check file names and sizes are correct
- Structure: Check directory paths preserved
- Formatting: Check appropriate DocItem types used
- Metadata: Check archive-level statistics captured

For each category with score < 95:
- List specific issues
- Assign severity: "critical", "major", "minor", or "info"
- Provide location if identifiable

Calculate overall_score as weighted average:
- completeness: 30%
- accuracy: 30%
- structure: 20%
- formatting: 15%
- metadata: 5%

Return JSON ONLY (no markdown, no explanation):
{{
  "overall_score": 0.0-1.0,
  "category_scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        zip_path.display(),
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: ZIP ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Completeness: {}/100",
        quality.category_scores.completeness
    );
    println!("  Accuracy:     {}/100", quality.category_scores.accuracy);
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Formatting:   {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    if !quality.findings.is_empty() {
        println!("\nDocItem Gaps:");
        for finding in &quality.findings {
            println!("  - {}", finding.description);
        }
    }

    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}

/// Test TAR DocItem completeness via JSON
///
/// MANDATORY TEST - NOT IGNORED
/// This validates TAR parser extracts file list to DocItems
#[tokio::test]
async fn test_llm_docitem_tar() {
    let verifier = create_verifier();

    // Parse TAR to DocItems
    let backend = ArchiveBackend::new(InputFormat::Tar).expect("Failed to create TAR backend");
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/archives/tar/simple.tar"
            ),
            &Default::default(),
        )
        .expect("Failed to parse TAR");

    // Export to JSON (the complete representation)
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", result.content_blocks.is_some());

    // Read original TAR for comparison
    let tar_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/archives/tar/simple.tar"
    ));

    // LLM validates: Does JSON contain ALL information from TAR?
    let prompt = format!(
        r#"Analyze if this DocItem JSON contains complete information from the original TAR archive.

ORIGINAL ARCHIVE: {}
(Open and analyze the TAR - list all contained files and metadata)

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness for ARCHIVE:

1. **Completeness (0-100)**: All files in archive listed?
2. **Accuracy (0-100)**: File names, sizes, paths correct?
3. **Structure (0-100)**: Directory hierarchy preserved?
4. **Formatting (0-100)**: Archive listing formatted correctly in DocItems?
5. **Metadata (0-100)**: Archive metadata (total files, sizes) captured?

For archives:
- Completeness: Check every file is listed (match file count)
- Accuracy: Check file names and sizes are correct
- Structure: Check directory paths preserved
- Formatting: Check appropriate DocItem types used
- Metadata: Check archive-level statistics captured

For each category with score < 95:
- List specific issues
- Assign severity: "critical", "major", "minor", or "info"
- Provide location if identifiable

Calculate overall_score as weighted average:
- completeness: 30%
- accuracy: 30%
- structure: 20%
- formatting: 15%
- metadata: 5%

Return JSON ONLY (no markdown, no explanation):
{{
  "overall_score": 0.0-1.0,
  "category_scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        tar_path.display(),
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: TAR ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Completeness: {}/100",
        quality.category_scores.completeness
    );
    println!("  Accuracy:     {}/100", quality.category_scores.accuracy);
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Formatting:   {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    if !quality.findings.is_empty() {
        println!("\nDocItem Gaps:");
        for finding in &quality.findings {
            println!("  - {}", finding.description);
        }
    }

    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}

/// Test EML DocItem completeness via JSON
///
/// MANDATORY TEST - NOT IGNORED
/// This validates EML parser extracts email headers and body to DocItems
#[tokio::test]
async fn test_llm_docitem_eml() {
    let verifier = create_verifier();

    // Parse EML to DocItems
    let backend = EmailBackend::new(InputFormat::Eml).expect("Failed to create EML backend");
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/email/eml/html_email.eml"
            ),
            &Default::default(),
        )
        .expect("Failed to parse EML");

    // Export to JSON (the complete representation)
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", result.content_blocks.is_some());

    // Read original EML for comparison
    let eml_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/email/eml/html_email.eml"
    ));
    let eml_content = std::fs::read_to_string(eml_path).expect("Failed to read EML");

    // LLM validates: Does JSON contain ALL information from EML?
    let prompt = format!(
        r#"Analyze if this DocItem JSON contains complete information from the original EML email.

ORIGINAL EMAIL:
```eml
{}
```

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness for EMAIL:

1. **Completeness (0-100)**: All headers (From, To, Subject, Date) and body extracted?
2. **Accuracy (0-100)**: Email content semantically correct?
3. **Structure (0-100)**: Email structure (headers + body) preserved?
4. **Formatting (0-100)**: Email formatting (HTML, plain text) represented in DocItems?
5. **Metadata (0-100)**: Email metadata (timestamp, sender, recipients) captured?

For emails:
- Completeness: Check From, To, Subject, Date, and body content all present
- Accuracy: Check email addresses and content are correct
- Structure: Check headers vs body clearly separated
- Formatting: Check HTML converted to text if present
- Metadata: Check all email headers captured

For each category with score < 95:
- List specific issues
- Assign severity: "critical", "major", "minor", or "info"
- Provide location if identifiable

Calculate overall_score as weighted average:
- completeness: 30%
- accuracy: 30%
- structure: 20%
- formatting: 15%
- metadata: 5%

Return JSON ONLY (no markdown, no explanation):
{{
  "overall_score": 0.0-1.0,
  "category_scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        eml_content,
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: EML ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Completeness: {}/100",
        quality.category_scores.completeness
    );
    println!("  Accuracy:     {}/100", quality.category_scores.accuracy);
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Formatting:   {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    if !quality.findings.is_empty() {
        println!("\nDocItem Gaps:");
        for finding in &quality.findings {
            println!("  - {}", finding.description);
        }
    }

    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}

/// Test MBOX DocItem completeness via JSON
///
/// MANDATORY TEST - NOT IGNORED
/// This validates MBOX parser extracts all emails to DocItems
#[tokio::test]
async fn test_llm_docitem_mbox() {
    let verifier = create_verifier();

    // Parse MBOX to DocItems
    let backend = EmailBackend::new(InputFormat::Mbox).expect("Failed to create MBOX backend");
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/email/mbox/archive.mbox"
            ),
            &Default::default(),
        )
        .expect("Failed to parse MBOX");

    // Export to JSON (the complete representation)
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", result.content_blocks.is_some());

    // Read original MBOX for comparison
    let mbox_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/email/mbox/archive.mbox"
    ));
    let mbox_content = std::fs::read_to_string(mbox_path).expect("Failed to read MBOX");

    // LLM validates: Does JSON contain ALL information from MBOX?
    let prompt = format!(
        r#"Analyze if this DocItem JSON contains complete information from the original MBOX mailbox.

ORIGINAL MAILBOX:
```mbox
{}
```

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness for MAILBOX:

1. **Completeness (0-100)**: All emails in mailbox extracted?
2. **Accuracy (0-100)**: Email content semantically correct?
3. **Structure (0-100)**: Mailbox structure (multiple emails) preserved?
4. **Formatting (0-100)**: Email formatting (headers + bodies) represented in DocItems?
5. **Metadata (0-100)**: Mailbox metadata (email count, timestamps) captured?

For mailboxes:
- Completeness: Check all emails present (count messages in MBOX)
- Accuracy: Check email addresses and content are correct
- Structure: Check emails clearly separated
- Formatting: Check each email has headers + body
- Metadata: Check mailbox-level statistics captured

For each category with score < 95:
- List specific issues
- Assign severity: "critical", "major", "minor", or "info"
- Provide location if identifiable

Calculate overall_score as weighted average:
- completeness: 30%
- accuracy: 30%
- structure: 20%
- formatting: 15%
- metadata: 5%

Return JSON ONLY (no markdown, no explanation):
{{
  "overall_score": 0.0-1.0,
  "category_scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        mbox_content,
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: MBOX ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Completeness: {}/100",
        quality.category_scores.completeness
    );
    println!("  Accuracy:     {}/100", quality.category_scores.accuracy);
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Formatting:   {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    if !quality.findings.is_empty() {
        println!("\nDocItem Gaps:");
        for finding in &quality.findings {
            println!("  - {}", finding.description);
        }
    }

    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}

/// Test EPUB DocItem completeness via JSON
///
/// MANDATORY TEST - NOT IGNORED
/// This validates EPUB parser extracts book metadata and chapters to DocItems
#[tokio::test]
async fn test_llm_docitem_epub() {
    let verifier = create_verifier();

    // Parse EPUB to DocItems
    let backend = EbooksBackend::new(InputFormat::Epub).expect("Failed to create EPUB backend");
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/ebooks/epub/simple.epub"
            ),
            &Default::default(),
        )
        .expect("Failed to parse EPUB");

    // Export to JSON (the complete representation)
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", result.content_blocks.is_some());

    // Read original EPUB for comparison
    let epub_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/ebooks/epub/simple.epub"
    ));

    // LLM validates: Does JSON contain ALL information from EPUB?
    let prompt = format!(
        r#"Analyze if this DocItem JSON contains complete information from the original EPUB ebook.

ORIGINAL EBOOK: {}
(Open and analyze the EPUB - check metadata, chapters, table of contents)

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness for EBOOK:

1. **Completeness (0-100)**: All chapters, metadata (title, author), TOC extracted?
2. **Accuracy (0-100)**: Content semantically correct?
3. **Structure (0-100)**: Book structure (chapters, sections) preserved?
4. **Formatting (0-100)**: Chapter headings, paragraphs formatted correctly in DocItems?
5. **Metadata (0-100)**: Book metadata (author, publisher, ISBN) captured?

For ebooks:
- Completeness: Check all chapters present, metadata complete
- Accuracy: Check chapter content correct
- Structure: Check chapter hierarchy preserved
- Formatting: Check headings, paragraphs properly structured
- Metadata: Check title, author, publisher, etc. captured

For each category with score < 95:
- List specific issues
- Assign severity: "critical", "major", "minor", or "info"
- Provide location if identifiable

Calculate overall_score as weighted average:
- completeness: 30%
- accuracy: 30%
- structure: 20%
- formatting: 15%
- metadata: 5%

Return JSON ONLY (no markdown, no explanation):
{{
  "overall_score": 0.0-1.0,
  "category_scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        epub_path.display(),
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: EPUB ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Completeness: {}/100",
        quality.category_scores.completeness
    );
    println!("  Accuracy:     {}/100", quality.category_scores.accuracy);
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Formatting:   {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    if !quality.findings.is_empty() {
        println!("\nDocItem Gaps:");
        for finding in &quality.findings {
            println!("  - {}", finding.description);
        }
    }

    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}

/// Test ODT DocItem completeness via JSON
///
/// MANDATORY TEST - NOT IGNORED
/// This validates ODT parser extracts document content and structure to DocItems
#[tokio::test]
async fn test_llm_docitem_odt() {
    let verifier = create_verifier();

    // Parse ODT to DocItems
    let backend = OpenDocumentBackend::new(InputFormat::Odt).expect("Failed to create ODT backend");
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/opendocument/odt/simple_text.odt"
            ),
            &Default::default(),
        )
        .expect("Failed to parse ODT");

    // Export to JSON (the complete representation)
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", result.content_blocks.is_some());

    // Read original ODT for comparison
    let odt_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/opendocument/odt/simple_text.odt"
    ));

    // LLM validates: Does JSON contain ALL information from ODT?
    let prompt = format!(
        r#"Analyze if this DocItem JSON contains complete information from the original ODT (OpenDocument Text) file.

ORIGINAL DOCUMENT: {}
(Open and analyze the ODT - check paragraphs, headings, formatting, metadata)

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness for ODT:

1. **Completeness (0-100)**: All paragraphs, headings, lists, tables extracted?
2. **Accuracy (0-100)**: Content semantically correct?
3. **Structure (0-100)**: Document hierarchy (headings, sections) preserved?
4. **Formatting (0-100)**: Paragraphs, lists, tables formatted correctly in DocItems?
5. **Metadata (0-100)**: Document properties (title, author, etc.) captured?

For ODT documents:
- Completeness: Check all text content present
- Accuracy: Check content correct
- Structure: Check heading hierarchy preserved
- Formatting: Check lists, tables, paragraphs properly structured
- Metadata: Check document properties captured

For each category with score < 95:
- List specific issues
- Assign severity: "critical", "major", "minor", or "info"
- Provide location if identifiable

Calculate overall_score as weighted average:
- completeness: 30%
- accuracy: 30%
- structure: 20%
- formatting: 15%
- metadata: 5%

Return JSON ONLY (no markdown, no explanation):
{{
  "overall_score": 0.0-1.0,
  "category_scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        odt_path.display(),
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: ODT ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Completeness: {}/100",
        quality.category_scores.completeness
    );
    println!("  Accuracy:     {}/100", quality.category_scores.accuracy);
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Formatting:   {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    if !quality.findings.is_empty() {
        println!("\nDocItem Gaps:");
        for finding in &quality.findings {
            println!("  - {}", finding.description);
        }
    }

    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}

/// Test ODS DocItem completeness via JSON
///
/// MANDATORY TEST - NOT IGNORED
/// This validates ODS parser extracts spreadsheet data and structure to DocItems
#[tokio::test]
async fn test_llm_docitem_ods() {
    let verifier = create_verifier();

    // Parse ODS to DocItems
    let backend = OpenDocumentBackend::new(InputFormat::Ods).expect("Failed to create ODS backend");
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/opendocument/ods/simple_spreadsheet.ods"
            ),
            &Default::default(),
        )
        .expect("Failed to parse ODS");

    // Export to JSON (the complete representation)
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", result.content_blocks.is_some());

    // Read original ODS for comparison
    let ods_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/opendocument/ods/simple_spreadsheet.ods"
    ));

    // LLM validates: Does JSON contain ALL information from ODS?
    let prompt = format!(
        r#"Analyze if this DocItem JSON contains complete information from the original ODS (OpenDocument Spreadsheet) file.

ORIGINAL SPREADSHEET: {}
(Open and analyze the ODS - check sheets, rows, columns, data, formulas)

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness for ODS:

1. **Completeness (0-100)**: All sheets, rows, columns, data extracted?
2. **Accuracy (0-100)**: Cell values semantically correct?
3. **Structure (0-100)**: Sheet structure, table headers preserved?
4. **Formatting (0-100)**: Tables formatted correctly in DocItems?
5. **Metadata (0-100)**: Spreadsheet properties (sheet names, etc.) captured?

For ODS spreadsheets:
- Completeness: Check all sheets and data present
- Accuracy: Check cell values correct
- Structure: Check sheet hierarchy and table structure preserved
- Formatting: Check tables properly structured
- Metadata: Check sheet names and properties captured

For each category with score < 95:
- List specific issues
- Assign severity: "critical", "major", "minor", or "info"
- Provide location if identifiable

Calculate overall_score as weighted average:
- completeness: 30%
- accuracy: 30%
- structure: 20%
- formatting: 15%
- metadata: 5%

Return JSON ONLY (no markdown, no explanation):
{{
  "overall_score": 0.0-1.0,
  "category_scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        ods_path.display(),
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: ODS ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Completeness: {}/100",
        quality.category_scores.completeness
    );
    println!("  Accuracy:     {}/100", quality.category_scores.accuracy);
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Formatting:   {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    if !quality.findings.is_empty() {
        println!("\nDocItem Gaps:");
        for finding in &quality.findings {
            println!("  - {}", finding.description);
        }
    }

    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}

/// Test ODP DocItem completeness via JSON
///
/// MANDATORY TEST - NOT IGNORED
/// This validates ODP parser extracts presentation content and structure to DocItems
#[tokio::test]
async fn test_llm_docitem_odp() {
    let verifier = create_verifier();

    // Parse ODP to DocItems
    let backend = OpenDocumentBackend::new(InputFormat::Odp).expect("Failed to create ODP backend");
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/opendocument/odp/simple_presentation.odp"
            ),
            &Default::default(),
        )
        .expect("Failed to parse ODP");

    // Export to JSON (the complete representation)
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", result.content_blocks.is_some());

    // Read original ODP for comparison
    let odp_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/opendocument/odp/simple_presentation.odp"
    ));

    // LLM validates: Does JSON contain ALL information from ODP?
    let prompt = format!(
        r#"Analyze if this DocItem JSON contains complete information from the original ODP (OpenDocument Presentation) file.

ORIGINAL PRESENTATION: {}
(Open and analyze the ODP - check slides, titles, bullet points, content)

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness for ODP:

1. **Completeness (0-100)**: All slides, titles, bullet points, content extracted?
2. **Accuracy (0-100)**: Content semantically correct?
3. **Structure (0-100)**: Slide hierarchy, bullet lists preserved?
4. **Formatting (0-100)**: Slides, lists formatted correctly in DocItems?
5. **Metadata (0-100)**: Presentation properties (slide count, etc.) captured?

For ODP presentations:
- Completeness: Check all slides and content present
- Accuracy: Check slide content correct
- Structure: Check slide hierarchy and list structure preserved
- Formatting: Check slides and lists properly structured
- Metadata: Check slide count and properties captured

For each category with score < 95:
- List specific issues
- Assign severity: "critical", "major", "minor", or "info"
- Provide location if identifiable

Calculate overall_score as weighted average:
- completeness: 30%
- accuracy: 30%
- structure: 20%
- formatting: 15%
- metadata: 5%

Return JSON ONLY (no markdown, no explanation):
{{
  "overall_score": 0.0-1.0,
  "category_scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        odp_path.display(),
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: ODP ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Completeness: {}/100",
        quality.category_scores.completeness
    );
    println!("  Accuracy:     {}/100", quality.category_scores.accuracy);
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Formatting:   {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    if !quality.findings.is_empty() {
        println!("\nDocItem Gaps:");
        for finding in &quality.findings {
            println!("  - {}", finding.description);
        }
    }

    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}

/// Test RTF DocItem completeness via JSON
///
/// MANDATORY TEST - NOT IGNORED
/// This validates RTF parser extracts document content and formatting to DocItems
#[tokio::test]
async fn test_llm_docitem_rtf() {
    let verifier = create_verifier();

    // Parse RTF to DocItems
    let backend = RtfBackend;
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/legacy/rtf/simple_text.rtf"
            ),
            &Default::default(),
        )
        .expect("Failed to parse RTF");

    // Export to JSON (the complete representation)
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", result.content_blocks.is_some());

    // Read original RTF for comparison
    let rtf_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/legacy/rtf/simple_text.rtf"
    ));
    let rtf_content = std::fs::read_to_string(rtf_path).expect("Failed to read RTF");

    // LLM validates: Does JSON contain ALL information from RTF?
    let prompt = format!(
        r#"Analyze if this DocItem JSON contains complete information from the original RTF (Rich Text Format) file.

ORIGINAL DOCUMENT:
```rtf
{}
```
(Analyze the RTF - check paragraphs, headings, formatting, lists)

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness for RTF:

1. **Completeness (0-100)**: All paragraphs, headings, lists, tables extracted?
2. **Accuracy (0-100)**: Content semantically correct?
3. **Structure (0-100)**: Document hierarchy preserved?
4. **Formatting (0-100)**: Paragraphs, lists formatted correctly in DocItems?
5. **Metadata (0-100)**: Document properties captured?

For RTF documents:
- Completeness: Check all text content present
- Accuracy: Check content correct
- Structure: Check document structure preserved
- Formatting: Check formatting properly captured
- Metadata: Check document properties captured

For each category with score < 95:
- List specific issues
- Assign severity: "critical", "major", "minor", or "info"
- Provide location if identifiable

Calculate overall_score as weighted average:
- completeness: 30%
- accuracy: 30%
- structure: 20%
- formatting: 15%
- metadata: 5%

Return JSON ONLY (no markdown, no explanation):
{{
  "overall_score": 0.0-1.0,
  "category_scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        rtf_content,
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: RTF ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Completeness: {}/100",
        quality.category_scores.completeness
    );
    println!("  Accuracy:     {}/100", quality.category_scores.accuracy);
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Formatting:   {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    if !quality.findings.is_empty() {
        println!("\nDocItem Gaps:");
        for finding in &quality.findings {
            println!("  - {}", finding.description);
        }
    }

    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}

/// Test GIF DocItem completeness via JSON
///
/// MANDATORY TEST - NOT IGNORED
/// This validates GIF parser extracts image content to DocItems
#[tokio::test]
async fn test_llm_docitem_gif() {
    let verifier = create_verifier();

    // Parse GIF to DocItems
    let backend = GifBackend;
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/images/gif/simple.gif"
            ),
            &Default::default(),
        )
        .expect("Failed to parse GIF");

    // Export to JSON (the complete representation)
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", result.content_blocks.is_some());

    // Read original GIF for comparison
    let gif_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/images/gif/simple.gif"
    ));

    // LLM validates: Does JSON contain text from GIF?
    let prompt = format!(
        r#"Analyze if this DocItem JSON contains text content extracted from the GIF image via OCR.

ORIGINAL GIF IMAGE: {}

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness for GIF IMAGE:

1. **Completeness (0-100)**: All visible text extracted via OCR?
2. **Accuracy (0-100)**: Extracted text semantically correct?
3. **Structure (0-100)**: Text layout preserved?
4. **Formatting (0-100)**: Text blocks properly structured?
5. **Metadata (0-100)**: Image properties (dimensions, etc.) captured?

For images (GIF):
- Completeness: Check OCR extracted all visible text
- Accuracy: Check OCR text correct
- Structure: Check text layout preserved
- Formatting: Check text blocks properly structured
- Metadata: Check image properties captured

Note: Images use OCR, so 90% threshold is acceptable (vs 95% for text formats)

For each category with score < 90:
- List specific issues
- Assign severity: "critical", "major", "minor", or "info"
- Provide location if identifiable

Calculate overall_score as weighted average:
- completeness: 30%
- accuracy: 30%
- structure: 20%
- formatting: 15%
- metadata: 5%

Return JSON ONLY (no markdown, no explanation):
{{
  "overall_score": 0.0-1.0,
  "category_scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        gif_path.display(),
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: GIF ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Completeness: {}/100",
        quality.category_scores.completeness
    );
    println!("  Accuracy:     {}/100", quality.category_scores.accuracy);
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Formatting:   {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    if !quality.findings.is_empty() {
        println!("\nDocItem Gaps:");
        for finding in &quality.findings {
            println!("  - {}", finding.description);
        }
    }

    // Images use OCR, so 90% threshold (vs 95% for text formats)
    assert!(
        quality.score >= 0.90,
        "DocItem completeness: {:.1}% (need 90% for images)",
        quality.score * 100.0
    );
}

// Test DocItem completeness, NOT output formatting!

// =======================================================================================
// NEW TESTS (N=1351): SVG, 7Z, RAR, VCF, ICS
// =======================================================================================

#[tokio::test]
async fn test_llm_docitem_svg() {
    let verifier = create_verifier();

    // Parse SVG to DocItems
    let backend = SvgBackend;
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/svg/simple_icon.svg"
            ),
            &Default::default(),
        )
        .expect("Failed to parse SVG");

    // Export to JSON (the complete representation)
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", result.content_blocks.is_some());

    // Read original SVG for comparison
    let svg_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/svg/simple_icon.svg"
    ));
    let svg_content = std::fs::read_to_string(svg_path).expect("Failed to read SVG");

    // LLM validates: Does JSON contain ALL information from SVG?
    let prompt = format!(
        r#"Analyze if this DocItem JSON contains complete information from the original SVG vector graphic.

ORIGINAL SVG:
```xml
{}
```
(Analyze the SVG - check for text elements, paths, shapes, and metadata)

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness for SVG (vector graphics):

1. **Completeness (0-100)**: All text elements and descriptions extracted?
2. **Accuracy (0-100)**: SVG content correctly represented?
3. **Structure (0-100)**: Hierarchy of elements preserved?
4. **Formatting (0-100)**: Text properly formatted?
5. **Metadata (0-100)**: SVG properties (viewBox, dimensions) captured?

For SVG files:
- Completeness: All <text> elements and title/desc tags extracted
- Accuracy: Text content matches SVG
- Structure: Element hierarchy preserved
- Formatting: Text blocks properly structured
- Metadata: SVG dimensions, viewBox captured

Respond with JSON:
{{
  "scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        svg_content,
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: SVG ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Completeness: {}/100",
        quality.category_scores.completeness
    );
    println!("  Accuracy:     {}/100", quality.category_scores.accuracy);
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Formatting:   {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    if !quality.findings.is_empty() {
        println!("\nDocItem Gaps:");
        for finding in &quality.findings {
            println!("  - {}", finding.description);
        }
    }

    // SVG is structured text format, use 95% threshold
    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}

#[tokio::test]
async fn test_llm_docitem_7z() {
    let verifier = create_verifier();

    // Parse 7Z to DocItems
    let backend = ArchiveBackend::new(InputFormat::SevenZ).expect("Failed to create 7Z backend");
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/archives/7z/simple_normal.7z"
            ),
            &Default::default(),
        )
        .expect("Failed to parse 7Z");

    // Export to JSON (the complete representation)
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", result.content_blocks.is_some());

    // Read original 7Z for comparison
    let sevenz_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/archives/7z/simple_normal.7z"
    ));

    // LLM validates: Does JSON contain ALL information from 7Z?
    let prompt = format!(
        r#"Analyze if this DocItem JSON contains complete information from the original 7Z archive.

ORIGINAL ARCHIVE: {}
(Open and analyze the 7Z - list all contained files and metadata)

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness for 7Z archive:

1. **Completeness (0-100)**: All files listed?
2. **Accuracy (0-100)**: File names and sizes correct?
3. **Structure (0-100)**: Directory hierarchy preserved?
4. **Formatting (0-100)**: Archive structure clear?
5. **Metadata (0-100)**: Compression info, dates captured?

For archives (7Z):
- Completeness: All files in archive listed
- Accuracy: File names match archive contents
- Structure: Folder hierarchy preserved
- Formatting: Clear archive structure presentation
- Metadata: File sizes, dates, compression ratio

Respond with JSON:
{{
  "scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        sevenz_path.display(),
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: 7Z ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Completeness: {}/100",
        quality.category_scores.completeness
    );
    println!("  Accuracy:     {}/100", quality.category_scores.accuracy);
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Formatting:   {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    if !quality.findings.is_empty() {
        println!("\nDocItem Gaps:");
        for finding in &quality.findings {
            println!("  - {}", finding.description);
        }
    }

    // Archives are structured listings, use 95% threshold
    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}

#[tokio::test]
async fn test_llm_docitem_rar() {
    let verifier = create_verifier();

    // Parse RAR to DocItems
    let backend = ArchiveBackend::new(InputFormat::Rar).expect("Failed to create RAR backend");
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/archives/rar/nested.rar"
            ),
            &Default::default(),
        )
        .expect("Failed to parse RAR");

    // Export to JSON (the complete representation)
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", result.content_blocks.is_some());

    // Read original RAR for comparison
    let rar_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/archives/rar/nested.rar"
    ));

    // LLM validates: Does JSON contain ALL information from RAR?
    let prompt = format!(
        r#"Analyze if this DocItem JSON contains complete information from the original RAR archive.

ORIGINAL ARCHIVE: {}
(Open and analyze the RAR - list all contained files and metadata)

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness for RAR archive:

1. **Completeness (0-100)**: All files listed?
2. **Accuracy (0-100)**: File names and sizes correct?
3. **Structure (0-100)**: Directory hierarchy preserved?
4. **Formatting (0-100)**: Archive structure clear?
5. **Metadata (0-100)**: Compression info, dates captured?

For archives (RAR):
- Completeness: All files in archive listed
- Accuracy: File names match archive contents
- Structure: Folder hierarchy preserved
- Formatting: Clear archive structure presentation
- Metadata: File sizes, dates, compression ratio

Respond with JSON:
{{
  "scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        rar_path.display(),
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: RAR ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Completeness: {}/100",
        quality.category_scores.completeness
    );
    println!("  Accuracy:     {}/100", quality.category_scores.accuracy);
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Formatting:   {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    if !quality.findings.is_empty() {
        println!("\nDocItem Gaps:");
        for finding in &quality.findings {
            println!("  - {}", finding.description);
        }
    }

    // Archives are structured listings, use 95% threshold
    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}

#[tokio::test]
async fn test_llm_docitem_vcf() {
    let verifier = create_verifier();

    // Parse VCF to DocItems
    let backend = EmailBackend::new(InputFormat::Vcf).expect("Failed to create VCF backend");
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/email/vcf/business_card.vcf"
            ),
            &Default::default(),
        )
        .expect("Failed to parse VCF");

    // Export to JSON (the complete representation)
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", result.content_blocks.is_some());

    // Read original VCF for comparison
    let vcf_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/email/vcf/business_card.vcf"
    ));
    let vcf_content = std::fs::read_to_string(vcf_path).expect("Failed to read VCF");

    // LLM validates: Does JSON contain ALL information from VCF?
    let prompt = format!(
        r#"Analyze if this DocItem JSON contains complete information from the original VCF (vCard) contact file.

ORIGINAL VCF:
```
{}
```
(Analyze the VCF - check all contact fields)

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness for VCF (vCard):

1. **Completeness (0-100)**: All contact fields extracted (name, email, phone, etc.)?
2. **Accuracy (0-100)**: Field values correct?
3. **Structure (0-100)**: Contact organization preserved?
4. **Formatting (0-100)**: Fields clearly formatted?
5. **Metadata (0-100)**: Version, encoding captured?

For VCF files:
- Completeness: All vCard fields (FN, EMAIL, TEL, ADR, etc.) extracted
- Accuracy: Field values match VCF
- Structure: Contact structure clear
- Formatting: Fields properly formatted
- Metadata: Version, encoding present

Respond with JSON:
{{
  "scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        vcf_content,
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: VCF ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Completeness: {}/100",
        quality.category_scores.completeness
    );
    println!("  Accuracy:     {}/100", quality.category_scores.accuracy);
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Formatting:   {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    if !quality.findings.is_empty() {
        println!("\nDocItem Gaps:");
        for finding in &quality.findings {
            println!("  - {}", finding.description);
        }
    }

    // VCF is structured text format, use 95% threshold
    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}

#[tokio::test]
async fn test_llm_docitem_ics() {
    let verifier = create_verifier();

    // Parse ICS to DocItems
    let backend = IcsBackend;
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/calendar/ics/meeting.ics"
            ),
            &Default::default(),
        )
        .expect("Failed to parse ICS");

    // Export to JSON (the complete representation)
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", result.content_blocks.is_some());

    // Read original ICS for comparison
    let ics_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/calendar/ics/meeting.ics"
    ));
    let ics_content = std::fs::read_to_string(ics_path).expect("Failed to read ICS");

    // LLM validates: Does JSON contain ALL information from ICS?
    let prompt = format!(
        r#"Analyze if this DocItem JSON contains complete information from the original ICS (iCalendar) file.

ORIGINAL ICS:
```
{}
```
(Analyze the ICS - check all calendar events and fields)

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness for ICS (iCalendar):

1. **Completeness (0-100)**: All events and fields extracted (SUMMARY, DTSTART, DTEND, LOCATION, etc.)?
2. **Accuracy (0-100)**: Event details correct?
3. **Structure (0-100)**: Calendar organization preserved?
4. **Formatting (0-100)**: Events clearly formatted?
5. **Metadata (0-100)**: Calendar metadata (VERSION, PRODID) captured?

For ICS files:
- Completeness: All VEVENT components and properties extracted
- Accuracy: Event details match ICS
- Structure: Event hierarchy preserved
- Formatting: Events properly formatted
- Metadata: Calendar version, timezone info

Respond with JSON:
{{
  "scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        ics_content,
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: ICS ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Completeness: {}/100",
        quality.category_scores.completeness
    );
    println!("  Accuracy:     {}/100", quality.category_scores.accuracy);
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Formatting:   {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    if !quality.findings.is_empty() {
        println!("\nDocItem Gaps:");
        for finding in &quality.findings {
            println!("  - {}", finding.description);
        }
    }

    // ICS is structured text format, use 95% threshold
    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}

#[tokio::test]
async fn test_llm_docitem_fb2() {
    let verifier = create_verifier();

    // Parse FB2 to DocItems
    let backend = EbooksBackend::new(InputFormat::Fb2).expect("Failed to create FB2 backend");
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/ebooks/fb2/fiction_novel.fb2"
            ),
            &Default::default(),
        )
        .expect("Failed to parse FB2");

    // Export to JSON (the complete representation)
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", result.content_blocks.is_some());

    // Read original FB2 for comparison
    let fb2_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/ebooks/fb2/fiction_novel.fb2"
    ));

    // LLM validates: Does JSON contain ALL information from FB2?
    let prompt = format!(
        r#"Analyze if this DocItem JSON contains complete information from the original FB2 (FictionBook 2.0) ebook file.

ORIGINAL FB2: {}
(Analyze the FB2 XML structure - check all text, chapters, metadata)

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness for FB2 (FictionBook):

1. **Completeness (0-100)**: All chapters, text, and sections extracted?
2. **Accuracy (0-100)**: Book content correct (text, dialogues, descriptions)?
3. **Structure (0-100)**: Chapter hierarchy and structure preserved?
4. **Formatting (0-100)**: Text formatting (emphasis, strong, etc.) captured?
5. **Metadata (0-100)**: Book metadata (title, author, annotations) captured?

For FB2 ebooks:
- Completeness: All <section>, <p>, <title> elements extracted
- Accuracy: Text content matches FB2 XML
- Structure: Chapter/section hierarchy preserved
- Formatting: Emphasis, strong, style tags captured
- Metadata: <description>, <title-info>, <document-info> extracted

Respond with JSON:
{{
  "scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        fb2_path.display(),
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: FB2 ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Completeness: {}/100",
        quality.category_scores.completeness
    );
    println!("  Accuracy:     {}/100", quality.category_scores.accuracy);
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Formatting:   {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    if !quality.findings.is_empty() {
        println!("\nDocItem Gaps:");
        for finding in &quality.findings {
            println!("  - {}", finding.description);
        }
    }

    // FB2 is structured ebook format, use 95% threshold
    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}

#[tokio::test]
async fn test_llm_docitem_mobi() {
    let verifier = create_verifier();

    // Parse MOBI to DocItems
    let backend = EbooksBackend::new(InputFormat::Mobi).expect("Failed to create MOBI backend");
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/ebooks/mobi/multi_chapter.mobi"
            ),
            &Default::default(),
        )
        .expect("Failed to parse MOBI");

    // Export to JSON (the complete representation)
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", result.content_blocks.is_some());

    // Read original MOBI for comparison
    let mobi_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/ebooks/mobi/multi_chapter.mobi"
    ));

    // LLM validates: Does JSON contain ALL information from MOBI?
    let prompt = format!(
        r#"Analyze if this DocItem JSON contains complete information from the original MOBI ebook file.

ORIGINAL MOBI: {}
(Analyze the MOBI structure - check all chapters, text, metadata)

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness for MOBI (Mobipocket):

1. **Completeness (0-100)**: All chapters, text, and sections extracted?
2. **Accuracy (0-100)**: Book content correct (all text, no corruption)?
3. **Structure (0-100)**: Chapter hierarchy and navigation preserved?
4. **Formatting (0-100)**: Text formatting (bold, italic, etc.) captured?
5. **Metadata (0-100)**: Book metadata (title, author, publisher) captured?

For MOBI ebooks:
- Completeness: All chapters and text content extracted
- Accuracy: Text matches MOBI content
- Structure: Chapter/section hierarchy preserved
- Formatting: Bold, italic, underline captured
- Metadata: Book title, author, publication info extracted

Respond with JSON:
{{
  "scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        mobi_path.display(),
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: MOBI ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Completeness: {}/100",
        quality.category_scores.completeness
    );
    println!("  Accuracy:     {}/100", quality.category_scores.accuracy);
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Formatting:   {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    if !quality.findings.is_empty() {
        println!("\nDocItem Gaps:");
        for finding in &quality.findings {
            println!("  - {}", finding.description);
        }
    }

    // MOBI is structured ebook format, use 95% threshold
    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}

#[tokio::test]
async fn test_llm_docitem_gpx() {
    let verifier = create_verifier();

    // Parse GPX to DocItems
    let backend = GpxBackend::new();
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/gps/gpx/hiking_trail.gpx"
            ),
            &Default::default(),
        )
        .expect("Failed to parse GPX");

    // Export to JSON (the complete representation)
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", result.content_blocks.is_some());

    // Read original GPX for comparison
    let gpx_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/gps/gpx/hiking_trail.gpx"
    ));
    let gpx_content = std::fs::read_to_string(gpx_path).expect("Failed to read GPX file");

    // LLM validates: Does JSON contain ALL information from GPX?
    let prompt = format!(
        r#"Analyze if this DocItem JSON contains complete information from the original GPX (GPS Exchange Format) file.

ORIGINAL GPX:
```xml
{}
```
(Analyze the GPX XML structure - check all tracks, waypoints, routes, metadata)

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness for GPX:

1. **Completeness (0-100)**: All tracks, waypoints, routes extracted?
2. **Accuracy (0-100)**: GPS coordinates and data correct?
3. **Structure (0-100)**: Track/route/waypoint hierarchy preserved?
4. **Formatting (0-100)**: GPS data clearly formatted?
5. **Metadata (0-100)**: Track metadata (name, time, elevation) captured?

For GPX files:
- Completeness: All <trk>, <wpt>, <rte> elements extracted
- Accuracy: Coordinates (lat/lon), elevation, time correct
- Structure: Track segments and waypoint organization preserved
- Formatting: GPS data readable
- Metadata: Track names, descriptions, timestamps captured

Respond with JSON:
{{
  "scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        gpx_content,
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: GPX ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Completeness: {}/100",
        quality.category_scores.completeness
    );
    println!("  Accuracy:     {}/100", quality.category_scores.accuracy);
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Formatting:   {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    if !quality.findings.is_empty() {
        println!("\nDocItem Gaps:");
        for finding in &quality.findings {
            println!("  - {}", finding.description);
        }
    }

    // GPX is structured GPS data format, use 95% threshold
    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}

#[tokio::test]
async fn test_llm_docitem_kml() {
    let verifier = create_verifier();

    // Parse KML to DocItems
    let backend = KmlBackend::new(InputFormat::Kml);
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/gps/kml/hiking_path.kml"
            ),
            &Default::default(),
        )
        .expect("Failed to parse KML");

    // Export to JSON (the complete representation)
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", result.content_blocks.is_some());

    // Read original KML for comparison
    let kml_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/gps/kml/hiking_path.kml"
    ));
    let kml_content = std::fs::read_to_string(kml_path).expect("Failed to read KML file");

    // LLM validates: Does JSON contain ALL information from KML?
    let prompt = format!(
        r#"Analyze if this DocItem JSON contains complete information from the original KML (Keyhole Markup Language) file.

ORIGINAL KML:
```xml
{}
```
(Analyze the KML XML structure - check all placemarks, paths, polygons, metadata)

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness for KML:

1. **Completeness (0-100)**: All placemarks, paths, polygons extracted?
2. **Accuracy (0-100)**: Geographic coordinates and data correct?
3. **Structure (0-100)**: Folder/placemark hierarchy preserved?
4. **Formatting (0-100)**: Geographic data clearly formatted?
5. **Metadata (0-100)**: Placemark metadata (name, description, style) captured?

For KML files:
- Completeness: All <Placemark>, <LineString>, <Polygon> elements extracted
- Accuracy: Coordinates, names, descriptions correct
- Structure: Folder hierarchy and organization preserved
- Formatting: Geographic data readable
- Metadata: Placemark names, descriptions, style info captured

Respond with JSON:
{{
  "scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        kml_content,
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: KML ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Completeness: {}/100",
        quality.category_scores.completeness
    );
    println!("  Accuracy:     {}/100", quality.category_scores.accuracy);
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Formatting:   {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    if !quality.findings.is_empty() {
        println!("\nDocItem Gaps:");
        for finding in &quality.findings {
            println!("  - {}", finding.description);
        }
    }

    // KML is structured geographic data format, use 95% threshold
    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}

#[tokio::test]
async fn test_llm_docitem_tex() {
    let verifier = create_verifier();

    // Parse TEX to DocItems
    let mut backend = LatexBackend::new().expect("Failed to create LaTeX backend");
    let tex_file_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/latex/simple_document.tex"
    ));
    let result = backend.parse(tex_file_path).expect("Failed to parse TEX");

    // Export to JSON (the complete representation)
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", result.content_blocks.is_some());

    // Read original TEX content for comparison
    let tex_content = std::fs::read_to_string(tex_file_path).expect("Failed to read TEX file");

    // LLM validates: Does JSON contain ALL information from TEX?
    let prompt = format!(
        r#"Analyze if this DocItem JSON contains complete information from the original TEX (LaTeX) file.

ORIGINAL TEX:
```latex
{}
```
(Analyze the LaTeX structure - check all sections, text, equations, environments)

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness for TEX (LaTeX):

1. **Completeness (0-100)**: All sections, text, equations extracted?
2. **Accuracy (0-100)**: Text content and math correct?
3. **Structure (0-100)**: Document hierarchy (sections, subsections) preserved?
4. **Formatting (0-100)**: LaTeX formatting (bold, italic, lists) captured?
5. **Metadata (0-100)**: Document metadata (title, author, date) captured?

For LaTeX files:
- Completeness: All \section, \subsection, paragraphs, equations extracted
- Accuracy: Text and math commands correct
- Structure: Document hierarchy preserved
- Formatting: \textbf, \textit, itemize, enumerate captured
- Metadata: \title, \author, \date extracted

Respond with JSON:
{{
  "scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        tex_content,
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: TEX ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Completeness: {}/100",
        quality.category_scores.completeness
    );
    println!("  Accuracy:     {}/100", quality.category_scores.accuracy);
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Formatting:   {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    if !quality.findings.is_empty() {
        println!("\nDocItem Gaps:");
        for finding in &quality.findings {
            println!("  - {}", finding.description);
        }
    }

    // TEX is structured document format, use 95% threshold
    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}

#[tokio::test]
async fn test_llm_docitem_kmz() {
    let verifier = create_verifier();

    // Parse KMZ to DocItems
    let backend = KmlBackend::new(InputFormat::Kmz);
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/gps/kml/simple_landmark.kmz"
            ),
            &Default::default(),
        )
        .expect("Failed to parse KMZ");

    // Export to JSON (the complete representation)
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", result.content_blocks.is_some());

    // Read original KMZ for comparison
    let kmz_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/gps/kml/simple_landmark.kmz"
    ));

    // LLM validates: Does JSON contain ALL information from KMZ?
    let prompt = format!(
        r#"Analyze if this DocItem JSON contains complete information from the original KMZ (compressed KML) file.

ORIGINAL KMZ: {}
(Analyze the KMZ/KML structure - check all placemarks, paths, polygons, metadata)

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness for KMZ (compressed KML):

1. **Completeness (0-100)**: All placemarks, paths, polygons extracted?
2. **Accuracy (0-100)**: Geographic coordinates and data correct?
3. **Structure (0-100)**: Folder/placemark hierarchy preserved?
4. **Formatting (0-100)**: Geographic data clearly formatted?
5. **Metadata (0-100)**: Placemark metadata (name, description, style) captured?

For KMZ files:
- Completeness: All <Placemark>, <LineString>, <Polygon> elements extracted
- Accuracy: Coordinates, names, descriptions correct
- Structure: Folder hierarchy and organization preserved
- Formatting: Geographic data readable
- Metadata: Placemark names, descriptions, style info captured

Respond with JSON:
{{
  "scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        kmz_path.display(),
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: KMZ ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Completeness: {}/100",
        quality.category_scores.completeness
    );
    println!("  Accuracy:     {}/100", quality.category_scores.accuracy);
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Formatting:   {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    if !quality.findings.is_empty() {
        println!("\nDocItem Gaps:");
        for finding in &quality.findings {
            println!("  - {}", finding.description);
        }
    }

    // KMZ is structured geographic data format (compressed KML), use 95% threshold
    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}

// =======================================================================================
// NEW TESTS (N=1352): DOC, VSDX, MPP, PAGES
// =======================================================================================

#[tokio::test]
async fn test_llm_docitem_doc() {
    let verifier = create_verifier();

    // Parse DOC to DocItems (via DOCX conversion)
    let doc_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/legacy/doc/simple_text.doc"
    );

    // Convert DOC to DOCX first
    let docx_path = DocBackend::convert_doc_to_docx(Path::new(doc_path))
        .expect("Failed to convert DOC to DOCX");

    // Parse DOCX to DocItems
    let backend = DocxBackend;
    let result = backend
        .parse_file(&docx_path, &Default::default())
        .expect("Failed to parse converted DOCX");

    // Clean up temp file
    let _ = std::fs::remove_file(&docx_path);

    // Export to JSON (the complete representation)
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", result.content_blocks.is_some());

    // Read original DOC for comparison
    let doc_path_obj = Path::new(doc_path);

    // LLM validates: Does JSON contain ALL information from DOC?
    let prompt = format!(
        r#"Analyze if this DocItem JSON contains complete information from the original DOC (MS Word legacy) file.

ORIGINAL DOCUMENT: {}
(Open and analyze the DOC structure - check paragraphs, formatting, metadata)

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness for DOC (legacy Word):

1. **Completeness (0-100)**: All paragraphs, headings, lists extracted?
2. **Accuracy (0-100)**: Content semantically correct?
3. **Structure (0-100)**: Document hierarchy preserved?
4. **Formatting (0-100)**: Paragraphs, lists formatted correctly in DocItems?
5. **Metadata (0-100)**: Document properties captured?

For DOC documents:
- Completeness: All text content present
- Accuracy: Content correct after DOC→DOCX conversion
- Structure: Document structure preserved
- Formatting: Formatting properly captured
- Metadata: Document properties captured

For each category with score < 95:
- List specific issues
- Assign severity: "critical", "major", "minor", or "info"
- Provide location if identifiable

Calculate overall_score as weighted average:
- completeness: 30%
- accuracy: 30%
- structure: 20%
- formatting: 15%
- metadata: 5%

Return JSON ONLY (no markdown, no explanation):
{{
  "overall_score": 0.0-1.0,
  "category_scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        doc_path_obj.display(),
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: DOC ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Completeness: {}/100",
        quality.category_scores.completeness
    );
    println!("  Accuracy:     {}/100", quality.category_scores.accuracy);
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Formatting:   {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    if !quality.findings.is_empty() {
        println!("\nDocItem Gaps:");
        for finding in &quality.findings {
            println!("  - {}", finding.description);
        }
    }

    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}

#[tokio::test]
async fn test_llm_docitem_vsdx() {
    let verifier = create_verifier();

    // Parse VSDX to DocItems
    let backend = VisioBackend::new();
    let result = backend
        .parse(Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../test-corpus/microsoft-visio/shapes_and_lines.vsdx"
        )))
        .expect("Failed to parse VSDX");

    // Export to JSON (the complete representation)
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", result.content_blocks.is_some());

    // Read original VSDX for comparison
    let vsdx_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/microsoft-visio/shapes_and_lines.vsdx"
    ));

    // LLM validates: Does JSON contain ALL information from VSDX?
    let prompt = format!(
        r#"Analyze if this DocItem JSON contains complete information from the original VSDX (Visio diagram) file.

ORIGINAL DOCUMENT: {}
(Open and analyze the VSDX structure - check pages, shapes, text, connectors)

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness for VSDX (Visio):

1. **Completeness (0-100)**: All pages, shapes, text extracted?
2. **Accuracy (0-100)**: Content semantically correct?
3. **Structure (0-100)**: Diagram hierarchy (pages, layers) preserved?
4. **Formatting (0-100)**: Shapes, connectors formatted correctly in DocItems?
5. **Metadata (0-100)**: Document properties (title, author, pages) captured?

For VSDX diagrams:
- Completeness: All pages, shapes, text content present
- Accuracy: Shape text and diagram content correct
- Structure: Page and layer hierarchy preserved
- Formatting: Shapes and connectors properly structured
- Metadata: Document properties captured

For each category with score < 95:
- List specific issues
- Assign severity: "critical", "major", "minor", or "info"
- Provide location if identifiable

Calculate overall_score as weighted average:
- completeness: 30%
- accuracy: 30%
- structure: 20%
- formatting: 15%
- metadata: 5%

Return JSON ONLY (no markdown, no explanation):
{{
  "overall_score": 0.0-1.0,
  "category_scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        vsdx_path.display(),
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: VSDX ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Completeness: {}/100",
        quality.category_scores.completeness
    );
    println!("  Accuracy:     {}/100", quality.category_scores.accuracy);
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Formatting:   {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    if !quality.findings.is_empty() {
        println!("\nDocItem Gaps:");
        for finding in &quality.findings {
            println!("  - {}", finding.description);
        }
    }

    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}

#[tokio::test]
async fn test_llm_docitem_mpp() {
    let verifier = create_verifier();

    // Parse MPP to DocItems
    // Note: Using sample2_2010.mpp instead of sample4_2003.mpp due to format compatibility
    let backend = ProjectBackend::new();
    let result = backend
        .parse_to_docitems(Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../test-corpus/microsoft-project/sample2_2010.mpp"
        )))
        .expect("Failed to parse MPP");

    // Export to JSON (the complete representation)
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", !result.body.children.is_empty());

    // Read original MPP for comparison
    let mpp_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/microsoft-project/sample2_2010.mpp"
    ));

    // LLM validates: Does JSON contain ALL information from MPP?
    let prompt = format!(
        r#"Analyze if this DocItem JSON contains complete information from the original MPP (MS Project) file.

ORIGINAL DOCUMENT: {}
(Open and analyze the MPP structure - check tasks, resources, timeline)

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness for MPP (MS Project):

1. **Completeness (0-100)**: All tasks, resources, milestones extracted?
2. **Accuracy (0-100)**: Task details (names, dates, durations) correct?
3. **Structure (0-100)**: Task hierarchy (parent/child) preserved?
4. **Formatting (0-100)**: Project data formatted correctly in DocItems?
5. **Metadata (0-100)**: Project properties (title, start date, resources) captured?

For MPP project files:
- Completeness: All tasks, resources, assignments present
- Accuracy: Task names, dates, durations correct
- Structure: Task hierarchy and dependencies preserved
- Formatting: Project data properly structured
- Metadata: Project properties and calendar captured

For each category with score < 95:
- List specific issues
- Assign severity: "critical", "major", "minor", or "info"
- Provide location if identifiable

Calculate overall_score as weighted average:
- completeness: 30%
- accuracy: 30%
- structure: 20%
- formatting: 15%
- metadata: 5%

Return JSON ONLY (no markdown, no explanation):
{{
  "overall_score": 0.0-1.0,
  "category_scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        mpp_path.display(),
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: MPP ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Completeness: {}/100",
        quality.category_scores.completeness
    );
    println!("  Accuracy:     {}/100", quality.category_scores.accuracy);
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Formatting:   {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    if !quality.findings.is_empty() {
        println!("\nDocItem Gaps:");
        for finding in &quality.findings {
            println!("  - {}", finding.description);
        }
    }

    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}

#[tokio::test]
async fn test_llm_docitem_pages() {
    let verifier = create_verifier();

    // Parse PAGES to DocItems
    let backend = PagesBackend::new();
    let result = backend
        .parse(Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../test-corpus/apple-pages/minimal-test.pages"
        )))
        .expect("Failed to parse PAGES");

    // Export to JSON (the complete representation)
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", !result.body.children.is_empty());

    // Read original PAGES for comparison
    let pages_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/apple-pages/minimal-test.pages"
    ));

    // LLM validates: Does JSON contain ALL information from PAGES?
    let prompt = format!(
        r#"Analyze if this DocItem JSON contains complete information from the original PAGES (Apple Pages) file.

ORIGINAL DOCUMENT: {}
(Open and analyze the PAGES structure - check paragraphs, formatting, metadata)

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness for PAGES (Apple Pages):

1. **Completeness (0-100)**: All paragraphs, headings, lists extracted?
2. **Accuracy (0-100)**: Content semantically correct?
3. **Structure (0-100)**: Document hierarchy preserved?
4. **Formatting (0-100)**: Paragraphs, lists formatted correctly in DocItems?
5. **Metadata (0-100)**: Document properties captured?

For PAGES documents:
- Completeness: All text content present
- Accuracy: Content correct
- Structure: Document structure preserved
- Formatting: Formatting properly captured
- Metadata: Document properties captured

For each category with score < 95:
- List specific issues
- Assign severity: "critical", "major", "minor", or "info"
- Provide location if identifiable

Calculate overall_score as weighted average:
- completeness: 30%
- accuracy: 30%
- structure: 20%
- formatting: 15%
- metadata: 5%

Return JSON ONLY (no markdown, no explanation):
{{
  "overall_score": 0.0-1.0,
  "category_scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        pages_path.display(),
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: PAGES ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Completeness: {}/100",
        quality.category_scores.completeness
    );
    println!("  Accuracy:     {}/100", quality.category_scores.accuracy);
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Formatting:   {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    if !quality.findings.is_empty() {
        println!("\nDocItem Gaps:");
        for finding in &quality.findings {
            println!("  - {}", finding.description);
        }
    }

    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}

#[tokio::test]
async fn test_llm_docitem_srt() {
    let verifier = create_verifier();

    // Parse SRT to DocItems
    let backend = SrtBackend::new().expect("Failed to create SRT backend");
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/subtitles/srt/simple_dialogue.srt"
            ),
            &Default::default(),
        )
        .expect("Failed to parse SRT");

    // Export to JSON (the complete representation)
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", result.content_blocks.is_some());

    // Read original SRT for comparison
    let srt_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/subtitles/srt/simple_dialogue.srt"
    ));
    let srt_content = std::fs::read_to_string(srt_path).expect("Failed to read SRT file");

    // LLM validates: Does JSON contain ALL information from SRT?
    let prompt = format!(
        r#"Analyze if this DocItem JSON contains complete information from the original SRT (SubRip subtitle) file.

ORIGINAL DOCUMENT:
```srt
{}
```
(Open and analyze the SRT structure - check subtitle entries, timestamps, text)

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness for SRT subtitle files:

1. **Completeness (0-100)**: All subtitle entries extracted?
2. **Accuracy (0-100)**: Subtitle text and timestamps correct?
3. **Structure (0-100)**: Subtitle sequence preserved?
4. **Formatting (0-100)**: Subtitle data formatted correctly in DocItems?
5. **Metadata (0-100)**: File properties captured?

For SRT subtitle files:
- Completeness: All subtitle entries present (check count)
- Accuracy: Text content matches exactly, timestamps correct
- Structure: Sequential order preserved
- Formatting: Subtitle text properly structured
- Metadata: File information captured

For each category with score < 95:
- List specific issues
- Assign severity: "critical", "major", "minor", or "info"
- Provide location if identifiable

Calculate overall_score as weighted average:
- completeness: 30%
- accuracy: 30%
- structure: 20%
- formatting: 15%
- metadata: 5%

Return JSON ONLY (no markdown, no explanation):
{{
  "overall_score": 0.0-1.0,
  "category_scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        srt_content,
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: SRT ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Completeness: {}/100",
        quality.category_scores.completeness
    );
    println!("  Accuracy:     {}/100", quality.category_scores.accuracy);
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Formatting:   {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    if !quality.findings.is_empty() {
        println!("\nDocItem Gaps:");
        for finding in &quality.findings {
            println!("  - {}", finding.description);
        }
    }

    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}

#[tokio::test]
async fn test_llm_docitem_ipynb() {
    let verifier = create_verifier();

    // Parse IPYNB to DocItems
    let backend = IpynbBackend::new();
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/notebook/ipynb/simple_data_analysis.ipynb"
            ),
            &Default::default(),
        )
        .expect("Failed to parse IPYNB");

    // Export to JSON (the complete representation)
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", result.content_blocks.is_some());

    // Read original IPYNB for comparison
    let ipynb_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/notebook/ipynb/simple_data_analysis.ipynb"
    ));
    let ipynb_content = std::fs::read_to_string(ipynb_path).expect("Failed to read IPYNB file");

    // LLM validates: Does JSON contain ALL information from IPYNB?
    let prompt = format!(
        r#"Analyze if this DocItem JSON contains complete information from the original IPYNB (Jupyter Notebook) file.

ORIGINAL DOCUMENT:
```json
{}
```
(Open and analyze the IPYNB structure - check code cells, markdown cells, outputs)

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness for Jupyter Notebooks:

1. **Completeness (0-100)**: All cells (code + markdown) extracted?
2. **Accuracy (0-100)**: Cell content correct? Outputs captured?
3. **Structure (0-100)**: Cell order preserved? Cell types correct?
4. **Formatting (0-100)**: Notebook data formatted correctly in DocItems?
5. **Metadata (0-100)**: Kernel info, execution counts captured?

For Jupyter Notebooks:
- Completeness: All code cells and markdown cells present
- Accuracy: Cell source code and outputs match
- Structure: Cell sequence and types preserved
- Formatting: Code, markdown, and outputs properly structured
- Metadata: Kernel, language, execution counts captured

For each category with score < 95:
- List specific issues
- Assign severity: "critical", "major", "minor", or "info"
- Provide location if identifiable

Calculate overall_score as weighted average:
- completeness: 30%
- accuracy: 30%
- structure: 20%
- formatting: 15%
- metadata: 5%

Return JSON ONLY (no markdown, no explanation):
{{
  "overall_score": 0.0-1.0,
  "category_scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        ipynb_content,
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: IPYNB ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Completeness: {}/100",
        quality.category_scores.completeness
    );
    println!("  Accuracy:     {}/100", quality.category_scores.accuracy);
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Formatting:   {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    if !quality.findings.is_empty() {
        println!("\nDocItem Gaps:");
        for finding in &quality.findings {
            println!("  - {}", finding.description);
        }
    }

    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}

#[tokio::test]
async fn test_llm_docitem_stl() {
    let verifier = create_verifier();

    // Parse STL to DocItems
    let backend = CadBackend::new(InputFormat::Stl).expect("Failed to create STL backend");
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/cad/stl/simple_cube.stl"
            ),
            &Default::default(),
        )
        .expect("Failed to parse STL");

    // Export to JSON (the complete representation)
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", result.content_blocks.is_some());

    // Read original STL for comparison
    let stl_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/cad/stl/simple_cube.stl"
    ));

    // LLM validates: Does JSON contain ALL information from STL?
    let prompt = format!(
        r#"Analyze if this DocItem JSON contains complete information from the original STL (STereoLithography) 3D model file.

ORIGINAL DOCUMENT: {}
(Open and analyze the STL structure - check triangles, vertices, geometry)

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness for STL 3D models:

1. **Completeness (0-100)**: All mesh data (triangles, vertices) extracted?
2. **Accuracy (0-100)**: Geometry statistics correct (counts, dimensions)?
3. **Structure (0-100)**: 3D model metadata preserved?
4. **Formatting (0-100)**: Mesh data formatted correctly in DocItems?
5. **Metadata (0-100)**: File properties (format, bounding box) captured?

For STL 3D models:
- Completeness: Triangle count, vertex count present
- Accuracy: Geometry statistics match (counts, bounding box)
- Structure: 3D model info properly organized
- Formatting: Mesh data properly structured
- Metadata: File format (ASCII/binary), dimensions captured

For each category with score < 95:
- List specific issues
- Assign severity: "critical", "major", "minor", or "info"
- Provide location if identifiable

Calculate overall_score as weighted average:
- completeness: 30%
- accuracy: 30%
- structure: 20%
- formatting: 15%
- metadata: 5%

Return JSON ONLY (no markdown, no explanation):
{{
  "overall_score": 0.0-1.0,
  "category_scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        stl_path.display(),
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: STL ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Completeness: {}/100",
        quality.category_scores.completeness
    );
    println!("  Accuracy:     {}/100", quality.category_scores.accuracy);
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Formatting:   {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    if !quality.findings.is_empty() {
        println!("\nDocItem Gaps:");
        for finding in &quality.findings {
            println!("  - {}", finding.description);
        }
    }

    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}

#[tokio::test]
async fn test_llm_docitem_obj() {
    let verifier = create_verifier();

    // Parse OBJ to DocItems
    let backend = CadBackend::new(InputFormat::Obj).expect("Failed to create OBJ backend");
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/cad/obj/textured_quad.obj"
            ),
            &Default::default(),
        )
        .expect("Failed to parse OBJ");

    // Export to JSON (the complete representation)
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", result.content_blocks.is_some());

    // Read original OBJ for comparison
    let obj_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/cad/obj/textured_quad.obj"
    ));
    let obj_content = std::fs::read_to_string(obj_path).expect("Failed to read OBJ file");

    // LLM validates: Does JSON contain ALL information from OBJ?
    let prompt = format!(
        r#"Analyze if this DocItem JSON contains complete information from the original OBJ (Wavefront Object) 3D model file.

ORIGINAL DOCUMENT:
```obj
{}
```
(Open and analyze the OBJ structure - check vertices, faces, normals, textures)

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness for OBJ 3D models:

1. **Completeness (0-100)**: All mesh data (vertices, faces, normals) extracted?
2. **Accuracy (0-100)**: Geometry statistics correct (counts, dimensions)?
3. **Structure (0-100)**: 3D model metadata preserved?
4. **Formatting (0-100)**: Mesh data formatted correctly in DocItems?
5. **Metadata (0-100)**: File properties (groups, materials) captured?

For OBJ 3D models:
- Completeness: Vertex count, face count, normal count present
- Accuracy: Geometry statistics match (counts, bounding box)
- Structure: 3D model info properly organized
- Formatting: Mesh data properly structured
- Metadata: Groups, materials, texture info captured

For each category with score < 95:
- List specific issues
- Assign severity: "critical", "major", "minor", or "info"
- Provide location if identifiable

Calculate overall_score as weighted average:
- completeness: 30%
- accuracy: 30%
- structure: 20%
- formatting: 15%
- metadata: 5%

Return JSON ONLY (no markdown, no explanation):
{{
  "overall_score": 0.0-1.0,
  "category_scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        obj_content,
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: OBJ ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Completeness: {}/100",
        quality.category_scores.completeness
    );
    println!("  Accuracy:     {}/100", quality.category_scores.accuracy);
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Formatting:   {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    if !quality.findings.is_empty() {
        println!("\nDocItem Gaps:");
        for finding in &quality.findings {
            println!("  - {}", finding.description);
        }
    }

    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}

#[tokio::test]
async fn test_llm_docitem_dxf() {
    let verifier = create_verifier();

    // Parse DXF to DocItems
    let backend = CadBackend::new(InputFormat::Dxf).expect("Failed to create DXF backend");
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/cad/dxf/simple_drawing.dxf"
            ),
            &Default::default(),
        )
        .expect("Failed to parse DXF");

    // Export to JSON (the complete representation)
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", result.content_blocks.is_some());

    // Read original DXF for comparison
    let dxf_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/cad/dxf/simple_drawing.dxf"
    ));
    let dxf_content = std::fs::read_to_string(dxf_path).expect("Failed to read DXF file");

    // LLM validates: Does JSON contain ALL information from DXF?
    let prompt = format!(
        r#"Analyze if this DocItem JSON contains complete information from the original DXF (Drawing Exchange Format) file.

ORIGINAL DOCUMENT:
```dxf
{}
```
(Open and analyze the DXF structure - check entities, layers, geometry)

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness for DXF CAD drawings:

1. **Completeness (0-100)**: All entities (lines, circles, arcs, etc.) extracted?
2. **Accuracy (0-100)**: Entity properties and geometry correct?
3. **Structure (0-100)**: Layers and hierarchy preserved?
4. **Formatting (0-100)**: Drawing data formatted correctly in DocItems?
5. **Metadata (0-100)**: File properties (version, units, bounds) captured?

For DXF CAD drawings:
- Completeness: All entity types and counts present
- Accuracy: Entity properties (coordinates, dimensions) match
- Structure: Layer organization preserved
- Formatting: Drawing data properly structured
- Metadata: DXF version, units, bounding box captured

For each category with score < 95:
- List specific issues
- Assign severity: "critical", "major", "minor", or "info"
- Provide location if identifiable

Calculate overall_score as weighted average:
- completeness: 30%
- accuracy: 30%
- structure: 20%
- formatting: 15%
- metadata: 5%

Return JSON ONLY (no markdown, no explanation):
{{
  "overall_score": 0.0-1.0,
  "category_scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        dxf_content,
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: DXF ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Completeness: {}/100",
        quality.category_scores.completeness
    );
    println!("  Accuracy:     {}/100", quality.category_scores.accuracy);
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Formatting:   {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    if !quality.findings.is_empty() {
        println!("\nDocItem Gaps:");
        for finding in &quality.findings {
            println!("  - {}", finding.description);
        }
    }

    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}

#[tokio::test]
async fn test_llm_docitem_gltf() {
    let verifier = create_verifier();

    // Parse GLTF to DocItems
    let backend = CadBackend::new(InputFormat::Gltf).expect("Failed to create GLTF backend");
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/cad/gltf/simple_triangle.gltf"
            ),
            &Default::default(),
        )
        .expect("Failed to parse GLTF");

    // Export to JSON
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");

    println!("DocItem JSON length: {} chars", json.len());
    println!("Has content_blocks: {}", result.content_blocks.is_some());

    let gltf_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/cad/gltf/simple_triangle.gltf"
    ));
    let gltf_content = std::fs::read_to_string(gltf_path).expect("Failed to read GLTF file");

    let prompt = format!(
        r#"Analyze if this DocItem JSON contains complete information from the original GLTF (GL Transmission Format) file.

ORIGINAL DOCUMENT:
```json
{}
```

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness for GLTF 3D models:

1. **Completeness (0-100)**: All mesh data (nodes, meshes, materials) extracted?
2. **Accuracy (0-100)**: Scene structure and geometry correct?
3. **Structure (0-100)**: Node hierarchy and metadata preserved?
4. **Formatting (0-100)**: 3D data formatted correctly in DocItems?
5. **Metadata (0-100)**: File properties (version, extensions) captured?

Calculate overall_score as weighted average:
- completeness: 30%
- accuracy: 30%
- structure: 20%
- formatting: 15%
- metadata: 5%

Return JSON ONLY (no markdown, no explanation):
{{
  "overall_score": 0.0-1.0,
  "category_scores": {{
    "completeness": 0-100,
    "accuracy": 0-100,
    "structure": 0-100,
    "formatting": 0-100,
    "metadata": 0-100
  }},
  "findings": [
    {{
      "category": "completeness|accuracy|structure|formatting|metadata",
      "severity": "critical|major|minor|info",
      "description": "Brief description of issue",
      "location": "Optional location reference"
    }}
  ],
  "reasoning": "Optional: Brief explanation of scores"
}}
"#,
        gltf_content,
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: GLTF ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);

    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}

#[tokio::test]
async fn test_llm_docitem_glb() {
    let verifier = create_verifier();

    // Parse GLB to DocItems
    let backend = CadBackend::new(InputFormat::Glb).expect("Failed to create GLB backend");
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/cad/gltf/box.glb"
            ),
            &Default::default(),
        )
        .expect("Failed to parse GLB");

    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");
    let glb_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/cad/gltf/box.glb"
    ));

    let prompt = format!(
        r"Analyze if this DocItem JSON contains complete information from the original GLB (GL Transmission Format Binary) file.

ORIGINAL DOCUMENT: {}

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction completeness for GLB 3D models (binary GLTF).

Return JSON ONLY with overall_score (0.0-1.0) and category_scores.
",
        glb_path.display(),
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: GLB ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);

    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}

#[tokio::test]
async fn test_llm_docitem_heif() {
    let verifier = create_verifier();

    // Parse HEIF to DocItems
    let backend = HeifBackend::new(InputFormat::Heif);
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/graphics/heif/photo_sample.heic"
            ),
            &Default::default(),
        )
        .expect("Failed to parse HEIF");

    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");
    let heif_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/graphics/heif/photo_sample.heic"
    ));

    let prompt = format!(
        r"Analyze if this DocItem JSON contains complete information from the original HEIF/HEIC image file.

ORIGINAL DOCUMENT: {}

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction for HEIF images (dimensions, format, metadata).

Return JSON ONLY with overall_score (0.0-1.0) and category_scores.
",
        heif_path.display(),
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: HEIF ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);

    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}

#[tokio::test]
async fn test_llm_docitem_avif() {
    let verifier = create_verifier();

    // Parse AVIF to DocItems
    let backend = AvifBackend::new();
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/graphics/avif/animation_frame.avif"
            ),
            &Default::default(),
        )
        .expect("Failed to parse AVIF");

    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");
    let avif_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/graphics/avif/animation_frame.avif"
    ));

    let prompt = format!(
        r"Analyze if this DocItem JSON contains complete information from the original AVIF image file.

ORIGINAL DOCUMENT: {}

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction for AVIF images (dimensions, format, metadata).

Return JSON ONLY with overall_score (0.0-1.0) and category_scores.
",
        avif_path.display(),
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: AVIF ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);

    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}

#[tokio::test]
async fn test_llm_docitem_dicom() {
    let verifier = create_verifier();

    // Parse DICOM to DocItems
    let backend = DicomBackend::new();
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/medical/xray_hand.dcm"
            ),
            &Default::default(),
        )
        .expect("Failed to parse DICOM");

    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");
    let dicom_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/medical/xray_hand.dcm"
    ));

    let prompt = format!(
        r"Analyze if this DocItem JSON contains complete information from the original DICOM medical image file.

ORIGINAL DOCUMENT: {}

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction for DICOM (patient info, study details, image metadata).

Return JSON ONLY with overall_score (0.0-1.0) and category_scores.
",
        dicom_path.display(),
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: DICOM ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);

    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}

#[tokio::test]
async fn test_llm_docitem_idml() {
    let verifier = create_verifier();

    // Parse IDML to DocItems
    let backend = IdmlBackend;
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/adobe/idml/simple_document.idml"
            ),
            &Default::default(),
        )
        .expect("Failed to parse IDML");

    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");
    let idml_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/adobe/idml/simple_document.idml"
    ));

    // Extract Story XML from IDML ZIP (IDML is a ZIP containing XML files)
    let file = std::fs::File::open(idml_path).expect("Failed to open IDML file");
    let mut archive = zip::ZipArchive::new(file).expect("Failed to read IDML as ZIP");
    let mut story_content = String::new();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).expect("Failed to read ZIP entry");
        if file.name().starts_with("Stories/") && file.name().ends_with(".xml") {
            std::io::Read::read_to_string(&mut file, &mut story_content)
                .expect("Failed to read Story XML");
            break;
        }
    }
    let idml_content = if story_content.is_empty() {
        // Fallback: show designmap.xml if no Story found
        let mut file = archive
            .by_name("designmap.xml")
            .expect("No Story or designmap found");
        let mut content = String::new();
        std::io::Read::read_to_string(&mut file, &mut content).expect("Failed to read designmap");
        content
    } else {
        story_content
    };

    let prompt = format!(
        r"Analyze if this DocItem JSON contains complete information from the original IDML (InDesign Markup Language) file.

ORIGINAL DOCUMENT:
```xml
{}
```

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction for IDML (text content, layout structure, formatting).

Return JSON ONLY with overall_score (0.0-1.0) and category_scores.
",
        idml_content,
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: IDML ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);

    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}

#[tokio::test]
async fn test_llm_docitem_key() {
    let verifier = create_verifier();

    // Parse Keynote to DocItems
    let backend = KeynoteBackend::new();
    let result = backend
        .parse(Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../test-corpus/apple-keynote/transitions-and-builds.key"
        )))
        .expect("Failed to parse KEY");

    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");
    let key_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/apple-keynote/transitions-and-builds.key"
    ));

    let prompt = format!(
        r"Analyze if this DocItem JSON contains complete information from the original Apple Keynote (.key) presentation file.

ORIGINAL DOCUMENT: {}

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction for Keynote presentations (slides, text, images, layout, transitions, and build animations).

Important: This test file specifically includes slide transitions (dissolve, push, wipe, cube, flip) and build animations (fade-in, fly-in, appear, rotate, scale).
Check if these features are properly extracted in the DocItems JSON.

Return JSON ONLY with overall_score (0.0-1.0) and category_scores.
",
        key_path.display(),
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: KEY ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    println!("Category Scores:");
    println!(
        "  Completeness: {}/100",
        quality.category_scores.completeness
    );
    println!("  Accuracy:     {}/100", quality.category_scores.accuracy);
    println!("  Structure:    {}/100", quality.category_scores.structure);
    println!("  Formatting:   {}/100", quality.category_scores.formatting);
    println!("  Metadata:     {}/100", quality.category_scores.metadata);

    // KEY format expectation: 70-80% (Priority 2)
    // Improvements from N=1711 added transition/build extraction: 70% → 80%
    assert!(
        quality.score >= 0.70,
        "DocItem completeness: {:.1}% (expected 70-80% for KEY format)",
        quality.score * 100.0
    );
}

#[tokio::test]
async fn test_llm_docitem_numbers() {
    let verifier = create_verifier();

    // Parse Numbers to DocItems
    let backend = NumbersBackend::new();
    let result = backend
        .parse(Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../test-corpus/apple-numbers/minimal-test.numbers"
        )))
        .expect("Failed to parse NUMBERS");

    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");
    let numbers_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/apple-numbers/minimal-test.numbers"
    ));

    let prompt = format!(
        r"Analyze if this DocItem JSON contains complete information from the original Apple Numbers (.numbers) spreadsheet file.

ORIGINAL DOCUMENT: {}

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction for Numbers spreadsheets (tables, formulas, data, formatting).

Return JSON ONLY with overall_score (0.0-1.0) and category_scores.
",
        numbers_path.display(),
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: NUMBERS ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);

    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}

#[tokio::test]
async fn test_llm_docitem_xps() {
    let verifier = create_verifier();

    // Parse XPS to DocItems
    let backend = XpsBackend;
    let result = backend
        .parse_file(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../test-corpus/xps/simple_text.xps"
            ),
            &Default::default(),
        )
        .expect("Failed to parse XPS");

    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");
    let xps_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-corpus/xps/simple_text.xps"
    ));

    let prompt = format!(
        r"Analyze if this DocItem JSON contains complete information from the original XPS (XML Paper Specification) file.

ORIGINAL DOCUMENT: {}

PARSED DOCITEMS (JSON):
```json
{}
```

Evaluate DocItem extraction for XPS documents (text, layout, formatting, pages).

Return JSON ONLY with overall_score (0.0-1.0) and category_scores.
",
        xps_path.display(),
        truncate_json_for_llm(&json, 80000)
    );

    let quality = verifier
        .custom_verification(&prompt)
        .await
        .expect("LLM API failed");

    println!("\n=== DocItem Completeness: XPS ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);

    assert!(
        quality.score >= 0.95,
        "DocItem completeness: {:.1}% (need 95%)",
        quality.score * 100.0
    );
}
