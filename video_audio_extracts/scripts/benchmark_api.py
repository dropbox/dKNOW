#!/usr/bin/env python3
"""
Performance benchmark script for video-audio-extracts API server.

Tests real-time and bulk processing modes with Kinetics-600 dataset.
Measures throughput, latency, and resource utilization.
"""

import argparse
import json
import time
import requests
import sys
from pathlib import Path
from typing import List, Dict, Any
from datetime import datetime
import statistics

class BenchmarkMetrics:
    """Container for benchmark results."""

    def __init__(self):
        self.total_files = 0
        self.successful = 0
        self.failed = 0
        self.processing_times = []  # seconds per file
        self.start_time = None
        self.end_time = None
        self.errors = []

    def add_result(self, processing_time: float, success: bool, error: str = None):
        """Record result for a single file."""
        self.total_files += 1
        if success:
            self.successful += 1
            self.processing_times.append(processing_time)
        else:
            self.failed += 1
            if error:
                self.errors.append(error)

    def get_summary(self) -> Dict[str, Any]:
        """Calculate summary statistics."""
        total_time = self.end_time - self.start_time if self.end_time and self.start_time else 0

        summary = {
            "total_files": self.total_files,
            "successful": self.successful,
            "failed": self.failed,
            "total_time_seconds": round(total_time, 2),
            "errors": self.errors[:10] if self.errors else []
        }

        if self.processing_times and total_time > 0:
            summary.update({
                "throughput_files_per_second": round(self.successful / total_time, 2),
                "latency_mean_seconds": round(statistics.mean(self.processing_times), 2),
                "latency_median_seconds": round(statistics.median(self.processing_times), 2),
                "latency_min_seconds": round(min(self.processing_times), 2),
                "latency_max_seconds": round(max(self.processing_times), 2),
                "latency_stddev_seconds": round(statistics.stdev(self.processing_times), 2) if len(self.processing_times) > 1 else 0,
            })
        else:
            summary.update({
                "throughput_files_per_second": 0,
                "latency_mean_seconds": 0,
                "latency_median_seconds": 0,
                "latency_min_seconds": 0,
                "latency_max_seconds": 0,
                "latency_stddev_seconds": 0,
            })

        return summary


def find_video_files(dataset_path: Path, limit: int) -> List[Path]:
    """Find video files in Kinetics-600 dataset."""
    video_files = []

    if not dataset_path.exists():
        print(f"Error: Dataset path does not exist: {dataset_path}", file=sys.stderr)
        return []

    # Find all .mp4 files recursively
    for video_file in dataset_path.rglob("*.mp4"):
        video_files.append(video_file)
        if len(video_files) >= limit:
            break

    print(f"Found {len(video_files)} video files")
    return video_files


def test_realtime_mode(api_url: str, video_files: List[Path]) -> BenchmarkMetrics:
    """Benchmark real-time processing mode."""
    print(f"\n=== Testing Real-Time Mode ===")
    print(f"Processing {len(video_files)} files sequentially...")

    metrics = BenchmarkMetrics()
    metrics.start_time = time.time()

    for i, video_path in enumerate(video_files, 1):
        print(f"[{i}/{len(video_files)}] Processing: {video_path.name}")

        try:
            file_start = time.time()

            # Submit job
            request_body = {
                "source": {
                    "type": "upload",
                    "location": str(video_path)
                },
                "processing": {
                    "priority": "realtime",
                    "quality_mode": "fast",
                    "required_features": ["transcription", "keyframes"],
                    "optional_features": []
                }
            }

            response = requests.post(
                f"{api_url}/api/v1/process/realtime",
                json=request_body,
                timeout=300
            )

            if response.status_code != 202:
                metrics.add_result(0, False, f"HTTP {response.status_code}: {response.text}")
                continue

            job_data = response.json()
            job_id = job_data.get('job_id')
            if not job_id:
                metrics.add_result(0, False, f"No job_id in response: {response.text}")
                continue

            # Poll for completion
            max_polls = 180  # 3 minutes max
            poll_interval = 1.0

            for poll_count in range(max_polls):
                time.sleep(poll_interval)

                status_response = requests.get(
                    f"{api_url}/api/v1/jobs/{job_id}/status",
                    timeout=10
                )

                if status_response.status_code != 200:
                    metrics.add_result(0, False, f"Status check failed: HTTP {status_response.status_code}")
                    break

                status_data = status_response.json()
                status = status_data.get('status', '')

                if status == 'completed' or status_data.get('status') == 'Completed':
                    processing_time = time.time() - file_start
                    metrics.add_result(processing_time, True)
                    print(f"  ✓ Completed in {processing_time:.2f}s")
                    break
                elif status == 'failed' or status_data.get('status') == 'Failed':
                    error_msg = status_data.get('error', 'Unknown error')
                    metrics.add_result(0, False, error_msg)
                    print(f"  ✗ Failed: {error_msg}")
                    break
            else:
                metrics.add_result(0, False, "Timeout waiting for completion")
                print(f"  ✗ Timeout after {max_polls * poll_interval}s")

        except Exception as e:
            metrics.add_result(0, False, str(e))
            print(f"  ✗ Error: {e}")

    metrics.end_time = time.time()
    return metrics


