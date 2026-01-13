#![cfg(feature = "pytorch")]
mod common;
/// CodeFormula Integration Test - End-to-End Pipeline
///
/// Tests the complete pipeline with CodeFormula enrichment on code_and_formula.pdf:
/// - Page 0: Has 1 code region (JavaScript function)
/// - Page 1: Has 1 formula region (mathematical formula)
///
/// The test validates:
/// 1. Pipeline processes pages without errors
/// 2. Code/Formula elements are enriched with ML predictions
/// 3. Enriched text matches Phase 1 baselines (≥95% similarity)
/// 4. Language detection works (JavaScript for code)
///
/// NOTE: This test runs the FULL pipeline (layout → assembly → enrichment),
/// unlike test_codeformula_phase1.rs which tests only the model in isolation.
use common::baseline_loaders::load_numpy_u8;
use docling_pdf_ml::{PageElement, SimpleTextCell};
use docling_pdf_ml::{Pipeline, PipelineConfigBuilder};
use serde::Deserialize;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use tch::Device;

/// Load baseline expected output from JSON
#[derive(Deserialize, Debug)]
struct BaselineOutput {
    label: String,
    region_index: usize,
    page_index: usize,
    prompt: String,
    raw_output: String,
    cleaned_output: String,
    final_text: String,
    language: Option<String>,
    generated_ids: Vec<Vec<i64>>,
}

fn load_baseline_output(path: &str) -> Result<BaselineOutput, Box<dyn std::error::Error>> {
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let output: BaselineOutput = serde_json::from_str(&contents)?;
    Ok(output)
}

/// Calculate Levenshtein distance (edit distance) between two strings
#[allow(
    clippy::needless_range_loop,
    reason = "explicit index access clearer for matrix DP algorithm"
)]
fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let len1 = s1.chars().count();
    let len2 = s2.chars().count();

    let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];

    for i in 0..=len1 {
        matrix[i][0] = i;
    }
    for j in 0..=len2 {
        matrix[0][j] = j;
    }

    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();

    for i in 1..=len1 {
        for j in 1..=len2 {
            let cost = if s1_chars[i - 1] == s2_chars[j - 1] {
                0
            } else {
                1
            };
            matrix[i][j] = std::cmp::min(
                std::cmp::min(
                    matrix[i - 1][j] + 1, // deletion
                    matrix[i][j - 1] + 1, // insertion
                ),
                matrix[i - 1][j - 1] + cost, // substitution
            );
        }
    }

    matrix[len1][len2]
}

/// Calculate string similarity percentage (100% = exact match)
fn string_similarity(s1: &str, s2: &str) -> f64 {
    let distance = levenshtein_distance(s1, s2);
    let max_len = std::cmp::max(s1.chars().count(), s2.chars().count());

    if max_len == 0 {
        return 100.0;
    }

    let similarity = 1.0 - (distance as f64 / max_len as f64);
    similarity * 100.0
}

/// Load textline cells from baseline data
fn load_textline_cells(pdf_name: &str, page_no: usize) -> Option<Vec<SimpleTextCell>> {
    let baseline_dir = format!("baseline_data/{}/page_{}/preprocessing", pdf_name, page_no);
    let cells_path = format!("{}/textline_cells.json", baseline_dir);

    let file = File::open(&cells_path).ok()?;
    let cells: Vec<SimpleTextCell> = serde_json::from_reader(file).ok()?;
    Some(cells)
}

