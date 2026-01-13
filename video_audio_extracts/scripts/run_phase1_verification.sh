#!/bin/bash
#
# Phase 1 AI Verification Execution Script
#
# Automates the verification of 50 Phase 1 tests
#
# Usage:
#   export ANTHROPIC_API_KEY="sk-ant-..."
#   bash scripts/run_phase1_verification.sh
#

set -e

# Check API key
if [ -z "$ANTHROPIC_API_KEY" ]; then
    echo "ERROR: ANTHROPIC_API_KEY environment variable not set"
    echo "Please run: export ANTHROPIC_API_KEY=\"sk-ant-...\""
    exit 1
fi

# Check binary exists
if [ ! -f "target/release/video-extract" ]; then
    echo "ERROR: Release binary not found"
    echo "Please run: cargo build --release"
    exit 1
fi

# Create report file
REPORT_FILE="docs/ai-verification/NEW_TESTS_AI_VERIFICATION_REPORT.md"
mkdir -p "$(dirname "$REPORT_FILE")"

# Initialize report
cat > "$REPORT_FILE" <<'EOF'
# AI Verification Report - New Tests (N=93-109)

**Date:** $(date +%Y-%m-%d)
**Tests Verified:** 0/50 (Phase 1)
**Verifier:** Claude Sonnet 4
**Status:** IN PROGRESS

---

## Summary

- Total verified: 0 tests
- CORRECT: 0 tests (0%)
- SUSPICIOUS: 0 tests (0%)
- INCORRECT: 0 tests (0%)
- Average confidence: N/A

### Confidence Distribution
- ≥0.90: 0 tests (0%)
- 0.70-0.89: 0 tests (0%)
- 0.50-0.69: 0 tests (0%)
- <0.50: 0 tests (0%)

---

## Detailed Results

EOF

echo "Phase 1 AI Verification Execution"
echo "=================================="
echo ""
echo "Report file: $REPORT_FILE"
echo ""

# Test counter
TOTAL=0
CORRECT=0
SUSPICIOUS=0
INCORRECT=0

# Function to verify a single test
verify_test() {
    local test_num=$1
    local test_name=$2
    local file_path=$3
    local operations=$4

    echo ""
    echo "[$test_num/50] Verifying: $test_name"
    echo "  File: $file_path"
    echo "  Operations: $operations"

    # Check if file exists
    if [ ! -f "$file_path" ]; then
        echo "  ERROR: File not found: $file_path"
        echo "  SKIPPED"
        return
    fi

    # Generate output
    echo "  Running video-extract..."
    ./target/release/video-extract debug --ops "$operations" "$file_path" 2>&1 | grep -v "^$" || true

    # Find the last operation in the pipeline
    last_op=$(echo "$operations" | rev | cut -d';' -f1 | rev)

    # Find the output file
    output_file=$(ls -t debug_output/stage_*_${last_op}.json 2>/dev/null | head -1)

    if [ -z "$output_file" ] || [ ! -f "$output_file" ]; then
        echo "  ERROR: Output file not found for operation: $last_op"
        echo "  SKIPPED"
        return
    fi

    echo "  Output: $output_file"

    # AI verification
    echo "  Running AI verification..."
    verification_result=$(python scripts/ai_verify_outputs.py "$file_path" "$output_file" "$last_op" 2>&1)

    # Parse result (extract JSON from Claude's response)
    status=$(echo "$verification_result" | grep -o '"status":[[:space:]]*"[^"]*"' | cut -d'"' -f4 || echo "UNKNOWN")
    confidence=$(echo "$verification_result" | grep -o '"confidence":[[:space:]]*[0-9.]*' | awk '{print $2}' || echo "0.0")

    echo "  Status: $status"
    echo "  Confidence: $confidence"

    # Update counters
    TOTAL=$((TOTAL + 1))
    case "$status" in
        CORRECT)
            CORRECT=$((CORRECT + 1))
            ;;
        SUSPICIOUS)
            SUSPICIOUS=$((SUSPICIOUS + 1))
            ;;
        INCORRECT)
            INCORRECT=$((INCORRECT + 1))
            ;;
    esac

    # Append to report
    cat >> "$REPORT_FILE" <<EOF

