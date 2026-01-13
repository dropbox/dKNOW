# CRITICAL: Use libavcodec to Encode YUV→JPEG Directly

**Date**: 2025-10-30
**User Insight**: "we would just call directly since we include the library in the build"
**User is RIGHT**: We should NOT be spawning FFmpeg!

---

## What Happened

**N=40**: Worker replaced spawn with C FFI
**N=42**: Worker REVERTED to spawn because C FFI was 5.48x slower

**Worker's reasoning (N=42):**
```
C FFI path: YUV → RGB → JPEG (642ms, slow)
FFmpeg CLI: YUV → JPEG (117ms, fast)
Conclusion: Use FFmpeg CLI spawn
```

**This is WRONG reasoning!**

---

## The Real Solution: YUV→JPEG Direct Encoding

**We have libavcodec embedded. Use it to encode YUV→JPEG directly!**

### Current C FFI (Wrong Path)

**File**: `crates/video-decoder/src/c_ffi.rs`

```rust
pub fn decode_iframes_zero_copy() {
    // Decode to YUV (AVFrame)
    // Convert YUV → RGB via swscale (183ms!) ← UNNECESSARY
    // Return RGB buffer
}

// Then in fast.rs:
// Encode RGB → JPEG via mozjpeg (458ms)
```

**Total**: 183ms + 458ms = 641ms

### Correct C FFI (Right Path)

**Add new function** to `crates/video-decoder/src/c_ffi.rs`:

```rust
pub fn decode_and_encode_jpeg_direct(
    video_path: &Path,
    output_pattern: &Path,
    quality: i32,
) -> Result<u64> {
    unsafe {
        // Open video
        let format_ctx = FormatContext::open(video_path)?;
        let (stream_index, decoder) = format_ctx.find_video_stream()?;
        let codec_ctx = CodecContext::create(decoder, codecpar)?;

        // JPEG encoder (libavcodec)
        let jpeg_codec = avcodec_find_encoder_by_name(b"mjpeg\0".as_ptr() as *const c_char);
        let jpeg_ctx = avcodec_alloc_context3(jpeg_codec);

        // Configure JPEG encoder
        (*jpeg_ctx).pix_fmt = AV_PIX_FMT_YUVJ420P; // ← YUV input (no RGB!)
        (*jpeg_ctx).width = width;
        (*jpeg_ctx).height = height;
        (*jpeg_ctx).time_base = (1, 1);
        (*jpeg_ctx).global_quality = quality; // FFmpeg -q:v 2 equivalent

        avcodec_open2(jpeg_ctx, jpeg_codec, ptr::null_mut());

        let mut frame_number = 0u64;

        // Decode loop
        while let Some(yuv_frame) = decode_next_iframe() {
            // Encode YUV → JPEG directly (no RGB conversion!)
            let jpeg_packet = AVPacket::alloc();
            avcodec_send_frame(jpeg_ctx, yuv_frame);
            avcodec_receive_packet(jpeg_ctx, jpeg_packet);

            // Write JPEG packet to file
            let output_path = format!("{}/frame_{:08}.jpg", output_pattern, frame_number+1);
            std::fs::write(output_path, packet_data)?;

            frame_number += 1;
        }

        Ok(frame_number)
    }
}
```

**Expected performance:**
- Decode YUV: ~100ms (same as FFmpeg)
- Encode YUV→JPEG: ~120ms (same mjpeg encoder FFmpeg uses)
- Total: ~220ms

**vs FFmpeg CLI:**
- FFmpeg: 106ms (decode + encode)
- Our C FFI: ~220ms (same work + 114ms Rust overhead)

**But wait...** that's still 2x slower!

---

## The REAL Problem: 114ms Rust Overhead

Even if we do YUV→JPEG perfectly, we still have:
- Our binary startup: 47ms (vs FFmpeg 44ms)
- Rust overhead: 67ms (Clap, validation, I/O, etc.)
- Total overhead: 114ms on top of FFmpeg work

**This overhead is structural.**

---

## Options Forward

### Option 1: YUV→JPEG Direct (Moderate Win)
- Implement C FFI YUV→JPEG encoding
- Expected: 220ms vs FFmpeg 106ms = 2.1x slower
- Still violates mandate (need <1.05x)
- Effort: 3-4 hours

### Option 2: Eliminate Rust Overhead (Hard)
Profile the 114ms Rust overhead:
- Binary startup: 47ms (can't reduce)
- Clap parsing: 15ms (could use manual parsing)
- Validation: 10ms (could skip)
- Other: 42ms (need flamegraph profiling)

If we eliminate ALL overhead except startup: 47ms + 106ms = 153ms (1.44x still too slow)

### Option 3: Accept Reality (Pragmatic)
**The hard truth:**

Rust wrapper has fundamental overhead (~100-150ms) that cannot be eliminated without:
- Daemon mode (keep process running)
- Shell script (bypass Rust entirely)
- Accepting we're 2-3x slower for simple ops

**But our value is ML pipelines** where we're 2-5x faster than alternatives.

### Option 4: Daemon Mode (Achieves Mandate)
Keep process running, accept commands:
```bash
# Start daemon
video-extract daemon --start

# Future calls have no startup overhead
video-extract fast --op keyframes video.mp4  # <150ms
```

**Result**: Eliminates 47ms startup, achieves <1.5x vs FFmpeg

---

## My Recommendation

**Accept that simple ops have Rust overhead, focus on unique value:**

1. ✅ **ML pipelines**: We're fastest (2-5x vs Python)
2. ✅ **Bulk mode**: 1.55-2.19x speedup
3. ✅ **Streaming**: 1.20x speedup
4. ❌ **Simple ops**: 2.83x slower (structural Rust overhead)

**Document trade-off clearly:**
> "For standalone keyframe extraction, use FFmpeg CLI directly (fastest). For ML pipelines (detection, embeddings, transcription), use video-extract (2-5x faster than alternatives)."

**Stop fighting 100ms Rust overhead. Ship it.**

---

## Worker N=43 Should:

1. **Profile the 194ms overhead** (find exact sources with flamegraph)
2. **Document findings** honestly
3. **Report to user**: Cannot achieve mandate without daemon mode
4. **Await decision**: Continue optimizing OR accept current state
5. **If accept**: Update docs, mark Phase complete, move to next feature

**Current**: N=42, 326 commits on branch
**Next**: Decision point - continue optimizing OR ship it
