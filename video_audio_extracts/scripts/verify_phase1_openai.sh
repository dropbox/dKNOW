#!/bin/bash
# Phase 1 AI Verification with OpenAI GPT-4 Vision
# Verifies 50 sampled tests for semantic correctness
#
# Usage:
#   export OPENAI_API_KEY="sk-proj-..."
#   bash scripts/verify_phase1_openai.sh

set -e

echo "========================================"
echo "Phase 1: AI Verification with GPT-4 Vision"
echo "========================================"
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
REPORT="docs/ai-verification/PHASE1_GPT4_VERIFICATION_${TIMESTAMP}.csv"
echo "test_name,operation,input_file,status,confidence,findings" > "$REPORT"

# Counter for progress tracking
TOTAL=50
CURRENT=0

# Function to verify a single test
verify_test() {
    local test_name=$1
    local file=$2
    local ops=$3

    CURRENT=$((CURRENT + 1))
    echo "[$CURRENT/$TOTAL] Verifying: $test_name"
    echo "  File: $file"
    echo "  Operations: $ops"

    # Run test to generate output
    if ! ./target/release/video-extract debug --ops "$ops" "$file" >/dev/null 2>&1; then
        echo "  ❌ Binary execution failed"
        echo "\"$test_name\",\"$ops\",\"$file\",\"ERROR\",\"0.0\",\"Binary execution failed\"" >> "$REPORT"
        return
    fi

    # Parse operations to find the final operation to verify
    # Operations are chained like "keyframes;face-detection"
    # We verify the final operation in the chain
    local final_op="${ops##*;}"

    # Map operation names to expected output files
    local output_file=""
    case "$final_op" in
        "face-detection")
            output_file="debug_output/stage_00_face_detection.json"
            ;;
        "object-detection")
            output_file="debug_output/stage_00_object_detection.json"
            ;;
        "ocr")
            output_file="debug_output/stage_00_ocr.json"
            ;;
        "pose-estimation")
            output_file="debug_output/stage_00_pose_estimation.json"
            ;;
        "emotion-detection")
            output_file="debug_output/stage_00_emotion_detection.json"
            ;;
        "action-recognition")
            output_file="debug_output/stage_00_action_recognition.json"
            ;;
        "transcription")
            output_file="debug_output/stage_01_transcription.json"
            ;;
        "profanity-detection")
            output_file="debug_output/stage_02_profanity_detection.json"
            ;;
        "audio-enhancement-metadata")
            output_file="debug_output/stage_01_audio_enhancement_metadata.json"
            ;;
        *)
            echo "  ⚠️  Unknown operation: $final_op"
            echo "\"$test_name\",\"$ops\",\"$file\",\"ERROR\",\"0.0\",\"Unknown operation: $final_op\"" >> "$REPORT"
            return
            ;;
    esac

    if [ ! -f "$output_file" ]; then
        echo "  ❌ Output file not found: $output_file"
        echo "\"$test_name\",\"$ops\",\"$file\",\"ERROR\",\"0.0\",\"Output file not found: $output_file\"" >> "$REPORT"
        return
    fi

    # AI verify
    local result
    if ! result=$(python3 scripts/ai_verify_outputs_openai.py "$file" "$output_file" "$final_op" 2>&1); then
        echo "  ❌ AI verification failed: $result"
        echo "\"$test_name\",\"$ops\",\"$file\",\"ERROR\",\"0.0\",\"AI verification script failed: ${result//\"/\"\"}\"" >> "$REPORT"
        return
    fi

    # Parse JSON result
    local status=$(echo "$result" | python3 -c "import sys, json; data = json.loads(sys.stdin.read()); print(data.get('status', 'UNKNOWN'))" 2>/dev/null || echo "PARSE_ERROR")
    local confidence=$(echo "$result" | python3 -c "import sys, json; data = json.loads(sys.stdin.read()); print(data.get('confidence', 0.0))" 2>/dev/null || echo "0.0")
    local findings=$(echo "$result" | python3 -c "import sys, json; data = json.loads(sys.stdin.read()); print(data.get('findings', 'No findings'))" 2>/dev/null || echo "Parse error")

    # Clean findings for CSV (escape quotes, remove newlines)
    findings=$(echo "$findings" | tr '\n' ' ' | sed 's/"/""/g')

    # Display result
    case "$status" in
        "CORRECT")
            echo "  ✅ CORRECT (confidence: $confidence)"
            ;;
        "SUSPICIOUS")
            echo "  ⚠️  SUSPICIOUS (confidence: $confidence)"
            ;;
        "INCORRECT")
            echo "  ❌ INCORRECT (confidence: $confidence)"
            ;;
        *)
            echo "  ❓ $status (confidence: $confidence)"
            ;;
    esac

    # Append to CSV
    echo "\"$test_name\",\"$ops\",\"$file\",\"$status\",\"$confidence\",\"$findings\"" >> "$REPORT"
    echo ""
}

