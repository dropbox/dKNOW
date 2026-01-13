#!/usr/bin/env python3
"""
Convert existing WebM test files to MP4 and MOV formats for format diversity.

This script takes existing WebM files and converts them to MP4/MOV using FFmpeg,
creating format coverage without downloading more large files from Wikimedia.

Usage:
    python3 tools/convert_webm_to_formats.py <feature> <target_format> <count>

Example:
    python3 tools/convert_webm_to_formats.py transcription mp4 3
"""

import subprocess
import sys
import json
from pathlib import Path


def convert_file(input_path, output_path, target_format):
    """Convert a video file using FFmpeg."""

    # Choose codec based on target format
    if target_format == "mp4":
        # H.264 video, AAC audio (most compatible)
        video_codec = "libx264"
        audio_codec = "aac"
    elif target_format == "mov":
        # H.264 video, AAC audio (QuickTime compatible)
        video_codec = "libx264"
        audio_codec = "aac"
    else:
        print(f"❌ Unsupported format: {target_format}")
        return False

    cmd = [
        "ffmpeg",
        "-i",
        str(input_path),
        "-c:v",
        video_codec,
        "-c:a",
        audio_codec,
        "-preset",
        "fast",  # Fast encoding
        "-crf",
        "23",  # Good quality
        "-y",  # Overwrite output
        str(output_path),
    ]

    try:
        print(f"  Converting {input_path.name} → {output_path.name}...")
        result = subprocess.run(cmd, capture_output=True, timeout=300)
        if result.returncode == 0:
            size_mb = output_path.stat().st_size / (1024 * 1024)
            print(f"  ✅ {output_path.name} ({size_mb:.1f} MB)")
            return True
        else:
            print(f"  ❌ FFmpeg error: {result.stderr.decode()[:200]}")
            return False
    except subprocess.TimeoutExpired:
        print(f"  ❌ Timeout converting {input_path.name}")
        return False
    except Exception as e:
        print(f"  ❌ Error: {e}")
        return False


def convert_feature_to_format(feature, target_format, count=3):
    """Convert N WebM files for a feature to target format."""

    webm_dir = Path("test_files_wikimedia/webm") / feature
    target_dir = Path("test_files_wikimedia") / target_format / feature

    if not webm_dir.exists():
        print(f"❌ Source directory not found: {webm_dir}")
        return []

    # Find WebM files
    webm_files = sorted([f for f in webm_dir.glob("*.webm")])

    if not webm_files:
        print(f"❌ No WebM files found in {webm_dir}")
        return []

    # Select first N files
    selected = webm_files[:count]

    print(f"\n{'='*60}")
    print(f"Converting: {feature} (webm → {target_format})")
    print(f"{'='*60}")
    print(f"Source: {webm_dir}")
    print(f"Target: {target_dir}")
    print(f"Files: {len(selected)}/{count}")

    # Create target directory
    target_dir.mkdir(parents=True, exist_ok=True)

    # Convert files
    converted = []
    for webm_file in selected:
        # Keep similar naming: 01_name.webm → 01_name.mp4
        output_name = webm_file.stem + f".{target_format}"
        output_path = target_dir / output_name

        if convert_file(webm_file, output_path, target_format):
            converted.append(
                {
                    "source": str(webm_file),
                    "output": str(output_path),
                    "size": output_path.stat().st_size,
                }
            )

    # Save metadata
    if converted:
        metadata = {
            "conversion_source": "webm",
            "target_format": target_format,
            "feature": feature,
            "files": converted,
        }

        metadata_path = target_dir / "metadata.json"
        with open(metadata_path, "w") as f:
            json.dump(metadata, f, indent=2)

        print(f"\n✅ Converted {len(converted)}/{count} files")
        print(f"📄 Metadata saved: {metadata_path}")

    return converted


if __name__ == "__main__":
    if len(sys.argv) < 4:
        print(
            "Usage: python3 tools/convert_webm_to_formats.py <feature> <format> <count>"
        )
        print("Example: python3 tools/convert_webm_to_formats.py transcription mp4 3")
        sys.exit(1)

    feature = sys.argv[1]
    target_format = sys.argv[2]
    count = int(sys.argv[3])

    converted = convert_feature_to_format(feature, target_format, count)

    print(f"\n{'='*60}")
    print(f"Summary: Converted {len(converted)} files")
    print(f"{'='*60}")
