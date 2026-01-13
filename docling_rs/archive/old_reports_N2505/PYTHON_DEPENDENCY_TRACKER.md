# PYTHON DEPENDENCY TRACKER - Must Fix ALL

**Created:** N=260
**Purpose:** Track elimination of Python dependencies
**Rule:** NO format can use python_bridge in backend code

---

## 🚨 VIOLATIONS TO FIX (Priority Order)

### #1: LaTeX ❌ BLOCKED (User Priority)

**Status:** USES PYTHON
**File:** crates/docling-latex/src/latex.rs:71
**Violation:** `python_bridge::convert_via_python()`

**Current flow:**
```
LaTeX → pandoc → markdown → Python parser → DocItems  ❌
```

**Required flow:**
```
LaTeX → Rust parser → DocItems  ✅
```

**User said:** "Rust LaTeX parser sounds best to me"

**Fix options:**
- [ ] Use `tectonic` crate (Rust TeX engine)
- [ ] Use `latex2text` crate
- [ ] Manual LaTeX parser

**Assigned to:** Worker
**Deadline:** Next 5-8 commits
**Test files:** 13 ready in test-corpus/latex/
**Tests to add:** 13 integration tests
**Fixed at N:** _____

---

### #2: Visio (VSDX) ❌ BLOCKED

**Status:** USES PYTHON
**File:** crates/docling-microsoft-extended/src/visio.rs
**Violation:** `python_bridge::convert_via_python()`

**Current flow:**
```
VSDX → LibreOffice → markdown → Python → DocItems  ❌
```

**Required flow:**
```
VSDX → Parse XML → DocItems (Rust)  ✅
```

**Fix:** VSDX is ZIP + XML (like DOCX!), parse directly in Rust

**Assigned to:** Worker
**Deadline:** Next 5-8 commits after LaTeX
**Test files:** 5 ready in test-corpus/microsoft-visio/
**Fixed at N:** _____

---

### #3: Publisher (PUB) ❌ BLOCKED

**Status:** USES PYTHON
**File:** crates/docling-microsoft-extended/src/publisher.rs
**Violation:** `python_bridge::convert_via_python()`

**Current flow:**
```
PUB → LibreOffice → PDF → Python → DocItems  ❌
```

**Required flow (Option A):**
```
PUB → LibreOffice → DOCX → Rust DocxBackend → DocItems  ✅
```

**Required flow (Option B):**
```
PUB → Parse binary format → DocItems (Rust/C++)  ✅
```

**Fix:** Convert to DOCX (not PDF!), then parse with Rust

**Assigned to:** Worker
**Deadline:** After Visio
**Test files:** Need to verify/create
**Fixed at N:** _____

---

### #4: OneNote (ONE) ❌ BLOCKED

**Status:** USES PYTHON
**File:** crates/docling-microsoft-extended/src/onenote.rs
**Violation:** `python_bridge::convert_via_python()`

**Current flow:**
```
ONE → LibreOffice → PDF → Python → DocItems  ❌
```

**Required flow:**
```
ONE → LibreOffice → DOCX → Rust DocxBackend → DocItems  ✅
```

**Fix:** Convert to DOCX, parse with Rust (or defer if too complex)

**Assigned to:** Worker
**Deadline:** After Publisher
**Test files:** 5 ready in test-corpus/microsoft-onenote/
**Fixed at N:** _____

---

### #5: Project (MPP) ❌ BLOCKED

**Status:** USES PYTHON
**File:** crates/docling-microsoft-extended/src/project.rs
**Violation:** `python_bridge::convert_via_python()`

**Current flow:**
```
MPP → LibreOffice → PDF → Python → DocItems  ❌
```

**Required flow:**
```
MPP → LibreOffice → DOCX → Rust DocxBackend → DocItems  ✅
```

**Fix:** Convert to DOCX, parse with Rust (or defer if too complex)

**Assigned to:** Worker
**Deadline:** After OneNote
**Test files:** 5 ready in test-corpus/microsoft-project/
**Fixed at N:** _____

---

### #6: Access (MDB) ❌ BLOCKED

**Status:** USES PYTHON
**File:** crates/docling-microsoft-extended/src/access.rs
**Violation:** `python_bridge::convert_via_python()`

**Current flow:**
```
MDB → mdb-tools → CSV → Python → DocItems  ❌
```

**Required flow:**
```
MDB → mdb-tools → CSV → Rust CsvBackend → DocItems  ✅
```

**Fix:** Parse mdb-tools output with Rust CsvBackend (or use C++ FFI)

**Assigned to:** Worker
**Deadline:** After Project
**Test files:** 5 ready in test-corpus/microsoft-access/
**Fixed at N:** _____

---

## ACCEPTANCE CRITERIA

**All formats fixed when:**
```bash
# Check for violations
grep -r "python_bridge" crates/docling-*/src/*.rs | grep -v "^crates/docling-core"

# Should return: 0 results
```

**Each backend must:**
- [ ] Generate DocItems in Rust or C++
- [ ] No python_bridge calls
- [ ] No conversion chains through Python
- [ ] Integration tests pass
- [ ] LLM validation added

---

## MANAGER CHECKPOINTS

**After each fix, verify:**
```bash
# 1. Python bridge call removed?
grep "python_bridge" crates/docling-{format}/src/

# 2. DocItems generated in Rust?
grep "content_blocks: Some" crates/docling-{format}/src/

# 3. Tests pass?
USE_RUST_BACKEND=1 cargo test test_{format}

# 4. Can mark complete?
# Only if all above pass!
```

---

## CURRENT STATUS

- [ ] LaTeX - BLOCKED (uses Python)
- [ ] Visio - BLOCKED (uses Python)
- [ ] Publisher - BLOCKED (uses Python)
- [ ] OneNote - BLOCKED (uses Python)
- [ ] Project - BLOCKED (uses Python)
- [ ] Access - BLOCKED (uses Python)

**0/6 fixed. All must be fixed.**

**Next AI: Start with LaTeX (pure Rust parser per user request).**

---

**This tracker stays updated until ALL Python dependencies eliminated.**
