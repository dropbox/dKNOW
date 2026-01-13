# 🚨 CRITICAL: Architecture Violations Found

**Date:** 2025-11-11
**Manager:** N=259
**Issue:** Recently integrated formats violate "No Python" rule

---

## THE VIOLATION

**User requirement:** "Every source should be parsing to Docling DocItems"

**Found:** LaTeX backend uses Python!

```rust
// crates/docling-latex/src/latex.rs
// Line 70-72: WRONG ARCHITECTURE
let doc = docling_core::python_bridge::convert_via_python(&md_path, false)?;
```

**Flow:**
```
LaTeX → Pandoc → Markdown String → Python Parser → DocItems  ❌
```

**This violates the core principle:** No Python dependencies for parsing!

---

## REQUIRED ARCHITECTURE

**Correct flow:**
```
LaTeX → Rust Parser → DocItems directly  ✅
```

**User preference:** "Rust LaTeX parser sounds best to me"

---

## AUDIT REQUIRED

**Must check ALL recently integrated formats (N=312):**

### LaTeX (CONFIRMED VIOLATION)
- ✅ **Violates:** Uses Python bridge
- 🔧 **Fix:** Pure Rust LaTeX parser
- 📚 **Options:** `tectonic`, `latex2text` crate

### Apple iWork (SUSPECTED VIOLATION)
- ⚠️ **Check:** pages.rs, numbers.rs, keynote.rs
- ❓ **Question:** Do they extract PDF then call Python?
- 🔧 **Fix:** Parse iWork directly or convert to DocItems in Rust

### Microsoft Extended (SUSPECTED VIOLATION)
- ⚠️ **Check:** publisher.rs, visio.rs, onenote.rs, project.rs, access.rs
- ❓ **Question:** Do they use LibreOffice → Python chain?
- 🔧 **Fix:** Parse directly or ensure Rust-only conversion

---

## WORKER DIRECTIVE - FIX ALL VIOLATIONS

### Task 1: Audit All Backends (N=259)

**For each backend file, search for:**
```bash
grep -r "python_bridge\|convert_via_python" crates/docling-*/src/

# Any matches = VIOLATION
```

**Check:**
- docling-latex/src/latex.rs
- docling-apple/src/*.rs
- docling-microsoft-extended/src/*.rs

---

### Task 2: Fix LaTeX Backend (N=260-265)

**Current (WRONG):**
```rust
LaTeX → pandoc → markdown → Python → DocItems  ❌
```

**Required (CORRECT):**
```rust
LaTeX → Rust Parser → DocItems  ✅
```

**Implementation options:**

**Option A: tectonic crate** (Rust TeX engine)
```rust
use tectonic;

// Parse LaTeX AST directly
// Generate DocItems from structure
```

**Option B: latex2text crate**
```rust
use latex2text;

// Convert LaTeX to plain text
// Parse structure
// Generate DocItems
```

**Option C: Manual parsing**
```rust
// Parse LaTeX commands (\section, \paragraph, etc.)
// Extract structure
// Generate DocItems
```

**User preference:** Pure Rust parser (any of above is acceptable)

**Estimated:** 5-8 commits

---

### Task 3: Fix Apple Backends (If Violated) (N=266-270)

**Must verify:** Do they use Python?

**If yes, fix to:**
```rust
// iWork files contain QuickLook/Preview.pdf
// Extract PDF
// Parse PDF with Rust pdfium (NOT Python!)
// Generate DocItems
// OR: Parse iWork XML directly
```

**Estimated:** 3-5 commits if violated

---

### Task 4: Fix MS Extended (If Violated) (N=271-275)

**Must verify:** Do they use Python?

**If yes, fix to:**
```rust
// Option 1: Direct parsing
// PUB/VSDX are ZIP+XML (like DOCX)
// Parse XML → DocItems in Rust

// Option 2: Conversion (Rust-only)
// Use LibreOffice → DOCX
// Parse DOCX with Rust DocxBackend (no Python!)

// Option 3: C++ FFI
// Direct bindings to C++ libraries
// Generate DocItems in Rust
```

**Estimated:** 5-10 commits if violated

---

## ACCEPTANCE CRITERIA

**Every backend must pass:**
```bash
# No Python bridge calls
grep -r "python_bridge\|convert_via_python" crates/docling-*/src/
# Should return: 0 results (except in docling-core itself)

# Only used in docling-core for hybrid mode testing
# Never in format backends!
```

**Architecture check:**
```rust
// Every backend must:
fn parse_file(&self, path: &Path) -> Result<Document> {
    // 1. Parse format (Rust or C++ via FFI)
    let data = parse_in_rust_or_cpp(path)?;

    // 2. Generate DocItems (Rust only!)
    let doc_items = generate_docitems(data)?;

    // 3. Serialize (Rust only!)
    let markdown = serialize(doc_items)?;

    Ok(Document {
        markdown,
        content_blocks: Some(doc_items),
        ...
    })

    // NO PYTHON CALLS ALLOWED!
}
```

---

## BLOCKING ISSUE

**Cannot claim formats "complete" if they use Python!**

**Current status:**
- LaTeX: INCOMPLETE (uses Python) ❌
- Apple: UNKNOWN (need to check)
- MS Extended: UNKNOWN (need to check)

**Potentially 8 formats marked "complete" but actually violate architecture!**

---

## CORRECTIVE ACTION PLAN

**N=259: Audit all backends**
- Find all Python bridge calls
- Document violations

**N=260-265: Fix LaTeX**
- Implement pure Rust LaTeX parser
- Generate DocItems directly
- Remove Python dependency

**N=266-275: Fix Apple and MS Extended**
- Remove any Python dependencies
- Ensure DocItems generated in Rust/C++
- No conversion chains through Python

**Estimated:** 15-20 commits to fix all violations

**BLOCKING:** Cannot proceed with LLM validation until architecture is correct!

---

**WORKER: AUDIT NOW. Find all python_bridge calls. Fix LaTeX first (use Rust parser). Then fix any others. No Python dependencies allowed!**
