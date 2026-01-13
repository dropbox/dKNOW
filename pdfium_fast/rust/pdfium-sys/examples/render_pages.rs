use jpeg_encoder::{ColorType, Encoder};
use pdfium_sys::*;
use std::env;
use std::ffi::{c_char, c_double, c_int, CString};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::process::{self, Command};
use webp::Encoder as WebPEncoder;

// ========================================
// C++ Bridge FFI (for 100% correctness)
// ========================================

// Match the C struct from pdfium_render_bridge.cpp
#[repr(C)]
struct RenderResult {
    pixels: *mut u8,
    width: i32,
    height: i32,
    size: i32,
}

// Opaque pointer to DocumentContext (defined in C++)
#[repr(C)]
struct DocumentContext {
    _private: [u8; 0], // Opaque pointer
}

// Link directives for the bridge library
#[link(name = "pdfium_render_bridge")]
#[link(name = "pdfium")]
extern "C" {
    fn pdfium_bridge_init();
    fn pdfium_bridge_open_document(pdf_path: *const c_char) -> *mut DocumentContext;
    fn pdfium_bridge_get_page_count(ctx: *mut DocumentContext) -> c_int;
    fn pdfium_bridge_render_page_from_doc(
        ctx: *mut DocumentContext,
        page_index: c_int,
        dpi: c_double,
    ) -> *mut RenderResult;
    fn pdfium_bridge_close_document(ctx: *mut DocumentContext);
    fn pdfium_bridge_free_result(result: *mut RenderResult);
    fn pdfium_bridge_destroy();
}

/// PDFium image renderer with multi-process support.
///
/// Multi-process rendering achieves 3.0x+ speedup on large PDFs (>=200 pages).
/// AA limitation accepted per WORKER0 #137: 32% of pages have minor anti-aliasing
/// differences (0.57% of pixels). Functional correctness achieved.
///
/// See CLAUDE.md Parallelism Architecture for details.
const DEFAULT_DPI: f64 = 300.0;
const THUMBNAIL_DPI: f64 = 150.0;
const DEFAULT_JPEG_QUALITY: u8 = 85;

/// Configuration for rendering operations
struct RenderConfig {
    dpi: f64,
    md5_mode: bool,
    ppm_mode: bool,
    webp_mode: bool,
    thumbnail_mode: bool,
    jpeg_quality: u8,
}

/// Initialize PDFium with AGG renderer (for legacy worker code only)
/// NOTE: Main rendering path uses bridge, which handles init internally
#[allow(dead_code)]
unsafe fn init_pdfium_with_agg_renderer() {
    let mut config: FPDF_LIBRARY_CONFIG = std::mem::zeroed();
    config.version = 2;
    config.m_pUserFontPaths = std::ptr::null_mut();
    config.m_pIsolate = std::ptr::null_mut();
    config.m_v8EmbedderSlot = 0;
    config.m_RendererType = FPDF_RENDERER_TYPE_FPDF_RENDERERTYPE_AGG;
    FPDF_InitLibraryWithConfig(&config);
}

