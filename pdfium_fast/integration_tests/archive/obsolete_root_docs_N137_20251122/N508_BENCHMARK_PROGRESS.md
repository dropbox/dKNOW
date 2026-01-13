# N=508 Benchmark Progress

**Status**: IN PROGRESS (started N=507, continuing as N=508)

## Background

N=507 attempted to run full benchmark suite (N mod 13 = 0 regular benchmark cycle).
Two previous attempts (N=506, N=507 attempt 1) were interrupted after ~125-146 tests.

## Current Run (3rd Attempt)

**Started**: 2025-11-19T12:29:30Z (approximately)
**Background Task**: 7e74fd
**Command**: `python3 -m pytest -q 2>&1 | tee /tmp/pytest_output_txt`
**Output File**: `/tmp/pytest_output_txt`

**Progress** (as of 2025-11-19T12:42:00Z):
- Tests completed: ~1,155 (42%)
- Total expected: 2,760 tests
- Pace: ~1% per 3 minutes
- Expected completion: ~100 minutes total (~1h 40m)
- Expected finish time: ~2025-11-19T14:10:00Z

**System Health**:
- Load: 3.18 (healthy, < 6.0)
- Disk: 43% used (523GB free)
- No hung processes

## Next AI Instructions

1. Check if background task 7e74fd is still running:
   ```bash
   # From integration_tests directory
   tail -20 /tmp/pytest_output_txt
   ```

2. If completed, extract session ID:
   ```bash
   grep "passed.*in.*s" /tmp/pytest_output_txt
   # Or check telemetry:
   tail -5 telemetry/runs.csv
   ```

3. Analyze results vs N=443 baseline:
   - N=443 session: sess_20251119_070838_95545fc9
   - N=443 result: 2,757 passed, 2 env variance, 1 xfailed
   - Expected N=508: 2,759 passed, 1 xfailed (99.96% pass rate)

4. Update todos and commit final N=508 results

5. Next cycle: N=508+1 continues regular work or N=520 (next cleanup cycle)

## Test History

- **N=443**: Last successful full run (2,757 passed, 1 xfailed, 2 env variance)
- **N=506**: Interrupted after 146 tests at ~12:24:56Z
- **N=507 attempt 1**: Interrupted after 125 tests at ~12:28:19Z
- **N=508 (current)**: Running, 42% at 12:42:00Z

## Expected Outcomes

**Success**:
- 2,759 passed, 1 xfailed (99.96% pass rate)
- 0 new failures
- Deterministic (multiple runs identical)

**Acceptable variance**:
- ±2 tests due to environmental factors (as seen in N=443)
- Performance tests may show ±17% text, ±7% image variance

## Reference

- Baseline session: N=443 sess_20251119_070838_95545fc9
- Git commit (N=507): dbd11e6438f551833ea18fbf028ed79f5e3c68d5
- CLAUDE.md: N mod 13 = 0 benchmark protocol
