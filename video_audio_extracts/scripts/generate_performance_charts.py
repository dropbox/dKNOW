#!/usr/bin/env python3
"""
Generate performance comparison charts from benchmark data.
Creates HTML/SVG charts without requiring matplotlib.
"""

import json
import os
from pathlib import Path

# Benchmark data extracted from PERFORMANCE_BENCHMARKS.md (N=149: 23/33 operations)
BENCHMARK_DATA = {
    "operations": [
        # Core Extraction Operations
        {"name": "subtitle_extraction", "category": "Intelligence", "latency_ms": 50, "memory_mb": 24.8, "throughput_mbs": 0.34},
        {"name": "scene_detection", "category": "Intelligence", "latency_ms": 53, "memory_mb": 14.65, "throughput_mbs": 2.80},
        {"name": "format_conversion", "category": "Utility", "latency_ms": 54, "memory_mb": 14.84, "throughput_mbs": 0.55},
        {"name": "image_quality_assessment", "category": "Intelligence", "latency_ms": 55, "memory_mb": 14.64, "throughput_mbs": None},
        {"name": "smart_thumbnail", "category": "Intelligence", "latency_ms": 56, "memory_mb": 14.71, "throughput_mbs": 2.69},
        {"name": "object_detection", "category": "Vision", "latency_ms": 56, "memory_mb": 14.56, "throughput_mbs": None},
        {"name": "audio_embeddings", "category": "Embeddings", "latency_ms": 56, "memory_mb": 15.15, "throughput_mbs": 8.00},
        {"name": "audio_extraction", "category": "Core", "latency_ms": 57, "memory_mb": 14.67, "throughput_mbs": 2.61},
        {"name": "audio_classification", "category": "Audio", "latency_ms": 57, "memory_mb": 14.60, "throughput_mbs": 7.85},
        {"name": "transcription", "category": "Audio", "latency_ms": 57, "memory_mb": 14.84, "throughput_mbs": 1.58},
        {"name": "duplicate_detection", "category": "Utility", "latency_ms": 58, "memory_mb": 14.51, "throughput_mbs": 0.51},
        {"name": "keyframes", "category": "Core", "latency_ms": 59, "memory_mb": 14.57, "throughput_mbs": 2.55},
        {"name": "voice_activity_detection", "category": "Audio", "latency_ms": 59, "memory_mb": 14.57, "throughput_mbs": 7.62},
        {"name": "ocr", "category": "Vision", "latency_ms": 63, "memory_mb": 14.59, "throughput_mbs": None},
        {"name": "face_detection", "category": "Vision", "latency_ms": 65, "memory_mb": 14.64, "throughput_mbs": None},
        {"name": "vision_embeddings", "category": "Embeddings", "latency_ms": 66, "memory_mb": 14.53, "throughput_mbs": None},
        {"name": "metadata_extraction", "category": "Core", "latency_ms": 86, "memory_mb": 14.59, "throughput_mbs": 1.74},
        # New operations from N=146-149
        {"name": "shot_classification", "category": "Intelligence", "latency_ms": 130, "memory_mb": 17.3, "throughput_mbs": None},
        {"name": "audio_enhancement_metadata", "category": "Audio", "latency_ms": 140, "memory_mb": 17.9, "throughput_mbs": 3.35},
        {"name": "acoustic_scene_classification", "category": "Audio", "latency_ms": 190, "memory_mb": 71.3, "throughput_mbs": 2.47},
        {"name": "pose_estimation", "category": "Vision", "latency_ms": 260, "memory_mb": 89.5, "throughput_mbs": None},
        {"name": "diarization", "category": "Audio", "latency_ms": 350, "memory_mb": 108.2, "throughput_mbs": 1.34},
        {"name": "profanity_detection", "category": "Audio", "latency_ms": 450, "memory_mb": 309.0, "throughput_mbs": None},
    ],
    "concurrency_scaling": [
        {"workers": 1, "time_s": 1.93, "throughput_fps": 4.13, "speedup": 1.00, "efficiency_pct": 100.0},
        {"workers": 2, "time_s": 1.12, "throughput_fps": 7.12, "speedup": 1.72, "efficiency_pct": 86.0},
        {"workers": 4, "time_s": 1.00, "throughput_fps": 8.00, "speedup": 1.93, "efficiency_pct": 48.2},
        {"workers": 8, "time_s": 0.92, "throughput_fps": 8.71, "speedup": 2.10, "efficiency_pct": 26.2},
        {"workers": 16, "time_s": 0.97, "throughput_fps": 8.27, "speedup": 2.00, "efficiency_pct": 12.5},
    ]
}

