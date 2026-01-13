#!/usr/bin/env python3
"""
Validate MLX vs PyTorch model outputs.

Phase 3, Step 1: Ensure MLX and PyTorch produce equivalent outputs
on the same inputs before trusting MLX training results.
"""

import argparse
import json
import time
from pathlib import Path
from typing import Dict, Tuple

import numpy as np


def load_pytorch_model(model_name: str):
    """Load PyTorch T5 encoder."""
    import torch
    from transformers import T5EncoderModel, AutoTokenizer

    device = torch.device("mps" if torch.backends.mps.is_available() else "cpu")
    tokenizer = AutoTokenizer.from_pretrained(model_name)
    model = T5EncoderModel.from_pretrained(model_name).to(device)
    model.eval()

    return model, tokenizer, device


def load_mlx_model(model_name: str):
    """Load MLX T5 encoder."""
    import mlx.core as mx
    from transformers import AutoTokenizer
    import sys
    sys.path.insert(0, str(Path(__file__).parent))
    from train_xtr_mlx import T5Encoder

    tokenizer = AutoTokenizer.from_pretrained(model_name)
    model = T5Encoder.from_pretrained(model_name, use_lora=False)

    return model, tokenizer


def encode_pytorch(model, tokenizer, texts: list, device) -> np.ndarray:
    """Encode texts with PyTorch model."""
    import torch

    inputs = tokenizer(
        texts,
        padding=True,
        truncation=True,
        max_length=128,
        return_tensors="pt",
    ).to(device)

    with torch.no_grad():
        outputs = model(**inputs)

    return outputs.last_hidden_state.cpu().numpy()


def encode_mlx(model, tokenizer, texts: list) -> np.ndarray:
    """Encode texts with MLX model."""
    import mlx.core as mx

    inputs = tokenizer(
        texts,
        padding=True,
        truncation=True,
        max_length=128,
        return_tensors="np",
    )

    input_ids = mx.array(inputs["input_ids"])
    attention_mask = mx.array(inputs["attention_mask"])

    outputs = model(input_ids, attention_mask)
    mx.eval(outputs)

    return np.array(outputs)


def compute_similarity_metrics(pt_emb: np.ndarray, mlx_emb: np.ndarray) -> Dict[str, float]:
    """Compute similarity metrics between PyTorch and MLX embeddings."""
    # Flatten for overall comparison
    pt_flat = pt_emb.flatten()
    mlx_flat = mlx_emb.flatten()

    # Correlation
    correlation = np.corrcoef(pt_flat, mlx_flat)[0, 1]

    # Mean absolute error
    mae = np.mean(np.abs(pt_flat - mlx_flat))

    # Max absolute error
    max_ae = np.max(np.abs(pt_flat - mlx_flat))

    # Relative error (normalized)
    rel_error = np.mean(np.abs(pt_flat - mlx_flat) / (np.abs(pt_flat) + 1e-8))

    # Cosine similarity per sample
    cosine_sims = []
    for i in range(pt_emb.shape[0]):
        pt_vec = pt_emb[i].mean(axis=0)  # Mean pool
        mlx_vec = mlx_emb[i].mean(axis=0)
        cos_sim = np.dot(pt_vec, mlx_vec) / (np.linalg.norm(pt_vec) * np.linalg.norm(mlx_vec) + 1e-8)
        cosine_sims.append(cos_sim)

    return {
        "correlation": correlation,
        "mae": mae,
        "max_ae": max_ae,
        "rel_error": rel_error,
        "mean_cosine_sim": np.mean(cosine_sims),
        "min_cosine_sim": np.min(cosine_sims),
    }


