# Training Data Plan: Rust + Formal Verification Code Search

## Goal

Create embeddings optimized for searching:
1. **Rust code** - general purpose
2. **Formal verification code** - Kani proofs, TLA+ specs, Verus, Prusti
3. **Proof documentation** - understanding what is being verified

## Training Data Sources

### Tier 1: Critical (Must Have)

| Source | Type | Est. Pairs | License |
|--------|------|------------|---------|
| Kani (model-checking/kani) | Rust model checker | 500-1,000 | Apache-2.0/MIT |
| Verus (verus-lang/verus) | Rust verification | 500-1,000 | MIT |
| Rustc (rust-lang/rust) | Compiler + stdlib | 5,000-10,000 | Apache-2.0/MIT |
| Your repos (sg, docling, pdfium) | Domain code | 3,000 | Apache-2.0/MIT |

### Tier 2: High Priority

| Source | Type | Est. Pairs | License |
|--------|------|------------|---------|
| Prusti (viperproject/prusti-dev) | Rust verifier | 500-1,000 | MPL-2.0 |
| Creusot (creusot-rs/creusot) | Rust proofs | 300-500 | LGPL |
| TLA+ toolbox | Spec language tools | 200-500 | MIT |
| Apalache | TLA+ model checker | 200-400 | Apache-2.0 |

### Tier 3: Nice to Have

| Source | Type | Est. Pairs | License |
|--------|------|------------|---------|
| Lean4 | Theorem prover | 1,000-2,000 | Apache-2.0 |
| Z3 (Rust bindings) | SMT solver | 200-500 | MIT |
| CodeSearchNet | General code | 2M (sampled) | MIT |

## Training Pair Types

### Type 1: Doc Comment → Function (Standard)

```json
{
  "query": "Verify that the given memory region is valid and accessible",
  "positive": "#[kani::proof]\nfn check_memory_valid() {\n    let ptr = kani::any::<*const u8>();\n    kani::assume(!ptr.is_null());\n    kani::assert(unsafe { *ptr } >= 0);\n}",
  "language": "rust",
  "category": "verification"
}
```

### Type 2: File Path Context (NEW)

Include file path as semantic signal:

```json
{
  "query": "kani proof memory safety bounds checking",
  "positive": "fn verify_bounds() { ... }",
  "file_path": "kani/library/kani/src/mem.rs",
  "file_context": "kani/mem.rs"
}
```

**Why?** File names like `kani_proofs.rs`, `invariants.rs`, `model_checking.rs` carry strong semantic signal.

### Type 3: README → Code Linking (NEW)

Link documentation to implementation:

```json
{
  "query": "Kani uses CBMC bounded model checker to verify Rust programs have no undefined behavior",
  "positive": "#[kani::proof]\npub fn verify_no_ub() { ... }",
  "source": "README.md"
}
```

### Type 4: Proof-Specific Patterns (NEW)

Augment with verification-specific queries:

```json
{
  "query": "prove absence of panic unwinding",
  "positive": "#[kani::proof]\n#[kani::unwind(10)]\nfn verify_no_panic() { ... }"
}
```

## Extraction Pipeline

### Step 1: Clone Repositories

```bash
# Formal verification (critical)
git clone --depth 1 https://github.com/model-checking/kani.git
git clone --depth 1 https://github.com/verus-lang/verus.git
git clone --depth 1 https://github.com/viperproject/prusti-dev.git
git clone --depth 1 https://github.com/creusot-rs/creusot.git

# TLA+
git clone --depth 1 https://github.com/tlaplus/tlaplus.git
git clone --depth 1 https://github.com/informalsystems/apalache.git

# Core Rust
git clone --depth 1 https://github.com/rust-lang/rust.git

# Theorem provers
git clone --depth 1 https://github.com/leanprover/lean4.git
git clone --depth 1 https://github.com/Z3Prover/z3.git
```

### Step 2: Extract Doc→Code Pairs

```bash
python scripts/extract_rust_training_data.py \
    ~/training-repos/kani \
    ~/training-repos/verus \
    ~/training-repos/prusti \
    ~/training-repos/creusot \
    ~/training-repos/rustc \
    ~/sg ~/docling_rs ~/pdfium_fast/rust \
    -o data/rust_verification_training.jsonl
```

### Step 3: Extract README→Code Pairs

```bash
# Extract from all repos (1,099 pairs from 88 repos)
python scripts/extract_readme_pairs.py \
    ~/training-repos/* \
    -o data/readme_pairs_all.jsonl \
    --check-license
```

**Results (2025-12-31):** 1,099 pairs from 88 Apache/MIT/BSD repos
- Top: bevy (485), dsp_rs (277), rustc (65), langchain_rs (61), wasmtime (30)

### Step 4: Add File Path Context

`scripts/extract_rust_training_data.py` now emits `file_context` with the
repo name + relative path (example: `kani/library/kani/src/mem.rs`) so file
name semantics are included in training pairs.

### Step 5: Combine and Deduplicate

```bash
cat data/*.jsonl | python scripts/deduplicate.py > data/combined_training.jsonl
```

## What NOT to Include

| Content | Include? | Reason |
|---------|----------|--------|
| arXiv papers | NO | Wrong modality (PDF text vs code) |
| Test files | LIMITED | Mostly boilerplate, but some proof tests are valuable |
| Generated code | NO | Not human-written, low quality |
| Vendored deps | NO | Duplicates, not project-specific |

## Available Training Data (2025-12-31)

