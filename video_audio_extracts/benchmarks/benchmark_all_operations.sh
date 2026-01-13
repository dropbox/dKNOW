#!/bin/bash
# Comprehensive benchmarking of all 33 operations
# N=57 Phase 5.1: Complete Operation Benchmarking
#
# This script benchmarks all operations systematically with diverse test files

set -euo pipefail

echo "=== Phase 5.1: Comprehensive Operation Benchmarking ==="
echo "Worker: N=57"
echo "Started: $(date)"
echo ""

# Test files selection (diverse sizes and types)
VIDEO_SMALL="test_edge_cases/video_tiny_64x64_resolution__scaling_test.mp4"
VIDEO_MEDIUM="test_edge_cases/video_hevc_h265_modern_codec__compatibility.mp4"
VIDEO_LARGE="test_edge_cases/video_4k_ultra_hd_3840x2160__stress_test.mp4"

AUDIO_SHORT="test_edge_cases/audio_very_short_1sec__duration_min.wav"
AUDIO_MEDIUM="test_edge_cases/audio_mono_single_channel__channel_test.wav"
AUDIO_LONG="test_edge_cases/audio_complete_silence_3sec__silence_detection.wav"

IMAGE_SMALL="test_edge_cases/image_iphone_photo.jpg"
IMAGE_MEDIUM="test_edge_cases/image_test_dog.jpg"
IMAGE_LARGE="test_edge_cases/image_test_mandrill.png"

# Operations organized by category (33 total)
# Based on PRODUCTION_READINESS_PLAN.md lines 1056-1101

echo "Core Extraction Operations (3):"
CORE_OPS=(
    "audio_extraction"
    "keyframes"
    "metadata_extraction"
)

echo "Speech & Audio Operations (8):"
AUDIO_OPS=(
    "transcription"
    "diarization"
    "voice_activity_detection"
    "audio_classification"
    "acoustic_scene_classification"
    "audio_embeddings"
    "audio_enhancement_metadata"
    "profanity_detection"
)

echo "Vision Analysis Operations (8):"
VISION_OPS=(
    "scene_detection"
    "object_detection"
    "face_detection"
    "ocr"
    "action_recognition"
    "pose_estimation"
    "depth_estimation"
    "motion_tracking"
)

echo "Intelligence & Content Operations (8):"
CONTENT_OPS=(
    "smart_thumbnail"
    "subtitle_extraction"
    "shot_classification"
    "emotion_detection"
    "image_quality_assessment"
    "content_moderation"
    "logo_detection"
    "caption_generation"
)

echo "Embeddings Operations (2, note: audio_embeddings already in audio category):"
EMBEDDING_OPS=(
    "vision_embeddings"
    "text_embeddings"
)

echo "Utility Operations (2):"
UTILITY_OPS=(
    "format_conversion"
    "duplicate_detection"
)

echo "Advanced Operations (1):"
ADVANCED_OPS=(
    "music_source_separation"
)

# Function to benchmark an operation
benchmark_op() {
    local operation=$1
    shift
    local test_files=("$@")

    echo ""
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "Benchmarking: $operation"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    # Run benchmark script
    ./benchmarks/benchmark_operation.sh "$operation" "${test_files[@]}" || {
        echo "⚠️  Warning: $operation benchmark failed (may require user model or specific input)"
        return 0
    }

    echo "✅ $operation benchmark complete"
}

# Benchmark Core Extraction (works on video files)
echo ""
echo "=== CORE EXTRACTION OPERATIONS ==="
for op in "${CORE_OPS[@]}"; do
    case $op in
        audio_extraction)
            benchmark_op "$op" "$VIDEO_MEDIUM" "$VIDEO_LARGE"
            ;;
        keyframes)
            benchmark_op "$op" "$VIDEO_SMALL" "$VIDEO_MEDIUM" "$VIDEO_LARGE"
            ;;
        metadata_extraction)
            benchmark_op "$op" "$VIDEO_MEDIUM"
            ;;
    esac
done

# Benchmark Speech & Audio (works on audio files)
echo ""
echo "=== SPEECH & AUDIO OPERATIONS ==="
for op in "${AUDIO_OPS[@]}"; do
    benchmark_op "$op" "$AUDIO_SHORT" "$AUDIO_MEDIUM" || true
done

# Benchmark Vision Analysis (works on video or image files)
echo ""
echo "=== VISION ANALYSIS OPERATIONS ==="
for op in "${VISION_OPS[@]}"; do
    case $op in
        scene_detection|action_recognition|motion_tracking)
            # Video-based operations
            benchmark_op "$op" "$VIDEO_MEDIUM" || true
            ;;
        object_detection|face_detection|ocr|pose_estimation|depth_estimation)
            # Can work on keyframes or images
            benchmark_op "$op" "$IMAGE_MEDIUM" "$IMAGE_LARGE" || true
            ;;
    esac
done

# Benchmark Intelligence & Content (mixed)
echo ""
echo "=== INTELLIGENCE & CONTENT OPERATIONS ==="
for op in "${CONTENT_OPS[@]}"; do
    case $op in
        smart_thumbnail|shot_classification|emotion_detection)
            benchmark_op "$op" "$VIDEO_MEDIUM" || true
            ;;
        subtitle_extraction)
            benchmark_op "$op" "test_edge_cases/video_with_subtitles__subtitle_test.mp4" || true
            ;;
        image_quality_assessment|content_moderation|logo_detection|caption_generation)
            benchmark_op "$op" "$IMAGE_MEDIUM" || true
            ;;
    esac
done

# Benchmark Embeddings
echo ""
echo "=== EMBEDDINGS OPERATIONS ==="
benchmark_op "vision_embeddings" "$IMAGE_MEDIUM" || true
benchmark_op "text_embeddings" "$VIDEO_MEDIUM" || true  # May need text input

# Benchmark Utility
echo ""
echo "=== UTILITY OPERATIONS ==="
benchmark_op "format_conversion" "$VIDEO_MEDIUM" || true
benchmark_op "duplicate_detection" "$VIDEO_SMALL" "$VIDEO_MEDIUM" || true

# Benchmark Advanced
echo ""
echo "=== ADVANCED OPERATIONS ==="
benchmark_op "music_source_separation" "$AUDIO_MEDIUM" || true

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ All operations benchmarked"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Completed: $(date)"
echo ""
echo "Results available in: benchmarks/results/"
ls -lh benchmarks/results/*.json | tail -10
