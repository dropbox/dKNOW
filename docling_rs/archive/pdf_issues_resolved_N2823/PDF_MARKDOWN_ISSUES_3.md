# PDF Parsing & Markdown Issues (Set 3) — Current Code Audit

1) `crates/docling-pdf-ml/src/convert_to_core.rs`: TableCell metadata (column_header/row_header/from_ocr/confidence) is dropped; table semantics and OCR confidence are lost.  
2) `convert_to_core.rs`: Cell-level provenance is not preserved; only table-level prov is kept, so markdown/debug cannot trace which OCR bbox produced a cell.  
3) `convert_to_core.rs`: OTSL sequence from TableFormer is discarded; downstream loses access to raw structure tokens for debugging/regression checks.  
4) `convert_to_core.rs`: `content_layer` is derived from `fmt::Debug` + lowercase; renaming/casing enum variants silently breaks layer filtering.  
5) `convert_to_core.rs`: No validation that `num_rows/num_cols` match max row/col indices in `table_cells`; inconsistent inputs are accepted and later misrender.  
6) `crates/docling-pdf-ml/src/pipeline/table_inference.rs`: Bbox remapping back to page ignores `table_scale`; with `table_scale != 1.0`, cell boxes are inflated vs original page.  
7) `table_inference.rs`: ONNX path ignores `table_scale` entirely; ONNX/PyTorch produce incompatible bboxes when scale ≠ 1.  
8) `table_inference.rs`: OCR text matching doesn’t warn when OCR is absent/disabled, so all table cells become empty silently.  
9) `table_inference.rs`: Span computation uses OTSL grid length without bounds checks against `num_rows/num_cols`; malformed sequences can panic or truncate spans.  
10) `table_inference.rs`: Header flags (column_header/row_header) computed but not stored in core TableCell; markdown cannot render headers distinctly.  
11) `crates/docling-pdf-ml/src/pipeline/executor.rs`: When both ONNX and PyTorch models exist, ONNX is chosen unconditionally; no user control or version/compat check to prefer PyTorch.  
12) `executor.rs`: With `skip_validation`, `table_structure_enabled=true` and missing models yields pages with no table content and no warning.  
13) `executor.rs`: Reading order vector excludes table/figure elements; tables appear out of order in markdown export.  
14) `crates/docling-core/src/serializer/markdown.rs`: Tables drop footnotes/references fields entirely; PDF footnote content is lost.  
15) `markdown.rs`: Multi-header-row tables unsupported; only the first row treated as header, flattening multi-level headers.  
16) `markdown.rs`: Rich table cells with hyperlinks/formatting serialize to plain text; links/styling lost inside tables.  
17) `markdown.rs`: Ragged rows aren’t padded or warned; rows shorter than `num_cols` are truncated, losing data with uneven rows or spans.  
18) `markdown.rs`: Math/markdown characters in table cells aren’t escaped; pipes/backticks/dollar signs from OCR can break table formatting or math fences.  
19) `crates/docling-backend/src/pdf.rs`: `content_blocks` populated only from `core_docling_doc.texts`; tables/figures never reach callers even when parsed.  
20) `pdf.rs`: Backend never returns the constructed `DoclingDocument`; callers cannot inspect full PDF structure/tables to debug discrepancies.
