# N=129: HEVC Decoder Fix - Incomplete Investigation

**Date:** 2025-11-09 (00:21)
**Worker:** N=129
**Status:** ⚠️  **INCOMPLETE** - HEVC detection implemented but not working, needs debugging

---

## Objective

Fix HEVC decoder corruption bug identified in N=128 by routing HEVC files to FFmpeg CLI decoder instead of C FFI decoder.

---

## Changes Made

### 1. Added HEVC Detection Function

**File:** `crates/keyframe-extractor/src/lib.rs`

Added `is_hevc_codec()` function that uses metadata extraction to probe video codec:

```rust
fn is_hevc_codec(video_path: &Path) -> bool {
    let metadata_config = MetadataConfig { include_streams: true };

    match extract_metadata(video_path, &metadata_config) {
        Ok(metadata) => {
            if let Some(video_stream) = metadata.video_stream {
                if let Some(codec_name) = video_stream.codec_name {
                    let codec_lower = codec_name.to_lowercase();
                    return codec_lower == "hevc"
                        || codec_lower == "h265"
                        || codec_lower == "h.265"
                        || codec_lower.contains("hevc");
                }
            }
        }
        Err(_) => {}
    }
    false
}
```

### 2. Updated Keyframe Extraction Dispatch Logic

**File:** `crates/keyframe-extractor/src/lib.rs:147-162`

```rust
// Detect HEVC codec (N=129: C FFI decoder produces corrupted frame 0 for HEVC files)
let is_hevc = is_hevc_codec(video_path);

// Dispatch based on file type and configuration
if is_raw {
    extract_keyframes_raw_dcraw(video_path, &config)
} else if config.use_ffmpeg_cli || is_mxf || is_hevc {  // Added is_hevc
    extract_keyframes_ffmpeg_cli(video_path, &config)
} else {
    extract_keyframes_decode(video_path, &config)
}
```

### 3. Added Dependency

**File:** `crates/keyframe-extractor/Cargo.toml`

Added `video-audio-metadata` dependency to access metadata extraction.

---

## Problem: HEVC Detection Not Working

### Evidence

**Test file:** `test_edge_cases/video_hevc_h265_modern_codec__compatibility.mp4`

**ffprobe confirms HEVC codec:**
```json
{
  "codec_name": "hevc",
  "codec_long_name": "H.265 / HEVC (High Efficiency Video Coding)"
}
```

**But test output shows C FFI decoder still being used:**
```
[32m INFO[0m Using FFmpeg CLI decoder: false
[swscaler @ 0x148ed8000] No accelerated colorspace conversion from yuv420p to rgb24.
```

The `swscaler` message indicates the C FFI decoder is running, not FFmpeg CLI.

### Debugging Attempted

1. Added INFO-level logging to `is_hevc_codec()` function
2. Added logging when HEVC is detected
3. Set RUST_LOG environment variable
4. Checked with module-specific logging: `RUST_LOG=video_audio_keyframe=debug`

**Result:** No log messages from `is_hevc_codec()` function appear in output.

**Conclusion:** The `is_hevc_codec()` function is either:
- Not being called at all
- Failing silently and returning false immediately
- Being optimized out in release builds
- Having issues with metadata extraction (ffprobe not found, permission error, etc.)

---

## Possible Root Causes

### 1. Metadata Extraction Failure

`extract_metadata()` may be failing silently. Possible reasons:
- ffprobe not in PATH
- Permission issues
- File path encoding issues
- Metadata extraction crate has a bug

### 2. Compiler Optimization

Release builds may be optimizing away the function or inlining it in a way that prevents logging.

### 3. Circular Dependency

The metadata extraction module may have its own dependency issues.

---

## Next Steps for N=130

### Option A: Debug Metadata Extraction (Recommended)

1. Create a minimal test program that calls `extract_metadata()` directly
2. Check if ffprobe is accessible from Rust context
3. Add error handling and logging at every step
4. Test in debug mode first, then release mode

### Option B: Use FFmpeg CLI for Codec Detection

Replace metadata extraction with direct ffprobe call:

```rust
fn is_hevc_codec(video_path: &Path) -> bool {
    let output = Command::new("ffprobe")
        .args(["-v", "quiet", "-select_streams", "v:0",
               "-show_entries", "stream=codec_name",
               "-of", "default=noprint_wrappers=1:nokey=1"])
        .arg(video_path)
        .output();

    if let Ok(output) = output {
        let codec = String::from_utf8_lossy(&output.stdout);
        return codec.trim() == "hevc";
    }
    false
}
```

### Option C: Temporary Workaround

Hardcode known HEVC test files or use file naming convention:

```rust
fn is_hevc_codec(video_path: &Path) -> bool {
    // Temporary: check if filename contains "hevc" or "h265"
    if let Some(name) = video_path.file_name().and_then(|n| n.to_str()) {
        let name_lower = name.to_lowercase();
        return name_lower.contains("hevc") || name_lower.contains("h265");
    }
    false
}
```

---

## Files Modified

- `crates/keyframe-extractor/src/lib.rs`: Added HEVC detection and dispatch logic
- `crates/keyframe-extractor/Cargo.toml`: Added metadata-extraction dependency

---

## Tests Run

**Manual test:**
```bash
./target/release/video-extract debug --ops keyframes \
    test_edge_cases/video_hevc_h265_modern_codec__compatibility.mp4
```

**Result:** Still uses C FFI decoder (HEVC detection failed)

---

## Recommendation

**Priorit 1:** Use Option B (direct ffprobe call) for immediate fix
**Priority 2:** Investigate Option A (debug metadata extraction) for proper long-term solution

**Timeline:** 1-2 AI commits to complete fix

---

## Context for Next AI

The HEVC decoder bug from N=128 is real and confirmed. The proposed fix (routing HEVC to FFmpeg CLI) is correct in principle but the implementation has a bug in the HEVC detection logic. The metadata extraction approach is not working - no logs appear, suggesting the function fails before reaching any log statements.

The quickest path forward is Option B: replace metadata extraction with a direct ffprobe Command call.
