# WORKER0: Your Next Iteration is N=640

**Last iteration:** N=639 (roadmap updated)
**This iteration:** N=640 (BEGIN Phase 1 implementation)
**Branch:** feature/v1.7.0-implementation (you're here)

---

## What You Do Now

**Read these files:**
1. `ROADMAP_V1.7.0.md` - Phase 1, Step 1.1
2. `MANAGER_DIRECTIVE_V1.7.0_REVISED.md` - Context
3. `MANAGER_TO_WORKER.md` - Workflow clarification

**Then execute Phase 1, Step 1.1:**

Create 4 files:
- core/fxge/apple/fx_apple_metal.h
- core/fxge/apple/fx_apple_metal.mm
- Update core/fxge/apple/BUILD.gn
- Update examples/pdfium_cli.cpp (add --gpu flag)

Build and test:
```bash
ninja -C out/Release pdfium_cli
cd integration_tests && pytest -m smoke
```

Commit as N=640, continue to N=641.

**Commit directly to this branch. No PRs until phase complete.**

---

## Workflow

**Commit freely:**
- N=640, N=641, N=642... all on this branch
- Push after each commit (git push)
- No PR needed for each commit

**PR only when complete:**
- After Phase 1 done (N~648): Create PR
- After Phase 2 done: Create PR
- 5 PRs total (one per phase)

---

## START NOW

Execute ROADMAP_V1.7.0.md Phase 1, Step 1.1.
