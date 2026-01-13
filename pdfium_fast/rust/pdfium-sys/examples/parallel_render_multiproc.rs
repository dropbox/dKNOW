use pdfium_sys::*;
use std::env;
use std::ffi::CString;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;
use std::process::{self, Command};

fn main() {
    let args: Vec<String> = env::args().collect();

    // Check if this is a worker process
    if args.len() >= 2 && args[1] == "--worker" {
        worker_main();
        return;
    }

    // Controller process
    if args.len() < 3 || args.len() > 5 {
        eprintln!(
            "Usage: {} <input.pdf> <output_dir> [worker_count] [dpi]",
            args[0]
        );
        eprintln!("  worker_count: 1-16 (default: 4)");
        eprintln!("  dpi: dots per inch for rendering (default: 300)");
        process::exit(1);
    }

    let pdf_path = &args[1];
    let output_dir = &args[2];
    let worker_count = if args.len() >= 4 {
        args[3].parse::<usize>().unwrap_or_else(|_| {
            eprintln!("Error: worker_count must be a number between 1 and 16");
            process::exit(1);
        })
    } else {
        4
    };
    let dpi = if args.len() >= 5 {
        args[4].parse::<f64>().unwrap_or_else(|_| {
            eprintln!("Error: DPI must be a number");
            process::exit(1);
        })
    } else {
        300.0
    };

    if !(1..=16).contains(&worker_count) {
        eprintln!("Error: worker_count must be between 1 and 16");
        process::exit(1);
    }

    if !Path::new(pdf_path).exists() {
        eprintln!("Error: PDF file not found: {}", pdf_path);
        process::exit(1);
    }

    // Create output directory if it doesn't exist
    std::fs::create_dir_all(output_dir).unwrap_or_else(|e| {
        eprintln!("Error: Failed to create output directory: {}", e);
        process::exit(1);
    });

    match controller_main(pdf_path, output_dir, worker_count, dpi) {
        Ok(_) => process::exit(0),
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
}

fn controller_main(
    pdf_path: &str,
    output_dir: &str,
    worker_count: usize,
    dpi: f64,
) -> Result<(), String> {
    unsafe {
        // Initialize PDFium just to get page count
        FPDF_InitLibrary();
        let c_path = CString::new(pdf_path).unwrap();
        let doc = FPDF_LoadDocument(c_path.as_ptr(), std::ptr::null());

        if doc.is_null() {
            FPDF_DestroyLibrary();
            return Err(format!("Failed to load PDF: {}", pdf_path));
        }

        let page_count = FPDF_GetPageCount(doc);
        FPDF_CloseDocument(doc);
        FPDF_DestroyLibrary();

        // Allow 0-page PDFs (upstream behavior: no output files, exit 0)
        if page_count == 0 {
            return Ok(());
        }

        eprintln!(
            "Rendering {} pages with {} workers at {} DPI (multi-process)",
            page_count, worker_count, dpi
        );

        // Distribute pages across workers
        let pages_per_worker = (page_count as usize).div_ceil(worker_count);
        let mut worker_processes = vec![];

        // Spawn worker processes
        let start_time = std::time::Instant::now();
        for worker_id in 0..worker_count {
            let start_page = worker_id * pages_per_worker;
            let end_page = ((worker_id + 1) * pages_per_worker).min(page_count as usize);

            if start_page >= page_count as usize {
                break;
            }

            // Spawn worker process
            let child = Command::new(std::env::current_exe().unwrap())
                .arg("--worker")
                .arg(pdf_path)
                .arg(output_dir)
                .arg(start_page.to_string())
                .arg(end_page.to_string())
                .arg(dpi.to_string())
                .spawn()
                .map_err(|e| format!("Failed to spawn worker {}: {}", worker_id, e))?;

            worker_processes.push(child);
        }

        // Wait for all workers to complete
        for (worker_id, mut child) in worker_processes.into_iter().enumerate() {
            let status = child
                .wait()
                .map_err(|e| format!("Worker {} failed: {}", worker_id, e))?;

            if !status.success() {
                return Err(format!(
                    "Worker {} exited with error: {}",
                    worker_id, status
                ));
            }
        }

        let elapsed = start_time.elapsed().as_secs_f64();
        println!("Rendered {} pages in {:.2} seconds", page_count, elapsed);

        Ok(())
    }
}

fn worker_main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 7 {
        eprintln!("Worker usage: --worker <pdf> <output_dir> <start_page> <end_page> <dpi>");
        process::exit(1);
    }

    let pdf_path = &args[2];
    let output_dir = &args[3];
    let start_page: usize = args[4].parse().unwrap();
    let end_page: usize = args[5].parse().unwrap();
    let dpi: f64 = args[6].parse().unwrap();

    match render_pages(pdf_path, output_dir, start_page, end_page, dpi) {
        Ok(_) => process::exit(0),
        Err(e) => {
            eprintln!("Worker error: {}", e);
            process::exit(1);
        }
    }
}

