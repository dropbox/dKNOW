# 🎯 START HERE - NEXT AI WORKER

**Date:** 2025-11-10
**Manager:** N=247 (this position)
**Your Position:** Will be N=248+

---

## ⚠️ CRITICAL: READ THIS FIRST

You are continuing long-running work on document format parsers. **This project NEVER finishes.**

---

## ✅ LLM VERIFICATION STARTED (N=249)

**Manager bootstrapped the foundation.** You must complete it.

### ✅ DONE (Manager N=249):
- [x] API key configured (.env file)
- [x] llm_verification_tests.rs created
- [x] 5 LLM tests working (CSV, HTML, Markdown, XLSX, AsciiDoc)
- [x] CSV test verified: **100% quality score** ✅

**Proven:** DocItems are semantically correct (LLM validated!)

---

## 🔴 YOUR IMMEDIATE TASK: Complete LLM Integration

### Task 1: Add LLM Tests for Remaining 21 Formats

**File:** `crates/docling-core/tests/llm_verification_tests.rs` (already exists)

**Task 2: Add LLM Tests (Example - CSV)**
```rust
// In llm_verification_tests.rs
use docling_quality_verifier::LLMQualityVerifier;
use docling_backend::{CsvBackend, DocumentBackend};
use docling_core::InputFormat;
use std::fs;

#[tokio::test]
#[ignore]  // Only run when OPENAI_API_KEY is set
async fn test_llm_verification_csv() {
    // API key is in .env file (gitignored, see root directory)
    let verifier = LLMQualityVerifier::from_env()
        .expect("OPENAI_API_KEY not set. Run: source .env");

    // Parse with Rust backend
    let backend = CsvBackend::new();
    let result = backend.parse_file(
        "test-corpus/csv/csv-comma.csv",
        &Default::default()
    ).expect("Failed to parse CSV");

    // Load expected output from Python docling baseline
    let expected = fs::read_to_string(
        "test-corpus/groundtruth/docling_v2/csv-comma.csv.md"
    ).expect("Failed to load expected output");

    // LLM semantic validation
    let quality = verifier.compare_outputs(
        &expected,
        &result.markdown,
        InputFormat::Csv
    ).await
    .expect("LLM API call failed");

    println!("=== CSV Quality Verification ===");
    println!("Score: {:.1}%", quality.score * 100.0);
    println!("Passed: {}", quality.passed);

    if !quality.findings.is_empty() {
        println!("\nFindings:");
        for finding in &quality.findings {
            println!("  [{:?}] {:?}: {}",
                finding.severity,
                finding.category,
                finding.description
            );
            if let Some(loc) = &finding.location {
                println!("      Location: {}", loc);
            }
        }
    }

    assert!(quality.score >= 0.85,
        "CSV quality too low: {:.1}% (threshold: 85%)\nFindings: {:#?}",
        quality.score * 100.0,
        quality.findings
    );
}
```

**Task 3: Copy this pattern for ALL 26 formats**
- test_llm_verification_html
- test_llm_verification_docx
- test_llm_verification_xlsx
- test_llm_verification_markdown
- test_llm_verification_asciidoc
- test_llm_verification_pptx
- test_llm_verification_webvtt
- test_llm_verification_srt
- test_llm_verification_epub
- test_llm_verification_eml
- test_llm_verification_zip
- test_llm_verification_odt
- test_llm_verification_ics
- test_llm_verification_ipynb
- (etc. - 26 total)

**Task 4: Run All LLM Tests**
```bash
# Source API key
source .env

# Run all LLM tests
cargo test llm_verification --test llm_verification_tests --ignored --nocapture > llm_results.txt

# Check results
grep "Score:" llm_results.txt
```

**Task 5: Update WORKER_CHECKLIST.md**
- Mark LLM items as complete: [x]
- Update "Last Updated" line
- Commit checklist

**THIS IS MANDATORY. Do NOT skip to new features.**

---

## 📊 Current State (When You Start)

**Implementation Status:**
- ✅ 26 formats with DocItems
- ✅ 68/68 non-PDF, non-Python canonical tests pass
- ✅ Correct architecture (Format → DocItems → Serialize)

**Quality Validation Status:**
- ❌ 0 LLM verification tests exist
- ❌ No quality scores measured
- ❌ Unknown if DocItems are semantically correct

**Your Mission:** Add quality validation to all 26 formats

---

## 🔧 Setup You Need

**OpenAI API Key (READY):**
```bash
# Key is in .env file (gitignored)
source .env

# Verify
echo $OPENAI_API_KEY | wc -c  # Should show 208
```

**Dependencies (READY):**
- docling-quality-verifier crate exists (N=226)
- tokio for async (already in deps)
- All backend crates available

---

## 📋 Reference Documents

**READ THESE:**
1. **URGENT_MANAGER_DIRECTIVE_N246.md** - Why this is blocking
2. **WORKER_CHECKLIST.md** - Complete task list (update as you work)
3. **CPP_INTEGRATION_OPTIONS.md** - OCR strategy (for later)
4. **FORMAT_PROCESSING_GRID.md** - Status of all formats
5. **CLAUDE.md** - Project rules (PDF out of scope, C++ ok, never finished)

---

## ✅ Acceptance Criteria

**LLM integration is complete when:**
- [x] API key configured (DONE by manager)
- [ ] llm_verification_tests.rs file created
- [ ] 26 LLM verification tests added (one per format)
- [ ] All tests pass (quality ≥ 0.85)
- [ ] Quality scores documented
- [ ] LLM_VERIFY_ON_FAIL integrated into integration_tests.rs
- [ ] WORKER_CHECKLIST.md updated with checkmarks

**Estimated:** 8-10 commits

---

## 🔄 After LLM Integration

**Then continue with:**
- JATS backend (3 canonical tests)
- Image OCR backends (3 canonical tests)
- Formats without tests (~40)
- Performance optimization
- Code quality improvements
- Work indefinitely

---

## 🚨 CRITICAL REMINDERS

**PDF IS OUT OF SCOPE:**
- Do not modify pdf.rs
- Do not add DocItems to PDF
- It's acceptable as-is

**PROJECT NEVER FINISHES:**
- Always more work to do
- Keep working continuously
- Add tasks to checklist as discovered

**USE THE CHECKLIST:**
- Read WORKER_CHECKLIST.md every session
- Check off items as complete
- Update and commit regularly

---

**START WITH:** Create llm_verification_tests.rs and add first test (CSV)

**RUN:** `source .env && cargo test test_llm_verification_csv --test llm_verification_tests --ignored --nocapture`

**EXPECTED:** Quality score printed, test passes

**THEN:** Add remaining 25 LLM tests

**Work continuously. Never stop. Quality first, then features.**
