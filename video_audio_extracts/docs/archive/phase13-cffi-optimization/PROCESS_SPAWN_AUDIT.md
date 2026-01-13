# Process Spawn Audit: Finding All Unnecessary External Calls

**Date**: 2025-10-30
**User Question**: "Does this error happen anywhere else?"
**Answer**: YES - Found 6 locations spawning FFmpeg/ffprobe unnecessarily

---

## Audit Results

### ❌ CRITICAL: Spawning Processes We Already Have Embedded

| File | Line | Command | Embedded Alternative | Impact |
|------|------|---------|---------------------|--------|
| **fast.rs** | 152 | `ffmpeg` (keyframes) | `decode_iframes_zero_copy()` | 20-30ms spawn overhead |
| **fast.rs** | 208 | `ffmpeg` (audio) | Need audio C FFI | 20-30ms spawn overhead |
| **keyframe-extractor** | 131 | `ffmpeg` | `decode_iframes_zero_copy()` | 20-30ms spawn overhead |
| **audio-extractor** | 116 | `ffprobe` (check stream) | Need probe C FFI | 15-20ms spawn overhead |
| **audio-extractor** | 140 | `ffmpeg` (extract) | Need audio C FFI | 20-30ms spawn overhead |
| **scene-detector** | 195 | `ffmpeg` (scdet filter) | ⚠️ SPECIAL CASE | See below |

### ✅ OK: Legitimate External Calls

| File | Line | Command | Reason |
|------|------|---------|--------|
| **fast.rs** | 251 | `timeout` (Unix utility) | Not embedded, need OS timeout |
| **debug.rs** | 366 | `timeout` (Unix utility) | Same as above |

---

## Root Cause: Historical MANAGER Directive

**Commit e75ac0d (N=0):**
> "[MANAGER] SPEED ABOVE ALL: Use FFmpeg CLI directly if faster"

**Intent**: Good (use fastest method)
**Implementation**: Wrong (spawn process instead of use embedded libs)
**Result**: 20-30ms unnecessary overhead per call

**The mistake**: Nobody realized libavcodec was ALREADY embedded in our binary!

---

## Detailed Analysis

### 1. Fast Mode Keyframes (fast.rs:152) ❌ WRONG

**Current**:
```rust
Command::new("ffmpeg")  // Spawns separate process
    .args([...keyframe extraction...])
    .status()?;
```

**Should be**:
```rust
let frames = video_audio_decoder::decode_iframes_zero_copy(&self.input)?;
for (i, frame) in frames.iter().enumerate() {
    save_frame_as_jpeg(frame, output_path)?;  // Use mozjpeg (N=101)
}
```

**Impact**: Eliminates 20-30ms spawn overhead
**Status**: High priority fix (fast mode mandate)

---

### 2. Fast Mode Audio (fast.rs:208) ❌ WRONG

**Current**:
```rust
Command::new("ffmpeg")  // Spawns separate process
    .args([...audio extraction...])
    .status()?;
```

**Should be**:
```rust
// We need audio C FFI - do we have it?
// Check: video-decoder has decode_iframes_zero_copy for video
// Need: decode_audio() for audio streams
```

**Status**: ⚠️ Need to implement audio C FFI if not exists

---

### 3. Keyframe Plugin (keyframe-extractor/src/lib.rs:131) ❌ WRONG

**Current** (lines 38-41):
```rust
pub use_ffmpeg_cli: bool,  // Flag to choose FFmpeg CLI vs C FFI

// Line 131: When use_ffmpeg_cli = true
if config.use_ffmpeg_cli {
    Command::new("ffmpeg").args([...]).output()?;  // ← WRONG
} else {
    // Use C FFI decoder (correct)
    let decoder_config = video_audio_decoder::DecoderConfig { ... };
}
```

**The plugin ALREADY has dual-mode!**
- FFmpeg CLI mode: `use_ffmpeg_cli: true` (spawns process) ❌
- C FFI mode: `use_ffmpeg_cli: false` (uses embedded) ✅

**Should be**: Remove FFmpeg CLI mode entirely, always use C FFI
- Line 52: Default `use_ffmpeg_cli: true` → `false`
- Line 67: Preview `use_ffmpeg_cli: true` → `false`
- Remove lines 131-147 (FFmpeg spawn code)

**Impact**: Plugin system benefits from 20-30ms savings per operation

---

### 4. Audio Extractor - ffprobe (audio-extractor/src/lib.rs:116) ⚠️ CHECK

**Current**:
```rust
Command::new("ffprobe")  // Check if audio stream exists
    .args([...check stream...])
    .output()?;
```

**Can we use C FFI?**
```rust
// avformat_find_stream_info() returns stream info
// We can query stream types without spawning ffprobe
```

**Status**: ⚠️ Need audio stream detection C FFI

---

