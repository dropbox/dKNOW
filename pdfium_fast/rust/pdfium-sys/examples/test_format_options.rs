//! Test BGR and GRAY output format options for memory optimization
//! N=32: Validates FPDF_PARALLEL_OPTIONS.output_format field
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Instant;

struct RenderStats {
    count: i32,
    total_bytes: usize,
    bytes_per_pixel: i32,
}

fn test_format(
    doc: pdfium_sys::FPDF_DOCUMENT,
    num_pages: i32,
    format_name: &str,
    format_value: u32,
) -> RenderStats {
    let stats: Arc<Mutex<RenderStats>> = Arc::new(Mutex::new(RenderStats {
        count: 0,
        total_bytes: 0,
        bytes_per_pixel: match format_value {
            0 => 4, // BGRx
            1 => 3, // BGR
            2 => 1, // GRAY
            _ => 4,
        },
    }));
    let stats_clone = Arc::clone(&stats);

    extern "C" fn callback(
        _page_index: std::ffi::c_int,
        buffer: *const std::ffi::c_void,
        _width: std::ffi::c_int,
        height: std::ffi::c_int,
        stride: std::ffi::c_int,
        user_data: *mut std::ffi::c_void,
        success: pdfium_sys::FPDF_BOOL,
    ) {
        if success != 0 && !buffer.is_null() {
            let stats = unsafe { &*(user_data as *const Arc<Mutex<RenderStats>>) };
            let mut guard = stats.lock().unwrap();
            guard.count += 1;
            guard.total_bytes += (height * stride) as usize;
        }
    }

    let mut options = pdfium_sys::FPDF_PARALLEL_OPTIONS {
        worker_count: 4,
        max_queue_size: 0,
        form_handle: std::ptr::null_mut(),
        dpi: 150.0,
        output_format: format_value as i32,
        reserved: [std::ptr::null_mut(); 1],
    };

    let start = Instant::now();
    let success = unsafe {
        pdfium_sys::FPDF_RenderPagesParallelV2(
            doc,
            0,
            num_pages,
            0,
            0,
            0,
            0,
            &mut options,
            Some(callback),
            &stats_clone as *const _ as *mut std::ffi::c_void,
        )
    };
    let elapsed = start.elapsed();

    let result = stats.lock().unwrap();
    println!("\n--- {} (format={}) ---", format_name, format_value);
    println!("Pages rendered: {}/{}", result.count, num_pages);
    println!(
        "Total bytes: {} ({:.1} MB)",
        result.total_bytes,
        result.total_bytes as f64 / 1048576.0
    );
    println!("Bytes per pixel: {}", result.bytes_per_pixel);
    println!("Time: {:?}", elapsed);
    println!("Success: {}", if success != 0 { "YES" } else { "NO" });

    RenderStats {
        count: result.count,
        total_bytes: result.total_bytes,
        bytes_per_pixel: result.bytes_per_pixel,
    }
}

fn main() {
    println!("Testing BGR and GRAY output format options...\n");

    unsafe {
        pdfium_sys::FPDF_InitLibrary();
    }

    let pdf_path = Path::new("/Users/ayates/docling_rs/test-corpus/pdf/2305.03393v1.pdf");
    if !pdf_path.exists() {
        println!("Test PDF not found: {:?}", pdf_path);
        return;
    }

    let c_path = std::ffi::CString::new(pdf_path.to_str().unwrap()).unwrap();
    let doc = unsafe { pdfium_sys::FPDF_LoadDocument(c_path.as_ptr(), std::ptr::null()) };

    if doc.is_null() {
        println!("Failed to load document");
        return;
    }

    let num_pages = unsafe { pdfium_sys::FPDF_GetPageCount(doc) };
    println!("Document: {:?}", pdf_path);
    println!("Pages: {}", num_pages);

    // Test all three formats
    let bgrx_stats = test_format(
        doc,
        num_pages,
        "BGRx (default)",
        pdfium_sys::FPDF_PARALLEL_FORMAT_BGRx,
    );
    let bgr_stats = test_format(
        doc,
        num_pages,
        "BGR (3 bytes)",
        pdfium_sys::FPDF_PARALLEL_FORMAT_BGR,
    );
    let gray_stats = test_format(
        doc,
        num_pages,
        "GRAY (1 byte)",
        pdfium_sys::FPDF_PARALLEL_FORMAT_GRAY,
    );

    // Summary
    println!("\n=== MEMORY SAVINGS SUMMARY ===");
    println!(
        "BGRx baseline: {:.1} MB",
        bgrx_stats.total_bytes as f64 / 1048576.0
    );
    if bgrx_stats.total_bytes > 0 {
        println!(
            "BGR savings: {:.1}% ({:.1} MB saved)",
            (1.0 - bgr_stats.total_bytes as f64 / bgrx_stats.total_bytes as f64) * 100.0,
            (bgrx_stats.total_bytes - bgr_stats.total_bytes) as f64 / 1048576.0
        );
        println!(
            "GRAY savings: {:.1}% ({:.1} MB saved)",
            (1.0 - gray_stats.total_bytes as f64 / bgrx_stats.total_bytes as f64) * 100.0,
            (bgrx_stats.total_bytes - gray_stats.total_bytes) as f64 / 1048576.0
        );
    }

    // Validate all pages rendered
    let all_passed = bgrx_stats.count == num_pages
        && bgr_stats.count == num_pages
        && gray_stats.count == num_pages;
    println!("\n=== VALIDATION ===");
    println!(
        "All formats rendered all pages: {}",
        if all_passed { "PASS" } else { "FAIL" }
    );

    // Cleanup
    unsafe {
        pdfium_sys::FPDF_CloseDocument(doc);
        pdfium_sys::FPDF_DestroyThreadPool();
        pdfium_sys::FPDF_DestroyLibrary();
    }
}
