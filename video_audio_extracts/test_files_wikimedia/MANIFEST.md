# Wikimedia Commons Test Files - Tier 1 Original Files Only

**Downloaded**: 2025-11-01 (N=243, N=246, N=252, N=253, N=257, N=258, N=259, N=261, N=262, N=263, N=264, N=265, N=266)
**File Reuse Expansion**: 2025-11-01 (N=254 - reused existing files for new features)
**Partial Coverage Completion**: 2025-11-01 (N=257 - completed partial JPG/FLAC coverage)
**Test Matrix Depth Expansion**: 2025-11-01 (N=258 - increased files per cell for high-priority features)
**New Category Exploration**: 2025-11-01 (N=259 - discovered Music videos, Educational videos, Animations categories)
**Massive JPG Category Expansion #1**: 2025-11-01 (N=261 - explored Architecture, Landscapes, Sports, People, Signs, Technology categories, +56 unique JPG files, 96.6% unique rate)
**Massive JPG Category Expansion #2**: 2025-11-01 (N=262 - explored Wildlife, Art, Food, Interior design, Fashion, Astronomy categories, +62 unique JPG files, 98.4% unique rate)
**Massive JPG Category Expansion #3**: 2025-11-01 (N=263 - explored Flowers, Insects, Trees, Gardens, Mountains, Monuments, Bridges, Paintings, Sculptures, Streets, Waterfalls, Beaches, Forests, Animals categories, +130 unique JPG files, 99.2% unique rate)
**Massive JPG Category Expansion #4**: 2025-11-01 (N=264 - explored Rivers, Lakes, Sunsets, Geology, Castles, Ships, Buildings, Cities, Rocks, Cats, Dogs, Statues, Birds of prey, Skies, Caves, Deserts, Airplanes categories, +162 unique JPG files, 100% unique rate)
**Massive JPG Category Expansion #5**: 2025-11-01 (N=265 - explored Boats, Temples, Horses, Fish, Motorcycles, Bicycles, Furniture, Tools, Books, Newspapers, Babies, Children, Automobiles, Trucks, Clocks, Stairs, Posters, Athletes, Flags, Coins, Diagrams, Magazines, Jewellery, Paintings (more), Corridors, Billboards, +243 unique JPG files, 96.0% unique rate)
**TIER 1 COMPLETION**: 2025-11-01 (N=266 - explored Fruits, Vegetables, Musical instruments, Shops, Dancers, Festivals, Pottery, Textiles, Postal cards categories, +90 unique JPG files, 100% unique rate, TIER 1 TARGET EXCEEDED: 801/750 unique files = 106.8%)
**Converted files deleted**: 2025-11-01 (N=251 - Manager directive)
**Total files on disk**: 2,945 media files (19 GB, many are copied across features via file reuse strategy)
**Unique source files**: 801 unique files by content hash (28 WEBM + 756 JPG + 5 PNG + 8 WAV + 4 FLAC)
**Files on disk breakdown**: WEBM (209), JPG (2,510), PNG (60), WAV (56), FLAC (20)
**Source**: 100% original Wikimedia Commons downloads (no conversions)
**Purpose**: Test Matrix Phase 2 - Real-world test coverage with genuine encoding diversity

**Note on file counting**: Many files are reused across features (e.g., same WEBM file used for transcription, keyframes, scene-detection). The "unique source files" count represents actual distinct media files, while "total files on disk" includes all copies/reuses.

---

## Manager Directive (N=250)

**USER feedback**: "Format conversion doesn't capture nuances of real files with different encoding patterns"

**Corrective action taken at N=251**:
- Deleted 36 converted files (22 MP4 + 14 MOV)
- All converted files had identical encoding (ffmpeg, CRF 23, fast preset, H.264+AAC)
- Conversion created FAKE diversity - all MP4s had same encoder/settings
- Real MP4s have diverse encoders (cameras, HandBrake, different tools/settings)
- **Real files test real-world encoding diversity**

