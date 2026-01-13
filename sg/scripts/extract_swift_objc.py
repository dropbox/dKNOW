#!/usr/bin/env python3
"""
Extract (doc comment, function) pairs from Swift and Objective-C code.
"""

import argparse
import json
import re
from pathlib import Path
from dataclasses import dataclass, asdict
from concurrent.futures import ProcessPoolExecutor, as_completed
import multiprocessing


@dataclass
class TrainingPair:
    query: str
    positive: str
    file_path: str
    func_name: str
    language: str


def extract_swift_pairs(file_path: Path) -> list[TrainingPair]:
    """Extract (doc comment, function) pairs from Swift file.

    Swift uses /// or /** */ for doc comments.
    """
    pairs = []
    try:
        content = file_path.read_text(encoding='utf-8', errors='replace')
    except:
        return pairs

    # Pattern 1: /// doc comments (multiple lines) followed by func/class/struct
    triple_slash = r'((?:^\s*///.*\n)+)\s*((?:@\w+(?:\([^)]*\))?\s*)*)(?:public\s+|private\s+|internal\s+|fileprivate\s+|open\s+)?(?:final\s+)?(?:class|struct|enum|func|var|let|protocol|extension)\s+(\w+)'

    # Pattern 2: /** */ block comments
    block_comment = r'/\*\*\s*(.*?)\s*\*/\s*((?:@\w+(?:\([^)]*\))?\s*)*)(?:public\s+|private\s+|internal\s+|fileprivate\s+|open\s+)?(?:final\s+)?(?:class|struct|enum|func|var|let|protocol|extension)\s+(\w+)'

    for pattern, is_triple in [(triple_slash, True), (block_comment, False)]:
        flags = re.MULTILINE | re.DOTALL if not is_triple else re.MULTILINE
        for match in re.finditer(pattern, content, flags):
            if is_triple:
                # Clean up /// prefixes
                doc = match.group(1)
                doc = re.sub(r'^\s*///\s?', '', doc, flags=re.MULTILINE).strip()
            else:
                doc = match.group(1).strip()
                # Clean up * prefixes
                doc = re.sub(r'^\s*\*\s?', '', doc, flags=re.MULTILINE).strip()

            name = match.group(3)

            # Extract main description (before - Parameters:, - Returns:, etc)
            main_desc = re.split(r'\n\s*-\s*(?:Parameters?|Returns?|Throws?):', doc)[0].strip()

            if len(main_desc) < 20 or name.startswith('_'):
                continue

            # Find the definition body
            start = match.start()
            brace_pos = content.find('{', match.end())
            if brace_pos == -1 or brace_pos - match.end() > 200:
                continue

            # Find matching }
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

            func_code = content[start:pos].strip()
            if len(func_code) < 50:
                continue

            if len(main_desc) > 2000:
                main_desc = main_desc[:2000]
            if len(func_code) > 4000:
                func_code = func_code[:4000] + "\n    // ... truncated"

            pairs.append(TrainingPair(
                query=main_desc,
                positive=func_code,
                file_path=str(file_path),
                func_name=name,
                language="swift",
            ))

    return pairs