| File | Pairs | Size | Description |
|------|-------|------|-------------|
| `data/training_data_full.jsonl` | 127,368 | ~130MB | **Full multi-language corpus** |
| `data/training_data_200k.jsonl` | 113,849 | 119MB | Extended Rust+Python corpus |
| `data/lean_java_training.jsonl` | 12,255 | ~15MB | Lean 4 + TLA+ Java tools |
| `data/rust_training_100k.jsonl` | 93,960 | 90MB | Full 91-repo extraction (tree-sitter) |
| `data/apache_mit_training.jsonl` | 5,133 | 7MB | License-filtered subset |
| `data/readme_pairs_all.jsonl` | 1,099 | 3MB | README→code pairs |
| `data/verification_training.jsonl` | 2,219 | 3MB | Kani/Verus/Prusti pairs |

### Full Multi-Language Corpus (127K) - By Language

| Language | Pairs | Focus Areas |
|----------|-------|-------------|
| **Rust** | 96,193 | Core focus: verification, systems, web |
| **Python** | 17,644 | ML, utilities, scripts |
| **Lean** | 9,316 | Theorem proving, formal proofs |
| **Java** | 2,939 | TLA+ tools (model checking) |
| **TypeScript** | 766 | Frontend, tooling |
| **C++** | 510 | Z3 SMT solver |

### Extended Corpus (113K) - Top Sources

| Repository | Pairs | Type |
|------------|-------|------|
| servo | 8,710 | Browser engine |
| rustc | 6,974 | Compiler |
| dashflow | 5,818 | Internal |
| wasmtime | 5,463 | WASM runtime |
| RustPython | 5,164 | Python in Rust |
| substrate | 5,058 | Blockchain |
| bevy | 4,953 | Game engine |
| polars | 4,280 | DataFrame |
| dashprove | 4,125 | Internal |
| arrow-rs | 3,528 | Arrow |
| embassy | 2,812 | Embedded |
| polkadot | 2,184 | Blockchain |
| solana | 2,033 | Blockchain |

## Training Configurations

### Option 1: Quick Training (Recommended)

Train on 5,133 license-verified pairs. Best balance of quality and speed.

```bash
# Config: config/train_rust_direct.yaml
# Time: ~40-60 min on M1 Mac (MPS)
# Result: P@1 = 0.93 hybrid, P@1 = 0.67 semantic
python scripts/train_xtr_code.py --config config/train_rust_direct.yaml
python scripts/merge_lora.py checkpoints/xtr-rust-direct -o checkpoints/xtr-rust-merged
```

### Option 2: Large-Scale Training (93K)

Train on 93,960 pairs from 91 repositories. May improve semantic-only retrieval.

```bash
# Config: config/train_rust_100k.yaml
# Time: ~3-6 hours on M1 Mac (MPS), ~1-2 hours on GPU
# Expected: Better generalization across diverse Rust code
python scripts/train_xtr_code.py --config config/train_rust_100k.yaml
python scripts/merge_lora.py checkpoints/xtr-rust-100k -o checkpoints/xtr-rust-100k-merged
```

### Option 3: Extended Training (113K)

Train on 113,849 pairs from 95+ repositories including additional WASM, embedded, and blockchain code.

```bash
# Config: config/train_rust_extended.yaml
# Time: ~4-8 hours on M1 Mac (MPS), ~2-3 hours on GPU
# Expected: Broader coverage of Rust ecosystem
python scripts/train_xtr_code.py --config config/train_rust_extended.yaml
python scripts/merge_lora.py checkpoints/xtr-rust-extended -o checkpoints/xtr-rust-extended-merged
```

### Option 4: Full Multi-Language (127K)

Train on the combined Rust + Python + Lean + Java + TypeScript + C++ corpus.

```bash
# Config: config/train_multilang_full.yaml
# Time: ~4-8 hours on M1 Mac (MPS), ~2-3 hours on GPU
# Expected: Broader multilingual/generalization coverage beyond Rust-only
python scripts/train_xtr_code.py --config config/train_multilang_full.yaml
python scripts/merge_lora.py checkpoints/xtr-multilang-full -o checkpoints/xtr-multilang-full-merged
```

### Option 5: CodeSearchNet (Best Quality, Requires GPU)

First train on CodeSearchNet, then fine-tune on local code.

```bash
# Config: config/train_codesearchnet.yaml
# Time: ~8-12 hours on A100
# Requires: 50GB CodeSearchNet download
```

## Current Results (2025-12-31)

| Model | Training Data | Mode | P@1 | MRR |
|-------|---------------|------|-----|-----|
| XTR (base) | none | semantic | 0.50 | 0.68 |
| XTR (base) | none | hybrid | 0.60 | 0.80 |
| XTR (fine-tuned) | 5,133 pairs | semantic | 0.67 | 0.78 |
| **XTR (fine-tuned)** | **5,133 pairs** | **hybrid** | **0.93** | **0.97** |
| UniXcoder | n/a | hybrid | 0.93 | 0.97 |

**Target achieved:** P@1 = 0.93 (target was 0.80)

## Target Metrics

| Metric | Target | Achieved |
|--------|--------|----------|
| Total pairs | 100,000-200,000 | **127,368** |
| Rust pairs | 50,000+ | **96,193** |
| Lean/Formal proofs | 5,000+ | **9,316** |
| TLA+ tooling (Java) | 1,000+ | **2,939** |
| Kani/verification pairs | 2,000+ | **2,219** |
| Languages covered | 4+ | **6** (Rust, Python, Lean, Java, TS, C++) |
| Avg query length | 20-100 chars | ✓ |
| Avg code length | 100-2000 chars | ✓ |

## Questions Resolved

1. **File names in search?** YES - add as context field
2. **arXiv papers?** NO - wrong modality for code search
3. **Markdown docs?** YES - README sections as queries
4. **TLA+ specs?** MAYBE - as separate category, not mixed with Rust
