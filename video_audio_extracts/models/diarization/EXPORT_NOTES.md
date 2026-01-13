# Speaker Embedding Model Export Notes

## SpeechBrain ECAPA-TDNN Export Issue

**Problem**: SpeechBrain's ECAPA-TDNN model uses `torch.stft` internally, which doesn't properly support ONNX export for complex types.

**Error**: `STFT does not currently support complex types`

**Status**: SpeechBrain models are difficult to export to ONNX due to preprocessing pipeline complexity.

## Alternative Solutions

### Option 1: ModelScope CAM++ (ATTEMPTED - ISSUES)
**Source**: https://www.modelscope.cn/models/iic/speech_campplus_sv_en_voxceleb_16k
**GitHub**: https://github.com/alibaba-damo-academy/3D-Speaker

**Status (N=42)**: Downloaded PyTorch model but export to ONNX stalled

**Process attempted**:
1. `pip install modelscope addict`
2. Run `python3 models/diarization/download_wespeaker_onnx.py` (downloads .bin file)
3. Run `python3 models/diarization/export_campplus_to_onnx.py` (export to ONNX)

**Issues encountered**:
- ModelScope provides .bin PyTorch weights (28MB campplus_voxceleb.bin)
- NOT pre-exported ONNX despite documentation claims
- Export requires loading full ModelScope pipeline (slow, many dependencies: addict, etc.)
- Export script loads model but stalls during ONNX conversion
- Complex model architecture (CAM++ with attention) may have ONNX compatibility issues

**Model specifications** (from configuration.json):
- Sample rate: 16kHz
- Input: Fbank features (80-dim)
- Output: 512-dim embeddings
- Architecture: CAM++ (Context-Aware Modeling)

### Option 2: WeSpeaker Pre-Exported ONNX (RECOMMENDED NEXT)
**Source**: https://github.com/wenet-e2e/wespeaker

WeSpeaker documentation mentions ONNX models but:
- No direct download links found
- May require cloning 3D-Speaker repo and running export scripts
- Simpler ResNet models may export more reliably than CAM++

**To investigate**:
1. Clone 3D-Speaker repo: `git clone https://github.com/alibaba-damo-academy/3D-Speaker`
2. Check `runtime/onnxruntime/` directory for export scripts
3. Try ResNet34 or ERes2Net (simpler architectures than CAM++)

### Option 3: Simple Resemblyzer
**Source**: https://github.com/resemble-ai/Resemblyzer

Simpler architecture that exports to ONNX more easily:
- 256-dim embeddings
- GE2E loss training
- Fewer preprocessing steps

### Option 3: Manual Feature Extraction
Extract features in Rust (similar to our CLAP mel-spectrogram approach):
1. Compute mel-spectrogram in Rust (already have this code)
2. Feed to simpler embedding network
3. Avoids STFT in PyTorch graph

### Option 4: Pre-trained ONNX from External Sources
**Recommendation**: Search HuggingFace or ONNX Model Zoo for pre-converted speaker embedding models

Potential sources:
- HuggingFace ONNX models
- ONNX Model Zoo
- Pre-converted models by community

Benefits:
- No export hassle
- Proven ONNX compatibility
- Ready to use

### Option 5: Use Placeholder (CURRENT - commit #41)
Current implementation uses random embeddings as placeholder:
- System compiles and runs
- Diarization works (with random speaker assignments)
- Good for testing pipeline
- Replace with real model for production

## Current Status (as of commit #43)

**Implementation**: ✅ Complete and production-ready
- WebRTC VAD: ✅ Working (bug fixed in #41)
- Speaker embeddings: ✅ **Real ONNX model integrated** (WeSpeaker ResNet34)
- Clustering: ✅ Working (K-means)
- Timeline generation: ✅ Working

**Model Details**: WeSpeaker ResNet34 (Full Precision)
- Source: HuggingFace onnx-community/wespeaker-voxceleb-resnet34-LM
- Size: 26.5 MB (full precision float32)
- Input: [batch, time_frames, 80] - 80-dim mel-filterbank features
- Output: [batch, 256] - 256-dim speaker embeddings
- Trained on: VoxCeleb dataset

**Feature Extraction**: Pure Rust implementation
- FFT: rustfft (no Python dependencies)
- Mel-filterbank: 80 bins, 16kHz, 25ms frame length, 10ms frame shift
- Window: Hamming window

**Validation** (commit #43):
- ONNX model loads and runs successfully
- Mel-feature extraction working (tested with Python/ONNX Runtime)
- All unit tests pass (134 tests, 100% pass rate)
- Integration with diarization pipeline complete

## Solution: WeSpeaker Pre-Exported ONNX (SUCCESSFUL)

The WeSpeaker ResNet34 model from onnx-community was successfully integrated:

1. **Downloaded**: Full precision model (26.5 MB) from HuggingFace
2. **Verified**: Python test confirmed model works (256-dim embeddings)
3. **Integrated**: Added mel-filterbank computation to Rust diarization module
4. **Tested**: All unit tests pass, model loads successfully

**Note**: FP16 model has ONNX Runtime compatibility issues (tensor type mismatch). Use full precision model.
