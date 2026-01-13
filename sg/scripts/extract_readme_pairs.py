#!/usr/bin/env python3
"""
Extract README section -> code pairs for training embeddings.

Usage:
    python scripts/extract_readme_pairs.py ~/repo1 ~/repo2 -o data/readme_pairs.jsonl
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Iterable, Optional

try:
    from extract_rust_training_data import extract_rust_pairs, check_license
except ImportError:
    print(
        "Error: extract_rust_training_data.py not found. Run from repo root.",
        file=sys.stderr,
    )
    sys.exit(1)


@dataclass
class ReadmePair:
    query: str
    positive: str
    source: str
    file_path: str
    func_name: Optional[str] = None
    language: str = "rust"


def normalize_whitespace(text: str) -> str:
    return re.sub(r"\s+", " ", text).strip()


def clean_markdown(text: str) -> str:
    text = re.sub(r"\[([^\]]+)\]\([^)]+\)", r"\1", text)
    text = re.sub(r"`([^`]+)`", r"\1", text)
    text = re.sub(r"<[^>]+>", " ", text)
    text = text.replace("**", "").replace("__", "")
    text = text.replace("*", "").replace("_", "")
    return normalize_whitespace(text)


def is_rust_path(raw_path: str) -> bool:
    path = raw_path.strip()
    if not path:
        return False
    if path.startswith("#"):
        return False
    if "://" in path:
        return False
    if not path.lower().endswith(".rs"):
        return False
    return True


def normalize_link(raw_path: str) -> str:
    path = raw_path.strip().strip(")>,.")
    path = path.split("#", 1)[0].split("?", 1)[0]
    return path


def iter_readme_files(root: Path) -> Iterable[Path]:
    for path in root.rglob("README*"):
        if not path.is_file():
            continue
        parts = path.parts
        if any(skip in parts for skip in ["target", "build", ".git", "vendor"]):
            continue
        yield path


def parse_sections(text: str) -> list[tuple[Optional[str], str, set[str]]]:
    sections: list[tuple[Optional[str], str, set[str]]] = []
    heading: Optional[str] = None
    body_lines: list[str] = []
    links: set[str] = set()
    in_code_block = False

    def flush_section() -> None:
        nonlocal body_lines, links, heading
        body_text = normalize_whitespace(" ".join(body_lines))
        if body_text or links:
            sections.append((heading, body_text, set(links)))
        body_lines = []
        links = set()

    for line in text.splitlines():
        if re.match(r"^\s*```|^\s*~~~", line):
            in_code_block = not in_code_block
            continue
        if in_code_block:
            continue

        heading_match = re.match(r"^(#{1,6})\s+(.*)$", line)
        if heading_match:
            flush_section()
            heading = heading_match.group(2).strip()
            continue

        for target in re.findall(r"\[[^\]]*\]\(([^)]+)\)", line):
            normalized = normalize_link(target)
            if is_rust_path(normalized):
                links.add(normalized)

        for target in re.findall(r"\b[\w./-]+\.rs\b", line):
            normalized = normalize_link(target)
            if is_rust_path(normalized):
                links.add(normalized)

        body_lines.append(line.strip())

    flush_section()
    return sections


def resolve_link_path(
    raw_path: str, repo_root: Path, readme_path: Path
) -> Optional[Path]:
    raw_path = normalize_link(raw_path)
    if not is_rust_path(raw_path):
        return None

    if raw_path.startswith("/"):
        candidate = (repo_root / raw_path.lstrip("/")).resolve()
    else:
        candidate = (readme_path.parent / raw_path).resolve()

    try:
        candidate.relative_to(repo_root.resolve())
    except ValueError:
        return None

    if not candidate.exists() or not candidate.is_file():
        return None

    return candidate


def build_query(heading: Optional[str], body: str, max_len: int) -> str:
    parts = [heading] if heading else []
    if body:
        parts.append(body)
    query = clean_markdown(" ".join(parts))
    if len(query) > max_len:
        query = query[:max_len].rstrip()
    return query


def extract_pairs_from_readme(
    repo_root: Path,
    readme_path: Path,
    max_pairs_per_file: int,
    min_query_len: int,
    max_query_len: int,
    max_code_len: int,
    verbose: bool = False,
) -> list[ReadmePair]:
    pairs: list[ReadmePair] = []
    try:
        text = readme_path.read_text(encoding="utf-8")
    except Exception as exc:
        print(f"  Warning: Could not read {readme_path}: {exc}", file=sys.stderr)
        return pairs

    for heading, body, links in parse_sections(text):
        if not links:
            continue
        query = build_query(heading, body, max_query_len)
        if len(query) < min_query_len:
            continue

        for link in sorted(links):
            target_path = resolve_link_path(link, repo_root, readme_path)
            if not target_path:
                continue

            rust_pairs = list(extract_rust_pairs(target_path, repo_root=repo_root))
            if rust_pairs:
                for rust_pair in rust_pairs[:max_pairs_per_file]:
                    pairs.append(
                        ReadmePair(
                            query=query,
                            positive=rust_pair.positive,
                            source=str(readme_path),
                            file_path=rust_pair.file_path,
                            func_name=rust_pair.func_name,
                        )
                    )
                continue

            try:
                file_text = target_path.read_text(encoding="utf-8")
            except Exception as exc:
                if verbose:
                    print(f"  Warning: Could not read {target_path}: {exc}")
                continue

            if len(file_text) > max_code_len:
                file_text = file_text[:max_code_len] + "\n// ... truncated"

            pairs.append(
                ReadmePair(
                    query=query,
                    positive=file_text,
                    source=str(readme_path),
                    file_path=str(target_path),
                )
            )

    return pairs


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Extract README section -> code pairs for embedding fine-tuning",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
    python scripts/extract_readme_pairs.py ~/sg ~/rust-warp -o data/readme_pairs.jsonl
    python scripts/extract_readme_pairs.py ~/repos/* -o data/readme_pairs.jsonl -v
        """,
    )
    parser.add_argument("directories", nargs="+", type=Path, help="Repo roots")
    parser.add_argument(
        "--output",
        "-o",
        type=Path,
        default=Path("readme_code_pairs.jsonl"),
        help="Output JSONL file",
    )
    parser.add_argument(
        "--max-pairs-per-file",
        type=int,
        default=3,
        help="Limit README section pairs per linked file",
    )
    parser.add_argument(
        "--min-query-len",
        type=int,
        default=40,
        help="Minimum README query length",
    )
    parser.add_argument(
        "--max-query-len",
        type=int,
        default=500,
        help="Maximum README query length",
    )
    parser.add_argument(
        "--max-code-len",
        type=int,
        default=4000,
        help="Maximum code length when falling back to full file",
    )
    parser.add_argument("--verbose", "-v", action="store_true", help="Verbose output")
    parser.add_argument(
        "--check-license",
        action="store_true",
        help="Only include repos with Apache/MIT/BSD license",
    )
    args = parser.parse_args()

    all_pairs: list[ReadmePair] = []
    repo_stats: list[tuple[str, int]] = []

    for repo_root in args.directories:
        if not repo_root.exists():
            print(f"Warning: {repo_root} does not exist, skipping", file=sys.stderr)
            continue
        if not repo_root.is_dir():
            print(f"Warning: {repo_root} is not a directory, skipping", file=sys.stderr)
            continue

        if args.check_license:
            license_type = check_license(repo_root)
            if not license_type:
                print(f"Skipping {repo_root.name}: no Apache/MIT/BSD license found")
                continue
            if args.verbose:
                print(f"Found {license_type} license in {repo_root.name}")

        readmes = list(iter_readme_files(repo_root))
        if args.verbose:
            print(f"Scanning {repo_root} ({len(readmes)} README files)...")

        repo_pairs: list[ReadmePair] = []
        for readme_path in readmes:
            repo_pairs.extend(
                extract_pairs_from_readme(
                    repo_root=repo_root,
                    readme_path=readme_path,
                    max_pairs_per_file=args.max_pairs_per_file,
                    min_query_len=args.min_query_len,
                    max_query_len=args.max_query_len,
                    max_code_len=args.max_code_len,
                    verbose=args.verbose,
                )
            )

        if repo_pairs:
            all_pairs.extend(repo_pairs)
            repo_stats.append((repo_root.name, len(repo_pairs)))
            print(f"{repo_root.name}: {len(repo_pairs)} pairs")
        else:
            print(f"{repo_root.name}: 0 pairs")

    with open(args.output, "w") as output_file:
        for pair in all_pairs:
            output_file.write(json.dumps(asdict(pair)) + "\n")

    print(f"\nTotal: {len(all_pairs)} pairs written to {args.output}")
    if repo_stats:
        print("\nPer-repo breakdown:")
        for name, count in sorted(repo_stats, key=lambda x: -x[1]):
            print(f"  {name}: {count} pairs")


if __name__ == "__main__":
    main()
