# 🔴 CRITICAL: Output Validation Framework Required
**Date**: 2025-11-01
**Priority**: ⭐⭐⭐⭐⭐⭐ URGENT - Must implement BEFORE continuing test matrix
**Issue**: Tests only check exit codes, NOT output correctness
**Risk**: Binary could produce garbage outputs and all tests would pass

---

## USER CONCERN (Valid)

**Question**: "How do I know that the outputs from the binaries are good? We need tracking and testing that the outputs are real and not changing"

**Current Problem**: Tests check "did it crash?" but NOT "are results correct?"

---

## Evidence of Gap

### Current Test Behavior ❌

**smoke_test_comprehensive.rs** (48 tests):
```rust
fn test_format(file: &str, op: &str) {
    let output = Command::new("./target/release/video-extract")
        .args(["fast", "-o", op, file])
        .output()
        .expect("Failed to execute");

    assert!(output.status.success());  // ← ONLY checks didn't crash
    // ❌ Does NOT check if output is correct
    // ❌ Does NOT check if output changed
    // ❌ Does NOT validate JSON structure
}
```

###Suspicious Output Example

**Keyframes extraction** returns:
```json
{
  "hash": 0,           ← Should be perceptual hash (uint64), not 0
  "sharpness": 0.0     ← Should be calculated float, not 0.0
}
```

**Questions**:
- Is hash=0 correct? (could be disabled in fast mode)
- Is sharpness=0.0 a bug? (should be calculated)
- Has this changed? (no way to detect regression)

**We have NO WAY to tell** if these values are correct!

---

## What's Missing

### 1. Golden Outputs ❌

**Need**: Expected outputs for each test case
```
golden_outputs/
├── keyframes/
│   ├── video_hevc_h265__keyframes.json  (expected output)
│   ├── video_hevc_h265__keyframes.sha256 (output hash)
│   └── video_hevc_h265__metadata.json (test metadata)
├── object-detection/
│   └── ...
└── transcription/
    └── ...
```

**Usage**: Compare actual output vs golden output
```rust
let actual = run_extraction("keyframes", "test.mp4");
let expected = load_golden("keyframes/test_mp4.json");
assert_outputs_match(actual, expected, tolerance);
```

---

### 2. Output Validation ❌

**Need**: Semantic checks on output structure and values

**Example for object-detection**:
```rust
fn validate_object_detection(output: &ObjectDetectionOutput) {
    // Structure validation
    assert!(output.detections.len() > 0, "Should detect some objects");

    for detection in &output.detections {
        // Semantic validation
        assert!(detection.confidence >= 0.0 && detection.confidence <= 1.0,
                "Confidence must be in [0, 1]");
        assert!(detection.bbox.x >= 0.0 && detection.bbox.x <= 1.0,
                "BBox coordinates must be normalized");
        assert!(detection.class_id < 80,  // COCO has 80 classes
                "Class ID out of range");
        assert!(!detection.class_name.is_empty(),
                "Class name should not be empty");
    }
}
```

**Example for keyframes**:
```rust
fn validate_keyframes(output: &KeyframesOutput) {
    assert!(output.keyframes.len() > 0, "Should extract some keyframes");

    for kf in &output.keyframes {
        assert!(kf.frame_number >= 0, "Frame number should be positive");
        assert!(kf.timestamp >= 0.0, "Timestamp should be positive");

        // Check for suspicious default values
        if kf.hash == 0 {
            warn!("Perceptual hash is 0 - may not be calculated");
        }
        if kf.sharpness == 0.0 {
            warn!("Sharpness is 0.0 - may not be calculated");
        }

        // Validate paths exist
        for path in kf.thumbnail_paths.values() {
            assert!(Path::new(path).exists(), "Thumbnail should exist: {}", path);
        }
    }
}
```

---

### 3. Determinism Testing ❌

**Need**: Same input → same output (run 3x, compare)

