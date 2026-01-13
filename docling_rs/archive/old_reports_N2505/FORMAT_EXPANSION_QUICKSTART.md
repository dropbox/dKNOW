# Format Expansion Quick Start Guide

**For AI Workers: Start Here**

---

## Overview

This project is expanding document format support from **15 formats to 67 formats** (+52 new formats).

**Full Plan:** See `FORMAT_EXPANSION_COMPREHENSIVE.md` (1,851 lines, 102 sections)

**This Document:** Quick-start guide for AI workers to begin implementation.

---

## Current Status

**Formats Supported:** 15
- Office: PDF, DOCX, PPTX, XLSX
- Web: HTML, CSV, Markdown, AsciiDoc
- Specialized: JATS, WebVTT
- Images: PNG, JPEG, TIFF, WebP, BMP

**Implementation:** `crates/docling-core/src/format.rs`

---

## New Formats to Add (52 total)

### By Category

1. **Audio** (2): WAV, MP3
2. **E-books** (4): EPUB, MOBI, AZW3, FB2
3. **Email** (5): EML, MSG, MBOX, PST, VCF
4. **Apple iWork** (3): Pages, Numbers, Keynote
5. **Adobe Extended** (5): IDML, AI, PSD, XFA, INDD
6. **Microsoft Extended** (6): PUB, VSDX, ONE, MPP, MDB/ACCDB, XPS
7. **CAD/Engineering** (4): DWG, DXF, STL, IFC
8. **3D Formats** (3): OBJ, FBX, GLTF/GLB
9. **Archives** (4): ZIP, RAR, 7Z, TAR/TGZ
10. **Video** (5): MP4, MKV, AVI, MOV, SRT
11. **Specialized** (7): DICOM, KML/KMZ, GPX, ICS, VCF (genomic), LaTeX, Jupyter
12. **Legacy** (4): RTF, WordPerfect, DOC, WPS

### By Priority

**Tier 1 - High Priority (16 formats):**
- Archives: ZIP, TAR, 7Z, RAR
- Audio: WAV, MP3
- Video subtitles: SRT, MP4, MKV, MOV, AVI
- Email: EML, MBOX, VCF
- Images: GIF
- OpenDocument: ODS

**Tier 2 - Medium Priority (20 formats):**
- E-books: EPUB, FB2, MOBI
- Email: MSG, PST
- OpenDocument: ODT, ODP
- Microsoft: XPS, VSDX
- Graphics: SVG, HEIF, AVIF
- Calendar: ICS
- Notebooks: Jupyter

**Tier 3-5 - Lower Priority (16 formats):**
- Apple: Pages, Numbers, Keynote
- Adobe: IDML, AI, PSD, XFA, INDD
- Microsoft: PUB, ONE, MPP, MDB
- CAD: DWG, DXF, IFC
- 3D: STL, OBJ, FBX, GLTF
- Legacy: RTF, DOC, WordPerfect, WPS
- Specialized: DICOM, KML, GPX, LaTeX, VCF (genomic)

---

## AI Execution Instructions

### IMMEDIATE TASK: Start Phase A

#### Step 1: Create Branch
```bash
git checkout -b feature/phase-a-foundation
```

#### Step 2: Implement ZIP Archive Support

**Why ZIP first?**
- Enables recursive document extraction
- Many formats are ZIP-based (EPUB, DOCX, ODT, etc.)
- High utility, low complexity

**Implementation Checklist:**

1. **Research** (5 min)
   - [ ] Verify `zip` crate v0.6+ in Cargo.toml
   - [ ] Read zip crate documentation
   - [ ] Understand recursive extraction pattern

2. **Test Corpus** (20 min)
   - [ ] Create `test-corpus/archives/zip/`
   - [ ] Collect/create 5 diverse ZIP files:
     - [ ] Document ZIP (multiple PDFs/DOCs)
     - [ ] Source code ZIP (mixed text files)
     - [ ] Image archive (multiple images)
     - [ ] Mixed content (various file types)
     - [ ] Nested ZIP (ZIP containing ZIPs)
   - [ ] Document sources in `test-corpus/archives/README.md`

