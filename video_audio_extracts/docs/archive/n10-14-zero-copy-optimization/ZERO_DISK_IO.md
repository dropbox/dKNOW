# ZERO DISK I/O: Use Memory Buffers

**Problem**: We write JPEGs to disk then read them back (100ms+ overhead)

**Solution**: Decode to memory, pass pointers. NO DISK.

## Current (Stupid)
```
FFmpeg CLI → frame0001.jpg (disk write 50ms)
          → frame0002.jpg (disk write 50ms)
          ...
Our code  ← read frame0001.jpg (disk read 50ms)
          ← read frame0002.jpg (disk read 50ms)
          ...
YOLO      ← process memory buffer

TOTAL DISK I/O: 100+ frames × 1ms = 100ms+ WASTED
```

## Correct (Smart)
```
libavcodec → AVFrame* (memory buffer, zero-copy)
           → Pass pointer to ONNX
YOLO       ← process memory buffer directly

TOTAL DISK I/O: 0ms
```

## Implementation

```rust
// crates/video-decoder/src/direct_ffi.rs

use std::ffi::{CString, c_int, c_char};
use std::ptr;

#[repr(C)]
struct AVFormatContext { _private: [u8; 0] }
#[repr(C)]
struct AVCodecContext { _private: [u8; 0] }
#[repr(C)]
struct AVFrame { _private: [u8; 0] }
#[repr(C)]
struct AVPacket { _private: [u8; 0] }

#[link(name = "avformat")]
#[link(name = "avcodec")]
#[link(name = "avutil")]
extern "C" {
    fn avformat_open_input(
        ps: *mut *mut AVFormatContext,
        url: *const c_char,
        fmt: *mut c_void,
        options: *mut c_void
    ) -> c_int;

    fn av_read_frame(
        s: *mut AVFormatContext,
        pkt: *mut AVPacket
    ) -> c_int;

    fn avcodec_send_packet(
        avctx: *mut AVCodecContext,
        avpkt: *const AVPacket
    ) -> c_int;

    fn avcodec_receive_frame(
        avctx: *mut AVCodecContext,
        frame: *mut AVFrame
    ) -> c_int;

    fn av_frame_get_buffer(
        frame: *mut AVFrame,
        align: c_int
    ) -> c_int;
}

pub struct RawFrameBuffer {
    pub data: *mut u8,   // Pointer to pixel data (RGB or YUV)
    pub width: u32,
    pub height: u32,
    pub linesize: usize,
    _frame: *mut AVFrame,  // Keep AVFrame alive
}

impl Drop for RawFrameBuffer {
    fn drop(&mut self) {
        unsafe {
            // Free AVFrame when dropped
            av_frame_free(&mut self._frame);
        }
    }
}

pub unsafe fn decode_iframes_to_memory(video_path: &Path) -> Result<Vec<RawFrameBuffer>> {
    let path = CString::new(video_path.to_str().unwrap())?;
    let mut format_ctx = ptr::null_mut();

    // Open video
    if avformat_open_input(&mut format_ctx, path.as_ptr(), ptr::null_mut(), ptr::null_mut()) < 0 {
        return Err("Failed to open video");
    }

    // Find video stream, create decoder, etc.
    // ...

    let mut frames = Vec::new();

    loop {
        let mut packet = av_packet_alloc();
        if av_read_frame(format_ctx, packet) < 0 {
            break;  // EOF
        }

        // Send packet to decoder
        avcodec_send_packet(codec_ctx, packet);

        // Receive frame
        let mut frame = av_frame_alloc();
        if avcodec_receive_frame(codec_ctx, frame) == 0 {
            // Check if I-frame
            if (*frame).pict_type == AV_PICTURE_TYPE_I {
                // Keep frame in MEMORY (don't write to disk)
                frames.push(RawFrameBuffer {
                    data: (*frame).data[0],  // Direct pointer to pixel data
                    width: (*frame).width as u32,
                    height: (*frame).height as u32,
                    linesize: (*frame).linesize[0] as usize,
                    _frame: frame,  // Keep alive
                });
            }
        }

        av_packet_free(&mut packet);
    }

    Ok(frames)
}
```

## Then Pass to ONNX Directly

```rust
// crates/object-detection/src/lib.rs

pub fn detect_objects_zero_copy(frame: &RawFrameBuffer) -> Result<Vec<Detection>> {
    // Convert raw pixels to ONNX tensor (zero-copy if possible)
    let tensor = ndarray::ArrayView3::from_shape_ptr(
        (frame.height as usize, frame.width as usize, 3),
        frame.data as *const u8
    );

    // Run ONNX inference directly on memory buffer
    session.run(vec![tensor])?;

    // NO disk I/O anywhere
}
```

## Performance Impact

**Current**:
- Decode: 1500ms
- Write 100 JPEGs: 100ms ← REMOVE THIS
- Read 100 JPEGs: 100ms ← REMOVE THIS
- YOLO: 50ms

**With memory buffers**:
- Decode: 1500ms
- Pass pointer: <1ms ← INSTANT
- YOLO: 50ms

**Save: 200ms (reduces 2.07s → 1.87s, faster than FFmpeg CLI)**

## Worker Directive

**File**: crates/video-decoder/src/lib.rs

**Add**: Direct C FFI decoder that returns memory buffers

**Modify**: Object detection, embeddings to accept memory buffers

**Remove**: All disk writes/reads in hot path

**Test**: Prove we're faster than FFmpeg CLI (save 200ms)

**Success**: ≤1.8s (beat FFmpeg 1.82s) by eliminating disk I/O
