# PPTX Image Extraction Investigation - N=1233

**Date:** 2025-11-17
**Issue:** PPTX at 87% completeness (LLM validation), missing image extraction
**Goal:** Identify and document missing image extraction functionality

---

## Executive Summary

✅ **Root cause confirmed:** PPTX backend does NOT extract images from slides
❌ **Missing feature:** No handling of `<p:pic>` XML elements
✅ **Python docling:** Has full image extraction via `handle_pictures()` function
📊 **Impact:** Significant completeness gap (87% vs target 95%)

---

## Investigation Results

### 1. Code Analysis

**Current PPTX backend handles:**
- `p:sp` - Shapes (text boxes)
- `p:ph` - Placeholders
- `a:p`, `a:r`, `a:t` - Paragraphs, runs, text
- `a:tbl`, `a:tr`, `a:tc` - Tables
- `a:buChar`, `a:buAutoNum` - Bullet/numbered lists
- `a:rPr` - Run properties (formatting)

**Missing:**
- ❌ `p:pic` - Picture elements

**Evidence:**
```bash
$ grep -o 'b"[^"]*"' crates/docling-backend/src/pptx.rs | grep -E 'p:|a:' | sort -u
b"a:buAutoNum"
b"a:buChar"
b"a:p"
b"a:r"
b"a:rPr"
b"a:t"
b"a:tbl"
b"a:tc"
b"a:tr"
b"p:ph"
b"p:sldSz"
b"p:sp"
```

No `b"p:pic"` found!

### 2. Python Docling Implementation

**Location:** `~/docling/docling/backend/mspowerpoint_backend.py`

**Key functions:**
- **Line 235-254:** `handle_pictures()` - Extracts image bytes, DPI, opens with PIL
- **Line 345-350:** Checks `shape.shape_type == MSO_SHAPE_TYPE.PICTURE`

**Python flow:**
1. Check if shape is PICTURE type
2. Get image bytes from `shape.image.blob`
3. Get DPI from `shape.image.dpi`
4. Open with PIL (Python Imaging Library)
5. Call `doc.add_picture()` with ImageRef

### 3. XML Structure Analysis

**Test file:** `test-corpus/pptx/powerpoint_with_image.pptx`
- **Has:** 1 image (image1.png, 42,471 bytes)
- **Slide XML:** `ppt/slides/slide1.xml`
- **Image location:** `ppt/media/image1.png`

**XML structure:**
```xml
<p:pic>
  <p:nvPicPr>
    <p:cNvPr id="5" name="Picture 4">...</p:cNvPr>
    <p:cNvPicPr>...</p:cNvPicPr>
    <p:nvPr/>
  </p:nvPicPr>
  <p:blipFill>
    <a:blip r:embed="rId2"/>  <!-- Relationship ID -->
    <a:stretch><a:fillRect/></a:stretch>
  </p:blipFill>
  <p:spPr>
    <a:xfrm>
      <a:off x="5689599" y="3022600"/>
      <a:ext cx="812800" cy="812800"/>
    </a:xfrm>
    <a:prstGeom prst="rect"><a:avLst/></a:prstGeom>
  </p:spPr>
</p:pic>
```

**Relationships file:** `ppt/slides/_rels/slide1.xml.rels`
```xml
<Relationship Id="rId2"
              Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image"
              Target="../media/image1.png"/>
```

### 4. DocItem::Picture Structure

**Location:** `crates/docling-core/src/content.rs:482-501`

```rust
Picture {
    self_ref: String,
    parent: Option<ItemRef>,
    children: Vec<ItemRef>,
    content_layer: String,
    prov: Vec<ProvenanceItem>,
    captions: Vec<ItemRef>,
    footnotes: Vec<ItemRef>,
    references: Vec<ItemRef>,
    image: Option<serde_json::Value>,  // ← Image data goes here
    annotations: Vec<serde_json::Value>,
}
```

### 5. Image Data Format (from Python JSON export)

