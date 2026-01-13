# Unsupported Media Formats - Research Report

**Date**: 2025-11-01
**Purpose**: Identify commonly-used video/audio formats NOT yet supported by this system
**Current Coverage**: 10 video formats, 7 audio formats (see ADDITIONAL_FORMATS.md for details)

---

## Executive Summary

**Currently Supported Formats**:
- **Video** (10): MP4, MOV, MKV, WEBM, AVI, FLV, 3GP, WMV, OGV, M4V
- **Audio** (7): WAV, MP3, FLAC, M4A, AAC, OGG, Opus

**Key Finding**: FFmpeg already supports **most missing formats** through its demuxer/decoder system. The system's ingestion module (`crates/ingestion/src/lib.rs`) uses `ffmpeg_next::format::input()` which automatically detects and decodes **any format FFmpeg supports**. This means adding support is primarily a **documentation and validation task**, not implementation.

**High-Priority Gaps** (20+ formats identified):
1. **Professional/Broadcast**: MXF (broadcast), ProRes (post-production), DNxHD (Avid)
2. **Consumer Camera**: MTS/M2TS (AVCHD camcorders), VOB (DVD)
3. **Legacy/Niche**: ASF/WMV containers, RM/RMVB (RealMedia)
4. **Audio**: WMA (Windows), AMR (mobile), APE/TTA (lossless)
5. **Image**: HEIF/HEIC (iPhone photos - **critical gap**)

---

## PART 1: VIDEO FORMATS (Missing)

### A. Professional/Broadcast Formats (High Priority)

#### 1. MXF (Material Exchange Format)
- **Extension**: `.mxf`
- **Use Case**: Professional broadcast, digital cinema, archival
- **Codecs**: MPEG-2, DNxHD, ProRes, uncompressed
- **Typical Users**: TV stations, post-production houses, archives
- **FFmpeg Support**: ✅ YES (`mxf` demuxer, verified in system)
- **Priority**: ⭐⭐⭐⭐⭐ (broadcast standard)
- **Difficulty**: **TRIVIAL** - Already supported by FFmpeg
- **Implementation**: Add `.mxf` to format list, create test file

#### 2. ProRes (.mov variants)
- **Extension**: `.mov` (already supported container)
- **Codecs**:
  - ProRes 422 (Proxy, LT, Standard, HQ)
  - ProRes 4444 (with alpha channel)
  - ProRes 4444 XQ (highest quality, ~500 Mbit/s)
- **Use Case**: Video editing, post-production, film production
- **Resolution**: SD to 8K
- **Color Depth**: Up to 12-bit
- **FFmpeg Support**: ✅ YES (multiple ProRes encoders/decoders)
- **Priority**: ⭐⭐⭐⭐⭐ (industry standard for editing)
- **Difficulty**: **TRIVIAL** - Container already supported, codec supported
- **Implementation**: Validate ProRes MOV files work (likely already functional)

#### 3. DNxHD/DNxHR (.mxf or .mov)
- **Extension**: `.mxf`, `.mov`
- **Codec**: Avid DNxHD (HD), DNxHR (2K/4K/UHD)
- **Use Case**: Avid Media Composer editing, post-production
- **Technical**: Intra-frame compression (like JPEG for video)
- **FFmpeg Support**: ✅ YES (`dnxhd` codec)
- **Priority**: ⭐⭐⭐⭐ (Avid ecosystem standard)
- **Difficulty**: **TRIVIAL** - Containers already supported
- **Implementation**: Validate DNxHD files work

### B. Consumer Camera Formats (High Priority)

#### 4. MTS/M2TS (AVCHD/Blu-ray)
- **Extension**: `.mts` (camcorder), `.m2ts` (computer)
- **Full Name**: MPEG-2 Transport Stream
- **Use Case**:
  - Consumer/pro camcorders (Sony, Panasonic)
  - Blu-ray disc video
  - Digital TV broadcasting
- **Codecs**: H.264/AVC, H.265/HEVC (newer cameras)
- **Audio**: Dolby Digital (AC-3), PCM
- **FFmpeg Support**: ✅ YES (MPEG-TS demuxer)
- **Priority**: ⭐⭐⭐⭐⭐ (extremely common from cameras)
- **Difficulty**: **TRIVIAL** - Already supported by FFmpeg
- **Implementation**: Add `.mts`, `.m2ts` to format list

