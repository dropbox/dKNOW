//! CLI tool for batch review of PDF extractions
//!
//! Processes multiple PDFs, flags problematic pages, and generates summary statistics.
//!
//! # Usage
//!
//! ```bash
//! # Review all PDFs in a directory
//! dlviz-batch-review ./corpus/ --output ./flagged/
//!
//! # Flag low-confidence detections (threshold 0.8)
//! dlviz-batch-review ./corpus/ --min-confidence 0.8 --output ./flagged/
//!
//! # Statistics only (no page rendering)
//! dlviz-batch-review ./corpus/ --stats-only
//!
//! # Flag overlapping bboxes and empty text
//! dlviz-batch-review ./corpus/ --flag-overlapping --flag-empty-text
//! ```

use clap::Parser;
use docling_viz_bridge::{
    dlviz_get_page_count, dlviz_get_stage_snapshot, dlviz_load_pdf, dlviz_pipeline_free,
    dlviz_pipeline_new, dlviz_render_visualization, dlviz_run_to_stage, DlvizBBox, DlvizElement,
    DlvizResult, DlvizStage, DlvizStageSnapshot,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::CString;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::ptr;

/// Batch review tool for PDF extractions
#[derive(Parser, Debug)]
#[command(name = "dlviz-batch-review")]
#[command(version, about, long_about = None)]
struct Args {
    /// Input directory containing PDFs, or list of PDF files
    #[arg(required = true)]
    inputs: Vec<PathBuf>,

    /// Output directory for flagged pages and reports
    #[arg(short, long, default_value = "./flagged")]
    output: PathBuf,

    /// Minimum confidence threshold (elements below this are flagged)
    #[arg(long, default_value = "0.8")]
    min_confidence: f32,

    /// Flag pages with overlapping bounding boxes
    #[arg(long)]
    flag_overlapping: bool,

    /// Flag elements with empty text content
    #[arg(long)]
    flag_empty_text: bool,

    /// Overlap threshold (IoU) for flagging overlapping boxes
    #[arg(long, default_value = "0.5")]
    overlap_threshold: f32,

    /// Only output statistics, don't render flagged pages
    #[arg(long)]
    stats_only: bool,

    /// Output format for summary (json or text)
    #[arg(long, default_value = "json")]
    format: String,

    /// Render scale for flagged page images
    #[arg(long, default_value = "2.0")]
    scale: f32,

    /// Maximum number of pages to flag per document (0 = no limit)
    #[arg(long, default_value = "0")]
    max_flagged_pages: usize,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

/// Statistics for a single document
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DocumentStats {
    /// Document filename
    filename: String,
    /// Total pages
    total_pages: usize,
    /// Pages flagged for review
    flagged_pages: usize,
    /// Total elements across all pages
    total_elements: usize,
    /// Elements below confidence threshold
    low_confidence_elements: usize,
    /// Pages with overlapping boxes (if checked)
    overlap_pages: usize,
    /// Elements with empty text (if checked)
    empty_text_elements: usize,
    /// Average confidence score
    avg_confidence: f32,
    /// Minimum confidence score
    min_confidence: f32,
    /// Element count by label
    label_counts: HashMap<String, usize>,
    /// Processing time (ms)
    processing_time_ms: f64,
    /// List of flagged page numbers
    flagged_page_list: Vec<usize>,
}

/// Corpus-wide statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CorpusStats {
    /// Total documents processed
    total_documents: usize,
    /// Documents with flagged pages
    documents_with_flags: usize,
    /// Total pages across corpus
    total_pages: usize,
    /// Total flagged pages
    total_flagged_pages: usize,
    /// Total elements across corpus
    total_elements: usize,
    /// Total low-confidence elements
    total_low_confidence: usize,
    /// Average confidence across corpus
    avg_confidence: f32,
    /// Minimum confidence across corpus
    min_confidence: f32,
    /// Element count by label across corpus
    label_counts: HashMap<String, usize>,
    /// Total processing time (ms)
    total_processing_time_ms: f64,
    /// Per-document statistics
    documents: Vec<DocumentStats>,
    /// Configuration used
    config: ReviewConfig,
}

/// Configuration for the review
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReviewConfig {
    min_confidence_threshold: f32,
    flag_overlapping: bool,
    flag_empty_text: bool,
    overlap_threshold: f32,
}

/// Reason for flagging a page
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FlagReason {
    element_id: u32,
    reason: String,
    confidence: Option<f32>,
    label: String,
}

/// Flagged page info
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FlaggedPage {
    document: String,
    page: usize,
    reasons: Vec<FlagReason>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Initialize logging
    if args.verbose {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }

    // Create output directory
    fs::create_dir_all(&args.output)?;

    // Collect PDF files
    let pdf_files = collect_pdf_files(&args.inputs)?;

    if pdf_files.is_empty() {
        eprintln!("No PDF files found in input paths");
        std::process::exit(1);
    }

    println!("Found {} PDF files to process", pdf_files.len());

    let config = ReviewConfig {
        min_confidence_threshold: args.min_confidence,
        flag_overlapping: args.flag_overlapping,
        flag_empty_text: args.flag_empty_text,
        overlap_threshold: args.overlap_threshold,
    };

    let mut corpus_stats = CorpusStats {
        total_documents: 0,
        documents_with_flags: 0,
        total_pages: 0,
        total_flagged_pages: 0,
        total_elements: 0,
        total_low_confidence: 0,
        avg_confidence: 0.0,
        min_confidence: 1.0,
        label_counts: HashMap::new(),
        total_processing_time_ms: 0.0,
        documents: Vec::new(),
        config,
    };

    let mut all_flagged_pages: Vec<FlaggedPage> = Vec::new();
    let mut total_confidence_sum = 0.0f64;
    let mut total_confidence_count = 0usize;

    // Process each PDF
    for pdf_path in &pdf_files {
        let result = process_pdf(pdf_path, &args, &mut all_flagged_pages);

        match result {
            Ok(doc_stats) => {
                // Update corpus stats
                corpus_stats.total_documents += 1;
                if doc_stats.flagged_pages > 0 {
                    corpus_stats.documents_with_flags += 1;
                }
                corpus_stats.total_pages += doc_stats.total_pages;
                corpus_stats.total_flagged_pages += doc_stats.flagged_pages;
                corpus_stats.total_elements += doc_stats.total_elements;
                corpus_stats.total_low_confidence += doc_stats.low_confidence_elements;
                corpus_stats.total_processing_time_ms += doc_stats.processing_time_ms;

                // Track min confidence
                if doc_stats.min_confidence < corpus_stats.min_confidence {
                    corpus_stats.min_confidence = doc_stats.min_confidence;
                }

                // Accumulate confidence for average
                total_confidence_sum +=
                    doc_stats.avg_confidence as f64 * doc_stats.total_elements as f64;
                total_confidence_count += doc_stats.total_elements;

                // Merge label counts
                for (label, count) in &doc_stats.label_counts {
                    *corpus_stats.label_counts.entry(label.clone()).or_insert(0) += count;
                }

                corpus_stats.documents.push(doc_stats);
            }
            Err(e) => {
                eprintln!("Error processing {}: {}", pdf_path.display(), e);
            }
        }
    }

    // Calculate average confidence
    if total_confidence_count > 0 {
        corpus_stats.avg_confidence = (total_confidence_sum / total_confidence_count as f64) as f32;
    }

    // Write output
    write_output(&args, &corpus_stats, &all_flagged_pages)?;

    // Print summary
    print_summary(&corpus_stats);

    Ok(())
}

/// Collect PDF files from input paths (files or directories)
fn collect_pdf_files(inputs: &[PathBuf]) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut pdf_files = Vec::new();

    for input in inputs {
        if input.is_dir() {
            // Recursively find PDFs in directory
            for entry in walkdir(input)? {
                if entry
                    .extension()
                    .map(|e| e.to_ascii_lowercase() == "pdf")
                    .unwrap_or(false)
                {
                    pdf_files.push(entry);
                }
            }
        } else if input.is_file() {
            if input
                .extension()
                .map(|e| e.to_ascii_lowercase() == "pdf")
                .unwrap_or(false)
            {
                pdf_files.push(input.clone());
            }
        }
    }

    // Sort for deterministic order
    pdf_files.sort();
    Ok(pdf_files)
}

