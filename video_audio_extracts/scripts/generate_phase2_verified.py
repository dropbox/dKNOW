#!/usr/bin/env python3
"""
Generate Phase 2 verification script with verified file paths.

Uses VERIFIED_TEST_FILES.json to select 30 test files that actually exist,
covering diverse formats and operations.
"""

import json
from pathlib import Path
from typing import Dict, List


def load_inventory() -> Dict:
    """Load verified test file inventory."""
    inventory_path = Path("docs/ai-verification/VERIFIED_TEST_FILES.json")
    with open(inventory_path) as f:
        return json.load(f)


def select_phase2_tests(inventory: Dict) -> List[Dict]:
    """
    Select 30 diverse tests for Phase 2.

    Priority operations:
    - face-detection
    - object-detection
    - ocr
    - emotion-detection
    - pose-estimation
    - transcription

    Formats: JPG (10), PNG (10), WebP (5), MP3/WAV (5)
    """
    tests = []

    # Target distribution
    target = {
        "jpg": {
            "face-detection": 2,
            "object-detection": 2,
            "ocr": 2,
            "emotion-detection": 2,
            "pose-estimation": 2,
        },
        "png": {
            "face-detection": 2,
            "object-detection": 2,
            "ocr": 2,
            "emotion-detection": 2,
            "pose-estimation": 2,
        },
        "webp": {
            "face-detection": 1,
            "object-detection": 2,
            "ocr": 1,
            "emotion-detection": 1,
        },
        "mp3": {
            "transcription": 2,
        },
        "wav": {
            "transcription": 3,
        },
    }

    test_num = 1

    for format_name, ops in target.items():
        if format_name not in inventory:
            print(f"Warning: {format_name} not in inventory")
            continue

        for operation, count in ops.items():
            if operation not in inventory[format_name]:
                print(f"Warning: {operation} not in {format_name} inventory")
                continue

            files = inventory[format_name][operation]
            selected = files[:count]

            for file_path in selected:
                test_name = f"{format_name}_{operation.replace('-', '_')}_{test_num}"
                tests.append({
                    "name": test_name,
                    "file": file_path,
                    "operation": operation,
                    "format": format_name,
                })
                test_num += 1

    return tests


