# Comprehensive Testing Strategy - ALL Formats

**Date:** 2025-11-11
**Manager:** N=250
**Principle:** Every format must have tests + validation, even without Python ground truth

---

## Testing Tiers

### Tier 1: Formats with Python Docling Ground Truth ✅

**Method:** String comparison + LLM semantic validation

**Formats (from Python docling v2.58.0):**
- PDF (out of scope)
- DOCX, PPTX, XLSX
- HTML, CSV, Markdown, AsciiDoc
- JATS
- WebVTT
- PNG, JPEG, TIFF, WEBP (images with OCR)

**Testing approach:**
```rust
#[test]
fn test_canon_csv_comma() {
    let result = parse("test.csv");
    let expected = load_python_baseline("expected.md");
    assert_eq!(normalize(result.markdown), normalize(expected));  // String match
}

#[tokio::test]
#[ignore]
async fn test_llm_verification_csv() {
    let result = parse("test.csv");
    let expected = load_python_baseline("expected.md");
    let quality = llm_verifier.compare_outputs(&expected, &result.markdown);
    assert!(quality.score >= 0.85);  // Semantic validation
}
```

**Status:** ✅ LLM tests implemented for all (N=249-253)

---

### Tier 2: Formats WITHOUT Python Ground Truth (Need LLM Validation)

**These formats extend beyond Python docling scope. No authoritative baseline.**

**Method:** Dual validation approach

#### 2A. Self-Consistency Tests

**Verify basic functionality:**
```rust
#[test]
fn test_epub_basic() {
    let backend = EpubBackend::new();
    let result = backend.parse_file("test.epub").unwrap();

    // Basic assertions
    assert!(result.content_blocks.is_some(), "Must generate DocItems");
    assert!(!result.markdown.is_empty(), "Must have content");
    assert!(result.metadata.title.is_some(), "Should extract title");

    // Structure checks
    let doc_items = result.content_blocks.unwrap();
    assert!(doc_items.len() > 0, "Must have DocItems");

    // JSON export works
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("\"label\""), "JSON must have labels");
}
```

#### 2B. LLM Baseline Validation

**Use LLM to validate against source document:**
```rust
#[tokio::test]
#[ignore]
async fn test_llm_baseline_epub() {
    let verifier = LLMQualityVerifier::from_env().unwrap();

    // Parse with Rust
    let result = parse("test.epub");

    // Generate baseline from source (one-time)
    // Option 1: Use mode 1 (establish_baseline)
    let baseline = verifier.establish_baseline(
        "test.epub",
        InputFormat::Epub
    ).await.unwrap();

    // Option 2: Use mode 3 (standalone verification)
    let quality = verifier.verify_standalone(
        "test.epub",
        &result.markdown,
        InputFormat::Epub
    ).await.unwrap();

    // LLM checks: Does output accurately represent input?
    assert!(quality.score >= 0.75,  // Lower threshold without ground truth
        "EPUB quality: {:.1}%",
        quality.score * 100.0
    );
}
```

#### 2C. Round-Trip Testing

**For formats that can be regenerated:**
```rust
#[test]
fn test_json_roundtrip() {
    let result1 = parse("test.json");

    // Export to JSON
    let json = serde_json::to_string(&result1).unwrap();

    // Re-import
    let result2: Document = serde_json::from_str(&json).unwrap();

    // Should be identical
    assert_eq!(result1, result2, "Round-trip must preserve all data");
}
```

**Formats this applies to:**
- Archives (ZIP, TAR, 7Z, RAR)
- Email (EML, MBOX, VCF, MSG)
- Ebooks (EPUB, FB2, MOBI)
- Calendar (ICS)
- Notebooks (IPYNB)
- GPS (GPX, KML)
- OpenDocument (ODT, ODS, ODP)
- Subtitles (SRT)

---

### Tier 3: Formats with Visual/Binary Content

**Testing approach for formats with visual elements:**