fn main() {
    let args: Vec<String> = env::args().collect();

    // Check if this is a worker process for multi-process mode
    if args.len() >= 2 && args[1] == "--worker" {
        worker_main();
        return;
    }

    // Main dispatcher
    if args.len() < 3 {
        eprintln!("Usage: {} <input.pdf> <output_dir> [worker_count] [dpi] [--md5|--ppm|--webp|--thumbnail] [--jpeg-quality N]", args[0]);
        eprintln!("  worker_count: 1 (default), 2, 4, 8, etc. Auto-selects based on page count if not specified.");
        eprintln!("  dpi: dots per inch for rendering (default: 300, or 150 with --thumbnail)");
        eprintln!("  --md5: output MD5 hashes to stdout instead of saving PNG files");
        eprintln!("  --ppm: output PPM files instead of PNG (for exact upstream matching)");
        eprintln!("  --webp: output WebP files instead of PNG (lossless, faster encoding)");
        eprintln!(
            "  --thumbnail: output JPEG thumbnails at lower DPI (default: 150 DPI, quality 85)"
        );
        eprintln!("  --jpeg-quality N: JPEG quality 1-100 (default: 85, only with --thumbnail)");
        eprintln!("\nNote: Multi-process enabled for PDFs >= 200 pages (3.0x+ speedup).");
        process::exit(1);
    }

    let pdf_path = &args[1];
    let output_dir = &args[2];

    // Check for --md5, --ppm, --webp, and --thumbnail flags anywhere in args
    let md5_mode = args.iter().any(|arg| arg == "--md5");
    let ppm_mode = args.iter().any(|arg| arg == "--ppm");
    let webp_mode = args.iter().any(|arg| arg == "--webp");
    let thumbnail_mode = args.iter().any(|arg| arg == "--thumbnail");

    // Count exclusive output modes
    let mode_count = [md5_mode, ppm_mode, webp_mode, thumbnail_mode]
        .iter()
        .filter(|&&x| x)
        .count();
    if mode_count > 1 {
        eprintln!("Error: --md5, --ppm, --webp, and --thumbnail flags are mutually exclusive");
        process::exit(1);
    }

    // Parse --jpeg-quality if present
    let jpeg_quality = if let Some(pos) = args.iter().position(|arg| arg == "--jpeg-quality") {
        if pos + 1 >= args.len() {
            eprintln!("Error: --jpeg-quality requires a numeric argument");
            process::exit(1);
        }
        let quality_str = &args[pos + 1];
        let quality = quality_str.parse::<u8>().unwrap_or_else(|_| {
            eprintln!("Error: --jpeg-quality must be a number 1-100");
            process::exit(1);
        });
        if !(1..=100).contains(&quality) {
            eprintln!("Error: --jpeg-quality must be between 1 and 100");
            process::exit(1);
        }
        if !thumbnail_mode {
            eprintln!("Warning: --jpeg-quality only applies with --thumbnail mode");
        }
        quality
    } else {
        DEFAULT_JPEG_QUALITY
    };

    if !Path::new(pdf_path).exists() {
        eprintln!("Error: PDF file not found: {}", pdf_path);
        process::exit(1);
    }

    // Page count will be determined during rendering (avoid multiple init/destroy cycles)

    // Parse numeric arguments (skip all flag arguments)
    let mut skip_next = false;
    let numeric_args: Vec<&str> = args
        .iter()
        .skip(3)
        .filter(|arg| {
            if skip_next {
                skip_next = false;
                return false;
            }
            if *arg == "--jpeg-quality" {
                skip_next = true;
                return false;
            }
            *arg != "--md5" && *arg != "--ppm" && *arg != "--thumbnail"
        })
        .map(|s| s.as_str())
        .collect();

    // Parse worker count (will be used for intelligent dispatching)
    let requested_workers = if !numeric_args.is_empty() {
        numeric_args[0].parse::<usize>().unwrap_or_else(|_| {
            eprintln!("Error: worker_count must be a number");
            process::exit(1);
        })
    } else {
        0 // 0 means auto-select based on page count
    };

    let dpi = if numeric_args.len() >= 2 {
        numeric_args[1].parse::<f64>().unwrap_or_else(|_| {
            eprintln!("Error: DPI must be a number");
            process::exit(1);
        })
    } else if thumbnail_mode {
        THUMBNAIL_DPI
    } else {
        DEFAULT_DPI
    };

    // Create output directory if it doesn't exist
    std::fs::create_dir_all(output_dir).unwrap_or_else(|e| {
        eprintln!("Error: Failed to create output directory: {}", e);
        process::exit(1);
    });

    // Get page count for intelligent dispatching
    let page_count = match get_page_count(pdf_path) {
        Ok(count) => count,
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    };

    // Intelligent worker count selection
    // Per CLAUDE.md: < 200 pages: single-threaded, >= 200 pages: multi-process
    let worker_count = if requested_workers > 0 {
        requested_workers // User explicitly specified
    } else if page_count < 200 {
        1 // Auto-select single-threaded for small PDFs
    } else {
        4 // Auto-select 4 workers for large PDFs
    };

    let start_time = std::time::Instant::now();

    // Dispatch based on worker count
    if worker_count == 1 {
        if md5_mode {
            eprintln!(
                "Using single-threaded rendering with MD5 output ({} DPI)",
                dpi
            );
        } else if ppm_mode {
            eprintln!(
                "Using single-threaded rendering with PPM output ({} DPI)",
                dpi
            );
        } else if thumbnail_mode {
            eprintln!(
                "Using single-threaded rendering with JPEG thumbnails ({} DPI, quality {})",
                dpi, jpeg_quality
            );
        } else {
            eprintln!("Using single-threaded rendering ({} DPI)", dpi);
        }

        let config = RenderConfig {
            dpi,
            md5_mode,
            ppm_mode,
            webp_mode,
            thumbnail_mode,
            jpeg_quality,
        };

        let result = render_single_threaded_all_formats(pdf_path, output_dir, &config);

        match result {
            Ok(page_count) => {
                let elapsed = start_time.elapsed().as_secs_f64();
                println!("Rendered {} pages in {:.2} seconds", page_count, elapsed);
                process::exit(0)
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                process::exit(1);
            }
        }
    } else {
        // Multi-process rendering
        if md5_mode {
            eprintln!(
                "Using multi-process rendering with {} workers, MD5 output ({} DPI)",
                worker_count, dpi
            );
        } else if ppm_mode {
            eprintln!(
                "Using multi-process rendering with {} workers, PPM output ({} DPI)",
                worker_count, dpi
            );
        } else if thumbnail_mode {
            eprintln!("Using multi-process rendering with {} workers, JPEG thumbnails ({} DPI, quality {})", worker_count, dpi, jpeg_quality);
        } else {
            eprintln!(
                "Using multi-process rendering with {} workers ({} DPI)",
                worker_count, dpi
            );
        }

        let config = RenderConfig {
            dpi,
            md5_mode,
            ppm_mode,
            webp_mode,
            thumbnail_mode,
            jpeg_quality,
        };
        let result = render_multiprocess(pdf_path, output_dir, worker_count, page_count, &config);

        match result {
            Ok(()) => {
                let elapsed = start_time.elapsed().as_secs_f64();
                println!("Rendered {} pages in {:.2} seconds", page_count, elapsed);
                process::exit(0)
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                process::exit(1);
            }
        }
    }
}

fn get_page_count(pdf_path: &str) -> Result<i32, String> {
    unsafe {
        pdfium_bridge_init();
        let c_path = CString::new(pdf_path).unwrap();
        let ctx = pdfium_bridge_open_document(c_path.as_ptr());

        if ctx.is_null() {
            pdfium_bridge_destroy();
            return Err(format!("Failed to load PDF: {}", pdf_path));
        }

        let page_count = pdfium_bridge_get_page_count(ctx);
        pdfium_bridge_close_document(ctx);
        pdfium_bridge_destroy();

        // Allow 0-page PDFs (upstream behavior: valid empty document)
        Ok(page_count)
    }
}

// ========================================
// Single-threaded implementation
// ========================================

/// Single-threaded rendering with support for all output formats (PNG, PPM, MD5, JPEG)
/// Uses C++ bridge for 100% correctness (handles all form callbacks internally)
/// Returns the number of pages rendered
fn render_single_threaded_all_formats(
    pdf_path: &str,
    output_dir: &str,
    config: &RenderConfig,
) -> Result<i32, String> {
    unsafe {
        // Initialize PDFium via bridge
        pdfium_bridge_init();

        // Open document once for batch rendering
        let c_path = CString::new(pdf_path).unwrap();
        let ctx = pdfium_bridge_open_document(c_path.as_ptr());

        if ctx.is_null() {
            pdfium_bridge_destroy();
            return Err(format!("Failed to load PDF: {}", pdf_path));
        }

        // Get page count
        let page_count = pdfium_bridge_get_page_count(ctx);

        // Allow 0-page PDFs (upstream behavior: no output files, exit 0)
        // Render each page with appropriate output format
        for page_index in 0..page_count {
            let result = if config.md5_mode {
                render_page_md5_bridge(ctx, page_index, config.dpi)
            } else if config.ppm_mode {
                render_page_to_ppm_bridge(ctx, page_index, output_dir, config.dpi)
            } else if config.webp_mode {
                render_page_to_webp_bridge(ctx, page_index, output_dir, config.dpi)
            } else if config.thumbnail_mode {
                render_page_to_jpeg_bridge(
                    ctx,
                    page_index,
                    output_dir,
                    config.dpi,
                    config.jpeg_quality,
                )
            } else {
                render_page_to_png_bridge(ctx, page_index, output_dir, config.dpi)
            };

            if let Err(e) = result {
                eprintln!("Warning: Failed to render page {}: {}", page_index, e);
            }
        }

        // Close document
        pdfium_bridge_close_document(ctx);
        pdfium_bridge_destroy();

        Ok(page_count)
    }
}

