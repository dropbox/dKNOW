# HEIF Fix Attempt - N=2299

**Status:** INCOMPLETE - Approach needs adjustment

---

## What I Tried

Changed HEIF metadata items from `Text` to `ListItem` in `crates/docling-backend/src/heif.rs`:

```rust
// OLD: Plain text items
let image_type = format!("Type: {format_name} Image");
doc_items.push(create_text_item(item_index, image_type, vec![]));

// NEW: List items with bullets
let image_type = format!("**Type:** {format_name} Image");
doc_items.push(create_list_item(
    item_index,
    image_type,
    "-".to_string(),  // Bullet marker
    false,            // Not enumerated
    vec![],          // No provenance
    None,            // No parent
));
```

---

## Problem

**List items don't render as bullets in markdown output.**

**Expected output:**
```markdown
## Image Details

- **Type:** HEIF/HEIC Image
- **Brand:** heic
- **Dimensions:** 800x600 pixels
- **File Size:** 4.2 KB
```

**Actual output:**
```markdown
## Image Details

Type: HEIF/HEIC Image

Brand: heic

Dimensions: 800x600 pixels

File Size: 4.2 KB
```

No bullets appear - items render as plain text.

---

## Root Cause (Hypothesis)

The markdown serializer (`crates/docling-core/src/serializer/markdown.rs`) may require ListItems to be inside a `List` group for proper rendering.

**Evidence:**
- Line 357-424: ListItem serialization logic exists
- Line 931-941: References to List groups containing ListItems
- Standalone ListItems might not trigger bullet rendering

**Next step:** Check if ListItems need a parent List group:

```rust
// Might need:
doc_items.push(DocItem::List {
    self_ref: format!("#/groups/{}", index),
    children: vec![
        ItemRef::Text(0), // Reference to Type list item
        ItemRef::Text(1), // Reference to Brand list item
        // etc.
    ],
    enumerated: false,
    // ...
});
```

---

## Alternative Approach (Not Tried)

**Add subsections (level 3 headers)** instead of bullets:

```rust
// Add "Format Information" subsection
doc_items.push(create_section_header(
    item_index,
    "Format Information".to_string(),
    3,  // Level 3 (###)
    vec![],
));
item_index += 1;

// Then Type and Brand as plain text
// ...

// Add "Dimensions and Size" subsection
doc_items.push(create_section_header(
    item_index,
    "Dimensions and Size".to_string(),
    3,
    vec![],
));
item_index += 1;

// Then dimensions and file size as plain text
```

**This approach:**
- Adds visual structure (more ### headers)
- Doesn't require bullets
- May satisfy LLM's "clear distinction between sections" requirement
- Easier to implement (no List group complexity)

---

## Recommendation for Next Worker (N=2300)

**Try subsection approach first:**
1. Add 3 subsections under "Image Details":
   - "### Format Information" (Type, Brand)
   - "### Dimensions and Size" (Dimensions, File Size)
   - "### Content Extraction" (Note about OCR)
2. Keep items as plain Text (not ListItem)
3. Test if structure score improves

**If subsections don't work, try List group approach:**
1. Study how `epub.rs` or other backends use ListItems
2. Check if they wrap ListItems in a List group
3. Implement List group with ListItem children
4. Test rendering

---

## Files to Reference

- **HEIF backend:** `crates/docling-backend/src/heif.rs` (lines 570-720, function `heif_to_docitems`)
- **Markdown serializer:** `crates/docling-core/src/serializer/markdown.rs` (lines 357-424 for ListItem, lines 931-941 for List groups)
- **Utils:** `crates/docling-backend/src/utils.rs` (line 545: `create_list_item`, check if there's `create_list_group`)
- **EPUB backend:** `crates/docling-core/src/ebook.rs` (may have examples of List usage)

---

## Testing Command

```bash
# Generate HEIF output
cargo run --bin docling -p docling-cli -- convert test-corpus/graphics/heif/large_image.heic 2>/dev/null

# Test LLM score
source .env && export $(cat .env | xargs)
cargo test -p docling-core --test llm_verification_tests test_llm_mode3_heif -- --ignored --nocapture
```

---

## Time Estimate

- **Subsection approach:** 15-30 minutes (simple, add 2 section headers)
- **List group approach:** 1-2 hours (need to understand List structure, test)

**Recommendation:** Try subsections first (quick win), fall back to List groups if needed.
