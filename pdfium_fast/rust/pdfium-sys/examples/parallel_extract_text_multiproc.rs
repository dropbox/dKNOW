use pdfium_sys::*;
use std::env;
use std::ffi::CString;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{self, Command};

fn main() {
    let args: Vec<String> = env::args().collect();

    // Check if this is a worker process
    if args.len() >= 2 && args[1] == "--worker" {
        worker_main();
        return;
    }

    // Controller process
    if args.len() < 3 || args.len() > 4 {
        eprintln!("Usage: {} <input.pdf> <output.txt> [worker_count]", args[0]);
        eprintln!("  worker_count: 1-16 (default: 4)");
        process::exit(1);
    }

    let pdf_path = &args[1];
    let output_path = &args[2];
    let worker_count = if args.len() == 4 {
        args[3].parse::<usize>().unwrap_or_else(|_| {
            eprintln!("Error: worker_count must be a number between 1 and 16");
            process::exit(1);
        })
    } else {
        4
    };

    if !(1..=16).contains(&worker_count) {
        eprintln!("Error: worker_count must be between 1 and 16");
        process::exit(1);
    }

    if !Path::new(pdf_path).exists() {
        eprintln!("Error: PDF file not found: {}", pdf_path);
        process::exit(1);
    }

    match controller_main(pdf_path, output_path, worker_count) {
        Ok(_) => process::exit(0),
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
}

fn controller_main(pdf_path: &str, output_path: &str, worker_count: usize) -> Result<(), String> {
    unsafe {
        // Initialize PDFium just to get page count
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

        // Allow 0-page PDFs (upstream behavior: output BOM only)
        if page_count == 0 {
            let mut output_file = File::create(output_path)
                .map_err(|e| format!("Failed to create output file: {}", e))?;
            output_file
                .write_all(&[0xFF, 0xFE, 0x00, 0x00])
                .map_err(|e| format!("Failed to write BOM: {}", e))?;
            return Ok(());
        }

        eprintln!(
            "Processing {} pages with {} workers (multi-process)",
            page_count, worker_count
        );

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
            // Pass worker_id to determine BOM handling
            let child = Command::new(std::env::current_exe().unwrap())
                .arg("--worker")
                .arg(pdf_path)
                .arg(&temp_path)
                .arg(start_page.to_string())
                .arg(end_page.to_string())
                .arg(worker_id.to_string())
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
        let mut output_file = File::create(output_path)
            .map_err(|e| format!("Failed to create output file: {}", e))?;

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

            // Worker 0's first page BOM was skipped (we added one above)
            // All other workers include all their page BOMs
            output_file
                .write_all(&buffer)
                .map_err(|e| format!("Failed to write to output: {}", e))?;

            // Clean up temp file
            let _ = std::fs::remove_file(temp_file);
        }

        Ok(())
    }
}

fn worker_main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 7 {
        eprintln!("Worker usage: --worker <pdf> <output> <start_page> <end_page> <worker_id>");
        process::exit(1);
    }

    let pdf_path = &args[2];
    let output_path = &args[3];
    let start_page: usize = args[4].parse().unwrap();
    let end_page: usize = args[5].parse().unwrap();
    let worker_id: usize = args[6].parse().unwrap();

    match extract_pages(pdf_path, output_path, start_page, end_page, worker_id) {
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
                eprintln!("Warning: Failed to load text for page {}", page_index);
                continue;
            }

            let char_count = FPDFText_CountChars(text_page);

            // Build page buffer
            let mut page_buffer = Vec::with_capacity((char_count * 4) as usize);

            // Add BOM for this page (will be stripped/handled by controller for page 0)
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
            // Only worker 0's first page skips BOM (controller adds file-level BOM)
            // All other pages (including other workers' first pages) include BOMs
            if worker_id == 0 && page_index == start_page {
                // Worker 0, first page: skip BOM
                output_file
                    .write_all(&page_buffer[4..])
                    .map_err(|e| format!("Failed to write page {}: {}", page_index, e))?;
            } else {
                // All other pages: include BOM
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
