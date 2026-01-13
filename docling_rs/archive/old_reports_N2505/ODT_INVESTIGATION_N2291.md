# ODT Investigation - N=2291 (Incomplete)

**Date:** 2025-11-25
**Worker:** N=2291
**Status:** Investigation started, not completed

---

## LLM Test Results

**Score:** 85% (Completeness: 95, Accuracy: 100, Structure: 90, Formatting: 90, Metadata: 100)

**Complaints:**
1. **Structure (90/100):** "No clear separation between paragraphs"
   Location: Body of document
2. **Formatting (90/100):** "No indication of lists or formatting styles"
   Location: Body of document

---

## Investigation Status

**Files Examined:**
- `crates/docling-opendocument/src/odt.rs` - Parser (extracts paragraphs and lists)
- `crates/docling-backend/src/opendocument.rs` - Backend (converts to DocItems)

**Findings:**
- Parser DOES extract paragraphs (paragraph_count tracked)
- Parser DOES extract lists (list_item handling present)
- Backend converts paragraphs to `DocItem::Text`
- Need to verify: How DocItems are serialized to markdown (spacing between paragraphs)

**Possible Issues:**
1. Paragraphs might not have blank lines between them in markdown output
2. List items might not have proper markdown formatting (bullets/numbers)
3. These could be serialization issues, not parsing issues

---

## Next Steps for Future Worker

1. **Check serializer:** How are DocItems converted to markdown?
   - File: `crates/docling-core/src/serializers/markdown.rs` (likely)
   - Question: Are paragraphs separated by blank lines?
   - Question: Are lists formatted with bullets/numbers?

2. **Run actual test and inspect output:**
   ```bash
   cargo test -p docling-core --lib test_odt -- --nocapture
   ```
   - Look at actual markdown output
   - Compare with test file content

3. **Fix if real issues found:**
   - If paragraphs not separated: Add blank lines between Text DocItems
   - If lists not formatted: Ensure ListItem DocItems get bullets/numbers

4. **Re-test:**
   ```bash
   source .env
   OPENAI_API_KEY=... cargo test -p docling-core --test llm_verification_tests \
     test_llm_mode3_odt -- --exact --ignored --nocapture
   ```

---

## Test File

Test file: `test-corpus/opendocument/odt/sample.odt` (or similar)
- Need to extract and inspect content.xml
- Verify what paragraph separation looks like in source

---

## Priority

**ODT at 85% is close to threshold (95%).**
- Only 10 points away
- Both complaints are about formatting/structure (not missing content)
- Likely fixable with serialization improvements

**Estimated effort:** 1-2 hours to investigate and fix

---

**Worker N=2291: Investigation started, passed to next worker for completion**
