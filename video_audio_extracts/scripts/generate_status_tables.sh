#!/bin/bash
# Generate Status Tables from Test Results
#
# This script runs the format_conversion_suite tests and generates an updated
# FORMAT_CONVERSION_STATUS.md with actual pass/fail rates from test execution.
#
# Usage:
#   ./scripts/generate_status_tables.sh [--quick]
#
# Options:
#   --quick   Run only a sample of tests (10 tests) for quick verification
#   (no flag) Run all tests in the suite
#
# Requirements:
#   - video-extract binary built in release mode
#   - FFmpeg installed (for ffprobe validation)
#   - Test media files present in test directories
#
# Output:
#   - docs/FORMAT_CONVERSION_STATUS.md (updated with test results)
#   - Timestamp and test pass/fail counts embedded in document
#
# Note: This script runs cargo test with --ignored --test-threads=1 flags
# to ensure tests execute properly (format conversion tests are expensive).

set -e  # Exit on error

QUICK_MODE=false
if [[ "$1" == "--quick" ]]; then
    QUICK_MODE=true
    echo "Running in QUICK mode (sample tests only)"
fi

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Generating Format Conversion Status Table ===${NC}"
echo ""

# 0. Set required environment variables (per ENVIRONMENT_SETUP.md)
echo "Setting environment variables..."
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH"
export PKG_CONFIG_PATH="/opt/homebrew/lib/pkgconfig:/opt/homebrew/opt/ffmpeg/lib/pkgconfig"
echo -e "${GREEN}✓ Environment configured${NC}"
echo ""

# 1. Check prerequisites
echo "Checking prerequisites..."

if [[ ! -f "./target/release/video-extract" ]]; then
    echo -e "${RED}ERROR: video-extract binary not found at ./target/release/video-extract${NC}"
    echo "Please build in release mode first:"
    echo "  cargo build --release"
    exit 1
fi

if ! command -v ffprobe &> /dev/null; then
    echo -e "${RED}ERROR: ffprobe not found${NC}"
    echo "Please install FFmpeg first."
    exit 1
fi

echo -e "${GREEN}✓ Prerequisites OK${NC}"
echo ""

# 2. Set environment variables
export VIDEO_EXTRACT_THREADS=4
echo "Set VIDEO_EXTRACT_THREADS=4 (prevent system overload)"
echo ""

# 3. Run tests and capture output
TIMESTAMP=$(date '+%Y-%m-%d %H:%M:%S')
TEST_OUTPUT_FILE="/tmp/format_conversion_test_results_$$.txt"

echo -e "${BLUE}Running format conversion tests...${NC}"
echo "(This may take 5-15 minutes depending on system speed)"
echo ""

if [[ "$QUICK_MODE" == "true" ]]; then
    # Quick mode: run only a sample of tests
    # Run 10 specific tests (cargo test accepts test name as argument)
    echo "Quick mode: running 10 sample tests"
    echo ""

    # Note: Run tests one by one in quick mode to avoid pattern matching issues
    QUICK_TESTS=(
        "convert_avi"
        "convert_mp4"
        "convert_flv"
        "convert_wav"
        "convert_mp3"
        "convert_ogg"
        "convert_with_web_preset"
        "convert_with_mobile_preset"
        "convert_mov"
        "convert_flac"
    )

    # Run each test and accumulate output
    for test in "${QUICK_TESTS[@]}"; do
        echo "Running: $test"
        cargo test --release --test format_conversion_suite "$test" -- --ignored --test-threads=1 --nocapture 2>&1 | tee -a "$TEST_OUTPUT_FILE"
    done
    TEST_EXIT_CODE=${PIPESTATUS[0]}
else
    # Full mode: run all tests
    echo "Full mode: running all 41 tests"
    echo ""

    if cargo test --release --test format_conversion_suite -- --ignored --test-threads=1 2>&1 | tee "$TEST_OUTPUT_FILE"; then
        TEST_EXIT_CODE=0
    else
        TEST_EXIT_CODE=$?
    fi
