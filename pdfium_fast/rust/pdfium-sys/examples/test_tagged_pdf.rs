//! Test tagged PDF detection API
//! N=32: Validates FPDFCatalog_IsTagged() binding works
use std::path::Path;

#[allow(dead_code)]
fn check_pdf(path: &Path) {
    let path_str = path.to_str().unwrap();
    let c_path = std::ffi::CString::new(path_str).unwrap();

    let doc = unsafe { pdfium_sys::FPDF_LoadDocument(c_path.as_ptr(), std::ptr::null()) };
    if doc.is_null() {
        println!(
            "{}: FAILED TO LOAD",
            path.file_name().unwrap().to_str().unwrap()
        );
        return;
    }

    let num_pages = unsafe { pdfium_sys::FPDF_GetPageCount(doc) };
    let is_tagged = unsafe { pdfium_sys::FPDFCatalog_IsTagged(doc) };

    println!(
        "{}: {} pages, tagged={}",
        path.file_name().unwrap().to_str().unwrap(),
        num_pages,
        if is_tagged != 0 { "YES" } else { "NO" }
    );

    unsafe {
        pdfium_sys::FPDF_CloseDocument(doc);
    }
}

fn main() {
    println!("Testing FPDFCatalog_IsTagged() API...\n");

    unsafe {
        pdfium_sys::FPDF_InitLibrary();
    }

    // Check test corpus
    let test_dir = Path::new("/Users/ayates/docling_rs/test-corpus/pdf");
    if test_dir.exists() {
        println!("Scanning {}:\n", test_dir.display());

        let mut tagged_count = 0;
        let mut total_count = 0;

        for entry in std::fs::read_dir(test_dir).unwrap().flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "pdf").unwrap_or(false) {
                let c_path = std::ffi::CString::new(path.to_str().unwrap()).unwrap();
                let doc =
                    unsafe { pdfium_sys::FPDF_LoadDocument(c_path.as_ptr(), std::ptr::null()) };
                if !doc.is_null() {
                    total_count += 1;
                    let is_tagged = unsafe { pdfium_sys::FPDFCatalog_IsTagged(doc) };
                    if is_tagged != 0 {
                        tagged_count += 1;
                        println!("  [TAGGED] {}", path.file_name().unwrap().to_str().unwrap());
                    }
                    unsafe {
                        pdfium_sys::FPDF_CloseDocument(doc);
                    }
                }
            }
        }

        println!("\nSummary:");
        println!("  Total PDFs: {}", total_count);
        println!(
            "  Tagged PDFs: {} ({:.1}%)",
            tagged_count,
            if total_count > 0 {
                tagged_count as f64 / total_count as f64 * 100.0
            } else {
                0.0
            }
        );
        println!("  Untagged PDFs: {}", total_count - tagged_count);
    } else {
        println!("Test directory not found: {:?}", test_dir);
    }

    unsafe {
        pdfium_sys::FPDF_DestroyLibrary();
    }
}
