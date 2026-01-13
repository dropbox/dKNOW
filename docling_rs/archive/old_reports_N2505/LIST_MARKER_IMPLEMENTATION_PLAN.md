# List Marker Implementation Plan

**Created:** N=1227
**Priority:** HIGH - Blocks 95% DocItem completeness target
**Current Status:** 93% completeness (need 95%+)

---

## Problem Statement

**Issue:** DOCX backend does not extract list marker information to DocItems

**Evidence:**
- LLM DocItem validation: Structure score 90/100 (target: 95+)
- Finding: "List items do not have markers or enumeration preserved"
- Code location: `crates/docling-backend/src/docx.rs:1167`
  ```rust
  let marker = String::new();  // ❌ Empty marker!
  ```
- Comment admits: "Both numbered and bullet lists use empty marker in DocItem"

**Impact:**
- DocItem JSON export incomplete (marker field empty)
- `enumerated` field always false (should be true for numbered lists)
- Other systems reading DocItem JSON won't know list formatting

---

## Current Rust Implementation

### What Works ✅

**Extraction from XML (lines 474-502):**
```rust
// Already extracts from <w:numPr>:
b"w:numId" => {
    // Extract list numbering ID from <w:numPr><w:numId w:val="19"/>
    paragraph_builder.num_id = Some(num_id);
}
b"w:ilvl" => {
    // Extract list indentation level from <w:numPr><w:ilvl w:val="0"/>
    paragraph_builder.ilvl = Some(ilvl);
}
```

**Data Available:**
- `num_id: Option<i32>` - Numbering definition ID
- `ilvl: Option<i32>` - Indentation level (0, 1, 2 for nested lists)

### What's Missing ❌

**No numbering.xml parsing:**
- Current: Always sets `marker = String::new()` (line 1167)
- Current: Hardcoded heuristic for `is_numbered` (lines 1115-1121)
- Needed: Parse `word/numbering.xml` to get actual format

**Hack in current code (lines 1115-1121):**
```rust
let is_numbered = if let Some(20) = self.num_id {
    false // numId=20 is bullets in this document
} else {
    self.style_name.as_ref().map_or(false, |s| {
        s.to_lowercase().contains("number")
    })
};
```
This is document-specific and wrong!

---

## Python Reference Implementation

**File:** `~/docling/docling/backend/msword_backend.py`

### Key Methods

**1. `_is_numbered_list(docx_obj, numId, ilvl)` (lines 387-470)**

Determines if list is numbered or bullet:

```python
# Parse word/numbering.xml
numbering_part = find_part("numbering")
numbering_root = numbering_part.element

# Find numbering definition by numId
num_element = numbering_root.find(f".//w:num[@w:numId='{numId}']")
abstract_num_id = num_element.find(".//w:abstractNumId").get("w:val")

# Find abstract numbering definition
abstract_num = numbering_root.find(
    f".//w:abstractNum[@w:abstractNumId='{abstract_num_id}']"
)

# Get level definition for ilvl
lvl_element = abstract_num.find(f".//w:lvl[@w:ilvl='{ilvl}']")
num_fmt = lvl_element.find(".//w:numFmt").get("w:val")

# Check format type
numbered_formats = {
    "decimal",         # 1, 2, 3
    "lowerRoman",      # i, ii, iii
    "upperRoman",      # I, II, III
    "lowerLetter",     # a, b, c
    "upperLetter",     # A, B, C
    "decimalZero",     # 01, 02, 03
}
return num_fmt in numbered_formats
```

**2. `_add_list_item(doc, numid, ilevel, elements, is_numbered)` (lines 1143-1240)**

Generates marker string:

```python
if is_numbered:
    counter = self._get_list_counter(numid, ilevel)
    enum_marker = str(counter) + "."  # "1.", "2.", "3."
else:
    enum_marker = ""  # Bullet (empty in DocItem, serializer adds "- ")

self._add_formatted_list_item(
    doc, elements, enum_marker, is_numbered, level
)
```

**3. `_add_formatted_list_item(doc, elements, marker, enumerated, level)` (lines 1098-1142)**

Creates DocItem with marker:

```python
doc.add_list_item(
    marker=marker,          # "1.", "2.", "" for bullets
    enumerated=enumerated,  # True/False
    parent=self.parents[level],
    text=text,
)
```

### Counter Management

Python tracks counters per numId/ilvl:
```python
self._reset_list_counters_for_new_sequence(numid)
counter = self._get_list_counter(numid, ilevel)
```

Each numbered list has independent counter that increments.

---

## Implementation Plan for Rust

### Phase 1: Parse numbering.xml (2-3 hours)

**Add new module:** `crates/docling-backend/src/docx_numbering.rs`

