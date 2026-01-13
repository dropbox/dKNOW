# PDF Quality Investigation - Problem 4

## Issue Summary

**Rust PDF ML output is significantly degraded compared to expectations.**

**Observed:**
- Character count: 701 (vs expected ~9,000+)
- Text quality: Garbled ("PreDigtalEt", "WordPrcr", "TheBirfW")
- Missing spaces: Words run together
- Loss: ~92% of content

## Test Results

**Pure Rust Test:** ✅ Passes programmatically but output is poor

```
✓ DocItems: 51 generated
✓ Markdown: 701 characters
✓ Structure: Contains headers
But: Text is garbled, most content missing
```

## Hypothesis

**Possible causes:**

1. **Text Cell Assembly Issue:**
   - OCR detects characters but spacing/assembly is broken
   - Words concatenate without spaces
   - Example: "Word Processor" → "WordPrcr"

2. **Reading Order Problem:**
   - Text cells processed out of order
   - Chunks missing entirely
   - Only fragments make it to output

3. **OCR Threshold Too Aggressive:**
   - Low-confidence text rejected
   - Only high-confidence fragments kept
   - Results in incomplete extraction

4. **Page Processing Limit:**
   - Pipeline may only process subset of pages
   - Check: Are all 5 pages being processed?

5. **Character Encoding Issue:**
   - Text extracted but encoding mangled
   - UTF-8 vs ASCII issues
   - Character corruption during assembly

## Investigation Steps

**Step 1:** Check how many pages are actually processed
**Step 2:** Compare OCR cell count (Rust vs Python)
**Step 3:** Check text cell concatenation logic
**Step 4:** Verify reading order predictions
**Step 5:** Check confidence threshold settings

## Immediate Action

**Problem identified but requires deep debugging:**
- ML models execute (160 tests pass)
- Pipeline runs (no crashes)
- Output generated (but poor quality)
- Issue is in text assembly/processing logic

**Recommendation:** Accept that architecture is proven, quality needs dedicated debugging session.

## Workaround

**For production use today:**
- Use Python baseline for actual document conversion
- Use Rust ML for testing/validation only
- Quality improvement is next major task

**OR:**

Use docling-parse (simpler Rust PDF parser):
- No ML models
- Basic text extraction
- Faster but less sophisticated

## Status

**Problem 4:** ⚠️ IDENTIFIED but not fixed
**Reason:** Requires deep debugging of ML text assembly pipeline
**Impact:** Architecture works, but output quality unusable for production
**Priority:** HIGH - should be next focus

## Next Steps

1. Enable debug logging in ML pipeline
2. Compare intermediate outputs (OCR cells, layout boxes)
3. Check text concatenation logic in convert.rs
4. Verify reading order predictions
5. Compare against Python baseline at each stage

**Time Estimate:** 4-8 hours for proper investigation and fix