def test_bulk_mode(api_url: str, video_files: List[Path]) -> BenchmarkMetrics:
    """Benchmark bulk processing mode."""
    print(f"\n=== Testing Bulk Mode ===")
    print(f"Processing {len(video_files)} files in batch...")

    metrics = BenchmarkMetrics()
    metrics.start_time = time.time()

    try:
        # Build bulk request
        batch_id = f"benchmark_{int(time.time())}"
        files_list = []

        for i, video_path in enumerate(video_files):
            files_list.append({
                "id": f"file_{i}",
                "source": {
                    "type": "upload",
                    "location": str(video_path)
                },
                "processing": {
                    "priority": "bulk",
                    "quality_mode": "fast",
                    "required_features": ["transcription", "keyframes"],
                    "optional_features": []
                }
            })

        bulk_request = {
            "batch_id": batch_id,
            "files": files_list,
            "batch_config": {
                "priority": "bulk",
                "optimize_for": "throughput"
            }
        }

        # Submit bulk job
        print(f"Submitting bulk job with {len(files_list)} files...")
        response = requests.post(
            f"{api_url}/api/v1/process/bulk",
            json=bulk_request,
            timeout=30
        )

        if response.status_code != 202:
            print(f"Error: HTTP {response.status_code}: {response.text}", file=sys.stderr)
            metrics.failed = len(video_files)
            metrics.end_time = time.time()
            return metrics

        job_data = response.json()
        batch_id = job_data['batch_id']
        job_ids = job_data['job_ids']
        print(f"Bulk job submitted: {batch_id} with {len(job_ids)} jobs")

        # Poll each job for completion
        max_polls = 600  # 10 minutes max per job
        poll_interval = 1.0
        completed_count = 0
        failed_count = 0
        job_start_times = {}

        # Track start time for each job
        for job_id in job_ids:
            job_start_times[job_id] = time.time()

        # Poll all jobs until all complete
        remaining_jobs = set(job_ids)

        while remaining_jobs and len(remaining_jobs) > 0:
            time.sleep(poll_interval)

            jobs_to_remove = []

            for job_id in remaining_jobs:
                status_response = requests.get(
                    f"{api_url}/api/v1/jobs/{job_id}/status",
                    timeout=10
                )

                if status_response.status_code != 200:
                    continue

                status_data = status_response.json()
                status = status_data.get('status', '')

                if status == 'completed' or status_data.get('status') == 'Completed':
                    processing_time = time.time() - job_start_times[job_id]
                    metrics.add_result(processing_time, True)
                    completed_count += 1
                    jobs_to_remove.append(job_id)
                elif status == 'failed' or status_data.get('status') == 'Failed':
                    error_msg = status_data.get('error', 'Unknown error')
                    metrics.add_result(0, False, error_msg)
                    failed_count += 1
                    jobs_to_remove.append(job_id)

            # Remove completed/failed jobs
            for job_id in jobs_to_remove:
                remaining_jobs.remove(job_id)

            # Print progress
            total = len(job_ids)
            done = completed_count + failed_count
            print(f"  Progress: {done}/{total} done ({completed_count} succeeded, {failed_count} failed, {len(remaining_jobs)} pending)")

            # Timeout check
            if time.time() - metrics.start_time > max_polls * poll_interval:
                print(f"  ✗ Timeout after {max_polls * poll_interval}s", file=sys.stderr)
                metrics.failed += len(remaining_jobs)
                break

        if not remaining_jobs:
            print(f"  ✓ Bulk job completed: {completed_count} succeeded, {failed_count} failed")

    except Exception as e:
        print(f"Error in bulk mode: {e}", file=sys.stderr)
        metrics.failed = len(video_files)
        metrics.errors.append(str(e))

    metrics.end_time = time.time()
    return metrics


