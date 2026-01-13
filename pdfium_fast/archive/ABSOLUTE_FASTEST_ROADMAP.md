# Absolute Fastest PDFium - Rigorous Optimization Roadmap

**Mission**: Make PDFium_fast the ABSOLUTE FASTEST PDF renderer and text extractor
**Method**: Iterative profiling, optimization, validation loops across DIVERSE PDFs
**Baseline**: v1.1.0 (7.54x rendering, 3.11x text with K=8 threading)
**Target**: 20-30x rendering, 5-8x text extraction

---

## The Diversity Problem

**Bad approach**: "Optimize for test.pdf, claim it's fast"
**Rigorous approach**: Profile 100+ PDFs across categories, optimize for AVERAGE case

### PDF Diversity Matrix

**Our corpus** (integration_tests/pdfs/):
- **arxiv/**: Academic papers (text-heavy, equations, figures) - 50+ PDFs
- **web/**: Web captures (mixed content, images, forms) - 40+ PDFs
- **edinet/**: Financial reports (tables, Japanese text, charts) - 30+ PDFs
- **cc/**: Corporate docs (varies widely) - 50+ PDFs
- **benchmark/**: Large multi-page docs (100-1000 pages) - 10+ PDFs
- **synthetic/**: Generated test cases (edge cases) - 20+ PDFs

**Total**: 200+ PDFs with diverse characteristics

**Categories by content**:
- Text-heavy (>80% text): ~60 PDFs
- Image-heavy (>80% images): ~40 PDFs
- Mixed (balanced): ~70 PDFs
- Scanned (JPEG fast path): ~30 PDFs

---

## Phase 1: Establish Rigorous Baseline (N=224-228, 2-3 hours)

### N=224: Corpus-Wide Performance Baseline

**Task**: Measure CURRENT performance across ALL 200+ PDFs

**Commands**:
```bash
cd integration_tests

# Measure rendering
for pdf in pdfs/*/*.pdf; do
    /usr/bin/time -l ../out/Release/pdfium_cli --threads 1 render-pages $pdf /tmp/out_k1/ 2>&1
    /usr/bin/time -l ../out/Release/pdfium_cli --threads 8 render-pages $pdf /tmp/out_k8/ 2>&1
done > baseline_rendering_corpus.txt

