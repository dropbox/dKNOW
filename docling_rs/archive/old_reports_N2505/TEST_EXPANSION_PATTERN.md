# Test Expansion Pattern

**Status:** PROVEN - 5/5 backends successfully expanded (100% success rate)
**Date Created:** 2025-11-12 (N=343)
**Commits:** N=337-339, N=341-342

---

## Overview

This document describes a proven pattern for expanding unit test coverage of backend parsers in the docling_rs project. The pattern was developed and validated across 5 backends (XLSX, WebVTT, SRT, XPS, IDML) in commits N=337-342.

### Success Metrics

- **Backends Expanded:** 5 (XLSX, WebVTT, SRT, XPS, IDML)
- **Tests Added:** +63 total (+12.6 tests/backend average)
- **Growth Rate:** +637% average per backend
- **Pass Rate:** 100% (no test failures)
- **Regressions:** 0
- **Clippy Warnings:** 0
- **Performance Impact:** Negligible (<0.03s for all backend tests)

---

## When to Use This Pattern

### Good Candidates

Backends with minimal test coverage (1-3 tests) that only test:
- Basic backend creation
- Format identification
- Simple error cases

### Target Profile

- **Starting Coverage:** 1-3 tests
- **Target Coverage:** 12-15 tests
- **Expected Growth:** 10-13 new tests
- **Time Estimate:** 1-2 hours per backend

### Priority Backends

1. **Medium/High Priority Formats:** Complex formats used in production (XPS, IDML, XLSX, etc.)
2. **Well-Defined Formats:** Formats with clear specifications (SRT, WebVTT, XLSX)
3. **Pure Rust Implementations:** Backends without complex external dependencies

### Low Priority (Skip)

- Trait definitions (`traits.rs`) - Limited test value
- Integration wrappers (`converter.rs`) - Tested via integration tests
- Simple wrappers (`archive.rs`) - Minimal complexity
- Already comprehensive (≥10 tests)

---

## Pattern Structure

### Test Categories (4 categories, ~13 tests total)

#### 1. Metadata Tests (3 tests)

Test metadata extraction and formatting:

```rust
#[test]
fn test_<format>_metadata_complete() {
    // Full metadata: title, author, timestamps, etc.
    // Verify all fields present in output
}

#[test]
fn test_<format>_metadata_partial() {
    // Partial metadata: title only, author only, etc.
    // Verify partial frontmatter/headers correct
}

#[test]
fn test_<format>_metadata_empty() {
    // No metadata
    // Verify no frontmatter/headers generated
}
```

