# HTML and Web Formats Guide

Complete guide to working with HTML, Markdown, and web-based document formats in docling-rs.

---

## Overview

**Formats:** HTML, Markdown, AsciiDoc, CSV
**Extensions:** `.html`, `.htm`, `.md`, `.markdown`, `.adoc`, `.asciidoc`, `.csv`
**Backend:** Python docling
**Status:** Production-ready ✅

**Test Coverage:**
- HTML: 100% (15/15 canonical tests passing)
- Markdown: 100% (all tests passing)
- AsciiDoc: 100% (3/3 canonical tests passing)
- CSV: 100% (7/7 canonical tests passing)

**Performance:**
- HTML: 0.002-0.011s per document (avg 0.005s)
- Markdown: <0.005s per document
- CSV: <0.01s per file

---

## HTML Documents

### Quick Start

```rust
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;
use docling_core::Result;

fn main() -> Result<()> {
    let converter = DocumentConverter::new()?;
    let result = converter.convert("webpage.html")?;

    println!("{}", result.document.markdown);
    Ok(())
}
```

### Features

**HTML Elements Supported:**
- ✅ Headings (`<h1>` - `<h6>`)
- ✅ Paragraphs (`<p>`)
- ✅ Lists (`<ul>`, `<ol>`, `<li>`)
- ✅ Tables (`<table>`, `<tr>`, `<td>`, `<th>`)
- ✅ Links (`<a href>`)
- ✅ Emphasis (`<b>`, `<strong>`, `<i>`, `<em>`)
- ✅ Code blocks (`<pre>`, `<code>`)
- ✅ Blockquotes (`<blockquote>`)
- ✅ Images (`<img>` → markdown image reference)
- ✅ Horizontal rules (`<hr>`)

**Not Extracted:**
- ❌ CSS styles (colors, fonts, layout)
- ❌ JavaScript code
- ❌ Forms (`<form>`, `<input>`)
- ❌ Navigation menus (extracted as lists)
- ❌ Iframes and embedded content

---

### Example: Web Page to Markdown

**Input HTML:**
```html
<!DOCTYPE html>
<html>
<head>
    <title>Product Documentation</title>
</head>
<body>
    <h1>Getting Started</h1>
    <p>Welcome to our <strong>product</strong>!</p>

    <h2>Features</h2>
    <ul>
        <li>Easy to use</li>
        <li>Fast performance</li>
    </ul>

    <h2>Pricing</h2>
    <table>
        <tr><th>Plan</th><th>Price</th></tr>
        <tr><td>Basic</td><td>$10/mo</td></tr>
        <tr><td>Pro</td><td>$20/mo</td></tr>
    </table>
</body>
</html>
```

**Output Markdown:**
```markdown
# Getting Started

Welcome to our **product**!

## Features

- Easy to use
- Fast performance

## Pricing

| Plan  | Price  |
|-------|--------|
| Basic | $10/mo |
| Pro   | $20/mo |
```

**Conversion Time:** ~0.005s

---

### HTML Best Practices

**1. Use Semantic HTML**

```html
<!-- GOOD: Semantic tags -->
<h1>Title</h1>
<p>Content</p>
<ul><li>Item</li></ul>

<!-- BAD: Styled divs -->
<div class="title">Title</div>
<div class="content">Content</div>
```

Semantic HTML preserves document structure in markdown.

**2. Avoid Complex Layouts**

```html
<!-- GOOD: Simple structure -->
<article>
    <h1>Title</h1>
    <p>Paragraph</p>
</article>

<!-- BAD: Complex grid layouts -->
<div class="grid">
    <div class="col-1">...</div>
    <div class="col-2">...</div>
</div>
```

Complex CSS layouts may produce confusing markdown structure.

**3. Use Native Tables**

```html
<!-- GOOD: HTML tables -->
<table>
    <tr><th>Header</th></tr>
    <tr><td>Data</td></tr>
</table>

<!-- BAD: CSS grid as table -->
<div class="table">
    <div class="row">
        <div class="cell">Header</div>
    </div>
</div>
```

---

### Use Cases

**Use Case 1: Web Scraping**

