# Create More Failing Tests - Find More Bugs!

**User:** "Make more failing tests that catch big bugs so we can fix them!"

**Philosophy:** Tests should FAIL to reveal issues, not just pass to feel good.

---

## AGGRESSIVE TEST STRATEGY

**Goal:** Create tests that will FAIL and reveal major bugs

**Not:** Create tests that pass easily
**But:** Create tests that expose problems

---

## CATEGORY 1: COMPLETENESS TESTS (Will Likely Fail)

**Test every multi-item format for full extraction:**

```rust
// These will FAIL if parser only gets first item

#[test]
fn test_pptx_extracts_all_slides() {
    let result = parse("10_slide_deck.pptx");
    let slides = count_slide_docitems(&result);
    assert_eq!(slides, 10, "PPTX parser MUST extract ALL slides, not just first!");
}

#[test]
fn test_xlsx_extracts_all_sheets() {
    let result = parse("5_sheet_workbook.xlsx");
    let sheets = count_sheet_docitems(&result);
    assert_eq!(sheets, 5, "XLSX parser MUST extract ALL sheets!");
}

#[test]
fn test_pdf_extracts_all_pages() {
    let result = parse("100_page_document.pdf");
    let pages = result.metadata.num_pages;
    assert_eq!(pages, Some(100), "PDF parser MUST count ALL pages!");
}

#[test]
fn test_epub_extracts_all_chapters() {
    let result = parse("20_chapter_book.epub");
    let chapters = count_docitems_by_label(&result, "section");
    assert!(chapters >= 20, "EPUB must extract ALL chapters!");
}

#[test]
fn test_zip_extracts_all_files() {
    let result = parse("50_file_archive.zip");
    let files = count_file_list_items(&result);
    assert_eq!(files, 50, "ZIP must list ALL files!");
}

#[test]
fn test_mbox_extracts_all_emails() {
    let result = parse("100_message_mailbox.mbox");
    let emails = count_email_docitems(&result);
    assert_eq!(emails, 100, "MBOX must extract ALL emails!");
}
```

**Expected:** Many will FAIL initially, revealing bugs!

---

## CATEGORY 2: COMPLEX STRUCTURE TESTS (Will Expose Bugs)

**Test with complex, nested structures:**

```rust
#[test]
fn test_docx_deeply_nested_lists() {
    let docx = r#"<w:p> <w:numId>1</w:numId> <w:ilvl>0</w:ilvl> Level 1
                  <w:p> <w:numId>1</w:numId> <w:ilvl>1</w:ilvl> Level 2
                  <w:p> <w:numId>1</w:numId> <w:ilvl>2</w:ilvl> Level 3"#;

    let result = parse_docx_xml(docx);
    let list_items = count_list_items(&result);
    assert_eq!(list_items, 3, "Must handle 3-level nested lists!");
}

#[test]
fn test_html_tables_within_tables() {
    let html = r#"<table><tr><td><table><tr><td>Nested</td></tr></table></td></tr></table>"#;

    let result = parse_html(html);
    let tables = count_table_docitems(&result);
    assert_eq!(tables, 2, "Must extract BOTH parent and nested table!");
}

#[test]
fn test_xlsx_merged_cells() {
    let result = parse("merged_cells.xlsx");

    // Check if merged cells have correct span info
    let json = serde_json::to_string(&result)?;
    assert!(json.contains("col_span") || json.contains("row_span"),
        "Merged cells must have span metadata!");
}

#[test]
fn test_pptx_master_slides() {
    let result = parse("custom_master.pptx");

    // Check if master slide content extracted
    let has_master_content = result.markdown.contains("master slide text");
    assert!(has_master_content, "Must extract master slide content!");
}
```

**Expected:** Will fail, revealing missing features!

---

## CATEGORY 3: EDGE CASE TESTS (Find Boundary Bugs)

