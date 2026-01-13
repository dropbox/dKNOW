# Embedding Training Guide

This document describes how we fine-tune XTR embeddings for semantic code search.

## Overview

We fine-tune Google's XTR (Cross-encoder Text Retrieval) model using contrastive learning on code-query pairs. The goal is to make the model better at matching natural language queries to relevant code snippets.

**Base Model:** `google/xtr-base-en` (112M parameters)
**Method:** LoRA (Low-Rank Adaptation) - trains only 2.5% of parameters
**Output:** Multi-vector embeddings with MaxSim scoring

## Training Pipeline

```
Raw Code Repositories
        │
        ▼
┌───────────────────┐
│ Data Extraction   │  Extract (query, code) pairs from docstrings,
│                   │  function signatures, comments
└───────────────────┘
        │
        ▼
┌───────────────────┐
│ Quality Filtering │  Remove boilerplate, check semantic overlap,
│                   │  filter short/low-quality pairs
└───────────────────┘
        │
        ▼
┌───────────────────┐
│ Query Augmentation│  Add verb synonyms, question forms,
│                   │  simplified versions
└───────────────────┘
        │
        ▼
┌───────────────────┐
│ Contrastive       │  InfoNCE loss with in-batch negatives,
│ Training          │  language-aware batching, margin loss
└───────────────────┘
        │
        ▼
┌───────────────────┐
│ Validation        │  MRR evaluation, early stopping,
│                   │  checkpoint selection
└───────────────────┘
        │
        ▼
    Fine-tuned Model
```

## Data Collection

### Sources

Training data is extracted from high-quality open source repositories:

| Source | Languages | Examples | Notes |
|--------|-----------|----------|-------|
| Local codebases | Rust, Python, etc. | ~170K | Docstrings + signatures |
| CodeSearchNet | 6 languages | ~2M | Curated benchmark |
| The Stack | 300+ languages | Large | Requires HuggingFace auth |

### Extraction Scripts

```bash
# Extract from Rust projects
python scripts/extract_rust_training_data.py ~/code/project -o data/rust.jsonl

# Extract from Python
python scripts/extract_python_training_data.py ~/code/pyproject -o data/python.jsonl

# Combine and deduplicate
python scripts/combine_training_data.py data/*.jsonl -o data/combined.jsonl
```

### Data Format

Each training example is a JSON line:

```json
{
  "query": "check if a number is prime",
  "positive": "def is_prime(n: int) -> bool:\n    if n < 2:\n        return False\n    for i in range(2, int(n**0.5) + 1):\n        if n % i == 0:\n            return False\n    return True",
  "language": "python",
  "hard_negatives": ["def is_even(n): return n % 2 == 0", "..."]
}
```

## Data Quality Pipeline

### 1. Boilerplate Filtering

Remove generic, non-informative pairs:

```python
BOILERPLATE_PATTERNS = [
    r"^Returns?\s+(self|the|a|an|true|false|none|ok|err)\b",
    r"^Gets?\s+the\s+",
    r"^Sets?\s+the\s+",
    r"^Creates?\s+(a\s+)?new\s+",
    r"^Default\s+implementation",
    r"^TODO",
    r"^FIXME",
]
```

**Impact:** Removes ~10% of data, improves signal quality.

### 2. Semantic Overlap Check

Ensure query and code are semantically related but not trivially similar:

```python
# Reject if query appears verbatim in code (too easy)
if query.lower() in code.lower():
    continue

# Reject if no word overlap (likely unrelated)
query_words = set(query.lower().split())
code_words = set(re.findall(r'\w+', code.lower()))
if len(query_words & code_words) == 0:
    continue
```

### 3. Query Augmentation

Generate variations to improve robustness:

| Original | Augmented |
|----------|-----------|
| "parse JSON config" | "load JSON config" |
| "parse JSON config" | "how to parse JSON config?" |
| "parse JSON config" | "JSON parsing" |

