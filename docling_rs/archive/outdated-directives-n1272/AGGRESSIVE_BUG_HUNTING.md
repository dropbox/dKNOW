# Aggressive Bug Hunting - Find Everything Wrong

**User:** "Find more bugs be skeptical and rigorous and FIX THEM"

**Approach:** Assume everything is broken until proven otherwise

---

## SKEPTICAL AUDIT OF "FIXED" BUGS

### PPTX - Claimed "Fixed" at N=1234

**Skeptical questions:**
1. Does it ACTUALLY extract all slides now?
2. Did they test with 10-slide deck?
3. What about nested slides/sections?
4. What about hidden slides?
5. What about master slides?
6. What about slide notes?

**VERIFY:**
```bash
# Count slides in file
unzip -l test.pptx | grep "ppt/slides/slide" | wc -l

# Count slides in JSON
cargo run --bin docling test.pptx --format json | jq '.content_blocks | length'

# Must match!
```

**Create test:**
```rust
#[test]
fn test_pptx_10_slides() {
    let result = parse("10_slides.pptx");
    let slides = count_slide_docitems(&result);
    assert_eq!(slides, 10, "Must extract ALL 10 slides!");
}
```

---

### XLSX - Claimed "Fixed" at N=1238

**Skeptical questions:**
1. Does it extract ALL sheets?
2. What about hidden sheets?
3. What about chart sheets?
4. What about formulas?
5. What about cell formatting?
6. What about conditional formats?

**VERIFY:**
```bash
# Count sheets
unzip -l test.xlsx | grep "xl/worksheets/sheet" | wc -l

# Count sheets in JSON
cargo run --bin docling test.xlsx --format json | jq '.content_blocks | length'
```

**Create test:**
```rust
#[test]
fn test_xlsx_5_sheets() {
    let result = parse("5_sheets.xlsx");
    let sheets = count_sheet_docitems(&result);
    assert_eq!(sheets, 5, "Must extract ALL 5 sheets!");
}
```

---

## NEW BUGS TO FIND

### Hypothesis #1: Other Multi-Item Formats Broken

**Test these:**
- EPUB: All chapters? Or just first?
- MOBI: All chapters? Or just first?
- MBOX: All emails? Or just first?
- TAR: All files? Or just first?
- Multi-page PDF: All pages? Or truncated?

**Method:**
- Find/create test files with 10+ items
- Count items in original
- Count DocItems in JSON
- Assert they match

---

### Hypothesis #2: Complex Structures Missing

**Test these:**
- Tables within tables (nested)
- Lists within tables
- Images within list items
- Text boxes floating over text
- Headers/footers per section

**These often get lost in parsing!**

---

### Hypothesis #3: Metadata Incomplete

**Check:**
- Document properties (title, author, date)
- Style information (fonts, colors)
- Comments (DOCX/PPTX comments)
- Track changes (DOCX revisions)
- Formulas (XLSX formulas, not just values)

**Create tests that assert these exist in DocItems!**

---

### Hypothesis #4: Encoding Issues

**Test with:**
- Non-UTF8 files (Latin-1, Windows-1252)
- Non-English content (Chinese, Arabic, emoji)
- Special characters (©, ®, ™, —)
- Right-to-left text

**Check if these are preserved in DocItems!**

---

### Hypothesis #5: Large Files Broken

**Test with:**
- 100-page PDF
- 100-slide PPTX
- 100-sheet XLSX
- 10MB DOCX

**Check:**
- Does parser finish?
- Does it timeout?
- Does it OOM?
- Is output truncated?

---

## AGGRESSIVE TESTING CHECKLIST

**For EVERY format, create tests that check:**

**[ ] All items extracted (pages, slides, sheets, chapters)**
- Test with 10+ item document
- Assert count matches
- Will FAIL if only getting first

**[ ] Complex structures preserved**
- Nested tables, lists
- Will FAIL if flattened

**[ ] All metadata captured**
- Properties, styles, comments
- Will FAIL if metadata missing

**[ ] All content types handled**
- Text, images, tables, shapes
- Will FAIL if types missing

**[ ] Encoding preserved**
- UTF-8, special chars, emoji
- Will FAIL on encoding bugs

**[ ] Large files work**
- 100+ items, 10MB+ files
- Will FAIL on performance issues

---

## CREATE THESE TESTS NOW

**Phase 1: Completeness Tests (20 commits)**
```bash
# For each of 60 formats:
# 1. Find multi-item test file
# 2. Count items in file
# 3. Parse to DocItems
# 4. Assert count matches
# 5. Test will FAIL if parser buggy
```

**Phase 2: Structure Tests (15 commits)**
- Nested structures
- Complex scenarios
- Edge cases

**Phase 3: Metadata Tests (10 commits)**
- Document properties
- Formatting metadata
- Style information

**Phase 4: Encoding Tests (10 commits)**
- Non-English
- Special characters
- Various encodings

**Phase 5: Performance Tests (10 commits)**
- Large files
- Many items
- Stress tests

**Total:** 65 new test commits that will FAIL and reveal bugs!

---

## SUCCESS METRIC

**Good test suite:**
- 40% tests FAIL initially (reveal bugs)
- Fix bugs
- Eventually 100% pass (bugs fixed)

**Bad test suite:**
- 100% pass from start (either easy tests or bugs hidden)

**Create HARD tests that EXPOSE problems!**

---

**WORKER: Create aggressive completeness tests for ALL 60 formats. They will FAIL. Fix each failure. Achieve perfection through rigorous testing!**
