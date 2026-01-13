#!/bin/bash
# Analyze LLM score improvements from N=1021 to N=1379
# Compares baseline scores to current test run

echo "========================================="
echo "LLM Score Improvement Analysis"
echo "Baseline: N=1021 (2025-11-15)"
echo "Current: N=1379 (2025-11-18)"
echo "========================================="
echo ""

# N=1021 Baseline Scores (from reports/feature/phase-e-open-standards/N1021_comprehensive_llm_test_results_2025-11-15.md)
declare -A baseline_scores=(
    # Verification tests
    ["csv"]="100"
    ["html"]="98"
    ["markdown"]="97"
    ["xlsx"]="98"
    ["asciidoc"]="97"
    ["docx"]="100"
    ["pptx"]="98"
    ["webvtt"]="100"
    ["jats"]="98"

    # Mode3 tests - Archives
    ["zip"]="90"
    ["tar"]="84"
    ["7z"]="85"
    ["rar"]="85"

    # Mode3 tests - Email/Contact
    ["eml"]="92"
    ["mbox"]="95"
    ["vcf"]="90"

    # Mode3 tests - Ebooks
    ["epub"]="79"
    ["fb2"]="87"
    ["mobi"]="85"

    # Mode3 tests - OpenDocument
    ["odt"]="75"
    ["ods"]="85"
    ["odp"]="77"

    # Mode3 tests - Specialized
    ["ics"]="92"
    ["ipynb"]="92"

    # Mode3 tests - GPS/GIS (ARCHITECTURALLY VIOLATED - expect improvement)
    ["gpx"]="87"
    ["kml"]="90"
    ["kmz"]="93"

    # Mode3 tests - Images (ARCHITECTURALLY VIOLATED - expect improvement)
    ["bmp"]="88"
    ["gif"]="88"
    ["heif"]="82"
    ["avif"]="82"
    ["svg"]="85"
    ["dicom"]="87"

    # Mode3 tests - CAD (ARCHITECTURALLY VIOLATED - expect improvement)
    ["stl"]="85"
    ["obj"]="90"
    ["gltf"]="85"
    ["glb"]="95"
    ["dxf"]="63"
)

# Find most recent comprehensive results file
RESULTS_FILE=$(ls -t llm_comprehensive_results_*.txt 2>/dev/null | head -1)

if [ -z "$RESULTS_FILE" ]; then
    echo "❌ ERROR: No comprehensive results file found"
    echo "Expected: llm_comprehensive_results_*.txt"
    exit 1
fi

echo "Reading results from: $RESULTS_FILE"
echo ""

# Parse current scores from results file
declare -A current_scores
declare -A improvements
declare -A regressions

TOTAL_FORMATS=0
IMPROVED_COUNT=0
REGRESSED_COUNT=0
UNCHANGED_COUNT=0
TOTAL_IMPROVEMENT=0

for format in "${!baseline_scores[@]}"; do
    baseline=${baseline_scores[$format]}

    # Extract score from results file (look for "Overall Score: XX.X%")
    score_line=$(grep -A 20 "mode3_$format\|verification_$format" "$RESULTS_FILE" | grep "Overall Score:" | head -1)

    if [ -n "$score_line" ]; then
        current=$(echo "$score_line" | sed -E 's/.*Overall Score: ([0-9]+).*/\1/')
        current_scores[$format]=$current

        diff=$((current - baseline))

        TOTAL_FORMATS=$((TOTAL_FORMATS + 1))

        if [ $diff -gt 0 ]; then
            improvements[$format]=$diff
            IMPROVED_COUNT=$((IMPROVED_COUNT + 1))
            TOTAL_IMPROVEMENT=$((TOTAL_IMPROVEMENT + diff))
        elif [ $diff -lt 0 ]; then
            regressions[$format]=$diff
            REGRESSED_COUNT=$((REGRESSED_COUNT + 1))
        else
            UNCHANGED_COUNT=$((UNCHANGED_COUNT + 1))
        fi
    fi
done

# Report improvements
echo "========================================="
echo "IMPROVEMENTS (N=1021 → N=1379)"
echo "========================================="
echo ""