#### Method 1: Metadata Extraction Validation
```rust
#[test]
fn test_image_metadata_bmp() {
    let backend = BmpBackend::new();
    let result = backend.parse_file("test.bmp").unwrap();

    // Check metadata extracted
    assert!(result.metadata.num_characters > 0);

    // Check DocItems
    let items = result.content_blocks.unwrap();
    assert!(matches!(items[0], DocItem::Picture { .. }));

    // Check image dimensions present in markdown
    assert!(result.markdown.contains("width") || result.markdown.contains("×"));
}
```

#### Method 2: OCR Output Validation (with LLM)
```rust
#[tokio::test]
#[ignore]
async fn test_llm_ocr_png() {
    let verifier = LLMQualityVerifier::from_env().unwrap();

    // Parse with OCR
    let result = parse_with_ocr("test.png");

    // LLM validates: Does OCR text make sense for this image?
    let quality = verifier.verify_standalone(
        "test.png",
        &result.markdown,
        InputFormat::Png
    ).await.unwrap();

    assert!(quality.score >= 0.70,  // OCR is imperfect
        "PNG OCR quality: {:.1}%",
        quality.score * 100.0
    );
}
```

**Formats:**
- Images: BMP, GIF, HEIF, AVIF
- CAD: STL, OBJ, GLTF, GLB, DXF
- Medical: DICOM
- Graphics: SVG, XPS

---

## Comprehensive Test Matrix

### Formats with Ground Truth (15 formats)

| Format | String Tests | LLM Tests | Status |
|--------|--------------|-----------|--------|
| DOCX | ✅ 14 | ✅ 1 | LLM @ N=251-253 |
| HTML | ✅ 24 | ✅ 1 | LLM @ N=249 |
| CSV | ✅ 8 | ✅ 1 | LLM @ N=249, 100% |
| Markdown | ✅ 9 | ✅ 1 | LLM @ N=249 |
| AsciiDoc | ✅ 3 | ✅ 1 | LLM @ N=249 |
| PPTX | ✅ 3 | ✅ 1 | LLM @ N=251-253 |
| XLSX | ✅ 3 | ✅ 1 | LLM @ N=249 |
| WebVTT | ✅ 3 | ✅ 1 | LLM @ N=251-253 |
| JATS | ✅ 5 | ⏳ TODO | Worker must add |
| PNG | ✅ 4 (Python) | ⏳ TODO | Need Rust + OCR first |
| JPEG | ✅ 4 (Python) | ⏳ TODO | Need Rust + OCR first |
| TIFF | ✅ 4 (Python) | ⏳ TODO | Need Rust + OCR first |
| WEBP | ✅ 1 (Python) | ⏳ TODO | Need Rust + OCR first |
| PDF | ✅ 24 | ❌ OUT OF SCOPE | Do not test |

---

### Formats WITHOUT Ground Truth (Need LLM Mode 3 Validation)

