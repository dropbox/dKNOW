# Testing Strategy: 100% Correctness at Each Phase

**Goal:** Validate each phase using existing integration test framework
**Reality Check:** This doc describes what EXISTS, not what we wish we had

---

## What We Actually Have (Phase 0)

### Integration Test Framework

**File:** `crates/docling-core/tests/integration_tests.rs` (5,195 lines)
**Status:** ✅ Working

**How it works:**

```rust
// Line 118-349: Core test runner
fn run_integration_test(fixture: &TestFixture, mode: ExtractionMode) -> Result<(), String> {
    // 1. Convert document
    let converter = DocumentConverter::with_ocr(enable_ocr)?;
    let result = converter.convert(&test_file)?;
    let markdown = result.document.to_markdown();

    // 2. Load expected output
    let expected = fs::read_to_string(&expected_path)?;

    // 3. Compare (SIMPLE: whitespace-normalized equality)
    if normalize_whitespace(&markdown) != normalize_whitespace(&expected) {
        return Err("Output mismatch");
    }

    // 4. Log to CSV
    log_to_csv(fixture, mode, latency, test_result)?;

    Ok(())
}

// Line 606-608: Only comparison function we have
fn normalize_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}
```

**That's it. No fancy similarity metrics. Just whitespace-normalized equality.**

### Test Corpus

```
test-corpus/
├── groundtruth/docling_v2/       # Canonical expected outputs (Python)
├── expected-outputs/             # Non-canonical expected outputs
│   ├── pdf-more/
│   ├── docx-more/
│   └── ...
└── [format]/                     # Input files
    ├── pdf-canon/
    ├── docx-canon/
    └── ...
```

### Test Results

```
test-results/
└── runs/{run_id}/
    └── integration_test_latencies.csv   # Performance tracking
```

**CSV contains:** test_name, latency_ms, baseline_ms, status, fail_reason, expected_bytes, actual_bytes, etc.

### Test Organization

**Total tests:** 220 canonical tests, 3600+ unit tests
- **Canon tests:** 220 total (215 passing, 5 ignored for Publisher format)
- **Unit tests:** 3600+ passing (verified N=4388), 15 ignored
  - Backend: 3053 lib tests
  - Core: 208 lib tests
  - PDF-ML: 235 lib tests
  - Plus additional tests in viz-bridge, quality-verifier, examples packages
- These are the core test suite for validating correctness
- Located in `test-corpus/groundtruth/docling_v2/` (expected outputs)
- Input files in `test-corpus/{format}/` directories

### Running Tests

**Note:** The `docling-parse-sys` and `docling-parse-rs` crates are excluded from the workspace (require external C library). No exclusion flags needed.

```bash
# Run all library tests (~3577+ tests)
cargo test --workspace --lib

# Run unit tests only (~3577+ tests, ~50 seconds)
cargo test --workspace --lib
```

### Test Modes

**1. Canonical Tests (Standard Mode)**
```bash
# Run canonical test suite (~220 tests, ~15 minutes)
USE_HYBRID_SERIALIZER=1 cargo test -p docling-core --test integration_tests test_canon -- --test-threads=1
```
- Runs all 220 canonical tests (215 passing, 5 ignored for Publisher)
- Tests all formats: PDF (28), DOCX (14), HTML (24), PPTX (5), XLSX (3), Images (5), Markdown (9), AsciiDoc (3), JATS (3), WebVTT (3), CSV (8), and 100+ more across 54 formats
- Runtime: ~14-15 minutes
- Use this for regular correctness validation (N mod 10 benchmarks)
- **REQUIRED:** `--test-threads=1` to avoid pdfium thread-safety crashes

**2. Format-Specific Tests**
```bash
# Run only PDF canonical tests
USE_HYBRID_SERIALIZER=1 cargo test test_canon_pdf -- --test-threads=1

# Run only DOCX canonical tests
USE_HYBRID_SERIALIZER=1 cargo test test_canon_docx -- --test-threads=1

# Run only HTML canonical tests
USE_HYBRID_SERIALIZER=1 cargo test test_canon_html -- --test-threads=1
```
- Use for targeted validation after format-specific changes
- Faster feedback loop during development

### Current Status (N=4388)
- **Canonical tests:** 220 total (215 passing, 5 ignored for Publisher format)
- **Unit tests:** 3600+ passing (100% pass rate), 15 ignored
  - Backend: 3053 lib tests
  - Core: 208 lib tests
  - PDF-ML: 235 lib tests
