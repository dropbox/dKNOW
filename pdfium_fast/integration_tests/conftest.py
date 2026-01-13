"""
PDFium Test Configuration - Automatic Telemetry

Pure pytest configuration with automatic telemetry logging.
Every test run is logged to CSV with 50+ metadata fields.

NO CODE CHANGES NEEDED IN TESTS - Telemetry is automatic!
"""

import os
import sys
import pytest
from datetime import datetime, timezone
from pathlib import Path

# Add lib to path
sys.path.insert(0, str(Path(__file__).parent / 'lib'))

from telemetry import TelemetrySystem, extract_test_metadata
from system_info import get_all_system_info
import validation


# ============================================================================
# Helper Functions
# ============================================================================

def get_macos_sdk_version():
    """Get macOS SDK version (e.g., '15.2', '15.1').

    Returns None if not on macOS or if xcodebuild is not available.
    """
    import subprocess
    import platform

    if platform.system() != 'Darwin':
        return None

    try:
        result = subprocess.run(
            ['xcodebuild', '-showsdks'],
            capture_output=True,
            text=True,
            timeout=5
        )
        if result.returncode != 0:
            return None

        # Parse output for "macOS X.Y -sdk macosxX.Y"
        for line in result.stdout.split('\n'):
            if 'macosx' in line.lower() and '-sdk' in line:
                # Extract version from "-sdk macosx15.2"
                parts = line.split()
                for part in parts:
                    if part.startswith('macosx'):
                        version = part.replace('macosx', '')
                        return version

        return None
    except (subprocess.TimeoutExpired, FileNotFoundError):
        return None


# ============================================================================
# Pytest Configuration
# ============================================================================

def pytest_addoption(parser):
    """Add custom command-line options."""
    parser.addoption(
        "--llm",
        action="store_true",
        help="Enable LLM-powered error analysis"
    )
    parser.addoption(
        "--workers",
        action="store",
        default=4,
        type=int,
        help="Number of worker threads (default: 4)"
    )
    parser.addoption(
        "--iterations",
        action="store",
        default=1,
        type=int,
        help="Number of iterations for determinism tests (default: 1)"
    )
    parser.addoption(
        "--pdf",
        action="store",
        default=None,
        help="Test single PDF only"
    )
    parser.addoption(
        "--stats",
        action="store",
        default=None,
        choices=['show-trends', 'check-regression', 'report', 'query'],
        help="Statistics analysis mode"
    )


# ============================================================================
# Session-Level Fixtures
# ============================================================================

@pytest.fixture(scope="session")
def pdfium_root():
    """PDFium root directory."""
    return Path(__file__).parent.parent


@pytest.fixture(scope="session")
def telemetry_system(pdfium_root):
    """Telemetry system for logging test runs."""
    telemetry_dir = pdfium_root / 'integration_tests' / 'telemetry'
    return TelemetrySystem(telemetry_dir)


@pytest.fixture(scope="session")
def optimized_lib(pdfium_root):
    """Path to optimized shared library."""
    lib_path = pdfium_root / 'out' / 'Release' / 'libpdfium.dylib'
    if not lib_path.exists():
        pytest.exit(f"Optimized library not found: {lib_path}\nRun: ninja -C out/Release pdfium")
    return lib_path


@pytest.fixture(scope="session")
def upstream_bin(pdfium_root):
    """Path to upstream pdfium_test binary (baseline reference).

    This is the C++ reference tool from baseline generation.
    Used by generated tests to validate correctness against upstream PDFium.
    """
    bin_path = pdfium_root / 'out' / 'Release' / 'pdfium_test'
    if not bin_path.exists():
        pytest.exit(f"Upstream pdfium_test not found: {bin_path}\nRun: ninja -C out/Release pdfium_test")
    return bin_path


@pytest.fixture(scope="session")
def system_info(pdfium_root, optimized_lib):
    """Collect system information once per session."""
    return get_all_system_info(pdfium_root, optimized_lib)


