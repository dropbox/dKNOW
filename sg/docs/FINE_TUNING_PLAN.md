# XTR Fine-Tuning Plan for Code Search

## Overview

Fine-tune XTR (multi-vector) model for improved Rust/code search using:
1. **Public data**: CodeSearchNet (2M pairs, 6 languages)
2. **Personal repos**: ~45 Rust repositories (Apache-licensed only)

**License**: All training data is Apache 2.0 or MIT licensed, so the resulting embeddings can be released publicly.

**Note**: Dropbox codebase training excluded (requires legal approval for embedding release).

## Training Data Sources

### 1. CodeSearchNet (Public)

| Property | Value |
|----------|-------|
| Size | ~20 GB (3.5 GB compressed) |
| Pairs | 2 million (docstring, code) pairs |
| Languages | Python, JavaScript, Ruby, Go, Java, PHP |
| Format | JSONLines |

**Download:**
```bash
# Python only (quick start)
wget https://s3.amazonaws.com/code-search-net/CodeSearchNet/v2/python.zip

# All languages
for lang in python javascript go java ruby php; do
  wget https://s3.amazonaws.com/code-search-net/CodeSearchNet/v2/${lang}.zip
done
```

### 2. Personal Rust Repositories (~45 repos)

**Estimated data:**
- ~45 repositories
- ~500-2000 files with doc comments
- ~5,000-20,000 (doc, function) pairs

**Extraction approach:**
- Parse `///` doc comments and pair with function bodies
- Extract `//!` module-level docs
- Use tree-sitter-rust for accurate AST parsing

---

## Training Data Format

All training data should be in JSONL format:

```json
{"query": "Calculate the sum of two numbers", "positive": "fn add(a: i32, b: i32) -> i32 { a + b }", "language": "rust"}
{"query": "Read file contents to string", "positive": "fn read_file(path: &Path) -> Result<String> { std::fs::read_to_string(path) }", "language": "rust"}
```

For contrastive learning, add hard negatives:
```json
{"query": "...", "positive": "...", "negative": "similar but wrong function", "language": "rust"}
```

---

## Rust Training Data Extraction

### Script: `scripts/extract_rust_training_data.py`

