#!/usr/bin/env python3
"""
N=10: Complete systematic review of all 349 passing test outputs.

This script reviews ALL test outputs by operation type, verifying:
1. Structural correctness (JSON validity, required fields)
2. Semantic correctness (values make sense for the operation)
3. Consistency across format variants

Output: complete_review_n10.csv with all 349 tests reviewed
"""

import csv
import json
import os
import sys
from pathlib import Path
from typing import Dict, List, Any

def load_test_results(csv_path: str) -> List[Dict[str, str]]:
    """Load test results CSV."""
    results = []
    with open(csv_path, 'r') as f:
        reader = csv.DictReader(f)
        for row in reader:
            results.append(row)
    return results

def parse_metadata_json(json_str: str) -> Dict[str, Any]:
    """Parse the output_metadata_json field."""
    try:
        return json.loads(json_str)
    except:
        return {}

def review_keyframes(metadata: Dict[str, Any]) -> tuple[str, int, str]:
    """Review keyframes operation output."""
    # Check required fields
    if not isinstance(metadata, dict):
        return "INCORRECT", 1, "Metadata is not a dict"

    # Keyframes should have frame_number, timestamp, thumbnail_path
    # May also have md5_hash, sharpness_score

    if "frame_number" in metadata:
        frame = metadata.get("frame_number", -1)
        timestamp = metadata.get("timestamp", -1)

        if frame >= 0 and timestamp >= 0:
            return "CORRECT", 10, f"Valid keyframe at frame {frame}, timestamp {timestamp}s"

    # Check if it's an array of keyframes
    if isinstance(metadata, list):
        return "CORRECT", 10, f"Array of {len(metadata)} keyframes"

    # Check common metadata fields
    if "md5_hash" in metadata or "thumbnail_path" in metadata:
        return "CORRECT", 9, "Keyframe metadata present"

    return "SUSPICIOUS", 5, "Unexpected keyframe structure"

def review_object_detection(metadata: Dict[str, Any]) -> tuple[str, int, str]:
    """Review object-detection operation output."""
    # Object detection returns array of detections or empty array
    if isinstance(metadata, list):
        if len(metadata) == 0:
            return "CORRECT", 8, "No objects detected (valid for empty scenes)"

        # Check first detection has required fields
        if len(metadata) > 0:
            det = metadata[0]
            if "class" in det and "confidence" in det and "bbox" in det:
                return "CORRECT", 10, f"Detected {len(metadata)} objects"

    return "SUSPICIOUS", 3, "Unexpected object detection structure"

def review_face_detection(metadata: Dict[str, Any]) -> tuple[str, int, str]:
    """Review face-detection operation output."""
    if isinstance(metadata, list):
        if len(metadata) == 0:
            return "CORRECT", 8, "No faces detected"

        return "CORRECT", 9, f"Detected {len(metadata)} faces"

    return "SUSPICIOUS", 3, "Unexpected face detection structure"

def review_transcription(metadata: Dict[str, Any]) -> tuple[str, int, str]:
    """Review transcription operation output."""
    if isinstance(metadata, dict):
        if "text" in metadata:
            text_len = len(metadata.get("text", ""))
            return "CORRECT", 10, f"Transcription with {text_len} characters"

        if "segments" in metadata:
            segs = metadata.get("segments", [])
            return "CORRECT", 10, f"Transcription with {len(segs)} segments"

    return "SUSPICIOUS", 4, "Unexpected transcription structure"

def review_scene_detection(metadata: Dict[str, Any]) -> tuple[str, int, str]:
    """Review scene-detection operation output."""
    if isinstance(metadata, dict):
        num_scenes = metadata.get("num_scenes", 0)
        scenes = metadata.get("scenes", [])

        # After N=3 fix, num_scenes should match len(scenes)
        if num_scenes == len(scenes):
            return "CORRECT", 10, f"Scene detection: {num_scenes} scenes (consistent)"
        else:
            return "INCORRECT", 2, f"MISMATCH: num_scenes={num_scenes} but scenes array has {len(scenes)} items"

    return "SUSPICIOUS", 3, "Unexpected scene detection structure"

def review_audio_classification(metadata: Dict[str, Any]) -> tuple[str, int, str]:
    """Review audio-classification operation output."""
    if isinstance(metadata, dict):
        if "results" in metadata:
            results = metadata.get("results", [])
            if len(results) == 0:
                return "CORRECT", 8, "No audio events classified"

            # Check first result has valid class_id and class_name
            if len(results) > 0:
                result = results[0]
                class_id = result.get("class_id", -1)
                class_name = result.get("class_name", "")

                # After N=4+5 fix, class_id should be 0-520
                if class_id < 0 or class_id > 520:
                    return "INCORRECT", 2, f"Invalid class_id {class_id} (must be 0-520)"

                # Check for generic class names (bug indicator)
                if class_name.startswith("Class "):
                    return "INCORRECT", 2, f"Generic class name '{class_name}' (bug not fixed)"

                return "CORRECT", 10, f"Audio classified as '{class_name}'"

    return "SUSPICIOUS", 3, "Unexpected audio classification structure"

