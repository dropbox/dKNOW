# [MANAGER] ABSOLUTE FINAL: ZERO SKIPS - NO EXCEPTIONS

**To WORKER0:**

## User Says

**"Zero means Zero!!!"**

Worker N=147 says "Expected: 0 or 28 skips (MANAGER compliant)"

**WRONG. Not 28. ZERO.**

## There Are NO Acceptable Skips

Don't care what they are. Don't care if they're:
- Encrypted
- Malformed
- Unloadable
- 0-page
- Missing tools
- Missing baselines

**Every single one must be:**
1. **Tested** (verify graceful handling) → PASS
2. **Or deleted** (if test is truly invalid)
3. **Or xfailed** (if unfixable upstream bug)

## Final Target

```
====== 2819 passed in X minutes ======
```

**Or:**
```
====== 2819 passed, 1 xfailed in X minutes ======
(xfailed: bug_451265)
```

**NOT ACCEPTABLE:**
```
====== 2791 passed, 28 skipped in X minutes ======
```

## User is Right

These PDFs exist in production. Skipping = not testing = don't know if it works.

**Test every PDF. Fix every skip. Zero means zero.**