/// Simple recursive directory walk
fn walkdir(dir: &PathBuf) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut files = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            files.extend(walkdir(&path)?);
        } else {
            files.push(path);
        }
    }

    Ok(files)
}

/// Process a single PDF and collect statistics
fn process_pdf(
    pdf_path: &PathBuf,
    args: &Args,
    flagged_pages: &mut Vec<FlaggedPage>,
) -> Result<DocumentStats, Box<dyn std::error::Error>> {
    let filename = pdf_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    if args.verbose {
        println!("Processing: {}", pdf_path.display());
    }

    let start_time = std::time::Instant::now();

    // Create pipeline and load PDF
    let pipeline = dlviz_pipeline_new();
    if pipeline.is_null() {
        return Err("Failed to create pipeline".into());
    }

    let pdf_cstr = CString::new(pdf_path.to_string_lossy().as_bytes())?;
    let result = unsafe { dlviz_load_pdf(pipeline, pdf_cstr.as_ptr()) };

    if result != DlvizResult::Success {
        unsafe { dlviz_pipeline_free(pipeline) };
        return Err(format!("Failed to load PDF: {:?}", result).into());
    }

    let page_count = unsafe { dlviz_get_page_count(pipeline) };

    let mut doc_stats = DocumentStats {
        filename: filename.clone(),
        total_pages: page_count,
        flagged_pages: 0,
        total_elements: 0,
        low_confidence_elements: 0,
        overlap_pages: 0,
        empty_text_elements: 0,
        avg_confidence: 0.0,
        min_confidence: 1.0,
        label_counts: HashMap::new(),
        processing_time_ms: 0.0,
        flagged_page_list: Vec::new(),
    };

    let mut confidence_sum = 0.0f64;
    let mut confidence_count = 0usize;

    // Process each page
    for page_num in 0..page_count {
        // Run ML pipeline for this page to populate snapshot data
        let run_result =
            unsafe { dlviz_run_to_stage(pipeline, page_num, DlvizStage::ReadingOrder) };

        if run_result != DlvizResult::Success {
            if args.verbose {
                eprintln!("  Warning: ML pipeline failed for page {}", page_num);
            }
            continue;
        }

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

        // Analyze elements on this page
        let elements: Vec<DlvizElement> =
            if snapshot.element_count > 0 && !snapshot.elements.is_null() {
                unsafe {
                    std::slice::from_raw_parts(snapshot.elements, snapshot.element_count).to_vec()
                }
            } else {
                Vec::new()
            };

        let mut page_reasons: Vec<FlagReason> = Vec::new();

        for elem in &elements {
            doc_stats.total_elements += 1;
            confidence_sum += elem.confidence as f64;
            confidence_count += 1;

            // Track min confidence
            if elem.confidence < doc_stats.min_confidence {
                doc_stats.min_confidence = elem.confidence;
            }

            // Track label counts
            let label_str = format!("{:?}", elem.label);
            *doc_stats.label_counts.entry(label_str.clone()).or_insert(0) += 1;

            // Check confidence threshold
            if elem.confidence < args.min_confidence {
                doc_stats.low_confidence_elements += 1;
                page_reasons.push(FlagReason {
                    element_id: elem.id,
                    reason: format!(
                        "Low confidence: {:.2} < {:.2}",
                        elem.confidence, args.min_confidence
                    ),
                    confidence: Some(elem.confidence),
                    label: label_str.clone(),
                });
            }
        }

        // Check for overlapping boxes if requested
        if args.flag_overlapping {
            let overlaps = find_overlapping_boxes(&elements, args.overlap_threshold);
            for (id1, id2, iou) in overlaps {
                page_reasons.push(FlagReason {
                    element_id: id1,
                    reason: format!("Overlaps with element {} (IoU: {:.2})", id2, iou),
                    confidence: None,
                    label: String::new(),
                });
            }
            if !page_reasons.iter().any(|r| r.reason.contains("Overlaps")) {
                // No overlaps found
            } else {
                doc_stats.overlap_pages += 1;
            }
        }

        // Flag page if there are reasons
        if !page_reasons.is_empty() {
            // Check max flagged pages limit
            if args.max_flagged_pages == 0 || doc_stats.flagged_pages < args.max_flagged_pages {
                doc_stats.flagged_pages += 1;
                doc_stats.flagged_page_list.push(page_num);

                flagged_pages.push(FlaggedPage {
                    document: filename.clone(),
                    page: page_num,
                    reasons: page_reasons,
                });

                // Render flagged page if not stats-only
                if !args.stats_only {
                    let page_output = args.output.join(format!(
                        "{}_{:03}_flagged.png",
                        pdf_path.file_stem().unwrap_or_default().to_string_lossy(),
                        page_num
                    ));

                    let output_cstr = CString::new(page_output.to_string_lossy().as_bytes())?;
                    unsafe {
                        dlviz_render_visualization(
                            pipeline,
                            page_num,
                            DlvizStage::ReadingOrder,
                            args.scale,
                            output_cstr.as_ptr(),
                        )
                    };

                    if args.verbose {
                        println!("  Flagged page {} -> {}", page_num, page_output.display());
                    }
                }
            }
        }
    }

    // Calculate average confidence
    if confidence_count > 0 {
        doc_stats.avg_confidence = (confidence_sum / confidence_count as f64) as f32;
    }

    doc_stats.processing_time_ms = start_time.elapsed().as_secs_f64() * 1000.0;

    // Cleanup
    unsafe { dlviz_pipeline_free(pipeline) };

    if args.verbose {
        println!(
            "  {} pages, {} flagged, avg conf: {:.3}",
            doc_stats.total_pages, doc_stats.flagged_pages, doc_stats.avg_confidence
        );
    }

    Ok(doc_stats)
}

