#!/usr/bin/env python3
"""
Extract training data from The Stack dataset.

Extracts (query, code) pairs from docstrings, comments, and function signatures.
Uses streaming to avoid downloading the full 6TB dataset.

Usage:
    # Extract from specific languages
    python scripts/extract_thestack_training.py --languages python rust go --output data/thestack_training.jsonl

    # Extract with limit per language
    python scripts/extract_thestack_training.py --languages python --max-per-lang 50000 --output data/thestack_python.jsonl

Requirements:
    pip install datasets huggingface_hub
    # Must be logged in: huggingface-cli login
"""

import argparse
import json
import random
import re
import sys
from collections import defaultdict
from pathlib import Path
from typing import Dict, Generator, List, Optional, Tuple

try:
    from datasets import load_dataset
except ImportError:
    print("Please install datasets: pip install datasets")
    sys.exit(1)


# Language-specific extraction patterns
EXTRACTORS = {
    "python": {
        "docstring": r'"""(.*?)"""',
        "function": r"def\s+(\w+)\s*\([^)]*\)\s*(?:->.*?)?:",
        "class": r"class\s+(\w+)\s*(?:\([^)]*\))?:",
    },
    "rust": {
        "docstring": r"///\s*(.*?)$",
        "function": r"(?:pub\s+)?fn\s+(\w+)\s*(?:<[^>]*>)?\s*\([^)]*\)",
        "struct": r"(?:pub\s+)?struct\s+(\w+)",
    },
    "go": {
        "docstring": r"//\s*(.*?)$",
        "function": r"func\s+(?:\([^)]*\)\s*)?(\w+)\s*\([^)]*\)",
        "struct": r"type\s+(\w+)\s+struct",
    },
    "javascript": {
        "docstring": r"/\*\*(.*?)\*/",
        "function": r"(?:async\s+)?function\s+(\w+)\s*\([^)]*\)",
        "class": r"class\s+(\w+)",
    },
    "typescript": {
        "docstring": r"/\*\*(.*?)\*/",
        "function": r"(?:async\s+)?function\s+(\w+)\s*(?:<[^>]*>)?\s*\([^)]*\)",
        "class": r"class\s+(\w+)",
    },
    "java": {
        "docstring": r"/\*\*(.*?)\*/",
        "function": r"(?:public|private|protected)?\s*(?:static)?\s*\w+\s+(\w+)\s*\([^)]*\)",
        "class": r"(?:public\s+)?class\s+(\w+)",
    },
    "cpp": {
        "docstring": r"/\*\*(.*?)\*/|///\s*(.*?)$",
        "function": r"(?:\w+\s+)+(\w+)\s*\([^)]*\)\s*(?:const)?(?:\s*override)?(?:\s*\{|;)",
        "class": r"class\s+(\w+)",
    },
    "c": {
        "docstring": r"/\*\*(.*?)\*/|///\s*(.*?)$",
        "function": r"(?:\w+\s+)+(\w+)\s*\([^)]*\)\s*\{",
    },
    "ruby": {
        "docstring": r"#\s*(.*?)$",
        "function": r"def\s+(\w+)",
        "class": r"class\s+(\w+)",
    },
    "php": {
        "docstring": r"/\*\*(.*?)\*/",
        "function": r"(?:public|private|protected)?\s*function\s+(\w+)\s*\([^)]*\)",
        "class": r"class\s+(\w+)",
    },
    "swift": {
        "docstring": r"///\s*(.*?)$",
        "function": r"func\s+(\w+)\s*(?:<[^>]*>)?\s*\([^)]*\)",
        "class": r"class\s+(\w+)",
        "struct": r"struct\s+(\w+)",
    },
    "kotlin": {
        "docstring": r"/\*\*(.*?)\*/",
        "function": r"fun\s+(?:<[^>]*>\s*)?(\w+)\s*\([^)]*\)",
        "class": r"class\s+(\w+)",
    },
    "scala": {
        "docstring": r"/\*\*(.*?)\*/",
        "function": r"def\s+(\w+)\s*(?:\[[^\]]*\])?\s*\([^)]*\)",
        "class": r"class\s+(\w+)",
    },
}

# Map The Stack language names to our extractors
LANG_MAP = {
    "python": "python",
    "rust": "rust",
    "go": "go",
    "javascript": "javascript",
    "typescript": "typescript",
    "java": "java",
    "c++": "cpp",
    "c": "c",
    "ruby": "ruby",
    "php": "php",
    "swift": "swift",
    "kotlin": "kotlin",
    "scala": "scala",
}


