#!/usr/bin/env python3
"""
Extract (doc comment, function) pairs from Rust code for training embeddings.

Usage:
    python extract_rust_training_data.py ~/repo1 ~/repo2 --output training_data.jsonl
    python extract_rust_training_data.py ~/my-repos/* --output all_rust_data.jsonl

License: Apache 2.0 - Only use on Apache/MIT licensed code for releasable embeddings.
"""

import argparse
import json
import re
from pathlib import Path
from dataclasses import dataclass, asdict
from typing import Iterator, Optional
import sys


@dataclass
class TrainingPair:
    query: str          # Doc comment (natural language)
    positive: str       # Function body (code)
    file_path: str      # Source file
    func_name: str      # Function name
    file_context: Optional[str] = None  # Repo-relative context for filename signal
    language: str = "rust"


def clean_doc_comment(doc: str) -> str:
    """Clean doc comment by removing /// prefixes and normalizing whitespace."""
    lines = []
    for line in doc.strip().split('\n'):
        line = line.strip()
        if line.startswith('///'):
            lines.append(line[3:].strip())
        elif line.startswith('//!'):
            lines.append(line[3:].strip())
        elif line.startswith('*'):
            # Block comment line
            lines.append(line[1:].strip())

    text = ' '.join(lines)
    # Normalize whitespace
    text = re.sub(r'\s+', ' ', text)
    return text.strip()


def build_file_context(file_path: Path, repo_root: Optional[Path]) -> Optional[str]:
    """Return repo-relative file context like repo_name/path/to/file.rs."""
    if repo_root is None:
        return None
    try:
        relative_path = file_path.relative_to(repo_root)
    except ValueError:
        return None
    return f"{repo_root.name}/{relative_path.as_posix()}"


def extract_rust_pairs(file_path: Path, repo_root: Optional[Path] = None) -> Iterator[TrainingPair]:
    """Extract (doc comment, function) pairs from a Rust file."""
    try:
        content = file_path.read_text(encoding='utf-8')
    except Exception as e:
        print(f"  Warning: Could not read {file_path}: {e}", file=sys.stderr)
        return

    # Pattern to match doc comments followed by function definitions
    # This handles:
    # - /// line comments
    # - /** block comments */
    # - Multiple attributes like #[derive], #[cfg], etc.

    # First, find all /// doc comment blocks
    line_doc_pattern = r'''
        # Doc comments (/// lines)
        ((?:[ \t]*///[^\n]*\n)+)
        # Followed by attributes (optional, multiple)
        (?:[ \t]*\#\[[^\]]*\]\s*)*
        # Followed by visibility and modifiers (optional)
        [ \t]*(?:pub(?:\s*\([^)]*\))?\s+)?
        (?:async\s+)?
        (?:const\s+)?
        (?:unsafe\s+)?
        (?:extern\s+"[^"]*"\s+)?
        # Function definition
        fn\s+(\w+)
    '''

    for match in re.finditer(line_doc_pattern, content, re.MULTILINE | re.VERBOSE):
        doc_comment = match.group(1)
        func_name = match.group(2)

        # Get the full function (find matching braces)
        func_start = match.start()
        fn_match = re.search(r'fn\s+\w+[^{]*\{', content[func_start:])
        if not fn_match:
            continue

        brace_start = func_start + fn_match.end() - 1
        brace_count = 1
        pos = brace_start + 1

        while pos < len(content) and brace_count > 0:
            if content[pos] == '{':
                brace_count += 1
            elif content[pos] == '}':
                brace_count -= 1
            pos += 1

        if brace_count != 0:
            continue

        func_body = content[func_start:pos]

        # Clean doc comment
        doc_text = clean_doc_comment(doc_comment)

        # Skip if doc is too short or too long
        if len(doc_text) < 20:
            continue
        if len(doc_text) > 2000:
            doc_text = doc_text[:2000]

        # Skip test functions and internal functions
        if func_name.startswith('test_') or func_name.startswith('_'):
            continue

        # Skip if function is too short (likely just a wrapper)
        if len(func_body) < 50:
            continue

        # Truncate very long functions
        if len(func_body) > 4000:
            func_body = func_body[:4000] + "\n    // ... truncated"

        yield TrainingPair(
            query=doc_text,
            positive=func_body,
            file_path=str(file_path),
            func_name=func_name,
            file_context=build_file_context(file_path, repo_root),
        )


