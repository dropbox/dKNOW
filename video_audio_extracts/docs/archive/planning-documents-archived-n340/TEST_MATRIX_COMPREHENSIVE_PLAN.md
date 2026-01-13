# Comprehensive Test Matrix Plan
**Date**: 2025-11-01
**Purpose**: 5 test cases per (feature × format) cell using REAL media from Wikimedia Commons
**Goal**: Exhaustive test coverage with real-world files (not synthetic)

---

## Test Matrix Dimensions

### Current Features (23 functional plugins)

**Video Features** (13 plugins):
1. keyframes
2. scene-detection
3. object-detection
4. face-detection
5. ocr
6. action-recognition
7. motion-tracking
8. pose-estimation
9. smart-thumbnail
10. subtitle-extraction
11. shot-classification
12. emotion-detection
13. image-quality-assessment

**Audio Features** (7 plugins):
14. audio-extraction
15. transcription
16. diarization
17. audio-classification
18. audio-enhancement-metadata
19. audio-embeddings
20. metadata-extraction

**Embeddings** (3 plugins):
21. vision-embeddings
22. text-embeddings
23. audio-embeddings (counted above)

### Current Formats (23 formats)

**Video** (10): MP4, MOV, MKV, WEBM, AVI, FLV, 3GP, WMV, OGV, M4V
**Audio** (7): WAV, MP3, FLAC, M4A, AAC, OGG, Opus
**Image** (6): JPEG, PNG, WEBP, BMP, TIFF, GIF

### Planned Additions (Research Complete)

**New Formats** (31 identified, priority subset):
- ⭐⭐⭐ **HEIF/HEIC** (iPhone photos)
- ⭐⭐ MTS/M2TS, MXF, ProRes, TS, VOB (professional/consumer)
- ⭐ WMA, AMR, APE (audio)
- Camera RAW (CR2, NEF, ARW, DNG)

**New Features** (68 identified, quick wins):
- Language detection, VAD, acoustic scenes, profanity detection
- Visual search, cross-modal search, text-in-video search
- Duplicate detection, video summarization, semantic segmentation

---

## Test Matrix Compatibility

### Not All (Feature × Format) Combinations Are Valid

**Example incompatibilities**:
- transcription ❌ JPEG (no audio in images)
- keyframes ❌ MP3 (no video in audio files)
- subtitle-extraction ❌ WAV (no subtitles in audio)

**Valid compatibility rules**:

**Video features** work with: Video formats only (10 formats)
- keyframes, scene-detection, object-detection, action-recognition, etc.

**Audio features** work with: Video + Audio formats (17 formats)
- transcription, diarization, audio-classification work with both

**Vision features** work with: Video + Image formats (16 formats)
- face-detection, ocr, emotion-detection, image-quality work with both

**Audio-only features** work with: Audio formats only (7 formats)
- audio-enhancement-metadata (needs raw audio, not video audio)

---

## Test Matrix Size Calculation

### Current System (23 plugins × 23 formats)

**Video Features** (13) × **Video Formats** (10) = 130 cells
**Audio Features** (7) × **Audio+Video Formats** (17) = 119 cells
**Vision Features** (7) × **Video+Image Formats** (16) = 112 cells
**Embeddings** (3) × **All Formats** (23) = 69 cells

**Total Valid Cells**: ~350-400 cells (accounting for overlaps)

**Test Cases Needed**: 350 × 5 = **1,750 test cases** (current system)

---

### With New Formats and Features

**Conservative Estimate** (priority features only):
- Add 8 quick-win features
- Add 10 priority formats (HEIF, MTS, MXF, WMA, etc.)
- New combinations: ~200 additional cells
- **Total cells**: ~550-600
- **Test cases needed**: 550 × 5 = **2,750 test cases**

**Full Expansion** (all 68 features, all 31 formats):
- (23+68) plugins = 91 features
- (23+31) formats = 54 formats
- Valid cells: ~2,000-2,500 cells
- **Test cases needed**: 2,000 × 5 = **10,000 test cases**

---

## Phased Approach (Realistic)

### Phase 1: Core Coverage (Current System)
**Goal**: 5 test cases per cell for current 23 plugins × 23 formats

**Priority 1 Cells** (High Impact): ~150 cells, 750 test cases
- Common combinations: MP4/MOV/MKV with keyframes/transcription/object-detection
- Popular workflows: Audio extraction + transcription, keyframes + face detection

