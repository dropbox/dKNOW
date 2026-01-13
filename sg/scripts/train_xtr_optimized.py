#!/usr/bin/env python3
"""
Optimized XTR fine-tuning for code search using LoRA adapters.

Key optimizations:
- Gradient checkpointing for memory efficiency
- Mixed precision training (fp16/bf16) where supported
- Efficient DataLoader with prefetching and multiple workers
- Tokenization caching to disk (avoids re-tokenizing each epoch)
- Gradient accumulation for effective larger batch sizes
- torch.compile() for speedup (when available)
- In-batch negatives for efficient contrastive learning

Usage:
    python scripts/train_xtr_optimized.py --config config/train_multilang_full.yaml
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import pickle
import random
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, Iterator, List, Optional, Tuple

import torch
import torch.nn.functional as F
import yaml
from peft import LoraConfig, get_peft_model
from torch.utils.data import DataLoader, Dataset, IterableDataset
from transformers import AutoTokenizer, T5EncoderModel, get_cosine_schedule_with_warmup


# ============================================================================
# Data Structures
# ============================================================================

@dataclass
class TrainingExample:
    query: str
    positive: str
    language: Optional[str] = None


@dataclass
class TokenizedExample:
    query_ids: List[int]
    query_mask: List[int]
    pos_ids: List[int]
    pos_mask: List[int]


# ============================================================================
# Configuration
# ============================================================================

def load_config(path: Path) -> Dict:
    with path.open("r", encoding="utf-8") as handle:
        return yaml.safe_load(handle)


def get_cache_path(data_path: Path, max_length: int, languages: Optional[set]) -> Path:
    """Generate a unique cache path based on data file and tokenization params."""
    # Create hash of configuration
    config_str = f"{data_path.name}_{max_length}_{sorted(languages) if languages else 'all'}"
    config_hash = hashlib.md5(config_str.encode()).hexdigest()[:12]
    cache_dir = data_path.parent / ".cache"
    cache_dir.mkdir(exist_ok=True)
    return cache_dir / f"tokenized_{config_hash}.pkl"


# ============================================================================
# Dataset Classes
# ============================================================================

class CodeSearchDataset(Dataset):
    """
    Memory-mapped dataset with tokenization caching.

    On first load, tokenizes all examples and saves to disk.
    On subsequent loads, reads from cache for fast startup.
    """

    def __init__(
        self,
        data_path: Path,
        tokenizer: AutoTokenizer,
        max_length: int,
        languages: Optional[set[str]] = None,
        use_cache: bool = True,
    ):
        self.max_length = max_length
        self.tokenizer = tokenizer
        self.examples: List[TokenizedExample] = []

        cache_path = get_cache_path(data_path, max_length, languages)

        if use_cache and cache_path.exists():
            print(f"Loading tokenized data from cache: {cache_path}")
            start = time.time()
            with cache_path.open("rb") as f:
                self.examples = pickle.load(f)
            print(f"  Loaded {len(self.examples)} examples in {time.time() - start:.1f}s")
        else:
            print(f"Tokenizing dataset: {data_path}")
            self._load_and_tokenize(data_path, languages)

            if use_cache:
                print(f"Saving tokenized data to cache: {cache_path}")
                with cache_path.open("wb") as f:
                    pickle.dump(self.examples, f)

    def _load_and_tokenize(self, data_path: Path, languages: Optional[set[str]]) -> None:
        """Load JSONL file and tokenize all examples."""
        raw_examples = []

        # First pass: load raw examples
        with data_path.open("r", encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    payload = json.loads(line)
                except json.JSONDecodeError:
                    continue

                if languages and payload.get("language") not in languages:
                    continue

                query = payload.get("query")
                positive = payload.get("positive")
                if query and positive:
                    raw_examples.append(TrainingExample(
                        query=query,
                        positive=positive,
                        language=payload.get("language"),
                    ))

        print(f"  Found {len(raw_examples)} examples")

        # Batch tokenize for efficiency
        batch_size = 1000
        start = time.time()

        for i in range(0, len(raw_examples), batch_size):
            batch = raw_examples[i:i + batch_size]
            queries = [ex.query for ex in batch]
            positives = [ex.positive for ex in batch]

            query_enc = self.tokenizer(
                queries,
                padding=False,
                truncation=True,
                max_length=self.max_length,
            )
            pos_enc = self.tokenizer(
                positives,
                padding=False,
                truncation=True,
                max_length=self.max_length,
            )

            for j in range(len(batch)):
                self.examples.append(TokenizedExample(
                    query_ids=query_enc["input_ids"][j],
                    query_mask=query_enc["attention_mask"][j],
                    pos_ids=pos_enc["input_ids"][j],
                    pos_mask=pos_enc["attention_mask"][j],
                ))

            if (i + batch_size) % 10000 == 0 or i + batch_size >= len(raw_examples):
                elapsed = time.time() - start
                rate = (i + batch_size) / elapsed
                print(f"  Tokenized {min(i + batch_size, len(raw_examples))}/{len(raw_examples)} ({rate:.0f}/s)")

    def __len__(self) -> int:
        return len(self.examples)

    def __getitem__(self, idx: int) -> TokenizedExample:
        return self.examples[idx]


def collate_fn(batch: List[TokenizedExample]) -> Dict[str, torch.Tensor]:
    """Collate tokenized examples into padded batches."""
    # Find max lengths in this batch
    max_query_len = max(len(ex.query_ids) for ex in batch)
    max_pos_len = max(len(ex.pos_ids) for ex in batch)

    # Pad sequences
    query_ids = []
    query_mask = []
    pos_ids = []
    pos_mask = []

    for ex in batch:
        # Pad query
        pad_len = max_query_len - len(ex.query_ids)
        query_ids.append(ex.query_ids + [0] * pad_len)
        query_mask.append(ex.query_mask + [0] * pad_len)

        # Pad positive
        pad_len = max_pos_len - len(ex.pos_ids)
        pos_ids.append(ex.pos_ids + [0] * pad_len)
        pos_mask.append(ex.pos_mask + [0] * pad_len)

    return {
        "query_ids": torch.tensor(query_ids, dtype=torch.long),
        "query_mask": torch.tensor(query_mask, dtype=torch.long),
        "pos_ids": torch.tensor(pos_ids, dtype=torch.long),
        "pos_mask": torch.tensor(pos_mask, dtype=torch.long),
    }


# ============================================================================
# Model and Loss Functions
# ============================================================================

def select_device(preferred: Optional[str]) -> torch.device:
    """Select the best available device."""
    if preferred:
        return torch.device(preferred)
    if torch.cuda.is_available():
        return torch.device("cuda")
    if hasattr(torch.backends, "mps") and torch.backends.mps.is_available():
        return torch.device("mps")
    return torch.device("cpu")


def get_dtype_for_device(device: torch.device) -> torch.dtype:
    """Get the appropriate dtype for mixed precision on the given device."""
    if device.type == "cuda":
        # Use bfloat16 on newer GPUs, float16 otherwise
        if torch.cuda.is_bf16_supported():
            return torch.bfloat16
        return torch.float16
    elif device.type == "mps":
        # MPS supports float16
        return torch.float16
    return torch.float32


def maxsim_scores(
    query_emb: torch.Tensor,
    query_mask: torch.Tensor,
    doc_emb: torch.Tensor,
    doc_mask: torch.Tensor,
) -> torch.Tensor:
    """
    Compute MaxSim scores between queries and documents.

    Uses L2 normalization and mean pooling over query tokens for stable training.
    """
    # L2 normalize embeddings
    query_emb = F.normalize(query_emb, p=2, dim=-1)
    doc_emb = F.normalize(doc_emb, p=2, dim=-1)

    batch_size = query_emb.size(0)

    # Compute all-pairs similarity more efficiently
    # query_emb: [B, Q, D], doc_emb: [B, K, D]
    # We want: for each query i, compute similarity to all docs j

    scores = []
    doc_mask_expanded = doc_mask.unsqueeze(1)  # [B, 1, K]

    for i in range(batch_size):
        # query_vecs: [Q, D]
        query_vecs = query_emb[i]

        # Compute similarity with all documents: [B, Q, K]
        sim = torch.einsum("qd,bkd->bqk", query_vecs, doc_emb)

        # Mask out padding tokens in documents
        # Use -1e4 instead of -1e9 to avoid overflow in float16
        sim = sim.masked_fill(doc_mask_expanded == 0, -1e4)

        # MaxSim: max over document tokens, then mean over query tokens
        max_sim = sim.max(dim=2).values  # [B, Q]

        # Mean over query tokens (only non-padding)
        query_len = query_mask[i].sum()
        max_sim = max_sim * query_mask[i].unsqueeze(0)
        scores.append(max_sim.sum(dim=1) / query_len)

    return torch.stack(scores, dim=0)  # [B, B]


def infonce_loss(
    scores: torch.Tensor,
    temperature: float,
    hard_negatives: Optional[int] = None,
) -> torch.Tensor:
    """
    Compute InfoNCE loss with in-batch negatives.

    Optionally uses hard negative mining for more challenging training.
    """
    batch_size = scores.size(0)

    if hard_negatives is None or hard_negatives >= batch_size - 1:
        # Standard InfoNCE with all in-batch negatives
        logits = scores / temperature
        labels = torch.arange(batch_size, device=scores.device)
        return F.cross_entropy(logits, labels)

    # Hard negative mining: use top-k negatives per sample
    losses = []
    for i in range(batch_size):
        row = scores[i]
        pos = row[i]
        neg = torch.cat([row[:i], row[i + 1:]])

        # Select top-k hardest negatives
        k = min(hard_negatives, neg.numel())
        neg_topk = torch.topk(neg, k=k, largest=True).values

        logits = torch.cat([pos.unsqueeze(0), neg_topk]) / temperature
        labels = torch.zeros(1, dtype=torch.long, device=scores.device)
        losses.append(F.cross_entropy(logits.unsqueeze(0), labels))

    return torch.stack(losses).mean()


# ============================================================================
# Training Loop
# ============================================================================

def train(config: Dict) -> None:
    """Main training function with all optimizations."""
    training_cfg = config["training"]
    data_cfg = config["data"]
    model_cfg = config["model"]

    # Set seeds for reproducibility
    seed = int(training_cfg.get("seed", 42))
    random.seed(seed)
    torch.manual_seed(seed)

    # Device and dtype setup
    device = select_device(training_cfg.get("device"))
    use_amp = training_cfg.get("use_amp", True) and device.type in ("cuda", "mps")
    if use_amp and device.type == "mps":
        # AMP on MPS can hang during long-running training; force-disable for stability.
        print("Mixed precision: disabled on MPS (known stability issues)")
        use_amp = False
    amp_dtype = get_dtype_for_device(device) if use_amp else torch.float32

    print(f"Device: {device}")
    print(f"Mixed precision: {use_amp} (dtype: {amp_dtype})")

    # Load model
    base_model = model_cfg["base"]
    print(f"Loading model: {base_model}")
    model = T5EncoderModel.from_pretrained(base_model)
    tokenizer = AutoTokenizer.from_pretrained(base_model)

    # Enable gradient checkpointing for memory efficiency
    use_gradient_checkpointing = training_cfg.get("gradient_checkpointing", True)
    if use_gradient_checkpointing:
        model.gradient_checkpointing_enable()
        print("Gradient checkpointing: enabled")

    model.to(device)

    # Apply LoRA
    if training_cfg.get("method") != "lora":
        raise ValueError("Only LoRA training is supported.")

    lora_config = LoraConfig(
        r=training_cfg["lora_r"],
        lora_alpha=training_cfg["lora_alpha"],
        lora_dropout=training_cfg["lora_dropout"],
        target_modules=training_cfg["target_modules"],
    )
    model = get_peft_model(model, lora_config)
    model.print_trainable_parameters()

    # Try to compile model for speedup (PyTorch 2.0+)
    use_compile = training_cfg.get("use_compile", False)
    if use_compile and hasattr(torch, "compile"):
        print("Compiling model with torch.compile()...")
        try:
            model = torch.compile(model)
            print("  Model compiled successfully")
        except Exception as e:
            print(f"  torch.compile() failed: {e}")

    # Output directory
    output_dir = Path(model_cfg["output"])
    output_dir.mkdir(parents=True, exist_ok=True)

    # Load dataset with caching
    train_path = Path(data_cfg["train"])
    languages = set(data_cfg.get("languages", [])) or None
    max_length = int(training_cfg.get("max_length", 512))
    use_cache = training_cfg.get("cache_tokenized", True)

    dataset = CodeSearchDataset(
        data_path=train_path,
        tokenizer=tokenizer,
        max_length=max_length,
        languages=languages,
        use_cache=use_cache,
    )

    # Training hyperparameters
    batch_size = int(training_cfg["batch_size"])
    gradient_accumulation_steps = int(training_cfg.get("gradient_accumulation_steps", 1))
    effective_batch_size = batch_size * gradient_accumulation_steps
    epochs = int(training_cfg["epochs"])
    learning_rate = float(training_cfg["learning_rate"])
    warmup_steps = int(training_cfg.get("warmup_steps", 0))
    weight_decay = float(training_cfg.get("weight_decay", 0.01))
    max_grad_norm = float(training_cfg.get("max_grad_norm", 1.0))

    temperature = float(training_cfg.get("temperature", 0.07))
    hard_negatives = training_cfg.get("hard_negatives")
    log_every = int(training_cfg.get("log_every", 100))
    save_every = int(training_cfg.get("save_every", 0))  # 0 = only at end

    # DataLoader with multiple workers for prefetching
    num_workers = int(training_cfg.get("num_workers", 4))
    # Note: MPS doesn't work well with multiprocessing, reduce workers
    if device.type == "mps":
        num_workers = min(num_workers, 2)

    print(f"\nTraining configuration:")
    print(f"  Dataset size: {len(dataset)}")
    print(f"  Batch size: {batch_size}")
    print(f"  Gradient accumulation: {gradient_accumulation_steps}")
    print(f"  Effective batch size: {effective_batch_size}")
    print(f"  Epochs: {epochs}")
    print(f"  Learning rate: {learning_rate}")
    print(f"  Warmup steps: {warmup_steps}")
    print(f"  Temperature: {temperature}")
    print(f"  Hard negatives: {hard_negatives}")
    print(f"  Num workers: {num_workers}")

    # Calculate total steps
    steps_per_epoch = math.ceil(len(dataset) / batch_size)
    total_optimization_steps = (steps_per_epoch * epochs) // gradient_accumulation_steps

    print(f"  Steps per epoch: {steps_per_epoch}")
    print(f"  Total optimization steps: {total_optimization_steps}")

    # Optimizer with weight decay
    optimizer = torch.optim.AdamW(
        model.parameters(),
        lr=learning_rate,
        weight_decay=weight_decay,
        betas=(0.9, 0.999),
    )

    # Cosine schedule with warmup
    scheduler = get_cosine_schedule_with_warmup(
        optimizer,
        num_warmup_steps=warmup_steps,
        num_training_steps=total_optimization_steps,
    )

    # GradScaler for mixed precision (only for CUDA)
    scaler = None
    if use_amp and device.type == "cuda":
        scaler = torch.cuda.amp.GradScaler()

    # Training loop
    global_step = 0
    best_loss = float("inf")
    training_start = time.time()

    for epoch in range(epochs):
        epoch_start = time.time()
        model.train()

        # Create DataLoader with shuffling
        dataloader = DataLoader(
            dataset,
            batch_size=batch_size,
            shuffle=True,
            collate_fn=collate_fn,
            num_workers=num_workers,
            pin_memory=(device.type == "cuda"),
            drop_last=True,  # Drop last incomplete batch for stable training
            persistent_workers=False,  # Disabled for MPS stability
        )

        total_loss = 0.0
        step_loss = 0.0
        step = 0

        for batch_idx, batch in enumerate(dataloader):
            import sys
            if batch_idx == 0:
                print(f"  Processing first batch...")
                sys.stdout.flush()
            elif batch_idx % 10 == 0:
                print(f"  Batch {batch_idx}...", end="\r")
                sys.stdout.flush()

            # Move batch to device
            query_ids = batch["query_ids"].to(device)
            query_mask = batch["query_mask"].to(device)
            pos_ids = batch["pos_ids"].to(device)
            pos_mask = batch["pos_mask"].to(device)

            # Forward pass with optional mixed precision
            if use_amp and device.type == "cuda":
                with torch.cuda.amp.autocast(dtype=amp_dtype):
                    query_out = model(input_ids=query_ids, attention_mask=query_mask)
                    pos_out = model(input_ids=pos_ids, attention_mask=pos_mask)

                    query_emb = query_out.last_hidden_state
                    pos_emb = pos_out.last_hidden_state

                    scores = maxsim_scores(
                        query_emb, query_mask.float(),
                        pos_emb, pos_mask.float()
                    )
                    loss = infonce_loss(scores, temperature, hard_negatives)
                    loss = loss / gradient_accumulation_steps

                scaler.scale(loss).backward()
            elif use_amp and device.type == "mps":
                # MPS: use autocast but no GradScaler
                with torch.autocast(device_type="mps", dtype=amp_dtype):
                    query_out = model(input_ids=query_ids, attention_mask=query_mask)
                    pos_out = model(input_ids=pos_ids, attention_mask=pos_mask)

                    query_emb = query_out.last_hidden_state
                    pos_emb = pos_out.last_hidden_state

                    scores = maxsim_scores(
                        query_emb, query_mask.float(),
                        pos_emb, pos_mask.float()
                    )
                    loss = infonce_loss(scores, temperature, hard_negatives)
                    loss = loss / gradient_accumulation_steps

                loss.backward()
            else:
                # No mixed precision
                if batch_idx == 0:
                    print(f"    Forward pass (query)...")
                    import sys
                    sys.stdout.flush()
                query_out = model(input_ids=query_ids, attention_mask=query_mask)

                if batch_idx == 0:
                    print(f"    Forward pass (positive)...")
                    sys.stdout.flush()
                pos_out = model(input_ids=pos_ids, attention_mask=pos_mask)

                query_emb = query_out.last_hidden_state
                pos_emb = pos_out.last_hidden_state

                if batch_idx == 0:
                    print(f"    Computing MaxSim scores...")
                    sys.stdout.flush()
                scores = maxsim_scores(
                    query_emb, query_mask.float(),
                    pos_emb, pos_mask.float()
                )

                if batch_idx == 0:
                    print(f"    Computing loss...")
                    sys.stdout.flush()
                loss = infonce_loss(scores, temperature, hard_negatives)
                loss = loss / gradient_accumulation_steps

                if batch_idx == 0:
                    print(f"    Backward pass...")
                    sys.stdout.flush()
                loss.backward()

                if batch_idx == 0:
                    print(f"    First batch complete! Loss: {loss.item():.4f}")
                    sys.stdout.flush()

            # MPS synchronization to avoid hangs
            if device.type == "mps" and batch_idx < 5:
                torch.mps.synchronize()

            step_loss += loss.item() * gradient_accumulation_steps

            # Gradient accumulation step
            if (batch_idx + 1) % gradient_accumulation_steps == 0:
                # Gradient clipping
                if scaler is not None:
                    scaler.unscale_(optimizer)
                torch.nn.utils.clip_grad_norm_(model.parameters(), max_grad_norm)

                # Optimizer step
                if scaler is not None:
                    scaler.step(optimizer)
                    scaler.update()
                else:
                    optimizer.step()

                scheduler.step()
                optimizer.zero_grad()

                total_loss += step_loss
                global_step += 1
                step += 1

                # Logging
                if step % log_every == 0:
                    avg_loss = total_loss / step
                    lr = scheduler.get_last_lr()[0]
                    elapsed = time.time() - epoch_start
                    samples_per_sec = (step * effective_batch_size) / elapsed

                    print(
                        f"Epoch {epoch + 1}/{epochs} | "
                        f"Step {step}/{steps_per_epoch // gradient_accumulation_steps} | "
                        f"Loss: {step_loss:.4f} (avg: {avg_loss:.4f}) | "
                        f"LR: {lr:.2e} | "
                        f"Speed: {samples_per_sec:.1f} samples/s"
                    )

                step_loss = 0.0

                # Checkpoint saving
                if save_every > 0 and global_step % save_every == 0:
                    checkpoint_dir = output_dir / f"checkpoint-{global_step}"
                    model.save_pretrained(checkpoint_dir)
                    print(f"  Saved checkpoint to {checkpoint_dir}")

        # End of epoch
        epoch_time = time.time() - epoch_start
        avg_loss = total_loss / max(step, 1)
        print(f"\nEpoch {epoch + 1} complete:")
        print(f"  Average loss: {avg_loss:.4f}")
        print(f"  Time: {epoch_time / 60:.1f} minutes")
        print(f"  Throughput: {len(dataset) / epoch_time:.1f} samples/s")

        # Save best model
        if avg_loss < best_loss:
            best_loss = avg_loss
            best_dir = output_dir / "best"
            model.save_pretrained(best_dir)
            print(f"  New best model saved to {best_dir}")

    # Save final model
    total_time = time.time() - training_start
    model.save_pretrained(output_dir)
    tokenizer.save_pretrained(output_dir)

    print(f"\n{'='*60}")
    print(f"Training complete!")
    print(f"  Total time: {total_time / 60:.1f} minutes")
    print(f"  Final loss: {avg_loss:.4f}")
    print(f"  Best loss: {best_loss:.4f}")
    print(f"  Output: {output_dir}")
    print(f"{'='*60}")


def dry_run(config: Dict) -> None:
    """Verify pipeline without training."""
    training_cfg = config["training"]
    data_cfg = config["data"]
    model_cfg = config["model"]

    device = select_device(training_cfg.get("device"))
    print(f"Device: {device}")

    base_model = model_cfg["base"]
    print(f"\nLoading model: {base_model}")
    model = T5EncoderModel.from_pretrained(base_model)
    tokenizer = AutoTokenizer.from_pretrained(base_model)

    # Test gradient checkpointing
    if training_cfg.get("gradient_checkpointing", True):
        model.gradient_checkpointing_enable()
        print("Gradient checkpointing: enabled")

    model.to(device)
    print(f"  Model: {model.config.hidden_size}d, {model.config.num_hidden_layers} layers")

    # Apply LoRA
    if training_cfg.get("method") == "lora":
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
    max_length = int(training_cfg.get("max_length", 512))

    print(f"\nLoading dataset: {train_path}")
    dataset = CodeSearchDataset(
        data_path=train_path,
        tokenizer=tokenizer,
        max_length=max_length,
        languages=languages,
        use_cache=training_cfg.get("cache_tokenized", True),
    )

    print(f"  Examples: {len(dataset)}")
    print(f"  Languages: {languages or 'all'}")

    # Training config summary
    batch_size = int(training_cfg["batch_size"])
    grad_accum = int(training_cfg.get("gradient_accumulation_steps", 1))
    epochs = int(training_cfg["epochs"])
    steps_per_epoch = math.ceil(len(dataset) / batch_size)

    print(f"\nTraining configuration:")
    print(f"  Batch size: {batch_size}")
    print(f"  Gradient accumulation: {grad_accum}")
    print(f"  Effective batch: {batch_size * grad_accum}")
    print(f"  Epochs: {epochs}")
    print(f"  Steps/epoch: {steps_per_epoch}")
    print(f"  Total steps: {steps_per_epoch * epochs // grad_accum}")

    # Test one batch
    print("\nTesting one batch...")
    dataloader = DataLoader(
        dataset,
        batch_size=batch_size,
        shuffle=False,
        collate_fn=collate_fn,
    )

    batch = next(iter(dataloader))
    query_ids = batch["query_ids"].to(device)
    query_mask = batch["query_mask"].to(device)
    pos_ids = batch["pos_ids"].to(device)
    pos_mask = batch["pos_mask"].to(device)

    print(f"  Query shape: {query_ids.shape}")
    print(f"  Positive shape: {pos_ids.shape}")

    # Test forward pass
    with torch.no_grad():
        query_out = model(input_ids=query_ids, attention_mask=query_mask)
        pos_out = model(input_ids=pos_ids, attention_mask=pos_mask)

    print(f"  Query embedding: {query_out.last_hidden_state.shape}")
    print(f"  Positive embedding: {pos_out.last_hidden_state.shape}")

    # Test loss computation
    scores = maxsim_scores(
        query_out.last_hidden_state, query_mask.float(),
        pos_out.last_hidden_state, pos_mask.float()
    )
    loss = infonce_loss(scores, temperature=0.07)
    print(f"  Test loss: {loss.item():.4f}")

    # Memory estimate
    if device.type == "mps":
        print(f"\n  (MPS memory stats not available)")
    elif device.type == "cuda":
        mem_alloc = torch.cuda.memory_allocated() / 1e9
        mem_reserved = torch.cuda.memory_reserved() / 1e9
        print(f"\n  GPU memory: {mem_alloc:.2f}GB allocated, {mem_reserved:.2f}GB reserved")

    print("\nDry run complete. Pipeline ready for training.")


def main() -> None:
    parser = argparse.ArgumentParser(description="Optimized XTR fine-tuning for code search")
    parser.add_argument("--config", required=True, help="Path to training config")
    parser.add_argument("--dry-run", action="store_true", help="Verify pipeline without training")
    args = parser.parse_args()

    config = load_config(Path(args.config))

    if args.dry_run:
        dry_run(config)
    else:
        train(config)


if __name__ == "__main__":
    main()