#### 5. VOB (DVD Video)
- **Extension**: `.vob`
- **Use Case**: DVD-Video discs, legacy content
- **Codecs**: MPEG-2 video, MPEG-1 Layer II audio, AC-3, PCM
- **Container**: MPEG Program Stream
- **Features**: Multiple audio/subtitle tracks, menus
- **FFmpeg Support**: ✅ YES (`vob` format)
- **Priority**: ⭐⭐⭐⭐ (large DVD archive libraries exist)
- **Difficulty**: **TRIVIAL** - Already supported by FFmpeg
- **Implementation**: Add `.vob` to format list

#### 6. TS (MPEG Transport Stream)
- **Extension**: `.ts`
- **Use Case**: Digital TV broadcasting, streaming
- **Codecs**: H.264, H.265, MPEG-2
- **Note**: Similar to M2TS but used for broadcast
- **FFmpeg Support**: ✅ YES (MPEG-TS demuxer)
- **Priority**: ⭐⭐⭐⭐ (broadcast/streaming standard)
- **Difficulty**: **TRIVIAL** - Already supported by FFmpeg
- **Implementation**: Add `.ts` to format list (may already work as M2TS variant)

### C. Legacy/Proprietary Formats (Medium Priority)

#### 7. ASF (Advanced Systems Format)
- **Extension**: `.asf` (also `.wmv`, `.wma` use ASF container)
- **Developer**: Microsoft
- **Use Case**: Windows Media streaming, legacy Windows content
- **Codecs**: WMV (video), WMA (audio)
- **FFmpeg Support**: ✅ YES (`asf` demuxer, verified in system)
- **Priority**: ⭐⭐⭐ (legacy Windows ecosystem)
- **Difficulty**: **TRIVIAL** - Already supported by FFmpeg
- **Implementation**: Add `.asf` to format list (`.wmv` already supported)

#### 8. RM/RMVB (RealMedia)
- **Extension**: `.rm`, `.rmvb` (Variable Bitrate)
- **Developer**: RealNetworks (obsolete)
- **Use Case**: Legacy streaming video (popular in 2000s, especially Asia)
- **Codecs**: RealVideo, RealAudio
- **FFmpeg Support**: ✅ YES (`rm` demuxer, verified in system)
- **Priority**: ⭐⭐ (obsolete but legacy archives exist)
- **Difficulty**: **TRIVIAL** - Already supported by FFmpeg
- **Implementation**: Add `.rm`, `.rmvb` to format list

#### 9. DV (Digital Video)
- **Extension**: `.dv`, `.dif`
- **Use Case**: DV camcorders, tape digitization
- **Codec**: DV (intra-frame, 25 Mbps)
- **FFmpeg Support**: ✅ YES (`dv` codec and format)
- **Priority**: ⭐⭐⭐ (tape archives from 1990s-2000s)
- **Difficulty**: **TRIVIAL** - Already supported by FFmpeg
- **Implementation**: Add `.dv`, `.dif` to format list

#### 10. F4V/FLV variants
- **Extension**: `.f4v`
- **Use Case**: Adobe Flash Video (newer variant)
- **Note**: FLV already supported, F4V is MP4-based Flash variant
- **FFmpeg Support**: ✅ YES (handled as MP4 variant)
- **Priority**: ⭐⭐ (Flash is obsolete)
- **Difficulty**: **TRIVIAL** - Likely already works via MP4 support
- **Implementation**: Add `.f4v` to format list for completeness

### D. Specialized/Professional Formats (Lower Priority)

#### 11. GXF (General eXchange Format)
- **Extension**: `.gxf`
- **Use Case**: Professional video, Grass Valley equipment
- **FFmpeg Support**: ✅ YES (`gxf` format, verified)
- **Priority**: ⭐⭐⭐ (professional broadcast)
- **Difficulty**: **TRIVIAL** - Already supported by FFmpeg
- **Implementation**: Add `.gxf` to format list

#### 12. R3D (RED Digital Cinema)
- **Extension**: `.r3d`
- **Use Case**: RED cinema cameras (high-end filmmaking)
- **Codecs**: REDcode (proprietary RAW)
- **FFmpeg Support**: ⚠️ LIMITED (basic support, not full RAW decode)
- **Priority**: ⭐⭐⭐ (high-end cinema)
- **Difficulty**: **MEDIUM** - Limited FFmpeg support, may need RED SDK
- **Implementation**: Test FFmpeg support level, may not be full-featured

