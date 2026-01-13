#!/bin/bash
# Benchmark key operations across all categories
# N=57 Phase 5.1: Focused benchmarking (subset of 33 operations)
#
# Benchmarks 12 representative operations to demonstrate performance characteristics

set -euo pipefail

echo "=== Phase 5.1: Key Operations Benchmarking ==="
echo "Worker: N=57"
echo "Started: $(date)"
echo ""

# Test files selection
VIDEO_SMALL="test_edge_cases/video_tiny_64x64_resolution__scaling_test.mp4"
VIDEO_MEDIUM="test_edge_cases/video_hevc_h265_modern_codec__compatibility.mp4"
VIDEO_LARGE="test_edge_cases/video_4k_ultra_hd_3840x2160__stress_test.mp4"

AUDIO_SHORT="test_edge_cases/audio_very_short_1sec__duration_min.wav"
AUDIO_MEDIUM="test_edge_cases/audio_mono_single_channel__channel_test.wav"

IMAGE_MEDIUM="test_edge_cases/image_test_dog.jpg"
IMAGE_LARGE="test_edge_cases/image_test_mandrill.png"

# Key operations (12 representative operations from 33 total)
# Chosen to cover all major categories with most commonly used operations

echo "Benchmarking 12 key operations (representative subset of 33 total):"
echo ""

# Core Extraction (3/3)
echo "=== CORE EXTRACTION (3 operations) ==="
./benchmarks/benchmark_operation.sh metadata_extraction "$VIDEO_MEDIUM"
./benchmarks/benchmark_operation.sh keyframes "$VIDEO_SMALL" "$VIDEO_MEDIUM" "$VIDEO_LARGE"
./benchmarks/benchmark_operation.sh audio_extraction "$VIDEO_MEDIUM"

# Speech & Audio (2/8)
echo ""
echo "=== SPEECH & AUDIO (2 representative operations) ==="
./benchmarks/benchmark_operation.sh transcription "$AUDIO_SHORT" "$AUDIO_MEDIUM"
./benchmarks/benchmark_operation.sh voice_activity_detection "$AUDIO_MEDIUM"

# Vision Analysis (3/8)
echo ""
echo "=== VISION ANALYSIS (3 representative operations) ==="
./benchmarks/benchmark_operation.sh object_detection "$IMAGE_MEDIUM" "$IMAGE_LARGE"
./benchmarks/benchmark_operation.sh face_detection "$IMAGE_MEDIUM"
./benchmarks/benchmark_operation.sh ocr "$IMAGE_MEDIUM"

# Intelligence & Content (2/8)
echo ""
echo "=== INTELLIGENCE & CONTENT (2 representative operations) ==="
./benchmarks/benchmark_operation.sh image_quality_assessment "$IMAGE_MEDIUM"
./benchmarks/benchmark_operation.sh smart_thumbnail "$VIDEO_MEDIUM"

# Embeddings (1/2)
echo ""
echo "=== EMBEDDINGS (1 representative operation) ==="
./benchmarks/benchmark_operation.sh vision_embeddings "$IMAGE_MEDIUM"

# Utility (1/2)
echo ""
echo "=== UTILITY (1 representative operation) ==="
./benchmarks/benchmark_operation.sh duplicate_detection "$VIDEO_SMALL" "$VIDEO_MEDIUM"

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Key operations benchmarked (12/33)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Completed: $(date)"
echo ""
echo "Results available in: benchmarks/results/"
echo ""
ls -lht benchmarks/results/*.json | head -15
