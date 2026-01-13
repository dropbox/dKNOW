use chrono::Utc;
use pdfium_sys::*;
use serde::Serialize;
use std::env;
use std::ffi::CString;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{self, Command};

/// Smart text extraction dispatcher that chooses the best strategy based on document size.
///
/// Strategy:
/// - Small PDFs (< 200 pages): Single-threaded (avoids process overhead)
/// - Large PDFs (≥ 200 pages): Multi-process with 4 workers (true parallelism, 3.0x+ speedup)
///
/// Performance characteristics (4 workers):
/// - 100 pages: 1.54x speedup (overhead dominates)
/// - 821 pages: 3.21x speedup (CPU-bound work parallelizes)
///
/// Threshold of 200 pages optimizes for overall throughput.
const PAGE_THRESHOLD: i32 = 200;
const DEFAULT_WORKERS: usize = 4;

// ========================================
// JSONL Data Structures
// ========================================

#[derive(Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
enum JsonlRecord {
    Metadata {
        pdf: String,
        pages: i32,
        version: String,
        created: String,
    },
    Page {
        page: i32,
        width: f64,
        height: f64,
    },
    Char {
        page: i32,
        index: i32,
        #[serde(rename = "char")]
        character: String,
        unicode: u32,
        bbox: BBox,
        origin: Point,
        angle: f32,
        font: Font,
        color: Color,
        flags: Flags,
    },
}

#[derive(Serialize)]
struct BBox {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Serialize)]
struct Point {
    x: f64,
    y: f64,
}

#[derive(Serialize)]
struct Font {
    name: String,
    size: f64,
    weight: i32,
}

#[derive(Serialize)]
struct Color {
    fill: String,
    stroke: String,
}

#[derive(Serialize)]
struct Flags {
    generated: bool,
    hyphen: bool,
    unicode_error: bool,
}

// ========================================
// Output Format Configuration
// ========================================

#[derive(Clone, Copy, PartialEq)]
enum OutputFormat {
    Text,  // UTF-32 LE (default)
    Jsonl, // JSON Lines with rich annotations
}

fn main() {
    let args: Vec<String> = env::args().collect();

    // Check if this is a worker process for multi-process mode
    if args.len() >= 2 && args[1] == "--worker" {
        worker_main();
        return;
    }

    // Parse arguments
    let mut pdf_path: Option<String> = None;
    let mut output_path: Option<String> = None;
    let mut worker_count: Option<usize> = None;
    let mut output_format = OutputFormat::Text;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--jsonl" => {
                output_format = OutputFormat::Jsonl;
                i += 1;
            }
            "--workers" => {
                if i + 1 >= args.len() {
                    eprintln!("Error: --workers requires a number");
                    process::exit(1);
                }
                worker_count = Some(args[i + 1].parse().unwrap_or_else(|_| {
                    eprintln!("Error: --workers must be a number between 1 and 16");
                    process::exit(1);
                }));
                i += 2;
            }
            "--help" => {
                print_help(&args[0]);
                process::exit(0);
            }
            arg if !arg.starts_with("--") => {
                if pdf_path.is_none() {
                    pdf_path = Some(arg.to_string());
                } else if output_path.is_none() {
                    output_path = Some(arg.to_string());
                } else {
                    eprintln!("Error: unexpected argument: {}", arg);
                    process::exit(1);
                }
                i += 1;
            }
            arg => {
                eprintln!("Error: unknown flag: {}", arg);
                process::exit(1);
            }
        }
    }

    // Validate required arguments
    let pdf_path = pdf_path.unwrap_or_else(|| {
        eprintln!("Error: PDF input file required");
        print_help(&args[0]);
        process::exit(1);
    });

    let output_path = output_path.unwrap_or_else(|| {
        eprintln!("Error: output file required");
        print_help(&args[0]);
        process::exit(1);
    });

    if !Path::new(&pdf_path).exists() {
        eprintln!("Error: PDF file not found: {}", pdf_path);
        process::exit(1);
    }

    // Get page count to determine strategy
    let page_count = match get_page_count(&pdf_path) {
        Ok(count) => count,
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    };

    // Determine worker count (explicit or auto)
    let worker_count = worker_count.unwrap_or({
        // Auto-select based on document size
        if page_count < PAGE_THRESHOLD {
            1 // Single-threaded for small PDFs
        } else {
            DEFAULT_WORKERS // Multi-process for large PDFs
        }
    });

    if !(1..=16).contains(&worker_count) {
        eprintln!("Error: worker_count must be between 1 and 16");
        process::exit(1);
    }

    // Route to appropriate implementation based on format and worker count
    let result = match (output_format, worker_count) {
        (OutputFormat::Text, 1) => {
            eprintln!(
                "Using single-threaded text extraction ({} pages)",
                page_count
            );
            extract_text_single_threaded(&pdf_path, &output_path)
        }
        (OutputFormat::Text, n) => {
            eprintln!(
                "Using multi-process text extraction ({} pages, {} workers)",
                page_count, n
            );
            extract_text_multiprocess(&pdf_path, &output_path, n, page_count)
        }
        (OutputFormat::Jsonl, 1) => {
            eprintln!(
                "Using single-threaded JSONL extraction ({} pages)",
                page_count
            );
            extract_jsonl_single_threaded(&pdf_path, &output_path)
        }
        (OutputFormat::Jsonl, n) => {
            eprintln!(
                "Using multi-process JSONL extraction ({} pages, {} workers)",
                page_count, n
            );
            extract_jsonl_multiprocess(&pdf_path, &output_path, n, page_count)
        }
    };

    match result {
        Ok(_) => process::exit(0),
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
}

