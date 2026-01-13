#!/bin/bash
# Benchmark a single operation across multiple test files
# Usage: ./benchmark_operation.sh <operation> <file1> [file2] [file3] ...
#
# Example:
#   ./benchmark_operation.sh keyframes test_edge_cases/video_4k_ultra_hd_3840x2160__stress_test.mp4
#
# Output: JSON file with benchmark results in benchmarks/results/

set -euo pipefail

OPERATION=$1
shift
TEST_FILES=("$@")

if [ ${#TEST_FILES[@]} -eq 0 ]; then
    echo "Usage: $0 <operation> <file1> [file2] ..."
    exit 1
fi

# Ensure results directory exists
mkdir -p benchmarks/results

# Output file
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
OUTPUT_FILE="benchmarks/results/${OPERATION}_${TIMESTAMP}.json"

# Binary path
BINARY="./target/release/video-extract"

if [ ! -f "$BINARY" ]; then
    echo "Error: Binary not found at $BINARY"
    echo "Please build with: cargo build --release"
    exit 1
fi

echo "Benchmarking operation: $OPERATION"
echo "Test files: ${#TEST_FILES[@]}"
echo "Output: $OUTPUT_FILE"
echo ""

# JSON header
cat > "$OUTPUT_FILE" << EOF
{
  "operation": "$OPERATION",
  "timestamp": "$TIMESTAMP",
  "hardware": {
    "cpu": "$(sysctl -n machdep.cpu.brand_string)",
    "memory_gb": $(sysctl -n hw.memsize | awk '{print $1/1024/1024/1024}'),
    "os": "$(uname -s) $(uname -r)"
  },
  "results": [
EOF

FIRST=true

for TEST_FILE in "${TEST_FILES[@]}"; do
    if [ ! -f "$TEST_FILE" ]; then
        echo "Warning: File not found: $TEST_FILE (skipping)"
        continue
    fi

    echo "Benchmarking: $TEST_FILE"

    # Get file size
    FILE_SIZE=$(stat -f%z "$TEST_FILE" 2>/dev/null || stat -c%s "$TEST_FILE" 2>/dev/null)
    FILE_SIZE_MB=$(echo "scale=2; $FILE_SIZE / 1024 / 1024" | bc)

    # Run hyperfine benchmark (10 runs for latency percentiles)
    # Capture output to temporary file
    TEMP_FILE=$(mktemp)

    hyperfine \
        --warmup 2 \
        --runs 10 \
        --export-json "$TEMP_FILE" \
        --shell=none \
        "VIDEO_EXTRACT_THREADS=4 $BINARY debug --ops $OPERATION --output-dir /tmp/benchmark_output_$$ $TEST_FILE" \
        2>&1 | grep -E "Time|Range" || true

    # Extract metrics from hyperfine JSON
    MEAN=$(jq -r '.results[0].mean' "$TEMP_FILE")
    STDDEV=$(jq -r '.results[0].stddev' "$TEMP_FILE")
    MIN=$(jq -r '.results[0].min' "$TEMP_FILE")
    MAX=$(jq -r '.results[0].max' "$TEMP_FILE")
    MEDIAN=$(jq -r '.results[0].median' "$TEMP_FILE")

    # Calculate p95 and p99 approximations from times array
    # Note: hyperfine doesn't provide exact percentiles, so we approximate
    P95=$(jq -r '.results[0].times | sort | .[8]' "$TEMP_FILE") # 9th of 10 runs
    P99=$(jq -r '.results[0].times | sort | .[9]' "$TEMP_FILE") # 10th of 10 runs (max)

    # Measure peak memory with single run
    MEM_OUTPUT=$(/usr/bin/time -l $BINARY debug --ops $OPERATION --output-dir /tmp/benchmark_output_$$ "$TEST_FILE" 2>&1)
    PEAK_MEM_BYTES=$(echo "$MEM_OUTPUT" | grep "maximum resident set size" | awk '{print $1}')
    PEAK_MEM_MB=$(echo "scale=2; $PEAK_MEM_BYTES / 1024 / 1024" | bc)

    # Calculate throughput
    THROUGHPUT_MBS=$(echo "scale=2; $FILE_SIZE_MB / $MEAN" | bc)

    # Cleanup temp files
    rm -f "$TEMP_FILE"
    rm -rf /tmp/benchmark_output_$$

    # Append to JSON (with comma separator if not first)
    if [ "$FIRST" = false ]; then
        echo "," >> "$OUTPUT_FILE"
    fi
    FIRST=false

    cat >> "$OUTPUT_FILE" << EOF
    {
      "file": "$TEST_FILE",
      "file_size_mb": $FILE_SIZE_MB,
      "latency": {
        "mean": $MEAN,
        "median": $MEDIAN,
        "stddev": $STDDEV,
        "min": $MIN,
        "max": $MAX,
        "p95": $P95,
        "p99": $P99
      },
      "memory": {
        "peak_mb": $PEAK_MEM_MB
      },
      "throughput": {
        "mb_per_sec": $THROUGHPUT_MBS
      }
    }
EOF

    echo "  Mean: ${MEAN}s, P95: ${P95}s, Peak memory: ${PEAK_MEM_MB} MB, Throughput: ${THROUGHPUT_MBS} MB/s"
    echo ""
done

# JSON footer
cat >> "$OUTPUT_FILE" << EOF

  ]
}
EOF

echo "Benchmark complete: $OUTPUT_FILE"
