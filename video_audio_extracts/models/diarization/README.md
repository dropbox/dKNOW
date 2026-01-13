# Speaker Embedding Model for Diarization

## Current Model: WeSpeaker ResNet34 ✅

The diarization module uses **WeSpeaker ResNet34** for speaker embeddings.

### Download

If `speaker_embedding.onnx` is not present:

```bash
cd models/diarization
wget -O speaker_embedding.onnx \
  "https://huggingface.co/onnx-community/wespeaker-voxceleb-resnet34-LM/resolve/main/onnx/model.onnx"
```

### Specifications

- **Architecture**: ResNet34
- **Size**: 26.5 MB (full precision float32)
- **Input**: `[batch, time_frames, 80]` - 80-dim mel-filterbank features
- **Output**: `[batch, 256]` - 256-dim speaker embeddings
- **Training**: VoxCeleb dataset
- **Source**: [onnx-community/wespeaker-voxceleb-resnet34-LM](https://huggingface.co/onnx-community/wespeaker-voxceleb-resnet34-LM)

### Preprocessing

The diarization module automatically computes mel-filterbank features in pure Rust:
- Sample rate: 16kHz
- Frame length: 25ms (400 samples)
- Frame shift: 10ms (160 samples)
- Mel bins: 80
- Window: Hamming
- FFT: rustfft (no Python dependencies)

### Performance

- Embedding extraction: ~10ms per speech segment (CPU)
- No GPU required for embeddings (CPU-only ONNX inference)
- Total pipeline: VAD → Mel features → ONNX inference → Clustering

## Alternative Models (Not Tested)

### Option 1: SpeechBrain ECAPA-TDNN
- **Source**: https://huggingface.co/speechbrain/spkrec-ecapa-voxceleb
- **Dimensions**: 192
- **Note**: STFT export issues to ONNX (see EXPORT_NOTES.md)

### Option 2: Resemblyzer
- **Source**: https://github.com/resemble-ai/Resemblyzer
- **Dimensions**: 256
- **Note**: Simpler architecture, may export to ONNX more reliably

### Option 3: WeSpeaker CAM++
- **Source**: ModelScope/3D-Speaker
- **Note**: Complex architecture, export stalled (see EXPORT_NOTES.md)

## Status

✅ **Production Ready** (as of commit #43)
- Real ONNX model integrated
- Pure Rust feature extraction
- All tests passing
