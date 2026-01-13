# [MANAGER] FINAL: Eliminate Last Skips - Clear Action Plan

**Target**: WORKER0
**Priority**: HIGH

## Current Situation (Verified at N=103)

**Smoke Tests**: 67/67 PASS (100%) ✅
**Extended Tests**: 957 PASS, 6 SKIP ⚠️
**Total Tests**: 2,819

## Remaining Skips to Fix

### 1. Large PDF Skips (6 skips) - FIRST PRIORITY

**Location**: test_005_image_correctness.py line 127-129
**Code**:
```python
MAX_PAGES_FOR_PPM = 500  # Line 127
if page_count > MAX_PAGES_FOR_PPM:
    pytest.skip(f"PDF too large...")  # Line 129
```

**PDFs Affected**: cc_001_931p, cc_002_522p, cc_006_594p, + 3 more

**Action**:
1. Verify `pytest.render_with_md5()` in conftest.py deletes PPM files after each page
2. If yes: Remove MAX_PAGES_FOR_PPM limit entirely (lines 127-129)
3. If no: Add `os.unlink(ppm_file)` after MD5 computation, then remove limit
4. Run test_005 to verify 6 large PDFs now PASS

**Expected Result**: 6 skips → 6 PASS

### 2. JSONL Skips (Verify Actual Count)

**Manager's Analysis**: 426 JSONL files exist, 423 have data in manifest
**Actual Missing**: Only ~29 PDFs without JSONL (not 158)

**Some JSONL files are EMPTY** (0 bytes):
- cc_001_931p/jsonl/page_0000.jsonl - 0 bytes
- These need regeneration

**Action**:
1. Find all 0-byte JSONL files
2. Regenerate those using extract_text_jsonl tool
3. Update manifests with correct JSONL data
4. Run JSONL tests to verify they PASS

## Success Criteria

**Target**: 0 failures, 0 skips (or < 10 valid skips with written justification)

**Priority Order**:
1. Fix large PDF skips (simple, 6 tests)
2. Fix empty JSONL files (harder, verify count first)
3. Generate any truly missing JSONLs (if any)

Focus on #1 first - should be quick win.
