#!/usr/bin/env bash
#
# FFmpeg CLI vs video-extract Competitive Benchmarking
#
# Purpose: Prove whether video-extract is faster than FFmpeg CLI for keyframe extraction
# Usage: ./benchmarks/ffmpeg_comparison.sh
#
# Outputs:
# - benchmarks/results_keyframes_<timestamp>.csv
# - benchmarks/analysis_<timestamp>.md

set -euo pipefail

# Configuration
BENCHMARK_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$BENCHMARK_DIR")"
VIDEO_EXTRACT_BIN="$PROJECT_ROOT/target/release/video-extract"
RESULTS_DIR="$BENCHMARK_DIR/results"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
CSV_FILE="$RESULTS_DIR/keyframes_${TIMESTAMP}.csv"

# Test file selection (10 MP4 files from Kinetics dataset)
TEST_FILES=(
    "/Users/ayates/Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/Kinetics dataset (5%)/kinetics400_5per/kinetics400_5per/train/abseiling/-WKCwDRp_jk.mp4"
    "/Users/ayates/Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/Kinetics dataset (5%)/kinetics400_5per/kinetics400_5per/train/abseiling/1atkTs6LO6s.mp4"
    "/Users/ayates/Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/Kinetics dataset (5%)/kinetics400_5per/kinetics400_5per/train/abseiling/03NbvjuSoxk.mp4"
    "/Users/ayates/Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/Kinetics dataset (5%)/kinetics400_5per/kinetics400_5per/train/abseiling/60hYTVgs8EQ.mp4"
    "/Users/ayates/Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/Kinetics dataset (5%)/kinetics400_5per/kinetics400_5per/train/abseiling/51taNPfG89o.mp4"
    "/Users/ayates/Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/Kinetics dataset (5%)/kinetics400_5per/kinetics400_5per/train/air drumming/DNXNsDRCLtE.mp4"
    "/Users/ayates/Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/Kinetics dataset (5%)/kinetics400_5per/kinetics400_5per/train/air drumming/CD1DwNuzStM.mp4"
    "/Users/ayates/Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/Kinetics dataset (5%)/kinetics400_5per/kinetics400_5per/train/air drumming/5M80ZTWfzOU.mp4"
    "/Users/ayates/Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/Kinetics dataset (5%)/kinetics400_5per/kinetics400_5per/train/air drumming/CErkhE_nKZs.mp4"
    "/Users/ayates/Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/Kinetics dataset (5%)/kinetics400_5per/kinetics400_5per/train/air drumming/8RqNei3MH98.mp4"
)

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Setup
mkdir -p "$RESULTS_DIR"
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

echo "═══════════════════════════════════════════════════"
echo "  FFmpeg CLI vs video-extract Benchmark"
echo "═══════════════════════════════════════════════════"
echo ""
echo "Test files: ${#TEST_FILES[@]}"
echo "Output: $CSV_FILE"
echo ""

# CSV header
echo "file,size_mb,ffmpeg_time_s,ffmpeg_frames,video_extract_time_s,video_extract_frames,speedup" > "$CSV_FILE"

# Validate files before benchmarking
echo "Validating files (timeout 10s each)..."
VALID_FILES=()
VALID_INDICES=()
for i in "${!TEST_FILES[@]}"; do
    file="${TEST_FILES[$i]}"
    filename=$(basename "$file")

    if [[ ! -f "$file" ]]; then
        echo -e "${RED}✗ File not found: $filename${NC}"
        continue
    fi

    # Pre-validate with timeout to catch corrupted files (N=161-162 approach)
    if timeout 10 ffprobe -v error -show_entries format=duration -of default=noprint_wrappers=1:nokey=1 "$file" > /dev/null 2>&1; then
        echo -e "${GREEN}✓ Valid: $filename${NC}"
        VALID_FILES+=("$file")
        VALID_INDICES+=("$i")
    else
        echo -e "${RED}✗ Corrupted or timeout: $filename${NC}"
    fi
done

echo ""
echo "Valid files: ${#VALID_FILES[@]}/${#TEST_FILES[@]}"
echo ""

