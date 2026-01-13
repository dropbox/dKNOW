#!/usr/bin/env python3
"""
Extract high-value training pairs from priority repositories.

These pairs are UPWEIGHTED in training to ensure the model learns:
1. Rust idioms and patterns
2. Formal verification concepts (specs → code)
3. User's personal coding style

Upweighting strategy:
- Each pair from priority repos appears 3-5x in training
- Formal verification pairs (specs, proofs) get 5x weight
- User's own code gets 3x weight
"""

import argparse
import json
import re
from pathlib import Path
from typing import Iterator


def extract_rust_pairs(content: str, file_path: str) -> list[dict]:
    """Extract docstring-function pairs from Rust code."""
    pairs = []

    # Pattern: /// doc comments followed by fn
    pattern = r'((?:\s*///.*\n)+)\s*((?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?(?:unsafe\s+)?(?:const\s+)?fn\s+\w+[^{]*\{)'

    for match in re.finditer(pattern, content):
        doc_lines = match.group(1)
        fn_start = match.group(2)

        # Clean docstring
        docstring = " ".join(
            line.strip().lstrip("/").strip()
            for line in doc_lines.split("\n")
            if line.strip() and not line.strip().startswith("///!")
        )

        # Skip short or empty docstrings
        if len(docstring) < 20:
            continue

        # Get function body (approximate - find matching braces)
        start_idx = match.end() - 1  # Position of opening {
        brace_count = 1
        end_idx = start_idx + 1

        while end_idx < len(content) and brace_count > 0:
            if content[end_idx] == '{':
                brace_count += 1
            elif content[end_idx] == '}':
                brace_count -= 1
            end_idx += 1

        func_body = fn_start + content[start_idx+1:end_idx]

        if len(func_body) >= 50 and len(func_body) <= 4000:
            pairs.append({
                "query": docstring,
                "positive": func_body,
                "file": file_path,
                "language": "rust",
            })

    return pairs


def extract_lean_pairs(content: str, file_path: str) -> list[dict]:
    """Extract docstring-theorem pairs from Lean code."""
    pairs = []

    # Pattern: /-- doc -/ followed by theorem/lemma/def
    pattern = r'/--\s*(.*?)\s*-/\s*((?:theorem|lemma|def|structure|inductive)\s+\w+[^:]*:[^:=]*(?::=.*?(?=\n(?:theorem|lemma|def|structure|inductive|/-|$)))?)'

    for match in re.finditer(pattern, content, re.DOTALL):
        docstring = match.group(1).strip()
        code = match.group(2).strip()

        if len(docstring) >= 15 and len(code) >= 30:
            pairs.append({
                "query": docstring,
                "positive": code,
                "file": file_path,
                "language": "lean",
                "is_formal": True,
            })

    return pairs


def extract_tla_pairs(content: str, file_path: str) -> list[dict]:
    """Extract comment-spec pairs from TLA+ code."""
    pairs = []

    # Pattern: \* comment followed by operator/definition
    pattern = r'(\\\*.*(?:\n\\\*.*)*)\n\s*(\w+\s*(?:\([^)]*\))?\s*==.*?)(?=\n(?:\\\*|\w+\s*==|$))'

    for match in re.finditer(pattern, content):
        comment = " ".join(
            line.lstrip("\\*").strip()
            for line in match.group(1).split("\n")
            if line.strip()
        )
        spec = match.group(2).strip()

        if len(comment) >= 15 and len(spec) >= 20:
            pairs.append({
                "query": comment,
                "positive": spec,
                "file": file_path,
                "language": "tla",
                "is_formal": True,
            })

    return pairs


def extract_smt_pairs(content: str, file_path: str) -> list[dict]:
    """Extract comment-assertion pairs from SMT-LIB code."""
    pairs = []

    # Pattern: ; comment followed by (assert ...) or (define-fun ...)
    pattern = r'((?:;.*\n)+)\s*(\((?:assert|define-fun|declare-fun|declare-const)[^)]*(?:\([^)]*\)[^)]*)*\))'

    for match in re.finditer(pattern, content):
        comment = " ".join(
            line.lstrip(";").strip()
            for line in match.group(1).split("\n")
            if line.strip()
        )
        smt = match.group(2).strip()

        if len(comment) >= 15 and len(smt) >= 20:
            pairs.append({
                "query": comment,
                "positive": smt,
                "file": file_path,
                "language": "smt",
                "is_formal": True,
            })

    return pairs


