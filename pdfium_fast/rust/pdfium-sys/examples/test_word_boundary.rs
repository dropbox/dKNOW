//! Test word boundary extraction API (P9)
//! N=106: Validates FPDFText_CountWords() and FPDFText_ExtractWords() bindings
//!
//! This API extracts words with bounding boxes for advanced text analysis.

use pdfium_sys::*;
use std::path::Path;

fn test_word_extraction(path: &Path) -> bool {
    let path_str = path.to_str().unwrap();
    let c_path = std::ffi::CString::new(path_str).unwrap();

    let doc = unsafe { FPDF_LoadDocument(c_path.as_ptr(), std::ptr::null()) };
    if doc.is_null() {
        println!(
            "  {}: FAILED TO LOAD",
            path.file_name().unwrap().to_str().unwrap()
        );
        return false;
    }

    let num_pages = unsafe { FPDF_GetPageCount(doc) };
    let file_name = path.file_name().unwrap().to_str().unwrap();

    // Test first page only
    let page = unsafe { FPDF_LoadPage(doc, 0) };
    if page.is_null() {
        println!("  {}: Page 0 FAILED TO LOAD", file_name);
        unsafe {
            FPDF_CloseDocument(doc);
        }
        return false;
    }

    let text_page = unsafe { FPDFText_LoadPage(page) };
    if text_page.is_null() {
        println!("  {}: NO TEXT PAGE", file_name);
        unsafe {
            FPDF_ClosePage(page);
            FPDF_CloseDocument(doc);
        }
        return false;
    }

    // Count words
    let word_count = unsafe { FPDFText_CountWords(text_page) };
    if word_count < 0 {
        println!(
            "  {}: FPDFText_CountWords FAILED (returned {})",
            file_name, word_count
        );
        unsafe {
            FPDFText_ClosePage(text_page);
            FPDF_ClosePage(page);
            FPDF_CloseDocument(doc);
        }
        return false;
    }

    // Count chars for text buffer
    let char_count = unsafe { FPDFText_CountChars(text_page) };

    if word_count == 0 {
        println!(
            "  {}: {} pages, 0 words (empty or scanned PDF)",
            file_name, num_pages
        );
        unsafe {
            FPDFText_ClosePage(text_page);
            FPDF_ClosePage(page);
            FPDF_CloseDocument(doc);
        }
        return true; // Valid case
    }

    // Allocate buffers
    let mut words: Vec<FPDF_WORD_INFO> = vec![
        FPDF_WORD_INFO {
            left: 0.0,
            bottom: 0.0,
            right: 0.0,
            top: 0.0,
            start_char: 0,
            end_char: 0,
            text_offset: 0,
            text_length: 0,
        };
        word_count as usize
    ];
    let mut text_buffer: Vec<u16> = vec![0u16; (char_count + 1) as usize];

    // Extract words
    let extracted = unsafe {
        FPDFText_ExtractWords(
            text_page,
            words.as_mut_ptr(),
            word_count,
            text_buffer.as_mut_ptr(),
            char_count + 1,
        )
    };

    if extracted < 0 {
        println!(
            "  {}: FPDFText_ExtractWords FAILED (returned {})",
            file_name, extracted
        );
        unsafe {
            FPDFText_ClosePage(text_page);
            FPDF_ClosePage(page);
            FPDF_CloseDocument(doc);
        }
        return false;
    }

    println!(
        "  {}: {} pages, {} words extracted from page 0",
        file_name, num_pages, extracted
    );

    // Validate bounding boxes
    let mut valid_boxes = 0;
    for i in 0..extracted.min(5) {
        let word = &words[i as usize];
        let bbox_valid = word.left <= word.right && word.bottom <= word.top;
        if bbox_valid {
            valid_boxes += 1;
        }

        // Get word text
        let text_start = word.text_offset as usize;
        let text_end = text_start + word.text_length as usize;
        if text_end <= text_buffer.len() {
            let text_slice = &text_buffer[text_start..text_end];
            let text: String = text_slice
                .iter()
                .filter(|&&c| c != 0)
                .map(|&c| char::from_u32(c as u32).unwrap_or('?'))
                .collect();
            let preview: String = text.chars().take(20).collect();

            if i < 3 {
                println!(
                    "    Word {}: \"{}\" bbox=({:.1},{:.1},{:.1},{:.1})",
                    i, preview, word.left, word.bottom, word.right, word.top
                );
            }
        }
    }

    println!(
        "    First {} words: {}/{} have valid bboxes",
        extracted.min(5),
        valid_boxes,
        extracted.min(5)
    );

    unsafe {
        FPDFText_ClosePage(text_page);
        FPDF_ClosePage(page);
        FPDF_CloseDocument(doc);
    }

    true
}

fn main() {
    println!("Testing Word Boundary API (P9)");
    println!("==============================");
    println!("APIs: FPDFText_CountWords(), FPDFText_ExtractWords()\n");

    unsafe {
        FPDF_InitLibrary();
    }

    let mut success_count = 0;
    let mut total_count = 0;

    // Test with docling test corpus
    let test_dir = Path::new("/Users/ayates/docling_rs/test-corpus/pdf");
    if test_dir.exists() {
        println!("Scanning {}:\n", test_dir.display());

        for entry in std::fs::read_dir(test_dir).unwrap().flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "pdf").unwrap_or(false) {
                total_count += 1;
                if test_word_extraction(&path) {
                    success_count += 1;
                }
            }
        }
    } else {
        // Fall back to edge_cases directory
        let alt_dir = Path::new("/Users/ayates/pdfium_fast/integration_tests/pdfs/edge_cases");
        if alt_dir.exists() {
            println!("Scanning {}:\n", alt_dir.display());

            for entry in std::fs::read_dir(alt_dir).unwrap().take(10).flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "pdf").unwrap_or(false) {
                    total_count += 1;
                    if test_word_extraction(&path) {
                        success_count += 1;
                    }
                }
            }
        }
    }

    println!("\n=== SUMMARY ===");
    println!("Tested: {} PDFs", total_count);
    println!("Success: {} PDFs", success_count);
    println!("Failed: {} PDFs", total_count - success_count);

    if success_count == total_count && total_count > 0 {
        println!("\nWord Boundary Test: PASS");
        println!("FPDFText_CountWords(): WORKING");
        println!("FPDFText_ExtractWords(): WORKING");
    } else if success_count > 0 {
        println!(
            "\nWord Boundary Test: PARTIAL PASS ({}/{})",
            success_count, total_count
        );
    } else {
        println!("\nWord Boundary Test: FAIL");
    }

    unsafe {
        FPDF_DestroyLibrary();
    }
}
