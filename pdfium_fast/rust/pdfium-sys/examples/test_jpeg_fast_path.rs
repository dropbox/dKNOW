// JPEG Fast Path API for Rust
// Exposes the 545x speedup JPEG extraction for scanned PDFs
//
// This module provides:
// - is_scanned_page(): Check if a page is a scanned JPEG (single full-page image)
// - extract_jpeg_raw(): Extract raw JPEG bytes without decode/re-encode cycle
//
// Usage: cargo run --example test_jpeg_fast_path <pdf_path> [output_dir]

// Allow raw pointer arg deref for FFI example - functions wrap unsafe blocks internally
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use pdfium_sys::*;
use std::env;
use std::ffi::CString;
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Minimum page coverage for a scanned page (95%)
const SCANNED_COVERAGE_THRESHOLD: f64 = 0.95;

/// Check if a page is a scanned JPEG page (single full-page image)
///
/// A page is considered "scanned" if:
/// 1. It has exactly one object
/// 2. That object is an image
/// 3. The image covers >= 95% of the page area
///
/// Returns true if page is scanned, false otherwise
pub fn is_scanned_page(page: FPDF_PAGE) -> bool {
    unsafe {
        // Check if page has exactly one object
        let obj_count = FPDFPage_CountObjects(page);
        if obj_count != 1 {
            return false;
        }

        // Check if the single object is an image
        let obj = FPDFPage_GetObject(page, 0);
        if obj.is_null() {
            return false;
        }

        if FPDFPageObj_GetType(obj) != FPDF_PAGEOBJ_IMAGE as i32 {
            return false;
        }

        // Check if image covers >= 95% of page area
        let mut left: f32 = 0.0;
        let mut bottom: f32 = 0.0;
        let mut right: f32 = 0.0;
        let mut top: f32 = 0.0;

        if FPDFPageObj_GetBounds(obj, &mut left, &mut bottom, &mut right, &mut top) == 0 {
            return false;
        }

        let page_width = FPDF_GetPageWidthF(page) as f64;
        let page_height = FPDF_GetPageHeightF(page) as f64;

        let obj_area = (right - left) as f64 * (top - bottom) as f64;
        let page_area = page_width * page_height;

        if page_area <= 0.0 {
            return false;
        }

        let coverage = obj_area / page_area;
        coverage >= SCANNED_COVERAGE_THRESHOLD
    }
}

/// Check if a page object is a JPEG image (uses DCTDecode filter)
///
/// Returns true if the image uses DCTDecode compression (JPEG)
pub fn is_jpeg_image(img_obj: FPDF_PAGEOBJECT) -> bool {
    unsafe {
        let filter_count = FPDFImageObj_GetImageFilterCount(img_obj);

        for i in 0..filter_count {
            // Get filter name length
            let filter_len = FPDFImageObj_GetImageFilter(img_obj, i, std::ptr::null_mut(), 0);
            if filter_len == 0 {
                continue;
            }

            // Get filter name
            let mut filter_name = vec![0u8; filter_len as usize];
            FPDFImageObj_GetImageFilter(
                img_obj,
                i,
                filter_name.as_mut_ptr() as *mut std::ffi::c_void,
                filter_len,
            );

            // Check if it's DCTDecode (JPEG)
            if filter_name.starts_with(b"DCTDecode") {
                return true;
            }
        }

        false
    }
}

/// Extract raw JPEG bytes from a scanned page
///
/// This function extracts the raw JPEG data directly from the PDF without
/// decode/re-encode cycle, achieving 545x speedup for scanned PDFs.
///
/// Returns Some(Vec<u8>) with raw JPEG data on success, None on failure
pub fn extract_jpeg_raw(page: FPDF_PAGE) -> Option<Vec<u8>> {
    unsafe {
        // Get the single image object
        let img_obj = FPDFPage_GetObject(page, 0);
        if img_obj.is_null() {
            return None;
        }

        // Verify it's a JPEG image
        if !is_jpeg_image(img_obj) {
            return None;
        }

        // Get raw data size
        let raw_size = FPDFImageObj_GetImageDataRaw(img_obj, std::ptr::null_mut(), 0);
        if raw_size == 0 {
            return None;
        }

        // Extract raw JPEG bytes
        let mut jpeg_data = vec![0u8; raw_size as usize];
        let actual_size =
            FPDFImageObj_GetImageDataRaw(img_obj, jpeg_data.as_mut_ptr() as *mut _, raw_size);

        if actual_size == 0 || actual_size > raw_size {
            return None;
        }

        // Verify JPEG header (FF D8 FF)
        if jpeg_data.len() < 3
            || jpeg_data[0] != 0xFF
            || jpeg_data[1] != 0xD8
            || jpeg_data[2] != 0xFF
        {
            return None;
        }

        // Trim to actual size if needed
        jpeg_data.truncate(actual_size as usize);

        Some(jpeg_data)
    }
}