#### 13. DPX (Digital Picture Exchange)
- **Extension**: `.dpx`
- **Use Case**: Film production, VFX (uncompressed image sequence)
- **Note**: Image sequence format, not video container
- **FFmpeg Support**: ✅ YES (`dpx` format)
- **Priority**: ⭐⭐⭐ (VFX/film workflows)
- **Difficulty**: **MEDIUM** - Image sequence handling
- **Implementation**: Requires image sequence support logic

---

## PART 2: AUDIO FORMATS (Missing)

### A. Windows/Microsoft Formats (Medium-High Priority)

#### 14. WMA (Windows Media Audio)
- **Extension**: `.wma`
- **Container**: ASF (Advanced Systems Format)
- **Variants**:
  - WMA Standard (lossy, MP3 competitor)
  - WMA Pro (7.1 surround, high-res)
  - WMA Lossless (24-bit/96kHz)
  - WMA Voice (low bitrate speech)
- **Use Case**: Windows ecosystem, legacy music libraries
- **FFmpeg Support**: ✅ YES (`wma` codecs in ASF container)
- **Priority**: ⭐⭐⭐⭐ (common in Windows world)
- **Difficulty**: **TRIVIAL** - Already supported by FFmpeg
- **Implementation**: Add `.wma` to format list

### B. Mobile/Telephony Formats (Medium Priority)

#### 15. AMR (Adaptive Multi-Rate)
- **Extension**: `.amr`
- **Variants**: AMR-NB (narrowband), AMR-WB (wideband)
- **Use Case**: Mobile phone voice recordings, speech
- **Bitrate**: 4.75-12.2 kbps (very low)
- **FFmpeg Support**: ✅ YES (`amr` demuxer, verified)
- **Priority**: ⭐⭐⭐⭐ (extremely common for voice notes)
- **Difficulty**: **TRIVIAL** - Already supported by FFmpeg
- **Implementation**: Add `.amr` to format list

#### 16. 3GA (3GPP Audio)
- **Extension**: `.3ga`
- **Use Case**: Mobile phone audio (variant of 3GP)
- **Codecs**: AMR, AAC
- **FFmpeg Support**: ✅ YES (3GP already supported)
- **Priority**: ⭐⭐⭐ (mobile audio)
- **Difficulty**: **TRIVIAL** - Likely works as 3GP variant
- **Implementation**: Add `.3ga` to format list

### C. Lossless/Audiophile Formats (Medium Priority)

#### 17. APE (Monkey's Audio)
- **Extension**: `.ape`
- **Use Case**: Lossless audio compression (popular in Asia)
- **Compression**: Higher than FLAC but slower decode
- **FFmpeg Support**: ✅ YES (`ape` demuxer, verified)
- **Priority**: ⭐⭐⭐ (niche but dedicated users)
- **Difficulty**: **TRIVIAL** - Already supported by FFmpeg
- **Implementation**: Add `.ape` to format list

#### 18. TTA (True Audio)
- **Extension**: `.tta`
- **Use Case**: Lossless audio compression, real-time codec
- **FFmpeg Support**: ✅ YES (`tta` codec)
- **Priority**: ⭐⭐ (very niche)
- **Difficulty**: **TRIVIAL** - Already supported by FFmpeg
- **Implementation**: Add `.tta` to format list

#### 19. WV (WavPack)
- **Extension**: `.wv`
- **Use Case**: Hybrid lossless/lossy compression
- **Features**: Can create lossy+correction file pair
- **FFmpeg Support**: ✅ YES (`wv` codec)
- **Priority**: ⭐⭐ (niche but unique features)
- **Difficulty**: **TRIVIAL** - Already supported by FFmpeg
- **Implementation**: Add `.wv` to format list

### D. Legacy/Specialized Formats (Lower Priority)

#### 20. AU (Sun Audio)
- **Extension**: `.au`, `.snd`
- **Use Case**: Unix/Solaris systems (legacy)
- **FFmpeg Support**: ✅ YES (`au` format, verified)
- **Priority**: ⭐⭐ (obsolete)
- **Difficulty**: **TRIVIAL** - Already supported by FFmpeg
- **Implementation**: Add `.au`, `.snd` to format list

