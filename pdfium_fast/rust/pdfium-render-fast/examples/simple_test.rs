//! Simple test to validate pdfium-render-fast basic functionality
//!
//! Usage: cargo run --example simple_test <pdf_path>

use pdfium_render_fast::{PdfRenderConfig, Pdfium, PixelFormat};
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <pdf_path> [output_dir]", args[0]);
        std::process::exit(1);
    }

    let pdf_path = &args[1];
    let output_dir = args
        .get(2)
        .map(|s| s.as_str())
        .unwrap_or("/tmp/pdfium-render-fast-test");

    // Create output directory
    std::fs::create_dir_all(output_dir)?;

    println!("Testing pdfium-render-fast with: {}", pdf_path);

    // Initialize PDFium
    let pdfium = Pdfium::new()?;
    println!("PDFium initialized successfully");

    // Open the document
    let doc = pdfium.load_pdf_from_file(pdf_path, None)?;
    let page_count = doc.page_count();
    println!("Opened document: {} pages", page_count);

    // Check if document is tagged
    if doc.is_tagged() {
        println!("Document is tagged (has structure tree)");
    }

    // Get metadata
    if let Some(title) = doc.metadata("Title") {
        println!("Title: {}", title);
    }
    if let Some(author) = doc.metadata("Author") {
        println!("Author: {}", author);
    }

    // Process first page (or all if small document)
    let pages_to_process = std::cmp::min(page_count, 3);

    for i in 0..pages_to_process {
        println!("\n--- Page {} ---", i + 1);

        let page = doc.page(i)?;
        let (width, height) = page.size();
        println!("Size: {:.1} x {:.1} points", width, height);

        // Check if scanned
        if page.is_scanned() {
            println!("Page appears to be scanned (single JPEG)");
        }

        // Extract text
        let text = page.text()?;
        let char_count = text.char_count();
        let word_count = text.word_count();
        println!("Text: {} characters, {} words", char_count, word_count);

        // Show first 100 characters
        let all_text = text.all();
        if !all_text.is_empty() {
            let preview: String = all_text.chars().take(100).collect();
            println!("Preview: {}...", preview.replace('\n', " "));
        }

        // Get words with bounding boxes
        let words = text.words();
        if !words.is_empty() {
            println!("First 3 words with bboxes:");
            for word in words.iter().take(3) {
                println!(
                    "  \"{}\" at ({:.1}, {:.1}, {:.1}, {:.1})",
                    word.text, word.left, word.bottom, word.right, word.top
                );
            }
        }

        // Get text cells with font info (batch extraction)
        let cells = text.cells();
        if !cells.is_empty() {
            println!("Text cells (batch extraction): {} total", cells.len());
            println!("First 3 cells:");
            for cell in cells.iter().take(3) {
                let mut flags = String::new();
                if cell.is_bold() {
                    flags.push_str(" bold");
                }
                if cell.is_italic() {
                    flags.push_str(" italic");
                }
                let preview: String = cell.text.chars().take(30).collect();
                println!("  \"{}\" size={:.1}pt{}", preview, cell.font_size, flags);
            }
        }

        // Render page to PNG
        let output_path = format!("{}/page_{}.png", output_dir, i);
        let config = PdfRenderConfig::new()
            .set_target_dpi(150.0) // Lower DPI for testing
            .set_pixel_format(PixelFormat::Bgra);

        let bitmap = page.render_with_config(&config)?;
        println!(
            "Rendered: {}x{} pixels, {} bytes/pixel",
            bitmap.width(),
            bitmap.height(),
            bitmap.format().bytes_per_pixel()
        );

        bitmap.save_as_png(&output_path)?;
        println!("Saved to: {}", output_path);
    }

    // Test parallel rendering if document has more than 1 page
    if page_count > 1 {
        println!("\n--- Parallel Rendering Test ---");

        let config = PdfRenderConfig::new().set_target_dpi(150.0);
        let optimal_threads = doc.optimal_thread_count();
        println!("Optimal thread count for document: {}", optimal_threads);

        let start = std::time::Instant::now();
        let pages = doc.render_pages_parallel(&config)?;
        let elapsed = start.elapsed();

        println!(
            "Rendered {} pages in {:?} ({:.2} pages/sec)",
            pages.len(),
            elapsed,
            pages.len() as f64 / elapsed.as_secs_f64()
        );

        // Save first parallel-rendered page
        if let Some(first_page) = pages.first() {
            let output_path = format!("{}/parallel_page_0.png", output_dir);
            first_page.save_as_png(&output_path)?;
            println!(
                "Saved parallel render: {} ({}x{} pixels)",
                output_path, first_page.width, first_page.height
            );
        }
    }

    println!("\nTest completed successfully!");
    Ok(())
}
