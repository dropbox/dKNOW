#!/bin/bash
# Get TRUE state from actual codebase - no speculation

echo "=== FORMATS ===="
echo "Format directories in test_files_wikimedia:"
ls test_files_wikimedia/ | grep -v MANIFEST | sort | while read dir; do
    count=$(find "test_files_wikimedia/$dir" -type f 2>/dev/null | wc -l | tr -d ' ')
    echo "$dir: $count files"
done

echo ""
echo "=== PLUGINS ==="
echo "Plugin YAML files in config/plugins:"
ls config/plugins/*.yaml | wc -l
ls config/plugins/*.yaml | sed 's|config/plugins/||' | sed 's|\.yaml$||' | sort

echo ""
echo "=== TEST SUITES ==="
echo "Test files in tests/:"
ls tests/*.rs | grep -E "(test|smoke)" | while read f; do
    tests=$(grep -c "^fn test_\|^fn smoke_" "$f" 2>/dev/null || echo "0")
    echo "$f: $tests functions"
done

echo ""
echo "=== TEST FILE COUNTS ==="
find test_files_wikimedia -type f | wc -l
find test_edge_cases -type f 2>/dev/null | wc -l || echo "0"
find test_media_generated -type f 2>/dev/null | wc -l || echo "0"
