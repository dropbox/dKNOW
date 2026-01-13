# DocItem Test Coverage Report (N=1356)

**Date:** 2025-11-18
**Branch:** feature/phase-e-open-standards

---

## Summary

**Progress:** 49/65 → 53/65 formats with DocItem validation tests (82%)

**Added in this session:**
- IDML (Adobe InDesign Markup Language)
- KEY (Apple Keynote)
- NUMBERS (Apple Numbers)
- XPS (XML Paper Specification)

---

## Current Coverage: 53/65 Formats (82%)

### Formats WITH DocItem Tests (53)

**Documents (6):**
- DOCX ✅
- DOC ✅
- ODT ✅
- RTF ✅
- PAGES ✅
- TEX ✅

**Presentations (2):**
- PPTX ✅
- ODP ✅

**Spreadsheets (3):**
- XLSX ✅
- ODS ✅
- CSV ✅

**Markup/Web (5):**
- HTML ✅
- MD (Markdown) ✅
- ASCIIDOC ✅
- JATS ✅
- SVG ✅

**Email (3):**
- EML ✅
- MBOX ✅
- VCF ✅

**Ebooks (3):**
- EPUB ✅
- MOBI ✅
- FB2 ✅

**Archives (4):**
- ZIP ✅
- TAR ✅
- 7Z ✅
- RAR ✅

**Web Media (2):**
- WEBVTT ✅
- SRT ✅

**Notebooks (1):**
- IPYNB ✅

**Images (9):**
- PNG ✅
- JPEG ✅
- TIFF ✅
- WEBP ✅
- BMP ✅
- GIF ✅
- HEIF ✅
- AVIF ✅
- DICOM ✅

**CAD/3D (5):**
- DXF ✅
- STL ✅
- OBJ ✅
- GLTF ✅
- GLB ✅

**GPS (3):**
- GPX ✅
- KML ✅
- KMZ ✅

**Microsoft Extended (2):**
- VSDX (Visio) ✅
- MPP (Project) ✅

**Calendar (1):**
- ICS ✅

**Adobe (1):**
- IDML ✅ **(NEW)**

**Apple (2):**
- KEY (Keynote) ✅ **(NEW)**
- NUMBERS ✅ **(NEW)**

**Document Formats (1):**
- XPS ✅ **(NEW)**

---

## Formats WITHOUT DocItem Tests (12/65)

### Out of Scope (8 formats)

**PDF (1):** OUT OF SCOPE
- Requires 5-6 ML models (layout, tableformer, OCR)
- Separate strategic initiative
- Current pdfium-based backend acceptable as-is

**Audio/Video (6):** OUT OF SCOPE
- AVI, MKV, MOV, MP3, MP4, WAV
- Handled by separate media processing system

**Database (1):** OUT OF SCOPE
- MDB (Microsoft Access)
- Requires database query engine

### Native Format (1 format)

**JSONDocling:** No test needed
- Native Docling JSON format
- 100% complete by definition (ground truth format)
- Testing would be circular (deserialize JSON → serialize to JSON)

### Missing Test Files (1 format)

**MSG (Outlook Message):** Cannot test
- EmailBackend supports MSG format ✅
- No test files in corpus ❌
- README exists with instructions to create test files
- Would need Microsoft Outlook to create .msg files

### No Backend Implementation (2 formats)

**PUB (Microsoft Publisher):** Cannot test
- PublisherBackend exists but only has `convert_to_pdf()` method
- No `parse_file()` or DocItem generation
- Uses LibreOffice conversion workflow

**ONE (Microsoft OneNote):** Cannot test
- OneNoteBackend exists but only has `parse_error()` method
- Desktop .one format unsupported by Rust libraries
- Returns error message explaining limitation

---

## Quality Scores (Known)

**Perfect (100%):**
- CSV: 100%
- DOCX: 100%

**Excellent (98%):**
- XLSX: 98%
- PPTX: 98%

**Pending (49 formats):**
- Tests exist, need to run LLM validation
- Estimated cost: ~$1.00 (49 tests × $0.02 each)
- Estimated time: ~15 minutes

---

## Next Steps

### Immediate Actions

1. **Run all 53 DocItem tests** (~$1.00, 15 min)
   ```bash
   source .env
   cargo test llm_docitem -- --nocapture --test-threads=4
   ```

2. **Document quality scores**
   - Create quality matrix
   - Identify formats needing fixes
   - Prioritize by usage/importance

3. **Fix quality gaps**
   - Target: 95%+ for all tested formats
   - Focus on high-usage formats first

### Future Work

**Create MSG Test Files:**
- Requires Microsoft Outlook
- Follow instructions in test-corpus/email/msg/README.md
- 5 test files planned (simple, HTML, attachments, meeting, thread)

**Extend PUB Backend:**
- Add `parse_file()` method
- Generate DocItems (not just convert to PDF)
- May require libmspub FFI or OLE parsing

**Monitor OneNote Library:**
- Track onenote.rs development
- Wait for desktop format support (v0.4.0+)
- Add test when library matures

---

## Technical Notes

### Apple Formats (KEY, NUMBERS)
- Use `.parse(Path::new(...))` not `.parse_file(...)`
- Implemented in `docling-apple` crate
- Work with .key/.numbers package formats

### IDML Format
- Adobe InDesign Markup Language
- XML-based format
- Test files in test-corpus/adobe/idml/

### XPS Format
- XML Paper Specification (Microsoft)
- PDF alternative
- Test files in test-corpus/xps/

---

## Statistics

**Total Formats:** 65
**With Tests:** 53 (82%)
**Without Tests:** 12 (18%)
**Out of Scope:** 8 (12%)
**Cannot Test (technical):** 3 (5%)
**No Test Needed:** 1 (2%)

**Effective Coverage:** 53 / (65 - 8 out of scope - 1 native format) = **53/56 = 95%**

---

## Conclusion

**Status:** ✅ Excellent coverage

DocItem test coverage is now at 82% (53/65 formats), or **95% of testable formats** (53/56).

All in-scope formats with working backends and test files now have LLM validation tests.

Next: Run all tests, document quality scores, fix gaps to 95%+.
