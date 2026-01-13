#![cfg(feature = "pytorch")]
// Phase 1 validation: Test CodeFormula model with preprocessed baseline inputs
//
// This test validates that the Rust CodeFormula implementation produces outputs
// matching the Python baseline when given identical preprocessed inputs.
//
// Baseline data extracted from: code_and_formula.pdf pages 0-1
// - Page 0: code region (JavaScript function)
// - Page 1: formula region (mathematical formula)
use npyz::NpyFile;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

// Import our CodeFormula model
use docling_pdf_ml::models::code_formula::CodeFormulaModel;

/// Load baseline expected output from JSON
#[derive(serde::Deserialize, Debug)]
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

#[test]
#[ignore = "Requires model weights"]
fn test_codeformula_phase1_code_region() -> Result<(), Box<dyn std::error::Error>> {
    // Load baseline data
    let base_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let baseline_dir = base_dir.join("baseline_data/code_and_formula/page_0/code_formula");

    let pixel_values_path = baseline_dir.join("code_0_pixel_values.npy");
    let input_ids_path = baseline_dir.join("code_0_input_ids.npy");
    let attention_mask_path = baseline_dir.join("code_0_attention_mask.npy");
    let expected_output_path = baseline_dir.join("code_0_phase1_output.json");

    // Verify files exist
    assert!(
        pixel_values_path.exists(),
        "Missing pixel_values: {:?}",
        pixel_values_path
    );
    assert!(
        input_ids_path.exists(),
        "Missing input_ids: {:?}",
        input_ids_path
    );
    assert!(
        attention_mask_path.exists(),
        "Missing attention_mask: {:?}",
        attention_mask_path
    );
    assert!(
        expected_output_path.exists(),
        "Missing expected output: {:?}",
        expected_output_path
    );

    // Load expected output
    let expected = load_baseline_output(expected_output_path.to_str().unwrap())
        .expect("Failed to load baseline output");

    println!("\n=== Code Region Baseline ===");
    println!("Label: {}", expected.label);
    println!("Expected text: {}", expected.final_text);
    println!("Expected language: {:?}", expected.language);
    println!(
        "Expected first 20 token IDs: {:?}",
        &expected.generated_ids[0][..20]
    );

    // Load model
    println!("\n=== Loading CodeFormula Model ===");
    let model_name = "ds4sd/CodeFormulaV2";
    let device = tch::Device::cuda_if_available();
    println!("Using device: {:?}", device);

    // Resolve model name to cache path
    let model_slug = model_name.replace('/', "--");
    let home_dir = std::env::var("HOME").expect("HOME not set");
    let cache_path = format!("{home_dir}/.cache/huggingface/hub/models--{model_slug}/snapshots");

    println!("Looking for model in: {}", cache_path);

    // Get the first snapshot
    let model_dir = std::fs::read_dir(&cache_path)
        .expect("Failed to read snapshots directory")
        .next()
        .expect("No snapshot found")?
        .path();

    println!("Loading model from: {:?}", model_dir);

    let model = CodeFormulaModel::from_pretrained(&model_dir, device)
        .expect("Failed to load CodeFormula model");

    println!("Model loaded successfully");

    // Load input tensors from .npy files
    println!("\n=== Loading Input Tensors ===");

    // Load pixel_values using npyz
    let pixel_values_file =
        File::open(&pixel_values_path).expect("Failed to open pixel_values file");
    let pixel_values_npy =
        NpyFile::new(pixel_values_file).expect("Failed to parse pixel_values .npy");

    let pixel_values_shape: Vec<i64> = pixel_values_npy.shape().iter().map(|&x| x as i64).collect();
    println!("Pixel values shape: {:?}", pixel_values_shape);

    let pixel_values_vec: Vec<f32> = pixel_values_npy
        .into_vec()
        .expect("Failed to read pixel_values data");

    let pixel_values = tch::Tensor::from_slice(&pixel_values_vec)
        .reshape(&pixel_values_shape)
        .to_device(device);

    // Load input_ids
    let input_ids_file = File::open(&input_ids_path).expect("Failed to open input_ids file");
    let input_ids_npy = NpyFile::new(input_ids_file).expect("Failed to parse input_ids .npy");

    let input_ids_shape: Vec<i64> = input_ids_npy.shape().iter().map(|&x| x as i64).collect();
    println!("Input IDs shape: {:?}", input_ids_shape);

    let input_ids_vec: Vec<i64> = input_ids_npy
        .into_vec()
        .expect("Failed to read input_ids data");

    let input_ids = tch::Tensor::from_slice(&input_ids_vec)
        .reshape(&input_ids_shape)
        .to_device(device);

    // Load attention_mask
    let attention_mask_file =
        File::open(&attention_mask_path).expect("Failed to open attention_mask file");
    let attention_mask_npy =
        NpyFile::new(attention_mask_file).expect("Failed to parse attention_mask .npy");

    let attention_mask_shape: Vec<i64> = attention_mask_npy
        .shape()
        .iter()
        .map(|&x| x as i64)
        .collect();
    println!("Attention mask shape: {:?}", attention_mask_shape);

    let attention_mask_vec: Vec<i64> = attention_mask_npy
        .into_vec()
        .expect("Failed to read attention_mask data");

    let attention_mask = tch::Tensor::from_slice(&attention_mask_vec)
        .reshape(&attention_mask_shape)
        .to_device(device);

    // Run inference
    println!("\n=== Running Inference ===");
    let output_ids = model
        .generate_from_preprocessed(
            &input_ids,
            &pixel_values,
            Some(&attention_mask),
            512, // max_new_tokens
        )
        .expect("Inference failed");

    println!("Generated {} tokens", output_ids.size()[1]);

    // Debug: print first 20 and last 20 token IDs
    let output_vec: Vec<i64> = output_ids
        .flatten(0, -1)
        .try_into()
        .expect("Failed to extract token IDs");
    println!(
        "Rust first 20 generated tokens: {:?}",
        &output_vec[..20.min(output_vec.len())]
    );
    println!(
        "Rust last 20 generated tokens: {:?}",
        &output_vec[output_vec.len().saturating_sub(20)..]
    );

    // Decode output
    let output_text = model
        .decode_tokens(&output_ids)
        .expect("Failed to decode tokens");

    println!("\n=== Generated Output ===");
    println!("Raw: {}", output_text);

    // Post-process output (remove special tokens, extract language)
    let cleaned = model.post_process(&output_text);

    println!("\n=== Cleaned Output ===");
    println!("Text: {}", cleaned.text);
    println!("Language: {:?}", cleaned.language);

    // Validate output
    println!("\n=== Validation ===");

    // Check language match
    let language_match = cleaned.language == expected.language;
    println!(
        "Language match: {} (expected: {:?}, got: {:?})",
        if language_match { "✓" } else { "✗" },
        expected.language,
        cleaned.language
    );

    // Check text similarity
    let similarity = string_similarity(&cleaned.text, &expected.final_text);
    println!("Text similarity: {:.2}%", similarity);

    if cleaned.text != expected.final_text {
        println!("\nText mismatch:");
        println!("Expected: {}", expected.final_text);
        println!("Got:      {}", cleaned.text);
    }

    // Acceptance criteria: exact match or ≥95% similarity
    assert!(
        similarity >= 95.0,
        "Text similarity {:.2}% below threshold 95.0%",
        similarity
    );
    assert!(
        language_match,
        "Language mismatch: expected {:?}, got {:?}",
        expected.language, cleaned.language
    );

    println!("\n✓ Code region validation PASSED");
    Ok(())
}

