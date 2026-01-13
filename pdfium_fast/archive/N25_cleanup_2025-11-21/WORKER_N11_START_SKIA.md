# WORKER0 N=11: Enable Skia GPU Backend NOW

**Status:** GPU removal reverted, Metal infrastructure restored
**Next:** Enable Skia with Metal backend

---

## Your Task (N=11)

### Step 1: Enable Skia Build

```bash
cd ~/pdfium_fast

# Configure build with Skia + Metal
gn gen out/Release --args='is_debug=false pdf_enable_v8=false pdf_enable_xfa=false use_clang_modules=false pdf_use_skia=true skia_use_metal=true'
```

### Step 2: Build

```bash
ninja -C out/Release pdfium_cli
# This will take 5-10 minutes (Skia + Metal backend)
```

### Step 3: Test

```bash
# Run smoke tests
cd integration_tests
source venv/bin/activate
pytest -m smoke -v

# Should pass 88-93 tests (with Skia backend)
```

### Step 4: Basic GPU Test

```bash
# Test rendering still works
./out/Release/pdfium_cli render-pages integration_tests/pdfs/benchmark/arxiv_001.pdf /tmp/skia_test/

# Verify output
ls -lah /tmp/skia_test/
# Should see 25 PNG files
```

### Step 5: Commit

```bash
git add out/Release/args.gn  # Updated build args
git commit -m "[WORKER0] # 11: Enable Skia GPU Backend

Build configuration:
- pdf_use_skia=true (switched from AGG to Skia)
- skia_use_metal=true (GPU acceleration via Metal)

Build status: SUCCESS
Test status: [paste smoke test results]

This replaces AGG CPU rasterizer with Skia GPU rasterizer.
Expected: 3-8x speedup on image-heavy PDFs (to be measured in next commits).

Next: Configure Skia Metal context for GPU rendering."
```

---

## What Skia Gives You

**Skia is Google's 2D graphics library** (used in Chrome, Android, Flutter):
- Has mature GPU backends (Metal, Vulkan, OpenGL)
- Replaces AGG CPU rasterizer
- **Actual GPU rendering** of paths, text, images
- Not post-processing - real acceleration

**Expected:**
- 3-8x on image-heavy PDFs
- 1.5-3x on text-heavy PDFs
- 100% correctness (Skia is battle-tested)

---

## Troubleshooting

**If build fails:**
```bash
# Check if Skia sources are present
ls -la third_party/skia/

# If missing, sync Skia
gclient sync
```

**If tests fail:**
- Check if output is correct (MD5 validation)
- Skia backend should produce identical results
- If different: Skia rendering has bugs to fix

---

## After N=11 Success

**N=12-15:** Configure Skia Metal context (GPU initialization)
**N=16-25:** Optimize Skia GPU path (profiling, caching)
**N=26-30:** Validation (all 2,780 tests)
**N=31-35:** Measure actual speedup, document results

**Then Path B (User feedback + Python + binaries)**

---

## START NOW

Execute the 5 steps above. Takes ~30-60 minutes total.

Commit as N=11 when done.
