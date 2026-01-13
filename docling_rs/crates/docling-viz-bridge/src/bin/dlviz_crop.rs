//! CLI tool to crop individual elements from PDF visualizations
//!
//! Extracts a zoomed image of a single element for AI review, OCR verification,
//! or detailed inspection.
//!
//! # Usage
//!
//! ```bash
//! # Crop single element by ID
//! dlviz-crop ./review/page_000.json --element 3 --output element_3.png
//!
//! # Crop with padding (10 pixels)
//! dlviz-crop ./review/page_000.json --element 5 --padding 10 --output element_5.png
//!
//! # Crop all elements of a specific type
//! dlviz-crop ./review/page_000.json --label table --output-dir ./tables/
//!
//! # Scale up for OCR verification (2x)
//! dlviz-crop ./review/page_000.json --element 3 --scale 2.0 --output element_3_2x.png
//! ```

use ab_glyph::{FontRef, PxScale};
use clap::Parser;
use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};
use imageproc::drawing::{draw_hollow_rect_mut, draw_text_mut};
use imageproc::rect::Rect;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// =============================================================================
// Data Structures (shared with dlviz-apply-corrections)
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BBoxInfo {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PageSizeInfo {
    width: f32,
    height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ElementInfo {
    id: u32,
    label: String,
    confidence: f32,
    bbox: BBoxInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    reading_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VisualizationSidecar {
    pdf: String,
    page: usize,
    #[serde(default)]
    page_size: Option<PageSizeInfo>,
    stage: String,
    render_time_ms: f64,
    element_count: usize,
    elements: Vec<ElementInfo>,
}

// =============================================================================
// CLI Arguments
// =============================================================================

/// Crop individual elements from PDF visualizations
#[derive(Parser, Debug)]
#[command(name = "dlviz-crop")]
#[command(version, about, long_about = None)]
struct Args {
    /// JSON sidecar file from dlviz-screenshot
    json_file: PathBuf,

    /// Element ID to crop (use --list to see available IDs)
    #[arg(short, long)]
    element: Option<u32>,

    /// Crop all elements with this label
    #[arg(short, long)]
    label: Option<String>,

    /// List available elements and exit
    #[arg(long)]
    list: bool,

    /// Padding around the element (pixels)
    #[arg(short, long, default_value = "5")]
    padding: u32,

    /// Scale factor for output (2.0 = 2x zoom)
    #[arg(short, long, default_value = "1.0")]
    scale: f32,

    /// Output PNG file (for single element mode)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Output directory (for --label mode)
    #[arg(long)]
    output_dir: Option<PathBuf>,

    /// Draw border around cropped element
    #[arg(long)]
    border: bool,

    /// Include element info annotation
    #[arg(long)]
    annotate: bool,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

// =============================================================================
// Crop Functions
// =============================================================================

fn crop_element(
    img: &DynamicImage,
    element: &ElementInfo,
    padding: u32,
    scale: f32,
    draw_border: bool,
    annotate: bool,
) -> RgbaImage {
    let (img_width, img_height) = img.dimensions();

    // Calculate crop region with padding
    let x1 = (element.bbox.x as i32 - padding as i32).max(0) as u32;
    let y1 = (element.bbox.y as i32 - padding as i32).max(0) as u32;
    let x2 = ((element.bbox.x + element.bbox.width) as u32 + padding).min(img_width);
    let y2 = ((element.bbox.y + element.bbox.height) as u32 + padding).min(img_height);

    let crop_width = x2 - x1;
    let crop_height = y2 - y1;

    // Crop the image
    let cropped = img.crop_imm(x1, y1, crop_width, crop_height);

    // Scale if requested
    let scaled = if (scale - 1.0).abs() > 0.01 {
        let new_width = (crop_width as f32 * scale) as u32;
        let new_height = (crop_height as f32 * scale) as u32;
        cropped.resize_exact(new_width, new_height, image::imageops::FilterType::Lanczos3)
    } else {
        cropped
    };

    let mut output = scaled.to_rgba8();

    // Draw border if requested
    if draw_border {
        let (w, h) = output.dimensions();
        let border_color = get_label_color(&element.label);
        let rect = Rect::at(0, 0).of_size(w, h);
        draw_hollow_rect_mut(&mut output, rect, border_color);

        // Draw inner border for visibility
        if w > 4 && h > 4 {
            let inner_rect = Rect::at(1, 1).of_size(w - 2, h - 2);
            draw_hollow_rect_mut(&mut output, inner_rect, border_color);
        }
    }

    // Add annotation if requested
    if annotate {
        add_annotation(&mut output, element, scale);
    }

    output
}

fn get_label_color(label: &str) -> Rgba<u8> {
    match label.to_lowercase().as_str() {
        "title" => Rgba([255, 0, 0, 255]),        // Red
        "text" => Rgba([0, 128, 0, 255]),         // Green
        "table" => Rgba([0, 0, 255, 255]),        // Blue
        "picture" => Rgba([255, 165, 0, 255]),    // Orange
        "caption" => Rgba([128, 0, 128, 255]),    // Purple
        "formula" => Rgba([0, 255, 255, 255]),    // Cyan
        "footnote" => Rgba([255, 192, 203, 255]), // Pink
        "section_header" | "sectionheader" => Rgba([255, 255, 0, 255]), // Yellow
        "list" | "list_item" | "listitem" => Rgba([0, 255, 0, 255]), // Lime
        "code" => Rgba([128, 128, 128, 255]),     // Gray
        "header" | "page_header" | "pageheader" => Rgba([0, 100, 0, 255]), // Dark green
        "footer" | "page_footer" | "pagefooter" => Rgba([139, 69, 19, 255]), // Brown
        _ => Rgba([100, 100, 100, 255]),          // Default gray
    }
}

fn add_annotation(img: &mut RgbaImage, element: &ElementInfo, scale: f32) {
    // Load a basic font (embedded in binary)
    let font_data: &[u8] = include_bytes!("../../assets/DejaVuSansMono.ttf");
    let font = match FontRef::try_from_slice(font_data) {
        Ok(f) => f,
        Err(_) => return, // Skip annotation if font fails to load
    };

    let font_size = (12.0 * scale).max(10.0);
    let px_scale = PxScale::from(font_size);
    let color = Rgba([255, 255, 255, 255]);
    let bg_color = Rgba([0, 0, 0, 200]);

    // Create annotation text
    let text = format!(
        "#{} {} ({:.0}%)",
        element.id,
        element.label,
        element.confidence * 100.0
    );

    // Draw background rectangle for text
    let (w, _h) = img.dimensions();
    let text_width = (text.len() as f32 * font_size * 0.6) as u32;
    let text_height = (font_size * 1.5) as u32;

    // Draw semi-transparent background
    for y in 0..text_height.min(img.height()) {
        for x in 0..text_width.min(w) {
            let pixel = img.get_pixel_mut(x, y);
            // Blend with background
            let alpha = bg_color[3] as f32 / 255.0;
            pixel[0] = ((1.0 - alpha) * pixel[0] as f32 + alpha * bg_color[0] as f32) as u8;
            pixel[1] = ((1.0 - alpha) * pixel[1] as f32 + alpha * bg_color[1] as f32) as u8;
            pixel[2] = ((1.0 - alpha) * pixel[2] as f32 + alpha * bg_color[2] as f32) as u8;
        }
    }

    // Draw text
    draw_text_mut(img, color, 2, 2, px_scale, &font, &text);
}

// =============================================================================
// Main
// =============================================================================

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Read JSON sidecar
    if !args.json_file.exists() {
        eprintln!("Error: JSON file not found: {}", args.json_file.display());
        std::process::exit(1);
    }

    let content = std::fs::read_to_string(&args.json_file)?;
    let sidecar: VisualizationSidecar = serde_json::from_str(&content)?;

    // List mode
    if args.list {
        println!("Elements in {} (page {}):", sidecar.pdf, sidecar.page);
        println!("{:-<60}", "");
        println!(
            "{:>4}  {:>4}  {:12}  {:>6}  {:>6}  {:>6}  {:>6}",
            "ID", "Ord", "Label", "X", "Y", "W", "H"
        );
        println!("{:-<60}", "");

        let mut elements = sidecar.elements.clone();
        elements.sort_by_key(|e| e.reading_order);

        for elem in &elements {
            println!(
                "{:>4}  {:>4}  {:12}  {:>6.0}  {:>6.0}  {:>6.0}  {:>6.0}",
                elem.id,
                elem.reading_order,
                elem.label,
                elem.bbox.x,
                elem.bbox.y,
                elem.bbox.width,
                elem.bbox.height
            );
        }

        println!("{:-<60}", "");
        println!("Total: {} elements", elements.len());

        // Count by label
        let mut label_counts: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::new();
        for elem in &elements {
            *label_counts.entry(&elem.label).or_insert(0) += 1;
        }

        println!("\nBy label:");
        let mut counts: Vec<_> = label_counts.into_iter().collect();
        counts.sort_by(|a, b| b.1.cmp(&a.1));
        for (label, count) in counts {
            println!("  {}: {}", label, count);
        }

        return Ok(());
    }

    // Find corresponding PNG file
    let png_path = args.json_file.with_extension("png");
    if !png_path.exists() {
        eprintln!("Error: PNG file not found: {}", png_path.display());
        eprintln!("(Expected alongside JSON sidecar)");
        std::process::exit(1);
    }

    let img = image::open(&png_path)?;
    if args.verbose {
        println!(
            "Loaded image: {} ({}x{})",
            png_path.display(),
            img.width(),
            img.height()
        );
    }

    // Determine which elements to crop
    let elements_to_crop: Vec<&ElementInfo> = if let Some(element_id) = args.element {
        // Single element by ID
        match sidecar.elements.iter().find(|e| e.id == element_id) {
            Some(elem) => vec![elem],
            None => {
                eprintln!("Error: Element ID {} not found", element_id);
                eprintln!("Use --list to see available element IDs");
                std::process::exit(1);
            }
        }
    } else if let Some(ref label) = args.label {
        // All elements with matching label
        let matches: Vec<_> = sidecar
            .elements
            .iter()
            .filter(|e| e.label.to_lowercase() == label.to_lowercase())
            .collect();

        if matches.is_empty() {
            eprintln!("Error: No elements found with label '{}'", label);
            eprintln!("Use --list to see available labels");
            std::process::exit(1);
        }

        matches
    } else {
        eprintln!("Error: Must specify --element ID or --label TYPE");
        eprintln!("Use --list to see available elements");
        std::process::exit(1);
    };

    // Determine output location
    // Use single-element mode only when --element is specified (not --label)
    let use_single_mode = args.element.is_some() && elements_to_crop.len() == 1;

    if use_single_mode {
        // Single element - use --output or default
        let output_path = args.output.clone().unwrap_or_else(|| {
            let stem = args
                .json_file
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("element");
            PathBuf::from(format!("{}_element_{}.png", stem, elements_to_crop[0].id))
        });

        let cropped = crop_element(
            &img,
            elements_to_crop[0],
            args.padding,
            args.scale,
            args.border,
            args.annotate,
        );
        cropped.save(&output_path)?;

        println!(
            "Saved: {} ({} x {})",
            output_path.display(),
            cropped.width(),
            cropped.height()
        );
        if args.verbose {
            let elem = elements_to_crop[0];
            println!(
                "  Element #{}: {} (conf: {:.0}%)",
                elem.id,
                elem.label,
                elem.confidence * 100.0
            );
            println!(
                "  BBox: ({:.0}, {:.0}) {}x{}",
                elem.bbox.x, elem.bbox.y, elem.bbox.width, elem.bbox.height
            );
            if let Some(ref text) = elem.text {
                let preview: String = text.chars().take(50).collect();
                println!("  Text: {}...", preview);
            }
        }
    } else {
        // Multiple elements - use --output-dir
        let output_dir = args.output_dir.clone().unwrap_or_else(|| {
            let stem = args
                .json_file
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("crops");
            PathBuf::from(format!("{}_crops", stem))
        });

        std::fs::create_dir_all(&output_dir)?;

        println!(
            "Cropping {} elements to {}/",
            elements_to_crop.len(),
            output_dir.display()
        );

        for elem in &elements_to_crop {
            let output_path = output_dir.join(format!(
                "{}_{:03}_{}.png",
                elem.label, elem.id, elem.reading_order
            ));

            let cropped = crop_element(
                &img,
                elem,
                args.padding,
                args.scale,
                args.border,
                args.annotate,
            );
            cropped.save(&output_path)?;

            if args.verbose {
                println!(
                    "  Saved: {} ({}x{})",
                    output_path.display(),
                    cropped.width(),
                    cropped.height()
                );
            }
        }

        println!("Done: {} elements cropped", elements_to_crop.len());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_label_color() {
        assert_eq!(get_label_color("title"), Rgba([255, 0, 0, 255]));
        assert_eq!(get_label_color("text"), Rgba([0, 128, 0, 255]));
        assert_eq!(get_label_color("table"), Rgba([0, 0, 255, 255]));
        assert_eq!(get_label_color("TITLE"), Rgba([255, 0, 0, 255])); // Case insensitive
    }

    #[test]
    fn test_bbox_info() {
        let bbox = BBoxInfo {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };
        assert_eq!(bbox.x, 10.0);
        assert_eq!(bbox.width, 100.0);
    }
}
