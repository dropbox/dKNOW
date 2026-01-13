# Music Source Separation Models

This directory contains ONNX models for music source separation (isolating vocals, drums, bass, and other instruments).

## Required Files

The music source separation plugin requires two files:

1. **`demucs.onnx`** (or `spleeter.onnx`) - ONNX model for source separation
2. **`stems.txt`** - List of stem names (one per line), matching model output order

## Model Specification

### Input
- **Format**: Float32 tensor `[batch, channels, samples]`
- **Sample rate**: 44,100 Hz (standard for music)
- **Channels**: 1 (mono) or 2 (stereo), depending on model
- **Samples**: Variable length (model-dependent, typically 4-8 seconds per chunk)

### Output
- **Format**: Float32 tensor `[batch, num_stems, channels, samples]`
- **Stems**: 4 stems (Demucs), 2/4/5 stems (Spleeter configurations)
- **Order**: Must match `stems.txt` line order

### Example stems.txt (Demucs 4-stem)
```
drums
bass
other
vocals
```

### Example stems.txt (Spleeter 2-stem)
```
vocals
accompaniment
```

---

## Available Models

### 1. Demucs (Recommended)

**Demucs v4 (Hybrid Transformer Demucs)** - State-of-the-art source separation (9.0+ dB SDR)

- **Repository**: https://github.com/facebookresearch/demucs
- **Architecture**: Hybrid time-frequency transformer
- **Stems**: drums, bass, vocals, other (4 stems)
- **Performance**: 9.00-9.20 dB SDR (Signal-to-Distortion Ratio)
- **License**: MIT
- **Model sizes**:
  - `htdemucs`: ~800MB (default, best quality)
  - `htdemucs_ft`: ~800MB (fine-tuned variant)
  - `htdemucs_6s`: ~950MB (experimental 6-stem with guitar/piano)

**Export to ONNX**:
```bash
# Install Demucs
pip install demucs

# Download pre-trained model (PyTorch checkpoint)
python3 -m demucs.separate --mp3 -n htdemucs test.mp3  # Downloads model

# Export to ONNX (requires custom export script)
# NOTE: Demucs does not officially support ONNX export yet
# You will need to implement PyTorch → ONNX conversion manually
# See: https://pytorch.org/docs/stable/onnx.html

# Example export script structure:
import torch
import torch.onnx
from demucs.pretrained import get_model

model = get_model('htdemucs')
model.eval()

# Create dummy input (44.1kHz stereo, 4 seconds)
dummy_input = torch.randn(1, 2, 176400)

# Export to ONNX
torch.onnx.export(
    model,
    dummy_input,
    "demucs.onnx",
    export_params=True,
    opset_version=17,
    input_names=['audio'],
    output_names=['drums', 'bass', 'other', 'vocals'],
    dynamic_axes={
        'audio': {0: 'batch', 2: 'samples'},
        'drums': {0: 'batch', 2: 'samples'},
        'bass': {0: 'batch', 2: 'samples'},
        'other': {0: 'batch', 2: 'samples'},
        'vocals': {0: 'batch', 2: 'samples'},
    }
)
```

**Challenges with Demucs ONNX export**:
- Complex hybrid architecture (spectrogram + waveform processing)
- Transformer layers may not export cleanly to ONNX
- Large model size (~800MB) may impact inference performance
- May require ONNX Runtime optimizations (graph optimizations, quantization)

---

### 2. Spleeter

**Spleeter** - Deezer's source separation library (100x faster than real-time on GPU)

- **Repository**: https://github.com/deezer/spleeter
- **Architecture**: U-Net based (TensorFlow)
- **Stems**: 2/4/5 stems (configurable)
- **Performance**: Good quality, very fast
- **License**: MIT
- **Model sizes**:
  - 2-stem (vocals/accompaniment): ~50MB
  - 4-stem (vocals/drums/bass/other): ~90MB
  - 5-stem (vocals/drums/bass/piano/other): ~110MB

**Export to ONNX**:
```bash
# Install Spleeter
pip install spleeter

# Download pre-trained models
spleeter separate -p spleeter:4stems -o output audio.mp3  # Downloads 4-stem model

# Convert TensorFlow SavedModel to ONNX
pip install tf2onnx

# Export TensorFlow model to ONNX
python3 -m tf2onnx.convert \
    --saved-model ~/.cache/spleeter/4stems \
    --output spleeter_4stems.onnx \
    --opset 17

# Create stems.txt
cat > stems.txt << EOF
vocals
drums
bass
other
EOF
```