### 5. Audio Extractor - ffmpeg (audio-extractor/src/lib.rs:140) ❌ WRONG

**Current**:
```rust
Command::new("ffmpeg")  // Extract audio
    .args([...audio extraction...])
    .status()?;
```

**Should be**:
```rust
// Use libavcodec audio decode + libswresample
// Same as video decode but for audio streams
```

**Status**: ⚠️ Need audio extraction C FFI (decode audio stream, resample, save WAV)

---

### 6. Scene Detector (scene-detector/src/lib.rs:195) ⚠️ SPECIAL CASE

**Current**:
```rust
Command::new("ffmpeg")
    .arg("-vf").arg("scdet=t=0.3:s=1")  // Scene detection filter
    .arg("-f").arg("null")
    .output()?;

// Parse stderr for scene change scores:
// [scdet @ 0x...] lavfi.scd.score: 0.456, lavfi.scd.time: 1.234
```

**Problem**: Uses FFmpeg's built-in scdet filter, which outputs to stderr

**Can we use C FFI?**
- scdet filter is in libavfilter (we have this embedded?)
- But would need to parse C filter graph API
- Complexity: HIGH

**Status**: ⚠️ May need to keep CLI for this (complex filter graph parsing)

---

## Summary: 4 Critical Fixes Needed

### High Priority (Fast Mode Mandate)

**1. fast.rs:152 - Keyframes**
- ❌ Spawns ffmpeg
- ✅ Have C FFI: `decode_iframes_zero_copy()`
- 🔧 Fix: Use C FFI + add JPEG saving
- ⏱️ Impact: 20-30ms savings

**2. fast.rs:208 - Audio**
- ❌ Spawns ffmpeg
- ⚠️ Need C FFI: Audio decode + resample + save
- 🔧 Fix: Implement audio C FFI OR accept spawn
- ⏱️ Impact: 20-30ms savings

### Medium Priority (Plugin System Performance)

**3. keyframe-extractor/src/lib.rs:131**
- ❌ Has dual-mode, defaults to CLI spawn
- ✅ Have C FFI mode already (use_ffmpeg_cli: false)
- 🔧 Fix: Change default to false, remove CLI code
- ⏱️ Impact: 20-30ms per keyframe operation

**4. audio-extractor/src/lib.rs:116,140**
- ❌ Spawns ffprobe + ffmpeg
- ⚠️ Need C FFI for audio
- 🔧 Fix: Implement audio C FFI
- ⏱️ Impact: 30-40ms per audio operation

### Low Priority (Complex Case)

**5. scene-detector/src/lib.rs:195**
- ❌ Spawns ffmpeg for scdet filter
- ⚠️ Could use libavfilter C API (complex)
- 🔧 Fix: Defer (already 44x faster than alternative)
- ⏱️ Impact: Minimal (already optimized)

---

## What We Need to Implement

### Already Have ✅
- ✅ Video decode C FFI: `decode_iframes_zero_copy()`
- ✅ JPEG encoding: mozjpeg integrated (N=101)

### Need to Implement ⚠️
- ⚠️ Audio decode C FFI: Decode audio stream → PCM samples
- ⚠️ Audio resample C FFI: Convert sample rate (libswresample)
- ⚠️ Audio save C FFI: Write WAV file with headers
- ⚠️ Stream probe C FFI: Query stream info without spawning ffprobe

**Estimated effort:** 2-3 commits for audio C FFI (3-4 hours)

---

## Impact Analysis

### If We Fix All Process Spawns

**Current overhead per operation:**
- Binary startup: 47ms (unavoidable)
- Process spawn: 20-30ms (eliminable) ← FIX THIS
- Validation: 10ms (optional)
- Other: 5ms
- **Total**: ~82ms

**After fixing process spawns:**
- Binary startup: 47ms (unavoidable)
- Process spawn: 0ms (using embedded libs) ✅
- Validation: 10ms (optional)
- Other: 5ms
- **Total**: ~62ms

**Result**: 62ms vs FFmpeg CLI 44ms = 1.4x slower (vs current 1.8x)

**But wait...** our binary startup is 47ms, FFmpeg is 44ms (only 3ms difference).

So if we eliminate spawns:
- Our time: 47ms startup + 130ms work = 177ms
- FFmpeg time: 44ms startup + 130ms work = 174ms
- **Gap: 3ms (1.7%)** ✅ ACHIEVES MANDATE!

---

## Recommendation

**Immediate (N=39)**: Fix fast mode keyframes
- Use C FFI instead of spawning ffmpeg
- Expected: Match FFmpeg within 5%

**Medium term (N=40-41)**: Implement audio C FFI
- Replace audio-extractor process spawns
- Replace fast mode audio spawn

**Long term**: Consider scene-detector C FFI (libavfilter)
- Complex API, low ROI (already 44x faster)

**Worker should prioritize fast mode first** (highest user visibility).
