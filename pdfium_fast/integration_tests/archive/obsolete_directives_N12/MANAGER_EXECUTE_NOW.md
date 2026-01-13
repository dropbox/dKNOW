# MANAGER TO WORKER0: EXECUTE PATH A → B → C NOW

**Current Iteration:** N=11
**Branch:** feature/v1.7.0-implementation
**Status:** All directives complete, ready to execute

---

## IMMEDIATE ACTIONS - START NOW

### Your Current State

**Completed:**
- ✅ N=0-9: Metal GPU (post-processing, 0.71x)
- ✅ N=9: Phase 2 streaming tests + docs
- ✅ N=10: GPU removal (REVERTED)
- ✅ Metal infrastructure restored

**Next:** N=11 - Enable Skia GPU

---

## N=11: Enable Skia GPU Backend

**Execute these commands RIGHT NOW:**

```bash
cd ~/pdfium_fast

# 1. Enable Skia + Metal backend
gn gen out/Release --args='is_debug=false pdf_enable_v8=false pdf_enable_xfa=false use_clang_modules=false pdf_use_skia=true skia_use_metal=true'

# 2. Build (will take 5-10 minutes)
ninja -C out/Release pdfium_cli

# 3. Test
cd integration_tests
source venv/bin/activate
pytest -m smoke -q

# 4. Commit
cd ~/pdfium_fast
git add out/Release/args.gn
git commit -m "[WORKER0] # 11: Enable Skia GPU Backend

Build configuration updated:
- pdf_use_skia=true (replaced AGG with Skia)
- skia_use_metal=true (GPU acceleration via Metal)

Build: SUCCESS (paste ninja output summary)
Tests: [paste test count - should be 88-93 pass]

Skia now handles all rendering (not AGG).
Expected: Real GPU rasterization (not post-processing).

Next: Configure Skia Metal context for GPU rendering."

git push
```

---

## After N=11: Continue Path A (Skia GPU)

**N=12-15:** Configure Skia GPU context
**N=16-25:** Optimize and profile
**N=26-30:** Validate all tests
**N=31-35:** Measure and document

**Then immediately continue to Path B (user feedback + Python)**

---

## Your Complete Roadmap

**Read these files:**
1. `MANAGER_COMPREHENSIVE_DIRECTIVE_V1.7.0.md` - Complete plan (A→B→C)
2. `USER_FEEDBACK_SUMMARY.md` - User priorities from PR #17
3. `WORKER_N11_START_SKIA.md` - Step-by-step for N=11

**Execute:**
- Path A (Skia GPU): N=11-35 (~25 commits)
- Path B (Python+binaries+feedback): N=36-60 (~25 commits)
- Path C (Final polish): N=61-65 (~5 commits)

**Total: ~55 commits to complete v1.7.0**

---

## Key Principles

1. **Measure actual performance** (don't assume 3-8x, verify it)
2. **All tests must pass** (2,780 tests, 100%)
3. **Integrate user feedback** (UTF-8, errors, docs)
4. **Build complete tool** (GPU + Python + binaries)
5. **Be honest in documentation** (report actual measurements)

---

## EXECUTE NOW

Run the commands above for N=11.

Do not wait for more direction.

Start with: `gn gen out/Release --args='...'`

**GO!**