**Priority 2 Cells** (Medium Impact): ~120 cells, 600 test cases
- Less common but important: WEBM with vision features, FLAC transcription

**Priority 3 Cells** (Full Coverage): ~100 cells, 500 test cases
- Edge cases: 3GP/FLV, rare combinations

**Total Phase 1**: 1,850 test cases

### Phase 2: Critical New Formats
**Goal**: Add HEIF/HEIC + 5 priority formats, test with all compatible features

**New Formats**: HEIF, MTS, MXF, WMA, AMR, VOB (6 formats)
**Compatible Features**: ~20 features per format
**New Cells**: ~120 cells
**Test Cases**: 120 × 5 = 600 test cases

**Total After Phase 2**: 2,450 test cases

### Phase 3: Quick Win Features
**Goal**: Add 8 quick-win features, test with all compatible formats

**New Features**: Language detection, VAD, acoustic scenes, profanity, visual search, cross-modal search, text-in-video search, duplicate detection (8 features)
**Compatible Formats**: ~15 formats per feature
**New Cells**: ~120 cells
**Test Cases**: 120 × 5 = 600 test cases

**Total After Phase 3**: 3,050 test cases

---

## Wikimedia Commons as Test Source

### Why Wikimedia Commons?

✅ **Real-world media**: Not synthetic/generated
✅ **Diverse content**: Global coverage, many languages, scenarios
✅ **Legal**: Public domain or Creative Commons licenses
✅ **Quality**: High-quality professional media
✅ **Metadata**: Rich metadata (categories, descriptions, dates)
✅ **API Access**: Wikimedia Commons API for programmatic download
✅ **Large collection**: Millions of files across all formats

### Wikimedia Commons Categories

**Video**:
- Educational videos
- News footage
- Nature/wildlife
- Historical footage
- Sports
- Demonstrations
- Time-lapses

**Audio**:
- Spoken word (multiple languages)
- Music (classical, jazz, folk)
- Sound effects
- Podcasts
- Audiobooks
- Nature sounds

**Images**:
- Photographs (nature, people, objects, scenes)
- Artwork
- Diagrams
- Historical photos
- Scientific images

---

## Test File Acquisition Strategy

### Wikimedia Commons API

**API Endpoint**: https://commons.wikimedia.org/w/api.php

**Search Query Examples**:
```bash
# Search for MP4 videos
https://commons.wikimedia.org/w/api.php?action=query&list=categorymembers&cmtitle=Category:Videos&cmtype=file&cmlimit=500&format=json

# Search by format
https://commons.wikimedia.org/w/api.php?action=query&list=allimages&aiprop=url|size|mediatype&aiminsize=1000000&aimaxsize=50000000&ailimit=50&format=json

# Get file URL
https://commons.wikimedia.org/w/api.php?action=query&titles=File:Example.mp4&prop=imageinfo&iiprop=url&format=json
```

**Python API Wrapper**:
```python
import requests

def search_wikimedia_files(format_ext, category=None, min_size=1000000, max_size=50000000, limit=100):
    """Search Wikimedia Commons for files by format and category"""
    api_url = "https://commons.wikimedia.org/w/api.php"

    params = {
        "action": "query",
        "list": "allimages",
        "aiprop": "url|size|mediatype|mime",
        "aiminsize": min_size,
        "aimaxsize": max_size,
        "ailimit": limit,
        "format": "json"
    }

    response = requests.get(api_url, params=params)
    data = response.json()

    # Filter by extension
    files = []
    for item in data.get("query", {}).get("allimages", []):
        if item.get("url", "").lower().endswith(format_ext.lower()):
            files.append({
                "name": item["name"],
                "url": item["url"],
                "size": item["size"],
                "mime": item.get("mime", "")
            })

    return files

# Example usage
mp4_files = search_wikimedia_files(".mp4", min_size=10_000_000, max_size=100_000_000, limit=100)
for f in mp4_files[:5]:
    print(f"{f['name']}: {f['size']/1_000_000:.1f} MB - {f['url']}")
```

---

## Test Matrix Prioritization

### Tier 1: Essential (Must Have) - 150 cells, 750 test cases

**Most Common Workflows**:

