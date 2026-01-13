# PDF Pipeline Fix Roadmap

**Created:** 2025-12-05 by MANAGER
**Total Issues:** 120 (20 from TABLE_OCR + 100 from FRAMEWORK)
**Deduplicated:** ~105 unique issues (some overlap)

---

## OVERALL STATUS (Updated N=2385)

| Phase | Status | Notes |
|-------|--------|-------|
| Phase 1A | ✅ COMPLETE | Table cell data issues fixed |
| Phase 1B | ✅ COMPLETE | Preprocessing issues fixed |
| Phase 2 | ✅ ANALYZED | All issues BY DESIGN or NOT USED - no bugs |
| Phase 3A | ✅ COMPLETE | Error handling (F1, F3, F9, F10, F12, F15) - ALL DONE |
| Phase 3B | ✅ ANALYZED | F2, F14 NOT ISSUES (N=2368), F5 is LOW |
| Phase 3C | ✅ ANALYZED | F8 NOT AN ISSUE (N=2368), rest are LOW |
| Phase 3D | ✅ ANALYZED | F7 BY DESIGN (N=2368), F19 MEDIUM, rest LOW |
| Phase 4 | ✅ ANALYZED | Most issues NOT APPLICABLE or BY DESIGN |
| Phase 5 | ✅ F58 DONE | ONNX tables implemented (table_structure_onnx.rs) |
| Phase 6-7 | ✅ ANALYZED | F81 BY DESIGN (N=2382), rest are LOW/MEDIUM enhancements |
| Phase 8 | ✅ VERIFIED | F89, F90, F91 all already implemented |
| Phase 9 | ✅ F100 DONE | Documentation updated (N=2366) |

**All 28 PDF canonical tests passing (100%)**
**All 6/6 success criteria complete**

---

## PROGRESS TRACKER

### Completed (Worker Commits 93f9926e, fa8199e3, a1f1f976, 62ec6daf)
| ID | File | Issue | Status |
|----|------|-------|--------|
| T1 | table_inference.rs:276 | num_rows off by 1 | ✅ FIXED |
| T2 | table_inference.rs:277-285 | num_cols first row only | ✅ FIXED |
| T3 | table_inference.rs:243 | page_no hardcoded to 0 | ✅ FIXED |
| T5 | table_inference.rs:230-239 | Row/col spans always 1 | ✅ FIXED (fa8199e3) |
| T6 | table_inference.rs:214-223 | Header flags parsed but dropped | ✅ FIXED (ac3233c8) |
| T7 | table_inference.rs:189-205 | OCR text matching no ordering/IoU | ✅ FIXED (fa8199e3) |
| F13 | executor.rs | page_no propagation | ✅ FIXED |
| F27 | - | find_matching_ocr_text lacks IoU | ✅ FIXED (fa8199e3) |
| F32 | - | Header/row_section not emitted | ✅ FIXED (ac3233c8) |
| T11 | table_inference.rs:206-223 | No bounds check on cell assignment | ✅ FIXED (2c8b0388) |
| T9 | table_inference.rs:115-132 | Crop no guard for inverted bbox | ✅ FIXED (this commit) |
| F25 | - | No zero-sized crop check | ✅ FIXED (this commit) |
| F33 | table_inference.rs | num_rows from nl count | ✅ FIXED |
| F34 | table_inference.rs | num_cols from first nl | ✅ FIXED |

**Total Fixed: 14 issues (11 unique, 3 duplicates)**

**Verified:** 2025-12-05 04:02 - All 28 PDF canonical tests passing

---

## PHASE 1: CRITICAL - Table Output (17 issues)

