# Format Quality Session N=2182-2183

## Summary

Tested multiple formats with LLM quality verification. Made improvements to GLB and ODS formats.

## Formats Tested This Session

### ✅ GLB - 95% (PASSING)
- **Before:** 95% with "bullet points should be consistently formatted"
- **After:** 95% with "inconsistent use of bullet points" (minor wording change)
- **Fix Applied:** Indented material properties as sub-bullets (2-space indent)
- **Commit:** N=2182
- **Verdict:** Passing threshold. LLM feedback may be false positive - structure is correct.

### ⚠️  ODS - 85% (NOT PASSING, but improved)
- **Before:** 85% with "header not in clear format, lacking distinction from metadata"
- **After:** 85% with different complaints (table headers, type annotations)
- **Fix Applied:** Added "Sheets" section header, changed sheet headers from level 2 to level 3
- **Impact:** Structure improved 90% → 95% (+5%)
- **Commit:** N=2183
- **New complaints:**
  - "age values not integers" → FALSE POSITIVE (markdown has no type annotations)
  - "table header not visually distinct" → Might be real table rendering issue
- **Next Steps:** Investigate table header rendering in markdown

### ❌ VCF - 92% (FALSE POSITIVE)
- **Score:** 92%
- **Complaint:** "BEGIN:VCARD and END:VCARD tags not shown in output"
- **Verdict:** FALSE POSITIVE - Tags ARE preserved (verified vcf.rs lines 376, 423 in code blocks)
- **Action:** None needed - this is LLM variance

### ❌ TAR - 92% (REAL ISSUE)
- **Score:** 92%
- **Complaint:** "List formatting could be improved for clarity in Contents section"
- **Location:** archive.rs lines 275-285
- **Verdict:** Likely REAL - file listings format needs investigation
- **Next Steps:** Check Contents section formatting

### ❌ OBJ - 89% (LLM VARIANCE)
- **Score:** 89% (was 92% in N=2181, no code changes)
- **Complaint:** Vague "minor issues in completeness and formatting"
- **Verdict:** LLM variance - score dropped without code changes
- **Action:** None - don't chase variance

### ❌ DXF - 85% (MIXED)
- **Score:** 85%
- **Complaints:**
  1. "Missing header variables (DIMLTYPE, DIMLTEX1, DIMLTEX2)"  → Might be REAL
  2. "Doesn't preserve original section structure (HEADER, TABLES, etc.)" → FALSE POSITIVE (parser converts to markdown, not raw DXF)
- **Next Steps:** Check if important header variables are missing

## Key Learnings

### 1. LLM Feedback Verification is Critical

**Always verify complaints before fixing:**
- ✅ ODS "header not clear" → REAL (Structure +5% confirms fix worked)
- ❌ VCF "tags not shown" → FALSE (verified in code, tags ARE present)
- ❌ DXF "preserve section structure" → FALSE (parser purpose is to convert, not preserve)

**How to verify:**
1. Read LLM complaint carefully
2. Check actual code (grep, Read tool)
3. Verify claim in source
4. Make judgment: Real bug or false positive?

### 2. LLM Variance Exists

**Example: OBJ dropped 92% → 89% without code changes**
- Don't chase variance without verification
- Score changes without code changes = variance
- Focus on specific actionable complaints

### 3. Measuring Impact

**When making fixes, measure category improvements:**
- ODS Structure: 90% → 95% (+5%) confirms fix worked
- If categories don't improve, fix may not have worked or complaint was false

### 4. Document Structure Hierarchy Matters

**ODS improvement shows hierarchy importance:**
```markdown
# Spreadsheet: filename     (level 1 - document title)
## Metadata                 (level 2 - main section)
1 sheet: Sheet1

## Sheets                   (level 2 - main section)  ← ADDED
### Sheet: Sheet1           (level 3 - subsection)   ← CHANGED FROM 2 TO 3
[table here]
### Sheet: Sheet2           (level 3 - subsection)
[table here]
```

**Before:** Metadata and Sheet headers both level 2 (flat, unclear)
**After:** Sheets section groups sheet headers as level 3 (hierarchical, clear)

### 5. Formatting Details Matter

**GLB Materials section improvement:**
```markdown
## Materials
### Material Name

<!-- Before (flat bullets) -->
- Base Color: (1.0, 1.0, 1.0, 1.0)
- Metallic: 0.5

<!-- After (sub-bullets with 2-space indent) -->

  - Base Color: (1.0, 1.0, 1.0, 1.0)
  - Metallic: 0.5
```

**Result:** Better visual grouping, though LLM still sees "minor inconsistency"

## Actionable Next Steps (Priority Order)

### 1. TAR Format - 92% (Likely Real Issue)
- **Complaint:** "List formatting could be improved"
- **Location:** archive.rs:275-285
- **Action:** Check file listing format in Contents section
- **Priority:** High (specific, actionable)

### 2. ODS Format - 85% (Table Headers)
- **Complaint:** "Table header not visually distinct"
- **Action:** Investigate how first row is rendered (might need bold header row)
- **Priority:** Medium (ODS still needs +10% improvement)

### 3. DXF Format - 85% (Header Variables)
- **Complaint:** "Missing DIMLTYPE, DIMLTEX1, DIMLTEX2"
- **Action:** Check DXF parser - are these variables actually missing?
- **Priority:** Medium (verify if real first)

### 4. Ignore False Positives
- VCF "tags not shown" (verified false)
- DXF "preserve structure" (false - parser converts)
- OBJ variance (no code changes)

## Testing Protocol Reminder

**For each format:**
```bash
source .env  # Load API key
cargo test -p docling-core --test llm_verification_tests \
  test_llm_mode3_{format} -- --exact --ignored --nocapture
```

**Before fixing:**
1. Read "Findings" section carefully
2. Verify complaint in code
3. Confirm it's a real issue, not false positive
4. Only fix real issues

**After fixing:**
1. Re-run LLM test
2. Check if category scores improved
3. If no improvement: fix didn't work or complaint was false

## Format Status Reference (From WORLD_BEST_PARSER.txt)

**95%+ (Passing):**
- 12 formats including GLB (95%), AsciiDoc (100%)

**90-94% (Close):**
- GLB: 95% ✅ (improved this session)
- TAR: 92% ⚠️  (needs list formatting fix)
- VCF: 92% ❓ (false positive)
- OBJ: 93% → 89% ❓ (variance)
- ODS: 90% (structure improved, but still 85% overall)

**85-89% (Needs Work):**
- ODS: 85% ⚠️  (structure fixed +5%, table headers next)
- DXF: 85% ⚠️  (mixed real/false complaints)

**Philosophy Reminder:**
- Never stop improving
- Fix every real bug
- Don't accept "good enough"
- Verify complaints before fixing
- Measure impact after fixes
