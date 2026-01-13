# Test System Capabilities - What Tests Actually Validate

**Date**: 2025-10-30
**User Questions**:
1. "does the test system report performance?"
2. "exact match of values extracted?"

---

## Current Test Capabilities

### ✅ What Tests DO Check:

**1. Success/Failure** (All 98 tests)
```rust
assert!(result.passed, "Test failed: {:?}", result.error);
```
- Checks if binary exits with code 0
- Captures stderr if failed
- **Validation**: Process succeeds without errors

**2. Performance Timing** (Some tests)
```rust
assert!(
    result.duration_secs < 2.0,
    "Performance regression: took {:.2}s (expected <2.0s)",
    result.duration_secs
);
```
- Measures wall-clock time
- Checks against threshold
- **Validation**: Performance within expected range

**3. Output Exists** (Implicit)
```rust
// Tests use "debug" mode which saves to debug_output/
// If operations succeed, outputs are created
```
- **Validation**: Files created in debug_output/

---

### ❌ What Tests DON'T Check:

**1. Exact Output Values**
- No JSON comparison
- No transcription text validation
- No detection bounding box verification
- No embedding vector validation

**Example - NOT validated:**
```rust
// Test runs transcription but doesn't check:
// - Transcript text matches expected
// - Word timestamps are correct
// - Language detection is accurate
```

**2. Output Quality**
- No JPEG quality checks
- No detection accuracy validation
- No transcription WER (Word Error Rate)
- No embedding similarity checks

**3. Output Format**
- No JSON schema validation
- No field presence checks
- No data type verification

---

## Test Philosophy

**Current approach**: "Smoke tests"
- Verify operations complete successfully
- Check performance isn't regressed
- Catch crashes and obvious failures

**What's missing**: "Correctness tests"
- Verify outputs match expected values
- Check detection accuracy
- Validate transcription quality
- Confirm embedding vectors

---

## Example: What A Transcription Test Checks

**Current (lines 891-900):**
```rust
fn characteristic_audio_codec_aac() {
    let result = run_video_extract("transcription", &file);

    // ✅ Checks: Process succeeded
    assert!(result.passed, "AAC codec test failed");

    // ✅ Checks: Performance threshold
    assert!(result.duration_secs < 2.0, "Too slow");

    // ❌ Doesn't check: Transcript text
    // ❌ Doesn't check: Word timestamps
    // ❌ Doesn't check: Language detected
}
```

**What it validates:**
- Binary doesn't crash ✅
- Completes in <2s ✅
- Produces some output ✅

**What it doesn't validate:**
- Transcript says "Hello world" (or whatever audio contains)
- Timestamps are [0.0, 0.5, 1.0, ...] (correct timing)
- Language is "en" (correct detection)

---

## Example: What An Object Detection Test Checks

**Current (lines 76-78):**
```rust
fn format_mp4_quick_pipeline() {
    let result = run_video_extract("keyframes,object-detection", &file);
    assert!(result.passed, "Test failed");
    println!("✅ MP4: {:.2}s", result.duration_secs);
}
```

**What it validates:**
- Pipeline completes ✅
- No crashes ✅

**What it doesn't validate:**
- Detected "person" at [0.1, 0.2, 0.3, 0.4] (correct bbox)
- Confidence is 0.85 (reasonable)
- No false positives (detecting objects that aren't there)

---

## Recommendation: Add Correctness Tests

**Minimal correctness validation:**

```rust
fn test_transcription_correctness() {
    let result = run_video_extract("transcription", &test_audio);

    // Existing: Check succeeded
    assert!(result.passed);

    // NEW: Check output content
    let transcript = std::fs::read_to_string("debug_output/stage_00_transcription.json")?;
    let json: serde_json::Value = serde_json::from_str(&transcript)?;

    // Validate expected words are in transcript
    let text = json["segments"][0]["text"].as_str().unwrap();
    assert!(text.contains("expected word"), "Transcript doesn't contain expected content");

    // Validate timestamp format
    let start = json["segments"][0]["start"].as_f64().unwrap();
    assert!(start >= 0.0 && start < 60.0, "Invalid timestamp");
}
```

**Benefits:**
- Catch regressions in output quality
- Verify operations produce correct results
- Validate data formats and schemas

---

## Current Test Quality Assessment

**Strengths:**
- ✅ 98 tests covering diverse formats and operations
- ✅ Performance regression detection
- ✅ Crash detection
- ✅ Clean, maintainable code

**Weaknesses:**
- ❌ No output correctness validation
- ❌ No accuracy/quality checks
- ❌ No schema validation
- ❌ Could pass even if outputs are garbage (as long as they're created)

---

## Answer to User Questions

**Q1: "does the test system report performance?"**

**A: YES** - Tests measure `duration_secs` and some check thresholds
- Example: characteristic_audio_codec_aac checks <2.0s
- Tests print timing: "✅ MP4 (34MB): 3.45s"
- But not all tests have performance thresholds

**Q2: "exact match of values extracted?"**

**A: NO** - Tests only check success/failure, not output content
- Don't validate transcription text
- Don't check detection bounding boxes
- Don't verify embedding vectors
- Don't compare against golden outputs

**Tests are "smoke tests" not "correctness tests"**

---

## Recommendation for Worker

**N=54+**: After fixing AAC regression, consider adding correctness tests:
1. Golden output files for key operations
2. JSON schema validation
3. Content checks (expected words in transcripts, expected detections)
4. Quality thresholds (minimum confidence, maximum WER)

**Estimated effort**: 2-3 commits to add basic correctness validation

**Value**: Catch quality regressions, not just crash regressions