**Result**: 170 files → 134 files (kept only original Wikimedia downloads)

---

## Summary by Format

- **flac**: 20 files across 5 features (4 files per feature, reused across features)
- **jpg**: 2,600 files across 13 features (varies 180-220 files per feature) - +4 at N=258, +1 at N=259, +56 at N=261, +63 at N=262, +130 at N=263, +162 at N=264, +253 at N=265, +90 at N=266
- **png**: 60 files across 12 features (5 files per feature via reuse)
- **wav**: 56 files across 7 features (8 files per feature via reuse)
- **webm**: 209 files across 19 features (varies 5-25 files per feature) - +50 at N=258, +15 at N=259, +49 at N=261

**Total**: 2,945 files on disk across **55 (feature, format) combinations**
**Unique source files**: 801 by content hash (28 WEBM + 756 JPG + 5 PNG + 8 WAV + 4 FLAC)
**Note**: File reuse strategy means same source file used across multiple features. "Files on disk" counts physical copies; "unique source files" counts distinct media by hash.

---

## Files by (Feature, Format)

### FLAC Format (20 files) - EXPANDED at N=254, N=257
- **audio-embeddings**: 4 files (FLAC files)
- **audio-extraction**: 4 files (FLAC files) - ✓ COMPLETED at N=257
- **diarization**: 4 files (reused from audio-embeddings) ✓ NEW at N=254
- **metadata-extraction**: 4 files (reused from audio-embeddings) ✓ NEW at N=254
- **transcription**: 4 files (FLAC files) - ✓ COMPLETED at N=257
**Note**: File reuse via copying. Wikimedia has limited FLAC availability (<5 files found), 4 files per feature is acceptable.

### JPG Format (68 files) - EXPANDED at N=254, N=257
- **caption-generation**: 6 files (reused from vision-embeddings) ✓ NEW at N=254
- **content-moderation**: 6 files (reused from vision-embeddings) ✓ NEW at N=254
- **depth-estimation**: 6 files (reused from vision-embeddings) ✓ NEW at N=254
- **emotion-detection**: 5 files (Portrait photographs) ✓
- **face-detection**: 5 files (Portrait photographs) ✓
- **image-quality-assessment**: 5 files (JPEG files) - ✓ COMPLETED at N=257
- **logo-detection**: 6 files (reused from vision-embeddings) ✓ NEW at N=254
- **object-detection**: 5 files (JPEG files) - ✓ COMPLETED at N=257
- **ocr**: 6 files (JPEG files with text) ✓
- **pose-estimation**: 6 files (JPEG files) ✓
- **shot-classification**: 6 files (reused from vision-embeddings) ✓ NEW at N=254
- **vision-embeddings**: 6 files (JPEG files) ✓

### PNG Format (60 files) - EXPANDED at N=254
- **caption-generation**: 5 files (reused from vision-embeddings) ✓ NEW at N=254
- **content-moderation**: 5 files (reused from vision-embeddings) ✓ NEW at N=254
- **depth-estimation**: 5 files (reused from vision-embeddings) ✓ NEW at N=254
- **emotion-detection**: 5 files (PNG files) ✓
- **face-detection**: 5 files (PNG files) ✓
- **image-quality-assessment**: 5 files (PNG files) ✓
- **logo-detection**: 5 files (reused from vision-embeddings) ✓ NEW at N=254
- **object-detection**: 5 files (PNG files) ✓
- **ocr**: 5 files (PNG files with text) ✓
- **pose-estimation**: 5 files (PNG files) ✓
- **shot-classification**: 5 files (reused from vision-embeddings) ✓ NEW at N=254
- **vision-embeddings**: 5 files (PNG files) ✓

### WAV Format (56 files) - EXPANDED at N=254
- **audio-classification**: 8 files (WAV files) ✓
- **audio-embeddings**: 8 files (WAV files) ✓
- **audio-enhancement-metadata**: 8 files (reused from audio-classification) ✓ NEW at N=254
- **audio-extraction**: 8 files (WAV files) ✓
- **diarization**: 8 files (reused from audio-classification) ✓ NEW at N=254
- **metadata-extraction**: 8 files (reused from audio-classification) ✓ NEW at N=254
- **transcription**: 8 files (WAV files) ✓

