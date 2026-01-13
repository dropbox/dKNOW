# Benchmark PDFs

**196 normal PDFs for performance and correctness testing.**

---

## Sources

### arxiv (39 PDFs)
- Academic papers, LaTeX-generated
- Examples: arxiv_001.pdf through arxiv_040.pdf
- Characteristics: Text-heavy, mathematical notation, figures
- Page range: 4-80 pages

### edinet (50 PDFs)
- Japanese corporate filings (EDINET system)
- Examples: edinet_2025-06-26_0914_E01057_SOFT99corporation.pdf
- Characteristics: Japanese text (CJK), tables, multi-language
- Page range: 2-200 pages

### web (45 PDFs)
- Web-converted documents
- Examples: web_001.pdf through web_045.pdf
- Characteristics: Mixed content, varied formatting
- Page range: 1-300 pages

### cc (20 PDFs - CommonCrawl)
- Legal/corporate documents from CommonCrawl
- Examples: cc_001_931p.pdf, cc_008_116p.pdf
- Characteristics: Large documents, dense text
- Page range: 101-931 pages

### XXXpages_ (25 PDFs)
- Various sizes with page count in filename
- Examples: 0100pages_*, 0821pages_*, 1931pages_*
- Characteristics: Varied content
- Page range: 100-1931 pages

### other (17 PDFs)
- japanese_*.pdf (Japanese-specific tests)
- fax_*.pdf (fax-like documents)
- Miscellaneous test cases

---

## Size Distribution

- **Small** (10-50 pages): 40 PDFs
- **Medium** (50-200 pages): 100 PDFs
- **Large** (200+ pages): 56 PDFs

**Largest:** 1931pages_7ZNNFJGHOEFFP6I4OARCZGH3GPPDNDXC.pdf (1,931 pages)

---

## Content Types

- **Text-heavy:** Dense paragraphs, minimal images
- **Image-heavy:** Many figures, photos, diagrams
- **Mixed:** Balanced text and images
- **Multi-language:** English, Japanese, CJK

---

## Used By Tests

- **test_002_text_correctness.py** - 60 curated PDFs
- **test_003_extended_corpus.py** - All 196 PDFs
- **test_005_image_correctness.py** - All 196 PDFs
- **test_007_performance.py** - 3 large PDFs (100p, 116p, 821p)
- **test_008_scaling.py** - 3 large PDFs

---

## Purpose

Provides comprehensive coverage of:
- Various document types
- Various content types
- Various languages
- Various sizes

**This corpus validates that parallel optimizations work correctly across diverse real-world documents.**
