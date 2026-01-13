# Training Data for Embedding Fine-Tuning

## Available Datasets

### 1. CodeSearchNet (Recommended for Code Search)

**Best for:** Fine-tuning code search embeddings

| Property | Value |
|----------|-------|
| Size | ~20 GB (3.5 GB compressed) |
| Pairs | 2 million (docstring, code) pairs |
| Languages | Python, JavaScript, Ruby, Go, Java, PHP |
| Format | JSONLines |
| License | MIT |

**Download:**
```bash
# Direct S3 download per language
wget https://s3.amazonaws.com/code-search-net/CodeSearchNet/v2/python.zip
wget https://s3.amazonaws.com/code-search-net/CodeSearchNet/v2/javascript.zip
wget https://s3.amazonaws.com/code-search-net/CodeSearchNet/v2/go.zip
wget https://s3.amazonaws.com/code-search-net/CodeSearchNet/v2/java.zip
wget https://s3.amazonaws.com/code-search-net/CodeSearchNet/v2/ruby.zip
wget https://s3.amazonaws.com/code-search-net/CodeSearchNet/v2/php.zip

# Or via HuggingFace
pip install datasets
python -c "from datasets import load_dataset; ds = load_dataset('code_search_net', 'python')"
```

**Format:**
```json
{
  "docstring": "Calculate the sum of two numbers",
  "code": "def add(a, b):\n    return a + b",
  "language": "python",
  "func_name": "add"
}
```

**Subset for Quick Testing:**
```python
from datasets import load_dataset
ds = load_dataset('code_search_net', 'python', split='train[:10000]')
# 10K pairs, ~50MB
```

---

### 2. The Stack (Large-Scale Pretraining)

**Best for:** Large-scale pretraining (too big for fine-tuning)

| Property | Value |
|----------|-------|
| Size | 6+ TB |
| Files | 5.28 billion |
| Languages | 358 programming languages |
| License | Permissive (varies by repo) |

**Not recommended for fine-tuning** - too large. Use CodeSearchNet instead.

---

### 3. CoSQA (Web Queries → Code)

**Best for:** Natural language query → code retrieval

| Property | Value |
|----------|-------|
| Size | ~20K pairs |
| Type | Web search queries paired with code |
| Language | Python |
| Source | Microsoft CodeXGLUE |

**Download:**
```bash
git clone https://github.com/microsoft/CodeXGLUE
cd CodeXGLUE/Text-Code/NL-code-search-WebQuery
```

---

### 4. Local Corpus (Your Codebase)

**Best for:** Project-specific fine-tuning

**Auto-extraction script:**
```python
import ast
import json
from pathlib import Path

def extract_pairs(file_path: Path) -> list:
    """Extract (docstring, function) pairs from Python file."""
    pairs = []
    try:
        with open(file_path) as f:
            tree = ast.parse(f.read())

        for node in ast.walk(tree):
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                docstring = ast.get_docstring(node)
                if docstring and len(docstring) > 20:
                    # Remove docstring from body for clean code
                    code = ast.unparse(node)
                    pairs.append({
                        "query": docstring,
                        "code": code,
                        "func_name": node.name,
                        "file": str(file_path)
                    })
    except:
        pass
    return pairs

def extract_from_directory(root: Path, extensions=['.py']) -> list:
    """Extract pairs from all files in directory."""
    all_pairs = []
    for ext in extensions:
        for file in root.rglob(f'*{ext}'):
            all_pairs.extend(extract_pairs(file))
    return all_pairs

# Usage
pairs = extract_from_directory(Path('~/my-project'))
with open('training_pairs.jsonl', 'w') as f:
    for pair in pairs:
        f.write(json.dumps(pair) + '\n')
print(f"Extracted {len(pairs)} pairs")
```

**For Rust (using tree-sitter):**
```python
import tree_sitter_rust as ts_rust
from tree_sitter import Language, Parser

# Parse Rust and extract doc comments + functions
# (Implementation would use tree-sitter for AST parsing)
```

---

## Recommended Training Configurations

### Quick Test (30 min on M1 Mac)
```
Dataset: CodeSearchNet Python (10K pairs)
Model: XTR or UniXcoder
Method: LoRA (r=8)
Epochs: 1
Batch size: 8
```

### Full Fine-Tune (4-8 hours on M1 Mac)
```
Dataset: CodeSearchNet (all languages, 2M pairs)
Model: XTR
Method: LoRA (r=16)
Epochs: 3
Batch size: 32
```

### Local Corpus Tune (2-4 hours)
```
Dataset: Your codebase (1K-10K pairs)
Model: Pre-trained XTR or UniXcoder
Method: LoRA (r=16)
Epochs: 5-10 (small data needs more epochs)
Batch size: 16
```

---

## Training Script Outline

```python
from transformers import AutoModel, AutoTokenizer
from peft import LoraConfig, get_peft_model
import torch

# Load base model
model = AutoModel.from_pretrained("google/xtr-base-en")
tokenizer = AutoTokenizer.from_pretrained("google/xtr-base-en")

# Add LoRA adapters
lora_config = LoraConfig(
    r=16,
    lora_alpha=32,
    target_modules=["q_proj", "v_proj"],
    lora_dropout=0.1,
)
model = get_peft_model(model, lora_config)

# Training loop with contrastive loss
def contrastive_loss(query_emb, positive_emb, negative_embs, temperature=0.07):
    """InfoNCE contrastive loss."""
    pos_sim = torch.cosine_similarity(query_emb, positive_emb)
    neg_sims = torch.stack([
        torch.cosine_similarity(query_emb, neg)
        for neg in negative_embs
    ])
    logits = torch.cat([pos_sim.unsqueeze(0), neg_sims]) / temperature
    labels = torch.zeros(1, dtype=torch.long)
    return F.cross_entropy(logits.unsqueeze(0), labels)

# Save LoRA weights only (~2MB vs 500MB full model)
model.save_pretrained("xtr-code-lora")
```

---

## Disk Space Requirements

| Dataset | Compressed | Uncompressed |
|---------|------------|--------------|
| CodeSearchNet (all) | 3.5 GB | ~20 GB |
| CodeSearchNet (Python only) | 600 MB | ~3 GB |
| CoSQA | 50 MB | 200 MB |
| Local corpus (typical) | N/A | 10-100 MB |
| LoRA checkpoint | N/A | 2-10 MB |

---

## Download Script

```bash
#!/bin/bash
# download_training_data.sh

mkdir -p ~/sg-training-data
cd ~/sg-training-data

echo "Downloading CodeSearchNet Python subset..."
wget -q https://s3.amazonaws.com/code-search-net/CodeSearchNet/v2/python.zip
unzip -q python.zip
rm python.zip

echo "Downloading CoSQA..."
git clone --depth 1 https://github.com/microsoft/CodeXGLUE
mv CodeXGLUE/Text-Code/NL-code-search-WebQuery cosqa
rm -rf CodeXGLUE

echo "Done! Training data in ~/sg-training-data/"
du -sh ~/sg-training-data/*
```

---

## Next Steps

1. **Download CodeSearchNet Python** (~600MB) for quick experiments
2. **Extract pairs from your codebase** using the script above
3. **Run LoRA fine-tuning** (see EMBEDDING_ROADMAP.md Phase 3)
4. **Evaluate** on sg's code corpus to measure improvement