fn print_help(program_name: &str) {
    eprintln!("Usage: {} <input.pdf> <output> [OPTIONS]", program_name);
    eprintln!();
    eprintln!("OPTIONS:");
    eprintln!(
        "  --jsonl              Output JSONL rich annotation format (default: UTF-32 LE text)"
    );
    eprintln!("  --workers N          Number of workers (1-16, default: auto-select)");
    eprintln!("  --help               Show this help");
    eprintln!();
    eprintln!("EXAMPLES:");
    eprintln!("  # Plain text extraction (default)");
    eprintln!("  {} input.pdf output.txt", program_name);
    eprintln!();
    eprintln!("  # JSONL rich annotation");
    eprintln!("  {} input.pdf output.jsonl --jsonl", program_name);
    eprintln!();
    eprintln!("  # JSONL with explicit worker count");
    eprintln!(
        "  {} input.pdf output.jsonl --jsonl --workers 8",
        program_name
    );
    eprintln!();
    eprintln!("  # Force single-threaded");
    eprintln!("  {} input.pdf output.txt --workers 1", program_name);
}

fn get_page_count(pdf_path: &str) -> Result<i32, String> {
    unsafe {
        FPDF_InitLibrary();
        let c_path = CString::new(pdf_path).unwrap();
        let doc = FPDF_LoadDocument(c_path.as_ptr(), std::ptr::null());

        if doc.is_null() {
            FPDF_DestroyLibrary();
            return Err(format!("Failed to load PDF: {}", pdf_path));
        }

        let page_count = FPDF_GetPageCount(doc);
        FPDF_CloseDocument(doc);
        FPDF_DestroyLibrary();

        // Allow 0-page PDFs (upstream behavior: valid empty document)
        Ok(page_count)
    }
}

// ========================================
// Single-threaded implementation
// ========================================