```rust
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;
use docling_core::Result;

fn scrape_documentation(url: &str) -> Result<String> {
    // Step 1: Download HTML (using reqwest or similar)
    let html_content = reqwest::blocking::get(url)?
        .text()?;

    // Step 2: Save to temp file
    let temp_file = "/tmp/page.html";
    std::fs::write(temp_file, html_content)?;

    // Step 3: Convert to markdown
    let converter = DocumentConverter::new()?;
    let result = converter.convert(temp_file)?;

    Ok(result.document.markdown)
}
```

**Use Case 2: Email HTML to Markdown**

```rust
fn convert_email_body(html_email: &str) -> Result<String> {
    // Save HTML email body
    std::fs::write("/tmp/email.html", html_email)?;

    // Convert to readable markdown
    let converter = DocumentConverter::new()?;
    let result = converter.convert("/tmp/email.html")?;

    Ok(result.document.markdown)
}
```

---

## Markdown Documents

### Quick Start

```rust
let converter = DocumentConverter::new()?;
let result = converter.convert("document.md")?;

// Markdown → Markdown conversion
// Useful for normalization and validation
```

### Why Convert Markdown to Markdown?

**Use Cases:**
1. **Normalization:** Convert non-standard markdown to CommonMark
2. **Validation:** Check if markdown is well-formed
3. **Preprocessing:** Part of pipeline (markdown → docling → structured data)
4. **Consistency:** Ensure consistent formatting across documents

### Example: Markdown Normalization

**Input:** `document.md` (non-standard)
```markdown
# Title
Some text

  * Inconsistent indentation
    * Sub-item

## Section

More text
```

**Output:** (Normalized CommonMark)
```markdown
# Title

Some text

- Inconsistent indentation
  - Sub-item

## Section

More text
```

---

## AsciiDoc Format

### Quick Start

```rust
let converter = DocumentConverter::new()?;
let result = converter.convert("document.adoc")?;

// AsciiDoc → Markdown
```

### Features

**AsciiDoc Elements:**
- ✅ Headings (`=`, `==`, `===`)
- ✅ Paragraphs
- ✅ Lists (bullet, numbered, definition)
- ✅ Tables
- ✅ Code blocks
- ✅ Admonitions (NOTE, TIP, WARNING)
- ✅ Cross-references
- ✅ Includes (if files exist)

### Example

**Input AsciiDoc:**
```asciidoc
= Document Title

== Introduction

This is a paragraph.

.Code Example
[source,rust]
----
fn main() {
    println!("Hello!");
}
----

== Features

* Feature 1
* Feature 2

[NOTE]
====
This is an important note.
====
```

**Output Markdown:**
```markdown
# Document Title

## Introduction

This is a paragraph.

**Code Example:**

```rust
fn main() {
    println!("Hello!");
}
```

## Features

- Feature 1
- Feature 2

> **Note:** This is an important note.
```

**Test Coverage:** 100% (3/3 canonical tests passing)

---

## CSV Format

### Quick Start

```rust
let converter = DocumentConverter::new()?;
let result = converter.convert("data.csv")?;

// CSV → Markdown table
```

### Features

**CSV Parsing:**
- ✅ Standard CSV (comma-separated)
- ✅ TSV (tab-separated)
- ✅ Custom delimiters (auto-detected)
- ✅ Quoted fields (`"value, with comma"`)
- ✅ Escaped quotes (`""` → `"`)
- ✅ Multi-line fields
- ✅ Header row detection

### Example

**Input CSV:**
```csv
Name,Age,City
Alice,30,New York
Bob,25,San Francisco
Charlie,35,Seattle
```

**Output Markdown:**
```markdown
| Name    | Age | City          |
|---------|-----|---------------|
| Alice   | 30  | New York      |
| Bob     | 25  | San Francisco |
| Charlie | 35  | Seattle       |
```

### Use Cases

**Use Case 1: Data Export**

```rust
fn export_database_to_markdown(csv_path: &str) -> Result<String> {
    // CSV from database export → Markdown table
    let converter = DocumentConverter::new()?;
    let result = converter.convert(csv_path)?;

    Ok(result.document.markdown)
}
```

**Use Case 2: Report Generation**

```rust
fn generate_report(sales_csv: &str) -> Result<String> {
    let converter = DocumentConverter::new()?;
    let result = converter.convert(sales_csv)?;

    // CSV table → Markdown → HTML report
    Ok(result.document.markdown)
}
```

