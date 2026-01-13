# Test PDFs

**All PDFs used for testing, organized by type.**

---

## Structure

```
pdfs/
├── benchmark/     196 PDFs (1.5GB)
└── edge_cases/    256 PDFs (4MB)
```

---

## benchmark/ (196 PDFs)

**Purpose:** Normal benchmark corpus for performance and correctness testing

**Sources:**
- **arxiv** (39 PDFs): Academic papers, LaTeX-generated
- **edinet** (50 PDFs): Japanese corporate filings
- **web** (45 PDFs): Web-converted documents
- **cc** (20 PDFs): CommonCrawl, legal documents
- **XXXpages_** (25 PDFs): Various page counts
- **other** (17 PDFs): japanese_*, fax_*, etc.

**Size range:** 10 pages to 1,931 pages
**Content:** Text-heavy, image-heavy, mixed, multi-language

**Used by tests:**
- test_002_text_correctness.py (60 PDFs)
- test_003_extended_corpus.py (all 196 PDFs)
- test_005_image_correctness.py (all 196 PDFs)
- test_007_performance.py (3 large PDFs)
- test_008_scaling.py (3 large PDFs)

---

## edge_cases/ (256 PDFs)

**Purpose:** Unusual/malformed PDFs for crash testing

**Categories:**
- **Encrypted** (11 PDFs): R2/R3/R5/R6 encryption, with/without passwords
- **Corrupted** (10+ PDFs): bad_dict_keys, bad_annots_entry, empty_xref
- **Bug cases** (100+ PDFs): bug_113, bug_1139, bug_1206, etc.
- **Annotations** (50+ PDFs): annotation_*, various annotation types
- **Special** (50+ PDFs): empty, blank, unusual structures

**Size:** Mostly small (< 100KB each)

**Used by tests:**
- test_004_edge_cases.py (all 256 PDFs × 2 = 512 tests)

**Test goal:** Must not crash or hang on unusual input

---

## Total

**452 PDFs** covering:
- Normal documents (benchmark)
- Unusual/malformed documents (edge_cases)
- Multi-language (English, Japanese, CJK)
- Various sources (academic, corporate, web, legal)
- Size range: 1KB to 500MB

**Everything needed for comprehensive testing.**
