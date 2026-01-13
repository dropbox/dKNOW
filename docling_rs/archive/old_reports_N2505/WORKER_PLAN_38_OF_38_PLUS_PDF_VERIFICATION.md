# Worker Execution Plan: 38/38 Formats + Exhaustive PDF Verification

**Date:** 2025-12-02
**Branch:** Create new branch from main
**Manager:** Plan verified and approved
**Estimated Time:** 8-12 hours

---

## EXECUTIVE SUMMARY

### Goal 1: Get MOBI from 87% to 95% (38/38 formats)

**Problem:** Embedded HTML table TOC appears in output
**Root Cause:** Code looks for TOC in `<blockquote>` but Project Gutenberg uses `<table>`
**Solution:** Expand TOC detection to find and remove `<table>` elements containing TOC

### Goal 2: Exhaustively verify PDF parsing is correct

**Problem:** PDF was "fixed" but only tested on 1 file (multi_page.pdf)
**Skeptical Testing Required:** Test ALL 12 PDF files against groundtruth
**Verification:** DocItems count, types, content, markdown output, LLM quality

---

## PART 1: MOBI FIX (38/38 FORMATS)

### 1.1 The Problem (Evidence)

Running `./target/release/docling convert test-corpus/ebooks/mobi/multi_chapter.mobi` shows:

**Good (extracted TOC):**
```markdown
## Table of Contents
- [Chapter 1](#chapter_0)
- [Chapter 2](#chapter_1)
...
```

**Bad (embedded table TOC NOT removed):**
```
Contents [CHAPTER XII.]() | [CHAPTER I.]() |
Down the Rabbit-Hole | |------------------|---------------------------------| |
[CHAPTER II.]() | The Pool of Tears | ...
```

This embedded table appears AFTER the proper TOC because:
- Current code (`mobi.rs:217-301`) only looks for TOC in `<blockquote>` elements
- Project Gutenberg uses `<table>` elements for embedded TOC
- The table is NOT detected, NOT removed

### 1.2 The Fix

**File:** `crates/docling-ebook/src/mobi.rs`

**Strategy:** After finding blockquote TOC, ALSO look for and remove table TOC

**Implementation:**

```rust
// In extract_embedded_toc_with_removal(), after line 285:
// Add table detection

// Also check for TOC in table elements (Project Gutenberg pattern)
let table_selector = Selector::parse("table").ok()?;
for table in document.select(&table_selector) {
    let links: Vec<_> = table.select(&link_selector).collect();

    // Count TOC-like links (Chapter, Roman numerals, etc.)
    let toc_link_count = links.iter()
        .filter(|link| {
            let text = link.text().collect::<String>();
            is_toc_entry(text.trim())
        })
        .count();

    // If table has many TOC-like links (>= 5), remove it
    if toc_link_count >= 5 {
        // Find and mark table HTML for removal
        if let Some(start) = html.find("<table") {
            if let Some(end_offset) = html[start..].find("</table>") {
                let end = start + end_offset + "</table>".len();
                // Remove this table section
                cleaned_html = format!("{}{}", &cleaned_html[..start], &cleaned_html[end..]);
            }
        }
    }
}
```

### 1.3 Testing MOBI Fix

```bash
# 1. Build
cargo build -p docling-cli --release

# 2. Test multi_chapter.mobi (Alice in Wonderland)
./target/release/docling convert test-corpus/ebooks/mobi/multi_chapter.mobi > /tmp/mobi_output.md

# 3. Verify NO embedded table TOC
grep -c "Contents \[CHAPTER" /tmp/mobi_output.md
# Should be 0 (table TOC removed)

# 4. Verify proper TOC still exists
grep -c "## Table of Contents" /tmp/mobi_output.md
# Should be 1

# 5. Run LLM quality test
source .env
cargo test -p docling-backend test_ebook_quality_mobi -- --ignored --nocapture
# Should show >= 95%
```

### 1.4 MOBI Success Criteria

- [ ] Embedded table TOC removed from output
- [ ] Proper extracted TOC still present
- [ ] All 5 MOBI test files produce clean output
- [ ] LLM quality >= 95%
- [ ] No regression in other ebook formats (EPUB, FB2)

---

## PART 2: EXHAUSTIVE PDF VERIFICATION

### 2.1 PDF Test Corpus (12 files)

| File | Size | Description |
|------|------|-------------|
| multi_page.pdf | 128KB | 5 pages, word processor history |
| 2206.01062.pdf | 4.3MB | ArXiv paper, complex layout |
| 2305.03393v1.pdf | 4.3MB | ArXiv paper |
| 2305.03393v1-pg9.pdf | 162KB | Single page extract |
| 2203.01017v2.pdf | 7.2MB | ArXiv paper |
| amt_handbook_sample.pdf | 673KB | Technical handbook with OCR |
| code_and_formula.pdf | 89KB | Code blocks and formulas |
| picture_classification.pdf | 213KB | Image-heavy document |
| redp5110_sampled.pdf | 1.3MB | IBM Redbook sample |
| right_to_left_01.pdf | 103KB | RTL text (Hebrew/Arabic) |
| right_to_left_02.pdf | 92KB | RTL text |
| right_to_left_03.pdf | 278KB | RTL text |

