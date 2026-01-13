# LLM MODE 3 TEST GRID - Worker Must Complete

**Purpose:** Track LLM quality validation for ALL 45+ formats
**Worker:** You MUST fill out this grid and add tests
**Update:** Check off [x] as each test is added

**N=489 UPDATE:** ✅ **Verifier bug FIXED!** 12 binary format tests unblocked (77% pass rate, up from 46%)

---

## INSTRUCTIONS

**For each format below:**
1. Check if Mode 2 or Mode 3 applies
2. Add LLM test to `crates/docling-core/tests/llm_verification_tests.rs`
3. Run test: `OPENAI_API_KEY="..." cargo test test_llm_{format} -- --ignored --nocapture`
4. Document quality score
5. Check off [x] in grid
6. Commit

**Mode 2:** Formats with Python ground truth (compare outputs)
**Mode 3:** Formats WITHOUT Python ground truth (standalone validation)

---

## MODE 2 TESTS (With Ground Truth) - 9/9 Complete, 6/9 Pass ✅

| Format | Test Function | Status | Quality Score | Notes |
|--------|---------------|--------|---------------|-------|
| **CSV** | test_llm_verification_csv | [x] PASS ✅ | **100%** | Perfect score |
| **DOCX** | test_llm_verification_docx | [x] PASS ✅ | **100%** | Perfect score |
| **Markdown** | test_llm_verification_markdown | [x] PASS ✅ | **97%** | Minor heading issues |
| **JATS** | test_llm_verification_jats | [x] PASS ✅ | **95%** | Minor technical terms (up from 92.5%) |
| **WebVTT** | test_llm_verification_webvtt | [x] PASS ✅ | **85%** | Missing speaker attribution |
| **XLSX** | test_llm_verification_xlsx | [x] FAIL ❌ | **83%** | Missing header, formatting (down from 85%) |
| **HTML** | test_llm_verification_html | [x] FAIL ❌ | **78%** | Misplaced image, missing heading format (down from 83%) |
| **AsciiDoc** | test_llm_verification_asciidoc | [x] FAIL ❌ | **75%** | Missing title/abstract, wrong headings |
| **PPTX** | test_llm_docitem_pptx | [x] PASS ✅ | **85-88%** | With image extraction (N=1234), realistic quality for core features |
| **RTF** | test_llm_verification_rtf | [ ] TODO | - | Has ground truth? |
| **DOC** | test_llm_verification_doc | [ ] TODO | - | Has ground truth? |
| **SRT** | test_llm_verification_srt | [ ] TODO | - | Has ground truth? |
| **LaTeX** | test_llm_verification_latex | [ ] TODO | - | Has ground truth? |
| **Visio** | test_llm_verification_vsdx | [ ] TODO | - | Has ground truth? |
| **PNG** | test_llm_verification_png | [ ] TODO | - | After OCR verified |
| **TIFF** | test_llm_verification_tiff | [ ] TODO | - | After OCR verified |
| **WEBP** | test_llm_verification_webp | [ ] TODO | - | After OCR verified |

**Progress:** 9/17 complete (53%)

---

## MODE 3 TESTS (No Ground Truth) - 30/32 Complete (All Implemented Formats)

**⚠️ WORKER: You MUST add these tests using verify_standalone()!**

### Archives (4 formats)

| Format | Test Function | Status | Quality Score | Test File |
|--------|---------------|--------|---------------|-----------|
| **ZIP** | test_llm_mode3_zip | [x] PASS ✅ | **90%** | Verifier bug FIXED at N=489 |
| **TAR** | test_llm_mode3_tar | [x] PASS ✅ | **86%** | Verifier bug FIXED at N=489 |
| **7Z** | test_llm_mode3_7z | [x] PASS ✅ | **85%** | Verifier bug FIXED at N=489 |
| **RAR** | test_llm_mode3_rar | [x] PASS ✅ | **85%** | Verifier bug FIXED at N=489 |

**Progress:** 4/4 pass (100%) ✅

---

### Email (4 formats)

| Format | Test Function | Status | Quality Score | Test File |
|--------|---------------|--------|---------------|-----------|
| **EML** | test_llm_mode3_eml | [x] PASS ✅ | **87%** | Minor date format change |
| **MBOX** | test_llm_mode3_mbox | [x] PASS ✅ | **95%** | Excellent email parsing |
| **VCF** | test_llm_mode3_vcf | [x] PASS ✅ | **90%** | Good genomics format |
| **MSG** | test_llm_mode3_msg | [x] FAIL ❌ | **Parser Bug** | "Failed to parse MSG" |

**Progress:** 3/4 pass (1 parser bug)

---

### Ebooks (3 formats)

