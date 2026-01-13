# MANAGER DECISION: GPU Path Forward + Streaming Documentation

**Date:** 2025-11-20
**To:** WORKER0 (N=8+)
**Priority:** CRITICAL DECISION

---

## USER QUESTIONS ANSWERED

### Q1: Why is GPU slower (0.71x)?

**Answer:** Current GPU implementation is POST-PROCESSING, not ACCELERATION.

**What happens:**
```
1. CPU renders page fully using AGG rasterizer (90% of work)
2. Upload CPU-rendered bitmap to GPU (5% overhead)
3. GPU shader does trivial resample (identity operation, 20% overhead)
4. Download result from GPU (20% overhead)
5. Write to disk (5%)

Total: 140% work vs 100% CPU-only = 0.71x slower
```

**The GPU shader:**
```metal
return tex.sample(s, in.texCoord);  // Just reads texture, no computation
```

This is not acceleration - it's adding overhead to already-complete work.

---

### Q2: Is real GPU acceleration impossible or just needs more work?

**Answer:** NOT impossible - needs **Skia GPU backend** (major refactor, but doable)

**Three paths identified:**

| Option | Approach | Gain | Effort | Risk |
|--------|----------|------|--------|------|
| **A: Skia GPU** | Replace AGG with Skia+Metal | 3-8x | 20-30 commits | Medium |
| **B: MPS Hybrid** | GPU for specific ops only | 1.3-2x | 8-12 commits | Low |
| **C: Custom Metal** | Write GPU PDF rasterizer | 5-15x | 50-80 commits | High |

**Option A (Skia GPU) is realistic and proven:**
- Skia has mature Metal backend
- PDFium already supports Skia (PDF_USE_SKIA flag exists)
- Other projects use Skia GPU successfully (Chromium, Flutter)
- Expected: 3-8x on image-heavy PDFs, 1.5-3x on text

---

### Q3: Why keep current GPU if it's slower?

**Two reasons:**

1. **Infrastructure reuse:** Metal device, command queue, batch logic reusable for Skia
2. **Honest engineering:** Document what works/doesn't (0.71x measured, not hidden)

But you're right - if it provides no value, should we remove it or fix it properly?

---

## DECISION REQUIRED FROM USER

### Option 1: DO SKIA GPU NOW (Recommended if you want GPU)

**WORKER0 pivots to Skia GPU implementation:**
- Abandon current post-processing GPU
- Enable PDF_USE_SKIA build flag
- Integrate Skia Metal backend
- Validate correctness (all 2,780 tests must pass)
- **Expected: 3-8x real GPU acceleration**
- **Effort: 20-30 commits (~10-15 hours)**

**Then continue Phases 3-5:**
- Phase 3: Binaries (5-8 commits)
- Phase 4: Python (8-10 commits)
- Phase 5: Validation (5-8 commits)

**Total v1.7.0: ~45-65 commits**

### Option 2: REMOVE GPU, FOCUS ON USER FEATURES (Faster to ship)

**Skip GPU entirely:**
- Remove current GPU code (slower, no value)
- Focus on HIGH VALUE features:
  - Phase 3: Linux binaries (4 commits)
  - Phase 4: Python bindings (8-10 commits) ⭐ HIGHEST USER VALUE
  - Phase 5: Cross-platform tests (5-8 commits)

**Total v1.7.0: ~17-22 commits (~8-11 hours)**

**Ship v1.7.0 FAST, do GPU in v1.8.0 when more time**

### Option 3: HYBRID (Middle ground)

**Keep current GPU as experimental:**
- Document honestly (0.71x, not recommended)
- Add note: "GPU acceleration coming in v1.8.0 with Skia"
- Focus on Phases 3-5 (user features)

**Do Skia GPU in v1.8.0 separately**

---

## STREAMING DOCUMENTATION (USER IS CORRECT)

**Worker analyzed but didn't document!** You're right - need to:

1. **Add streaming tests** to smoke test suite
2. **Document in README** that streaming is built-in
3. **Add memory tests** (verify <100MB for large PDFs)

**This is 2-3 commits of work (should be done).**

---

## MY RECOMMENDATION TO USER

**Do Option 2: Focus on Python Bindings + Binaries**

**Why:**
1. **Python bindings** = HIGHEST user value (library integration)
2. **Linux binaries** = Remove build barrier (accessibility)
3. **Ship v1.7.0 FAST** = Users get value sooner
4. **GPU in v1.8.0** = Do Skia GPU properly when ready

**Skia GPU is doable (20-30 commits) but delays v1.7.0 by ~2 weeks.**

Better to ship Python + binaries now (huge value), then GPU in v1.8.0.

---

## REVISED v1.7.0 PLAN

**If you choose Option 2:**

**Phase 2 (Streaming):** Add tests + docs (2-3 commits)
**Phase 3 (Linux Binaries):** Docker build (4 commits)
**Phase 4 (Python):** pip install support (8-10 commits) ⭐
**Phase 5 (Validation):** Cross-platform tests (5-8 commits)
**Remove GPU:** Delete experimental code (1 commit)

**Total: 20-26 commits (~10-13 hours) = Ship v1.7.0 in 1-2 days**

---

## WORKER DIRECTIVE PENDING YOUR DECISION

**What should worker do next?**

A. Pivot to Skia GPU (20-30 commits, real 3-8x acceleration)
B. Remove GPU, focus on Python+binaries (20-26 commits, fast ship)
C. Keep GPU experimental, continue with binaries+Python

**Which path?**
