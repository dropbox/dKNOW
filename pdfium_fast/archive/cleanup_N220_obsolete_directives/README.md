# Archived Obsolete Directives (N=220 Cleanup)

**Archived**: 2025-11-22 (WORKER0 # 220)
**Reason**: Completed directives and temporary files no longer needed in root directory

## Files Archived

### Completed MANAGER Directives

1. **MANAGER_RUN_SANITIZERS_NOW.md**
   - Directive: Run ASan and TSan for memory/threading bug detection
   - Completed: N=217-218
   - Status: Zero memory bugs, zero threading bugs detected
   - Also archived in: archive/sanitizer_directive_completed_N217-218/

2. **MANAGER_FULL_BENCHMARK_NOW.md**
   - Directive: Run full test suite (N=187)
   - Completed: Historical (N=187)
   - Status: Obsolete, tests now run regularly

3. **MANAGER_MERGE_FIXES_NOW.md**
   - Directive: Merge critical fixes to main (N=197-212)
   - Completed: Historical
   - Status: Fixes merged via PR #22

### Historical Documentation

4. **BASELINE_REGENERATION_REQUIRED.md**
   - Issue: All 452 image tests failing (N=194)
   - Completed: N=213 (baseline regeneration complete)
   - Status: Resolved, baselines current

5. **BASELINE_REGENERATION_EXPLAINED.md**
   - Documentation: Explains baseline regeneration concept
   - Status: Educational content, no longer needed in root

6. **BLOCKER_ANALYSIS.md**
   - Issue: Feature branch ahead of main by 23 commits
   - Status: Historical, branch management issue resolved

7. **BUG_RETROSPECTIVE_CRITICAL.md**
   - Analysis: How N=41 BGR optimization broke correctness
   - Status: Historical lessons documented, bugs fixed
   - Lessons: Preserved in CLAUDE.md and git history

### Draft Files (Never Used)

8. **GIT_COMMIT_MESSAGE_N522.txt**
9. **GIT_COMMIT_MESSAGE_N523.txt**
   - Status: Draft commit messages for hypothetical future iterations
   - Current iteration: N=220 (these were never used)

10. **WORKER_START_PROMPT.txt**
    - Status: Temporary file

## Current System Status (N=220)

- Tests: 96/96 smoke tests pass (100%)
- Session: sess_20251122_184726_133aff07
- Sanitizers: Clean (0 memory bugs, 0 threading bugs)
- Baselines: Up to date (424/452 PDFs regenerated)
- Branch status: Clean, production-ready

## References

- N=217-218: Sanitizer validation
- N=213: Baseline regeneration
- N=220: This cleanup cycle
