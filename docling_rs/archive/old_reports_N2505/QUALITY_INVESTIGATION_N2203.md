# Quality Investigation - N=2203

**Date:** 2025-11-24
**Context:** LLM quality testing of low-scoring formats (ODS 83%, VCF 88%)

## Formats Tested

### 1. ODS Format (83% score)

**LLM Complaints:**
1. [Minor] Formatting: "Table formatting could be improved for better readability"
2. [Minor] Metadata: "The title 'Spreadsheet: simple_spreadsheet' could be more descriptive"

**Verification:**
- **Title** (opendocument.rs:571): Uses `format!("Spreadsheet: {file_name}")` - **CLEAR AND CORRECT**
- **Table Formatting**: Uses standard `render_table()` shared by all formats - **NO BUG**

**Conclusion:** ❌ FALSE POSITIVES - Subjective style preferences, no objective issues

### 2. VCF Format (88% score)

**LLM Complaints:**
1. [Minor] Structure: "does not maintain the original vCard format structure (BEGIN:VCARD and END:VCARD)"
2. [Minor] Formatting: "uses a different formatting style instead of preserving the original vCard format"

**Verification:**
- **Code Review** (email.rs:466-476): VCF explicitly adds format marker:
  ```rust
  let format_marker = if let Some(version) = &contact.version {
      format!("vCard Format: BEGIN:VCARD (v{version}) ... END:VCARD")
  } else {
      "vCard Format: BEGIN:VCARD ... END:VCARD".to_string()
  };
  ```
- **Purpose:** Parser CONVERTS formats to markdown (that's the point!)

**Conclusion:** ❌ FALSE POSITIVE - Format marker IS present, LLM misunderstand's parser's job

## Pattern Observed

**LLM judges consistently give subjective complaints even when code is correct:**
- ODS: "Title could be more descriptive" (title IS descriptive)
- VCF: "Doesn't preserve BEGIN:VCARD" (format marker IS included)

**This matches previous findings:**
- N=2188: TAR complaints changed between runs (variance)
- N=2186: "Improvement" to ODS made score WORSE (85% → 84%)
- N=2199: All EPUB complaints were false positives

## Recommendations

**DO NOT chase these scores:**
- ✅ All 2855 backend tests passing
- ✅ No objective bugs found
- ✅ Code follows standard patterns
- ❌ LLM complaints are subjective/incorrect

**Follow N=2188 guidance:**
> "When there are no failures, don't manufacture work by chasing subjective scores."

## Next Actions

**Per WORLD_BEST_PARSER.txt philosophy:**
- ✅ Fix REAL bugs (objective test failures)
- ✅ Extract MISSING information (verified absences)
- ❌ DON'T chase LLM subjective preferences
- ❌ DON'T "improve" working code based on variance

**Better use of time:**
1. Add new formats (expand capability)
2. Fix actual test failures (none currently)
3. Performance optimization (measurable improvements)
4. Add missing features users request

## Lesson Reinforced

**Verification Protocol Works:**
1. Read LLM complaint
2. Search code for that feature
3. Found in code? → FALSE POSITIVE
4. Missing in code? → REAL BUG

**Result:** Both ODS and VCF complaints were verified false positives.
