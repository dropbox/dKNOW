# v1.8.0 Roadmap - ARM Speedup Focus

**Status:** COMPLETED (2025-11-21, WORKER0 N=30-36)
**Target:** Q1-Q2 2025
**Goal:** Make pdfium_fast FASTER on Apple Silicon (ARM64)
**Baseline:** 72x speedup (v1.7.0)
**Challenge:** System is 90% memory-bound (profiling confirmed)

---

## v1.8.0 COMPLETION SUMMARY

**Implementation Status:**
- ✅ Async I/O (N=31) - Implemented
- ✅ Memory-mapped I/O (N=32) - Implemented
- ❌ jemalloc (N=33) - Deferred (version conflict)
- ❌ RGB mode (N=34) - Not viable (PDFium API constraint)
- ✅ DPI control (N=35) - Implemented

**Test Status:**
- Smoke tests: 92/92 pass (100%)
- Performance tests: 7/7 pass (100%)
- Total suite: 2,787/2,787 pass (100%)
- Session: sess_20251121_015505_6ca56048

**Actual Gains (Measured N=36):**
- Base performance (300 DPI): No degradation vs v1.7.0
- DPI control gains:
  - 150 DPI: **1.8x speedup** (231 vs 128 pages/sec, -47% memory)
  - 72 DPI: **2.3x speedup** (289 vs 128 pages/sec, -59% memory)
- Memory efficiency: -47% to -59% for lower DPI modes

**Conclusion:**
v1.8.0 delivers **configurable performance** through DPI control rather than universal speedup. The Async I/O and mmap optimizations maintain baseline performance while DPI control enables 1.8-2.3x gains for thumbnail/preview use cases.

**Production Readiness:** ✅ READY FOR RELEASE

---

## Reality Check: Memory-Bound Bottleneck

**Profiling evidence (N=343, N=392, N=405):**
- 90% of time: Memory stalls (waiting for RAM)
- 10% of time: Actual CPU work
- NO function >2% CPU time

**What this means:**
- CPU optimizations (SIMD, GPU, algorithms) have <10% of work to accelerate
- Memory bandwidth is the bottleneck (cannot make RAM faster)
- Must reduce AMOUNT of memory operations, not speed of computation

---

## Viable Approaches for ARM Speedup

### Approach 1: Async I/O (Overlap Disk + Rendering) - **5-15% gain**

**Problem:** Disk writes happen sequentially after rendering
**Solution:** Render next page while writing current page (hide I/O latency)

**Implementation:**
```cpp
// Current (sequential):
for (page in pages) {
  render(page);      // 100ms
  write_disk(page);  // 20ms
}
// Total: 120ms per page

// Async (overlapped):
ThreadPool writers(4);
for (page in pages) {
  render(page);                              // 100ms
  writers.submit([=]{ write_disk(page); });  // Async (overlaps with next render)
}
// Total: 100ms per page (20ms hidden)
// Speedup: 1.2x (20% faster)
```

**Realistic Gain:** 5-15% (depends on I/O latency)
**Effort:** 5-8 commits
**Risk:** Low (well-understood pattern)

**Status:** Not tried yet

---

### Approach 2: Memory-Mapped I/O (mmap) - **3-8% gain**

**Problem:** fwrite() copies data (user space → kernel space)
**Solution:** mmap() output files (write directly to memory-mapped pages)

**Implementation:**
```cpp
// Current: fwrite (copy overhead)
FILE* f = fopen(path, "wb");
fwrite(png_data, size, 1, f);  // Copies data to kernel

// Proposed: mmap (zero-copy)
int fd = open(path, O_RDWR | O_CREAT, 0644);
ftruncate(fd, size);
void* mapped = mmap(NULL, size, PROT_WRITE, MAP_SHARED, fd, 0);
memcpy(mapped, png_data, size);  // Direct write to file pages
munmap(mapped, size);
```

**Realistic Gain:** 3-8% (eliminate I/O copy overhead)
**Effort:** 3-5 commits
**Risk:** Low

**Status:** Not tried yet

---

### Approach 3: WebP Output Format - **TESTED, FAILED (N=328)**

**Expected:** 1.37x speedup (WebP faster than PNG)
**Actual:** 0.50x (2x SLOWER)
**Reason:** Disk I/O bottleneck (larger writes)

**Verdict:** REJECTED (evidence from N=328)

---

### Approach 4: Skip Unnecessary Work - **10-20% potential**

**Analyze what rendering does that user doesn't need:**