fn extract_text_single_threaded(pdf_path: &str, output_path: &str) -> Result<(), String> {
    unsafe {
        FPDF_InitLibrary();

        let c_path = CString::new(pdf_path).unwrap();
        let doc = FPDF_LoadDocument(c_path.as_ptr(), std::ptr::null());

        if doc.is_null() {
            FPDF_DestroyLibrary();
            return Err(format!("Failed to load PDF: {}", pdf_path));
        }

        let mut output_file = File::create(output_path)
            .map_err(|e| format!("Failed to create output file: {}", e))?;

        // Write UTF-32 LE BOM
        output_file
            .write_all(&[0xFF, 0xFE, 0x00, 0x00])
            .map_err(|e| format!("Failed to write BOM: {}", e))?;

        let page_count = FPDF_GetPageCount(doc);

        // Reusable page buffer (Task 2.3: Buffer Pooling)
        // Allocate once, reuse across pages to reduce malloc/free overhead
        let mut page_buffer = Vec::with_capacity(256 * 1024); // 256KB initial capacity

        // Allow 0-page PDFs (upstream behavior: output BOM only)
        // Extract text from each page
        let mut successful_pages = 0;
        for page_index in 0..page_count {
            let page = FPDF_LoadPage(doc, page_index);
            if page.is_null() {
                eprintln!("Warning: Failed to load page {}", page_index);
                continue;
            }

            // Write BOM for each page after the first successful page
            if successful_pages > 0 {
                output_file
                    .write_all(&[0xFF, 0xFE, 0x00, 0x00])
                    .map_err(|e| format!("Failed to write BOM: {}", e))?;
            }
            successful_pages += 1;

            let text_page = FPDFText_LoadPage(page);
            if text_page.is_null() {
                FPDF_ClosePage(page);
                eprintln!("Warning: Failed to load text for page {}", page_index);
                continue;
            }

            let char_count = FPDFText_CountChars(text_page);

            // Clear buffer for reuse (retains capacity)
            page_buffer.clear();
            // Ensure capacity for this page
            let required_capacity = (char_count * 4) as usize;
            if page_buffer.capacity() < required_capacity {
                page_buffer.reserve(required_capacity - page_buffer.len());
            }

            // Extract text character by character, handling UTF-16 surrogate pairs
            let mut i = 0;
            while i < char_count {
                let code_unit = FPDFText_GetUnicode(text_page, i);

                let codepoint = if (0xD800..=0xDBFF).contains(&code_unit) {
                    // High surrogate - read low surrogate
                    i += 1;
                    if i < char_count {
                        let low_surrogate = FPDFText_GetUnicode(text_page, i);
                        ((code_unit - 0xD800) << 10) + (low_surrogate - 0xDC00) + 0x10000
                    } else {
                        code_unit
                    }
                } else {
                    code_unit
                };

                page_buffer.extend_from_slice(&codepoint.to_le_bytes());
                i += 1;
            }

            // Single write for entire page (buffered I/O)
            output_file
                .write_all(&page_buffer)
                .map_err(|e| format!("Failed to write to output: {}", e))?;

            FPDFText_ClosePage(text_page);
            FPDF_ClosePage(page);
        }

        FPDF_CloseDocument(doc);
        FPDF_DestroyLibrary();

        Ok(())
    }
}

// ========================================
// Multi-process implementation
// ========================================

