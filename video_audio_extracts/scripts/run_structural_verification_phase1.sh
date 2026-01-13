#!/bin/bash
# Structural Verification of Phase 1 Tests (50 tests)
# Uses corrected test paths from PHASE_1_CORRECTED_PATHS.md
# Without Anthropic API Key - verifies execution and structure only

set -euo pipefail

# Environment setup (required for cargo to be in PATH)
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH"
export PKG_CONFIG_PATH="/opt/homebrew/lib/pkgconfig:/opt/homebrew/opt/ffmpeg/lib/pkgconfig"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Counters
TOTAL_TESTS=50
TESTS_PASS=0
TESTS_FAIL=0
START_TIME=$(date +%s)

# Report file
REPORT="docs/ai-verification/N116_STRUCTURAL_VERIFICATION_REPORT.md"
mkdir -p "$(dirname "$REPORT")"

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}Phase 1 Structural Verification${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""
echo "Total tests: $TOTAL_TESTS"
echo "Report: $REPORT"
echo ""

# Initialize report
cat > "$REPORT" << 'EOF'
# N=116 Structural Verification Report

**Date:** 2025-11-08
**Tests Verified:** 50 Phase 1 tests
**Verification Type:** Execution verification (via cargo test)
**API Key:** Not available (semantic verification blocked)

---

## Verification Methodology

This verification runs each test via `cargo test` to ensure:
1. Test executes without crashes
2. Test completes successfully (exit code 0)
3. No runtime errors occur

**What this verifies:**
- Binary is functional
- All 50 tests execute without crashing
- Basic correctness (tests have pass/fail assertions)

**What this doesn't verify:**
- Semantic correctness of outputs (requires AI vision verification)
- Quality of ML predictions
- Whether outputs match ground truth

---

## Test Results

EOF

# All 50 Phase 1 tests from PHASE_1_CORRECTED_PATHS.md
TESTS=(
    # Category 1: RAW formats (10)
    "smoke_format_arw_face_detection"
    "smoke_format_arw_object_detection"
    "smoke_format_cr2_face_detection"
    "smoke_format_cr2_object_detection"
    "smoke_format_dng_face_detection"
    "smoke_format_dng_ocr"
    "smoke_format_nef_face_detection"
    "smoke_format_nef_pose_estimation"
    "smoke_format_raf_face_detection"
    "smoke_format_raf_object_detection"

    # Category 2: New video formats (10)
    "smoke_format_mxf_face_detection"
    "smoke_format_mxf_object_detection"
    "smoke_format_vob_face_detection"
    "smoke_format_vob_emotion_detection"
    "smoke_format_asf_face_detection"
    "smoke_format_asf_emotion_detection"
    "smoke_format_alac_transcription"
    "smoke_format_alac_profanity_detection"
    "smoke_format_alac_audio_enhancement_metadata"
    "smoke_format_mkv_transcription"

    # Category 3: Audio operations (10)
    "smoke_format_mp3_profanity_detection"
    "smoke_format_mp3_audio_enhancement_metadata"
    "smoke_format_m4a_profanity_detection"
    "smoke_format_m4a_audio_enhancement_metadata"
    "smoke_format_ogg_profanity_detection"
    "smoke_format_ogg_audio_enhancement_metadata"
    "smoke_format_flac_profanity_detection"
    "smoke_format_flac_audio_enhancement_metadata"
    "smoke_format_wav_profanity_detection"
    "smoke_format_wav_audio_enhancement_metadata"

    # Category 4: Video operations (10)
    "smoke_format_mp4_emotion_detection"
    "smoke_format_mp4_action_recognition"
    "smoke_format_mov_emotion_detection"
    "smoke_format_mov_action_recognition"
    "smoke_format_webm_emotion_detection"
    "smoke_format_webm_action_recognition"
    "smoke_format_mkv_emotion_detection"
    "smoke_format_mkv_action_recognition"
    "smoke_format_avi_emotion_detection"
    "smoke_format_avi_action_recognition"

    # Category 5: Random sampling (10)
    "smoke_format_jpg_face_detection"
    "smoke_format_jpg_ocr"
    "smoke_format_png_object_detection"
    "smoke_format_png_ocr"
    "smoke_format_bmp_object_detection"
    "smoke_format_heic_face_detection"
    "smoke_format_webp_object_detection"
    "smoke_format_mp4_transcription"
    "smoke_format_webm_transcription"
    "smoke_format_flv_transcription"
)

# Run each test
for test_name in "${TESTS[@]}"; do
    TEST_NUM=$((TESTS_PASS + TESTS_FAIL + 1))
    echo -e "${BLUE}[$TEST_NUM/$TOTAL_TESTS]${NC} Running: $test_name"

    # Run the test (sequential mode required, with thread limiting)
    if VIDEO_EXTRACT_THREADS=4 cargo test --release --test smoke_test_comprehensive "$test_name" -- --ignored --test-threads=1 > /tmp/test_output_$$.txt 2>&1; then
        TESTS_PASS=$((TESTS_PASS + 1))
        echo -e "${GREEN}  ✓ PASS${NC}"

        # Append to report
        echo "### Test $TEST_NUM: $test_name" >> "$REPORT"
        echo "**Status:** ✅ PASS" >> "$REPORT"
        echo "" >> "$REPORT"
    else
        TESTS_FAIL=$((TESTS_FAIL + 1))
        echo -e "${RED}  ✗ FAIL${NC}"

        # Append to report with error details
        echo "### Test $TEST_NUM: $test_name" >> "$REPORT"
        echo "**Status:** ❌ FAIL" >> "$REPORT"
        echo "" >> "$REPORT"
        echo '```' >> "$REPORT"
        tail -20 /tmp/test_output_$$.txt >> "$REPORT"
        echo '```' >> "$REPORT"
        echo "" >> "$REPORT"
    fi

    # Clean up temp file
    rm -f /tmp/test_output_$$.txt
done

# Calculate summary
END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))
PASS_RATE=$(( (TESTS_PASS * 100) / TOTAL_TESTS ))

