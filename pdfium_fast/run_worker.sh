#!/bin/bash
# run_worker.sh - Autonomous continuous worker with worker role support
#
# Usage:
#   ./run_worker.sh                                    # Standard worker
#   ./run_worker.sh "You are WORKER1. Focus on X."    # Worker with role
#
# Worker runs autonomously following CLAUDE.md protocol
# Parent Claude or human can provide hints via HINT.txt file
# All hints are logged to HINTS_HISTORY.log for tracking

# Exit on error, but handle signals gracefully
set -e
set -o pipefail

LOG_DIR="worker_logs"
SESSION_START=$(date +%Y%m%d_%H%M%S)
HINTS_LOG="HINTS_HISTORY.log"
PROGRESS_LOG="$LOG_DIR/worker_session_${SESSION_START}.log"
mkdir -p "$LOG_DIR"

# Get worker role from command-line argument (optional)
WORKER_ROLE="$1"

# Signal handler for graceful shutdown
cleanup() {
    local signal=$1
    echo "" | tee -a "$PROGRESS_LOG"
    echo "⚠️  Received signal: $signal at $(date)" | tee -a "$PROGRESS_LOG"
    echo "⚠️  Worker interrupted at iteration $iteration" | tee -a "$PROGRESS_LOG"
    echo "⚠️  Exiting gracefully..." | tee -a "$PROGRESS_LOG"
    exit 1
}

# Trap common signals
trap 'cleanup SIGINT' INT
trap 'cleanup SIGTERM' TERM
trap 'cleanup SIGHUP' HUP

# Log script start
echo "🚀 Worker script started at $(date)" | tee -a "$PROGRESS_LOG"
if [ -n "$WORKER_ROLE" ]; then
    echo "👷 Worker Role: $WORKER_ROLE" | tee -a "$PROGRESS_LOG"
fi

iteration=1
while true; do
    # Log iteration start
    echo "📝 Starting iteration $iteration" >> "$PROGRESS_LOG"

    echo ""
    echo "========================================"
    echo "=== Worker Iteration $iteration"
    echo "=== Started at $(date)"
    echo "========================================"
    echo ""

    # Build prompt
    PROMPT=""

    # Add worker role on first iteration only
    if [ -n "$WORKER_ROLE" ] && [ $iteration -eq 1 ]; then
        TIMESTAMP=$(date '+%Y-%m-%d %H:%M:%S')
        echo "[$TIMESTAMP] Worker Role: $WORKER_ROLE" >> "$HINTS_LOG"
        PROMPT="$WORKER_ROLE

"
        echo "👷 Applying worker role (first iteration only)"
        echo ""
    fi

    # Check for optional runtime hint (HINT.txt)
    if [ -f "HINT.txt" ]; then
        HINT=$(cat HINT.txt)
        TIMESTAMP=$(date '+%Y-%m-%d %H:%M:%S')

        # Log the hint to history
        echo "[$TIMESTAMP] Iteration $iteration: $HINT" >> "$HINTS_LOG"

        PROMPT="${PROMPT}$HINT

"
        rm HINT.txt  # Consume the hint (one-time use)
        echo "📝 Applied runtime hint: $HINT"
        echo "   (Logged to $HINTS_LOG)"
        echo ""
    fi

    # Add "continue" directive
    PROMPT="${PROMPT}continue"

    # Run Claude with streaming JSON output (real-time)
    # Raw JSON saved to .jsonl, pretty text to console via Python converter
    LOG_FILE="$LOG_DIR/worker_iter_${iteration}_$(date +%Y%m%d_%H%M%S).jsonl"

    echo "🤖 Launching Claude..." >> "$PROGRESS_LOG"

    # Disable errexit temporarily for pipeline (we want to check exit code manually)
    set +e
    claude --dangerously-skip-permissions -p "$PROMPT" \
        --permission-mode acceptEdits \
        --output-format stream-json \
        --verbose 2>&1 | tee "$LOG_FILE" | ./json_to_text.py
    exit_code=${PIPESTATUS[0]}
    set -e

    echo "✅ Pipeline completed with exit code $exit_code at $(date)" >> "$PROGRESS_LOG"

    echo ""
    echo "=== Worker Iteration $iteration completed ==="
    echo "=== Exit code: $exit_code ==="
    echo "=== Log saved to: $LOG_FILE ==="
    echo ""

    # Log error but continue (Ctrl-C still stops via signal handler)
    if [ $exit_code -ne 0 ]; then
        echo "⚠️  WARNING: Worker exited with error code $exit_code. Continuing to next iteration..." | tee -a "$PROGRESS_LOG"
        # Don't break - let the loop continue
    fi

    echo "✅ Iteration $iteration complete, preparing for next..." >> "$PROGRESS_LOG"

    iteration=$((iteration + 1))

    # Brief pause between iterations
    sleep 2
done

echo "" | tee -a "$PROGRESS_LOG"
echo "🏁 Worker loop completed after $((iteration - 1)) iterations" | tee -a "$PROGRESS_LOG"
echo "Review hints history: cat $HINTS_LOG"
echo "Review progress log: cat $PROGRESS_LOG"
