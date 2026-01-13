# Five Problems Found and To Fix

## Problem 1: Integration Tests Still Reference Python Bridge
**Location:** crates/docling-core/tests/integration_tests.rs
**Issue:** 5 references to USE_HYBRID_SERIALIZER, python-bridge, python_bridge
**Impact:** Tests are broken/misleading
**Fix:** Remove or update integration tests to use pure Rust

## Problem 2: CLI Is Broken
**Location:** crates/docling-cli/src/main.rs
**Issue:** Compilation error - references removed modules (performance, converter)
**Impact:** CLI tool doesn't compile
**Fix:** Remove Python references, use RustDocumentConverter from docling-backend

## Problem 3: Blocking Directive Files Still Exist
**Location:** Root directory
**Files:** FIX_VERIFIED_BUGS_NOW.txt, API_KEY_EXISTS_NO_EXCUSES.txt, USER_DIRECTIVE_QUALITY_95_PERCENT.txt
**Issue:** Old directive files clutter repo, may confuse future developers
**Impact:** Confusing outdated directives
**Fix:** Archive or delete outdated directives

## Problem 4: PDF Output Quality Is Poor
**Location:** Pure Rust PDF test output
**Issue:** Garbled text ("PreDigtalEt", "WordPrcr" instead of "Pre-Digital Era", "Word Processor")
**Output:** 701 chars vs Python's 9,456 chars
**Impact:** Rust PDF quality significantly lower than Python baseline
**Fix:** Investigate ML pipeline text extraction/assembly

## Problem 5: Clippy Warnings in PDF ML
**Location:** crates/docling-pdf-ml/
**Issue:** Multiple deprecation warnings, unused imports
**Impact:** Code quality, maintenance burden
**Fix:** Run cargo fix, remove unused imports

---

**Execution Order:**
1. Problem 3 (easy - delete files)
2. Problem 5 (easy - cargo fix)
3. Problem 1 (medium - update tests)
4. Problem 2 (medium - fix CLI)
5. Problem 4 (hard - investigate PDF quality)
