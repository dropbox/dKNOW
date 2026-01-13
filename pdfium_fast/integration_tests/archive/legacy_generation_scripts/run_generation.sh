#!/bin/bash
# Run baseline generation with progress monitoring
# For use by next AI

set -e

cd "$(dirname "$0")"

echo "Starting baseline generation..."
echo "PID will be saved to .generation_pid"
echo "Monitor progress with: ./monitor_generation.sh"

# Run with output to log file (not /dev/null - that breaks multiprocessing)
python lib/generate_expected_outputs.py --workers 4 >> generation_output.log 2>&1 &
PID=$!
echo $PID > .generation_pid

echo "Started PID: $PID"
echo "Log: generation_output.log"
echo "Monitor: tail -f generation_output.log"