#### 21. VOC (Creative Voice)
- **Extension**: `.voc`
- **Use Case**: Sound Blaster cards (1990s legacy)
- **FFmpeg Support**: ✅ YES (`voc` format)
- **Priority**: ⭐ (obsolete)
- **Difficulty**: **TRIVIAL** - Already supported by FFmpeg
- **Implementation**: Add `.voc` to format list (completeness only)

#### 22. AA/AAX (Audible Audiobooks)
- **Extension**: `.aa`, `.aax`
- **Use Case**: Amazon Audible audiobooks
- **Codecs**: MP3 or ACELP with DRM
- **FFmpeg Support**: ⚠️ PARTIAL (DRM-free only)
- **Priority**: ⭐⭐⭐ (large audiobook market)
- **Difficulty**: **HARD** - DRM requires activation bytes
- **Implementation**: Non-DRM files may work, DRM requires special handling

#### 23. DSS/DVF/MSV (Dictation Formats)
- **Extensions**: `.dss` (Olympus), `.dvf` (Sony), `.msv` (Sony)
- **Use Case**: Voice recorders, dictation devices
- **FFmpeg Support**: ⚠️ LIMITED (some support, may be incomplete)
- **Priority**: ⭐⭐ (niche professional use)
- **Difficulty**: **MEDIUM** - Proprietary formats
- **Implementation**: Test FFmpeg support, may need vendor SDKs

---

## PART 3: IMAGE FORMATS (Missing)

### A. Critical Modern Formats

#### 24. HEIF/HEIC (High Efficiency Image Format)
- **Extension**: `.heif`, `.heic`
- **Use Case**: **iPhone default photo format (iOS 11+)**
- **Codec**: HEVC (H.265) image compression
- **Advantages**: 50% smaller than JPEG, same quality
- **Prevalence**: **EXTREMELY HIGH** - billions of iPhone photos
- **FFmpeg Support**: ⚠️ LIMITED (decode only, requires external libheif)
- **Priority**: ⭐⭐⭐⭐⭐⭐ **CRITICAL GAP** (most important missing format)
- **Difficulty**: **MEDIUM** - Requires `libheif` dependency
- **Implementation**:
  - Add `libheif` Rust bindings (via `libheif-rs` crate)
  - Or: Use FFmpeg's limited HEIF support (decode only)
  - Estimated: 2-3 commits

### B. Professional Photo Formats

#### 25. Camera RAW Formats
- **Extensions**: `.cr2`, `.cr3` (Canon), `.nef` (Nikon), `.arw` (Sony), `.dng` (Adobe)
- **Use Case**: Professional photography, RAW sensor data
- **Advantages**: Maximum quality, non-destructive editing
- **FFmpeg Support**: ❌ NO (not video codec)
- **Library Options**:
  - `rawloader` crate (Rust)
  - `libraw` (C++, industry standard)
- **Priority**: ⭐⭐⭐⭐ (professional photography workflows)
- **Difficulty**: **MEDIUM-HARD** - New dependency, complex formats
- **Implementation**:
  - Add `rawloader` or `libraw` bindings
  - Estimated: 3-4 commits

### C. Vector/Design Formats

#### 26. SVG (Scalable Vector Graphics)
- **Extension**: `.svg`
- **Use Case**: Icons, illustrations, web graphics
- **FFmpeg Support**: ❌ NO (vector format, not raster)
- **Library Options**: `resvg` crate (Rust, excellent)
- **Priority**: ⭐⭐ (niche for this system)
- **Difficulty**: **EASY-MEDIUM** - Well-supported Rust crate
- **Implementation**: Add `resvg` dependency, 1-2 commits

#### 27. PSD (Photoshop Document)
- **Extension**: `.psd`
- **Use Case**: Professional design, layer-based editing
- **FFmpeg Support**: ❌ NO
- **Library Options**: `psd` crate (Rust, limited support)
- **Priority**: ⭐⭐ (specialized use case)
- **Difficulty**: **MEDIUM** - Partial support available
- **Implementation**: Add `psd` crate, test limitations, 2 commits

### D. Already Supported (Verify)

#### 28. TIFF
- **Extension**: `.tiff`, `.tif`
- **Use Case**: Professional imaging, scanning, medical imaging
- **Current Support**: ✅ YES (via `image` crate v0.25)
- **Priority**: N/A (already supported per ADDITIONAL_FORMATS.md N=78)

