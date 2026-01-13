#!/usr/bin/env python3
"""
Compare MLX vs PyTorch training loss curves.

Phase 3, Step 2: Train both frameworks for 500 steps and compare:
1. Loss values should be similar
2. Loss curves should follow similar patterns
3. Training should converge comparably
"""

import argparse
import json
import random
import time
from pathlib import Path
from typing import Dict, List, Tuple

import numpy as np


def train_pytorch(
    data_path: Path,
    steps: int,
    batch_size: int,
    max_length: int,
    model_name: str = "google/xtr-base-en",
) -> Tuple[List[float], float]:
    """Train with PyTorch and return loss history."""
    import torch
    from transformers import T5EncoderModel, AutoTokenizer

    print("=" * 60)
    print("PyTorch Training")
    print("=" * 60)

    device = torch.device("mps" if torch.backends.mps.is_available() else "cpu")
    print(f"Device: {device}")

    # Load model
    tokenizer = AutoTokenizer.from_pretrained(model_name)
    model = T5EncoderModel.from_pretrained(model_name).to(device)
    model.train()

    # Only train last 2 layers for speed
    for name, param in model.named_parameters():
        if "block.10" in name or "block.11" in name or "final" in name:
            param.requires_grad = True
        else:
            param.requires_grad = False

    optimizer = torch.optim.AdamW(
        [p for p in model.parameters() if p.requires_grad],
        lr=2e-5,
        weight_decay=0.01,
    )

    # Load data
    examples = []
    with open(data_path, "r") as f:
        for i, line in enumerate(f):
            if i >= 10000:  # Only use first 10K for speed
                break
            try:
                data = json.loads(line.strip())
                if "query" in data and "positive" in data:
                    examples.append(data)
            except:
                continue

    print(f"Loaded {len(examples)} examples")

    def get_batch():
        batch = random.sample(examples, batch_size)
        queries = [ex["query"] for ex in batch]
        positives = [ex["positive"] for ex in batch]

        q_enc = tokenizer(
            queries, padding=True, truncation=True, max_length=max_length, return_tensors="pt"
        ).to(device)
        p_enc = tokenizer(
            positives, padding=True, truncation=True, max_length=max_length, return_tensors="pt"
        ).to(device)

        return q_enc, p_enc

    def contrastive_loss(q_emb, p_emb, temperature=0.07):
        # Mean pool
        q_vec = q_emb.mean(dim=1)
        p_vec = p_emb.mean(dim=1)

        # Normalize
        q_vec = q_vec / q_vec.norm(dim=-1, keepdim=True)
        p_vec = p_vec / p_vec.norm(dim=-1, keepdim=True)

        # Similarity matrix
        logits = q_vec @ p_vec.T / temperature

        # Labels: diagonal
        labels = torch.arange(len(q_vec), device=device)
        loss = torch.nn.functional.cross_entropy(logits, labels)
        return loss

    # Training loop
    losses = []
    start_time = time.time()

    for step in range(steps):
        q_enc, p_enc = get_batch()

        q_emb = model(**q_enc).last_hidden_state
        p_emb = model(**p_enc).last_hidden_state

        loss = contrastive_loss(q_emb, p_emb)

        optimizer.zero_grad()
        loss.backward()
        optimizer.step()

        loss_val = loss.item()
        losses.append(loss_val)

        if (step + 1) % 50 == 0:
            elapsed = time.time() - start_time
            print(f"Step {step + 1}/{steps}: Loss = {loss_val:.4f}, "
                  f"Time = {elapsed:.1f}s, Samples/s = {(step + 1) * batch_size / elapsed:.1f}")

    total_time = time.time() - start_time
    print(f"\nPyTorch: {steps} steps in {total_time:.1f}s ({steps * batch_size / total_time:.1f} samples/s)")

    return losses, total_time


