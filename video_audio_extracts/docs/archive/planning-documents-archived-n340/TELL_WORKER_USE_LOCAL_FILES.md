# [MANAGER] Use Local MP4/MP3 Files - Wikimedia Doesn't Have Them

Worker discovered: Wikimedia Commons primarily has WebM, not MP4.

USER REJECTED CONVERSIONS: "fake conversions not good"

SOLUTION: Use existing local files from your computer.

---

## Local MP4 Files Found

Search results from COMPLETE_TEST_FILE_INVENTORY.md:

**~/Desktop/stuff/stuff/**:
- mission control video demo 720.mov (277MB) - TOO LARGE
- relevance-annotations-first-pass.mp4 (97MB) ✅ Under 100MB
- Investor update - Calendar Agent.mp4 (366MB) - TOO LARGE  

**~/video_audio_extracts/benchmark_n103/**:
- test1.mov (34MB) ✅
- test2.mp4 (38MB) ✅

**Use these smaller files**:
```bash
cp ~/video_audio_extracts/benchmark_n103/test2.mp4 test_files_wikimedia/mp4/transcription/01_benchmark_test.mp4
cp ~/video_audio_extracts/benchmark_n103/test1.mov test_files_wikimedia/mov/transcription/01_benchmark_test.mov
cp ~/Desktop/stuff/stuff/relevance-annotations-first-pass.mp4 test_files_wikimedia/mp4/keyframes/01_relevance_annotations.mp4
```

---

## Local MP3 Files

**From Dropbox** (COMPLETE_TEST_FILE_INVENTORY.md):
- ~/Library/CloudStorage/Dropbox-*/a.test/public/librivox/
  - fabula_01_024_esopo_64kb.mp3 (375KB) ✅
  - fabula_01_018_esopo_64kb.mp3 (1.1MB) ✅

**Use these**:
```bash
find ~/Library/CloudStorage/Dropbox* -name "*.mp3" -size -10M | head -20 | while read f; do
  echo "Found: $f"
done
```

---

## Alternative: Internet Archive

**Internet Archive has diverse MP4/MP3 files**:

Search: https://archive.org/details/movies
- Filter by: MP4, < 100MB
- Public domain content
- Download URLs available

```python
import requests

# Internet Archive API
url = "https://archive.org/advancedsearch.php"
params = {
    "q": "mediatype:movies AND format:MPEG4",  # MP4 files
    "fl[]": ["identifier", "title", "downloads"],
    "rows": 50,
    "output": "json"
}

response = requests.get(url, params=params)
items = response.json()["response"]["docs"]

# Download items with size < 100MB
for item in items:
    # Check file size, download if suitable
    ...
```

---

## Alternative: Sample Media Repositories

**CC0 (Public Domain) video sources**:
- Pexels Videos: https://www.pexels.com/videos/ (MP4, free license)
- Pixabay Videos: https://pixabay.com/videos/ (MP4, CC0)
- Videvo: https://www.videvo.net/ (MP4, free clips)

**CC0 audio sources**:
- Freesound: https://freesound.org/ (MP3, WAV, various licenses)
- Free Music Archive: https://freemusicarchive.org/ (MP3, various licenses)
- ccMixter: http://ccmixter.org/ (MP3, Creative Commons)

---

## RECOMMENDATION FOR WORKER

**Priority 1**: Use local files
- Search ~/Desktop, ~/Downloads, benchmark_n103, Dropbox
- Copy existing MP4/MOV/MP3 files <100MB
- Mark as "local" in metadata.json (not Wikimedia)

**Priority 2**: Internet Archive
- Large public domain collection
- Has MP4 files
- Download via API

**Priority 3**: CC0 media sites
- Pexels, Pixabay (MP4 videos)
- Freesound (MP3 audio)
- Properly licensed, downloadable

**Do NOT**: Convert WebM → MP4 (user rejected this)

---

## Commands for Worker (N=323)

```bash
# Use local files
cp ~/video_audio_extracts/benchmark_n103/test2.mp4 test_files_wikimedia/mp4/transcription/01_local_benchmark.mp4
cp ~/video_audio_extracts/benchmark_n103/test1.mov test_files_wikimedia/mov/keyframes/01_local_benchmark.mov

# Find more local MP4/MP3
find ~/Desktop ~/Downloads -name "*.mp4" -size -100M -type f | head -10
find ~/Downloads -name "*.mp3" -size -50M -type f | head -10

# Mark source in metadata
cat > test_files_wikimedia/mp4/transcription/metadata.json << 'JSON'
{
  "source": "local",
  "files": [
    {"path": "01_local_benchmark.mp4", "original": "~/video_audio_extracts/benchmark_n103/test2.mp4", "size": 38000000}
  ]
}
JSON
```

This gives you real MP4/MP3 files with real encoding diversity (not conversions).

---

## USER PROVIDED SOURCE

**file-examples.com**: https://file-examples.com/index.php/sample-video-files/sample-mp4-files/

Sample MP4 files available:
- Various sizes (1MB-20MB range)
- Different codecs and quality levels
- Free download
- Good for testing

**Download these**:
```bash
# Small MP4 samples from file-examples.com
wget https://file-examples.com/storage/fe7119f849b8c6cd7c1707a/2017/04/file_example_MP4_480_1_5MG.mp4
wget https://file-examples.com/storage/fe7119f849b8c6cd7c1707a/2017/04/file_example_MP4_640_3MG.mp4
wget https://file-examples.com/storage/fe7119f849b8c6cd7c1707a/2017/04/file_example_MP4_1280_10MG.mp4

# Place in test_files_wikimedia/mp4/
```

---

## MANAGER FOUND ON YOUR COMPUTER

**Kinetics Dataset MP4 files** (Dropbox):
- 20+ MP4 files in ~/Library/CloudStorage/Dropbox*/Kinetics dataset/carving ice/
- All under 20MB (small clips)
- Real YouTube videos (diverse encoding)

**Desktop MP4 files**:
- May 5 - live labeling mocks.mp4 (38MB) ✅
- video1509128771.mp4 (75MB) ✅
- video1171640589.mp4 (89MB) ✅

**Dropbox MP3 files**:
- librivox audiobooks: fabula_01_*.mp3 (375KB-1.1MB each)
- 10+ files available

**Use these local files** - they're real media with diverse encoding!

---

## Priority Order for N=323

1. **Local files** (best): Copy from Dropbox/Desktop (real encoding diversity)
2. **file-examples.com** (good): Download sample MP4/MP3 files
3. **Internet Archive** (excellent): Public domain, diverse content
4. **Pexels/Pixabay** (good): Free CC0 videos

**DO NOT**: Convert WebM → MP4 (user rejected)

Commands ready in this file. Execute at N=323.