| Format | Basic Tests | LLM Baseline | LLM Standalone | Status |
|--------|-------------|--------------|----------------|--------|
| **Archives (4)** |
| ZIP | ✅ 18 | ⏳ TODO | ⏳ TODO | Worker must add |
| TAR | ✅ (incl.) | ⏳ TODO | ⏳ TODO | Worker must add |
| 7Z | ✅ (incl.) | ⏳ TODO | ⏳ TODO | Worker must add |
| RAR | ✅ (incl.) | ⏳ TODO | ⏳ TODO | Worker must add |
| **Email (4)** |
| EML | ✅ 39 | ⏳ TODO | ⏳ TODO | Worker must add |
| MBOX | ✅ (incl.) | ⏳ TODO | ⏳ TODO | Worker must add |
| VCF | ✅ (incl.) | ⏳ TODO | ⏳ TODO | Worker must add |
| MSG | ✅ (incl.) | ⏳ TODO | ⏳ TODO | Worker must add |
| **Ebooks (3)** |
| EPUB | ✅ 20 | ⏳ TODO | ⏳ TODO | Worker must add |
| FB2 | ✅ (incl.) | ⏳ TODO | ⏳ TODO | Worker must add |
| MOBI | ✅ (incl.) | ⏳ TODO | ⏳ TODO | Worker must add |
| **Subtitles** |
| SRT | ✅ 5 | ⏳ TODO | ⏳ TODO | Worker must add |
| **Calendar/Notebook (2)** |
| ICS | ✅ 5 | ⏳ TODO | ⏳ TODO | Worker must add |
| IPYNB | ✅ 5 | ⏳ TODO | ⏳ TODO | Worker must add |
| **OpenDocument (3)** |
| ODT | ✅ 15 | ⏳ TODO | ⏳ TODO | Worker must add |
| ODS | ✅ (incl.) | ⏳ TODO | ⏳ TODO | Worker must add |
| ODP | ✅ (incl.) | ⏳ TODO | ⏳ TODO | Worker must add |
| **Images (4)** |
| BMP | ✅ Unit | ⏳ TODO | ⏳ TODO | Worker must add |
| GIF | ✅ Unit | ⏳ TODO | ⏳ TODO | Worker must add |
| HEIF | ✅ Unit | ⏳ TODO | ⏳ TODO | Worker must add |
| AVIF | ✅ Unit | ⏳ TODO | ⏳ TODO | Worker must add |
| **GPS (3)** |
| GPX | ✅ Unit | ⏳ TODO | ⏳ TODO | Worker must add |
| KML | ✅ Unit | ⏳ TODO | ⏳ TODO | Worker must add |
| KMZ | ✅ (incl.) | ⏳ TODO | ⏳ TODO | Worker must add |
| **Others** |
| RTF | ✅ Unit | ⏳ TODO | ⏳ TODO | Worker must add |
| SVG | ✅ Unit | ⏳ TODO | ⏳ TODO | Worker must add |
| CAD | ✅ Unit | ⏳ TODO | ⏳ TODO | Worker must add |
| DICOM | ✅ Unit | ⏳ TODO | ⏳ TODO | Worker must add |
| XPS | ⏳ TODO | ⏳ TODO | ⏳ TODO | Worker must add |
| IDML | ⏳ TODO | ⏳ TODO | ⏳ TODO | Worker must add |

**Total: ~30 formats need LLM validation tests**

---

## LLM Mode 3 (Standalone Verification) Pattern

**For formats without Python ground truth, use Mode 3:**

```rust
#[tokio::test]
#[ignore]
async fn test_llm_standalone_epub() {
    let verifier = LLMQualityVerifier::from_env().unwrap();

    // Parse EPUB with Rust
    let backend = EpubBackend::new();
    let result = backend.parse_file("test-corpus/ebooks/sample.epub", &Default::default())
        .expect("Failed to parse EPUB");

    // Verify DocItems exist
    assert!(result.content_blocks.is_some(), "Must generate DocItems");

    // LLM Mode 3: Validate output represents input document accurately
    let quality = verifier.verify_standalone(
        "test-corpus/ebooks/sample.epub",  // Original file
        &result.markdown,                   // Our output
        InputFormat::Epub
    ).await
    .expect("LLM verification failed");

    println!("=== EPUB Standalone Validation ===");
    println!("Score: {:.1}%", quality.score * 100.0);
    println!("Assessment: Does output accurately represent input?");

    for finding in &quality.findings {
        println!("  [{:?}] {}", finding.severity, finding.description);
    }

    // Lower threshold since no ground truth
    assert!(quality.score >= 0.75,
        "EPUB quality too low: {:.1}%",
        quality.score * 100.0
    );
}
```

**Key differences from Mode 2 (comparative):**
- No Python expected output
- LLM reads original file directly (vision API for images/PDFs)
- LLM assesses: "Does this output faithfully represent the input?"
- Lower threshold (0.75 vs 0.85) due to no authoritative baseline

---

## Implementation Priority

### Phase 1: Complete Mode 2 Tests (Ground Truth) - N=254-260

