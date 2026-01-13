# HEIF/HEIC Support Investigation - N=238

**Date**: 2025-11-01
**Status**: Partial support (individual tiles decode, full Tile Grid composition requires stream group support)
**Priority**: ⭐⭐⭐⭐⭐⭐ CRITICAL (billions of iPhone photos)

---

## Executive Summary

**Finding**: FFmpeg 8.0 CAN decode HEIC files, but requires **stream group support** (FFmpeg 6.1+) to properly compose Tile Grid images. Current decoder implementation uses `av_find_best_stream()` which doesn't handle stream groups.

**Current Status**:
- ✅ FFmpeg ingestion recognizes HEIC files
- ✅ Individual tile streams (512x512) can be decoded
- ❌ Full Tile Grid images (e.g., 1280x720) require stream group composition
- ❌ Current decoder returns 0 keyframes for HEIC files

**Path Forward**: Implement stream group support in C FFI decoder (2-3 commits estimated)

---

## Investigation Details

### Test File Generation
```bash
# macOS `sips` tool generates proper HEIC with Tile Grid
ffmpeg -f lavfi -i testsrc=duration=1:size=1280x720:rate=1 -frames:v 1 /tmp/test_image.jpg
sips -s format heic /tmp/test_image.jpg --out test_edge_cases/image_iphone_photo.heic
```

**Result**:
- File type: `ISO Media, HEIF Image HEVC Main or Main Still Picture Profile`
- Size: 1280x720 (full image)
- Tile Grid: 6 streams × 512x512 tiles
- Frame type: `pict_type=I` (keyframe)

### FFmpeg Stream Analysis

```bash
$ ffprobe test_edge_cases/image_iphone_photo.heic 2>&1
```

**Output**:
```
Input #0, mov,mp4,m4a,3gp,3g2,mj2, from 'test_edge_cases/image_iphone_photo.heic':
  Duration: N/A, start: 0.000000, bitrate: N/A
  Stream group #0:0[0x7]: Tile Grid: hevc (Main Still Picture) (hvc1 / 0x31637668), yuvj420p(pc, smpte170m/unknown/unknown), 1280x720 (default)
    Stream #0:0[0x1], 1, 1/1: Video: hevc (Main Still Picture), 1 reference frame (hvc1 / 0x31637668), yuvj420p(pc, smpte170m/unknown/unknown, left), 512x512, 0/1, 1 fps, 1 tbr, 1 tbn (dependent)
    Stream #0:1[0x2], 1, 1/1: Video: hevc (Main Still Picture), 1 reference frame (hvc1 / 0x31637668), yuvj420p(pc, smpte170m/unknown/unknown, left), 512x512, 0/1, 1 fps, 1 tbr, 1 tbn (dependent)
    ... (4 more tile streams)
```

**Key observations**:
1. **Stream group #0:0**: "Tile Grid" with full 1280x720 resolution (what users expect)
2. **Streams #0:0 through #0:5**: Individual 512x512 tiles marked as `(dependent)`
3. **Stream group is NOT a regular stream**: Not accessible via `av_find_best_stream()`

### Decoder Behavior Test

```bash
$ VIDEO_EXTRACT_THREADS=4 ./target/release/video-extract fast --op keyframes test_edge_cases/image_iphone_photo.heic
[mov,mp4,m4a,3gp,3g2,mj2 @ 0x15c904560] Ignoring duplicate CLLI/COLL
... (6 warnings about CLLI/COLL metadata)
Extracted 0 keyframes to ./fast_output
✓ Completed in 0.003s (FFmpeg: 0.003s, overhead: 0ms)
```

