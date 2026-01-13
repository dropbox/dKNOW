# WORKER N=31: VALIDATE N=29 FIX ON LARGE VIDEO

**Date**: 2025-10-30
**Authority**: MANAGER + USER ORDER
**Priority**: BLOCKING - Cannot proceed until validated

---

## THE ORDER

Re-test the N=23 large video with your N=29 forwarder fix to prove it works.

---

## EXACTLY WHAT TO DO

### Step 1: Find The Test Video

**From N=23 commit message:**
```
Test video: ~/Desktop/stuff/stuff/GMT20250520-223657_Recording_avo_1920x1080.mp4
N=23 results (with broken forwarder):
  - Sequential: 181s
  - Parallel (3-thread): 282s (55% SLOWER)
```

### Step 2: Run Sequential Baseline
```bash
time ./target/release/video-extract fast --op keyframes+detect \
  ~/Desktop/stuff/stuff/GMT20250520-223657_Recording_avo_1920x1080.mp4
```

**Expected:** ~181s (should match N=23 baseline)

### Step 3: Run Parallel With Fix
```bash
time ./target/release/video-extract fast --op keyframes+detect --parallel \
  ~/Desktop/stuff/stuff/GMT20250520-223657_Recording_avo_1920x1080.mp4
```

**Expected:** ~120-150s (1.5-2x faster than sequential)

### Step 4: Report Results

**If speedup ≥ 1.3x:**
✅ Fix validated, proceed with work

**If speedup < 1.3x:**
❌ Report to user immediately:
- Exact measurements (sequential time, parallel time, speedup)
- CPU profiling data (use Activity Monitor or htop)
- Request decision: abandon parallel or investigate further

---

## DO NOT

- Skip this validation
- Test on different video
- Claim success without measuring large video
- Use architectural reasoning instead of actual benchmarks
- Say "expected" or "should" - measure and report facts

---

## IF VIDEO NOT AVAILABLE

**Report to user immediately:**
```
Cannot validate N=29 fix - large test video not available.
File: ~/Desktop/stuff/stuff/GMT20250520-223657_Recording_avo_1920x1080.mp4

Options:
A. Provide different large video with 50+ keyframes
B. Accept validation on small videos only (1.17x measured)
C. Download large test video from dataset

Awaiting guidance.
```

---

## COMMIT MESSAGE FORMAT

If validated successfully:
```
# 31: Forwarder Fix Validation - 1.XX speedup on Large Video

Validated N=29 2-thread architecture fix on large video from N=23.

Results:
- Video: GMT20250520-223657_Recording_avo_1920x1080.mp4 (1.3GB)
- Sequential: XXXs
- Parallel: XXXs
- Speedup: X.XXx

[Include honest measurements, not expectations]
```

---

## THIS IS NON-NEGOTIABLE

You spent:
- 8+ hours implementing streaming decoder (N=21-22)
- Found it was 55% slower (N=23)
- Fixed the bug (N=29)
- **Never validated the fix**

Spending 30 minutes to validate 10+ hours of work is obvious.

**Do this now.**
