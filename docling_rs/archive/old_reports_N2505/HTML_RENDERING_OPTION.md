# HTML Rendering for Visual Validation

**User:** "Worker can try rendering documents as HTML, too"

**Why HTML:** Better than PDF for some formats, easier to debug

---

## HTML RENDERING APPROACH

### Option 1: Document → HTML (Direct)

**LibreOffice can export to HTML:**
```bash
# DOCX → HTML
soffice --headless --convert-to html test.docx

# PPTX → HTML
soffice --headless --convert-to html test.pptx

# Result: HTML with embedded or linked images
```

**Benefits:**
- Preserves formatting (CSS)
- Images extracted automatically
- Tables as HTML tables
- Easier to debug (view source)

---

### Option 2: Markdown → HTML

**We already generate markdown, convert it:**
```bash
# Using pandoc
pandoc output.md -o output.html

# Or markdown library in Rust
# Already have markdown parsing crates
```

**Then compare:**
- Original HTML (from LibreOffice)
- Our markdown HTML (from our output)
- Visual diff or LLM comparison

---

### Option 3: Both Side-by-Side

```html
<!-- Create comparison page -->
<html>
<body>
<div style="display: flex;">
  <iframe src="original.html" style="width: 50%;"></iframe>
  <iframe src="our_output.html" style="width: 50%;"></iframe>
</div>
</body>
</html>
```

**View in browser:** See differences immediately

---

## LLM Vision with HTML

**Can use HTML instead of PDF:**

```rust
pub async fn compare_visual_html(
    &self,
    original_html: &str,
    output_html: &str,
    format: &str,
) -> Result<QualityReport> {
    // Screenshot both HTML pages
    let original_png = html_to_screenshot(original_html)?;
    let output_png = html_to_screenshot(output_html)?;

    // LLM compares screenshots
    self.compare_screenshots(original_png, output_png, format).await
}
```

**Or send HTML directly to LLM:**
```rust
// GPT-4o can read HTML
let prompt = format!("Compare these HTML renderings...");
// Include HTML in prompt (if not too large)
```

---

## BENEFITS OF HTML APPROACH

**Easier Debugging:**
- View HTML source
- Inspect CSS
- Check image links
- Validate table structure

**Better for Web Formats:**
- HTML → HTML (direct comparison)
- CSS preserved
- More accurate than PDF

**Faster Iteration:**
- No PDF conversion needed
- Instant browser preview
- Easier to spot issues

**More Informative:**
- Can check element presence (has `<strong>` for bold?)
- Can validate structure (correct heading levels?)
- Can test programmatically

---

## IMPLEMENTATION

### Add HTML Visual Tests

```rust
#[tokio::test]
#[ignore]
async fn test_visual_html_docx() {
    // Original DOCX → HTML (LibreOffice)
    let original_html = document_to_html("test.docx")?;

    // Our output: DOCX → Markdown → HTML
    let our_markdown = parse_docx("test.docx")?;
    let our_html = markdown_to_html(&our_markdown)?;

    // Compare HTML visually
    let score = compare_html_visual(original_html, our_html).await?;

    // OR compare as text/structure
    let score = compare_html_structure(original_html, our_html)?;

    assert!(score >= 1.0);
}
```

### HTML Structure Validation

**Can check programmatically:**
```rust
// Does our HTML have bold tags?
assert!(our_html.contains("<strong>") || our_html.contains("<b>"));

// Does our HTML have images?
assert!(our_html.contains("<img"));

// Does our HTML have tables?
assert!(our_html.contains("<table>"));
```

**Complement to visual LLM tests!**

---

## WORKER SHOULD USE BOTH

**PDF approach:**
- Good for visual fidelity
- Matches print output
- Better for Office documents

**HTML approach:**
- Better for debugging
- Easier to validate structure
- Good for web-based formats
- Faster iteration

**Use both for comprehensive validation!**

---

**Worker: Try HTML rendering too! Convert documents to HTML, compare to our markdown HTML. Easier to debug than PDF. See what's missing!**
