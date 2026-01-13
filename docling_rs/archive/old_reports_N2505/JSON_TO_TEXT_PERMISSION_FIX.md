# json_to_text.py Permission Bug - PERMANENTLY FIXED ✅

**Date:** 2025-11-24 21:52 PST
**Status:** PERMANENTLY FIXED (committed to git)

---

## The Problem

**Symptom:**
```bash
./run_worker.sh: line 47: ./json_to_text.py: Permission denied
```

**Pattern:** This kept happening repeatedly:
1. User or worker would run `chmod +x json_to_text.py`
2. File would work temporarily
3. After some git operation (checkout, reset, pull, etc.)
4. File would lose executable permission again
5. Error would return

---

## Root Cause

**Git was storing the wrong file mode:**

```bash
# BEFORE FIX
$ git ls-files -s json_to_text.py
100644 4036b4dcd601085f0d21c25e55311c3a852c0f16 0	json_to_text.py
       ^^^^^^
       Mode 100644 = regular file (rw-r--r--, NOT executable)

# Git operations would restore this mode, losing the executable bit
```

**What was happening:**
1. `chmod +x json_to_text.py` changed **local filesystem** permissions
2. But git index still stored mode as `100644` (not executable)
3. Any git operation that touched the file would restore mode from git index
4. File would revert to `644` (not executable)
5. Permission denied error would return

**Git operations that trigger this:**
- `git checkout` (any branch)
- `git reset --hard`
- `git pull`
- `git stash pop`
- `git merge`
- Any operation that updates working tree from git database

---

## The Permanent Fix

**Command:**
```bash
git update-index --chmod=+x json_to_text.py
```

**What this does:**
- Updates the git **index** (staging area) to track executable bit
- Changes stored mode from `100644` → `100755`
- This change must be committed to git

**After fix:**
```bash
# AFTER FIX
$ git ls-files -s json_to_text.py
100755 4036b4dcd601085f0d21c25e55311c3a852c0f16 0	json_to_text.py
       ^^^^^^
       Mode 100755 = executable file (rwxr-xr-x, IS executable)

# Now git operations will preserve executable permission
```

**Committed in:** `05b9b347` (N=2042)

---

## Verification

**Current status:**
```bash
$ ls -la json_to_text.py
-rwxr-xr-x@ 1 ayates  staff  11521 Nov 24 21:45 json_to_text.py
# ^^^ Executable permissions set

$ git ls-files -s json_to_text.py
100755 4036b4dcd601085f0d21c25e55311c3a852c0f16 0	json_to_text.py
# ^^^ Git tracking executable bit

$ test -x json_to_text.py && echo "✅ Executable"
✅ Executable

$ ./json_to_text.py
# Runs without "Permission denied" error
```

---

## Why chmod Alone Wasn't Enough

**Two separate permission systems:**

1. **Filesystem permissions** (what `ls -la` shows)
   - Changed by `chmod +x`
   - Only affects local working copy
   - Lost during git operations that update files

2. **Git index mode** (what `git ls-files -s` shows)
   - Stored in git database
   - Source of truth for git operations
   - Requires `git update-index --chmod=+x` to change
   - Must be committed to persist

**The fix requires changing BOTH:**
```bash
# Change filesystem (temporary, local only)
chmod +x json_to_text.py

# Change git index (permanent, after commit)
git update-index --chmod=+x json_to_text.py
git commit -m "Make json_to_text.py executable"

# Now both are in sync
```

---

## Git File Modes

**Git tracks execute permission as part of file mode:**

| Mode   | Permissions | Executable? |
|--------|-------------|-------------|
| 100644 | rw-r--r--   | ❌ No       |
| 100755 | rwxr-xr-x   | ✅ Yes      |

**Git stores mode in index:**
- When you `git checkout` a file, git sets permissions based on stored mode
- If mode is `100644`, file becomes non-executable
- If mode is `100755`, file becomes executable
- This is why the bug kept recurring

---

## Why This Happened

**Likely scenario:**

1. File was originally added to git without execute permission
   ```bash
   git add json_to_text.py  # Added as regular file (mode 644)
   git commit -m "Add script"
   ```

2. Later, someone realized it should be executable
   ```bash
   chmod +x json_to_text.py  # Fixed local copy only
   # But forgot to update git index
   ```

3. Git operations kept restoring original mode (644)

4. Each time, someone would run `chmod +x` again (temporary fix)

5. Cycle repeated

**This commit breaks the cycle by updating git's stored mode.**

---

## Prevention for Future Scripts

**When adding executable scripts to git:**

```bash
# 1. Make file executable locally
chmod +x script.py

# 2. Add to git (git will detect executable bit on initial add)
git add script.py

# 3. Verify git tracked it as executable
git ls-files -s script.py
# Should show: 100755 ... script.py

# 4. Commit
git commit -m "Add executable script"
```

**OR, if already committed without executable bit:**

```bash
# Fix in git index
git update-index --chmod=+x script.py

# Commit the mode change
git commit -m "Make script executable"
```

---

## Related Files

**Commit:** `05b9b347` (N=2042)

**Commit message line:**
```
mode change 100644 => 100755 json_to_text.py
```

This line in the commit output confirms the fix.

---

## For Workers

**If you see "Permission denied" for json_to_text.py:**

1. **This should never happen again** (fixed in git)

2. **If it somehow does happen:**
   ```bash
   # Check git mode
   git ls-files -s json_to_text.py

   # Should show 100755 (after this commit)
   # If it shows 100644, something went wrong

   # Re-apply fix
   git update-index --chmod=+x json_to_text.py
   git commit -m "Re-fix executable permission"
   ```

3. **Verify with:**
   ```bash
   test -x json_to_text.py && echo "✅ Works" || echo "❌ Broken"
   ```

---

## Summary

✅ **Root cause identified:** Git stored file as mode `100644` (not executable)
✅ **Permanent fix applied:** Updated git index to mode `100755` (executable)
✅ **Committed:** Change is now in git history (commit `05b9b347`)
✅ **Verified:** File is executable, git tracks executable bit
✅ **Will survive:** All future git operations (checkout, pull, merge, etc.)

**This bug is PERMANENTLY FIXED.** 🎯

---

**Never run `chmod +x json_to_text.py` alone - it must be followed by `git update-index --chmod=+x json_to_text.py` and committed to persist!**