### 2.2 Verification Protocol

**For EACH PDF file:**

1. **Parse with Rust ML backend**
```bash
source setup_env.sh
cargo test -p docling-backend --test pdf_verification_FILENAME --features pdf-ml -- --nocapture
```

2. **Compare DocItems count** against groundtruth JSON
```bash
# Python groundtruth
jq '.texts | length' test-corpus/groundtruth/docling_v2/FILENAME.json

# Rust output
cargo test show_docitems_FILENAME -- --nocapture | grep "Total DocItems"
```

3. **Compare DocItems by type**
```bash
# Python: count by label
jq '[.texts[].label] | group_by(.) | map({(.[0]): length})' FILENAME.json

# Rust: parse output and count
```

4. **Compare markdown length**
```bash
# Python groundtruth
wc -c test-corpus/groundtruth/docling_v2/FILENAME.md

# Rust output
./target/release/docling convert test-corpus/pdf/FILENAME.pdf | wc -c
```

5. **Visual diff first 500 chars**
```bash
head -c 500 test-corpus/groundtruth/docling_v2/FILENAME.md
./target/release/docling convert test-corpus/pdf/FILENAME.pdf | head -c 500
```

### 2.3 Create Comprehensive PDF Test

**Create new test file:** `crates/docling-backend/tests/pdf_exhaustive_test.rs`

```rust
//! Exhaustive PDF verification - ALL 12 PDF files

#[cfg(feature = "pdf-ml")]
mod tests {
    use docling_backend::{BackendOptions, DocumentBackend, PdfBackend};
    use std::fs;
    use std::path::Path;

    const PDF_TEST_FILES: &[(&str, usize, usize)] = &[
        // (filename, expected_docitems, expected_markdown_chars)
        ("multi_page.pdf", 53, 9456),
        ("2206.01062.pdf", 0, 0),  // Fill in after verification
        ("2305.03393v1.pdf", 0, 0),
        ("2305.03393v1-pg9.pdf", 0, 0),
        ("2203.01017v2.pdf", 0, 0),
        ("amt_handbook_sample.pdf", 0, 0),
        ("code_and_formula.pdf", 0, 0),
        ("picture_classification.pdf", 0, 0),
        ("redp5110_sampled.pdf", 0, 0),
        ("right_to_left_01.pdf", 0, 0),
        ("right_to_left_02.pdf", 0, 0),
        ("right_to_left_03.pdf", 0, 0),
    ];

    #[test]
    fn test_all_pdfs_docitems_count() {
        let backend = PdfBackend::new().expect("Failed to create backend");

        for (filename, expected_items, expected_chars) in PDF_TEST_FILES {
            let path = format!("../../test-corpus/pdf/{}", filename);
            if !Path::new(&path).exists() {
                println!("SKIP: {} not found", filename);
                continue;
            }

            let pdf_data = fs::read(&path).expect("Failed to read file");
            let doc = backend.parse_bytes(&pdf_data, &BackendOptions::default())
                .expect("Failed to parse");

            let actual_items = doc.content_blocks.as_ref().map(|v| v.len()).unwrap_or(0);
            let actual_chars = doc.markdown.len();

            println!("{}: DocItems={} (expected {}), Chars={} (expected {})",
                filename, actual_items, expected_items, actual_chars, expected_chars);

            if *expected_items > 0 {
                assert_eq!(actual_items, *expected_items,
                    "{}: DocItems mismatch", filename);
            }
            if *expected_chars > 0 {
                let tolerance = (*expected_chars as f64 * 0.05) as usize; // 5% tolerance
                assert!((actual_chars as i64 - *expected_chars as i64).abs() < tolerance as i64,
                    "{}: Chars {} outside 5% of {}", filename, actual_chars, expected_chars);
            }
        }
    }

    #[test]
    fn collect_pdf_baselines() {
        // Run this ONCE to collect expected values from groundtruth
        for entry in fs::read_dir("../../test-corpus/groundtruth/docling_v2").unwrap() {
            let entry = entry.unwrap();
            let name = entry.file_name().to_string_lossy().to_string();

            if name.ends_with(".json") && !name.contains("pages.meta") {
                let json_data = fs::read_to_string(entry.path()).unwrap();
                let json: serde_json::Value = serde_json::from_str(&json_data).unwrap();

                let docitems = json.get("texts")
                    .and_then(|t| t.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);

                // Check if corresponding .md exists
                let md_path = entry.path().with_extension("md");
                let md_chars = if md_path.exists() {
                    fs::read_to_string(&md_path).map(|s| s.len()).unwrap_or(0)
                } else {
                    0
                };

                if name.contains("2206") || name.contains("2305") || name.contains("2203")
                    || name.contains("multi_page") || name.contains("amt")
                    || name.contains("code") || name.contains("picture")
                    || name.contains("redp") || name.contains("right") {
                    println!("(\"{}\", {}, {}),",
                        name.replace(".json", ".pdf"), docitems, md_chars);
                }
            }
        }
    }
}
```

