#![cfg(feature = "pytorch")]
// Test: Compare Rust vs Python layer-by-layer decoder outputs
//
// This test extracts hidden states after each of the 30 decoder layers
// and compares with Python baseline to identify exactly where divergence occurs.

use docling_pdf_ml::models::code_formula::CodeFormulaModel;
use npyz::{NpyFile, WriteOptions, WriterBuilder};
use std::fs::File;
use std::path::PathBuf;

#[test]
#[ignore] // Ignore by default (requires model weights)
fn test_layer_by_layer() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n============================================================");
    println!("Layer-by-Layer Comparison: Rust vs Python");
    println!("============================================================");

    // Load baseline data
    let base_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let baseline_dir = base_dir.join("baseline_data/code_and_formula/page_0/code_formula");

    let pixel_values_path = baseline_dir.join("code_0_pixel_values.npy");
    let input_ids_path = baseline_dir.join("code_0_input_ids.npy");
    let attention_mask_path = baseline_dir.join("code_0_attention_mask.npy");

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

    // Convert binary mask to 4D causal mask
    let seq_len = binary_mask.size()[1];
    let batch_size = binary_mask.size()[0];

    let causal_mask = {
        let mask = tch::Tensor::full(
            [seq_len, seq_len],
            f64::NEG_INFINITY,
            (tch::Kind::Float, device),
        );
        for i in 0..seq_len {
            for j in 0..=i {
                let _ = mask.narrow(0, i, 1).narrow(1, j, 1).fill_(0.0);
            }
        }
        mask.unsqueeze(0)
            .unsqueeze(0)
            .expand([batch_size, 1, seq_len, seq_len], false)
    };

    println!("Causal attention mask shape: {:?}", causal_mask.size());

    // Get image features and merge embeddings
    println!("\n=== Preparing Embeddings ===");
    let image_features = model.inner_model().get_image_features(&pixel_values)?;
    let embeddings = model.inner_model().get_text_embeddings(&input_ids);

    let image_token_id = 100270i64;
    let input_ids_vec: Vec<i64> = input_ids.flatten(0, -1).try_into()?;
    let image_positions: Vec<usize> = input_ids_vec
        .iter()
        .enumerate()
        .filter_map(|(i, &id)| if id == image_token_id { Some(i) } else { None })
        .collect();

    println!("Found {} image token positions", image_positions.len());

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

    println!("Merged embeddings shape: {:?}", merged_embeddings.size());

    // Extract layer-by-layer outputs
    println!("\n=== Extracting Rust Layer Outputs ===");
    let layer_outputs =
        model
            .inner_model()
            .get_layer_outputs(&merged_embeddings, Some(&causal_mask), false)?;

    println!("Extracted {} layer outputs", layer_outputs.len());

    // Save outputs and compare with Python
    let output_dir = base_dir.join("debug_output/rust_layer_outputs");
    std::fs::create_dir_all(&output_dir)?;

    let python_layer_dir = base_dir.join("debug_output/layer_outputs");

    println!("\n=== Comparing Layer Outputs ===");
    println!(
        "{:<10} {:<12} {:<12} {:<12} {:<12} {:<12}",
        "Layer", "Shape", "Min", "Max", "Mean", "Status"
    );
    println!("{}", "-".repeat(80));

    for (layer_idx, hidden_states) in &layer_outputs {
        let layer_name = if *layer_idx == 99 {
            "final_norm".to_string()
        } else {
            format!("layer_{:02}", layer_idx)
        };

        // Get statistics
        let size = hidden_states.size();
        let hidden_vec: Vec<f32> = hidden_states.flatten(0, -1).try_into()?;
        let min_val = hidden_vec.iter().cloned().fold(f32::INFINITY, f32::min);
        let max_val = hidden_vec.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let mean_val = hidden_vec.iter().sum::<f32>() / hidden_vec.len() as f32;

        // Save Rust output
        let rust_path = output_dir.join(format!("{}_output.npy", layer_name));
        let rust_file = File::create(&rust_path)?;
        let mut writer = WriteOptions::new()
            .default_dtype()
            .shape(&size.iter().map(|&x| x as u64).collect::<Vec<_>>())
            .writer(rust_file)
            .begin_nd()?;
        writer.extend(hidden_vec.iter().cloned())?;
        writer.finish()?;

        // Compare with Python if available
        let python_path = if *layer_idx == 99 {
            python_layer_dir.join("final_output.npy")
        } else {
            python_layer_dir.join(format!("layer_{:02}_output.npy", layer_idx))
        };

        let status = if python_path.exists() {
            let python_file = File::open(&python_path)?;
            let python_npy = NpyFile::new(python_file)?;
            let python_vec: Vec<f32> = python_npy.into_vec()?;

            // Compute differences
            let mut diffs: Vec<f32> = hidden_vec
                .iter()
                .zip(python_vec.iter())
                .map(|(r, p)| (r - p).abs())
                .collect();

            diffs.sort_by(|a, b| b.partial_cmp(a).unwrap());

            let max_diff = diffs[0];
            let mean_diff = diffs.iter().sum::<f32>() / diffs.len() as f32;

            // Compute correlation
            let rust_mean = hidden_vec.iter().sum::<f32>() / hidden_vec.len() as f32;
            let python_mean = python_vec.iter().sum::<f32>() / python_vec.len() as f32;

            let numerator: f32 = hidden_vec
                .iter()
                .zip(python_vec.iter())
                .map(|(r, p)| (r - rust_mean) * (p - python_mean))
                .sum();

            let rust_var: f32 = hidden_vec
                .iter()
                .map(|r| (r - rust_mean).powi(2))
                .sum::<f32>()
                .sqrt();

            let python_var: f32 = python_vec
                .iter()
                .map(|p| (p - python_mean).powi(2))
                .sum::<f32>()
                .sqrt();

            let correlation = numerator / (rust_var * python_var);

            if correlation > 0.99 {
                format!("✅ corr={:.6}", correlation)
            } else if correlation > 0.9 {
                format!("⚠️  corr={:.6}", correlation)
            } else {
                format!("❌ corr={:.6}", correlation)
            }
        } else {
            "⏸️  (no baseline)".to_string()
        };

        println!(
            "{:<10} {:>4}x{:<6} {:>12.4} {:>12.4} {:>12.4} {}",
            layer_name, size[1], size[2], min_val, max_val, mean_val, status
        );
    }

    println!("\n=== Done ===");
    println!("Rust outputs saved to: {:?}", output_dir);
    println!("Python baseline at: {:?}", python_layer_dir);

    Ok(())
}
