# Upload Test Files to GitHub Release - Quick Steps

**Archive Ready:** `test-corpus-v1.tar.gz` (105MB) in repo root

---

## Steps (2 minutes):

1. **Open GitHub Releases:**
   - Go to: https://github.com/ayates_dbx/docling_rs/releases/new
   - (Or: Your repo → Releases tab → Draft a new release)

2. **Fill in Release Form:**
   - **Tag:** `test-corpus-v1.0`
   - **Title:** `Test Corpus v1.0`
   - **Description:**
     ```
     Complete test corpus for docling_rs parser

     - 241MB (2,509 test files)
     - 39 document formats
     - Use with: cargo test

     Download and extract:
     curl -L -O https://github.com/ayates_dbx/docling_rs/releases/download/test-corpus-v1.0/test-corpus-v1.tar.gz
     tar -xzf test-corpus-v1.tar.gz
     ```

3. **Attach File:**
   - Drag `test-corpus-v1.tar.gz` to "Attach binaries" area
   - Or click and select: `~/docling_rs/test-corpus-v1.tar.gz`

4. **Publish:**
   - Click "Publish release"

---

## Download URL (After Publishing)

**Other system uses:**
```
https://github.com/ayates_dbx/docling_rs/releases/download/test-corpus-v1.0/test-corpus-v1.tar.gz
```

That's it! Files stored separately, downloadable by anyone, doesn't affect repo size.
