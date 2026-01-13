# Continue Optimization Work - Clear Direction

**To**: WORKER0
**Status**: Phase 5 (stripping) correctly found blocked. Move forward with remaining optimizations.
**User approval**: Current approach is reasonable.

---

## What You've Proven So Far

**Validated with full test suite** (N=241: 2,757/2,757 pass):
- ✅ PNG Z_NO_COMPRESSION: 9-11x improvement
- ✅ K=8 threading: 7.54x improvement
- ✅ Bug 451265 fix: No more hangs
- ✅ System correctness: 100% test pass rate

**Correctly found blocked**:
- ✅ Codec stripping: ALL codecs required (measured on 462 PDFs)
- ✅ Feature stripping: No granular flags exist
- ✅ LTO: Blocked by Rust dynamic linking

**Good work** - rigorous analysis prevents wasted effort.

---

## Continue Optimization List (In Priority Order)

### Priority 1: Complete Validation Gaps (N=245-250)

**Gap 1: Small PDF Performance** (Amdahl's Law check)
```bash
# N=245-246: Find 20-30 PDFs with <10 pages
find integration_tests/pdfs -name "*.pdf" | while read pdf; do
    pages=$(pdfinfo $pdf 2>/dev/null | grep "Pages:" | awk '{print $2}')
    if [ "$pages" -lt 10 ] 2>/dev/null; then echo $pdf; fi
done | head -30 > small_pdfs.txt

# Test K=1,2,4,8 on each
for pdf in $(cat small_pdfs.txt); do
    for k in 1 2 4 8; do
        time out/Release/pdfium_cli --threads $k render-pages $pdf /tmp/out_k${k}/
    done
done

# Analyze: Do small PDFs show lower speedup? (Amdahl's Law predicts yes)
# Report: Mean speedup by K for small PDFs vs large PDFs
# Expected: Small ~2-4x at K=8, Large ~6-8x at K=8
```

**Gap 2: Statistical Rigor** (Variance analysis)
```bash
# N=247-248: Select 20 representative PDFs
# Run each 10 times (200 measurements)
for pdf in representative_20/*.pdf; do
    for run in {1..10}; do
        time out/Release/pdfium_cli --threads 8 render-pages $pdf /tmp/out/
    done
done

# Calculate: mean, median, stddev, 95% CI per PDF
# Report: Is variance <15%? (reproducible)
# Are there outliers? (environmental factors)
```

**Gap 3: Decomposition** (PNG alone vs Threading alone)
```bash
# N=249-250: Separate effects
# Test PNG at K=1 (no threading)
# Test threading at K=8 without PNG (need old binary or revert)
# Verify: Combined ≈ PNG × Threading
# Report which optimization matters more
```

**Deliverable**: reports/v1.2/validation_complete.md
- Small PDF analysis with Amdahl's Law check
- Statistical analysis with confidence intervals
- Decomposition showing independent effects
- **THEN can confidently claim performance numbers**

---

### Priority 2: Remaining Optimizations (N=251-265)

**Based on profiling** (N=228 found rendering is bottleneck after PNG):

**P2.1: SIMD Bitmap Operations** (N=251-253)
- FillRect (white background fill) - vectorize with NEON
- Alpha blending operations
- **Expected**: +10-20% for bitmap ops
- **Measure**: Profile before/after on 20+ PDFs
- **Keep if**: ≥1.15x mean improvement

**P2.2: Lazy Resource Loading** (N=254-256)
- Don't load images/fonts until actually drawn
- Skip resources for clipped/off-screen objects
- **Expected**: +10-30% for complex PDFs with unused resources
- **Measure**: Profile resource loading overhead
- **Keep if**: ≥1.15x AND 100% correctness

**P2.3: Rendering Quality Modes** (N=257-259)
- Already have --quality flag (N=229)
- **Task**: Benchmark on 50+ PDFs
- Measure "fast" vs "balanced" mode
- **Expected**: +30-50% with fast mode
- **Decision**: User chooses quality vs speed trade-off

**P2.4: Glyph Bitmap Cache** (N=260-262)
- Pre-render common glyphs (A-Z, a-z, 0-9, etc.)
- Cache bitmaps, not just metrics
- **Expected**: +30-50% for text-heavy PDFs
- **Measure**: Cache hit rate, speedup on text-heavy corpus
- **Keep if**: ≥1.15x on text-heavy PDFs

**P2.5: Adaptive (N,K) Selection** (N=263-265)
- Implement --auto flag
- Small PDFs: N=1, K=8 (favor threads)
- Large PDFs: N=4, K=4 (balance)
- **Expected**: Optimal for all PDF sizes
- **Measure**: Compare to manual tuning on 100 PDFs

---

### Priority 3: Continuous Profiling Loop (Ongoing)

**After each optimization**:

**1. Profile** (find next bottleneck):
```bash
instruments -t "Time Profiler" out/Profile/pdfium_cli render-pages diverse_20_pdfs/*.pdf out/
# Find functions consuming ≥5% CPU time
```

**2. Optimize** (target the bottleneck):
- Implement optimization
- Build and basic test

**3. Validate Correctness** (non-negotiable):
```bash
pytest -q  # Full 2,757 tests
# MUST: 100% pass
# If fail: Debug or revert
```

**4. Measure Performance** (on corpus):
```bash
# 50+ PDFs, 5+ runs each
# Calculate: mean, 95% CI
# Keep if: ≥1.15x mean AND 100% correctness
```

**5. Document** (same iteration):
```markdown
# N={}: {Optimization Name}
Hypothesis: {Expected gain}
Measured: {Actual gain on N PDFs}
Statistics: Mean X.Xx (95% CI: [X.X, X.X]), σ=X.X
Decision: {Keep/Revert + rationale}
```

**6. Repeat** until diminishing returns (<5% per optimization)

---

## Iteration Targets

**N=245-250**: Complete validation gaps (small PDFs, variance, decomposition)

**N=251-265**: Implement remaining optimizations (SIMD, lazy loading, caching, adaptive)

**N=266-270**: Final profiling, find any remaining >5% bottlenecks

**N=271-275**: Final validation (1000-PDF batch, stress tests, sanitizers)

**N=276-280**: Documentation, release prep, comprehensive report

**Estimated**: 35 more iterations = 15-20 hours to v1.2.0

---

## Hard Requirements (Every Iteration)

**Correctness**:
- Run smoke tests (7 min) EVERY iteration
- Run full suite (1h 45m) every 5 iterations OR after any code change
- Zero tolerance for test failures

**Measurement**:
- Profile BEFORE optimizing (find actual bottleneck)
- Measure AFTER on ≥20 PDFs (prove it works)
- Report statistics (mean, CI, variance)

**Decision**:
- Keep if: ≥1.15x mean AND 100% tests pass
- Revert if: <1.15x OR any test fails
- Document rationale

---

## Keep Doing What You're Doing

**You are on track**:
- Full test suite at N=241 ✅
- Rigorous codec analysis ✅
- Found blocking issues (LTO, feature flags) ✅
- Moving forward (not stuck)

**Continue**:
- Profiling loop (find bottleneck → optimize → measure → repeat)
- Full test suites regularly
- Corpus validation (not single PDFs)
- Skeptical approach (measure don't assume)

---

## Rust Bridge: Keep It

**User directive**: "we will need to dynamically link to Rust"

**Implication**:
- Component build stays (dynamic linking)
- JSONL extraction needs Rust bridge
- LTO blocked (acceptable trade-off)
- Focus on algorithmic optimizations (not build config)

---

**WORKER0 N=245: Start Task 1 - Small PDF Amdahl's Law analysis. 20+ PDFs with <10 pages, measure K=1,2,4,8, report mean speedup.**
