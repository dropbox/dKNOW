#!/bin/bash
# Purge test_files_wikimedia from all git history using native git commands

echo "Creating clean branch from latest code..."

# Find first commit WITHOUT test files (after N=432 cleanup)
CLEAN_COMMIT=$(git log --oneline --all --grep "Git Repository Size Reduction" | head -1 | cut -d' ' -f1)
echo "Clean commit: $CLEAN_COMMIT"

# Create orphan branch (fresh history)
git checkout --orphan alpha-release

# Add only current files (test files already excluded via .gitignore)
git add -A

# Commit
git commit -m "Alpha v0.1.0 - Clean history without test files

39 formats, 32 plugins, comprehensive documentation.
Test files excluded (documented in TEST_DATA_MANIFEST.md).

All code and documentation preserved.
Git history rewritten to remove large test files."

echo ""
echo "✅ Clean branch created: alpha-release"
echo "Next: git push origin alpha-release"
