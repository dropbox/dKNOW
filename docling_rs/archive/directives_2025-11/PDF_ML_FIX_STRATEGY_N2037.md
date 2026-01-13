# PDF ML Fix Strategy - Session N=2037 Continuation

**Date:** 2025-11-24
**Status:** Strategy Documented

## Analysis Complete - stage09 is NOT the Bug

**Finding:** `stage09_document_assembler.rs` files are 99% identical between repos (only formatting diffs).

**This means:**
- Text assembly logic is the SAME in both repos
- Bug is NOT in stage09
- Bug must be in an EARLIER stage OR in how the pipeline is configured

## Where is the Bug?

**Garbled output:** "PreDigtalEt" (missing spaces, letters)

**This suggests ONE of:**

1. **OCR text extraction broken** - OCR cells have garbled text
   - Location: `ocr/recognition.rs` or `ocr/detection.rs`
   - Test: Print OCR output BEFORE assembly

2. **Text cell confidence filtering too aggressive** - Filters out most text
   - Location: Pipeline configuration or filtering stages
   - Test: Lower confidence threshold, see if more text appears

3. **Reading order completely wrong** - Text cells assembled in wrong order
   - Location: `stage10_reading_order.rs` or `pipeline/reading_order.rs`
   - Test: Print reading order, verify it's sequential

4. **Pipeline configuration different** - Source repo uses different config
   - Location: `Pipeline::new()` and `PipelineConfig::default()`
   - Test: Compare PipelineConfig between repos

## Most Likely: Pipeline Configuration

**Hypothesis:** The source repo (`~/docling_debug_pdf_parsing`) uses a different `PipelineConfig` that produces correct output.

**Evidence:**
- stage09 is identical
- Code logic is the same
- But output quality drastically different

**This points to CONFIGURATION, not CODE.**

## Diagnostic Steps (In Order)

### Step 1: Compare Pipeline Configurations

```bash
# Source repo
grep -A 30 "impl Default for PipelineConfig" ~/docling_debug_pdf_parsing/src/pipeline/executor.rs

# Current repo
grep -A 30 "impl Default for PipelineConfig" crates/docling-pdf-ml/src/pipeline/executor.rs
```

**Look for:**
- OCR enabled/disabled
- Confidence thresholds
- Text extraction method
- Model selection

### Step 2: Compare Pipeline Construction

```bash
# How is Pipeline::new() called?
# Source repo
grep -B 5 -A 10 "Pipeline::new" ~/docling_debug_pdf_parsing/src/bin/docling.rs

# Current repo
grep -B 5 -A 10 "Pipeline::new" crates/docling-backend/src/pdf.rs
```

### Step 3: Add Debug Logging

If configs are the same, add logging to see WHERE text gets garbled:

```rust
// In stage09_document_assembler.rs, line 287
let textlines: Vec<String> = cluster
    .cells
    .iter()
    .map(|cell| {
        eprintln!("CELL TEXT: '{}'", cell.text);  // ADD THIS
        cell.text.replace('\x02', "-").trim().to_string()
    })
    .filter(|text| !text.is_empty())
    .collect();

eprintln!("SANITIZED: '{}'", self.sanitize_text(&textlines));  // ADD THIS
```

Run test and check output:
- If CELL TEXT is garbled → Bug is BEFORE stage09
- If CELL TEXT is clean but SANITIZED is garbled → Bug is IN sanitize_text

### Step 4: Check OCR Output

If cells are garbled, check OCR:

```rust
// In ocr/recognition.rs or wherever OCR runs
eprintln!("OCR OUTPUT: '{}'", recognized_text);
```

### Step 5: Source Repo Working Test

**CRITICAL:** Verify source repo ACTUALLY produces good output:

```bash
cd ~/docling_debug_pdf_parsing
source ../docling_rs/setup_env.sh

# Build and run on same PDF
cargo build --release
cargo run --bin docling -- ../../docling_rs/test-corpus/pdf/multi_page.pdf > /tmp/source_output.md

# Check output
wc -c /tmp/source_output.md  # Should be ~9,456 chars
head -20 /tmp/source_output.md  # Should have "Pre-Digital Era", not "PreDigtalEt"
```

**If source repo ALSO produces garbled output:**
- Both repos are broken
- Need to debug from scratch
- Start with OCR module

**If source repo produces CLEAN output:**
- Difference is in pipeline configuration or library versions
- Compare configs, model paths, library versions

## Quick Win Option: Use Source Repo Binary

**If source repo works, short-term fix:**

```bash
# Use source repo's working binary
cd ~/docling_debug_pdf_parsing
cargo build --release --bin docling

# Copy binary to current repo
cp target/release/docling ~/docling_rs/docling-pdf-ml-binary

# Call from Rust via subprocess (temporary hack)
# In crates/docling-backend/src/pdf.rs:
let output = Command::new("./docling-pdf-ml-binary")
    .arg(pdf_path)
    .output()?;
```

This gives you WORKING PDF ML while you debug the root cause.

## Time Estimates

- **Option 1:** Compare configs (30 min) → Fix if different (1 hour)
- **Option 2:** Add debug logging (1 hour) → Find bug (2-4 hours) → Fix (1-2 hours)
- **Option 3:** Debug OCR from scratch (8-16 hours)
- **Option 4:** Use source binary as subprocess (2 hours implementation)

## Recommendation

**Start with Step 5** (verify source repo works):
- If it works → Copy its configuration
- If it's broken → Both repos have same bug, need deep debugging

**Then Option 1** (compare configs):
- Most likely to reveal quick fix
- 30 min time investment

**Then Option 2** (debug logging):
- Will pinpoint exact location of bug
- 3-5 hours total

**Avoid Option 3** unless necessary:
- Only if configs identical AND debug logging unclear
- 8-16 hours

## Success Criteria

**Test command:**
```bash
source setup_env.sh
cargo test -p docling-backend --test pdf_honest_test --features pdf-ml -- --nocapture
```

**Current:** 701 chars, "PreDigtalEt"
**Target:** 9,000+ chars, "Pre-Digital Era"
**Pass:** Test assertion succeeds (output >=90% of Python baseline)

## Files to Check

**Priority 1:**
- `crates/docling-pdf-ml/src/pipeline/executor.rs` (PipelineConfig)
- `~/docling_debug_pdf_parsing/src/pipeline/executor.rs` (compare)
- `crates/docling-backend/src/pdf.rs` line 1109 (Pipeline::new call)

**Priority 2:**
- `crates/docling-pdf-ml/src/ocr/recognition.rs` (OCR output)
- `crates/docling-pdf-ml/src/pipeline/reading_order.rs` (text ordering)

**Priority 3:**
- Library versions in Cargo.toml (tch, ort versions)
- Model file paths/versions
