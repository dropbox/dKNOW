#!/bin/bash
#
# Generate Expected Outcomes (Baselines)
#
# Runs UPSTREAM PDFium to generate ground truth outputs for all PDFs.
# These baselines are the expected outcomes that tests compare against.
#
# Usage:
#   ./generate_baselines.sh                    # Generate all
#   ./generate_baselines.sh --pdf arxiv_001.pdf  # Single PDF
#   ./generate_baselines.sh --check             # Check what's missing
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Upstream PDFium binary (official, unmodified)
# Use existing build if it exists, otherwise use pdfium-official
if [ -f "$(dirname "$SCRIPT_DIR")/out/Optimized-Shared/pdfium_test" ]; then
    UPSTREAM_BIN="$(dirname "$SCRIPT_DIR")/out/Optimized-Shared/pdfium_test"
elif [ -f ~/pdfium-official/out/Release/pdfium_test ]; then
    UPSTREAM_BIN=~/pdfium-official/out/Release/pdfium_test
else
    echo "ERROR: No pdfium_test binary found"
    echo "Expected locations:"
    echo "  - $(dirname "$SCRIPT_DIR")/out/Optimized-Shared/pdfium_test"
    echo "  - ~/pdfium-official/out/Release/pdfium_test"
    exit 1
fi

echo "Using PDFium binary: $UPSTREAM_BIN"

# Baseline directories
BASELINE_TEXT=baselines/upstream/text
BASELINE_IMAGES=baselines/upstream/images

# Create directories
mkdir -p "$BASELINE_TEXT" "$BASELINE_IMAGES"

# ============================================================================
# Functions
# ============================================================================

generate_text_baseline() {
    local pdf_path="$1"
    local pdf_name=$(basename "$pdf_path")
    local pdf_stem="${pdf_name%.pdf}"

    echo "Generating text baseline: $pdf_name"

    # Run upstream PDFium to extract text
    if [ ! -f "$UPSTREAM_BIN" ]; then
        echo "  ✗ Upstream binary not found: $UPSTREAM_BIN"
        echo "  Build it first or check path"
        exit 1
    fi

    # Create temp directory for per-page text files
    local temp_dir=$(mktemp -d)
    local original_dir=$(pwd)

    # Copy PDF to temp dir (pdfium_test creates output files in current dir)
    cp "$pdf_path" "$temp_dir/"
    cd "$temp_dir"

    # Extract text (creates <pdf-name>.pdf.<page>.txt files)
    "$UPSTREAM_BIN" --txt "$pdf_name" 2>/dev/null || {
        cd "$original_dir"
        rm -rf "$temp_dir"
        echo "  ⚠ Failed to extract text (may be encrypted/corrupt - OK for edge cases)"
        touch "$BASELINE_TEXT/${pdf_stem}.txt"  # Create empty file as marker
        return
    }

    # Concatenate all page text files in order
    local output_file="$original_dir/$BASELINE_TEXT/${pdf_stem}.txt"
    > "$output_file"  # Create/clear output file

    # Find all generated text files and sort numerically by page number
    for txt_file in "${pdf_name}".*.txt; do
        if [ -f "$txt_file" ]; then
            cat "$txt_file" >> "$output_file"
        fi
    done

    # Clean up temp directory
    cd "$original_dir"
    rm -rf "$temp_dir"

    # Compute MD5 hash of text content (macOS uses 'md5', Linux uses 'md5sum')
    if [ -s "$BASELINE_TEXT/${pdf_stem}.txt" ]; then
        if command -v md5 &> /dev/null; then
            md5 -q "$BASELINE_TEXT/${pdf_stem}.txt" > "$BASELINE_TEXT/${pdf_stem}.txt.md5"
        elif command -v md5sum &> /dev/null; then
            md5sum "$BASELINE_TEXT/${pdf_stem}.txt" | awk '{print $1}' > "$BASELINE_TEXT/${pdf_stem}.txt.md5"
        fi
        echo "  ✓ Text: $(wc -c < "$BASELINE_TEXT/${pdf_stem}.txt") bytes"
        echo "  ✓ MD5: $(cat "$BASELINE_TEXT/${pdf_stem}.txt.md5")"
    else
        echo "  ⚠ No text extracted (0 bytes)"
    fi
}