def extract_docstring_pairs(
    content: str,
    language: str,
) -> Generator[Tuple[str, str], None, None]:
    """Extract (docstring, code) pairs from file content."""
    extractor = EXTRACTORS.get(LANG_MAP.get(language.lower(), language.lower()))
    if not extractor:
        return

    # Try to find docstrings followed by code
    docstring_pattern = extractor.get("docstring", "")
    if not docstring_pattern:
        return

    # Split into chunks around functions/classes
    lines = content.split("\n")

    current_doc = []
    in_docstring = False
    docstring_end_line = -1

    for i, line in enumerate(lines):
        # Check for docstring
        if language.lower() == "python":
            # Python triple-quote docstrings
            if '"""' in line or "'''" in line:
                quote = '"""' if '"""' in line else "'''"
                count = line.count(quote)
                if count == 2:
                    # Single line docstring
                    match = re.search(f'{quote}(.*?){quote}', line)
                    if match:
                        doc = match.group(1).strip()
                        if len(doc) > 10:
                            current_doc = [doc]
                            docstring_end_line = i
                elif count == 1:
                    if in_docstring:
                        # End of multi-line docstring
                        in_docstring = False
                        docstring_end_line = i
                    else:
                        # Start of multi-line docstring
                        in_docstring = True
                        current_doc = []
                        # Get content after opening quote
                        idx = line.index(quote) + 3
                        if idx < len(line):
                            current_doc.append(line[idx:])
                elif in_docstring:
                    current_doc.append(line)
        else:
            # Comment-based docstrings (///, /**, #)
            if re.match(r'\s*///\s*', line) or re.match(r'\s*#\s*', line):
                doc_match = re.search(r'(?:///|#)\s*(.*)$', line)
                if doc_match:
                    current_doc.append(doc_match.group(1))
                    docstring_end_line = i

        # Check for function/class definition after docstring
        if current_doc and i > docstring_end_line and i <= docstring_end_line + 2:
            for pattern_name in ["function", "class", "struct"]:
                pattern = extractor.get(pattern_name)
                if pattern:
                    match = re.search(pattern, line)
                    if match:
                        # Found a definition - extract code block
                        doc_text = " ".join(current_doc).strip()
                        doc_text = re.sub(r'\s+', ' ', doc_text)

                        # Get the code (next ~20 lines or until blank line)
                        code_lines = []
                        indent_level = len(line) - len(line.lstrip())

                        for j in range(i, min(i + 30, len(lines))):
                            code_line = lines[j]
                            code_lines.append(code_line)

                            # Stop at end of function (dedent in Python, closing brace in others)
                            if j > i:
                                if language.lower() == "python":
                                    if code_line.strip() and not code_line.startswith(" " * (indent_level + 1)):
                                        if not code_line.strip().startswith("#"):
                                            break
                                elif code_line.strip() == "}":
                                    break

                        code = "\n".join(code_lines)

                        if len(doc_text) > 15 and len(code) > 50:
                            yield (doc_text, code)

                        current_doc = []
                        break

        # Reset if we've gone too far past the docstring
        if current_doc and i > docstring_end_line + 3:
            current_doc = []


def extract_function_signature_pairs(
    content: str,
    language: str,
) -> Generator[Tuple[str, str], None, None]:
    """Extract (signature-based query, code) pairs."""
    extractor = EXTRACTORS.get(LANG_MAP.get(language.lower(), language.lower()))
    if not extractor:
        return

    function_pattern = extractor.get("function")
    if not function_pattern:
        return

    lines = content.split("\n")

    for i, line in enumerate(lines):
        match = re.search(function_pattern, line)
        if match:
            func_name = match.group(1)

            # Skip trivial names
            if func_name in ["main", "new", "init", "__init__", "setup", "teardown"]:
                continue
            if len(func_name) < 3:
                continue

            # Convert camelCase/snake_case to natural language query
            query = func_name_to_query(func_name)
            if len(query) < 10:
                continue

            # Extract code block
            code_lines = [line]
            indent_level = len(line) - len(line.lstrip())

            for j in range(i + 1, min(i + 30, len(lines))):
                code_line = lines[j]
                code_lines.append(code_line)

                if language.lower() == "python":
                    if code_line.strip() and not code_line.startswith(" " * (indent_level + 1)):
                        if not code_line.strip().startswith("#"):
                            break
                elif code_line.strip() == "}":
                    break

            code = "\n".join(code_lines)

            if len(code) > 50:
                yield (query, code)