**Struct definitions:**
```rust
#[derive(Debug, Clone)]
pub struct NumberingDefinitions {
    /// Map numId → AbstractNumId
    num_map: HashMap<i32, i32>,
    /// Map AbstractNumId → AbstractNum
    abstract_nums: HashMap<i32, AbstractNum>,
}

#[derive(Debug, Clone)]
pub struct AbstractNum {
    abstract_num_id: i32,
    /// Map ilvl → LevelDefinition
    levels: HashMap<i32, LevelDefinition>,
}

#[derive(Debug, Clone)]
pub struct LevelDefinition {
    ilvl: i32,
    num_fmt: NumFormat,
    start_val: i32,  // Starting number (usually 1)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NumFormat {
    Decimal,        // 1, 2, 3
    LowerRoman,     // i, ii, iii
    UpperRoman,     // I, II, III
    LowerLetter,    // a, b, c
    UpperLetter,    // A, B, C
    DecimalZero,    // 01, 02, 03
    Bullet,         // •, ◦, ▪
}

impl NumFormat {
    pub fn is_numbered(&self) -> bool {
        !matches!(self, NumFormat::Bullet)
    }
}
```

**Parser function:**
```rust
pub fn parse_numbering_xml(
    zip_archive: &mut ZipArchive<impl Read + Seek>
) -> Result<NumberingDefinitions> {
    let mut xml_file = zip_archive.by_name("word/numbering.xml")?;
    let mut xml_content = String::new();
    xml_file.read_to_string(&mut xml_content)?;

    // Use quick-xml to parse
    let mut reader = Reader::from_str(&xml_content);

    // Parse <w:num> elements (numId → abstractNumId)
    // Parse <w:abstractNum> elements (definitions)
    // Parse <w:lvl> elements (level definitions)
    // Parse <w:numFmt> elements (format type)

    Ok(NumberingDefinitions { ... })
}
```

**Integration:**
```rust
impl DocumentBackend for DocxBackend {
    fn parse_file(&self, path: &str, options: &BackendOptions)
        -> Result<DoclingDocument>
    {
        let file = File::open(path)?;
        let mut zip = ZipArchive::new(file)?;

        // NEW: Parse numbering definitions
        let numbering = parse_numbering_xml(&mut zip)
            .unwrap_or_else(|_| NumberingDefinitions::empty());

        // Pass numbering to paragraph processing
        self.parse_document_xml(&mut zip, numbering)?
    }
}
```

### Phase 2: Counter Management (1-2 hours)

**Add counter tracking:**
```rust
#[derive(Debug)]
struct ListCounters {
    /// Map (numId, ilvl) → current counter value
    counters: HashMap<(i32, i32), i32>,
}

impl ListCounters {
    fn reset_for_sequence(&mut self, num_id: i32) {
        // Reset all counters for this numId
        self.counters.retain(|(nid, _), _| *nid != num_id);
    }

    fn get_and_increment(&mut self, num_id: i32, ilvl: i32) -> i32 {
        let counter = self.counters.entry((num_id, ilvl)).or_insert(1);
        let value = *counter;
        *counter += 1;
        value
    }
}
```

### Phase 3: Marker Generation (1-2 hours)

**Generate marker strings:**
```rust
fn generate_marker(
    numbering: &NumberingDefinitions,
    counters: &mut ListCounters,
    num_id: i32,
    ilvl: i32,
) -> (String, bool) {
    let level_def = numbering.get_level(num_id, ilvl);

    if level_def.num_fmt.is_numbered() {
        let counter = counters.get_and_increment(num_id, ilvl);
        let marker = match level_def.num_fmt {
            NumFormat::Decimal => format!("{}.", counter),
            NumFormat::LowerRoman => format!("{}.", to_lower_roman(counter)),
            NumFormat::UpperRoman => format!("{}.", to_upper_roman(counter)),
            NumFormat::LowerLetter => format!("{}.", to_lower_letter(counter)),
            NumFormat::UpperLetter => format!("{}.", to_upper_letter(counter)),
            NumFormat::DecimalZero => format!("{:02}.", counter),
            _ => String::new(),
        };
        (marker, true)  // enumerated = true
    } else {
        // Bullet list - marker is empty in DocItem
        // (serializer adds "- " or "* " when rendering)
        (String::new(), false)  // enumerated = false
    }
}
```

### Phase 4: Update ParagraphBuilder (1 hour)

**Replace hack with proper implementation:**
```rust
impl ParagraphBuilder {
    fn finalize_paragraph(
        self,
        numbering: &NumberingDefinitions,
        counters: &mut ListCounters,
    ) -> Option<DocItem> {
        // Check if this is a list item
        if let (Some(num_id), Some(ilvl)) = (self.num_id, self.ilvl) {
            // Generate marker (replaces lines 1115-1167)
            let (marker, enumerated) = generate_marker(
                numbering, counters, num_id, ilvl
            );

            Some(DocItem::ListItem {
                // ...
                marker,      // ✅ Now populated!
                enumerated,  // ✅ Now correct!
                // ...
            })
        }
        // ... rest of paragraph logic
    }
}
```

### Phase 5: Testing (2-3 hours)