**Examples:**
- XLSX: ZIP archive metadata from `docProps/core.xml`
- WebVTT: No metadata (format doesn't support it)
- IDML: YAML frontmatter from `designmap.xml`
- XPS: Markdown-style metadata section

#### 2. DocItem Generation Tests (3 tests)

Test markdown generation from parsed structures:

```rust
#[test]
fn test_<format>_single_<unit>_markdown() {
    // Single document unit (page, story, sheet, etc.)
    // Verify correct markdown structure
}

#[test]
fn test_<format>_multi_<unit>_markdown() {
    // Multiple document units
    // Verify all units present, correct ordering
}

#[test]
fn test_<format>_empty_document() {
    // Empty or minimal document
    // Verify graceful handling (empty string or minimal output)
}
```

**Examples:**
- XLSX: Single sheet vs multi-sheet workbooks
- WebVTT: Single subtitle vs multiple subtitles
- IDML: Single story vs multi-story documents
- XPS: Single page vs multi-page documents

#### 3. Format-Specific Features (4-5 tests)

Test unique features of the format:

```rust
#[test]
fn test_<format>_<feature_1>() {
    // Format's primary unique feature
}

#[test]
fn test_<format>_<feature_2>() {
    // Secondary feature or variant
}

#[test]
fn test_<format>_<feature_3>() {
    // Edge case of primary feature
}

#[test]
fn test_<format>_<feature_4>() {
    // Additional format-specific behavior
}
```

**Examples:**
- XLSX: Merged cells, data bounds, datetime parsing, table markdown
- WebVTT: Speaker handling, timestamp format (HH:MM:SS.mmm with period)
- SRT: Timestamp format (HH:MM:SS,mmm with comma), header format, brackets
- XPS: Page dimensions, text positioning, font size extraction
- IDML: Heading styles (Heading1-6), story IDs, paragraph styles

#### 4. Edge Case Tests (3 tests)

Test error handling and boundary conditions:

```rust
#[test]
fn test_<format>_parse_bytes_behavior() {
    // parse_bytes with invalid data
    // Or verify temp file creation for ZIP formats
}

#[test]
fn test_<format>_whitespace_handling() {
    // Whitespace trimming, normalization
    // Format-specific whitespace rules
}

#[test]
fn test_<format>_error_case() {
    // Format-specific error condition
    // Empty content, malformed data, etc.
}
```

**Examples:**
- XLSX: parse_bytes temp file, empty cells, invalid datetime
- WebVTT: Empty text, empty parentheses (speaker without classes)
- SRT: Empty text, timestamp edge cases (0s, 99h)
- XPS: parse_bytes rejection (ZIP format), whitespace trimming
- IDML: parse_bytes not supported (ZIP package), empty story, trailing whitespace

---

## Implementation Steps

### Step 1: Read and Understand

1. Read the backend implementation file (`crates/docling-backend/src/<format>.rs`)
2. Identify the serializer/converter (often in a separate crate)
3. Read the format's data structures (types.rs, parser.rs, serializer.rs)
4. Understand the markdown generation logic

**Time:** 10-15 minutes

### Step 2: Plan Test Categories

1. Identify what metadata the format supports
2. Identify the document's structural units (pages, stories, sheets, etc.)
3. List format-specific features (3-5 unique aspects)
4. Identify edge cases and error conditions

**Time:** 5-10 minutes

### Step 3: Implement Tests

1. Add imports to test module (types, structures from format crate)
2. Implement metadata tests (3 tests)
3. Implement DocItem generation tests (3 tests)
4. Implement format-specific tests (4-5 tests)
5. Implement edge case tests (3 tests)

**Time:** 30-60 minutes

### Step 4: Verify and Commit

1. Run tests: `cargo test --package docling-backend --lib <format>`
2. Run all backend tests: `cargo test --package docling-backend --lib`
3. Run clippy: `cargo clippy --package docling-backend --lib -- -D warnings`
4. Verify no regressions, no warnings
5. Git commit with comprehensive message

**Time:** 10-15 minutes

---

## Code Template

### Test Module Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use docling_<format>::{FormatDocument, FormatMetadata, FormatStructure};

    #[test]
    fn test_<format>_backend_creation() {
        // Existing test (keep as-is)
    }

    // ========================================
    // Metadata Tests
    // ========================================

    #[test]
    fn test_<format>_metadata_complete() {
        // ...
    }

    #[test]
    fn test_<format>_metadata_partial() {
        // ...
    }

    #[test]
    fn test_<format>_metadata_empty() {
        // ...
    }

    // ========================================
    // DocItem Generation Tests
    // ========================================

    #[test]
    fn test_<format>_single_<unit>_markdown() {
        // ...
    }

    #[test]
    fn test_<format>_multi_<unit>_markdown() {
        // ...
    }

    #[test]
    fn test_<format>_empty_document() {
        // ...
    }

    // ========================================
    // Format-Specific Feature Tests
    // ========================================

    #[test]
    fn test_<format>_<feature_1>() {
        // ...
    }

    #[test]
    fn test_<format>_<feature_2>() {
        // ...
    }

    #[test]
    fn test_<format>_<feature_3>() {
        // ...
    }

    #[test]
    fn test_<format>_<feature_4>() {
        // ...
    }

    // ========================================
    // Edge Case Tests
    // ========================================

    #[test]
    fn test_<format>_parse_bytes_behavior() {
        // ...
    }

    #[test]
    fn test_<format>_whitespace_handling() {
        // ...
    }

    #[test]
    fn test_<format>_error_case() {
        // ...
    }

    // Note: <Format> is a complex format requiring <details>.
    // Full integration tests would require real <format> files.
    // Parser implementation in docling-<format> crate has its own tests.
}
```

---

## Real Examples

### Example 1: XLSX Backend (N=337)

**Starting:** 3 tests (creation, DocItem, format)
**Target:** 17 tests (+14 tests, +467%)
**Commit:** `# 337: Add XLSX Backend Unit Tests`

**Categories:**
- **Metadata (3):** Timestamps (ISO 8601), complete metadata (workbook properties), empty metadata
- **DocItem (3):** Single sheet, multi-sheet, empty workbook
- **Format-Specific (5):** Merged cells, data bounds, datetime parsing, table markdown, table boundaries
- **Edge Cases (3):** parse_bytes temp file, empty cells, invalid datetime

