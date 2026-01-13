use pdfium_sys::*;
use std::env;
use std::ffi::CString;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process;
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let args: Vec<String> = env::args().collect();

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

    // Check input file exists
    if !Path::new(pdf_path).exists() {
        eprintln!("Error: PDF file not found: {}", pdf_path);
        process::exit(1);
    }

    // Extract text in parallel
    match parallel_extract_text(pdf_path, output_path, worker_count) {
        Ok(_) => process::exit(0),
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
}

fn parallel_extract_text(
    pdf_path: &str,
    output_path: &str,
    worker_count: usize,
) -> Result<(), String> {
    unsafe {
        // Initialize PDFium library (single-threaded, before any parallel work)
        FPDF_InitLibrary();

        // Load document (single-threaded)
        let c_path = CString::new(pdf_path).unwrap();
        let doc = FPDF_LoadDocument(c_path.as_ptr(), std::ptr::null());

        if doc.is_null() {
            FPDF_DestroyLibrary();
            return Err(format!("Failed to load PDF: {}", pdf_path));
        }

        // Get page count (read-only operation)
        let page_count = FPDF_GetPageCount(doc);

        // Allow 0-page PDFs (upstream behavior: output BOM only)
        if page_count == 0 {
            // Write BOM and exit successfully
            let mut output_file = File::create(output_path)
                .map_err(|e| format!("Failed to create output file: {}", e))?;
            output_file
                .write_all(&[0xFF, 0xFE, 0x00, 0x00])
                .map_err(|e| format!("Failed to write BOM: {}", e))?;
            FPDF_CloseDocument(doc);
            FPDF_DestroyLibrary();
            return Ok(());
        }

        eprintln!(
            "Processing {} pages with {} workers",
            page_count, worker_count
        );

        // OPTIMIZATION: Pre-load all pages before parallel phase to reduce mutex contention
        // This dramatically improves parallel efficiency by moving expensive FPDF_LoadPage
        // calls out of the per-worker critical section
        let mut pages = Vec::with_capacity(page_count as usize);
        for page_index in 0..page_count {
            let page = FPDF_LoadPage(doc, page_index);
            if page.is_null() {
                // Clean up already-loaded pages
                for p in pages {
                    FPDF_ClosePage(p);
                }
                FPDF_CloseDocument(doc);
                FPDF_DestroyLibrary();
                return Err(format!("Failed to load page {}", page_index));
            }
            pages.push(page);
        }

        // CRITICAL: PDFium is not thread-safe. From fpdfview.h:
        // "None of the PDFium APIs are thread-safe. They expect to be called from a single thread.
        //  Barring that, embedders are required to ensure (via a mutex or similar) that only a
        //  single PDFium call can be made at a time."
        //
        // We use a global mutex to serialize PDFium API calls across threads.
        // With pages pre-loaded, workers only need the mutex for FPDFText_* operations.
        let pdfium_mutex = Arc::new(Mutex::new(()));

        // Wrap page array for thread safety
        // SAFETY: We protect all PDFium API access with pdfium_mutex
        struct PageHandle(FPDF_PAGE);
        unsafe impl Send for PageHandle {}
        unsafe impl Sync for PageHandle {}

        let pages: Vec<PageHandle> = pages.into_iter().map(PageHandle).collect();
        let pages = Arc::new(pages);

        // Vector to hold per-page results
        let page_texts = Arc::new(Mutex::new(vec![Vec::<u8>::new(); page_count as usize]));

        // Distribute pages across workers
        let pages_per_worker = (page_count as usize).div_ceil(worker_count);

        let mut handles = vec![];

        for worker_id in 0..worker_count {
            let start_page = worker_id * pages_per_worker;
            let end_page = ((worker_id + 1) * pages_per_worker).min(page_count as usize);

            if start_page >= page_count as usize {
                break;
            }

            let pdfium_mutex = Arc::clone(&pdfium_mutex);
            let page_texts = Arc::clone(&page_texts);
            let pages = Arc::clone(&pages);

            let handle = thread::spawn(move || {
                for page_index in start_page..end_page {
                    // Get pre-loaded page (no lock needed - read-only access)
                    let page = pages[page_index].0;

                    // Phase 1: Load text page and get character count (under lock)
                    let (text_page, char_count) = {
                        let _guard = pdfium_mutex.lock().unwrap();
                        let text_page = FPDFText_LoadPage(page);
                        if text_page.is_null() {
                            eprintln!(
                                "Warning: Worker {} failed to load text for page {}",
                                worker_id, page_index
                            );
                            continue;
                        }
                        let char_count = FPDFText_CountChars(text_page);
                        (text_page, char_count)
                    };
                    // Lock released - other workers can now proceed

                    // Phase 2: Extract all character codes (OPTIMIZED: Hold lock for entire extraction)
                    // Testing showed that extracting all characters in one lock acquisition
                    // is faster than per-character locking due to reduced lock overhead
                    let page_buffer = {
                        let _guard = pdfium_mutex.lock().unwrap();

                        let mut page_buffer = Vec::with_capacity((char_count * 4) as usize);
                        page_buffer.extend_from_slice(&[0xFF, 0xFE, 0x00, 0x00]); // UTF-32 LE BOM

                        // Extract all characters (FPDFText_GetUnicode returns UTF-16 code units)
                        let mut i = 0;
                        while i < char_count {
                            let code_unit = FPDFText_GetUnicode(text_page, i);

                            // Handle UTF-16 surrogate pairs
                            let codepoint = if (0xD800..=0xDBFF).contains(&code_unit) {
                                i += 1;
                                if i < char_count {
                                    let low_surrogate = FPDFText_GetUnicode(text_page, i);
                                    ((code_unit - 0xD800) << 10)
                                        + (low_surrogate - 0xDC00)
                                        + 0x10000
                                } else {
                                    code_unit
                                }
                            } else {
                                code_unit
                            };

                            page_buffer.extend_from_slice(&codepoint.to_le_bytes());
                            i += 1;
                        }

                        page_buffer
                    };
                    // Lock released

                    // Phase 3: Clean up text page (under lock)
                    {
                        let _guard = pdfium_mutex.lock().unwrap();
                        FPDFText_ClosePage(text_page);
                    }

                    // Phase 4: Store result (separate mutex, no PDFium calls)
                    let mut texts = page_texts.lock().unwrap();
                    texts[page_index] = page_buffer;
                }
            });

            handles.push(handle);
        }

        // Wait for all workers to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Clean up pre-loaded pages
        let pages_vec = Arc::try_unwrap(pages).unwrap_or_else(|_arc| {
            // If Arc still has references, this is a programming error
            panic!("Pages Arc still has references after workers completed");
        });
        for page_handle in pages_vec.into_iter() {
            FPDF_ClosePage(page_handle.0);
        }

        // Write results to file in page order
        let mut output_file = File::create(output_path)
            .map_err(|e| format!("Failed to create output file: {}", e))?;

        // Write initial UTF-32 LE BOM
        output_file
            .write_all(&[0xFF, 0xFE, 0x00, 0x00])
            .map_err(|e| format!("Failed to write BOM: {}", e))?;

        let texts = page_texts.lock().unwrap();
        for (page_index, page_buffer) in texts.iter().enumerate() {
            if page_buffer.is_empty() {
                eprintln!("Warning: Page {} produced no text", page_index);
                continue;
            }

            // Write page text (skip the first BOM from page 0, write BOM for pages 1+)
            if page_index == 0 {
                // Page 0: skip the BOM we already included, write the rest
                output_file
                    .write_all(&page_buffer[4..])
                    .map_err(|e| format!("Failed to write page {}: {}", page_index, e))?;
            } else {
                // Pages 1+: write the entire buffer (including BOM)
                output_file
                    .write_all(page_buffer)
                    .map_err(|e| format!("Failed to write page {}: {}", page_index, e))?;
            }
        }

        // Clean up document resources
        FPDF_CloseDocument(doc);
        FPDF_DestroyLibrary();

        Ok(())
    }
}
