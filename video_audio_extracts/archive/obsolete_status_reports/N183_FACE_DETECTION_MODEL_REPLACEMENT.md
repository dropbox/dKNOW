# Face Detection Model Replacement - N=183

**Date:** 2025-11-10
**Status:** PARTIAL - Model replaced, tensor parsing fixed, output still empty
**Continuation Required:** Yes - need to debug filtering/postprocessing

## Work Completed

### 1. Downloaded UltraFace RFB-320 ONNX Model
- Source: https://github.com/Linzaer/Ultra-Light-Fast-Generic-Face-Detector-1MB
- File: models/onnx/version-RFB-320.onnx
- Size: 1.2MB
- Old model backed up: models/face-detection/retinaface_mnet025.onnx.backup

### 2. Updated Postprocessing Code for UltraFace Format

**Tensor Format Changes** (crates/face-detection/src/lib.rs:387-437):

| Aspect | Old (RetinaFace expected) | New (UltraFace actual) |
|--------|---------------------------|------------------------|
| Tensor name | "scores" | "confidences" (with "scores" fallback) |
| Shape | [1, N, 2] or [1, 2, N] | [1, N, 2] confirmed |
| Data layout | Unknown | [box0_bg, box0_face, box1_bg, box1_face, ...] |
| Confidence index | scores_data[i * 2 + 1] | scores_data[i * 2 + 1] ✅ |

**Key Code Changes**:
```rust
// Line 387: Accept both "confidences" and "scores"
let scores_value = outputs
    .get("confidences")
    .or_else(|| outputs.get("scores"))
    .ok_or_else(|| { ... })?;

// Line 419-420: Correct shape parsing
let num_boxes = scores_shape[1] as usize;  // Was scores_shape[2]
let _num_classes = scores_shape[2] as usize;

// Line 437: Correct confidence indexing
let face_conf = scores_data[i * 2 + 1];  // Box i, class 1 (face)
```

### 3. Verification

**Model Loading**: ✅ SUCCESS
- ONNX Runtime loads model without errors
- CoreML acceleration active (35 partitions)
- Session initializes successfully

**Tensor Shapes**: ✅ CORRECT
- Input: [1, 3, 240, 320] (as expected)
- Confidences: [1, 4420, 2] (num_detections=4420, num_classes=2)
- Boxes: [1, 4420, 4] (x1, y1, x2, y2 normalized)

**Inference**: ✅ RUNS
- No runtime errors
- Postprocessing completes without exceptions

**Output**: ❌ EMPTY
- JSON: `[]` (empty array)
- No faces detected despite lena.jpg having obvious face
- **Root cause**: Faces likely detected but filtered out

## Problem: Empty Output Despite Correct Model

The model runs successfully but outputs no faces. Possible causes:

### 1. Confidence Values Too Low
The confidence threshold is 0.5 (set in N=182). UltraFace confidences may be:
- Raw logits (need softmax?)
- Different scale than expected
- Lower than 0.5 for valid faces

**Debug needed**: Log first 10 confidence values to see actual range.

### 2. Bounding Box Coordinates Out of Range
Boxes are validated and clamped to [0, 1]. If boxes are:
- Outside this range before clamping
- Invalid after clamping (x2 <= x1 or y2 <= y1)
They'll be rejected at line 447-449.

**Debug needed**: Log box coordinates before/after clamping.

### 3. Edge Margin or Min Box Size Filtering
Current thresholds (N=182):
- edge_margin: 0.02 (reject faces within 2% of edge)
- min_box_size: 0.02 (reject boxes <2% of image)

These are reasonable but could still reject legitimate faces.

### 4. NMS (Non-Maximum Suppression)
NMS removes overlapping detections. If IoU threshold is too aggressive, it could remove all faces.

**Debug needed**: Log number of faces before/after NMS.

## Next Steps (Priority Order)

### 1. Add Debug Logging (5 minutes)

Add to crates/face-detection/src/lib.rs around line 433:

```rust
// After extracting scores/boxes
println!("DEBUG: num_boxes = {}", num_boxes);
println!("DEBUG: First 10 confidences (face class):");
for i in 0..10.min(num_boxes) {
    let face_conf = scores_data[i * 2 + 1];
    println!("  Box {}: confidence = {:.4}", i, face_conf);
}

let mut faces_before_filter = 0;
for i in 0..num_boxes {
    let face_conf = scores_data[i * 2 + 1];
    if face_conf >= config.confidence_threshold {
        faces_before_filter += 1;
    }
}
println!("DEBUG: {} faces above threshold {}", faces_before_filter, config.confidence_threshold);
```

Add before NMS (around line 475):
```rust
println!("DEBUG: {} faces before NMS", faces.len());
```

### 2. Lower Confidence Threshold Temporarily

Change to 0.1 or even 0.01 to see if ANY faces are detected:
```rust
confidence_threshold: 0.1,  // Line 100 in FaceDetectionConfig::default()
```

### 3. Check Softmax Requirement

UltraFace models may output raw logits that need softmax. Check if:
- Confidence values are in [0, 1] range
- Background + face probs sum to ~1.0

If not, may need to add softmax:
```rust
let bg_score = scores_data[i * 2];
let face_score = scores_data[i * 2 + 1];
let exp_bg = bg_score.exp();
let exp_face = face_score.exp();
let face_conf = exp_face / (exp_bg + exp_face);
```

### 4. Verify Preprocessing

Check if UltraFace expects different normalization. Current (line 352-363):
```rust
input[[0, 0, y, x]] = f32::from(pixel[2]) - 104.0; // B
input[[0, 1, y, x]] = f32::from(pixel[1]) - 117.0; // G
input[[0, 2, y, x]] = f32::from(pixel[0]) - 123.0; // R
```

This is ImageNet mean subtraction. UltraFace typically uses this, but verify.

### 5. Compare with Reference Implementation

Check https://github.com/Linzaer/Ultra-Light-Fast-Generic-Face-Detector-1MB/blob/master/detect_imgs_onnx.py
- Line 30-40: Preprocessing
- Line 50-70: Postprocessing
- Line 80-90: Confidence threshold used

## Test Command

```bash
./target/release/video-extract debug --ops face-detection test_files_faces/lena.jpg
```

**Expected Output**: At least 1 face detected (lena.jpg is standard face detection test image)

## References

- N=159: Original root cause analysis (model/code mismatch)
- N=182: Threshold improvements (confidence=0.5, edge_margin=0.02, min_box_size=0.02)
- UltraFace GitHub: https://github.com/Linzaer/Ultra-Light-Fast-Generic-Face-Detector-1MB
- UltraFace Paper: https://arxiv.org/abs/1905.00641
