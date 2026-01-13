# Training Execution Plan: SOTA Code Search

## Goal

Train XTR (Google's ColBERTv2) on maximum available permissive-licensed code data to achieve SOTA Recall@K AUC for agentic code search.

## Data Sources

| Dataset | Size | License | Status |
|---------|------|---------|--------|
| **CodeSearchNet** | ~2M pairs | MIT | Download |
| **The Stack v2** | Extract ~500K+ | Permissive | Download + Extract |
| **Your 138K** | 138K pairs | Apache/MIT | Already have |
| **Total Target** | ~2.5-3M pairs | | |

## Execution Steps

### Step 1: Download CodeSearchNet
```bash
python scripts/download_codesearchnet.py --output data/codesearchnet_all.jsonl
```
- 6 languages: Python, Java, Go, PHP, JavaScript, Ruby
- ~2M docstring-code pairs after filtering

### Step 2: Download The Stack (Permissive Subset)
```bash
python scripts/download_thestack.py \
    --languages python,java,go,rust,typescript,javascript \
    --max-per-language 100000 \
    --output data/thestack_extracted.jsonl
```
- Extract function-docstring pairs from raw code
- Filter to permissive licenses only
- Target: ~500K additional pairs

### Step 3: Combine and Clean
```bash
python scripts/prepare_training_data.py \
    --inputs data/codesearchnet_all.jsonl \
             data/thestack_extracted.jsonl \
             data/training_data_extended.jsonl \
    --output data/combined_training.jsonl \
    --dedupe \
    --min-query-len 15 \
    --min-code-len 50 \
    --max-code-len 8000
```

### Step 4: Train
```bash
# On GPU (A100/H100)
python scripts/train_recall_optimized.py --config config/train_combined_gpu.yaml

# On M2 Max (slower but works)
python scripts/train_recall_optimized.py --config config/train_combined_mps.yaml
```

### Step 5: Evaluate
```bash
./target/release/sg eval \
    --spec eval/code_queries.json \
    --model-path checkpoints/xtr-combined-v1 \
    --hybrid --verbose
```

## Timeline

| Step | Time (GPU) | Time (M2 Max) |
|------|------------|---------------|
| Download | 30 min | 30 min |
| Extract & Clean | 1 hour | 1 hour |
| Train | 8-12 hours | 24-36 hours |
| Evaluate | 10 min | 10 min |
| **Total** | ~10-14 hours | ~26-38 hours |

## Expected Results

| Metric | Current | Expected |
|--------|---------|----------|
| P@1 | 0.87 | 0.95+ |
| MRR | 0.93 | 0.98+ |
| R@100 AUC | ~0.75 | 0.92+ |
