// Minimal test WITH Form APIs
use pdfium_sys::*;
use std::ffi::CString;
use std::fs::File;
use std::io::Write;

fn main() {
    unsafe {
        FPDF_InitLibrary();

        let pdf_path = CString::new("/Users/ayates/pdfium/integration_tests/pdfs/benchmark/0100pages_7FKQLKX273JBHXAAW5XDRT27JGMIZMCI.pdf").unwrap();
        let doc = FPDF_LoadDocument(pdf_path.as_ptr(), std::ptr::null());

        if doc.is_null() {
            eprintln!("Failed to load PDF");
            FPDF_DestroyLibrary();
            return;
        }

        // ADD: Form environment
        let mut form_callbacks: FPDF_FORMFILLINFO = std::mem::zeroed();
        form_callbacks.version = 2;
        let form_handle = FPDFDOC_InitFormFillEnvironment(doc, &mut form_callbacks);

        if !form_handle.is_null() {
            FPDF_SetFormFieldHighlightColor(form_handle, 0, 0xFFE4DD);
            FPDF_SetFormFieldHighlightAlpha(form_handle, 100);
            FORM_DoDocumentJSAction(form_handle);
            FORM_DoDocumentOpenAction(form_handle);
        }

        let page = FPDF_LoadPage(doc, 10);
        if page.is_null() {
            eprintln!("Failed to load page 10");
            if !form_handle.is_null() {
                FPDFDOC_ExitFormFillEnvironment(form_handle);
            }
            FPDF_CloseDocument(doc);
            FPDF_DestroyLibrary();
            return;
        }

        // ADD: Form page callbacks
        if !form_handle.is_null() {
            FORM_OnAfterLoadPage(page, form_handle);
            FORM_DoPageAAction(page, form_handle, FPDFPAGE_AACTION_OPEN as i32);
        }

        let width_pts = FPDF_GetPageWidthF(page) as f64;
        let height_pts = FPDF_GetPageHeightF(page) as f64;

        let scale = 4.166666f64;
        let width_px = (width_pts * scale) as i32;
        let height_px = (height_pts * scale) as i32;

        println!("Page 10 with forms: {}x{} px", width_px, height_px);

        let bitmap = FPDFBitmap_Create(width_px, height_px, 0);
        if bitmap.is_null() {
            eprintln!("Failed to create bitmap");
            if !form_handle.is_null() {
                FORM_OnBeforeClosePage(page, form_handle);
            }
            FPDF_ClosePage(page);
            if !form_handle.is_null() {
                FPDFDOC_ExitFormFillEnvironment(form_handle);
            }
            FPDF_CloseDocument(doc);
            FPDF_DestroyLibrary();
            return;
        }

        FPDFBitmap_FillRect(bitmap, 0, 0, width_px, height_px, 0xFFFFFFFF);

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

        // ADD: Close form callbacks
        if !form_handle.is_null() {
            FORM_DoPageAAction(page, form_handle, FPDFPAGE_AACTION_CLOSE as i32);
            FORM_OnBeforeClosePage(page, form_handle);
        }

        let buffer = FPDFBitmap_GetBuffer(bitmap) as *const u8;
        let stride = FPDFBitmap_GetStride(bitmap) as usize;

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

        let mut file = File::create("/tmp/minimal_with_forms_page10.ppm").unwrap();
        write!(
            file,
            "P6\n# PDF test render\n{} {}\n255\n",
            width_px, height_px
        )
        .unwrap();
        file.write_all(&rgb_data).unwrap();

        println!("Wrote /tmp/minimal_with_forms_page10.ppm");

        FPDFBitmap_Destroy(bitmap);
        FPDF_ClosePage(page);

        if !form_handle.is_null() {
            FORM_DoDocumentAAction(form_handle, 0x10);
            FPDFDOC_ExitFormFillEnvironment(form_handle);
        }

        FPDF_CloseDocument(doc);
        FPDF_DestroyLibrary();
    }
}
