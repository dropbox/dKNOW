use pdfium_sys::*;
use std::env;
use std::ffi::CString;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process;
use std::time::Instant;

/// Profiling version of text extraction that measures time for each operation.
/// Reports detailed timing breakdown to help identify optimization opportunities.
struct TimingStats {
    load_page_ns: Vec<u128>,
    load_textpage_ns: Vec<u128>,
    count_chars_ns: Vec<u128>,
    get_text_ns: Vec<u128>,
    write_output_ns: Vec<u128>,
    close_page_ns: Vec<u128>,
}

impl TimingStats {
    fn new() -> Self {
        TimingStats {
            load_page_ns: Vec::new(),
            load_textpage_ns: Vec::new(),
            count_chars_ns: Vec::new(),
            get_text_ns: Vec::new(),
            write_output_ns: Vec::new(),
            close_page_ns: Vec::new(),
        }
    }

    fn report(&self, page_count: i32) {
        let sum_ns = |v: &Vec<u128>| v.iter().sum::<u128>();
        let avg_ns = |v: &Vec<u128>| {
            if v.is_empty() {
                0
            } else {
                v.iter().sum::<u128>() / v.len() as u128
            }
        };

        let total_load_page = sum_ns(&self.load_page_ns);
        let total_load_textpage = sum_ns(&self.load_textpage_ns);
        let total_count_chars = sum_ns(&self.count_chars_ns);
        let total_get_text = sum_ns(&self.get_text_ns);
        let total_write = sum_ns(&self.write_output_ns);
        let total_close = sum_ns(&self.close_page_ns);

        let grand_total = total_load_page
            + total_load_textpage
            + total_count_chars
            + total_get_text
            + total_write
            + total_close;

        eprintln!("\n========== TEXT EXTRACTION PROFILING REPORT ==========");
        eprintln!("Pages: {}", page_count);
        eprintln!(
            "Total time: {:.3} sec",
            grand_total as f64 / 1_000_000_000.0
        );
        eprintln!("\nPer-Operation Breakdown:");
        eprintln!(
            "  FPDF_LoadPage():         {:.3} sec ({:>5.1}%) avg={:.2}ms/page",
            total_load_page as f64 / 1_000_000_000.0,
            (total_load_page as f64 / grand_total as f64) * 100.0,
            avg_ns(&self.load_page_ns) as f64 / 1_000_000.0
        );
        eprintln!(
            "  FPDFText_LoadPage():     {:.3} sec ({:>5.1}%) avg={:.2}ms/page",
            total_load_textpage as f64 / 1_000_000_000.0,
            (total_load_textpage as f64 / grand_total as f64) * 100.0,
            avg_ns(&self.load_textpage_ns) as f64 / 1_000_000.0
        );
        eprintln!(
            "  FPDFText_CountChars():   {:.3} sec ({:>5.1}%) avg={:.2}ms/page",
            total_count_chars as f64 / 1_000_000_000.0,
            (total_count_chars as f64 / grand_total as f64) * 100.0,
            avg_ns(&self.count_chars_ns) as f64 / 1_000_000.0
        );
        eprintln!(
            "  FPDFText_GetText():      {:.3} sec ({:>5.1}%) avg={:.2}ms/page",
            total_get_text as f64 / 1_000_000_000.0,
            (total_get_text as f64 / grand_total as f64) * 100.0,
            avg_ns(&self.get_text_ns) as f64 / 1_000_000.0
        );
        eprintln!(
            "  File I/O (write):        {:.3} sec ({:>5.1}%) avg={:.2}ms/page",
            total_write as f64 / 1_000_000_000.0,
            (total_write as f64 / grand_total as f64) * 100.0,
            avg_ns(&self.write_output_ns) as f64 / 1_000_000.0
        );
        eprintln!(
            "  FPDF_ClosePage():        {:.3} sec ({:>5.1}%) avg={:.2}ms/page",
            total_close as f64 / 1_000_000_000.0,
            (total_close as f64 / grand_total as f64) * 100.0,
            avg_ns(&self.close_page_ns) as f64 / 1_000_000.0
        );
        eprintln!(
            "\nThroughput: {:.2} pages/sec",
            page_count as f64 / (grand_total as f64 / 1_000_000_000.0)
        );
        eprintln!("========================================================\n");
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: {} <input.pdf> <output.txt>", args[0]);
        eprintln!("  Profiles text extraction and reports detailed timing breakdown");
        process::exit(1);
    }

