# ZIP Archive Test Corpus

This directory contains test files for ZIP archive support in docling_rs.

**Format:** ZIP Archive
**Extensions:** `.zip`
**Category:** Archive Formats (Phase A)
**Status:** Implemented (Phase A Step 1)

---

## Test Files Required (5 diverse examples)

### 1. simple_documents.zip - Basic Document Archive
**Purpose:** Test basic ZIP extraction with simple text files

**Contents:**
- README.txt (plain text)
- doc1.txt (plain text)
- doc2.txt (plain text)

**Size:** ~5 KB
**Creation:**
```bash
echo "This is the README" > README.txt
echo "Document 1 content" > doc1.txt
echo "Document 2 content" > doc2.txt
zip simple_documents.zip README.txt doc1.txt doc2.txt
rm README.txt doc1.txt doc2.txt
```

**Expected Output:**
- Should extract and process 3 text files
- Each file should appear as a separate section in markdown

---

### 2. mixed_formats.zip - Multiple Document Formats
**Purpose:** Test format detection and recursive processing

**Contents:**
- sample.md (markdown file)
- data.csv (CSV file)
- notes.txt (plain text)
- Optional: small PDF, DOCX if available

**Size:** ~100-500 KB
**Creation:**
```bash
# Create test files with diverse formats
echo "# Markdown Test\n\nThis is markdown." > sample.md
echo "Name,Age,City\nJohn,30,NYC\nJane,25,LA" > data.csv
echo "Meeting notes\n- Topic 1\n- Topic 2" > notes.txt
zip mixed_formats.zip sample.md data.csv notes.txt
rm sample.md data.csv notes.txt
```

**Expected Output:**
- Each file processed according to its format
- Markdown rendered correctly
- CSV data structured properly
- Unsupported formats listed but not processed

---

### 3. nested_archive.zip - ZIP within ZIP
**Purpose:** Test recursive archive handling

**Contents:**
- inner.zip (contains file1.txt, file2.txt)
- outer_file.txt (plain text)

**Size:** ~20 KB
**Creation:**
```bash
# Create inner archive
echo "Inner file 1" > file1.txt
echo "Inner file 2" > file2.txt
zip inner.zip file1.txt file2.txt
rm file1.txt file2.txt

# Create outer archive
echo "Outer file content" > outer_file.txt
zip nested_archive.zip inner.zip outer_file.txt
rm inner.zip outer_file.txt
```

**Expected Output:**
- Outer archive processed
- Inner archive detected and recursively processed
- All files from both archives extracted
- Proper nested structure in output

---

### 4. large_archive.zip - Many Small Files
**Purpose:** Test performance with large number of files

**Contents:**
- 50+ small text files (file_001.txt through file_050.txt)

**Size:** ~500 KB
**Creation:**
```bash
# Generate 50 files
for i in {1..50}; do
  printf "This is test file number %03d\n" $i > file_$(printf "%03d" $i).txt
done
zip large_archive.zip file_*.txt
rm file_*.txt
```

**Expected Output:**
- All 50 files extracted and processed
- Processing completes in reasonable time (< 5 seconds)
- Memory usage remains reasonable

---

### 5. directory_structure.zip - Preserves Hierarchy
**Purpose:** Test directory structure handling

**Contents:**
```
docs/
  reports/
    2024/
      q1.txt
      q2.txt
  presentations/
    slides.txt
README.txt
```

**Size:** ~50 KB
**Creation:**
```bash
# Create directory structure
mkdir -p docs/reports/2024
mkdir -p docs/presentations

echo "Q1 Report" > docs/reports/2024/q1.txt
echo "Q2 Report" > docs/reports/2024/q2.txt
echo "Presentation slides" > docs/presentations/slides.txt
echo "Main README" > README.txt

zip -r directory_structure.zip docs/ README.txt
rm -rf docs/ README.txt
```

**Expected Output:**
- All files extracted with paths preserved
- Files listed with full relative paths
- Directory structure reflected in output

---

## Edge Cases (Optional)

### 6. encrypted.zip - Password Protected (Error Testing)
**Purpose:** Test error handling for encrypted archives