| Format | Test Function | Status | Quality Score | Test File |
|--------|---------------|--------|---------------|-----------|
| **EPUB** | test_llm_mode3_epub | [x] PASS ✅ | **84%** | Verifier bug FIXED at N=489 |
| **FB2** | test_llm_mode3_fb2 | [x] PASS ✅ | **84%** | Chapter title inconsistency |
| **MOBI** | test_llm_mode3_mobi | [x] FAIL ❌ | **File Missing** | "No such file or directory" |

**Progress:** 2/3 pass (1 file missing)

---

### OpenDocument (3 formats)

| Format | Test Function | Status | Quality Score | Test File |
|--------|---------------|--------|---------------|-----------|
| **ODT** | test_llm_mode3_odt | [x] FAIL ❌ | **74%** | Below 75% threshold - parser quality issue |
| **ODS** | test_llm_mode3_ods | [x] PASS ✅ | **85%** | Verifier bug FIXED at N=489 |
| **ODP** | test_llm_mode3_odp | [x] PASS ✅ | **80%** | Verifier bug FIXED at N=489 |

**Progress:** 2/3 pass (1 quality issue)

---

### Calendar/Notebook (2 formats)

| Format | Test Function | Status | Quality Score | Test File |
|--------|---------------|--------|---------------|-----------|
| **ICS** | test_llm_mode3_ics | [x] PASS ✅ | **93%** | Missing UID field (up from 88%) |
| **IPYNB** | test_llm_mode3_ipynb | [x] PASS ✅ | **87%** | Minor code block formatting (down from 94%) |

**Progress:** 2/2 pass (100%) ✅

---

### GPS (3 formats)

| Format | Test Function | Status | Quality Score | Test File |
|--------|---------------|--------|---------------|-----------|
| **GPX** | test_llm_mode3_gpx | [x] FAIL ❌ | **78%** | Missing full track points - below threshold |
| **KML** | test_llm_mode3_kml | [x] PASS ✅ | **84%** | Coordinate format improved |
| **KMZ** | test_llm_mode3_kmz | [x] PASS ✅ | **84%** | Verifier bug FIXED at N=489 |

**Progress:** 2/3 pass (1 quality issue)

---

### Images (8 formats - non-OCR)

| Format | Test Function | Status | Quality Score | Test File |
|--------|---------------|--------|---------------|-----------|
| **BMP** | test_llm_mode3_bmp | [x] FAIL ❌ | **Parser Bug** | "Invalid BMP header signature" |
| **GIF** | test_llm_mode3_gif | [x] PASS ✅ | **88%** | Verifier bug FIXED at N=489 |
| **HEIF** | test_llm_mode3_heif | [x] PASS ✅ | **81%** | Verifier bug FIXED at N=489 |
| **AVIF** | test_llm_mode3_avif | [x] PASS ✅ | **83%** | Verifier bug FIXED at N=489 |
| **DICOM** | test_llm_mode3_dicom | [x] PASS ✅ | **88%** | Verifier bug FIXED at N=489 |
| **PNG** | test_llm_mode3_png | [ ] Not Tested | - | Not in N=489 results |
| **JPEG** | test_llm_mode3_jpeg | [ ] Not Tested | - | Not in N=489 results |
| **TIFF** | test_llm_mode3_tiff | [ ] Not Tested | - | Not in N=489 results |

**Progress:** 4/8 pass (1 parser bug, 3 not tested)

---

### CAD/3D (5 formats)

| Format | Test Function | Status | Quality Score | Test File |
|--------|---------------|--------|---------------|-----------|
| **STL** | test_llm_mode3_stl | [x] PASS ✅ | **87%** | Missing facet normals (up from 84%) |
| **OBJ** | test_llm_mode3_obj | [x] PASS ✅ | **93%** | Minor title format issue (up from 92%) |
| **GLTF** | test_llm_mode3_gltf | [x] PASS ✅ | **83%** | Missing accessor/buffer details |
| **GLB** | test_llm_mode3_glb | [x] PASS ✅ | **92%** | Verifier bug FIXED at N=489 |
| **DXF** | test_llm_mode3_dxf | [x] FAIL ❌ | **54%** | Missing entity info, wrong version - very low |

**Progress:** 4/5 pass (1 quality issue)

---

### Other Formats (2 formats)

| Format | Test Function | Status | Quality Score | Test File |
|--------|---------------|--------|---------------|-----------|
| **SVG** | test_llm_mode3_svg | [x] PASS ✅ | **83%** | Missing hierarchical structure |

**Progress:** 1/1 pass (100%) ✅

**Note:** DICOM moved to Images section above

---

## TOTAL PROGRESS

**TESTS RUN:** 39/39 (100%) ✅
**TESTS PASSED:** 30/39 (77%) ✅✅
**TESTS FAILED:** 9/39 (23%)

### N=489 Update: Verifier Bug FIXED!

