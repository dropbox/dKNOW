#!/bin/bash
# Monitor test progress
while true; do
  clear
  echo "=== Test Progress Monitor ==="
  date
  echo ""
  
  if pgrep -f "pytest.*complete_test_results_v1.4_N48" > /dev/null; then
    echo "STATUS: Tests running"
    echo ""
    tail -5 complete_test_results_v1.4_N48.txt 2>/dev/null | grep -E "PASSED|FAILED|%"
    echo ""
    wc -l complete_test_results_v1.4_N48.txt 2>/dev/null
  else
    echo "STATUS: Tests completed or not running"
    echo ""
    tail -20 complete_test_results_v1.4_N48.txt 2>/dev/null
    break
  fi
  
  sleep 300  # Check every 5 minutes
done
