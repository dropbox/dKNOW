# Worker System - Continuous Claude with Parent Supervision

## Architecture

**Worker Claude (continuous):**
- Runs autonomously following CLAUDE.md protocol
- Each iteration runs `continue` command
- Can receive optional hints for guidance
- Makes git commits per N-based tracking

**Parent Claude (on-demand):**
- You invoke manually to check progress
- Reads logs, git commits, and code
- Provides analysis and suggests direction
- Can send hints to worker via HINT.txt

**You (human):**
- Watch worker output in real-time
- Invoke parent Claude when you want review
- Can send hints manually anytime
- Stop/restart worker for major changes

## Usage

### Start the Worker

```bash
cd ~/docling_rs
./run_worker.sh
```

Worker will run continuously until:
- You Ctrl+C to stop
- Worker encounters an error
- Worker completes all work and exits cleanly

### Send a Hint to Worker

From another terminal or during parent Claude session:

```bash
cd ~/docling_rs
echo "Focus on fixing table serialization bugs" > HINT.txt
```

Worker will pick up the hint at the **start of its next iteration**.

The hint is:
- Applied once (prepended to `continue` command)
- Logged to `HINTS_HISTORY.log` with timestamp
- Deleted after use (consumed)

### Invoke Parent Claude for Review

In another terminal:

```bash
cd ~/docling_rs
claude -p "Check on the worker progress and advise on next steps"
```

Parent Claude will:
1. Read recent logs from `worker_logs/`
2. Check git commits to see what was accomplished
3. Analyze current code state
4. Report findings to you
5. Optionally send a hint to worker (create HINT.txt)

### Review Hint History

```bash
cat HINTS_HISTORY.log
```

Example output:
```
[2025-10-23 09:15:32] Iteration 3: Focus on table serialization bugs
[2025-10-23 11:42:18] Iteration 8: Run the canonical test suite
[2025-10-23 14:05:44] Iteration 12: Write progress retrospective
```

### Review Worker Logs

```bash
# List all logs
ls -ltr worker_logs/

# View latest log
tail -f worker_logs/worker_iter_*.log

# Search logs for specific content
grep -r "error" worker_logs/
```

## Files

- `run_worker.sh` - Worker script (runs continuously)
- `HINT.txt` - Drop hint here (consumed on next iteration)
- `HINTS_HISTORY.log` - All hints sent, with timestamps
- `worker_logs/` - Logs from each worker iteration
- `CLAUDE.md` - Main protocol (worker follows this)

## Example Session

```bash
# Terminal 1: Start worker
./run_worker.sh

# Terminal 2: Monitor in real-time
tail -f worker_logs/worker_iter_*.log

# Terminal 3: Parent Claude check-in (15 minutes later)
claude -p "Review worker progress, check git commits, advise on next steps"

# Parent Claude might create HINT.txt after review:
echo "Run canonical tests and fix any failures" > HINT.txt

# Worker picks up hint on next iteration automatically
```

## When to Use Hints

**Good uses:**
- Focus on a specific area: "Focus on table serialization"
- Run tests: "Run the canonical test suite"
- Generate reports: "Write a progress retrospective"
- Change approach: "Try a different serialization strategy"

**Avoid:**
- Micro-management (let worker be autonomous)
- Multiple hints at once (send one, let it complete)
- Hints that conflict with CLAUDE.md protocol

## When to Stop Worker

Stop and restart when:
- Major direction change needed
- Worker is stuck in a loop
- You want to review before continuing
- Need to update CLAUDE.md protocol itself

Otherwise, let it run continuously for maximum progress.
