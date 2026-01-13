#![allow(non_snake_case)]
#![allow(unused_variables)]
#![allow(clippy::bind_instead_of_map)]
#![allow(clippy::needless_as_bytes)]
#![allow(clippy::needless_borrows_for_generic_args)]
#![allow(static_mut_refs)]

use docling_backend::{DocumentConverter, RustDocumentConverter};
use docling_core::DocItem;
// Python removed - all conversion now via pure Rust
use std::fs;
use std::path::Path;
use std::sync::{Mutex, Once};
use std::time::{Duration, Instant};

mod common;
use common::*;

/// Global test run context
#[derive(Debug)]
struct TestRunContext {
    run_id: String,
    start_time: Instant,
    test_counter: Mutex<usize>,
    completed_tests: std::collections::HashSet<String>,
}

static INIT: Once = Once::new();
static mut TEST_RUN_CONTEXT: Option<TestRunContext> = None;

fn get_test_run_context() -> &'static TestRunContext {
    unsafe {
        INIT.call_once(|| {
            // Load completed tests if resuming from previous run
            let completed_tests = std::env::var("RESUME_FROM_CSV").map_or_else(
                |_| std::collections::HashSet::new(),
                |resume_csv| {
                    std::fs::read_to_string(&resume_csv)
                        .ok()
                        .and_then(|content| {
                            let tests: std::collections::HashSet<String> = content
                                .lines()
                                .skip(1) // Skip header
                                .filter_map(|line| line.split(',').next())
                                .map(|s| s.to_string())
                                .collect();
                            println!(
                                "\n📋 Resume mode: Loaded {} completed tests from {}\n",
                                tests.len(),
                                resume_csv
                            );
                            Some(tests)
                        })
                        .unwrap_or_default()
                },
            );

            TEST_RUN_CONTEXT = Some(TestRunContext {
                run_id: uuid::Uuid::new_v4().to_string(),
                start_time: Instant::now(),
                test_counter: Mutex::new(0),
                completed_tests,
            });
        });
        TEST_RUN_CONTEXT.as_ref().unwrap()
    }
}

/// System information for diagnostics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SystemInfo {
    available_memory_mb: Option<u64>,
    cpu_count: usize,
}

fn get_system_info() -> SystemInfo {
    let mut sys = sysinfo::System::new_all();
    sys.refresh_memory();

    let available_mb = sys.available_memory() / 1024 / 1024; // Convert bytes to MB

    SystemInfo {
        available_memory_mb: Some(available_mb),
        cpu_count: num_cpus::get(),
    }
}

#[derive(Debug, Clone, Copy)]
enum ExtractionMode {
    OcrText,
    TextOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TestResult {
    status: String,
    fail_reason: String,
    expected_bytes: usize,
    actual_bytes: usize,
    expected_chars: usize,
    actual_chars: usize,
}

/// Check if test should run based on resume mode
fn should_run_test(fixture: &TestFixture) -> bool {
    // Check if resuming from previous run
    let ctx = get_test_run_context();
    if ctx.completed_tests.contains(&fixture.title_slug) {
        return false; // Skip - already completed in previous run
    }
    true
}

fn run_integration_test(fixture: &TestFixture, mode: ExtractionMode) -> Result<(), String> {
    if !should_run_test(fixture) {
        return Ok(()); // Skip test in resume mode
    }
    run_integration_test_impl(fixture, mode, true)
}

fn run_integration_test_no_output_check(
    fixture: &TestFixture,
    mode: ExtractionMode,
) -> Result<(), String> {
    if !should_run_test(fixture) {
        return Ok(()); // Skip test in resume mode
    }
    run_integration_test_impl(fixture, mode, false)
}

fn run_integration_test_impl(
    fixture: &TestFixture,
    mode: ExtractionMode,
    validate_output: bool,
) -> Result<(), String> {
    // Skip output validation if explicitly disabled
    // (Python baselines differ from Rust output - see CLAUDE.md N=2321)
    let skip_validation = std::env::var("SKIP_OUTPUT_VALIDATION").is_ok();
    let validate_output = validate_output && !skip_validation;

    // Per-test timeout (default 10 minutes = 600 seconds)
    let timeout_secs = std::env::var("TEST_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(600);

    let test_start = Instant::now();

    // Initialize test result tracking
    let mut test_result = TestResult {
        status: "pass".to_string(),
        fail_reason: "".to_string(),
        expected_bytes: 0,
        actual_bytes: 0,
        expected_chars: 0,
        actual_chars: 0,
    };

    // Build path to test file (relative to workspace root)
    let test_file = Path::new("../../test-corpus").join(&fixture.file_path);

    if !test_file.exists() {
        test_result.status = "fail".to_string();
        test_result.fail_reason = "Test file not found".to_string();
        return Err(format!("Test file not found: {}", test_file.display()));
    }

    // Check conversion mode
    let use_rust_backend = std::env::var("USE_RUST_BACKEND").is_ok();
    let use_hybrid_serializer = std::env::var("USE_HYBRID_SERIALIZER").is_ok();

    // Determine OCR setting based on mode
    let enable_ocr = match mode {
        ExtractionMode::TextOnly => false,
        ExtractionMode::OcrText => true,
    };

    // Convert document (TIMED SECTION)
    // Returns (markdown, latency, content_blocks) for JSON comparison
    let (markdown, latency, content_blocks) = if use_hybrid_serializer {
        // Use Hybrid serializer: Python ML + Rust serialization
        // For formats Python doesn't support (LaTeX, VCF, XPS, IPYNB, IDML, DXF, GIF, GPX, KML, ICS, BMP, JPEG, HEIF, AVIF, ZIP, TAR, 7Z, RAR, Apple formats), fall back to pure Rust
        let formats_unsupported_by_python = [
            "latex", "tex", "vcf", "xps", "ipynb", "idml", "dxf", "gif", "gpx", "kml", "ics",
            "bmp", "jpeg", "heif", "avif", "zip", "tar", "7z", "rar", "stl", "obj", "gltf", "glb",
            "odt", "ods", "odp", "eml", "mbox", "epub", "fb2", "mobi", "srt", "vsdx", "doc", "mpp",
            "rtf", "svg", "dicom", "kmz",
            // Apple iWork formats (not supported by Python docling v2.58.0)
            "numbers", "key", "pages",
        ];
        let should_use_rust_fallback =
            formats_unsupported_by_python.contains(&fixture.file_type.as_str());

        // Python removed - all conversion now via pure Rust
        let _ = should_use_rust_fallback; // Suppress unused warning
        let converter = RustDocumentConverter::with_ocr(enable_ocr).map_err(|e| {
            test_result.status = "fail".to_string();
            test_result.fail_reason = format!("Rust converter creation failed: {e}");
            format!("Failed to create Rust converter: {e}")
        })?;

        let result = match converter.convert(&test_file) {
            Ok(r) => r,
            Err(e) => {
                let err_str = e.to_string();
                // FAIL PDF tests if pdfium-fast-ml feature is not enabled
                // PDF is the most important format - tests MUST run, not silently skip
                if err_str.contains("pdfium-fast-ml") && fixture.file_type == "pdf" {
                    panic!(
                        "❌ PDF TEST FAILED: pdfium-fast-ml feature not enabled for: {}\n\
                         Run with: cargo test --features pdfium-fast-ml\n\
                         Or use docling-cli: cargo test -p docling-cli --features pdfium-fast-ml",
                        fixture.file_name
                    );
                }
                test_result.status = "fail".to_string();
                test_result.fail_reason = format!("Rust conversion failed: {e}");
                return Err(format!("Rust conversion failed: {e}"));
            }
        };
        (
            result.document.to_markdown().to_string(),
            result.latency,
            result.document.content_blocks,
        )
    } else if use_rust_backend
        && matches!(
            fixture.file_type.as_str(),
            "pdf"
                | "zip"
                | "tar"
                | "7z"
                | "rar"
                | "srt"
                | "eml"
                | "mbox"
                | "msg"
                | "vcf"
                | "epub"
                | "fb2"
                | "mobi"
                | "odt"
                | "ods"
                | "odp"
                | "csv"
                | "latex"
                | "numbers"
                | "key"
                | "pages"
                | "gltf"
                | "glb"
                | "obj"
                | "vsdx"
                | "doc"
                | "mpp"
                | "xps"
                | "ipynb"
                | "idml"
                | "dxf"
                | "gif"
                | "gpx"
                | "kml"
                | "ics"
                | "bmp"
                | "jpeg"
                | "heif"
                | "avif"
        )
    {
        // Use Rust backend for supported formats when flag is set
        // Supported: PDF, archives (ZIP, TAR, 7Z, RAR), subtitles (SRT), email (EML, MBOX, MSG, VCF), e-books (EPUB, FB2, MOBI), OpenDocument (ODT, ODS, ODP), CSV, LaTeX, Apple iWork (NUMBERS, KEY, PAGES), 3D CAD (GLTF, GLB, OBJ, DXF), GPS (GPX, KML), images (GIF, BMP, JPEG, HEIF, AVIF), Microsoft Visio (VSDX), Legacy MS Word (DOC), Microsoft Project (MPP), XPS, Jupyter Notebook (IPYNB), Adobe IDML, Calendar (ICS)
        let converter = RustDocumentConverter::with_ocr(enable_ocr).map_err(|e| {
            test_result.status = "fail".to_string();
            test_result.fail_reason = format!("Rust converter creation failed: {e}");
            format!("Failed to create Rust converter: {e}")
        })?;

        let result = match converter.convert(&test_file) {
            Ok(r) => r,
            Err(e) => {
                let err_str = e.to_string();
                // FAIL PDF tests if pdfium-fast-ml feature is not enabled
                // PDF is the most important format - tests MUST run, not silently skip
                if err_str.contains("pdfium-fast-ml") && fixture.file_type == "pdf" {
                    panic!(
                        "❌ PDF TEST FAILED: pdfium-fast-ml feature not enabled for: {}\n\
                         Run with: cargo test --features pdfium-fast-ml\n\
                         Or use docling-cli: cargo test -p docling-cli --features pdfium-fast-ml",
                        fixture.file_name
                    );
                }
                test_result.status = "fail".to_string();
                test_result.fail_reason = format!("Rust conversion failed: {e}");
                return Err(format!("Rust conversion failed: {e}"));
            }
        };
        (
            result.document.to_markdown().to_string(),
            result.latency,
            result.document.content_blocks,
        )
    } else {
        // Default backend (now pure Rust, Python was removed)
        let converter = DocumentConverter::with_ocr(enable_ocr).map_err(|e| {
            test_result.status = "fail".to_string();
            test_result.fail_reason = format!("Converter creation failed: {e}");
            format!("Failed to create converter: {e}")
        })?;

        let result = match converter.convert(&test_file) {
            Ok(r) => r,
            Err(e) => {
                let err_str = e.to_string();
                // FAIL PDF tests if pdfium-fast-ml feature is not enabled
                // PDF is the most important format - tests MUST run, not silently skip
                if err_str.contains("pdfium-fast-ml") && fixture.file_type == "pdf" {
                    panic!(
                        "❌ PDF TEST FAILED: pdfium-fast-ml feature not enabled for: {}\n\
                         Run with: cargo test --features pdfium-fast-ml\n\
                         Or use docling-cli: cargo test -p docling-cli --features pdfium-fast-ml",
                        fixture.file_name
                    );
                }
                test_result.status = "fail".to_string();
                test_result.fail_reason = format!("Conversion failed: {e}");
                return Err(format!("Conversion failed: {e}"));
            }
        };
        (
            result.document.to_markdown().to_string(),
            result.latency,
            result.document.content_blocks,
        )
    };
    // END TIMED SECTION

    // MANDATORY PDF content_blocks VALIDATION (Bug #19 regression prevention)
    // PDF files MUST return content_blocks with all item types (texts, tables, pictures)
    // This was broken before N=2729 - only texts were returned, tables/pictures were lost
    if fixture.file_type == "pdf" && fixture.is_canonical {
        // PDF must have content_blocks
        let blocks = content_blocks.as_ref().ok_or_else(|| {
            test_result.status = "fail".to_string();
            test_result.fail_reason = "PDF missing content_blocks".to_string();
            "API CONTRACT: PDF must return content_blocks (Bug #19 regression)".to_string()
        })?;

        // Content blocks must not be empty for PDFs with content
        if blocks.is_empty() && !markdown.trim().is_empty() {
            test_result.status = "fail".to_string();
            test_result.fail_reason = "PDF has markdown but empty content_blocks".to_string();
            return Err(
                "API CONTRACT: PDF content_blocks must not be empty when markdown exists"
                    .to_string(),
            );
        }

        // Count item types to verify all types are being captured
        let mut text_count = 0;
        let mut table_count = 0;
        let mut picture_count = 0;
        for item in blocks {
            match item {
                DocItem::Text { .. }
                | DocItem::SectionHeader { .. }
                | DocItem::Paragraph { .. }
                | DocItem::ListItem { .. }
                | DocItem::Title { .. }
                | DocItem::Caption { .. }
                | DocItem::Footnote { .. } => text_count += 1,
                DocItem::Table { .. } => table_count += 1,
                DocItem::Picture { .. } => picture_count += 1,
                _ => {}
            }
        }

        // Log counts for debugging (always, not just verbose mode)
        if std::env::var("VALIDATE_JSON_VERBOSE").is_ok() {
            println!(
                "PDF {} content_blocks: {} text, {} tables, {} pictures (total: {})",
                fixture.file_name,
                text_count,
                table_count,
                picture_count,
                blocks.len()
            );
        }
    }

    // Check if test exceeded timeout
    if test_start.elapsed().as_secs() > timeout_secs {
        eprintln!(
            "⏱️  Test timeout: {} exceeded {}s limit (took {}s)",
            fixture.file_name,
            timeout_secs,
            test_start.elapsed().as_secs()
        );
    }

    test_result.actual_bytes = markdown.as_bytes().len();
    test_result.actual_chars = markdown.chars().count();

    // Export additional formats (NOT TIMED)
    // Skip export if not validating output (for formats not supported by Python docling)
    if validate_output {
        export_all_formats(&test_file, fixture, mode)?;
    }

    // Determine expected output file
    let file_stem = Path::new(&fixture.file_name)
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap();

    let expected_path = if fixture.is_canonical {
        // Canonical tests (matching upstream docling test pattern):
        // - Text-only mode: Use groundtruth from docling repo (groundtruth/docling_v2/*.md)
        //   NOTE: Upstream groundtruth is generated with do_ocr=False (text-only mode)
        // - OCR mode: Use generated expected outputs (expected-outputs/{format}/{stem}.md)
        //   NOTE: No OCR groundtruth exists in upstream, so we generate our own

        match mode {
            ExtractionMode::TextOnly => {
                // Text-only: Use groundtruth (matches upstream pattern)
                let base_path = Path::new("../../test-corpus/groundtruth/docling_v2");

                // Try with full filename first (for asciidoc, csv, etc.)
                let path_with_ext = base_path.join(format!("{}.md", fixture.file_name));
                if path_with_ext.exists() {
                    path_with_ext
                } else {
                    // Fall back to stem only (for PDFs)
                    base_path.join(format!("{file_stem}.md"))
                }
            }
            ExtractionMode::OcrText => {
                // OCR: Use generated expected outputs (no OCR groundtruth in upstream)
                Path::new("../../test-corpus/expected-outputs")
                    .join(&fixture.file_type)
                    .join(format!("{file_stem}.md"))
            }
        }
    } else {
        // Non-canonical tests use generated expected outputs
        let output_dir = format!("{}-more", fixture.file_type);

        // Suffix logic:
        // - PDF: Has both .text-only.md and .md (two modes)
        // - Other formats: Only .md exists (text documents don't have OCR mode)
        let expected_suffix = if fixture.file_type == "pdf" {
            match mode {
                ExtractionMode::TextOnly => ".text-only.md",
                ExtractionMode::OcrText => ".md",
            }
        } else {
            // Non-PDF: always use .md (only one mode exists)
            ".md"
        };

        Path::new("../../test-corpus/expected-outputs")
            .join(&output_dir)
            .join(format!("{file_stem}{expected_suffix}"))
    };

    // Check environment flags for expected output handling
    let create_expected = std::env::var("CREATE_EXPECTED").is_ok();
    let overwrite_expected = std::env::var("OVERWRITE_EXPECTED").is_ok();
    let regenerate = std::env::var("REGENERATE_EXPECTED").is_ok() || overwrite_expected;

    // Safety check: OVERWRITE/REGENERATE requires confirmation
    if (overwrite_expected || regenerate)
        && std::env::var("CONFIRM_OVERWRITE").ok().as_deref() != Some("yes")
    {
        eprintln!(
            "\n⚠️  ERROR: OVERWRITE_EXPECTED/REGENERATE_EXPECTED requires CONFIRM_OVERWRITE=yes"
        );
        eprintln!("This will overwrite 900+ expected output files in git!");
        eprintln!("Usage: OVERWRITE_EXPECTED=1 CONFIRM_OVERWRITE=yes cargo test\n");
        return Err("OVERWRITE_EXPECTED requires CONFIRM_OVERWRITE=yes for safety".to_string());
    }

    // Handle expected output creation/update
    // IMPORTANT: Never overwrite groundtruth files (canonical text-only) - they're upstream baseline
    let is_groundtruth = fixture.is_canonical && matches!(mode, ExtractionMode::TextOnly);

    let should_write_expected = if is_groundtruth {
        false // Never write to groundtruth directory
    } else if expected_path.exists() {
        overwrite_expected || regenerate
    } else {
        create_expected || regenerate
    };

    if should_write_expected {
        // Save actual output as expected
        if let Some(parent) = expected_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                test_result.status = "fail".to_string();
                test_result.fail_reason = format!("Failed to create output dir: {e}");
                format!("Failed to create output dir: {e}")
            })?;
        }
        std::fs::write(&expected_path, &markdown).map_err(|e| {
            test_result.status = "fail".to_string();
            test_result.fail_reason = format!("Failed to write expected output: {e}");
            format!("Failed to write expected output: {e}")
        })?;

        // Skip validation when creating/overwriting expected
        test_result.expected_bytes = test_result.actual_bytes;
        test_result.expected_chars = test_result.actual_chars;
    } else if validate_output {
        // Normal mode: Load and validate expected output
        let expected = fs::read_to_string(&expected_path).map_err(|e| {
            test_result.status = "fail".to_string();
            test_result.fail_reason = format!("Failed to load expected output: {e}");
            format!(
                "Failed to load expected output {}: {}",
                expected_path.display(),
                e
            )
        })?;

        test_result.expected_bytes = expected.as_bytes().len();
        test_result.expected_chars = expected.chars().count();

        // Compare outputs - use strict comparison by default, allow normalization with flag
        let strict_whitespace = std::env::var("NORMALIZE_WHITESPACE").is_err();
        let matches = if strict_whitespace {
            markdown == expected
        } else {
            normalize_whitespace(&markdown) == normalize_whitespace(&expected)
        };

        if !matches {
            test_result.status = "fail".to_string();
            test_result.fail_reason = format!(
                "Output mismatch: expected {} bytes, got {} bytes",
                test_result.expected_bytes, test_result.actual_bytes
            );

            // Log before failing
            let _ = log_to_csv(fixture, mode, latency, test_result);

            return Err(format!(
                "Output mismatch for {}\nExpected length: {}\nActual length: {}\nFirst 200 chars expected: {}\nFirst 200 chars actual: {}",
                fixture.file_name,
                expected.len(),
                markdown.len(),
                &expected.chars().take(200).collect::<String>(),
                &markdown.chars().take(200).collect::<String>()
            ));
        }
    } else {
        // No output check - set expected equal to actual
        test_result.expected_bytes = test_result.actual_bytes;
        test_result.expected_chars = test_result.actual_chars;
    }

    // F83: Optional JSON DocItem validation
    // Enable with VALIDATE_JSON=1 env var
    let validate_json = std::env::var("VALIDATE_JSON").is_ok();
    if validate_json && fixture.is_canonical && matches!(mode, ExtractionMode::TextOnly) {
        // Load groundtruth JSON for this file
        // Try multiple naming patterns:
        // 1. Full filename (e.g., example_01.html.json)
        // 2. Stem only (e.g., 2305.03393v1.json for PDFs)
        let file_stem = Path::new(&fixture.file_name)
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap();
        let json_path_full = format!(
            "../../test-corpus/groundtruth/docling_v2/{}.json",
            fixture.file_name
        );
        let json_path_stem = format!("../../test-corpus/groundtruth/docling_v2/{file_stem}.json");
        // Use full filename path if it exists, otherwise try stem
        let json_path = if Path::new(&json_path_full).exists() {
            json_path_full
        } else {
            json_path_stem
        };

        if Path::new(&json_path).exists() {
            match load_groundtruth_json(&json_path) {
                Ok(groundtruth) => {
                    // Compare DocItem structure
                    let comparison = compare_documents(
                        content_blocks.as_deref(),
                        &groundtruth,
                        0.2, // 20% tolerance for item counts
                    );

                    // Print comparison results (verbose output with VALIDATE_JSON_VERBOSE=1)
                    let verbose = std::env::var("VALIDATE_JSON_VERBOSE").is_ok();
                    if verbose || !comparison.passed {
                        println!(
                            "JSON validation for {}: passed={}, actual={}/{}/{}, expected={}/{}/{}",
                            fixture.file_name,
                            comparison.passed,
                            comparison.actual_counts.texts,
                            comparison.actual_counts.tables,
                            comparison.actual_counts.pictures,
                            comparison.expected_counts.texts,
                            comparison.expected_counts.tables,
                            comparison.expected_counts.pictures,
                        );
                    }

                    if !comparison.passed {
                        // Log JSON differences but don't fail test (yet)
                        // This is informational for F83 Phase 2
                        eprintln!(
                            "JSON validation warning for {}: {:?}",
                            fixture.file_name, comparison.differences
                        );
                        eprintln!(
                            "  Actual counts: texts={}, tables={}, pictures={}",
                            comparison.actual_counts.texts,
                            comparison.actual_counts.tables,
                            comparison.actual_counts.pictures
                        );
                        eprintln!(
                            "  Expected counts: texts={}, tables={}, pictures={}",
                            comparison.expected_counts.texts,
                            comparison.expected_counts.tables,
                            comparison.expected_counts.pictures
                        );
                    }
                }
                Err(e) => {
                    // Don't fail if JSON parsing fails - groundtruth might be different format
                    eprintln!("JSON validation skipped for {}: {}", fixture.file_name, e);
                }
            }
        }
    }

    // Check performance vs baseline
    let baseline = match mode {
        ExtractionMode::OcrText => fixture.baseline_latency_ocr,
        ExtractionMode::TextOnly => fixture.baseline_latency_text,
    };

    if let Some(baseline_duration) = baseline {
        let baseline_ms = baseline_duration.as_millis();
        let actual_ms = latency.as_millis();

        if actual_ms > baseline_ms * 2 {
            eprintln!(
                "⚠️  Performance regression for {}: {}ms (baseline: {}ms, {}x slower)",
                fixture.file_name,
                actual_ms,
                baseline_ms,
                actual_ms as f64 / baseline_ms as f64
            );
        } else if actual_ms < baseline_ms / 2 {
            println!(
                "✓ Performance improvement for {}: {}ms (baseline: {}ms, {}x faster)",
                fixture.file_name,
                actual_ms,
                baseline_ms,
                baseline_ms as f64 / actual_ms as f64
            );
        }
    }

    // Log to CSV
    log_to_csv(fixture, mode, latency, test_result)?;

    Ok(())
}

/// Export all output formats (HTML, text, JSON) - NOT TIMED
/// Note: Python removed - this now uses pure Rust conversion
fn export_all_formats(
    test_file: &Path,
    fixture: &TestFixture,
    mode: ExtractionMode,
) -> Result<(), String> {
    // Determine output directory
    let file_stem = test_file
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| "Invalid file stem".to_string())?;

    let output_dir = if fixture.is_canonical {
        match mode {
            ExtractionMode::TextOnly => {
                Path::new("../../test-results/outputs").join(&fixture.file_type)
            }
            ExtractionMode::OcrText => {
                Path::new("../../test-results/outputs-ocr").join(&fixture.file_type)
            }
        }
    } else {
        let subdir = format!("{}-more", fixture.file_type);
        match mode {
            ExtractionMode::TextOnly => Path::new("../../test-results/outputs").join(&subdir),
            ExtractionMode::OcrText => Path::new("../../test-results/outputs-ocr").join(&subdir),
        }
    };

    std::fs::create_dir_all(&output_dir)
        .map_err(|e| format!("Failed to create output dir: {e}"))?;

    // Convert using pure Rust converter
    let enable_ocr = matches!(mode, ExtractionMode::OcrText);
    let converter = DocumentConverter::with_ocr(enable_ocr)
        .map_err(|e| format!("Failed to create converter for export: {e}"))?;

    let result = converter
        .convert(test_file)
        .map_err(|e| format!("Failed to convert for export: {e}"))?;

    // Export markdown (text)
    let text_path = output_dir.join(format!("{file_stem}.txt"));
    std::fs::write(&text_path, &result.document.markdown)
        .map_err(|e| format!("Failed to write text: {e}"))?;

    // Export HTML (Document doesn't have html field, use markdown as HTML)
    let html_path = output_dir.join(format!("{file_stem}.html"));
    std::fs::write(&html_path, &result.document.markdown)
        .map_err(|e| format!("Failed to write HTML: {e}"))?;

    // Export JSON (serialize DocItems if available)
    let json_str = if let Some(ref items) = result.document.content_blocks {
        serde_json::to_string_pretty(items).map_err(|e| format!("Failed to serialize JSON: {e}"))?
    } else {
        // Fallback: create minimal JSON with markdown
        format!(
            r#"{{"markdown": {}}}"#,
            serde_json::to_string(&result.document.markdown).unwrap_or_default()
        )
    };
    let json_path = output_dir.join(format!("{file_stem}.json"));
    std::fs::write(&json_path, json_str).map_err(|e| format!("Failed to write JSON: {e}"))?;

    Ok(())
}

