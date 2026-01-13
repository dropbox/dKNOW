#!/usr/bin/env python3
"""
Identify unreviewed tests and sample 25 for manual semantic review
"""

import csv
import json
import random
from collections import defaultdict

# Load reviewed tests from Tier 1, 2, 3
def load_reviewed_tests():
    reviewed = set()

    # Tier 1
    try:
        with open('docs/ai-output-review/output_review_tier1.csv', 'r') as f:
            reader = csv.DictReader(f)
            for row in reader:
                reviewed.add(row['test_name'])
    except FileNotFoundError:
        pass

    # Tier 2
    try:
        with open('docs/ai-output-review/output_review_tier2.csv', 'r') as f:
            reader = csv.DictReader(f)
            for row in reader:
                reviewed.add(row['test_name'])
    except FileNotFoundError:
        pass

    # Tier 3: Extract from summary (test names mentioned)
    # These are approximate - we'll use operation-based matching
    tier3_operations = [
        'scene-detection', 'action-recognition', 'emotion-detection',
        'pose-estimation', 'ocr', 'shot-classification', 'smart-thumbnail',
        'vision-embeddings', 'image-quality-assessment', 'duplicate-detection',
        'metadata-extraction', 'voice-activity-detection', 'subtitle-extraction',
        'audio-extraction', 'audio-enhancement-metadata', 'text-embeddings'
    ]

    return reviewed, tier3_operations

# Load all tests
def load_all_tests():
    tests = []
    with open('test_results/latest/test_results.csv', 'r') as f:
        reader = csv.DictReader(f)
        for row in reader:
            tests.append(row)
    return tests

def main():
    reviewed_tests, tier3_ops = load_reviewed_tests()
    all_tests = load_all_tests()

    print(f"Total tests: {len(all_tests)}")
    print(f"Explicitly reviewed (Tier 1+2): {len(reviewed_tests)}")
    print()

    # Identify unreviewed tests
    # A test is considered reviewed if:
    # 1. It's in the reviewed_tests set (Tier 1+2)
    # 2. Its primary operation is in tier3_ops (Tier 3)

    unreviewed = []
    reviewed_count = 0

    for test in all_tests:
        test_name = test['test_name']
        operation = test['operation']

        # Check if explicitly reviewed
        if test_name in reviewed_tests:
            reviewed_count += 1
            continue

        # Check if operation was covered in Tier 3
        primary_op = operation.split(';')[0] if ';' in operation else operation
        if primary_op in tier3_ops or operation in tier3_ops:
            reviewed_count += 1
            continue

        # Check composite operations (e.g., "keyframes;object-detection")
        if ';' in operation:
            parts = operation.split(';')
            if all(p in tier3_ops for p in parts):
                reviewed_count += 1
                continue

        unreviewed.append(test)

    print(f"Reviewed (based on operation coverage): {reviewed_count}")
    print(f"Unreviewed: {len(unreviewed)}")
    print()

    # Group unreviewed by operation
    by_operation = defaultdict(list)
    for test in unreviewed:
        by_operation[test['operation']].append(test)

    print("Unreviewed tests by operation:")
    for op in sorted(by_operation.keys()):
        print(f"  {op}: {len(by_operation[op])} tests")
    print()

    # Sample 25 tests (or all if < 25)
    sample_size = min(25, len(unreviewed))

    # Stratified sampling: try to get diverse operations
    sampled = []
    operations_to_sample = list(by_operation.keys())
    random.seed(42)  # Reproducible sampling

    # Round-robin sampling from operations
    while len(sampled) < sample_size and len(operations_to_sample) > 0:
        for op in operations_to_sample[:]:
            if len(sampled) >= sample_size:
                break
            if len(by_operation[op]) > 0:
                test = by_operation[op].pop(0)
                sampled.append(test)
            else:
                operations_to_sample.remove(op)

    print(f"Sampled {len(sampled)} tests for manual review:")
    print()

    # Print sampled tests
    for i, test in enumerate(sampled, 1):
        print(f"{i:2d}. {test['test_name']}")
        print(f"    Operation: {test['operation']}")
        print(f"    File: {test['file_path']}")

        # Parse and show brief metadata
        metadata_json = test.get('output_metadata_json', '')
        if metadata_json:
            try:
                metadata = json.loads(metadata_json)
                # Show first-level keys
                if isinstance(metadata, dict):
                    keys = list(metadata.keys())[:5]
                    print(f"    Metadata keys: {', '.join(keys)}")
                elif isinstance(metadata, list):
                    print(f"    Metadata: array with {len(metadata)} items")
            except:
                print(f"    Metadata: (parse error)")
        print()

    # Save sample to file for easy reference
    with open('sampled_tests_for_review.csv', 'w') as f:
        writer = csv.DictWriter(f, fieldnames=sampled[0].keys())
        writer.writeheader()
        writer.writerows(sampled)

    print(f"Saved sampled tests to: sampled_tests_for_review.csv")

if __name__ == '__main__':
    main()
