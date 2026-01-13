# Manager Report: MLX Training & Semantic Search Evaluation

**Date:** 2026-01-03
**Session:** MLX Training Acceleration + Evaluation
**Status:** COMPLETE - Handoff to next manager

---

## Executive Summary

Successfully completed MLX training acceleration achieving **2.7x speedup** and **P@1=0.93** on keyword queries. However, evaluation revealed a critical gap: **semantic understanding is weak** (R@10=0.70 on conceptual queries). The model learned vocabulary matching, not true semantic understanding.

**Key finding:** Training on unsupervised docstring-code pairs does not teach the model to understand what code *does*. LLM-as-judge training is needed.

---

## What Was Accomplished

### 1. OOM Fix & Profiling
- Fixed memory crash in profiling script (was loading 172K examples)
- Profiling revealed MaxSim is only **1.4%** of training time (not the bottleneck)
- Backward pass is **63.5%** of time - cannot be accelerated with custom Metal kernels

### 2. MLX Training Infrastructure
- Ported T5 encoder to Apple MLX framework
- Implemented LoRA fine-tuning in MLX
- Created training scripts: `scripts/train_xtr_mlx.py`, `scripts/mlx_t5/t5.py`
- Achieved **2.7x speedup** over PyTorch MPS (66 vs 10 samples/s on small batches)

### 3. Full Training Run
- **172,750 examples** across 8 languages (Rust 123K, Python 28K, Lean 11K, etc.)
- **7.89 hours** training time on Apple M2
- **Final loss:** 0.1084
- **Model saved:** `checkpoints/xtr-mlx-merged`

### 4. Smarter Hybrid Search
- Implemented confidence-aware fusion (uses actual scores, not just ranks)
- Added minimum result guarantees from each source
- Result: No improvement - problem is model understanding, not fusion algorithm

---

## Evaluation Results

### Keyword Queries (original test set)
Queries contain keywords that match code vocabulary.

| Mode | P@1 | R@5 | R@10 |
|------|-----|-----|------|
| Semantic | 0.67 | 0.93 | 1.00 |
| Hybrid | **0.93** | **1.00** | **1.00** |

**Verdict:** Excellent for keyword-matching queries.

### Semantic Queries (conceptual, no keyword overlap)
Queries describe what code does without using code vocabulary.

| Mode | P@1 | R@5 | R@10 |
|------|-----|-----|------|
| Semantic | 0.40 | 0.50 | **0.60** |
| Hybrid | 0.20 | 0.50 | **0.70** |

**Verdict:** Poor. 4 out of 10 queries completely fail.

### Failed Queries Analysis

| Query | Expected | Why It Fails |
|-------|----------|--------------|
| "detecting when files change on disk" | watcher.rs | No "change" or "detect" in code |
| "communication between separate programs" | server.rs | No "communication" or "programs" |
| "finding the root directory of a codebase" | project.rs | No "root" or "codebase" |
| "remembering previous computations" | dedup.rs | Conceptual mismatch |

---

## Root Cause Analysis

### Training Data Problem

The model was trained on **unsupervised pairs**:
```
docstring: "Implements k-means clustering for bucket assignment"
code: fn assign_buckets(...) { /* k-means implementation */ }
```

This teaches: **"docstring words ≈ code words"**

It does NOT teach: **"this code does X functionality"**

### What's Needed: LLM-as-Judge Training

```
Query: "code that groups similar vectors together"
Code: fn assign_buckets(...) { /* k-means */ }
LLM Judge Score: 5/5 (highly relevant)

Query: "detecting when files change"
Code: fn watch_directory(...) { notify::watch() }
LLM Judge Score: 5/5 (highly relevant)
```

Train model to predict these scores → learns true semantic understanding.

---

## Recommendations for Next Manager

### Priority 1: LLM-as-Judge Training Pipeline

**Estimated effort:** 20-30 commits

1. **Generate candidate pairs** from codebase
   - Function/struct + synthetic queries
   - Use LLM to generate diverse query phrasings

2. **LLM scoring** (Claude Haiku for cost)
   - Score each (query, code) pair 1-5
   - Include reasoning for quality control

3. **Train with ordinal loss**
   - Contrastive loss with margin based on score difference
   - Or: regression to predict score directly

4. **Distill to efficient model**
   - Fine-tune XTR/ColBERT on LLM scores
   - Target: R@10 > 0.90 on semantic queries

### Priority 2: Better Evaluation

- Current test set is too small (15 queries)
- Need 100+ queries with ground truth
- Include Q&A style, conceptual, and keyword queries

### Priority 3: (Optional) Smarter Retrieval

- Two-stage: fast retrieval → LLM rerank
- Already have `--rerank` flag with Claude
- Could improve R@10 without retraining

---

## Files Created This Session

| File | Purpose |
|------|---------|
| `docs/PROFILING_RESULTS.md` | Training bottleneck analysis |
| `docs/MLX_TRAINING_REPORT.md` | MLX training results |
| `docs/MANAGER_REPORT_2026_01_03.md` | This report |
| `config/train_mlx.yaml` | MLX training config |
| `config/train_mlx_test.yaml` | Quick test config |
| `config/profile.yaml` | Profiling config |
| `eval/semantic_queries.json` | Semantic evaluation queries |
| `checkpoints/xtr-mlx/` | MLX training checkpoints |
| `checkpoints/xtr-mlx-merged/` | Merged PyTorch model |

---

## Git Commits This Session

```
[W]#1: Add training profiler and document bottleneck analysis
[W]#2: Revise training acceleration plan - MLX approach
[W]#7: Verify MLX training works - 2.7x speedup confirmed
[W]#8: MLX training complete - P@1=0.93 achieved
```

---

## Metrics Summary

| Metric | Value | Target | Status |
|--------|-------|--------|--------|
| Training speedup | 2.7x | 2x | ✅ Exceeded |
| P@1 (keyword, hybrid) | 0.93 | 0.93 | ✅ Met |
| R@10 (keyword) | 1.00 | 0.95 | ✅ Exceeded |
| R@10 (semantic) | 0.70 | 0.90 | ❌ Gap |

---

## Conclusion

The MLX training infrastructure works well and achieves good results on keyword queries. However, **true semantic understanding requires a different training approach** - supervised learning from LLM judgments rather than unsupervised docstring-code pairs.

The next manager should prioritize the LLM-as-judge training pipeline to close the semantic understanding gap.

---

**Handoff to:** Next AI Manager
**Priority:** LLM-as-Judge Training Pipeline
**Estimated effort:** 20-30 commits