**Key Insights:**
- Calamine 0.31+ merged cell API
- Excel coordinate system (0-based vs 1-based)
- ZIP archive metadata from docProps/core.xml

### Example 2: WebVTT Backend (N=338)

**Starting:** 3 tests (creation, DocItem, format)
**Target:** 14 tests (+11 tests, +367%)
**Commit:** `# 338: Add WebVTT Backend Unit Tests`

**Categories:**
- **Metadata (1):** N/A (WebVTT has no metadata field, skipped this category)
- **DocItem (5):** Single subtitle, multiple subtitles, with speaker, empty file, format ID
- **Format-Specific (6):** Speaker with classes, speaker without classes, timestamp edge cases, timestamp hours, empty parentheses
- **Edge Cases (2):** Empty text, whitespace handling

**Key Insights:**
- WebVTT timestamp format: `HH:MM:SS.mmm` (period separator)
- Speaker format: `Name (class1, class2):  text` (2 spaces after colon)
- No empty parentheses when speaker has no classes

### Example 3: XPS Backend (N=341)

**Starting:** 1 test (creation only)
**Target:** 14 tests (+13 tests, +1300%)
**Commit:** `# 341: Add XPS Backend Unit Tests`

**Categories:**
- **Metadata (3):** Timestamps (ISO 8601), complete metadata, empty metadata
- **DocItem (3):** Single page (no page headers), multi-page (with page headers), empty document
- **Format-Specific (4):** Page dimensions (XPS units), text element positioning, font size, multiple text elements
- **Edge Cases (3):** parse_bytes rejection (ZIP), whitespace trimming, metadata without title

**Key Insights:**
- ZIP archive format (requires temp file for parse_bytes)
- XPS units: 1/96 inch (816x1056 = 8.5"x11")
- Single page: no "## Page N" headers, multi-page: has headers

### Example 4: IDML Backend (N=342)

**Starting:** 2 tests (creation, parse_bytes error)
**Target:** 15 tests (+13 tests, +650%)
**Commit:** `# 342: Add IDML Backend Unit Tests`

**Categories:**
- **Metadata (3):** Complete (YAML frontmatter), title only, empty
- **DocItem (3):** Single story, multi-story, empty document
- **Format-Specific (5):** Heading styles (1-3), all heading levels (1-6), story IDs (u1000), paragraph style variants
- **Edge Cases (3):** Trailing whitespace (trim_end), empty story, mixed content and metadata

**Key Insights:**
- ZIP package format (parse_bytes not supported)
- YAML frontmatter for metadata (not markdown-style)
- Story IDs follow InDesign convention: u1000, u2000, u3000
- trim_end() affects frontmatter-only documents

---

## Commit Message Template

```
# N: Add <Format> Backend Unit Tests
**Current Plan**: Continue Test Expansion Pattern (previous success, target <format> next)
**Checklist Status**: ✅ COMPLETE - <Format> backend expanded from X to Y tests (+Z tests, +P%)

## Changes

**Purpose:**
Test coverage expansion for <Format> backend, following successful N=337-342 pattern. Expanded from X tests to Y comprehensive tests covering metadata, DocItem generation, format-specific features, and edge cases.

**File Modified:**
- `crates/docling-backend/src/<format>.rs` - Added Z unit tests

**Test Categories Added:**

**Metadata Tests (3 tests):**
- `test_<format>_metadata_<test1>` - <description>
- `test_<format>_metadata_<test2>` - <description>
- `test_<format>_metadata_<test3>` - <description>

**DocItem Generation Tests (3 tests):**
- `test_<format>_<docitem_test1>` - <description>
- `test_<format>_<docitem_test2>` - <description>
- `test_<format>_<docitem_test3>` - <description>

**Format-Specific Features (4-5 tests):**
- `test_<format>_<feature_test1>` - <description>
- `test_<format>_<feature_test2>` - <description>
- ... (list all format-specific tests)

**Edge Cases (3 tests):**
- `test_<format>_<edge_test1>` - <description>
- `test_<format>_<edge_test2>` - <description>
- `test_<format>_<edge_test3>` - <description>

**Test Results:**
- **<Format> Backend:** Y/Y PASS (was X/X, +Z tests, +P%)
- **All Backend Tests:** T/T PASS (was T-Z/T-Z, +Z tests, +Q%)
- **Pass Rate:** 100% (no regressions)
- **Clippy Warnings:** 0
- **Runtime:** <Rs (backend tests remain extremely fast)

**Pattern Consistency:**
Followed N=337-342 pattern with same categories and structure.

## New Lessons

**<Format> Backend Characteristics:**
- <Key characteristic 1>
- <Key characteristic 2>
- <Key characteristic 3>

**<Specific insight or edge case discovered>:**
<Detailed explanation>

## Information Expiration

**Backend Test Count:**
- Previous (N=X): T-Z backend tests
- Current (N=Y): T backend tests (+Z, +Q%)
- **New Baseline:** T backend tests (100% pass)

**<Format> Backend Coverage:**
- Previous: X tests (minimal)
- Current: Y tests (comprehensive)
- Status: ✅ IMPROVED (meets ≥10 test minimum)

## Next AI: <Next recommended action> (N=Z)

<Detailed recommendations>

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>
```

