//! CLI tool for comparing PDF detections against groundtruth
//!
//! Compares current ML detections against expected (groundtruth) output,
//! highlighting differences such as missed elements, extra detections, and
//! label mismatches.
//!
//! # Usage
//!
//! ```bash
//! # Compare single page against groundtruth JSON
//! dlviz-diff paper.pdf --expected groundtruth.json --output diff.png
//!
//! # Compare all pages against groundtruth directory
//! dlviz-diff paper.pdf --expected-dir ./groundtruth/ --output-dir ./diffs/
//!
//! # Output statistics only
//! dlviz-diff paper.pdf --expected groundtruth.json --stats-only
//! ```

use clap::Parser;
use docling_viz_bridge::{
    dlviz_get_page_count, dlviz_get_page_size, dlviz_get_stage_snapshot, dlviz_load_pdf,
    dlviz_pipeline_free, dlviz_pipeline_new, dlviz_run_to_stage, DlvizElement, DlvizResult,
    DlvizStage, DlvizStageSnapshot,
};
use image::{Rgb, RgbImage};
use imageproc::drawing::draw_hollow_rect_mut;
use imageproc::rect::Rect;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::CString;
use std::fs;
use std::path::PathBuf;
use std::ptr;

/// Compare detections against groundtruth
#[derive(Parser, Debug)]
#[command(name = "dlviz-diff")]
#[command(version, about, long_about = None)]
struct Args {
    /// Input PDF file
    #[arg(required = true)]
    pdf: PathBuf,

    /// Page number (0-indexed). Use --all for all pages.
    #[arg(short, long, default_value = "0")]
    page: usize,

    /// Process all pages
    #[arg(long)]
    all: bool,

    /// Expected groundtruth JSON file (for single page)
    #[arg(short, long)]
    expected: Option<PathBuf>,

    /// Expected groundtruth directory (for all pages)
    /// Files should be named: {stem}_page_{N:03}.json
    #[arg(long)]
    expected_dir: Option<PathBuf>,

    /// Output PNG file (for single page)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Output directory (for --all mode)
    #[arg(long)]
    output_dir: Option<PathBuf>,

    /// Only output statistics, no visualization
    #[arg(long)]
    stats_only: bool,

    /// IoU threshold for matching elements (default 0.5)
    #[arg(long, default_value = "0.5")]
    iou_threshold: f32,

    /// Render scale for visualization
    #[arg(long, default_value = "2.0")]
    scale: f32,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

/// Element from groundtruth JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GroundtruthElement {
    id: u32,
    label: String,
    #[serde(default)]
    confidence: f32,
    bbox: BBox,
    #[serde(default)]
    reading_order: i32,
}

/// Bounding box from JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BBox {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

/// Groundtruth page data
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GroundtruthPage {
    #[serde(default)]
    document: String,
    page: usize,
    #[serde(default)]
    page_size: Option<PageSize>,
    elements: Vec<GroundtruthElement>,
}

/// Page size
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PageSize {
    width: f32,
    height: f32,
}

/// Diff result for a single element
#[derive(Debug, Clone, Serialize)]
enum DiffType {
    /// Element matches (within IoU threshold)
    Match { iou: f32 },
    /// Labels don't match
    LabelMismatch { expected: String, actual: String },
    /// Element in groundtruth but not detected
    Missing,
    /// Element detected but not in groundtruth
    Extra,
}

/// Diff result for a pair of elements
#[derive(Debug, Clone, Serialize)]
struct ElementDiff {
    groundtruth_id: Option<u32>,
    detected_id: Option<u32>,
    diff_type: DiffType,
    groundtruth_bbox: Option<BBox>,
    detected_bbox: Option<BBox>,
    label: String,
}

/// Diff statistics
#[derive(Debug, Clone, Serialize)]
struct DiffStats {
    total_groundtruth: usize,
    total_detected: usize,
    matches: usize,
    label_mismatches: usize,
    missing: usize,
    extra: usize,
    precision: f32,
    recall: f32,
    f1: f32,
    /// Per-label statistics
    label_stats: HashMap<String, LabelStats>,
}

/// Per-label statistics
#[derive(Debug, Clone, Default, Serialize)]
struct LabelStats {
    groundtruth: usize,
    detected: usize,
    matches: usize,
    precision: f32,
    recall: f32,
    f1: f32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Initialize logging
    if args.verbose {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }

    // Validate arguments
    if args.expected.is_none() && args.expected_dir.is_none() {
        eprintln!("Error: Either --expected or --expected-dir must be provided");
        std::process::exit(1);
    }

    if !args.stats_only && args.output.is_none() && args.output_dir.is_none() {
        eprintln!("Error: Either --output, --output-dir, or --stats-only must be provided");
        std::process::exit(1);
    }

    // Load PDF
    let pipeline = dlviz_pipeline_new();
    if pipeline.is_null() {
        return Err("Failed to create pipeline".into());
    }

    let pdf_cstr = CString::new(args.pdf.to_string_lossy().as_bytes())?;
    let result = unsafe { dlviz_load_pdf(pipeline, pdf_cstr.as_ptr()) };

    if result != DlvizResult::Success {
        unsafe { dlviz_pipeline_free(pipeline) };
        return Err(format!("Failed to load PDF: {:?}", result).into());
    }

    let page_count = unsafe { dlviz_get_page_count(pipeline) };
    println!("Loaded PDF with {} pages", page_count);

    let pdf_stem = args
        .pdf
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("document");

    // Create output directory if needed
    if let Some(ref dir) = args.output_dir {
        fs::create_dir_all(dir)?;
    }

    let mut all_diffs: Vec<(usize, Vec<ElementDiff>)> = Vec::new();
    let mut total_stats = DiffStats {
        total_groundtruth: 0,
        total_detected: 0,
        matches: 0,
        label_mismatches: 0,
        missing: 0,
        extra: 0,
        precision: 0.0,
        recall: 0.0,
        f1: 0.0,
        label_stats: HashMap::new(),
    };

    // Process pages
    let pages: Vec<usize> = if args.all {
        (0..page_count).collect()
    } else {
        vec![args.page]
    };

