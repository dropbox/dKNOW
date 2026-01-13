# Completed MANAGER Directions Archive

This file archives completed MANAGER directions from CLAUDE.md to keep the main file concise.

---

## EMBEDDING IMPROVEMENTS (2025-12-31) - COMPLETE

**Status:** All phases complete. See `docs/EMBEDDING_ROADMAP.md` for detailed results.

### Phase 1: Jina-ColBERT-v2 for Multilingual (#537-#538)
- Model: `jinaai/jina-colbert-v2`
- 94 languages including Japanese, Chinese, Korean
- **Result: P@1 = 1.00 on Japanese corpus (semantic-only, no hybrid needed)**

### Phase 2a: Jina-Code-v2 (#539)
- Model: `jinaai/jina-embeddings-v2-base-code`
- 30+ programming languages, 8192 token context
- **Result: P@1 = 0.80 with hybrid (matches UniXcoder), P@1 = 0.70 semantic-only**

### Phase 2b: XTR LoRA Fine-Tuning (#540-#546)
- LoRA fine-tuned XTR on local Rust code (596 training pairs)
- Training: ~30 min on M1 Mac with MPS
- **Result: +20% P@1 improvement (0.50→0.60 semantic-only), +17% hybrid (0.60→0.70)**

### Phase 3: Local Corpus Tuning Infrastructure
- Training script: `scripts/train_xtr_code.py`
- Data extraction: `scripts/extract_rust_training_data.py`
- Merge script: `scripts/merge_lora.py`
- CLI support: `--model-path` option

### Files Added
- `crates/sg-core/src/embedder_jina_colbert.rs`
- `crates/sg-core/src/embedder_jina_code.rs`
- `scripts/train_xtr_code.py`
- `scripts/extract_rust_training_data.py`
- `scripts/merge_lora.py`
- `config/train_rust_direct.yaml`

---

## FIX CODE SEARCH (2025-12-30) - RESOLVED

**Problem:** Code search P@1 was only 0.50 with XTR model.

**Solution:** Use UniXcoder + hybrid search for code.

| Configuration | P@1 | MRR |
|---------------|-----|-----|
| XTR semantic-only | 0.50 | 0.69 |
| XTR hybrid | 0.60 | 0.80 |
| UniXcoder semantic | 0.60 | 0.74 |
| **UniXcoder hybrid** | **0.80** | **0.88** |

*Note: Original results (0.90/0.93) were inflated due to non-deterministic HashMap ordering in RRF. Fixed in #532 with secondary sort by doc_id.*

**Changes Made:**
1. Added `--hybrid` flag to `sg eval` command
2. Tested all combinations of model + search mode
3. Documented best configuration: `--model unixcoder --hybrid`
4. Fixed determinism in RRF ordering (#532)

**Optional improvements completed:**
1. Auto-detect content type - `--auto-model` flag (#515)
2. Hybrid search default - ON by default (`--no-hybrid` to disable)
3. "Unix socket IPC" query - Returns server.rs at rank 1 (#516)

---

## FIX BROKEN SEARCH (2025-12-29) - RESOLVED

**Issue:** Suspected broken search was actually stale/missing index data.

**Resolution:** Gutenberg corpus was properly indexed with 536 chunks for Dracula.
Re-running evaluation confirmed correct results (9/10 at rank 1).

**Completed Work (iterations #361-#418):**
- Critical content truncation bug fixed (v0.1.0 → v0.2.0)
- ONNX/CoreML/CUDA backends added
- Storage limits implemented
- Search result explanations added
- Product quantization implemented (32x compression)
- HNSW navigation added (O(log n) cluster selection)
- Hierarchy-aware markdown chunking integrated

---

## Integrate Production Chunker (2025-12-29) - COMPLETE

**Status:** DONE in iteration #393

**Completed tasks:**
1. markdown_chunker crate copied to workspace
2. chunker.rs updated to use markdown_chunker
3. Chunk struct updated with header_context field
4. Storage schema updated with header_context column
5. Search result display updated to show header context
6. All tests pass

**Source:** `git@github.com:dropbox/chunker.git`

**Features:**
- Hierarchy-aware chunking (preserves markdown/code structure)
- Header context (`header_hierarchy` for each chunk)
- Never splits code blocks or tables
- Multilingual (CJK support)
- Semantic overlap at sentence boundaries

---

## Multi-Embedding Evaluation (2025-12-29) - COMPLETE

**Status:** Results documented in `eval/MULTI_EMBEDDING_RESULTS.md`

**Findings:**
- Code embeddings (UniXcoder) beat general (XTR) on code by 40%+
- XTR performs well on English prose
- Specialized embeddings outperform on target domains

**Implemented:**
- `multi_embedder.rs` with `EmbedderType` enum
- Content-type detection (`detect_content_type`)
- Model selection via `--model` flag

**Available Corpora (LOCAL):**
- English Prose: `/Users/ayates/eval/gutenberg/`
- Japanese Financial: `/Users/ayates/video_audio_extracts/test_pdf_corpus/all/edinet*.pdf`
- Code: `/Users/ayates/sg/crates/`
- Research Papers: `/Users/ayates/video_audio_extracts/test_pdf_corpus/all/`
- PDF Test Corpus: `/Users/ayates/pdfium_fast/` (567 PDFs)

---

## Indexing Performance Optimizations (2025-12-29) - COMPLETE

**Status:** All P0-P3 items done. See `INDEXING_ROADMAP.md` checklist.

**Implemented (#398-#410, #443):**
1. Batch embedding within file (3-5x faster per-file indexing)
2. Batch DB inserts (5-10x faster DB writes)
3. Model pre-warming at startup

---

## Integrate docling_rs for Document Processing (2025-12-29) - COMPLETE

**Status:** Implemented in iterations #425-#448. See `docs/ROADMAP.md` Phase 10A.

**Source:** `git@github.com:dropbox/docling_rs.git`

**Features:**
- Layout-aware text extraction (preserves headers, paragraphs, lists)
- Table extraction
- OCR fallback for scanned documents
- Support for 60+ document formats (not just PDF)

**Architecture:**
```
sg indexing
    ├── Code files (.rs, .py, etc.)
    │   └── Read directly → chunk → embed
    └── Documents (.pdf, .docx, .xlsx, etc.)
        └── docling_rs → markdown → chunk → embed
```
