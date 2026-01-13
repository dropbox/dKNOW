# CONSOLIDATED ROADMAP - Execute in Order

**WORKER0**: Execute these tasks in order. No skipping. No idle mode.

**Your first iteration**: Read this file, start Task 1.

---

## PHASE 1: Fix Problem Areas (20-30 iterations)

### Task 1: Smart Mode + Threading (N=1-10)

**Problem**: JPEG→JPEG fast path (545x) disabled when K>1

**Your job**: Make smart mode work with threading

**Steps**:
1. Find where smart mode is disabled for K>1
2. Understand why (race condition? pre-loading?)
3. Fix the issue
4. Test: Scanned PDFs at K=8 should get 545x (not 43x)
5. Validate: Full suite 2,760/2,760 pass

**Success**: Scanned PDFs get 545x even with --threads 8

---

### Task 2: Large PDF K=8 Investigation (N=11-20)

**Problem**: 1931-page PDF: K=8 (0.90x) slower than K=4 (1.41x)

**Your job**: Find out why and fix or document

**Steps**:
1. Profile 1931-page PDF at K=8
2. Find bottleneck (mutex? cache? bandwidth?)
3. If fixable: Fix it
4. If not: Document why K=4 is optimal for huge PDFs
5. Validate: Update adaptive threading if needed

**Success**: Understand and document the regression

---

### Task 3: Text Extraction Optimization (SKIPPED - N=525)

**Problem**: Text extraction 3x, rendering 7.5x (should match)

**Status**: ✅ SKIPPED - No performance problem exists (N=444 investigation)

**Finding (N=444)**: Text extraction and rendering have **equivalent speedups**:
- 569-page PDF: Text 3.68x vs Render 3.78x (only 3% difference)
- Both operations memory-bound (90% time in memory stalls, N=343)
- Original claim was measurement artifact (baseline confusion)

**Conclusion**: Text extraction already optimal. No optimization needed.

**See**: reports/main/N444_TEXT_VS_RENDER_PERFORMANCE_ANALYSIS.md

---

## PHASE 2: Complete Optimization Tracker (10-15 iterations)

### Task 4: Aggressive Compiler Flags (N=36-40)

**Tracker item #24**

**Implementation**:
```gn
# out/Release/args.gn
clang_optimize = "3"  # -O3
use_cxx_optimize_more = true  # Extra opts
```

**Steps**:
1. Add flags to build
2. Rebuild: `ninja -C out/Release pdfium_cli`
3. Benchmark on 50+ PDFs
4. Measure gain (expected: +3-7%)
5. Mark ✅ DONE in tracker

---

### Task 5: PGO - Profile-Guided Optimization (N=41-50)

**Tracker item #25** - DO LAST

**Implementation**:
```bash
# Training build
gn gen out/PGO-Train --args='chrome_pgo_phase=1'
ninja -C out/PGO-Train pdfium_cli

# Collect profile (run on 100+ PDFs)
for pdf in corpus/*.pdf; do
    out/PGO-Train/pdfium_cli render-pages $pdf /tmp/out/
done

# Optimized build
gn gen out/PGO-Opt --args='chrome_pgo_phase=2'
ninja -C out/PGO-Opt pdfium_cli

# Benchmark
# Expected: +10-20%
```

**Build for OSX + Linux**:
- Separate profile collection for each platform
- Ship 2 binaries (pdfium_cli_osx, pdfium_cli_linux)

**Success**: Measure actual PGO gain, create binaries

---

## PHASE 3: Final Documentation (5-10 iterations)

### Task 6: Update Tracker to 30/30 (N=51-53)

**Update OPTIMIZATION_COMPLETION_TRACKER.md**:
- Item #24: ✅ DONE (with gain measurement)
- Item #25: ✅ DONE (with OSX/Linux binaries)
- All 30 items have status

**Create final report**: reports/OPTIMIZATION_COMPLETE.md
- All 30 items documented
- Every optimization tried or documented why not
- Performance matrix (baseline → final)
- Comprehensive record for user

---

### Task 7: Update Problem Areas Status (N=54-56)

**Update TOP_5_PROBLEM_AREAS.md**:
- Problem #1: ✅ FIXED or ❌ DOCUMENTED
- Problem #2: ✅ FIXED or ❌ DOCUMENTED
- Problem #3: ✅ FIXED or ❌ DOCUMENTED

**Create final report**: reports/PROBLEM_AREAS_RESOLUTION.md

---

### Task 8: Create Final Release (N=57-60)

**Tag release**: v1.5.0 or v2.0.0 (depending on gains)

**Create PR**: With all changes

**Documentation**: Complete README, CLAUDE.md, release notes

---

## Timeline

**Phase 1** (problems): 35 iterations (~15 hours)
**Phase 2** (tracker): 15 iterations (~6 hours)
**Phase 3** (docs): 10 iterations (~4 hours)
**Total**: 60 iterations (~25 hours)

---

## Hard Rules

**1. NO idle mode**: Always working on next task

**2. NO skipping**: Complete all 3 phases in order

**3. NO "production-ready"**: Until all 60 iterations done

**4. Full test suite**: After every code change

**5. Document everything**: Every task gets a report

---

## Stop Conditions

**ONLY stop when**:
- All 60 iterations complete
- OR user explicitly says "stop working"

**NOT when**:
- "Tests pass"
- "It's fast"
- "Profiling says stop"
- "Almost done"

---

## Your First Task (Iteration 1)

**Read this roadmap**

**Start Task 1**: Smart mode + threading investigation

**Find**: Where is smart mode disabled for K>1?

**Begin**: Debugging why JPEG extraction doesn't work with threading

---

**Execute this roadmap. Do not deviate. Do not enter idle mode. Complete all 60 iterations.**
