# GitHub Releases - Share Test Files Without Bloating Repo

**Perfect Solution:** GitHub Releases let you attach large files as downloadable assets.

## What GitHub Releases Are

- Separate download area (doesn't affect repo size)
- Can attach files up to 2GB each
- Permanent URLs for downloading
- Versioned (can update later)
- No Git LFS needed

**URL format:** `https://github.com/ayates_dbx/docling_rs/releases/download/v1.0/test-corpus.tar.gz`

---

## Option A: Using GitHub CLI (Fastest)

```bash
# 1. Create archive
cd ~/docling_rs
tar -czf test-corpus-v1.tar.gz test-corpus/

# 2. Create release with attached file (one command!)
gh release create test-corpus-v1.0 test-corpus-v1.tar.gz \
  --title "Test Corpus v1.0" \
  --notes "Docling test corpus: 241MB, 2,509 test files for all formats"

# Done! Other systems can download:
# https://github.com/ayates_dbx/docling_rs/releases/download/test-corpus-v1.0/test-corpus-v1.tar.gz
```

**Pros:**
- One command does everything
- No web UI needed
- Scriptable

**Cons:**
- Requires `gh` CLI installed

---

## Option B: Using Web UI (Easiest if no CLI)

```bash
# 1. Create archive
cd ~/docling_rs
tar -czf test-corpus-v1.tar.gz test-corpus/
# Creates: test-corpus-v1.tar.gz (~80-100MB compressed)

# 2. Go to GitHub:
open https://github.com/ayates_dbx/docling_rs/releases/new

# 3. Fill in:
- Tag: test-corpus-v1.0
- Title: Test Corpus v1.0
- Description: Test files for docling_rs (241MB uncompressed, 2509 files)

# 4. Drag test-corpus-v1.tar.gz to "Attach binaries" area

# 5. Click "Publish release"
```

**Pros:**
- No CLI needed
- Visual interface
- Easy

**Cons:**
- Manual upload

---

## How Other System Uses It

**To download and use:**
```bash
# Download from release
wget https://github.com/ayates_dbx/docling_rs/releases/download/test-corpus-v1.0/test-corpus-v1.tar.gz

# Or with curl:
curl -L -O https://github.com/ayates_dbx/docling_rs/releases/download/test-corpus-v1.0/test-corpus-v1.tar.gz

# Extract
tar -xzf test-corpus-v1.tar.gz

# Now they have: test-corpus/ directory with all 2,509 files
```

**Or add to their script:**
```bash
#!/bin/bash
if [ ! -d "test-corpus" ]; then
  echo "Downloading test corpus..."
  curl -L -O https://github.com/ayates_dbx/docling_rs/releases/download/test-corpus-v1.0/test-corpus-v1.tar.gz
  tar -xzf test-corpus-v1.tar.gz
fi
cargo test
```

---

## Next Steps

**Want me to create the release for you?**

**Using gh CLI (if available):**
```bash
gh release create test-corpus-v1.0 test-corpus-v1.tar.gz \
  --title "Test Corpus v1.0" \
  --notes "Complete test corpus (241MB uncompressed, 2509 files)"
```

**Or I can:**
1. Create the tar.gz archive (done)
2. Give you instructions to upload via web UI
3. Test the download URL works

**Which approach do you want?**
