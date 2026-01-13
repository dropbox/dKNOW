#!/bin/bash
# Baseline performance benchmarking for PDFium
# WORKER0 N=1 - 2025-10-31

export DYLD_LIBRARY_PATH="out/Optimized-Shared"

echo "=== TEXT EXTRACTION BENCHMARK ==="
echo ""

# Test on 100-page PDF
PDF="integration_tests/pdfs/benchmark/0100pages_7FKQLKX273JBHXAAW5XDRT27JGMIZMCI.pdf"
echo "Testing: $(basename $PDF) (100 pages)"
echo "Command: pdfium_test --txt"
echo ""

# Run 3 times and average
total=0
for i in 1 2 3; do
    start=$(date +%s.%N)
    out/Optimized-Shared/pdfium_test --txt "$PDF" > /dev/null 2>&1
    end=$(date +%s.%N)
    elapsed=$(echo "$end - $start" | bc)
    pps=$(echo "scale=2; 100 / $elapsed" | bc)
    echo "Run $i: ${elapsed}s (${pps} pages/sec)"
    total=$(echo "$total + $elapsed" | bc)
    rm "$PDF".*.txt 2>/dev/null || true
done

avg=$(echo "scale=3; $total / 3" | bc)
avg_pps=$(echo "scale=2; 100 / $avg" | bc)
echo ""
echo "Average: ${avg}s (${avg_pps} pages/sec)"
echo ""

echo "=== IMAGE RENDERING BENCHMARK ==="
echo ""

# Test rendering first 10 pages
echo "Testing: $(basename $PDF) (10 pages)"
echo "Command: pdfium_test --png --pages=0-9"
echo ""

# Run 3 times and average
total=0
for i in 1 2 3; do
    start=$(date +%s.%N)
    out/Optimized-Shared/pdfium_test --png --pages=0-9 "$PDF" > /dev/null 2>&1
    end=$(date +%s.%N)
    elapsed=$(echo "$end - $start" | bc)
    pps=$(echo "scale=2; 10 / $elapsed" | bc)
    echo "Run $i: ${elapsed}s (${pps} pages/sec)"
    total=$(echo "$total + $elapsed" | bc)
    rm "$PDF".*.png 2>/dev/null || true
done

avg=$(echo "scale=3; $total / 3" | bc)
avg_pps=$(echo "scale=2; 10 / $avg" | bc)
echo ""
echo "Average: ${avg}s (${avg_pps} pages/sec)"