fi

echo ""
echo -e "${BLUE}Test run completed (exit code: $TEST_EXIT_CODE)${NC}"
echo ""

# 4. Parse test results
echo "Parsing test results..."

# Extract all test summary lines and sum them up
# Each cargo test run outputs: "test result: ok. N passed; M failed; ..."
# We need to sum all the passed/failed counts

TESTS_PASSED=0
TESTS_FAILED=0

while IFS= read -r line; do
    # Match "test result: FAILED. 34 passed; 7 failed" or "test result: ok. 10 passed"
    # Use correct regex to capture passed count that comes AFTER other text
    if [[ "$line" =~ ([0-9]+)\ passed ]]; then
        PASSED_COUNT="${BASH_REMATCH[1]}"
        TESTS_PASSED=$((TESTS_PASSED + PASSED_COUNT))
    fi
    if [[ "$line" =~ ([0-9]+)\ failed ]]; then
        FAILED_COUNT="${BASH_REMATCH[1]}"
        TESTS_FAILED=$((TESTS_FAILED + FAILED_COUNT))
    fi
done < "$TEST_OUTPUT_FILE"

TESTS_TOTAL=$((TESTS_PASSED + TESTS_FAILED))

if [[ $TESTS_TOTAL -eq 0 ]]; then
    echo -e "${RED}ERROR: No tests were run${NC}"
    echo "Test output saved to: $TEST_OUTPUT_FILE"
    exit 1
fi

echo "Tests passed: $TESTS_PASSED"
echo "Tests failed: $TESTS_FAILED"
echo "Tests total: $TESTS_TOTAL"
echo ""

# Calculate pass rate
if [[ $TESTS_TOTAL -gt 0 ]]; then
    PASS_RATE=$(echo "scale=1; $TESTS_PASSED * 100 / $TESTS_TOTAL" | bc)
else
    PASS_RATE="0.0"
fi

# 5. Extract individual test results
echo "Extracting individual test results..."

# Create temporary file with test names and results
TEST_RESULTS_FILE="/tmp/format_conversion_individual_results_$$.txt"

# Parse test output for individual results
# Format: Single line per test
#   "test convert_avi ... ok" or "test convert_ac3 ... FAILED"
# We need to match test names and check status on the SAME line

# Extract test lines that start with "test convert_" or "test test_"
grep -E "^test (convert_|test_)" "$TEST_OUTPUT_FILE" > /tmp/test_lines_$$.txt || true

while IFS= read -r line_content; do
    # Extract test name (second field)
    TEST_NAME=$(echo "$line_content" | awk '{print $2}')

    # Check if line ends with "ok" or "FAILED"
    if echo "$line_content" | grep -q "\.\.\. ok$"; then
        echo "$TEST_NAME PASS"
    elif echo "$line_content" | grep -q "\.\.\. FAILED$"; then
        echo "$TEST_NAME FAIL"
    else
        # If we can't determine status, mark as UNKNOWN instead of assuming PASS
        echo "$TEST_NAME UNKNOWN"
    fi
done < /tmp/test_lines_$$.txt > "$TEST_RESULTS_FILE"

INDIVIDUAL_RESULTS_COUNT=$(wc -l < "$TEST_RESULTS_FILE" | tr -d ' ')
echo "Extracted $INDIVIDUAL_RESULTS_COUNT individual test results"
rm -f /tmp/test_lines_$$.txt
echo ""

# 6. Generate updated FORMAT_CONVERSION_STATUS.md
echo -e "${BLUE}Generating docs/FORMAT_CONVERSION_STATUS.md...${NC}"

OUTPUT_FILE="docs/FORMAT_CONVERSION_STATUS.md"

# Determine status emoji
if [[ $TESTS_FAILED -eq 0 ]]; then
    STATUS_EMOJI="✅"
    STATUS_TEXT="ALL PASS"
else
    STATUS_EMOJI="⚠️"
    STATUS_TEXT="SOME FAILURES"