fn extract_text_multiprocess(
    pdf_path: &str,
    output_path: &str,
    worker_count: usize,
    page_count: i32,
) -> Result<(), String> {
    // Distribute pages across workers
    let pages_per_worker = (page_count as usize).div_ceil(worker_count);
    let mut worker_processes = vec![];
    let mut temp_files = vec![];

    // Spawn worker processes
    for worker_id in 0..worker_count {
        let start_page = worker_id * pages_per_worker;
        let end_page = ((worker_id + 1) * pages_per_worker).min(page_count as usize);

        if start_page >= page_count as usize {
            break;
        }

        // Create temp file for this worker's output
        let temp_path = format!(
            "/tmp/pdfium_worker_{}_{}.bin",
            std::process::id(),
            worker_id
        );
        temp_files.push(temp_path.clone());

        // Spawn worker process
        let child = Command::new(std::env::current_exe().unwrap())
            .arg("--worker")
            .arg(pdf_path)
            .arg(&temp_path)
            .arg(start_page.to_string())
            .arg(end_page.to_string())
            .arg(worker_id.to_string())
            .arg("text")
            .spawn()
            .map_err(|e| format!("Failed to spawn worker {}: {}", worker_id, e))?;

        worker_processes.push(child);
    }

    // Wait for all workers to complete
    for (worker_id, mut child) in worker_processes.into_iter().enumerate() {
        let status = child
            .wait()
            .map_err(|e| format!("Worker {} failed: {}", worker_id, e))?;

        if !status.success() {
            return Err(format!(
                "Worker {} exited with error: {}",
                worker_id, status
            ));
        }
    }

    // Combine results in page order
    let mut output_file =
        File::create(output_path).map_err(|e| format!("Failed to create output file: {}", e))?;

    // Write UTF-32 LE BOM
    output_file
        .write_all(&[0xFF, 0xFE, 0x00, 0x00])
        .map_err(|e| format!("Failed to write BOM: {}", e))?;

    // Read and write each worker's output (already in page order within each file)
    for temp_file in temp_files.iter() {
        let mut file = File::open(temp_file)
            .map_err(|e| format!("Failed to open worker output {}: {}", temp_file, e))?;

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .map_err(|e| format!("Failed to read worker output: {}", e))?;

        output_file
            .write_all(&buffer)
            .map_err(|e| format!("Failed to write to output: {}", e))?;

        // Clean up temp file
        let _ = std::fs::remove_file(temp_file);
    }

    Ok(())
}

// ========================================
// Worker process (for multi-process mode)
// ========================================

fn worker_main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 8 {
        eprintln!(
            "Worker usage: --worker <pdf> <output> <start_page> <end_page> <worker_id> <format>"
        );
        process::exit(1);
    }

    let pdf_path = &args[2];
    let output_path = &args[3];
    let start_page: usize = args[4].parse().unwrap();
    let end_page: usize = args[5].parse().unwrap();
    let worker_id: usize = args[6].parse().unwrap();
    let format = &args[7];

    let result = match format.as_str() {
        "text" => extract_pages(pdf_path, output_path, start_page, end_page, worker_id),
        "jsonl" => extract_pages_jsonl(pdf_path, output_path, start_page, end_page, worker_id),
        _ => Err(format!("Unknown format: {}", format)),
    };

    match result {
        Ok(_) => process::exit(0),
        Err(e) => {
            eprintln!("Worker error: {}", e);
            process::exit(1);
        }
    }
}

fn extract_pages(
    pdf_path: &str,
    output_path: &str,
    start_page: usize,
    end_page: usize,
    worker_id: usize,
) -> Result<(), String> {
    unsafe {
        // Each worker has its own PDFium instance - NO SHARED STATE
        FPDF_InitLibrary();

        let c_path = CString::new(pdf_path).unwrap();
        let doc = FPDF_LoadDocument(c_path.as_ptr(), std::ptr::null());

        if doc.is_null() {
            FPDF_DestroyLibrary();
            return Err(format!("Failed to load PDF: {}", pdf_path));
        }

        let mut output_file = File::create(output_path)
            .map_err(|e| format!("Failed to create output file: {}", e))?;

        // Reusable page buffer (Task 2.3: Buffer Pooling)
        let mut page_buffer = Vec::with_capacity(256 * 1024); // 256KB initial capacity

        // Process assigned pages
        for page_index in start_page..end_page {
            let page = FPDF_LoadPage(doc, page_index as i32);
            if page.is_null() {
                eprintln!("Warning: Failed to load page {}", page_index);
                continue;
            }

            let text_page = FPDFText_LoadPage(page);
            if text_page.is_null() {
                FPDF_ClosePage(page);
                eprintln!("Warning: Failed to load page {}", page_index);
                continue;
            }

            let char_count = FPDFText_CountChars(text_page);

            // Clear buffer for reuse (retains capacity)
            page_buffer.clear();
            // Ensure capacity for this page (includes BOM: 4 bytes + char_count * 4)
            let required_capacity = 4 + (char_count * 4) as usize;
            if page_buffer.capacity() < required_capacity {
                page_buffer.reserve(required_capacity - page_buffer.len());
            }

            // Add BOM for this page
            page_buffer.extend_from_slice(&[0xFF, 0xFE, 0x00, 0x00]);

            // Extract characters
            let mut i = 0;
            while i < char_count {
                let code_unit = FPDFText_GetUnicode(text_page, i);

                let codepoint = if (0xD800..=0xDBFF).contains(&code_unit) {
                    i += 1;
                    if i < char_count {
                        let low_surrogate = FPDFText_GetUnicode(text_page, i);
                        ((code_unit - 0xD800) << 10) + (low_surrogate - 0xDC00) + 0x10000
                    } else {
                        code_unit
                    }
                } else {
                    code_unit
                };

                page_buffer.extend_from_slice(&codepoint.to_le_bytes());
                i += 1;
            }

            // Write page to output
            // Worker 0's first page skips BOM (controller adds file-level BOM)
            // All other pages include BOMs
            if worker_id == 0 && page_index == start_page {
                output_file
                    .write_all(&page_buffer[4..])
                    .map_err(|e| format!("Failed to write page {}: {}", page_index, e))?;
            } else {
                output_file
                    .write_all(&page_buffer)
                    .map_err(|e| format!("Failed to write page {}: {}", page_index, e))?;
            }

            FPDFText_ClosePage(text_page);
            FPDF_ClosePage(page);
        }

        FPDF_CloseDocument(doc);
        FPDF_DestroyLibrary();

        Ok(())
    }
}