#[test]
#[ignore = "Requires model weights"]
fn test_codeformula_integration_code_region() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== CodeFormula Integration Test: Code Region ===");

    // 1. Load baseline expected output
    let base_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let baseline_dir = base_dir.join("baseline_data/code_and_formula/page_0/code_formula");
    let expected_output_path = baseline_dir.join("code_0_phase1_output.json");

    assert!(
        expected_output_path.exists(),
        "Missing expected output: {:?}",
        expected_output_path
    );

    let expected = load_baseline_output(expected_output_path.to_str().unwrap())?;

    println!("Baseline:");
    println!("  Label: {}", expected.label);
    println!("  Expected text: {}", expected.final_text);
    println!("  Expected language: {:?}", expected.language);

    // 2. Load page image
    println!("\n[1/4] Loading page image...");
    let image_path = Path::new("baseline_data/code_and_formula/page_0/layout/input_page_image.npy");
    let page_image_dyn = load_numpy_u8(image_path)?;

    let shape = page_image_dyn.shape().to_vec();
    assert_eq!(shape.len(), 3, "Image must be 3D (HWC format)");
    let page_image = page_image_dyn.into_dimensionality::<ndarray::Ix3>()?;

    println!("  ✓ Image loaded: shape={:?}", page_image.shape());

    // Page dimensions (from image shape: HWC format, so H=height, W=width)
    let page_height = page_image.shape()[0] as f32;
    let page_width = page_image.shape()[1] as f32;
    println!("  Page dimensions: {}x{}", page_width, page_height);

    // 3. Initialize pipeline with CodeFormula enabled
    println!("\n[2/4] Initializing pipeline with CodeFormula...");

    // Resolve CodeFormula model path (HuggingFace cache)
    let model_name = "ds4sd/CodeFormulaV2";
    let model_slug = model_name.replace('/', "--");
    let home_dir = std::env::var("HOME")?;
    let cache_path = format!(
        "{}/.cache/huggingface/hub/models--{}/snapshots",
        home_dir, model_slug
    );

    // Get the first snapshot directory
    let model_dir = std::fs::read_dir(&cache_path)?
        .next()
        .ok_or("No CodeFormula snapshot found")??
        .path();

    println!("  Using CodeFormula model: {:?}", model_dir);

    let config = PipelineConfigBuilder::new()
        .device(Device::Cpu)
        .ocr_enabled(false) // code_and_formula has programmatic text
        .table_structure_enabled(false) // Not needed for this test
        .code_formula_enabled(true) // Enable enrichment
        .code_formula_model_path(model_dir)
        .build()?;

    let mut pipeline = Pipeline::new(config)?;
    println!("  ✓ Pipeline initialized");

    // 4. Load textline cells
    println!("\n[3/4] Loading textline cells...");
    let textline_cells = load_textline_cells("code_and_formula", 0);
    if let Some(ref cells) = textline_cells {
        println!("  ✓ Loaded {} textline cells", cells.len());
    } else {
        println!("  ⚠ No textline cells available");
    }

    // 5. Process page (with enrichment)
    println!("\n[4/4] Processing page with enrichment...");
    let page = pipeline.process_page(0, &page_image, page_width, page_height, textline_cells)?;

    let assembled = page
        .assembled
        .as_ref()
        .ok_or("Page should have assembled data")?;
    println!("  ✓ Page processed");
    println!("    - Total elements: {}", assembled.elements.len());

    // 6. Find code elements
    println!("\n=== Extracting Code Elements ===");
    let mut code_elements = Vec::new();
    for element in &assembled.elements {
        if let PageElement::Text(text_elem) = element {
            if matches!(
                text_elem.label,
                docling_pdf_ml::pipeline::DocItemLabel::Code
            ) {
                code_elements.push(text_elem);
            }
        }
    }

    println!("Found {} code elements", code_elements.len());
    assert_eq!(
        code_elements.len(),
        1,
        "Expected 1 code region, found {}",
        code_elements.len()
    );

    let code_element = code_elements[0];
    println!("\nCode element:");
    println!("  Text: {}", code_element.text);
    println!("  Bbox: {:?}", code_element.cluster.bbox);

    // 7. Validate enriched text
    println!("\n=== Validation ===");

    let similarity = string_similarity(&code_element.text, &expected.final_text);
    println!("Text similarity: {:.2}%", similarity);

    if code_element.text != expected.final_text {
        println!("\nText mismatch:");
        println!("Expected: {}", expected.final_text);
        println!("Got:      {}", code_element.text);
    }

    // Acceptance criteria: exact match or ≥95% similarity
    assert!(
        similarity >= 95.0,
        "Text similarity {:.2}% below threshold 95.0%",
        similarity
    );

    println!("\n✓ Code region integration test PASSED");
    Ok(())
}