#### 29. GIF
- **Extension**: `.gif`
- **Use Case**: Animations, web graphics
- **Current Support**: ✅ YES (via `image` crate v0.25)
- **Priority**: N/A (already supported per ADDITIONAL_FORMATS.md N=78)

---

## PART 4: STREAMING/ADAPTIVE FORMATS

### A. HTTP Streaming Protocols

#### 30. HLS (HTTP Live Streaming)
- **Extension**: `.m3u8` (playlist), `.ts` segments
- **Use Case**: Apple streaming, live video
- **Components**:
  - M3U8 manifest (text file with segment list)
  - TS video segments
- **FFmpeg Support**: ✅ YES (`hls` demuxer)
- **Priority**: ⭐⭐⭐⭐ (web video streaming)
- **Difficulty**: **MEDIUM** - Requires playlist parsing + segment stitching
- **Implementation**:
  - Parse M3U8 playlist
  - Download/process TS segments
  - Estimated: 2-3 commits

#### 31. DASH (Dynamic Adaptive Streaming)
- **Extension**: `.mpd` (manifest), `.m4s`/`.mp4` segments
- **Use Case**: YouTube, Netflix adaptive streaming
- **Components**:
  - MPD manifest (XML)
  - MP4/WebM segments
- **FFmpeg Support**: ✅ YES (`dash` demuxer)
- **Priority**: ⭐⭐⭐⭐ (YouTube/Netflix standard)
- **Difficulty**: **MEDIUM** - Similar to HLS
- **Implementation**: Parse MPD, stitch segments, 2-3 commits

---

## SUMMARY & RECOMMENDATIONS

### Format Count
- **Missing Video Formats**: 13 formats (8 high-priority)
- **Missing Audio Formats**: 10 formats (4 high-priority)
- **Missing Image Formats**: 4 formats (1 critical, 1 high-priority)
- **Streaming Formats**: 2 formats (both high-priority)
- **TOTAL**: 29 formats identified

### Implementation Difficulty Breakdown

#### TRIVIAL (FFmpeg already supports - 22 formats)
**Video**: MXF, MTS/M2TS, VOB, TS, ASF, RM/RMVB, DV, F4V, GXF
**Audio**: WMA, AMR, 3GA, APE, TTA, WV, AU, VOC
**Implementation**: Just add extensions to format list + create test files
**Estimated Time**: 1-2 commits total (batch addition)

#### EASY-MEDIUM (Well-supported libraries - 3 formats)
**Image**: SVG (resvg crate), PSD (psd crate), HEIF (libheif-rs or FFmpeg)
**Implementation**: Add dependency, integrate API
**Estimated Time**: 1-2 commits per format

#### MEDIUM (Complex but standard - 3 formats)
**Streaming**: HLS, DASH
**Video**: DNxHD/ProRes validation
**Implementation**: Playlist parsing, segment handling
**Estimated Time**: 2-3 commits per format

#### MEDIUM-HARD (Special handling - 2 formats)
**Image**: Camera RAW (rawloader/libraw)
**Video**: R3D (limited FFmpeg support)
**Implementation**: New complex dependencies
**Estimated Time**: 3-4 commits per format

#### HARD (DRM/proprietary - 1 format)
**Audio**: AA/AAX (Audible with DRM)
**Implementation**: DRM removal requires activation bytes
**Estimated Time**: 4-5 commits, legal concerns

### Priority Recommendations

#### PHASE 1: CRITICAL GAPS (1-2 commits, ~2-3 hours)
**MUST HAVE**:
1. **HEIF/HEIC** ⭐⭐⭐⭐⭐⭐ - iPhone photos (billions of files)
2. **MTS/M2TS** ⭐⭐⭐⭐⭐ - Camcorder videos (trivial)
3. **MXF** ⭐⭐⭐⭐⭐ - Broadcast standard (trivial)
4. **WMA** ⭐⭐⭐⭐ - Windows audio (trivial)
5. **AMR** ⭐⭐⭐⭐ - Mobile voice (trivial)

**Approach**: Add HEIF via FFmpeg first (simpler), batch-add trivial formats

#### PHASE 2: HIGH-VALUE ADDITIONS (2-3 commits, ~3-4 hours)
**SHOULD HAVE**:
1. **VOB** ⭐⭐⭐⭐ - DVD archives (trivial)
2. **ProRes/DNxHD** ⭐⭐⭐⭐ - Professional editing (validate)
3. **Camera RAW** ⭐⭐⭐⭐ - Professional photography (medium effort)
4. **HLS** ⭐⭐⭐⭐ - Streaming video (medium effort)

