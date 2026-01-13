#!/bin/bash
# check_worker.sh - Manager script to monitor worker and generate reports
#
# Can be run manually or via cron
# Claude will check worker state, analyze progress, and write a report
#
# Usage:
#   ./check_worker.sh              # Monitor mode (default) - report only
#   ./check_worker.sh --auto       # Autonomous mode - can take actions
#
# Autonomous mode allows manager to:
#   - Restart worker if stopped
#   - Send hints to redirect work
#   - Kill stuck worker processes

set -e

# Configuration
AUTO_MODE=false
if [[ "$1" == "--auto" ]]; then
    AUTO_MODE=true
fi

REPORT_DIR="worker_reports"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
REPORT_FILE="$REPORT_DIR/worker_status_$TIMESTAMP.md"
ACTION_LOG="$REPORT_DIR/manager_actions.log"

# Create report directory
mkdir -p "$REPORT_DIR"

echo "=== Worker Manager Check - $TIMESTAMP ==="
echo "Starting Claude manager session..."

# Run Claude in non-interactive mode to check worker
# Claude will:
# 1. Read recent worker logs
# 2. Check git commits since last check
# 3. Analyze current progress and test status
# 4. Write detailed report to $REPORT_FILE
# 5. Optionally create HINT.txt if intervention needed
# 6. Exit automatically when done

claude --print "
You are the manager checking on the worker Claude.

**Mode:** $(if $AUTO_MODE; then echo 'AUTONOMOUS - You can take actions'; else echo 'MONITOR ONLY - Report only, no actions'; fi)

## Your Task

1. **Check worker logs**: Read recent files from worker_logs/ (last 1-3 iterations)
2. **Check git history**: See what commits were made since last manager check
3. **Read last manager report**: Check $REPORT_DIR/latest_report.md to see what was recommended last time
4. **Check test status**: Understand current pass/fail state if mentioned in logs
5. **Check if worker is running**: Use 'ps aux | grep claude.*continue' to see if worker is active
6. **Analyze progress**: Is worker making progress? Stuck? Completed work?
7. **Write report**: Create comprehensive report at $REPORT_FILE
8. **Take action (if --auto mode)**: Based on analysis, you may:
   - Create HINT.txt to redirect worker
   - Start worker if stopped: cd /Users/ayates/docling_rs && nohup ./run_worker.sh > /dev/null 2>&1 &
   - Log all actions to $ACTION_LOG

## Autonomous Mode Decision Rules

**If worker is STOPPED and progress was good:**
- Action: Restart worker (run ./run_worker.sh in background)
- Log: Echo action to $ACTION_LOG with timestamp

**If worker is RUNNING but seems stuck (same commit for >2 hours):**
- Action: Create HINT.txt with guidance based on last commit message
- Log: Echo hint sent to $ACTION_LOG

**If worker needs direction change:**
- Action: Create HINT.txt with specific guidance
- Log: Echo hint to $ACTION_LOG

**If everything looks good:**
- Action: None needed, just report status

## Report Format

Write a report to $REPORT_FILE in this format:

\`\`\`markdown
# Worker Status Report

**Date:** [current date/time]
**Mode:** [AUTONOMOUS or MONITOR ONLY]

## Status
- Worker: [Running/Stopped/Unknown]
- Last Activity: [timestamp of last commit or log]
- Last Commit: [N=X: brief title]
- Time Since Last Commit: [hours/minutes]

## Progress Since Last Check
[What worker accomplished - list recent commits with N numbers]

## Test Status
[If available: X/Y tests passing, key failures]

## Analysis
[Is worker making good progress? Any issues? Stuck on anything?]

## Actions Taken (Autonomous Mode Only)
[List any actions taken: worker restarted, hint sent, etc. Or 'None']

## Recommendation
[For user: What should happen next]
\`\`\`

Be concise but thorough. This report is for the human user.

IMPORTANT: If in autonomous mode and you take actions (restart worker, send hint), you MUST log them to $ACTION_LOG like this:
echo \"[\$(date)] ACTION: [description]\" >> $ACTION_LOG
" \
--permission-mode acceptEdits \
--dangerously-skip-permissions \
2>&1 | tee "$REPORT_DIR/manager_log_$TIMESTAMP.txt"

exit_code=${PIPESTATUS[0]}

echo ""
echo "=== Manager Check Complete ==="
echo "Exit code: $exit_code"
echo "Report should be at: $REPORT_FILE"
echo ""

# Create symlink to latest report for easy access
cd "$REPORT_DIR"
ln -sf "worker_status_$TIMESTAMP.md" latest_report.md
cd ..

echo "View report: cat $REPORT_DIR/latest_report.md"

exit $exit_code
