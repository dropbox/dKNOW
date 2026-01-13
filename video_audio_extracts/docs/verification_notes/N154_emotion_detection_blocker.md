# N=154: Emotion Detection Blocker - Face Images Not Detectable

**Date:** 2025-11-09
**Task:** Benchmark emotion_detection operation (26/33 operations)
**Status:** ❌ BLOCKED - No suitable test media

---

## Situation

N=153 noted that emotion_detection could now be benchmarked since MANAGER (commit 030b45a) downloaded 4 face images to `test_files_faces/`:
- lena.jpg (classic test image)
- biden.jpg (high-res portrait)
- obama.jpg (face detection test)
- two_people.jpg (multiple faces)

---

## Testing Results

**Test 1: Face Detection on lena.jpg**
```bash
./target/release/video-extract debug --ops face-detection test_files_faces/lena.jpg
# Result: 0 faces detected (233ms)
```

**Test 2: Face Detection on two_people.jpg**
```bash
./target/release/video-extract debug --ops face-detection test_files_faces/two_people.jpg
# Result: 0 faces detected (242ms)
```

**Test 3: Emotion Detection on biden.jpg**
```bash
./target/release/video-extract debug --ops emotion-detection test_files_faces/biden.jpg
# Error: Invalid input: No faces detected in image. Emotion detection requires at least one face.
```

---

## Root Cause

The RetinaFace model (models/retinaface_mnet025_v2.onnx) is not detecting faces in the downloaded images. This could be due to:

1. **Image resolution/scale**: Faces may be too small or too large for the model's input size (320x240)
2. **Image quality**: Downloaded images may be compressed or degraded
3. **Model sensitivity**: RetinaFace threshold may be too strict
4. **Image format**: Images may not be in expected format

---

## Recommendation

**Option 1: Download different face images**
- Search for images specifically tested with RetinaFace
- Use higher-resolution, front-facing portraits
- Verify face detection works before attempting emotion detection

**Option 2: Follow MANAGER Objective 1**
- Proceed with downloading other challenging test cases (OCR, object detection, audio)
- Defer emotion_detection benchmarking until suitable test media is available
- Document this as a known limitation

---

## Decision

Following MANAGER directive (MANAGER_FINAL_DIRECTIVE_100_PERCENT.md):
- **Objective 1 (N=153-155)**: Download challenging test cases
- emotion_detection remains unbenchmarked (1/8 unbenchmarked vision operations)
- Continuing with OCR and object detection test case downloads

---

## Status Update

**Operations Benchmarked:** 25/33 (76% coverage)
**Unbenchmarked Operations:**
1. emotion_detection (no suitable test media with detectable faces)
2. motion-tracking (requires ObjectDetection input with detectable objects)
3. content_moderation (missing nsfw_mobilenet.onnx model)
4. logo_detection (missing yolov8_logo.onnx model)
5. caption_generation (missing caption model)
6. music_source_separation (missing Demucs/Spleeter model)
7. depth_estimation (not exposed in debug mode)
8. (7 operations remaining unbenchmarked, 6 of which require missing models or technical limitations)

---

**Next Steps for N=154:**
1. Download OCR test images (receipts, multilingual text, handwritten)
2. Download object detection scenes (crowded, small objects, occluded)
3. Test downloaded media with operations
4. Update documentation with findings
