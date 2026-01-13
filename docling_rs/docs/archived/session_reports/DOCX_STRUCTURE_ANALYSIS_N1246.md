# DOCX Structure Analysis - N=1246

**Date:** 2025-11-17
**Branch:** feature/phase-e-open-standards
**Context:** N=1245 discovered DOCX at 92% (needs 95%), Structure score 85/100

## Investigation Results

### Exported DocItem JSON Analysis

**Test File:** test-corpus/docx/word_sample.docx
**JSON Size:** 121,971 bytes (119.1 KB)
**Total DocItems:** 25

**DocItem Breakdown:**
- Title: 1 ✅
- SectionHeader: 2 ✅
- Text: 11
- ListItem: 9 ✅
- Table: 1 ✅
- Picture: 1 ✅

### Structure Captured Correctly

**Verified from JSON (lines in /tmp/word_sample_docitems.json):**
1. Line 33: `"label": "title"` → "Swimming in the lake" ✅
2. Line 118: `"label": "section_header"` → "Let's swim!" ✅
3. Line 321: `"label": "section_header"` → "Let's eat" ✅
4. Lines 157-281: 6 list_items (bullet + numbered lists) ✅
5. Line 398: `"label": "table"` → 4x3 table ✅
6. Lines 570-612: 3 list_items (final numbered list) ✅

**Total: 9 list items, 2 section headers, 1 title = Structure IS being captured**

### Potential Issues Identified

**Issue 1: "Summer activities" Not a Heading**
- Line 14 in JSON: `"label": "text"`
- In markdown (line 2): appears as plain text before `# Swimming in the lake`
- **Question:** Should this be a heading? Need to check original DOCX style

**Issue 2: Heading Levels**
From markdown (line 2):
```markdown
Summer activities

# Swimming in the lake     ← Title style
## Let's swim!            ← SectionHeader level 1?
### Let's eat            ← SectionHeader level 2?
```

The markdown shows:
- `#` = Title (1 hash)
- `##` = SectionHeader (2 hashes)
- `###` = SectionHeader (3 hashes)

But we only have 2 SectionHeader entries (lines 118, 321). Let me check their levels:

Looking at line 122-137 (first SectionHeader):
```json
"label": "section_header",
"self_ref": "#/headers/0",
"content_layer": "body",
"level": 1,
...
"text": "Let's swim!"
```

Line 325-340 (second SectionHeader):
```json
"label": "section_header",
"self_ref": "#/headers/0",  ← BUG: Should be #/headers/1
"content_layer": "body",
"level": 2,
...
"text": "Let's eat"
```

**FOUND BUG:** Both SectionHeaders have `self_ref: "#/headers/0"` - the second should be `#/headers/1`

### LLM Test Feedback

**From test_llm_docitem_docx output:**
```
Overall Score: 92.0%
Category Scores:
  Text Content: 95/100 ✅
  Structure:    85/100 ❌
  Tables:       90/100
  Images:       90/100
  Metadata:     100/100 ✅

DocItem Gaps:
  - Some text content may not be semantically accurate due to potential parsing errors.
  - Section headers and list structures are not consistently identified or preserved.
  - List formatting markers are not always correctly identified.
```

**LLM says:** "Section headers and list structures not consistently identified"

**But JSON shows:**
- 2 section headers ✅ (correctly identified)
- 9 list items ✅ (correctly identified)
- List markers present (enumerated field) ✅

