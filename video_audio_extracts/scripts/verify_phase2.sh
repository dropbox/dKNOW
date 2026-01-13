#!/bin/bash
# Phase 2 AI Verification (30 tests)
# Tests supported formats: JPG, PNG, WebP + Transcription (MP3, WAV)
# Expands coverage to more diverse test files and operations
#
# Usage:
#   export OPENAI_API_KEY="$(cat OPENAI_API_KEY.txt)"
#   bash scripts/verify_phase2.sh

set -e

echo "=========================================="
echo "Phase 2 AI Verification (30 tests)"
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
REPORT="docs/ai-verification/PHASE2_GPT4_VERIFICATION_${TIMESTAMP}.csv"
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

# JPG Tests (10 tests - face, object, ocr, emotion, pose)
verify_test "jpg_face_1" "test_files_wikimedia/jpg/face-detection/02_123Makossa.jpg" "face-detection"
verify_test "jpg_face_2" "test_files_wikimedia/jpg/face-detection/01_2006_06_Marokko_131_Essaouira.jpg" "face-detection"
verify_test "jpg_object_1" "test_files_wikimedia/jpg/object-detection/01_2012_Tulip_Festival.jpg" "object-detection"
verify_test "jpg_object_2" "test_files_wikimedia/jpg/object-detection/01_139235_green_praying_mantis_PikiWiki_Israel.jpg" "object-detection"
verify_test "jpg_ocr_1" "test_files_wikimedia/jpg/ocr/02_@_Sainte_Marie_de_Bellaing_le_24_janvier_2012_-_047.jpg" "ocr"
verify_test "jpg_ocr_2" "test_files_wikimedia/jpg/ocr/01_2015_11_21_Breslau_025.jpg" "ocr"
verify_test "jpg_emotion_1" "test_files_wikimedia/jpg/emotion-detection/02_112615_FAMILY_PHOTO_0030_(23112090623).jpg" "emotion-detection"
verify_test "jpg_emotion_2" "test_files_wikimedia/jpg/emotion-detection/01_112615_FAMILY_PHOTO_0025_(23138314094).jpg" "emotion-detection"
verify_test "jpg_pose_1" "test_files_wikimedia/jpg/pose-estimation/02_18th_century_portrait_paintings_of_men,_with_Unidentified_sitter.jpg" "pose-estimation"
verify_test "jpg_pose_2" "test_files_wikimedia/jpg/pose-estimation/01_2018_Asian_Games_Taekwondo_Freestyle_Poomsae_Pair_28.jpg" "pose-estimation"

# PNG Tests (10 tests - face, object, ocr, emotion, pose)
verify_test "png_face_1" "test_files_wikimedia/png/face-detection/02_1st_Lt._Ryan_Mierau,_Colorado_Springs,_Colo.png" "face-detection"
verify_test "png_face_2" "test_files_wikimedia/png/face-detection/01_"The_Three_Years_Later,"_volume_2_-_DPLA_-_bf4d3c88b9f74c63f0a5098b07b3cf48_(page_48)_crop.png" "face-detection"
verify_test "png_object_1" "test_files_wikimedia/png/object-detection/06_Emulsions_for_nanomedicine_synthesis.png" "object-detection"
verify_test "png_object_2" "test_files_wikimedia/png/object-detection/05_Curva_bioclimática_PMV-PPD.png" "object-detection"
verify_test "png_ocr_1" "test_files_wikimedia/png/ocr/02_04d_Mapa_del_Palacio_de_Bellas_Artes_(Ciudad_de_México).png" "ocr"
verify_test "png_ocr_2" "test_files_wikimedia/png/ocr/01_00logo_Ferromex.png" "ocr"
verify_test "png_emotion_1" "test_files_wikimedia/png/emotion-detection/02_2020-05-11_Telefonieren_per_Videokonferenz_im_Homeoffice_während_der_Corona_Pandemie_01.png" "emotion-detection"
verify_test "png_emotion_2" "test_files_wikimedia/png/emotion-detection/01_2013-05-11_-_Denise_Lebon_em_Contagem-MG.png" "emotion-detection"
verify_test "png_pose_1" "test_files_wikimedia/png/pose-estimation/02_1890_Manet_Frau_mit_Fächer_anagoria.png" "pose-estimation"
verify_test "png_pose_2" "test_files_wikimedia/png/pose-estimation/01_08.-Miquel-Barceló-Paso-doble-2006-Bronze-147-×-58-×-40-cm.-Courtesy-of-the-artist-©-Miquel-Barceló-VEGAP-Madrid-2022.png" "pose-estimation"

# WebP Tests (5 tests - face, object, ocr, emotion)
verify_test "webp_face_1" "test_files_wikimedia/webp/emotion-detection/01_webp_lossy.webp" "face-detection"
verify_test "webp_object_1" "test_files_wikimedia/webp/object-detection/01_@_Sailly_Les_Lannoy_le_28_mai_2022_-_010.webp" "object-detection"
verify_test "webp_object_2" "test_files_wikimedia/webp/object-detection/02_A_barn_in_Ørnes.webp" "object-detection"
verify_test "webp_ocr_1" "test_files_wikimedia/webp/ocr/01_'Duffy_and_the_Devil'_by_Harve_Zemach_02.webp" "ocr"
verify_test "webp_emotion_1" "test_files_wikimedia/webp/emotion-detection/01_webp_lossy.webp" "emotion-detection"

# Transcription Tests (5 tests - MP3 + WAV)
verify_test "mp3_transcript_1" "test_files_wikimedia/mp3/transcription/02_Carla_Scaletti_on_natural_sounds_and_physical_modeling_(1999).mp3" "transcription"
verify_test "mp3_transcript_2" "test_files_wikimedia/mp3/transcription/file_example_MP3_1MG.mp3" "transcription"
verify_test "wav_transcript_1" "test_files_wikimedia/wav/transcription/02_2015-06-27-MayorStephenReed-re-Harrisburg.wav" "transcription"
verify_test "wav_transcript_2" "test_files_wikimedia/wav/transcription/01_2010_07_11_4_Almenrausch_u_Edelweiß_-_Instrumental.wav" "transcription"
verify_test "wav_transcript_3" "test_files_wikimedia/wav/audio-enhancement-metadata/03_LL-Q150_(fra)-WikiLucas00-audio.wav" "transcription"

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
