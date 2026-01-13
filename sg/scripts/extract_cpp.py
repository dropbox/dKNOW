#!/usr/bin/env python3
"""
Extract (doxygen/comment, function) pairs from C++ code for training embeddings.
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
    language: str = "cpp"


def extract_cpp_pairs(file_path: Path) -> list[TrainingPair]:
    """Extract (doxygen, function) pairs from a C++ file."""
    pairs = []

    try:
        content = file_path.read_text(encoding='utf-8', errors='replace')
    except Exception as e:
        return pairs

    # Pattern 1: /** Doxygen */ or /*! Doxygen */ followed by function
    doxygen_pattern = r'/\*[*!]\s*(.*?)\s*\*/\s*(?:template\s*<[^>]*>\s*)?(?:(?:static|inline|virtual|explicit|constexpr|const|volatile|unsigned|signed)\s+)*(\w+(?:<[^>]+>)?(?:\s*\*+|\s*&)*)\s+(\w+)\s*\('

    # Pattern 2: /// Doxygen single-line comments (multiple lines)
    # We'll handle these separately

    for match in re.finditer(doxygen_pattern, content, re.DOTALL):
        docstring = match.group(1).strip()
        return_type = match.group(2)
        func_name = match.group(3)

        # Clean up doxygen (remove * prefixes and backslash commands)
        docstring = re.sub(r'^\s*\*\s?', '', docstring, flags=re.MULTILINE)
        # Extract brief description (before @param, \param, etc)
        main_desc = re.split(r'\n\s*[@\\]', docstring)[0].strip()
        # Remove \brief prefix if present
        main_desc = re.sub(r'^\\brief\s+', '', main_desc)

        if len(main_desc) < 20:
            continue
        if func_name.startswith('_') or func_name in ('if', 'for', 'while', 'switch', 'catch'):
            continue

        # Find the function body
        open_paren = content.find('(', match.end() - 1)
        if open_paren == -1:
            continue

        # Find matching close paren
        depth = 1
        pos = open_paren + 1
        while pos < len(content) and depth > 0:
            if content[pos] == '(':
                depth += 1
            elif content[pos] == ')':
                depth -= 1
            pos += 1

        # Look for { after the )
        brace_pos = content.find('{', pos)
        if brace_pos == -1 or brace_pos - pos > 100:  # Too far, probably declaration
            continue

        # Find matching closing brace
        depth = 1
        pos = brace_pos + 1
        while pos < len(content) and depth > 0:
            if content[pos] == '{':
                depth += 1
            elif content[pos] == '}':
                depth -= 1
            pos += 1

        if depth != 0:
            continue

        func_code = content[match.start():pos].strip()

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
            func_name=func_name,
        ))

    return pairs


def extract_from_directory(root: Path, verbose: bool = False) -> list[TrainingPair]:
    """Extract pairs from all C++ files in directory."""
    pairs = []

    files = []
    for ext in ['*.cpp', '*.cc', '*.cxx', '*.hpp', '*.hh', '*.hxx']:
        files.extend(root.rglob(ext))

    # Filter
    skip_dirs = {'node_modules', '.git', 'build', 'dist', 'target', 'vendor', 'test', 'tests', 'third_party', 'external'}
    files = [f for f in files if not any(skip in f.parts for skip in skip_dirs)]

    if verbose:
        print(f"  Found {len(files)} C++ files")

    for file_path in files:
        try:
            file_pairs = extract_cpp_pairs(file_path)
            pairs.extend(file_pairs)
        except Exception as e:
            pass

    return pairs


def main():
    parser = argparse.ArgumentParser(description='Extract C++ training data')
    parser.add_argument('directories', nargs='+', type=Path)
    parser.add_argument('--output', '-o', type=Path, default=Path('cpp_training.jsonl'))
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

    print(f"\nPer-directory breakdown:")
    for name, count in sorted(repo_stats, key=lambda x: -x[1])[:20]:
        if count > 0:
            print(f"  {name}: {count} pairs")


if __name__ == '__main__':
    main()