```python
#!/usr/bin/env python3
"""
Extract (doc comment, function) pairs from Rust code for training embeddings.

Usage:
    python extract_rust_training_data.py ~/my-repos --output training_data.jsonl
    python extract_rust_training_data.py ~/dropbox-code --output dropbox_data.jsonl
"""

import argparse
import json
import re
from pathlib import Path
from dataclasses import dataclass
from typing import Iterator

@dataclass
class TrainingPair:
    query: str          # Doc comment
    positive: str       # Function body
    file_path: str      # Source file
    func_name: str      # Function name
    language: str = "rust"

def extract_rust_pairs(file_path: Path) -> Iterator[TrainingPair]:
    """Extract (doc comment, function) pairs from a Rust file."""
    try:
        content = file_path.read_text(encoding='utf-8')
    except Exception:
        return

    # Pattern to match doc comments followed by function definitions
    # Handles both /// and /** */ style comments
    pattern = r'''
        # Doc comments (/// or //!)
        ((?:[ \t]*///[^\n]*\n)+)
        # Followed by attributes (optional)
        (?:[ \t]*#\[[^\]]*\]\n)*
        # Followed by pub/async/const/unsafe keywords (optional)
        [ \t]*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?(?:const\s+)?(?:unsafe\s+)?
        # Function definition
        fn\s+(\w+)\s*(?:<[^>]*>)?\s*\([^)]*\)(?:\s*->\s*[^{]+)?\s*\{
        # Function body (greedy match to closing brace - simplified)
        ([^}]*(?:\{[^}]*\}[^}]*)*)
        \}
    '''

    for match in re.finditer(pattern, content, re.MULTILINE | re.VERBOSE):
        doc_comment = match.group(1)
        func_name = match.group(2)
        func_body = match.group(3)

        # Clean doc comment: remove /// prefix and strip
        doc_lines = []
        for line in doc_comment.strip().split('\n'):
            line = line.strip()
            if line.startswith('///'):
                doc_lines.append(line[3:].strip())
            elif line.startswith('//!'):
                doc_lines.append(line[3:].strip())

        doc_text = ' '.join(doc_lines)

        # Skip if doc is too short or too long
        if len(doc_text) < 20 or len(doc_text) > 1000:
            continue

        # Skip test functions
        if func_name.startswith('test_'):
            continue

        # Reconstruct function for positive example
        full_func = f"fn {func_name}(...) {{ {func_body.strip()} }}"

        # Skip if function is too short
        if len(full_func) < 30:
            continue

        yield TrainingPair(
            query=doc_text,
            positive=full_func[:2000],  # Truncate long functions
            file_path=str(file_path),
            func_name=func_name,
        )

def extract_from_directory(root: Path, extensions: list[str] = ['.rs']) -> list[TrainingPair]:
    """Extract pairs from all Rust files in directory tree."""
    pairs = []
    for ext in extensions:
        for file_path in root.rglob(f'*{ext}'):
            # Skip target directories and tests
            if 'target' in file_path.parts:
                continue
            for pair in extract_rust_pairs(file_path):
                pairs.append(pair)
    return pairs

def main():
    parser = argparse.ArgumentParser(description='Extract Rust training data')
    parser.add_argument('directories', nargs='+', type=Path, help='Directories to scan')
    parser.add_argument('--output', '-o', type=Path, default=Path('rust_training_data.jsonl'))
    parser.add_argument('--min-pairs', type=int, default=100, help='Minimum pairs per repo to include')
    args = parser.parse_args()

    all_pairs = []
    for directory in args.directories:
        print(f"Scanning {directory}...")
        pairs = extract_from_directory(directory)
        print(f"  Found {len(pairs)} pairs")
        all_pairs.extend(pairs)

    # Write to JSONL
    with open(args.output, 'w') as f:
        for pair in all_pairs:
            f.write(json.dumps({
                'query': pair.query,
                'positive': pair.positive,
                'file_path': pair.file_path,
                'func_name': pair.func_name,
                'language': pair.language,
            }) + '\n')

    print(f"\nTotal: {len(all_pairs)} pairs written to {args.output}")

if __name__ == '__main__':
    main()
```

### Better Extraction with Tree-Sitter

For production use, use tree-sitter for accurate AST parsing:

```python
# Requires: pip install tree-sitter tree-sitter-rust
import tree_sitter_rust as ts_rust
from tree_sitter import Language, Parser

def extract_with_tree_sitter(file_path: Path) -> Iterator[TrainingPair]:
    """Use tree-sitter for accurate Rust parsing."""
    parser = Parser(ts_rust.language())

    content = file_path.read_bytes()
    tree = parser.parse(content)

    def find_functions(node):
        if node.type == 'function_item':
            # Find doc comment (previous sibling)
            doc = None
            prev = node.prev_named_sibling
            if prev and prev.type == 'line_comment':
                # Check if it's a doc comment
                text = content[prev.start_byte:prev.end_byte].decode()
                if text.startswith('///'):
                    doc = text

            if doc:
                func_text = content[node.start_byte:node.end_byte].decode()
                name = None
                for child in node.children:
                    if child.type == 'identifier':
                        name = content[child.start_byte:child.end_byte].decode()
                        break

                if name:
                    yield TrainingPair(
                        query=doc.replace('///', '').strip(),
                        positive=func_text,
                        file_path=str(file_path),
                        func_name=name,
                    )

        for child in node.children:
            yield from find_functions(child)

    yield from find_functions(tree.root_node)
```

---

## Training Configuration

### Phase 1: Public Data (CodeSearchNet)

