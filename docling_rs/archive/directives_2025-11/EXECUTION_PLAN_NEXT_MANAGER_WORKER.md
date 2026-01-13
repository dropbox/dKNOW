# Execution Plan - Next Manager & Worker

**Date:** 2025-11-25 01:10 PST
**Session:** Manager N=2042-2050 (completed)
**Current State:** Feature branch ready, PDF bug identified, worker ready to execute

---

## EXECUTIVE SUMMARY

**Status:** ✅ All infrastructure ready, PDF bug identified and isolated

**Problem:** PDF produces 80 DocItems instead of 53 (51% over-fragmented)

**Root Cause:** Bug in docling_rs integration code (crates/docling-backend/src/pdf.rs), NOT in source repo

**Next Step:** Worker fixes integration bug on feature branch

**Time Estimate:** 6-8 hours to reach 100% PDF quality

---

## WHAT WAS ACCOMPLISHED (Manager Session)

### 1. ✅ Setup & Infrastructure
- Fixed json_to_text.py executable permissions (permanently in git)
- Set up .env and .env.backup with API keys (write-protected)
- Updated CLAUDE.md with PDF top priority
- Merged with origin/main (71 commits)
- Downloaded test corpus (105MB from releases)
- Cleaned API keys from git history
- Closed 16 outdated PRs

### 2. ✅ Root Cause Analysis
- Tested source repo (~/docling_debug_pdf_parsing)
- Confirmed source repo is CORRECT (21/21 comprehensive tests pass)
- Identified bug is in docling_rs integration, not source code
- Found two bugs:
  1. Over-fragmentation: 53 → 80 DocItems
  2. Reading order: Title at #4 instead of #0

### 3. ✅ Created Clear Directives
- START_HERE_FIX_PDF_NOW.txt - Main worker directive
- PDF_MUST_BE_EXACT_53_DOCITEMS.txt - Zero tolerance requirement
- URGENT_PDF_FRAGMENTATION_BUG.txt - Bug description
- CRITICAL_DOCITEMS_NOT_MARKDOWN.txt - Focus on DocItems
- CRITICAL_QUESTION_FOR_USER.md - Investigation results

### 4. ✅ Branch Management
- Created feature branch: feature/manager-pdf-investigation-n2042-2310
- Pushed to remote (includes manager work + worker progress N=2051-2311)
- Main branch synced with origin
- Clean PR list (all outdated PRs closed)

---

## CURRENT STATE

### Repository Structure

**Main branch:**
- Synced with origin/main
- Clean (no outstanding work)
- At N=2311

**Feature branch:** `feature/manager-pdf-investigation-n2042-2310`
- Contains all manager's PDF investigation work
- Contains all PDF directives
- Test corpus downloaded and wired up
- Source code from ~/docling_debug_pdf_parsing integrated
- Worker continued work to N=2311

### The PDF Bug (Isolated)

**File:** test-corpus/pdf/multi_page.pdf

**Python baseline:**
```
53 DocItems (16 text, 11 section_header, 26 list_item)
9,456 chars markdown
Reading order: Title first, then body
```

**Current Rust:**
```
80 DocItems (27 MORE - 51% over-fragmented)
7,400 chars markdown
Reading order: Body first, title at #4 (WRONG)
```

**Bug location:** crates/docling-backend/src/pdf.rs (lines 1250-1350)
- Integration code that calls source repo functions
- Reading order calculation ignores assembled.reading_order field
- Possible extra fragmentation in text cell handling

**Source repo status:** ✅ CORRECT (confirmed by other AI)
- Passes 21/21 comprehensive tests on 4 PDFs
- to_docling_document_multi() function works correctly
- Bug is NOT in source code, bug is in HOW we call it

---

## EXECUTION PLAN - NEXT MANAGER

### Role: Verify Worker Progress & Provide Direction

**Duration:** 30-60 minutes per check-in

### Tasks:

#### 1. Check Worker Status (15 min)
```bash
git fetch origin
git checkout feature/manager-pdf-investigation-n2042-2310
git log --oneline -10
git log -1 --format="%B"  # Read latest commit
```