### WEBM Format (95 files) - EXPANDED at N=254
- **action-recognition**: 5 files (Videos)
- **audio-classification**: 5 files (Videos)
- **audio-embeddings**: 5 files (Videos)
- **audio-extraction**: 5 files (Videos) - completed at N=252
- **diarization**: 5 files (Videos) - completed at N=252
- **emotion-detection**: 5 files (Videos)
- **face-detection**: 5 files (Videos)
- **format-conversion**: 5 files (reused from action-recognition) ✓ NEW at N=254
- **keyframes**: 5 files (Videos) - completed at N=252
- **metadata-extraction**: 5 files (reused from action-recognition) ✓ NEW at N=254
- **motion-tracking**: 5 files (Videos)
- **object-detection**: 5 files (Videos)
- **pose-estimation**: 5 files (Videos)
- **scene-detection**: 5 files (Videos)
- **shot-classification**: 5 files (Videos)
- **smart-thumbnail**: 5 files (Videos)
- **subtitle-extraction**: 5 files (reused from action-recognition) ✓ NEW at N=254
- **transcription**: 5 files (Videos) - completed at N=252
- **vision-embeddings**: 5 files (Videos)

---

## Wikimedia Categories Used

- **Videos**: General Wikimedia videos category (webm format)
- **Animations**: 3D animations and motion graphics (webm format)
- **JPEG files**: Broad category for diverse JPG images
- **PNG files**: Broad category for PNG images
- **Portrait photographs**: Category for face/emotion detection tests

---

## Download History

### N=243: Tier 1 Batch 1 (62 files, 294 MB)
- Initial downloads across JPG, PNG, WEBM formats
- 13 files over 100MB excluded (GitHub file size limit)
- Learned: Wikimedia has limited MP4/MOV coverage, WebM is primary video format

### N=246: Tier 1 Batch 2 (71 files, 4.5 GB)
- Expanded coverage to 31 (feature, format) combinations
- Added PNG variants for all image features
- Added WEBM variants for all video features
- All files under 100MB per file (but large total size due to video content)

### N=247-249: Format Conversions (DELETED at N=251)
- N=247: Attempted expansion blocked by file size limits
- N=248-249: Created 36 converted MP4/MOV files (deleted at N=251 per manager directive)

### N=252: Audio Format Expansion + Complete Partial Coverage (28 files, 300 MB)
- **Completed partially-covered features**: Added 7 WEBM files to complete audio-extraction (3→5), diarization (3→5), transcription (3→5), keyframes (4→5)
- **Added audio-only formats**: 12 WAV files (4 features × 3 files each) + 9 FLAC files (3 features × 3 files each)
- **All files <100MB**: Updated downloader script with MAX_SIZE=99_000_000 filter
- **Result**: 134 → 162 files, 31 → 41 (feature, format) combinations
- **MP4/MOV coverage**: Attempted but no files <100MB found in Wikimedia Commons (confirmed N=243 findings)
- N=248-249: Created 36 converted MP4/MOV files from WebM sources
- **Problem identified**: Conversions had uniform encoding (all H.264 CRF 23, AAC)
- **Manager directive**: Delete conversions, keep original files only
- **N=251**: Deleted all converted files per directive

### N=258: Test Matrix Depth Expansion (+7 Unique Files, 54 Files on Disk)
- **Strategy shift**: Increase depth of existing (feature, format) combinations rather than adding new formats
- **Downloads**:
  - WEBM: 10 files each for transcription, keyframes, scene-detection, face-detection, action-recognition
  - WEBM: 3 files from "Animations" category for vision-embeddings
  - JPG: 4 files for object-detection (from "Photographs" and "Still life photographs")
