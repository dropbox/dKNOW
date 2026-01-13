# PDF ML Table/OCR Issue List (Source Review)

Fresh issues observed directly from the current source.

1) `crates/docling-pdf-ml/src/pipeline/table_inference.rs:276` — `num_rows` counts only `nl` tokens and never adds 1, so single-row tables produce `num_rows=0` and downstream grids are empty.  
2) `table_inference.rs:277-285` — `num_cols` is derived from the first `nl` position; later rows are ignored, so multi-row tables undercount columns and misplace cells.  
3) `table_inference.rs:243` — `page_no` is hardcoded to 0 when building `TableElement`, breaking provenance for any page beyond the first.  
4) `table_inference.rs:247-259` — Cell bboxes are scaled to the unscaled table bbox while inference ran on a 2.0x crop; `_scale` is unused, so bboxes are off by the TABLE_SCALE factor.  
5) `table_inference.rs:230-239` — Row/col spans are always 1; TableFormer span semantics are discarded, so merged cells are lost.  
6) `table_inference.rs:214-223` — Header/row-section flags are parsed but dropped; header/section metadata never reaches outputs.  
7) `table_inference.rs:189-205` — OCR text matching concatenates any overlapping OCR cell with no ordering/scoring, yielding nondeterministic or mixed cell text.  
8) `table_inference.rs:137-162` — Preprocess swaps W/H and divides by 255 after a 255*mean/std normalization; likely deviates from Python and skews inputs.  
9) `table_inference.rs:115-132` — Crop uses scaled bbox without guarding against inverted/negative ranges; malformed bboxes can panic on slice.  
10) `table_inference.rs:93` — `TABLE_SCALE` fixed at 2.0; if renderer DPI differs, crops fed to TableFormer are mis-scaled.  
11) `table_inference.rs:206-223` — Cells are assigned purely by tag order with no bounds check against `num_rows/num_cols`; overlong sequences overflow grids silently.  
12) `table_inference.rs:288-293` — `table_map` keys only by cluster id; cross-page id collisions overwrite tables.  
13) `table_inference.rs:333-336` — `otsl_seq` filters tags and drops start/end, so its length can diverge from cells/coords, complicating validation.  
14) `crates/docling-pdf-ml/src/convert.rs:239-262` — `unwrap_or(0)` defaults missing row/col offsets to top-left, masking indexing bugs and misplacing cells.  
15) `convert.rs:247-262` — Spanned cells are replicated into every covered slot (with spans still set), duplicating text instead of representing merged spans.  
16) `convert.rs:320-353` — Container elements become empty Text DocItems, discarding nested structure and labels (lists/sections lose hierarchy).  
17) `convert.rs:354-396` — `cluster_to_doc_item` maps table clusters to empty grids (`num_rows=0,num_cols=0`), so cluster-based paths drop table content.  
18) `crates/docling-pdf-ml/build.rs:9` — Rpath hardcoded to a specific Python torch lib; ignores `$LIBTORCH` and breaks on other installs, contributing to libtorch load crashes.  
19) `setup_env.sh` — DYLD paths pinned to Python 3.14 torch; no preference for `$LIBTORCH/lib` or active venv, increasing likelihood of wrong libtorch being loaded.  
20) `README.md` + feature flags — `pdf-ml-onnx` ships without any table backend; ONNX builds can’t emit tables yet the pipeline doesn’t warn/fail when `table_structure_enabled=true`.