**Hypothesis:** LLM may be confused by:
1. Duplicate `self_ref` values (#/headers/0 appearing twice) ← **BUG**
2. "Summer activities" not being a heading (if it should be)
3. Possible issues with list nesting representation

### Code Review - docx.rs:1211-1221

```rust
} else if let Some(level) = heading_level {
    // Heading (use SectionHeader variant)
    // Note: level is used as-is (Heading1 → level 1, Heading2 → level 2)
    // The markdown serializer adds +1 to the level when generating # hashes
    // So: Title → # (1 hash), SectionHeader{level:1} → ## (2 hashes)
    Some(create_section_header(
        0, // Will be fixed later ← **BUG: This is the problem!**
        text.to_string(),
        level, // Use level as-is, serializer handles the increment
        vec![create_default_provenance(1, CoordOrigin::TopLeft)],
    ))
```

**FOUND BUG #2:** Line 1217 passes `0` as the index for self_ref generation, which means ALL section headers get `#/headers/0`. This is the duplicate self_ref bug!

### Root Cause Analysis

**Duplicate self_ref Bug:**
- Location: crates/docling-backend/src/docx.rs:1217
- Issue: `create_section_header(0, ...)` hard-codes index to 0
- Result: All section headers get `self_ref: "#/headers/0"`
- Impact: LLM may interpret this as only 1 section header existing

**Similar Issue in Other DocItem Types:**
Looking at nearby code:
- Line 1185: `self_ref: format!("#/list_items/{}", 0)` ← All list items get index 0
- Line 1201: `self_ref: format!("#/titles/{}", 0)` ← All titles get index 0
- Line 1225: `self_ref: format!("#/texts/{}", 0)` ← All text items get index 0

**Comment says:** "Will be fixed later" (line 1217)

**But it was never fixed!**

### The Fix

**Required:** Implement proper index tracking for DocItem creation

**Approach:**
1. Add counters for each DocItem type (title_count, header_count, text_count, list_count, etc.)
2. Increment counters as DocItems are created
3. Use actual counter value instead of hard-coded 0

**Example Fix:**
```rust
// Add to parser state
struct DocxParser {
    title_count: usize,
    header_count: usize,
    text_count: usize,
    list_count: usize,
    table_count: usize,
    // ... existing fields
}

// When creating SectionHeader:
Some(create_section_header(
    self.header_count, // Use actual counter, not 0
    text.to_string(),
    level,
    vec![create_default_provenance(1, CoordOrigin::TopLeft)],
))
self.header_count += 1; // Increment after use
```

### Why This Matters

**self_ref Purpose:**
- Unique identifier for each DocItem
- Used for references, relationships, DOM structure
- LLM validation likely checks uniqueness
- Duplicate refs → LLM thinks structure is incomplete

**Impact on Structure Score:**
- LLM sees `#/headers/0` twice → "only 1 header?"
- LLM sees `#/list_items/0` nine times → "only 1 list item?"
- Structure appears incomplete even though content is correct
- This explains 85/100 instead of 95+

### Secondary Issue: "Summer activities"

**Current:** Labeled as "text"
**Possible:** Should be a heading (need to verify original DOCX style)

**To Verify:**
1. Open word_sample.docx in Word/LibreOffice
2. Check style of "Summer activities" paragraph
3. If it's "Normal" style → correct as "text"
4. If it's "Heading" style → detection logic needs fix

### Recommended Fix Priority

**Priority 1: Fix duplicate self_ref values** (High Impact)
- Implement proper index counters
- Update all DocItem creation sites
- This alone may boost Structure score significantly

**Priority 2: Verify "Summer activities" style** (Medium Impact)
- Check original DOCX
- Adjust detection logic if needed

**Priority 3: Verify list nesting** (Low Impact)
- Check if list hierarchy is represented
- May need parent/children relationships

### Expected Improvement

**Current:**
- Structure: 85/100 (failing)
- Overall: 92% (failing)

**After Fix:**
- Structure: 95+/100 (passing) - unique refs fix structural issues
- Overall: 95+% (passing) - should push overall score over threshold

**Confidence:** High - this is a real bug with clear fix

### Test Verification

**After implementing fix:**
```bash
export OPENAI_API_KEY="sk-proj-..."
cargo test --test llm_docitem_validation_tests test_llm_docitem_docx -- --exact --nocapture
```

**Success Criteria:**
- Overall score ≥ 95%
- Structure score ≥ 95/100
- All DocItems have unique self_ref values

### Code Locations

**Primary Fix Location:**
- crates/docling-backend/src/docx.rs:1117-1240 (ParagraphBuilder::build method)

**Affected Lines:**
- Line 1185: ListItem creation (hard-coded index 0)
- Line 1201: Title creation (hard-coded index 0)
- Line 1217: SectionHeader creation (hard-coded index 0)
- Line 1225: Text creation (hard-coded index 0)
- docx.rs:897-963: Table creation (hard-coded index 0)

**All need proper counters!**

### Lessons for Next AI

**Lesson 1: Comments Like "Will be fixed later" Are Technical Debt**
- Line 1217 says "Will be fixed later"
- It was never fixed!
- **Rule:** Search codebase for "TODO", "FIXME", "Will be fixed later" comments
- **Rule:** These are bugs waiting to be discovered

**Lesson 2: LLM Test Feedback Can Be Vague**
- LLM said "section headers not consistently identified"
- But all section headers ARE identified!
- Real issue: duplicate self_ref values confuse the LLM
- **Rule:** Export and manually inspect JSON when LLM feedback is vague

**Lesson 3: 92% Doesn't Mean "Close Enough"**
- Previous AI claimed "95-100%" based on git history
- Actual test shows 92% with real bugs
- 3% gap seems small but represents real structural issues
- **Rule:** Fix bugs, don't rationalize them away

## Next AI: Fix Duplicate self_ref Values

**Task:** Implement proper index counters for all DocItem types

**Estimated:** 2-3 hours (straightforward fix, but need to:
1. Add counters to parser state
2. Update all DocItem creation sites (5+ locations)
3. Test thoroughly to avoid breaking existing functionality
4. Re-run DocItem validation test to verify improvement

**Success Criteria:**
- All DocItems have unique self_ref values
- Structure score ≥ 95/100
- Overall score ≥ 95%
- Backend tests still 2848/2848 passing

**Files:**
- crates/docling-backend/src/docx.rs (main changes)
- /tmp/word_sample_docitems.json (reference for verification)

---

**Status:** Investigation complete, bug identified, fix approach documented
**Confidence:** High - this is a real, fixable bug
**Impact:** Should improve Structure from 85 → 95+ with this single fix
