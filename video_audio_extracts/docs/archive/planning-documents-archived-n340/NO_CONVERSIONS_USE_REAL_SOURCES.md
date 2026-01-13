# [MANAGER] STOP: Worker is Converting Again - Use Real Sources

USER OBSERVATION (correct): "I see that the worker is simply converting files, which isn't great."

USER IS RIGHT. Worker is converting files AGAIN despite directive.

---

## EVIDENCE

test_files_wikimedia/mp3/transcription/03_audio.mp3 - Generic name "audio.mp3" (suspicious)
test_files_wikimedia/mp4/keyframes/01_video.mp4 - Generic name "video.mp4" (suspicious)

Check encoder tags - if "Lavc" (ffmpeg) = conversion, not original.

---

## STRICT RULE (NO EXCEPTIONS)

**NO CONVERSIONS**

User rejected this 3 times:
1. "fake conversions not good"  
2. "doesn't capture nuances of real files"
3. "I see worker converting files, which isn't great"

**If it's a conversion, DELETE IT.**

---

## USE THESE REAL SOURCES

**Priority 1: Wikimedia Commons MP3** (worker found some!)
- Aphex Twin interview (15MB) ✅ REAL
- Carla Scaletti (2.7MB) ✅ REAL
- Search more: "Interviews", "Speeches", "Music"

**Priority 2: file-examples.com** (user provided)
```bash
wget https://file-examples.com/storage/fe7119f849b8c6cd7c1707a/2017/04/file_example_MP4_480_1_5MG.mp4
wget https://file-examples.com/storage/fe7119f849b8c6cd7c1707a/2017/04/file_example_MP4_640_3MG.mp4
wget https://file-examples.com/storage/fe7119f849b8c6cd7c1707a/2017/04/file_example_MP4_1280_10MG.mp4
```

**Priority 3: Freesound.org** (user suggested)
- MP3 audio files
- Creative Commons licensed
- Real recordings (not synthetic)
- API available: https://freesound.org/docs/api/

**Priority 4: Local Files** (manager found)
- Kinetics dataset: 20+ MP4 files in Dropbox
- Desktop: 3 MP4 files (38-89MB)
- Dropbox: 10+ MP3 audiobooks

**Priority 5: Local MOV files**
- benchmark_n103/test1.mov (34MB)
- Desktop: relevance-annotations-first-pass.mov (97MB)

---

## VERIFICATION

**For EACH file, check**:
```bash
ffprobe -v quiet -show_format <file> | grep encoder
```

**If encoder = "Lavc" or "ffmpeg"**: CONVERSION - Delete it
**If encoder = original (HandBrake, x264, camera, etc.)**: REAL - Keep it

---

## COMMANDS FOR N=323

```bash
# Remove any conversions
find test_files_wikimedia/{mp3,mp4} -name "*audio.mp3" -o -name "*video.mp4" -delete

# Download from file-examples.com
cd test_files_wikimedia/mp4/keyframes
wget https://file-examples.com/storage/fe7119f849b8c6cd7c1707a/2017/04/file_example_MP4_480_1_5MG.mp4
wget https://file-examples.com/storage/fe7119f849b8c6cd7c1707a/2017/04/file_example_MP4_640_3MG.mp4

# Copy local Kinetics MP4
cp ~/Library/CloudStorage/Dropbox*/Kinetics*/carving\ ice/*.mp4 test_files_wikimedia/mp4/action-recognition/

# Download Wikimedia MP3 (found by worker)
python3 tools/download_wikimedia_tests.py transcription mp3 5 "Interviews" 1000000 20000000
python3 tools/download_wikimedia_tests.py audio-classification mp3 5 "Music" 1000000 20000000

# Verify NO conversions
for f in test_files_wikimedia/{mp3,mp4}/*/*.{mp3,mp4}; do
  encoder=$(ffprobe -v quiet -show_format "$f" 2>&1 | grep encoder)
  if echo "$encoder" | grep -qi "lavc\|ffmpeg"; then
    echo "CONVERSION DETECTED: $f - DELETE THIS"
  fi
done
```

---

## USER WANTS REAL FILES

Not conversions. Real files from real sources:
- Wikimedia (original uploads)
- file-examples.com (sample files)
- Freesound.org (real recordings)
- Local files (real recordings)

**Get 20-30 real MP4/MP3 files** without any conversions.
