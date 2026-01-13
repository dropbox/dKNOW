#!/usr/bin/env python3
"""
Profile XTR training to identify bottlenecks for Metal acceleration.

Measures time spent in:
- Data loading/transfer
- Query forward pass
- Document forward pass
- MaxSim computation
- Loss computation
- Backward pass

Usage:
    python scripts/profile_training.py --config config/train_improved.yaml --steps 100
"""

from __future__ import annotations

import argparse
import json
import time
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, List

import numpy as np
import torch
import torch.nn.functional as F
import yaml
from peft import LoraConfig, get_peft_model
from torch.utils.data import DataLoader
from transformers import AutoTokenizer, T5EncoderModel

# Import from training script
import sys
sys.path.insert(0, str(Path(__file__).parent))
from train_xtr_improved import (
    ImprovedCodeSearchDataset,
    LanguageAwareBatchSampler,
    TokenizedExample,
    collate_fn,
    maxsim_scores,
    multiple_negatives_ranking_loss,
    infonce_loss,
    combined_loss,
    select_device,
)
# Make TokenizedExample available for unpickling
sys.modules['__main__'].TokenizedExample = TokenizedExample


@dataclass
class TimingStats:
    """Aggregated timing statistics."""
    data_transfer: List[float]
    forward_query: List[float]
    forward_doc: List[float]
    maxsim: List[float]
    loss: List[float]
    backward: List[float]
    optimizer: List[float]
    total: List[float]

    def __init__(self):
        self.data_transfer = []
        self.forward_query = []
        self.forward_doc = []
        self.maxsim = []
        self.loss = []
        self.backward = []
        self.optimizer = []
        self.total = []

    def add(self, name: str, value: float):
        getattr(self, name).append(value)

    def summary(self) -> Dict[str, Dict[str, float]]:
        """Return summary statistics for all components."""
        result = {}
        for name in ["data_transfer", "forward_query", "forward_doc", "maxsim",
                     "loss", "backward", "optimizer", "total"]:
            values = getattr(self, name)
            if values:
                # Skip first few steps (warmup)
                values = values[3:] if len(values) > 3 else values
                result[name] = {
                    "mean_ms": np.mean(values) * 1000,
                    "std_ms": np.std(values) * 1000,
                    "min_ms": np.min(values) * 1000,
                    "max_ms": np.max(values) * 1000,
                    "total_s": np.sum(values),
                }
        return result


def sync_device(device: torch.device):
    """Synchronize device for accurate timing."""
    if device.type == "cuda":
        torch.cuda.synchronize()
    elif device.type == "mps":
        torch.mps.synchronize()