Train on CodeSearchNet first to establish code understanding:

```yaml
# config/train_codesearchnet.yaml
model:
  base: "google/xtr-base-en"
  output: "xtr-code-base"

data:
  train: "data/codesearchnet_train.jsonl"
  valid: "data/codesearchnet_valid.jsonl"
  languages: ["python", "javascript", "go", "java"]

training:
  method: "lora"
  lora_r: 16
  lora_alpha: 32
  lora_dropout: 0.1
  target_modules: ["q_proj", "v_proj"]

  epochs: 3
  batch_size: 32
  learning_rate: 2e-5
  warmup_steps: 1000

  loss: "infonce"
  temperature: 0.07
  hard_negatives: 7
```

**Compute:**
- GPU: 1x A100 (40GB) or 4x V100
- Time: ~8-12 hours
- Storage: ~50GB

### Phase 2: Personal Rust Repos

Fine-tune the CodeSearchNet checkpoint on personal Rust code:

```yaml
# config/train_rust.yaml
model:
  base: "checkpoints/xtr-code-base"  # From Phase 1
  output: "xtr-rust-personal"

data:
  train: "data/rust_training_data.jsonl"  # From extraction script

training:
  method: "lora"
  lora_r: 8  # Smaller rank for fine-tuning
  epochs: 5  # More epochs for smaller dataset
  batch_size: 16
```

**Compute (M1 Mac):**
- Time: ~2-4 hours (MPS acceleration)
- Storage: ~5GB

---

## Training Script

```python
#!/usr/bin/env python3
"""
Fine-tune XTR for code search using LoRA.

Usage:
    python train_xtr_code.py --config config/train_codesearchnet.yaml
"""

import torch
from transformers import AutoModel, AutoTokenizer, get_scheduler
from peft import LoraConfig, get_peft_model
from torch.utils.data import DataLoader
import json

def load_training_data(path: str):
    """Load JSONL training data."""
    pairs = []
    with open(path) as f:
        for line in f:
            pairs.append(json.loads(line))
    return pairs

def infonce_loss(query_emb, positive_emb, negative_embs, temperature=0.07):
    """InfoNCE contrastive loss for multi-vector embeddings."""
    # For multi-vector: use MaxSim between query and positives/negatives

    # Compute similarity with positive
    # query_emb: [batch, query_tokens, dim]
    # positive_emb: [batch, doc_tokens, dim]
    pos_sim = maxsim_score(query_emb, positive_emb)  # [batch]

    # Compute similarity with negatives (in-batch negatives)
    # Stack all negatives: [batch * num_neg, doc_tokens, dim]
    neg_sims = []
    for neg_emb in negative_embs:
        neg_sims.append(maxsim_score(query_emb, neg_emb))
    neg_sims = torch.stack(neg_sims, dim=1)  # [batch, num_neg]

    # InfoNCE
    logits = torch.cat([pos_sim.unsqueeze(1), neg_sims], dim=1) / temperature
    labels = torch.zeros(logits.size(0), dtype=torch.long, device=logits.device)
    return torch.nn.functional.cross_entropy(logits, labels)

def maxsim_score(query_emb, doc_emb):
    """MaxSim scoring for multi-vector embeddings."""
    # query_emb: [batch, q_tokens, dim]
    # doc_emb: [batch, d_tokens, dim]

    # Compute all pairwise similarities
    sim = torch.bmm(query_emb, doc_emb.transpose(1, 2))  # [batch, q_tokens, d_tokens]

    # MaxSim: for each query token, find best-matching doc token
    max_sim = sim.max(dim=2).values  # [batch, q_tokens]

    # Sum over query tokens
    return max_sim.sum(dim=1)  # [batch]

def train(config):
    """Main training loop."""
    # Load base model
    model = AutoModel.from_pretrained(config['model']['base'])
    tokenizer = AutoTokenizer.from_pretrained(config['model']['base'])

    # Add LoRA adapters
    lora_config = LoraConfig(
        r=config['training']['lora_r'],
        lora_alpha=config['training']['lora_alpha'],
        target_modules=config['training']['target_modules'],
        lora_dropout=config['training']['lora_dropout'],
    )
    model = get_peft_model(model, lora_config)
    model.print_trainable_parameters()

    # Load data
    train_data = load_training_data(config['data']['train'])

    # Training loop
    optimizer = torch.optim.AdamW(model.parameters(), lr=config['training']['learning_rate'])

    for epoch in range(config['training']['epochs']):
        model.train()
        total_loss = 0

        for batch in DataLoader(train_data, batch_size=config['training']['batch_size'], shuffle=True):
            # Tokenize queries and positives
            query_inputs = tokenizer(batch['query'], return_tensors='pt', padding=True, truncation=True)
            pos_inputs = tokenizer(batch['positive'], return_tensors='pt', padding=True, truncation=True)

            # Get embeddings
            query_emb = model(**query_inputs).last_hidden_state
            pos_emb = model(**pos_inputs).last_hidden_state

            # In-batch negatives: use other positives as negatives
            neg_embs = [pos_emb[torch.arange(len(pos_emb)) != i] for i in range(len(pos_emb))]

            # Compute loss
            loss = infonce_loss(query_emb, pos_emb, neg_embs, config['training']['temperature'])

            optimizer.zero_grad()
            loss.backward()
            optimizer.step()

            total_loss += loss.item()

        print(f"Epoch {epoch+1}: loss = {total_loss:.4f}")

    # Save LoRA weights only (~2-5MB)
    model.save_pretrained(config['model']['output'])
    print(f"Saved to {config['model']['output']}")

if __name__ == '__main__':
    import yaml
    import argparse

    parser = argparse.ArgumentParser()
    parser.add_argument('--config', required=True)
    args = parser.parse_args()

    with open(args.config) as f:
        config = yaml.safe_load(f)

    train(config)
```

