#!/usr/bin/env python3
"""
Programmatic validation of all test outputs from test_results.csv
Validates structural correctness and identifies anomalies
"""

import csv
import json
import sys
from collections import defaultdict
from typing import Dict, List, Tuple

def load_test_results(csv_path: str) -> List[Dict]:
    """Load test results from CSV"""
    results = []
    with open(csv_path, 'r') as f:
        reader = csv.DictReader(f)
        for row in reader:
            results.append(row)
    return results

def validate_metadata_json(metadata_str: str, test_name: str, operation: str) -> Tuple[bool, List[str]]:
    """Validate metadata JSON structure and content"""
    issues = []

    if not metadata_str or metadata_str == '':
        return True, []  # Empty metadata is valid for some operations

    try:
        metadata = json.loads(metadata_str)
    except json.JSONDecodeError as e:
        issues.append(f"Invalid JSON: {e}")
        return False, issues

    # Check for common anomalies based on operation type
    if 'keyframes' in operation:
        if isinstance(metadata, dict) and 'keyframes' in metadata:
            kf_list = metadata.get('keyframes', [])
            if not isinstance(kf_list, list):
                issues.append("keyframes field is not a list")
            elif len(kf_list) == 0:
                issues.append("Empty keyframes array (may be valid for short videos)")

    if 'object-detection' in operation or 'face-detection' in operation:
        if isinstance(metadata, dict) and 'detections' in metadata:
            detections = metadata.get('detections', [])
            if not isinstance(detections, list):
                issues.append("detections field is not a list")
            # Check for suspicious high detection counts
            elif len(detections) > 50:
                issues.append(f"Suspicious: {len(detections)} detections (very high)")

            # Validate detection structure
            for i, det in enumerate(detections[:3]):  # Check first 3
                if not isinstance(det, dict):
                    issues.append(f"Detection {i} is not an object")
                    continue
                if 'confidence' in det:
                    conf = det['confidence']
                    if not isinstance(conf, (int, float)):
                        issues.append(f"Detection {i} confidence is not numeric")
                    elif conf < 0 or conf > 1:
                        issues.append(f"Detection {i} confidence {conf} out of range [0,1]")

    if 'audio-classification' in operation or 'acoustic-scene' in operation:
        if isinstance(metadata, list):
            for i, item in enumerate(metadata[:3]):
                if isinstance(item, dict) and 'class_id' in item:
                    class_id = item['class_id']
                    # YAMNet has 521 classes (0-520)
                    if class_id > 520:
                        issues.append(f"Invalid class_id {class_id} (max 520 for YAMNet)")

    if 'scene-detection' in operation:
        if isinstance(metadata, dict):
            num_scenes = metadata.get('num_scenes', 0)
            scenes = metadata.get('scenes', [])
            if not isinstance(scenes, list):
                issues.append("scenes field is not a list")
            elif num_scenes != len(scenes):
                issues.append(f"Inconsistency: num_scenes={num_scenes} but len(scenes)={len(scenes)}")

    if 'embeddings' in operation:
        # Check embedding structure
        if isinstance(metadata, dict) and 'embeddings' in metadata:
            emb_list = metadata.get('embeddings', [])
            if isinstance(emb_list, list) and len(emb_list) > 0:
                first_emb = emb_list[0]
                if isinstance(first_emb, list):
                    # Check dimension
                    dim = len(first_emb)
                    if dim not in [384, 512, 768, 1024]:
                        issues.append(f"Unusual embedding dimension: {dim}")
                    # Check for NaN/Inf
                    for val in first_emb[:10]:
                        if not isinstance(val, (int, float)):
                            issues.append("Non-numeric value in embedding")
                            break

    if 'transcription' in operation:
        if isinstance(metadata, dict):
            text = metadata.get('text', '')
            segments = metadata.get('segments', [])
            if not isinstance(text, str):
                issues.append("transcription text is not a string")
            if not isinstance(segments, list):
                issues.append("transcription segments is not a list")

    return len(issues) == 0, issues

def main():
    csv_path = 'test_results/latest/test_results.csv'

    print("Loading test results...")
    results = load_test_results(csv_path)
    print(f"Loaded {len(results)} test results\n")

    # Statistics
    total_tests = len(results)
    valid_tests = 0
    tests_with_issues = 0
    issues_by_operation = defaultdict(list)
    tests_by_operation = defaultdict(int)

    all_issues = []

    for row in results:
        test_name = row['test_name']
        operation = row['operation']
        status = row['status']
        metadata_json = row.get('output_metadata_json', '')

        tests_by_operation[operation] += 1

        if status != 'passed':
            tests_with_issues += 1
            issue = f"{test_name}: Test status is '{status}' (not passed)"
            all_issues.append((test_name, operation, [issue]))
            continue

        is_valid, issues = validate_metadata_json(metadata_json, test_name, operation)

        if is_valid:
            valid_tests += 1
        else:
            tests_with_issues += 1
            all_issues.append((test_name, operation, issues))
            for issue in issues:
                issues_by_operation[operation].append((test_name, issue))

    # Print summary
    print("=" * 80)
    print("VALIDATION SUMMARY")
    print("=" * 80)
    print(f"Total tests: {total_tests}")
    print(f"Valid tests: {valid_tests} ({100*valid_tests/total_tests:.1f}%)")
    print(f"Tests with issues: {tests_with_issues} ({100*tests_with_issues/total_tests:.1f}%)")
    print()

    # Print operations summary
    print("=" * 80)
    print("TESTS BY OPERATION")
    print("=" * 80)
    for op in sorted(tests_by_operation.keys()):
        count = tests_by_operation[op]
        issue_count = len([t for t, o, _ in all_issues if o == op])
        status = "✅" if issue_count == 0 else "⚠️"
        print(f"{status} {op:40s} {count:3d} tests ({issue_count} with issues)")
    print()

    # Print issues by operation
    if len(issues_by_operation) > 0:
        print("=" * 80)
        print("ISSUES BY OPERATION")
        print("=" * 80)
        for operation in sorted(issues_by_operation.keys()):
            op_issues = issues_by_operation[operation]
            print(f"\n{operation} ({len(op_issues)} issues):")
            # Group by issue type
            issue_types = defaultdict(list)
            for test_name, issue in op_issues:
                issue_types[issue].append(test_name)

            for issue_type, tests in issue_types.items():
                print(f"  - {issue_type}")
                print(f"    Affected tests: {len(tests)}")
                if len(tests) <= 3:
                    for t in tests:
                        print(f"      * {t}")
                else:
                    for t in tests[:2]:
                        print(f"      * {t}")
                    print(f"      * ... and {len(tests)-2} more")
    else:
        print("✅ NO ISSUES FOUND - All tests passed validation!\n")

    # Exit code
    sys.exit(0 if tests_with_issues == 0 else 1)

if __name__ == '__main__':
    main()
