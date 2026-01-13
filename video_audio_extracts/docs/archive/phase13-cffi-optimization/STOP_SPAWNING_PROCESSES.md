# STOP SPAWNING PROCESSES - USE EMBEDDED LIBRARIES

**Date**: 2025-10-30
**Authority**: USER ORDER via MANAGER
**User**: "that's stupid. don't spawn processes"

---

## THE ORDER

**STOP spawning FFmpeg/ffprobe processes. Use embedded libavcodec/libavformat.**

No exceptions. No "but it's slower". No "but it's harder".

**Just do it.**

---

## WORKER N=44: Implement YUV→JPEG Direct Encoding

### File: crates/video-decoder/src/c_ffi.rs

**Add this function NOW:**

```rust
/// Decode I-frames and encode directly to JPEG (no RGB conversion)
/// Uses libavcodec mjpeg encoder - same as FFmpeg CLI
pub fn decode_and_save_jpegs_direct(
    video_path: &Path,
    output_dir: &Path,
    quality: i32,
) -> Result<u64> {
    unsafe {
        // 1. Open video and find I-frames (same as decode_iframes_zero_copy)
        let format_ctx = FormatContext::open(video_path)?;
        let (stream_index, decoder) = format_ctx.find_video_stream()?;
        let codecpar = format_ctx.get_codecpar(stream_index);
        let codec_ctx = CodecContext::create(decoder, codecpar)?;

        // 2. Create JPEG encoder (mjpeg codec)
        let jpeg_codec = avcodec_find_encoder_by_name(b"mjpeg\0".as_ptr() as *const c_char);
        if jpeg_codec.is_null() {
            return Err(ProcessingError::FFmpegError("mjpeg encoder not found".to_string()));
        }

        let jpeg_ctx = avcodec_alloc_context3(jpeg_codec);
        (*jpeg_ctx).pix_fmt = AV_PIX_FMT_YUVJ420P; // YUV input (FFmpeg uses this)
        (*jpeg_ctx).width = (*codec_ctx.ptr).width;
        (*jpeg_ctx).height = (*codec_ctx.ptr).height;
        (*jpeg_ctx).time_base = AVRational { num: 1, den: 1 };

        // Quality mapping: FFmpeg -q:v 2 ≈ global_quality 2-5
        (*jpeg_ctx).flags |= AV_CODEC_FLAG_QSCALE;
        (*jpeg_ctx).global_quality = quality * FF_QP2LAMBDA;

        let ret = avcodec_open2(jpeg_ctx, jpeg_codec, ptr::null_mut());
        if ret < 0 {
            return Err(ProcessingError::FFmpegError(format!("Failed to open JPEG encoder: {}", ret)));
        }

        let mut frame_number = 0u64;
        let mut packet = AVPacket::alloc();

        // 3. Decode loop
        loop {
            // Read packet
            // Decode I-frame (same as decode_iframes_zero_copy)
            // ... decode logic ...

            if is_iframe {
                // 4. Encode YUV → JPEG directly (NO RGB conversion!)
                let ret = avcodec_send_frame(jpeg_ctx, decoded_frame);
                if ret < 0 {
                    continue; // Skip this frame
                }

                while avcodec_receive_packet(jpeg_ctx, packet.ptr) == 0 {
                    // 5. Write JPEG to file
                    let output_path = output_dir.join(format!("frame_{:08}.jpg", frame_number + 1));
                    let jpeg_data = std::slice::from_raw_parts(
                        (*packet.ptr).data,
                        (*packet.ptr).size as usize,
                    );
                    std::fs::write(&output_path, jpeg_data)?;

                    frame_number += 1;
                    av_packet_unref(packet.ptr);
                }
            }
        }

        // Cleanup
        avcodec_free_context(&mut jpeg_ctx);

        Ok(frame_number)
    }
}
```

### File: crates/video-extract-cli/src/commands/fast.rs

**Change line 152** from:
```rust
Command::new("ffmpeg")  // ← DELETE THIS
```

to:
```rust
// Use embedded libavcodec (NO SPAWN)
video_audio_decoder::decode_and_save_jpegs_direct(
    &self.input,
    &self.output_dir,
    2, // quality (matches FFmpeg -q:v 2)
)?;
```

---

## WHY THIS IS RIGHT

**User is correct:** We have libavcodec embedded. We should use it.

**FFmpeg libs in our binary:**
```bash
$ otool -L target/release/video-extract | grep avcodec
/opt/homebrew/opt/ffmpeg/lib/libavcodec.62.dylib ✅
```

**These are NOT separate programs. They're libraries linked into our executable.**

**Spawning external ffmpeg = loading SAME libraries in NEW process = WASTEFUL**

---

## EXPECTED PERFORMANCE

**FFmpeg CLI work:** ~106ms (decode YUV + encode JPEG)

**Our embedded approach:**
- Binary startup: 47ms (only 3ms slower than FFmpeg's 44ms)
- Decode + encode: ~106ms (SAME libavcodec code)
- Rust overhead: 10-20ms (Clap, validation, I/O)
- **Total: ~163-173ms**

**vs FFmpeg CLI: ~160ms** (1.5x slower, acceptable)

**Much better than current 300ms (2.83x slower)!**

---

## NO MORE ANALYSIS

Worker has spent 7 commits (N=35-43) analyzing why C FFI is slow.

**The answer is simple:** Use libavcodec mjpeg encoder for YUV→JPEG.

**Stop analyzing. Start implementing.**

---

## WORKER N=44 INSTRUCTIONS

1. Add `decode_and_save_jpegs_direct()` to `c_ffi.rs`
2. Use embedded mjpeg encoder (libavcodec)
3. Update `fast.rs` to call it
4. Benchmark
5. Report measurements

**No more "expected" or "should"**. Measure and report facts.

**Estimated:** 1 commit, 3-4 hours

**DO IT NOW.**
