#!/bin/bash
# Simple benchmark script for a single operation
# Does not require hyperfine - uses shell timing and /usr/bin/time
# Usage: ./benchmark_operation_simple.sh <operation> <file1> [file2] [file3] ...

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

    # Run 10 times for latency statistics
    TIMES=()
    echo "  Running 10 iterations..."

    for i in {1..10}; do
        START=$(date +%s.%N)
        VIDEO_EXTRACT_THREADS=4 $BINARY debug --ops $OPERATION --output-dir /tmp/benchmark_output_$$ "$TEST_FILE" > /dev/null 2>&1
        END=$(date +%s.%N)
        ELAPSED=$(echo "$END - $START" | bc)
        TIMES+=($ELAPSED)
        echo -n "."
    done
    echo ""

    # Calculate statistics
    # Sort times for percentiles
    SORTED_TIMES=($(printf '%s\n' "${TIMES[@]}" | sort -n))

    MIN=${SORTED_TIMES[0]}
    MAX=${SORTED_TIMES[9]}
    MEDIAN=$(echo "(${SORTED_TIMES[4]} + ${SORTED_TIMES[5]}) / 2" | bc -l)
    P95=${SORTED_TIMES[9]}  # 10th of 10 = max
    P99=${SORTED_TIMES[9]}  # Same as max for 10 samples

    # Calculate mean
    SUM=0
    for TIME in "${TIMES[@]}"; do
        SUM=$(echo "$SUM + $TIME" | bc)
    done
    MEAN=$(echo "scale=6; $SUM / 10" | bc)

    # Calculate standard deviation
    VARIANCE_SUM=0
    for TIME in "${TIMES[@]}"; do
        DIFF=$(echo "$TIME - $MEAN" | bc)
        DIFF_SQ=$(echo "$DIFF * $DIFF" | bc)
        VARIANCE_SUM=$(echo "$VARIANCE_SUM + $DIFF_SQ" | bc)
    done
    VARIANCE=$(echo "scale=6; $VARIANCE_SUM / 10" | bc)
    STDDEV=$(echo "scale=6; sqrt($VARIANCE)" | bc)

    # Measure peak memory with single run using /usr/bin/time
    MEM_OUTPUT=$(/usr/bin/time -l $BINARY debug --ops $OPERATION --output-dir /tmp/benchmark_output_$$ "$TEST_FILE" 2>&1)
    PEAK_MEM_BYTES=$(echo "$MEM_OUTPUT" | grep "maximum resident set size" | awk '{print $1}')
    PEAK_MEM_MB=$(echo "scale=2; $PEAK_MEM_BYTES / 1024 / 1024" | bc)

    # Calculate throughput
    THROUGHPUT_MBS=$(echo "scale=2; $FILE_SIZE_MB / $MEAN" | bc)

    # Cleanup temp files
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

    echo "  Mean: ${MEAN}s, Median: ${MEDIAN}s, P95: ${P95}s, Peak memory: ${PEAK_MEM_MB} MB, Throughput: ${THROUGHPUT_MBS} MB/s"
    echo ""
done

# JSON footer
cat >> "$OUTPUT_FILE" << EOF

  ]
}
EOF

echo "Benchmark complete: $OUTPUT_FILE"
echo ""
echo "Summary:"
jq -r '.results[] | "  \(.file | split("/")[-1]): \(.latency.mean)s mean, \(.throughput.mb_per_sec) MB/s, \(.memory.peak_mb) MB peak"' "$OUTPUT_FILE"
