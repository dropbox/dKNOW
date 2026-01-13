# SuperGrep (sg) Development Roadmap

**Last Updated:** 2025-12-31
**Current Version:** v0.2.0

---

## Overview

Build a semantic code search CLI that uses XTR-WARP multi-vector retrieval. The system runs as a background daemon, automatically indexes projects, and provides instant search via the `sg` command.

**Reference:** rust-warp (`~/rust-warp/`) is documented inspiration, not a dependency. Core algorithms were studied and re-implemented in sg.

---

## Current State (v0.2.0)

### Completed Features

| Feature | Status | Iteration |
|---------|--------|-----------|
| Core search functionality | DONE | Phase 1-5 |
| Daemon architecture | DONE | Phase 2 |
| File watching & incremental | DONE | Phase 3 |
| Smart directory detection | DONE | Phase 4 |
| Chunk-level embedding | DONE | #361 |
| Product Quantization (32x compression) | DONE | #384 |
| HNSW for O(log n) cluster lookup | DONE | #385 |
| ONNX Runtime backend | DONE | #369-373 |
| CoreML backend (macOS) | DONE | #375 |
| CUDA backend | DONE | #376 |
| OpenVINO backend | DONE | #379 |
| TensorRT backend | DONE | #380 |
| Storage limits (max_total_mb) | DONE | #381 |
| Search result explanations | DONE | #382 |
| Progress spinners | DONE | #383 |
| WAL mode for SQLite | DONE | (default) |
| PDF indexing (document-processing) | DONE | #425 |
| DOCX/XLSX/ODS indexing | DONE | #426 |
| PPTX indexing | DONE | #427 |
| EPUB indexing | DONE | #431 |
| PDF metadata extraction | DONE | #432 |
| Markdown frontmatter extraction | DONE | #433 |
| Magic byte file type detection | DONE | #434 |
| Encoding detection (UTF-16, Latin-1) | DONE | #435 |
| Adaptive cluster count | DONE | #436 |
| Cluster rebalancing | DONE | #437 |
| Index health monitoring | DONE | #438 |
| Code preprocessing (identifier splitting) | DONE | #447 |
| OCR fallback for scanned PDFs | DONE | #448 |
| PDF table extraction | DONE | #449 |
| Query preprocessing for code search | DONE | #450 |
| Code block language extraction | DONE | #451-#452 |

### Evaluation Results

| Corpus | P@1 | MRR | Target | Status |
|--------|-----|-----|--------|--------|
| Gutenberg (10 queries) | 1.00 | 1.00 | 0.50 | PASS |
| Code (15 queries)* | 0.93 | 0.97 | 0.50 | PASS |
| Multilingual/Japanese (6 queries)** | 1.00 | 1.00 | 0.50 | PASS |
| PDF (8 queries) | 1.00 | 1.00 | 0.50 | PASS |

*Code results with `--model unixcoder --hybrid` and filename relevance boost (recommended for code search)
**Japanese with XTR hybrid; Jina-ColBERT achieves 1.00 with semantic-only

---

## Historical Phases (v0.1.0 - Complete)

<details>
<summary>Phase 1: Core (Complete)</summary>

### Tasks
- [x] Fork/vendor rust-warp code into `crates/sg-core/`
- [x] Create CLI crate `crates/sg/`
- [x] Basic integration test

</details>

<details>
<summary>Phase 2: Daemon Architecture (Complete)</summary>

### Tasks
- [x] Create daemon crate `crates/sg-daemon/`
- [x] JSON-RPC protocol
- [x] CLI routes through daemon

</details>

<details>
<summary>Phase 3: File Watching & Incremental (Complete)</summary>

### Tasks
- [x] File watcher with debouncing
- [x] Incremental updates (hash-based)
- [x] Progressive clustering (LSH -> online k-means)
- [x] Resource throttling

</details>

<details>
<summary>Phase 4: Smart Directory Detection (Complete)</summary>

### Tasks
- [x] Project root detection
- [x] Auto-discovery
- [x] Auto-watch on cd
- [x] Storage lifecycle (LRU eviction)

</details>

<details>
<summary>Phase 5: Polish & Ship (Complete)</summary>

### Tasks
- [x] Output formatting (colored, context snippets, --json)
- [x] Status command
- [x] Hybrid search (semantic + regex)
- [x] Performance benchmarking (`sg benchmark`)
- [x] Documentation

</details>

---

## Phase 6: Production Chunker Integration

