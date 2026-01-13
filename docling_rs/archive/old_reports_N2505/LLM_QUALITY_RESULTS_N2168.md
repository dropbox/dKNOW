# LLM Quality Test Results - N=2168

## Summary

**Date:** 2025-11-24
**Test Run:** Quality verification of 90-94% formats
**Formats Improved:** 2 (GLB, AsciiDoc)
**New Total at 95%+:** 14 formats (was 12)

## Test Results

### ✅ Formats Now at 95%

| Format | Previous | Current | Finding |
|--------|----------|---------|---------|
| GLB    | 94%      | 95%     | [Minor] Extra newline at end of summary section |
| AsciiDoc | 93%    | 95%     | [Minor] Extra blank lines before/after lists |

### ❌ Formats Still Below 95%

| Format | Score | Category Issues | Findings |
|--------|-------|----------------|----------|
| GPX    | 92%   | Structure: 90/100 | [Major] Waypoints section doesn't clearly separate from routes/tracks |
| IPYNB  | 93%   | Structure: 90/100 | [Minor] Output doesn't maintain original cell separation with clear dividers |
| KML    | 93%   | Structure: 95/100, Formatting: 95/100 | [Minor] Doesn't preserve XML structure; coordinates have decimal point inconsistency |
| JATS   | 92%   | Accuracy: 90/100, Formatting: 90/100 | [Major] 'Zfp809' formatted as *Zfp809* instead of plain text; adjusted p-value uses *adjusted p-value* |
| OBJ    | 92%   | Structure: 90/100 | [Minor] Section titles don't match original document structure |
| VCF    | 90%   | Metadata: 80/100 | [Minor] Title/header format not preserved: original 'FN' vs output 'John Doe' |
| TAR    | 88%   | Accuracy: 90/100, Formatting: 90/100 | [Major] Total archive size reported as 392 bytes (individual files = 392 bytes, missing TAR overhead); [Minor] List formatting could use bullet points |
| ODS    | 85%   | Accuracy: 90/100, Formatting: 90/100 | [Major] Age values lack units/context; [Minor] Table lacks proper alignment and spacing |

## Analysis

### Categories of Issues

**1. Structural Separation (GPX, IPYNB)**
- GPX: Waypoints blended with routes/tracks without clear section breaks
- IPYNB: Cell boundaries not clearly marked

**2. Formatting Consistency (KML, JATS, VCF)**
- KML: Coordinate decimal formatting inconsistent with original
- JATS: Emphasis markup (*word*) appearing when shouldn't
- VCF: Header format changed from technical (FN) to display name

**3. Data Accuracy (TAR, ODS)**
- TAR: Archive size calculation missing TAR format overhead
- ODS: Missing units on numeric values (age)

**4. Table/List Presentation (TAR, ODS)**
- TAR: Contents list could use bullet formatting
- ODS: Table alignment and spacing needs improvement

### Patterns Observed

**Minor vs Major Issues:**
- Minor (90-95 in subcategory): Formatting, extra whitespace, structure variations
- Major (80-89 in subcategory): Data accuracy, missing context, significant format deviations

**Quick Wins (92-94% → 95%):**
- GLB ✅: Already fixed (score verified)
- AsciiDoc ✅: Already fixed (score verified)
- GPX, IPYNB, KML: Require backend structural improvements
- JATS, OBJ: Require serialization fixes

**Medium Effort (85-90% → 95%):**
- VCF: Header format preservation
- TAR: Size calculation + list formatting
- ODS: Units + table alignment

## Detailed Findings

### GLB (95% ✅)
```
Overall Score: 95.0%
Completeness: 100/100
Accuracy: 100/100
Structure: 100/100
Formatting: 95/100 ⚠️
Metadata: 100/100

[Minor] Formatting: The summary section has an extra newline at the end.
Location: Summary
```

### AsciiDoc (95% ✅)
```
Overall Score: 95.0%
Completeness: 100/100
Accuracy: 100/100
Structure: 100/100
Formatting: 95/100 ⚠️
Metadata: 100/100

[Minor] Formatting: Extra blank lines before and after lists and paragraphs.
Location: Section 1.1
```

### GPX (92%)
```
Overall Score: 92.0%
Completeness: 95/100
Accuracy: 95/100
Structure: 90/100 ⚠️
Formatting: 95/100
Metadata: 100/100

[Major] Structure: The waypoints section does not clearly separate the waypoints from the routes and tracks, which could confuse readers.
Location: Waypoints Section
```

