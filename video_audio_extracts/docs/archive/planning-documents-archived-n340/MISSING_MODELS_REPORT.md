# Missing ONNX Models Report
**Date**: 2025-11-01 (N=227)
**Purpose**: Document which models are missing for "user-provided" plugins
**Status**: 5 plugins implemented but non-functional without models

---

## Executive Summary

**Finding**: 5 plugins are **SKELETON ONLY** - no ONNX models provided

**These plugins will FAIL at runtime**:
1. ❌ music-source-separation (needs demucs.onnx or spleeter.onnx)
2. ❌ depth-estimation (needs midas_v3_small.onnx or dpt_hybrid.onnx)
3. ❌ content-moderation (needs nsfw_mobilenet.onnx)
4. ❌ logo-detection (needs yolov8_logo.onnx + logos.txt)
5. ❌ caption-generation (needs blip_caption.onnx)

**Plugin Count Reality Check**:
- **Claimed**: 27 plugins operational
- **Actually operational**: 22 plugins (with bundled models)
- **Non-functional**: 5 plugins (missing models)

---

## Model Availability Analysis

### Fully Operational Plugins (22 plugins) ✅

**Models Present** (verified in `models/` directory):

| Plugin | Model File | Size | Status |
|--------|------------|------|--------|
| embeddings (CLIP vision) | clip_vit_b32.onnx | 779MB | ✅ Present |
| embeddings (text) | all_minilm_l6_v2.onnx | 779MB | ✅ Present |
| embeddings (audio) | clap.onnx | 779MB | ✅ Present |
| whisper transcription | various .bin files | 141MB | ✅ Present |
| emotion-detection | emotion_resnet18.onnx | 43MB | ✅ Present |
| diarization | speaker_embedding.onnx | 25MB | ✅ Present |
| object-detection | yolov8n.onnx | 19MB | ✅ Present |
| pose-estimation | yolov8n-pose.onnx | 16MB | ✅ Present |
| audio-classification | yamnet.onnx | 15MB | ✅ Present |
| ocr | ch_PP-OCRv4_det.onnx, ch_PP-OCRv4_rec.onnx | 13MB | ✅ Present |
| image-quality | nima_mobilenetv2.onnx | 8.5MB | ✅ Present |
| face-detection | retinaface_mnet025.onnx | 1.2MB | ✅ Present |

Plus non-ML plugins (11): audio-extraction, keyframes, metadata-extraction, scene-detection, subtitle-extraction, smart-thumbnail, shot-classification, action-recognition, motion-tracking, format-conversion

**Total operational**: 22 plugins

---

### Non-Functional Plugins (5 plugins) ❌

**Models MISSING** (only README files present):

#### 1. music-source-separation ❌

**Location**: `models/music-source-separation/`
**Contents**:
- README.md (8KB)
- stems.txt.example (340 bytes)
- ❌ **NO ONNX MODEL**

**Missing File**: `demucs.onnx` (800MB) or `spleeter.onnx` (90MB)

**Why Missing**:
- Demucs: No official ONNX export available (PyTorch only)
- Spleeter: TensorFlow model, requires tf2onnx conversion
- Complex models, user must export themselves

**What happens if you try to use it**:
```
Error: Failed to load music source separation model from models/music-source-separation/demucs.onnx: No such file or directory
```

---

#### 2. depth-estimation ❌

**Location**: `models/depth-estimation/`
**Contents**:
- README.md (8.5KB)
- ❌ **NO ONNX MODEL**

**Missing Files**:
- `midas_v3_small.onnx` (15MB) or
- `dpt_hybrid.onnx` (400MB) or
- `dpt_large.onnx` (1.3GB)

**Why Missing**:
- Available from HuggingFace but requires PyTorch → ONNX export
- Not pre-packaged in ONNX format
- User must download and export

**What happens if you try to use it**:
```
Error: Failed to load depth estimation model from models/depth-estimation/midas_v3_small.onnx: No such file or directory
```

---

#### 3. content-moderation ❌

**Location**: `models/content-moderation/`
**Contents**:
- README.md (2.6KB)
- ❌ **NO ONNX MODEL**

**Missing File**: `nsfw_mobilenet.onnx` (~9MB)