**Creation:**
```bash
echo "Secret content" > secret.txt
zip -e encrypted.zip secret.txt  # Password: "test123"
rm secret.txt
```

**Expected Behavior:**
- Should return `ArchiveError::PasswordProtected`
- Graceful error message to user
- No crash or hang

---

### 7. empty.zip - Empty Archive
**Purpose:** Test handling of empty archives

**Creation:**
```bash
zip empty.zip -0  # Create empty ZIP
```

**Expected Behavior:**
- Should return empty result or "(Empty archive)" message
- No errors

---

### 8. corrupted.zip - Invalid Archive (Error Testing)
**Purpose:** Test handling of corrupted files

**Creation:**
```bash
# Create a fake ZIP file
echo "This is not a real ZIP file" > corrupted.zip
```

**Expected Behavior:**
- Should return `ArchiveError::InvalidZip`
- Graceful error message
- No crash

---

## Expected Output Format

For a simple ZIP with 2 text files:

```markdown
## File: file1.txt

[Contents of file1.txt]

---

## File: file2.txt

[Contents of file2.txt]
```

For nested archives:

```markdown
## Archive: inner.zip

### Nested Archive: inner.zip

#### File: file1.txt

[Contents]

---

#### File: file2.txt

[Contents]

---

## File: outer_file.txt

[Contents]
```

For unsupported formats:

```markdown
## File: unknown.xyz (unsupported format: .xyz)
```

---

## Validation Checklist

Before committing test files, verify:

- [ ] All 5 required test files created
- [ ] File sizes are reasonable (< 1 MB each)
- [ ] Test files contain diverse content
- [ ] ZIP files are valid (can open with system tools)
- [ ] No sensitive or copyrighted content included
- [ ] Edge cases documented (encrypted, empty, corrupted)
- [ ] Expected outputs documented

---

## Known Limitations

### 1. Password-Protected Archives
- Encrypted ZIP files return `PasswordProtected` error
- No password prompt or decryption support
- **Workaround:** User must decrypt manually before processing

### 2. Very Large Archives
- Archives > 1 GB may hit size limits
- Individual files > 100 MB are skipped with warning
- **Workaround:** Extract large archives manually

### 3. Symbolic Links
- Symlinks in archives are skipped
- Not followed or resolved
- **Workaround:** Resolve symlinks before archiving

### 4. Special Characters in Filenames
- Some Unicode characters may not display correctly
- Paths with special chars might cause issues on some systems
- **Workaround:** Sanitize filenames if needed

### 5. Maximum Nesting Depth
- Archives nested > 10 levels deep return error
- Prevents infinite recursion
- **Workaround:** Flatten archive structure

---

## Testing Commands

### Run ZIP Tests
```bash
# Unit tests
cargo test -p docling-archive

# Integration tests (when test files exist)
cargo test test_archive_zip

# Specific test
cargo test test_archive_zip_nested -- --exact
```

### Manual Testing
```bash
# Test with Rust backend
USE_RUST_BACKEND=1 cargo run -- convert test-corpus/archives/zip/simple_documents.zip

# Test with hybrid serializer
USE_HYBRID_SERIALIZER=1 cargo run -- convert test-corpus/archives/zip/mixed_formats.zip
```

---

## Future Enhancements

1. **Password Support:** Accept optional password parameter
2. **Streaming:** True streaming for very large archives
3. **Progress Reporting:** Show extraction progress for large archives
4. **Selective Extraction:** Allow filtering which files to process
5. **Archive Creation:** Support creating ZIP archives
6. **Other Formats:** Extend to TAR, 7Z, RAR (Phase A Steps 2-4)

---

## References

- ZIP Format Specification: https://pkware.cachefly.net/webdocs/casestudies/APPNOTE.TXT
- `zip` crate documentation: https://docs.rs/zip/
- Research document: `reports/feature-phase-a-archives/zip_research_2025-11-07.md`

---

**Last Updated:** 2025-11-07
**Phase:** A (Archive Formats)
**Step:** 1 (ZIP Support)
**Status:** Implemented, awaiting test files
