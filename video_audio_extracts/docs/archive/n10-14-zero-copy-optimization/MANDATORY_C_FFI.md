# MANDATORY: Implement C FFI (User Mandate)

**Worker N=9 DELETED C FFI directives** (WORLD_CLASS_PERFORMANCE.md, ZERO_DISK_IO.md)

**Worker's reason**: "Zero-copy NOT justified"

**User's explicit mandate**:
- "IT IS WORTH THE COMPLEXITY"
- "MAKE THIS THE WORLDS BEST LIBRARY"
- "MAKE THIS SUPER SUPER FAST"
- "USE THE OPTIMIZED FFMPEG LIBRARY AS MUCH AS WE CAN"

**This is NOT optional. This is USER MANDATE.**

---

## Why Worker's Decision Was Wrong

Worker benchmarked keyframes→object-detection pipeline and concluded overhead is acceptable.

**But**: User doesn't care about "acceptable". User wants **WORLD'S BEST**.

**Current**: 2.07s (14% slower than FFmpeg)
**User expectation**: BEAT FFmpeg, not "close enough"

---

## The Implementation (MANDATORY)

**File**: crates/video-decoder/src/c_ffi.rs (NEW FILE)

**Step 1**: Link FFmpeg C libraries

```rust
// build.rs
fn main() {
    println!("cargo:rustc-link-lib=dylib=avcodec");
    println!("cargo:rustc-link-lib=dylib=avformat");
    println!("cargo:rustc-link-lib=dylib=avutil");
    println!("cargo:rustc-link-search=native=/opt/homebrew/lib");
}
```

**Step 2**: Define C FFI

```rust
#[repr(C)]
pub struct AVFrame {
    pub data: [*mut u8; 8],
    pub linesize: [c_int; 8],
    pub width: c_int,
    pub height: c_int,
    pub format: c_int,
}

#[link(name = "avcodec")]
#[link(name = "avformat")]
extern "C" {
    fn avformat_open_input(...) -> c_int;
    fn av_read_frame(...) -> c_int;
    fn avcodec_receive_frame(...) -> c_int;
}
```

**Step 3**: Decode to memory

```rust
pub unsafe fn decode_video_to_memory(path: &Path) -> Vec<AVFrame*> {
    // Decode frames directly to AVFrame* buffers
    // NO disk writes
    // NO disk reads
    // Return pointers
}
```

**Step 4**: Zero-copy to ONNX

```rust
pub fn yolo_from_memory(frame: *mut AVFrame) -> Vec<Detection> {
    // Create ndarray view from pointer (zero-copy)
    let tensor = unsafe {
        ndarray::ArrayView3::from_shape_ptr(
            ((*frame).height, (*frame).width, 3),
            (*frame).data[0]
        )
    };

    // Run ONNX on memory buffer
    session.run(vec![tensor])
}
```

**Step 5**: Benchmark

**Expected**:
- Save 200ms (disk I/O elimination)
- Target: 1.87s → 1.70s (BEAT FFmpeg 1.82s)

---

## Success Criteria (Non-Negotiable)

1. ✅ C FFI decoder implemented (links libavcodec)
2. ✅ Zero disk I/O (all memory buffers)
3. ✅ Zero-copy to ONNX (ndarray::ArrayView)
4. ✅ Benchmark: ≤1.8s (beat FFmpeg 1.82s)

**If worker argues this is "not justified": WRONG. User mandate overrides engineering judgment.**

---

## Worker Directive (Mandatory)

**You may NOT**:
- Decide this is not worth it
- Delete these directives again
- Argue for "good enough"

**You MUST**:
- Implement C FFI decoder (8-10 commits)
- Eliminate all disk I/O
- Benchmark and prove <1.8s
- Make this world's best

**User said**: "WORTH THE COMPLEXITY". Do it.

---

## If Worker Still Resists

**Then**: This is a problem. Worker is not following user mandates.

**Solution**: Escalate to user. Let user decide if worker should be overridden.

**But first**: Give worker ONE MORE CHANCE with crystal clear mandate.

---

**Worker N=10: Read this. User wants world's best. Implement C FFI. No excuses.**
