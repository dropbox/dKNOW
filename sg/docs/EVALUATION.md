# SuperGrep Evaluation Plan

## Problem

The current tests verify basic functionality but do NOT measure search quality:
- No Recall@k metrics
- No precision measurements
- No comparison to baselines
- No ground truth dataset

**Current "evaluation" is circular** - testing sg on its own 31-file codebase with hand-crafted queries proves nothing.

## Evaluation Corpora

We need multiple corpora to properly validate:

### Corpus 1: Large Code Repository (Recall Test)

Download a large, well-known codebase:
```bash
# Option A: Linux kernel (huge, 70K+ files)
git clone --depth 1 https://github.com/torvalds/linux.git /tmp/eval/linux

# Option B: Rust compiler (large, diverse)
git clone --depth 1 https://github.com/rust-lang/rust.git /tmp/eval/rust

# Option C: VS Code (TypeScript, 10K+ files)
git clone --depth 1 https://github.com/microsoft/vscode.git /tmp/eval/vscode
```

### Corpus 2: Project Gutenberg Books (Stress Test)

Test on non-code text to verify embeddings generalize:
```bash
# Download top 100 Gutenberg books
mkdir -p /tmp/eval/gutenberg
cd /tmp/eval/gutenberg
# Pride and Prejudice
curl -o pride.txt https://www.gutenberg.org/files/1342/1342-0.txt
# Moby Dick
curl -o moby.txt https://www.gutenberg.org/files/2701/2701-0.txt
# War and Peace
curl -o war.txt https://www.gutenberg.org/files/2600/2600-0.txt
# Frankenstein
curl -o frank.txt https://www.gutenberg.org/files/84/84-0.txt
# ... add 10-20 more
```

### Corpus 3: CodeSearchNet Benchmark (Standard)

Industry-standard code search evaluation:
```bash
# Download CodeSearchNet evaluation set
# https://github.com/github/CodeSearchNet
pip install datasets
python -c "from datasets import load_dataset; d = load_dataset('code_search_net', 'python'); d.save_to_disk('/tmp/eval/codesearchnet')"
```

### Corpus 4: nfcorpus (from rust-warp)

Already available at `~/rust-warp/datasets/nfcorpus.tsv`:
- 3,633 medical documents
- Known queries with relevance judgments
- Can compute NDCG directly

## Evaluation Strategy

### 1. Ground Truth Dataset

Create `eval/ground_truth.json` with queries and expected relevant files:

```json
{
  "queries": [
    {
      "query": "k-means clustering algorithm",
      "relevant": ["crates/sg-core/src/index.rs"],
      "description": "Should find the clustering implementation"
    },
    {
      "query": "SQLite database storage embeddings",
      "relevant": ["crates/sg-core/src/storage.rs"],
      "description": "Should find the storage module"
    },
    {
      "query": "file watcher notify debounce",
      "relevant": ["crates/sg-daemon/src/watcher.rs"],
      "description": "Should find the file watching code"
    },
    {
      "query": "Unix socket IPC server",
      "relevant": ["crates/sg-daemon/src/server.rs"],
      "description": "Should find the daemon server"
    },
    {
      "query": "XTR transformer embeddings model",
      "relevant": ["crates/sg-core/src/embedder.rs"],
      "description": "Should find the embedder"
    },
    {
      "query": "CLI argument parsing clap",
      "relevant": ["crates/sg/src/main.rs"],
      "description": "Should find the CLI entry point"
    },
    {
      "query": "project root detection git cargo",
      "relevant": ["crates/sg-daemon/src/project.rs"],
      "description": "Should find project detection logic"
    },
    {
      "query": "MaxSim scoring similarity search",
      "relevant": ["crates/sg-core/src/embedder.rs", "crates/sg-core/src/search.rs"],
      "description": "Should find MaxSim implementation"
    },
    {
      "query": "daemon start stop status pid",
      "relevant": ["crates/sg-daemon/src/main.rs", "crates/sg/src/main.rs"],
      "description": "Should find daemon management code"
    },
    {
      "query": "JSON serialization serde protocol",
      "relevant": ["crates/sg-daemon/src/protocol.rs"],
      "description": "Should find the IPC protocol types"
    }
  ]
}
```

### 2. Metrics to Compute

| Metric | Formula | What it measures |
|--------|---------|------------------|
| **Recall@k** | (relevant ∩ top-k) / relevant | % of relevant docs found in top k |
| **MRR** | 1 / rank_of_first_relevant | How quickly we find a relevant result |
| **P@1** | 1 if top result is relevant, else 0 | Is the #1 result correct? |
| **NDCG@k** | DCG@k / IDCG@k | Quality of ranking (position matters) |

### 3. Baselines for Comparison

1. **Random**: Shuffle files, measure expected metrics
2. **Keyword (ripgrep)**: `rg -l "query terms"` - lexical matching
3. **Semantic-only**: `sg --no-hybrid "query"`
4. **Hybrid**: `sg "query"` (default)

### 4. Implementation

Add `sg eval` command that:

```bash
$ sg eval
Running evaluation on 10 queries...

Query: "k-means clustering algorithm"
  Expected: crates/sg-core/src/index.rs
  Got #1:   crates/sg-core/src/index.rs ✓ (score: 1.00)
  Recall@5: 1.00, MRR: 1.00

Query: "SQLite database storage"
  Expected: crates/sg-core/src/storage.rs
  Got #1:   crates/sg-core/src/storage.rs ✓ (score: 0.98)
  Recall@5: 1.00, MRR: 1.00

...

=== Summary ===
Queries:     10
Mean MRR:    0.92
Mean R@5:    0.95
Mean P@1:    0.90
Mean NDCG@5: 0.91

Baseline comparison:
  Semantic:  MRR=0.92, R@5=0.95
  Keyword:   MRR=0.65, R@5=0.70
  Random:    MRR=0.05, R@5=0.15
```

### 5. Success Criteria

For SuperGrep to be considered "working":

| Metric | Target | Rationale |
|--------|--------|-----------|
| MRR | ≥ 0.80 | First relevant result usually in top 2 |
| Recall@5 | ≥ 0.90 | Find 90% of relevant docs in top 5 |
| P@1 | ≥ 0.70 | Top result is correct 70% of time |
| vs Keyword | +20% MRR | Semantic should beat keyword search |

### 6. Files to Create

```
eval/
├── ground_truth.json    # Query -> relevant files mapping
├── run_eval.rs          # Evaluation harness (or add to CLI)
└── results/
    └── baseline.json    # Stored baseline results
```

## Implementation Steps

1. Create `eval/ground_truth.json` with 10-20 queries
2. Add `sg eval` command to CLI
3. Implement metric calculations
4. Run evaluation and report results
5. Compare against keyword baseline
6. Document results in README
