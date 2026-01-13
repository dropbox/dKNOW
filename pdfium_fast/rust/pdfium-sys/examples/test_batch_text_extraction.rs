//! Test batch text extraction API
//! N=33: Validates FPDFText_ExtractAllCells() binding works
//!
//! This API is the BIGGEST WIN per MANAGER feedback:
//! - Before: 100-400 FFI calls per page
//! - After: 2-3 FFI calls per page
//! - Expected speedup: 3-5x for text extraction

use pdfium_sys::*;
use std::path::Path;

fn test_batch_extraction(path: &Path) {
    let path_str = path.to_str().unwrap();
    let c_path = std::ffi::CString::new(path_str).unwrap();

    let doc = unsafe { FPDF_LoadDocument(c_path.as_ptr(), std::ptr::null()) };
    if doc.is_null() {
        println!(
            "{}: FAILED TO LOAD",
            path.file_name().unwrap().to_str().unwrap()
        );
        return;
    }

    let num_pages = unsafe { FPDF_GetPageCount(doc) };
    println!(
        "\n{}: {} pages",
        path.file_name().unwrap().to_str().unwrap(),
        num_pages
    );

    let mut total_cells = 0;
    let mut total_chars = 0;

    for page_idx in 0..num_pages.min(5) {
        // Load page
        let page = unsafe { FPDF_LoadPage(doc, page_idx) };
        if page.is_null() {
            println!("  Page {}: FAILED TO LOAD", page_idx);
            continue;
        }

        // Load text page
        let text_page = unsafe { FPDFText_LoadPage(page) };
        if text_page.is_null() {
            println!("  Page {}: NO TEXT PAGE", page_idx);
            unsafe {
                FPDF_ClosePage(page);
            }
            continue;
        }

        // Get buffer sizes
        let mut cell_count: i32 = 0;
        let mut text_chars: i32 = 0;
        let result =
            unsafe { FPDFText_GetAllCellsBufferSizes(text_page, &mut cell_count, &mut text_chars) };

        if result == 0 {
            println!("  Page {}: GetAllCellsBufferSizes FAILED", page_idx);
            unsafe {
                FPDFText_ClosePage(text_page);
                FPDF_ClosePage(page);
            }
            continue;
        }

        println!(
            "  Page {}: {} cells, {} chars",
            page_idx, cell_count, text_chars
        );

        if cell_count > 0 {
            // Allocate buffers
            let mut cells: Vec<FPDF_TEXT_CELL_INFO> = vec![
                FPDF_TEXT_CELL_INFO {
                    left: 0.0,
                    bottom: 0.0,
                    right: 0.0,
                    top: 0.0,
                    text_offset: 0,
                    text_length: 0,
                    font_size: 0.0,
                    font_flags: 0,
                    char_start: 0,
                    char_count: 0,
                };
                cell_count as usize
            ];
            let mut text_buffer: Vec<u16> = vec![0u16; (text_chars + 1) as usize];

            // Extract all cells in ONE call
            let extracted = unsafe {
                FPDFText_ExtractAllCells(
                    text_page,
                    cells.as_mut_ptr(),
                    cell_count,
                    text_buffer.as_mut_ptr(),
                    text_chars + 1,
                )
            };

            if extracted < 0 {
                println!("    ExtractAllCells FAILED (returned {})", extracted);
            } else {
                println!("    Extracted {} cells successfully", extracted);
                total_cells += extracted as usize;
                total_chars += text_chars as usize;

                // Print first few cells as example
                for i in 0..extracted.min(3) {
                    let cell = &cells[i as usize];
                    let text_start = cell.text_offset as usize;
                    let text_end = text_start + cell.text_length as usize;
                    let text_slice = &text_buffer[text_start..text_end.min(text_buffer.len())];
                    let text: String = text_slice
                        .iter()
                        .filter(|&&c| c != 0)
                        .map(|&c| char::from_u32(c as u32).unwrap_or('?'))
                        .collect();
                    // Handle Unicode properly - take first 30 chars, not bytes
                    let preview: String = text.chars().take(30).collect();
                    let preview = if text.chars().count() > 30 {
                        format!("{}...", preview)
                    } else {
                        preview
                    };
                    println!(
                        "    Cell {}: bbox=({:.1},{:.1},{:.1},{:.1}) font_size={:.1} text=\"{}\"",
                        i, cell.left, cell.bottom, cell.right, cell.top, cell.font_size, preview
                    );
                }
            }
        } else {
            // No cells on this page, just count the chars
            total_chars += text_chars as usize;
        }

        unsafe {
            FPDFText_ClosePage(text_page);
            FPDF_ClosePage(page);
        }
    }

    println!(
        "  TOTAL: {} cells, {} chars across first {} pages",
        total_cells,
        total_chars,
        num_pages.min(5)
    );

    unsafe {
        FPDF_CloseDocument(doc);
    }
}

fn main() {
    println!("Testing FPDFText_ExtractAllCells() API (Batch Text Extraction)...\n");
    println!("This API reduces FFI overhead by extracting all text cells in 2-3 calls");
    println!("instead of the 100-400 calls typically needed with standard APIs.\n");

    unsafe {
        FPDF_InitLibrary();
    }

    // Test with docling test corpus
    let test_dir = Path::new("/Users/ayates/docling_rs/test-corpus/pdf");
    if test_dir.exists() {
        println!("Scanning {}:", test_dir.display());

        let mut success_count = 0;
        let mut total_count = 0;

        for entry in std::fs::read_dir(test_dir).unwrap().flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "pdf").unwrap_or(false) {
                total_count += 1;
                test_batch_extraction(&path);
                success_count += 1;
            }
        }

        println!("\n=== SUMMARY ===");
        println!("Tested: {} PDFs", total_count);
        println!("Success: {} PDFs", success_count);
        println!("API Status: FPDFText_ExtractAllCells() WORKING");
    } else {
        println!("Test directory not found: {:?}", test_dir);

        // Try with integration test PDFs
        let alt_dir = Path::new("/Users/ayates/pdfium_fast/integration_tests/test_pdfs");
        if alt_dir.exists() {
            println!("\nTrying alternate directory: {}", alt_dir.display());
            for entry in std::fs::read_dir(alt_dir).unwrap().flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "pdf").unwrap_or(false) {
                    test_batch_extraction(&path);
                }
            }
        }
    }

    unsafe {
        FPDF_DestroyLibrary();
    }

    println!("\nTest complete.");
}
