# Format Support Matrix - Quick Reference

**Date**: 2025-11-03 (N=410)
**Last Updated**: N=410 grid expansion (HEIF image format)
**See**: COMPLETE_TEST_FILE_INVENTORY.md for test file list

---

## Currently Supported (26 formats)

| Category | Count | Formats |
|----------|-------|---------|
| **Video** | 15 | MP4, MOV, MKV, WEBM, AVI, FLV, 3GP, WMV, OGV, M4V, MPG, TS, M2TS, MTS, MXF |
| **Audio** | 7 | WAV, MP3, FLAC, M4A, AAC, OGG, Opus |
| **Image** | 8 | HEIC, HEIF, JPG, PNG, WEBP, BMP, ICO, AVIF |

**Total**: 26 formats with 1006 format-function combinations tested (N=398-410)

---

## Test Coverage Summary (N=410)

**Test Suite Size**: 330 comprehensive smoke tests (~250s runtime)

**Coverage Breakdown**:
- **Audio formats**: 7 formats × 7 functions = 49 combinations (100% coverage)
- **Image formats**: 8 formats × 7-8 avg functions = 62 combinations
  - JPG/PNG/WEBP/BMP/ICO/AVIF: 8 direct-input plugins each (face/object/pose-detection, ocr, shot-classification, image-quality, vision-embeddings, duplicate-detection)
  - HEIC: 8 plugins (keyframes + 7 keyframes-based plugins)
  - HEIF: 7 plugins (keyframes + 6 keyframes-based plugins, OCR skipped due to CoreML error)
- **Video formats**: 15 formats × variable functions = 895 combinations (unchanged)
  - Mainstream (MP4, MOV, MKV, WEBM): Full plugin coverage (N=403)
  - Specialized (FLV, 3GP, TS, M2TS, MTS): Full plugin coverage (N=402)
  - Legacy (WMV, OGV, M4V, MPG, MXF, AVI): Partial to full coverage (N=404-406)

---

## Known Format Issues

### MXF Format (Professional Broadcast)
**Status**: ⚠️ PARTIAL SUPPORT - Audio pipeline only
**Issue**: Keyframe extraction returns 0 keyframes (MPEG4 decoding errors)
**Test File**: test_files_wikimedia/mxf/keyframes/C0023S01.mxf
**Discovered**: N=404

### Duplicate Detection Plugin Format Limitations
**Status**: ⚠️ LIMITED FORMAT SUPPORT
**Supported**: MP4, MOV, MKV, M2TS, TS, 3GP, FLV, WEBM
**Not Supported**: WMV, OGV, M4V, MPG
**Discovered**: N=404

---

## Recent Expansion History

**N=400** (Audio): 50 new combinations - All audio formats to 7 functions (100% coverage)
**N=401** (Image): Placeholder - corrected in N=407
**N=402** (Specialized Video): 73 new combinations - FLV, 3GP, TS, M2TS, MTS to full plugin sets
**N=403** (Mainstream Video): 58 new combinations - MP4, MOV, MKV, WEBM to full plugin sets
**N=404** (Legacy Video): 56 new combinations - WMV, OGV, M4V, MPG, MXF to partial plugin sets
**N=406** (AVI Format): 15 new combinations - AVI to full plugin coverage (was error-handling only)
**N=407** (Image Formats): 39 new combinations - JPG, PNG, WEBP, BMP (8 plugins each), HEIC (+7 beyond keyframes)
**N=408** (ICO Format): 8 new combinations - ICO format with all 8 image processing plugins
**N=409** (AVIF Format): 8 new combinations - AVIF format with all 8 image processing plugins
**N=410** (HEIF Format): 7 new combinations - HEIF format with 7 keyframes-based plugins (OCR skipped)

**Total Grid Expansion**: 679 new format-function combinations added (N=400-410)

---

**Last Updated**: 2025-11-03 (N=410)
