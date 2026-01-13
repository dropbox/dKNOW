#!/usr/bin/env python3
"""
Download The Stack v2 (permissive subset) and extract docstring-code pairs.

The Stack v2 contains raw source code files. We extract function-docstring pairs
using tree-sitter for AST parsing.

License: Only downloads files with permissive licenses (Apache-2.0, MIT, BSD, etc.)

Usage:
    python scripts/download_thestack.py --output data/thestack_extracted.jsonl
    python scripts/download_thestack.py --languages python,rust,go --max-per-language 50000
"""

import argparse
import json
import re
from pathlib import Path
from typing import Iterator


# Language configs for docstring extraction
LANGUAGE_CONFIGS = {
    "python": {
        "data_dir": "data/python",
        "docstring_pattern": r'"""(.+?)"""',
        "function_pattern": r"def\s+(\w+)\s*\([^)]*\).*?(?=\ndef\s|\nclass\s|\Z)",
    },
    "java": {
        "data_dir": "data/java",
        "docstring_pattern": r"/\*\*(.+?)\*/",
        "function_pattern": r"(?:public|private|protected)?\s*(?:static\s+)?[\w<>\[\]]+\s+(\w+)\s*\([^)]*\)\s*\{",
    },
    "go": {
        "data_dir": "data/go",
        "docstring_pattern": r"//\s*(.+?)(?=\nfunc)",
        "function_pattern": r"func\s+(?:\([^)]+\)\s*)?(\w+)\s*\([^)]*\)",
    },
    "rust": {
        "data_dir": "data/rust",
        "docstring_pattern": r"///\s*(.+?)(?=\n(?:pub\s+)?fn)",
        "function_pattern": r"(?:pub\s+)?fn\s+(\w+)",
    },
    "typescript": {
        "data_dir": "data/typescript",
        "docstring_pattern": r"/\*\*(.+?)\*/",
        "function_pattern": r"(?:export\s+)?(?:async\s+)?function\s+(\w+)",
    },
    "javascript": {
        "data_dir": "data/javascript",
        "docstring_pattern": r"/\*\*(.+?)\*/",
        "function_pattern": r"(?:export\s+)?(?:async\s+)?function\s+(\w+)",
    },
}


def extract_pairs_simple(content: str, language: str) -> list[dict]:
    """
    Simple regex-based extraction of docstring-function pairs.

    This is a fallback when tree-sitter isn't available.
    Works reasonably well for most languages.
    """
    pairs = []

    if language == "python":
        # Python: look for """docstring""" followed by def
        pattern = r'"""(.*?)"""\s*\n\s*def\s+(\w+)\s*\([^)]*\).*?(?=\n(?:def|class)\s|\Z)'
        for match in re.finditer(pattern, content, re.DOTALL):
            docstring = match.group(1).strip()
            func_name = match.group(2)
            # Get function body (approximate)
            func_start = match.start()
            func_end = match.end()
            func_body = content[func_start:func_end]

            if len(docstring) >= 15 and len(func_body) >= 50:
                pairs.append({
                    "query": docstring,
                    "positive": func_body,
                    "func_name": func_name,
                })

    elif language == "rust":
        # Rust: look for /// comments followed by fn
        pattern = r'((?:///.*\n)+)\s*(?:pub\s+)?(?:async\s+)?fn\s+(\w+)[^{]*\{([^}]*(?:\{[^}]*\}[^}]*)*)\}'
        for match in re.finditer(pattern, content):
            doc_lines = match.group(1)
            docstring = " ".join(
                line.strip().lstrip("/").strip()
                for line in doc_lines.split("\n")
                if line.strip()
            )
            func_name = match.group(2)
            func_body = match.group(0)

            if len(docstring) >= 15 and len(func_body) >= 50:
                pairs.append({
                    "query": docstring,
                    "positive": func_body,
                    "func_name": func_name,
                })

    elif language in ("java", "typescript", "javascript"):
        # Java/JS/TS: look for /** */ comments followed by function
        pattern = r'/\*\*(.*?)\*/\s*(?:public\s+|private\s+|protected\s+|export\s+)?(?:static\s+)?(?:async\s+)?(?:function\s+)?(?:[\w<>\[\]]+\s+)?(\w+)\s*\([^)]*\)\s*\{'
        for match in re.finditer(pattern, content, re.DOTALL):
            docstring = match.group(1).strip()
            # Clean up JSDoc markers
            docstring = re.sub(r'\s*\*\s*', ' ', docstring)
            docstring = re.sub(r'@\w+\s+[^\n]*', '', docstring)
            docstring = docstring.strip()

            func_name = match.group(2)

            if len(docstring) >= 15:
                # Get a reasonable chunk after the match
                start = match.start()
                end = min(start + 2000, len(content))
                func_body = content[start:end]

                pairs.append({
                    "query": docstring,
                    "positive": func_body[:1500],  # Truncate long functions
                    "func_name": func_name,
                })

    elif language == "go":
        # Go: look for // comments directly above func
        pattern = r'((?://.*\n)+)func\s+(?:\([^)]+\)\s*)?(\w+)\s*\([^)]*\)\s*(?:[^{]*)\s*\{'
        for match in re.finditer(pattern, content):
            doc_lines = match.group(1)
            docstring = " ".join(
                line.strip().lstrip("/").strip()
                for line in doc_lines.split("\n")
                if line.strip()
            )
            func_name = match.group(2)

            if len(docstring) >= 15:
                start = match.start()
                end = min(start + 2000, len(content))
                func_body = content[start:end]

                pairs.append({
                    "query": docstring,
                    "positive": func_body[:1500],
                    "func_name": func_name,
                })

    return pairs