def process_repo(repo_path: Path, weight: int = 1) -> list[dict]:
    """Process a repository and extract training pairs."""
    pairs = []

    # Rust files
    for rust_file in repo_path.rglob("*.rs"):
        if "target" in str(rust_file) or "test" in rust_file.name.lower():
            continue
        try:
            content = rust_file.read_text(encoding="utf-8", errors="ignore")
            file_pairs = extract_rust_pairs(content, str(rust_file.relative_to(repo_path)))
            pairs.extend(file_pairs)
        except Exception as e:
            print(f"  Error processing {rust_file}: {e}")

    # Lean files
    for lean_file in repo_path.rglob("*.lean"):
        if "build" in str(lean_file):
            continue
        try:
            content = lean_file.read_text(encoding="utf-8", errors="ignore")
            file_pairs = extract_lean_pairs(content, str(lean_file.relative_to(repo_path)))
            pairs.extend(file_pairs)
        except Exception as e:
            print(f"  Error processing {lean_file}: {e}")

    # TLA+ files
    for tla_file in repo_path.rglob("*.tla"):
        try:
            content = tla_file.read_text(encoding="utf-8", errors="ignore")
            file_pairs = extract_tla_pairs(content, str(tla_file.relative_to(repo_path)))
            pairs.extend(file_pairs)
        except Exception as e:
            print(f"  Error processing {tla_file}: {e}")

    # SMT files
    for smt_file in list(repo_path.rglob("*.smt2")) + list(repo_path.rglob("*.smt")):
        try:
            content = smt_file.read_text(encoding="utf-8", errors="ignore")
            file_pairs = extract_smt_pairs(content, str(smt_file.relative_to(repo_path)))
            pairs.extend(file_pairs)
        except Exception as e:
            print(f"  Error processing {smt_file}: {e}")

    # Apply weight (duplicate pairs)
    if weight > 1:
        weighted_pairs = []
        for pair in pairs:
            pair["weight"] = weight
            for _ in range(weight):
                weighted_pairs.append(pair.copy())
        return weighted_pairs

    return pairs


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--repos-dir", type=Path, default=Path("data/priority_repos"))
    parser.add_argument("--output", type=Path, default=Path("data/priority_training.jsonl"))
    parser.add_argument("--formal-weight", type=int, default=5, help="Weight for formal verification repos")
    parser.add_argument("--user-weight", type=int, default=3, help="Weight for user code repos")
    args = parser.parse_args()

    # Repos with formal verification focus (5x weight)
    formal_repos = ["dashprove", "lean5", "z4", "tRust", "tla2", "kani_fast", "gamma-crown"]

    # Other user repos (3x weight)
    other_repos = ["dashflow", "chunker", "sg"]

    all_pairs = []

    for repo_name in args.repos_dir.iterdir():
        if not repo_name.is_dir() or repo_name.name.startswith("."):
            continue

        if repo_name.name in formal_repos:
            weight = args.formal_weight
            tag = "FORMAL"
        elif repo_name.name in other_repos:
            weight = args.user_weight
            tag = "USER"
        else:
            weight = args.user_weight
            tag = "USER"

        print(f"\n[{tag}] Processing {repo_name.name} (weight={weight})...")
        pairs = process_repo(repo_name, weight=weight)
        print(f"  Extracted {len(pairs)} pairs (after weighting)")
        all_pairs.extend(pairs)

    # Also process the sg repo itself
    sg_path = Path(__file__).parent.parent
    if (sg_path / "crates").exists():
        print(f"\n[USER] Processing sg (this repo) (weight={args.user_weight})...")
        pairs = process_repo(sg_path, weight=args.user_weight)
        print(f"  Extracted {len(pairs)} pairs (after weighting)")
        all_pairs.extend(pairs)

    # Write output
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("w", encoding="utf-8") as f:
        for pair in all_pairs:
            f.write(json.dumps(pair, ensure_ascii=False) + "\n")

    # Stats
    formal_count = sum(1 for p in all_pairs if p.get("is_formal"))
    rust_count = sum(1 for p in all_pairs if p.get("language") == "rust")

    print(f"\n{'='*60}")
    print(f"Total pairs: {len(all_pairs)}")
    print(f"  Formal verification: {formal_count}")
    print(f"  Rust: {rust_count}")
    print(f"Output: {args.output}")


if __name__ == "__main__":
    main()
