# Indexing Performance Roadmap

Fast indexing for both cold start (initial corpus) and continuous updates (file changes).

## Current Architecture Analysis

**Pipeline:** File → Chunks → Embed (1 at a time) → SQLite

**Bottlenecks identified:**
1. **Sequential embedding** - chunks embedded one-by-one (`search.rs:627-634`)
2. **Sequential file processing** - files indexed one-by-one (`search.rs:706-728`)
3. **Per-chunk DB transactions** - each `add_chunk_embeddings` is separate
4. **Synchronous I/O** - file reads block embedding
5. **No warm-up** - model loads on first embed, not at startup

---

## Phase 1: Batch Embedding (High Impact, Medium Effort)

### 1.1 Batch Chunks Within a File

The embedder already has `embed_batch()` but it's unused. Batch all chunks from a file:

```rust
// Current (search.rs:627-634)
for chunk in chunks {
    let chunk_id = db.add_chunk(...)?;
    let result = embedder.embed_document(&chunk.content)?;  // One at a time
    db.add_chunk_embeddings(chunk_id, ...)?;
}

// Improved: batch embedding
let texts: Vec<&str> = chunks.iter().map(|c| c.content.as_str()).collect();
let embeddings = embedder.embed_batch(&texts, DOC_MAXLEN)?;  // All at once

for (chunk, embedding) in chunks.iter().zip(embeddings.iter()) {
    let chunk_id = db.add_chunk(...)?;
    db.add_chunk_embeddings(chunk_id, &embedding_to_vec(embedding)?, ...)?;
}
```

**Expected Impact:** 3-5x faster per-file indexing (GPU utilization, reduced kernel launch overhead)

### 1.2 Cross-File Batch Embedding

For cold start, batch embeddings across multiple files:

```rust
const BATCH_SIZE: usize = 64;

// Collect chunks from multiple files
let mut batch: Vec<(PathBuf, usize, Chunk)> = Vec::new();

for file_path in files {
    let content = fs::read_to_string(&file_path)?;
    let chunks = chunk_document(&content);

    for chunk in chunks {
        batch.push((file_path.clone(), chunk.index, chunk));

        if batch.len() >= BATCH_SIZE {
            process_batch(&mut batch, db, embedder)?;
        }
    }
}
// Process remaining
if !batch.is_empty() {
    process_batch(&mut batch, db, embedder)?;
}
```

**Expected Impact:** 5-10x faster cold start (maximizes GPU/NPU throughput)

---

## Phase 2: Parallel File Processing (High Impact, Medium Effort)

### 2.1 Parallel File Reading with Async I/O

Use rayon for CPU-bound work, tokio for I/O:

```rust
use rayon::prelude::*;

// Parallel file collection and chunking
let file_chunks: Vec<_> = files.par_iter()
    .filter_map(|path| {
        let content = fs::read_to_string(path).ok()?;
        if content.trim().is_empty() { return None; }
        let chunks = chunk_document(&content);
        Some((path.clone(), content, chunks))
    })
    .collect();

// Sequential embedding (GPU is the bottleneck)
for (path, content, chunks) in file_chunks {
    // ... embed and store
}
```

**Expected Impact:** 2-3x faster file discovery and chunking

### 2.2 Pipeline Architecture

Producer-consumer pattern to overlap I/O and embedding:

```
┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│  File Reader │───>│   Embedder   │───>│  DB Writer   │
│   (Async)    │    │   (Batch)    │    │  (Batched)   │
└──────────────┘    └──────────────┘    └──────────────┘
     Thread 1           Thread 2           Thread 3
```

