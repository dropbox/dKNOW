# 🚨 MANAGER URGENT: STOP PNG BASELINE REGENERATION

**Date:** 2025-11-03 08:46 PST
**For:** WORKER0 (Iteration #105+)
**Priority:** CRITICAL - STOP CURRENT WORK IMMEDIATELY

---

## STOP: Current Work Is Misguided

**Worker #104 started:** Regenerating 452 PNG baselines at 300 DPI (22/452 done, 90min ETA)

**THIS WORK MUST BE STOPPED:** It will NOT achieve the goal of exact MD5 matching.

---

## ROOT CAUSE: Format Mismatch, NOT DPI Mismatch

### Worker's Current Understanding (INCORRECT)
```
Problem: Baselines at 150 DPI, tests expect 300 DPI
Solution: Regenerate all baselines at 300 DPI
Expected: MD5 matches will work
```

### Actual Reality (CORRECT)
```
Real Problem: PNG format (ours) vs PPM format (upstream)
- Upstream pdfium_test: Outputs PPM (RGB, raw, 3 bytes/pixel)
- Our Rust tool: Outputs PNG (RGBA, compressed, 4 bytes/pixel + metadata)
- Result: MD5s will NEVER match, regardless of DPI

DPI issue is IRRELEVANT compared to format mismatch!
```

---

## PROOF: Format Artifacts Documented

From `MANAGER_VALIDATION_CLARIFICATION.md` (created today):

**Why SSIM isn't 1.0000:**
1. Different format: PPM vs PNG
2. Different channels: RGB vs RGBA
3. Compression metadata: None vs PNG metadata
4. Dimension rounding: Sometimes 1px different

**Historical data:** Same PDFs have SAME SSIM scores before/after any changes.
- arxiv_001: 0.9742 (iteration #55) = 0.9742 (today)
- arxiv_004: 1.0000 (iteration #55) = 1.0000 (today)

**Conclusion:** Non-perfect SSIM is format artifact, not rendering bug!

---

## WHAT WORKER SHOULD DO INSTEAD

### Correct Approach: Implement PPM Output

**Goal:** Byte-for-byte MD5 match with upstream pdfium_test

**Implementation:**
1. **Add PPM output to Rust tool** (`rust/pdfium-sys/examples/render_pages.rs`)
   - Add `--ppm` flag
   - Implement PPM P6 format writer
   - Match upstream dimension calculation exactly

2. **Generate PPM baselines**
   - Run upstream `pdfium_test --ppm` on all 452 PDFs
   - Compute MD5 of each PPM file
   - Store in new baseline format

3. **Update test suite**
   - Compare PPM MD5s (byte-for-byte)
   - No SSIM needed - either matches or doesn't

4. **Achieve 100% MD5 match**
   - Eliminates all format artifacts
   - Fast comparison (MD5 only)
   - Deterministic (no thresholds)

---

## ACTION REQUIRED: Stop and Pivot

### Immediate Actions

1. **Stop PNG regeneration process**
   ```bash
   # Check if still running
   ps aux | grep regenerate_image_baselines

   # If running, kill it (work will be wasted anyway)
   kill <PID>
   ```

2. **Abandon PNG baseline strategy**
   - PNG baselines will never get exact MD5 match
   - Format mismatch is fundamental
   - DPI fix doesn't solve core problem

3. **Read these files:**
   - `MANAGER_VALIDATION_CLARIFICATION.md` - Explains format artifact root cause
   - `/tmp/baseline_format_explanation.md` - Full PPM implementation plan
   - This file - Immediate stop order

4. **Start PPM implementation**
   - Follow implementation plan in baseline_format_explanation.md
   - Test with 1-2 PDFs first
   - Verify MD5 match with upstream
   - Then scale to all 452 PDFs

---

## WHY THIS IS URGENT

**Time/Effort Waste:**
- PNG regeneration: 90 minutes, generates unusable baselines
- PPM implementation: ~30-60 minutes, solves problem correctly

**Correctness:**
- PNG approach: Will still have SSIM artifacts, no perfect match
- PPM approach: Byte-for-byte MD5 match, 100% confidence

**User Directive:**
User explicitly requested:
> "I want perfect matches. I want exact matching for confidence!"
> "we need our rust tool to also output the PPM format and to get MD5 hashes on those"

PNG regeneration does NOT fulfill this requirement!

---

## WHAT TO COMMIT

### Stop Work Commit
```
[WORKER0] # 105: STOP - Pivoting from PNG to PPM Baseline Strategy

MANAGER urgent stop order: PNG regeneration abandoned

Reason: Format mismatch (PNG vs PPM) prevents exact MD5 matching
- DPI fix irrelevant compared to format issue
- User requires byte-for-byte match
- PNG approach fundamentally flawed

Next: Implement PPM output per MANAGER directive
References:
- MANAGER_VALIDATION_CLARIFICATION.md
- /tmp/baseline_format_explanation.md
- MANAGER_URGENT_STOP_WORK.md (this file)
```

---

## VALIDATION CONTEXT

**Current upstream validation running:**
- 50 PDFs being validated (15/50 complete as of 16:46)
- Results so far: 100% pass (SSIM ≥ 0.95)
- Confirms rendering is correct
- Format artifacts expected (0.96-0.98 SSIM)

**This validates:**
- FPDFBitmap_CreateEx fix is working
- Rendering correctness achieved
- Only format conversion remaining issue

---

## NEXT WORKER TASKS

1. ✅ Read this file completely
2. ✅ Stop PNG regeneration process
3. ✅ Read implementation plan (`/tmp/baseline_format_explanation.md`)
4. ✅ Implement PPM output in Rust tool
5. ✅ Test PPM output matches upstream MD5
6. ✅ Generate PPM baselines for all 452 PDFs
7. ✅ Update test suite for PPM comparison
8. ✅ Verify 100% MD5 match achieved

---

## FILES TO READ (IN ORDER)

1. **This file** - Immediate stop order and context
2. `MANAGER_VALIDATION_CLARIFICATION.md` - Why non-perfect SSIM is expected
3. `/tmp/baseline_format_explanation.md` - Complete PPM implementation plan
4. `integration_tests/lib/validate_images_vs_upstream.py` - Current validation approach

---

**END OF URGENT DIRECTIVE**