**Look for:**
- Has worker committed anything?
- What did they work on?
- Are they following directives?

#### 2. If Worker Made Progress (15 min)

**Check test results:**
```bash
cd ~/docling_rs
git checkout feature/manager-pdf-investigation-n2042-2310
source setup_env.sh
cargo test -p docling-backend --test pdf_honest_test --features pdf-ml -- --nocapture 2>&1 | grep "Total DocItems\|Output:"
```

**Questions:**
- How many DocItems now? (Target: EXACTLY 53)
- How many chars? (Target: ~9,456)
- Did reading order improve?

**If improved:** Encourage, check remaining issues
**If stuck:** Read worker's commits, provide hints
**If worse:** Redirect to correct approach

#### 3. If Worker Is Off-Track (15 min)

**Check what they're doing:**
- Read START_HERE_FIX_PDF_NOW.txt - did they read it?
- Are they debugging wrong thing?
- Did they add logging as instructed?

**Redirect:**
- Create new directive if needed
- Point to specific line numbers in pdf.rs
- Clarify the bug if confused

#### 4. If Worker Succeeded (30 min)

**Verify 100% quality:**
```bash
source .env
cargo test -p docling-backend --test pdf_honest_test \
  test_pure_rust_vs_python_baseline_with_llm \
  --features pdf-ml -- --ignored --nocapture
```

**Must show:**
- ✅ EXACTLY 53 DocItems
- ✅ ~9,456 chars output
- ✅ Test PASSES (not fails)
- ✅ LLM quality: 100%

**Then:**
- Help worker create PR to merge feature branch to main
- Document success
- Identify next priorities

### Manager Success Criteria

- ✅ Provided clear status assessment
- ✅ Gave actionable direction
- ✅ Kept worker on track
- ✅ Verified quality when claimed complete

---

## EXECUTION PLAN - NEXT WORKER

### Role: Fix PDF DocItems Bug

**Duration:** 6-8 hours (single focused session)

### Prerequisites (5 min)

```bash
cd ~/docling_rs
git checkout feature/manager-pdf-investigation-n2042-2310
git pull origin feature/manager-pdf-investigation-n2042-2310

# Read directives
cat START_HERE_FIX_PDF_NOW.txt
cat PDF_MUST_BE_EXACT_53_DOCITEMS.txt

# Set up environment
source setup_env.sh
source .env
```

### Step 1: Add Stage-by-Stage Logging (1 hour)

**File:** crates/docling-backend/src/pdf.rs

**Add these eprintln statements:**

```rust
// Line ~145 (after pdfium extraction)
eprintln!("[STAGE 1] Pdfium text segments: {}", segment_count);

// Line ~218 (after merge_horizontal_cells)
eprintln!("[STAGE 2] Merged text cells: {}", text_cells.len());

// Line ~1296 (after ML pipeline)
let pe_count = page_result.assembled.as_ref().map(|a| a.elements.len()).unwrap_or(0);
eprintln!("[STAGE 3] Page {} PageElements: {}", page_idx, pe_count);

// Line ~1323 (after to_docling_document_multi)
eprintln!("[STAGE 4] DocItems before convert: {}", pdf_ml_docling_doc.texts.len());

// Line ~1335 (after convert_to_core)
eprintln!("[STAGE 5] DocItems after convert: {}", core_docling_doc.content_blocks.as_ref().map(|v| v.len()).unwrap_or(0));
```

**Test:**
```bash
cargo test -p docling-backend --test pdf_honest_test --features pdf-ml -- --nocapture 2>&1 | grep STAGE
```

**Expected output like:**
```
[STAGE 1] Pdfium text segments: 200
[STAGE 2] Merged text cells: 80
[STAGE 3] Page 0 PageElements: 14
[STAGE 3] Page 1 PageElements: 9
...
[STAGE 4] DocItems before convert: 53 (or 80?)
[STAGE 5] DocItems after convert: 80
```

**This tells you WHERE the bug is!**

