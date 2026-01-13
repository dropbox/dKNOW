#!/usr/bin/env python3
"""
Combine multiple extraction files with SMT assertion sampling.

SMT assertions dominate the extracted data (~150M), but we want a balanced
dataset. This script keeps all non-SMT pairs and samples SMT assertions
down to a reasonable number using reservoir sampling.
"""

import argparse
import json
import random
from collections import Counter
from pathlib import Path


def reservoir_sample(iterator, k):
    """Reservoir sampling for streaming data."""
    reservoir = []
    for i, item in enumerate(iterator):
        if i < k:
            reservoir.append(item)
        else:
            j = random.randint(0, i)
            if j < k:
                reservoir[j] = item
    return reservoir


def main():
    parser = argparse.ArgumentParser(description="Combine extractions with SMT sampling")
    parser.add_argument("inputs", nargs="+", help="Input JSONL files")
    parser.add_argument("-o", "--output", required=True, help="Output JSONL file")
    parser.add_argument("--max-smt", type=int, default=100000, help="Max SMT assertions to keep")
    parser.add_argument("--seed", type=int, default=42, help="Random seed")
    args = parser.parse_args()

    random.seed(args.seed)

    # Collect all pairs, streaming SMT for sampling
    print("Processing input files...")
    non_smt_pairs = []
    smt_assertions = []  # Will be sampled via reservoir sampling

    lang_counts = Counter()
    source_counts = Counter()
    smt_count = 0
    file_count = 0

    for input_file in args.inputs:
        print(f"  Reading {input_file}...")
        with open(input_file) as f:
            for line_num, line in enumerate(f):
                try:
                    d = json.loads(line)
                except json.JSONDecodeError:
                    continue

                source = d.get("source", "")
                lang = d.get("language", "")

                if source == "assert" and lang == "smt":
                    # Use reservoir sampling for SMT assertions
                    smt_count += 1
                    if len(smt_assertions) < args.max_smt:
                        smt_assertions.append(d)
                    else:
                        j = random.randint(0, smt_count - 1)
                        if j < args.max_smt:
                            smt_assertions[j] = d
                else:
                    # Keep all non-SMT pairs
                    non_smt_pairs.append(d)
                    lang_counts[lang] += 1
                    source_counts[source] += 1

                file_count += 1
                if file_count % 10000000 == 0:
                    print(f"    Processed {file_count/1000000:.0f}M lines...")

    print(f"\nFound {len(non_smt_pairs):,} non-SMT pairs")
    print(f"Found {smt_count:,} SMT assertions, sampled {len(smt_assertions):,}")

    # Update counts with sampled SMT
    lang_counts["smt"] += len(smt_assertions)
    source_counts["assert"] += len(smt_assertions)

    # Combine and shuffle
    print("\nCombining and shuffling...")
    all_pairs = non_smt_pairs + smt_assertions
    random.shuffle(all_pairs)

    # Write output
    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)

    print(f"\nWriting {len(all_pairs):,} pairs to {output_path}")
    with open(output_path, "w") as f:
        for pair in all_pairs:
            f.write(json.dumps(pair) + "\n")

    print("\nBy language:")
    for lang, count in lang_counts.most_common(20):
        print(f"  {lang}: {count:,}")

    print("\nBy source (top 15):")
    for source, count in source_counts.most_common(15):
        print(f"  {source}: {count:,}")

    print(f"\nTotal: {len(all_pairs):,} training pairs")


if __name__ == "__main__":
    main()
