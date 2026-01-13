#!/bin/bash
# Phase 2 AI Verification (Retry with Verified Paths)
# Generated from verified test file inventory
# Tests supported formats: JPG, PNG, WebP + Transcription (MP3, WAV)
#
# Usage:
#   export OPENAI_API_KEY="$(cat OPENAI_API_KEY.txt)"
#   bash scripts/verify_phase2_verified.sh

set -e

echo "=========================================="
echo "Phase 2 AI Verification (30 tests - Verified Paths)"
echo "JPG/PNG/WebP + Transcription"
echo "=========================================="
echo ""

# Check API key
if [ -z "$OPENAI_API_KEY" ]; then
    if [ -f "OPENAI_API_KEY.txt" ]; then
        export OPENAI_API_KEY="$(cat OPENAI_API_KEY.txt)"
        echo "Loaded API key from OPENAI_API_KEY.txt"
    else
        echo "ERROR: OPENAI_API_KEY not set and OPENAI_API_KEY.txt not found"
        exit 1
    fi
fi

# Create output directory
mkdir -p docs/ai-verification

# Output file
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
REPORT="docs/ai-verification/PHASE2_RETRY_GPT4_VERIFICATION_${TIMESTAMP}.csv"
echo "test_name,operation,input_file,status,confidence,findings" > "$REPORT"

TOTAL=30
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
        "emotion-detection") output_file="debug_output/stage_00_emotion_detection.json" ;;
        "pose-estimation") output_file="debug_output/stage_00_pose_estimation.json" ;;
    esac

    if [ ! -f "$output_file" ]; then
        echo "  ❌ Output not found"
        echo "\"$test_name\",\"$op\",\"$file\",\"ERROR\",\"0.0\",\"Output not found\"" >> "$REPORT"
        return
    fi

    local result
    if ! result=$(python3 scripts/ai_verify_openai.py "$file" "$output_file" "$op" 2>&1); then
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