- **Result**: ~7 new unique source files (3 unique WEBM + 4 unique JPG), +54 files on disk via reuse
- **File counts**: 299→353 files on disk, 15 unique WEBM (12→15), 12 unique JPG (9→12)
- **Disk usage**: 5.9 GB → 7.7 GB
- **Tests**: 49/49 comprehensive smoke tests passing (45.79s)
- **Note**: Many downloaded files are duplicates/reuses due to limited Wikimedia availability in "Videos" category

### N=259: New Category Exploration (+14 Unique Files, 16 Files on Disk)
- **Strategy shift**: Explore new Wikimedia categories to acquire truly unique source files (not just depth expansion via reuse)
- **New categories discovered**:
  - **"Music videos"**: 10 WEBM files (scene-detection) - Excellent category with diverse music videos <100MB
  - **"Educational videos"**: 3 WEBM files (transcription) - Museum education, teaching methodology videos
  - **"Animations"**: 3 WEBM files (keyframes) - 3D animations, lighting tutorials
  - **"Food photographs"**: 1 JPG file (object-detection) - Food photography
- **Result**: +14 unique source files (13 unique WEBM + 1 unique JPG), +16 files on disk
- **File counts**: 353→369 files on disk, 28 unique WEBM (15→28), 13 unique JPG (12→13)
- **Disk usage**: 7.7 GB → 8.4 GB (+0.7 GB)
- **Tests**: 49/49 comprehensive smoke tests passing (93.62s)
- **Progress**: 58/750 = 7.7% of Tier 1 target (improved from 5.9%)
- **Key insight**: "Music videos" category is highly productive for WEBM <100MB, yielding 10 diverse files in single download batch

---

## Validation

- ✅ All files validated with ffprobe (JPG, PNG, WEBM)
- ✅ Representative files tested with video-extract (keyframes, audio operations)
- ✅ WebM files: VP8/VP9 video codec, various resolutions (mostly 720x540, 30fps), Opus/Vorbis audio
- ✅ JPG files: JPEG images, various resolutions
- ✅ PNG files: RGB24 format, various resolutions (some >2000px wide)
- ✅ 49/49 comprehensive smoke tests passing (N=250)

---

## Metadata

Each (format, feature) directory contains:
- **[01-05]_*.{ext}**: Downloaded media files
- **metadata.json**: Source URLs, file sizes, titles, MIME types from Wikimedia Commons API

---

## Coverage Analysis

### Fully Covered Features (5-15 files) - DEPTH EXPANDED at N=258 ✓
- **action-recognition**: WEBM (10) ✓ EXPANDED
- **audio-classification**: WAV (8), WEBM (5) - 13 files across 2 formats ✓
- **audio-embeddings**: FLAC (4), WAV (8), WEBM (5) - 17 files across 3 formats ✓
- **audio-extraction**: FLAC (4), WAV (8), WEBM (5) - 17 files across 3 formats ✓
- **diarization**: WEBM (5) ✓
- **emotion-detection**: JPG (5), PNG (5), WEBM (5) - 15 files across 3 formats ✓
- **face-detection**: JPG (5), PNG (5), WEBM (10) - 20 files across 3 formats ✓ EXPANDED
- **image-quality-assessment**: JPG (5), PNG (5) - 10 files across 2 formats ✓
- **keyframes**: WEBM (10) ✓ EXPANDED
- **motion-tracking**: WEBM (5) ✓
- **object-detection**: JPG (9), PNG (5), WEBM (5) - 19 files across 3 formats ✓ EXPANDED
- **ocr**: JPG (6), PNG (5) - 11 files across 2 formats ✓
- **pose-estimation**: JPG (6), PNG (5), WEBM (5) - 16 files across 3 formats ✓
- **scene-detection**: WEBM (10) ✓ EXPANDED
- **shot-classification**: WEBM (5) ✓
- **smart-thumbnail**: WEBM (5) ✓
- **transcription**: FLAC (4), WAV (8), WEBM (10) - 22 files across 3 formats ✓ EXPANDED
- **vision-embeddings**: JPG (6), PNG (5), WEBM (8) - 19 files across 3 formats ✓ EXPANDED

