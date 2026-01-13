# 🔴 CRITICAL CORRECTION: STOP FORMAT CONVERSIONS
**Date**: 2025-11-01
**Authority**: USER via MANAGER
**Priority**: ⭐⭐⭐⭐⭐⭐ URGENT - Stop immediately

---

## USER FEEDBACK (Correct)

**Manager said**: "Converted files are still based on real Wikimedia media - this is acceptable"

**User corrected**: ❌ **"This is NOT good because it doesn't capture the nuances of real files that may have different encoding patterns"**

**User is RIGHT** - format conversion defeats the purpose of real test files.

---

## Why Format Conversion is WRONG

### Real Files Have Unique Characteristics

**Real Wikimedia MP4**:
- Diverse encoders (ffmpeg, HandBrake, professional tools, phone cameras)
- Different encoding settings (variable bitrate, CBR, VBR, 2-pass)
- Real-world artifacts (compression, noise, frame drops, timing issues)
- Different H.264 profiles (baseline, main, high)
- Different GOP structures (keyframe intervals)
- Real metadata (camera info, GPS, timestamps)
- Edge cases (corrupted frames, unusual dimensions, odd frame rates)

**Converted MP4** (WebM → MP4 via FFmpeg):
- ❌ Single encoder (ffmpeg with preset settings)
- ❌ Uniform encoding (CRF 23, fast preset, predictable)
- ❌ No real-world artifacts (clean conversion)
- ❌ Same H.264 profile every time
- ❌ Predictable GOP structure
- ❌ No original metadata
- ❌ No edge cases (conversion normalizes everything)

**Result**: Converted files are essentially SYNTHETIC - they don't test real-world diversity.

---

## What This Means for Testing

### Format Conversion Creates Fake Diversity

**Example**: Worker converted WebM → MP4
- Source: 5 WebM files from Wikimedia (real)
- Output: 12 "MP4" files (converted)
- **Problem**: All 12 MP4 files have IDENTICAL encoding characteristics
  - Same encoder: ffmpeg libx264
  - Same preset: fast
  - Same CRF: 23
  - Same profile: high
  - Same settings: everything
- **They're not testing MP4 diversity** - they're testing "WebM files re-encoded as MP4"

**This is like**:
- Downloading 100 different JPEG photos
- Converting all to PNG
- Claiming you have "100 PNG test files"
- Reality: You have 1 PNG encoder tested 100 times with different image content

**Content diversity ≠ Format diversity**

---

## Correct Approach: ORIGINAL FILES ONLY

### Download Constraints

**GitHub 100MB limit**: Real constraint

**Solution**: Download ONLY files <100MB from Wikimedia
- Filter by file size in API query
- Accept that some formats will have fewer test files
- Some cells may only have 3-4 files instead of 5
- **Quality over quantity** - 3 real files > 5 converted files

**Wikimedia API supports size filtering**:
```python
params = {
    "aiminsize": 1_000_000,    # Min 1MB
    "aimaxsize": 99_000_000,   # Max 99MB (under GitHub limit)
}
```

---

## What Worker Must Do IMMEDIATELY

### N=250: STOP Conversions, REVERT to Original Files Only

**1. Delete all converted MP4/MOV files**:
```bash
# Find converted files (those in mp4/mov directories with WebM sources)
find test_files_wikimedia/mp4 test_files_wikimedia/mov -type f -name "*.mp4" -o -name "*.mov"
# Delete them (they're converted, not original)
rm -rf test_files_wikimedia/mp4/
rm -rf test_files_wikimedia/mov/
```

**2. Keep ONLY original Wikimedia downloads**:
- WebM files: ✅ Keep (original from Wikimedia)
- JPG files: ✅ Keep (original from Wikimedia)
- PNG files: ✅ Keep (original from Wikimedia)
- MP4 files with conversion_source: ❌ Delete (converted, not original)
- MOV files with conversion_source: ❌ Delete (converted, not original)

**3. Update downloader to respect 100MB limit**:
```python
# tools/download_wikimedia_tests.py

def search_files(self, mime_type=None, min_size=1_000_000, max_size=99_000_000, limit=50):
    """Search Wikimedia with size constraints"""
    params = {
        "action": "query",
        "list": "allimages",
        "aiminsize": min_size,
        "aimaxsize": max_size,  # Under 100MB GitHub limit
        "ailimit": limit,
    }
    # ... rest of code
```

**4. Re-download files with size constraint**:
- Focus on formats that are naturally <100MB:
  - MP4: Short clips, educational videos, interviews (<5 min)
  - MOV: Screen recordings, demonstrations
  - FLAC: Audio files (naturally smaller)
  - M4A: Podcasts, audiobooks
- Accept that some cells may have only 3-4 files (not 5)

---

## What About MP4/MOV Test Coverage?

### Original Files <100MB Exist on Wikimedia

**Search strategies**:

**MP4 - Short videos**:
- Category: "Short films"
- Category: "Video clips"
- Category: "Demonstrations"
- Category: "Interviews" (filter 2-5 min)
- Size filter: 10MB-95MB

**MOV - Screen recordings**:
- Category: "Screen recordings"
- Category: "Software demonstrations"
- Category: "QuickTime videos"
- Size filter: 10MB-95MB

**Real MP4/MOV files exist** - just need to:
- Filter by size (<100MB)
- Use targeted categories (shorter videos)
- Accept 3-4 files per cell instead of 5

