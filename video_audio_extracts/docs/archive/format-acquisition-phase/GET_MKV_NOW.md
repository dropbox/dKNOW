# [MANAGER] MKV FILES - Found Them, Get NOW at N=351

USER: "you cannot find MKV? ultrathink"

MANAGER FOUND THEM!

---

## MKV Files Located on Your System

**Kinetics Dataset** (1 file, 11MB):
```
~/Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/Kinetics dataset (5%)/kinetics600_5per/kinetics600_5per/train/ice climbing/eb9jznQnhK8_raw.mkv
```

**Low-Res Super Resolution Dataset** (20+ files, ~10MB each):
```
~/Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/Low-Res Super Resolution Dataset video/converted/mkv/youku_00050_00099_l/*.mkv
```

Files:
- Youku_00072_l.mkv
- Youku_00069_l.mkv
- Youku_00070_l.mkv
- Youku_00089_l.mkv
- Youku_00074_l.mkv
- (15+ more)

**USER WANTS 5 FILES MINIMUM**

---

## Copy at N=351 (IMMEDIATE)

```bash
mkdir -p test_files_wikimedia/mkv/keyframes
mkdir -p test_files_wikimedia/mkv/transcription
mkdir -p test_files_wikimedia/mkv/scene-detection
mkdir -p test_files_wikimedia/mkv/object-detection
mkdir -p test_files_wikimedia/mkv/face-detection

# Copy Kinetics MKV
cp ~/Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/Kinetics\ dataset\ \(5%\)/kinetics600_5per/kinetics600_5per/train/ice\ climbing/eb9jznQnhK8_raw.mkv \
   test_files_wikimedia/mkv/keyframes/01_kinetics_ice_climbing.mkv

# Copy 5 Low-Res MKV files (minimum 5 per user requirement)
find ~/Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/Low-Res\ Super\ Resolution\ Dataset\ video/converted/mkv/youku_00050_00099_l/ -name "*.mkv" | head -5 | while read f; do
  name=$(basename "$f")
  cp "$f" test_files_wikimedia/mkv/transcription/$name
done

# Verify
find test_files_wikimedia/mkv -name "*.mkv" | wc -l
# Should be: 6+ files (1 Kinetics + 5 Low-Res)
```

---

## IETF Source (Alternative if local fails)

```bash
git clone https://github.com/ietf-wg-cellar/matroska-test-files.git /tmp/mkv_tests
find /tmp/mkv_tests -name "test*.mkv" -exec cp {} test_files_wikimedia/mkv/keyframes/ \;
# Gets 8 official IETF test files
```

---

## USER REQUIREMENT

"I must have 5 for every format"

MKV must have MINIMUM 5 files.

Get them at N=351. They're on your computer.

---

## COPY THESE EXACT PATHS

```bash
# File 1 (Kinetics)
/Users/ayates/Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/Kinetics dataset (5%)/kinetics600_5per/kinetics600_5per/train/ice climbing/eb9jznQnhK8_raw.mkv

# Files 2-6 (Low-Res, pick 5)
/Users/ayates/Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/Low-Res Super Resolution Dataset video/converted/mkv/youku_00050_00099_l/Youku_00072_l.mkv
/Users/ayates/Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/Low-Res Super Resolution Dataset video/converted/mkv/youku_00050_00099_l/Youku_00069_l.mkv
/Users/ayates/Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/Low-Res Super Resolution Dataset video/converted/mkv/youku_00050_00099_l/Youku_00070_l.mkv
/Users/ayates/Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/Low-Res Super Resolution Dataset video/converted/mkv/youku_00050_00099_l/Youku_00089_l.mkv
/Users/ayates/Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/Low-Res Super Resolution Dataset video/converted/mkv/youku_00050_00099_l/Youku_00074_l.mkv
```

These files exist. Copy them NOW at N=351.