# Measure text extraction
for pdf in pdfs/*/*.pdf; do
    /usr/bin/time -l ../out/Release/pdfium_cli --workers 1 extract-text $pdf /tmp/out.txt 2>&1
    /usr/bin/time -l ../out/Release/pdfium_cli --workers 8 extract-text $pdf /tmp/out.txt 2>&1
done > baseline_text_corpus.txt
```

**Analyze**:
- Group by PDF category
- Calculate: mean, median, stddev, min, max for each category
- Identify: Which PDFs are slowest? Which categories benefit most from threading?

**Document**: reports/v1.2/baseline_corpus_analysis.md

**Deliverables**:
- Performance distribution histogram
- Speedup by category (text-heavy vs image-heavy)
- Outliers (PDFs that are 10x slower than median)
- Bottleneck categories (where optimization will help most)

---

### N=225-226: Deep Profiling on Representative PDFs

**Select 20 PDFs** (diverse sample):
- 5 fastest (learn what makes them fast)
- 5 slowest (learn what makes them slow)
- 5 text-heavy
- 5 image-heavy

**Profile EACH with Instruments**:
```bash
for pdf in selected_20/*.pdf; do
    instruments -t "Time Profiler" -D profile_$pdf.trace \
        out/Profile/pdfium_cli --threads 1 render-pages $pdf out/
done
```

**Analyze**:
- Top 10 functions by CPU time for each PDF
- Group functions by category (PNG encoding, AGG rendering, FreeType, parsing)
- Calculate average % for each category across 20 PDFs

**Expected output**:
```
PNG encoding: 68% (σ=12%, range 45-85%)
AGG rendering: 18% (σ=8%, range 10-30%)
FreeType: 8% (σ=4%, range 3-15%)
Parsing: 4% (σ=2%, range 2-8%)
Other: 2%
```

**Success criteria**: Identify bottlenecks with statistical confidence (not single-PDF anecdotes)

---

### N=227-228: Text Extraction Profiling

**Profile text extraction** on same 20 PDFs:

```bash
instruments -t "Time Profiler" out/Profile/pdfium_cli --workers 1 extract-text $pdf out.txt
```

**Analyze**:
- Character positioning calculations
- Font metric lookups
- Unicode normalization
- std::vector operations

**Document**: Which operations dominate? Where's the time going?

---

## Phase 2: PNG Optimization Loop (N=229-235, 3-4 hours)

### N=229: Implement Z_NO_COMPRESSION

**Based on profiling results from N=225-226**.

**IF PNG encoding >50% average**: Implement optimization
**IF PNG encoding <30% average**: Skip, move to rendering optimization

**Implementation**:
```cpp
// testing/image_diff/image_diff_png_libpng.cpp
png_set_compression_level(png_ptr, Z_NO_COMPRESSION);
png_set_filter(png_ptr, 0, PNG_FILTER_NONE);
```

### N=230: Measure Impact Across Corpus

**Test on ALL 200+ PDFs** (not just 5!):

```bash
# Before optimization
time_all_pdfs > before_png_opt.txt

# After optimization
time_all_pdfs > after_png_opt.txt

# Analyze
python analyze_speedup.py before_png_opt.txt after_png_opt.txt
```

**Report**:
- Mean speedup across corpus
- Speedup by category
- Best case, worst case, outliers
- File size impact

**Success criteria**: ≥1.8x mean speedup IF PNG was >60% bottleneck

### N=231-232: SIMD Color Conversion

Port NEON/AVX2 from old version, measure on corpus.

### N=233-235: Alternative Formats

**Test**: Skip PNG entirely, output raw BGRA

**Measure**: Speedup across corpus
**Decision**: Keep if ≥2.5x, otherwise revert

---

## Phase 3: Rendering Pipeline Loop (N=236-250, 6-8 hours)

### Optimization Cycle (Repeat for each optimization)

**1. Profile** (1 iteration):
- Find next bottleneck via Instruments
- Measure on 20+ diverse PDFs
- Calculate average contribution

**2. Hypothesize** (document before coding):
- What optimization will help?
- Expected speedup?
- Trade-offs?

**3. Implement** (1-2 iterations):
- Make the change
- Build, basic test

**4. Validate** (1 iteration):
- **Correctness**: Full test suite (2,751 tests)
- **Performance**: Measure on 50+ PDFs
- **Scale**: 100-PDF batch test

**5. Decide** (keep or revert):
- IF speedup ≥1.15x AND 100% correctness: Keep
- IF speedup <1.15x OR any test fails: Revert

**6. Document** (same iteration):
- reports/v1.2/optimization_N{}_results.md
- Include: before/after data, decision rationale

**7. Re-profile** (next cycle):
- Bottleneck moved? Find new one.
- Repeat until diminishing returns

---

### Candidate Optimizations (Priority by Expected Impact)

**1. Anti-Aliasing Quality** (N=236-240):
- Test: None, Low (2x2), Medium (4x4), High (8x8)
- Measure on 50+ PDFs
- Create quality flag: `--quality {fast|balanced|high}`
- Expected: +30-50% with "fast" mode

**2. Skip Operations When Possible** (N=241-244):
- Transparency: Skip alpha blending for opaque pages
- Empty pages: Detect and skip rendering
- Off-screen objects: Cull before rendering
- Expected: +10-20% average, +50% for simple PDFs

**3. Glyph Bitmap Cache** (N=245-249):
- Pre-render A-Z, a-z, 0-9 to bitmaps
- Cache by (font, char, size)
- Measure hit rate across corpus
- Expected: +30-50% for text-heavy PDFs

**4. Memory Allocator Tuning** (N=250):
- Test: jemalloc vs PartitionAlloc
- Measure on corpus
- Expected: +5-10% if allocation is bottleneck

---

## Phase 4: Adaptive Scheduling (N=251-258, 3-4 hours)

### N=251-253: Implement --auto Flag

**Smart (N,K) selection based on**:
- Page count (from Phase 1 analysis)
- PDF category (fast classifier)
- CPU cores available

**Algorithm**:
```python
# Learn optimal (N,K) from Phase 1 data
for each category in [text-heavy, image-heavy, mixed, scanned]:
    for each size in [small<50, medium 50-200, large>200]:
        optimal_nk = find_best_from_baseline(category, size)
        lookup_table[category][size] = optimal_nk
```

**Test**:
- Run --auto on 100+ PDFs
- Compare to manually optimized (N,K) for each
- Measure: How often is auto within 20% of optimal?

**Success criteria**: Auto is optimal or near-optimal (>80% of cases)

### N=254-258: Validate Adaptive Scheduling at Scale

**Test scenarios**:
- Batch of 100 small PDFs (<50 pages each)
- Batch of 100 large PDFs (>200 pages each)
- Mixed batch (50 small, 50 large)
- All with --auto flag

**Measure**:
- Total throughput (PDFs/min)
- Resource utilization (CPU, memory)
- Compare to fixed (N,K)

---

## Phase 5: Text Extraction Optimization (N=259-270, 4-5 hours)

### Text Extraction is Currently SLOWER (3.11x vs 7.54x rendering)

**Why?** Profile to find out.

### N=259-261: Text Extraction Profiling

**Profile 20+ text-heavy PDFs**:
```bash
instruments -t "Time Profiler" out/Profile/pdfium_cli extract-text $pdf out.txt
```

**Find**:
- What % is character positioning?
- What % is font metrics?
- What % is Unicode processing?
- What % is std::vector operations?

### N=262-265: Optimize Dominant Bottleneck

**Based on profiling**, implement targeted optimization.

**Examples**:
- Pre-allocate vectors (if allocation is bottleneck)
- Cache font metrics (if lookups dominate)
- Batch Unicode conversion (if normalization is slow)

**Measure on corpus** (50+ PDFs).

### N=266-270: Iterate Until Diminishing Returns

**Goal**: Match rendering speedup (7.54x)
**Method**: Profile → optimize → measure → repeat

**Stop when**: Further optimization gives <10% improvement

---

## Phase 6: Scale and Stress Testing (N=271-285, 6-8 hours)

### N=271-275: Massive Scale Testing

**Test 1**: 1000-PDF batch
```bash
for pdf in corpus_1000/*.pdf; do
    pdfium_cli --auto render-pages $pdf out/
done
```

**Measure**:
- Success rate (how many succeed?)
- Crash rate (any hangs or crashes?)
- Memory usage over time (does it grow?)
- Throughput stability (constant or degrading?)

**Test 2**: 10,000-page single PDF

**Test 3**: Concurrent processing (50 PDFs in parallel processes)

### N=276-280: Stress Testing

**Oversubscription**:
- N=16 workers × K=8 threads = 128 threads on 8-core machine
- Does it gracefully degrade?
- Warning messages?

**Resource limits**:
- Process 1000 PDFs with 1GB RAM limit
- Does it handle memory pressure?

**Malformed PDFs**:
- Already have 254 edge case tests
- Run with threading enabled
- Any new crashes?

### N=281-285: Variance Analysis

**Run each benchmark 50 times** (not 10):
```bash
for i in {1..50}; do
    time pdfium_cli --threads 8 render-pages large.pdf out/
done > variance_k8.txt

# Analyze
python analyze_variance.py variance_k8.txt
```

**Calculate**:
- Distribution (histogram)
- Outliers (identify environmental factors)
- Confidence intervals (95% CI for claimed speedup)

**Success criteria**: Variance <15%, reproducible results

---

## Phase 7: Compiler & Build Optimization (N=286-295, 4-5 hours)

### N=286-288: LTO (Link-Time Optimization)

```gn
use_thin_lto = true
```

**Benchmark on 100+ PDFs**:
- Before LTO: Mean, median, stddev
- After LTO: Mean, median, stddev
- Calculate improvement with confidence intervals

**Expected**: +10-15%
**Verify**: Measure, don't assume

### N=289-291: Strip Unnecessary Code

```gn
pdf_enable_forms = false
pdf_enable_edit = false
pdf_enable_annotations = false
pdf_enable_javascript = false
# Keep ONLY: render-pages, extract-text, extract-jsonl
```

**Measure**:
- Binary size reduction
- Performance impact (cache locality)
- Verify no test regressions

**Expected**: +5-10%

### N=292-295: Aggressive Flags + Profile-Guided Iteration

**Test compiler flags**:
```gn
optimize_for_speed = true
clang_optimize = "3"  # -O3
use_lld = true  # Faster linker
```

**Each flag**: Benchmark on corpus, measure impact

---

## Phase 8: Final Validation (N=296-300, 3-4 hours)

### N=296-298: Comprehensive Correctness

**Full test suite with all optimizations**:
```bash
# All thread counts
pytest -q  # 2,751 tests
PDFIUM_CLI=out/Release_K2/pdfium_cli pytest -q
PDFIUM_CLI=out/Release_K4/pdfium_cli pytest -q
PDFIUM_CLI=out/Release_K8/pdfium_cli pytest -q
```

**Success criteria**: 100% pass rate for ALL thread counts

### N=299: Final Benchmark - Prove Absolute Fastest

**Benchmark suite**:
- 200+ PDFs from corpus
- 10 runs each (2000+ total measurements)
- Document: mean, median, P95, P99 for each PDF

**Compare to**:
- Upstream PDFium (baseline)
- v1.0.0 (multi-process only)
- v1.1.0 (threading)
- v1.2.0 (optimized)

**Create performance matrix**:
```
                Upstream  v1.0   v1.1   v1.2
Text (mean)     1.00x     3.0x   3.1x   5.5x
Render (mean)   1.00x     6.8x   7.5x   22.0x
```

**Prove claims with data**.

### N=300: Release v1.2.0

**Requirements**:
- 100% test pass rate
- Performance ≥ v1.1.0 (no regressions)
- Documented on 100+ PDFs (not anecdotes)
- Scale tested (1000+ PDFs processed without crashes)
- Variance documented (reproducible results)

---

## Optimization Discovery Loop (Ongoing)

**For any optimization**:

### Step 1: Profile (1 iteration)
- Run Instruments on 20+ diverse PDFs
- Identify function consuming ≥5% CPU time
- Document: function name, average %, range across PDFs

### Step 2: Hypothesize (same iteration)
- What optimization will help this function?
- Expected speedup? (be conservative)
- Trade-offs? (quality, memory, complexity)
- Document hypothesis BEFORE coding

### Step 3: Implement (1-2 iterations)
- Make the change
- Build successfully
- Basic smoke test

### Step 4: Correctness Validation (1 iteration)
```bash
# Full test suite
pytest -q
# Expected: 100% pass (2,751/2,751)

# Revert if ANY test fails
```

### Step 5: Performance Validation (1-2 iterations)
```bash
# Measure on 50+ PDFs (not 5!)
for pdf in corpus_sample_50/*.pdf; do
    time_before_and_after $pdf
done

# Analyze
python analyze_speedup.py --min-sample 50 --confidence 95%
```

**Calculate**:
- Mean speedup with 95% confidence interval
- Speedup by category (text/image/mixed)
- Success rate (% of PDFs that got faster)

### Step 6: Decision (same iteration)
**Keep IF**:
- Mean speedup ≥1.15x (15% improvement)
- Confidence interval doesn't include 1.0x
- ≥70% of PDFs improved
- 100% correctness maintained

**Revert IF**:
- Mean speedup <1.15x (not worth complexity)
- Any test failures
- Regression in any category

### Step 7: Document (same iteration)
```markdown
# Optimization N={}: {Name}

Hypothesis: {What we expected}
Result: {What we measured}
Decision: {Keep/Revert + rationale}

Performance:
- Mean: X.XXx (95% CI: [X.XX, X.XX])
- By category: text=X.Xx, image=X.Xx, mixed=X.Xx
- Success rate: XX% improved, XX% unchanged, XX% worse

Files: {what changed}
Trade-offs: {speed vs quality vs memory}
```

---

## Statistical Rigor Requirements

### Minimum Sample Sizes

**For any performance claim**:
- Micro-benchmark (single function): ≥100 runs
- PDF-level benchmark: ≥10 runs per PDF
- Corpus-level benchmark: ≥50 PDFs, ≥5 runs each
- Variance analysis: ≥30 samples minimum

### Reporting Standards

**Always report**:
- Mean (central tendency)
- Median (robust to outliers)
- Standard deviation (variability)
- Min/Max (range)
- Sample size N
- P95/P99 for latency analysis

**Never claim**:
- "2.5x faster" without confidence interval
- "Works on test.pdf" without corpus validation
- "PNG is 74%" without measuring your build

### Decision Thresholds

**Optimization deemed successful** if:
- Mean improvement ≥1.15x (15%)
- 95% CI lower bound ≥1.10x (10% minimum)
- Success rate ≥70% (majority improved)
- Zero test regressions

**Optimization deemed failed** if:
- Mean improvement <1.10x (<10%)
- OR 95% CI includes 1.0x (uncertain benefit)
- OR success rate <50% (hurts more than helps)
- OR any correctness regression

---

## Progress Tracking

**Every 5 iterations**: Comprehensive report
- Current total speedup vs baseline
- Optimizations attempted vs kept
- Bottlenecks remaining
- Estimated ceiling (how much more possible?)

**Every 13 iterations**: Full corpus benchmark
- 200+ PDFs, 10+ runs each
- Statistical analysis
- Regression check vs previous benchmark

---

## Hard Stop Conditions

**Stop optimizing when**:
1. **Diminishing returns**: 5 consecutive optimizations give <5% each
2. **Ceiling reached**: Profile shows no function >3% CPU time
3. **Risk > reward**: Further optimization requires correctness trade-offs
4. **Target achieved**: 20-25x rendering, 5-8x text extraction verified on corpus

**Then**: Document final results, release v1.2.0, conclude.

---

## Anti-Patterns to Avoid

**❌ Optimizing for one PDF**: "Made test.pdf 10x faster!" (not representative)
**✅ Optimizing for corpus**: "Mean 2.3x across 100 PDFs, 82% improved"

**❌ Accepting claims**: "PNG is 74%" (from old version)
**✅ Verifying claims**: "Profiled 20 PDFs, PNG is 68% ± 12%"

**❌ Small samples**: "Tested on 5 PDFs" (statistically weak)
**✅ Large samples**: "Tested on 50 PDFs, 10 runs each (500 measurements)"

**❌ Ignoring variance**: "7.54x speedup" (could be 6x-9x)
**✅ Reporting variance**: "7.54x mean (95% CI: [7.2x, 7.9x], σ=0.6x, N=50)"

---

## Expected Timeline

**Phase 1** (Baseline): N=224-228 (2-3 hours)
**Phase 2** (PNG): N=229-235 (3-4 hours)
**Phase 3** (Rendering): N=236-250 (6-8 hours)
**Phase 4** (Scheduling): N=251-258 (3-4 hours)
**Phase 5** (Text): N=259-270 (4-5 hours)
**Phase 6** (Scale): N=271-285 (6-8 hours)
**Phase 7** (Compiler): N=286-295 (4-5 hours)
**Phase 8** (Final): N=296-300 (3-4 hours)

**Total**: ~75-80 iterations = 30-40 hours of rigorous optimization work

---

## Success Metrics (Must Achieve)

**Performance** (verified on 100+ PDFs with statistical confidence):
- Rendering: 20-25x mean speedup (vs upstream baseline)
- Text extraction: 5-8x mean speedup
- Small PDFs (<50p): 10-15x speedup
- Large PDFs (>200p): 25-30x speedup

**Correctness** (non-negotiable):
- 100% test pass rate (2,751/2,751)
- Deterministic (multiple runs identical)
- Zero crashes in 1000-PDF batch

**Reproducibility**:
- Variance <15% for all benchmarks
- Results consistent across multiple benchmark sessions
- Performance claims have 95% confidence intervals

**Documentation**:
- Every optimization: hypothesis, measurement, decision
- Final report: comprehensive performance matrix
- User guide: when to use which flags

---

**WORKER0 N=224: Start Phase 1. Profile PNG encoding on 20+ diverse PDFs. Be skeptical. Measure everything.**
