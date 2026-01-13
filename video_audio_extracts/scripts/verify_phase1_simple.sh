#!/bin/bash
# Phase 1 AI Verification (Simplified) with OpenAI GPT-4 Vision
# Focuses on 30 tests that GPT-4 Vision can directly verify
# - Standard image formats (JPG, PNG, WebP, BMP, HEIC)
# - Transcription tests (text verification)
#
# Skips:
# - RAW formats (ARW, CR2, DNG, NEF, RAF) - GPT-4 Vision doesn't support
# - Complex multi-stage operations requiring keyframe extraction
#
# Usage:
#   export OPENAI_API_KEY="sk-proj-..."
#   bash scripts/verify_phase1_simple.sh

set -e

echo "=========================================="
echo "Phase 1: AI Verification (Simplified)"
echo "30 tests - Standard formats only"
echo "=========================================="
echo ""

# Check API key
if [ -z "$OPENAI_API_KEY" ]; then
    echo "ERROR: OPENAI_API_KEY not set"
    echo "Run: export OPENAI_API_KEY=\"\$(cat OPENAI_API_KEY.txt)\""
    exit 1
fi

# Create output directory
mkdir -p docs/ai-verification

# Output file with timestamp
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
REPORT="docs/ai-verification/PHASE1_SIMPLE_GPT4_VERIFICATION_${TIMESTAMP}.csv"
echo "test_name,operation,input_file,status,confidence,findings" > "$REPORT"

# Counter for progress tracking
TOTAL=30
CURRENT=0

# Function to verify a single test (simple image input)
verify_simple_image() {
    local test_name=$1
    local file=$2
    local op=$3

    CURRENT=$((CURRENT + 1))
    echo "[$CURRENT/$TOTAL] Verifying: $test_name"
    echo "  File: $file"
    echo "  Operation: $op"

    # Run test to generate output
    if ! ./target/release/video-extract debug --ops "$op" "$file" >/dev/null 2>&1; then
        echo "  ❌ Binary execution failed"
        echo "\"$test_name\",\"$op\",\"$file\",\"ERROR\",\"0.0\",\"Binary execution failed\"" >> "$REPORT"
        return
    fi

    # Map operation to output file
    local output_file=""
    case "$op" in
        "face-detection") output_file="debug_output/stage_00_face_detection.json" ;;
        "object-detection") output_file="debug_output/stage_00_object_detection.json" ;;
        "ocr") output_file="debug_output/stage_00_ocr.json" ;;
        "pose-estimation") output_file="debug_output/stage_00_pose_estimation.json" ;;
        "emotion-detection") output_file="debug_output/stage_00_emotion_detection.json" ;;
        "transcription") output_file="debug_output/stage_00_transcription.json" ;;
        *)
            echo "  ⚠️  Unknown operation: $op"
            echo "\"$test_name\",\"$op\",\"$file\",\"ERROR\",\"0.0\",\"Unknown operation: $op\"" >> "$REPORT"
            return
            ;;
    esac

    if [ ! -f "$output_file" ]; then
        echo "  ❌ Output file not found: $output_file"
        echo "\"$test_name\",\"$op\",\"$file\",\"ERROR\",\"0.0\",\"Output file not found: $output_file\"" >> "$REPORT"
        return
    fi

    # AI verify
    local result
    if ! result=$(python3 scripts/ai_verify_outputs_openai.py "$file" "$output_file" "$op" 2>&1); then
        echo "  ❌ AI verification failed: ${result:0:100}..."
        local error_msg=$(echo "$result" | tr '\n' ' ' | sed 's/"/""/g')
        echo "\"$test_name\",\"$op\",\"$file\",\"ERROR\",\"0.0\",\"${error_msg:0:200}\"" >> "$REPORT"
        return
    fi

    # Parse JSON result
    local status=$(echo "$result" | python3 -c "import sys, json; data = json.loads(sys.stdin.read()); print(data.get('status', 'UNKNOWN'))" 2>/dev/null || echo "PARSE_ERROR")
    local confidence=$(echo "$result" | python3 -c "import sys, json; data = json.loads(sys.stdin.read()); print(data.get('confidence', 0.0))" 2>/dev/null || echo "0.0")
    local findings=$(echo "$result" | python3 -c "import sys, json; data = json.loads(sys.stdin.read()); print(data.get('findings', 'No findings'))" 2>/dev/null || echo "Parse error")

    # Clean findings for CSV
    findings=$(echo "$findings" | tr '\n' ' ' | sed 's/"/""/g')

    # Display result
    case "$status" in
        "CORRECT") echo "  ✅ CORRECT (confidence: $confidence)" ;;
        "SUSPICIOUS") echo "  ⚠️  SUSPICIOUS (confidence: $confidence)" ;;
        "INCORRECT") echo "  ❌ INCORRECT (confidence: $confidence)" ;;
        *) echo "  ❓ $status (confidence: $confidence)" ;;
    esac

    # Append to CSV
    echo "\"$test_name\",\"$op\",\"$file\",\"$status\",\"$confidence\",\"$findings\"" >> "$REPORT"
    echo ""
}

# Category 1: Standard Image Formats - Face Detection (5 tests)
echo "=== Category 1: Image Face Detection (5 tests) ==="
verify_simple_image "jpg_face_detection" "test_files_wikimedia/jpg/object-detection/01_0butterfly1_up-08316.jpg" "face-detection"
verify_simple_image "png_face_detection" "test_files_wikimedia/png/emotion-detection/02_123inkt_logo_transparent_bg_small.png" "face-detection"
verify_simple_image "webp_face_detection" "test_files_wikimedia/webp/emotion-detection/01_webp_lossy.webp" "face-detection"
verify_simple_image "bmp_face_detection" "test_files_wikimedia/bmp/emotion-detection/01_bmp_24bit.bmp" "face-detection"
verify_simple_image "heic_face_detection" "test_edge_cases/image_iphone_photo.heic" "face-detection"

