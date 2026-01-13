#!/usr/bin/env python3
"""
Fast Rust training data extraction using tree-sitter.
10-100x faster than regex-based extraction.
"""

import argparse
import json
from pathlib import Path
from dataclasses import dataclass, asdict
from typing import Iterator
import sys
from concurrent.futures import ProcessPoolExecutor, as_completed
import multiprocessing

try:
    import tree_sitter_rust as ts_rust
    from tree_sitter import Language, Parser
    HAS_TREE_SITTER = True
except ImportError:
    HAS_TREE_SITTER = False
    print("Warning: tree-sitter not available, using regex fallback", file=sys.stderr)


@dataclass
class TrainingPair:
    query: str
    positive: str
    file_path: str
    func_name: str
    language: str = "rust"


def extract_with_tree_sitter(file_path: Path) -> list[TrainingPair]:
    """Fast extraction using tree-sitter AST parsing."""
    pairs = []

    try:
        content = file_path.read_bytes()
        text = content.decode('utf-8', errors='replace')
    except Exception as e:
        return pairs

    lang = Language(ts_rust.language())
    parser = Parser(lang)
    tree = parser.parse(content)

    def get_text(node):
        return content[node.start_byte:node.end_byte].decode('utf-8', errors='replace')

    def find_doc_comment(node):
        """Find doc comment preceding a node."""
        # Look for line_comment siblings before this node
        doc_lines = []
        prev = node.prev_named_sibling

        # Collect consecutive doc comments going backwards
        while prev:
            if prev.type == 'line_comment':
                comment_text = get_text(prev)
                if comment_text.startswith('///') or comment_text.startswith('//!'):
                    doc_lines.insert(0, comment_text)
                    prev = prev.prev_named_sibling
                    continue
            break

        if doc_lines:
            # Clean up doc comments
            cleaned = []
            for line in doc_lines:
                if line.startswith('///'):
                    cleaned.append(line[3:].strip())
                elif line.startswith('//!'):
                    cleaned.append(line[3:].strip())
            return ' '.join(cleaned)

        return None

    def process_node(node):
        if node.type == 'function_item':
            # Get function name
            name_node = None
            for child in node.children:
                if child.type == 'identifier':
                    name_node = child
                    break

            if not name_node:
                return

            func_name = get_text(name_node)

            # Skip test functions
            if func_name.startswith('test_') or func_name.startswith('_'):
                return

            # Get doc comment
            doc = find_doc_comment(node)

            if doc and len(doc) >= 20:
                func_text = get_text(node)

                # Skip very short functions
                if len(func_text) < 50:
                    return

                # Truncate very long functions
                if len(func_text) > 4000:
                    func_text = func_text[:4000] + "\n    // ... truncated"

                # Truncate very long docs
                if len(doc) > 2000:
                    doc = doc[:2000]

                pairs.append(TrainingPair(
                    query=doc,
                    positive=func_text,
                    file_path=str(file_path),
                    func_name=func_name,
                ))

        # Recurse into children
        for child in node.children:
            process_node(child)

    process_node(tree.root_node)
    return pairs


def extract_file(file_path: Path) -> list[TrainingPair]:
    """Extract pairs from a single file."""
    # Skip certain directories
    parts = file_path.parts
    if any(skip in parts for skip in ['target', 'build', '.git', 'vendor', 'test', 'tests']):
        return []

    if HAS_TREE_SITTER:
        return extract_with_tree_sitter(file_path)
    else:
        # Fallback to simple regex (slower)
        return []


def extract_from_directory(root: Path, verbose: bool = False, parallel: bool = True) -> list[TrainingPair]:
    """Extract pairs from all Rust files using parallel processing."""
    rust_files = list(root.rglob('*.rs'))

    # Filter out unwanted directories
    rust_files = [f for f in rust_files if not any(
        skip in f.parts for skip in ['target', 'build', '.git', 'vendor']
    )]

    if verbose:
        print(f"  Found {len(rust_files)} Rust files")

    all_pairs = []

    if parallel and len(rust_files) > 100:
        # Use multiprocessing for large directories
        num_workers = min(multiprocessing.cpu_count(), 8)
        with ProcessPoolExecutor(max_workers=num_workers) as executor:
            futures = {executor.submit(extract_file, f): f for f in rust_files}
            for future in as_completed(futures):
                try:
                    pairs = future.result()
                    all_pairs.extend(pairs)
                except Exception as e:
                    pass
    else:
        for file_path in rust_files:
            pairs = extract_file(file_path)
            all_pairs.extend(pairs)

    return all_pairs


def main():
    parser = argparse.ArgumentParser(description='Fast Rust training data extraction')
    parser.add_argument('directories', nargs='+', type=Path)
    parser.add_argument('--output', '-o', type=Path, default=Path('rust_training_data.jsonl'))
    parser.add_argument('--verbose', '-v', action='store_true')
    parser.add_argument('--no-parallel', action='store_true', help='Disable parallel processing')
    args = parser.parse_args()

    if not HAS_TREE_SITTER:
        print("ERROR: tree-sitter-rust not installed. Run: pip install tree-sitter tree-sitter-rust")
        sys.exit(1)

    all_pairs = []
    repo_stats = []

    for directory in args.directories:
        if not directory.exists():
            print(f"Warning: {directory} does not exist", file=sys.stderr)
            continue

        print(f"Scanning {directory}...")
        pairs = extract_from_directory(directory, verbose=args.verbose, parallel=not args.no_parallel)
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

    # Write output
    with open(args.output, 'w') as f:
        for pair in unique_pairs:
            f.write(json.dumps(asdict(pair)) + '\n')

    print(f"\n{'='*60}")
    print(f"Total: {len(unique_pairs)} unique pairs written to {args.output}")
    print(f"\nPer-directory breakdown:")
    for name, count in sorted(repo_stats, key=lambda x: -x[1]):
        print(f"  {name}: {count} pairs")


if __name__ == '__main__':
    main()
