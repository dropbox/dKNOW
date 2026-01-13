"""
Telemetry System - Automatic CSV Logging

Logs every test run with comprehensive metadata:
- Temporal: timestamp, duration, run number
- Git: commit, branch, dirty status
- Test: id, name, category, level, result
- PDF: name, pages, size
- System: CPU, RAM, load, temp
- Binary: MD5, timestamp, path
- Performance: pps, speedup
- Validation: edit distance, pixel diff
"""

import csv
import json
import uuid
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Any
from filelock import FileLock


class TelemetrySystem:
    """Centralized telemetry collection and logging."""

    def __init__(self, telemetry_dir: Path):
        self.telemetry_dir = Path(telemetry_dir)
        self.csv_path = self.telemetry_dir / 'runs.csv'
        self.history_dir = self.telemetry_dir / 'history'
        self.lock_path = self.telemetry_dir / 'runs.csv.lock'

        # Ensure directories exist
        self.telemetry_dir.mkdir(parents=True, exist_ok=True)
        self.history_dir.mkdir(parents=True, exist_ok=True)

        # Session ID (unique per pytest session)
        self.session_id = self._generate_session_id()

        # Run counter
        self.run_counter = self._get_next_run_number()

    def _generate_session_id(self) -> str:
        """Generate unique session ID."""
        timestamp = datetime.utcnow().strftime('%Y%m%d_%H%M%S')
        unique = uuid.uuid4().hex[:8]
        return f"sess_{timestamp}_{unique}"

    def _get_next_run_number(self) -> int:
        """Get next run number from CSV."""
        if not self.csv_path.exists():
            return 1

        try:
            with open(self.csv_path, 'r') as f:
                reader = csv.DictReader(f)
                rows = list(reader)
                if rows:
                    max_run = max(int(row.get('run_number', 0)) for row in rows if row.get('run_number', '').isdigit())
                    return max_run + 1
        except:
            pass

        return 1

    def log_test_run(self, data: Dict[str, Any]):
        """
        Log test run to CSV with file locking for concurrent pytest runs.

        Args:
            data: Dictionary with all telemetry fields
        """
        # Add session/run metadata
        data['session_id'] = self.session_id
        data['run_number'] = self.run_counter
        data['run_id'] = f"run_{data['timestamp'].replace(':', '').replace('-', '').replace('.', '').replace('Z', '')}_{data['run_number']}"

        self.run_counter += 1

        # Ensure all fields are strings for CSV
        data_clean = {k: self._serialize_value(v) for k, v in data.items()}

        # Thread-safe CSV append with file lock
        lock = FileLock(str(self.lock_path), timeout=10)
        with lock:
            file_exists = self.csv_path.exists()

            with open(self.csv_path, 'a', newline='') as f:
                # Get all possible fields (union of existing + new)
                if file_exists and self.csv_path.stat().st_size > 0:
                    # Read existing headers
                    with open(self.csv_path, 'r') as rf:
                        existing_fields = csv.DictReader(rf).fieldnames or []
                    all_fields = list(dict.fromkeys(list(existing_fields) + list(data_clean.keys())))
                else:
                    all_fields = self._get_standard_fields()

                writer = csv.DictWriter(f, fieldnames=all_fields, extrasaction='ignore')

                # Write header if new file
                if not file_exists or self.csv_path.stat().st_size == 0:
                    writer.writeheader()

                # Write data row
                writer.writerow(data_clean)

        # Also save detailed JSON
        json_path = self.history_dir / f"{data['run_id']}.json"
        with open(json_path, 'w') as f:
            json.dump(data, f, indent=2, default=str)

    def _serialize_value(self, value: Any) -> str:
        """Convert value to CSV-safe string."""
        if value is None:
            return ''
        elif isinstance(value, bool):
            return 'true' if value else 'false'
        elif isinstance(value, (int, float)):
            return str(value)
        elif isinstance(value, (list, dict)):
            return json.dumps(value)
        else:
            return str(value)

    def _get_standard_fields(self) -> List[str]:
        """Standard CSV field order."""
        return [
            # Temporal
            'timestamp', 'run_id', 'run_number', 'session_id', 'duration_sec',

            # Git
            'git_commit_hash', 'git_commit_short', 'git_branch', 'git_dirty',
            'git_timestamp', 'git_author',

            # Test Identity
            'test_id', 'test_file', 'test_name', 'test_function',
            'test_category', 'test_level', 'test_type', 'test_pdf_count',

            # Test Result
            'result', 'passed', 'failed', 'skipped',
            'error_message', 'error_type',

            # PDF
            'pdf_name', 'pdf_path', 'pdf_pages', 'pdf_size_mb', 'pdf_category',

            # Execution
            'worker_count', 'iteration_number', 'thread_count_actual',

            # Validation (Text)
            'text_edit_distance', 'text_edit_distance_relative', 'text_similarity',
            'text_char_diff', 'text_line_diff',
            'jsonl_field_errors', 'jsonl_mismatch_count',

            # Validation (Image)
            'md5_match', 'md5_expected', 'md5_actual',
            'pixel_diff_count', 'pixel_diff_pct', 'image_hash',

            # Performance
            'pages_per_sec', 'speedup_vs_1w', 'total_pages', 'total_time_sec',
            'throughput_mb_per_sec',

            # Performance details (1w baseline + all workers)
            'perf_1w_pps', 'perf_1w_duration',
            'perf_2w_speedup', 'perf_8w_speedup',

            # System Hardware
            'cpu_model', 'cpu_cores_physical', 'cpu_cores_logical',
            'cpu_freq_mhz', 'cpu_temp_c',
            'ram_total_gb', 'ram_used_gb', 'ram_free_gb', 'ram_percent',

            # System State
            'load_avg_1m', 'load_avg_5m', 'load_avg_15m',
            'swap_used_gb', 'swap_percent',
            'disk_total_gb', 'disk_used_gb', 'disk_free_gb', 'disk_percent',

            # Binary
            'binary_path', 'binary_md5', 'binary_size_mb', 'binary_timestamp',
            'build_config',

            # Environment
            'python_version', 'pytest_version', 'platform', 'platform_release',
            'platform_version', 'machine_id', 'dyld_library_path',

            # LLM
            'llm_enabled', 'llm_called', 'llm_model', 'llm_cost_usd',
            'llm_tokens_input', 'llm_tokens_output',
        ]


def extract_test_metadata(docstring: str) -> Dict[str, str]:
    """
    Extract structured metadata from test docstring.

    Expected format:
        META:
          id: test_001
          category: correctness
          level: smoke
          ...

    Returns dict of metadata fields.
    """
    if not docstring:
        return {}

    metadata = {}
    in_meta_section = False

    for line in docstring.split('\n'):
        line = line.strip()

        if line == 'META:':
            in_meta_section = True
            continue

        if in_meta_section:
            # End of META section
            if line and not line.startswith(' ') and ':' not in line:
                break

            # Parse "key: value"
            if ':' in line:
                key, value = line.split(':', 1)
                key = key.strip()
                value = value.strip()

                # Handle lists
                if value.startswith('[') and value.endswith(']'):
                    value = value[1:-1].split(',')
                    value = [v.strip() for v in value]

                metadata[key] = value

    return metadata
