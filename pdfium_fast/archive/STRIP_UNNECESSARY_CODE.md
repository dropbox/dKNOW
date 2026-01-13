# Strip Unnecessary Code - Minimal Build for Rendering + Extraction

**Goal**: Remove all PDFium features except rendering + rich text extraction
**Method**: Measure before/after, verify no regressions
**Expected gain**: +5-15% (smaller binary, better cache locality, less initialization)

---

## What We Need (Keep)

**Core functionality**:
- FPDF_LoadDocument, FPDF_LoadPage, FPDF_ClosePage
- FPDF_RenderPageBitmap (image rendering)
- FPDFText_LoadPage, FPDFText_GetText (text extraction)
- FPDF_GetPageCount, FPDF_GetPageWidth/Height

**Codecs** (need to check corpus):
- JPEG decode (libjpeg-turbo) - REQUIRED
- PNG decode (libpng) - REQUIRED
- FreeType (text rendering) - REQUIRED
- AGG (anti-aliasing rendering) - REQUIRED
- JBIG2? (check if any PDFs use it)
- JPX/JPEG2000? (check if any PDFs use it)

---

## What We Don't Need (Strip)

### Category 1: Interactive Features (CAN REMOVE)

**Form filling**: ~176KB (fpdfsdk/formfiller/)
```gn
# Already partially disabled
pdf_enable_forms = false  # NEW FLAG (if exists)
```

**Annotations editing**: ~216KB (fpdf_annot*)
- We render annotations but don't edit them
- Keep rendering, strip editing APIs

**Signatures**: ~16KB (fpdf_signature*)
- Digital signatures, certificates
- Not needed for rendering/extraction

### Category 2: Editing Features (CAN REMOVE)

**Page editing**: ~40KB (fpdf_editpage*)
- Adding/removing pages
- Rotating pages
- Not needed

**Content editing**: ~88KB (fpdf_edit*, fpdf_editpath*, fpdf_edittext*, fpdf_editimg*)
- Adding text/images/paths
- Modifying objects
- Not needed

**Document saving**: ~20KB (fpdf_save*)
- Writing modified PDFs
- Incremental updates
- Not needed (we only read)

### Category 3: Optional Codecs (MEASURE FIRST)

**JBIG2**: Check if any PDFs in corpus use it
```bash
# Test
for pdf in integration_tests/pdfs/*/*.pdf; do
    pdfinfo $pdf | grep -i jbig2 && echo $pdf
done

# If no PDFs use JBIG2:
# Strip it from build (save ~100KB, faster binary)
```

**JPX/JPEG2000**: Check if any PDFs use it
```bash
# Similar test
# If unused: Strip it
```

**CCITTFax (G3/G4)**: Fax encoding
- Unlikely in modern PDFs
- Check corpus, strip if unused

### Category 4: Print-Specific Code (MAYBE REMOVE)

**Print rendering path**: Separate from screen rendering
- Check if any code paths only for printing
- If so, can strip

---

## Build Configuration Changes

### Current (out/Release/args.gn):
```gn
is_debug = false
pdf_enable_v8 = false
pdf_enable_xfa = false
use_clang_modules = false
```

### Proposed Minimal Build:
```gn
is_debug = false
pdf_enable_v8 = false
pdf_enable_xfa = false
pdf_enable_forms = false  # NEW
pdf_enable_edit = false  # NEW
is_component_build = false
use_clang_modules = false
use_thin_lto = true  # Enable LTO for size/speed
optimize_for_speed = true  # Not size
symbol_level = 0  # No debug symbols
```

---

## Measurement Protocol

### Step 1: Baseline Binary Size

```bash
ls -lh out/Release/pdfium_cli out/Release/libpdfium.dylib
# Current sizes (need to measure)
```

### Step 2: Create Minimal Build

```bash
gn gen out/Minimal --args='
  is_debug=false
  pdf_enable_v8=false
  pdf_enable_xfa=false
  pdf_enable_forms=false
  pdf_enable_edit=false
  use_clang_modules=false
  use_thin_lto=true
  optimize_for_speed=true
  symbol_level=0
'

ninja -C out/Minimal pdfium_cli
```

### Step 3: Verify Nothing Breaks

```bash
# Smoke tests
PDFIUM_CLI=out/Minimal/pdfium_cli pytest -m smoke -q
# Expected: 67/67 pass

# Full suite
PDFIUM_CLI=out/Minimal/pdfium_cli pytest -q
# Expected: 2,751/2,751 pass

# If ANY test fails: Identify what feature is needed, don't strip it
```