fi

# Helper function to get test result
get_test_result() {
    local test_name="$1"
    if [[ -f "$TEST_RESULTS_FILE" ]]; then
        local result=$(grep "^${test_name} " "$TEST_RESULTS_FILE" | awk '{print $2}')
        if [[ "$result" == "PASS" ]]; then
            echo "✅"
        elif [[ "$result" == "FAIL" ]]; then
            echo "❌"
        elif [[ "$result" == "UNKNOWN" ]]; then
            echo "⚪"  # Status unknown
        else
            echo "⚪"  # Not tested (quick mode)
        fi
    else
        echo "⚪"
    fi
}

# Start generating the file
cat > "$OUTPUT_FILE" << 'EOF_HEADER'
# Format Conversion Test Status

**Auto-Generated:** This file is generated by scripts/generate_status_tables.sh
**Do not edit manually** - changes will be overwritten

EOF_HEADER

# Add metadata with actual values
cat >> "$OUTPUT_FILE" << EOF
**Last Test Run:** $TIMESTAMP
**Test Suite:** tests/format_conversion_suite.rs
**Tests Passed:** $TESTS_PASSED / $TESTS_TOTAL ($PASS_RATE%)
**Status:** $STATUS_EMOJI $STATUS_TEXT

EOF

# Add quick mode notice if applicable
if [[ "$QUICK_MODE" == "true" ]]; then
    cat >> "$OUTPUT_FILE" << 'EOF'
**Note:** This status table was generated in QUICK mode (10 sample tests).
For full test coverage, run: `./scripts/generate_status_tables.sh` (no --quick flag)

EOF
fi

cat >> "$OUTPUT_FILE" << 'EOF'
## Overview

This document tracks the official test status for format conversion functionality. All data is derived from actual test runs of `tests/format_conversion_suite.rs`.

**What "tested" means:**
- ✅ Test written in Rust test framework
- ✅ Test committed to git
- ✅ Test runs with `cargo test`
- ✅ Test asserts on correctness (ffprobe validation)
- ✅ Test is repeatable and documented

**Status Icons:**
- ✅ Test PASSED (last run)
- ❌ Test FAILED (last run)
- ⚪ Test NOT RUN (quick mode only)

## Test Suite Summary

| Metric | Value |
|--------|-------|
| **Total Tests** | 41 conversion tests |
| **Video Format Tests** | 17 tests |
| **Audio Format Tests** | 16 tests |
| **Preset Tests** | 7 tests + 2 additional |
| **Common Tests** | 1 test (duplicate MP4) |
| **Test Suite File** | tests/format_conversion_suite.rs |
| **Run Command** | `VIDEO_EXTRACT_THREADS=4 cargo test --release --test format_conversion_suite -- --ignored --test-threads=1` |

## Video Format Coverage

### Tested Video Formats (17 formats)

EOF