3. **Core Implementation** (2-3 hours)
   - [ ] Update `crates/docling-core/src/format.rs`:
     - [ ] Add `Archive` or keep individual extensions
     - [ ] Update `from_extension()` for `.zip`
   - [ ] Create `crates/docling-archive/` crate (or add to existing)
   - [ ] Implement ZIP extraction:
     ```rust
     pub fn extract_and_parse_zip(path: &Path) -> Result<Vec<Document>> {
         // 1. Open ZIP archive
         // 2. Iterate through entries
         // 3. For each file, detect format
         // 4. Recursively parse based on extension
         // 5. Collect all documents
         // 6. Return aggregated result
     }
     ```
   - [ ] Handle errors gracefully (corrupted ZIP, etc.)
   - [ ] Add logging/progress tracking

4. **Integration** (1 hour)
   - [ ] Update `DocumentConverter` to handle archives
   - [ ] Route `.zip` files to archive extractor
   - [ ] Decide on output format:
     - Option A: Multiple documents (one per file)
     - Option B: Single document with sections
     - Option C: Structured output with file tree

5. **Testing** (1 hour)
   - [ ] Create `crates/docling-core/tests/test_archives.rs`
   - [ ] Add integration tests for each ZIP file
   - [ ] Run: `cargo test test_archive_zip`
   - [ ] Verify all 5 files pass
   - [ ] Check edge cases:
     - [ ] Empty ZIP
     - [ ] Corrupted ZIP
     - [ ] Very large ZIP
     - [ ] ZIP with unsupported files

6. **Documentation** (30 min)
   - [ ] Add rustdoc comments to all functions
   - [ ] Create `docs/formats/ZIP.md` with:
     - Format description
     - Implementation notes
     - Limitations
     - Examples
   - [ ] Update `MASTER_PLAN.md`:
     - Mark ZIP as ✅ Implemented
     - Note completion in Phase A

7. **Git Commit** (10 min)
   - [ ] Stage all changes
   - [ ] Commit with message (following CLAUDE.md format):
   ```
   Add ZIP archive support with recursive document extraction

   ## Changes
   - Added ZIP format to InputFormat enum
   - Implemented recursive extraction in docling-archive crate
   - Created 5 diverse test files with various archive contents
   - Added integration tests for ZIP parsing
   - Documented ZIP format support and limitations

   ## Test Results
   - All 5 ZIP test files pass
   - Edge cases handled: empty, corrupted, nested archives
   - Performance: <100ms for typical archives

   ## Next AI: Continue Phase A with TAR/TGZ Support
   - Implement TAR extraction similar to ZIP
   - Handle .tar, .tar.gz, .tar.bz2, .tgz variants
   - Reuse recursive parsing logic from ZIP implementation
   ```

#### Step 3: Continue Phase A

After ZIP is complete, implement in order:
1. TAR/TAR.GZ archives
2. 7Z archives
3. RAR archives
4. SRT subtitles
5. GIF images

---

## Implementation Patterns

### Pattern 1: Archive Formats (ZIP, TAR, 7Z, RAR)

```rust
// General structure for all archives

pub fn parse_archive(path: &Path) -> Result<Vec<Document>> {
    let entries = extract_archive(path)?;

    let mut documents = Vec::new();
    for entry in entries {
        match detect_format(&entry) {
            Some(format) => {
                if let Ok(doc) = parse_by_format(&entry, format) {
                    documents.push(doc);
                }
            }
            None => continue, // Skip unsupported files
        }
    }

    Ok(documents)
}
```

### Pattern 2: Text-based Formats (SRT, CSV, RTF)

```rust
pub fn parse_text_format(content: &str) -> Result<Document> {
    // Parse line by line or use nom parser
    // Extract structure
    // Build Document
    Ok(document)
}
```

### Pattern 3: ZIP-based Formats (EPUB, DOCX, ODT)

