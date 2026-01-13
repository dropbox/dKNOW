# Format Conversion Test Suite

**Created:** N=173 (2025-11-10)
**Updated:** N=176 (automation script added)
**Location:** `tests/format_conversion_suite.rs`
**Status Table:** `docs/FORMAT_CONVERSION_STATUS.md` (auto-generated)
**Generation Script:** `scripts/generate_status_tables.sh`
**Purpose:** Automated testing of format conversion plugin for all supported input formats

## Overview

The format conversion test suite verifies that the `format_conversion` plugin correctly handles all supported input formats (video + audio). This is part of the infrastructure plan from MANAGER_DIRECTIVE_BUILD_INFRA_FIRST.md to create automated tests instead of manual verification.

## Test Coverage

**Total Tests:** 41 automated tests (N=174)

### Video Format Tests (19 tests)
- AVI → MP4 (web preset)
- MP4 → MP4 (web preset, multiple tests)
- FLV → MP4
- 3GP → MP4
- WMV → MP4
- OGV → MP4
- M4V → MP4
- MOV → MP4
- MKV → MP4
- WebM → MP4
- MXF → MP4
- TS → MP4
- VOB → MP4
- RM → MP4
- ASF → MP4
- DV → MP4
- F4V → MP4

### Audio Format Tests (16 tests)
- WAV → MP4 (audioonly preset)
- MP3 → MP4 (audioonly preset)
- OGG → MP4 (audioonly preset)
- Opus → MP4 (audioonly preset)
- FLAC → MP4 (audioonly preset)
- M4A → MP4 (audioonly preset)
- WMA → MP4 (audioonly preset)
- AMR → MP4 (audioonly preset)
- AAC → MP4 (audioonly preset)
- AC3 → MP4 (audioonly preset)
- DTS → MP4 (audioonly preset)
- APE → MP4 (audioonly preset)
- TTA → MP4 (audioonly preset)
- WV (WavPack) → MP4 (audioonly preset)
- MPC (Musepack) → MP4 (audioonly preset)
- AU → MP4 (audioonly preset)

### Preset Tests (7 tests)
- Web preset (H.264/AAC, 1080p max, CRF 28)
- Mobile preset (H.264/AAC, 720p max, CRF 32)
- Archive preset (H.265/AAC, CRF 20)
- Compatible preset (H.264/AAC, CRF 18, near-lossless)
- Low bandwidth preset (H.264/AAC, 480p max, CRF 35)
- Audio-only preset (AAC only, no video)
- Copy preset (codec copy, remux only)

## Usage

```bash
# Run all format conversion tests
VIDEO_EXTRACT_THREADS=4 \
cargo test --release --test format_conversion_suite -- --ignored --test-threads=1

# Run specific test
VIDEO_EXTRACT_THREADS=4 \
cargo test --release --test format_conversion_suite convert_avi -- --ignored --test-threads=1
```

## Test Methodology

Each test:
1. Runs video-extract with specified preset
2. Verifies output JSON contains valid paths and metadata
3. Uses ffprobe to verify output file is valid and playable
4. Checks compression ratios and file sizes
5. Cleans up temporary files

Tests use `#[ignore]` attribute because:
- Format conversion is slow (transcoding can take seconds to minutes)
- Tests require FFmpeg installed
- Should be run on-demand, not in pre-commit hook

## Automated Status Table Generation (N=176)

The script `scripts/generate_status_tables.sh` automatically runs tests and generates the status table:

```bash
# Run all tests and generate full status table (5-15 minutes)
./scripts/generate_status_tables.sh

# Quick verification with sample tests (~2 minutes)
./scripts/generate_status_tables.sh --quick
```

**Output:** `docs/FORMAT_CONVERSION_STATUS.md` (auto-generated with actual test results)

**Features:**
- Runs tests with proper environment setup
- Parses pass/fail results from cargo test output
- Updates status table with ✅ (pass), ❌ (fail), or ⚪ (not run)
- Records test run timestamp and pass rates
- Maintains test results history

