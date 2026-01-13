# BUG: Face Detection False Positives

**Severity:** HIGH
**Status:** CONFIRMED by manual inspection
**Date:** 2025-11-05
**Reporter:** MANAGER (manual review of AI audit)

---

## Summary

Face detection produces massive false positives on video codec test footage containing zero actual faces.

**Test:** `smoke_format_mp4_face_detection`
**Video:** `test_edge_cases/video_hevc_h265_modern_codec__compatibility.mp4`
**Expected:** 0 faces (video shows compression artifacts/glitches, not people)
**Actual:** 70 faces detected

**False Positive Rate:** 100% (70 false positives, 0 true positives)

---

## Evidence

### Test Video Content

Manual inspection of extracted keyframe:
- Video shows horizontal streaks/lines on black background
- Blue horizontal bands visible
- Compression artifact/glitch pattern test footage
- **NO human faces present**

Keyframe: `/tmp/video-extract/keyframes/video_hevc_h265_modern_codec__compatibility/keyframes/*_00000000_640x480.jpg`

### Detection Output

File: `test_results/latest/outputs/smoke_format_keyframes;face-detection/stage_00_face_detection.json`

**Pattern analysis:**
- 70 faces detected total
- 22 faces at top edge (y1 = 0.0, 31%)
- 10 faces with perfect confidence (1.0)
- Average bbox size: 1.5% width × 5.5% height (tiny boxes)
- Boxes cluster along top edge and left edge

**Sample false positive:**
```json
{
  "bbox": {"x1": 0.0, "x2": 0.021, "y1": 0.0, "y2": 0.050},
  "confidence": 1.0,
  "landmarks": null
}
```

Position: Top-left corner, 2.1% × 5.0% of image
Confidence: 1.0 (perfect)
Content: Horizontal line artifact (not a face)

---

## Root Cause Analysis

**Hypothesis:** Face detection model (likely ONNX YuNet or similar) is triggering on:
1. Horizontal line patterns resembling facial features
2. Image edges/borders as false face boundaries
3. Compression artifacts with regular spacing

**Model behavior:**
- High confidence on edge-aligned detections (suspicious)
- Tiny bounding boxes (1-6% of image dimensions)
- Clustered patterns suggesting systematic false triggering

---

## Impact

### Tests Affected

All face detection tests using this video:
- `smoke_format_mp4_face_detection` (16 tests - all MP4 variants)
- `smoke_format_keyframes;face-detection`
- Others with same test video

**Estimated scope:** 20-30 tests with false positives

### User Impact

**CRITICAL for production:**
- Users processing videos with compression artifacts, lines, borders, UI elements will get massive false positives
- Confidence scores unreliable (1.0 confidence on non-faces)
- Makes face detection feature unusable for real-world content

**Example real-world scenarios:**
- Screen recordings with UI borders → false faces
- Videos with subtitle bars → false faces
- Glitch effects, transitions → false faces
- Broadcast graphics, score overlays → false faces

---

## Reproduction

```bash
# Extract faces from test video
./target/release/video-extract debug --ops keyframes;face-detection \
  --output-dir /tmp/face_bug \
  test_edge_cases/video_hevc_h265_modern_codec__compatibility.mp4

# View output
cat /tmp/face_bug/stage_01_face_detection.json | python3 -m json.tool | less

# Expected: 0 faces
# Actual: 70 faces
```

---

## Recommended Fixes

### Short-term (Immediate)

1. **Adjust confidence threshold:**
   - Current: Reporting all detections
   - Proposed: Filter detections with confidence < 0.7
   - Impact: Reduces false positives, may miss some real faces

2. **Add edge filter:**
   - Reject detections at image borders (x=0, y=0, x=1, y=1)
   - Prevents edge artifacts from being detected as faces

3. **Add size filter:**
   - Reject tiny boxes (< 2% of image dimensions)
   - Prevents artifact patterns from triggering detection

### Medium-term (1-2 weeks)

4. **Evaluate different model:**
   - Current model may be too sensitive
   - Test alternatives: RetinaFace, SCRFD, MediaPipe
   - Benchmark false positive rate on diverse test set

5. **Add NMS (Non-Maximum Suppression):**
   - May already be implemented, verify settings
   - Aggressive NMS could reduce clustering of false positives

### Long-term (1-2 months)

6. **Two-stage verification:**
   - Stage 1: Current detector (high recall)
   - Stage 2: Verification model (high precision)
   - Only report faces that pass both stages

7. **Add test media with actual faces:**
   - Current test media doesn't contain faces
   - Need positive examples to validate true positive rate

---

## Testing After Fix

**Success criteria:**
- Test video (0 actual faces) → 0-2 detected faces (allow small margin)
- Videos with real faces → maintain current detection rate
- Confidence scores more meaningful (1.0 only for clear faces)

**Regression tests:**
- Create baseline of expected face counts for test videos
- Alert if detection count changes by >10%

---

## Related Issues

- Empty outputs for pose estimation (expected - no people)
- Empty outputs for OCR (needs verification)
- All validators: need to add false positive detection logic

---

**Priority:** HIGH - Blocks production use of face detection feature
**Assigned:** Next worker
**Estimated fix time:** 2-4 AI commits (~30-60 minutes)