#[test]
#[ignore = "Requires model weights"]
fn test_codeformula_phase1_formula_region() -> Result<(), Box<dyn std::error::Error>> {
    // Load baseline data
    let base_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let baseline_dir = base_dir.join("baseline_data/code_and_formula/page_1/code_formula");

    let pixel_values_path = baseline_dir.join("formula_1_pixel_values.npy");
    let input_ids_path = baseline_dir.join("formula_1_input_ids.npy");
    let attention_mask_path = baseline_dir.join("formula_1_attention_mask.npy");
    let expected_output_path = baseline_dir.join("formula_1_phase1_output.json");

    // Verify files exist
    assert!(
        pixel_values_path.exists(),
        "Missing pixel_values: {:?}",
        pixel_values_path
    );
    assert!(
        input_ids_path.exists(),
        "Missing input_ids: {:?}",
        input_ids_path
    );
    assert!(
        attention_mask_path.exists(),
        "Missing attention_mask: {:?}",
        attention_mask_path
    );
    assert!(
        expected_output_path.exists(),
        "Missing expected output: {:?}",
        expected_output_path
    );

    // Load expected output
    let expected = load_baseline_output(expected_output_path.to_str().unwrap())
        .expect("Failed to load baseline output");

    println!("\n=== Formula Region Baseline ===");
    println!("Label: {}", expected.label);
    println!("Expected text: {}", expected.final_text);
    println!("Expected language: {:?}", expected.language);

    // Load model
    println!("\n=== Loading CodeFormula Model ===");
    let model_name = "ds4sd/CodeFormulaV2";
    let device = tch::Device::cuda_if_available();
    println!("Using device: {:?}", device);

    // Resolve model name to cache path
    let model_slug = model_name.replace('/', "--");
    let home_dir = std::env::var("HOME").expect("HOME not set");
    let cache_path = format!("{home_dir}/.cache/huggingface/hub/models--{model_slug}/snapshots");

    println!("Looking for model in: {}", cache_path);

    // Get the first snapshot
    let model_dir = std::fs::read_dir(&cache_path)
        .expect("Failed to read snapshots directory")
        .next()
        .expect("No snapshot found")?
        .path();

    println!("Loading model from: {:?}", model_dir);

    let model = CodeFormulaModel::from_pretrained(&model_dir, device)
        .expect("Failed to load CodeFormula model");

    println!("Model loaded successfully");

    // Load input tensors from .npy files
    println!("\n=== Loading Input Tensors ===");

    // Load pixel_values
    let pixel_values_file =
        File::open(&pixel_values_path).expect("Failed to open pixel_values file");
    let pixel_values_npy =
        NpyFile::new(pixel_values_file).expect("Failed to parse pixel_values .npy");

    let pixel_values_shape: Vec<i64> = pixel_values_npy.shape().iter().map(|&x| x as i64).collect();
    println!("Pixel values shape: {:?}", pixel_values_shape);

    let pixel_values_vec: Vec<f32> = pixel_values_npy
        .into_vec()
        .expect("Failed to read pixel_values data");

    let pixel_values = tch::Tensor::from_slice(&pixel_values_vec)
        .reshape(&pixel_values_shape)
        .to_device(device);

    // Load input_ids
    let input_ids_file = File::open(&input_ids_path).expect("Failed to open input_ids file");
    let input_ids_npy = NpyFile::new(input_ids_file).expect("Failed to parse input_ids .npy");

    let input_ids_shape: Vec<i64> = input_ids_npy.shape().iter().map(|&x| x as i64).collect();
    println!("Input IDs shape: {:?}", input_ids_shape);

    let input_ids_vec: Vec<i64> = input_ids_npy
        .into_vec()
        .expect("Failed to read input_ids data");

    let input_ids = tch::Tensor::from_slice(&input_ids_vec)
        .reshape(&input_ids_shape)
        .to_device(device);

    // Load attention_mask
    let attention_mask_file =
        File::open(&attention_mask_path).expect("Failed to open attention_mask file");
    let attention_mask_npy =
        NpyFile::new(attention_mask_file).expect("Failed to parse attention_mask .npy");

    let attention_mask_shape: Vec<i64> = attention_mask_npy
        .shape()
        .iter()
        .map(|&x| x as i64)
        .collect();
    println!("Attention mask shape: {:?}", attention_mask_shape);

    let attention_mask_vec: Vec<i64> = attention_mask_npy
        .into_vec()
        .expect("Failed to read attention_mask data");

    let attention_mask = tch::Tensor::from_slice(&attention_mask_vec)
        .reshape(&attention_mask_shape)
        .to_device(device);

    // Run inference
    println!("\n=== Running Inference ===");
    let output_ids = model
        .generate_from_preprocessed(
            &input_ids,
            &pixel_values,
            Some(&attention_mask),
            512, // max_new_tokens
        )
        .expect("Inference failed");

    println!("Generated {} tokens", output_ids.size()[1]);

    // Decode output
    let output_text = model
        .decode_tokens(&output_ids)
        .expect("Failed to decode tokens");

    println!("\n=== Generated Output ===");
    println!("Raw: {}", output_text);

    // Post-process output (remove special tokens, extract language)
    let cleaned = model.post_process(&output_text);

    println!("\n=== Cleaned Output ===");
    println!("Text: {}", cleaned.text);
    println!("Language: {:?}", cleaned.language);

    // Validate output
    println!("\n=== Validation ===");

    // Check language match (formulas should have no language)
    let language_match = cleaned.language == expected.language;
    println!(
        "Language match: {} (expected: {:?}, got: {:?})",
        if language_match { "✓" } else { "✗" },
        expected.language,
        cleaned.language
    );

    // Check text similarity
    let similarity = string_similarity(&cleaned.text, &expected.final_text);
    println!("Text similarity: {:.2}%", similarity);

    if cleaned.text != expected.final_text {
        println!("\nText mismatch:");
        println!("Expected: {}", expected.final_text);
        println!("Got:      {}", cleaned.text);
    }

    // Acceptance criteria: exact match or ≥95% similarity
    assert!(
        similarity >= 95.0,
        "Text similarity {:.2}% below threshold 95.0%",
        similarity
    );
    assert!(
        language_match,
        "Language mismatch: expected {:?}, got {:?}",
        expected.language, cleaned.language
    );

    println!("\n✓ Formula region validation PASSED");
    Ok(())
}