| Feature | Formats | Priority | Test Cases |
|---------|---------|----------|------------|
| keyframes | MP4, MOV, MKV | ⭐⭐⭐ | 15 (5 per format) |
| transcription | MP4, MOV, WAV, MP3, M4A | ⭐⭐⭐ | 25 (5 per format) |
| object-detection | MP4, MOV, JPEG, PNG | ⭐⭐⭐ | 20 (5 per format) |
| face-detection | MP4, JPEG, PNG | ⭐⭐⭐ | 15 (5 per format) |
| audio-extraction | MP4, MOV, MKV, WEBM | ⭐⭐⭐ | 20 (5 per format) |
| ocr | MP4, JPEG, PNG, PDF (future) | ⭐⭐⭐ | 20 (5 per format) |
| scene-detection | MP4, MOV, MKV | ⭐⭐ | 15 (5 per format) |
| diarization | MP4, WAV, MP3 | ⭐⭐ | 15 (5 per format) |
| vision-embeddings | JPEG, PNG, MP4, MOV | ⭐⭐ | 20 (5 per format) |
| audio-embeddings | WAV, MP3, M4A, MP4 | ⭐⭐ | 20 (5 per format) |

**Subtotal Tier 1**: ~185 test cases (conservative)

---

### Tier 2: Important (Should Have) - 200 cells, 1,000 test cases

**Less Common but Valuable**:

| Feature | Formats | Priority | Test Cases |
|---------|---------|----------|------------|
| pose-estimation | MP4, JPEG, PNG | ⭐⭐ | 15 |
| emotion-detection | MP4, JPEG | ⭐⭐ | 10 |
| audio-classification | WAV, MP3, OGG, MP4 | ⭐⭐ | 20 |
| subtitle-extraction | MP4, MKV | ⭐ | 10 |
| action-recognition | MP4, MOV, WEBM | ⭐ | 15 |
| motion-tracking | MP4, MOV | ⭐ | 10 |
| image-quality | JPEG, PNG, WEBP | ⭐ | 15 |
| shot-classification | MP4, MOV | ⭐ | 10 |
| smart-thumbnail | MP4, MOV | ⭐ | 10 |
| All features | WEBM, FLAC, AAC, OGG | ⭐ | 100+ |

**Subtotal Tier 2**: ~215 test cases

---

### Tier 3: Complete Coverage - 1,300+ test cases

**All Remaining Combinations**:
- All plugins with all compatible formats
- Edge cases (FLV, 3GP, WMV, OGV)
- Rare combinations
- Stress testing (large files, long duration, high resolution)

---

## Wikimedia Commons File Search Strategy

### By Format and Content Type

**MP4/MOV Videos**:
- Category:Educational videos
- Category:Lectures
- Category:Nature videos
- Category:Interviews
- Category:Demonstrations
- Category:Time-lapses

**WAV/MP3 Audio**:
- Category:Spoken word recordings
- Category:Audiobooks
- Category:Speeches
- Category:Music (public domain)
- Category:Nature sounds
- Category:Language recordings

**JPEG/PNG Images**:
- Category:Photographs
- Category:People
- Category:Faces
- Category:Text (for OCR)
- Category:Logos (public domain)
- Category:Nature

### Search Criteria

**For Each (Feature, Format) Cell**:

**Example: (transcription, MP4)**
- Find 5 MP4 files with clear speech
- Variety: Different languages, accents, speakers
- Size range: 1-50MB each
- Duration range: 30s - 5min
- Source: Wikimedia Commons lectures, interviews, speeches

**Example: (face-detection, JPEG)**
- Find 5 JPEG images with human faces
- Variety: Different ages, genders, ethnicities, expressions
- Resolution range: 640x480 - 4K
- Source: Wikimedia Commons portrait photography

**Example: (object-detection, MOV)**
- Find 5 MOV files with multiple objects
- Variety: Indoor/outdoor, different object types, varying counts
- Duration: 10s - 2min
- Source: Wikimedia Commons nature videos, demonstrations

---

## Wikimedia Commons API Integration

### Python Script for Automated Download