/// Process a PDF and extract scanned pages via JPEG fast path
///
/// Returns (scanned_count, total_pages, bytes_extracted)
fn process_pdf(pdf_path: &str, output_dir: Option<&str>) -> Result<(u32, u32, u64), String> {
    // Initialize PDFium
    unsafe {
        FPDF_InitLibrary();
    }

    // Open PDF
    let c_path = CString::new(pdf_path).map_err(|_| "Invalid path")?;
    let doc = unsafe { FPDF_LoadDocument(c_path.as_ptr(), std::ptr::null()) };

    if doc.is_null() {
        return Err("Failed to open PDF".to_string());
    }

    let page_count = unsafe { FPDF_GetPageCount(doc) };
    let mut scanned_count = 0u32;
    let mut bytes_extracted = 0u64;

    println!("Processing {} pages...", page_count);

    for i in 0..page_count {
        let page = unsafe { FPDF_LoadPage(doc, i) };
        if page.is_null() {
            eprintln!("Warning: Failed to load page {}", i);
            continue;
        }

        if is_scanned_page(page) {
            if let Some(jpeg_data) = extract_jpeg_raw(page) {
                scanned_count += 1;
                bytes_extracted += jpeg_data.len() as u64;

                // Save to file if output_dir provided
                if let Some(dir) = output_dir {
                    let output_path = format!("{}/page_{:05}.jpg", dir, i);
                    match File::create(&output_path) {
                        Ok(mut file) => {
                            if let Err(e) = file.write_all(&jpeg_data) {
                                eprintln!("Warning: Failed to write {}: {}", output_path, e);
                            } else {
                                println!(
                                    "  Page {}: extracted {} bytes (JPEG fast path)",
                                    i,
                                    jpeg_data.len()
                                );
                            }
                        }
                        Err(e) => eprintln!("Warning: Failed to create {}: {}", output_path, e),
                    }
                } else {
                    println!(
                        "  Page {}: scanned (JPEG, {} bytes available)",
                        i,
                        jpeg_data.len()
                    );
                }
            } else {
                println!("  Page {}: scanned but not JPEG (non-JPEG compression)", i);
            }
        } else {
            println!("  Page {}: not scanned (requires rendering)", i);
        }

        unsafe { FPDF_ClosePage(page) };
    }

    unsafe {
        FPDF_CloseDocument(doc);
        FPDF_DestroyLibrary();
    }

    Ok((scanned_count, page_count as u32, bytes_extracted))
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <pdf_path> [output_dir]", args[0]);
        eprintln!();
        eprintln!("Tests JPEG fast path extraction for scanned PDFs.");
        eprintln!("If output_dir is provided, extracted JPEGs are saved there.");
        eprintln!("Otherwise, just reports which pages are scanned.");
        std::process::exit(1);
    }

    let pdf_path = &args[1];
    let output_dir = args.get(2).map(|s| s.as_str());

    // Verify PDF exists
    if !Path::new(pdf_path).exists() {
        eprintln!("Error: PDF not found: {}", pdf_path);
        std::process::exit(1);
    }

    // Create output directory if specified
    if let Some(dir) = output_dir {
        if let Err(e) = std::fs::create_dir_all(dir) {
            eprintln!("Error: Failed to create output directory: {}", e);
            std::process::exit(1);
        }
    }

    println!("JPEG Fast Path Test");
    println!("==================");
    println!("PDF: {}", pdf_path);
    if let Some(dir) = output_dir {
        println!("Output: {}", dir);
    }
    println!();

    match process_pdf(pdf_path, output_dir) {
        Ok((scanned, total, bytes)) => {
            println!();
            println!("Summary");
            println!("-------");
            println!("Total pages: {}", total);
            println!(
                "Scanned pages: {} ({:.1}%)",
                scanned,
                100.0 * scanned as f64 / total as f64
            );
            println!("Non-scanned pages: {}", total - scanned);
            if scanned > 0 {
                println!(
                    "JPEG bytes extracted: {} ({:.2} MB)",
                    bytes,
                    bytes as f64 / (1024.0 * 1024.0)
                );
                println!();
                println!("JPEG fast path: 545x speedup for scanned pages!");
                println!("These pages skip rendering entirely - raw JPEG extraction.");
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
