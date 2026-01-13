# WORLD CLASS PERFORMANCE: Link FFmpeg C Libraries Statically

**User mandate**: "WE CAN GET THE SOURCE CODE. WE CAN LINK. MAKE THIS SUPER SUPER FAST!"

**Goal**: Beat every alternative. Zero compromises.

## The Plan: Maximum Performance

### 1. Statically Link FFmpeg C Libraries

```toml
# Cargo.toml or build.rs

[dependencies]
# NO ffmpeg-next (Rust wrapper adds overhead)
# Link C libraries directly

[build-dependencies]
cc = "1.0"

# build.rs
fn main() {
    // Link FFmpeg C libraries
    println!("cargo:rustc-link-lib=dylib=avcodec");
    println!("cargo:rustc-link-lib=dylib=avformat");
    println!("cargo:rustc-link-lib=dylib=avutil");
    println!("cargo:rustc-link-lib=dylib=swscale");

    // Or static link for even faster:
    println!("cargo:rustc-link-lib=static=avcodec");
    println!("cargo:rustc-link-lib=static=avformat");

    // Point to FFmpeg installation
    println!("cargo:rustc-link-search=native=/opt/homebrew/lib");
}
```

### 2. Direct C FFI (Zero Overhead)

```rust
// crates/video-decoder/src/c_ffi.rs

#![allow(non_camel_case_types)]

use std::os::raw::{c_int, c_char, c_void};

// FFmpeg C types
#[repr(C)] pub struct AVFormatContext { _private: [u8; 0] }
#[repr(C)] pub struct AVCodecContext { _private: [u8; 0] }
#[repr(C)] pub struct AVCodecParameters { _private: [u8; 0] }
#[repr(C)] pub struct AVStream { _private: [u8; 0] }
#[repr(C)] pub struct AVCodec { _private: [u8; 0] }
#[repr(C)] pub struct AVPacket { _private: [u8; 0] }
#[repr(C)] pub struct AVFrame {
    pub data: [*mut u8; 8],
    pub linesize: [c_int; 8],
    pub width: c_int,
    pub height: c_int,
    pub pict_type: c_int,
    // ... other fields
}

// FFmpeg C functions
#[link(name = "avformat")]
#[link(name = "avcodec")]
#[link(name = "avutil")]
extern "C" {
    fn avformat_open_input(
        ps: *mut *mut AVFormatContext,
        url: *const c_char,
        fmt: *mut c_void,
        options: *mut *mut c_void
    ) -> c_int;

    fn avformat_find_stream_info(
        ic: *mut AVFormatContext,
        options: *mut *mut c_void
    ) -> c_int;

    fn av_find_best_stream(
        ic: *mut AVFormatContext,
        media_type: c_int,  // AVMEDIA_TYPE_VIDEO = 0
        wanted_stream_nb: c_int,
        related_stream: c_int,
        decoder_ret: *mut *mut AVCodec,
        flags: c_int
    ) -> c_int;

    fn avcodec_alloc_context3(codec: *const AVCodec) -> *mut AVCodecContext;

    fn avcodec_parameters_to_context(
        codec: *mut AVCodecContext,
        par: *const AVCodecParameters
    ) -> c_int;

    fn avcodec_open2(
        avctx: *mut AVCodecContext,
        codec: *const AVCodec,
        options: *mut *mut c_void
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

    fn av_frame_alloc() -> *mut AVFrame;
    fn av_frame_free(frame: *mut *mut AVFrame);
    fn av_packet_alloc() -> *mut AVPacket;
    fn av_packet_free(pkt: *mut *mut AVPacket);
}

// I-frame detection
const AV_PICTURE_TYPE_I: c_int = 1;
```

### 3. Zero-Copy Pipeline

