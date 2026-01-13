# Rigorous Validation Framework - Prove Every Claim

**Skepticism**: "68.7x speedup" sounds too good. Is it real? Is it representative?
**Method**: Hard measurement, statistical rigor, full test suites

---

## The 68.7x Claim - Is It Real?

**Worker's claim** (N=225): "68.7x combined speedup"

**Breakdown** (need to verify):
- PNG optimization alone: 9.35x (single-threaded)
- Threading alone: 7.54x (K=8)
- Combined: 9.35 × 7.54 = 70.5x theoretical (close to 68.7x)

**Questions**:
1. **One PDF or corpus average?** (likely one PDF - cc_001_931p.pdf)
2. **Is that PDF representative?** (need to test 100+ PDFs)
3. **What about small PDFs?** (Amdahl's Law + overhead)
4. **Variance?** (run 10+ times, report stddev)

**Task**: Validate claim across diverse corpus

---

## Rigorous Measurement Protocol

### Rule 1: Decompose Combined Effects

**DON'T measure**: "68.7x total speedup" (combines multiple optimizations)

**DO measure** (separately):
```bash
# Baseline (v1.0.0)
time pdfium_cli_v1.0 --threads 1 render-pages test.pdf out/  # A

# PNG optimization only
time pdfium_cli_v1.2 --threads 1 render-pages test.pdf out/  # B
Speedup_PNG = A / B  # Should be ~9-11x per worker claim

# Threading only (without PNG opt)
time pdfium_cli_v1.1 --threads 1 render-pages test.pdf out/  # C
time pdfium_cli_v1.1 --threads 8 render-pages test.pdf out/  # D
Speedup_Threading = C / D  # Should be ~7.5x per CLAUDE.md

# Combined
time pdfium_cli_v1.2 --threads 8 render-pages test.pdf out/  # E
Speedup_Combined = A / E  # Verify it's ~B×D
```

**Verify**: Combined = PNG × Threading (not some magic extra gain)

### Rule 2: Test on Diverse Corpus (Not One PDF)

**Minimum sample sizes**:
- **Quick validation**: 20 PDFs (diverse categories)
- **Corpus validation**: 100+ PDFs (statistical confidence)
- **Full validation**: All 462 PDFs (complete coverage)

**Stratify by**:
- Size: <10p, 10-50p, 50-200p, >200p
- Content: text-heavy, image-heavy, mixed, scanned
- Complexity: simple, moderate, complex (CJK, forms, patterns)

**Report**:
- Mean, median, stddev
- Min, max (identify outliers)
- Per-category breakdown
- Variance analysis

**Example output**:
```
PNG Optimization Impact (100 PDFs, 10 runs each):

Overall:
- Mean: 9.2x (95% CI: [8.8x, 9.6x])
- Median: 9.4x
- Stddev: 1.2x
- Range: 3.1x-14.7x

By Category:
- Text-heavy (n=30): Mean 11.2x (PNG dominant)
- Image-heavy (n=25): Mean 6.3x (rendering matters more)
- Small <10p (n=20): Mean 3.1x (overhead visible)
- Large >200p (n=25): Mean 12.8x (amortized overhead)

Decision: Keep (mean >9x, all categories improved)
```

### Rule 3: Amdahl's Law Analysis

**Theoretical maximum** for threading:
```
Speedup = 1 / [(1-P) + P/N]

Where:
P = parallelizable fraction
N = thread count
```

**For K=8 threads**:
- If P=95% (5% serial): Max = 6.9x
- If P=90% (10% serial): Max = 4.7x
- Claimed 7.5x suggests P≈97% (very good!)

**Validate**:
- Measure serial overhead (pre-loading time)
- Calculate P from actual data
- Verify speedup matches theoretical

**For small PDFs** (e.g., 10 pages):
- Thread spawn overhead: ~50ms
- Page render time: ~10ms each
- Total: 50ms overhead + 100ms work = 150ms
- With K=8: 50ms + 100ms/8 = 62.5ms
- Speedup: 150/62.5 = 2.4x (not 8x due to overhead)

**Expect**: Diminishing returns on small PDFs

### Rule 4: Full Test Suite After Every Optimization

**Frequency**: After EVERY code change that affects rendering/extraction

**Command**:
```bash
cd integration_tests
python3 -m pytest -q
```

**Required result**: 2,751/2,751 pass (100%)
- 2,749 passed
- 2 xfailed (known upstream bugs)
- 0 failed
- 0 skipped

**If ANY test fails**:
1. STOP optimization work
2. Debug the failure
3. Fix or revert optimization
4. Re-run full suite
5. Only continue when 100% pass

**Cadence**:
- After each optimization: Full suite (2,751 tests, ~1h 45m)
- Quick check: Smoke tests (67 tests, ~7 min)
- Before merge: Full suite + stress tests

---

## What We Need to Prove

### Claim 1: "68.7x speedup with PNG + K=8"

**To prove**:
- [ ] Measure on 100+ PDFs (not just cc_001_931p.pdf)
- [ ] Report distribution (mean, median, range, outliers)
- [ ] Verify combined = PNG × Threading (not inflated)
- [ ] Test small PDFs (<10p) - expect lower speedup
- [ ] Test huge PDFs (>1000p) - expect higher speedup
- [ ] Run 50+ iterations per PDF (check variance)

**Acceptance criteria**:
- Mean ≥ 50x (allow some regression from cherry-picked 68x)
- Median ≥ 45x
- Stddev < 20x (reproducible)
- Small PDFs ≥ 5x (overhead matters)
- Large PDFs ≥ 60x (amortized overhead)

### Claim 2: "PNG was 97% bottleneck"

**To prove**:
- [ ] Profile 20+ diverse PDFs with Instruments
- [ ] Calculate PNG % for each
- [ ] Report: mean, min, max across corpus
- [ ] Verify claim holds for different PDF types

**Acceptance criteria**:
- Mean PNG % ≥ 80% (within ballpark of 97%)
- Holds for ≥70% of PDFs tested
- Documented variance (not all PDFs same)

### Claim 3: "Anti-aliasing optimization gives 1.3-1.5x"

**To prove**:
- [ ] Benchmark --quality fast vs balanced on 50+ PDFs
- [ ] Measure on rendering-heavy PDFs specifically
- [ ] Visual quality assessment (acceptable degradation?)

**Acceptance criteria**:
- Mean ≥ 1.15x (15% minimum per roadmap)
- Rendering-heavy PDFs ≥ 1.3x
- Quality degradation acceptable
- Zero correctness failures

### Claim 4: "K=8 threading gives 7.5x"

**To prove**:
- [ ] Test on 50+ PDFs with SAME PNG settings
- [ ] Measure K=1,2,4,8,16 (full scaling curve)
- [ ] Check Amdahl's Law compliance
- [ ] Test small PDFs (expect lower speedup)

**Acceptance criteria**:
- Large PDFs: 6-8x at K=8 (matches theory)
- Small PDFs: 2-4x at K=8 (overhead matters)
- Scaling curve follows Amdahl's Law
- Zero data races (TSan clean)

---

## Small PDF Problem (Amdahl + Overhead)

**Hypothesis**: Small PDFs won't benefit much from parallelism

**Why**:
```
Small PDF (10 pages, 10ms each to render):
- K=1: 100ms total
- K=8: Thread spawn (50ms) + render (100ms/8) + join (5ms) = 67.5ms
- Speedup: 100/67.5 = 1.48x (not 8x!)

Large PDF (1000 pages, 10ms each):
- K=1: 10,000ms
- K=8: 50ms + 10,000ms/8 + 5ms = 1,305ms
- Speedup: 10,000/1,305 = 7.66x (close to 8x!)
```

**Test protocol**:
- 20 PDFs with <10 pages
- 20 PDFs with 10-50 pages
- 20 PDFs with 50-200 pages
- 20 PDFs with >200 pages

**Expected results**:
- <10p: 1.5-3x speedup (overhead dominates)
- 10-50p: 3-5x speedup (balanced)
- 50-200p: 5-7x speedup (approaching theoretical)
- >200p: 6-8x speedup (near theoretical max)

**This validates Amdahl's Law** and justifies adaptive scheduling.

---

## Correctness Validation Cadence

**After every optimization** (non-negotiable):

### Quick Check (7 minutes):
```bash
cd integration_tests
pytest -m smoke -q
# 67 tests, ensures nothing catastrophically broken
```

### Full Check (1h 45m):
```bash
pytest -q
# 2,751 tests, ensures ALL edge cases work
```

### Recommended Cadence:
- **Every iteration**: Smoke tests (67 tests)
- **Every 5 iterations**: Full suite (2,751 tests)
- **Before merge**: Full suite + stress tests + ASan/TSan

**Hard rule**: If ANY test fails, STOP and debug before continuing.

---

## Statistical Rigor Requirements

### Minimum Samples for Claims

**Micro-benchmark** (single PDF):
- ≥50 runs (check variance)
- Report mean ± 95% CI

**Corpus benchmark** (multiple PDFs):
- ≥20 PDFs per category
- ≥10 runs per PDF
- Report per-category statistics

**Production claim** (e.g., "68x faster"):
- ≥100 PDFs tested
- ≥5 runs per PDF (500+ measurements)
- Report: mean, median, P95, P99, range
- Identify outliers and explain

### Red Flags for Inflated Claims

**🚩 Single PDF**: "Tested on test.pdf, 68x faster!" (not representative)
**🚩 Cherry-picking**: "Best case 68x!" (what about worst case?)
**🚩 Combining effects**: "Total speedup 68x" (which optimization contributed what?)
**🚩 No variance**: "Exactly 68.7x" (where's the error bars?)
**🚩 Round numbers**: "70x faster!" (suspiciously perfect)

### Green Flags for Valid Claims

**✅ Corpus tested**: "Mean 52x across 100 PDFs"
**✅ Decomposed**: "PNG: 9.2x, Threading: 7.1x, Combined: 65x"
**✅ With variance**: "52x mean (σ=12x, range 18x-89x)"
**✅ By category**: "Text-heavy: 61x, Image-heavy: 43x, Small: 8x"
**✅ Statistical**: "52x (95% CI: [48x, 56x], N=500 measurements)"

---

## Validation Tasks for Worker

### Immediate (N=233):

**1. Validate PNG optimization on corpus**:
```bash
# Measure PNG opt impact on 100 PDFs (not just cc_001_931p.pdf)
# Report: mean, median, per-category breakdown
# Document: reports/v1.2/png_optimization_corpus_validation.md
```

**2. Run full test suite**:
```bash
pytest -q
# Ensure 2,751/2,751 pass after PNG optimization
# Document any failures
```

### Regular (Every 5 Iterations):

**1. Full test suite run**:
```bash
# N=235, 240, 245, 250, etc.
pytest -q
# Document: Session ID, pass/fail, duration
# Track: Any test flakiness or regressions
```

**2. Performance regression check**:
```bash
# Benchmark same 20 PDFs repeatedly
# Ensure performance doesn't regress
# Alert if any PDF shows >20% variance
```

### Before Release:

**1. Comprehensive corpus benchmark**:
```bash
# All 462 PDFs, 10 runs each (4,620 measurements)
# Full statistical analysis
# Document: complete performance matrix
```

**2. Stress testing**:
```bash
# 1000-PDF batch (no crashes)
# 10,000-page single PDF
# Concurrent processes (N=16 on 8-core)
# Memory stability over hours
```

**3. Sanitizer validation**:
```bash
# ASan: 100% memory safe
# TSan: 0 data races
# UBSan: 0 undefined behavior
```

---

## Amdahl's Law Reality Check

**Theoretical max for K=8 threading**:

If 95% parallelizable: Max = 5.9x
If 97% parallelizable: Max = 7.0x
If 99% parallelizable: Max = 7.9x

**Worker claims 7.54x at K=8** - this suggests ~98% parallelizable (excellent!)

**But verify**:
- Measure serial overhead (pre-loading)
- Calculate actual P from measurements
- Check if small PDFs show lower speedup (overhead not amortized)

---

## What Worker Should Do Next

### N=233: Corpus Validation of PNG Optimization

```bash
# Measure on 100+ diverse PDFs
# Before/after PNG optimization
# K=1 and K=8 separately

for pdf in corpus_100/*.pdf; do
    # Before PNG (need old binary or revert)
    time old_pdfium --threads 1 render-pages $pdf out/
    time old_pdfium --threads 8 render-pages $pdf out/

    # After PNG
    time new_pdfium --threads 1 render-pages $pdf out/
    time new_pdfium --threads 8 render-pages $pdf out/
done

# Analyze
python analyze_corpus.py --min-sample 100 --confidence 95%
```

**Report**:
- PNG optimization: Mean, median, range (K=1)
- Threading: Mean, median, range (with new PNG)
- Combined: Mean, median, range
- By category: Small, medium, large PDFs
- By content: Text, image, mixed

### N=234: Full Test Suite (Regular Pulse)

```bash
pytest -q
# Document: Session, pass/fail, duration, any issues
```

### N=235: Small PDF Analysis

```bash
# Test 50 PDFs with <10 pages
# Measure K=1,2,4,8 speedup
# Verify Amdahl's Law + overhead theory

# Expected: Lower speedup due to overhead
# Document: Crossover point (where threading helps)
```

---

## Success Criteria (Provable)

**Performance** (must measure on corpus):
- [ ] PNG opt: ≥8x mean across 100+ PDFs (currently claimed 9-11x)
- [ ] Threading: ≥6x mean at K=8 across 100+ PDFs (currently claimed 7.5x)
- [ ] Combined: ≥50x mean (theoretical: 8×6=48x, allows overhead)
- [ ] Small PDFs: ≥2x (overhead matters, can't expect 50x)
- [ ] Variance: <25% (reproducible results)

**Correctness** (run after every change):
- [ ] Full test suite: 2,751/2,751 pass (100%)
- [ ] Stress test: 1000 PDFs, 0 crashes
- [ ] ASan: 0 errors
- [ ] TSan: 0 data races

**Scale** (prove production-ready):
- [ ] 1000-PDF batch completes
- [ ] 10,000-page PDF works
- [ ] Memory stable over 1-hour run
- [ ] No disk space leaks

---

## Red Flags Worker Should Avoid

**🚩 Claiming speedup without corpus validation**
- "68x on cc_001_931p.pdf" ≠ "68x on average"

**🚩 Not running full test suites**
- "Smoke tests pass" ≠ "All 2,751 tests pass"

**🚩 Ignoring small PDF performance**
- Must report crossover point (where threading helps)

**🚩 No variance analysis**
- Single run ≠ reproducible result

**🚩 Combining optimizations without decomposition**
- Can't tell which optimization contributed what

---

## Cadence for Worker

**Every iteration**: Smoke tests (7 min)
**Every 5 iterations**: Full suite (1h 45m)
**Every 10 iterations**: Corpus benchmark (50+ PDFs)
**Before release**: Complete validation (all above + stress + sanitizers)

**Hard stop**: If ANY test fails, debug before continuing

---

## Bottom Line

**Current "68.7x" claim**: Needs corpus validation, not single-PDF anecdote

**What worker should do**:
1. Validate PNG opt on 100+ PDFs (decompose from threading)
2. Run full test suite after every optimization
3. Measure small PDF performance (Amdahl's Law check)
4. Report statistics (mean, median, CI, variance)
5. Prove claims with data

**Goal**: Rigorous evidence for every performance claim, 100% correctness maintained throughout.
