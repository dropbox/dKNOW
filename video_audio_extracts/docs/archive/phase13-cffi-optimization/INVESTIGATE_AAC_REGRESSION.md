# INVESTIGATE: AAC Performance Regression (12.65s vs <2.0s Expected)

**Date**: 2025-10-30
**Authority**: USER directive via MANAGER
**Priority**: CRITICAL - Test failure blocking 100% pass rate

---

## THE ISSUE

**Test**: characteristic_audio_codec_aac
**Expected**: <2.0s
**Actual**: 12.65s (6.3x slower than threshold)
**Status**: FAILING with performance regression

```
thread 'characteristic_audio_codec_aac' panicked at tests/standard_test_suite.rs:895:5:
Performance regression: AAC transcription took 12.65s (expected <2.0s)
```

**This is blocking 100% test pass rate.**

---

## WORKER N=54: Find Root Cause

### Step 1: Identify the Test File

**File**: tests/standard_test_suite.rs line 887-900

```rust
#[test]
#[ignore]
fn characteristic_audio_codec_aac() {
    // Find which file this test uses
}
```

**Action**: Read test to find input file path

### Step 2: Run Test Manually

```bash
# Find the file path from test
# Run directly to see timing breakdown
./target/release/video-extract debug --ops transcription [AAC_FILE]
```

**Measure:**
- Audio extraction time
- Transcription time
- Total time

**Identify which stage is slow.**

### Step 3: Compare to Baseline

**If audio extraction is slow (>1s):**
- Issue: N=52 audio C FFI may be slower than FFmpeg spawn
- Check: Does AAC format hit slow path in C FFI?
- Profile: audio_extractor vs fast mode audio C FFI

**If transcription is slow (>10s):**
- Issue: Whisper model issue or audio format problem
- Check: What sample rate/channels does AAC produce?
- Profile: Is Whisper getting wrong audio format?

### Step 4: Fix or Adjust

**If real regression:**
- Fix the slow code path
- Ensure AAC uses fast C FFI path

**If test threshold wrong:**
- Update threshold to realistic value
- Document why (file size, audio length, etc.)

**If file changed:**
- Check if test file path was updated in N=53
- Verify new file isn't much larger

---

## Expected Findings

**Most likely cause**: N=52 audio C FFI slower for AAC format

**Why**: AAC may require extra decode steps vs WAV/MP3

**Fix options:**
1. Optimize AAC decode path in C FFI
2. Use FFmpeg spawn for AAC specifically
3. Update test threshold to realistic value

---

## Success Criteria

**After N=54:**
- ✅ Root cause identified (audio extraction vs transcription)
- ✅ Performance measured (breakdown by stage)
- ✅ Fix implemented OR threshold updated
- ✅ Test passes consistently

**Target**: 98/98 tests passing (100%)

---

## WORKER INSTRUCTIONS

1. Read test code to find AAC file path
2. Run test manually with timing
3. Identify slow stage (audio extraction or transcription)
4. Profile and fix
5. Rerun test to verify fix
6. Commit with measurements

**Estimated**: 1-2 hours (investigation + fix)

**Report findings even if unable to fix** - performance regression needs diagnosis.