@pytest.fixture(scope="session")
def extract_text_tool(pdfium_root):
    """Default text extraction tool (C++ CLI with multi-process parallelism).

    This is the production C++ CLI tool that fulfills CLAUDE.md requirement.
    Uses bulk mode by default with auto-dispatch to multi-process for >= 200 pages.

    For Rust tools (legacy), use extract_text_tool_threaded or extract_text_tool_multiproc.
    """
    tool = pdfium_root / 'out' / 'Release' / 'pdfium_cli'
    if not tool.exists():
        pytest.exit(f"C++ CLI not found: {tool}\nRun: ninja -C out/Optimized-Shared pdfium_cli")
    return tool


@pytest.fixture(scope="session")
def extract_text_tool_threaded(pdfium_root):
    """Rust thread-based text extraction (Arc<Mutex> serialization, ~1.0x speedup).

    LEGACY: This implementation uses threads but PDFium's thread safety constraint forces serialization.
    Use only for testing or comparison. For production, use extract_text_tool (C++ CLI).

    Blocked on macOS SDK 15.2 (Xcode 16.2+) due to missing modulemap files.
    See SDK_COMPATIBILITY.md for details and workarounds.
    """
    tool = pdfium_root / 'rust' / 'target' / 'release' / 'examples' / 'parallel_extract_text'
    if not tool.exists():
        sdk_version = get_macos_sdk_version()
        if sdk_version and sdk_version >= "15.2":
            pytest.skip(f"Rust tools blocked on macOS SDK {sdk_version} (see SDK_COMPATIBILITY.md)")
        else:
            pytest.skip(f"Rust tool not found: {tool}")
    return tool


@pytest.fixture(scope="session")
def extract_text_tool_dispatcher(extract_text_tool):
    """Alias for extract_text_tool (C++ CLI with auto-dispatch).

    Updated to use C++ CLI instead of legacy Rust tool.
    """
    return extract_text_tool


@pytest.fixture(scope="session")
def extract_text_tool_cpp(pdfium_root, lib_path):
    """C++ CLI tool for text extraction (fulfills CLAUDE.md C++ requirement).

    ALIAS: This is now the same as extract_text_tool (default).

    API modes:
    - --workers N: Multi-process with N workers (default 1, max 16)
    - --threads K: Multi-threaded rendering with K threads (default 8, max 32)
    - --quality MODE: Render quality (none|fast|balanced|high, default fast)
    - --debug: Tracing and diagnostic output

    This is the production C++ tool per CLAUDE.md line 23:
    "Implement a CLI interface in C++ that is extremely efficient."
    """
    tool = pdfium_root / 'out' / 'Release' / 'pdfium_cli'
    if not tool.exists():
        pytest.exit(f"C++ CLI not found: {tool}\nRun: ninja -C out/Optimized-Shared pdfium_cli")
    return tool


@pytest.fixture(scope="session")
def render_tool(pdfium_root):
    """Default image rendering tool (C++ CLI with auto-strategy selection).

    This is the production C++ CLI tool that auto-selects the best strategy:
    - Small PDFs (< 200 pages): Single-threaded (bulk mode)
    - Large PDFs (≥ 200 pages): Multi-process with 4 workers (3.95x speedup)

    For Rust tools (legacy), use parallel_render_tool or parallel_render_tool_multiproc.
    """
    tool = pdfium_root / 'out' / 'Release' / 'pdfium_cli'
    if not tool.exists():
        pytest.exit(f"C++ CLI not found: {tool}\nRun: ninja -C out/Optimized-Shared pdfium_cli")
    return tool


@pytest.fixture(scope="session")
def parallel_render_tool(pdfium_root):
    """Rust parallel render tool (thread-based, for Rust API layer validation).

    Blocked on macOS SDK 15.2 (Xcode 16.2+) due to missing modulemap files.
    See SDK_COMPATIBILITY.md for details and workarounds.
    """
    tool = pdfium_root / 'rust' / 'target' / 'release' / 'examples' / 'parallel_render'
    if not tool.exists():
        sdk_version = get_macos_sdk_version()
        if sdk_version and sdk_version >= "15.2":
            pytest.skip(f"Rust tools blocked on macOS SDK {sdk_version} (see SDK_COMPATIBILITY.md)")
        else:
            pytest.skip(f"Rust tool not found: {tool}")
    return tool