### Step 2: Identify Bug Location (30 min)

**Based on logs:**

**If STAGE 3 total = ~53 but STAGE 4 = 80:**
- Bug in to_docling_document_multi() expansion
- But source repo tests prove this works
- Check: Are we passing wrong parameters?

**If STAGE 4 = 53 but STAGE 5 = 80:**
- Bug in convert_to_core_docling_document()
- It's splitting DocItems during conversion
- Fix the converter

**If STAGE 3 total = ~80:**
- Bug in ML pipeline creating too many PageElements
- OR bug in STAGE 2 (merge_horizontal_cells not merging enough)
- Check merge logic

**If STAGE 2 = 80:**
- Bug is in merge_horizontal_cells (not merging aggressively enough)
- Check thresholds (horizontal_threshold, vertical_threshold)

### Step 3: Fix the Bug (2-4 hours)

**Most Likely Fix (based on manager's analysis):**

**Reading Order Bug (pdf.rs lines 1304-1320):**

Current code:
```rust
let page_reading_orders: Vec<Vec<usize>> = pages
    .iter()
    .map(|page| {
        if let Some(assembled) = &page.assembled {
            assembled.elements.iter().enumerate().map(|(i, _)| i).collect()
        } else {
            vec![]
        }
    })
    .collect();
```

**This ignores the actual reading_order!**

Fixed code:
```rust
let page_reading_orders: Vec<Vec<usize>> = pages
    .iter()
    .map(|page| {
        if let Some(assembled) = &page.assembled {
            // Use actual reading order if available
            if !assembled.reading_order.is_empty() {
                assembled.reading_order.clone()
            } else {
                // Fallback to sequential
                (0..assembled.elements.len()).collect()
            }
        } else {
            vec![]
        }
    })
    .collect();
```

**Other possible fixes:**
- Adjust merge_horizontal_cells thresholds if STAGE 2 = 80
- Fix PageElement grouping if STAGE 3 = 80
- Fix converter if STAGE 5 = 80

### Step 4: Test After Each Change (15 min per iteration)

```bash
source setup_env.sh
cargo test -p docling-backend --test pdf_honest_test --features pdf-ml -- --nocapture 2>&1 | grep -E "Total DocItems|Output:|STAGE"
```

**Keep fixing until:**
- Total DocItems: 53
- Output: ~9,456 chars
- STAGE logs show 53 at all stages

### Step 5: Verify 100% Quality (30 min)

**Run LLM verification:**
```bash
source .env
cargo test -p docling-backend --test pdf_honest_test \
  test_pure_rust_vs_python_baseline_with_llm \
  --features pdf-ml -- --ignored --nocapture
```

**Must show:**
- ✅ LLM quality: 100% (or very close, 98%+)
- ✅ Test PASSES
- ✅ No major complaints

### Step 6: Commit and Document (30 min)

```bash
git add -A
git commit -m "# NNNN: PDF DocItems Bug FIXED - Exactly 53 DocItems (100% Match)

**Status:** PDF produces EXACTLY 53 DocItems matching Python baseline

## Changes

### Bug Fix: [describe what you fixed]

**Root cause:** [explain the bug]

**Solution:** [explain your fix]

**Files modified:**
- crates/docling-backend/src/pdf.rs: [what changed]

### Test Results

**Before:**
- DocItems: 80 (51% over-fragmented)
- Output: 7,400 chars
- Reading order: Wrong (title at #4)

**After:**
- DocItems: 53 (EXACTLY matches Python) ✅
- Output: 9,456 chars (EXACTLY matches Python) ✅
- Reading order: Correct (title first) ✅
- LLM quality: 100% ✅

## Next AI: Create PR to merge feature branch to main

**Success:** PDF parsing is now 100% correct
**Branch:** feature/manager-pdf-investigation-n2042-2310
**Action:** Create PR, merge to main, celebrate

🤖 Generated with [Claude Code](https://claude.com/claude-code)
Co-Authored-By: Claude <noreply@anthropic.com>
"

git push origin feature/manager-pdf-investigation-n2042-2310
```

### Worker Success Criteria

- ✅ PDF test produces EXACTLY 53 DocItems (not 52, not 54, not 80)
- ✅ DocItems types match: 16 text, 11 section_header, 26 list_item
- ✅ Reading order correct: Title first, body second
- ✅ No fragmentation: "Pre-Digital Era..." = 1 DocItem
- ✅ Markdown: ~9,456 chars
- ✅ LLM quality: 100%
- ✅ Test PASSES (not expected to fail)
- ✅ All logging removed (clean code)
- ✅ Committed and pushed to feature branch

---

## KEY INSIGHTS FOR NEXT SESSIONS

### 1. Source Repo Is Correct

**~/docling_debug_pdf_parsing:**
- ✅ Passes 21/21 comprehensive tests
- ✅ Produces correct DocItems for 4 PDFs (validated)
- ✅ to_docling_document_multi() function works correctly

**Confirmed by other AI working on that repo.**

### 2. Bug Is in Integration

**docling_rs integration code (pdf.rs):**
- ❌ Reading order ignores assembled.reading_order field (line 1304-1320)
- ❌ Possible extra fragmentation in how we call source functions
- ❌ Need to check text cell handling
- ❌ Need to check PageElement manipulation

### 3. Why Tests Didn't Catch This

**Source repo tests validate:**
- PageElements count (ML pipeline output)
- Intermediate stages
- ±100 tolerance for ML variance

**Source repo tests DON'T validate:**
- DocItems count (final output)
- End-to-end structure
- multi_page.pdf (not in source test corpus)

**The gap:** Tests validate ML correctness, not integration correctness

### 4. Python Baselines Vary

**ArXiv 2206.01062:**
- Very granular: 325 DocItems on page 1
- 47% items <20 chars
- 26% single-word items

**Multi_page.pdf:**
- Very cohesive: 53 DocItems total (5 pages)
- 0% items <20 chars
- Avg 174 chars/item

**Lesson:** Can't assume all Python baselines have same granularity

---

## CRITICAL FILES FOR REFERENCE

### On Feature Branch

**Directives (READ FIRST):**
1. START_HERE_FIX_PDF_NOW.txt - Main directive for worker
2. PDF_MUST_BE_EXACT_53_DOCITEMS.txt - Zero tolerance requirement
3. URGENT_PDF_FRAGMENTATION_BUG.txt - Bug description
4. CRITICAL_DOCITEMS_NOT_MARKDOWN.txt - DocItems focus

**Manager Reports:**
- MANAGER_FINAL_DIRECTIVE_FOR_USER.md - Manager's summary
- PR_CLEANUP_COMPLETE.md - PR cleanup status
- MERGE_STRATEGY_LOCAL_VS_REMOTE.md - Merge approach
- MANAGER_PDF_STATUS_N2048.md - Earlier status

**Investigation:**
- CRITICAL_QUESTION_FOR_USER.md - Manager's investigation
- PDF_STATUS_CLARIFICATION.md - Why confusion existed
- WORKER_STATUS_OFF_TRACK.md - Early worker assessment

### Test Files

**Test:** crates/docling-backend/tests/pdf_honest_test.rs
- Shows current failure (80 DocItems vs 53)
- Has LLM verification test
- Has investigation tests added by manager

**Corpus:** test-corpus/pdf/multi_page.pdf
- 5-page test document
- Python baseline: test-corpus/groundtruth/docling_v2/multi_page.json

### Code to Fix

**Primary:** crates/docling-backend/src/pdf.rs (lines 1250-1350)
- parse_bytes() function
- Text cell extraction (lines 145-348)
- ML pipeline integration (lines 1273-1296)
- Reading order calculation (lines 1304-1320)
- DocItems conversion (lines 1322-1335)

---

## VERIFICATION CHECKLIST

### For Manager: How to Check Worker Progress

**Run this test:**
```bash
cd ~/docling_rs
git checkout feature/manager-pdf-investigation-n2042-2310
source setup_env.sh
cargo test -p docling-backend --test pdf_honest_test --features pdf-ml -- --nocapture 2>&1 | grep -A 5 "Total DocItems"
```

**Expected progression:**
- Session 1: 80 DocItems (current)
- Session 2: 65 DocItems (improving)
- Session 3: 53 DocItems (SUCCESS!) ✅

**If stuck at 80 after 8 hours:**
- Review worker's approach
- Provide more specific hints
- Consider pair debugging

### For Worker: How to Verify Success

**Test 1: DocItems Count**
```bash
cargo test -p docling-backend --test pdf_honest_test show_docitems_details --features pdf-ml -- --nocapture 2>&1 | grep "Total DocItems"
# Must show: Total DocItems: 53
```

**Test 2: Structure Match**
```bash
cargo test -p docling-backend --test pdf_honest_test --features pdf-ml -- --nocapture
# Test must PASS (not fail)
# Output: 9,456 chars (or very close)
```

**Test 3: LLM Quality**
```bash
source .env
cargo test -p docling-backend --test pdf_honest_test test_pure_rust_vs_python_baseline_with_llm --features pdf-ml -- --ignored --nocapture
# Must show: Score >= 98% (ideally 100%)
```

**All three must pass!**

---

## DECISION TREES

### Manager: Worker Claims Success

**Verify independently:**
1. Run tests yourself (don't trust claims)
2. Check DocItems count (must be EXACTLY 53)
3. Check markdown length (must be ~9,456)
4. Run LLM test (must be 100%)

**If all pass:**
- ✅ Help create PR
- ✅ Document success
- ✅ Plan next work

**If any fail:**
- ❌ Worker was wrong
- ❌ Point out specific failure
- ❌ Continue debugging

### Worker: Stuck After 4 Hours

**Options:**

**Option A: Continue with current approach (if making progress)**
- If went from 80 → 70 → 60: Keep going
- Trend shows will reach 53 soon

**Option B: Try different approach (if stuck at same number)**
- Review START_HERE_FIX_PDF_NOW.txt again
- Check all 4 potential bug locations
- Add more logging

**Option C: Ask manager for help**
- Document what you tried
- Show logs/evidence
- Manager can provide hints

**Option D: Compare with Python source (last resort)**
- Look at Python docling code for this feature
- Port the exact algorithm
- Time-consuming but guaranteed to work

---

## SUCCESS METRICS

### PDF Parsing Quality

**Current:**
- 80 DocItems (WRONG)
- 7,400 chars (78% of target)
- Reading order incorrect
- Test FAILS

**Target:**
- 53 DocItems (EXACT)
- 9,456 chars (EXACT)
- Reading order correct
- Test PASSES
- LLM: 100%

### Worker Effectiveness

**Success indicators:**
- Commits show clear progress (80 → 70 → 60 → 53)
- Following directive (added logging, tested systematically)
- Bug isolated and fixed in < 8 hours
- Code is clean (logging removed)

**Failure indicators:**
- Working on wrong thing (not following START_HERE)
- No progress after multiple commits (stuck at 80)
- Debugging without logging (guessing randomly)
- Claiming success at 78% (not 100%)

---

## BRANCH STRATEGY

### Current Setup

**main:** Production-ready code
**feature/manager-pdf-investigation-n2042-2310:** PDF bug fix work

### When Worker Succeeds

**Create PR:**
```bash
git checkout feature/manager-pdf-investigation-n2042-2310
gh pr create --repo dropbox/dKNOW/docling_rs \
  --base main \
  --title "PDF Parsing: Fix DocItems Fragmentation (80→53, 100% Match)" \
  --body "## Summary

Fixes PDF parsing to produce exactly 53 DocItems matching Python baseline.

**Before:**
- 80 DocItems (51% over-fragmented)
- 7,400 chars (78% quality)
- Reading order wrong

**After:**
- 53 DocItems (EXACT match)
- 9,456 chars (100% quality)
- Reading order correct
- LLM quality: 100%

## Bug Root Cause

[Worker fills in what they found]

## Changes

- crates/docling-backend/src/pdf.rs: [describe fix]

## Testing

- ✅ pdf_honest_test passes
- ✅ 53 DocItems exactly
- ✅ LLM quality 100%

## Test Plan

\`\`\`bash
source setup_env.sh
cargo test -p docling-backend --test pdf_honest_test --features pdf-ml -- --nocapture
\`\`\`

Should show: Total DocItems: 53, Output: 9456 chars, Test: PASSED

🤖 Generated with [Claude Code](https://claude.com/claude-code)
"
```

**Merge when:**
- All tests passing
- Manager verified 100% quality
- PR approved

---

## HANDOFF INFORMATION

### Environment

**API Key:** In .env file (gitignored, write-protected)
```bash
source .env  # Loads OPENAI_API_KEY
```

**Libtorch:** In setup_env.sh
```bash
source setup_env.sh  # Sets LIBTORCH_USE_PYTORCH=1, DYLD_LIBRARY_PATH
```

**Test Corpus:** Downloaded to test-corpus/
- All format test files present
- Groundtruth baselines in test-corpus/groundtruth/docling_v2/

### Build Commands

**Build PDF ML:**
```bash
source setup_env.sh
cargo build -p docling-pdf-ml --features pytorch,opencv-preprocessing
cargo build -p docling-backend --features pdf-ml
```

**Run PDF test:**
```bash
cargo test -p docling-backend --test pdf_honest_test --features pdf-ml -- --nocapture
```

**Note:** pdf-ml is gitignored, so git operations won't track it. This is intentional.

### Communication

**Between manager sessions:**
- Git commits are the only permanent record
- Put all important info in commit messages
- Reference file paths and line numbers
- Document what you tried and what worked

**If stuck:**
- Create STATUS_STUCK.md with evidence
- Show what you tried
- Ask specific questions
- Manager will help

---

## TIMELINE & EXPECTATIONS

### Realistic Timeline

**Worker Session 1 (6-8 hours):**
- Add logging
- Find bug
- Fix bug
- Test until 53 DocItems
- LLM verify
- Commit and push

**If not done in Session 1:**

**Manager Check-in (30 min):**
- Review progress
- Provide direction
- Estimate remaining time

**Worker Session 2 (2-4 hours):**
- Continue with manager's guidance
- Complete fix
- Verify 100%
- Create PR

### Don't Rush

**Quality over speed:**
- 53 DocItems EXACTLY is required
- 0% tolerance
- Take time to get it right
- Better to take 10 hours and succeed than 6 hours and get 78%

---

## FINAL NOTES

### For Next Manager

**Your job:**
- Check worker progress periodically
- Verify quality claims
- Keep worker on track
- Provide strategic direction
- Document status for user

**Don't do worker's job:**
- Don't write the fix yourself (unless worker really stuck)
- Give hints and direction
- Let worker learn and execute

### For Next Worker

**Your job:**
- Fix the PDF bug
- Reach 100% quality
- Follow the directive
- Document your work
- Ask for help if stuck

**Remember:**
- SOURCE REPO IS CORRECT - don't debug it
- Bug is in pdf.rs integration code
- Use logging to find exact location
- Test systematically
- 0% tolerance - must be EXACTLY 53

---

## CONTACT/ESCALATION

**If things go wrong:**
1. Worker stuck >10 hours: Create STATUS_STUCK.md, call for manager
2. Manager unsure: Document situation for user
3. Critical blocker: Leave clear notes in git commit

**Communication channel:**
- Git commits (permanent)
- Status files (for complex situations)
- Directive files (for clear instructions)

---

## SUCCESS = PDF PRODUCES EXACTLY 53 DOCITEMS

**USER:** "Rust should equal Python"
**USER:** "0% tolerance! only 100% equal is allowed!"

**When you achieve this:**
- Feature branch → PR → Main
- PDF work complete
- Move to next priority
- Document lessons learned

---

**This execution plan should get PDF to 100% quality in 1-2 worker sessions.**

**Good luck!** 🎯
