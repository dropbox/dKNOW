//! CLI tool to render DoclingViz visualizations for AI testing
//!
//! Renders PDF pages with bounding box overlays for AI visual quality assessment.
//!
//! # Usage
//!
//! ```bash
//! # Single page
//! dlviz-screenshot input.pdf --page 0 --stage reading_order --output viz.png
//!
//! # All pages
//! dlviz-screenshot input.pdf --all --stage reading_order --output-dir ./screenshots/
//!
//! # Multiple PDFs to directory
//! dlviz-screenshot *.pdf --all --output-dir ./screenshots/
//! ```

use clap::{Parser, ValueEnum};
use docling_viz_bridge::{
    dlviz_get_page_count, dlviz_load_pdf, dlviz_pipeline_free, dlviz_pipeline_new,
    dlviz_render_all_pages, dlviz_render_visualization, DlvizResult, DlvizStage,
};
use std::ffi::CString;
use std::path::PathBuf;

/// Pipeline stage for visualization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ValueEnum)]
enum Stage {
    /// Raw PDF render (no ML)
    RawPdf,
    /// OCR text detection
    OcrDetection,
    /// OCR text recognition
    OcrRecognition,
    /// Layout detection (YOLO)
    LayoutDetection,
    /// Cell-to-cluster assignment
    CellAssignment,
    /// Empty cluster removal
    EmptyClusterRemoval,
    /// Orphan cell detection
    OrphanDetection,
    /// BBox adjustment iteration 1
    BboxAdjust1,
    /// BBox adjustment iteration 2
    BboxAdjust2,
    /// Final element assembly
    FinalAssembly,
    /// Reading order assignment
    ReadingOrder,
}

impl Stage {
    fn to_dlviz_stage(self) -> DlvizStage {
        match self {
            Stage::RawPdf => DlvizStage::RawPdf,
            Stage::OcrDetection => DlvizStage::OcrDetection,
            Stage::OcrRecognition => DlvizStage::OcrRecognition,
            Stage::LayoutDetection => DlvizStage::LayoutDetection,
            Stage::CellAssignment => DlvizStage::CellAssignment,
            Stage::EmptyClusterRemoval => DlvizStage::EmptyClusterRemoval,
            Stage::OrphanDetection => DlvizStage::OrphanDetection,
            Stage::BboxAdjust1 => DlvizStage::BBoxAdjust1,
            Stage::BboxAdjust2 => DlvizStage::BBoxAdjust2,
            Stage::FinalAssembly => DlvizStage::FinalAssembly,
            Stage::ReadingOrder => DlvizStage::ReadingOrder,
        }
    }
}

/// Render DoclingViz visualizations for AI visual testing
#[derive(Parser, Debug)]
#[command(name = "dlviz-screenshot")]
#[command(version, about, long_about = None)]
struct Args {
    /// Input PDF file(s)
    #[arg(required = true)]
    pdfs: Vec<PathBuf>,

    /// Page number (0-indexed). Use --all for all pages.
    #[arg(short, long, default_value = "0")]
    page: usize,

    /// Process all pages
    #[arg(long)]
    all: bool,

    /// Pipeline stage to visualize
    #[arg(short, long, default_value = "reading-order")]
    stage: Stage,

    /// Output PNG file (for single page mode)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Output directory (for --all or multiple PDFs)
    #[arg(long)]
    output_dir: Option<PathBuf>,

    /// Render scale (2.0 = 144 DPI, recommended for AI vision)
    #[arg(long, default_value = "2.0")]
    scale: f32,

    /// Skip JSON sidecar generation
    #[arg(long)]
    no_json: bool,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Initialize logging
    if args.verbose {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }

    // Validate arguments
    if args.pdfs.len() > 1 && args.output.is_some() {
        eprintln!("Error: --output cannot be used with multiple PDFs. Use --output-dir instead.");
        std::process::exit(1);
    }

    if args.all && args.output.is_some() {
        eprintln!("Error: --output cannot be used with --all. Use --output-dir instead.");
        std::process::exit(1);
    }

    // Determine output directory
    let output_dir = args.output_dir.clone().unwrap_or_else(|| {
        args.output
            .as_ref()
            .and_then(|o| o.parent())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
    });

    // Create output directory
    std::fs::create_dir_all(&output_dir)?;

    let stage = args.stage.to_dlviz_stage();
    let mut total_pages = 0;
    let mut total_errors = 0;

    for pdf_path in &args.pdfs {
        if !pdf_path.exists() {
            eprintln!("Warning: PDF not found: {}", pdf_path.display());
            total_errors += 1;
            continue;
        }

        let pdf_path_str = pdf_path.to_string_lossy();
        println!("Processing: {}", pdf_path_str);

        // Create pipeline and load PDF
        let pipeline = dlviz_pipeline_new();
        if pipeline.is_null() {
            eprintln!("  Error: Failed to create pipeline");
            total_errors += 1;
            continue;
        }

        let pdf_cstr = CString::new(pdf_path_str.as_bytes())?;
        let result = unsafe { dlviz_load_pdf(pipeline, pdf_cstr.as_ptr()) };

        if result != DlvizResult::Success {
            eprintln!("  Error: Failed to load PDF: {:?}", result);
            unsafe { dlviz_pipeline_free(pipeline) };
            total_errors += 1;
            continue;
        }

        let page_count = unsafe { dlviz_get_page_count(pipeline) };
        println!("  Pages: {}", page_count);

        if args.all {
            // Process all pages
            let output_dir_cstr = CString::new(output_dir.to_string_lossy().as_bytes())?;
            let pages_processed = unsafe {
                dlviz_render_all_pages(pipeline, stage, args.scale, output_dir_cstr.as_ptr())
            };

            if pages_processed < 0 {
                eprintln!("  Error: Failed to render pages: {}", pages_processed);
                total_errors += 1;
            } else {
                println!(
                    "  Rendered {} pages to {}",
                    pages_processed,
                    output_dir.display()
                );
                total_pages += pages_processed as usize;
            }
        } else {
            // Process single page
            if args.page >= page_count {
                eprintln!(
                    "  Error: Page {} out of bounds (total: {})",
                    args.page, page_count
                );
                unsafe { dlviz_pipeline_free(pipeline) };
                total_errors += 1;
                continue;
            }

            let output_path = args.output.clone().unwrap_or_else(|| {
                let stem = pdf_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("output");
                output_dir.join(format!(
                    "{}_page_{:03}_{:?}.png",
                    stem, args.page, args.stage
                ))
            });

            let output_cstr = CString::new(output_path.to_string_lossy().as_bytes())?;
            let result = unsafe {
                dlviz_render_visualization(
                    pipeline,
                    args.page,
                    stage,
                    args.scale,
                    output_cstr.as_ptr(),
                )
            };

            if result != DlvizResult::Success {
                eprintln!("  Error: Failed to render page {}: {:?}", args.page, result);
                total_errors += 1;
            } else {
                println!("  Saved: {}", output_path.display());
                if !args.no_json {
                    let json_path = output_path.with_extension("json");
                    println!("  Saved: {}", json_path.display());
                }
                total_pages += 1;
            }
        }

        // Cleanup
        unsafe { dlviz_pipeline_free(pipeline) };
    }

    // Summary
    println!();
    println!("Summary:");
    println!("  Total pages rendered: {}", total_pages);
    if total_errors > 0 {
        println!("  Errors: {}", total_errors);
    }

    if total_errors > 0 {
        std::process::exit(1);
    }

    Ok(())
}
