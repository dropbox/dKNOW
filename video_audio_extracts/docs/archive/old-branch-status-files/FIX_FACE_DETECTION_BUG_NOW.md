# FIX FACE DETECTION BUG - IMMEDIATE

**Priority:** CRITICAL
**Bug:** Face detection has 70 false positives on video with 0 faces
**Status:** Confirmed by MANAGER manual inspection

---

## The Bug

**File:** `crates/face-detection/src/lib.rs`
**Issue:** 70 faces detected on compression artifact video with 0 actual faces

**Current behavior:**
- confidence_threshold: 0.7 (default, line 96)
- Detects tiny boxes (1.5% × 5.5% average) at image edges
- Perfect confidence (1.0) on compression artifacts
- 100% false positive rate on test video

---

## Test Images Available

**Location:** `test_files_wikimedia/jpg/face-detection/`
**Count:** 175 JPG images with actual human faces

Use these to verify:
1. True positive rate (detect real faces)
2. False positive rate after fix (reject artifacts)

---

## Recommended Fixes (Implement ALL)

### 1. Increase Confidence Threshold
```rust
// Line 96 in crates/face-detection/src/lib.rs
confidence_threshold: 0.85,  // was 0.7, increase to reduce false positives
```

### 2. Add Minimum Box Size Filter
```rust
// Add to FaceDetectionConfig (line 82)
pub min_box_size: f32,  // Minimum box size as fraction of image (e.g., 0.02 = 2%)

// In default() (line 96)
min_box_size: 0.02,  // Reject boxes smaller than 2% of image dimensions

// In post processing (after line 278)
faces.retain(|face| {
    let width = face.bbox.x2 - face.bbox.x1;
    let height = face.bbox.y2 - face.bbox.y1;
    width >= config.min_box_size && height >= config.min_box_size
});
```

### 3. Add Edge Detection Filter
```rust
// Add to FaceDetectionConfig
pub edge_margin: f32,  // Reject detections within this margin of edges (e.g., 0.05 = 5%)

// In default()
edge_margin: 0.05,  // Reject faces touching image borders

// In post processing
faces.retain(|face| {
    face.bbox.x1 > config.edge_margin &&
    face.bbox.y1 > config.edge_margin &&
    face.bbox.x2 < (1.0 - config.edge_margin) &&
    face.bbox.y2 < (1.0 - config.edge_margin)
});
```

---

## Testing After Fix

### Test 1: Artifact Video (Should Detect 0 Faces)
```bash
./target/release/video-extract debug --ops keyframes;face-detection \
  --output-dir /tmp/test1 \
  test_edge_cases/video_hevc_h265_modern_codec__compatibility.mp4

# Check result
cat /tmp/test1/stage_01_face_detection.json | python3 -c "import sys,json; print(f'{len(json.load(sys.stdin))} faces')"

# Expected: 0-2 faces (down from 70)
```

### Test 2: Real Face Images (Should Still Detect Faces)
```bash
./target/release/video-extract debug --ops face-detection \
  --output-dir /tmp/test2 \
  "test_files_wikimedia/jpg/face-detection/01_'Touch'_by_Gavin_Evans,_2009.jpg"

# Check result
cat /tmp/test2/stage_00_face_detection.json | python3 -c "import sys,json; print(f'{len(json.load(sys.stdin))} faces')"

# Expected: 1+ faces (should detect actual faces)
```

### Test 3: Run Full Test Suite
```bash
VIDEO_EXTRACT_THREADS=4 cargo test --release --test smoke_test_comprehensive smoke_format_mp4_face_detection -- --ignored --nocapture

# Verify: Tests still pass, but face count reduced
```

---

## Success Criteria

- [ ] Artifact video: 0-2 faces detected (down from 70)
- [ ] Real face images: Still detect faces (maintain true positive rate)
- [ ] All 363 tests still pass
- [ ] Update MASTER_AUDIT_CHECKLIST.csv with new face counts
- [ ] Document fix in bug report

---

## Implementation Steps

1. Read `crates/face-detection/src/lib.rs`
2. Implement 3 fixes above
3. Rebuild: `cargo build --release`
4. Run Test 1: Verify artifact video → 0-2 faces
5. Run Test 2: Verify real faces still detected
6. Run Test 3: Verify all tests pass
7. Update MASTER_AUDIT_CHECKLIST.csv
8. Commit with test results

---

**Estimated:** 2-3 AI commits (~30 minutes)
**Priority:** Fix this BEFORE implementing regression tests or validators