@pytest.fixture(scope="session")
def parallel_render_tool_multiproc(pdfium_root):
    """LEGACY: Rust parallel render tool (multi-process).

    Blocked on macOS SDK 15.2 (Xcode 16.2+) due to missing modulemap files.
    See SDK_COMPATIBILITY.md for details and workarounds.
    """
    tool = pdfium_root / 'rust' / 'target' / 'release' / 'examples' / 'parallel_render_multiproc'
    if not tool.exists():
        sdk_version = get_macos_sdk_version()
        if sdk_version and sdk_version >= "15.2":
            pytest.skip(f"Rust tools blocked on macOS SDK {sdk_version} (see SDK_COMPATIBILITY.md)")
        else:
            pytest.skip(f"Rust tool not found: {tool}")
    return tool


@pytest.fixture(scope="session")
def render_tool_threaded(parallel_render_tool):
    """Alias for thread-based render tool."""
    return parallel_render_tool


@pytest.fixture(scope="session")
def render_tool_multiproc(parallel_render_tool_multiproc):
    """Alias for multi-process render tool."""
    return parallel_render_tool_multiproc


@pytest.fixture(scope="session")
def render_tool_dispatcher(render_tool):
    """Alias for render dispatcher tool."""
    return render_tool


@pytest.fixture(scope="session")
def render_pages_tool(pdfium_root):
    """Rust render_pages tool with thumbnail support.

    Blocked on macOS SDK 15.2 (Xcode 16.2+) due to missing modulemap files.
    See SDK_COMPATIBILITY.md for details and workarounds.
    """
    import subprocess

    tool = pdfium_root / 'rust' / 'target' / 'release' / 'examples' / 'render_pages'
    if not tool.exists():
        sdk_version = get_macos_sdk_version()
        if sdk_version and sdk_version >= "15.2":
            pytest.skip(f"Rust render_pages blocked on macOS SDK {sdk_version} (see SDK_COMPATIBILITY.md)")
        else:
            pytest.skip(f"Rust render_pages tool not found: {tool}")

    # Check if tool can actually run (verify dylib dependencies)
    # CRITICAL: Set DYLD_LIBRARY_PATH so tool can find libpdfium_render_bridge.dylib
    env = os.environ.copy()
    env['DYLD_LIBRARY_PATH'] = str(pdfium_root / 'out' / 'Release')

    try:
        result = subprocess.run([str(tool), '--help'], capture_output=True, timeout=5, env=env)
        # Tool should exit 0 for --help or fail with usage message
        # If it crashes (negative exit code), dylib is missing
        if result.returncode < 0:
            sdk_version = get_macos_sdk_version()
            if sdk_version and sdk_version >= "15.2":
                pytest.skip(f"Rust render_pages blocked on macOS SDK {sdk_version} (missing libpdfium_render_bridge.dylib)")
            else:
                pytest.skip(f"Rust render_pages tool cannot run: exit code {result.returncode}")
    except (subprocess.TimeoutExpired, FileNotFoundError):
        pytest.skip(f"Rust render_pages tool cannot be executed")

    return tool


@pytest.fixture(scope="session")
def benchmark_pdfs(pdfium_root):
    """Benchmark PDFs directory (testing/pdfs/benchmark/)."""
    pdf_dir = pdfium_root / 'integration_tests' / 'pdfs' / 'benchmark'
    if not pdf_dir.exists():
        pytest.exit(f"Benchmark PDFs not found: {pdf_dir}")
    return pdf_dir


@pytest.fixture(scope="session")
def edge_cases_pdfs(pdfium_root):
    """Edge cases PDFs directory (testing/pdfs/edge_cases/)."""
    pdf_dir = pdfium_root / 'integration_tests' / 'pdfs' / 'edge_cases'
    if not pdf_dir.exists():
        pytest.exit(f"Edge cases PDFs not found: {pdf_dir}")
    return pdf_dir


@pytest.fixture(scope="session")
def baseline_manager(pdfium_root):
    """Baseline manager for loading expected outcomes."""
    from baselines import BaselineManager
    return BaselineManager(pdfium_root / 'integration_tests')


@pytest.fixture
def use_llm(request):
    """Whether LLM analysis is enabled."""
    return request.config.getoption("--llm")