---

## Quality Checklist

Before committing, verify:

- [ ] All tests pass: `cargo test --package docling-backend --lib <format>`
- [ ] No regressions: `cargo test --package docling-backend --lib`
- [ ] No clippy warnings: `cargo clippy --package docling-backend --lib -- -D warnings`
- [ ] Test count increased by 10-13 tests
- [ ] 100% pass rate maintained
- [ ] Test categories match pattern (metadata, DocItem, format-specific, edge cases)
- [ ] Test names are descriptive and follow convention
- [ ] Comments explain format-specific behavior
- [ ] Commit message follows template

---

## Anti-Patterns (Things to Avoid)

### ❌ Don't: Skip Test Categories

Even if a format has no metadata, document this:
```rust
// Note: WebVTT format has no metadata field, so metadata tests are not applicable.
```

### ❌ Don't: Test External Dependencies

Focus on backend logic, not external parser implementation:
```rust
// ❌ BAD: Testing external parser
#[test]
fn test_xlsx_calamine_parsing() {
    // This tests calamine library, not our backend
}

// ✅ GOOD: Testing our backend's use of parsed data
#[test]
fn test_xlsx_merged_cells() {
    // This tests our backend's merged cell handling
}
```

### ❌ Don't: Add Integration Tests

These are unit tests. Don't add tests that:
- Require real files on disk
- Make network requests
- Depend on environment variables
- Take >1 second to run

### ❌ Don't: Modify Existing Tests

Only add new tests. Don't modify working tests unless fixing bugs.

### ❌ Don't: Add Tests for Out-of-Scope Formats

Per CLAUDE.md:
- PDF parsing (separate initiative)
- Audio/video (separate system)
- Databases (separate tools)

---

## Future Work

### Remaining Low-Coverage Backends

**After N=342, low-priority backends remain:**
- `traits.rs`: 1 test (trait definitions, skip)
- `converter.rs`: 2 tests (integration layer, skip)
- `archive.rs`: 3 tests (simple wrapper, skip)
- `csv.rs`: 4 tests (acceptable, skip)

**Recommendation:** Skip these. Diminishing returns on test expansion. Focus on features, documentation, or user requests instead.

### Pattern Evolution

If expanding pattern in future:
1. Add new test categories as needed
2. Document new format-specific patterns
3. Update this file with new examples
4. Maintain 100% success rate

---

## References

### Commits Implementing This Pattern

- N=337: XLSX backend (3 → 17 tests, +467%)
- N=338: WebVTT backend (3 → 14 tests, +367%)
- N=339: SRT backend (3 → 15 tests, +400%)
- N=341: XPS backend (1 → 14 tests, +1300%)
- N=342: IDML backend (2 → 15 tests, +650%)

### Related Documents

- `CLAUDE.md` - Project instructions and protocols
- `TESTING_STRATEGY.md` - Overall testing approach
- `FORMAT_PROCESSING_GRID.md` - Format coverage matrix
- `reports/feature/phase-e-open-standards/benchmark_n340_2025-11-12.md` - Benchmark report

### Success Metrics

- **5/5 backends** successfully expanded (100% success rate)
- **+63 tests** added across 5 backends
- **+12.6 tests/backend** average growth
- **0 regressions** introduced
- **0 clippy warnings** generated
- **100% pass rate** maintained throughout

---

**Last Updated:** 2025-11-12 (N=343)
**Pattern Status:** PROVEN - Ready for reuse
**Maintainer:** AI Assistant (docling_rs project)
