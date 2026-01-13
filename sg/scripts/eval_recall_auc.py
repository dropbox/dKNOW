#!/usr/bin/env python3
"""
Evaluate Recall@K AUC for code search models.

Recall@K AUC is the area under the Recall@K curve, which measures how well
the model surfaces relevant results across all K values (not just K=1).

Usage:
    python scripts/eval_recall_auc.py --spec eval/code_queries.json
    python scripts/eval_recall_auc.py --spec eval/code_queries.json --model-path checkpoints/xtr-codesearchnet-v1

For CodeSearchNet evaluation:
    python scripts/eval_recall_auc.py --codesearchnet --language python
"""

import argparse
import json
import subprocess
import time
from pathlib import Path
from typing import Dict, List, Optional


def run_sg_search(
    query: str,
    corpus_path: str,
    model_path: Optional[str] = None,
    top_k: int = 100,
) -> tuple[list[dict], float]:
    """Run sg search and return results with latency."""
    # Use release binary from cargo build
    sg_binary = Path(__file__).parent.parent / "target" / "release" / "sg"
    if not sg_binary.exists():
        sg_binary = "sg"  # Fall back to PATH

    cmd = [str(sg_binary), "search", query, "--path", corpus_path, "--top", str(top_k), "--json"]

    if model_path:
        cmd.extend(["--model-path", model_path])

    start = time.perf_counter()
    result = subprocess.run(cmd, capture_output=True, text=True)
    latency = (time.perf_counter() - start) * 1000  # ms

    if result.returncode != 0:
        print(f"  Error: {result.stderr}")
        return [], latency

    try:
        results = json.loads(result.stdout)
        return results.get("results", []), latency
    except json.JSONDecodeError:
        return [], latency


def compute_recall_at_k(results: list[dict], relevant: list[str], k: int) -> float:
    """Compute Recall@K for a single query."""
    if not relevant:
        return 0.0

    top_k_files = [r.get("file", r.get("path", "")) for r in results[:k]]

    # Check if any relevant file is in top-k
    for rel in relevant:
        for top_file in top_k_files:
            if rel in top_file or top_file.endswith(rel):
                return 1.0

    return 0.0


def compute_recall_auc(results: list[dict], relevant: list[str], max_k: int = 100) -> float:
    """Compute area under Recall@K curve."""
    if not results:
        return 0.0

    recalls = []
    for k in range(1, min(max_k + 1, len(results) + 1)):
        recalls.append(compute_recall_at_k(results, relevant, k))

    if not recalls:
        return 0.0

    return sum(recalls) / len(recalls)


def find_relevant_position(results: list[dict], relevant: list[str]) -> int:
    """Find position of first relevant result (1-indexed), or 0 if not found."""
    for i, r in enumerate(results):
        file_path = r.get("file", r.get("path", ""))
        for rel in relevant:
            if rel in file_path or file_path.endswith(rel):
                return i + 1
    return 0


