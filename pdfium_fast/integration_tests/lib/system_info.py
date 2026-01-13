"""
System Information Collector

Captures comprehensive system state for telemetry:
- CPU model, cores, frequency, temperature
- Memory usage, pressure
- Disk space
- System load averages
- Platform information
"""

import os
import platform
import subprocess
import psutil
from pathlib import Path
from typing import Dict, Tuple


def get_cpu_info() -> Dict[str, any]:
    """Get CPU information."""
    try:
        # Get CPU model
        if platform.system() == 'Darwin':  # macOS
            cpu_brand = subprocess.check_output(
                ['sysctl', '-n', 'machdep.cpu.brand_string'],
                text=True
            ).strip()
        else:
            cpu_brand = platform.processor()

        return {
            'cpu_model': cpu_brand,
            'cpu_cores_physical': psutil.cpu_count(logical=False),
            'cpu_cores_logical': psutil.cpu_count(logical=True),
            'cpu_freq_mhz': psutil.cpu_freq().current if psutil.cpu_freq() else 0,
        }
    except:
        return {
            'cpu_model': 'unknown',
            'cpu_cores_physical': 0,
            'cpu_cores_logical': 0,
            'cpu_freq_mhz': 0,
        }


def get_cpu_temp() -> float:
    """Get CPU temperature (Celsius)."""
    try:
        if platform.system() == 'Darwin':
            # macOS - requires additional tools, return 0 if not available
            result = subprocess.run(
                ['osx-cpu-temp'], capture_output=True, text=True, timeout=2
            )
            if result.returncode == 0:
                return float(result.stdout.strip().replace('°C', ''))
    except:
        pass
    return 0.0


def get_load_avg() -> Tuple[float, float, float]:
    """Get system load averages (1m, 5m, 15m)."""
    try:
        load1, load5, load15 = os.getloadavg()
        return load1, load5, load15
    except:
        return 0.0, 0.0, 0.0


def get_memory_info() -> Dict[str, any]:
    """Get memory information."""
    try:
        mem = psutil.virtual_memory()
        swap = psutil.swap_memory()

        return {
            'ram_total_gb': mem.total / (1024**3),
            'ram_used_gb': mem.used / (1024**3),
            'ram_free_gb': mem.available / (1024**3),
            'ram_percent': mem.percent,
            'swap_used_gb': swap.used / (1024**3),
            'swap_percent': swap.percent,
        }
    except:
        return {
            'ram_total_gb': 0,
            'ram_used_gb': 0,
            'ram_free_gb': 0,
            'ram_percent': 0,
            'swap_used_gb': 0,
            'swap_percent': 0,
        }


def get_disk_info() -> Dict[str, any]:
    """Get disk space information."""
    try:
        disk = psutil.disk_usage('/')
        return {
            'disk_total_gb': disk.total / (1024**3),
            'disk_used_gb': disk.used / (1024**3),
            'disk_free_gb': disk.free / (1024**3),
            'disk_percent': disk.percent,
        }
    except:
        return {
            'disk_total_gb': 0,
            'disk_used_gb': 0,
            'disk_free_gb': 0,
            'disk_percent': 0,
        }


def get_git_info(repo_path: Path) -> Dict[str, any]:
    """Get git repository information."""
    try:
        # Current commit
        commit_hash = subprocess.check_output(
            ['git', 'rev-parse', 'HEAD'],
            cwd=repo_path, text=True
        ).strip()

        commit_short = subprocess.check_output(
            ['git', 'rev-parse', '--short', 'HEAD'],
            cwd=repo_path, text=True
        ).strip()

        # Branch
        branch = subprocess.check_output(
            ['git', 'rev-parse', '--abbrev-ref', 'HEAD'],
            cwd=repo_path, text=True
        ).strip()

        # Dirty status
        status = subprocess.check_output(
            ['git', 'status', '--porcelain'],
            cwd=repo_path, text=True
        ).strip()
        dirty = len(status) > 0

        # Commit timestamp
        commit_timestamp = subprocess.check_output(
            ['git', 'log', '-1', '--format=%aI'],
            cwd=repo_path, text=True
        ).strip()

        # Author
        commit_author = subprocess.check_output(
            ['git', 'log', '-1', '--format=%an'],
            cwd=repo_path, text=True
        ).strip()

        return {
            'git_commit_hash': commit_hash,
            'git_commit_short': commit_short,
            'git_branch': branch,
            'git_dirty': dirty,
            'git_timestamp': commit_timestamp,
            'git_author': commit_author,
        }
    except:
        return {
            'git_commit_hash': 'unknown',
            'git_commit_short': 'unknown',
            'git_branch': 'unknown',
            'git_dirty': False,
            'git_timestamp': '',
            'git_author': '',
        }


def get_binary_info(binary_path: Path) -> Dict[str, any]:
    """Get binary metadata."""
    try:
        import hashlib
        import time

        # MD5
        md5 = hashlib.md5()
        with open(binary_path, 'rb') as f:
            for chunk in iter(lambda: f.read(8192), b''):
                md5.update(chunk)

        # Size
        size_mb = binary_path.stat().st_size / (1024**2)

        # Timestamp
        mtime = binary_path.stat().st_mtime
        timestamp = time.strftime('%Y-%m-%dT%H:%M:%S', time.localtime(mtime))

        return {
            'binary_path': str(binary_path),
            'binary_md5': md5.hexdigest(),
            'binary_size_mb': round(size_mb, 2),
            'binary_timestamp': timestamp,
        }
    except:
        return {
            'binary_path': str(binary_path) if binary_path else '',
            'binary_md5': '',
            'binary_size_mb': 0,
            'binary_timestamp': '',
        }


def get_machine_id() -> str:
    """Get unique machine identifier."""
    try:
        # Use hostname + platform
        hostname = platform.node()
        system = platform.system()
        machine = platform.machine()
        return f"{hostname}_{system}_{machine}".replace(' ', '_')
    except:
        return 'unknown'


def get_all_system_info(repo_path: Path, binary_path: Path = None) -> Dict[str, any]:
    """Get comprehensive system information."""
    cpu_info = get_cpu_info()
    mem_info = get_memory_info()
    disk_info = get_disk_info()
    git_info = get_git_info(repo_path)
    load1, load5, load15 = get_load_avg()

    info = {
        # CPU
        **cpu_info,
        'cpu_temp_c': get_cpu_temp(),

        # Memory
        **mem_info,

        # Disk
        **disk_info,

        # Load
        'load_avg_1m': round(load1, 2),
        'load_avg_5m': round(load5, 2),
        'load_avg_15m': round(load15, 2),

        # Git
        **git_info,

        # Platform
        'platform': platform.system(),
        'platform_release': platform.release(),
        'platform_version': platform.version(),
        'python_version': platform.python_version(),
        'machine_id': get_machine_id(),
    }

    # Binary info (if provided)
    if binary_path and binary_path.exists():
        info.update(get_binary_info(binary_path))

    return info