def profile_training(config: Dict, num_steps: int = 100) -> TimingStats:
    """Run training for num_steps and collect detailed timing."""

    training_cfg = config["training"]
    data_cfg = config["data"]
    model_cfg = config["model"]

    # Device
    device = select_device(training_cfg.get("device"))
    print(f"Device: {device}")

    # Load model
    base_model = model_cfg["base"]
    print(f"Loading model: {base_model}")
    model = T5EncoderModel.from_pretrained(base_model)
    tokenizer = AutoTokenizer.from_pretrained(base_model)

    # Gradient checkpointing (keep same as training)
    if training_cfg.get("gradient_checkpointing", True):
        model.gradient_checkpointing_enable()

    model.to(device)

    # LoRA
    lora_config = LoraConfig(
        r=training_cfg["lora_r"],
        lora_alpha=training_cfg["lora_alpha"],
        lora_dropout=training_cfg["lora_dropout"],
        target_modules=training_cfg["target_modules"],
    )
    model = get_peft_model(model, lora_config)
    model.print_trainable_parameters()

    # Load dataset
    train_path = Path(data_cfg["train"])
    languages = set(data_cfg.get("languages", [])) or None
    max_length = training_cfg.get("max_length", 512)
    max_hard_negs = training_cfg.get("max_hard_negatives", 3)

    # Check for max_examples limit (to prevent OOM)
    max_examples = config["data"].get("max_examples", None)

    dataset = ImprovedCodeSearchDataset(
        data_path=train_path,
        tokenizer=tokenizer,
        max_length=max_length,
        max_hard_negatives=max_hard_negs,
        languages=languages,
        use_cache=True,
    )

    # Limit dataset size if specified (prevents OOM on large datasets)
    if max_examples and max_examples > 0 and len(dataset) > max_examples:
        print(f"  Limiting to {max_examples} of {len(dataset)} examples")
        dataset.examples = dataset.examples[:max_examples]
        # Rebuild language indices for limited examples
        dataset.language_indices.clear()
        for i, ex in enumerate(dataset.examples):
            dataset.language_indices[ex.language].append(i)

    # Training params
    batch_size = training_cfg["batch_size"]
    temperature = training_cfg.get("temperature", 0.07)
    use_margin_loss = training_cfg.get("use_margin_loss", True)
    margin = training_cfg.get("margin", 0.2)
    margin_weight = training_cfg.get("margin_weight", 0.1)
    use_mnr_loss = training_cfg.get("use_mnr_loss", False)
    mnr_scale = training_cfg.get("mnr_scale", 20.0)

    # Create dataloader
    use_language_aware = training_cfg.get("language_aware_batching", True)
    if use_language_aware:
        batch_sampler = LanguageAwareBatchSampler(dataset, batch_size, drop_last=True, shuffle=True)
        dataloader = DataLoader(
            dataset,
            batch_sampler=batch_sampler,
            collate_fn=collate_fn,
            num_workers=0,  # No multiprocessing for clean timing
            pin_memory=False,
        )
    else:
        dataloader = DataLoader(
            dataset,
            batch_size=batch_size,
            shuffle=True,
            collate_fn=collate_fn,
            num_workers=0,
            pin_memory=False,
        )

    # Optimizer
    optimizer = torch.optim.AdamW(model.parameters(), lr=float(training_cfg["learning_rate"]))

    # AMP setup
    use_amp = training_cfg.get("use_amp", True)
    if device.type == "mps":
        amp_device_type = "mps"
        amp_dtype = torch.float16
    elif device.type == "cuda":
        amp_device_type = "cuda"
        amp_dtype = torch.float16
    else:
        amp_device_type = "cpu"
        amp_dtype = torch.bfloat16
        use_amp = False

    print(f"\nProfiling configuration:")
    print(f"  Batch size: {batch_size}")
    print(f"  Max length: {max_length}")
    print(f"  AMP: {use_amp} (device={amp_device_type})")
    print(f"  Profiling {num_steps} steps...")

    # Timing collection
    stats = TimingStats()
    model.train()

    # Warmup
    print("\nWarmup (3 steps)...")
    dataloader_iter = iter(dataloader)
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
            loss = infonce_loss(scores, temperature)

        loss.backward()
        optimizer.step()
        optimizer.zero_grad()

    sync_device(device)

    # Profile
    print(f"\nProfiling {num_steps} steps...")
    for step in range(num_steps):
        step_start = time.perf_counter()

        try:
            batch = next(dataloader_iter)
        except StopIteration:
            dataloader_iter = iter(dataloader)
            batch = next(dataloader_iter)

        # === DATA TRANSFER ===
        sync_device(device)
        t0 = time.perf_counter()

        query_ids = batch["query_ids"].to(device)
        query_mask = batch["query_mask"].to(device)
        pos_ids = batch["pos_ids"].to(device)
        pos_mask = batch["pos_mask"].to(device)

        sync_device(device)
        stats.add("data_transfer", time.perf_counter() - t0)

        # === FORWARD QUERY ===
        t0 = time.perf_counter()

        with torch.amp.autocast(device_type=amp_device_type, enabled=use_amp, dtype=amp_dtype):
            query_out = model(input_ids=query_ids, attention_mask=query_mask)
            query_emb = query_out.last_hidden_state

        sync_device(device)
        stats.add("forward_query", time.perf_counter() - t0)

        # === FORWARD DOC ===
        t0 = time.perf_counter()

        with torch.amp.autocast(device_type=amp_device_type, enabled=use_amp, dtype=amp_dtype):
            pos_out = model(input_ids=pos_ids, attention_mask=pos_mask)
            pos_emb = pos_out.last_hidden_state

        sync_device(device)
        stats.add("forward_doc", time.perf_counter() - t0)

        # === MAXSIM ===
        t0 = time.perf_counter()

        scores = maxsim_scores(query_emb, query_mask.float(), pos_emb, pos_mask.float())

        sync_device(device)
        stats.add("maxsim", time.perf_counter() - t0)

        # === LOSS ===
        t0 = time.perf_counter()

        if use_mnr_loss:
            loss = multiple_negatives_ranking_loss(scores, temperature, mnr_scale)
        elif use_margin_loss:
            loss, _ = combined_loss(scores, temperature, margin, margin_weight)
        else:
            loss = infonce_loss(scores, temperature)

        sync_device(device)
        stats.add("loss", time.perf_counter() - t0)

        # === BACKWARD ===
        t0 = time.perf_counter()

        loss.backward()

        sync_device(device)
        stats.add("backward", time.perf_counter() - t0)

        # === OPTIMIZER ===
        t0 = time.perf_counter()

        optimizer.step()
        optimizer.zero_grad()

        sync_device(device)
        stats.add("optimizer", time.perf_counter() - t0)

        # Total step time
        stats.add("total", time.perf_counter() - step_start)

        if (step + 1) % 10 == 0:
            print(f"  Step {step + 1}/{num_steps}", flush=True)

    return stats


