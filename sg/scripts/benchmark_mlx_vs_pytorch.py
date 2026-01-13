#!/usr/bin/env python3
"""Benchmark MLX vs PyTorch MPS for training-relevant operations.

This benchmarks operations that matter for XTR training:
- Matrix multiplication (attention, projections)
- Einsum (MaxSim computation)
- Softmax
- Layer normalization
"""

import time
import mlx.core as mx
import numpy as np

# Try importing torch
try:
    import torch
    TORCH_AVAILABLE = True
    MPS_AVAILABLE = torch.backends.mps.is_available()
except ImportError:
    TORCH_AVAILABLE = False
    MPS_AVAILABLE = False

def benchmark_mlx(name: str, fn, warmup: int = 5, runs: int = 20):
    """Benchmark an MLX operation."""
    # Warmup
    for _ in range(warmup):
        result = fn()
        mx.eval(result)

    # Timed runs
    times = []
    for _ in range(runs):
        start = time.perf_counter()
        result = fn()
        mx.eval(result)  # Force sync
        times.append(time.perf_counter() - start)

    return np.mean(times) * 1000, np.std(times) * 1000  # ms

def benchmark_pytorch(name: str, fn, warmup: int = 5, runs: int = 20):
    """Benchmark a PyTorch operation."""
    if not MPS_AVAILABLE:
        return None, None

    # Warmup
    for _ in range(warmup):
        result = fn()
        if hasattr(result, 'cpu'):
            torch.mps.synchronize()

    # Timed runs
    times = []
    for _ in range(runs):
        start = time.perf_counter()
        result = fn()
        torch.mps.synchronize()  # Force sync
        times.append(time.perf_counter() - start)

    return np.mean(times) * 1000, np.std(times) * 1000  # ms