generate_image_baseline() {
    local pdf_path="$1"
    local pdf_name=$(basename "$pdf_path")
    local pdf_stem="${pdf_name%.pdf}"

    echo "Generating image baseline: $pdf_name"

    # Use Rust render_pages tool for consistent PNG encoding
    local RENDER_TOOL="$(dirname "$SCRIPT_DIR")/rust/target/release/examples/render_pages"
    local PDFIUM_LIB="$(dirname "$SCRIPT_DIR")/out/Optimized-Shared/libpdfium.dylib"

    if [ ! -f "$RENDER_TOOL" ]; then
        echo "  ✗ Rust render_pages not found: $RENDER_TOOL"
        echo "  Build it first: cd rust && cargo build --release --examples"
        exit 1
    fi

    if [ ! -f "$PDFIUM_LIB" ]; then
        echo "  ✗ libpdfium.dylib not found: $PDFIUM_LIB"
        echo "  Build it first: gn gen out/Optimized-Shared && ninja -C out/Optimized-Shared pdfium"
        exit 1
    fi

    # Create temp directory for render_pages (output_dir required but not used with --md5)
    local temp_dir=$(mktemp -d)

    # Set library path and render with MD5 hashes
    # render_pages <pdf> <output_dir> <workers> <dpi> --md5
    # NOTE: Use 4 workers (matches upstream PDFium output exactly; 1-worker has rendering bugs)
    local md5_output=$(DYLD_LIBRARY_PATH="$(dirname "$PDFIUM_LIB")" \
        "$RENDER_TOOL" "$pdf_path" "$temp_dir" 4 300 --md5 2>/dev/null || {
        rm -rf "$temp_dir"
        echo "  ⚠ Failed to render images (may be encrypted/corrupt - OK for edge cases)"
        echo "{}" > "$BASELINE_IMAGES/${pdf_stem}.json"  # Create empty JSON as marker
        return
    })

    # Parse MD5 output and create JSON mapping
    # Format: {"0": "md5hash1", "1": "md5hash2", ...}
    # MD5 output format from render_pages: MD5:page_0000.png:<md5_hash>
    local json_file="$BASELINE_IMAGES/${pdf_stem}.json"

    # Start JSON object
    local json_content="{"
    local first=true

    while IFS= read -r line; do
        # Extract page number and MD5 from line like: "MD5:page_0000.png:abc123..."
        if [[ $line =~ MD5:page_([0-9]+)\.png:([a-f0-9]+) ]]; then
            # Convert page number with leading zeros (e.g., "0000") to plain number
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

    # Clean up temp directory
    rm -rf "$temp_dir"

    # Report results
    local page_count=$(grep -o '"[0-9]*"' "$json_file" | wc -l | tr -d ' ')
    if [ "$page_count" -gt 0 ]; then
        echo "  ✓ Images: $page_count pages rendered"
        echo "  ✓ Baselines: $json_file"
    else
        echo "  ⚠ No images rendered (0 pages)"
    fi
}

check_missing_baselines() {
    echo "Checking for missing baselines..."
    echo ""

    local missing_text=0
    local missing_images=0
    local total_pdfs=0

    for pdf in pdfs/benchmark/*.pdf pdfs/edge_cases/*.pdf resources/*.pdf; do
        if [ -f "$pdf" ]; then
            total_pdfs=$((total_pdfs + 1))
            pdf_name=$(basename "$pdf")
            pdf_stem="${pdf_name%.pdf}"

            # Check text baseline
            if [ ! -f "$BASELINE_TEXT/${pdf_stem}.txt" ] || [ ! -s "$BASELINE_TEXT/${pdf_stem}.txt" ]; then
                missing_text=$((missing_text + 1))
            fi

            # Check image baseline
            if [ ! -f "$BASELINE_IMAGES/${pdf_stem}.json" ] || [ ! -s "$BASELINE_IMAGES/${pdf_stem}.json" ]; then
                missing_images=$((missing_images + 1))
            fi
        fi
    done

    echo "PDF Coverage:"
    echo "  Total PDFs: $total_pdfs"
    echo "  Text baselines: $((total_pdfs - missing_text))/$total_pdfs"
    echo "  Image baselines: $((total_pdfs - missing_images))/$total_pdfs"
    echo ""
    echo "Missing:"
    echo "  Text: $missing_text"
    echo "  Images: $missing_images"
}

# ============================================================================
# Main
# ============================================================================

case "${1:-all}" in
    --check)
        check_missing_baselines
        ;;

    --pdf)
        if [ -z "$2" ]; then
            echo "Usage: $0 --pdf <pdf_name>"
            exit 1
        fi

        pdf_path="pdfs/$2"
        if [ ! -f "$pdf_path" ]; then
            pdf_path="pdfs/benchmark/$2"
        fi
        if [ ! -f "$pdf_path" ]; then
            pdf_path="pdfs/edge_cases/$2"
        fi
        if [ ! -f "$pdf_path" ]; then
            pdf_path="resources/$2"
        fi

        if [ ! -f "$pdf_path" ]; then
            echo "PDF not found: $2"
            echo "Searched in: pdfs/, pdfs/benchmark/, pdfs/edge_cases/, resources/"
            exit 1
        fi

        generate_text_baseline "$pdf_path"
        generate_image_baseline "$pdf_path"
        ;;

    all)
        echo "Generating baselines for all PDFs..."
        echo "This will take 1-2 hours for 450 PDFs."
        echo ""

        total=0
        # Benchmark PDFs
        for pdf in pdfs/benchmark/*.pdf; do
            if [ -f "$pdf" ]; then
                generate_text_baseline "$pdf"
                generate_image_baseline "$pdf"
                total=$((total + 1))
            fi
        done

        # Edge case PDFs
        for pdf in pdfs/edge_cases/*.pdf; do
            if [ -f "$pdf" ]; then
                generate_text_baseline "$pdf"
                generate_image_baseline "$pdf"
                total=$((total + 1))
            fi
        done

        # Resource PDFs (if any)
        for pdf in resources/*.pdf; do
            if [ -f "$pdf" ]; then
                generate_text_baseline "$pdf"
                generate_image_baseline "$pdf"
                total=$((total + 1))
            fi
        done

        echo ""
        echo "✓ Generated baselines for $total PDFs"
        echo ""
        check_missing_baselines
        ;;

    *)
        echo "Usage:"
        echo "  $0                 # Generate all baselines"
        echo "  $0 --pdf <name>    # Generate for single PDF"
        echo "  $0 --check         # Check what's missing"
        ;;
esac