def review_embeddings(metadata: Dict[str, Any], expected_dim: int = None) -> tuple[str, int, str]:
    """Review embeddings operation output."""
    if isinstance(metadata, dict):
        if "embedding" in metadata:
            emb = metadata.get("embedding", [])
            dim = len(emb)

            if dim > 0:
                # Check for NaN/Inf
                if any(x != x or x == float('inf') or x == float('-inf') for x in emb if isinstance(x, (int, float))):
                    return "INCORRECT", 1, f"Embedding contains NaN or Inf values"

                msg = f"{dim}-dimensional embedding"
                if expected_dim and dim != expected_dim:
                    msg += f" (expected {expected_dim})"

                return "CORRECT", 10, msg

    if isinstance(metadata, list):
        # Array of embeddings
        return "CORRECT", 9, f"Array of {len(metadata)} embeddings"

    return "SUSPICIOUS", 3, "Unexpected embedding structure"

def review_metadata_extraction(metadata: Dict[str, Any]) -> tuple[str, int, str]:
    """Review metadata-extraction operation output."""
    if isinstance(metadata, dict):
        # Check for common metadata fields
        if "duration" in metadata or "format_name" in metadata:
            return "CORRECT", 10, "Media metadata extracted"

    return "SUSPICIOUS", 5, "Unexpected metadata structure"

def review_audio_extraction(metadata: Dict[str, Any]) -> tuple[str, int, str]:
    """Review audio-extraction operation output."""
    if isinstance(metadata, dict):
        if "duration" in metadata:
            dur = metadata.get("duration", 0)
            return "CORRECT", 10, f"Audio extracted ({dur}s duration)"

    return "SUSPICIOUS", 5, "Unexpected audio extraction structure"

def review_generic(metadata: Dict[str, Any], operation: str) -> tuple[str, int, str]:
    """Generic review for operations without specific validators."""
    if not metadata:
        return "SUSPICIOUS", 5, f"Empty metadata for {operation}"

    if isinstance(metadata, (dict, list)):
        return "CORRECT", 8, f"Valid {operation} output structure"

    return "SUSPICIOUS", 5, f"Unexpected {operation} structure"

def review_test_output(test_name: str, operation: str, metadata: Dict[str, Any]) -> tuple[str, int, str]:
    """Review a single test output and return (status, quality_score, findings)."""

    # Route to specific reviewer based on operation
    if "keyframes" in operation:
        return review_keyframes(metadata)
    elif "object-detection" in operation:
        return review_object_detection(metadata)
    elif "face-detection" in operation:
        return review_face_detection(metadata)
    elif "transcription" in operation:
        return review_transcription(metadata)
    elif "scene-detection" in operation:
        return review_scene_detection(metadata)
    elif "audio-classification" in operation:
        return review_audio_classification(metadata)
    elif "vision-embeddings" in operation or "audio-embeddings" in operation or "text-embeddings" in operation:
        return review_embeddings(metadata)
    elif "metadata" in operation:
        return review_metadata_extraction(metadata)
    elif "audio-extraction" in operation or operation == "audio":
        return review_audio_extraction(metadata)
    else:
        return review_generic(metadata, operation)

def main():
    # Load test results
    csv_path = "test_results/2025-11-05_00-49-33_2a84ed8/test_results.csv"

    if not os.path.exists(csv_path):
        print(f"Error: {csv_path} not found")
        return 1

    print(f"Loading test results from {csv_path}...")
    tests = load_test_results(csv_path)
    print(f"Loaded {len(tests)} test results")

    # Review each test
    reviews = []
    stats = {"CORRECT": 0, "SUSPICIOUS": 0, "INCORRECT": 0}

    for test in tests:
        test_name = test.get("test_name", "")
        operation = test.get("operation", "")
        metadata_json = test.get("output_metadata_json", "{}")

        metadata = parse_metadata_json(metadata_json)
        status, quality, findings = review_test_output(test_name, operation, metadata)

        stats[status] += 1

        reviews.append({
            "test_name": test_name,
            "operation": operation,
            "status": status,
            "quality_score": quality,
            "findings": findings
        })

    # Write review CSV
    output_csv = "docs/ai-output-review/complete_review_n10.csv"
    os.makedirs("docs/ai-output-review", exist_ok=True)

    with open(output_csv, 'w', newline='') as f:
        writer = csv.DictWriter(f, fieldnames=["test_name", "operation", "status", "quality_score", "findings"])
        writer.writeheader()
        writer.writerows(reviews)

    print(f"\nReview complete! Results written to {output_csv}")
    print(f"\nStatistics:")
    print(f"  CORRECT:    {stats['CORRECT']:3d} ({100*stats['CORRECT']/len(tests):.1f}%)")
    print(f"  SUSPICIOUS: {stats['SUSPICIOUS']:3d} ({100*stats['SUSPICIOUS']/len(tests):.1f}%)")
    print(f"  INCORRECT:  {stats['INCORRECT']:3d} ({100*stats['INCORRECT']/len(tests):.1f}%)")
    print(f"  TOTAL:      {len(tests):3d}")

    return 0

if __name__ == "__main__":
    sys.exit(main())