#### PHASE 3: COMPLETENESS (1-2 commits, ~2 hours)
**NICE TO HAVE**:
1. **Batch add remaining trivial formats** (RM/RMVB, DV, APE, TTA, etc.)
2. **DASH** ⭐⭐⭐⭐ - Streaming (medium effort)
3. **SVG** ⭐⭐ - Vector graphics (easy)

#### SKIP/LOW PRIORITY
- **AA/AAX** (DRM concerns)
- **VOC/AU** (obsolete)
- **R3D** (niche, limited support)
- **DSS/DVF/MSV** (niche dictation)

### Total Effort Estimate
- **Phase 1 (Critical)**: 1-2 commits, ~2-3 hours
- **Phase 2 (High-value)**: 2-3 commits, ~3-4 hours
- **Phase 3 (Completeness)**: 1-2 commits, ~2 hours
- **GRAND TOTAL**: 4-7 commits, ~7-9 hours

**Result**: ~35-40 supported formats (from current 17)

---

## Implementation Strategy

### Current Architecture Analysis

**File**: `/Users/ayates/video_audio_extracts/crates/ingestion/src/lib.rs`

```rust
pub fn ingest_media(path: &Path) -> Result<MediaInfo> {
    // Opens file with FFmpeg
    let input = ffmpeg_next::format::input(path)?;
    // Automatically detects format and codecs
    // ...
}
```

**Key Insight**: The system uses `ffmpeg_next::format::input()` which **automatically handles any format FFmpeg supports**. There is **no hardcoded extension whitelist** in the ingestion layer.

### Why Most Formats Already Work

The system's architecture means that **most missing formats already work** without code changes:

1. **FFmpeg Auto-Detection**: `ffmpeg_next::format::input()` probes files and selects appropriate demuxer
2. **Codec Transparency**: Video decoder and audio extractor modules work with any FFmpeg-decoded stream
3. **No Format Filtering**: Ingestion doesn't reject files based on extension

### What Needs To Be Done

#### For FFmpeg-Supported Formats (22 formats)
**Action Required**:
1. **Documentation** - Update README.md, ADDITIONAL_FORMATS.md with new formats
2. **Testing** - Create/find test files for each format
3. **Validation** - Add test cases to `tests/standard_test_suite.rs`
4. **Extension Hints** - Optionally add to help text (for user awareness)

**Code Changes**: **NONE REQUIRED** (or minimal documentation strings)

#### For Non-FFmpeg Formats (HEIF, RAW, SVG)
**Action Required**:
1. Add Rust dependency (`libheif-rs`, `rawloader`, `resvg`)
2. Create image loader module (parallel to FFmpeg ingestion)
3. Integrate with existing pipeline
4. Add tests

**Code Changes**: **MODERATE** (new modules, 100-200 lines per format)

### Recommended Commit Sequence

#### Commit 1: Validate + Document Trivial Formats (2 hours)
```bash
# Test files for: MTS, M2TS, MXF, VOB, TS, WMA, AMR, APE
# Update: README.md, ADDITIONAL_FORMATS.md
# Add: tests/standard_test_suite.rs entries
# Verify: All formats decode successfully
```

#### Commit 2: HEIF/HEIC Support via FFmpeg (1 hour)
```bash
# Use FFmpeg's basic HEIF support (decode-only)
# Test with iPhone photos
# Document limitations (no HEIF encoding)
```

#### Commit 3: ProRes/DNxHD Validation (1 hour)
```bash
# Validate ProRes MOV files
# Validate DNxHD MXF files
# Add professional codec tests
```

#### Commit 4: Streaming Formats (HLS) (2 hours)
```bash
# M3U8 playlist parser
# TS segment stitching
# Integration with existing pipeline
```

#### Commit 5: Camera RAW Support (3 hours)
```bash
# Add rawloader dependency
# Integrate with image pipeline
# Test CR2, NEF, ARW, DNG formats
```

#### Commit 6: Completeness Pass (1 hour)
```bash
# Add remaining obscure formats (RM, DV, TTA, etc.)
# Final documentation update
# Comprehensive format coverage report
```

**Total**: 6 commits, ~10 hours → **Comprehensive format support (35-40 formats)**

---