# Category 2: Object Detection (5 tests)
echo "=== Category 2: Object Detection (5 tests) ==="
verify_simple_image "jpg_object_detection" "test_files_wikimedia/jpg/object-detection/01_0butterfly1_up-08316.jpg" "object-detection"
verify_simple_image "png_object_detection" "test_files_wikimedia/png/emotion-detection/02_123inkt_logo_transparent_bg_small.png" "object-detection"
verify_simple_image "webp_object_detection" "test_files_wikimedia/webp/emotion-detection/01_webp_lossy.webp" "object-detection"
verify_simple_image "bmp_object_detection" "test_files_wikimedia/bmp/emotion-detection/01_bmp_24bit.bmp" "object-detection"
verify_simple_image "heic_object_detection" "test_edge_cases/image_iphone_photo.heic" "object-detection"

# Category 3: OCR (5 tests)
echo "=== Category 3: OCR (5 tests) ==="
verify_simple_image "jpg_ocr" "test_files_wikimedia/jpg/object-detection/01_0butterfly1_up-08316.jpg" "ocr"
verify_simple_image "png_ocr" "test_files_wikimedia/png/emotion-detection/02_123inkt_logo_transparent_bg_small.png" "ocr"
verify_simple_image "webp_ocr" "test_files_wikimedia/webp/emotion-detection/01_webp_lossy.webp" "ocr"
verify_simple_image "bmp_ocr" "test_files_wikimedia/bmp/emotion-detection/01_bmp_24bit.bmp" "ocr"
verify_simple_image "heic_ocr" "test_edge_cases/image_iphone_photo.heic" "ocr"

# Category 4: Pose Estimation (5 tests)
echo "=== Category 4: Pose Estimation (5 tests) ==="
verify_simple_image "jpg_pose" "test_files_wikimedia/jpg/object-detection/01_0butterfly1_up-08316.jpg" "pose-estimation"
verify_simple_image "png_pose" "test_files_wikimedia/png/emotion-detection/02_123inkt_logo_transparent_bg_small.png" "pose-estimation"
verify_simple_image "webp_pose" "test_files_wikimedia/webp/emotion-detection/01_webp_lossy.webp" "pose-estimation"
verify_simple_image "bmp_pose" "test_files_wikimedia/bmp/emotion-detection/01_bmp_24bit.bmp" "pose-estimation"
verify_simple_image "heic_pose" "test_edge_cases/image_iphone_photo.heic" "pose-estimation"

# Category 5: Emotion Detection (5 tests)
echo "=== Category 5: Emotion Detection (5 tests) ==="
verify_simple_image "jpg_emotion" "test_files_wikimedia/jpg/object-detection/01_0butterfly1_up-08316.jpg" "emotion-detection"
verify_simple_image "png_emotion" "test_files_wikimedia/png/emotion-detection/02_123inkt_logo_transparent_bg_small.png" "emotion-detection"
verify_simple_image "webp_emotion" "test_files_wikimedia/webp/emotion-detection/01_webp_lossy.webp" "emotion-detection"
verify_simple_image "bmp_emotion" "test_files_wikimedia/bmp/emotion-detection/01_bmp_24bit.bmp" "emotion-detection"
verify_simple_image "heic_emotion" "test_edge_cases/image_iphone_photo.heic" "emotion-detection"

# Category 6: Transcription (5 tests)
echo "=== Category 6: Audio Transcription (5 tests) ==="
verify_simple_image "mp3_transcription" "test_files_wikimedia/mp3/transcription/02_Carla_Scaletti_on_natural_sounds_and_physical_modeling_(1999).mp3" "transcription"
verify_simple_image "m4a_transcription" "test_files_wikimedia/m4a/transcription/zoom_audio_sept18.m4a" "transcription"
verify_simple_image "ogg_transcription" "test_edge_cases/format_test_ogg.ogg" "transcription"
verify_simple_image "flac_transcription" "test_files_wikimedia/flac/transcription/04_Aina_zilizo_hatarini.flac.flac" "transcription"
verify_simple_image "wav_transcription" "test_files_wikimedia/wav/audio-enhancement-metadata/03_LL-Q150_(fra)-WikiLucas00-audio.wav" "transcription"

# Summary
echo "=========================================="
echo "Phase 1 (Simplified) Verification Complete"
echo "=========================================="
echo ""
echo "Results saved to: $REPORT"
echo ""

# Count results
CORRECT=$(grep -c "\"CORRECT\"" "$REPORT" || true)
SUSPICIOUS=$(grep -c "\"SUSPICIOUS\"" "$REPORT" || true)
INCORRECT=$(grep -c "\"INCORRECT\"" "$REPORT" || true)
ERROR=$(grep -c "\"ERROR\"" "$REPORT" || true)

echo "Summary:"
echo "  ✅ CORRECT:    $CORRECT"
echo "  ⚠️  SUSPICIOUS: $SUSPICIOUS"
echo "  ❌ INCORRECT:  $INCORRECT"
echo "  ❓ ERROR:      $ERROR"
echo "  Total:        $TOTAL"
echo ""

# Calculate success rate
if [ "$TOTAL" -gt 0 ]; then
    SUCCESS_RATE=$(echo "scale=1; $CORRECT * 100 / $TOTAL" | bc)
    echo "Success rate: ${SUCCESS_RATE}%"
    echo ""

    if [ "$CORRECT" -ge 27 ]; then
        echo "✅ Phase 1 PASSED (≥90% correct)"
    else
        echo "⚠️  Phase 1 needs investigation (<90% correct)"
    fi
fi
