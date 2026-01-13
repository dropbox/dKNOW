#!/usr/bin/env python3
"""
Baseline benchmark: FFmpeg-only manual workflow.

This script represents a minimal "alternative" approach using just FFmpeg
for audio extraction and keyframe extraction. This provides a lower-bound
baseline to compare against our integrated system.

This measures:
1. Raw FFmpeg performance (audio + keyframe extraction)
2. Sequential processing overhead
3. No orchestration, no parallelization

Note: Transcription is excluded to focus on FFmpeg operations that
both systems perform. This isolates the overhead of our Rust wrapper
and orchestration layer.
"""

import argparse
import json
import time
import subprocess
import sys
import tempfile
import shutil
from pathlib import Path
from typing import List, Dict, Any, Tuple
from datetime import datetime
import statistics


class BaselineMetrics:
    """Container for baseline benchmark results."""

    def __init__(self):
        self.total_files = 0
        self.successful = 0
        self.failed = 0
        self.processing_times = []
        self.audio_extraction_times = []
        self.keyframe_extraction_times = []
        self.start_time = None
        self.end_time = None
        self.errors = []

    def add_result(
        self,
        processing_time: float,
        success: bool,
        audio_time: float = 0,
        keyframe_time: float = 0,
        error: str = None
    ):
        """Record result for a single file."""
        self.total_files += 1
        if success:
            self.successful += 1
            self.processing_times.append(processing_time)
            self.audio_extraction_times.append(audio_time)
            self.keyframe_extraction_times.append(keyframe_time)
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
                # Per-task breakdowns
                "audio_extraction_mean_seconds": round(statistics.mean(self.audio_extraction_times), 2) if self.audio_extraction_times else 0,
                "keyframe_extraction_mean_seconds": round(statistics.mean(self.keyframe_extraction_times), 2) if self.keyframe_extraction_times else 0,
            })
        else:
            summary.update({
                "throughput_files_per_second": 0,
                "latency_mean_seconds": 0,
                "latency_median_seconds": 0,
                "latency_min_seconds": 0,
                "latency_max_seconds": 0,
                "latency_stddev_seconds": 0,
                "audio_extraction_mean_seconds": 0,
                "keyframe_extraction_mean_seconds": 0,
            })

        return summary


def extract_audio_ffmpeg(video_path: Path, output_dir: Path) -> Tuple[Path, float]:
    """Extract audio using FFmpeg. Returns (output_path, duration)."""
    output_path = output_dir / f"{video_path.stem}.wav"

    cmd = [
        "ffmpeg",
        "-i", str(video_path),
        "-vn",  # No video
        "-acodec", "pcm_s16le",  # 16-bit PCM
        "-ar", "16000",  # 16kHz sample rate
        "-ac", "1",  # Mono
        "-y",  # Overwrite
        str(output_path)
    ]

    start = time.time()
    result = subprocess.run(
        cmd,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        timeout=60
    )
    duration = time.time() - start

    if result.returncode != 0:
        raise RuntimeError(f"FFmpeg audio extraction failed with code {result.returncode}")

    return output_path, duration


def extract_keyframes_ffmpeg(video_path: Path, output_dir: Path) -> Tuple[List[Path], float]:
    """Extract keyframes using FFmpeg. Returns (keyframe_paths, duration)."""
    output_pattern = output_dir / f"{video_path.stem}_frame_%04d.jpg"

    # Extract keyframes at 1 FPS (similar to our system's default)
    cmd = [
        "ffmpeg",
        "-i", str(video_path),
        "-vf", "fps=1",  # 1 frame per second
        "-q:v", "2",  # High quality JPEG
        "-y",
        str(output_pattern)
    ]

    start = time.time()
    result = subprocess.run(
        cmd,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        timeout=60
    )
    duration = time.time() - start

    if result.returncode != 0:
        raise RuntimeError(f"FFmpeg keyframe extraction failed with code {result.returncode}")

    # Find generated frames
    keyframes = sorted(output_dir.glob(f"{video_path.stem}_frame_*.jpg"))
    return keyframes, duration


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


def run_baseline_benchmark(video_files: List[Path]) -> BaselineMetrics:
    """
    Run baseline benchmark using FFmpeg-only manual workflow.
    """
    print(f"\n=== Baseline FFmpeg-Only Workflow Benchmark ===")
    print(f"Processing {len(video_files)} files sequentially...")
    print(f"Tasks: Audio extraction + Keyframe extraction (1 FPS)\n")

    metrics = BaselineMetrics()
    metrics.start_time = time.time()

    # Create temp directory for output files
    temp_dir = Path(tempfile.mkdtemp(prefix="baseline_ffmpeg_"))

    try:
        for i, video_path in enumerate(video_files, 1):
            print(f"[{i}/{len(video_files)}] Processing: {video_path.name}")

            file_start = time.time()

            try:
                # Step 1: Extract audio
                audio_path, audio_time = extract_audio_ffmpeg(video_path, temp_dir)
                print(f"  Audio: {audio_time:.2f}s")

                # Step 2: Extract keyframes
                keyframes, keyframe_time = extract_keyframes_ffmpeg(video_path, temp_dir)
                print(f"  Keyframes: {keyframe_time:.2f}s ({len(keyframes)} frames)")

                # Clean up intermediate files
                audio_path.unlink()
                for kf in keyframes:
                    kf.unlink()

                processing_time = time.time() - file_start
                metrics.add_result(
                    processing_time,
                    True,
                    audio_time,
                    keyframe_time
                )
                print(f"  ✓ Total: {processing_time:.2f}s\n")

            except Exception as e:
                metrics.add_result(0, False, error=str(e))
                print(f"  ✗ Error: {e}\n")

    finally:
        # Clean up temp directory
        shutil.rmtree(temp_dir, ignore_errors=True)

    metrics.end_time = time.time()
    return metrics


def main():
    parser = argparse.ArgumentParser(
        description="Baseline benchmark: FFmpeg-only manual workflow"
    )
    parser.add_argument(
        "--dataset-path",
        type=Path,
        default=Path.home() / "Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/Kinetics dataset (5%)/kinetics600_5per/kinetics600_5per/train",
        help="Path to Kinetics-600 dataset"
    )
    parser.add_argument(
        "--num-files",
        type=int,
        default=10,
        help="Number of files to process (default: 10)"
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("baseline_ffmpeg_results.json"),
        help="Output file for results (default: baseline_ffmpeg_results.json)"
    )

    args = parser.parse_args()

    # Find video files
    video_files = find_video_files(args.dataset_path, args.num_files)
    if not video_files:
        print("No video files found. Exiting.", file=sys.stderr)
        sys.exit(1)

    # Run benchmark
    metrics = run_baseline_benchmark(video_files)

    # Print summary
    summary = metrics.get_summary()
    print("\n=== Baseline Benchmark Results ===")
    print(json.dumps(summary, indent=2))

    # Save to file
    output_data = {
        "benchmark_type": "baseline_ffmpeg_only",
        "timestamp": datetime.now().isoformat(),
        "configuration": {
            "num_files": args.num_files,
            "dataset_path": str(args.dataset_path),
            "tools": ["FFmpeg (audio + keyframes)"],
            "note": "Transcription excluded - FFmpeg operations only"
        },
        "results": summary
    }

    with open(args.output, 'w') as f:
        json.dump(output_data, f, indent=2)

    print(f"\nResults saved to: {args.output}")

    # Exit with error code if any files failed
    if metrics.failed > 0:
        sys.exit(1)


if __name__ == "__main__":
    main()
