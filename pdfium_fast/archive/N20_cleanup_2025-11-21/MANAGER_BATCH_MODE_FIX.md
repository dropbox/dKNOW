# MANAGER: Batch Mode Documentation Bug

**Issue:** Batch mode EXISTS (v1.6.0, N=617) but NOT shown in --help

## Problem

**Batch mode works:**
```bash
./pdfium_cli --batch extract-text /directory/ /output/
./pdfium_cli --batch --pattern "report_*.pdf" extract-text /dir/ /out/
./pdfium_cli --batch --recursive extract-text /dir/ /out/
```

**But --help doesn't show these flags:**
- --batch (missing)
- --pattern (missing)
- --recursive (missing)

**User feedback PR #17 asked for batch mode (Priority 3)**

**Reality:** Feature already exists, just needs documentation!

---

## Fix Required (Add to Path B)

**Between JPEG and error messages (N=46):**

### Add Batch Flags to Help Text

**File:** examples/pdfium_cli.cpp (around line 543-560)

**Add after other flags:**
```cpp
fprintf(stderr, "  --batch           Process directory of PDFs\n");
fprintf(stderr, "  --pattern GLOB    File pattern for batch (default: *.pdf)\n");
fprintf(stderr, "  --recursive       Recursive directory search\n");
```

**Add batch examples:**
```cpp
fprintf(stderr, "\nBatch Processing:\n");
fprintf(stderr, "  ./pdfium_cli --batch extract-text /pdfs/ /output/\n");
fprintf(stderr, "  ./pdfium_cli --batch --pattern \"report_*.pdf\" extract-text /docs/ /out/\n");
fprintf(stderr, "  ./pdfium_cli --batch --recursive extract-text /archive/ /extracted/\n");
```

**Commit:**
```
[WORKER0] # 46: Document Batch Mode in Help Text

Batch mode exists (v1.6.0, N=617) but was missing from --help.

Added to help:
- --batch flag
- --pattern flag
- --recursive flag
- Batch processing examples

User feedback (PR #17) asked for this. Already implemented, just needed docs!
```

---

## User Feedback Response

**PR #17 Priority 3: "Batch Mode (4-6 hours)"**

**Response:** "Batch mode already exists in v1.6.0! Just needed documentation (15 minutes)."

---

## Add to Path B Execution

**Updated Path B order:**
- N=41-42: UTF-8 output
- N=43-45: JPEG output (CRITICAL)
- N=46: **Document batch mode** (15 min) ← INSERT HERE
- N=47-49: Better error messages
- N=50: User README
- N=51-56: Linux binaries
- N=57-67: Python bindings
- N=68-72: Cross-platform validation

**Total Path B: Still ~30 commits**

---

**Worker: Add this to your Path B checklist after JPEG implementation.**
