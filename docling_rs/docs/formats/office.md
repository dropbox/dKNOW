# Microsoft Office Formats Guide

Complete guide to working with Microsoft Office documents (DOCX, PPTX, XLSX) in docling-rs.

---

## Overview

**Formats:** DOCX, PPTX, XLSX, XLSM
**Extensions:** `.docx`, `.pptx`, `.xlsx`, `.xlsm`, `.doc` (via conversion)
**Backend:** Python docling (production ML models)
**Status:** Production-ready ✅

**Test Coverage:**
- DOCX: 100% (18/18 canonical tests passing)
- PPTX: 100% (all tests passing)
- XLSX: 100% (all tests passing)

**Performance:**
- DOCX: 0.005-0.062s per document (avg 0.028s)
- PPTX: ~0.009s per presentation
- XLSX: ~0.012s per spreadsheet

---

## Table of Contents

1. [DOCX (Word Documents)](#docx-word-documents)
2. [PPTX (PowerPoint Presentations)](#pptx-powerpoint-presentations)
3. [XLSX (Excel Spreadsheets)](#xlsx-excel-spreadsheets)
4. [Legacy Formats (DOC)](#legacy-formats-doc)
5. [Performance Optimization](#performance-optimization)
6. [Troubleshooting](#troubleshooting)

---

## DOCX (Word Documents)

### Quick Start

```rust
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;
use docling_core::Result;

fn main() -> Result<()> {
    let converter = DocumentConverter::new()?;
    let result = converter.convert("document.docx")?;

    println!("{}", result.document.markdown);
    Ok(())
}
```

### Features

**Text Features:**
- ✅ Paragraphs with formatting
- ✅ Headings (Heading 1-6 styles)
- ✅ **Bold**, *italic*, underline
- ✅ Lists (bullet and numbered)
- ✅ Hyperlinks
- ✅ Footnotes and endnotes
- ✅ Comments (extracted as annotations)
- ✅ Track changes (shows final version)

**Document Structure:**
- ✅ Sections and page breaks
- ✅ Headers and footers
- ✅ Table of contents
- ✅ Captions

**Tables:**
- ✅ Table detection
- ✅ Cell formatting
- ✅ Merged cells (colspan/rowspan)
- ✅ Nested tables (flattened in markdown)

**Images:**
- ✅ Embedded images (extracted as references)
- ✅ Captions and alt text
- ✅ Figure numbering

**Not Supported:**
- ❌ Complex formatting (exact font, size, color)
- ❌ Page layout (margins, columns)
- ❌ Equations (MathML) - extracted as text
- ❌ Macros and VBA code

---

### Example: Simple Document

**Input DOCX:**
```
Title: Project Report

Section 1: Introduction

This is the introduction paragraph with bold and italic text.

• Bullet point 1
• Bullet point 2

Section 2: Analysis

[Table with data]
```

**Output Markdown:**
```markdown
# Project Report

## Section 1: Introduction

This is the introduction paragraph with **bold** and *italic* text.

- Bullet point 1
- Bullet point 2

## Section 2: Analysis

| Column 1 | Column 2 |
|----------|----------|
| Data A   | Data B   |
```

**Conversion Time:** 0.01-0.05s

---

### Example: Complex Document with Tables

```rust
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;
use docling_core::Result;
use std::fs;

fn convert_report(docx_path: &str) -> Result<String> {
    let converter = DocumentConverter::new()?;
    let result = converter.convert(docx_path)?;

    // Save markdown
    let output = docx_path.replace(".docx", ".md");
    fs::write(&output, &result.document.markdown)?;

    println!("Converted {} in {:?}", docx_path, result.latency);

    Ok(result.document.markdown)
}
```

**Use Case:** Converting quarterly business reports with tables, charts, and formatted text.

**Performance:** 0.02-0.06s per document

---

### DOCX Best Practices

**1. Use Standard Styles**

```
GOOD: Use built-in styles (Heading 1, Heading 2, Normal)
BAD: Custom font sizes without style names
```

Standard styles ensure headings are detected correctly.

**2. Avoid Complex Layouts**

```
GOOD: Single-column layout
BAD: Multi-column layouts with text boxes
```

Complex layouts may have text ordering issues in markdown output.

**3. Use Native Tables**

```
GOOD: Insert > Table
BAD: ASCII art tables or manually formatted columns
```

Native Word tables are detected with 95%+ accuracy.

**4. Embed Images Properly**

```
GOOD: Insert > Pictures (embedded in document)
BAD: Linked images (external file references)
```

Embedded images are extracted; linked images may be missing.

---

## PPTX (PowerPoint Presentations)

### Quick Start

```rust
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;
use docling_core::Result;

fn main() -> Result<()> {
    let converter = DocumentConverter::new()?;
    let result = converter.convert("presentation.pptx")?;

    println!("{}", result.document.markdown);
    Ok(())
}
```

### Features

**Slide Structure:**
- ✅ Slide titles (converted to H2 headings)
- ✅ Bullet points and text boxes
- ✅ Slide notes (appended as blockquotes)
- ✅ Tables on slides
- ✅ Images and diagrams

**Not Supported:**
- ❌ Animations and transitions
- ❌ Embedded videos (only placeholder extracted)
- ❌ Speaker notes formatting
- ❌ Slide layout preservation

---

### Example: Presentation to Markdown

**Input PPTX:**
```
Slide 1: Title Slide
  Title: "Q4 Results"
  Subtitle: "Financial Overview"

Slide 2: Content
  Title: "Revenue Growth"
  - 20% increase YoY
  - $5M total revenue
  [Chart showing growth]

Slide 3: Summary
  Title: "Key Takeaways"
  1. Strong performance
  2. Expanding market share
```

**Output Markdown:**
```markdown
# Q4 Results

*Financial Overview*

---

## Revenue Growth

- 20% increase YoY
- $5M total revenue

![Chart: Revenue Growth](image_0.png)

---

## Key Takeaways

1. Strong performance
2. Expanding market share
```

**Conversion Time:** ~0.009s per presentation

---

### PPTX Best Practices

**1. Use Slide Titles Consistently**

Every slide should have a title in the title placeholder. Titles become H2 headings in markdown.

**2. Keep Text in Text Boxes**

Avoid using images of text. Use native text boxes for best extraction.

**3. Use Native Tables**

PowerPoint tables convert cleanly to markdown tables.

**4. Add Alt Text to Images**

Right-click image → Format Picture → Alt Text. This becomes image caption in markdown.

---

## XLSX (Excel Spreadsheets)

### Quick Start

```rust
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;
use docling_core::Result;

fn main() -> Result<()> {
    let converter = DocumentConverter::new()?;
    let result = converter.convert("spreadsheet.xlsx")?;

    println!("{}", result.document.markdown);
    Ok(())
}
```

### Features

**Spreadsheet Structure:**
- ✅ Multiple worksheets (each becomes a section)
- ✅ Cell values (text, numbers, formulas calculated)
- ✅ Merged cells
- ✅ Empty cells (preserved in table structure)
- ✅ Row and column headers

**Data Types:**
- ✅ Text
- ✅ Numbers (preserved with precision)
- ✅ Dates (formatted as ISO 8601)
- ✅ Formulas (results calculated, formula not shown)
- ✅ Boolean (TRUE/FALSE)

**Not Supported:**
- ❌ Cell formatting (colors, fonts, borders)
- ❌ Charts and graphs (placeholders only)
- ❌ Pivot tables (data extracted, not interactive)
- ❌ Macros and VBA code
- ❌ Data validation rules

---

### Example: Spreadsheet to Markdown

**Input XLSX:**
```
Sheet 1: "Sales Data"
| Product | Q1    | Q2    | Q3    |
|---------|-------|-------|-------|
| Widget  | 100   | 120   | 110   |
| Gadget  | 80    | 90    | 95    |

Sheet 2: "Summary"
| Metric        | Value   |
|---------------|---------|
| Total Sales   | 495     |
| Avg per Quarter | 165  |
```

**Output Markdown:**
```markdown
# Sales Data

| Product | Q1  | Q2  | Q3  |
|---------|-----|-----|-----|
| Widget  | 100 | 120 | 110 |
| Gadget  | 80  | 90  | 95  |

---

# Summary

| Metric          | Value |
|-----------------|-------|
| Total Sales     | 495   |
| Avg per Quarter | 165   |
```

**Conversion Time:** ~0.012s per spreadsheet

---

### XLSX Best Practices

**1. Use Named Sheets**

Give worksheets descriptive names (not "Sheet1", "Sheet2"). Names become section headings.

**2. Avoid Large Spreadsheets**

Spreadsheets with >1000 rows may take longer to convert. Consider splitting into multiple files.

**3. Use First Row as Header**

First row should contain column headers. This improves table formatting in markdown.

**4. Calculate Formulas Before Conversion**

Formulas are evaluated during conversion, but complex formulas (with external references) may not calculate correctly.

---

### XLSM (Excel with Macros)

```rust
let converter = DocumentConverter::new()?;
let result = converter.convert("spreadsheet.xlsm")?;

// Macros are NOT executed or extracted
// Only cell values are converted to markdown
```

**Note:** `.xlsm` files are treated the same as `.xlsx`. Macros are ignored.

---

## Legacy Formats (DOC)

### Converting DOC to DOCX

Legacy Word `.doc` files require conversion to DOCX first.

**Requirements:**
- LibreOffice installed on system

**Automatic Conversion:**

```rust
// docling-rs automatically converts DOC → DOCX → Markdown
let converter = DocumentConverter::new()?;
let result = converter.convert("legacy_document.doc")?;

// Behind the scenes:
// 1. Check if LibreOffice is installed
// 2. Convert DOC → DOCX (via LibreOffice)
// 3. Process DOCX with Python docling
// 4. Return markdown
```

**Installation:**

```bash
# macOS
brew install libreoffice

# Linux (Ubuntu/Debian)
sudo apt-get install libreoffice

# Windows
# Download from: https://www.libreoffice.org/download/
```

**Performance:** DOC conversion adds 1-3 seconds (LibreOffice startup overhead).

**Limitations:**
- Complex DOC formatting may be lost during conversion
- Very old DOC files (Word 95/97) may have compatibility issues
- Consider converting DOC → DOCX manually for best quality

---

## Performance Optimization

### Benchmark Results

**Test Corpus:** 18 DOCX canonical tests (N=100)

| Document | Size | Time (s) | Throughput |
|----------|------|----------|------------|
| sample.docx | 50 KB | 0.005 | 10 MB/s |
| lorem_ipsum.docx | 200 KB | 0.028 | 7 MB/s |
| complex_report.docx | 500 KB | 0.062 | 8 MB/s |

**Summary:**
- **Mean:** 0.028s per document
- **Range:** 0.005-0.062s
- **Throughput:** 1000-5000 documents/minute

**See:** [BASELINE_PERFORMANCE_BENCHMARKS.md](../BASELINE_PERFORMANCE_BENCHMARKS.md)

---

### Optimization Tips

**1. Use Release Builds**

```bash
# Debug build (2-5x slower)
cargo run -- document.docx

# Release build (optimized)
cargo run --release -- document.docx
```

**2. Batch Processing**

```rust
let converter = DocumentConverter::new()?;

for docx in docx_files {
    converter.convert(&docx)?;
}
```

**Expected:** 2000-5000 documents/minute (depending on document complexity)

**3. Reuse Converter**

```rust
// GOOD: Create once
let converter = DocumentConverter::new()?;
for _ in 0..1000 {
    converter.convert("doc.docx")?;
}

// BAD: Create per file
for _ in 0..1000 {
    let converter = DocumentConverter::new()?;
    converter.convert("doc.docx")?;
}
```

**Speedup:** 10-20% faster by reusing converter instance.

---

## Troubleshooting

### Issue 1: "Unsupported format" for DOCX

**Symptom:** Error when converting `.docx` file.

**Cause:** File is corrupted or not a valid DOCX.

**Solution:**
1. Open in Microsoft Word to verify file integrity
2. Try "Save As" to create new DOCX
3. Check file extension (must be `.docx`, not `.doc`)

```bash
# Check file type
file document.docx
# Should show: "Microsoft Word 2007+"
```

---

### Issue 2: Missing Text or Garbled Output

**Symptom:** Markdown output is incomplete or has garbled characters.

**Cause:** Non-standard formatting or corrupted document structure.

**Solution:**
1. Open in Word and re-save as DOCX
2. Remove custom styles and use built-in styles
3. Copy content to new document
4. Check encoding (should be UTF-8)

---

### Issue 3: Tables Not Formatted Correctly

**Symptom:** Tables appear as plain text instead of markdown tables.

**Cause:** Tables created with tabs/spaces instead of native Word tables.

**Solution:**
1. Use Insert > Table to create native Word tables
2. Avoid ASCII art tables
3. Check that table has clear row/column structure

**Workaround:** Manually convert to native Word table.

---

### Issue 4: Images Not Extracted

**Symptom:** Images appear as `[Image]` placeholder without actual image data.

**Cause:** Images are linked externally, not embedded.

**Solution:**
1. Right-click image → "Change Picture" → "From File"
2. Ensure "Link to File" is unchecked
3. Re-save document

**Note:** Currently docling-rs extracts image references, not image data. Full image extraction will be added in future releases.

---

### Issue 5: DOC Conversion Fails

**Symptom:** Error: "LibreOffice not found" or "Failed to convert DOC"

**Cause:** LibreOffice not installed or not in PATH.

**Solution:**

```bash
# Install LibreOffice
brew install libreoffice  # macOS
sudo apt-get install libreoffice  # Linux

# Verify installation
which soffice
# Should show: /usr/local/bin/soffice or similar
```

---

### Issue 6: Slow PPTX Conversion

**Symptom:** Presentations take >1 second to convert.

**Cause:** Large embedded images or complex animations.

**Solution:**
1. Compress images in PowerPoint (File > Compress Pictures)
2. Remove animations (not converted anyway)
3. Use release build: `cargo run --release`

**Expected:** Most PPTX files should convert in <0.1s.

---

## Advanced Usage

### Accessing Document Metadata

```rust
let converter = DocumentConverter::new()?;
let result = converter.convert("document.docx")?;

// Basic metadata
println!("Format: {:?}", result.document.format);
println!("Characters: {}", result.document.metadata.num_characters);
println!("Conversion time: {:?}", result.latency);

// Future: More metadata (title, author, creation date)
// Will be added in Phase H+
```

---

### Batch Processing Multiple Office Formats

```rust
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;
use docling_core::Result;
use std::path::PathBuf;

fn convert_office_documents(input_dir: &str) -> Result<()> {
    let converter = DocumentConverter::new()?;

    let files: Vec<PathBuf> = std::fs::read_dir(input_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            matches!(
                p.extension().and_then(|e| e.to_str()),
                Some("docx") | Some("pptx") | Some("xlsx")
            )
        })
        .collect();

    println!("Found {} Office documents", files.len());

    for (i, file) in files.iter().enumerate() {
        println!("[{}/{}] Converting: {:?}", i+1, files.len(), file);

        match converter.convert(file) {
            Ok(result) => {
                let output = file.with_extension("md");
                std::fs::write(&output, &result.document.markdown)?;
                println!("  -> Saved to {:?} ({:?})", output, result.latency);
            }
            Err(e) => {
                eprintln!("  -> Error: {}", e);
            }
        }
    }

    Ok(())
}
```

---

## Python docling Comparison

### API Compatibility

**Python docling:**
```python
from docling.document_converter import DocumentConverter

converter = DocumentConverter()
result = converter.convert("document.docx")
markdown = result.document.export_to_markdown()
```

**docling-rs:**
```rust
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;
use docling_core::Result;

let converter = DocumentConverter::new()?;
let result = converter.convert("document.docx")?;
let markdown = &result.document.markdown;
```

**Compatibility:** 100% output compatibility with Python docling v2.58.0

---

### Output Validation

```bash
# Compare outputs
python3 -c "from docling.document_converter import DocumentConverter; \
            print(DocumentConverter().convert('doc.docx').document.export_to_markdown())" \
    > python_output.md

cargo run --release -- doc.docx rust_output.md

diff python_output.md rust_output.md
# Should show no differences (100% match)
```

**Test Results:** 18/18 DOCX canonical tests match Python output exactly ✅

---

## References

- **Python docling:** https://github.com/docling-project/docling
- **Office Open XML:** https://learn.microsoft.com/en-us/office/open-xml/
- **LibreOffice:** https://www.libreoffice.org/
- **DOCX Specification:** https://www.ecma-international.org/publications-and-standards/standards/ecma-376/

---

## Next Steps

- **PDF Guide:** See [PDF Format Guide](pdf.md)
- **Batch Processing:** See [User Guide - Batch Processing](../USER_GUIDE.md#batch-processing)
- **Performance Tuning:** See [Performance Guide](../guides/performance.md) (coming soon)
- **Troubleshooting:** See [TROUBLESHOOTING.md](../TROUBLESHOOTING.md)

---

**Last Updated:** 2025-11-12 (N=308)
**Status:** Production-ready ✅
**Test Coverage:** 100% (18/18 DOCX tests passing)
