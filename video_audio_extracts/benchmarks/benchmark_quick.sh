#!/bin/bash
# Quick benchmark of key operations (single run, sufficient for documentation)
# N=57 Phase 5.1: Performance documentation
#
# Uses /usr/bin/time for fast, single-run measurements

set -euo pipefail

echo "=== Phase 5.1: Quick Performance Benchmarking ==="
echo "Worker: N=57"
echo "Started: $(date)"
echo ""

# Output file
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
OUTPUT_FILE="benchmarks/results/quick_benchmark_${TIMESTAMP}.json"
mkdir -p benchmarks/results

# Hardware info
CPU=$(sysctl -n machdep.cpu.brand_string)
MEMORY_GB=$(sysctl -n hw.memsize | awk '{print $1/1024/1024/1024}')
OS="$(uname -s) $(uname -r)"

echo "Hardware:"
echo "  CPU: $CPU"
echo "  Memory: ${MEMORY_GB} GB"
echo "  OS: $OS"
echo ""

# Test files
VIDEO_SMALL="test_edge_cases/video_tiny_64x64_resolution__scaling_test.mp4"
VIDEO_MEDIUM="test_edge_cases/video_hevc_h265_modern_codec__compatibility.mp4"
VIDEO_LARGE="test_edge_cases/video_4k_ultra_hd_3840x2160__stress_test.mp4"
AUDIO_SHORT="test_edge_cases/audio_very_short_1sec__duration_min.wav"
AUDIO_MEDIUM="test_edge_cases/audio_mono_single_channel__channel_test.wav"
IMAGE_MEDIUM="test_edge_cases/image_test_dog.jpg"

BINARY="./target/release/video-extract"

# JSON header
cat > "$OUTPUT_FILE" << EOF
{
  "benchmark_type": "quick_single_run",
  "timestamp": "$TIMESTAMP",
  "hardware": {
    "cpu": "$CPU",
    "memory_gb": $MEMORY_GB,
    "os": "$OS"
  },
  "operations": [
EOF

# Function to benchmark an operation
bench_op() {
    local op=$1
    local file=$2
    local name=$3

    echo "Benchmarking: $op on $name"

    # Get file size
    FILE_SIZE=$(stat -f%z "$file")
    FILE_SIZE_MB=$(echo "scale=2; $FILE_SIZE / 1024 / 1024" | bc)

    # Run with /usr/bin/time
    OUTPUT_DIR="/tmp/benchmark_$$"
    mkdir -p "$OUTPUT_DIR"

    START=$(date +%s.%N)
    MEM_OUTPUT=$(VIDEO_EXTRACT_THREADS=4 /usr/bin/time -l $BINARY performance --ops "$op" "$file" 2>&1)
    END=$(date +%s.%N)

    DURATION=$(echo "$END - $START" | bc)
    PEAK_MEM_BYTES=$(echo "$MEM_OUTPUT" | grep "maximum resident set size" | awk '{print $1}')
    PEAK_MEM_MB=$(echo "scale=2; $PEAK_MEM_BYTES / 1024 / 1024" | bc)
    THROUGHPUT_MBS=$(echo "scale=2; $FILE_SIZE_MB / $DURATION" | bc)

    # Check if operation succeeded
    if echo "$MEM_OUTPUT" | grep -q "Error:"; then
        echo "  ⚠️  Failed (may require specific input or model)"
        return 1
    fi

    echo "  Duration: ${DURATION}s, Memory: ${PEAK_MEM_MB} MB, Throughput: ${THROUGHPUT_MBS} MB/s"

    # Append to JSON
    cat >> "$OUTPUT_FILE" << EOF
    {
      "operation": "$op",
      "file": "$name",
      "file_size_mb": $FILE_SIZE_MB,
      "duration_sec": $DURATION,
      "peak_memory_mb": $PEAK_MEM_MB,
      "throughput_mb_per_sec": $THROUGHPUT_MBS
    },
EOF

    rm -rf "$OUTPUT_DIR"
    return 0
}

# Benchmark operations
echo "=== Core Extraction ==="
bench_op "metadata_extraction" "$VIDEO_MEDIUM" "video_medium" || true
bench_op "keyframes" "$VIDEO_MEDIUM" "video_medium" || true
bench_op "audio_extraction" "$VIDEO_MEDIUM" "video_medium" || true

echo ""
echo "=== Speech & Audio ==="
bench_op "transcription" "$AUDIO_SHORT" "audio_short" || true
bench_op "voice_activity_detection" "$AUDIO_MEDIUM" "audio_medium" || true
bench_op "audio_classification" "$AUDIO_MEDIUM" "audio_medium" || true

echo ""
echo "=== Vision Analysis ==="
bench_op "object_detection" "$IMAGE_MEDIUM" "image_medium" || true
bench_op "face_detection" "$IMAGE_MEDIUM" "image_medium" || true
bench_op "ocr" "$IMAGE_MEDIUM" "image_medium" || true

echo ""
echo "=== Intelligence & Content ==="
bench_op "image_quality_assessment" "$IMAGE_MEDIUM" "image_medium" || true
bench_op "smart_thumbnail" "$VIDEO_MEDIUM" "video_medium" || true
bench_op "scene_detection" "$VIDEO_MEDIUM" "video_medium" || true

echo ""
echo "=== Embeddings ==="
bench_op "vision_embeddings" "$IMAGE_MEDIUM" "image_medium" || true
bench_op "audio_embeddings" "$AUDIO_MEDIUM" "audio_medium" || true

echo ""
echo "=== Utility ==="
bench_op "duplicate_detection" "$VIDEO_SMALL" "video_small" || true
bench_op "format_conversion" "$VIDEO_SMALL" "video_small" || true

# Remove trailing comma and close JSON
sed -i '' '$ s/,$//' "$OUTPUT_FILE"
cat >> "$OUTPUT_FILE" << EOF

  ]
}
EOF

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Quick benchmark complete"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Completed: $(date)"
echo ""
echo "Results: $OUTPUT_FILE"
cat "$OUTPUT_FILE"