    for page_num in pages {
        if page_num >= page_count {
            eprintln!("Warning: Page {} out of bounds", page_num);
            continue;
        }

        // Load groundtruth for this page
        let groundtruth_path = if let Some(ref expected) = args.expected {
            expected.clone()
        } else if let Some(ref expected_dir) = args.expected_dir {
            expected_dir.join(format!("{}_page_{:03}.json", pdf_stem, page_num))
        } else {
            continue;
        };

        if !groundtruth_path.exists() {
            if args.verbose {
                eprintln!(
                    "Warning: Groundtruth not found for page {}: {}",
                    page_num,
                    groundtruth_path.display()
                );
            }
            continue;
        }

        let groundtruth: GroundtruthPage = match fs::read_to_string(&groundtruth_path) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(gt) => gt,
                Err(e) => {
                    eprintln!(
                        "Error parsing groundtruth {}: {}",
                        groundtruth_path.display(),
                        e
                    );
                    continue;
                }
            },
            Err(e) => {
                eprintln!(
                    "Error reading groundtruth {}: {}",
                    groundtruth_path.display(),
                    e
                );
                continue;
            }
        };

        // Run ML pipeline for this page
        let run_result =
            unsafe { dlviz_run_to_stage(pipeline, page_num, DlvizStage::ReadingOrder) };

        if run_result != DlvizResult::Success {
            eprintln!("Warning: ML pipeline failed for page {}", page_num);
            continue;
        }

        // Get detected elements
        let mut snapshot = DlvizStageSnapshot {
            stage: DlvizStage::ReadingOrder,
            element_count: 0,
            elements: ptr::null(),
            cell_count: 0,
            cells: ptr::null(),
            processing_time_ms: 0.0,
        };

        let result = unsafe {
            dlviz_get_stage_snapshot(pipeline, page_num, DlvizStage::ReadingOrder, &mut snapshot)
        };

        if result != DlvizResult::Success {
            continue;
        }

        let detected: Vec<DlvizElement> =
            if snapshot.element_count > 0 && !snapshot.elements.is_null() {
                unsafe {
                    std::slice::from_raw_parts(snapshot.elements, snapshot.element_count).to_vec()
                }
            } else {
                Vec::new()
            };

        // Compare detections against groundtruth
        let diffs = compare_elements(&groundtruth.elements, &detected, args.iou_threshold);
        let stats = calculate_stats(&diffs);

        // Accumulate total stats
        total_stats.total_groundtruth += stats.total_groundtruth;
        total_stats.total_detected += stats.total_detected;
        total_stats.matches += stats.matches;
        total_stats.label_mismatches += stats.label_mismatches;
        total_stats.missing += stats.missing;
        total_stats.extra += stats.extra;

        // Merge label stats
        for (label, ls) in &stats.label_stats {
            let entry = total_stats.label_stats.entry(label.clone()).or_default();
            entry.groundtruth += ls.groundtruth;
            entry.detected += ls.detected;
            entry.matches += ls.matches;
        }

        if args.verbose {
            println!(
                "Page {}: {} groundtruth, {} detected, {} matches, {} missing, {} extra",
                page_num,
                stats.total_groundtruth,
                stats.total_detected,
                stats.matches,
                stats.missing,
                stats.extra
            );
        }

        // Generate visualization if not stats-only
        if !args.stats_only {
            let output_path = if let Some(ref output) = args.output {
                output.clone()
            } else if let Some(ref output_dir) = args.output_dir {
                output_dir.join(format!("{}_page_{:03}_diff.png", pdf_stem, page_num))
            } else {
                continue;
            };

            // Get page size for rendering
            let mut width: f32 = 0.0;
            let mut height: f32 = 0.0;
            unsafe {
                dlviz_get_page_size(pipeline, page_num, &mut width, &mut height);
            }

            render_diff_visualization(
                &groundtruth.elements,
                &detected,
                &diffs,
                width,
                height,
                args.scale,
                &output_path,
            )?;

            println!("Saved diff visualization: {}", output_path.display());
        }

        all_diffs.push((page_num, diffs));
    }

    // Calculate final precision/recall/F1
    if total_stats.total_detected > 0 {
        total_stats.precision = total_stats.matches as f32 / total_stats.total_detected as f32;
    }
    if total_stats.total_groundtruth > 0 {
        total_stats.recall = total_stats.matches as f32 / total_stats.total_groundtruth as f32;
    }
    if total_stats.precision + total_stats.recall > 0.0 {
        total_stats.f1 = 2.0 * total_stats.precision * total_stats.recall
            / (total_stats.precision + total_stats.recall);
    }

    // Calculate per-label metrics
    for ls in total_stats.label_stats.values_mut() {
        if ls.detected > 0 {
            ls.precision = ls.matches as f32 / ls.detected as f32;
        }
        if ls.groundtruth > 0 {
            ls.recall = ls.matches as f32 / ls.groundtruth as f32;
        }
        if ls.precision + ls.recall > 0.0 {
            ls.f1 = 2.0 * ls.precision * ls.recall / (ls.precision + ls.recall);
        }
    }

    // Output statistics
    println!("\n========== Diff Statistics ==========\n");
    println!("Groundtruth elements: {}", total_stats.total_groundtruth);
    println!("Detected elements:    {}", total_stats.total_detected);
    println!("Matches:              {}", total_stats.matches);
    println!("Label mismatches:     {}", total_stats.label_mismatches);
    println!("Missing (FN):         {}", total_stats.missing);
    println!("Extra (FP):           {}", total_stats.extra);
    println!();
    println!("Precision:            {:.3}", total_stats.precision);
    println!("Recall:               {:.3}", total_stats.recall);
    println!("F1 Score:             {:.3}", total_stats.f1);
    println!();

    if !total_stats.label_stats.is_empty() {
        println!("Per-label statistics:");
        let mut labels: Vec<_> = total_stats.label_stats.iter().collect();
        labels.sort_by(|a, b| b.1.groundtruth.cmp(&a.1.groundtruth));
        for (label, ls) in labels {
            println!(
                "  {}: P={:.3} R={:.3} F1={:.3} (gt={}, det={}, match={})",
                label, ls.precision, ls.recall, ls.f1, ls.groundtruth, ls.detected, ls.matches
            );
        }
    }

    println!("\n======================================");

    // Save stats to JSON if output dir specified
    if let Some(ref output_dir) = args.output_dir {
        let stats_path = output_dir.join("diff_stats.json");
        let stats_json = serde_json::to_string_pretty(&total_stats)?;
        fs::write(&stats_path, &stats_json)?;
        println!("\nSaved stats to: {}", stats_path.display());
    }

    // Cleanup
    unsafe { dlviz_pipeline_free(pipeline) };

    Ok(())
}