// ========================================
// JSONL worker process (for multi-process mode)
// ========================================

fn extract_pages_jsonl(
    pdf_path: &str,
    output_path: &str,
    start_page: usize,
    end_page: usize,
    _worker_id: usize,
) -> Result<(), String> {
    unsafe {
        // Each worker has its own PDFium instance - NO SHARED STATE
        FPDF_InitLibrary();

        let c_path = CString::new(pdf_path).unwrap();
        let doc = FPDF_LoadDocument(c_path.as_ptr(), std::ptr::null());

        if doc.is_null() {
            FPDF_DestroyLibrary();
            return Err(format!("Failed to load PDF: {}", pdf_path));
        }

        let mut output_file = File::create(output_path)
            .map_err(|e| format!("Failed to create output file: {}", e))?;

        // Process assigned pages
        for page_index in start_page..end_page {
            let page = FPDF_LoadPage(doc, page_index as i32);
            if page.is_null() {
                eprintln!("Warning: Failed to load page {}", page_index);
                continue;
            }

            // Write page record
            let page_width = FPDF_GetPageWidth(page);
            let page_height = FPDF_GetPageHeight(page);

            let page_record = JsonlRecord::Page {
                page: page_index as i32,
                width: page_width as f64,
                height: page_height as f64,
            };

            writeln!(
                output_file,
                "{}",
                serde_json::to_string(&page_record).unwrap()
            )
            .map_err(|e| format!("Failed to write page record: {}", e))?;

            let text_page = FPDFText_LoadPage(page);
            if text_page.is_null() {
                FPDF_ClosePage(page);
                eprintln!("Warning: Failed to load text for page {}", page_index);
                continue;
            }

            let char_count = FPDFText_CountChars(text_page);

            // Extract each character with rich annotations
            let mut i = 0;
            while i < char_count {
                let code_unit = FPDFText_GetUnicode(text_page, i);

                // Handle UTF-16 surrogate pairs
                let unicode = if (0xD800..=0xDBFF).contains(&code_unit) {
                    i += 1;
                    if i < char_count {
                        let low_surrogate = FPDFText_GetUnicode(text_page, i);
                        ((code_unit - 0xD800) << 10) + (low_surrogate - 0xDC00) + 0x10000
                    } else {
                        code_unit
                    }
                } else {
                    code_unit
                };

                // Get character string
                let char_str = char::from_u32(unicode)
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| format!("\\u{{{:x}}}", unicode));

                // Get bounding box
                let (mut left, mut bottom, mut right, mut top) = (0.0, 0.0, 0.0, 0.0);
                FPDFText_GetCharBox(text_page, i, &mut left, &mut bottom, &mut right, &mut top);

                // Get origin
                let (mut origin_x, mut origin_y) = (0.0, 0.0);
                FPDFText_GetCharOrigin(text_page, i, &mut origin_x, &mut origin_y);

                // Get font info
                let font_size = FPDFText_GetFontSize(text_page, i);
                let font_weight = FPDFText_GetFontWeight(text_page, i);

                let mut font_name_buf = vec![0u8; 256];
                let mut flags: i32 = 0;
                let name_len = FPDFText_GetFontInfo(
                    text_page,
                    i,
                    font_name_buf.as_mut_ptr() as *mut _,
                    256,
                    &mut flags,
                );

                let font_name = if name_len > 0 {
                    String::from_utf8_lossy(&font_name_buf[0..(name_len as usize).min(255)])
                        .trim_end_matches('\0')
                        .to_string()
                } else {
                    "Unknown".to_string()
                };

                // Get colors
                let (mut r, mut g, mut b, mut a) = (0, 0, 0, 255);
                FPDFText_GetFillColor(text_page, i, &mut r, &mut g, &mut b, &mut a);
                let fill_color = format!("#{:02x}{:02x}{:02x}{:02x}", r, g, b, a);

                FPDFText_GetStrokeColor(text_page, i, &mut r, &mut g, &mut b, &mut a);
                let stroke_color = format!("#{:02x}{:02x}{:02x}{:02x}", r, g, b, a);

                // Get angle
                let angle = FPDFText_GetCharAngle(text_page, i);

                // Get flags
                let generated = FPDFText_IsGenerated(text_page, i) != 0;
                let hyphen = FPDFText_IsHyphen(text_page, i) != 0;
                let unicode_error = FPDFText_HasUnicodeMapError(text_page, i) != 0;

                // Build character record
                let char_record = JsonlRecord::Char {
                    page: page_index as i32,
                    index: i,
                    character: char_str,
                    unicode,
                    bbox: BBox {
                        x: left,
                        y: bottom,
                        width: right - left,
                        height: top - bottom,
                    },
                    origin: Point {
                        x: origin_x,
                        y: origin_y,
                    },
                    angle,
                    font: Font {
                        name: font_name,
                        size: font_size as f64,
                        weight: font_weight,
                    },
                    color: Color {
                        fill: fill_color,
                        stroke: stroke_color,
                    },
                    flags: Flags {
                        generated,
                        hyphen,
                        unicode_error,
                    },
                };

                // Write character record
                writeln!(
                    output_file,
                    "{}",
                    serde_json::to_string(&char_record).unwrap()
                )
                .map_err(|e| format!("Failed to write character record: {}", e))?;

                i += 1;
            }

            FPDFText_ClosePage(text_page);
            FPDF_ClosePage(page);
        }

        FPDF_CloseDocument(doc);
        FPDF_DestroyLibrary();

        Ok(())
    }
}

