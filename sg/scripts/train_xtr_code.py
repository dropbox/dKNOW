#!/usr/bin/env python3
"""
Fine-tune XTR for code search using LoRA adapters.

Usage:
    python scripts/train_xtr_code.py --config config/train_codesearchnet.yaml
"""

from __future__ import annotations

import argparse
import json
import math
import os
import random
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, Iterable, Iterator, List, Optional

import torch
import torch.nn.functional as F
import yaml
from peft import LoraConfig, get_peft_model
from transformers import AutoTokenizer, T5EncoderModel, get_scheduler


@dataclass
class TrainingExample:
    query: str
    positive: str
    language: Optional[str]


def load_config(path: Path) -> Dict:
    with path.open("r", encoding="utf-8") as handle:
        return yaml.safe_load(handle)


def count_examples(path: Path, languages: Optional[set[str]]) -> int:
    count = 0
    with path.open("r", encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if not line:
                continue
            try:
                payload = json.loads(line)
            except json.JSONDecodeError:
                continue
            if languages and payload.get("language") not in languages:
                continue
            if "query" not in payload or "positive" not in payload:
                continue
            count += 1
    return count


def iter_examples(
    path: Path,
    languages: Optional[set[str]],
    max_examples: Optional[int],
    shuffle_buffer: int,
    seed: int,
) -> Iterator[TrainingExample]:
    rng = random.Random(seed)
    buffer: List[TrainingExample] = []
    yielded = 0

    def flush_buffer() -> Iterator[TrainingExample]:
        rng.shuffle(buffer)
        while buffer:
            yield buffer.pop()

    with path.open("r", encoding="utf-8") as handle:
        for line in handle:
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
            if not query or not positive:
                continue
            buffer.append(
                TrainingExample(
                    query=query,
                    positive=positive,
                    language=payload.get("language"),
                )
            )
            if len(buffer) >= shuffle_buffer:
                idx = rng.randrange(len(buffer))
                yield buffer.pop(idx)
                yielded += 1
                if max_examples and yielded >= max_examples:
                    return

        for example in flush_buffer():
            yield example
            yielded += 1
            if max_examples and yielded >= max_examples:
                return


def batch_examples(examples: Iterable[TrainingExample], batch_size: int) -> Iterator[List[TrainingExample]]:
    batch: List[TrainingExample] = []
    for example in examples:
        batch.append(example)
        if len(batch) == batch_size:
            yield batch
            batch = []
    if batch:
        yield batch


def select_device(preferred: Optional[str]) -> torch.device:
    if preferred:
        return torch.device(preferred)
    if torch.cuda.is_available():
        return torch.device("cuda")
    if getattr(torch.backends, "mps", None) and torch.backends.mps.is_available():
        return torch.device("mps")
    return torch.device("cpu")


def maxsim_scores(
    query_emb: torch.Tensor,
    query_mask: torch.Tensor,
    doc_emb: torch.Tensor,
    doc_mask: torch.Tensor,
) -> torch.Tensor:
    """Compute MaxSim scores between queries and documents.

    L2-normalizes embeddings to keep similarity in [-1, 1] range,
    then uses mean (not sum) over query tokens for stable training.
    """
    # L2 normalize embeddings (standard for contrastive learning)
    query_emb = F.normalize(query_emb, p=2, dim=-1)
    doc_emb = F.normalize(doc_emb, p=2, dim=-1)

    batch_size, query_tokens, _ = query_emb.shape
    scores: List[torch.Tensor] = []
    doc_mask_exp = doc_mask.unsqueeze(1)
    for i in range(batch_size):
        query_vecs = query_emb[i]
        sim = torch.einsum("qd,bkd->bqk", query_vecs, doc_emb)
        sim = sim.masked_fill(doc_mask_exp == 0, -1e9)
        max_sim = sim.max(dim=2).values  # [B, Q]
        # Mean over query tokens (not sum) for stable training
        query_len = query_mask[i].sum()
        max_sim = max_sim * query_mask[i].unsqueeze(0)
        scores.append(max_sim.sum(dim=1) / query_len)
    return torch.stack(scores, dim=0)


def infonce_loss(
    scores: torch.Tensor,
    temperature: float,
    hard_negatives: Optional[int],
) -> torch.Tensor:
    batch_size = scores.size(0)
    if not hard_negatives or hard_negatives >= batch_size:
        logits = scores / temperature
        labels = torch.arange(batch_size, device=scores.device)
        return F.cross_entropy(logits, labels)

    losses: List[torch.Tensor] = []
    for i in range(batch_size):
        row = scores[i]
        pos = row[i]
        neg = torch.cat([row[:i], row[i + 1 :]])
        k = min(hard_negatives, neg.numel())
        neg_topk = torch.topk(neg, k=k, largest=True).values
        logits = torch.cat([pos.unsqueeze(0), neg_topk], dim=0) / temperature
        labels = torch.zeros(1, dtype=torch.long, device=scores.device)
        losses.append(F.cross_entropy(logits.unsqueeze(0), labels))
    return torch.stack(losses).mean()


def tokenize_batch(
    tokenizer: AutoTokenizer,
    batch: List[TrainingExample],
    max_length: int,
    device: torch.device,
) -> tuple[Dict[str, torch.Tensor], Dict[str, torch.Tensor]]:
    queries = [example.query for example in batch]
    positives = [example.positive for example in batch]

    query_inputs = tokenizer(
        queries,
        return_tensors="pt",
        padding=True,
        truncation=True,
        max_length=max_length,
    )
    pos_inputs = tokenizer(
        positives,
        return_tensors="pt",
        padding=True,
        truncation=True,
        max_length=max_length,
    )
    query_inputs = {k: v.to(device) for k, v in query_inputs.items()}
    pos_inputs = {k: v.to(device) for k, v in pos_inputs.items()}
    return query_inputs, pos_inputs


def train(config: Dict) -> None:
    training_cfg = config["training"]
    data_cfg = config["data"]
    model_cfg = config["model"]

    seed = int(training_cfg.get("seed", 42))
    random.seed(seed)
    torch.manual_seed(seed)

    device = select_device(training_cfg.get("device"))
    print(f"Using device: {device}")

    base_model = model_cfg["base"]
    model = T5EncoderModel.from_pretrained(base_model)
    tokenizer = AutoTokenizer.from_pretrained(base_model)
    model.to(device)

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

    train_path = Path(data_cfg["train"])
    languages = set(data_cfg.get("languages", [])) or None
    max_length = int(training_cfg.get("max_length", 512))
    shuffle_buffer = int(training_cfg.get("shuffle_buffer", 10000))
    max_examples = training_cfg.get("max_examples")
    hard_negatives = training_cfg.get("hard_negatives")
    temperature = float(training_cfg.get("temperature", 0.07))

    batch_size = int(training_cfg["batch_size"])
    epochs = int(training_cfg["epochs"])
    learning_rate = float(training_cfg["learning_rate"])
    warmup_steps = int(training_cfg.get("warmup_steps", 0))
    max_steps = training_cfg.get("max_steps")

    if max_examples is not None:
        max_examples = int(max_examples)

    optimizer = torch.optim.AdamW(model.parameters(), lr=learning_rate)

    if max_steps is None and warmup_steps > 0:
        total_examples = count_examples(train_path, languages)
        steps_per_epoch = math.ceil(total_examples / batch_size)
        max_steps = steps_per_epoch * epochs
        print(f"Estimated training steps: {max_steps}")

    scheduler = None
    if warmup_steps > 0 and max_steps is not None:
        scheduler = get_scheduler(
            "linear",
            optimizer=optimizer,
            num_warmup_steps=warmup_steps,
            num_training_steps=max_steps,
        )

    for epoch in range(epochs):
        model.train()
        total_loss = 0.0
        step = 0
        examples = iter_examples(
            train_path,
            languages=languages,
            max_examples=max_examples,
            shuffle_buffer=shuffle_buffer,
            seed=seed + epoch,
        )
        for batch in batch_examples(examples, batch_size=batch_size):
            query_inputs, pos_inputs = tokenize_batch(tokenizer, batch, max_length, device)
            query_out = model(**query_inputs)
            pos_out = model(**pos_inputs)

            query_emb = query_out.last_hidden_state
            pos_emb = pos_out.last_hidden_state
            query_mask = query_inputs["attention_mask"].float()
            pos_mask = pos_inputs["attention_mask"].float()

            scores = maxsim_scores(query_emb, query_mask, pos_emb, pos_mask)
            loss = infonce_loss(scores, temperature, hard_negatives)

            optimizer.zero_grad()
            loss.backward()
            optimizer.step()
            if scheduler is not None:
                scheduler.step()

            total_loss += loss.item()
            step += 1
            if step % int(training_cfg.get("log_every", 50)) == 0:
                avg_loss = total_loss / step
                print(f"Epoch {epoch + 1} step {step}: loss={avg_loss:.6f}")

        avg_loss = total_loss / max(step, 1)
        print(f"Epoch {epoch + 1} complete: loss={avg_loss:.6f}")

    model.save_pretrained(output_dir)
    tokenizer.save_pretrained(output_dir)
    print(f"Saved LoRA adapters to {output_dir}")


def dry_run(config: Dict) -> None:
    """Verify pipeline without training: load model, tokenizer, data."""
    training_cfg = config["training"]
    data_cfg = config["data"]
    model_cfg = config["model"]

    device = select_device(training_cfg.get("device"))
    print(f"Device: {device}")

    base_model = model_cfg["base"]
    print(f"Loading model: {base_model}")
    model = T5EncoderModel.from_pretrained(base_model)
    tokenizer = AutoTokenizer.from_pretrained(base_model)
    model.to(device)
    print(f"  Model loaded: {model.config.hidden_size}d, {model.config.num_hidden_layers} layers")

    if training_cfg.get("method") == "lora":
        lora_config = LoraConfig(
            r=training_cfg["lora_r"],
            lora_alpha=training_cfg["lora_alpha"],
            lora_dropout=training_cfg["lora_dropout"],
            target_modules=training_cfg["target_modules"],
        )
        model = get_peft_model(model, lora_config)
        model.print_trainable_parameters()

    train_path = Path(data_cfg["train"])
    languages = set(data_cfg.get("languages", [])) or None
    total = count_examples(train_path, languages)
    print(f"Training data: {train_path}")
    print(f"  Examples: {total}")
    print(f"  Languages: {languages or 'all'}")

    batch_size = int(training_cfg["batch_size"])
    epochs = int(training_cfg["epochs"])
    steps_per_epoch = math.ceil(total / batch_size)
    print(f"Training config:")
    print(f"  Epochs: {epochs}")
    print(f"  Batch size: {batch_size}")
    print(f"  Steps/epoch: {steps_per_epoch}")
    print(f"  Total steps: {steps_per_epoch * epochs}")

    # Test one batch
    print("Testing one batch...")
    examples = list(iter_examples(train_path, languages, max_examples=batch_size, shuffle_buffer=100, seed=42))
    max_length = int(training_cfg.get("max_length", 512))
    query_inputs, pos_inputs = tokenize_batch(tokenizer, examples, max_length, device)
    with torch.no_grad():
        query_out = model(**query_inputs)
        pos_out = model(**pos_inputs)
    print(f"  Query shape: {query_out.last_hidden_state.shape}")
    print(f"  Pos shape: {pos_out.last_hidden_state.shape}")

    print("\nDry run complete. Pipeline ready for training.")


def main() -> None:
    parser = argparse.ArgumentParser(description="Fine-tune XTR for code search")
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