### Partially Covered Features - NONE (all completed at N=257)
**N=257 completed partial coverage**:
- image-quality-assessment (JPG): 3→5 files (+2 new JPG downloads)
- object-detection (JPG): 4→5 files (+1 new JPG download)
- audio-extraction (FLAC): 3→4 files (+1 file reuse, Wikimedia has <5 FLAC files available)
- transcription (FLAC): 3→4 files (+1 file reuse, Wikimedia has <5 FLAC files available)

---

## Format Coverage Gaps (Post-Deletion)

### No longer covered (after N=251 deletion):
- **MP4 format**: 0 files (deleted 22 converted files)
- **MOV format**: 0 files (deleted 14 converted files)
- **MKV format**: 0 files (never added)
- **Audio-only formats** (WAV, MP3, M4A, OGG): 0 files

### Strategy going forward:
- **Download original MP4/MOV files** from Wikimedia (if available <100MB)
- **Accept 3-4 files per cell** (quality over quantity)
- **Prioritize encoding diversity** over file count
- **70 real diverse files > 170 mixed real+converted**

---

## Next Steps (N=251+)

### Priority 1: Download Original MP4/MOV Files
- Update downloader with max_size=99_000_000 (GitHub 100MB limit)
- Re-download original MP4/MOV files <100MB from Wikimedia
- Accept 3-4 files per (feature, format) cell if that's what's available
- Target: 20-40 original MP4/MOV files from Wikimedia

### Priority 2: Add Audio-Only Format Coverage
- Download original WAV, MP3, M4A, OGG files from Wikimedia
- Focus on audio-extraction, audio-embeddings, audio-classification, transcription, diarization
- Target: 15-30 original audio files

### Priority 3: Complete Partially-Covered Features
- Download 2 more WEBM files for: audio-extraction, diarization, transcription (need 5 each)
- Download 1 more WEBM file for keyframes (need 5)
- Target: +7 files to complete these features

### Priority 4: Update Smoke Tests
- Add representative smoke tests for new files
- Current: 49 smoke tests
- Target: 55-65 smoke tests after adding original MP4/MOV coverage

---

## Known Issues

### Large File Sizes
- Total: 4.8 GB (approaching GitHub repository size limits)
- Individual files >100MB cannot be committed (GitHub hard limit)
- Consider: Git LFS for large test files, or external storage
- Alternative: Focus on smaller files (<50MB) going forward

### Wikimedia Format Limitations
- WebM is dominant video format (MP4/MOV rare in Wikimedia Commons)
- Audio formats (WAV, MP3, M4A) very limited
- Some categories empty or low coverage
- Broad categories yield better results than specific categories
- Broad "Videos" category returns large files (consistently >100MB)

### GitHub File Size Constraints
- GitHub rejects individual files >100MB
- N=243 excluded 13 large files
- N=247 excluded 4 large files
- Must use max_size filter in downloader going forward

---

## File Integrity

All files verified:
- ✅ ffprobe validation (correct codecs, no corruption)
- ✅ video-extract testing (keyframes, audio extraction working)
- ✅ Metadata preserved (source URLs, sizes, MIME types)

---

## Current Status (N=265)

**Progress**: 711 unique source files / 2,855 files on disk - **94.8% of Tier 1 target** (711/750 unique files)

**Coverage Summary**:
- **55 (feature, format) combinations** (stable since N=254)
- Image features: JPG (13 features, 2,510 files) + PNG (12 features, 60 files) - **massive JPG expansion #5** ✓
- Video features: WEBM (19 features, 209 files) - comprehensive VP8/VP9 encoding coverage ✓
- Audio features: WAV (7 features, 56 files) + FLAC (5 features, 20 files) - strong audio coverage ✓
- **All 55 combinations have 4+ files per cell** (most JPG features now have 180-210 files) ✓
- **NO partial coverage remaining** - all (feature, format) cells have ≥4 files ✓

