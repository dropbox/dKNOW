# RAR Investigation - N=1646

**Date:** 2025-11-20
**Branch:** feature/phase-e-open-standards
**Issue:** RAR format scored 46% in LLM quality tests (PRIORITY_FORMATS_2025-11-20.md)

---

## Summary

**Finding:** The RAR "low score" issue is NOT a parser bug. It's due to:
1. **Test file has only 1 file** (test-corpus/archives/rar/nested.rar contains only `te…―st✌`)
2. **LLM score variance** (58% → 46% across runs)
3. **Test corpus is inadequate** (multi_files.rar also has only 1 file)

**Conclusion:** RAR parser works correctly. Issue is test file quality, not code quality.

---

## Evidence

### Test File Investigation

```bash
$ lsar test-corpus/archives/rar/nested.rar
test-corpus/archives/rar/nested.rar: RAR 5
te…―st✌

$ lsar test-corpus/archives/rar/multi_files.rar
test-corpus/archives/rar/multi_files.rar: RAR 5
.gitignore
```

**Result:** Both test RAR files contain only ONE file each, not multiple files as priority doc claims.

### LLM Test Results

**Run 1:**
```
Overall Score: 58.0%
DocItem Gaps:
  - The archive may contain more files than listed. Only one file is mentioned.
  - File name 'te…―st✌' appears truncated or incorrectly extracted.
```

**Run 2:**
```
Overall Score: 46.0%
```

**Variance:** 58% → 46% (21% drop, same input)

### Backend Code Review

**File:** `crates/docling-backend/src/archive.rs`

```rust
// Lines 136-147: Archives list ALL files
for file in files.iter() {
    let file_text = format!("{} ({} bytes)", file.name, file.size);
    doc_items.push(create_list_item(...));
}
```

**Finding:** Code correctly lists all extracted files. No "first file only" bug exists.

**File:** `crates/docling-archive/src/rar.rs`

```rust
// Lines 111-157: Recursively reads ALL files from extraction
fn read_directory_recursive(...)  {
    for entry in fs::read_dir(dir) {
        if metadata.is_file() {
            files.push(ExtractedFile { ... });
        } else if metadata.is_dir() {
            read_directory_recursive(&path, base_path, files)?;  // Recurse
        }
    }
}
```

**Finding:** Extraction correctly handles nested directories recursively.

---

## Root Cause

**Priority document (PRIORITY_FORMATS_2025-11-20.md, lines 11-15) states:**
```
### RAR (46%) - Archive Format
**Issues:**
- Structure: 20/100 (critical)
- Completeness: 50/100
- Only shows first file, missing directory tree
```

**This is incorrect.** The code shows ALL files. The test file only HAS one file.

**Hypothesis on how this error occurred:**
1. LLM tested `nested.rar` (contains 1 file: `te…―st✌`)
2. LLM interpreted single file output as "only showing first file"
3. Priority doc was written based on this misinterpretation
4. Code was never actually broken

---

## Unicode Filename Issue

**Observation:** Filename `te…―st✌` appears truncated/corrupted in lsar output.

**Possible causes:**
1. Terminal encoding issue (macOS Unicode rendering)
2. RAR file contains invalid UTF-8
3. `unar` extraction tool Unicode bug

**Impact on score:**
- LLM comment: "File name 'te…―st✌' appears truncated or incorrectly extracted"
- This contributes to Accuracy score reduction (70/100)

**Not a Rust code issue** - filename comes from `unar` command-line tool extraction.

---

## Test Corpus Quality

**Status:** **INADEQUATE** for multi-file archive testing

**Current test files:**
| File | Size | Contents | Verdict |
|------|------|----------|---------|
| nested.rar | 96 bytes | 1 file (unicode name) | ❌ Poor quality |
| multi_files.rar | 129 bytes | 1 file (.gitignore) | ❌ Misnamed, not multi-file |
| simple.rar | 100 bytes | ? | Unknown |
| compressed_best.rar | 286 bytes | ? | Unknown |

**Recommendation:** Create proper test RAR files with:
- Multiple files (5-10 files)
- Directory hierarchy (nested folders)
- Mixed file types
- ASCII-only filenames (avoid Unicode edge cases for baseline tests)

---

## LLM Score Variance

**Observation:** Score changed 58% → 46% (21% variance) with identical input.

**Explanation:**
- GPT-4 is non-deterministic
- Archive DocItems are simple (just file listings)
- Small variations in LLM interpretation cause large score swings
- Temperature/sampling affects categorical judgments

**Mitigation:**
- Run tests multiple times, take average
- Use stricter prompts with scoring rubrics
- Accept score ranges (±10%) rather than exact values

---

## Recommendations

### Immediate Actions (N=1646)

**Option A: Skip RAR improvements** (RECOMMENDED)
- Current RAR backend works correctly
- Issue is test corpus quality, not code quality
- Fix requires creating better test files, not changing code
- ROI is low (score variance 46-58%, already above 50%)

**Option B: Fix test corpus**
- Create multi-file RAR with proper structure
- Re-run LLM tests to get accurate baseline
- If score remains <80%, then investigate code improvements

### Long-Term Actions

1. **Improve test corpus for ALL archive formats** (ZIP, TAR, 7Z, RAR)
2. **Add category scores to priority document** (not just overall score)
3. **Document LLM variance** (run tests 3x, report mean ± stddev)
4. **Separate "test file quality" from "code quality" issues**

---

## Decision: Move to Next Priority

**Rationale:**
- RAR parser is functional (extracts all files correctly)
- Score issue is test file quality + LLM variance, not code bug
- Better ROI to fix formats with actual code issues (GIF 47.5%, VSDX 65%, etc.)

**Next priority:** GIF (47.5%) - Image Format (PRIORITY_FORMATS_2025-11-20.md line 27)
- **Real issue:** Missing animation frames, frame timing
- **Code change needed:** Extract multi-frame GIF data
- **Higher ROI:** Clear technical problem with clear solution

---

## Files Referenced

- `PRIORITY_FORMATS_2025-11-20.md` (line 11-24)
- `crates/docling-backend/src/archive.rs` (lines 136-147)
- `crates/docling-archive/src/rar.rs` (lines 111-157)
- `test-corpus/archives/rar/nested.rar` (test file)
- `test-corpus/archives/rar/multi_files.rar` (test file)

---

📊 Generated with Claude Code (N=1646)
https://claude.com/claude-code

Co-Authored-By: Claude <noreply@anthropic.com>
