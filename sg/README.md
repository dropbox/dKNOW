# sg - SuperGrep

| Director | Status |
|:--------:|:------:|
| KNOW | ACTIVE |

[![Build](https://img.shields.io/badge/build-passing-brightgreen)]()
[![Tests](https://img.shields.io/badge/tests-675%20passing-brightgreen)]()
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange)]()
[![License](https://img.shields.io/badge/license-MIT-blue)]()

Semantic code search that understands meaning, not just text.

```bash
# Traditional grep - exact matches only
$ grep "handle auth" -r .
(nothing)

# SuperGrep - understands intent
$ sg "handle auth"
1. src/auth/login.ts:42 (score: 0.85)
      41 | // Validate user credentials
      42 | function validateCredentials(user, password) {
      43 |   return checkPassword(user.passwordHash, password);

2. src/middleware/session.ts:18 (score: 0.78)
      17 | // Check session token validity
      18 | function checkSessionToken(req) {
      19 |   return sessions.verify(req.headers.authorization);
```

## What is this?

`sg` is a semantic search tool for code. It uses multi-vector embeddings (XTR-WARP) to find code by meaning rather than exact text matches. Think of it as "grep that understands what you're looking for."

**Key features:**
- **Semantic search** - finds code by concept, not just keywords
- **Zero config** - indexes projects automatically
- **Background daemon** - indexes continuously without blocking
- **Hybrid search** - combines semantic + keyword matching
- **JSON output** - scriptable with `--json` flag
- **Document processing** - indexes PDF, DOCX, XLSX, PPTX, EPUB files (requires `document-processing` feature)
- **Audio transcription** - indexes audio/video files via Whisper (requires `audio-transcription` feature)
- **Multi-model support** - routes code vs prose to specialized embedders

## Installation

```bash
# Build from source
cargo build --release

# Apple Silicon (optional Metal acceleration)
cargo build --release --features metal

# Optional ONNX Runtime backend
cargo build --release --features onnx
SG_EMBEDDER_BACKEND=onnx sg "search query"

# Optional document processing for PDFs and Office files
cargo build --release --features document-processing

# Optional audio/video transcription with Whisper
cargo build --release --features audio-transcription

# Optional CoreML backend (macOS only, uses ONNX Runtime with CoreML acceleration)
cargo build --release --features coreml
SG_EMBEDDER_BACKEND=coreml sg "search query"

# Install binaries
cargo install --path crates/sg
cargo install --path crates/sg-daemon
```

## Quick Start

```bash
# Index a directory
sg index ~/code/myproject

# Index with automatic model selection (code vs prose)
# Uses UniXcoder if >50% of files are code (by extension), otherwise XTR
sg index --auto-model ~/code/myproject

# Search (hybrid: semantic + keyword, default)
sg "error handling"

# Search with semantic-only mode
sg "database connection" --no-hybrid

# Output as JSON
sg "auth flow" --json

# Limit results
sg "parse config" -n 5

# Start background daemon (optional, for faster repeated searches)
sg daemon start

# Check status
sg status
```

## Commands

### Search
```bash
sg "query"                  # Search (hybrid: semantic + keyword)
sg "query" --no-hybrid      # Semantic-only search
sg "query" --auto-hybrid    # Auto-select: semantic for docstrings, hybrid for natural language
sg "query" --rerank         # LLM reranking for higher precision (requires ANTHROPIC_API_KEY)
sg "query" --json           # Output as JSON for scripting
sg "query" -n 20            # Return 20 results (default: 10)
sg "query" --path ./src     # Search within specific directory
sg "query" -t rs            # Filter to Rust files only
sg "query" -t rs -t py      # Filter to Rust and Python files
sg "query" -T test.rs       # Exclude test files from results
sg "query" -T spec.js -T test.ts  # Exclude multiple patterns
sg "query" -C 5             # Show 5 context lines (default: 2)
sg "query" --no-auto-index  # Skip auto-indexing unindexed projects
sg "query" --direct         # Bypass daemon, run search directly
sg search "status"          # Explicit search (when query matches a command name)
```

### Index Management
```bash
sg index                    # Index current directory
sg index ~/code/project     # Index specific directory
sg index --auto-model ./src # Auto-select model based on file mix
sg index --force            # Re-index all files (ignore content hashes)
sg status                   # Show index status and stats
sg files                    # List all indexed files
sg files ./src              # List indexed files under a directory
sg files --summary          # Show only totals (file/line/chunk counts)
sg files --json             # Output as JSON
sg chunks ./src/main.rs     # Show chunk details for a file
sg chunks ./src/main.rs -c  # Include content preview
sg chunks ./src/main.rs --json  # Output as JSON
```

### Daemon Control
```bash
sg daemon start             # Start background daemon
sg daemon stop              # Stop daemon
sg daemon status            # Show daemon status
sg daemon start --foreground  # Run in foreground (for debugging)
```

### Project Management
```bash
sg project list             # List known projects
sg project discover         # Scan common locations for projects
sg project detect           # Detect project root from current directory
```

### Shell Completions
```bash
sg completions bash         # Generate bash completions
sg completions zsh          # Generate zsh completions
sg completions fish         # Generate fish completions

# Install completions (bash)
sg completions bash > ~/.local/share/bash-completion/completions/sg

# Install completions (zsh)
sg completions zsh > ~/.zfunc/_sg

# Install completions (fish)
sg completions fish > ~/.config/fish/completions/sg.fish
```

### Shell Integration (Auto-indexing on cd)
```bash
# Enable auto-indexing when you cd into project directories

# Bash - add to ~/.bashrc
eval "$(sg init bash)"

# Zsh - add to ~/.zshrc
eval "$(sg init zsh)"

# Fish - add to ~/.config/fish/config.fish
sg init fish | source
```

When enabled, `sg` will automatically register projects with the daemon
as you navigate to them, enabling faster subsequent searches.

## How It Works

1. **Index files** - XTR model converts code to multi-vector embeddings (128-dim)
2. **Store in SQLite** - Embeddings stored alongside file content hashes
3. **Search** - Query embedded, matched against index via MaxSim scoring
4. **Hybrid fusion** - Optionally combine with keyword search using RRF

## Architecture

```
sg CLI ──────▶ sg-daemon (background) ──────▶ SQLite Index
                    │
                    ├── File watcher (incremental updates)
                    ├── Embedder (XTR model via Candle)
                    ├── Search engine (MaxSim + hybrid)
                    └── Project manager (auto-discovery)
```

## Output Formats

### Human-readable (default)
```
Found 3 results for "handle errors":

1. src/api/handler.rs:45 (score: 0.82)
    44 | impl ErrorHandler {
    45 |     pub fn handle(&self, err: Error) -> Response {
    46 |         match err.kind() {
```

### JSON (`--json`)
```json
{
  "query": "handle errors",
  "count": 3,
  "results": [
    {
      "path": "src/api/handler.rs",
      "score": 0.82,
      "line": 45,
      "snippet": "impl ErrorHandler {\n    pub fn handle..."
    }
  ]
}
```

## Configuration

Data is stored in platform-specific directories:
- **macOS**: `~/Library/Application Support/sg/`
- **Linux**: `~/.local/share/sg/`

Files:
- `index.db` - SQLite database with documents and embeddings (macOS: `~/Library/Application Support/sg/index.db`, Linux: `~/.local/share/sg/index.db`)
- `daemon.pid` - Daemon PID file (macOS: `~/Library/Application Support/sg/daemon.pid`, Linux: `~/.local/share/sg/daemon.pid`)
- `daemon.sock` - Unix socket for IPC (macOS: `~/Library/Application Support/sg/daemon.sock`, Linux: `~/.local/share/sg/daemon.sock`)

Config file (read by `sg-daemon` for a small set of settings):
- **macOS**: `~/Library/Application Support/sg/config.toml`
- **Linux**: `~/.config/sg/config.toml`
Supported keys:
- `[indexing] stale_project_days`
- `[indexing] idle_threshold_secs`
- `[daemon] socket`

## Based On

- **Retrieval engine:** [rust-warp](https://github.com/jhansen_dbx/rust-warp) - Rust port of XTR-WARP
- **Algorithm:** [XTR-WARP](https://github.com/jlscheerer/xtr-warp) (Stanford/Google)
- **Model:** XTR (Google DeepMind) - multi-vector embeddings with MaxSim scoring
- **ML runtime:** [Candle](https://github.com/huggingface/candle) - Rust ML framework

## Development

```bash
# Run all tests
cargo test

# Build release
cargo build --release

# Run with debug logging
RUST_LOG=debug sg "query"

# Run performance benchmark
sg benchmark .
```

## Performance

On macOS, `sg` uses Apple's Accelerate framework for 2x faster indexing (enabled by default).
Metal GPU acceleration is available with `--features metal` for faster embedding inference on Apple Silicon.

Typical performance on Apple Silicon:
- Model load: ~0.4s
- Index rate: ~4-5 files/s (~2000 lines/s)
- Search latency: <20ms

## Alternative Backends (Optional)

### ONNX Backend

For cross-platform deployment or alternative inference, `sg` supports ONNX Runtime:

```bash
# Build with ONNX support
cargo build --release --features onnx

# Export model to ONNX format (requires Python)
pip install torch transformers onnx onnxruntime
python scripts/export_onnx.py

# Use ONNX backend
SG_EMBEDDER_BACKEND=onnx sg "search query"
```

The ONNX model is stored at `~/.cache/sg/models/onnx/xtr_encoder.onnx` (~400MB).

### CoreML Backend (macOS only)

For Apple Silicon optimization using Apple's Neural Engine, `sg` supports CoreML via ONNX Runtime:

```bash
# Build with CoreML support (macOS only)
cargo build --release --features coreml

# Uses the same ONNX model as the ONNX backend
python scripts/export_onnx.py

# Use CoreML backend
SG_EMBEDDER_BACKEND=coreml sg "search query"
```

CoreML uses the CPU + Neural Engine for optimal performance on Apple Silicon devices.

### CUDA Backend (NVIDIA GPUs)

For GPU acceleration on Linux/Windows with NVIDIA GPUs:

```bash
# Build with CUDA support
cargo build --release --features cuda

# Use CUDA backend
SG_EMBEDDER_BACKEND=cuda sg "search query"
```

### OpenVINO Backend (Intel CPUs)

For optimized inference on Intel CPUs:

```bash
# Build with OpenVINO support
cargo build --release --features openvino

# Use OpenVINO backend
SG_EMBEDDER_BACKEND=openvino sg "search query"
```

### TensorRT Backend (NVIDIA GPUs)

For highly optimized inference on NVIDIA GPUs:

```bash
# Build with TensorRT support
cargo build --release --features tensorrt

# Use TensorRT backend
SG_EMBEDDER_BACKEND=tensorrt sg "search query"
```

### Document Processing

Index rich document formats (PDF, Office docs, EPUB):

```bash
# Build with document processing support
cargo build --release --features document-processing

# Index documents
sg index ~/documents/

# Supported formats: PDF, DOCX, XLSX, PPTX, ODS, EPUB
```

For scanned PDFs with OCR support:

```bash
# Build with OCR support (requires OCR models)
cargo build --release --features "document-processing,ocr"
```

### Multi-Model Embedding

`sg` can route content to specialized embedding models:

```bash
# List available models
sg model list

# Get model recommendation for a query
sg model recommend "getUserById"          # → UniXcoder (code)
sg model recommend "vampire Transylvania"  # → XTR (prose)

# Index with a specific model
sg index --model unixcoder ./src/

# Auto-select model based on file mix (code vs prose)
sg index --auto-model ./src/

# Force a model via environment variable
SG_EMBEDDER_MODEL=unixcoder sg "parse function"
```

Notes:
- Each index is built with a single embedding model. If you change models, re-index with `sg index --force`.
- Jina-Code and Jina-ColBERT require the ONNX feature (`cargo build --features onnx`) and download large model files.

**Model performance by content type (P@1):**

| Content Type | Best Model | P@1 | Notes |
|--------------|------------|-----|-------|
| English prose | XTR + hybrid | **1.00** | Default model |
| Source code | UniXcoder + hybrid | **0.93** | Auto-routed for code, +filename boost |
| Source code | Jina-Code + hybrid | **0.80** | Longer context (8192 tokens) |
| Japanese/CJK | Jina-ColBERT | **1.00** | Auto-routed for CJK |
| PDF documents | XTR + hybrid | **1.00** | |

Note: Hybrid search is on by default. CJK queries auto-route to Jina-ColBERT; code patterns auto-route to UniXcoder. The filename relevance boost surfaces implementation files over files that merely use the functionality.

### LLM Reranking (Optional)

Use Claude Haiku to rerank search results for higher precision on complex queries:

```bash
# Enable LLM reranking (requires ANTHROPIC_API_KEY environment variable)
export ANTHROPIC_API_KEY=sk-ant-...
sg "complex authentication flow" --rerank
```

How it works:
1. Fast semantic search retrieves top 50 candidates (~20ms)
2. Claude Haiku reranks by relevance to query (~100-200ms)
3. Returns reordered top-N results

LLM reranking improves P@1 by 10-15% on nuanced queries where semantic similarity alone is insufficient.

### Auto-Hybrid Mode

Auto-select between semantic-only and hybrid search based on query style:

```bash
sg "Returns the user count" --auto-hybrid    # → semantic (docstring-style)
sg "function that handles auth" --auto-hybrid # → hybrid (natural language)
```

Query style detection:
- **Docstring-style** (e.g., "Returns the...", "@param name") → semantic-only (0.9:0.1 weighting)
- **Natural language** (e.g., "where is error handling") → full hybrid (equal weighting)

Auto-hybrid improves CodeSearchNet benchmark results by +6% P@1 over fixed hybrid weights.

### Fine-Tuning (Optional)

Fine-tune XTR on your own codebase for improved code search:

```bash
# Step 1: Extract training data from Rust code
python scripts/extract_rust_training_data.py ~/my-project -o data/training.jsonl

# Step 2: Fine-tune XTR with LoRA (~30 min on M1 Mac)
pip install -r scripts/requirements.txt
python scripts/train_xtr_code.py --config config/train_rust_direct.yaml

# Step 3: Merge LoRA adapters
python scripts/merge_lora.py checkpoints/xtr-rust-direct -o checkpoints/xtr-merged

# Step 4: Use fine-tuned model
sg index --model-path checkpoints/xtr-merged ~/my-project
```

Fine-tuning improves code P@1 by ~20% on local codebases. See `docs/EMBEDDING_ROADMAP.md` for details.

## Documentation

- [ROADMAP.md](docs/ROADMAP.md) - Development phases and milestones
- [ARCHITECTURE.md](docs/ARCHITECTURE.md) - Technical design details
- [EMBEDDING_TRAINING.md](docs/EMBEDDING_TRAINING.md) - How we train code search embeddings
- [CLAUDE.md](CLAUDE.md) - AI worker instructions

## Status

**v0.2.0** - Feature complete ✓

| Component | Status |
|-----------|--------|
| Core semantic search (MaxSim) | ✓ |
| Chunk-level embedding (documents split into 350-word chunks) | ✓ |
| Background daemon with IPC | ✓ |
| File watching & incremental updates | ✓ |
| Smart project detection | ✓ |
| Auto-indexing on cd (shell integration) | ✓ |
| Hybrid search (semantic + keyword) | ✓ |
| JSON output | ✓ |
| Colored terminal output | ✓ |
| Shell completions (bash/zsh/fish) | ✓ |
| Optional ONNX Runtime backend | ✓ |
| Optional CoreML backend (macOS) | ✓ |
| Optional CUDA backend (NVIDIA GPUs) | ✓ |
| Optional OpenVINO backend (Intel CPUs) | ✓ |
| Optional TensorRT backend (NVIDIA GPUs) | ✓ |
| Document processing (PDF, DOCX, XLSX, PPTX, EPUB) | ✓ |
| Multi-model embedding (XTR, UniXcoder, Jina-ColBERT, Jina-Code) | ✓ |
| Local model fine-tuning (LoRA) | ✓ |
| Index health monitoring & auto-rebalancing | ✓ |
| LLM reranking with Claude Haiku | ✓ |
| Adaptive query-style detection (auto-hybrid) | ✓ |

**Stats:** 40,000+ lines of Rust, 675 tests passing

## License

MIT
