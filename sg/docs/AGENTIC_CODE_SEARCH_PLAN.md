# State-of-the-Art Code Search for Agentic Systems

## Executive Summary

**Goal:** Build a code search system that enables AI agents to find exactly the right code on the first query.

**Current State:**
- P@1 = 0.93 on your Rust codebase (UniXcoder + hybrid + filename boost)
- 138K training pairs (Rust, Python, Lean, Java, TypeScript, C++, Swift, ObjC)
- LoRA fine-tuning infrastructure working on M1 Mac (MPS)

**Gap to SOTA:**
| Benchmark | Training Size | Your Data |
|-----------|---------------|-----------|
| CodeSearchNet | 2M pairs | 138K pairs |
| CodeBERT | 6.4M bimodal | - |
| UniXcoder | 9.5M pairs | - |
| StarCoder | 15B tokens | - |

**Recommendation:** Your 138K is sufficient for domain-specific excellence. Focus on architectural improvements over more data.

---

## What Makes Agentic Code Search Different

Agents have stricter requirements than human developers:

| Requirement | Human Search | Agentic Search |
|-------------|--------------|----------------|
| **Precision** | Can scan results | Needs correct code on first result |
| **Latency** | 500ms acceptable | <50ms for interactive agents |
| **Context** | Single query | Multi-turn, remembers previous searches |
| **Scope** | "Find X" | "Find where X is defined, who calls it, and how to modify it" |
| **Failure mode** | Human reviews results | Agent may hallucinate if wrong code returned |

---

## The Plan: 5 Phases

### Phase 1: Benchmark Against SOTA (Week 1)

Before improving, establish baselines on standard benchmarks.

**Action:** Evaluate on CodeSearchNet test set (all 6 languages)

```bash
# Download CodeSearchNet test queries
wget https://s3.amazonaws.com/code-search-net/CodeSearchNet/test/python.jsonl
wget https://s3.amazonaws.com/code-search-net/CodeSearchNet/test/java.jsonl
# ... etc

# Evaluate current models
sg eval --model unixcoder --hybrid --spec eval/codesearchnet_python.json
sg eval --model jina-code --hybrid --spec eval/codesearchnet_python.json
sg eval --model-path checkpoints/xtr-rust-merged --hybrid --spec eval/codesearchnet_python.json
```

**Expected Results:**
| Model | Your Corpus (P@1) | CodeSearchNet (MRR) | Note |
|-------|-------------------|---------------------|------|
| UniXcoder (base) | 0.93 | ~0.45 | Public benchmark |
| XTR (fine-tuned 138K) | 0.93 | ~0.35 | Domain-specific |
| CodeBERT | - | 0.67 | Published SOTA |

This tells us: Are we overfitting to our corpus, or generalizing well?

---

### Phase 2: Scale Training Data (Weeks 2-3)

**Option A: Full CodeSearchNet (2M pairs)**

Most bang for buck. Standard training corpus.

```yaml
# config/train_codesearchnet_full.yaml
data:
  train: "data/codesearchnet_all.jsonl"  # 2M pairs, 6 languages

model:
  base: "google/xtr-base-en"
  output: "checkpoints/xtr-codesearchnet-v1"

training:
  method: "lora"
  lora_r: 32  # Higher rank for more capacity
  lora_alpha: 64
  batch_size: 64
  gradient_accumulation_steps: 4  # Effective batch = 256
  epochs: 3
  learning_rate: 1e-5
  warmup_steps: 5000
  hard_negatives: 31  # More hard negatives with larger batch
```

**Compute:** A100 (40GB) x 1, ~8-12 hours
**Expected:** +15-20% MRR on CodeSearchNet benchmark

**Option B: The Stack Subset (10M+ pairs)**

Larger scale, more languages.

```bash
# Download from HuggingFace (filtered, deduplicated)
python scripts/download_stack.py --languages rust,python,java,typescript --max-pairs 10000000

# Train with distributed data parallel
torchrun --nproc_per_node=4 scripts/train_xtr_distributed.py --config config/train_stack_10m.yaml
```

**Compute:** 4x A100 (80GB), ~24-48 hours
**Expected:** +25-30% MRR, better generalization

---

### Phase 3: Architectural Improvements (Weeks 3-4)

Current system does text-to-text matching. Agents need code structure understanding.

#### 3.1: Add Code Structure Signals

**Why:** "Find the function that handles user auth" should match `authenticate_user()` even if the word "auth" doesn't appear.

