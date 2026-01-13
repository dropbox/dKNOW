use pdfium_sys::*;
use std::env;
use std::ffi::CString;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: {} <input.pdf> <output.txt>", args[0]);
        process::exit(1);
    }

    let pdf_path = &args[1];
    let output_path = &args[2];

    // Check input file exists
    if !Path::new(pdf_path).exists() {
        eprintln!("Error: PDF file not found: {}", pdf_path);
        process::exit(1);
    }

    // Extract text
    match extract_text(pdf_path, output_path) {
        Ok(_) => process::exit(0),
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
}

fn extract_text(pdf_path: &str, output_path: &str) -> Result<(), String> {
    unsafe {
        // Initialize PDFium library
        FPDF_InitLibrary();

        // Load document
        let c_path = CString::new(pdf_path).unwrap();
        let doc = FPDF_LoadDocument(c_path.as_ptr(), std::ptr::null());

        if doc.is_null() {
            FPDF_DestroyLibrary();
            return Err(format!("Failed to load PDF: {}", pdf_path));
        }

        // Open output file
        let mut output_file = File::create(output_path)
            .map_err(|e| format!("Failed to create output file: {}", e))?;

        // Write UTF-32 LE BOM (0x0000FEFF in little-endian)
        output_file
            .write_all(&[0xFF, 0xFE, 0x00, 0x00])
            .map_err(|e| format!("Failed to write BOM: {}", e))?;

        // Get page count
        let page_count = FPDF_GetPageCount(doc);

        // Allow 0-page PDFs (upstream behavior: output BOM only)
        // Extract text from each page
        for page_index in 0..page_count {
            // Write UTF-32 LE BOM for each page (matching pdfium_test behavior)
            // pdfium_test writes one .txt file per page with BOM,
            // then concatenates them, so each page gets a BOM
            if page_index > 0 {
                output_file
                    .write_all(&[0xFF, 0xFE, 0x00, 0x00])
                    .map_err(|e| format!("Failed to write BOM: {}", e))?;
            }

            // Load page
            let page = FPDF_LoadPage(doc, page_index);
            if page.is_null() {
                eprintln!("Warning: Failed to load page {}", page_index);
                continue;
            }

            // Load text page
            let text_page = FPDFText_LoadPage(page);
            if text_page.is_null() {
                FPDF_ClosePage(page);
                eprintln!("Warning: Failed to load text for page {}", page_index);
                continue;
            }

            // Get character count
            let char_count = FPDFText_CountChars(text_page);

            // Extract text character by character using FPDFText_GetUnicode
            // FPDFText_GetUnicode returns UTF-16 code units, so we need to handle surrogate pairs
            let mut i = 0;
            while i < char_count {
                let code_unit = FPDFText_GetUnicode(text_page, i);

                // Check if this is a high surrogate (U+D800..U+DBFF)
                let codepoint = if (0xD800..=0xDBFF).contains(&code_unit) {
                    // High surrogate - need to read low surrogate
                    i += 1;
                    if i < char_count {
                        let low_surrogate = FPDFText_GetUnicode(text_page, i);
                        // Combine surrogates into UTF-32 codepoint
                        // Formula: ((H - 0xD800) << 10) + (L - 0xDC00) + 0x10000
                        ((code_unit - 0xD800) << 10) + (low_surrogate - 0xDC00) + 0x10000
                    } else {
                        code_unit // Incomplete pair, keep as-is
                    }
                } else {
                    code_unit
                };

                let bytes = codepoint.to_le_bytes();
                output_file
                    .write_all(&bytes)
                    .map_err(|e| format!("Failed to write to output: {}", e))?;

                i += 1;
            }

            // Clean up page resources
            FPDFText_ClosePage(text_page);
            FPDF_ClosePage(page);
        }

        // Clean up document resources
        FPDF_CloseDocument(doc);
        FPDF_DestroyLibrary();

        Ok(())
    }
}