**1. Unit tests for numbering parser:**
```rust
#[test]
fn test_parse_numbering_xml() {
    let numbering = parse_numbering_xml(&mut zip).unwrap();
    assert_eq!(numbering.get_format(1, 0), NumFormat::Decimal);
    assert_eq!(numbering.get_format(20, 0), NumFormat::Bullet);
}
```

**2. Unit tests for marker generation:**
```rust
#[test]
fn test_generate_decimal_marker() {
    let mut counters = ListCounters::new();
    assert_eq!(generate_marker(&numbering, &mut counters, 1, 0),
               ("1.".to_string(), true));
    assert_eq!(generate_marker(&numbering, &mut counters, 1, 0),
               ("2.".to_string(), true));
}

#[test]
fn test_generate_bullet_marker() {
    let mut counters = ListCounters::new();
    assert_eq!(generate_marker(&numbering, &mut counters, 20, 0),
               (String::new(), false));
}
```

**3. Integration test with real DOCX:**
```rust
#[test]
fn test_docx_list_markers() {
    let doc = DocxBackend.parse_file("test-corpus/docx/word_sample.docx",
                                     &Default::default()).unwrap();

    let list_items: Vec<_> = doc.content_blocks.unwrap()
        .iter()
        .filter_map(|item| match item {
            DocItem::ListItem { marker, enumerated, text, .. } =>
                Some((marker.clone(), *enumerated, text.clone())),
            _ => None,
        })
        .collect();

    // First numbered item should have "1."
    assert_eq!(list_items[0].0, "1.");
    assert_eq!(list_items[0].1, true);

    // First bullet should have "" (serializer adds "- ")
    assert_eq!(list_items[5].0, "");
    assert_eq!(list_items[5].1, false);
}
```

**4. Re-run DocItem validation test:**
```bash
export OPENAI_API_KEY="..." && cargo test -p docling-core --test llm_docitem_validation_tests test_llm_docitem_docx -- --ignored --nocapture --exact
```

Expected: Structure score improves from 90/100 → 95+/100

---

## Estimated Effort

**Total:** 7-11 hours for complete implementation

| Phase | Effort | Description |
|-------|--------|-------------|
| 1. Parse numbering.xml | 2-3 hrs | XML parsing, data structures |
| 2. Counter management | 1-2 hrs | Track counters per numId/ilvl |
| 3. Marker generation | 1-2 hrs | Format-specific marker strings |
| 4. Update ParagraphBuilder | 1 hr | Replace hack with proper logic |
| 5. Testing | 2-3 hrs | Unit tests, integration tests, validation |

**Prerequisite knowledge:**
- DOCX XML structure (already understood)
- quick-xml crate usage (already used in docx.rs)
- Python reference code (documented in this file)

---

## Test Validation

**Success Criteria:**

1. **Unit tests pass** ✅
   - Numbering XML parsing works
   - Marker generation correct for all formats

2. **Integration tests pass** ✅
   - DOCX backend tests still pass (2836/2836)
   - List items have proper markers
   - Enumerated flag correct

3. **DocItem validation improves** ✅
   - LLM DocItem test: Structure score 90 → 95+
   - Overall completeness: 93% → 95%+
   - No new findings about list markers

4. **Markdown output unchanged** ✅
   - Canonical tests still pass (97/97)
   - Serializer handles new markers correctly
   - No regression in output quality

---

## Alternative: Simpler Heuristic (NOT RECOMMENDED)

**Could we use a simpler approach?**

**Option:** Detect numbered vs bullet from style name only
```rust
let is_numbered = style_name.contains("number") || style_name.contains("Number");
let marker = if is_numbered { "1." } else { "" };
```

**Why this is WRONG:**
- Style names unreliable (documents use custom styles)
- Can't track counters (all items would be "1.")
- Can't handle nested lists correctly
- Can't support roman numerals, letters, etc.
- Fails LLM validation (would still report missing markers)

**Verdict:** Must implement proper numbering.xml parsing. No shortcuts!

---

## Notes for Next AI (N=1228+)

**Current Status:**
- DocItem validation test works ✅
- Identified gap: List markers not extracted ✅
- Python reference code analyzed ✅
- Implementation plan documented ✅

**Next Steps:**
1. Start with Phase 1 (parse numbering.xml)
2. Add unit tests as you go
3. Implement phases 2-4 incrementally
4. Run DocItem validation test to verify improvement
5. Ensure all existing tests still pass

**Files to modify:**
- `crates/docling-backend/src/docx_numbering.rs` (NEW)
- `crates/docling-backend/src/docx.rs` (update ParagraphBuilder)
- `crates/docling-backend/src/lib.rs` (add docx_numbering module)

**Files to reference:**
- Python: `~/docling/docling/backend/msword_backend.py:387-470, 1143-1240`
- Test: `crates/docling-core/tests/llm_docitem_validation_tests.rs`
- Results: `DOCITEM_VALIDATION_RESULTS_N1227.md`

**Don't:**
- Rush implementation (take time to do it right)
- Skip testing (validate each phase)
- Break existing tests (ensure backward compatibility)

---

**Status:** 📋 **PLAN COMPLETE** - Ready for implementation in N=1228+