```rust
// Add to embedder pipeline
struct EnrichedEmbedding {
    text_embedding: Vec<f32>,      // Current XTR output
    structure_embedding: Vec<f32>, // NEW: AST-based features
    symbol_embedding: Vec<f32>,    // NEW: Function/class name embedding
}

// Structure features to extract:
struct CodeStructure {
    function_name: String,
    class_name: Option<String>,
    parameters: Vec<String>,
    return_type: Option<String>,
    imports: Vec<String>,
    calls: Vec<String>,  // Functions called by this code
}
```

**Implementation:**
1. Use tree-sitter for AST parsing (already fast, multi-language)
2. Embed structure features with separate encoder
3. Fuse with text embedding (learned weighted combination)

#### 3.2: Add Cross-File Context

**Why:** Agents need to understand "where is X defined" and "who calls X"

```rust
// Index relationships, not just code
struct CodeIndex {
    // Current: code chunks
    chunks: Vec<ChunkEmbedding>,

    // NEW: Definition index
    definitions: HashMap<Symbol, Location>,

    // NEW: Call graph
    call_graph: DirectedGraph<Symbol>,

    // NEW: Import graph
    import_graph: DirectedGraph<Module>,
}

// Query expansion for agents
fn agent_search(query: &str, context: &AgentContext) -> Results {
    // 1. Semantic search (current)
    let semantic = search_semantic(query);

    // 2. If query mentions a symbol, find its definition
    let definitions = find_definitions(extract_symbols(query));

    // 3. If agent is editing file X, boost results from related files
    let context_boost = boost_related_files(context.current_file);

    fuse_results(semantic, definitions, context_boost)
}
```

#### 3.3: LLM Reranking

**Why:** Small models miss nuance. LLM reranking improves P@1 significantly.

```python
# After semantic search returns top-20
def rerank_with_llm(query: str, candidates: list[CodeChunk]) -> list[CodeChunk]:
    prompt = f"""
    Query: {query}

    Rank these code snippets by relevance (most relevant first):

    {format_candidates(candidates)}

    Return ranking as JSON: ["id1", "id2", ...]
    """

    ranking = llm.complete(prompt, model="claude-3-haiku")
    return reorder(candidates, ranking)
```

**Compute:** ~100ms per query (acceptable for agents)
**Expected:** +10-15% P@1 on hard queries

---

### Phase 4: Training Improvements (Weeks 4-5)

#### 4.1: Hard Negative Mining

Current: Random in-batch negatives
Better: Mine hard negatives that confuse the model

```python
# Hard negative mining
def mine_hard_negatives(model, corpus, queries):
    hard_negatives = []

    for query, positive in training_pairs:
        # Embed query
        q_emb = model.embed_query(query)

        # Find top-k that are NOT the positive
        candidates = semantic_search(q_emb, corpus, k=100)
        hard_negs = [c for c in candidates if c != positive][:15]

        hard_negatives.append((query, positive, hard_negs))

    return hard_negatives
```

**Expected:** +5-10% on hard queries

#### 4.2: Curriculum Learning

Start easy, get harder.

```python
training_schedule = [
    {"epoch": 1, "hard_negatives": 3, "temperature": 0.1},   # Easy
    {"epoch": 2, "hard_negatives": 7, "temperature": 0.07},  # Medium
    {"epoch": 3, "hard_negatives": 15, "temperature": 0.05}, # Hard
]
```

#### 4.3: Multi-Task Learning

Train on multiple objectives simultaneously:

```python
losses = [
    contrastive_loss(query_emb, code_emb),           # Current
    mlm_loss(masked_code, model),                     # Masked language modeling
    structure_loss(predicted_ast, actual_ast),        # AST prediction
    type_loss(predicted_types, actual_types),         # Type inference
]
total_loss = sum(w * l for w, l in zip(weights, losses))
```

---

### Phase 5: Agentic-Specific Features (Weeks 5-6)

#### 5.1: Query Understanding

Agents ask different questions than humans.

```python
# Classify query intent
query_types = {
    "find_definition": "Where is X defined?",
    "find_usage": "Where is X used?",
    "find_implementation": "How is X implemented?",
    "find_similar": "Find code similar to X",
    "find_change": "What code needs to change for X?",
}

def route_query(query: str) -> SearchStrategy:
    intent = classify_intent(query)

    if intent == "find_definition":
        return DefinitionSearch()  # Use symbol index
    elif intent == "find_usage":
        return UsageSearch()  # Use call graph
    else:
        return SemanticSearch()  # Current approach
```

#### 5.2: Context-Aware Search

Use agent's current state to improve results.

