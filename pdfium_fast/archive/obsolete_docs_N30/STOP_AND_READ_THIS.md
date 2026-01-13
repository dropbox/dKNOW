# STOP - CRITICAL DIRECTIVE CHANGE

**WORKER0: STOP what you're doing and read this.**

---

## USER CHANGED DECISION AFTER YOUR N=11

**You removed GPU in N=10-11** based on earlier decision (Option 2).

**USER THEN SAID:**

> "Do Path A, then Path B, then C"

**Path A = Skia GPU implementation**

**This means:**
1. User wants GPU (Skia version, not removal)
2. Your N=10-11 GPU removal was based on OLD decision
3. You must RESTORE GPU and implement Skia

---

## WHAT YOU NEED TO DO NOW (N=12)

### Step 1: Restore Metal Files

Metal files still exist in `core/fxge/apple/`:
- fx_apple_metal.h
- fx_apple_metal.mm
- metal_shaders.metal

These are good foundation. Keep them.

### Step 2: Restore GPU in CLI

You removed GPU from `examples/pdfium_cli.cpp` in N=11.

**DO NOT just add it back** - that was post-processing (0.71x).

**Instead: Enable Skia GPU** (real acceleration)

### Step 3: Enable Skia Build

```bash
# This is YOUR NEXT COMMIT (N=12)
gn gen out/Release --args='is_debug=false pdf_enable_v8=false pdf_enable_xfa=false use_clang_modules=false pdf_use_skia=true skia_use_metal=true'

ninja -C out/Release pdfium_cli

cd integration_tests && pytest -m smoke -q
```

**Commit as:**
```
[WORKER0] # 12: Enable Skia GPU Backend - Path A Begins

User directive: Implement Path A (Skia GPU) first, then Path B, then Path C.

Previous N=10-11 GPU removal was based on outdated decision.
User wants Skia GPU implemented (real 3-8x acceleration).

Build: Skia + Metal enabled
Tests: [results]

Next: Configure Skia Metal backend for GPU rendering.
```

---

## WHY THIS MATTERS

**Skia GPU is DIFFERENT from what you built:**

**What you built (N=0-6):**
- Post-processing (0.71x slower)
- GPU resamples CPU-rendered bitmap
- No real acceleration

**What Skia GPU does:**
- Real GPU rasterization
- Replaces AGG CPU renderer
- Expected 3-8x speedup

---

## EXECUTION ORDER

**You MUST do in this order:**

1. **N=12-40:** Path A (Skia GPU) - ~25-30 commits
2. **N=41-70:** Path B (User feedback + Python + binaries) - ~30 commits
   - UTF-8 output
   - **JPEG output (CRITICAL from PR #18)**
   - Better errors
   - Python bindings
   - Linux binaries
3. **N=71-75:** Path C (Polish) - ~5 commits

**User also merged PR #18:** JPEG output is CRITICAL
- User hit 87 GB of PNGs in 30 minutes
- Need `--format jpg` flag
- This goes in Path B after UTF-8

---

## DO NOT CONTINUE GPU REMOVAL

**You removed GPU based on Option 2 decision.**

**User then changed to:**
"Do Path A (GPU), then Path B, then C"

**You must implement Skia GPU, not remove it.**

---

## N=12: ENABLE SKIA

Execute the GN and ninja commands above.

This is non-negotiable. User explicitly ordered Path A first.

**START NOW.**