def generate_html_header():
    """Generate HTML header with SVG styling."""
    return """<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Performance Benchmarks - video-audio-extracts</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Arial, sans-serif;
            max-width: 1400px;
            margin: 40px auto;
            padding: 20px;
            background: #f5f5f5;
        }
        .chart-container {
            background: white;
            padding: 30px;
            margin-bottom: 40px;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }
        h1 {
            color: #333;
            border-bottom: 3px solid #007AFF;
            padding-bottom: 10px;
        }
        h2 {
            color: #555;
            margin-top: 0;
        }
        .metadata {
            color: #666;
            font-size: 14px;
            margin-bottom: 20px;
        }
        svg {
            display: block;
            margin: 0 auto;
        }
        .bar { transition: opacity 0.2s; }
        .bar:hover { opacity: 0.8; cursor: pointer; }
        .axis-label { font-size: 14px; fill: #333; }
        .axis-line { stroke: #ccc; stroke-width: 1; }
        .grid-line { stroke: #eee; stroke-width: 1; }
        text { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Arial, sans-serif; }
        .legend { font-size: 12px; }
        .tooltip {
            position: absolute;
            background: rgba(0, 0, 0, 0.8);
            color: white;
            padding: 8px;
            border-radius: 4px;
            font-size: 12px;
            pointer-events: none;
            display: none;
        }
    </style>
</head>
<body>
    <h1>Performance Benchmarks - video-audio-extracts</h1>
    <div class="metadata">
        <strong>Date:</strong> 2025-11-09 (N=148) |
        <strong>Hardware:</strong> Apple M2 Max, 64 GB RAM, macOS Darwin 24.6.0 |
        <strong>Binary:</strong> video-extract v1.0.0 (release build) |
        <strong>Operations:</strong> 22/33 benchmarked
    </div>
"""

def generate_html_footer():
    """Generate HTML footer."""
    return """
    <div class="chart-container">
        <h2>About These Benchmarks</h2>
        <p><strong>Test Configuration:</strong></p>
        <ul>
            <li>Environment: VIDEO_EXTRACT_THREADS=4 (limits thread pool sizes)</li>
            <li>Execution mode: performance mode (optimized for speed, no debug overhead)</li>
            <li>Measurement tools: /usr/bin/time -l (macOS) for memory and wall-clock time</li>
            <li>Test files: Small (30-450 KB) test files from test_edge_cases/ directory</li>
        </ul>
        <p><strong>Important Notes:</strong></p>
        <ul>
            <li>Latency measurements (53-86ms) are dominated by binary startup overhead (~50ms)</li>
            <li>For production workloads with larger files (1-100 MB), processing time will dominate</li>
            <li>Throughput metrics (MB/s) become more representative on files >1 MB</li>
            <li>Memory usage scales with file size for production workloads (256-1024 MB for heavy operations)</li>
        </ul>
        <p><strong>Source:</strong> <code>docs/PERFORMANCE_BENCHMARKS.md</code></p>
    </div>
</body>
</html>
"""

