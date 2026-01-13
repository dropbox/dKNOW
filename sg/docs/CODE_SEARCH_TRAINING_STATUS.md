# Code Search Training Status

**Date:** 2026-01-01
**Goal:** State-of-the-art code search for agentic coding systems

## TL;DR - Do We Need All This Data?

**Short answer: Probably not.**

| Dataset | Pairs | Necessity |
|---------|-------|-----------|
| CodeSearchNet 2M | 1,985,218 | ⚠️ Overkill - 172K achieved MRR 0.5774 |
| The Stack 300K | 300,000 | ⚠️ Redundant - overlaps with CodeSearchNet |
| Priority Repos 114K | 114,583 | ✅ Essential - YOUR code, formal methods |

**Recommendation:** Train on Priority Repos (114K) + subset of CodeSearchNet (~200K) instead of full 2M. The 172K training that completed achieved good results.

## What's Running Now

```
2M CodeSearchNet Training
- Step 40/41,357 (0.1%)
- Speed: 7 samples/s
- ETA: ~75 hours
- Config: config/train_codesearchnet_improved.yaml
```

**To monitor:** `tail -f logs/training_2m_codesearchnet.log`

**To kill:** `pkill -f train_xtr_improved.py`

## Data Sources

### 1. CodeSearchNet (MIT License)
- **Source:** https://github.com/github/CodeSearchNet
- **Size:** 2M query-code pairs
- **Languages:** Python, Java, JavaScript, PHP, Go, Ruby
- **Paper:** https://arxiv.org/abs/1909.09436

### 2. The Stack (Permissive Licenses)
- **Source:** https://huggingface.co/datasets/bigcode/the-stack
- **Size:** 300K pairs extracted (50K per language)
- **Languages:** Python, Java, Go, Rust, JavaScript, TypeScript
- **Paper:** https://arxiv.org/abs/2211.15533

### 3. Priority Repos (Your Code)
- **Source:** github.com/dropbox/*
- **Size:** 114K weighted pairs
- **Weighting:** Formal methods 5x, User code 3x

## Files Created

```
data/
├── combined_training.jsonl      # 2M CodeSearchNet + existing 138K
├── combined_training.val.jsonl  # 20K validation
├── thestack_extracted.jsonl     # 300K from The Stack
├── priority_training.jsonl      # 114K from your repos
└── priority_repos/              # Cloned repos
    ├── dashprove/               # Formal verification
    ├── lean5/                   # Lean4 in Rust
    ├── z4/                      # Z3 in Rust
    ├── tRust/                   # Trusted Rust
    ├── kani_fast/               # Kani fork
    ├── tla2/                    # TLA+ in Rust
    ├── gamma-crown/             # NN verification
    ├── dashflow/                # AI workflows
    └── chunker/                 # Chunking lib

scripts/
├── download_codesearchnet.py    # Download CodeSearchNet
├── download_thestack_v2.py      # Download The Stack
├── extract_priority_training.py # Extract from your repos
└── train_xtr_improved.py        # Main training script

config/
├── train_codesearchnet_improved.yaml  # 2M training config
└── train_rust_direct.yaml             # Direct fine-tuning config

docs/
├── DOMAIN_SPECIFIC_EMBEDDING_STRATEGY.md  # Optimization strategy
└── CODE_SEARCH_TRAINING_STATUS.md         # This file
```

## Key Findings

### 1. MPS (Apple Silicon) Training is Unstable
- Hangs after ~100-1000 batches without proper sync
- **Fix:** `torch.mps.synchronize()` after every batch
- **Fix:** Smaller effective batch size (48 vs 96)
- See commit bf2324a

### 2. 172K Training Already Achieved Good Results
- MRR: 0.5774
- Completed in ~7 hours
- Config: `config/train_improved.yaml`

### 3. Domain-Specific Upweighting Works
- Formal verification repos: 5x weight
- User code repos: 3x weight
- This ensures model learns YOUR patterns

## Recommended Next Steps

### Option A: Let 2M Training Complete (~75 hours)
Then evaluate and compare to 172K baseline.

### Option B: Cancel and Train Smarter (Recommended)
```bash
# Kill current training
pkill -f train_xtr_improved.py

# Train on priority data only
python scripts/train_xtr_improved.py --config config/train_rust_direct.yaml
```

### Option C: Two-Stage Training
1. Quick general training on 200K CodeSearchNet subset
2. Fine-tune on 114K priority repos

## References

### Papers
- [ColBERT](https://arxiv.org/abs/2004.12832) - Late interaction retrieval
- [ColBERTv2](https://arxiv.org/abs/2112.01488) - Improved training
- [XTR](https://arxiv.org/abs/2304.01982) - Google's ColBERT variant (what we use)
- [CodeSearchNet](https://arxiv.org/abs/1909.09436) - Code search benchmark
- [The Stack](https://arxiv.org/abs/2211.15533) - Large code dataset

### Code
- XTR Base Model: `google/xtr-base-en` on HuggingFace
- Our training: `scripts/train_xtr_improved.py`
- Evaluation: `sg eval --model-path checkpoints/xtr-*`

## Contact

Training started by Claude Code session on 2026-01-01.
Config and data prepared for handoff to next manager.
