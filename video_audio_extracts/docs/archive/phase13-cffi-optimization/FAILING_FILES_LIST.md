# Complete List of Failing Test Files

**Date**: 2025-10-30
**Purpose**: Document all test files causing failures for fix
**Tests affected**: 8 tests (6.1% of suite)

---

## THE 5 FILES

### 1. fabula_01_018_esopo_64kb.mp3 (1.1M)
```
Path: ~/Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/public/librivox/fabula_01_018_esopo_64kb.mp3
Status: EXISTS but Dropbox on-demand sync
Tests affected:
  - format_mp3_audiobook (line 166)
  - characteristic_audio_codec_mp3 (line 901)
  - characteristic_audio_size_small_5mb (line 973)
Issue: ffprobe times out after 10s waiting for Dropbox download
```

### 2. fabula_01_024_esopo_64kb.mp3 (375K)
```
Path: ~/Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/public/librivox/fabula_01_024_esopo_64kb.mp3
Status: EXISTS but Dropbox on-demand sync
Tests affected:
  - random_sample_mp3_librivox_batch (line 1797)
Issue: Same as #1
```

### 3. JKAWup5iKho_raw.f251.webm (2.2M)
```
Path: ~/Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/Kinetics dataset (5%)/kinetics600_5per/kinetics600_5per/train/zumba/JKAWup5iKho_raw.f251.webm
Status: EXISTS but Dropbox on-demand sync
Tests affected:
  - format_webm_kinetics (line 117)
  - random_sample_webm_audio_only (line 1757)
Issue: Same as #1
```

### 4. State of Affairs_ROUGHMIX.wav (56M)
```
Path: ~/Music/Music/Media.localized/Music/Unknown Artist/Unknown Album/State of Affairs_ROUGHMIX.wav
Status: EXISTS (local file)
Tests affected:
  - additional_audio_embeddings_music (line 2475)
Issue: May also be in iCloud sync, need to verify accessibility
```

### 5. sample_10s_audio-aac.aac (146K)
```
Path: ~/docling/tests/data/audio/sample_10s_audio-aac.aac
Status: EXISTS (local file)
Tests affected:
  - additional_audio_embeddings_speech (line 2496)
Issue: Need to verify accessibility
```

---

## FIX STRATEGY

### Option A: Force Dropbox Download (Immediate)
```bash
# Trigger download by copying to local
mkdir -p test_files_local/
cp ~/Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/public/librivox/fabula_01_018_esopo_64kb.mp3 test_files_local/
cp ~/Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/public/librivox/fabula_01_024_esopo_64kb.mp3 test_files_local/
cp ~/Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/Kinetics\ dataset\ \(5%\)/kinetics600_5per/kinetics600_5per/train/zumba/JKAWup5iKho_raw.f251.webm test_files_local/
cp ~/Music/Music/Media.localized/Music/Unknown\ Artist/Unknown\ Album/State\ of\ Affairs_ROUGHMIX.wav test_files_local/
cp ~/docling/tests/data/audio/sample_10s_audio-aac.aac test_files_local/
```

### Option B: Update Test Paths
```rust
// tests/standard_test_suite.rs
// Change all paths from CloudStorage to test_files_local/
let file = PathBuf::from("test_files_local/fabula_01_018_esopo_64kb.mp3");
```

### Option C: Use test_edge_cases Files Instead
```rust
// Replace with existing known-good files
let file = PathBuf::from("test_edge_cases/audio_lowquality_16kbps__compression_test.mp3");
```

---

## RECOMMENDED FIX (Option A + B)

1. Copy files to local directory
2. Update test paths to use local copies
3. Verify all tests pass

**Estimated time**: 30 minutes
**Expected result**: 98/98 tests passing (100%)