def generate_throughput_chart():
    """Generate throughput comparison bar chart."""
    ops = [op for op in BENCHMARK_DATA["operations"] if op["throughput_mbs"] is not None]
    ops_sorted = sorted(ops, key=lambda x: x["throughput_mbs"], reverse=True)

    # SVG dimensions
    width, height = 1200, 600
    margin = {"top": 40, "right": 40, "bottom": 120, "left": 150}
    chart_width = width - margin["left"] - margin["right"]
    chart_height = height - margin["top"] - margin["bottom"]

    # Scale
    max_throughput = max(op["throughput_mbs"] for op in ops_sorted)
    x_scale = chart_width / max_throughput
    y_scale = chart_height / len(ops_sorted)
    bar_height = y_scale * 0.7

    # Colors by category
    colors = {
        "Core": "#007AFF",
        "Audio": "#34C759",
        "Vision": "#FF9500",
        "Intelligence": "#AF52DE",
        "Embeddings": "#FF2D55",
        "Utility": "#5AC8FA"
    }

    svg = f'<svg width="{width}" height="{height}" xmlns="http://www.w3.org/2000/svg">\n'

    # Title
    svg += f'  <text x="{width/2}" y="20" text-anchor="middle" font-size="18" font-weight="bold">Throughput Comparison (MB/s)</text>\n'

    # Grid lines
    for i in range(0, int(max_throughput) + 2, 2):
        x = margin["left"] + i * x_scale
        svg += f'  <line x1="{x}" y1="{margin["top"]}" x2="{x}" y2="{height - margin["bottom"]}" class="grid-line"/>\n'
        svg += f'  <text x="{x}" y="{height - margin["bottom"] + 20}" text-anchor="middle" font-size="12" fill="#666">{i}</text>\n'

    # Bars
    for i, op in enumerate(ops_sorted):
        y = margin["top"] + i * y_scale + (y_scale - bar_height) / 2
        bar_width = op["throughput_mbs"] * x_scale
        color = colors.get(op["category"], "#999")

        svg += f'  <rect x="{margin["left"]}" y="{y}" width="{bar_width}" height="{bar_height}" fill="{color}" class="bar" opacity="0.85">\n'
        svg += f'    <title>{op["name"]}: {op["throughput_mbs"]:.2f} MB/s</title>\n'
        svg += f'  </rect>\n'

        # Operation name
        svg += f'  <text x="{margin["left"] - 10}" y="{y + bar_height/2 + 4}" text-anchor="end" font-size="12" fill="#333">{op["name"].replace("_", " ")}</text>\n'

        # Value label
        svg += f'  <text x="{margin["left"] + bar_width + 5}" y="{y + bar_height/2 + 4}" font-size="11" fill="#666">{op["throughput_mbs"]:.2f}</text>\n'

    # Legend
    legend_x = width - margin["right"] - 180
    legend_y = margin["top"]
    svg += f'  <text x="{legend_x}" y="{legend_y}" font-size="12" font-weight="bold" fill="#333">Categories:</text>\n'
    for i, (category, color) in enumerate(colors.items()):
        y = legend_y + 20 + i * 20
        svg += f'  <rect x="{legend_x}" y="{y - 10}" width="12" height="12" fill="{color}" opacity="0.85"/>\n'
        svg += f'  <text x="{legend_x + 18}" y="{y}" font-size="11" fill="#333">{category}</text>\n'

    svg += '</svg>\n'
    return svg

def generate_latency_chart():
    """Generate latency distribution chart."""
    ops = BENCHMARK_DATA["operations"]
    ops_sorted = sorted(ops, key=lambda x: x["latency_ms"])

    # SVG dimensions
    width, height = 1200, 600
    margin = {"top": 40, "right": 40, "bottom": 120, "left": 150}
    chart_width = width - margin["left"] - margin["right"]
    chart_height = height - margin["top"] - margin["bottom"]

    # Scale
    max_latency = max(op["latency_ms"] for op in ops_sorted)
    x_scale = chart_width / max_latency
    y_scale = chart_height / len(ops_sorted)
    bar_height = y_scale * 0.7

    # Colors by category
    colors = {
        "Core": "#007AFF",
        "Audio": "#34C759",
        "Vision": "#FF9500",
        "Intelligence": "#AF52DE",
        "Embeddings": "#FF2D55",
        "Utility": "#5AC8FA"
    }

    svg = f'<svg width="{width}" height="{height}" xmlns="http://www.w3.org/2000/svg">\n'

    # Title
    svg += f'  <text x="{width/2}" y="20" text-anchor="middle" font-size="18" font-weight="bold">Latency Distribution (milliseconds)</text>\n'

    # Grid lines
    for i in range(0, int(max_latency) + 20, 20):
        x = margin["left"] + i * x_scale
        svg += f'  <line x1="{x}" y1="{margin["top"]}" x2="{x}" y2="{height - margin["bottom"]}" class="grid-line"/>\n'
        svg += f'  <text x="{x}" y="{height - margin["bottom"] + 20}" text-anchor="middle" font-size="12" fill="#666">{i}</text>\n'

    # Bars
    for i, op in enumerate(ops_sorted):
        y = margin["top"] + i * y_scale + (y_scale - bar_height) / 2
        bar_width = op["latency_ms"] * x_scale
        color = colors.get(op["category"], "#999")

        svg += f'  <rect x="{margin["left"]}" y="{y}" width="{bar_width}" height="{bar_height}" fill="{color}" class="bar" opacity="0.85">\n'
        svg += f'    <title>{op["name"]}: {op["latency_ms"]}ms</title>\n'
        svg += f'  </rect>\n'

        # Operation name
        svg += f'  <text x="{margin["left"] - 10}" y="{y + bar_height/2 + 4}" text-anchor="end" font-size="12" fill="#333">{op["name"].replace("_", " ")}</text>\n'

        # Value label
        svg += f'  <text x="{margin["left"] + bar_width + 5}" y="{y + bar_height/2 + 4}" font-size="11" fill="#666">{op["latency_ms"]}ms</text>\n'

    svg += '</svg>\n'
    return svg