**Why Missing**:
- NSFW models have licensing/distribution restrictions
- OpenNSFW2 is TensorFlow (needs conversion)
- User must obtain and export their own model

**What happens if you try to use it**:
```
Error: Failed to load content moderation model from models/content-moderation/nsfw_mobilenet.onnx: No such file or directory
```

---

#### 4. logo-detection ❌

**Location**: `models/logo-detection/`
**Contents**:
- README.md (7.5KB)
- ❌ **NO ONNX MODEL**
- ❌ **NO logos.txt**

**Missing Files**:
- `yolov8_logo.onnx` (6-136MB depending on YOLOv8 variant)
- `logos.txt` (text file with logo class names)

**Why Missing**:
- Requires training YOLOv8 on logo dataset (LogoDet-3K, FlickrLogos-32, etc.)
- Logo datasets have intellectual property restrictions
- No pre-trained ONNX models publicly available
- User must train their own model

**What happens if you try to use it**:
```
Error: Failed to load logo detection model from models/logo-detection/yolov8_logo.onnx: No such file or directory
```

---

#### 5. caption-generation ❌

**Location**: `models/caption-generation/`
**Contents**:
- ❌ **DIRECTORY DOESN'T EXIST** (just created as empty)
- ❌ **NO README**
- ❌ **NO ONNX MODEL**

**Missing Files**:
- `blip_caption.onnx` (500MB-7GB depending on model)
- Or: `blip2_opt.onnx`, `vit_gpt2.onnx`, `llava.onnx`

**Why Missing**:
- Caption models are HUGE (500MB-7GB)
- Not feasible to bundle with repository
- HuggingFace Transformers models require export
- User must download and export

**What happens if you try to use it**:
```
Error: Failed to load caption generation model from models/caption-generation/blip_caption.onnx: No such file or directory
```

---

## Summary: Model Availability

| Plugin | Model Status | Size | Effort to Obtain | Functional |
|--------|--------------|------|------------------|------------|
| **22 core plugins** | ✅ **Included** | 1.1GB | N/A (bundled) | ✅ **YES** |
| music-source-separation | ❌ **Missing** | 90-800MB | High (manual export) | ❌ **NO** |
| depth-estimation | ❌ **Missing** | 15MB-1.3GB | Medium (PyTorch export) | ❌ **NO** |
| content-moderation | ❌ **Missing** | ~9MB | Medium (need model + export) | ❌ **NO** |
| logo-detection | ❌ **Missing** | 6-136MB | Very High (train YOLOv8) | ❌ **NO** |
| caption-generation | ❌ **Missing** | 500MB-7GB | High (HF export) | ❌ **NO** |

---

## Why These Models Aren't Included

### 1. Size Constraints
- **caption-generation**: 500MB-7GB (too large for git repository)
- **depth-estimation**: 400MB-1.3GB for high-quality models
- **music-source-separation**: 800MB for Demucs

**Total**: Would add 1.5-9GB to repository (current models: 1.1GB)

### 2. Licensing/IP Issues
- **content-moderation**: NSFW models have distribution restrictions
- **logo-detection**: Brand logos are intellectual property
- Both require user to ensure compliance

### 3. Model Availability
- **music-source-separation**: Demucs has no official ONNX export
- **logo-detection**: Requires custom training on specific logo dataset
- **caption-generation**: Multiple model options, user chooses based on needs

### 4. Configuration Variability
- **logo-detection**: User needs specific brand list (logos.txt)
- **music-source-separation**: User chooses 2/4/5/6-stem configuration
- **caption-generation**: BLIP vs BLIP-2 vs ViT-GPT2 vs LLaVA (different use cases)

---

## Can These Models Be Obtained?

### ✅ YES - Moderately Easy

**depth-estimation** (15 minutes):
```bash
# Download and export MiDaS Small from HuggingFace
pip install torch onnx
python3 -c "
import torch
model = torch.hub.load('intel-isl/MiDaS', 'MiDaS_small')
dummy = torch.randn(1, 3, 256, 256)
torch.onnx.export(model, dummy, 'midas_v3_small.onnx')
"
mv midas_v3_small.onnx models/depth-estimation/
```

**Expected result**: Plugin becomes operational

---

### ⚠️ MAYBE - Moderate Effort

