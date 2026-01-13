# Library Modules

**6 Python modules providing test infrastructure.**

---

## Modules

### telemetry.py (7.7KB)
**Purpose:** Automatic CSV logging system

**Key classes:**
- `TelemetrySystem` - Manages CSV append, file locking, session tracking

**What it does:**
- Captures every test run automatically (via conftest.py hooks)
- Logs to `testing/telemetry/runs.csv` with 91 fields
- Thread-safe CSV append (file locking for parallel pytest)
- Generates run_id, session_id, run_number
- Saves detailed JSON per run to history/

**Used by:** conftest.py (automatic, no code changes needed)

---

### validation.py (5.6KB)
**Purpose:** Validation methods beyond simple diff

**Functions:**
- `calculate_edit_distance(str1, str2)` - Levenshtein distance
- `calculate_similarity(str1, str2)` - Similarity ratio (0.0-1.0)
- `calculate_image_md5(image_path)` - MD5 hash of image
- `compare_images_pixel_level(exp, act)` - Pixel-by-pixel comparison
- `analyze_text_with_llm(exp, act, diff)` - OpenAI text error analysis
- `analyze_image_with_llm(exp, act)` - OpenAI Vision image analysis

**Used by:** All test files

---

### system_info.py (6.9KB)
**Purpose:** Collect comprehensive system metadata

**Functions:**
- `get_cpu_info()` - Model, cores, frequency
- `get_cpu_temp()` - Temperature (if available)
- `get_load_avg()` - System load (1m, 5m, 15m)
- `get_memory_info()` - RAM usage, swap
- `get_disk_info()` - Disk space
- `get_git_info()` - Commit, branch, dirty status, author
- `get_binary_info()` - MD5, size, timestamp
- `get_all_system_info()` - Complete system snapshot

**Used by:** conftest.py (collected once per session)

---

### baselines.py (7.4KB)
**Purpose:** Manage expected outcomes (upstream PDFium outputs)

**Key class:**
- `BaselineManager` - Load/save baselines with MD5 verification

**Functions:**
- `load_text_baseline(pdf_name)` - Load expected text (with MD5 check)
- `load_image_baseline(pdf_name)` - Load expected MD5 hashes per page
- `save_text_baseline(pdf_name, text)` - Save text + MD5
- `save_image_baseline(pdf_name, hashes)` - Save page hashes
- `count_baselines()` - Statistics
- `list_missing_baselines()` - Check what needs generation

**Storage:**
- `baselines/upstream/text/*.txt` + `*.txt.md5`
- `baselines/upstream/images/*.json`

**Used by:** test_002, generate_baselines.sh

---

### timeseries.py (19KB)
**Purpose:** Time series analysis and reporting

**Key class:**
- `TimeSeriesAnalyzer` - Analyze telemetry CSV over time

**Functions:**
- `generate_performance_timeseries()` - Extract time series data
- `generate_aggregate_stats()` - Group by test/PDF/worker
- `generate_trend_analysis()` - Detect trends (improving/regressing/stable)
- `detect_regressions()` - Auto-detect performance regressions
- `compare_git_commits()` - Performance comparison
- `export_timeseries_csv()` - Export filtered data
- `export_summary_json()` - Aggregate statistics
- `generate_markdown_report()` - Human-readable report

**Used by:** conftest.py for `pytest --stats` commands

---

### statistics.py (6.6KB)
**Purpose:** Statistical analysis on telemetry data

**Key class:**
- `StatisticsAnalyzer` - Basic statistics and regression detection

**Functions:**
- `show_trends()` - Display performance trends
- `check_regression()` - Alert on performance drops
- `generate_report()` - Summary statistics

**Used by:** conftest.py for `pytest --stats` commands

---

## Dependencies

```python
# Core
pytest, pandas, psutil, filelock

# Optional
openai  # LLM analysis
Pillow  # Pixel comparison
numpy   # Image arrays
```

---

## Import Example

```python
# In tests
import sys
sys.path.insert(0, 'lib')

import validation
from baselines import BaselineManager

# Use
edit_dist = validation.calculate_edit_distance(text1, text2)
baseline = BaselineManager(Path('testing')).load_text_baseline('arxiv_001.pdf')
```

---

## Summary

6 modules, ~54KB total code

**Core modules** (must keep):
- telemetry.py - CSV logging
- validation.py - Edit distance, LLM
- system_info.py - System metadata
- baselines.py - Upstream comparison

**Optional modules** (can delete):
- timeseries.py - Convenience (can use pandas directly)
- statistics.py - Convenience (can analyze CSV manually)
