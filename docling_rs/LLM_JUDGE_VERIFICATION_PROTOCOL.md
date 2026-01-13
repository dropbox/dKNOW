# LLM as Judge - Verification Protocol

**Created:** 2025-11-24
**Purpose:** Document the correct approach for handling LLM quality test feedback
**Key Insight:** User's request - "Read the explanation and verify if reasonable"

---

## The Problem Worker Had

**Worker's approach at N=1836, N=1976, N=1978:**
1. Run LLM tests
2. See variance (±8%)
3. Conclude "all variance, zero bugs"
4. Stop working

**What was missing:** Actually verifying the specific LLM complaints by checking code

**Result:** Declared work complete at 34/38 (89.5%) with at least 1 verified bug still present (ODP images)

---

## Correct Protocol (User-Validated)

### Step 1: Run LLM Test and Get Feedback

```bash
source .env
cargo test -p docling-core --test llm_verification_tests \
  test_llm_mode3_{format} -- --exact --ignored --nocapture
```

**Capture:**
- Overall score (e.g., 88%)
- Category scores (Completeness, Accuracy, Structure, Formatting, Metadata)
- **LLM Findings section** - Specific complaints with locations

---

### Step 2: Read Each Specific Complaint

**Example - ODP at 88%:**
```
Findings:
  [Major] Completeness: "Missing slide content details such as bullet points or images"
  [Minor] Structure: "Lack of clear separation between slides"
  [Major] Formatting: "No indication of formatting styles for headers or text"
```

**DON'T stop here and say "variance"!**
**DO read each complaint individually and verify.**

---

### Step 3: Verify Each Complaint in Code

**For each complaint, ask:**
- What specific feature does LLM say is missing?
- Does that feature exist in the code?
- Check the actual code that generates output for that format

**Verification Methods:**

**Method A: Code Search**
```bash
# ODP complaint: "Missing images"
grep -r "draw:image" crates/docling-opendocument/src/odp.rs
# Result: 0 matches → Image extraction NOT implemented → REAL BUG ✅
```

**Method B: Code Reading**
```bash
# EPUB complaint: "TOC not proper list"
# Check: crates/docling-backend/src/ebooks.rs:207
# Finding: doc_items.push(create_list_item(...))
# Result: Uses ListItem → LLM IS WRONG ❌
```

**Method C: Output Inspection**
```bash
# If unclear from code, check actual output
cargo test -p docling-backend --lib test_mobi_to_markdown -- --nocapture
# Count TOC entries vs chapters in output
```

---

### Step 4: Make Judgment Call

**Decision Tree:**

```
Is the feature missing in code?
  YES → ✅ REAL BUG
    → Fix it
    → Expected: Score improves 5-15%
    → Example: ODP missing images

  NO → Feature exists in code
    ↓
    Is LLM complaint factually wrong?
      YES → ❌ FALSE POSITIVE
        → Dismiss it
        → Document for future reference
        → Example: EPUB TOC uses ListItems but LLM says "plain text"

      NO → Complaint is about quality not presence
        ↓
        Does fix break unit tests?
          YES → ❌ SKIP (tests are correct)
            → This is variance/preference

          NO → Does fix objectively improve output?
            YES → ✅ IMPROVE
              → Make the improvement
              → Test doesn't break? Commit it

            NO → ❌ VARIANCE
              → Subjective preference
              → Skip it
```

---

### Step 5: Document Findings

**For Real Bugs:**
```markdown
Format: ODP (88%)
Complaint: "Missing images"
Verification: Searched for draw:image handling → 0 results
Judgment: ✅ REAL BUG - Images not extracted
Fix: Add image element parsing
Expected: 88% → 93-95%
```

**For False Positives:**
```markdown
Format: EPUB (88%)
Complaint: "TOC not proper list"
Verification: Line 207 uses create_list_item()
Judgment: ❌ FALSE POSITIVE - TOC already uses ListItems
Action: No fix needed, LLM is wrong
```

**For Variance:**
```markdown
Format: OBJ (85-93%)
Complaint: "Title format not exact match"
Verification: Same complaint, score varies 93%→85% on identical code
Judgment: ❌ VARIANCE - Score fluctuates ±8%
Action: Skip, cannot reliably improve
```

---

## Examples from N=2018 Analysis

### ODP - Real Bug Found ✅

**LLM Said:** "Missing slide content details such as bullet points or images"

**Verification Process:**
1. Check bullets: `grep "•\|bullet" crates/docling-opendocument/src/odp.rs`
   - Found: Line 434 has bullet extraction ✅
2. Check images: `grep "draw:image\|image" crates/docling-opendocument/src/odp.rs`
   - Found: 0 results ❌