### IPYNB (93%)
```
Overall Score: 93.0%
Completeness: 95/100
Accuracy: 95/100
Structure: 90/100 ⚠️
Formatting: 95/100
Metadata: 100/100

[Minor] Structure: The output does not maintain the original cell separation with clear dividers between cells.
Location: Between Cell 2 and Cell 3
```

### KML (93%)
```
Overall Score: 93.0%
Completeness: 100/100
Accuracy: 100/100
Structure: 95/100 ⚠️
Formatting: 95/100 ⚠️
Metadata: 100/100

[Minor] Structure: The output does not preserve the original XML structure, specifically the lack of explicit XML tags.
Location: Overall document structure

[Minor] Formatting: The coordinates are presented with a decimal point in the output, which is not consistent with the original format.
Location: Placemarks section
```

### JATS (92%)
```
Overall Score: 92.0%
Completeness: 95/100
Accuracy: 90/100 ⚠️
Formatting: 90/100 ⚠️
Structure: 95/100
Metadata: 100/100

[Major] Accuracy: The term 'Zfp809' is formatted differently as *Zfp809* in the actual output, which may impact the interpretation of the text.
Location: Introduction section

[Minor] Formatting: The formatting of the adjusted p-value in the actual output uses *adjusted p-value* instead of the expected format.
Location: Results section
```

### OBJ (92%)
```
Overall Score: 92.0%
Completeness: 95/100
Accuracy: 95/100
Structure: 90/100 ⚠️
Formatting: 95/100
Metadata: 100/100

[Minor] Structure: The section titles in the parser output do not match the original document's structure, particularly the lack of a direct correspondence to the sections in the original input.
Location: Geometry Statistics
```

### VCF (90%)
```
Overall Score: 90.0%
Completeness: 100/100
Accuracy: 100/100
Structure: 100/100
Formatting: 100/100
Metadata: 80/100 ⚠️

[Minor] Metadata: The title/header format is not preserved; the original input uses 'FN' while the output uses 'John Doe' as a header.
Location: Header
```

### TAR (88%)
```
Overall Score: 88.0%
Completeness: 95/100
Accuracy: 90/100 ⚠️
Structure: 95/100
Formatting: 90/100 ⚠️
Metadata: 100/100

[Major] Accuracy: The total size of the archive is reported as 392 bytes, but the individual file sizes add up to 392 bytes, not including any additional TAR overhead.
Location: Archive Summary

[Minor] Formatting: The list of contents could be formatted with bullet points or indentation for better readability.
Location: Contents
```

### ODS (85%)
```
Overall Score: 85.0%
Completeness: 95/100
Accuracy: 90/100 ⚠️
Structure: 95/100
Formatting: 90/100 ⚠️
Metadata: 100/100

[Major] Accuracy: The age values are presented without units or context, which could lead to misinterpretation.
Location: Sheet: Sheet1

[Minor] Formatting: The table lacks proper alignment and spacing for better readability.
Location: Sheet: Sheet1
```

## Implementation Priorities

### Phase 1: Quick Wins (92-94% formats)
1. **GPX** - Add horizontal rule or extra spacing between Waypoints section and previous sections
2. **IPYNB** - Add cell separator comments or horizontal rules between cells
3. **KML** - Review coordinate formatting to match original precision
4. **OBJ** - Review section header generation to match input structure

### Phase 2: Format-Specific Fixes (90-92%)
5. **JATS** - Fix emphasis serialization (don't add * around already-emphasized terms)
6. **VCF** - Preserve FN field in header instead of converting to display name

### Phase 3: Complex Fixes (85-90%)
7. **TAR** - Calculate actual TAR archive size (file sizes + headers + padding)
8. **TAR** - Use bullet points for file list formatting
9. **ODS** - Add "years" suffix to age column values
10. **ODS** - Improve table cell alignment/spacing in markdown output

## Next Session

**Continue with GPX fix** - Add section separator before Waypoints heading.

**Test command:**
```bash
export OPENAI_API_KEY="sk-proj-..." && \
cargo test -p docling-core --test llm_verification_tests test_llm_mode3_gpx -- --exact --ignored --nocapture
```

**Expected result:** GPX: 92% → 95%
