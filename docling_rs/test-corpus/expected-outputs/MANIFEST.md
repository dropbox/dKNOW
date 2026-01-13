# Expected Outputs Generation Manifest

## ⚠️  VERSION MISMATCH ALERT

**Expected outputs were generated with docling 2.55.0**
**Current system has docling 2.57.0**

**Impact:** 112 PDF OCR tests fail due to output differences between versions.

---

## Generation Metadata

**Generated:** 2025-10-21 07:06:27 PST
**Docling Version:** 2.55.0 (OUTDATED - needs regeneration with 2.57.0)
**Script:** `scripts/initialize_integration_tests.py`
**Commit:** 2893b70
**Duration:** ~8 hours (612 files × 2 modes)

## Configuration Used

```python
converter = DocumentConverter()  # Default settings
```

**Default settings in docling 2.55.0:**
- `do_ocr`: True (default)
- `do_table_structure`: True (default)
- `ocr_engine`: auto
- `accelerator`: mps (Apple Silicon)

## Files Generated

| Directory | Files | Formats |
|-----------|-------|---------|
| pdf/ | 24 | .md, .text-only.md |
| pdf-more/ | 582 | .md, .text-only.md |
| docx-more/ | 46 | .md |
| html-more/ | 24 | .md |
| pptx-more/ | 50 | .md |
| xlsx-more/ | 50 | .md |
| png-more/ | 48 | .md |
| jpeg-more/ | 7 | .md |
| **Total** | **831** | Markdown only |

## Known Issues

### Version Mismatch (CRITICAL)
- Expected outputs: docling 2.55.0
- Current tests: docling 2.57.0
- Impact: 112 PDF tests fail (14% failure rate)
- Fix: Regenerate with `python scripts/regenerate_expected_outputs.py`

### Missing Formats
- Only Markdown format generated
- JSON, HTML, text, doctags not generated
- Future: Generate all formats for comprehensive validation

## Test Results With Current Manifest

**With docling 2.57.0 (version mismatch):**
- 779/902 tests pass (86%)
- 123 tests fail:
  - 112 PDF output mismatches (version difference)
  - 7 HTML recursion errors (genuinely broken)
  - 4 large images (DecompressionBomb)

**Expected after regeneration with 2.57.0:**
- ~890/902 tests pass (99%)
- Only genuinely broken tests fail

## Regeneration Instructions

```bash
# 1. Verify docling version
python3 -c "import docling; print(docling.__version__)"
# Should output: 2.57.0

# 2. Regenerate all expected outputs
python scripts/regenerate_expected_outputs.py
# Time: ~1-2 hours
# Generates: ~600 OCR outputs with docling 2.57.0

# 3. Update this MANIFEST
# Edit MANIFEST.md with new generation date, version, settings

# 4. Run tests
cargo test -p docling-core --test integration_tests
# Expected: 99% pass rate
```

## Future: Automated Version Checking

**Planned enhancement:**
- Tests validate docling version matches MANIFEST on startup
- Fail fast with clear error if version mismatch
- Include version in CSV logs
- Track settings hash for configuration validation

See: `TEST_ENVIRONMENT_TRACKING.md` for implementation plan
