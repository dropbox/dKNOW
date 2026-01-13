# VSDX Improvement Plan (N=1648+)

**Goal:** Improve VSDX parser from 64% to 95% LLM quality score
**Current Score:** 64% (Structure: 50/100, major gap)
**Estimated Effort:** 3-4 commits

---

## Current Implementation

**Location:** `crates/docling-microsoft-extended/src/visio.rs`

**What it does:**
- ✅ Extracts shape text from `<Text>` elements
- ✅ Extracts PinY position for vertical sorting
- ✅ Supports multi-page diagrams (`visio/pages/page*.xml`)
- ✅ Generates DocItems (Text items with shape text)
- ✅ Handles unicode, XML entities, long text

**What it's missing:**
- ❌ Shape connections (connectors between shapes)
- ❌ Shape metadata (size, position beyond PinY, color, type)
- ❌ Diagram hierarchy representation in DocItems
- ❌ Connector DocItems (arrows, lines between shapes)
- ❌ Layer information
- ❌ Shape IDs and references

---

## LLM Feedback (N=1648)

**Overall Score:** 64%

**Category Scores:**
- Completeness: 70/100
- Accuracy: 80/100
- **Structure: 50/100** ← Major gap
- Formatting: 60/100
- Metadata: 80/100

**Specific Gaps:**
1. "Not all pages, shapes, and text content may be present"
2. "Diagram hierarchy (pages, layers) not preserved"
3. "Shapes and connectors not properly structured"

---

## VSDX File Structure

**VSDX is Office Open XML (ZIP-based):**

```
diagram.vsdx/
├── visio/
│   ├── document.xml          # Overall document metadata
│   ├── pages/
│   │   ├── pages.xml         # Page list
│   │   ├── page1.xml         # Page 1 shapes
│   │   ├── page2.xml         # Page 2 shapes
│   │   └── ...
│   ├── masters/              # Master shapes (templates)
│   └── ...
├── _rels/                    # Relationships (connections)
│   └── ...
└── [Content_Types].xml
```

**Key XML Elements:**

### Shapes (`<Shape>` in page*.xml)
```xml
<Shape ID="1" Type="Shape">
  <Cell N="PinX" V="3.0"/>      <!-- X position -->
  <Cell N="PinY" V="5.0"/>      <!-- Y position -->
  <Cell N="Width" V="2.0"/>     <!-- Width -->
  <Cell N="Height" V="1.0"/>    <!-- Height -->
  <Text>Shape Label</Text>      <!-- Text content -->
</Shape>
```

### Connectors (`<Shape Type="..." >` with connector master)
```xml
<Shape ID="2" Type="Shape" Master="4">  <!-- Master=4 might be connector template -->
  <Cell N="BeginX" V="4.0"/>            <!-- Start point -->
  <Cell N="BeginY" V="5.0"/>
  <Cell N="EndX" V="6.0"/>              <!-- End point -->
  <Cell N="EndY" V="7.0"/>
  <Section N="Connection">              <!-- Connection metadata -->
    <Row T="Connection" IX="0">
      <Cell N="X" V="..." F="..."/>
      <Cell N="Y" V="..." F="..."/>
    </Row>
  </Section>
</Shape>
```

### Connects (relationships between shapes)
```xml
<Connects>
  <Connect FromSheet="2" FromCell="BeginX" ToSheet="1" ToCell="PinX"/>
  <Connect FromSheet="2" FromCell="EndX" ToSheet="3" ToCell="PinX"/>
</Connects>
```

**FromSheet/ToSheet:** Shape IDs
**FromCell/ToCell:** Connection points

---

## Implementation Plan

### Phase 1: Extract Shape Metadata (1 commit)

**Goal:** Capture full shape properties, not just text.

**Changes to `VisioShape` struct:**
```rust
struct VisioShape {
    id: Option<String>,        // Shape ID for referencing
    text: String,              // Existing
    pin_x: Option<f64>,        // X position (NEW)
    pin_y: Option<f64>,        // Existing
    width: Option<f64>,        // NEW
    height: Option<f64>,       // NEW
    shape_type: Option<String>, // NEW (for connectors)
    master: Option<String>,    // NEW (template reference)
}
```

**Parser changes:**
- Extract `Cell` elements with names: `PinX`, `PinY`, `Width`, `Height`
- Extract `Shape` attributes: `ID`, `Type`, `Master`

**DocItem generation:**
- Add metadata to Text DocItems (position, size)
- Or: Use Picture DocItem for shapes with geometric properties?

**Estimated time:** 2-3 hours

---

### Phase 2: Extract and Represent Connections (1-2 commits) ✅ COMPLETED N=1674

**Goal:** Parse connector shapes and their relationships. ✅

