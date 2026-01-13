# Telemetry Directory

**Auto-generated test run data. Created automatically by pytest.**

---

## Files

### runs.csv
**Purpose:** Complete log of all test runs

**Schema:** 91 fields per row:
- Temporal: timestamp, run_number, session_id, duration
- Git: commit, branch, dirty, author
- Test: test_id, category, level, result
- PDF: name, pages, size
- Validation: edit_distance, similarity, pixel_diff
- Performance: perf_1w_pps, pages_per_sec, speedup_vs_1w
- System: CPU, RAM, load, temp
- Binary: MD5, timestamp, path
- LLM: enabled, called, cost

**Created:** Automatically on first test run
**Updated:** Appended after every test (thread-safe with file locking)
**Size:** Grows over time (~1KB per test)

**Example:**
```csv
timestamp,test_id,pdf_name,worker_count,pages_per_sec,speedup_vs_1w,...
2025-10-31T06:00:00Z,smoke_text_001,arxiv_001.pdf,4,87.3,2.1,...
```

---

### history/
**Purpose:** Detailed JSON per test run

**Contents:** One .json file per test run with complete data

**Filename format:** `run_YYYYMMDDTHHMMSS_N.json`

**Example:**
```json
{
  "timestamp": "2025-10-31T06:00:00Z",
  "test_id": "smoke_text_001",
  "pdf_name": "arxiv_001.pdf",
  "result": "passed",
  "duration_sec": 0.603,
  "pages_per_sec": 87.3,
  "speedup_vs_1w": 2.1,
  "git_commit_short": "8addf31",
  "cpu_model": "Apple M3 Max",
  "load_avg_1m": 6.2,
  ...
}
```

**Purpose:** Detailed debugging, can reconstruct CSV if needed

---

### statistics/
**Purpose:** Generated reports and exports

**Contents:** (created by `pytest --stats` commands)
- `timeseries_Ndays.csv` - Filtered time series data
- `summary_Ndays.json` - Aggregate statistics
- `report_Ndays.md` - Markdown reports
- `*.png` - Performance plots (if matplotlib installed)

**Example usage:**
```bash
pytest --stats show-trends
# Creates: statistics/timeseries_30days.csv
#          statistics/summary_30days.json
#          statistics/report_30days.md
```

---

## How It Works

1. **Test runs:** `pytest -m smoke`
2. **Pytest hook:** conftest.py captures test data automatically
3. **CSV append:** telemetry.py writes to runs.csv (thread-safe)
4. **JSON save:** Detailed data saved to history/
5. **Analysis:** Use `pytest --stats` or analyze CSV directly

**Zero manual logging - completely automatic!**

---

## Analysis Examples

### pandas
```python
import pandas as pd

df = pd.read_csv('telemetry/runs.csv')

# Performance over time
df.groupby('timestamp')['pages_per_sec'].mean().plot()

# By worker count
df.groupby('worker_count')['pages_per_sec'].mean()

# Regression detection
df.sort_values('timestamp').tail(10)['speedup_vs_1w'].mean()
```

### pytest --stats
```bash
pytest --stats show-trends        # Performance trends
pytest --stats check-regression   # Auto-detect regressions
pytest --stats report             # Generate summary
pytest --stats query              # Interactive pandas shell
```

---

## CSV Fields (91 total)

**Temporal (5):** timestamp, run_id, run_number, session_id, duration_sec

**Git (6):** commit_hash, commit_short, branch, dirty, timestamp, author

**Test (8):** test_id, test_file, test_name, test_function, category, level, type, pdf_count

**Result (5):** result, passed, failed, skipped, error_message

**PDF (5):** pdf_name, pdf_path, pdf_pages, pdf_size_mb, pdf_category

**Execution (3):** worker_count, iteration_number, thread_count_actual

**Validation Text (7):** text_edit_distance, text_edit_distance_relative, text_similarity, text_char_diff, text_line_diff, jsonl_field_errors, jsonl_mismatch_count

**Validation Image (5):** md5_match, md5_expected, md5_actual, pixel_diff_count, pixel_diff_pct

**Performance (9):** pages_per_sec, speedup_vs_1w, total_pages, total_time_sec, throughput_mb_per_sec, perf_1w_pps, perf_1w_duration, perf_2w_speedup, perf_8w_speedup

**System Hardware (5):** cpu_model, cpu_cores_physical, cpu_cores_logical, cpu_freq_mhz, cpu_temp_c

**System State (9):** ram_total_gb, ram_used_gb, ram_free_gb, ram_percent, load_avg_1m, load_avg_5m, load_avg_15m, swap_used_gb, swap_percent, disk_free_gb, disk_percent

**Binary (4):** binary_path, binary_md5, binary_size_mb, binary_timestamp

**Environment (6):** python_version, pytest_version, platform, platform_release, platform_version, machine_id, dyld_library_path

**LLM (5):** llm_enabled, llm_called, llm_model, llm_cost_usd, llm_tokens

---

## Summary

**Automatic telemetry system** - no code changes needed in tests.

Every test run captured with 91 fields covering:
- What ran (test, PDF)
- How it performed (timing, speedup)
- What happened (result, validation)
- When it ran (timestamp, git commit)
- Where it ran (system, binary)

**Complete historical record for performance tracking and debugging.**