fn render_pages(
    pdf_path: &str,
    output_dir: &str,
    start_page: usize,
    end_page: usize,
    dpi: f64,
) -> Result<(), String> {
    unsafe {
        // Each worker has its own PDFium instance - NO SHARED STATE
        FPDF_InitLibrary();

        let c_path = CString::new(pdf_path).unwrap();
        let doc = FPDF_LoadDocument(c_path.as_ptr(), std::ptr::null());

        if doc.is_null() {
            FPDF_DestroyLibrary();
            return Err(format!("Failed to load PDF: {}", pdf_path));
        }

        // Get PDF basename for output filenames
        let pdf_basename = Path::new(pdf_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output")
            .to_string();

        // Process assigned pages
        for page_index in start_page..end_page {
            let page = FPDF_LoadPage(doc, page_index as i32);
            if page.is_null() {
                eprintln!("Warning: Failed to load page {}", page_index);
                continue;
            }

            // Get page dimensions in points
            let page_width = FPDF_GetPageWidthF(page);
            let page_height = FPDF_GetPageHeightF(page);

            // Convert to pixels at specified DPI
            let width_px = ((page_width as f64) * dpi / 72.0).round() as i32;
            let height_px = ((page_height as f64) * dpi / 72.0).round() as i32;

            // Create bitmap
            let bitmap = FPDFBitmap_Create(width_px, height_px, 1);
            if bitmap.is_null() {
                FPDF_ClosePage(page);
                eprintln!("Warning: Failed to create bitmap for page {}", page_index);
                continue;
            }

            // Fill with white background
            FPDFBitmap_FillRect(bitmap, 0, 0, width_px, height_px, 0xFFFFFFFF);

            // Render page to bitmap
            FPDF_RenderPageBitmap(bitmap, page, 0, 0, width_px, height_px, 0, 0);

            // Get bitmap data
            let buffer = FPDFBitmap_GetBuffer(bitmap) as *const u8;
            let stride = FPDFBitmap_GetStride(bitmap);
            let width = FPDFBitmap_GetWidth(bitmap) as usize;
            let height = FPDFBitmap_GetHeight(bitmap) as usize;

            if buffer.is_null() {
                FPDFBitmap_Destroy(bitmap);
                FPDF_ClosePage(page);
                eprintln!(
                    "Warning: Failed to get bitmap buffer for page {}",
                    page_index
                );
                continue;
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

            // Clean up PDFium resources
            FPDFBitmap_Destroy(bitmap);
            FPDF_ClosePage(page);

            // Write PNG file
            let output_filename = format!("{}/{}.{}.png", output_dir, pdf_basename, page_index);
            write_png(&output_filename, &rgba_data, width as u32, height as u32)?;
        }

        FPDF_CloseDocument(doc);
        FPDF_DestroyLibrary();

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