**music-source-separation** (30-60 minutes):
```bash
# Option 1: Spleeter (easier)
pip install spleeter tf2onnx
spleeter separate -p spleeter:4stems -o output audio.mp3
python3 -m tf2onnx.convert \
  --saved-model ~/.cache/spleeter/4stems \
  --output models/music-source-separation/spleeter.onnx \
  --opset 17

# Create stems.txt
cat > models/music-source-separation/stems.txt << EOF
vocals
drums
bass
other
EOF
```

**Challenge**: tf2onnx conversion may require debugging
**Expected result**: Plugin becomes operational if export succeeds

---

### ⚠️ MAYBE - Requires Model Sourcing

**content-moderation** (Variable, depends on model availability):
- Need to find pre-trained NSFW classifier
- OpenNSFW2 available but TensorFlow (needs conversion)
- May find ONNX version on HuggingFace
- Licensing/distribution restrictions apply

**Effort**: 30 minutes (if ONNX version found) to 2 hours (if need to export)

---

### ❌ DIFFICULT - High Effort

**logo-detection** (4-8 hours minimum):
- Requires training YOLOv8 on logo dataset
- Need logo dataset (LogoDet-3K, FlickrLogos-32, or custom)
- Training: 2-4 hours on GPU
- Plus dataset preparation, annotation verification
- Plus legal review for logo IP issues

**caption-generation** (30-60 minutes):
- Large models (500MB-7GB)
- HuggingFace models available
- Export to ONNX straightforward but slow
- Need to choose model (BLIP vs BLIP-2 vs others)

---

## Recommendation

### High Priority: Easy Wins

**1. depth-estimation** - 15 minutes
- Download MiDaS Small from torch.hub
- Export to ONNX
- Place in models/depth-estimation/
- **Result**: Plugin becomes functional

### Medium Priority: Worth Trying

**2. music-source-separation** - 30-60 minutes
- Try Spleeter tf2onnx export
- If fails, skip (Demucs export very complex)
- **Result**: Plugin functional if export succeeds

**3. content-moderation** - 30 minutes - 2 hours
- Search HuggingFace for ONNX NSFW model
- If found, download and test
- If not, requires export from TensorFlow
- **Result**: Plugin functional if model found

### Low Priority: High Effort

**4. logo-detection** - 4-8 hours
- Requires training YOLOv8 on logo dataset
- Or finding pre-trained model (rare)
- Legal/IP considerations
- **Recommendation**: Skip unless user has specific need

**5. caption-generation** - 30-60 minutes
- Large models (500MB-7GB)
- Many options (BLIP, BLIP-2, ViT-GPT2, LLaVA)
- **Recommendation**: Only if user needs captions, let them choose model

---

## Truth About Plugin Claims

### README Says: "27 plugins operational"

**Reality**:
- ✅ 22 plugins **fully operational** (with bundled models)
- ⚠️ 5 plugins **skeleton only** (missing models, will error at runtime)
- Total: 27 plugin code modules exist, but only 22 work

### What "Operational" Actually Means

**For 22 core plugins**: ✅ True
- Model files bundled (1.1GB in `models/`)
- Zero configuration required
- Work out of the box

**For 5 "user-provided" plugins**: ❌ Misleading
- Plugin code exists
- Compiles successfully
- Registers in plugin system
- **But fails when executed** (model not found error)
- Needs 15 minutes to 8 hours of work to obtain models

---

## Model Directory Status

**Actual Contents**:
```
models/
├── audio-classification/     779KB  ✅ yamnet.onnx
├── content-moderation/         4KB  ❌ README.md only (NO MODEL)
├── depth-estimation/          12KB  ❌ README.md only (NO MODEL)
├── diarization/               25MB  ✅ speaker_embedding.onnx
├── embeddings/               779MB  ✅ clip, text, audio models
├── emotion-detection/         43MB  ✅ emotion_resnet18.onnx
├── face-detection/           1.2MB  ✅ retinaface_mnet025.onnx
├── image-quality/            8.5MB  ✅ nima_mobilenetv2.onnx
├── logo-detection/             8KB  ❌ README.md only (NO MODEL)
├── music-source-separation/   12KB  ❌ README.md + stems.txt.example (NO MODEL)
├── object-detection/          19MB  ✅ yolov8n.onnx
├── ocr/                       13MB  ✅ ch_PP-OCRv4 det+rec models
├── pose-estimation/           16MB  ✅ yolov8n-pose.onnx
├── whisper/                  141MB  ✅ base/small/medium/large models
└── caption-generation/         0KB  ❌ EMPTY DIRECTORY
```

