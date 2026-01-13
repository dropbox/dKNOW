//! CLI tool to apply AI corrections to PDF detection data
//!
//! Reads AI-generated corrections and applies them to detection data,
//! then exports to COCO and YOLO training formats.
//!
//! # Usage
//!
//! ```bash
//! dlviz-apply-corrections ./review/ --output ./golden/ --format coco
//! ```

use chrono::{DateTime, Utc};
use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Output format for training data export
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ValueEnum)]
enum OutputFormat {
    /// COCO JSON format (for detectron2, mmdetection)
    Coco,
    /// YOLO TXT format (for ultralytics)
    Yolo,
    /// Both COCO and YOLO
    Both,
}

/// Apply AI corrections and export to training formats
#[derive(Parser, Debug)]
#[command(name = "dlviz-apply-corrections")]
#[command(version, about, long_about = None)]
struct Args {
    /// Directory containing review outputs and corrections.json
    review_dir: PathBuf,

    /// Output directory for corrected/training data
    #[arg(short, long, default_value = "./golden")]
    output: PathBuf,

    /// Output format
    #[arg(long, value_enum, default_value = "both")]
    format: OutputFormat,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

// =============================================================================
// Input Data Structures (from dlviz-screenshot output)
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
// Corrections Format (written by AI)
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum Correction {
    #[serde(rename = "bbox")]
    BBox {
        page: usize,
        element_id: u32,
        original: BBoxInfo,
        corrected: BBoxInfo,
        reason: String,
    },
    #[serde(rename = "label")]
    Label {
        page: usize,
        element_id: u32,
        original: String,
        corrected: String,
        reason: String,
    },
    #[serde(rename = "add")]
    Add {
        page: usize,
        label: String,
        bbox: BBoxInfo,
        #[serde(skip_serializing_if = "Option::is_none")]
        text: Option<String>,
        reason: String,
    },
    #[serde(rename = "delete")]
    Delete {
        page: usize,
        element_id: u32,
        reason: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CorrectionsSummary {
    pages_reviewed: usize,
    total_corrections: usize,
    bbox_corrections: usize,
    label_corrections: usize,
    additions: usize,
    deletions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CorrectionsFile {
    document: String,
    reviewed_by: String,
    timestamp: DateTime<Utc>,
    corrections: Vec<Correction>,
    summary: CorrectionsSummary,
}

// =============================================================================
// COCO Format
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CocoImage {
    id: u32,
    file_name: String,
    width: u32,
    height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CocoAnnotation {
    id: u32,
    image_id: u32,
    category_id: u32,
    bbox: [f32; 4], // [x, y, width, height]
    area: f32,
    iscrowd: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CocoCategory {
    id: u32,
    name: String,
    supercategory: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CocoDataset {
    images: Vec<CocoImage>,
    annotations: Vec<CocoAnnotation>,
    categories: Vec<CocoCategory>,
}

// =============================================================================
// Label Mapping
// =============================================================================

/// Map label strings to category IDs
fn label_to_category_id(label: &str) -> u32 {
    match label.to_lowercase().as_str() {
        "caption" => 1,
        "footnote" => 2,
        "formula" => 3,
        "list" | "listitem" | "list_item" => 4,
        "footer" | "pagefooter" | "page_footer" => 5,
        "header" | "pageheader" | "page_header" => 6,
        "picture" => 7,
        "section" | "sectionheader" | "section_header" => 8,
        "table" => 9,
        "text" => 10,
        "title" => 11,
        "code" => 12,
        "checkbox" | "checkboxselected" | "checkboxunselected" => 13,
        "index" | "documentindex" | "document_index" => 14,
        "form" => 15,
        "kv" | "keyvalueregion" | "key_value_region" => 16,
        _ => 0, // Unknown
    }
}

/// Get all category definitions
fn get_coco_categories() -> Vec<CocoCategory> {
    vec![
        CocoCategory {
            id: 1,
            name: "caption".to_string(),
            supercategory: "document".to_string(),
        },
        CocoCategory {
            id: 2,
            name: "footnote".to_string(),
            supercategory: "document".to_string(),
        },
        CocoCategory {
            id: 3,
            name: "formula".to_string(),
            supercategory: "document".to_string(),
        },
        CocoCategory {
            id: 4,
            name: "list_item".to_string(),
            supercategory: "document".to_string(),
        },
        CocoCategory {
            id: 5,
            name: "page_footer".to_string(),
            supercategory: "document".to_string(),
        },
        CocoCategory {
            id: 6,
            name: "page_header".to_string(),
            supercategory: "document".to_string(),
        },
        CocoCategory {
            id: 7,
            name: "picture".to_string(),
            supercategory: "document".to_string(),
        },
        CocoCategory {
            id: 8,
            name: "section_header".to_string(),
            supercategory: "document".to_string(),
        },
        CocoCategory {
            id: 9,
            name: "table".to_string(),
            supercategory: "document".to_string(),
        },
        CocoCategory {
            id: 10,
            name: "text".to_string(),
            supercategory: "document".to_string(),
        },
        CocoCategory {
            id: 11,
            name: "title".to_string(),
            supercategory: "document".to_string(),
        },
        CocoCategory {
            id: 12,
            name: "code".to_string(),
            supercategory: "document".to_string(),
        },
        CocoCategory {
            id: 13,
            name: "checkbox".to_string(),
            supercategory: "document".to_string(),
        },
        CocoCategory {
            id: 14,
            name: "document_index".to_string(),
            supercategory: "document".to_string(),
        },
        CocoCategory {
            id: 15,
            name: "form".to_string(),
            supercategory: "document".to_string(),
        },
        CocoCategory {
            id: 16,
            name: "key_value_region".to_string(),
            supercategory: "document".to_string(),
        },
    ]
}

// =============================================================================
// Correction Application
// =============================================================================

fn apply_corrections(
    mut elements: Vec<ElementInfo>,
    corrections: &[Correction],
    page: usize,
    next_id: &mut u32,
) -> Vec<ElementInfo> {
    // Apply corrections for this page
    for correction in corrections {
        match correction {
            Correction::BBox {
                page: p,
                element_id,
                corrected,
                ..
            } => {
                if *p == page {
                    if let Some(elem) = elements.iter_mut().find(|e| e.id == *element_id) {
                        elem.bbox = corrected.clone();
                    }
                }
            }
            Correction::Label {
                page: p,
                element_id,
                corrected,
                ..
            } => {
                if *p == page {
                    if let Some(elem) = elements.iter_mut().find(|e| e.id == *element_id) {
                        elem.label = corrected.clone();
                    }
                }
            }
            Correction::Add {
                page: p,
                label,
                bbox,
                text,
                ..
            } => {
                if *p == page {
                    elements.push(ElementInfo {
                        id: *next_id,
                        label: label.clone(),
                        confidence: 1.0, // Human-corrected = 100% confidence
                        bbox: bbox.clone(),
                        text: text.clone(),
                        reading_order: -1, // Needs to be recalculated
                    });
                    *next_id += 1;
                }
            }
            Correction::Delete {
                page: p,
                element_id,
                ..
            } => {
                if *p == page {
                    elements.retain(|e| e.id != *element_id);
                }
            }
        }
    }

    elements
}

// =============================================================================
// Export Functions
// =============================================================================

fn export_to_coco(
    pages: &[(usize, Vec<ElementInfo>, (u32, u32), String)],
    output_dir: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut images = Vec::new();
    let mut annotations = Vec::new();
    let mut annotation_id: u32 = 1;

    for (page_idx, (_page_num, elements, (width, height), image_file)) in pages.iter().enumerate() {
        let image_id = page_idx as u32 + 1;

        images.push(CocoImage {
            id: image_id,
            file_name: image_file.clone(),
            width: *width,
            height: *height,
        });

        for elem in elements {
            let category_id = label_to_category_id(&elem.label);
            if category_id == 0 {
                continue; // Skip unknown labels
            }

            let area = elem.bbox.width * elem.bbox.height;
            annotations.push(CocoAnnotation {
                id: annotation_id,
                image_id,
                category_id,
                bbox: [elem.bbox.x, elem.bbox.y, elem.bbox.width, elem.bbox.height],
                area,
                iscrowd: 0,
            });
            annotation_id += 1;
        }
    }

    let dataset = CocoDataset {
        images,
        annotations,
        categories: get_coco_categories(),
    };

    let output_path = output_dir.join("annotations.json");
    let json = serde_json::to_string_pretty(&dataset)?;
    std::fs::write(&output_path, json)?;

    println!("  COCO: {}", output_path.display());
    Ok(())
}

fn export_to_yolo(
    pages: &[(usize, Vec<ElementInfo>, (u32, u32), String)],
    output_dir: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let labels_dir = output_dir.join("labels");
    std::fs::create_dir_all(&labels_dir)?;

    for (_page_num, elements, (width, height), image_file) in pages {
        let label_file = labels_dir.join(
            PathBuf::from(image_file)
                .with_extension("txt")
                .file_name()
                .unwrap(),
        );

        let mut lines = Vec::new();
        for elem in elements {
            let category_id = label_to_category_id(&elem.label);
            if category_id == 0 {
                continue;
            }

            // YOLO format: class_id x_center y_center width height (normalized 0-1)
            let x_center = (elem.bbox.x + elem.bbox.width / 2.0) / *width as f32;
            let y_center = (elem.bbox.y + elem.bbox.height / 2.0) / *height as f32;
            let norm_width = elem.bbox.width / *width as f32;
            let norm_height = elem.bbox.height / *height as f32;

            // YOLO uses 0-indexed class IDs
            lines.push(format!(
                "{} {:.6} {:.6} {:.6} {:.6}",
                category_id - 1,
                x_center,
                y_center,
                norm_width,
                norm_height
            ));
        }

        std::fs::write(&label_file, lines.join("\n"))?;
    }

    // Write classes.txt
    let classes_path = output_dir.join("classes.txt");
    let classes: Vec<&str> = vec![
        "caption",
        "footnote",
        "formula",
        "list_item",
        "page_footer",
        "page_header",
        "picture",
        "section_header",
        "table",
        "text",
        "title",
        "code",
        "checkbox",
        "document_index",
        "form",
        "key_value_region",
    ];
    std::fs::write(&classes_path, classes.join("\n"))?;

    println!("  YOLO: {}/", labels_dir.display());
    println!("  Classes: {}", classes_path.display());
    Ok(())
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

    println!("Applying corrections from: {}", args.review_dir.display());

    // Read corrections.json
    let corrections_path = args.review_dir.join("corrections.json");
    let corrections: CorrectionsFile = if corrections_path.exists() {
        let content = std::fs::read_to_string(&corrections_path)?;
        serde_json::from_str(&content)?
    } else {
        println!("No corrections.json found - will export uncorrected data");
        CorrectionsFile {
            document: "unknown".to_string(),
            reviewed_by: "none".to_string(),
            timestamp: Utc::now(),
            corrections: vec![],
            summary: CorrectionsSummary {
                pages_reviewed: 0,
                total_corrections: 0,
                bbox_corrections: 0,
                label_corrections: 0,
                additions: 0,
                deletions: 0,
            },
        }
    };

    if !corrections.corrections.is_empty() {
        println!(
            "  Document: {} (reviewed by {})",
            corrections.document, corrections.reviewed_by
        );
        println!("  Corrections: {}", corrections.corrections.len());
    }

    // Find all JSON sidecar files
    let mut json_files: Vec<PathBuf> = std::fs::read_dir(&args.review_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().is_some_and(|e| e == "json")
                && p.file_name().map_or(true, |n| n != "corrections.json")
        })
        .collect();

    json_files.sort();

    if json_files.is_empty() {
        println!("No JSON sidecar files found in review directory");
        return Ok(());
    }

    println!("  Found {} page JSON files", json_files.len());

    // Process each page
    let mut pages: Vec<(usize, Vec<ElementInfo>, (u32, u32), String)> = Vec::new();
    let mut next_id: u32 = 10000; // Start added elements at high ID

    for json_path in &json_files {
        let content = std::fs::read_to_string(json_path)?;
        let sidecar: VisualizationSidecar = serde_json::from_str(&content)?;

        // Get page size (default to letter size if not specified)
        let page_size = sidecar
            .page_size
            .map(|s| (s.width as u32, s.height as u32))
            .unwrap_or((612, 792));

        // Apply corrections to this page
        let corrected_elements = apply_corrections(
            sidecar.elements,
            &corrections.corrections,
            sidecar.page,
            &mut next_id,
        );

        // Get corresponding PNG filename
        let image_file = json_path
            .with_extension("png")
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();

        pages.push((sidecar.page, corrected_elements, page_size, image_file));
    }

    // Create output directory
    std::fs::create_dir_all(&args.output)?;

    // Save corrected JSON files
    let corrected_dir = args.output.join("corrected");
    std::fs::create_dir_all(&corrected_dir)?;
    for (page_num, elements, page_size, _) in &pages {
        let output_path = corrected_dir.join(format!("page_{:03}.json", page_num));
        let output = serde_json::json!({
            "page": page_num,
            "page_size": {
                "width": page_size.0,
                "height": page_size.1
            },
            "element_count": elements.len(),
            "elements": elements
        });
        std::fs::write(&output_path, serde_json::to_string_pretty(&output)?)?;
    }
    println!("  Corrected JSON: {}/", corrected_dir.display());

    // Export to requested format(s)
    println!("Exporting to {}:", args.output.display());

    match args.format {
        OutputFormat::Coco => {
            export_to_coco(&pages, &args.output)?;
        }
        OutputFormat::Yolo => {
            export_to_yolo(&pages, &args.output)?;
        }
        OutputFormat::Both => {
            export_to_coco(&pages, &args.output)?;
            export_to_yolo(&pages, &args.output)?;
        }
    }

    // Print summary
    println!();
    println!("Summary:");
    println!("  Pages processed: {}", pages.len());
    let total_elements: usize = pages.iter().map(|(_, e, _, _)| e.len()).sum();
    println!("  Total elements: {}", total_elements);
    if !corrections.corrections.is_empty() {
        println!("  Corrections applied: {}", corrections.corrections.len());
    }

    Ok(())
}
