# Format Support Gaps - N=251

**Date:** 2025-11-13
**Issue:** Test files exist for professional video formats but plugins don't support them yet

## Context

During test expansion work (N=251), attempted to add test coverage for professional broadcast formats (GXF, F4V, DPX) that have test files in `test_files_video_formats_dpx_gxf_f4v/` and `test_files_professional_video_gxf/`.

FFmpeg can decode these formats successfully, but the video-extract plugin system does not yet recognize them as valid input formats.

## Unsupported Formats with Available Test Files

### 1. GXF (General eXchange Format)
- **Test files:** 5 files in `test_files_professional_video_gxf/`
  - 01_gxf_pal.gxf (533KB)
  - 02_gxf_pal_mandelbrot.gxf (4.5MB)
  - 03_gxf_ntsc_smpte.gxf (135KB)
  - 04_gxf_rgb_test.gxf (97KB)
  - 05_gxf_solid_color.gxf (45KB)
- **Status:** FFmpeg can decode (format: `gxf`, codec: `mpeg2video`)
- **Error:** `Plugin keyframes does not support input type: gxf`
- **Use case:** Professional broadcast industry standard

### 2. F4V (Flash Video MP4)
- **Test files:** 5 files in `test_files_video_formats_dpx_gxf_f4v/`
  - 01_f4v_h264.f4v (32KB)
  - 02_f4v_mandelbrot.f4v (3.3MB)
  - 03_f4v_smpte.f4v (9.9KB)
  - 04_f4v_rgb.f4v (19KB)
  - 05_f4v_solid.f4v (4.5KB)
- **Status:** FFmpeg can decode (format: `mov,mp4,m4a,3gp,3g2,mj2`, H.264)
- **Error:** `Plugin keyframes does not support input type: f4v`
- **Use case:** Flash Video with H.264 codec
- **Note:** Listed in FORMAT_CONVERSION_STATUS.md

### 3. DPX (Digital Picture Exchange)
- **Test files:** 4 files in `test_files_video_formats_dpx_gxf_f4v/`
  - 01_dpx_testsrc.dpx (5.9MB)
  - 02_dpx_mandelbrot.dpx (6.3MB)
  - 04_dpx_smpte.dpx (5.9MB)
  - 05_dpx_gray.dpx (5.9MB)
- **Status:** Professional image format (image sequences)
- **Error:** Likely `Plugin keyframes does not support input type: dpx`
- **Use case:** Professional film/VFX workflows

## Root Cause

The plugin system uses format detection to determine which plugins can handle which input formats. These professional formats are not registered as supported input types in the format detection logic.

## Verification Commands

```bash
# Test GXF
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH"
export PKG_CONFIG_PATH="/opt/homebrew/lib/pkgconfig:/opt/homebrew/opt/ffmpeg/lib/pkgconfig"
./target/release/video-extract debug --ops keyframes --output-dir debug_output test_files_professional_video_gxf/01_gxf_pal.gxf

# Test F4V
./target/release/video-extract debug --ops keyframes --output-dir debug_output test_files_video_formats_dpx_gxf_f4v/01_f4v_h264.f4v

# Verify FFmpeg can decode GXF
ffprobe test_files_professional_video_gxf/01_gxf_pal.gxf
# Output: Input #0, gxf, from '...'
#   Stream #0:0: Video: mpeg2video (Main), yuv420p(tv, progressive), 720x480, 59.94 fps
```

## Required Work

To add support for these formats:

1. **Identify format detection logic** - Find where input formats are defined (enum/struct)
2. **Add format variants** - Add `Gxf`, `F4v`, `Dpx` to supported format enum
3. **Update format detection** - Extend format detection to recognize these extensions and FFmpeg format names
4. **Update each plugin** - Add these formats to supported_formats() for relevant plugins:
   - keyframes (all three formats)
   - All vision plugins (for GXF and F4V video formats)
   - Image plugins (for DPX image format)
5. **Add tests** - Once support is added, create comprehensive test coverage:
   - GXF: 5 files × 8 vision plugins = 40 tests
   - F4V: 5 files × 24 plugins = 120 tests
   - DPX: 4 files × 8 vision plugins = 32 tests
   - **Total:** 192 professional format tests

## Test Coverage Gap

Current test suite: 878 tests
Blocked tests due to missing format support: 192 tests (GXF + F4V + DPX)
Potential total after implementation: 1,070 tests

## Priority

**Medium-High**

These are professional broadcast and production formats with real-world use cases. Test files are already available. The main work is registering the formats in the plugin system's format detection logic.

## Next Steps

1. Search codebase for format detection logic (look for "mp4", "mkv", "webm" format handling)
2. Add GXF, F4V, DPX to format enum and detection
3. Update plugin supported_formats() methods
4. Add 192 comprehensive tests
5. Verify with smoke test suite

## Related Files

- `docs/FORMAT_CONVERSION_STATUS.md` - Documents F4V as recognized but not yet converted
- `test_files_professional_video_gxf/` - GXF test files (5.3MB total)
- `test_files_video_formats_dpx_gxf_f4v/` - F4V and DPX test files (27.5MB total)
- `tests/smoke_test_comprehensive.rs` - Where tests would be added once support exists

## Success Criteria

- GXF files process successfully with keyframes plugin
- F4V files process successfully with all 24 plugins
- DPX files process successfully with vision plugins
- All 192 new tests pass
- Documentation updated
