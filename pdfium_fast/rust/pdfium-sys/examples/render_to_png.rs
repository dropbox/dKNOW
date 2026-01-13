use pdfium_sys::*;
use std::env;
use std::ffi::CString;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 || args.len() > 5 {
        eprintln!(
            "Usage: {} <input.pdf> <output_dir> [page_num] [dpi]",
            args[0]
        );
        eprintln!("  page_num: specific page (0-indexed) or 'all' (default: all)");
        eprintln!("  dpi: dots per inch for rendering (default: 300)");
        process::exit(1);
    }

    let pdf_path = &args[1];
    let output_dir = &args[2];
    let page_spec = if args.len() >= 4 { &args[3] } else { "all" };
    let dpi = if args.len() >= 5 {
        args[4].parse::<f64>().unwrap_or_else(|_| {
            eprintln!("Error: DPI must be a number");
            process::exit(1);
        })
    } else {
        300.0
    };

    // Check input file exists
    if !Path::new(pdf_path).exists() {
        eprintln!("Error: PDF file not found: {}", pdf_path);
        process::exit(1);
    }

    // Create output directory if it doesn't exist
    std::fs::create_dir_all(output_dir).unwrap_or_else(|e| {
        eprintln!("Error: Failed to create output directory: {}", e);
        process::exit(1);
    });

    // Render
    match render_pdf(pdf_path, output_dir, page_spec, dpi) {
        Ok(_) => process::exit(0),
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
}

fn render_pdf(pdf_path: &str, output_dir: &str, page_spec: &str, dpi: f64) -> Result<(), String> {
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

        // Get page count
        let page_count = FPDF_GetPageCount(doc);

        // Allow 0-page PDFs (upstream behavior: no output files, exit 0)
        if page_count == 0 {
            FPDF_CloseDocument(doc);
            FPDF_DestroyLibrary();
            return Ok(());
        }

        // Determine which pages to render
        let pages_to_render: Vec<i32> = if page_spec == "all" {
            (0..page_count).collect()
        } else {
            match page_spec.parse::<i32>() {
                Ok(page_num) if page_num >= 0 && page_num < page_count => vec![page_num],
                _ => {
                    FPDF_CloseDocument(doc);
                    FPDF_DestroyLibrary();
                    return Err(format!(
                        "Invalid page number: {} (document has {} pages)",
                        page_spec, page_count
                    ));
                }
            }
        };

        eprintln!(
            "Rendering {} page(s) from {} at {} DPI",
            pages_to_render.len(),
            pdf_path,
            dpi
        );

        // Render each page
        for &page_index in &pages_to_render {
            let result = render_page(doc, page_index, output_dir, pdf_path, dpi);
            if let Err(e) = result {
                eprintln!("Warning: Failed to render page {}: {}", page_index, e);
            }
        }

        // Clean up
        FPDF_CloseDocument(doc);
        FPDF_DestroyLibrary();

        Ok(())
    }
}

fn render_page(
    doc: FPDF_DOCUMENT,
    page_index: i32,
    output_dir: &str,
    pdf_name: &str,
    dpi: f64,
) -> Result<(), String> {
    unsafe {
        // Load page
        let page = FPDF_LoadPage(doc, page_index);
        if page.is_null() {
            return Err(format!("Failed to load page {}", page_index));
        }

        // Get page dimensions in points (1 point = 1/72 inch)
        let page_width = FPDF_GetPageWidthF(page);
        let page_height = FPDF_GetPageHeightF(page);

        // Convert to pixels at specified DPI
        let width_px = ((page_width as f64) * dpi / 72.0).round() as i32;
        let height_px = ((page_height as f64) * dpi / 72.0).round() as i32;

        eprintln!(
            "  Page {}: {} x {} points -> {} x {} pixels",
            page_index, page_width, page_height, width_px, height_px
        );

        // Create bitmap (BGRA format with alpha)
        let bitmap = FPDFBitmap_Create(width_px, height_px, 1);
        if bitmap.is_null() {
            FPDF_ClosePage(page);
            return Err("Failed to create bitmap".to_string());
        }

        // Fill with white background (0xFFFFFFFF = white in BGRA)
        FPDFBitmap_FillRect(bitmap, 0, 0, width_px, height_px, 0xFFFFFFFF);

        // Render page to bitmap
        FPDF_RenderPageBitmap(
            bitmap, page, 0,         // start_x
            0,         // start_y
            width_px,  // size_x
            height_px, // size_y
            0,         // rotation (0 = no rotation)
            0,         // flags (0 = default rendering)
        );

        // Get bitmap data
        let buffer = FPDFBitmap_GetBuffer(bitmap) as *const u8;
        let stride = FPDFBitmap_GetStride(bitmap);
        let width = FPDFBitmap_GetWidth(bitmap) as usize;
        let height = FPDFBitmap_GetHeight(bitmap) as usize;

        if buffer.is_null() {
            FPDFBitmap_Destroy(bitmap);
            FPDF_ClosePage(page);
            return Err("Failed to get bitmap buffer".to_string());
        }

        // Convert BGRA to RGBA for PNG encoding
        let mut rgba_data = vec![0u8; width * height * 4];
        for y in 0..height {
            for x in 0..width {
                let src_offset = (y * stride as usize) + (x * 4);
                let dst_offset = (y * width + x) * 4;

                // BGRA -> RGBA
                let b = *buffer.add(src_offset);
                let g = *buffer.add(src_offset + 1);
                let r = *buffer.add(src_offset + 2);
                let a = *buffer.add(src_offset + 3);

                rgba_data[dst_offset] = r;
                rgba_data[dst_offset + 1] = g;
                rgba_data[dst_offset + 2] = b;
                rgba_data[dst_offset + 3] = a;
            }
        }

        // Write PNG file
        let pdf_basename = Path::new(pdf_name)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");
        let output_filename = format!("{}/{}.{}.png", output_dir, pdf_basename, page_index);

        write_png(&output_filename, &rgba_data, width as u32, height as u32)?;

        // Clean up
        FPDFBitmap_Destroy(bitmap);
        FPDF_ClosePage(page);

        Ok(())
    }
}

fn write_png(filename: &str, data: &[u8], width: u32, height: u32) -> Result<(), String> {
    let file = File::create(filename).map_err(|e| format!("Failed to create PNG file: {}", e))?;
    let writer = BufWriter::new(file);

    let mut encoder = png::Encoder::new(writer, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);

    let mut writer = encoder
        .write_header()
        .map_err(|e| format!("Failed to write PNG header: {}", e))?;

    writer
        .write_image_data(data)
        .map_err(|e| format!("Failed to write PNG data: {}", e))?;

    Ok(())
}
