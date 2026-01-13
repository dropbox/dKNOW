# Domain-Specific Embedding Strategy

## Goal
Make embeddings **especially good** for:
1. **Rust** - Your primary language
2. **Formal Methods** - Specs, proofs, verification
3. **Your Code** - Personal coding style and patterns

## Strategy 1: Data Weighting (Implemented ✅)

### Upweighting Priority Data
We duplicate high-value training pairs to increase their influence:

| Category | Weight | Effect |
|----------|--------|--------|
| Formal verification (Lean, TLA+, SMT) | 5x | Model sees these 5x more often |
| Your personal repos | 3x | Model learns your style |
| General code (CodeSearchNet) | 1x | Baseline coverage |

**Result:** 114K weighted pairs from your repos added to training.

### Language Distribution After Weighting
Before weighting: Python/Java/PHP dominate (CodeSearchNet)
After weighting: Rust is heavily represented through your repos

## Strategy 2: Curriculum Learning (Recommended)

Train in stages:
1. **Stage 1:** General code understanding (CodeSearchNet 2M)
2. **Stage 2:** Rust specialization (Rust from The Stack + your repos)
3. **Stage 3:** Formal methods fine-tuning (Lean, TLA+, SMT from your repos)

This prevents forgetting general knowledge while specializing.

## Strategy 3: Contrastive Hard Negatives (Recommended)

For Rust and formal methods, create **domain-specific hard negatives**:

```python
# Example: For a Rust function query, hard negatives should be:
query = "Check if a value is within bounds"

positive = "fn check_bounds(val: usize, max: usize) -> bool { val < max }"

# GOOD hard negatives (same domain, different semantics):
hard_neg_1 = "fn get_bounds(arr: &[T]) -> (usize, usize)"  # Similar name, different purpose
hard_neg_2 = "fn clamp(val: i32, min: i32, max: i32) -> i32"  # Related concept

# BAD hard negatives:
bad_neg = "def check_bounds(val, max): return val < max"  # Python - too easy
```

## Strategy 4: Query Augmentation for Formal Methods

Formal methods have unique query patterns. Augment training with:

```python
# Specification queries
"prove that this function terminates"
"verify the invariant holds"
"check postcondition after loop"

# Implementation queries
"implement a function that satisfies: ∀x. P(x) → Q(f(x))"
"write code matching this TLA+ spec"
"translate this Lean theorem to Rust"
```

## Strategy 5: Evaluation-Driven Tuning

Create domain-specific eval sets:

```yaml
# eval/rust_queries.json - Rust-specific queries
# eval/formal_queries.json - Formal methods queries
# eval/personal_queries.json - Queries about YOUR code

# Target metrics:
# - Rust P@1: 0.95+
# - Formal methods P@1: 0.90+
# - Personal code P@1: 0.98+
```

## Implementation Plan

### Phase 1: Current Training (Running)
- 2M CodeSearchNet + 114K priority pairs
- ETA: ~67 hours

### Phase 2: Specialized Fine-tuning (After Phase 1)
```yaml
# config/train_rust_specialist.yaml
data:
  train: "data/rust_specialist.jsonl"  # Rust + formal methods only
  validation: "data/rust_eval.jsonl"

training:
  # Start from CodeSearchNet checkpoint
  base_model: "checkpoints/xtr-codesearchnet-v1/best"

  # Smaller LR for fine-tuning
  learning_rate: 5e-6
  epochs: 3

  # More hard negatives from same domain
  language_aware_batching: true
  max_hard_negatives: 7
```

### Phase 3: Personal Code Specialization
Final fine-tuning on just YOUR repos with smallest LR:
```yaml
base_model: "checkpoints/xtr-rust-specialist/best"
learning_rate: 1e-6
epochs: 5
```

## Expected Results

| Corpus | Before | After Phase 1 | After Phase 2 | After Phase 3 |
|--------|--------|---------------|---------------|---------------|
| General Code | 0.83 | 0.88 | 0.85 | 0.83 |
| Rust | 0.75 | 0.85 | 0.92 | 0.90 |
| Formal Methods | 0.60 | 0.70 | 0.85 | 0.88 |
| Your Code | 0.80 | 0.88 | 0.92 | 0.98 |

The trade-off: Slight degradation on general code to maximize performance on YOUR priority domains.

## Monitoring During Training

Key metrics to watch:
1. **Loss on priority subset** - Should decrease faster than general loss
2. **Rust recall@5** - Target 0.95+
3. **Formal methods exact match** - Specs should find exact implementations

## Alternative: Mixture of Experts (Future)

For maximum specialization without forgetting:
- Train separate LoRA adapters for each domain
- Route queries to appropriate adapter at inference time
- Rust query → Rust adapter
- Formal query → Formal adapter
- General query → Base model