def evaluate_spec(
    spec_path: Path,
    model_path: Optional[str] = None,
    max_k: int = 100,
) -> dict:
    """Evaluate a query spec file."""
    with spec_path.open() as f:
        spec = json.load(f)

    corpus = spec.get("corpus", "")
    queries = spec.get("queries", [])

    print(f"\nEvaluating: {spec_path.name}")
    print(f"Corpus: {corpus}")
    print(f"Queries: {len(queries)}")
    print(f"Model: {model_path or 'default'}")
    print("-" * 60)

    # Determine corpus path
    if corpus == "crates":
        corpus_path = str(Path.cwd() / "crates")
    else:
        corpus_path = corpus

    results = {
        "recall_at_k": {1: [], 5: [], 10: [], 20: [], 50: [], 100: []},
        "recall_auc": [],
        "mrr": [],
        "latency_ms": [],
        "queries": [],
    }

    for q in queries:
        query_text = q["query"]
        relevant = q["relevant"]

        # Run search
        search_results, latency = run_sg_search(
            query_text, corpus_path, model_path, top_k=max_k
        )

        # Compute metrics
        position = find_relevant_position(search_results, relevant)
        mrr = 1.0 / position if position > 0 else 0.0
        auc = compute_recall_auc(search_results, relevant, max_k)

        for k in results["recall_at_k"]:
            recall = compute_recall_at_k(search_results, relevant, k)
            results["recall_at_k"][k].append(recall)

        results["recall_auc"].append(auc)
        results["mrr"].append(mrr)
        results["latency_ms"].append(latency)

        results["queries"].append({
            "query": query_text,
            "position": position,
            "mrr": mrr,
            "recall_auc": auc,
            "latency_ms": latency,
        })

        status = "HIT" if position > 0 else "MISS"
        print(f"  [{status}] pos={position:2d} auc={auc:.3f} {latency:5.0f}ms | {query_text[:50]}")

    # Aggregate
    print("-" * 60)
    print(f"Results (n={len(queries)}):")

    avg_recall_at_k = {k: sum(v) / len(v) if v else 0 for k, v in results["recall_at_k"].items()}
    avg_recall_auc = sum(results["recall_auc"]) / len(results["recall_auc"])
    avg_mrr = sum(results["mrr"]) / len(results["mrr"])
    avg_latency = sum(results["latency_ms"]) / len(results["latency_ms"])
    p50_latency = sorted(results["latency_ms"])[len(results["latency_ms"]) // 2]
    p99_latency = sorted(results["latency_ms"])[int(len(results["latency_ms"]) * 0.99)]

    print(f"  R@1:  {avg_recall_at_k[1]:.3f}")
    print(f"  R@5:  {avg_recall_at_k[5]:.3f}")
    print(f"  R@10: {avg_recall_at_k[10]:.3f}")
    print(f"  R@100: {avg_recall_at_k[100]:.3f}")
    print(f"  Recall@K AUC: {avg_recall_auc:.3f}")
    print(f"  MRR:  {avg_mrr:.3f}")
    print(f"  Latency: p50={p50_latency:.0f}ms, p99={p99_latency:.0f}ms, avg={avg_latency:.0f}ms")

    return {
        "spec": str(spec_path),
        "model": model_path,
        "num_queries": len(queries),
        "recall_at_1": avg_recall_at_k[1],
        "recall_at_5": avg_recall_at_k[5],
        "recall_at_10": avg_recall_at_k[10],
        "recall_at_100": avg_recall_at_k[100],
        "recall_auc": avg_recall_auc,
        "mrr": avg_mrr,
        "latency_p50_ms": p50_latency,
        "latency_p99_ms": p99_latency,
        "queries": results["queries"],
    }


def main():
    parser = argparse.ArgumentParser(description="Evaluate Recall@K AUC")
    parser.add_argument("--spec", type=Path, help="Query spec JSON file")
    parser.add_argument("--model-path", type=str, help="Custom model checkpoint")
    parser.add_argument("--max-k", type=int, default=100, help="Max K for evaluation")
    parser.add_argument("--output", "-o", type=Path, help="Output JSON file")
    args = parser.parse_args()

    if not args.spec:
        # Default: evaluate all specs
        eval_dir = Path("eval")
        specs = list(eval_dir.glob("*_queries.json"))
    else:
        specs = [args.spec]

    all_results = []
    for spec in specs:
        result = evaluate_spec(spec, args.model_path, args.max_k)
        all_results.append(result)

    if args.output:
        with args.output.open("w") as f:
            json.dump(all_results, f, indent=2)
        print(f"\nResults saved to: {args.output}")

    # Summary
    print("\n" + "=" * 60)
    print("SUMMARY")
    print("=" * 60)
    for r in all_results:
        print(f"{Path(r['spec']).stem}:")
        print(f"  R@1={r['recall_at_1']:.3f}  R@10={r['recall_at_10']:.3f}  AUC={r['recall_auc']:.3f}  MRR={r['mrr']:.3f}")


if __name__ == "__main__":
    main()
