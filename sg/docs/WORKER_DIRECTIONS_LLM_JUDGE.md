# WORKER DIRECTIONS: LLM-as-Judge Training Pipeline

**Date:** 2026-01-03
**Priority:** CRITICAL
**Estimated Effort:** 20-30 commits
**Target:** R@10 > 0.90 on semantic queries

## Status (Worker #9-10)

| Component | Status |
|-----------|--------|
| `docs/WORKER_DIRECTIONS_LLM_JUDGE.md` | DONE |
| `scripts/score_relevance.py` | DONE |
| `scripts/train_llm_judge.py` | DONE (tested) |
| `config/train_llm_judge.yaml` | DONE |
| Generate scored data | BLOCKED (needs ANTHROPIC_API_KEY) |
| Train on scored data | PENDING |
| Evaluate R@10 | PENDING |

---

## Problem Statement

Current model (MLX-trained on 172K pairs) achieves:
- **Keyword queries:** R@10 = 1.0 (excellent)
- **Semantic queries:** R@10 = 0.7 (4/10 complete failures)

**Root cause:** Training data was unsupervised (docstring → code pairs). Model learned vocabulary matching, not semantic understanding.

**Example failure:**
```
Query: "detecting when files change on disk"
Expected: watcher.rs
Result: MISS (no keyword overlap between query and code)
```

---

## Solution: LLM-as-Judge Training

Instead of unsupervised pairs, use LLM to score (query, code) relevance:

```
Query: "code that groups similar vectors together"
Code: fn assign_buckets(...) { /* k-means implementation */ }
LLM Score: 5 (highly relevant - k-means groups similar vectors)

Query: "detecting when files change"
Code: fn watch_directory(...) { notify::watch() }
LLM Score: 5 (highly relevant - file watching detects changes)
```

Train model to predict these scores → learns true semantic understanding.

---

## Implementation Plan

### Phase 1: Relevance Scoring Pipeline (5-8 commits)

**Goal:** Generate scored training data using Claude Haiku.

**Script:** `scripts/score_relevance.py`

**Approach:**
1. Take existing (query, code) pairs
2. Generate diverse query rephrasings (semantic variations)
3. Score each (query, code) pair with LLM on 1-5 scale
4. Include reasoning for quality control
5. Output: JSONL with scores

**Prompt template:**
```
You are evaluating code search relevance.

Query: {query}

Code:
```{language}
{code}
```

Score the relevance from 1-5:
1 = Completely irrelevant
2 = Slightly related topic
3 = Related but not what user wants
4 = Good match, mostly relevant
5 = Excellent match, exactly what user wants

Respond with JSON:
{"score": N, "reasoning": "brief explanation"}
```

**Data generation:**
- Start with 10K high-quality pairs from existing training data
- Generate 3 semantic query variations per pair using LLM
- Score all 30K pairs
- Cost estimate: ~$3-5 with Haiku

### Phase 2: Training Script Modifications (5-8 commits)

**Goal:** Train model to predict relevance scores.

**Script:** `scripts/train_llm_judge.py` (new) or modify `train_xtr_mlx.py`

**Loss function options:**

1. **MSE Regression** (simplest):
   ```python
   loss = mse(model_score, llm_score)
   ```

2. **Ordinal Regression** (better for 1-5 scale):
   ```python
   # Predict P(score >= k) for k in [2,3,4,5]
   loss = bce_sum(model_probs, ordinal_targets)
   ```

3. **Contrastive with Margin** (best for ranking):
   ```python
   # Margin proportional to score difference
   margin = alpha * (score_pos - score_neg)
   loss = max(0, margin - (sim_pos - sim_neg))
   ```

**Recommended:** Start with MSE, evaluate, then try ordinal if needed.

### Phase 3: Training Run (3-5 commits)

**Goal:** Train on scored data and evaluate.

**Config:** `config/train_llm_judge.yaml`