## Benefits

- ✅ **Automated** - No manual steps required
- ✅ **Integrated** - Part of cargo test framework
- ✅ **CI/CD ready** - Can run in GitHub Actions
- ✅ **Reproducible** - Clear pass/fail criteria
- ✅ **Coverage tracking** - Know which formats are tested
- ✅ **Auto-generated docs** - Status table always reflects actual test results

## Known Issues

### Preset Container Override Bug

**Problem:** When using presets with format-conversion, the CLI default container ("mp4") overrides the preset's container setting.

**Example:**
```bash
# Expected: WebM output (webopen preset specifies WebM container)
# Actual: MP4 output (CLI default "mp4" overrides preset)
video-extract debug --ops "format-conversion:preset=webopen" input.avi
```

**Root Cause:**
- CLI parsing (debug.rs:460) defaults `container` to "mp4" when not explicitly provided
- Plugin code (plugin.rs:157) then overrides the preset's container with this default

**Workaround:**
- Tests use presets that output MP4 anyway (web, mobile, archive, compatible)
- Tests verify ffprobe can read the output (format validity), not specific container type

**Fix Required:** Plugin should only override preset container if explicitly provided by user

## Format Coverage

### Tested Input Formats (N=174)
- Video: MP4, AVI, FLV, 3GP, WMV, OGV, M4V, MOV, MKV, WebM, MXF, TS, VOB, RM, ASF, DV, F4V (17 formats)
- Audio: WAV, MP3, OGG, Opus, FLAC, M4A, WMA, AMR, AAC, AC3, DTS, APE, TTA, WV, MPC, AU (16 formats)
- **Total: 33 formats tested**

### Untested Input Formats (Remaining Work)
- Video: MTS, M2TS, MPG, MPEG, RMVB, GXF, DPX (7 formats - rare/specialized)
- Audio: None (all common audio formats covered)

## Next Steps (N=175-176)

Per MANAGER_DIRECTIVE_BUILD_INFRA_FIRST.md:

**N=174:** ✅ Complete
- Added 22 more format conversion tests (42 total)
- Tested all common video formats (MOV, MKV, WebM, MXF, TS, VOB, RM, ASF, DV, F4V)
- Tested all common audio formats (FLAC, M4A, WMA, AMR, AAC, AC3, DTS, APE, TTA, WV, MPC, AU)
- Verified sample tests pass (MOV, MKV, FLAC tested successfully)

**N=175:** Next
- Create docs/FORMAT_CONVERSION_STATUS.md (official status table)
- Generate from test results
- Include test names, coverage percentages, pass rates

**N=176:** After N=175
- Create script to auto-generate status tables from test results
- Script: scripts/generate_status_tables.sh
- Updates FORMAT_CONVERSION_STATUS.md automatically

## References

- Implementation: tests/format_conversion_suite.rs (N=173-174)
- Plugin: crates/format-conversion/ (N=1-5)
- Config: config/plugins/format_conversion.yaml
- Manual tests: docs/FORMAT_CONVERSION_MATRIX.md (N=1-5)
- Manager directive: MANAGER_DIRECTIVE_BUILD_INFRA_FIRST.md

## Changelog

**N=174 (2025-11-10):**
- Added 21 new format conversion tests (20→41 tests)
- Added 10 video format tests: MOV, MKV, WebM, MXF, TS, VOB, RM, ASF, DV, F4V
- Added 12 audio format tests: FLAC, M4A, WMA, AMR, AAC, AC3, DTS, APE, TTA, WV, MPC, AU
- Verified sample tests pass (MOV, MKV, FLAC)
- Coverage: 33 formats tested (all common formats)

**N=173 (2025-11-10):**
- Initial test suite created (20 tests)
- Video formats: AVI, MP4, FLV, 3GP, WMV, OGV, M4V
- Audio formats: WAV, MP3, OGG, Opus
- Preset tests: web, mobile, archive, compatible, lowbandwidth, copy
