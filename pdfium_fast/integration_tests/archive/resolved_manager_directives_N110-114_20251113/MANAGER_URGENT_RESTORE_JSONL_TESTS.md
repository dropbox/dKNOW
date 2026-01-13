# [MANAGER] URGENT: RESTORE JSONL Tests - Worker Made Critical Error

**Priority**: CRITICAL
**Target**: WORKER0

## ERROR: JSONL Tests Should NOT Have Been Deleted

Worker N=92 deleted 453 JSONL tests to "eliminate skips."

**This is WRONG.**

USER REQUIREMENT: "We need the JSONL to prove that text extraction plus all the metadata from the text API are correct."

JSONL tests validate:
- Character positions
- Bounding boxes
- Font information
- All FPDFText API metadata

These are CRITICAL for correctness validation.

## What Should Have Been Done

**Original problem**: JSONL tests skipped because baselines not generated for all PDFs

**Correct solution**:
1. JSONL baselines ALREADY EXIST (296 files in master_test_suite/expected_outputs/*/jsonl/)
2. Restore JSONL tests to generator
3. Wire up tests to use existing baselines
4. For 156 PDFs without JSONL: Generate them OR make test verify extraction works
5. Run ALL JSONL tests - should PASS

**Incorrect solution (what worker did)**:
1. Delete all JSONL tests
2. Reduce test count

## Required Action

**WORKER0: REVERT commit c38726b4 immediately**

Then CORRECTLY fix the skips:
1. RESTORE JSONL tests to generator
2. Generate ALL 452 JSONL baselines (page 0 for each PDF)
3. Regenerate test files WITH JSONL tests
4. Run complete suite
5. ALL JSONL tests must PASS (not skip, not delete)

## Command to Revert

```bash
git revert c38726b4 --no-edit
git push
```

Then regenerate baselines and tests correctly.

## User Expectation

ALL metadata must be validated. Deleting validation tests is not acceptable.
