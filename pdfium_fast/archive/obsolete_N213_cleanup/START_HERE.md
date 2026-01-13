# START HERE - WORKER0 on feature/image-threading

**Branch**: feature/image-threading (fresh start)
**Your Role**: WORKER0
**First Iteration**: #0
**Mission**: Implement multi-threaded image rendering

---

## Your Task (Simple)

**Copy working code** from ~/pdfium-old-threaded and make it work.

**Proven results**: 4.28x speedup at K=8 threads (Session 51 in old version)

---

## Step 1 (Your First Commit)

Copy atomic ref-counting:

```bash
cd ~/pdfium_fast

cp ~/pdfium-old-threaded/core/fxcrt/string_data_template.h core/fxcrt/
cp ~/pdfium-old-threaded/core/fxcrt/string_data_template.cpp core/fxcrt/
cp ~/pdfium-old-threaded/core/fxcrt/weak_ptr.h core/fxcrt/
cp ~/pdfium-old-threaded/core/fxcrt/retain_ptr.h core/fxcrt/

# Build
ninja -C out/Release pdfium_cli

# If build errors: Fix the specific error, don't revert

# Commit
git add core/fxcrt/*
git commit -m "[WORKER0] # 0: Copy atomic ref-counting from old version

Copied std::atomic<intptr_t> refs_ implementation for thread-safe
reference counting. Required foundation for multi-threading.

Files: string_data_template.{h,cpp}, weak_ptr.h, retain_ptr.h
Source: ~/pdfium-old-threaded (proven working)

Next: Copy threading infrastructure (fpdf_parallel.cpp)"
```

---

## Complete Plan

**Read**: THREADING_MISSION.md (6 simple steps, 5-7 hours total)

**Key points**:
1. Copy working code (don't invent)
2. Old version achieved K=8 (it works!)
3. Debug with ASan (don't revert)
4. K=2 already proved working in previous branch

---

## Important Context

**Old version terminology**: "workers" meant THREADS (not processes)
- Session 48: 4 threads = 3.00x
- Session 51: 8 threads = 4.28x
- See TERMINOLOGY_CONFUSION.md for details

**v1.0 (current)**: 6.8x with N=4 processes
**Target (v1.1)**: 20-25x with N=4 processes × K=4 threads each

---

**Your first task**: Execute Step 1 above (copy atomic ref-counting).