**Implementation (N=1674):**
- ✅ Connector chain resolution algorithm
- ✅ Parse Begin/End connection points
- ✅ Resolve connectors: `Begin→A, End→B` means `A→B`
- ✅ Support labeled edges: `[Source] -[Label]→ [Target]`

**Results:**
- Before: 22 connections with "[Unknown]" connectors
- After: 11 clean shape-to-shape connections
- Examples: `[Hiring need reported] → [Log hiring request]`
- Decision branches: `[Candidate accepts?] -[Yes]→ [Hire candidate]`

**Estimated quality improvement:** 64% → 75-80% (awaiting LLM test verification)

---

### Phase 3: Represent Diagram Hierarchy (1 commit)

**Goal:** Preserve page structure and layers in DocItems.

**Current behavior:**
- All shapes from all pages concatenated into flat list
- No distinction between pages

**Desired behavior:**
- Group shapes by page
- Represent pages as sections/chapters
- Preserve page order

**DocItem representation:**
Options:
1. Use `Section` DocItems for pages
2. Use `parent`/`children` relationships in DocItems
3. Add page metadata to shape DocItems

**Example structure:**
```
DocItem::Section {
    text: "Page 1",
    children: [shapes on page 1],
}
DocItem::Section {
    text: "Page 2",
    children: [shapes on page 2],
}
```

**Estimated time:** 2-3 hours

---

### Phase 4: Testing and Refinement (incorporated into above)

**Test improvements:**
- Add test for multi-page with hierarchy
- Add test for connectors between shapes
- Add test for shape metadata extraction
- Update LLM test to verify 90%+ score

**Estimated time:** 1-2 hours (distributed across commits)

---

## Total Effort Estimate

**3-4 commits:**
1. Extract shape metadata (PinX, Width, Height, ID)
2. Parse and represent connections
3. Represent diagram hierarchy (pages as sections)
4. (Optional) Refinement based on LLM feedback

**Total time:** 8-12 hours AI execution

---

## Open Questions

### Q1: How to represent connections in DocItems?

**Option A:** Add as metadata to Text DocItems
```rust
DocItem::Text {
    text: "Shape A",
    metadata: {
        "connects_to": ["Shape B", "Shape C"]
    }
}
```

**Option B:** Use markdown links
```markdown
Shape A → Shape B
Shape A → Shape C
```

**Option C:** New DocItem variant
```rust
DocItem::Connection {
    from: "Shape A",
    to: "Shape B",
    connector_text: "→",
}
```

**Recommendation:** Start with Option B (markdown links) for simplicity, can enhance later.

---

### Q2: Should we preserve connector labels?

**Context:** Connectors can have text labels (e.g., "Yes", "No" on decision flowchart arrows)

**Recommendation:** YES - connectors are shapes too, extract their text.

---

### Q3: How to handle shape IDs in markdown?

**Context:** Shape IDs are internal (e.g., "Sheet.5"), not user-facing.

**Options:**
- Use shape text as identifier: `[Process Step] → [Decision]`
- Use numbered identifiers: `[Shape 1] → [Shape 2]`
- Use auto-generated names: `[Shape A] → [Shape B]`

**Recommendation:** Use shape text if available, fallback to `[Shape N]` for text-less shapes.

---

## Success Criteria

**LLM Quality Test:**
- Overall score: 90%+ (target 95%)
- Structure score: 85%+ (currently 50%)
- Completeness: 85%+ (currently 70%)

**Functional Requirements:**
- ✅ All shapes extracted (with text and metadata)
- ✅ All connectors identified (with source/target)
- ✅ Pages represented as hierarchy
- ✅ Markdown output shows diagram structure

---

## References

**VSDX Format Specification:**
- [MS-VSDX]: Visio Drawing File Format
- https://learn.microsoft.com/en-us/office/client-developer/visio/visio-file-format-reference

**Python docling VSDX backend:**
- Location: `~/docling/docling/backend/visio_backend.py`
- Approach: Uses LibreOffice conversion to PDF (NOT pure Python parsing)
- Our approach: Pure Rust XML parsing (better!)

**Current Implementation:**
- `crates/docling-microsoft-extended/src/visio.rs`

---

## Status Update

**N=1674 Status:**
- ✅ Phase 1: Shape metadata extraction (completed N=1672)
- ✅ Phase 2: Connector resolution (completed N=1674)
- ⏸️ Phase 3: Hierarchy improvements (deferred - test quality first)

**Next Steps:**
1. Run LLM quality test on VSDX (verify 90%+ score)
2. If below 90%, implement Phase 3 (hierarchy)
3. If above 90%, move to next priority format (TEX 66%)

---

📊 Generated with Claude Code (N=1648)
https://claude.com/claude-code

Co-Authored-By: Claude <noreply@anthropic.com>