**Result**: 0 keyframes extracted (decoder doesn't see stream group)

### FFmpeg CLI Comparison

```bash
$ ffmpeg -i test_edge_cases/image_iphone_photo.heic -frames:v 1 test.jpg 2>&1
Stream mapping:
  Stream #0:0 -> #0:0 (hevc (native) -> mjpeg (native))
Output #0, image2, to 'test.jpg':
  Stream #0:0: Video: mjpeg, yuvj420p, 512x512, q=2-31, 200 kb/s, 1 fps, 1 tbn (dependent)
```

**Result**: FFmpeg CLI decodes stream #0:0 (first tile, 512x512) successfully but **NOT the full Tile Grid** (1280x720)

---

## Technical Analysis

### Current Decoder Implementation

**File**: `crates/video-decoder/src/c_ffi.rs`

```rust
pub fn decode_iframes_yuv(video_path: &Path) -> Result<Vec<YuvFrame>> {
    unsafe {
        let format_ctx = FormatContext::open(video_path)?;
        let (stream_index, decoder) = format_ctx.find_video_stream()?;  // ← Issue here
        // ...
    }
}
```

**`find_video_stream()` implementation** (line 438):
```rust
pub fn find_video_stream(&self) -> Result<(c_int, *const AVCodec)> {
    unsafe {
        let mut decoder: *const AVCodec = ptr::null();
        let stream_index = av_find_best_stream(
            self.ptr,
            AVMEDIA_TYPE_VIDEO,
            -1, -1,
            &mut decoder,
            0
        );  // ← av_find_best_stream() does NOT find stream groups
        // ...
    }
}
```

**Problem**: `av_find_best_stream()` finds regular video streams but **NOT stream groups** (Tile Grid).

### FFmpeg Stream Groups API (FFmpeg 6.1+)

**Stream groups** (`AVStreamGroup`) were added in FFmpeg 6.1 (2023-05-25) to support:
- HEIF/HEIC Tile Grid
- IAMF (Immersive Audio)
- Multi-view video

**Required API changes**:
```c
// Check if stream groups exist
if (fmt_ctx->nb_stream_groups > 0) {
    AVStreamGroup *group = fmt_ctx->stream_groups[0];

    // For Tile Grid:
    if (group->type == AV_STREAM_GROUP_PARAMS_TILE_GRID) {
        AVStreamGroupTileGrid *tile_grid = &group->params.tile_grid;
        // Access: tile_grid->coded_width, tile_grid->coded_height
        // Access: tile_grid->nb_tiles, tile_grid->offsets
    }
}
```

**Decoding approach**:
1. Detect stream group type (Tile Grid)
2. Decode all tile streams (Streams #0:0 through #0:5)
3. Compose tiles into full image using offsets from `AVStreamGroupTileGrid`

---

## Implementation Plan

### Commit 1: Add Stream Group Detection (1-2 hours)

**Changes**:
1. Update `FormatContext` wrapper to expose stream groups
2. Add `find_stream_group()` method for Tile Grid detection
3. Modify `decode_iframes_yuv()` to check for stream groups first

**Files**:
- `crates/video-decoder/src/c_ffi.rs` (FormatContext, decode_iframes_yuv)

**Pseudo-code**:
```rust
pub fn decode_iframes_yuv(video_path: &Path) -> Result<Vec<YuvFrame>> {
    unsafe {
        let format_ctx = FormatContext::open(video_path)?;

        // Check for stream groups first (HEIF/HEIC Tile Grid)
        if let Some(tile_grid) = format_ctx.find_tile_grid()? {
            return decode_tile_grid(format_ctx, tile_grid);
        }

        // Fall back to regular video stream
        let (stream_index, decoder) = format_ctx.find_video_stream()?;
        // ... existing code
    }
}
```

### Commit 2: Implement Tile Grid Decoding (2-3 hours)

**Changes**:
1. Add `decode_tile_grid()` function
2. Decode all tile streams in parallel
3. Compose tiles into full image using tile offsets

**Complexity**:
- Multiple stream decoding (already supported by decoder loop)
- Tile composition requires copying YUV planes with correct offsets
- Need to handle different tile sizes and layouts

**Pseudo-code**:
```rust
unsafe fn decode_tile_grid(
    format_ctx: FormatContext,
    tile_grid: TileGridInfo,
) -> Result<Vec<YuvFrame>> {
    let tiles = Vec::new();

    // Decode each tile stream
    for tile_stream_idx in tile_grid.stream_indices {
        let tile_frame = decode_single_tile(format_ctx, tile_stream_idx)?;
        tiles.push(tile_frame);
    }

    // Compose tiles into full image
    let full_frame = compose_tiles(tiles, tile_grid.layout)?;

    Ok(vec![full_frame])
}
```

### Commit 3: Add Tests and Documentation (30 min)

**Changes**:
1. Add smoke test for HEIC format (test_edge_cases/image_iphone_photo.heic)
2. Update UNSUPPORTED_FORMATS_RESEARCH.md (mark HEIC as ✅ SUPPORTED)
3. Update README.md (add HEIC/HEIF to supported formats)

**Test**:
```rust
#[test]
#[ignore]
fn smoke_format_heic() {
    // HEIC iPhone photo (1280x720 Tile Grid with 6×512x512 tiles)
    test_format("test_edge_cases/image_iphone_photo.heic", "keyframes");

    // Verify 1 keyframe extracted (full image composition)
    let output_dir = Path::new("./fast_output");
    let frames: Vec<_> = std::fs::read_dir(output_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(frames.len(), 1, "Expected 1 keyframe for HEIC image");
}
```

---

## Estimated Effort

**Total**: 3 commits, 4-6 hours

| Task | Commits | Hours | Status |
|------|---------|-------|--------|
| Stream group detection | 1 | 1-2 | ⏳ Not started |
| Tile Grid decoding | 1 | 2-3 | ⏳ Not started |
| Tests + docs | 1 | 0.5-1 | ⏳ Not started |
| **TOTAL** | **3** | **4-6** | - |

**Complexity**: MEDIUM (requires new FFmpeg API, tile composition logic)

---

## Alternative Approach: Use `libheif` Library

**Pros**:
- Dedicated HEIF library with full spec support
- Handles Tile Grid, overlays, transformations automatically
- Rust bindings available (`libheif-rs` crate)

**Cons**:
- New dependency (adds ~2MB to binary)
- Separate code path from FFmpeg (maintenance burden)
- FFmpeg already supports HEIF (just needs stream group handling)

**Recommendation**: Prefer FFmpeg stream group support (already integrated, no new dependencies)

---

## Compatibility Notes

### FFmpeg Versions
- **FFmpeg 6.1+**: Stream groups API available
- **FFmpeg 7.0+**: Improved HEIF support
- **FFmpeg 8.0** (current): Full HEIF decode support with stream groups

**Verification**:
```bash
$ ffmpeg -version | head -1
ffmpeg version 8.0
$ ffmpeg -formats 2>&1 | grep -i heif
# (No "heif" muxer, decode-only as expected)
```

### Image Formats Summary

| Format | Extension | Current Support | Action |
|--------|-----------|----------------|--------|
| JPEG | .jpg, .jpeg | ✅ Full | None |
| PNG | .png | ✅ Full | None |
| GIF | .gif | ✅ Full | None |
| TIFF | .tif, .tiff | ✅ Full | None |
| HEIF/HEIC | .heif, .heic | ⚠️ Partial (tiles only) | Implement stream groups |
| WebP | .webp | ✅ Full | None |

---

## References

- FFmpeg Stream Groups: https://ffmpeg.org/doxygen/trunk/structAVStreamGroup.html
- HEIF Specification: ISO/IEC 23008-12
- FFmpeg 6.1 Release Notes: https://ffmpeg.org/index.html#news (2023-05-25)
- Test file: `test_edge_cases/image_iphone_photo.heic` (1280x720, 6 tiles)

---

## Status: Ready for Implementation

**Next steps for N=239**:
1. Implement stream group detection (Commit 1)
2. Implement Tile Grid decoding (Commit 2)
3. Add tests and documentation (Commit 3)

**Blockers**: None (FFmpeg 8.0 supports stream groups)

**Alternative**: If stream group implementation proves too complex, fall back to `libheif-rs` crate (add 1-2 commits)
