# 🔴 CRITICAL: Download Missing Formats NOW
**Date**: 2025-11-01
**Authority**: USER urgent directive via MANAGER
**Priority**: ⭐⭐⭐⭐⭐⭐ URGENT - Next priority

---

## USER DIRECTIVE (Urgent)

**User**: "get those missing formats. Find them! We need them!"

**Critical gap identified**: 0 files for most common formats

---

## CRITICAL MISSING FORMATS

**Have 0 files**:
- ❌ **MP4** (0 files) - MOST COMMON video format
- ❌ **MOV** (0 files) - Apple/professional video
- ❌ **MKV** (0 files) - Popular open container
- ❌ **MP3** (0 files) - MOST COMMON audio format
- ❌ **M4A** (0 files) - Apple audio format
- ❌ **HEIC** (0 files) - iPhone photos (billions exist!)

**Current formats** (have files):
- ✅ WEBM: 209 files
- ✅ JPG: 2,510 files
- ✅ PNG: 60 files
- ✅ WAV: 56 files
- ✅ FLAC: 20 files

**Problem**: Missing the MOST COMMON formats used by consumers!

---

## MANDATORY ACTION: Download Missing Formats

### Priority 1: MP4 (MOST URGENT)

**Most common video format** - must have this

**Download script**:
```bash
# MP4 short videos from Wikimedia (under 100MB)
python3 tools/download_wikimedia_tests.py transcription mp4 5 "Short films" 10000000 95000000
python3 tools/download_wikimedia_tests.py keyframes mp4 5 "Video clips" 10000000 95000000
python3 tools/download_wikimedia_tests.py object-detection mp4 5 "Demonstrations" 10000000 95000000
python3 tools/download_wikimedia_tests.py face-detection mp4 5 "Interviews" 10000000 95000000
python3 tools/download_wikimedia_tests.py scene-detection mp4 5 "Short films" 10000000 95000000

# Target: 25+ MP4 files (5 features × 5 files)
```

**Wikimedia categories for MP4 <100MB**:
- "Short films" (2-10 minute films)
- "Video clips" (excerpts, demonstrations)
- "Interviews" (talking heads, 5-15 min)
- "Demonstrations" (how-to videos)
- "Educational videos" (filtered by duration/size)

---

### Priority 2: MP3 (Audio, CRITICAL)

**Most common audio format** - must have this

**Download script**:
```bash
# MP3 audio from Wikimedia (under 100MB)
python3 tools/download_wikimedia_tests.py transcription mp3 5 "Speeches" 5000000 95000000
python3 tools/download_wikimedia_tests.py audio-classification mp3 5 "Music" 5000000 95000000
python3 tools/download_wikimedia_tests.py audio-embeddings mp3 5 "Podcasts" 5000000 95000000
python3 tools/download_wikimedia_tests.py diarization mp3 5 "Interviews" 5000000 95000000

# Target: 20+ MP3 files (4 features × 5 files)
```

**Wikimedia categories for MP3**:
- "Speeches" (political, educational)
- "Music" (public domain, classical)
- "Podcasts" (if available)
- "Audio files in MP3 format" (generic category)

---

### Priority 3: MOV (Apple/Professional)

**Apple ecosystem + professional video**

**Download script**:
```bash
# MOV files from Wikimedia (under 100MB)
python3 tools/download_wikimedia_tests.py keyframes mov 5 "QuickTime videos" 10000000 95000000
python3 tools/download_wikimedia_tests.py transcription mov 5 "Screen recordings" 10000000 95000000
python3 tools/download_wikimedia_tests.py face-detection mov 5 "Demonstrations" 10000000 95000000

# Target: 15+ MOV files (3 features × 5 files)
```

**Wikimedia categories**:
- "QuickTime videos"
- "Screen recordings" (often MOV format on macOS)
- "Demonstrations" (software demos, tutorials)

---

### Priority 4: HEIC (iPhone Photos)

**Billions of iPhone photos use HEIC** - format supported but no test files!

**Challenge**: Wikimedia Commons has limited HEIC (most photos uploaded as JPEG)

**Strategy**:
```bash
# Search for HEIC on Wikimedia (may find few)
python3 tools/download_wikimedia_tests.py face-detection heic 5 "Photographs" 100000 10000000

# If insufficient, create from iPhone photos:
# 1. Take 10 photos with iPhone (HEIC format)
# 2. Or convert existing high-quality JPGs to HEIC:
sips -s format heic input.jpg --out output.heic

# Target: 10-20 HEIC files (acceptable to use iPhone-generated if Wikimedia sparse)
```

---

