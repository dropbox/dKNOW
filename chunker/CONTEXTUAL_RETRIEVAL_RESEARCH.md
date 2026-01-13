# Contextual Retrieval with Local LLMs - Research Report

## Executive Summary

This report evaluates options for implementing Anthropic-style contextual retrieval using local LLMs to generate semantic context for document chunks. The goal is to enhance RAG performance by 35-67% (per Anthropic's research) while maintaining efficiency through local inference.

**Recommendation:** Use **Llama 3.2 3B Instruct** with **llama.cpp** for optimal balance of quality, speed, and resource efficiency.

---

## Background: Anthropic's Contextual Retrieval

### What It Is
Anthropic's contextual retrieval prepends LLM-generated explanatory context to each chunk before embedding:

**Without context:**
```
Chunk: "The Q2 revenue increased by 15% year-over-year."
```

**With context:**
```
Context: "This chunk is from ACME Corp's Q2 2023 SEC filing discussing financial performance."
Chunk: "The Q2 revenue increased by 15% year-over-year."
```

### Performance Impact
- **Contextual Embeddings alone:** 35% reduction in retrieval failure
- **Contextual Embeddings + BM25:** 49% reduction in retrieval failure
- **With reranking:** 67% reduction in retrieval failure

### Implementation Requirements
1. **Context generation prompt** that takes:
   - Full document or section
   - Individual chunk
   - Returns 1-2 sentence context explaining the chunk
2. **Fast inference** for processing many chunks
3. **Consistent quality** across diverse content types

---

## Local LLM Options Analysis

### Evaluation Criteria
1. **Quality:** Context generation accuracy and coherence
2. **Speed:** Tokens/second throughput
3. **Memory:** RAM/VRAM requirements
4. **Integration:** Ease of use with Rust
5. **Multilingual:** Support for EN, JA, ZH, KO

---

## Option 1: Llama 3.2 (1B/3B) - **RECOMMENDED**

### Models
- **Llama 3.2 1B Instruct:** Ultra-fast, lightweight
- **Llama 3.2 3B Instruct:** Best quality/speed tradeoff

### Specifications (3B model)
- **Size:** ~2 GB (Q4_K_M quantized)
- **Speed:** 50-100 tokens/sec (M1/M2 Mac, CPU)
- **Memory:** 4-6 GB RAM
- **Context:** 128K tokens
- **Multilingual:** Strong EN, good JA/ZH/KO

### Integration: llama.cpp (Rust binding)
```toml
[dependencies]
llama-cpp-2 = "0.1"  # Rust bindings for llama.cpp
```

```rust
use llama_cpp_2::{context::LlamaContext, model::LlamaModel};

pub struct ContextGenerator {
    model: LlamaModel,
    context: LlamaContext,
}

impl ContextGenerator {
    pub fn new(model_path: &str) -> Result<Self> {
        let model = LlamaModel::load_from_file(model_path, params)?;
        let context = model.new_context(&builder)?;
        Ok(Self { model, context })
    }

    pub fn generate_context(&self, document: &str, chunk: &str) -> String {
        let prompt = format!(
            "<|begin_of_text|><|start_header_id|>system<|end_header_id|>\n\
            You are a precise context generator. Given a document and a chunk, \
            provide 1-2 sentences explaining what the chunk is about.\
            <|eot_id|><|start_header_id|>user<|end_header_id|>\n\
            Document: {}\n\nChunk: {}\n\n\
            Provide a brief context for this chunk.\
            <|eot_id|><|start_header_id|>assistant<|end_header_id|>\n",
            document, chunk
        );

        self.context.complete(&prompt, max_tokens)
    }
}
```

### Pros
✅ Excellent quality for size
✅ Fast inference on CPU (50-100 tok/s)
✅ Low memory footprint
✅ Native llama.cpp support (mature, fast)
✅ Strong multilingual capabilities
✅ Free and open source (Llama 3.2 license)

### Cons
❌ Requires separate model download (~2 GB)
❌ CPU inference slower than GPU

### Performance Estimate
For 10K word document → ~15 chunks → ~15 context generations:
- **Time:** ~5-10 seconds (CPU), ~2-3 seconds (GPU)
- **Memory:** 4-6 GB RAM

---

## Option 2: Gemma 2 (2B/9B)

### Models
- **Gemma 2 2B Instruct:** Compact, efficient
- **Gemma 2 9B Instruct:** Higher quality, slower

### Specifications (2B model)
- **Size:** ~1.5 GB (Q4_K_M quantized)
- **Speed:** 40-80 tokens/sec (M1/M2 Mac, CPU)
- **Memory:** 3-5 GB RAM
- **Context:** 8K tokens
- **Multilingual:** Good EN, moderate JA/ZH/KO

### Integration
Same as Llama (llama.cpp), just different model format.

### Pros
✅ Smaller than Llama 3.2
✅ Good quality for size
✅ Google-backed, well-maintained

### Cons
❌ 8K context limit (vs 128K for Llama 3.2)
❌ Weaker multilingual support than Llama
❌ Slightly lower quality than Llama 3.2 3B

---

## Option 3: Phi-3 (Mini/Small/Medium)

### Models
- **Phi-3-mini (3.8B):** Fast, good quality
- **Phi-3-small (7B):** Better quality
- **Phi-3-medium (14B):** Best quality, slower

### Specifications (mini 3.8B)
- **Size:** ~2.3 GB (Q4_K_M quantized)
- **Speed:** 40-70 tokens/sec (M1/M2 Mac, CPU)
- **Memory:** 4-6 GB RAM
- **Context:** 128K tokens
- **Multilingual:** Good EN, moderate JA/ZH/KO

### Pros
✅ Strong reasoning capabilities
✅ 128K context window
✅ Microsoft-backed

### Cons
❌ Weaker multilingual than Llama
❌ Slightly slower than Llama 3.2

---

## Option 4: Mistral/Mixtral (7B/8x7B)

### Models
- **Mistral 7B Instruct:** Solid all-rounder
- **Mixtral 8x7B:** MoE, higher quality

### Specifications (Mistral 7B)
- **Size:** ~4.1 GB (Q4_K_M quantized)
- **Speed:** 30-60 tokens/sec (M1/M2 Mac, CPU)
- **Memory:** 8-10 GB RAM
- **Context:** 32K tokens
- **Multilingual:** Good EN, good JA/ZH

### Pros
✅ Excellent instruction following
✅ Strong code and technical content
✅ Good multilingual support

### Cons
❌ Larger memory footprint
❌ Slower than smaller models
❌ Overkill for simple context generation

---

## Option 5: Qwen2.5 (0.5B-72B)

### Models
- **Qwen2.5 0.5B/1.5B/3B:** Ultra-efficient
- **Qwen2.5 7B/14B:** Higher quality

### Specifications (3B model)
- **Size:** ~1.9 GB (Q4_K_M quantized)
- **Speed:** 50-100 tokens/sec (M1/M2 Mac, CPU)
- **Memory:** 4-6 GB RAM
- **Context:** 32K tokens
- **Multilingual:** **Excellent** for EN, JA, ZH, KO

### Pros
✅ **Best multilingual support** (Chinese-focused)
✅ Very fast inference
✅ Multiple size options
✅ Strong technical/code capabilities

### Cons
❌ Less well-known in Western markets
❌ Training data biased toward Chinese content

---

## Rust Integration Options

### 1. llama.cpp bindings (RECOMMENDED)
**Crate:** `llama-cpp-2` or `llama-cpp-rs`

```toml
[dependencies]
llama-cpp-2 = "0.1"
```

**Pros:**
- Mature, fast C++ backend
- Excellent CPU optimization (SIMD, AVX2, etc.)
- Supports all GGUF quantized models
- Active development

**Cons:**
- Requires llama.cpp library installed
- Build complexity

---

### 2. candle (Rust-native)
**Crate:** `candle-core`, `candle-transformers`

```toml
[dependencies]
candle-core = "0.4"
candle-transformers = "0.4"
```

**Pros:**
- Pure Rust implementation
- No C/C++ dependencies
- Nice API ergonomics

**Cons:**
- Slower than llama.cpp
- Less mature ecosystem
- Fewer model optimizations

---

### 3. llm (High-level Rust abstraction)
**Crate:** `llm`

```toml
[dependencies]
llm = "0.1"
```

**Pros:**
- Simple, unified API
- Multiple backend support
- Easy model loading

**Cons:**
- Less control over inference
- Slower than direct llama.cpp

---

### 4. HTTP API (e.g., llama.cpp server)
**Approach:** Run llama.cpp server, use HTTP client

```bash
# Terminal 1: Start llama.cpp server
./llama-server -m model.gguf --port 8080

# Terminal 2: Use in Rust
```

```toml
[dependencies]
reqwest = { version = "0.11", features = ["json"] }
```

**Pros:**
- Decoupled from Rust build
- Easy to swap models
- Can run on separate machine/GPU

**Cons:**
- Network overhead
- Extra process management
- Latency for each request

---

## Implementation Architecture

### Proposed Design

```rust
// src/context_generator.rs

use crate::metadata::Chunk;

pub struct ContextGenerator {
    model: Box<dyn LLMBackend>,
    prompt_template: String,
}

pub trait LLMBackend {
    fn generate(&self, prompt: &str, max_tokens: usize) -> Result<String>;
}

// Backend implementations
pub struct LlamaCppBackend { /* ... */ }
pub struct CandleBackend { /* ... */ }
pub struct HttpBackend { /* ... */ }

impl ContextGenerator {
    pub fn generate_chunk_context(
        &self,
        document_summary: &str,
        chunk: &Chunk,
    ) -> Result<String> {
        let prompt = format!(
            "Document: {}\n\nChunk:\n{}\n\nBrief context:",
            document_summary,
            &chunk.content[..chunk.content.len().min(500)]
        );

        self.model.generate(&prompt, 100)
    }

    pub fn enrich_chunks(
        &self,
        document_summary: &str,
        chunks: Vec<Chunk>,
    ) -> Result<Vec<EnrichedChunk>> {
        chunks.into_iter()
            .map(|chunk| {
                let context = self.generate_chunk_context(document_summary, &chunk)?;
                Ok(EnrichedChunk { chunk, context })
            })
            .collect()
    }
}

pub struct EnrichedChunk {
    pub chunk: Chunk,
    pub context: String,
}

impl EnrichedChunk {
    /// Get content with prepended context for embedding
    pub fn content_with_context(&self) -> String {
        format!("{}\n\n{}", self.context, self.chunk.content)
    }
}
```

### Usage

```rust
use markdown_chunker::{Chunker, ContextGenerator};

// 1. Chunk the document
let chunker = Chunker::default();
let chunks = chunker.chunk(&markdown_text);

// 2. Generate document summary (optional, for context)
let doc_summary = "Technical documentation about API authentication";

// 3. Enrich chunks with LLM-generated context
let generator = ContextGenerator::new("llama-3.2-3b-instruct-q4.gguf")?;
let enriched = generator.enrich_chunks(doc_summary, chunks)?;

// 4. Embed with context
for enriched_chunk in enriched {
    let content_to_embed = enriched_chunk.content_with_context();
    let embedding = embed_model.embed(&content_to_embed)?;
    // Store in vector DB...
}
```

---

## Recommended Implementation Plan

### Phase 4A: Basic Context Generation (MVP)
**Goal:** Get contextual retrieval working with local LLM

1. **Add llama.cpp integration**
   - Dependency: `llama-cpp-2 = "0.1"`
   - Download Llama 3.2 3B Instruct (Q4_K_M)
   - Basic inference wrapper

2. **Implement ContextGenerator trait**
   - Simple prompt template
   - Batch processing for multiple chunks
   - Error handling for LLM failures

3. **Add tests**
   - Mock LLM backend for testing
   - Integration test with real model (optional)

4. **Benchmarks**
   - Measure context generation overhead
   - Compare retrieval quality with/without context

**Time estimate:** 4-6 hours
**Performance target:** <1s per chunk on CPU

---

### Phase 4B: Optimization & Production Polish
**Goal:** Make it production-ready

1. **Caching**
   - Cache context for identical chunks
   - Persistent cache (SQLite or sled)

2. **Parallel generation**
   - Batch inference (process multiple chunks together)
   - Thread pool for concurrent generation

3. **Prompt engineering**
   - Test different prompt templates
   - Domain-specific prompts (code, docs, etc.)

4. **Quality evaluation**
   - Compare retrieval metrics
   - A/B test against structural context

**Time estimate:** 6-8 hours
**Performance target:** <100ms per chunk (amortized)

---

### Phase 4C: Advanced Features
**Goal:** State-of-the-art RAG

1. **Hybrid context**
   - Combine structural (headers) + semantic (LLM) context
   - Configurable weighting

2. **Multi-strategy**
   - Use LLM context for paragraphs
   - Use structural context for code/tables

3. **BM25 integration**
   - Add contextual BM25 scorer
   - Hybrid retrieval (vector + BM25)

4. **Reranking**
   - Integrate reranking model (e.g., BGE reranker)
   - Full Anthropic-style pipeline

**Time estimate:** 8-12 hours
**Expected improvement:** 49-67% reduction in retrieval failures

---

## Resource Requirements

### Development Machine (M1/M2/M3 Mac)
- **RAM:** 8 GB minimum (16 GB recommended)
- **Disk:** 5 GB for models
- **CPU:** Apple Silicon (Metal acceleration)

### Production Deployment
- **Option A (CPU-only):** 4 cores, 8 GB RAM
- **Option B (GPU):** 1x GPU with 4-8 GB VRAM
- **Disk:** 10-20 GB (multiple models)

### Latency Estimates
| Document Size | Chunks | Context Gen (CPU) | Context Gen (GPU) |
|--------------|--------|-------------------|-------------------|
| 1K words     | 2-3    | 0.5-1s           | 0.2-0.3s         |
| 10K words    | 15-20  | 5-10s            | 2-3s             |
| 100K words   | 150    | 50-100s          | 20-30s           |

**Note:** With caching and parallelization, subsequent runs can be 10-50x faster.

---

## Alternative Approaches

### 1. Structural Context Only (Current)
**Pros:** Fast, deterministic, no dependencies
**Cons:** 0% improvement over baseline

### 2. Cloud LLM (Claude/GPT-4)
**Pros:** Best quality, no local resources
**Cons:** Latency, cost ($0.01-0.05 per document), API dependency

### 3. Embedding-based Context (No LLM)
**Pros:** Fast, no LLM needed
**Cons:** Lower quality than semantic context

**Approach:** Use sentence embeddings to find most similar sentences from full document, prepend to chunk.

```rust
// Pseudo-code
let doc_embedding = embed_model.embed(document);
for chunk in chunks {
    let chunk_embedding = embed_model.embed(chunk);
    let similar_sentences = find_top_k_similar(chunk_embedding, doc_sentences, k=3);
    let context = similar_sentences.join(" ");
    enriched_chunk = format!("{}\n\n{}", context, chunk);
}
```

**Performance:** 15-25% reduction in retrieval failures (estimated)

---

## Conclusion

### Recommended Solution: Llama 3.2 3B + llama.cpp

**Why:**
1. **Best quality/speed tradeoff** for context generation
2. **Low resource requirements** (4-6 GB RAM)
3. **Strong multilingual support** (EN, JA, ZH, KO)
4. **Mature Rust ecosystem** (llama-cpp-2)
5. **Free and open source**

### Implementation Priority
1. **Phase 4A (MVP):** Basic context generation with Llama 3.2 3B
2. **Measure impact:** Compare retrieval quality vs. structural-only
3. **If <20% improvement:** Consider alternative approaches
4. **If >20% improvement:** Proceed to Phase 4B optimization

### Expected Outcomes
- **Retrieval quality:** 25-35% improvement (conservative estimate)
- **Latency:** +5-10s per document (10K words, CPU)
- **Memory:** +4-6 GB RAM
- **Cost:** $0 (free, local inference)

---

## Appendix A: Model Download Links

### Llama 3.2 3B Instruct (Recommended)
```bash
# Using Hugging Face CLI
huggingface-cli download \
  bartowski/Llama-3.2-3B-Instruct-GGUF \
  Llama-3.2-3B-Instruct-Q4_K_M.gguf \
  --local-dir ./models
```

**URL:** https://huggingface.co/bartowski/Llama-3.2-3B-Instruct-GGUF

### Llama 3.2 1B Instruct (Lightweight)
```bash
huggingface-cli download \
  bartowski/Llama-3.2-1B-Instruct-GGUF \
  Llama-3.2-1B-Instruct-Q4_K_M.gguf \
  --local-dir ./models
```

**URL:** https://huggingface.co/bartowski/Llama-3.2-1B-Instruct-GGUF

### Qwen2.5 3B Instruct (Best Multilingual)
```bash
huggingface-cli download \
  Qwen/Qwen2.5-3B-Instruct-GGUF \
  qwen2.5-3b-instruct-q4_k_m.gguf \
  --local-dir ./models
```

**URL:** https://huggingface.co/Qwen/Qwen2.5-3B-Instruct-GGUF

---

## Appendix B: Prompt Templates

### Template 1: Concise Context
```
<|begin_of_text|><|start_header_id|>system<|end_header_id|>
You are a precise context generator. Provide a brief 1-2 sentence explanation of what the chunk discusses.
<|eot_id|><|start_header_id|>user<|end_header_id|>
Document: {document_summary}

Chunk:
{chunk_content}

Provide concise context for this chunk.
<|eot_id|><|start_header_id|>assistant<|end_header_id|>
```

### Template 2: Detailed Context (Anthropic-style)
```
<|begin_of_text|><|start_header_id|>system<|end_header_id|>
You explain what document chunks are about in 1-2 clear sentences.
<|eot_id|><|start_header_id|>user<|end_header_id|>
Here is the document:
<document>
{full_document_or_summary}
</document>

Here is the chunk we want to describe:
<chunk>
{chunk_content}
</chunk>

Please give a short succinct context to situate this chunk within the overall document for the purposes of improving search retrieval of the chunk. Answer only with the succinct context and nothing else.
<|eot_id|><|start_header_id|>assistant<|end_header_id|>
```

### Template 3: Multilingual Context
```
<|begin_of_text|><|start_header_id|>system<|end_header_id|>
Provide context for the chunk in the same language as the chunk. Be concise (1-2 sentences).
<|eot_id|><|start_header_id|>user<|end_header_id|>
Document: {document_summary}

Chunk:
{chunk_content}

Context (in same language):
<|eot_id|><|start_header_id|>assistant<|end_header_id|>
```

---

## Appendix C: Benchmark Script

```rust
// benches/context_generation.rs

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use markdown_chunker::{Chunker, ContextGenerator};

fn benchmark_context_generation(c: &mut Criterion) {
    let markdown = include_str!("../tests/fixtures/complex_structure.md");
    let chunker = Chunker::default();
    let chunks = chunker.chunk(markdown);

    let generator = ContextGenerator::new("models/llama-3.2-3b-instruct-q4.gguf")
        .expect("Failed to load model");

    c.bench_function("context_generation_per_chunk", |b| {
        b.iter(|| {
            for chunk in &chunks {
                let context = generator.generate_chunk_context(
                    black_box("Technical documentation"),
                    black_box(chunk),
                );
                black_box(context);
            }
        });
    });
}

criterion_group!(benches, benchmark_context_generation);
criterion_main!(benches);
```

---

## References

1. **Anthropic Contextual Retrieval**
   https://www.anthropic.com/news/contextual-retrieval

2. **Llama 3.2 Release**
   https://ai.meta.com/blog/llama-3-2-connect-2024-vision-edge-mobile-devices/

3. **llama.cpp**
   https://github.com/ggerganov/llama.cpp

4. **Qwen2.5 Release**
   https://qwenlm.github.io/blog/qwen2.5/

5. **RAG Best Practices**
   https://www.pinecone.io/learn/retrieval-augmented-generation/

---

**Document Version:** 1.0
**Last Updated:** 2025-10-06
**Author:** AI Agent Session 4
