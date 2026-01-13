# Embedding Model Roadmap

## Current State

| Content | Model | Type | P@1 | Auto-Route |
|---------|-------|------|-----|------------|
| English Prose | XTR | Multi-vector (MaxSim) | 1.00 | Yes |
| Code | UniXcoder | Single-vector (Cosine) | 0.93 | Yes |
| Code | Jina-Code | Single-vector (Mean) | 0.80 | No |
| Japanese/CJK | Jina-ColBERT | Multi-vector (MaxSim) | 1.00 | Yes |
| PDF | XTR | Multi-vector | 1.00 | No |

**Note:** Code P@1 improved from 0.80 to 0.93 with filename relevance boost (#559).

**Auto-routing:** Queries with CJK characters → Jina-ColBERT, code patterns → UniXcoder, text → XTR

## Phase 1: Add Jina-ColBERT-v2 for Multilingual

**Model:** `jinaai/jina-colbert-v2`
**Type:** Multi-vector (late interaction)
**Languages:** 94 languages including Japanese, Chinese, Korean
**Context:** 8192 tokens (vs XTR's 512)

### Benefits
- Native CJK tokenization (not relying on keyword fallback)
- Longer context for documents
- Multi-vector MaxSim scoring (same architecture as XTR)

### Implementation
```rust
// Add to crates/sg-core/src/embedder_jina_colbert.rs
pub struct JinaColBertEmbedder {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
}

impl EmbedderBackend for JinaColBertEmbedder {
    fn embed_document(&mut self, text: &str) -> Result<EmbeddingResult>;
    fn embed_query(&mut self, text: &str) -> Result<EmbeddingResult>;
}
```

### Files to Modify
- `crates/sg-core/src/embedder.rs` - Add `EmbeddingModel::JinaColBert`
- `crates/sg-core/src/lib.rs` - Export new module
- `crates/sg/src/main.rs` - Add CLI option `--model jina-colbert`

---

## Phase 2: Code Embedding Improvements

### Option A: Use Better Single-Vector Code Model

Replace UniXcoder with newer code models:

| Model | Params | Languages | Context |
|-------|--------|-----------|---------|
| UniXcoder (current) | 125M | 6 | 512 |
| CodeT5+ | 110M | 9 | 512 |
| Jina-Code-v2 | 137M | 30+ | 8192 |
| SFR-Code-400M | 400M | 20+ | 8192 |

**Recommendation:** Try Jina-Code-v2 for longer context and more languages.

### Option B: Fine-tune XTR on Code (Multi-Vector)

Create a code-specific multi-vector model by fine-tuning XTR.

**Training Data:**

| Dataset | Size | Description |
|---------|------|-------------|
| CodeSearchNet | 2M pairs | Code-docstring pairs, 6 languages |
| CoSQA | 20K | Web queries → code |
| StackOverflow | 500K+ | Q&A pairs |
| GitHub Code | 10M+ | Function-docstring extraction |

**Training Approach:**
```python
# Using PyLate library for late-interaction training
from pylate import ColBERT, Trainer

model = ColBERT.from_pretrained("google/xtr-base-en")
trainer = Trainer(
    model=model,
    train_dataset=codesearchnet,
    loss="contrastive",
    mining="hard_negative",
)
trainer.train(epochs=3, batch_size=32)
model.save("xtr-code-v1")
```

**Compute Requirements:**
- GPU: 1x A100 (40GB) or 4x V100
- Time: ~8-12 hours for CodeSearchNet
- Storage: ~50GB for datasets + checkpoints

---

## Phase 3: Local Corpus Tuning System

Allow users to fine-tune embeddings on their own codebase overnight.

### Architecture

```
User's Codebase
      ↓
[Automatic Mining]
  - Extract function → docstring pairs
  - Extract variable → type pairs
  - Extract import → usage pairs
      ↓
[Hard Negative Mining]
  - Find similar but different functions
  - Same file, different functions
      ↓
[Contrastive Training]
  - LoRA fine-tuning (small delta)
  - ~1000 training pairs minimum
      ↓
[Local Model Checkpoint]
  ~/.cache/sg/models/local/my-project-v1/
```

### CLI Interface

**Option A: Direct fine-tuning (recommended for most users)**

Fine-tune directly from XTR base on your local code. No CodeSearchNet download required.

```bash
# Step 1: Extract training data from Rust code
python scripts/extract_rust_training_data.py ~/my-project -o data/rust_training_data.jsonl

# Step 2: Install training dependencies
pip install -r scripts/requirements.txt

# Step 3: Fine-tune (~30-60 min on M1 Mac with MPS)
python scripts/train_xtr_code.py --config config/train_rust_direct.yaml

# Step 4: Merge LoRA adapters into base model
python scripts/merge_lora.py checkpoints/xtr-rust-direct -o checkpoints/xtr-rust-merged

# Step 5: Use tuned model for indexing
sg index ~/my-project --model-path checkpoints/xtr-rust-merged
```

**Option B: Two-phase training (best quality, requires GPU)**

First train on CodeSearchNet, then fine-tune on local code:

```bash
# Phase 1: Train on CodeSearchNet (8-12 hours on GPU)
wget https://s3.amazonaws.com/code-search-net/CodeSearchNet/v2/python.zip
python scripts/train_xtr_code.py --config config/train_codesearchnet.yaml

# Phase 2: Fine-tune on local code
python scripts/train_xtr_code.py --config config/train_rust.yaml

# Merge and use
python scripts/merge_lora.py checkpoints/xtr-rust -o checkpoints/xtr-rust-merged
sg index ~/my-project --model-path checkpoints/xtr-rust-merged
```

### Training Data Generation

```rust
// Auto-extract training pairs from code
struct TrainingPair {
    query: String,      // Docstring, comment, or function name
    positive: String,   // Function body
    negative: String,   // Similar but different function
}

fn extract_pairs(file: &Path) -> Vec<TrainingPair> {
    // 1. Parse AST
    // 2. Extract function → docstring pairs
    // 3. Extract class → method pairs
    // 4. Mine hard negatives (same file, different function)
}
```

### Minimum Requirements

**Option A (Direct fine-tuning):**

| Requirement | Value |
|-------------|-------|
| Training pairs | 500+ (596 extracted from sg codebase) |
| GPU/MPS | Optional (MPS recommended on Mac) |
| Time (M1 Mac) | ~30-60 minutes |
| Disk space | ~2GB |

**Option B (Two-phase training):**

| Requirement | Value |
|-------------|-------|
| Training pairs | 2M+ from CodeSearchNet |
| GPU | Required (A100 or 4x V100) |
| Time | ~8-12 hours |
| Disk space | ~50GB |

### LoRA Configuration

```python
lora_config = {
    "r": 16,           # Rank
    "alpha": 32,       # Scaling
    "dropout": 0.1,
    "target_modules": ["q", "v"],  # T5 attention layers (not q_proj/v_proj)
}
# Results in ~2MB checkpoint (vs 420MB encoder model)
```

---

## Implementation Priority

1. **Phase 1: Jina-ColBERT-v2** (1-2 days)
   - Immediate improvement for Japanese/CJK
   - Same architecture as XTR (easy integration)

2. **Phase 2a: Try Jina-Code-v2** (1 day)
   - Quick win for code search
   - 8K context helps with large files

3. **Phase 3: Local Tuning** (3-5 days)
   - Training data extraction
   - LoRA fine-tuning pipeline
   - CLI integration

4. **Phase 2b: XTR-Code** (1-2 weeks)
   - Requires training infrastructure
   - Best long-term solution for code

---

## Datasets for Code Training

### Public Datasets

| Dataset | URL | Size | Use |
|---------|-----|------|-----|
| CodeSearchNet | github.com/github/CodeSearchNet | 2M | Primary training |
| CoSQA | github.com/microsoft/CodeXGLUE | 20K | Web query → code |
| CodeXGLUE | github.com/microsoft/CodeXGLUE | 14 tasks | Benchmark |
| The Stack | huggingface.co/datasets/bigcode/the-stack | 6TB | Large-scale pretraining |

### Synthetic Generation

```python
# Generate query-code pairs from docstrings
def generate_pairs(code_file):
    tree = ast.parse(code_file)
    pairs = []
    for node in ast.walk(tree):
        if isinstance(node, ast.FunctionDef):
            if node.docstring:
                pairs.append({
                    "query": node.docstring,
                    "code": ast.unparse(node),
                })
    return pairs
```

---

## Success Metrics

| Metric | Current | Target | Status |
|--------|---------|--------|--------|
| Code P@1 (XTR) | 0.70 | 0.80+ | Fine-tuned hybrid |
| Code P@1 (UniXcoder) | **0.93** | 0.90+ | **ACHIEVED** (+filename boost) |
| Japanese P@1 (Jina-ColBERT) | **1.00** | 1.00 | **ACHIEVED** |
| Japanese P@1 (XTR semantic-only) | 0.67 | - | (baseline) |
| Local-tuned P@1 | **+20%** | +10% vs base | **ACHIEVED** |

### Phase 1 Results (2025-12-31)

**Jina-ColBERT-v2 on Japanese corpus:**
- P@1: 1.00 (6/6 queries)
- MRR: 1.00
- Mode: Semantic-only (no hybrid search needed)

**Comparison with XTR (same corpus):**
- XTR semantic-only: P@1 = 0.67, MRR = 0.83
- XTR + hybrid: P@1 = 1.00 (requires keyword fallback)

**Key finding:** Jina-ColBERT-v2 achieves perfect Japanese retrieval without hybrid search, demonstrating native CJK tokenization superiority over XTR.

### Phase 2a Results (2025-12-31)

**Jina-Code-v2 on code corpus:**

| Model | Mode | P@1 | MRR |
|-------|------|-----|-----|
| UniXcoder | semantic | 0.60 | 0.74 |
| UniXcoder | hybrid | **0.80** | 0.88 |
| Jina-Code | semantic | **0.70** | 0.83 |
| Jina-Code | hybrid | **0.80** | 0.90 |

**Key findings:**
- Jina-Code semantic-only P@1 = 0.70 is 17% better than UniXcoder (0.60)
- Both achieve P@1 = 0.80 with hybrid search
- Jina-Code has 16x longer context (8192 vs 512 tokens)
- Jina-Code is useful for longer code files or when hybrid isn't available

### Phase 2b Status (2025-12-31) - XTR LoRA Fine-Tuning COMPLETE

**Training Data:**
- Original: 2,914 pairs from docling_rs (1,446), pdfium_fast (963), sg (670)
- Expanded: 5,133 pairs adding kani (1,505), verus (714) - all Apache/MIT licensed

**Training Results:**
- Training time: ~40 minutes on M1 Mac (MPS)
- Initial loss: 0.16 → Final loss: 0.02 (88% reduction)
- Output: 418MB merged model in `checkpoints/xtr-rust-merged/`

**Evaluation Results:**

| Model | Mode | P@1 | MRR | Notes |
|-------|------|-----|-----|-------|
| XTR (base) | semantic | 0.50 | 0.68 | Baseline |
| XTR (596 pairs) | semantic | 0.60 | 0.74 | +20% P@1 |
| **XTR (2914 pairs)** | **semantic** | **0.73** | **0.84** | **+46% P@1** |
| XTR (base) | hybrid | 0.60 | 0.80 | |
| XTR (596 pairs) | hybrid | 0.70 | 0.85 | +17% P@1 |
| **XTR (2914 pairs)** | **hybrid** | **0.93** | **0.97** | **+55% P@1** |
| UniXcoder | hybrid | 0.93 | 0.97 | (+filename boost) |
| XTR (5133 pairs) | semantic | 0.67 | 0.78 | More diverse training |
| XTR (5133 pairs) | hybrid | 0.93 | 0.97 | +verification repos |

**Key findings:**
- Larger training data (5x) significantly improves results
- Fine-tuned XTR (2914 pairs) **matches UniXcoder+hybrid** at P@1=0.93
- Semantic-only improved by **+46%** over base (0.50→0.73)
- Hybrid improved by **+55%** over base (0.60→0.93)
- Adding verification repos (5133 pairs) maintains hybrid P@1=0.93 but reduces semantic-only (more general model)

**CLI support:**
- `--model-path` option added to `sg index` and `sg eval`
- Usage: `sg index --model-path checkpoints/xtr-rust-merged ~/my-project`
- Usage: `sg eval --model-path checkpoints/xtr-rust-merged --spec eval/code_queries.json`

**Infrastructure:**
- Training script: `scripts/train_xtr_code.py` (T5EncoderModel + LoRA)
- Training data extraction: `scripts/extract_rust_training_data.py`
- Merge script: `scripts/merge_lora.py`
- Configs: `config/train_rust_direct.yaml`, `config/train_codesearchnet.yaml`

### Phase 3: 138K Multi-Language Training (IN PROGRESS)

**Training Data (138,122 pairs):**

| Language | Pairs | Source |
|----------|-------|--------|
| Rust | 96,193 | crates.io (Apache/MIT repos) |
| Python | 25,194 | PyTorch, popular ML libraries |
| Lean | 9,316 | mathlib4, formal proofs |
| Swift | 3,191 | Swift stdlib, open-source iOS |
| Java | 2,939 | Android, popular libraries |
| TypeScript | 766 | Popular npm packages |
| C++ | 510 | LLVM, open-source projects |
| ObjC | 13 | Apple open-source |

**Training Configuration:**
- Config: `config/train_extended_138k.yaml`
- Script: `scripts/train_xtr_optimized.py`
- LoRA modules: q, v, k, o, wi_0, wi_1, wo (full T5 attention + MLP)
- Batch size: 8, gradient accumulation: 8 (effective batch: 64)
- Learning rate: 2e-5 with 1000 warmup steps
- Epochs: 2, total steps: ~4,316

**Current Status:**
- Training started: 2025-12-31 15:12
- Device: MPS (Apple M1)
- Estimated time: ~12-14 hours

**Early Training Metrics (warmup phase):**

| Step | Loss | Avg Loss | LR | Speed |
|------|------|----------|-----|-------|
| 50 | 7.47 | 6.92 | 1e-6 | 4.7 |
| 100 | 6.32 | 6.94 | 2e-6 | 5.3 |
| 150 | 6.71 | 6.93 | 3e-6 | 5.5 |

**Extraction Scripts:**
- `scripts/extract_rust_training_data.py` - Rust doc-comment/function pairs
- `scripts/extract_python_training_data.py` - Python docstring/function pairs
- `scripts/extract_lean_java.py` - Lean/Java extraction
- `scripts/extract_swift_objc.py` - Swift/ObjC extraction
- `scripts/extract_typescript.py` - TypeScript extraction
- `scripts/extract_cpp.py` - C++ extraction