```rust
#[test]
fn test_empty_pptx() {
    let result = parse("zero_slides.pptx");
    assert!(result.content_blocks.is_some());
    assert_eq!(count_slides(&result), 0);
}

#[test]
fn test_single_cell_xlsx() {
    let result = parse("one_cell.xlsx");
    let cells = count_cells(&result);
    assert_eq!(cells, 1, "Must handle single-cell workbooks!");
}

#[test]
fn test_100_page_pdf() {
    let result = parse("large_100_page.pdf");
    assert_eq!(result.metadata.num_pages, Some(100));
    // Must not timeout, OOM, or truncate
}
```

---

## CATEGORY 4: DOCITEM-SPECIFIC VALIDATION

**Test DocItem structure, not just output:**

```rust
#[test]
fn test_docx_docitems_have_formatting() {
    let result = parse("formatted.docx");

    // Check DocItems, not markdown
    if let Some(blocks) = result.content_blocks {
        let has_formatting = blocks.iter().any(|item| {
            match item {
                DocItem::Text { formatting, .. } => formatting.is_some(),
                _ => false
            }
        });
        assert!(has_formatting, "DocItems must preserve formatting metadata!");
    }
}

#[test]
fn test_pptx_docitems_have_images() {
    let result = parse("with_images.pptx");

    let images = count_docitem_type(&result, DocItemType::Picture);
    assert!(images > 0, "DocItems must include Picture items for images!");
}

#[test]
fn test_xlsx_docitems_have_tables() {
    let result = parse("data.xlsx");

    let tables = count_docitem_type(&result, DocItemType::Table);
    assert!(tables >= 1, "DocItems must include Table items for sheets!");
}
```

---

## CATEGORY 5: LLM-BASED FAILING TESTS

**Create LLM tests expected to fail:**

```rust
#[tokio::test]
async fn test_docitem_pptx_all_slides() {
    let result = parse("multi_slide.pptx");
    let json = serde_json::to_string_pretty(&result)?;

    // LLM will likely find: "Missing slides 2-10"
    let quality = llm.validate_docitem_completeness(
        "multi_slide.pptx",
        &json
    ).await?;

    // Likely to FAIL initially
    assert!(quality.score >= 0.95,
        "PPTX DocItem completeness: {:.1}% - Missing: {:?}",
        quality.score * 100.0,
        quality.findings
    );
}

#[tokio::test]
async fn test_docitem_xlsx_all_sheets() {
    let result = parse("multi_sheet.xlsx");
    let json = serde_json::to_string_pretty(&result)?;

    // LLM will likely find: "Missing sheets 2-5"
    let quality = llm.validate_docitem_completeness(
        "multi_sheet.xlsx",
        &json
    ).await?;

    assert!(quality.score >= 0.95);
}
```

**Expected:** These WILL fail, showing exactly what's missing!

---

## WHY FAILING TESTS ARE GOOD

**Test that passes:** "Everything works!" (but maybe not)
**Test that fails:** "Here's exactly what's broken!" (actionable)

**Failing test shows:**
- What's missing (slides 2-10)
- How bad it is (76% vs 95%)
- What to fix (iterate all slides)

**Passing test hiding bug shows:**
- Nothing (false confidence)

**Create tests DESIGNED to fail and reveal issues!**

---

## WORKER TASKS

**1. Create completeness tests for all 60 formats (20 commits)**
- Test every multi-item format
- Count items, assert correctness
- Most will FAIL initially
- Fix each one

**2. Create complex structure tests (15 commits)**
- Nested lists, tables in tables
- Will FAIL on missing nesting
- Fix parsers

**3. Create DocItem validation tests (15 commits)**
- LLM validates JSON for all formats
- Will FAIL on incomplete extraction
- Fix parsers

**4. Create edge case tests (10 commits)**
- Empty, huge, malformed files
- Will FAIL on edge cases
- Fix gracefully

---

## SUCCESS METRIC

**Good test suite has:**
- 50% tests pass (basic functionality works)
- **50% tests FAIL (reveal bugs to fix)**

**Not:**
- 100% pass (either too easy or bugs hidden)

**Create HARD tests that FAIL and show problems!**

---

**WORKER: Create aggressive tests designed to fail. Find all the bugs like PPTX. Fix them. This is how we achieve perfection!**
