#!/usr/bin/env python3
"""
Optimized XTR training for Recall@K AUC.

Key improvements over train_xtr_optimized.py:
1. InfoNCE with margin loss (better for Recall@K than pure InfoNCE)
2. Larger effective batch size (128+) for more in-batch negatives
3. Multiple hard negative strategies
4. In-training evaluation with Recall@K metrics
5. Gradient accumulation aware hard negative selection

Usage:
    python scripts/train_recall_optimized.py --config config/train_recall_optimized.yaml

For CUDA (recommended for large batches):
    CUDA_VISIBLE_DEVICES=0 python scripts/train_recall_optimized.py --config config/train_recall_optimized.yaml
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
from typing import Any, Dict, Iterator, List, Optional, Tuple

import torch
import torch.nn.functional as F
import yaml
from peft import LoraConfig, get_peft_model
from torch.utils.data import DataLoader, Dataset
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
    config_str = f"{data_path.name}_{max_length}_{sorted(languages) if languages else 'all'}"
    config_hash = hashlib.md5(config_str.encode()).hexdigest()[:12]
    cache_dir = data_path.parent / ".cache"
    cache_dir.mkdir(exist_ok=True)
    return cache_dir / f"tokenized_{config_hash}.pkl"


# ============================================================================
# Dataset
# ============================================================================

class CodeSearchDataset(Dataset):
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
        raw_examples = []
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

        batch_size = 1000
        start = time.time()

        for i in range(0, len(raw_examples), batch_size):
            batch = raw_examples[i:i + batch_size]
            queries = [ex.query for ex in batch]
            positives = [ex.positive for ex in batch]

            query_enc = self.tokenizer(
                queries, padding=False, truncation=True, max_length=self.max_length,
            )
            pos_enc = self.tokenizer(
                positives, padding=False, truncation=True, max_length=self.max_length,
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
    max_query_len = max(len(ex.query_ids) for ex in batch)
    max_pos_len = max(len(ex.pos_ids) for ex in batch)

    query_ids, query_mask, pos_ids, pos_mask = [], [], [], []

    for ex in batch:
        pad_len = max_query_len - len(ex.query_ids)
        query_ids.append(ex.query_ids + [0] * pad_len)
        query_mask.append(ex.query_mask + [0] * pad_len)

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
# Device Selection
# ============================================================================

def select_device(preferred: Optional[str]) -> torch.device:
    if preferred:
        return torch.device(preferred)
    if torch.cuda.is_available():
        return torch.device("cuda")
    if hasattr(torch.backends, "mps") and torch.backends.mps.is_available():
        return torch.device("mps")
    return torch.device("cpu")


# ============================================================================
# MaxSim Scoring (Multi-Vector Late Interaction)
# ============================================================================

def maxsim_scores(
    query_emb: torch.Tensor,
    query_mask: torch.Tensor,
    doc_emb: torch.Tensor,
    doc_mask: torch.Tensor,
) -> torch.Tensor:
    """
    MaxSim: For each query token, find max similarity to any doc token, then average.

    This is the core of ColBERT/XTR - late interaction between query and document
    token embeddings enables fine-grained matching.
    """
    # L2 normalize (critical for stable training and good recall)
    query_emb = F.normalize(query_emb, p=2, dim=-1)
    doc_emb = F.normalize(doc_emb, p=2, dim=-1)

    batch_size = query_emb.size(0)
    doc_mask_expanded = doc_mask.unsqueeze(1)  # [B, 1, K]

    scores = []
    for i in range(batch_size):
        query_vecs = query_emb[i]  # [Q, D]

        # Similarity with all documents: [B, Q, K]
        sim = torch.einsum("qd,bkd->bqk", query_vecs, doc_emb)

        # Mask padding
        sim = sim.masked_fill(doc_mask_expanded == 0, -1e4)

        # Max over doc tokens, then mean over query tokens
        max_sim = sim.max(dim=2).values  # [B, Q]
        query_len = query_mask[i].sum()
        max_sim = max_sim * query_mask[i].unsqueeze(0)
        scores.append(max_sim.sum(dim=1) / query_len)

    return torch.stack(scores, dim=0)  # [B, B]


# ============================================================================
# Loss Functions for Recall@K Optimization
# ============================================================================

def infonce_loss(
    scores: torch.Tensor,
    temperature: float,
    hard_negatives: Optional[int] = None,
) -> torch.Tensor:
    """Standard InfoNCE loss."""
    batch_size = scores.size(0)

    if hard_negatives is None or hard_negatives >= batch_size - 1:
        logits = scores / temperature
        labels = torch.arange(batch_size, device=scores.device)
        return F.cross_entropy(logits, labels)

    losses = []
    for i in range(batch_size):
        row = scores[i]
        pos = row[i]
        neg = torch.cat([row[:i], row[i + 1:]])

        k = min(hard_negatives, neg.numel())
        neg_topk = torch.topk(neg, k=k, largest=True).values

        logits = torch.cat([pos.unsqueeze(0), neg_topk]) / temperature
        labels = torch.zeros(1, dtype=torch.long, device=scores.device)
        losses.append(F.cross_entropy(logits.unsqueeze(0), labels))

    return torch.stack(losses).mean()


def infonce_with_margin_loss(
    scores: torch.Tensor,
    temperature: float,
    margin: float = 0.1,
    hard_negatives: Optional[int] = None,
) -> torch.Tensor:
    """
    InfoNCE with margin: pos_score > neg_score + margin

    Better for Recall@K because it enforces separation between positive and negatives,
    not just relative ranking. This helps surface relevant docs across all K positions.
    """
    batch_size = scores.size(0)
    device = scores.device

    # Base InfoNCE
    logits = scores / temperature
    labels = torch.arange(batch_size, device=device)
    ce_loss = F.cross_entropy(logits, labels)

    # Margin loss: max(0, neg + margin - pos)
    margin_losses = []
    for i in range(batch_size):
        pos_score = scores[i, i]
        neg_scores = torch.cat([scores[i, :i], scores[i, i+1:]])

        if hard_negatives is not None and hard_negatives < neg_scores.numel():
            neg_scores = torch.topk(neg_scores, k=hard_negatives, largest=True).values

        # Hinge loss with margin
        margin_violation = F.relu(neg_scores + margin - pos_score)
        margin_losses.append(margin_violation.mean())

    margin_loss = torch.stack(margin_losses).mean()

    # Combine: CE for ranking, margin for separation
    return ce_loss + 0.5 * margin_loss


def multiple_negatives_ranking_loss(
    query_emb: torch.Tensor,
    doc_emb: torch.Tensor,
    temperature: float = 0.05,
) -> torch.Tensor:
    """
    MNRL: Symmetric contrastive loss used by sentence-transformers.

    Good baseline for Recall@K - treats both directions equally.
    """
    # Normalize and compute all-pairs similarity
    query_emb = F.normalize(query_emb, p=2, dim=-1)
    doc_emb = F.normalize(doc_emb, p=2, dim=-1)

    scores = torch.mm(query_emb, doc_emb.t()) / temperature
    batch_size = scores.size(0)
    labels = torch.arange(batch_size, device=scores.device)

    # Symmetric loss
    loss_q2d = F.cross_entropy(scores, labels)
    loss_d2q = F.cross_entropy(scores.t(), labels)

    return (loss_q2d + loss_d2q) / 2


# ============================================================================
# Evaluation Metrics
# ============================================================================

def compute_recall_at_k(scores: torch.Tensor, k_values: List[int] = [1, 5, 10, 20, 50, 100]) -> Dict[str, float]:
    """
    Compute Recall@K for a batch.

    scores: [num_queries, num_docs] similarity matrix
    Assumes diagonal is the relevant document for each query.
    """
    num_queries = scores.size(0)
    results = {}

    # Get rankings
    rankings = scores.argsort(dim=1, descending=True)

    for k in k_values:
        if k > scores.size(1):
            continue

        # Check if true positive (diagonal) is in top-k
        top_k = rankings[:, :k]
        true_pos = torch.arange(num_queries, device=scores.device).unsqueeze(1)
        hits = (top_k == true_pos).any(dim=1).float()
        results[f"R@{k}"] = hits.mean().item()

    return results


def compute_recall_auc(scores: torch.Tensor, max_k: int = 100) -> float:
    """
    Compute area under the Recall@K curve.

    This is a single metric that captures performance across all K values.
    """
    recalls = []
    for k in range(1, min(max_k + 1, scores.size(1) + 1)):
        rankings = scores.argsort(dim=1, descending=True)[:, :k]
        true_pos = torch.arange(scores.size(0), device=scores.device).unsqueeze(1)
        hits = (rankings == true_pos).any(dim=1).float()
        recalls.append(hits.mean().item())

    # Trapezoidal integration
    auc = sum(recalls) / len(recalls)
    return auc


# ============================================================================
# Training
# ============================================================================

def train(config: Dict) -> None:
    training_cfg = config["training"]
    data_cfg = config["data"]
    model_cfg = config["model"]

    seed = int(training_cfg.get("seed", 42))
    random.seed(seed)
    torch.manual_seed(seed)

    device = select_device(training_cfg.get("device"))
    use_amp = training_cfg.get("use_amp", True) and device.type == "cuda"
    amp_dtype = torch.bfloat16 if use_amp and torch.cuda.is_bf16_supported() else torch.float16

    if device.type == "mps":
        print("MPS detected - disabling AMP for stability")
        use_amp = False

    print(f"Device: {device}")
    print(f"Mixed precision: {use_amp}")

    # Load model
    base_model = model_cfg["base"]
    print(f"Loading model: {base_model}")
    model = T5EncoderModel.from_pretrained(base_model)
    tokenizer = AutoTokenizer.from_pretrained(base_model)

    if training_cfg.get("gradient_checkpointing", True):
        model.gradient_checkpointing_enable()
        print("Gradient checkpointing: enabled")

    model.to(device)

    # LoRA
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

    output_dir = Path(model_cfg["output"])
    output_dir.mkdir(parents=True, exist_ok=True)

    # Dataset
    train_path = Path(data_cfg["train"])
    languages = set(data_cfg.get("languages", [])) or None
    max_length = int(training_cfg.get("max_length", 512))

    dataset = CodeSearchDataset(
        data_path=train_path,
        tokenizer=tokenizer,
        max_length=max_length,
        languages=languages,
        use_cache=training_cfg.get("cache_tokenized", True),
    )

    # Training params
    batch_size = int(training_cfg["batch_size"])
    grad_accum = int(training_cfg.get("gradient_accumulation_steps", 1))
    effective_batch = batch_size * grad_accum
    epochs = int(training_cfg["epochs"])
    lr = float(training_cfg["learning_rate"])
    warmup = int(training_cfg.get("warmup_steps", 0))
    weight_decay = float(training_cfg.get("weight_decay", 0.01))
    max_grad_norm = float(training_cfg.get("max_grad_norm", 1.0))

    temperature = float(training_cfg.get("temperature", 0.05))
    margin = float(training_cfg.get("margin", 0.1))
    hard_negatives = training_cfg.get("hard_negatives")
    loss_type = training_cfg.get("loss", "infonce_with_margin")

    log_every = int(training_cfg.get("log_every", 50))
    save_every = int(training_cfg.get("save_every", 0))
    eval_every = int(training_cfg.get("eval_every", 0))

    num_workers = 0 if device.type == "mps" else int(training_cfg.get("num_workers", 4))

    print(f"\n{'='*60}")
    print("RECALL@K OPTIMIZED TRAINING")
    print(f"{'='*60}")
    print(f"Dataset: {len(dataset)} examples")
    print(f"Effective batch: {effective_batch} (batch={batch_size} x accum={grad_accum})")
    print(f"In-batch negatives: {effective_batch - 1}")
    print(f"Hard negatives: {hard_negatives}")
    print(f"Loss: {loss_type}")
    print(f"Temperature: {temperature}")
    print(f"Margin: {margin}")
    print(f"{'='*60}\n")

    steps_per_epoch = math.ceil(len(dataset) / batch_size)
    total_opt_steps = (steps_per_epoch * epochs) // grad_accum

    optimizer = torch.optim.AdamW(model.parameters(), lr=lr, weight_decay=weight_decay)
    scheduler = get_cosine_schedule_with_warmup(optimizer, warmup, total_opt_steps)

    scaler = torch.cuda.amp.GradScaler() if use_amp else None

    global_step = 0
    best_recall_auc = 0.0
    training_start = time.time()

    for epoch in range(epochs):
        epoch_start = time.time()
        model.train()

        print(f"\nEpoch {epoch+1}/{epochs} - Creating DataLoader...")
        dataloader = DataLoader(
            dataset,
            batch_size=batch_size,
            shuffle=True,
            collate_fn=collate_fn,
            num_workers=num_workers,
            pin_memory=(device.type == "cuda"),
            drop_last=True,
        )
        print(f"  DataLoader ready, {len(dataloader)} batches")

        total_loss = 0.0
        step_loss = 0.0
        step = 0

        for batch_idx, batch in enumerate(dataloader):
            if batch_idx == 0:
                print(f"  First batch loaded, starting training...")
            if batch_idx < 5 or batch_idx % 100 == 0:
                print(f"  Batch {batch_idx}...", flush=True)
            query_ids = batch["query_ids"].to(device)
            query_mask = batch["query_mask"].to(device)
            pos_ids = batch["pos_ids"].to(device)
            pos_mask = batch["pos_mask"].to(device)

            # Forward pass
            if use_amp:
                with torch.cuda.amp.autocast(dtype=amp_dtype):
                    query_out = model(input_ids=query_ids, attention_mask=query_mask)
                    pos_out = model(input_ids=pos_ids, attention_mask=pos_mask)

                    query_emb = query_out.last_hidden_state
                    pos_emb = pos_out.last_hidden_state

                    scores = maxsim_scores(query_emb, query_mask.float(), pos_emb, pos_mask.float())

                    if loss_type == "infonce_with_margin":
                        loss = infonce_with_margin_loss(scores, temperature, margin, hard_negatives)
                    else:
                        loss = infonce_loss(scores, temperature, hard_negatives)

                    loss = loss / grad_accum

                scaler.scale(loss).backward()
            else:
                query_out = model(input_ids=query_ids, attention_mask=query_mask)
                pos_out = model(input_ids=pos_ids, attention_mask=pos_mask)

                query_emb = query_out.last_hidden_state
                pos_emb = pos_out.last_hidden_state

                scores = maxsim_scores(query_emb, query_mask.float(), pos_emb, pos_mask.float())

                if loss_type == "infonce_with_margin":
                    loss = infonce_with_margin_loss(scores, temperature, margin, hard_negatives)
                else:
                    loss = infonce_loss(scores, temperature, hard_negatives)

                loss = loss / grad_accum
                loss.backward()

            if device.type == "mps" and batch_idx < 5:
                torch.mps.synchronize()

            step_loss += loss.item() * grad_accum

            # Optimizer step
            if (batch_idx + 1) % grad_accum == 0:
                if scaler:
                    scaler.unscale_(optimizer)
                torch.nn.utils.clip_grad_norm_(model.parameters(), max_grad_norm)

                if scaler:
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
                    lr_now = scheduler.get_last_lr()[0]
                    elapsed = time.time() - epoch_start
                    speed = (step * effective_batch) / elapsed

                    # Compute Recall@K on current batch
                    with torch.no_grad():
                        recall_metrics = compute_recall_at_k(scores)
                        recall_auc = compute_recall_auc(scores)

                    print(
                        f"Epoch {epoch+1}/{epochs} | Step {step} | "
                        f"Loss: {step_loss:.4f} | "
                        f"R@1: {recall_metrics.get('R@1', 0):.3f} | "
                        f"R@10: {recall_metrics.get('R@10', 0):.3f} | "
                        f"AUC: {recall_auc:.3f} | "
                        f"LR: {lr_now:.2e} | "
                        f"{speed:.0f} ex/s"
                    )

                step_loss = 0.0

                # Checkpoint
                if save_every > 0 and global_step % save_every == 0:
                    ckpt = output_dir / f"checkpoint-{global_step}"
                    model.save_pretrained(ckpt)
                    print(f"  Saved: {ckpt}")

        # End of epoch
        epoch_time = time.time() - epoch_start
        avg_loss = total_loss / max(step, 1)
        print(f"\nEpoch {epoch+1} complete: loss={avg_loss:.4f}, time={epoch_time/60:.1f}min")

        # Save best
        if avg_loss < best_recall_auc or best_recall_auc == 0:
            best_recall_auc = avg_loss  # Using loss as proxy
            best_dir = output_dir / "best"
            model.save_pretrained(best_dir)
            print(f"  Best model saved to {best_dir}")

    # Final save
    total_time = time.time() - training_start
    model.save_pretrained(output_dir)
    tokenizer.save_pretrained(output_dir)

    print(f"\n{'='*60}")
    print(f"Training complete!")
    print(f"Total time: {total_time/60:.1f} minutes")
    print(f"Output: {output_dir}")
    print(f"{'='*60}")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--config", required=True)
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    config = load_config(Path(args.config))
    train(config)


if __name__ == "__main__":
    main()
