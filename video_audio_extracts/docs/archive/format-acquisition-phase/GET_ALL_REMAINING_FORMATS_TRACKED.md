# GET ALL REMAINING FORMATS - Comprehensive Tracked Acquisition
**Date**: 2025-11-02
**Authority**: USER directive - "ok. Get all the formations. get all the variances of images. track this. WE want real examples"
**Goal**: Acquire EVERY remaining format with REAL files, track systematically

---

## USER DIRECTIVE

"Get all the formations. get all the variances of images. track this. WE want real examples"

**Current**: 33 formats
**Target**: 40+ formats (ALL variants)
**Quality**: REAL files only (no conversions)

---

## MISSING FORMATS - COMPLETE LIST

### VIDEO (9 missing)

**Mainstream** (1):
1. ❌ MKV (0 files) - IETF/Kodi sources provided ← **TOP PRIORITY**

**Professional/Cinema** (5):
2. ❌ ProRes (.mov) - Check existing MOV files for ProRes codec
3. ❌ DNxHD/DNxHR (.mov/.mxf) - Check existing files
4. ❌ R3D (.r3d) - RED camera RAW (very specialized)
5. ❌ GXF (.gxf) - Broadcast format
6. ❌ DPX (.dpx) - Digital cinema frames

**Streaming** (2):
7. ❌ HLS (.m3u8) - Playlist files
8. ❌ DASH (.mpd) - Playlist files

**Variants** (1):
9. ❌ F4V (.f4v) - Flash variant

### AUDIO (7 missing)

**Lossless** (4):
10. ❌ ALAC (.m4a) - Apple Lossless (search ~/Music for alac codec)
11. ❌ APE (.ape) - Monkey's Audio
12. ❌ TTA (.tta) - True Audio
13. ❌ WavPack (.wv) - Lossless compression

**Surround** (2):
14. ❌ AC3 (.ac3) - Dolby Digital
15. ❌ DTS (.dts) - DTS audio

**Other** (1):
16. ❌ AAX (.aax) - Audible audiobook (DRM)

### IMAGE (8 missing)

**Modern** (2):
17. ❌ SVG (.svg) - Vector graphics ← **EASY**

**Professional Camera RAW** (5):
18. ❌ CR2 (.cr2) - Canon RAW ← **Search ~/Pictures**
19. ❌ NEF (.nef) - Nikon RAW ← **Search ~/Pictures**
20. ❌ ARW (.arw) - Sony RAW ← **Search ~/Pictures**
21. ❌ DNG (.dng) - Adobe Digital Negative ← **Search ~/Pictures**
22. ❌ RAF (.raf) - Fujifilm RAW

**Verified Coverage** (need to confirm):
23. ⚠️ WEBP (.webp) - Check test_edge_cases
24. ⚠️ BMP (.bmp) - Check test_edge_cases

**Other** (1):
25. ❌ ICO (.ico) - Windows icon (if relevant)

---

## ACQUISITION PLAN WITH TRACKING

### PHASE 1: Critical Formats (N=344-346)

#### N=344: MKV (CRITICAL)
**Source**: IETF + Kodi
**Commands**:
```bash
git clone https://github.com/ietf-wg-cellar/matroska-test-files.git /tmp/mkv
cp /tmp/mkv/test_files/test*.mkv test_files_wikimedia/mkv/keyframes/
# Get 8 official test files

# Also from local
cp ~/Library/CloudStorage/Dropbox*/Kinetics*/*/ice\ climbing/*.mkv test_files_wikimedia/mkv/transcription/
```
**Target**: 10-15 MKV files
**Track**: ✅ when complete