---

## Directory Structure

```
~/sg/
├── scripts/
│   ├── extract_rust_training_data.py
│   ├── train_xtr_code.py
│   └── download_codesearchnet.sh
├── config/
│   ├── train_codesearchnet.yaml
│   └── train_rust.yaml
├── data/
│   ├── codesearchnet_train.jsonl
│   └── rust_training_data.jsonl
└── checkpoints/
    ├── xtr-code-base/
    └── xtr-rust/
```

---

## Execution Plan

### Step 1: Download CodeSearchNet (1 hour)
```bash
cd ~/sg-training-data
./scripts/download_codesearchnet.sh
```

### Step 2: Extract Rust Training Data (30 min)
```bash
# From personal Apache-licensed repos (all 45 repos)
python scripts/extract_rust_training_data.py \
    ~/sg ~/rust-warp ~/docling_rs ~/pdfium_fast \
    --output data/rust_training_data.jsonl
```

### Step 3: Train on CodeSearchNet (8-12 hours on GPU)
```bash
python scripts/train_xtr_code.py --config config/train_codesearchnet.yaml
```

### Step 4: Fine-tune on Rust (2-4 hours on M1)
```bash
python scripts/train_xtr_code.py --config config/train_rust.yaml
```

### Step 5: Evaluate
```bash
sg eval --model xtr-rust --spec eval/code_queries.json
```

---

## Expected Results

| Stage | Model | Code P@1 (est.) |
|-------|-------|-----------------|
| Baseline | XTR | 0.60 |
| + CodeSearchNet | xtr-code-base | 0.75 |
| + Personal Rust | xtr-rust | 0.85+ |

---

## Next Steps

1. Clone/sync all 45 personal Rust repos to local machine
2. Run extraction script to generate training data
3. Start training on CodeSearchNet
4. Fine-tune on personal Rust code
5. Evaluate and iterate
6. Release model weights (Apache-licensed training data = releasable)
