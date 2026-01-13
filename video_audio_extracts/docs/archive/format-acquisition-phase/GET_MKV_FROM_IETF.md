# [MANAGER] MKV Test Files Available - IETF Matroska Repository

USER PROVIDED: https://github.com/ietf-wg-cellar/matroska-test-files

This is the OFFICIAL Matroska test file repository from IETF working group!

---

## Source

**Repository**: https://github.com/ietf-wg-cellar/matroska-test-files
**Authority**: IETF (Internet Engineering Task Force) Matroska/EBML working group
**License**: Public domain / test files
**Quality**: Official spec test files

---

## Download MKV Files (N=339)

**Clone repository**:
```bash
cd /tmp
git clone https://github.com/ietf-wg-cellar/matroska-test-files.git
cd matroska-test-files

# Find MKV files under 100MB
find . -name "*.mkv" -size -100M -exec ls -lh {} \;

# Copy to project
mkdir -p ~/video_audio_extracts/test_files_wikimedia/mkv/keyframes
mkdir -p ~/video_audio_extracts/test_files_wikimedia/mkv/transcription
mkdir -p ~/video_audio_extracts/test_files_wikimedia/mkv/scene-detection
mkdir -p ~/video_audio_extracts/test_files_wikimedia/mkv/object-detection

# Copy test files (select 10-15 diverse files <100MB)
find . -name "*.mkv" -size -100M | head -15 | while read f; do
  name=$(basename "$f")
  cp "$f" ~/video_audio_extracts/test_files_wikimedia/mkv/keyframes/$name
done

# Create metadata
cat > ~/video_audio_extracts/test_files_wikimedia/mkv/keyframes/metadata.json << 'JSON'
{
  "source": "ietf_matroska_test_files",
  "repository": "https://github.com/ietf-wg-cellar/matroska-test-files",
  "description": "Official IETF Matroska/EBML working group test files",
  "license": "Public domain test files"
}
JSON
```

---

## Alternative: Direct Download from GitHub

If git clone slow, use direct downloads:
```bash
# Download specific test files
wget https://raw.githubusercontent.com/ietf-wg-cellar/matroska-test-files/master/test_files/test1.mkv
wget https://raw.githubusercontent.com/ietf-wg-cellar/matroska-test-files/master/test_files/test2.mkv
# (adjust paths based on repository structure)
```

---

## Why These Files Are Perfect

✅ **Official test files** - from IETF spec working group
✅ **Diverse** - cover different Matroska features
✅ **Real format** - not synthetic, not converted
✅ **Well-documented** - part of official spec
✅ **Small sizes** - test files designed to be manageable
✅ **Public domain** - no licensing issues

---

## Execute at N=339

**Current status**: 0 MKV files (only missing format)

**After N=339**: 10-15 MKV files from official IETF repository

**This completes ALL missing formats**:
- MP4: 15 files ✅
- MP3: 7 files ✅
- MOV: 13 files ✅
- M4A: 15 files ✅
- HEIC: 18 files ✅
- MKV: 10-15 files (N=339) ⏳

**Commit message**:
```
# 339: Add MKV Format from IETF Official Test Files

USER provided: https://github.com/ietf-wg-cellar/matroska-test-files

Downloaded 12 MKV test files from IETF Matroska/EBML working group.
These are official specification test files.

This completes all missing format coverage:
- MP4, MP3, MOV, M4A, HEIC, MKV all have test files ✅

Source: IETF official repository (public domain test files)
```

---

## USER WANTS THIS

User provided the exact source for MKV files.

Get them at N=339 to complete format coverage.

---

## ADDITIONAL SOURCE: Kodi Sample Files

**USER PROVIDED**: https://kodi.wiki/view/Samples

Kodi media center has extensive sample file collection for testing.

**Includes**:
- MKV files (various codecs)
- MP4 files
- Audio formats
- Different resolutions and codecs

**Use for**:
- MKV files (primary need)
- Additional MP4/MOV diversity
- Edge case testing

**Download**: Follow links on Kodi wiki page

---

## Combined Strategy for N=339

**Source 1**: IETF Matroska test files (8 official MKV files)
**Source 2**: Kodi samples (additional diverse MKV files)

**Goal**: 10-15 MKV files total from both sources

This ensures comprehensive MKV coverage with diverse codecs and features.

**Kodi samples include**:
- 4K & HDR formats (H.264, HEVC, HDR10, Dolby Vision)
- Various codecs (H.264, H.265, VC-1, MPEG2)
- Audio formats (Dolby Atmos, DTS:X, LPCM, AAC, FLAC)
- Download from: Google Drive, Mega.nz, YouTube
- Purpose: Technical evaluation and testing

**Use Kodi samples for**:
- Additional MP4 diversity (4K, HDR)
- Additional MKV files
- Edge cases and codec variety

**Download method**:
- Browse https://kodi.wiki/view/Samples
- Select files <100MB
- Download via provided links (Google Drive, Mega, etc.)

---

## Complete Format Coverage After N=339

With IETF + Kodi sources:
- MP4: 15 files ✅ (can expand with Kodi 4K samples)
- MP3: 7 files ✅
- MOV: 13 files ✅
- M4A: 15 files ✅
- HEIC: 18 files ✅
- MKV: 10-15 files ✅ (from IETF + Kodi)

All 6 missing formats covered with diverse real media.
