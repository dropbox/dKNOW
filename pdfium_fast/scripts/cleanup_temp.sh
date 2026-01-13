#!/bin/bash
# Cleanup temporary rendering output
# Run this if disk fills up

echo "Cleaning temporary benchmark/test output..."

# Kill any hung pdfium_cli processes
echo "Checking for hung processes..."
ps aux | grep pdfium_cli | grep bug_451265 | awk '{print $2}' | xargs kill -9 2>/dev/null
echo "Killed hung processes (if any)"

# Remove temp output directories
echo "Removing /tmp output directories..."
rm -rf /tmp/*benchmark* /tmp/*optimized* /tmp/*baseline* /tmp/*test* /tmp/*quality* /tmp/*profile* /tmp/*bench* /tmp/out_* 2>/dev/null

# Check disk space
echo ""
echo "Disk space after cleanup:"
df -h / | tail -1

echo ""
echo "Remaining large /tmp directories:"
du -sh /tmp/* 2>/dev/null | sort -rh | head -10

echo ""
echo "✅ Cleanup complete"