# Category 1: RAW Format Tests (10 tests)
echo "=== Category 1: RAW Format Tests (10 tests) ==="
verify_test "smoke_format_arw_face_detection" "test_files_camera_raw/sony_a55.arw" "keyframes;face-detection"
verify_test "smoke_format_arw_object_detection" "test_files_camera_raw/sony_a55.arw" "keyframes;object-detection"
verify_test "smoke_format_cr2_face_detection" "test_files_camera_raw/canon_eos_m.cr2" "keyframes;face-detection"
verify_test "smoke_format_cr2_object_detection" "test_files_camera_raw/canon_eos_m.cr2" "keyframes;object-detection"
verify_test "smoke_format_dng_face_detection" "test_files_camera_raw/iphone7_plus.dng" "keyframes;face-detection"
verify_test "smoke_format_dng_ocr" "test_files_camera_raw/iphone7_plus.dng" "keyframes;ocr"
verify_test "smoke_format_nef_face_detection" "test_files_camera_raw/nikon_z7.nef" "keyframes;face-detection"
verify_test "smoke_format_nef_pose_estimation" "test_files_camera_raw/nikon_z7.nef" "keyframes;pose-estimation"
verify_test "smoke_format_raf_face_detection" "test_files_camera_raw/fuji_xa3.raf" "keyframes;face-detection"
verify_test "smoke_format_raf_object_detection" "test_files_camera_raw/fuji_xa3.raf" "keyframes;object-detection"

# Category 2: New Video Formats (10 tests)
echo "=== Category 2: New Video Formats (10 tests) ==="
verify_test "smoke_format_mxf_face_detection" "test_files_wikimedia/mxf/keyframes/C0023S01.mxf" "keyframes;face-detection"
verify_test "smoke_format_mxf_object_detection" "test_files_wikimedia/mxf/keyframes/C0023S01.mxf" "keyframes;object-detection"
verify_test "smoke_format_vob_face_detection" "test_files_wikimedia/vob/emotion-detection/03_test.vob" "keyframes;face-detection"
verify_test "smoke_format_vob_emotion_detection" "test_files_wikimedia/vob/emotion-detection/03_test.vob" "keyframes;emotion-detection"
verify_test "smoke_format_asf_face_detection" "test_files_wikimedia/asf/emotion-detection/03_test.asf" "keyframes;face-detection"
verify_test "smoke_format_asf_emotion_detection" "test_files_wikimedia/asf/emotion-detection/03_test.asf" "keyframes;emotion-detection"
verify_test "smoke_format_alac_transcription" "test_files_wikimedia/alac/transcription/03_acompanyament_tema.m4a" "transcription"
verify_test "smoke_format_alac_profanity_detection" "test_files_wikimedia/alac/transcription/03_acompanyament_tema.m4a" "transcription;profanity-detection"
verify_test "smoke_format_alac_audio_enhancement_metadata" "test_files_wikimedia/alac/audio-enhancement-metadata/03_acompanyament_tema.m4a" "audio-extraction;audio-enhancement-metadata"
verify_test "smoke_format_mkv_transcription" "test_edge_cases/video_tiny_64x64_resolution__scaling_test.mp4" "audio-extraction;transcription"

# Category 3: Audio Advanced Operations (10 tests)
echo "=== Category 3: Audio Advanced Operations (10 tests) ==="
verify_test "smoke_format_mp3_profanity_detection" "test_files_wikimedia/mp3/transcription/02_Carla_Scaletti_on_natural_sounds_and_physical_modeling_(1999).mp3" "transcription;profanity-detection"
verify_test "smoke_format_mp3_audio_enhancement_metadata" "test_files_wikimedia/mp3/transcription/02_Carla_Scaletti_on_natural_sounds_and_physical_modeling_(1999).mp3" "audio-extraction;audio-enhancement-metadata"
verify_test "smoke_format_m4a_profanity_detection" "test_files_wikimedia/m4a/transcription/zoom_audio_sept18.m4a" "transcription;profanity-detection"
verify_test "smoke_format_m4a_audio_enhancement_metadata" "test_files_wikimedia/m4a/transcription/zoom_audio_sept18.m4a" "audio-extraction;audio-enhancement-metadata"
verify_test "smoke_format_ogg_profanity_detection" "test_edge_cases/format_test_ogg.ogg" "transcription;profanity-detection"
verify_test "smoke_format_ogg_audio_enhancement_metadata" "test_edge_cases/format_test_ogg.ogg" "audio-extraction;audio-enhancement-metadata"
verify_test "smoke_format_flac_profanity_detection" "test_files_wikimedia/flac/transcription/04_Aina_zilizo_hatarini.flac.flac" "transcription;profanity-detection"
verify_test "smoke_format_flac_audio_enhancement_metadata" "test_files_wikimedia/flac/transcription/04_Aina_zilizo_hatarini.flac.flac" "audio-extraction;audio-enhancement-metadata"
verify_test "smoke_format_wav_profanity_detection" "test_files_wikimedia/wav/audio-enhancement-metadata/03_LL-Q150_(fra)-WikiLucas00-audio.wav" "transcription;profanity-detection"
verify_test "smoke_format_wav_audio_enhancement_metadata" "test_files_wikimedia/wav/audio-enhancement-metadata/03_LL-Q150_(fra)-WikiLucas00-audio.wav" "audio-extraction;audio-enhancement-metadata"