def generate_script(tests: List[Dict], output_path: str):
    """Generate bash verification script."""

    script = '''#!/bin/bash
# Phase 2 AI Verification (Retry with Verified Paths)
# Generated from verified test file inventory
# Tests supported formats: JPG, PNG, WebP + Transcription (MP3, WAV)
#
# Usage:
#   export OPENAI_API_KEY="$(cat OPENAI_API_KEY.txt)"
#   bash scripts/verify_phase2_verified.sh

set -e

echo "=========================================="
echo "Phase 2 AI Verification (30 tests - Verified Paths)"
echo "JPG/PNG/WebP + Transcription"
echo "=========================================="
echo ""

# Check API key
if [ -z "$OPENAI_API_KEY" ]; then
    if [ -f "OPENAI_API_KEY.txt" ]; then
        export OPENAI_API_KEY="$(cat OPENAI_API_KEY.txt)"
        echo "Loaded API key from OPENAI_API_KEY.txt"
    else
        echo "ERROR: OPENAI_API_KEY not set and OPENAI_API_KEY.txt not found"
        exit 1
    fi
fi

# Create output directory
mkdir -p docs/ai-verification

# Output file
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
REPORT="docs/ai-verification/PHASE2_RETRY_GPT4_VERIFICATION_${TIMESTAMP}.csv"
echo "test_name,operation,input_file,status,confidence,findings" > "$REPORT"

TOTAL=30
CURRENT=0

# Function to verify
verify_test() {
    local test_name=$1
    local file=$2
    local op=$3

    CURRENT=$((CURRENT + 1))
    echo "[$CURRENT/$TOTAL] $test_name"

    if ! ./target/release/video-extract debug --ops "$op" "$file" >/dev/null 2>&1; then
        echo "  ❌ Binary failed"
        echo "\\"$test_name\\",\\"$op\\",\\"$file\\",\\"ERROR\\",\\"0.0\\",\\"Binary execution failed\\"" >> "$REPORT"
        return
    fi

    local output_file=""
    case "$op" in
        "face-detection") output_file="debug_output/stage_00_face_detection.json" ;;
        "object-detection") output_file="debug_output/stage_00_object_detection.json" ;;
        "ocr") output_file="debug_output/stage_00_ocr.json" ;;
        "transcription") output_file="debug_output/stage_00_transcription.json" ;;
        "emotion-detection") output_file="debug_output/stage_00_emotion_detection.json" ;;
        "pose-estimation") output_file="debug_output/stage_00_pose_estimation.json" ;;
    esac

    if [ ! -f "$output_file" ]; then
        echo "  ❌ Output not found"
        echo "\\"$test_name\\",\\"$op\\",\\"$file\\",\\"ERROR\\",\\"0.0\\",\\"Output not found\\"" >> "$REPORT"
        return
    fi

    local result
    if ! result=$(python3 scripts/ai_verify_openai.py "$file" "$output_file" "$op" 2>&1); then
        echo "  ❌ AI failed"
        echo "\\"$test_name\\",\\"$op\\",\\"$file\\",\\"ERROR\\",\\"0.0\\",\\"AI verification failed\\"" >> "$REPORT"
        return
    fi

    local status=$(echo "$result" | python3 -c "import sys, json; print(json.loads(sys.stdin.read()).get('status', 'UNKNOWN'))" 2>/dev/null || echo "ERROR")
    local conf=$(echo "$result" | python3 -c "import sys, json; print(json.loads(sys.stdin.read()).get('confidence', 0.0))" 2>/dev/null || echo "0.0")
    local find=$(echo "$result" | python3 -c "import sys, json; print(json.loads(sys.stdin.read()).get('findings', ''))" 2>/dev/null || echo "")
    find=$(echo "$find" | tr '\\n' ' ' | sed 's/"/""/g')

    case "$status" in
        "CORRECT") echo "  ✅ CORRECT ($conf)" ;;
        "SUSPICIOUS") echo "  ⚠️  SUSPICIOUS ($conf)" ;;
        "INCORRECT") echo "  ❌ INCORRECT ($conf)" ;;
        *) echo "  ❓ $status" ;;
    esac

    echo "\\"$test_name\\",\\"$op\\",\\"$file\\",\\"$status\\",\\"$conf\\",\\"$find\\"" >> "$REPORT"
}

# Tests with verified file paths
'''

    # Add verify_test calls (escape quotes in file paths for bash)
    for test in tests:
        file_escaped = test["file"].replace('"', '\\"')
        script += f'verify_test "{test["name"]}" "{file_escaped}" "{test["operation"]}"\n'

    script += '''
# Summary
echo ""
echo "=========================================="
echo "Results: $REPORT"
echo "=========================================="

CORRECT=$(grep -c "\\"CORRECT\\"" "$REPORT" || true)
SUSPICIOUS=$(grep -c "\\"SUSPICIOUS\\"" "$REPORT" || true)
INCORRECT=$(grep -c "\\"INCORRECT\\"" "$REPORT" || true)
ERROR=$(grep -c "\\"ERROR\\"" "$REPORT" || true)

echo "✅ CORRECT:    $CORRECT / $TOTAL"
echo "⚠️  SUSPICIOUS: $SUSPICIOUS / $TOTAL"
echo "❌ INCORRECT:  $INCORRECT / $TOTAL"
echo "❓ ERROR:      $ERROR / $TOTAL"
echo ""

if [ "$CORRECT" -ge 27 ]; then
    echo "✅ Verification PASSED (≥90% correct: $CORRECT/$TOTAL)"
elif [ "$CORRECT" -ge 24 ]; then
    echo "⚠️  Verification ACCEPTABLE (80-90% correct: $CORRECT/$TOTAL)"
else
    echo "❌ Verification NEEDS INVESTIGATION (<80% correct: $CORRECT/$TOTAL)"
fi
'''

    with open(output_path, "w") as f:
        f.write(script)

    # Make executable
    import os
    os.chmod(output_path, 0o755)


def main():
    print("Loading verified test file inventory...")
    inventory = load_inventory()

    print("Selecting 30 diverse tests for Phase 2...")
    tests = select_phase2_tests(inventory)

    print(f"\nSelected {len(tests)} tests:")
    for test in tests:
        print(f"  - {test['name']:40s} | {test['operation']:20s} | {test['file']}")

    print("\nGenerating verification script...")
    output_path = "scripts/verify_phase2_verified.sh"
    generate_script(tests, output_path)

    print(f"\n✅ Script generated: {output_path}")
    print("\nTo run:")
    print("  export OPENAI_API_KEY=\"$(cat OPENAI_API_KEY.txt)\"")
    print("  bash scripts/verify_phase2_verified.sh")


if __name__ == "__main__":
    main()
