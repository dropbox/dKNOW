#![cfg(feature = "pytorch")]
// Debug test: Extract Rust logits at first generation step
//
// This test compares Rust vs Python logits after processing the prompt
// to identify where generation diverges.

use docling_pdf_ml::models::code_formula::CodeFormulaModel;
use npyz::{NpyFile, WriteOptions, WriterBuilder};
use std::fs::File;
use std::path::PathBuf;

#[test]
#[ignore] // Ignore by default (requires model weights)
fn test_first_logits_code() -> Result<(), Box<dyn std::error::Error>> {
    extract_first_logits("code", 0)
}

#[test]
#[ignore] // Ignore by default (requires model weights)
fn test_first_logits_formula() -> Result<(), Box<dyn std::error::Error>> {
    extract_first_logits("formula", 1)
}

fn extract_first_logits(
    label: &str,
    region_index: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n============================================================");
    println!("Processing {} region", label);
    println!("============================================================");

    // Load baseline data
    let base_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let page_idx = if label == "code" { 0 } else { 1 };
    let baseline_dir = base_dir.join(format!(
        "baseline_data/code_and_formula/page_{}/code_formula",
        page_idx
    ));

    let pixel_values_path =
        baseline_dir.join(format!("{}_{}_pixel_values.npy", label, region_index));
    let input_ids_path = baseline_dir.join(format!("{}_{}_input_ids.npy", label, region_index));
    let attention_mask_path =
        baseline_dir.join(format!("{}_{}_attention_mask.npy", label, region_index));

    // Load model
    let model_name = "ds4sd/CodeFormulaV2";
    let device = tch::Device::Cpu; // Use CPU for consistency with Python
    println!("Using device: {:?}", device);

    let model_slug = model_name.replace('/', "--");
    let home_dir = std::env::var("HOME").expect("HOME not set");
    let cache_path = format!("{home_dir}/.cache/huggingface/hub/models--{model_slug}/snapshots");

    let model_dir = std::fs::read_dir(&cache_path)?
        .next()
        .expect("No snapshot found")?
        .path();

    println!("Loading model from: {:?}", model_dir);
    let model = CodeFormulaModel::from_pretrained(&model_dir, device)?;
    println!("Model loaded successfully");

    // Load input tensors
    println!("\n=== Loading Input Tensors ===");

    let pixel_values_file = File::open(&pixel_values_path)?;
    let pixel_values_npy = NpyFile::new(pixel_values_file)?;
    let pixel_values_shape: Vec<i64> = pixel_values_npy.shape().iter().map(|&x| x as i64).collect();
    println!("Pixel values shape: {:?}", pixel_values_shape);
    let pixel_values_vec: Vec<f32> = pixel_values_npy.into_vec()?;
    let pixel_values = tch::Tensor::from_slice(&pixel_values_vec)
        .reshape(&pixel_values_shape)
        .to_device(device);

    let input_ids_file = File::open(&input_ids_path)?;
    let input_ids_npy = NpyFile::new(input_ids_file)?;
    let input_ids_shape: Vec<i64> = input_ids_npy.shape().iter().map(|&x| x as i64).collect();
    println!("Input IDs shape: {:?}", input_ids_shape);
    let input_ids_vec: Vec<i64> = input_ids_npy.into_vec()?;
    let input_ids = tch::Tensor::from_slice(&input_ids_vec)
        .reshape(&input_ids_shape)
        .to_device(device);

    let first_20: Vec<i64> = input_ids.narrow(1, 0, 20).flatten(0, -1).try_into()?;
    println!("First 20 input IDs: {:?}", first_20);

    let attention_mask_file = File::open(&attention_mask_path)?;
    let attention_mask_npy = NpyFile::new(attention_mask_file)?;
    let attention_mask_shape: Vec<i64> = attention_mask_npy
        .shape()
        .iter()
        .map(|&x| x as i64)
        .collect();
    let attention_mask_vec: Vec<i64> = attention_mask_npy.into_vec()?;
    let binary_mask = tch::Tensor::from_slice(&attention_mask_vec)
        .reshape(&attention_mask_shape)
        .to_device(device);

    println!("Binary attention mask shape: {:?}", binary_mask.size());
    println!("Binary attention mask dtype: {:?}", binary_mask.kind());

    // Convert binary mask [batch, seq_len] to 4D causal mask [batch, 1, seq_len, seq_len]
    // Python uses: 0.0 for attend, -inf for ignore, with causal (lower triangular) masking
    let seq_len = binary_mask.size()[1];
    let batch_size = binary_mask.size()[0];

    // Create causal mask (lower triangular): 0.0 for attend, -inf for ignore
    let causal_mask = {
        // Create 2D causal matrix [seq_len, seq_len]
        let mask = tch::Tensor::full(
            [seq_len, seq_len],
            f64::NEG_INFINITY,
            (tch::Kind::Float, device),
        );

        // Set lower triangle (including diagonal) to 0.0
        for i in 0..seq_len {
            for j in 0..=i {
                let _ = mask.narrow(0, i, 1).narrow(1, j, 1).fill_(0.0);
            }
        }

        // Expand to 4D [batch, 1, seq_len, seq_len]
        mask.unsqueeze(0)
            .unsqueeze(0)
            .expand([batch_size, 1, seq_len, seq_len], false)
    };

    println!("Causal attention mask shape: {:?}", causal_mask.size());
    println!("Causal mask dtype: {:?}", causal_mask.kind());

    // Sample a few values to verify
    let sample_00: f32 = causal_mask.get(0).get(0).get(0).get(0).try_into()?;
    let sample_01: f32 = causal_mask.get(0).get(0).get(0).get(1).try_into()?;
    let sample_11: f32 = causal_mask.get(0).get(0).get(1).get(1).try_into()?;
    println!("Causal mask [0,0,0,0] (attend): {:.4}", sample_00);
    println!("Causal mask [0,0,0,1] (ignore): {:.4}", sample_01);
    println!("Causal mask [0,0,1,1] (attend): {:.4}", sample_11);

    // Run forward pass to get logits and hidden states
    println!("\n=== Running Forward Pass ===");

    // Get logits using forward_with_preprocessed (now with 4D causal mask)
    let logits = model.forward_with_preprocessed(&input_ids, &pixel_values, Some(&causal_mask))?;
    println!("Logits shape: {:?}", logits.size());

    // Get logits for LAST position [vocab_size]
    let last_logits = logits.select(1, -1).squeeze();
    println!("Last logits shape: {:?}", last_logits.size());

    // Also extract hidden states (before LM head) for comparison
    println!("\n=== Extracting Hidden States ===");

    // We need to get image features and merge manually to get hidden states
    let image_features = model.inner_model().get_image_features(&pixel_values)?;
    let embeddings = model.inner_model().get_text_embeddings(&input_ids);

    // Find <image> token positions
    let image_token_id = 100270i64; // CodeFormulaV2 image token ID (NOT 128257!)
    let input_ids_vec: Vec<i64> = input_ids.flatten(0, -1).try_into()?;
    let image_positions: Vec<usize> = input_ids_vec
        .iter()
        .enumerate()
        .filter_map(|(i, &id)| if id == image_token_id { Some(i) } else { None })
        .collect();

    println!("Image token ID: {}", image_token_id);
    println!("Found {} image token positions", image_positions.len());
    if !image_positions.is_empty() {
        println!(
            "First 10 positions: {:?}",
            &image_positions[..10.min(image_positions.len())]
        );
    }

    let merged_embeddings = if !image_positions.is_empty() {
        let embeddings_mut = embeddings.shallow_clone();
        let size = image_features.size();
        let total_patches = (size[0] * size[1]) as usize;
        let image_features_flat = image_features.view([total_patches as i64, size[2]]);

        for (patch_idx, &token_pos) in image_positions.iter().enumerate() {
            let patch = image_features_flat
                .narrow(0, patch_idx as i64, 1)
                .squeeze_dim(0);
            embeddings_mut
                .narrow(1, token_pos as i64, 1)
                .squeeze_dim(1)
                .copy_(&patch);
        }
        embeddings_mut
    } else {
        embeddings
    };

    // Create output directory for saving
    let output_dir = PathBuf::from("debug_output/rust_first_logits");
    std::fs::create_dir_all(&output_dir)?;

    // Save merged embeddings for debugging
    println!("\n=== Saving Merged Embeddings ===");
    let merged_emb_vec: Vec<f32> = merged_embeddings.flatten(0, -1).try_into()?;
    let merged_emb_shape = merged_embeddings.size();
    println!("Merged embeddings shape: {:?}", merged_emb_shape);
    println!(
        "Merged embeddings min: {:.4}",
        merged_emb_vec.iter().cloned().fold(f32::INFINITY, f32::min)
    );
    println!(
        "Merged embeddings max: {:.4}",
        merged_emb_vec
            .iter()
            .cloned()
            .fold(f32::NEG_INFINITY, f32::max)
    );
    println!(
        "Merged embeddings mean: {:.4}",
        merged_emb_vec.iter().sum::<f32>() / merged_emb_vec.len() as f32
    );

    let merged_emb_npy_path = output_dir.join(format!("{}_merged_embeddings.npy", label));
    let merged_file = File::create(&merged_emb_npy_path)?;
    let mut writer = WriteOptions::new()
        .default_dtype()
        .shape(
            &merged_emb_shape
                .iter()
                .map(|&x| x as u64)
                .collect::<Vec<_>>(),
        )
        .writer(merged_file)
        .begin_nd()?;
    writer.extend(merged_emb_vec.iter().cloned())?;
    writer.finish()?;
    println!("Saved merged embeddings to: {:?}", merged_emb_npy_path);

    // Get hidden states (before LM head) - use causal mask, not binary mask
    let hidden_states =
        model
            .inner_model()
            .get_hidden_states(&merged_embeddings, Some(&causal_mask), false)?;
    println!("\n=== Hidden States ===");
    println!("Hidden states shape: {:?}", hidden_states.size());

    // Get last position hidden state
    let last_hidden = hidden_states.select(1, -1).squeeze();
    println!("Last hidden state shape: {:?}", last_hidden.size());

    // Convert to Vec for statistics and saving
    let last_hidden_vec: Vec<f32> = last_hidden.try_into()?;

    // Save hidden states as numpy for easy comparison
    let hidden_states_npy_path = output_dir.join(format!("{}_last_hidden_state.npy", label));
    let hidden_file = File::create(&hidden_states_npy_path)?;
    let mut writer = WriteOptions::new()
        .default_dtype()
        .shape(&[last_hidden_vec.len() as u64])
        .writer(hidden_file)
        .begin_1d()?;
    writer.extend(last_hidden_vec.iter().cloned())?;
    writer.finish()?;
    println!(
        "Saved last position hidden states to: {:?}",
        hidden_states_npy_path
    );
    println!(
        "Last hidden min: {:.4}",
        last_hidden_vec
            .iter()
            .cloned()
            .fold(f32::INFINITY, f32::min)
    );
    println!(
        "Last hidden max: {:.4}",
        last_hidden_vec
            .iter()
            .cloned()
            .fold(f32::NEG_INFINITY, f32::max)
    );
    println!(
        "Last hidden mean: {:.4}",
        last_hidden_vec.iter().sum::<f32>() / last_hidden_vec.len() as f32
    );

    // Get top-20 predictions
    let (topk_values, topk_indices) = last_logits.topk(20, -1, true, true);
    let topk_values_vec: Vec<f32> = topk_values.try_into()?;
    let topk_indices_vec: Vec<i64> = topk_indices.try_into()?;

    println!("\nTop-20 predictions at FIRST generation step:");
    for i in 0..20 {
        println!(
            "  {}. Token ID: {:6}, Logit: {:10.4}",
            i + 1,
            topk_indices_vec[i],
            topk_values_vec[i]
        );
    }

    // Greedy (argmax)
    let predicted_token = i64::try_from(&last_logits.argmax(-1, false))?;
    println!("\nGreedy (argmax) prediction: {}", predicted_token);

    // Save logits for comparison
    let last_logits_path = output_dir.join(format!("{}_last_logits.pt", label));
    last_logits.save(&last_logits_path)?;
    println!("\nSaved last position logits to: {:?}", last_logits_path);

    // Also extract to Vec for comparison
    let last_logits_vec: Vec<f32> = last_logits.try_into()?;

    // Compare hidden states with Python baseline
    let python_hidden_path = base_dir.join("debug_output/hidden_states/last_hidden_state.npy");
    if python_hidden_path.exists() && label == "code" {
        println!("\n=== Comparing Hidden States with Python Baseline ===");
        let python_file = File::open(&python_hidden_path)?;
        let python_npy = NpyFile::new(python_file)?;
        let python_hidden_vec: Vec<f32> = python_npy.into_vec()?;

        // Compute differences
        let mut hidden_diffs: Vec<f32> = last_hidden_vec
            .iter()
            .zip(python_hidden_vec.iter())
            .map(|(r, p)| (r - p).abs())
            .collect();

        hidden_diffs.sort_by(|a, b| b.partial_cmp(a).unwrap());

        println!("Hidden states max diff: {:.6}", hidden_diffs[0]);
        println!(
            "Hidden states mean diff: {:.6}",
            hidden_diffs.iter().sum::<f32>() / hidden_diffs.len() as f32
        );
        println!("Hidden states top 10 diffs: {:?}", &hidden_diffs[..10]);
    }

    // Load Python baseline and compare logits
    let python_logits_path = base_dir.join(format!(
        "debug_output/first_logits/{}_last_logits.npy",
        label
    ));
    if python_logits_path.exists() {
        println!("\n=== Comparing Logits with Python Baseline ===");
        let python_file = File::open(&python_logits_path)?;
        let python_npy = NpyFile::new(python_file)?;
        let python_vec: Vec<f32> = python_npy.into_vec()?;

        // Compute differences
        let mut diffs: Vec<f32> = last_logits_vec
            .iter()
            .zip(python_vec.iter())
            .map(|(r, p)| (r - p).abs())
            .collect();

        diffs.sort_by(|a, b| b.partial_cmp(a).unwrap());

        println!("Logits max diff: {:.6}", diffs[0]);
        println!(
            "Logits mean diff: {:.6}",
            diffs.iter().sum::<f32>() / diffs.len() as f32
        );
        println!("Logits top 10 diffs: {:?}", &diffs[..10]);

        // Check if prediction matches
        let python_argmax = python_vec
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(idx, _)| idx)
            .unwrap();

        println!("\nPython argmax: {}", python_argmax);
        println!("Rust argmax: {}", predicted_token);
        println!("Match: {}", python_argmax as i64 == predicted_token);
    }

    Ok(())
}
