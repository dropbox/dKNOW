# Image Threading Mission - Simple Plan

**Branch**: feature/image-threading
**Goal**: Add multi-threaded image rendering (K threads per worker)
**Target**: 20-25x speedup (N=4 workers × K=4 threads)

---

## What We Know Works (Proven in ~/pdfium-old-threaded)

**Session 48**: 3.00x speedup at K=4 threads ✅
**Session 51**: 4.28x speedup at K=8 threads ✅

**This is REAL, TESTED, WORKING code** - not theory.

---

## The Simple Plan (6 Steps)

### Step 1: Copy Atomic Ref-Counting (1 hour)

Copy from ~/pdfium-old-threaded:
```bash
cp ~/pdfium-old-threaded/core/fxcrt/string_data_template.h core/fxcrt/
cp ~/pdfium-old-threaded/core/fxcrt/string_data_template.cpp core/fxcrt/
cp ~/pdfium-old-threaded/core/fxcrt/weak_ptr.h core/fxcrt/
cp ~/pdfium-old-threaded/core/fxcrt/retain_ptr.h core/fxcrt/
```

**Changes**: intptr_t refs_ → std::atomic<intptr_t> refs_

**Test**: Build with no errors

### Step 2: Copy Threading Infrastructure (1 hour)

```bash
cp ~/pdfium-old-threaded/fpdfsdk/fpdf_parallel.cpp fpdfsdk/
cp ~/pdfium-old-threaded/public/fpdf_parallel.h public/
cp ~/pdfium-old-threaded/third_party/concurrentqueue/concurrentqueue.h third_party/concurrentqueue/
```

Update fpdfsdk/BUILD.gn to include fpdf_parallel.cpp

**Test**: Build with no errors

### Step 3: Add CLI --threads Flag (30 min)

Add to examples/pdfium_cli.cpp:
```cpp
int thread_count = 1;  // Default: single-threaded

// Parse --threads K flag
if (strcmp(argv[i], "--threads") == 0) {
    thread_count = atoi(argv[i+1]);
}

// In render_pages_bulk(), use FPDF_RenderPagesParallelV2
// (Copy implementation from old version examples or Rust bridge)
```

### Step 4: Test K=2 (30 min)

```bash
ninja -C out/Release pdfium_cli
out/Release/pdfium_cli --threads 2 render-pages test.pdf out/

# Compare MD5s
out/Release/pdfium_cli --threads 1 render-pages test.pdf out1/
out/Release/pdfium_cli --threads 2 render-pages test.pdf out2/
diff -r out1/ out2/  # Should be identical
```

**Expected**: Works (old version K=4 worked)

### Step 5: Debug K=4 If Crashes (1-2 hours)

```bash
# Build with ASan
gn gen out/ASan --args='is_asan=true pdf_enable_v8=false use_clang_modules=false'
ninja -C out/ASan pdfium_cli

# Get crash location
out/ASan/pdfium_cli --threads 4 render-pages test.pdf out/
# ASan prints exact file:line

# Fix that specific bug (add mutex, fix logic, etc.)
# Test again
```

**Expected**: 1-2 bugs to fix, then K=4 works

### Step 6: Test K=8 and Measure Performance (1 hour)

```bash
# Test higher thread counts
out/Release/pdfium_cli --threads 8 render-pages large.pdf out/

# Measure performance
time out/Release/pdfium_cli --threads 1 render-pages large.pdf out/
time out/Release/pdfium_cli --threads 4 render-pages large.pdf out/
time out/Release/pdfium_cli --threads 8 render-pages large.pdf out/

# Expected: 3-4x at K=4, 6-7x at K=8 (old version achieved 4.28x at K=8)
```

---

## Total Time: 5-7 Hours

**Not months. Not weeks. Hours.**

Because we're copying working code, not inventing it.

---

## Rules for This Branch

1. **COPY working code** from ~/pdfium-old-threaded (don't invent your own)
2. **If it crashes**: Debug with ASan, fix specific bug, continue
3. **No reverting** without user approval
4. **K=2 already proved working** in previous attempts - build on that

---

## Quick Reference

**Working code location**: ~/pdfium-old-threaded
**Proven results**: 4.28x at K=8 threads
**Your K=2 worked**: Commit 747b107f (before revert)
**Target**: N=4 × K=4 = 16 cores = 20-25x total

---

**START**: Copy step 1 (atomic ref-counting), build, test, commit.