/// Compare detected elements against groundtruth
fn compare_elements(
    groundtruth: &[GroundtruthElement],
    detected: &[DlvizElement],
    iou_threshold: f32,
) -> Vec<ElementDiff> {
    let mut diffs: Vec<ElementDiff> = Vec::new();
    let mut matched_gt: Vec<bool> = vec![false; groundtruth.len()];
    let mut matched_det: Vec<bool> = vec![false; detected.len()];

    // Match each detected element to best groundtruth
    for (det_idx, det) in detected.iter().enumerate() {
        let det_bbox = BBox {
            x: det.bbox.x,
            y: det.bbox.y,
            width: det.bbox.width,
            height: det.bbox.height,
        };

        let mut best_match: Option<(usize, f32)> = None;

        for (gt_idx, gt) in groundtruth.iter().enumerate() {
            if matched_gt[gt_idx] {
                continue;
            }

            let iou = calculate_iou(&gt.bbox, &det_bbox);
            if iou >= iou_threshold {
                if best_match.is_none_or(|(_, s)| iou > s) {
                    best_match = Some((gt_idx, iou));
                }
            }
        }

        if let Some((gt_idx, iou)) = best_match {
            let gt = &groundtruth[gt_idx];
            let det_label = format!("{:?}", det.label);

            // Compare labels case-insensitively
            if gt.label.to_lowercase() == det_label.to_lowercase() {
                diffs.push(ElementDiff {
                    groundtruth_id: Some(gt.id),
                    detected_id: Some(det.id),
                    diff_type: DiffType::Match { iou },
                    groundtruth_bbox: Some(gt.bbox.clone()),
                    detected_bbox: Some(det_bbox),
                    label: gt.label.clone(),
                });
            } else {
                diffs.push(ElementDiff {
                    groundtruth_id: Some(gt.id),
                    detected_id: Some(det.id),
                    diff_type: DiffType::LabelMismatch {
                        expected: gt.label.clone(),
                        actual: det_label,
                    },
                    groundtruth_bbox: Some(gt.bbox.clone()),
                    detected_bbox: Some(det_bbox),
                    label: gt.label.clone(),
                });
            }

            matched_gt[gt_idx] = true;
            matched_det[det_idx] = true;
        }
    }

    // Mark unmatched groundtruth as missing
    for (gt_idx, gt) in groundtruth.iter().enumerate() {
        if !matched_gt[gt_idx] {
            diffs.push(ElementDiff {
                groundtruth_id: Some(gt.id),
                detected_id: None,
                diff_type: DiffType::Missing,
                groundtruth_bbox: Some(gt.bbox.clone()),
                detected_bbox: None,
                label: gt.label.clone(),
            });
        }
    }

    // Mark unmatched detections as extra
    for (det_idx, det) in detected.iter().enumerate() {
        if !matched_det[det_idx] {
            diffs.push(ElementDiff {
                groundtruth_id: None,
                detected_id: Some(det.id),
                diff_type: DiffType::Extra,
                groundtruth_bbox: None,
                detected_bbox: Some(BBox {
                    x: det.bbox.x,
                    y: det.bbox.y,
                    width: det.bbox.width,
                    height: det.bbox.height,
                }),
                label: format!("{:?}", det.label),
            });
        }
    }

    diffs
}

/// Calculate IoU between two bboxes
fn calculate_iou(a: &BBox, b: &BBox) -> f32 {
    let x1 = a.x.max(b.x);
    let y1 = a.y.max(b.y);
    let x2 = (a.x + a.width).min(b.x + b.width);
    let y2 = (a.y + a.height).min(b.y + b.height);

    if x2 <= x1 || y2 <= y1 {
        return 0.0;
    }

    let intersection = (x2 - x1) * (y2 - y1);
    let area_a = a.width * a.height;
    let area_b = b.width * b.height;
    let union = area_a + area_b - intersection;

    if union > 0.0 {
        intersection / union
    } else {
        0.0
    }
}