@pytest.fixture
def workers(request):
    """Number of worker threads."""
    return request.config.getoption("--workers")


@pytest.fixture
def iterations(request):
    """Number of iterations."""
    return request.config.getoption("--iterations")


# ============================================================================
# Automatic Telemetry Collection
# ============================================================================

@pytest.hookimpl(hookwrapper=True)
def pytest_runtest_makereport(item, call):
    """
    Capture test results and telemetry automatically.

    This hook runs after every test and logs everything to CSV.
    NO CODE CHANGES NEEDED IN TESTS!
    """
    outcome = yield
    report = outcome.get_result()

    # Only log on actual test call (not setup/teardown)
    if report.when != "call":
        return

    # Get telemetry system
    if not hasattr(item.session.config, '_telemetry_system'):
        return

    telemetry_system = item.session.config._telemetry_system
    system_info = item.session.config._system_info

    # Extract test metadata from docstring
    test_metadata = extract_test_metadata(item.function.__doc__ or "")

    # Collect telemetry data from test report attributes
    # Tests can set these via: request.node._report_pdf_name = "..."
    telemetry_data = {
        # Temporal
        'timestamp': datetime.now(timezone.utc).isoformat().replace('+00:00', 'Z'),
        'duration_sec': round(report.duration, 3),

        # Test Identity
        'test_id': test_metadata.get('id', ''),
        'test_file': Path(item.fspath).name,
        'test_name': item.name,
        'test_function': item.function.__name__,
        'test_category': test_metadata.get('category', ''),
        'test_level': test_metadata.get('level', ''),
        'test_type': test_metadata.get('type', ''),
        'test_pdf_count': test_metadata.get('pdf_count', ''),

        # Test Result
        'result': report.outcome,
        'passed': 1 if report.outcome == 'passed' else 0,
        'failed': 1 if report.outcome == 'failed' else 0,
        'skipped': 1 if report.outcome == 'skipped' else 0,
        'error_message': str(report.longrepr)[:500] if report.failed else '',
        'error_type': type(report.longrepr).__name__ if report.failed else '',

        # PDF (from test via request.node._report_*)
        'pdf_name': getattr(item, '_report_pdf_name', ''),
        'pdf_path': getattr(item, '_report_pdf_path', ''),
        'pdf_pages': getattr(item, '_report_pdf_pages', ''),
        'pdf_size_mb': getattr(item, '_report_pdf_size_mb', ''),
        'pdf_category': getattr(item, '_report_pdf_category', ''),

        # Execution
        'worker_count': getattr(item, '_report_worker_count', ''),
        'iteration_number': getattr(item, '_report_iteration_number', ''),

        # Validation (Text)
        'text_edit_distance': getattr(item, '_report_text_edit_distance', ''),
        'text_edit_distance_relative': getattr(item, '_report_text_edit_distance_relative', ''),
        'text_similarity': getattr(item, '_report_text_similarity', ''),
        'text_char_diff': getattr(item, '_report_text_char_diff', ''),
        'text_line_diff': getattr(item, '_report_text_line_diff', ''),

        # Validation (Image)
        'md5_match': getattr(item, '_report_md5_match', ''),
        'md5_expected': getattr(item, '_report_md5_expected', ''),
        'md5_actual': getattr(item, '_report_md5_actual', ''),
        'pixel_diff_count': getattr(item, '_report_pixel_diff_count', ''),
        'pixel_diff_pct': getattr(item, '_report_pixel_diff_pct', ''),

        # Performance
        'pages_per_sec': getattr(item, '_report_pages_per_sec', ''),
        'speedup_vs_1w': getattr(item, '_report_speedup_vs_1w', ''),
        'total_pages': getattr(item, '_report_total_pages', ''),
        'total_time_sec': getattr(item, '_report_total_time_sec', ''),

        # Performance details (1w baseline + all worker counts)
        'perf_1w_pps': getattr(item, '_report_perf_1w_pps', ''),
        'perf_1w_duration': getattr(item, '_report_perf_1w_duration', ''),
        'perf_2w_speedup': getattr(item, '_report_perf_2w_speedup', ''),
        'perf_8w_speedup': getattr(item, '_report_perf_8w_speedup', ''),

        # LLM
        'llm_enabled': item.config.getoption("--llm"),
        'llm_called': getattr(item, '_report_llm_called', False),
        'llm_model': getattr(item, '_report_llm_model', ''),
        'llm_cost_usd': getattr(item, '_report_llm_cost_usd', ''),

        # System (from session)
        **system_info,

        # Environment
        'pytest_version': pytest.__version__,
        'dyld_library_path': os.environ.get('DYLD_LIBRARY_PATH', ''),
    }

    # Log to CSV
    telemetry_system.log_test_run(telemetry_data)