**Priority:** HIGH
**Status:** COMPLETE (iteration #393)

### Goal

Replace simple word-count chunker with production-grade hierarchy-aware chunker.

### Source

`git@github.com:dropbox/chunker.git` - markdown_chunker crate

### Tasks

| # | Task | Description | Effort |
|---|------|-------------|--------|
| 6.1 | Add markdown_chunker dependency | Path or workspace member | 1 commit |
| 6.2 | Update chunker.rs | Use hierarchy-aware chunking | 1 commit |
| 6.3 | Add header_context field | Track markdown headers per chunk | 1 commit |
| 6.4 | Update storage schema | Add header_context column | 1 commit |
| 6.5 | Update search display | Show header context in results | 1 commit |
| 6.6 | Verify evaluation metrics | P@1 >= 0.80 maintained | 1 commit |

### Benefits

- Never splits code blocks or tables
- Preserves markdown hierarchy context
- Multilingual support (CJK)
- Semantic overlap at sentence boundaries

### Success Criteria

- Code blocks never split mid-content
- Header context displayed: `# Module > ## Function`
- Evaluation metrics maintained (P@1 >= 0.80)

---

## Phase 7: Multi-Embedding Evaluation

**Priority:** HIGH
**Status:** COMPLETE (iteration #401)

### Goal

Prove that specialized embeddings outperform general embeddings on their target domain.

### Available Corpora (LOCAL)

| Corpus | Type | Location | Count |
|--------|------|----------|-------|
| Gutenberg | English prose | `~/eval/gutenberg/` | 10 books |
| EDINET | Japanese financial | `~/video_audio_extracts/test_pdf_corpus/all/edinet*.pdf` | 10 PDFs |
| sg codebase | Rust code | `~/sg/crates/` | ~30 files |
| Research papers | English technical | `~/video_audio_extracts/test_pdf_corpus/all/` | bert.pdf, etc. |

### Embedding Models to Compare

| Model | Type | Dimension | Best For |
|-------|------|-----------|----------|
| XTR (current) | General | 128 (multi-vector) | English text |
| microsoft/unixcoder-base | Code | 768 | Programming languages |
| jinaai/jina-embeddings-v2-base-code | Code | 768 | Code retrieval |
| Multilingual model | CJK | 768+ | Japanese/Chinese |

### Tasks

| # | Task | Description | Effort |
|---|------|-------------|--------|
| 7.1 | Create multi-corpus eval harness | Test across code/prose/CJK | 2 commits |
| 7.2 | Implement multi-model backend | EmbedderType enum, routing | 3 commits |
| 7.3 | Content-type detection | Auto-route to appropriate embedder | 1 commit |
| 7.4 | Run comparative evaluation | Generate results report | 1 commit |

### Success Criteria

- Code embeddings beat general on code (>=20% improvement)
- General embeddings beat code embeddings on prose
- Results documented in `eval/MULTI_EMBEDDING_RESULTS.md`

---

## Phase 8: Test Suite & Benchmarks

**Priority:** HIGH
**Status:** COMPLETE (criterion benchmarks #396-#397, #493; regression tests #497; memory profiling #498)

### Goal

Build infrastructure to prove optimization impact.

### Tasks

| # | Task | Description | Effort | Status |
|---|------|-------------|--------|--------|
| 8.1 | Add criterion benchmarks | Micro-benchmarks for hot paths | 2 commits | DONE #493 |
| 8.2 | Extend sg benchmark command | p50/p95/p99 latencies, memory | 2 commits | DONE |
| 8.3 | Add regression tests | Ensure optimizations don't break | 1 commit | DONE #497 |
| 8.4 | Memory profiling | Track allocations in hot paths | 1 commit | DONE #498 |

### Benchmark Targets

```
crates/sg-core/benches/
├── embedding_bench.rs      # embed_document throughput
├── maxsim_bench.rs         # MaxSim scoring
├── kmeans_bench.rs         # K-means clustering
├── quantizer_bench.rs      # PQ encode/decode
└── search_bench.rs         # End-to-end search latency
```

### Metrics to Report

| Metric | Current | Target | Notes |
|--------|---------|--------|-------|
| Model load time | ~3s | <1s | Lazy loading |
| Embed throughput | ~10 docs/s | ~50 docs/s | Batching |
| Search latency (p50) | ~100ms | <50ms | GPU acceleration |
| Search latency (p99) | ~500ms | <200ms | Parallel scoring |
| Memory per 1K docs | ~50MB | ~20MB | PQ compression |

---

## Phase 9: Performance Optimizations

**Priority:** MEDIUM-HIGH
**Status:** Planned

These optimizations are derived from rust-warp analysis but adapted to sg's architecture.

### 9A: Quick Wins (Low Effort, Immediate Impact)

**Status:** N/A - These optimizations were from rust-warp reference code and don't exist in sg's codebase.

| # | Optimization | Location | Impact | Effort | Status |
|---|--------------|----------|--------|--------|--------|
| 9A.1 | Static inv_compand table | quantizer.rs | Low | 1 commit | N/A (no companding in sg) |
| 9A.2 | FTS5 optimize vs rebuild | storage.rs | Medium | 1 commit | N/A (no FTS5 in sg) |
| 9A.3 | Eliminate temp tables in search | search.rs | Medium | 1 commit | N/A (no temp tables in sg) |

**9A.1 Static Inverse Companding Table**

```rust
// Current: table computed each call
fn from_companded_q4_bytes(...) {
    let table: [f32; 16] = compute_table();
    ...
}

// Optimization: static lookup table
use once_cell::sync::Lazy;
static INV_COMPAND_TABLE: Lazy<[f32; 16]> = Lazy::new(|| compute_table());
```

**9A.2 FTS5 Optimize vs Rebuild**

```rust
// Current: rebuilds entire index
"INSERT INTO document_fts(document_fts) VALUES('rebuild')"

// Optimization: incremental optimize
"INSERT INTO document_fts(document_fts) VALUES('optimize')"
```

**9A.3 Eliminate Temp Tables**

```rust
// Current: creates temp table per search
db.execute("CREATE TEMPORARY TABLE temp(id INTEGER)")?;
for id in cluster_ids { insert_temp.execute((id,))?; }
db.execute("DROP TABLE temp")?;

// Optimization: parameterized IN clause
let placeholders = (0..cluster_ids.len()).map(|i| format!("?{}", i+1)).join(",");
let sql = format!("SELECT ... WHERE id IN ({})", placeholders);
```

### 9B: Medium Impact (Batching & Parallelization)

| # | Optimization | Location | Impact | Effort |
|---|--------------|----------|--------|--------|
| 9B.1 | Batch embedding generation | embedder.rs | High | 3 commits |
| 9B.2 | Parallel document scoring | index.rs | High | DONE #423 |
| 9B.3 | Vectorize stretch_rows | quantizer.rs | Medium | 1 commit |
| 9B.4 | Consolidate device transfers | embedder.rs | Medium | 2 commits |

**9B.1 Batch Embedding Generation**

```rust
// Current: single document at a time
for doc in documents {
    let emb = embedder.embed_document(&doc.content)?;
}

// Optimization: batch multiple documents (8-16 at a time)
const BATCH_SIZE: usize = 8;
for batch in documents.chunks(BATCH_SIZE) {
    let texts: Vec<_> = batch.iter().map(|d| &d.content).collect();
    let embeddings = embedder.embed_batch(&texts)?;
}
```

**9B.2 Parallel Document Scoring**

```rust
// Current: sequential scoring
for chunk_id in chunk_ids {
    let score = maxsim(&query_emb, &chunk_emb)?;
    scored.push((chunk_id, score));
}

// Optimization: parallel with rayon
use rayon::prelude::*;
let scored: Vec<_> = chunk_ids.par_iter()
    .filter_map(|&id| {
        let emb = db.get_chunk_embeddings(id).ok()??;
        let score = maxsim(&query_emb, &emb).ok()?;
        Some((id, score))
    })
    .collect();
```

**9B.3 Vectorize stretch_rows**

```rust
// Current: scalar per-element loop
for i in 0..m {
    let row = self.get(i)?;
    let v = row.to_vec1::<f32>()?;
    let mut max = f32::MIN;
    for x in &v { max = max.max(x.abs()); }
}

// Optimization: tensor operations
let abs_rows = self.abs()?;
let max_vals = abs_rows.max_keepdim(1)?;
let scaled = self.broadcast_div(&(max_vals + 1e-6)?)?;
```

**9B.4 Consolidate Device Transfers**

```rust
// Current: multiple CPU<->GPU transfers
let all_embeddings = Tensor::cat(&embeddings, 0)?;
let all_embeddings = all_embeddings.to_device(query.device())?;
let sim = query.matmul(&all_embeddings.t()?)?;
let sim = sim.to_device(&Device::Cpu)?;

// Optimization: keep on GPU until final result
// Minimize transfers, batch operations on same device
```

### 9C: High Impact (GPU Acceleration)

| # | Optimization | Location | Impact | Effort |
|---|--------------|----------|--------|--------|
| 9C.1 | Flash Attention in T5 | embedder.rs | High | 5 commits |
| 9C.2 | GPU-accelerated search | search.rs | High | 5 commits |
| 9C.3 | GPU-side k-means | index.rs | High | 3 commits |

**9C.1 Flash Attention**

```rust
// Current (standard attention):
let scores = q.matmul(&k.t()?)?;
let scores = (scores / (head_dim as f64).sqrt())?;
let attn = candle_nn::ops::softmax(&scores, D::Minus1)?;
let output = attn.matmul(&v)?;

// Optimization: use flash attention kernel when available
#[cfg(feature = "flash-attn")]
{
    use candle_flash_attn::flash_attn;
    let output = flash_attn(&q, &k, &v, softmax_scale, causal)?;
}
```

**9C.2 GPU-Accelerated Search**

```rust
// Current: CPU-only Vec-based MaxSim
pub fn maxsim_from_vecs(query: &[f32], doc: &[f32], ...) -> f32 {
    // CPU loops
}

// Optimization: GPU tensor operations
pub fn maxsim_gpu(query: &Tensor, doc: &Tensor) -> Result<f32> {
    let sim = query.matmul(&doc.t()?)?;  // GPU matmul
    let max_per_query = sim.max(D::Minus1)?;
    let score = max_per_query.sum_all()?.to_scalar::<f32>()?;
    Ok(score)
}
```

**9C.3 GPU-Side K-Means**

```rust
// Current: CPU-side index building
cluster_assignments.to_vec1::<u32>()?.iter().enumerate().for_each(|(j, x)| {
    if *x == i as u32 { indices.push(j as u32); }
});

// Optimization: GPU scatter/gather operations
let mask = cluster_assignments.eq(i as u32)?;
let indices = mask.nonzero()?;
```

---

## Phase 10: Document Processing Pipeline

**Priority:** HIGH
**Status:** Phase 10A.1 and 10B complete

### Goal

Support rich document types beyond plain text: PDF, Office docs, images with OCR.

### Current State

- **10A.1 DONE (#425):** PDF indexing enabled for document-processing builds
- Uses `--features document-processing` flag to enable
- Layout-aware extraction uses `docling-backend` (pure Rust, no Python required)

### 10A: PDF Extraction Improvements

| # | Task | Description | Effort | Status |
|---|------|-------------|--------|--------|
| 10A.1 | Add PDF to indexable types | Enable PDF indexing in main pipeline | 1 commit | DONE #425 |
| 10A.2 | Layout-aware extraction | Preserve document structure (headers, paragraphs) | 3 commits | DONE #428 |
| 10A.3 | Table extraction | Extract tables as structured text | 2 commits | DONE #449 |
| 10A.4 | OCR fallback | Use docling-ocr for scanned PDFs | 1 commit | DONE #448 |
| 10A.5 | PDF metadata extraction | Title, author, creation date → search boost | 1 commit | DONE #432 |

**10A.2 Layout-Aware PDF Extraction**

Implemented with `docling-backend` (pdfium-backed, pure Rust) to preserve structure in markdown output.

```rust
// Layout-aware conversion (pure Rust, no Python required)
let converter = docling_backend::RustDocumentConverter::new()?;
let result = converter.convert(path)?;
let text = result.document.markdown;

// Structured extraction
struct PdfDocument {
    title: Option<String>,
    sections: Vec<PdfSection>,
    tables: Vec<Table>,
    metadata: PdfMetadata,
}

fn extract_pdf_structured(path: &Path) -> Result<PdfDocument> {
    // Layout-aware extraction preserving structure
}
```

**10A.4 OCR Fallback for Scanned PDFs**

Implemented with `docling-ocr` (PaddleOCR via ONNX Runtime) and `pdfium-render`:
- Automatically detects scanned PDFs (text < 50 chars)
- Renders PDF pages to images using pdfium
- OCR via PaddleOCR PP-OCRv4 models (ONNX)
- Enable with `--features ocr` flag
- Requires OCR models in docling-ocr assets directory

```rust
// OCR integration in document.rs
fn extract_pdf_text(path: &Path) -> Result<String> {
    // Try docling-backend first (layout-aware)
    // Then pdf_extract fallback
    // If text < 50 chars, try OCR
    #[cfg(feature = "ocr")]
    if text.trim().len() < 50 {
        return try_ocr_pdf(path);
    }
}
```

**10A.3 Table Extraction**

Implemented with `table_detector` module for heuristic-based table detection:
- Extracts text cells from PDF pages using pdfium
- Clusters cells into rows based on vertical alignment
- Detects table regions based on column structure patterns
- Converts detected tables to markdown format
- Integrated into PDF extraction pipeline (appends tables to extracted text)

```rust
// Table extraction integrated into document.rs
fn extract_pdf_text(path: &Path) -> Result<String> {
    // ... text extraction ...
    // Also extract tables
    let tables = try_extract_pdf_tables(path);
    Ok(format!("{}{}{}", metadata_header, text, tables))
}
```

Requires `pdfium` library to be available at runtime. Tables are appended as markdown after the main document text.

### 10B: Office Document Support

| # | Task | Description | Effort | Status |
|---|------|-------------|--------|--------|
| 10B.1 | DOCX extraction | Microsoft Word documents | 2 commits | DONE #426 |
| 10B.2 | XLSX extraction | Excel spreadsheets → structured text | 2 commits | DONE #426 |
| 10B.3 | PPTX extraction | PowerPoint slides | 1 commit | DONE #427 |
| 10B.4 | ODT/ODS support | OpenDocument formats | 1 commit | DONE #426 |
| 10B.5 | EPUB extraction | Ebook format | 1 commit | DONE #431 |

Implemented crates:
- `docx-lite` for Word documents (lightweight text extraction)
- `calamine` for Excel/OpenDocument spreadsheets (XLSX, XLS, XLSM, XLSB, ODS)
- `epub` for EPUB ebooks (extracts text from chapters with metadata)
- `pptx-to-md` for PowerPoint presentations (converts slides to markdown)

### 10C: Markdown Enhancements

| # | Task | Description | Effort | Status |
|---|------|-------------|--------|--------|
| 10C.1 | Frontmatter extraction | YAML/TOML frontmatter → metadata | 1 commit | DONE #433 |
| 10C.2 | Link extraction | Extract and propagate links to search results | 2 commits | DONE #453-#454 |
| 10C.3 | Embed code blocks | Special handling for fenced code | 2 commits | DONE #451-#452 |

Already have markdown_chunker, these are enhancements.

**10C.3 Code Block Language Extraction (Complete)**

Language extraction from fenced code blocks is fully implemented:
- `parse_code_fence_language()` extracts language from opening fence (e.g., \`\`\`rust → "rust")
- Language is normalized to lowercase
- Handles metadata after language (e.g., \`\`\`rust,linenos → "rust")
- Language stored in `ChunkMetadata.language` field
- Passed through to `Chunk.language` in sg-core
- Language persisted to database (chunks.language column with migration)
- Language displayed in search results as `[rust]` in magenta
- Available for future embedding routing (code vs prose models)

**10C.1 Frontmatter Extraction**

Implemented support for extracting YAML (---) and TOML (+++) frontmatter:
- Extracts title, description, author, date, tags, categories
- Handles common aliases (summary/excerpt, authors, created/published, category)
- Formats metadata as markdown header prepended to content
- Applied automatically when reading .md/.markdown/.mdx files

**10C.2 Link Extraction (Complete)**

Link extraction from markdown content is fully implemented:
- Extracts markdown links `[text](url)`, wiki-style `[[page]]` and `[[page|text]]`
- Supports reference-style links `[text][ref]` and autolinks `<url>`
- Detects internal vs external links based on URL scheme
- Links stored in chunks table as JSON (with migration for existing DBs)
- Links propagated to search results as `SearchResultLink` structs
- Available via `SearchResult.links` field in search API

---

## Phase 11: Indexing Pipeline Improvements

**Priority:** HIGH
**Status:** COMPLETE (iterations #398-#443)

### Goal

Make indexing faster, more efficient, and support incremental updates.

### 11A: Parallel Indexing

| # | Task | Description | Effort | Status |
|---|------|-------------|--------|--------|
| 11A.1 | Parallel file reading | Read files concurrently with rayon | 1 commit | DONE #409 |
| 11A.2 | Parallel chunking | Chunk documents in parallel | 1 commit | DONE #409 |
| 11A.3 | Batch embedding queue | Queue chunks, embed in batches | 2 commits | DONE #402 |
| 11A.4 | Pipeline architecture | Overlap I/O with embedding | 2 commits | DONE #403 |

**Current Pipeline (Sequential)**

```
for file in files:
    content = read(file)           # I/O bound
    chunks = chunk(content)        # CPU bound
    for chunk in chunks:
        embedding = embed(chunk)   # GPU bound
        store(embedding)           # I/O bound
```

**Improved Pipeline (Parallel)**

```
files.par_iter()                   # Parallel file iteration
    .map(|f| (f, read(f)))         # Concurrent I/O
    .flat_map(|(f, c)| chunk(c))   # Parallel chunking
    .chunks(BATCH_SIZE)            # Batch for GPU
    .for_each(|batch| {
        embeddings = embed_batch(batch)  # Batched GPU
        store_batch(embeddings)          # Batched DB writes
    })
```

### 11B: Incremental Index Updates

| # | Task | Description | Effort | Status |
|---|------|-------------|--------|--------|
| 11B.1 | Chunk-level hashing | Detect which chunks changed | 2 commits | DONE #408 |
| 11B.2 | Partial re-embedding | Only re-embed changed chunks | 2 commits | DONE #408 |
| 11B.3 | Index compaction | Remove stale chunks, defragment | 2 commits | DONE #442 |
| 11B.4 | Bloom filter for dedup | Fast duplicate detection | 1 commit | DONE #443 |

**11B.4 Implementation:**

Cross-file deduplication via `BloomDedup` Bloom filter:
- Probabilistic data structure for O(1) content hash existence checks
- ~120KB memory for 100K items at 1% false positive rate
- Persisted to DB between sessions via base64-encoded index_state
- Integrated into `index_directory_with_options_backend()` via `use_bloom_filter` option (enabled by default)
- When chunking a file, checks if content_hash exists in Bloom filter
- If "maybe exists", queries DB for existing embedding to reuse
- Skips embedding computation for duplicate chunks across files
- New content hashes added to filter after embedding

**11B.1-11B.2 Implementation:**

Chunk-level hashing and partial re-embedding via `content_hash` field:
- Each chunk has a `content_hash` (xxHash64) computed from its content
- On re-index, only chunks with changed hashes are re-embedded
- Unchanged chunks reuse existing embeddings from database
- 10-100x faster for small edits (most common case)

**11B.3 Implementation:**

Database compaction via `sg compact` command:
- Removes orphaned embeddings (embeddings without a document)
- Removes orphaned chunks (chunks without a document)
- Removes orphaned chunk_embeddings (chunk_embeddings without a chunk)
- Removes stale centroids (if no embeddings exist)
- Runs VACUUM to reclaim disk space
- Runs ANALYZE to update query planner statistics

**Current**: Hash entire file, re-embed all chunks if file changes.

**Improved**: Hash each chunk, only re-embed chunks that changed.

```rust
// Current: file-level hash
let file_hash = sha256(&content);
if file_hash != stored_hash {
    delete_all_chunks(doc_id);
    reindex_entire_file(content);
}

// Improved: chunk-level hash
let chunks = chunk_document(&content);
for (i, chunk) in chunks.iter().enumerate() {
    let chunk_hash = sha256(&chunk.content);
    if chunk_hash != get_stored_chunk_hash(doc_id, i) {
        update_chunk(doc_id, i, chunk);
    }
}
```

### 11C: Index Quality Improvements

| # | Task | Description | Effort | Status |
|---|------|-------------|--------|--------|
| 11C.1 | Adaptive cluster count | k = f(corpus_size) | 1 commit | DONE #436 |
| 11C.2 | Cluster balancing | Redistribute unbalanced buckets | 2 commits | DONE #437 |
| 11C.3 | Index health monitoring | Metrics for search quality | 1 commit | DONE #438 |
| 11C.4 | Automatic rebalancing | Trigger rebalance when skewed | 1 commit | DONE #441 |

**11C.1 Implementation:**

Adaptive cluster count based on corpus size using `compute_adaptive_cluster_count(doc_count)`:
- Formula: k ≈ sqrt(n), rounded to nearest power of 2
- Range: [16, 256] clusters
- Small corpora (<256 docs): 16 clusters
- Medium corpora (1K docs): 32 clusters
- Large corpora (10K docs): 128 clusters
- Very large corpora (>65K docs): 256 clusters (capped)

**11C.2 Implementation:**

Rebalance highly imbalanced clusters with `rebalance_clusters()`:
- Detects 100x imbalance between largest and smallest non-empty cluster
- Moves farthest embeddings from the largest cluster into underfull clusters
- Recomputes affected centers and rebuilds HNSW (if enabled)

**11C.3 Implementation:**

Comprehensive health metrics via `IndexHealthMetrics` struct:
- `cluster_count`, `total_docs`, `empty_clusters` - basic counts
- `largest_cluster`, `smallest_cluster`, `avg_cluster_size` - distribution stats
- `cluster_std_dev` - standard deviation of cluster sizes
- `imbalance_ratio` - largest/smallest ratio for detecting skew
- `health_score` - overall quality (0.0=perfect, 1.0=poor)
- `needs_rebalancing` - flag when 100x imbalance detected
- `using_quantization`, `using_hnsw`, `using_kmeans` - feature flags
- `storage_bytes` - estimated embedding storage size

The `sg status` command now displays a "Cluster Health" section showing these metrics.

**11C.4 Implementation:**

Automatic rebalancing via `auto_rebalance()` method:
- Checks `IndexHealthMetrics.needs_rebalancing` flag
- Automatically triggered every 100 `improve()` calls via `improve_counter`
- Only rebalances if corpus has >= 100 documents (ONLINE_KMEANS_THRESHOLD)
- Calls `rebalance_clusters()` when imbalance detected

### 11D: File Type Detection

| # | Task | Description | Effort | Status |
|---|------|-------------|--------|--------|
| 11D.1 | Magic byte detection | Detect file type by content, not extension | 1 commit | DONE #434 |
| 11D.2 | Binary file filtering | Skip binary files reliably | 1 commit | DONE #434 |
| 11D.3 | Encoding detection | Handle UTF-16, Latin-1, etc. | 1 commit | DONE #435 |

**11D.1-11D.2 Implementation:**

Uses the `infer` crate for magic byte detection. New public API:

**11D.3 Implementation:**

Uses `chardetng` for statistical encoding detection and `encoding_rs` for transcoding:
- `detect_encoding(buffer)` - Detect encoding from bytes (UTF-8, UTF-16, UTF-32, Latin-1, etc.)
- `decode_to_utf8(buffer)` - Auto-detect and convert to UTF-8 string
- `read_text_file(path)` - Read file with auto-detection and transcoding
- `read_text_file_utf8(path)` - Convenience wrapper returning just the string
- `is_valid_text_encoding(buffer)` - Check if buffer is valid text in any encoding

Supports:
- UTF-8 (with and without BOM)
- UTF-16 LE/BE (with and without BOM)
- UTF-32 LE/BE (with and without BOM)
- Legacy encodings (ISO-8859-1, Windows-1252, etc.) via chardetng

**11D.1-11D.2 Magic Byte API:**
- `detect_file_type(path)` - Detect file type from magic bytes
- `detect_file_type_from_buffer(buffer)` - Detect from byte buffer
- `is_binary_file(path)` - Check if file is binary (images, audio, video, executables)
- `is_indexable_by_content(path)` - Check if file should be indexed based on content
- `validate_text_file(path)` - Verify file with text extension is actually text

```rust
// Current: extension-based
if is_indexable_extension(ext) { ... }

// Improved: content-based
let file_type = infer::get_from_path(path)?;
match file_type {
    Some(t) if t.mime_type().starts_with("text/") => index_text(path),
    Some(t) if t.mime_type() == "application/pdf" => index_pdf(path),
    _ => skip_binary(path),
}
```

---

## Phase 12: Additional Features

**Priority:** LOW
**Status:** Mostly COMPLETE (6/8 items done)

| # | Feature | Description | Effort | Status |
|---|---------|-------------|--------|--------|
| 12.1 | Query caching | LRU cache for repeated queries | 1 commit | DONE #444 |
| 12.2 | Bulk search API | CSV input/output for batch queries | 1 commit | DONE #445 |
| 12.3 | FTS5 hybrid search | BM25 + semantic via RRF | 3 commits | DONE #446 |
| 12.4 | Code-specific tokenization | Language-aware tokenization | 2 commits | DONE #447 |
| 12.5 | Query preprocessing | Auto-detect and preprocess code queries | 1 commit | DONE #450 |
| 12.6 | Train projection layer | Fine-tune 768->128 on code data | 5 commits | Planned |
| 12.7 | Image search | CLIP embeddings for images | 5 commits | DONE #508-#512 |
| 12.8 | Audio transcription | Whisper for audio/video files | 1 commit | DONE #500 |

**12.8 Audio Transcription Implementation:**

Audio/video transcription using OpenAI's Whisper model via candle-transformers:
- Supports common audio formats: MP3, WAV, FLAC, M4A, OGG, OPUS, AAC, AIFF
- Supports video formats: MP4, MKV, AVI, MOV, WMV, FLV, WEBM, MPEG
- Enable with `--features audio-transcription` flag
- Audio is decoded with symphonia, resampled to 16kHz with rubato
- Model weights downloaded from HuggingFace on first use

```rust
// File type detection
is_audio_file(path)  // Check if audio file
is_video_file(path)  // Check if video file
is_media_file(path)  // Check if either

// Transcription API
let mut transcriber = Transcriber::new()?;
let text = transcriber.transcribe_file("podcast.mp3")?;

// Model sizes available
WhisperModel::Tiny   // ~39M params, fastest
WhisperModel::Base   // ~74M params, default
WhisperModel::Small  // ~244M params
WhisperModel::Medium // ~769M params
WhisperModel::LargeV3 // ~1.5B params, best quality
```

**12.5 Query Preprocessing Implementation:**

Auto-detect and preprocess code-like queries for better search results:
- Detects camelCase, snake_case, and PascalCase patterns in queries
- Preprocesses queries the same way indexed code content is preprocessed
- "getUserName" → "get user name" at query time to match indexed embeddings
- Enabled by default via `SearchOptions.preprocess_query = true`
- Natural language queries pass through unchanged

```rust
// Detection patterns:
// - camelCase: "getUserName", "parseHTTPResponse"
// - snake_case: "get_user_name", "HTTP_STATUS_CODE"
// - PascalCase: "GetUserName", "HTTPServer"

// Usage:
let options = SearchOptions {
    preprocess_query: true,  // default: true
    ..SearchOptions::default()
};
```

**12.7 Image Search Implementation (DONE #508-#512):**

CLIP model integration for cross-modal image search:
- CLIP model: `openai/clip-vit-base-patch32` (512-dim single-vector)
- Enable with `--features clip` flag
- Image formats: PNG, JPG, JPEG, GIF, BMP, WebP, TIFF, ICO, HEIC, HEIF, AVIF
- Model weights downloaded from HuggingFace on first use

**Completed:**
- ClipEmbedder: text and image embedding via CLIP
- Image preprocessing (resize 224x224, normalize)
- `is_image_file()` file type detection
- `is_indexable_path()` includes images with clip feature
- `embed_image_file()` and `embed_image_file_with_embedder()` API
- Exports from sg-core: `ClipEmbedder`, `CLIP_DIM`, `CLIP_IMAGE_SIZE`, `ImageEmbedding`
- Dual-index architecture: images and image_embeddings tables (512-dim CLIP vectors)
- `search_images()` for cross-modal text-to-image search
- `index_image()` and `index_images_in_directory()` for batch indexing
- CLI: `sg --images "query"` for image search
- CLI: `sg index-images <dir>` for image indexing

**Architecture Note:**
CLIP embeddings (512-dim single-vector, cosine similarity) are fundamentally
different from XTR text embeddings (128-dim multi-vector, MaxSim scoring).
They cannot share the same index and require separate storage/retrieval paths.

```rust
// Image embedding API
use sg_core::{embed_image_file, ClipEmbedder, CLIP_DIM};

// One-off embedding
let emb = embed_image_file(Path::new("photo.jpg"))?;
assert_eq!(emb.data.len(), CLIP_DIM);  // 512

// Batch embedding (reuse model)
let device = Device::Cpu;
let mut clip = ClipEmbedder::new(&device)?;
for path in image_paths {
    let emb = embed_image_file_with_embedder(&path, &mut clip)?;
}
```

**12.1 Query Caching Implementation:**

Added `QueryCache` for LRU caching of query embeddings:
- `query_cache.rs` module with bounded HashMap + FIFO eviction
- `semantic_search_cached()` and `search_cached()` functions
- Cache statistics (hits, misses, hit rate)
- Default capacity: 128 queries

**12.2 Bulk Search API Implementation:**

Added `sg bulk` command for batch query processing:
- CSV input from file (`-i`) or stdin
- CSV output to file (`-o`) or stdout
- Input format: `query[,limit]` columns (header optional with `--no-header`)
- Output format: `query,rank,path,score,line,quality,snippet`
- Progress bar and statistics (disable with `-q`)
- Example usage:
  ```bash
  # From CSV file
  sg bulk -i queries.csv -o results.csv

  # Piped queries (one per line)
  echo -e "function definition\nerror handling" | sg bulk -q

  # Interactive mode
  sg bulk
  ```

---

## Implementation Priority

### Immediate (Next 10 Iterations)

1. **Phase 6** - Production chunker integration (DONE #393)
2. **Phase 7** - Multi-embedding evaluation (DONE #401)
3. **Phase 8.1** - Criterion benchmarks (DONE #396-#397)

### Short-term (Next 30 Iterations)

4. **Phase 10A.1** - Add PDF to indexable types (DONE #425)
5. **Phase 11A.1-11A.2** - Parallel file reading & chunking (DONE #409)
6. **Phase 9A.1-9A.3** - Quick wins (DONE #398-#399)
7. **Phase 9B.1** - Batch embedding generation (DONE #398, #402)

### Medium-term (Next 100 Iterations)

8. **Phase 9B.2** - Parallel document scoring (DONE #423)
9. **Phase 10A.2-10A.4** - Layout-aware PDF, tables, OCR
10. **Phase 11B.1-11B.2** - Chunk-level hashing, partial re-embedding
11. **Phase 9C.1** - Flash attention
12. **Phase 10B** - Office document support (DOCX, XLSX, PPTX)

### Long-term (Future)

13. **Phase 9C.2-9C.3** - GPU-accelerated search & k-means
14. **Phase 11C** - Index quality (adaptive clusters, balancing)
15. **Phase 12** - Additional features (CLIP images, Whisper audio)

---

## Success Metrics

| Milestone | Metric | Target | Verification |
|-----------|--------|--------|--------------|
| Phase 6 | Search quality | P@1 >= 0.80 | `sg eval` |
| Phase 7 | Model comparison | Code embed +20% on code | `eval/MULTI_EMBEDDING_RESULTS.md` |
| Phase 8 | Benchmark coverage | All hot paths covered | `cargo bench` |
| Phase 9 | Search latency | p50 < 50ms | `sg benchmark` |

---

## Architecture Comparison: sg vs rust-warp

| Aspect | rust-warp | sg | Winner |
|--------|-----------|-----|--------|
| Chunking | Document-level | Chunk-level (350 words) | sg |
| Quantization | 8-bit + 4-bit residuals | Product Quantization (32x) | sg |
| Indexing | Batch k-means | Progressive LSH -> online k-means | sg |
| Incremental updates | Full rebuild | O(1) add | sg |
| Search acceleration | GPU matmul | CPU-only (to be improved) | rust-warp |
| Backends | Candle only | Candle, ONNX, CoreML, CUDA, etc. | sg |
| File watching | No | Yes | sg |
| Node.js bindings | Yes | No | rust-warp |

**Conclusion:** sg has better architecture for end-user CLI tool. rust-warp patterns should be backported for search performance (GPU acceleration, batch operations).

---

## Optimization Impact Summary

| # | Optimization | Location | Impact | Phase |
|---|--------------|----------|--------|-------|
| 1 | Batch embedding generation | embedder.rs | High - GPU utilization | 9B.1 |
| 2 | Flash attention | embedder.rs | High - memory & speed | 9C.1 |
| 3 | Static inv_compand table | quantizer.rs | Low - avoid recompute | 9A.1 |
| 4 | Vectorize stretch_rows | quantizer.rs | Medium - CPU vectorization | 9B.3 |
| 5 | FTS5 optimize vs rebuild | storage.rs | Medium - startup time | 9A.2 |
| 6 | Eliminate temp tables | search.rs | Medium - reduce SQL overhead | 9A.3 |
| 7 | GPU-side k-means | index.rs | High - reduce transfers | 9C.3 |
| 8 | Consolidate device transfers | embedder.rs | Medium - reduce latency | 9B.4 |
| 9 | Parallel document scoring | index.rs | High - multicore scaling | DONE #423 |
| 10 | GPU-accelerated search | search.rs | High - matmul speedup | 9C.2 |

---

## References

- rust-warp: `~/rust-warp/` (documented inspiration)
- XTR paper: https://arxiv.org/abs/2304.01982
- WARP paper: https://arxiv.org/abs/2501.17788
- Product Quantization: https://arxiv.org/abs/1106.2069
- HNSW: https://arxiv.org/abs/1603.09320
- CodeSearchNet: https://github.com/github/CodeSearchNet
- UniXcoder: https://huggingface.co/microsoft/unixcoder-base
- Jina Code: https://huggingface.co/jinaai/jina-embeddings-v2-base-code
