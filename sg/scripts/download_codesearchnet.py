#!/usr/bin/env python3
"""
Download CodeSearchNet and convert to training format.

CodeSearchNet: MIT Licensed, 2M+ query-code pairs across 6 languages.
This is the gold standard for code search training.

Uses HuggingFace mirrors (original S3 bucket is deprecated).

Usage:
    python scripts/download_codesearchnet.py --output data/codesearchnet_all.jsonl
    python scripts/download_codesearchnet.py --languages python,java,go --output data/codesearchnet_subset.jsonl
"""

import argparse
import json
from pathlib import Path


# HuggingFace dataset mappings (these are mirrors/preprocessed versions)
HF_DATASETS = {
    "python": "AhmedSSoliman/CodeSearchNet-Python",
    "java": "Nan-Do/code-search-net-java",
    "go": "Nan-Do/code-search-net-go",
    "javascript": "Nan-Do/code-search-net-javascript",
    "php": "Nan-Do/code-search-net-php",
    "ruby": "Nan-Do/code-search-net-ruby",
}


def download_language(lang: str, max_pairs: int | None = None) -> tuple[list[dict], int]:
    """Download and parse CodeSearchNet for a single language from HuggingFace."""
    from datasets import load_dataset

    dataset_name = HF_DATASETS.get(lang)
    if not dataset_name:
        print(f"  No HuggingFace dataset for {lang}")
        return [], 0

    print(f"  Loading {dataset_name}...")

    try:
        ds = load_dataset(dataset_name, split="train")
    except Exception as e:
        print(f"  Error loading {lang}: {e}")
        return [], 0

    print(f"  Processing {len(ds):,} examples...")
    pairs = []
    skipped = 0

    for example in ds:
        if max_pairs and len(pairs) >= max_pairs:
            break

        # Try different field names (datasets have inconsistent naming)
        docstring = (
            example.get("func_documentation_string", "")
            or example.get("docstring", "")
            or example.get("documentation", "")
            or ""
        ).strip()

        code = (
            example.get("whole_func_string", "")
            or example.get("code", "")
            or example.get("function", "")
            or ""
        ).strip()

        func_name = example.get("func_name", "") or example.get("name", "") or ""

        # Quality filters
        if not docstring or not code:
            skipped += 1
            continue

        if len(docstring) < 10:
            skipped += 1
            continue

        if len(code) < 20:
            skipped += 1
            continue

        if len(code) > 10000:
            skipped += 1
            continue

        # Skip trivial docstrings
        trivial_patterns = [
            "todo", "fixme", "xxx", "hack",
            "generated", "auto-generated",
            "see ", "deprecated",
        ]
        docstring_lower = docstring.lower()
        if any(p in docstring_lower for p in trivial_patterns) and len(docstring) < 50:
            skipped += 1
            continue

        pairs.append({
            "query": docstring,
            "positive": code,
            "language": lang,
            "func_name": func_name,
            "source": "codesearchnet",
        })

        if len(pairs) % 50000 == 0:
            print(f"    {len(pairs):,} pairs extracted...")

    return pairs, skipped


def download_and_convert(languages: list[str], output: Path, max_per_language: int | None = None):
    """Download CodeSearchNet and convert to our training format."""
    all_languages = list(HF_DATASETS.keys())

    if not languages:
        languages = all_languages
    else:
        languages = [l.lower() for l in languages]
        invalid = set(languages) - set(all_languages)
        if invalid:
            raise ValueError(f"Invalid languages: {invalid}. Valid: {all_languages}")

    print(f"Downloading CodeSearchNet for: {languages}")
    print(f"Output: {output}")

    output.parent.mkdir(parents=True, exist_ok=True)

    total_pairs = 0
    stats = {}

    with output.open("w", encoding="utf-8") as f:
        for lang in languages:
            print(f"\nProcessing {lang}...")

            pairs, skipped = download_language(lang, max_per_language)

            for pair in pairs:
                f.write(json.dumps(pair, ensure_ascii=False) + "\n")

            stats[lang] = {"written": len(pairs), "skipped": skipped}
            total_pairs += len(pairs)
            print(f"  {lang}: {len(pairs):,} pairs (skipped {skipped:,})")

    print(f"\n{'='*60}")
    print(f"Total: {total_pairs:,} training pairs")
    print(f"Output: {output}")
    print(f"{'='*60}")

    # Write stats
    stats_file = output.with_suffix(".stats.json")
    with stats_file.open("w") as f:
        json.dump({"total": total_pairs, "by_language": stats}, f, indent=2)
    print(f"Stats: {stats_file}")

    return total_pairs


def main():
    parser = argparse.ArgumentParser(description="Download CodeSearchNet for training")
    parser.add_argument("--output", "-o", type=Path, default=Path("data/codesearchnet_all.jsonl"))
    parser.add_argument("--languages", "-l", type=str, default="",
                        help="Comma-separated languages (default: all)")
    parser.add_argument("--max-per-language", type=int, default=None,
                        help="Max pairs per language (for testing)")
    args = parser.parse_args()

    languages = [l.strip() for l in args.languages.split(",") if l.strip()]
    download_and_convert(languages, args.output, args.max_per_language)


if __name__ == "__main__":
    main()