def train_mlx(
    data_path: Path,
    steps: int,
    batch_size: int,
    max_length: int,
    model_name: str = "google/xtr-base-en",
) -> Tuple[List[float], float]:
    """Train with MLX and return loss history."""
    import mlx.core as mx
    import mlx.nn as nn
    import mlx.optimizers as optim
    from transformers import AutoTokenizer
    import sys
    sys.path.insert(0, str(Path(__file__).parent))
    from train_xtr_mlx import T5Encoder, contrastive_loss

    print("=" * 60)
    print("MLX Training")
    print("=" * 60)

    # Load model
    tokenizer = AutoTokenizer.from_pretrained(model_name)
    model = T5Encoder.from_pretrained(model_name, use_lora=True, lora_r=8, lora_alpha=16)

    # Count trainable params
    def count_params(params, prefix=""):
        total = 0
        if isinstance(params, dict):
            for k, v in params.items():
                total += count_params(v, f"{prefix}.{k}")
        elif isinstance(params, list):
            for i, v in enumerate(params):
                total += count_params(v, f"{prefix}.{i}")
        elif isinstance(params, mx.array):
            total += params.size
        return total

    total = count_params(model.parameters())
    trainable = count_params(model.trainable_parameters())
    print(f"Trainable: {trainable:,} / {total:,} ({100*trainable/max(1,total):.2f}%)")

    optimizer = optim.AdamW(learning_rate=2e-5, weight_decay=0.01)

    # Load data
    examples = []
    with open(data_path, "r") as f:
        for i, line in enumerate(f):
            if i >= 10000:
                break
            try:
                data = json.loads(line.strip())
                if "query" in data and "positive" in data:
                    examples.append(data)
            except:
                continue

    print(f"Loaded {len(examples)} examples")

    def get_batch():
        batch = random.sample(examples, batch_size)
        queries = [ex["query"] for ex in batch]
        positives = [ex["positive"] for ex in batch]

        q_enc = tokenizer(
            queries, padding=True, truncation=True, max_length=max_length, return_tensors="np"
        )
        p_enc = tokenizer(
            positives, padding=True, truncation=True, max_length=max_length, return_tensors="np"
        )

        return {
            "query_ids": mx.array(q_enc["input_ids"]),
            "query_mask": mx.array(q_enc["attention_mask"], dtype=mx.float32),
            "pos_ids": mx.array(p_enc["input_ids"]),
            "pos_mask": mx.array(p_enc["attention_mask"], dtype=mx.float32),
        }

    def loss_fn(model, batch):
        q_emb = model(batch["query_ids"], batch["query_mask"])
        p_emb = model(batch["pos_ids"], batch["pos_mask"])
        return contrastive_loss(q_emb, p_emb, batch["query_mask"], batch["pos_mask"], temperature=0.07)

    loss_and_grad = nn.value_and_grad(model, loss_fn)

    # Training loop
    losses = []
    start_time = time.time()

    for step in range(steps):
        batch = get_batch()
        loss, grads = loss_and_grad(model, batch)

        # NOTE: grads is nested dict matching model structure. Only trainable
        # params (LoRA) have gradients due to freeze/unfreeze. Pass full grads.
        optimizer.update(model, grads)
        mx.eval(loss, model.parameters())

        loss_val = float(loss)
        losses.append(loss_val)

        if (step + 1) % 50 == 0:
            elapsed = time.time() - start_time
            print(f"Step {step + 1}/{steps}: Loss = {loss_val:.4f}, "
                  f"Time = {elapsed:.1f}s, Samples/s = {(step + 1) * batch_size / elapsed:.1f}")

    total_time = time.time() - start_time
    print(f"\nMLX: {steps} steps in {total_time:.1f}s ({steps * batch_size / total_time:.1f} samples/s)")

    return losses, total_time