**Unique Source Files** (by hash):
- WEBM: 28 unique files (Music videos, Educational videos, Animations, generic Videos)
- JPG: 666 unique files (Rivers, Lakes, Sunsets, Geology, Castles, Ships, Buildings, Cities, Rocks, Cats, Dogs, Statues, Birds of prey, Skies, Caves, Deserts, Airplanes, Flowers, Insects, Trees, Gardens, Mountains, Monuments, Bridges, Paintings, Sculptures, Streets, Waterfalls, Beaches, Forests, Animals, Wildlife, Art, Food, Interior design, Fashion, Astronomy, Architecture, Landscapes, Sports, People, Signs, Technology, Photographs, Portrait photographs, Boats, Temples, Horses, Fish, Motorcycles, Bicycles, Furniture, Tools, Books, Newspapers, Babies, Children, Automobiles, Trucks, Clocks, Stairs, Posters, Athletes, Flags, Coins, Diagrams, Magazines, Jewellery, Corridors, Billboards)
- PNG: 5 unique files (PNG files category)
- WAV: 8 unique files (Audio files in WAV format)
- FLAC: 4 unique files (Audio files in FLAC format)

**Format Gaps** (Wikimedia availability constraints):
- Video formats MP4/MOV: 0 files (Wikimedia Commons lacks <100MB files)
- Audio formats MP3, M4A, OGG: 0 files (limited availability in Wikimedia Commons)

