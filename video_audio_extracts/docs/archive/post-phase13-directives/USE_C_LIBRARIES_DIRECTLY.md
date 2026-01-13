# USE C LIBRARIES DIRECTLY - NO RUST OVERHEAD

**Problem**: Rust bindings (ffmpeg-next, whisper-rs) add overhead
**Solution**: Call C libraries directly via FFI or spawn CLI binaries

## Option 1: Direct FFmpeg CLI (Fastest, Zero Overhead)

```rust
// crates/keyframe-extractor/src/lib.rs
use std::process::Command;

pub fn extract_keyframes_fast(video_path: &Path, output_dir: &Path) -> Result<Vec<PathBuf>> {
    // Call FFmpeg CLI directly - THIS IS FAST (4 seconds)
    let output = Command::new("ffmpeg")
        .args(&[
            "-hide_banner",
            "-loglevel", "error",
            "-i", video_path.to_str().unwrap(),
            "-vf", "select='eq(pict_type\\,I)'",
            "-vsync", "vfr",
            "-q:v", "2",  // High quality
            &format!("{}/frame%04d.jpg", output_dir.display())
        ])
        .output()?;

    if !output.status.success() {
        return Err(format!("FFmpeg failed: {}", String::from_utf8_lossy(&output.stderr)));
    }

    // Find generated frames
    let frames: Vec<PathBuf> = std::fs::read_dir(output_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map_or(false, |ext| ext == "jpg"))
        .collect();

    Ok(frames)
}
```

**Benchmark this**. If it's 4-5s, USE IT. Forget the Rust decoder.

## Option 2: Direct C FFI (No Rust Bindings)

```rust
// Use libavcodec C API directly, not ffmpeg-next wrapper
#[link(name = "avcodec")]
extern "C" {
    fn avcodec_find_decoder(codec_id: c_int) -> *mut AVCodec;
    fn avcodec_alloc_context3(codec: *const AVCodec) -> *mut AVCodecContext;
    // Direct C calls - NO Rust abstractions
}

unsafe fn decode_frame_c(/* ... */) {
    // Raw C calls - maximum performance
    // Skip Rust safety if it adds overhead
}
```

## Option 3: Call whisper.cpp CLI Directly

```rust
pub fn transcribe_fast(audio_path: &Path) -> Result<String> {
    let output = Command::new("whisper")
        .args(&[
            audio_path.to_str().unwrap(),
            "--model", "base",
            "--output_format", "txt",
            "--language", "en"
        ])
        .output()?;

    // Read whisper output file
    let transcript_path = audio_path.with_extension("txt");
    std::fs::read_to_string(transcript_path)
}
```

## The Perceptual Hashing Problem

**Current**: Perceptual hashing takes 10+ seconds
**FFmpeg**: Doesn't do it (that's why it's fast)

**Options**:
1. **Remove it entirely** - Speed > deduplication
2. **Make it optional** - Default OFF
3. **Parallelize it** - Hash while decoding next frame

**Decision**: REMOVE IT. It's the main slowdown source.

```rust
// DELETE THIS ENTIRE FUNCTION
fn compute_perceptual_hash(image: &DynamicImage) -> Result<u64> {
    // This is 10+ seconds of overhead
    // FFmpeg doesn't do this
    // REMOVE IT
}
```

## The Real Bottleneck (Profiling Data)

Based on the 17.7s vs 4s gap:

**Time breakdown (estimated)**:
- FFmpeg decode: 3s (same for both)
- Perceptual hashing: 10s (WE ADD THIS)
- Logging: 0.5s (WE ADD THIS)
- Rust overhead: 1s (WE ADD THIS)
- File I/O: 1s (similar)
- JPEG encoding: 2s (similar)

**Remove perceptual hashing = save 10s = match FFmpeg speed**

## Worker Directive (N=194)

**Do NOT**:
- Optimize Rust code line by line
- Try to make our decoder faster
- Add more CoreML optimizations
- Profile again

**DO**:
1. Remove perceptual hashing from keyframe extractor (delete the function)
2. Benchmark again (should be ~7s now, not 17s)
3. If still slow, replace with FFmpeg CLI call (use Command::new)
4. Benchmark until ≤ 5s

**Code to delete**: crates/keyframe-extractor/src/lib.rs - the perceptual hashing code

**Target**: ≤ 5s for keyframe extraction (vs FFmpeg 4s)

**If you can't beat FFmpeg, use FFmpeg CLI.** Period.
