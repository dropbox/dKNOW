#!/usr/bin/env python3
"""Benchmark MLX vs PyTorch MPS training speed."""

import time
import sys

def benchmark_mlx():
    """Benchmark MLX training speed."""
    import mlx.core as mx
    import mlx.nn as nn
    import mlx.optimizers as optim
    
    sys.path.insert(0, '/Users/ayates/sg/scripts')
    from train_xtr_mlx import T5Encoder, maxsim, contrastive_loss, margin_loss
    
    print("Loading MLX model...")
    model = T5Encoder.from_pretrained(
        'google/xtr-base-en',
        use_lora=True,
        lora_r=16,
        lora_alpha=32,
        dtype=mx.float32,
    )
    
    # Create dummy data
    batch_size = 8
    seq_len = 256
    query_ids = mx.random.randint(0, 32128, (batch_size, seq_len))
    query_mask = mx.ones((batch_size, seq_len), dtype=mx.float32)
    doc_ids = mx.random.randint(0, 32128, (batch_size, seq_len))
    doc_mask = mx.ones((batch_size, seq_len), dtype=mx.float32)
    
    optimizer = optim.AdamW(learning_rate=2e-5)
    
    def loss_fn(model, batch):
        query_emb = model(batch["query_ids"], batch["query_mask"])
        doc_emb = model(batch["pos_ids"], batch["pos_mask"])
        loss_c = contrastive_loss(query_emb, doc_emb, batch["query_mask"], batch["pos_mask"], 0.07)
        loss_m = margin_loss(query_emb, doc_emb, batch["query_mask"], batch["pos_mask"], 0.3)
        return loss_c + 0.1 * loss_m
    
    loss_and_grad = nn.value_and_grad(model, loss_fn)
    
    batch = {
        "query_ids": query_ids,
        "query_mask": query_mask,
        "pos_ids": doc_ids,
        "pos_mask": doc_mask,
    }
    
    # Warmup
    print("MLX warmup...")
    for _ in range(3):
        loss, grads = loss_and_grad(model, batch)
        lora_grads = {k: v for k, v in grads.items() if "lora_A" in k or "lora_B" in k}
        optimizer.update(model, lora_grads)
        mx.eval(loss, model.parameters())
    
    # Benchmark
    print("MLX benchmark...")
    n_steps = 20
    start = time.time()
    for _ in range(n_steps):
        loss, grads = loss_and_grad(model, batch)
        lora_grads = {k: v for k, v in grads.items() if "lora_A" in k or "lora_B" in k}
        optimizer.update(model, lora_grads)
        mx.eval(loss, model.parameters())
    elapsed = time.time() - start
    
    mlx_samples_per_sec = n_steps * batch_size / elapsed
    mlx_ms_per_step = elapsed / n_steps * 1000
    print(f"MLX: {mlx_samples_per_sec:.1f} samples/s, {mlx_ms_per_step:.1f} ms/step")
    
    return mlx_samples_per_sec, mlx_ms_per_step


def benchmark_pytorch():
    """Benchmark PyTorch MPS training speed."""
    import torch
    import torch.nn.functional as F
    from transformers import T5EncoderModel, AutoTokenizer
    from peft import LoraConfig, get_peft_model
    
    device = torch.device("mps") if torch.backends.mps.is_available() else torch.device("cpu")
    print(f"Using device: {device}")
    
    print("Loading PyTorch model...")
    model = T5EncoderModel.from_pretrained("google/xtr-base-en")
    
    lora_config = LoraConfig(
        r=16,
        lora_alpha=32,
        lora_dropout=0.0,
        target_modules=["q", "v"],
    )
    model = get_peft_model(model, lora_config)
    model.to(device)
    
    # Create dummy data
    batch_size = 8
    seq_len = 256
    query_ids = torch.randint(0, 32128, (batch_size, seq_len), device=device)
    query_mask = torch.ones((batch_size, seq_len), device=device)
    doc_ids = torch.randint(0, 32128, (batch_size, seq_len), device=device)
    doc_mask = torch.ones((batch_size, seq_len), device=device)
    
    optimizer = torch.optim.AdamW(model.parameters(), lr=2e-5)
    
    def maxsim_scores(query_emb, query_mask, doc_emb, doc_mask):
        query_emb = F.normalize(query_emb, p=2, dim=-1)
        doc_emb = F.normalize(doc_emb, p=2, dim=-1)
        sim = torch.einsum("iqd,jkd->ijqk", query_emb, doc_emb)
        doc_mask_expanded = doc_mask.unsqueeze(0).unsqueeze(2)
        sim = sim.masked_fill(doc_mask_expanded == 0, -1e4)
        max_sim = sim.max(dim=3).values
        query_mask_expanded = query_mask.unsqueeze(1).float()
        max_sim = max_sim * query_mask_expanded
        query_lens = query_mask.sum(dim=1, keepdim=True).unsqueeze(1).float().clamp(min=1.0)
        return max_sim.sum(dim=2) / query_lens.squeeze(2)
    
    # Warmup
    print("PyTorch warmup...")
    model.train()
    for _ in range(3):
        optimizer.zero_grad()
        query_out = model(input_ids=query_ids, attention_mask=query_mask)
        doc_out = model(input_ids=doc_ids, attention_mask=doc_mask)
        scores = maxsim_scores(query_out.last_hidden_state, query_mask, doc_out.last_hidden_state, doc_mask)
        labels = torch.arange(batch_size, device=device)
        loss = F.cross_entropy(scores / 0.07, labels)
        loss.backward()
        optimizer.step()
    
    # Benchmark
    print("PyTorch benchmark...")
    n_steps = 20
    start = time.time()
    for _ in range(n_steps):
        optimizer.zero_grad()
        query_out = model(input_ids=query_ids, attention_mask=query_mask)
        doc_out = model(input_ids=doc_ids, attention_mask=doc_mask)
        scores = maxsim_scores(query_out.last_hidden_state, query_mask, doc_out.last_hidden_state, doc_mask)
        labels = torch.arange(batch_size, device=device)
        loss = F.cross_entropy(scores / 0.07, labels)
        loss.backward()
        optimizer.step()
        torch.mps.synchronize()
    elapsed = time.time() - start
    
    pytorch_samples_per_sec = n_steps * batch_size / elapsed
    pytorch_ms_per_step = elapsed / n_steps * 1000
    print(f"PyTorch: {pytorch_samples_per_sec:.1f} samples/s, {pytorch_ms_per_step:.1f} ms/step")
    
    return pytorch_samples_per_sec, pytorch_ms_per_step


if __name__ == "__main__":
    print("=" * 60)
    print("Training Speed Benchmark: MLX vs PyTorch MPS")
    print("=" * 60)
    print()
    
    mlx_sps, mlx_ms = benchmark_mlx()
    print()
    pytorch_sps, pytorch_ms = benchmark_pytorch()
    
    print()
    print("=" * 60)
    print("Results:")
    print(f"  MLX:     {mlx_sps:.1f} samples/s ({mlx_ms:.1f} ms/step)")
    print(f"  PyTorch: {pytorch_sps:.1f} samples/s ({pytorch_ms:.1f} ms/step)")
    print(f"  Speedup: {mlx_sps/pytorch_sps:.2f}x")
    print("=" * 60)