#### N=345: Camera RAW Formats (CR2, NEF, ARW, DNG)
**Source**: Search local ~/Pictures
**Commands**:
```bash
# Canon CR2
find ~/Pictures -name "*.cr2" -size -50M | head -5 | while read f; do
  cp "$f" test_files_wikimedia/cr2/object-detection/
done

# Nikon NEF
find ~/Pictures -name "*.nef" -size -50M | head -5 | while read f; do
  cp "$f" test_files_wikimedia/nef/face-detection/
done

# Sony ARW
find ~/Pictures -name "*.arw" -size -50M | head -5 | while read f; do
  cp "$f" test_files_wikimedia/arw/vision-embeddings/
done

# Adobe DNG
find ~/Pictures -name "*.dng" -size -50M | head -5 | while read f; do
  cp "$f" test_files_wikimedia/dng/object-detection/
done
```
**Target**: 3-5 files per RAW format
**Track**: ✅ CR2, ✅ NEF, ✅ ARW, ✅ DNG when complete

#### N=346: SVG (Vector Graphics)
**Source**: W3C samples, Wikimedia
**Commands**:
```bash
mkdir -p test_files_wikimedia/svg/ocr

# W3C SVG samples
wget https://dev.w3.org/SVG/tools/svgweb/samples/svg-files/tiger.svg -O test_files_wikimedia/svg/ocr/01_tiger.svg
wget https://upload.wikimedia.org/wikipedia/commons/0/02/SVG_logo.svg -O test_files_wikimedia/svg/ocr/02_svg_logo.svg
wget https://dev.w3.org/SVG/tools/svgweb/samples/svg-files/car.svg -O test_files_wikimedia/svg/ocr/03_car.svg
```
**Target**: 5-10 SVG files
**Track**: ✅ when complete

---

### PHASE 2: Lossless Audio (N=347-349)

#### N=347: ALAC (Apple Lossless)
**Source**: ~/Music library
**Commands**:
```bash
# Find ALAC files (in M4A container)
find ~/Music -name "*.m4a" | while read f; do
  codec=$(ffprobe -v quiet -select_streams a:0 -show_streams "$f" 2>&1 | grep "codec_name=alac")
  if [ -n "$codec" ]; then
    cp "$f" test_files_wikimedia/alac/transcription/
  fi
done | head -5
```
**Target**: 3-5 ALAC files
**Track**: ✅ when complete

#### N=348: APE, TTA, WavPack
**Source**: Search ~/Music, convert if needed
**Commands**:
```bash
# Search for lossless audio
find ~/Music -name "*.ape" -size -50M | head -3
find ~/Music -name "*.tta" -size -50M | head -3
find ~/Music -name "*.wv" -size -50M | head -3

# If not found, convert from FLAC (acceptable for testing rare formats)
ffmpeg -i test_files_wikimedia/flac/transcription/01*.flac -c:a ape test_files_wikimedia/ape/transcription/01_test.ape
```
**Target**: 3 files each
**Track**: ✅ APE, ✅ TTA, ✅ WavPack when complete

#### N=349: AC3, DTS (Surround)
**Source**: Extract from video with surround sound
**Commands**:
```bash
# Find videos with AC3/DTS audio
find ~/Movies -name "*.mkv" -o -name "*.mp4" | while read f; do
  codec=$(ffprobe -v quiet -select_streams a:0 -show_streams "$f" 2>&1 | grep "codec_name=ac3\|codec_name=dts")
  if [ -n "$codec" ]; then
    echo "$f has surround audio"
  fi
done | head -5

# Extract AC3 track
ffmpeg -i <file_with_ac3> -vn -c:a copy test_files_wikimedia/ac3/transcription/01_surround.ac3
```
**Target**: 3 files each
**Track**: ✅ AC3, ✅ DTS when complete

---

### PHASE 3: Verify/Expand Image Formats (N=350-352)

#### N=350: Verify WEBP, BMP Coverage
**Source**: test_edge_cases (check if exist)
**Commands**:
```bash
# Check what we have
find test_edge_cases test_files_wikimedia -name "*.webp" -o -name "*.bmp"

# If insufficient, download samples
wget https://www.gstatic.com/webp/gallery/1.webp
wget https://www.gstatic.com/webp/gallery/2.webp
```
**Target**: 5 files each
**Track**: ✅ WEBP, ✅ BMP when verified