**Remaining formats with ground truth:**
- [ ] JATS (3 tests) - Need Rust implementation first
- [ ] PNG (1 test) - Need OCR implementation
- [ ] TIFF (1 test) - Need OCR implementation
- [ ] WEBP (1 test) - Need OCR implementation

**Estimated:** 5-6 commits after implementing the formats

---

### Phase 2: Implement Mode 3 Tests (No Ground Truth) - N=260-290

**Categories to test:**

**Archives (4 formats, ~18 existing tests):**
```rust
// Add LLM standalone validation:
test_llm_standalone_zip() - Validates file listing is complete
test_llm_standalone_tar() - Validates archive contents
test_llm_standalone_7z() - Validates structure
test_llm_standalone_rar() - Validates extraction
```

**Email (4 formats, ~39 existing tests):**
```rust
test_llm_standalone_eml() - Validates email headers + body
test_llm_standalone_mbox() - Validates message structure
test_llm_standalone_vcf() - Validates contact fields
test_llm_standalone_msg() - Validates Outlook message
```

**Ebooks (3 formats, ~20 existing tests):**
```rust
test_llm_standalone_epub() - Validates book structure, chapters, TOC
test_llm_standalone_fb2() - Validates FictionBook structure
test_llm_standalone_mobi() - Validates Kindle book
```

**OpenDocument (3 formats, ~15 existing tests):**
```rust
test_llm_standalone_odt() - Validates text document structure
test_llm_standalone_ods() - Validates spreadsheet cells
test_llm_standalone_odp() - Validates presentation slides
```

**Calendar/Notebook (2 formats, ~10 existing tests):**
```rust
test_llm_standalone_ics() - Validates calendar events
test_llm_standalone_ipynb() - Validates notebook cells
```

**Images (4 formats):**
```rust
test_llm_standalone_bmp() - Validates metadata extraction
test_llm_standalone_gif() - Validates frame info
test_llm_standalone_heif() - Validates image metadata
test_llm_standalone_avif() - Validates AV1 image
```

**Subtitles (1 format, ~5 existing tests):**
```rust
test_llm_standalone_srt() - Validates subtitle timing + text
```

**Specialized (6+ formats):**
```rust
test_llm_standalone_gpx() - Validates GPS tracks/waypoints
test_llm_standalone_kml() - Validates geospatial data
test_llm_standalone_rtf() - Validates rich text
test_llm_standalone_svg() - Validates vector graphics structure
test_llm_standalone_dicom() - Validates medical image metadata
test_llm_standalone_stl() - Validates 3D mesh metadata
```

**Estimated:** 30 tests, 10-15 commits

---

### Phase 3: Formats with NO Tests Yet - N=290-330

**Must create test files + tests:**

**Apple iWork (3 formats):**
- [ ] PAGES - Create 2 test files, add tests, LLM validate
- [ ] NUMBERS - Create 2 test files, add tests, LLM validate
- [ ] KEY - Create 2 test files, add tests, LLM validate

**MS Extended (6 formats):**
- [ ] DOC - Create 2 test files, add tests, LLM validate
- [ ] PUB - Create 2 test files, add tests, LLM validate
- [ ] VSDX - Create 2 test files, add tests, LLM validate
- [ ] MPP - Create 2 test files, add tests, LLM validate
- [ ] ONE - Create 2 test files, add tests, LLM validate (very hard)
- [ ] MDB - Create 2 test files, add tests, LLM validate

**Audio/Video (6 formats):**
- [ ] WAV - Create 2 test files, metadata validation
- [ ] MP3 - Create 2 test files, metadata validation
- [ ] MP4 - Create 2 test files, metadata validation
- [ ] MKV - Create 2 test files, metadata validation
- [ ] MOV - Create 2 test files, metadata validation
- [ ] AVI - Create 2 test files, metadata validation

**LaTeX (1 format):**
- [ ] TEX - Create 3 test files, structure validation

**Estimated:** 40-50 commits (2-3 per format)

---

## LLM Verifier Mode Reference