def compare_losses(pt_losses: List[float], mlx_losses: List[float]) -> Dict[str, float]:
    """Compare loss curves."""
    pt = np.array(pt_losses)
    mlx = np.array(mlx_losses)

    # Use windows for smoothing
    window = min(50, len(pt) // 5)

    def smooth(arr, w):
        return np.convolve(arr, np.ones(w)/w, mode='valid')

    pt_smooth = smooth(pt, window)
    mlx_smooth = smooth(mlx, window)

    metrics = {
        "pt_initial_loss": float(pt[:10].mean()),
        "pt_final_loss": float(pt[-50:].mean()),
        "mlx_initial_loss": float(mlx[:10].mean()),
        "mlx_final_loss": float(mlx[-50:].mean()),
        "pt_loss_reduction": float((pt[:10].mean() - pt[-50:].mean()) / pt[:10].mean() * 100),
        "mlx_loss_reduction": float((mlx[:10].mean() - mlx[-50:].mean()) / mlx[:10].mean() * 100),
        "curve_correlation": float(np.corrcoef(pt_smooth, mlx_smooth)[0, 1]),
    }

    return metrics


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--data", type=str, default="data/training_improved.jsonl")
    parser.add_argument("--steps", type=int, default=300)
    parser.add_argument("--batch-size", type=int, default=8)
    parser.add_argument("--max-length", type=int, default=128)
    parser.add_argument("--seed", type=int, default=42)
    args = parser.parse_args()

    random.seed(args.seed)
    np.random.seed(args.seed)

    data_path = Path(args.data)
    if not data_path.exists():
        print(f"Data file not found: {data_path}")
        return 1

    # Train MLX first (usually faster)
    mlx_losses, mlx_time = train_mlx(
        data_path, args.steps, args.batch_size, args.max_length
    )

    # Reset seed for fair comparison
    random.seed(args.seed)
    np.random.seed(args.seed)

    # Train PyTorch
    pt_losses, pt_time = train_pytorch(
        data_path, args.steps, args.batch_size, args.max_length
    )

    # Compare
    print("\n" + "=" * 60)
    print("Comparison Results")
    print("=" * 60)

    metrics = compare_losses(pt_losses, mlx_losses)

    print(f"\nPyTorch:")
    print(f"  Initial loss: {metrics['pt_initial_loss']:.4f}")
    print(f"  Final loss:   {metrics['pt_final_loss']:.4f}")
    print(f"  Reduction:    {metrics['pt_loss_reduction']:.1f}%")

    print(f"\nMLX:")
    print(f"  Initial loss: {metrics['mlx_initial_loss']:.4f}")
    print(f"  Final loss:   {metrics['mlx_final_loss']:.4f}")
    print(f"  Reduction:    {metrics['mlx_loss_reduction']:.1f}%")

    print(f"\nCurve correlation: {metrics['curve_correlation']:.4f}")
    print(f"Speedup: {pt_time / mlx_time:.2f}x")

    # Save results
    results = {
        "pytorch_losses": pt_losses,
        "mlx_losses": mlx_losses,
        "pytorch_time": pt_time,
        "mlx_time": mlx_time,
        "metrics": metrics,
        "config": {
            "steps": args.steps,
            "batch_size": args.batch_size,
            "max_length": args.max_length,
            "seed": args.seed,
        }
    }

    output_path = Path("data/loss_comparison.json")
    with output_path.open("w") as f:
        json.dump(results, f, indent=2)
    print(f"\nResults saved to: {output_path}")

    # Verdict
    print("\n" + "=" * 60)
    both_converge = (metrics['pt_loss_reduction'] > 10 and metrics['mlx_loss_reduction'] > 10)
    similar_final = abs(metrics['pt_final_loss'] - metrics['mlx_final_loss']) < 0.5

    if both_converge and similar_final:
        print("VALIDATION PASSED: Both frameworks converge similarly")
    else:
        print("VALIDATION ISSUES: Check loss curves")
    print("=" * 60)

    return 0


if __name__ == "__main__":
    exit(main())