**Binary file verifier bug was FIXED at N=489**, unblocking 12 tests that were previously failing due to UTF-8 read errors on binary formats (ZIP, TAR, 7Z, RAR, EPUB, ODT, ODS, ODP, KMZ, GIF, HEIF, AVIF, GLB, DICOM).

### Breakdown:

**Mode 2 (with ground truth):** 6/9 pass (67%)
- Perfect: CSV, DOCX (100%)
- Good: Markdown (97%), JATS (95%)
- Acceptable: WebVTT (85%)
- **Failed:** XLSX (83%), HTML (78%), AsciiDoc (75%), PPTX (66%)

**Mode 3 (without ground truth):** 24/30 pass (80%) ✅
- **Archives:** 4/4 pass (ZIP 90%, TAR 86%, 7Z 85%, RAR 85%)
- **Email:** 3/4 pass (MBOX 95%, VCF 90%, EML 87%; MSG parser bug)
- **Ebooks:** 2/3 pass (EPUB 84%, FB2 84%; MOBI file missing)
- **OpenDocument:** 2/3 pass (ODS 85%, ODP 80%; ODT 74% quality)
- **Calendar/Notebook:** 2/2 pass (ICS 93%, IPYNB 87%)
- **GPS:** 2/3 pass (KML 84%, KMZ 84%; GPX 78% quality)
- **Images:** 4/5 pass (GIF 88%, DICOM 88%, AVIF 83%, HEIF 81%; BMP parser bug)
- **CAD/3D:** 4/5 pass (OBJ 93%, GLB 92%, STL 87%, GLTF 83%; DXF 54% quality)
- **Other:** 1/1 pass (SVG 83%)

### Root Cause Analysis:

**Remaining 9 failures breakdown:**
- **3 parser quality issues** (HTML 78%, AsciiDoc 75%, ODT 74% - below 75% threshold)
- **3 very low quality** (GPX 78%, PPTX 66%, DXF 54% - far below threshold)
- **2 parser bugs** (MSG "Failed to parse", BMP "Invalid header")
- **1 file missing** (MOBI "No such file or directory")

### After Fixes:

**Current status (N=489):** 30/39 pass (77%) ✅
**If remaining issues fixed:** 39/39 pass (100%) 🎯

**TARGET:** Fix 2 parser bugs + 1 file issue + 6 quality issues → 100% pass rate

---

## MODE 3 TEST PATTERN

**Copy this pattern for each format:**

```rust
#[tokio::test]
#[ignore]  // Only run when OPENAI_API_KEY set
async fn test_llm_mode3_epub() {
    let verifier = create_verifier();

    // Parse with Rust backend
    let backend = EpubBackend::new();
    let result = backend.parse_file(
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../test-corpus/ebooks/sample.epub"),
        &Default::default()
    ).expect("Failed to parse EPUB");

    // Verify DocItems exist
    assert!(result.content_blocks.is_some(), "Must generate DocItems");

    // Mode 3: Standalone validation (no ground truth)
    let input_file = std::path::Path::new(
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../test-corpus/ebooks/sample.epub")
    );

    let quality = verifier.verify_standalone(
        input_file,
        &result.markdown,
        InputFormat::Epub
    ).await.expect("LLM API failed");

    print_quality_report("EPUB Mode 3", &quality);

    assert!(quality.score >= 0.75,
        "EPUB quality too low: {:.1}% (Mode 3 threshold: 75%)",
        quality.score * 100.0
    );
}
```

**Change for each format:**
- Function name: `test_llm_mode3_{format}`
- Backend type: `EpubBackend` → `ZipBackend`, etc.
- File path: Correct test file
- InputFormat enum: `InputFormat::Epub` → `InputFormat::Zip`, etc.

---

## ACCEPTANCE CRITERIA

**Grid is complete when:**
- [ ] All 49 formats have LLM tests
- [ ] All tests pass (quality ≥ threshold)
- [ ] Quality scores documented in grid
- [ ] This grid updated with checkmarks

**Current:** 18% complete
**Target:** 100% complete

---

## WORKER DIRECTIVE

**YOU MUST:**
1. Add Mode 3 test for Archives (4 tests) - N+1 to N+4
2. Add Mode 3 test for Email (4 tests) - N+5 to N+8
3. Add Mode 3 test for Ebooks (3 tests) - N+9 to N+11
4. Continue systematically through all 32 formats
5. Update THIS GRID as you go (check off [x])
6. Document quality scores
7. Commit updated grid every 5 tests

**Estimated:** 15-20 commits to complete all Mode 3 tests

**Start with Archives (ZIP test first), then continue through the grid.**

---

**WORKER: FILL OUT THIS GRID. ADD ALL 32 MODE 3 TESTS. THIS IS YOUR PRIORITY AFTER PYTHON ELIMINATION.**
