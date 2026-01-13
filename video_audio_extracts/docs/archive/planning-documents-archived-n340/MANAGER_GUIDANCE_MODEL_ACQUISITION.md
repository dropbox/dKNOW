# [MANAGER] Guidance for Worker - Model Acquisition & Feature Expansion
**Date**: 2025-11-01
**For**: Worker AI starting N=228+
**Context**: 5 plugins need models, user wants models acquired + more formats/features suggested

---

## IMMEDIATE TASK: Acquire Missing Models

**Disk Space**: ✅ 198GB available (plenty of space)
**Current Models**: 1.0GB
**Missing Models**: 5 plugins need ONNX files

### Blocker Discovered: PyTorch Version Issue

**Problem**: PyTorch 2.4.0 installed, transformers requires 2.6+ (CVE-2025-32434)
```
ValueError: Due to a serious vulnerability issue in `torch.load`, even with `weights_only=True`,
we now require users to upgrade torch to at least v2.6
```

**Impact**: Can't use `transformers.from_pretrained()` → `torch.onnx.export()` workflow

**Solutions**:
1. ✅ **Use pre-converted ONNX models** from HuggingFace (recommended)
2. ✅ **Use optimum-cli** (HuggingFace ONNX export tool)
3. ⚠️ **Upgrade PyTorch to 2.6+** (may break other dependencies)
4. ✅ **Try safetensors format** (doesn't use torch.load)

---

## Model Acquisition Priority (Easiest First)

### 1. depth-estimation ⭐⭐⭐ (Highest Priority)

**Why**: Easy to obtain, widely useful (AR/VR, 3D), reasonable size (15-400MB)

**Option A**: Search HuggingFace for pre-converted ONNX
```bash
# Search for DPT/MiDaS ONNX models
huggingface-cli repo-files Intel/dpt-hybrid-midas

# Look for ONNX versions on HF
# Try: Intel/dpt-large-onnx, Intel/dpt-hybrid-onnx (if they exist)
# Try community uploads: search "midas onnx" or "dpt onnx"
```

**Option B**: Use optimum-cli for ONNX export
```bash
pip3 install optimum[onnxruntime]

optimum-cli export onnx \
  --model Intel/dpt-hybrid-midas \
  --task depth-estimation \
  models/depth-estimation/

# This creates .onnx files without using torch.load
```

**Option C**: Use safetensors (bypass torch.load vulnerability)
```python
from transformers import DPTForDepthEstimation
model = DPTForDepthEstimation.from_pretrained("Intel/dpt-hybrid-midas", use_safetensors=True)
# Then export with torch.onnx.export()
```

**Expected Result**: models/depth-estimation/dpt_hybrid.onnx (~400MB)

---

### 2. content-moderation ⭐⭐ (Medium Priority)

**Why**: Useful for content filtering, relatively small (~9MB), but licensing concerns

**Approach**: Search for pre-converted ONNX NSFW models
```bash
# Search HuggingFace
huggingface-cli download Falconsai/nsfw_image_detection --include "*.onnx" --local-dir models/content-moderation/ 2>/dev/null

# Alternative searches:
# - search "nsfw onnx" on https://huggingface.co/models
# - search "opennsfw onnx"
# - search "content moderation onnx"
```

**If no ONNX version exists**:
- Look for PyTorch/TensorFlow versions
- May need to use optimum-cli or tf2onnx
- **Fallback**: Document as "user must provide model" and skip

**Expected Result**: models/content-moderation/nsfw_mobilenet.onnx (~9MB)

---

### 3. music-source-separation ⭐ (Lower Priority)

**Why**: Complex export, large models (90-800MB), niche use case

**Approach**: Try Spleeter with tf2onnx
```bash
# Check if tensorflow available
python3 -c "import tensorflow; print(tensorflow.__version__)"

# If tensorflow available:
pip3 install spleeter tf2onnx

# Download Spleeter 4-stem model
spleeter separate -p spleeter:4stems -o /tmp/test_output test_edge_cases/audio_lowquality_16kbps__compression_test.mp3

# Convert to ONNX
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

**Challenge**: Spleeter is TensorFlow, may have export issues

**Fallback**: Search HuggingFace for "spleeter onnx" or "demucs onnx" pre-converted models

**Expected Result**: models/music-source-separation/spleeter.onnx (~90MB)

---

### 4. caption-generation ⏸️ (Defer)

**Why**: Very large (500MB-7GB), complex multi-model architecture, user should choose

**Recommendation**: ❌ **Skip for now**
- Let user choose model (BLIP vs BLIP-2 vs ViT-GPT2 vs LLaVA)
- Models too large to bundle
- Complex encoder-decoder architecture
- User can add later if needed

**If user insists**: Try optimum-cli with BLIP base
```bash
pip3 install optimum[onnxruntime]

optimum-cli export onnx \
  --model Salesforce/blip-image-captioning-base \
  --task image-to-text \
  models/caption-generation/

# Creates multiple ONNX files (encoder + decoder)
```

---

### 5. logo-detection ❌ (Skip)

**Why**: Requires training YOLOv8 on logo dataset (4-8 hours), IP concerns

**Recommendation**: ❌ **Do NOT implement unless user has specific need**
- Requires logo dataset (LogoDet-3K, FlickrLogos-32)
- Training takes 2-4 hours on GPU
- Logo IP restrictions
- Niche use case

**Alternative**: Search HuggingFace/Roboflow for pre-trained models, but rare

---

## Summary: Model Acquisition Plan

**Feasible** (attempt these):
1. ✅ depth-estimation - Try optimum-cli or safetensors
2. ⚠️ content-moderation - Search HF for pre-converted ONNX
3. ⚠️ music-source-separation - Try Spleeter if TensorFlow available

**Defer** (too complex or low priority):
4. ⏸️ caption-generation - Too large, let user choose
5. ❌ logo-detection - Requires training, skip

**Expected Outcome**:
- Best case: 3 new models acquired (depth, content-mod, music-sep)
- Realistic: 1-2 models acquired (depth + maybe one other)
- Minimum: Document blockers, provide instructions for user

---

## FORMATS RESEARCH: Agent Findings

**Documents Created by Agent**:
- UNSUPPORTED_FORMATS_RESEARCH.md
- FORMAT_SUPPORT_MATRIX.md

**Key Findings**: 31 additional formats identified

**CRITICAL DISCOVERY**: ⭐ **HEIF/HEIC** (iPhone photos since iOS 11)
- Billions of files in the wild
- 50% smaller than JPEG with same quality
- Requires libheif library
- **HIGHEST PRIORITY** format to add

**Easy Additions** (FFmpeg already supports): 22 formats
- Professional: MXF, DNxHD, ProRes (detailed)
- Consumer: MTS/M2TS (camcorders), VOB (DVDs), TS (broadcast)
- Audio: WMA (Windows), AMR (mobile voice), APE (lossless)
- Legacy: RM/RMVB (RealMedia), ASF
- **Can add in 1-2 commits** (batch addition)

**Medium Effort**:
- HEIF/HEIC (requires libheif crate)
- Camera RAW (CR2, NEF, ARW, DNG) (requires rawloader crate)
- HLS streaming (.m3u8)
- DASH streaming (.mpd)

**Recommendation**: **Start with HEIF/HEIC** (massive impact, 2-3 commits)

---

## ML FEATURES RESEARCH: Agent Findings

**Document Created by Agent**:
- MISSING_ML_FEATURES_ANALYSIS.md

**Key Findings**: 68 missing features identified

**CRITICAL INSIGHT**: Many features are 80% done already!
- Language detection: Already in Whisper, just need to expose
- Voice Activity Detection: Already have WebRTC VAD (used in diarization)
- Visual search: Already have CLIP + Qdrant
- Cross-modal search: Already have all 3 embedding types

**Quick Wins** (≤5 commits each, 15-21 commits total):
1. ✅ **Language Detection** (1 commit) - Expose Whisper language info
2. ✅ **Voice Activity Detection** (2 commits) - Expose WebRTC VAD as plugin
3. ✅ **Visual Search** (2-3 commits) - Query interface for CLIP embeddings
4. ✅ **Cross-Modal Search** (2-3 commits) - Multi-modal query (text→video, audio→image)
5. ✅ **Text-in-Video Search** (2-3 commits) - Combine OCR + transcription
6. ✅ **Acoustic Scene Classification** (1 commit) - Expose YAMNet scene detection
7. ✅ **Duplicate Detection** (3-4 commits) - Perceptual hashing for video/audio
8. ✅ **Profanity Detection** (2 commits) - Text filter on transcription

**High-Value Features** (5-15 commits each):
- Video summarization (keyframe + transcription + importance scoring)
- Semantic segmentation (SAM - Segment Anything Model)
- Video captioning (dense captioning every N seconds)
- Speaker verification (identify known speakers)
- Scene understanding (indoor/outdoor, time of day, weather)

**Recommended Priority**: Quick wins first (15-21 commits, 2-3 weeks)

---

## MANAGER ASSESSMENT

**Current State** (N=227):
- ✅ 23 plugins functional (22 ML + 1 utility)
- ❌ 5 plugins missing models (skeleton code only)
- ✅ System production-ready for core functionality
- ✅ 198GB disk space available
- ❌ AI stuck in status verification loop (N=180-227, 48 wasted commits)

**Blockers for Model Acquisition**:
- PyTorch 2.4.0 too old for transformers (needs 2.6+)
- torch.hub SSL certificate issues
- Solutions exist (optimum-cli, safetensors, pre-converted ONNX)

**Recommended Path Forward**:

### Option A: Model Acquisition First (N=228-230, 3 commits)
1. **N=228**: Acquire depth model (try optimum-cli or search pre-converted ONNX)
2. **N=229**: Acquire content-moderation model (search HF for pre-converted)
3. **N=230**: Acquire music-sep model (if TensorFlow available) OR skip and document

**Goal**: Get 1-3 of the 5 plugins fully operational

### Option B: Quick Win Features (N=228-235, 8 commits)
1. **N=228**: Language detection plugin (expose Whisper language)
2. **N=229**: VAD plugin (expose WebRTC VAD)
3. **N=230**: Cleanup cycle (N mod 5)
4. **N=231**: Acoustic scene classification (expose YAMNet scenes)
5. **N=232**: Profanity detection (text filter on transcription)
6. **N=233**: Visual search interface (CLIP query API)
7. **N=234**: Duplicate detection (perceptual hashing)
8. **N=235**: Cleanup cycle (N mod 5)

**Goal**: 7 new functional features using existing infrastructure

### Option C: Format Expansion (N=228-232, 5 commits)
1. **N=228**: HEIF/HEIC support (libheif integration) ← **CRITICAL**
2. **N=229**: Batch-add 22 trivial formats (MTS, MXF, WMA, AMR, VOB, TS, etc.)
3. **N=230**: Cleanup cycle (N mod 5)
4. **N=231**: Camera RAW support (rawloader integration)
5. **N=232**: HLS streaming support (.m3u8 playlists)

**Goal**: 17 → 35+ formats supported, including iPhone HEIC

---

## MY RECOMMENDATION

**Priority 1**: Option C (Format Expansion)
- **HEIF/HEIC is CRITICAL** (iPhone photos everywhere)
- Massive impact for AI search (billions of photos)
- Clean, well-scoped work
- No model download blockers

**Priority 2**: Option B (Quick Win Features)
- 7 features using existing code (language detection, VAD, etc.)
- High value for AI search/agent workflows
- No external dependencies

**Priority 3**: Option A (Model Acquisition)
- **Blocked by PyTorch version issue**
- Needs PyTorch upgrade (risky) or alternative approaches
- Lower priority than HEIF/HEIC

**Start with HEIF/HEIC** (iPhone photos) - this alone is worth the effort.

---

## DOCUMENTS CREATED FOR WORKER

### Model Analysis
1. ✅ **MISSING_MODELS_REPORT.md** - Which models are missing, why, how to get them
2. ✅ **models/depth-estimation/README.md** - MiDaS/DPT acquisition guide
3. ✅ **models/content-moderation/README.md** - NSFW model guide
4. ✅ **models/logo-detection/README.md** - Logo detection training guide
5. ✅ **models/music-source-separation/README.md** - Demucs/Spleeter guide

### Format Research
6. ✅ **UNSUPPORTED_FORMATS_RESEARCH.md** - 31 formats analyzed (created by agent)
7. ✅ **FORMAT_SUPPORT_MATRIX.md** - Quick reference (created by agent)

### Feature Research
8. ✅ **MISSING_ML_FEATURES_ANALYSIS.md** - 68 features analyzed (created by agent)

### State Reports
9. ✅ **REAL_STATE_REPORT_N227.md** - Honest assessment of current state
10. ✅ **COMPREHENSIVE_FEATURE_REPORT_N227.md** - Complete feature/format/optimization inventory

### Test Infrastructure
11. ✅ **TEST_EXPANSION_BEFORE_OPTIMIZATION.md** - 54 test plan (NOT implemented)
12. ✅ **LOCAL_PERFORMANCE_IMPROVEMENTS.md** - 15 optimizations (COMPLETE at N=163)
13. ✅ **TEST_FILE_GAP_ANALYSIS.md** - Test file gaps
14. ✅ **test_media_generated/** - 17 new test files (106MB)

---

## NEXT WORKER INSTRUCTIONS

**You are N=228**. Read these files in order:

1. **COMPREHENSIVE_FEATURE_REPORT_N227.md** - Current state (23 functional plugins, 5 need models)
2. **UNSUPPORTED_FORMATS_RESEARCH.md** - 31 formats to add (HEIF/HEIC CRITICAL)
3. **MISSING_ML_FEATURES_ANALYSIS.md** - 68 features to add (8 quick wins)

**Your mission**: Choose ONE path:

**Path A**: Add HEIF/HEIC support (N=228-229, 2 commits) ← **RECOMMENDED**
- Massive impact (iPhone photos everywhere)
- Clean, well-scoped
- Use libheif crate

**Path B**: Acquire 1-2 models (N=228-229, 2 commits)
- Try optimum-cli for depth-estimation
- Search HF for pre-converted ONNX NSFW model
- **Blocked by PyTorch 2.4.0 issue** (may need workarounds)

**Path C**: Quick win features (N=228-235, 8 commits)
- Add language detection plugin
- Add VAD plugin
- Add acoustic scene classification
- Add profanity detection
- Add visual search interface
- Add duplicate detection
- Add cross-modal search

**RECOMMENDATION**: Path A (HEIF/HEIC) - biggest bang for buck, no blockers

---

## PyTorch/Transformers Issue Resolution

**Problem**: transformers requires torch 2.6+, we have 2.4.0

**Options for Worker**:

### Option 1: Use optimum-cli (No torch.load required)
```bash
pip3 install optimum[onnxruntime]

# Export without using transformers.from_pretrained()
optimum-cli export onnx --model Intel/dpt-hybrid-midas --task depth-estimation models/depth-estimation/
```

### Option 2: Search for pre-converted ONNX on HuggingFace
```bash
# Many models have community ONNX conversions
# Search: "dpt onnx", "midas onnx", "nsfw onnx", "blip onnx"
huggingface-cli download <repo-name> --include "*.onnx" --local-dir models/
```

### Option 3: Use safetensors (Bypass vulnerability)
```python
from transformers import DPTForDepthEstimation
model = DPTForDepthEstimation.from_pretrained("Intel/dpt-hybrid-midas", use_safetensors=True)
# Then torch.onnx.export() works
```

### Option 4: Upgrade PyTorch (Risky)
```bash
pip3 install --upgrade torch>=2.6.0
# Risk: May break other dependencies (onnxruntime, sentence-transformers, etc.)
# Recommendation: Try other options first
```

**Recommended**: Option 1 (optimum-cli) or Option 2 (pre-converted ONNX)

---

## FORMAT EXPANSION RECOMMENDATIONS (From Agent Research)

**Phase 1: Critical Formats** (1-2 commits)
1. ⭐⭐⭐⭐⭐⭐ **HEIF/HEIC** (iPhone photos) ← **DO THIS FIRST**
2. Batch-add 8 trivial formats: MTS, M2TS, MXF, WMA, AMR, VOB, TS, DNxHD

**Implementation**:
```rust
// For HEIF/HEIC, add libheif crate
[dependencies]
libheif-rs = "1.0"  // or libheif-sys

// Create heif decoder in ingestion crate
pub fn load_heif(path: &Path) -> Result<RgbImage> {
    let ctx = libheif::Context::new()?;
    let handle = ctx.read_from_file(path)?;
    let image = handle.decode(...)?;
    // Convert to RgbImage
}

// Register in format detection
match extension {
    "heic" | "heif" => load_heif(path),
    // ...
}
```

**Testing**:
- Get test HEIC files (iPhone photos)
- Verify decode works
- Add to smoke tests

---

## FEATURE EXPANSION RECOMMENDATIONS (From Agent Research)

**Phase 1: Quick Wins** (8 features, 15-21 commits, 2-3 weeks)

**Super Easy** (already 80% done):
1. ✅ **Language Detection** (1 commit) - Whisper already detects language, just expose in API
2. ✅ **Acoustic Scene Classification** (1 commit) - YAMNet outputs scene classes, just expose
3. ✅ **Voice Activity Detection** (2 commits) - WebRTC VAD already used internally, expose as plugin

**Easy** (using existing infrastructure):
4. ✅ **Profanity Detection** (2 commits) - Filter transcription with profanity word list
5. ✅ **Visual Search** (2-3 commits) - Query interface for CLIP embeddings in Qdrant
6. ✅ **Cross-Modal Search** (2-3 commits) - Query across vision/text/audio embeddings
7. ✅ **Text-in-Video Search** (2-3 commits) - Combine OCR + transcription text search
8. ✅ **Duplicate Detection** (3-4 commits) - Perceptual hashing (pHash) for images/video

**Total**: 15-21 commits for 8 high-value features

---

## WORK PRIORITIES FOR NEXT WORKER

### Recommended Order:

**Week 1** (N=228-232, 5 commits):
1. N=228: **HEIF/HEIC support** ← **CRITICAL IMPACT**
2. N=229: Batch-add trivial formats (MTS, MXF, WMA, VOB, etc.)
3. N=230: Cleanup cycle (N mod 5)
4. N=231: Language detection plugin (expose Whisper)
5. N=232: VAD plugin (expose WebRTC VAD)

**Week 2** (N=233-237, 5 commits):
6. N=233: Acoustic scene classification plugin (expose YAMNet)
7. N=234: Profanity detection plugin
8. N=235: Cleanup cycle (N mod 5)
9. N=236: Try depth model acquisition (optimum-cli)
10. N=237: Try content-moderation model acquisition (search HF)

**Week 3+** (N=238+):
11. Visual search interface
12. Cross-modal search
13. Text-in-video search
14. Duplicate detection
15. ... continue with remaining features

---

## STOP DOING: Status Verification Loops

**Problem**: N=180-227 (48 commits) were just "Status Verification - System Healthy"

**Cause**: AI has no clear work after optimization phase complete

**Solution**: This guidance provides **clear actionable work**:
- HEIF/HEIC support (massive impact)
- 8 quick win features (high value)
- 22 trivial format additions (easy wins)
- Model acquisition attempts (if time permits)

**Total Work**: 30-50 commits of productive work available

**DO NOT** commit unless you've done substantive work:
- ❌ Don't commit "status verification" repeatedly
- ❌ Don't commit "system healthy" with no changes
- ✅ DO commit when you've added a feature, fixed a bug, or completed a task
- ✅ DO commit cleanup cycles at N mod 5

---

## SESSION ACCOMPLISHMENTS (MANAGER)

**This Manager Session** (N=145 → N=227 analysis):

**Documents Created**:
1. TEST_EXPANSION_BEFORE_OPTIMIZATION.md (54 test plan)
2. LOCAL_PERFORMANCE_IMPROVEMENTS.md (15 optimizations, PGO forbidden)
3. TEST_FILE_GAP_ANALYSIS.md (test coverage analysis)
4. MISSING_MODELS_REPORT.md (5 plugin model analysis)
5. REAL_STATE_REPORT_N227.md (honest assessment)
6. COMPREHENSIVE_FEATURE_REPORT_N227.md (complete inventory)
7. MANAGER_GUIDANCE_MODEL_ACQUISITION.md (this document)
8. Agent-generated: UNSUPPORTED_FORMATS_RESEARCH.md (31 formats)
9. Agent-generated: FORMAT_SUPPORT_MATRIX.md (format matrix)
10. Agent-generated: MISSING_ML_FEATURES_ANALYSIS.md (68 features)

**Test Files Generated**:
- 17 new synthetic test files (106MB in test_media_generated/)
- Duration/codec/resolution/bitrate diversity

**Analysis Completed**:
- ✅ Real state audit (N=145-227)
- ✅ Plugin operational status (23 functional, 5 skeleton)
- ✅ Optimization verification (10 active, 7 rejected, 4 deferred)
- ✅ Library modification check (ZERO modifications, all upstream)
- ✅ Format gap analysis (31 formats identified, HEIF/HEIC critical)
- ✅ Feature gap analysis (68 features identified, 8 quick wins)

---

## FINAL SUMMARY FOR NEXT WORKER

**Your Task (N=228+)**:

1. **Read COMPREHENSIVE_FEATURE_REPORT_N227.md** (understand current state)
2. **Implement HEIF/HEIC support** (biggest impact, 2-3 commits)
3. **Add quick win features** (8 features, 15-21 commits)
4. **Try model acquisition** (if time permits, optimum-cli or pre-converted ONNX)

**Do NOT**:
- Waste commits on status verification loops
- Upgrade PyTorch without careful testing
- Implement complex features before quick wins
- Skip HEIF/HEIC (it's the most important missing format)

**Total Productive Work Available**: 30-50 commits (4-6 weeks)

**End status verification loops. Start building features.**
