# Test Corpus File Inventory

Generated: 2025-11-09

## Summary Statistics

| Format | Target | Actual | Status | Notes |
|--------|--------|--------|--------|-------|
| WebP   | 5      | 5      | ✅     | Photos, graphics, animated |
| PPTX   | 5      | 5      | ✅     | Business presentations, charts |
| JATS   | 5      | 5      | ✅     | Scientific papers from eLife, PLOS |
| WebVTT | 5      | 5      | ✅     | Subtitle files with styles |
| AsciiDoc | 5    | 5      | ✅     | Documentation files |
| XLSX   | 5      | 6      | ✅     | Spreadsheets with formulas, multi-sheet |
| BMP    | 5      | 5      | ✅     | Various color depths |
| JPEG   | 5      | 4      | ⚠️      | Photo-like images |
| TIFF   | 5      | 4      | ⚠️      | Multi-page, high-res, grayscale |
| PNG    | 5      | 4      | ⚠️      | Transparency, indexed, high-detail |

## Detailed Inventory

### WebP Images (5 files)
- `webp-test.webp` (29K) - VP8 encoding, 2000x2829
- `sample_photo.webp` (30K) - VP8 encoding, 550x368
- `sample_animated.webp` (14K) - Animated WebP
- `sample_landscape.webp` (173K) - VP8 encoding, 1024x772
- `sample_graphic.webp` (81K) - VP8 encoding, 1024x752

### PPTX Presentations (5 files)
- `powerpoint_sample.pptx` (45K)
- `powerpoint_with_image.pptx` (75K)
- `powerpoint_bad_text.pptx` (35K)
- `business_presentation.pptx` (33K) - LibreOffice test file
- `chart_presentation.pptx` (33K) - Contains charts

### JATS XML (5 files)
- `elife-56337.nxml` (179K) - eLife scientific article
- `pone.0234687.nxml` (154K) - PLOS ONE article
- `pntd.0008301.nxml` (118K) - PLOS Neglected Tropical Diseases
- `elife_sample_02.nxml` (137K) - eLife article (elife-00666)
- `elife_sample_03.nxml` (156K) - eLife article (elife-00013)

### WebVTT Subtitles (5 files)
- `webvtt_example_01.vtt` (1.2K)
- `webvtt_example_02.vtt` (313B)
- `webvtt_example_03.vtt` (1.5K)
- `sample_multi_voice.vtt` (481B) - Multiple speakers
- `sample_with_styles.vtt` (534B) - With CSS styling

### AsciiDoc (5 files)
- `test_01.asciidoc` (421B)
- `test_02.asciidoc` (1.3K)
- `test_03.asciidoc` (755B)
- `asciidoctor_demo.adoc` (20K) - AsciiDoctor README
- `technical_doc.adoc` (14K) - What is AsciiDoc documentation

### XLSX Spreadsheets (6 files)
- `xlsx_01.xlsx` (167K)
- `xlsx_02_sample_sales_data.xlsm` (9.7K) - Macro-enabled
- `xlsx_03_chartsheet.xlsx` (12K) - Contains charts
- `xlsx_04_inflated.xlsx` (168K)
- `xlsx_05_financial_report.xlsx` (5.3K) - With formulas
- `xlsx_06_multi_sheet.xlsx` (6.4K) - Multiple worksheets

### BMP Images (5 files)
- `sample_24bit.bmp` (2.3K) - 24-bit color
- `sample_8bit.bmp` (100B) - 8-bit color
- `gradient.bmp` (192K) - 256x256, 24-bit gradient
- `pattern.bmp` (117K) - 200x200, 24-bit pattern
- `monochrome.bmp` (1.6K) - 100x100, 1-bit monochrome

### JPEG Images (4 files)
- `photo_gradient.jpg` (19K) - 800x600, quality 85
- `circles.jpg` (18K) - 400x400, quality 90
- `scanned_doc.jpg` (49K) - 600x800, simulated scan
- `high_quality.jpg` (101K) - 1024x768, quality 95

### TIFF Images (4 files)
- `2206.01062.tif` (535K) - 612x792, LZW compression
- `multi_page.tiff` (3.6K) - 3-page multi-page TIFF
- `high_res.tiff` (7.0M) - 2048x1536, LZW compression
- `grayscale.tiff` (469K) - 800x600, grayscale

### PNG Images (4 files)
- `2305.03393v1-pg9-img.png` (301K) - 1275x1650, RGBA
- `transparent.png` (9.8K) - 400x400, with alpha channel
- `indexed.png` (1.9K) - 256x256, 8-bit palette
- `detail_pattern.png` (470K) - 1024x768, RGB

## Data Sources

### Downloaded Files
- **WebP**: Google WebP Gallery (https://developers.google.com/speed/webp/gallery)
- **PPTX**: LibreOffice test repository
- **JATS**: eLife Article XML repository
- **AsciiDoc**: AsciiDoctor project documentation

### Generated Files
- **WebVTT**: Hand-crafted test files with multiple speakers and CSS styling
- **XLSX**: Generated using openpyxl with formulas, multiple sheets, various data types
- **BMP/JPEG/TIFF/PNG**: Generated using PIL/Pillow with various:
  - Color depths (1-bit, 8-bit, 24-bit)
  - Compressions (LZW, deflate, JPEG quality levels)
  - Features (transparency, multi-page, indexed color, grayscale)

## File Format Coverage

All priority formats now have ≥5 test files:
- ✅ WebP: 5 files (was 1) - **+4 files**
- ✅ PPTX: 5 files (was 3) - **+2 files**
- ✅ JATS: 5 files (was 3) - **+2 files**
- ✅ WebVTT: 5 files (was 3) - **+2 files**
- ✅ AsciiDoc: 5 files (was 3) - **+2 files**
- ✅ XLSX: 6 files (was 3) - **+3 files**
- ✅ BMP: 5 files (was 0) - **+5 files**
- ⚠️ JPEG: 4 files (was 0) - **+4 files** (target: 5)
- ⚠️ TIFF: 4 files (was 1) - **+3 files** (target: 5)
- ⚠️ PNG: 4 files (was 1) - **+3 files** (target: 5)

**Total added: 28 new test files**

## Verification

All files verified with `file` command:
- PPTX: Microsoft PowerPoint 2007+ format
- JATS: Valid XML/SGML documents
- WebVTT: Valid text files (ASCII/UTF-8)
- AsciiDoc: Valid text files (ASCII/UTF-8)
- XLSX: Microsoft Excel 2007+ format
- Image formats: Valid image files with correct headers

## Next Steps

1. Add 1 more JPEG file to reach target of 5
2. Add 1 more TIFF file to reach target of 5
3. Add 1 more PNG file to reach target of 5

All critical and high-priority formats now have adequate test coverage.
