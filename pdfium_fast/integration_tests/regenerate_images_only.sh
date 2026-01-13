#!/bin/bash
# Regenerate ONLY image baselines (skip text - much faster)
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

BASELINE_IMAGES=baselines/upstream/images
RENDER_TOOL="../rust/target/release/examples/render_pages"
PDFIUM_LIB="../out/Optimized-Shared/libpdfium.dylib"

if [ ! -f "$RENDER_TOOL" ]; then
    echo "ERROR: Rust render_pages not found: $RENDER_TOOL"
    exit 1
fi

if [ ! -f "$PDFIUM_LIB" ]; then
    echo "ERROR: libpdfium.dylib not found: $PDFIUM_LIB"
    exit 1
fi

mkdir -p "$BASELINE_IMAGES"

echo "Regenerating image baselines only (452 PDFs)..."
echo "Using: $RENDER_TOOL"
echo ""

generate_image_baseline() {
    local pdf_path="$1"
    local pdf_name=$(basename "$pdf_path")
    local pdf_stem="${pdf_name%.pdf}"

    echo "Generating image baseline: $pdf_name"

    local temp_dir=$(mktemp -d)
    local md5_output=$(DYLD_LIBRARY_PATH="$(dirname "$PDFIUM_LIB")" \
        "$RENDER_TOOL" "$pdf_path" "$temp_dir" 4 300 --md5 2>/dev/null || {
        rm -rf "$temp_dir"
        echo "  ⚠ Failed to render (may be encrypted/corrupt)"
        echo "{}" > "$BASELINE_IMAGES/${pdf_stem}.json"
        return
    })

    local json_file="$BASELINE_IMAGES/${pdf_stem}.json"
    local json_content="{"
    local first=true

    while IFS= read -r line; do
        if [[ $line =~ MD5:page_([0-9]+)\.png:([a-f0-9]+) ]]; then
            local page_num=$((10#${BASH_REMATCH[1]}))
            local md5_hash="${BASH_REMATCH[2]}"

            if [ "$first" = true ]; then
                first=false
            else
                json_content+=","
            fi
            json_content+=$'\n'"  \"$page_num\": \"$md5_hash\""
        fi
    done <<< "$md5_output"

    json_content+=$'\n'"}"
    echo "$json_content" > "$json_file"
    rm -rf "$temp_dir"

    local page_count=$(grep -o '"[0-9]*"' "$json_file" | wc -l | tr -d ' ')
    echo "  ✓ $page_count pages"
}

total=0
for pdf in pdfs/benchmark/*.pdf pdfs/edge_cases/*.pdf; do
    if [ -f "$pdf" ]; then
        generate_image_baseline "$pdf"
        total=$((total + 1))
    fi
done

echo ""
echo "✓ Complete: $total PDFs processed"
echo "✓ Baselines saved to: $BASELINE_IMAGES/"
