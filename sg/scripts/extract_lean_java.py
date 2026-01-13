#!/usr/bin/env python3
"""
Extract (docstring, function) pairs from Lean 4 and Java code for training embeddings.
"""

import argparse
import json
import re
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
    language: str


def extract_lean_pairs(file_path: Path) -> list[TrainingPair]:
    """Extract (docstring, definition) pairs from a Lean 4 file.

    Lean 4 uses /-- ... -/ for doc comments.
    """
    pairs = []

    try:
        content = file_path.read_text(encoding='utf-8', errors='replace')
    except Exception as e:
        return pairs

    # Pattern: /-- doc comment -/ followed by def/theorem/lemma/structure
    # Lean doc comments: /-- ... -/ (can be multiline)
    pattern = r'/--\s*(.*?)\s*-/\s*(?:@\[.*?\]\s*)*(def|theorem|lemma|structure|inductive|class|instance|abbrev|partial def|unsafe def)\s+(\w+)'

    for match in re.finditer(pattern, content, re.DOTALL):
        docstring = match.group(1).strip()
        kind = match.group(2)
        name = match.group(3)

        if len(docstring) < 20:
            continue
        if name.startswith('_'):
            continue

        # Get the full definition (up to next top-level def or end of reasonable block)
        start_pos = match.start()
        # Find end - look for next doc comment or next unindented def
        end_match = re.search(r'\n(?=/--|(?:def|theorem|lemma|structure|inductive|class)\s)', content[match.end():])
        if end_match:
            end_pos = match.end() + end_match.start()
        else:
            end_pos = min(match.end() + 2000, len(content))

        func_code = content[match.start():end_pos].strip()

        # Skip very short definitions
        if len(func_code) < 50:
            continue

        # Truncate if too long
        if len(docstring) > 2000:
            docstring = docstring[:2000]
        if len(func_code) > 4000:
            func_code = func_code[:4000] + "\n  -- ... truncated"

        pairs.append(TrainingPair(
            query=docstring,
            positive=func_code,
            file_path=str(file_path),
            func_name=name,
            language="lean",
        ))

    return pairs


def extract_java_pairs(file_path: Path) -> list[TrainingPair]:
    """Extract (javadoc, method) pairs from a Java file.

    Java uses /** ... */ for javadoc comments.
    """
    pairs = []

    try:
        content = file_path.read_text(encoding='utf-8', errors='replace')
    except Exception as e:
        return pairs

    # Pattern: /** javadoc */ followed by method signature
    # Captures multiline javadoc and the following method
    pattern = r'/\*\*\s*(.*?)\s*\*/\s*(?:@\w+(?:\([^)]*\))?\s*)*(?:public|private|protected)?\s*(?:static)?\s*(?:final)?\s*(?:synchronized)?\s*(?:<[^>]+>\s*)?(\w+(?:<[^>]+>)?)\s+(\w+)\s*\('

    for match in re.finditer(pattern, content, re.DOTALL):
        docstring = match.group(1).strip()
        return_type = match.group(2)
        method_name = match.group(3)

        # Clean up javadoc (remove * prefixes)
        docstring = re.sub(r'^\s*\*\s?', '', docstring, flags=re.MULTILINE)
        docstring = docstring.strip()

        if len(docstring) < 20:
            continue
        if method_name.startswith('_') or method_name in ('get', 'set', 'is', 'has'):
            continue

        # Find the method body - count braces
        method_start = match.start()
        brace_pos = content.find('{', match.end())
        if brace_pos == -1:
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

        method_code = content[method_start:pos].strip()

        # Skip very short methods
        if len(method_code) < 50:
            continue

        # Truncate if too long
        if len(docstring) > 2000:
            docstring = docstring[:2000]
        if len(method_code) > 4000:
            method_code = method_code[:4000] + "\n    // ... truncated"

        pairs.append(TrainingPair(
            query=docstring,
            positive=method_code,
            file_path=str(file_path),
            func_name=method_name,
            language="java",
        ))

    return pairs


def extract_from_directory(root: Path, language: str, verbose: bool = False) -> list[TrainingPair]:
    """Extract pairs from all files of given language in directory."""
    pairs = []

    if language == "lean":
        files = list(root.rglob('*.lean'))
        extract_fn = extract_lean_pairs
    elif language == "java":
        files = list(root.rglob('*.java'))
        extract_fn = extract_java_pairs
    else:
        return pairs

    # Filter out common non-source directories
    skip_dirs = {'node_modules', '.git', 'build', 'dist', 'target', 'vendor', 'test', 'tests', '.lake'}
    files = [f for f in files if not any(skip in f.parts for skip in skip_dirs)]

    if verbose:
        print(f"  Found {len(files)} {language} files")

    for file_path in files:
        try:
            file_pairs = extract_fn(file_path)
            pairs.extend(file_pairs)
        except Exception as e:
            if verbose:
                print(f"  Error processing {file_path}: {e}")

    return pairs


def main():
    parser = argparse.ArgumentParser(description='Extract Lean/Java training data')
    parser.add_argument('directories', nargs='+', type=Path)
    parser.add_argument('--output', '-o', type=Path, default=Path('lean_java_training.jsonl'))
    parser.add_argument('--language', '-l', choices=['lean', 'java', 'both'], default='both')
    parser.add_argument('--verbose', '-v', action='store_true')
    args = parser.parse_args()

    all_pairs = []
    repo_stats = []

    languages = ['lean', 'java'] if args.language == 'both' else [args.language]

    for directory in args.directories:
        if not directory.exists():
            print(f"Skipping {directory} (not found)")
            continue

        for lang in languages:
            print(f"Scanning {directory} for {lang}...")
            pairs = extract_from_directory(directory, lang, verbose=args.verbose)
            all_pairs.extend(pairs)
            if pairs:
                repo_stats.append((f"{directory.name} ({lang})", len(pairs)))
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

    # Stats by language
    by_lang = {}
    for p in unique_pairs:
        by_lang[p.language] = by_lang.get(p.language, 0) + 1
    print(f"\nBy language:")
    for lang, count in sorted(by_lang.items(), key=lambda x: -x[1]):
        print(f"  {lang}: {count}")

    print(f"\nPer-directory breakdown:")
    for name, count in sorted(repo_stats, key=lambda x: -x[1])[:30]:
        if count > 0:
            print(f"  {name}: {count} pairs")


if __name__ == '__main__':
    main()
