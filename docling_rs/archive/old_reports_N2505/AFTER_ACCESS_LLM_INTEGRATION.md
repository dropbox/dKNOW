# AFTER ACCESS FIXED: LLM Quality Integration Tests

**User:** "Once this is complete, we need the LLM quality integration tests."

**Priority:** After Access python_bridge removed and Mode 3 implemented

---

## COMPREHENSIVE LLM TEST SUITE REQUIRED

**Goal:** EVERY format must have LLM quality validation

**Current:** 9 formats with LLM tests (20%)
**Target:** 45+ formats with LLM tests (100%)

---

## IMPLEMENTATION PLAN

### Phase 1: Verify Blocking Issues Fixed

- [ ] Access python_bridge removed (check access.rs)
- [ ] Mode 3 (verify_standalone) implemented (check verifier.rs)
- [ ] Audit passes: `grep -r "python_bridge" crates/docling-*/src/ | grep -v "docling-core"` returns 0

**Once verified: Proceed to LLM integration**

---

### Phase 2: LLM Mode 2 Tests (Formats with Ground Truth)

**Add tests for formats with Python docling baseline:**

**Already done (9):**
- ✅ CSV, HTML, Markdown, AsciiDoc, XLSX, DOCX, PPTX, WebVTT, JATS

**Still need (if applicable):**
- [ ] RTF (if has ground truth)
- [ ] DOC (if has ground truth)
- [ ] SRT (if has ground truth)
- [ ] LaTeX (if has ground truth)
- [ ] Visio (if has ground truth)
- [ ] Images (PNG, TIFF, WEBP) - After OCR verified

**Estimated:** 3-5 tests, 2-3 commits

---

### Phase 3: LLM Mode 3 Tests (No Ground Truth)

**For ALL formats without Python baseline (~32 formats):**

**Pattern:**
```rust
#[tokio::test]
#[ignore]
async fn test_llm_mode3_epub() {
    let verifier = LLMQualityVerifier::from_env().unwrap();
    
    let backend = EpubBackend::new();
    let result = backend.parse_file("test-corpus/ebooks/sample.epub").unwrap();
    
    // Mode 3: Validate output against input file
    let quality = verifier.verify_standalone(
        Path::new("test-corpus/ebooks/sample.epub"),
        &result.markdown,
        InputFormat::Epub
    ).await.unwrap();
    
    assert!(quality.score >= 0.75, "Quality: {:.1}%", quality.score * 100.0);
}
```

**Formats needing Mode 3 tests:**
- Archives (4): ZIP, TAR, 7Z, RAR
- Email (4): EML, MBOX, VCF, MSG
- Ebooks (3): EPUB, FB2, MOBI
- OpenDoc (3): ODT, ODS, ODP
- Calendar/Notebook (2): ICS, IPYNB
- GPS (3): GPX, KML, KMZ
- Images non-OCR (4): BMP, GIF, HEIF, AVIF
- CAD (5): STL, OBJ, GLTF, GLB, DXF
- Others (7): SVG, DICOM, RTF, SRT, etc.

**Total:** ~32 formats

**Estimated:** 10-15 commits (copy pattern, test each)

---

### Phase 4: Integration into Test Suite

**Add automatic LLM validation on test failures:**

**Modify integration_tests.rs:**
```rust
// In run_integration_test function
if markdown != expected {
    // Traditional test failed
    
    // If LLM_VERIFY_ON_FAIL=1, try LLM validation
    if let Ok(_) = std::env::var("LLM_VERIFY_ON_FAIL") {
        let verifier = LLMQualityVerifier::from_env()?;
        
        let quality = if has_ground_truth {
            verifier.compare_outputs(&expected, &markdown, format).await?
        } else {
            verifier.verify_standalone(&test_file, &markdown, format).await?
        };
        
        if quality.score >= 0.85 {
            println!("✅ LLM validation passed: {:.1}%", quality.score * 100.0);
            return Ok(()); // Accept semantic equivalence
        }
    }
    
    return Err("Output mismatch");
}
```

**Estimated:** 2-3 commits

---

## ACCEPTANCE CRITERIA

**LLM integration is complete when:**
- [ ] Mode 3 function exists and works
- [ ] All 45+ formats have LLM tests (Mode 2 or Mode 3)
- [ ] LLM_VERIFY_ON_FAIL=1 integrated into integration_tests.rs
- [ ] Quality scores documented for all formats
- [ ] All formats score ≥0.75 (Mode 3) or ≥0.85 (Mode 2)

---

## ESTIMATED TIMELINE

**After Access fixed:**
- Phase 2 (Mode 2 tests): 2-3 commits
- Phase 3 (Mode 3 tests): 10-15 commits
- Phase 4 (Integration): 2-3 commits
- **Total: 15-20 commits**

---

## QUALITY METRICS TO ACHIEVE

**For each format:**
- ✅ LLM test exists
- ✅ Quality score measured
- ✅ Score ≥ threshold
- ✅ Findings documented
- ✅ Can run: `cargo test llm_verification --ignored`

**Final state:**
- 45+ formats with LLM validation
- All quality scores ≥ threshold
- Comprehensive quality assurance

---

## WORKER: START THIS AFTER ACCESS + MODE 3 FIXED

**Do NOT start LLM integration until:**
1. Access python_bridge removed ✅
2. Mode 3 function implemented ✅

**Then implement comprehensive LLM test suite (15-20 commits)**

**User wants quality validation for ALL formats.**