if [ ${#improvements[@]} -eq 0 ]; then
    echo "No improvements detected"
else
    for format in "${!improvements[@]}"; do
        baseline=${baseline_scores[$format]}
        current=${current_scores[$format]}
        diff=${improvements[$format]}
        echo "✅ ${format^^}: ${baseline}% → ${current}% (+${diff}%)"
    done
fi

echo ""

# Report regressions
echo "========================================="
echo "REGRESSIONS (N=1021 → N=1379)"
echo "========================================="
echo ""

if [ ${#regressions[@]} -eq 0 ]; then
    echo "No regressions detected"
else
    for format in "${!regressions[@]}"; do
        baseline=${baseline_scores[$format]}
        current=${current_scores[$format]}
        diff=${regressions[$format]}
        echo "❌ ${format^^}: ${baseline}% → ${current}% (${diff}%)"
    done
fi

echo ""

# Summary statistics
echo "========================================="
echo "SUMMARY STATISTICS"
echo "========================================="
echo ""
echo "Total Formats Tested: $TOTAL_FORMATS"
echo "Improved: $IMPROVED_COUNT ($((IMPROVED_COUNT * 100 / TOTAL_FORMATS))%)"
echo "Regressed: $REGRESSED_COUNT ($((REGRESSED_COUNT * 100 / TOTAL_FORMATS))%)"
echo "Unchanged: $UNCHANGED_COUNT ($((UNCHANGED_COUNT * 100 / TOTAL_FORMATS))%)"
echo ""
echo "Total Improvement: +${TOTAL_IMPROVEMENT} percentage points"
echo "Average Improvement: +$((TOTAL_IMPROVEMENT / IMPROVED_COUNT)) percentage points per improved format"
echo ""

# Architectural violation fixes analysis
echo "========================================="
echo "ARCHITECTURAL VIOLATION FIX IMPACT"
echo "========================================="
echo ""
echo "These formats had architectural violations (markdown→DocItems pattern):"
echo "Expected to show improvement after fixes in N=1367-1378"
echo ""

ARCH_FORMATS=("gpx" "kml" "kmz" "bmp" "gif" "heif" "avif" "svg" "dicom" "stl" "obj" "gltf" "glb" "dxf")
ARCH_IMPROVED=0
ARCH_TOTAL_IMPROVEMENT=0

for format in "${ARCH_FORMATS[@]}"; do
    if [ -n "${improvements[$format]}" ]; then
        baseline=${baseline_scores[$format]}
        current=${current_scores[$format]}
        diff=${improvements[$format]}
        echo "✅ ${format^^}: ${baseline}% → ${current}% (+${diff}%)"
        ARCH_IMPROVED=$((ARCH_IMPROVED + 1))
        ARCH_TOTAL_IMPROVEMENT=$((ARCH_TOTAL_IMPROVEMENT + diff))
    elif [ -n "${current_scores[$format]}" ]; then
        baseline=${baseline_scores[$format]}
        current=${current_scores[$format]}
        echo "   ${format^^}: ${baseline}% → ${current}% (no change)"
    fi
done

echo ""
echo "Architectural Fix Impact:"
echo "- Formats with violations: ${#ARCH_FORMATS[@]}"
echo "- Formats improved: $ARCH_IMPROVED"
echo "- Total improvement: +${ARCH_TOTAL_IMPROVEMENT} percentage points"
if [ $ARCH_IMPROVED -gt 0 ]; then
    echo "- Average improvement: +$((ARCH_TOTAL_IMPROVEMENT / ARCH_IMPROVED)) percentage points"
fi
echo ""

# Final verdict
echo "========================================="
echo "VERDICT"
echo "========================================="
echo ""

if [ $IMPROVED_COUNT -gt $REGRESSED_COUNT ] && [ $ARCH_IMPROVED -gt 0 ]; then
    echo "✅ ARCHITECTURAL FIXES SUCCESSFUL"
    echo "   - More formats improved than regressed"
    echo "   - Architectural violations fixed showed measurable improvement"
    echo "   - Quality scores restored or improved"
elif [ $IMPROVED_COUNT -eq 0 ] && [ $REGRESSED_COUNT -eq 0 ]; then
    echo "⚠️  NO CHANGES DETECTED"
    echo "   - All scores unchanged from N=1021"
    echo "   - Architectural fixes may not have impacted LLM scores"
    echo "   - Consider investigating why scores didn't improve"
else
    echo "⚠️  MIXED RESULTS"
    echo "   - Some improvements, some regressions"
    echo "   - Review individual format changes"
fi
