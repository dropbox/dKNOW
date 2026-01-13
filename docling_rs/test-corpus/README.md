# Test Corpus

**Location:** `test-corpus/` (git-ignored, local only)

## Current Contents

### PDF - Original Docling (15 files)
**Directory:** `test-corpus/pdf/`

Original test PDFs from Python docling with ground truth:
- `2203.01017v2.pdf` - Academic paper
- `2206.01062.pdf` - Academic paper
- `2305.03393v1.pdf` - Academic paper (full)
- `2305.03393v1-pg9.pdf` - Academic paper (page 9)
- `amt_handbook_sample.pdf` - Handbook sample
- `code_and_formula.pdf` - Code and mathematical formulas
- `edinet_sample.pdf` - Japanese financial report (EDINET)
- `jfk_scanned.pdf` - Scanned document (OCR test)
- `multi_page.pdf` - Multi-page document
- `picture_classification.pdf` - Image classification doc
- `redp5110_sampled.pdf` - IBM Redbook sample
- `right_to_left_01.pdf` - RTL language (Hebrew/Arabic)
- `right_to_left_02.pdf` - RTL language
- `right_to_left_03.pdf` - RTL language
- `test_complex_table.pdf` - Complex table structures

**Has groundtruth:** `test-corpus/groundtruth/docling_v2/`
**Source:** Python docling test suite

### PDF - A_Categorizer Real (162 files, ~276MB)
**Directory:** `test-corpus/pdf-acategorizer/`

Real-world documents from a_categorizer integration tests:
- Arxiv research papers (~30)
- Business documents (Apple product pages, sales ebooks)
- EDINET Japanese financial reports (~25)
- McKinsey presentations
- Tax/legal forms (1099s, client intakes)
- Various commercial PDFs

**Source:** `~/a_categorizer/pdf_extractor/tests/fixtures/real/`

### PDF - Synthetic (129 files)
**Directory:** `test-corpus/pdf-synthetic/`

Synthetic test PDFs with controlled content:
- Multi-column layouts
- Forms and tables
- Code blocks and technical specs
- Multi-language (English, Japanese, Thai)
- Nested structures
- Edge cases

**Source:** `~/a_categorizer/pdf_extractor/tests/fixtures/synthetic/`

**Total PDFs: 306 (15 + 162 + 129)**

### DOCX - Original Docling (14 files)
**Directory:** `test-corpus/docx/`

Original test DOCX files from Python docling with ground truth:
- `drawingml.docx` - DrawingML graphics
- `equations.docx` - Mathematical equations
- `lorem_ipsum.docx` - Basic text content
- `table_with_equations.docx` - Tables with math
- `tablecell.docx` - Table cell formatting
- `test_emf_docx.docx` - EMF image format
- `textbox.docx` - Text boxes
- `unit_test_formatting.docx` - Text formatting (bold, italic, etc.)
- `unit_test_headers_numbered.docx` - Numbered headers
- `unit_test_headers.docx` - Header styles
- `unit_test_lists.docx` - Bullet and numbered lists
- `word_image_anchors.docx` - Image anchoring
- `word_sample.docx` - General Word document
- `word_tables.docx` - Table structures

**Has groundtruth:** `test-corpus/groundtruth/docling_v2/`
**Source:** Python docling test suite

### PPTX - Original Docling (3 files)
**Directory:** `test-corpus/pptx/`

Original test PowerPoint files from Python docling with ground truth:
- `powerpoint_bad_text.pptx` - Edge case text handling
- `powerpoint_sample.pptx` - General presentation
- `powerpoint_with_image.pptx` - Slides with images

**Has groundtruth:** `test-corpus/groundtruth/docling_v2/`
**Source:** Python docling test suite

### XLSX - Original Docling (3 files)
**Directory:** `test-corpus/xlsx/`

Original test Excel files from Python docling with ground truth:
- `xlsx_01.xlsx` - Basic spreadsheet
- `xlsx_02_sample_sales_data.xlsm` - Macro-enabled workbook with data
- `xlsx_03_chartsheet.xlsx` - Spreadsheet with charts

**Has groundtruth:** `test-corpus/groundtruth/docling_v2/`
**Source:** Python docling test suite

### HTML, Markdown, CSV, etc.
**Directories:** `test-corpus/{html,md,csv,asciidoc,jats,webvtt}/`

Various format test files from Python docling test suite with ground truth.

**Has groundtruth:** `test-corpus/groundtruth/docling_v2/`
**Source:** Python docling test suite

### Microsoft Visio (.vsdx) - 5 files
**Directory:** `test-corpus/microsoft-visio/`

VSDX diagram files for testing Visio format support:
- `colors_diagram.vsdx` - Color usage diagram (15K)
- `hr_recruiting_flowchart.vsdx` - HR recruiting workflow (35K)
- `sample_diagram.vsdx` - Basic diagram (1.4K)
- `shape_properties.vsdx` - Shape properties demo (18K)
- `shapes_and_lines.vsdx` - Basic shapes and connectors (22K)

**Source:** github.com/dave-howard/vsdx, github.com/jgraph/drawio-diagrams

### LaTeX (.tex) - 13 files
**Directory:** `test-corpus/latex/`

LaTeX document templates and examples:
- `academic_cv.tex` - Academic CV (AltaCV template)
- `beamer_slides.tex` - Presentation slides
- `bibliography.tex` - Bibliography example
- `book_template.tex` - Book/thesis template
- `business_letter.tex` - Business letter (Awesome-CV)
- `complex_formatting.tex` - Complex formatting
- `cv_template.tex` - Modern CV template
- `equations.tex` - Mathematical equations
- `neurips_template.tex` - NeurIPS conference paper
- `resume_template.tex` - Resume template
- `sample_arxiv.tex` - arXiv paper sample
- `simple_document.tex` - Simple document
- `thesis_template.tex` - PhD thesis template

**Source:** Various GitHub LaTeX template repositories

### Microsoft Publisher (.pub) - 0 files
**Directory:** `test-corpus/publisher/`

**Status:** No valid .pub files found. Microsoft Publisher files are extremely rare on GitHub and public repositories. See `reports/feature-phase-e-open-standards/test_files_download_*.md` for details on download attempts.

## Regenerating Test Corpus

```bash
# Copy PDFs from a_categorizer
cp ~/a_categorizer/pdf_extractor/tests/fixtures/real/*.pdf test-corpus/pdf/

# Copy from Python docling tests (if needed)
# cp ~/src/worktrees/docling-python/tests/data/* test-corpus/
```

## Note
Test corpus is intentionally not tracked in git to keep repository size small.
Each developer should set up their own local test corpus.