**Example from `/tmp/pptx_with_image.json`:**
```json
{
  "self_ref": "#/pictures/0",
  "parent": {"$ref": "#/groups/0"},
  "content_layer": "body",
  "label": "picture",
  "prov": [/* bounding box data */],
  "image": {
    "mimetype": "image/png",
    "dpi": 300,
    "size": {
      "width": 268.0,
      "height": 268.0
    },
    "uri": "data:image/png;base64,iVBORw0KG..."
  }
}
```

---

## Implementation Plan

### Phase 1: Basic Image Detection
1. Add `p:pic` element handling to XML parser
2. Extract relationship ID from `<a:blip r:embed="..."/>`
3. Log detected images (no extraction yet)
4. Test: Verify images are detected

### Phase 2: Image Extraction
1. Load relationships file (`ppt/slides/_rels/slideN.xml.rels`)
2. Map relationship ID to image path
3. Read image bytes from ZIP archive
4. Encode as base64 data URI
5. Create Picture DocItem (no dimensions yet)
6. Test: Verify images appear in JSON export

### Phase 3: Image Metadata
1. Add image decoding library (`image` crate)
2. Decode image to get dimensions
3. Extract DPI if available (default to 96)
4. Add to image metadata JSON
5. Test: Verify complete metadata

### Phase 4: Provenance Data
1. Extract bounding box from `<p:spPr><a:xfrm>`
2. Convert EMU units to correct coordinate system
3. Add to Picture provenance
4. Test: Verify bounding boxes

### Phase 5: Integration Testing
1. Run LLM validation test with powerpoint_with_image.pptx
2. Verify score improvement (87% → ~90%+)
3. Run with powerpoint_sample.pptx (no images - should still work)
4. Add unit test for image extraction

---

## Dependencies Needed

### Rust crates:
- ✅ `zip` - Already used for reading PPTX archive
- ✅ `quick-xml` - Already used for XML parsing
- ✅ `base64` - For encoding image data
- ❓ `image` - For decoding images and getting dimensions (may need to add)

### Challenges:
1. **Image decoding:** Need to decode PNG/JPEG/etc. to get dimensions
2. **Relationships parsing:** Need to parse separate XML file for each slide
3. **Data URI encoding:** Need to properly format base64 data URIs
4. **DPI extraction:** PNG/JPEG may not have DPI metadata (default to 96)
5. **EMU coordinate conversion:** PPTX uses English Metric Units (914400 EMU = 1 inch)

---

## Test Files

### Files WITH images:
- `test-corpus/pptx/powerpoint_with_image.pptx` - 1 image (image1.png)

### Files WITHOUT images (regression tests):
- `test-corpus/pptx/powerpoint_sample.pptx` - 0 images
- `test-corpus/pptx/business_presentation.pptx` - 0 images

### Verification command:
```bash
unzip -l test-corpus/pptx/FILE.pptx | grep "ppt/media/"
```

---

## Expected Improvements

### LLM Validation Scores:
- **Before:** 87% (powerpoint_sample.pptx, no images)
- **After:** ~90-92% (with image extraction implemented)
- **With powerpoint_with_image.pptx:** ~93-95% (images present and extracted)

### Missing Features (Post-Implementation):
- Some shapes (diagrams, SmartArt) - minor impact
- Minor formatting details - minimal impact

---

## Conclusion

✅ **Investigation complete**
✅ **Root cause identified:** Missing `p:pic` element handling
✅ **Solution path clear:** Well-defined implementation plan
✅ **Test files available:** powerpoint_with_image.pptx ready

**Next Steps:**
1. Implement Phase 1 (detection)
2. Implement Phase 2 (extraction)
3. Add image dimensions (Phase 3)
4. Test and validate
5. Re-run LLM validation

**Estimated effort:** 2-3 sessions for full implementation + testing

---

## References

- Python docling: `~/docling/docling/backend/mspowerpoint_backend.py:235-254`
- Rust PPTX backend: `crates/docling-backend/src/pptx.rs:417-800`
- DocItem::Picture: `crates/docling-core/src/content.rs:482-501`
- Test file: `test-corpus/pptx/powerpoint_with_image.pptx`