```yaml
data:
  train: "data/scored_training.jsonl"
  validation: "data/scored_validation.jsonl"
  score_field: "llm_score"  # 1-5 relevance score

model:
  base: "checkpoints/xtr-mlx-merged"  # Start from MLX-trained
  output: "checkpoints/xtr-llm-judge"

training:
  loss: "mse"  # or "ordinal" or "margin_weighted"
  batch_size: 16
  epochs: 3
  learning_rate: 1e-5  # Lower LR for fine-tuning
```

### Phase 4: Evaluation & Iteration (5-8 commits)

**Goal:** Achieve R@10 > 0.90 on semantic queries.

**Eval commands:**
```bash
# Semantic queries (current: R@10=0.70, target: >0.90)
sg eval --model-path checkpoints/xtr-llm-judge-merged \
  --spec eval/semantic_queries.json --verbose

# Keyword queries (maintain R@10=1.0)
sg eval --model-path checkpoints/xtr-llm-judge-merged \
  --spec eval/code_queries.json --hybrid
```

**If R@10 < 0.90:**
1. Add more semantic query variations
2. Try ordinal loss
3. Increase training data (50K pairs)
4. Add hard negatives with low LLM scores

---

## File Structure

```
scripts/
├── score_relevance.py      # NEW: LLM relevance scoring
├── generate_semantic_queries.py  # NEW: Query variation generation
├── train_llm_judge.py      # NEW: Score-based training
└── train_xtr_mlx.py        # Existing MLX training

config/
├── train_llm_judge.yaml    # NEW: LLM judge training config
└── score_relevance.yaml    # NEW: Scoring config

data/
├── scored_training.jsonl   # NEW: Training data with LLM scores
└── scored_validation.jsonl # NEW: Validation data with scores

eval/
├── semantic_queries.json   # Existing: 10 semantic test queries
└── code_queries.json       # Existing: keyword queries
```

---

## Success Criteria

| Metric | Current | Target |
|--------|---------|--------|
| R@10 (semantic) | 0.70 | > 0.90 |
| R@10 (keyword) | 1.00 | >= 0.95 |
| P@1 (hybrid) | 0.93 | >= 0.90 |

---

## Risk Mitigation

1. **Cost overrun:** Start with 10K pairs, use Haiku ($0.25/1M tokens)
2. **Quality variance:** Include reasoning, filter low-confidence scores
3. **Regression on keywords:** Evaluate both query types after training
4. **Overfitting:** Use validation set, early stopping

---

## Quick Start

### Prerequisites

Set your Anthropic API key:
```bash
export ANTHROPIC_API_KEY="your-key-here"
```

### Pipeline

```bash
# Phase 1: Generate scored data (requires API key, ~$3-5 for 10K pairs)
python scripts/score_relevance.py \
  --input data/training_improved.jsonl \
  --output data/scored_training.jsonl \
  --sample 10000

# Phase 2: Train on scored data
python scripts/train_llm_judge.py \
  --config config/train_llm_judge.yaml

# Phase 3: Merge LoRA weights to PyTorch format
python scripts/convert_mlx_to_pytorch.py \
  --checkpoint checkpoints/xtr-llm-judge/checkpoint-final \
  --base checkpoints/xtr-mlx-merged \
  --output checkpoints/xtr-llm-judge-merged

# Phase 4: Evaluate
sg eval --model-path checkpoints/xtr-llm-judge-merged \
  --spec eval/semantic_queries.json --verbose
```

### Testing (verified working)

```bash
# Test training pipeline with synthetic scores (no API key needed)
python scripts/train_llm_judge.py --config config/train_llm_judge_test.yaml
```

---

## References

- Manager handoff: `docs/MANAGER_HANDOFF.md`
- Previous training: `docs/MLX_TRAINING_REPORT.md`
- Semantic test queries: `eval/semantic_queries.json`
- Existing reranker: `crates/sg-core/src/rerank.rs` (Anthropic API patterns)
