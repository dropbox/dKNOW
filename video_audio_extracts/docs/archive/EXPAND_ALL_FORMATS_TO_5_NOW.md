# [MANAGER] EXPAND ALL FORMATS TO 5 FILES NOW

USER: "Expand those 17 formats!"

Current: 20/37 formats at 5+ files
Need: Expand 17 formats to 5 minimum (31 files needed)

---

## FORMATS TO EXPAND (Priority Order)

**Need 4 more**:
- APE: 1→5 (copy from FFmpeg samples or generate)

**Need 3 more**:
- TTA: 2→5
- HLS: 1→5 (generate more HLS manifests)

**Need 2 more** (12 formats):
- AC3, ALAC, AMR, ASF, AVIF, DTS, DV, VOB, WavPack, WMA (10)

**Need 1 more** (4 formats):
- MKV, MXF, RM, SVG

---

## EXECUTE AT N=374

**Quick wins** (already have sources):

```bash
# MKV (need 1 more) - Use Dropbox Low-Res
find ~/Library/CloudStorage/Dropbox*/a.test/Low-Res*/converted/mkv/*/*.mkv | head -1 | xargs -I {} cp {} test_files_wikimedia/mkv/transcription/

# SVG (need 1 more) - Download from W3C
wget https://dev.w3.org/SVG/tools/svgweb/samples/svg-files/lion.svg -O test_files_wikimedia/svg/ocr/05_lion.svg

# MXF, RM (need 1 more each) - Copy from FFmpeg samples or generate

# APE, TTA, etc. - Generate from existing audio files using ffmpeg
```

**Generate missing audio formats**:
```bash
# APE (need 4 more)
for i in {2..5}; do
  ffmpeg -i test_files_wikimedia/wav/transcription/0${i}*.wav -c:a ape test_files_wikimedia/ape/transcription/0${i}_test.ape
done

# Similar for TTA, ALAC, WavPack, etc.
```

---

## COMMIT AS YOU GO

After each expansion, commit:
```
# 374: Expand MKV, SVG, MXF to 5 Files (3 files added)
# 375: Expand APE, TTA, HLS to 5 Files (11 files added)
# 376: Expand AC3, DTS, ALAC to 5 Files (6 files added)
# 377: Expand Remaining 9 Formats to 5 Files (11 files added)
```

Total: 31 files to reach 5 minimum for all 37 formats.