### Priority 5: MKV (Popular Container)

**Open source container, widely used**

**Download script**:
```bash
# MKV files from Wikimedia (under 100MB)
python3 tools/download_wikimedia_tests.py keyframes mkv 5 "Matroska videos" 10000000 95000000
python3 tools/download_wikimedia_tests.py transcription mkv 5 "Videos" 10000000 95000000

# Target: 10+ MKV files (2 features × 5 files)
```

---

### Priority 6: M4A (Apple Audio)

**Apple audio format, common on iOS/macOS**

**Download script**:
```bash
# M4A files from Wikimedia (under 100MB)
python3 tools/download_wikimedia_tests.py transcription m4a 5 "Audio files in M4A format" 5000000 95000000
python3 tools/download_wikimedia_tests.py audio-classification m4a 5 "Podcasts" 5000000 95000000

# Target: 10+ M4A files (2 features × 5 files)
```

---

## Size Filter Strategy

**GitHub 100MB limit** is the constraint

**Solution**: Filter Wikimedia API by file size

**Update downloader** (tools/download_wikimedia_tests.py):
```python
def download_test_matrix_files(self, feature, format_ext, count=5, category=None,
                              min_size=1_000_000, max_size=99_000_000):  # ADD SIZE PARAMS
    """Download files with size constraints"""

    # Search with size filter
    files = self.search_files(
        mime_type=mime_type,
        min_size=min_size,      # Default: 1MB minimum
        max_size=max_size,      # Default: 99MB maximum (under GitHub 100MB limit)
        limit=count*3
    )
```

**Usage**:
```bash
# Download MP4 files between 10MB-95MB
python3 tools/download_wikimedia_tests.py transcription mp4 5 "Short films" 10000000 95000000

# Download MP3 files between 5MB-95MB
python3 tools/download_wikimedia_tests.py transcription mp3 5 "Speeches" 5000000 95000000
```

---

## Fallback: Use Existing Test Files

**If Wikimedia doesn't have enough <100MB files**:

**MP4 files we already have**:
```bash
# Check existing MP4 test files
find ~/Desktop/stuff ~/Downloads test_edge_cases -name "*.mp4" -size -100M | head -20

# Use existing test files (documented in COMPLETE_TEST_FILE_INVENTORY.md)
# Copy to test_files_wikimedia (mark as "local" not "wikimedia" in metadata)
```

**MP3 files we already have**:
```bash
# Existing MP3 test files
find test_edge_cases ~/Downloads -name "*.mp3" -size -100M | head -20
```

**Accept local files** if Wikimedia lacks coverage (better than nothing)

---

## Expected Outcome (N=259-265)

**N=259**: Download 20-30 MP4 files (5+ features)
**N=260**: Download 15-20 MP3 files (3-4 features)
**N=261**: Download 10-15 MOV files (2-3 features)
**N=262**: Download 10-15 MKV files (2-3 features)
**N=263**: Download 10-15 M4A files (2-3 features)
**N=264**: Download 10-20 HEIC files (2-3 features)
**N=265**: Cleanup cycle

**Result**: 75-115 new files, 6 new formats, comprehensive coverage

---

## Commit Message Template

```
# 259: Critical Missing Formats - MP4 Support Added (25 Files)

**Current Plan**: USER directive - Download missing critical formats
**Checklist**: MP4 format added - 25 files downloaded from Wikimedia, 5 features covered

## Changes

USER: "get those missing formats. Find them! We need them!"

Downloaded MP4 files (most common video format):
- transcription × mp4: 5 files (Short films, 15-85MB)
- keyframes × mp4: 5 files (Video clips, 20-90MB)
- object-detection × mp4: 5 files (Demonstrations, 25-80MB)
- face-detection × mp4: 5 files (Interviews, 30-95MB)
- scene-detection × mp4: 5 files (Short films, 40-88MB)

**Total**: 25 MP4 files, 1.2GB
**Source**: Wikimedia Commons (original files, not conversions)
**Size filter**: 10MB-95MB (under GitHub 100MB limit)
**Authenticity**: All original MP4 files with diverse encoding (various encoders, settings, sources)

**Progress**: Filled critical format gap (was 0 MP4 files, now 25)

## Next AI

Continue with MP3 format (N=260) - second most critical missing format.
```

---

## THIS IS URGENT

**User is emphatic**: "Find them! We need them!"

**Worker must prioritize** missing formats over expanding existing formats

**Next 7 commits** (N=259-265): Download 6 missing formats (MP4, MP3, MOV, MKV, M4A, HEIC)

**Do NOT continue expanding JPG/WEBM** until missing formats covered.
