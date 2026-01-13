# BASELINE REGENERATION REQUIRED - All 452 PDFs

**Date**: 2025-11-22
**Discovered By**: WORKER0 N=194
**Status**: URGENT - All image tests failing due to baseline mismatch

## Problem

ALL 452 image rendering tests are failing because PPM baselines no longer match current rendering output.

**Evidence**:
- Current rendering: `24a8b7cf9cd0e99b3902bb684047d606` (hello_world.pdf page 0)
- Baseline MD5: `f09aa45bbe70022928c377dc8c276407` (hello_world.pdf page 0)
- **Binary is deterministic**: Multiple runs with same params produce same MD5

## Root Cause

The rendering logic changed between:
1. **When baselines were created** (v1.3.0, commit b853bced19)
2. **Current state** (feature/v1.7.0-implementation)

Possible causes:
- Code changes to rendering engine (anti-aliasing, blending, color space)
- Compiler/optimization level changes
- Library updates (AGG, FreeType, libjpeg, etc.)
- Platform/SDK changes

## Verification Steps Performed

```bash
# Manual rendering with test parameters
out/Release/pdfium_cli --workers 1 --threads 1 --quality balanced --ppm \
  render-pages pdfs/edge_cases/hello_world.pdf /tmp/debug/

# Result: MD5 = 24a8b7cf9cd0e99b3902bb684047d606
# ✓ Matches test run (deterministic)
# ✗ Does NOT match baseline (f09aa45bbe70022928c377dc8c276407)
```

## Solution: Regenerate All Baselines

### Command

```bash
cd integration_tests
./generate_baselines.sh --format ppm --upstream
```

### What This Does

1. Renders all 452 PDFs using upstream pdfium_test binary
2. Computes MD5 for each page
3. Updates all `baselines/upstream/images_ppm/*.json` files

### Duration

- Estimated: ~2 hours (based on PDF corpus size)
- Progress logged to console

### Risks

**HIGH RISK**: This replaces ALL baselines. If something is wrong with:
- The binary
- The rendering logic
- The generation script

...then we lose the ability to detect rendering regressions.

### Mitigation Strategy

**BEFORE regenerating**:
1. **Verify upstream pdfium_test works correctly**:
   ```bash
   # Build upstream pdfium from scratch
   cd ~/pdfium-upstream/
   git pull
   gclient sync
   ninja -C out/Release pdfium_test

   # Test on hello_world.pdf
   out/Release/pdfium_test --ppm --scale=4.166666 \
     ~/pdfium_fast/integration_tests/pdfs/edge_cases/hello_world.pdf \
     /tmp/upstream_test/

   # Check MD5
   md5 /tmp/upstream_test/hello_world.pdf.0.ppm
   ```

2. **Compare with our binary**:
   ```bash
   # Render with our binary
   ~/pdfium_fast/out/Release/pdfium_cli --workers 1 --threads 1 --quality balanced --ppm \
     render-pages ~/pdfium_fast/integration_tests/pdfs/edge_cases/hello_world.pdf \
     /tmp/our_test/

   # Compare visually
   open /tmp/upstream_test/hello_world.pdf.0.ppm
   open /tmp/our_test/page_0000.ppm

   # Should look identical
   ```

3. **If MD5s differ between upstream and our binary**: INVESTIGATE, don't regenerate
   - This means our rendering diverged from upstream
   - Must understand why before updating baselines

4. **If MD5s match between upstream and our binary**: Safe to regenerate
   - Both produce same output
   - Old baselines are just outdated

## Alternative: Selective Regeneration

If full regeneration is too risky, regenerate one category at a time:

```bash
# Edge cases only (274 PDFs)
./generate_baselines.sh --format ppm --upstream --category edge_cases

# Then test
python3 -m pytest -k edge_cases -m image_rendering

# If pass, continue with other categories
./generate_baselines.sh --format ppm --upstream --category arxiv
./generate_baselines.sh --format ppm --upstream --category cc
# etc.
```

## Decision Tree

```
Is upstream pdfium_test MD5 same as our binary MD5?
├─ YES → Safe to regenerate all baselines
│         Rendering matches upstream, old baselines just outdated
│
└─ NO → STOP! Investigate rendering difference first
          ├─ Visual check: Do images look correct?
          │   ├─ YES → Maybe acceptable difference (e.g., anti-aliasing improvement)
          │   │         Document reason and regenerate
          │   └─ NO → Bug in our rendering, must fix before regenerating
          │
          └─ Our rendering may have regressed
```

## Files Affected

- `integration_tests/baselines/upstream/images_ppm/*.json` (452 files)
- Total baseline updates: ~10,000-15,000 page MD5s (depends on PDF page counts)

## Next AI Action

**STOP**: Do NOT regenerate baselines yet.

**FIRST**: Verify upstream vs our binary as described above.

**THEN**: If safe, regenerate baselines.

**FINALLY**: Run full test suite to verify 100% pass rate.

## References

- N=194 investigation: reports/feature__v1.7.0-implementation/N194_CRITICAL_N193_FIX_WAS_WRONG_2025-11-22-02-32.md
- Baseline generation script: integration_tests/generate_baselines.sh
- Test that revealed issue: tests/pdfs/edge_cases/test_hello_world.py::test_image_rendering_hello_world
