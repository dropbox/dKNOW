# MANAGER: MERGE CRITICAL FIXES TO MAIN NOW

**BLOCKER:** Main branch missing critical bug fixes from feature branch

---

## Critical Commits to Merge (N=197-212)

**N=197:** K=1 vs K>1 rendering correctness bug
**N=200-202:** Form field rendering fixes
**N=203-206:** Baseline regeneration
**N=207:** Bitmap format regression fix
**N=209-212:** Threading race condition fixes

**These are ESSENTIAL for correctness.**

---

## Solution: Create Clean PR with Fixes Only

**Worker N=213:**

```bash
cd ~/pdfium_fast
git checkout main
git pull origin main

# Create clean branch for fixes only
git checkout -b hotfix/critical-rendering-fixes

# Cherry-pick essential fixes (avoid health loop commits)
git cherry-pick 6daab72250  # N=197: K=1 vs K>1 fix
git cherry-pick b9d5daff84  # N=200: Form field rendering
git cherry-pick 73a2f4ba08  # N=202: Transparency fix
git cherry-pick 4cc523c985  # N=207: Bitmap format fix
git cherry-pick 28b2940f9c  # N=210: Threading race fix
git cherry-pick fb6ac2cbf2  # N=212: Baseline script fix

# Test
cd integration_tests
pytest -m smoke

# If pass: Push
git push -u origin hotfix/critical-rendering-fixes

# Create PR
gh pr create --title "HOTFIX: Critical Rendering & Threading Fixes" \
  --body "Merges essential bug fixes from feature branch:
- N=197: K=1 vs K>1 correctness
- N=200-202: Form rendering
- N=207: Bitmap format regression
- N=210: Threading race condition
- N=212: Baseline fixes

Tests: Should pass 96/96

Critical for correctness." --base main
```

---

## Or: Merge Entire Feature Branch

If cherry-pick is too complex:

```bash
git checkout main
git merge feature/v1.7.0-implementation -m "Merge critical fixes from feature branch"
# Resolve conflicts
# Remove clutter (keep 6 essential .md files)
git push origin main
```

---

## Worker: Do This NOW (N=213)

**Priority:** CRITICAL

Get these fixes onto main, then run full benchmark.
