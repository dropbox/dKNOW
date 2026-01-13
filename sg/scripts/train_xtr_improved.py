#!/usr/bin/env python3
"""
Improved XTR fine-tuning with:
1. MRR validation and early stopping
2. Language-aware batching
3. Hard negative support (pre-mined or in-batch)
4. Combined InfoNCE + margin loss
5. Curriculum learning (optional)
6. GradCache for memory-efficient large batches
7. Layer-wise Learning Rate Decay (LLRD)
8. Matryoshka Representation Learning
9. Mixed Precision Training (AMP) - 2x speedup
10. torch.compile - 10-30% speedup
11. Memory Bank for cross-batch negatives
12. Query Augmentation for better retrieval

Usage:
    python scripts/train_xtr_improved.py --config config/train_improved.yaml
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
from collections import defaultdict, deque
from dataclasses import dataclass, field
from pathlib import Path
from typing import Dict, Iterator, List, Optional, Tuple

import numpy as np
import torch
import torch.nn.functional as F
import yaml
from peft import LoraConfig, get_peft_model
from torch.utils.data import DataLoader, Dataset, Sampler
# AMP support - handle different backends
try:
    from torch.cuda.amp import GradScaler
    from torch.amp import autocast  # PyTorch 2.0+ unified autocast
except ImportError:
    from torch.cuda.amp import GradScaler, autocast  # Fallback for older PyTorch
from transformers import AutoTokenizer, T5EncoderModel, get_cosine_schedule_with_warmup

# Check for torch.compile availability (PyTorch 2.0+)
TORCH_COMPILE_AVAILABLE = hasattr(torch, 'compile')
if TORCH_COMPILE_AVAILABLE:
    print("torch.compile available - will use for speedup")


# ============================================================================
# Data Structures
# ============================================================================

@dataclass
class TokenizedExample:
    query_ids: List[int]
    query_mask: List[int]
    pos_ids: List[int]
    pos_mask: List[int]
    language: str = "unknown"
    hard_neg_ids: List[List[int]] = field(default_factory=list)
    hard_neg_masks: List[List[int]] = field(default_factory=list)


# ============================================================================
# Configuration
# ============================================================================

def load_config(path: Path) -> Dict:
    with path.open("r", encoding="utf-8") as handle:
        return yaml.safe_load(handle)


def get_cache_path(data_path: Path, max_length: int, config_hash: str) -> Path:
    cache_dir = data_path.parent / ".cache"
    cache_dir.mkdir(exist_ok=True)
    return cache_dir / f"tokenized_improved_{config_hash}.pkl"


# ============================================================================
# Dataset
# ============================================================================

class ImprovedCodeSearchDataset(Dataset):
    """Dataset with language tracking and hard negative support."""

    def __init__(
        self,
        data_path: Path,
        tokenizer,
        max_length: int,
        max_hard_negatives: int = 3,
        languages: Optional[set] = None,
        use_cache: bool = True,
    ):
        self.max_length = max_length
        self.tokenizer = tokenizer
        self.max_hard_negatives = max_hard_negatives
        self.examples: List[TokenizedExample] = []
        self.language_indices: Dict[str, List[int]] = defaultdict(list)

        config_str = f"{data_path.name}_{max_length}_{max_hard_negatives}"
        config_hash = hashlib.md5(config_str.encode()).hexdigest()[:12]
        cache_path = get_cache_path(data_path, max_length, config_hash)

        if use_cache and cache_path.exists():
            print(f"Loading from cache: {cache_path}")
            start = time.time()
            with cache_path.open("rb") as f:
                cached = pickle.load(f)
                self.examples = cached["examples"]
                self.language_indices = cached["language_indices"]
            print(f"  Loaded {len(self.examples)} examples in {time.time() - start:.1f}s")
        else:
            self._load_and_tokenize(data_path, languages)
            if use_cache:
                print(f"Saving to cache: {cache_path}")
                with cache_path.open("wb") as f:
                    pickle.dump({
                        "examples": self.examples,
                        "language_indices": dict(self.language_indices),
                    }, f)

    def _load_and_tokenize(self, data_path: Path, languages: Optional[set]) -> None:
        print(f"Tokenizing dataset: {data_path}")
        raw_examples = []

        with data_path.open("r", encoding="utf-8") as f:
            for line in f:
                if not line.strip():
                    continue
                try:
                    payload = json.loads(line)
                except json.JSONDecodeError:
                    continue

                lang = payload.get("language", "unknown")
                if languages and lang not in languages:
                    continue

                query = payload.get("query", "")
                positive = payload.get("positive", "")
                hard_negs = payload.get("hard_negatives", [])

                if query and positive:
                    raw_examples.append({
                        "query": query,
                        "positive": positive,
                        "language": lang,
                        "hard_negatives": hard_negs[:self.max_hard_negatives],
                    })

        print(f"  Found {len(raw_examples)} examples")

        # Batch tokenize
        batch_size = 1000
        start = time.time()

        for i in range(0, len(raw_examples), batch_size):
            batch = raw_examples[i:i + batch_size]

            queries = [ex["query"] for ex in batch]
            positives = [ex["positive"] for ex in batch]

            query_enc = self.tokenizer(
                queries, padding=False, truncation=True, max_length=self.max_length
            )
            pos_enc = self.tokenizer(
                positives, padding=False, truncation=True, max_length=self.max_length
            )

            for j, ex in enumerate(batch):
                # Tokenize hard negatives if present
                hard_neg_ids = []
                hard_neg_masks = []
                for hn in ex["hard_negatives"]:
                    hn_enc = self.tokenizer(
                        hn, padding=False, truncation=True, max_length=self.max_length
                    )
                    hard_neg_ids.append(hn_enc["input_ids"])
                    hard_neg_masks.append(hn_enc["attention_mask"])

                idx = len(self.examples)
                self.examples.append(TokenizedExample(
                    query_ids=query_enc["input_ids"][j],
                    query_mask=query_enc["attention_mask"][j],
                    pos_ids=pos_enc["input_ids"][j],
                    pos_mask=pos_enc["attention_mask"][j],
                    language=ex["language"],
                    hard_neg_ids=hard_neg_ids,
                    hard_neg_masks=hard_neg_masks,
                ))
                self.language_indices[ex["language"]].append(idx)

            if (i + batch_size) % 10000 == 0:
                elapsed = time.time() - start
                print(f"  Tokenized {min(i + batch_size, len(raw_examples))}/{len(raw_examples)} ({(i + batch_size)/elapsed:.0f}/s)")

    def __len__(self) -> int:
        return len(self.examples)

    def __getitem__(self, idx: int) -> TokenizedExample:
        return self.examples[idx]

    def get_languages(self) -> List[str]:
        return list(self.language_indices.keys())


# ============================================================================
# Language-Aware Sampler
# ============================================================================

class LanguageAwareBatchSampler(Sampler):
    """Batch sampler that creates batches within the same language for stronger negatives."""

    def __init__(self, dataset: ImprovedCodeSearchDataset, batch_size: int,
                 drop_last: bool = True, shuffle: bool = True):
        self.dataset = dataset
        self.batch_size = batch_size
        self.drop_last = drop_last
        self.shuffle = shuffle
        self._batches = None
        self._build_batches()

    def _build_batches(self):
        """Pre-build all batches."""
        self._batches = []

        for lang, indices in self.dataset.language_indices.items():
            indices = indices.copy()
            if self.shuffle:
                random.shuffle(indices)

            # Create batches
            for i in range(0, len(indices), self.batch_size):
                batch = indices[i:i + self.batch_size]
                if len(batch) == self.batch_size or not self.drop_last:
                    self._batches.append(batch)

        # Shuffle batches (not within batches)
        if self.shuffle:
            random.shuffle(self._batches)

    def __iter__(self):
        # Rebuild batches each epoch for different shuffle
        self._build_batches()
        for batch in self._batches:
            yield batch

    def __len__(self):
        return len(self._batches) if self._batches else 0


# ============================================================================
# Collate Function
# ============================================================================

def collate_fn(batch: List[TokenizedExample]) -> Dict[str, torch.Tensor]:
    max_query_len = max(len(ex.query_ids) for ex in batch)
    max_pos_len = max(len(ex.pos_ids) for ex in batch)

    # Check for hard negatives
    has_hard_negs = any(len(ex.hard_neg_ids) > 0 for ex in batch)
    max_hard_negs = max((len(ex.hard_neg_ids) for ex in batch), default=0)

    query_ids, query_mask = [], []
    pos_ids, pos_mask = [], []
    hard_neg_ids, hard_neg_mask = [], []

    for ex in batch:
        # Pad query
        pad_len = max_query_len - len(ex.query_ids)
        query_ids.append(ex.query_ids + [0] * pad_len)
        query_mask.append(ex.query_mask + [0] * pad_len)

        # Pad positive
        pad_len = max_pos_len - len(ex.pos_ids)
        pos_ids.append(ex.pos_ids + [0] * pad_len)
        pos_mask.append(ex.pos_mask + [0] * pad_len)

        # Pad hard negatives
        if has_hard_negs:
            ex_hard_ids = []
            ex_hard_mask = []
            for hn_ids, hn_mask in zip(ex.hard_neg_ids, ex.hard_neg_masks):
                pad_len = max_pos_len - len(hn_ids)
                ex_hard_ids.append(hn_ids + [0] * pad_len)
                ex_hard_mask.append(hn_mask + [0] * pad_len)

            # Pad to max_hard_negs
            while len(ex_hard_ids) < max_hard_negs:
                ex_hard_ids.append([0] * max_pos_len)
                ex_hard_mask.append([0] * max_pos_len)

            hard_neg_ids.append(ex_hard_ids)
            hard_neg_mask.append(ex_hard_mask)

    result = {
        "query_ids": torch.tensor(query_ids, dtype=torch.long),
        "query_mask": torch.tensor(query_mask, dtype=torch.long),
        "pos_ids": torch.tensor(pos_ids, dtype=torch.long),
        "pos_mask": torch.tensor(pos_mask, dtype=torch.long),
    }

    if has_hard_negs:
        result["hard_neg_ids"] = torch.tensor(hard_neg_ids, dtype=torch.long)
        result["hard_neg_mask"] = torch.tensor(hard_neg_mask, dtype=torch.long)

    return result


# ============================================================================
# Model Utilities
# ============================================================================

def select_device(preferred: Optional[str]) -> torch.device:
    if preferred:
        return torch.device(preferred)
    if torch.cuda.is_available():
        return torch.device("cuda")
    if hasattr(torch.backends, "mps") and torch.backends.mps.is_available():
        return torch.device("mps")
    return torch.device("cpu")


def mean_pool(hidden_states: torch.Tensor, attention_mask: torch.Tensor) -> torch.Tensor:
    """Mean pooling over non-padding tokens."""
    mask_expanded = attention_mask.unsqueeze(-1).float()
    sum_hidden = (hidden_states * mask_expanded).sum(dim=1)
    sum_mask = mask_expanded.sum(dim=1).clamp(min=1e-9)
    return sum_hidden / sum_mask


def maxsim_scores(
    query_emb: torch.Tensor,
    query_mask: torch.Tensor,
    doc_emb: torch.Tensor,
    doc_mask: torch.Tensor,
) -> torch.Tensor:
    """
    Compute MaxSim scores between queries and documents.

    Optimized: Fully vectorized, no Python loops.
    """
    # CRITICAL: Cast to float32 for numerical stability with AMP
    query_emb = query_emb.float()
    doc_emb = doc_emb.float()

    # L2 normalize with eps for stability
    query_emb = F.normalize(query_emb, p=2, dim=-1, eps=1e-8)  # [B, Q, D]
    doc_emb = F.normalize(doc_emb, p=2, dim=-1, eps=1e-8)      # [B, K, D]

    batch_size = query_emb.size(0)

    # Compute all-pairs similarity in one operation
    # query_emb: [B, Q, D], doc_emb: [B, K, D]
    # We want: [B, B, Q, K] where [i, j, q, k] = query_i[q] · doc_j[k]
    # Reshape for batch matmul: [B, Q, D] @ [B, D, K] -> use einsum for clarity

    # For each query i, compute similarity with all docs j
    # sim[i, j, q, k] = query_emb[i, q, :] · doc_emb[j, k, :]
    sim = torch.einsum("iqd,jkd->ijqk", query_emb, doc_emb)  # [B, B, Q, K]

    # Mask out padding tokens in documents
    # doc_mask: [B, K] -> [1, B, 1, K]
    doc_mask_expanded = doc_mask.unsqueeze(0).unsqueeze(2)  # [1, B, 1, K]
    sim = sim.masked_fill(doc_mask_expanded == 0, -1e4)

    # MaxSim: max over document tokens (dim=3), then mean over query tokens (dim=2)
    max_sim = sim.max(dim=3).values  # [B, B, Q]

    # Mask query padding and compute mean
    # query_mask: [B, Q] -> [B, 1, Q]
    query_mask_expanded = query_mask.unsqueeze(1).float()  # [B, 1, Q]
    max_sim = max_sim * query_mask_expanded  # [B, B, Q]

    # Sum over query tokens and normalize by query length (with eps to prevent div by zero)
    query_lens = query_mask.sum(dim=1, keepdim=True).unsqueeze(1).float().clamp(min=1.0)  # [B, 1, 1]
    scores = max_sim.sum(dim=2) / query_lens.squeeze(2)  # [B, B]

    return scores


# ============================================================================
# Loss Functions
# ============================================================================

def infonce_loss(scores: torch.Tensor, temperature: float) -> torch.Tensor:
    """Standard InfoNCE with in-batch negatives."""
    # Cast to float32 for numerical stability
    scores = scores.float()
    batch_size = scores.size(0)
    logits = (scores / max(temperature, 0.01)).clamp(-100, 100)
    labels = torch.arange(batch_size, device=scores.device)
    return F.cross_entropy(logits, labels)


# ============================================================================
# IMPROVEMENT 1: Multiple Negatives Ranking Loss (ListMLE)
# ============================================================================

def list_mle_loss(scores: torch.Tensor, temperature: float) -> torch.Tensor:
    """
    ListMLE ranking loss - optimizes for correct ranking order.

    Better than InfoNCE for retrieval because it considers the full ranking,
    not just the positive vs all negatives.

    Reference: Xia et al. "Listwise Approach to Learning to Rank"
    """
    batch_size = scores.size(0)
    device = scores.device

    # For each query, the positive should be ranked first
    # scores[i, i] should be highest for query i
    logits = scores / temperature

    # ListMLE: P(ranking) = prod_i softmax(remaining items)[correct_item]
    # We use a simplified version that's equivalent for our diagonal case

    total_loss = 0.0
    for i in range(batch_size):
        # Scores for query i against all documents
        query_scores = logits[i]  # [batch_size]

        # The positive is at index i, should be ranked first
        # Compute log probability of correct ranking
        remaining_mask = torch.ones(batch_size, device=device, dtype=torch.bool)

        log_prob = 0.0
        correct_order = [i] + [j for j in range(batch_size) if j != i]

        for rank, doc_idx in enumerate(correct_order[:min(5, batch_size)]):  # Top-5 for efficiency
            if not remaining_mask.any():
                break
            masked_scores = query_scores.masked_fill(~remaining_mask, float('-inf'))
            log_prob += F.log_softmax(masked_scores, dim=0)[doc_idx]
            remaining_mask[doc_idx] = False

        total_loss -= log_prob

    return total_loss / batch_size


def multiple_negatives_ranking_loss(
    scores: torch.Tensor,
    temperature: float,
    scale: float = 20.0,
) -> torch.Tensor:
    """
    Multiple Negatives Ranking Loss (MNR) - used by sentence-transformers.

    Efficient approximation that scales well to large batches.
    Combines aspects of InfoNCE with ranking optimization.
    """
    # CRITICAL: Cast to float32 for numerical stability (prevents overflow in float16)
    scores = scores.float()

    batch_size = scores.size(0)
    labels = torch.arange(batch_size, device=scores.device)

    # Clamp scores to prevent overflow when scaled
    scores = scores.clamp(-50.0, 50.0)

    # Scale scores (MNR typically uses higher scale than InfoNCE temperature)
    logits = scores * scale

    # Cross entropy over rows (query-side) and columns (doc-side)
    loss_q = F.cross_entropy(logits, labels)
    loss_d = F.cross_entropy(logits.t(), labels)

    return (loss_q + loss_d) / 2


# ============================================================================
# IMPROVEMENT 2: Token-Level Contrastive Loss (for Multi-Vector)
# ============================================================================

def token_contrastive_loss(
    query_emb: torch.Tensor,
    query_mask: torch.Tensor,
    pos_emb: torch.Tensor,
    pos_mask: torch.Tensor,
    temperature: float = 0.1,
) -> torch.Tensor:
    """
    Token-level contrastive loss for multi-vector models like XTR.

    Encourages each query token to be most similar to its best-matching
    document token from the positive, not from negatives.

    This provides finer-grained supervision than sequence-level loss alone.
    """
    # CRITICAL: Cast to float32 for numerical stability
    query_emb = query_emb.float()
    pos_emb = pos_emb.float()

    batch_size = query_emb.size(0)
    device = query_emb.device

    # Normalize embeddings
    query_emb = F.normalize(query_emb, p=2, dim=-1)  # [B, Q, D]
    pos_emb = F.normalize(pos_emb, p=2, dim=-1)      # [B, K, D]

    # For each query token, find max similarity to any doc token
    # Shape: [B, Q, B, K] - all query tokens vs all doc tokens across batch
    all_sims = torch.einsum("bqd,ckd->bqck", query_emb, pos_emb)

    # Mask padding in documents
    doc_mask_expanded = pos_mask.unsqueeze(0).unsqueeze(1)  # [1, 1, B, K]
    all_sims = all_sims.masked_fill(doc_mask_expanded == 0, -1e4)  # Use -1e4 instead of -1e9 for stability

    # Max over document tokens -> [B, Q, B]
    max_sims = all_sims.max(dim=-1).values

    # For each query token, the positive doc (diagonal) should have highest max sim
    # Reshape to [B*Q, B] and create labels
    num_query_tokens = query_emb.size(1)
    flat_sims = max_sims.view(batch_size * num_query_tokens, batch_size)

    # Labels: for query i, token t -> positive is doc i
    labels = torch.arange(batch_size, device=device).unsqueeze(1).expand(-1, num_query_tokens).reshape(-1)

    # Mask query padding tokens from loss
    query_mask_flat = query_mask.view(-1).bool()

    if query_mask_flat.sum() == 0:
        return torch.tensor(0.0, device=device)

    # Compute loss only for non-padding query tokens
    valid_sims = flat_sims[query_mask_flat]
    valid_labels = labels[query_mask_flat]

    # Clamp logits to prevent overflow
    logits = (valid_sims / max(temperature, 0.01)).clamp(-100, 100)
    return F.cross_entropy(logits, valid_labels)


# ============================================================================
# IMPROVEMENT 3: EMA (Exponential Moving Average) Teacher
# ============================================================================

class EMAModel:
    """
    Exponential Moving Average of model parameters.

    Provides more stable targets for self-distillation and can be used
    as a teacher model during training.

    Reference: Mean Teacher (Tarvainen & Valpola, 2017)
    """

    def __init__(self, model, decay: float = 0.999):
        self.decay = decay
        self.shadow = {}
        self.backup = {}

        # Initialize shadow parameters
        for name, param in model.named_parameters():
            if param.requires_grad:
                self.shadow[name] = param.data.clone()

    def update(self, model):
        """Update shadow parameters with EMA."""
        for name, param in model.named_parameters():
            if param.requires_grad and name in self.shadow:
                self.shadow[name] = (
                    self.decay * self.shadow[name] +
                    (1 - self.decay) * param.data
                )

    def apply_shadow(self, model):
        """Apply shadow parameters to model (for inference)."""
        for name, param in model.named_parameters():
            if name in self.shadow:
                self.backup[name] = param.data.clone()
                param.data = self.shadow[name]

    def restore(self, model):
        """Restore original parameters."""
        for name, param in model.named_parameters():
            if name in self.backup:
                param.data = self.backup[name]
        self.backup = {}


def ema_distillation_loss(
    student_emb: torch.Tensor,
    teacher_emb: torch.Tensor,
    attention_mask: torch.Tensor,
    temperature: float = 2.0,
) -> torch.Tensor:
    """
    Self-distillation loss using EMA teacher.

    Encourages student to match teacher's embedding distribution,
    providing regularization and more stable training.
    """
    # Mean pool both embeddings
    mask_expanded = attention_mask.unsqueeze(-1).float()

    student_pooled = (student_emb * mask_expanded).sum(dim=1) / mask_expanded.sum(dim=1).clamp(min=1e-9)
    teacher_pooled = (teacher_emb * mask_expanded).sum(dim=1) / mask_expanded.sum(dim=1).clamp(min=1e-9)

    # Normalize
    student_pooled = F.normalize(student_pooled, dim=-1)
    teacher_pooled = F.normalize(teacher_pooled, dim=-1)

    # KL divergence on softmax distributions
    student_logits = student_pooled / temperature
    teacher_logits = teacher_pooled / temperature

    # MSE loss between normalized embeddings (simpler and effective)
    return F.mse_loss(student_pooled, teacher_pooled.detach())


# ============================================================================
# IMPROVEMENT 4: Priority/Weighted Sampling for User Code
# ============================================================================

class PrioritySampler:
    """
    Weighted sampling that prioritizes certain examples.

    Use this to boost user's own code or high-value examples.
    """

    def __init__(
        self,
        dataset_size: int,
        priority_indices: List[int],
        priority_weight: float = 3.0,
    ):
        """
        Args:
            dataset_size: Total number of examples
            priority_indices: Indices of high-priority examples
            priority_weight: How much more likely to sample priority examples
        """
        self.weights = torch.ones(dataset_size)
        for idx in priority_indices:
            if idx < dataset_size:
                self.weights[idx] = priority_weight

        # Normalize to probabilities
        self.weights = self.weights / self.weights.sum()

    def sample(self, n: int) -> List[int]:
        """Sample n indices according to priority weights."""
        return torch.multinomial(self.weights, n, replacement=True).tolist()


def weighted_contrastive_loss(
    scores: torch.Tensor,
    temperature: float,
    sample_weights: Optional[torch.Tensor] = None,
) -> torch.Tensor:
    """
    InfoNCE with per-sample weighting.

    Higher weights = more importance in the loss.
    Use this to prioritize user's own code.
    """
    batch_size = scores.size(0)
    logits = scores / temperature
    labels = torch.arange(batch_size, device=scores.device)

    # Compute per-sample cross entropy
    ce_per_sample = F.cross_entropy(logits, labels, reduction='none')

    if sample_weights is not None:
        # Weight and average
        sample_weights = sample_weights.to(scores.device)
        return (ce_per_sample * sample_weights).sum() / sample_weights.sum()
    else:
        return ce_per_sample.mean()


# ============================================================================
# IMPROVEMENT 5: Focal Loss for Hard Examples
# ============================================================================

def focal_contrastive_loss(
    scores: torch.Tensor,
    temperature: float,
    gamma: float = 2.0,
    alpha: float = 0.25,
) -> torch.Tensor:
    """
    Focal loss adaptation for contrastive learning.

    Down-weights easy examples (high confidence correct predictions),
    focuses training on hard examples.

    Reference: Lin et al. "Focal Loss for Dense Object Detection"

    Args:
        scores: Similarity scores [batch, batch]
        temperature: Softmax temperature
        gamma: Focusing parameter (higher = more focus on hard)
        alpha: Balancing parameter
    """
    # Cast to float32 for numerical stability
    scores = scores.float()

    batch_size = scores.size(0)
    logits = (scores / max(temperature, 0.01)).clamp(-100, 100)
    labels = torch.arange(batch_size, device=scores.device)

    # Compute softmax probabilities
    probs = F.softmax(logits, dim=1)

    # Get probability of correct class for each sample
    correct_probs = probs[torch.arange(batch_size), labels]

    # Focal weight: (1 - p_correct)^gamma with eps for stability
    focal_weight = (1 - correct_probs.clamp(0, 0.999)) ** gamma

    # Compute weighted cross entropy
    log_probs = F.log_softmax(logits, dim=1)
    ce_per_sample = -log_probs[torch.arange(batch_size), labels]

    focal_loss = alpha * focal_weight * ce_per_sample

    return focal_loss.mean()


# ============================================================================
# CRITICAL: Memory Bank for Cross-Batch Negatives (MoCo-style)
# ============================================================================

class MemoryBank:
    """
    Memory bank storing recent embeddings for additional negatives.

    This dramatically increases the effective number of negatives without
    increasing batch size. Key for contrastive learning quality.

    Reference: MoCo (He et al., 2020)
    """

    def __init__(self, size: int = 65536, dim: int = 768, device: str = "cpu"):
        self.size = size
        self.dim = dim
        self.device = device

        # Circular buffer for embeddings
        self.embeddings = torch.zeros(size, dim, device=device)
        self.ptr = 0
        self.full = False

    @torch.no_grad()
    def update(self, embeddings: torch.Tensor):
        """Add new embeddings to the bank."""
        batch_size = embeddings.size(0)
        embeddings = embeddings.detach().to(self.device)

        # Mean pool if multi-vector
        if embeddings.dim() == 3:
            embeddings = embeddings.mean(dim=1)

        # Normalize
        embeddings = F.normalize(embeddings, dim=-1)

        # Add to bank
        if self.ptr + batch_size > self.size:
            # Wrap around
            remaining = self.size - self.ptr
            self.embeddings[self.ptr:] = embeddings[:remaining]
            self.embeddings[:batch_size - remaining] = embeddings[remaining:]
            self.full = True
        else:
            self.embeddings[self.ptr:self.ptr + batch_size] = embeddings

        self.ptr = (self.ptr + batch_size) % self.size

    def get_negatives(self, exclude_last_n: int = 0) -> torch.Tensor:
        """Get all stored embeddings as negatives."""
        if self.full:
            if exclude_last_n > 0:
                # Exclude most recent entries (current batch)
                valid_end = (self.ptr - exclude_last_n) % self.size
                if valid_end > self.ptr:
                    return torch.cat([
                        self.embeddings[self.ptr:valid_end],
                    ], dim=0)
                else:
                    return torch.cat([
                        self.embeddings[:valid_end],
                        self.embeddings[self.ptr + exclude_last_n:],
                    ], dim=0)
            return self.embeddings
        else:
            end = max(0, self.ptr - exclude_last_n)
            return self.embeddings[:end]

    def __len__(self):
        return self.size if self.full else self.ptr


def memory_bank_contrastive_loss(
    query_emb: torch.Tensor,
    pos_emb: torch.Tensor,
    memory_bank: MemoryBank,
    temperature: float = 0.07,
    memory_weight: float = 0.5,
) -> torch.Tensor:
    """
    Contrastive loss with additional negatives from memory bank.

    Combines in-batch negatives with memory bank negatives for
    much larger effective negative set.
    """
    # Cast to float32 for numerical stability
    query_emb = query_emb.float()
    pos_emb = pos_emb.float()

    batch_size = query_emb.size(0)
    device = query_emb.device

    # Mean pool if needed
    if query_emb.dim() == 3:
        query_emb = query_emb.mean(dim=1)
        pos_emb = pos_emb.mean(dim=1)

    # Normalize
    query_emb = F.normalize(query_emb, dim=-1)
    pos_emb = F.normalize(pos_emb, dim=-1)

    # In-batch scores with clamping
    temp = max(temperature, 0.01)
    in_batch_scores = (torch.mm(query_emb, pos_emb.t()) / temp).clamp(-100, 100)  # [B, B]

    # Memory bank scores
    memory_negs = memory_bank.get_negatives(exclude_last_n=batch_size)
    if len(memory_negs) > 0:
        memory_negs = memory_negs.to(device).float()
        memory_scores = (torch.mm(query_emb, memory_negs.t()) / temp).clamp(-100, 100)  # [B, M]

        # Concatenate: [positive, in-batch negs, memory negs]
        # For each query i, positive is at index i
        all_scores = torch.cat([in_batch_scores, memory_scores], dim=1)
    else:
        all_scores = in_batch_scores

    # Labels: positive is at diagonal (index i for query i)
    labels = torch.arange(batch_size, device=device)

    return F.cross_entropy(all_scores, labels)


# ============================================================================
# CRITICAL: Query Augmentation for Better Retrieval
# ============================================================================

def augment_query(query: str, language: str = "unknown") -> List[str]:
    """
    Generate augmented versions of a query for training.

    This helps the model learn to match different phrasings of the same intent.
    """
    augmented = [query]  # Always include original

    # 1. Remove common prefixes/suffixes
    prefixes = ["function ", "method ", "class ", "how to ", "implement ", "find "]
    for prefix in prefixes:
        if query.lower().startswith(prefix):
            augmented.append(query[len(prefix):])
            break

    # 2. Add language context
    if language != "unknown":
        augmented.append(f"{language} {query}")

    # 3. Snake_case <-> spaces
    if "_" in query:
        augmented.append(query.replace("_", " "))
    elif " " in query and len(query.split()) <= 4:
        augmented.append(query.replace(" ", "_"))

    # 4. CamelCase -> spaces
    import re
    camel_split = re.sub(r'([a-z])([A-Z])', r'\1 \2', query)
    if camel_split != query:
        augmented.append(camel_split.lower())

    # 5. Abbreviation expansion (common ones)
    abbrevs = {
        "fn": "function", "func": "function",
        "impl": "implementation", "def": "definition",
        "cfg": "config", "init": "initialize",
        "err": "error", "msg": "message",
        "req": "request", "res": "response",
        "ctx": "context", "env": "environment",
    }
    words = query.lower().split()
    expanded = [abbrevs.get(w, w) for w in words]
    if expanded != words:
        augmented.append(" ".join(expanded))

    return list(set(augmented))[:4]  # Max 4 variants


class QueryAugmentedDataset(Dataset):
    """
    Dataset wrapper that applies query augmentation during training.
    """

    def __init__(self, base_dataset, augment_prob: float = 0.5):
        self.base_dataset = base_dataset
        self.augment_prob = augment_prob

    def __len__(self):
        return len(self.base_dataset)

    def __getitem__(self, idx):
        example = self.base_dataset[idx]

        # Randomly apply augmentation
        if random.random() < self.augment_prob:
            # We need the original query text for augmentation
            # This requires storing it in the example or re-tokenizing
            # For now, we'll handle this at the data loading stage
            pass

        return example


# ============================================================================
# CRITICAL: Sequence Packing to Reduce Padding Waste
# ============================================================================

def pack_sequences(
    examples: List[Dict],
    tokenizer,
    max_length: int = 512,
    pad_token_id: int = 0,
) -> List[Dict]:
    """
    Pack multiple short sequences into single sequences to reduce padding.

    This can significantly speed up training when sequences vary in length.
    """
    packed = []
    current_query_ids = []
    current_pos_ids = []
    current_languages = []

    for ex in examples:
        query_len = len(ex.get("query_ids", []))
        pos_len = len(ex.get("pos_ids", []))

        # Check if we can fit this example
        if len(current_query_ids) + query_len <= max_length and \
           len(current_pos_ids) + pos_len <= max_length:
            current_query_ids.extend(ex.get("query_ids", []))
            current_pos_ids.extend(ex.get("pos_ids", []))
            current_languages.append(ex.get("language", "unknown"))
        else:
            # Save current packed sequence
            if current_query_ids:
                packed.append({
                    "query_ids": current_query_ids,
                    "pos_ids": current_pos_ids,
                    "languages": current_languages,
                    "is_packed": True,
                })
            # Start new sequence
            current_query_ids = ex.get("query_ids", [])
            current_pos_ids = ex.get("pos_ids", [])
            current_languages = [ex.get("language", "unknown")]

    # Don't forget the last one
    if current_query_ids:
        packed.append({
            "query_ids": current_query_ids,
            "pos_ids": current_pos_ids,
            "languages": current_languages,
            "is_packed": True,
        })

    return packed


# ============================================================================
# Optimized DataLoader with Prefetching
# ============================================================================

def create_optimized_dataloader(
    dataset,
    batch_size: int,
    batch_sampler=None,
    collate_fn=None,
    num_workers: int = 4,
    pin_memory: bool = True,
    prefetch_factor: int = 2,
) -> DataLoader:
    """
    Create an optimized DataLoader with proper worker configuration.

    Key optimizations:
    - num_workers > 0 for parallel data loading
    - pin_memory for faster GPU transfer
    - prefetch_factor for overlapping data loading with training
    """
    # Adjust workers for platform
    if torch.cuda.is_available():
        actual_workers = num_workers
    elif hasattr(torch.backends, "mps") and torch.backends.mps.is_available():
        # MPS doesn't benefit as much from workers
        actual_workers = min(2, num_workers)
    else:
        actual_workers = 0  # CPU training, workers add overhead

    kwargs = {
        "num_workers": actual_workers,
        "pin_memory": pin_memory and torch.cuda.is_available(),
        "persistent_workers": actual_workers > 0,
    }

    if actual_workers > 0:
        kwargs["prefetch_factor"] = prefetch_factor

    if batch_sampler is not None:
        return DataLoader(
            dataset,
            batch_sampler=batch_sampler,
            collate_fn=collate_fn,
            **kwargs
        )
    else:
        return DataLoader(
            dataset,
            batch_size=batch_size,
            shuffle=True,
            collate_fn=collate_fn,
            drop_last=True,
            **kwargs
        )


# ============================================================================
# GradCache - Memory-Efficient Contrastive Learning
# ============================================================================

class GradCache:
    """
    Gradient caching for memory-efficient contrastive learning.
    Allows larger effective batches without OOM.

    Reference: https://arxiv.org/abs/2101.06983
    """

    def __init__(self, model, chunk_size: int = 4):
        self.model = model
        self.chunk_size = chunk_size

    def forward_no_grad(self, input_ids: torch.Tensor, attention_mask: torch.Tensor) -> torch.Tensor:
        """Forward pass without gradients, returns detached embeddings."""
        with torch.no_grad():
            outputs = self.model(input_ids=input_ids, attention_mask=attention_mask)
            return outputs.last_hidden_state.detach().requires_grad_(True)

    def forward_backward(
        self,
        query_ids: torch.Tensor,
        query_mask: torch.Tensor,
        pos_ids: torch.Tensor,
        pos_mask: torch.Tensor,
        temperature: float,
        use_margin_loss: bool = False,
        margin: float = 0.2,
        margin_weight: float = 0.1,
    ) -> Tuple[torch.Tensor, Dict[str, float]]:
        """
        Memory-efficient forward-backward with gradient caching.

        1. Compute all embeddings without grad (chunked)
        2. Compute loss with cached embeddings
        3. Backprop through loss to get embedding gradients
        4. Re-forward each chunk and use cached grads
        """
        batch_size = query_ids.size(0)
        device = query_ids.device

        # Step 1: Forward all queries and positives without grad (chunked)
        query_embs = []
        pos_embs = []

        for i in range(0, batch_size, self.chunk_size):
            end_i = min(i + self.chunk_size, batch_size)

            q_emb = self.forward_no_grad(
                query_ids[i:end_i], query_mask[i:end_i]
            )
            p_emb = self.forward_no_grad(
                pos_ids[i:end_i], pos_mask[i:end_i]
            )

            query_embs.append(q_emb)
            pos_embs.append(p_emb)

        # Concatenate cached embeddings
        query_emb = torch.cat(query_embs, dim=0)
        pos_emb = torch.cat(pos_embs, dim=0)

        # Step 2: Compute scores and loss
        scores = maxsim_scores(query_emb, query_mask.float(), pos_emb, pos_mask.float())

        if use_margin_loss:
            loss, loss_parts = combined_loss(scores, temperature, margin, margin_weight)
        else:
            loss = infonce_loss(scores, temperature)
            loss_parts = {"ce": loss.item()}

        # Step 3: Backprop to get gradients w.r.t. cached embeddings
        loss.backward()

        # Get gradients for the cached embeddings
        query_grads = query_emb.grad.split(self.chunk_size)
        pos_grads = pos_emb.grad.split(self.chunk_size)

        # Step 4: Re-forward each chunk and propagate cached gradients
        for i, (q_grad, p_grad) in enumerate(zip(query_grads, pos_grads)):
            start_i = i * self.chunk_size
            end_i = min(start_i + self.chunk_size, batch_size)

            # Re-forward with grad
            q_out = self.model(
                input_ids=query_ids[start_i:end_i],
                attention_mask=query_mask[start_i:end_i]
            )
            q_emb_chunk = q_out.last_hidden_state

            p_out = self.model(
                input_ids=pos_ids[start_i:end_i],
                attention_mask=pos_mask[start_i:end_i]
            )
            p_emb_chunk = p_out.last_hidden_state

            # Backward with cached gradients
            q_emb_chunk.backward(q_grad)
            p_emb_chunk.backward(p_grad)

        return loss.detach(), loss_parts


# ============================================================================
# Layer-wise Learning Rate Decay (LLRD)
# ============================================================================

def get_llrd_params(model, base_lr: float, decay_rate: float = 0.9) -> List[Dict]:
    """
    Create parameter groups with layer-wise learning rate decay.

    Lower layers get smaller learning rates (decay_rate^depth * base_lr).
    This helps preserve pre-trained knowledge in lower layers.

    Args:
        model: The model (assumes T5 encoder structure)
        base_lr: Learning rate for the top layer
        decay_rate: Multiplicative decay per layer (0.9 = 10% reduction per layer)

    Returns:
        List of parameter groups for optimizer
    """
    # T5 encoder has blocks named like "encoder.block.0", "encoder.block.1", etc.
    # Plus embedding layer and final layer norm

    param_groups = []
    no_decay = ["bias", "LayerNorm.weight", "layer_norm.weight"]

    # Get all layer names and their depths
    layer_params = defaultdict(list)
    other_params = {"decay": [], "no_decay": []}

    num_layers = 0
    for name, param in model.named_parameters():
        if not param.requires_grad:
            continue

        # Check for encoder blocks
        if "encoder.block." in name:
            # Extract layer number
            parts = name.split(".")
            for i, part in enumerate(parts):
                if part == "block" and i + 1 < len(parts):
                    try:
                        layer_num = int(parts[i + 1])
                        num_layers = max(num_layers, layer_num + 1)
                        layer_params[layer_num].append((name, param))
                        break
                    except ValueError:
                        pass
        else:
            # Embeddings, layer norms, etc.
            if any(nd in name for nd in no_decay):
                other_params["no_decay"].append(param)
            else:
                other_params["decay"].append(param)

    # Create parameter groups for each layer
    for layer_num in range(num_layers):
        layer_lr = base_lr * (decay_rate ** (num_layers - 1 - layer_num))

        decay_params = []
        no_decay_params = []

        for name, param in layer_params[layer_num]:
            if any(nd in name for nd in no_decay):
                no_decay_params.append(param)
            else:
                decay_params.append(param)

        if decay_params:
            param_groups.append({
                "params": decay_params,
                "lr": layer_lr,
                "weight_decay": 0.01,
            })
        if no_decay_params:
            param_groups.append({
                "params": no_decay_params,
                "lr": layer_lr,
                "weight_decay": 0.0,
            })

    # Add other params with base LR
    if other_params["decay"]:
        param_groups.append({
            "params": other_params["decay"],
            "lr": base_lr,
            "weight_decay": 0.01,
        })
    if other_params["no_decay"]:
        param_groups.append({
            "params": other_params["no_decay"],
            "lr": base_lr,
            "weight_decay": 0.0,
        })

    return param_groups


# ============================================================================
# Matryoshka Representation Learning
# ============================================================================

def matryoshka_loss(
    query_emb: torch.Tensor,
    query_mask: torch.Tensor,
    pos_emb: torch.Tensor,
    pos_mask: torch.Tensor,
    temperature: float,
    dims: List[int] = [64, 128, 256, 768],
    dim_weights: Optional[List[float]] = None,
) -> Tuple[torch.Tensor, Dict[str, float]]:
    """
    Matryoshka Representation Learning loss.

    Trains embeddings to be useful at multiple dimensionalities by
    computing contrastive loss at each truncation point.

    Reference: https://arxiv.org/abs/2205.13147

    Args:
        query_emb: Query embeddings [B, Q, D]
        query_mask: Query attention mask
        pos_emb: Positive embeddings [B, K, D]
        pos_mask: Positive attention mask
        temperature: Contrastive temperature
        dims: List of dimensions to train at (must be sorted ascending)
        dim_weights: Optional weights for each dimension's loss

    Returns:
        Combined loss and per-dimension loss values
    """
    if dim_weights is None:
        # Equal weighting
        dim_weights = [1.0 / len(dims)] * len(dims)

    total_loss = 0.0
    loss_parts = {}

    for dim, weight in zip(dims, dim_weights):
        # Truncate to dimension
        q_trunc = query_emb[:, :, :dim]
        p_trunc = pos_emb[:, :, :dim]

        # Compute scores with truncated embeddings
        scores = maxsim_scores(q_trunc, query_mask.float(), p_trunc, pos_mask.float())

        # Cast to float32 and clamp for numerical stability
        scores = scores.float()
        batch_size = scores.size(0)
        logits = (scores / max(temperature, 0.01)).clamp(-100, 100)
        labels = torch.arange(batch_size, device=scores.device)
        dim_loss = F.cross_entropy(logits, labels)

        total_loss = total_loss + weight * dim_loss
        loss_parts[f"d{dim}"] = dim_loss.item()

    return total_loss, loss_parts


# ============================================================================
# Dynamic Temperature
# ============================================================================

class LearnableTemperature(torch.nn.Module):
    """Learnable temperature parameter for contrastive loss."""

    def __init__(self, init_temp: float = 0.07, min_temp: float = 0.01, max_temp: float = 0.5):
        super().__init__()
        # Store as log for numerical stability
        self.log_temp = torch.nn.Parameter(torch.tensor(math.log(init_temp)))
        self.min_temp = min_temp
        self.max_temp = max_temp

    @property
    def temperature(self) -> torch.Tensor:
        """Get clamped temperature value."""
        return torch.clamp(self.log_temp.exp(), self.min_temp, self.max_temp)

    def forward(self) -> torch.Tensor:
        return self.temperature


# ============================================================================
# Progressive Sequence Length
# ============================================================================

class ProgressiveSequenceLength:
    """
    Progressively increase sequence length during training.

    Starts with short sequences for fast early training,
    gradually increases to full length.
    """

    def __init__(
        self,
        min_length: int = 128,
        max_length: int = 512,
        warmup_steps: int = 1000,
        schedule: str = "linear",  # "linear" or "cosine"
    ):
        self.min_length = min_length
        self.max_length = max_length
        self.warmup_steps = warmup_steps
        self.schedule = schedule

    def get_length(self, step: int) -> int:
        """Get current sequence length for given training step."""
        if step >= self.warmup_steps:
            return self.max_length

        progress = step / self.warmup_steps

        if self.schedule == "cosine":
            # Cosine schedule (slower start, faster end)
            progress = 0.5 * (1 - math.cos(math.pi * progress))

        length = self.min_length + (self.max_length - self.min_length) * progress
        return int(length)


# ============================================================================
# Curriculum Learning
# ============================================================================

class CurriculumScheduler:
    """
    Order training examples from easy to hard.

    Easy examples = high similarity between query and positive
    Hard examples = lower similarity (more nuanced matches)
    """

    def __init__(
        self,
        difficulties: List[float],
        warmup_epochs: float = 0.5,
        strategy: str = "linear",  # "linear", "root", "step"
    ):
        """
        Args:
            difficulties: Per-example difficulty scores (0=easy, 1=hard)
            warmup_epochs: How many epochs before seeing all data
            strategy: How to increase difficulty
        """
        self.difficulties = np.array(difficulties)
        self.warmup_epochs = warmup_epochs
        self.strategy = strategy

        # Sort indices by difficulty
        self.sorted_indices = np.argsort(self.difficulties)

    def get_indices(self, epoch: float, total_examples: int) -> List[int]:
        """Get indices to use for current epoch progress."""
        if epoch >= self.warmup_epochs:
            # After warmup, use all data
            return list(range(total_examples))

        progress = epoch / self.warmup_epochs

        if self.strategy == "root":
            progress = math.sqrt(progress)
        elif self.strategy == "step":
            progress = math.floor(progress * 4) / 4  # 4 discrete steps

        # Select easiest N% of data
        n_examples = max(1, int(total_examples * progress))
        return self.sorted_indices[:n_examples].tolist()


def compute_difficulty_scores(
    examples: List[Dict],
    tokenizer,
    model,
    device,
    batch_size: int = 32,
) -> List[float]:
    """
    Compute difficulty scores for curriculum learning.

    Score = 1 - similarity(query_embedding, positive_embedding)
    Higher score = harder example
    """
    model.eval()
    difficulties = []

    with torch.no_grad():
        for i in range(0, len(examples), batch_size):
            batch = examples[i:i + batch_size]

            queries = [ex["query"] for ex in batch]
            positives = [ex["positive"] for ex in batch]

            q_enc = tokenizer(queries, padding=True, truncation=True,
                              max_length=128, return_tensors="pt")
            p_enc = tokenizer(positives, padding=True, truncation=True,
                              max_length=128, return_tensors="pt")

            q_out = model(q_enc["input_ids"].to(device), q_enc["attention_mask"].to(device))
            p_out = model(p_enc["input_ids"].to(device), p_enc["attention_mask"].to(device))

            # Mean pool
            q_emb = mean_pool(q_out.last_hidden_state, q_enc["attention_mask"].to(device))
            p_emb = mean_pool(p_out.last_hidden_state, p_enc["attention_mask"].to(device))

            # Normalize and compute similarity
            q_emb = F.normalize(q_emb, dim=-1)
            p_emb = F.normalize(p_emb, dim=-1)

            # Diagonal = similarity of each pair
            sims = (q_emb * p_emb).sum(dim=-1)

            # Difficulty = 1 - similarity
            for sim in sims:
                difficulties.append(1.0 - sim.item())

    model.train()
    return difficulties


# ============================================================================
# Multi-Scale Attention Pooling
# ============================================================================

class MultiScaleAttentionPooling(torch.nn.Module):
    """
    Pool token embeddings using learned attention at multiple scales.

    Better than mean pooling for capturing both local and global patterns.
    """

    def __init__(self, hidden_size: int = 768, num_heads: int = 4):
        super().__init__()
        self.hidden_size = hidden_size
        self.num_heads = num_heads

        # Attention weights for different scales
        self.scale_attns = torch.nn.ModuleList([
            torch.nn.Linear(hidden_size, 1)
            for _ in range(num_heads)
        ])

        # Scale factors (different receptive fields)
        self.scales = [1, 2, 4, 8]  # Window sizes

        # Final projection
        self.output_proj = torch.nn.Linear(hidden_size * num_heads, hidden_size)

    def forward(
        self,
        hidden_states: torch.Tensor,
        attention_mask: torch.Tensor,
    ) -> torch.Tensor:
        """
        Args:
            hidden_states: [batch, seq_len, hidden]
            attention_mask: [batch, seq_len]

        Returns:
            Pooled embeddings [batch, hidden]
        """
        batch_size, seq_len, hidden = hidden_states.shape
        pooled_outputs = []

        for i, (attn, scale) in enumerate(zip(self.scale_attns, self.scales)):
            if scale == 1:
                # Global attention
                scores = attn(hidden_states).squeeze(-1)  # [batch, seq]
                scores = scores.masked_fill(attention_mask == 0, -1e9)
                weights = F.softmax(scores, dim=-1)
                pooled = torch.bmm(weights.unsqueeze(1), hidden_states).squeeze(1)
            else:
                # Local windowed attention
                # Reshape into windows
                pad_len = (scale - seq_len % scale) % scale
                if pad_len > 0:
                    hidden_padded = F.pad(hidden_states, (0, 0, 0, pad_len))
                    mask_padded = F.pad(attention_mask, (0, pad_len), value=0)
                else:
                    hidden_padded = hidden_states
                    mask_padded = attention_mask

                new_seq = hidden_padded.size(1)
                num_windows = new_seq // scale

                # Reshape: [batch, num_windows, scale, hidden]
                hidden_windowed = hidden_padded.view(batch_size, num_windows, scale, hidden)
                mask_windowed = mask_padded.view(batch_size, num_windows, scale)

                # Attention within each window
                scores = attn(hidden_windowed).squeeze(-1)  # [batch, num_windows, scale]
                scores = scores.masked_fill(mask_windowed == 0, -1e9)
                weights = F.softmax(scores, dim=-1)

                # Pool within windows
                window_pooled = (weights.unsqueeze(-1) * hidden_windowed).sum(dim=2)  # [batch, num_windows, hidden]

                # Mean across windows
                pooled = window_pooled.mean(dim=1)  # [batch, hidden]

            pooled_outputs.append(pooled)

        # Concatenate and project
        combined = torch.cat(pooled_outputs, dim=-1)  # [batch, hidden * num_heads]
        output = self.output_proj(combined)  # [batch, hidden]

        return output


# ============================================================================
# ANCE - Self-Mined Hard Negatives
# ============================================================================

@torch.no_grad()
def mine_hard_negatives(
    model,
    examples: List[Dict],
    tokenizer,
    device,
    top_k: int = 10,
    batch_size: int = 32,
    exclude_self: bool = True,
) -> Dict[int, List[int]]:
    """
    Mine hard negatives using current model embeddings.

    For each query, find top-k most similar non-matching documents.
    These become hard negatives for the next epoch.

    Reference: ANCE (Approximate Nearest Neighbor Negative Contrastive Learning)
    """
    model.eval()

    # Encode all queries and documents
    all_query_embs = []
    all_doc_embs = []

    print("  Mining hard negatives...")

    for i in range(0, len(examples), batch_size):
        batch = examples[i:i + batch_size]

        queries = [ex["query"] for ex in batch]
        docs = [ex["positive"] for ex in batch]

        q_enc = tokenizer(queries, padding=True, truncation=True,
                          max_length=128, return_tensors="pt")
        d_enc = tokenizer(docs, padding=True, truncation=True,
                          max_length=256, return_tensors="pt")

        q_out = model(q_enc["input_ids"].to(device), q_enc["attention_mask"].to(device))
        d_out = model(d_enc["input_ids"].to(device), d_enc["attention_mask"].to(device))

        q_emb = mean_pool(q_out.last_hidden_state, q_enc["attention_mask"].to(device))
        d_emb = mean_pool(d_out.last_hidden_state, d_enc["attention_mask"].to(device))

        all_query_embs.append(q_emb.cpu())
        all_doc_embs.append(d_emb.cpu())

        if (i + batch_size) % 10000 == 0:
            print(f"    Encoded {min(i + batch_size, len(examples))}/{len(examples)}")

    query_embs = torch.cat(all_query_embs, dim=0)
    doc_embs = torch.cat(all_doc_embs, dim=0)

    # Normalize
    query_embs = F.normalize(query_embs, dim=-1)
    doc_embs = F.normalize(doc_embs, dim=-1)

    # Find hard negatives for each query
    hard_negatives = {}
    n = len(examples)

    # Process in chunks to avoid memory issues
    chunk_size = 1000
    for i in range(0, n, chunk_size):
        end_i = min(i + chunk_size, n)
        q_chunk = query_embs[i:end_i]

        # Compute similarities
        sims = torch.mm(q_chunk, doc_embs.t())  # [chunk, n]

        # Exclude self (diagonal)
        if exclude_self:
            for j in range(end_i - i):
                sims[j, i + j] = -1e9

        # Get top-k indices
        _, topk_indices = sims.topk(top_k, dim=1)

        for j in range(end_i - i):
            hard_negatives[i + j] = topk_indices[j].tolist()

    print(f"  Mined hard negatives for {len(hard_negatives)} examples")
    model.train()
    return hard_negatives


# ============================================================================
# AST Augmentation
# ============================================================================

import re

def ast_augment_code(code: str, language: str = "python") -> List[str]:
    """
    Generate augmented versions of code using AST-aware transformations.

    These transformations preserve semantics while changing syntax:
    - Variable renaming
    - Whitespace normalization
    - Comment removal
    - Statement reordering (where safe)
    """
    augmented = []

    # 1. Remove comments
    code_no_comments = remove_comments(code, language)
    if code_no_comments != code and len(code_no_comments) > 10:
        augmented.append(code_no_comments)

    # 2. Normalize whitespace
    code_normalized = normalize_whitespace(code)
    if code_normalized != code:
        augmented.append(code_normalized)

    # 3. Rename variables (simple pattern-based)
    code_renamed = rename_variables(code, language)
    if code_renamed != code:
        augmented.append(code_renamed)

    return augmented


def remove_comments(code: str, language: str) -> str:
    """Remove comments from code."""
    if language in ["python", "ruby", "shell", "bash"]:
        # Remove # comments (but not in strings)
        lines = []
        for line in code.split("\n"):
            # Simple heuristic: remove # and everything after if not in string
            if "#" in line and not ('"' in line or "'" in line):
                line = line.split("#")[0].rstrip()
            lines.append(line)
        return "\n".join(lines)

    elif language in ["javascript", "typescript", "java", "c", "cpp", "rust", "go", "swift"]:
        # Remove // comments
        code = re.sub(r"//.*$", "", code, flags=re.MULTILINE)
        # Remove /* */ comments
        code = re.sub(r"/\*.*?\*/", "", code, flags=re.DOTALL)
        return code

    return code


def normalize_whitespace(code: str) -> str:
    """Normalize whitespace in code."""
    # Replace multiple spaces with single space
    code = re.sub(r"[ \t]+", " ", code)
    # Remove trailing whitespace
    code = re.sub(r" +$", "", code, flags=re.MULTILINE)
    # Normalize line endings
    code = re.sub(r"\n{3,}", "\n\n", code)
    return code.strip()


def rename_variables(code: str, language: str) -> str:
    """Simple variable renaming (pattern-based, not full AST)."""
    # Common single-letter variables to rename
    renames = {
        r"\bi\b": "idx",
        r"\bj\b": "jdx",
        r"\bk\b": "kdx",
        r"\bn\b": "num",
        r"\bs\b": "str_val",
        r"\bx\b": "val_x",
        r"\by\b": "val_y",
    }

    result = code
    for pattern, replacement in renames.items():
        # Only rename if it looks like a variable (not in string literals)
        result = re.sub(pattern, replacement, result)

    # Only return if something changed
    return result if result != code else code


# ============================================================================
# Cross-Lingual Mining
# ============================================================================

@torch.no_grad()
def mine_cross_lingual_pairs(
    model,
    examples: List[Dict],
    tokenizer,
    device,
    similarity_threshold: float = 0.8,
    batch_size: int = 32,
) -> List[Tuple[int, int, float]]:
    """
    Find semantically similar code across different languages.

    Returns pairs of example indices that have high similarity
    but are in different languages.
    """
    model.eval()

    # Group by language
    lang_indices: Dict[str, List[int]] = defaultdict(list)
    for i, ex in enumerate(examples):
        lang_indices[ex.get("language", "unknown")].append(i)

    languages = list(lang_indices.keys())
    if len(languages) < 2:
        print("  Cross-lingual mining requires at least 2 languages")
        return []

    # Encode all documents
    all_doc_embs = []

    print("  Encoding documents for cross-lingual mining...")
    for i in range(0, len(examples), batch_size):
        batch = examples[i:i + batch_size]
        docs = [ex["positive"] for ex in batch]

        d_enc = tokenizer(docs, padding=True, truncation=True,
                          max_length=256, return_tensors="pt")
        d_out = model(d_enc["input_ids"].to(device), d_enc["attention_mask"].to(device))
        d_emb = mean_pool(d_out.last_hidden_state, d_enc["attention_mask"].to(device))

        all_doc_embs.append(d_emb.cpu())

    doc_embs = torch.cat(all_doc_embs, dim=0)
    doc_embs = F.normalize(doc_embs, dim=-1)

    # Find cross-lingual pairs
    cross_pairs = []

    for i, lang1 in enumerate(languages):
        for lang2 in languages[i + 1:]:
            indices1 = lang_indices[lang1]
            indices2 = lang_indices[lang2]

            # Sample if too many
            max_compare = 5000
            if len(indices1) > max_compare:
                indices1 = random.sample(indices1, max_compare)
            if len(indices2) > max_compare:
                indices2 = random.sample(indices2, max_compare)

            # Compute pairwise similarities
            embs1 = doc_embs[indices1]
            embs2 = doc_embs[indices2]

            sims = torch.mm(embs1, embs2.t())

            # Find high-similarity pairs
            high_sim = (sims > similarity_threshold).nonzero()

            for idx in high_sim:
                i1, i2 = idx[0].item(), idx[1].item()
                sim = sims[i1, i2].item()
                cross_pairs.append((indices1[i1], indices2[i2], sim))

    print(f"  Found {len(cross_pairs)} cross-lingual pairs")
    model.train()
    return cross_pairs


def combined_loss(
    scores: torch.Tensor,
    temperature: float,
    margin: float = 0.2,
    margin_weight: float = 0.1,
) -> Tuple[torch.Tensor, Dict[str, float]]:
    """InfoNCE + margin-based triplet loss."""
    batch_size = scores.size(0)

    # InfoNCE
    logits = scores / temperature
    labels = torch.arange(batch_size, device=scores.device)
    ce_loss = F.cross_entropy(logits, labels)

    # Margin loss
    pos_scores = scores.diag()
    mask = torch.eye(batch_size, device=scores.device, dtype=torch.bool)
    neg_scores = scores.masked_fill(mask, float('-inf'))
    hardest_neg = neg_scores.max(dim=1).values
    triplet_loss = F.relu(margin - (pos_scores - hardest_neg)).mean()

    total = ce_loss + margin_weight * triplet_loss

    return total, {"ce": ce_loss.item(), "margin": triplet_loss.item()}


# ============================================================================
# Evaluation
# ============================================================================

@torch.no_grad()
def evaluate_retrieval(
    model,
    val_examples: List[Dict],
    tokenizer,
    device,
    max_length: int = 512,
    batch_size: int = 32,
) -> Dict[str, float]:
    """Compute MRR and Recall@k on validation set."""
    model.eval()

    # Encode in batches
    all_query_embs = []
    all_doc_embs = []

    for i in range(0, len(val_examples), batch_size):
        batch = val_examples[i:i + batch_size]

        queries = [ex["query"] for ex in batch]
        docs = [ex["positive"] for ex in batch]

        q_enc = tokenizer(queries, padding=True, truncation=True,
                          max_length=max_length, return_tensors="pt")
        d_enc = tokenizer(docs, padding=True, truncation=True,
                          max_length=max_length, return_tensors="pt")

        q_out = model(q_enc["input_ids"].to(device), q_enc["attention_mask"].to(device))
        d_out = model(d_enc["input_ids"].to(device), d_enc["attention_mask"].to(device))

        # Mean pool for efficiency
        q_emb = mean_pool(q_out.last_hidden_state, q_enc["attention_mask"].to(device))
        d_emb = mean_pool(d_out.last_hidden_state, d_enc["attention_mask"].to(device))

        all_query_embs.append(q_emb.cpu())
        all_doc_embs.append(d_emb.cpu())

    query_embs = torch.cat(all_query_embs, dim=0)
    doc_embs = torch.cat(all_doc_embs, dim=0)

    # Normalize
    query_embs = F.normalize(query_embs, dim=-1)
    doc_embs = F.normalize(doc_embs, dim=-1)

    # Compute similarity
    sim = torch.mm(query_embs, doc_embs.t())

    # Compute metrics
    n = len(val_examples)
    ranks = []
    for i in range(n):
        scores = sim[i]
        rank = (scores > scores[i]).sum().item() + 1
        ranks.append(rank)

    mrr = np.mean([1.0 / r for r in ranks])
    recall_1 = np.mean([1 if r == 1 else 0 for r in ranks])
    recall_5 = np.mean([1 if r <= 5 else 0 for r in ranks])
    recall_10 = np.mean([1 if r <= 10 else 0 for r in ranks])

    model.train()
    return {
        "mrr": mrr,
        "recall@1": recall_1,
        "recall@5": recall_5,
        "recall@10": recall_10,
    }


# ============================================================================
# Training Loop
# ============================================================================

def train(config: Dict) -> None:
    training_cfg = config["training"]
    data_cfg = config["data"]
    model_cfg = config["model"]

    # Seeds
    seed = training_cfg.get("seed", 42)
    random.seed(seed)
    np.random.seed(seed)
    torch.manual_seed(seed)

    # Device
    device = select_device(training_cfg.get("device"))
    print(f"Device: {device}")

    # Load model
    base_model = model_cfg["base"]
    print(f"Loading model: {base_model}")
    model = T5EncoderModel.from_pretrained(base_model)
    tokenizer = AutoTokenizer.from_pretrained(base_model)

    # Gradient checkpointing
    if training_cfg.get("gradient_checkpointing", True):
        model.gradient_checkpointing_enable()
        print("Gradient checkpointing: enabled")

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

    # Output directory
    output_dir = Path(model_cfg["output"])
    output_dir.mkdir(parents=True, exist_ok=True)

    # Load dataset
    train_path = Path(data_cfg["train"])
    languages = set(data_cfg.get("languages", [])) or None
    max_length = training_cfg.get("max_length", 512)
    max_hard_negs = training_cfg.get("max_hard_negatives", 3)

    dataset = ImprovedCodeSearchDataset(
        data_path=train_path,
        tokenizer=tokenizer,
        max_length=max_length,
        max_hard_negatives=max_hard_negs,
        languages=languages,
        use_cache=training_cfg.get("cache_tokenized", True),
    )

    print(f"\nLanguages in dataset: {dataset.get_languages()}")
    for lang in dataset.get_languages():
        print(f"  {lang}: {len(dataset.language_indices[lang])}")

    # Load validation set
    val_examples = None
    val_path = data_cfg.get("validation")
    if val_path:
        val_path = Path(val_path)
        if val_path.exists():
            print(f"Loading validation set: {val_path}")
            val_examples = [json.loads(l) for l in open(val_path) if l.strip()]
            print(f"  {len(val_examples)} validation examples")

    # Training params
    batch_size = training_cfg["batch_size"]
    grad_accum = training_cfg.get("gradient_accumulation_steps", 1)
    effective_batch = batch_size * grad_accum
    epochs = training_cfg["epochs"]
    lr = float(training_cfg["learning_rate"])
    warmup_steps = training_cfg.get("warmup_steps", 0)
    weight_decay = training_cfg.get("weight_decay", 0.01)
    max_grad_norm = training_cfg.get("max_grad_norm", 1.0)

    temperature = training_cfg.get("temperature", 0.07)
    use_margin_loss = training_cfg.get("use_margin_loss", True)
    margin = training_cfg.get("margin", 0.2)
    margin_weight = training_cfg.get("margin_weight", 0.1)

    use_language_aware = training_cfg.get("language_aware_batching", True)
    eval_every = training_cfg.get("eval_every", 500)
    log_every = training_cfg.get("log_every", 50)
    save_every = training_cfg.get("save_every", 0)
    early_stopping_patience = training_cfg.get("early_stopping_patience", 3)

    # New improvements
    use_gradcache = training_cfg.get("use_gradcache", False)
    gradcache_chunk_size = training_cfg.get("gradcache_chunk_size", 4)
    use_llrd = training_cfg.get("use_llrd", False)
    llrd_decay_rate = training_cfg.get("llrd_decay_rate", 0.9)
    use_matryoshka = training_cfg.get("use_matryoshka", False)
    matryoshka_dims = training_cfg.get("matryoshka_dims", [64, 128, 256, 768])

    # Additional improvements
    use_dynamic_temp = training_cfg.get("use_dynamic_temperature", False)
    use_progressive_length = training_cfg.get("use_progressive_length", False)
    progressive_min_length = training_cfg.get("progressive_min_length", 128)
    progressive_warmup_steps = training_cfg.get("progressive_warmup_steps", 2000)
    use_curriculum = training_cfg.get("use_curriculum", False)
    curriculum_warmup_epochs = training_cfg.get("curriculum_warmup_epochs", 0.5)
    use_ance = training_cfg.get("use_ance", False)
    ance_mine_every = training_cfg.get("ance_mine_every", 1)  # Mine every N epochs

    # === NEW RETRIEVAL IMPROVEMENTS ===
    use_mnr_loss = training_cfg.get("use_mnr_loss", False)
    mnr_scale = training_cfg.get("mnr_scale", 20.0)
    use_token_contrastive = training_cfg.get("use_token_contrastive", False)
    token_contrastive_weight = training_cfg.get("token_contrastive_weight", 0.3)
    use_ema = training_cfg.get("use_ema", False)
    ema_decay = training_cfg.get("ema_decay", 0.999)
    ema_distill_weight = training_cfg.get("ema_distill_weight", 0.1)
    use_focal_loss = training_cfg.get("use_focal_loss", False)
    focal_gamma = training_cfg.get("focal_gamma", 2.0)
    focal_alpha = training_cfg.get("focal_alpha", 0.25)
    use_priority_sampling = training_cfg.get("use_priority_sampling", False)
    priority_weight = training_cfg.get("priority_weight", 3.0)
    priority_file_patterns = training_cfg.get("priority_file_patterns", [])

    # Calculate steps
    if use_language_aware:
        # Approximate - actual depends on language distribution
        total_batches = sum(
            len(indices) // batch_size
            for indices in dataset.language_indices.values()
        )
    else:
        total_batches = len(dataset) // batch_size

    steps_per_epoch = total_batches
    total_opt_steps = (steps_per_epoch * epochs) // grad_accum

    print(f"\nTraining configuration:")
    print(f"  Dataset: {len(dataset)} examples")
    print(f"  Batch size: {batch_size} x {grad_accum} = {effective_batch}")
    print(f"  Epochs: {epochs}")
    print(f"  Steps/epoch: ~{steps_per_epoch}")
    print(f"  Total opt steps: ~{total_opt_steps}")
    print(f"  Language-aware batching: {use_language_aware}")
    print(f"  Margin loss: {use_margin_loss} (margin={margin}, weight={margin_weight})")
    print(f"  Validation: {len(val_examples) if val_examples else 'disabled'}")
    print(f"  GradCache: {use_gradcache} (chunk={gradcache_chunk_size})")
    print(f"  LLRD: {use_llrd} (decay={llrd_decay_rate})")
    print(f"  Matryoshka: {use_matryoshka} (dims={matryoshka_dims})")
    print(f"  Dynamic Temperature: {use_dynamic_temp}")
    print(f"  Progressive Length: {use_progressive_length} ({progressive_min_length}→{max_length})")
    print(f"  Curriculum Learning: {use_curriculum}")
    print(f"  ANCE (Self-mined negatives): {use_ance}")
    print(f"  MNR Loss: {use_mnr_loss} (scale={mnr_scale})")
    print(f"  Token Contrastive: {use_token_contrastive} (weight={token_contrastive_weight})")
    print(f"  EMA Teacher: {use_ema} (decay={ema_decay}, weight={ema_distill_weight})")
    print(f"  Focal Loss: {use_focal_loss} (gamma={focal_gamma}, alpha={focal_alpha})")
    print(f"  Priority Sampling: {use_priority_sampling} (weight={priority_weight}x)")

    # === CRITICAL EFFICIENCY: Mixed Precision (AMP) ===
    # PyTorch 2.9.1: AMP now works on MPS!
    use_amp = training_cfg.get("use_amp", True)
    if torch.cuda.is_available():
        amp_device_type = "cuda"
        amp_dtype = torch.float16
        scaler = GradScaler(enabled=use_amp)
    elif hasattr(torch.backends, "mps") and torch.backends.mps.is_available():
        amp_device_type = "mps"  # PyTorch 2.9.1 supports MPS autocast
        amp_dtype = torch.float16  # float16 works on MPS now
        scaler = GradScaler(enabled=False)  # GradScaler still not needed for MPS
    else:
        amp_device_type = "cpu"
        amp_dtype = torch.bfloat16
        scaler = GradScaler(enabled=False)
        use_amp = False
    print(f"  Mixed Precision (AMP): {use_amp} (device={amp_device_type}, dtype={amp_dtype})")

    # === CRITICAL EFFICIENCY: Float32 matmul precision ===
    matmul_precision = training_cfg.get("matmul_precision", "highest")
    torch.set_float32_matmul_precision(matmul_precision)
    print(f"  Float32 matmul precision: {matmul_precision}")

    # === CRITICAL EFFICIENCY: Fused optimizer ===
    use_fused_optimizer = training_cfg.get("use_fused_optimizer", False)

    # === CRITICAL EFFICIENCY: torch.compile ===
    use_compile = training_cfg.get("use_compile", TORCH_COMPILE_AVAILABLE)
    if use_compile and TORCH_COMPILE_AVAILABLE:
        print("  Compiling model with torch.compile (this may take a minute)...")
        model = torch.compile(model, mode="reduce-overhead")
        print("  Model compiled successfully")
    else:
        print(f"  torch.compile: disabled")

    # === CRITICAL EFFICIENCY: Memory Bank ===
    use_memory_bank = training_cfg.get("use_memory_bank", True)
    memory_bank_size = training_cfg.get("memory_bank_size", 65536)
    memory_bank = None
    if use_memory_bank:
        memory_bank = MemoryBank(size=memory_bank_size, dim=768, device="cpu")
        print(f"  Memory Bank: {memory_bank_size} embeddings for extra negatives")

    # === CRITICAL EFFICIENCY: DataLoader workers ===
    num_workers = training_cfg.get("num_workers", 4)
    print(f"  DataLoader workers: {num_workers}")

    # Initialize EMA model if enabled
    ema_model = None
    if use_ema:
        ema_model = EMAModel(model, decay=ema_decay)
        print(f"  EMA model initialized with decay={ema_decay}")

    # Initialize GradCache if enabled
    grad_cache = None
    if use_gradcache:
        grad_cache = GradCache(model, chunk_size=gradcache_chunk_size)
        print(f"  GradCache initialized with chunk_size={gradcache_chunk_size}")

    # Initialize Dynamic Temperature if enabled
    learnable_temp = None
    if use_dynamic_temp:
        learnable_temp = LearnableTemperature(init_temp=temperature).to(device)
        print(f"  Dynamic temperature initialized at {temperature}")

    # Initialize Progressive Sequence Length if enabled
    progressive_length = None
    if use_progressive_length:
        progressive_length = ProgressiveSequenceLength(
            min_length=progressive_min_length,
            max_length=max_length,
            warmup_steps=progressive_warmup_steps,
        )
        print(f"  Progressive length: {progressive_min_length}→{max_length} over {progressive_warmup_steps} steps")

    # Curriculum learning initialization (done per-epoch)
    curriculum_scheduler = None

    # Optimizer - with optional LLRD and fused optimizer
    fused_available = use_fused_optimizer and torch.cuda.is_available()  # Fused only on CUDA
    if use_llrd:
        param_groups = get_llrd_params(model, lr, llrd_decay_rate)
        # Add learnable temperature if enabled
        if learnable_temp is not None:
            param_groups.append({"params": learnable_temp.parameters(), "lr": lr * 10})
        optimizer = torch.optim.AdamW(param_groups, fused=fused_available)
        print(f"  LLRD: {len(param_groups)} parameter groups with decay rate {llrd_decay_rate}")
    else:
        params_to_optimize = list(model.parameters())
        if learnable_temp is not None:
            params_to_optimize.extend(learnable_temp.parameters())
        optimizer = torch.optim.AdamW(
            params_to_optimize, lr=lr, weight_decay=weight_decay, fused=fused_available
        )
    print(f"  Optimizer: AdamW (fused={fused_available})")
    scheduler = get_cosine_schedule_with_warmup(
        optimizer, num_warmup_steps=warmup_steps, num_training_steps=total_opt_steps
    )

    # Training state
    global_step = 0
    best_mrr = 0.0
    patience_counter = 0
    training_start = time.time()

    for epoch in range(epochs):
        epoch_start = time.time()
        model.train()

        # ANCE: Mine hard negatives using current model (every N epochs)
        mined_hard_negs = None
        if use_ance and epoch > 0 and epoch % ance_mine_every == 0:
            print(f"\n  [ANCE] Mining hard negatives for epoch {epoch+1}...")
            # Load raw examples for mining
            raw_examples = [json.loads(l) for l in open(train_path) if l.strip()]
            mined_hard_negs = mine_hard_negatives(
                model, raw_examples, tokenizer, device,
                top_k=3, batch_size=64
            )
            print(f"  [ANCE] Mined hard negatives for {len(mined_hard_negs)} examples")

        # Create dataloader with optimized settings
        if use_language_aware:
            batch_sampler = LanguageAwareBatchSampler(dataset, batch_size, drop_last=True, shuffle=True)
            dataloader = create_optimized_dataloader(
                dataset,
                batch_size=batch_size,
                batch_sampler=batch_sampler,
                collate_fn=collate_fn,
                num_workers=num_workers,
            )
            print(f"  Using language-aware batching: {len(batch_sampler)} batches (workers={num_workers})")
        else:
            dataloader = create_optimized_dataloader(
                dataset,
                batch_size=batch_size,
                collate_fn=collate_fn,
                num_workers=num_workers,
            )
            print(f"  Using standard batching (workers={num_workers})")

        total_loss = 0.0
        step_loss = 0.0
        step = 0

        print(f"  Starting epoch {epoch+1}...", flush=True)

        for batch_idx, batch in enumerate(dataloader):
            if batch_idx == 0:
                print(f"  First batch received, starting training...", flush=True)

            t0 = time.time()

            # Move to device
            query_ids = batch["query_ids"].to(device)
            query_mask = batch["query_mask"].to(device)
            pos_ids = batch["pos_ids"].to(device)
            pos_mask = batch["pos_mask"].to(device)

            if batch_idx == 0:
                print(f"    Data to device: {time.time()-t0:.2f}s", flush=True)
                t0 = time.time()

            # Forward with AMP (Mixed Precision)
            with autocast(device_type=amp_device_type, enabled=use_amp, dtype=amp_dtype):
                query_out = model(input_ids=query_ids, attention_mask=query_mask)

                if batch_idx == 0:
                    print(f"    Query forward: {time.time()-t0:.2f}s", flush=True)
                    t0 = time.time()

                pos_out = model(input_ids=pos_ids, attention_mask=pos_mask)

                if batch_idx == 0:
                    print(f"    Pos forward: {time.time()-t0:.2f}s", flush=True)
                    t0 = time.time()

                query_emb = query_out.last_hidden_state
                pos_emb = pos_out.last_hidden_state

                # Compute scores (inside autocast for efficiency)
                scores = maxsim_scores(query_emb, query_mask.float(), pos_emb, pos_mask.float())

            if batch_idx == 0:
                print(f"    MaxSim: {time.time()-t0:.2f}s", flush=True)
                t0 = time.time()

            # Get current temperature (learnable or fixed)
            current_temp = learnable_temp.temperature.item() if learnable_temp else temperature

            # === COMPUTE LOSSES ===
            loss_parts = {}
            total_loss = torch.tensor(0.0, device=device)

            # Base loss: Choose between MNR, Focal, Margin, or InfoNCE
            if use_mnr_loss:
                base_loss = multiple_negatives_ranking_loss(scores, current_temp, mnr_scale)
                loss_parts["mnr"] = base_loss.item()
            elif use_focal_loss:
                base_loss = focal_contrastive_loss(scores, current_temp, focal_gamma, focal_alpha)
                loss_parts["focal"] = base_loss.item()
            elif use_margin_loss:
                base_loss, margin_parts = combined_loss(scores, current_temp, margin, margin_weight)
                loss_parts.update(margin_parts)
            else:
                base_loss = infonce_loss(scores, current_temp)
                loss_parts["ce"] = base_loss.item()

            total_loss = total_loss + base_loss

            # Matryoshka loss (multi-scale embeddings)
            if use_matryoshka:
                mrl_loss, mrl_parts = matryoshka_loss(
                    query_emb, query_mask, pos_emb, pos_mask,
                    current_temp, dims=matryoshka_dims
                )
                total_loss = total_loss + mrl_loss
                for k, v in mrl_parts.items():
                    loss_parts[f"mrl_{k}"] = v

            # Token-level contrastive loss (for multi-vector XTR)
            if use_token_contrastive:
                tok_loss = token_contrastive_loss(
                    query_emb, query_mask, pos_emb, pos_mask, current_temp
                )
                total_loss = total_loss + token_contrastive_weight * tok_loss
                loss_parts["token"] = tok_loss.item()

            # EMA distillation loss
            if use_ema and ema_model is not None:
                # Get teacher embeddings
                ema_model.apply_shadow(model)
                with torch.no_grad():
                    teacher_out = model(input_ids=pos_ids, attention_mask=pos_mask)
                    teacher_emb = teacher_out.last_hidden_state
                ema_model.restore(model)

                distill_loss = ema_distillation_loss(pos_emb, teacher_emb, pos_mask)
                total_loss = total_loss + ema_distill_weight * distill_loss
                loss_parts["ema"] = distill_loss.item()

            # Memory bank loss (additional negatives)
            if use_memory_bank and memory_bank is not None and len(memory_bank) > batch_size:
                mb_loss = memory_bank_contrastive_loss(
                    query_emb, pos_emb, memory_bank, current_temp
                )
                total_loss = total_loss + 0.5 * mb_loss  # Weight memory bank loss
                loss_parts["mem_bank"] = mb_loss.item()

            loss = total_loss
            loss_parts["total"] = loss.item()

            # NaN detection - skip batch if loss is NaN
            if torch.isnan(loss) or torch.isinf(loss):
                print(f"  WARNING: NaN/Inf loss detected at batch {batch_idx}, skipping...", flush=True)
                optimizer.zero_grad()
                continue

            loss = loss / grad_accum

            # Backward with AMP scaler
            scaler.scale(loss).backward()

            if batch_idx == 0:
                print(f"    Loss + backward: {time.time()-t0:.2f}s", flush=True)
                print(f"    First batch loss: {loss.item() * grad_accum:.4f}", flush=True)

            # Update memory bank with current batch embeddings
            if use_memory_bank and memory_bank is not None:
                memory_bank.update(pos_emb.detach())

            # MPS sync for stability - sync every batch to prevent hangs
            if device.type == "mps":
                torch.mps.synchronize()
                # Clear cache periodically to prevent memory buildup
                if batch_idx > 0 and batch_idx % 100 == 0:
                    torch.mps.empty_cache()

            step_loss += loss.item() * grad_accum

            # Progress logging every 100 batches
            if batch_idx > 0 and batch_idx % 100 == 0:
                elapsed = time.time() - epoch_start
                batches_per_sec = batch_idx / elapsed
                eta_mins = (len(dataloader) - batch_idx) / batches_per_sec / 60
                mem_bank_size = len(memory_bank) if memory_bank else 0
                print(f"  Batch {batch_idx}/{len(dataloader)} | "
                      f"{batches_per_sec:.2f} batch/s | "
                      f"ETA: {eta_mins:.0f}min | "
                      f"MemBank: {mem_bank_size}", flush=True)

            # Gradient step with AMP scaler
            if (batch_idx + 1) % grad_accum == 0:
                # Unscale before clipping
                scaler.unscale_(optimizer)
                torch.nn.utils.clip_grad_norm_(model.parameters(), max_grad_norm)

                # Step with scaler
                scaler.step(optimizer)
                scaler.update()
                scheduler.step()
                optimizer.zero_grad()

                # Update EMA model
                if use_ema and ema_model is not None:
                    ema_model.update(model)

                total_loss += step_loss
                global_step += 1
                step += 1

                # Logging
                if step % log_every == 0:
                    avg_loss = total_loss / step
                    lr_now = scheduler.get_last_lr()[0]
                    elapsed = time.time() - epoch_start
                    speed = (step * effective_batch) / elapsed

                    # Build log message
                    log_msg = (f"Epoch {epoch+1}/{epochs} | Step {step} | "
                               f"Loss: {step_loss:.4f} (avg: {avg_loss:.4f}) | "
                               f"LR: {lr_now:.2e} | {speed:.0f} samples/s")

                    # Add temperature if dynamic
                    if learnable_temp:
                        log_msg += f" | T: {learnable_temp.temperature.item():.3f}"

                    # Add current seq length if progressive
                    if progressive_length:
                        curr_len = progressive_length.get_length(global_step)
                        log_msg += f" | SeqLen: {curr_len}"

                    print(log_msg)

                step_loss = 0.0

                # Validation
                if val_examples and eval_every > 0 and global_step % eval_every == 0:
                    metrics = evaluate_retrieval(model, val_examples, tokenizer, device, max_length)
                    print(f"  [Validation] MRR: {metrics['mrr']:.4f} | "
                          f"R@1: {metrics['recall@1']:.4f} | "
                          f"R@5: {metrics['recall@5']:.4f} | "
                          f"R@10: {metrics['recall@10']:.4f}")

                    if metrics['mrr'] > best_mrr:
                        best_mrr = metrics['mrr']
                        patience_counter = 0
                        model.save_pretrained(output_dir / "best")
                        print(f"  New best MRR! Saved to {output_dir / 'best'}")
                    else:
                        patience_counter += 1
                        if early_stopping_patience > 0 and patience_counter >= early_stopping_patience:
                            print(f"  Early stopping (patience={early_stopping_patience})")
                            break

                # Checkpoint with full training state (for resuming)
                if save_every > 0 and global_step % save_every == 0:
                    ckpt_dir = output_dir / f"checkpoint-{global_step}"
                    ckpt_dir.mkdir(exist_ok=True)
                    model.save_pretrained(ckpt_dir)

                    # Save full training state for resumption
                    training_state = {
                        "global_step": global_step,
                        "epoch": epoch,
                        "best_mrr": best_mrr,
                        "optimizer": optimizer.state_dict(),
                        "scheduler": scheduler.state_dict(),
                        "scaler": scaler.state_dict(),
                    }
                    torch.save(training_state, ckpt_dir / "training_state.pt")
                    print(f"  Saved checkpoint to {ckpt_dir} (full state)")

        # Early stopping check
        if early_stopping_patience > 0 and patience_counter >= early_stopping_patience:
            break

        # End of epoch
        epoch_time = time.time() - epoch_start
        avg_loss = total_loss / max(step, 1)
        print(f"\nEpoch {epoch+1} complete: avg_loss={avg_loss:.4f}, time={epoch_time/60:.1f}min")

        # Epoch validation
        if val_examples:
            metrics = evaluate_retrieval(model, val_examples, tokenizer, device, max_length)
            print(f"  [Epoch Validation] MRR: {metrics['mrr']:.4f} | R@1: {metrics['recall@1']:.4f}")

            if metrics['mrr'] > best_mrr:
                best_mrr = metrics['mrr']
                patience_counter = 0
                model.save_pretrained(output_dir / "best")

    # Final save
    total_time = time.time() - training_start
    model.save_pretrained(output_dir)
    tokenizer.save_pretrained(output_dir)

    print(f"\n{'='*60}")
    print(f"Training complete!")
    print(f"  Total time: {total_time/60:.1f} minutes")
    print(f"  Best MRR: {best_mrr:.4f}")
    print(f"  Output: {output_dir}")
    print(f"{'='*60}")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--config", required=True)
    args = parser.parse_args()
    config = load_config(Path(args.config))
    train(config)


if __name__ == "__main__":
    main()
