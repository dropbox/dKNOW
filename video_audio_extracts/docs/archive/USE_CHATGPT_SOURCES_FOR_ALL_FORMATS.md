# [MANAGER] Use ChatGPT Sample File Sources - Expand All Formats

USER PROVIDED: Comprehensive list of sample file sources from ChatGPT

---

## DIRECT DOWNLOAD LINKS (16 formats)

### Audio Formats (7)

**APE (.ape)**:
- Source: https://filesamples.com/formats/ape
- Download multiple APE samples

**TTA (.tta)**:
- Source: https://filesamples.com/formats/tta
- Download multiple TTA samples

**AC3 (.ac3)**:
- Source: https://filesamples.com/categories/audio
- AC3 audio files available

**ALAC (.alac)**:
- Source: https://getsamplefiles.com/sample-audio-files/alac
- Sample ALAC audio files

**DTS, AMR, WavPack (.wv), WMA (.wma)**:
- Search: "sample .dts file download", "sample .amr file download", etc.
- Or generate from WAV using FFmpeg

### Video Formats (4)

**ASF (.asf)**:
- Source: https://filesamples.com/formats/asf
- Sample ASF video files

**MXF (.mxf)**:
- Source: https://toolsfairy.com/video-test/sample-mxf-files
- Professional MXF samples for testing

**VOB (.vob)**:
- Source: https://wangchujiang.com/filesamples/video.html
- Sample VOB files (sample_640x360.vob)

**RM (.rm)**:
- Source: https://wangchujiang.com/filesamples/video.html
- RealMedia sample files

**DV (.dv)**:
- Generate or search for DV samples

### Image/Other (5)

**AVIF (.avif)**:
- Generate from JPEG using avifenc
- Or search online samples

**HLS (.m3u8)**:
- Generate HLS playlists (streaming format)
- Need to create manifest + segments

---

## GENERAL REPOSITORIES

**filesamples.com**:
- 814 sample files in 180 formats
- https://filesamples.com/

**Sample-Files.com**:
- Audio files in all major formats
- https://sample-files.com/audio/

**File-Examples.com**:
- Most popular file formats
- https://file-examples.com/

---

## EXECUTION PLAN (N=387-390)

### N=387: Download from filesamples.com
```bash
# APE (need 4 more)
wget https://filesamples.com/samples/audio/ape/sample1.ape -O test_files_wikimedia/ape/transcription/02_sample.ape
# (download 4 files)

# TTA (need 3 more)
wget https://filesamples.com/samples/audio/tta/sample1.tta -O test_files_wikimedia/tta/transcription/03_sample.tta
# (download 3 files)

# ASF (need 2 more)
wget https://filesamples.com/samples/video/asf/sample1.asf -O test_files_wikimedia/asf/keyframes/04_sample.asf
# (download 2 files)
```

### N=388: Download from getsamplefiles.com + toolsfairy.com
```bash
# ALAC (need 2 more)
wget https://getsamplefiles.com/download/alac/sample1.m4a -O test_files_wikimedia/alac/transcription/04_sample.m4a

# MXF (need 1 more)
wget https://toolsfairy.com/samples/mxf/sample1.mxf -O test_files_wikimedia/mxf/keyframes/05_sample.mxf
```

### N=389: Download from wangchujiang.com
```bash
# VOB (need 2 more)
wget https://wangchujiang.com/filesamples/video/sample_640x360.vob -O test_files_wikimedia/vob/keyframes/04_sample.vob

# RM (need 1 more)
wget https://wangchujiang.com/filesamples/video/sample.rm -O test_files_wikimedia/rm/keyframes/05_sample.rm
```

### N=390: Generate remaining formats
```bash
# WMA, DTS, AMR, WavPack, DV, AVIF (generate from existing files)
# HLS (generate more manifests)
# MKV, SVG (copy 1 more each)
```

---

## PDF CLEANUP

**Current**: 3,314 PDF files (too many!)

**Action**: Keep only 5-10 representative PDFs:
```bash
# Keep first 10 PDFs, remove rest
cd test_files_wikimedia/pdf/ocr
ls *.pdf | head -10 > keep.txt
ls *.pdf | tail -n +11 | xargs rm
```

---

## TARGET

After N=387-390:
- All 38 formats at 5+ files ✅
- PDF trimmed to 10 files ✅
- Complete format coverage ✅

User provided excellent sources - use them!