```rust
#[test]
fn test_keyframes_determinism() {
    let results: Vec<_> = (0..3)
        .map(|_| run_extraction("keyframes", "test.mp4"))
        .collect();

    // All 3 runs should produce identical output
    assert_eq!(results[0], results[1], "Run 1 vs Run 2 mismatch");
    assert_eq!(results[1], results[2], "Run 2 vs Run 3 mismatch");

    // For non-deterministic features (like timestamps), use tolerance
    assert_outputs_similar(&results[0], &results[1], tolerance=0.01);
}
```

---

### 4. Regression Detection ❌

**Need**: Detect when outputs change between versions

```rust
#[test]
fn test_no_regression_keyframes() {
    let current = run_extraction("keyframes", "test.mp4");
    let baseline = load_baseline("baselines/keyframes_test_mp4.json");

    // Compare outputs
    let diff = compare_outputs(&current, &baseline);

    assert!(
        diff.is_compatible(),
        "Output regression detected:\n{}",
        diff.summary()
    );

    // Allow additions but not removals/changes
    assert!(diff.keyframes_removed.is_empty(), "Keyframes missing");
    assert!(diff.keyframes_changed.is_empty(), "Keyframe data changed");
    // Allow new keyframes (improvements OK)
}
```

---

## Implementation Plan (URGENT)

### Phase 1: Golden Outputs Infrastructure (N=243-247, 5 commits)

**N=243: Create golden output framework**
```rust
// tests/golden_outputs/mod.rs

pub struct GoldenOutput {
    pub test_name: String,
    pub feature: String,
    pub input_file: String,
    pub expected_output: serde_json::Value,
    pub output_hash: String,  // SHA256 of canonical JSON
    pub metadata: GoldenMetadata,
}

pub struct GoldenMetadata {
    pub created_date: String,
    pub git_hash: String,
    pub binary_hash: String,
    pub notes: String,
}

impl GoldenOutput {
    pub fn save(&self, path: &Path) -> Result<()> {
        // Save expected output
        // Save metadata
        // Calculate and save hash
    }

    pub fn load(path: &Path) -> Result<Self> {
        // Load saved golden output
    }

    pub fn compare(&self, actual: &serde_json::Value) -> ComparisonResult {
        // Compare actual vs expected
        // Return detailed diff
    }
}
```

**N=244: Implement output validators**
- Create validators for each plugin type
- Structure validation + semantic validation
- Suspicious value detection (hash=0, empty results, etc.)

**N=245: Generate golden outputs for existing tests**
- Run all 48 smoke tests
- Save outputs as golden baselines
- Store in `golden_outputs/{feature}/{test_name}.json`

**N=246: Add regression tests**
- Compare current outputs vs golden outputs
- Fail if outputs changed unexpectedly
- Allow tolerance for non-deterministic values

**N=247: Implement determinism tests**
- Run same test 3x
- Compare outputs
- Detect non-deterministic behavior

---

### Phase 2: Integrate with Test Matrix (N=248-250)

**N=248: Add output validation to Wikimedia tests**
- Validate each downloaded file's output
- Save as golden output
- Detect suspicious values

**N=249: Create output comparison reports**
- Compare all test outputs vs golden
- Generate diff reports
- Identify regressions

**N=250: Cleanup + smoke test integration**
- Add representative Wikimedia tests to smoke suite
- Verify all outputs are validated
- Document validation framework

---

## Validation Rules by Plugin

### Keyframes
```rust
- keyframes.len() > 0 (should extract some)
- frame_number sequential
- timestamp monotonic increasing
- hash != 0 (unless disabled in fast mode, check)
- sharpness >= 0.0 (if enabled)
- thumbnail paths exist on disk
```

### Object Detection
```rust
- confidence ∈ [0, 1]
- bbox coordinates ∈ [0, 1] (normalized)
- class_id < 80 (COCO classes)
- class_name not empty
- bbox width/height > 0
```

### Face Detection
```rust
- confidence ∈ [0, 1]
- bbox coordinates ∈ [0, 1]
- landmarks.len() == 5 (RetinaFace)
- landmark coordinates ∈ [0, 1]
```