def main():
    print("=" * 70)
    print("MLX vs PyTorch MPS Benchmark")
    print("=" * 70)
    print(f"\nMLX version: {mx.__version__}")
    if TORCH_AVAILABLE:
        print(f"PyTorch version: {torch.__version__}")
        print(f"MPS available: {MPS_AVAILABLE}")
    else:
        print("PyTorch not available")
    print()

    # Typical XTR training dimensions
    batch_size = 32
    seq_len = 128
    hidden_dim = 768
    num_heads = 12
    head_dim = hidden_dim // num_heads

    # Test 1: Matrix multiplication (attention projections)
    print("-" * 70)
    print(f"Test 1: MatMul [{batch_size}x{seq_len}x{hidden_dim}] @ [{hidden_dim}x{hidden_dim}]")
    print("-" * 70)

    # MLX
    mlx_a = mx.random.normal((batch_size, seq_len, hidden_dim))
    mlx_b = mx.random.normal((hidden_dim, hidden_dim))
    mx.eval(mlx_a, mlx_b)

    mlx_mean, mlx_std = benchmark_mlx("matmul", lambda: mlx_a @ mlx_b)
    print(f"MLX:     {mlx_mean:8.3f} ms ± {mlx_std:.3f}")

    if MPS_AVAILABLE:
        torch_a = torch.randn(batch_size, seq_len, hidden_dim, device='mps')
        torch_b = torch.randn(hidden_dim, hidden_dim, device='mps')
        torch.mps.synchronize()

        torch_mean, torch_std = benchmark_pytorch("matmul", lambda: torch_a @ torch_b)
        print(f"PyTorch: {torch_mean:8.3f} ms ± {torch_std:.3f}")
        print(f"Speedup: {torch_mean/mlx_mean:.2f}x")

    # Test 2: Einsum for MaxSim (the key operation)
    print("\n" + "-" * 70)
    print(f"Test 2: MaxSim einsum [{batch_size}x{seq_len}x{hidden_dim}] . [{batch_size}x{seq_len}x{hidden_dim}]")
    print("-" * 70)

    # MLX
    mlx_q = mx.random.normal((batch_size, seq_len, hidden_dim))
    mlx_d = mx.random.normal((batch_size, seq_len, hidden_dim))
    mx.eval(mlx_q, mlx_d)

    def mlx_maxsim():
        sims = mx.einsum('iqd,jkd->iqjk', mlx_q, mlx_d)
        return mx.max(sims, axis=-1)

    mlx_mean, mlx_std = benchmark_mlx("maxsim", mlx_maxsim)
    print(f"MLX:     {mlx_mean:8.3f} ms ± {mlx_std:.3f}")

    if MPS_AVAILABLE:
        torch_q = torch.randn(batch_size, seq_len, hidden_dim, device='mps')
        torch_d = torch.randn(batch_size, seq_len, hidden_dim, device='mps')
        torch.mps.synchronize()

        def torch_maxsim():
            sims = torch.einsum('iqd,jkd->iqjk', torch_q, torch_d)
            return torch.max(sims, dim=-1).values

        torch_mean, torch_std = benchmark_pytorch("maxsim", torch_maxsim)
        print(f"PyTorch: {torch_mean:8.3f} ms ± {torch_std:.3f}")
        print(f"Speedup: {torch_mean/mlx_mean:.2f}x")

    # Test 3: Softmax (used extensively in transformers)
    print("\n" + "-" * 70)
    print(f"Test 3: Softmax [{batch_size}x{num_heads}x{seq_len}x{seq_len}]")
    print("-" * 70)

    # MLX
    mlx_attn = mx.random.normal((batch_size, num_heads, seq_len, seq_len))
    mx.eval(mlx_attn)

    mlx_mean, mlx_std = benchmark_mlx("softmax", lambda: mx.softmax(mlx_attn, axis=-1))
    print(f"MLX:     {mlx_mean:8.3f} ms ± {mlx_std:.3f}")

    if MPS_AVAILABLE:
        torch_attn = torch.randn(batch_size, num_heads, seq_len, seq_len, device='mps')
        torch.mps.synchronize()

        torch_mean, torch_std = benchmark_pytorch("softmax", lambda: torch.softmax(torch_attn, dim=-1))
        print(f"PyTorch: {torch_mean:8.3f} ms ± {torch_std:.3f}")
        print(f"Speedup: {torch_mean/mlx_mean:.2f}x")

    # Test 4: Layer normalization
    print("\n" + "-" * 70)
    print(f"Test 4: LayerNorm [{batch_size}x{seq_len}x{hidden_dim}]")
    print("-" * 70)

    # MLX
    mlx_x = mx.random.normal((batch_size, seq_len, hidden_dim))
    mlx_gamma = mx.ones((hidden_dim,))
    mlx_beta = mx.zeros((hidden_dim,))
    mx.eval(mlx_x, mlx_gamma, mlx_beta)

    def mlx_layernorm():
        mean = mx.mean(mlx_x, axis=-1, keepdims=True)
        var = mx.var(mlx_x, axis=-1, keepdims=True)
        return mlx_gamma * (mlx_x - mean) / mx.sqrt(var + 1e-5) + mlx_beta

    mlx_mean, mlx_std = benchmark_mlx("layernorm", mlx_layernorm)
    print(f"MLX:     {mlx_mean:8.3f} ms ± {mlx_std:.3f}")

    if MPS_AVAILABLE:
        torch_x = torch.randn(batch_size, seq_len, hidden_dim, device='mps')
        torch_ln = torch.nn.LayerNorm(hidden_dim).to('mps')
        torch.mps.synchronize()

        torch_mean, torch_std = benchmark_pytorch("layernorm", lambda: torch_ln(torch_x))
        print(f"PyTorch: {torch_mean:8.3f} ms ± {torch_std:.3f}")
        print(f"Speedup: {torch_mean/mlx_mean:.2f}x")

    # Test 5: Backward pass simulation (gradient computation)
    print("\n" + "-" * 70)
    print(f"Test 5: Backward pass (matmul gradient) [{batch_size}x{seq_len}x{hidden_dim}]")
    print("-" * 70)

    # MLX with value_and_grad
    mlx_a = mx.random.normal((batch_size, seq_len, hidden_dim))
    mlx_b = mx.random.normal((hidden_dim, hidden_dim))
    mx.eval(mlx_a, mlx_b)

    def mlx_forward(b):
        return mx.sum((mlx_a @ b) ** 2)

    mlx_grad_fn = mx.grad(mlx_forward)

    def mlx_backward():
        return mlx_grad_fn(mlx_b)

    mlx_mean, mlx_std = benchmark_mlx("backward", mlx_backward)
    print(f"MLX:     {mlx_mean:8.3f} ms ± {mlx_std:.3f}")

    if MPS_AVAILABLE:
        torch_a = torch.randn(batch_size, seq_len, hidden_dim, device='mps')
        torch_b = torch.randn(hidden_dim, hidden_dim, device='mps', requires_grad=True)
        torch.mps.synchronize()

        def torch_backward():
            if torch_b.grad is not None:
                torch_b.grad.zero_()
            out = torch.sum((torch_a @ torch_b) ** 2)
            out.backward()
            return torch_b.grad

        torch_mean, torch_std = benchmark_pytorch("backward", torch_backward)
        print(f"PyTorch: {torch_mean:8.3f} ms ± {torch_std:.3f}")
        print(f"Speedup: {torch_mean/mlx_mean:.2f}x")

    print("\n" + "=" * 70)
    print("Summary")
    print("=" * 70)
    print("\nNote: Speedup > 1.0 means MLX is faster than PyTorch MPS")
    print("      Speedup < 1.0 means PyTorch MPS is faster than MLX")

if __name__ == "__main__":
    main()