// ========================================
// JSONL extraction implementation
// ========================================

fn extract_jsonl_single_threaded(pdf_path: &str, output_path: &str) -> Result<(), String> {
    unsafe {
        FPDF_InitLibrary();

        let c_path = CString::new(pdf_path).unwrap();
        let doc = FPDF_LoadDocument(c_path.as_ptr(), std::ptr::null());

        if doc.is_null() {
            FPDF_DestroyLibrary();
            return Err(format!("Failed to load PDF: {}", pdf_path));
        }

        let mut output_file = File::create(output_path)
            .map_err(|e| format!("Failed to create output file: {}", e))?;

        let page_count = FPDF_GetPageCount(doc);

        // Write metadata record
        let pdf_filename = Path::new(pdf_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown.pdf");

        let metadata = JsonlRecord::Metadata {
            pdf: pdf_filename.to_string(),
            pages: page_count,
            version: "1.0".to_string(),
            created: Utc::now().to_rfc3339(),
        };

        writeln!(output_file, "{}", serde_json::to_string(&metadata).unwrap())
            .map_err(|e| format!("Failed to write metadata: {}", e))?;

        // Process each page
        for page_index in 0..page_count {
            let page = FPDF_LoadPage(doc, page_index);
            if page.is_null() {
                eprintln!("Warning: Failed to load page {}", page_index);
                continue;
            }

            // Write page record
            let page_width = FPDF_GetPageWidth(page);
            let page_height = FPDF_GetPageHeight(page);

            let page_record = JsonlRecord::Page {
                page: page_index,
                width: page_width as f64,
                height: page_height as f64,
            };

            writeln!(
                output_file,
                "{}",
                serde_json::to_string(&page_record).unwrap()
            )
            .map_err(|e| format!("Failed to write page record: {}", e))?;

            let text_page = FPDFText_LoadPage(page);
            if text_page.is_null() {
                FPDF_ClosePage(page);
                eprintln!("Warning: Failed to load text for page {}", page_index);
                continue;
            }

            let char_count = FPDFText_CountChars(text_page);

            // Extract each character with rich annotations
            let mut i = 0;
            while i < char_count {
                let code_unit = FPDFText_GetUnicode(text_page, i);

                // Handle UTF-16 surrogate pairs
                let unicode = if (0xD800..=0xDBFF).contains(&code_unit) {
                    // High surrogate - read low surrogate
                    i += 1;
                    if i < char_count {
                        let low_surrogate = FPDFText_GetUnicode(text_page, i);
                        ((code_unit - 0xD800) << 10) + (low_surrogate - 0xDC00) + 0x10000
                    } else {
                        code_unit
                    }
                } else {
                    code_unit
                };

                // Get character string
                let char_str = char::from_u32(unicode)
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| format!("\\u{{{:x}}}", unicode));

                // Get bounding box
                let (mut left, mut bottom, mut right, mut top) = (0.0, 0.0, 0.0, 0.0);
                FPDFText_GetCharBox(text_page, i, &mut left, &mut bottom, &mut right, &mut top);

                // Get origin
                let (mut origin_x, mut origin_y) = (0.0, 0.0);
                FPDFText_GetCharOrigin(text_page, i, &mut origin_x, &mut origin_y);

                // Get font info
                let font_size = FPDFText_GetFontSize(text_page, i);
                let font_weight = FPDFText_GetFontWeight(text_page, i);

                let mut font_name_buf = vec![0u8; 256];
                let mut flags: i32 = 0;
                let name_len = FPDFText_GetFontInfo(
                    text_page,
                    i,
                    font_name_buf.as_mut_ptr() as *mut _,
                    256,
                    &mut flags,
                );

                let font_name = if name_len > 0 {
                    String::from_utf8_lossy(&font_name_buf[0..(name_len as usize).min(255)])
                        .trim_end_matches('\0')
                        .to_string()
                } else {
                    "Unknown".to_string()
                };

                // Get colors
                let (mut r, mut g, mut b, mut a) = (0, 0, 0, 255);
                FPDFText_GetFillColor(text_page, i, &mut r, &mut g, &mut b, &mut a);
                let fill_color = format!("#{:02x}{:02x}{:02x}{:02x}", r, g, b, a);

                FPDFText_GetStrokeColor(text_page, i, &mut r, &mut g, &mut b, &mut a);
                let stroke_color = format!("#{:02x}{:02x}{:02x}{:02x}", r, g, b, a);

                // Get angle
                let angle = FPDFText_GetCharAngle(text_page, i);

                // Get flags
                let generated = FPDFText_IsGenerated(text_page, i) != 0;
                let hyphen = FPDFText_IsHyphen(text_page, i) != 0;
                let unicode_error = FPDFText_HasUnicodeMapError(text_page, i) != 0;

                // Build character record
                let char_record = JsonlRecord::Char {
                    page: page_index,
                    index: i,
                    character: char_str,
                    unicode,
                    bbox: BBox {
                        x: left,
                        y: bottom,
                        width: right - left,
                        height: top - bottom,
                    },
                    origin: Point {
                        x: origin_x,
                        y: origin_y,
                    },
                    angle,
                    font: Font {
                        name: font_name,
                        size: font_size as f64,
                        weight: font_weight,
                    },
                    color: Color {
                        fill: fill_color,
                        stroke: stroke_color,
                    },
                    flags: Flags {
                        generated,
                        hyphen,
                        unicode_error,
                    },
                };

                // Write character record
                writeln!(
                    output_file,
                    "{}",
                    serde_json::to_string(&char_record).unwrap()
                )
                .map_err(|e| format!("Failed to write character record: {}", e))?;

                i += 1;
            }

            FPDFText_ClosePage(text_page);
            FPDF_ClosePage(page);
        }

        FPDF_CloseDocument(doc);
        FPDF_DestroyLibrary();

        Ok(())
    }
}