def generate_memory_chart():
    """Generate memory usage comparison chart."""
    ops = BENCHMARK_DATA["operations"]

    # Group by category and calculate averages
    categories = {}
    for op in ops:
        cat = op["category"]
        if cat not in categories:
            categories[cat] = []
        categories[cat].append(op["memory_mb"])

    category_stats = []
    for cat, mems in categories.items():
        avg = sum(mems) / len(mems)
        min_mem = min(mems)
        max_mem = max(mems)
        category_stats.append({"category": cat, "avg": avg, "min": min_mem, "max": max_mem})

    category_stats.sort(key=lambda x: x["avg"], reverse=True)

    # SVG dimensions
    width, height = 1000, 500
    margin = {"top": 40, "right": 40, "bottom": 80, "left": 100}
    chart_width = width - margin["left"] - margin["right"]
    chart_height = height - margin["top"] - margin["bottom"]

    # Scale (dynamic based on max memory)
    max_memory = max(stat["max"] for stat in category_stats)
    max_memory = max(max_memory * 1.1, 20)  # Add 10% headroom, minimum 20 MB
    y_scale = chart_height / max_memory
    bar_width = chart_width / len(category_stats) * 0.7
    bar_spacing = chart_width / len(category_stats)

    # Colors
    colors = {
        "Core": "#007AFF",
        "Audio": "#34C759",
        "Vision": "#FF9500",
        "Intelligence": "#AF52DE",
        "Embeddings": "#FF2D55",
        "Utility": "#5AC8FA"
    }

    svg = f'<svg width="{width}" height="{height}" xmlns="http://www.w3.org/2000/svg">\n'

    # Title
    svg += f'  <text x="{width/2}" y="20" text-anchor="middle" font-size="18" font-weight="bold">Memory Usage by Category (MB)</text>\n'

    # Y-axis grid lines (dynamic step based on max memory)
    step = 50 if max_memory > 150 else (20 if max_memory > 50 else 10)
    for i in range(0, int(max_memory) + step, step):
        y = height - margin["bottom"] - i * y_scale
        svg += f'  <line x1="{margin["left"]}" y1="{y}" x2="{width - margin["right"]}" y2="{y}" class="grid-line"/>\n'
        svg += f'  <text x="{margin["left"] - 10}" y="{y + 4}" text-anchor="end" font-size="12" fill="#666">{i}</text>\n'

    # Bars with error bars (min/max range)
    for i, stat in enumerate(category_stats):
        x = margin["left"] + i * bar_spacing + (bar_spacing - bar_width) / 2
        bar_height_px = stat["avg"] * y_scale
        y = height - margin["bottom"] - bar_height_px
        color = colors.get(stat["category"], "#999")

        # Average bar
        svg += f'  <rect x="{x}" y="{y}" width="{bar_width}" height="{bar_height_px}" fill="{color}" class="bar" opacity="0.85">\n'
        svg += f'    <title>{stat["category"]}: {stat["avg"]:.2f} MB (range: {stat["min"]:.2f}-{stat["max"]:.2f} MB)</title>\n'
        svg += f'  </rect>\n'

        # Min/max error bars
        min_y = height - margin["bottom"] - stat["min"] * y_scale
        max_y = height - margin["bottom"] - stat["max"] * y_scale
        center_x = x + bar_width / 2

        svg += f'  <line x1="{center_x}" y1="{min_y}" x2="{center_x}" y2="{max_y}" stroke="#333" stroke-width="2"/>\n'
        svg += f'  <line x1="{center_x - 5}" y1="{min_y}" x2="{center_x + 5}" y2="{min_y}" stroke="#333" stroke-width="2"/>\n'
        svg += f'  <line x1="{center_x - 5}" y1="{max_y}" x2="{center_x + 5}" y2="{max_y}" stroke="#333" stroke-width="2"/>\n'

        # Category label
        svg += f'  <text x="{x + bar_width/2}" y="{height - margin["bottom"] + 20}" text-anchor="middle" font-size="12" fill="#333">{stat["category"]}</text>\n'

        # Average value label
        svg += f'  <text x="{x + bar_width/2}" y="{y - 5}" text-anchor="middle" font-size="11" fill="#666">{stat["avg"]:.2f}</text>\n'

    # Legend
    svg += f'  <text x="{width - margin["right"] - 150}" y="{margin["top"]}" font-size="11" fill="#666">Bars show average, error bars show min/max range</text>\n'

    svg += '</svg>\n'
    return svg

