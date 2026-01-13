#![cfg(feature = "pytorch")]
// Test: Extract Layer 3 Attention Components for Debugging Position 550
//
// This test extracts Q, K, V, attention scores, and attention weights from layer 3
// to compare with Python baseline and identify why position 550 diverges.

use docling_pdf_ml::models::code_formula::CodeFormulaModel;
use npyz::{NpyFile, WriteOptions, WriterBuilder};
use std::fs::File;
use std::path::PathBuf;
use tch::{Kind, Tensor};

#[test]
#[ignore] // Ignore by default (requires model weights)
fn test_layer3_attention_extraction() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n============================================================");
    println!("Layer 3 Attention Component Extraction");
    println!("============================================================");

    // Load baseline data
    let base_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let baseline_dir = base_dir.join("baseline_data/code_and_formula/page_0/code_formula");

    let pixel_values_path = baseline_dir.join("code_0_pixel_values.npy");
    let input_ids_path = baseline_dir.join("code_0_input_ids.npy");
    let attention_mask_path = baseline_dir.join("code_0_attention_mask.npy");

    // Load model
    let model_name = "ds4sd/CodeFormulaV2";
    let device = tch::Device::Cpu;
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
    let pixel_values = Tensor::from_slice(&pixel_values_vec)
        .reshape(&pixel_values_shape)
        .to_device(device);

    let input_ids_file = File::open(&input_ids_path)?;
    let input_ids_npy = NpyFile::new(input_ids_file)?;
    let input_ids_shape: Vec<i64> = input_ids_npy.shape().iter().map(|&x| x as i64).collect();
    println!("Input IDs shape: {:?}", input_ids_shape);
    let input_ids_vec: Vec<i64> = input_ids_npy.into_vec()?;
    let input_ids = Tensor::from_slice(&input_ids_vec)
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
    let binary_mask = Tensor::from_slice(&attention_mask_vec)
        .reshape(&attention_mask_shape)
        .to_device(device);

    // Convert to causal mask
    let seq_len = binary_mask.size()[1];
    let batch_size = binary_mask.size()[0];

    let causal_mask = {
        let mask = Tensor::full([seq_len, seq_len], f64::NEG_INFINITY, (Kind::Float, device));
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

    // Run through layers 0-2 to get layer 3 input
    println!("\n=== Running Layers 0-2 ===");
    let text_decoder = model.inner_model().text_decoder();

    let mut hidden_states = merged_embeddings;

    // Get RoPE embeddings
    let position_ids = Tensor::arange(seq_len, (Kind::Int64, device)).unsqueeze(0);
    let (cos, sin) = text_decoder.get_rope_embeddings(&position_ids, seq_len as usize);

    // Process through layers 0, 1, 2
    for layer_idx in 0..3 {
        let layer_output = text_decoder.layers()[layer_idx].forward(
            &hidden_states,
            &cos,
            &sin,
            Some(&causal_mask),
            false, // train = false
        )?;
        hidden_states = layer_output;
        println!(
            "  Layer {} output shape: {:?}",
            layer_idx,
            hidden_states.size()
        );
    }

    // Now extract layer 3 attention components
    println!("\n=== Extracting Layer 3 Attention Components ===");

    let output_dir = base_dir.join("debug_output/rust_layer3_attention");
    std::fs::create_dir_all(&output_dir)?;

    // Get layer 3
    let layer3 = &text_decoder.layers()[3];

    // Save layer 3 input
    save_tensor(&hidden_states, &output_dir.join("layer3_input.npy"))?;
    println!("  Saved layer3_input.npy: {:?}", hidden_states.size());

    // Apply input layernorm
    let attn_input_normed = layer3.input_layernorm().forward(&hidden_states);
    save_tensor(
        &attn_input_normed,
        &output_dir.join("layer3_input_normed.npy"),
    )?;
    println!(
        "  Saved layer3_input_normed.npy: {:?}",
        attn_input_normed.size()
    );

    // Get attention module
    let self_attn = layer3.self_attn();

    // Project Q, K, V
    let q_proj = attn_input_normed.apply(self_attn.q_proj());
    let k_proj = attn_input_normed.apply(self_attn.k_proj());
    let v_proj = attn_input_normed.apply(self_attn.v_proj());

    save_tensor(&q_proj, &output_dir.join("q_proj.npy"))?;
    save_tensor(&k_proj, &output_dir.join("k_proj.npy"))?;
    save_tensor(&v_proj, &output_dir.join("v_proj.npy"))?;

    println!("  Q proj: {:?}", q_proj.size());
    println!("  K proj: {:?}", k_proj.size());
    println!("  V proj: {:?}", v_proj.size());

    // Reshape for multi-head attention
    let size = attn_input_normed.size();
    let batch_size = size[0];
    let seq_len = size[1];

    let num_q_heads = self_attn.num_q_heads();
    let num_kv_heads = self_attn.num_kv_heads();
    let head_dim = self_attn.head_dim();

    let q = q_proj
        .view([batch_size, seq_len, num_q_heads, head_dim])
        .transpose(1, 2);
    let k = k_proj
        .view([batch_size, seq_len, num_kv_heads, head_dim])
        .transpose(1, 2);
    let v = v_proj
        .view([batch_size, seq_len, num_kv_heads, head_dim])
        .transpose(1, 2);

    save_tensor(&q, &output_dir.join("q_reshaped.npy"))?;
    save_tensor(&k, &output_dir.join("k_reshaped.npy"))?;
    save_tensor(&v, &output_dir.join("v_reshaped.npy"))?;

    println!("  Q reshaped: {:?}", q.size());
    println!("  K reshaped: {:?}", k.size());
    println!("  V reshaped: {:?}", v.size());

    // Get RoPE embeddings
    let position_ids = Tensor::arange(seq_len, (Kind::Int64, device)).unsqueeze(0);
    let (cos, sin) = text_decoder.get_rope_embeddings(&position_ids, seq_len as usize);

    save_tensor(&cos, &output_dir.join("rope_cos.npy"))?;
    save_tensor(&sin, &output_dir.join("rope_sin.npy"))?;

    println!("  RoPE cos: {:?}", cos.size());
    println!("  RoPE sin: {:?}", sin.size());

    // Apply RoPE
    fn rotate_half(x: &Tensor) -> Tensor {
        let size = x.size();
        let half_dim = size[size.len() - 1] / 2;
        let x1 = x.narrow(-1, 0, half_dim);
        let x2 = x.narrow(-1, half_dim, half_dim);
        Tensor::cat(&[-&x2, x1], -1)
    }

    let q_rotated = &q * &cos + &rotate_half(&q) * &sin;
    let k_rotated = &k * &cos + &rotate_half(&k) * &sin;

    save_tensor(&q_rotated, &output_dir.join("q_after_rope.npy"))?;
    save_tensor(&k_rotated, &output_dir.join("k_after_rope.npy"))?;

    println!("  Q after RoPE: {:?}", q_rotated.size());
    println!("  K after RoPE: {:?}", k_rotated.size());

    // Repeat KV heads for grouped query attention
    fn repeat_kv(x: &Tensor, num_kv_groups: i64) -> Tensor {
        if num_kv_groups == 1 {
            return x.shallow_clone();
        }
        let size = x.size();
        let batch = size[0];
        let num_kv_heads = size[1];
        let seq_len = size[2];
        let head_dim = size[3];

        x.unsqueeze(2)
            .expand(
                [batch, num_kv_heads, num_kv_groups, seq_len, head_dim],
                false,
            )
            .reshape([batch, num_kv_heads * num_kv_groups, seq_len, head_dim])
    }

    let num_kv_groups = num_q_heads / num_kv_heads;
    let k_repeated = repeat_kv(&k_rotated, num_kv_groups);
    let v_repeated = repeat_kv(&v, num_kv_groups);

    save_tensor(&k_repeated, &output_dir.join("k_repeated.npy"))?;
    save_tensor(&v_repeated, &output_dir.join("v_repeated.npy"))?;

    println!("  K repeated: {:?}", k_repeated.size());
    println!("  V repeated: {:?}", v_repeated.size());
    println!(
        "  KV groups: {} (num_q_heads={}, num_kv_heads={})",
        num_kv_groups, num_q_heads, num_kv_heads
    );

    // Compute attention scores
    let scale = (head_dim as f64).powf(-0.5);
    let attn_scores = q_rotated.matmul(&k_repeated.transpose(-2, -1)) * scale;

    save_tensor(&attn_scores, &output_dir.join("attn_scores_raw.npy"))?;
    println!("  Attention scores (raw): {:?}", attn_scores.size());

    // Apply causal mask (already computed earlier)
    let causal_mask_2d = causal_mask
        .narrow(1, 0, 1)
        .narrow(0, 0, 1)
        .squeeze_dim(0)
        .squeeze_dim(0);
    save_tensor(&causal_mask_2d, &output_dir.join("causal_mask.npy"))?;

    let attn_scores_masked = &attn_scores + &causal_mask;
    save_tensor(
        &attn_scores_masked,
        &output_dir.join("attn_scores_masked.npy"),
    )?;

    // Softmax
    let attn_weights = attn_scores_masked.softmax(-1, Kind::Float);
    save_tensor(&attn_weights, &output_dir.join("attn_weights.npy"))?;
    println!("  Attention weights: {:?}", attn_weights.size());

    // Attention output
    let attn_output = attn_weights.matmul(&v_repeated);
    save_tensor(&attn_output, &output_dir.join("attn_output_pre_proj.npy"))?;

    // Reshape and project
    let attn_output_reshaped = attn_output.transpose(1, 2).contiguous().view([
        batch_size,
        seq_len,
        num_q_heads * head_dim,
    ]);

    let attn_output_proj = attn_output_reshaped.apply(self_attn.o_proj());
    save_tensor(&attn_output_proj, &output_dir.join("attn_output_proj.npy"))?;
    println!(
        "  Attention output (projected): {:?}",
        attn_output_proj.size()
    );

    // Save position-specific data for position 550
    println!("\n=== Extracting Position 550 ===");
    let position = 550;

    if seq_len > position {
        // Q at position 550: [num_heads, head_dim]
        let q_550 = q_rotated
            .narrow(2, position, 1)
            .squeeze_dim(2)
            .squeeze_dim(0);
        save_tensor(&q_550, &output_dir.join("position_550_q.npy"))?;

        // K all: [num_heads, seq_len, head_dim]
        let k_550_all = k_repeated.squeeze_dim(0);
        save_tensor(&k_550_all, &output_dir.join("position_550_k_all.npy"))?;

        // Attention weights at position 550: [num_heads, seq_len]
        let attn_weights_550 = attn_weights
            .narrow(2, position, 1)
            .squeeze_dim(2)
            .squeeze_dim(0);
        save_tensor(
            &attn_weights_550,
            &output_dir.join("position_550_attn_weights.npy"),
        )?;

        // Attention scores at position 550: [num_heads, seq_len]
        let attn_scores_550 = attn_scores
            .narrow(2, position, 1)
            .squeeze_dim(2)
            .squeeze_dim(0);
        save_tensor(
            &attn_scores_550,
            &output_dir.join("position_550_attn_scores.npy"),
        )?;

        println!("  Saved position 550 components");

        // Print some statistics
        let q_550_vec: Vec<f32> = q_550.flatten(0, -1).try_into()?;
        let attn_weights_550_vec: Vec<f32> = attn_weights_550.flatten(0, -1).try_into()?;

        println!("  Q[0, :5]: {:?}", &q_550_vec[0..5]);
        println!(
            "  Q[0] magnitude: {:.6}",
            (0..64).map(|i| q_550_vec[i].powi(2)).sum::<f32>().sqrt()
        );
        println!(
            "  Attention weights[0, :10]: {:?}",
            &attn_weights_550_vec[0..10]
        );
        println!(
            "  Attention weights[0] sum: {:.6}",
            (0..seq_len as usize)
                .map(|i| attn_weights_550_vec[i])
                .sum::<f32>()
        );
    } else {
        println!("  Warning: Sequence length {} < {}", seq_len, position);
    }

    // Also save position 0 for comparison
    println!("\n=== Extracting Position 0 (text token) ===");
    let q_0 = q_rotated.narrow(2, 0, 1).squeeze_dim(2).squeeze_dim(0);
    save_tensor(&q_0, &output_dir.join("position_0_q.npy"))?;

    let attn_weights_0 = attn_weights.narrow(2, 0, 1).squeeze_dim(2).squeeze_dim(0);
    save_tensor(
        &attn_weights_0,
        &output_dir.join("position_0_attn_weights.npy"),
    )?;

    let q_0_vec: Vec<f32> = q_0.flatten(0, -1).try_into()?;
    println!("  Q[0, :5]: {:?}", &q_0_vec[0..5]);
    println!(
        "  Q[0] magnitude: {:.6}",
        (0..64).map(|i| q_0_vec[i].powi(2)).sum::<f32>().sqrt()
    );

    println!("\n=== Done! ===");
    println!("Saved components to {:?}", output_dir);

    Ok(())
}

// Helper function to save tensor as .npy file
fn save_tensor(tensor: &Tensor, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let size = tensor.size();
    let tensor_vec: Vec<f32> = tensor.flatten(0, -1).try_into()?;

    let file = File::create(path)?;
    let mut writer = WriteOptions::new()
        .default_dtype()
        .shape(&size.iter().map(|&x| x as u64).collect::<Vec<_>>())
        .writer(file)
        .begin_nd()?;
    writer.extend(tensor_vec.iter().cloned())?;
    writer.finish()?;

    Ok(())
}