```python
#!/usr/bin/env python3
"""
Download real media files from Wikimedia Commons for test matrix.
Creates organized directory structure: test_files_wikimedia/{format}/{feature}/
"""

import requests
import json
import os
from pathlib import Path
from urllib.parse import urlparse, unquote

class WikimediaDownloader:
    """Download test files from Wikimedia Commons"""

    API_URL = "https://commons.wikimedia.org/w/api.php"

    def __init__(self, output_dir="test_files_wikimedia"):
        self.output_dir = Path(output_dir)
        self.session = requests.Session()
        self.session.headers.update({
            'User-Agent': 'VideoAudioExtractTestSuite/1.0 (Research/Testing)'
        })

    def search_files(self, category=None, mime_type=None, min_size=1_000_000,
                    max_size=50_000_000, limit=50):
        """Search Wikimedia Commons for files matching criteria"""

        params = {
            "action": "query",
            "list": "allimages",
            "aiprop": "url|size|mime|mediatype|timestamp",
            "aiminsize": min_size,
            "aimaxsize": max_size,
            "ailimit": limit,
            "format": "json"
        }

        if mime_type:
            params["aimime"] = mime_type

        response = self.session.get(self.API_URL, params=params)
        data = response.json()

        return data.get("query", {}).get("allimages", [])

    def search_by_category(self, category, limit=50):
        """Search files in a specific Wikimedia Commons category"""

        params = {
            "action": "query",
            "list": "categorymembers",
            "cmtitle": f"Category:{category}",
            "cmtype": "file",
            "cmlimit": limit,
            "format": "json"
        }

        response = self.session.get(self.API_URL, params=params)
        data = response.json()

        members = data.get("query", {}).get("categorymembers", [])

        # Get file info for each member
        file_infos = []
        for member in members:
            file_info = self.get_file_info(member["title"])
            if file_info:
                file_infos.append(file_info)

        return file_infos

    def get_file_info(self, title):
        """Get detailed info for a specific file"""

        params = {
            "action": "query",
            "titles": title,
            "prop": "imageinfo",
            "iiprop": "url|size|mime|mediatype|timestamp",
            "format": "json"
        }

        response = self.session.get(self.API_URL, params=params)
        data = response.json()

        pages = data.get("query", {}).get("pages", {})
        for page_id, page in pages.items():
            if "imageinfo" in page:
                info = page["imageinfo"][0]
                return {
                    "title": title,
                    "url": info["url"],
                    "size": info["size"],
                    "mime": info["mime"],
                    "mediatype": info.get("mediatype", "")
                }

        return None

    def download_file(self, url, output_path):
        """Download file from URL to output path"""

        output_path.parent.mkdir(parents=True, exist_ok=True)

        print(f"Downloading {output_path.name}...")
        response = self.session.get(url, stream=True)
        response.raise_for_status()

        with open(output_path, 'wb') as f:
            for chunk in response.iter_content(chunk_size=8192):
                f.write(chunk)

        return output_path

    def download_test_matrix_files(self, feature, format_ext, count=5,
                                   category=None, min_size=1_000_000, max_size=50_000_000):
        """Download test files for a specific (feature, format) cell"""

        print(f"\n=== Downloading tests for ({feature}, {format_ext}) ===")

        # Search for files
        if category:
            files = self.search_by_category(category, limit=count*2)
        else:
            # Infer MIME type from extension
            mime_map = {
                "mp4": "video/mp4",
                "mov": "video/quicktime",
                "mkv": "video/x-matroska",
                "webm": "video/webm",
                "wav": "audio/wav",
                "mp3": "audio/mpeg",
                "flac": "audio/flac",
                "m4a": "audio/mp4",
                "jpg": "image/jpeg",
                "jpeg": "image/jpeg",
                "png": "image/png",
            }
            mime_type = mime_map.get(format_ext.lower())
            files = self.search_files(mime_type=mime_type, min_size=min_size,
                                     max_size=max_size, limit=count*2)

        # Filter by extension (API may return similar formats)
        filtered = [f for f in files if f["url"].lower().endswith(format_ext.lower())]

        # Take first N files
        selected = filtered[:count]

        if len(selected) < count:
            print(f"⚠️  Warning: Only found {len(selected)}/{count} files")

        # Download each file
        downloaded_files = []
        for i, file_info in enumerate(selected, 1):
            # Create filename from Wikimedia title
            filename = file_info["title"].replace("File:", "").replace(" ", "_")

            # Ensure correct extension
            if not filename.lower().endswith(format_ext.lower()):
                filename = f"{filename}.{format_ext}"

            output_path = self.output_dir / format_ext / feature / f"{i:02d}_{filename}"

            try:
                self.download_file(file_info["url"], output_path)
                downloaded_files.append({
                    "path": str(output_path),
                    "size": file_info["size"],
                    "url": file_info["url"],
                    "title": file_info["title"]
                })
            except Exception as e:
                print(f"  ❌ Error downloading {filename}: {e}")

        # Save metadata
        metadata_path = self.output_dir / format_ext / feature / "metadata.json"
        metadata_path.parent.mkdir(parents=True, exist_ok=True)
        with open(metadata_path, 'w') as f:
            json.dump(downloaded_files, f, indent=2)

        print(f"✅ Downloaded {len(downloaded_files)}/{count} files for ({feature}, {format_ext})")
        return downloaded_files

# Usage example
downloader = WikimediaDownloader()

# Download for high-priority cells
downloader.download_test_matrix_files("transcription", "mp4", count=5, category="Speeches")
downloader.download_test_matrix_files("face-detection", "jpg", count=5, category="Portrait photographs")
downloader.download_test_matrix_files("object-detection", "png", count=5, category="Still life photographs")
```

