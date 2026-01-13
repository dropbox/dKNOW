#!/bin/bash
# Minimal AI Verification Test (10 tests)
# Tests only confirmed working formats: JPG, PNG, WebP + Transcription
#
# Usage:
#   export OPENAI_API_KEY="sk-proj-..."
#   bash scripts/verify_phase1_minimal.sh

set -e

echo "=========================================="
echo "Minimal AI Verification (10 tests)"
echo "JPG/PNG/WebP + Transcription only"
echo "=========================================="
echo ""

# Check API key
if [ -z "$OPENAI_API_KEY" ]; then
    echo "ERROR: OPENAI_API_KEY not set"
    exit 1
fi

# Create output directory
mkdir -p docs/ai-verification

# Output file
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
REPORT="docs/ai-verification/PHASE1_MINIMAL_GPT4_VERIFICATION_${TIMESTAMP}.csv"
echo "test_name,operation,input_file,status,confidence,findings" > "$REPORT"

TOTAL=10
CURRENT=0

# Function to verify
verify_test() {
    local test_name=$1
    local file=$2
    local op=$3

    CURRENT=$((CURRENT + 1))
    echo "[$CURRENT/$TOTAL] $test_name"

    if ! ./target/release/video-extract debug --ops "$op" "$file" >/dev/null 2>&1; then
        echo "  ❌ Binary failed"
        echo "\"$test_name\",\"$op\",\"$file\",\"ERROR\",\"0.0\",\"Binary execution failed\"" >> "$REPORT"
        return
    fi

    local output_file=""
    case "$op" in
        "face-detection") output_file="debug_output/stage_00_face_detection.json" ;;
        "object-detection") output_file="debug_output/stage_00_object_detection.json" ;;
        "ocr") output_file="debug_output/stage_00_ocr.json" ;;
        "transcription") output_file="debug_output/stage_00_transcription.json" ;;
    esac

    if [ ! -f "$output_file" ]; then
        echo "  ❌ Output not found"
        echo "\"$test_name\",\"$op\",\"$file\",\"ERROR\",\"0.0\",\"Output not found\"" >> "$REPORT"
        return
    fi

    local result
    if ! result=$(python3 scripts/ai_verify_outputs_openai.py "$file" "$output_file" "$op" 2>&1); then
        echo "  ❌ AI failed"
        echo "\"$test_name\",\"$op\",\"$file\",\"ERROR\",\"0.0\",\"AI verification failed\"" >> "$REPORT"
        return
    fi

    local status=$(echo "$result" | python3 -c "import sys, json; print(json.loads(sys.stdin.read()).get('status', 'UNKNOWN'))" 2>/dev/null || echo "ERROR")
    local conf=$(echo "$result" | python3 -c "import sys, json; print(json.loads(sys.stdin.read()).get('confidence', 0.0))" 2>/dev/null || echo "0.0")
    local find=$(echo "$result" | python3 -c "import sys, json; print(json.loads(sys.stdin.read()).get('findings', ''))" 2>/dev/null || echo "")
    find=$(echo "$find" | tr '\n' ' ' | sed 's/"/""/g')

    case "$status" in
        "CORRECT") echo "  ✅ CORRECT ($conf)" ;;
        "SUSPICIOUS") echo "  ⚠️  SUSPICIOUS ($conf)" ;;
        "INCORRECT") echo "  ❌ INCORRECT ($conf)" ;;
        *) echo "  ❓ $status" ;;
    esac

    echo "\"$test_name\",\"$op\",\"$file\",\"$status\",\"$conf\",\"$find\"" >> "$REPORT"
}

# JPG Tests (3)
verify_test "jpg_face" "test_files_wikimedia/jpg/object-detection/01_0butterfly1_up-08316.jpg" "face-detection"
verify_test "jpg_object" "test_files_wikimedia/jpg/object-detection/01_0butterfly1_up-08316.jpg" "object-detection"
verify_test "jpg_ocr" "test_files_wikimedia/jpg/object-detection/01_0butterfly1_up-08316.jpg" "ocr"

# PNG Tests (3)
verify_test "png_face" "test_files_wikimedia/png/emotion-detection/02_123inkt_logo_transparent_bg_small.png" "face-detection"
verify_test "png_object" "test_files_wikimedia/png/emotion-detection/02_123inkt_logo_transparent_bg_small.png" "object-detection"
verify_test "png_ocr" "test_files_wikimedia/png/emotion-detection/02_123inkt_logo_transparent_bg_small.png" "ocr"

# WebP Tests (2)
verify_test "webp_face" "test_files_wikimedia/webp/emotion-detection/01_webp_lossy.webp" "face-detection"
verify_test "webp_object" "test_files_wikimedia/webp/emotion-detection/01_webp_lossy.webp" "object-detection"

# Transcription Tests (2)
verify_test "mp3_transcript" "test_files_wikimedia/mp3/transcription/02_Carla_Scaletti_on_natural_sounds_and_physical_modeling_(1999).mp3" "transcription"
verify_test "wav_transcript" "test_files_wikimedia/wav/audio-enhancement-metadata/03_LL-Q150_(fra)-WikiLucas00-audio.wav" "transcription"

# Summary
echo ""
echo "=========================================="
echo "Results: $REPORT"
echo "=========================================="

CORRECT=$(grep -c "\"CORRECT\"" "$REPORT" || true)
SUSPICIOUS=$(grep -c "\"SUSPICIOUS\"" "$REPORT" || true)
INCORRECT=$(grep -c "\"INCORRECT\"" "$REPORT" || true)
ERROR=$(grep -c "\"ERROR\"" "$REPORT" || true)

echo "✅ CORRECT:    $CORRECT"
echo "⚠️  SUSPICIOUS: $SUSPICIOUS"
echo "❌ INCORRECT:  $INCORRECT"
echo "❓ ERROR:      $ERROR"
echo "Total:        $TOTAL"
echo ""

if [ "$CORRECT" -ge 9 ]; then
    echo "✅ Verification PASSED (≥90% correct)"
else
    echo "⚠️  Verification needs investigation"
fi