# Video format table with test results
cat >> "$OUTPUT_FILE" << EOF
| Format | Extension | Test Name | Status | Input File |
|--------|-----------|-----------|--------|------------|
| AVI | .avi | convert_avi | $(get_test_result "convert_avi") | test_edge_cases/format_test_avi.avi |
| MP4 | .mp4 | convert_mp4 | $(get_test_result "convert_mp4") | test_edge_cases/video_single_frame_only__minimal.mp4 |
| FLV | .flv | convert_flv | $(get_test_result "convert_flv") | test_edge_cases/format_test_flv.flv |
| 3GP | .3gp | convert_3gp | $(get_test_result "convert_3gp") | test_edge_cases/format_test_3gp.3gp |
| WMV | .wmv | convert_wmv | $(get_test_result "convert_wmv") | test_edge_cases/format_test_wmv.wmv |
| OGV | .ogv | convert_ogv | $(get_test_result "convert_ogv") | test_edge_cases/format_test_ogv.ogv |
| M4V | .m4v | convert_m4v | $(get_test_result "convert_m4v") | test_edge_cases/format_test_m4v.m4v |
| MOV | .mov | convert_mov | $(get_test_result "convert_mov") | test_edge_cases/video_no_audio_stream__error_test.mov |
| MKV | .mkv | convert_mkv | $(get_test_result "convert_mkv") | test_files_wikimedia/mkv/action-recognition/01_h264_from_mp4.mkv |
| WebM | .webm | convert_webm | $(get_test_result "convert_webm") | test_edge_cases/video_single_frame_only__minimal.webm |
| MXF | .mxf | convert_mxf | $(get_test_result "convert_mxf") | test_files_wikimedia/mxf/action-recognition/C0023S01.mxf |
| TS | .ts | convert_ts | $(get_test_result "convert_ts") | test_files_streaming_hls_dash/hls_01_basic/segment_000.ts |
| VOB | .vob | convert_vob | $(get_test_result "convert_vob") | test_files_wikimedia/vob/action-recognition/03_test.vob |
| RM | .rm | convert_rm | $(get_test_result "convert_rm") | test_files_wikimedia/rm/action-recognition/05_sample_1280x720.rm |
| ASF | .asf | convert_asf | $(get_test_result "convert_asf") | test_files_wikimedia/asf/action-recognition/02_elephant.asf |
| DV | .dv | convert_dv | $(get_test_result "convert_dv") | test_files_wikimedia/dv/action-recognition/01_shots0000.dv |
| F4V | .f4v | convert_f4v | $(get_test_result "convert_f4v") | test_files_video_formats_dpx_gxf_f4v/01_f4v_h264.f4v |

### Untested Video Formats (7 formats - rare/specialized)

| Format | Extension | Reason |
|--------|-----------|--------|
| MTS | .mts | Specialized camcorder format |
| M2TS | .m2ts | Blu-ray disc format |
| MPG | .mpg | Legacy MPEG-1/2 format |
| MPEG | .mpeg | Legacy MPEG-1/2 format |
| RMVB | .rmvb | RealMedia variable bitrate (rare) |
| GXF | .gxf | Professional broadcast format (rare) |
| DPX | .dpx | Digital picture exchange (image sequence) |

## Audio Format Coverage

### Tested Audio Formats (16 formats)

| Format | Extension | Test Name | Status | Input File |
|--------|-----------|-----------|--------|------------|
| WAV | .wav | convert_wav | $(get_test_result "convert_wav") | test_edge_cases/audio_mono_single_channel__channel_test.wav |
| MP3 | .mp3 | convert_mp3 | $(get_test_result "convert_mp3") | test_edge_cases/audio_lowquality_16kbps__compression_test.mp3 |
| OGG | .ogg | convert_ogg | $(get_test_result "convert_ogg") | test_edge_cases/format_test_ogg.ogg |
| Opus | .opus | convert_opus | $(get_test_result "convert_opus") | test_edge_cases/format_test_opus.opus |
| FLAC | .flac | convert_flac | $(get_test_result "convert_flac") | test_files_wikimedia/flac/audio-classification/04_Aina_zilizo_hatarini.flac.flac |
| M4A | .m4a | convert_m4a | $(get_test_result "convert_m4a") | test_files_wikimedia/alac/audio-classification/01_rodzaje_sygnalow.m4a |
| WMA | .wma | convert_wma | $(get_test_result "convert_wma") | test_files_wikimedia/wma/audio-classification/01_bangles.wma |
| AMR | .amr | convert_amr | $(get_test_result "convert_amr") | test_files_wikimedia/amr/audio-classification/01_sample.amr |
| AAC | .aac | convert_aac | $(get_test_result "convert_aac") | test_files_local/sample_10s_audio-aac.aac |
| AC3 | .ac3 | convert_ac3 | $(get_test_result "convert_ac3") | test_files_wikimedia/ac3/audio-classification/04_test.ac3 |
| DTS | .dts | convert_dts | $(get_test_result "convert_dts") | test_files_wikimedia/dts/audio-classification/03_test.dts |
| APE | .ape | convert_ape | $(get_test_result "convert_ape") | test_files_wikimedia/ape/audio-classification/01_concret_vbAccelerator.ape |
| TTA | .tta | convert_tta | $(get_test_result "convert_tta") | test_files_legacy_audio/tta/03_test.tta |
| WavPack | .wv | convert_wv | $(get_test_result "convert_wv") | test_files_wikimedia/wavpack/audio-classification/01_premsa_version.wv |
| Musepack | .mpc | convert_mpc | $(get_test_result "convert_mpc") | test_files_audio_formats_musepack/01_pumpkin.mpc |
| AU | .au | convert_au | $(get_test_result "convert_au") | test_files_legacy_audio/au/garelka.au |