def validate_outputs(model_name: str = "google/xtr-base-en"):
    """Compare MLX and PyTorch outputs on test inputs."""
    print("=" * 60)
    print("MLX vs PyTorch Output Validation")
    print("=" * 60)
    print(f"\nModel: {model_name}")

    # Test inputs
    test_texts = [
        "def fibonacci(n): return n if n < 2 else fibonacci(n-1) + fibonacci(n-2)",
        "How to implement binary search in Python",
        "The quick brown fox jumps over the lazy dog",
        "impl Iterator for MyStruct { fn next(&mut self) -> Option<Self::Item> }",
        "Calculate the maximum sum of contiguous subarray",
        "fn main() { println!(\"Hello, world!\"); }",
        "SELECT * FROM users WHERE age > 21",
        "Machine learning model for image classification",
    ]

    print(f"\nTest inputs: {len(test_texts)} samples")

    # Load PyTorch model
    print("\nLoading PyTorch model...")
    pt_start = time.time()
    pt_model, pt_tokenizer, device = load_pytorch_model(model_name)
    print(f"  Loaded in {time.time() - pt_start:.2f}s (device: {device})")

    # Load MLX model
    print("\nLoading MLX model...")
    mlx_start = time.time()
    mlx_model, mlx_tokenizer = load_mlx_model(model_name)
    print(f"  Loaded in {time.time() - mlx_start:.2f}s")

    # Encode with PyTorch
    print("\nEncoding with PyTorch...")
    pt_start = time.time()
    pt_emb = encode_pytorch(pt_model, pt_tokenizer, test_texts, device)
    pt_time = time.time() - pt_start
    print(f"  Shape: {pt_emb.shape}, Time: {pt_time*1000:.1f}ms")

    # Encode with MLX
    print("\nEncoding with MLX...")
    mlx_start = time.time()
    mlx_emb = encode_mlx(mlx_model, mlx_tokenizer, test_texts)
    mlx_time = time.time() - mlx_start
    print(f"  Shape: {mlx_emb.shape}, Time: {mlx_time*1000:.1f}ms")

    # Compare outputs
    print("\n" + "=" * 60)
    print("Output Comparison")
    print("=" * 60)

    metrics = compute_similarity_metrics(pt_emb, mlx_emb)

    print(f"\n  Correlation:      {metrics['correlation']:.6f}")
    print(f"  Mean Abs Error:   {metrics['mae']:.6f}")
    print(f"  Max Abs Error:    {metrics['max_ae']:.6f}")
    print(f"  Relative Error:   {metrics['rel_error']:.6f}")
    print(f"  Mean Cosine Sim:  {metrics['mean_cosine_sim']:.6f}")
    print(f"  Min Cosine Sim:   {metrics['min_cosine_sim']:.6f}")

    # Pass/fail thresholds
    print("\n" + "=" * 60)
    print("Validation Results")
    print("=" * 60)

    passed = True
    checks = []

    # Correlation should be > 0.99
    corr_ok = metrics['correlation'] > 0.99
    checks.append(("Correlation > 0.99", corr_ok, metrics['correlation']))
    passed = passed and corr_ok

    # Mean cosine similarity should be > 0.99
    cos_ok = metrics['mean_cosine_sim'] > 0.99
    checks.append(("Mean cosine > 0.99", cos_ok, metrics['mean_cosine_sim']))
    passed = passed and cos_ok

    # Max absolute error should be < 0.1
    mae_ok = metrics['max_ae'] < 0.1
    checks.append(("Max abs error < 0.1", mae_ok, metrics['max_ae']))
    passed = passed and mae_ok

    for check_name, check_passed, value in checks:
        status = "PASS" if check_passed else "FAIL"
        print(f"  [{status}] {check_name}: {value:.6f}")

    print("\n" + "=" * 60)
    if passed:
        print("VALIDATION PASSED: MLX and PyTorch outputs are equivalent")
    else:
        print("VALIDATION FAILED: Outputs differ significantly")
    print("=" * 60)

    # Performance comparison
    print(f"\nPerformance: MLX {pt_time/mlx_time:.2f}x {'faster' if mlx_time < pt_time else 'slower'} than PyTorch")

    return passed, metrics


def main():
    parser = argparse.ArgumentParser(description="Validate MLX vs PyTorch outputs")
    parser.add_argument(
        "--model",
        type=str,
        default="google/xtr-base-en",
        help="Model name or path",
    )
    args = parser.parse_args()

    passed, metrics = validate_outputs(args.model)
    exit(0 if passed else 1)


if __name__ == "__main__":
    main()
