//! CLI tool to visualize table structure from PDF documents
//!
//! Renders detailed table structure including cell boundaries, row/column
//! layout, spanning cells, and header indicators.
//!
//! # Usage
//!
//! ```bash
//! # List tables in a PDF
//! dlviz-table paper.pdf --list
//!
//! # Visualize table structure for a specific table
//! dlviz-table paper.pdf --page 0 --table 0 --output table_0.png
//!
//! # Visualize all tables on a page
//! dlviz-table paper.pdf --page 0 --all --output-dir ./tables/
//!
//! # Scale up for clarity
//! dlviz-table paper.pdf --page 0 --table 0 --scale 2.0 --output table_0.png
//! ```

use ab_glyph::{FontRef, PxScale};
use clap::Parser;
use docling_pdf_ml::{PageElement, Pipeline, PipelineConfigBuilder, TableCell, TableElement};
use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};
use imageproc::drawing::{draw_hollow_rect_mut, draw_text_mut};
use imageproc::rect::Rect;
use pdfium_render::prelude::*;
use std::path::PathBuf;

// =============================================================================
// CLI Arguments
// =============================================================================

/// Visualize table structure from PDF documents
#[derive(Parser, Debug)]
#[command(name = "dlviz-table")]
#[command(version, about, long_about = None)]
struct Args {
    /// Input PDF file
    pdf_file: PathBuf,

    /// Page number (0-indexed)
    #[arg(short, long, default_value = "0")]
    page: usize,

    /// Table index on the page (0-indexed)
    #[arg(short, long)]
    table: Option<usize>,

    /// Process all tables on the page
    #[arg(long)]
    all: bool,

    /// List tables and exit
    #[arg(long)]
    list: bool,

    /// Output PNG file (for single table mode)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Output directory (for --all mode)
    #[arg(long)]
    output_dir: Option<PathBuf>,

    /// Render scale (2.0 = 144 DPI)
    #[arg(short, long, default_value = "2.0")]
    scale: f32,

    /// Padding around the table (pixels)
    #[arg(long, default_value = "20")]
    padding: u32,

    /// Show cell text content
    #[arg(long)]
    show_text: bool,

    /// Show row/column indices
    #[arg(long)]
    show_indices: bool,

