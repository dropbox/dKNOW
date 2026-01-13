# Objective Improvement Checklist

**Purpose:** Guide future AI sessions to make verifiable, deterministic improvements without relying on LLM variance.

**Context:** Post-N=1871 strategy - Focus on objective improvements with clear rationale.

---

## Quick Reference: How to Make an Objective Improvement

1. **Pick a format** (prefer formats with no recent quality work)
2. **Read the code** (understand current implementation)
3. **Check for objective issues** (use checklist below)
4. **Make 1-2 improvements** (clear rationale, verifiable)
5. **Add/update unit tests** (deterministic verification)
6. **Commit with evidence** (document before/after)

---

## Objective Issue Checklist

### Category 1: Missing Metadata (HIGH CONFIDENCE)

**Question:** Does the format spec define metadata that we don't extract?

**How to check:**
- Read format specification (W3C, ISO, RFC, etc.)
- Compare spec fields vs. extracted fields
- Check if metadata exists in test files

**Examples:**
- ✅ EXIF GPS data exists but not shown
- ✅ EPUB author field in spec but not extracted
- ✅ ICS timezone information available but not used

**Verification:**
- Use reference tool (exiftool, calibre, etc.) to confirm metadata exists
- Add unit test that checks for metadata presence

**Commit template:**
```
# N++: Quality - {FORMAT} Missing Metadata Fix ({field_name})

**Objective improvement:** Extract {field_name} from {format} metadata

## Evidence
- Format spec: {spec_url} section {X} defines {field_name}
- Test file has {field_name}: {value}
- Before: {field_name} not shown
- After: {field_name} displayed as "{output}"

## Verification
- Unit test: test_{format}_extracts_{field_name}
- Pass rate: 100% (X/X tests)
```

### Category 2: Calculation Errors (HIGH CONFIDENCE)

**Question:** Are calculated values (dimensions, file sizes, counts) correct?

**How to check:**
- Compare output values with known correct values
- Use calculator or reference tool to verify
- Check format spec for calculation formulas

**Examples:**
- ✅ BMP file size calculation for monochrome images (deterministic math)
- ✅ ZIP compressed/uncompressed size mismatch
- ✅ Image dimensions off by 1 pixel (fencepost error)

**Verification:**
- Calculate expected value manually
- Add unit test with known input/output

**Commit template:**
```
# N++: Quality - {FORMAT} Calculation Fix ({field_name})

**Objective improvement:** Correct {field_name} calculation

## Evidence
- Input: {test_file}
- Expected {field_name}: {expected} (calculated/verified with {tool})
- Before: Output shows {actual_before} (WRONG)
- After: Output shows {actual_after} (CORRECT)

## Calculation
{show_formula_or_reasoning}

## Verification
- Unit test: test_{format}_{field}_calculation_correct
- Verified with: {reference_tool_or_manual_calculation}
```

### Category 3: Format Spec Violations (HIGH CONFIDENCE)

**Question:** Does output violate format specification requirements?

**How to check:**
- Read format specification
- Look for MUST/SHALL requirements
- Check if output omits required elements

**Examples:**
- ✅ VCF spec requires BEGIN:VCARD/END:VCARD (check if present)
- ✅ ICS spec requires VERSION:2.0 (check if present)
- ✅ JATS XML requires specific element hierarchy

**Verification:**
- Spec validator tool (if available)
- Unit test checks for required elements

**Commit template:**
```
# N++: Quality - {FORMAT} Spec Compliance ({requirement})

**Objective improvement:** Add required {element} per {spec_name} section {X}

## Evidence
- Spec: {spec_url}
- Requirement: "{quoted_spec_text}"
- Before: {element} missing (spec violation)
- After: {element} present (spec compliant)

## Verification
- Unit test: test_{format}_includes_required_{element}
- All {X} tests passing
```

### Category 4: Structure Improvements (MEDIUM CONFIDENCE)

**Question:** Is output structure objectively clearer/better organized?

**How to check:**
- Compare with similar formats (consistency)
- Check if information is logically grouped
- Verify hierarchy is preserved

**Examples:**
- ✅ N=1870: Added ## Summary header to archives (objective improvement)
- ✅ Section headers missing where appropriate
- ✅ Related information scattered instead of grouped

**Verification:**
- Before/after comparison shows objective improvement
- Unit tests verify structure is present

**Requirements for HIGH confidence:**
- Improvement follows established pattern in codebase
- Makes output objectively easier to parse/understand
- Doesn't break existing tests

**Commit template:**
```
# N++: Quality - {FORMAT} Structure Improvement ({change})

**Objective improvement:** {description}

## Rationale
{why_this_is_objectively_better}

## Evidence
**Before:**
```
{before_output_sample}
```

**After:**
```
{after_output_sample}
```

## Consistency
- Similar formats use this pattern: {examples}
- Makes output easier to: {specific_benefit}

## Verification
- Unit tests: All {X} tests passing
- Structure tests: test_{format}_has_{structure_element}
```

### Category 5: Missing Required Fields (HIGH CONFIDENCE)

**Question:** Does format always have certain fields that we don't show?

**How to check:**
- Read format documentation
- Check test files - do they all have this field?
- Compare with reference implementation

**Examples:**
- ✅ DOCX always has document dimensions (page size)
- ✅ JPEG always has color space (RGB, CMYK, grayscale)
- ✅ PDF always has page count

**Verification:**
- Check 5+ sample files - all have the field
- Reference tool shows the field
- Unit test verifies field is extracted