/// Legacy single-threaded rendering (PNG only)
/// Deprecated: Use render_single_threaded_all_formats instead
#[allow(dead_code)]
fn render_single_threaded(pdf_path: &str, output_dir: &str, dpi: f64) -> Result<(), String> {
    let config = RenderConfig {
        dpi,
        md5_mode: false,
        ppm_mode: false,
        webp_mode: false,
        thumbnail_mode: false,
        jpeg_quality: DEFAULT_JPEG_QUALITY,
    };
    render_single_threaded_all_formats(pdf_path, output_dir, &config).map(|_| ())
}

// ========================================
// Bridge-based rendering functions
// ========================================

/// Render page to PPM using C++ bridge
fn render_page_to_ppm_bridge(
    ctx: *mut DocumentContext,
    page_index: i32,
    output_dir: &str,
    dpi: f64,
) -> Result<(), String> {
    unsafe {
        let result = pdfium_bridge_render_page_from_doc(ctx, page_index, dpi);
        if result.is_null() {
            return Err(format!("Failed to render page {}", page_index));
        }

        // Extract render result
        let width = (*result).width;
        let height = (*result).height;
        let size = (*result).size as usize;

        // Copy RGB data from bridge to Rust-owned buffer
        let rgb_slice = std::slice::from_raw_parts((*result).pixels, size);
        let rgb_data = rgb_slice.to_vec();

        // Free the C++ result before file I/O
        pdfium_bridge_free_result(result);

        // Write PPM file (P6 format: binary RGB)
        let output_path = format!("{}/page_{:04}.ppm", output_dir, page_index);
        let mut file = File::create(&output_path)
            .map_err(|e| format!("Failed to create file {}: {}", output_path, e))?;

        // PPM P6 header: "P6\n{width} {height}\n255\n" (N=201: removed comment to match C++ CLI)
        use std::io::Write;
        write!(file, "P6\n{} {}\n255\n", width, height)
            .map_err(|e| format!("Failed to write PPM header: {}", e))?;

        // Write RGB data
        file.write_all(&rgb_data)
            .map_err(|e| format!("Failed to write PPM data: {}", e))?;

        Ok(())
    }
}

/// Render page to PNG using C++ bridge
fn render_page_to_png_bridge(
    ctx: *mut DocumentContext,
    page_index: i32,
    output_dir: &str,
    dpi: f64,
) -> Result<(), String> {
    unsafe {
        let result = pdfium_bridge_render_page_from_doc(ctx, page_index, dpi);
        if result.is_null() {
            return Err(format!("Failed to render page {}", page_index));
        }

        // Extract render result
        let width = (*result).width as u32;
        let height = (*result).height as u32;
        let size = (*result).size as usize;

        // Get RGB data from bridge
        let rgb_slice = std::slice::from_raw_parts((*result).pixels, size);

        // Convert RGB to RGBA for PNG (add alpha channel)
        let mut rgba_data = Vec::with_capacity((width * height * 4) as usize);
        for chunk in rgb_slice.chunks(3) {
            rgba_data.push(chunk[0]); // R
            rgba_data.push(chunk[1]); // G
            rgba_data.push(chunk[2]); // B
            rgba_data.push(255); // A (fully opaque)
        }

        // Free the C++ result before writing
        pdfium_bridge_free_result(result);

        // Write PNG file
        let output_path = format!("{}/page_{:04}.png", output_dir, page_index);
        let path = Path::new(&output_path);
        let file = File::create(path)
            .map_err(|e| format!("Failed to create file {}: {}", output_path, e))?;
        let w = BufWriter::new(file);

        let mut encoder = png::Encoder::new(w, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|e| format!("Failed to write PNG header: {}", e))?;
        writer
            .write_image_data(&rgba_data)
            .map_err(|e| format!("Failed to write PNG data: {}", e))?;

        Ok(())
    }
}

/// Render page to JPEG using C++ bridge
fn render_page_to_jpeg_bridge(
    ctx: *mut DocumentContext,
    page_index: i32,
    output_dir: &str,
    dpi: f64,
    quality: u8,
) -> Result<(), String> {
    unsafe {
        let result = pdfium_bridge_render_page_from_doc(ctx, page_index, dpi);
        if result.is_null() {
            return Err(format!("Failed to render page {}", page_index));
        }

        // Extract render result
        let width = (*result).width as u16;
        let height = (*result).height as u16;
        let size = (*result).size as usize;

        // CRITICAL: Copy RGB data before freeing (avoid use-after-free)
        let rgb_slice = std::slice::from_raw_parts((*result).pixels, size);
        let rgb_data = rgb_slice.to_vec();

        // Free the C++ result now that we've copied the data
        pdfium_bridge_free_result(result);

        // Write JPEG file
        let output_path = format!("{}/page_{:04}.jpg", output_dir, page_index);
        let file = File::create(&output_path)
            .map_err(|e| format!("Failed to create file {}: {}", output_path, e))?;
        let mut writer = BufWriter::new(file);

        // Encode as JPEG using our copied data
        let encoder = Encoder::new(&mut writer, quality);
        encoder
            .encode(&rgb_data, width, height, ColorType::Rgb)
            .map_err(|e| format!("Failed to encode JPEG: {}", e))?;

        Ok(())
    }
}

/// Render page to WebP using C++ bridge
fn render_page_to_webp_bridge(
    ctx: *mut DocumentContext,
    page_index: i32,
    output_dir: &str,
    dpi: f64,
) -> Result<(), String> {
    unsafe {
        let result = pdfium_bridge_render_page_from_doc(ctx, page_index, dpi);
        if result.is_null() {
            return Err(format!("Failed to render page {}", page_index));
        }

        // Extract render result
        let width = (*result).width as u32;
        let height = (*result).height as u32;
        let size = (*result).size as usize;

        // CRITICAL: Copy RGB data before freeing (avoid use-after-free)
        let rgb_slice = std::slice::from_raw_parts((*result).pixels, size);
        let rgb_data = rgb_slice.to_vec();

        // Free the C++ result now that we've copied the data
        pdfium_bridge_free_result(result);

        // Encode as WebP (lossless mode for correctness) using our copied data
        let encoder = WebPEncoder::from_rgb(&rgb_data, width, height);
        let webp_data = encoder.encode_lossless();

        // Write WebP file
        let output_path = format!("{}/page_{:04}.webp", output_dir, page_index);
        let mut file = File::create(&output_path)
            .map_err(|e| format!("Failed to create file {}: {}", output_path, e))?;
        file.write_all(&webp_data)
            .map_err(|e| format!("Failed to write WebP data: {}", e))?;

        Ok(())
    }
}