**Test Coverage:** 100% (7/7 canonical tests passing)

---

## Performance

### Benchmark Results (N=100)

| Format | Avg Time | Throughput |
|--------|----------|------------|
| HTML | 0.005s | 10,000 docs/min |
| Markdown | 0.003s | 15,000 docs/min |
| AsciiDoc | 0.007s | 8,000 docs/min |
| CSV | 0.008s | 7,500 docs/min |

**Key Insight:** Web formats are the fastest to convert (5-10x faster than PDFs).

**See:** [BASELINE_PERFORMANCE_BENCHMARKS.md](../BASELINE_PERFORMANCE_BENCHMARKS.md)

---

## Troubleshooting

### Issue 1: Missing Content from HTML

**Symptom:** Some text or sections missing in markdown output.

**Cause:** Content in JavaScript-rendered elements (single-page apps).

**Solution:**
1. Use browser DevTools to save fully-rendered HTML
2. Or use headless browser (Puppeteer, Selenium) to render first
3. Then convert saved HTML to markdown

```rust
// Requires fully-rendered HTML (not JavaScript placeholders)
let converter = DocumentConverter::new()?;
let result = converter.convert("rendered_page.html")?;
```

---

### Issue 2: HTML Tables Not Formatted

**Symptom:** Tables appear as plain text.

**Cause:** Tables created with CSS grid/flexbox, not `<table>` tags.

**Solution:**
Use native HTML tables (`<table>`, `<tr>`, `<td>`). CSS-based tables are not detected.

---

### Issue 3: CSV Parse Errors

**Symptom:** "Failed to parse CSV" error.

**Cause:** Malformed CSV (unclosed quotes, inconsistent columns).

**Solution:**
1. Validate CSV: `csvlint data.csv`
2. Check for:
   - Unclosed quotes
   - Inconsistent column counts
   - Invalid escape sequences
3. Re-export from source (Excel, database) with standard CSV format

---

### Issue 4: AsciiDoc Includes Not Found

**Symptom:** `include::file.adoc[]` not expanded.

**Cause:** Included file path is relative or file doesn't exist.

**Solution:**
Ensure included files exist relative to main AsciiDoc file.

---

## Advanced Usage

### Batch HTML Conversion

```rust
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;
use docling_core::Result;
use std::path::PathBuf;

fn convert_website_docs(html_dir: &str) -> Result<()> {
    let converter = DocumentConverter::new()?;

    let html_files: Vec<PathBuf> = std::fs::read_dir(html_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("html"))
        .collect();

    println!("Converting {} HTML files...", html_files.len());

    for html_file in html_files {
        let result = converter.convert(&html_file)?;

        let md_file = html_file.with_extension("md");
        std::fs::write(&md_file, &result.document.markdown)?;

        println!("✓ {:?} → {:?}", html_file, md_file);
    }

    Ok(())
}
```

**Expected Performance:** 5000-15000 files/minute (depending on file size)

---

## Python docling Comparison

### Output Compatibility

**HTML:** 100% compatible (15/15 canonical tests match)
**AsciiDoc:** 100% compatible (3/3 canonical tests match)
**CSV:** 100% compatible (7/7 canonical tests match)

### Validation

```bash
# Compare HTML conversion
python3 -c "from docling.document_converter import DocumentConverter; \
            print(DocumentConverter().convert('page.html').document.export_to_markdown())" \
    > python_output.md

cargo run --release -- page.html rust_output.md

diff python_output.md rust_output.md
# Should show no differences ✅
```

---

## References

- **Python docling:** https://github.com/docling-project/docling
- **CommonMark Spec:** https://commonmark.org/
- **AsciiDoc:** https://asciidoc.org/
- **CSV RFC 4180:** https://tools.ietf.org/html/rfc4180
- **HTML5 Spec:** https://html.spec.whatwg.org/

---

## Next Steps

- **PDF Guide:** See [PDF Format Guide](pdf.md)
- **Office Formats:** See [Office Formats Guide](office.md)
- **Image Formats:** See [Image Formats Guide](images.md)
- **Performance Tuning:** See [Performance Guide](../guides/performance.md)

---

**Last Updated:** 2025-11-12 (N=308)
**Status:** Production-ready ✅
**Test Coverage:** 100% (HTML 15/15, AsciiDoc 3/3, CSV 7/7)