### Untested Audio Formats

None. All common audio formats are covered.

## Conversion Preset Coverage

### Tested Presets (9 tests)

| Preset | Test Name | Status | Description | Input File |
|--------|-----------|--------|-------------|------------|
| web | convert_with_web_preset | $(get_test_result "convert_with_web_preset") | H.264/AAC, 1080p max, CRF 28 | test_edge_cases/video_4k_ultra_hd_3840x2160__stress_test.mp4 |
| mobile | convert_with_mobile_preset | $(get_test_result "convert_with_mobile_preset") | H.264/AAC, 720p max, CRF 32 | test_edge_cases/video_variable_framerate_vfr__timing_test.mp4 |
| mobile | convert_mp4_mobile_preset | $(get_test_result "convert_mp4_mobile_preset") | H.264/AAC, 720p max, CRF 32 | test_edge_cases/video_tiny_64x64_resolution__scaling_test.mp4 |
| archive | convert_with_archive_preset | $(get_test_result "convert_with_archive_preset") | H.265/AAC, CRF 20 | test_edge_cases/format_test_avi.avi |
| archive | convert_mp4_archive_preset | $(get_test_result "convert_mp4_archive_preset") | H.265/AAC, CRF 20 | test_edge_cases/video_variable_framerate_vfr__timing_test.mp4 |
| compatible | convert_with_compatible_preset | $(get_test_result "convert_with_compatible_preset") | H.264/AAC, CRF 18, near-lossless | test_edge_cases/format_test_avi.avi |
| lowbandwidth | convert_with_lowbandwidth_preset | $(get_test_result "convert_with_lowbandwidth_preset") | H.264/AAC, 480p max, CRF 35 | test_edge_cases/video_4k_ultra_hd_3840x2160__stress_test.mp4 |
| audioonly | convert_wav (and others) | $(get_test_result "convert_wav") | AAC only, no video | test_edge_cases/audio_mono_single_channel__channel_test.wav |
| copy | convert_with_copy_preset | $(get_test_result "convert_with_copy_preset") | Codec copy, remux only | test_edge_cases/format_test_avi.avi |

## Test Methodology

Each test in the suite:

1. **Runs video-extract** with specified preset and input file
2. **Verifies output JSON** contains valid paths, sizes, and compression ratios
3. **Validates with ffprobe** to ensure output file is valid and playable
4. **Checks metadata** such as compression ratio, input/output sizes
5. **Cleans up** temporary files after verification

**Validation Criteria:**
- Output JSON must be valid and parsable
- Output file must exist at specified path
- FFprobe must successfully read the output file
- FFprobe must return a valid format name (proves file is playable)