```python
VERB_SYNONYMS = {
    "parse": ["load", "read", "decode", "deserialize"],
    "create": ["make", "build", "construct", "generate"],
    "check": ["verify", "validate", "test", "ensure"],
    # ...
}
```

**Impact:** +40% training examples, better generalization.

## Training Configuration

### LoRA Setup

We use LoRA to efficiently fine-tune without modifying most weights:

```yaml
training:
  method: "lora"
  lora_r: 16              # Rank of adaptation matrices
  lora_alpha: 32          # Scaling factor
  lora_dropout: 0.05
  target_modules:         # Which layers to adapt
    - "q"                 # Query projection
    - "v"                 # Value projection
    - "k"                 # Key projection
    - "o"                 # Output projection
    - "wi_0"              # FFN layer 1
    - "wi_1"              # FFN layer 2
    - "wo"                # FFN output
```

**Trainable parameters:** 2.8M / 112M (2.5%)

### Hyperparameters

```yaml
training:
  batch_size: 16
  gradient_accumulation_steps: 4  # Effective batch = 64
  epochs: 3
  learning_rate: 3.0e-5
  warmup_steps: 500
  weight_decay: 0.01
  max_grad_norm: 1.0
  temperature: 0.07               # Contrastive temperature
  max_length: 384                 # Token limit
```

### Loss Function

We use a combined loss:

1. **InfoNCE Loss** - Standard contrastive loss with in-batch negatives
2. **Margin Loss** - Ensures minimum separation between positives and hard negatives

```python
# InfoNCE: softmax over similarities
logits = scores / temperature
labels = torch.arange(batch_size)
ce_loss = F.cross_entropy(logits, labels)

# Margin: triplet-style separation
pos_scores = scores.diag()
hardest_neg = scores.masked_fill(eye_mask, -inf).max(dim=1)
margin_loss = F.relu(margin - (pos_scores - hardest_neg)).mean()

# Combined
total_loss = ce_loss + 0.1 * margin_loss
```

## Advanced Techniques

### Language-Aware Batching

