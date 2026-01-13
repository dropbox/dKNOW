#!/usr/bin/env python3
"""
Regenerate metadata.json files for directories with reused files.
This fixes paths that point to source directories instead of actual locations.
"""

import json
import os
from pathlib import Path


def regenerate_metadata(directory):
    """Regenerate metadata.json for a directory based on actual files present."""
    dir_path = Path(directory)

    if not dir_path.exists():
        print(f"Directory does not exist: {directory}")
        return

    # Find all media files (not metadata.json)
    media_files = []
    for ext in [".jpg", ".png", ".webm", ".wav", ".flac", ".mp4", ".mov"]:
        media_files.extend(dir_path.glob(f"*{ext}"))

    media_files = sorted(media_files)

    if not media_files:
        print(f"No media files found in {directory}")
        return

    # Load existing metadata to get URLs and other info
    metadata_path = dir_path / "metadata.json"
    existing_metadata = {}

    if metadata_path.exists():
        try:
            with open(metadata_path, "r") as f:
                existing_entries = json.load(f)
                # Index by filename
                for entry in existing_entries:
                    if "path" in entry:
                        filename = Path(entry["path"]).name
                        existing_metadata[filename] = entry
        except Exception as e:
            print(
                f"Warning: Could not load existing metadata from {metadata_path}: {e}"
            )

    # Build new metadata entries
    new_metadata = []
    for media_file in media_files:
        filename = media_file.name

        # Use existing metadata if available, otherwise create new entry
        if filename in existing_metadata:
            entry = existing_metadata[filename].copy()
            # Fix the path to point to actual location
            entry["path"] = str(media_file)
        else:
            # Create minimal entry
            entry = {
                "path": str(media_file),
                "size": media_file.stat().st_size,
                "url": None,  # Unknown for reused files
                "title": filename,
                "mime": get_mime_type(filename),
                "format": media_file.suffix[1:],  # Remove leading dot
            }

        new_metadata.append(entry)

    # Write new metadata.json
    with open(metadata_path, "w") as f:
        json.dump(new_metadata, f, indent=2)

    print(f"✓ Regenerated {metadata_path} ({len(new_metadata)} entries)")


def get_mime_type(filename):
    """Get MIME type from filename extension."""
    ext = Path(filename).suffix.lower()
    mime_types = {
        ".jpg": "image/jpeg",
        ".jpeg": "image/jpeg",
        ".png": "image/png",
        ".webm": "video/webm",
        ".wav": "audio/wav",
        ".flac": "audio/flac",
        ".mp4": "video/mp4",
        ".mov": "video/quicktime",
    }
    return mime_types.get(ext, "application/octet-stream")


if __name__ == "__main__":
    # List of directories created via file reuse (N=254)
    reused_directories = [
        "test_files_wikimedia/wav/audio-enhancement-metadata",
        "test_files_wikimedia/webm/format-conversion",
        "test_files_wikimedia/webm/subtitle-extraction",
        "test_files_wikimedia/webm/shot-classification",
        "test_files_wikimedia/png/content-moderation",
        "test_files_wikimedia/png/shot-classification",
        "test_files_wikimedia/png/depth-estimation",
        "test_files_wikimedia/png/logo-detection",
        "test_files_wikimedia/png/caption-generation",
        "test_files_wikimedia/jpg/content-moderation",
        "test_files_wikimedia/jpg/shot-classification",
        "test_files_wikimedia/jpg/depth-estimation",
        "test_files_wikimedia/jpg/logo-detection",
        "test_files_wikimedia/jpg/caption-generation",
        # Also check diarization and metadata-extraction directories
        "test_files_wikimedia/wav/diarization",
        "test_files_wikimedia/wav/metadata-extraction",
        "test_files_wikimedia/flac/diarization",
        "test_files_wikimedia/flac/metadata-extraction",
        "test_files_wikimedia/webm/metadata-extraction",
    ]

    print(f"Regenerating metadata.json for {len(reused_directories)} directories...")
    print("=" * 60)

    for directory in reused_directories:
        regenerate_metadata(directory)

    print("=" * 60)
    print("Done!")
