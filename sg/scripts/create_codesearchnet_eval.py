#!/usr/bin/env python3
"""
Create a CodeSearchNet evaluation corpus and spec.

Takes validation data and creates:
1. A corpus directory with code files
2. An eval spec JSON for use with `sg eval`

Usage:
    python scripts/create_codesearchnet_eval.py --sample 50 --output eval/codesearchnet_sample
"""

import argparse
import json
import hashlib
from pathlib import Path
from collections import defaultdict


def load_validation_data(path: Path, sample_per_lang: int) -> dict[str, list[dict]]:
    """Load validation data, grouped by language."""
    by_lang = defaultdict(list)

    with open(path) as f:
        for line in f:
            item = json.loads(line)
            lang = item.get("language", "unknown")
            if len(by_lang[lang]) < sample_per_lang:
                by_lang[lang].append(item)

    return dict(by_lang)


def create_corpus_and_spec(data_by_lang: dict, output_dir: Path):
    """Create corpus files and eval spec."""
    corpus_dir = output_dir / "corpus"
    corpus_dir.mkdir(parents=True, exist_ok=True)

    queries = []
    file_index = 0

    ext_map = {
        "python": "py", "java": "java", "go": "go", "javascript": "js",
        "php": "php", "ruby": "rb", "rust": "rs", "typescript": "ts"
    }

    for lang, items in data_by_lang.items():
        for item in items:
            code = item["positive"]
            query = item["query"]
            func_name = item.get("func_name", "")

            # Create filename from hash to ensure uniqueness
            code_hash = hashlib.md5(code.encode()).hexdigest()[:8]
            ext = ext_map.get(lang, "txt")
            filename = f"{lang}_{file_index}_{code_hash}.{ext}"
            filepath = corpus_dir / filename

            # Write code to file
            filepath.write_text(code)

            # Create query entry
            # Truncate very long queries
            short_query = query[:200] if len(query) > 200 else query
            queries.append({
                "query": short_query,
                "relevant": [f"corpus/{filename}"],
                "description": f"{lang}: {func_name}" if func_name else f"{lang} function"
            })

            file_index += 1

    # Create eval spec
    spec = {
        "corpus": str(corpus_dir),
        "description": f"CodeSearchNet validation ({file_index} queries across {len(data_by_lang)} languages)",
        "queries": queries
    }

    spec_path = output_dir / "eval_spec.json"
    spec_path.write_text(json.dumps(spec, indent=2))

    return spec_path, file_index


def main():
    parser = argparse.ArgumentParser(description="Create CodeSearchNet eval corpus")
    parser.add_argument("--data", default="data/combined_training.val.jsonl", help="Validation data path")
    parser.add_argument("--sample", type=int, default=10, help="Samples per language")
    parser.add_argument("--output", default="eval/codesearchnet_sample", help="Output directory")
    parser.add_argument("--languages", help="Comma-separated languages (default: all)")
    args = parser.parse_args()

    print(f"Loading validation data from {args.data}...")
    all_data = load_validation_data(Path(args.data), args.sample)

    if args.languages:
        selected = args.languages.split(",")
        all_data = {k: v for k, v in all_data.items() if k in selected}

    print(f"  Loaded {sum(len(v) for v in all_data.values())} pairs across {len(all_data)} languages")
    for lang, items in sorted(all_data.items()):
        print(f"    {lang}: {len(items)}")

    output_dir = Path(args.output)
    spec_path, n_files = create_corpus_and_spec(all_data, output_dir)

    print(f"\nCreated evaluation corpus:")
    print(f"  Corpus: {output_dir}/corpus/ ({n_files} files)")
    print(f"  Spec: {spec_path}")
    print(f"\nRun evaluation with:")
    print(f"  sg eval --spec {spec_path} --hybrid")
    print(f"  sg eval --spec {spec_path} --model-path checkpoints/xtr-improved-merged --hybrid")


if __name__ == "__main__":
    main()
