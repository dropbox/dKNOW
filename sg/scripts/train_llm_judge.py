#!/usr/bin/env python3
"""
LLM-as-Judge Score-Based Training for Semantic Understanding.

Trains the model to predict LLM-assigned relevance scores (1-5) for (query, code) pairs.
This enables true semantic understanding rather than vocabulary matching.

Usage:
    python scripts/train_llm_judge.py --config config/train_llm_judge.yaml

Key difference from train_xtr_mlx.py:
- Uses MSE/ordinal loss on LLM scores instead of contrastive loss
- Learns to predict relevance scores directly
- Better semantic understanding for conceptual queries
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
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, Iterator, List, Optional, Tuple

import mlx.core as mx
import mlx.nn as nn
import mlx.optimizers as optim
import numpy as np
import yaml

# Import T5 encoder from train_xtr_mlx (same directory)
# This avoids duplicating 400+ lines of T5 implementation
import sys
sys.path.insert(0, str(Path(__file__).parent))
from train_xtr_mlx import T5Encoder, count_parameters, _add_grads, _scale_grads, save_lora_weights


# ============================================================================
# LoRA Layer (copied from train_xtr_mlx.py for standalone use)
# ============================================================================

class LoRALinear(nn.Module):
    """Linear layer with LoRA (Low-Rank Adaptation)."""

    def __init__(
        self,
        in_features: int,
        out_features: int,
        r: int = 8,
        alpha: float = 16.0,
        dropout: float = 0.0,
        bias: bool = False,
    ):
        super().__init__()
        self.in_features = in_features
        self.out_features = out_features
        self.r = r
        self.alpha = alpha
        self.scale = alpha / r

        self.weight = mx.zeros((out_features, in_features))
        if bias:
            self.bias = mx.zeros((out_features,))

        self.lora_A = mx.random.normal((r, in_features)) * (1.0 / math.sqrt(r))
        self.lora_B = mx.zeros((out_features, r))

        self.freeze(keys=["weight", "bias"], recurse=False)

    def __call__(self, x: mx.array) -> mx.array:
        out = x @ self.weight.T
        if hasattr(self, "bias"):
            out = out + self.bias
        lora_out = (x @ self.lora_A.T) @ self.lora_B.T
        out = out + lora_out * self.scale
        return out

    @staticmethod
    def from_linear(
        linear: nn.Linear,
        r: int = 8,
        alpha: float = 16.0,
        dropout: float = 0.0,
    ) -> "LoRALinear":
        in_features = linear.weight.shape[1]
        out_features = linear.weight.shape[0]
        has_bias = linear.bias is not None

        lora_layer = LoRALinear(
            in_features, out_features, r=r, alpha=alpha, dropout=dropout, bias=has_bias
        )
        lora_layer.weight = linear.weight
        if has_bias:
            lora_layer.bias = linear.bias
        return lora_layer


# ============================================================================
# Score-Based Loss Functions
# ============================================================================

def maxsim(
    query_emb: mx.array,
    doc_emb: mx.array,
    query_mask: mx.array,
    doc_mask: mx.array,
) -> mx.array:
    """MaxSim scoring for ColBERT-style multi-vector retrieval."""
    # query_emb: [batch, query_len, dim]
    # doc_emb: [batch, doc_len, dim]

    # Normalize embeddings
    query_emb = query_emb / (mx.linalg.norm(query_emb, axis=-1, keepdims=True) + 1e-8)
    doc_emb = doc_emb / (mx.linalg.norm(doc_emb, axis=-1, keepdims=True) + 1e-8)

    # Compute similarity: [batch, batch, query_len, doc_len]
    sim = mx.einsum("bqd,cld->bcql", query_emb, doc_emb)

    # Mask invalid positions
    doc_mask_expanded = doc_mask[:, None, None, :]  # [batch, 1, 1, doc_len]
    sim = mx.where(doc_mask_expanded > 0, sim, mx.array(-1e9))

    # MaxSim: max over doc tokens for each query token
    max_sim = mx.max(sim, axis=-1)  # [batch, batch, query_len]

    # Mask query positions and sum
    query_mask_expanded = query_mask[:, None, :]  # [batch, 1, query_len]
    max_sim = mx.where(query_mask_expanded > 0, max_sim, mx.array(0.0))

    # Mean over query tokens (normalized by query length)
    query_lengths = mx.sum(query_mask, axis=-1, keepdims=True)[:, None, :]  # [batch, 1, 1]
    scores = mx.sum(max_sim, axis=-1) / (mx.squeeze(query_lengths, axis=-1) + 1e-8)

    return scores  # [batch, batch]


def mse_score_loss(
    query_emb: mx.array,
    doc_emb: mx.array,
    query_mask: mx.array,
    doc_mask: mx.array,
    scores: mx.array,
    scale_factor: float = 5.0,
) -> mx.array:
    """MSE loss between MaxSim scores and LLM relevance scores.

    Args:
        query_emb: Query embeddings [batch, query_len, dim]
        doc_emb: Document embeddings [batch, doc_len, dim]
        query_mask: Query attention mask [batch, query_len]
        doc_mask: Document attention mask [batch, doc_len]
        scores: LLM relevance scores [batch], values 1-5
        scale_factor: Scale MaxSim scores to match LLM score range

    Returns:
        MSE loss value
    """
    # Get diagonal scores (query_i paired with doc_i)
    sim_matrix = maxsim(query_emb, doc_emb, query_mask, doc_mask)
    predicted_scores = mx.diag(sim_matrix)  # [batch]

    # Scale predicted scores to 1-5 range
    # MaxSim typically produces scores in 0.3-0.9 range
    # Linear transform: pred_scaled = pred * 5 + offset
    predicted_scaled = predicted_scores * scale_factor

    # Normalize target scores to 0-1 range for stability
    target_normalized = (scores - 1.0) / 4.0  # 1-5 -> 0-1
    predicted_normalized = predicted_scaled / scale_factor  # Keep in similar range

    # MSE loss
    loss = mx.mean((predicted_normalized - target_normalized) ** 2)
    return loss


def ordinal_loss(
    query_emb: mx.array,
    doc_emb: mx.array,
    query_mask: mx.array,
    doc_mask: mx.array,
    scores: mx.array,
) -> mx.array:
    """Ordinal regression loss for 1-5 scores.

    Predicts P(score >= k) for k in [2,3,4,5] using cumulative thresholds.
    Better suited for ordinal data than MSE.
    """
    sim_matrix = maxsim(query_emb, doc_emb, query_mask, doc_mask)
    predicted_scores = mx.diag(sim_matrix)  # [batch]

    # Create ordinal targets: for score s, targets are [s>=2, s>=3, s>=4, s>=5]
    # Score 1: [0,0,0,0], Score 2: [1,0,0,0], Score 3: [1,1,0,0], etc.
    thresholds = mx.array([2.0, 3.0, 4.0, 5.0])
    ordinal_targets = (scores[:, None] >= thresholds).astype(mx.float32)

    # Predict probabilities using sigmoid on scaled scores
    # Higher MaxSim -> higher probability of being >= each threshold
    pred_logits = predicted_scores[:, None] * 10 - thresholds  # Scale and shift
    pred_probs = mx.sigmoid(pred_logits)

    # Binary cross-entropy for each threshold
    eps = 1e-7
    bce = -(
        ordinal_targets * mx.log(pred_probs + eps) +
        (1 - ordinal_targets) * mx.log(1 - pred_probs + eps)
    )

    return mx.mean(bce)


def contrastive_with_margin_from_scores(
    query_emb: mx.array,
    doc_emb: mx.array,
    query_mask: mx.array,
    doc_mask: mx.array,
    scores: mx.array,
    base_margin: float = 0.2,
    temperature: float = 0.07,
) -> mx.array:
    """Contrastive loss with margin proportional to score difference.

    Better pairs (higher score) should have higher similarity than worse pairs.
    Margin between pairs is proportional to their score difference.
    """
    sim_matrix = maxsim(query_emb, doc_emb, query_mask, doc_mask)
    batch_size = query_emb.shape[0]

    # Diagonal scores (query_i with doc_i)
    pos_scores = mx.diag(sim_matrix)

    # For each query, compute margin loss against all other docs
    total_loss = mx.array(0.0)
    count = 0

    for i in range(batch_size):
        for j in range(batch_size):
            if i != j:
                # Margin proportional to score difference
                score_diff = scores[i] - scores[j]
                margin = base_margin * mx.abs(score_diff) / 4.0  # Normalize by max diff (4)

                # If score_i > score_j, query_i should prefer doc_i over doc_j
                if scores[i] > scores[j]:
                    loss_ij = mx.maximum(
                        mx.array(0.0),
                        margin - (sim_matrix[i, i] - sim_matrix[i, j])
                    )
                    total_loss = total_loss + loss_ij
                    count += 1

    return total_loss / max(count, 1)


# ============================================================================
# Data Loading for Scored Data
# ============================================================================

@dataclass
class ScoredExample:
    query_ids: List[int]
    query_mask: List[int]
    pos_ids: List[int]
    pos_mask: List[int]
    score: float  # LLM relevance score 1-5
    language: str = "unknown"


class ScoredDataset:
    """Dataset for scored (query, code) pairs."""

    def __init__(
        self,
        data_path: Path,
        tokenizer,
        max_length: int,
        score_field: str = "llm_score",
        use_cache: bool = True,
    ):
        self.max_length = max_length
        self.tokenizer = tokenizer
        self.score_field = score_field
        self.examples: List[ScoredExample] = []

        config_str = f"{data_path.name}_{max_length}_{score_field}"
        config_hash = hashlib.md5(config_str.encode()).hexdigest()[:12]
        cache_dir = data_path.parent / ".cache"
        cache_dir.mkdir(exist_ok=True)
        cache_path = cache_dir / f"scored_mlx_{config_hash}.pkl"

        if use_cache and cache_path.exists():
            print(f"Loading from cache: {cache_path}")
            with cache_path.open("rb") as f:
                self.examples = pickle.load(f)
        else:
            print(f"Tokenizing {data_path}...")
            self._load_and_tokenize(data_path)

            with cache_path.open("wb") as f:
                pickle.dump(self.examples, f)
            print(f"Cached to {cache_path}")

        print(f"Loaded {len(self.examples)} scored examples")

        # Print score distribution
        score_counts = defaultdict(int)
        for ex in self.examples:
            score_counts[int(ex.score)] += 1
        print("Score distribution:", dict(sorted(score_counts.items())))

    def _load_and_tokenize(self, data_path: Path):
        with open(data_path) as f:
            for line in f:
                if not line.strip():
                    continue

                ex = json.loads(line)

                # Skip examples without scores
                if self.score_field not in ex:
                    continue

                score = float(ex[self.score_field])
                if score < 1 or score > 5:
                    continue

                query = ex["query"]
                code = ex["positive"]
                language = ex.get("language", "unknown")

                # Tokenize
                q_tokens = self.tokenizer.encode(
                    query, max_length=self.max_length, truncation=True
                )
                c_tokens = self.tokenizer.encode(
                    code, max_length=self.max_length, truncation=True
                )

                self.examples.append(
                    ScoredExample(
                        query_ids=q_tokens,
                        query_mask=[1] * len(q_tokens),
                        pos_ids=c_tokens,
                        pos_mask=[1] * len(c_tokens),
                        score=score,
                        language=language,
                    )
                )

    def __len__(self) -> int:
        return len(self.examples)


def scored_collate_fn(
    examples: List[ScoredExample], pad_id: int = 0
) -> Dict[str, mx.array]:
    """Collate scored examples into batched tensors."""
    batch_size = len(examples)

    max_q_len = max(len(ex.query_ids) for ex in examples)
    max_d_len = max(len(ex.pos_ids) for ex in examples)

    query_ids = np.zeros((batch_size, max_q_len), dtype=np.int32)
    query_mask = np.zeros((batch_size, max_q_len), dtype=np.float32)
    doc_ids = np.zeros((batch_size, max_d_len), dtype=np.int32)
    doc_mask = np.zeros((batch_size, max_d_len), dtype=np.float32)
    scores = np.zeros(batch_size, dtype=np.float32)

    for i, ex in enumerate(examples):
        q_len = len(ex.query_ids)
        d_len = len(ex.pos_ids)

        query_ids[i, :q_len] = ex.query_ids
        query_mask[i, :q_len] = ex.query_mask
        doc_ids[i, :d_len] = ex.pos_ids
        doc_mask[i, :d_len] = ex.pos_mask
        scores[i] = ex.score

    return {
        "query_ids": mx.array(query_ids),
        "query_mask": mx.array(query_mask),
        "doc_ids": mx.array(doc_ids),
        "doc_mask": mx.array(doc_mask),
        "scores": mx.array(scores),
    }


class ScoredDataLoader:
    """DataLoader for scored examples."""

    def __init__(
        self,
        dataset: ScoredDataset,
        batch_size: int,
        shuffle: bool = True,
        pad_id: int = 0,
    ):
        self.dataset = dataset
        self.batch_size = batch_size
        self.shuffle = shuffle
        self.pad_id = pad_id

    def __iter__(self) -> Iterator[Dict[str, mx.array]]:
        indices = list(range(len(self.dataset)))
        if self.shuffle:
            random.shuffle(indices)

        for i in range(0, len(indices), self.batch_size):
            batch_indices = indices[i : i + self.batch_size]
            examples = [self.dataset.examples[j] for j in batch_indices]
            yield scored_collate_fn(examples, self.pad_id)

    def __len__(self) -> int:
        return (len(self.dataset) + self.batch_size - 1) // self.batch_size


# ============================================================================
# Model Loading
# ============================================================================

def load_model(
    model_path: str,
    use_lora: bool = True,
    lora_r: int = 16,
    lora_alpha: float = 32.0,
) -> T5Encoder:
    """Load T5 encoder model with optional LoRA."""
    print(f"Loading model from {model_path}...")
    model = T5Encoder.from_pretrained(
        model_path,
        use_lora=use_lora,
        lora_r=lora_r,
        lora_alpha=lora_alpha,
    )
    return model


# ============================================================================
# Training Loop
# ============================================================================

def train_epoch(
    model: T5Encoder,
    dataloader: ScoredDataLoader,
    optimizer: optim.Optimizer,
    loss_type: str = "mse",
    gradient_accumulation: int = 1,
) -> float:
    """Train for one epoch."""

    def loss_fn(model, batch):
        query_emb = model(batch["query_ids"], batch["query_mask"])
        doc_emb = model(batch["doc_ids"], batch["doc_mask"])

        if loss_type == "mse":
            return mse_score_loss(
                query_emb, doc_emb,
                batch["query_mask"], batch["doc_mask"],
                batch["scores"],
            )
        elif loss_type == "ordinal":
            return ordinal_loss(
                query_emb, doc_emb,
                batch["query_mask"], batch["doc_mask"],
                batch["scores"],
            )
        elif loss_type == "margin_weighted":
            return contrastive_with_margin_from_scores(
                query_emb, doc_emb,
                batch["query_mask"], batch["doc_mask"],
                batch["scores"],
            )
        else:
            raise ValueError(f"Unknown loss type: {loss_type}")

    loss_and_grad_fn = nn.value_and_grad(model, loss_fn)

    total_loss = 0.0
    num_batches = 0
    accumulated_grads = None
    accum_count = 0

    for i, batch in enumerate(dataloader):
        loss, grads = loss_and_grad_fn(model, batch)
        mx.eval(loss, grads)

        # Accumulate gradients
        if accumulated_grads is None:
            accumulated_grads = grads
        else:
            accumulated_grads = _add_grads(accumulated_grads, grads)
        accum_count += 1

        total_loss += float(loss)
        num_batches += 1

        # Update weights every gradient_accumulation steps
        if accum_count >= gradient_accumulation:
            # Average gradients
            if accum_count > 1:
                accumulated_grads = _scale_grads(accumulated_grads, 1.0 / accum_count)
            optimizer.update(model, accumulated_grads)
            mx.eval(model.parameters())
            accumulated_grads = None
            accum_count = 0

        if num_batches % 50 == 0:
            avg_loss = total_loss / num_batches
            print(f"  Batch {num_batches}: loss={avg_loss:.4f}")

    return total_loss / num_batches if num_batches > 0 else 0.0


def evaluate(
    model: T5Encoder,
    dataloader: ScoredDataLoader,
    loss_type: str = "mse",
) -> Tuple[float, float]:
    """Evaluate model on validation set.

    Returns:
        (loss, correlation) - loss and Pearson correlation with LLM scores
    """
    total_loss = 0.0
    num_batches = 0
    all_predicted = []
    all_actual = []

    for batch in dataloader:
        query_emb = model(batch["query_ids"], batch["query_mask"])
        doc_emb = model(batch["doc_ids"], batch["doc_mask"])

        # Compute loss
        if loss_type == "mse":
            loss = mse_score_loss(
                query_emb, doc_emb,
                batch["query_mask"], batch["doc_mask"],
                batch["scores"],
            )
        elif loss_type == "ordinal":
            loss = ordinal_loss(
                query_emb, doc_emb,
                batch["query_mask"], batch["doc_mask"],
                batch["scores"],
            )
        else:
            loss = mx.array(0.0)

        total_loss += loss.item()
        num_batches += 1

        # Collect predictions for correlation
        sim_matrix = maxsim(query_emb, doc_emb, batch["query_mask"], batch["doc_mask"])
        predicted = mx.diag(sim_matrix)
        all_predicted.extend(predicted.tolist())
        all_actual.extend(batch["scores"].tolist())

    avg_loss = total_loss / num_batches if num_batches > 0 else 0.0

    # Compute Pearson correlation
    if len(all_predicted) > 1:
        pred_np = np.array(all_predicted)
        actual_np = np.array(all_actual)
        correlation = np.corrcoef(pred_np, actual_np)[0, 1]
    else:
        correlation = 0.0

    return avg_loss, correlation


# ============================================================================
# Checkpoint Saving
# ============================================================================

def save_checkpoint(
    model: T5Encoder,
    optimizer: optim.Optimizer,
    step,
    output_dir: Path,
    lora_config: Dict[str, Any],
):
    """Save LoRA weights checkpoint."""
    checkpoint_dir = output_dir / f"checkpoint-{step}"
    checkpoint_dir.mkdir(parents=True, exist_ok=True)

    # Use save_lora_weights from train_xtr_mlx
    save_lora_weights(model, checkpoint_dir)

    print(f"Saved checkpoint to {checkpoint_dir}")


def load_config(config_path: str) -> Dict[str, Any]:
    """Load training configuration from YAML."""
    with open(config_path) as f:
        return yaml.safe_load(f)


# ============================================================================
# Main Training Script
# ============================================================================

def main():
    parser = argparse.ArgumentParser(description="LLM-as-Judge Score Training")
    parser.add_argument("--config", required=True, help="Config YAML file")
    parser.add_argument("--resume", help="Resume from checkpoint")
    args = parser.parse_args()

    config = load_config(args.config)

    # Extract config sections
    data_config = config["data"]
    model_config = config["model"]
    train_config = config["training"]

    # Setup paths
    output_dir = Path(model_config["output"])
    output_dir.mkdir(parents=True, exist_ok=True)

    # Load tokenizer
    from transformers import AutoTokenizer
    tokenizer = AutoTokenizer.from_pretrained(model_config["base"])

    # Load datasets
    train_dataset = ScoredDataset(
        Path(data_config["train"]),
        tokenizer,
        max_length=train_config.get("max_length", 512),
        score_field=data_config.get("score_field", "llm_score"),
    )

    val_dataset = None
    if "validation" in data_config:
        val_dataset = ScoredDataset(
            Path(data_config["validation"]),
            tokenizer,
            max_length=train_config.get("max_length", 512),
            score_field=data_config.get("score_field", "llm_score"),
        )

    # Create dataloaders
    train_loader = ScoredDataLoader(
        train_dataset,
        batch_size=train_config.get("batch_size", 16),
        shuffle=True,
        pad_id=tokenizer.pad_token_id or 0,
    )

    val_loader = None
    if val_dataset:
        val_loader = ScoredDataLoader(
            val_dataset,
            batch_size=train_config.get("batch_size", 16),
            shuffle=False,
            pad_id=tokenizer.pad_token_id or 0,
        )

    # LoRA configuration
    lora_config = {
        "r": train_config.get("lora_r", 16),
        "alpha": train_config.get("lora_alpha", 32),
        "dropout": train_config.get("lora_dropout", 0.05),
        "target_modules": train_config.get("target_modules", ["q", "v", "k", "o"]),
    }

    # Load model with LoRA
    model = load_model(
        model_config["base"],
        use_lora=True,
        lora_r=lora_config["r"],
        lora_alpha=lora_config["alpha"],
    )

    # Count parameters
    total_params, trainable_params = count_parameters(model)
    print(f"Total params: {total_params:,}")
    print(f"Trainable params: {trainable_params:,} ({100*trainable_params/total_params:.2f}%)")

    # Setup optimizer
    lr = train_config.get("learning_rate", 1e-5)
    optimizer = optim.AdamW(learning_rate=lr)

    # Training loop
    loss_type = train_config.get("loss", "mse")
    epochs = train_config.get("epochs", 3)
    gradient_accumulation = train_config.get("gradient_accumulation_steps", 1)
    eval_every = train_config.get("eval_every", 500)
    save_every = train_config.get("save_every", 1000)

    print(f"\nStarting training:")
    print(f"  Loss type: {loss_type}")
    print(f"  Epochs: {epochs}")
    print(f"  Batch size: {train_config.get('batch_size', 16)}")
    print(f"  Gradient accumulation: {gradient_accumulation}")
    print(f"  Learning rate: {lr}")

    global_step = 0
    best_correlation = -1.0

    for epoch in range(epochs):
        print(f"\n=== Epoch {epoch + 1}/{epochs} ===")

        avg_loss = train_epoch(
            model,
            train_loader,
            optimizer,
            loss_type=loss_type,
            gradient_accumulation=gradient_accumulation,
        )

        print(f"Epoch {epoch + 1} average loss: {avg_loss:.4f}")

        # Evaluate
        if val_loader:
            val_loss, correlation = evaluate(model, val_loader, loss_type)
            print(f"Validation loss: {val_loss:.4f}, correlation: {correlation:.4f}")

            # Save best model
            if correlation > best_correlation:
                best_correlation = correlation
                save_checkpoint(model, optimizer, epoch + 1, output_dir, lora_config)
                print(f"New best correlation: {correlation:.4f}")

    # Save final model
    save_checkpoint(model, optimizer, "final", output_dir, lora_config)
    print(f"\nTraining complete. Model saved to {output_dir}")


if __name__ == "__main__":
    main()
