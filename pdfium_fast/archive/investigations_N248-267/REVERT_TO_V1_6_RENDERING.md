# SOLUTION: Revert Rendering to Exact v1.6.0 Behavior

**User is correct:** We should use v1.6.0 baselines and v1.6.0 rendering code.

---

## The Problem

**After v1.6.0:** 400+ commits to pdfium_cli.cpp

**Many changed rendering:**
- BGR mode drama (N=41-213)
- Form rendering tweaks
- Quality flag changes (N=193, N=413)
- Threading changes
- Format changes

**Result:** Output completely different from v1.6.0 baselines.

---

## The Solution (Clean Slate)

**Revert pdfium_cli.cpp to v1.6.0 version:**

```bash
cd ~/pdfium_fast

# Restore v1.6.0 rendering code
git checkout v1.6.0 -- examples/pdfium_cli.cpp

# Keep v1.7-v2.0 features we want:
# - JPEG output (manually re-add)
# - Presets (manually re-add)
# - Zero-flag defaults (manually re-add)

# But use v1.6.0 RENDERING logic (no form changes, no format changes)
```

**Then tests should pass with v1.6.0 baselines.**

---

## Alternative: Accept We've Diverged

**Reality:** We've made 400+ commits to rendering code.

**We cannot go back to v1.6.0 and keep all features.**

**Must choose:**
1. **v1.6.0 compatibility** (revert to v1.6.0 rendering, lose features)
2. **Keep features** (accept divergence, regenerate baselines)

---

## My Recommendation

**User wants v1.6.0 baselines** - this means they want v1.6.0 RENDERING BEHAVIOR.

**To achieve this:**
1. Revert examples/pdfium_cli.cpp to v1.6.0
2. Re-add ONLY safe features (JPEG output, presets, zero-flags)
3. Do NOT add rendering changes (no form tweaks, no BGR, no format changes)
4. Test against v1.6.0 baselines
5. Should pass 100%

**This gives clean slate with known-good rendering.**