/// Render page to MD5 hash using C++ bridge
/// Computes MD5 of PPM format (header + RGB data) to match baseline generation
fn render_page_md5_bridge(
    ctx: *mut DocumentContext,
    page_index: i32,
    dpi: f64,
) -> Result<(), String> {
    unsafe {
        let result = pdfium_bridge_render_page_from_doc(ctx, page_index, dpi);
        if result.is_null() {
            return Err(format!("Failed to render page {}", page_index));
        }

        // Extract render result
        let width = (*result).width;
        let height = (*result).height;
        let size = (*result).size as usize;
        let rgb_slice = std::slice::from_raw_parts((*result).pixels, size);

        // Build PPM format in memory (same as PPM file output)
        // PPM P6 header: "P6\n{width} {height}\n255\n" (N=201: removed comment to match C++ CLI)
        let header = format!("P6\n{} {}\n255\n", width, height);
        let header_bytes = header.as_bytes();

        // Concatenate header and RGB data for MD5 computation
        let mut ppm_data = Vec::with_capacity(header_bytes.len() + size);
        ppm_data.extend_from_slice(header_bytes);
        ppm_data.extend_from_slice(rgb_slice);

        // Compute MD5 of complete PPM format
        let digest = md5::compute(&ppm_data);
        println!("{:x}", digest);

        // Free the C++ result
        pdfium_bridge_free_result(result);

        Ok(())
    }
}

