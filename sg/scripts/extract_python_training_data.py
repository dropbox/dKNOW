#!/usr/bin/env python3
"""
Extract (docstring, function) pairs from Python code for training embeddings.
"""

import argparse
import ast
import json
from pathlib import Path
from dataclasses import dataclass, asdict
from typing import Iterator
import sys


@dataclass
class TrainingPair:
    query: str
    positive: str
    file_path: str
    func_name: str
    language: str = "python"


def extract_python_pairs(file_path: Path) -> list[TrainingPair]:
    """Extract (docstring, function) pairs from a Python file."""
    pairs = []

    try:
        content = file_path.read_text(encoding='utf-8')
        tree = ast.parse(content)
    except Exception as e:
        return pairs

    for node in ast.walk(tree):
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
            docstring = ast.get_docstring(node)

            if docstring and len(docstring) >= 20:
                func_name = node.name

                # Skip private/test functions
                if func_name.startswith('_') or func_name.startswith('test_'):
                    continue

                try:
                    func_code = ast.unparse(node)
                except:
                    continue

                # Skip very short functions
                if len(func_code) < 50:
                    continue

                # Truncate
                if len(docstring) > 2000:
                    docstring = docstring[:2000]
                if len(func_code) > 4000:
                    func_code = func_code[:4000] + "\n    # ... truncated"

                pairs.append(TrainingPair(
                    query=docstring,
                    positive=func_code,
                    file_path=str(file_path),
                    func_name=func_name,
                ))

    return pairs


def extract_from_directory(root: Path, verbose: bool = False) -> list[TrainingPair]:
    """Extract pairs from all Python files in directory."""
    pairs = []
    python_files = list(root.rglob('*.py'))

    # Filter
    python_files = [f for f in python_files if not any(
        skip in f.parts for skip in ['venv', '.venv', 'site-packages', '__pycache__', '.git', 'build', 'dist']
    )]

    if verbose:
        print(f"  Found {len(python_files)} Python files")

    for file_path in python_files:
        file_pairs = extract_python_pairs(file_path)
        pairs.extend(file_pairs)

    return pairs


def main():
    parser = argparse.ArgumentParser(description='Extract Python training data')
    parser.add_argument('directories', nargs='+', type=Path)
    parser.add_argument('--output', '-o', type=Path, default=Path('python_training_data.jsonl'))
    parser.add_argument('--verbose', '-v', action='store_true')
    args = parser.parse_args()

    all_pairs = []
    repo_stats = []

    for directory in args.directories:
        if not directory.exists():
            continue

        print(f"Scanning {directory}...")
        pairs = extract_from_directory(directory, verbose=args.verbose)
        all_pairs.extend(pairs)
        repo_stats.append((directory.name, len(pairs)))
        print(f"  Found {len(pairs)} pairs")

    # Deduplicate
    seen = set()
    unique_pairs = []
    for pair in all_pairs:
        key = (pair.func_name, pair.query[:100])
        if key not in seen:
            seen.add(key)
            unique_pairs.append(pair)

    print(f"\nDeduplication: {len(all_pairs)} -> {len(unique_pairs)} pairs")

    with open(args.output, 'w') as f:
        for pair in unique_pairs:
            f.write(json.dumps(asdict(pair)) + '\n')

    print(f"\nTotal: {len(unique_pairs)} unique pairs written to {args.output}")
    print(f"\nPer-directory breakdown:")
    for name, count in sorted(repo_stats, key=lambda x: -x[1])[:30]:
        if count > 0:
            print(f"  {name}: {count} pairs")


if __name__ == '__main__':
    main()
