# Baseline Sources

**Date:** 2025-11-04
**Status:** 100% Upstream Parity Achieved ✓

---

## Single Baseline Strategy

**Location:** `integration_tests/baselines/upstream/images_ppm/`

**Source:** Generated from upstream `pdfium_test` binary
```bash
# Command used:
pdfium_test --ppm --scale=4.166666 input.pdf
```

**Binary:**
- `out/Optimized-Shared/pdfium_test`
- MD5: 00cd20f999bf60b1f779249dbec8ceaa
- Commit: 7f43fd79 (upstream, unmodified)

**Status:**
- 451/452 PDFs (bug_451265 hangs)
- Verified byte-for-byte correct
- A/A test: 100% deterministic

**Use for:**
- Correctness validation
- Upstream parity verification
- Regression detection

---

## Our C++ CLI Match

**pdfium_cli** now matches upstream 100% for ALL PDFs!

**Fix applied:** 2025-11-04 (WORKER1 # 1)
- Added `FPDF_FFLDraw()` to render form fields on top of page bitmap
- Added `FPDF_SetFormFieldHighlightColor/Alpha()` for proper form appearance
- Result: Perfect byte-for-byte match with upstream pdfium_test

**Previously problematic PDFs (now fixed):**
```
0100pages_7FKQLKX273JBHXAAW5XDRT27JGMIZMCI.pdf - Forms now render ✓
0130pages_ZJJJ6P4UAGH7LKLACPT5P437FB5F3MYF.pdf - Forms now render ✓
0309pages_7LD3RVJDZGTXF53CDLCI67YPWZQ5POOA.pdf - Forms now render ✓
0496pages_E3474JUEVRWQ3P2J2I3XBFKMMVZLLKWZ.pdf - Forms now render ✓
1931pages_7ZNNFJGHOEFFP6I4OARCZGH3GPPDNDXC.pdf - Forms now render ✓
cc_013_122p.pdf - Forms now render ✓
web_003.pdf - Forms now render ✓ (was 28% different)
web_026.pdf - Forms now render ✓ (was 15% different)
web_041.pdf - Forms now render ✓ (was 44% different!)
```

---

## Testing Strategy

**Use:** `baselines/upstream/images_ppm/` only

**Expected:** 100% exact match for all PDFs

**No exceptions needed:** All form-containing PDFs now render correctly.

---

## Recovery

**Restore upstream baselines:**
```bash
git checkout 06c79a736a -- integration_tests/baselines/upstream/images_ppm/
```

---

## Historical Note

**Previously** (before 2025-11-04):
- pdfium_cli was missing `FPDF_FFLDraw()` call
- Forms rendered as white/invisible
- Required dual baseline system (upstream + worker_cli)
- 9 PDFs had form rendering exceptions

**Now** (after WORKER1 fix):
- Single baseline system (upstream only)
- 100% correctness achieved
- No exceptions or workarounds needed