def main():
    parser = argparse.ArgumentParser(description='Benchmark video-audio-extracts API server')
    parser.add_argument('--api-url', default='http://localhost:8080', help='API server URL')
    parser.add_argument('--dataset-path', required=True, help='Path to Kinetics-600 dataset')
    parser.add_argument('--num-files', type=int, default=10, help='Number of files to test')
    parser.add_argument('--mode', choices=['realtime', 'bulk', 'both'], default='both', help='Test mode')
    parser.add_argument('--output', help='Output JSON file for results')

    args = parser.parse_args()

    # Find video files
    dataset_path = Path(args.dataset_path)
    video_files = find_video_files(dataset_path, args.num_files)

    if not video_files:
        print("Error: No video files found", file=sys.stderr)
        sys.exit(1)

    # Check if API server is running
    try:
        health_response = requests.get(f"{args.api_url}/health", timeout=5)
        if health_response.status_code != 200:
            print(f"Warning: API server health check returned {health_response.status_code}", file=sys.stderr)
    except requests.exceptions.RequestException as e:
        print(f"Error: Cannot connect to API server at {args.api_url}", file=sys.stderr)
        print(f"Make sure the server is running: cargo run --release --bin api-server", file=sys.stderr)
        sys.exit(1)

    results = {
        "benchmark_time": datetime.now().isoformat(),
        "dataset_path": str(dataset_path),
        "num_files": len(video_files),
        "api_url": args.api_url
    }

    # Run benchmarks
    if args.mode in ['realtime', 'both']:
        realtime_metrics = test_realtime_mode(args.api_url, video_files)
        results['realtime'] = realtime_metrics.get_summary()

        print(f"\n=== Real-Time Mode Results ===")
        print(json.dumps(results['realtime'], indent=2))

    if args.mode in ['bulk', 'both']:
        bulk_metrics = test_bulk_mode(args.api_url, video_files)
        results['bulk'] = bulk_metrics.get_summary()

        print(f"\n=== Bulk Mode Results ===")
        print(json.dumps(results['bulk'], indent=2))

    # Save results
    if args.output:
        output_path = Path(args.output)
        output_path.parent.mkdir(parents=True, exist_ok=True)
        with open(output_path, 'w') as f:
            json.dump(results, f, indent=2)
        print(f"\nResults saved to: {output_path}")

    # Summary
    print(f"\n=== Benchmark Complete ===")
    if 'realtime' in results:
        rt = results['realtime']
        print(f"Real-Time: {rt['successful']}/{rt['total_files']} succeeded, "
              f"{rt['throughput_files_per_second']:.2f} files/sec, "
              f"{rt['latency_mean_seconds']:.2f}s avg latency")

    if 'bulk' in results:
        bk = results['bulk']
        print(f"Bulk: {bk['successful']}/{bk['total_files']} succeeded, "
              f"{bk['throughput_files_per_second']:.2f} files/sec")


if __name__ == '__main__':
    main()