def generate_concurrency_chart():
    """Generate concurrency scaling efficiency chart."""
    data = BENCHMARK_DATA["concurrency_scaling"]

    # SVG dimensions
    width, height = 1000, 600
    margin = {"top": 60, "right": 100, "bottom": 80, "left": 80}
    chart_width = width - margin["left"] - margin["right"]
    chart_height = height - margin["top"] - margin["bottom"]

    # Scale
    max_workers = max(d["workers"] for d in data)
    max_speedup = max(d["speedup"] for d in data)
    max_efficiency = 100

    x_scale = chart_width / max_workers
    y_speedup_scale = chart_height / (max_speedup + 0.5)
    y_efficiency_scale = chart_height / max_efficiency

    svg = f'<svg width="{width}" height="{height}" xmlns="http://www.w3.org/2000/svg">\n'

    # Title
    svg += f'  <text x="{width/2}" y="25" text-anchor="middle" font-size="18" font-weight="bold">Concurrency Scaling Efficiency</text>\n'
    svg += f'  <text x="{width/2}" y="45" text-anchor="middle" font-size="12" fill="#666">Keyframes extraction with 8 test files</text>\n'

    # Y-axis grid lines (speedup)
    for i in range(0, int(max_speedup) + 2):
        y = height - margin["bottom"] - i * y_speedup_scale
        svg += f'  <line x1="{margin["left"]}" y1="{y}" x2="{width - margin["right"]}" y2="{y}" class="grid-line"/>\n'
        svg += f'  <text x="{margin["left"] - 10}" y="{y + 4}" text-anchor="end" font-size="12" fill="#007AFF">{i}x</text>\n'

    # X-axis labels
    for d in data:
        x = margin["left"] + d["workers"] * x_scale
        svg += f'  <text x="{x}" y="{height - margin["bottom"] + 25}" text-anchor="middle" font-size="12" fill="#333">{d["workers"]}</text>\n'

    svg += f'  <text x="{width/2}" y="{height - margin["bottom"] + 55}" text-anchor="middle" font-size="14" fill="#333">Number of Workers</text>\n'
    svg += f'  <text x="{margin["left"] - 60}" y="{height/2}" text-anchor="middle" font-size="14" fill="#007AFF" transform="rotate(-90, {margin["left"] - 60}, {height/2})">Speedup (x baseline)</text>\n'
    svg += f'  <text x="{width - margin["right"] + 60}" y="{height/2}" text-anchor="middle" font-size="14" fill="#34C759" transform="rotate(90, {width - margin["right"] + 60}, {height/2})">Parallel Efficiency (%)</text>\n'

    # Speedup line
    svg += '  <polyline points="'
    for d in data:
        x = margin["left"] + d["workers"] * x_scale
        y = height - margin["bottom"] - d["speedup"] * y_speedup_scale
        svg += f'{x},{y} '
    svg += '" fill="none" stroke="#007AFF" stroke-width="3"/>\n'

    # Speedup points
    for d in data:
        x = margin["left"] + d["workers"] * x_scale
        y = height - margin["bottom"] - d["speedup"] * y_speedup_scale
        svg += f'  <circle cx="{x}" cy="{y}" r="6" fill="#007AFF">\n'
        svg += f'    <title>{d["workers"]} workers: {d["speedup"]:.2f}x speedup</title>\n'
        svg += f'  </circle>\n'
        svg += f'  <text x="{x}" y="{y - 15}" text-anchor="middle" font-size="11" fill="#007AFF" font-weight="bold">{d["speedup"]:.2f}x</text>\n'

    # Efficiency line
    svg += '  <polyline points="'
    for d in data:
        x = margin["left"] + d["workers"] * x_scale
        y = height - margin["bottom"] - d["efficiency_pct"] * y_efficiency_scale
        svg += f'{x},{y} '
    svg += '" fill="none" stroke="#34C759" stroke-width="3" stroke-dasharray="5,5"/>\n'

    # Efficiency points
    for d in data:
        x = margin["left"] + d["workers"] * x_scale
        y = height - margin["bottom"] - d["efficiency_pct"] * y_efficiency_scale
        svg += f'  <circle cx="{x}" cy="{y}" r="6" fill="#34C759">\n'
        svg += f'    <title>{d["workers"]} workers: {d["efficiency_pct"]:.1f}% efficiency</title>\n'
        svg += f'  </circle>\n'
        svg += f'  <text x="{x}" y="{y + 20}" text-anchor="middle" font-size="11" fill="#34C759" font-weight="bold">{d["efficiency_pct"]:.1f}%</text>\n'

    # Legend
    legend_x = width - margin["right"] - 170
    legend_y = margin["top"] + 20
    svg += f'  <line x1="{legend_x}" y1="{legend_y}" x2="{legend_x + 30}" y2="{legend_y}" stroke="#007AFF" stroke-width="3"/>\n'
    svg += f'  <text x="{legend_x + 40}" y="{legend_y + 4}" font-size="12" fill="#333">Speedup (vs 1 worker)</text>\n'
    svg += f'  <line x1="{legend_x}" y1="{legend_y + 20}" x2="{legend_x + 30}" y2="{legend_y + 20}" stroke="#34C759" stroke-width="3" stroke-dasharray="5,5"/>\n'
    svg += f'  <text x="{legend_x + 40}" y="{legend_y + 24}" font-size="12" fill="#333">Parallel Efficiency</text>\n'

    svg += '</svg>\n'
    return svg

