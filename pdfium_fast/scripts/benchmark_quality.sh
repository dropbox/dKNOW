#!/bin/bash
# N=230: Anti-aliasing quality benchmark
# Tests 20 diverse PDFs in balanced vs fast mode

set -e

PDFIUM_CLI="./out/Release/pdfium_cli"
PDF_LIST="/tmp/profiling_pdfs_20.json"
OUTPUT_DIR="/tmp/quality_benchmark_$$"
RESULTS_FILE="benchmark_quality_results_$$.txt"

mkdir -p "$OUTPUT_DIR/balanced" "$OUTPUT_DIR/fast"

echo "=== Anti-Aliasing Quality Benchmark ===" | tee "$RESULTS_FILE"
echo "Date: $(date -u +"%Y-%m-%dT%H:%M:%SZ")" | tee -a "$RESULTS_FILE"
echo "Binary: $PDFIUM_CLI" | tee -a "$RESULTS_FILE"
echo "PDFs: 20 diverse samples" | tee -a "$RESULTS_FILE"
echo "" | tee -a "$RESULTS_FILE"

# Extract PDF paths from JSON
PDF_PATHS=$(python3 -c "import json; data=json.load(open('$PDF_LIST')); print('\n'.join(data['selected_pdfs']))")

# Benchmark function
benchmark_pdf() {
    local pdf_path="$1"
    local quality_mode="$2"
    local output_subdir="$3"
    local pdf_name=$(basename "$pdf_path")

    echo "Testing: $pdf_name (quality=$quality_mode)" | tee -a "$RESULTS_FILE"

    # Run pdfium_cli and capture timing output
    local start_time=$(date +%s.%N)
    local output=$("$PDFIUM_CLI" --quality "$quality_mode" render-pages "$pdf_path" "$OUTPUT_DIR/$output_subdir/" 2>&1 || true)
    local end_time=$(date +%s.%N)
    local duration=$(echo "$end_time - $start_time" | bc)

    # Extract timing data from CLI output (render/encode/write breakdown)
    echo "  Duration: ${duration}s" | tee -a "$RESULTS_FILE"
    echo "$output" | grep -E "(Page [0-9]+:|render=|encode=|write=)" | tee -a "$RESULTS_FILE"
    echo "" | tee -a "$RESULTS_FILE"
}

echo "=== Phase 1: Balanced Mode (Baseline) ===" | tee -a "$RESULTS_FILE"
echo "" | tee -a "$RESULTS_FILE"

for pdf in $PDF_PATHS; do
    if [ -f "$pdf" ]; then
        benchmark_pdf "$pdf" "balanced" "balanced"
    else
        echo "WARNING: $pdf not found, skipping" | tee -a "$RESULTS_FILE"
    fi
done

echo "=== Phase 2: Fast Mode (AA Disabled) ===" | tee -a "$RESULTS_FILE"
echo "" | tee -a "$RESULTS_FILE"

for pdf in $PDF_PATHS; do
    if [ -f "$pdf" ]; then
        benchmark_pdf "$pdf" "fast" "fast"
    else
        echo "WARNING: $pdf not found, skipping" | tee -a "$RESULTS_FILE"
    fi
done

echo "=== Benchmark Complete ===" | tee -a "$RESULTS_FILE"
echo "Results saved to: $RESULTS_FILE" | tee -a "$RESULTS_FILE"
echo "Output images: $OUTPUT_DIR/{balanced,fast}/" | tee -a "$RESULTS_FILE"

# Keep output directory for visual quality comparison
echo "" | tee -a "$RESULTS_FILE"
echo "To compare visual quality, examine images in:" | tee -a "$RESULTS_FILE"
echo "  $OUTPUT_DIR/balanced/" | tee -a "$RESULTS_FILE"
echo "  $OUTPUT_DIR/fast/" | tee -a "$RESULTS_FILE"