**Test Attributes:**
- All tests use \`#[ignore]\` attribute (must run explicitly with \`--ignored\`)
- Tests require FFmpeg installed on system
- Tests are slow (transcoding takes seconds to minutes)
- Tests should run on-demand, not in pre-commit hook

## Usage

### Regenerate This Status Table

\`\`\`bash
# Run all tests and regenerate (5-15 minutes)
./scripts/generate_status_tables.sh

# Quick verification (10 sample tests, ~2 minutes)
./scripts/generate_status_tables.sh --quick
\`\`\`

### Run All Format Conversion Tests Manually

\`\`\`bash
VIDEO_EXTRACT_THREADS=4 \\
cargo test --release --test format_conversion_suite -- --ignored --test-threads=1
\`\`\`

### Run Specific Test

\`\`\`bash
VIDEO_EXTRACT_THREADS=4 \\
cargo test --release --test format_conversion_suite convert_avi -- --ignored --test-threads=1
\`\`\`

### Run Only Video Format Tests

\`\`\`bash
VIDEO_EXTRACT_THREADS=4 \\
cargo test --release --test format_conversion_suite convert_ -- --ignored --test-threads=1
\`\`\`

### Run Only Preset Tests

\`\`\`bash
VIDEO_EXTRACT_THREADS=4 \\
cargo test --release --test format_conversion_suite with_ -- --ignored --test-threads=1
\`\`\`

## Coverage Statistics

### Overall Coverage

- **Total Supported Formats:** 40 formats (18 video + 22 audio)
- **Tested Formats:** 33 formats (17 video + 16 audio)
- **Coverage:** 82.5% (33/40)

### Video Format Coverage

- **Common Formats:** 17/17 tested (100%)
- **Rare Formats:** 0/7 tested (0%)
- **Total Video:** 17/24 tested (70.8%)

### Audio Format Coverage

- **All Formats:** 16/16 tested (100%)

### Preset Coverage

- **All Presets:** 7/7 tested (100%)
- **Total Preset Tests:** 9 tests (includes multiple tests per preset)

## Known Issues

### Preset Container Override Bug

**Problem:** When using presets with format-conversion, the CLI default container ("mp4") overrides the preset's container setting.

**Example:**
\`\`\`bash
# Expected: WebM output (webopen preset specifies WebM container)
# Actual: MP4 output (CLI default "mp4" overrides preset)
video-extract debug --ops "format-conversion:preset=webopen" input.avi
\`\`\`

**Root Cause:**
- CLI parsing (debug.rs:460) defaults \`container\` to "mp4" when not explicitly provided
- Plugin code (plugin.rs:157) then overrides the preset's container with this default

**Workaround:**
- Tests use presets that output MP4 anyway (web, mobile, archive, compatible)
- Tests verify ffprobe can read the output (format validity), not specific container type

**Fix Required:** Plugin should only override preset container if explicitly provided by user

## References

- **Test Suite:** tests/format_conversion_suite.rs (N=173-174)
- **Generation Script:** scripts/generate_status_tables.sh (N=176)
- **Plugin Implementation:** crates/format-conversion/
- **Plugin Config:** config/plugins/format_conversion.yaml
- **Manager Directive:** MANAGER_DIRECTIVE_BUILD_INFRA_FIRST.md

## Test Results History

EOF

# Add current test run to history
cat >> "$OUTPUT_FILE" << EOF
**$TIMESTAMP:** $TESTS_PASSED / $TESTS_TOTAL tests passed ($PASS_RATE%) $STATUS_EMOJI

EOF

# Add footer
cat >> "$OUTPUT_FILE" << 'EOF'

---

**This file is auto-generated.** To update, run:
\`\`\`bash
./scripts/generate_status_tables.sh
\`\`\`
EOF

echo -e "${GREEN}✓ Generated: $OUTPUT_FILE${NC}"
echo ""

# 7. Clean up
rm -f "$TEST_OUTPUT_FILE" "$TEST_RESULTS_FILE"

# 8. Summary
echo -e "${BLUE}=== Summary ===${NC}"
echo "Status table generated: $OUTPUT_FILE"
echo "Tests passed: $TESTS_PASSED / $TESTS_TOTAL ($PASS_RATE%)"
echo ""

if [[ $TESTS_FAILED -eq 0 ]]; then
    echo -e "${GREEN}✓ All tests PASSED${NC}"
    exit 0
else
    echo -e "${YELLOW}⚠ Some tests FAILED${NC}"
    echo "Review test output above for details."
    exit 1
fi
