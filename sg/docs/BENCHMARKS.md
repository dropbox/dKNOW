# SuperGrep Benchmarks

Benchmark results from `sg benchmark` command. All tests run on macOS with Metal GPU acceleration.

## Hardware

- Apple Silicon (M-series)
- Metal GPU acceleration enabled
- Device: `Metal(MetalDevice(DeviceId(1)))`

## Results

### Small Project (31 files, 15K lines)

| Metric | Value |
|--------|-------|
| Files | 31 |
| Lines | 15,009 |
| Index time | 2.02s |
| Index rate | 15.3 files/s |
| Index size | 7.3 MB |
| **Search latency (avg)** | **16.2ms** |
| **Search latency (p50)** | **16.1ms** |
| **Search latency (p95)** | **17.2ms** |
| Search latency (p99) | 17.4ms |
| Search throughput | 61.9 queries/s |
| Peak memory | 0.9 GB |

### Medium Project (560 files, 409K lines)

| Metric | Value |
|--------|-------|
| Files | 560 |
| Lines | 408,786 |
| Index time | 34.57s |
| Index rate | 16.2 files/s |
| Index size | 125.3 MB |
| **Search latency (avg)** | **172.1ms** |
| **Search latency (p50)** | **173.9ms** |
| **Search latency (p95)** | **175.7ms** |
| Search latency (p99) | 176.6ms |
| Search throughput | 5.8 queries/s |

### Large-Medium Project (5.3K files, 1.9M lines)

| Metric | Value |
|--------|-------|
| Files | 5,275 |
| Lines | 1,945,569 |
| Index time | 386s (~6.4 min) |
| Index rate | 13.7 files/s |
| Index size | 1.4 GB |
| **Search latency (avg)** | **1608ms** |
| **Search latency (p50)** | **1610ms** |
| **Search latency (p95)** | **1615ms** |
| Search latency (p99) | 1615ms |
| Search throughput | 0.6 queries/s |
| Peak memory | 8.4 GB |

### Large Codebase (32K files, 14M lines)

| Metric | Value |
|--------|-------|
| Files | 32,695 |
| Lines | 14,185,130 |
| Index time | 2011s (~33 min) |
| Index rate | 16.3 files/s |
| Index size | 7.2 GB |
| **Search latency (avg)** | **9624ms** |
| **Search latency (p50)** | **9583ms** |
| **Search latency (p95)** | **10063ms** |
| Search latency (p99) | 10105ms |
| Search throughput | 0.1 queries/s |
| Peak memory | ~45 GB (estimated) |

## Analysis

**Note:** These benchmarks were run with brute-force search. As of iteration #420,
the CLI uses **clustered search with HNSW** for indices with >100 documents, which
should significantly improve search latency for large codebases.

- **Small projects (<100 files)**: Search latency ~16ms, well under 100ms target
- **Medium projects (100-1K files)**: Search latency ~170ms, reasonable for interactive use
- **Large-medium projects (1K-10K files)**: Search latency ~1.6s, acceptable for batch use
- **Large codebases (10K+ files)**: Search latency increases proportionally with index size (~10s for 32K files)
- **Index throughput**: Consistent 13-16 files/s regardless of codebase size
- **Model load time**: ~0.4s with Metal acceleration
- **Memory usage**: ~0.9 GB base + scales with index size (8.4 GB for 1.4 GB index)

## Component Microbenchmarks

Criterion benchmarks measuring individual component performance (run via `cargo bench -p sg-core`).

### MaxSim Scoring (Hot Path)

MaxSim computes similarity between multi-vector query and document embeddings.

| Benchmark | Time | Throughput |
|-----------|------|------------|
| 8 query tokens × 100 doc tokens | 101 µs | 9,900/s |
| 16 query tokens × 100 doc tokens | 204 µs | 4,900/s |
| 32 query tokens × 100 doc tokens | 406 µs | 2,460/s |
| 64 query tokens × 100 doc tokens | 828 µs | 1,208/s |
| 16 query tokens × 50 doc tokens | 100 µs | 10,000/s |
| 16 query tokens × 200 doc tokens | 204 µs | 4,900/s |
| 16 query tokens × 400 doc tokens | 791 µs | 1,264/s |

**Batch Scoring (typical search operation):**

| Documents | Time | Per-doc |
|-----------|------|---------|
| 10 docs | 2.06 ms | 206 µs |
| 50 docs | 10.3 ms | 206 µs |
| 100 docs | 20.3 ms | 203 µs |
| 500 docs | 79.3 ms | 159 µs |

Scaling is roughly linear O(n) with excellent cache efficiency at higher batch sizes.

### Embedding Generation (XTR Model)

Text embedding with Metal GPU acceleration on Apple Silicon.

| Input Size | Time | Tokens/s |
|------------|------|----------|
| Query (3 words) | 6.98 ms | ~4 tok |
| Query (5 words) | 6.86 ms | ~7 tok |
| Query (10 words) | 7.65 ms | ~13 tok |
| Query (20 words) | 8.26 ms | ~24 tok |
| Document (50 words) | 8.11 ms | ~65 tok |
| Document (100 words) | 13.6 ms | ~130 tok |
| Document (200 words) | 26.6 ms | ~260 tok |
| Document (350 words) | 56.1 ms | ~450 tok |

**Batch Embedding:**

| Batch Size | Time | Per-doc | Speedup vs Single |
|------------|------|---------|-------------------|
| 1 doc | 13.6 ms | 13.6 ms | 1.0× |
| 4 docs | 63.4 ms | 15.9 ms | 0.86× |
| 8 docs | 149 ms | 18.6 ms | 0.73× |
| 16 docs | 388 ms | 24.3 ms | 0.56× |

**Note:** Batch embedding shows sublinear scaling due to GPU memory bandwidth limits.
For best throughput, use batch sizes of 4-8 on Apple Silicon.

## Running Benchmarks

```bash
# End-to-end benchmark (current directory)
sg benchmark

# Benchmark specific path with 20 iterations
sg benchmark /path/to/code -n 20

# Capture peak resident memory (macOS)
/usr/bin/time -l sg benchmark /path/to/code -n 20

# Build release binary first for accurate results (Metal on macOS)
cargo build --release --features metal
./target/release/sg benchmark

# Run criterion microbenchmarks
cargo bench -p sg-core --bench maxsim_bench
cargo bench -p sg-core --bench embedding_bench
cargo bench -p sg-core --bench search_bench
```
