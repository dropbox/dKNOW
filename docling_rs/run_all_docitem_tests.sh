#!/bin/bash

# Run all 49 DocItem tests and capture scores
# Output: CSV file with format,score,status

# Load OpenAI API key from .env file (gitignored)
source .env
export PATH="$HOME/.cargo/bin:$PATH"

OUTPUT_FILE="/tmp/docitem_test_results.csv"
echo "Format,Score,Status,Issues" > "$OUTPUT_FILE"

TESTS=(
    "docx" "csv" "pptx" "xlsx" "html" "markdown" "asciidoc" "jats" "webvtt"
    "png" "jpeg" "tiff" "webp" "bmp"
    "zip" "tar" "eml" "mbox" "epub"
    "odt" "ods" "odp" "rtf" "gif"
    "svg" "7z" "rar" "vcf" "ics"
    "fb2" "mobi" "gpx" "kml" "tex"
    "kmz" "doc" "vsdx" "mpp" "pages"
    "srt" "ipynb" "stl" "obj" "dxf"
    "gltf" "glb" "heif" "avif" "dicom"
)

for format in "${TESTS[@]}"; do
    echo "Testing $format..."

    output=$(cargo test -p docling-core --test llm_docitem_validation_tests test_llm_docitem_$format -- --exact --nocapture 2>&1)

    # Extract score
    score=$(echo "$output" | grep "Overall Score:" | sed 's/.*Overall Score: \([0-9.]*\)%.*/\1/')

    # Extract status
    if echo "$output" | grep -q "test.*ok"; then
        status="PASS"
    else
        status="FAIL"
    fi

    # Extract first issue line
    issues=$(echo "$output" | grep "DocItem Gaps:" -A1 | tail -1 | sed 's/^[[:space:]]*//')

    echo "$format,$score,$status,\"$issues\"" >> "$OUTPUT_FILE"
done

echo "Results saved to $OUTPUT_FILE"
cat "$OUTPUT_FILE"
