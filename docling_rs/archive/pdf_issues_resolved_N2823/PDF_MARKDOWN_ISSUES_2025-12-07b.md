# PDF Parsing & Markdown Issues (Set 6) — 2025-12-07 Audit

1) `crates/docling-pdf-ml/src/convert_to_core.rs`: Table cells are not ordered/sorted before grid placement; duplicate or unsorted cells can overwrite each other nondeterministically.  
2) `convert_to_core.rs`: No deduplication of captions; duplicated caption refs render repeated captions without warning.  
3) `convert_to_core.rs`: Table/picture DocItems are created with `content_layer` directly from ML; mislabeled layers (e.g., not "body") are later filtered out, dropping content with no override.  
4) `convert_to_core.rs`: DocItems are emitted without checking for duplicate `self_ref` collisions across pages; references can clash and make serializer skip items.  
5) `convert_to_core.rs`: TableCell span indices are not clamped to `num_rows/num_cols` before conversion; invalid spans can corrupt grids downstream.  
6) `convert_to_core.rs`: TableElement `text`/summary is always `None`; consumers relying on a textual fallback for tables receive empty content.  
7) `crates/docling-pdf-ml/src/pipeline/table_inference.rs`: No tolerance margin in OCR overlap test; cells that only touch edges pull in adjacent text, causing text bleeding across columns.  
8) `table_inference.rs`: No minimum bbox area/size filter; tiny noise detections become cells, inflating table size with junk.  
9) `table_inference.rs`: Coordinates/OTSL length mismatch is not validated; excess coords or tags are silently ignored, hiding model/schema drift.  
10) `table_inference.rs`: Cell bboxes aren’t clamped to page bounds post-scaling; spans can extend outside the page with no warning.  
11) `crates/docling-pdf-ml/src/pipeline/executor.rs`: When ONNX and PyTorch table models coexist, ONNX is chosen without version check or user override; may load stale ONNX while newer PyTorch exists.  
12) `executor.rs`: If OCR is disabled and no textline cells are provided, table inference still runs and returns empty text instead of auto-enabling OCR or emitting an error.  
13) `executor.rs`: Does not expose per-page assembled data or DoclingDocument to callers; downstream cannot inspect page-level PDF ML output for debugging/regression.  
14) `crates/docling-core/src/serializer/markdown.rs`: No page-break markers; multi-page PDF output is flattened, losing page context compared to Python exports.  
15) `markdown.rs`: Table header determination is hardcoded to first row; tables with explicit header flags (column_header) but non-header-first rows are misrendered.  
16) `markdown.rs`: Empty-header tables still emit separator lines, producing malformed markdown with zero-length headers.  
17) `markdown.rs`: Inline math/code inside tables is not escaped/isolated; adjacent content can merge when cells are collapsed to single lines.  
18) `markdown.rs`: No control-character sanitization; OCR control chars can leak into markdown and break rendering.  
19) `markdown.rs`: Figures emit only a placeholder or data URI; no alt text is derived from captions/titles, hurting accessibility and fidelity.  
20) `crates/docling-backend/src/pdf.rs`: `content_blocks` are built only from `core_docling_doc.texts` and not sorted by reading order; even if tables/figures were included, ordering would not match the assembled document.
