#!/usr/bin/env python3
"""
MLX-based XTR fine-tuning for ~2x faster training on Apple Silicon.

This script ports the core training loop from train_xtr_improved.py to MLX,
achieving ~2x speedup over PyTorch MPS while maintaining the same API.

Usage:
    python scripts/train_xtr_mlx.py --config config/train_mlx.yaml
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
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, Iterator, List, Optional, Tuple

import mlx.core as mx
import mlx.nn as nn
import mlx.optimizers as optim
import numpy as np
import yaml


# ============================================================================
# LoRA Layer Implementation
# ============================================================================

class LoRALinear(nn.Module):
    """Linear layer with LoRA (Low-Rank Adaptation).

    Implements: output = Wx + (BA)x * scale
    where W is frozen, and only B and A are trained.
    """

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

        # Frozen base weights (marked as non-trainable)
        self.weight = mx.zeros((out_features, in_features))
        if bias:
            self.bias = mx.zeros((out_features,))

        # LoRA adapters (trainable) - initialize properly
        self.lora_A = mx.random.normal((r, in_features)) * (1.0 / math.sqrt(r))
        self.lora_B = mx.zeros((out_features, r))

        # Freeze base weights
        self.freeze(keys=["weight", "bias"], recurse=False)

    def __call__(self, x: mx.array) -> mx.array:
        # Base output (frozen)
        out = x @ self.weight.T
        if hasattr(self, "bias"):
            out = out + self.bias

        # LoRA output (trainable)
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
        """Convert a regular Linear layer to LoRALinear."""
        in_features = linear.weight.shape[1]
        out_features = linear.weight.shape[0]
        has_bias = linear.bias is not None

        lora_layer = LoRALinear(
            in_features, out_features, r=r, alpha=alpha, dropout=dropout, bias=has_bias
        )
        # Copy frozen weights
        lora_layer.weight = linear.weight
        if has_bias:
            lora_layer.bias = linear.bias

        return lora_layer


# ============================================================================
# T5 Encoder with LoRA
# ============================================================================

def _relative_position_bucket(
    relative_position, bidirectional=True, num_buckets=32, max_distance=128
):
    """Compute relative position buckets for T5 attention."""
    relative_buckets = 0
    if bidirectional:
        num_buckets //= 2
        relative_buckets += (relative_position > 0).astype(mx.int16) * num_buckets
        relative_position = mx.abs(relative_position)
    else:
        relative_position = -mx.minimum(
            relative_position, mx.zeros_like(relative_position)
        )

    max_exact = num_buckets // 2
    is_small = relative_position < max_exact

    scale = (num_buckets - max_exact) / np.log(max_distance / max_exact)
    relative_position_if_large = max_exact + (
        mx.log(relative_position.astype(mx.float32) / max_exact) * scale
    ).astype(mx.int16)
    relative_position_if_large = mx.minimum(relative_position_if_large, num_buckets - 1)
    relative_buckets += mx.where(
        is_small, relative_position, relative_position_if_large
    )
    return relative_buckets


class RelativePositionBias(nn.Module):
    def __init__(self, config, bidirectional: bool):
        super().__init__()
        self.bidirectional = bidirectional
        self.num_buckets = config.relative_attention_num_buckets
        self.max_distance = getattr(config, "relative_attention_max_distance", 128)
        self.n_heads = config.num_heads
        self.embeddings = nn.Embedding(
            config.relative_attention_num_buckets, config.num_heads
        )

    def __call__(self, query_length: int, key_length: int, offset: int = 0):
        context_position = mx.arange(offset, query_length)[:, None]
        memory_position = mx.arange(key_length)[None, :]
        relative_position = memory_position - context_position
        relative_position_bucket = _relative_position_bucket(
            relative_position,
            bidirectional=self.bidirectional,
            num_buckets=self.num_buckets,
            max_distance=self.max_distance,
        )
        values = self.embeddings(relative_position_bucket)
        return values.transpose(2, 0, 1)


class MultiHeadAttention(nn.Module):
    """Multi-head attention with optional LoRA."""

    def __init__(self, config, use_lora: bool = False, lora_r: int = 8, lora_alpha: float = 16.0):
        super().__init__()
        inner_dim = config.d_kv * config.num_heads
        self.num_heads = config.num_heads

        if use_lora:
            self.query_proj = LoRALinear(config.d_model, inner_dim, r=lora_r, alpha=lora_alpha, bias=False)
            self.value_proj = LoRALinear(config.d_model, inner_dim, r=lora_r, alpha=lora_alpha, bias=False)
            self.key_proj = nn.Linear(config.d_model, inner_dim, bias=False)
            self.out_proj = nn.Linear(inner_dim, config.d_model, bias=False)
        else:
            self.query_proj = nn.Linear(config.d_model, inner_dim, bias=False)
            self.key_proj = nn.Linear(config.d_model, inner_dim, bias=False)
            self.value_proj = nn.Linear(config.d_model, inner_dim, bias=False)
            self.out_proj = nn.Linear(inner_dim, config.d_model, bias=False)

    def __call__(
        self,
        queries: mx.array,
        keys: mx.array,
        values: mx.array,
        mask: Optional[mx.array],
    ) -> mx.array:
        queries = self.query_proj(queries)
        keys = self.key_proj(keys)
        values = self.value_proj(values)

        num_heads = self.num_heads
        B, L, _ = queries.shape
        _, S, _ = keys.shape
        queries = queries.reshape(B, L, num_heads, -1).transpose(0, 2, 1, 3)
        keys = keys.reshape(B, S, num_heads, -1).transpose(0, 2, 3, 1)
        values = values.reshape(B, S, num_heads, -1).transpose(0, 2, 1, 3)

        scores = queries @ keys
        if mask is not None:
            scores = scores + mask.astype(scores.dtype)

        scores = mx.softmax(scores.astype(mx.float32), axis=-1).astype(scores.dtype)
        values_hat = (scores @ values).transpose(0, 2, 1, 3).reshape(B, L, -1)
        return self.out_proj(values_hat)


class DenseActivation(nn.Module):
    def __init__(self, config):
        super().__init__()
        mlp_dims = config.d_ff or config.d_model * 4
        self.gated = hasattr(config, "feed_forward_proj")
        activation = (
            "relu"
            if not self.gated
            else config.feed_forward_proj.removeprefix("gated-")
        )
        if self.gated:
            self.wi_0 = nn.Linear(config.d_model, mlp_dims, bias=False)
            self.wi_1 = nn.Linear(config.d_model, mlp_dims, bias=False)
        else:
            self.wi = nn.Linear(config.d_model, mlp_dims, bias=False)
        self.wo = nn.Linear(mlp_dims, config.d_model, bias=False)

        if activation == "relu":
            self.act = nn.relu
        elif activation == "gelu":
            self.act = nn.gelu
        elif activation == "silu":
            self.act = nn.silu
        else:
            raise ValueError(f"Unknown activation: {activation}")

    def __call__(self, x):
        if self.gated:
            hidden_act = self.act(self.wi_0(x))
            hidden_linear = self.wi_1(x)
            x = hidden_act * hidden_linear
        else:
            x = self.act(self.wi(x))
        return self.wo(x)


class TransformerEncoderLayer(nn.Module):
    def __init__(self, config, use_lora: bool = False, lora_r: int = 8, lora_alpha: float = 16.0):
        super().__init__()
        self.attention = MultiHeadAttention(config, use_lora=use_lora, lora_r=lora_r, lora_alpha=lora_alpha)
        self.ln1 = nn.RMSNorm(config.d_model, eps=config.layer_norm_epsilon)
        self.ln2 = nn.RMSNorm(config.d_model, eps=config.layer_norm_epsilon)
        self.dense = DenseActivation(config)

    def __call__(self, x, mask):
        y = self.ln1(x)
        y = self.attention(y, y, y, mask=mask)
        x = x + y
        y = self.ln2(x)
        y = self.dense(y)
        return x + y


class T5Encoder(nn.Module):
    """T5 Encoder with optional LoRA for fine-tuning."""

    def __init__(self, config, use_lora: bool = False, lora_r: int = 8, lora_alpha: float = 16.0):
        super().__init__()
        self.config = config
        self.embed_tokens = nn.Embedding(config.vocab_size, config.d_model)
        self.layers = [
            TransformerEncoderLayer(config, use_lora=use_lora, lora_r=lora_r, lora_alpha=lora_alpha)
            for _ in range(config.num_layers)
        ]
        self.ln = nn.RMSNorm(config.d_model, eps=config.layer_norm_epsilon)
        self.relative_attention_bias = RelativePositionBias(config, bidirectional=True)

    def __call__(self, input_ids: mx.array, attention_mask: Optional[mx.array] = None) -> mx.array:
        x = self.embed_tokens(input_ids)
        pos_bias = self.relative_attention_bias(x.shape[1], x.shape[1])

        # Apply attention mask if provided
        if attention_mask is not None:
            # Create causal mask from attention mask: [B, 1, 1, S]
            extended_mask = attention_mask[:, None, None, :]
            extended_mask = (1.0 - extended_mask) * -1e9
            pos_bias = pos_bias + extended_mask

        for layer in self.layers:
            x = layer(x, mask=pos_bias)
        return self.ln(x)

    @staticmethod
    def sanitize(weights: Dict[str, mx.array]) -> Dict[str, mx.array]:
        """Convert HuggingFace T5 weights to MLX format."""
        new_weights = {}

        # Handle shared embedding
        if "shared.weight" in weights:
            new_weights["embed_tokens.weight"] = weights["shared.weight"]

        # Find number of layers
        num_layers = max(
            int(k.split(".")[2]) for k in weights.keys()
            if k.startswith("encoder.block.") and k.split(".")[2].isdigit()
        ) + 1

        # Map encoder weights
        for i in range(num_layers):
            prefix = f"encoder.block.{i}"
            new_prefix = f"layers.{i}"

            # Attention weights
            if f"{prefix}.layer.0.SelfAttention.q.weight" in weights:
                new_weights[f"{new_prefix}.attention.query_proj.weight"] = weights[f"{prefix}.layer.0.SelfAttention.q.weight"]
            if f"{prefix}.layer.0.SelfAttention.k.weight" in weights:
                new_weights[f"{new_prefix}.attention.key_proj.weight"] = weights[f"{prefix}.layer.0.SelfAttention.k.weight"]
            if f"{prefix}.layer.0.SelfAttention.v.weight" in weights:
                new_weights[f"{new_prefix}.attention.value_proj.weight"] = weights[f"{prefix}.layer.0.SelfAttention.v.weight"]
            if f"{prefix}.layer.0.SelfAttention.o.weight" in weights:
                new_weights[f"{new_prefix}.attention.out_proj.weight"] = weights[f"{prefix}.layer.0.SelfAttention.o.weight"]

            # Layer norms
            if f"{prefix}.layer.0.layer_norm.weight" in weights:
                new_weights[f"{new_prefix}.ln1.weight"] = weights[f"{prefix}.layer.0.layer_norm.weight"]
            if f"{prefix}.layer.1.layer_norm.weight" in weights:
                new_weights[f"{new_prefix}.ln2.weight"] = weights[f"{prefix}.layer.1.layer_norm.weight"]

            # FFN weights (gated)
            if f"{prefix}.layer.1.DenseReluDense.wi_0.weight" in weights:
                new_weights[f"{new_prefix}.dense.wi_0.weight"] = weights[f"{prefix}.layer.1.DenseReluDense.wi_0.weight"]
            if f"{prefix}.layer.1.DenseReluDense.wi_1.weight" in weights:
                new_weights[f"{new_prefix}.dense.wi_1.weight"] = weights[f"{prefix}.layer.1.DenseReluDense.wi_1.weight"]
            # FFN weights (non-gated)
            if f"{prefix}.layer.1.DenseReluDense.wi.weight" in weights:
                new_weights[f"{new_prefix}.dense.wi.weight"] = weights[f"{prefix}.layer.1.DenseReluDense.wi.weight"]
            if f"{prefix}.layer.1.DenseReluDense.wo.weight" in weights:
                new_weights[f"{new_prefix}.dense.wo.weight"] = weights[f"{prefix}.layer.1.DenseReluDense.wo.weight"]

        # Relative position bias (only in first layer)
        if "encoder.block.0.layer.0.SelfAttention.relative_attention_bias.weight" in weights:
            new_weights["relative_attention_bias.embeddings.weight"] = weights["encoder.block.0.layer.0.SelfAttention.relative_attention_bias.weight"]

        # Final layer norm
        if "encoder.final_layer_norm.weight" in weights:
            new_weights["ln.weight"] = weights["encoder.final_layer_norm.weight"]

        return new_weights

    @classmethod
    def from_pretrained(
        cls,
        path_or_repo: str,
        use_lora: bool = False,
        lora_r: int = 8,
        lora_alpha: float = 16.0,
        dtype: mx.Dtype = mx.float32,
    ) -> "T5Encoder":
        """Load T5 encoder from HuggingFace model."""
        from huggingface_hub import snapshot_download
        from types import SimpleNamespace
        import sys

        path = Path(path_or_repo)
        if not path.exists():
            path = Path(
                snapshot_download(
                    repo_id=path_or_repo,
                    allow_patterns=["*.json", "*.safetensors", "*.model"],
                )
            )

        with open(path / "config.json", "r") as f:
            config = SimpleNamespace(**json.load(f))

        model = cls(config, use_lora=use_lora, lora_r=lora_r, lora_alpha=lora_alpha)
        weights = mx.load(str(path / "model.safetensors"))
        weights = cls.sanitize(weights)
        weights = {k: v.astype(dtype) for k, v in weights.items()}

        # Load weights with strict=False to allow missing LoRA params
        model.load_weights(list(weights.items()), strict=False)

        # Freeze all non-LoRA parameters
        if use_lora:
            model.freeze()  # Freeze everything first
            # Then unfreeze LoRA params
            for layer in model.layers:
                if hasattr(layer.attention.query_proj, 'lora_A'):
                    layer.attention.query_proj.unfreeze(keys=["lora_A", "lora_B"], recurse=False)
                if hasattr(layer.attention.value_proj, 'lora_A'):
                    layer.attention.value_proj.unfreeze(keys=["lora_A", "lora_B"], recurse=False)

        return model


# ============================================================================
# Loss Functions (from mlx_losses.py)
# ============================================================================

def maxsim(
    query_emb: mx.array,
    doc_emb: mx.array,
    query_mask: mx.array,
    doc_mask: mx.array,
) -> mx.array:
    """MaxSim scoring for ColBERT-style retrieval."""
    # L2 normalize
    query_emb = query_emb / (mx.linalg.norm(query_emb, axis=-1, keepdims=True) + 1e-9)
    doc_emb = doc_emb / (mx.linalg.norm(doc_emb, axis=-1, keepdims=True) + 1e-9)

    # All-pairs similarity: [B_q, Q, B_d, K]
    sims = mx.einsum("iqd,jkd->iqjk", query_emb, doc_emb)

    # Mask doc padding
    doc_mask_expanded = doc_mask[None, None, :, :]
    sims = mx.where(doc_mask_expanded > 0, sims, mx.array(-1e9))

    # Max over doc tokens
    max_sims = mx.max(sims, axis=-1)  # [B_q, Q, B_d]

    # Mask query and mean
    query_mask_expanded = query_mask[:, :, None]
    max_sims = mx.where(query_mask_expanded > 0, max_sims, mx.array(0.0))

    query_lengths = mx.sum(query_mask, axis=1, keepdims=True)
    scores = mx.sum(max_sims, axis=1) / (query_lengths + 1e-9)

    return scores


def contrastive_loss(
    query_emb: mx.array,
    doc_emb: mx.array,
    query_mask: mx.array,
    doc_mask: mx.array,
    temperature: float = 0.05,
) -> mx.array:
    """InfoNCE contrastive loss."""
    scores = maxsim(query_emb, doc_emb, query_mask, doc_mask)
    scores = scores / temperature

    batch_size = query_emb.shape[0]
    labels = mx.arange(batch_size)

    log_probs = nn.log_softmax(scores, axis=-1)
    loss = -mx.mean(log_probs[mx.arange(batch_size), labels])

    return loss


def margin_loss(
    query_emb: mx.array,
    doc_emb: mx.array,
    query_mask: mx.array,
    doc_mask: mx.array,
    margin: float = 0.3,
) -> mx.array:
    """Triplet margin loss."""
    scores = maxsim(query_emb, doc_emb, query_mask, doc_mask)
    batch_size = query_emb.shape[0]

    pos_scores = mx.diag(scores)
    mask = 1.0 - mx.eye(batch_size)
    neg_scores = scores * mask + mx.eye(batch_size) * (-1e9)
    hard_neg_scores = mx.max(neg_scores, axis=-1)

    losses = mx.maximum(mx.array(0.0), margin - (pos_scores - hard_neg_scores))
    return mx.mean(losses)


# ============================================================================
# Data Loading
# ============================================================================

@dataclass
class TokenizedExample:
    query_ids: List[int]
    query_mask: List[int]
    pos_ids: List[int]
    pos_mask: List[int]
    language: str = "unknown"


class CodeSearchDataset:
    """Dataset for code search training."""

    def __init__(
        self,
        data_path: Path,
        tokenizer,
        max_length: int,
        use_cache: bool = True,
    ):
        self.max_length = max_length
        self.tokenizer = tokenizer
        self.examples: List[TokenizedExample] = []
        self.language_indices: Dict[str, List[int]] = defaultdict(list)

        config_str = f"{data_path.name}_{max_length}"
        config_hash = hashlib.md5(config_str.encode()).hexdigest()[:12]
        cache_dir = data_path.parent / ".cache"
        cache_dir.mkdir(exist_ok=True)
        cache_path = cache_dir / f"tokenized_mlx_{config_hash}.pkl"

        if use_cache and cache_path.exists():
            print(f"Loading from cache: {cache_path}")
            with cache_path.open("rb") as f:
                cached = pickle.load(f)
                self.examples = cached["examples"]
                self.language_indices = cached["language_indices"]
            print(f"  Loaded {len(self.examples)} examples")
        else:
            self._load_and_tokenize(data_path)
            if use_cache:
                with cache_path.open("wb") as f:
                    pickle.dump({
                        "examples": self.examples,
                        "language_indices": dict(self.language_indices),
                    }, f)

    def _load_and_tokenize(self, data_path: Path) -> None:
        print(f"Tokenizing dataset: {data_path}")

        with data_path.open("r", encoding="utf-8") as f:
            for idx, line in enumerate(f):
                if not line.strip():
                    continue
                try:
                    payload = json.loads(line)
                except json.JSONDecodeError:
                    continue

                lang = payload.get("language", "unknown")
                query = payload.get("query", "")
                positive = payload.get("positive", "")

                if not query or not positive:
                    continue

                # Tokenize
                query_enc = self.tokenizer(
                    query,
                    max_length=self.max_length,
                    truncation=True,
                    padding=False,
                    return_attention_mask=True,
                )
                pos_enc = self.tokenizer(
                    positive,
                    max_length=self.max_length,
                    truncation=True,
                    padding=False,
                    return_attention_mask=True,
                )

                example = TokenizedExample(
                    query_ids=query_enc["input_ids"],
                    query_mask=query_enc["attention_mask"],
                    pos_ids=pos_enc["input_ids"],
                    pos_mask=pos_enc["attention_mask"],
                    language=lang,
                )

                self.examples.append(example)
                self.language_indices[lang].append(len(self.examples) - 1)

                if idx > 0 and idx % 10000 == 0:
                    print(f"  Processed {idx} examples...")

        print(f"  Total: {len(self.examples)} examples")

    def __len__(self) -> int:
        return len(self.examples)

    def __getitem__(self, idx: int) -> TokenizedExample:
        return self.examples[idx]

    def get_languages(self) -> List[str]:
        return list(self.language_indices.keys())


def collate_batch(
    examples: List[TokenizedExample],
) -> Dict[str, mx.array]:
    """Collate examples into a batch of MLX arrays."""
    max_query_len = max(len(ex.query_ids) for ex in examples)
    max_pos_len = max(len(ex.pos_ids) for ex in examples)

    query_ids, query_mask = [], []
    pos_ids, pos_mask = [], []

    for ex in examples:
        # Pad query
        pad_len = max_query_len - len(ex.query_ids)
        query_ids.append(ex.query_ids + [0] * pad_len)
        query_mask.append(ex.query_mask + [0] * pad_len)

        # Pad positive
        pad_len = max_pos_len - len(ex.pos_ids)
        pos_ids.append(ex.pos_ids + [0] * pad_len)
        pos_mask.append(ex.pos_mask + [0] * pad_len)

    return {
        "query_ids": mx.array(query_ids),
        "query_mask": mx.array(query_mask, dtype=mx.float32),
        "pos_ids": mx.array(pos_ids),
        "pos_mask": mx.array(pos_mask, dtype=mx.float32),
    }


def create_batches(
    dataset: CodeSearchDataset,
    batch_size: int,
    shuffle: bool = True,
) -> Iterator[Dict[str, mx.array]]:
    """Create batches from dataset."""
    indices = list(range(len(dataset)))
    if shuffle:
        random.shuffle(indices)

    for i in range(0, len(indices), batch_size):
        batch_indices = indices[i:i + batch_size]
        if len(batch_indices) < batch_size:
            continue  # Skip incomplete batches
        examples = [dataset[idx] for idx in batch_indices]
        yield collate_batch(examples)


# ============================================================================
# Training
# ============================================================================

def get_trainable_params(model: T5Encoder) -> List[Tuple[str, mx.array]]:
    """Get only LoRA parameters for training."""
    params = []
    for name, param in model.parameters().items():
        if "lora_A" in name or "lora_B" in name:
            params.append((name, param))
    return params


def flatten_params(params, prefix: str = "") -> List[Tuple[str, mx.array]]:
    """Flatten nested parameter dict/list to list of (name, array) tuples."""
    result = []
    if isinstance(params, dict):
        for k, v in params.items():
            name = f"{prefix}.{k}" if prefix else k
            result.extend(flatten_params(v, name))
    elif isinstance(params, list):
        for i, v in enumerate(params):
            name = f"{prefix}.{i}" if prefix else str(i)
            result.extend(flatten_params(v, name))
    elif isinstance(params, mx.array):
        result.append((prefix, params))
    return result


def count_parameters(model: T5Encoder) -> Tuple[int, int]:
    """Count total and trainable parameters."""
    total = 0
    trainable = 0
    for name, param in flatten_params(model.parameters()):
        total += param.size
    for name, param in flatten_params(model.trainable_parameters()):
        trainable += param.size
    return total, trainable


def train_step(
    model: T5Encoder,
    batch: Dict[str, mx.array],
    temperature: float,
    margin: float,
    margin_weight: float,
) -> Tuple[mx.array, Dict[str, float]]:
    """Single training step."""
    # Forward pass
    query_emb = model(batch["query_ids"], batch["query_mask"])
    doc_emb = model(batch["pos_ids"], batch["pos_mask"])

    # Compute losses
    loss_contrastive = contrastive_loss(
        query_emb, doc_emb, batch["query_mask"], batch["pos_mask"], temperature
    )
    loss_margin = margin_loss(
        query_emb, doc_emb, batch["query_mask"], batch["pos_mask"], margin
    )

    total_loss = loss_contrastive + margin_weight * loss_margin

    return total_loss, {
        "contrastive": float(loss_contrastive),
        "margin": float(loss_margin),
        "total": float(total_loss),
    }


def _add_grads(g1, g2):
    """Add two nested gradient dicts element-wise."""
    if isinstance(g1, dict):
        return {k: _add_grads(g1[k], g2[k]) for k in g1}
    elif isinstance(g1, list):
        return [_add_grads(a, b) for a, b in zip(g1, g2)]
    else:
        return g1 + g2


def _scale_grads(grads, scale):
    """Scale nested gradient dict by a scalar."""
    if isinstance(grads, dict):
        return {k: _scale_grads(v, scale) for k, v in grads.items()}
    elif isinstance(grads, list):
        return [_scale_grads(v, scale) for v in grads]
    else:
        return grads * scale


def train(config: Dict) -> None:
    """Main training function."""
    training_cfg = config["training"]
    data_cfg = config["data"]
    model_cfg = config["model"]

    # Seeds
    seed = training_cfg.get("seed", 42)
    random.seed(seed)
    np.random.seed(seed)
    mx.random.seed(seed)

    # Load tokenizer
    from transformers import AutoTokenizer
    base_model = model_cfg["base"]
    print(f"Loading tokenizer: {base_model}")
    tokenizer = AutoTokenizer.from_pretrained(base_model)

    # Load model with LoRA
    print(f"Loading model: {base_model}")
    lora_r = training_cfg.get("lora_r", 8)
    lora_alpha = training_cfg.get("lora_alpha", 16)
    dtype_str = training_cfg.get("dtype", "float32")
    dtype = getattr(mx, dtype_str)

    model = T5Encoder.from_pretrained(
        base_model,
        use_lora=True,
        lora_r=lora_r,
        lora_alpha=lora_alpha,
        dtype=dtype,
    )

    total_params, trainable_params = count_parameters(model)
    print(f"Total parameters: {total_params:,}")
    print(f"Trainable parameters: {trainable_params:,} ({100*trainable_params/total_params:.2f}%)")

    # Output directory
    output_dir = Path(model_cfg["output"])
    output_dir.mkdir(parents=True, exist_ok=True)

    # Load dataset
    train_path = Path(data_cfg["train"])
    max_length = training_cfg.get("max_length", 512)

    dataset = CodeSearchDataset(
        data_path=train_path,
        tokenizer=tokenizer,
        max_length=max_length,
    )

    print(f"\nLanguages: {dataset.get_languages()}")
    for lang in dataset.get_languages():
        print(f"  {lang}: {len(dataset.language_indices[lang])}")

    # Training params
    batch_size = training_cfg["batch_size"]
    epochs = training_cfg["epochs"]
    lr = float(training_cfg["learning_rate"])
    warmup_steps = int(training_cfg.get("warmup_steps", 0))
    grad_accum_steps = int(training_cfg.get("gradient_accumulation_steps", 1))
    temperature = training_cfg.get("temperature", 0.07)
    margin = training_cfg.get("margin", 0.2)
    margin_weight = training_cfg.get("margin_weight", 0.1)
    log_every = training_cfg.get("log_every", 50)
    save_every = training_cfg.get("save_every", 1000)

    steps_per_epoch = len(dataset) // batch_size
    total_batches = steps_per_epoch * epochs
    total_opt_steps = math.ceil(total_batches / grad_accum_steps)

    print(f"\nTraining configuration:")
    print(f"  Dataset: {len(dataset)} examples")
    print(f"  Batch size: {batch_size}")
    print(f"  Gradient accumulation: {grad_accum_steps}")
    print(f"  Effective batch size: {batch_size * grad_accum_steps}")
    print(f"  Epochs: {epochs}")
    print(f"  Steps/epoch: {steps_per_epoch}")
    print(f"  Total optimizer steps: {total_opt_steps}")
    print(f"  Learning rate: {lr}")
    print(f"  Warmup steps: {warmup_steps}")
    print(f"  LoRA r={lora_r}, alpha={lora_alpha}")
    print(f"  Temperature: {temperature}")
    print(f"  Margin: {margin} (weight={margin_weight})")

    # Optimizer - AdamW with weight decay
    weight_decay = training_cfg.get("weight_decay", 0.01)
    optimizer = optim.AdamW(learning_rate=lr, weight_decay=weight_decay)

    # Create loss function with gradient
    def loss_fn(model, batch):
        query_emb = model(batch["query_ids"], batch["query_mask"])
        doc_emb = model(batch["pos_ids"], batch["pos_mask"])

        loss_c = contrastive_loss(
            query_emb, doc_emb, batch["query_mask"], batch["pos_mask"], temperature
        )
        loss_m = margin_loss(
            query_emb, doc_emb, batch["query_mask"], batch["pos_mask"], margin
        )

        return loss_c + margin_weight * loss_m

    # Value and gradient function
    loss_and_grad = nn.value_and_grad(model, loss_fn)

    def get_lr(step: int) -> float:
        if step <= 0:
            return 0.0
        if warmup_steps > 0 and step <= warmup_steps:
            return lr * step / warmup_steps
        if total_opt_steps <= warmup_steps:
            return lr
        progress = (step - warmup_steps) / (total_opt_steps - warmup_steps)
        progress = min(max(progress, 0.0), 1.0)
        return lr * 0.5 * (1.0 + math.cos(math.pi * progress))

    # Training loop
    global_step = 0
    batch_step = 0
    start_time = time.time()
    running_loss = 0.0
    running_batches = 0
    accum_grads = None
    accum_count = 0

    print("\nStarting training...")

    for epoch in range(epochs):
        epoch_start = time.time()
        epoch_loss = 0.0
        epoch_steps = 0

        for batch_idx, batch in enumerate(
            create_batches(dataset, batch_size, shuffle=True), start=1
        ):
            batch_step += 1
            # Forward + backward
            loss, grads = loss_and_grad(model, batch)

            # NOTE: grads is a nested dict matching model structure. Only trainable
            # params (LoRA) have gradients due to freeze/unfreeze in from_pretrained().
            # Pass full grads to optimizer - frozen params have no gradients.

            # Evaluate to ensure computation happens
            mx.eval(loss, grads)

            loss_val = float(loss)
            running_loss += loss_val
            running_batches += 1
            epoch_loss += loss_val
            epoch_steps += 1
            accum_count += 1

            # Accumulate gradients (nested dict structure)
            if accum_grads is None:
                accum_grads = grads
            else:
                accum_grads = _add_grads(accum_grads, grads)

            is_last_batch = batch_idx == steps_per_epoch
            if accum_count == grad_accum_steps or is_last_batch:
                global_step += 1
                current_lr = get_lr(global_step)
                optimizer.learning_rate = current_lr
                scaled_grads = _scale_grads(accum_grads, 1.0 / accum_count)
                optimizer.update(model, scaled_grads)
                mx.eval(model.parameters())
                accum_grads = None
                accum_count = 0

                if log_every > 0 and global_step % log_every == 0:
                    avg_loss = running_loss / running_batches
                    elapsed = time.time() - start_time
                    samples_per_sec = batch_step * batch_size / elapsed

                    print(f"Step {global_step}/{total_opt_steps} | "
                          f"Loss: {avg_loss:.4f} | "
                          f"LR: {current_lr:.2e} | "
                          f"Samples/s: {samples_per_sec:.1f} | "
                          f"Elapsed: {elapsed/60:.1f}m")
                    running_loss = 0.0
                    running_batches = 0

                if save_every > 0 and global_step % save_every == 0:
                    checkpoint_path = output_dir / f"checkpoint-{global_step}"
                    checkpoint_path.mkdir(exist_ok=True)
                    save_lora_weights(model, checkpoint_path)
                    print(f"Saved checkpoint: {checkpoint_path}")

        epoch_time = time.time() - epoch_start
        avg_epoch_loss = epoch_loss / epoch_steps
        print(f"\nEpoch {epoch + 1}/{epochs} complete | "
              f"Loss: {avg_epoch_loss:.4f} | "
              f"Time: {epoch_time/60:.1f}m\n")

    # Save final model
    final_path = output_dir / "final"
    final_path.mkdir(exist_ok=True)
    save_lora_weights(model, final_path)
    print(f"\nTraining complete! Model saved to: {final_path}")

    total_time = time.time() - start_time
    print(f"Total training time: {total_time/3600:.2f}h")
    print(f"Average samples/s: {total_batches * batch_size / total_time:.1f}")


def save_lora_weights(model: T5Encoder, path: Path) -> None:
    """Save only LoRA weights."""
    def flatten_params(params, prefix=""):
        """Flatten nested parameter dict to list of (name, array) tuples."""
        result = {}
        if isinstance(params, dict):
            for k, v in params.items():
                name = f"{prefix}.{k}" if prefix else k
                result.update(flatten_params(v, name))
        elif isinstance(params, list):
            for i, v in enumerate(params):
                name = f"{prefix}.{i}" if prefix else str(i)
                result.update(flatten_params(v, name))
        elif isinstance(params, mx.array):
            result[prefix] = params
        return result

    all_params = flatten_params(model.parameters())
    lora_weights = {name: param for name, param in all_params.items()
                    if "lora_A" in name or "lora_B" in name}

    print(f"  Saving {len(lora_weights)} LoRA weights to {path}")
    mx.savez(str(path / "lora_weights.npz"), **lora_weights)

    # Save config
    config = {
        "lora_r": model.layers[0].attention.query_proj.r,
        "lora_alpha": model.layers[0].attention.query_proj.alpha,
    }
    with open(path / "lora_config.json", "w") as f:
        json.dump(config, f)


def load_config(path: Path) -> Dict:
    with path.open("r", encoding="utf-8") as f:
        return yaml.safe_load(f)


def main():
    parser = argparse.ArgumentParser(description="MLX XTR Training")
    parser.add_argument(
        "--config",
        type=str,
        required=True,
        help="Path to YAML config file",
    )
    args = parser.parse_args()

    config = load_config(Path(args.config))
    train(config)


if __name__ == "__main__":
    main()