**Commit template:**
```
# N++: Quality - {FORMAT} Always-Present Field ({field_name})

**Objective improvement:** Extract {field_name} (present in all {format} files)

## Evidence
- Checked {N} sample files: All have {field_name}
- Reference tool ({tool_name}) shows: {value}
- Before: {field_name} not extracted
- After: {field_name} shown as "{output}"

## Verification
- Unit test: test_{format}_always_has_{field_name}
- Tested on {N} files: 100% have field
```

---

## Anti-Patterns: What NOT to Do

### ❌ Subjective Style Changes

**Examples of subjective changes:**
- "200×100" → "200 x 100" (× vs x for dimensions)
- "Format:" → "**Format:**" (plain vs bold)
- "Type: BMP Image" → "Image Type: BMP" (wording preference)

**Why to avoid:**
- No objective improvement
- May break tests
- LLM preferences vary between runs

**Exception:** If subjective change makes code consistent with 10+ other formats.

### ❌ Chasing LLM Scores

**Examples:**
- "LLM gave 88%, let's try different wording to get 95%"
- "LLM complained about X, even though code is correct"
- "Let's test this 5 times to see if score improves"

**Why to avoid:**
- LLM scores vary ±2-5% on identical code
- Wastes time and API credits
- Doesn't actually improve code

**Alternative:** Make improvements for objective reasons, ignore LLM scores.

### ❌ Breaking Tests for LLM Preferences

**Example:**
- Unit test expects "BEGIN:VCARD" and "END:VCARD"
- LLM wants different format
- Changing would break 15 tests

**Why to avoid:**
- Unit tests are deterministic and correct
- LLM preferences may be wrong or variance
- Breaking tests creates bugs

**Rule:** If improvement breaks tests, investigate test first. Tests are usually correct.

### ❌ Unverifiable Improvements

**Examples:**
- "This looks better" (subjective)
- "LLM will probably like this more" (unverifiable)
- "I think this is clearer" (opinion without evidence)

**Why to avoid:**
- Cannot verify improvement
- May introduce regressions
- Wastes time

**Required:** Before/after comparison showing objective benefit.

---

## Format Priority List (Ordered by Recent Attention)

**Tier 1: Zero Recent Attention (Best Candidates)**
- ODT (last: N=301, ~9 months ago)
- ODS (last: N=301, ~9 months ago)
- ODP (last: N=301, ~9 months ago)

**Tier 2: Minimal Recent Attention**
- GLTF (check git log)
- DXF (check git log)
- Various ebook formats (check git log)

**Tier 3: Recent Attention But May Have Issues**
- EPUB (N=1689: TOC work)
- MOBI (N=1850: TOC work)
- FB2 (N=1855: duplicate title fix)

**Tier 4: Extensively Worked Recently**
- Archives: ZIP, TAR, RAR, 7Z (N=1870)
- SVG (N=1868-1869)
- IPYNB (N=1863)
- Images: HEIF, AVIF, BMP, GIF (various sessions)

**Strategy:** Focus on Tier 1-2 first (highest likelihood of finding objective issues).

---

## Investigation Template

**For each format investigation:**

```markdown
# {FORMAT} Investigation (N={current})

## Step 1: Code Review
- File: crates/{crate}/src/{format}.rs
- Lines of code: {X}
- Last modified: N={Y} ({date})
- Key functions: {list}

## Step 2: Spec Check
- Specification: {URL or RFC number}
- Key requirements: {list}
- Optional features: {list}

## Step 3: Current Output Check
- Test file: test-corpus/{format}/{file}
- Current output length: {X} lines
- Sections present: {list}
- Missing sections: {list}

## Step 4: Comparison with Spec
- Required fields extracted: {X}/{Y}
- Missing required fields: {list}
- Extra fields (not in spec): {list}

## Step 5: Findings
**Objective issues found:**
1. {issue_1}
2. {issue_2}

**Subjective preferences (ignore):**
1. {preference_1}

## Step 6: Action Plan
- Fix issue #1: {approach}
- Fix issue #2: {approach}
- Add tests: {test_plan}

## Step 7: Verification Plan
- Unit tests: {list}
- Reference tool: {tool_name}
- Expected outcome: {description}
```

---

## Success Criteria

**For each improvement session:**

1. ✅ At least 1 objective improvement made
2. ✅ Clear before/after evidence documented
3. ✅ Unit tests added or updated (100% passing)
4. ✅ Commit message includes rationale and verification
5. ✅ No existing tests broken

**Red flags (stop and reconsider):**
- ❌ Improvement is based solely on LLM feedback
- ❌ Cannot articulate why improvement is objectively better
- ❌ Would break existing tests
- ❌ No way to verify improvement worked

---

## Example: Good Improvement (N=1870 Archive Structure)

**What was done:**
- Added ## Summary section header
- Improved dotfile detection (.gitignore → "DOTFILE")

**Why it's good:**
- ✅ Objective: Structure is clearly better organized
- ✅ Verifiable: All 32 tests passing
- ✅ Consistent: Other formats use ## headers
- ✅ Documented: Clear before/after in commit

**Why LLM scores didn't reflect it:**
- LLM variance (±2-3% on identical code)
- Subjective complaints changed between runs
- Improvement was still valid!

**Lesson:** Good code is good code, regardless of LLM scores.

---

## Next Session Recommendation

**Start with ODT format:**

1. Read crates/docling-opendocument/src/odt.rs
2. Check ODT spec: https://docs.oasis-open.org/office/v1.2/OpenDocument-v1.2.pdf
3. List required metadata fields from spec
4. Compare with extracted fields
5. Find 1-2 missing required fields
6. Implement extraction
7. Add unit tests
8. Commit with evidence

**Expected time:** 30-60 minutes

**Expected outcome:** 1-2 objective improvements with deterministic verification

---

**Remember:** Focus on making code objectively better, document clearly, verify with tests. LLM scores are unreliable, but good engineering practices never fail.
