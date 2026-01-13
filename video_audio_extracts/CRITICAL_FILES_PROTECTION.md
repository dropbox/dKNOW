# Critical Files Protection

**Created**: N=435
**Purpose**: Prevent accidental deletion of essential worker files

## Protected Files

The following files are protected by git pre-commit hook and cannot be deleted:

1. **json_to_text.py** - Converts Claude JSON output to readable text
2. **run_worker.sh** - Worker execution script

## Protection Mechanism

**Git Hook**: `.git/hooks/pre-commit-critical-files`

- Checks if protected files are being deleted in commit
- Checks if protected files exist before allowing commit
- Blocks commit if files are missing or being removed
- Override: `git commit --no-verify` (NOT RECOMMENDED)

## If Files Are Accidentally Deleted

```bash
# Restore from git history
git checkout HEAD -- json_to_text.py run_worker.sh
```

## Modifying Protected Files

Editing is allowed - only deletion is blocked. You can:
- ✅ Edit file contents
- ✅ Rename files (but update hook)
- ❌ Delete files (blocked by hook)

## To Add More Protected Files

Edit `.git/hooks/pre-commit-critical-files` and add to CRITICAL_FILES array:
```bash
CRITICAL_FILES=(
    "json_to_text.py"
    "run_worker.sh"
    "your_new_file.sh"  # Add here
)
```
