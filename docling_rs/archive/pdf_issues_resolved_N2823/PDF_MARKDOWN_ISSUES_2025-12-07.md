# PDF Parsing & Markdown Issues (Set 5) — 2025-12-07 Audit

1) `crates/docling-pdf-ml/src/convert_to_core.rs`: Table cells are inserted in input order with no stable sort by (row,col); overlapping placements overwrite each other nondeterministically.  
2) `convert_to_core.rs`: Span replication can overwrite earlier cells without any warning; conflicting spans are not detected.  
3) `convert_to_core.rs`: `content_layer` from PDF-ML is lowercased verbatim; non-`body` layers (e.g., mislabeled by ML) are later filtered out with no mapping or override.  
4) `convert_to_core.rs`: Footnotes/references/annotations/image fields on tables/pictures default to empty/None; any upstream metadata is dropped.  
5) `convert_to_core.rs`: Caption refs are copied blindly; duplicate refs produce duplicated captions with no deduplication.  
6) `crates/docling-pdf-ml/src/pipeline/table_inference.rs`: Assumes `coordinates.len()` matches number of cell tags; mismatch truncates silently and is not reported.  
7) `table_inference.rs`: `bboxes_overlap` treats touching edges as overlap; adjacent OCR boxes can bleed text into neighboring cells.  
8) `table_inference.rs`: No minimum cell-size filter; tiny noisy detections become cells, creating spurious tables.  
9) `table_inference.rs`: No validation against NaN/inf in TableFormer outputs; invalid coords propagate and can poison assembly/markdown.  
10) `table_inference.rs`: Ignores `class_logits` confidence entirely; low-confidence detections are accepted unfiltered.  
11) `crates/docling-pdf-ml/src/pipeline/executor.rs`: When ONNX fails and PyTorch is unavailable, tablestructure is set to None and assembly proceeds with no warning to callers.  
12) `executor.rs`: `table_model_dir` selection picks the first snapshot directory arbitrarily; may load outdated weights instead of latest.  
13) `executor.rs`: If OCR is disabled and textline cells are empty, table inference still runs and returns empty text instead of falling back to enabling OCR.  
14) `crates/docling-core/src/serializer/markdown.rs`: Column width uses `chars().count()` instead of display width; CJK/double-width characters misalign tables.  
15) `markdown.rs`: Tables with empty header row still emit a separator row, producing malformed markdown with zero-length headers.  
16) `markdown.rs`: Row/column header semantics are not rendered; row headers appear as ordinary cells, losing structure for accessibility/consumers.  
17) `markdown.rs`: No blank line emitted after tables; back-to-back tables/paragraphs can run together versus Python output.  
18) `markdown.rs`: Visited-set suppression also removes repeated caption/figure references; later references lose their associated content entirely.  
19) `crates/docling-backend/src/pdf.rs`: Furniture-layer content (page headers/footers) is always dropped with no config to include; PDFs relying on them lose titles/page numbers.  
20) `pdf.rs`: Metadata such as `num_pages` isn’t adjusted when `max_pages` truncation is used; markdown represents a shortened doc while metadata still reflects full length.