def main():
    """Generate all performance charts."""
    output_dir = Path("docs/charts")
    output_dir.mkdir(exist_ok=True)

    # Generate HTML file with all charts
    html_content = generate_html_header()

    html_content += '    <div class="chart-container">\n'
    html_content += '        <h2>1. Throughput Comparison</h2>\n'
    html_content += '        <p>Operations sorted by throughput (MB/s). Higher is better. Only operations with meaningful throughput metrics shown.</p>\n'
    html_content += generate_throughput_chart()
    html_content += '    </div>\n'

    html_content += '    <div class="chart-container">\n'
    html_content += '        <h2>2. Latency Distribution</h2>\n'
    html_content += '        <p>Operations sorted by latency (milliseconds). Lower is better. All 22 benchmarked operations shown.</p>\n'
    html_content += generate_latency_chart()
    html_content += '    </div>\n'

    html_content += '    <div class="chart-container">\n'
    html_content += '        <h2>3. Memory Usage by Category</h2>\n'
    html_content += '        <p>Average memory usage per operation category with min/max ranges. Most operations use ~14-18 MB, with heavier ML operations (diarization, pose estimation, profanity detection) requiring 70-309 MB.</p>\n'
    html_content += generate_memory_chart()
    html_content += '    </div>\n'

    html_content += '    <div class="chart-container">\n'
    html_content += '        <h2>4. Concurrency Scaling Efficiency</h2>\n'
    html_content += '        <p>Speedup and parallel efficiency vs number of workers. Best efficiency at 2 workers (86%), best absolute speedup at 8 workers (2.1x).</p>\n'
    html_content += generate_concurrency_chart()
    html_content += '    </div>\n'

    html_content += generate_html_footer()

    # Write HTML file
    html_path = output_dir / "performance_charts.html"
    with open(html_path, "w") as f:
        f.write(html_content)

    print(f"✅ Performance charts generated: {html_path}")
    print(f"   Open in browser: open {html_path}")

    # Generate individual SVG files
    svg_files = {
        "throughput_comparison.svg": generate_throughput_chart(),
        "latency_distribution.svg": generate_latency_chart(),
        "memory_usage.svg": generate_memory_chart(),
        "concurrency_scaling.svg": generate_concurrency_chart(),
    }

    for filename, svg_content in svg_files.items():
        svg_path = output_dir / filename
        with open(svg_path, "w") as f:
            f.write(svg_content)
        print(f"✅ Generated: {svg_path}")

    print(f"\n📊 All charts generated successfully in {output_dir}/")

if __name__ == "__main__":
    main()