---

## Test Matrix Organization

### Directory Structure

```
test_files_wikimedia/
├── mp4/
│   ├── keyframes/
│   │   ├── 01_Nature_Documentary.mp4
│   │   ├── 02_Educational_Lecture.mp4
│   │   ├── 03_News_Footage.mp4
│   │   ├── 04_Wildlife_Video.mp4
│   │   ├── 05_Time_Lapse.mp4
│   │   └── metadata.json
│   ├── transcription/
│   │   ├── 01_Speech_English.mp4
│   │   ├── 02_Speech_Spanish.mp4
│   │   ├── 03_Lecture_French.mp4
│   │   ├── 04_Interview_German.mp4
│   │   ├── 05_Presentation_Chinese.mp4
│   │   └── metadata.json
│   ├── object-detection/
│   │   └── ...
│   └── face-detection/
│       └── ...
├── mov/
│   └── ...
├── wav/
│   └── ...
├── jpg/
│   └── ...
└── README.md (catalog of all downloaded files)
```

---

## Implementation Plan

### Phase 1: Infrastructure (N=228-232, 5 commits)

**N=228: Create Wikimedia downloader script**
- Implement WikimediaDownloader class
- Add API search methods
- Add file download methods
- Test with 10 sample files

**N=229: Create test matrix definition**
- Define valid (feature, format) cells
- Prioritize cells (Tier 1/2/3)
- Calculate total test cases needed
- Document in TEST_MATRIX_DEFINITION.md

**N=230: Cleanup cycle** (N mod 5)

**N=231: Download Tier 1 priority tests (part 1)**
- Download 5 tests each for: keyframes × MP4/MOV/MKV
- Download 5 tests each for: transcription × MP4/WAV/MP3
- Download 5 tests each for: object-detection × MP4/JPEG
- Total: ~75 files

**N=232: Download Tier 1 priority tests (part 2)**
- Download 5 tests each for: face-detection × JPEG/PNG/MP4
- Download 5 tests each for: audio-extraction × MP4/MOV/MKV
- Download 5 tests each for: ocr × JPEG/PNG/MP4
- Total: ~75 files

---

### Phase 2: Tier 1 Test Matrix (N=233-250, 18 commits)

**Goal**: 750 test cases covering highest-priority combinations

**Approach**:
- Download 75-100 files per commit (manageable chunks)
- Organize by (format, feature) directories
- Save metadata.json with source URLs
- Validate each file with ffprobe/video-extract
- Update test inventory

**Commits N=233-250**: Download and validate Tier 1 cells
- ~10 commits for downloads (75 files each)
- ~3 cleanup cycles (N=235, 240, 245)
- ~5 commits for validation and test integration

---

### Phase 3: Expand for New Formats (N=251-270, 20 commits)

**After HEIF/HEIC and other formats added**:
- Download 5 test files for each new format
- Test with all compatible features
- Expand matrix coverage

**HEIF/HEIC Specifically** (HIGH PRIORITY):
- Find 5 HEIF/HEIC files on Wikimedia Commons
- If sparse, accept 1-2 generated (iPhone photos exported to HEIC)
- Test with: vision-embeddings, object-detection, face-detection, ocr, image-quality

---

