# MANDATORY Validation Requirements - Non-Negotiable

**To**: WORKER0
**Priority**: CRITICAL
**Status**: You MUST complete these before claiming any optimization is "done"

---

## STOP Optimization Work Until These Are Complete

### Requirement 1: Full Test Suite (BLOCKING)

**You have NOT run full test suite since N=225 PNG optimization.**

**Evidence missing**: No "2,749 passed" in any commit since #225

**REQUIRED NOW**:
```bash
cd ~/pdfium_fast/integration_tests
pytest -q > full_suite_results_N233.txt 2>&1

# MUST show:
# 2,749 passed, 2 xfailed, 0 failed, 0 skipped
```

**If ANY test fails**:
1. STOP all optimization work
2. Debug the failure
3. Fix or revert the optimization that broke it
4. Re-run full suite
5. Only continue when 100% pass rate restored

**Frequency going forward**: After EVERY code change that affects rendering/extraction

---

### Requirement 2: Small PDF Analysis (Amdahl's Law)

**You have NOT tested small PDFs** (<10 pages).

**Theoretical expectation**:
- 5-page PDF: K=8 speedup ≈ 2-3x (not 7.5x, overhead dominates)
- 100-page PDF: K=8 speedup ≈ 6-7x (overhead amortized)

**REQUIRED**:
```bash
# Find 20 PDFs with <10 pages
find integration_tests/pdfs -name "*.pdf" -exec sh -c 'pdfinfo {} | grep -q "Pages:.*[1-9]$" && echo {}' \; | head -20 > small_pdfs.txt

# Test each with K=1,2,4,8
for pdf in $(cat small_pdfs.txt); do
    time out/Release/pdfium_cli --threads 1 render-pages $pdf /tmp/out1/
    time out/Release/pdfium_cli --threads 8 render-pages $pdf /tmp/out8/
done

# Calculate actual speedup
# Report: Mean, range, compare to theory
```

**Expected results**:
- <10 pages: 2-4x speedup (overhead matters)
- If seeing 7x+: Theory is wrong, investigate why

**Document**: reports/v1.2/small_pdf_analysis.md

---

### Requirement 3: Decompose PNG and Threading

**You claim "68.7x combined" but haven't proven decomposition.**

**REQUIRED**:
```bash
# Test 1: PNG optimization alone (K=1, no threading)
time out/Release/pdfium_cli --threads 1 render-pages large.pdf out/
# vs old binary without PNG opt
# Calculate: PNG_gain = old_time / new_time

# Test 2: Threading alone (without PNG opt)
# Build old version or temporarily revert PNG
time old_pdfium_cli --threads 1 render-pages large.pdf out/
time old_pdfium_cli --threads 8 render-pages large.pdf out/
# Calculate: Threading_gain = K1_time / K8_time

# Test 3: Verify combined
Combined_measured = baseline / (PNG_opt + K8)
Combined_theory = PNG_gain × Threading_gain

# MUST show: Combined_measured ≈ Combined_theory (within 10%)
```

**Test on 20+ PDFs, not just 1.**

**Document**: reports/v1.2/decomposition_analysis.md

---

### Requirement 4: Statistical Rigor

**Current reports lack**:
- Standard deviation
- Confidence intervals
- Multiple runs per PDF
- Variance analysis

**REQUIRED for any performance claim**:

```bash
# Example: Test PNG optimization properly
for pdf in sample_50_pdfs; do
    for run in {1..10}; do
        time out/Release/pdfium_cli render-pages $pdf out/
    done
done > measurements.txt

python analyze.py measurements.txt
# Output MUST include:
# Mean: X.Xx pages/s
# Median: X.Xx pages/s
# Stddev: X.Xx pages/s
# 95% CI: [X.X, X.X]
# Per-PDF variance: <15% acceptable, >25% = environmental
```

**Minimum sample sizes**:
- Performance optimization: 50 PDFs, 10 runs each = 500 measurements
- Quick check: 20 PDFs, 5 runs each = 100 measurements
- Single PDF deep dive: 100 runs, report distribution

---

### Requirement 5: Codec Usage Before Stripping

**Before removing ANY codec**:

```bash
# Check what corpus actually uses
for pdf in integration_tests/pdfs/*/*.pdf; do
    pdfimages -list $pdf 2>/dev/null | grep -E "jpeg|jpeg2000|jbig2|ccitt"
done > codec_usage.txt

# Count usage
echo "JPEG:" && grep -c jpeg codec_usage.txt
echo "JPEG2000:" && grep -c jpeg2000 codec_usage.txt
echo "JBIG2:" && grep -c jbig2 codec_usage.txt
echo "CCITT:" && grep -c ccitt codec_usage.txt

# Decision:
# If count = 0: Can strip (after verifying tests still pass)
# If count > 0: MUST keep (corpus needs it)
```

**Then measure impact**:
```bash
# Create minimal build
gn gen out/Minimal --args='<stripped config>'
ninja -C out/Minimal pdfium_cli

# Full test suite
PDFIUM_CLI=out/Minimal/pdfium_cli pytest -q
# MUST: 2,751/2,751 pass

# Performance on 50 PDFs
# Report MEASURED gain (not expected)
# Realistic: +3-7%, not +50%
```

---

## Acceptance Criteria for "Optimization Complete"

**An optimization is NOT complete until**:

1. ✅ Full test suite: 2,749 passed, 2 xfailed
2. ✅ Corpus validation: ≥50 PDFs tested
3. ✅ Statistical analysis: Mean, median, stddev, 95% CI reported
4. ✅ Decomposition: If combining effects, test separately
5. ✅ Amdahl's Law: Small PDFs tested if claiming threading gains
6. ✅ Reproducibility: Variance <25%, multiple runs
7. ✅ Documentation: Complete report with data

**If missing ANY of these**: Optimization is "claimed" not "proven"

---

## Your Task List (Non-Optional)

**IMMEDIATE (Before any more optimization)**:

### Task 1: Run Full Test Suite (N=233)
```bash
cd integration_tests
pytest -q
# Save output
# Verify 2,749 passed, 2 xfailed
# If failures: DEBUG before continuing
```

### Task 2: Small PDF Analysis (N=234)
```bash
# Test ≥20 PDFs with <10 pages
# Measure K=1,2,4,8 for each
# Calculate mean speedup by thread count
# Compare to Amdahl's Law theory
# Report: reports/v1.2/small_pdf_amdahls_law.md
```

### Task 3: Decompose Effects (N=235)
```bash
# Test PNG alone (K=1)
# Test threading alone (K=1 vs K=8, no PNG)
# Verify combined = PNG × Threading
# Report: reports/v1.2/optimization_decomposition.md
```

### Task 4: Statistical Analysis (N=236)
```bash
# 50 PDFs, 10 runs each = 500 measurements
# Calculate proper statistics
# Report with confidence intervals
```

---

## After These 4 Tasks Are Complete

**THEN and ONLY THEN**:
- Continue with more optimizations
- Consider code stripping
- Work on adaptive scheduling

**NOT BEFORE.**

---

## Hard Rules

**🚫 NO MORE**: "Tested on cc_001_931p.pdf"
**✅ YES**: "Tested on 50 PDFs (mean 9.2x, σ=1.4x, 95% CI [8.8x, 9.6x])"

**🚫 NO MORE**: "Smoke tests pass"
**✅ YES**: "Full test suite: 2,749 passed, 0 failed (session: sess_xxx)"

**🚫 NO MORE**: "Expected 1.3-1.5x"
**✅ YES**: "Measured 1.38x mean on 50 PDFs (95% CI [1.29x, 1.47x])"

**🚫 NO MORE**: Combining optimizations without decomposition
**✅ YES**: "PNG: 9.2x, Threading: 7.1x, Combined: 65.3x (matches theory)"

---

## Bottom Line

**You have done good work** (PNG optimization, threading, bug fixes).

**But validation is incomplete**:
- Missing: Full test suite runs
- Missing: Small PDF testing
- Missing: Decomposition analysis
- Missing: Statistical rigor

**Complete these 4 validation tasks** before continuing optimization.

**Then** we can confidently claim performance gains with proof.

---

**WORKER0 N=233: Run full test suite NOW. Report results. Then do Tasks 2-4.**