### Test $test_num: $test_name
- **Input:** $file_path
- **Operations:** $operations
- **Output:** $output_file
- **Status:** $status
- **Confidence:** $confidence
- **Verification Result:**
\`\`\`
$verification_result
\`\`\`

EOF

    echo "  Documented in report"
}

# Category 1: RAW Format Tests (10 tests)
echo ""
echo "Category 1: RAW Format Tests"
echo "=============================="

verify_test 1 "ARW + face-detection" \
    "test_files_camera_raw_samples/arw/sample.arw" \
    "face-detection"

verify_test 2 "ARW + object-detection" \
    "test_files_camera_raw_samples/arw/sample.arw" \
    "object-detection"

verify_test 3 "CR2 + face-detection" \
    "test_files_camera_raw_samples/cr2/sample.cr2" \
    "face-detection"

verify_test 4 "CR2 + object-detection" \
    "test_files_camera_raw_samples/cr2/sample.cr2" \
    "object-detection"

verify_test 5 "DNG + face-detection" \
    "test_files_camera_raw_samples/dng/sample.dng" \
    "face-detection"

verify_test 6 "DNG + ocr" \
    "test_files_camera_raw_samples/dng/sample.dng" \
    "ocr"

verify_test 7 "NEF + face-detection" \
    "test_files_camera_raw_samples/nef/sample.nef" \
    "face-detection"

verify_test 8 "NEF + pose-estimation" \
    "test_files_camera_raw_samples/nef/sample.nef" \
    "pose-estimation"

verify_test 9 "RAF + face-detection" \
    "test_files_camera_raw_samples/raf/sample.raf" \
    "face-detection"

verify_test 10 "RAF + object-detection" \
    "test_files_camera_raw_samples/raf/sample.raf" \
    "object-detection"

# Category 2: New Video Formats (10 tests)
echo ""
echo "Category 2: New Video Formats"
echo "=============================="

verify_test 11 "MXF + face-detection" \
    "test_files_wikimedia/mxf/keyframes/C0023S01.mxf" \
    "keyframes;face-detection"

verify_test 12 "MXF + object-detection" \
    "test_files_wikimedia/mxf/keyframes/C0023S01.mxf" \
    "keyframes;object-detection"

verify_test 13 "MXF + ocr" \
    "test_files_wikimedia/mxf/keyframes/C0023S01.mxf" \
    "keyframes;ocr"

verify_test 14 "VOB + face-detection" \
    "test_files_wikimedia/vob/Carrie Fisher Tribute at the Saturn Awards.vob" \
    "keyframes;face-detection"

verify_test 15 "VOB + object-detection" \
    "test_files_wikimedia/vob/Carrie Fisher Tribute at the Saturn Awards.vob" \
    "keyframes;object-detection"

verify_test 16 "VOB + scene-detection" \
    "test_files_wikimedia/vob/Carrie Fisher Tribute at the Saturn Awards.vob" \
    "keyframes;scene-detection"

verify_test 17 "ASF + face-detection" \
    "test_files_wikimedia/asf/Carrie Fisher Tribute at the Saturn Awards.asf" \
    "keyframes;face-detection"

verify_test 18 "ASF + object-detection" \
    "test_files_wikimedia/asf/Carrie Fisher Tribute at the Saturn Awards.asf" \
    "keyframes;object-detection"

verify_test 19 "ASF + ocr" \
    "test_files_wikimedia/asf/Carrie Fisher Tribute at the Saturn Awards.asf" \
    "keyframes;ocr"

verify_test 20 "ASF + scene-detection" \
    "test_files_wikimedia/asf/Carrie Fisher Tribute at the Saturn Awards.asf" \
    "keyframes;scene-detection"

# Category 3: Audio Advanced Operations (10 tests)
echo ""
echo "Category 3: Audio Advanced Operations"
echo "======================================"

verify_test 21 "MP4 + profanity-detection" \
    "test_edge_cases/video_test_av1.mp4" \
    "profanity-detection"

verify_test 22 "MKV + profanity-detection" \
    "test_edge_cases/video_test_vp9.mkv" \
    "profanity-detection"

verify_test 23 "MXF + profanity-detection" \
    "test_files_wikimedia/mxf/audio_extraction/C0023S01.mxf" \
    "profanity-detection"

verify_test 24 "FLAC + profanity-detection" \
    "test_files_audio/flac/sample.flac" \
    "profanity-detection"

verify_test 25 "ALAC + profanity-detection" \
    "test_files_audio/alac/sample.m4a" \
    "profanity-detection"

verify_test 26 "MP4 + audio-enhancement-metadata" \
    "test_edge_cases/video_test_av1.mp4" \
    "audio-enhancement-metadata"

verify_test 27 "MKV + audio-enhancement-metadata" \
    "test_edge_cases/video_test_vp9.mkv" \
    "audio-enhancement-metadata"

verify_test 28 "MXF + audio-enhancement-metadata" \
    "test_files_wikimedia/mxf/audio_extraction/C0023S01.mxf" \
    "audio-enhancement-metadata"

verify_test 29 "ALAC + audio-enhancement-metadata" \
    "test_files_audio/alac/sample.m4a" \
    "audio-enhancement-metadata"

verify_test 30 "WAV + audio-enhancement-metadata" \
    "test_files_audio/wav/sample.wav" \
    "audio-enhancement-metadata"

# Category 4: Video Advanced Operations (10 tests)
echo ""
echo "Category 4: Video Advanced Operations"
echo "======================================"

verify_test 31 "MP4 + action-recognition" \
    "test_edge_cases/video_test_av1.mp4" \
    "keyframes;action-recognition"

verify_test 32 "MKV + action-recognition" \
    "test_edge_cases/video_test_vp9.mkv" \
    "keyframes;action-recognition"

verify_test 33 "MXF + action-recognition" \
    "test_files_wikimedia/mxf/keyframes/C0023S01.mxf" \
    "keyframes;action-recognition"

verify_test 34 "VOB + action-recognition" \
    "test_files_wikimedia/vob/Carrie Fisher Tribute at the Saturn Awards.vob" \
    "keyframes;action-recognition"

verify_test 35 "ASF + action-recognition" \
    "test_files_wikimedia/asf/Carrie Fisher Tribute at the Saturn Awards.asf" \
    "keyframes;action-recognition"

verify_test 36 "MP4 + emotion-detection" \
    "test_edge_cases/video_test_av1.mp4" \
    "keyframes;emotion-detection"

verify_test 37 "MKV + emotion-detection" \
    "test_edge_cases/video_test_vp9.mkv" \
    "keyframes;emotion-detection"

verify_test 38 "MXF + emotion-detection" \
    "test_files_wikimedia/mxf/keyframes/C0023S01.mxf" \
    "keyframes;emotion-detection"

verify_test 39 "VOB + emotion-detection" \
    "test_files_wikimedia/vob/Carrie Fisher Tribute at the Saturn Awards.vob" \
    "keyframes;emotion-detection"

verify_test 40 "ASF + emotion-detection" \
    "test_files_wikimedia/asf/Carrie Fisher Tribute at the Saturn Awards.asf" \
    "keyframes;emotion-detection"

# Category 5: Random Sampling (10 tests)
echo ""
echo "Category 5: Random Sampling"
echo "==========================="

verify_test 41 "ARW + vision-embeddings" \
    "test_files_camera_raw_samples/arw/sample.arw" \
    "vision-embeddings"

verify_test 42 "DNG + image-quality-assessment" \
    "test_files_camera_raw_samples/dng/sample.dng" \
    "image-quality-assessment"

verify_test 43 "MXF + pose-estimation" \
    "test_files_wikimedia/mxf/keyframes/C0023S01.mxf" \
    "keyframes;pose-estimation"

verify_test 44 "VOB + vision-embeddings" \
    "test_files_wikimedia/vob/Carrie Fisher Tribute at the Saturn Awards.vob" \
    "keyframes;vision-embeddings"

verify_test 45 "ASF + vision-embeddings" \
    "test_files_wikimedia/asf/Carrie Fisher Tribute at the Saturn Awards.asf" \
    "keyframes;vision-embeddings"

verify_test 46 "ALAC + audio-embeddings" \
    "test_files_audio/alac/sample.m4a" \
    "audio-embeddings"

verify_test 47 "ALAC + diarization" \
    "test_files_audio/alac/sample.m4a" \
    "diarization"

verify_test 48 "MXF + smart-thumbnail" \
    "test_files_wikimedia/mxf/keyframes/C0023S01.mxf" \
    "keyframes;smart-thumbnail"

verify_test 49 "VOB + shot-classification" \
    "test_files_wikimedia/vob/Carrie Fisher Tribute at the Saturn Awards.vob" \
    "keyframes;shot-classification"

verify_test 50 "ASF + shot-classification" \
    "test_files_wikimedia/asf/Carrie Fisher Tribute at the Saturn Awards.asf" \
    "keyframes;shot-classification"

# Final summary
echo ""
echo "=================================="
echo "Phase 1 Verification Complete"
echo "=================================="
echo ""
echo "Total verified: $TOTAL/50"
echo "CORRECT: $CORRECT"
echo "SUSPICIOUS: $SUSPICIOUS"
echo "INCORRECT: $INCORRECT"
echo ""
echo "Report: $REPORT_FILE"
echo ""

# Update summary in report
CORRECT_PCT=$((CORRECT * 100 / TOTAL))
SUSPICIOUS_PCT=$((SUSPICIOUS * 100 / TOTAL))
INCORRECT_PCT=$((INCORRECT * 100 / TOTAL))

# This would need to be done more sophisticatedly in a real implementation
echo ""
echo "Please manually update the summary section in:"
echo "  $REPORT_FILE"
echo ""
echo "With these values:"
echo "  Total verified: $TOTAL tests"
echo "  CORRECT: $CORRECT tests ($CORRECT_PCT%)"
echo "  SUSPICIOUS: $SUSPICIOUS tests ($SUSPICIOUS_PCT%)"
echo "  INCORRECT: $INCORRECT tests ($INCORRECT_PCT%)"
echo ""