/// Find overlapping bounding boxes
fn find_overlapping_boxes(elements: &[DlvizElement], threshold: f32) -> Vec<(u32, u32, f32)> {
    let mut overlaps = Vec::new();

    for i in 0..elements.len() {
        for j in (i + 1)..elements.len() {
            let iou = calculate_iou(&elements[i].bbox, &elements[j].bbox);
            if iou > threshold {
                overlaps.push((elements[i].id, elements[j].id, iou));
            }
        }
    }

    overlaps
}

/// Calculate Intersection over Union (IoU) for two bboxes
fn calculate_iou(a: &DlvizBBox, b: &DlvizBBox) -> f32 {
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

/// Write output files
fn write_output(
    args: &Args,
    corpus_stats: &CorpusStats,
    flagged_pages: &[FlaggedPage],
) -> Result<(), Box<dyn std::error::Error>> {
    // Write corpus statistics
    let stats_path = args.output.join("corpus_stats.json");
    let stats_json = serde_json::to_string_pretty(corpus_stats)?;
    fs::write(&stats_path, &stats_json)?;
    println!("\nWrote corpus stats to: {}", stats_path.display());

    // Write flagged pages summary
    let flagged_path = args.output.join("flagged_pages.json");
    let flagged_json = serde_json::to_string_pretty(flagged_pages)?;
    fs::write(&flagged_path, &flagged_json)?;
    println!("Wrote flagged pages to: {}", flagged_path.display());

    // Write human-readable summary if text format requested
    if args.format == "text" {
        let summary_path = args.output.join("summary.txt");
        let mut file = fs::File::create(&summary_path)?;

        writeln!(file, "PDF Batch Review Summary")?;
        writeln!(file, "========================\n")?;
        writeln!(file, "Configuration:")?;
        writeln!(
            file,
            "  Min confidence threshold: {:.2}",
            corpus_stats.config.min_confidence_threshold
        )?;
        writeln!(
            file,
            "  Flag overlapping: {}",
            corpus_stats.config.flag_overlapping
        )?;
        writeln!(
            file,
            "  Flag empty text: {}",
            corpus_stats.config.flag_empty_text
        )?;
        writeln!(file)?;

        writeln!(file, "Corpus Statistics:")?;
        writeln!(file, "  Total documents: {}", corpus_stats.total_documents)?;
        writeln!(
            file,
            "  Documents with flags: {}",
            corpus_stats.documents_with_flags
        )?;
        writeln!(file, "  Total pages: {}", corpus_stats.total_pages)?;
        writeln!(
            file,
            "  Flagged pages: {} ({:.1}%)",
            corpus_stats.total_flagged_pages,
            corpus_stats.total_flagged_pages as f64 / corpus_stats.total_pages as f64 * 100.0
        )?;
        writeln!(file, "  Total elements: {}", corpus_stats.total_elements)?;
        writeln!(
            file,
            "  Low confidence elements: {} ({:.1}%)",
            corpus_stats.total_low_confidence,
            corpus_stats.total_low_confidence as f64 / corpus_stats.total_elements as f64 * 100.0
        )?;
        writeln!(
            file,
            "  Average confidence: {:.3}",
            corpus_stats.avg_confidence
        )?;
        writeln!(
            file,
            "  Minimum confidence: {:.3}",
            corpus_stats.min_confidence
        )?;
        writeln!(
            file,
            "  Processing time: {:.1}s",
            corpus_stats.total_processing_time_ms / 1000.0
        )?;
        writeln!(file)?;

        writeln!(file, "Element Label Distribution:")?;
        let mut labels: Vec<_> = corpus_stats.label_counts.iter().collect();
        labels.sort_by(|a, b| b.1.cmp(a.1));
        for (label, count) in labels {
            writeln!(
                file,
                "  {}: {} ({:.1}%)",
                label,
                count,
                *count as f64 / corpus_stats.total_elements as f64 * 100.0
            )?;
        }
        writeln!(file)?;

        writeln!(file, "Documents Requiring Review:")?;
        for doc in &corpus_stats.documents {
            if doc.flagged_pages > 0 {
                writeln!(
                    file,
                    "  {} - {} flagged pages ({:?})",
                    doc.filename, doc.flagged_pages, doc.flagged_page_list
                )?;
            }
        }

        println!("Wrote summary to: {}", summary_path.display());
    }

    Ok(())
}

/// Print summary to stdout
fn print_summary(corpus_stats: &CorpusStats) {
    println!("\n========== Batch Review Summary ==========\n");
    println!("Documents processed:    {}", corpus_stats.total_documents);
    println!(
        "Documents with issues:  {}",
        corpus_stats.documents_with_flags
    );
    println!("Total pages:            {}", corpus_stats.total_pages);
    println!(
        "Flagged pages:          {} ({:.1}%)",
        corpus_stats.total_flagged_pages,
        if corpus_stats.total_pages > 0 {
            corpus_stats.total_flagged_pages as f64 / corpus_stats.total_pages as f64 * 100.0
        } else {
            0.0
        }
    );
    println!("Total elements:         {}", corpus_stats.total_elements);
    println!(
        "Low confidence:         {} ({:.1}%)",
        corpus_stats.total_low_confidence,
        if corpus_stats.total_elements > 0 {
            corpus_stats.total_low_confidence as f64 / corpus_stats.total_elements as f64 * 100.0
        } else {
            0.0
        }
    );
    println!("Average confidence:     {:.3}", corpus_stats.avg_confidence);
    println!("Minimum confidence:     {:.3}", corpus_stats.min_confidence);
    println!(
        "Processing time:        {:.1}s",
        corpus_stats.total_processing_time_ms / 1000.0
    );
    println!("\n==========================================");
}