**Option A: Skip alpha channel for PDFs without transparency**
- PDFs without transparency: ~90% of corpus
- Current: Always render RGBA (4 bytes per pixel)
- Proposed: Render RGB (3 bytes per pixel) when no alpha
- **Memory savings:** 25% fewer bytes → 25% less memory bandwidth
- **Expected gain:** 10-15% (25% less data × 0.6 memory weight)

**Option B: Lower DPI for thumbnails**
- Current default: 300 DPI (2550×3300 pixels = 32 MB)
- Proposed: Add --dpi flag, default 150 DPI for batch
- **Memory savings:** 4x fewer pixels (2x per dimension)
- **Expected gain:** 3-4x for low-DPI use cases

**Effort:** 3-5 commits each
**Risk:** Low

---

### Approach 5: Better Memory Allocator (jemalloc) - **2-5% gain**

**Problem:** System malloc may fragment memory
**Solution:** Use jemalloc (thread-aware, cache-friendly)

**Implementation:**
```bash
# Build with jemalloc
brew install jemalloc
gn gen out/Release --args='... use_allocator="jemalloc"'
ninja -C out/Release pdfium_cli
```

**Expected Gain:** 2-5% (better cache locality)
**Effort:** 1-2 commits (just build configuration)
**Risk:** Low

**Status:** Not tried yet

---

## v1.8.0 Recommended Plan

### Phase 1: Low-Hanging Fruit (5-8 commits, 5-15% gain)

**N=1-3: Async I/O** (5-8 commits)
- Overlap rendering with disk writes
- Expected: 5-15% gain
- High confidence

**N=4-6: Memory-Mapped I/O** (3-5 commits)
- Replace fwrite with mmap
- Expected: 3-8% gain
- High confidence

**N=7-8: jemalloc** (1-2 commits)
- Try better allocator
- Expected: 2-5% gain
- Low effort, worth testing

**Total Phase 1: 1.10-1.32x potential (10-32% faster)**

### Phase 2: Skip Unnecessary Work (5-8 commits, 10-25% gain)

**N=9-11: RGB mode (no alpha)** (3-5 commits)
- Detect PDFs without transparency
- Render RGB instead of RGBA (25% less data)
- Expected: 10-15% gain on 90% of PDFs

**N=12-14: DPI flag** (2-3 commits)
- Add --dpi flag (default 300, allow 72-600)
- Lower DPI for thumbnails = 4x fewer pixels
- Expected: 3-4x for thumbnail use cases

**Total Phase 2: 1.10-1.25x potential (10-25% faster)**

### Phase 3: Measure and Validate (3-5 commits)

**N=15-17:** Run full benchmarks
**N=18-19:** Document actual gains
**N=20:** Tag v1.8.0

---

## Combined v1.8.0 Potential

**Realistic cumulative gain:**
- Phase 1: 1.10-1.32x (async I/O + mmap + jemalloc)
- Phase 2: 1.10-1.25x (skip unnecessary work)
- **Total: 1.21-1.65x (21-65% faster than v1.7.0)**

**New total:** 72x × 1.43x = **103x speedup** (midpoint estimate)

**Best case:** 72x × 1.65x = **119x speedup**

---

## Why NO GPU in v1.8.0

**GPU requires moving 90% of work** (the memory operations):
- Skia GPU: Unavailable in PDFium build (N=14 analysis)
- Metal GPU: Post-processing only, 0.71x slower (N=0-11 measured)
- Custom GPU: 50+ commits, high risk

**Better strategy:**
- Focus on I/O optimizations (async, mmap) = 10-20% for 8 commits
- Skip unnecessary work (RGB mode, DPI) = 10-25% for 8 commits
- **Higher ROI than GPU** (less effort, proven techniques)

---

## v1.8.0 Timeline

**Phase 1 (I/O):** 5-8 commits (~4 hours)
**Phase 2 (Skip work):** 5-8 commits (~4 hours)
**Phase 3 (Validate):** 3-5 commits (~2 hours)

**Total: 13-21 commits (~10 hours) for 21-65% speedup**

**vs. GPU:** 30-50 commits (~25 hours) for 0-50% speedup (high risk)

---

## Bottom Line: v1.8.0 Strategy

**Focus on memory efficiency** (not GPU):
1. Async I/O (hide latency)
2. Memory-mapped I/O (zero-copy)
3. jemalloc (better allocation)
4. RGB mode (25% less data)
5. DPI control (user choice)

**Expected: 1.21-1.65x faster (21-65% gain)**

**This is REALISTIC and ACHIEVABLE on ARM.**

GPU deferred until Chromium's Skia GPU stack is available.
