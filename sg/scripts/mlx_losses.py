#!/usr/bin/env python3
"""MLX implementations of loss functions for XTR training.

These losses are optimized for Metal acceleration.
"""

import mlx.core as mx
import mlx.nn as nn


def maxsim(
    query_emb: mx.array,
    doc_emb: mx.array,
    query_mask: mx.array,
    doc_mask: mx.array,
) -> mx.array:
    """Compute MaxSim scores between query and document embeddings.

    MaxSim is the core scoring function for ColBERT-style retrieval:
    - For each query token, find max similarity to any doc token
    - Average across all query tokens

    Args:
        query_emb: Query embeddings [B, Q, D]
        doc_emb: Document embeddings [B, K, D]
        query_mask: Query attention mask [B, Q] (1=valid, 0=pad)
        doc_mask: Document attention mask [B, K] (1=valid, 0=pad)

    Returns:
        Similarity matrix [B, B] where [i, j] = similarity(query_i, doc_j)
    """
    # L2 normalize embeddings
    query_emb = query_emb / (mx.linalg.norm(query_emb, axis=-1, keepdims=True) + 1e-9)
    doc_emb = doc_emb / (mx.linalg.norm(doc_emb, axis=-1, keepdims=True) + 1e-9)

    # Compute all pairwise similarities: [B_q, Q, B_d, K]
    # Using einsum for batched dot product
    sims = mx.einsum("iqd,jkd->iqjk", query_emb, doc_emb)

    # Mask out padding tokens in documents
    # doc_mask [B, K] -> [1, 1, B, K]
    doc_mask_expanded = doc_mask[None, None, :, :]
    sims = mx.where(doc_mask_expanded > 0, sims, mx.array(-1e9))

    # Max over document tokens: [B_q, Q, B_d]
    max_sims = mx.max(sims, axis=-1)

    # Mask query tokens and compute mean
    # query_mask [B, Q] -> [B, Q, 1]
    query_mask_expanded = query_mask[:, :, None]
    max_sims = mx.where(query_mask_expanded > 0, max_sims, mx.array(0.0))

    # Mean over query tokens
    query_lengths = mx.sum(query_mask, axis=1, keepdims=True)  # [B, 1]
    scores = mx.sum(max_sims, axis=1) / (query_lengths + 1e-9)  # [B, B_d]

    return scores


def contrastive_loss(
    query_emb: mx.array,
    doc_emb: mx.array,
    query_mask: mx.array,
    doc_mask: mx.array,
    temperature: float = 0.05,
) -> mx.array:
    """InfoNCE contrastive loss with in-batch negatives.

    For each query, the positive is the corresponding document (same index).
    All other documents in the batch are negatives.

    Args:
        query_emb: Query embeddings [B, Q, D]
        doc_emb: Document embeddings [B, K, D]
        query_mask: Query attention mask [B, Q]
        doc_mask: Document attention mask [B, K]
        temperature: Softmax temperature (lower = sharper)

    Returns:
        Scalar loss value
    """
    # Compute MaxSim scores: [B, B]
    scores = maxsim(query_emb, doc_emb, query_mask, doc_mask)

    # Scale by temperature
    scores = scores / temperature

    # Labels: diagonal (each query matches its own document)
    batch_size = query_emb.shape[0]
    labels = mx.arange(batch_size)

    # Cross-entropy loss
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
    """Triplet margin loss for contrastive learning.

    For each query, ensures positive doc score > negative doc scores + margin.

    Args:
        query_emb: Query embeddings [B, Q, D]
        doc_emb: Document embeddings [B, K, D]
        query_mask: Query attention mask [B, Q]
        doc_mask: Document attention mask [B, K]
        margin: Minimum score difference

    Returns:
        Scalar loss value
    """
    # Compute MaxSim scores: [B, B]
    scores = maxsim(query_emb, doc_emb, query_mask, doc_mask)

    batch_size = query_emb.shape[0]

    # Positive scores (diagonal)
    pos_scores = mx.diag(scores)  # [B]

    # Create mask to exclude diagonal (positive samples)
    mask = 1.0 - mx.eye(batch_size)

    # Negative scores (off-diagonal)
    neg_scores = scores * mask + mx.eye(batch_size) * (-1e9)

    # Hardest negative per query
    hard_neg_scores = mx.max(neg_scores, axis=-1)  # [B]

    # Margin loss: max(0, margin - (pos - neg))
    losses = mx.maximum(mx.array(0.0), margin - (pos_scores - hard_neg_scores))

    return mx.mean(losses)