def extract_from_directory(root: Path, verbose: bool = False) -> list[TrainingPair]:
    """Extract pairs from all Rust files in directory tree."""
    pairs = []
    rust_files = list(root.rglob('*.rs'))

    for file_path in rust_files:
        # Skip target directories, tests, and build artifacts
        parts = file_path.parts
        if any(skip in parts for skip in ['target', 'build', '.git', 'vendor']):
            continue

        file_pairs = list(extract_rust_pairs(file_path, repo_root=root))
        if verbose and file_pairs:
            print(f"  {file_path}: {len(file_pairs)} pairs")
        pairs.extend(file_pairs)

    return pairs


def check_license(directory: Path) -> Optional[str]:
    """Check if directory has Apache/MIT license."""
    for license_file in ['LICENSE', 'LICENSE.md', 'LICENSE.txt', 'LICENSE-APACHE', 'LICENSE-MIT']:
        license_path = directory / license_file
        if license_path.exists():
            try:
                content = license_path.read_text().lower()
                if 'apache' in content:
                    return 'Apache-2.0'
                if 'mit' in content:
                    return 'MIT'
                if 'bsd' in content:
                    return 'BSD'
            except:
                pass

    # Check Cargo.toml
    cargo_path = directory / 'Cargo.toml'
    if cargo_path.exists():
        try:
            content = cargo_path.read_text()
            if 'license = "Apache-2.0"' in content or 'license = "MIT"' in content:
                return 'Apache-2.0/MIT'
        except:
            pass

    return None


def main():
    parser = argparse.ArgumentParser(
        description='Extract Rust training data for embedding fine-tuning',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog='''
Examples:
    # Extract from specific directories
    python extract_rust_training_data.py ~/sg ~/rust-warp -o training.jsonl

    # Extract from all repos in a directory
    python extract_rust_training_data.py ~/repos/* -o all_data.jsonl

    # Verbose output
    python extract_rust_training_data.py ~/sg -o data.jsonl -v
        '''
    )
    parser.add_argument('directories', nargs='+', type=Path,
                        help='Directories to scan for Rust code')
    parser.add_argument('--output', '-o', type=Path,
                        default=Path('rust_training_data.jsonl'),
                        help='Output JSONL file')
    parser.add_argument('--verbose', '-v', action='store_true',
                        help='Print detailed progress')
    parser.add_argument('--check-license', action='store_true',
                        help='Only include repos with Apache/MIT license')
    parser.add_argument('--min-pairs', type=int, default=0,
                        help='Minimum pairs per repo to include')
    args = parser.parse_args()

    all_pairs = []
    repo_stats = []

    for directory in args.directories:
        if not directory.exists():
            print(f"Warning: {directory} does not exist, skipping", file=sys.stderr)
            continue

        if not directory.is_dir():
            print(f"Warning: {directory} is not a directory, skipping", file=sys.stderr)
            continue

        # Check license if requested
        if args.check_license:
            license_type = check_license(directory)
            if not license_type:
                print(f"Skipping {directory.name}: no Apache/MIT license found")
                continue
            if args.verbose:
                print(f"Found {license_type} license in {directory.name}")

        print(f"Scanning {directory}...")
        pairs = extract_from_directory(directory, verbose=args.verbose)

        if len(pairs) >= args.min_pairs:
            all_pairs.extend(pairs)
            repo_stats.append((directory.name, len(pairs)))
            print(f"  Found {len(pairs)} pairs")
        else:
            print(f"  Found {len(pairs)} pairs (below minimum {args.min_pairs}, skipping)")

    # Deduplicate by (func_name, query) to avoid duplicates from shared code
    seen = set()
    unique_pairs = []
    for pair in all_pairs:
        key = (pair.func_name, pair.query[:100])
        if key not in seen:
            seen.add(key)
            unique_pairs.append(pair)

    print(f"\nDeduplication: {len(all_pairs)} -> {len(unique_pairs)} pairs")

    # Write to JSONL
    with open(args.output, 'w') as f:
        for pair in unique_pairs:
            f.write(json.dumps(asdict(pair)) + '\n')

    print(f"\n{'='*60}")
    print(f"Total: {len(unique_pairs)} unique pairs written to {args.output}")
    print(f"\nPer-repo breakdown:")
    for name, count in sorted(repo_stats, key=lambda x: -x[1]):
        print(f"  {name}: {count} pairs")

    # Show sample
    if unique_pairs and args.verbose:
        print(f"\nSample pair:")
        sample = unique_pairs[0]
        print(f"  Query: {sample.query[:100]}...")
        print(f"  Function: {sample.func_name}")
        print(f"  File: {sample.file_path}")


if __name__ == '__main__':
    main()
