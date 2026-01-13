# 🚨 MANAGER ORDER: Image Validation REQUIRED NOW

**From**: MANAGER
**To**: WORKER0
**Priority**: CRITICAL - TOP PRIORITY
**Status**: BLOCKING - Must complete before any other work

---

## ORDER

**YOU MUST validate images against upstream pdfium_test.**

This is the ONLY component without upstream validation.

---

## WHY THIS IS CRITICAL

**Current state**:
- Text: ✅ 100% validated vs upstream
- JSONL: ✅ 95% validated vs upstream
- Images: ❌ **0% validated** (self-consistency only)

**Problem**: We test "does output change" not "is output correct"

**Risk**: Could have rendering bugs that all our tests miss

---

## YOUR TASK (Execute Immediately)

### Step 1: Create Validation Script (30 min)

**File**: `lib/validate_images_vs_upstream.py`

```python
#!/usr/bin/env python3
"""
Validate image rendering against upstream pdfium_test

Compares our PNG renders vs upstream pdfium_test .ppm/.png renders
"""

import subprocess
import tempfile
from pathlib import Path
import os
import hashlib

pdfium_root = Path(__file__).parent.parent.parent
pdfium_test = pdfium_root / 'out' / 'Optimized-Shared' / 'pdfium_test'
render_pages = pdfium_root / 'rust' / 'target' / 'release' / 'examples' / 'render_pages'

# Test PDFs - 50 representative samples
TEST_PDFS = [
    # Arxiv (10)
    'arxiv_001.pdf', 'arxiv_004.pdf', 'arxiv_010.pdf', 'arxiv_015.pdf', 'arxiv_020.pdf',
    'arxiv_025.pdf', 'arxiv_030.pdf', 'arxiv_035.pdf', 'arxiv_038.pdf', 'arxiv_040.pdf',

    # CC (10)
    'cc_007_101p.pdf', 'cc_015_101p.pdf', 'cc_008_116p.pdf', 'cc_013_122p.pdf',
    'cc_009_188p.pdf', 'cc_010_206p.pdf', 'cc_001_931p.pdf', 'cc_002_522p.pdf',
    'cc_003_162p.pdf', 'cc_004_291p.pdf',

    # Edinet (10)
    'edinet_2025-06-24_1318_E01920_Makita Corporation.pdf',
    'edinet_2025-06-25_1608_E02628_KIMURATAN CORPORATION.pdf',
    # Add 8 more

    # Web (10)
    'web_005.pdf', 'web_011.pdf', 'web_007.pdf', 'web_014.pdf',
    # Add 6 more

    # Pages (10)
    '0100pages_7FKQLKX273JBHXAAW5XDRT27JGMIZMCI.pdf',
    # Add 9 more
]

def validate_pdf(pdf_name):
    pdf_path = pdfium_root / 'integration_tests' / 'pdfs' / 'benchmark' / pdf_name

    with tempfile.TemporaryDirectory() as upstream_dir:
        with tempfile.TemporaryDirectory() as ours_dir:
            env = os.environ.copy()
            env['DYLD_LIBRARY_PATH'] = str(pdfium_test.parent)

            # Generate upstream (pdfium_test creates .ppm files)
            os.chdir(upstream_dir)
            result = subprocess.run([str(pdfium_test), str(pdf_path)],
                                  capture_output=True, env=env, timeout=120)

            if result.returncode != 0:
                return {'pdf': pdf_name, 'status': 'upstream_failed', 'error': result.stderr}

            # Convert ppm to png (requires ImageMagick)
            ppm_files = list(Path(upstream_dir).glob('*.ppm'))
            for ppm in ppm_files:
                subprocess.run(['convert', str(ppm), str(ppm).replace('.ppm', '.png')],
                             check=True)

            # Generate ours
            subprocess.run([str(render_pages), str(pdf_path), ours_dir, '1', '300'],
                         capture_output=True, env=env, timeout=120, check=True)

            # Compare MD5 for each page
            matches = 0
            differs = 0

            upstream_pngs = sorted(Path(upstream_dir).glob('*.png'))
            our_pngs = sorted(Path(ours_dir).glob('*.png'))

            for up_png, our_png in zip(upstream_pngs, our_pngs):
                up_md5 = hashlib.md5(up_png.read_bytes()).hexdigest()
                our_md5 = hashlib.md5(our_png.read_bytes()).hexdigest()

                if up_md5 == our_md5:
                    matches += 1
                else:
                    differs += 1

            return {
                'pdf': pdf_name,
                'status': 'compared',
                'matches': matches,
                'differs': differs,
                'total_pages': len(upstream_pngs)
            }

# Run validation
results = []
for pdf in TEST_PDFS:
    result = validate_pdf(pdf)
    results.append(result)
    print(f"{pdf}: {result['status']}")

# Report
print(f"\n{'='*70}")
print(f"Image Validation Summary")
print(f"{'='*70}")
total_match = sum(r['matches'] for r in results if 'matches' in r)
total_differ = sum(r['differs'] for r in results if 'differs' in r)
print(f"Pages matched: {total_match}")
print(f"Pages differed: {total_differ}")
print(f"Match rate: {total_match/(total_match+total_differ)*100:.1f}%")
```

### Step 2: Run Validation (2-3 hours)

```bash
cd integration_tests
python lib/validate_images_vs_upstream.py > image_validation_results.txt 2>&1
```

### Step 3: Document Results (30 min)

Create `UPSTREAM_IMAGE_VALIDATION_RESULTS.md` with:
- Match percentage
- List of any differences
- SSIM analysis if needed
- Conclusion: Do images match upstream?

### Step 4: Commit (5 min)

```bash
git add -A
git commit -m "[WORKER0] # 53: Image Validation vs Upstream pdfium_test

**Validation**: Compared our renders vs upstream pdfium_test on 50 PDFs

**Results**:
- Pages matched: X/Y (Z%)
- Pages differed: N
- Analysis: <brief summary>

**Conclusion**: Images <do/do not> match upstream pdfium_test

See UPSTREAM_IMAGE_VALIDATION_RESULTS.md for full results."
```

---

## BLOCKING

**DO NOT do any other work until image validation is complete.**

This is the critical gap in our validation.

---

## Expected Timeline

- Script creation: 30 min
- Validation run: 2-3 hours
- Documentation: 30 min
- Commit: 5 min
- **Total: 3-4 hours**

---

## Questions?

Read: `PRECISE_VALIDATION_AUDIT.md` for full context

**This order is non-negotiable.** Images must be validated vs upstream.
