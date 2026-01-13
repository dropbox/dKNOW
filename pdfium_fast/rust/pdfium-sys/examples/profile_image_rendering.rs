use pdfium_sys::*;
use std::env;
use std::ffi::CString;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process;
use std::time::Instant;

/// Profiling version of image rendering that measures time for each operation.
/// Reports detailed timing breakdown to help identify optimization opportunities.
struct TimingStats {
    load_page_ns: Vec<u128>,
    create_bitmap_ns: Vec<u128>,
    render_page_ns: Vec<u128>,
    png_encode_ns: Vec<u128>,
    write_file_ns: Vec<u128>,
    close_page_ns: Vec<u128>,
}

impl TimingStats {
    fn new() -> Self {
        TimingStats {
            load_page_ns: Vec::new(),
            create_bitmap_ns: Vec::new(),
            render_page_ns: Vec::new(),
            png_encode_ns: Vec::new(),
            write_file_ns: Vec::new(),
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
        let total_create_bitmap = sum_ns(&self.create_bitmap_ns);
        let total_render = sum_ns(&self.render_page_ns);
        let total_encode = sum_ns(&self.png_encode_ns);
        let total_write = sum_ns(&self.write_file_ns);
        let total_close = sum_ns(&self.close_page_ns);

        let grand_total = total_load_page
            + total_create_bitmap
            + total_render
            + total_encode
            + total_write
            + total_close;

        eprintln!("\n========== IMAGE RENDERING PROFILING REPORT ==========");
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
            "  FPDFBitmap_Create():     {:.3} sec ({:>5.1}%) avg={:.2}ms/page",
            total_create_bitmap as f64 / 1_000_000_000.0,
            (total_create_bitmap as f64 / grand_total as f64) * 100.0,
            avg_ns(&self.create_bitmap_ns) as f64 / 1_000_000.0
        );
        eprintln!(
            "  FPDF_RenderPageBitmap(): {:.3} sec ({:>5.1}%) avg={:.2}ms/page",
            total_render as f64 / 1_000_000_000.0,
            (total_render as f64 / grand_total as f64) * 100.0,
            avg_ns(&self.render_page_ns) as f64 / 1_000_000.0
        );
        eprintln!(
            "  PNG Encoding:            {:.3} sec ({:>5.1}%) avg={:.2}ms/page",
            total_encode as f64 / 1_000_000_000.0,
            (total_encode as f64 / grand_total as f64) * 100.0,
            avg_ns(&self.png_encode_ns) as f64 / 1_000_000.0
        );
        eprintln!(
            "  File I/O (write PNG):    {:.3} sec ({:>5.1}%) avg={:.2}ms/page",
            total_write as f64 / 1_000_000_000.0,
            (total_write as f64 / grand_total as f64) * 100.0,
            avg_ns(&self.write_file_ns) as f64 / 1_000_000.0
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
        eprintln!("Usage: {} <input.pdf> <output_dir>", args[0]);
        eprintln!("  Profiles image rendering and reports detailed timing breakdown");
        process::exit(1);
    }

    let pdf_path = &args[1];
    let output_dir = &args[2];

    if !Path::new(pdf_path).exists() {
        eprintln!("Error: PDF file not found: {}", pdf_path);
        process::exit(1);
    }

    match profile_image_rendering(pdf_path, output_dir) {
        Ok(_) => process::exit(0),
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
}

fn profile_image_rendering(pdf_path: &str, output_dir: &str) -> Result<(), String> {
    let mut stats = TimingStats::new();

    // Create output directory
    std::fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create output directory: {}", e))?;

    unsafe {
        // Library initialization (not counted - one-time setup)
        FPDF_InitLibrary();

        let c_path = CString::new(pdf_path).unwrap();
        let doc = FPDF_LoadDocument(c_path.as_ptr(), std::ptr::null());

        if doc.is_null() {
            FPDF_DestroyLibrary();
            return Err(format!("Failed to load PDF: {}", pdf_path));
        }

        let page_count = FPDF_GetPageCount(doc);

        // Render each page with detailed timing
        for page_index in 0..page_count {
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

            // Get page dimensions
            let width = FPDF_GetPageWidthF(page);
            let height = FPDF_GetPageHeightF(page);

            // Calculate bitmap dimensions at 300 DPI (scale = 300/72 = 4.166666)
            let scale = 300.0 / 72.0;
            let bitmap_width = (width * scale) as i32;
            let bitmap_height = (height * scale) as i32;

            // Time: FPDFBitmap_Create
            let t0 = Instant::now();
            let bitmap = FPDFBitmap_Create(bitmap_width, bitmap_height, 0);
            let t1 = Instant::now();
            stats
                .create_bitmap_ns
                .push(t1.duration_since(t0).as_nanos());

            if bitmap.is_null() {
                FPDF_ClosePage(page);
                FPDF_CloseDocument(doc);
                FPDF_DestroyLibrary();
                return Err(format!("Failed to create bitmap for page {}", page_index));
            }

            // Fill with white background
            FPDFBitmap_FillRect(bitmap, 0, 0, bitmap_width, bitmap_height, 0xFFFFFFFF);

            // Time: FPDF_RenderPageBitmap
            let t0 = Instant::now();
            FPDF_RenderPageBitmap(
                bitmap,
                page,
                0,
                0,
                bitmap_width,
                bitmap_height,
                0,
                FPDF_ANNOT as i32,
            );
            let t1 = Instant::now();
            stats.render_page_ns.push(t1.duration_since(t0).as_nanos());

            // Time: PNG encoding
            let t0 = Instant::now();
            let buffer = FPDFBitmap_GetBuffer(bitmap) as *const u8;
            let stride = FPDFBitmap_GetStride(bitmap) as usize;

            // Convert BGRA to RGBA for PNG
            let mut rgba_data = Vec::with_capacity((bitmap_width * bitmap_height * 4) as usize);
            for y in 0..bitmap_height {
                let row_start = (y as usize) * stride;
                for x in 0..bitmap_width {
                    let pixel_offset = row_start + (x as usize) * 4;
                    let b = *buffer.add(pixel_offset);
                    let g = *buffer.add(pixel_offset + 1);
                    let r = *buffer.add(pixel_offset + 2);
                    let a = *buffer.add(pixel_offset + 3);
                    rgba_data.push(r);
                    rgba_data.push(g);
                    rgba_data.push(b);
                    rgba_data.push(a);
                }
            }

            // Encode to PNG in memory
            let mut png_data = Vec::new();
            {
                let mut encoder =
                    png::Encoder::new(&mut png_data, bitmap_width as u32, bitmap_height as u32);
                encoder.set_color(png::ColorType::Rgba);
                encoder.set_depth(png::BitDepth::Eight);

                let mut writer = encoder
                    .write_header()
                    .map_err(|e| format!("Failed to write PNG header: {}", e))?;
                writer
                    .write_image_data(&rgba_data)
                    .map_err(|e| format!("Failed to encode PNG: {}", e))?;
            }
            let t1 = Instant::now();
            stats.png_encode_ns.push(t1.duration_since(t0).as_nanos());

            // Time: File I/O (write to disk)
            let t0 = Instant::now();
            let output_path = format!("{}/page_{:04}.png", output_dir, page_index);
            let mut file = File::create(&output_path)
                .map_err(|e| format!("Failed to create output file: {}", e))?;
            file.write_all(&png_data)
                .map_err(|e| format!("Failed to write PNG file: {}", e))?;
            let t1 = Instant::now();
            stats.write_file_ns.push(t1.duration_since(t0).as_nanos());

            FPDFBitmap_Destroy(bitmap);

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
