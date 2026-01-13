#!/usr/bin/env python3
"""
Create verified test file inventory for AI verification.

Enumerates all test files in test_files_wikimedia/ and organizes them by:
- Format (jpg, png, webp, gif, mp3, wav, mp4, heic)
- Operation (object-detection, face-detection, emotion-detection, transcription, etc.)

Output: docs/ai-verification/VERIFIED_TEST_FILES.json
"""

import json
import os
from pathlib import Path
from collections import defaultdict
from typing import Dict, List

def create_test_inventory() -> Dict[str, Dict[str, List[str]]]:
    """
    Enumerate all test files and organize by format and operation.

    Returns:
        Dictionary structure: {format: {operation: [file1, file2, ...]}}
    """
    inventory = defaultdict(lambda: defaultdict(list))

    test_dir = Path("test_files_wikimedia")
    if not test_dir.exists():
        raise FileNotFoundError(f"Test directory not found: {test_dir}")

    # Find all media files
    extensions = [".jpg", ".png", ".webp", ".gif", ".mp3", ".wav", ".mp4", ".heic"]

    for ext in extensions:
        # Remove the leading dot for format name
        format_name = ext.lstrip(".")

        # Find all files with this extension
        files = list(test_dir.rglob(f"*{ext}"))

        for file_path in files:
            # Extract operation from path structure
            # Expected: test_files_wikimedia/{format}/{operation}/{filename}
            parts = file_path.parts

            if len(parts) >= 3 and parts[0] == "test_files_wikimedia":
                operation = parts[2] if len(parts) >= 3 else "unknown"
                relative_path = str(file_path)

                inventory[format_name][operation].append(relative_path)

    # Convert defaultdict to regular dict for JSON serialization
    return {fmt: dict(ops) for fmt, ops in inventory.items()}


def select_diverse_samples(inventory: Dict[str, Dict[str, List[str]]],
                          samples_per_format: int = 5) -> List[Dict[str, str]]:
    """
    Select diverse test samples for verification.

    Args:
        inventory: Test file inventory
        samples_per_format: Number of samples to select per format

    Returns:
        List of test samples with format, operation, and file path
    """
    samples = []

    for format_name, operations in sorted(inventory.items()):
        for operation, files in sorted(operations.items()):
            # Skip operations with fewer than 2 files
            if len(files) < 2:
                continue

            # Select first N files (deterministic sampling)
            selected = files[:min(samples_per_format, len(files))]

            for file_path in selected:
                samples.append({
                    "format": format_name,
                    "operation": operation,
                    "file": file_path
                })

    return samples


def main():
    print("Creating test file inventory...")

    inventory = create_test_inventory()

    # Print statistics
    total_files = sum(len(files) for ops in inventory.values() for files in ops.values())
    print(f"\nInventory Statistics:")
    print(f"Total files: {total_files}")
    print(f"Formats: {len(inventory)}")

    for format_name, operations in sorted(inventory.items()):
        format_total = sum(len(files) for files in operations.values())
        print(f"  {format_name}: {format_total} files across {len(operations)} operations")

    # Save full inventory
    output_path = Path("docs/ai-verification/VERIFIED_TEST_FILES.json")
    output_path.parent.mkdir(parents=True, exist_ok=True)

    with open(output_path, "w") as f:
        json.dump(inventory, f, indent=2, sort_keys=True)

    print(f"\nFull inventory saved to: {output_path}")

    # Select diverse samples for Phase 2
    samples = select_diverse_samples(inventory, samples_per_format=2)
    print(f"\nSelected {len(samples)} diverse samples for verification")

    # Save samples
    samples_path = Path("docs/ai-verification/PHASE2_SAMPLES.json")
    with open(samples_path, "w") as f:
        json.dump(samples, f, indent=2)

    print(f"Phase 2 samples saved to: {samples_path}")

    # Print first 10 samples as example
    print("\nFirst 10 samples:")
    for i, sample in enumerate(samples[:10], 1):
        print(f"  {i}. {sample['format']:5s} | {sample['operation']:25s} | {sample['file']}")


if __name__ == "__main__":
    main()