```rust
use crossbeam_channel::{bounded, Receiver, Sender};

fn index_pipeline(files: Vec<PathBuf>, embedder: &mut Embedder, db: &DB) {
    let (file_tx, file_rx) = bounded::<(PathBuf, Vec<Chunk>)>(32);
    let (embed_tx, embed_rx) = bounded::<(PathBuf, Vec<(Chunk, Vec<f32>)>)>(16);

    // Reader thread
    thread::spawn(move || {
        for path in files {
            let content = fs::read_to_string(&path)?;
            let chunks = chunk_document(&content);
            file_tx.send((path, chunks))?;
        }
    });

    // Embedder thread (main, owns embedder)
    thread::spawn(move || {
        let mut batch = Vec::new();
        while let Ok((path, chunks)) = file_rx.recv() {
            batch.extend(chunks.into_iter().map(|c| (path.clone(), c)));
            if batch.len() >= 64 {
                let embedded = embed_batch(&batch, embedder)?;
                embed_tx.send(embedded)?;
                batch.clear();
            }
        }
    });

    // Writer thread
    for (path, chunk_embeddings) in embed_rx {
        db.batch_insert_chunks(&path, &chunk_embeddings)?;
    }
}
```

**Expected Impact:** Near 100% utilization of I/O, GPU, and DB simultaneously

---

## Phase 3: Database Optimizations (Medium Impact, Low Effort)

### 3.1 Batch Inserts with Single Transaction

Current: Each chunk is a separate transaction.
Improved: Batch all chunks in one transaction.

```rust
impl DB {
    pub fn batch_add_chunks(
        &self,
        doc_id: u32,
        chunks: &[(usize, usize, usize, &str, &[f32], usize)],  // index, start, end, header, emb, n_tok
    ) -> Result<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;

        {
            let mut chunk_stmt = tx.prepare_cached(
                "INSERT INTO chunks (doc_id, chunk_index, start_line, end_line, header_context)
                 VALUES (?, ?, ?, ?, ?)"
            )?;
            let mut emb_stmt = tx.prepare_cached(
                "INSERT INTO indexed_chunk2 (chunk_id, embeddings, num_tokens)
                 VALUES (?, ?, ?)"
            )?;

            for (index, start, end, header, emb, n_tok) in chunks {
                chunk_stmt.execute(params![doc_id, index, start, end, header])?;
                let chunk_id = tx.last_insert_rowid();
                emb_stmt.execute(params![chunk_id, emb, n_tok])?;
            }
        }

        tx.commit()?;
        Ok(())
    }
}
```

**Expected Impact:** 5-10x faster DB writes (reduced fsync overhead)

### 3.2 Prepared Statement Caching

Ensure all hot-path queries use `prepare_cached`:

```rust
// Current: may recreate statement each call
let mut stmt = conn.prepare("SELECT ...")?;

// Improved: cache across calls
let mut stmt = conn.prepare_cached("SELECT ...")?;
```

### 3.3 Bulk Insert with INSERT OR REPLACE

For re-indexing, use upsert pattern:

```sql
INSERT OR REPLACE INTO chunks (doc_id, chunk_index, start_line, end_line, header_context)
VALUES (?, ?, ?, ?, ?)
```

Avoids separate DELETE + INSERT.

### 3.4 Deferred Foreign Key Checks

During bulk indexing, defer FK checks:

```rust
conn.execute_batch("PRAGMA defer_foreign_keys = ON")?;
// ... bulk inserts ...
conn.execute_batch("PRAGMA defer_foreign_keys = OFF")?;
```

---

## Phase 4: Incremental/Continuous Updates (High Impact, High Effort)

### 4.1 File Watcher Integration

Use notify crate for real-time file watching:

```rust
use notify::{Watcher, RecursiveMode, watcher};

fn start_watcher(db: Arc<DB>, embedder: Arc<Mutex<Embedder>>) -> Result<()> {
    let (tx, rx) = channel();
    let mut watcher = watcher(tx, Duration::from_millis(100))?;

    watcher.watch("/path/to/watch", RecursiveMode::Recursive)?;

    loop {
        match rx.recv() {
            Ok(DebouncedEvent::Write(path)) | Ok(DebouncedEvent::Create(path)) => {
                let mut embedder = embedder.lock().unwrap();
                index_file_backend(&db, &mut *embedder, &path)?;
            }
            Ok(DebouncedEvent::Remove(path)) => {
                db.remove_document(&path)?;
            }
            _ => {}
        }
    }
}
```

### 4.2 Content-Hash Based Change Detection

Only re-index if content actually changed:

