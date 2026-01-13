// Minimal test - simplest possible wrapper
use pdfium_sys::*;
use std::ffi::CString;
use std::fs::File;
use std::io::Write;

fn main() {
    unsafe {
        // Step 1: Init library (simplest possible)
        FPDF_InitLibrary();

        // Step 2: Load PDF
        let pdf_path = CString::new("/Users/ayates/pdfium/integration_tests/pdfs/benchmark/0100pages_7FKQLKX273JBHXAAW5XDRT27JGMIZMCI.pdf").unwrap();
        let doc = FPDF_LoadDocument(pdf_path.as_ptr(), std::ptr::null());

        if doc.is_null() {
            eprintln!("Failed to load PDF");
            FPDF_DestroyLibrary();
            return;
        }

        // Step 3: Load page 10
        let page = FPDF_LoadPage(doc, 10);
        if page.is_null() {
            eprintln!("Failed to load page 10");
            FPDF_CloseDocument(doc);
            FPDF_DestroyLibrary();
            return;
        }

        // Step 4: Get dimensions
        let width_pts = FPDF_GetPageWidthF(page) as f64;
        let height_pts = FPDF_GetPageHeightF(page) as f64;

        let scale = 4.166666f64; // 300 DPI
        let width_px = (width_pts * scale) as i32;
        let height_px = (height_pts * scale) as i32;

        println!(
            "Page 10: {}x{} pts -> {}x{} px",
            width_pts, height_pts, width_px, height_px
        );

        // Step 5: Create bitmap (SIMPLEST - let PDFium allocate)
        let bitmap = FPDFBitmap_CreateEx(
            width_px,
            height_px,
            FPDFBitmap_BGRx as i32,
            std::ptr::null_mut(),
            0,
        );
        if bitmap.is_null() {
            eprintln!("Failed to create bitmap");
            FPDF_ClosePage(page);
            FPDF_CloseDocument(doc);
            FPDF_DestroyLibrary();
            return;
        }

        // Step 6: Fill white background
        FPDFBitmap_FillRect(bitmap, 0, 0, width_px, height_px, 0xFFFFFFFF);

        // Step 7: Render (SIMPLEST - just render, no form APIs)
        FPDF_RenderPageBitmap(
            bitmap,
            page,
            0,
            0,
            width_px,
            height_px,
            0,
            FPDF_ANNOT as i32,
        );

        // Step 8: Get buffer
        let buffer = FPDFBitmap_GetBuffer(bitmap) as *const u8;
        let stride = FPDFBitmap_GetStride(bitmap) as usize;

        println!("Stride: {}", stride);

        // Step 9: Convert BGRA to RGB
        let mut rgb_data = Vec::new();
        for y in 0..height_px {
            let row_offset = (y as usize) * stride;
            for x in 0..width_px {
                let pixel_offset = row_offset + (x as usize) * 4;
                let b = *buffer.add(pixel_offset);
                let g = *buffer.add(pixel_offset + 1);
                let r = *buffer.add(pixel_offset + 2);
                rgb_data.push(r);
                rgb_data.push(g);
                rgb_data.push(b);
            }
        }

        // Step 10: Write PPM
        let mut file = File::create("/tmp/minimal_page10.ppm").unwrap();
        write!(
            file,
            "P6\n# PDF test render\n{} {}\n255\n",
            width_px, height_px
        )
        .unwrap();
        file.write_all(&rgb_data).unwrap();

        println!("Wrote /tmp/minimal_page10.ppm");

        // Step 11: Cleanup
        FPDFBitmap_Destroy(bitmap);
        FPDF_ClosePage(page);
        FPDF_CloseDocument(doc);
        FPDF_DestroyLibrary();
    }
}