```rust
pub unsafe fn decode_iframes_zero_copy(video_path: &Path) -> Result<Vec<FrameBuffer>> {
    use std::ffi::CString;
    use std::ptr;

    let path = CString::new(video_path.to_str().unwrap())?;
    let mut format_ctx: *mut AVFormatContext = ptr::null_mut();

    // Open video file
    if avformat_open_input(&mut format_ctx, path.as_ptr(), ptr::null_mut(), ptr::null_mut()) < 0 {
        return Err("Failed to open");
    }

    // Find stream info
    avformat_find_stream_info(format_ctx, ptr::null_mut());

    // Find video stream
    let mut decoder: *mut AVCodec = ptr::null_mut();
    let stream_index = av_find_best_stream(format_ctx, 0, -1, -1, &mut decoder, 0);

    // Allocate codec context
    let codec_ctx = avcodec_alloc_context3(decoder);

    // ... setup codec ...

    avcodec_open2(codec_ctx, decoder, ptr::null_mut());

    let mut frames = Vec::new();

    // Decode loop
    loop {
        let packet = av_packet_alloc();
        if av_read_frame(format_ctx, packet) < 0 {
            av_packet_free(&mut packet);
            break;  // EOF
        }

        avcodec_send_packet(codec_ctx, packet);

        loop {
            let frame = av_frame_alloc();
            let ret = avcodec_receive_frame(codec_ctx, frame);

            if ret < 0 {
                av_frame_free(&mut frame);
                break;
            }

            // Check if I-frame (ZERO disk I/O)
            if (*frame).pict_type == AV_PICTURE_TYPE_I {
                // Keep in memory, pass pointer
                frames.push(FrameBuffer {
                    width: (*frame).width as u32,
                    height: (*frame).height as u32,
                    data_ptr: (*frame).data[0],  // Direct pointer
                    linesize: (*frame).linesize[0] as usize,
                    _owner: frame,  // Keep AVFrame alive
                });
            } else {
                av_frame_free(&mut frame);
            }
        }

        av_packet_free(&mut packet);
    }

    Ok(frames)
}

pub struct FrameBuffer {
    pub width: u32,
    pub height: u32,
    pub data_ptr: *mut u8,
    pub linesize: usize,
    _owner: *mut AVFrame,  // Keeps memory alive
}

impl Drop for FrameBuffer {
    fn drop(&mut self) {
        unsafe { av_frame_free(&mut self._owner); }
    }
}
```

### 4. Zero-Copy to ONNX

```rust
pub fn run_yolo_zero_copy(frame: &FrameBuffer) -> Result<Vec<Detection>> {
    // Convert pointer to ndarray (zero-copy view)
    let img_array = unsafe {
        ndarray::ArrayView3::from_shape_ptr(
            (frame.height as usize, frame.width as usize, 3),
            frame.data_ptr
        )
    };

    // Preprocess (resize, normalize) - still zero-copy where possible
    let input_tensor = preprocess_zero_copy(img_array)?;

    // Run ONNX (operates on memory buffer directly)
    session.run(vec![input_tensor])?;

    // NO DISK ANYWHERE
}
```

### 5. Memory-Mapped Files (For Huge Videos)

```rust
use memmap2::Mmap;

// For >1GB files, mmap instead of read()
let file = File::open(video_path)?;
let mmap = unsafe { Mmap::map(&file)? };

// Pass mmap pointer to libavcodec
// Even 10GB files use zero RAM (OS pages them in)
```

## Performance Targets

**Keyframes**:
- FFmpeg CLI: 1.82s
- Target: 1.50s (18% FASTER)
- How: Zero disk I/O (save 200ms), direct C FFI (save 100ms)

**Object Detection Pipeline** (keyframes → YOLO):
- Current (with disk): 15s
- Target: 12s (20% faster)
- How: Zero-copy memory (no write/read 100 JPEGs)

**Transcription**:
- Already fast (whisper.cpp is C)
- Target: Match whisper CLI

**Embeddings**:
- Already using ONNX C API
- Optimize: Batch inference, GPU (CoreML)

## Worker Directive

**Implement in order**:

1. **Direct C FFI decoder** (2-3 commits)
   - File: crates/video-decoder/src/c_ffi.rs
   - Link libavcodec statically or dynamically
   - Decode to AVFrame* memory
   - Test: Verify frames extracted

2. **Zero-copy to ONNX** (2 commits)
   - File: crates/object-detection/src/lib.rs
   - Accept *mut u8 pointer instead of PathBuf
   - Use ndarray::ArrayView (zero-copy)
   - Test: YOLO works on memory buffer

3. **Integrate end-to-end** (1 commit)
   - Keyframe plugin calls C FFI decoder
   - Passes memory pointers to YOLO
   - No disk I/O in entire pipeline
   - Benchmark: Prove <1.8s

4. **Optimize further** (3-5 commits)
   - GPU decode (VideoToolbox on Mac)
   - Parallel frame decode (use all cores)
   - SIMD (use AVX2/NEON)
   - Memory pooling (reuse AVFrame buffers)

## Success Criteria

- Beat FFmpeg CLI: <1.8s (vs 1.82s)
- Beat pipelines: keyframes+YOLO in <12s (vs 15s)
- Zero disk I/O (all memory buffers)
- Tests: 98/98 passing
- Benchmark proves it

**Timeline**: 8-13 commits (~4-6 hours AI time)

**This is the path to world's best.** Worker: Implement C FFI decoder NOW.