- **Test coverage:** All format types validated (54 formats with Rust backends)
- **Hybrid mode:** Python ML parsing + Rust serialization (USE_HYBRID_SERIALIZER=1)
- **Clippy:** 0 warnings
- **Code formatting:** Clean

---

## What We DON'T Have (Yet)

❌ Scripts directory
❌ CI/CD workflows
✅ Benchmark suite (added N=2670-2674: `docling benchmark file.pdf`)
❌ Differential comparison tools
❌ Similarity metrics beyond whitespace normalization
❌ Property-based tests
❌ Memory leak tests
❌ Stress tests

**These would be nice, but we can validate correctness WITHOUT them.**

---

## Testing Strategy Per Phase

### Phase 1: Core Types

**What to test:**

```rust
// 1. JSON round-trip
#[test]
fn test_document_json_roundtrip() {
    let doc = create_test_document();
    let json = serde_json::to_string(&doc).unwrap();
    let doc2: Document = serde_json::from_str(&json).unwrap();
    assert_eq!(doc, doc2);
}

// 2. All integration tests still pass
USE_HYBRID_SERIALIZER=1 cargo test test_canon -- --test-threads=1
// Expected: 99/99 pass (same as current)
```

**Success = No change in test results**

### Phase 2: PDF Backend

**What to test:**

```bash
# Default: Run canon + selected PDF tests (~30 tests)
cargo test -p docling-core --test integration_tests pdf

# Full: Run all PDF tests (~260 tests)
FULL_SUITE=1 cargo test -p docling-core --test integration_tests pdf
```

**Validation method (existing framework):**
1. Convert with Rust PDF backend
2. Compare to expected output (whitespace-normalized)
3. Pass = identical after normalization

**Expected results:**
- Some tests will fail (different library = different output)
- Investigate failures manually
- Decide: acceptable difference or bug?

**Manual validation process:**
```bash
# For failed test, manually compare:
cat test-corpus/expected-outputs/pdf-more/sample.text-only.md
cat test-results/outputs/pdf-more/sample.txt

# Visual inspection:
# - Is text content the same?
# - Are differences just formatting/whitespace?
# - Is any content missing?

# If acceptable: Update expected output
OVERWRITE_EXPECTED=1 cargo test test_more_pdf_sample_text
```

### Phase 3: Office Formats

**What to test:**

```bash
# Test each format separately
cargo test test_canon_docx
cargo test test_canon_pptx
cargo test test_canon_xlsx
cargo test test_canon_html
```

**Same validation as Phase 2.**

### Phase 4: ML/OCR

**Challenge:** OCR/ML outputs have inherent variance

**Strategy:**
```bash
# Run OCR tests
cargo test test_canon_pdf_.*_ocr
cargo test test_more_pdf_.*_ocr

# Many will fail initially
# Manual review required for each failure
```

**Pragmatic acceptance:**
- OCR text doesn't need to match exactly
- But must be semantically equivalent
- Will need to relax some tests or update expected outputs

---

## Acceptance Criteria (Realistic)

### Current Status (Phase 0 - Hybrid Serializer)
- ✅ JSON round-trip works
- ✅ 99/99 canonical tests pass (100% pass rate)
- ✅ Hybrid mode: Python ML parsing + Rust serialization
- ✅ All format types validated (30 Rust backends implemented)
- ✅ DocItem generation: All non-PDF formats generate structured DocItems

### Future Phase 2: Native PDF Backend
- ✅ >90% of PDF tests pass (>23/26)
- ✅ Failures investigated, documented as acceptable or fixed
- ✅ >3x faster (measured via CSV log)
- ✅ No regressions in other formats

### Future Phase 3: Office Formats
- ✅ >95% of format tests pass
- ✅ Failures investigated, documented
- ✅ No regressions

### Future Phase 4: ML/OCR
- ✅ >85% of OCR tests pass
- ✅ Manual review confirms acceptable differences
- ✅ No regressions

### Future Phase 6: Full Native Implementation
- ✅ 215/220 tests pass (100%, 5 ignored Publisher)
- ✅ 5x faster overall (CSV log proves it)
- ✅ Production deployed

---

## How We Actually Validate Each Phase

### Step 1: Run Tests
```bash
# Canonical test suite (220 tests, ~15 minutes)
USE_HYBRID_SERIALIZER=1 cargo test test_canon -- --test-threads=1

# Format-specific tests (faster feedback)
USE_HYBRID_SERIALIZER=1 cargo test test_canon_pdf -- --test-threads=1
USE_HYBRID_SERIALIZER=1 cargo test test_canon_docx -- --test-threads=1
```