    /// Show cell confidence values
    #[arg(long)]
    show_confidence: bool,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

// =============================================================================
// Colors
// =============================================================================

const COLOR_CELL_BORDER: Rgba<u8> = Rgba([0, 100, 200, 255]); // Blue for cell borders
const COLOR_TABLE_BORDER: Rgba<u8> = Rgba([200, 0, 0, 255]); // Red for table border
const COLOR_HEADER_COL: Rgba<u8> = Rgba([255, 200, 0, 200]); // Yellow for column header bg
const COLOR_HEADER_ROW: Rgba<u8> = Rgba([0, 200, 255, 200]); // Cyan for row header bg
const COLOR_HEADER_BOTH: Rgba<u8> = Rgba([0, 255, 100, 200]); // Green for both headers
const COLOR_SPAN_BORDER: Rgba<u8> = Rgba([255, 100, 0, 255]); // Orange for spanning cells
const COLOR_TEXT: Rgba<u8> = Rgba([0, 0, 0, 255]); // Black text
const COLOR_INDEX_TEXT: Rgba<u8> = Rgba([128, 128, 128, 255]); // Gray for indices
const COLOR_GRID_LINE: Rgba<u8> = Rgba([180, 180, 180, 200]); // Light gray for grid lines

// =============================================================================
// Table Visualization
// =============================================================================

fn visualize_table(
    base_image: &DynamicImage,
    table: &TableElement,
    scale: f32,
    page_height_pts: f32,
    padding: u32,
    show_text: bool,
    show_indices: bool,
    show_confidence: bool,
) -> RgbaImage {
    // Get table bbox and convert to image coordinates
    // BoundingBox uses l, t, r, b (left, top, right, bottom)
    // For TopLeft origin: t < b (t is at top, b is at bottom)
    // For BottomLeft origin: t > b (t is at top but y increases upward)
    let table_bbox = &table.cluster.bbox;
    let table_x = (table_bbox.l * scale) as i32;

    // Determine height and y position based on coordinate system
    let (table_y, table_h) = if table_bbox.t < table_bbox.b {
        // TopLeft origin (t < b): y increases downward
        let y = (table_bbox.t * scale) as i32;
        let h = ((table_bbox.b - table_bbox.t) * scale) as u32;
        (y, h)
    } else {
        // BottomLeft origin (t > b): y increases upward, convert to image coordinates
        let y = ((page_height_pts - table_bbox.t) * scale) as i32;
        let h = ((table_bbox.t - table_bbox.b) * scale) as u32;
        (y, h)
    };
    let table_w = ((table_bbox.r - table_bbox.l) * scale) as u32;

    // Calculate crop region with padding
    let (img_w, img_h) = base_image.dimensions();
    let crop_x = (table_x - padding as i32).max(0) as u32;
    let crop_y = (table_y - padding as i32).max(0) as u32;
    let crop_w = (table_w + 2 * padding).min(img_w - crop_x);
    let crop_h = (table_h + 2 * padding).min(img_h - crop_y);

    // Crop the image
    let cropped = base_image.crop_imm(crop_x, crop_y, crop_w, crop_h);
    let mut output = cropped.to_rgba8();

    // Offset for drawing within cropped region
    let offset_x = (table_x - crop_x as i32) as i32;
    let offset_y = (table_y - crop_y as i32) as i32;

    // Load font
    let font_data: &[u8] = include_bytes!("../../assets/DejaVuSansMono.ttf");
    let font = FontRef::try_from_slice(font_data).ok();

    // Draw table border (thick)
    for t in 0..3u32 {
        if table_w > 2 * t && table_h > 2 * t {
            let inner = Rect::at(offset_x + t as i32, offset_y + t as i32)
                .of_size(table_w - 2 * t, table_h - 2 * t);
            draw_hollow_rect_mut(&mut output, inner, COLOR_TABLE_BORDER);
        }
    }

    // Draw grid lines first (behind cells)
    let (row_boundaries, col_boundaries) = calculate_grid_boundaries(
        &table.table_cells,
        scale,
        page_height_pts,
        offset_x as f32,
        offset_y as f32,
        table_x,
        table_y,
    );
    draw_grid_lines(
        &mut output,
        &row_boundaries,
        &col_boundaries,
        table_w,
        table_h,
        offset_x,
        offset_y,
    );

    // Draw cells (on top of grid lines)
    for cell in &table.table_cells {
        draw_cell(
            &mut output,
            cell,
            scale,
            page_height_pts,
            offset_x as f32 - table_x as f32,
            offset_y as f32 - table_y as f32,
            show_text,
            show_indices,
            show_confidence,
            font.as_ref(),
        );
    }

    output
}

fn draw_cell(
    img: &mut RgbaImage,
    cell: &TableCell,
    scale: f32,
    page_height_pts: f32,
    offset_x: f32,
    offset_y: f32,
    show_text: bool,
    show_indices: bool,
    show_confidence: bool,
    font: Option<&FontRef>,
) {
    // Convert cell bbox to image coordinates
    // Handle both TopLeft and BottomLeft coordinate origins
    let cell_x = ((cell.bbox.l * scale) + offset_x) as i32;
    let (cell_y, cell_h) = if cell.bbox.t < cell.bbox.b {
        // TopLeft origin (t < b)
        let y = ((cell.bbox.t * scale) + offset_y) as i32;
        let h = ((cell.bbox.b - cell.bbox.t) * scale) as u32;
        (y, h)
    } else {
        // BottomLeft origin (t > b)
        let y = (((page_height_pts - cell.bbox.t) * scale) + offset_y) as i32;
        let h = ((cell.bbox.t - cell.bbox.b) * scale) as u32;
        (y, h)
    };
    let cell_w = ((cell.bbox.r - cell.bbox.l) * scale) as u32;

    // Skip cells with invalid dimensions
    if cell_w == 0 || cell_h == 0 || cell_x < 0 || cell_y < 0 {
        return;
    }

    let (img_w, img_h) = img.dimensions();
    if cell_x as u32 >= img_w || cell_y as u32 >= img_h {
        return;
    }

    // Draw header background if applicable
    let bg_color = if cell.column_header && cell.row_header {
        Some(COLOR_HEADER_BOTH)
    } else if cell.column_header {
        Some(COLOR_HEADER_COL)
    } else if cell.row_header {
        Some(COLOR_HEADER_ROW)
    } else {
        None
    };

    if let Some(color) = bg_color {
        fill_rect_alpha(img, cell_x as u32, cell_y as u32, cell_w, cell_h, color);
    }

    // Draw cell border (use orange for spanning cells)
    let border_color = if cell.row_span > 1 || cell.col_span > 1 {
        COLOR_SPAN_BORDER
    } else {
        COLOR_CELL_BORDER
    };

    // Draw border with appropriate thickness
    let thickness = if cell.row_span > 1 || cell.col_span > 1 {
        2
    } else {
        1
    };
    for t in 0..thickness {
        if cell_w > 2 * t && cell_h > 2 * t {
            let rect = Rect::at(cell_x + t as i32, cell_y + t as i32)
                .of_size(cell_w - 2 * t, cell_h - 2 * t);
            draw_hollow_rect_mut(img, rect, border_color);
        }
    }

    // Draw text content
    if let Some(ref font) = font {
        let font_size = (10.0 * scale).max(8.0).min(14.0);
        let px_scale = PxScale::from(font_size);
        let text_x = cell_x + 2;
        let mut text_y = cell_y + 2;

        // Draw cell indices if requested
        if show_indices {
            let idx_text = format!(
                "R{}C{} ({}x{})",
                cell.start_row_offset_idx, cell.start_col_offset_idx, cell.row_span, cell.col_span
            );
            draw_text_mut(
                img,
                COLOR_INDEX_TEXT,
                text_x,
                text_y,
                px_scale,
                font,
                &idx_text,
            );
            text_y += font_size as i32 + 2;
        }

        // Draw confidence if requested
        if show_confidence {
            if let Some(conf) = cell.confidence {
                let conf_text = format!("{:.0}%", conf * 100.0);
                draw_text_mut(
                    img,
                    COLOR_INDEX_TEXT,
                    text_x,
                    text_y,
                    px_scale,
                    font,
                    &conf_text,
                );
                text_y += font_size as i32 + 2;
            }
        }

        // Draw cell text if requested
        if show_text && !cell.text.is_empty() {
            // Truncate text if too long
            let max_chars = (cell_w as f32 / (font_size * 0.6)) as usize;
            let display_text: String = if cell.text.len() > max_chars && max_chars > 3 {
                format!(
                    "{}...",
                    &cell.text.chars().take(max_chars - 3).collect::<String>()
                )
            } else {
                cell.text.clone()
            };
            draw_text_mut(
                img,
                COLOR_TEXT,
                text_x,
                text_y,
                px_scale,
                font,
                &display_text,
            );
        }
    }
}

fn fill_rect_alpha(img: &mut RgbaImage, x: u32, y: u32, w: u32, h: u32, color: Rgba<u8>) {
    let (img_w, img_h) = img.dimensions();
    let alpha = color.0[3] as f32 / 255.0;

    for py in y..(y + h).min(img_h) {
        for px in x..(x + w).min(img_w) {
            let pixel = img.get_pixel_mut(px, py);
            pixel[0] = ((1.0 - alpha) * pixel[0] as f32 + alpha * color.0[0] as f32) as u8;
            pixel[1] = ((1.0 - alpha) * pixel[1] as f32 + alpha * color.0[1] as f32) as u8;
            pixel[2] = ((1.0 - alpha) * pixel[2] as f32 + alpha * color.0[2] as f32) as u8;
        }
    }
}

/// Draw horizontal line with alpha blending
fn draw_horizontal_line(img: &mut RgbaImage, x1: i32, x2: i32, y: i32, color: Rgba<u8>) {
    let (img_w, img_h) = img.dimensions();
    if y < 0 || y >= img_h as i32 {
        return;
    }
    let y = y as u32;
    let start_x = x1.max(0) as u32;
    let end_x = (x2.min(img_w as i32 - 1).max(0) as u32).max(start_x);

    let alpha = color.0[3] as f32 / 255.0;
    for px in start_x..=end_x {
        if px < img_w {
            let pixel = img.get_pixel_mut(px, y);
            pixel[0] = ((1.0 - alpha) * pixel[0] as f32 + alpha * color.0[0] as f32) as u8;
            pixel[1] = ((1.0 - alpha) * pixel[1] as f32 + alpha * color.0[1] as f32) as u8;
            pixel[2] = ((1.0 - alpha) * pixel[2] as f32 + alpha * color.0[2] as f32) as u8;
        }
    }
}

/// Draw vertical line with alpha blending
fn draw_vertical_line(img: &mut RgbaImage, x: i32, y1: i32, y2: i32, color: Rgba<u8>) {
    let (img_w, img_h) = img.dimensions();
    if x < 0 || x >= img_w as i32 {
        return;
    }
    let x = x as u32;
    let start_y = y1.max(0) as u32;
    let end_y = (y2.min(img_h as i32 - 1).max(0) as u32).max(start_y);

    let alpha = color.0[3] as f32 / 255.0;
    for py in start_y..=end_y {
        if py < img_h {
            let pixel = img.get_pixel_mut(x, py);
            pixel[0] = ((1.0 - alpha) * pixel[0] as f32 + alpha * color.0[0] as f32) as u8;
            pixel[1] = ((1.0 - alpha) * pixel[1] as f32 + alpha * color.0[1] as f32) as u8;
            pixel[2] = ((1.0 - alpha) * pixel[2] as f32 + alpha * color.0[2] as f32) as u8;
        }
    }
}

/// Calculate grid boundaries from cell positions
/// Returns (row_boundaries, col_boundaries) as sorted vectors of y and x coordinates in image space
fn calculate_grid_boundaries(
    cells: &[TableCell],
    scale: f32,
    page_height_pts: f32,
    offset_x: f32,
    offset_y: f32,
    table_x: i32,
    table_y: i32,
) -> (Vec<i32>, Vec<i32>) {
    use std::collections::BTreeSet;

    let mut row_positions: BTreeSet<i32> = BTreeSet::new();
    let mut col_positions: BTreeSet<i32> = BTreeSet::new();

    for cell in cells {
        // Convert cell bbox coordinates to image space
        let cell_left = ((cell.bbox.l * scale) + offset_x - table_x as f32) as i32;
        let cell_right = ((cell.bbox.r * scale) + offset_x - table_x as f32) as i32;

        // Handle both TopLeft and BottomLeft coordinate origins
        let (cell_top, cell_bottom) = if cell.bbox.t < cell.bbox.b {
            // TopLeft origin (t < b)
            let top = ((cell.bbox.t * scale) + offset_y - table_y as f32) as i32;
            let bottom = ((cell.bbox.b * scale) + offset_y - table_y as f32) as i32;
            (top, bottom)
        } else {
            // BottomLeft origin (t > b)
            let top =
                (((page_height_pts - cell.bbox.t) * scale) + offset_y - table_y as f32) as i32;
            let bottom =
                (((page_height_pts - cell.bbox.b) * scale) + offset_y - table_y as f32) as i32;
            (top, bottom)
        };

        // Add boundaries (rounded to avoid sub-pixel differences)
        col_positions.insert(cell_left);
        col_positions.insert(cell_right);
        row_positions.insert(cell_top);
        row_positions.insert(cell_bottom);
    }

    (
        row_positions.into_iter().collect(),
        col_positions.into_iter().collect(),
    )
}

/// Draw grid lines on the table visualization
fn draw_grid_lines(
    img: &mut RgbaImage,
    row_boundaries: &[i32],
    col_boundaries: &[i32],
    table_width: u32,
    table_height: u32,
    offset_x: i32,
    offset_y: i32,
) {
    // Draw horizontal grid lines
    for &y in row_boundaries {
        let y_in_img = y + offset_y;
        draw_horizontal_line(
            img,
            offset_x,
            offset_x + table_width as i32,
            y_in_img,
            COLOR_GRID_LINE,
        );
    }

    // Draw vertical grid lines
    for &x in col_boundaries {
        let x_in_img = x + offset_x;
        draw_vertical_line(
            img,
            x_in_img,
            offset_y,
            offset_y + table_height as i32,
            COLOR_GRID_LINE,
        );
    }
}

// =============================================================================
// PDF Processing
// =============================================================================

fn process_pdf(
    pdf_path: &PathBuf,
    page_no: usize,
    scale: f32,
) -> Result<(DynamicImage, Vec<TableElement>, f32, f32), String> {
    // Initialize pdfium for rendering
    let pdfium = Pdfium::default();
    let document = pdfium
        .load_pdf_from_file(pdf_path, None)
        .map_err(|e| format!("Failed to load PDF: {e}"))?;

    let pages = document.pages();
    if page_no >= pages.len() as usize {
        let total = pages.len();
        return Err(format!("Page {page_no} out of bounds (total: {total})"));
    }

    let page = pages
        .get(page_no as u16)
        .map_err(|e| format!("Failed to get page: {e}"))?;
    let page_width_pts = page.width().value;
    let page_height_pts = page.height().value;

    // Render page to image
    let render_config = PdfRenderConfig::new()
        .set_target_width((page_width_pts * scale) as i32)
        .set_maximum_height((page_height_pts * scale) as i32);

    let bitmap = page
        .render_with_config(&render_config)
        .map_err(|e| format!("Failed to render page: {e}"))?;

    let image = bitmap
        .as_image()
        .as_rgba8()
        .ok_or("Failed to convert to RGBA8")?
        .clone();

    let base_image = DynamicImage::ImageRgba8(image);

    // Initialize ML pipeline with table structure enabled
    let config = PipelineConfigBuilder::fast()
        .ocr_enabled(false)
        .table_structure_enabled(true)
        .build()
        .map_err(|e| format!("Pipeline config error: {e}"))?;

    let mut pipeline = Pipeline::new(config).map_err(|e| format!("Pipeline init error: {e}"))?;

    // Convert image to RGB array for pipeline
    let (width, height) = base_image.dimensions();
    let rgba_bytes = base_image.to_rgba8();
    let mut rgb_data = Vec::with_capacity((width * height * 3) as usize);
    for pixel in rgba_bytes.pixels() {
        rgb_data.push(pixel.0[0]);
        rgb_data.push(pixel.0[1]);
        rgb_data.push(pixel.0[2]);
    }

    let page_image =
        ndarray::Array3::from_shape_vec((height as usize, width as usize, 3), rgb_data)
            .map_err(|e| format!("Array error: {e}"))?;

    // Process page
    let result = pipeline
        .process_page(
            page_no,
            &page_image,
            page_width_pts,
            page_height_pts,
            None::<Vec<docling_pdf_ml::SimpleTextCell>>,
        )
        .map_err(|e| format!("Pipeline error: {e}"))?;

    // Extract tables from assembled output
    let tables: Vec<TableElement> = result
        .assembled
        .map(|a| {
            a.elements
                .into_iter()
                .filter_map(|e| {
                    if let PageElement::Table(t) = e {
                        Some(t)
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    Ok((base_image, tables, page_width_pts, page_height_pts))
}

// =============================================================================
// Main
// =============================================================================

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Initialize logging
    if args.verbose {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }

    // Validate input file
    if !args.pdf_file.exists() {
        eprintln!("Error: PDF file not found: {}", args.pdf_file.display());
        std::process::exit(1);
    }

    // Process PDF
    println!(
        "Processing: {} (page {})",
        args.pdf_file.display(),
        args.page
    );
    let (base_image, tables, _page_width_pts, page_height_pts) =
        process_pdf(&args.pdf_file, args.page, args.scale)?;

    println!("Found {} tables on page {}", tables.len(), args.page);

    if tables.is_empty() {
        println!("No tables detected on this page.");
        return Ok(());
    }

    // List mode
    if args.list {
        println!("\nTables on page {}:", args.page);
        println!("{:-<70}", "");
        println!(
            "{:>4}  {:>4}  {:>4}  {:>8}  {:>8}  {:>8}  {:>8}  {:>5}",
            "Idx", "Rows", "Cols", "X", "Y", "Width", "Height", "Cells"
        );
        println!("{:-<70}", "");

        for (i, table) in tables.iter().enumerate() {
            let bbox = &table.cluster.bbox;
            println!(
                "{:>4}  {:>4}  {:>4}  {:>8.1}  {:>8.1}  {:>8.1}  {:>8.1}  {:>5}",
                i,
                table.num_rows,
                table.num_cols,
                bbox.l,
                bbox.t,
                bbox.r - bbox.l,
                bbox.t - bbox.b,
                table.table_cells.len()
            );
        }

        println!("{:-<70}", "");
        return Ok(());
    }

    // Determine which tables to visualize
    let tables_to_viz: Vec<(usize, &TableElement)> = if args.all {
        tables.iter().enumerate().collect()
    } else if let Some(table_idx) = args.table {
        if table_idx >= tables.len() {
            eprintln!(
                "Error: Table index {} out of bounds (found {} tables)",
                table_idx,
                tables.len()
            );
            std::process::exit(1);
        }
        vec![(table_idx, &tables[table_idx])]
    } else {
        // Default to first table
        vec![(0, &tables[0])]
    };

    // Visualize tables
    if tables_to_viz.len() == 1 && !args.all {
        // Single table mode
        let (idx, table) = tables_to_viz[0];
        let output_path = args.output.clone().unwrap_or_else(|| {
            let stem = args
                .pdf_file
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("table");
            PathBuf::from(format!("{}_page{}_table{}.png", stem, args.page, idx))
        });

        let viz = visualize_table(
            &base_image,
            table,
            args.scale,
            page_height_pts,
            args.padding,
            args.show_text,
            args.show_indices,
            args.show_confidence,
        );

        viz.save(&output_path)?;
        println!(
            "Saved: {} ({}x{})",
            output_path.display(),
            viz.width(),
            viz.height()
        );

        if args.verbose {
            println!(
                "  Table: {}x{} ({} cells)",
                table.num_rows,
                table.num_cols,
                table.table_cells.len()
            );

            // Count headers
            let col_headers = table.table_cells.iter().filter(|c| c.column_header).count();
            let row_headers = table.table_cells.iter().filter(|c| c.row_header).count();
            let spanning = table
                .table_cells
                .iter()
                .filter(|c| c.row_span > 1 || c.col_span > 1)
                .count();

            println!("  Column headers: {}", col_headers);
            println!("  Row headers: {}", row_headers);
            println!("  Spanning cells: {}", spanning);
        }
    } else {
        // Multiple tables - use output directory
        let output_dir = args.output_dir.clone().unwrap_or_else(|| {
            let stem = args
                .pdf_file
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("tables");
            PathBuf::from(format!("{}_page{}_tables", stem, args.page))
        });

        std::fs::create_dir_all(&output_dir)?;
        println!(
            "Saving {} tables to {}/",
            tables_to_viz.len(),
            output_dir.display()
        );

        let num_tables = tables_to_viz.len();
        for (idx, table) in tables_to_viz {
            let output_path = output_dir.join(format!(
                "table_{:02}_{}x{}.png",
                idx, table.num_rows, table.num_cols
            ));

            let viz = visualize_table(
                &base_image,
                table,
                args.scale,
                page_height_pts,
                args.padding,
                args.show_text,
                args.show_indices,
                args.show_confidence,
            );

            viz.save(&output_path)?;

            if args.verbose {
                println!(
                    "  Saved: {} ({}x{}, {} cells)",
                    output_path.display(),
                    viz.width(),
                    viz.height(),
                    table.table_cells.len()
                );
            }
        }

        println!("Done: {} tables visualized", num_tables);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fill_rect_alpha() {
        let mut img = RgbaImage::from_pixel(100, 100, Rgba([255, 255, 255, 255]));
        fill_rect_alpha(&mut img, 10, 10, 50, 50, Rgba([0, 0, 255, 128]));

        // Check center pixel is blended
        let pixel = img.get_pixel(35, 35);
        assert!(pixel.0[2] > pixel.0[0]); // Should have more blue than red
    }

    #[test]
    fn test_colors_distinct() {
        assert_ne!(COLOR_CELL_BORDER, COLOR_TABLE_BORDER);
        assert_ne!(COLOR_HEADER_COL, COLOR_HEADER_ROW);
        assert_ne!(COLOR_SPAN_BORDER, COLOR_CELL_BORDER);
    }
}
