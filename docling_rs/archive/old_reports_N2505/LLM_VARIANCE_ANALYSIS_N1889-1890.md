# LLM Quality Variance Analysis (N=1889-1890)

**Date:** 2025-11-22
**Sessions:** N=1889 (DICOM), N=1890 (JATS)
**Purpose:** Document LLM variance and incorrect feedback during quality improvement work

---

## Executive Summary

Two formats analyzed (DICOM 94%, JATS 93%) revealed:
1. **LLM Variance:** DICOM scored 92% three times with different complaints each time
2. **Incorrect Feedback:** JATS LLM complaint contradicts source XML (Rust is correct)
3. **Recommendation:** Focus on deterministic fixes, not chasing LLM subjective preferences

---

## Case 1: DICOM (94% → 92% variance)

### Original Score
- Comprehensive test (N=1888): 94%
- Complaints: dimension format "640 × 480" should be "640x480", title too generic

### Changes Made
- Changed `×` to `x` in dimension format (objective improvement - more standard)
- Added modality-specific titles (objective improvement - more informative)

### Post-Change Testing
- **Run 1:** 92% - "section headers not consistently formatted"
- **Run 2:** 92% - "birth date format unclear", "list formatting issues"
- **Run 3:** 92% - vague minor issues, no specific findings

### Analysis
- Same code, three different complaints = **pure variance**
- Changes were objectively correct improvements
- Score variation 92-94% is within expected ±2% LLM variance range
- All markdown headers ARE consistently formatted with `##`

### Conclusion
DICOM improvements are valid. 92-94% variance is expected LLM behavior.

---

## Case 2: JATS (93% italics "issue")

### LLM Complaint
> "The term 'Zfp809' is formatted differently in the actual output (italicized)
> compared to the expected output (not italicized)."

> "The phrase 'adjusted p-value' is formatted differently in the actual output
> (italicized) compared to the expected output (not italicized)."

### Investigation
Checked source XML file `test-corpus/jats/elife-56337.nxml`:

```xml
Indeed, <italic>Zfp809</italic> knock-out (KO) in mice
...
<italic>adjusted p-value</italic><​0.00001
```

### Reality
- **Source XML:** Contains `<italic>` tags for both terms
- **Rust behavior:** Correctly preserves italic formatting from source
- **Python behavior:** Strips italic formatting (incorrect)
- **LLM feedback:** Penalizes Rust for being MORE correct than Python

### Decision
**NO changes to JATS.** Rust implementation is objectively correct.

Preserving source formatting is the right behavior. Python's stripping of italics is a bug in Python, not Rust.

---

## Key Lessons

### 1. LLM Variance is Real
- ±2-3% score variation on identical code
- Different complaints each run
- Cannot achieve 100% reliability with LLM scoring

### 2. LLM Feedback Can Be Wrong
- JATS case: LLM preferred incorrect Python behavior over correct Rust behavior
- Must validate LLM feedback against source documents and standards
- "Use better judgement" (per user directive)

### 3. Decision Framework (from USER_DIRECTIVE)

```
Is the issue deterministic and verifiable?
  YES → Implement fix (dimensions, byte counts, calculations)
  NO → Check if feedback is consistent across runs

Does LLM complain about same thing on multiple runs?
  YES → Probably real, investigate
  NO → Likely variance, use discretion

Does the fix break unit tests?
  YES → Unit tests are correct, skip LLM feedback
  NO → Continue evaluation

Does source document support the LLM complaint?
  NO → LLM is wrong (JATS italic case)
  YES → LLM may be right, verify against standards
```

### 4. What to Fix vs Skip

**DO FIX (Deterministic Issues):**
- ✅ Missing dimensions (HEIF, AVIF) - extractable from metadata
- ✅ Wrong byte counts (archives) - calculable, verifiable
- ✅ Missing metadata fields (EML subject label) - clear structural improvement
- ✅ Format standardization (x vs ×) - industry standards

**DON'T FIX (Subjective/Variance):**
- ❌ "Section headers not consistently formatted" when they ARE consistent
- ❌ "List formatting could be improved" with no specific issue
- ❌ Complaints that contradict source documents (JATS italics)
- ❌ Varying complaints across multiple runs of same code

---

## Recommended Strategy Going Forward

### Priority 1: Objective Improvements (85-90% range)
Focus on formats with clear, deterministic issues:
- TAR (85-87%): File type specification, byte count accuracy
- HEIF/AVIF (84-85%): Missing dimensions (extractable)
- BMP (85-88%): File size calculation errors
- Archives (84-87%): Structure and labeling

### Priority 2: Structural Improvements (80-85% range)
- EPUB/MOBI/FB2: TOC structure
- ODP: Missing slide content
- SVG: Missing elements (circle)
- DXF: Missing header variables

### Skip for Now: High Variance Formats (92-94%)
- OBJ, IPYNB, GPX, ICS, KML, KMZ
- These show variance and/or subjective complaints
- Revisit after lower-hanging fruit is addressed

---

## Cost Analysis

Each LLM test run: ~$0.005 (0.5 cents)
- DICOM: 3 runs = $0.015
- Total spent: ~$0.02
- Learned: Variance exists, feedback can be wrong
- Value: Documented decision framework for future work

Budget remaining: ~25 formats × $0.01/each = ~$0.25

---

## References

- USER_DIRECTIVE_QUALITY_95_PERCENT.txt
- PRIORITY_ACHIEVE_95_PERCENT_QUALITY.md
- llm_comprehensive_results_20251122_023139.txt (baseline scores)
