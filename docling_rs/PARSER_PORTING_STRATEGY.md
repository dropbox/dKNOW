# Parser Porting Strategy

**For Phase 2+: Porting PDF, DOCX, HTML, and other format parsers from Python to Rust**

## Core Principle: Line-by-Line Translation

When porting parsers (not serializers), use **strict line-by-line translation** from Python source to Rust.

## Why Line-by-Line?

**Parsers are complex:**
- Handle malformed input
- Deal with edge cases you won't think of
- Have subtle logic that's hard to rediscover
- Bugs compound - one missed case breaks everything downstream

**Python docling has been battle-tested:**
- Handles real-world documents
- Edge cases already discovered and fixed
- Logic is proven correct

**Your job:** Port the working algorithm, don't reinvent it.

## The Approach

### 1. Read First, Code Second

**Before writing any Rust code:**
1. Read the entire Python file you're porting
2. Understand the overall structure and flow
3. Identify key functions and their purpose
4. Note dependencies and helper functions

**Example for PDF parser:**
```bash
# Read the Python source
less ~/docling/docling/backend/docling_parse_v4_backend.py

# Understand the main entry point
# Trace through the parsing logic
# Identify helper functions
```

### 2. Create a Function Map

Document the Python → Rust mapping before you start:

```
Python Source: ~/docling/docling/backend/docling_parse_v4_backend.py

Function Map:
- DoclingParseV4Backend.__init__() → PdfBackend::new()
- DoclingParseV4Backend.__call__() → PdfBackend::parse()
- _extract_text_from_page() → extract_page_text()
- _process_table_structure() → process_table_structure()
- _apply_ocr_to_region() → apply_ocr()
```

Save this to `reports/{branch}/python_rust_function_map_{parser_name}.md`

### 3. Port Function-by-Function

**For each Python function:**

1. **Copy the Python code as comments** in your Rust file:
   ```rust
   // Python source (markdown.py:120-145):
   // def serialize_title(self, item):
   //     text = self._get_text(item)
   //     level = item.get("level", 1)
   //     prefix = "#" * level
   //     return f"{prefix} {text}\n"

   fn serialize_title(&self, item: &DocItem) -> String {
       let text = self.get_text(item);
       let level = item.level.unwrap_or(1);
       let prefix = "#".repeat(level);
       format!("{} {}\n", prefix, text)
   }
   ```

2. **Translate line-by-line:**
   - Keep the same logic flow
   - Use equivalent Rust idioms
   - Preserve comments and structure
   - Don't try to "improve" it yet

3. **Add source citation in code:**
   ```rust
   /// Extracts text from a PDF page
   ///
   /// Python source: docling_parse_v4_backend.py:234-267
   fn extract_page_text(&self, page: &Page) -> Result<String> {
       // ...
   }
   ```

### 4. What NOT to Do

**Don't:**
- ❌ "Improve" the algorithm during porting
- ❌ Skip functions that "seem unnecessary"
- ❌ Reorder logic "to make more sense"
- ❌ Combine multiple Python functions into one Rust function
- ❌ Use tests to figure out what the parser should do

**Why?** You'll miss edge cases and subtle logic. Port first, optimize later.

### 5. Validation Strategy

**After porting a complete module:**

1. **Unit tests** - Port Python's unit tests if they exist
2. **JSON comparison** - Compare Rust output vs Python output structure:
   ```bash
   # Generate both outputs
   python -m docling convert test.pdf --to json > python.json
   cargo run -- convert test.pdf --to json > rust.json

   # Compare structure
   diff <(jq -S . python.json) <(jq -S . rust.json)
   ```
3. **Integration tests** - Use existing test corpus

### 6. Document Differences

If you MUST deviate from Python (due to Rust constraints), document it:

```rust
/// NOTE: Python uses mutable global state here (docling_parse_v4_backend.py:145)
/// Rust implementation uses a struct field instead to avoid unsafe code.
/// Behavior is identical but implementation differs.
```

## Example: Porting PDF Backend

**Phase 2 checklist:**

```
[ ] Read docling_parse_v4_backend.py completely
[ ] Create function map document
[ ] Port __init__() → new()
[ ] Port __call__() → parse()
[ ] Port _extract_text_from_page() → extract_page_text()
[ ] Port _process_table_structure() → process_table_structure()
[ ] Port _handle_image_extraction() → extract_images()
[ ] Add Python source citations to all functions
[ ] Compare JSON output on 10 test files
[ ] Run integration tests
[ ] Document any deviations from Python
```

## When to Deviate

**Acceptable reasons to change the algorithm:**
1. Python uses C library that doesn't exist in Rust (find equivalent)
2. Python uses mutable global state (use Rust struct fields)
3. Python uses dynamic typing in a way that can't translate (use enums)
4. Performance is catastrophically bad (document why and fix)

**Always document the deviation and why it was necessary.**

## Summary

**For parsers:**
- Read Python source completely before coding
- Create function map document
- Copy Python code as comments, translate line-by-line
- Cite Python source line numbers
- Don't improve during initial port
- Validate with JSON comparison + integration tests
- Document any deviations

**For serializers:**
- Balanced approach OK (test-driven + source reading)
- More flexibility in implementation
- Output matching is the main goal

---

**This strategy applies to:**
- Phase 2: PDF parser (docling_parse_v4_backend.py)
- Phase 3: Office format parsers (DOCX, PPTX, XLSX, HTML backends)
- Phase 4: ML/OCR integration

**Date:** 2025-10-23
**Status:** Planning document for future phases