    let pdf_path = &args[1];
    let output_path = &args[2];

    if !Path::new(pdf_path).exists() {
        eprintln!("Error: PDF file not found: {}", pdf_path);
        process::exit(1);
    }

    match profile_text_extraction(pdf_path, output_path) {
        Ok(_) => process::exit(0),
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
}

fn profile_text_extraction(pdf_path: &str, output_path: &str) -> Result<(), String> {
    let mut stats = TimingStats::new();

    unsafe {
        // Library initialization (not counted - one-time setup)
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

        // Extract text from each page with detailed timing
        for page_index in 0..page_count {
            // Write BOM for each page after the first
            if page_index > 0 {
                output_file
                    .write_all(&[0xFF, 0xFE, 0x00, 0x00])
                    .map_err(|e| format!("Failed to write BOM: {}", e))?;
            }

            // Time: FPDF_LoadPage
            let t0 = Instant::now();
            let page = FPDF_LoadPage(doc, page_index);
            let t1 = Instant::now();
            stats.load_page_ns.push(t1.duration_since(t0).as_nanos());

            if page.is_null() {
                FPDF_CloseDocument(doc);
                FPDF_DestroyLibrary();
                return Err(format!("Failed to load page {}", page_index));
            }

            // Time: FPDFText_LoadPage
            let t0 = Instant::now();
            let text_page = FPDFText_LoadPage(page);
            let t1 = Instant::now();
            stats
                .load_textpage_ns
                .push(t1.duration_since(t0).as_nanos());

            if text_page.is_null() {
                FPDF_ClosePage(page);
                FPDF_CloseDocument(doc);
                FPDF_DestroyLibrary();
                return Err(format!("Failed to load text page {}", page_index));
            }

            // Time: FPDFText_CountChars
            let t0 = Instant::now();
            let char_count = FPDFText_CountChars(text_page);
            let t1 = Instant::now();
            stats.count_chars_ns.push(t1.duration_since(t0).as_nanos());

            if char_count > 0 {
                // Time: FPDFText_GetText (UTF-16 LE extraction)
                let buffer_size = (char_count as usize + 1) * 2; // UTF-16 LE: 2 bytes per char
                let mut buffer = vec![0u8; buffer_size];

                let t0 = Instant::now();
                let chars_written =
                    FPDFText_GetText(text_page, 0, char_count, buffer.as_mut_ptr() as *mut u16);
                let t1 = Instant::now();
                stats.get_text_ns.push(t1.duration_since(t0).as_nanos());

                if chars_written > 0 {
                    // Convert UTF-16 LE to UTF-32 LE
                    let utf16_data = &buffer[..((chars_written as usize - 1) * 2)];
                    let utf16_chars: Vec<u16> = utf16_data
                        .chunks_exact(2)
                        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                        .collect();

                    let utf32_string = String::from_utf16(&utf16_chars).map_err(|e| {
                        format!("UTF-16 decode error on page {}: {}", page_index, e)
                    })?;

                    // Time: File I/O (UTF-32 LE conversion + write)
                    let t0 = Instant::now();
                    for ch in utf32_string.chars() {
                        let code_point = ch as u32;
                        output_file
                            .write_all(&code_point.to_le_bytes())
                            .map_err(|e| format!("Failed to write character: {}", e))?;
                    }
                    let t1 = Instant::now();
                    stats.write_output_ns.push(t1.duration_since(t0).as_nanos());
                } else {
                    stats.get_text_ns.push(0);
                    stats.write_output_ns.push(0);
                }
            } else {
                stats.get_text_ns.push(0);
                stats.write_output_ns.push(0);
            }

            FPDFText_ClosePage(text_page);

            // Time: FPDF_ClosePage
            let t0 = Instant::now();
            FPDF_ClosePage(page);
            let t1 = Instant::now();
            stats.close_page_ns.push(t1.duration_since(t0).as_nanos());
        }

        FPDF_CloseDocument(doc);
        FPDF_DestroyLibrary();

        // Print profiling report
        stats.report(page_count);

        Ok(())
    }
}