echo ""
echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}Summary${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""
echo -e "Total tests:   $TOTAL_TESTS"
echo -e "${GREEN}Passed:        $TESTS_PASS${NC}"
echo -e "${RED}Failed:        $TESTS_FAIL${NC}"
echo -e "Pass rate:     $PASS_RATE%"
echo -e "Duration:      ${DURATION}s"
echo ""

# Append summary to report
cat >> "$REPORT" << EOF

---

## Summary

- **Total tests:** $TOTAL_TESTS
- **Passed:** $TESTS_PASS (${PASS_RATE}%)
- **Failed:** $TESTS_FAIL
- **Duration:** ${DURATION}s
- **Date:** $(date)

---

## Interpretation

### What PASS Means

A test passing means:
- ✅ Test executed without crashing
- ✅ Binary is functional
- ✅ Basic test assertions passed

### What PASS Doesn't Mean

- ❓ Outputs are semantically correct (not verified)
- ❓ ML predictions are accurate (not verified)
- ❓ Results match ground truth (not verified)

**For semantic verification:** Set ANTHROPIC_API_KEY and run \`scripts/run_phase1_verification.sh\`

---

## Next Steps

### If All Tests Pass ($TESTS_PASS/$TOTAL_TESTS)

1. **System is stable** - All 50 tests execute without crashes
2. **Ready for semantic verification** when API key is available
3. **Can proceed** with Phase 2 sampling if needed

### If Some Tests Fail

1. **Investigate failures** - Check error messages above
2. **Fix bugs** if any are found
3. **Re-run verification** after fixes
4. **Do not proceed** to semantic verification until all tests pass

---

**End of N116_STRUCTURAL_VERIFICATION_REPORT.md**
EOF

echo "Report saved to: $REPORT"
echo ""

# Exit with appropriate code
if [ $TESTS_FAIL -eq 0 ]; then
    echo -e "${GREEN}✓ All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}✗ Some tests failed. See report for details.${NC}"
    exit 1
fi