### Step 4: Measure Binary Size Impact

```bash
ls -lh out/Release/pdfium_cli out/Minimal/pdfium_cli
# Calculate reduction %

# Also check linked library size
otool -L out/Minimal/pdfium_cli
# See what's actually linked
```

### Step 5: Measure Performance Impact

**Theory**: Smaller binary = better cache locality = faster

**Test on 50+ PDFs**:
```bash
# Release build
time_50_pdfs_release > release.txt

# Minimal build
time_50_pdfs_minimal > minimal.txt

# Analyze
python compare_performance.py release.txt minimal.txt
```

**Expected impact**:
- Best case: +10-15% (significant code removed)
- Realistic: +3-7% (most removed code rarely executed)
- Worst case: 0% (removed code wasn't in hot path)

**Decision threshold**: If <+5%, not worth maintaining separate build config

---

## Codec Usage Analysis

### Task: Check Which Codecs Are Actually Used

```bash
cd ~/pdfium_fast

# Create test script
cat > check_codec_usage.sh << 'EOF'
#!/bin/bash
for pdf in integration_tests/pdfs/*/*.pdf; do
    # Use pdfinfo or similar
    strings $pdf | grep -E "Filter|/FlateDecode|/DCTDecode|/JBIG2Decode|/JPXDecode|/CCITTFaxDecode" >> codec_usage.txt
done

# Analyze
echo "JPEG (DCTDecode):" && grep -c DCTDecode codec_usage.txt
echo "JBIG2:" && grep -c JBIG2Decode codec_usage.txt
echo "JPEG2000 (JPX):" && grep -c JPXDecode codec_usage.txt
echo "Fax (CCITT):" && grep -c CCITTFaxDecode codec_usage.txt
EOF

chmod +x check_codec_usage.sh
./check_codec_usage.sh
```

**Decision**:
- If codec count = 0: Strip from build (save binary size, faster linking)
- If codec count > 0: Must keep (needed for correctness)

---

## Expected Gains from Stripping

**Realistic expectations** (not optimistic):

**Binary size**: -20-40% (forms, editing, signatures, annotations)
- Before: ~5MB
- After: ~3-3.5MB
- Benefit: Faster cold start, better instruction cache

**Linking time**: -10-20% (fewer object files)
- Before: ~60 seconds
- After: ~48 seconds
- Benefit: Faster iteration during development

**Performance**: +3-8% (realistic)
- Best case: +8% (removed code was in hot path)
- Likely: +5% (better cache locality)
- Worst case: +0% (removed code never executed)

**NOT expecting**: +50% or dramatic gains
**Reasoning**: Removed features mostly in initialization, not render loop

---

## Validation Requirements

**Correctness** (non-negotiable):
```bash
# Full test suite with minimal build
PDFIUM_CLI=out/Minimal/pdfium_cli pytest -q
# MUST: 2,751/2,751 pass (100%)

# If any test fails:
1. Identify what feature is needed
2. Re-enable that feature
3. Re-test
4. Only strip what doesn't break tests
```

**Performance** (measure, don't assume):
```bash
# Benchmark on 100+ PDFs
# Calculate mean speedup with 95% CI
# Report per-category (does stripping help text-heavy vs image-heavy?)

# Only claim gain if:
# - Mean > 1.05x (5% minimum)
# - 95% CI lower bound > 1.03x
# - No category shows regression
```

---

## Step-by-Step Task for Worker

**N=233**: Analyze codec usage in corpus
- Which codecs are actually used?
- Can we strip JBIG2, JPX, CCITTFax?

**N=234**: Create minimal build config
- Disable forms, edit, save, annotations
- Build and test

**N=235**: Validate correctness
- Full test suite with minimal build
- Fix any breakage

**N=236**: Measure performance
- 50+ PDFs, before/after
- Report actual gain (not expected)

**N=237**: Decision
- IF gain ≥5% AND 100% tests pass: Keep minimal build
- IF gain <5%: Revert (not worth complexity)
- IF tests fail: Re-enable needed features

---

## Bottom Line - No Optimism

**Expected**: +3-7% performance from stripping (realistic)
**Measure**: On 50+ PDFs, 95% confidence interval
**Validate**: 100% test pass rate maintained
**Decision**: Data-driven (not hopeful thinking)

**Worker should be skeptical** and prove every claim with measurements.
