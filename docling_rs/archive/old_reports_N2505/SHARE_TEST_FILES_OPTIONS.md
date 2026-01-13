# Options for Sharing Test Files via GitHub

**Test Corpus Size:** 241MB (2,509 files)
**Current Status:** Gitignored in main repo

---

## Option 1: Separate Test Files Repository (RECOMMENDED)

Create a dedicated repo for test files:

```bash
# Create new repo
cd ~/
mkdir docling_rs_test_files
cd docling_rs_test_files
git init
git remote add origin git@github.com:ayates_dbx/docling_rs_test_files.git

# Copy test files
cp -r ~/docling_rs/test-corpus/* .

# Commit and push
git add .
git commit -m "Add docling test corpus (241MB, 2509 files)"
git push -u origin main
```

**Pros:**
- Clean separation
- Other systems can clone just test files
- Doesn't bloat main repo history
- Easy to update independently

**Cons:**
- Need to create new GitHub repo
- Two repos to manage

---

## Option 2: GitHub Release with Archive (GOOD FOR ONE-TIME SHARE)

Create a release with test files as downloadable archive:

```bash
# Create archive
cd ~/docling_rs
tar -czf test-corpus.tar.gz test-corpus/

# Check size
ls -lh test-corpus.tar.gz

# Then manually:
# 1. Go to https://github.com/ayates_dbx/docling_rs/releases/new
# 2. Create new release (e.g., "test-corpus-v1.0")
# 3. Upload test-corpus.tar.gz as release asset
# 4. Other systems download from release page
```

**Pros:**
- No new repo needed
- One-time upload
- Easy download URL

**Cons:**
- Manual upload via web UI
- Updates require new release
- Not version controlled

---

## Option 3: Git LFS in Main Repo

Use Git Large File Storage for binary files:

```bash
# Install Git LFS
brew install git-lfs  # or: apt-get install git-lfs
git lfs install

# Track large files
git lfs track "test-corpus/**/*.pdf"
git lfs track "test-corpus/**/*.docx"
git lfs track "test-corpus/**/*.xlsx"
git lfs track "test-corpus/**/*.pptx"
# ... etc for all binary formats

# Update .gitignore to allow test-corpus
# Then commit
git add .gitattributes test-corpus/
git commit -m "Add test corpus via Git LFS"
git push origin main
```

**Pros:**
- Part of main repo
- Version controlled
- Handles large files well

**Cons:**
- Requires Git LFS setup
- GitHub LFS has bandwidth limits (1GB/month free)
- More complex setup

---

## Option 4: Remove from gitignore (If Size OK)

Commit directly to main repo if size acceptable:

```bash
# Check if any files > 100MB (GitHub limit)
find test-corpus -type f -size +100M

# If none, can commit directly:
# Edit .gitignore - remove "/test-corpus/*" line
vim .gitignore

# Commit
git add test-corpus/
git commit -m "Add test corpus for sharing (241MB)"
git push origin main
```

**Pros:**
- Simplest approach
- Everything in one repo

**Cons:**
- 241MB will bloat repo
- Slow clones for users who don't need test files
- GitHub recommends <1GB total repo size

---

## RECOMMENDATION: Option 1 (Separate Repo)

**Best for your use case:**
1. Create `docling_rs_test_files` repo
2. Copy test-corpus into it
3. Share repo URL with other system
4. They clone and get all test files

**Quick start:**
```bash
# I can create the repo and push files if you want
# Just confirm and I'll do it
```

**Which option do you prefer?**
