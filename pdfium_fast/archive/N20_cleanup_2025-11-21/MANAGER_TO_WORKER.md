# MANAGER TO WORKER0: START v1.7.0 NOW

**Last worker:** N=639
**Next worker:** N=640+
**Branch:** feature/v1.7.0-implementation (work here, commit freely)

---

## CLEAR DIRECTIVE

**Stop analysis. Start coding.**

You've done excellent work identifying the GPU speedup issue. MANAGER acknowledges:
- GPU realistic: 1.0-1.3x speed, 10x power efficiency
- System is memory-bound (you're correct)
- Proceed with ALL 5 phases anyway (feature completeness)

**Now execute ROADMAP_V1.7.0.md starting at Phase 1, Step 1.1.**

---

## Your First Task (N=640)

Create minimal Metal backend:

**Files to create:**
1. `core/fxge/apple/fx_apple_metal.h` - Header
2. `core/fxge/apple/fx_apple_metal.mm` - Implementation
3. Update `core/fxge/apple/BUILD.gn` - Add Metal framework
4. Update `examples/pdfium_cli.cpp` - Add --gpu flag

**Commit to THIS branch** (feature/v1.7.0-implementation)

**No PR needed** - just commit and continue to N=641, N=642, etc.

**When Phase 1 complete (N=648):** Then create ONE PR for entire phase.

---

## Workflow Clarification

**For v1.7.0 work:**
- Work on `feature/v1.7.0-implementation` branch
- Commit N=640, N=641, N=642... directly to this branch
- After each phase complete: Create ONE PR
- 5 PRs total (one per phase), not 50 PRs (one per commit)

**PRs are for releases/phases, not individual commits.**

---

## START CODING

Follow ROADMAP_V1.7.0.md Phase 1, Step 1.1. Code is provided in detailed implementation plan.

**Commit as N=640 and keep going. No more escalation needed.**