```rust
pub fn parse_zip_format(path: &Path) -> Result<Document> {
    let archive = zip::ZipArchive::new(File::open(path)?)?;

    // Extract key files (content.xml, etc.)
    let content_xml = extract_file(&archive, "content.xml")?;

    // Parse XML
    let doc = parse_xml(&content_xml)?;

    Ok(doc)
}
```

---

## Key Decisions for AI Workers

### Decision 1: Archive Output Format

**Question:** How should ZIP/archive contents be returned?

**Options:**
- A) **Multiple Documents** - Return `Vec<Document>`, one per file
- B) **Single Document** - Aggregate all content into one document with sections
- C) **Structured Output** - Return document with file tree structure

**Recommendation:** Start with Option A (multiple documents), add Option B later if needed.

### Decision 2: External Dependencies

**Question:** When should we use external tools vs pure Rust?

**Guidelines:**
- **Prefer Rust crates** when:
  - Mature, well-maintained crate exists
  - Format is well-documented
  - Pure Rust avoids deployment issues
- **Use external tools** when:
  - No suitable Rust crate exists
  - Format is extremely complex/proprietary
  - Conversion is the standard approach (e.g., PUB → PDF via LibreOffice)
- **Make external tools optional** via Cargo feature flags

### Decision 3: Error Handling

**Question:** How to handle parsing errors?

**Approach:**
```rust
// Return Result, never panic
pub fn parse(path: &Path) -> Result<Document, ParseError> {
    // ...
}

// Provide partial results when possible
pub fn parse_best_effort(path: &Path) -> (Option<Document>, Vec<Warning>) {
    // ...
}

// Log warnings, don't fail
warn!("Unsupported feature: {}", feature_name);
```

### Decision 4: Performance vs Correctness

**Priority:** Correctness first, then performance

**Approach:**
1. Implement basic parsing (focus on correctness)
2. Add comprehensive tests
3. Profile performance
4. Optimize hot paths if needed

**Don't:**
- Skip error handling for speed
- Use unsafe code without justification
- Compromise correctness for minor performance gains

---

## Testing Strategy

### Per Format

**Required Tests:**
1. Simple example (basic features)
2. Complex example (rich features)
3. Real-world example (actual file)
4. Edge case (large/unusual/corrupted)
5. Multi-language (if applicable)

**Test Template:**
```rust
#[test]
fn test_format_simple() {
    let path = test_corpus_path("format/simple.ext");
    let result = parse_format(&path);
    assert!(result.is_ok());
    let doc = result.unwrap();
    assert_eq!(doc.content.len(), expected_length);
}
```

### Integration Tests

**Location:** `crates/docling-core/tests/integration_tests.rs`

**Pattern:**
```rust
// Add to fixtures
const FORMAT_FILES: &[&str] = &[
    "simple.ext",
    "complex.ext",
    "realworld.ext",
    "edgecase.ext",
    "multilang.ext",
];

// Generate tests
for file in FORMAT_FILES {
    // Create test function
}
```

### Running Tests

```bash
# All tests
cargo test

# Specific format
cargo test test_format_name

# With output
cargo test -- --nocapture

# Single test
cargo test test_format_simple -- --exact
```

---

## Progress Tracking

### In Git Commits

Track progress in each commit message:
```
Formats completed: X / 52
Test files collected: Y / 260
Integration tests passing: Z / 260
Current phase: Phase A
Next format: Format Name
```

### In Files

**Update `MASTER_PLAN.md`:**
```markdown
### Format Status

| Format | Status | Test Files | Tests Passing | Notes |
|--------|--------|------------|---------------|-------|
| ZIP | ✅ Done | 5/5 | 5/5 | Recursive extraction working |
| TAR | 🚧 In Progress | 3/5 | 0/5 | Implementing extraction |
| 7Z | ❌ Not Started | 0/5 | 0/5 | Waiting for library research |
```

