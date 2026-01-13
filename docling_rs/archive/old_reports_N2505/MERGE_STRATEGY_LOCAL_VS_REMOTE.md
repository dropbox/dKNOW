# Merge Strategy: Local vs Remote Main

**Date:** 2025-11-24 23:50 PST
**Status:** Branches diverged, merge conflicts detected

---

## Current State

**Merge Base:** 03cddb02 "[MANAGER] FINAL ANSWER - 100% Complete with Empirical Proof"

**Local main (41 commits ahead):**
- N=2040-2050: PDF ML integration work
- Manager directives for PDF fixing
- API key setup
- json_to_text.py permissions fix
- ODP image extraction fix
- Source code copied from ~/docling_debug_pdf_parsing

**Remote main (71 commits ahead):**
- N=2202-2221: Quality improvements for various formats
- FB2, TEX, TAR, ODS, GLB, VCF, etc. improvements
- README updates with test corpus download
- LLM quality analysis
- CLI backend whitelist fixes

---

## Merge Conflicts Detected

**When attempting `git merge origin/main`:**

1. **CLAUDE.md** - both modified
2. **NEXT_SESSION_START_HERE.txt** - deleted remote, modified local
3. **API_KEY_EXISTS_NO_EXCUSES.txt** - renamed/deleted conflict
4. **FIX_VERIFIED_BUGS_NOW.txt** - renamed/deleted conflict
5. **crates/docling-cli/src/main.rs** - both modified
6. **crates/docling-pdf-ml/** - MANY test files conflicted
   - executor.rs
   - 19 test files with conflicts

**Root cause of pdf-ml conflicts:**
- Local (N=2049): Copied entire src/ from ~/docling_debug_pdf_parsing
- Remote: Has different version of pdf-ml crate
- These are fundamentally different codebases for pdf-ml

---

## Merge Strategy Options

### Option A: Merge Remote Into Local (Keep PDF Work)

**Approach:**
1. Attempt merge: `git merge origin/main`
2. For each conflict:
   - CLAUDE.md: Keep local (has PDF directives)
   - pdf-ml files: Keep local (copied from source)
   - CLI: Keep local (has fixes)
   - Directive files: Keep local
3. Resolve all conflicts favoring local
4. Test compilation
5. Commit merge

**Pros:**
- Keeps PDF work intact
- Gets remote's format quality improvements
- Preserves manager directives

**Cons:**
- Loses remote's pdf-ml improvements (if any)
- Many conflicts to resolve (30+)
- Might break something

**Time:** 2-3 hours

### Option B: Rebase Local Onto Remote

**Approach:**
```bash
git rebase origin/main
# Resolve conflicts commit-by-commit
```

**Pros:**
- Linear history
- Each conflict resolved in context

**Cons:**
- 41 commits to rebase
- Very time-consuming (4-6 hours)
- Risk of breaking something midway

**Time:** 4-6 hours

### Option C: Cherry-Pick Remote Changes

**Approach:**
1. Stay on local main
2. Cherry-pick useful commits from remote:
   - Format quality improvements
   - README updates
3. Skip conflicts
4. Manually integrate test corpus info

**Pros:**
- Selective integration
- Keep local PDF work
- Faster than full merge

**Cons:**
- Manual process
- Might miss important changes
- Not a true merge

**Time:** 2-3 hours

### Option D: Start Fresh Merge Branch

**Approach:**
1. Create branch from origin/main
2. Copy PDF work from local
3. Test and commit

**Pros:**
- Clean state
- Easy to test

**Cons:**
- Loses local commit history
- Need to manually copy PDF work

**Time:** 3-4 hours

---

## Recommendation

**Use Option A (Merge origin/main, favor local for conflicts):**

### Why:
- Remote has valuable quality improvements we want
- Local has critical PDF work that's in progress
- Both can coexist
- Preserves full history

### Strategy:

**For pdf-ml files:**
- **Keep local version** (copied from ~/docling_debug_pdf_parsing)
- Remote version is outdated/broken
- Local has working code from source

**For CLAUDE.md:**
- **Merge carefully** - combine both sets of changes
- Remote might have useful additions
- Local has PDF priority directives
- Need manual merge

**For format backends:**
- **Keep remote versions** - they have quality improvements
- Don't conflict with PDF work

**For directive files:**
- **Keep local** - manager's PDF directives are current priority

---

## Test Corpus from Releases

**Per remote commit a0c45544:**

```bash
# Download test corpus
cd ~/docling_rs
curl -L https://github.com/ayates_dbx/docling_rs/releases/download/test-corpus-v1.0/test-corpus-v1.tar.gz -o test-corpus-v1.tar.gz

# Extract
tar -xzf test-corpus-v1.tar.gz

# Verify
ls -la test-corpus/
```

**This will populate test-corpus/ with test files needed for canonical tests.**

---

## Execution Plan

### Phase 1: Merge (2 hours)

1. Start merge: `git merge origin/main`

2. Resolve conflicts:
   ```bash
   # Keep local for pdf-ml files
   git checkout --ours crates/docling-pdf-ml/

   # Merge CLAUDE.md manually
   # (combine both versions)

   # Keep local for directives
   git checkout --ours *.txt

   # Keep local for CLI
   git checkout --ours crates/docling-cli/src/main.rs
   ```

3. Mark resolved: `git add .`

4. Complete merge: `git commit`

### Phase 2: Download Test Corpus (30 min)

```bash
cd ~/docling_rs
curl -L https://github.com/ayates_dbx/docling_rs/releases/download/test-corpus-v1.0/test-corpus-v1.tar.gz -o test-corpus-v1.tar.gz
tar -xzf test-corpus-v1.tar.gz
```

### Phase 3: Test (30 min)

```bash
# Verify build
cargo build --workspace

# Run tests
cargo test --lib

# Run canonical tests
USE_HYBRID_SERIALIZER=1 cargo test test_canon -- --test-threads=1
```

### Phase 4: Commit and Document (30 min)

```bash
git add test-corpus/
git commit -m "Manager: Merge origin/main + download test corpus"
```

**Total time: 3-4 hours**

---

## Risks

**Conflict resolution might break:**
- Format backends if we choose wrong version
- Build if dependencies mismatch
- Tests if test data incompatible

**Mitigation:**
- Test after each major resolution
- Keep both versions in backup
- Can abort and try different strategy

---

## Alternative: Let Worker Decide

**If merge is too complex for manager:**
- Document the situation
- Let worker handle merge conflicts
- Worker has more context about code changes
- Manager focuses on strategy/direction

---

## Next Steps

1. Decide which option to pursue
2. Execute merge strategy
3. Download test corpus
4. Test compilation
5. Document merge results
6. Continue PDF work

**Current todo list active with merge tasks.**

---

**Recommendation: Option A - Merge favoring local, then download test corpus.**
