# ODT/ODS Quality Investigation (N=2196)

## Summary

Investigated ODT (80%) and ODS (88%) LLM quality scores. Both are FALSE POSITIVES - parsers are working correctly.

## Key Findings

### ODT (80%) - LLM Complaints vs Reality

**LLM Said:**
- Completeness (90/100): "Missing additional content or sections"
- Structure (90/100): "Paragraph breaks not accurately represented"
- Formatting (85/100): "Lack of bold, italics, or lists"

**Reality Check:**
1. **Test file is extremely simple:**
   - 1 heading: "Simple Document"
   - 2 paragraphs: "This is a simple ODT document." / "It has two paragraphs."
   - NO `<text:span>` formatting in source XML
   - NO tables, NO lists, NO bold/italic

2. **Parser HAS formatting support:**
   - `OdtParagraphBuilder` (opendocument.rs:67-185) tracks bold/italic/underline
   - Handles `<text:span>` elements with style-name heuristics
   - Correctly generates DocItems with Formatting attributes
   - Test files just don't HAVE formatting to extract

3. **ODT is NEW format:**
   - Python docling v2.58.0 does NOT support ODT/ODS/ODP
   - This is NEW functionality in Rust
   - LLM is judging against idealized expectations, not against baseline

4. **Paragraph spacing is correct:**
   - Text DocItems: "\n\n" after each (markdown_helper.rs:77-78)
   - SectionHeaders: "\n\n" after each (markdown_helper.rs:93-94)
   - Proper markdown paragraph separation

**Conclusion:** Parser is complete and correct. Low score is due to simple test data.

### ODS (88%) - LLM Complaints vs Reality

**LLM Said:**
- Metadata (80/100): "Missing document metadata like author/creation date"

**Reality Check:**
1. **Test files have NO metadata:**
   ```bash
   $ unzip -l test-corpus/opendocument/ods/simple_spreadsheet.ods
   Archive:  test-corpus/opendocument/ods/simple_spreadsheet.ods
     Length      Date    Time    Name
   ---------  ---------- -----   ----
          46  11-07-2025 09:55   mimetype
         374  11-07-2025 09:55   META-INF/manifest.xml
        1105  11-07-2025 09:55   content.xml
   ```
   - NO `meta.xml` file in any test ODS files
   - 5 test files checked: budget.ods, inventory.ods, multi_sheet.ods, simple_spreadsheet.ods, test_data.ods
   - All lack `meta.xml`

2. **Parser correctly tries to extract metadata:**
   - `extract_metadata()` in opendocument.rs:216-272
   - Returns `(None, None, None, None, None)` when meta.xml doesn't exist (line 226)
   - **You can't extract what doesn't exist!**

**Conclusion:** FALSE POSITIVE - LLM wants metadata that doesn't exist in source files.

## Recommendations

**DO NOT modify ODT/ODS parsers.** Both are working correctly:

✅ **ODT Parser:**
- Complete formatting support (bold, italic, underline)
- Correct paragraph spacing
- Handles headings, lists, tables, text
- Extracts metadata when present

✅ **ODS Parser:**
- Extracts all sheet data
- Handles multiple sheets
- Attempts metadata extraction (returns None when unavailable)
- Correct table serialization

**Why Low Scores:**
1. Test corpus files are extremely simple (minimal content)
2. Test files lack metadata entirely (no meta.xml)
3. LLM judges against idealized expectations (not baseline Python)
4. Python docling doesn't even support ODT/ODS, so no regression possible

## Philosophy Alignment

Per CLAUDE.md "World's Best Parser":
> "the past doesn't matter. only the current state matters"

**Current state:** Both parsers extract ALL available information from source files.
- ODT: Extracts all text, headings, formatting (when present)
- ODS: Extracts all tables, sheets, metadata (when present)

The parsers ARE world-class - they extract everything correctly. The low LLM scores reflect test data simplicity, not parser quality.

## Action: Move On

Focus on formats below 85% with REAL issues, not false positives.

**Next Targets:**
- Formats with actual bugs
- Formats with missing features
- Formats with extraction errors

**Skip:**
- ODT (80%): Test files too simple, parser is complete
- ODS (88%): Test files lack metadata, parser is complete

**Time Saved:** ~2-4 hours by not chasing false positives
