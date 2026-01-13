# SuperGrep Retrieval Architecture

## Design: Two-Stage Retrieval

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         QUERY                                           │
│                    "find authentication handler"                        │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  STAGE 1: FAST RETRIEVAL (XTR/ColBERT)                                  │
│  ─────────────────────────────────────                                  │
│  • Latency: <20ms                                                       │
│  • Output: Top 100 candidates                                           │
│  • Method: MaxSim late interaction                                      │
│  • Pre-computed: Document embeddings stored in index                    │
│  • At query time: Only encode query (single forward pass)               │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼ Top 100
┌─────────────────────────────────────────────────────────────────────────┐
│  STAGE 2: RERANKER (Optional, for maximum precision)                    │
│  ─────────────────────────────────────────────────────                  │
│  • Latency: +50-200ms                                                   │
│  • Output: Top 10 reranked                                              │
│  • Method: Cross-encoder or LLM scoring                                 │
│  • Sees: Full (query, document) pairs                                   │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼ Top 10
┌─────────────────────────────────────────────────────────────────────────┐
│                         RESULTS                                         │
│  1. src/auth/handler.rs:authenticate_user()                             │
│  2. src/auth/middleware.rs:AuthMiddleware                               │
│  3. ...                                                                 │
└─────────────────────────────────────────────────────────────────────────┘
```

## Why This Architecture?

### Stage 1: XTR/ColBERT (Late Interaction)

**Architecture:**
```
Query:  "find auth handler"
         ↓
     [Encoder]
         ↓
    [q1, q2, q3]  ← Per-token embeddings (128-dim each)
         │
         ├──MaxSim──→ doc1: [d1, d2, d3, d4, d5]  → score: 0.87
         ├──MaxSim──→ doc2: [d1, d2, d3]          → score: 0.72
         └──MaxSim──→ doc3: [d1, d2, d3, d4]      → score: 0.65
```

**MaxSim formula:**
```
score(q, d) = (1/|q|) * Σᵢ maxⱼ(qᵢ · dⱼ)
```

For each query token, find the most similar document token, then average.

**Why it's fast:**
- Document embeddings pre-computed at index time
- Query encoding: single forward pass (~5ms)
- MaxSim: matrix operations, vectorized (~10-15ms for 100K docs)

**Why it's good for Recall:**
- Per-token matching captures partial matches
- "authentication" matches "auth" (shared subword tokens)
- Better than single-vector (captures multiple aspects)

### Stage 2: Reranker (Cross-Encoder)

**Architecture:**
```
For each (query, candidate) pair:
    input = "[CLS] query [SEP] code [SEP]"
    score = CrossEncoder(input)  → single relevance score
```

**Why it's accurate:**
- Full attention between query and document
- Can understand complex relationships
- Much more expressive than bi-encoder

**Why it's slow:**
- Must run encoder for EACH (query, doc) pair
- 100 candidates = 100 forward passes

**Options:**
1. **Small cross-encoder** (MiniLM): ~5ms/pair → 500ms total
2. **Code-specific** (CodeBERT): ~10ms/pair → 1s total
3. **LLM reranking** (Haiku): ~50ms/batch → 100-200ms total

## Implementation

### Stage 1: Fast Retrieval (Current)

```rust
// crates/sg-core/src/search.rs

pub fn search_semantic(
    query: &str,
    index: &Index,
    top_k: usize,
) -> Vec<SearchResult> {
    // 1. Encode query (single forward pass)
    let query_emb = embedder.embed_query(query)?;  // ~5ms

    // 2. MaxSim against all documents
    let scores = index.maxsim(&query_emb)?;  // ~10-15ms

    // 3. Top-K selection
    top_k_by_score(scores, top_k)  // ~1ms
}
```

### Stage 2: Reranker (To Add)

```rust
// crates/sg-core/src/rerank.rs

pub trait Reranker {
    fn rerank(&self, query: &str, candidates: &[SearchResult], top_k: usize)
        -> Vec<SearchResult>;
}

pub struct CrossEncoderReranker {
    model: CrossEncoder,
}

pub struct LLMReranker {
    client: AnthropicClient,
    model: String,  // "claude-3-haiku-20240307"
}