#### N=351: Additional Image Variants
**Source**: Generate/download
**Commands**:
```bash
# ICO (Windows icons) - if relevant for icon detection
wget http://www.icoconverter.com/download/sample.ico

# PSD (Photoshop) - if can find samples
# Very specialized, may skip
```
**Target**: 3 files if available
**Track**: ✅ ICO, ⚠️ PSD (optional)

#### N=352: Fujifilm RAF
**Source**: Search local photos
**Commands**:
```bash
find ~/Pictures -name "*.raf" -size -50M | head -3
```
**Target**: 3 files if available
**Track**: ✅ RAF when complete

---

## TRACKING SYSTEM

**Create tracking file**: FORMAT_ACQUISITION_TRACKER.md

Update after each acquisition:
```markdown
# Format Acquisition Tracker

## Status: 33/41 formats acquired (80.5%)

### Video (18/26)
- [x] MP4, MOV, WEBM, AVI (mainstream)
- [ ] MKV ← NEXT (N=344)
- [x] MXF, VOB, TS, MTS, M2TS, MPG
- [x] ASF, RM, DV, WMV, FLV, 3GP, M4V, OGV
- [ ] ProRes, DNxHD (check existing)
- [ ] R3D, GXF, DPX, F4V (specialized)

### Audio (8/15)
- [x] WAV, MP3, FLAC, M4A, AAC, OGG, Opus
- [x] AMR, WMA (niche)
- [ ] ALAC, APE, TTA, WavPack (lossless) ← N=347-348
- [ ] AC3, DTS (surround) ← N=349

### Image (7/15)
- [x] JPG, PNG, HEIC, HEIF
- [x] AVIF, GIF, TIFF
- [ ] SVG ← N=346
- [ ] CR2, NEF, ARW, DNG, RAF (Camera RAW) ← N=345
- [ ] WEBP, BMP (verify), ICO (optional)

## Next: N=344 (MKV)
```

---

## EXECUTION RULES

**For EACH format**:
1. ✅ Find real files (local, download, official samples)
2. ✅ Verify authenticity (ffprobe, no Lavf encoder if applicable)
3. ✅ Copy to project (min 3 files, <100MB each)
4. ✅ Create metadata.json with source
5. ✅ Update FORMAT_ACQUISITION_TRACKER.md
6. ✅ Commit with format count update

**NO CONVERSIONS** unless:
- Format is synthetic by nature (SVG)
- Format extremely rare (TTA, APE) AND no real files found
- Document as "test-only" in metadata

---

## TIMELINE

**N=344-346** (3 commits): MKV, Camera RAW, SVG
**N=347-349** (3 commits): ALAC, APE/TTA/WavPack, AC3/DTS
**N=350-352** (3 commits): Image verification, additional variants

**Total**: 9 commits for all remaining formats

**Update tracker** after each commit with ✅/❌ status

---

## VERIFICATION

**After N=352, verify**:
```bash
# Count total formats
find test_files_wikimedia -maxdepth 1 -type d | wc -l
# Should be: 40+ format directories

# List all
ls test_files_wikimedia/
# Should include: cr2, nef, arw, dng, svg, alac, ape, tta, ac3, mkv, etc.
```

---

## COMMIT TEMPLATE

```
# 344: Add MKV Format (IETF Official) + Update Tracker (34/41 formats)

Added MKV format from IETF repository (10 files).
Updated FORMAT_ACQUISITION_TRACKER.md: 33→34 formats (82.9%).

Remaining: 7 formats (Camera RAW, SVG, lossless audio, surround audio)
Next: N=345 (Camera RAW formats)
```

---

## USER WANTS

✅ ALL formations (complete coverage)
✅ ALL image variants (Camera RAW, SVG, every format)
✅ Track systematically (tracker updated each commit)
✅ REAL examples (no conversions, verify authenticity)

**Execute starting N=344. Track progress in FORMAT_ACQUISITION_TRACKER.md.**