Batch examples from the same programming language together. This creates stronger in-batch negatives (similar code that ISN'T the answer).

```python
class LanguageAwareBatchSampler:
    """Groups examples by language within each batch."""

    def _build_batches(self):
        for lang, indices in self.dataset.language_indices.items():
            # Create batches within this language
            for i in range(0, len(indices), batch_size):
                batch = indices[i:i + batch_size]
                self._batches.append(batch)
```

**Impact:** +5-10% MRR improvement.

### Layer-wise Learning Rate Decay (LLRD)

Lower layers (closer to input) get smaller learning rates to preserve pre-trained knowledge:

```python
# Layer 0 (bottom): lr * 0.9^11 = lr * 0.31
# Layer 6 (middle): lr * 0.9^5  = lr * 0.59
# Layer 11 (top):   lr * 0.9^0  = lr * 1.0
layer_lr = base_lr * (decay_rate ** (num_layers - 1 - layer_num))
```

**Impact:** Better preservation of general language understanding.

### Matryoshka Representation Learning

Train embeddings to be useful at multiple dimensions (64, 128, 256, 768):

```python
def matryoshka_loss(query_emb, pos_emb, dims=[64, 128, 256, 768]):
    total_loss = 0
    for dim in dims:
        # Truncate to dimension
        q_trunc = query_emb[:, :, :dim]
        p_trunc = pos_emb[:, :, :dim]

        # Compute contrastive loss at this dimension
        scores = maxsim_scores(q_trunc, p_trunc)
        total_loss += infonce_loss(scores) / len(dims)

    return total_loss
```

**Benefit:** Use smaller dimensions (64-128) for faster search with minimal quality loss.

### GradCache (Memory Efficiency)

Cache gradients to enable larger effective batch sizes without OOM:

```python
class GradCache:
    def forward_backward(self, queries, positives):
        # Step 1: Forward all examples without grad (chunked)
        with torch.no_grad():
            all_query_embs = [model(chunk) for chunk in queries.chunks(4)]
            all_pos_embs = [model(chunk) for chunk in positives.chunks(4)]

        # Step 2: Compute loss on full batch
        query_emb = torch.cat(all_query_embs).requires_grad_(True)
        pos_emb = torch.cat(all_pos_embs).requires_grad_(True)
        loss = contrastive_loss(query_emb, pos_emb)
        loss.backward()

        # Step 3: Re-forward chunks and propagate cached gradients
        for i, (q_grad, p_grad) in enumerate(zip(query_emb.grad.chunks(4), ...)):
            q_out = model(queries[i])
            q_out.backward(q_grad)
```

**Benefit:** 4-8x larger effective batch size.

## Validation & Metrics

### MRR (Mean Reciprocal Rank)

Primary metric - measures average position of correct result:

```
MRR = mean(1/rank for each query)

MRR = 1.0  → correct result always ranked #1
MRR = 0.5  → correct result ranked #2 on average
MRR = 0.33 → correct result ranked #3 on average
```

### Recall@K

Secondary metrics - what fraction of queries have correct result in top K:

| Metric | Meaning |
|--------|---------|
| R@1 | Exact top match (most strict) |
| R@5 | Correct in top 5 |
| R@10 | Correct in top 10 |

### Early Stopping

Stop training when validation MRR doesn't improve for N evaluations:

```yaml
training:
  eval_every: 300           # Evaluate every 300 steps
  early_stopping_patience: 5  # Stop after 5 evals without improvement
```

## Running Training

### Quick Start

```bash
cd sg/

# 1. Prepare data
python scripts/improve_training_data.py \
    --input data/raw_training.jsonl \
    --output data/training_improved.jsonl \
    --filter --augment

# 2. Train
python scripts/train_xtr_improved.py --config config/train_improved_v2.yaml

# 3. Monitor
tail -f logs/training_improved.log
```

### Configuration Files

| Config | Description |
|--------|-------------|
| `config/train_improved.yaml` | Standard training with all base improvements |
| `config/train_improved_v2.yaml` | + LLRD + Matryoshka |
| `config/train_codesearchnet.yaml` | Large-scale CodeSearchNet training |

### Expected Results

| Dataset Size | Training Time | Best MRR | R@1 |
|--------------|---------------|----------|-----|
| 170K examples | ~11 hours | 0.58 | 49% |
| 2M examples | ~70 hours | TBD | TBD |

## Deploying Fine-tuned Model

### Option 1: LoRA Adapters (Recommended)

Keep adapters separate, load at runtime:

```bash
# Checkpoint structure
checkpoints/xtr-improved/
├── adapter_config.json
├── adapter_model.safetensors  # 11MB
└── tokenizer.json
```

### Option 2: Merged Model

Merge LoRA weights into base model:

```bash
python scripts/merge_lora.py \
    checkpoints/xtr-improved \
    -o checkpoints/xtr-merged

# Use merged model
sg index --model-path checkpoints/xtr-merged ~/code/
```

## Troubleshooting

### Out of Memory

- Reduce `batch_size`
- Enable `gradient_checkpointing: true`
- Enable `use_gradcache: true`
- Reduce `max_length`

### Training Too Slow

- Reduce `max_length` (384 vs 512)
- Use `progressive_seq_length` (start short, grow)
- Ensure MPS/CUDA is being used

### Loss Not Decreasing

- Check data quality (run filtering)
- Lower learning rate
- Increase warmup steps
- Enable LLRD

### MRR Plateaus

- Add more diverse training data
- Enable language-aware batching
- Try margin loss
- Add query augmentation

## References

- [XTR Paper](https://arxiv.org/abs/2304.01982) - Multi-vector retrieval
- [LoRA Paper](https://arxiv.org/abs/2106.09685) - Parameter-efficient fine-tuning
- [Matryoshka Paper](https://arxiv.org/abs/2205.13147) - Multi-dimensional embeddings
- [GradCache Paper](https://arxiv.org/abs/2101.06983) - Memory-efficient contrastive learning
- [CodeSearchNet](https://github.com/github/CodeSearchNet) - Code search benchmark