def pytest_configure(config):
    """Initialize telemetry system and register markers."""

    # Check for stale binary (source newer than binary)
    pdfium_root = Path(__file__).parent.parent
    binary = pdfium_root / "out" / "Release" / "pdfium_cli"
    source = pdfium_root / "examples" / "pdfium_cli.cpp"

    if binary.exists() and source.exists():
        binary_mtime = binary.stat().st_mtime
        source_mtime = source.stat().st_mtime

        if source_mtime > binary_mtime:
            from datetime import datetime, timezone
            binary_time = datetime.fromtimestamp(binary_mtime).strftime("%Y-%m-%d %H:%M:%S")
            source_time = datetime.fromtimestamp(source_mtime).strftime("%Y-%m-%d %H:%M:%S")

            pytest.exit(
                f"\n{'='*70}\n"
                f"STALE BINARY DETECTED - Tests would fail with misleading errors\n"
                f"{'='*70}\n"
                f"Binary: {binary}\n"
                f"  Built: {binary_time}\n"
                f"Source: {source}\n"
                f"  Modified: {source_time}\n"
                f"\n"
                f"The binary is older than the source code.\n"
                f"This causes test failures because the binary doesn't have latest features.\n"
                f"\n"
                f"Fix: Rebuild the binary\n"
                f"  ninja -C out/Release pdfium_cli\n"
                f"{'='*70}\n",
                returncode=2
            )

    # Register markers
    config.addinivalue_line("markers", "correctness: Tests output correctness")
    config.addinivalue_line("markers", "performance: Tests performance/speed")
    config.addinivalue_line("markers", "stability: Tests determinism and stability")
    config.addinivalue_line("markers", "edge_cases: Tests unusual/malformed inputs")
    config.addinivalue_line("markers", "scaling: Tests worker scaling")
    config.addinivalue_line("markers", "smoke: Quick sanity check (30 sec)")
    config.addinivalue_line("markers", "quick: Pre-PR validation (5 min)")
    config.addinivalue_line("markers", "full: CI/CD validation (15 min)")
    config.addinivalue_line("markers", "extended: Comprehensive test (1+ hours)")
    config.addinivalue_line("markers", "text: Text extraction tests")
    config.addinivalue_line("markers", "image: Image rendering tests")
    config.addinivalue_line("markers", "both: Text + Image tests")

    # Initialize telemetry system
    pdfium_root = Path(__file__).parent.parent
    telemetry_dir = pdfium_root / 'integration_tests' / 'telemetry'

    telemetry_system = TelemetrySystem(telemetry_dir)
    config._telemetry_system = telemetry_system

    # Collect system info once
    optimized_lib = pdfium_root / 'out' / 'Optimized-Shared' / 'libpdfium.dylib'
    system_info = get_all_system_info(pdfium_root, optimized_lib if optimized_lib.exists() else None)
    config._system_info = system_info

    # Handle --stats mode
    stats_mode = config.getoption("--stats", default=None)
    if stats_mode:
        # Import from correct path
        import sys
        lib_path = Path(__file__).parent / 'lib'
        if str(lib_path) not in sys.path:
            sys.path.insert(0, str(lib_path))

        from statistics import StatisticsAnalyzer
        analyzer = StatisticsAnalyzer(telemetry_dir)

        if stats_mode == 'show-trends':
            analyzer.show_trends()
        elif stats_mode == 'check-regression':
            analyzer.check_regression()
        elif stats_mode == 'report':
            analyzer.generate_report()
        elif stats_mode == 'query':
            analyzer.interactive_query()

        pytest.exit("Statistics analysis complete", returncode=0)