**Recent Strategy (N=258-265)**:
- N=258 (depth expansion): Added more files to existing cells, but limited unique diversity (~13% unique rate)
- N=259 (category exploration): Discovered productive new categories ("Music videos", "Educational videos"), achieved 87.5% unique rate
- N=261 (JPG category explosion #1): Explored 6 new JPG categories, achieved **96.6% unique rate** (+56 unique files)
- N=262 (JPG category explosion #2): Explored 6 more JPG categories, achieved **98.4% unique rate** (+62 unique files)
- N=263 (JPG category explosion #3): Explored 14 new JPG categories, achieved **99.2% unique rate** (+130 unique files)
- N=264 (JPG category explosion #4): Explored 17 new JPG categories, achieved **100% unique rate** (+162 unique files)
- N=265 (JPG category explosion #5): Explored 26 new JPG categories, achieved **96.0% unique rate** (+243 unique files)
- **Key lesson**: JPG categories on Wikimedia are extremely diverse - nearly every file downloaded is unique (96-100% unique rate sustained across 5 sessions)

**Productive JPG Categories Discovered (N=261-265)**:
- ✅ "Flowers" (10 files, ~100% unique) - N=263
- ✅ "Insects" (10 files, ~100% unique) - N=263
- ✅ "Trees" (10 files, ~100% unique) - N=263
- ✅ "Gardens" (10 files, ~100% unique) - N=263
- ✅ "Mountains" (10 files, ~100% unique) - N=263
- ✅ "Monuments" (10 files, ~100% unique) - N=263
- ✅ "Bridges" (10 files, ~100% unique) - N=263
- ✅ "Paintings" (10 files, ~100% unique) - N=263
- ✅ "Sculptures" (10 files, ~100% unique) - N=263
- ✅ "Streets" (7 files, ~100% unique) - N=263
- ✅ "Waterfalls" (10 files, ~100% unique) - N=263
- ✅ "Beaches" (10 files, ~100% unique) - N=263
- ✅ "Forests" (10 files, ~100% unique) - N=263
- ✅ "Animals" (3 files, ~100% unique) - N=263
- ✅ "Wildlife" (10 files, ~100% unique) - N=262
- ✅ "Art" (10 files, ~100% unique) - N=262
- ✅ "Food" (10 files, ~100% unique) - N=262
- ✅ "Interior design" (10 files, ~100% unique) - N=262
- ✅ "Fashion" (10 files, ~100% unique) - N=262
- ✅ "Astronomy" (10 files, ~100% unique) - N=262
- ✅ "Architecture" (10 files, ~100% unique) - N=261
- ✅ "Landscapes" (6 files, ~100% unique) - N=261
- ✅ "Sports" (10 files, ~100% unique) - N=261
- ✅ "People" (10 files, ~100% unique) - N=261
- ✅ "Signs" (10 files, ~100% unique) - N=261
- ✅ "Technology" (10 files, ~100% unique) - N=261
- ✅ "Rivers" (10 files, 100% unique) - N=264
- ✅ "Lakes" (10 files, 100% unique) - N=264
- ✅ "Sunsets" (10 files, 100% unique) - N=264
- ✅ "Geology" (10 files, 100% unique) - N=264
- ✅ "Castles" (10 files, 100% unique) - N=264
- ✅ "Ships" (4 files, 100% unique) - N=264
- ✅ "Buildings" (10 files, 100% unique) - N=264
- ✅ "Cities" (8 files, 100% unique) - N=264
- ✅ "Rocks" (10 files, 100% unique) - N=264
- ✅ "Cats" (10 files, 100% unique) - N=264
- ✅ "Dogs" (10 files, 100% unique) - N=264
- ✅ "Statues" (10 files, 100% unique) - N=264
- ✅ "Birds of prey" (10 files, 100% unique) - N=264
- ✅ "Skies" (10 files, 100% unique) - N=264
- ✅ "Caves" (10 files, 100% unique) - N=264
- ✅ "Deserts" (10 files, 100% unique) - N=264
- ✅ "Airplanes" (10 files, 100% unique) - N=264
- ✅ "Boats" (10 files, ~95% unique) - N=265
- ✅ "Temples" (10 files, ~100% unique) - N=265
- ✅ "Horses" (10 files, ~100% unique) - N=265
- ✅ "Fish" (10 files, ~100% unique) - N=265
- ✅ "Motorcycles" (10 files, ~95% unique) - N=265
- ✅ "Bicycles" (10 files, ~100% unique) - N=265
- ✅ "Furniture" (10 files, ~100% unique, some duplicates with content-moderation) - N=265
- ✅ "Tools" (10 files, ~100% unique) - N=265
- ✅ "Books" (10 files, ~100% unique) - N=265
- ✅ "Newspapers" (10 files, ~100% unique) - N=265
- ✅ "Babies" (10 files, ~100% unique) - N=265
- ✅ "Children" (10 files, ~100% unique) - N=265
- ✅ "Automobiles" (3 files, 100% unique) - N=265
- ✅ "Trucks" (10 files, ~100% unique) - N=265
- ✅ "Clocks" (10 files, ~100% unique) - N=265
- ✅ "Stairs" (10 files, ~100% unique) - N=265
- ✅ "Posters" (10 files, ~100% unique) - N=265
- ✅ "Athletes" (10 files, ~100% unique) - N=265
- ✅ "Flags" (10 files, ~100% unique) - N=265
- ✅ "Coins" (10 files, ~100% unique) - N=265
- ✅ "Diagrams" (10 files, ~100% unique) - N=265
- ✅ "Magazines" (10 files, ~100% unique) - N=265
- ✅ "Jewellery" (10 files, ~100% unique) - N=265
- ✅ "Paintings" (10 more files, ~100% unique) - N=265
- ✅ "Corridors" (10 files, ~100% unique) - N=265
- ✅ "Billboards" (10 files, ~100% unique) - N=265

**Next Priorities** (N=266+):
1. **CLOSE TO TIER 1 COMPLETION**: 711/750 unique files = 94.8% complete, only 39 files remaining
2. Continue JPG category exploration to reach 750+ unique files (100% of Tier 1 target)
3. Candidate JPG categories: Locomotives, Bridges (more), Churches (try alternatives), Museums (alternative names), Weather phenomena, Festivals, etc.
4. Consider updating smoke tests after reaching 750 unique files milestone
5. Transition to Phase 4 (quick-win features) or Tier 2 test matrix after Tier 1 completion