# Benchmark each valid file
for idx in "${!VALID_FILES[@]}"; do
    file="${VALID_FILES[$idx]}"
    i="${VALID_INDICES[$idx]}"
    filename=$(basename "$file")
    file_size_mb=$(stat -f%z "$file" | awk '{printf "%.2f", $1/1048576}')

    echo -e "${YELLOW}[$((idx+1))/${#VALID_FILES[@]}] Testing: $filename (${file_size_mb}MB)${NC}"

    # Test 1: FFmpeg CLI
    echo -n "  FFmpeg CLI...        "
    ffmpeg_output_dir="$TEMP_DIR/ffmpeg_$i"
    mkdir -p "$ffmpeg_output_dir"

    ffmpeg_start=$(date +%s.%N)
    if ffmpeg -hide_banner -loglevel error -i "$file" \
        -vf "select='eq(pict_type,I)'" -vsync vfr \
        "$ffmpeg_output_dir/frame%04d.jpg" > /dev/null 2>&1; then
        ffmpeg_end=$(date +%s.%N)
        ffmpeg_time=$(echo "$ffmpeg_end - $ffmpeg_start" | bc)
        ffmpeg_frames=$(find "$ffmpeg_output_dir" -name "frame*.jpg" | wc -l | tr -d ' ')
        echo -e "${GREEN}${ffmpeg_time}s (${ffmpeg_frames} frames)${NC}"
    else
        echo -e "${RED}FAILED${NC}"
        ffmpeg_time="ERROR"
        ffmpeg_frames="0"
    fi

    # Test 2: video-extract
    # Note: video-extract saves to /tmp/video-extract/keyframes/<file_id>/keyframes/*.jpg
    # regardless of --output-dir flag. We need to find the file ID and count frames there.
    echo -n "  video-extract...     "

    extract_start=$(date +%s.%N)
    if (cd "$PROJECT_ROOT" && "$VIDEO_EXTRACT_BIN" debug -o keyframes "$file" > /dev/null 2>&1); then
        extract_end=$(date +%s.%N)
        extract_time=$(echo "$extract_end - $extract_start" | bc)

        # Find the file ID by looking for newest directory in /tmp/video-extract/keyframes/
        # that contains keyframes subdirectory
        extract_file_id=""
        for dir in /tmp/video-extract/keyframes/*/; do
            if [[ -d "${dir}keyframes" ]]; then
                # Check if this directory was modified recently (within last minute)
                if [[ $(find "$dir" -type d -mmin -1 | wc -l) -gt 0 ]]; then
                    extract_file_id=$(basename "$dir")
                    break
                fi
            fi
        done

        if [[ -n "$extract_file_id" ]]; then
            extract_frames=$(find "/tmp/video-extract/keyframes/${extract_file_id}/keyframes" -name "*.jpg" 2>/dev/null | wc -l | tr -d ' ')
        else
            extract_frames="0"
        fi

        echo -e "${GREEN}${extract_time}s (${extract_frames} frames)${NC}"
    else
        echo -e "${RED}FAILED${NC}"
        extract_time="ERROR"
        extract_frames="0"
    fi

    # Calculate speedup
    if [[ "$ffmpeg_time" != "ERROR" && "$extract_time" != "ERROR" ]]; then
        speedup=$(echo "scale=2; $ffmpeg_time / $extract_time" | bc)
        if (( $(echo "$speedup > 1" | bc -l) )); then
            speedup_display="${GREEN}${speedup}x faster${NC}"
        elif (( $(echo "$speedup < 1" | bc -l) )); then
            speedup_inverse=$(echo "scale=2; 1 / $speedup" | bc)
            speedup_display="${RED}${speedup_inverse}x slower${NC}"
        else
            speedup_display="equal"
        fi
        echo -e "  Speedup:             $speedup_display"
    else
        speedup="ERROR"
    fi

    # Write CSV row
    echo "$filename,$file_size_mb,$ffmpeg_time,$ffmpeg_frames,$extract_time,$extract_frames,$speedup" >> "$CSV_FILE"

    echo ""
done

# Generate summary
echo "═══════════════════════════════════════════════════"
echo "  Summary"
echo "═══════════════════════════════════════════════════"
echo ""

# Calculate aggregate statistics
total_files=$(grep -v "^file," "$CSV_FILE" | grep -v "ERROR" | wc -l | tr -d ' ')
avg_speedup=$(awk -F',' 'NR>1 && $7!="ERROR" {sum+=$7; count++} END {if(count>0) printf "%.2f", sum/count}' "$CSV_FILE")
wins=$(awk -F',' 'NR>1 && $7>1 {count++} END {print count+0}' "$CSV_FILE")
losses=$(awk -F',' 'NR>1 && $7<1 && $7!="ERROR" {count++} END {print count+0}' "$CSV_FILE")

echo "Files tested:      $total_files"
echo "Average speedup:   ${avg_speedup}x"
echo "Wins (faster):     $wins"
echo "Losses (slower):   $losses"
echo ""
echo "Results saved to:  $CSV_FILE"
echo ""

if (( $(echo "$avg_speedup > 1" | bc -l) )); then
    echo -e "${GREEN}✓ video-extract is FASTER than FFmpeg CLI${NC}"
elif (( $(echo "$avg_speedup < 1" | bc -l) )); then
    echo -e "${RED}✗ video-extract is SLOWER than FFmpeg CLI${NC}"
else
    echo -e "${YELLOW}= video-extract has COMPARABLE performance to FFmpeg CLI${NC}"
fi