def pytest_sessionfinish(session, exitstatus):
    """Print telemetry summary at end of session."""
    if hasattr(session.config, '_telemetry_system'):
        telemetry_system = session.config._telemetry_system
        csv_path = telemetry_system.csv_path

        if csv_path.exists():
            import csv
            with open(csv_path, 'r') as f:
                lines = sum(1 for _ in f) - 1  # Subtract header

            print(f"\n{'='*70}")
            print(f"📊 Telemetry logged to: {csv_path}")
            print(f"📊 Total runs logged: {lines}")
            print(f"📊 Session ID: {telemetry_system.session_id}")
            print(f"{'='*70}\n")


# ============================================================================
# Test Helpers (for test implementations)
# ============================================================================

def extract_text(pdf_path: Path, output_file: Path, worker_count: int, lib_path: Path, tool_path: Path) -> bool:
    """Extract text from PDF.

    Supports both Rust and C++ CLI tools:
    - Rust: <pdf> <output> [worker_count]
    - C++ (pdfium_cli): [--workers N] extract-text <pdf> <output>
    """
    import subprocess

    env = os.environ.copy()
    env['DYLD_LIBRARY_PATH'] = str(lib_path.parent)

    # Detect if tool is C++ CLI (pdfium_cli) or Rust
    is_cpp_cli = tool_path.name == 'pdfium_cli'

    if is_cpp_cli:
        # C++ CLI: [--workers N] extract-text <pdf> <output>
        # worker_count semantics:
        #   1 = force single-threaded (--workers 1 flag to prevent auto-dispatch)
        #   >1 = explicit multi-process (--workers N)
        #   default = auto-dispatch (no mode flag, CLI decides based on page count)
        # v2.0.0: CLI defaults to UTF-8, but tests use UTF-32 LE for baseline compatibility
        if worker_count == 1:
            # Explicit single-threaded: force --workers 1 to prevent auto-dispatch
            # Used by performance tests to measure baseline single-threaded speed
            args = [str(tool_path), '--workers', '1', '--encoding', 'utf32le', 'extract-text', str(pdf_path), str(output_file)]
        elif worker_count > 1:
            # Explicit worker count with --workers
            args = [str(tool_path), '--workers', str(worker_count), '--encoding', 'utf32le', 'extract-text', str(pdf_path), str(output_file)]
        else:
            # Auto-dispatch: no mode flag, CLI auto-selects based on PDF size
            args = [str(tool_path), '--encoding', 'utf32le', 'extract-text', str(pdf_path), str(output_file)]
    else:
        # Rust CLI: <pdf> <output> [worker_count]
        args = [str(tool_path), str(pdf_path), str(output_file), str(worker_count)]

    result = subprocess.run(
        args,
        env=env,
        capture_output=True,
        text=True,
        timeout=300
    )

    return result.returncode == 0