```rust
impl DB {
    pub fn needs_reindex(&self, path: &Path, content: &str) -> Result<bool> {
        let hash = xxhash_rust::xxh3::xxh3_64(content.as_bytes());

        if let Some(doc) = self.get_document_by_path(path)? {
            Ok(doc.content_hash != hash)
        } else {
            Ok(true)  // New file
        }
    }
}
```

**Note:** sg already has this (`needs_reindex` at line 606), but ensure hash comparison is efficient.

### 4.3 Differential Chunk Updates

Only re-embed changed chunks, not entire files:

```rust
fn update_file_incremental(db: &DB, embedder: &mut Embedder, path: &Path) -> Result<()> {
    let old_chunks = db.get_chunks_for_document(path)?;
    let new_content = fs::read_to_string(path)?;
    let new_chunks = chunk_document(&new_content);

    let old_set: HashSet<_> = old_chunks.iter().map(|c| hash_chunk(c)).collect();
    let new_set: HashSet<_> = new_chunks.iter().map(|c| hash_chunk(c)).collect();

    // Remove deleted chunks
    for chunk in old_chunks.iter().filter(|c| !new_set.contains(&hash_chunk(c))) {
        db.delete_chunk(chunk.id)?;
    }

    // Add new chunks only
    for chunk in new_chunks.iter().filter(|c| !old_set.contains(&hash_chunk(c))) {
        let result = embedder.embed_document(&chunk.content)?;
        let chunk_id = db.add_chunk(...)?;
        db.add_chunk_embeddings(chunk_id, ...)?;
    }

    Ok(())
}
```

**Expected Impact:** 10-100x faster for small edits (most common case)

### 4.4 Background Index Improvement

Run online k-means optimization during idle time:

```rust
impl LazyIndex {
    /// Run background optimization (call periodically)
    pub fn background_optimize(&mut self, max_iterations: usize) {
        for _ in 0..max_iterations {
            if !self.needs_work() {
                break;
            }
            self.improve();
        }

        // Periodically rebuild HNSW
        if self.use_hnsw && self.total_seen % 10000 == 0 {
            self.rebuild_hnsw();
        }
    }
}
```

---

## Phase 5: Cold Start Optimizations (High Impact, Medium Effort)

### 5.1 Model Pre-warming

Load model at startup, not first embed:

```rust
fn main() {
    // Warm up embedder immediately
    let device = make_device();
    let mut embedder = Embedder::new(&device)?;

    // Run dummy inference to initialize GPU kernels
    let _ = embedder.embed_document("warmup")?;

    // Now ready for fast indexing
    index_directory(&db, &mut embedder, &path)?;
}
```

### 5.2 Parallel Model Loading

Load tokenizer and model weights in parallel:

```rust
use rayon::join;

fn load_embedder_parallel() -> Result<Embedder> {
    let (tokenizer, model) = join(
        || Tokenizer::from_file("tokenizer.json"),
        || load_model_weights("model.safetensors"),
    );

    Embedder::from_parts(tokenizer?, model?)
}
```

### 5.3 Memory-Mapped Model Weights

Use mmap for faster model loading:

```rust
use memmap2::Mmap;

fn load_weights_mmap(path: &Path) -> Result<Mmap> {
    let file = File::open(path)?;
    unsafe { Mmap::map(&file) }
}
```

Candle supports this via `safetensors` with mmap.

### 5.4 Index Persistence with Fast Reload

Save index state to disk for instant cold start:

```rust
impl LazyIndex {
    pub fn save(&self, path: &Path) -> Result<()> {
        let state = IndexState {
            centers: self.export_centers(),
            quantizer: self.quantizer.as_ref().map(|q| q.serialize()),
            hnsw: self.hnsw.as_ref().map(|h| h.serialize()),
        };

        let file = File::create(path)?;
        bincode::serialize_into(file, &state)?;
        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let state: IndexState = bincode::deserialize_from(file)?;

        let mut index = LazyIndex::new(state.centers.1);
        index.import_centers(&state.centers.0, state.centers.1)?;
        // ... restore quantizer, hnsw
        Ok(index)
    }
}
```

**Expected Impact:** Sub-second startup for existing indexes

---

## Phase 6: Hardware-Specific Optimizations

### 6.1 GPU Batch Size Tuning

Auto-tune batch size based on available VRAM:

