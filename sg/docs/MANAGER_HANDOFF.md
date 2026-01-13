# Manager Handoff: 2026-01-03

## TL;DR

MLX training works (2.7x faster). Keyword search is great (R@10=1.0). **Semantic search is broken** (R@10=0.7). Need LLM-as-judge training.

---

## Current State

| What | Status |
|------|--------|
| MLX training | ✅ Working, 2.7x speedup |
| Keyword queries | ✅ R@10=1.0, P@1=0.93 |
| Semantic queries | ❌ R@10=0.7, 4/10 fail completely |
| Model location | `checkpoints/xtr-mlx-merged` |

---

## The Problem

Training data was **unsupervised** (docstring → code pairs).

Model learned: `"words in docstring" ≈ "words in code"`

Model did NOT learn: `"what the code does"`

**Example failure:**
```
Query: "detecting when files change on disk"
Expected: watcher.rs
Result: MISS (no keyword overlap)
```

---

## Next Priority: LLM-as-Judge Training

**Goal:** Teach model true semantic understanding.

**Approach:**
1. Generate (query, code) pairs
2. LLM scores relevance 1-5
3. Train model to predict scores
4. Target: R@10 > 0.90 on semantic queries

**Estimated effort:** 20-30 commits

---

## Key Files

| File | What |
|------|------|
| `docs/MANAGER_REPORT_2026_01_03.md` | Detailed session report |
| `docs/PROFILING_RESULTS.md` | Why Metal kernels were abandoned |
| `docs/MLX_TRAINING_REPORT.md` | MLX training results |
| `eval/semantic_queries.json` | 10 semantic test queries |
| `scripts/train_xtr_mlx.py` | MLX training script |

---

## Don't Bother With

- Custom Metal kernels (MaxSim is only 1.4% of time)
- Smarter hybrid fusion (problem is model, not fusion)
- More unsupervised data (won't help semantic understanding)

---

## Quick Test Commands

```bash
# Keyword queries (should be 0.93 P@1)
cargo run --release -p sg -- eval --model-path checkpoints/xtr-mlx-merged --spec eval/code_queries.json --hybrid

# Semantic queries (currently 0.70 R@10)
cargo run --release -p sg -- eval --model-path checkpoints/xtr-mlx-merged --spec eval/semantic_queries.json --verbose
```

---

**Previous session:** MLX training + evaluation
**Next session:** LLM-as-judge training pipeline
