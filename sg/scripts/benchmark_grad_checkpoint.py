#!/usr/bin/env python3
"""
Benchmark gradient checkpointing tradeoff: memory vs speed.

Worker #2 investigation based on profiling results that show backward pass
is 63.5% of training time due to gradient checkpointing.

Usage:
    python scripts/benchmark_grad_checkpoint.py
"""

from __future__ import annotations

import gc
import time
from pathlib import Path

import torch
import yaml
from peft import LoraConfig, get_peft_model
from torch.utils.data import DataLoader
from transformers import AutoTokenizer, T5EncoderModel

import sys
sys.path.insert(0, str(Path(__file__).parent))
from train_xtr_improved import (
    ImprovedCodeSearchDataset,
    collate_fn,
    maxsim_scores,
    infonce_loss,
    select_device,
    TokenizedExample,
)
sys.modules['__main__'].TokenizedExample = TokenizedExample


def get_memory_mb(device: torch.device) -> float:
    """Get current memory usage in MB."""
    if device.type == "mps":
        # MPS doesn't have reliable memory query, estimate from allocated
        return torch.mps.current_allocated_memory() / (1024 * 1024)
    elif device.type == "cuda":
        return torch.cuda.memory_allocated() / (1024 * 1024)
    return 0.0


def sync_device(device: torch.device):
    """Synchronize device for accurate timing."""
    if device.type == "cuda":
        torch.cuda.synchronize()
    elif device.type == "mps":
        torch.mps.synchronize()


def benchmark_training(
    use_gradient_checkpointing: bool,
    batch_size: int,
    max_length: int,
    num_steps: int = 20,
) -> dict:
    """Run training with specified settings and return metrics."""

    device = select_device(None)
    print(f"\n{'='*60}")
    print(f"Benchmarking: gradient_checkpointing={use_gradient_checkpointing}")
    print(f"  batch_size={batch_size}, max_length={max_length}")
    print(f"  device={device}")
    print(f"{'='*60}")

    # Clear memory
    gc.collect()
    if device.type == "mps":
        torch.mps.empty_cache()
    elif device.type == "cuda":
        torch.cuda.empty_cache()

    # Load model
    base_model = "google/xtr-base-en"
    model = T5EncoderModel.from_pretrained(base_model)
    tokenizer = AutoTokenizer.from_pretrained(base_model)

    # Gradient checkpointing
    if use_gradient_checkpointing:
        model.gradient_checkpointing_enable()
        print("  Gradient checkpointing: ENABLED")
    else:
        print("  Gradient checkpointing: DISABLED")

    model.to(device)

    # LoRA
    lora_config = LoraConfig(
        r=16,
        lora_alpha=32,
        lora_dropout=0.05,
        target_modules=["q", "v", "k", "o", "wi_0", "wi_1", "wo"],
    )
    model = get_peft_model(model, lora_config)

    # Dataset
    data_path = Path("data/profiling_subset.jsonl")
    dataset = ImprovedCodeSearchDataset(
        data_path=data_path,
        tokenizer=tokenizer,
        max_length=max_length,
        max_hard_negatives=0,
        languages=None,
        use_cache=True,
    )

    dataloader = DataLoader(
        dataset,
        batch_size=batch_size,
        shuffle=True,
        collate_fn=collate_fn,
        num_workers=0,
    )

    # Optimizer
    optimizer = torch.optim.AdamW(model.parameters(), lr=2e-5)

    # AMP setup
    use_amp = True
    if device.type == "mps":
        amp_device_type = "mps"
        amp_dtype = torch.float16
    elif device.type == "cuda":
        amp_device_type = "cuda"
        amp_dtype = torch.float16
    else:
        use_amp = False
        amp_device_type = "cpu"
        amp_dtype = torch.bfloat16

    print(f"  AMP: {use_amp}")

    # Warmup
    print("  Warmup (3 steps)...")
    model.train()
    dataloader_iter = iter(dataloader)

    try:
        for _ in range(3):
            try:
                batch = next(dataloader_iter)
            except StopIteration:
                dataloader_iter = iter(dataloader)
                batch = next(dataloader_iter)

            query_ids = batch["query_ids"].to(device)
            query_mask = batch["query_mask"].to(device)
            pos_ids = batch["pos_ids"].to(device)
            pos_mask = batch["pos_mask"].to(device)

            with torch.amp.autocast(device_type=amp_device_type, enabled=use_amp, dtype=amp_dtype):
                query_out = model(input_ids=query_ids, attention_mask=query_mask)
                pos_out = model(input_ids=pos_ids, attention_mask=pos_mask)
                scores = maxsim_scores(query_out.last_hidden_state, query_mask.float(),
                                       pos_out.last_hidden_state, pos_mask.float())
                loss = infonce_loss(scores, 0.07)

            loss.backward()
            optimizer.step()
            optimizer.zero_grad()

        sync_device(device)
    except RuntimeError as e:
        if "out of memory" in str(e).lower():
            print(f"  OOM during warmup!")
            return {"status": "OOM", "use_gradient_checkpointing": use_gradient_checkpointing}
        raise

    # Measure peak memory after warmup
    memory_after_warmup = get_memory_mb(device)
    print(f"  Memory after warmup: {memory_after_warmup:.0f} MB")

    # Benchmark
    print(f"  Running {num_steps} steps...")
    step_times = []
    peak_memory = memory_after_warmup

    try:
        for step in range(num_steps):
            try:
                batch = next(dataloader_iter)
            except StopIteration:
                dataloader_iter = iter(dataloader)
                batch = next(dataloader_iter)

            sync_device(device)
            t0 = time.perf_counter()

            query_ids = batch["query_ids"].to(device)
            query_mask = batch["query_mask"].to(device)
            pos_ids = batch["pos_ids"].to(device)
            pos_mask = batch["pos_mask"].to(device)

            with torch.amp.autocast(device_type=amp_device_type, enabled=use_amp, dtype=amp_dtype):
                query_out = model(input_ids=query_ids, attention_mask=query_mask)
                pos_out = model(input_ids=pos_ids, attention_mask=pos_mask)
                scores = maxsim_scores(query_out.last_hidden_state, query_mask.float(),
                                       pos_out.last_hidden_state, pos_mask.float())
                loss = infonce_loss(scores, 0.07)

            loss.backward()
            optimizer.step()
            optimizer.zero_grad()

            sync_device(device)
            step_times.append(time.perf_counter() - t0)

            # Track peak memory
            current_mem = get_memory_mb(device)
            peak_memory = max(peak_memory, current_mem)

            if (step + 1) % 5 == 0:
                print(f"    Step {step + 1}/{num_steps}: {step_times[-1]*1000:.0f}ms, mem={current_mem:.0f}MB")

    except RuntimeError as e:
        if "out of memory" in str(e).lower():
            print(f"  OOM at step {step}!")
            return {
                "status": "OOM",
                "use_gradient_checkpointing": use_gradient_checkpointing,
                "oom_step": step,
            }
        raise

    # Calculate stats (skip first 3 steps as warmup)
    warmup_skip = min(3, len(step_times) - 1)
    measured_times = step_times[warmup_skip:]

    results = {
        "status": "OK",
        "use_gradient_checkpointing": use_gradient_checkpointing,
        "batch_size": batch_size,
        "max_length": max_length,
        "num_steps": num_steps,
        "mean_step_ms": sum(measured_times) / len(measured_times) * 1000,
        "steps_per_second": len(measured_times) / sum(measured_times),
        "samples_per_second": (len(measured_times) * batch_size) / sum(measured_times),
        "peak_memory_mb": peak_memory,
    }

    print(f"\n  Results:")
    print(f"    Mean step time: {results['mean_step_ms']:.0f} ms")
    print(f"    Steps/second: {results['steps_per_second']:.2f}")
    print(f"    Samples/second: {results['samples_per_second']:.1f}")
    print(f"    Peak memory: {results['peak_memory_mb']:.0f} MB")

    # Cleanup
    del model, optimizer, dataset, dataloader
    gc.collect()
    if device.type == "mps":
        torch.mps.empty_cache()

    return results


