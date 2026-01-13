# Test Files

**8 test files containing 943 tests across 452 PDFs.**

---

## Test Files

### test_001_smoke.py (16 tests, 30 seconds)
**Purpose:** Quick sanity check before every commit

**PDFs:** 5 diverse (100p text-heavy, 39p academic, 147p Japanese, 116p CommonCrawl, 50p web)

**Tests:**
- `test_text_1worker_doesnt_crash` - 1-worker extraction (5 PDFs)
- `test_text_4workers_matches_1worker` - 4w vs 1w comparison (5 PDFs)
- `test_image_4workers_completes` - 4-worker rendering (5 PDFs) **[CRASHES]**
- `test_prerequisites` - Verify tools/libs exist

**Run:** `pytest -m smoke`

---

### test_002_text_correctness.py (60 tests, 15 minutes)
**Purpose:** Comprehensive text correctness with upstream comparison

**PDFs:** 60 curated (large, medium, small from various sources)

**Tests:**
- `test_text_1worker_vs_4worker` - Three-way validation:
  1. 1-worker vs upstream baseline
  2. 4-worker vs upstream baseline
  3. 1-worker vs 4-worker (internal consistency)

**Validation:**
- Edit distance (Levenshtein)
- Similarity ratio
- LLM error analysis (if --llm and differences exist)

**Run:** `pytest -m full`

---

### test_003_extended_corpus.py (196 tests, 1 hour)
**Purpose:** Validate all 196 benchmark PDFs

**PDFs:** All PDFs in pdfs/benchmark/

**Tests:**
- `test_extended_text_correctness` - 1w vs 4w on all 196 PDFs

**Run:** `pytest -m extended`

---

### test_004_edge_cases.py (512 tests, 30 minutes)
**Purpose:** Crash testing on unusual/malformed PDFs

**PDFs:** All 256 PDFs in pdfs/edge_cases/

**Tests:**
- `test_edge_case_text_no_crash` - Text extraction must not crash (256 PDFs)
- `test_edge_case_image_no_crash` - Rendering must not crash (256 PDFs) **[CRASHES]**

**Success criteria:** No crashes or hangs (extraction may fail, that's OK)

**Run:** `pytest -m edge_cases`

---

### test_005_image_correctness.py (196 tests, 1 hour) **[BROKEN]**
**Purpose:** Validate image rendering vs upstream

**PDFs:** All 196 benchmark PDFs

**Tests:**
- `test_image_rendering_correctness` - 4-worker vs upstream baseline

**Status:** ⚠️ ALL CRASH - parallel_render tool has SIGSEGV

**Run:** `pytest -m 'extended and image'` (will fail)

---

### test_006_determinism.py (10 tests, 5 minutes)
**Purpose:** Detect non-deterministic behavior

**PDFs:** 5 diverse PDFs

**Tests:**
- `test_text_determinism_multirun` - Run N times, verify identical output (5 PDFs)
- `test_image_determinism_multirun` - Image determinism (5 PDFs) **[CRASHES]**

**Run:** `pytest -m stability --iterations 10`

---

### test_007_performance.py (6 tests, 2 minutes)
**Purpose:** Validate speedup requirements

**PDFs:** 3 large PDFs (100p, 116p, 821p)

**Tests:**
- `test_text_speedup_requirement` - Assert 4w/1w >= 2.0x (3 PDFs)
- `test_image_speedup_requirement` - Assert 4w/1w >= 3.0x (3 PDFs) **[CRASHES]**

**Requirements:** Per CLAUDE.md
- Text: >= 2.0x speedup at 4 workers
- Image: >= 3.0x speedup at 4 workers

**Run:** `pytest -m performance`

---

### test_008_scaling.py (6 tests, 5 minutes)
**Purpose:** Worker scaling analysis (1/2/4/8 workers)

**PDFs:** 3 large PDFs

**Tests:**
- `test_text_worker_scaling` - Test 1/2/4/8 workers, verify 4w/1w >= 2.0x (3 PDFs)
- `test_image_worker_scaling` - Image scaling 1/2/4/8 workers (3 PDFs) **[CRASHES]**

**Captures:**
- Absolute performance (perf_1w_pps, pages_per_sec)
- Scaling multipliers (speedup_vs_1w, perf_2w_speedup, perf_8w_speedup)

**Run:** `pytest -m scaling`

---

## Summary

| File | Tests | PDFs | Status | Duration |
|------|-------|------|--------|----------|
| test_001_smoke | 16 | 5 | 11 work, 5 crash | 30s |
| test_002_text_correctness | 60 | 60 | ✅ Working | 15m |
| test_003_extended_corpus | 196 | 196 | ✅ Working | 1h |
| test_004_edge_cases | 512 | 256 | 256 work, 256 crash | 30m |
| test_005_image_correctness | 196 | 196 | ❌ All crash | - |
| test_006_determinism | 10 | 5 | 5 work, 5 crash | 5m |
| test_007_performance | 6 | 3 | 3 work, 3 crash | 2m |
| test_008_scaling | 6 | 3 | 3 work, 3 crash | 5m |
| **TOTAL** | **943** | **452** | **532 working** | |

**Working:** 532 text tests
**Broken:** 212 image tests (parallel_render segfaults)