---

## Corrected File Validity Rules

### What Counts as "Real"

✅ **REAL** (acceptable):
- Original file downloaded from Wikimedia Commons
- Original format (WebM, MP4, MOV, FLAC, etc.) preserved
- Original encoding settings preserved
- Original metadata preserved
- Under 100MB (GitHub constraint)

❌ **NOT REAL** (unacceptable):
- Converted from one format to another (WebM → MP4)
- Re-encoded with different settings
- Normalized/cleaned up during conversion
- Loses original encoder characteristics
- **Even if content is from Wikimedia**

⚠️ **ACCEPTABLE WITH LIMITS** (max 1-2 per cell):
- Synthetic files for specific test cases (edge cases, controlled tests)
- Generated files when Wikimedia lacks coverage
- But NOT converted real files (that's worse than synthetic)

---

## File Count Impact

**Current** (N=249):
- 170 files total
- ~70 original Wikimedia (WebM, JPG, PNG)
- ~100 converted (MP4, MOV from WebM) ← DELETE THESE

**After correction** (N=250):
- ~70 files (original Wikimedia only)
- Need to download ~80-130 more ORIGINAL files
- Focus on formats <100MB
- Some cells may have 3-4 files instead of 5 (acceptable)

**Quality over quantity** - 70 real files > 170 mixed real/converted

---

## Immediate Actions for N=250

### 1. Delete Converted Files
```bash
# Remove all MP4/MOV directories (these are conversions)
rm -rf test_files_wikimedia/mp4/
rm -rf test_files_wikimedia/mov/

# Verify only original formats remain
find test_files_wikimedia -type f | wc -l
# Should be ~70 (WebM, JPG, PNG originals only)
```

### 2. Update Downloader Script
```python
# tools/download_wikimedia_tests.py
# Add max_size=99_000_000 to ALL search queries

def download_test_matrix_files(self, feature, format_ext, count=5, category=None):
    # ... existing code ...

    # Add size constraint
    files = self.search_files(
        mime_type=mime_type,
        min_size=1_000_000,      # 1MB minimum
        max_size=99_000_000,     # 99MB maximum (GitHub limit)
        limit=count*3
    )
```

### 3. Download Original MP4/MOV Files <100MB
```bash
# Search for small MP4 files
python3 tools/download_wikimedia_tests.py transcription mp4 5 "Short films"
python3 tools/download_wikimedia_tests.py keyframes mp4 5 "Video clips"

# Search for small MOV files
python3 tools/download_wikimedia_tests.py face-detection mov 5 "Screen recordings"
```

### 4. Document File Authenticity
```markdown
# test_files_wikimedia/AUTHENTICITY.md

## File Validity Rules

✅ REAL: Original files from Wikimedia Commons
- Downloaded in original format
- Original encoding preserved
- Size: <100MB (GitHub constraint)
- Source URL documented in metadata.json

❌ NOT ACCEPTABLE: Format conversions
- WebM → MP4 conversions: DELETED (N=250)
- Any re-encoded files: NOT ALLOWED
- Normalized files: NOT ALLOWED

✅ ACCEPTABLE (max 1-2 per cell): Synthetic for edge cases
- Controlled test cases
- When Wikimedia lacks coverage
```

---

## Commit Message (N=250)

```
# 250: CRITICAL: Remove Converted Files, Keep Only Original Wikimedia Media

**Current Plan**: USER directive - Files must be ORIGINAL, not converted
**Checklist**: Correction complete - Removed 100 converted MP4/MOV files, keeping only 70 original Wikimedia files (WebM, JPG, PNG)

## Changes

USER feedback: "Format conversion doesn't capture nuances of real files with different encoding patterns"

**MANAGER is CORRECT** - format conversion defeats the purpose of real test files.

**Removed**:
- All MP4 files in test_files_wikimedia/mp4/ (converted from WebM, not original)
- All MOV files in test_files_wikimedia/mov/ (converted from WebM, not original)
- tools/convert_webm_to_formats.py (conversion approach abandoned)
- Total removed: ~100 files

**Kept**:
- WebM files: 62 original downloads from Wikimedia ✅
- JPG files: ~30 original downloads from Wikimedia ✅
- PNG files: ~10 original downloads from Wikimedia ✅
- Total kept: ~70 original files, 2.5GB

**Rationale**:
- Real files have diverse encoding patterns (different encoders, settings, artifacts)
- Converted files are uniform (same encoder, same settings = essentially synthetic)
- 70 real diverse files > 170 mixed real+converted files
- Quality over quantity

**Next Steps**:
- Download original MP4/MOV files <100MB from Wikimedia (N=251+)
- Update downloader with size filter: max_size=99_000_000
- Accept 3-4 files per cell if large files unavailable (better than conversions)

## New Lessons

Format conversion creates synthetic test files even from real content:
- Content diversity ≠ Format diversity
- 100 converted MP4s test 1 encoder, not MP4 format diversity
- Real files have encoding quirks that conversions normalize away

**File authenticity matters for comprehensive testing.**
```

---

## THIS IS URGENT

**Worker has been creating FAKE diversity** by converting WebM → MP4

**100 of 170 files are conversions** (not real)

**Must revert at N=250** before continuing

**User is right** - I was wrong to accept this approach.