3. **Judgment:** Bullets extracted ✅, Images NOT extracted ❌

**Result:** **REAL BUG CONFIRMED** - ODP doesn't handle images

**Fix:** Add image element handling to parser

---

### EPUB - False Positive ❌

**LLM Said:** "Table of contents not formatted as a proper list; appears as plain text"

**Verification Process:**
1. Find TOC generation code: `crates/docling-backend/src/ebooks.rs:143-220`
2. Check line 207: `doc_items.push(create_list_item(...))`
3. Verify: Uses `ListItem` DocItem type with "- " markers
4. **Judgment:** Code already uses proper list format

**Result:** **FALSE POSITIVE** - LLM is factually wrong

**Action:** No fix needed

---

### OBJ - Variance Confirmed ±8% ❌

**LLM Said:** "Title format not exact match to original"

**Verification Process:**
1. Run test twice on identical code (N=1976)
2. Result: 93% → 85% (same complaint, different score)
3. Category scores: Formatting improved 95→100, yet overall dropped
4. **Judgment:** Mathematically inconsistent - this is variance

**Result:** **VARIANCE CONFIRMED** - Cannot fix unreliable metric

**Action:** Accept as effective pass (93% baseline)

---

## Integration with Testing Strategy

**Update TESTING_STRATEGY.md to include:**

### When LLM Test Fails (<95%)

**OLD approach (WRONG):**
```
Score < 95% → Assume variance → Move on
```

**NEW approach (CORRECT):**
```
Score < 95% → Read LLM Findings section → Verify each complaint:

For each complaint:
  1. What specific feature is missing?
  2. Search code for that feature
  3. If missing → REAL BUG → Fix it
  4. If present → FALSE POSITIVE → Dismiss it
  5. If unclear → Check actual output → Verify
```

**Then decide:**
- Found real bugs? → Fix them, expect improvement
- All false positives? → Dismiss as variance
- Mix? → Fix real ones, ignore false ones

---

## Statistics from N=2018 Analysis

**Formats analyzed:** 4 (ODP, EPUB, FB2, MOBI)

**Complaint breakdown:**
- ✅ Real bugs: 1 confirmed (ODP images)
- ❌ False positives: 1 confirmed (EPUB TOC)
- 🟡 Unverified: 2 need checking (FB2, MOBI)

**Worker's claim:** "4/4 are variance"
**Reality:** "1/4 confirmed real, 2/4 need verification"

**False positive rate:** 25% (1/4), not 100%
**Real bug rate:** 25% (1/4), not 0%

**Key lesson:** MUST verify complaints, cannot assume all variance

---

## Implementation Checklist

**When handling LLM test failure:**

- [ ] Read overall score
- [ ] Read category scores
- [ ] **Read Findings section carefully** ⭐ (NEW)
- [ ] **For each finding, verify in code** ⭐ (NEW)
  - [ ] Search for mentioned feature/element
  - [ ] Check if extraction code exists
  - [ ] Verify output if needed
- [ ] **Classify each complaint:** ⭐ (NEW)
  - [ ] Real bug (missing in code)
  - [ ] False positive (present in code)
  - [ ] Variance (inconsistent across runs)
- [ ] **Fix only real bugs** ⭐ (NEW)
- [ ] Document verification for future reference
- [ ] Re-test after fixes

---

## Success Story: How This Should Work

**N=2018 demonstrated correct process:**

1. **ODP scored 88%**
2. **Read complaint:** "Missing images"
3. **Verified:** Searched code for image handling
4. **Found:** No draw:image extraction exists
5. **Judgment:** ✅ REAL BUG
6. **Action:** Fix needed (directive created)
7. **Expected:** 88% → 93-95% after fix

**vs. Worker's approach:**
1. ODP scored 88%
2. ~~Assumed variance~~
3. ~~Stopped working~~
4. ~~Declared complete~~

---

## Document Updates Needed

1. **TESTING_STRATEGY.md** - Add "LLM Complaint Verification Protocol"
2. **CLAUDE.md** - Reference this protocol when handling LLM tests
3. **NEVER_FINISHED_ROADMAP.md** - Update Phase 1-3 to include verification step

---

## Critical Principle

**USER'S INSIGHT:**
> "How is it that we cannot get the LLM as Judge to recognize perfection?"

**ANSWER:**
Because the features aren't perfect! When LLM says something is missing:
1. Check if it's actually missing → Fix it
2. Check if it's actually present → Dismiss it

**DON'T assume variance without verification.**

This is "better judgment" - verify before concluding.

---

**Worker: Fix the build, then fix ODP images, then verify FB2/MOBI, then get to 38/38.**