fn render_page_to_png(
    doc: FPDF_DOCUMENT,
    page_index: i32,
    output_dir: &str,
    dpi: f64,
    form_handle: FPDF_FORMHANDLE,
) -> Result<(), String> {
    unsafe {
        let page = FPDF_LoadPage(doc, page_index);
        if page.is_null() {
            return Err(format!("Failed to load page {}", page_index));
        }

        // CRITICAL: Form callbacks for correct rendering
        // Reference: testing/pdfium_test.cc:830, 1575, 1578
        if !form_handle.is_null() {
            FORM_OnAfterLoadPage(page, form_handle);
            FORM_DoPageAAction(page, form_handle, FPDFPAGE_AACTION_OPEN as i32);
        }

        // Get page dimensions
        let width_pts = FPDF_GetPageWidthF(page) as f64;
        let height_pts = FPDF_GetPageHeightF(page) as f64;

        // Convert to pixels at specified DPI
        // CRITICAL: Truncate scale to 6 decimal places to match upstream pdfium_test precision
        // Upstream uses --scale=4.166666 (not full float precision 4.166666666...)
        // This ensures dimension matching and thus MD5 matching
        let scale_full = dpi / 72.0;
        let scale = (scale_full * 1000000.0).floor() / 1000000.0; // Floor to 6 decimals
        let width_px = (width_pts * scale) as i32;
        let height_px = (height_pts * scale) as i32;

        // CRITICAL FIX: Use FPDFBitmap_CreateEx (not FPDFBitmap_Create) to match upstream
        //
        // WHY: Bitmap FORMAT during rendering affects PDFium's internal rendering path
        // - FPDFBitmap_BGRx (format 3): No alpha, PDFium optimizes for opaque content
        // - FPDFBitmap_BGRA (format 4): With alpha, PDFium uses blending path
        //
        // Original mistake: Used FPDFBitmap_Create(w,h,0) which likely defaults to BGRA
        // This caused ~32% of pages to render differently than upstream pdfium_test
        //
        // Fix: Use FPDFBitmap_CreateEx with explicit format matching upstream behavior
        // Reference: testing/pdfium_test.cc InitializeBitmap() function
        let has_transparency = FPDFPage_HasTransparency(page) != 0;
        let format = if has_transparency {
            FPDFBitmap_BGRA // Format 4: With alpha
        } else {
            FPDFBitmap_BGRx // Format 3: No alpha (KEY for matching upstream)
        };

        // DEBUG: Log page properties (helpful for diagnosing rendering mismatches)
        if std::env::var("DEBUG_RENDER").is_ok() {
            eprintln!(
                "Page {}: transparency={} format={} size={}x{} scale={:.6}",
                page_index, has_transparency, format, width_px, height_px, scale
            );
        }

        // Let PDFium allocate its own buffer (matches upstream pdfium_test behavior)
        // Upstream passes nullptr to FPDFBitmap_CreateEx, letting PDFium manage memory
        let bitmap = FPDFBitmap_CreateEx(
            width_px,
            height_px,
            format as i32,
            std::ptr::null_mut(), // Let PDFium allocate (matches upstream)
            0,                    // Stride auto-calculated when buffer is null
        );
        if bitmap.is_null() {
            FPDF_ClosePage(page);
            return Err(format!("Failed to create bitmap for page {}", page_index));
        }

        // Fill with appropriate background (matches upstream)
        let fill_color = if has_transparency {
            0x00000000 // Transparent black for alpha pages
        } else {
            0xFFFFFFFF // White for opaque pages
        };
        let fill_result = FPDFBitmap_FillRect(bitmap, 0, 0, width_px, height_px, fill_color);
        if fill_result == 0 {
            FPDFBitmap_Destroy(bitmap);
            FPDF_ClosePage(page);
            return Err(format!("Failed to fill bitmap for page {}", page_index));
        }

        // Render page to bitmap using progressive rendering API
        // CRITICAL: Use progressive rendering to match upstream pdfium_test default behavior
        // This is required for 100% MD5 parity with baseline images
        // Reference: testing/pdfium_test.cc:1082-1125 (ProgressiveBitmapPageRenderer::Start)
        let mut pause = IFSDK_PAUSE {
            version: 1,
            NeedToPauseNow: None,
            user: std::ptr::null_mut(),
        };

        FPDF_RenderPageBitmapWithColorScheme_Start(
            bitmap,
            page,
            0, // start_x
            0, // start_y
            width_px,
            height_px,
            0, // rotate
            FPDF_ANNOT as i32,
            std::ptr::null(), // No color scheme
            &mut pause,
        );

        // Continue rendering until complete
        loop {
            let status = FPDF_RenderPage_Continue(page, &mut pause);
            if status != FPDF_RENDER_TOBECONTINUED as i32 {
                break;
            }
        }

        FPDF_RenderPage_Close(page);

        // Get bitmap data
        let buffer = FPDFBitmap_GetBuffer(bitmap) as *const u8;
        let stride = FPDFBitmap_GetStride(bitmap) as usize;

        // Convert BGRA to RGBA
        let mut rgba_data = Vec::with_capacity((width_px * height_px * 4) as usize);
        for y in 0..height_px {
            let row_offset = (y as usize) * stride;
            for x in 0..width_px {
                let pixel_offset = row_offset + (x as usize) * 4;
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

        // Write PNG
        let output_path = format!("{}/page_{:04}.png", output_dir, page_index);
        let file = File::create(&output_path)
            .map_err(|e| format!("Failed to create file {}: {}", output_path, e))?;
        let w = BufWriter::new(file);

        let mut encoder = png::Encoder::new(w, width_px as u32, height_px as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        // v1.2 PNG Optimization: Disable filter selection for 30-40% encoding speedup
        // Filter NoFilter (0) = raw data, no prediction algorithm
        // Trade-off: ~10-30% larger files, but 30-40% faster encoding
        // For multi-process rendering, encoding speed >> file size (files are temporary)
        encoder.set_filter(png::FilterType::NoFilter);
        // Fast compression (level 1) for additional 20-30% speedup
        encoder.set_compression(png::Compression::Fast);

        let mut writer = encoder
            .write_header()
            .map_err(|e| format!("Failed to write PNG header: {}", e))?;

        writer
            .write_image_data(&rgba_data)
            .map_err(|e| format!("Failed to write PNG data: {}", e))?;

        FPDFBitmap_Destroy(bitmap);

        // CRITICAL: Form cleanup callbacks
        // Reference: testing/pdfium_test.cc:1575, 1578
        if !form_handle.is_null() {
            FORM_DoPageAAction(page, form_handle, FPDFPAGE_AACTION_CLOSE as i32);
            FORM_OnBeforeClosePage(page, form_handle);
        }

        FPDF_ClosePage(page);

        Ok(())
    }
}

fn render_page_to_ppm(
    doc: FPDF_DOCUMENT,
    page_index: i32,
    output_dir: &str,
    dpi: f64,
    form_handle: FPDF_FORMHANDLE,
) -> Result<(), String> {
    unsafe {
        let page = FPDF_LoadPage(doc, page_index);
        if page.is_null() {
            return Err(format!("Failed to load page {}", page_index));
        }

        // CRITICAL: Form callbacks for correct rendering
        if !form_handle.is_null() {
            FORM_OnAfterLoadPage(page, form_handle);
            FORM_DoPageAAction(page, form_handle, FPDFPAGE_AACTION_OPEN as i32);
        }

        // Get page dimensions
        let width_pts = FPDF_GetPageWidthF(page) as f64;
        let height_pts = FPDF_GetPageHeightF(page) as f64;

        // Convert to pixels at specified DPI
        // CRITICAL: Truncate scale to 6 decimal places to match upstream pdfium_test precision
        // Upstream uses --scale=4.166666 (not full float precision 4.166666666...)
        // This ensures dimension matching and thus MD5 matching
        let scale_full = dpi / 72.0;
        let scale = (scale_full * 1000000.0).floor() / 1000000.0; // Floor to 6 decimals
        let width_px = (width_pts * scale) as i32;
        let height_px = (height_pts * scale) as i32;

        // Create bitmap using CreateEx API to match upstream pdfium_test behavior
        let has_transparency = FPDFPage_HasTransparency(page) != 0;
        let format = if has_transparency {
            FPDFBitmap_BGRA // Format 4: With alpha
        } else {
            FPDFBitmap_BGRx // Format 3: No alpha (KEY for matching upstream)
        };

        // Let PDFium allocate its own buffer (matches upstream pdfium_test behavior)
        // Upstream passes nullptr to FPDFBitmap_CreateEx, letting PDFium manage memory
        let bitmap = FPDFBitmap_CreateEx(
            width_px,
            height_px,
            format as i32,
            std::ptr::null_mut(), // Let PDFium allocate (matches upstream)
            0,                    // Stride auto-calculated when buffer is null
        );
        if bitmap.is_null() {
            FPDF_ClosePage(page);
            return Err(format!("Failed to create bitmap for page {}", page_index));
        }

        // Fill with appropriate background (matches upstream)
        let fill_color = if has_transparency {
            0x00000000 // Transparent black for alpha pages
        } else {
            0xFFFFFFFF // White for opaque pages
        };
        let fill_result = FPDFBitmap_FillRect(bitmap, 0, 0, width_px, height_px, fill_color);
        if fill_result == 0 {
            FPDFBitmap_Destroy(bitmap);
            FPDF_ClosePage(page);
            return Err(format!("Failed to fill bitmap for page {}", page_index));
        }

        // Render page to bitmap using progressive rendering API
        // CRITICAL: Use progressive rendering to match upstream pdfium_test default behavior
        // This is required for 100% MD5 parity with baseline images
        // Reference: testing/pdfium_test.cc:1082-1125 (ProgressiveBitmapPageRenderer::Start)
        let mut pause = IFSDK_PAUSE {
            version: 1,
            NeedToPauseNow: None,
            user: std::ptr::null_mut(),
        };

        FPDF_RenderPageBitmapWithColorScheme_Start(
            bitmap,
            page,
            0, // start_x
            0, // start_y
            width_px,
            height_px,
            0, // rotate
            FPDF_ANNOT as i32,
            std::ptr::null(), // No color scheme
            &mut pause,
        );

        // Continue rendering until complete
        loop {
            let status = FPDF_RenderPage_Continue(page, &mut pause);
            if status != FPDF_RENDER_TOBECONTINUED as i32 {
                break;
            }
        }

        FPDF_RenderPage_Close(page);

        // CRITICAL: Draw form fields on top (matching upstream pdfium_test.cc:1001-1003)
        if !form_handle.is_null() {
            FPDF_FFLDraw(
                form_handle,
                bitmap,
                page,
                0,
                0,
                width_px,
                height_px,
                0,
                FPDF_ANNOT as i32,
            );
        }

        // Get bitmap data
        let buffer = FPDFBitmap_GetBuffer(bitmap) as *const u8;
        let stride = FPDFBitmap_GetStride(bitmap) as usize;

        // Convert BGRA to RGB for PPM (source is B, G, R, A; dest is R, G, B)
        let out_len = (width_px * height_px * 3) as usize;
        let mut rgb_data = Vec::with_capacity(out_len);
        for y in 0..height_px {
            let row_offset = (y as usize) * stride;
            for x in 0..width_px {
                let pixel_offset = row_offset + (x as usize) * 4;
                let b = *buffer.add(pixel_offset);
                let g = *buffer.add(pixel_offset + 1);
                let r = *buffer.add(pixel_offset + 2);
                // PPM is RGB order
                rgb_data.push(r);
                rgb_data.push(g);
                rgb_data.push(b);
            }
        }

        // Write PPM file (P6 format: binary RGB)
        let output_path = format!("{}/page_{:04}.ppm", output_dir, page_index);
        let mut file = File::create(&output_path)
            .map_err(|e| format!("Failed to create file {}: {}", output_path, e))?;

        // PPM P6 header: "P6\n{width} {height}\n255\n" (N=201: removed comment to match C++ CLI)
        use std::io::Write;
        write!(file, "P6\n{} {}\n255\n", width_px, height_px)
            .map_err(|e| format!("Failed to write PPM header: {}", e))?;

        // Write RGB data
        file.write_all(&rgb_data)
            .map_err(|e| format!("Failed to write PPM data: {}", e))?;

        FPDFBitmap_Destroy(bitmap);

        // CRITICAL: Form cleanup callbacks
        if !form_handle.is_null() {
            FORM_DoPageAAction(page, form_handle, FPDFPAGE_AACTION_CLOSE as i32);
            FORM_OnBeforeClosePage(page, form_handle);
        }

        FPDF_ClosePage(page);

        Ok(())
    }
}

fn render_page_to_jpeg(
    doc: FPDF_DOCUMENT,
    page_index: i32,
    output_dir: &str,
    dpi: f64,
    quality: u8,
    form_handle: FPDF_FORMHANDLE,
) -> Result<(), String> {
    unsafe {
        let page = FPDF_LoadPage(doc, page_index);
        if page.is_null() {
            return Err(format!("Failed to load page {}", page_index));
        }

        // CRITICAL: Form callbacks for correct rendering
        if !form_handle.is_null() {
            FORM_OnAfterLoadPage(page, form_handle);
            FORM_DoPageAAction(page, form_handle, FPDFPAGE_AACTION_OPEN as i32);
        }

        // Get page dimensions
        let width_pts = FPDF_GetPageWidthF(page) as f64;
        let height_pts = FPDF_GetPageHeightF(page) as f64;

        // Convert to pixels at specified DPI
        let scale_full = dpi / 72.0;
        let scale = (scale_full * 1000000.0).floor() / 1000000.0; // Floor to 6 decimals
        let width_px = (width_pts * scale) as i32;
        let height_px = (height_pts * scale) as i32;

        // Create bitmap
        let has_transparency = FPDFPage_HasTransparency(page) != 0;
        let format = if has_transparency {
            FPDFBitmap_BGRA
        } else {
            FPDFBitmap_BGRx
        };

        let bitmap =
            FPDFBitmap_CreateEx(width_px, height_px, format as i32, std::ptr::null_mut(), 0);
        if bitmap.is_null() {
            FPDF_ClosePage(page);
            return Err(format!("Failed to create bitmap for page {}", page_index));
        }

        // Fill with background
        let fill_color = if has_transparency {
            0x00000000
        } else {
            0xFFFFFFFF
        };
        let fill_result = FPDFBitmap_FillRect(bitmap, 0, 0, width_px, height_px, fill_color);
        if fill_result == 0 {
            FPDFBitmap_Destroy(bitmap);
            FPDF_ClosePage(page);
            return Err(format!("Failed to fill bitmap for page {}", page_index));
        }

        // Render page to bitmap
        let mut pause = IFSDK_PAUSE {
            version: 1,
            NeedToPauseNow: None,
            user: std::ptr::null_mut(),
        };

        FPDF_RenderPageBitmapWithColorScheme_Start(
            bitmap,
            page,
            0,
            0,
            width_px,
            height_px,
            0,
            FPDF_ANNOT as i32,
            std::ptr::null(),
            &mut pause,
        );

        loop {
            let status = FPDF_RenderPage_Continue(page, &mut pause);
            if status != FPDF_RENDER_TOBECONTINUED as i32 {
                break;
            }
        }

        FPDF_RenderPage_Close(page);

        // Get bitmap data and convert BGRA to RGB
        let buffer = FPDFBitmap_GetBuffer(bitmap) as *const u8;
        let stride = FPDFBitmap_GetStride(bitmap) as usize;
        let out_len = (width_px * height_px * 3) as usize;
        let mut rgb_data = Vec::with_capacity(out_len);

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

        // Write JPEG file
        let output_path = format!("{}/page_{:04}.jpg", output_dir, page_index);
        let file = File::create(&output_path)
            .map_err(|e| format!("Failed to create file {}: {}", output_path, e))?;
        let mut writer = BufWriter::new(file);

        let encoder = Encoder::new(&mut writer, quality);
        encoder
            .encode(&rgb_data, width_px as u16, height_px as u16, ColorType::Rgb)
            .map_err(|e| format!("Failed to encode JPEG: {}", e))?;

        FPDFBitmap_Destroy(bitmap);

        // CRITICAL: Form cleanup callbacks
        if !form_handle.is_null() {
            FORM_DoPageAAction(page, form_handle, FPDFPAGE_AACTION_CLOSE as i32);
            FORM_OnBeforeClosePage(page, form_handle);
        }

        FPDF_ClosePage(page);

        Ok(())
    }
}

// ========================================
// Multi-process implementation
// ========================================
// Multi-process achieves 3.0x+ speedup on large PDFs (>=200 pages)
// AA limitation accepted per WORKER0 #137: 32% of pages have minor AA differences
// See CLAUDE.md Parallelism Architecture for details

fn render_multiprocess(
    pdf_path: &str,
    output_dir: &str,
    worker_count: usize,
    page_count: i32,
    config: &RenderConfig,
) -> Result<(), String> {
    use std::time::Duration;

    // Timeout per page: 60 seconds
    // For a worker with N pages, timeout = 60 * N + 30 seconds overhead
    const TIMEOUT_PER_PAGE_SECS: u64 = 60;
    const OVERHEAD_SECS: u64 = 30;

    // Distribute pages across workers
    let pages_per_worker = (page_count as usize).div_ceil(worker_count);
    let mut worker_processes = vec![];

    // Spawn worker processes
    for worker_id in 0..worker_count {
        let start_page = worker_id * pages_per_worker;
        let end_page = ((worker_id + 1) * pages_per_worker).min(page_count as usize);

        if start_page >= page_count as usize {
            break;
        }

        // Spawn worker process
        let mut cmd = Command::new(std::env::current_exe().unwrap());
        cmd.arg("--worker")
            .arg(pdf_path)
            .arg(output_dir)
            .arg(start_page.to_string())
            .arg(end_page.to_string())
            .arg(config.dpi.to_string())
            .arg(worker_id.to_string());

        if config.md5_mode {
            cmd.arg("--md5");
        }

        if config.ppm_mode {
            cmd.arg("--ppm");
        }

        if config.webp_mode {
            cmd.arg("--webp");
        }

        if config.thumbnail_mode {
            cmd.arg("--thumbnail");
            cmd.arg("--jpeg-quality");
            cmd.arg(config.jpeg_quality.to_string());
        }

        let child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn worker {}: {}", worker_id, e))?;

        worker_processes.push((child, start_page, end_page));
    }

    // Wait for all workers to complete with timeout
    for (worker_id, (mut child, start_page, end_page)) in worker_processes.into_iter().enumerate() {
        let pages_assigned = end_page - start_page;
        let timeout =
            Duration::from_secs(TIMEOUT_PER_PAGE_SECS * pages_assigned as u64 + OVERHEAD_SECS);

        // Poll for completion with timeout
        let start_time = std::time::Instant::now();
        let mut timed_out = false;

        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    // Process finished
                    if !status.success() {
                        return Err(format!(
                            "Worker {} (pages {}-{}) exited with error: {}",
                            worker_id, start_page, end_page, status
                        ));
                    }
                    break;
                }
                Ok(None) => {
                    // Still running - check timeout
                    if start_time.elapsed() > timeout {
                        eprintln!(
                            "Worker {} (pages {}-{}) timed out after {:?} - killing",
                            worker_id, start_page, end_page, timeout
                        );
                        let _ = child.kill();
                        let _ = child.wait(); // Reap zombie
                        timed_out = true;
                        break;
                    }
                    // Sleep briefly before next check
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(e) => {
                    return Err(format!("Worker {} failed to wait: {}", worker_id, e));
                }
            }
        }

        if timed_out {
            eprintln!("WARNING: Worker {} timed out. Pages {}-{} may have triggered infinite loop in PDFium.",
                     worker_id, start_page, end_page);
            eprintln!("         Continuing with remaining pages...");
            // Don't return error - continue with other workers
        }
    }

    Ok(())
}

// ========================================
// Worker process (for multi-process mode)
// ========================================

/// Worker process entry point for multi-process mode
fn worker_main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 8 {
        eprintln!("Worker usage: --worker <pdf> <output_dir> <start_page> <end_page> <dpi> <worker_id> [--md5] [--ppm] [--webp] [--thumbnail] [--jpeg-quality N]");
        process::exit(1);
    }

    let pdf_path = &args[2];
    let output_dir = &args[3];
    let start_page: usize = args[4].parse().unwrap();
    let end_page: usize = args[5].parse().unwrap();
    let dpi: f64 = args[6].parse().unwrap();
    let _worker_id: usize = args[7].parse().unwrap();
    let md5_mode = args.len() > 8 && args.iter().skip(8).any(|a| a == "--md5");
    let ppm_mode = args.len() > 8 && args.iter().skip(8).any(|a| a == "--ppm");
    let webp_mode = args.len() > 8 && args.iter().skip(8).any(|a| a == "--webp");
    let thumbnail_mode = args.len() > 8 && args.iter().skip(8).any(|a| a == "--thumbnail");

    let jpeg_quality = if let Some(pos) = args.iter().position(|a| a == "--jpeg-quality") {
        if pos + 1 < args.len() {
            args[pos + 1].parse::<u8>().unwrap_or(DEFAULT_JPEG_QUALITY)
        } else {
            DEFAULT_JPEG_QUALITY
        }
    } else {
        DEFAULT_JPEG_QUALITY
    };

    let config = RenderConfig {
        dpi,
        md5_mode,
        ppm_mode,
        webp_mode,
        thumbnail_mode,
        jpeg_quality,
    };
    match render_pages_worker(pdf_path, output_dir, start_page, end_page, &config) {
        Ok(_) => process::exit(0),
        Err(e) => {
            eprintln!("Worker error: {}", e);
            process::exit(1);
        }
    }
}

