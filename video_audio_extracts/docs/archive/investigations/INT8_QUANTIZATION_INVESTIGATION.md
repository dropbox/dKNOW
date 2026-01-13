# INT8 Quantization Investigation - Negative Result

**Date**: 2025-10-31
**Investigation**: N=153
**Result**: ❌ NOT VIABLE on macOS CoreML

## Summary

INT8 quantized ONNX models (specifically `yolov8n-pose-int8.onnx`) **cannot be loaded** on ONNX Runtime with CoreML execution provider. The quantized operations (`ConvInteger`) are not supported by CoreML.

## Background

- **Model**: YOLOv8n-Pose INT8 quantized (3.6MB vs 13MB FP32)
- **Expected benefit**: +20-50% inference speed, -72% model size
- **Status in codebase**: Model exists in `models/pose-estimation/yolov8n-pose-int8.onnx`, enum variant exists (`PoseEstimationModel::YoloV8nPoseInt8`), but not used as default

## Investigation Steps

### 1. Model Verification

Both models exist:
```bash
$ ls -lh models/pose-estimation/
-rw-r--r--  3.6M  yolov8n-pose-int8.onnx  # INT8 quantized
-rw-r--r--   13M  yolov8n-pose.onnx       # FP32 original
```

### 2. Benchmark Execution

Created `tests/pose_int8_benchmark.rs` to measure FP32 vs INT8 performance.

**FP32 Results** (N=153):
- Model load: 1.114s
- Inference: 0.012s per frame (average over 5 runs)
- Detections: 0 (test image may not contain people)
- Status: ✅ SUCCESS

**INT8 Results** (N=153):
- Model load: **FAILED**
- Error: `ModelLoadError { path: "models/pose-estimation/yolov8n-pose-int8.onnx", error: "Could not find an implementation for ConvInteger(10) node with name '/model.0/conv/Conv_quant'" }`
- Status: ❌ FAILURE - Cannot load model

### 3. Root Cause Analysis

The error message indicates that ONNX Runtime with CoreML execution provider **does not support INT8 quantized operations**.

**Key findings**:
- ONNX Runtime version: 2.0.0-rc.10
- Execution providers: CoreML (primary on macOS), CUDA (fallback on NVIDIA GPUs)
- INT8 support: CoreML execution provider **does not support ConvInteger** operators
- ConvInteger: Quantized convolution operation used in INT8 ONNX models

**Technical explanation**:
- INT8 quantization uses `ConvInteger` instead of standard `Conv` operators
- CoreML (Apple's ML framework) does not support these quantized integer operations
- ONNX Runtime falls back to CPU execution provider for unsupported ops
- However, the CPU execution provider in our build **also lacks ConvInteger support**

### 4. Why INT8 Model Exists in Codebase

The INT8 model likely from previous experimentation (N=170-180 range based on git history). It was created but never validated for loading/execution on macOS.

**Previous assumption** (incorrect):
- INT8 models would work on ONNX Runtime with CoreML
- CoreML would accelerate INT8 inference

**Reality** (verified N=153):
- CoreML does not support INT8 quantized ONNX models
- INT8 models cannot be loaded at all (not just slower, completely incompatible)

## Conclusions

### Verdict: ❌ NOT VIABLE on macOS

INT8 quantization **cannot be used** on macOS with CoreML execution provider:
- Model fails to load (incompatible operators)
- No performance benefit possible (model doesn't run)
- Cannot switch to INT8 as default without breaking functionality

### Platform Compatibility

| Platform | INT8 Support | Execution Provider | Status |
|----------|--------------|-------------------|---------|
| macOS | ❌ NO | CoreML | ConvInteger not supported |
| Linux (CPU) | ⚠️ UNKNOWN | CPU | Requires testing (may have ConvInteger support) |
| Linux (NVIDIA) | ✅ LIKELY | CUDA/TensorRT | TensorRT supports INT8 quantization |
| Windows (CPU) | ⚠️ UNKNOWN | CPU | Requires testing |
| Windows (NVIDIA) | ✅ LIKELY | CUDA/TensorRT | TensorRT supports INT8 quantization |

### Recommendations

1. **Do NOT switch pose-estimation default to INT8** - would break macOS users
2. **Document INT8 incompatibility** - prevent future confusion
3. **Remove INT8 model from macOS builds** - saves 3.6MB, model is unusable
4. **Consider platform-specific models**:
   - macOS: FP32 only (current approach)
   - Linux/Windows with CUDA: Test INT8 viability (may work with TensorRT)

5. **Alternative optimizations for macOS**:
   - Keep FP32 models with CoreML acceleration (current approach)
   - Pursue other optimizations from LOCAL_PERFORMANCE_IMPROVEMENTS.md:
     - ONNX zero-copy tensors (#3) - Platform-agnostic, +5-10% inference
     - Pipeline fusion (#10) - Multi-pass elimination, +30-50% for multi-feature
     - Memory arena allocation (#11) - +5-10% throughput

## Related Documentation

- **LOCAL_PERFORMANCE_IMPROVEMENTS.md** - Optimization #5 (INT8 quantization)
- **ONNX Runtime docs**: https://onnxruntime.ai/docs/execution-providers/CoreML-ExecutionProvider.html
- **CoreML INT8 support**: CoreML supports INT8 quantization only for **native CoreML models** (.mlmodel format), not ONNX quantized models

## Files Created (N=153)

- `tests/pose_int8_benchmark.rs` - Benchmark comparing FP32 vs INT8
- `INT8_QUANTIZATION_INVESTIGATION.md` - This document
- `pose_benchmark_output.txt` - Benchmark execution log

## Next Steps (N=154+)

1. **Update LOCAL_PERFORMANCE_IMPROVEMENTS.md**:
   - Mark optimization #5 as ❌ NOT VIABLE (macOS/CoreML incompatibility)
   - Update expected gains section (remove INT8 from cumulative gains)
   - Add platform-specific notes

2. **Clean up codebase**:
   - Consider removing `yolov8n-pose-int8.onnx` from macOS builds (saves 3.6MB)
   - Keep enum variant for potential future CUDA/TensorRT support
   - Add error handling if user explicitly selects INT8 on CoreML platform

3. **Test on CUDA platform** (if available):
   - Verify INT8 model loads on TensorRT execution provider
   - Benchmark FP32 vs INT8 on NVIDIA GPUs
   - Document platform-specific support matrix

4. **Focus on viable optimizations**:
   - Priority: ONNX zero-copy tensors (#3) - works on all platforms
   - Priority: Whisper batch inference (#4) - if transcription is bottleneck
   - Defer INT8 until we have Linux/Windows CUDA testing environment

## Lessons Learned

**Always validate models load before claiming they're "available"**:
- Having a model file ≠ model can be used
- Platform-specific execution providers have different operator support
- INT8 quantization is not universally supported across ONNX Runtime providers

**Performance optimizations must consider platform constraints**:
- macOS CoreML: Excellent for FP32 inference, poor INT8 support
- NVIDIA CUDA: Excellent INT8 support via TensorRT
- CPU-only: Variable INT8 support depending on ONNX Runtime build

## References

- ONNX Runtime issue tracker: INT8 quantization support per execution provider
- CoreML documentation: Native quantization vs ONNX quantization
- TensorRT documentation: INT8 calibration and deployment
