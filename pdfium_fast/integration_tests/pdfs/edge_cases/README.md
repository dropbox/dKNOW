# Edge Case PDFs

**256 unusual/malformed PDFs for robustness testing.**

**Purpose:** Ensure PDFium doesn't crash on unusual input.

---

## Categories

### Encrypted PDFs (11 files)
```
encrypted.pdf
encrypted_hello_world_r2.pdf
encrypted_hello_world_r3.pdf
encrypted_hello_world_r5.pdf
encrypted_hello_world_r6.pdf
encrypted_hello_world_r2_bad_okey.pdf
encrypted_hello_world_r3_bad_okey.pdf
```

**Tests:** Various encryption standards (R2/R3/R5/R6), with/without passwords
**Expected:** Extraction may fail gracefully, but no crashes

---

### Corrupted/Malformed PDFs (10+ files)
```
bad_dict_keys.pdf
bad_annots_entry.pdf
bad_page_type.pdf
empty_xref.pdf
zero_length_stream.pdf
```

**Tests:** Invalid PDF structures, missing required entries
**Expected:** Graceful error handling, no crashes

---

### Bug Reproduction Cases (100+ files)
```
bug_113.pdf
bug_1139.pdf
bug_1206.pdf
bug_1029.pdf
bug_1055869.pdf
bug_1124998.pdf
...
```

**Tests:** PDFs from actual bug reports in PDFium issue tracker
**Expected:** Bugs are fixed, PDFs render correctly or fail gracefully

---

### Annotation Tests (50+ files)
```
annotation_fileattachment.pdf
annotation_highlight_*.pdf
annotation_ink_multiple.pdf
annotation_markup_*.pdf
annotation_stamp_with_ap.pdf
```

**Tests:** Various PDF annotation types and edge cases
**Expected:** Annotations render correctly

---

### Special Cases (50+ files)
```
about_blank.pdf          - Empty/blank page
black.pdf                - Solid black page
bigtable_mini.pdf        - Complex tables
bookmarks_circular.pdf   - Circular bookmark references
```

**Tests:** Unusual but valid PDF structures
**Expected:** Handle correctly without crashes

---

## Test Strategy

**Goal:** Must NOT crash or hang

**Success criteria:**
- No segfaults
- No infinite loops
- No unhandled exceptions
- Graceful failures OK (e.g., encrypted PDFs may fail extraction)

---

## Used By

- **test_004_edge_cases.py** - All 256 PDFs × 2 tests = 512 tests
  - `test_edge_case_text_no_crash` (256 PDFs)
  - `test_edge_case_image_no_crash` (256 PDFs) **[CRASHES]**

---

## File Size

**Total:** ~4MB (256 files)
**Average:** ~16KB per file (most are small)

Most edge case PDFs are small because they test specific malformed structures, not large document handling.

---

## Summary

256 PDFs covering:
- All encryption types
- Common corruption patterns
- Known bug cases
- Annotation edge cases
- Special/unusual structures

**Validates robustness: PDFium must handle ANY input safely.**