### Phase 4: Expand for New Features (N=271-300, 30 commits)

**After quick-win features added**:
- Test each new feature with all compatible formats
- Example: Language detection × all audio/video formats (17 formats × 5 = 85 test cases)
- Example: Visual search × all image/video formats (16 formats × 5 = 80 test cases)

---

## Test Case Diversity Requirements

### For Each (Feature, Format) Cell, Ensure Variety:

**Video Files**:
- ✅ Multiple durations: <1min, 1-5min, 5-15min, >15min, >1hr (use at least 3 duration ranges)
- ✅ Multiple resolutions: 480p, 720p, 1080p, 4K (use at least 2 resolutions)
- ✅ Multiple codecs: H.264, H.265, VP9 (use at least 2 codecs per format)
- ✅ Multiple content types: nature, people, objects, text, faces (use at least 3 types)
- ⚠️ Only 0-1 synthetic files (prefer real media)

**Audio Files**:
- ✅ Multiple durations: <1min, 1-5min, 5-15min, >15min (use at least 3 ranges)
- ✅ Multiple languages: English, Spanish, Chinese, French, etc. (use at least 3 languages)
- ✅ Multiple speakers: Male, female, child, elderly (use at least 2)
- ✅ Multiple scenarios: Speech, music, nature sounds, mixed (use at least 3)
- ⚠️ Only 0-1 synthetic files

**Image Files**:
- ✅ Multiple resolutions: 640x480, 1920x1080, 4K+ (use at least 2)
- ✅ Multiple content types: People, objects, scenes, text, logos (use at least 3)
- ✅ Multiple lighting: Day, night, indoor, outdoor (use at least 2)
- ⚠️ Only 0-1 synthetic files

---

## Synthetic File Policy

**USER DIRECTIVE**: "Real media per test. 1 or 2 generated examples may be tolerated."

**Policy**:
- ✅ **Prefer real files from Wikimedia Commons** (80-100% real media)
- ⚠️ **Allow 1-2 synthetic files** per (feature, format) cell IF:
  - Wikimedia Commons has insufficient files for that format
  - Need controlled test (specific resolution, duration, characteristic)
  - Testing edge cases (corrupted files, minimal files, extreme cases)
- ❌ **Do NOT create 3+ synthetic files** per cell

**Example Acceptable Mix**:
- (transcription, MP4): 4 real Wikimedia speeches + 1 synthetic test (extreme long duration)
- (object-detection, JPEG): 5 real Wikimedia photos (0 synthetic)
- (keyframes, FLV): 3 real Wikimedia + 2 synthetic (FLV rare on Wikimedia)

---

## Success Metrics

**Phase 1 Success** (Tier 1 complete):
- ✅ 750 test cases downloaded (150 cells × 5 each)
- ✅ 80-100% real media (from Wikimedia Commons)
- ✅ Covers all high-priority (feature, format) combinations
- ✅ Metadata documented (source URLs, licenses, descriptions)
- ✅ All tests pass (validate with video-extract)

**Phase 2 Success** (Tier 2 complete):
- ✅ 1,750 total test cases (Tier 1 + Tier 2)
- ✅ Covers all important combinations
- ✅ Includes less common formats (WEBM, FLAC, OGG)

**Phase 3 Success** (Full coverage):
- ✅ 3,000+ total test cases
- ✅ Every valid (feature, format) cell has 5 tests
- ✅ Includes new formats (HEIF, MTS, MXF) and new features (language detection, VAD, etc.)

---

## Estimated Timeline

**Phase 1: Infrastructure** (N=228-232, 5 commits, 1 week)
- Wikimedia downloader script
- Test matrix definition
- Initial downloads (150 files)

**Phase 2: Tier 1 Matrix** (N=233-250, 18 commits, 3 weeks)
- Download 750 test cases
- Validate all files
- Integrate into test suite

**Phase 3: Format Expansion** (N=251-270, 20 commits, 3 weeks)
- Add new formats (HEIF, MTS, MXF, etc.)
- Download tests for new formats
- 600 additional test cases

**Phase 4: Feature Expansion** (N=271-300, 30 commits, 5 weeks)
- Add new features (language detection, VAD, etc.)
- Download tests for new features
- 700 additional test cases

**Total Timeline**: ~12-15 weeks for comprehensive coverage (3,000+ test cases)

---