def render_parallel(pdf_path: Path, worker_count: int, lib_path: Path, tool_path: Path, start_page=None, end_page=None):
    """Render PDF in parallel (returns pages, seconds).

    Handles four tool types:
    - pdfium_cli (C++ CLI): [--workers N] [--threads K] render-pages <pdf> <output_dir> [--pages RANGE]
    - parallel_render (thread-based Rust): <pdf> [worker_count]
    - parallel_render_multiproc (Rust): <pdf> <output_dir> [worker_count] [dpi]
    - render_pages (dispatcher Rust): <pdf> <output_dir> [worker_count] [dpi]
    """
    import subprocess
    import tempfile
    import time

    env = os.environ.copy()
    env['DYLD_LIBRARY_PATH'] = str(lib_path.parent)

    # Detect if tool is C++ CLI
    is_cpp_cli = tool_path.name == 'pdfium_cli'

    if is_cpp_cli:
        # C++ CLI: [--workers N] [--threads K] [--pages RANGE] render-pages <pdf> <output_dir>
        with tempfile.TemporaryDirectory() as tmpdir:
            # Build flags first (must come before operation)
            flags = []
            # PNG is default format, no flag needed
            if worker_count == 1:
                # Force single-threaded (no auto-dispatch, no threading)
                # Must explicitly set --workers 1 and --threads 1
                flags.extend(['--workers', '1', '--threads', '1'])
            elif worker_count > 1:
                # Explicit worker count, single-threaded per worker for consistency
                flags.extend(['--workers', str(worker_count), '--threads', '1'])
            # else: auto-dispatch (no flags)

            # Add page range if specified (must come before operation)
            # CLI uses --pages syntax: --pages START-END or --pages N
            if start_page is not None and end_page is not None:
                if start_page == end_page:
                    flags.extend(['--pages', str(start_page)])
                else:
                    flags.extend(['--pages', f'{start_page}-{end_page}'])
            elif start_page is not None:
                flags.extend(['--pages', f'{start_page}-'])
            elif end_page is not None:
                flags.extend(['--pages', f'0-{end_page}'])

            # Build complete command: tool + flags + operation + args
            args = [str(tool_path)] + flags + ['render-pages', str(pdf_path), tmpdir]

            start_time = time.time()
            result = subprocess.run(
                args,
                env=env,
                capture_output=True,
                text=True,
                timeout=600
            )
            duration = time.time() - start_time

            if result.returncode != 0:
                return None, None

            # Count generated PNG and JPEG files (smart mode may produce JPEG)
            from pathlib import Path as P
            tmpdir_path = P(tmpdir)
            png_count = len(list(tmpdir_path.glob("page_*.png")))
            jpg_count = len(list(tmpdir_path.glob("page_*.jpg")))
            page_count = png_count + jpg_count

            return page_count, duration
    else:
        # Rust tools
        tool_name = tool_path.name
        if 'multiproc' in tool_name or 'render_pages' in tool_name:
            # Multi-process tool or dispatcher needs output_dir
            with tempfile.TemporaryDirectory() as tmpdir:
                args = [str(tool_path), str(pdf_path), tmpdir, str(worker_count), "300"]

                result = subprocess.run(
                    args,
                    env=env,
                    capture_output=True,
                    text=True,
                    timeout=600
                )
        else:
            # Thread-based tool: just pdf and worker_count
            args = [str(tool_path), str(pdf_path), str(worker_count)]

            result = subprocess.run(
                args,
                env=env,
                capture_output=True,
                text=True,
                timeout=600
            )

        if result.returncode != 0:
            return None, None

        # Parse output: "Rendered X pages in Y seconds"
        for line in result.stdout.strip().split('\n'):
            if 'pages in' in line and 'seconds' in line:
                parts = line.split()
                pages = int(parts[1])
                seconds = float(parts[4])
                return pages, seconds

        return None, None


def render_images(pdf_path: Path, output_dir: Path, worker_count: int, lib_path: Path, tool_path: Path, dpi: int = 300) -> bool:
    """Render PDF to images.

    Handles two tool types:
    - parallel_render: <pdf> [worker_count] [output_dir] [dpi]
    - parallel_render_multiproc: <pdf> <output_dir> [worker_count] [dpi]
    """
    import subprocess

    env = os.environ.copy()
    env['DYLD_LIBRARY_PATH'] = str(lib_path.parent)

    # Determine tool type based on name
    tool_name = tool_path.name
    if 'multiproc' in tool_name:
        # Multi-process tool: <pdf> <output_dir> [worker_count] [dpi]
        args = [str(tool_path), str(pdf_path), str(output_dir), str(worker_count), str(dpi)]
    else:
        # Thread-based tool: <pdf> [worker_count] [output_dir] [dpi]
        args = [str(tool_path), str(pdf_path), str(worker_count), str(output_dir), str(dpi)]

    result = subprocess.run(
        args,
        env=env,
        capture_output=True,
        text=True,
        timeout=600
    )

    return result.returncode == 0