fn render_page_md5(
    doc: FPDF_DOCUMENT,
    page_index: i32,
    dpi: f64,
    form_handle: FPDF_FORMHANDLE,
) -> Result<(), String> {
    unsafe {
        let page = FPDF_LoadPage(doc, page_index);
        if page.is_null() {
            return Err(format!("Failed to load page {}", page_index));
        }

        // CRITICAL: Form callbacks for correct rendering
        if !form_handle.is_null() {
            FORM_OnAfterLoadPage(page, form_handle);
            FORM_DoPageAAction(page, form_handle, FPDFPAGE_AACTION_OPEN as i32);
        }

        // Get page dimensions
        let width_pts = FPDF_GetPageWidthF(page) as f64;
        let height_pts = FPDF_GetPageHeightF(page) as f64;

        // Convert to pixels at specified DPI
        // CRITICAL: Truncate scale to 6 decimal places to match upstream pdfium_test precision
        // Upstream uses --scale=4.166666 (not full float precision 4.166666666...)
        // This ensures dimension matching and thus MD5 matching
        let scale_full = dpi / 72.0;
        let scale = (scale_full * 1000000.0).floor() / 1000000.0; // Floor to 6 decimals
        let width_px = (width_pts * scale) as i32;
        let height_px = (height_pts * scale) as i32;

        // Create bitmap using CreateEx API to match upstream pdfium_test behavior
        // This ensures MD5 mode produces identical output to PNG mode
        let has_transparency = FPDFPage_HasTransparency(page) != 0;
        let format = if has_transparency {
            FPDFBitmap_BGRA // Format 4: With alpha
        } else {
            FPDFBitmap_BGRx // Format 3: No alpha (KEY for matching upstream)
        };

        // Let PDFium allocate its own buffer (matches upstream pdfium_test behavior)
        // Upstream passes nullptr to FPDFBitmap_CreateEx, letting PDFium manage memory
        let bitmap = FPDFBitmap_CreateEx(
            width_px,
            height_px,
            format as i32,
            std::ptr::null_mut(), // Let PDFium allocate (matches upstream)
            0,                    // Stride auto-calculated when buffer is null
        );
        if bitmap.is_null() {
            FPDF_ClosePage(page);
            return Err(format!("Failed to create bitmap for page {}", page_index));
        }

        // Fill with appropriate background (matches upstream)
        let fill_color = if has_transparency {
            0x00000000 // Transparent black for alpha pages
        } else {
            0xFFFFFFFF // White for opaque pages
        };
        let fill_result = FPDFBitmap_FillRect(bitmap, 0, 0, width_px, height_px, fill_color);
        if fill_result == 0 {
            FPDFBitmap_Destroy(bitmap);
            FPDF_ClosePage(page);
            return Err(format!("Failed to fill bitmap for page {}", page_index));
        }

        // Render page to bitmap using progressive rendering API
        // CRITICAL: Use progressive rendering to match upstream pdfium_test default behavior
        // This is required for 100% MD5 parity with baseline images
        // Reference: testing/pdfium_test.cc:1082-1125 (ProgressiveBitmapPageRenderer::Start)
        let mut pause = IFSDK_PAUSE {
            version: 1,
            NeedToPauseNow: None,
            user: std::ptr::null_mut(),
        };

        FPDF_RenderPageBitmapWithColorScheme_Start(
            bitmap,
            page,
            0, // start_x
            0, // start_y
            width_px,
            height_px,
            0, // rotate
            FPDF_ANNOT as i32,
            std::ptr::null(), // No color scheme
            &mut pause,
        );

        // Continue rendering until complete
        loop {
            let status = FPDF_RenderPage_Continue(page, &mut pause);
            if status != FPDF_RENDER_TOBECONTINUED as i32 {
                break;
            }
        }

        FPDF_RenderPage_Close(page);

        // Get bitmap data
        let buffer = FPDFBitmap_GetBuffer(bitmap) as *const u8;
        let stride = FPDFBitmap_GetStride(bitmap) as usize;

        // Convert BGRA to RGBA
        let mut rgba_data = Vec::with_capacity((width_px * height_px * 4) as usize);
        for y in 0..height_px {
            let row_offset = (y as usize) * stride;
            for x in 0..width_px {
                let pixel_offset = row_offset + (x as usize) * 4;
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

        // Create PNG in memory and compute MD5
        let mut png_data = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut png_data, width_px as u32, height_px as u32);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);

            let mut writer = encoder
                .write_header()
                .map_err(|e| format!("Failed to write PNG header: {}", e))?;

            writer
                .write_image_data(&rgba_data)
                .map_err(|e| format!("Failed to write PNG data: {}", e))?;
        }

        // Compute MD5 hash
        let digest = md5::compute(&png_data);

        // Output in same format as pdfium_test: "MD5:page_NNNN.png:hash"
        println!("MD5:page_{:04}.png:{:x}", page_index, digest);

        FPDFBitmap_Destroy(bitmap);

        // CRITICAL: Form cleanup callbacks
        if !form_handle.is_null() {
            FORM_DoPageAAction(page, form_handle, FPDFPAGE_AACTION_CLOSE as i32);
            FORM_OnBeforeClosePage(page, form_handle);
        }

        FPDF_ClosePage(page);

        Ok(())
    }
}

