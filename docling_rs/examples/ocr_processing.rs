//! OCR Processing Example
//!
//! This example demonstrates OCR (Optical Character Recognition) for
//! scanned documents and images.
//!
//! **Auto-Detection:** Scanned PDFs are automatically detected! If a PDF has
//! no programmatic text and consists of scanned images, OCR is enabled automatically.
//!
//! Run with:
//! ```bash
//! cargo run --example ocr_processing -- path/to/scanned-document.pdf
//! ```

use docling_backend::DocumentConverter;
use docling_core::Result;
use std::env;
use std::path::Path;

fn main() -> Result<()> {
    // Get input file from command-line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <path-to-document>", args[0]);
        eprintln!("\nExample:");
        eprintln!("  {} scanned_invoice.pdf  # Auto-detected!", args[0]);
        eprintln!("  {} photo.jpg            # Force OCR", args[0]);
        eprintln!("  {} scan.png             # Force OCR", args[0]);
        std::process::exit(1);
    }

    let input_path = &args[1];
    let path = Path::new(input_path);

    println!("Processing document: {input_path}");
    println!();

    // Check if this is an image file (requires explicit OCR)
    let is_image = if let Some(ext) = path.extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();
        matches!(
            ext_str.as_str(),
            "jpg" | "jpeg" | "png" | "tiff" | "tif" | "bmp" | "webp" | "gif"
        )
    } else {
        false
    };

    // Create converter:
    // - Images: Force OCR on (images always need OCR for text extraction)
    // - PDFs: Use auto-detection (scanned PDFs are detected automatically!)
    let converter = if is_image {
        println!("Image file detected - forcing OCR on");
        DocumentConverter::with_ocr(true)?
    } else {
        println!("PDF file - using auto-detection for scanned documents");
        println!("(Scanned PDFs will have OCR enabled automatically)");
        DocumentConverter::new()?
    };

    println!("Converting...");
    println!();

    // Convert the document
    let result = converter.convert(input_path)?;

    // Display conversion results
    println!("✓ Conversion successful!");
    println!();
    println!("Document Metadata:");
    println!("  Pages: {:?}", result.document.metadata.num_pages);
    println!(
        "  Characters extracted: {}",
        result.document.metadata.num_characters
    );
    println!("  Conversion time: {:?}", result.latency);
    println!();

    // Display the markdown output
    println!("Extracted Text (Markdown):");
    println!("{}", "=".repeat(80));
    println!("{}", result.document.markdown);
    println!("{}", "=".repeat(80));

    // Tips for better OCR results
    println!();
    println!("Tips for Better OCR Results:");
    println!("  • Use high-resolution images (300+ DPI)");
    println!("  • Ensure good contrast between text and background");
    println!("  • Avoid skewed or rotated images");
    println!("  • Clean images work better than noisy ones");

    Ok(())
}