**Total actual model size**: 1.1GB (operational models only)
**Potential with missing models**: 2.6-10GB (if all 5 acquired)

---

## Download Sources for Missing Models

### 1. depth-estimation (15 min, easy)

**Recommended**: MiDaS v3.1 Small (15MB)

```bash
pip install torch onnx

python3 << 'EOF'
import torch
model = torch.hub.load('intel-isl/MiDaS', 'MiDaS_small')
model.eval()
dummy = torch.randn(1, 3, 256, 256)
torch.onnx.export(model, dummy, 'midas_v3_small.onnx',
                  input_names=['input'], output_names=['output'],
                  dynamic_axes={'input': {0: 'batch'}, 'output': {0: 'batch'}},
                  opset_version=14)
EOF

mv midas_v3_small.onnx models/depth-estimation/
```

**Alternative**: DPT Hybrid from HuggingFace
```bash
pip install transformers torch onnx

python3 << 'EOF'
from transformers import DPTForDepthEstimation
import torch

model = DPTForDepthEstimation.from_pretrained("Intel/dpt-hybrid-midas")
model.eval()
dummy = torch.randn(1, 3, 384, 384)
torch.onnx.export(model, dummy, 'dpt_hybrid.onnx',
                  input_names=['pixel_values'], output_names=['predicted_depth'],
                  dynamic_axes={'pixel_values': {0: 'batch'}, 'predicted_depth': {0: 'batch'}},
                  opset_version=14)
EOF

mv dpt_hybrid.onnx models/depth-estimation/
```

---

### 2. music-source-separation (30-60 min, moderate)

**Recommended**: Spleeter 4-stem (90MB, easier export)

```bash
pip install spleeter tf2onnx tensorflow

# Download Spleeter models
spleeter separate -p spleeter:4stems -o /tmp/test_output /path/to/test.mp3

# Convert TensorFlow SavedModel to ONNX
python3 -m tf2onnx.convert \
  --saved-model ~/.cache/spleeter/4stems \
  --output models/music-source-separation/spleeter.onnx \
  --opset 17

# Create stems.txt
cat > models/music-source-separation/stems.txt << EOF
vocals
drums
bass
other
EOF
```

**Alternative**: Demucs (800MB, complex export)
- No official ONNX export
- Requires custom PyTorch → ONNX conversion
- Complex hybrid architecture may not export cleanly
- **Not recommended unless Spleeter fails**

---

### 3. content-moderation (30 min - 2 hours, depends on model)

**Option 1**: Search HuggingFace for pre-exported ONNX NSFW model
```bash
# Search HuggingFace
https://huggingface.co/models?search=nsfw+onnx

# If found, download
huggingface-cli download <repo-name> nsfw.onnx
mv nsfw.onnx models/content-moderation/nsfw_mobilenet.onnx
```

**Option 2**: Convert OpenNSFW2 from TensorFlow (if no ONNX version exists)
```bash
pip install opennsfw2 tf2onnx tensorflow

python3 << 'EOF'
# Load OpenNSFW2 model
# Export to ONNX
# ...requires custom code
EOF

mv nsfw_model.onnx models/content-moderation/nsfw_mobilenet.onnx
```

---

### 4. logo-detection (4-8 hours, difficult)

**Requires Training**:
1. Download logo dataset (LogoDet-3K or FlickrLogos-32)
2. Prepare dataset in YOLOv8 format
3. Train YOLOv8 model (2-4 hours on GPU)
4. Export to ONNX
5. Extract class names to logos.txt

**Alternative**: Search for pre-trained YOLOv8 logo model
- Roboflow Universe: https://universe.roboflow.com/ (search "logo detection")
- GitHub repositories
- **Rare**: Most logo models are proprietary

**Recommendation**: ❌ **Skip** unless user has specific logo detection need

---

### 5. caption-generation (30-60 min, moderate)

**Recommended**: BLIP base (500MB, balanced quality/size)