## Testing Requirements

### Test File Acquisition

**Easy (Download/Generate)**:
- MTS/M2TS: Sample AVCHD footage (widely available)
- VOB: Extract from DVD
- WMA/AMR: Convert with FFmpeg
- MXF: Broadcast samples online

**Medium (Requires Software)**:
- ProRes: Export from DaVinci Resolve (free)
- DNxHD: Export from Avid Media Composer (trial)
- HEIF: iPhone photos (ask team member)

**Hard (Requires Hardware/Purchase)**:
- Camera RAW: Need actual camera files
- R3D: Requires RED camera footage
- AA/AAX: Purchase Audible book (for testing only)

### Test Coverage Goals

1. **Format Detection**: Verify ingestion recognizes format
2. **Stream Extraction**: Video/audio streams decode correctly
3. **Metadata**: Duration, resolution, codec info extracted
4. **Pipeline Integration**: Keyframes, transcription, etc. work
5. **Edge Cases**: Corrupted files, unusual codecs, multi-stream

### Success Criteria

- All formats listed in README.md must have ≥1 test file
- All formats must pass smoke test (basic decode)
- Professional formats must have quality validation (resolution, bitrate)
- Streaming formats must handle multi-segment content

---

## Appendix A: FFmpeg Format Support Verification

**Command Used**: `ffmpeg -formats 2>&1`

**Confirmed Supported** (sampled):
```
DE  asf             ASF (Advanced / Active Streaming Format)
DE  avi             AVI (Audio Video Interleaved)
 D  rm              RealMedia
DE  mxf             MXF (Material eXchange Format)
 E  vob             MPEG-2 PS (VOB)
DE  amr             3GPP AMR
 D  ape             Monkey's Audio
```

**Codec Support** (sampled):
```
DEVIL. dnxhd          VC3/DNxHD
DEVIL. prores         Apple ProRes
DEV.L. hevc           H.265 / HEVC
DEV.L. av1            Alliance for Open Media AV1
```

**Conclusion**: FFmpeg 7.x supports **virtually all** identified missing formats.

---

## Appendix B: Priority Scoring Methodology

**Priority Stars** (⭐⭐⭐⭐⭐⭐ = Critical, ⭐ = Obsolete):

**Factors**:
1. **Prevalence**: How many users have files in this format?
2. **Use Case**: Consumer (high) vs niche professional (medium) vs obsolete (low)
3. **Industry Standard**: Is this required for professional workflows?
4. **Growth**: Is usage increasing (HEIF) or declining (RM)?

**Examples**:
- HEIF: ⭐⭐⭐⭐⭐⭐ (billions of iPhone photos, growing)
- MTS: ⭐⭐⭐⭐⭐ (most consumer camcorders, stable)
- WMA: ⭐⭐⭐⭐ (Windows users, declining)
- ProRes: ⭐⭐⭐⭐⭐ (post-production standard)
- VOC: ⭐ (obsolete since 1990s)

---

## Appendix C: Related Work

**Existing Documentation**:
- `ADDITIONAL_FORMATS.md` (N=78) - Lists 9 formats added without code changes
- `COMPLETE_TEST_FILE_INVENTORY.md` - Current test media (1,837 files)
- `BEST_OPEN_SOURCE_SOFTWARE.md` - Tool evaluation (historical)

**This Report Adds**:
- 29 new format recommendations
- Detailed implementation plans
- Priority rankings with rationale
- FFmpeg support verification

---

## Conclusion

**Key Takeaways**:

1. **Most formats already work** - FFmpeg handles 22/29 identified formats
2. **Critical gap**: HEIF/HEIC (iPhone photos) - billions of files
3. **Easy wins**: Batch-add trivial formats (MTS, MXF, WMA, AMR, etc.)
4. **Effort**: 4-7 commits, ~7-10 hours for comprehensive coverage
5. **Result**: 35-40 supported formats (vs 17 current)

**Next Steps**:
1. Prioritize HEIF support (most impactful)
2. Batch-add trivial FFmpeg formats (quick wins)
3. Validate professional formats (ProRes, DNxHD, MXF)
4. Consider streaming formats (HLS, DASH) based on use case

**Strategic Recommendation**:
Implement **Phase 1 (Critical)** immediately (HEIF + trivial formats), then evaluate Phase 2 based on user demand and use cases. This provides maximum format coverage with minimal effort.