def func_name_to_query(name: str) -> str:
    """Convert function name to natural language query."""
    # Split camelCase
    name = re.sub(r'([a-z])([A-Z])', r'\1 \2', name)
    # Split snake_case
    name = name.replace("_", " ")
    # Lowercase
    name = name.lower()
    # Clean up
    name = re.sub(r'\s+', ' ', name).strip()
    return name


def is_quality_pair(query: str, code: str) -> bool:
    """Check if a (query, code) pair meets quality standards."""
    # Query checks
    if len(query) < 10 or len(query) > 200:
        return False
    if query.lower().startswith(("todo", "fixme", "xxx", "hack")):
        return False
    if re.match(r'^[a-z]+$', query):  # Single word
        return False

    # Code checks
    if len(code) < 50 or len(code) > 5000:
        return False

    # Boilerplate patterns
    boilerplate = [
        r"^returns?\s+(self|the|a|an|true|false|none|null)\b",
        r"^gets?\s+the\s+",
        r"^sets?\s+the\s+",
        r"^creates?\s+(a\s+)?new\s+",
        r"^constructor",
        r"^destructor",
        r"^default",
    ]
    for pattern in boilerplate:
        if re.match(pattern, query.lower()):
            return False

    return True


def process_file(sample: Dict, language: str) -> List[Dict]:
    """Process a single file and extract training pairs."""
    content = sample.get("content", "")
    if not content or len(content) < 100:
        return []

    pairs = []

    # Extract docstring pairs
    for query, code in extract_docstring_pairs(content, language):
        if is_quality_pair(query, code):
            pairs.append({
                "query": query,
                "positive": code,
                "language": language,
                "source": "thestack_docstring",
            })

    # Extract function signature pairs
    for query, code in extract_function_signature_pairs(content, language):
        if is_quality_pair(query, code):
            pairs.append({
                "query": query,
                "positive": code,
                "language": language,
                "source": "thestack_signature",
            })

    return pairs


def main():
    parser = argparse.ArgumentParser(description="Extract training data from The Stack")
    parser.add_argument(
        "--languages",
        nargs="+",
        default=["python", "rust", "go", "javascript", "typescript", "java"],
        help="Languages to extract",
    )
    parser.add_argument(
        "--output",
        type=str,
        default="data/thestack_training.jsonl",
        help="Output file",
    )
    parser.add_argument(
        "--max-per-lang",
        type=int,
        default=100000,
        help="Maximum examples per language",
    )
    parser.add_argument(
        "--max-files",
        type=int,
        default=500000,
        help="Maximum files to process per language",
    )
    args = parser.parse_args()

    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)

    all_pairs = []
    lang_counts = defaultdict(int)

    for language in args.languages:
        print(f"\nProcessing {language}...")

        try:
            # Load dataset in streaming mode
            ds = load_dataset(
                "bigcode/the-stack",
                data_dir=f"data/{language}",
                split="train",
                streaming=True,
            )
        except Exception as e:
            print(f"  Error loading {language}: {e}")
            continue

        file_count = 0
        pair_count = 0

        for sample in ds:
            if file_count >= args.max_files:
                print(f"  Reached max files ({args.max_files})")
                break

            if pair_count >= args.max_per_lang:
                print(f"  Reached max pairs ({args.max_per_lang})")
                break

            pairs = process_file(sample, language)
            for pair in pairs:
                if pair_count < args.max_per_lang:
                    all_pairs.append(pair)
                    pair_count += 1
                    lang_counts[language] += 1

            file_count += 1

            if file_count % 10000 == 0:
                print(f"  Processed {file_count} files, {pair_count} pairs")

        print(f"  {language}: {pair_count} pairs from {file_count} files")

    # Shuffle
    random.shuffle(all_pairs)

    # Write output
    print(f"\nWriting {len(all_pairs)} pairs to {output_path}")
    with output_path.open("w") as f:
        for pair in all_pairs:
            f.write(json.dumps(pair) + "\n")

    print("\nPer-language counts:")
    for lang, count in sorted(lang_counts.items()):
        print(f"  {lang}: {count}")

    print(f"\nTotal: {len(all_pairs)} training pairs")


if __name__ == "__main__":
    main()