# Tests with verified file paths
verify_test "jpg_face_detection_1" "test_files_wikimedia/jpg/face-detection/06_2023-06-20_Vills_Foscari_16.jpg" "face-detection"
verify_test "jpg_face_detection_2" "test_files_wikimedia/jpg/face-detection/09_Alexey_Akindinov._Boy_and_Crystal._1997.jpg" "face-detection"
verify_test "jpg_object_detection_3" "test_files_wikimedia/jpg/object-detection/01_\"Amelia\"_(Homage_to_Amelia_Earhart)_by_Mary_Curtis_Ratcliff.jpg" "object-detection"
verify_test "jpg_object_detection_4" "test_files_wikimedia/jpg/object-detection/04_150229-ColourwithStorage-Scene1_output.jpg" "object-detection"
verify_test "jpg_ocr_5" "test_files_wikimedia/jpg/ocr/01_\"Amelia\"_(Homage_to_Amelia_Earhart)_by_Mary_Curtis_Ratcliff.jpg" "ocr"
verify_test "jpg_ocr_6" "test_files_wikimedia/jpg/ocr/09_Israeli_postal_card_50s.jpg" "ocr"
verify_test "jpg_emotion_detection_7" "test_files_wikimedia/jpg/emotion-detection/01_\"Amelia\"_(Homage_to_Amelia_Earhart)_by_Mary_Curtis_Ratcliff.jpg" "emotion-detection"
verify_test "jpg_emotion_detection_8" "test_files_wikimedia/jpg/emotion-detection/04_Дерево_в_городе_Валуйки.jpg" "emotion-detection"
verify_test "jpg_pose_estimation_9" "test_files_wikimedia/jpg/pose-estimation/01_\"Amelia\"_(Homage_to_Amelia_Earhart)_by_Mary_Curtis_Ratcliff.jpg" "pose-estimation"
verify_test "jpg_pose_estimation_10" "test_files_wikimedia/jpg/pose-estimation/10_Australian_Shepherd_red_merle_portrait_Canon_EOS_700D_Pentacon_200mm_f4.jpg" "pose-estimation"
verify_test "png_face_detection_11" "test_files_wikimedia/png/face-detection/01_'Aside_4.'_-_small_abstract_painting_sketch_on_paper,_made_in_2016_in_watercolor_by_Dutch_artist_Fons_Heijnsbroek.png" "face-detection"
verify_test "png_face_detection_12" "test_files_wikimedia/png/face-detection/05_A_setiset_with_duplicated_piece.png" "face-detection"
verify_test "png_object_detection_13" "test_files_wikimedia/png/object-detection/01_'Aside_4.'_-_small_abstract_painting_sketch_on_paper,_made_in_2016_in_watercolor_by_Dutch_artist_Fons_Heijnsbroek.png" "object-detection"
verify_test "png_object_detection_14" "test_files_wikimedia/png/object-detection/05_A_setiset_with_duplicated_piece.png" "object-detection"
verify_test "png_ocr_15" "test_files_wikimedia/png/ocr/01_'Aside_4.'_-_small_abstract_painting_sketch_on_paper,_made_in_2016_in_watercolor_by_Dutch_artist_Fons_Heijnsbroek.png" "ocr"
verify_test "png_ocr_16" "test_files_wikimedia/png/ocr/05_A_setiset_with_duplicated_piece.png" "ocr"
verify_test "png_emotion_detection_17" "test_files_wikimedia/png/emotion-detection/01_'Aside_4.'_-_small_abstract_painting_sketch_on_paper,_made_in_2016_in_watercolor_by_Dutch_artist_Fons_Heijnsbroek.png" "emotion-detection"
verify_test "png_emotion_detection_18" "test_files_wikimedia/png/emotion-detection/05_A_setiset_with_duplicated_piece.png" "emotion-detection"
verify_test "png_pose_estimation_19" "test_files_wikimedia/png/pose-estimation/01_'Aside_4.'_-_small_abstract_painting_sketch_on_paper,_made_in_2016_in_watercolor_by_Dutch_artist_Fons_Heijnsbroek.png" "pose-estimation"
verify_test "png_pose_estimation_20" "test_files_wikimedia/png/pose-estimation/05_A_setiset_with_duplicated_piece.png" "pose-estimation"
verify_test "webp_object_detection_21" "test_files_wikimedia/webp/object-detection/02_webp_lossy.webp" "object-detection"
verify_test "webp_object_detection_22" "test_files_wikimedia/webp/object-detection/05_webp_lossy.webp" "object-detection"
verify_test "webp_ocr_23" "test_files_wikimedia/webp/ocr/01_webp_lossy.webp" "ocr"
verify_test "webp_emotion_detection_24" "test_files_wikimedia/webp/emotion-detection/01_webp_lossy.webp" "emotion-detection"
verify_test "mp3_transcription_25" "test_files_wikimedia/mp3/transcription/02_Carla_Scaletti_on_natural_sounds_and_physical_modeling_(1999).mp3" "transcription"
verify_test "mp3_transcription_26" "test_files_wikimedia/mp3/transcription/file_example_MP3_1MG.mp3" "transcription"
verify_test "wav_transcription_27" "test_files_wikimedia/wav/transcription/03_LL-Q150_(fra)-WikiLucas00-audio.wav" "transcription"
verify_test "wav_transcription_28" "test_files_wikimedia/wav/transcription/01_01a_rodzaje_sygnalow_alarmowych.wav" "transcription"
verify_test "wav_transcription_29" "test_files_wikimedia/wav/transcription/02_03a_rodzaje_sygnalow_alarmowych.wav" "transcription"

# Summary
echo ""
echo "=========================================="
echo "Results: $REPORT"
echo "=========================================="

CORRECT=$(grep -c "\"CORRECT\"" "$REPORT" || true)
SUSPICIOUS=$(grep -c "\"SUSPICIOUS\"" "$REPORT" || true)
INCORRECT=$(grep -c "\"INCORRECT\"" "$REPORT" || true)
ERROR=$(grep -c "\"ERROR\"" "$REPORT" || true)

echo "✅ CORRECT:    $CORRECT / $TOTAL"
echo "⚠️  SUSPICIOUS: $SUSPICIOUS / $TOTAL"
echo "❌ INCORRECT:  $INCORRECT / $TOTAL"
echo "❓ ERROR:      $ERROR / $TOTAL"
echo ""

if [ "$CORRECT" -ge 27 ]; then
    echo "✅ Verification PASSED (≥90% correct: $CORRECT/$TOTAL)"
elif [ "$CORRECT" -ge 24 ]; then
    echo "⚠️  Verification ACCEPTABLE (80-90% correct: $CORRECT/$TOTAL)"
else
    echo "❌ Verification NEEDS INVESTIGATION (<80% correct: $CORRECT/$TOTAL)"
fi