impl Reranker for LLMReranker {
    fn rerank(&self, query: &str, candidates: &[SearchResult], top_k: usize)
        -> Vec<SearchResult>
    {
        let prompt = format!(
            "Rank these code snippets by relevance to: {query}\n\n{}",
            format_candidates(candidates)
        );

        let response = self.client.complete(&prompt)?;
        parse_ranking(response, candidates)
    }
}
```

### Combined Pipeline

```rust
pub fn search_with_rerank(
    query: &str,
    index: &Index,
    reranker: Option<&dyn Reranker>,
) -> Vec<SearchResult> {
    // Stage 1: Fast retrieval
    let candidates = search_semantic(query, index, 100);  // Top 100

    // Stage 2: Rerank (optional)
    match reranker {
        Some(r) => r.rerank(query, &candidates, 10),
        None => candidates.into_iter().take(10).collect(),
    }
}
```

## Speed Targets

| Operation | Target | Current | Notes |
|-----------|--------|---------|-------|
| Query encoding | <10ms | ~5ms | XTR forward pass |
| MaxSim (10K docs) | <20ms | ~15ms | Vectorized |
| MaxSim (100K docs) | <50ms | ~80ms | Needs HNSW |
| Rerank (100 docs) | <200ms | N/A | LLM batch |
| **Total (10K)** | **<30ms** | ~20ms | Without rerank |
| **Total (100K)** | **<250ms** | N/A | With rerank |

## Optimizations

### 1. PLAID (ColBERTv2)

Pre-compute centroids, only score against nearby clusters:
```
Index time: cluster doc tokens into C centroids
Query time: find top-c centroids per query token, only score those
```

Speedup: 10-100x for large corpora

### 2. Product Quantization

Compress 128-dim vectors to 32 bytes:
```rust
struct QuantizedEmbedding {
    codes: [u8; 32],  // 32 subquantizers, 256 codes each
}
```

Memory: 4x reduction
Speed: Slightly slower (decode step)

### 3. HNSW Pre-filtering

For very large corpora, use HNSW to pre-filter:
```
1. HNSW → Top 1000 approximate candidates
2. MaxSim → Top 100 exact scores
3. Rerank → Top 10
```

## Data Flow

```
┌──────────────┐
│  Raw Code    │
└──────┬───────┘
       │ Index time (background)
       ▼
┌──────────────┐
│  Chunker     │  Split into semantic chunks (functions, classes)
└──────┬───────┘
       │
       ▼
┌──────────────┐
│  Embedder    │  XTR: text → [N, 128] token embeddings
└──────┬───────┘
       │
       ▼
┌──────────────┐
│   SQLite     │  Store: chunk_id, embeddings, metadata
└──────┬───────┘
       │
       │ Query time
       ▼
┌──────────────┐
│  MaxSim      │  Score query against all chunks
└──────┬───────┘
       │
       ▼
┌──────────────┐
│  Reranker    │  (Optional) LLM or cross-encoder
└──────┬───────┘
       │
       ▼
┌──────────────┐
│  Results     │  Top-K with scores and context
└──────────────┘
```

## Configuration

```bash
# Fast mode (Stage 1 only)
sg search "auth handler" --top 10

# High precision mode (Stage 1 + Stage 2)
sg search "auth handler" --top 10 --rerank

# Reranker options
sg search "auth handler" --rerank --reranker llm      # Use Claude Haiku
sg search "auth handler" --rerank --reranker cross    # Use cross-encoder

# Speed vs accuracy tradeoff
sg search "auth handler" --candidates 50 --rerank     # Faster, less recall
sg search "auth handler" --candidates 200 --rerank    # Slower, more recall
```

## Why XTR/ColBERT is Best for Code Search

1. **Token-level matching**: "getUserAuth" matches "get_user_authentication"
2. **Partial matches**: Query mentions subset of functionality
3. **Multi-aspect**: Code has multiple concepts (function name, params, logic)
4. **Pre-computable**: Documents indexed once, queries are fast

Single-vector models (BERT, sentence-transformers) compress everything into one vector, losing fine-grained information needed for code.

## References

- [ColBERT: Efficient and Effective Passage Search](https://arxiv.org/abs/2004.12832)
- [ColBERTv2: Effective and Efficient Retrieval via Lightweight Late Interaction](https://arxiv.org/abs/2112.01488)
- [XTR: Rethinking the Role of Token Retrieval in Multi-Vector Retrieval](https://arxiv.org/abs/2304.01982)
- [PLAID: An Efficient Engine for Late Interaction Retrieval](https://arxiv.org/abs/2205.09707)