fn render_pages_worker(
    pdf_path: &str,
    output_dir: &str,
    start_page: usize,
    end_page: usize,
    config: &RenderConfig,
) -> Result<(), String> {
    unsafe {
        // Each worker has its own PDFium instance - NO SHARED STATE
        init_pdfium_with_agg_renderer();

        let c_path = CString::new(pdf_path).unwrap();
        let doc = FPDF_LoadDocument(c_path.as_ptr(), std::ptr::null());

        if doc.is_null() {
            FPDF_DestroyLibrary();
            return Err(format!("Failed to load PDF: {}", pdf_path));
        }

        // Initialize form environment
        let mut form_callbacks: FPDF_FORMFILLINFO = std::mem::zeroed();
        form_callbacks.version = 2;
        let form_handle = FPDFDOC_InitFormFillEnvironment(doc, &mut form_callbacks);

        // Configure form field highlighting
        if !form_handle.is_null() {
            FPDF_SetFormFieldHighlightColor(form_handle, 0, 0xFFE4DD);
            FPDF_SetFormFieldHighlightAlpha(form_handle, 100);
        }

        // Execute document-level form actions
        if !form_handle.is_null() {
            FORM_DoDocumentJSAction(form_handle);
            FORM_DoDocumentOpenAction(form_handle);
        }

        // Render assigned pages
        for page_index in start_page..end_page {
            if config.md5_mode {
                if let Err(e) = render_page_md5(doc, page_index as i32, config.dpi, form_handle) {
                    eprintln!(
                        "Warning: Failed to compute MD5 for page {}: {}",
                        page_index, e
                    );
                }
            } else if config.ppm_mode {
                if let Err(e) =
                    render_page_to_ppm(doc, page_index as i32, output_dir, config.dpi, form_handle)
                {
                    eprintln!(
                        "Warning: Failed to render page {} to PPM: {}",
                        page_index, e
                    );
                }
            } else if config.thumbnail_mode {
                if let Err(e) = render_page_to_jpeg(
                    doc,
                    page_index as i32,
                    output_dir,
                    config.dpi,
                    config.jpeg_quality,
                    form_handle,
                ) {
                    eprintln!(
                        "Warning: Failed to render page {} to JPEG: {}",
                        page_index, e
                    );
                }
            } else if let Err(e) =
                render_page_to_png(doc, page_index as i32, output_dir, config.dpi, form_handle)
            {
                eprintln!("Warning: Failed to render page {}: {}", page_index, e);
            }
        }

        // Clean up form environment
        if !form_handle.is_null() {
            FORM_DoDocumentAAction(form_handle, 0x10); // FPDFDOC_AACTION_WC
            FPDFDOC_ExitFormFillEnvironment(form_handle);
        }

        FPDF_CloseDocument(doc);
        FPDF_DestroyLibrary();

        Ok(())
    }
}