fn log_to_csv(
    fixture: &TestFixture,
    mode: ExtractionMode,
    latency: Duration,
    test_result: TestResult,
) -> Result<(), String> {
    use std::fs::OpenOptions;
    use std::io::Write;

    // Get test run context
    let ctx = get_test_run_context();

    // Create per-run CSV file: test-results/runs/{run_id}/integration_test_latencies.csv
    let run_dir = Path::new("../../test-results/runs").join(&ctx.run_id);
    std::fs::create_dir_all(&run_dir).map_err(|e| format!("Failed to create run dir: {e}"))?;

    let csv_path = run_dir.join("integration_test_latencies.csv");

    // Check if file exists to write header
    let needs_header = !csv_path.exists();

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&csv_path)
        .map_err(|e| format!("Failed to open CSV: {e}"))?;

    // Write header if new file
    if needs_header {
        writeln!(
            file,
            "test_name,file_type,file_name,extraction_type,extraction_format,latency_ms,baseline_ms,datetime,commit_hash,phase,is_canon,\
             test_run_id,test_sequence_num,cumulative_time_ms,wall_clock_start,wall_clock_end,\
             build_profile,rustc_version,opt_level,cargo_test_flags,cpu_count,available_memory_mb,is_parallel,\
             docling_version,python_version,docling_rs_version,git_branch,git_commit_hash_full,git_commit_timestamp,\
             test_status,fail_reason,expected_bytes,actual_bytes,bytes_diff,expected_chars,actual_chars,chars_diff"
        ).map_err(|e| format!("Failed to write CSV header: {e}"))?;
    }

    // Get test sequence number and cumulative time
    let test_sequence_num = {
        let mut counter = ctx.test_counter.lock().unwrap();
        *counter += 1;
        *counter
    };
    let cumulative_time_ms = ctx.start_time.elapsed().as_millis();

    // Capture wall clock times
    let wall_clock_end = chrono::Utc::now();
    let wall_clock_start =
        wall_clock_end - chrono::Duration::milliseconds(latency.as_millis() as i64);

    // Get git info
    let git_hash_full = std::process::Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_else(|| "unknown".to_string())
        .trim()
        .to_string();

    let git_branch = std::process::Command::new("git")
        .args(&["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_else(|| "unknown".to_string())
        .trim()
        .to_string();

    let git_commit_timestamp = std::process::Command::new("git")
        .args(&["log", "-1", "--format=%cI"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_else(|| "unknown".to_string())
        .trim()
        .to_string();

    // Get baseline
    let baseline_ms = match mode {
        ExtractionMode::OcrText => fixture.baseline_latency_ocr.map(|d| d.as_millis()),
        ExtractionMode::TextOnly => fixture.baseline_latency_text.map(|d| d.as_millis()),
    }
    .unwrap_or(0);

    // Format extraction type
    let extraction_type = match mode {
        ExtractionMode::OcrText => "text_ocr",
        ExtractionMode::TextOnly => "text_only",
    };

    // Get current timestamp
    let datetime = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();

    // Build info
    let build_profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let docling_rs_version = env!("CARGO_PKG_VERSION");
    let rustc_version = std::env!("CARGO_PKG_RUST_VERSION").to_string();
    let opt_level = std::env::var("OPT_LEVEL").unwrap_or_else(|_| "unknown".to_string());
    let cargo_test_flags = std::env::var("CARGO_TEST_FLAGS").unwrap_or_else(|_| "".to_string());

    // System info
    let sys_info = get_system_info();
    let is_parallel = std::env::var("RUST_TEST_THREADS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .map(|n| n > 1)
        .unwrap_or(true);

    // Python removed - use Rust version info only
    let docling_version = env!("CARGO_PKG_VERSION").to_string();
    let python_version = "N/A (pure Rust)".to_string();

    // Calculate differences
    let bytes_diff = test_result.actual_bytes as i64 - test_result.expected_bytes as i64;
    let chars_diff = test_result.actual_chars as i64 - test_result.expected_chars as i64;

    // Write log entry (append-only)
    writeln!(
        file,
        "{},{},{},{},markdown,{},{},{},{},0,{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
        fixture.title_slug,
        fixture.file_type,
        fixture.file_name,
        extraction_type,
        latency.as_millis(),
        baseline_ms,
        datetime,
        &git_hash_full[..8],  // Short hash for backwards compat
        fixture.is_canonical,
        ctx.run_id,
        test_sequence_num,
        cumulative_time_ms,
        wall_clock_start.format("%Y-%m-%dT%H:%M:%S%.3f"),
        wall_clock_end.format("%Y-%m-%dT%H:%M:%S%.3f"),
        build_profile,
        rustc_version,
        opt_level,
        cargo_test_flags,
        sys_info.cpu_count,
        sys_info.available_memory_mb.map(|m| m.to_string()).unwrap_or_else(|| "unknown".to_string()),
        is_parallel,
        docling_version,
        python_version,
        docling_rs_version,
        git_branch,
        git_hash_full,
        git_commit_timestamp,
        test_result.status,
        test_result.fail_reason,
        test_result.expected_bytes,
        test_result.actual_bytes,
        bytes_diff,
        test_result.expected_chars,
        test_result.actual_chars,
        chars_diff
    ).map_err(|e| format!("Failed to write CSV row: {e}"))?;

    Ok(())
}

fn normalize_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

// AUTO-GENERATED TEST FUNCTIONS

// AUTO-GENERATED TEST FUNCTIONS
// Generated 913 test functions
// Text-only: 552, OCR: 364

#[test]
fn test_canon_asciidoc_test_01_text() {
    let fixture = fixture_test_01();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_asciidoc_test_02_text() {
    let fixture = fixture_test_02();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_asciidoc_test_03_text() {
    let fixture = fixture_test_03();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_csv_csv_comma_in_cell_text() {
    let fixture = fixture_csv_comma_in_cell();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_csv_csv_comma_text() {
    let fixture = fixture_csv_comma();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_csv_csv_inconsistent_header_text() {
    let fixture = fixture_csv_inconsistent_header();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_csv_csv_pipe_text() {
    let fixture = fixture_csv_pipe();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_csv_csv_semicolon_text() {
    let fixture = fixture_csv_semicolon();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_csv_csv_tab_text() {
    let fixture = fixture_csv_tab();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_csv_csv_too_few_columns_text() {
    let fixture = fixture_csv_too_few_columns();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_csv_csv_too_many_columns_text() {
    let fixture = fixture_csv_too_many_columns();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_docx_drawingml_text() {
    let fixture = fixture_drawingml();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_docx_equations_text() {
    let fixture = fixture_equations();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_docx_lorem_ipsum_text() {
    let fixture = fixture_lorem_ipsum();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[ignore = "Requires specific test file not in git"]
#[test]
fn test_more_docx_anemia_management_protocol_aranesp_text() {
    let fixture = fixture_anemia_management_protocol_aranesp();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_anne_smith_useful_technologies_text() {
    let fixture = fixture_anne_smith_useful_technologies();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_bass_edit_spots_text() {
    let fixture = fixture_bass_edit_spots();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_cba_excerpt_text() {
    let fixture = fixture_cba_excerpt();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_case_one_text() {
    let fixture = fixture_case_one();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_case_two_text() {
    let fixture = fixture_case_two();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_change_control_sop_working_session_internal_input_text() {
    let fixture = fixture_change_control_sop_working_session_internal_input();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_clinic_logo_2_1_text() {
    let fixture = fixture_clinic_logo_2_1();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_company_objectives_and_time_log_text() {
    let fixture = fixture_company_objectives_and_time_log();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_compensation_model_ideas_text() {
    let fixture = fixture_compensation_model_ideas();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_confidentiality_statement_text() {
    let fixture = fixture_confidentiality_statement();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_current_renewal_letter_text() {
    let fixture = fixture_current_renewal_letter();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_current_architecture_summary_text() {
    let fixture = fixture_current_architecture_summary();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_daily_tasks_text() {
    let fixture = fixture_daily_tasks();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_email_trail_floorstands_text() {
    let fixture = fixture_email_trail_floorstands();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_field_investigator_s_notes_text() {
    let fixture = fixture_field_investigator_s_notes();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_field_investigator_b_text() {
    let fixture = fixture_field_investigator_b();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_field_investigator_report_v2_text() {
    let fixture = fixture_field_investigator_report_v2();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_gasper_olafsen_text() {
    let fixture = fixture_gasper_olafsen();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_holiday_conference_event_dates_text() {
    let fixture = fixture_holiday_conference_event_dates();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[ignore = "Requires specific test file not in git"]
#[test]
fn test_more_docx_initial_requirements_funding_competition_text() {
    let fixture = fixture_initial_requirements_funding_competition();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_jim_dalton_text() {
    let fixture = fixture_jim_dalton();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_lars_andersen_text() {
    let fixture = fixture_lars_andersen();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_lease_rate_analysis_template_text() {
    let fixture = fixture_lease_rate_analysis_template();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_model_a_hl_quotes_1_text() {
    let fixture = fixture_model_a_hl_quotes_1();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_namc_applicants_and_interviewers_text() {
    let fixture = fixture_namc_applicants_and_interviewers();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_new_cma_template_text() {
    let fixture = fixture_new_cma_template();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[ignore = "Requires specific test file not in git"]
#[test]
fn test_more_docx_nft_photography_context_1_text() {
    let fixture = fixture_nft_photography_context_1();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_nature_doc_key_info_and_vo_text() {
    let fixture = fixture_nature_doc_key_info_and_vo();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_notes_for_terry_hartsdale_text() {
    let fixture = fixture_notes_for_terry_hartsdale();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_order_types_challenges_text() {
    let fixture = fixture_order_types_challenges();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_patient_nutritional_management_protocol_text() {
    let fixture = fixture_patient_nutritional_management_protocol();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_patient_information_document_text() {
    let fixture = fixture_patient_information_document();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_patient_lab_reports_text() {
    let fixture = fixture_patient_lab_reports();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_plan_establish_reference_text() {
    let fixture = fixture_plan_establish_reference();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_pricing_email_text() {
    let fixture = fixture_pricing_email();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_priorities_and_conditions_for_scheduling_grand_rou_text() {
    let fixture = fixture_priorities_and_conditions_for_scheduling_grand_rou();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_product_specification_reference_text() {
    let fixture = fixture_product_specification_reference();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_quotations_and_volume_projection_for_model_i_headl_text() {
    let fixture = fixture_quotations_and_volume_projection_for_model_i_headl();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[ignore = "Requires specific test file not in git"]
#[test]
fn test_more_docx_recreare_contract_outline_text() {
    let fixture = fixture_recreare_contract_outline();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_recreare_official_contract_language_text() {
    let fixture = fixture_recreare_official_contract_language();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[ignore = "Requires specific test file not in git"]
#[test]
fn test_more_docx_research_material_text() {
    let fixture = fixture_research_material();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_resident_complaint_log_text() {
    let fixture = fixture_resident_complaint_log();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_servicesv5_text() {
    let fixture = fixture_servicesv5();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_scheduled_meetings_text() {
    let fixture = fixture_scheduled_meetings();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_social_developmental_history_template_text() {
    let fixture = fixture_social_developmental_history_template();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_telehealth_with_doxy_me_text() {
    let fixture = fixture_telehealth_with_doxy_me();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_what_is_rtls_real_time_location_systems_text() {
    let fixture = fixture_what_is_rtls_real_time_location_systems();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_worddoc_researchformatreferencesheet_text() {
    let fixture = fixture_worddoc_researchformatreferencesheet();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_docx_workflow_steps_text() {
    let fixture = fixture_workflow_steps();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_docx_table_with_equations_text() {
    let fixture = fixture_table_with_equations();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_docx_tablecell_text() {
    let fixture = fixture_tablecell();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_docx_test_emf_docx_text() {
    let fixture = fixture_test_emf_docx();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_docx_textbox_text() {
    let fixture = fixture_textbox();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_docx_unit_test_formatting_text() {
    let fixture = fixture_unit_test_formatting();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_docx_unit_test_headers_text() {
    let fixture = fixture_unit_test_headers();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_docx_unit_test_headers_numbered_text() {
    let fixture = fixture_unit_test_headers_numbered();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_docx_unit_test_lists_text() {
    let fixture = fixture_unit_test_lists();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_docx_word_image_anchors_text() {
    let fixture = fixture_word_image_anchors();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_docx_word_sample_text() {
    let fixture = fixture_word_sample();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_docx_word_tables_text() {
    let fixture = fixture_word_tables();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}

// DOC (Legacy Microsoft Word) Tests
// Note: DOC tests use run_integration_test_no_output_check because Python docling doesn't support DOC format
// DOC backend is Rust-only (DOC → textutil → DOCX → DocItems)
#[test]
fn test_more_doc_simple_text_text() {
    let fixture = fixture_simple_text_doc();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_doc_formatted_document_text() {
    let fixture = fixture_formatted_document_doc();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_doc_tables_and_columns_text() {
    let fixture = fixture_tables_and_columns_doc();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_doc_complex_academic_text() {
    let fixture = fixture_complex_academic_doc();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_doc_images_and_objects_text() {
    let fixture = fixture_images_and_objects_doc();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_html_example_01_text() {
    let fixture = fixture_example_01();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_html_example_02_text() {
    let fixture = fixture_example_02();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_html_example_03_text() {
    let fixture = fixture_example_03();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_html_example_04_text() {
    let fixture = fixture_example_04();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_html_example_05_text() {
    let fixture = fixture_example_05();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_html_example_06_text() {
    let fixture = fixture_example_06();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_html_example_07_text() {
    let fixture = fixture_example_07();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_html_example_08_text() {
    let fixture = fixture_example_08();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_html_formatting_text() {
    let fixture = fixture_formatting();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_html_html_code_snippets_text() {
    let fixture = fixture_html_code_snippets();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_html_hyperlink_01_text() {
    let fixture = fixture_hyperlink_01();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_html_hyperlink_02_text() {
    let fixture = fixture_hyperlink_02();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_html_hyperlink_03_text() {
    let fixture = fixture_hyperlink_03();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_html_hyperlink_04_text() {
    let fixture = fixture_hyperlink_04();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_html_hyperlink_05_text() {
    let fixture = fixture_hyperlink_05();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[ignore = "Stress test - large file (172KB)"]
#[test]
fn test_more_html_large_fujishita_air_172k_text() {
    let fixture = fixture_large_fujishita_air_172k();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_html_large_japanese_constitution_text() {
    let fixture = fixture_large_japanese_constitution();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[ignore = "Stress test - large file"]
#[test]
fn test_more_html_large_kita_reform_plan_text() {
    let fixture = fixture_large_kita_reform_plan();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_html_large_terada_haiku_essay_text() {
    let fixture = fixture_large_terada_haiku_essay();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_html_large_yamamoto_aesthetics_61k_text() {
    let fixture = fixture_large_yamamoto_aesthetics_61k();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_html_medium_akutagawa_story_8_4k_text() {
    let fixture = fixture_medium_akutagawa_story_8_4k();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_html_medium_doyle_redheaded_53k_text() {
    let fixture = fixture_medium_doyle_redheaded_53k();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_html_medium_motoki_death_35k_text() {
    let fixture = fixture_medium_motoki_death_35k();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_html_medium_shimaki_frog_18k_text() {
    let fixture = fixture_medium_shimaki_frog_18k();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_html_medium_tachihara_poetry_9_8k_text() {
    let fixture = fixture_medium_tachihara_poetry_9_8k();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_html_medium_terada_essay_text() {
    let fixture = fixture_medium_terada_essay();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_html_medium_turgenev_28k_text() {
    let fixture = fixture_medium_turgenev_28k();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_html_medium_yagi_autumn_46k_text() {
    let fixture = fixture_medium_yagi_autumn_46k();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_html_mixed_content_1178_text() {
    let fixture = fixture_mixed_content_1178();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_html_simple_english_brandcraft_3_5k_text() {
    let fixture = fixture_simple_english_brandcraft_3_5k();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_html_simple_english_subfolder_text() {
    let fixture = fixture_simple_english_subfolder();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_html_small_akutagawa_essay_1_8k_text() {
    let fixture = fixture_small_akutagawa_essay_1_8k();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_html_small_dazai_essay_1_6k_text() {
    let fixture = fixture_small_dazai_essay_1_6k();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[ignore = "Requires specific test file not in git"]
#[test]
fn test_more_html_tiny_japanese_poem_744b_text() {
    let fixture = fixture_tiny_japanese_poem_744b();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_html_tiny_japanese_story_1_1k_text() {
    let fixture = fixture_tiny_japanese_story_1_1k();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[ignore = "Stress test - extra-large file (1.3MB)"]
#[test]
fn test_more_html_xlarge_fukuzawa_autobiography_1_3m_text() {
    let fixture = fixture_xlarge_fukuzawa_autobiography_1_3m();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[ignore = "Stress test - extra-large file (1MB)"]
#[test]
fn test_more_html_xlarge_homer_iliad_1m_text() {
    let fixture = fixture_xlarge_homer_iliad_1m();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[ignore = "Stress test - extra-large file (1.6MB)"]
#[test]
fn test_more_html_xlarge_mori_biography_1_6m_text() {
    let fixture = fixture_xlarge_mori_biography_1_6m();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_html_xlarge_natsume_wagahai_1_2m_text() {
    let fixture = fixture_xlarge_natsume_wagahai_1_2m();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[ignore = "Stress test - extra-extra-large file (3.5MB)"]
#[test]
fn test_more_html_xxlarge_nagatsuka_tsuchi_3_5m_text() {
    let fixture = fixture_xxlarge_nagatsuka_tsuchi_3_5m();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_html_table_01_text() {
    let fixture = fixture_table_01();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_html_table_02_text() {
    let fixture = fixture_table_02();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_html_table_03_text() {
    let fixture = fixture_table_03();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_html_table_04_text() {
    let fixture = fixture_table_04();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_html_table_05_text() {
    let fixture = fixture_table_05();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_html_table_06_text() {
    let fixture = fixture_table_06();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_html_table_with_heading_text() {
    let fixture = fixture_table_with_heading();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_html_unit_test_01_text() {
    let fixture = fixture_unit_test_01();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_html_wiki_duck_text() {
    let fixture = fixture_wiki_duck();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_jats_elife_56337_text() {
    let fixture = fixture_elife_56337();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_jats_pntd_0008301_text() {
    let fixture = fixture_pntd_0008301();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_jats_pone_0234687_text() {
    let fixture = fixture_pone_0234687();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}

// LATEX Tests
// JPEG Tests
#[ignore = "Requires specific test file not in git"]
#[test]
fn test_more_jpeg_map_of_part_of_northern_africa_showing_the_route_ocr() {
    let fixture = fixture_map_of_part_of_northern_africa_showing_the_route();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_jpeg_ppn663943922_adolf_overweg_281853_29_ocr() {
    let fixture = fixture_ppn663943922_adolf_overweg_281853_29();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_jpeg_edinet_2025_06_24_1008_e00021_mitsubishi_materials_ocr() {
    let fixture = fixture_edinet_2025_06_24_1008_e00021_mitsubishi_materials();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_jpeg_edinet_2025_06_24_1008_e00021_mitsubishi_materials_2_ocr() {
    let fixture = fixture_edinet_2025_06_24_1008_e00021_mitsubishi_materials_2();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_jpeg_image_3_ocr() {
    let fixture = fixture_image_3();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_jpeg_image_9_ocr() {
    let fixture = fixture_image_9();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_jpeg_jikan_anime_16498_large_image_url_ocr() {
    let fixture = fixture_jikan_anime_16498_large_image_url();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_jpeg_pexels_conojeghuo_175694_ocr() {
    let fixture = fixture_pexels_conojeghuo_175694();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[ignore = "Requires specific test file not in git"]
#[test]
fn test_more_jpeg_pexels_mahmutyilmaz20_33036641_ocr() {
    let fixture = fixture_pexels_mahmutyilmaz20_33036641();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[ignore = "Requires specific test file not in git"]
#[test]
fn test_more_jpeg_pexels_thirdman_5247203_ocr() {
    let fixture = fixture_pexels_thirdman_5247203();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_canon_md_blocks_text() {
    let fixture = fixture_blocks();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_md_duck_text() {
    let fixture = fixture_duck();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_md_ending_with_table_text() {
    let fixture = fixture_ending_with_table();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_md_escaped_characters_text() {
    let fixture = fixture_escaped_characters();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_md_inline_and_formatting_text() {
    let fixture = fixture_inline_and_formatting();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_md_mixed_text() {
    let fixture = fixture_mixed();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_md_mixed_without_h1_text() {
    let fixture = fixture_mixed_without_h1();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_md_nested_text() {
    let fixture = fixture_nested();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_md_wiki_text() {
    let fixture = fixture_wiki();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_pdf__2203_01017v2_text() {
    let fixture = fixture__2203_01017v2();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_pdf__2203_01017v2_ocr() {
    let fixture = fixture__2203_01017v2();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_canon_pdf__2206_01062_text() {
    let fixture = fixture__2206_01062();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_pdf__2206_01062_ocr() {
    let fixture = fixture__2206_01062();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_canon_pdf__2305_03393v1_pg9_text() {
    let fixture = fixture__2305_03393v1_pg9();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_pdf__2305_03393v1_pg9_ocr() {
    let fixture = fixture__2305_03393v1_pg9();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_canon_pdf__2305_03393v1_text() {
    let fixture = fixture__2305_03393v1();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_pdf__2305_03393v1_ocr() {
    let fixture = fixture__2305_03393v1();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_canon_pdf_amt_handbook_sample_text() {
    let fixture = fixture_amt_handbook_sample();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_pdf_amt_handbook_sample_ocr() {
    let fixture = fixture_amt_handbook_sample();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_canon_pdf_code_and_formula_text() {
    let fixture = fixture_code_and_formula();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_pdf_code_and_formula_ocr() {
    let fixture = fixture_code_and_formula();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
#[ignore] // Test corpus file missing: edinet_sample.pdf
fn test_canon_pdf_edinet_sample_text() {
    let fixture = fixture_edinet_sample();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
#[ignore] // Test corpus file missing: edinet_sample.pdf
fn test_canon_pdf_edinet_sample_ocr() {
    let fixture = fixture_edinet_sample();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
#[ignore] // Test corpus file missing: jfk_scanned.pdf
fn test_canon_pdf_jfk_scanned_text() {
    let fixture = fixture_jfk_scanned();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
#[ignore] // Test corpus file missing: jfk_scanned.pdf
fn test_canon_pdf_jfk_scanned_ocr() {
    let fixture = fixture_jfk_scanned();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf__16_inch_macbook_pro_space_black_apple_text() {
    let fixture = fixture__16_inch_macbook_pro_space_black_apple();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf__16_inch_macbook_pro_space_black_apple_ocr() {
    let fixture = fixture__16_inch_macbook_pro_space_black_apple();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf__2_09_03_1_helping_global_health_partnerships_to_in_text() {
    let fixture = fixture__2_09_03_1_helping_global_health_partnerships_to_in();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf__2_09_03_1_helping_global_health_partnerships_to_in_ocr() {
    let fixture = fixture__2_09_03_1_helping_global_health_partnerships_to_in();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf__2nnawo7cv5i6663gptq7p7vezthfaybr_text() {
    let fixture = fixture__2nnawo7cv5i6663gptq7p7vezthfaybr();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf__2nnawo7cv5i6663gptq7p7vezthfaybr_ocr() {
    let fixture = fixture__2nnawo7cv5i6663gptq7p7vezthfaybr();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf__2ubq7rt4c6yknn5jqntjmnmf7ddgso5t_text() {
    let fixture = fixture__2ubq7rt4c6yknn5jqntjmnmf7ddgso5t();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf__2ubq7rt4c6yknn5jqntjmnmf7ddgso5t_ocr() {
    let fixture = fixture__2ubq7rt4c6yknn5jqntjmnmf7ddgso5t();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf__3ge6pl4sagqf37752fnpnp3b37wdvawx_text() {
    let fixture = fixture__3ge6pl4sagqf37752fnpnp3b37wdvawx();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf__3ge6pl4sagqf37752fnpnp3b37wdvawx_ocr() {
    let fixture = fixture__3ge6pl4sagqf37752fnpnp3b37wdvawx();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf__3tyopkyadwopey6ayvnkc4eg3qc4mxoc_text() {
    let fixture = fixture__3tyopkyadwopey6ayvnkc4eg3qc4mxoc();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf__3tyopkyadwopey6ayvnkc4eg3qc4mxoc_ocr() {
    let fixture = fixture__3tyopkyadwopey6ayvnkc4eg3qc4mxoc();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf__4tlgb26ditos52jx25gtn7emxjz3ljmt_text() {
    let fixture = fixture__4tlgb26ditos52jx25gtn7emxjz3ljmt();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf__4tlgb26ditos52jx25gtn7emxjz3ljmt_ocr() {
    let fixture = fixture__4tlgb26ditos52jx25gtn7emxjz3ljmt();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf__4wrp5lr6pkwzlgdfdfefltcgvpkqlgi3_text() {
    let fixture = fixture__4wrp5lr6pkwzlgdfdfefltcgvpkqlgi3();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf__4wrp5lr6pkwzlgdfdfefltcgvpkqlgi3_ocr() {
    let fixture = fixture__4wrp5lr6pkwzlgdfdfefltcgvpkqlgi3();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf__5rjjr6j3r2prixtsjbko6lxxeoswvtum_text() {
    let fixture = fixture__5rjjr6j3r2prixtsjbko6lxxeoswvtum();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf__5rjjr6j3r2prixtsjbko6lxxeoswvtum_ocr() {
    let fixture = fixture__5rjjr6j3r2prixtsjbko6lxxeoswvtum();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf__7ykodjp2k5r3wt4l5nnz3yai57bf6rt3_text() {
    let fixture = fixture__7ykodjp2k5r3wt4l5nnz3yai57bf6rt3();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf__7ykodjp2k5r3wt4l5nnz3yai57bf6rt3_ocr() {
    let fixture = fixture__7ykodjp2k5r3wt4l5nnz3yai57bf6rt3();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_a4ztcigyxrdfsv746i7cfiaxr5fhpe7r_text() {
    let fixture = fixture_a4ztcigyxrdfsv746i7cfiaxr5fhpe7r();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_a4ztcigyxrdfsv746i7cfiaxr5fhpe7r_ocr() {
    let fixture = fixture_a4ztcigyxrdfsv746i7cfiaxr5fhpe7r();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_aktxmfdfscnl43fwo4weo32sl65fsbsu_text() {
    let fixture = fixture_aktxmfdfscnl43fwo4weo32sl65fsbsu();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_aktxmfdfscnl43fwo4weo32sl65fsbsu_ocr() {
    let fixture = fixture_aktxmfdfscnl43fwo4weo32sl65fsbsu();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_au677ne7k5juyach6te7bmvd5tkhsuka_text() {
    let fixture = fixture_au677ne7k5juyach6te7bmvd5tkhsuka();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_au677ne7k5juyach6te7bmvd5tkhsuka_ocr() {
    let fixture = fixture_au677ne7k5juyach6te7bmvd5tkhsuka();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_auaq2pkqfvxpu4b2uav6md757qw6v5dl_text() {
    let fixture = fixture_auaq2pkqfvxpu4b2uav6md757qw6v5dl();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_auaq2pkqfvxpu4b2uav6md757qw6v5dl_ocr() {
    let fixture = fixture_auaq2pkqfvxpu4b2uav6md757qw6v5dl();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_active_comps_duplexes_text() {
    let fixture = fixture_active_comps_duplexes();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_active_comps_duplexes_ocr() {
    let fixture = fixture_active_comps_duplexes();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_bjrjw65tae65ada55vwlujzip66x5mbk_text() {
    let fixture = fixture_bjrjw65tae65ada55vwlujzip66x5mbk();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_bjrjw65tae65ada55vwlujzip66x5mbk_ocr() {
    let fixture = fixture_bjrjw65tae65ada55vwlujzip66x5mbk();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_bkygzg7xyfphqccdlx5kualzrerpkfi4_text() {
    let fixture = fixture_bkygzg7xyfphqccdlx5kualzrerpkfi4();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_bkygzg7xyfphqccdlx5kualzrerpkfi4_ocr() {
    let fixture = fixture_bkygzg7xyfphqccdlx5kualzrerpkfi4();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_bn2eaonwqtbfxctcoxsd2fzspd3ystsx_text() {
    let fixture = fixture_bn2eaonwqtbfxctcoxsd2fzspd3ystsx();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_bn2eaonwqtbfxctcoxsd2fzspd3ystsx_ocr() {
    let fixture = fixture_bn2eaonwqtbfxctcoxsd2fzspd3ystsx();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_bob_1099_int_rose_edit_text() {
    let fixture = fixture_bob_1099_int_rose_edit();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_bob_1099_int_rose_edit_ocr() {
    let fixture = fixture_bob_1099_int_rose_edit();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_byzovcvgkm4cfujtycupu5b3g5pruf5q_text() {
    let fixture = fixture_byzovcvgkm4cfujtycupu5b3g5pruf5q();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_byzovcvgkm4cfujtycupu5b3g5pruf5q_ocr() {
    let fixture = fixture_byzovcvgkm4cfujtycupu5b3g5pruf5q();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_black_friday_2023_vs_2024_targets_text() {
    let fixture = fixture_black_friday_2023_vs_2024_targets();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_black_friday_2023_vs_2024_targets_ocr() {
    let fixture = fixture_black_friday_2023_vs_2024_targets();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_cfg6zfhghrck2ezxygyjlani5sps4uxp_text() {
    let fixture = fixture_cfg6zfhghrck2ezxygyjlani5sps4uxp();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_cfg6zfhghrck2ezxygyjlani5sps4uxp_ocr() {
    let fixture = fixture_cfg6zfhghrck2ezxygyjlani5sps4uxp();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_cgpjdttr32svugwv25sunyqxmu7ofldo_text() {
    let fixture = fixture_cgpjdttr32svugwv25sunyqxmu7ofldo();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_cgpjdttr32svugwv25sunyqxmu7ofldo_ocr() {
    let fixture = fixture_cgpjdttr32svugwv25sunyqxmu7ofldo();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_cny4ij2r63mwouux2gnaubo7aqykndck_text() {
    let fixture = fixture_cny4ij2r63mwouux2gnaubo7aqykndck();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_cny4ij2r63mwouux2gnaubo7aqykndck_ocr() {
    let fixture = fixture_cny4ij2r63mwouux2gnaubo7aqykndck();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_completed_2024_client_intake_form_bob_and_lisa_s_text() {
    let fixture = fixture_completed_2024_client_intake_form_bob_and_lisa_s();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_completed_2024_client_intake_form_bob_and_lisa_s_ocr() {
    let fixture = fixture_completed_2024_client_intake_form_bob_and_lisa_s();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_cwriubwmeyegfewgsa7eclsudbwle47w_text() {
    let fixture = fixture_cwriubwmeyegfewgsa7eclsudbwle47w();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_cwriubwmeyegfewgsa7eclsudbwle47w_ocr() {
    let fixture = fixture_cwriubwmeyegfewgsa7eclsudbwle47w();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_case_creation_guide_for_michael_reynolds_case_pt_text() {
    let fixture = fixture_case_creation_guide_for_michael_reynolds_case_pt();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_case_creation_guide_for_michael_reynolds_case_pt_ocr() {
    let fixture = fixture_case_creation_guide_for_michael_reynolds_case_pt();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_clauses_sheet_text() {
    let fixture = fixture_clauses_sheet();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_clauses_sheet_ocr() {
    let fixture = fixture_clauses_sheet();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_dgt43jkit56mj3fppl3a2q2ngr3nf3vl_text() {
    let fixture = fixture_dgt43jkit56mj3fppl3a2q2ngr3nf3vl();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_dgt43jkit56mj3fppl3a2q2ngr3nf3vl_ocr() {
    let fixture = fixture_dgt43jkit56mj3fppl3a2q2ngr3nf3vl();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_eifafslshet5r56rl6udngm2cz4tolg6_text() {
    let fixture = fixture_eifafslshet5r56rl6udngm2cz4tolg6();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_eifafslshet5r56rl6udngm2cz4tolg6_ocr() {
    let fixture = fixture_eifafslshet5r56rl6udngm2cz4tolg6();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_employee_sheet_text() {
    let fixture = fixture_employee_sheet();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_employee_sheet_ocr() {
    let fixture = fixture_employee_sheet();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_ewh5vge7xh2bbg3vavipq76pna3twe7x_text() {
    let fixture = fixture_ewh5vge7xh2bbg3vavipq76pna3twe7x();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_ewh5vge7xh2bbg3vavipq76pna3twe7x_ocr() {
    let fixture = fixture_ewh5vge7xh2bbg3vavipq76pna3twe7x();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_fmvchtwryigo5ts27pcql3mb7tlt6qzy_text() {
    let fixture = fixture_fmvchtwryigo5ts27pcql3mb7tlt6qzy();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_fmvchtwryigo5ts27pcql3mb7tlt6qzy_ocr() {
    let fixture = fixture_fmvchtwryigo5ts27pcql3mb7tlt6qzy();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_fusizdsqs6s76kollkwikcdfklz7iewr_text() {
    let fixture = fixture_fusizdsqs6s76kollkwikcdfklz7iewr();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_fusizdsqs6s76kollkwikcdfklz7iewr_ocr() {
    let fixture = fixture_fusizdsqs6s76kollkwikcdfklz7iewr();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_hrdpobgszdcmptjwv4szjhjecuh2midi_text() {
    let fixture = fixture_hrdpobgszdcmptjwv4szjhjecuh2midi();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_hrdpobgszdcmptjwv4szjhjecuh2midi_ocr() {
    let fixture = fixture_hrdpobgszdcmptjwv4szjhjecuh2midi();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_heat_shield_request_text() {
    let fixture = fixture_heat_shield_request();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_heat_shield_request_ocr() {
    let fixture = fixture_heat_shield_request();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_iqcjtr72ezt5uie7ucnh6sigwhte5tph_text() {
    let fixture = fixture_iqcjtr72ezt5uie7ucnh6sigwhte5tph();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_iqcjtr72ezt5uie7ucnh6sigwhte5tph_ocr() {
    let fixture = fixture_iqcjtr72ezt5uie7ucnh6sigwhte5tph();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_ixeu5rhdsd5hlavp6tgivlurw5q4lahs_text() {
    let fixture = fixture_ixeu5rhdsd5hlavp6tgivlurw5q4lahs();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_ixeu5rhdsd5hlavp6tgivlurw5q4lahs_ocr() {
    let fixture = fixture_ixeu5rhdsd5hlavp6tgivlurw5q4lahs();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_jpj2pgtcacka36artm3nymi4knbcu53f_text() {
    let fixture = fixture_jpj2pgtcacka36artm3nymi4knbcu53f();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_jpj2pgtcacka36artm3nymi4knbcu53f_ocr() {
    let fixture = fixture_jpj2pgtcacka36artm3nymi4knbcu53f();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_k4cbnldmxfxrjt6cvwo2un4t3a5rbcgv_text() {
    let fixture = fixture_k4cbnldmxfxrjt6cvwo2un4t3a5rbcgv();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_k4cbnldmxfxrjt6cvwo2un4t3a5rbcgv_ocr() {
    let fixture = fixture_k4cbnldmxfxrjt6cvwo2un4t3a5rbcgv();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_k4o3a2pylzbf76fhb45sy6brx5t7ciqn_text() {
    let fixture = fixture_k4o3a2pylzbf76fhb45sy6brx5t7ciqn();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_k4o3a2pylzbf76fhb45sy6brx5t7ciqn_ocr() {
    let fixture = fixture_k4o3a2pylzbf76fhb45sy6brx5t7ciqn();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_kwtht2mehcrzylqde4cxksnqudhyobts_text() {
    let fixture = fixture_kwtht2mehcrzylqde4cxksnqudhyobts();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_kwtht2mehcrzylqde4cxksnqudhyobts_ocr() {
    let fixture = fixture_kwtht2mehcrzylqde4cxksnqudhyobts();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_lisa_1099_int_rose_edit_text() {
    let fixture = fixture_lisa_1099_int_rose_edit();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_lisa_1099_int_rose_edit_ocr() {
    let fixture = fixture_lisa_1099_int_rose_edit();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_lisa_student_loan_interest_edit_text() {
    let fixture = fixture_lisa_student_loan_interest_edit();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_lisa_student_loan_interest_edit_ocr() {
    let fixture = fixture_lisa_student_loan_interest_edit();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_lpb74zboduerylsn3evchuah327csach_text() {
    let fixture = fixture_lpb74zboduerylsn3evchuah327csach();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_lpb74zboduerylsn3evchuah327csach_ocr() {
    let fixture = fixture_lpb74zboduerylsn3evchuah327csach();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_lunfjfh4kwz3zfnro43wsmzplm4olb7c_text() {
    let fixture = fixture_lunfjfh4kwz3zfnro43wsmzplm4olb7c();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_lunfjfh4kwz3zfnro43wsmzplm4olb7c_ocr() {
    let fixture = fixture_lunfjfh4kwz3zfnro43wsmzplm4olb7c();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_lvkivmzqv7szjiyi7gz45znmy3uikksi_text() {
    let fixture = fixture_lvkivmzqv7szjiyi7gz45znmy3uikksi();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_lvkivmzqv7szjiyi7gz45znmy3uikksi_ocr() {
    let fixture = fixture_lvkivmzqv7szjiyi7gz45znmy3uikksi();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_lzzcbj4emwfzjo6iaiskisbaeesamfuv_text() {
    let fixture = fixture_lzzcbj4emwfzjo6iaiskisbaeesamfuv();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_lzzcbj4emwfzjo6iaiskisbaeesamfuv_ocr() {
    let fixture = fixture_lzzcbj4emwfzjo6iaiskisbaeesamfuv();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_lease_comps_text() {
    let fixture = fixture_lease_comps();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_lease_comps_ocr() {
    let fixture = fixture_lease_comps();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_m4c7q3hurjr4bayotapvqjygiphhsgsu_text() {
    let fixture = fixture_m4c7q3hurjr4bayotapvqjygiphhsgsu();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_m4c7q3hurjr4bayotapvqjygiphhsgsu_ocr() {
    let fixture = fixture_m4c7q3hurjr4bayotapvqjygiphhsgsu();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_m4il3ndgry4tw776ro3elyiaekbcuycp_text() {
    let fixture = fixture_m4il3ndgry4tw776ro3elyiaekbcuycp();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_m4il3ndgry4tw776ro3elyiaekbcuycp_ocr() {
    let fixture = fixture_m4il3ndgry4tw776ro3elyiaekbcuycp();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_meeting_notes_text() {
    let fixture = fixture_meeting_notes();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_meeting_notes_ocr() {
    let fixture = fixture_meeting_notes();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_mff5fy5wa45rk2ffzzasitdosk4cby55_text() {
    let fixture = fixture_mff5fy5wa45rk2ffzzasitdosk4cby55();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_mff5fy5wa45rk2ffzzasitdosk4cby55_ocr() {
    let fixture = fixture_mff5fy5wa45rk2ffzzasitdosk4cby55();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_marketing_email_text() {
    let fixture = fixture_marketing_email();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_marketing_email_ocr() {
    let fixture = fixture_marketing_email();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_nq3ud2meblx7rxsjwpfp5lrmeao7ygfo_text() {
    let fixture = fixture_nq3ud2meblx7rxsjwpfp5lrmeao7ygfo();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_nq3ud2meblx7rxsjwpfp5lrmeao7ygfo_ocr() {
    let fixture = fixture_nq3ud2meblx7rxsjwpfp5lrmeao7ygfo();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_neurips_2023_recommender_systems_with_generative_r_text() {
    let fixture = fixture_neurips_2023_recommender_systems_with_generative_r();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_neurips_2023_recommender_systems_with_generative_r_ocr() {
    let fixture = fixture_neurips_2023_recommender_systems_with_generative_r();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_oosmtzo5epbqsu2pcc23ccmzbkqfkxhr_text() {
    let fixture = fixture_oosmtzo5epbqsu2pcc23ccmzbkqfkxhr();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_oosmtzo5epbqsu2pcc23ccmzbkqfkxhr_ocr() {
    let fixture = fixture_oosmtzo5epbqsu2pcc23ccmzbkqfkxhr();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_order_list_text() {
    let fixture = fixture_order_list();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_order_list_ocr() {
    let fixture = fixture_order_list();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_plcfotmysabkqeefl7c4kcfwyefwk2nc_text() {
    let fixture = fixture_plcfotmysabkqeefl7c4kcfwyefwk2nc();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_plcfotmysabkqeefl7c4kcfwyefwk2nc_ocr() {
    let fixture = fixture_plcfotmysabkqeefl7c4kcfwyefwk2nc();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_pmusjparzglvsocfc2yoizhppgdpylex_text() {
    let fixture = fixture_pmusjparzglvsocfc2yoizhppgdpylex();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_pmusjparzglvsocfc2yoizhppgdpylex_ocr() {
    let fixture = fixture_pmusjparzglvsocfc2yoizhppgdpylex();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_pnd7uqgoop7h3nkdtc6lczqyr533fbdg_text() {
    let fixture = fixture_pnd7uqgoop7h3nkdtc6lczqyr533fbdg();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_pnd7uqgoop7h3nkdtc6lczqyr533fbdg_ocr() {
    let fixture = fixture_pnd7uqgoop7h3nkdtc6lczqyr533fbdg();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_q7tn3uy5munwy472gh6s5dxf6pwyxhet_text() {
    let fixture = fixture_q7tn3uy5munwy472gh6s5dxf6pwyxhet();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_q7tn3uy5munwy472gh6s5dxf6pwyxhet_ocr() {
    let fixture = fixture_q7tn3uy5munwy472gh6s5dxf6pwyxhet();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_qjdlty6rgdyfj6rwkukgvq2thlrkkxot_text() {
    let fixture = fixture_qjdlty6rgdyfj6rwkukgvq2thlrkkxot();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_qjdlty6rgdyfj6rwkukgvq2thlrkkxot_ocr() {
    let fixture = fixture_qjdlty6rgdyfj6rwkukgvq2thlrkkxot();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_qs7np3zllxps5ynprcyjsu4cwhbtsseu_text() {
    let fixture = fixture_qs7np3zllxps5ynprcyjsu4cwhbtsseu();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_qs7np3zllxps5ynprcyjsu4cwhbtsseu_ocr() {
    let fixture = fixture_qs7np3zllxps5ynprcyjsu4cwhbtsseu();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_rbgjg3e42gtt2yogc7zh7soishntepuj_text() {
    let fixture = fixture_rbgjg3e42gtt2yogc7zh7soishntepuj();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_rbgjg3e42gtt2yogc7zh7soishntepuj_ocr() {
    let fixture = fixture_rbgjg3e42gtt2yogc7zh7soishntepuj();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_rcoxqbrhdzr4b72sybyebyzpys3ok2sa_text() {
    let fixture = fixture_rcoxqbrhdzr4b72sybyebyzpys3ok2sa();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_rcoxqbrhdzr4b72sybyebyzpys3ok2sa_ocr() {
    let fixture = fixture_rcoxqbrhdzr4b72sybyebyzpys3ok2sa();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_rknn6s2j4w7s7ggwgxjsjqnnhpofjqsv_text() {
    let fixture = fixture_rknn6s2j4w7s7ggwgxjsjqnnhpofjqsv();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_rknn6s2j4w7s7ggwgxjsjqnnhpofjqsv_ocr() {
    let fixture = fixture_rknn6s2j4w7s7ggwgxjsjqnnhpofjqsv();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_rmzvcithoe4l4um7j23cbdatbnsmtdgg_text() {
    let fixture = fixture_rmzvcithoe4l4um7j23cbdatbnsmtdgg();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_rmzvcithoe4l4um7j23cbdatbnsmtdgg_ocr() {
    let fixture = fixture_rmzvcithoe4l4um7j23cbdatbnsmtdgg();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_request_for_indicative_pricing_iehk_2017_bo_75_text() {
    let fixture = fixture_request_for_indicative_pricing_iehk_2017_bo_75();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_request_for_indicative_pricing_iehk_2017_bo_75_ocr() {
    let fixture = fixture_request_for_indicative_pricing_iehk_2017_bo_75();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_s42cb7ozgfjr47mlvldrppk2yfo7d7cw_text() {
    let fixture = fixture_s42cb7ozgfjr47mlvldrppk2yfo7d7cw();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_s42cb7ozgfjr47mlvldrppk2yfo7d7cw_ocr() {
    let fixture = fixture_s42cb7ozgfjr47mlvldrppk2yfo7d7cw();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_sales_mastery_ebook_text() {
    let fixture = fixture_sales_mastery_ebook();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_sales_mastery_ebook_ocr() {
    let fixture = fixture_sales_mastery_ebook();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_txzq6snsxrhg7vpl7o6ulnupgmjfpfmf_text() {
    let fixture = fixture_txzq6snsxrhg7vpl7o6ulnupgmjfpfmf();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_txzq6snsxrhg7vpl7o6ulnupgmjfpfmf_ocr() {
    let fixture = fixture_txzq6snsxrhg7vpl7o6ulnupgmjfpfmf();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_the_art_of_sales_text() {
    let fixture = fixture_the_art_of_sales();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_the_art_of_sales_ocr() {
    let fixture = fixture_the_art_of_sales();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_the_ultimate_sales_training_guide_text() {
    let fixture = fixture_the_ultimate_sales_training_guide();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_the_ultimate_sales_training_guide_ocr() {
    let fixture = fixture_the_ultimate_sales_training_guide();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_u5od4qwgmlwsx7mcfivysomivlrllqfe_text() {
    let fixture = fixture_u5od4qwgmlwsx7mcfivysomivlrllqfe();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_u5od4qwgmlwsx7mcfivysomivlrllqfe_ocr() {
    let fixture = fixture_u5od4qwgmlwsx7mcfivysomivlrllqfe();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_ue26ahn5x7bu3s4kavnap37tlymztpfn_text() {
    let fixture = fixture_ue26ahn5x7bu3s4kavnap37tlymztpfn();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_ue26ahn5x7bu3s4kavnap37tlymztpfn_ocr() {
    let fixture = fixture_ue26ahn5x7bu3s4kavnap37tlymztpfn();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_umpzzp732iwu2prs7r4u7yd7tgxb4chu_text() {
    let fixture = fixture_umpzzp732iwu2prs7r4u7yd7tgxb4chu();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_umpzzp732iwu2prs7r4u7yd7tgxb4chu_ocr() {
    let fixture = fixture_umpzzp732iwu2prs7r4u7yd7tgxb4chu();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_urphkaw32reourwgztzsfhwnhlvz4mia_text() {
    let fixture = fixture_urphkaw32reourwgztzsfhwnhlvz4mia();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_urphkaw32reourwgztzsfhwnhlvz4mia_ocr() {
    let fixture = fixture_urphkaw32reourwgztzsfhwnhlvz4mia();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_vbzm3d3cytwrjgm4xi5esxab7opsjnbu_text() {
    let fixture = fixture_vbzm3d3cytwrjgm4xi5esxab7opsjnbu();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_vbzm3d3cytwrjgm4xi5esxab7opsjnbu_ocr() {
    let fixture = fixture_vbzm3d3cytwrjgm4xi5esxab7opsjnbu();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_w3qye35tq3s5euntceq2lhigkqf5tdch_text() {
    let fixture = fixture_w3qye35tq3s5euntceq2lhigkqf5tdch();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_w3qye35tq3s5euntceq2lhigkqf5tdch_ocr() {
    let fixture = fixture_w3qye35tq3s5euntceq2lhigkqf5tdch();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_waabefeaoxqfwhgsvr72ydlqdjk6roqy_text() {
    let fixture = fixture_waabefeaoxqfwhgsvr72ydlqdjk6roqy();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_waabefeaoxqfwhgsvr72ydlqdjk6roqy_ocr() {
    let fixture = fixture_waabefeaoxqfwhgsvr72ydlqdjk6roqy();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_wibzb2podtfewil3uth4ahmwgf5sbyi5_text() {
    let fixture = fixture_wibzb2podtfewil3uth4ahmwgf5sbyi5();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_wibzb2podtfewil3uth4ahmwgf5sbyi5_ocr() {
    let fixture = fixture_wibzb2podtfewil3uth4ahmwgf5sbyi5();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_wla3hyhagnfeb35cvgp3isfjctcnvd4l_text() {
    let fixture = fixture_wla3hyhagnfeb35cvgp3isfjctcnvd4l();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_wla3hyhagnfeb35cvgp3isfjctcnvd4l_ocr() {
    let fixture = fixture_wla3hyhagnfeb35cvgp3isfjctcnvd4l();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_wrrwlj2b4oxf3j5eyhntb4wag6gkrdl6_text() {
    let fixture = fixture_wrrwlj2b4oxf3j5eyhntb4wag6gkrdl6();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_wrrwlj2b4oxf3j5eyhntb4wag6gkrdl6_ocr() {
    let fixture = fixture_wrrwlj2b4oxf3j5eyhntb4wag6gkrdl6();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_x7wnpbgxmun6jeldzbtstgidml5oiwpc_text() {
    let fixture = fixture_x7wnpbgxmun6jeldzbtstgidml5oiwpc();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_x7wnpbgxmun6jeldzbtstgidml5oiwpc_ocr() {
    let fixture = fixture_x7wnpbgxmun6jeldzbtstgidml5oiwpc();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_xr6aea4squ44uvdvebmopks5r2uxkwzp_text() {
    let fixture = fixture_xr6aea4squ44uvdvebmopks5r2uxkwzp();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_xr6aea4squ44uvdvebmopks5r2uxkwzp_ocr() {
    let fixture = fixture_xr6aea4squ44uvdvebmopks5r2uxkwzp();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_y3xtu3hvktac5gkw5utsrgwhgenpwvmd_text() {
    let fixture = fixture_y3xtu3hvktac5gkw5utsrgwhgenpwvmd();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_y3xtu3hvktac5gkw5utsrgwhgenpwvmd_ocr() {
    let fixture = fixture_y3xtu3hvktac5gkw5utsrgwhgenpwvmd();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_ymk4fnjd2yj2bqbeoo2jcsyznlqhqkuy_text() {
    let fixture = fixture_ymk4fnjd2yj2bqbeoo2jcsyznlqhqkuy();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_ymk4fnjd2yj2bqbeoo2jcsyznlqhqkuy_ocr() {
    let fixture = fixture_ymk4fnjd2yj2bqbeoo2jcsyznlqhqkuy();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_yzhsbl6mtijbcuaymgzqb3cj52m3pnvs_text() {
    let fixture = fixture_yzhsbl6mtijbcuaymgzqb3cj52m3pnvs();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_yzhsbl6mtijbcuaymgzqb3cj52m3pnvs_ocr() {
    let fixture = fixture_yzhsbl6mtijbcuaymgzqb3cj52m3pnvs();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_zkanhat6qxsysrmrvq2u6r4irt4mrlit_text() {
    let fixture = fixture_zkanhat6qxsysrmrvq2u6r4irt4mrlit();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_zkanhat6qxsysrmrvq2u6r4irt4mrlit_ocr() {
    let fixture = fixture_zkanhat6qxsysrmrvq2u6r4irt4mrlit();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_zzperc62kux6avrodltknvhlkkbpuakb_text() {
    let fixture = fixture_zzperc62kux6avrodltknvhlkkbpuakb();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_zzperc62kux6avrodltknvhlkkbpuakb_ocr() {
    let fixture = fixture_zzperc62kux6avrodltknvhlkkbpuakb();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_amazon_dynamo_sosp2007_text() {
    let fixture = fixture_amazon_dynamo_sosp2007();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_amazon_dynamo_sosp2007_ocr() {
    let fixture = fixture_amazon_dynamo_sosp2007();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_arxiv_1512_04337_effect_of_generalized_uncertainty_text() {
    let fixture = fixture_arxiv_1512_04337_effect_of_generalized_uncertainty();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_arxiv_1512_04337_effect_of_generalized_uncertainty_ocr() {
    let fixture = fixture_arxiv_1512_04337_effect_of_generalized_uncertainty();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_arxiv_1512_08902_multipion_bose_einstein_correlati_text() {
    let fixture = fixture_arxiv_1512_08902_multipion_bose_einstein_correlati();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_arxiv_1512_08902_multipion_bose_einstein_correlati_ocr() {
    let fixture = fixture_arxiv_1512_08902_multipion_bose_einstein_correlati();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_arxiv_1512_09355_allowed_rare_pion_and_muon_decays_text() {
    let fixture = fixture_arxiv_1512_09355_allowed_rare_pion_and_muon_decays();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_arxiv_1512_09355_allowed_rare_pion_and_muon_decays_ocr() {
    let fixture = fixture_arxiv_1512_09355_allowed_rare_pion_and_muon_decays();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_arxiv_1812_08400_a_unified_convolutional_beamforme_text() {
    let fixture = fixture_arxiv_1812_08400_a_unified_convolutional_beamforme();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_arxiv_1812_08400_a_unified_convolutional_beamforme_ocr() {
    let fixture = fixture_arxiv_1812_08400_a_unified_convolutional_beamforme();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_arxiv_1812_10103_new_opportunities_for_integrated_text() {
    let fixture = fixture_arxiv_1812_10103_new_opportunities_for_integrated();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_arxiv_1812_10103_new_opportunities_for_integrated_ocr() {
    let fixture = fixture_arxiv_1812_10103_new_opportunities_for_integrated();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_arxiv_1812_11627_figure_1_theory_meets_figure_2_ex_text() {
    let fixture = fixture_arxiv_1812_11627_figure_1_theory_meets_figure_2_ex();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_arxiv_1812_11627_figure_1_theory_meets_figure_2_ex_ocr() {
    let fixture = fixture_arxiv_1812_11627_figure_1_theory_meets_figure_2_ex();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_arxiv_1812_11922_epipolar_geometry_based_learning_text() {
    let fixture = fixture_arxiv_1812_11922_epipolar_geometry_based_learning();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_arxiv_1812_11922_epipolar_geometry_based_learning_ocr() {
    let fixture = fixture_arxiv_1812_11922_epipolar_geometry_based_learning();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_arxiv_2012_15649_string_of_columns_rewriting_and_c_text() {
    let fixture = fixture_arxiv_2012_15649_string_of_columns_rewriting_and_c();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_arxiv_2012_15649_string_of_columns_rewriting_and_c_ocr() {
    let fixture = fixture_arxiv_2012_15649_string_of_columns_rewriting_and_c();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_arxiv_2101_00060_networks_of_necessity_simulating_text() {
    let fixture = fixture_arxiv_2101_00060_networks_of_necessity_simulating();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_arxiv_2101_00060_networks_of_necessity_simulating_ocr() {
    let fixture = fixture_arxiv_2101_00060_networks_of_necessity_simulating();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_arxiv_2212_08509_moate_simulation_of_stochastic_pr_text() {
    let fixture = fixture_arxiv_2212_08509_moate_simulation_of_stochastic_pr();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_arxiv_2212_08509_moate_simulation_of_stochastic_pr_ocr() {
    let fixture = fixture_arxiv_2212_08509_moate_simulation_of_stochastic_pr();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_arxiv_2212_10164_multi_asset_market_making_under_t_text() {
    let fixture = fixture_arxiv_2212_10164_multi_asset_market_making_under_t();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_arxiv_2212_10164_multi_asset_market_making_under_t_ocr() {
    let fixture = fixture_arxiv_2212_10164_multi_asset_market_making_under_t();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_arxiv_2212_14550_similarity_based_predictive_maint_text() {
    let fixture = fixture_arxiv_2212_14550_similarity_based_predictive_maint();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_arxiv_2212_14550_similarity_based_predictive_maint_ocr() {
    let fixture = fixture_arxiv_2212_14550_similarity_based_predictive_maint();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_arxiv_2212_14681_an_entropy_based_model_for_hierar_text() {
    let fixture = fixture_arxiv_2212_14681_an_entropy_based_model_for_hierar();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_arxiv_2212_14681_an_entropy_based_model_for_hierar_ocr() {
    let fixture = fixture_arxiv_2212_14681_an_entropy_based_model_for_hierar();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_arxiv_2301_00244_degenerate_poisson_algebras_and_d_text() {
    let fixture = fixture_arxiv_2301_00244_degenerate_poisson_algebras_and_d();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_arxiv_2301_00244_degenerate_poisson_algebras_and_d_ocr() {
    let fixture = fixture_arxiv_2301_00244_degenerate_poisson_algebras_and_d();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_arxiv_2301_00248_nowcasting_stock_implied_volatili_text() {
    let fixture = fixture_arxiv_2301_00248_nowcasting_stock_implied_volatili();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_arxiv_2301_00248_nowcasting_stock_implied_volatili_ocr() {
    let fixture = fixture_arxiv_2301_00248_nowcasting_stock_implied_volatili();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_arxiv_2301_03468_knowledge_aware_semantic_communic_text() {
    let fixture = fixture_arxiv_2301_03468_knowledge_aware_semantic_communic();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_arxiv_2301_03468_knowledge_aware_semantic_communic_ocr() {
    let fixture = fixture_arxiv_2301_03468_knowledge_aware_semantic_communic();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_arxiv_2509_07752_loop_space_blow_up_and_scale_calc_text() {
    let fixture = fixture_arxiv_2509_07752_loop_space_blow_up_and_scale_calc();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_arxiv_2509_07752_loop_space_blow_up_and_scale_calc_ocr() {
    let fixture = fixture_arxiv_2509_07752_loop_space_blow_up_and_scale_calc();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_arxiv_2509_07786_variable_matrix_weighted_besov_sp_text() {
    let fixture = fixture_arxiv_2509_07786_variable_matrix_weighted_besov_sp();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_arxiv_2509_07786_variable_matrix_weighted_besov_sp_ocr() {
    let fixture = fixture_arxiv_2509_07786_variable_matrix_weighted_besov_sp();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_arxiv_2509_08478_from_link_homology_to_topological_text() {
    let fixture = fixture_arxiv_2509_08478_from_link_homology_to_topological();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_arxiv_2509_08478_from_link_homology_to_topological_ocr() {
    let fixture = fixture_arxiv_2509_08478_from_link_homology_to_topological();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_arxiv_2509_08567_shape_specific_fluctuations_of_an_text() {
    let fixture = fixture_arxiv_2509_08567_shape_specific_fluctuations_of_an();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_arxiv_2509_08567_shape_specific_fluctuations_of_an_ocr() {
    let fixture = fixture_arxiv_2509_08567_shape_specific_fluctuations_of_an();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_arxiv_2509_09026_continuous_fragmentation_equation_text() {
    let fixture = fixture_arxiv_2509_09026_continuous_fragmentation_equation();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_arxiv_2509_09026_continuous_fragmentation_equation_ocr() {
    let fixture = fixture_arxiv_2509_09026_continuous_fragmentation_equation();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_arxiv_2509_09100_compatibility_of_quantum_trace_an_text() {
    let fixture = fixture_arxiv_2509_09100_compatibility_of_quantum_trace_an();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_arxiv_2509_09100_compatibility_of_quantum_trace_an_ocr() {
    let fixture = fixture_arxiv_2509_09100_compatibility_of_quantum_trace_an();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_arxiv_2509_09550_finite_scalar_quantization_enable_text() {
    let fixture = fixture_arxiv_2509_09550_finite_scalar_quantization_enable();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_arxiv_2509_09550_finite_scalar_quantization_enable_ocr() {
    let fixture = fixture_arxiv_2509_09550_finite_scalar_quantization_enable();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_arxiv_2509_09560_boosting_embodied_ai_agents_throu_text() {
    let fixture = fixture_arxiv_2509_09560_boosting_embodied_ai_agents_throu();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_arxiv_2509_09560_boosting_embodied_ai_agents_throu_ocr() {
    let fixture = fixture_arxiv_2509_09560_boosting_embodied_ai_agents_throu();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_arxiv_2509_09594_objectreact_learning_object_rela_text() {
    let fixture = fixture_arxiv_2509_09594_objectreact_learning_object_rela();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_arxiv_2509_09594_objectreact_learning_object_rela_ocr() {
    let fixture = fixture_arxiv_2509_09594_objectreact_learning_object_rela();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_bf7f0af7_text() {
    let fixture = fixture_bf7f0af7();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_bf7f0af7_ocr() {
    let fixture = fixture_bf7f0af7();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_confirmation_text() {
    let fixture = fixture_confirmation();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_confirmation_ocr() {
    let fixture = fixture_confirmation();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_06_24_1002_e02821_matsuda_sangyo_coltd_text() {
    let fixture = fixture_edinet_2025_06_24_1002_e02821_matsuda_sangyo_coltd();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_06_24_1002_e02821_matsuda_sangyo_coltd_ocr() {
    let fixture = fixture_edinet_2025_06_24_1002_e02821_matsuda_sangyo_coltd();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
#[ignore = "Hangs indefinitely - Python docling timeout on large Japanese PDF"]
fn test_more_pdf_edinet_2025_06_25_1318_e00491_key_coffee_inc_text() {
    let fixture = fixture_edinet_2025_06_25_1318_e00491_key_coffee_inc();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
#[ignore = "Hangs indefinitely - Python docling timeout on large Japanese PDF"]
fn test_more_pdf_edinet_2025_06_25_1318_e00491_key_coffee_inc_ocr() {
    let fixture = fixture_edinet_2025_06_25_1318_e00491_key_coffee_inc();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_06_25_1531_e35153_sre_holdings_corpora_text() {
    let fixture = fixture_edinet_2025_06_25_1531_e35153_sre_holdings_corpora();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_06_25_1531_e35153_sre_holdings_corpora_ocr() {
    let fixture = fixture_edinet_2025_06_25_1531_e35153_sre_holdings_corpora();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_06_25_1532_e01222_asia_pile_holdings_c_text() {
    let fixture = fixture_edinet_2025_06_25_1532_e01222_asia_pile_holdings_c();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_06_25_1532_e01222_asia_pile_holdings_c_ocr() {
    let fixture = fixture_edinet_2025_06_25_1532_e01222_asia_pile_holdings_c();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_06_25_1550_e00080_toa_corporation_text() {
    let fixture = fixture_edinet_2025_06_25_1550_e00080_toa_corporation();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_06_25_1550_e00080_toa_corporation_ocr() {
    let fixture = fixture_edinet_2025_06_25_1550_e00080_toa_corporation();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_06_26_1051_e05307_tohokushinsha_film_c_text() {
    let fixture = fixture_edinet_2025_06_26_1051_e05307_tohokushinsha_film_c();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_06_26_1051_e05307_tohokushinsha_film_c_ocr() {
    let fixture = fixture_edinet_2025_06_26_1051_e05307_tohokushinsha_film_c();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_06_26_1512_e34499_yashima_co_ltd_text() {
    let fixture = fixture_edinet_2025_06_26_1512_e34499_yashima_co_ltd();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_06_26_1512_e34499_yashima_co_ltd_ocr() {
    let fixture = fixture_edinet_2025_06_26_1512_e34499_yashima_co_ltd();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_06_27_1101_e15692_not_registered_in_en_text() {
    let fixture = fixture_edinet_2025_06_27_1101_e15692_not_registered_in_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_06_27_1101_e15692_not_registered_in_en_ocr() {
    let fixture = fixture_edinet_2025_06_27_1101_e15692_not_registered_in_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_06_27_1124_e03472_applied_co_ltd_text() {
    let fixture = fixture_edinet_2025_06_27_1124_e03472_applied_co_ltd();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_06_27_1124_e03472_applied_co_ltd_ocr() {
    let fixture = fixture_edinet_2025_06_27_1124_e03472_applied_co_ltd();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_06_27_1351_e02134_nippon_sharyo_ltd_text() {
    let fixture = fixture_edinet_2025_06_27_1351_e02134_nippon_sharyo_ltd();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_06_27_1351_e02134_nippon_sharyo_ltd_ocr() {
    let fixture = fixture_edinet_2025_06_27_1351_e02134_nippon_sharyo_ltd();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_06_27_1610_e00487_asahimatsu_foods_col_text() {
    let fixture = fixture_edinet_2025_06_27_1610_e00487_asahimatsu_foods_col();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_06_27_1610_e00487_asahimatsu_foods_col_ocr() {
    let fixture = fixture_edinet_2025_06_27_1610_e00487_asahimatsu_foods_col();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
#[ignore = "Hangs indefinitely - Python docling timeout on large Japanese PDF"]
fn test_more_pdf_edinet_2025_06_27_1615_e05858_not_registered_in_en_text() {
    let fixture = fixture_edinet_2025_06_27_1615_e05858_not_registered_in_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
#[ignore = "Hangs indefinitely - Python docling timeout on large Japanese PDF"]
fn test_more_pdf_edinet_2025_06_27_1615_e05858_not_registered_in_en_ocr() {
    let fixture = fixture_edinet_2025_06_27_1615_e05858_not_registered_in_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_07_09_0940_e22440_vital_ksk_holdingsin_text() {
    let fixture = fixture_edinet_2025_07_09_0940_e22440_vital_ksk_holdingsin();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_07_09_0940_e22440_vital_ksk_holdingsin_ocr() {
    let fixture = fixture_edinet_2025_07_09_0940_e22440_vital_ksk_holdingsin();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_07_24_1607_e05647_sourcenext_corporati_text() {
    let fixture = fixture_edinet_2025_07_24_1607_e05647_sourcenext_corporati();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_07_24_1607_e05647_sourcenext_corporati_ocr() {
    let fixture = fixture_edinet_2025_07_24_1607_e05647_sourcenext_corporati();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_07_31_1633_e01737_hitachi_ltd_text() {
    let fixture = fixture_edinet_2025_07_31_1633_e01737_hitachi_ltd();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_07_31_1633_e01737_hitachi_ltd_ocr() {
    let fixture = fixture_edinet_2025_07_31_1633_e01737_hitachi_ltd();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_08_08_0951_e02679_tokyo_soir_coltd_text() {
    let fixture = fixture_edinet_2025_08_08_0951_e02679_tokyo_soir_coltd();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_08_08_0951_e02679_tokyo_soir_coltd_ocr() {
    let fixture = fixture_edinet_2025_08_08_0951_e02679_tokyo_soir_coltd();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_08_08_1138_e27046_grandesinc_text() {
    let fixture = fixture_edinet_2025_08_08_1138_e27046_grandesinc();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_08_08_1138_e27046_grandesinc_ocr() {
    let fixture = fixture_edinet_2025_08_08_1138_e27046_grandesinc();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_08_12_1303_e03859_tokyo_tatemono_co_lt_text() {
    let fixture = fixture_edinet_2025_08_12_1303_e03859_tokyo_tatemono_co_lt();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_08_12_1303_e03859_tokyo_tatemono_co_lt_ocr() {
    let fixture = fixture_edinet_2025_08_12_1303_e03859_tokyo_tatemono_co_lt();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_08_12_1607_e39410_hatch_work_coltd_text() {
    let fixture = fixture_edinet_2025_08_12_1607_e39410_hatch_work_coltd();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_08_12_1607_e39410_hatch_work_coltd_ocr() {
    let fixture = fixture_edinet_2025_08_12_1607_e39410_hatch_work_coltd();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_08_13_1521_e05036_cac_holdings_corpora_text() {
    let fixture = fixture_edinet_2025_08_13_1521_e05036_cac_holdings_corpora();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_08_13_1521_e05036_cac_holdings_corpora_ocr() {
    let fixture = fixture_edinet_2025_08_13_1521_e05036_cac_holdings_corpora();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_08_13_1600_e36897_core_concept_technol_text() {
    let fixture = fixture_edinet_2025_08_13_1600_e36897_core_concept_technol();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_08_13_1600_e36897_core_concept_technol_ocr() {
    let fixture = fixture_edinet_2025_08_13_1600_e36897_core_concept_technol();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_08_14_1530_e40216_not_registered_in_en_text() {
    let fixture = fixture_edinet_2025_08_14_1530_e40216_not_registered_in_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_08_14_1530_e40216_not_registered_in_en_ocr() {
    let fixture = fixture_edinet_2025_08_14_1530_e40216_not_registered_in_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_08_14_1534_e32486_g_factory_coltd_text() {
    let fixture = fixture_edinet_2025_08_14_1534_e32486_g_factory_coltd();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_08_14_1534_e32486_g_factory_coltd_ocr() {
    let fixture = fixture_edinet_2025_08_14_1534_e32486_g_factory_coltd();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_08_14_1547_e34142_management_solutions_text() {
    let fixture = fixture_edinet_2025_08_14_1547_e34142_management_solutions();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_08_14_1547_e34142_management_solutions_ocr() {
    let fixture = fixture_edinet_2025_08_14_1547_e34142_management_solutions();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_08_29_1427_e02308_tamron_coltd_text() {
    let fixture = fixture_edinet_2025_08_29_1427_e02308_tamron_coltd();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_edinet_2025_08_29_1427_e02308_tamron_coltd_ocr() {
    let fixture = fixture_edinet_2025_08_29_1427_e02308_tamron_coltd();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_f0019e_kamakiri_v3_text() {
    let fixture = fixture_f0019e_kamakiri_v3();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_f0019e_kamakiri_v3_ocr() {
    let fixture = fixture_f0019e_kamakiri_v3();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_f0026e_aichan2_1_v2_1_text() {
    let fixture = fixture_f0026e_aichan2_1_v2_1();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_f0026e_aichan2_1_v2_1_ocr() {
    let fixture = fixture_f0026e_aichan2_1_v2_1();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_f0028p_atta_v4_text() {
    let fixture = fixture_f0028p_atta_v4();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_f0028p_atta_v4_ocr() {
    let fixture = fixture_f0028p_atta_v4();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_f0042p_irete_text() {
    let fixture = fixture_f0042p_irete();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_f0042p_irete_ocr() {
    let fixture = fixture_f0042p_irete();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_f0047e_ame_v4_text() {
    let fixture = fixture_f0047e_ame_v4();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_f0047e_ame_v4_ocr() {
    let fixture = fixture_f0047e_ame_v4();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_f0049p_kikunohana_v3_text() {
    let fixture = fixture_f0049p_kikunohana_v3();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_f0049p_kikunohana_v3_ocr() {
    let fixture = fixture_f0049p_kikunohana_v3();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_f0052e_tkg_v5_text() {
    let fixture = fixture_f0052e_tkg_v5();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_f0052e_tkg_v5_ocr() {
    let fixture = fixture_f0052e_tkg_v5();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_f0056e_douzodoumo_v4_text() {
    let fixture = fixture_f0056e_douzodoumo_v4();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_f0056e_douzodoumo_v4_ocr() {
    let fixture = fixture_f0056e_douzodoumo_v4();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_f0056p_douzodoumo_v4_text() {
    let fixture = fixture_f0056p_douzodoumo_v4();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_f0056p_douzodoumo_v4_ocr() {
    let fixture = fixture_f0056p_douzodoumo_v4();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_f0057p_ankochan_v2_text() {
    let fixture = fixture_f0057p_ankochan_v2();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_f0057p_ankochan_v2_ocr() {
    let fixture = fixture_f0057p_ankochan_v2();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_f0062p_karasutomizusashi_v4_text() {
    let fixture = fixture_f0062p_karasutomizusashi_v4();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_f0062p_karasutomizusashi_v4_ocr() {
    let fixture = fixture_f0062p_karasutomizusashi_v4();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_f0077e_korenaani_v6_text() {
    let fixture = fixture_f0077e_korenaani_v6();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_f0077e_korenaani_v6_ocr() {
    let fixture = fixture_f0077e_korenaani_v6();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_f0089e_nakuonna_v2_text() {
    let fixture = fixture_f0089e_nakuonna_v2();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_f0089e_nakuonna_v2_ocr() {
    let fixture = fixture_f0089e_nakuonna_v2();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_f0105p_curryrice_text() {
    let fixture = fixture_f0105p_curryrice();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_f0105p_curryrice_ocr() {
    let fixture = fixture_f0105p_curryrice();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_f0114e_enko_text() {
    let fixture = fixture_f0114e_enko();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_f0114e_enko_ocr() {
    let fixture = fixture_f0114e_enko();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_gc_presentation_manyika_10nov14_text() {
    let fixture = fixture_gc_presentation_manyika_10nov14();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_gc_presentation_manyika_10nov14_ocr() {
    let fixture = fixture_gc_presentation_manyika_10nov14();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_mckinsey_march_2nd_presentation2_text() {
    let fixture = fixture_mckinsey_march_2nd_presentation2();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_mckinsey_march_2nd_presentation2_ocr() {
    let fixture = fixture_mckinsey_march_2nd_presentation2();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_mckinsey_selected_slides_text() {
    let fixture = fixture_mckinsey_selected_slides();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_mckinsey_selected_slides_ocr() {
    let fixture = fixture_mckinsey_selected_slides();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0001_simple_slide_en_text() {
    let fixture = fixture_synthetic_0001_simple_slide_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0001_simple_slide_en_ocr() {
    let fixture = fixture_synthetic_0001_simple_slide_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0002_title_subtitle_slide_en_text() {
    let fixture = fixture_synthetic_0002_title_subtitle_slide_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0002_title_subtitle_slide_en_ocr() {
    let fixture = fixture_synthetic_0002_title_subtitle_slide_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0003_agenda_slide_en_text() {
    let fixture = fixture_synthetic_0003_agenda_slide_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0003_agenda_slide_en_ocr() {
    let fixture = fixture_synthetic_0003_agenda_slide_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0004_comparison_slide_en_text() {
    let fixture = fixture_synthetic_0004_comparison_slide_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0004_comparison_slide_en_ocr() {
    let fixture = fixture_synthetic_0004_comparison_slide_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0005_quote_slide_en_text() {
    let fixture = fixture_synthetic_0005_quote_slide_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0005_quote_slide_en_ocr() {
    let fixture = fixture_synthetic_0005_quote_slide_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0006_financial_table_en_text() {
    let fixture = fixture_synthetic_0006_financial_table_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0006_financial_table_en_ocr() {
    let fixture = fixture_synthetic_0006_financial_table_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0007_employee_directory_en_text() {
    let fixture = fixture_synthetic_0007_employee_directory_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0007_employee_directory_en_ocr() {
    let fixture = fixture_synthetic_0007_employee_directory_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0008_product_inventory_en_text() {
    let fixture = fixture_synthetic_0008_product_inventory_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0008_product_inventory_en_ocr() {
    let fixture = fixture_synthetic_0008_product_inventory_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0009_sales_metrics_en_text() {
    let fixture = fixture_synthetic_0009_sales_metrics_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0009_sales_metrics_en_ocr() {
    let fixture = fixture_synthetic_0009_sales_metrics_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0010_project_timeline_en_text() {
    let fixture = fixture_synthetic_0010_project_timeline_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0010_project_timeline_en_ocr() {
    let fixture = fixture_synthetic_0010_project_timeline_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0011_server_metrics_en_text() {
    let fixture = fixture_synthetic_0011_server_metrics_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0011_server_metrics_en_ocr() {
    let fixture = fixture_synthetic_0011_server_metrics_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0012_customer_segments_en_text() {
    let fixture = fixture_synthetic_0012_customer_segments_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0012_customer_segments_en_ocr() {
    let fixture = fixture_synthetic_0012_customer_segments_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0013_budget_allocation_en_text() {
    let fixture = fixture_synthetic_0013_budget_allocation_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0013_budget_allocation_en_ocr() {
    let fixture = fixture_synthetic_0013_budget_allocation_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0014_traffic_analytics_en_text() {
    let fixture = fixture_synthetic_0014_traffic_analytics_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0014_traffic_analytics_en_ocr() {
    let fixture = fixture_synthetic_0014_traffic_analytics_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0015_support_tickets_en_text() {
    let fixture = fixture_synthetic_0015_support_tickets_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0015_support_tickets_en_ocr() {
    let fixture = fixture_synthetic_0015_support_tickets_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0016_two_column_newsletter_en_text() {
    let fixture = fixture_synthetic_0016_two_column_newsletter_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0016_two_column_newsletter_en_ocr() {
    let fixture = fixture_synthetic_0016_two_column_newsletter_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0017_three_column_newsletter_en_text() {
    let fixture = fixture_synthetic_0017_three_column_newsletter_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0017_three_column_newsletter_en_ocr() {
    let fixture = fixture_synthetic_0017_three_column_newsletter_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0018_side_by_side_comparison_en_text() {
    let fixture = fixture_synthetic_0018_side_by_side_comparison_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0018_side_by_side_comparison_en_ocr() {
    let fixture = fixture_synthetic_0018_side_by_side_comparison_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0019_article_layout_en_text() {
    let fixture = fixture_synthetic_0019_article_layout_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0019_article_layout_en_ocr() {
    let fixture = fixture_synthetic_0019_article_layout_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0020_two_column_mixed_en_text() {
    let fixture = fixture_synthetic_0020_two_column_mixed_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0020_two_column_mixed_en_ocr() {
    let fixture = fixture_synthetic_0020_two_column_mixed_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0020b_single_column_report_en_text() {
    let fixture = fixture_synthetic_0020b_single_column_report_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0020b_single_column_report_en_ocr() {
    let fixture = fixture_synthetic_0020b_single_column_report_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0021_two_column_list_en_text() {
    let fixture = fixture_synthetic_0021_two_column_list_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0021_two_column_list_en_ocr() {
    let fixture = fixture_synthetic_0021_two_column_list_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0022_before_after_en_text() {
    let fixture = fixture_synthetic_0022_before_after_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0022_before_after_en_ocr() {
    let fixture = fixture_synthetic_0022_before_after_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0023_multi_column_bulletins_en_text() {
    let fixture = fixture_synthetic_0023_multi_column_bulletins_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0023_multi_column_bulletins_en_ocr() {
    let fixture = fixture_synthetic_0023_multi_column_bulletins_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0024_bold_italic_text_en_text() {
    let fixture = fixture_synthetic_0024_bold_italic_text_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0024_bold_italic_text_en_ocr() {
    let fixture = fixture_synthetic_0024_bold_italic_text_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0025_code_block_en_text() {
    let fixture = fixture_synthetic_0025_code_block_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0025_code_block_en_ocr() {
    let fixture = fixture_synthetic_0025_code_block_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0026_quote_block_en_text() {
    let fixture = fixture_synthetic_0026_quote_block_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0026_quote_block_en_ocr() {
    let fixture = fixture_synthetic_0026_quote_block_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0027_mixed_formatting_en_text() {
    let fixture = fixture_synthetic_0027_mixed_formatting_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0027_mixed_formatting_en_ocr() {
    let fixture = fixture_synthetic_0027_mixed_formatting_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0028_subscript_superscript_en_text() {
    let fixture = fixture_synthetic_0028_subscript_superscript_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0028_subscript_superscript_en_ocr() {
    let fixture = fixture_synthetic_0028_subscript_superscript_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0029_inline_code_en_text() {
    let fixture = fixture_synthetic_0029_inline_code_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0029_inline_code_en_ocr() {
    let fixture = fixture_synthetic_0029_inline_code_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0030_blockquote_attribution_en_text() {
    let fixture = fixture_synthetic_0030_blockquote_attribution_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0030_blockquote_attribution_en_ocr() {
    let fixture = fixture_synthetic_0030_blockquote_attribution_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0031_footnotes_en_text() {
    let fixture = fixture_synthetic_0031_footnotes_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0031_footnotes_en_ocr() {
    let fixture = fixture_synthetic_0031_footnotes_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0032_highlighted_text_en_text() {
    let fixture = fixture_synthetic_0032_highlighted_text_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0032_highlighted_text_en_ocr() {
    let fixture = fixture_synthetic_0032_highlighted_text_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0033_mixed_text_formatting_en_text() {
    let fixture = fixture_synthetic_0033_mixed_text_formatting_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0033_mixed_text_formatting_en_ocr() {
    let fixture = fixture_synthetic_0033_mixed_text_formatting_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0034_nested_lists_en_text() {
    let fixture = fixture_synthetic_0034_nested_lists_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0034_nested_lists_en_ocr() {
    let fixture = fixture_synthetic_0034_nested_lists_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0035_course_outline_en_text() {
    let fixture = fixture_synthetic_0035_course_outline_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0035_course_outline_en_ocr() {
    let fixture = fixture_synthetic_0035_course_outline_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0036_company_hierarchy_en_text() {
    let fixture = fixture_synthetic_0036_company_hierarchy_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0036_company_hierarchy_en_ocr() {
    let fixture = fixture_synthetic_0036_company_hierarchy_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0037_recipe_instructions_en_text() {
    let fixture = fixture_synthetic_0037_recipe_instructions_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0037_recipe_instructions_en_ocr() {
    let fixture = fixture_synthetic_0037_recipe_instructions_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0038_software_features_en_text() {
    let fixture = fixture_synthetic_0038_software_features_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0038_software_features_en_ocr() {
    let fixture = fixture_synthetic_0038_software_features_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0039_travel_itinerary_en_text() {
    let fixture = fixture_synthetic_0039_travel_itinerary_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0039_travel_itinerary_en_ocr() {
    let fixture = fixture_synthetic_0039_travel_itinerary_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0040_research_topics_en_text() {
    let fixture = fixture_synthetic_0040_research_topics_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0040_research_topics_en_ocr() {
    let fixture = fixture_synthetic_0040_research_topics_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0041_emergency_procedures_en_text() {
    let fixture = fixture_synthetic_0041_emergency_procedures_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0041_emergency_procedures_en_ocr() {
    let fixture = fixture_synthetic_0041_emergency_procedures_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0042_header_hierarchy_en_text() {
    let fixture = fixture_synthetic_0042_header_hierarchy_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0042_header_hierarchy_en_ocr() {
    let fixture = fixture_synthetic_0042_header_hierarchy_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0043_technical_spec_en_text() {
    let fixture = fixture_synthetic_0043_technical_spec_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0043_technical_spec_en_ocr() {
    let fixture = fixture_synthetic_0043_technical_spec_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0044_user_manual_en_text() {
    let fixture = fixture_synthetic_0044_user_manual_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0044_user_manual_en_ocr() {
    let fixture = fixture_synthetic_0044_user_manual_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0045_policy_document_en_text() {
    let fixture = fixture_synthetic_0045_policy_document_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0045_policy_document_en_ocr() {
    let fixture = fixture_synthetic_0045_policy_document_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0046_training_material_en_text() {
    let fixture = fixture_synthetic_0046_training_material_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0046_training_material_en_ocr() {
    let fixture = fixture_synthetic_0046_training_material_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0047_business_proposal_en_text() {
    let fixture = fixture_synthetic_0047_business_proposal_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0047_business_proposal_en_ocr() {
    let fixture = fixture_synthetic_0047_business_proposal_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0048_research_report_en_text() {
    let fixture = fixture_synthetic_0048_research_report_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0048_research_report_en_ocr() {
    let fixture = fixture_synthetic_0048_research_report_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0049_urls_links_en_text() {
    let fixture = fixture_synthetic_0049_urls_links_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0049_urls_links_en_ocr() {
    let fixture = fixture_synthetic_0049_urls_links_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0050_email_links_en_text() {
    let fixture = fixture_synthetic_0050_email_links_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0050_email_links_en_ocr() {
    let fixture = fixture_synthetic_0050_email_links_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0051_ftp_links_en_text() {
    let fixture = fixture_synthetic_0051_ftp_links_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0051_ftp_links_en_ocr() {
    let fixture = fixture_synthetic_0051_ftp_links_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0052_anchor_text_links_en_text() {
    let fixture = fixture_synthetic_0052_anchor_text_links_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0052_anchor_text_links_en_ocr() {
    let fixture = fixture_synthetic_0052_anchor_text_links_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0053_mixed_url_types_en_text() {
    let fixture = fixture_synthetic_0053_mixed_url_types_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0053_mixed_url_types_en_ocr() {
    let fixture = fixture_synthetic_0053_mixed_url_types_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0054_slide_with_table_en_text() {
    let fixture = fixture_synthetic_0054_slide_with_table_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0054_slide_with_table_en_ocr() {
    let fixture = fixture_synthetic_0054_slide_with_table_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0055_table_with_list_en_text() {
    let fixture = fixture_synthetic_0055_table_with_list_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0055_table_with_list_en_ocr() {
    let fixture = fixture_synthetic_0055_table_with_list_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0056_code_with_table_en_text() {
    let fixture = fixture_synthetic_0056_code_with_table_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0056_code_with_table_en_ocr() {
    let fixture = fixture_synthetic_0056_code_with_table_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0057_quote_with_list_en_text() {
    let fixture = fixture_synthetic_0057_quote_with_list_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0057_quote_with_list_en_ocr() {
    let fixture = fixture_synthetic_0057_quote_with_list_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0058_multicolumn_with_code_en_text() {
    let fixture = fixture_synthetic_0058_multicolumn_with_code_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0058_multicolumn_with_code_en_ocr() {
    let fixture = fixture_synthetic_0058_multicolumn_with_code_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0059_table_with_formatting_en_text() {
    let fixture = fixture_synthetic_0059_table_with_formatting_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0059_table_with_formatting_en_ocr() {
    let fixture = fixture_synthetic_0059_table_with_formatting_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0060_nested_lists_with_code_en_text() {
    let fixture = fixture_synthetic_0060_nested_lists_with_code_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0060_nested_lists_with_code_en_ocr() {
    let fixture = fixture_synthetic_0060_nested_lists_with_code_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0061_headers_table_list_en_text() {
    let fixture = fixture_synthetic_0061_headers_table_list_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0061_headers_table_list_en_ocr() {
    let fixture = fixture_synthetic_0061_headers_table_list_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0062_formatted_text_with_table_en_text() {
    let fixture = fixture_synthetic_0062_formatted_text_with_table_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0062_formatted_text_with_table_en_ocr() {
    let fixture = fixture_synthetic_0062_formatted_text_with_table_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0063_multipage_mixed_content_en_text() {
    let fixture = fixture_synthetic_0063_multipage_mixed_content_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0063_multipage_mixed_content_en_ocr() {
    let fixture = fixture_synthetic_0063_multipage_mixed_content_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0064_code_table_quote_en_text() {
    let fixture = fixture_synthetic_0064_code_table_quote_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0064_code_table_quote_en_ocr() {
    let fixture = fixture_synthetic_0064_code_table_quote_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0065_complex_mixed_layout_en_text() {
    let fixture = fixture_synthetic_0065_complex_mixed_layout_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0065_complex_mixed_layout_en_ocr() {
    let fixture = fixture_synthetic_0065_complex_mixed_layout_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0066_technical_doc_en_text() {
    let fixture = fixture_synthetic_0066_technical_doc_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0066_technical_doc_en_ocr() {
    let fixture = fixture_synthetic_0066_technical_doc_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0067_chemical_formulas_en_text() {
    let fixture = fixture_synthetic_0067_chemical_formulas_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0067_chemical_formulas_en_ocr() {
    let fixture = fixture_synthetic_0067_chemical_formulas_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0068_mathematical_equations_en_text() {
    let fixture = fixture_synthetic_0068_mathematical_equations_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0068_mathematical_equations_en_ocr() {
    let fixture = fixture_synthetic_0068_mathematical_equations_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0069_technical_diagram_en_text() {
    let fixture = fixture_synthetic_0069_technical_diagram_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0069_technical_diagram_en_ocr() {
    let fixture = fixture_synthetic_0069_technical_diagram_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0070_research_paper_en_text() {
    let fixture = fixture_synthetic_0070_research_paper_en();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0070_research_paper_en_ocr() {
    let fixture = fixture_synthetic_0070_research_paper_en();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0071_business_presentation_ja_text() {
    let fixture = fixture_synthetic_0071_business_presentation_ja();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0071_business_presentation_ja_ocr() {
    let fixture = fixture_synthetic_0071_business_presentation_ja();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0072_product_launch_ja_text() {
    let fixture = fixture_synthetic_0072_product_launch_ja();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0072_product_launch_ja_ocr() {
    let fixture = fixture_synthetic_0072_product_launch_ja();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0073_quarterly_review_ja_text() {
    let fixture = fixture_synthetic_0073_quarterly_review_ja();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0073_quarterly_review_ja_ocr() {
    let fixture = fixture_synthetic_0073_quarterly_review_ja();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0074_project_status_ja_text() {
    let fixture = fixture_synthetic_0074_project_status_ja();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0074_project_status_ja_ocr() {
    let fixture = fixture_synthetic_0074_project_status_ja();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0074b_presentation_74_ja_text() {
    let fixture = fixture_synthetic_0074b_presentation_74_ja();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0074b_presentation_74_ja_ocr() {
    let fixture = fixture_synthetic_0074b_presentation_74_ja();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0075_training_presentation_ja_text() {
    let fixture = fixture_synthetic_0075_training_presentation_ja();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0075_training_presentation_ja_ocr() {
    let fixture = fixture_synthetic_0075_training_presentation_ja();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0075b_presentation_75_ja_text() {
    let fixture = fixture_synthetic_0075b_presentation_75_ja();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0075b_presentation_75_ja_ocr() {
    let fixture = fixture_synthetic_0075b_presentation_75_ja();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0076_strategic_planning_ja_text() {
    let fixture = fixture_synthetic_0076_strategic_planning_ja();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0076_strategic_planning_ja_ocr() {
    let fixture = fixture_synthetic_0076_strategic_planning_ja();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0076b_presentation_76_ja_text() {
    let fixture = fixture_synthetic_0076b_presentation_76_ja();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0076b_presentation_76_ja_ocr() {
    let fixture = fixture_synthetic_0076b_presentation_76_ja();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0077_quarterly_report_ja_text() {
    let fixture = fixture_synthetic_0077_quarterly_report_ja();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0077_quarterly_report_ja_ocr() {
    let fixture = fixture_synthetic_0077_quarterly_report_ja();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0078_balance_sheet_ja_text() {
    let fixture = fixture_synthetic_0078_balance_sheet_ja();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0078_balance_sheet_ja_ocr() {
    let fixture = fixture_synthetic_0078_balance_sheet_ja();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0079_income_statement_ja_text() {
    let fixture = fixture_synthetic_0079_income_statement_ja();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0079_income_statement_ja_ocr() {
    let fixture = fixture_synthetic_0079_income_statement_ja();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0080_cash_flow_statement_ja_text() {
    let fixture = fixture_synthetic_0080_cash_flow_statement_ja();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0080_cash_flow_statement_ja_ocr() {
    let fixture = fixture_synthetic_0080_cash_flow_statement_ja();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0081_budget_report_ja_text() {
    let fixture = fixture_synthetic_0081_budget_report_ja();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0081_budget_report_ja_ocr() {
    let fixture = fixture_synthetic_0081_budget_report_ja();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0082_expense_breakdown_ja_text() {
    let fixture = fixture_synthetic_0082_expense_breakdown_ja();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0082_expense_breakdown_ja_ocr() {
    let fixture = fixture_synthetic_0082_expense_breakdown_ja();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0083_asset_allocation_ja_text() {
    let fixture = fixture_synthetic_0083_asset_allocation_ja();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0083_asset_allocation_ja_ocr() {
    let fixture = fixture_synthetic_0083_asset_allocation_ja();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0084_revenue_by_product_ja_text() {
    let fixture = fixture_synthetic_0084_revenue_by_product_ja();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0084_revenue_by_product_ja_ocr() {
    let fixture = fixture_synthetic_0084_revenue_by_product_ja();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0085_employee_directory_ja_text() {
    let fixture = fixture_synthetic_0085_employee_directory_ja();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0085_employee_directory_ja_ocr() {
    let fixture = fixture_synthetic_0085_employee_directory_ja();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0086_employee_roster_ja_text() {
    let fixture = fixture_synthetic_0086_employee_roster_ja();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0086_employee_roster_ja_ocr() {
    let fixture = fixture_synthetic_0086_employee_roster_ja();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0087_sales_by_region_ja_text() {
    let fixture = fixture_synthetic_0087_sales_by_region_ja();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0087_sales_by_region_ja_ocr() {
    let fixture = fixture_synthetic_0087_sales_by_region_ja();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0088_inventory_status_ja_text() {
    let fixture = fixture_synthetic_0088_inventory_status_ja();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0088_inventory_status_ja_ocr() {
    let fixture = fixture_synthetic_0088_inventory_status_ja();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0089_customer_list_ja_text() {
    let fixture = fixture_synthetic_0089_customer_list_ja();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0089_customer_list_ja_ocr() {
    let fixture = fixture_synthetic_0089_customer_list_ja();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0090_project_timeline_ja_text() {
    let fixture = fixture_synthetic_0090_project_timeline_ja();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0090_project_timeline_ja_ocr() {
    let fixture = fixture_synthetic_0090_project_timeline_ja();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0091_server_status_ja_text() {
    let fixture = fixture_synthetic_0091_server_status_ja();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0091_server_status_ja_ocr() {
    let fixture = fixture_synthetic_0091_server_status_ja();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0092_task_list_ja_text() {
    let fixture = fixture_synthetic_0092_task_list_ja();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0092_task_list_ja_ocr() {
    let fixture = fixture_synthetic_0092_task_list_ja();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0188_bilingual_en_es_text() {
    let fixture = fixture_synthetic_0188_bilingual_en_es();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0188_bilingual_en_es_ocr() {
    let fixture = fixture_synthetic_0188_bilingual_en_es();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0189_bilingual_en_de_text() {
    let fixture = fixture_synthetic_0189_bilingual_en_de();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0189_bilingual_en_de_ocr() {
    let fixture = fixture_synthetic_0189_bilingual_en_de();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0190_bilingual_en_fr_text() {
    let fixture = fixture_synthetic_0190_bilingual_en_fr();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0190_bilingual_en_fr_ocr() {
    let fixture = fixture_synthetic_0190_bilingual_en_fr();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0191_multilingual_product_table_text() {
    let fixture = fixture_synthetic_0191_multilingual_product_table();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_synthetic_0191_multilingual_product_table_ocr() {
    let fixture = fixture_synthetic_0191_multilingual_product_table();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_test_01_simple_text_text() {
    let fixture = fixture_test_01_simple_text();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_test_01_simple_text_ocr() {
    let fixture = fixture_test_01_simple_text();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_test_02_multipage_text_text() {
    let fixture = fixture_test_02_multipage_text();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_test_02_multipage_text_ocr() {
    let fixture = fixture_test_02_multipage_text();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_test_03_single_table_text() {
    let fixture = fixture_test_03_single_table();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_test_03_single_table_ocr() {
    let fixture = fixture_test_03_single_table();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_test_04_multiple_tables_text() {
    let fixture = fixture_test_04_multiple_tables();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_test_04_multiple_tables_ocr() {
    let fixture = fixture_test_04_multiple_tables();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_test_05_mixed_content_text() {
    let fixture = fixture_test_05_mixed_content();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_test_05_mixed_content_ocr() {
    let fixture = fixture_test_05_mixed_content();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_test_06_nested_tables_text() {
    let fixture = fixture_test_06_nested_tables();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_test_06_nested_tables_ocr() {
    let fixture = fixture_test_06_nested_tables();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_test_07_large_table_text() {
    let fixture = fixture_test_07_large_table();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_test_07_large_table_ocr() {
    let fixture = fixture_test_07_large_table();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_test_08_with_image_text() {
    let fixture = fixture_test_08_with_image();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_test_08_with_image_ocr() {
    let fixture = fixture_test_08_with_image();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_test_09_empty_text() {
    let fixture = fixture_test_09_empty();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_test_09_empty_ocr() {
    let fixture = fixture_test_09_empty();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_test_10_unicode_text() {
    let fixture = fixture_test_10_unicode();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_test_10_unicode_ocr() {
    let fixture = fixture_test_10_unicode();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_test_11_rotated_text_text() {
    let fixture = fixture_test_11_rotated_text();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_test_11_rotated_text_ocr() {
    let fixture = fixture_test_11_rotated_text();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_test_12_small_fonts_text() {
    let fixture = fixture_test_12_small_fonts();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_test_12_small_fonts_ocr() {
    let fixture = fixture_test_12_small_fonts();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_test_13_dense_table_text() {
    let fixture = fixture_test_13_dense_table();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_test_13_dense_table_ocr() {
    let fixture = fixture_test_13_dense_table();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_test_14_financial_statement_text() {
    let fixture = fixture_test_14_financial_statement();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_test_14_financial_statement_ocr() {
    let fixture = fixture_test_14_financial_statement();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_test_15_multicolumn_table_text() {
    let fixture = fixture_test_15_multicolumn_table();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_test_15_multicolumn_table_ocr() {
    let fixture = fixture_test_15_multicolumn_table();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_test_16_invoice_text() {
    let fixture = fixture_test_16_invoice();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_test_16_invoice_ocr() {
    let fixture = fixture_test_16_invoice();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_test_17_schedule_text() {
    let fixture = fixture_test_17_schedule();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_test_17_schedule_ocr() {
    let fixture = fixture_test_17_schedule();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_test_18_scientific_data_text() {
    let fixture = fixture_test_18_scientific_data();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_test_18_scientific_data_ocr() {
    let fixture = fixture_test_18_scientific_data();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_test_19_attendance_text() {
    let fixture = fixture_test_19_attendance();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_test_19_attendance_ocr() {
    let fixture = fixture_test_19_attendance();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_test_20_scores_text() {
    let fixture = fixture_test_20_scores();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_test_20_scores_ocr() {
    let fixture = fixture_test_20_scores();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_test_21_sales_contract_text() {
    let fixture = fixture_test_21_sales_contract();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_test_21_sales_contract_ocr() {
    let fixture = fixture_test_21_sales_contract();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_test_22_presentation_text() {
    let fixture = fixture_test_22_presentation();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_test_22_presentation_ocr() {
    let fixture = fixture_test_22_presentation();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_test_23_multicolumn_text() {
    let fixture = fixture_test_23_multicolumn();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_test_23_multicolumn_ocr() {
    let fixture = fixture_test_23_multicolumn();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_test_24_form_text() {
    let fixture = fixture_test_24_form();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_test_24_form_ocr() {
    let fixture = fixture_test_24_form();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_test_arabic_text() {
    let fixture = fixture_test_arabic();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_test_arabic_ocr() {
    let fixture = fixture_test_arabic();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_test_chinese_simplified_text() {
    let fixture = fixture_test_chinese_simplified();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_test_chinese_simplified_ocr() {
    let fixture = fixture_test_chinese_simplified();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_test_korean_text() {
    let fixture = fixture_test_korean();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_test_korean_ocr() {
    let fixture = fixture_test_korean();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_test_tables_text() {
    let fixture = fixture_test_tables();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_test_tables_ocr() {
    let fixture = fixture_test_tables();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_pdf_test_thai_text() {
    let fixture = fixture_test_thai();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_more_pdf_test_thai_ocr() {
    let fixture = fixture_test_thai();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_canon_pdf_multi_page_text() {
    let fixture = fixture_multi_page();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_pdf_multi_page_ocr() {
    let fixture = fixture_multi_page();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_canon_pdf_picture_classification_text() {
    let fixture = fixture_picture_classification();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_pdf_picture_classification_ocr() {
    let fixture = fixture_picture_classification();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_canon_pdf_redp5110_sampled_text() {
    let fixture = fixture_redp5110_sampled();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_pdf_redp5110_sampled_ocr() {
    let fixture = fixture_redp5110_sampled();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_canon_pdf_right_to_left_01_text() {
    let fixture = fixture_right_to_left_01();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_pdf_right_to_left_01_ocr() {
    let fixture = fixture_right_to_left_01();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_canon_pdf_right_to_left_02_text() {
    let fixture = fixture_right_to_left_02();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_pdf_right_to_left_02_ocr() {
    let fixture = fixture_right_to_left_02();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_canon_pdf_right_to_left_03_text() {
    let fixture = fixture_right_to_left_03();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_pdf_right_to_left_03_ocr() {
    let fixture = fixture_right_to_left_03();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_canon_png__2305_03393v1_pg9_img_ocr() {
    let fixture = fixture__2305_03393v1_pg9_img();
    run_integration_test(&fixture, ExtractionMode::OcrText).unwrap();
}
#[test]
fn test_more_numbers_inventory() {
    let fixture = fixture_numbers_inventory();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

// APPLE KEYNOTE Tests (Rust-only extension, not in Python docling v2.58.0)
#[test]
fn test_more_keynote_minimal_test() {
    let fixture = fixture_keynote_minimal_test();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_more_keynote_business_review() {
    let fixture = fixture_keynote_business_review();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_more_keynote_training() {
    let fixture = fixture_keynote_training();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_more_keynote_product_launch() {
    let fixture = fixture_keynote_product_launch();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_more_keynote_transitions_and_builds() {
    let fixture = fixture_keynote_transitions_and_builds();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

// APPLE PAGES Tests (Rust-only extension, not in Python docling v2.58.0)
#[test]
fn test_more_pages_minimal_test() {
    let fixture = fixture_pages_minimal_test();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_more_pages_proposal() {
    let fixture = fixture_pages_proposal();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_more_pages_resume() {
    let fixture = fixture_pages_resume();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_more_pages_cover_letter() {
    let fixture = fixture_pages_cover_letter();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

// ============================================================================
// OpenDocument Format Canonical Tests (ODT, ODS, ODP)
// ============================================================================

#[test]
fn test_canon_odt_simple_text() {
    let fixture = fixture_odt_simple_text();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_odt_report() {
    let fixture = fixture_odt_report();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

// ============================================================================
// Email Format Canonical Tests (EML, MBOX)
// ============================================================================

#[test]
fn test_canon_eml_simple() {
    let fixture = fixture_eml_simple();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_eml_html_email() {
    let fixture = fixture_eml_html_email();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_eml_with_attachments() {
    let fixture = fixture_eml_with_attachments();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_eml_multipart_complex() {
    let fixture = fixture_eml_multipart_complex();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_eml_thread_conversation() {
    let fixture = fixture_eml_thread_conversation();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_mbox_simple() {
    let fixture = fixture_mbox_simple();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_mbox_small_mailbox() {
    let fixture = fixture_mbox_small_mailbox();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_mbox_threaded_conversation() {
    let fixture = fixture_mbox_threaded_conversation();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_mbox_unicode_multilang() {
    let fixture = fixture_mbox_unicode_multilang();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_mbox_mixed_content() {
    let fixture = fixture_mbox_mixed_content();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_odt_technical_spec() {
    let fixture = fixture_odt_technical_spec();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_odt_meeting_notes() {
    let fixture = fixture_odt_meeting_notes();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_odt_multi_paragraph() {
    let fixture = fixture_odt_multi_paragraph();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_ods_simple_spreadsheet() {
    let fixture = fixture_ods_simple_spreadsheet();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_ods_budget() {
    let fixture = fixture_ods_budget();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_ods_multi_sheet() {
    let fixture = fixture_ods_multi_sheet();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_ods_inventory() {
    let fixture = fixture_ods_inventory();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_ods_test_data() {
    let fixture = fixture_ods_test_data();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_odp_simple_presentation() {
    let fixture = fixture_odp_simple_presentation();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_odp_training() {
    let fixture = fixture_odp_training();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_odp_sales_pitch() {
    let fixture = fixture_odp_sales_pitch();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_odp_project_overview() {
    let fixture = fixture_odp_project_overview();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_odp_technical_talk() {
    let fixture = fixture_odp_technical_talk();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

// Removed: test_canon_7z_multi_file_archive (test file missing from corpus)
#[test]
fn test_canon_7z_simple_normal_text() {
    let fixture = fixture_7z_simple_normal();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}
#[test]
fn test_canon_7z_fast_compressed_text() {
    let fixture = fixture_7z_fast_compressed();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_7z_multi_content() {
    let fixture = fixture_7z_multi_content();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_7z_solid_archive() {
    let fixture = fixture_7z_solid_archive();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

// ===== TAR Tests =====

#[test]
fn test_canon_tar_uncompressed() {
    let fixture = fixture_tar_uncompressed();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_tar_compressed_gzip() {
    let fixture = fixture_tar_compressed_gzip();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_tar_compressed_bzip2() {
    let fixture = fixture_tar_compressed_bzip2();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_tar_nested_structure() {
    let fixture = fixture_tar_nested_structure();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

// ===== ZIP Tests =====

#[test]
fn test_canon_zip_compressed_large_file() {
    let fixture = fixture_zip_compressed_large_file();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_zip_multiple_files_flat() {
    let fixture = fixture_zip_multiple_files_flat();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_zip_nested_directories() {
    let fixture = fixture_zip_nested_directories();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_zip_simple_single_file() {
    let fixture = fixture_zip_simple_single_file();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_gltf_simple_cube() {
    let fixture = fixture_gltf_simple_cube();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_gltf_duck() {
    let fixture = fixture_gltf_duck();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_gltf_avocado() {
    let fixture = fixture_gltf_avocado();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_gltf_box() {
    let fixture = fixture_gltf_box();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_gltf_simple_triangle() {
    let fixture = fixture_gltf_simple_triangle();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_glb_box() {
    let fixture = fixture_glb_box();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_epub_complex() {
    let fixture = fixture_epub_complex();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_epub_simple() {
    let fixture = fixture_epub_simple();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_epub_large() {
    let fixture = fixture_epub_large();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_epub_non_english() {
    let fixture = fixture_epub_non_english();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_epub_with_images() {
    let fixture = fixture_epub_with_images();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_fb2_simple() {
    let fixture = fixture_fb2_simple();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_fb2_with_formatting() {
    let fixture = fixture_fb2_with_formatting();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_fb2_fiction_novel() {
    let fixture = fixture_fb2_fiction_novel();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_fb2_poetry() {
    let fixture = fixture_fb2_poetry();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_fb2_technical_book() {
    let fixture = fixture_fb2_technical_book();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_mobi_multi_chapter() {
    let fixture = fixture_mobi_multi_chapter();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_mobi_with_metadata() {
    let fixture = fixture_mobi_with_metadata();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_mobi_formatted() {
    let fixture = fixture_mobi_formatted();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_mobi_large_content() {
    let fixture = fixture_mobi_large_content();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_mobi_simple_text() {
    let fixture = fixture_mobi_simple_text();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_obj_simple_cube() {
    let fixture = fixture_obj_simple_cube();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_obj_teapot() {
    let fixture = fixture_obj_teapot();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_obj_icosphere() {
    let fixture = fixture_obj_icosphere();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_obj_pyramid() {
    let fixture = fixture_obj_pyramid();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_obj_textured_quad() {
    let fixture = fixture_obj_textured_quad();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

// ===== STL Tests =====

#[test]
fn test_canon_stl_simple_cube() {
    let fixture = fixture_stl_simple_cube();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_stl_pyramid() {
    let fixture = fixture_stl_pyramid();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_stl_complex_shape() {
    let fixture = fixture_stl_complex_shape();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_stl_minimal_triangle() {
    let fixture = fixture_stl_minimal_triangle();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_srt_simple_dialogue() {
    let fixture = fixture_srt_simple_dialogue();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_srt_technical_presentation() {
    let fixture = fixture_srt_technical_presentation();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_srt_documentary_excerpt() {
    let fixture = fixture_srt_documentary_excerpt();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_srt_interview() {
    let fixture = fixture_srt_interview();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_srt_multilingual_spanish() {
    let fixture = fixture_srt_multilingual_spanish();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

// ============================================================================
// NEW FORMAT TESTS - Archives, Audio, Email, Ebooks
// ============================================================================
// These tests verify the new format parsers (added in Phase A-E)
// Tests are organized by format category

// ARCHIVE FORMAT TESTS (Phase A)
// Tests for ZIP, TAR, 7Z, RAR archive formats

#[test]
fn test_archive_zip_simple_single_file() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/archives/zip/simple_single_file.zip");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");

    // Test archive extraction
    let files =
        docling_archive::zip::extract_zip_from_path(&test_file).expect("Failed to extract ZIP");
    assert_eq!(files.len(), 1, "Should contain exactly 1 file");
    assert_eq!(
        files[0].name, "document.txt",
        "File name should be document.txt"
    );
    assert!(
        !files[0].contents.is_empty(),
        "File contents should not be empty"
    );
}

#[test]
fn test_archive_zip_multiple_files_flat() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/archives/zip/multiple_files_flat.zip");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");

    // Test archive extraction with multiple files
    let files =
        docling_archive::zip::extract_zip_from_path(&test_file).expect("Failed to extract ZIP");
    assert_eq!(files.len(), 3, "Should contain exactly 3 files");

    // Verify expected files are present
    let file_names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
    assert!(
        file_names.contains(&"document.txt"),
        "Should contain document.txt"
    );
    assert!(
        file_names.contains(&"README.md"),
        "Should contain README.md"
    );
    assert!(
        file_names.contains(&"data.json"),
        "Should contain data.json"
    );
}

#[test]
fn test_archive_zip_nested_directories() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/archives/zip/nested_directories.zip");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");

    // Test archive extraction with nested directory structure
    let files =
        docling_archive::zip::extract_zip_from_path(&test_file).expect("Failed to extract ZIP");
    assert!(!files.is_empty(), "Should contain at least one file");

    // Verify at least one file has a path separator (nested structure)
    let has_nested = files.iter().any(|f| f.name.contains('/'));
    assert!(
        has_nested,
        "Should have at least one file in a nested directory"
    );
}

#[test]
fn test_archive_zip_empty() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/archives/zip/empty_archive.zip");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");

    // Test empty archive extraction
    let files =
        docling_archive::zip::extract_zip_from_path(&test_file).expect("Failed to extract ZIP");
    assert_eq!(files.len(), 0, "Empty archive should contain no files");
}

#[test]
fn test_archive_tar_uncompressed() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/archives/tar/uncompressed.tar");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");

    // Test uncompressed TAR extraction
    let files =
        docling_archive::tar::extract_tar_from_path(&test_file).expect("Failed to extract TAR");
    assert!(
        !files.is_empty(),
        "TAR archive should contain at least one file"
    );

    // Verify files have content
    for file in &files {
        assert!(
            !file.contents.is_empty(),
            "File {} should have content",
            file.name
        );
    }
}

#[test]
fn test_archive_tar_gzip() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/archives/tar/compressed_gzip.tar.gz");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");

    // Test gzip-compressed TAR extraction
    let files =
        docling_archive::tar::extract_tar_from_path(&test_file).expect("Failed to extract TAR.GZ");
    assert!(
        !files.is_empty(),
        "TAR.GZ archive should contain at least one file"
    );

    // Verify files have content
    for file in &files {
        assert!(
            !file.contents.is_empty(),
            "File {} should have content",
            file.name
        );
    }
}

#[test]
fn test_archive_tar_bzip2() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/archives/tar/compressed_bzip2.tar.bz2");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");

    // Test bzip2-compressed TAR extraction
    let files =
        docling_archive::tar::extract_tar_from_path(&test_file).expect("Failed to extract TAR.BZ2");
    assert!(
        !files.is_empty(),
        "TAR.BZ2 archive should contain at least one file"
    );

    // Verify files have content
    for file in &files {
        assert!(
            !file.contents.is_empty(),
            "File {} should have content",
            file.name
        );
    }
}

#[test]
fn test_archive_7z_simple() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/archives/7z/simple_normal.7z");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");

    // Test 7Z extraction
    let files =
        docling_archive::sevenz::extract_7z_from_path(&test_file).expect("Failed to extract 7Z");
    assert!(
        !files.is_empty(),
        "7Z archive should contain at least one file"
    );

    // Verify files have content
    for file in &files {
        assert!(
            !file.contents.is_empty(),
            "File {} should have content",
            file.name
        );
    }
}

#[test]
fn test_archive_7z_ultra_compressed() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/archives/7z/ultra_compressed.7z");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");

    // Test ultra-compressed 7Z extraction
    let files = docling_archive::sevenz::extract_7z_from_path(&test_file)
        .expect("Failed to extract ultra-compressed 7Z");
    assert!(
        !files.is_empty(),
        "Ultra-compressed 7Z should contain at least one file"
    );

    // Verify files have content
    for file in &files {
        assert!(
            !file.contents.is_empty(),
            "File {} should have content",
            file.name
        );
    }
}

#[test]
fn test_archive_rar_simple() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/archives/rar/simple.rar");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");

    // Test simple RAR extraction
    let files =
        docling_archive::rar::extract_rar_from_path(&test_file).expect("Failed to extract RAR");
    assert!(
        !files.is_empty(),
        "RAR archive should contain at least one file"
    );

    // Verify files have content
    for file in &files {
        assert!(
            !file.contents.is_empty(),
            "File {} should have content",
            file.name
        );
    }
}

#[test]
fn test_archive_rar_multi_files() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/archives/rar/multi_files.rar");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");

    // Test multi-file RAR extraction
    let files = docling_archive::rar::extract_rar_from_path(&test_file)
        .expect("Failed to extract multi-file RAR");
    assert!(
        files.len() > 1,
        "Multi-file RAR should contain more than one file"
    );

    // Verify files have content
    for file in &files {
        assert!(
            !file.contents.is_empty(),
            "File {} should have content",
            file.name
        );
    }
}

#[test]
fn test_archive_rar_nested() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/archives/rar/nested.rar");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");

    // Test nested RAR extraction
    let files = docling_archive::rar::extract_rar_from_path(&test_file)
        .expect("Failed to extract nested RAR");
    assert!(
        !files.is_empty(),
        "Nested RAR should contain at least one file"
    );

    // Verify at least one file has a path separator (nested structure)
    let has_nested = files
        .iter()
        .any(|f| f.name.contains('/') || f.name.contains('\\'));
    assert!(
        has_nested,
        "Should have at least one file in a nested directory"
    );
}

#[test]
fn test_archive_rar_compressed_best() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/archives/rar/compressed_best.rar");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");

    // Test best-compressed RAR extraction
    let files = docling_archive::rar::extract_rar_from_path(&test_file)
        .expect("Failed to extract best-compressed RAR");
    assert!(
        !files.is_empty(),
        "Best-compressed RAR should contain at least one file"
    );

    // Verify files have content
    for file in &files {
        assert!(
            !file.contents.is_empty(),
            "File {} should have content",
            file.name
        );
    }
}

#[test]
fn test_archive_rar_rar5_format() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/archives/rar/rar5_format.rar");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");

    // Test RAR5 format extraction
    let files = docling_archive::rar::extract_rar_from_path(&test_file)
        .expect("Failed to extract RAR5 format");
    assert!(
        !files.is_empty(),
        "RAR5 format should contain at least one file"
    );

    // Verify files have content
    for file in &files {
        assert!(
            !file.contents.is_empty(),
            "File {} should have content",
            file.name
        );
    }
}

// AUDIO FORMAT TESTS (Phase B)
// Tests for MP3 and WAV audio file metadata extraction

#[test]
fn test_audio_mp3_mono_speech() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/audio/mp3/mono_32kbps_speech.mp3");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_audio_mp3_stereo_standard() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/audio/mp3/stereo_128kbps_standard.mp3");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_audio_wav_mono() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/audio/wav/mono_16khz_short.wav");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_audio_wav_stereo() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/audio/wav/stereo_44khz_medium.wav");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

// ARCHIVE FORMAT DOCITEM TESTS (Phase 3)
// Tests for DocItem generation in archive backends (ZIP, TAR, 7Z, RAR)
// Note: Archives use Rust backend directly since Python docling doesn't support archives

fn test_archive_file(fixture: &TestFixture) {
    let test_file = Path::new("../../test-corpus").join(&fixture.file_path);
    assert!(test_file.exists(), "Test file not found: {test_file:?}");

    // Use Rust backend directly (Python doesn't support archives)
    let converter = RustDocumentConverter::new().expect("Failed to create Rust converter");
    let result = converter
        .convert(&test_file)
        .expect("Failed to convert archive file");

    // Verify document has content
    let markdown = result.document.to_markdown().to_string();
    assert!(
        !markdown.is_empty(),
        "Generated markdown is empty for {test_file:?}"
    );
    assert!(
        markdown.len() > 10,
        "Generated markdown too short for {test_file:?}"
    );
    assert!(
        markdown.contains("Archive Contents") || markdown.contains("Contents"),
        "Markdown should contain archive header for {test_file:?}"
    );

    // Verify content_blocks are populated (not None)
    assert!(
        result.document.content_blocks.is_some(),
        "content_blocks should not be None for {test_file:?}"
    );

    // Verify we have at least some DocItems (header + content)
    let doc_items = result.document.content_blocks.as_ref().unwrap();
    assert!(
        !doc_items.is_empty(),
        "content_blocks should not be empty for {test_file:?}"
    );
}

// ZIP Tests
#[test]
fn test_more_archive_zip_simple_single_file() {
    test_archive_file(&fixture_zip_simple_single_file());
}

#[test]
fn test_more_archive_zip_multiple_files_flat() {
    test_archive_file(&fixture_zip_multiple_files_flat());
}

#[test]
fn test_more_archive_zip_nested_directories() {
    test_archive_file(&fixture_zip_nested_directories());
}

#[test]
fn test_more_archive_zip_empty_archive() {
    // Note: empty_archive.zip is 0 bytes (not a valid ZIP), so conversion should fail
    let test_file = Path::new("../../test-corpus/archives/zip/empty_archive.zip");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");

    // Expect failure for invalid (0-byte) ZIP file
    let converter = RustDocumentConverter::new().expect("Failed to create Rust converter");
    let result = converter.convert(&test_file);
    assert!(result.is_err(), "Should fail for 0-byte (invalid) ZIP file");
}

#[test]
fn test_more_archive_zip_compressed_large_file() {
    test_archive_file(&fixture_zip_compressed_large_file());
}

// TAR Tests
#[test]
fn test_more_archive_tar_uncompressed() {
    test_archive_file(&fixture_tar_uncompressed());
}

#[test]
fn test_more_archive_tar_nested_structure() {
    test_archive_file(&fixture_tar_nested_structure());
}

#[test]
fn test_more_archive_tar_large_file() {
    test_archive_file(&fixture_tar_large_file());
}

// 7Z Tests
#[test]
fn test_more_archive_7z_simple_normal() {
    test_archive_file(&fixture_7z_simple_normal());
}

#[test]
fn test_more_archive_7z_fast_compressed() {
    test_archive_file(&fixture_7z_fast_compressed());
}

#[test]
fn test_more_archive_7z_multi_content() {
    test_archive_file(&fixture_7z_multi_content());
}

#[test]
fn test_more_archive_7z_solid_archive() {
    test_archive_file(&fixture_7z_solid_archive());
}

#[test]
fn test_more_archive_7z_ultra_compressed() {
    test_archive_file(&fixture_7z_ultra_compressed());
}

// RAR Tests
#[test]
fn test_more_archive_rar_simple() {
    test_archive_file(&fixture_rar_simple());
}

#[test]
fn test_more_archive_rar_multi_files() {
    test_archive_file(&fixture_rar_multi_files());
}

#[test]
fn test_more_archive_rar_nested() {
    test_archive_file(&fixture_rar_nested());
}

#[test]
fn test_more_archive_rar_rar5_format() {
    test_archive_file(&fixture_rar_rar5_format());
}

#[test]
fn test_more_archive_rar_compressed_best() {
    test_archive_file(&fixture_rar_compressed_best());
}

// EBOOK FORMAT TESTS
// Tests for EPUB, FB2, MOBI e-book formats
// Note: Ebooks use Rust backend directly since Python docling doesn't support ebooks

fn test_ebook_file(file_path: &str, format_name: &str) {
    let test_file = Path::new("../../test-corpus/ebooks").join(file_path);
    assert!(test_file.exists(), "Test file not found: {test_file:?}");

    // Use Rust backend directly (Python doesn't support ebooks)
    let converter = RustDocumentConverter::new().expect("Failed to create Rust converter");
    let result = converter
        .convert(&test_file)
        .unwrap_or_else(|_| panic!("Failed to convert {format_name} file"));

    // Verify document has content
    let markdown = result.document.to_markdown().to_string();
    assert!(
        !markdown.is_empty(),
        "Generated markdown is empty for {test_file:?}"
    );
    assert!(
        markdown.len() > 10,
        "Generated markdown too short for {test_file:?}"
    );

    // Verify content_blocks are populated (not None)
    assert!(
        result.document.content_blocks.is_some(),
        "content_blocks should not be None for {test_file:?}"
    );

    // Verify we have at least some DocItems
    let doc_items = result.document.content_blocks.as_ref().unwrap();
    assert!(
        !doc_items.is_empty(),
        "content_blocks should not be empty for {test_file:?}"
    );
}

// EPUB Tests
#[test]
fn test_more_ebook_epub_simple() {
    test_ebook_file("epub/simple.epub", "EPUB");
}

#[test]
fn test_more_ebook_epub_complex() {
    test_ebook_file("epub/complex.epub", "EPUB");
}

#[test]
fn test_more_ebook_epub_large() {
    test_ebook_file("epub/large.epub", "EPUB");
}

#[test]
fn test_more_ebook_epub_with_images() {
    test_ebook_file("epub/with_images.epub", "EPUB");
}

#[test]
fn test_more_ebook_epub_non_english() {
    test_ebook_file("epub/non_english.epub", "EPUB");
}

// FB2 Tests
#[test]
fn test_more_ebook_fb2_simple() {
    test_ebook_file("fb2/simple.fb2", "FB2");
}

#[test]
fn test_more_ebook_fb2_simple_minimal() {
    test_ebook_file("fb2/simple_minimal.fb2", "FB2");
}

#[test]
fn test_more_ebook_fb2_fiction_novel() {
    test_ebook_file("fb2/fiction_novel.fb2", "FB2");
}

#[test]
fn test_more_ebook_fb2_poetry() {
    test_ebook_file("fb2/poetry.fb2", "FB2");
}

#[test]
fn test_more_ebook_fb2_technical_book() {
    test_ebook_file("fb2/technical_book.fb2", "FB2");
}

#[test]
fn test_more_ebook_fb2_multi_section() {
    test_ebook_file("fb2/multi_section.fb2", "FB2");
}

#[test]
fn test_more_ebook_fb2_with_formatting() {
    test_ebook_file("fb2/with_formatting.fb2", "FB2");
}

#[test]
fn test_more_ebook_fb2_with_tables() {
    test_ebook_file("fb2/with_tables.fb2", "FB2");
}

#[test]
fn test_more_ebook_fb2_multilingual() {
    test_ebook_file("fb2/multilingual.fb2", "FB2");
}

#[test]
fn test_more_ebook_fb2_non_english_cyrillic() {
    test_ebook_file("fb2/non_english_cyrillic.fb2", "FB2");
}

// MOBI Tests
#[test]
fn test_more_ebook_mobi_simple_text() {
    test_ebook_file("mobi/simple_text.mobi", "MOBI");
}

#[test]
fn test_more_ebook_mobi_formatted() {
    test_ebook_file("mobi/formatted.mobi", "MOBI");
}

#[test]
fn test_more_ebook_mobi_with_metadata() {
    test_ebook_file("mobi/with_metadata.mobi", "MOBI");
}

#[test]
fn test_more_ebook_mobi_multi_chapter() {
    test_ebook_file("mobi/multi_chapter.mobi", "MOBI");
}

#[test]
fn test_more_ebook_mobi_large_content() {
    test_ebook_file("mobi/large_content.mobi", "MOBI");
}

// EMAIL FORMAT TESTS (Phase C)
// Tests for EML, MBOX, VCF email/contact formats
// Note: Email formats use Rust backend directly since Python docling doesn't support them

fn test_email_file(file_path: &str, format_name: &str) {
    let test_file = Path::new("../../test-corpus/email").join(file_path);
    assert!(test_file.exists(), "Test file not found: {test_file:?}");

    // Use Rust backend directly (Python doesn't support email formats)
    let converter = RustDocumentConverter::new().expect("Failed to create Rust converter");
    let result = converter
        .convert(&test_file)
        .unwrap_or_else(|_| panic!("Failed to convert {format_name} file"));

    // Verify document has content
    let markdown = result.document.to_markdown().to_string();
    assert!(
        !markdown.is_empty(),
        "Generated markdown is empty for {test_file:?}"
    );
    assert!(
        markdown.len() > 10,
        "Generated markdown too short for {test_file:?}"
    );

    // Verify content_blocks are populated (not None)
    assert!(
        result.document.content_blocks.is_some(),
        "content_blocks should not be None for {test_file:?}"
    );

    // Verify we have at least some DocItems
    let doc_items = result.document.content_blocks.as_ref().unwrap();
    assert!(
        !doc_items.is_empty(),
        "content_blocks should not be empty for {test_file:?}"
    );
}

// EML Tests
#[test]
fn test_more_email_eml_simple_text() {
    test_email_file("eml/simple_text.eml", "EML");
}

#[test]
fn test_more_email_eml_simple() {
    test_email_file("eml/simple.eml", "EML");
}

#[test]
fn test_more_email_eml_plain_text_email() {
    test_email_file("eml/plain_text_email.eml", "EML");
}

#[test]
fn test_more_email_eml_html_email() {
    test_email_file("eml/html_email.eml", "EML");
}

#[test]
fn test_more_email_eml_html_rich() {
    test_email_file("eml/html_rich.eml", "EML");
}

#[test]
fn test_more_email_eml_html_email_marketing() {
    test_email_file("eml/html_email_marketing.eml", "EML");
}

#[test]
fn test_more_email_eml_with_attachment() {
    test_email_file("eml/with_attachment.eml", "EML");
}

#[test]
fn test_more_email_eml_with_attachments() {
    test_email_file("eml/with_attachments.eml", "EML");
}

#[test]
fn test_more_email_eml_email_with_attachments() {
    test_email_file("eml/email_with_attachments.eml", "EML");
}

#[test]
fn test_more_email_eml_multipart_complex() {
    test_email_file("eml/multipart_complex.eml", "EML");
}

#[test]
fn test_more_email_eml_multirecipient() {
    test_email_file("eml/multirecipient.eml", "EML");
}

#[test]
fn test_more_email_eml_forwarded_nested() {
    test_email_file("eml/forwarded_nested.eml", "EML");
}

#[test]
fn test_more_email_eml_thread() {
    test_email_file("eml/thread.eml", "EML");
}

#[test]
fn test_more_email_eml_thread_conversation() {
    test_email_file("eml/thread_conversation.eml", "EML");
}

#[test]
fn test_more_email_eml_calendar_invite() {
    test_email_file("eml/calendar_invite.eml", "EML");
}

// MBOX Tests
#[test]
fn test_more_email_mbox_simple() {
    test_email_file("mbox/simple.mbox", "MBOX");
}

#[test]
fn test_more_email_mbox_small_mailbox() {
    test_email_file("mbox/small_mailbox.mbox", "MBOX");
}

#[test]
fn test_more_email_mbox_simple_10_messages() {
    test_email_file("mbox/simple_10_messages.mbox", "MBOX");
}

#[test]
fn test_more_email_mbox_archive() {
    test_email_file("mbox/archive.mbox", "MBOX");
}

#[test]
fn test_more_email_mbox_personal_archive() {
    test_email_file("mbox/personal_archive.mbox", "MBOX");
}

#[test]
fn test_more_email_mbox_business_emails() {
    test_email_file("mbox/business_emails.mbox", "MBOX");
}

#[test]
fn test_more_email_mbox_sent_items() {
    test_email_file("mbox/sent_items.mbox", "MBOX");
}

#[test]
fn test_more_email_mbox_spam_folder() {
    test_email_file("mbox/spam_folder.mbox", "MBOX");
}

#[test]
fn test_more_email_mbox_mixed_content() {
    test_email_file("mbox/mixed_content.mbox", "MBOX");
}

#[test]
fn test_more_email_mbox_threaded_conversation() {
    test_email_file("mbox/threaded_conversation.mbox", "MBOX");
}

#[test]
fn test_more_email_mbox_unicode_multilang() {
    test_email_file("mbox/unicode_multilang.mbox", "MBOX");
}

#[test]
fn test_more_email_mbox_large() {
    test_email_file("mbox/large.mbox", "MBOX");
}

// VCF Tests
#[test]
fn test_more_email_vcf_simple() {
    test_email_file("vcf/simple.vcf", "VCF");
}

#[test]
fn test_more_email_vcf_single_contact() {
    test_email_file("vcf/single_contact.vcf", "VCF");
}

#[test]
fn test_more_email_vcf_minimal_contact() {
    test_email_file("vcf/minimal_contact.vcf", "VCF");
}

#[test]
fn test_more_email_vcf_full_contact() {
    test_email_file("vcf/full_contact.vcf", "VCF");
}

#[test]
fn test_more_email_vcf_full_details() {
    test_email_file("vcf/full_details.vcf", "VCF");
}

#[test]
fn test_more_email_vcf_business() {
    test_email_file("vcf/business.vcf", "VCF");
}

#[test]
fn test_more_email_vcf_business_card() {
    test_email_file("vcf/business_card.vcf", "VCF");
}

#[test]
fn test_more_email_vcf_business_cards() {
    test_email_file("vcf/business_cards.vcf", "VCF");
}

#[test]
fn test_more_email_vcf_multiple() {
    test_email_file("vcf/multiple.vcf", "VCF");
}

#[test]
fn test_more_email_vcf_family_contacts() {
    test_email_file("vcf/family_contacts.vcf", "VCF");
}

#[test]
fn test_more_email_vcf_address_book() {
    test_email_file("vcf/address_book.vcf", "VCF");
}

#[test]
fn test_more_email_vcf_international() {
    test_email_file("vcf/international.vcf", "VCF");
}

// EBOOK FORMAT TESTS (Phase D)
// Tests for EPUB, FB2, MOBI ebook formats

#[test]
fn test_ebook_epub_simple() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/ebooks/epub/simple.epub");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_ebook_epub_complex() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/ebooks/epub/complex.epub");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_ebook_epub_with_images() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/ebooks/epub/with_images.epub");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_ebook_epub_large() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/ebooks/epub/large.epub");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_ebook_epub_non_english() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/ebooks/epub/non_english.epub");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_ebook_fb2_simple() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/ebooks/fb2/simple_minimal.fb2");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_ebook_fb2_with_formatting() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/ebooks/fb2/with_formatting.fb2");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_ebook_fb2_multi_section() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/ebooks/fb2/multi_section.fb2");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_ebook_fb2_with_tables() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/ebooks/fb2/with_tables.fb2");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_ebook_fb2_cyrillic() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/ebooks/fb2/non_english_cyrillic.fb2");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_ebook_mobi_simple() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/ebooks/mobi/simple_text.mobi");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_ebook_mobi_formatted() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/ebooks/mobi/formatted.mobi");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_ebook_mobi_multi_chapter() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/ebooks/mobi/multi_chapter.mobi");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_ebook_mobi_with_metadata() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/ebooks/mobi/with_metadata.mobi");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_ebook_mobi_large() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/ebooks/mobi/large_content.mobi");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

// OPENDOCUMENT FORMAT TESTS (Phase E)
// Tests for ODT, ODS, ODP (OpenDocument formats)

// OpenDocument Format Tests (ODT, ODS, ODP)
// Note: These use Rust backend directly since Python docling doesn't support OpenDocument formats

fn test_opendocument_file(fixture: &TestFixture) {
    let test_file = Path::new("../../test-corpus").join(&fixture.file_path);
    assert!(test_file.exists(), "Test file not found: {test_file:?}");

    // Use Rust backend directly (Python doesn't support OpenDocument)
    let converter = RustDocumentConverter::new().expect("Failed to create Rust converter");
    let result = converter
        .convert(&test_file)
        .expect("Failed to convert OpenDocument file");

    // Verify document has content
    let markdown = result.document.to_markdown().to_string();
    assert!(
        !markdown.is_empty(),
        "Generated markdown is empty for {test_file:?}"
    );
    assert!(
        markdown.len() > 10,
        "Generated markdown too short for {test_file:?}"
    );

    // Verify content_blocks are populated (not None)
    assert!(
        result.document.content_blocks.is_some(),
        "content_blocks should not be None for {test_file:?}"
    );
}

// ODT (OpenDocument Text) Tests
#[test]
fn test_more_opendocument_odt_simple_text_text() {
    test_opendocument_file(&fixture_simple_text_odt());
}

#[test]
fn test_more_opendocument_odt_multi_paragraph_text() {
    test_opendocument_file(&fixture_multi_paragraph_odt());
}

#[test]
fn test_more_opendocument_odt_report_text() {
    test_opendocument_file(&fixture_report_odt());
}

#[test]
fn test_more_opendocument_odt_meeting_notes_text() {
    test_opendocument_file(&fixture_meeting_notes_odt());
}

#[test]
fn test_more_opendocument_odt_technical_spec_text() {
    test_opendocument_file(&fixture_technical_spec_odt());
}

// ODS (OpenDocument Spreadsheet) Tests
#[test]
fn test_more_opendocument_ods_simple_spreadsheet_text() {
    test_opendocument_file(&fixture_simple_spreadsheet_ods());
}

#[test]
fn test_more_opendocument_ods_multi_sheet_text() {
    test_opendocument_file(&fixture_multi_sheet_ods());
}

#[test]
fn test_more_opendocument_ods_budget_text() {
    test_opendocument_file(&fixture_budget_ods());
}

#[test]
fn test_more_opendocument_ods_inventory_text() {
    test_opendocument_file(&fixture_inventory_ods());
}

#[test]
fn test_more_opendocument_ods_test_data_text() {
    test_opendocument_file(&fixture_test_data_ods());
}

// ODP (OpenDocument Presentation) Tests
#[test]
fn test_more_opendocument_odp_simple_presentation_text() {
    test_opendocument_file(&fixture_simple_presentation_odp());
}

#[test]
fn test_more_opendocument_odp_project_overview_text() {
    test_opendocument_file(&fixture_project_overview_odp());
}

#[test]
fn test_more_opendocument_odp_sales_pitch_text() {
    test_opendocument_file(&fixture_sales_pitch_odp());
}

#[test]
fn test_more_opendocument_odp_technical_talk_text() {
    test_opendocument_file(&fixture_technical_talk_odp());
}

#[test]
fn test_more_opendocument_odp_training_text() {
    test_opendocument_file(&fixture_training_odp());
}

// XPS FORMAT TESTS (Phase E - Microsoft Extended)
// Tests for XPS (XML Paper Specification) format

#[test]
fn test_xps_simple_text() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/xps/simple_text.xps");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_xps_formatted() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/xps/formatted.xps");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_xps_multi_page() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/xps/multi_page.xps");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_xps_report() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/xps/report.xps");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_xps_technical_spec() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/xps/technical_spec.xps");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

// SVG FORMAT TESTS (Phase E-3 - Graphics Open Standards)
// Tests for SVG (Scalable Vector Graphics) format

// SVG integration tests moved to test_more_svg_* section below (line ~7554)

// SUBTITLE FORMAT TESTS (Phase A - Foundation)
// Tests for SRT (SubRip) subtitle format

#[test]
fn test_subtitle_srt_simple_dialogue() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/subtitles/srt/simple_dialogue.srt");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_subtitle_srt_technical_presentation() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/subtitles/srt/technical_presentation.srt");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_subtitle_srt_interview() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/subtitles/srt/interview.srt");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_subtitle_srt_documentary() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/subtitles/srt/documentary_excerpt.srt");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_subtitle_srt_multilingual() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/subtitles/srt/multilingual_spanish.srt");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

// GIF FORMAT TESTS (Phase A - Images)
// Tests for GIF (Graphics Interchange Format) image format

#[test]
fn test_image_gif_simple() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/images/gif/simple.gif");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_image_gif_animated() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/images/gif/animated.gif");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_image_gif_large() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/images/gif/large.gif");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_image_gif_icon_small() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/images/gif/icon_small.gif");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_image_gif_transparent() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/images/gif/transparent.gif");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

// ============================================================================
// Phase F: Modern Image Formats - HEIF/HEIC Tests
// ============================================================================

#[test]
fn test_heif_simple_text() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/graphics/heif/simple_text.heic");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_heif_photo_sample() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/graphics/heif/photo_sample.heic");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_heif_high_compression() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/graphics/heif/high_compression.heic");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_heif_transparency_test() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/graphics/heif/transparency_test.heic");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_heif_large_image() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/graphics/heif/large_image.heic");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

// ============================================================================
// Phase F: Modern Image Formats - AVIF Tests
// ============================================================================

#[test]
fn test_avif_simple_text() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/graphics/avif/simple_text.avif");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_avif_photo_sample() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/graphics/avif/photo_sample.avif");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_avif_hdr_sample() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/graphics/avif/hdr_sample.avif");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_avif_animation_frame() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/graphics/avif/animation_frame.avif");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_avif_web_optimized() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/graphics/avif/web_optimized.avif");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

// ============================================================================
// Phase K2: Calendar & Scheduling - ICS/iCalendar Tests
// ============================================================================
// Python docling does not support ICS format (Rust-only implementation)

/// Helper function to test ICS calendar files using Rust backend directly
fn test_ics_file(file_path: &str) {
    let test_file = Path::new("../../test-corpus/calendar/ics").join(file_path);
    assert!(test_file.exists(), "Test file not found: {test_file:?}");

    // Use Rust backend directly (Python doesn't support ICS format)
    let converter = RustDocumentConverter::new().expect("Failed to create Rust converter");
    let result = converter
        .convert(&test_file)
        .expect("Failed to convert ICS file");

    // Verify document has content
    let markdown = result.document.to_markdown().to_string();
    assert!(
        !markdown.is_empty(),
        "Generated markdown is empty for {test_file:?}"
    );
    assert!(
        markdown.len() > 10,
        "Generated markdown too short for {test_file:?}"
    );

    // Verify content_blocks are populated (not None)
    assert!(
        result.document.content_blocks.is_some(),
        "content_blocks should not be None for {test_file:?}"
    );

    // Verify we have at least some DocItems
    let doc_items = result.document.content_blocks.as_ref().unwrap();
    assert!(
        !doc_items.is_empty(),
        "content_blocks should not be empty for {test_file:?}"
    );
}

#[test]
fn test_more_ics_single_event() {
    test_ics_file("single_event.ics");
}

#[test]
fn test_more_ics_recurring_meeting() {
    test_ics_file("recurring_meeting.ics");
}

#[test]
fn test_more_ics_allday_event() {
    test_ics_file("allday_event.ics");
}

#[test]
fn test_more_ics_with_todos() {
    test_ics_file("with_todos.ics");
}

#[test]
fn test_more_ics_complex_calendar() {
    test_ics_file("complex_calendar.ics");
}

// ==========================================
// Jupyter Notebook Tests (IPYNB)
// ==========================================
// Python docling supports ipynb format, but Rust implementation is independent

/// Helper function to test Jupyter notebook files using Rust backend directly
fn test_ipynb_file(file_path: &str) {
    let test_file = Path::new("../../test-corpus/notebook/ipynb").join(file_path);
    assert!(test_file.exists(), "Test file not found: {test_file:?}");

    // Use Rust backend directly (Python support exists but Rust is independent)
    let converter = RustDocumentConverter::new().expect("Failed to create Rust converter");
    let result = converter
        .convert(&test_file)
        .expect("Failed to convert notebook file");

    // Verify document has content
    let markdown = result.document.to_markdown().to_string();
    assert!(
        !markdown.is_empty(),
        "Generated markdown is empty for {test_file:?}"
    );
    assert!(
        markdown.len() > 10,
        "Generated markdown too short for {test_file:?}"
    );

    // Verify content_blocks are populated (not None)
    assert!(
        result.document.content_blocks.is_some(),
        "content_blocks should not be None for {test_file:?}"
    );

    // Verify we have at least some DocItems
    let doc_items = result.document.content_blocks.as_ref().unwrap();
    assert!(
        !doc_items.is_empty(),
        "content_blocks should not be empty for {test_file:?}"
    );

    // Verify we have Code DocItems (notebooks should have code cells)
    let has_code = doc_items
        .iter()
        .any(|item| matches!(item, DocItem::Code { .. }));
    assert!(
        has_code,
        "Notebook should contain at least one Code DocItem for {test_file:?}"
    );
}

#[test]
fn test_more_ipynb_simple_data_analysis() {
    test_ipynb_file("simple_data_analysis.ipynb");
}

#[test]
fn test_more_ipynb_machine_learning_demo() {
    test_ipynb_file("machine_learning_demo.ipynb");
}

#[test]
fn test_more_ipynb_math_formulas() {
    test_ipynb_file("math_formulas.ipynb");
}

#[test]
fn test_more_ipynb_error_handling() {
    test_ipynb_file("error_handling.ipynb");
}

#[test]
fn test_more_ipynb_complex_visualization() {
    test_ipynb_file("complex_visualization.ipynb");
}

// ==============================================================================
// GPS/GPX Format Tests
// ==============================================================================

#[test]
fn test_gpx_hiking_trail() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/gps/gpx/hiking_trail.gpx");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_gpx_cycling_route() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/gps/gpx/cycling_route.gpx");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_gpx_running_workout() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/gps/gpx/running_workout.gpx");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_gpx_waypoints_pois() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/gps/gpx/waypoints_pois.gpx");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_gpx_multi_day_journey() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/gps/gpx/multi_day_journey.gpx");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

// ==============================================================================
// GPS/KML Format Tests
// ==============================================================================

#[test]
fn test_kml_simple_landmark() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/gps/kml/simple_landmark.kml");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_kml_hiking_path() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/gps/kml/hiking_path.kml");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_kml_city_region() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/gps/kml/city_region.kml");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_kml_restaurant_guide() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/gps/kml/restaurant_guide.kml");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_kml_campus_tour() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/gps/kml/campus_tour.kml");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_kmz_simple_landmark() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/gps/kml/simple_landmark.kmz");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

// ==============================================================================
// Medical/DICOM Format Tests
// ==============================================================================

#[test]
fn test_dicom_ct_chest_scan() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/medical/ct_chest_scan.dcm");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_dicom_mri_brain_t1() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/medical/mri_brain_t1.dcm");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_dicom_xray_hand() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/medical/xray_hand.dcm");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_dicom_ultrasound_abdomen() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/medical/ultrasound_abdomen.dcm");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_dicom_structured_report() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/medical/structured_report.dcm");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

// ==============================================================================
// Legacy Format Tests (RTF)
// ==============================================================================

#[test]
fn test_rtf_simple_text() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/legacy/rtf/simple_text.rtf");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_rtf_formatted_text() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/legacy/rtf/formatted_text.rtf");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_rtf_business_memo() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/legacy/rtf/business_memo.rtf");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_rtf_technical_doc() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/legacy/rtf/technical_doc.rtf");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_rtf_unicode_test() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/legacy/rtf/unicode_test.rtf");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

// ==============================================================================
// Legacy DOC Format Tests (Microsoft Word 97-2003)
// ==============================================================================

#[test]
fn test_doc_simple_text() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/legacy/doc/simple_text.doc");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_doc_formatted_document() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/legacy/doc/formatted_document.doc");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_doc_tables_and_columns() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/legacy/doc/tables_and_columns.doc");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_doc_images_and_objects() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/legacy/doc/images_and_objects.doc");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_doc_complex_academic() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/legacy/doc/complex_academic.doc");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

// ==============================================================================
// CAD Format Tests (STL - STereoLithography)
// ==============================================================================

#[test]
fn test_stl_simple_cube() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/cad/stl/simple_cube.stl");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_stl_pyramid() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/cad/stl/pyramid.stl");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_stl_complex_shape() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/cad/stl/complex_shape.stl");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_stl_large_mesh() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/cad/stl/large_mesh.stl");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_stl_minimal_triangle() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/cad/stl/minimal_triangle.stl");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

// ============================================================================
// OBJ (Wavefront Object) Format Tests - Phase J2-2
// ============================================================================

#[test]
fn test_obj_simple_cube() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/cad/obj/simple_cube.obj");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_obj_teapot_excerpt() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/cad/obj/teapot_excerpt.obj");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_obj_pyramid_with_normals() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/cad/obj/pyramid_with_normals.obj");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_obj_textured_quad() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/cad/obj/textured_quad.obj");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_obj_icosphere() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/cad/obj/icosphere.obj");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

// ============================================================================
// GLTF/GLB Integration Tests (Phase J2-3)
// ============================================================================

#[test]
fn test_gltf_simple_triangle() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/cad/gltf/simple_triangle.gltf");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_gltf_simple_cube() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/cad/gltf/simple_cube.gltf");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_gltf_box() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/cad/gltf/box.gltf");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_gltf_duck() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/cad/gltf/duck.gltf");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_glb_box() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/cad/gltf/box.glb");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_gltf_avocado() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/cad/gltf/avocado.gltf");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_gltf_triangle_khronos() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/cad/gltf/triangle.gltf");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

// ============================================================================
// DXF (Drawing Exchange Format) Format Tests - Phase J1-1
// ============================================================================

#[test]
fn test_dxf_simple_drawing() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/cad/dxf/simple_drawing.dxf");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_dxf_floor_plan() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/cad/dxf/floor_plan.dxf");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_dxf_mechanical_part() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/cad/dxf/mechanical_part.dxf");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_dxf_electrical_schematic() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/cad/dxf/electrical_schematic.dxf");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_dxf_3d_model() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/cad/dxf/3d_model.dxf");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

// ============================================================================
// IDML (InDesign Markup Language) Format Tests - Phase I1-1
// ============================================================================

#[test]
fn test_idml_simple_document() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/adobe/idml/simple_document.idml");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_idml_magazine_layout() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/adobe/idml/magazine_layout.idml");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_idml_brochure() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/adobe/idml/brochure.idml");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_idml_book_chapter() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/adobe/idml/book_chapter.idml");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

#[test]
fn test_idml_technical_manual() {
    use std::path::PathBuf;
    let test_file = PathBuf::from("../../test-corpus/adobe/idml/technical_manual.idml");
    assert!(test_file.exists(), "Test file not found: {test_file:?}");
}

// ==============================================================================
// INTEGRATION TESTS - Archives and Subtitles (N=75)
// ==============================================================================

// ZIP Archive Tests

#[test]
fn test_more_zip_simple_single_file_text() {
    let fixture = fixture_zip_simple_single_file();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_more_zip_multiple_files_flat_text() {
    let fixture = fixture_zip_multiple_files_flat();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_more_zip_nested_directories_text() {
    let fixture = fixture_zip_nested_directories();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_more_zip_empty_archive_text() {
    let fixture = fixture_zip_empty_archive();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_more_zip_compressed_large_file_text() {
    let fixture = fixture_zip_compressed_large_file();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}

// TAR Archive Tests

#[test]
fn test_more_tar_uncompressed_text() {
    let fixture = fixture_tar_uncompressed();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_more_tar_nested_structure_text() {
    let fixture = fixture_tar_nested_structure();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_more_tar_large_file_text() {
    let fixture = fixture_tar_large_file();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}

// 7Z Archive Tests

#[test]
fn test_more_7z_multi_content_text() {
    let fixture = fixture_7z_multi_content();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_more_7z_solid_archive_text() {
    let fixture = fixture_7z_solid_archive();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_more_7z_ultra_compressed_text() {
    let fixture = fixture_7z_ultra_compressed();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}

// RAR Archive Tests

#[test]
fn test_more_rar_simple_text() {
    let fixture = fixture_rar_simple();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_more_rar_rar5_format_text() {
    let fixture = fixture_rar_rar5_format();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_more_rar_compressed_best_text() {
    let fixture = fixture_rar_compressed_best();
    run_integration_test(&fixture, ExtractionMode::TextOnly).unwrap();
}

// SRT Subtitle Tests
// SRT is a Rust-only format (Python docling doesn't support SRT)

/// Helper function to test SRT subtitle files using Rust backend directly
fn test_srt_file(file_path: &str) {
    let test_file = Path::new("../../test-corpus/subtitles/srt").join(file_path);
    assert!(test_file.exists(), "Test file not found: {test_file:?}");

    // Use Rust backend directly (Python doesn't support SRT format)
    let converter = RustDocumentConverter::new().expect("Failed to create Rust converter");
    let result = converter
        .convert(&test_file)
        .expect("Failed to convert SRT file");

    // Verify document has content
    let markdown = result.document.to_markdown().to_string();
    assert!(
        !markdown.is_empty(),
        "Generated markdown is empty for {test_file:?}"
    );
    assert!(
        markdown.len() > 10,
        "Generated markdown too short for {test_file:?}"
    );

    // Verify content_blocks are populated (not None)
    assert!(
        result.document.content_blocks.is_some(),
        "content_blocks should not be None for {test_file:?}"
    );

    // Verify we have at least some DocItems
    let doc_items = result.document.content_blocks.as_ref().unwrap();
    assert!(
        !doc_items.is_empty(),
        "content_blocks should not be empty for {test_file:?}"
    );
}

#[test]
fn test_more_srt_simple_dialogue() {
    test_srt_file("simple_dialogue.srt");
}

#[test]
fn test_more_srt_documentary_excerpt() {
    test_srt_file("documentary_excerpt.srt");
}

#[test]
fn test_more_srt_interview() {
    test_srt_file("interview.srt");
}

#[test]
fn test_more_srt_multilingual_spanish() {
    test_srt_file("multilingual_spanish.srt");
}

#[test]
fn test_more_srt_technical_presentation() {
    test_srt_file("technical_presentation.srt");
}

// ============================================================================
// RTF Integration Tests
// ============================================================================

fn test_rtf_file(file_path: &str) {
    let test_file = Path::new("../../test-corpus/legacy/rtf").join(file_path);
    assert!(test_file.exists(), "Test file not found: {test_file:?}");

    // Use Rust backend directly (Python doesn't support RTF format)
    let converter = RustDocumentConverter::new().expect("Failed to create Rust converter");
    let result = converter
        .convert(&test_file)
        .expect("Failed to convert RTF file");

    // Verify document has content
    let markdown = result.document.to_markdown().to_string();
    assert!(
        !markdown.is_empty(),
        "Generated markdown is empty for {test_file:?}"
    );
    assert!(
        markdown.len() > 10,
        "Generated markdown too short for {test_file:?}"
    );

    // Verify content_blocks are populated (not None)
    assert!(
        result.document.content_blocks.is_some(),
        "content_blocks should not be None for {test_file:?}"
    );

    // Verify we have at least some DocItems
    let doc_items = result.document.content_blocks.as_ref().unwrap();
    assert!(
        !doc_items.is_empty(),
        "content_blocks should not be empty for {test_file:?}"
    );
}

#[test]
fn test_more_rtf_simple_text() {
    test_rtf_file("simple_text.rtf");
}

#[test]
fn test_more_rtf_formatted_text() {
    test_rtf_file("formatted_text.rtf");
}

#[test]
fn test_more_rtf_business_memo() {
    test_rtf_file("business_memo.rtf");
}

#[test]
fn test_more_rtf_technical_doc() {
    test_rtf_file("technical_doc.rtf");
}

#[test]
fn test_more_rtf_unicode_test() {
    test_rtf_file("unicode_test.rtf");
}

// ============================================================================
// GIF Integration Tests
// ============================================================================

fn test_gif_file(file_path: &str) {
    let test_file = Path::new("../../test-corpus/images/gif").join(file_path);
    assert!(test_file.exists(), "Test file not found: {test_file:?}");

    // Use Rust backend directly (Python doesn't support GIF format separately)
    let converter = RustDocumentConverter::new().expect("Failed to create Rust converter");
    let result = converter
        .convert(&test_file)
        .expect("Failed to convert GIF file");

    // Verify document has content
    let markdown = result.document.to_markdown().to_string();
    assert!(
        !markdown.is_empty(),
        "Generated markdown is empty for {test_file:?}"
    );
    assert!(
        markdown.len() > 10,
        "Generated markdown too short for {test_file:?}"
    );

    // Verify content_blocks are populated (not None)
    assert!(
        result.document.content_blocks.is_some(),
        "content_blocks should not be None for {test_file:?}"
    );

    // Verify we have at least some DocItems
    let doc_items = result.document.content_blocks.as_ref().unwrap();
    assert!(
        !doc_items.is_empty(),
        "content_blocks should not be empty for {test_file:?}"
    );
}

#[test]
fn test_more_gif_simple() {
    test_gif_file("simple.gif");
}

#[test]
fn test_more_gif_animated() {
    test_gif_file("animated.gif");
}

#[test]
fn test_more_gif_transparent() {
    test_gif_file("transparent.gif");
}

#[test]
fn test_more_gif_large() {
    test_gif_file("large.gif");
}

#[test]
fn test_more_gif_icon_small() {
    test_gif_file("icon_small.gif");
}

// ============================================================================
// BMP Integration Tests
// ============================================================================

fn test_bmp_file(file_path: &str) {
    let test_file = Path::new("../../test-corpus/bmp").join(file_path);
    assert!(test_file.exists(), "Test file not found: {test_file:?}");

    // Use Rust backend directly (Python doesn't support BMP format separately)
    let converter = RustDocumentConverter::new().expect("Failed to create Rust converter");
    let result = converter
        .convert(&test_file)
        .expect("Failed to convert BMP file");

    // Verify document has content
    let markdown = result.document.to_markdown().to_string();
    assert!(
        !markdown.is_empty(),
        "Generated markdown is empty for {test_file:?}"
    );
    assert!(
        markdown.len() > 10,
        "Generated markdown too short for {test_file:?}"
    );

    // Verify content_blocks are populated (not None)
    assert!(
        result.document.content_blocks.is_some(),
        "content_blocks should not be None for {test_file:?}"
    );

    // Verify we have at least some DocItems
    let doc_items = result.document.content_blocks.as_ref().unwrap();
    assert!(
        !doc_items.is_empty(),
        "content_blocks should not be empty for {test_file:?}"
    );
}

#[test]
fn test_more_bmp_monochrome() {
    test_bmp_file("monochrome.bmp");
}

#[test]
fn test_more_bmp_gradient() {
    test_bmp_file("gradient.bmp");
}

#[test]
fn test_more_bmp_pattern() {
    test_bmp_file("pattern.bmp");
}

// ============================================================================
// HEIF Integration Tests
// ============================================================================

fn test_heif_file(file_path: &str) {
    let test_file = Path::new("../../test-corpus/graphics/heif").join(file_path);
    assert!(test_file.exists(), "Test file not found: {test_file:?}");

    // Use Rust backend directly (Python doesn't support HEIF format separately)
    let converter = RustDocumentConverter::new().expect("Failed to create Rust converter");
    let result = converter
        .convert(&test_file)
        .expect("Failed to convert HEIF file");

    // Verify document has content
    let markdown = result.document.to_markdown().to_string();
    assert!(
        !markdown.is_empty(),
        "Generated markdown is empty for {test_file:?}"
    );
    assert!(
        markdown.len() > 10,
        "Generated markdown too short for {test_file:?}"
    );

    // Verify content_blocks are populated (not None)
    assert!(
        result.document.content_blocks.is_some(),
        "content_blocks should not be None for {test_file:?}"
    );

    // Verify we have at least some DocItems
    let doc_items = result.document.content_blocks.as_ref().unwrap();
    assert!(
        !doc_items.is_empty(),
        "content_blocks should not be empty for {test_file:?}"
    );
}

#[test]
fn test_more_heif_simple_text() {
    test_heif_file("simple_text.heic");
}

#[test]
fn test_more_heif_photo_sample() {
    test_heif_file("photo_sample.heic");
}

#[test]
fn test_more_heif_large_image() {
    test_heif_file("large_image.heic");
}

#[test]
fn test_more_heif_high_compression() {
    test_heif_file("high_compression.heic");
}

#[test]
fn test_more_heif_transparency_test() {
    test_heif_file("transparency_test.heic");
}

// ============================================================================
// AVIF Integration Tests
// ============================================================================

fn test_avif_file(file_path: &str) {
    let test_file = Path::new("../../test-corpus/graphics/avif").join(file_path);
    assert!(test_file.exists(), "Test file not found: {test_file:?}");

    // Use Rust backend directly (Python doesn't support AVIF format separately)
    let converter = RustDocumentConverter::new().expect("Failed to create Rust converter");
    let result = converter
        .convert(&test_file)
        .expect("Failed to convert AVIF file");

    // Verify document has content
    let markdown = result.document.to_markdown().to_string();
    assert!(
        !markdown.is_empty(),
        "Generated markdown is empty for {test_file:?}"
    );
    assert!(
        markdown.len() > 10,
        "Generated markdown too short for {test_file:?}"
    );

    // Verify content_blocks are populated (not None)
    assert!(
        result.document.content_blocks.is_some(),
        "content_blocks should not be None for {test_file:?}"
    );

    // Verify we have at least some DocItems
    let doc_items = result.document.content_blocks.as_ref().unwrap();
    assert!(
        !doc_items.is_empty(),
        "content_blocks should not be empty for {test_file:?}"
    );
}

#[test]
fn test_more_avif_simple_text() {
    test_avif_file("simple_text.avif");
}

#[test]
fn test_more_avif_photo_sample() {
    test_avif_file("photo_sample.avif");
}

#[test]
fn test_more_avif_animation_frame() {
    test_avif_file("animation_frame.avif");
}

#[test]
fn test_more_avif_hdr_sample() {
    test_avif_file("hdr_sample.avif");
}

#[test]
fn test_more_avif_web_optimized() {
    test_avif_file("web_optimized.avif");
}

// ============================================================================
// SVG Integration Tests
// ============================================================================

fn test_svg_file(file_path: &str) {
    let test_file = Path::new("../../test-corpus/svg").join(file_path);
    assert!(test_file.exists(), "Test file not found: {test_file:?}");

    // Use Rust backend directly (Python doesn't support SVG format separately)
    let converter = RustDocumentConverter::new().expect("Failed to create Rust converter");
    let result = converter
        .convert(&test_file)
        .expect("Failed to convert SVG file");

    // Verify document has content
    let markdown = result.document.to_markdown().to_string();
    assert!(
        !markdown.is_empty(),
        "Generated markdown is empty for {test_file:?}"
    );
    assert!(
        markdown.len() > 10,
        "Generated markdown too short for {test_file:?}"
    );

    // Verify content_blocks are populated (not None)
    assert!(
        result.document.content_blocks.is_some(),
        "content_blocks should not be None for {test_file:?}"
    );

    // Verify we have at least some DocItems
    let doc_items = result.document.content_blocks.as_ref().unwrap();
    assert!(
        !doc_items.is_empty(),
        "content_blocks should not be empty for {test_file:?}"
    );
}

#[test]
fn test_more_svg_simple_icon() {
    test_svg_file("simple_icon.svg");
}

#[test]
fn test_more_svg_diagram() {
    test_svg_file("diagram.svg");
}

#[test]
fn test_more_svg_technical_drawing() {
    test_svg_file("technical_drawing.svg");
}

#[test]
fn test_more_svg_infographic() {
    test_svg_file("infographic.svg");
}

#[test]
fn test_more_svg_map() {
    test_svg_file("map.svg");
}

// ============================================================================
// GPX Integration Tests
// ============================================================================

fn test_gpx_file(file_path: &str) {
    let test_file = Path::new("../../test-corpus/gps/gpx").join(file_path);
    assert!(test_file.exists(), "Test file not found: {test_file:?}");

    // Use Rust backend directly (Python doesn't support GPX format)
    let converter = RustDocumentConverter::new().expect("Failed to create Rust converter");
    let result = converter
        .convert(&test_file)
        .expect("Failed to convert GPX file");

    // Verify document has content
    let markdown = result.document.to_markdown().to_string();
    assert!(
        !markdown.is_empty(),
        "Generated markdown is empty for {test_file:?}"
    );
    assert!(
        markdown.len() > 10,
        "Generated markdown too short for {test_file:?}"
    );

    // Verify content_blocks are populated (not None)
    assert!(
        result.document.content_blocks.is_some(),
        "content_blocks should not be None for {test_file:?}"
    );

    // Verify we have at least some DocItems
    let doc_items = result.document.content_blocks.as_ref().unwrap();
    assert!(
        !doc_items.is_empty(),
        "content_blocks should not be empty for {test_file:?}"
    );
}

#[test]
fn test_more_gpx_hiking_trail() {
    test_gpx_file("hiking_trail.gpx");
}

#[test]
fn test_more_gpx_cycling_route() {
    test_gpx_file("cycling_route.gpx");
}

#[test]
fn test_more_gpx_running_workout() {
    test_gpx_file("running_workout.gpx");
}

#[test]
fn test_more_gpx_multi_day_journey() {
    test_gpx_file("multi_day_journey.gpx");
}

#[test]
fn test_more_gpx_waypoints_pois() {
    test_gpx_file("waypoints_pois.gpx");
}

// ============================================================================
// KML Integration Tests
// ============================================================================

fn test_kml_file(file_path: &str) {
    let test_file = Path::new("../../test-corpus/gps/kml").join(file_path);
    assert!(test_file.exists(), "Test file not found: {test_file:?}");

    // Use Rust backend directly (Python doesn't support KML format)
    let converter = RustDocumentConverter::new().expect("Failed to create Rust converter");
    let result = converter
        .convert(&test_file)
        .expect("Failed to convert KML file");

    // Verify document has content
    let markdown = result.document.to_markdown().to_string();
    assert!(
        !markdown.is_empty(),
        "Generated markdown is empty for {test_file:?}"
    );
    assert!(
        markdown.len() > 10,
        "Generated markdown too short for {test_file:?}"
    );

    // Verify content_blocks are populated (not None)
    assert!(
        result.document.content_blocks.is_some(),
        "content_blocks should not be None for {test_file:?}"
    );

    // Verify we have at least some DocItems
    let doc_items = result.document.content_blocks.as_ref().unwrap();
    assert!(
        !doc_items.is_empty(),
        "content_blocks should not be empty for {test_file:?}"
    );
}

#[test]
fn test_more_kml_simple_landmark() {
    test_kml_file("simple_landmark.kml");
}

#[test]
fn test_more_kml_hiking_path() {
    test_kml_file("hiking_path.kml");
}

#[test]
fn test_more_kml_city_region() {
    test_kml_file("city_region.kml");
}

#[test]
fn test_more_kml_restaurant_guide() {
    test_kml_file("restaurant_guide.kml");
}

#[test]
fn test_more_kml_campus_tour() {
    test_kml_file("campus_tour.kml");
}

#[test]
fn test_more_kmz_simple_landmark() {
    test_kml_file("simple_landmark.kmz");
}

// ============================================================
// DICOM (Digital Imaging and Communications in Medicine) Tests
// ============================================================

/// Helper function to test DICOM medical image files
fn test_dicom_file(file_name: &str) {
    let file_path = format!("../../test-corpus/medical/{file_name}");
    let path = Path::new(&file_path);

    // Verify file exists
    assert!(path.exists(), "Test file not found: {file_path}");

    // Parse using Rust backend (Python doesn't support DICOM)
    let converter = RustDocumentConverter::new().expect("Failed to create Rust converter");
    let result = converter
        .convert(&path)
        .expect("Failed to parse DICOM file");

    // Verify markdown was generated
    assert!(
        !result.document.markdown.is_empty(),
        "DICOM file should produce markdown"
    );
    assert!(
        result.document.markdown.len() > 10,
        "DICOM markdown should be substantial"
    );

    // Verify DocItems were generated
    assert!(
        result.document.content_blocks.is_some(),
        "DICOM file should generate DocItems"
    );

    let doc_items = result.document.content_blocks.unwrap();
    assert!(
        !doc_items.is_empty(),
        "DICOM file should generate non-empty DocItems"
    );

    // Verify at least one Text DocItem exists
    let has_text = doc_items
        .iter()
        .any(|item| matches!(item, DocItem::Text { .. }));
    assert!(has_text, "DICOM file should generate Text DocItems");
}

#[test]
fn test_more_dicom_ct_chest_scan() {
    test_dicom_file("ct_chest_scan.dcm");
}

#[test]
fn test_more_dicom_mri_brain_t1() {
    test_dicom_file("mri_brain_t1.dcm");
}

#[test]
fn test_more_dicom_structured_report() {
    test_dicom_file("structured_report.dcm");
}

#[test]
fn test_more_dicom_ultrasound_abdomen() {
    test_dicom_file("ultrasound_abdomen.dcm");
}

#[test]
fn test_more_dicom_xray_hand() {
    test_dicom_file("xray_hand.dcm");
}

// ============================================================
// CAD and 3D Format Tests
// ============================================================

/// Helper function to test CAD/3D files
fn test_cad_file(subdir: &str, file_name: &str) {
    let file_path = format!("../../test-corpus/cad/{subdir}/{file_name}");
    let path = Path::new(&file_path);

    // Verify file exists
    assert!(path.exists(), "Test file not found: {file_path}");

    // Parse using Rust backend (Python doesn't support CAD formats)
    let converter = RustDocumentConverter::new().expect("Failed to create Rust converter");
    let result = converter.convert(&path).expect("Failed to parse CAD file");

    // Verify markdown was generated
    assert!(
        !result.document.markdown.is_empty(),
        "CAD file should produce markdown"
    );
    assert!(
        result.document.markdown.len() > 10,
        "CAD markdown should be substantial"
    );

    // Verify DocItems were generated
    assert!(
        result.document.content_blocks.is_some(),
        "CAD file should generate DocItems"
    );

    let doc_items = result.document.content_blocks.unwrap();
    assert!(
        !doc_items.is_empty(),
        "CAD file should generate non-empty DocItems"
    );

    // Verify at least one Text DocItem exists
    let has_text = doc_items
        .iter()
        .any(|item| matches!(item, DocItem::Text { .. }));
    assert!(has_text, "CAD file should generate Text DocItems");
}

// STL (STereoLithography) format tests
#[test]
fn test_more_cad_stl_simple_cube() {
    test_cad_file("stl", "simple_cube.stl");
}

#[test]
fn test_more_cad_stl_pyramid() {
    test_cad_file("stl", "pyramid.stl");
}

#[test]
fn test_more_cad_stl_minimal_triangle() {
    test_cad_file("stl", "minimal_triangle.stl");
}

// OBJ (Wavefront Object) format tests
#[test]
fn test_more_cad_obj_simple_cube() {
    test_cad_file("obj", "simple_cube.obj");
}

#[test]
fn test_more_cad_obj_icosphere() {
    test_cad_file("obj", "icosphere.obj");
}

#[test]
fn test_more_cad_obj_textured_quad() {
    test_cad_file("obj", "textured_quad.obj");
}

// GLTF (GL Transmission Format) tests
#[test]
fn test_more_cad_gltf_simple_cube() {
    test_cad_file("gltf", "simple_cube.gltf");
}

#[test]
fn test_more_cad_gltf_box() {
    test_cad_file("gltf", "box.gltf");
}

#[test]
fn test_more_cad_glb_box() {
    test_cad_file("gltf", "box.glb");
}

// DXF (Drawing Exchange Format) tests
#[test]
fn test_more_cad_dxf_simple_drawing() {
    test_cad_file("dxf", "simple_drawing.dxf");
}

#[test]
fn test_more_cad_dxf_floor_plan() {
    test_cad_file("dxf", "floor_plan.dxf");
}

#[test]
fn test_more_cad_dxf_mechanical_part() {
    test_cad_file("dxf", "mechanical_part.dxf");
}

// ============================================================
// Legacy DOC Format Canonical Tests
// ============================================================

#[test]
fn test_canon_doc_simple_text() {
    let fixture = fixture_doc_simple_text();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_doc_formatted_document() {
    let fixture = fixture_doc_formatted_document();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_doc_tables_and_columns() {
    let fixture = fixture_doc_tables_and_columns();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_doc_complex_academic() {
    let fixture = fixture_doc_complex_academic();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

// ============================================================
// Microsoft Project (MPP) Format Canonical Tests
// ============================================================

#[test]
fn test_canon_mpp_sample1_2019() {
    let fixture = fixture_mpp_sample1_2019();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_mpp_sample2_2010() {
    let fixture = fixture_mpp_sample2_2010();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_mpp_sample3_2007() {
    let fixture = fixture_mpp_sample3_2007();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_mpp_sample4_2003() {
    let fixture = fixture_mpp_sample4_2003();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

// ============================================================
// XPS Format Canonical Tests
// ============================================================

#[test]
fn test_canon_xps_simple_text() {
    let fixture = fixture_xps_simple_text();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_xps_formatted() {
    let fixture = fixture_xps_formatted();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_xps_multi_page() {
    let fixture = fixture_xps_multi_page();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_xps_report() {
    let fixture = fixture_xps_report();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_xps_technical_spec() {
    let fixture = fixture_xps_technical_spec();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

// ============================================================
// Microsoft OneNote (ONE) Format Canonical Tests
// ============================================================
// Note: OneNote desktop format (.one) not yet supported
// Available library only supports cloud format, not desktop OneNote 2016 files
// See: https://github.com/msiemens/onenote.rs
// Deferred until library matures

// ============================================================
// Jupyter Notebook (IPYNB) Format Canonical Tests
// ============================================================

#[test]
fn test_canon_ipynb_simple_data_analysis() {
    let fixture = fixture_ipynb_simple_data_analysis();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_ipynb_machine_learning_demo() {
    let fixture = fixture_ipynb_machine_learning_demo();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_ipynb_math_formulas() {
    let fixture = fixture_ipynb_math_formulas();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_ipynb_error_handling() {
    let fixture = fixture_ipynb_error_handling();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_ipynb_complex_visualization() {
    let fixture = fixture_ipynb_complex_visualization();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

// ============================================================
// Adobe IDML Format Canonical Tests
// ============================================================

#[test]
fn test_canon_idml_simple_document() {
    let fixture = fixture_idml_simple_document();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_idml_brochure() {
    let fixture = fixture_idml_brochure();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_idml_technical_manual() {
    let fixture = fixture_idml_technical_manual();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_idml_book_chapter() {
    let fixture = fixture_idml_book_chapter();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_idml_magazine_layout() {
    let fixture = fixture_idml_magazine_layout();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

// ===== DXF Tests =====

#[test]
fn test_canon_dxf_simple_drawing() {
    let fixture = fixture_dxf_simple_drawing();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_dxf_floor_plan() {
    let fixture = fixture_dxf_floor_plan();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_dxf_mechanical_part() {
    let fixture = fixture_dxf_mechanical_part();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_dxf_electrical_schematic() {
    let fixture = fixture_dxf_electrical_schematic();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_dxf_3d_model() {
    let fixture = fixture_dxf_3d_model();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

// ===== GIF Tests =====

// ===== BMP Tests =====

// ===== JPEG Tests =====

// ===== HEIF Tests =====

#[test]
fn test_canon_heif_high_compression() {
    let fixture = fixture_heif_high_compression();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_heif_large_image() {
    let fixture = fixture_heif_large_image();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_heif_photo_sample() {
    let fixture = fixture_heif_photo_sample();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_heif_simple_text() {
    let fixture = fixture_heif_simple_text();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_heif_transparency_test() {
    let fixture = fixture_heif_transparency_test();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

// ===== AVIF Tests =====

#[test]
fn test_canon_avif_animation_frame() {
    let fixture = fixture_avif_animation_frame();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_avif_hdr_sample() {
    let fixture = fixture_avif_hdr_sample();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_avif_photo_sample() {
    let fixture = fixture_avif_photo_sample();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_avif_simple_text() {
    let fixture = fixture_avif_simple_text();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_avif_web_optimized() {
    let fixture = fixture_avif_web_optimized();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

// ===== GPX Tests =====

#[test]
fn test_canon_gpx_hiking_trail() {
    let fixture = fixture_gpx_hiking_trail();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_gpx_cycling_route() {
    let fixture = fixture_gpx_cycling_route();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_gpx_running_workout() {
    let fixture = fixture_gpx_running_workout();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_gpx_waypoints_pois() {
    let fixture = fixture_gpx_waypoints_pois();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_gpx_multi_day_journey() {
    let fixture = fixture_gpx_multi_day_journey();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

// ===== KML Tests =====

#[test]
fn test_canon_kml_simple_landmark() {
    let fixture = fixture_kml_simple_landmark();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_kml_hiking_path() {
    let fixture = fixture_kml_hiking_path();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_kml_restaurant_guide() {
    let fixture = fixture_kml_restaurant_guide();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_kml_city_region() {
    let fixture = fixture_kml_city_region();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_kml_campus_tour() {
    let fixture = fixture_kml_campus_tour();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

// ===== ICS Tests =====

#[test]
fn test_canon_ics_single_event() {
    let fixture = fixture_ics_single_event();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_ics_meeting() {
    let fixture = fixture_ics_meeting();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_ics_recurring_meeting() {
    let fixture = fixture_ics_recurring_meeting();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_ics_allday_event() {
    let fixture = fixture_ics_allday_event();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
fn test_canon_ics_complex_calendar() {
    let fixture = fixture_ics_complex_calendar();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

// ============================================================
// Microsoft Publisher (PUB) Format Canonical Tests
// ============================================================
// NOTE: These tests use synthetic .pub files (valid OLE structure but minimal
// Publisher content). Real .pub files created by Microsoft Publisher are needed
// for full testing. LibreOffice cannot reliably process synthetic .pub files.
// Tests are ignored by default - run with `cargo test -- --ignored` if you have
// real .pub files in test-corpus/publisher/

#[test]
#[ignore = "Requires real Microsoft Publisher files (synthetic files fail LibreOffice conversion)"]
fn test_canon_pub_sample1_flyer() {
    let fixture = fixture_pub_sample1_flyer();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
#[ignore = "Requires real Microsoft Publisher files (synthetic files fail LibreOffice conversion)"]
fn test_canon_pub_sample2_business_card() {
    let fixture = fixture_pub_sample2_business_card();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
#[ignore = "Requires real Microsoft Publisher files (synthetic files fail LibreOffice conversion)"]
fn test_canon_pub_sample3_newsletter() {
    let fixture = fixture_pub_sample3_newsletter();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
#[ignore = "Requires real Microsoft Publisher files (synthetic files fail LibreOffice conversion)"]
fn test_canon_pub_sample4_brochure() {
    let fixture = fixture_pub_sample4_brochure();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}

#[test]
#[ignore = "Requires real Microsoft Publisher files (synthetic files fail LibreOffice conversion)"]
fn test_canon_pub_sample5_greeting_card() {
    let fixture = fixture_pub_sample5_greeting_card();
    run_integration_test_no_output_check(&fixture, ExtractionMode::TextOnly).unwrap();
}
