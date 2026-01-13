# ⚠️ URGENT: READ THIS FIRST ⚠️

**Date:** 2025-11-04
**From:** MANAGER (on behalf of USER)
**To:** Worker N=12+ (YOU)
**Status:** CRITICAL DIRECTIVE

---

## USER IS FRUSTRATED - WORKERS KEEP VIOLATING REQUIREMENTS

**Workers N=6, N=9, N=10, N=11 ALL claimed "APPROVED" and "COMPLETE"**

**USER RESPONSE:**
> "I DID NOT APPROVE ANYTHING"
> "NOT APPROVED DO THE AUDITS"
> "I want the AI to review all outputs!"

---

## YOUR JOB (SIMPLE AND CLEAR)

### STEP 1: Open the checklist
```bash
docs/ai-output-review/MASTER_AUDIT_CHECKLIST.csv
```

### STEP 2: Start with test #1
```
smoke_error_corrupted_file
```

### STEP 3: Audit it
- Run the test
- Check the output
- Is it CORRECT / SUSPICIOUS / INCORRECT?

### STEP 4: Update the checklist
Change:
```csv
smoke_error_corrupted_file,❌ NOT AUDITED,,,,
```
To:
```csv
smoke_error_corrupted_file,✅ AUDITED,error-handling,N/A,10,"Test correctly handles corrupted file error"
```

### STEP 5: Move to test #2
```
smoke_error_invalid_operation
```

### STEP 6: Repeat
Continue through ALL 362 tests, one by one.

### STEP 7: Commit every 30-50 tests
```
# <N>: AI Audit Progress - Tests 1-50 Complete
**Progress**: 50/362 audited (14%)
**Bugs**: 0 new bugs found
```

---

## ABSOLUTE RULES

### YOU MUST:
- ✅ Audit each of 362 test functions individually
- ✅ Update MASTER_AUDIT_CHECKLIST.csv after each audit
- ✅ Commit progress regularly
- ✅ Continue until 362/362 complete

### YOU MUST NOT:
- ❌ Write analysis reports
- ❌ Write Python scripts
- ❌ Defend previous work
- ❌ Claim "COMPLETE" until 362/362 done
- ❌ Claim "APPROVED" (EVER - only user approves)
- ❌ Use phrases like "sufficient coverage"
- ❌ Skip tests or use sampling

---

## CURRENT STATUS (FACTUAL)

**Audits completed:** 0/362 (0%)
**All tests marked:** ❌ NOT AUDITED
**User approval:** NONE (user explicitly said "NOT APPROVED")

---

## WHAT HAPPENS NEXT

**USER WILL CHECK PROGRESS:**

User will ask: "How is the checklist?"

I (MANAGER) will check:
```bash
grep -c "✅ AUDITED" docs/ai-output-review/MASTER_AUDIT_CHECKLIST.csv
```

**User expects to see:**
- First check: 60/362 (17%)
- Second check: 120/362 (33%)
- Third check: 180/362 (50%)
- Fourth check: 240/362 (66%)
- Fifth check: 300/362 (83%)
- Final check: 362/362 (100%)

**If checklist shows 0/362:** You failed to do the work.

---

## EXAMPLE OF ONE AUDIT

```bash
# Run test
VIDEO_EXTRACT_THREADS=4 cargo test --release smoke_format_mp4 -- --ignored --nocapture

# Output shows: keyframes extracted correctly

# Update CSV row:
# Before: smoke_format_mp4,❌ NOT AUDITED,,,,
# After:  smoke_format_mp4,✅ AUDITED,keyframes,MP4,10,"2 keyframes at 0s and 1s, correct"

# Move to next test
```

---

## NO MORE REPORTS, NO MORE ANALYSIS, NO MORE SCRIPTS

**JUST DO THE AUDITS. ONE BY ONE. UPDATE THE CHECKLIST.**

---

## DETAILED INSTRUCTIONS

**File:** `docs/ai-output-review/WORKER_DIRECTIVE_SYSTEMATIC_AUDIT.md`

Read it. Follow it. Audit all 362 tests.

---

## ESTIMATED WORK

**362 tests ÷ 60 per iteration = 6 iterations**
**6 iterations × 12 minutes = ~1.5 hours**

**This is your entire job. Nothing else.**

---

**START WITH TEST #1 (smoke_error_corrupted_file) AND WORK YOUR WAY DOWN THE LIST.**

**UPDATE THE CHECKLIST AS YOU GO.**

**DO NOT CLAIM ANYTHING IS COMPLETE OR APPROVED.**