def combined_loss(
    query_emb: mx.array,
    doc_emb: mx.array,
    query_mask: mx.array,
    doc_mask: mx.array,
    temperature: float = 0.05,
    margin: float = 0.3,
    margin_weight: float = 0.1,
) -> tuple[mx.array, dict]:
    """Combined InfoNCE + margin loss.

    Args:
        query_emb: Query embeddings [B, Q, D]
        doc_emb: Document embeddings [B, K, D]
        query_mask: Query attention mask [B, Q]
        doc_mask: Document attention mask [B, K]
        temperature: InfoNCE temperature
        margin: Margin for triplet loss
        margin_weight: Weight for margin loss component

    Returns:
        Tuple of (total_loss, loss_components_dict)
    """
    infonce = contrastive_loss(
        query_emb, doc_emb, query_mask, doc_mask, temperature
    )
    margin_l = margin_loss(query_emb, doc_emb, query_mask, doc_mask, margin)

    total = infonce + margin_weight * margin_l

    return total, {"infonce": infonce, "margin": margin_l, "total": total}


def test_losses():
    """Test loss functions with random data."""
    print("Testing MLX loss functions...")

    batch_size = 8
    query_len = 32
    doc_len = 64
    hidden_dim = 768

    # Create random embeddings
    query_emb = mx.random.normal((batch_size, query_len, hidden_dim))
    doc_emb = mx.random.normal((batch_size, doc_len, hidden_dim))

    # Create masks (all valid for simplicity)
    query_mask = mx.ones((batch_size, query_len))
    doc_mask = mx.ones((batch_size, doc_len))

    # Test MaxSim
    scores = maxsim(query_emb, doc_emb, query_mask, doc_mask)
    mx.eval(scores)
    print(f"MaxSim scores shape: {scores.shape}")
    print(f"MaxSim scores range: [{float(mx.min(scores)):.3f}, {float(mx.max(scores)):.3f}]")

    # Test contrastive loss
    loss = contrastive_loss(query_emb, doc_emb, query_mask, doc_mask)
    mx.eval(loss)
    print(f"Contrastive loss: {float(loss):.4f}")

    # Test margin loss
    loss = margin_loss(query_emb, doc_emb, query_mask, doc_mask)
    mx.eval(loss)
    print(f"Margin loss: {float(loss):.4f}")

    # Test combined loss
    total, components = combined_loss(query_emb, doc_emb, query_mask, doc_mask)
    mx.eval(total)
    print(f"Combined loss: {float(total):.4f}")

    # Test gradient computation
    print("\nTesting gradients...")

    def loss_fn(q_emb, d_emb):
        return contrastive_loss(q_emb, d_emb, query_mask, doc_mask)

    grad_fn = mx.grad(loss_fn, argnums=(0, 1))
    q_grad, d_grad = grad_fn(query_emb, doc_emb)
    mx.eval(q_grad, d_grad)
    print(f"Query gradient shape: {q_grad.shape}")
    print(f"Doc gradient shape: {d_grad.shape}")
    print(f"Query gradient norm: {float(mx.linalg.norm(q_grad.reshape(-1))):.4f}")
    print(f"Doc gradient norm: {float(mx.linalg.norm(d_grad.reshape(-1))):.4f}")

    print("\nAll tests passed!")


if __name__ == "__main__":
    test_losses()
