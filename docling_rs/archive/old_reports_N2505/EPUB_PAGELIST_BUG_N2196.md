# EPUB PageList Bug - Missing Illustrations Index (N=2196)

## Summary

Found REAL bug in EPUB parser: **pageList (illustrations index) not extracted**.

## LLM Test Result

**EPUB Score: 84%** (below 85% threshold)

**Complaints:**
1. ✅ **REAL**: "Does not include the full list of illustrations" 
2. ❓ **UNCLEAR**: "Some chapter links are incomplete"

## Bug Analysis

### What's Missing

EPUB files have TWO navigation structures in `toc.ncx`:
1. **`<navMap>`**: Chapter navigation (✅ EXTRACTED)
2. **`<pageList>`**: Page/illustration references (❌ NOT EXTRACTED)

### Source File Evidence

`test-corpus/ebooks/epub/simple.epub` (Pride and Prejudice):
- Has 63 chapter entries in `<navMap>` ✅
- Has 324 page targets in `<pageList>` ❌ NOT EXTRACTED
- Contains ~27 illustration images (.jpg/.png files)

**Example from toc.ncx:**
```xml
<pageList id="pages" class="pagelist">
  <navLabel>
    <text>Pages</text>
  </navLabel>
  <pageTarget id="pt-1" value="1" type="front" playOrder="2">
    <navLabel>
      <text>{vii}</text>
    </navLabel>
    <content src="8546256623413036110_1342-h-0.htm.xhtml#pgepubid00003"/>
  </pageTarget>
  <!-- ... 324 total page targets ... -->
</pageList>
```

### Current Code Issue

**File:** `crates/docling-ebook/src/epub.rs:79-91`

```rust
/// Extract table of contents from EPUB
fn extract_toc(doc: &EpubDoc<std::io::BufReader<std::fs::File>>) -> Result<Vec<TocEntry>> {
    let mut toc = Vec::new();

    // Get table of contents from EPUB
    // The epub crate provides toc() method that returns Vec<NavPoint>
    for (spine_order, nav_point) in doc.toc.iter().enumerate() {
        let entry = extract_toc_entry(nav_point, Some(spine_order));
        toc.push(entry);
    }

    Ok(toc)
}
```

**Problem:** `doc.toc` only returns `<navMap>` entries. The `pageList` is not accessed.

## Fix Required

The `epub` crate (external dependency) likely doesn't expose `pageList` in its API. Two options:

### Option 1: Parse toc.ncx Manually (RECOMMENDED)

Add a function to extract pageList manually from ZIP:

```rust
/// Extract page list (illustrations index) from EPUB
fn extract_page_list(archive: &mut ZipArchive<...>) -> Result<Vec<PageEntry>> {
    // 1. Extract toc.ncx from EPUB ZIP
    // 2. Parse XML to find <pageList> section  
    // 3. Extract <pageTarget> entries
    // 4. Return as Vec<PageEntry> with page numbers and links
}
```

Then include in markdown output as "## List of Illustrations" section.

### Option 2: Check epub Crate API

Check if `epub` crate has pageList support:
```rust
// Check if epub::doc::EpubDoc has page_list() or similar method
```

If not, use Option 1.

## Expected Improvement

Current: **84%** (Completeness: 90/100)
After fix: **~90%+** (Completeness should reach 95-100/100)

The pageList is substantial (324 entries) and LLM correctly identified it as missing.

## Next Steps for AI

1. ✅ Verify pageList exists in test file (CONFIRMED)
2. **Implement pageList extraction** (Option 1 recommended)
3. **Add "List of Illustrations" section** to markdown output
4. **Re-test with LLM** to verify improvement
5. **Check other EPUB files** (complex.epub, with_images.epub) for similar issues

## Files to Modify

- `crates/docling-ebook/src/epub.rs` - Add extract_page_list()
- `crates/docling-backend/src/ebooks.rs` - Include pageList in markdown generation

## Additional Finding

The TOC labels have weird formatting issues (already handled by normalize_toc_label):
- "I hope Mr. Bingley will like it. CHAPTER II." → "Chapter II" ✅
- "CHAPTERXXVII" (no space) → "Chapter XXVII" ✅

This normalization is working correctly.

## Conclusion

**This is a REAL bug**, not LLM variance. The EPUB parser is missing a significant feature (pageList extraction) that exists in the source file.
