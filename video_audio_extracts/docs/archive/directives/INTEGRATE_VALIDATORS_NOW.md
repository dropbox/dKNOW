# INTEGRATE OUTPUT VALIDATORS INTO ALL TESTS

**USER ORDER:** "option 1" - Integrate validators immediately

**Branch:** main
**Priority:** CRITICAL
**Current State:** Validators exist (728 lines) but only 4/363 tests use them (1.1%)

---

## THE PROBLEM

**Current tests only check:** "Did it crash?" ✅
**Current tests DON'T check:** "Is the output correct?" ❌

**Risk:** System could produce garbage outputs and 363/363 tests would still pass.

**Examples that would pass:**
- Object detection returns empty array for every image
- Transcription returns empty text for every audio file
- Face detection returns confidence=-0.5 (invalid range)
- Embeddings contain NaN or Inf
- Timestamps go backwards (non-monotonic)

---

## THE SOLUTION

Validation framework exists at `tests/common/validators.rs` (728 lines, well-designed).

**Just needs to be integrated into smoke tests.**

---

## TASK: Integrate Validators (1-2 commits)

### Step 1: Modify test_format() Helper

**File:** `tests/smoke_test_comprehensive.rs`
**Function:** `test_format()` (around line 3736)

**Current code:**
```rust
fn test_format(file: &str, operation: &str) {
    // ... run test ...
    assert!(passed, "Format {} should be supported", file);
    println!("✅ Format test passed: {} ({:.2}s)", file, elapsed.as_secs_f64());
}
```

**Add validation BEFORE the assert:**
```rust
fn test_format(file: &str, operation: &str) {
    // ... existing execution code ...

    // NEW: Validate output if test passed
    if passed {
        // Read output files from debug output directory
        let output_files = std::fs::read_dir(&output_dir)
            .ok()
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().extension().map_or(false, |ext| ext == "json"))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        // Validate each output file
        for entry in output_files {
            let path = entry.path();
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    // Determine operation name from filename (stage_00_keyframes.json → keyframes)
                    if let Some(filename) = path.file_stem() {
                        let filename_str = filename.to_string_lossy();
                        if let Some(op_name) = filename_str.strip_prefix("stage_00_").or_else(|| filename_str.strip_prefix("stage_01_")) {
                            let op = op_name.replace("_", "-");
                            let validation = validators::validate_output(&op, &json);

                            // Warnings are OK (hash=0, empty results may be valid)
                            for warning in &validation.warnings {
                                eprintln!("⚠️  {}: {}", file, warning);
                            }

                            // Errors are FATAL
                            assert!(
                                validation.valid,
                                "Output validation failed for {} ({}): {:?}",
                                file, op, validation.errors
                            );
                        }
                    }
                }
            }
        }
    }

    assert!(passed, "Format {} should be supported", file);
    println!("✅ Format test passed: {} ({:.2}s)", file, elapsed.as_secs_f64());
}
```

### Step 2: Add validators module import

**At top of smoke_test_comprehensive.rs** (around line 45):

**Add:**
```rust
mod validators;  // Import validators from tests/common/validators.rs
```

Or if common module already imported:
```rust
use crate::validators;  // Use existing common module
```

### Step 3: Verify and Test

**Run tests to see what breaks:**
```bash
VIDEO_EXTRACT_THREADS=4 cargo test --release --test smoke_test_comprehensive -- --ignored smoke_format_mp4 --test-threads=1
```

**Expected outcomes:**
- Some tests may fail (this is GOOD - finds bugs)
- Some tests show warnings (hash=0, sharpness=0.0 - expected, document)
- Most tests should pass with validation

### Step 4: Fix Any Issues

**If validators find bugs:**
- Document them in commit message
- Fix critical bugs (out-of-range values, NaN/Inf)
- Document intentional behaviors (hash=0 in fast mode)

**If validators have wrong assumptions:**
- Fix the validator (not the output)
- Example: If validator expects different JSON schema

### Step 5: Run Full Test Suite

```bash
VIDEO_EXTRACT_THREADS=4 cargo test --release --test smoke_test_comprehensive -- --ignored --test-threads=1
```

**Goal:** 363/363 tests pass WITH output validation

### Step 6: Commit

```bash
git add tests/smoke_test_comprehensive.rs
git commit -m "# 41: Integrate Output Validators Into All Smoke Tests

USER ORDER: Integrate validators to verify output correctness, not just exit codes.

## Changes

Modified test_format() helper to call validators on all outputs:
- Read JSON output files from debug_output directory
- Call validators::validate_output() for each operation
- Assert on validation errors (out-of-range, NaN, invalid structure)
- Log warnings (hash=0, empty results - may be valid)

## Test Results

Before: 363/363 tests pass (only checking exit codes)
After: [X]/363 tests pass WITH output validation

[If any tests failed, list them and explain why:]
- Test X: Failed because [validator found bug/validator wrong assumption]
- Fixed by: [code fix/validator fix]

[If any warnings found:]
- hash=0, sharpness=0.0 in keyframes: Expected (disabled in fast mode for speed)
- Empty object detection arrays: Valid (some images have no objects)

## New Lessons

[Any bugs found by validators]
[Any validator assumptions that were wrong]
[Any intentional behaviors that validators warned about]

## Validation Coverage

Operations with validators integrated:
- keyframes: ✅ [X] tests
- object-detection: ✅ [X] tests
- face-detection: ✅ [X] tests
- transcription: ✅ [X] tests
- ocr: ✅ [X] tests
- embeddings: ✅ [X] tests

Operations without validators (still checking exit code only):
- scene-detection, diarization, audio-classification, etc. (21 operations)

Coverage: ~60-100 tests now validate outputs (was 4 tests)

## Information Expiration

None.

## Next AI: Add Validators for Remaining Operations

Create validators for the 21 operations that don't have them yet.
Priority order: scene-detection, diarization, voice-activity-detection, audio-classification.

See tests/common/validators.rs for implementation patterns.
"
```

---

## CRITICAL NOTES

**This WILL expose bugs if they exist.** That's good.

**Common issues you might find:**
1. Validators expect different JSON schema than actual output (fix validator)
2. Fast mode disables features (hash=0) - document as expected
3. Some outputs are empty (valid - no faces in image) - log warning only
4. Actual bugs in output generation - fix the bug

**Don't skip validation if tests fail** - that's the whole point!

**If >10 tests fail:** Something is systematically wrong, investigate before continuing.

---

## TIME ESTIMATE

- Integration: 30 minutes (modify helper, add import)
- Testing: 30 minutes (run tests, see what breaks)
- Fixing: 30-60 minutes (fix validators or bugs)
- **Total: 1.5-2 hours (1-2 AI commits)**

---

## SUCCESS CRITERIA

- [ ] test_format() calls validators on all outputs
- [ ] Validators integrated via common module import
- [ ] All 363 smoke tests pass WITH validation enabled
- [ ] Bugs found by validators are fixed OR documented as expected
- [ ] Commit message documents validation coverage increase

**Then:** Output correctness is verified, not just exit codes.

---

## START IMMEDIATELY

This is critical for system correctness. You have 363 tests that pass but don't verify outputs.

Read this file, execute the task, report findings.