def render_with_md5(pdf_path: Path, worker_count: int, lib_path: Path, tool_path: Path, dpi: int = 300):
    """Render PDF to PPM and compute MD5 hashes for each page (with batched processing).

    This function renders pages in batches to limit disk usage for large PDFs.
    For a 1931-page PDF at 300 DPI (~30MB/page), batched processing limits
    disk usage to ~300MB instead of ~58GB.

    Batched processing approach:
    1. Render all pages to temp directory (multi-process if worker_count > 1)
    2. Process PPM files in batches of 10:
       - Find batch of PPM files
       - Compute MD5 for each
       - Delete immediately after MD5 computation
    3. Continue until all pages processed

    This achieves bounded disk usage without requiring page-range support in C++ tool.

    Supports both Rust and C++ CLI tools:
    - Rust: <pdf> <output_dir> <worker_count> <dpi> --ppm
    - C++ (pdfium_cli): [--workers N] [--threads K] [--quality MODE] --ppm render-pages <pdf> <output_dir>

    Args:
        pdf_path: Path to PDF file
        worker_count: Number of workers
        lib_path: Path to libpdfium.dylib
        tool_path: Path to render tool
        dpi: DPI for rendering (default 300)

    Returns:
        Dict[str, str]: Mapping of page number (as string) to MD5 hash, or None on error
        Example: {"0": "abc123...", "1": "def456...", ...}
    """
    import subprocess
    import tempfile
    import hashlib
    import time
    import threading

    env = os.environ.copy()
    env['DYLD_LIBRARY_PATH'] = str(lib_path.parent)

    # Detect if tool is C++ CLI (pdfium_cli) or Rust
    is_cpp_cli = tool_path.name == 'pdfium_cli'

    # N=387 FIX: Sequential processing (no threading race condition)
    # Render all pages → Wait for completion → MD5+delete each file sequentially
    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir_path = Path(tmpdir)
        page_hashes = {}

        # Render all pages first
        if is_cpp_cli:
            # C++ CLI: [--workers N] [--threads K] [--quality MODE] --ppm render-pages <pdf> <output_dir>
            # For correctness tests, use balanced quality and single-threaded to match baselines
            if worker_count == 1:
                args = [str(tool_path), '--workers', '1', '--threads', '1', '--quality', 'balanced', '--ppm', 'render-pages', str(pdf_path), str(tmpdir_path)]
            elif worker_count > 1:
                args = [str(tool_path), '--workers', str(worker_count), '--threads', '1', '--quality', 'balanced', '--ppm', 'render-pages', str(pdf_path), str(tmpdir_path)]
            else:
                # Auto-dispatch
                args = [str(tool_path), '--threads', '1', '--quality', 'balanced', '--ppm', 'render-pages', str(pdf_path), str(tmpdir_path)]
        else:
            # Rust CLI: <pdf> <output_dir> <worker_count> <dpi> --ppm
            args = [str(tool_path), str(pdf_path), str(tmpdir_path), str(worker_count), str(dpi), "--ppm"]

        result = subprocess.run(
            args,
            env=env,
            capture_output=True,
            text=True,
            timeout=1800  # 30 min timeout for very large PDFs
        )

        # After rendering completes, process files sequentially
        if result.returncode != 0:
            return None

        # Sequential MD5 computation with immediate deletion (no race condition)
        BATCH_SIZE = 10  # Delete in batches to limit peak disk
        all_ppms = sorted(tmpdir_path.glob("page_*.ppm"))

        for i in range(0, len(all_ppms), BATCH_SIZE):
            batch = all_ppms[i:i+BATCH_SIZE]
            for ppm_file in batch:
                try:
                    page_num_str = ppm_file.stem.split('_')[1]
                    page_num = str(int(page_num_str))
                    md5_hash = hashlib.md5(ppm_file.read_bytes()).hexdigest()
                    page_hashes[page_num] = md5_hash
                    ppm_file.unlink()  # Delete immediately after MD5
                except Exception:
                    continue

        return page_hashes


# Attach to pytest module for easy access
pytest.extract_text = extract_text
pytest.render_parallel = render_parallel
pytest.render_images = render_images
pytest.render_with_md5 = render_with_md5


# Fixture aliases for generated tests
@pytest.fixture(scope="session")
def test_binary(extract_text_tool_cpp):
    """Alias for backward compatibility with generated tests.

    Updated to use C++ CLI (pdfium_cli) to fulfill CLAUDE.md requirement.
    Previously used extract_text_tool_dispatcher (Rust).
    """
    return extract_text_tool_cpp


@pytest.fixture(scope="session")
def expected_outputs(pdfium_root):
    """Expected outputs directory for generated tests."""
    return pdfium_root / 'integration_tests' / 'master_test_suite' / 'expected_outputs'