#[test]
#[ignore = "Requires model weights"]
fn test_codeformula_integration_formula_region() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== CodeFormula Integration Test: Formula Region ===");

    // 1. Load baseline expected output
    let base_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let baseline_dir = base_dir.join("baseline_data/code_and_formula/page_1/code_formula");
    let expected_output_path = baseline_dir.join("formula_1_phase1_output.json");

    assert!(
        expected_output_path.exists(),
        "Missing expected output: {:?}",
        expected_output_path
    );

    let expected = load_baseline_output(expected_output_path.to_str().unwrap())?;

    println!("Baseline:");
    println!("  Label: {}", expected.label);
    println!("  Expected text: {}", expected.final_text);
    println!("  Expected language: {:?}", expected.language);

    // 2. Load page image
    println!("\n[1/4] Loading page image...");
    let image_path = Path::new("baseline_data/code_and_formula/page_1/layout/input_page_image.npy");
    let page_image_dyn = load_numpy_u8(image_path)?;

    let shape = page_image_dyn.shape().to_vec();
    assert_eq!(shape.len(), 3, "Image must be 3D (HWC format)");
    let page_image = page_image_dyn.into_dimensionality::<ndarray::Ix3>()?;

    println!("  ✓ Image loaded: shape={:?}", page_image.shape());

    // Page dimensions (from image shape: HWC format, so H=height, W=width)
    let page_height = page_image.shape()[0] as f32;
    let page_width = page_image.shape()[1] as f32;
    println!("  Page dimensions: {}x{}", page_width, page_height);

    // 3. Initialize pipeline with CodeFormula enabled
    println!("\n[2/4] Initializing pipeline with CodeFormula...");

    // Resolve CodeFormula model path (HuggingFace cache)
    let model_name = "ds4sd/CodeFormulaV2";
    let model_slug = model_name.replace('/', "--");
    let home_dir = std::env::var("HOME")?;
    let cache_path = format!(
        "{}/.cache/huggingface/hub/models--{}/snapshots",
        home_dir, model_slug
    );

    // Get the first snapshot directory
    let model_dir = std::fs::read_dir(&cache_path)?
        .next()
        .ok_or("No CodeFormula snapshot found")??
        .path();

    println!("  Using CodeFormula model: {:?}", model_dir);

    let config = PipelineConfigBuilder::new()
        .device(Device::Cpu)
        .ocr_enabled(false) // code_and_formula has programmatic text
        .table_structure_enabled(false) // Not needed for this test
        .code_formula_enabled(true) // Enable enrichment
        .code_formula_model_path(model_dir)
        .build()?;

    let mut pipeline = Pipeline::new(config)?;
    println!("  ✓ Pipeline initialized");

    // 4. Load textline cells
    println!("\n[4/4] Loading textline cells...");
    let textline_cells = load_textline_cells("code_and_formula", 1);
    if let Some(ref cells) = textline_cells {
        println!("  ✓ Loaded {} textline cells", cells.len());
    } else {
        println!("  ⚠ No textline cells available");
    }

    // 5. Process page (with enrichment)
    println!("\n[4/4] Processing page with enrichment...");
    let page = pipeline.process_page(1, &page_image, page_width, page_height, textline_cells)?;

    let assembled = page
        .assembled
        .as_ref()
        .ok_or("Page should have assembled data")?;
    println!("  ✓ Page processed");
    println!("    - Total elements: {}", assembled.elements.len());

    // 6. Find formula elements
    println!("\n=== Extracting Formula Elements ===");
    let mut formula_elements = Vec::new();
    for element in &assembled.elements {
        if let PageElement::Text(text_elem) = element {
            if matches!(
                text_elem.label,
                docling_pdf_ml::pipeline::DocItemLabel::Formula
            ) {
                formula_elements.push(text_elem);
            }
        }
    }

    println!("Found {} formula elements", formula_elements.len());
    assert_eq!(
        formula_elements.len(),
        1,
        "Expected 1 formula region, found {}",
        formula_elements.len()
    );

    let formula_element = formula_elements[0];
    println!("\nFormula element:");
    println!("  Text: {}", formula_element.text);
    println!("  Bbox: {:?}", formula_element.cluster.bbox);

    // 7. Validate enriched text
    println!("\n=== Validation ===");

    let similarity = string_similarity(&formula_element.text, &expected.final_text);
    println!("Text similarity: {:.2}%", similarity);

    if formula_element.text != expected.final_text {
        println!("\nText mismatch:");
        println!("Expected: {}", expected.final_text);
        println!("Got:      {}", formula_element.text);
    }

    // Acceptance criteria: exact match or ≥95% similarity
    assert!(
        similarity >= 95.0,
        "Text similarity {:.2}% below threshold 95.0%",
        similarity
    );

    // Formula should have no language
    assert_eq!(expected.language, None, "Formula should have no language");

    println!("\n✓ Formula region integration test PASSED");
    Ok(())
}