```python
class AgentContext:
    current_file: Path
    recent_files: list[Path]
    recent_queries: list[str]
    task_description: str

def contextual_search(query: str, context: AgentContext) -> Results:
    # 1. Standard semantic search
    base_results = semantic_search(query)

    # 2. Boost files related to current work
    for result in base_results:
        if is_related(result.file, context.current_file):
            result.score *= 1.3  # 30% boost

    # 3. Penalize recently viewed (agent probably wants something new)
    for result in base_results:
        if result.file in context.recent_files:
            result.score *= 0.8  # 20% penalty

    return base_results.sort_by_score()
```

#### 5.3: Multi-Hop Retrieval

For complex queries that require multiple searches.

```python
def multi_hop_search(query: str, max_hops: int = 3) -> Results:
    results = []
    context = []

    for hop in range(max_hops):
        # Generate sub-query based on context
        sub_query = generate_sub_query(query, context)

        # Search
        hop_results = semantic_search(sub_query)
        results.extend(hop_results)

        # Update context
        context.append(hop_results[0])

        # Check if we've answered the query
        if is_sufficient(query, results):
            break

    return dedupe_and_rank(results)
```

---

## Recommended Execution Order

| Phase | Action | Compute | Time | Expected Gain |
|-------|--------|---------|------|---------------|
| **1** | Benchmark on CodeSearchNet | CPU | 1 day | Baseline |
| **2a** | Train on CodeSearchNet (2M) | A100 x1 | 8-12h | +15-20% MRR |
| **3.3** | Add LLM reranking | API | 2 days | +10-15% P@1 |
| **3.1** | Add code structure signals | CPU | 3 days | +5-10% on structure queries |
| **4.1** | Hard negative mining | A100 x1 | 12h | +5-10% on hard queries |
| **5.2** | Context-aware search | CPU | 2 days | +10% for agents |

**Total compute needed:**
- A100 (40GB): ~30 GPU-hours for training
- API costs: ~$10-50 for LLM reranking experiments

---

## Alternative: Use Existing SOTA Models

Instead of training, use best available:

| Model | MRR (CodeSearchNet) | Access |
|-------|---------------------|--------|
| **OpenAI Code Search** | Unknown | API |
| **GitHub Copilot Search** | Unknown | VSCode |
| **Voyage Code-2** | ~0.75 | API |
| **Cohere Embed-v3** | ~0.70 | API |

For agentic systems, **Voyage Code-2** is worth evaluating:
- Trained on code, optimized for search
- 16K context (vs 512 for XTR)
- $0.10 per 1M tokens

```bash
# Quick evaluation
python scripts/eval_voyage.py --spec eval/code_queries.json
```

---

## Success Metrics for Agentic Code Search

| Metric | Current | Target | Why It Matters |
|--------|---------|--------|----------------|
| **P@1** | 0.93 | 0.98 | Agent needs correct code first try |
| **MRR** | 0.97 | 0.99 | Reduce agent search iterations |
| **Latency (p50)** | ~100ms | <50ms | Fast interactive agents |
| **Latency (p99)** | ~500ms | <200ms | No long tails |
| **Multi-hop success** | N/A | 0.85 | Complex queries |
| **Context-aware P@1** | N/A | +0.10 | When agent context available |

---

## Data Quality Improvements (Quick Wins)

Before training on more data, improve existing 138K:

### 1. Filter Low-Quality Pairs

```python
def filter_quality(pairs):
    return [p for p in pairs if
        len(p["query"]) > 20 and          # Min query length
        len(p["positive"]) > 100 and      # Min code length
        len(p["positive"]) < 5000 and     # Max code length
        not is_trivial_docstring(p["query"]) and  # Not just "TODO"
        has_meaningful_content(p["positive"])     # Not just imports
    ]
```

### 2. Deduplicate Near-Duplicates

```python
def dedupe_by_embedding(pairs, threshold=0.95):
    embeddings = [embed(p["positive"]) for p in pairs]
    clusters = cluster_by_similarity(embeddings, threshold)
    return [pairs[c[0]] for c in clusters]  # Keep one per cluster
```

### 3. Balance Languages

Current: Rust (96K), Python (25K), others (<10K each)

```python
# Upsample minority languages
target_per_language = 20000
for lang in languages:
    pairs[lang] = resample(pairs[lang], target_per_language)
```

---

## Next Steps

1. **This week:** Run CodeSearchNet benchmark to establish baseline
2. **Decision point:** If baseline is <0.50 MRR, prioritize training on CodeSearchNet
3. **Decision point:** If baseline is >0.60 MRR, prioritize architectural improvements

Want me to:
1. Create the CodeSearchNet evaluation pipeline?
2. Set up the GPU training configuration?
3. Implement LLM reranking first (quick win)?
4. Something else?
