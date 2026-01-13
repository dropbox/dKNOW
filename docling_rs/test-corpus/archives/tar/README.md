# TAR Archive Test Corpus

**Category:** Archives
**Format:** TAR (Tape Archive)
**Extensions:** `.tar`, `.tar.gz`, `.tgz`, `.tar.bz2`, `.tbz2`
**Status:** Test files to be created by user
**Date:** 2025-11-07

---

## Overview

This directory contains TAR archive test files for validating docling's TAR extraction and processing capabilities. TAR is the standard archiving format for Unix/Linux systems, commonly used for software distribution and backups.

## Compression Support

Docling supports the following TAR compression formats:
- **Uncompressed** (`.tar`) - No compression
- **Gzip** (`.tar.gz`, `.tgz`) - DEFLATE compression (most common)
- **Bzip2** (`.tar.bz2`, `.tbz2`) - Better compression, slower

## Test File Requirements

Create **5 diverse TAR archive files** representing common use cases:

### 1. `simple_documents.tar` - Basic Document Archive
**Type:** Uncompressed TAR
**Contents:** 3-5 simple text files
**Size:** ~5-10 KB
**Purpose:** Validate basic TAR extraction

**Create with:**
```bash
cd test-corpus/archives/tar/samples
echo "Document 1 content" > doc1.txt
echo "Document 2 content" > doc2.md
echo "Document 3 content" > doc3.csv
tar -cf ../simple_documents.tar doc1.txt doc2.md doc3.csv
```

**Expected Output:**
- Extract 3 files
- Process each file based on format
- Generate aggregated markdown with sections for each file

---

### 2. `mixed_formats.tar.gz` - Multiple Format Archive
**Type:** Gzip compressed TAR
**Contents:** Mixed document types (MD, CSV, TXT, HTML)
**Size:** ~100-500 KB
**Purpose:** Test multi-format processing with compression

**Create with:**
```bash
cd test-corpus/archives/tar/samples
# Create diverse files
echo "# Readme\n\nSample project" > README.md
echo "Name,Age,City\nAlice,30,NYC" > data.csv
echo "Plain text file" > notes.txt
echo "<html><body><h1>Test</h1></body></html>" > index.html

tar -czf ../mixed_formats.tar.gz README.md data.csv notes.txt index.html
```

**Expected Output:**
- Extract 4 files
- Decompress gzip automatically
- Process each format correctly
- Markdown with sections for each file

---

### 3. `nested_archive.tar.gz` - TAR within TAR
**Type:** Gzip compressed TAR containing another TAR
**Contents:** Outer TAR contains inner.tar with documents
**Size:** ~20 KB
**Purpose:** Test recursive archive processing

**Create with:**
```bash
cd test-corpus/archives/tar/samples
# Create inner archive
echo "Inner file 1" > inner1.txt
echo "Inner file 2" > inner2.txt
tar -cf inner.tar inner1.txt inner2.txt

# Create outer archive containing inner.tar
echo "Outer file" > outer.txt
tar -czf ../nested_archive.tar.gz outer.txt inner.tar
```

**Expected Output:**
- Extract outer TAR (gzip)
- Detect inner.tar as archive
- Recursively process inner archive
- Hierarchical markdown with nested sections
- Depth tracking (headers adjust for nesting)

---

### 4. `large_archive.tar.bz2` - Performance Test
**Type:** Bzip2 compressed TAR
**Contents:** 50+ small files
**Size:** ~500 KB - 1 MB
**Purpose:** Test performance with many files

**Create with:**
```bash
cd test-corpus/archives/tar/samples
mkdir large_archive_files
for i in {1..50}; do
  echo "File $i content with some text to compress" > large_archive_files/file_$i.txt
done
tar -cjf ../large_archive.tar.bz2 large_archive_files/
```

**Expected Output:**
- Extract all 50+ files
- Decompress bzip2 automatically
- Process each file
- Complete within reasonable time (<10 seconds)
- Memory-efficient extraction

---

### 5. `directory_structure.tar` - Hierarchy Preservation
**Type:** Uncompressed TAR
**Contents:** Files in nested directory structure
**Size:** ~50 KB
**Purpose:** Verify directory structure handling

**Create with:**
```bash
cd test-corpus/archives/tar/samples
mkdir -p project/src project/docs project/tests
echo "Main code" > project/src/main.rs
echo "Utility code" > project/src/utils.rs
echo "# Documentation" > project/docs/README.md
echo "# Tests" > project/tests/test.rs

tar -cf ../directory_structure.tar project/
```

**Expected Output:**
- Extract files preserving paths
- Process files from all directories
- Markdown includes full paths (e.g., "project/src/main.rs")
- Directories themselves are skipped (not extracted)

---

## Edge Cases and Special Files

### Optional Test Cases

#### `empty.tar` - Empty Archive
```bash
tar -cf empty.tar -T /dev/null
```
**Expected:** "Empty archive" message, no errors

#### `corrupted.tar` - Corrupted Archive
```bash
dd if=/dev/urandom of=corrupted.tar bs=1024 count=10
```
**Expected:** Graceful error: "Failed to extract TAR: ..."

#### `mixed_compression.tar` - Wrong Extension
Save a gzip TAR as `.tar` (no compression indicator)
**Expected:** Auto-detection via magic bytes (future enhancement)

#### `tar_with_zip.tar.gz` - Mixed Archive Types
TAR containing both TAR and ZIP files
**Expected:** Recursive processing of both archive types

---

## Expected Output Format

For each TAR archive, docling should generate markdown with this structure:

```markdown
## File: filename1.txt

Content from filename1...

---

## File: filename2.md

Content from filename2...

---

## Archive: nested.tar

### Nested Archive: nested.tar

#### File: inner_file.txt

Content from inner file...
```

**Output Characteristics:**
- Top-level files: `## File: {name}`
- Nested archives: `### Nested Archive: {name}`
- Files in nested archives: `#### File: {name}`
- Unsupported files: `## File: {name} (unsupported format: .{ext})`
- Section separators: `---` between files

---

## Validation Checklist

After creating test files, verify the following:

### Format Recognition
- [x] `.tar` files recognized as TAR
- [x] `.tar.gz` files recognized as TAR (gzip)
- [x] `.tgz` files recognized as TAR (gzip)
- [x] `.tar.bz2` files recognized as TAR (bzip2)
- [x] `.tbz2` files recognized as TAR (bzip2)

### Extraction
- [ ] Uncompressed TAR extracts correctly
- [ ] Gzip TAR decompresses and extracts
- [ ] Bzip2 TAR decompresses and extracts
- [ ] Directories are skipped (not treated as files)
- [ ] File paths preserved correctly

### Content Processing
- [ ] Text files processed
- [ ] Markdown files processed
- [ ] CSV files processed
- [ ] HTML files processed
- [ ] Unsupported files listed (not crash)

### Recursive Processing
- [ ] Nested TAR detected and processed
- [ ] Nested ZIP detected and processed
- [ ] Depth tracking works (max 10 levels)
- [ ] Header hierarchy correct (##, ###, ####)

### Error Handling
- [ ] Empty archives handled gracefully
- [ ] Corrupted archives report error (no crash)
- [ ] Oversized files skipped with warning
- [ ] Missing files report error (no crash)

### Performance
- [ ] Large archives (50+ files) process quickly
- [ ] Memory usage reasonable (streaming extraction)
- [ ] No memory leaks on repeated extractions

---

## Testing Commands

### Extract Test Archive Manually
```bash
# Uncompressed
tar -xf simple_documents.tar

# Gzip compressed
tar -xzf mixed_formats.tar.gz

# Bzip2 compressed
tar -xjf large_archive.tar.bz2
```

### List Archive Contents
```bash
tar -tf simple_documents.tar
tar -tzf mixed_formats.tar.gz
tar -tjf large_archive.tar.bz2
```

### Validate TAR File
```bash
tar -tzf archive.tar.gz > /dev/null && echo "Valid TAR" || echo "Corrupted TAR"
```

---

## Implementation Notes

### TAR Format Details

**Header Structure (POSIX ustar):**
- 512 bytes per header
- Contains: filename, size, permissions, owner, timestamp
- Followed by file data (rounded to 512-byte blocks)
- Archive ends with two 512-byte zero blocks

**Compression Detection:**
1. By extension (primary method)
2. By magic bytes (future enhancement):
   - Gzip: `0x1f 0x8b`
   - Bzip2: `0x42 0x5a` ('BZ')

**Special File Types (skipped):**
- Directories (entry_type.is_file() == false)
- Symbolic links
- Hard links
- Device files

### Size Limits

Defined in `docling-archive/src/tar.rs`:
- `MAX_FILE_SIZE`: 100 MB per file (files larger are skipped)
- `MAX_ARCHIVE_SIZE`: 1 GB total
- `MAX_NESTING_DEPTH`: 10 levels (prevents infinite recursion)

Files exceeding limits are logged with `eprintln!` and skipped, not errors.

---

## Known Limitations

1. **Compression Support:** XZ and Zstandard not yet supported (future enhancement)
2. **Magic Byte Detection:** Not yet implemented (relies on extension)
3. **Sparse Files:** Treated as regular files (expanded fully)
4. **Special Files:** Symlinks, device files ignored
5. **PAX Attributes:** Parsed but not all attributes preserved
6. **Permissions:** Not preserved in output (metadata-only in TAR header)

---

## Future Enhancements

1. **Additional Compressions:**
   - Add `xz2` crate for `.tar.xz` support
   - Add `zstd` crate for `.tar.zst` support

2. **Smart Detection:**
   - Auto-detect compression from magic bytes
   - Handle mismatched extensions (.tar.gz saved as .tar)

3. **Selective Extraction:**
   - Extract only files matching pattern
   - Extract only specific file types

4. **Metadata Preservation:**
   - Include file permissions in output
   - Include timestamps and ownership

5. **Performance:**
   - Parallel extraction (multiple files at once)
   - In-memory processing for small archives

---

## Related Documentation

- **Phase A Checklist:** FORMAT_EXPANSION_COMPREHENSIVE.md (Phase A, Step 2)
- **TAR Research:** reports/feature-phase-a-archives/tar_research_2025-11-07.md
- **ZIP Test Corpus:** test-corpus/archives/zip/README.md (similar structure)
- **Rust tar Crate Docs:** https://docs.rs/tar/
- **Compression Crates:**
  - flate2: https://docs.rs/flate2/
  - bzip2: https://docs.rs/bzip2/

---

## Contact & Contribution

**Maintainer:** Andrew Yates <ayates@dropbox.com>
**Project:** docling_rs
**Version:** 2.58.0
**Date:** 2025-11-07

**To Create Test Files:**
1. Follow the "Create with:" commands above
2. Verify files are valid TAR archives
3. Test with docling: `docling-cli convert <tar_file>`
4. Validate output matches expected format

**Note:** Do NOT commit large binary TAR files to git. Commit only this README.
Create test files locally as needed using the commands provided.

---

**END OF TAR TEST CORPUS DOCUMENTATION**