def print_report(stats: TimingStats, output_path: Path = None):
    """Print and optionally save profiling report."""

    summary = stats.summary()

    # Calculate percentages
    total_time = sum(s["total_s"] for name, s in summary.items() if name != "total")

    report_lines = []
    report_lines.append("=" * 60)
    report_lines.append("XTR TRAINING PROFILING RESULTS")
    report_lines.append("=" * 60)
    report_lines.append("")
    report_lines.append(f"Total steps profiled: {len(stats.total)}")
    report_lines.append(f"Total time: {summary['total']['total_s']:.2f}s")
    report_lines.append(f"Steps/second: {len(stats.total) / summary['total']['total_s']:.2f}")
    report_lines.append("")
    report_lines.append("-" * 60)
    report_lines.append(f"{'Component':<20} {'Mean (ms)':<12} {'Std (ms)':<12} {'% Total':<10}")
    report_lines.append("-" * 60)

    components = [
        ("data_transfer", "Data Transfer"),
        ("forward_query", "Forward (Query)"),
        ("forward_doc", "Forward (Doc)"),
        ("maxsim", "MaxSim"),
        ("loss", "Loss"),
        ("backward", "Backward"),
        ("optimizer", "Optimizer"),
    ]

    for key, name in components:
        s = summary[key]
        pct = (s["total_s"] / total_time) * 100
        report_lines.append(f"{name:<20} {s['mean_ms']:>10.2f}  {s['std_ms']:>10.2f}  {pct:>8.1f}%")

    report_lines.append("-" * 60)
    report_lines.append(f"{'TOTAL':<20} {summary['total']['mean_ms']:>10.2f}  {summary['total']['std_ms']:>10.2f}  {'100.0%':>8}")
    report_lines.append("=" * 60)

    # Key findings
    report_lines.append("")
    report_lines.append("KEY FINDINGS:")
    report_lines.append("")

    # Sort by percentage
    sorted_components = sorted(
        [(key, name, summary[key]["total_s"] / total_time * 100)
         for key, name in components],
        key=lambda x: x[2],
        reverse=True
    )

    for i, (key, name, pct) in enumerate(sorted_components[:3], 1):
        report_lines.append(f"  {i}. {name}: {pct:.1f}% of training time")

    report_lines.append("")
    report_lines.append("RECOMMENDATIONS:")
    report_lines.append("")

    # Check if MaxSim is significant
    maxsim_pct = summary["maxsim"]["total_s"] / total_time * 100
    if maxsim_pct > 10:
        report_lines.append(f"  - MaxSim is {maxsim_pct:.1f}% of time → HIGH PRIORITY for Metal acceleration")
    else:
        report_lines.append(f"  - MaxSim is only {maxsim_pct:.1f}% of time → Lower priority for Metal")

    # Check forward pass
    forward_pct = (summary["forward_query"]["total_s"] + summary["forward_doc"]["total_s"]) / total_time * 100
    if forward_pct > 50:
        report_lines.append(f"  - Forward pass is {forward_pct:.1f}% of time → Model inference is bottleneck")

    # Check backward
    backward_pct = summary["backward"]["total_s"] / total_time * 100
    if backward_pct > 30:
        report_lines.append(f"  - Backward pass is {backward_pct:.1f}% of time → Gradient computation is bottleneck")

    report_lines.append("")

    report = "\n".join(report_lines)
    print(report)

    if output_path:
        output_path.parent.mkdir(parents=True, exist_ok=True)
        with output_path.open("w") as f:
            f.write(report)
        print(f"\nReport saved to: {output_path}")

    return summary


def main():
    parser = argparse.ArgumentParser(description="Profile XTR training")
    parser.add_argument("--config", type=Path, required=True, help="Training config YAML")
    parser.add_argument("--steps", type=int, default=100, help="Number of steps to profile")
    parser.add_argument("--max-examples", type=int, default=1000,
                        help="Max training examples to load (prevents OOM). Use -1 for all.")
    parser.add_argument("--output", type=Path, default=Path("docs/PROFILING_RESULTS.md"),
                        help="Output file for report")
    args = parser.parse_args()

    # Load config
    with args.config.open("r") as f:
        config = yaml.safe_load(f)

    # Limit dataset size to prevent OOM (default: 1000 examples)
    if args.max_examples > 0:
        config["data"]["max_examples"] = args.max_examples
        print(f"Limiting dataset to {args.max_examples} examples to prevent OOM")

    # Run profiling
    stats = profile_training(config, args.steps)

    # Print report
    print_report(stats, args.output)


if __name__ == "__main__":
    main()
