# Merge Blocked: Immutable File Requires User Action

**Date**: 2025-11-16
**Worker**: WORKER0 N=200
**Status**: BLOCKED - Requires sudo access

## Problem

Merge of `feature/image-threading` into `main` is blocked by an immutable file that Git cannot remove.

**File**: `/Users/ayates/pdfium_fast/json_to_text.py`
**Permissions**: `-r-xr-xr-x@` (macOS extended attributes prevent deletion)
**Error**: `error: unable to unlink old 'json_to_text.py': Operation not permitted`

## What Was Done (N=200)

1. ✅ Moved conflicting untracked files to `/tmp/pdfium_merge_backup_N200/`:
   - fpdfsdk/fpdf_parallel.cpp
   - public/fpdf_parallel.h
   - reports/feature-image-threading/
   - third_party/concurrentqueue/

2. ✅ Verified working tree is clean (no untracked files)

3. ❌ Attempted merge: Blocked by immutable `json_to_text.py`

## User Action Required

Remove the immutable file:

```bash
cd /Users/ayates/pdfium_fast
sudo chflags nouchg json_to_text.py
sudo rm json_to_text.py
```

Then the merge command will work:

```bash
git merge --no-ff feature/image-threading
```

(The merge message is prepared in MERGE_BLOCKER.md, but Git should auto-generate it since the merge command was interrupted)

## Alternative: Try Without Sudo

If you don't have sudo access, try:

```bash
chflags nouchg json_to_text.py
rm json_to_text.py
git merge --no-ff feature/image-threading
```

## Why This Happens

The `json_to_text.py` file has macOS extended attributes that make it immutable (unchangeable/undeletable). This was likely set accidentally with `chflags uchg` or similar. Git cannot override these OS-level protections.

## Next Steps

After you remove the file:

1. Run: `git merge --no-ff feature/image-threading`
2. Next AI (N=201) will:
   - Verify merge success
   - Run smoke tests (67 tests)
   - Run full test suite (2,757 tests)
   - Update documentation
   - Clean up MERGE_BLOCKER.md and this file

## Notes

- File is already in .gitignore (commit 663e528c)
- Backup exists at: `/tmp/pdfium_merge_backup_N200/` (optional restoration)
- Working tree is otherwise clean and ready for merge