```bash
pip install transformers torch onnx optimum

# Download BLIP base model
python3 << 'EOF'
from transformers import BlipForConditionalGeneration
import torch

model = BlipForConditionalGeneration.from_pretrained("Salesforce/blip-image-captioning-base")
model.eval()

# Export vision encoder
vision_dummy = torch.randn(1, 3, 384, 384)
torch.onnx.export(model.vision_model, vision_dummy, 'blip_vision.onnx')

# NOTE: Full BLIP export is complex (encoder + decoder)
# May need Optimum library: optimum-cli export onnx --model Salesforce/blip-image-captioning-base blip_onnx/
EOF

# Or use Optimum
optimum-cli export onnx \
  --model Salesforce/blip-image-captioning-base \
  --task image-to-text \
  models/caption-generation/
```

**Challenges**:
- BLIP has encoder-decoder architecture (2 ONNX files)
- Full generation pipeline export is complex
- May require Optimum library for proper export

---

## Filesystem Search Results

**Searched locations**:
- ✅ `models/` directory - Found 13 existing models (1.1GB)
- ❌ `~/Downloads` - No ONNX models found
- ❌ `~/Library` - No relevant models found
- ❌ `~/.cache` - No relevant models found
- ❌ Home directory (3 levels deep) - No nsfw/demucs/midas models found

**Conclusion**: Missing models are NOT on filesystem. Must be downloaded/exported.

---

## Honest Plugin Count

**Functional out-of-the-box**: 22 plugins ✅
1. audio-extraction
2. keyframes
3. metadata-extraction
4. transcription
5. diarization
6. audio-classification
7. audio-enhancement-metadata
8. scene-detection
9. object-detection
10. face-detection
11. ocr
12. action-recognition
13. motion-tracking
14. pose-estimation
15. smart-thumbnail
16. subtitle-extraction
17. shot-classification
18. emotion-detection
19. image-quality-assessment
20. vision-embeddings
21. text-embeddings
22. audio-embeddings

**Non-functional (missing models)**: 5 plugins ❌
23. music-source-separation (needs demucs.onnx or spleeter.onnx)
24. depth-estimation (needs midas_v3_small.onnx)
25. content-moderation (needs nsfw_mobilenet.onnx)
26. logo-detection (needs yolov8_logo.onnx + logos.txt)
27. caption-generation (needs blip_caption.onnx)

**Plus**: 1 utility plugin
28. format-conversion (doesn't require ML model)

**Total**: 28 plugin codebases, 23 functional (22 + format-conversion)

---

## Recommendation

### For User

**If you want all 27 ML plugins working**:
1. ✅ **Easy win**: depth-estimation (15 min, MiDaS Small export)
2. ⚠️ **Worth trying**: music-source-separation (30-60 min, Spleeter export)
3. ⚠️ **Depends**: content-moderation (30 min if ONNX exists, 2 hours if need export)
4. ❌ **Skip**: logo-detection (4-8 hours training, IP issues)
5. ⚠️ **Optional**: caption-generation (30-60 min, large models 500MB-7GB)

**Minimum effort for maximum gain**: Just do #1 (depth-estimation)
- Takes 15 minutes
- Widely useful (3D reconstruction, AR/VR)
- No licensing issues (MiDaS is MIT licensed)
- Reasonable model size (15MB)

### For Documentation

**Fix README.md** to be honest:
- Current: "27 plugins operational"
- Accurate: "23 plugins operational (22 ML + 1 utility), 5 require user-provided models"
- Or: "27 plugin implementations, 23 functional out-of-the-box, 5 require model downloads"

---

## Next Steps

If user wants the 5 missing-model plugins functional:

**N=228: Acquire depth-estimation model** (15 min)
- Download and export MiDaS Small
- Test plugin
- Mark as operational

**N=229: Acquire music-source-separation model** (30-60 min)
- Export Spleeter 4-stem to ONNX
- Test plugin
- Mark as operational or skip if export fails

**N=230: Investigate content-moderation model** (30 min - 2 hours)
- Search for ONNX NSFW model
- If found, test plugin
- If not found, document as "requires custom model"

**N=231: Update documentation** (honest plugin counts)
- Fix README.md: "23 functional, 5 require user models"
- Update REAL_STATE_REPORT_N227.md
- Document model acquisition process

**Skip**:
- logo-detection (too much effort, niche use case)
- caption-generation (user can add if needed, models too large to bundle)
