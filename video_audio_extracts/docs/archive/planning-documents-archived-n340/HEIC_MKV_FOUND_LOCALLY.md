# [MANAGER] HEIC and MKV Files Found Locally!

USER: "We absolutely want those HEIC files!"

MANAGER FOUND THEM on your computer!

---

## HEIC Files Found (iPhone Photos)

**Location**: ~/Desktop/stuff/stuff/
- IMG_2153.HEIC
- IMG_2154.HEIC
- IMG_2155.HEIC
- IMG_2156.HEIC
- IMG_2157.HEIC

**These are REAL iPhone photos** ✅

**Copy to project** (N=326):
```bash
mkdir -p test_files_wikimedia/heic/face-detection
mkdir -p test_files_wikimedia/heic/object-detection
mkdir -p test_files_wikimedia/heic/vision-embeddings
mkdir -p test_files_wikimedia/heic/ocr

cp ~/Desktop/stuff/stuff/IMG_2153.HEIC test_files_wikimedia/heic/face-detection/01_iphone_photo.heic
cp ~/Desktop/stuff/stuff/IMG_2154.HEIC test_files_wikimedia/heic/object-detection/01_iphone_photo.heic
cp ~/Desktop/stuff/stuff/IMG_2155.HEIC test_files_wikimedia/heic/vision-embeddings/01_iphone_photo.heic
cp ~/Desktop/stuff/stuff/IMG_2156.HEIC test_files_wikimedia/heic/ocr/01_iphone_photo.heic
cp ~/Desktop/stuff/stuff/IMG_2157.HEIC test_files_wikimedia/heic/face-detection/02_iphone_photo.heic

# Create metadata
cat > test_files_wikimedia/heic/face-detection/metadata.json << 'JSON'
{
  "source": "local_iphone_photos",
  "description": "Real iPhone HEIC photos from ~/Desktop/stuff/stuff/",
  "format": "heic",
  "files": [
    {"path": "01_iphone_photo.heic", "original": "IMG_2153.HEIC"},
    {"path": "02_iphone_photo.heic", "original": "IMG_2157.HEIC"}
  ]
}
JSON
```

**Result**: 5+ HEIC test files (real iPhone photos!)

---

## MKV Files Found (Matroska Videos)

**Location**: ~/Library/CloudStorage/Dropbox*/a.test/

**Kinetics Dataset** (11MB):
- ice climbing/eb9jznQnhK8_raw.mkv

**Low-Res Super Resolution Dataset** (20+ files, ~10MB each):
- Youku_00072_l.mkv
- Youku_00069_l.mkv
- Youku_00070_l.mkv
- Youku_00089_l.mkv
- Youku_00074_l.mkv
- (15+ more available)

**Copy to project** (N=326):
```bash
mkdir -p test_files_wikimedia/mkv/keyframes
mkdir -p test_files_wikimedia/mkv/transcription
mkdir -p test_files_wikimedia/mkv/object-detection

# Kinetics MKV
cp ~/Library/CloudStorage/Dropbox*/a.test/Kinetics*/*/ice\ climbing/eb9jznQnhK8_raw.mkv \
   test_files_wikimedia/mkv/keyframes/01_kinetics_ice_climbing.mkv

# Low-Res MKV files (copy 5-10)
find ~/Library/CloudStorage/Dropbox*/a.test/Low-Res*/converted/mkv/*/*.mkv | head -10 | while read f; do
  name=$(basename "$f")
  cp "$f" test_files_wikimedia/mkv/transcription/$name
done
```

**Result**: 10-15 MKV test files

---

## Execute at N=326

**HEIC first** (user priority: "We absolutely want those HEIC files!")
1. Copy 5 HEIC files from ~/Desktop/stuff/stuff/
2. Place in heic/face-detection, heic/object-detection, heic/vision-embeddings
3. Verify HEIC format supported (test added at N=241)

**MKV second**:
1. Copy 10-15 MKV files from Dropbox Kinetics + Low-Res datasets
2. Place in mkv/keyframes, mkv/transcription
3. Test with video-extract

**Commit**: "# 326: Add HEIC (iPhone Photos) and MKV (Matroska) Formats"

---

## Alternative HEIC Sources (if need more)

**Generate from JPEG** (macOS sips command):
```bash
sips -s format heic input.jpg --out output.heic
```

**Search Wikimedia** (limited but possible):
```bash
python3 tools/download_wikimedia_tests.py face-detection heic 5 "Photographs" 100000 5000000
```

But local IMG_*.HEIC files are perfect - real iPhone photos!

---

## USER WANTS HEIC

"We absolutely want those HEIC files!"

You have 5 real iPhone HEIC files in ~/Desktop/stuff/stuff/

Copy them at N=326. This is high priority.