```rust
fn optimal_batch_size(device: &Device) -> usize {
    match device {
        Device::Cuda(_) => {
            // ~2GB VRAM per batch of 64 with T5-base
            let vram_gb = get_cuda_vram_gb();
            ((vram_gb / 2.0) * 64.0) as usize
        }
        Device::Metal(_) => {
            // Apple Silicon unified memory
            64  // Conservative default
        }
        Device::Cpu => 8,  // CPU is memory-limited
    }
}
```

### 6.2 CoreML/ANE for Apple Silicon

sg already has CoreML backend - ensure ANE (Neural Engine) is used:

```rust
// In embedder_coreml.rs
let config = MLModelConfiguration::new();
config.set_compute_units(MLComputeUnits::All);  // Include ANE
```

### 6.3 TensorRT for NVIDIA

sg has TensorRT backend - optimize for specific GPU:

```rust
// Build TensorRT engine with FP16
let builder_config = builder.create_builder_config()?;
builder_config.set_flag(BuilderFlag::Fp16)?;
builder_config.set_memory_pool_limit(MemoryPoolType::Workspace, 1 << 30)?;
```

---

## Implementation Priority

| Priority | Item | Effort | Impact | Dependencies | Status |
|----------|------|--------|--------|--------------|--------|
| **P0** | Batch chunks within file | Low | High | None | ✓ #398 |
| **P0** | Batch DB inserts | Low | High | None | ✓ #398 |
| **P0** | Model pre-warming | Low | Medium | None | ✓ #398 |
| **P1** | Cross-file batch embedding | Medium | Very High | P0 | ✓ #402 |
| **P1** | Pipeline architecture | Medium | High | P0 | ✓ #403 |
| **P1** | Prepared statement caching | Low | Medium | None | ✓ #399 |
| **P2** | Differential chunk updates | High | High | None | ✓ #408 |
| **P2** | File watcher integration | Medium | High | None | ✓ daemon |
| **P2** | Index persistence | Medium | High | None | ✓ #407 |
| **P3** | Parallel file reading | Medium | Medium | None | ✓ #409 |
| **P3** | GPU batch size tuning | Low | Medium | P1 | ✓ #410 |

---

## Benchmarks to Track

```rust
// benches/indexing_benchmark.rs
use criterion::{criterion_group, Criterion};

fn bench_indexing(c: &mut Criterion) {
    let corpus = load_test_corpus();  // 1000 files, ~100KB each

    c.bench_function("cold_start_1000_files", |b| {
        b.iter(|| {
            let db = DB::new_in_memory()?;
            let mut embedder = Embedder::new(&device)?;
            index_directory(&db, &mut embedder, &corpus)?;
        })
    });

    c.bench_function("reindex_single_file", |b| {
        // Pre-populate index
        let db = setup_index(&corpus)?;
        let file = &corpus.files[0];

        b.iter(|| {
            index_file(&db, &mut embedder, file)?;
        })
    });

    c.bench_function("incremental_small_edit", |b| {
        let db = setup_index(&corpus)?;
        let file = modify_file(&corpus.files[0], 1);  // 1 line change

        b.iter(|| {
            update_file_incremental(&db, &mut embedder, &file)?;
        })
    });
}
```

### Target Metrics

| Metric | Current | Target |
|--------|---------|--------|
| Cold start (1000 files) | TBD | <30s |
| Single file reindex | TBD | <100ms |
| Incremental update (1 line) | TBD | <50ms |
| Embedding throughput | TBD | >500 chunks/sec |
| DB write throughput | TBD | >1000 chunks/sec |

---

## Quick Wins Checklist

