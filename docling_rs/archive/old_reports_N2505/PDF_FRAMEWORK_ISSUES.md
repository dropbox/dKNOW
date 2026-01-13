# PDF Parsing / Markdown / ML Framework Issues (Batch)

Notes collected from current source and docs; focus on PDF pipeline, markdown serialization, and ML wiring. Items are concise for triage.

1) `crates/docling-pdf-ml/src/pipeline/executor.rs`: No fast-fail when TableFormer is requested but model directory is missing; pipeline silently skips tables.  
2) `executor.rs`: `table_structure_enabled` default depends on config defaults; there is no explicit CLI/config validation to ensure tables run when PDFs contain tables.  
3) `executor.rs`: No guard that RapidOCR models exist before enabling OCR; errors surface only at inference time.  
4) `executor.rs`: Batch processing uses layout batch only; OCR/table paths aren’t batched, losing throughput and consistency.  
5) `executor.rs`: Reading order always uses default config; no exposure of tuning params (beam, thresholds) from Python parity.  
6) `executor.rs`: Profiling toggles don’t capture OCR/table timings when those stages are skipped due to missing models.  
7) `executor.rs`: When OCR is disabled but `textline_cells` are empty, modular pipeline is skipped, leading to layout-only outputs without warning.  
8) `executor.rs`: `Device::Cuda` path only checks `tch::Cuda::is_available`; does not verify correct libtorch build (CPU vs CUDA) before running.  
9) `executor.rs`: Error messages for model load don’t include searched paths, slowing triage for missing weights.  
10) `executor.rs`: `table_former` and `code_formula` options are compiled out without `pytorch` feature but no runtime warning when user requests them.  
11) `executor.rs`: `process_pages_batch` discards OCR for batch inputs; OCR must be re-run per page after batch layout, causing redundant work.  
12) `executor.rs`: No backoff to ONNX layout when PyTorch backend fails; a single backend failure aborts entire page.  
13) `executor.rs`: `page_no` propagation relies on caller; internal creation of `TableElement` still sets `page_no` to 0 (see table_inference).  
14) `executor.rs`: `table_structure_enabled=false` is the default in some presets; docs imply tables enabled by default—config mismatch.  
15) `executor.rs`: `ocr_enabled` default combined with missing RapidOCR assets yields runtime error instead of an early configuration error.  
16) `executor.rs`: No MPS (Apple GPU) handling; `Device::Cuda` or CPU only, despite PyTorch MPS availability.  
17) `executor.rs`: `convert_textline_coords` always runs even if OCR skipped; this can panic on None in future refactors.  
18) `executor.rs`: `process_page` accepts `textline_cells` but clones them for table; expensive for large OCR outputs (no borrowing).  
19) `executor.rs`: No validation that `page_width/page_height` match image dimensions; scaling errors propagate unnoticed to bbox math.  
20) `executor.rs`: `modular_pipeline` created per pipeline instance; no option to reuse across pipelines for perf-sensitive use.  
21) `executor.rs`: `profiling_enabled` flag must be set manually; CLI doesn’t expose profiling flags.  
22) `executor.rs`: `code_formula_enabled` path runs after assembly; if assembly fails, code/formula enrichment never reports why.  
23) `executor.rs`: Logging is debug-level; no structured event output for stage timings to feed benchmarks.  
24) `crates/docling-pdf-ml/src/pipeline/table_inference.rs`: `TABLE_SCALE` constant cannot be configured per document; PDFs with different render DPI will mis-scale tables.  
25) `table_inference.rs`: No check for zero-sized crop regions; empty crops produce runtime errors in preprocess.  
26) `table_inference.rs`: Preprocess uses bilinear upsample without antialias; high-frequency patterns may be distorted vs Python OpenCV defaults.  
27) `table_inference.rs`: `find_matching_ocr_text` uses any overlap; lacks IoU threshold and ordering, diverging from Python cell_matcher.  
28) `table_inference.rs`: No fallback text for empty OCR matches; cells become empty even when TableFormer predicted structure.  
29) `table_inference.rs`: `table_map` keyed by cluster id only; multi-page batches with reused ids overwrite table data.  
30) `table_inference.rs`: Tag parsing drops `<start>/<end>` but doesn’t validate minimum sequence; malformed outputs go unchecked.  
31) `table_inference.rs`: No confidence scores for cells; downstream cannot filter low-quality detections.  
32) `table_inference.rs`: Header/row_section flags not emitted; markdown/table serializers can’t style headers differently.  
33) `table_inference.rs`: `num_rows` derived from count of `nl`; if last row lacks trailing `nl`, row count is off by -1.  
34) `table_inference.rs`: `num_cols` derived from position of first `nl`; tables with variable-width rows break.  
35) `table_inference.rs`: No detection of degenerate tables (0 rows/cols) to short-circuit markdown serialization with a warning.  
36) `table_inference.rs`: Coordinates converted to page space ignore image scaling factor; mismatched to layout coordinates if preprocessing resized the page.  
37) `crates/docling-pdf-ml/src/convert.rs`: No validation that `num_rows/num_cols` align with `table_cells`; inconsistent grids are silently generated.  
38) `convert.rs`: `unwrap_or(0)` for row/col indices masks missing indices and misplaces cells.  
39) `convert.rs`: Spanned cells are duplicated into every covered slot; merged-cell semantics lost and text duplicated in markdown.  
40) `convert.rs`: Container elements downgraded to empty Text; document hierarchy (lists/sections) lost in DocItems.  
41) `convert.rs`: `cluster_to_doc_item` path for tables emits empty grids; any caller using clusters loses table content entirely.  
42) `convert.rs`: Simple markdown export prints only table dimensions; no actual cell content in test/export paths.  
43) `convert.rs`: No escaping of markdown special characters in text export; output can break formatting.  
44) `convert.rs`: `PageElement::Container` uses hardcoded `self_ref` and blank provenance, breaking traceability.  
45) `convert.rs`: `DocItem::SectionHeader` level is hardcoded to 1; multi-level headers from layout are lost.  
46) `convert.rs`: No image/caption propagation for tables/figures; captions remain empty even if present in upstream data structures.  
47) `convert.rs`: Footnotes/references fields always empty; upstream data discarded.  
48) `crates/docling-core/src/serializer/markdown.rs`: Early return if `grid.is_empty()` drops tables silently; no warning emitted.  
49) `markdown.rs`: No handling of row/col spans; merged cells not reflected in markdown output.  
50) `markdown.rs`: Does not render table headers differently; header metadata lost.  
51) `markdown.rs`: RTL content not handled; markdown order incorrect for RTL PDFs.  
52) `markdown.rs`: No configurable column width/justification; wide content may wrap unpredictably compared to Python output.  
53) `markdown.rs`: Table text not escaped; pipes and backticks can break table formatting.  
54) `markdown.rs`: Lacks fallback for very large tables (performance/timeouts) compared to Python chunking.  
55) `crates/docling-pdf-ml/src/models/layout_predictor`: ONNX and PyTorch variants diverge in output types; executor doesn’t normalize labels/confidence thresholds between them.  
56) `layout_predictor`: Thresholds for layout detection are hardcoded; no config exposure for tuning recall/precision.  
57) `layout_predictor`: Batch inference path not used in single-page pipeline; performance penalty.  
58) `crates/docling-pdf-ml/src/models/table_structure`: No ONNX backend; ONNX feature cannot produce tables.  
59) `table_structure`: Weight loading lacks checksum validation; corrupted downloads go unnoticed.  
60) `table_structure`: Model caching relies on HF cache; no explicit path configuration for offline use.  
61) `crates/docling-pdf-ml/src/models/ocr`: RapidOCR uses local assets; no asset presence check at startup.  
62) `ocr`: Angle classification/recognition models are not version-checked; mixing versions may degrade accuracy silently.  
63) `ocr`: No language selection/config; defaults may underperform on non-Latin scripts.  
64) `crates/docling-pdf-ml/src/pipeline/mod.rs`: Device enum stub for non-pytorch builds diverges from tch::Device; potential mismatch when toggling features.  
65) `pipeline/mod.rs`: Default config builder doesn’t enforce table/OCR presets matching Python defaults.  
66) `pipeline/mod.rs`: Missing validation of mutually dependent features (e.g., table requires pytorch).  
67) `crates/docling-pdf-ml/src/pipeline/layout_postprocessor.rs`: Expects non-empty OCR cells; empty cells skip stage silently, leaving clusters unordered.  
68) `layout_postprocessor.rs`: No logging when clusters are dropped by modular pipeline; hard to debug missing content.  
69) `layout_postprocessor.rs`: Coordinate normalization assumes page/image alignment; no assertion of image size vs page size.  
70) `crates/docling-pdf-ml/src/pipeline/page_assembly.rs`: No verification that reading order was applied; assembled elements may be unsorted.  
71) `page_assembly.rs`: Does not merge text spans across lines; may fragment paragraphs compared to Python behavior.  
72) `page_assembly.rs`: Table elements lack linkage back to OCR cells; text provenance is lost.  
73) `crates/docling-pdf-ml/src/pipeline/docling_export.rs`: Minimal validation when constructing DoclingDocument; missing fields not reported.  
74) `docling_export.rs`: Does not inject table headers/section info into DoclingDocument tables.  
75) `crates/docling-pdf-ml/src/preprocessing`: Page image normalization assumes 0-255; doesn’t validate input dtype/range.  
76) `preprocessing`: No handling of grayscale or 4-channel images; assumes 3-channel RGB.  
77) `preprocessing`: Rotation/orientation correction not integrated; rotated scans may fail layout/table detection.  
78) `preprocessing`: Lacks DPI estimation; TABLE_SCALE assumes fixed DPI.  
79) `crates/docling-backend/src/pdf.rs`: pdfium rendering path not verified; missing libpdfium silently disables PDF ML end-to-end.  
80) `pdf.rs`: No option to select ONNX vs PyTorch backend per run; relies on compile-time features only.  
81) `pdf.rs`: Does not surface ML errors (e.g., missing models) up to the CLI with actionable messages.  
82) `pdf.rs`: No streaming/page-chunking for large PDFs; memory usage may spike.  
83) `pdf.rs`: DocItems from PDF ML not round-tripped through canonical tests; regression risk.  
84) `crates/docling-cli`: CLI help still marks PyTorch backend default; conflicts with docs recommending ONNX for stability.  
85) `CLI`: No command to dump intermediate ML stages (layout clusters, table crops) for debugging.  
86) `CLI`: No flag to force-table-disable while keeping layout/OCR for comparison testing.  
87) `crates/docling-pdf-ml/tests`: Many critical tests are `#[ignore]` (TableFormer phases); they do not run in CI, hiding regressions.  
88) `tests`: OCR integration relies on `models/rapidocr` presence but doesn’t assert it; tests may vacuously pass when assets are missing.  
89) `tests`: No golden-output tests for markdown serialization of tables; grid mistakes go undetected.  
90) `tests`: No multi-page PDF ML integration test that spans OCR + layout + table + markdown end-to-end.  
91) `tests`: No RTL PDF fixtures in Rust ML tests; RTL regressions untested.  
92) `tests`: No performance smoke test to catch extreme slowness in TableFormer/ocr.  
93) `tests`: No negative tests for missing models (expect clean error), causing user-hostile panics.  
94) `tests`: No coverage for grayscale/4-channel page images.  
95) `setup_env.sh`: Hardcodes torch lib path; fails on systems without that Python install, causing user confusion.  
96) `setup_env.sh`: Sets `LIBTORCH_BYPASS_VERSION_CHECK` unconditionally; may mask ABI mismatches.  
97) `crates/docling-pdf-ml/build.rs`: Hardcoded rpath order can force wrong libtorch even when `$LIBTORCH` is set.  
98) `crates/docling-pdf-ml/README.md`: States models auto-download; in offline environments this fails silently with poor guidance.  
99) `README.md` (root): ONNX backend documented as lacking tables; once fixed, docs must be updated to avoid misguiding users.  
100) `docs`/status files: Several status docs still claim PDF is “experimental/out of scope”, conflicting with directives requiring 0% diff—future workers may stop short of full parity.
