# Cron Setup for Worker Manager

## Overview

The manager script (`check_worker.sh`) can run periodically via cron to monitor the worker and take actions automatically.

## Manager Modes

### Monitor Mode (default)
```bash
./check_worker.sh
```
- Checks worker status
- Writes report
- **No actions taken** - just observes

### Autonomous Mode
```bash
./check_worker.sh --auto
```
- Checks worker status
- Writes report
- **Can take actions:**
  - Restart worker if stopped
  - Send hints if stuck
  - Log all actions to `worker_reports/manager_actions.log`

## Cron Setup

### Option 1: Check Every 15 Minutes (Monitor Only)

```bash
# Edit crontab
crontab -e

# Add this line:
*/15 * * * * cd /Users/ayates/docling_rs && ./check_worker.sh >> worker_reports/cron.log 2>&1
```

**What happens:**
- Manager checks worker every 15 minutes
- Writes reports to `worker_reports/worker_status_*.md`
- You review reports and take action manually

### Option 2: Check Every 30 Minutes (Autonomous)

```bash
# Edit crontab
crontab -e

# Add this line:
*/30 * * * * cd /Users/ayates/docling_rs && ./check_worker.sh --auto >> worker_reports/cron.log 2>&1
```

**What happens:**
- Manager checks worker every 30 minutes
- Writes reports to `worker_reports/worker_status_*.md`
- **Automatically restarts worker** if stopped
- **Sends hints** if worker needs redirection
- Logs actions to `worker_reports/manager_actions.log`

### Option 3: Daytime Only (Business Hours)

```bash
# Check every 20 minutes, Monday-Friday, 9am-6pm
*/20 9-18 * * 1-5 cd /Users/ayates/docling_rs && ./check_worker.sh --auto >> worker_reports/cron.log 2>&1
```

## Viewing Reports

### Latest Report
```bash
cat worker_reports/latest_report.md
```

### All Reports
```bash
ls -lt worker_reports/worker_status_*.md | head -10
```

### Action Log (Autonomous Mode)
```bash
cat worker_reports/manager_actions.log
```

### Cron Log
```bash
tail -50 worker_reports/cron.log
```

## Monitoring the Manager

### Check Cron Status
```bash
# List your cron jobs
crontab -l

# Check cron is running
ps aux | grep cron
```

### Test Manager Manually
```bash
# Test monitor mode
./check_worker.sh

# Test autonomous mode (be careful - will restart worker!)
./check_worker.sh --auto
```

## Safety Notes

### Autonomous Mode Risks

**Manager will:**
- ✅ Restart worker if it stopped cleanly
- ✅ Send hints based on analysis
- ✅ Log all actions

**Manager will NOT:**
- ❌ Kill running worker processes (unless you extend the script)
- ❌ Modify code
- ❌ Push to git

### Rate Limiting

Don't run manager too frequently:
- **Every 15 min** = Good for monitoring
- **Every 30 min** = Good for autonomous mode
- **Every 5 min** = Too frequent (wastes tokens)

## Stopping Everything

### Stop Worker
```bash
pkill -f "claude.*continue"
```

### Disable Cron
```bash
# Edit crontab
crontab -e

# Comment out or delete the line
# */30 * * * * cd /Users/ayates/docling_rs && ./check_worker.sh --auto
```

## Example Workflow

**Day 1, 9am:** Start worker manually
```bash
./run_worker.sh
```

**Day 1, 9am:** Set up autonomous manager cron
```bash
crontab -e
# Add: */30 * * * * cd /Users/ayates/docling_rs && ./check_worker.sh --auto >> worker_reports/cron.log 2>&1
```

**Day 1-7:** Manager runs every 30 min:
- Worker making progress → Manager just reports
- Worker stopped → Manager restarts it
- Worker stuck → Manager sends hint

**Day 7:** You come back, review reports:
```bash
# See latest status
cat worker_reports/latest_report.md

# See all actions taken
cat worker_reports/manager_actions.log

# See git progress
git log --oneline --since="7 days ago"
```

Perfect for **long-running autonomous work** while you're away!