/// Calculate statistics from diffs
fn calculate_stats(diffs: &[ElementDiff]) -> DiffStats {
    let mut stats = DiffStats {
        total_groundtruth: 0,
        total_detected: 0,
        matches: 0,
        label_mismatches: 0,
        missing: 0,
        extra: 0,
        precision: 0.0,
        recall: 0.0,
        f1: 0.0,
        label_stats: HashMap::new(),
    };

    for diff in diffs {
        match &diff.diff_type {
            DiffType::Match { .. } => {
                stats.matches += 1;
                stats.total_groundtruth += 1;
                stats.total_detected += 1;

                let ls = stats.label_stats.entry(diff.label.clone()).or_default();
                ls.matches += 1;
                ls.groundtruth += 1;
                ls.detected += 1;
            }
            DiffType::LabelMismatch { .. } => {
                stats.label_mismatches += 1;
                stats.total_groundtruth += 1;
                stats.total_detected += 1;

                let ls = stats.label_stats.entry(diff.label.clone()).or_default();
                ls.groundtruth += 1;
            }
            DiffType::Missing => {
                stats.missing += 1;
                stats.total_groundtruth += 1;

                let ls = stats.label_stats.entry(diff.label.clone()).or_default();
                ls.groundtruth += 1;
            }
            DiffType::Extra => {
                stats.extra += 1;
                stats.total_detected += 1;

                let ls = stats.label_stats.entry(diff.label.clone()).or_default();
                ls.detected += 1;
            }
        }
    }

    // Calculate metrics
    if stats.total_detected > 0 {
        stats.precision = stats.matches as f32 / stats.total_detected as f32;
    }
    if stats.total_groundtruth > 0 {
        stats.recall = stats.matches as f32 / stats.total_groundtruth as f32;
    }
    if stats.precision + stats.recall > 0.0 {
        stats.f1 = 2.0 * stats.precision * stats.recall / (stats.precision + stats.recall);
    }

    stats
}

/// Render diff visualization
fn render_diff_visualization(
    _groundtruth: &[GroundtruthElement],
    _detected: &[DlvizElement],
    diffs: &[ElementDiff],
    page_width: f32,
    page_height: f32,
    scale: f32,
    output_path: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let img_width = (page_width * scale) as u32;
    let img_height = (page_height * scale) as u32;

    // Create white background
    let mut img = RgbImage::from_pixel(img_width, img_height, Rgb([255, 255, 255]));

    // Colors for different diff types
    let color_match = Rgb([0, 200, 0]); // Green - correct match
    let color_mismatch = Rgb([255, 165, 0]); // Orange - label mismatch
    let color_missing = Rgb([255, 0, 0]); // Red - missing (FN)
    let color_extra = Rgb([0, 0, 255]); // Blue - extra (FP)

    // Draw all diffs
    for diff in diffs {
        let color = match &diff.diff_type {
            DiffType::Match { .. } => color_match,
            DiffType::LabelMismatch { .. } => color_mismatch,
            DiffType::Missing => color_missing,
            DiffType::Extra => color_extra,
        };

        // Draw groundtruth bbox (if present) - dashed would be nice but solid for now
        if let Some(ref bbox) = diff.groundtruth_bbox {
            draw_bbox(&mut img, bbox, page_height, scale, color, 2);
        }

        // Draw detected bbox (if present) with different thickness
        if let Some(ref bbox) = diff.detected_bbox {
            // Slightly offset for visibility when overlapping
            draw_bbox(&mut img, bbox, page_height, scale, color, 3);
        }
    }

    // Save image
    img.save(output_path)?;

    Ok(())
}

/// Draw a bounding box on the image
fn draw_bbox(
    img: &mut RgbImage,
    bbox: &BBox,
    page_height: f32,
    scale: f32,
    color: Rgb<u8>,
    thickness: i32,
) {
    // Convert from PDF coordinates (origin bottom-left) to image coordinates (origin top-left)
    let x = (bbox.x * scale) as i32;
    let y = ((page_height - bbox.y - bbox.height) * scale) as i32;
    let w = (bbox.width * scale) as i32;
    let h = (bbox.height * scale) as i32;

    if w <= 0 || h <= 0 {
        return;
    }

    // Draw rectangle with thickness
    for t in 0..thickness {
        let rect = Rect::at(x - t, y - t).of_size((w + 2 * t) as u32, (h + 2 * t) as u32);
        draw_hollow_rect_mut(img, rect, color);
    }
}