**Spleeter model configurations**:
- **2-stem**: `spleeter:2stems` → vocals, accompaniment
- **4-stem**: `spleeter:4stems` → vocals, drums, bass, other
- **5-stem**: `spleeter:5stems` → vocals, drums, bass, piano, other

---

## Model Recommendations

| Use Case | Recommended Model | Reason |
|----------|------------------|--------|
| **Best Quality** | Demucs v4 (htdemucs) | State-of-the-art SDR (9.0+ dB), hybrid architecture |
| **Best Speed** | Spleeter 4-stem | 100x faster than real-time, smaller models |
| **Karaoke/Vocals** | Spleeter 2-stem or Demucs | Fast 2-stem isolation for vocals/accompaniment |
| **Music Production** | Demucs htdemucs_6s | 6 stems including guitar/piano separation |
| **Low Memory** | Spleeter 2-stem | ~50MB model vs ~800MB Demucs |

---

## Sample Rate and Audio Format

**Input requirements**:
- Sample rate: 44,100 Hz (CD quality, standard for music)
- Format: Mono or stereo (depends on model training)
- Length: Variable (models process in chunks, typically 4-8 seconds)

**Output format**:
- Sample rate: 44,100 Hz (same as input)
- Channels: Same as input (mono or stereo)
- Stems: Separate audio streams for each instrument/voice class

---

## Licensing and Usage Considerations

### Academic/Research Use
- Both Demucs and Spleeter are MIT licensed
- Free to use for research and non-commercial purposes

### Commercial Use
- MIT license permits commercial use
- Consider music copyright and IP issues:
  - Separating stems from copyrighted music may require rights/licenses
  - Commercial karaoke services may need music licensing agreements
  - Remixing applications may require copyright clearance

### Best Practices
- Only process music you own or have permission to process
- For commercial applications, consult legal counsel about music rights
- Respect artist and producer IP when using source separation

---

## Testing and Validation

Once you have exported an ONNX model and created `stems.txt`, test the plugin:

```bash
# Extract audio from video (44.1kHz)
./video-extract debug \
    --input test_media/sample.mp4 \
    --operations audio,music-source-separation \
    --verbose

# Check output
cat output/music_source_separation.json
```

Expected output structure:
```json
{
  "stems": [
    {
      "stem_name": "vocals",
      "audio": [...],
      "channels": 2
    },
    {
      "stem_name": "drums",
      "audio": [...],
      "channels": 2
    },
    ...
  ]
}
```

---

## Performance Expectations

**Demucs (htdemucs)**:
- Processing time: ~5-15 seconds per minute of audio (GPU)
- Memory: ~2-4GB GPU VRAM
- CPU fallback: 30-120 seconds per minute (very slow)

**Spleeter (4-stem)**:
- Processing time: ~0.6-2 seconds per minute of audio (GPU)
- Memory: ~1-2GB GPU VRAM
- CPU fallback: 10-30 seconds per minute

**Recommendation**: GPU acceleration strongly recommended for music source separation

---

## Troubleshooting

### Model export fails
- Demucs uses complex transformer architecture - may require custom ONNX export
- Try Spleeter first (simpler U-Net architecture, easier to export)
- Check PyTorch/TensorFlow versions match ONNX export tool requirements

### ONNX Runtime errors
- Verify opset version compatibility (use opset 17 or higher)
- Check input tensor shapes match model expectations
- Try simplifying model architecture (quantization, pruning)

### Poor separation quality
- Ensure 44.1kHz sample rate (not 16kHz)
- Verify model is pre-trained (not randomly initialized)
- Check stems.txt order matches model output order

### Out of memory
- Reduce chunk size (process smaller audio segments)
- Use Spleeter instead of Demucs (smaller model)
- Enable ONNX Runtime memory optimizations

---

## Additional Resources

- **Demucs paper**: https://arxiv.org/abs/2111.03600
- **Spleeter paper**: https://joss.theoj.org/papers/10.21105/joss.02154
- **ONNX export guide**: https://pytorch.org/docs/stable/onnx.html
- **Music source separation benchmark**: https://source-separation.github.io/tutorial/
- **Audio processing with ONNX**: https://onnxruntime.ai/docs/tutorials/