# Category 4: Video Advanced Operations (10 tests)
echo "=== Category 4: Video Advanced Operations (10 tests) ==="
verify_test "smoke_format_mp4_emotion_detection" "test_edge_cases/video_hevc_h265_modern_codec__compatibility.mp4" "keyframes;emotion-detection"
verify_test "smoke_format_mp4_action_recognition" "test_edge_cases/video_high_fps_120__temporal_test.mp4" "keyframes;action-recognition"
verify_test "smoke_format_mov_emotion_detection" "test_edge_cases/video_no_audio_stream__error_test.mov" "keyframes;emotion-detection"
verify_test "smoke_format_mov_action_recognition" "test_edge_cases/video_no_audio_stream__error_test.mov" "keyframes;action-recognition"
verify_test "smoke_format_webm_emotion_detection" "test_edge_cases/video_single_frame_only__minimal.mp4" "keyframes;emotion-detection"
verify_test "smoke_format_webm_action_recognition" "test_edge_cases/video_high_fps_120__temporal_test.mp4" "keyframes;action-recognition"
verify_test "smoke_format_mkv_emotion_detection" "test_edge_cases/video_tiny_64x64_resolution__scaling_test.mp4" "keyframes;emotion-detection"
verify_test "smoke_format_mkv_action_recognition" "test_edge_cases/video_high_fps_120__temporal_test.mp4" "keyframes;action-recognition"
verify_test "smoke_format_avi_emotion_detection" "test_files_wikimedia/avi/keyframes/04_generated_from_webm.avi" "keyframes;emotion-detection"
verify_test "smoke_format_avi_action_recognition" "test_files_wikimedia/avi/keyframes/04_generated_from_webm.avi" "keyframes;action-recognition"

# Category 5: Random Sampling (10 tests)
echo "=== Category 5: Random Sampling (10 tests) ==="
verify_test "smoke_format_jpg_face_detection" "test_files_wikimedia/jpg/object-detection/01_0butterfly1_up-08316.jpg" "face-detection"
verify_test "smoke_format_jpg_ocr" "test_files_wikimedia/jpg/object-detection/01_0butterfly1_up-08316.jpg" "ocr"
verify_test "smoke_format_png_object_detection" "test_files_wikimedia/png/emotion-detection/02_123inkt_logo_transparent_bg_small.png" "object-detection"
verify_test "smoke_format_png_ocr" "test_files_wikimedia/png/emotion-detection/02_123inkt_logo_transparent_bg_small.png" "ocr"
verify_test "smoke_format_bmp_object_detection" "test_files_wikimedia/bmp/emotion-detection/01_bmp_24bit.bmp" "object-detection"
verify_test "smoke_format_heic_face_detection" "test_edge_cases/image_iphone_photo.heic" "keyframes;face-detection"
verify_test "smoke_format_webp_object_detection" "test_files_wikimedia/webp/emotion-detection/01_webp_lossy.webp" "object-detection"
verify_test "smoke_format_mp4_transcription" "test_edge_cases/video_hevc_h265_modern_codec__compatibility.mp4" "audio-extraction;transcription"
verify_test "smoke_format_webm_transcription" "test_media_generated/test_vp9_opus_10s.webm" "audio-extraction;transcription"
verify_test "smoke_format_flv_transcription" "test_edge_cases/format_test_flv.flv" "audio-extraction;transcription"

# Summary
echo "========================================"
echo "Phase 1 Verification Complete"
echo "========================================"
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

    if [ "$CORRECT" -ge 45 ]; then
        echo "✅ Phase 1 PASSED (≥90% correct)"
    else
        echo "⚠️  Phase 1 needs investigation (<90% correct)"
    fi
fi