### 1A: table_inference.rs - Cell Data (HIGH)
| ID | Line | Issue | Priority |
|----|------|-------|----------|
| T4 | 247-259 | Cell bbox not scaled by TABLE_SCALE | **DEFERRED** (both cell/OCR bboxes in same coord space - working correctly) |
| T5 | 230-239 | Row/col spans always 1 | ✅ FIXED |
| T6 | 214-223 | Header flags parsed but dropped | ✅ FIXED |
| T7 | 189-205 | OCR text matching no ordering/IoU | ✅ FIXED |
| T11 | 206-223 | No bounds check on cell assignment | ✅ FIXED |
| F27 | - | find_matching_ocr_text lacks IoU | ✅ FIXED |
| F28 | - | No fallback text for empty OCR | ✅ BY DESIGN - empty cells are valid (cell has no text) |
| F31 | - | No confidence scores for cells | LOW |
| F32 | - | Header/row_section not emitted | ✅ FIXED

### 1B: table_inference.rs - Preprocessing/Scale (MEDIUM)
| ID | Line | Issue | Priority |
|----|------|-------|----------|
| T8 | 137-162 | Preprocess W/H swap + normalization | ✅ VERIFIED (line 223 permute) |
| T9 | 115-132 | Crop no guard for inverted bbox | ✅ FIXED |
| T10 | 93 | TABLE_SCALE fixed at 2.0 | MEDIUM |
| T12 | 288-293 | table_map keys by cluster id only | ✅ NOT AN ISSUE - table_map is page-scoped, IDs don't need global uniqueness |
| T13 | 333-336 | otsl_seq length divergence | LOW |
| F24 | - | TABLE_SCALE not configurable | MEDIUM |
| F25 | - | No zero-sized crop check | ✅ FIXED |
| F26 | - | Preprocess lacks antialias | LOW |

---

## PHASE 2: convert.rs - ANALYSIS COMPLETE (N=2365)

**Summary:** All Phase 2 issues analyzed - most are intentional design or defensive code.

| ID | Line | Issue | Status |
|----|------|-------|--------|
| T14 | 239-262 | unwrap_or(0) masks missing indices | ✅ BY DESIGN - defaults for optional fields |
| T15 | 247-262 | Spanned cells replicated | ✅ BY DESIGN - matches Python behavior (comment confirms) |
| T16 | 320-353 | Container → empty Text | ✅ BY DESIGN - containers not produced by current pipeline |
| T17 | 354-396 | cluster_to_doc_item empty grids | ✅ NOT USED - function only in tests, main pipeline uses table_element_to_doc_item |
| F37 | - | No num_rows/num_cols validation | LOW - grid dimensions implicit from data |
| F38 | - | unwrap_or(0) masks bugs | ✅ BY DESIGN - same as T14 |
| F39 | - | Spanned cells duplicated | ✅ BY DESIGN - same as T15 |
| F40 | - | Container → empty Text | ✅ BY DESIGN - same as T16 |
| F41 | - | cluster_to_doc_item empty | ✅ NOT USED - same as T17 |
| F43 | - | No markdown escaping | ✅ BY DESIGN - matches Python (no escaping in tables) |
| F45 | - | SectionHeader level hardcoded | ✅ BY DESIGN - Python also uses level 1 (subtitle-level-1) |

---

## PHASE 3: MEDIUM - executor.rs (23 issues)

### 3A: Error Handling
| ID | Issue | Priority |
|----|-------|----------|
| F1 | No fast-fail when TableFormer missing | ✅ FIXED (N=2364) |
| F3 | No guard for RapidOCR models | ✅ FIXED (N=2364) |
| F9 | Error messages don't include paths | ✅ FIXED (N=2364) |
| F10 | pytorch feature options no runtime warning | ✅ FIXED (N=2366) |
| F12 | No backoff to ONNX on PyTorch fail | ✅ FIXED (N=2366) |
| F15 | Missing RapidOCR → runtime error not config | ✅ ADDRESSED (same as F3 - config-time check) |

### 3B: Configuration
| ID | Issue | Priority |
|----|-------|----------|
| F2 | table_structure_enabled default unclear | ✅ NOT AN ISSUE - docs clearly state "Tables enabled" for new() |
| F5 | Reading order uses default config only | LOW |
| F14 | Config mismatch tables enabled | ✅ NOT AN ISSUE - presets (minimal/fast) explicitly disable tables, documented |