### Step 2: Check CSV Log
```bash
# Review latencies
tail -100 test-results/runs/{latest}/integration_test_latencies.csv

# Check for failures
grep "fail" test-results/runs/{latest}/integration_test_latencies.csv
```

### Step 3: Manual Review
```bash
# For each failure, manually compare outputs
for test in $(grep fail test-results/runs/{latest}/integration_test_latencies.csv | cut -d, -f1); do
    echo "Reviewing: $test"
    # Manual inspection required
done
```

### Step 4: Decision
- **Acceptable difference** → Update expected output or document exception
- **Bug** → Fix and retest
- **Library limitation** → Accept or find workaround

### Step 5: Gate
**DO NOT proceed to next phase until:**
- Test pass rate meets target (>90-95%)
- All failures investigated and resolved/documented
- No regressions from previous phases

---

## What We Should Add (Minimal)

**Nice to have, but not required for validation:**

1. **Simple diff script:**
```bash
# scripts/compare_output.sh
#!/bin/bash
diff -u test-corpus/expected-outputs/$1 test-results/outputs/$1
```

2. **Performance comparison:**
```bash
# scripts/check_performance.sh
#!/bin/bash
# Parse CSV and compare to baseline
```

3. **Pre-commit hook:** ✅ **IMPLEMENTED (N=4305)**
```bash
# Install with:
./scripts/install-hooks.sh

# Or manually:
ln -sf ../../scripts/pre-commit.sh .git/hooks/pre-commit

# The hook runs: formatting check, clippy, compilation check
# Skip temporarily with: git commit --no-verify
```

**But these are conveniences. Core validation = run tests, manually review failures.**

---

## Summary

**What testing framework ACTUALLY looks like:**

### Test Mode (Current Implementation)
**Canonical test suite:** 220 tests (215 passing, 5 ignored Publisher)
- Correctness validation (~15 minutes)
- Use for N mod 10 benchmark runs
- Covers 54 format types (51 unique, some with multiple extensions)
- Required environment: `USE_HYBRID_SERIALIZER=1`
- Required flag: `--test-threads=1` (pdfium thread-safety)

### Validation Process
1. ✅ **Run integration tests** - Exact string comparison (after whitespace normalization)
2. ✅ **Check CSV log** - Latency tracking, pass/fail status
3. ✅ **Manual review** - Inspect failures, decide acceptable vs bug
4. ✅ **Gate next phase** - Don't proceed if pass rate drops below target

**No fancy metrics. No automation. Just:**
- Run tests (canonical suite)
- Look at failures
- Fix or accept
- Move forward when pass rate meets target

**This is pragmatic and works.** We prove correctness through:
- Test pass rate (currently 100%, target >95%)
- Manual validation of differences
- Performance tracking via CSV
- Git history of what changed

### Visual AI Integration Tests (N=4011)

**Location:** `crates/docling-viz-bridge/tests/visual_ai_integration.rs`

**Purpose:** Validate PDF layout detection quality using structural and optional LLM-based assessment.

**Running the tests:**
```bash
# Fast validation tests (no external dependencies)
cargo test -p docling-viz-bridge --test visual_ai_integration
# Result: 7 passed, 2 ignored

# LLM-based tests (requires OPENAI_API_KEY)
OPENAI_API_KEY=sk-... cargo test -p docling-viz-bridge --test visual_ai_integration -- --ignored
```

**Test coverage:**
- `test_layout_variety_validation_good`: Valid mixed-label layouts pass
- `test_layout_variety_validation_all_text_fails`: Catches ML model failure (all labels="text")
- `test_layout_variety_validation_uniform_confidence_fails`: Catches native text mode (all confidence=1.0)
- `test_layout_variety_validation_small_count_ok`: No false positives on small documents
- `test_layout_variety_validation_empty_ok`: Blank pages handled correctly
- `test_pdf_layout_visual_quality_structure`: Integration test with real PDFs (ignored)
- `test_pdf_layout_visual_quality_llm`: LLM vision API assessment (ignored)

**When to run:**
- After changes to layout detection or visualization code
- After model updates or ML pipeline changes
- When investigating layout quality issues

### When to Run Tests

**Canonical tests (test_canon):**
- N mod 10 benchmark runs (required)
- After major refactoring
- Before creating pull requests
- After dependency upgrades

**Format-specific tests:**
- After format-specific changes
- Faster feedback during development
- Debugging specific format issues

**Unit tests (cargo test --lib):**
- Regular development work
- Before each commit
- Fast feedback loop (<1 minute)