def main():
    print("=" * 70)
    print("Gradient Checkpointing Benchmark")
    print("=" * 70)
    print("\nThis benchmark compares training speed and memory with and without")
    print("gradient checkpointing to help determine the best tradeoff.\n")

    # Test configurations
    configs = [
        # (batch_size, max_length)
        (8, 512),
        (12, 512),  # Default from profiling
        (16, 512),  # Higher batch if memory allows
    ]

    all_results = []

    for batch_size, max_length in configs:
        # Test WITH gradient checkpointing (should use less memory but be slower)
        result_with = benchmark_training(
            use_gradient_checkpointing=True,
            batch_size=batch_size,
            max_length=max_length,
            num_steps=20,
        )
        all_results.append(result_with)

        # Test WITHOUT gradient checkpointing (should use more memory but be faster)
        result_without = benchmark_training(
            use_gradient_checkpointing=False,
            batch_size=batch_size,
            max_length=max_length,
            num_steps=20,
        )
        all_results.append(result_without)

    # Summary
    print("\n" + "=" * 70)
    print("SUMMARY")
    print("=" * 70)
    print(f"\n{'Config':<25} {'Grad Ckpt':<12} {'Step (ms)':<12} {'Samples/s':<12} {'Memory (MB)':<12}")
    print("-" * 70)

    for r in all_results:
        if r["status"] == "OOM":
            print(f"B={r.get('batch_size', '?')} L={r.get('max_length', '?'):<10} "
                  f"{'ON' if r['use_gradient_checkpointing'] else 'OFF':<12} "
                  f"{'OOM':<12} {'--':<12} {'--':<12}")
        else:
            config = f"B={r['batch_size']} L={r['max_length']}"
            ckpt = "ON" if r['use_gradient_checkpointing'] else "OFF"
            print(f"{config:<25} {ckpt:<12} {r['mean_step_ms']:<12.0f} "
                  f"{r['samples_per_second']:<12.1f} {r['peak_memory_mb']:<12.0f}")

    # Calculate speedup
    print("\n" + "-" * 70)
    print("Speedup Analysis:")

    for i in range(0, len(all_results), 2):
        with_ckpt = all_results[i]
        without_ckpt = all_results[i+1]

        if with_ckpt["status"] == "OK" and without_ckpt["status"] == "OK":
            speedup = without_ckpt["samples_per_second"] / with_ckpt["samples_per_second"]
            mem_increase = without_ckpt["peak_memory_mb"] / with_ckpt["peak_memory_mb"]
            print(f"  B={with_ckpt['batch_size']}: Disabling grad ckpt gives "
                  f"{speedup:.2f}x speedup, {mem_increase:.2f}x memory")
        elif without_ckpt["status"] == "OOM":
            print(f"  B={with_ckpt.get('batch_size', '?')}: Cannot disable grad ckpt (OOM)")


if __name__ == "__main__":
    main()