fn extract_jsonl_multiprocess(
    pdf_path: &str,
    output_path: &str,
    worker_count: usize,
    page_count: i32,
) -> Result<(), String> {
    // Distribute pages across workers
    let pages_per_worker = (page_count as usize).div_ceil(worker_count);
    let mut worker_processes = vec![];
    let mut temp_files = vec![];

    // Spawn worker processes
    for worker_id in 0..worker_count {
        let start_page = worker_id * pages_per_worker;
        let end_page = ((worker_id + 1) * pages_per_worker).min(page_count as usize);

        if start_page >= page_count as usize {
            break;
        }

        // Create temp file for this worker's output
        let temp_path = format!(
            "/tmp/pdfium_worker_{}_{}.jsonl",
            std::process::id(),
            worker_id
        );
        temp_files.push(temp_path.clone());

        // Spawn worker process
        let child = Command::new(std::env::current_exe().unwrap())
            .arg("--worker")
            .arg(pdf_path)
            .arg(&temp_path)
            .arg(start_page.to_string())
            .arg(end_page.to_string())
            .arg(worker_id.to_string())
            .arg("jsonl")
            .spawn()
            .map_err(|e| format!("Failed to spawn worker {}: {}", worker_id, e))?;

        worker_processes.push(child);
    }

    // Wait for all workers to complete
    for (worker_id, mut child) in worker_processes.into_iter().enumerate() {
        let status = child
            .wait()
            .map_err(|e| format!("Worker {} failed: {}", worker_id, e))?;

        if !status.success() {
            return Err(format!(
                "Worker {} exited with error: {}",
                worker_id, status
            ));
        }
    }

    // Combine results in page order
    let mut output_file =
        File::create(output_path).map_err(|e| format!("Failed to create output file: {}", e))?;

    // Write metadata record first
    let pdf_filename = Path::new(pdf_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown.pdf");

    let metadata = JsonlRecord::Metadata {
        pdf: pdf_filename.to_string(),
        pages: page_count,
        version: "1.0".to_string(),
        created: Utc::now().to_rfc3339(),
    };

    writeln!(output_file, "{}", serde_json::to_string(&metadata).unwrap())
        .map_err(|e| format!("Failed to write metadata: {}", e))?;

    // Concatenate worker outputs line-by-line (JSONL format)
    for temp_file in temp_files.iter() {
        let file = File::open(temp_file)
            .map_err(|e| format!("Failed to open worker output {}: {}", temp_file, e))?;

        use std::io::BufRead;
        let reader = std::io::BufReader::new(file);

        for line in reader.lines() {
            let line = line.map_err(|e| format!("Failed to read worker output: {}", e))?;
            writeln!(output_file, "{}", line)
                .map_err(|e| format!("Failed to write to output: {}", e))?;
        }

        // Clean up temp file
        let _ = std::fs::remove_file(temp_file);
    }

    Ok(())
}