**Update `FORMAT_EXPANSION_COMPREHENSIVE.md`:**
- Check off completed items in AI Execution Checklist
- Add implementation notes
- Document any blockers or issues

---

## Common Issues and Solutions

### Issue 1: Cannot Find Test Files

**Problem:** Cannot locate 5 diverse examples for format

**Solutions:**
1. Search GitHub for sample files: `filename:*.ext`
2. Create simple examples programmatically
3. Convert from related formats
4. Ask user for test file sources
5. Use format-specific sample repositories

### Issue 2: No Suitable Rust Crate

**Problem:** Format has no mature Rust library

**Solutions:**
1. Check if format is text-based → write custom parser
2. Check if format is XML/ZIP-based → use quick-xml/zip
3. Use C library via FFI (e.g., libheif, libavif)
4. Shell out to external tool (e.g., LibreOffice, ffmpeg)
5. Require conversion to supported format
6. Document as "blocked" and move to next format

### Issue 3: Complex Format Structure

**Problem:** Format is too complex to fully parse

**Solutions:**
1. Start with text extraction only (ignore formatting)
2. Implement core features, document limitations
3. Use existing converter/viewer as reference
4. Parse metadata only (better than nothing)
5. Provide "best effort" parsing with warnings

### Issue 4: Performance Issues

**Problem:** Parsing is too slow

**Solutions:**
1. Profile with `cargo flamegraph` or `perf`
2. Use streaming parsers for large files
3. Parallelize with rayon for multiple files
4. Cache parsed results
5. Lazy-load large embedded resources

### Issue 5: Integration Test Failures

**Problem:** Tests failing unexpectedly

**Solutions:**
1. Compare output to expected output character-by-character
2. Check for whitespace differences
3. Verify file paths are correct
4. Ensure test files are valid
5. Check for platform-specific issues (Windows vs Unix line endings)

---

## When to Ask for Help

**Ask the user when:**
1. Cannot find suitable Rust library after research
2. Format requires licensed/proprietary software
3. Test files are unavailable or restricted
4. Unclear about user requirements or priorities
5. Blocked by external dependencies
6. Need clarification on implementation approach

**Document in reports:**
- What was tried
- What blockers exist
- What options are available
- What recommendation you have

---

## Next Steps Summary

### Immediate (This Session)
1. ✅ Review this quick-start guide
2. ✅ Review comprehensive plan (`FORMAT_EXPANSION_COMPREHENSIVE.md`)
3. → **START:** Create branch `feature/phase-a-foundation`
4. → **IMPLEMENT:** ZIP archive support (first task)

### Phase A (Foundation)
- Complete: ZIP, TAR, 7Z, RAR, SRT, GIF
- Duration: 6 formats
- Branch: `feature/phase-a-foundation`

### Phase B (Audio/Video)
- Complete: WAV, MP3, MP4, MKV, AVI, MOV
- Duration: 6 formats
- Branch: `feature/phase-b-audio-video`

### Phase C-L (Continue)
- Follow comprehensive plan
- One phase per branch
- Regular commits with progress updates

---

## Quick Reference

**Main Documents:**
- `FORMAT_EXPANSION_COMPREHENSIVE.md` - Full plan (1,851 lines)
- `FORMAT_EXPANSION_QUICKSTART.md` - This guide
- `MASTER_PLAN.md` - Project master plan
- `CLAUDE.md` - Project instructions and commit format

**Key Files:**
- `crates/docling-core/src/format.rs` - InputFormat enum
- `crates/docling-core/tests/integration_tests.rs` - Integration tests
- `test-corpus/` - Test files

**Commands:**
```bash
# Create branch
git checkout -b feature/phase-a-foundation

# Run tests
cargo test
cargo test test_format_name

# Build
cargo build --release

# Check code
cargo clippy
cargo fmt --check
```

**Commit Format:**
Follow `CLAUDE.md` template with:
- Brief title
- Current plan
- Changes description
- Next AI directive

---

**AI Worker: Begin with ZIP archive implementation. Good luck!**