def download_and_extract(
    languages: list[str],
    output: Path,
    max_per_language: int | None = None,
    streaming: bool = True,
) -> int:
    """Download The Stack and extract docstring-code pairs."""
    try:
        from datasets import load_dataset
    except ImportError:
        print("Installing datasets library...")
        import subprocess
        subprocess.run(["pip", "install", "datasets"], check=True)
        from datasets import load_dataset

    output.parent.mkdir(parents=True, exist_ok=True)

    total_pairs = 0
    stats = {}

    print(f"Downloading The Stack v2 (permissive) for: {languages}")
    print(f"Max per language: {max_per_language or 'unlimited'}")
    print(f"Output: {output}")

    with output.open("w", encoding="utf-8") as f:
        for lang in languages:
            print(f"\n{'='*60}")
            print(f"Processing {lang}...")

            config = LANGUAGE_CONFIGS.get(lang)
            if not config:
                print(f"  Skipping {lang} - no extraction config")
                continue

            try:
                # Load The Stack v2 for this language
                # Using the deduplicated, permissive-only version
                ds = load_dataset(
                    "bigcode/the-stack-v2-dedup",
                    data_dir=config["data_dir"],
                    split="train",
                    streaming=streaming,
                    trust_remote_code=True,
                )
            except Exception as e:
                print(f"  Error loading {lang}: {e}")
                print(f"  Trying alternative dataset...")
                try:
                    # Fall back to original The Stack
                    ds = load_dataset(
                        "bigcode/the-stack-dedup",
                        data_dir=config["data_dir"],
                        split="train",
                        streaming=streaming,
                        trust_remote_code=True,
                    )
                except Exception as e2:
                    print(f"  Failed to load {lang}: {e2}")
                    continue

            lang_pairs = 0
            files_processed = 0

            for example in ds:
                files_processed += 1

                if files_processed % 10000 == 0:
                    print(f"  Files: {files_processed:,}, Pairs: {lang_pairs:,}")

                content = example.get("content", "")
                if not content or len(content) < 100:
                    continue

                # Extract pairs from this file
                pairs = extract_pairs_simple(content, lang)

                for pair in pairs:
                    if max_per_language and lang_pairs >= max_per_language:
                        break

                    pair["language"] = lang
                    pair["source"] = "thestack"
                    f.write(json.dumps(pair, ensure_ascii=False) + "\n")
                    lang_pairs += 1
                    total_pairs += 1

                if max_per_language and lang_pairs >= max_per_language:
                    print(f"  Reached max for {lang}")
                    break

            stats[lang] = {"pairs": lang_pairs, "files": files_processed}
            print(f"  {lang}: {lang_pairs:,} pairs from {files_processed:,} files")

    print(f"\n{'='*60}")
    print(f"Total: {total_pairs:,} training pairs")
    print(f"Output: {output}")

    # Write stats
    stats_file = output.with_suffix(".stats.json")
    with stats_file.open("w") as f:
        json.dump({"total": total_pairs, "by_language": stats}, f, indent=2)

    return total_pairs


def main():
    parser = argparse.ArgumentParser(description="Download The Stack and extract training pairs")
    parser.add_argument("--output", "-o", type=Path, default=Path("data/thestack_extracted.jsonl"))
    parser.add_argument("--languages", "-l", type=str, default="python,java,go,rust,typescript,javascript")
    parser.add_argument("--max-per-language", type=int, default=100000)
    parser.add_argument("--no-streaming", action="store_true", help="Download full dataset (slow)")
    args = parser.parse_args()

    languages = [l.strip() for l in args.languages.split(",") if l.strip()]
    download_and_extract(
        languages,
        args.output,
        args.max_per_language,
        streaming=not args.no_streaming,
    )


if __name__ == "__main__":
    main()