### 3C: Performance/Features
| ID | Issue | Priority |
|----|-------|----------|
| F4 | Batch processing layout only | LOW |
| F8 | CUDA check doesn't verify libtorch build | ✅ NOT AN ISSUE - tch::Cuda::is_available() checks libtorch CUDA support |
| F11 | process_pages_batch discards OCR | LOW |
| F16 | No MPS (Apple GPU) handling | LOW |
| F18 | Clones textline_cells (expensive) | LOW |
| F20 | modular_pipeline created per instance | LOW |

### 3D: Validation/Logging
| ID | Issue | Priority |
|----|-------|----------|
| F6 | Profiling doesn't capture OCR/table | LOW |
| F7 | OCR disabled + empty cells skips pipeline | ✅ BY DESIGN - comment at line 1676 explains why (N=629) |
| F17 | convert_textline_coords runs when OCR skipped | LOW |
| F19 | No page_width/height validation | ✅ FIXED (N=2376) |
| F21 | profiling_enabled not in CLI | LOW |
| F22 | code_formula after assembly failure | LOW |
| F23 | Logging debug-level only | LOW |

---

## PHASE 4: markdown.rs - ANALYSIS COMPLETE (N=2365)

**Summary:** Most issues not applicable or intentionally match Python behavior.

| ID | Issue | Status |
|----|-------|--------|
| F48 | Early return on empty grid, no warning | ✅ BY DESIGN - empty grids can't render to markdown |
| F49 | No handling of row/col spans | ✅ NOT APPLICABLE - Markdown doesn't support spans, cells replicated in convert.rs |
| F50 | Headers not rendered differently | ✅ IMPLEMENTED - First row is header with separator line |
| F51 | RTL content not handled | LOW - RTL PDFs pass tests (3 RTL tests in suite) |
| F52 | No column width/justification | ✅ IMPLEMENTED - col_widths calculated, numeric columns right-aligned |
| F53 | Table text not escaped | ✅ BY DESIGN - Python doesn't escape either (verified in code) |
| F54 | No large table fallback | LOW - would need spec for fallback format |

---

## PHASE 5: LOW - Models (12 issues)

### layout_predictor (3)
| F55 | ONNX/PyTorch output types diverge | ✅ NOT AN ISSUE - numerical divergence within 1e-3 tolerance (logits 4.05e-4, boxes 2.99e-5), validated in tests |
| F56 | Thresholds hardcoded | LOW |
| F57 | Batch inference not used | LOW |

### table_structure (3)
| F58 | No ONNX backend for tables | ✅ IMPLEMENTED (table_structure_onnx.rs) |
| F59 | No checksum validation | LOW |
| F60 | HF cache only, no offline config | LOW |

### ocr (3)
| F61 | No asset presence check | ✅ ALREADY FIXED (N=2364, same as F3) - RapidOCR models validated at config-time |
| F62 | No version check | LOW |
| F63 | No language selection | LOW |

### pipeline/mod.rs (3)
| F64 | Device enum stub mismatch | LOW |
| F65 | Default config doesn't match Python | ✅ NOT AN ISSUE - Rust defaults match Python: OCR=true, table_structure=true, code_formula=false |
| F66 | No validation of dependent features | ✅ NOT AN ISSUE - Features are independent (OCR, tables, code/formula work independently) |

---

## PHASE 6: LOW - Other Pipeline (9 issues)

### layout_postprocessor.rs (3)
| F67 | Empty cells skip silently | ✅ BY DESIGN - empty clusters removed, orphans created from unassigned cells |
| F68 | No logging when clusters dropped | LOW |
| F69 | No image size assertion | LOW |

### page_assembly.rs (3)
| F70 | No reading order verification | LOW |
| F71 | No text span merging | ✅ IMPLEMENTED (pdf.rs:556-752) - merge_horizontal_cells() ports Python pypdfium2_backend.py:158-251 |
| F72 | Table-OCR linkage lost | LOW |

### docling_export.rs (2)
| F73 | Minimal validation | LOW |
| F74 | No table headers in DoclingDocument | ✅ ALREADY FIXED (T6/F32) - column_header/row_header exported at line 599-600 |

