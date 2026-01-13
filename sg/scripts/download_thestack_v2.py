#!/usr/bin/env python3
"""
Download The Stack and extract docstring-code pairs.
Uses the correct dataset format after HF auth.
"""

import argparse
import json
import re
from pathlib import Path


def extract_pairs(content: str, language: str) -> list[dict]:
    """Extract docstring-function pairs from code."""
    pairs = []

    if language == "python":
        # Python: """docstring""" followed by def
        pattern = r'"""(.*?)"""\s*\n\s*def\s+(\w+)\s*\([^)]*\).*?(?=\ndef\s|\nclass\s|\Z)'
        for match in re.finditer(pattern, content, re.DOTALL):
            docstring = match.group(1).strip()
            if len(docstring) >= 20 and len(match.group(0)) >= 50:
                pairs.append({
                    "query": docstring[:500],
                    "positive": match.group(0)[:2000],
                    "language": language,
                })

    elif language == "rust":
        # Rust: /// comments followed by fn
        pattern = r'((?:///.*\n)+)\s*(?:pub\s+)?(?:async\s+)?fn\s+(\w+)[^{]*\{'
        for match in re.finditer(pattern, content):
            docstring = " ".join(
                line.strip().lstrip("/").strip()
                for line in match.group(1).split("\n")
                if line.strip()
            )
            if len(docstring) >= 20:
                start = match.start()
                pairs.append({
                    "query": docstring[:500],
                    "positive": content[start:start+2000],
                    "language": language,
                })

    elif language in ("java", "typescript", "javascript"):
        # JSDoc: /** */ followed by function
        pattern = r'/\*\*(.*?)\*/\s*(?:public\s+|private\s+|export\s+)?(?:async\s+)?(?:function\s+)?(?:\w+\s+)?(\w+)\s*\('
        for match in re.finditer(pattern, content, re.DOTALL):
            docstring = re.sub(r'\s*\*\s*', ' ', match.group(1))
            docstring = re.sub(r'@\w+\s+[^\n]*', '', docstring).strip()
            if len(docstring) >= 20:
                start = match.start()
                pairs.append({
                    "query": docstring[:500],
                    "positive": content[start:start+2000],
                    "language": language,
                })

    elif language == "go":
        # Go: // comments followed by func
        pattern = r'((?://.*\n)+)func\s+(?:\([^)]+\)\s*)?(\w+)\s*\('
        for match in re.finditer(pattern, content):
            docstring = " ".join(
                line.strip().lstrip("/").strip()
                for line in match.group(1).split("\n")
                if line.strip()
            )
            if len(docstring) >= 20:
                start = match.start()
                pairs.append({
                    "query": docstring[:500],
                    "positive": content[start:start+2000],
                    "language": language,
                })

    return pairs


def download_language(language: str, output_file, max_pairs: int) -> int:
    """Download and extract pairs for a language."""
    from datasets import load_dataset

    print(f"\n{'='*60}")
    print(f"Processing {language}...")

    # Map language names to The Stack data_dir format
    lang_map = {
        "python": "data/python",
        "rust": "data/rust",
        "java": "data/java",
        "javascript": "data/javascript",
        "typescript": "data/typescript",
        "go": "data/go",
        "ruby": "data/ruby",
        "php": "data/php",
    }

    data_dir = lang_map.get(language, f"data/{language}")

    try:
        ds = load_dataset(
            "bigcode/the-stack",
            data_dir=data_dir,
            split="train",
            streaming=True,
        )
    except Exception as e:
        print(f"  Error: {e}")
        return 0

    pairs_count = 0
    files_processed = 0

    for example in ds:
        files_processed += 1

        if files_processed % 10000 == 0:
            print(f"  Files: {files_processed:,}, Pairs: {pairs_count:,}")

        content = example.get("content", "")
        if not content or len(content) < 100:
            continue

        # Extract pairs
        pairs = extract_pairs(content, language)

        for pair in pairs:
            pair["source"] = "thestack"
            output_file.write(json.dumps(pair, ensure_ascii=False) + "\n")
            pairs_count += 1

            if pairs_count >= max_pairs:
                print(f"  Reached {max_pairs:,} pairs for {language}")
                return pairs_count

    print(f"  {language}: {pairs_count:,} pairs from {files_processed:,} files")
    return pairs_count


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", "-o", type=Path, default=Path("data/thestack_extracted.jsonl"))
    parser.add_argument("--languages", "-l", default="python,rust,java,go,javascript,typescript")
    parser.add_argument("--max-per-language", type=int, default=50000)
    args = parser.parse_args()

    languages = [l.strip() for l in args.languages.split(",")]

    args.output.parent.mkdir(parents=True, exist_ok=True)

    total = 0
    with args.output.open("w", encoding="utf-8") as f:
        for lang in languages:
            count = download_language(lang, f, args.max_per_language)
            total += count

    print(f"\n{'='*60}")
    print(f"Total: {total:,} pairs")
    print(f"Output: {args.output}")


if __name__ == "__main__":
    main()