### 2.4 Skeptical Verification Checklist

**For multi_page.pdf (the "fixed" file):**

- [ ] DocItems count = EXACTLY 53
- [ ] DocItems types: 16 text, 11 section_header, 26 list_item
- [ ] Reading order: Title first ("The Evolution of the Word Processor")
- [ ] No fragmentation: "Pre-Digital Era..." is ONE DocItem (not split)
- [ ] Markdown length within 1% of 9,456 chars
- [ ] First 200 chars match groundtruth exactly
- [ ] LLM quality >= 98%

**For 2206.01062.pdf (complex ArXiv):**

- [ ] Page 1 DocItems reasonable for academic paper
- [ ] Title extracted correctly
- [ ] Authors extracted
- [ ] Abstract present
- [ ] Tables/figures handled
- [ ] No garbled text (encoding issues)

**For code_and_formula.pdf:**

- [ ] Code blocks preserved
- [ ] Formulas rendered (or noted as formula)
- [ ] Indentation correct

**For amt_handbook_sample.pdf (OCR):**

- [ ] OCR text readable
- [ ] No garbage characters
- [ ] Tables handled

**For right_to_left_*.pdf:**

- [ ] RTL text direction handled
- [ ] No reversed characters
- [ ] Hebrew/Arabic readable

### 2.5 PDF Verification Success Criteria

- [ ] ALL 12 PDF files parse without error
- [ ] DocItems counts match groundtruth (±5% for complex PDFs)
- [ ] Markdown length within 10% of groundtruth
- [ ] No garbled text in any output
- [ ] LLM quality >= 95% for standard PDFs
- [ ] RTL PDFs produce readable output
- [ ] Code/formula PDF preserves structure

---

## PART 3: EXECUTION ORDER

### Phase 1: Setup (30 min)

```bash
# 1. Create feature branch
git checkout main
git pull origin main
git checkout -b feature/38-of-38-plus-pdf-verification

# 2. Verify environment
source setup_env.sh
source .env
cargo build -p docling-backend --features pdf-ml
```

### Phase 2: MOBI Fix (2-3 hours)

1. Read `crates/docling-ebook/src/mobi.rs` lines 200-320
2. Understand current `extract_embedded_toc_with_removal()` logic
3. Add table TOC detection (see section 1.2)
4. Test on multi_chapter.mobi
5. Test on all 5 MOBI files
6. Run LLM quality test
7. Commit when passing

### Phase 3: PDF Exhaustive Test Creation (2-3 hours)

1. Create `pdf_exhaustive_test.rs`
2. Run `collect_pdf_baselines` test to get expected values
3. Fill in expected values from groundtruth
4. Run tests on all 12 PDFs
5. Document any failures

### Phase 4: PDF Verification (3-4 hours)

1. For each of 12 PDFs:
   - Parse with Rust backend
   - Compare DocItems count
   - Compare markdown output
   - Check for quality issues
2. Document results in VERIFICATION_REPORT.md
3. Fix any issues found
4. Re-run until all pass

### Phase 5: Final Verification (1 hour)

1. Run full test suite
2. Run LLM quality on MOBI (>= 95%)
3. Run LLM quality on PDFs (>= 95%)
4. Create PR with detailed results

---

## COMMIT TEMPLATE

```
# NNNN: [Description]

**Status:** [Progress]

## Changes

### MOBI (38/38)
- [x] Added table TOC detection to extract_embedded_toc_with_removal()
- [x] Embedded table TOC now removed from output
- [x] LLM quality: XX% (was 87%)

### PDF Exhaustive Verification
- [x] Created pdf_exhaustive_test.rs
- [x] Tested all 12 PDF files
- [x] Results: [summary]

## Test Results

| File | DocItems | Expected | Match |
|------|----------|----------|-------|
| multi_page.pdf | 53 | 53 | ✅ |
| ... | ... | ... | ... |

## Next AI: [Direction]

🤖 Generated with [Claude Code](https://claude.com/claude-code)
Co-Authored-By: Claude <noreply@anthropic.com>
```

---

## CRITICAL WARNINGS

### Do NOT:
- Skip any PDF file in verification
- Claim success without running LLM tests
- Assume PDF is correct because one test passes
- Ignore RTL PDF failures
- Accept <95% LLM quality for MOBI

### Do:
- Test EVERY PDF file
- Document exact DocItems counts
- Compare against groundtruth JSON, not just markdown
- Run LLM verification with `source .env` first
- Commit frequently with detailed messages

---

## SUCCESS = 38/38 + ALL 12 PDFs VERIFIED

**MOBI:** LLM quality >= 95%
**PDF:** All 12 files match groundtruth (±5%)

When both are achieved, create PR to merge to main.