### Transcription
```rust
- text not empty (unless silent audio)
- language detected (2-letter code)
- segments.len() > 0
- timestamps monotonic
- confidence ∈ [0, 1] per segment
```

### OCR
```rust
- text_regions.len() >= 0 (may be 0 if no text)
- bbox coordinates ∈ [0, 1]
- text not empty per region
- confidence ∈ [0, 1]
```

### Embeddings (Vision/Audio/Text)
```rust
- embedding.len() == expected_dims (512 for CLIP, etc.)
- values ∈ [-inf, inf] (no NaN, no Inf)
- L2 norm ≈ 1.0 (if normalized)
```

---

## Example: Complete Test with Output Validation

```rust
#[test]
#[ignore]
fn test_keyframes_with_validation() {
    // Run extraction
    let output = Command::new("./target/release/video-extract")
        .args(["debug", "--ops", "keyframes", "test.mp4"])
        .output()
        .expect("Failed to execute");

    // Check 1: Did it crash?
    assert!(output.status.success(), "Should not crash");

    // Check 2: Parse output
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: KeyframesOutput = serde_json::from_str(&stdout)
        .expect("Should produce valid JSON");

    // Check 3: Validate structure
    validate_keyframes_structure(&result);

    // Check 4: Validate semantics
    validate_keyframes_semantics(&result);

    // Check 5: Compare vs golden output
    let golden = GoldenOutput::load("golden_outputs/keyframes/test_mp4.json")
        .expect("Golden output should exist");
    let comparison = golden.compare(&serde_json::to_value(&result).unwrap());
    assert!(comparison.is_compatible(), "Output regression: {}", comparison.diff());

    // Check 6: Determinism (run 2 more times)
    let result2 = run_and_parse("keyframes", "test.mp4");
    let result3 = run_and_parse("keyframes", "test.mp4");
    assert_eq!(result, result2, "Non-deterministic output detected");
    assert_eq!(result2, result3, "Non-deterministic output detected");
}
```

---

## Severity Assessment

**Current Risk**: 🔴 **HIGH**

**Scenario**: Binary could be producing incorrect results and all 48 tests would pass

**Examples of undetected issues**:
- Object detection returning random bboxes (but JSON is valid)
- Transcription returning empty text (but doesn't crash)
- Embeddings returning zeros (but correct dimension)
- Keyframes with hash=0, sharpness=0.0 (suspicious but unchecked)

**Impact**:
- ❌ Can't trust test suite (only checks "didn't crash")
- ❌ Can't detect regressions (outputs could change silently)
- ❌ Can't validate optimizations (results might be wrong)
- ❌ Can't ensure correctness (no ground truth)

---

## Immediate Action Required

**BEFORE continuing test matrix downloads**, implement output validation:

### Priority 1: Output Validation Framework (N=243-247, 5 commits) ← DO THIS FIRST

**N=243**: Golden output infrastructure
**N=244**: Output validators (structure + semantic checks)
**N=245**: Generate golden outputs for 48 existing tests
**N=246**: Add regression tests
**N=247**: Add determinism tests

**Result**: Every test validates outputs, not just exit codes

### Priority 2: Then Resume Test Matrix (N=248+)

**N=248+**: Download Wikimedia files WITH output validation
- Each downloaded file gets golden output saved
- Validate outputs make sense
- Detect suspicious values (hash=0, empty results)

---

## Recommendation

**STOP** test matrix downloads temporarily

**START** output validation framework (N=243-247, 5 commits, ~1-2 days)

**THEN** resume test matrix WITH validation

**Rationale**: Downloading 3,000 test files WITHOUT output validation is risky
- Can't verify results are correct
- Can't detect regressions
- Can't trust the test suite

**Better**: Implement validation framework FIRST, then download with validation enabled

---

## Documents Created

**CRITICAL_OUTPUT_VALIDATION_REQUIRED.md** - This document (for worker to read at N=243)

**Next Worker Directive**: PAUSE test matrix, implement output validation framework, THEN continue downloads with validation

This is exactly what TEST_EXPANSION_BEFORE_OPTIMIZATION.md was about - we need baseline/golden outputs!
