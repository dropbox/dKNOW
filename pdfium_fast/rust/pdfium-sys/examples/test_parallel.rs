//! Quick test for parallel rendering
use std::path::Path;
use std::time::Instant;

fn main() {
    println!("Testing pdfium_fast parallel rendering...\n");

    // Initialize pdfium
    unsafe {
        pdfium_sys::FPDF_InitLibrary();
    }

    let pdf_path = Path::new("/Users/ayates/docling_rs/test-corpus/pdf/2305.03393v1.pdf");
    if !pdf_path.exists() {
        println!("Test PDF not found");
        return;
    }

    let c_path = std::ffi::CString::new(pdf_path.to_str().unwrap()).unwrap();
    let doc = unsafe { pdfium_sys::FPDF_LoadDocument(c_path.as_ptr(), std::ptr::null()) };

    if doc.is_null() {
        println!("Failed to load document");
        return;
    }

    let num_pages = unsafe { pdfium_sys::FPDF_GetPageCount(doc) };
    println!("Document has {} pages", num_pages);

    let optimal_threads = unsafe { pdfium_sys::FPDF_GetOptimalWorkerCountForDocument(doc) };
    println!("Optimal thread count: {}", optimal_threads);

    // Test single-threaded rendering first
    println!("\n--- Sequential rendering (1 thread) ---");
    let start = Instant::now();
    for i in 0..num_pages {
        let page = unsafe { pdfium_sys::FPDF_LoadPage(doc, i) };
        if !page.is_null() {
            let width = unsafe { pdfium_sys::FPDF_GetPageWidth(page) };
            let height = unsafe { pdfium_sys::FPDF_GetPageHeight(page) };
            let scale = 150.0 / 72.0;
            let bm_width = (width * scale) as i32;
            let bm_height = (height * scale) as i32;

            let bitmap = unsafe { pdfium_sys::FPDFBitmap_Create(bm_width, bm_height, 0) };
            if !bitmap.is_null() {
                unsafe {
                    pdfium_sys::FPDFBitmap_FillRect(bitmap, 0, 0, bm_width, bm_height, 0xFFFFFFFF);
                    pdfium_sys::FPDF_RenderPageBitmap(
                        bitmap, page, 0, 0, bm_width, bm_height, 0, 0,
                    );
                    pdfium_sys::FPDFBitmap_Destroy(bitmap);
                }
            }
            unsafe {
                pdfium_sys::FPDF_ClosePage(page);
            }
        }
    }
    let sequential_time = start.elapsed();
    println!(
        "Sequential: {} pages in {:?} ({:.1} pages/sec)",
        num_pages,
        sequential_time,
        num_pages as f64 / sequential_time.as_secs_f64()
    );

    // Test parallel rendering
    println!("\n--- Parallel rendering ({} threads) ---", optimal_threads);

    use std::sync::{Arc, Mutex};
    let render_count: Arc<Mutex<i32>> = Arc::new(Mutex::new(0));
    let render_count_clone = Arc::clone(&render_count);

    extern "C" fn callback(
        _page_index: std::ffi::c_int,
        _buffer: *const std::ffi::c_void,
        _width: std::ffi::c_int,
        _height: std::ffi::c_int,
        _stride: std::ffi::c_int,
        user_data: *mut std::ffi::c_void,
        success: pdfium_sys::FPDF_BOOL,
    ) {
        if success != 0 {
            let count = unsafe { &*(user_data as *const Arc<Mutex<i32>>) };
            let mut guard = count.lock().unwrap();
            *guard += 1;
        }
    }

    let mut options = pdfium_sys::FPDF_PARALLEL_OPTIONS {
        worker_count: optimal_threads,
        max_queue_size: 0,
        form_handle: std::ptr::null_mut(),
        dpi: 150.0,
        output_format: pdfium_sys::FPDF_PARALLEL_FORMAT_DEFAULT as i32, // N=32: BGRx (4 bytes)
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
            &render_count_clone as *const _ as *mut std::ffi::c_void,
        )
    };
    let parallel_time = start.elapsed();

    let rendered = *render_count.lock().unwrap();
    if success != 0 && rendered == num_pages {
        println!(
            "Parallel: {} pages in {:?} ({:.1} pages/sec)",
            rendered,
            parallel_time,
            rendered as f64 / parallel_time.as_secs_f64()
        );

        let speedup = sequential_time.as_secs_f64() / parallel_time.as_secs_f64();
        println!(
            "\n==> SPEEDUP: {:.2}x faster with parallel rendering!",
            speedup
        );
    } else {
        println!(
            "Parallel rendering failed or incomplete: success={}, rendered={}/{}",
            success, rendered, num_pages
        );
    }

    // Cleanup
    unsafe {
        pdfium_sys::FPDF_CloseDocument(doc);
        pdfium_sys::FPDF_DestroyThreadPool();
        pdfium_sys::FPDF_DestroyLibrary();
    }
}