- [x] Use `embed_batch()` for chunks within a file (done #398)
- [x] Wrap file's chunks in single DB transaction (done #398)
- [x] Add `prepare_cached` to hot-path queries (done #399)
- [x] Warm up embedder model at startup (done #398)
- [x] Add content hash check before chunking (already exists in `needs_reindex`)
- [x] Enable WAL mode (already done in sg)
- [x] Cross-file batch embedding (done #402) - batches chunks across files in groups of 64
- [x] Pipeline architecture (done #403) - overlaps file I/O with embedding for faster cold start
- [x] Index persistence (done #407) - save/load LazyIndex state for fast startup
- [x] Differential chunk updates (done #408) - reuse embeddings for unchanged chunks, 10-100x faster for small edits
- [x] Parallel file reading (done #409) - uses rayon to parallelize I/O and chunking across CPU cores
- [x] GPU batch size tuning (done #410) - auto-tune batch size based on device type via `--batch-size auto`
- [x] Bloom filter for cross-file dedup (done #443) - O(1) content hash checks, reuse embeddings across files

---

## Test Corpus: pdfium_fast PDFs

Use the pdfium_fast test corpus for realistic benchmarking:

**Source:** `git@github.com:dropbox/pdfium_fast.git`
**Release:** https://github.com/dropbox/pdfium_fast/releases/tag/test-pdfs-v1

### Corpus Stats
- **567 PDFs** in repo (testing/resources, integration_tests/pdfs)
- **462 PDFs** in release tarball (1.4GB compressed)
- Categories: edge cases, bug reproductions, XFA forms, encrypted, scanned

### Setup

```bash
# Clone repo (has 567 PDFs already)
git clone git@github.com:dropbox/pdfium_fast.git ~/pdfium_fast

# Or download release corpus
mkdir -p ~/test-pdfs
cd ~/test-pdfs
curl -L https://github.com/dropbox/pdfium_fast/releases/download/test-pdfs-v1/pdfium_test_pdfs.tar.gz | tar xz
```

### Benchmark Integration

```rust
// benches/pdf_indexing_benchmark.rs
use criterion::{criterion_group, Criterion, BenchmarkId};
use std::path::PathBuf;

const PDFIUM_CORPUS: &str = env!("PDFIUM_TEST_PDFS", "~/pdfium_fast/testing/resources");

fn load_pdf_corpus() -> Vec<PathBuf> {
    walkdir::WalkDir::new(PDFIUM_CORPUS)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "pdf").unwrap_or(false))
        .map(|e| e.path().to_path_buf())
        .collect()
}

fn bench_pdf_indexing(c: &mut Criterion) {
    let pdfs = load_pdf_corpus();
    let mut group = c.benchmark_group("pdf_indexing");

    // Cold start: index all PDFs
    group.bench_function("cold_start_all_pdfs", |b| {
        b.iter_batched(
            || {
                let db = DB::new_in_memory().unwrap();
                let embedder = Embedder::new(&make_device()).unwrap();
                (db, embedder, pdfs.clone())
            },
            |(db, mut embedder, pdfs)| {
                for pdf in pdfs {
                    let _ = index_pdf(&db, &mut embedder, &pdf);
                }
            },
            criterion::BatchSize::PerIteration,
        )
    });

    // Sample sizes for scaling tests
    for size in [10, 50, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::new("index_n_pdfs", size),
            size,
            |b, &size| {
                let sample: Vec<_> = pdfs.iter().take(size).cloned().collect();
                b.iter(|| {
                    let db = DB::new_in_memory().unwrap();
                    let mut embedder = Embedder::new(&make_device()).unwrap();
                    for pdf in &sample {
                        let _ = index_pdf(&db, &mut embedder, pdf);
                    }
                })
            },
        );
    }

    group.finish();
}

fn bench_pdf_categories(c: &mut Criterion) {
    let mut group = c.benchmark_group("pdf_categories");

    // Edge cases (malformed, unusual structures)
    let edge_cases: Vec<PathBuf> = glob::glob(&format!("{}/edge_cases/*.pdf", PDFIUM_CORPUS))
        .unwrap()
        .filter_map(|p| p.ok())
        .collect();

    group.bench_function("edge_case_pdfs", |b| {
        b.iter(|| {
            let db = DB::new_in_memory().unwrap();
            let mut embedder = Embedder::new(&make_device()).unwrap();
            for pdf in &edge_cases {
                let _ = index_pdf(&db, &mut embedder, pdf);
            }
        })
    });

    // XFA forms (complex interactive PDFs)
    let xfa_pdfs: Vec<PathBuf> = glob::glob(&format!("{}/xfa/*.pdf", PDFIUM_CORPUS))
        .unwrap()
        .filter_map(|p| p.ok())
        .collect();

    group.bench_function("xfa_form_pdfs", |b| {
        b.iter(|| {
            let db = DB::new_in_memory().unwrap();
            let mut embedder = Embedder::new(&make_device()).unwrap();
            for pdf in &xfa_pdfs {
                let _ = index_pdf(&db, &mut embedder, pdf);
            }
        })
    });

    group.finish();
}

criterion_group!(pdf_benches, bench_pdf_indexing, bench_pdf_categories);
```

### Test Cases from Corpus

| Category | Count | Use Case |
|----------|-------|----------|
| `testing/resources/` | ~400 | General PDF parsing |
| `testing/resources/xfa/` | ~20 | XFA form extraction |
| `integration_tests/pdfs/edge_cases/` | ~50 | Error handling, malformed PDFs |
| `integration_tests/pdfs/scanned/` | ~10 | OCR/image-heavy PDFs |
| Encrypted PDFs | ~5 | `encrypted_hello_world_r*.pdf` |

### Regression Tests

```rust
#[test]
fn test_index_all_corpus_pdfs() {
    let corpus_path = std::env::var("PDFIUM_TEST_PDFS")
        .unwrap_or_else(|_| "~/pdfium_fast/testing/resources".to_string());

    let pdfs = load_pdf_corpus(&corpus_path);
    let db = DB::new_in_memory().unwrap();
    let mut embedder = Embedder::new(&Device::Cpu).unwrap();

    let mut successes = 0;
    let mut failures = Vec::new();

    for pdf in &pdfs {
        match index_pdf(&db, &mut embedder, pdf) {
            Ok(_) => successes += 1,
            Err(e) => failures.push((pdf.clone(), e.to_string())),
        }
    }

    // Report results
    println!("Indexed {}/{} PDFs successfully", successes, pdfs.len());

    // Allow some failures for known edge cases
    let failure_rate = failures.len() as f64 / pdfs.len() as f64;
    assert!(
        failure_rate < 0.05,
        "Too many failures ({:.1}%): {:?}",
        failure_rate * 100.0,
        failures.iter().take(10).collect::<Vec<_>>()
    );
}

#[test]
fn test_encrypted_pdf_handling() {
    let encrypted_pdfs = [
        "encrypted_hello_world_r2.pdf",
        "encrypted_hello_world_r3.pdf",
    ];

    for pdf_name in &encrypted_pdfs {
        let path = format!("{}/edge_cases/{}", PDFIUM_CORPUS, pdf_name);
        let result = index_pdf_file(&path);

        // Should either succeed (if password is empty) or fail gracefully
        match result {
            Ok(_) => println!("{}: indexed successfully", pdf_name),
            Err(e) => {
                assert!(
                    e.to_string().contains("encrypted") || e.to_string().contains("password"),
                    "{}: unexpected error: {}",
                    pdf_name,
                    e
                );
            }
        }
    }
}

#[test]
fn test_edge_case_pdfs_dont_crash() {
    let edge_cases = glob::glob(&format!("{}/edge_cases/*.pdf", PDFIUM_CORPUS))
        .unwrap()
        .filter_map(|p| p.ok());

    let db = DB::new_in_memory().unwrap();
    let mut embedder = Embedder::new(&Device::Cpu).unwrap();

    for pdf in edge_cases {
        // Should not panic, errors are acceptable
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = index_pdf(&db, &mut embedder, &pdf);
        }));
    }
}
```

### CI Integration

```yaml
# .github/workflows/benchmark.yml
name: PDF Indexing Benchmarks

on:
  push:
    branches: [main]
  pull_request:

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Download test PDFs
        run: |
          mkdir -p test-pdfs
          curl -L https://github.com/dropbox/pdfium_fast/releases/download/test-pdfs-v1/pdfium_test_pdfs.tar.gz | tar xz -C test-pdfs

      - name: Run benchmarks
        env:
          PDFIUM_TEST_PDFS: ./test-pdfs
        run: cargo bench --bench pdf_indexing_benchmark

      - name: Upload benchmark results
        uses: actions/upload-artifact@v4
        with:
          name: benchmark-results
          path: target/criterion/
```

---

*December 2024 - Analysis of sg indexing pipeline*