def extract_objc_pairs(file_path: Path) -> list[TrainingPair]:
    """Extract (doc comment, method) pairs from Objective-C file.

    ObjC uses /** */ or /// for doc comments.
    """
    pairs = []
    try:
        content = file_path.read_text(encoding='utf-8', errors='replace')
    except:
        return pairs

    # Pattern: /** */ followed by - or + method, or @interface/@implementation
    pattern = r'/\*\*\s*(.*?)\s*\*/\s*([+-]\s*\([^)]+\)\s*\w+[^{;]*[{;]|@(?:interface|implementation)\s+(\w+))'

    for match in re.finditer(pattern, content, re.DOTALL):
        doc = match.group(1).strip()
        # Clean up * prefixes
        doc = re.sub(r'^\s*\*\s?', '', doc, flags=re.MULTILINE).strip()

        # Extract main description
        main_desc = re.split(r'\n\s*@(?:param|return|throws|discussion):', doc, flags=re.IGNORECASE)[0].strip()

        if len(main_desc) < 20:
            continue

        # Get method/class name
        method_match = re.search(r'[+-]\s*\([^)]+\)\s*(\w+)', match.group(2))
        class_match = re.search(r'@(?:interface|implementation)\s+(\w+)', match.group(2))
        name = method_match.group(1) if method_match else (class_match.group(1) if class_match else None)

        if not name or name.startswith('_'):
            continue

        # Find the body
        start = match.start()
        if '{' in match.group(2):
            brace_pos = content.find('{', match.end() - len(match.group(2)))
            depth = 1
            pos = brace_pos + 1
            while pos < len(content) and depth > 0:
                if content[pos] == '{':
                    depth += 1
                elif content[pos] == '}':
                    depth -= 1
                pos += 1
            end = pos
        else:
            # Declaration only
            end = match.end()

        func_code = content[start:end].strip()
        if len(func_code) < 50:
            continue

        if len(main_desc) > 2000:
            main_desc = main_desc[:2000]
        if len(func_code) > 4000:
            func_code = func_code[:4000] + "\n    // ... truncated"

        pairs.append(TrainingPair(
            query=main_desc,
            positive=func_code,
            file_path=str(file_path),
            func_name=name,
            language="objc",
        ))

    return pairs


def process_file(args):
    """Process a single file (for parallel execution)."""
    file_path, lang = args
    try:
        if lang == "swift":
            return extract_swift_pairs(file_path)
        else:
            return extract_objc_pairs(file_path)
    except Exception as e:
        return []


def extract_from_directory(root: Path, verbose: bool = False) -> list[TrainingPair]:
    """Extract pairs from Swift and ObjC files in directory."""
    pairs = []

    swift_files = list(root.rglob('*.swift'))
    objc_files = list(root.rglob('*.m')) + list(root.rglob('*.mm'))

    # Filter
    skip_dirs = {'.git', 'build', 'DerivedData', 'Pods', 'Carthage', 'vendor', 'test', 'tests', '.build'}
    swift_files = [f for f in swift_files if not any(skip in f.parts for skip in skip_dirs)]
    objc_files = [f for f in objc_files if not any(skip in f.parts for skip in skip_dirs)]

    if verbose:
        print(f"  Found {len(swift_files)} Swift files, {len(objc_files)} ObjC files")

    # Prepare work items
    work_items = [(f, "swift") for f in swift_files] + [(f, "objc") for f in objc_files]

    # Use parallel processing
    num_workers = min(multiprocessing.cpu_count(), 8)
    with ProcessPoolExecutor(max_workers=num_workers) as executor:
        futures = [executor.submit(process_file, item) for item in work_items]
        for future in as_completed(futures):
            try:
                file_pairs = future.result()
                pairs.extend(file_pairs)
            except Exception as e:
                pass

    return pairs


def main():
    parser = argparse.ArgumentParser(description='Extract Swift/ObjC training data')
    parser.add_argument('directories', nargs='+', type=Path)
    parser.add_argument('--output', '-o', type=Path, default=Path('swift_objc_training.jsonl'))
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

    # Stats by language
    by_lang = {}
    for p in unique_pairs:
        by_lang[p.language] = by_lang.get(p.language, 0) + 1

    print(f"\nBy language:")
    for lang, count in sorted(by_lang.items(), key=lambda x: -x[1]):
        print(f"  {lang}: {count}")

    print(f"\nTotal: {len(unique_pairs)} unique pairs written to {args.output}")

    print(f"\nPer-directory breakdown:")
    for name, count in sorted(repo_stats, key=lambda x: -x[1])[:20]:
        if count > 0:
            print(f"  {name}: {count} pairs")


if __name__ == '__main__':
    main()
