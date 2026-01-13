// PDFium Rendering via C++ Bridge
//
// Uses pdfium_render_bridge.cpp for 100% compatibility with upstream.
// The C++ bridge handles all form callbacks internally, exposing a simple C API.
//
// Architecture:
//   This Rust code → Simple C API → C++ Bridge → PDFium (with full form support)

use std::env;
use std::ffi::{c_char, c_double, c_int, CString};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

// Match the C struct from pdfium_render_bridge.cpp
#[repr(C)]
struct RenderResult {
    pixels: *mut u8,
    width: i32,
    height: i32,
    size: i32,
}

// Link directives for the bridge library
#[link(name = "pdfium_render_bridge")]
#[link(name = "pdfium")]
extern "C" {
    fn pdfium_bridge_init();
    fn pdfium_bridge_render_page(
        pdf_path: *const c_char,
        page_index: c_int,
        dpi: c_double,
    ) -> *mut RenderResult;
    fn pdfium_bridge_free_result(result: *mut RenderResult);
    fn pdfium_bridge_destroy();
}

/// Write PPM (Portable Pixmap) P6 format
/// This is the format used by upstream pdfium_test for baseline comparisons
fn write_ppm(path: &Path, width: i32, height: i32, rgb_data: &[u8]) -> std::io::Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);

    // P6 header: magic number, comment, width, height, max color value
    // CRITICAL: Must include comment to match upstream pdfium_test MD5
    write!(writer, "P6\n# PDF test render\n{} {}\n255\n", width, height)?;

    // Binary RGB data
    writer.write_all(rgb_data)?;
    writer.flush()?;

    Ok(())
}

/// Render a single page using the C++ bridge
fn render_page(pdf_path: &str, page_index: i32, output_path: &str, dpi: f64) -> Result<(), String> {
    unsafe {
        let c_path = CString::new(pdf_path).map_err(|e| e.to_string())?;
        let result = pdfium_bridge_render_page(c_path.as_ptr(), page_index, dpi);

        if result.is_null() {
            return Err(format!("Failed to render page {}", page_index));
        }

        // Extract render result
        let width = (*result).width;
        let height = (*result).height;
        let size = (*result).size as usize;

        // Copy RGB data from C++ to Rust Vec
        let rgb_slice = std::slice::from_raw_parts((*result).pixels, size);
        let rgb_data = rgb_slice.to_vec();

        // Free the C++ result
        pdfium_bridge_free_result(result);

        // Write PPM file
        let output = Path::new(output_path);
        write_ppm(output, width, height, &rgb_data)
            .map_err(|e| format!("Failed to write PPM: {}", e))?;

        println!(
            "Rendered page {} -> {} ({}x{} @ {}dpi)",
            page_index, output_path, width, height, dpi
        );

        Ok(())
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 5 {
        eprintln!(
            "Usage: {} <pdf_path> <output_dir> <page_index> <dpi>",
            args[0]
        );
        eprintln!("Example: {} input.pdf /tmp/output 10 300", args[0]);
        eprintln!("Note: DPI is converted to scale internally (300 DPI = scale 4.166666)");
        std::process::exit(1);
    }

    let pdf_path = &args[1];
    let output_dir = &args[2];
    let page_index: i32 = args[3].parse().unwrap_or_else(|_| {
        eprintln!("Invalid page index: {}", args[3]);
        std::process::exit(1);
    });
    let dpi: f64 = args[4].parse().unwrap_or_else(|_| {
        eprintln!("Invalid DPI: {}", args[4]);
        std::process::exit(1);
    });

    // Initialize PDFium
    unsafe {
        pdfium_bridge_init();
    }

    // Create output directory if needed
    if !Path::new(output_dir).exists() {
        std::fs::create_dir_all(output_dir).unwrap_or_else(|e| {
            eprintln!("Failed to create output directory: {}", e);
            std::process::exit(1);
        });
    }

    // Render the page
    let output_path = format!("{}/page_{:04}.ppm", output_dir, page_index);
    match render_page(pdf_path, page_index, &output_path, dpi) {
        Ok(_) => {
            println!("Success!");
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            unsafe {
                pdfium_bridge_destroy();
            }
            std::process::exit(1);
        }
    }

    // Cleanup PDFium
    unsafe {
        pdfium_bridge_destroy();
    }
}
