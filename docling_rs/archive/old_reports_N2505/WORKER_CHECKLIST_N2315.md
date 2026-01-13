# Worker Checklist N=2315: Fix PDF Table Rendering

## ALSO FIX: False Positive Test

There's a test marked as failing that is actually correct behavior:
- **Judgment:** ❌ **FALSE POSITIVE** - This is accurate extraction, not a bug

Find and fix this test:
- [ ] Search for tests with "false positive" comments or incorrect assertions
- [ ] Update test expectations to match actual correct output
- [ ] The extraction is working correctly - the test expectation is wrong

## Environment Setup (REQUIRED FIRST)

- [ ] Run `source setup_env.sh` to set libtorch paths
- [ ] Run `source .env` to set API key (if needed for LLM tests)

## Phase 1: Understand Current State

- [ ] Build: `cargo build -p docling-cli --release --features pdf-ml`
- [ ] Test: `./target/release/docling convert test-corpus/pdf/2305.03393v1-pg9.pdf`
- [ ] Verify: Table content appears in output but NOT as markdown table format

## Phase 2: Debug Table Structure

### 2A: Check TableFormer Output in table_inference.rs

File: `crates/docling-pdf-ml/src/pipeline/table_inference.rs`

- [ ] Add debug logging after line 347:
  ```rust
  eprintln!("TABLE DEBUG: num_rows={}, num_cols={}", num_rows, num_cols);
  ```
- [ ] Add debug logging after line 437:
  ```rust
  eprintln!("TABLE DEBUG: Created {} cells with text",
      table_cells.iter().filter(|c| !c.text.is_empty()).count());
  ```
- [ ] Rebuild and test - verify num_rows/num_cols are non-zero

### 2B: Check Grid Generation in convert.rs

File: `crates/docling-pdf-ml/src/convert.rs`

- [ ] Add debug logging around line 278:
  ```rust
  eprintln!("GRID DEBUG: Building {}x{} grid from {} cells",
      num_rows, num_cols, cells.len());
  eprintln!("GRID DEBUG: First cell text: {:?}", cells.get(0).map(|c| &c.text));
  ```
- [ ] Rebuild and test - verify grid dimensions are correct

### 2C: Check MarkdownSerializer

File: `crates/docling-core/src/serializer/markdown.rs`

- [ ] Check line 736: `if data.grid.is_empty()` - is it returning early?
- [ ] Add debug: `eprintln!("SERIALIZE: grid.len()={}, num_rows={}", data.grid.len(), data.num_rows);`

## Phase 3: Fix the Bug

Based on debugging, apply ONE of these fixes:

### Option A: TableFormer returns 0 rows/cols
- [ ] Check line 335-343 in table_inference.rs
- [ ] If no "nl" tags found, num_rows = 0
- [ ] Fix: Use table_cells.len() / assumed columns as fallback

### Option B: Cells have empty text
- [ ] Check find_matching_ocr_text() at line 235
- [ ] Verify bboxes_overlap() is working correctly
- [ ] Fix: Adjust coordinate system or overlap threshold

### Option C: Grid not being used by serializer
- [ ] Check if DoclingDocument.tables is populated
- [ ] Check if serialize_table is being called
- [ ] Fix: Ensure table DocItem goes into correct collection

## Phase 4: Verify Fix

- [ ] Rebuild: `cargo build -p docling-cli --release --features pdf-ml`
- [ ] Test table PDF: `./target/release/docling convert test-corpus/pdf/2305.03393v1-pg9.pdf`
- [ ] Verify markdown table appears with proper format:
  ```
  | Column1 | Column2 | ... |
  |---------|---------|-----|
  | Data1   | Data2   | ... |
  ```

## Phase 5: Run Full Test Suite

- [ ] Test all 3 table PDFs:
  ```bash
  ./target/release/docling convert test-corpus/pdf/2305.03393v1-pg9.pdf
  ./target/release/docling convert test-corpus/pdf/2206.01062.pdf
  ./target/release/docling convert test-corpus/pdf/redp5110_sampled.pdf
  ```
- [ ] Compare output lengths to groundtruth (should be closer to expected)

## Phase 6: Commit

- [ ] Stage changes: `git add -A`
- [ ] Commit with message format:
  ```
  # 2315: Fix PDF Table Rendering - [describe fix]

  ## Changes
  - [What was wrong]
  - [How it was fixed]

  ## Next AI:
  - Test RTL PDFs
  - Run LLM quality tests
  ```

## Key Files Reference

| File | Purpose |
|------|---------|
| `crates/docling-pdf-ml/src/pipeline/table_inference.rs` | TableFormer output parsing |
| `crates/docling-pdf-ml/src/convert.rs` | TableElement → DocItem conversion |
| `crates/docling-core/src/serializer/markdown.rs` | DocItem → Markdown serialization |

## Success Metrics

- [ ] `2305.03393v1-pg9.pdf` renders markdown table (not just text)
- [ ] Table has correct number of rows and columns
- [ ] Cell content is populated correctly
- [ ] Output length closer to groundtruth (was -30.5%, target: within 10%)