### Mode 1: Baseline Creation
**Use:** First-time validation, create expected output
```rust
let baseline = verifier.establish_baseline(input_file, format).await?;
// Returns: ContentBaseline with structure info
```

### Mode 2: Comparative Analysis (Current)
**Use:** Compare against Python ground truth
```rust
let quality = verifier.compare_outputs(expected, actual, format).await?;
// Returns: QualityReport with score 0.0-1.0
```

### Mode 3: Standalone Verification (NEW - Need to implement)
**Use:** Validate output against input (no baseline)
```rust
let quality = verifier.verify_standalone(input_file, output, format).await?;
// Returns: QualityReport - Does output represent input accurately?
```

**Status:** Mode 1 and Mode 3 were designed (N=224) but **not implemented yet!**

**Worker must:** Implement Mode 3 in quality-verifier crate before using for non-groundtruth formats.

---

## Quality Thresholds

**With Python ground truth (Mode 2):**
- Threshold: ≥ 0.85 (85%)
- Rationale: Authoritative baseline exists

**Without ground truth (Mode 3):**
- Threshold: ≥ 0.75 (75%)
- Rationale: No authoritative baseline, self-validation only

**OCR formats (Mode 3):**
- Threshold: ≥ 0.70 (70%)
- Rationale: OCR inherently imperfect

---

## Worker Task Breakdown

### Immediate (N=255-260)
- [ ] Implement Mode 3 (standalone verification) in quality-verifier crate
- [ ] Test Mode 3 works with one format (EPUB)
- [ ] Add LLM standalone tests for archives (4 tests)
- [ ] Add LLM standalone tests for email (4 tests)

### Short-term (N=260-290)
- [ ] Add LLM standalone tests for remaining ~22 formats
- [ ] Run all tests, document quality scores
- [ ] Fix any formats scoring below threshold
- [ ] Update FORMAT_PROCESSING_GRID.md with quality scores

### Medium-term (N=290-330)
- [ ] Create test files for untested formats (~16)
- [ ] Add basic integration tests
- [ ] Add LLM validation tests
- [ ] Verify all pass

### Ongoing (N=330+)
- [ ] Add more test files (increase coverage)
- [ ] Re-run LLM validation periodically
- [ ] Fix quality regressions
- [ ] Optimize performance
- [ ] Continue indefinitely

---

## Acceptance Criteria

**Every format must have:**
- [ ] At least 1 test file in test-corpus/
- [ ] At least 1 integration test (string comparison or self-consistency)
- [ ] At least 1 LLM validation test (Mode 2 or Mode 3)
- [ ] Quality score ≥ threshold (0.85 with ground truth, 0.75 without)
- [ ] Documented in FORMAT_PROCESSING_GRID.md

**Current:** 9/56 formats have LLM tests (16%)
**Target:** 56/56 formats have LLM tests (100%)

---

## Cost Estimate

**Mode 2 (comparative) tests:**
- Cost: ~$0.0006 per test
- Total: ~15 formats = ~$0.01

**Mode 3 (standalone) tests:**
- Cost: ~$0.001 per test (vision API for input)
- Total: ~30 formats = ~$0.03

**One-time baseline creation (Mode 1):**
- Cost: ~$0.05 per format
- Total: ~30 formats = ~$1.50

**Total setup cost:** ~$1.60 one-time
**Ongoing cost:** ~$0.05 per full test run

**Acceptable for quality assurance.**

---

## Next Worker: Implement Mode 3, Add 30 LLM Tests

**Read:**
- COMPREHENSIVE_TESTING_STRATEGY.md (this file)
- WORKER_CHECKLIST.md (track progress)

**Tasks:**
1. Implement Mode 3 (verify_standalone) in quality-verifier (N=255-256)
2. Add LLM standalone tests for all formats without ground truth (N=257-285)
3. Run all, document quality scores (N=286)
4. Update grid with scores (N=287)
5. Fix any low-scoring formats (N=288+)
6. Continue working indefinitely

**Every format must have LLM validation. No exceptions.**
