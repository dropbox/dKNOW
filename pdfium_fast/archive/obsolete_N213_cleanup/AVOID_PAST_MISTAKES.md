# Avoid Past Mistakes - Learn from Previous Attempts

**For**: WORKER0 on feature/image-threading branch

---

## Mistake #1: Inventing Your Own Implementation

**Previous attempts**: Worker implemented their own thread pool, own mutexes, own logic
**Result**: Bugs, crashes, reverts

**Correct approach**: Copy fpdf_parallel.cpp from ~/pdfium-old-threaded EXACTLY
- Don't "improve" it
- Don't "simplify" it
- Just copy it

**It works because**: 200+ sessions debugged it already

---

## Mistake #2: Reverting When Bugs Appear

**Previous pattern**:
- K=2 works → K=4 crashes → Revert everything

**Correct approach**:
- K=2 works → K=4 crashes → Debug with ASan → Fix specific bug → K=4 works

**Old version had K=4 bugs too**: They fixed them, achieved K=8

---

## Mistake #3: Misreading Old Version Architecture

**Previous claim**: "Old version used processes (like v1.0), not threads"

**Reality**: Old version DID use threads
- fpdf_parallel.cpp has GlobalThreadPool with std::thread
- Session 48 tested "4 workers" = 4 THREADS on ONE document
- Achieved 3.00x at K=4, 4.28x at K=8

**Confusion**: Old version called threads "workers" (bad naming)

---

## Mistake #4: Claiming "Impossible" Without Evidence

**Previous claim**: "PDFium APIs not thread-safe, threading doesn't work"

**Reality**: Old version proves it works
- Multiple threads called FPDF_LoadPage() on same document
- With atomic ref-counting + mutexes = works
- 4.28x speedup achieved

**Evidence**: Git commit 64d1b4e in old version (Oct 28, 2025)

---

## What Actually Works (Proven)

**Foundation**: Atomic ref-counting
- std::atomic<intptr_t> refs_ in ByteString, WeakPtr, RetainPtr
- From old version core/fxcrt/*.h

**Threading**: fpdf_parallel.cpp
- GlobalThreadPool with std::thread workers_
- Lock-free concurrent queue
- Deferred page destruction (Session 79 fix)
- From old version fpdfsdk/fpdf_parallel.cpp

**Result**: K=8 threads, 4.28x speedup

---

## Your Advantage This Time

1. **K=2 already proved working** (commit 747b107f from old branch)
2. **All solutions documented** in old version
3. **Clear proof it works** (Session 48-51 results)
4. **User is watching** (no more abandoning allowed)

---

## Success Criteria

- ✅ K=2 works (already proved)
- ✅ K=4 works (1-2 bugs to fix)
- ✅ K=8 works (old version achieved this)
- ✅ Performance: 3-4x per worker minimum

---

**Follow THREADING_MISSION.md. Copy working code. Debug bugs. Success in 5-7 hours.**
