# CRITICAL: Form Rendering Added After v1.6.0

**This is why baselines don't match!**

---

## The Issue

**v1.6.0:** Did NOT render form fields
**Current:** DOES render form fields (added at N=200-202)

**Form rendering adds content to output → changes MD5!**

**This is a FEATURE, not a bug** - we render more content than v1.6.0.

---

## The Problem

**v1.6.0 baselines** were generated WITHOUT form rendering.

**Current code** renders forms (N=200-202).

**Result:** Every PDF with forms has different MD5.

**This is why user said baselines were "correct in the beginning" - they matched the code at that time!**

---

## The Impossible Situation

**Can't use v1.6.0 baselines IF we added form rendering:**
- v1.6.0 baselines expect NO forms
- Current code renders forms
- Incompatible!

**Options:**
1. Remove form rendering (revert to v1.6.0 behavior) - lose feature
2. Regenerate baselines with form rendering - lose upstream comparison
3. Have TWO sets of baselines (with/without forms) - complex

---

## User is Right - But We're Stuck

**User's point:** "Baselines were correct in the beginning"

**YES - but we ADDED features after v1.6.0:**
- Form field rendering
- Threading improvements
- Bug fixes

**These intentionally changed output.**

---

## What To Do

**Need user decision:**

**Option A: Revert to v1.6.0 Code**
- Remove form rendering
- Remove all changes after v1.6.0
- Baselines will match
- Lose features

**Option B: Accept Our Changes**
- Keep form rendering
- Regenerate baselines
- Lose upstream comparison baseline
- Keep features

**Option C: Dual Validation**
- Keep v1.6.0 baselines for non-form PDFs
- New baselines for form PDFs
- Complex but maintains ground truth

---

## My Assessment

**We can't go back to v1.6.0 baselines if we want:**
- Form rendering (N=200-202)
- Bug fixes (N=197-213)
- Features (v1.7-v2.0)

**These changed output intentionally.**

**User needs to decide:** Keep features (regenerate) OR match v1.6.0 (revert features)?
