#!/usr/bin/env python3
"""Extract and analyze LLM test scores from multiple runs."""

import re
import statistics
from collections import defaultdict

def extract_scores(filename):
    """Extract format scores from a test run file."""
    scores = {}
    with open(filename, 'r') as f:
        content = f.read()

    # Pattern matches both mode3 and verification tests
    # Examples: "test test_llm_mode3_bmp" or "test test_llm_verification_docx"
    # Followed by "Overall Score: 95.0%"
    pattern = r'===\s+(\w+)\s+(?:Mode 3 )?Quality Verification\s+===.*?Overall Score:\s+([\d.]+)%'
    matches = re.findall(pattern, content, re.DOTALL)

    for fmt, score in matches:
        scores[fmt.upper()] = float(score)

    return scores

def main():
    # Extract scores from each run
    runs = []
    for i in [1, 2, 3]:
        filename = f'llm_run_{i}.txt'
        try:
            scores = extract_scores(filename)
            runs.append(scores)
            print(f"Run {i}: {len(scores)} formats extracted")
        except FileNotFoundError:
            print(f"Error: {filename} not found")
            return

    # Combine all formats
    all_formats = set()
    for run in runs:
        all_formats.update(run.keys())
    all_formats = sorted(all_formats)

    print(f"\nTotal formats: {len(all_formats)}")
    print("\n{'Format':<15} {'Run1':>6} {'Run2':>6} {'Run3':>6} {'Mean':>6} {'StdDev':>7} {'Range':>6} {'Status':>7}")
    print("=" * 80)

    variance_high = []
    variance_low = []

    for fmt in all_formats:
        scores_list = [run.get(fmt, 0) for run in runs if fmt in run]

        if len(scores_list) < 3:
            print(f"{fmt:<15} {'N/A':>6} {'N/A':>6} {'N/A':>6} {'N/A':>6} {'N/A':>7} {'N/A':>6} {'MISSING':>7}")
            continue

        mean = statistics.mean(scores_list)
        stdev = statistics.stdev(scores_list) if len(scores_list) > 1 else 0
        range_val = max(scores_list) - min(scores_list)

        # Status: PASS if mean >= 95%, FAIL otherwise
        status = "PASS" if mean >= 95.0 else "FAIL"

        # Track variance
        if stdev > 5.0:
            variance_high.append((fmt, stdev))
        else:
            variance_low.append((fmt, stdev))

        print(f"{fmt:<15} {scores_list[0]:>6.1f} {scores_list[1]:>6.1f} {scores_list[2]:>6.1f} {mean:>6.1f} {stdev:>7.2f} {range_val:>6.1f} {status:>7}")

    print("\n" + "=" * 80)
    print(f"\nVariance Analysis:")
    print(f"  High variance (>5%): {len(variance_high)} formats")
    print(f"  Low variance (<=5%): {len(variance_low)} formats")

    if variance_high:
        print(f"\n  High variance formats:")
        for fmt, stdev in sorted(variance_high, key=lambda x: x[1], reverse=True):
            print(f"    {fmt}: σ={stdev:.2f}%")

    # Count passing/failing
    passing = sum(1 for fmt in all_formats
                  if len([r.get(fmt, 0) for r in runs if fmt in r]) == 3
                  and statistics.mean([r.get(fmt, 0) for r in runs if fmt in r]) >= 95.0)
    failing = len(all_formats) - passing

    print(f"\n  Passing (mean ≥95%): {passing}/{len(all_formats)} ({100*passing//len(all_formats)}%)")
    print(f"  Failing (mean <95%):  {failing}/{len(all_formats)} ({100*failing//len(all_formats)}%)")

if __name__ == '__main__':
    main()