## Prioritization Recommendations

### Start with Highest ROI:

**Week 1-2** (N=228-235, 8 commits):
1. Download Tier 1 most-common combinations:
   - transcription × (MP4, WAV, MP3) = 15 files
   - keyframes × (MP4, MOV, MKV) = 15 files
   - object-detection × (MP4, JPEG, PNG) = 15 files
   - face-detection × (JPEG, PNG, MP4) = 15 files
   - **Total: 60 high-value test files**

**Week 3-4** (N=236-245, 10 commits):
2. Download Tier 1 remaining:
   - audio-extraction, ocr, scene-detection, diarization
   - **Total: 690 test files** (complete Tier 1)

**Week 5+**: Continue with Tier 2, format expansion, feature expansion

---

## Real Media Search Keywords (Wikimedia Commons)

### For Transcription Tests
- **Categories**: "Speeches", "Lectures", "Interviews", "Audiobooks", "Podcasts"
- **Keywords**: "speech", "lecture", "interview", "presentation", "talk"
- **Languages**: Search in multiple language categories for diversity

### For Object Detection Tests
- **Categories**: "Still life photographs", "Nature photographs", "Street photography", "Indoor scenes"
- **Keywords**: "multiple objects", "furniture", "animals", "vehicles", "food"

### For Face Detection Tests
- **Categories**: "Portrait photographs", "Group photographs", "People"
- **Keywords**: "face", "portrait", "person", "people", "headshot"

### For OCR Tests
- **Categories**: "Text photographs", "Book pages", "Signs", "Documents"
- **Keywords**: "text", "sign", "document", "book", "newspaper"

### For Audio Classification Tests
- **Categories**: "Nature sounds", "Music", "Sound effects", "Animal sounds"
- **Keywords**: "bird", "rain", "music", "applause", "dog", "car", "bell"

---

## Test Metadata Requirements

**For Each Downloaded File, Record**:

```json
{
  "test_id": "transcription_mp4_001",
  "feature": "transcription",
  "format": "mp4",
  "source": "wikimedia_commons",
  "title": "Speech by Example Person",
  "url": "https://commons.wikimedia.org/wiki/File:Example.mp4",
  "license": "CC-BY-SA-4.0",
  "size_bytes": 15728640,
  "duration_seconds": 180.5,
  "resolution": "1920x1080",
  "codec": "h264",
  "description": "Educational lecture in English",
  "download_date": "2025-11-01",
  "characteristics": {
    "language": "en",
    "speaker_count": 1,
    "audio_quality": "clear",
    "video_quality": "HD"
  }
}
```

---

## Next Steps for Worker (N=228)

**Read these documents in order**:
1. **COMPREHENSIVE_FEATURE_REPORT_N227.md** - Current state (23 functional, 5 skeleton)
2. **TEST_MATRIX_COMPREHENSIVE_PLAN.md** - This document (test matrix plan)
3. **MANAGER_GUIDANCE_MODEL_ACQUISITION.md** - Model acquisition guidance

**Your mission (N=228+)**:

**Option A: Start Test Matrix** (Recommended if USER wants tests first)
- Implement WikimediaDownloader script
- Download 60 highest-priority test files (4 features × 3 formats × 5 files)
- Validate files work with video-extract

**Option B: Implement HEIF/HEIC** (Recommended if USER wants formats first)
- Add libheif support (critical missing format)
- Then start test matrix

**Option C: Quick Win Features** (Recommended if USER wants more features)
- Add language detection, VAD, acoustic scenes
- Then test with expanded matrix

**Ask USER which to prioritize**: Tests, formats, or features?

---

## Total Work Estimate

**Test Matrix**:
- Phase 1-2 (Tier 1+2): 1,750 test cases, ~23 commits, ~4-5 weeks
- Phase 3 (Format expansion): 600 test cases, ~20 commits, ~3 weeks
- Phase 4 (Feature expansion): 700 test cases, ~30 commits, ~5 weeks
- **Total**: 3,050 test cases, ~73 commits, ~12-15 weeks

**Plus**:
- Format additions: ~10-15 commits
- Feature additions: ~80-150 commits
- Model acquisition: ~1-3 commits

**Grand Total**: ~150-250 commits, ~6-12 months of work available

**Current issue**: AI stuck in loop because optimization work exhausted. This provides CLEAR next steps.
