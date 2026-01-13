# LLM Quality Investigation (N=2194)

## Summary

Investigated quality issues in 90-94% formats to determine which are real vs. LLM variance/false positives.

## Key Finding: Most 90-94% Scores Are Not Real Issues

**Formats tested:**
1. **IPYNB (93% → 95%)** - LLM variance, no code changes
2. **ICS (93% → 96%)** - LLM variance, no code changes
3. **GPX (93% → 95%)** - LLM variance, no code changes
4. **AsciiDoc (93% → 100%)** - LLM variance, no code changes
5. **VCF (90% → 92%)** - REAL ISSUE: Missing "vCard" in title (FIXED)
6. **KML (92%)** - False positive: LLM wants XML structure preserved, but we SHOULD convert to markdown
7. **JATS (92%)** - False positive: LLM says we add italics, but source XML HAS `<italic>` tags. **Rust is MORE correct than Python** (Python drops formatting)
8. **OBJ (92%)** - False positive: LLM wants raw vertex/face data, but we correctly summarize geometry
9. **TAR (90% → 93%)** - LLM variance, minor formatting preference
10. **ODS (88%)** - REAL ISSUE: Missing metadata (author, date) but test file has no meta.xml
11. **ODT (85%)** - REAL ISSUES: Section separation unclear, paragraph spacing not preserved

## Real Issues Found (and Fixed)

### VCF (90% → 92%) - FIXED N=2194
**Issue:** Title was "# Contacts (N total)" without indicating vCard format
**Fix:** Changed to "# vCard Contacts (N total)" or "# vCard Contact" (singular)
**Impact:** Metadata score 80 → 90

## Real Issues Found (Not Fixed Yet)

### ODT (85%)
**Issue 1:** Section separation not clear (Structure: 90/100)
**Issue 2:** Paragraph spacing not preserved (Formatting: 90/100)
**Impact:** Real structure/formatting problems in OpenDocument Text parser

### ODS (88%)
**Issue:** Missing document metadata like author/creation date (Metadata: 80/100)
**Note:** Test file `simple_spreadsheet.ods` has no `meta.xml`, so metadata may not exist
**Action Needed:** Check if ODS files in corpus have metadata; if yes, extract from meta.xml

## False Positives (Do NOT Fix)

### JATS (92%) - Rust More Correct Than Python
**LLM Complaint:** "Zfp809" and "adjusted p-value" formatted as `*Zfp809*` and `*adjusted p-value*`
**Reality:** Source XML has `<italic>Zfp809</italic>` and `<italic>adjusted p-value</italic>`
**Analysis:**
- Rust correctly preserves italic formatting from source
- Python docling DROPS italic formatting (bug in Python)
- LLM penalizes us for being more accurate than baseline
**Action:** DO NOT CHANGE - Rust behavior is correct

### KML (92%) - Incorrect Expectation
**LLM Complaint:** "Does not preserve original XML structure"
**Reality:** KML is XML format; we're SUPPOSED to convert to markdown
**Action:** DO NOT CHANGE - conversion is correct behavior

### OBJ (92%) - Incorrect Expectation
**LLM Complaint:** "Does not preserve face definitions line structure"
**Reality:** OBJ files have thousands of vertices/faces; we correctly summarize geometry
**Action:** DO NOT CHANGE - summary is correct for document parser

## LLM Variance Examples

Multiple formats scored 93% in previous tests but scored 95-100% in N=2194 with ZERO code changes:
- AsciiDoc: 93% → 100%
- IPYNB: 93% → 95%
- ICS: 93% → 96%
- GPX: 93% → 95%

**Lesson:** Scores in 90-95% range have ~2-5% variance due to LLM judgment variability

## Recommendations for Next AI

### Immediate Priority: ODT (85%)
- Investigate section/paragraph structure
- Check how Python docling handles ODT section markers
- May need to add explicit section boundaries (horizontal rules, extra spacing)

### Medium Priority: ODS (88%)
- Check if test corpus ODS files have meta.xml
- If yes: Parse author, creation date from meta.xml (ZIP extraction required)
- If no: LLM complaint is invalid (can't extract what doesn't exist)

### Low Priority: Remaining 90-94%
- Most are LLM variance or false positives
- Focus on formats below 90% for real improvements

### Do NOT Change
- JATS italic formatting (Rust is correct, Python is wrong)
- KML XML→markdown conversion (correct behavior)
- OBJ geometry summarization (correct for document parser)

## Statistics

**Formats investigated:** 11
**Real issues found:** 3 (VCF fixed, ODT/ODS remain)
**False positives:** 3 (JATS, KML, OBJ)
**LLM variance:** 5 (AsciiDoc, IPYNB, ICS, GPX, TAR)

**Key Insight:** 70% of 90-94% scores are NOT actionable code issues.
