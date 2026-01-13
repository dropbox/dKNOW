#!/usr/bin/env python3
"""
Extract (JSDoc, function) pairs from TypeScript code for training embeddings.
"""

import argparse
import json
import re
from pathlib import Path
from dataclasses import dataclass, asdict


@dataclass
class TrainingPair:
    query: str
    positive: str
    file_path: str
    func_name: str
    language: str = "typescript"


def extract_ts_pairs(file_path: Path) -> list[TrainingPair]:
    """Extract (JSDoc, function) pairs from a TypeScript file."""
    pairs = []

    try:
        content = file_path.read_text(encoding='utf-8', errors='replace')
    except Exception as e:
        return pairs

    # Pattern: /** JSDoc */ followed by function/method/class
    pattern = r'/\*\*\s*(.*?)\s*\*/\s*(?:export\s+)?(?:async\s+)?(?:function|const|class|interface|type|enum)\s+(\w+)'

    for match in re.finditer(pattern, content, re.DOTALL):
        docstring = match.group(1).strip()
        name = match.group(2)

        # Clean up JSDoc (remove * prefixes and @tags for query)
        clean_doc = re.sub(r'^\s*\*\s?', '', docstring, flags=re.MULTILINE)
        # Extract main description (before @param, @returns, etc)
        main_desc = re.split(r'\n\s*@', clean_doc)[0].strip()

        if len(main_desc) < 20:
            continue
        if name.startswith('_'):
            continue

        # Find the block - look for opening brace or = for const
        block_start = content.find('{', match.end())
        eq_pos = content.find('=', match.end())

        if block_start == -1 and eq_pos == -1:
            continue

        # Use whichever comes first
        start_pos = min(p for p in [block_start, eq_pos] if p != -1)

        if content[start_pos] == '{':
            # Find matching closing brace
            depth = 1
            pos = start_pos + 1
            while pos < len(content) and depth > 0:
                if content[pos] == '{':
                    depth += 1
                elif content[pos] == '}':
                    depth -= 1
                pos += 1
            end_pos = pos
        else:
            # For const = ..., find end of statement
            end_pos = content.find(';', start_pos)
            if end_pos == -1:
                end_pos = min(start_pos + 2000, len(content))

        func_code = content[match.start():end_pos].strip()

        # Skip very short
        if len(func_code) < 50:
            continue

        # Truncate if too long
        if len(main_desc) > 2000:
            main_desc = main_desc[:2000]
        if len(func_code) > 4000:
            func_code = func_code[:4000] + "\n  // ... truncated"

        pairs.append(TrainingPair(
            query=main_desc,
            positive=func_code,
            file_path=str(file_path),
            func_name=name,
        ))

    return pairs


def extract_from_directory(root: Path, verbose: bool = False) -> list[TrainingPair]:
    """Extract pairs from all TypeScript files in directory."""
    pairs = []

    files = list(root.rglob('*.ts'))
    files.extend(root.rglob('*.tsx'))

    # Filter
    skip_dirs = {'node_modules', '.git', 'build', 'dist', 'target', 'vendor', '__tests__', 'test'}
    files = [f for f in files if not any(skip in f.parts for skip in skip_dirs)]
    # Skip .d.ts declaration files
    files = [f for f in files if not f.name.endswith('.d.ts')]

    if verbose:
        print(f"  Found {len(files)} TypeScript files")

    for file_path in files:
        try:
            file_pairs = extract_ts_pairs(file_path)
            pairs.extend(file_pairs)
        except Exception as e:
            pass

    return pairs


def main():
    parser = argparse.ArgumentParser(description='Extract TypeScript training data')
    parser.add_argument('directories', nargs='+', type=Path)
    parser.add_argument('--output', '-o', type=Path, default=Path('typescript_training.jsonl'))
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
        if pairs:
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


if __name__ == '__main__':
    main()