### preprocessing (4)
| F75 | Assumes 0-255 range | LOW |
| F76 | No grayscale/4-channel handling | LOW |
| F77 | No document-level rotation correction | LOW - AngleNet handles text-line rotation (0°/180°). Document-level (90°/270°) would require Tesseract OSD or new model. Python only does this via Tesseract. |
| F78 | No DPI estimation | LOW - TABLE_SCALE now configurable (N=2392). Auto-estimation would require image analysis heuristics. |

---

## PHASE 7: LOW - Backend/CLI (9 issues)

### pdf.rs (5)
| F79 | pdfium not verified | LOW |
| F80 | No runtime backend selection | ✅ IMPLEMENTED - CLI `--ml-backend` flag selects PyTorch/ONNX at runtime |
| F81 | ML errors not surfaced | ✅ BY DESIGN - Graceful degradation: errors logged as warnings, doc produced with available content |
| F82 | No streaming for large PDFs | LOW |
| F83 | DocItems not in canonical tests | ✅ FIXED (N=2372-2374) - JSON comparison module added |

### CLI (3)
| F84 | Help marks PyTorch default | ✅ FIXED (N=2428) - Help now explains feature requirements and fallback |
| F85 | No intermediate stage dump | LOW |
| F86 | No force-table-disable flag | ✅ IMPLEMENTED - CLI `--no-tables` flag disables table parsing |

---

## PHASE 8: LOW - Tests (8 issues)

| F87 | Critical tests #[ignore] | ✅ NOT AN ISSUE - ignored tests require external deps or have known PyTorch issues |
| F88 | OCR tests don't assert assets | LOW - test_rapidocr_loading validates model loading |
| F89 | No table markdown golden tests | ✅ VERIFIED - 7 PDF table tests in canonical suite |
| F90 | No multi-page integration test | ✅ VERIFIED - multi_page.pdf + large PDFs in suite |
| F91 | No RTL PDF fixtures | ✅ VERIFIED - right_to_left_01/02/03.pdf tests |
| F92 | No performance smoke test | LOW |
| F93 | No negative tests for missing models | ✅ HANDLED - executor.rs has actionable "model not found" messages |
| F94 | No grayscale/4-channel coverage | LOW |

---

## PHASE 9: LOW - Build/Docs (6 issues)

| T18/F97 | build.rs hardcoded rpath | LOW |
| T19/F95 | setup_env.sh hardcoded paths | LOW |
| T20 | ONNX lacks table warning | ✅ RESOLVED - F58 implemented ONNX tables |
| F96 | LIBTORCH_BYPASS_VERSION_CHECK | LOW |
| F98-99 | README offline/table docs | LOW |
| F100 | Status docs say PDF experimental | ✅ FIXED (N=2366) |

---

## SYSTEMATIC EXECUTION ORDER

```
Worker should follow this order:

1. PHASE 1A: T4, T5, T6, T7 (cell data issues)
2. PHASE 1B: T8, T9, T10 (preprocessing issues)
3. PHASE 2: T14, T15, T16, T17 (convert.rs)
4. PHASE 4: F48, F49, F50 (markdown.rs tables)
5. PHASE 3A: F1, F3, F9 (error handling)
6. PHASE 5: F58 (ONNX tables)
7. PHASE 8: F89, F90 (tests)
8. Remaining by priority
```

---

## SUCCESS CRITERIA

- [x] All 28 PDF tests at 0% difference ✅ (verified 2363)
- [x] Tables render with correct row/col counts ✅ (T1, T2 fixed)
- [x] Merged cells handled properly ✅ (T5 - spans calculated from OTSL)
- [x] Headers styled correctly ✅ (T6 - column_header/row_header emitted)
- [x] No silent failures ✅ (T11, T9, F25 - bounds checking added)
- [x] Error messages actionable ✅ (F1, F3, F9 fixed - N=2364)

---

## WORKER INSTRUCTIONS

After each fix:
1. Run `cargo test test_canon_pdf -- --test-threads=1`
2. Check diff against baselines
3. Commit with issue ID reference
4. Mark issue as ✅ in this file
